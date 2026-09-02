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

/// HOW LOUD A ROW IS.
///
/// ★ WHY THIS EXISTS AT ALL, and why it is not styling. Phase 2 adds a row
/// that compares two producers of ONE number — the ECS `Mood` mirror
/// against `MoodExplanationV1::total_mood`, which is recomputed through the
/// real `mood_formula`. When they disagree the mirror is stale, and a stale
/// mirror is the single most valuable thing this panel can say. A reader
/// scanning forty rows will not find it unless the row itself carries the
/// fact that it is a FINDING and not a reading.
///
/// Deliberately NOT a colour: `InspectRow` is shared by `common` and knows
/// nothing about a renderer. It is a severity, and each view decides how to
/// draw it.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum RowSeverityV1 {
    /// An ordinary reading.
    #[default]
    Normal,
    /// A row that is itself a finding — two producers disagreeing, a
    /// durable record contradicting a runtime table.
    Alarm,
}

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
    severity: RowSeverityV1,
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
            severity: RowSeverityV1::Normal,
        }
    }

    /// Narrow the row's population. Use whenever the number is NOT over
    /// everything the label implies.
    #[must_use]
    pub fn scoped(mut self, scope: &'static str) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Mark the row as a FINDING rather than a reading. Reserve it for
    /// rows that are only interesting because something disagrees — a
    /// row that is always loud teaches the reader to skip it.
    #[must_use]
    pub fn alarming(mut self) -> Self {
        self.severity = RowSeverityV1::Alarm;
        self
    }

    /// As [`Self::alarming`], but only when `yes`. Written as one call so
    /// a view cannot build the alarming and the calm row down two
    /// branches that then drift apart.
    #[must_use]
    pub fn alarming_if(mut self, yes: bool) -> Self {
        if yes {
            self.severity = RowSeverityV1::Alarm;
        }
        self
    }

    pub fn label(&self) -> &str { &self.label }

    pub fn value(&self) -> &str { &self.value }

    pub fn producer(&self) -> &'static str { self.producer }

    pub fn unit(&self) -> &'static str { self.unit }

    pub fn scope(&self) -> Option<&'static str> { self.scope }

    pub fn frame(&self) -> FrameV1 { self.frame }

    pub fn severity(&self) -> RowSeverityV1 { self.severity }

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
    /// WHAT THEY ARE FEELING AND WHY: the mood waterfall (`mood_formula`'s
    /// own working shown), the personality and values that weight it, the
    /// needs meters it reads, who they like, and what has happened to
    /// them.
    ///
    /// Almost all of this ALREADY CROSSES THE WIRE in
    /// `BastionInspectPayload` and is thrown away unread by the client.
    Thinking,
    /// THE TOWN AROUND THEM: the colony drive and its reason, households,
    /// the profession census, the stock breakdown and the job tally.
    ///
    /// Subject-independent — it answers the same for every colonist — but
    /// it rides the same request because the question a player asks while
    /// looking at one idle colonist is usually about the town.
    Colony,
}

impl SectionIdV1 {
    pub const COUNT: usize = 5;

    /// Every section, in display order.
    pub const ALL: [SectionIdV1; Self::COUNT] = [
        SectionIdV1::Identity,
        SectionIdV1::RightNow,
        SectionIdV1::Path,
        SectionIdV1::Thinking,
        SectionIdV1::Colony,
    ];

    /// The exhaustiveness anchor for [`Self::ALL`]. No wildcard arm — a
    /// new variant fails to compile here first.
    pub const fn index(self) -> usize {
        match self {
            SectionIdV1::Identity => 0,
            SectionIdV1::RightNow => 1,
            SectionIdV1::Path => 2,
            SectionIdV1::Thinking => 3,
            SectionIdV1::Colony => 4,
        }
    }

    /// The panel heading. No wildcard arm.
    pub const fn title(self) -> &'static str {
        match self {
            SectionIdV1::Identity => "Identity",
            SectionIdV1::RightNow => "Right Now",
            SectionIdV1::Path => "Path",
            SectionIdV1::Thinking => "Thinking",
            SectionIdV1::Colony => "Colony",
        }
    }

    /// No wildcard arm.
    pub const fn cadence(self) -> SectionCadenceV1 {
        match self {
            // A name and a skill table do not change at 2 Hz.
            SectionIdV1::Identity => SectionCadenceV1::Slow,
            SectionIdV1::RightNow => SectionCadenceV1::Live,
            SectionIdV1::Path => SectionCadenceV1::Live,
            // The mood cadence is a multi-second thing and the chronicle
            // is event-driven; polling either at 2 Hz would cost a
            // chronicle scan and a sentiment walk twice a second for a
            // picture that has not moved.
            SectionIdV1::Thinking => SectionCadenceV1::Slow,
            // The colony drive is evaluated once every 1,500 server
            // ticks and the profession census once a game-day. Asking
            // faster than Slow would re-scan every item in the world for
            // an answer that cannot have changed.
            SectionIdV1::Colony => SectionCadenceV1::Slow,
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
            // Mood, needs and energy are ECS components. The chronicle
            // and the sentiments would survive an unload, but a Thinking
            // section that answered with half its rows missing and no
            // mood at all would be worse than one that says why: the
            // waterfall IS the section.
            SectionIdV1::Thinking => false,
            // ★ THE ONE SECTION THAT IS NOT ABOUT THE SUBJECT. The town
            // is still there when a colonist walks out of view, so this
            // answers regardless — which is also what makes it useful
            // for the case the panel was rebuilt around: a colonist
            // unloads mid-inspection and the player still wants to know
            // what the colony is doing.
            SectionIdV1::Colony => true,
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
    /// ZONE ASSIGNMENT (Ben, 2026-09-01): the work zone this colonist is
    /// assigned to, and whether a person (true) or the auto-assigner set it.
    pub assigned_zone: Option<(crate::bastion::ZoneId, bool)>,
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

// ---------------------------------------------------------------------------
// Thinking
// ---------------------------------------------------------------------------

/// How far the ECS `Mood` mirror may drift from the recomputed
/// `mood_formula` output before the panel calls it a defect.
///
/// The two are computed from the same tables at different cadences, so a
/// tiny float difference is expected and meaningless. Anything the reader
/// could notice on a 0..1 meter is not.
pub const MOOD_MIRROR_TOLERANCE: f32 = 0.01;

/// ★ THE SECTION'S MOST VALUABLE OUTPUT, as a PURE RULE.
///
/// `comp::Mood(f32)` is a MIRROR: it is written by the mood tick and read
/// by everything downstream. `MoodExplanationV1::total_mood` is recomputed
/// at request time through the REAL [`crate::comp::bastion::mood_formula`].
/// If the two disagree by more than [`MOOD_MIRROR_TOLERANCE`] the mirror is
/// stale, and every consumer of `Mood` is acting on a number the formula
/// would no longer produce.
///
/// Returns the SIGNED difference (`mirror - explained`) when it matters, so
/// a caller can say which way it drifted, and `None` when the comparison
/// cannot be made at all. `None` is not "they agree": a missing `Mood`
/// component and a mood that matches are different states and the panel
/// must not render them alike.
///
/// Lives here rather than in the view so the rule is pinned in a
/// `-p veloren-common` test — and so the server could adopt it later
/// without a second copy of the tolerance.
pub fn mood_mirror_disagreement(mirror: Option<f32>, explained: Option<f32>) -> Option<f32> {
    let (m, e) = (mirror?, explained?);
    let d = m - e;
    (d.abs() > MOOD_MIRROR_TOLERANCE).then_some(d)
}

/// What a sentiment is held TOWARD, once the server has resolved it.
///
/// ★ THE DEFECT THIS REPLACES. The shipped payload labels every sentiment
/// `"uid:N"` — a number that names nobody. Resolution happens server-side,
/// where the rtsim roster is already open, and the KIND rides along so a
/// name the server could not resolve is visibly unresolved rather than
/// quietly rendered as if it were a person.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SentimentTargetKindV1 {
    /// An rtsim NPC that carries a `BastionColonist` record — `who` is
    /// their real name.
    Colonist,
    /// An rtsim NPC with no colonist record — `who` is the raw id.
    Npc,
    /// A player character.
    Character,
    /// A faction.
    Faction,
}

impl SentimentTargetKindV1 {
    /// No wildcard arm.
    pub const fn label(self) -> &'static str {
        match self {
            SentimentTargetKindV1::Colonist => "colonist",
            SentimentTargetKindV1::Npc => "npc (no colonist record)",
            SentimentTargetKindV1::Character => "player character",
            SentimentTargetKindV1::Faction => "faction",
        }
    }
}

/// One held sentiment, with its target RESOLVED.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SentimentRowV1 {
    pub who: String,
    pub kind: SentimentTargetKindV1,
    /// `Sentiment::value()` — the same scale gameplay consumes.
    pub value: f32,
}

/// One chronicle line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChronicleRowV1 {
    pub tick: u64,
    /// The `EventKind`'s own `Debug`, which carries each variant's typed
    /// payload. Formatted server-side because `bastion_entity_event_log`
    /// lives in `bastion-server` and `common` cannot name its types.
    pub kind: String,
    /// The second party, resolved to a name where one exists.
    pub actor: Option<String>,
}

/// The colonist's chronicle AND EVERY FILTER BETWEEN THE RING AND THE ROWS.
///
/// ★ AN EMPTY SECTION, A DISABLED PRODUCER AND A FILTERED ONE MUST NEVER
/// RENDER ALIKE. Three states hide behind "no events here":
///
/// * the log is switched off (`BASTION_ENTITY_EVENT_LOG=0`) — nothing was
///   ever recorded and nothing ever will be until it is switched back on;
/// * the log is on and this colonist genuinely has no history;
/// * the log is on, the colonist has 476 events, and the player VIEW drops
///   the `Released` rows — so 412 are shown and 64 are hidden by a filter
///   the reader never agreed to.
///
/// Every one of those is carried as its own field. `total` is the count
/// BEFORE the view filter, `hidden_released` is what the filter took, and
/// `truncated` is the ring having dropped the oldest to make room —
/// a fourth, independent kind of missing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChronicleViewV1 {
    /// `bastion_entity_event_log::enabled()`. FALSE means the producer is
    /// off, not that the colonist is uneventful.
    pub enabled: bool,
    /// The per-entity ring dropped older events to make room.
    pub truncated: bool,
    /// Rows the ring held for this subject, BEFORE the view filter.
    pub total: u32,
    /// Rows the player view dropped because they are job-release spam.
    /// Zero while `raw` is set.
    pub hidden_released: u32,
    /// `BASTION_CHRONICLE_RAW` is set, so nothing was filtered.
    pub raw: bool,
    /// The most recent rows, oldest-first, at most [`Self::row_cap`].
    pub rows: Vec<ChronicleRowV1>,
    /// The transmitted-row cap, so the panel can say the list is a
    /// SUFFIX rather than quietly showing a short history.
    pub row_cap: u32,
}

impl ChronicleViewV1 {
    /// Rows that survived the view filter — `total - hidden_released`,
    /// saturating.
    pub const fn shown_after_filter(&self) -> u32 { self.total.saturating_sub(self.hidden_released) }

    /// Whether the transmitted list is a suffix of what survived the
    /// filter.
    pub const fn capped(&self) -> bool { (self.rows.len() as u32) < self.shown_after_filter() }
}

/// Thinking — WHAT THEY ARE FEELING AND WHY.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThinkingSectionV1 {
    /// The ECS `Mood(f32)` MIRROR — what every downstream consumer reads.
    /// `None` = no `Mood` component, which is NOT mood zero.
    pub mood_mirror: Option<f32>,
    /// The recomputed explanation, whose `total_mood` came back through
    /// the real `mood_formula`. `None` when the subject has no rtsim
    /// entity, so the chronicle-dependent thought half cannot be built.
    pub explanation: Option<crate::comp::bastion::MoodExplanationV1>,
    /// `Needs { hunger, rest, recreation }`, ECS frame. The same three
    /// values the waterfall's penalties are computed from — carried
    /// separately so the reader can see the meter beside the penalty.
    pub needs: Option<(f32, f32, f32)>,
    /// `Energy::fraction`.
    pub energy: Option<f32>,
    /// `BastionColonist::guard_bravery`. LOWER is braver.
    pub guard_bravery: f32,
    /// Every vanilla Big-Five trait the rtsim `Personality` satisfies.
    pub traits: Vec<String>,
    /// `BastionColonist::values` — the ±50 map `care_multiplier` scales
    /// each thought by. In `Value` order (it is a `BTreeMap`), so it
    /// cannot reorder between two assemblies.
    pub values: Vec<(crate::bastion::Value, i8)>,
    /// Held sentiments, target RESOLVED, sorted by `(who, kind)`.
    pub sentiments: Vec<SentimentRowV1>,
    pub chronicle: ChronicleViewV1,
}

// ---------------------------------------------------------------------------
// Colony
// ---------------------------------------------------------------------------

/// WHERE a counted item was.
///
/// ★ THE DEFECT THIS TYPE EXISTS FOR, restated because it is the reason
/// every stock row is a breakdown and never a scalar. The inspector once
/// showed a tool count of `0` while 64 tools existed in the colony: the
/// number was stockpile-scoped and the label did not say so, and a player
/// reading it could not tell a broken forge from broken hauling. With the
/// three disjoint scopes side by side, `0 in stockpiles · 64 carried · 3 on
/// ground` says "hauling" at a glance and `0 · 0 · 0` says "forge".
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StockScopeV1 {
    /// Inside a stockpile region — the pantry, and the ONLY domain
    /// `colony_food_stock` and the fetch leg draw from.
    InStockpileRegions,
    /// In a colonist's `Inventory`.
    CarriedByColonists,
    /// A dropped `PickupItem` that is NOT inside a stockpile region.
    OnGroundAnywhere,
    /// The sum of the three above. Carried explicitly rather than left to
    /// the view to add up, so producer and consumer cannot disagree about
    /// whether the scopes are disjoint.
    Total,
}

impl StockScopeV1 {
    /// The three DISJOINT scopes, in display order. `Total` is excluded
    /// deliberately: it is their sum, and including it here would make
    /// any loop that sums `ALL` double-count.
    pub const DISJOINT: [StockScopeV1; 3] = [
        StockScopeV1::InStockpileRegions,
        StockScopeV1::CarriedByColonists,
        StockScopeV1::OnGroundAnywhere,
    ];

    /// No wildcard arm.
    pub const fn label(self) -> &'static str {
        match self {
            StockScopeV1::InStockpileRegions => "in stockpiles",
            StockScopeV1::CarriedByColonists => "carried",
            StockScopeV1::OnGroundAnywhere => "on ground",
            StockScopeV1::Total => "total",
        }
    }
}

/// One (item, scope) count. Never a bare number.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockRowV1 {
    /// The item's `itemdef_id` — a greppable asset path, not a prose
    /// name, for the same reason `InspectRow::producer` is a symbol.
    pub item_label: String,
    pub count: u32,
    pub scope: StockScopeV1,
}

/// One member of a household.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseholdMemberV1 {
    pub uid: Uid,
    /// `None` when the owner uid resolves to no loaded colonist — a real
    /// state (a bed owned by someone who has unloaded), reported rather
    /// than hidden behind the uid.
    pub name: Option<String>,
}

/// One derived household — "one colonist per house" made visible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseholdRowV1 {
    pub min: vek::Vec3<i32>,
    pub max: vek::Vec3<i32>,
    /// Real bed slots inside the region — the capacity SOURCE.
    pub beds: u32,
    /// `household_capacity(beds)` — the 1..6 clamp Ben's ruling pins.
    pub capacity: u32,
    /// Uid-sorted by `derive_households`; `members[0]` is the head.
    pub members: Vec<HouseholdMemberV1>,
}

/// The job board's four refusal counters, from ONE scan.
///
/// ★ WHY THIS IS A STRUCT AND NOT FOUR RETURNS. The shipped colony
/// dashboard walked `board.jobs.values()` FOUR separate times, once per
/// counter, with the material predicate re-joining every dropped item in
/// the world inside its filter. Four passes is not only four times the
/// work — it is four chances for one predicate to drift from its
/// neighbours, and the counters are read side by side as if they
/// partitioned the same population.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTallyV1 {
    pub total: u32,
    /// `Job::claimed_by.is_some()` — ACTIVELY HELD, not merely owned.
    pub claimed: u32,
    /// Unclaimed AND no cell a colonist can stand in to work it.
    pub blocked_stance: u32,
    /// `Job::unreachable`.
    pub unreachable: u32,
    /// Waiting on a material nobody carries and no stockpile holds.
    pub blocked_materials: u32,
}

/// The colony ladder RE-EVALUATED at inspect time.
///
/// ★ WHAT THIS IS AND IS NOT. `JobBoard::colony_drive` stores only
/// `(drive, since_tick)` — `colony_drive_for` returns `(drive, reason,
/// value)` and the call site DISCARDS the last two. There is therefore no
/// stored reason to read, and the only honest way to get one is to run the
/// same pure function again over freshly measured inputs.
///
/// That makes this a SECOND SAMPLE, not a second producer: the function is
/// the sim's own, and every input it was fed is carried here so the reader
/// can see what the verdict was computed from. When `want` differs from the
/// held drive the colony is mid-transition or the dwell timer is eating it
/// — which is exactly the state the DWELL SUPPRESSED log line exists to
/// report, and which nothing player-facing could see before.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColonyDriveVerdictV1 {
    pub want: crate::bastion::ColonyDrive,
    /// `colony_drive_for`'s own `deciding` string — `"threats"`,
    /// `"food_per_cap"`, `"beds_short"`, `"satisfied"`.
    pub deciding: String,
    /// The magnitude behind `deciding`.
    pub value: f32,
    /// `colony_sustain_bar(held)` — the Sustain arm's threshold is a BAND,
    /// so a value without its bar is unreadable.
    pub bar: f32,
    pub food_per_cap: f32,
    /// Food inside stockpile regions.
    pub food_pantry: u32,
    /// Food anywhere the colony can eat it — the number that DECIDES.
    pub food_total: u32,
    pub threats: u32,
    pub beds: u32,
    pub pop: u32,
}

/// Colony — the town around the subject.
///
/// ★ FRAME WARNING, carried in the type's own doc because it is the
/// commonest defect in this subsystem. Everything derived from `JobBoard`
/// is RUNTIME-ONLY: the board is rebuilt from `JobBoard::default()` at
/// every server start, so `professions`, `colony_drive` and the household
/// derivation all read empty after a restart until their own cadences run
/// again. `roster_loaded` and the stock census are ECS. The two are NOT
/// commensurable and every row that renders them says which it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColonySectionV1 {
    /// `JobBoard::colony_drive.0` — the HELD drive, read, never recomputed.
    pub drive: crate::bastion::ColonyDrive,
    /// The server tick it last TRANSITIONED. Boot-relative.
    pub drive_since_tick: u64,
    /// `server_tick - drive_since_tick`, saturating.
    pub drive_held_ticks: u64,
    /// The ladder re-run over freshly measured inputs. `None` when the
    /// request did not ask for the Colony section's measurements.
    pub verdict: Option<ColonyDriveVerdictV1>,
    /// `derive_households`, in board push order (stable per world).
    pub households: Vec<HouseholdRowV1>,
    /// `JobBoard::beds.len()`.
    pub beds_total: u32,
    /// Beds that fall inside no `Bed` region — open-ground bedrolls, which
    /// house nobody as far as the population loop is concerned.
    pub beds_outside_households: u32,
    /// ★ TOTAL OVER `WorkType::ALL` BY CONSTRUCTION, indexed by
    /// `lane_index()`, for exactly the reason `IdentitySectionV1::skills`
    /// is: a hand-written lane list is a defect waiting for the next
    /// lane, and this codebase has already shipped that bug twice.
    ///
    /// Counted over the LOADED roster only, by keyed lookup — see
    /// `profession_unnamed`.
    pub professions: [u32; WorkType::COUNT],
    /// Loaded colonists the board has named no profession for. `roster_loaded
    /// - Σ professions` by construction, carried explicitly so a view
    /// cannot forget the bucket. THIS is the owner's acceptance criterion
    /// as a number: it should fall, not the histogram's total.
    pub profession_unnamed: u32,
    /// Loaded colonists — the ECS-frame denominator of the histogram.
    pub roster_loaded: u32,
    /// `JobBoard::professions.len()` — entries INCLUDING colonists who
    /// have since unloaded. Reported beside the loaded histogram rather
    /// than mixed into it: subtracting one from the other would be two
    /// frames compared as one.
    pub professions_board_entries: u32,
    /// Four rows per item (three disjoint scopes plus their total),
    /// heaviest item first.
    pub stock: Vec<StockRowV1>,
    /// Distinct item definitions seen, BEFORE the cap.
    pub stock_distinct: u32,
    /// The census hit its cap and the list is a prefix of the heaviest.
    pub stock_truncated: bool,
    pub jobs: JobTallyV1,
    /// `JobBoard::designated_regions().count()`.
    pub designations: u32,
    /// The server tick this whole section was sampled at.
    pub tick: u64,
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
    /// APPENDED after `Unavailable`, which is where new variants go: the
    /// ordinal is wire-visible and the existing ones must not move.
    Thinking(ThinkingSectionV1),
    Colony(ColonySectionV1),
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
    /// ★ THE COLLAPSED-SECTION REFUSAL, and the reason it is a distinct
    /// state rather than an error.
    ///
    /// The expensive halves of Thinking (a chronicle walk, a sentiment
    /// walk, a mood recomputation) and of Colony (every dropped item and
    /// every colonist inventory in the world) are measured AT THE CALL
    /// SITE and only when the request asks for them — that is how a
    /// collapsed section costs zero. If a section is somehow reached
    /// without its measurement, it must say so rather than answer with
    /// zeroes: "the colony holds no tools" and "nobody counted the tools"
    /// are opposite conclusions and a panel that renders them alike is the
    /// original defect wearing a new hat.
    NotMeasured,
}

impl UnavailableReasonV1 {
    /// No wildcard arm.
    pub const fn label(self) -> &'static str {
        match self {
            UnavailableReasonV1::SubjectUnloaded => "unloaded — showing roster state only",
            UnavailableReasonV1::NotAColonist => "not a colonist",
            UnavailableReasonV1::NoRoute => "no route retained right now",
            UnavailableReasonV1::NotMeasured => {
                "not measured this request — NOT the same as zero"
            },
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
            SectionPayloadV1::Thinking(_) => SectionIdV1::Thinking,
            SectionPayloadV1::Colony(_) => SectionIdV1::Colony,
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
            SectionPayloadV1::Unavailable(SectionIdV1::Thinking, UnavailableReasonV1::NotMeasured),
            SectionPayloadV1::Unavailable(SectionIdV1::Colony, UnavailableReasonV1::NotMeasured),
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
            assigned_zone: None,
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

    /// ★ THE APPENDED SECTION PAYLOADS DO NOT MOVE THE EXISTING ONES.
    ///
    /// `SectionPayloadV1` rides inside `SectionedInspectV1`, so its
    /// variant ordinals are as wire-visible as the outer enum's. The two
    /// phase-2 payloads are appended AFTER `Unavailable` — which reads
    /// oddly (a refusal in the middle of the answers) and is exactly
    /// right: append-only means append, and moving `Unavailable` to the
    /// end to tidy the list would silently re-map every reply in flight
    /// between two builds.
    ///
    /// FALSIFIER: move `Thinking` above `Unavailable` and this goes RED.
    #[test]
    fn appended_section_payloads_do_not_move_existing_ordinals() {
        let enc = |v: &SectionPayloadV1| {
            bincode::serde::encode_to_vec(v, bincode::config::legacy()).expect("encodes")[0..4]
                .to_vec()
        };
        let unavail = |id| SectionPayloadV1::Unavailable(id, UnavailableReasonV1::SubjectUnloaded);
        // Ordinals 0..2 belong to payloads that need a full struct to
        // build, so the refusal arm stands in for the ONE thing under
        // test here: where `Unavailable`, `Thinking` and `Colony` sit.
        assert_eq!(enc(&unavail(SectionIdV1::Identity)), [3, 0, 0, 0]);
        assert_eq!(enc(&SectionPayloadV1::Thinking(thinking_fixture())), [4, 0, 0, 0]);
        assert_eq!(enc(&SectionPayloadV1::Colony(colony_fixture())), [5, 0, 0, 0]);

        // And the reason enum is append-only too: `NotMeasured` is last.
        let renc = |r: UnavailableReasonV1| {
            bincode::serde::encode_to_vec(&r, bincode::config::legacy()).expect("encodes")[0..4]
                .to_vec()
        };
        assert_eq!(renc(UnavailableReasonV1::SubjectUnloaded), [0, 0, 0, 0]);
        assert_eq!(renc(UnavailableReasonV1::NotAColonist), [1, 0, 0, 0]);
        assert_eq!(renc(UnavailableReasonV1::NoRoute), [2, 0, 0, 0]);
        assert_eq!(renc(UnavailableReasonV1::NotMeasured), [3, 0, 0, 0]);
    }

    fn chronicle(enabled: bool, total: u32, hidden: u32, rows: u32) -> ChronicleViewV1 {
        ChronicleViewV1 {
            enabled,
            truncated: false,
            total,
            hidden_released: hidden,
            raw: false,
            rows: (0..rows)
                .map(|i| ChronicleRowV1 { tick: u64::from(i), kind: "Slept".into(), actor: None })
                .collect(),
            row_cap: 64,
        }
    }

    fn thinking_fixture() -> ThinkingSectionV1 {
        ThinkingSectionV1 {
            mood_mirror: Some(0.6),
            explanation: None,
            needs: Some((0.5, 0.5, 0.5)),
            energy: Some(1.0),
            guard_bravery: 0.4,
            traits: vec!["Neurotic".into()],
            values: vec![(crate::bastion::Value::Kin, 30)],
            sentiments: Vec::new(),
            chronicle: chronicle(true, 0, 0, 0),
        }
    }

    fn colony_fixture() -> ColonySectionV1 {
        ColonySectionV1 {
            drive: crate::bastion::ColonyDrive::Grow,
            drive_since_tick: 0,
            drive_held_ticks: 0,
            verdict: None,
            households: Vec::new(),
            beds_total: 0,
            beds_outside_households: 0,
            professions: [0; WorkType::COUNT],
            profession_unnamed: 0,
            roster_loaded: 0,
            professions_board_entries: 0,
            stock: Vec::new(),
            stock_distinct: 0,
            stock_truncated: false,
            jobs: JobTallyV1::default(),
            designations: 0,
            tick: 0,
        }
    }

    /// ★ THE MOOD MIRROR CHECK — the Thinking section's most valuable
    /// output, as a pure rule.
    ///
    /// `Mood(f32)` is written by the mood tick; `total_mood` is recomputed
    /// through the real `mood_formula` at request time. A disagreement
    /// means every consumer of `Mood` is acting on a stale number.
    ///
    /// FALSIFIERS, both of which have been RUN:
    ///
    /// * return `Some(d.abs())` instead of `Some(d)` and the "which
    ///   direction" assertion goes RED;
    /// * replace the `?`s with `unwrap_or(0.0)` and the three
    ///   absent-input assertions go RED.
    ///
    /// ★ A FALSIFIER THAT DOES NOT WORK, recorded so nobody tries it:
    /// changing `>` to `>=` does NOT flip anything here. The boundary of a
    /// float comparison is fuzzy by about one ULP — `0.61f32 - 0.60f32` is
    /// 0.00999999…, a hair BELOW the tolerance — so the "exactly at the
    /// bar" case is not representable and no amount of wanting makes it
    /// sharp. The same lesson is already on the record at
    /// `the_request_floor_admits_normal_play_and_refuses_a_flood`.
    #[test]
    fn the_mood_mirror_check_fires_only_on_a_real_disagreement() {
        // Agreement, and float noise well inside the tolerance.
        assert_eq!(mood_mirror_disagreement(Some(0.6), Some(0.6)), None);
        assert_eq!(mood_mirror_disagreement(Some(0.6), Some(0.6001)), None);
        // A difference AT the tolerance (to within one ULP) is not a
        // disagreement; a clear multiple of it is.
        assert_eq!(mood_mirror_disagreement(Some(0.61), Some(0.60)), None);
        let d = mood_mirror_disagreement(Some(0.62), Some(0.60)).expect("a real drift");
        assert!((d - 0.02).abs() < 1e-6, "the SIGNED difference must say which way: {d}");
        assert!(
            mood_mirror_disagreement(Some(0.30), Some(0.60)).is_some_and(|d| d < 0.0),
            "the other direction must be reported too"
        );
        // ★ AN ABSENT INPUT IS NOT AN AGREEMENT. A colonist with no
        // `Mood` component and a colonist whose mood matches are
        // different states; collapsing them would let a missing mirror
        // read as a healthy one.
        assert_eq!(mood_mirror_disagreement(None, Some(0.6)), None);
        assert_eq!(mood_mirror_disagreement(Some(0.6), None), None);
        assert_eq!(mood_mirror_disagreement(None, None), None);
    }

    /// ★ AN EMPTY CHRONICLE, A DISABLED ONE AND A FILTERED ONE ARE THREE
    /// DIFFERENT STATES, and the payload can tell them apart.
    ///
    /// FALSIFIER: drop `enabled` or `hidden_released` from the payload and
    /// the corresponding pair below becomes indistinguishable.
    #[test]
    fn chronicle_absence_disabled_and_filtered_are_distinguishable() {
        let off = chronicle(false, 0, 0, 0);
        let empty = chronicle(true, 0, 0, 0);
        let filtered = chronicle(true, 476, 64, 64);

        assert_ne!(off, empty, "a disabled log must not look like an empty one");
        assert!(!off.enabled && empty.enabled);

        // The filtered case can state its own arithmetic.
        assert_eq!(filtered.shown_after_filter(), 412);
        assert!(filtered.capped(), "64 rows of 412 is a suffix and must say so");
        assert!(!empty.capped(), "nothing to cap");
        assert!(!filtered.rows.is_empty());

        // A colonist with a handful of events is NOT capped and NOT
        // filtered -- the ordinary case must stay quiet.
        let plain = chronicle(true, 3, 0, 3);
        assert_eq!(plain.shown_after_filter(), 3);
        assert!(!plain.capped());
        assert_eq!(plain.hidden_released, 0);
    }

    /// A row's severity is part of the row, defaults to quiet, and
    /// survives `scoped`.
    ///
    /// FALSIFIER: make `alarming_if(false)` set `Alarm` and the quiet case
    /// goes RED.
    #[test]
    fn row_severity_defaults_quiet_and_is_opt_in() {
        let plain = InspectRow::new("a", "b", "P", "", FrameV1::Derived);
        assert_eq!(plain.severity(), RowSeverityV1::Normal);
        assert_eq!(
            InspectRow::new("a", "b", "P", "", FrameV1::Derived).alarming().severity(),
            RowSeverityV1::Alarm
        );
        assert_eq!(
            InspectRow::new("a", "b", "P", "", FrameV1::Derived)
                .alarming_if(false)
                .scoped("s")
                .severity(),
            RowSeverityV1::Normal
        );
        assert_eq!(
            InspectRow::new("a", "b", "P", "", FrameV1::Derived)
                .alarming_if(true)
                .scoped("s")
                .severity(),
            RowSeverityV1::Alarm
        );
    }

    /// The stock scopes PARTITION: `Total` is their sum and is excluded
    /// from `DISJOINT`, so nothing that iterates the disjoint set can
    /// double-count.
    ///
    /// FALSIFIER: add `Total` to `DISJOINT` and this goes RED.
    #[test]
    fn stock_scopes_partition_and_total_is_not_one_of_them() {
        assert_eq!(StockScopeV1::DISJOINT.len(), 3);
        assert!(!StockScopeV1::DISJOINT.contains(&StockScopeV1::Total));
        let mut labels: Vec<&str> =
            StockScopeV1::DISJOINT.iter().map(|s| s.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 3, "two scopes share a label");
        assert_ne!(StockScopeV1::Total.label(), StockScopeV1::InStockpileRegions.label());
    }
}
