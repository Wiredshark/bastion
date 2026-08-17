//! bastion (Project Bastion): ECS marker components for the overseer
//! interaction surface (B2a).

use serde::{Deserialize, Serialize};
use specs::{Component, NullStorage};

/// Marks the entity currently selected by the overseer (client-side; at most
/// a handful at once). Drives the inspection HUD and feeds the B1.6 cutaway
/// targets, replacing that block's focus+debug-marker stubs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionSelected;

impl Component for BastionSelected {
    type Storage = NullStorage<Self>;
}

/// A colony member (B3): the ECS mirror of the rtsim-side
/// [`crate::bastion::BastionColonist`], attached when the NPC promotes to a
/// loaded entity. Synced to clients (overhead markers, box-select, roster).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Colonist(pub crate::bastion::BastionColonist);

impl Component for Colonist {
    // Synced to clients → needs change-tracked storage.
    type Storage = specs::DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

/// Ownership tag: this entity belongs to THE player colony. Server-side only;
/// B2b's God-mode target restriction reads it.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerColony;

impl Component for PlayerColony {
    type Storage = NullStorage<Self>;
}

/// REQ-0074: short-lived server-authored proof that a colonist is traversing
/// a constructed, route-owned ladder rather than naturally climbing rock.
///
/// The normal `CharacterState::Climb` still owns movement, contact, collision,
/// skill-adjusted speed, interruption, and exit behavior. This token only
/// distinguishes the ladder energy contract and expires unless the validated
/// route transaction refreshes it every tick.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstructedLadderTraversal {
    pub route_owner: crate::uid::Uid,
    pub rung: vek::Vec3<i32>,
    pub expires_at: f64,
}

impl Component for ConstructedLadderTraversal {
    type Storage = specs::DenseVecStorage<Self>;
}

/// Stage-1 B5.8: the single shared movement-owner discriminator for a
/// route-owned off-mesh traversal.
///
/// This component carries no locomotion implementation. Agent/Chaser may own
/// only [`LinkApproach`](BastionTraversalMode::LinkApproach); every later live
/// mode excludes ordinary AI intent until the traversal task atomically
/// completes or aborts. Character behavior and physics remain authoritative
/// subordinate executors of Controller actions/contact.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BastionTraversalMode {
    LinkApproach,
    QueuedForLink,
    Reserved,
    TraversingLink,
    FrontierWork,
    ConfirmingExit,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BastionMovementWriter {
    AgentChaser,
    BastionTraversalTask,
    Orca,
    GenericGoto,
    GenericSoftSteering,
}

impl BastionTraversalMode {
    pub fn allows_agent_pathing(self) -> bool { matches!(self, Self::LinkApproach) }

    pub fn owns_movement_intent(self) -> bool { !self.allows_agent_pathing() }

    pub fn allows_writer(self, writer: BastionMovementWriter) -> bool {
        match self {
            Self::LinkApproach => matches!(writer, BastionMovementWriter::AgentChaser),
            Self::QueuedForLink
            | Self::Reserved
            | Self::TraversingLink
            | Self::FrontierWork
            | Self::ConfirmingExit => {
                matches!(writer, BastionMovementWriter::BastionTraversalTask)
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionTraversalOwnership {
    pub link_id: u64,
    pub route_owner: crate::uid::Uid,
    pub reserved_member: crate::uid::Uid,
    pub mode: BastionTraversalMode,
    /// Route-local terrain proof/fingerprint. PATH-1's eventual global terrain
    /// generation is not claimed by this Stage-1 adapter.
    pub terrain_revision: u64,
    /// bastion (M3, read-only inspection): fair-queue position at
    /// observation — 0 = head. `None` = not queued (a live task's reserved
    /// member has left the queue behind).
    pub queue_position: Option<u32>,
    /// bastion (M3): the member's fair-order ticket tick (the
    /// `(enqueue_tick, uid)` key's first half). `None` = not queued.
    pub queue_enqueue_tick: Option<u64>,
    /// bastion (M3): the link's reservation generation (head handover
    /// count) at observation.
    pub reservation_generation: u64,
    /// bastion (M3): total members queued on the link at observation.
    pub queue_len: u32,
}

impl Component for BastionTraversalOwnership {
    type Storage = specs::DenseVecStorage<Self>;
}

/// Need clocks, 1.0 = fully satisfied, 0.0 = starved/exhausted/miserable.
/// Attached in B3; decay + satisfaction behavior land in B7.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,
    pub rest: f32,
    pub recreation: f32,
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 1.0,
            rest: 1.0,
            recreation: 1.0,
        }
    }
}

impl Component for Needs {
    type Storage = specs::DenseVecStorage<Self>;
}

/// bastion (AUTON-0, row 48; AUTON-2 unification, 2026-08-08): the
/// arbiter's drive — WHAT a colonist's autonomy layer has decided it is
/// doing. Utility-AI shape (The Sims/RimWorld prior art per the
/// packet): score → pick max → COMMIT. `Personal` is the self-job
/// drive — covers all three `is_labor_hold_self_job` kinds
/// (`RestAt`/`EatFrom`/`Despond`), NOT just needs in the literal sense
/// (Despond is a breakdown, not a need; the name covers the whole
/// self-job-execution category on purpose, not "Need", which wouldn't
/// name its own third member). Flat, deliberately: carries no `JobId`
/// or job kind — `active_jobs`/`board.jobs` already own that fact, and
/// a `Personal(kind)` variant would be a second source of truth for
/// something the job board already tracks (self-jobs get a fresh job
/// id per retry, so a carried kind could go stale the moment a retry
/// swaps the underlying job while the Drive itself hasn't changed).
/// Work carries no JobId either, for the same reason — the ActiveJob
/// comp IS the work handle.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Drive {
    Work,
    Flee,
    Idle,
    Personal,
}

/// bastion (AUTON-0): the per-colonist arbiter state — the current
/// drive, the same-tier commitment deadline (anti-thrash hysteresis;
/// higher-tier Flee preemption ignores it per-tick), and the last
/// scored urgencies (work, flee, idle) as REPORTED telemetry.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arbiter {
    pub current: Drive,
    pub committed_until: f64,
    pub last_scores: (f32, f32, f32),
    /// bastion (CHOP-PROGRESS-INDICATOR, row 51.61): the colonist's CURRENT
    /// work job + its progress fraction (0..1), for the UI-4 inspector's
    /// "what are they doing" line. `None` = not on a progress-bearing work
    /// job (idle, or a self-job like rest/eat/haul that completes without a
    /// progress bar). REPORTED display state only — the sim never reads it;
    /// written by the work tick, ridden to the client on BastionInspectInfo.
    pub activity: Option<(crate::bastion::WorkType, f32)>,
    /// bastion (AUTON-2 unification, site 4/6, row 50, 2026-08-09): a
    /// SUSPENDED self-job (RestAt/EatFrom/Despond) this colonist still
    /// owns but isn't currently executing — set when a higher-priority
    /// self-job (or a genuinely unreachable target) bumps it out of
    /// `ActiveJob`, cleared on reclaim or staleness-discard. A POINTER
    /// only — the job's own data (bed_pos, item, and critically Despond's
    /// `until` deadline) never leaves `JobKind` itself; this field exists
    /// so severity computation (`personal_urgency`'s sticky branch) can
    /// see "I still have unfinished business" in O(1) without a board-
    /// wide scan for `claimed_by == this uid`, which is the ONLY reason
    /// this needs to be cached at all — `claimed_by` staying `Some(uid)`
    /// on the suspended job (never cleared to `None`) is what actually
    /// keeps it alive and immune to the orphan sweep; this field is
    /// purely an index into that fact, not a second source of truth for
    /// what the job IS.
    pub pending_self_job: Option<crate::bastion::JobId>,
}

impl Default for Arbiter {
    fn default() -> Self {
        Self {
            current: Drive::Idle,
            committed_until: 0.0,
            last_scores: (0.0, 0.0, 0.0),
            activity: None,
            pending_self_job: None,
        }
    }
}

impl Component for Arbiter {
    type Storage = specs::DenseVecStorage<Self>;
}

/// Mood aggregate, 0.0 (breakdown) ..= 1.0 (content). B7 feeds it.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mood(pub f32);

impl Default for Mood {
    fn default() -> Self { Self(0.6) }
}

impl Component for Mood {
    type Storage = specs::DenseVecStorage<Self>;
}

/// bastion (B7-0, row 44): decay all three meters by `dt` game-seconds,
/// saturating at 0.0. Pure; the caller owns the cadence (per-tick,
/// dt-scaled — decay is rate × time, cadence-independent).
pub fn decay_needs(needs: &mut Needs, dt: f32, cfg: &crate::bastion::MoodConfig) {
    needs.hunger = (needs.hunger - cfg.hunger.decay_per_sec * dt).max(0.0);
    needs.rest = (needs.rest - cfg.rest.decay_per_sec * dt).max(0.0);
    needs.recreation = (needs.recreation - cfg.recreation.decay_per_sec * dt).max(0.0);
}

/// bastion (ITEM 11): decay all three AND report the recreation comfort
/// crossing, for callers that want the edge witnessed.
///
/// WHY A SECOND ENTRY POINT rather than a flag on `decay_needs`: that
/// function is called on every colonist every tick and its signature is
/// depended on by tests and by the determinism arithmetic. This wraps it
/// — same call, same result — and returns the edge, so a caller that
/// wants the witness opts in and every existing caller is untouched.
///
/// RECREATION IS THE ONE-WAY RATCHET: it decays at `decay_per_sec`, feeds
/// a mood penalty through `shortfall`, and NOTHING in the codebase raises
/// it (hunger has `EatFrom`, rest has `RestAt`; `PendingNeed` has no
/// Recreate arm and recreation's interrupt is 0 = never preempts). The
/// crossing is therefore a ONCE-PER-COLONIST-PER-RUN event, which is
/// exactly what makes it worth an edge emit rather than a state check.
pub fn decay_needs_witnessed(
    needs: &mut Needs,
    dt: f32,
    cfg: &crate::bastion::MoodConfig,
) -> bool {
    let before = needs.recreation;
    decay_needs(needs, dt, cfg);
    crossed_comfort_downward(before, needs.recreation, cfg.recreation.comfort)
}

/// bastion (B7-0): a need's penalty basis — nonzero only BELOW the
/// comfort band, so a topped-up colonist is unperturbed and a starving
/// one is heavily penalized. Continuous (mood tracks pressure smoothly).
pub fn shortfall(value: f32, comfort: f32) -> f32 { (comfort - value).max(0.0) }

/// ITEM 11 (ITEM11-RECREATION-READ.md): did this need cross BELOW its comfort
/// band on this tick — the edge, not the state?
///
/// WHY AN EDGE AND WHY HERE. `recreation` decays at 0.0002/sec and NOTHING in
/// the codebase raises it: hunger has `EatFrom`, rest has `RestAt`,
/// recreation has a decay term, a −0.15 mood penalty and no producer. From
/// 1.0 that is **3000 sim-seconds** to reach comfort 0.4, while every current
/// fixture runs 2400–3600 sim-seconds TOTAL — which is exactly why the drag
/// has never been observed. Endurance v2 is the first run long enough.
///
/// The caller emits on `true`, so a multi-hour run reports the crossing as a
/// FACT instead of leaving it to be inferred from a mood number later. Pure
/// and edge-shaped so it fires ONCE per crossing rather than every tick below
/// the band — the same budgeted-diag discipline as the status stamp's
/// edge rule.
pub fn crossed_comfort_downward(before: f32, after: f32, comfort: f32) -> bool {
    before >= comfort && after < comfort
}

/// bastion (B7-0): a thought's decayed contribution — linear to zero
/// over its lifetime, a PURE function of `(deposit_time, now)` (no
/// per-tick state, no drift; the determinism house invariant).
pub fn thought_decay(magnitude: f32, deposit: f64, now: f64, lifetime: f64) -> f32 {
    if lifetime <= 0.0 {
        return 0.0;
    }
    let age = (now - deposit).max(0.0);
    if age >= lifetime {
        0.0
    } else {
        magnitude * (1.0 - age / lifetime) as f32
    }
}

/// bastion (B7-0): THE mood formula (design §3 — RimWorld's base+Σ,
/// named prior art): `clamp01(base + Σ w_need·shortfall(need) +
/// thought_sum)`. Order-free (addition commutes); RECOMPUTED each
/// cadence, never integrated across ticks (no float accumulation). The
/// thought term arrives summed (the server owns the chronicle query —
/// the kind table keys on rtsim's `ChronicleKind`, which common cannot
/// see; the formula is layering-agnostic by taking the sum).
pub fn mood_formula(cfg: &crate::bastion::MoodConfig, needs: &Needs, thought_sum: f32) -> f32 {
    (cfg.mood_base
        + cfg.hunger.weight * shortfall(needs.hunger, cfg.hunger.comfort)
        + cfg.rest.weight * shortfall(needs.rest, cfg.rest.comfort)
        + cfg.recreation.weight * shortfall(needs.recreation, cfg.recreation.comfort)
        + thought_sum)
        .clamp(0.0, 1.0)
}

/// engine-list T3.54 (mood explainability): the 3 need IDs
/// [`mood_formula`] weighs — a fixed sort key for [`MoodExplanationV1`],
/// not a new needs system (do not confuse with [`crate::bastion::Need`],
/// the unrelated FOCUS-scorer personal-need vocabulary).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MoodNeedId {
    Hunger,
    Rest,
    Recreation,
}

/// One need's penalty subtotal, recomputed (not cached) so display can
/// never drift from the authoritative formula.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeedPenaltyV1 {
    pub need: MoodNeedId,
    pub value: f32,
    pub comfort: f32,
    pub weight: f32,
    /// `weight * shortfall(value, comfort)` — the exact term `mood_formula`
    /// sums.
    pub penalty: f32,
}

/// One thought's decayed, care-scaled contribution — one row per
/// qualifying [`rtsim::data::ChronicleEvent`] `thought_sum` folds in,
/// kept individually here instead of pre-summed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThoughtContributionV1 {
    /// [`rtsim::data::ChronicleEvent::seq`] — the source event id.
    pub source_event_id: u64,
    /// The `ChronicleKind` discriminant driving this thought (common
    /// cannot name `rtsim::data::ChronicleKind` directly — cross-crate
    /// boundary `thought_sum` already has; carried as its stable u32
    /// wire tag instead).
    pub thought_id: u32,
    pub base_magnitude: f32,
    pub care_multiplier: f32,
    /// The final decayed, care-scaled term this thought adds to the sum.
    pub contribution: f32,
}

/// One threshold `mood_formula` reads — the comfort bands, surfaced so
/// "why is this colonist unhappy" reads the same numbers the formula
/// does, not a paraphrase.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoodThresholdV1 {
    pub need: MoodNeedId,
    pub comfort: f32,
}

/// engine-list T3.54: `MoodExplanationV1` — per-need penalties, active
/// thoughts, care multipliers (folded into each
/// [`ThoughtContributionV1`], not a separate list — a multiplier without
/// its thought is meaningless), and thresholds. Canonically sorted per
/// field (needs by [`MoodNeedId`], thoughts by
/// `(source_event_id, thought_id)`, thresholds by [`MoodNeedId`]) so two
/// independent constructions of the same input are byte-identical.
/// Diagnostic-only: nothing here is authoritative state — `mood_formula`
/// alone remains the source of truth, this is its own working shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoodExplanationV1 {
    pub snapshot_tick: u64,
    pub actor: crate::rtsim::Actor,
    pub needs: Vec<NeedPenaltyV1>,
    pub thoughts: Vec<ThoughtContributionV1>,
    pub thresholds: Vec<MoodThresholdV1>,
    /// The formula's own output, recomputed from `needs`/`thoughts` at
    /// construction time (never cached) so display and authority cannot
    /// diverge silently.
    pub total_mood: f32,
}

impl MoodExplanationV1 {
    /// Builds the needs/thresholds halves and recomputes `total_mood` via
    /// the real [`mood_formula`] fed the caller's `thought_sum` — the SAME
    /// value `mood_formula` was actually driven with, never re-derived by
    /// summing `thoughts` (that sum lacks `thought_sum`'s Neumaier
    /// compensation and could drift a ULP; diagnostic fields must never be
    /// able to move the authoritative number). The thoughts half
    /// (chronicle-dependent) is server-only; callers pass it in already
    /// built (`bastion-server::bastion_mood::thought_contributions`) —
    /// this function still re-sorts it, since sort order is part of the
    /// DTO's own contract, not the caller's to guarantee.
    pub fn build(
        snapshot_tick: u64,
        actor: crate::rtsim::Actor,
        cfg: &crate::bastion::MoodConfig,
        needs: &Needs,
        thought_sum: f32,
        mut thoughts: Vec<ThoughtContributionV1>,
    ) -> Self {
        thoughts.sort_by_key(|t| (t.source_event_id, t.thought_id));
        let need_penalty = |need: MoodNeedId, value: f32, comfort: f32, weight: f32| NeedPenaltyV1 {
            need,
            value,
            comfort,
            weight,
            penalty: weight * shortfall(value, comfort),
        };
        let mut needs_out = vec![
            need_penalty(MoodNeedId::Hunger, needs.hunger, cfg.hunger.comfort, cfg.hunger.weight),
            need_penalty(MoodNeedId::Rest, needs.rest, cfg.rest.comfort, cfg.rest.weight),
            need_penalty(MoodNeedId::Recreation, needs.recreation, cfg.recreation.comfort, cfg.recreation.weight),
        ];
        needs_out.sort_by_key(|n| n.need);
        let mut thresholds = vec![
            MoodThresholdV1 { need: MoodNeedId::Hunger, comfort: cfg.hunger.comfort },
            MoodThresholdV1 { need: MoodNeedId::Rest, comfort: cfg.rest.comfort },
            MoodThresholdV1 { need: MoodNeedId::Recreation, comfort: cfg.recreation.comfort },
        ];
        thresholds.sort_by_key(|t| t.need);
        let total_mood = mood_formula(cfg, needs, thought_sum);
        Self { snapshot_tick, actor, needs: needs_out, thoughts, thresholds, total_mood }
    }
}

/// bastion (B-AG3 slice 1): the care multiplier is CLAMPED — a stack of
/// scorned values can mute a thought to a quarter, never erase it; a
/// stack of held values can quadruple it, never explode it.
pub const CARE_MIN: f32 = 0.25;
pub const CARE_MAX: f32 = 4.0;
/// bastion (B-AG3 slice 1): a Neurotic colonist (vanilla Big-Five trait,
/// public `Personality::is` API) feels NEGATIVE thoughts half again as
/// hard — the one temperament term this slice consumes (DF/RimWorld's
/// standard neuroticism→bad-thought amplification).
pub const NEUROTIC_NEGATIVE_AMP: f32 = 1.5;

/// bastion (B-AG3 slice 1): how much THIS colonist cares about one
/// thought — the personalized multiplier on the thought's table weight.
/// `values` is the colonist's ±50 weight map; `affinities` is the
/// thought-kind's `(Value, affinity)` row (the ChronicleKind→Value table
/// lives server-side — this is the pure math, layering-agnostic exactly
/// like [`mood_formula`]'s summed thought term). Empty values OR an
/// empty affinity row → exactly 1.0 (+ the neurotic amp if applicable):
/// the pre-B-AG3 formula for unvalued colonists, bit-for-bit when
/// non-neurotic. PURE — no state, no rng; two colonists differing only
/// in `values` produce different multipliers from the SAME thought (the
/// slice's whole point).
/// bastion (FOCUS-0-DERIVE, row 43.1): the derived per-colonist NEED
/// WEIGHT — how much THIS colonist's mind makes them care about each
/// personal [`crate::bastion::Need`], from their rolled
/// [`crate::bastion::Value`] weights + vanilla Big-Five traits. Baseline 1.0; a
/// value-mapped need scales 1 + weight/50 (so ±50 spans 0..2); `Socialize`
/// reads the boolean trait API at 3 levels (Extroverted/Sociable 1.5,
/// Introverted 0.5, else 1.0 — the architect's no-vanilla-getter ruling);
/// `Drink`/`AdmireArt`/`Learn` have no clean correlate and STAY 1.0
/// (the design's degrade-gracefully law — no forced weak mapping, no
/// invented Value). Clamped 0..=2. PURE — a FOCUS-1 scorer eventually
/// consumes this; nothing does yet (this block produces + proves only).
pub fn derive_need_weight(
    need: crate::bastion::Need,
    personality: &crate::rtsim::Personality,
    values: &std::collections::BTreeMap<crate::bastion::Value, i8>,
) -> f32 {
    use crate::{
        bastion::{Need, Value},
        rtsim::PersonalityTrait,
    };
    let from_value =
        |v: Value| -> f32 { 1.0 + values.get(&v).copied().map_or(0.0, |w| f32::from(w) / 50.0) };
    let w = match need {
        // The near-1:1 vocabulary correspondences (the mapping is the
        // enums' own design — Pray↔Piety, Family↔Kin, Craft↔Craft,
        // SeeAnimals↔Nature, Acquire↔Wealth, Fight↔Glory).
        Need::Pray => from_value(Value::Piety),
        Need::Family => from_value(Value::Kin),
        Need::Craft => from_value(Value::Craft),
        Need::SeeAnimals => from_value(Value::Nature),
        Need::Acquire => from_value(Value::Wealth),
        Need::Fight => from_value(Value::Glory),
        // Temperament-derived: the boolean-trait 3-level.
        Need::Socialize => {
            if personality.is(PersonalityTrait::Extroverted)
                || personality.is(PersonalityTrait::Sociable)
            {
                1.5
            } else if personality.is(PersonalityTrait::Introverted) {
                0.5
            } else {
                1.0
            }
        },
        // No clean correlate — baseline (degrade gracefully; never
        // force a weak mapping).
        Need::Drink | Need::AdmireArt | Need::Learn => 1.0,
    };
    w.clamp(0.0, 2.0)
}

/// bastion (AUTON-2, row 50): the preempt-threshold SAFETY FLOOR — even
/// the hardiest possible colonist keeps a live preempt-to-eat edge above
/// zero (Opus's hard guard: the stagger WIDENS the recoverable band, it
/// never disables B7-2's backstop). At hunger decay 0.0004/s a 0.05
/// threshold still leaves ~2 sim-minutes of margin before empty.
pub const INTERRUPT_FLOOR: f32 = 0.05;

/// bastion (AUTON-2, row 50): the TRAIT-STAGGER — one colonist's
/// EFFECTIVE preempt threshold for a need (the per-colonist form of
/// `NeedTuning.interrupt`, the [`care_factor`] modulation pattern).
/// Dutiful/hardy colonists (Craft/Tradition-valuing, Conscientious)
/// tolerate a DEEPER deficit before abandoning work (lower threshold);
/// anxious ones (Neurotic, anti-valuing) preempt EARLIER (higher). The
/// spread is the death-spiral defense: a shortage never yanks the whole
/// crew off the farm at once. Hardiness h ∈ [−1.5, +1.5] (values ±0.5
/// each + Conscientious +0.5 / Neurotic −0.5); eff = base·(1 − 0.4·h),
/// clamped to [`INTERRUPT_FLOOR`-floored, base×1.5]. The `.min(base)`
/// on the floor keeps a base of 0.0 (recreation: never-preempts) at
/// exactly 0.0 — the stagger cannot INVENT a preempt class — and keeps
/// the clamp well-formed if a RON retunes base below the floor. The
/// ceiling base×1.5 (0.3 at the 0.2 default) stays under the 0.5
/// comfort band: nobody preempts while comfortable. PURE + RNG-free
/// (field reads only — the determinism house invariant).
pub fn stagger_interrupt(
    base: f32,
    values: &std::collections::BTreeMap<crate::bastion::Value, i8>,
    conscientious: bool,
    neurotic: bool,
) -> f32 {
    use crate::bastion::Value;
    let mut h = 0.0f32;
    for v in [Value::Craft, Value::Tradition] {
        if let Some(w) = values.get(&v) {
            h += f32::from(*w) / 100.0;
        }
    }
    if conscientious {
        h += 0.5;
    }
    if neurotic {
        h -= 0.5;
    }
    (base * (1.0 - 0.4 * h)).clamp(INTERRUPT_FLOOR.min(base), base * 1.5)
}

/// bastion (UI-4, row 62): the inspector payload — one selected
/// colonist's inner state, server→client on request (the
/// `BastionInspect`/`BastionInspectInfo` wire pair; request/response on
/// selection rather than comp-sync, because it is a single-target
/// on-demand query). Re-packages the reads the harness probes already
/// established; READ-ONLY by construction (the panel writes nothing).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionInspectPayload {
    pub name: String,
    pub hunger: f32,
    pub rest: f32,
    pub recreation: f32,
    pub mood: f32,
    /// (adventurous, worried, sociable_or_extroverted, introverted)
    pub personality4: (bool, bool, bool, bool),
    pub conscientious: bool,
    pub neurotic: bool,
    pub drive: Drive,
    /// (work, flee, idle) — the post-modulation urgencies (AUTON-3).
    pub last_scores: (f32, f32, f32),
    /// bastion (CHOP-PROGRESS-INDICATOR, row 51.61): current work job +
    /// progress fraction (0..1), or `None` if not on a progress-bearing
    /// work job. The inspector renders "Chopping 45%" so a base-cut (or any
    /// work) reads as PROGRESSING before it completes.
    pub activity: Option<(crate::bastion::WorkType, f32)>,
    /// bastion (STATUS-SURFACE): energy fraction (0..1). Energy now gates
    /// climbing (free-climb cap + the REQ-0071 recovery wait), so it is core
    /// meter state alongside hunger/rest/recreation.
    pub energy: f32,
    /// bastion (STATUS-SURFACE): the designed-wait/rescue status, or `None`
    /// when nothing designed is holding the colonist — a MOTIONLESS colonist
    /// with `None` here is the genuine-bug tell the four indistinguishable
    /// pit states needed. Tail-appended per the wire discipline.
    pub status: Option<BastionColonistStatus>,
    /// engine-list T3.54: mood explainability breakdown, or `None` if not
    /// requested/available. Tail-appended per the wire discipline.
    pub mood_explanation: Option<MoodExplanationV1>,
    /// engine-list T3.58: job ownership + Drive telemetry evidence, or
    /// `None` if not requested/available. Tail-appended per the wire
    /// discipline.
    pub ownership: Option<InspectorOwnershipV1>,
}

/// bastion (STATUS-SURFACE): the inspector's colonist status line — the
/// BACKSTOP-OPT designed-wait classifications, surfaced so working-as-designed
/// is distinguishable from broken. An enum (not a string) so the client owns
/// wording/i18n. Display-only by charter: no sim logic ever reads it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BastionColonistStatus {
    /// The REQ-0071 energy-recovery wait (designed, bounded — climbs when
    /// the gate passes).
    RestingToClimb,
    /// Queued/reserved on a single-owner route link (the M2 ladder gate).
    WaitingForLadder,
    /// Re-engage bound exhausted; the independent failsafe net delivers.
    RescueImminent,
    /// Zero-progress cycle detected; the route is being re-planned.
    Replanning,
}

/// bastion (UI-5, row 62.2): the Universal Debug Inspector's TARGET — the
/// generalization of UI-4's colonist-only `Uid`. Either a loaded entity
/// (a colonist, picked under the cursor) or a world CELL (a job /
/// designation / stockpile / farm plot / crop the player clicked). A cell
/// is the most general client-knowable handle: jobs carry server-internal
/// ids the client can't name, so the client sends WHERE it clicked and the
/// server resolves whatever Bastion-tracked object sits there.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BastionInspectTarget {
    Entity(crate::uid::Uid),
    Cell(vek::Vec3<i32>),
    /// bastion (ARC 2 item 10): the colony itself — the one target that is not
    /// a thing you can point at, which is exactly why the dashboard needs it.
    Colony,
}

/// bastion (UI-5, row 62.2): one inspected object's full internal state —
/// the reply payload, dispatched per target kind. The `Colonist` arm reuses
/// UI-4's [`BastionInspectPayload`] verbatim (zero churn to that path); the
/// rest are new debug views. READ-ONLY by construction (the panel writes
/// nothing back to the sim).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BastionInspectKind {
    Colonist(BastionInspectPayload),
    Job(BastionJobInspect),
    Stockpile(BastionStockpileInspect),
    Farm(BastionFarmInspect),
    FellSet(BastionFellSetInspect),
    /// bastion (ARC 2 item 10): the colony as a whole. Rides this enum rather
    /// than a new wire pair because the inspector is already the universal
    /// "ask the server about a thing" channel — a second path would be a
    /// second authority over the same question.
    Colony(BastionColonyInspect),
}

/// bastion (ARC 2 item 10): the colony dashboard's payload.
///
/// Every field is a COUNT OF SOMETHING THE SERVER ALREADY TRACKS. Nothing here
/// is new sim state, and nothing is derived by a formula that lives only in
/// this struct — `food_stock` in particular comes from `colony_food_stock`,
/// the single producer the colony-terminal check also reads, so the dashboard
/// cannot drift from the number that decides the colony is dead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionColonyInspect {
    /// Loaded colonists.
    pub colonists: u32,
    /// `FOOD_DEFS` items inside stockpile regions — the population `EatFrom`
    /// draws from, NOT every item in a pile.
    pub food_stock: u32,
    /// Jobs on the board, total.
    pub jobs_total: u32,
    /// Of those, actively held by a colonist.
    pub jobs_claimed: u32,
    /// Of those, flagged unreachable (the player's "why is nothing happening"
    /// answer).
    pub jobs_unreachable: u32,
    /// Standing designation orders.
    pub designations: u32,
    /// bastion (BLOCKED-MATERIALS row): jobs waiting on a material **nobody
    /// can supply** — not carried by any colonist and not fetchable from a
    /// stockpile.
    ///
    /// This is the number that explains an idle colony. The claim-collapse row
    /// measured a state where all 8 colonists were in Work drive, all 54 jobs
    /// were refused at the material gate, and the dashboard reported
    /// `jobs_claimed=0, jobs_unreachable=0` — a screen that was entirely true
    /// and explained nothing.
    ///
    /// **Counted PER JOB.** The selector's refusal count is per
    /// (job, colonist) pair — 54 jobs × 8 colonists was 432 — and shipping
    /// that would report candidate-evaluations as though they were jobs.
    #[serde(default)]
    pub jobs_blocked_materials: u32,
    /// bastion (COLONY-TICK row): the server `Tick` this sample was taken at.
    ///
    /// WHY: every other field here is a QUANTITY with no TIME. The blind-spot
    /// row could establish that `jobs_blocked_materials` moved, and could
    /// establish when the F3 branch chain was blind, but could not say whether
    /// any sample fell INSIDE a blind window — because the branch emit is
    /// ticked (server log) and this payload was not (driver log). Two series
    /// that cannot be aligned answer no question that spans them.
    ///
    /// THE SAME CLOCK, NOT A PARALLEL ONE: this is `crate::Tick` — the very
    /// resource `bastion F3-BRANCH` stamps (`bastion_jobs.rs`, `Read<'a,
    /// Tick>`), re-exported through `veloren-server`. A second clock that
    /// merely also counts up would make the alignment look sound and be wrong.
    ///
    /// Tail-appended with `serde(default)` per the wire discipline the sibling
    /// inspect struct already documents: an older client decodes 0 rather than
    /// failing, and 0 is distinguishable from any real post-boot tick.
    #[serde(default)]
    pub tick: u64,
}

/// bastion (UI-5): a job / designation-in-progress debug view. The claimant
/// is resolved to a NAME server-side so the client needn't map a `Uid`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionJobInspect {
    pub work: crate::bastion::WorkType,
    pub pos: vek::Vec3<i32>,
    /// 0.0..=1.0 toward completion.
    pub progress: f32,
    pub claimant: Option<String>,
    pub unreachable: bool,
    pub needs_materials: bool,
    /// True for a self-carved stair/ladder access job.
    pub is_access: bool,
    pub stuck_strikes: u8,
    /// bastion (task #55, 2026-07-30): `Some(cell)` when this job's pos
    /// falls inside a designation the auto-access planner gave up on --
    /// names the specific cell that blocked the WHOLE designation, so
    /// inspecting ANY job in a blocked volume answers "blocked by X at
    /// (x,y,z)" rather than only the one cell whose own carve attempt
    /// failed knowing it's unreachable. `None` if this job's designation
    /// isn't blocked (including if `unreachable` is true for some other
    /// reason, e.g. a stuck-strike release rather than a planner refusal).
    #[serde(default)]
    pub blocked_by: Option<vek::Vec3<i32>>,
    /// bastion (guard-generalization row, DECISIONS #47, 2026-08-04): the
    /// tick this job first missed the leave-unclaimed guard (stance-less,
    /// still unclaimed) and has continuously since -- see
    /// `JobBoard::benched_since`'s own doc comment for why this is a
    /// SEPARATE state from `unreachable`/`blocked_by` (a benched job is
    /// deliberately NOT flagged unreachable, so those two fields stay
    /// `false`/`None` for it, and without this field a benched job was
    /// indistinguishable from a healthy one about to be claimed). `None`
    /// if this job isn't currently benched.
    #[serde(default)]
    pub benched_since_tick: Option<u64>,
    /// bastion (ROW B′, 2026-08-04, replaces the withdrawn Row B's
    /// `amnesty_grants_owed: Option<u32>` -- renamed, not just
    /// retyped: a field holding a raw tick must not keep a name that
    /// says "grants," the exact name-vs-content mismatch this campaign
    /// kept finding elsewhere tonight): the sim tick this job becomes
    /// eligible for the amnesty grant again -- mirrors
    /// `common::bastion::Job::benched_until_tick`'s own doc for the full
    /// mechanism (a conjunction with the amnesty grant's `world_changed`
    /// signal, not a plain timer). `None` if this job isn't currently
    /// benched (the overwhelming majority, including every job while
    /// `BASTION_ROWB_BENCH` is unset).
    ///
    /// READ BUDGET (Fable's law, ratified off the mine_cell_diag
    /// bisection, 2026-08-04, and the standard this row's OWN redesign
    /// was built to satisfy after the 48-seed A/B caught Row B's
    /// per-grant iteration manufacturing a threshold crossing on seed
    /// 76): a PLAIN FIELD READ, not a call, and CHEAPER than Row B's
    /// version -- no HashMap lookup at all now, since the field lives
    /// directly on the `Job` already fetched by `bastion_inspect_cell`
    /// (both construction sites, `server/src/lib.rs` +
    /// `server/src/sys/msg/in_game.rs`). `mine_cell_diag`/
    /// `farm_cell_diag` then read `j.benched_until_tick` off the
    /// already-constructed struct -- zero further reads, zero further
    /// calls, per cell.
    #[serde(default)]
    pub benched_until_tick: Option<u64>,
}

/// bastion (UI-5): a stockpile's contents — the 51.64 legibility fix (a
/// painted stockpile finally shows WHAT it holds). Item def id → count,
/// summed over the zone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionStockpileInspect {
    pub contents: Vec<(String, u32)>,
    pub total: u32,
}

/// bastion (UI-5): a farm plot's cultivation state at the sampled cell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionFarmInspect {
    /// Growth stage of the sampled cell's crop sprite, if any (`None` =
    /// tilled/unsown; higher = maturing).
    pub growth: Option<u8>,
    /// Cells in the plot region.
    pub cells: u32,
}

/// bastion (UI-5): a tree fell-set mid-timber — how much of the crown is
/// still standing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionFellSetInspect {
    pub remaining: u32,
    pub total: u32,
}

/// bastion (AUTON-3, row 51): the DRIVE-ORDER guard — Flee's modulated
/// urgency can never sink below this, and the floor sits strictly above
/// Work's modulated CEILING (0.6), so the AUTON-0 safety ordering
/// (Flee > Work > Idle) survives EVERY possible trait roll. Baked in now
/// so B8's live threats inherit a correct invariant, not a landmine.
pub const FLEE_URGENCY_FLOOR: f32 = 0.8;

/// bastion (AUTON-3, row 51): TRAIT-MODULATED drive urgencies — the E2
/// legibility mechanism: two colonists in the SAME state score their
/// drives differently because of WHO THEY ARE. Distinct from AUTON-2's
/// threshold-stagger (when a need becomes urgent); this shapes which
/// drive WINS the arbiter's pick. One value + one personality pair per
/// axis, mirror-simple (the spec's own examples):
/// - WORK  × (1 + 0.4·g), g = Wealth/100 ∈ [−0.5, 0.5] → [0.4, 0.6] (the greedy
///   work harder; ceiling 0.6 < the Flee floor).
/// - FLEE  × (1 − 0.2·b), b = Glory/100 + 0.25·Adventurous − 0.25·Worried ∈
///   [−0.75, 0.75] → [0.85, 1.15], then `.max(floor)` (glory-seekers stand
///   their ground longer — but NEVER below the order guard; bravest possible =
///   0.85 > 0.8 floor > 0.6 ceiling).
/// - IDLE  × (1 + 0.4·s), s = Kin/100 + 0.25·(Sociable∨Extroverted) −
///   0.25·Introverted ∈ [−0.75, 0.75] → [0.07, 0.13] (the social idle richer;
///   ceiling 0.13 < Work's floor 0.4 — work-when-available still always wins,
///   the AUTON-0 liveness contract).
/// Returns (work, flee, idle) — the `Arbiter.last_scores` order. PURE +
/// RNG-free (field reads only — the determinism house invariant).
#[expect(clippy::too_many_arguments)]
pub fn modulated_urgencies(
    base: (f32, f32, f32),
    values: &std::collections::BTreeMap<crate::bastion::Value, i8>,
    adventurous: bool,
    worried: bool,
    sociable: bool,
    introverted: bool,
) -> (f32, f32, f32) {
    use crate::bastion::Value;
    let vw = |v: Value| -> f32 {
        values
            .get(&v)
            .copied()
            .map_or(0.0, |w| f32::from(w) / 100.0)
    };
    let g = vw(Value::Wealth);
    let b =
        vw(Value::Glory) + if adventurous { 0.25 } else { 0.0 } - if worried { 0.25 } else { 0.0 };
    let s =
        vw(Value::Kin) + if sociable { 0.25 } else { 0.0 } - if introverted { 0.25 } else { 0.0 };
    (
        base.0 * (1.0 + 0.4 * g),
        (base.1 * (1.0 - 0.2 * b)).max(FLEE_URGENCY_FLOOR.min(base.1)),
        base.2 * (1.0 + 0.4 * s),
    )
}

/// bastion (AUTON-2 unification, row 50, 2026-08-08): the `Drive::Personal`
/// urgency — folds the self-job need (`is_labor_hold_self_job`:
/// RestAt/EatFrom/Despond) into the SAME score→max→commit machinery
/// `modulated_urgencies` feeds, so GUARD-6's old unconditional bypass
/// becomes a normal (if lopsided) arbiter win instead of a special case.
/// Ceiling `< Flee` (per [`FLEE_URGENCY_FLOOR`]'s own floor, 0.8) so a
/// hostile signal always outranks a nap; floor `== work_urgency` at
/// `severity == 0.0` so an UNMET-but-inactive need never beats available
/// work by construction (the caller's `>` — not `>=` — tie-break in the
/// selection compare leans on this exact equality). Linear, not curved:
/// no live data yet justifies an asymptote over a ramp (AUTON-1/2's own
/// "flat v1, shape the curve later" precedent). PURE + RNG-free.
pub const URGENCY_PERSONAL_CEILING: f32 = 0.95;
pub fn personal_urgency(work_urgency: f32, severity: f32) -> f32 {
    work_urgency + (URGENCY_PERSONAL_CEILING - work_urgency) * severity.clamp(0.0, 1.0)
}

pub fn care_factor(
    values: &std::collections::BTreeMap<crate::bastion::Value, i8>,
    affinities: &[(crate::bastion::Value, f32)],
    neurotic: bool,
    base_weight: f32,
) -> f32 {
    let mut care = 1.0f32;
    for (value, affinity) in affinities {
        if let Some(w) = values.get(value) {
            care += (f32::from(*w) / 50.0) * affinity;
        }
    }
    let care = care.clamp(CARE_MIN, CARE_MAX);
    // The amp applies AFTER the clamp (a maxed-care neurotic feels a bad
    // thought at 6.0×, bounded) and only to negative thoughts — good
    // news is not amplified by anxiety.
    if neurotic && base_weight < 0.0 {
        care * NEUROTIC_NEGATIVE_AMP
    } else {
        care
    }
}

#[cfg(test)]
mod bastion_b70_tests {
    use super::*;

    /// AUTON-3: the drive-order guard pinned. THE FLEE-FLOOR ASSERT
    /// (the packet's load-bearing guard, unit form): the BRAVEST
    /// possible roll (Glory +50, Adventurous, not Worried → b = 0.75)
    /// still scores Flee at 0.85 — above the 0.8 floor and strictly
    /// above the GREEDIEST possible Work ceiling (Wealth +50 → 0.6):
    /// the AUTON-0 ordering survives every trait combination. Plus:
    /// identity (no traits = bases exactly), the zero-preservation
    /// guard (a no-signal flee base of 0.0 stays 0.0 — modulation can
    /// not invent a flee), and Idle's ceiling (most-social 0.13) under
    /// Work's floor (least-greedy 0.4) — work-when-available always
    /// wins.
    #[test]
    fn auton3_drive_order_guard() {
        use crate::bastion::Value;
        use std::collections::BTreeMap;
        let base = (0.5f32, 1.0f32, 0.1f32);
        let none = BTreeMap::new();
        // Identity.
        assert_eq!(
            modulated_urgencies(base, &none, false, false, false, false),
            base
        );
        // Bravest Flee vs greediest Work — the order guard, exact.
        let mut brave = BTreeMap::new();
        brave.insert(Value::Glory, 50i8);
        let (_, flee_min, _) = modulated_urgencies(base, &brave, true, false, false, false);
        let mut greedy = BTreeMap::new();
        greedy.insert(Value::Wealth, 50i8);
        let (work_max, _, _) = modulated_urgencies(base, &greedy, false, false, false, false);
        assert!((flee_min - 0.85).abs() < 1e-6);
        assert!((work_max - 0.6).abs() < 1e-6);
        assert!(flee_min > work_max);
        assert!(flee_min >= FLEE_URGENCY_FLOOR);
        // Zero-preservation: no flee signal (base 0.0) stays 0.0 even
        // for the most fearful roll (modulation cannot INVENT a flee).
        let mut fearful = BTreeMap::new();
        fearful.insert(Value::Glory, -50i8);
        let (_, f0, _) = modulated_urgencies((0.5, 0.0, 0.1), &fearful, false, true, false, false);
        assert_eq!(f0, 0.0);
        // Idle ceiling < Work floor: the liveness contract.
        let mut social = BTreeMap::new();
        social.insert(Value::Kin, 50i8);
        let (_, _, idle_max) = modulated_urgencies(base, &social, false, false, true, false);
        let mut lazy_poor = BTreeMap::new();
        lazy_poor.insert(Value::Wealth, -50i8);
        let (work_min, _, _) = modulated_urgencies(base, &lazy_poor, false, false, false, false);
        assert!((idle_max - 0.13).abs() < 1e-6);
        assert!((work_min - 0.4).abs() < 1e-6);
        assert!(idle_max < work_min);
    }

    /// AUTON-2: the trait-stagger pinned. THE OPUS FLOOR ASSERT (unit
    /// form): the hardiest POSSIBLE colonist (both values +50,
    /// Conscientious, not Neurotic → h = 1.5) still holds a strictly
    /// positive threshold at/above the floor — the preempt-to-eat
    /// backstop survives maximal hardiness. Plus: identity (no traits =
    /// base exactly), monotonicity (hardier ⇒ never higher), the
    /// anxious ceiling (< comfort), and recreation's 0.0 stays 0.0 (the
    /// stagger cannot invent a preempt class).
    #[test]
    fn auton2_stagger_interrupt_floor_and_shape() {
        use crate::bastion::Value;
        use std::collections::BTreeMap;
        let base = 0.2f32;
        let mut hardiest = BTreeMap::new();
        hardiest.insert(Value::Craft, 50i8);
        hardiest.insert(Value::Tradition, 50i8);
        let floor_case = stagger_interrupt(base, &hardiest, true, false);
        assert!(floor_case >= INTERRUPT_FLOOR);
        assert!(floor_case > 0.0);
        // h = 1.5 → 0.2 × (1 − 0.6) = 0.08 exactly.
        assert!((floor_case - 0.08).abs() < 1e-6);
        // Identity: empty values, no traits → base bit-for-bit.
        let none = BTreeMap::new();
        assert_eq!(stagger_interrupt(base, &none, false, false), base);
        // Monotone: each hardiness step never RAISES the threshold.
        let mut mid = BTreeMap::new();
        mid.insert(Value::Craft, 50i8);
        let steps = [
            stagger_interrupt(base, &none, false, true), // anxious
            stagger_interrupt(base, &none, false, false),
            stagger_interrupt(base, &mid, false, false),
            stagger_interrupt(base, &hardiest, false, false),
            stagger_interrupt(base, &hardiest, true, false),
        ];
        for w in steps.windows(2) {
            assert!(w[1] <= w[0]);
        }
        // The anxious ceiling: h = −1.5 → 0.2×1.6 = 0.32, clamped to
        // base×1.5 = 0.3 — still under the 0.5 comfort band.
        let mut anti = BTreeMap::new();
        anti.insert(Value::Craft, -50i8);
        anti.insert(Value::Tradition, -50i8);
        let anxious = stagger_interrupt(base, &anti, false, true);
        assert!((anxious - 0.3).abs() < 1e-6);
        assert!(anxious < 0.5);
        // Recreation's never-preempt base survives every temperament.
        assert_eq!(stagger_interrupt(0.0, &hardiest, true, false), 0.0);
        assert_eq!(stagger_interrupt(0.0, &anti, false, true), 0.0);
    }

    /// B7-0's formula pinned: topped-up == base exactly; the fully
    /// starved case matches the hand-computed value; decay arithmetic is
    /// ITEM 11: the comfort-crossing edge, all four cases plus the boundary.
    /// This is the witness for a need that DECAYS AND IS NEVER RESTORED --
    /// the edge must fire exactly once on the way down and never while the
    /// colonist sits below the band, or a multi-hour run drowns in it.
    #[test]
    fn recreation_comfort_crossing_is_an_edge_not_a_state() {
        let comfort = 0.4;
        // THE CROSSING: above -> below, the one true case.
        assert!(crossed_comfort_downward(0.41, 0.39, comfort));
        // ALREADY BELOW: still decaying, must stay silent -- this is the
        // case that would flood a 3000-second run.
        assert!(!crossed_comfort_downward(0.39, 0.38, comfort));
        // STILL ABOVE: nothing to report.
        assert!(!crossed_comfort_downward(0.99, 0.98, comfort));
        // UPWARD (when a satisfier finally exists, this must NOT read as a
        // downward crossing).
        assert!(!crossed_comfort_downward(0.39, 0.41, comfort));
        // THE BOUNDARY, both sides: landing exactly ON comfort is NOT below
        // (shortfall is 0 there, so mood is unperturbed -- the edge must
        // agree with `shortfall`'s own definition).
        assert!(!crossed_comfort_downward(0.41, comfort, comfort));
        assert!(crossed_comfort_downward(comfort, comfort - 0.001, comfort));
    }

    /// exact and saturates; thought decay is linear-pure; clamp holds.
    #[test]
    fn bastion_mood_formula_exact() {
        let cfg = crate::bastion::MoodConfig::default();
        let full = Needs::default();
        assert_eq!(mood_formula(&cfg, &full, 0.0), cfg.mood_base);
        // Fully starved: clamp01(0.6 − 0.5·0.5 − 0.4·0.5 − 0.15·0.4)
        // = clamp01(0.6 − 0.25 − 0.2 − 0.06) = 0.09.
        let starved = Needs {
            hunger: 0.0,
            rest: 0.0,
            recreation: 0.0,
        };
        assert!((mood_formula(&cfg, &starved, 0.0) - 0.09).abs() < 1e-6);
        // A big negative thought clamps at 0, a big positive at 1.
        assert_eq!(mood_formula(&cfg, &starved, -5.0), 0.0);
        assert_eq!(mood_formula(&cfg, &full, 5.0), 1.0);
        // Decay: exact rate × time, saturating at 0. Computed from cfg's
        // own fields, not a hardcoded literal -- #62 (2026-08-09): this
        // assert was previously pinned to the pre-retune rates (0.04,
        // 0.03 = the old 0.0004/0.0003 defaults × 100), exactly the
        // stale-literal trap the retune already hit twice in the harness
        // fixtures. Deriving the expected value from cfg.*.decay_per_sec
        // means a future default change can't silently desync this test.
        let mut n = Needs::default();
        decay_needs(&mut n, 100.0, &cfg);
        assert!((n.hunger - (1.0 - cfg.hunger.decay_per_sec * 100.0)).abs() < 1e-6);
        assert!((n.rest - (1.0 - cfg.rest.decay_per_sec * 100.0)).abs() < 1e-6);
        assert!((n.recreation - (1.0 - cfg.recreation.decay_per_sec * 100.0)).abs() < 1e-6);
        decay_needs(&mut n, 1.0e9, &cfg);
        assert_eq!((n.hunger, n.rest, n.recreation), (0.0, 0.0, 0.0));
        // Thought decay: full at age 0, half at half-life, zero past.
        assert!((thought_decay(-0.15, 0.0, 0.0, 100.0) + 0.15).abs() < 1e-6);
        assert!((thought_decay(-0.15, 0.0, 50.0, 100.0) + 0.075).abs() < 1e-6);
        assert_eq!(thought_decay(-0.15, 0.0, 100.0, 100.0), 0.0);
        assert_eq!(thought_decay(-0.15, 0.0, 500.0, 100.0), 0.0);
    }

    /// B-AG3 slice 1: the care multiplier pinned — identity for the
    /// unvalued; DIVERGENT for two colonists with different value maps on
    /// the SAME affinity row (the block's done-when in pure form); exact
    /// arithmetic at the ±50 scale; clamped both ways; the neurotic amp
    /// hits negative thoughts only, after the clamp.
    #[test]
    fn bastion_care_factor_exact() {
        use crate::bastion::Value;
        use std::collections::BTreeMap;
        let empty: BTreeMap<Value, i8> = BTreeMap::new();
        let row = [(Value::Kin, 0.6f32), (Value::Glory, -0.4)];
        // Identity: no values, or no affinity row -> exactly 1.0.
        assert_eq!(care_factor(&empty, &row, false, -0.15), 1.0);
        let mut kin = BTreeMap::new();
        kin.insert(Value::Kin, 50i8);
        assert_eq!(care_factor(&kin, &[], false, -0.15), 1.0);
        // DIVERGENCE: same row, two different value maps.
        let mut glory = BTreeMap::new();
        glory.insert(Value::Glory, 50i8);
        let care_kin = care_factor(&kin, &row, false, -0.15);
        let care_glory = care_factor(&glory, &row, false, -0.15);
        assert!((care_kin - 1.6).abs() < 1e-6); // 1 + (50/50)·0.6
        assert!((care_glory - 0.6).abs() < 1e-6); // 1 + (50/50)·(−0.4)
        assert!(care_kin > care_glory);
        // Scorn: a negative weight flips the affinity's direction.
        let mut scorns_kin = BTreeMap::new();
        scorns_kin.insert(Value::Kin, -50i8);
        assert!((care_factor(&scorns_kin, &row, false, -0.15) - 0.4).abs() < 1e-6);
        // Clamps: stacked scorn floors at CARE_MIN, stacked zeal caps at
        // CARE_MAX.
        let big_row = [(Value::Kin, 5.0f32)];
        assert_eq!(care_factor(&kin, &big_row, false, -0.15), CARE_MAX);
        let neg_row = [(Value::Kin, -5.0f32)];
        assert_eq!(care_factor(&kin, &neg_row, false, -0.15), CARE_MIN);
        // Neurotic: ×1.5 on NEGATIVE thoughts only, applied post-clamp.
        assert!((care_factor(&kin, &row, true, -0.15) - 1.6 * NEUROTIC_NEGATIVE_AMP).abs() < 1e-6);
        assert!((care_factor(&kin, &row, true, 0.15) - 1.6).abs() < 1e-6);
        assert_eq!(
            care_factor(&kin, &big_row, true, -0.15),
            CARE_MAX * NEUROTIC_NEGATIVE_AMP
        );
    }

    /// FOCUS-0-DERIVE (43.1): the derivation pinned — value-mapped needs
    /// scale 1 + weight/50 exactly (±50 spans 0..2); unmapped needs sit
    /// at baseline regardless of values; Socialize's 3-level trait gate
    /// is consistent with the public `.is()` API over a seeded
    /// personality sample, and both extremes occur in the sample.
    #[test]
    fn bastion_derive_need_weight_exact() {
        use crate::{
            bastion::{Need, Value},
            rtsim::{Personality, PersonalityTrait},
        };
        use rand::SeedableRng;
        use std::collections::BTreeMap;
        let mut rng = rand::rngs::StdRng::seed_from_u64(0xF0C0_5D34);
        let neutral = Personality::random(&mut rng);
        // Value arms: exact linear map, empty = baseline.
        let empty: BTreeMap<Value, i8> = BTreeMap::new();
        assert_eq!(derive_need_weight(Need::Pray, &neutral, &empty), 1.0);
        let mut v = BTreeMap::new();
        v.insert(Value::Piety, 50i8);
        v.insert(Value::Kin, -50);
        v.insert(Value::Wealth, 25);
        assert_eq!(derive_need_weight(Need::Pray, &neutral, &v), 2.0);
        assert_eq!(derive_need_weight(Need::Family, &neutral, &v), 0.0);
        assert!((derive_need_weight(Need::Acquire, &neutral, &v) - 1.5).abs() < 1e-6);
        // Unmapped needs: baseline even with a loud value map.
        assert_eq!(derive_need_weight(Need::Drink, &neutral, &v), 1.0);
        assert_eq!(derive_need_weight(Need::AdmireArt, &neutral, &v), 1.0);
        assert_eq!(derive_need_weight(Need::Learn, &neutral, &v), 1.0);
        // Socialize: 3-level, consistent with the public trait API; a
        // 400-draw seeded sample contains both extremes.
        let (mut saw_high, mut saw_low) = (false, false);
        for _ in 0..400 {
            let p = Personality::random(&mut rng);
            let w = derive_need_weight(Need::Socialize, &p, &empty);
            let expect = if p.is(PersonalityTrait::Extroverted) || p.is(PersonalityTrait::Sociable)
            {
                1.5
            } else if p.is(PersonalityTrait::Introverted) {
                0.5
            } else {
                1.0
            };
            assert_eq!(w, expect);
            saw_high |= w == 1.5;
            saw_low |= w == 0.5;
        }
        assert!(saw_high && saw_low, "seeded sample must span the gate");
    }

    /// T3.54: exact derived DTO order — needs by [`MoodNeedId`], thoughts
    /// by `(source_event_id, thought_id)` regardless of input order,
    /// thresholds by [`MoodNeedId`]. Also pins `total_mood` to the same
    /// value [`mood_formula`] returns for identical inputs.
    #[test]
    fn mood_explanation_v1_exact_evidence_order() {
        let cfg = crate::bastion::MoodConfig::default();
        let needs = Needs { hunger: 0.4, rest: 0.6, recreation: 0.9 };
        let thought_sum = 0.05f32;
        // Deliberately unsorted input — the DTO must sort it, not trust it.
        let thoughts = vec![
            ThoughtContributionV1 {
                source_event_id: 5,
                thought_id: 2,
                base_magnitude: 0.1,
                care_multiplier: 1.0,
                contribution: 0.1,
            },
            ThoughtContributionV1 {
                source_event_id: 1,
                thought_id: 9,
                base_magnitude: 0.2,
                care_multiplier: 1.0,
                contribution: 0.2,
            },
            ThoughtContributionV1 {
                source_event_id: 1,
                thought_id: 3,
                base_magnitude: -0.05,
                care_multiplier: 1.0,
                contribution: -0.05,
            },
        ];
        let actor = crate::rtsim::Actor::Character(crate::character::CharacterId(7));
        let explanation =
            MoodExplanationV1::build(42, actor, &cfg, &needs, thought_sum, thoughts);

        assert_eq!(
            explanation.needs.iter().map(|n| n.need).collect::<Vec<_>>(),
            vec![MoodNeedId::Hunger, MoodNeedId::Rest, MoodNeedId::Recreation]
        );
        assert_eq!(
            explanation
                .thoughts
                .iter()
                .map(|t| (t.source_event_id, t.thought_id))
                .collect::<Vec<_>>(),
            vec![(1, 3), (1, 9), (5, 2)]
        );
        assert_eq!(
            explanation.thresholds.iter().map(|t| t.need).collect::<Vec<_>>(),
            vec![MoodNeedId::Hunger, MoodNeedId::Rest, MoodNeedId::Recreation]
        );
        assert_eq!(
            explanation.total_mood,
            mood_formula(&cfg, &needs, thought_sum)
        );
    }

    /// T3.54: diagnostic/presentation fields (per-thought `base_magnitude`/
    /// `care_multiplier`, and the whole `thoughts`/`needs`/`thresholds`
    /// breakdown) must never move `total_mood` — it is driven ONLY by the
    /// caller's `thought_sum`, never re-derived from the vector.
    #[test]
    fn mood_explanation_v1_diagnostic_fields_do_not_move_authoritative_hash() {
        let cfg = crate::bastion::MoodConfig::default();
        let needs = Needs { hunger: 0.3, rest: 0.5, recreation: 0.7 };
        let thought_sum = 0.2f32;
        let actor = crate::rtsim::Actor::Character(crate::character::CharacterId(1));

        let real = ThoughtContributionV1 {
            source_event_id: 1,
            thought_id: 1,
            base_magnitude: 0.4,
            care_multiplier: 0.5,
            contribution: 0.2,
        };
        // A diagnostically-different row (different magnitude/care/id) that
        // still sums to the SAME `contribution` — irrelevant, since
        // `total_mood` never reads the vector at all.
        let relabeled = ThoughtContributionV1 {
            source_event_id: 99,
            thought_id: 42,
            base_magnitude: 999.0,
            care_multiplier: -3.0,
            contribution: 0.2,
        };

        let a = MoodExplanationV1::build(1, actor, &cfg, &needs, thought_sum, vec![real]);
        let b = MoodExplanationV1::build(1, actor, &cfg, &needs, thought_sum, vec![relabeled]);
        assert_eq!(a.total_mood, b.total_mood);
        assert_eq!(a.total_mood, mood_formula(&cfg, &needs, thought_sum));
        assert_ne!(a.thoughts, b.thoughts, "the diagnostic rows must actually differ");
    }

    fn test_uid(n: u64) -> crate::uid::Uid {
        crate::uid::Uid(std::num::NonZeroU64::new(n).unwrap())
    }

    /// T3.58: exact derived evidence — `self_job_*` is `Some` only for a
    /// pre-claimed self-job kind (never `Designated`), `intent_owner_kind`
    /// reads the claimant relative to the inspected actor, and
    /// `drive_scores_digest` is a pure function of `last_scores` (same
    /// input -> same digest, changed input -> a different one).
    #[test]
    fn inspector_ownership_v1_exact_evidence_order() {
        let me = test_uid(7);
        let other = test_uid(9);
        let active = ActiveJob {
            job: 42,
            state: ActiveJobState::Arrived,
            best_dist: 0.0,
            stuck_time: 0.0,
            reset_dist: 0.0,
            soft_granted: false,
            stance: vek::Vec3::new(0, 0, 1),
        };
        let designated = crate::bastion::JobKind::Designated(crate::bastion::DesignationKind::Mine);
        let self_job = crate::bastion::JobKind::RestAt { bed_pos: vek::Vec3::new(1, 2, 3) };
        let arb = Arbiter {
            current: Drive::Work,
            committed_until: 0.0,
            last_scores: (0.1, 0.2, 0.3),
            activity: Some((crate::bastion::WorkType::Mine, 0.5)),
            pending_self_job: None,
        };

        // Designated job, self-claimed: self_job_* absent, active_job_*
        // present, intent SelfClaimed.
        let a = InspectorOwnershipV1::build(9, me, Some(&active), Some(&designated), Some(me), Some(&arb));
        assert_eq!(a.self_job_id, None);
        assert_eq!(a.self_job_kind, None);
        assert_eq!(a.active_job_id, Some(42));
        assert_eq!(a.active_job_state, Some(ActiveJobState::Arrived));
        assert_eq!(a.intent_owner_kind, IntentOwnerKindV1::SelfClaimed);
        assert_eq!(a.lease_generation, None);
        assert_eq!(a.arbiter_activity, Some((crate::bastion::WorkType::Mine, 0.5)));

        // Self-job kind: self_job_* mirrors active_job_*.
        let b = InspectorOwnershipV1::build(9, me, Some(&active), Some(&self_job), Some(me), Some(&arb));
        assert_eq!(b.self_job_id, Some(42));
        assert_eq!(b.self_job_kind, Some(JobKindTagV1::RestAt));

        // Claimed by someone else / unclaimed / no active job.
        let c = InspectorOwnershipV1::build(9, me, Some(&active), Some(&designated), Some(other), Some(&arb));
        assert_eq!(c.intent_owner_kind, IntentOwnerKindV1::OtherClaimant);
        let d = InspectorOwnershipV1::build(9, me, Some(&active), Some(&designated), None, Some(&arb));
        assert_eq!(d.intent_owner_kind, IntentOwnerKindV1::Unclaimed);
        let e = InspectorOwnershipV1::build(9, me, None, None, None, None);
        assert_eq!(e.intent_owner_kind, IntentOwnerKindV1::NoActiveJob);
        assert_eq!(e.self_job_id, None);
        assert_eq!(e.active_job_id, None);

        // The digest is a pure function of last_scores.
        assert_eq!(a.drive_scores_digest, b.drive_scores_digest);
        let mut different = arb;
        different.last_scores = (0.9, 0.2, 0.3);
        let f = InspectorOwnershipV1::build(9, me, Some(&active), Some(&designated), Some(me), Some(&different));
        assert_ne!(a.drive_scores_digest, f.drive_scores_digest);
    }

    /// T3.58: diagnostic/presentation fields (`self_job_kind`,
    /// `active_job_state`, `arbiter_activity`) must never move
    /// `drive_scores_digest` — it is driven ONLY by `last_scores`.
    #[test]
    fn inspector_ownership_v1_diagnostic_fields_do_not_move_digest() {
        let me = test_uid(1);
        let arb = Arbiter {
            current: Drive::Idle,
            committed_until: 0.0,
            last_scores: (0.4, 0.4, 0.4),
            activity: None,
            pending_self_job: None,
        };
        let travel = ActiveJob {
            job: 1,
            state: ActiveJobState::Traveling,
            best_dist: 5.0,
            stuck_time: 1.0,
            reset_dist: 5.0,
            soft_granted: false,
            stance: vek::Vec3::new(0, 0, 1),
        };
        let arrived = ActiveJob { state: ActiveJobState::Arrived, ..travel };
        let haul = crate::bastion::JobKind::Haul { item: test_uid(2), destination: 3 };
        let despond = crate::bastion::JobKind::Despond { until: 0.0 };

        let a = InspectorOwnershipV1::build(1, me, Some(&travel), Some(&haul), Some(me), Some(&arb));
        let b = InspectorOwnershipV1::build(1, me, Some(&arrived), Some(&despond), Some(me), Some(&arb));
        assert_ne!(a.self_job_kind, b.self_job_kind, "the diagnostic rows must actually differ");
        assert_ne!(a.active_job_state, b.active_job_state);
        assert_eq!(a.drive_scores_digest, b.drive_scores_digest);
    }
}

/// The colonist's current job assignment (B4). Server-side only; the job
/// system owns the colonist's rtsim-controller activity while this exists.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveJob {
    pub job: crate::bastion::JobId,
    pub state: ActiveJobState,
    /// Travel watchdog: best distance-to-target achieved so far + time since
    /// it last improved. Displacement alone is useless — an agent pacing
    /// around an unreachable target moves plenty without progressing.
    pub best_dist: f32,
    pub stuck_time: f32,
    /// bastion (B-LIVE3, reviewer R3 fix-1 — stuck-time HYSTERESIS): the
    /// distance at the last stuck_time ZERO. The accumulator only resets
    /// on ≥1 block of NET progress since then, so sub-block jitter (magnet
    /// nudges, hover bobbing, physics wobble — all ≥ the 0.5 EPSILON)
    /// can't starve the watchdog forever; real walking (2+ blocks/s)
    /// resets comfortably. Without this, a hovering colonist generated
    /// ZERO timeouts → zero churn → no net ever fired.
    #[serde(default)]
    pub reset_dist: f32,
    /// bastion (B6 SOFT-0): this stall already got its soft-collision
    /// GRACE WINDOW (SOFT-COLLISION-design §0 trigger a). The watchdog
    /// grants soft-pass ONCE per assignment before degrading to the
    /// carve/unreachable pipeline — most chokepoint deadlocks clear in
    /// the grace; a still-stuck soft colonist is genuinely blocked.
    #[serde(default)]
    pub soft_granted: bool,
    /// bastion (B15 / reviewer FR12): the committed work-STANCE — the feet-cell
    /// OFFSET from `job.pos` where the colonist stands to work the block.
    /// (0,0,1) = ON-TOP (stand on the block; the default, = the pre-B15
    /// `job.pos + (0.5,0.5,1.0)` arrive-target). A cardinal
    /// `(±1,0,0)`/`(0,±1,0)` = an ADJACENT-ground stance (stand beside +
    /// mine sideways — the fix for hillside `+1`-arrival-gap cells whose
    /// on-top stance is a 1-wide slot the capsule can't occupy). PINNED at
    /// claim by the once-per-cycle standability pass, NOT re-picked each
    /// tick (avoids re-introducing the R3 steer oscillation). Server-only;
    /// the serde default is inert (never deserialized — every insert sets
    /// it explicitly).
    #[serde(default)]
    pub stance: vek::Vec3<i32>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveJobState {
    /// Walking to the job site.
    Traveling,
    /// At the site, ready to work (B5 hooks here).
    Arrived,
    /// bastion (B6, reviewer R3 fix-2): queued at a single-file vertical
    /// link — another colonist is closer to the staged access anchor, so
    /// this one WAITS ITS TURN. The watchdog skips Waiting entirely (no
    /// stall accrual, no unreachable, no strikes, no churn — queue-waiting
    /// is not stuckness); promotion back to Traveling happens every
    /// arbitration pass, which re-evaluates the queue order. Emergent
    /// single-file: nearest climbs, the rest hold.
    Waiting,
}

impl Component for ActiveJob {
    type Storage = specs::DenseVecStorage<Self>;
}

/// engine-list T3.58: coarse [`crate::bastion::JobKind`] tag for
/// [`InspectorOwnershipV1`] — same variants, payload dropped (diagnostic
/// surfacing only, never round-tripped back into a real `JobKind`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobKindTagV1 {
    Designated(crate::bastion::DesignationKind),
    Haul,
    DepositRun,
    RestAt,
    EatFrom,
    Despond,
    /// bastion (ITEM 11): recreation's self-job. APPENDED LAST — this is a
    /// VERSIONED tag (V1) whose ordinal is part of the digest, so a new
    /// variant may only go on the end; inserting it beside the other
    /// self-jobs would renumber `Despond` and move every hash that has
    /// ever included one.
    Recreate,
}

impl From<&crate::bastion::JobKind> for JobKindTagV1 {
    fn from(k: &crate::bastion::JobKind) -> Self {
        use crate::bastion::JobKind as J;
        match k {
            J::Designated(d) => JobKindTagV1::Designated(*d),
            J::Haul { .. } => JobKindTagV1::Haul,
            J::DepositRun { .. } => JobKindTagV1::DepositRun,
            J::RestAt { .. } => JobKindTagV1::RestAt,
            J::EatFrom { .. } => JobKindTagV1::EatFrom,
            J::Despond { .. } => JobKindTagV1::Despond,
            J::Recreate { .. } => JobKindTagV1::Recreate,
        }
    }
}

impl JobKindTagV1 {
    /// T3.58's "self job": pre-claimed FOR one colonist rather than drawn
    /// competitively from the shared `Designated` pool — matches the
    /// existing doc language on `Despond`/`DepositRun`/`RestAt`/`EatFrom`
    /// ("a pre-claimed self-job", see [`crate::bastion::JobKind::Despond`]).
    pub fn is_self_job(self) -> bool { !matches!(self, JobKindTagV1::Designated(_)) }
}

/// T3.58: where a job's claim stands relative to the inspected actor —
/// evidence/diagnostic only, never read by arbitration.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentOwnerKindV1 {
    NoActiveJob,
    SelfClaimed,
    Unclaimed,
    OtherClaimant,
}

/// engine-list T3.58: `InspectorOwnershipV1` — job ownership + Drive
/// telemetry for the utility-AI debug overlay. `self_job_*` is `Some`
/// only when [`JobKindTagV1::is_self_job`] holds for the active job (so
/// under the current one-job-per-colonist model it duplicates
/// `active_job_id` in value, but answers a DIFFERENT question — ownership
/// source, not runtime progress — per the ruled field list).
/// `lease_generation` is always `None`: [`crate::bastion::Job::claimed_by`]
/// is a plain `Option<Uid>` with no generation/epoch counter today
/// (distinct from the unrelated haul-link `reservation_generation`
/// fencing) — left an explicit absent placeholder rather than inventing
/// one, same discipline as [`MoodExplanationV1`]'s omitted "timers".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InspectorOwnershipV1 {
    pub snapshot_tick: u64,
    pub actor_id: crate::uid::Uid,
    pub self_job_id: Option<crate::bastion::JobId>,
    pub self_job_kind: Option<JobKindTagV1>,
    pub active_job_id: Option<crate::bastion::JobId>,
    pub active_job_state: Option<ActiveJobState>,
    pub intent_owner_kind: IntentOwnerKindV1,
    pub lease_generation: Option<u64>,
    pub arbiter_activity: Option<(crate::bastion::WorkType, f32)>,
    pub drive_scores_digest: u64,
}

impl InspectorOwnershipV1 {
    /// `job_kind`/`claimed_by` are the looked-up `board.jobs[active.job]`
    /// fields (the caller's `ActiveJob` lookup); passed separately rather
    /// than a `&Job` because this module cannot see `JobBoard`.
    pub fn build(
        snapshot_tick: u64,
        actor_id: crate::uid::Uid,
        active: Option<&ActiveJob>,
        job_kind: Option<&crate::bastion::JobKind>,
        claimed_by: Option<crate::uid::Uid>,
        arbiter: Option<&Arbiter>,
    ) -> Self {
        let active_job_id = active.map(|a| a.job);
        let active_job_state = active.map(|a| a.state);
        let tag = job_kind.map(JobKindTagV1::from);
        let (self_job_id, self_job_kind) = match tag {
            Some(t) if t.is_self_job() => (active_job_id, Some(t)),
            _ => (None, None),
        };
        let intent_owner_kind = match (active, claimed_by) {
            (None, _) => IntentOwnerKindV1::NoActiveJob,
            (Some(_), Some(c)) if c == actor_id => IntentOwnerKindV1::SelfClaimed,
            (Some(_), Some(_)) => IntentOwnerKindV1::OtherClaimant,
            (Some(_), None) => IntentOwnerKindV1::Unclaimed,
        };
        let scores = arbiter.map_or((0.0, 0.0, 0.0), |a| a.last_scores);
        let drive_scores_digest = crate::state_hash::stable_hash_u64(
            "bastion/inspector-ownership/drive-scores/v1",
            &(scores.0.to_bits(), scores.1.to_bits(), scores.2.to_bits()),
        );
        Self {
            snapshot_tick,
            actor_id,
            self_job_id,
            self_job_kind,
            active_job_id,
            active_job_state,
            intent_owner_kind,
            lease_generation: None,
            arbiter_activity: arbiter.and_then(|a| a.activity),
            drive_scores_digest,
        }
    }
}

/// The god-mode anchor marker (§4 standing directive): while the overseer is
/// active, the player's avatar entity carries this — the world must ignore it
/// (no targeting/aggro/greeting/pushback) and it must be invulnerable (the
/// server also applies a permanent `Invulnerability` buff). Removed on F9 /
/// anchor clear; mortality applies only under Embody (B12).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionGodAnchor;

impl Component for BastionGodAnchor {
    type Storage = NullStorage<Self>;
}

/// bastion (B-ASSET1): a direct movement order for test fixtures — the
/// colonist walks to `target` through the vanilla agent (the same
/// `NpcActivity::Goto` mechanism job travel uses) with the same 3D-arrival +
/// progress-watchdog semantics. Server-side only; inert unless inserted
/// (harness `--asset-test` and `--asset-arena` fixtures). Mutually exclusive
/// with [`ActiveJob`] by convention (the hook that inserts it refuses
/// job-holding colonists).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionTestGoto {
    pub target: vek::Vec3<f32>,
    /// Travel watchdog (same scheme as [`ActiveJob`]): best distance achieved
    /// so far + time since it last improved.
    pub best_dist: f32,
    pub stuck_time: f32,
    /// Sim seconds spent on this order (arrival-budget accounting).
    pub elapsed: f32,
    pub arrived: bool,
    /// The watchdog gave up: no progress within the stuck timeout.
    pub stuck: bool,
}

impl BastionTestGoto {
    pub fn new(target: vek::Vec3<f32>) -> Self {
        Self {
            target,
            best_dist: f32::INFINITY,
            stuck_time: 0.0,
            elapsed: 0.0,
            arrived: false,
            stuck: false,
        }
    }
}

impl Component for BastionTestGoto {
    type Storage = specs::DenseVecStorage<Self>;
}

/// A persistent colonist-produced item pile (B5.5). Entities carrying this:
/// never get a despawn timer (colonist output is a player resource — item
/// loss is an invariant violation), aggregate freely with each other via the
/// vanilla merge machinery, and NEVER merge across class with timed vanilla
/// drops (a pile merging into a timed drop would inherit its despawn — a
/// silent-loss path). Server-side only.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionPile;

impl Component for BastionPile {
    type Storage = NullStorage<Self>;
}
