//! bastion (INSPECTOR-M1): the server half of the modular colonist
//! inspector — the SECTION PROVIDER REGISTRY.
//!
//! One provider per [`SectionIdV1`], registered in [`provider_for`], which
//! is a match with NO wildcard arm. Appending a section id is therefore a
//! compile error until a provider exists for it.
//!
//! ★ READ-ONLY BY TYPE, NOT BY PROMISE. Everything a provider can see
//! arrives through [`InspectCtx`], and every field on it is a shared
//! reference. The path provider in particular takes `Option<&Chaser>`: a
//! `&Chaser` cannot call `chase`/`find_path`, which need `&mut self`, so
//! "the inspector must not trigger a path search" is enforced by the
//! borrow checker rather than by a comment. A comment cannot enforce.
//!
//! ★ FALLBACK IS IDENTITY. Nothing here is reachable unless a client sent
//! a `Sectioned` request. With the panel closed the server does exactly
//! what it did before, byte for byte.

use common::{
    bastion::WorkType,
    comp::bastion_inspect::{
        InspectFramesV1, JobTallyV1, SectionIdV1, SectionPayloadV1, SectionRequestV1, SectionSetV1,
        SectionedInspectV1, SentimentRowV1, StockRowV1, UnavailableReasonV1,
    },
    uid::Uid,
};

use crate::bastion_jobs::JobBoard;

pub mod colony;
pub mod identity;
pub mod path;
pub mod right_now;
pub mod thinking;

/// The MINIMUM gap the server enforces between answered requests for one
/// client, in seconds.
///
/// ★ THE CLIENT'S CADENCE IS A COURTESY; THIS IS THE GUARANTEE. The panel
/// throttles itself to ~2 Hz for live sections, but a modified or buggy
/// client is not bound by that, and a request handler whose only rate
/// limit lives in the requester is not rate limited at all. 100 ms is
/// comfortably below the panel's own fastest cadence, so this floor never
/// fires for a well-behaved client and is invisible in normal play.
pub const MIN_REQUEST_GAP_SECS: f64 = 0.1;

/// Everything a provider is allowed to see.
///
/// Split into "always" and [`LoadedCtx`] deliberately: the roster half
/// answers for an UNLOADED colonist, the ECS half does not. Keeping them
/// in one flat struct with a pile of `Option`s would let a provider read
/// an ECS field while believing it had roster data — the two-frames
/// confusion this whole design is built to prevent.
pub struct InspectCtx<'a> {
    pub subject: Uid,
    pub frames: InspectFramesV1,
    /// The PERSISTENT record, from the rtsim roster. Present whenever the
    /// subject is a known colonist at all, loaded or not.
    pub record: Option<&'a common::bastion::BastionColonist>,
    /// Resolved server-side from `BastionColonist::parent` (an `NpcId`,
    /// which means nothing to a client).
    pub parent_name: Option<String>,
    /// RUNTIME-ONLY state. Does not survive a server restart even though
    /// it looks durable.
    pub board: &'a JobBoard,
    /// `None` when the subject `Uid` resolves to no loaded ECS entity.
    pub loaded: Option<LoadedCtx<'a>>,
    /// Loaded colonists' `(uid, name)`, SORTED BY UID.
    ///
    /// Sorted so every lookup is a `binary_search` rather than a scan, and
    /// so nothing here can make a reply depend on `specs` join order.
    /// Empty when no requested section needs a name.
    pub names: &'a [(Uid, String)],
    /// The rtsim-derived MIND measurements. `None` means the Thinking
    /// section was not requested and nothing was measured — which the
    /// provider reports as `NotMeasured`, never as an empty mind.
    pub mind: Option<&'a MindCtx>,
    /// The colony-wide measurements. `None` means the Colony section was
    /// not requested. Building this walks every dropped item and every
    /// colonist inventory in the world, which is precisely why it is
    /// gated on the request and not built unconditionally.
    pub colony: Option<&'a ColonyCtx>,
}

/// The expensive, rtsim-backed half of the Thinking section.
///
/// ★ WHY THIS IS MEASURED AT THE CALL SITE AND NOT IN THE PROVIDER.
/// Building it needs the rtsim read guard, `IdMaps`, the chronicle and
/// the thought/affinity tables — none of which a provider can be handed
/// without dragging the whole server into `InspectCtx`. It is built ONLY
/// when the request names `SectionIdV1::Thinking`, so a collapsed panel
/// costs nothing, and its absence is a reportable state rather than a
/// default.
pub struct MindCtx {
    /// `MoodExplanationV1::build` — recomputed through the REAL
    /// `mood_formula`, never cached. `None` when the subject has no rtsim
    /// entity, so the chronicle-dependent thought half cannot be built.
    pub explanation: Option<common::comp::bastion::MoodExplanationV1>,
    /// Every vanilla Big-Five trait the rtsim `Personality` satisfies.
    pub traits: Vec<String>,
    /// Held sentiments with their targets already RESOLVED — the
    /// resolution needs the rtsim roster, which is open at the call site.
    pub sentiments: Vec<SentimentRowV1>,
}

/// The colony-wide measurements, all taken at the call site because they
/// need ECS joins a provider cannot have.
///
/// ★ EVERYTHING HERE IS A MEASUREMENT, NOT A DERIVATION. The provider
/// derives the households, the profession histogram and the drive verdict
/// from these plus the board; nothing in this struct is already-shaped
/// output. That split is deliberate: the derivations are the part worth
/// pinning, and pinning them means being able to build a ctx in a test
/// without a running server.
pub struct ColonyCtx {
    /// Loaded colonist uids, SORTED. The profession histogram is built by
    /// keyed lookup over this rather than by walking
    /// `JobBoard::professions`, so it cannot depend on `HashMap`
    /// iteration order — and so the denominator is honestly the ECS
    /// roster rather than a board table that still holds unloaded
    /// colonists.
    pub colonist_uids: Vec<Uid>,
    /// The item census, already scoped. See
    /// [`colony::stock_census`], which produces it.
    pub stock: Vec<StockRowV1>,
    pub stock_distinct: u32,
    pub stock_truncated: bool,
    /// The four job counters, from ONE scan. See [`tally_jobs`].
    pub jobs: JobTallyV1,
    /// `colony_food_stock` — food inside stockpile regions.
    pub food_pantry: u32,
    /// `colony_food_total` — food anywhere the colony can eat it. This is
    /// the number the drive ladder DECIDES on.
    pub food_total: u32,
    /// Colonists whose `Agent::target` is hostile — the colony's OWN
    /// perception of danger, the same predicate the flee signal and the
    /// hostile census read.
    ///
    /// ★ AN HONEST DUPLICATION, NAMED. `bastion_jobs`'s drive tick owns
    /// the original expression and there is no accessor to call, so this
    /// is a SECOND SITE holding the same predicate. The right repair is a
    /// `colony_threat_count` producer on the board's own module; until it
    /// exists, this comment is the only thing keeping the two in step and
    /// it is a comment, which cannot enforce.
    pub threats: u32,
}

/// The ECS half — present only while the subject is loaded.
pub struct LoadedCtx<'a> {
    pub pos: Option<vek::Vec3<f32>>,
    /// `Health::fraction`. `None` means NO HEALTH COMPONENT, which is not
    /// the same as zero health.
    pub health: Option<f32>,
    pub arbiter: Option<&'a common::comp::bastion::Arbiter>,
    pub active_job: Option<&'a common::comp::bastion::ActiveJob>,
    /// The retained route owner. SHARED reference: a provider physically
    /// cannot make it search.
    pub chaser: Option<&'a common::path::Chaser>,
    /// ECS `Mood(f32)` — the MIRROR every downstream consumer reads.
    ///
    /// `None` means NO `Mood` COMPONENT. The shipped colonist payload
    /// collapses that to `0.0` (`insp_moods.get(e).map_or(0.0, ..)`),
    /// which renders a colonist with no mood at all as a colonist in
    /// total despair. The `Option` is kept here for the same reason
    /// `health` keeps one.
    pub mood: Option<f32>,
    /// `Needs { hunger, rest, recreation }` — the meters the mood
    /// waterfall's penalties are computed from.
    pub needs: Option<(f32, f32, f32)>,
    /// `Energy::fraction`.
    pub energy: Option<f32>,
}

/// A section provider. Total by construction: it always returns a payload,
/// and an unanswerable section returns `Unavailable` WITH A REASON rather
/// than being omitted — a null needs a witness, or the panel cannot tell
/// "no job" from "could not look".
pub type ProviderFn = fn(&InspectCtx<'_>) -> SectionPayloadV1;

/// ★ THE SERVER REGISTRY. NO WILDCARD ARM.
///
/// Appending a variant to `SectionIdV1` fails to compile here until a
/// provider is written and registered. That is the entire point: a
/// section that is declared but not implemented must not be a runtime
/// surprise.
pub const fn provider_for(id: SectionIdV1) -> ProviderFn {
    match id {
        SectionIdV1::Identity => identity::provide,
        SectionIdV1::RightNow => right_now::provide,
        SectionIdV1::Path => path::provide,
        SectionIdV1::Thinking => thinking::provide,
        SectionIdV1::Colony => colony::provide,
    }
}

/// Whether a request asks for `id`.
///
/// ★ THE GATE THE CALL SITE MUST USE. The expensive measurements
/// ([`MindCtx`], [`ColonyCtx`]) are built before `assemble` runs, so the
/// "a collapsed section costs zero" contract is only kept if the call site
/// gates them on exactly the same predicate `assemble` iterates. Exported
/// as one function so there is one predicate and not two, and pinned
/// against `assemble`'s own output in
/// `assemble_computes_exactly_what_wants_admits`.
pub fn wants(req: &SectionRequestV1, id: SectionIdV1) -> bool {
    req.sections.sanitized().contains(id)
}

/// Answer a request.
///
/// The requested list is normalised into `SectionIdV1::ALL` ORDER and
/// deduplicated before any provider runs, so the reply's section order is
/// a function of the section registry and never of the order the client
/// happened to list them. Two identical requests therefore produce
/// byte-identical replies — the property the determinism pin asserts.
pub fn assemble(ctx: &InspectCtx<'_>, req: &SectionRequestV1) -> SectionedInspectV1 {
    // `sanitized()` drops bits naming no registered section: a NEWER
    // client talking to this build will legitimately set some, and a
    // hostile one may set all of them. Unknown bits are ignored rather
    // than refused, so a forward-compatible client still gets the
    // sections this server does know.
    let asked = req.sections.sanitized();
    let mut sections = Vec::with_capacity(asked.len());
    // Iteration is over the REGISTRY, so reply order is the registry's and
    // never the request's. Collapsed sections are absent from the set and
    // are not computed at all: nothing expanded => zero work.
    for id in asked.iter() {
        sections.push(provider_for(id)(ctx));
    }
    SectionedInspectV1 {
        subject: ctx.subject,
        seq: req.seq,
        loaded: ctx.loaded.is_some(),
        frames: ctx.frames,
        sections,
    }
}

/// Build the frames block.
///
/// `dt_secs` and `day_cycle_coefficient` are the server's own, so
/// `ticks_per_game_day` is DERIVED per server rather than assumed — a
/// harness running `day_length: 2.0` (coefficient 720) gets its own
/// figure instead of the default server's 54,000, a 15x error that has
/// already shipped once elsewhere in this codebase.
pub fn frames(
    server_tick: u64,
    rtsim_tick: u64,
    time_of_day: f64,
    dt_secs: f64,
    day_cycle_coefficient: f64,
    schedule_offset_hours: u32,
) -> InspectFramesV1 {
    InspectFramesV1 {
        server_tick,
        rtsim_tick,
        time_of_day,
        ticks_per_game_day: common::bastion::game_time::ticks_per_game_day(
            dt_secs,
            day_cycle_coefficient,
        ),
        schedule_offset_hours,
    }
}

/// The subject's OWN schedule rotation, in hours.
///
/// Today there are exactly two schedule frames: the night watch (offset
/// [`crate::bastion_jobs::NIGHT_WATCH_OFFSET`]) and everyone else. This is
/// a function rather than a field read because the watch is DERIVED
/// membership (`JobBoard::night_watch`, re-derived each cycle), not a
/// stored per-colonist value — so an inspector caching it would go stale
/// the moment the roster rotates.
pub fn schedule_offset_hours(board: &JobBoard, subject: Uid) -> u32 {
    if board.night_watch.contains(&subject) {
        // Same expression the schedule's own rotation uses, not a second
        // copy of the number: `NIGHT_WATCH_OFFSET` is `pub(crate)` and
        // this module is in the same crate, so there is exactly one 14 in
        // the build.
        crate::bastion_jobs::NIGHT_WATCH_OFFSET % 24
    } else {
        0
    }
}

/// ★ THE SERVER-SIDE FLOOR, as a PURE RULE.
///
/// `true` = answer this request. The stateful wrapper is
/// [`admit_request`]; this is the rule it applies, split out so it can be
/// pinned without a server.
///
/// A clock that ran BACKWARDS admits. The floor exists to stop a flood,
/// and a rewound clock (a test harness resetting `Time`, a restart) must
/// not silently mute a legitimate panel for however long the difference
/// happens to be — a guard must not starve the thing it protects.
pub fn admits(last_answered: Option<f64>, now: f64) -> bool {
    match last_answered {
        None => true,
        Some(t) if now < t => true,
        Some(t) => now - t >= MIN_REQUEST_GAP_SECS,
    }
}

/// Per-client "when did we last answer this client".
///
/// ★ HONEST NOTE ON THE SHAPE. This is a process-global map rather than an
/// ECS resource because `specs`' `Default`-based resource auto-setup does
/// not run in this build (resources here are inserted explicitly in
/// `Server::new`), and adding one would mean editing shared files that
/// three other agents are in right now. The consequences of the global
/// are bounded and worth stating: two servers in ONE process share the
/// map, so the floor is slightly STRICTER than intended there, never
/// looser; and `specs` recycles entity ids, so a reconnecting client can
/// inherit a predecessor's stamp, which can cost it one early request and
/// nothing else. Neither can admit a flood, which is the only thing this
/// guards.
static LAST_ANSWERED: std::sync::Mutex<Option<hashbrown::HashMap<u64, f64>>> =
    std::sync::Mutex::new(None);

/// Apply the floor for one client. Returns whether to answer, and stamps
/// the clock when it does.
pub fn admit_request(client_key: u64, now: f64) -> bool {
    let Ok(mut guard) = LAST_ANSWERED.lock() else {
        // A poisoned lock means another thread panicked mid-update. Fail
        // OPEN: refusing every inspector request forever because of an
        // unrelated panic would turn a cosmetic fault into a silent
        // feature outage, and the worst case of answering is one extra
        // read-only assembly.
        return true;
    };
    let map = guard.get_or_insert_with(hashbrown::HashMap::new);
    let last = map.get(&client_key).copied();
    if admits(last, now) {
        map.insert(client_key, now);
        true
    } else {
        false
    }
}

/// A section that cannot answer because the subject is not loaded.
pub(crate) fn unloaded(id: SectionIdV1) -> SectionPayloadV1 {
    SectionPayloadV1::Unavailable(id, UnavailableReasonV1::SubjectUnloaded)
}

/// A section that cannot answer because the subject is not a colonist.
pub(crate) fn not_a_colonist(id: SectionIdV1) -> SectionPayloadV1 {
    SectionPayloadV1::Unavailable(id, UnavailableReasonV1::NotAColonist)
}

/// A section reached without the measurements it needs — the request did
/// not ask for it, so nothing was measured.
///
/// Distinct from every other refusal on purpose: "nobody counted" and
/// "the count is zero" are opposite conclusions.
pub(crate) fn not_measured(id: SectionIdV1) -> SectionPayloadV1 {
    SectionPayloadV1::Unavailable(id, UnavailableReasonV1::NotMeasured)
}

/// A colonist's name, by uid, from the sorted table on [`InspectCtx`].
///
/// `None` when the uid names no LOADED colonist. That happens for real —
/// a bed whose owner has walked out of the loaded region still names
/// them — and it is reported as an unresolved uid rather than hidden.
pub(crate) fn name_of(names: &[(Uid, String)], uid: Uid) -> Option<&str> {
    names
        .binary_search_by(|(u, _)| u.0.get().cmp(&uid.0.get()))
        .ok()
        .map(|i| names[i].1.as_str())
}

/// ★ THE FOUR JOB COUNTERS, FROM ONE SCAN.
///
/// THE DEFECT THIS CLOSES. The shipped colony dashboard walked
/// `board.jobs.values()` FOUR times — once for `jobs_claimed`, once for
/// `jobs_blocked_stance`, once for `jobs_unreachable`, once for
/// `jobs_blocked_materials` — and the material pass re-joined every
/// dropped item in the world INSIDE its filter. Four passes is four
/// chances for one predicate to drift from its neighbours, on four
/// counters a reader compares side by side as though they partitioned one
/// population.
///
/// FALLBACK IS IDENTITY: each predicate below is character-for-character
/// the one the shipped dashboard applied, including the deliberate
/// asymmetries — `blocked_stance` is UNCLAIMED-only while `unreachable`
/// counts regardless of claim, and the material test skips `Haul` (a haul
/// job IS the material fetch).
///
/// The two expensive predicates arrive as closures because they need
/// terrain and an item join the inspector has no business holding:
/// `stance_missing` is `job_stance_missing(&terrain, job)` and
/// `material_unsupplied` is `!stockpile_has_material(def, items, board)`.
/// Both are called ONLY when the cheap structural half already passed, so
/// the scan does no more work per job than the four passes did.
///
/// ★ HONEST NOTE ON THE SINGLE-SCAN PROPERTY. `jobs` is an `Iterator`
/// taken BY VALUE, so a second pass over the source is not expressible
/// here — the guarantee is structural and the visit-count pin beside it is
/// a belt over it, in the same spirit as (and with the same weakness as)
/// `inspect_is_read_only`. The pin that can actually fail is the
/// correctness one: `tally_jobs_matches_the_four_shipped_predicates`
/// builds a board where all four counters differ and every asymmetry
/// matters.
pub fn tally_jobs<'a>(
    jobs: impl Iterator<Item = &'a common::bastion::Job>,
    stance_missing: impl Fn(&common::bastion::Job) -> bool,
    material_unsupplied: impl Fn(&'static str) -> bool,
) -> JobTallyV1 {
    let mut t = JobTallyV1::default();
    for j in jobs {
        t.total += 1;
        if j.claimed_by.is_some() {
            t.claimed += 1;
        } else if stance_missing(j) {
            // UNCLAIMED-only, as shipped: a claimed job's stance is the
            // claimant's problem, not the board's.
            t.blocked_stance += 1;
        }
        if j.unreachable {
            // NOT an `else` — a claimed job can still be flagged
            // unreachable, and the shipped counter counted it.
            t.unreachable += 1;
        }
        if j.needs_materials
            && !matches!(j.kind, common::bastion::JobKind::Haul { .. })
            && j.required_item.is_some_and(&material_unsupplied)
        {
            t.blocked_materials += 1;
        }
    }
    t
}

/// The lane tables, TOTAL over [`WorkType::ALL`] by construction.
///
/// ★ THE LIVE BUG THIS REPLACES. `server/src/sys/msg/in_game.rs:1748` and
/// `:1761` each hard-wrote a SEVEN-element lane array
/// `[Mine, Chop, Build, Haul, Cook, Farm, Guard]` while `WorkType::COUNT`
/// is 8 — so every player who inspected a blacksmith saw no craft skill
/// and no craft desire, in both tables, silently. The return type here is
/// a fixed array sized by `WorkType::COUNT` and filled by iterating
/// `WorkType::ALL`, so the list cannot fall behind a new lane: adding a
/// `WorkType` variant changes `COUNT`, which changes this type, which
/// fails to compile until it is handled.
pub(crate) fn lane_tables(
    c: &common::bastion::BastionColonist,
) -> ([u16; WorkType::COUNT], [f32; WorkType::COUNT]) {
    let mut skills = [0u16; WorkType::COUNT];
    let mut desires = [0f32; WorkType::COUNT];
    for w in WorkType::ALL {
        skills[w.lane_index()] = c.skills.level_for(w);
        desires[w.lane_index()] = c.desires.get(w);
    }
    (skills, desires)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::comp::bastion_inspect::SectionCadenceV1;

    fn uid(n: u64) -> Uid { Uid(std::num::NonZeroU64::new(n).expect("nonzero")) }

    fn record() -> common::bastion::BastionColonist {
        use rand::SeedableRng;
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let mut c = common::bastion::BastionColonist::generate(&mut rng);
        c.name = "Fixture".into();
        c.backstory = "quarry worker".into();
        c.born_tick = Some(1_000);
        c.born_day = Some(0);
        c.owned_bed = Some(vek::Vec3::new(4, 5, 6));
        // A distinctive craft level: the lane the shipped bug dropped.
        c.skills.set_level_for(WorkType::Craft, 9);
        c
    }

    fn frames_fixture() -> InspectFramesV1 {
        frames(
            123,
            54_000 * 3 + 1_000,
            7.0 * 3600.0,
            1.0 / 30.0,
            48.0,
            0,
        )
    }

    fn ctx<'a>(
        board: &'a JobBoard,
        rec: &'a common::bastion::BastionColonist,
        loaded: Option<LoadedCtx<'a>>,
    ) -> InspectCtx<'a> {
        InspectCtx {
            subject: uid(42),
            frames: frames_fixture(),
            record: Some(rec),
            parent_name: Some("Parent".into()),
            board,
            loaded,
            names: &[],
            mind: None,
            colony: None,
        }
    }

    /// A colony measurement fixture with nothing in it — enough to make
    /// the Colony provider ANSWER rather than refuse.
    fn colony_ctx(uids: Vec<Uid>) -> ColonyCtx {
        ColonyCtx {
            colonist_uids: uids,
            stock: Vec::new(),
            stock_distinct: 0,
            stock_truncated: false,
            jobs: JobTallyV1::default(),
            food_pantry: 0,
            food_total: 0,
            threats: 0,
        }
    }

    /// A fully-loaded ECS half, so ECS-frame sections answer.
    fn loaded_fixture<'a>() -> LoadedCtx<'a> {
        LoadedCtx {
            pos: Some(vek::Vec3::new(1.0, 2.0, 3.0)),
            health: Some(1.0),
            arbiter: None,
            active_job: None,
            chaser: None,
            mood: Some(0.6),
            needs: Some((0.5, 0.5, 0.5)),
            energy: Some(1.0),
        }
    }

    fn mind_fixture() -> MindCtx {
        MindCtx {
            explanation: None,
            traits: vec!["Neurotic".into()],
            sentiments: Vec::new(),
        }
    }

    /// ★ EVERY SECTION ID HAS A PROVIDER, and every provider answers the
    /// section it was registered under.
    ///
    /// The second half is what makes this more than a compile check: a
    /// registry can be total and still be MISWIRED (Identity's slot
    /// returning the Path provider). That would compile, and only a
    /// round-trip catches it.
    ///
    /// FALSIFIER: swap two arms in `provider_for` and this goes RED.
    #[test]
    fn inspect_section_ids_are_total() {
        let board = JobBoard::default();
        let rec = record();
        // Unloaded on purpose: every provider must answer SOMETHING even
        // with no ECS half at all.
        let c = ctx(&board, &rec, None);
        for id in SectionIdV1::ALL {
            let payload = provider_for(id)(&c);
            assert_eq!(
                payload.id(),
                id,
                "provider registered under {id:?} answered for {:?}",
                payload.id()
            );
        }
    }

    /// An unloaded subject gets roster sections answered and ECS sections
    /// REFUSED WITH A REASON — never silently dropped, never blanked.
    ///
    /// FALSIFIER: make `right_now::provide` return a default-filled
    /// payload instead of `Unavailable` and this goes RED.
    #[test]
    fn selection_survives_unload() {
        let board = JobBoard::default();
        let rec = record();
        let c = ctx(&board, &rec, None);
        let req = SectionRequestV1 {
            subject: uid(42),
            seq: 5,
            sections: SectionSetV1::all(),
        };
        let reply = assemble(&c, &req);
        assert!(!reply.loaded, "the fixture is deliberately unloaded");
        assert_eq!(reply.seq, 5, "the reply must echo the seq");
        assert_eq!(reply.sections.len(), SectionIdV1::COUNT);

        for p in &reply.sections {
            match p {
                // Identity is roster-backed: it must still ANSWER.
                SectionPayloadV1::Identity(i) => {
                    assert_eq!(i.name, "Fixture");
                    assert_eq!(i.skills[WorkType::Craft.lane_index()], 9);
                    assert!(i.health.is_none(), "no ECS => no health, and None != 0.0");
                },
                SectionPayloadV1::Unavailable(id, UnavailableReasonV1::SubjectUnloaded) => {
                    assert!(
                        !id.available_unloaded(),
                        "{id:?} claims it answers unloaded but refused on the unload"
                    );
                },
                // ★ A SECOND, ORTHOGONAL REFUSAL. `NotMeasured` is about
                // the REQUEST (this section's expensive measurement was
                // not taken), never about the subject — so it may
                // legitimately land on a section that DOES answer while
                // unloaded. Collapsing the two reasons into one arm is
                // exactly what would let "nobody counted" pass for "the
                // colonist is not here".
                SectionPayloadV1::Unavailable(_, UnavailableReasonV1::NotMeasured) => {},
                SectionPayloadV1::Unavailable(id, reason) => {
                    panic!("{id:?} refused for an unexpected reason: {reason:?}")
                },
                other => panic!("{:?} must not answer while unloaded", other.id()),
            }
        }
        // And the claim on the id agrees with what actually happened.
        assert!(SectionIdV1::Identity.available_unloaded());
        assert!(!SectionIdV1::RightNow.available_unloaded());
        assert!(!SectionIdV1::Path.available_unloaded());
        assert!(!SectionIdV1::Thinking.available_unloaded());
        assert!(SectionIdV1::Colony.available_unloaded());

        // ★ THE COLONY SECTION ANSWERS FOR AN UNLOADED SUBJECT — once it
        // has been measured. The town does not disappear when a colonist
        // walks out of view, and this is the case the whole `Uid`-keyed
        // selection was built for.
        let measured = colony_ctx(vec![uid(42)]);
        let c = InspectCtx { colony: Some(&measured), ..ctx(&board, &rec, None) };
        match provider_for(SectionIdV1::Colony)(&c) {
            SectionPayloadV1::Colony(col) => {
                assert_eq!(col.roster_loaded, 1);
                assert!(col.verdict.is_some(), "the ladder must be re-run");
            },
            other => panic!("Colony must answer while unloaded, got {:?}", other.id()),
        }
    }

    /// COLLAPSED SECTIONS ARE NOT COMPUTED. The reply carries exactly the
    /// requested set, never a helpful extra.
    ///
    /// FALSIFIER: change `assemble` to always push all of `ALL` and this
    /// goes RED.
    #[test]
    fn collapsed_sections_are_not_requested() {
        let board = JobBoard::default();
        let rec = record();
        let c = ctx(&board, &rec, None);

        // Nothing expanded => zero sections => a reply with no payload.
        let empty = assemble(&c, &SectionRequestV1 {
            subject: uid(42),
            seq: 1,
            sections: SectionSetV1::empty(),
        });
        assert!(empty.sections.is_empty(), "no expanded section may cost a payload");

        // One expanded => exactly that one.
        let one = assemble(&c, &SectionRequestV1 {
            subject: uid(42),
            seq: 2,
            sections: SectionSetV1::empty().with(SectionIdV1::Identity),
        });
        assert_eq!(one.sections.len(), 1);
        assert_eq!(one.sections[0].id(), SectionIdV1::Identity);
    }

    /// Section order is the REGISTRY's, not the request's, and duplicates
    /// collapse. Without this two clients listing the same sections in
    /// different orders would get different bytes for the same question.
    #[test]
    fn reply_order_is_registry_order_and_deduplicated() {
        let board = JobBoard::default();
        let rec = record();
        let c = ctx(&board, &rec, None);
        let scrambled = assemble(&c, &SectionRequestV1 {
            subject: uid(42),
            seq: 3,
            // Listed out of order and with a duplicate: a set cannot
            // represent either, which is the point -- the wire shape
            // makes "the client asked in a different order" unrepresentable
            // rather than relying on a normalising pass to undo it.
            sections: [
                SectionIdV1::Path,
                SectionIdV1::Colony,
                SectionIdV1::Identity,
                SectionIdV1::Path,
                SectionIdV1::Thinking,
                SectionIdV1::RightNow,
            ]
            .into_iter()
            .collect(),
        });
        let ids: Vec<SectionIdV1> = scrambled.sections.iter().map(|p| p.id()).collect();
        assert_eq!(ids, SectionIdV1::ALL.to_vec(), "reply order must be registry order");
    }

    /// ★ TWO ASSEMBLIES ARE BYTE-IDENTICAL — over two INDEPENDENTLY BUILT
    /// boards holding the same content.
    ///
    /// ★ WHY TWO BOARDS AND NOT TWO CALLS. Iterating one `HashMap` twice
    /// in one process yields the SAME order, so a naive "assemble twice,
    /// compare" pin would stay green against exactly the defect it is
    /// supposed to catch. `RandomState` seeds per MAP INSTANCE, so two
    /// separately-constructed maps with the same keys iterate differently
    /// — and the keys below are inserted in opposite orders as well.
    ///
    /// FALSIFIER (run manually): make `identity::provide` derive
    /// `bed_slot_agrees` by scanning `board.beds.iter()` for
    /// `slot.owner == Some(subject)` instead of a keyed `get`, and this
    /// goes RED.
    #[test]
    fn inspect_two_assemblies_are_byte_identical() {
        use common::bastion::{BedKind, BedSlot};

        let beds: Vec<vek::Vec3<i32>> = (0..24).map(|i| vek::Vec3::new(i, i * 2, 6)).collect();
        let build = |reverse: bool| {
            let mut b = JobBoard::default();
            let mut order: Vec<&vek::Vec3<i32>> = beds.iter().collect();
            if reverse {
                order.reverse();
            }
            for (n, p) in order.into_iter().enumerate() {
                b.beds.insert(*p, BedSlot {
                    kind: BedKind::Frame,
                    owner: Some(uid(n as u64 + 1)),
                    occupant: None,
                });
                b.professions.insert(uid(n as u64 + 1), WorkType::ALL[n % WorkType::COUNT]);
            }
            b.beds.insert(vek::Vec3::new(4, 5, 6), BedSlot {
                kind: BedKind::Frame,
                owner: Some(uid(42)),
                occupant: None,
            });
            b.professions.insert(uid(42), WorkType::Craft);
            b
        };

        let (b1, b2) = (build(false), build(true));
        let rec = record();
        let req = SectionRequestV1 {
            subject: uid(42),
            seq: 9,
            sections: SectionSetV1::all(),
        };
        // serde_json rather than bincode: `bastion-server` does not
        // depend on bincode and a determinism pin is not worth a new
        // dependency. Field and element ORDER is what is under test, and
        // JSON preserves both.
        let enc = |b: &JobBoard| {
            let c = ctx(b, &rec, None);
            serde_json::to_string(&assemble(&c, &req)).expect("the reply encodes")
        };
        assert_eq!(enc(&b1), enc(&b2), "assembly depends on HashMap iteration order");
        // Sanity: the fixture actually exercises the maps (an empty
        // filter would make this pin vacuous).
        let c = ctx(&b1, &rec, None);
        let ident = provider_for(SectionIdV1::Identity)(&c);
        match ident {
            SectionPayloadV1::Identity(i) => {
                assert_eq!(i.profession, Some(WorkType::Craft), "the board must be consulted");
                assert_eq!(i.bed_slot_agrees, Some(true), "the bed table must be consulted");
            },
            other => panic!("expected Identity, got {:?}", other.id()),
        }
    }

    /// ★ THE INSPECTOR MUST NOT PERTURB WHAT IT OBSERVES.
    ///
    /// HONEST NOTE ON THIS PIN'S STRENGTH: `InspectCtx` holds only shared
    /// references, so a provider that mutated the board would not
    /// COMPILE. This runtime check therefore cannot fail while the
    /// signatures hold — it is a belt over a structural guarantee, and it
    /// is worth having only because it goes red the day someone reaches
    /// for interior mutability (a `Cell`, a `RefCell`, an atomic counter)
    /// to "just record a stat" from inside a provider, which the type
    /// system would happily allow.
    #[test]
    fn inspect_is_read_only() {
        let mut board = JobBoard::default();
        board.professions.insert(uid(42), WorkType::Cook);
        board.total_claims = 17;
        board.done_count = 3;
        let rec = record();
        let req = SectionRequestV1 {
            subject: uid(42),
            seq: 1,
            sections: SectionSetV1::all(),
        };
        // A digest of the board fields any provider touches, plus the
        // global counters an interior-mutability "stat" would move.
        let digest = |b: &JobBoard| {
            (
                b.professions.len(),
                b.professions.get(&uid(42)).copied(),
                b.beds.len(),
                b.jobs.len(),
                b.total_claims,
                b.done_count,
                b.night_watch.len(),
            )
        };
        let before = digest(&board);
        for _ in 0..2 {
            let c = ctx(&board, &rec, None);
            let _ = assemble(&c, &req);
        }
        assert_eq!(digest(&board), before, "assembly perturbed the board");
    }

    /// The lane tables cover every lane, including the one the shipped
    /// seven-element array dropped.
    ///
    /// FALSIFIER: replace `WorkType::ALL` in `lane_tables` with the old
    /// seven-element literal and this goes RED on `Craft`.
    #[test]
    fn lane_tables_are_total_over_work_type() {
        let mut rec = record();
        for (n, w) in WorkType::ALL.into_iter().enumerate() {
            rec.skills.set_level_for(w, n as u16 + 1);
        }
        let (skills, desires) = lane_tables(&rec);
        for w in WorkType::ALL {
            assert_eq!(
                skills[w.lane_index()],
                rec.skills.level_for(w),
                "{w:?} skill did not round-trip"
            );
            assert!(desires[w.lane_index()] > 0.0, "{w:?} desire is unset (neutral is 1.0)");
        }
        assert_eq!(skills[WorkType::Craft.lane_index()], 8, "Craft is lane 7 and must be filled");
    }

    /// ★ THE ROTATED SCHEDULE, against the REAL producer.
    ///
    /// The night watch's own hour is not the wall hour, and the shared
    /// pure clock module must agree with `bastion_jobs`'s own rotation —
    /// two producers for one question is how the "evening palette fired
    /// on the watchman's wake-up" defect happened in the first place.
    ///
    /// ★ A CORRECTION TO THE BRIEF THIS WAS BUILT FROM, which asserted
    /// "watchman wall hour 02 => own hour 12 => Sleep". The first half is
    /// right and the second is not: `default_schedule_block(12)` is
    /// `Work` (Work is 8..=15). A watchman at 2am is WORKING — that is
    /// the entire purpose of the night watch. The Sleep case is wall hour
    /// 12 => own hour 22. Both directions are pinned below, because a
    /// one-sided rotation pin passes under an inverted sign.
    #[test]
    fn colonist_hour_rotates_the_watch() {
        use common::bastion::game_time;

        use crate::bastion_jobs::{
            NIGHT_WATCH_OFFSET, ScheduleBlock, colonist_effective_tod, default_schedule_block,
            hour_of_day, schedule_block_at,
        };

        let off = NIGHT_WATCH_OFFSET % 24;
        assert_eq!(off, 14, "the watch offset moved; the fixtures below assume 14");

        // `hashbrown`, not `std` -- `colonist_effective_tod` takes the
        // board's own set type and the two are distinct types with the
        // same name.
        let watch: hashbrown::HashSet<Uid> = [uid(42)].into_iter().collect();
        let hour = |h: u32| f64::from(h) * 3600.0;

        // --- Wall 02: the raid hour.
        let own_2 = hour_of_day(colonist_effective_tod(&watch, Some(&uid(42)), hour(2)));
        assert_eq!(own_2, 12, "wall 02 must rotate to own hour 12");
        assert_eq!(game_time::colonist_hour(hour(2), off), own_2, "the two clocks disagree");
        assert_eq!(
            schedule_block_at(off, 2),
            ScheduleBlock::Work,
            "a watchman at 2am is WORKING -- that is what the night watch is for"
        );
        // And the discrimination that matters: the same wall hour puts an
        // ordinary colonist in bed.
        assert_eq!(default_schedule_block(2), ScheduleBlock::Sleep);

        // --- Wall 12: the watchman's night.
        let own_12 = hour_of_day(colonist_effective_tod(&watch, Some(&uid(42)), hour(12)));
        assert_eq!(own_12, 22, "wall 12 must rotate to own hour 22");
        assert_eq!(game_time::colonist_hour(hour(12), off), own_12);
        assert_eq!(schedule_block_at(off, 12), ScheduleBlock::Sleep);
        assert_eq!(default_schedule_block(12), ScheduleBlock::Work);

        // --- Offset 0 is the IDENTITY, for every hour.
        for h in 0..24u32 {
            let non = hour_of_day(colonist_effective_tod(&watch, Some(&uid(7)), hour(h)));
            assert_eq!(non, h, "a non-watch colonist must read the wall clock");
            assert_eq!(game_time::colonist_hour(hour(h), 0), h);
            assert_eq!(schedule_block_at(0, h), default_schedule_block(h));
        }
    }

    /// ★ ONE PRODUCER FOR TICKS-PER-GAME-DAY.
    ///
    /// `bastion_jobs::ticks_per_game_day` predates this module and is the
    /// producer the coming-of-age gate reads. The inspector's own pure
    /// copy in `common` exists because `common` cannot depend on
    /// `bastion-server`, so this pin is the thing that keeps the two from
    /// drifting: GENERATOR AND CONSUMER MUST AGREE.
    ///
    /// FALSIFIER: change either implementation's fallback or formula and
    /// this goes RED.
    #[test]
    fn ticks_per_game_day_agrees_with_the_job_board_producer() {
        for dt in [1.0 / 30.0, 1.0 / 60.0, 0.05, 0.0, -1.0] {
            for coeff in [48.0, 720.0, 24.0, 0.0] {
                let a = crate::bastion_jobs::ticks_per_game_day(dt, coeff);
                let b = common::bastion::game_time::ticks_per_game_day(dt, coeff);
                assert_eq!(a, b, "the two ticks_per_game_day producers disagree at dt {dt} coeff {coeff}");
            }
        }
        // And the default server's figure, from the settings themselves.
        let f = frames_fixture();
        assert!((f.ticks_per_game_day - 54_000.0).abs() < 1e-6);
    }

    /// ★ THE SERVER FLOOR REFUSES A FLOOD AND ADMITS NORMAL PLAY.
    ///
    /// FALSIFIER: change `>=` to `>` in `admits` and the exactly-at-the-gap
    /// case flips; delete the `now < t` arm and the rewound-clock case
    /// goes RED.
    #[test]
    fn the_request_floor_admits_normal_play_and_refuses_a_flood() {
        // Never asked before.
        assert!(admits(None, 0.0));
        // A flood: same tick, and well inside the gap.
        assert!(!admits(Some(10.0), 10.0));
        assert!(!admits(Some(10.0), 10.0 + MIN_REQUEST_GAP_SECS / 2.0));
        // ★ THE BOUNDARY IS NOT EXACTLY TESTABLE, and the first version of
        // this pin got that wrong -- which is the pin doing its job.
        //
        // It asserted `admits(Some(10.0), 10.0 + MIN_REQUEST_GAP_SECS)`
        // and went RED. `admits` computes `now - t`, and the nearest f64
        // to `10.0 + 0.1` is 10.09999999999999964..., so the subtraction
        // yields 0.09999999999999964 -- a hair BELOW the gap. The
        // boundary of a floating-point comparison is fuzzy by about one
        // ULP and no amount of wanting will make it sharp.
        //
        // That fuzz is ~1e-16 seconds against a 0.1 second gate, so it
        // cannot matter in play. What is pinned instead is the property
        // that does matter and IS representable: a hair past the gap is
        // admitted, a hair before it is refused.
        assert!(admits(Some(10.0), 10.0 + MIN_REQUEST_GAP_SECS * 1.001));
        assert!(!admits(Some(10.0), 10.0 + MIN_REQUEST_GAP_SECS * 0.999));
        // The panel's own fastest cadence sails through.
        let live = f64::from(SectionCadenceV1::Live.min_interval_secs());
        assert!(admits(Some(10.0), 10.0 + live), "the floor must not throttle normal play");
        // A rewound clock admits rather than muting the panel.
        assert!(admits(Some(100.0), 1.0));
    }

    /// ★ THE FOUR JOB COUNTERS FROM ONE SCAN AGREE WITH THE FOUR SHIPPED
    /// PREDICATES — including every asymmetry between them.
    ///
    /// The fixture is built so all four counters DIFFER and so each
    /// asymmetry is load-bearing: a claimed-but-unreachable job (which
    /// counts as unreachable and NOT as blocked_stance), an unclaimed
    /// stance-blocked job, a `Haul` job that bills a material and must
    /// NOT count as blocked (a haul job IS the fetch), and a Build job
    /// that must.
    ///
    /// FALSIFIER: change `else if stance_missing` to a bare `if` and the
    /// claimed-stanceless case flips; drop the `!Haul` guard and
    /// `blocked_materials` goes to 3; make `unreachable` an `else` arm of
    /// the claim test and it goes to 1.
    #[test]
    fn tally_jobs_matches_the_four_shipped_predicates() {
        use common::bastion::{AffordanceClass, DesignationKind, Job, JobKind};

        let job = |claimed: bool, unreachable: bool, kind: JobKind, mat: Option<&'static str>| Job {
            kind,
            work: WorkType::Build,
            player_ordered: false,
            pos: vek::Vec3::zero(),
            skill_floor: 0,
            claimed_by: claimed.then(|| uid(1)),
            suspended_for: None,
            unreachable,
            progress: 0.0,
            required_item: mat,
            needs_materials: mat.is_some(),
            carve_attempted: false,
            is_access: false,
            stuck_strikes: 0,
            benched_until_tick: None,
            depth: 0,
            reservation: None,
            affordance: AffordanceClass::SolidTarget,
        };
        let build = JobKind::Designated(DesignationKind::Build);
        let haul = JobKind::Haul {
            item: uid(9),
            destination: 1,
        };
        let jobs = vec![
            // 0: claimed AND unreachable -> claimed + unreachable, never
            //    blocked_stance.
            job(true, true, build, None),
            // 1: unclaimed, stance-blocked.
            job(false, false, build, None),
            // 2: unclaimed, stance fine, bills an unsupplied material.
            job(false, false, build, Some("mat")),
            // 3: a HAUL that bills a material -- the fetch itself, never
            //    "blocked on materials".
            job(false, false, haul, Some("mat")),
            // 4: bills a material that IS supplied.
            job(false, false, build, Some("have")),
        ];
        // ★ THE STANCE PREDICATE MUST NOT ITSELF TEST THE CLAIM, and the
        // first version of this fixture did — `j.required_item.is_none()
        // && j.claimed_by.is_none()`. That made the pin GREEN under the
        // falsifier it names: turning `else if stance_missing` into a
        // bare `if` changed nothing, because the fixture's own predicate
        // had already excluded every claimed job. A falsifier that does
        // not fire is a pin that does not guard, and this one was found
        // by running it rather than by reading it.
        //
        // Now the predicate is claim-BLIND, so jobs 0 (claimed) and 1
        // (unclaimed) both have a missing stance and only the production
        // `else` keeps job 0 out of the count.
        let stance_missing = |j: &Job| j.required_item.is_none();
        let unsupplied = |def: &'static str| def == "mat";

        let t = tally_jobs(jobs.iter(), stance_missing, unsupplied);
        assert_eq!(t.total, 5);
        assert_eq!(t.claimed, 1);
        assert_eq!(t.blocked_stance, 1, "a claimed job's stance is not the board's problem");
        assert_eq!(t.unreachable, 1, "unreachable counts regardless of claim");
        assert_eq!(t.blocked_materials, 1, "a Haul job is the fetch, not a blockee");
        // The fixture is not degenerate: all four counters are distinct
        // from `total`, and the predicates actually discriminated.
        assert!(t.claimed < t.total && t.blocked_materials < t.total);

        // ★ ONE VISIT PER JOB. Honest note: `jobs` is an `Iterator` taken
        // by value, so a second pass over the SOURCE is not expressible
        // in this signature -- this counter is a belt over a structural
        // guarantee (same shape as `inspect_is_read_only`) and goes red
        // only if the signature is widened back to a re-iterable
        // collection and a second pass added. It is worth having for
        // exactly that day; it is not worth mistaking for the pin that
        // holds the counters, which is the one above.
        let visits = std::cell::Cell::new(0usize);
        let t2 = tally_jobs(
            jobs.iter().inspect(|_| visits.set(visits.get() + 1)),
            stance_missing,
            unsupplied,
        );
        assert_eq!(visits.get(), jobs.len(), "the board was scanned more than once");
        assert_eq!(t, t2, "the tally must not depend on being observed");
    }

    /// ★ AN EXPENSIVE SECTION THAT WAS NOT ASKED FOR IS NOT MEASURED, AND
    /// SAYS SO.
    ///
    /// The Thinking and Colony measurements are built at the CALL SITE
    /// (they need the rtsim guard and ECS joins a provider cannot hold),
    /// so "a collapsed section costs zero" only holds if the call site
    /// gates them on the same predicate `assemble` iterates. [`wants`] is
    /// that one predicate, and this pins it against `assemble`'s own
    /// output rather than against a second copy of the rule.
    ///
    /// FALSIFIER: drop `.sanitized()` from `wants` (an unknown bit then
    /// admits a measurement `assemble` will not use) or from `assemble`
    /// (the reverse), and this goes RED.
    #[test]
    fn assemble_computes_exactly_what_wants_admits() {
        let board = JobBoard::default();
        let rec = record();
        let c = ctx(&board, &rec, None);

        for mask in 0..(1u32 << SectionIdV1::COUNT) {
            let mut set = SectionSetV1::empty();
            for id in SectionIdV1::ALL {
                if mask & (1 << id.index()) != 0 {
                    set = set.with(id);
                }
            }
            let req = SectionRequestV1 { subject: uid(42), seq: 1, sections: set };
            let answered: Vec<SectionIdV1> =
                assemble(&c, &req).sections.iter().map(|p| p.id()).collect();
            let admitted: Vec<SectionIdV1> =
                SectionIdV1::ALL.into_iter().filter(|id| wants(&req, *id)).collect();
            assert_eq!(answered, admitted, "the gate and the assembler disagree at mask {mask}");
        }

        // And an unknown bit -- a NEWER client asking this build for a
        // section it has never heard of -- admits no measurement at all.
        let forward = SectionRequestV1 {
            subject: uid(42),
            seq: 1,
            sections: SectionSetV1::all().with(SectionIdV1::Identity),
        };
        assert!(wants(&forward, SectionIdV1::Identity));
        let unknown_only =
            SectionRequestV1 { subject: uid(42), seq: 1, sections: SectionSetV1::empty() };
        for id in SectionIdV1::ALL {
            assert!(!wants(&unknown_only, id));
        }
        assert!(assemble(&c, &unknown_only).sections.is_empty());
    }

    /// ★ AN UNMEASURED SECTION REFUSES WITH ITS OWN REASON — it never
    /// answers with zeroes.
    ///
    /// "The colony holds no tools" and "nobody counted the tools" are
    /// opposite conclusions. The refusal is the whole reason
    /// `UnavailableReasonV1::NotMeasured` exists.
    ///
    /// FALSIFIER: make `colony::provide` fall through to a
    /// `ColonySectionV1::default()`-shaped payload when `ctx.colony` is
    /// `None` and this goes RED.
    #[test]
    fn an_unmeasured_section_refuses_rather_than_answering_zero() {
        let board = JobBoard::default();
        let rec = record();
        let loaded = loaded_fixture();
        // Loaded, a real colonist, everything present EXCEPT the
        // measurements -- so the only reason to refuse is the one under
        // test.
        let c = ctx(&board, &rec, Some(loaded));
        for id in [SectionIdV1::Thinking, SectionIdV1::Colony] {
            match provider_for(id)(&c) {
                SectionPayloadV1::Unavailable(got, reason) => {
                    assert_eq!(got, id);
                    assert_eq!(reason, UnavailableReasonV1::NotMeasured);
                },
                other => panic!("{id:?} answered without a measurement: {:?}", other.id()),
            }
        }

        // With the measurements present they answer.
        let measured = colony_ctx(vec![uid(42)]);
        let mind = mind_fixture();
        let c = InspectCtx {
            mind: Some(&mind),
            colony: Some(&measured),
            ..ctx(&board, &rec, Some(loaded_fixture()))
        };
        assert!(matches!(
            provider_for(SectionIdV1::Colony)(&c),
            SectionPayloadV1::Colony(_)
        ));
        assert!(matches!(
            provider_for(SectionIdV1::Thinking)(&c),
            SectionPayloadV1::Thinking(_)
        ));
    }

    /// ★ THE COLONY DRIVE CARRIES ITS REASON, ITS MAGNITUDE, ITS BAR AND
    /// ITS AGE — none of which the board stores.
    ///
    /// `colony_drive_for` returns `(drive, reason, value)` and the call
    /// site keeps only the drive. This pins that the re-run reports the
    /// SAME reason the sim's own ladder would, that the held drive is
    /// READ (never recomputed over the top of it), and that a divergence
    /// between held and wanted is visible — the dwell-suppressed state,
    /// which had no player-facing witness at all.
    ///
    /// FALSIFIER: report `want` in the `drive` field instead of the held
    /// drive and the "held is read, not recomputed" assertion goes RED;
    /// hard-code `bar` to `COLONY_SUSTAIN_ENTER_PER_CAP` and the
    /// leaving-Sustain case goes RED.
    #[test]
    fn the_colony_drive_reports_its_reason_and_its_age() {
        use common::bastion::ColonyDrive as D;

        let rec = record();
        let mut board = JobBoard::default();
        // Held at Sustain since tick 23; the frames fixture is at 123.
        board.colony_drive = (D::Sustain, 23);
        let measured = ColonyCtx {
            // Two colonists, no food anywhere -> Sustain stands.
            colonist_uids: vec![uid(42), uid(43)],
            food_pantry: 0,
            food_total: 0,
            ..colony_ctx(Vec::new())
        };
        let c = InspectCtx { colony: Some(&measured), ..ctx(&board, &rec, None) };
        let SectionPayloadV1::Colony(col) = provider_for(SectionIdV1::Colony)(&c) else {
            panic!("Colony must answer once measured");
        };
        assert_eq!(col.drive, D::Sustain, "the held drive is READ, never recomputed over");
        assert_eq!(col.drive_since_tick, 23);
        assert_eq!(col.drive_held_ticks, 100, "123 - 23");
        let v = col.verdict.expect("the ladder must be re-run");
        assert_eq!(v.want, D::Sustain);
        assert_eq!(v.deciding, "food_per_cap", "the reason the board discards");
        assert_eq!(v.food_per_cap, 0.0);
        assert_eq!(v.pop, 2);
        // ★ THE BAND: leaving Sustain needs the EXIT bar, not the entry
        // one. A value printed without its bar is unreadable, and the bar
        // is not a constant.
        assert_eq!(v.bar, crate::bastion_jobs::COLONY_SUSTAIN_EXIT_PER_CAP);

        // A colony with food, beds short: a DIFFERENT reason and a
        // magnitude that is not zero -- so the reason field is genuinely
        // carrying information rather than a constant.
        let fed = ColonyCtx { food_total: 40, ..colony_ctx(vec![uid(42), uid(43)]) };
        let mut grow_board = JobBoard::default();
        grow_board.colony_drive = (D::Grow, 0);
        let c = InspectCtx { colony: Some(&fed), ..ctx(&grow_board, &rec, None) };
        let SectionPayloadV1::Colony(col) = provider_for(SectionIdV1::Colony)(&c) else {
            panic!("Colony must answer");
        };
        let v = col.verdict.expect("verdict");
        assert_eq!(v.deciding, "beds_short");
        assert_eq!(v.value, 2.0, "two colonists, no beds");
        assert_eq!(v.bar, crate::bastion_jobs::COLONY_SUSTAIN_ENTER_PER_CAP);
        assert_eq!(v.want, D::Grow);

        // ★ HELD ≠ WANTED — the case that makes "the held drive is READ"
        // a real assertion.
        //
        // The two fixtures above both happen to have the ladder agreeing
        // with the board, so `drive: held` and `drive: want` were
        // INDISTINGUISHABLE and the pin stayed green under its own named
        // falsifier. Found by running it. A colony that is starving while
        // the board still holds Defend separates them: the board says
        // Defend (no transition has been committed) and the ladder says
        // Sustain (no threats, no food).
        let mut stale = JobBoard::default();
        stale.colony_drive = (D::Defend, 11);
        let starving = ColonyCtx {
            colonist_uids: vec![uid(42), uid(43)],
            food_pantry: 0,
            food_total: 0,
            threats: 0,
            ..colony_ctx(Vec::new())
        };
        let c = InspectCtx { colony: Some(&starving), ..ctx(&stale, &rec, None) };
        let SectionPayloadV1::Colony(col) = provider_for(SectionIdV1::Colony)(&c) else {
            panic!("Colony must answer");
        };
        assert_eq!(col.drive, D::Defend, "the HELD drive is the board's, never the ladder's");
        let v = col.verdict.expect("verdict");
        assert_eq!(v.want, D::Sustain, "the ladder wants a different drive");
        assert_ne!(col.drive, v.want, "the fixture must actually separate held from wanted");
        assert_eq!(v.deciding, "food_per_cap");
        // Leaving a non-Sustain drive is judged against the ENTRY bar.
        assert_eq!(v.bar, crate::bastion_jobs::COLONY_SUSTAIN_ENTER_PER_CAP);

        // ★ A BACKWARDS CLOCK cannot manufacture an enormous age.
        let mut future = JobBoard::default();
        future.colony_drive = (D::Grow, u64::MAX);
        let c = InspectCtx { colony: Some(&fed), ..ctx(&future, &rec, None) };
        let SectionPayloadV1::Colony(col) = provider_for(SectionIdV1::Colony)(&c) else {
            panic!("Colony must answer");
        };
        assert_eq!(col.drive_held_ticks, 0, "a stale `since` must saturate, not wrap");
    }

    /// ★ EVERY STOCK ROW IS A BREAKDOWN, AND THE SCOPES PARTITION.
    ///
    /// The defect this closes by construction: a tool count of `0` that
    /// was stockpile-scoped, beside 64 tools carried and 3 on the ground.
    /// A single number could not tell a broken forge from broken hauling.
    ///
    /// HONEST LIMIT ON THIS PIN: `stock_census` needs `PickupItem` and
    /// `Inventory` values, which cannot be built without an asset load, so
    /// the census itself is exercised with EMPTY iterators here — what is
    /// pinned is the shape (four rows per label, `Total` equal to the sum
    /// of the three disjoint scopes) over a hand-built row list, plus the
    /// empty case. The live census is covered by the `-p veloren-server`
    /// build alone, which is stated rather than implied.
    #[test]
    fn every_stock_row_is_a_breakdown_and_total_is_their_sum() {
        use common::comp::bastion_inspect::{StockRowV1, StockScopeV1};

        let board = JobBoard::default();
        let (rows, distinct, truncated) =
            colony::stock_census(std::iter::empty(), std::iter::empty(), &board, 8);
        assert!(rows.is_empty(), "an empty world censuses to nothing");
        assert_eq!(distinct, 0);
        assert!(!truncated);

        // The rendered shape, as the provider hands it on: four rows per
        // item, and `Total` is the sum of the three DISJOINT ones.
        let hammer = |scope, count| StockRowV1 {
            item_label: "common.items.tool.craftsman_hammer".into(),
            count,
            scope,
        };
        let rows = vec![
            hammer(StockScopeV1::InStockpileRegions, 0),
            hammer(StockScopeV1::CarriedByColonists, 64),
            hammer(StockScopeV1::OnGroundAnywhere, 3),
            hammer(StockScopeV1::Total, 67),
        ];
        let disjoint: u32 = rows
            .iter()
            .filter(|r| StockScopeV1::DISJOINT.contains(&r.scope))
            .map(|r| r.count)
            .sum();
        let total = rows
            .iter()
            .find(|r| r.scope == StockScopeV1::Total)
            .expect("a total row")
            .count;
        assert_eq!(disjoint, total, "the scopes must partition");
        // The row that used to be a bare `0`: on its own it says "broken
        // forge", and the breakdown says "broken hauling".
        assert_eq!(
            rows.iter().find(|r| r.scope == StockScopeV1::InStockpileRegions).unwrap().count,
            0
        );
        assert_ne!(total, 0, "the number a single scalar would have hidden");
    }

    /// ★ THE PHASE-2 SECTIONS ARE BYTE-IDENTICAL ACROSS TWO INDEPENDENTLY
    /// BUILT BOARDS holding the same content.
    ///
    /// The same construction as the phase-1 determinism pin, extended to
    /// the sections that read `professions`, `beds` and `designated` —
    /// every one of which is a `HashMap` or an order-carrying `Vec`.
    /// `RandomState` seeds per MAP INSTANCE, so two separately-built maps
    /// with the same keys iterate differently, and the keys are inserted
    /// in opposite orders as well.
    ///
    /// FALSIFIER (RUN, and it fires): build the household member lists by
    /// walking `board.beds` directly instead of taking
    /// `derive_households`'s uid-sorted `members`, and this goes RED.
    ///
    /// ★ A FALSIFIER THAT DOES NOT WORK, recorded because the first
    /// version of this doc named it. Rewriting `profession_histogram` to
    /// iterate `board.professions` leaves this pin GREEN — and that is
    /// CORRECT, not a hole: the histogram's output is an array indexed by
    /// `lane_index()`, and counting into an array is order-free however
    /// the map is walked. A pin cannot catch a defect that is not one.
    /// The keyed lookup there is justified by the frame argument, not by
    /// determinism; see `colony::profession_histogram`'s own doc.
    #[test]
    fn colony_assembly_is_byte_identical_across_two_boards() {
        use common::bastion::{BedKind, BedSlot, DesignationKind, Region};

        let beds: Vec<vek::Vec3<i32>> = (0..24).map(|i| vek::Vec3::new(i, 0, 6)).collect();
        let build = |reverse: bool| {
            let mut b = JobBoard::default();
            // Two Bed regions covering the bed line, pushed in a fixed
            // order (the derivation's order IS the push order).
            for (lo, hi) in [(0, 11), (12, 23)] {
                b.designated.push((
                    Region {
                        min: vek::Vec3::new(lo, -1, 5),
                        max: vek::Vec3::new(hi, 1, 7),
                    },
                    DesignationKind::Bed,
                ));
            }
            let mut order: Vec<&vek::Vec3<i32>> = beds.iter().collect();
            if reverse {
                order.reverse();
            }
            // ★ THE OWNER IS KEYED ON THE BED, NOT ON THE INSERTION
            // INDEX -- and getting that wrong is what this pin caught on
            // its first run. Keying on `enumerate()` made the two boards
            // hold genuinely DIFFERENT content (bed x=0 owned by uid 1 in
            // one, by uid 24 in the other), so the reply differed for a
            // real reason and the pin would have been reporting a fixture
            // defect as a determinism defect. Only the ORDER of insertion
            // may differ between the two.
            for p in order {
                let owner = uid(p.x as u64 + 1);
                b.beds.insert(*p, BedSlot {
                    kind: BedKind::Frame,
                    owner: Some(owner),
                    occupant: None,
                });
                b.professions
                    .insert(owner, WorkType::ALL[p.x as usize % WorkType::COUNT]);
            }
            b.colony_drive = (common::bastion::ColonyDrive::Grow, 7);
            b
        };

        let (b1, b2) = (build(false), build(true));
        let rec = record();
        let roster: Vec<Uid> = (1..=24).map(uid).collect();
        let names: Vec<(Uid, String)> =
            roster.iter().map(|u| (*u, format!("Colonist{}", u.0.get()))).collect();
        let measured = colony_ctx(roster);
        let req = SectionRequestV1 {
            subject: uid(42),
            seq: 9,
            sections: SectionSetV1::empty().with(SectionIdV1::Colony),
        };
        let enc = |b: &JobBoard| {
            let c = InspectCtx {
                colony: Some(&measured),
                names: &names,
                ..ctx(b, &rec, None)
            };
            serde_json::to_string(&assemble(&c, &req)).expect("the reply encodes")
        };
        assert_eq!(enc(&b1), enc(&b2), "colony assembly depends on HashMap iteration order");

        // Sanity: the fixture actually exercises the maps and the names.
        let c = InspectCtx { colony: Some(&measured), names: &names, ..ctx(&b1, &rec, None) };
        let SectionPayloadV1::Colony(col) = provider_for(SectionIdV1::Colony)(&c) else {
            panic!("Colony must answer");
        };
        assert_eq!(col.households.len(), 2, "the households must derive");
        assert_eq!(col.beds_total, 24);
        assert_eq!(col.beds_outside_households, 0);
        assert_eq!(col.professions.iter().sum::<u32>(), 24, "every colonist is named");
        assert_eq!(col.profession_unnamed, 0);
        assert!(
            col.households.iter().flat_map(|h| &h.members).all(|m| m.name.is_some()),
            "the member names must resolve -- an empty filter would make this pin vacuous"
        );
        // Households cover every bed and every member exactly once.
        assert_eq!(col.households.iter().map(|h| h.beds).sum::<u32>(), 24);
        assert_eq!(col.households.iter().map(|h| h.members.len()).sum::<usize>(), 24);
        // Members are uid-sorted inside a household.
        for h in &col.households {
            let mut sorted = h.members.clone();
            sorted.sort_by_key(|m| m.uid.0.get());
            assert_eq!(&sorted, &h.members, "household members must be uid-sorted");
        }
    }

    /// Cadence is what the panel's request throttle is derived from, and
    /// the SERVER FLOOR is strictly looser than the panel's own fastest
    /// cadence — a floor that fired during normal play would be a
    /// throttle, not a guarantee.
    #[test]
    fn server_floor_is_below_the_panel_cadence() {
        let fastest = SectionCadenceV1::Live.min_interval_secs() as f64;
        assert!(
            MIN_REQUEST_GAP_SECS < fastest,
            "the server floor ({MIN_REQUEST_GAP_SECS}s) must not fire for a well-behaved \
             client (fastest cadence {fastest}s)"
        );
    }
}
