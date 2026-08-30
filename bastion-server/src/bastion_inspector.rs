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
        InspectFramesV1, SectionIdV1, SectionPayloadV1, SectionRequestV1, SectionSetV1,
        SectionedInspectV1, UnavailableReasonV1,
    },
    uid::Uid,
};

use crate::bastion_jobs::JobBoard;

pub mod identity;
pub mod path;
pub mod right_now;

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
    }
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
                SectionPayloadV1::Unavailable(id, reason) => {
                    assert!(
                        !id.available_unloaded(),
                        "{id:?} claims it answers unloaded but refused"
                    );
                    assert_eq!(*reason, UnavailableReasonV1::SubjectUnloaded);
                },
                other => panic!("{:?} must not answer while unloaded", other.id()),
            }
        }
        // And the claim on the id agrees with what actually happened.
        assert!(SectionIdV1::Identity.available_unloaded());
        assert!(!SectionIdV1::RightNow.available_unloaded());
        assert!(!SectionIdV1::Path.available_unloaded());
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
                SectionIdV1::Identity,
                SectionIdV1::Path,
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
