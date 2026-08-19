//! `JobBoard` and its closure, extracted from `bastion_jobs.rs` so that
//! `veloren-server` can depend on `bastion-core` instead of on
//! `bastion-server`. The ~20k lines of job LOGIC deliberately stay behind
//! in `bastion-server`, which then sits ABOVE `veloren-server`.
//!
//! Everything here was `pub`-ified only as far as crossing the crate
//! boundary requires; nothing changed semantically in the move.

use crate::{
    Tick,
    bastion_traversal::{BastionTraversalPhase, BastionTraversalPurpose, BastionTraversalTask},
};
use common::{
    bastion::{
        AffordanceClass, BUILD_MATERIAL_ITEM, CHOP_DROP_ITEM, DesignationKind, Job, JobAudit,
        JobId, MINE_DROP_ITEM, Region, ZExtent,
    },
    combat,
    comp,
    comp::{
        Item,
        bastion::{ActiveJob, ActiveJobState},
        item::PickupItem,
    },
    event::CreateItemDropEvent,
    resources::{DeltaTime, ProgramTime},
    terrain::{Block, BlockKind, SpriteKind, TerrainGrid, sprite::Growth},
    uid::{IdMaps, Uid},
    vol::{BaseVol, ReadVol},
};
use common_ecs::{Job as EcsJob, Origin, Phase, System};
use common_net::sync::InterpolatableComponent;
use common_state::BlockChange;
use hashbrown::{HashMap, HashSet};
use specs::{
    Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, Write, WriteExpect, WriteStorage,
};
use std::hash::{Hash, Hasher};
use tracing::{error, info};
use vek::*;

/// bastion (CHOP-FELLING): one tree's stored fell-set, keyed by its single
/// base-cut job — "what you saw outlined is what falls" (the set is FROZEN
/// at placement, immune to mid-life terrain drift; the frozen-plan-cells
/// precedent). `threshold` is the job's size-scaled completion bar,
/// `wood_count` the placement-time Wood tally (XP + threshold source; drops
/// re-read kinds at removal time for conservation).
pub struct ChopFell {
    pub cells: Vec<Vec3<i32>>,
    pub threshold: f32,
    pub wood_count: u32,
}

/// bastion (CHOP-FELLING): a tree mid-fall — the v1.5 top-down stagger.
/// `cells` sorted (z DESC, y, x): one z-band clears per tick, so at every
/// intermediate state the remainder is base-connected (no-float BY
/// CONSTRUCTION — the base is in the LAST band) and the order is a total
/// order (gate determinism). `cursor` = next cell to clear.
pub struct FellingTree {
    pub cells: Vec<Vec3<i32>>,
    pub cursor: usize,
}

pub fn watch_wipe(watch: &mut HashMap<Uid, f32>, uid: &Uid, reason: &'static str) {
    let had = watch.remove(uid);
    if had.is_some_and(|secs| secs >= 1.0)
        && std::env::var_os("BASTION_EGRESS_DIAG").is_some()
    {
        info!(
            uid = uid.0.get(),
            secs = had.unwrap_or(0.0),
            reason,
            "bastion: stuck_watch wiped"
        );
    }
}

/// The per-block job predicate, shared by both placement paths. Mine =
/// every filled block; Chop = wood only; Build = currently-empty positions
/// (placing blocks, not removing); Stockpile = none yet (B6 zones).
pub fn job_wanted(kind: DesignationKind, block: &Block) -> bool {
    match kind {
        DesignationKind::Mine => block.is_filled(),
        // CHOP redesign (FR10): a fell-set covers the WHOLE tree — trunk
        // (Wood) AND canopy (Leaves; cleared, no drop — the drop branch keys
        // on the block kind). Fixes the registry's "Chop-ignores-Leaves".
        DesignationKind::Chop => {
            matches!(block.kind(), BlockKind::Wood | BlockKind::Leaves)
        },
        // B5.8: a ladder rung, like Build, goes into currently-open space.
        // B7-1: a bed too.
        DesignationKind::Build | DesignationKind::Ladder | DesignationKind::Bed => {
            !block.is_filled()
        },
        DesignationKind::Stockpile | DesignationKind::Zone(_) => false,
        // GATHER (row 38): forage — one job per collectible PLANT sprite
        // (the TerrainResource food allowlist; Stones/Wood/Gem/Ore stay
        // with the Mine/Chop economies). `is_directly_collectible` =
        // vanilla's own "yields without a required item" predicate, so
        // every job the scan creates is one the authoritative Collect
        // handler will actually honor.
        // FARM (row 46): the paint registers a persistent plot and
        // generates NO jobs — the farm trigger pass owns per-cell job
        // creation from cell state (the Stockpile-registration shape).
        DesignationKind::Farm => false,
        DesignationKind::Gather => {
            block.is_directly_collectible()
                && block.get_rtsim_resource().is_some_and(|r| {
                    matches!(
                        r,
                        common::rtsim::TerrainResource::Grass
                            | common::rtsim::TerrainResource::Flower
                            | common::rtsim::TerrainResource::Fruit
                            | common::rtsim::TerrainResource::Vegetable
                            | common::rtsim::TerrainResource::Mushroom
                            | common::rtsim::TerrainResource::Plant
                    )
                })
        },
    }
}

/// bastion (task #64, KindAffordance): the stamped-vocabulary helper for
/// the two GENERIC multi-kind placement paths (`place_designation`,
/// `place_designation_surface` — the only sites that construct a `Job`
/// for an arbitrary caller-supplied `kind`). Every other construction
/// site builds one specific kind and stamps its `AffordanceClass`
/// directly, inline, where the physical shape is unambiguous. Kept
/// EXHAUSTIVE (no wildcard) even though `job_wanted` is false for
/// Stockpile/Zone/Farm at both call sites (so no `Job` is ever actually
/// built with those arms' value) — a future `DesignationKind` variant
/// must choose an arm here too, not fall through silently.
///
/// PURE-REFACTOR SCOPE (DECISIONS #45): Build and Bed declare
/// `OnTopAlways`, matching pre-#64 behaviour exactly (Ladder's own
/// control -- see `on_top_always`'s doc -- showed the physical-support
/// argument that motivated `AdjacentToBase` doesn't predict outcomes in a
/// system with no execution-proximity check, and Build/Bed never had
/// their own control). `AdjacentToBase` stays built and reachable, not
/// deleted, for whichever of Build/Bed earns its own evidence-gated row.
fn designation_affordance(kind: DesignationKind) -> AffordanceClass {
    match kind {
        DesignationKind::Mine | DesignationKind::Chop | DesignationKind::Gather => {
            AffordanceClass::SolidTarget
        },
        DesignationKind::Build | DesignationKind::Bed | DesignationKind::Ladder => {
            AffordanceClass::OnTopAlways
        },
        DesignationKind::Stockpile | DesignationKind::Zone(_) | DesignationKind::Farm => {
            AffordanceClass::Untargeted
        },
    }
}

/// The saturation field's coarse cell for a world position (euclidean division
/// so negative coordinates bucket correctly).
pub fn coord_cell(pos: Vec3<i32>) -> Vec2<i32> {
    Vec2::new(pos.x.div_euclid(COORD_CELL), pos.y.div_euclid(COORD_CELL))
}

/// bastion (#68 amendment, Opus/#60 falsifier prereg): the three reset
/// paths of the F3 stale-access-plan pruner (see the pruner's own doc
/// comment at its `if`/`else if`/`else` chain) -- `access_idle_secs ==
/// 0` alone cannot distinguish a classified material hold (which resets
/// forever by design while the hold lasts) from "no access jobs at
/// all" (nothing pathological), or either from a claimed/churning plan.
/// `Debug` prints the letter used in `ROW60-F3-PRUNER-FALSIFIER-
/// PREREG.md` and the `F3-BRANCH` emit, not the variant name.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum F3PruneBranch {
    MaterialHeld,
    Idle,
    ClaimedOrAbsent,
}

impl std::fmt::Debug for F3PruneBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            F3PruneBranch::MaterialHeld => "A",
            F3PruneBranch::Idle => "B",
            F3PruneBranch::ClaimedOrAbsent => "C",
        })
    }
}

/// bastion (B5.6b-2): the z of the topmost real-terrain block in column
/// (x, y), scanned in a window around `hint_z` (the painted plane). This is
/// THE per-column surface authority for surface-relative designations —
/// resolved ONCE at placement time (digging afterwards does not re-resolve
/// the volume). Returns `None` if no real terrain is in the window (e.g.
/// painted over open water or an unloaded chunk).
pub fn column_surface_z(terrain: &TerrainGrid, x: i32, y: i32, hint_z: i32) -> Option<i32> {
    (hint_z - SURFACE_SCAN_DOWN..=hint_z + SURFACE_SCAN_UP)
        .rev()
        .find(|z| {
            terrain
                .get(Vec3::new(x, y, *z))
                .is_ok_and(|b| is_surface_terrain(b.kind()))
        })
}

/// THE per-column surface authority a designation resolves against — flat-floor
/// mode ([`ZExtent::floor_z`] set) reaches the column's true crest
/// ([`column_flat_surface_z`], the flatten-hill fix), relative mode uses the
/// ±window around the paint plane ([`column_surface_z`]). ONE function so
/// job generation, echo bounds, AND the paint-time volume gate all resolve the
/// SAME surface (the echo-bounds invariant + an honest volume cap depend on
/// it) -- for z-extent kinds. Area2D kinds (Farm, Chop) never carry a
/// `ZExtent` and so cannot call this function at all; they resolve
/// separately by calling [`column_surface_z`] directly BY SIGNATURE, not
/// by choice (FARM-PAINT-FIX.md). If Area2D kinds ever gain flat-mode
/// semantics, unify first, before assuming this doc's "ONE function"
/// claim already covers them.
pub fn resolve_column_surface(
    terrain: &TerrainGrid,
    x: i32,
    y: i32,
    hint_z: i32,
    extent: &ZExtent,
) -> Option<i32> {
    match extent.floor_z {
        Some(floor) => column_flat_surface_z(terrain, x, y, floor),
        None => column_surface_z(terrain, x, y, hint_z),
    }
}

/// Diagnostic attribution for one ultimate stuck-rescue teleport.
#[derive(Clone, Debug)]
pub struct FailsafeTeleportEvent {
    pub uid: u64,
    pub name: String,
    pub feet: Vec3<i32>,
    pub destination: Vec3<i32>,
    pub stuck_seconds: f32,
    pub active_job: Option<JobId>,
    pub active_job_state: Option<String>,
    pub active_job_kind: Option<String>,
    pub active_job_is_access: Option<bool>,
    pub egress_verdicts: u64,
    /// item 4b split (Fable's ruling, 2026-08-11): the two unrelated
    /// producers `egress_verdicts` unions -- see `JobBoard`'s own fields
    /// for the full rationale.
    pub egress_verdicts_churn: u64,
    pub egress_verdicts_scan_ok: u64,
    pub egress_plans_emitted: u64,
    pub egress_no_route: u64,
    pub climb_free_active: bool,
    pub organic_destination: Option<Vec3<i32>>,
    pub head_clear: bool,
    /// #85: SNAPSHOT, read live at emit time (never cached) -- whether the
    /// entity's OWN chunk (`feet`'s, not the 0..=8-radius `dest` scan's) is
    /// loaded. `dest = Some` only proves SOME cell within 8 blocks was
    /// readable, not that THIS entity's chunk is -- this field is the only
    /// thing that separates those two facts. Producer:
    /// `terrain.contains_key(terrain.pos_key(feet))` at the emit site.
    pub in_loaded_chunk: bool,
    /// #85 gate 2: SNAPSHOT, read live at emit time. `phys::Sys` carries
    /// `!&read.is_riders, !&read.is_volume_riders` on both its gravity and
    /// collision joins -- a rider gets neither, so terrain can be perfectly
    /// readable and the entity still never falls. Producer:
    /// `Is<Rider>` or `Is<VolumeRider>` present on the entity, at the emit
    /// site.
    pub is_rider: bool,
    /// #85 (Fable's amendment): the physics-join membership witness, SPLIT
    /// into its four component bits rather than one AND -- if the AND came
    /// back false, the next question is which component is missing, and an
    /// AND cannot be decomposed after the fact (the split costs the same
    /// four reads). `body` in particular is REQUIRED for gravity but
    /// `.maybe()` for collision in `phys::Sys` -- these two joins do not
    /// filter identically, so the split distinguishes "outside gravity's
    /// join but inside collision's" from "outside both", which an AND
    /// renders identically. SNAPSHOT, read live at emit time.
    pub has_collider: bool,
    pub has_mass: bool,
    pub has_density: bool,
    pub has_body: bool,
    pub on_ground: bool,
    pub on_wall: bool,
    pub character_state: Option<String>,
    pub velocity: Vec3<f32>,
    pub access_jobs_pending: usize,
    pub terminal_cause: &'static str,
}

#[derive(Clone, Copy, Debug, Hash)]
pub struct EmergencyRouteDescriptor {
    pub kind: EmergencyTraversalKind,
    pub approach: Vec3<i32>,
    pub entry: Vec3<i32>,
    pub top_anchor: Vec3<i32>,
    pub dismount: Vec3<i32>,
    /// For a natural shaft this is the deterministic solid wall face that the
    /// existing CharacterState::Climb must contact. Constructed ladders derive
    /// their contact from the installed rung sprite instead.
    pub wall_dir: Option<Vec2<i32>>,
}

/// REQ-0068: bounded pre-route state for an airborne egress owner that must
/// first reach a real standable construction origin. This record does not
/// move the entity: it owns only the existing `NpcActivity::Goto`/`Stand`
/// handoff and suppresses the conflicting generic climb assist while normal
/// physics resolves the approach.
#[derive(Clone, Copy, Debug)]
pub struct EmergencySettleAnchor {
    pub anchor: Option<Vec3<i32>>,
    pub target: Vec3<i32>,
    pub started_tick: u64,
    pub last_progress_tick: u64,
    pub best_distance: f32,
}

/// REQ-0070: explicit walk-to-climb entry for a partially constructed route.
/// Ordinary navigation owns only the sweep-clear approach to `entry`; the
/// existing mount transaction takes over at the bounded handoff.
#[derive(Clone, Copy, Debug)]
pub struct EmergencyPartialRouteEntry {
    pub owner: Uid,
    pub frontier: JobId,
    pub entry: Vec3<i32>,
    pub top_z: i32,
    pub started_tick: u64,
}

/// bastion (task #55, 2026-07-30): one designation Region the auto-access
/// planner gave up on. See `JobBoard::blocked_regions`.
#[derive(Clone, Debug)]
pub struct BlockedRegionInfo {
    pub region: Region,
    pub blocking_cell: Vec3<i32>,
    /// bastion (task #56, 2026-07-30): the chat notification is deferred
    /// to the next arbitration cycle (which has `chat_emitter` in scope)
    /// rather than required at insertion time -- lets ANY code path
    /// record a block (e.g. `place_chop_fell`'s pre-designation
    /// reachability gate, which has no emitter access) without needing
    /// plumbing to reach one. `false` until the deferred drain fires it.
    pub notified: bool,
    /// bastion (task #61, 2026-08-03, Opus's catch): which mechanism
    /// recorded this entry -- currently always "plan_access" (the
    /// carve-planner failure site, the only producer). Added when a
    /// SECOND candidate producer (a task #61 lazy chop probe) was built
    /// alongside this one; without attribution, two mechanisms both
    /// landing in `blocked_regions` would be INDISTINGUISHABLE, making
    /// "is this cell blocked" a void test of which one actually fired --
    /// exactly the void-control defect that killed the cascade row. That
    /// second mechanism was measured and then parked (n=0 demonstrated
    /// cases; see `place_chop_fell`'s comment for the full history) --
    /// this field is kept because it's cheap, has zero runtime cost, and
    /// is the instrument that will answer the same question again the
    /// moment a real second producer exists. Report-only, never read for
    /// behavior.
    pub source: &'static str,
}

/// bastion (ARB-ATTEMPT-01, per-attempt record, 2026-08-04, spec at
/// `readme/INSTRUMENT-PER-ATTEMPT-RECORD-spec.md`, corrected after a
/// real structural finding -- the spec's "seven release sites" counted
/// `job.claimed_by = None` assignments and missed that one of those
/// seven is a SHARED CONSUMER (`to_release`) fed by 26 separate
/// producers, each with its own reason invisible at the consumer).
/// `Other` is the step-1 placeholder every producer starts with --
/// replaced one call site at a time as each is read, never inferred.
/// Report-only; never gates `pass`, no world writes, same contract as
/// task #59's counters.
/// bastion (entity event log, ROW-ENTITY-EVENT-LOG, Opus's catch
/// 2026-08-10): deliberately NOT `Serialize`/`Deserialize` here. `JobBoard`
/// is documented runtime-only (not serialized, not recorder-sampled), and
/// this enum is actively growing (`TargetChanged` added 2026-08-04 as a
/// 4th producer; the mover investigation may add a 5th) -- deriving serde
/// on the live gameplay type would freeze a still-discovered enum into a
/// save-format schema the moment stage 3's promoted-entity persistence
/// lands, turning every future variant add/rename into a save migration.
/// The event log serializes through `bastion_entity_event_log
/// ::ReleaseReasonV1` instead, an explicit versioned wire copy with its own
/// `From<ReleaseReason>` -- adding a variant here stays a one-line map
/// entry there, not a migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReleaseReason {
    /// Step-1 placeholder -- the true reason at this call site hasn't
    /// been classified yet. A nonzero count of these after step 2 is
    /// complete would mean a site was missed, not that it's genuinely
    /// unclassifiable.
    Other,
    /// bastion (step 2, 2026-08-04, scoped to sites that actually fired
    /// on seeds 71/66 via the `line!()` site-scan): the routine stuck-
    /// timeout/churn release path (the `job.unreachable = true; ...
    /// "job unreachable — claim released"` site). A COMPETITION reason --
    /// the colonist tried, timed out, released. Shape A's own signature.
    TimedOut,
    /// bastion (step 2, 2026-08-04): the job COMPLETED successfully
    /// (`board.remove_job` called for genuine completion, immediately
    /// before this push). Not a failure at all -- a positive outcome
    /// that happens to flow through the same `to_release` drain as every
    /// other release reason.
    Completed,
    /// bastion (step 2, 2026-08-04): the colonist's `active.job` no
    /// longer resolves to a live job on the board at all (`board.jobs
    /// .get_mut` returned `None`) -- "cancelled out from under the
    /// colonist" per the site's own comment. NOT the colonist's own
    /// timeout -- something ELSE removed the job (a region cancel, or
    /// task #57's phantom-job sweep retiring it while this colonist
    /// still held it). The shape-C CANDIDATE: a downstream mechanism
    /// caused the release, not competition/timeout at this colonist.
    RemovedExternally,
    /// bastion (step 2b, 2026-08-04, found closing seed 66's `Other:1`
    /// site-scan gap): the job's own target block changed mid-travel
    /// (site's own comment: "job moot mid-travel — target block changed;
    /// dropped"). Distinct from all three above -- not a timeout, not a
    /// completion, and the job still resolves on the board (unlike
    /// `RemovedExternally`) but the WORLD moved out from under it. A 4th,
    /// previously-undiscovered producer, not a residual bucket.
    TargetChanged,
}

/// The job board resource.
#[derive(Default)]
pub struct JobBoard {
    /// T1.3/T1.10 (T1-001): the command admission ledger for job-completion
    /// commands routed through the CommandReceipt/CommandStatus lifecycle.
    /// Runtime-only (JobBoard is not serialized and not recorder-sampled),
    /// pruned per completion (`forget`) so it stays bounded; the monotonic
    /// command-id counter never reuses ids.
    pub command_admission: common::command_protocol::AdmissionLedger,
    pub next_id: JobId,
    pub jobs: HashMap<JobId, Job>,
    /// bastion (AUTON-2 unification, FIXTURE 1's invariant made live in
    /// EVERY scenario, 2026-08-08): a silent, cumulative counter --
    /// incremented at the EXISTING orphan sweep's own cadence
    /// (`ARBITRATION_INTERVAL`, ~2 Hz), not a new per-tick scan (the
    /// acceptance spec's own budget rules that out: "settle-time
    /// reads only... no per-tick"). The sweep already computes this
    /// invariant's exact population for its own purposes (self-jobs
    /// with no claimant, about to be removed because nothing else can
    /// reach them) -- counting it there is free. NOT a debug_assert:
    /// pre-unification this invariant is EXPECTED to fire broadly
    /// (GUARD-6 site 1 makes self-jobs unconditionally unselectable, so
    /// today the invariant reduces to "no self-job may be unclaimed" --
    /// any release path, including preempt_scenario's own designed
    /// ENDURE degradation, legitimately trips it). Read-only via
    /// `Server::bastion_settle_invariant_violations` (a live snapshot
    /// at one instant) and this cumulative count (every sweep across
    /// the whole run, so a violation that self-heals before the next
    /// harness poll is still counted); becomes a real regression guard
    /// once GUARD-6 unification lands and the count should hold at 0.
    pub settle_invariant_violations: u64,
    /// bastion (B5.8): the union of placed designation volumes — the
    /// colony's terrain-claim mask. Auto carve-steps (self-rescue) is
    /// confined to this mask (expanded by the stair's own rise), so the
    /// system never carves wilderness to chase an out-of-scope target.
    /// Maintained by place (append) / cancel (exact AABB subtraction —
    /// the unit-tested `Region::subtract`).
    /// THE COLONY'S STANDING ORDERS: every live designation region paired
    /// with the KIND that was painted there.
    ///
    /// The kind rides with the region rather than in a parallel log,
    /// deliberately. `cancel_region` removes designations by INTERSECTION
    /// and subtracts AABBs — a second structure would have to replicate
    /// that predicate forever, and two structures that must agree is the
    /// drift this codebase has been bitten by more than once. One store,
    /// one `retain`, one truth.
    ///
    /// This is also what colony persistence serialises: an order is
    /// durable, a job is transient work derived from it.
    pub designated: Vec<(Region, DesignationKind)>,
    /// COLONY PERSISTENCE: orders read back from the save that have not yet
    /// been replayed, because `place_designation` needs a `TerrainGrid` and
    /// at server start no chunks are loaded.
    ///
    /// The rtsim tick SEEDS this (it can see the save); the bastion tick
    /// DRAINS it (it can see the terrain). Neither system can do both, which
    /// is why the queue exists rather than a direct restore.
    pub pending_restore: Vec<(Region, DesignationKind)>,
    /// Whether the one-shot seed has run this server lifetime. Without it
    /// the seed would re-add orders every tick, and a cancelled designation
    /// would resurrect itself.
    pub restore_seeded: bool,
    /// bastion (task #55, blocked-designation visibility, 2026-07-30): a
    /// designation Region the auto-access planner has given up on (its
    /// `plan_access` call returned `None`) -- names the specific cell that
    /// blocked it, so inspecting ANY job whose pos falls inside the Region
    /// answers "blocked by X at (x,y,z)" instead of only the one cell whose
    /// own carve attempt failed knowing it's unreachable. A Vec, not a map,
    /// keyed by equality on `region` (Region has no Hash impl; colonies are
    /// small, a linear scan is fine) -- checked before insert so the SAME
    /// region isn't re-recorded (and re-notified) every tick it stays
    /// blocked. Cleared when the region is cancelled (see the cancel path).
    pub blocked_regions: Vec<BlockedRegionInfo>,
    /// bastion (task #59, starvation measurement, 2026-07-30): per-job-cell
    /// arbitration-cycle counters testing the greedy-starvation hypothesis
    /// (no cooldown/penalty after a failed attempt -- a hard cell just
    /// loses the score comparison every cycle while easier unclaimed work
    /// exists). `starvation_cycles` = cycles this cell was open+unclaimed;
    /// `starvation_crowded_cycles` = of those, how many had at least one
    /// OTHER unclaimed job competing. A ratio near 1.0 supports the
    /// hypothesis; a cell unattempted for many cycles with an EMPTY field
    /// (crowded_cycles far below starvation_cycles) is Fable's kill case.
    /// Report-only, never gates `pass`, no world writes.
    pub starvation_cycles: HashMap<Vec3<i32>, u32>,
    pub starvation_crowded_cycles: HashMap<Vec3<i32>, u32>,
    /// Cycles since this cell's job was last actually claimed (reset to 0
    /// on claim, incremented once per arbitration cycle while unclaimed).
    pub cycles_since_last_claim: HashMap<Vec3<i32>, u32>,
    /// bastion (task #59, aging mechanism-level check, 2026-07-30): how
    /// many times this position was actually claimed this run -- the
    /// "times offered" Fable asked for, to show whether aging is
    /// increasing ATTEMPTS on previously-starved cells (not just changing
    /// which seeds pass).
    pub claims_by_pos: HashMap<Vec3<i32>, u32>,
    /// bastion (B5.8): ACCESS ANCHORS — base cells of the colony's vertical
    /// links (auto-built ladder pillars + player-built ladder lines). Travel
    /// stages an over-reach ascent through the nearest anchor (walk there
    /// flat, then let the climb assist do the vertical) because the vanilla
    /// incremental A* resets whenever the agent moves >2 blocks — an agent
    /// beelining-then-bobbing at a wall never finishes the search that
    /// would have found the ladder (b58 run-10 root cause).
    pub access_anchors: Vec<Vec3<i32>>,
    /// bastion (B5.8-E): per-colonist emergency-egress watch — (last anchor
    /// position, seconds stationary, egress already attempted). Jobless
    /// colonists have no travel watchdog and zone deletion empties the
    /// claim mask, so a trapped digger needs a trigger + permission source
    /// independent of BOTH (Ben's live-test entombment repro).
    pub egress_watch: HashMap<Uid, (Vec3<f32>, f32, bool)>,
    /// bastion (B5.8-E3): per-colonist CLAIM-CHURN watch — (anchor
    /// position, consecutive unreachable releases without leaving it). The
    /// stillness timer can't see a colonist that cycles claim→unreachable→
    /// re-claim (it reads as employed at nearly every pass, and its brief
    /// jobless windows rarely coincide with the sampling tick); the churn
    /// COUNT is the loop's own signature. Threshold → an on-the-spot
    /// annulus test → an egress request, employed or not.
    pub churn_watch: HashMap<Uid, (Vec3<f32>, u8)>,
    /// bastion (B5.8-E3): egress requests raised OUTSIDE the sampling pass
    /// (the churn detector fires from the every-tick upkeep loop); drained
    /// into the next egress pass, which owns one-plan-at-a-time gating.
    pub egress_pending: Vec<(Uid, Vec3<i32>, Vec3<i32>)>,
    pub egress_verdicts: HashMap<Uid, u64>,
    /// bastion (item 4b split, Fable's ruling 2026-08-11): `egress_verdicts`
    /// above unions two unrelated producers -- the churn detector's fire (at
    /// most one per pass, an emergency-annulus verdict) and the periodic
    /// `egress_scan` pass's own verdict (below-grade watch resolving to
    /// walkable/rim ground). Additive, not a replacement: `egress_verdicts`
    /// keeps its existing role in `terminal_cause`'s classification
    /// unchanged; these two split counters exist so a stuck-colonist
    /// specimen's ULTIMATE FAIL-SAFE line can distinguish "this colonist's
    /// verdicts were mostly churn-fires" from "mostly scan passes" without
    /// re-deriving it per incident.
    pub egress_verdicts_churn: HashMap<Uid, u64>,
    pub egress_verdicts_scan_ok: HashMap<Uid, u64>,
    pub egress_plans_emitted: HashMap<Uid, u64>,
    pub egress_no_route: HashMap<Uid, u64>,
    /// Stable non-destructive surface target for an active jobless egress
    /// episode. The climb assist reads this directly so horizontal intent
    /// does not depend on the idle NPC pathfinder preserving a `Goto`.
    pub egress_targets: HashMap<Uid, Vec3<i32>>,
    /// Best 3-D distance reached toward the stable egress target. Only a
    /// real improvement here may reset the universal teleport timer; random
    /// below-grade wandering remains unable to postpone rescue.
    pub egress_best_distance: HashMap<Uid, f32>,
    pub egress_no_progress_secs: HashMap<Uid, f32>,
    /// Provenance for temporary humanitarian carving. Pending job ids and
    /// original terrain cells are kept outside `Job`/resource accounting so
    /// ordinary B5.5 mining remains exactly conserved.
    pub emergency_access_jobs: HashMap<JobId, Uid>,
    pub emergency_access_cells: HashMap<Vec3<i32>, (Uid, Block)>,
    pub emergency_cleanup_pending: HashSet<Uid>,
    /// Colonist uid -> shared route owner. Multiple trapped colonists may
    /// use one temporary route; terrain survives until this set is empty.
    pub emergency_route_members: HashMap<Uid, Uid>,
    /// Route owner -> planner-selected permanent surface destination. This is
    /// not inferred from provenance cells because a stair's highest carved
    /// cell is head clearance rather than a standable waypoint.
    pub emergency_route_targets: HashMap<Uid, Vec3<i32>>,
    /// Route owner -> planner-preflighted adjacent ladder body lane at its
    /// base. Traversal must approach this cell before rising; recomputing a
    /// side after construction can choose the opposite side and cut
    /// diagonally through the solid ladder corner (smoke31b).
    pub emergency_route_mounts: HashMap<Uid, Vec3<i32>>,
    /// Route owner -> planner/executor contract. A route without this record is
    /// never allowed to fall through to an arbitrary permanent-target Goto.
    pub emergency_route_descriptors: HashMap<Uid, EmergencyRouteDescriptor>,
    /// Route owner -> pre-emission shipping-A* proof from the settled source
    /// to the selected constructed-ladder body lane. Runtime copies and
    /// revalidates this exact corridor; it never invents a post-build lane.
    pub emergency_route_approach_corridors: HashMap<Uid, Vec<Vec3<i32>>>,
    /// Route-local emitted order. Pending jobs remain in
    /// `emergency_access_jobs`; this immutable sequence lets REQ-0069 prove a
    /// contiguous completed prefix without guessing from height or HashMap
    /// iteration order.
    pub emergency_route_sequences: HashMap<Uid, Vec<(JobId, Vec3<i32>)>>,
    /// Traverser uid -> route-owned ladder transaction. While present this is
    /// the sole writer for the bounded mount/climb/top-exit movement mode.
    pub bastion_traversal_tasks: HashMap<Uid, BastionTraversalTask>,
    pub emergency_partial_route_entries: HashMap<Uid, EmergencyPartialRouteEntry>,
    pub emergency_approach_corridors: HashMap<Uid, EmergencyApproachCorridor>,
    /// Last skill-adjusted climb constants observed from the authoritative
    /// `CharacterState::Climb` for this route member. REQ-0073 uses this only
    /// to report the real energy/time geometry behind a recovery decision; it
    /// never changes Energy or climb behavior.
    pub emergency_climb_profiles: HashMap<Uid, (f32, f32)>,
    /// Traverser -> claimed next frontier whose previous rung completed after
    /// authoritative ladder contact was lost.  While present, the existing
    /// supported-entry/Goto/mount pipeline owns reacquisition and ordinary
    /// distance-based emergency arrival is suppressed.
    pub emergency_frontier_reacquire: HashMap<Uid, JobId>,
    /// BACKSTOP-OPT (B): consecutive fruitless traversal aborts per member.
    /// Cleared on frontier completion or verified dismount; on exceeding the
    /// bound the member is released to the independent failsafe tier instead
    /// of re-engaging forever (a genuinely-impossible route must not loop).
    pub emergency_reengage_aborts: HashMap<Uid, u32>,
    /// BACKSTOP-OPT: ticks spent in the REQ-0071 energy-recovery wait per
    /// member. The wait is a DESIGNED bounded non-progress state (idle energy
    /// strictly regenerates, the gate WILL pass) — the stuck-watch holds
    /// during it, but only up to a cumulative bound so a pathological
    /// never-recovering wait still reaches the independent failsafe net.
    /// (The old broken release path was masking this wait from the watch by
    /// accident — N1B regression under the (A) fix.)
    pub emergency_energy_wait_ticks: HashMap<Uid, u32>,
    /// BACKSTOP-OPT (B): STICKY exhaustion — a member the re-engage bound
    /// released is barred from ALL new route ownership/membership until
    /// DELIVERED (failsafe fire or verified dismount clears it). Without
    /// this, re-emission started a fresh engagement with fresh counters and
    /// the outer loop was unbounded (corpus C-leg: stranded the full budget).
    pub emergency_reengage_exhausted: HashSet<Uid>,
    /// BACKSTOP-OPT (C): members whose LAST abort had no escape progress
    /// since the previous abort (no frontier completion, no TopExit reach).
    /// In-set = the energy-wait hold is DENIED (a hopeless cycler's watch
    /// must accrue so the net delivers under the bar); absent = benefit of
    /// the doubt (first engagements and productive cycles keep the hold).
    /// Cleared at every real-progress site and at delivery.
    pub emergency_no_progress: HashSet<Uid>,
    /// STATUS-SURFACE: display-only per-member status, tick-stamped by its
    /// ONE writer — the granted energy-wait hold (re-stamped every tick the
    /// wait is real; queued-for-link is instead DERIVED from durable
    /// transaction phase at read time). Readers expire entries older than
    /// [`STATUS_DISPLAY_TTL_TICKS`], so no clear sites exist or are needed —
    /// expiry IS the wait ending. NEVER read by sim logic (the CHOP-PROGRESS
    /// sim-inert discipline).
    pub status_display:
        HashMap<Uid, (common::comp::bastion::BastionColonistStatus, u64)>,
    /// Airborne pre-route owners establishing a physically supported origin.
    /// This state remains visible to the deep harness so a leaked/pending
    /// settle episode can never be mistaken for clean `[0,0,0]` teardown.
    pub emergency_settle_anchors: HashMap<Uid, EmergencySettleAnchor>,
    /// Consecutive one-second samples at a verified stable exit. Temporary
    /// terrain is not restored on first rim contact: doing so removed the
    /// ladder under the climber and dropped it back into the pocket.
    pub emergency_safe_secs: HashMap<Uid, f32>,
    /// bastion (B6, reviewer F3): consecutive seconds the access economy
    /// has been IDLE (access jobs exist, none claimed). A stale abandoned
    /// plan — e.g. a half-carved egress staircase nobody needs after the
    /// crew found another way out — would otherwise freeze one-plan-at-a-
    /// time colony-wide forever AND sit flagged unreachable on the board.
    pub access_idle_secs: f32,
    /// bastion (ITEM 2, ROW-ITEM2-STALL-COUNTER-PACKET / ROW60-FIX-
    /// PROPOSAL option 2): consecutive seconds branch C (a claimed access
    /// job exists) has held with NO claimed access job making net
    /// progress. `access_idle_secs` only ever sees an UNCLAIMED plan — a
    /// colonist that claims an access job and then stalls (measured
    /// specimen: `job=703`, 22,080 ticks / ~12 min, `CLAIM` to `RELEASE`)
    /// resets that clock unconditionally and keeps the whole plan alive
    /// forever. Reset is EARNED by a same-job `progress` increase via
    /// `access_job_progress` below, same net-progress-with-hysteresis
    /// discipline as `stuck_job_progress` — never by claim-holding alone,
    /// never by any-movement (Fable's B6 constraint: a sub-block-wobble
    /// reset here would rebuild the bug this row exists to kill, one
    /// layer up).
    ///
    /// `pub` (Opus's finding, WAVE33-RESULTS.md, 2026-08-10): the PEAK
    /// alone (`b5_f3_stalled_peak`) cannot distinguish "stalled N seconds
    /// then recovered" from "still stalling when the run ended" -- same
    /// peak value, opposite meanings, and wave 33's seed 59 is exactly
    /// this ambiguity (`stalled_peak = 119.0`, no prune, unresolved). This
    /// CURRENT value, read at whatever moment the harness snapshots it
    /// (typically run end), is the disambiguator: near the peak means
    /// still stalling; near zero means recovered.
    pub access_stalled_secs: f32,
    /// bastion (ITEM 2): last-seen `progress` per currently-claimed
    /// access `JobId`, the per-job counterpart of `stuck_job_progress`
    /// (that one is per-colonist, for the teleport watchdog; this one is
    /// per-job, for the F3 stall counter above — different cadence,
    /// same earned-reset pattern). `JobId`s are never reused
    /// (monotonic), so nothing here is self-correcting: `remove_job`
    /// does not touch this map, so an insert-only implementation grows
    /// with every access job EVER claimed across a run, not the number
    /// alive (Opus's catch — the entries themselves are never read once
    /// stale, but that is not the same claim as bounded). Pruned to the
    /// live claimed set every F3 pass instead (`retain`, right after the
    /// insert loop below) — small per entry, but "small × forever" is
    /// exactly the shape a multi-day item-8 endurance run would surface
    /// and a short run never will.
    pub access_job_progress: HashMap<common::bastion::JobId, f32>,
    /// bastion (#68, port row): last-known claimant per LIVE access job,
    /// diffed each pass against current state to emit CLAIM/RELEASE
    /// events (`access_claim_diag_enabled()`). Maintained only while the
    /// diag is on (see the diff site) -- empty and untouched otherwise.
    /// Not persisted; a restart mid-claim rebuilds from empty, at worst
    /// one false CLAIM burst on the next pass, never a false RELEASE.
    pub access_claim_state: HashMap<JobId, Uid>,
    /// bastion (#68 amendment, Opus/#60 falsifier prereg): last-SEEN
    /// `F3PruneBranch`, updated every pass regardless of any diag flag
    /// (#70: `b5_f3_transitions` below needs this on corpus runs, which
    /// never set `BASTION_ACCESS_CLAIM_DIAG`). Only the `F3-BRANCH` LOG
    /// LINE stays gated on `access_claim_diag_enabled()`; the state
    /// write and the transition count do not. `None` only before the
    /// first pass ever runs.
    pub access_branch_state: Option<F3PruneBranch>,
    /// bastion (#70, ROW60-F3-CORPUS-FIELDS-PACKET): six pure
    /// accumulators over the F3 pruner's branch arms, exposed to the
    /// corpus via `bastion_f3_prune_stats` -- the wave-fan transport
    /// carries stdout JSON only (stderr, where `F3-BRANCH` writes, is
    /// discarded on every seed), so this is the ONLY route this data
    /// has to a wave. DIAGNOSTICS, not verdict terms: never enter the
    /// harness's `clauses` vec (see that packet's review check).
    /// Ticks spent in the `MaterialHeld` branch this run.
    pub b5_f3_ticks_branch_a: u64,
    /// Ticks spent in the `Idle` (accruing) branch this run.
    pub b5_f3_ticks_branch_b: u64,
    /// Ticks spent in the `ClaimedOrAbsent` branch this run.
    pub b5_f3_ticks_branch_c: u64,
    /// Branch changes over the run (a healthy colony changes a
    /// handful of times; a pinned defect never changes at all).
    pub b5_f3_transitions: u32,
    /// Max `access_idle_secs` EVER reached, captured before any reset
    /// -- the number the pruner's `ACCESS_STALE_SECS` threshold
    /// actually turns on.
    pub b5_f3_idle_peak: f32,
    /// bastion (ITEM 2, ROW-ITEM2-STALL-COUNTER-PACKET, Opus's catch
    /// 2026-08-10): `access_stalled_secs`'s sibling to `b5_f3_idle_peak`
    /// above -- same capture-before-any-reset discipline, same reason.
    /// Without this the wave-fan corpus (stdout JSON only, stderr where
    /// the live `F3-BRANCH` diag writes is discarded) has no route to
    /// this data at all, and `ACCESS_STALL_SECS` could never be set from
    /// measured seeds -- only from the single job=703 specimen it was
    /// explicitly ruled NOT to be calibrated from.
    pub b5_f3_stalled_peak: f32,
    /// Times the pruner actually removed a stale plan.
    pub b5_f3_prunes_fired: u32,
    /// bastion (DECISIONS #89, ROW69-OPTION-B-PACKET): the planted-
    /// failure feature-acceptance measure -- distinct colonist `Uid`s
    /// that ever completed an `EatFrom` ("ate — hunger restored") this
    /// run. Stack of N units, M hungry colonists should yield
    /// `min(N, M)` distinct eats; before this row a single stack fed
    /// at most one colonist regardless of N. DIAGNOSTIC under #88
    /// (`b5_eat_completions_distinct` in the corpus report): never a
    /// `clauses` entry.
    pub b5_eat_completions_distinct: HashSet<Uid>,
    /// ITEM8-CRASH-FINDING.md fix acceptance (Opus's ask, v3 prereg
    /// review): the PRECONDITION witness for the planted crash test's
    /// live trigger population. A silent `debug_assert` in `try_merge`
    /// cannot by itself distinguish "the fix worked" from "the trigger
    /// never occurred" (the sit-trap lesson applied to this row) --
    /// incremented every time `split_off_one` returns `Some` (a real
    /// split happened, i.e. a `PickupItem` briefly existed in the exact
    /// state that used to be unmergeable). Zero across a scored window
    /// means the run did not exercise the fix and must be read as VOID
    /// on this specific claim, never as proof the fix held.
    pub b5_split_off_one_fired: u32,
    /// Item 8 pre-flight: last-observed "rest below its interrupt
    /// threshold" state per colonist, the edge-detector state for the
    /// `NeedCrossed{Rest, _}` entity-event producer (need-order loop,
    /// candidates computation). Missing entry reads as `false` (not in
    /// band) -- correct default: a fresh colonist's first sub-threshold
    /// observation must still fire `Into` honestly.
    pub need_interrupt_rest: HashMap<Uid, bool>,
    /// Sibling of `need_interrupt_rest` for hunger.
    pub need_interrupt_hunger: HashMap<Uid, bool>,
    /// bastion (#89): the max simultaneous LIVE reservation count ever
    /// observed against any single item entity this run -- how
    /// contested the most-contested stack got. Updated inside `reserve`
    /// right after insertion (the true peak, same "capture before
    /// anything can shrink it back down" discipline as `b5_f3_idle_peak`,
    /// though nothing here ever un-peaks mid-run since reservations only
    /// grow between `reserve` calls). DIAGNOSTIC under #88: never a
    /// `clauses` entry.
    pub b5_stack_reserved_units_max: u32,
    /// bastion (ROW-ITEM6-WITNESS-PACKET part B1, Fable-named, 2026-08-10):
    /// item-6 pickup-refusal witness -- one counter PER VERDICT REASON,
    /// incremented at the same `record_pickup_verdict` call sites in
    /// `server/src/events/inventory_manip.rs` that already compute the
    /// verdict (a counter beside an existing decision point, not a new
    /// one). DELIBERATELY separate, never summed into a total: which LAYER
    /// refused is the entire diagnostic value a combined `refusals_total`
    /// would erase. Board accumulators, not flight-recorder events -- the
    /// proven #70 pattern; the flight recorder is gated off in a corpus
    /// fan and carries free-text notes a fan can't tally, which is why
    /// item 6 was invisible to wave33 at all despite the refusals being
    /// real and logged.
    ///
    /// FLAT, not split by picker class (Opus proposed a colonist/ambient
    /// split here, then withdrew it after my catch, `f12abbd333` ->
    /// ruling 2026-08-10): both this reason's and
    /// `ambient-loot-disabled`'s refusal `if` already require
    /// `bastion_colonists.get(entity).is_none()` to enter the branch, so a
    /// `_colonist` counter placed inside either one reads the SAME
    /// component the gate just tested, at the SAME instant -- 0 by
    /// construction, not evidence of anything. A counter inside a branch
    /// cannot vary on a predicate that branch has already fixed. See
    /// `b5_pickup_refused_ambient_uids` below for the timing-race check
    /// this became instead.
    pub b5_pickup_refused_pile_protected: u32,
    /// B1: `"ambient-loot-disabled"` -- #97's global ambient-loot gate's
    /// server-side belt-and-suspenders layer. FLAT for the same reason as
    /// `b5_pickup_refused_pile_protected` above.
    pub b5_pickup_refused_ambient_disabled: u32,
    /// B1: `"ambient-loot-disabled"`, THE TIMING-RACE WITNESS (replaces
    /// the withdrawn `_colonist` split, ruling 2026-08-10): every distinct
    /// picker `Uid` refused under this reason, mapped to the TICK of its
    /// first refusal. Unlike a same-instant counter, this is checked at a
    /// DIFFERENT time than the branch predicate -- see
    /// `bastion_item6_ambient_refusal_recheck`'s own doc for the deferred
    /// read this enables, which is what makes "was this uid a colonist
    /// LATER" a real, non-tautological question.
    pub b5_pickup_refused_ambient_uids: HashMap<Uid, u64>,
    /// B1: `"loot-owned"`, colonist picker -- KEPT split (Opus's ruling:
    /// this one is real signal, unlike the other two). `loot_owner
    /// .can_pickup` goes through groups/alignments/stats/players and never
    /// touches `bastion_colonists`, so a colonist genuinely CAN be refused
    /// here for an unrelated reason -- this branch's predicate does not
    /// fix the value the way the other two do.
    pub b5_pickup_refused_loot_owned_colonist: u32,
    /// B1: `"loot-owned"`, non-colonist picker. See
    /// `b5_pickup_refused_loot_owned_colonist`'s doc.
    pub b5_pickup_refused_loot_owned_ambient: u32,
    /// bastion (ROW-ITEM6-WITNESS-PACKET part B2, Fable-named): the pair
    /// that makes the row FALSIFIABLE -- a refusal count alone cannot
    /// distinguish "protection working" from "nobody ever tried to take a
    /// pile." Incremented at the `"accepted"` verdict site, only when the
    /// picked-up item is a `BastionPile`, split on whether the picker is a
    /// colony member. Expected shape under membership-only protection:
    /// `by_member` can be any value, `by_nonmember` must be EXACTLY 0 --
    /// see the acceptance framework's invariant.
    pub b5_pile_pickup_by_member: u32,
    /// B2: a non-member successfully took from a `BastionPile`. **Must be
    /// 0 in every seed** under #96/#97's protection -- a nonzero value here
    /// is the protection leaking, not a diagnostic curiosity. See
    /// `b5_pile_pickup_by_member`'s doc.
    pub b5_pile_pickup_by_nonmember: u32,
    /// bastion (B-LIVE3, mine lifecycle): designations that reached DONE
    /// (last non-access job completed). Telemetry for the harness/UI.
    pub done_count: u64,
    /// bastion (FR15 instrumentation): locomotion baseline counters —
    /// REPORTED telemetry, never gated. `no_progress_ticks` = travel-upkeep
    /// ticks spent not improving toward the current steer (the A*-bob
    /// signature accrues here); `travel_timeouts` = watchdog trips
    /// (`stuck_time > STUCK_TIMEOUT`); `failsafe_teleports` = ULTIMATE
    /// FAIL-SAFE fires. Baselined BEFORE the fix-1 movement change
    /// (playbook: instrument failure/progress first), compared after.
    /// This stayed true even through a brief 2026-07-30 detour:
    /// `b5_scenario` (bastion-harness/src/main.rs) briefly gated
    /// `failsafe_teleports == 0` inside the mine-completion invariant, an
    /// Opus fan-corpus review caught it conflating two unrelated failure
    /// classes, and it was reverted to report-only (`b5_rescue_fired`) —
    /// exactly what this doc always said. Keep it that way until a
    /// measured base rate justifies a real threshold.
    pub no_progress_ticks: u64,
    pub travel_timeouts: u64,
    /// bastion (mechanism-2 friction instrument, Ben/Fable-directed,
    /// 2026-07-30): per-job-POSITION travel-timeout tally — cheap always-on
    /// counters, distinct from the verbose BASTION_LEGC_DIAG log path
    /// (which stays env-gated; counters cost an increment, the log path
    /// costs real time and would perturb the run's timing profile).
    /// `max(values)` at report time is the TAIL signature: a target that
    /// keeps getting retried and never resolves, vs. ambient one-off
    /// friction spread across many targets. Keyed by position (not job id,
    /// which churns as jobs complete/get recreated) so "same target"
    /// literally means the same position. `.values().max()` is a
    /// commutative reduction, so HashMap iteration order never leaks into
    /// the reported value — no determinism surface despite the hash map.
    pub timeout_counts_by_pos: HashMap<Vec3<i32>, u32>,
    /// bastion (mechanism-2 terrain probe, Fable-directed, 2026-07-30):
    /// closest approach EVER achieved toward each job position, across
    /// every claim attempt (not reset per-claim, unlike the watchdog's own
    /// `active.best_dist`). A PURE position measurement -- shares no
    /// dependency with `has_standable_stance`, so it can catch what a
    /// path-exists probe built on that predicate structurally cannot: the
    /// colonist arriving close (predicate wrong / work-start bug) vs.
    /// never getting near (genuine travel/pathing failure). Updated every
    /// tick a colonist is actively traveling toward a job, unconditionally
    /// -- cheap (one comparison + maybe one write per active traveler).
    pub min_distance_to_target: HashMap<Vec3<i32>, f32>,
    /// bastion (mechanism-2 terrain probe, Fable-directed, 2026-07-30): the
    /// colonist's ACTUAL position at the moment of each job's most recent
    /// travel timeout. Lets the offline reachability probe run from where
    /// the failing attempt actually stood, not just from the colony's
    /// spawn point -- "reachable from spawn" and "reachable from here" are
    /// different questions, and only the second matches the observed
    /// failure.
    pub last_timeout_pos: HashMap<Vec3<i32>, Vec3<f32>>,
    /// bastion (mechanism-2 terrain probe, Fable-directed, 2026-07-30): the
    /// last question the corrected reachability probe can't answer alone --
    /// does the live A* fail to FIND a route the probe proves exists, or
    /// does it find one the mover then fails to EXECUTE? Reads the
    /// Chaser's own existing `diagnostic_snapshot()` (`common/src/path.rs`,
    /// already read-only, already built for exactly this) at EVERY timeout
    /// on this position (not just the most recent -- Fable's refinement:
    /// `route_next_idx` PINNED across successive timeouts means stuck at
    /// one waypoint; ADVANCING means real progress along a route that
    /// still times out, a different failure than getting stuck). Each
    /// entry is `(route_target.is_some(), route_complete,
    /// route_next_idx)`. No route at all points at the search itself
    /// never producing one, consistent with TGT-DRIFT's astar-reset
    /// repeatedly discarding whatever was found.
    pub timeout_route_states: HashMap<Vec3<i32>, Vec<(bool, Option<bool>, Option<usize>)>>,
    /// bastion (DPA-2 §5): the classified access-block reason — `Some(def)`
    /// while the descent frontier is HELD because dig-provisioned rung jobs
    /// need `def` (wood) and the colony has none reservable; `None`
    /// otherwise. Recomputed every gate pass (self-clearing — no stale-slot
    /// risk). Read by the inspector + the harness probe; never read by sim
    /// logic outside the gate pass that writes it.
    pub access_material_missing: Option<&'static str>,
    /// bastion (LEG-C amnesty fix): per-job amnesty bookkeeping —
    /// `(consecutive fruitless strikes, face-neighbor solidity fingerprint
    /// at last sweep)`. Entries live only while a job is unreachable
    /// (removed the sweep it clears); keyed access only, never iterated
    /// (no hash-order determinism surface).
    pub amnesty_dormancy: HashMap<JobId, (u8, u8)>,
    /// bastion (LEG-C amnesty fix, form (C)): consecutive sweeps in which
    /// the tracked unreachable set saw ZERO fingerprint change — the
    /// set-level quiet counter that grants/withholds the amnesty.
    pub amnesty_set_quiet: u8,
    /// bastion (guard-generalization row, 2026-08-04, Fable-ruled second
    /// limb): the tick a job FIRST missed the leave-unclaimed guard
    /// (`job.affordance != Untargeted && !standable.contains_key(&id)`),
    /// still unclaimed. This is the visibility this guard was missing —
    /// the amnesty sweep can't see these jobs (its own first line filters
    /// to `job.unreachable == true`, and a benched-not-claimed job is
    /// never flagged unreachable by design, see the guard's own comment)
    /// and nothing else tracked WHEN a stance-less job started waiting.
    /// Cleared the instant the job is claimed, gets a stance, or is
    /// removed — a resolved job carries no history. Report-only (surfaced
    /// via `BastionJobInspect::benched_since_tick`), never read for
    /// behavior — same discipline as `BlockedRegionInfo::source`.
    pub benched_since: HashMap<JobId, u64>,
    // ROW B (2026-08-04) originally lived here as `amnesty_grants_owed:
    // HashMap<JobId, u32>` -- WITHDRAWN 2026-08-04 after the 48-seed
    // paired A/B (seed 76: base 0 crossings/27-27/PASS -> variant 1
    // crossing/26-27/FAIL): the per-tick grant-loop decrement perturbed
    // timing enough to MANUFACTURE a threshold crossing that would not
    // otherwise have happened -- the observer effect, in the mechanism
    // itself rather than a diagnostic this time. REPLACED by ROW B′:
    // `Job::benched_until_tick` (common/src/bastion.rs), a field ON the
    // job (no map, no lookup, no per-grant iteration) compared against
    // `tick.0` at the two sites that already run regardless of this
    // row -- see that field's own doc for the full mechanism and the
    // #60 zero-per-tick-cost budget this design is built to satisfy.
    /// bastion (observability row, DECISIONS #49, 2026-08-04): per-caller
    /// `plan_access` CALL counts, keyed by the three call sites' own
    /// labels (`"self_rescue"`, `"emergency"`, `"proactive_descent"`).
    /// Counts every invocation regardless of outcome -- paired with
    /// `access_plan_emissions` below to answer "how often does this
    /// caller even reach `plan_access`", the question #61's
    /// non-call-vs-rejection falsification needed and the corpus
    /// couldn't answer (zero access-plan state visible anywhere).
    /// Report-only, never read for behavior.
    pub access_plan_calls: HashMap<&'static str, u32>,
    /// bastion (observability row, DECISIONS #49): per-caller SUCCESSFUL
    /// `plan_access` emissions (the `Some((kind, steps))` arm), same
    /// keys as `access_plan_calls`. `emissions <= calls` always; the gap
    /// is refusals, not non-calls.
    pub access_plan_emissions: HashMap<&'static str, u32>,
    /// bastion (observability row, DECISIONS #49): times the self-rescue
    /// site's colony-global `access_pending` bar (`.take(if access_pending
    /// {0} else {1})`) starved a GENUINELY PENDING carve request -- i.e.
    /// `carve_requests` was non-empty AND `access_pending` was true, so
    /// the loop body never ran for that tick's requests at all. This is
    /// the NON-CALL half #61's falsification needed: a rejected `plan_
    /// access` result and a starved-before-ever-called request look
    /// identical downstream (both leave the job's stance/claim state
    /// untouched) without this counter naming which happened.
    pub self_rescue_starved_by_access_pending: u32,
    /// bastion (observability row, DECISIONS #49): cumulative ticks (at
    /// the self-rescue site's own cadence, i.e. once per `run()` call
    /// past the `ARBITRATION_INTERVAL` gate) `access_pending` was
    /// observed `true`. Paired with the tick count itself (read via the
    /// harness hook) to derive a duty-cycle fraction rather than a raw
    /// count that means nothing without a denominator.
    pub access_pending_true_ticks: u64,
    /// bastion (ARB-ATTEMPT-01 step 2, batch item 1, 2026-08-04):
    /// cumulative count of `to_release` drains by `ReleaseReason`,
    /// incremented once per entity at the single shared consumer (not
    /// per producer -- the reason is already attached by the time it
    /// gets here). A nonzero `Other` count after step 2's scoped
    /// classification means a producer fired that wasn't among the ones
    /// discovered on the seeds actually checked (71/66) -- expected on
    /// OTHER seeds until their own site-scan is run, not a bug. Report-
    /// only, never read for behavior.
    pub release_reason_counts: HashMap<ReleaseReason, u32>,
    /// bastion (DPA-2, the prune-gap flicker guard): anchor COLUMNS whose
    /// rung plans went material-starved — DURABLE across the F3 prune →
    /// re-emit gap (deriving the hold from live rung jobs alone left a ~2s
    /// window each cycle where the starved anchor read as release-grade and
    /// deep cells could slip out — the seed-777 leg-A teleport). Extended
    /// while wood is missing; cleared WHOLESALE the moment wood is
    /// reservable again (the gate pass is the single writer).
    pub starved_anchor_columns: HashSet<Vec2<i32>>,
    /// CARVE-CASCADE PROBE (mechanism 1, predictions A/B — Opus, 2026-07-30):
    /// PURE TELEMETRY. Nothing here is read by any decision; the board is
    /// not hashed (checked, not assumed), so these cannot move the 72/72
    /// determinism baseline or re-roll a per-seed outcome.
    ///
    /// **Why the RESET count and not just the ceiling.** Prediction B is
    /// "`emergency_reengage_aborts` never exceeds 1-2". Measured as a
    /// ceiling alone that is a FALSE ALL-CLEAR: under the amplifier
    /// hypothesis a low ceiling is the SIGNATURE of the pathology, because
    /// `frontier-complete` keeps clearing the counter ("Real progress: both
    /// per-episode bounds reset"). A cascade that refills the budget fifty
    /// times reports max=2 and looks healthy. The pair is the proof —
    /// ceiling LOW while resets CLIMB — and either number alone is
    /// misreadable.
    ///
    /// Keyed access only, never iterated in sim logic; the harness reads
    /// them through a SORTED reduction so no hash order reaches output.
    pub cascade_frontier_completes: HashMap<Uid, u32>,
    /// Times the per-episode abort bound was CLEARED at frontier-complete
    /// — the refill count, and the discriminating measurement.
    pub cascade_abort_resets: HashMap<Uid, u32>,
    /// Highest `emergency_reengage_aborts` seen before a clear (B's
    /// ceiling). Expected LOW; that is the point.
    pub cascade_abort_max: HashMap<Uid, u32>,
    /// Access plans emitted per member — the escalation count.
    pub cascade_access_emissions: HashMap<Uid, u32>,
    pub failsafe_teleports: u64,
    pub failsafe_events: Vec<FailsafeTeleportEvent>,
    /// bastion (B-LIVE3 / reviewer F5): the UNIVERSAL stuck watchdog —
    /// seconds each colonist has been continuously BELOW GRADE without
    /// working. Feeds the VERDICT-INDEPENDENT teleport backstop: no
    /// `has_egress` gate, no churn threshold racing its own reset, and —
    /// critically — NOT movement-keyed (a colonist WANDERING below grade
    /// kept resetting a stationary timer and never teleported, staying
    /// stuck: the e-out hole). Reset only on reaching a SURFACE or
    /// completing a job (productive); accumulate while stuck below grade
    /// regardless of motion. Closes Ben's "no colonist EVER stuck"
    /// guarantee unconditionally.
    /// bastion (COORDINATION-stigmergic-v1, FR13-REV): the SATURATION FIELD —
    /// the "pheromone." A coarse-celled decaying scalar over the job board:
    /// a colonist WORKING a cell deposits each arbitration cycle; the field
    /// decays each cycle. High = "worked/crowded here", low = under-served.
    /// The claim scoring reads it as a penalty (LOCAL key lookup only — no
    /// global min-search, so no tie-break hazard; FR13-REV Q2, B0-safe:
    /// per-cell decay is order-free, deposits iterate the deterministic
    /// entity-ordered join). Generalizes the ±2XY clump repel into a smooth
    /// cross-frontier gradient; near-flat field ≈ today's behavior (Q5 — no
    /// small-job threshold needed).
    pub saturation: HashMap<Vec2<i32>, f32>,
    /// bastion (FR13-REV Q4): per-colonist bark cooldown — `allowed_to_speak`
    /// is a capability check, not a rate limit, so the coordination bark
    /// carries its own cadence.
    pub last_bark: HashMap<Uid, f64>,
    pub stuck_watch: HashMap<Uid, f32>,
    /// bastion ((α) STUCKJOB fix, architect-ruled): last observed
    /// `(job, progress)` per colonist — the stuck-watch teleport suppression
    /// must be EARNED by verified job progress, never by claim-HOLDING alone.
    /// Same-job progress increase since the last watchdog pass = earned reset;
    /// a claim SWITCH only re-baselines (claim-churn cycling through
    /// unreachable jobs must never suppress — the reopened-F5 hole the
    /// STUCKJOB falsifier pins: 200s+ sealed-vault suppression vs the 60s
    /// designed backstop). Stale entries are harmless (overwritten on the
    /// next held-job pass; bounded by colonist count).
    pub stuck_job_progress: HashMap<Uid, (common::bastion::JobId, f32)>,
    /// bastion (FREE-CLIMB CAP, Ben-ruled + Opus-cleared): per-colonist z of
    /// the last GENUINE FOOTHOLD (terrain solid directly below feet / physics
    /// on_ground) — the anchor for the spatial climb cap. Explicitly NEVER
    /// reset on on_wall/climbing (Opus R3: the lift's `supported` includes
    /// wall contact and would re-anchor a sheer-wall climber every tick =
    /// indefinite stage-climbing). stuck_watch map pattern; stale entries
    /// harmless (overwritten at the next foothold).
    pub climb_anchor: HashMap<Uid, i32>,
    /// FROZEN CAP-SKILL (Ben's steer): the climbing level SNAPSHOTTED at the
    /// start of a below-grade episode, held for the WHOLE episode. Free
    /// wall-climbing earns XP live, but a mid-escape level-up cannot grow the
    /// cap on the CURRENT escape (the seed-corpus bounce-farm); the leveled
    /// skill benefits the NEXT climb. Written lazily (entry.or_insert at cap
    /// consult — vacant-only, so foothold bounces structurally cannot
    /// re-read a leveled skill); cleared ONLY at genuine-surface sites.
    pub climb_cap_skill: HashMap<Uid, u16>,
    /// bastion (A2 co-requisite, Opus-cleared): per-colonist rescue-progress
    /// baseline for the PROGRESS-EARNED rescue_pending gate (mirrors
    /// STUCKJOB-α): (his best egress distance at last watchdog pass, count of
    /// access jobs serving his route at last pass). Suppression of the
    /// teleport backstop must be EARNED by verified per-colonist progress —
    /// a stale egress_target + someone else's access job never suppresses.
    pub rescue_progress: HashMap<Uid, (f32, usize)>,
    /// bastion (FR15-TIGHTDIG, flag-gated): per-colonist PROGRESS WINDOW —
    /// (window anchor position, window start time, committed-path index at
    /// window start). The drive-owned displacement signal: progressing =
    /// net displacement from the anchor ≥ [`TIGHTDIG_MIN_PROGRESS`] per
    /// [`TIGHTDIG_WINDOW`], AND (committed-path active ⇒ the path index
    /// advanced). Replaces the beeline best-dist inputs at the watchdog
    /// reset sites when [`tightdig_enabled`]; board-side because
    /// `ActiveJob` is `Copy` (the stuck_watch pattern).
    pub progress_watch: HashMap<Uid, (Vec3<f32>, f64, usize)>,
    /// bastion (FR15-TIGHTDIG, flag-gated): per-colonist COMMITTED PATH —
    /// (waypoints, next index, the target it was computed for). The
    /// reinstated FR15 committed-path steer: computed ONCE via
    /// `bastion_full_path` (bounded, one-shot), steered waypoint-by-
    /// waypoint, invalidated when the job target moves; `None`/exhausted
    /// falls back to the plain beeline steer (today's behavior).
    pub path_cache: HashMap<Uid, (Vec<Vec3<i32>>, usize, Vec3<f32>)>,
    /// bastion (FR15-TIGHTDIG, flag-gated): last tick's steer target —
    /// a steer SWITCH (anchor reached → real target, fetch engaged, …)
    /// re-anchors the progress window instead of reading as stall/jump
    /// (re-expresses the old `sdist > best_dist + 4.0` rebase under the
    /// new metric, no dangling beeline reader).
    pub last_steer: HashMap<Uid, Vec3<f32>>,
    /// bastion (B7-0/B7-1, the thought queue): (who, where, what kind) of
    /// chronicle THOUGHT to record — drained by the rtsim tick next tick
    /// (this system holds a long-lived rtsim READ guard for the LOD gate,
    /// so it can't write the chronicle itself; the tick owns the data
    /// mutably by construction). A one-tick deferral on multi-game-day
    /// thoughts. Emitters: cave-in fear (B7-0), sleep quality (B7-1).
    pub pending_thoughts: Vec<(
        common::rtsim::RtSimEntity,
        Vec3<i32>,
        ::rtsim::data::ChronicleKind,
    )>,
    /// bastion (B7-1): the bed slots, keyed by block position — the
    /// reservations-table shape (capacity-1 occupancy). OWNERSHIP truth
    /// persists on the colonist record; this is the runtime table
    /// (rebuilt as beds are built/assigned; the board is session-state).
    pub beds: HashMap<Vec3<i32>, common::bastion::BedSlot>,
    /// bastion (B7-2): per-colonist preempt cooldown — the (c) livelock
    /// guard: at most one need-preempt ATTEMPT per window regardless of
    /// outcome (a failed attempt cannot re-fire inside it — the colonist
    /// does reachable work meanwhile, the honest ENDURE; a successful
    /// sleep does not need it — the meter sits above interrupt). The
    /// last_bark shape.
    pub preempt_cooldown: HashMap<Uid, f64>,
    /// bastion (B7-3): when each colonist's mood first dropped below the
    /// break threshold (cleared on recovery) — the sustained-window arm
    /// of the breakdown staircase.
    pub mood_below_since: HashMap<Uid, f64>,
    /// bastion (AUTON-0): cumulative drive switches (REPORTED telemetry —
    /// the thrash-bound gate reads the delta over a window).
    pub drive_switches: u64,
    /// bastion (FARM/PROD-2, row 46): registered farm plots — persistent
    /// footprints (the stockpiles shape); the farm pass reads cell state
    /// inside them and generates till/sow/harvest jobs forever.
    pub farms: Vec<(common::bastion::ZoneId, Region)>,
    /// bastion (FARM-PAINT, row 2026-08-08): the real ground z per
    /// (x,y) column of a registered farm plot, resolved ONCE at
    /// registration via [`column_surface_z`] against the painted
    /// `region.min.z` as a HINT, not a literal. `farms` (above) keeps
    /// its exact 2-tuple shape — `.chain(board.stockpiles.iter())` at
    /// two call sites requires matching tuple arity, so the resolved
    /// z lives in this SEPARATE, flat, plot-agnostic map instead of a
    /// third tuple field. Keyed by raw (x,y): farm plots don't overlap
    /// in XY by construction (paint would create duplicate jobs
    /// otherwise, the same invariant every other designation kind
    /// already relies on). A column absent here (paint over open
    /// water / an unloaded chunk -- `column_surface_z` returned `None`)
    /// is silently skipped by the trigger pass, same as today's
    /// `!ground.is_filled()` "no field under a hole" case -- never a
    /// job on unresolved ground. Resolved once, not re-scanned per
    /// tick: cheap, and avoids the surface moving under a growing crop
    /// or a nearby dig re-answering the question mid-season.
    pub farm_column_z: std::collections::BTreeMap<(i32, i32), i32>,
    /// bastion (FARM): per-sown-cell last stage-advance time (game
    /// seconds) — the deterministic growth clock. BTreeMap: structural
    /// ordering, never hash-iteration order (the PATH-0 discipline).
    /// Evicted at harvest and by the pass when the sprite vanishes.
    pub farm_growth: std::collections::BTreeMap<(i32, i32, i32), f64>,
    /// bastion (B7-2): preempt attempts fired (telemetry — the
    /// anti-thrash assert counts these against the cooldown-rate bound).
    pub preempt_attempts: u64,
    /// ITEM8-V4 (route 1, famine root cause): fired every time the
    /// `to_release` drain clears `claimed_by` for a job that was
    /// `unreachable == true` at that moment — the precondition witness for
    /// this fix's own registered prediction, same "silent field, no
    /// consumer" lesson `b5_split_off_one_fired` taught the hard way:
    /// zero here does not mean the fix works, it means the trigger never
    /// occurred (Fable's F5, VOID not PASS on zero). Emitted on the
    /// food-stock heartbeat below so a killed server's log still carries
    /// its final value.
    pub claim_expiry_releases: u32,
    /// ITEM8-V4 (route 3, sweep extension — "the closer, never the fix"):
    /// count of unclaimed `Designated` jobs the backstop sweep removed for
    /// sitting unclaimed past `access_stall_secs()`'s own bound. Same
    /// witness discipline as `claim_expiry_releases` — emitted on the
    /// heartbeat, not left to a log line nobody re-reads.
    pub designated_sweep_reaps: u32,
    /// ITEM8-V4 sentinel S1 (Fable-ruled, log-only, never terminates the
    /// server): consecutive food-stock heartbeat samples reading exactly
    /// 0 — the edge-trigger state for the "colony terminal" log line, so
    /// it fires once per qualifying window rather than once per sample.
    /// Conservative by construction: a run whose stock merely dips to 0
    /// for a single sample (noise) does not fire; only a SUSTAINED empty
    /// stockpile does, matching v3's own famine signature (0 from tick
    /// 99300 onward, never a transient blip).
    pub colony_terminal_zero_streak: u32,
    /// ITEM8-V4 F6 (Opus-ruled, the F5-tension resolution): when each
    /// currently-claimed job's PRESENT claim episode began (game
    /// seconds), observed periodically rather than hooked at every claim
    /// call site — this session was punished four times today for an
    /// incomplete enumeration, and every `active_jobs.insert` site is
    /// exactly that shape of list. `or_insert` only writes on first
    /// observation of a live claim, so a re-claim after a genuine release
    /// gets a fresh start time on its next scan, not the old one.
    pub claim_leak_watch: HashMap<JobId, f64>,
    /// ITEM8-V4 route 3 v2 (v4-live-finding fix): when each currently-
    /// UNCLAIMED job's present unclaimed episode began (game seconds),
    /// observed periodically -- same technique and same reason as
    /// `claim_leak_watch` above, inverted polarity. Replaces the first
    /// version's use of `cycles_since_last_claim` (position-keyed, so a
    /// freshly-created job at a historically-stale position was born
    /// already past the reap threshold -- v4's live 186-reap churn
    /// finding). `or_insert` only writes on first observation of THIS
    /// job being unclaimed, so a job that was claimed then released gets
    /// a fresh clock, not the position's history.
    pub unclaimed_watch: HashMap<JobId, f64>,
    /// ITEM8-V4 F6: fires when the generic backstop (below) force-
    /// releases a claim that outlived `GENERIC_CLAIM_LEAK_SECS` —
    /// INVERTED bar (Opus-ruled): zero is the expected PASS on a healthy
    /// run; any nonzero value is a RECORDED FINDING (a leak route
    /// `claim_release_should_clear`'s targeted fix does not cover), never
    /// silently absorbed into a passing score.
    pub generic_claim_leak_releases: u32,
    /// ROW-INDESTRUCTIBLE-MINE-CELL.md: emergency-access job completions
    /// suppress every world effect (drop, XP, cave-in) by design, so they
    /// are counted HERE, never under `"bastion: job completed"` -- that
    /// line is the colony's own health metric and must mean real
    /// production. A v4 run's `job completed` count rose to 361 while
    /// every world-effect stayed at zero because this distinction did not
    /// exist; this counter is the honest replacement, not a suppression.
    pub emergency_access_completions: u32,
    /// bastion (B6 HAUL): painted stockpile zones `(id, region)` — the haul
    /// destinations. Registered at placement, dropped on cancel (dependent
    /// haul jobs cancel with their zone).
    pub stockpiles: Vec<(common::bastion::ZoneId, Region)>,
    /// bastion (ZONE-0): painted ACTIVITY zones `(id, kind, region)` — the
    /// soft-magnet footprints, mirrored into [`ActivityZones`] each
    /// arbitration pass. Same lifecycle as stockpiles.
    pub activity_zones: Vec<(common::bastion::ZoneId, common::bastion::ZoneKind, Region)>,
    pub next_zone: common::bastion::ZoneId,
    /// bastion (B6 JOB-CORE; reformulated DECISIONS #89, Option B): the
    /// reservation table -- ONE job holds ONE `ReservationId`, this map's
    /// forward direction. Stock itself stays DERIVED from physical items
    /// (D2: never a second mutable count); this table only prevents two
    /// jobs spending the same UNIT of an item. Charter line (Fable): "a
    /// conformance test can be green while the thing it conserves is the
    /// wrong unit" -- the OLD law was "at most one reservation per item
    /// entity"; the reformulated law is "sum of reserved units per item
    /// entity <= the entity's own stack amount", and `amount == 1`
    /// (every non-stackable) yields the old law exactly, as the
    /// degenerate case -- see `has_capacity`.
    pub reservations: HashMap<common::bastion::ReservationId, Uid>,
    /// T1.13 (reformulated DECISIONS #89): the reverse index of
    /// `reservations` (item `Uid` -> every LIVE reservation id against
    /// it, never empty while the key exists -- an item with zero live
    /// reservations has NO entry, not an empty `Vec`), maintained in
    /// lockstep at every mutator so `is_reserved`/`reserved_count` stay
    /// O(1)/O(k) instead of a linear `reservations.values()` scan. Kept
    /// as a cache of `reservations`, never an independent source of
    /// truth (the forward map's bidirectional-uniqueness invariant --
    /// each `ReservationId` still appears in at most one item's `Vec`,
    /// since each key of `reservations` maps to exactly one item -- is
    /// still enforced by `reserve`'s capacity `debug_assert`).
    pub reservations_by_item: HashMap<Uid, Vec<common::bastion::ReservationId>>,
    pub next_reservation: common::bastion::ReservationId,

    /// bastion (R10): the authoritative per-link fencing-epoch store —
    /// `link_id → current epoch` (absent = 0). Advanced ONLY at release-
    /// class events (release/abort/reacquire/re-election/teardown/despawn
    /// — the enumerated advance-sites, exhaustiveness-asserted); a NEW
    /// task ADOPTS the current value. Writers present their adopted epoch
    /// through [`crate::bastion_traversal::fenced_movement_write`]; stale
    /// = logged no-op. Keyed by owner-derived link id until R9/M3's
    /// persistent links land (epoch semantics unchanged by that
    /// migration — per-link monotone counter).
    pub link_epochs: HashMap<u64, u64>,
    /// bastion (M3): persistent traversal links — `link_id → TraversalLink`
    /// (the fair queue + generation + capacity). Keyed by the owner-derived
    /// link id (same key as `link_epochs`). Membership stays in
    /// `emergency_route_members`; the link adds ORDER (`(enqueue_tick, uid)`
    /// tickets). Maintained in lockstep with membership: every insert site
    /// pairs a `traversal_enqueue`, and `leave_route` is THE removal path
    /// (source-pinned) — so the queue can never disagree with membership.
    /// An empty link is pruned (the monotone fencing epoch lives in
    /// `link_epochs`, never here).
    pub traversal_links: HashMap<u64, crate::bastion_traversal::TraversalLink>,
    /// bastion (M3): per-member queue-wait bookkeeping —
    /// `(last observed queue position, ticks waited without the queue
    /// moving)`. The counter RE-ARMS when position decreases (the queue
    /// moving IS progress) and is dropped by `leave_route`. Read by the
    /// queue-wait hold (the energy-wait shape) against a position-scaled
    /// budget; never read by sim decisions.
    pub traversal_queue_wait: HashMap<Uid, (usize, u32)>,
    /// bastion (GATHER deposit ruling): per-colonist set of item defs its
    /// forage collects put in its bag — recorded at emit from the SAME
    /// reclaim source the authoritative handler consumes (a loot-TABLE
    /// sprite could roll a different def than we recorded; such a leftover
    /// rides the bag until a future re-roll — never lost, never duped).
    /// Drained by the end-of-forage [`JobKind::DepositRun`]; keyed by Uid
    /// so a demote/promote round-trip keeps the debt.
    pub gathered_defs: HashMap<Uid, std::collections::BTreeSet<String>>,
    /// bastion (CASE-003 belt, persistence form): consecutive ticks each
    /// colonist's capsule CORE has sat inside solid terrain. At
    /// [`EMBED_PERSIST_TICKS`] the colonist is genuinely WEDGED (the
    /// revert-locked class — the seed-21 tree teleport) and is relocated;
    /// transient core-solid states (a top-down digger settling into its own
    /// fresh 1-deep pocket) clear in a few ticks and never trip it.
    pub embed_watch: HashMap<Uid, u32>,
    /// bastion (B-LIVE4, mine-oscillation): CUMULATIVE count of job-claim
    /// events over the board's life — every `claimed_by = Some` in
    /// arbitration bumps it (initial claims AND re-claims after a release).
    /// Divided by jobs-that-existed it is the CLAIMS-PER-JOB ratio: 1.0 =
    /// each job claimed once (no bob), >1 = re-target churn (the play-tester
    /// measured 1.46× before the auto-ladder-off + commitment work). Pure
    /// telemetry for the harness; never gates.
    pub total_claims: u64,
    /// bastion (DETRNG belt, architect): cumulative MINE_DROP cells spawned
    /// by cave-in collapses — the conservation companion: total stone in the
    /// world == blocks mined + this (`stone_sum == mined + collapsed`), an
    /// invariant that holds under ANY rng mode.
    pub cavein_drop_cells: u64,
    /// bastion (AUTON-1, row 49): QUEUED BUILD PLANS — intent only, frozen
    /// cell lists (the farm-plot registration shape: the paint records, a
    /// generator owns job creation). The build generator emits jobs for
    /// unfilled cells; a plan whose every cell is filled retires. Frozen at
    /// queue time on purpose — re-resolving surfaces after partial builds
    /// would shift the target under the plan.
    pub plans: Vec<(common::bastion::ZoneId, Vec<Vec3<i32>>)>,
    /// bastion (AUTON-1): the mine generator's z-slab cursor — one slab of
    /// the scan volume per firing; session-state (a reboot rescans, which
    /// is idempotent by the dedupe).
    pub selfgen_cursor: i32,
    /// bastion (AUTON-1): columns of RETIRED plans — permanently off-limits
    /// to the mine generator. A finished platform is Rock-class, exposed,
    /// and near home: without this, the next plan's demand would send the
    /// diggers straight through the last plan's building.
    pub built_xy: std::collections::HashSet<Vec2<i32>>,
    /// bastion (AUTON-1): generator telemetry (REPORTED — the scenario's
    /// quiescence assert reads the deltas; never gates live play).
    pub gen_mine_jobs: u64,
    pub gen_build_jobs: u64,
    pub plans_completed: u64,
    /// bastion (CHOP-FELLING, row 51.6): fell-sets keyed by their base-cut
    /// job (the B6-HAUL container-store / B7 BedSlot co-located side-table
    /// shape — `Job` stays lean, only chop base-cuts have entries). Evicted
    /// by `remove_job` AND swept by `cancel_region` (whose in-region purge
    /// bypasses `remove_job` via `jobs.retain`).
    pub chop_fell_sets: HashMap<JobId, ChopFell>,
    /// bastion (CHOP-FELLING): trees mid-fall — drained one z-band per tick
    /// by the felling pass (top-down; ~0.4s for a typical tree at 30tps).
    /// Independent of the job (already completed): cancel can't stop a
    /// falling tree, and colonist death mid-fall changes nothing (XP was
    /// granted at completion).
    pub felling: Vec<FellingTree>,
}

impl JobBoard {
    /// PRECONDITION accessor for the eat/stack pair (instrument debt). The
    /// monotonic reservation id doubles as "how many reservations were EVER
    /// made" -- zero means the stack path never ran, so b5_stack_reserved_units_max
    /// and b5_eat_completions_distinct being zero is VACUOUS, not a real zero.
    pub fn reservation_total(&self) -> common::bastion::ReservationId {
        self.next_reservation
    }

    /// Stage-1 single reservation authority. A live task's reserved member is
    /// authoritative. Before a task exists, the head of the link's FAIR
    /// queue (`(enqueue_tick, uid)` — M3, replacing the R9-named
    /// lowest-UID-alone anti-pattern) is only the queue head; it is not a
    /// second reservation record and cannot own traversal until task
    /// creation succeeds.
    pub fn traversal_queue_head(&self, owner: Uid) -> Option<Uid> {
        self.bastion_traversal_tasks
            .values()
            .find(|task| {
                task.owner == owner
                    && task.reservation_matches(task.reserved_member)
                    && task.phase.mode().is_some()
            })
            .map(|task| task.reserved_member)
            .or_else(|| {
                self.traversal_links
                    .get(&owner.0.get())
                    .and_then(|link| link.head())
            })
    }

    /// bastion (M3): enqueue a member on the owner's traversal link.
    /// Idempotent — an already-queued member keeps its ORIGINAL ticket;
    /// fair re-enqueue-at-the-back comes from `leave_route` running first
    /// on any cancel/reacquire. Every `emergency_route_members.insert`
    /// site pairs with a call to this (source-pinned).
    pub fn traversal_enqueue(&mut self, owner: Uid, member: Uid, tick: u64) {
        let link = self.traversal_links.entry(owner.0.get()).or_default();
        let head_before = link.head();
        link.enqueue(member, tick);
        if head_before != link.head() {
            // First election on an empty queue is a handover too (no
            // epoch advance: no prior holder means no stale tuple exists).
            link.reservation_generation = link.reservation_generation.wrapping_add(1);
        }
    }

    /// bastion (M3): THE route-membership removal path (the B17/
    /// `retire_traversal_task` discipline applied to the queue): one place
    /// removes membership AND the queue ticket. When the departing member
    /// was the queue HEAD, this is a queue RE-ELECTION — a release-class
    /// event: the generation bumps and the link's fencing epoch advances so
    /// the next head acquires under a fresh epoch. A same-block task
    /// retirement may advance the epoch again; double-advance is safe by
    /// the fence's equality algebra (any advance orphans every outstanding
    /// tuple; new tasks adopt the current value), and both events are real.
    pub fn leave_route(&mut self, member: Uid) -> Option<Uid> {
        let owner = self.emergency_route_members.remove(&member)?;
        self.traversal_queue_wait.remove(&member);
        let link_id = owner.0.get();
        if let Some(link) = self.traversal_links.get_mut(&link_id) {
            let was_head = link.head() == Some(member);
            if link.dequeue(member) && was_head {
                link.reservation_generation = link.reservation_generation.wrapping_add(1);
                self.advance_epoch(link_id);
            }
            if self
                .traversal_links
                .get(&link_id)
                .is_some_and(|link| link.is_empty())
            {
                self.traversal_links.remove(&link_id);
            }
        }
        Some(owner)
    }

    /// BACKSTOP-OPT (B) / ROW-INDESTRUCTIBLE-MINE-CELL.md defect 2: release
    /// a member whose emergency re-engage attempts have exceeded
    /// `EMERGENCY_REENGAGE_BOUND` to the independent failsafe tier --
    /// STICKY (barred from re-emission until delivered, enforced by every
    /// `emergency_reengage_exhausted.contains` gate in this file). Shared
    /// by both exhaustion call sites (Abort-phase traversal aborts and
    /// ExhaustedReplan invalid-exit replans) so the release behavior can
    /// never drift between them, and so it is testable without a full ECS
    /// tick -- pure over `JobBoard`'s own maps, no entity/position lookup
    /// needed for this decision.
    pub fn release_reengage_exhausted(&mut self, member: Uid, watch_reason: &'static str) {
        self.emergency_reengage_aborts.remove(&member);
        self.emergency_reengage_exhausted.insert(member);
        self.leave_route(member);
        self.emergency_frontier_reacquire.remove(&member);
        watch_wipe(&mut self.stuck_watch, &member, watch_reason);
    }

    /// Decision-order view of the otherwise lookup-optimized job map.
    ///
    /// `HashMap` iteration is deliberately unspecified. In deterministic
    /// harness mode, sorting the monotonic `JobId`s pins equal-score claim
    /// ties and same-depth access-plan ties without imposing a tree-map cost
    /// on the live game.
    /// T0.38 (master build order; T0-003): decision order is a stable
    /// total order in LIVE and harness alike — arbitration outcomes must
    /// not ride HashMap iteration in any mode (the claim-determinism row;
    /// sorting ~thousands of u64 ids per arbitration pass is noise).
    pub fn decision_job_ids(&self) -> Vec<JobId> {
        let mut ids = self.jobs.keys().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        ids
    }

    /// Generate jobs for a validated designation region. Returns created ids.
    /// v1 generation: Mine = every filled block in the region; Chop = every
    /// wood block; Build = every currently-empty position (the inverse of
    /// Mine — you're placing new blocks, not removing existing ones), gated
    /// on `BUILD_MATERIAL_ITEM` (B5's single-material stand-in; B6 gives Build
    /// real per-blueprint recipes); Stockpile = none yet (B6 zones).
    /// The claim mask's REGIONS alone — for the many readers that ask
    /// "is this point inside any designation?" and do not care which kind.
    /// One accessor so those call sites did not each grow their own
    /// `.map(|(r, _)| r)`.
    pub fn designated_regions(&self) -> impl Iterator<Item = Region> + '_ {
        self.designated.iter().map(|(region, _)| *region)
    }

    pub fn place_designation(
        &mut self,
        terrain: &TerrainGrid,
        region: Region,
        kind: DesignationKind,
    ) -> Vec<JobId> {
        let mut created = Vec::new();
        let work = kind.work_type();
        // B5.8: the designation volume joins the colony's claim mask
        // (whether or not blocks matched — the CLAIM is the painted box).
        self.designated.push((region, kind));
        // B6 HAUL: a Stockpile paint REGISTERS a zone (the haul
        // destination) — it generates no block jobs (job_wanted = false);
        // haul jobs are generated separately against loose items.
        if kind == DesignationKind::Stockpile {
            let id = self.next_zone;
            self.next_zone += 1;
            self.stockpiles.push((id, region));
            info!(zone = id, ?region, "bastion: stockpile zone registered");
        }
        // ZONE-0: an activity zone registers its footprint the same way —
        // no jobs, just the magnet's geometry.
        if let DesignationKind::Zone(zk) = kind {
            let id = self.next_zone;
            self.next_zone += 1;
            self.activity_zones.push((id, zk, region));
            info!(zone = id, kind = ?zk, ?region, "bastion: activity zone registered");
        }
        // FARM-PAINT (row 2026-08-08, supersedes the old "v1 farms are
        // FLAT plots, region.min.z is the field's ground level" note):
        // Farm is Area2D (`kind.footprint_mode()`) -- the client NEVER
        // sends a z_extent for it (voxygen's session/mod.rs branches on
        // exactly that mode to send `None`), so `region.min.z` was never
        // more than the height the player's PICK PLANE happened to be
        // at when they dragged -- a guess, not a measurement, and the
        // trigger pass took it literally: one block off in either
        // direction and every column's `is_filled()` check failed,
        // silently producing zero jobs forever (the live-observed
        // defect this row fixes). Resolve each column's REAL surface
        // now, at registration, using `region.min.z` as a HINT into
        // `column_surface_z`'s existing ±window search (the same
        // resolver relative-mode z_extent designations already use via
        // `resolve_column_surface` -- Farm has no floor_z to dispatch
        // flat-mode from, so this calls the relative-mode half
        // directly rather than constructing a dummy ZExtent to route
        // through the dispatcher for no benefit).
        if kind == DesignationKind::Farm {
            let id = self.next_zone;
            self.next_zone += 1;
            self.farms.push((id, region));
            let mut resolved = 0u32;
            let mut unresolved = 0u32;
            for y in region.min.y..=region.max.y {
                for x in region.min.x..=region.max.x {
                    match column_surface_z(terrain, x, y, region.min.z) {
                        Some(z) => {
                            self.farm_column_z.insert((x, y), z);
                            resolved += 1;
                        },
                        None => unresolved += 1,
                    }
                }
            }
            info!(
                zone = id,
                ?region,
                resolved,
                unresolved,
                "bastion: farm plot registered, per-column surface resolved"
            );
        }
        // One job per block, regardless of kind: repainting a region — or
        // overlapping designations (Mine and Chop both match a Wood block,
        // since wood `is_filled()`) — must not create duplicate jobs. Each
        // duplicate would complete independently and drop loot from the same
        // single block: a free-item exploit reachable from the in-game paint
        // path, not just a bookkeeping wart.
        let occupied: HashSet<Vec3<i32>> = self.jobs.values().map(|j| j.pos).collect();
        for z in region.min.z..=region.max.z {
            for y in region.min.y..=region.max.y {
                for x in region.min.x..=region.max.x {
                    let pos = Vec3::new(x, y, z);
                    if occupied.contains(&pos) {
                        continue;
                    }
                    let Ok(block) = terrain.get(pos) else {
                        continue;
                    };
                    if job_wanted(kind, block) {
                        let id = self.next_id;
                        self.next_id += 1;
                        self.jobs.insert(id, Job {
                            kind: common::bastion::JobKind::Designated(kind),
                            work,
                            pos,
                            skill_floor: 0,
                            claimed_by: None,
                            suspended_for: None,
                            unreachable: false,
                            progress: 0.0,
                            // DPA-0 wood correction (ruled, LADDER only):
                            // rungs are timber (CHOP_DROP_ITEM) — pit props
                            // are wood, and it ties mining to CHOP/forestry.
                            // Build/Bed keep stone; stairs stay carved/free.
                            required_item: match kind {
                                DesignationKind::Build | DesignationKind::Bed => {
                                    Some(BUILD_MATERIAL_ITEM)
                                },
                                DesignationKind::Ladder => Some(CHOP_DROP_ITEM),
                                _ => None,
                            },
                            needs_materials: false,
                            carve_attempted: false,
                            is_access: false,
                            stuck_strikes: 0,
                            benched_until_tick: None,
                            // Box-top-relative depth: the descent gate's
                            // "how far below the way out".
                            depth: (region.max.z - z).clamp(0, 255) as u8,
                            reservation: None,
                            affordance: designation_affordance(kind),
                        });
                        created.push(id);
                    }
                }
            }
        }
        info!(?kind, jobs = created.len(), "bastion: designation placed");
        created
    }

    /// bastion (B5.6b-2): the surface-relative placement path — an XY
    /// footprint + [`ZExtent`] resolved per column against the terrain
    /// surface (see [`column_surface_z`]), replacing the client's old
    /// hardcoded flat `min.z-2` expansion. On a slope every painted column
    /// now gets jobs at ITS OWN surface (the B5.MINE-COVERAGE fix); on flat
    /// ground with the default extent this generates exactly what
    /// [`Self::place_designation`] did before. Columns with no resolvable
    /// surface (open water, void) are skipped, same as out-of-bounds blocks
    /// in the region path.
    pub fn place_designation_surface(
        &mut self,
        terrain: &TerrainGrid,
        min_xy: Vec2<i32>,
        max_xy: Vec2<i32>,
        hint_z: i32,
        extent: ZExtent,
        kind: DesignationKind,
    ) -> Vec<JobId> {
        let mut created = Vec::new();
        let work = kind.work_type();
        // B5.8: the resolved volume bounds join the claim mask (same tight
        // AABB the echo carries — computed inline as columns resolve).
        let mut mask_z = None::<(i32, i32)>;
        // Same one-job-per-block dedupe as the region path (free-item
        // exploit guard — see `place_designation`).
        let occupied: HashSet<Vec3<i32>> = self.jobs.values().map(|j| j.pos).collect();
        for y in min_xy.y..=max_xy.y {
            for x in min_xy.x..=max_xy.x {
                let Some(surface) = resolve_column_surface(terrain, x, y, hint_z, &extent) else {
                    continue;
                };
                // B5.6b-2.1: ONE range authority (relative or flat-floor).
                let Some((lo, hi)) = extent.column_range(surface) else {
                    continue;
                };
                mask_z = Some(mask_z.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
                for z in lo..=hi {
                    let pos = Vec3::new(x, y, z);
                    if occupied.contains(&pos) {
                        continue;
                    }
                    let Ok(block) = terrain.get(pos) else {
                        continue;
                    };
                    if job_wanted(kind, block) {
                        let id = self.next_id;
                        self.next_id += 1;
                        let depth = (surface - z).clamp(0, 255) as u8;
                        self.jobs.insert(id, Job {
                            kind: common::bastion::JobKind::Designated(kind),
                            work,
                            pos,
                            skill_floor: 0,
                            claimed_by: None,
                            suspended_for: None,
                            unreachable: false,
                            progress: 0.0,
                            // DPA-0 wood correction (ruled, LADDER only):
                            // rungs are timber (CHOP_DROP_ITEM) — pit props
                            // are wood, and it ties mining to CHOP/forestry.
                            // Build/Bed keep stone; stairs stay carved/free.
                            required_item: match kind {
                                DesignationKind::Build | DesignationKind::Bed => {
                                    Some(BUILD_MATERIAL_ITEM)
                                },
                                DesignationKind::Ladder => Some(CHOP_DROP_ITEM),
                                _ => None,
                            },
                            needs_materials: false,
                            carve_attempted: false,
                            is_access: false,
                            stuck_strikes: 0,
                            benched_until_tick: None,
                            // Per-column surface-relative depth: the
                            // descent gate's "how far below the way out".
                            depth,
                            reservation: None,
                            affordance: designation_affordance(kind),
                        });
                        created.push(id);
                    }
                }
            }
        }
        if let Some((z_min, z_max)) = mask_z {
            self.designated.push((
                Region {
                    min: Vec3::new(min_xy.x, min_xy.y, z_min),
                    max: Vec3::new(max_xy.x, max_xy.y, z_max),
                },
                kind,
            ));
        }
        info!(
            ?kind,
            jobs = created.len(),
            "bastion: surface designation placed"
        );
        created
    }

    /// COORDINATION-stigmergic-v1 (harness): the saturation at a world
    /// position's coarse cell — scenarios assert the field forms over a
    /// worked site and steers the split.
    pub fn saturation_at(&self, pos: Vec3<i32>) -> f32 {
        self.saturation
            .get(&coord_cell(pos))
            .copied()
            .unwrap_or(0.0)
    }

    /// CHOP-FELLING (row 51.6, refining FR10): ONE base-cut job per
    /// RESOLVED fell-set — the whole tree's positions, computed by the
    /// HANDLER via the World oracle (`get_area_trees` → `tree_valid_at` →
    /// [`tree_fell_set`]). The job sits at the ground-rooted BASE (always
    /// walkable-to — kills FR10's unreachable-canopy floating residual);
    /// the set is FROZEN in the `chop_fell_sets` side-table under the
    /// job's id; completion fells the whole set top-down (the timber
    /// event). Threshold = `CHOP_WORK_PER_BLOCK × wood_count` (bigger
    /// trees take proportionally longer — Ben's hard requirement). The
    /// set's tight AABB joins the claim mask exactly like a painted
    /// designation (same client outline echo; cancel-through-the-echo
    /// reaches the one job).
    pub fn place_chop_fell(
        &mut self,
        terrain: &TerrainGrid,
        base: Vec3<i32>,
        cells: &[Vec3<i32>],
    ) -> Option<JobId> {
        let occupied: HashSet<Vec3<i32>> = self.jobs.values().map(|j| j.pos).collect();
        // Re-validate the handed-in set against the same predicate the old
        // per-cell path used; the kept cells are what falls (and what the
        // AABB spans). Wood tallied here — the frozen threshold source.
        let mut kept = Vec::new();
        let mut wood_count: u32 = 0;
        let (mut min, mut max) = (Vec3::broadcast(i32::MAX), Vec3::broadcast(i32::MIN));
        for &pos in cells {
            let Ok(block) = terrain.get(pos) else {
                continue;
            };
            if !job_wanted(DesignationKind::Chop, block) {
                continue;
            }
            if block.kind() == BlockKind::Wood {
                wood_count += 1;
            }
            min = Vec3::partial_min(min, pos);
            max = Vec3::partial_max(max, pos);
            kept.push(pos);
        }
        // The base must be part of the valid set and unoccupied (one tree,
        // one job — a re-paint over a pending base is a no-op, the FR10
        // dedupe contract).
        if kept.is_empty() || !kept.contains(&base) || occupied.contains(&base) {
            info!(
                ?base,
                "bastion: chop fell-set rejected (empty/baseless/occupied)"
            );
            return None;
        }
        // TASK #61 (chop visibility, PARKED, 2026-08-03): two proactive/
        // lazy reachability-probe designs were built, measured, and
        // reverted here across the same day -- both are dead ends, kept
        // as a pointer so the next person doesn't retry them blind:
        // (a) placement-time, probing every newly-discovered tree at
        // designation -- reverted because the corpus (0-1 trees/seed)
        // structurally can't measure the cost of probing the reachable
        // majority; a live multi-tree paint could spike tick time
        // invisibly to a green fan (Opus's catch).
        // (b) lazy, firing once at the churn-release site (below, near
        // `job.unreachable = true`) after a chop job's first stuck-
        // release, using the actual failing colonist's position -- built,
        // instrumented (measured 85.79ms per 100k-node probe, mark-once
        // bounded), and then checked against the ONLY known genuinely-
        // unreachable chop case (b5 seed 80): its diagnostic never fired.
        // The pre-existing `plan_access` self-rescue path (see the
        // churn-release site's own comment) already reports that
        // specific tree, EARLIER, because chop trees DO enter
        // `self.designated` (a #56(c)-era comment claiming otherwise was
        // stale). Evidence base collapsed to n=0 -- (b) was parked, not
        // landed, per the same standard that killed the cascade row: an
        // unexercised path with a measured cost and zero demonstrated
        // benefit is a net negative regardless of how well it's built.
        //
        // What DOES exist and stays: `BlockedRegionInfo::source` +
        // `JobBoard::blocked_sources` (attribution, returns every
        // producer covering a cell, not just the first -- this is the
        // instrument that PROVED (b) never fired, and it will answer the
        // same question again the moment a real second reporter exists)
        // and `remove_job`'s blocked_regions pruning (fixes a genuine
        // pre-existing #55 staleness gap for Mine AND Chop alike,
        // evidenced independently of whether any lazy probe ever runs).
        //
        // Open, unproven hypothesis for whoever picks this back up: a
        // FLAT/SIDEWAYS-blocked tree (water/lava/cliff, not elevation)
        // never routes through `plan_access` at all (that path only
        // fires when `job.pos.z - feet.z > reach`), so it might still
        // need a reactive report of its own -- no corpus example found
        // yet; this is a hypothesis to hunt with evidence, not a design
        // to build speculatively.
        // Top-down total order NOW (z DESC, then y, x): the felling pass
        // drains bands in this exact order — deterministic, base LAST.
        kept.sort_unstable_by(|a, b| b.z.cmp(&a.z).then(a.y.cmp(&b.y)).then(a.x.cmp(&b.x)));
        let id = self.next_id;
        self.next_id += 1;
        self.jobs.insert(id, Job {
            kind: common::bastion::JobKind::Designated(DesignationKind::Chop),
            work: DesignationKind::Chop.work_type(),
            pos: base,
            skill_floor: 0,
            claimed_by: None,
            suspended_for: None,
            unreachable: false,
            progress: 0.0,
            required_item: None,
            needs_materials: false,
            carve_attempted: false,
            is_access: false,
            stuck_strikes: 0,
            benched_until_tick: None,
            depth: 0,
            reservation: None,
            affordance: AffordanceClass::SolidTarget,
        });
        self.chop_fell_sets.insert(id, ChopFell {
            cells: kept,
            threshold: (CHOP_WORK_PER_BLOCK * wood_count as f32).max(1.0),
            wood_count,
        });
        self.designated.push((Region { min, max }, DesignationKind::Chop));
        info!(
            job = id,
            ?base,
            wood = wood_count,
            "bastion: chop base-cut placed (CHOP-FELLING)"
        );
        Some(id)
    }

    /// bastion (AUTON-1, row 49): queue a BUILD PLAN — intent only, NO jobs
    /// (the farm-paint precedent: registration here, job creation owned by
    /// the generator pass). Cells are the region's currently-empty positions
    /// (`job_wanted(Build)`), FROZEN at queue time. The region joins the
    /// claim mask — queueing IS the player's paint-equivalent intent, so
    /// access carving may serve the plan like any painted designation.
    /// Returns the plan's cell count (0 = nothing queued).
    pub fn queue_build_plan(&mut self, terrain: &TerrainGrid, region: Region) -> usize {
        let mut cells = Vec::new();
        // Bottom-up cell order — the generator emits in this order, so
        // floors fill before the courses above them.
        for z in region.min.z..=region.max.z {
            for y in region.min.y..=region.max.y {
                for x in region.min.x..=region.max.x {
                    let pos = Vec3::new(x, y, z);
                    let Ok(block) = terrain.get(pos) else {
                        continue;
                    };
                    if job_wanted(DesignationKind::Build, block) {
                        cells.push(pos);
                    }
                }
            }
        }
        if cells.is_empty() {
            return 0;
        }
        let n = cells.len();
        let id = self.next_zone;
        self.next_zone += 1;
        self.designated.push((region, DesignationKind::Build));
        info!(plan = id, cells = n, "bastion: build plan queued (AUTON-1)");
        self.plans.push((id, cells));
        n
    }

    /// Cancel all jobs inside a region. Returns the uids whose claims were
    /// released (their `ActiveJob` comps are cleared by the system within
    /// one cycle because the job id no longer exists).
    pub fn cancel_region(&mut self, region: Region) -> Vec<Uid> {
        // B6 HAUL: erase intersecting stockpile zones; haul jobs whose
        // DESTINATION zone died are cancelled with it (their pos is the
        // ITEM, not the zone — pos-based cancel below can't catch them)
        // and their reservations released.
        let before = self.stockpiles.len();
        self.stockpiles.retain(|(_, r)| !r.intersects(&region));
        // ZONE-0: activity zones erase with the same brush.
        self.activity_zones
            .retain(|(_, _, r)| !r.intersects(&region));
        // AUTON-1: a plan any of whose cells the eraser touches dies whole —
        // otherwise the generator re-emits the jobs the player just erased
        // (the eraser-vs-generator fight; whole-plan removal because a
        // half-erased blueprint is not a smaller blueprint, it's a mistake).
        self.plans
            .retain(|(_, cells)| !cells.iter().any(|c| region.contains_point(*c)));
        if self.stockpiles.len() != before {
            let live: HashSet<common::bastion::ZoneId> =
                self.stockpiles.iter().map(|(z, _)| *z).collect();
            let dead: Vec<JobId> = self
                .jobs
                .iter()
                .filter(|(_, j)| match j.kind {
                    common::bastion::JobKind::Haul { destination, .. }
                    | common::bastion::JobKind::DepositRun { destination } => {
                        !live.contains(&destination)
                    },
                    _ => false,
                })
                .map(|(id, _)| *id)
                .collect();
            for id in dead {
                self.remove_job(id);
            }
        }
        // B5.8: the claim mask shrinks with the cancellation (exact AABB
        // subtraction, ≤6 pieces per intersected region).
        self.designated = std::mem::take(&mut self.designated)
            .into_iter()
            .flat_map(|(r, k)| {
                if r.intersects(&region) {
                    // The remainder of a partially cancelled order is still
                    // an order OF THE SAME KIND -- the pieces inherit it.
                    r.subtract(&region)
                        .into_iter()
                        .map(|piece| (piece, k))
                        .collect::<Vec<_>>()
                } else {
                    vec![(r, k)]
                }
            })
            .collect();
        // TASK #55: a cancelled/re-designated region's blocked-state record
        // must not survive it -- the exact `Region` value this was keyed on
        // no longer exists in `designated` after the subtraction above, and
        // leaving the stale entry would report "blocked" on a designation
        // the player already erased.
        self.blocked_regions
            .retain(|b| !b.region.intersects(&region));
        let mut released = Vec::new();
        let mut dead_rids = Vec::new();
        self.jobs.retain(|_, job| {
            let inside = job.pos.x >= region.min.x
                && job.pos.x <= region.max.x
                && job.pos.y >= region.min.y
                && job.pos.y <= region.max.y
                && job.pos.z >= region.min.z
                && job.pos.z <= region.max.z;
            if inside && let Some(uid) = job.claimed_by {
                released.push(uid);
            }
            if inside && let Some(rid) = job.reservation {
                // B6: a cancelled job's reservation dies with it.
                dead_rids.push(rid);
            }
            !inside
        });
        for rid in dead_rids {
            if let Some(item) = self.reservations.remove(&rid) {
                self.remove_one_reservation(item, rid);
            }
        }
        // CHOP-FELLING: the in-region purge above bypasses `remove_job`
        // (jobs.retain), so orphaned fell-sets are swept here — a set
        // whose base-cut job is gone must never fell (RFC-2229 disjoint
        // field capture makes the self.jobs read legal).
        self.chop_fell_sets
            .retain(|id, _| self.jobs.contains_key(id));
        info!(released = released.len(), "bastion: designation cancelled");
        released
    }

    /// bastion (B6; reformulated DECISIONS #89, Option B): reserve ONE
    /// UNIT of an item entity for a job. `amount` is the item's OWN
    /// current total (`PickupItem::amount()`, never `PickupItem::
    /// item().amount()` -- that method's own doc says its amount
    /// "should *not* be used"). Whole-entity callers (the six haul/fetch
    /// sites, gated by their own unchanged `!is_reserved` check) pass
    /// `u32::MAX`: their external gate already guarantees zero prior
    /// reservations, so the capacity check below is trivially satisfied
    /// and `amount` never needs to be a real number for them. The
    /// caller stores the id on `Job.reservation`; release goes through
    /// [`Self::remove_job`] or [`Self::release_reservation`].
    pub fn reserve(&mut self, item: Uid, amount: u32) -> common::bastion::ReservationId {
        // T1.13 (conservation cluster, reformulated #89): CAPACITY BY
        // CONSTRUCTION -- sum of reserved units against an item entity
        // never exceeds its own `amount`. `amount == 1` (every non-
        // stackable) makes this the OLD "at most one reservation" law
        // exactly, as the degenerate case. Every caller already gates
        // on its own predicate (`!is_reserved` for whole-entity,
        // `has_capacity` for per-unit); asserting it here makes a
        // forgotten check surface as a LOUD double-spend in debug/
        // verify builds (where the floors run) instead of a silent
        // item dupe or a starved job. Release builds keep the fast path.
        debug_assert!(
            self.reserved_count(item) < amount,
            "T1.13: reservation over capacity for item {item:?} \
             (reserved {} >= amount {amount}) -- a caller skipped its \
             capacity gate",
            self.reserved_count(item)
        );
        let id = self.next_reservation;
        self.next_reservation += 1;
        self.reservations.insert(id, item);
        let live = self.reservations_by_item.entry(item).or_default();
        live.push(id);
        // #89 diagnostic: the peak, captured right after the push -- the
        // true high-water mark for this item, same discipline as
        // `b5_f3_idle_peak`.
        self.b5_stack_reserved_units_max = self.b5_stack_reserved_units_max.max(live.len() as u32);
        id
    }

    /// bastion (#89): how many LIVE reservations currently sit against
    /// this item entity (0 for an unreserved item -- the map has no
    /// entry, never an empty `Vec`, per the reverse index's own doc).
    pub fn reserved_count(&self, item: Uid) -> u32 {
        self.reservations_by_item
            .get(&item)
            .map_or(0, |ids| ids.len() as u32)
    }

    /// bastion (#89, consumption sites only -- the eat path): is there
    /// room for one more reservation against this item's CURRENT total
    /// `amount`? Unlike `is_reserved` (ANY reservation, used by the six
    /// whole-entity haul/fetch sites -- a hauler must not carry off a
    /// stack while colonists are still walking to eat from it), this is
    /// the per-unit predicate: `amount == 1` degenerates to `!is_reserved`.
    pub fn has_capacity(&self, item: Uid, amount: u32) -> bool {
        self.reserved_count(item) < amount
    }

    /// bastion (#89): remove exactly ONE reservation id from an item's
    /// live set, clearing the map entry only once it empties -- shared
    /// by all three removal mutators (`release_reservation`,
    /// `remove_job`, the region-cancel sweep) so the "clear only when
    /// empty" rule lives in one place, not three.
    pub fn remove_one_reservation(&mut self, item: Uid, id: common::bastion::ReservationId) {
        if let hashbrown::hash_map::Entry::Occupied(mut e) = self.reservations_by_item.entry(item)
        {
            e.get_mut().retain(|&rid| rid != id);
            if e.get().is_empty() {
                e.remove();
            }
        }
    }

    pub fn release_reservation(&mut self, id: common::bastion::ReservationId) {
        if let Some(item) = self.reservations.remove(&id) {
            self.remove_one_reservation(item, id);
        }
    }

    /// bastion (R10): the link's current fencing epoch (absent = 0).
    pub fn current_epoch(&self, link: u64) -> u64 {
        self.link_epochs.get(&link).copied().unwrap_or(0)
    }

    /// bastion (M3, read-only probe): the link's fair queue in order —
    /// `(member uid, enqueue_tick)` pairs — plus the link's reservation
    /// generation. `None` = no live link container.
    pub fn bastion_traversal_queue_probe(&self, link: u64) -> Option<(Vec<(u64, u64)>, u64)> {
        self.traversal_links
            .get(&link)
            .map(|l| (l.snapshot(), l.reservation_generation))
    }

    /// bastion (M3, read-only probe): the named member's route owner uid.
    pub fn bastion_route_owner_probe(&self, member: Uid) -> Option<u64> {
        self.emergency_route_members
            .get(&member)
            .map(|owner| owner.0.get())
    }

    /// bastion (R10 N-FENCE probe): the member's live task as
    /// `(link_id, epoch)`. Read-only.
    pub fn bastion_traversal_tasks_probe(&self, member: Uid) -> Option<(u64, u64)> {
        self.bastion_traversal_tasks
            .get(&member)
            .map(|t| (t.link_id, t.epoch))
    }

    /// bastion (R10): the link's CURRENT reserved member — the fence's
    /// `current_member` input (`None` when no live non-Abort task holds
    /// the link; `reservation_matches` semantics).
    pub fn bastion_traversal_current_member(&self, link: u64) -> Option<Uid> {
        self.bastion_traversal_tasks
            .values()
            .find(|t| {
                t.link_id == link
                    && t.phase != crate::bastion_traversal::BastionTraversalPhase::Abort
            })
            .map(|t| t.reserved_member)
    }

    /// bastion (R10): advance the link's epoch — call ONLY from a release-
    /// class event (the enumerated advance-sites; the exhaustiveness assert
    /// pins the set). Returns the NEW epoch. Monotone by construction;
    /// never called on acquire (adopt-on-acquire is the locked semantic —
    /// advancing there would fence the acquirer's own writes).
    pub fn advance_epoch(&mut self, link: u64) -> u64 {
        let e = self.link_epochs.entry(link).or_insert(0);
        *e += 1;
        tracing::debug!(link, epoch = *e, "bastion R10: link epoch advanced");
        *e
    }

    /// bastion (R10): THE traversal-task retirement path — every task
    /// removal goes through here (the `remove_job` B17 one-removal-path
    /// discipline, which just caught the F3 reservation leak; the
    /// source-scan test pins that no raw remove exists elsewhere).
    /// Retirement IS a release-class event: the link's epoch advances,
    /// orphaning every authority tuple adopted under it — a delayed writer
    /// still holding the dead task's tuple is fenced by construction.
    /// Per-site sibling-table cleanup stays at the call sites (their
    /// semantics differ by exit path and are not R10's concern).
    pub fn retire_traversal_task(
        &mut self,
        member: Uid,
        reason: &'static str,
    ) -> Option<crate::bastion_traversal::BastionTraversalTask> {
        let task = self.bastion_traversal_tasks.remove(&member);
        if let Some(t) = &task {
            let epoch = self.advance_epoch(t.link_id);
            tracing::debug!(
                member = member.0.get(),
                link = t.link_id,
                epoch,
                reason,
                "bastion R10: traversal task retired — epoch advanced"
            );
        }
        task
    }

    /// Is this item entity already reserved by any job? O(1) via the T1.13
    /// reverse index, kept in lockstep with `reservations` at every mutator.
    pub fn is_reserved(&self, item: Uid) -> bool { self.reservations_by_item.contains_key(&item) }

    /// T1.13 (conservation cluster, REFORMULATED DECISIONS #89): the
    /// reservation ledger's structural-consistency audit -- item `Uid`s
    /// whose reverse-index entry doesn't exactly match what the forward
    /// map says should be there (empty = conserved). NOT a "how many
    /// reservations" business-rule check anymore (multiple per item is
    /// legitimate under the capacity law); a genuine corruption signal
    /// under either model. Wired into the board audit at T1.16;
    /// deterministic (sorted output).
    pub fn reservation_conflicts(&self) -> Vec<Uid> {
        duplicate_reservations(&self.reservations, &self.reservations_by_item)
    }

    pub fn reserved_item(&self, id: common::bastion::ReservationId) -> Option<Uid> {
        self.reservations.get(&id).copied()
    }

    /// bastion (B6): remove a job AND release its reservation — THE removal
    /// path (B17: one place, so a cancelled/moot/completed job can never
    /// leak a reservation).
    /// bastion (49.2/B37, harness probes): board vitals — private fields
    /// exposed read-only for the pinning scenario's exact cycle counting.
    pub fn probe_next_id(&self) -> u64 { self.next_id }

    pub fn probe_reservations(&self) -> usize { self.reservations.len() }


    pub fn remove_job(&mut self, id: JobId) -> Option<Job> {
        let job = self.jobs.remove(&id);
        if let Some(j) = &job
            && let Some(rid) = j.reservation
            && let Some(item) = self.reservations.remove(&rid)
        {
            self.remove_one_reservation(item, rid);
        }
        // T1.19: a removed RestAt releases its creation-reserved bed (any
        // cancel/moot path), so a bed can never leak occupied by a colonist
        // who is no longer coming. Only clears if THIS job's claimant still
        // holds it (a re-assigned bed is another job's custody).
        if let Some(j) = &job
            && let common::bastion::JobKind::RestAt { bed_pos } = j.kind
            && let Some(slot) = self.beds.get_mut(&bed_pos)
            && slot.occupant == j.claimed_by
        {
            slot.occupant = None;
        }
        // CHOP-FELLING: a removed base-cut takes its stored fell-set with
        // it (moot/unreachable-drop/cancel — the tree stays standing).
        self.chop_fell_sets.remove(&id);
        // TASK #61 (staleness fix, 2026-08-03, Opus's catch + Opus's own
        // two corrections, same pass): #55's `blocked_regions` previously
        // cleared ONLY on explicit designation cancel (`cancel_region`,
        // above) -- a job that resolves any OTHER way (completes
        // normally, gets dropped as moot/churning) left its blocked-
        // report sitting forever, even once the thing it reported no
        // longer exists. Concretely: task #61's lazy chop probe can latch
        // a definitive-at-the-time "unreachable" verdict that a LATER
        // terrain change (a cave-in, a dig from elsewhere) makes false --
        // the job was never gated on that verdict, so it can still get
        // claimed and chopped normally, and the stale report would
        // otherwise never retract.
        //
        // Two guards on the fix itself:
        // - PERFORMANCE: `remove_job` is the hottest path in the system
        //   (every completed Mine block calls it) -- early-out on
        //   `blocked_regions.is_empty()` (true the overwhelming majority
        //   of the time) before touching anything else, so the added
        //   cost collapses to one branch in the common case.
        // - OVER-PRUNING: a `blocked_regions` entry covers a REGION,
        //   which may hold several jobs (Mine's plan_access-triggered
        //   entries span an arbitrary designated AABB). Pruning on ANY
        //   contained job's removal would silently retract the report
        //   for OTHER still-unreachable jobs sharing that region -- "one
        //   reachable tree completing hides the report for four that
        //   aren't". Only prune an entry once NO job remains anywhere
        //   inside it (this specific job has already been removed from
        //   `self.jobs` above, so the check naturally excludes it without
        //   special-casing).
        if let Some(j) = &job
            && !self.blocked_regions.is_empty()
        {
            self.blocked_regions.retain(|b| {
                !b.region.contains_point(j.pos)
                    || self.jobs.values().any(|other| b.region.contains_point(other.pos))
            });
        }
        // ROW B′ (2026-08-04): no cleanup needed here -- `benched_until_
        // tick` lives ON the Job struct (`common/src/bastion.rs`), not a
        // side map keyed by JobId, so it's already gone the moment
        // `self.jobs.remove(&id)` (top of this fn) drops the job.
        job
    }

    /// bastion (B7-3): insert a PRE-CLAIMED EatFrom job (the RestAt
    /// shape) carrying its food reservation AND the matched def as
    /// `required_item` — the B6 fetch contract (the fetch's `carrying`
    /// flip derives from required_item; a reservation alone would be
    /// fetched-then-released as a moot material job — the b73 run-1
    /// silent-release find).
    pub fn insert_eat_job(
        &mut self,
        item: Uid,
        pos: Vec3<i32>,
        uid: Uid,
        reservation: common::bastion::ReservationId,
        required: &'static str,
    ) -> JobId {
        let id = self.next_id;
        self.next_id += 1;
        self.jobs.insert(id, Job {
            kind: common::bastion::JobKind::EatFrom { item },
            work: common::bastion::WorkType::Haul,
            pos,
            skill_floor: 0,
            claimed_by: Some(uid),
            suspended_for: None,
            unreachable: false,
            progress: 0.0,
            required_item: Some(required),
            needs_materials: false,
            carve_attempted: false,
            is_access: false,
            stuck_strikes: 0,
            benched_until_tick: None,
            depth: 0,
            reservation: Some(reservation),
            affordance: AffordanceClass::Untargeted,
        });
        self.total_claims += 1;
        id
    }

    /// bastion (B7-3): insert a PRE-CLAIMED Despond job at the
    /// colonist's own feet — the breakdown state as a top-tier self-job
    /// (blocks all claims until it lifts).
    /// bastion (ITEM 11): insert a PRE-CLAIMED Recreate job — recreation's
    /// first producer, in the `insert_despond_job` shape below (self-job at
    /// the colonist's own feet, pre-claimed so it never enters claim
    /// selection, `WorkType::Haul` like every other self-job).
    ///
    /// The need it answers decays and, until this existed, was never
    /// restored by anything: a measured one-way ratchet feeding an
    /// unopposed mood penalty (ITEM11-RECREATION-READ.md).
    pub fn insert_recreate_job(&mut self, feet: Vec3<i32>, uid: Uid, until: f64) -> JobId {
        let id = self.next_id;
        self.next_id += 1;
        self.jobs.insert(id, Job {
            kind: common::bastion::JobKind::Recreate { until },
            work: common::bastion::WorkType::Haul,
            pos: feet,
            skill_floor: 0,
            claimed_by: Some(uid),
            suspended_for: None,
            unreachable: false,
            progress: 0.0,
            required_item: None,
            needs_materials: false,
            carve_attempted: false,
            is_access: false,
            stuck_strikes: 0,
            benched_until_tick: None,
            depth: 0,
            reservation: None,
            affordance: AffordanceClass::Untargeted,
        });
        self.total_claims += 1;
        id
    }

    pub fn insert_despond_job(&mut self, feet: Vec3<i32>, uid: Uid, until: f64) -> JobId {
        let id = self.next_id;
        self.next_id += 1;
        self.jobs.insert(id, Job {
            kind: common::bastion::JobKind::Despond { until },
            work: common::bastion::WorkType::Haul,
            pos: feet,
            skill_floor: 0,
            claimed_by: Some(uid),
            suspended_for: None,
            unreachable: false,
            progress: 0.0,
            required_item: None,
            needs_materials: false,
            carve_attempted: false,
            is_access: false,
            stuck_strikes: 0,
            benched_until_tick: None,
            depth: 0,
            reservation: None,
            affordance: AffordanceClass::Untargeted,
        });
        self.total_claims += 1;
        id
    }

    /// bastion (B7-1): insert a PRE-CLAIMED RestAt job (the DepositRun
    /// shape — rides the whole proven travel pipeline; the caller inserts
    /// the ActiveJob comp).
    pub fn insert_rest_job(&mut self, bed_pos: Vec3<i32>, uid: Uid) -> JobId {
        let id = self.next_id;
        self.next_id += 1;
        self.jobs.insert(id, Job {
            kind: common::bastion::JobKind::RestAt { bed_pos },
            work: common::bastion::WorkType::Haul,
            pos: bed_pos,
            skill_floor: 0,
            claimed_by: Some(uid),
            suspended_for: None,
            unreachable: false,
            progress: 0.0,
            required_item: None,
            needs_materials: false,
            carve_attempted: false,
            is_access: false,
            stuck_strikes: 0,
            benched_until_tick: None,
            depth: 0,
            reservation: None,
            affordance: AffordanceClass::Untargeted,
        });
        // T1.19 (conservation cluster): reserve the bed at CREATION, not at
        // arrival. The assigner filters occupied beds, so claiming it now
        // closes the create→arrive window where a second colonist could be
        // routed to the same bed. Released by remove_job on any cancel/moot,
        // by the arrival path on wake, and by the dead-occupant sweep.
        if let Some(slot) = self.beds.get_mut(&bed_pos) {
            slot.occupant = Some(uid);
        }
        self.total_claims += 1;
        id
    }

    /// bastion (task #55, 2026-07-30): is this cell inside a designation
    /// the auto-access planner gave up on? Returns the specific blocking
    /// cell (not `cell` itself, unless `cell` IS the blocking cell) so an
    /// inspector query on any job in the volume can answer "blocked by X".
    pub fn blocked_by(&self, cell: Vec3<i32>) -> Option<Vec3<i32>> {
        self.blocked_regions
            .iter()
            .find(|b| b.region.contains_point(cell))
            .map(|b| b.blocking_cell)
    }

    /// bastion (task #61, attribution, 2026-08-03): EVERY mechanism that
    /// has recorded a block covering `cell`, not just the first. Two
    /// producers pushing DIFFERENT `Region` values for the same tree (a
    /// whole-designation AABB vs a single point) would NOT collapse via
    /// `already_recorded`'s exact-Region dedupe -- both could coexist. A
    /// scalar first-match here would silently hide whichever mechanism
    /// pushed second (Opus's catch, 2026-08-03) -- this proved that a
    /// task #61 candidate lazy chop probe never independently fired on
    /// the corpus's only genuinely-unreachable chop case (b5 seed 80,
    /// covered earlier by `plan_access` alone), and that probe was
    /// parked as a result. Kept as general infrastructure for whenever a
    /// real second producer exists.
    pub fn blocked_sources(&self, cell: Vec3<i32>) -> Vec<&'static str> {
        self.blocked_regions
            .iter()
            .filter(|b| b.region.contains_point(cell))
            .map(|b| b.source)
            .collect()
    }

    /// bastion (B6): is this cell inside a stockpile footprint? XY + a
    /// tolerant z-band (items REST ON the painted surface; the paint's
    /// z-band needn't contain the resting z exactly).
    pub fn stockpile_at(&self, cell: Vec3<i32>) -> Option<common::bastion::ZoneId> {
        self.stockpiles
            .iter()
            .find(|(_, r)| {
                r.contains_point_xy(cell) && cell.z >= r.min.z - 2 && cell.z <= r.max.z + 3
            })
            .map(|(id, _)| *id)
    }

    pub fn zone_region(&self, id: common::bastion::ZoneId) -> Option<Region> {
        self.stockpiles
            .iter()
            .find(|(z, _)| *z == id)
            .map(|(_, r)| *r)
    }

    /// bastion (B6-hotfix): drop access anchors whose base falls inside a
    /// region — used when the Erase tool deletes the ladders in that region
    /// so staged routing stops steering colonists at a now-ghost vertical
    /// link. (Player + auto-built anchors alike; a re-painted ladder
    /// re-registers its anchor on build.)
    pub fn drop_access_anchors_in(&mut self, region: Region) {
        let before = self.access_anchors.len();
        self.access_anchors.retain(|a| !region.contains_point(*a));
        let dropped = before - self.access_anchors.len();
        if dropped > 0 {
            info!(dropped, "bastion: access anchors dropped (ladder erased)");
        }
    }

    /// Audit for the harness gate: claim counts + distinctness.
    pub fn audit(&self) -> JobAudit {
        let mut seen: HashSet<Uid> = HashSet::new();
        let mut distinct = true;
        let mut claimed = 0;
        let mut unreachable = 0;
        for job in self.jobs.values() {
            if job.unreachable {
                unreachable += 1;
            }
            if let Some(uid) = job.claimed_by {
                claimed += 1;
                if !seen.insert(uid) {
                    distinct = false;
                }
            }
        }
        JobAudit {
            total: self.jobs.len(),
            claimed,
            unreachable,
            claims_distinct: distinct,
            // T1.16: surface the reservation-ledger bidirectional-uniqueness
            // audit (T1.13) in the one board-conservation verdict.
            reservation_conflicts: self.reservation_conflicts().len(),
        }
    }

    /// F5 falsifier introspection (READ-ONLY): does `uid` hold a live egress
    /// target, how many live access jobs does he own, and how many live
    /// access jobs exist colony-wide. The redesigned stuckjob leg asserts
    /// its own preconditions on these (the rev-1 leg asserted none and its
    /// decoys silently never existed).
    /// Staging support (INV-HARNESS-CLIMB-LEVEL): a level stage must also
    /// clear any live episode snapshot — the lazy or_insert can capture the
    /// PRE-staging spawn roll during setup ticks, and the stale entry then
    /// runs the whole episode at the unstaged cap (the frozen-verify tape:
    /// level=0, cap_blocks=6).
    pub fn staging_clear_climb_snapshot(&mut self, uid: &Uid) {
        self.climb_cap_skill.remove(uid);
    }

    /// M2 fixture introspection (READ-ONLY): the traversal task's phase (as
    /// its Debug name), whether this uid is the reserved member, and the
    /// abort reason if any. The fixture asserts phase progressions and
    /// single-owner invariants on this without touching the machinery.
    pub fn traversal_probe(&self, uid: &Uid) -> Option<(String, bool, Option<&'static str>)> {
        self.bastion_traversal_tasks.get(uid).map(|task| {
            (
                format!("{:?}", task.phase),
                task.reserved_member == *uid,
                task.abort_reason,
            )
        })
    }

    /// bastion (M2 fixture N1B, harness read): the member's route descriptor
    /// dismount anchor — a fingerprint cell ahead of the ascent, so the
    /// intent-faithful blocked-entry variant can aim a survivable mutation.
    pub fn route_dismount(&self, member: &Uid) -> Option<Vec3<i32>> {
        let task = self.bastion_traversal_tasks.get(member)?;
        self.emergency_route_descriptors
            .get(&task.owner)
            .map(|descriptor| descriptor.dismount)
    }

    pub fn egress_probe(&self, uid: Uid) -> (bool, usize, usize) {
        let has_target = self.egress_targets.contains_key(&uid);
        let owned = self
            .emergency_access_jobs
            .iter()
            .filter(|(id, owner)| **owner == uid && self.jobs.contains_key(*id))
            .count();
        let total = self.jobs.values().filter(|job| job.is_access).count();
        (has_target, owned, total)
    }
}

/// T1.13 (conservation cluster, REFORMULATED DECISIONS #89): item `Uid`s
/// involved in a structural inconsistency between the two tables (empty =
/// conserved).
///
/// Under the OLD at-most-one law this doubled as "reserved by more than
/// one job", a business-rule conflict. Under the CAPACITY law (sum of
/// reserved units <= the item's own stack amount) that is no longer a
/// conflict on its own — a stackable food's amount decides how many
/// reservations are legal, and this row's whole point is that MORE than
/// one is normal. So this now checks STRUCTURAL consistency instead: a
/// genuine corruption signal (a mutator forgot to update one side, or an
/// id leaked under the wrong item) under EITHER model, never a false
/// positive on a legitimately multiply-reserved stack.
///
/// Checks BOTH directions per id, not merely per-item set equality: an id
/// filed under the wrong item's reverse entry can leave that item's OWN
/// set "locally" matching its OWN forward count while the id's RIGHTFUL
/// item is silently missing it — a per-item-only check misses exactly
/// this class, so both the wrongly-filed-under item and the rightful
/// item are flagged. A pure function of both tables: no state, no RNG,
/// no wall-clock; output sorted+deduped so the verdict is deterministic.
pub fn duplicate_reservations(
    reservations: &HashMap<common::bastion::ReservationId, Uid>,
    reservations_by_item: &HashMap<Uid, Vec<common::bastion::ReservationId>>,
) -> Vec<Uid> {
    let mut broken: HashSet<Uid> = HashSet::new();
    // Every id filed under an item's reverse entry must have that SAME
    // item as its forward-map owner.
    for (item, ids) in reservations_by_item {
        for id in ids {
            match reservations.get(id) {
                Some(true_item) if true_item == item => {},
                Some(true_item) => {
                    broken.insert(*item);
                    broken.insert(*true_item);
                },
                None => {
                    broken.insert(*item); // a stale id, no forward entry at all
                },
            }
        }
    }
    // Every id's forward-map owner must have that id present in its own
    // reverse entry (catches a missing/dropped id the reverse-only walk
    // above cannot see, since it only ever iterates ids that ARE there).
    for (id, item) in reservations {
        let present = reservations_by_item
            .get(item)
            .is_some_and(|ids| ids.contains(id));
        if !present {
            broken.insert(*item);
        }
    }
    let mut broken: Vec<Uid> = broken.into_iter().collect();
    broken.sort_unstable_by_key(|u| u.0.get());
    broken
}

/// bastion (CHOP-FELLING, row 51.6): base-cut labor per WOOD cell of the
/// fell-set — the size-scaled completion threshold is
/// `CHOP_WORK_PER_BLOCK × wood_count` (progress-units; 1.0 = exactly the
/// old per-block model's cost per Wood block, so per-wood labor is
/// conserved through the granularity refactor; Leaves gate no labor by
/// the design's explicit pin). Code-const like the TOOL-0 factors; RON
/// promotion is a tuning-pass concern.
pub const CHOP_WORK_PER_BLOCK: f32 = 1.0;

/// bastion (COORDINATION-stigmergic-v1, FR13-REV): the field's tuning. CELL =
/// coarse-grid size in blocks. A worked cell gains DEPOSIT per worker per
/// cycle and the whole field decays by DECAY per cycle → a single steady
/// worker equilibrates at DEPOSIT/(1−DECAY) = 20, which × WEIGHT = ~15 score
/// units of repel (score is in distance-blocks; cf. the ±32 top-down band and
/// the +12 clump repel) — enough to out-pull a modest distance difference,
/// never enough to crush the top-down ordering. The bark fires only on a REAL
/// flow (the colonist's own cell is ≥ BARK_MIN_DIFF more saturated than the
/// claimed one) and at most once per BARK_COOLDOWN per colonist.
pub const COORD_CELL: i32 = 4;

/// How far above the paint hint the per-column surface scan starts, and how
/// far below it gives up. Painting happens near the surface; ±this window
/// covers any slope a single drag can span (a 64-wide footprint on a 45°
/// hillside is +/−32).
const SURFACE_SCAN_UP: i32 = 48;

const SURFACE_SCAN_DOWN: i32 = 96;

/// Real, standable terrain — the canonical surface-kind filter shared with
/// the harness scan and the client's `overlay_surface_z`
/// (BASTION_ARCHITECTURE §5). Deliberately excludes Wood/Leaves so tree
/// canopies never read as "the surface" (the recurring `is_filled()` gotcha
/// that caused B5.MINE-COVERAGE's cousin bugs).
pub fn is_surface_terrain(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Rock
            | BlockKind::WeakRock
            | BlockKind::GlowingRock
            | BlockKind::GlowingWeakRock
            | BlockKind::Grass
            | BlockKind::Snow
            | BlockKind::ArtSnow
            | BlockKind::Earth
            | BlockKind::Sand
            | BlockKind::Ice
    )
}

/// The TRUE crest z of a flat-floor column: the topmost real-terrain block at
/// or above `floor_z`, scanned up to `floor_z + FLAT_SURFACE_SCAN_MAX`. Unlike
/// [`column_surface_z`] (a ±window around the paint plane that caps a tall
/// hill), this reaches the column's real top so a flat-floor pit cuts the
/// whole hill down to the shared floor. Returns `None` when the floor cell
/// itself has no real terrain at/above it in range (the column's surface is
/// already at/below the floor — nothing to dig; [`ZExtent::column_range`]
/// agrees, returning `None` for `surface < floor`).
pub fn column_flat_surface_z(terrain: &TerrainGrid, x: i32, y: i32, floor_z: i32) -> Option<i32> {
    (floor_z..=floor_z + FLAT_SURFACE_SCAN_MAX).rev().find(|z| {
        terrain
            .get(Vec3::new(x, y, *z))
            .is_ok_and(|b| is_surface_terrain(b.kind()))
    })
}

/// REQ-0077: the planner and executor share one typed traversal contract.
/// Construction completion alone is not an executable route: the descriptor
/// records which authoritative locomotion seam owns the vertical transition.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum EmergencyTraversalKind {
    CarvedStair,
    ConstructedLadder,
    NaturalShaft,
}

/// How far ABOVE the flat floor a flat-floor column scans for its TRUE crest.
/// `column_surface_z`'s ±window is centred on the PAINT PLANE, so a flat-floor
/// pit painted from the BASE of a tall hill caps each hill column at
/// `hint + SURFACE_SCAN_UP` and leaves a stub above it (Ben live-bug #4 —
/// "the flatten doesn't flatten"). Flat mode instead scans up from the shared
/// floor to the column's real top. This bounds that scan so it can't run away
/// on pathological terrain; the paint's `MAX_DESIGNATION_VOLUME` gate (which
/// now measures against these true crests) rejects a dig taller than any
/// natural relief anyway.
pub const FLAT_SURFACE_SCAN_MAX: i32 = 128;

/// REQ-0081: the bounded walk portion of a constructed-ladder traversal
/// link.  The shipping A* produces the ordered feet cells; Bastion owns only
/// validation and the cursor until the existing ladder transaction takes
/// over.  This is intentionally separate from the general job-travel cache so
/// no ordinary writer can replace the route-owned corridor mid-handoff.
#[derive(Clone, Debug)]
pub struct EmergencyApproachCorridor {
    pub owner: Uid,
    pub frontier: JobId,
    pub entry: Vec3<i32>,
    pub waypoints: Vec<Vec3<i32>>,
    pub next_idx: usize,
    pub started_tick: u64,
    /// Commit-time member position: the stable anchor for the FIRST leg's
    /// runtime re-validation sweep. Runtime sweeps validate PLANNED SEGMENTS
    /// (origin→wp0, wp[i-1]→wp[i]), never live-position→waypoint — a moving
    /// member's off-center transit clipped the route's OWN rung cell and
    /// self-destructed the corridor (M2 layer 2; registry class 10 kin).
    pub origin: Vec3<f32>,
    /// B58 (s21 class): `(position, tick)` at the last stepper progress
    /// observation — the no-progress replan trigger's memory. The planned-
    /// segment sweeps deliberately never validate live-position→waypoint,
    /// so a member DISPLACED after commit (netted, shoved) can hold a clear
    /// corridor while terrain blocks the unvalidated member→wp0 gap: the
    /// drive pushed into the wall for the full failsafe window (seed-21
    /// tape). Trigger-based (not per-tick sweep — the B57 site-4 livelock
    /// class) replan-from-position closes it.
    pub last_check: Option<(Vec3<f32>, u64)>,
}
