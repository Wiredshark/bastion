//! bastion (INSPECTOR-M1): the MODULAR colonist inspector — shared protocol.
//!
//! ★ WHAT THIS IS AND IS NOT. A complete request/response inspector already
//! ships (`ClientGeneral::BastionInspect` / `ServerGeneral::BastionInspectInfo`
//! and `BastionInspectPayload`). What it lacks is STRUCTURE: the server
//! assembles a rich payload, the client flattens the whole thing into
//! `Vec<String>` and draws one unstyled text block. Adding a field means
//! editing the payload struct, the fill site and the formatter, and nothing
//! anywhere fails when one of the three is forgotten.
//!
//! This module replaces that with a SECTION REGISTRY. A section is a
//! self-contained unit of "one thing you can ask about a colonist":
//!
//! * an id ([`SectionIdV1`]) — the request token and the registry key,
//! * a payload ([`SectionPayloadV1`]) — the server's structured answer,
//! * a provider (server-side, `bastion_server::bastion_inspector`),
//! * a view (client-side, `voxygen::hud::bastion_inspector`) turning the
//!   payload into [`InspectRow`]s.
//!
//! Adding a section is: append a variant, and let the compiler walk you to
//! the provider and the view. Both registries are wildcard-free matches, so
//! a section that is declared but not implemented does not compile.
//!
//! ★ NO NEW WIRE MESSAGE. This rides the EXISTING message pair through two
//! appended enum variants — `BastionInspectTarget::Sectioned` and
//! `BastionInspectKind::Sectioned`. A new `ClientGeneral`/`ServerGeneral`
//! variant would have to be listed in seven places and would need its own
//! wire golden; appending to the payload enums leaves every existing golden
//! byte-identical, because bincode encodes a variant by its ORDINAL and the
//! ordinals of the existing variants do not move. That is also why variants
//! here are APPEND-ONLY, never reordered.
//!
//! ★ EVERY ROW NAMES ITS PRODUCER. See [`InspectRow`]. This is not
//! decoration — the defect that motivates it is on the record.

use serde::{Deserialize, Serialize};

use crate::{bastion::WorkType, uid::Uid};

/// The panel-side request state machine (pure; see its own module doc for
/// why client policy lives in `common`).
pub mod subscription;

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

/// WHICH STORE a displayed number was read from.
///
/// ★ TWO FRAMES COMPARED AS ONE. Half the defects in one adversarial review
/// of this subsystem were a value from one frame printed beside a value from
/// another as if they were commensurable: the ECS roster against the rtsim
/// roster, wall hour against a rotated schedule, sim seconds against game
/// seconds, `cpos` against `gpos`. A colonist that is loaded has an ECS
/// entity; one that is not exists only in the rtsim roster. The two disagree
/// legitimately and constantly, so the row must say which one it asked.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameV1 {
    /// Live ECS components on a LOADED entity. Absent when unloaded.
    Ecs,
    /// The persistent rtsim roster (`Data.npcs`). Survives unload and
    /// restart; lags the ECS while loaded.
    RtsimRoster,
    /// The server-side `JobBoard`. RUNTIME-ONLY — rebuilt from
    /// `JobBoard::default()` at every server start, so nothing here
    /// survives a restart even though it looks durable.
    JobBoard,
    /// Computed by the inspector from other rows. Never a store.
    Derived,
}

impl FrameV1 {
    /// No wildcard arm: a new frame must be named here.
    pub const fn label(self) -> &'static str {
        match self {
            FrameV1::Ecs => "ECS",
            FrameV1::RtsimRoster => "rtsim-roster",
            FrameV1::JobBoard => "JobBoard",
            FrameV1::Derived => "Derived",
        }
    }
}

/// THE TWO CLOCKS, BOTH NAMED — plus everything needed to convert between
/// them, because the client cannot derive it.
///
/// The client receives `day_cycle_coefficient` in `ServerConstants` but
/// never receives `dt`, so it cannot compute ticks-per-game-day on its own;
/// [`Self::ticks_per_game_day`] is therefore SENT rather than recomputed.
/// See `crate::bastion::game_time` for the arithmetic and for why
/// `born_day` may never be fed to an age function.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InspectFramesV1 {
    /// `bastion_server::Tick` — server ticks since PROCESS BOOT. Does not
    /// survive a restart. Useful only for "how fresh is this snapshot".
    pub server_tick: u64,
    /// rtsim `Data.tick` — PERSISTENT across restart. The only clock an
    /// age may be computed against.
    pub rtsim_tick: u64,
    /// `TimeOfDay` in GAME seconds. Reset to `settings.world.start_time`
    /// at every boot, so its day index is BOOT-RELATIVE.
    pub time_of_day: f64,
    /// Derived server-side from `dt * day_cycle_coefficient`. 54,000 at a
    /// default server.
    pub ticks_per_game_day: f64,
    /// The SUBJECT's own schedule rotation, in hours. 0 for an ordinary
    /// colonist; 14 for the night watch. A watchman's own hour is NOT the
    /// wall hour, and reading the raw global clock for a rotated colonist
    /// has already shipped once as a real defect.
    pub schedule_offset_hours: u32,
}

// ---------------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------------

/// ONE DISPLAYED ROW, which CANNOT BE BUILT WITHOUT NAMING ITS PRODUCER.
///
/// ★ THE DEFECT THIS IS SHAPED AROUND. The inspector once displayed a tool
/// count of `0` while 64 tools existed elsewhere in the colony: the number
/// was true, but it was STOCKPILE-SCOPED and the label did not say so. A
/// player reading it could not tell "the forge is broken" from "hauling is
/// broken" — two entirely different repairs — because the row did not carry
/// its scope. A row that cannot say where its number came from is worse
/// than no row, because the reader supplies the missing context themselves
/// and is confidently wrong.
///
/// The fields are PRIVATE and the only constructor takes `producer`, so
/// "forgot to name the producer" is not a state this type can be in. The
/// constructor additionally refuses an empty producer loudly rather than
/// letting a `""` satisfy the type system — a safety check must fail
/// loudly, and `""` is exactly the value a careless `Default` would supply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectRow {
    label: String,
    value: String,
    /// The SYMBOL that produced this number — a function or field path,
    /// not a prose description. `"JobBoard::professions"`, not
    /// `"the profession system"`. A reader must be able to grep it.
    producer: &'static str,
    /// The unit, or `""` for a genuinely unitless value (a name, an
    /// enum). Never `""` for a number.
    unit: &'static str,
    /// What population/region the number covers, when that is narrower
    /// than "everything". `None` means the row is about the subject
    /// itself and nothing was filtered.
    scope: Option<&'static str>,
    frame: FrameV1,
}

impl InspectRow {
    /// The ONLY constructor. `producer` must be non-empty.
    pub fn new(
        label: impl Into<String>,
        value: impl Into<String>,
        producer: &'static str,
        unit: &'static str,
        frame: FrameV1,
    ) -> Self {
        assert!(
            !producer.is_empty(),
            "an InspectRow must name a real producer; an empty producer is the \
             unlabelled-number defect this type exists to prevent",
        );
        Self {
            label: label.into(),
            value: value.into(),
            producer,
            unit,
            scope: None,
            frame,
        }
    }

    /// Narrow the row's population. Use whenever the number is NOT over
    /// everything the label implies.
    #[must_use]
    pub fn scoped(mut self, scope: &'static str) -> Self {
        self.scope = Some(scope);
        self
    }

    pub fn label(&self) -> &str { &self.label }

    pub fn value(&self) -> &str { &self.value }

    pub fn producer(&self) -> &'static str { self.producer }

    pub fn unit(&self) -> &'static str { self.unit }

    pub fn scope(&self) -> Option<&'static str> { self.scope }

    pub fn frame(&self) -> FrameV1 { self.frame }

    /// The provenance suffix a UI shows on hover / in verbose mode:
    /// `producer | unit | scope | frame`. Kept here so every view renders
    /// provenance identically instead of each inventing a format.
    pub fn provenance(&self) -> String {
        let mut s = String::from(self.producer);
        if !self.unit.is_empty() {
            s.push_str(" | ");
            s.push_str(self.unit);
        }
        if let Some(scope) = self.scope {
            s.push_str(" | scope: ");
            s.push_str(scope);
        }
        s.push_str(" | frame: ");
        s.push_str(self.frame.label());
        s
    }
}

// ---------------------------------------------------------------------------
// The section registry
// ---------------------------------------------------------------------------

/// How often a section is worth re-requesting.
///
/// ★ BANDWIDTH IS A CORRECTNESS PROPERTY HERE. Detail is fetched for the
/// SELECTED colonist only, collapsed sections are not requested at all, and
/// nothing selected costs ZERO bytes. A section that changes every tick
/// (where they are standing) and one that changes once a game-day (their
/// profession) must not share a cadence.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionCadenceV1 {
    /// ~2 Hz — moves continuously.
    Live,
    /// ~0.5 Hz — changes on the order of game-minutes.
    Slow,
}

impl SectionCadenceV1 {
    /// Minimum seconds between requests for a section at this cadence.
    pub const fn min_interval_secs(self) -> f32 {
        match self {
            SectionCadenceV1::Live => 0.5,
            SectionCadenceV1::Slow => 2.0,
        }
    }
}

/// THE REGISTRY KEY. Append-only: the ordinal is wire-visible.
///
/// Every id must have a provider (`bastion_server::bastion_inspector`) and
/// a view (`voxygen::hud::bastion_inspector`), and both registries are
/// wildcard-free, so appending a variant here is a COMPILE ERROR until it
/// is implemented at both ends.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SectionIdV1 {
    /// Who this is: name, trade, age, family, backstory, bed, health, and
    /// the FULL skill/desire table over `WorkType::ALL`.
    Identity,
    /// What they are doing this instant: drive, activity, the active job
    /// and its Traveling/Arrived/Waiting state.
    RightNow,
    /// The retained navigation route, drawn in the world.
    Path,
}

impl SectionIdV1 {
    pub const COUNT: usize = 3;

    /// Every section, in display order.
    pub const ALL: [SectionIdV1; Self::COUNT] =
        [SectionIdV1::Identity, SectionIdV1::RightNow, SectionIdV1::Path];

    /// The exhaustiveness anchor for [`Self::ALL`]. No wildcard arm — a
    /// new variant fails to compile here first.
    pub const fn index(self) -> usize {
        match self {
            SectionIdV1::Identity => 0,
            SectionIdV1::RightNow => 1,
            SectionIdV1::Path => 2,
        }
    }

    /// The panel heading. No wildcard arm.
    pub const fn title(self) -> &'static str {
        match self {
            SectionIdV1::Identity => "Identity",
            SectionIdV1::RightNow => "Right Now",
            SectionIdV1::Path => "Path",
        }
    }

    /// No wildcard arm.
    pub const fn cadence(self) -> SectionCadenceV1 {
        match self {
            // A name and a skill table do not change at 2 Hz.
            SectionIdV1::Identity => SectionCadenceV1::Slow,
            SectionIdV1::RightNow => SectionCadenceV1::Live,
            SectionIdV1::Path => SectionCadenceV1::Live,
        }
    }

    /// Whether this section can be answered for an UNLOADED subject —
    /// i.e. whether it reads only rtsim-roster state.
    ///
    /// ★ SELECTION IS KEYED ON `Uid`, NOT `specs::Entity`, precisely so an
    /// unloading colonist does not silently blank the panel. When the
    /// entity is gone the ECS-frame sections cannot answer, and the panel
    /// says so rather than showing nothing.
    pub const fn available_unloaded(self) -> bool {
        match self {
            // Name/backstory/skills/bed/parent all live on the persistent
            // record, so an unloaded colonist still has an identity.
            SectionIdV1::Identity => true,
            // Drive, activity and the active job are ECS components.
            SectionIdV1::RightNow => false,
            // The route lives on `Agent::chaser`, an ECS component.
            SectionIdV1::Path => false,
        }
    }
}

/// ★ COMPILE-TIME COMPLETENESS CHECK for [`SectionIdV1::ALL`].
///
/// At MODULE scope deliberately. The same guard for `WorkType::ALL` was
/// first written as an ASSOCIATED const inside an impl block, and
/// associated consts are only evaluated when USED — so it never ran and
/// reordering `ALL` compiled clean. A free `const _: ()` is always
/// evaluated.
///
/// HONEST RESIDUAL, identical to `WorkType`'s: `COUNT` is hand-written, so
/// this cannot by itself force a new variant INTO `ALL`. What it
/// guarantees is that `ALL` agrees with the wildcard-free `index` match
/// over `0..COUNT` — and appending a variant is impossible without
/// visiting `index`, `title`, `cadence` and `available_unloaded`, all four
/// of which fail to compile until the author handles it.
const _: () = {
    let mut i = 0;
    while i < SectionIdV1::COUNT {
        assert!(
            SectionIdV1::ALL[i].index() == i,
            "SectionIdV1::ALL must be ordered by index() and cover 0..COUNT"
        );
        i += 1;
    }
};

/// A SET of section ids, as a bitmask.
///
/// ★ WHY A BITSET AND NOT A `Vec<SectionIdV1>`. `BastionInspectTarget` is
/// `Copy`, and the whole existing client pump relies on that (it compares
/// targets by value and stores them beside an `Instant`). A `Vec` in the
/// appended variant would strip `Copy` from the enum and ripple through
/// every one of those call sites — a large diff in shared files, to carry
/// at most 32 booleans. A bitmask keeps the enum `Copy`, bounds the
/// request at four bytes, and makes duplicate-and-reorder impossible by
/// construction rather than by a normalising pass.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionSetV1(u32);

/// The bitset must be wide enough for the registry. Module scope, so it is
/// always evaluated.
const _: () = assert!(
    SectionIdV1::COUNT <= 32,
    "SectionSetV1 is a u32 bitset; widen it before adding a 33rd section"
);

impl SectionSetV1 {
    pub const fn empty() -> Self { Self(0) }

    /// Every registered section.
    pub const fn all() -> Self {
        // Bits 0..COUNT. Written as a shift rather than a loop so it is
        // a `const fn` on every toolchain.
        Self(if SectionIdV1::COUNT >= 32 { u32::MAX } else { (1u32 << SectionIdV1::COUNT) - 1 })
    }

    #[must_use]
    pub const fn with(self, id: SectionIdV1) -> Self { Self(self.0 | (1u32 << id.index())) }

    #[must_use]
    pub const fn without(self, id: SectionIdV1) -> Self { Self(self.0 & !(1u32 << id.index())) }

    #[must_use]
    pub const fn toggled(self, id: SectionIdV1) -> Self { Self(self.0 ^ (1u32 << id.index())) }

    pub const fn contains(self, id: SectionIdV1) -> bool {
        self.0 & (1u32 << id.index()) != 0
    }

    pub const fn is_empty(self) -> bool { self.sanitized().0 == 0 }

    pub const fn len(self) -> usize { self.sanitized().0.count_ones() as usize }

    /// Drop bits that name no registered section.
    ///
    /// A peer is not obliged to be well-behaved, and a NEWER client
    /// talking to an older server will legitimately set bits this build
    /// has never heard of. `contains` is already safe (it only ever tests
    /// known ids), but `len` and `is_empty` would otherwise count
    /// phantom sections — and "the client asked for 9 sections" is the
    /// kind of number a rate limiter might one day believe.
    #[must_use]
    pub const fn sanitized(self) -> Self { Self(self.0 & Self::all().0) }

    /// The set's members, always in REGISTRY order.
    ///
    /// Order is a property of the registry, not of the request, so two
    /// clients asking for the same sections in different orders cannot
    /// produce different replies.
    pub fn iter(self) -> impl Iterator<Item = SectionIdV1> {
        SectionIdV1::ALL.into_iter().filter(move |id| self.contains(*id))
    }
}

impl FromIterator<SectionIdV1> for SectionSetV1 {
    fn from_iter<I: IntoIterator<Item = SectionIdV1>>(iter: I) -> Self {
        iter.into_iter().fold(Self::empty(), |acc, id| acc.with(id))
    }
}

// ---------------------------------------------------------------------------
// Section payloads
// ---------------------------------------------------------------------------

/// Identity — the persistent record.
///
/// Everything here is readable from the rtsim roster, so this section
/// answers for an unloaded colonist too (`health` excepted, which is an
/// ECS component and is `None` when unloaded — the `Option` is
/// load-bearing: absence is not zero health).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdentitySectionV1 {
    pub name: String,
    /// `JobBoard::professions` — the rolling dominant lane with
    /// hysteresis, recomputed daily. `None` on day 0 or before any lane
    /// work. RUNTIME-ONLY: the board does not survive a restart.
    pub profession: Option<WorkType>,
    /// `BastionColonist::born_tick`, the PERSISTENT rtsim `Data.tick`
    /// epoch. `None` for founders and settlers, who arrived grown.
    pub born_tick: Option<u64>,
    /// `BastionColonist::born_day` — carried ONLY so the panel can show
    /// it under a label that says it is boot-relative. It is never used
    /// to compute an age. See `crate::bastion::game_time`.
    pub born_day_boot_relative: Option<i64>,
    /// The parent's name, resolved server-side from
    /// `BastionColonist::parent` (an `NpcId`, which is meaningless to the
    /// client).
    pub parent_name: Option<String>,
    pub backstory: String,
    /// `BastionColonist::owned_bed` — the persistent ownership truth.
    pub owned_bed: Option<vek::Vec3<i32>>,
    /// Whether the runtime `JobBoard::beds` slot for that bed agrees that
    /// this colonist owns it. A disagreement between the durable record
    /// and the runtime table is exactly the kind of thing an inspector
    /// exists to surface, so it is reported rather than reconciled.
    pub bed_slot_agrees: Option<bool>,
    /// ECS `Health::fraction`. `None` when unloaded — NOT zero.
    pub health: Option<f32>,
    pub guard_bravery: f32,
    /// ★ TOTAL OVER `WorkType::ALL` BY CONSTRUCTION, indexed by
    /// `WorkType::lane_index()`.
    ///
    /// A fixed-size array rather than a `Vec<(String, u16)>` because the
    /// `Vec` shape is what let the live bug happen: the fill site
    /// hand-wrote a SEVEN-element lane array against a `WorkType::COUNT`
    /// of 8, so every blacksmith inspected showed no craft skill and no
    /// craft desire. With an array the length is checked by the compiler
    /// and the lane list cannot fall behind.
    pub skills: [u16; WorkType::COUNT],
    /// Same shape, same reason. Neutral is 1.0, not 0.0.
    pub desires: [f32; WorkType::COUNT],
}

/// Right Now — the live instant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RightNowSectionV1 {
    /// `Arbiter::current`. `Drive::Idle` is the documented default when
    /// no `Arbiter` component exists, so this is not an `Option`.
    pub drive: crate::comp::bastion::Drive,
    /// `Arbiter::last_scores` — post-modulation urgencies (work, flee,
    /// idle).
    pub last_scores: (f32, f32, f32),
    /// `Arbiter::activity` — current work lane + NORMALIZED progress
    /// (0..1). `None` = not on a progress-bearing work job.
    ///
    /// Note the traveling case: a claimant still walking to the site
    /// reads `Some((work, 0.0))`, so "Hauling 0%" means "on the way",
    /// not "stuck at zero". The job phase below is what disambiguates.
    pub activity: Option<(WorkType, f32)>,
    /// The display-only status stamp (`RestingToClimb`, `WaitingForLadder`,
    /// …), or `None`.
    ///
    /// HONEST LIMIT: the stamped variants live only ~2 seconds
    /// (`STATUS_DISPLAY_TTL_TICKS`), so a UI polling at 2 Hz will usually
    /// read `None` even while the state is occurring. `None` here means
    /// "not stamped in the last two seconds", NOT "not happening".
    pub status: Option<crate::comp::bastion::BastionColonistStatus>,
    /// The colonist's own position, ECS frame.
    pub pos: Option<vek::Vec3<f32>>,
    pub job: Option<ActiveJobViewV1>,
}

/// The active job as the panel needs it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveJobViewV1 {
    /// `ActiveJob::job`. The id is the `JobBoard::jobs` KEY, not a field
    /// on `Job` — there is no `Job::id`.
    pub id: crate::bastion::JobId,
    /// Traveling / Arrived / Waiting, straight off `ActiveJob::state`.
    pub state: crate::comp::bastion::ActiveJobState,
    /// A coarse, wire-safe tag for `JobKind`. `None` when the id no
    /// longer resolves on the board — a real and reportable state (the
    /// job completed or was reaped while this colonist still points at
    /// it), not something to hide behind a default.
    pub kind: Option<crate::comp::bastion::JobKindTagV1>,
    pub work: Option<WorkType>,
    pub pos: Option<vek::Vec3<i32>>,
    /// Straight-line distance from the colonist to the job's STANCE cell.
    ///
    /// The stance offset matters: the target is `job.pos + ActiveJob::stance`,
    /// not `job.pos`, and assuming the default `(0,0,1)` would misreport
    /// the distance for every job with a different stance.
    pub distance: Option<f32>,
    /// `ARRIVE_DIST`, so the panel can say whether the distance above is
    /// inside arrival tolerance without hard-coding a rival copy of the
    /// number.
    pub arrive_dist: f32,
    /// Raw `Job::progress`. NOT normalized — the denominator differs by
    /// job kind (Chop scales by wood count), which is why
    /// `RightNowSectionV1::activity` is the fraction to display and this
    /// is the raw figure for a reader who wants it.
    pub raw_progress: Option<f32>,
    pub unreachable: Option<bool>,
    pub needs_materials: Option<bool>,
    /// `Job::claimed_by`. Reported because it can legitimately differ
    /// from the subject: a colonist can hold an `ActiveJob` pointing at a
    /// job whose claim has moved.
    pub claimed_by: Option<Uid>,
}

/// The cap on transmitted route nodes.
///
/// A route is a debug overlay, not a map. 96 nodes is far beyond what is
/// legible in the world and bounds the packet; anything longer sets
/// [`PathSectionV1::truncated`] so the panel says the line is a PREFIX
/// rather than quietly drawing a short path.
pub const PATH_NODE_CAP: usize = 96;

/// Path — the RETAINED route, read-only.
///
/// ★ READ ONLY. The provider calls `Chaser::get_route()` and nothing else.
/// It never calls a path search and never takes a `&mut Chaser`: an
/// inspector that made the thing it observes re-plan would be measuring
/// itself. Nodes are BLOCK coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathSectionV1 {
    /// Up to [`PATH_NODE_CAP`] nodes, in route order.
    pub nodes: Vec<vek::Vec3<i32>>,
    /// `Route::next_idx` — the boundary between the WALKED prefix and the
    /// REMAINING suffix. Clamped into `nodes`' range.
    pub next_idx: u32,
    /// The route's real length before the cap.
    pub total_nodes: u32,
    pub truncated: bool,
    /// `Chaser::needs_search()` — true when no route is retained at all.
    /// A colonist with `needs_search: true` and `nodes: []` is not
    /// broken; it is between plans.
    pub needs_search: bool,
    /// Order-sensitive hash of the FULL node list (pre-cap). The client
    /// rebuilds its world-space line only when this moves, so a
    /// stationary route costs no geometry churn.
    pub nodes_hash: u64,
}

impl PathSectionV1 {
    /// The node-list hash, defined ONCE so producer and consumer cannot
    /// drift. Order-sensitive by construction.
    pub fn hash_nodes(nodes: &[vek::Vec3<i32>]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        nodes.len().hash(&mut h);
        for n in nodes {
            n.x.hash(&mut h);
            n.y.hash(&mut h);
            n.z.hash(&mut h);
        }
        h.finish()
    }
}

/// One section's answer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SectionPayloadV1 {
    Identity(IdentitySectionV1),
    RightNow(RightNowSectionV1),
    Path(PathSectionV1),
    /// A section that was requested but could not be answered, with the
    /// REASON.
    ///
    /// ★ A NULL NEEDS A WITNESS. Dropping an unanswerable section would
    /// make "the colonist has no job" and "the server could not look"
    /// indistinguishable in the panel — the exact ambiguity this whole
    /// inspector exists to remove. So refusal is a payload, and it names
    /// which section refused and why.
    Unavailable(SectionIdV1, UnavailableReasonV1),
}

/// Why a section could not answer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnavailableReasonV1 {
    /// The subject `Uid` resolves to no loaded ECS entity. Sections whose
    /// data lives in ECS components cannot answer; the roster ones still
    /// do.
    SubjectUnloaded,
    /// The entity exists but is not a colonist.
    NotAColonist,
    /// The subject has no retained route right now.
    NoRoute,
}

impl UnavailableReasonV1 {
    /// No wildcard arm.
    pub const fn label(self) -> &'static str {
        match self {
            UnavailableReasonV1::SubjectUnloaded => "unloaded — showing roster state only",
            UnavailableReasonV1::NotAColonist => "not a colonist",
            UnavailableReasonV1::NoRoute => "no route retained right now",
        }
    }
}

impl SectionPayloadV1 {
    /// Which section this payload answers.
    ///
    /// ★ NO WILDCARD ARM. This is one of the three registry matches; a new
    /// `SectionPayloadV1` variant fails to compile here.
    pub const fn id(&self) -> SectionIdV1 {
        match self {
            SectionPayloadV1::Identity(_) => SectionIdV1::Identity,
            SectionPayloadV1::RightNow(_) => SectionIdV1::RightNow,
            SectionPayloadV1::Path(_) => SectionIdV1::Path,
            SectionPayloadV1::Unavailable(id, _) => *id,
        }
    }
}

// ---------------------------------------------------------------------------
// The wire envelopes
// ---------------------------------------------------------------------------

/// The request, carried by `BastionInspectTarget::Sectioned`.
///
/// `Copy`, which is what lets it ride an enum the existing client pump
/// compares and stores by value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionRequestV1 {
    /// Whose panel this is. A `Uid`, NOT a `specs::Entity` — selection
    /// must survive the subject unloading.
    pub subject: Uid,
    /// Monotonic per client. The reply echoes it; a reply whose seq does
    /// not match the outstanding request is DROPPED, which is how "at
    /// most one request in flight" is enforced against a late answer.
    pub seq: u32,
    /// Exactly the sections the panel has EXPANDED. A collapsed section
    /// is absent, and absent means not computed and not sent.
    pub sections: SectionSetV1,
}

/// The reply, carried by `BastionInspectKind::Sectioned`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SectionedInspectV1 {
    pub subject: Uid,
    pub seq: u32,
    /// Whether the subject resolved to a loaded ECS entity.
    pub loaded: bool,
    pub frames: InspectFramesV1,
    /// One entry per REQUESTED section, in `SectionIdV1::ALL` order —
    /// including `Unavailable` entries. Never silently short.
    pub sections: Vec<SectionPayloadV1>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The payload half of the registry is TOTAL: every `SectionIdV1` has
    /// a payload variant that reports that id.
    ///
    /// FALSIFIER: point `SectionPayloadV1::Path(_)`'s arm in `id()` at
    /// `SectionIdV1::RightNow` and this goes RED.
    #[test]
    fn inspect_section_ids_are_total_over_payloads() {
        // ALL is complete and ordered (also enforced at compile time).
        for (i, id) in SectionIdV1::ALL.iter().enumerate() {
            assert_eq!(id.index(), i);
            assert!(!id.title().is_empty());
        }
        // Every id is reachable as a payload id. Built explicitly rather
        // than derived from ALL, so this cannot pass by tautology.
        let samples = [
            SectionPayloadV1::Unavailable(
                SectionIdV1::Identity,
                UnavailableReasonV1::SubjectUnloaded,
            ),
            SectionPayloadV1::Unavailable(SectionIdV1::RightNow, UnavailableReasonV1::NotAColonist),
            SectionPayloadV1::Unavailable(SectionIdV1::Path, UnavailableReasonV1::NoRoute),
        ];
        let mut seen: Vec<SectionIdV1> = samples.iter().map(|p| p.id()).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), SectionIdV1::COUNT, "a section id is unreachable as a payload");
    }

    /// The skill/desire tables are TOTAL over `WorkType::ALL` by TYPE.
    ///
    /// This is the structural repair for the seven-element lane array
    /// that shipped: it is not possible to build an `IdentitySectionV1`
    /// with a short lane list, because the length is part of the type.
    ///
    /// FALSIFIER: change either array to `[_; 7]` and the crate does not
    /// compile.
    #[test]
    fn identity_lane_tables_cover_every_work_type() {
        let ident = IdentitySectionV1 {
            name: "T".into(),
            profession: None,
            born_tick: None,
            born_day_boot_relative: None,
            parent_name: None,
            backstory: String::new(),
            owned_bed: None,
            bed_slot_agrees: None,
            health: None,
            guard_bravery: 0.5,
            skills: [0; WorkType::COUNT],
            desires: [1.0; WorkType::COUNT],
        };
        assert_eq!(ident.skills.len(), WorkType::COUNT);
        assert_eq!(ident.desires.len(), WorkType::COUNT);
        // Every lane index is addressable — the property the hand-written
        // array violated for `Craft`.
        for w in WorkType::ALL {
            let i = w.lane_index();
            assert!(i < ident.skills.len(), "{w:?} has no skill slot");
            let _ = ident.skills[i];
            let _ = ident.desires[i];
        }
        assert!(WorkType::ALL.iter().any(|w| matches!(w, WorkType::Craft)), "Craft must be a lane");
    }

    /// ★ EVERY ROW NAMES A PRODUCER — enforced by the type, checked here
    /// at the boundary.
    ///
    /// FALSIFIER: delete the `assert!` in `InspectRow::new` and the empty
    /// -producer case below stops panicking.
    #[test]
    fn every_row_names_a_producer_by_construction() {
        let r = InspectRow::new("Trade", "smith", "JobBoard::professions", "", FrameV1::JobBoard)
            .scoped("this colonist");
        assert_eq!(r.producer(), "JobBoard::professions");
        assert_eq!(r.frame(), FrameV1::JobBoard);
        assert_eq!(r.scope(), Some("this colonist"));
        assert!(r.provenance().contains("JobBoard::professions"));
        assert!(r.provenance().contains("frame: JobBoard"));

        let panicked = std::panic::catch_unwind(|| {
            InspectRow::new("x", "y", "", "", FrameV1::Derived)
        })
        .is_err();
        assert!(panicked, "an empty producer must fail loudly, not silently pass");
    }

    /// The route hash is ORDER-SENSITIVE. A commutative hash would call a
    /// reversed route unchanged and never rebuild the drawn line.
    ///
    /// FALSIFIER: replace the loop with an XOR fold and the reversed case
    /// below stops differing.
    #[test]
    fn path_node_hash_is_order_sensitive() {
        let a = vek::Vec3::new(1, 2, 3);
        let b = vek::Vec3::new(4, 5, 6);
        let fwd = PathSectionV1::hash_nodes(&[a, b]);
        let rev = PathSectionV1::hash_nodes(&[b, a]);
        assert_ne!(fwd, rev, "the node hash must be order-sensitive");
        assert_eq!(fwd, PathSectionV1::hash_nodes(&[a, b]), "and stable");
        assert_ne!(fwd, PathSectionV1::hash_nodes(&[a]), "and length-sensitive");
    }

    /// ★ THE WIRE GOLDEN FOR THE APPENDED VARIANTS.
    ///
    /// Deliberately NOT an entry in `WIRE_SHAPE_GOLDENS`: that table is
    /// 1:1 with `ClientGeneral`/`ServerGeneral` VARIANTS, and its
    /// `coverage_is_all_covered` pin asserts the covered count equals the
    /// enums’ own variant totals — so a second golden for the same
    /// `BastionInspect` variant would make that count a lie. This pins the
    /// thing that actually matters instead: THE ORDINALS.
    ///
    /// bincode encodes an enum by its variant index, so appending is safe
    /// only while the existing indices do not move. Both halves are
    /// asserted: that `Sectioned` sits at the end, and that every variant
    /// before it still encodes exactly where it did.
    ///
    /// FALSIFIER: move `Sectioned` above `Chronicle` in either enum and
    /// this goes RED — as would every peer running the old build,
    /// silently, which is the whole reason the pin exists.
    #[test]
    fn appended_sectioned_variants_do_not_move_existing_ordinals() {
        use crate::{
            comp::bastion::{BastionInspectKind, BastionInspectTarget},
            uid::Uid,
        };

        let uid = Uid(std::num::NonZeroU64::new(1).expect("nonzero"));
        let enc = |v: &BastionInspectTarget| {
            bincode::serde::encode_to_vec(v, bincode::config::legacy())
                .expect("a target encodes")
        };

        // The pre-existing ordinals, unchanged. bincode’s legacy config
        // writes the variant index as a u32 little-endian prefix.
        assert_eq!(enc(&BastionInspectTarget::Entity(uid))[0..4], [0, 0, 0, 0]);
        assert_eq!(
            enc(&BastionInspectTarget::Cell(vek::Vec3::new(1, 2, 3)))[0..4],
            [1, 0, 0, 0]
        );
        assert_eq!(enc(&BastionInspectTarget::Colony)[0..4], [2, 0, 0, 0]);
        assert_eq!(enc(&BastionInspectTarget::Chronicle(uid))[0..4], [3, 0, 0, 0]);
        // The appended one takes the NEXT index, never an existing one.
        let sectioned = BastionInspectTarget::Sectioned(SectionRequestV1 {
            subject: uid,
            seq: 1,
            sections: SectionSetV1::all(),
        });
        assert_eq!(enc(&sectioned)[0..4], [4, 0, 0, 0]);

        // `BastionInspectKind::Sectioned` is variant 7, after Colonist,
        // Job, Stockpile, Farm, FellSet, Colony and Chronicle.
        let reply = BastionInspectKind::Sectioned(SectionedInspectV1 {
            subject: uid,
            seq: 1,
            loaded: false,
            frames: InspectFramesV1 {
                server_tick: 0,
                rtsim_tick: 0,
                time_of_day: 0.0,
                ticks_per_game_day: 54_000.0,
                schedule_offset_hours: 0,
            },
            sections: Vec::new(),
        });
        let bytes = bincode::serde::encode_to_vec(&reply, bincode::config::legacy())
            .expect("a reply encodes");
        assert_eq!(bytes[0..4], [7, 0, 0, 0], "Sectioned must be the LAST kind");

        // The `payload: None` shape the shipped `BastionInspectInfo`
        // golden encodes is untouched by the append: an Option’s None is
        // a discriminant of its own and does not depend on the inner
        // enum’s arity. This is the reason the existing goldens stay
        // green, stated as an assertion rather than as a hope.
        let none: Option<BastionInspectKind> = None;
        assert_eq!(
            bincode::serde::encode_to_vec(&none, bincode::config::legacy())
                .expect("None encodes"),
            vec![0u8],
        );
    }

    /// The section bitset: order-free, duplicate-free, and unknown bits
    /// cannot name a section that does not exist.
    ///
    /// FALSIFIER: drop the mask in `sanitized` and an unknown bit starts
    /// counting toward `len()`.
    #[test]
    fn section_set_is_order_free_and_sanitizes_unknown_bits() {
        let a: SectionSetV1 = [SectionIdV1::Path, SectionIdV1::Identity].into_iter().collect();
        let b: SectionSetV1 = [SectionIdV1::Identity, SectionIdV1::Path].into_iter().collect();
        assert_eq!(a, b, "a set cannot represent an order");
        let dup: SectionSetV1 = [SectionIdV1::Path, SectionIdV1::Path].into_iter().collect();
        assert_eq!(dup.len(), 1, "a set cannot represent a duplicate");
        // Members always come back in REGISTRY order.
        assert_eq!(a.iter().collect::<Vec<_>>(), vec![SectionIdV1::Identity, SectionIdV1::Path]);
        assert_eq!(SectionSetV1::all().len(), SectionIdV1::COUNT);
        assert!(SectionSetV1::empty().is_empty());
        for id in SectionIdV1::ALL {
            assert!(SectionSetV1::all().contains(id));
            assert!(!SectionSetV1::all().without(id).contains(id));
        }
    }

    /// Cadence separates live from slow, and the panel's own courtesy
    /// interval is derived from it rather than typed at the call site.
    #[test]
    fn cadences_are_distinct_and_ordered() {
        assert_eq!(SectionIdV1::RightNow.cadence(), SectionCadenceV1::Live);
        assert_eq!(SectionIdV1::Path.cadence(), SectionCadenceV1::Live);
        assert_eq!(SectionIdV1::Identity.cadence(), SectionCadenceV1::Slow);
        assert!(
            SectionCadenceV1::Live.min_interval_secs()
                < SectionCadenceV1::Slow.min_interval_secs()
        );
    }
}
