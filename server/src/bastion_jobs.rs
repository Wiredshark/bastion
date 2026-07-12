//! bastion (B4/B5): the job board — designations become jobs, colonists claim
//! jobs by priority/skill/distance, walk to them, and (B5) actually work them:
//! terrain edit + item drop + skill XP.
//!
//! Design (see design doc §B4/§B5 + `docs/BASTION_B4_FINDINGS.md` /
//! `BASTION_B5_FINDINGS.md`):
//! - [`JobBoard`] is a server ECS resource; jobs are block-level tasks
//!   generated from painted designations (`common::bastion::Region`).
//! - [`Sys`] runs every tick for travel upkeep + work ticks, and every
//!   [`ARBITRATION_INTERVAL`] ticks for claim arbitration.
//! - Colonist movement reuses the loaded-agent rtsim intent path: the system
//!   writes `NpcActivity::Goto` into `Agent::rtsim_controller`, which the
//!   vanilla behavior tree executes with real traversal. While a colonist has
//!   an [`comp::bastion::ActiveJob`], the rtsim brain's activity is *not*
//!   synced over it (gate in `rtsim::tick`).
//! - Work execution reuses the same authoritative terrain-edit path vanilla
//!   mining uses (`BlockChange`, not raw chonk writes) and the same
//!   `CreateItemDropEvent` item-drop path — see `MineBlockEvent`'s handler in
//!   `server/src/events/interaction.rs` for the pattern this mirrors.

use crate::Tick;
use common::{
    bastion::{
        BUILD_MATERIAL_ITEM, CHOP_DROP_ITEM, DesignationKind, Job, JobAudit, JobId,
        MINE_DROP_ITEM, Region, ZExtent,
    },
    comp,
    comp::{
        Item,
        bastion::{ActiveJob, ActiveJobState},
        item::PickupItem,
    },
    event::CreateItemDropEvent,
    resources::{DeltaTime, ProgramTime},
    terrain::{Block, BlockKind, SpriteKind, TerrainGrid},
    uid::{IdMaps, Uid},
    vol::ReadVol,
};
use common_ecs::{Job as EcsJob, Origin, Phase, System};
use common_state::BlockChange;
use hashbrown::{HashMap, HashSet};
use specs::{
    Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, Write, WriteExpect, WriteStorage,
};
use tracing::info;
use vek::*;

/// B5: seconds of work at skill level 0 to complete a job; higher skill
/// speeds this up (see `work_rate`). Ben's B5.8 live verdict: 3.0 read as
/// INSTANT — raised to a deliberate, satisfying pace (novice 6s, skill-10
/// 2s). The designer's TOOLS-UPGRADE system later makes dig speed
/// tool-gated on top of this base.
// TIMESCALE-DESIGN landmine (flagged, migration deferred): this is REAL
// seconds — under the per-game-time migration it derives from a game-time
// spec × day_cycle_coefficient (ServerConstants is an ECS resource, the
// plumbing is cheap when the design clears feasibility). At the B-LIVE2
// 10-minute overseer day, 6 real-seconds ≈ 14.4 game-minutes per block.
const WORK_DURATION_BASE: f32 = 6.0;
/// bastion (B6-hotfix, Ben live-test): master switch for the AUTO-BUILT
/// ladder-pillar access fallback (`plan_access`). `false` = the colony
/// carves STAIRS where geometry allows and builds no auto vertical link
/// elsewhere (the universal teleport-to-ground fail-safe covers the rest,
/// so no colonist is ever stuck); this removes the single-column
/// queue-fight Ben saw. Flip to `true` to restore the pillar fallback —
/// a one-line revert (all ladder code stays live for the player paint
/// tool). Re-enable once SOFT-1 ORCA makes the 1-wide queue orderly.
const AUTO_LADDER_ACCESS: bool = false;
/// bastion (B6 SOFT-0): how long a granted soft-collision pass lasts (the
/// watchdog grace window and the density relief both use it). Long enough
/// to physically squeeze past a blocker at walk speed; short enough that
/// spacing normalizes right after (expiry IS the hysteresis).
const SOFT_GRACE_SECS: f64 = 3.0;
/// bastion (B6 SOFT-0, trigger b): this many OTHER colonists within the
/// density radius = a chokepoint pile-up → soft relief before deadlock.
const SOFT_DENSITY_N: usize = 2;
const SOFT_DENSITY_R: f32 = 2.0;
/// Work-rate skill bonus: +20% speed per skill level.
const WORK_SKILL_BONUS: f32 = 0.2;
/// Flat completion XP grant (design doc: "grant skill XP on completion").
const COMPLETION_XP: f32 = 8.0;

pub(crate) fn work_rate(skill_level: u16) -> f32 {
    (1.0 + skill_level as f32 * WORK_SKILL_BONUS) / WORK_DURATION_BASE
}

/// bastion (B5.8): is `p` inside the colony's ACCESS mask — XY within a
/// claim box ±1, at or above the claim's floor (−1), UNBOUNDED upward? A
/// colony may always rise from its own claim to the open surface ("air
/// rights" — access is part of the dig plan, §3v), but never tunnels
/// sideways or downward beyond what was painted. This is what makes a
/// 1×1-painted shaft "tight" (stairs can't route) while a wide claim has
/// room — the geometry choice Ben asked for falls out of the claim shape.
fn in_access_mask(designated: &[Region], p: Vec3<i32>) -> bool {
    designated.iter().any(|r| {
        p.x >= r.min.x - 1
            && p.x <= r.max.x + 1
            && p.y >= r.min.y - 1
            && p.y <= r.max.y + 1
            && p.z >= r.min.z - 1
    })
}

/// bastion (B5.8): the auto-access LADDER — rung cells for a climbable
/// pillar topping out one block above the target level (the dismount needs
/// it). Chosen when stairs can't route (tight shaft / hollowed-out pit).
///
/// COLUMN CHOICE (run-15 finding): the pillar must stand AGAINST the pit
/// wall — a mid-pit pillar strands the climber over a horizontal gap at
/// the crest (lift stops off the ladder line; gravity wins the crossing).
/// Search open in-mask columns near the digger, preferring wall-adjacent
/// ones (a solid neighbor at the target level = a face-adjacent rim cell
/// to dismount onto), nearest first. The digger reaches the base by flat
/// walking (staged routing anchors there).
fn ladder_pillar(
    terrain: &TerrainGrid,
    designated: &[Region],
    from: Vec3<i32>,
    to_z: i32,
) -> Option<Vec<Vec3<i32>>> {
    let top = to_z + 1;
    if top <= from.z {
        return None;
    }
    let filled = |p: Vec3<i32>| terrain.get(p).map(|b| b.is_filled()).unwrap_or(false);
    let mut best: Option<(i32, Vec<Vec3<i32>>)> = None;
    for dx in -5..=5i32 {
        for dy in -5..=5i32 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let col = from.xy() + Vec2::new(dx, dy);
            // The column's own standing cell (floor may sit ±1 of the
            // digger's — lure holes, part-dug floors).
            let Some(stand_z) = (from.z - 1..=from.z + 2).find(|&z| {
                !filled(Vec3::new(col.x, col.y, z))
                    && filled(Vec3::new(col.x, col.y, z - 1))
            }) else {
                continue;
            };
            let cells: Vec<Vec3<i32>> = ((stand_z + 1)..=top)
                .map(|z| Vec3::new(col.x, col.y, z))
                .collect();
            if cells.is_empty()
                || !cells.iter().all(|p| {
                    in_access_mask(designated, *p) && !filled(*p)
                })
            {
                continue;
            }
            let wall_adjacent = [
                Vec2::new(1, 0),
                Vec2::new(-1, 0),
                Vec2::new(0, 1),
                Vec2::new(0, -1),
            ]
            .into_iter()
            .any(|d| filled(Vec3::new(col.x + d.x, col.y + d.y, to_z)));
            let score = dx.abs() + dy.abs() + if wall_adjacent { 0 } else { 100 };
            if best.as_ref().is_none_or(|(s, _)| score < *s) {
                best = Some((score, cells));
            }
        }
    }
    best.map(|(_, cells)| cells)
}

/// bastion (B5.8/B5.8-E): plan AND emit one access route from `from` up to
/// `to` — stairs (shared masked-switchback `carve_ramp`) where `mask` has
/// room, else a ladder pillar — inserting the step/rung jobs on the board
/// (material-free, `is_access`, anchor registered). ONE code path, two
/// permission sources: the colony claim mask for normal self-rescue, the
/// humanitarian bubble for emergency egress (which must work with ZERO
/// active zones — Ben's delete-the-zone entombment). Returns the plan kind
/// + job count, or `None` when no route fits the mask.
fn plan_access(
    board: &mut JobBoard,
    terrain: &TerrainGrid,
    mask: &[Region],
    from: Vec3<i32>,
    to: Vec3<i32>,
) -> Option<(DesignationKind, usize)> {
    let plan: Option<(Vec<Vec3<i32>>, DesignationKind)> = {
        let is_solid = |p: Vec3<i32>| terrain.get(p).map(|b| b.is_filled()).unwrap_or(false);
        let allowed = |p: Vec3<i32>| in_access_mask(mask, p);
        // Stair bases: the digger's own cell plus its walkable neighbors —
        // the first step of a pit-escape stair must cut into a WALL column
        // (floor rule), only adjacent from the pit's edge cells.
        let stairs = [
            Vec2::new(0, 0),
            Vec2::new(1, 0),
            Vec2::new(-1, 0),
            Vec2::new(0, 1),
            Vec2::new(0, -1),
        ]
        .into_iter()
        .filter_map(|d| {
            let f = Vec3::new(from.x + d.x, from.y + d.y, from.z);
            (d == Vec2::zero() || (!is_solid(f) && is_solid(f - Vec3::unit_z()))).then_some(f)
        })
        .find_map(|f| {
            common::bastion::carve_ramp(f, to, &is_solid, &allowed)
                .filter(|digs| !digs.is_empty())
        });
        match stairs {
            Some(digs) => Some((digs, DesignationKind::Mine)),
            // B6-hotfix (Ben live-test): AUTO-ladder access DISABLED — the
            // single-column auto-pillar caused a queue-fight ("they all
            // fight to use it") that did more harm than good. The colony
            // now plans STAIRS where they route and NO auto vertical link
            // where they don't; the universal teleport-to-ground fail-safe
            // (B6, entombment impossible by construction) backstops any
            // colonist a stair can't reach. REVERSIBLE by construction —
            // flip the flag to restore the pillar fallback (Ben may want
            // it back once SOFT-1 ORCA lands). ladder_pillar(),
            // DesignationKind::Ladder, and all climb-assist/magnetism code
            // STAY — the player Ladder paint tool + vertical-link
            // pathfinding still use them; only the AUTO fallback goes dark.
            None if AUTO_LADDER_ACCESS => ladder_pillar(terrain, mask, from, to.z)
                .map(|cells| (cells, DesignationKind::Ladder)),
            None => None,
        }
    };
    let (cells, kind) = plan?;
    // Register the vertical link's base for staged routing (cells are
    // emitted bottom-up; the first IS the base).
    if kind == DesignationKind::Ladder
        && let Some(base) = cells.first().copied()
        && !board
            .access_anchors
            .iter()
            .any(|a| a.xy().distance_squared(base.xy()) < 4)
    {
        info!(?base, "bastion: access anchor registered (plan)");
        board.access_anchors.push(base);
    }
    let occupied: HashSet<Vec3<i32>> = board.jobs.values().map(|j| j.pos).collect();
    let mut steps = 0;
    for pos in cells {
        if occupied.contains(&pos) {
            continue;
        }
        let id = board.next_id;
        board.next_id += 1;
        board.jobs.insert(id, Job {
            kind: common::bastion::JobKind::Designated(kind),
            work: kind.work_type(),
            pos,
            skill_floor: 0,
            claimed_by: None,
            unreachable: false,
            progress: 0.0,
            // Auto-access is material-free (infrastructure from spoil);
            // PLAYER-placed ladders still cost material.
            required_item: None,
            needs_materials: false,
            carve_attempted: true,
            // The plan marker: no cascades, and no NEW plan while these
            // are pending (overlapping plans dig each other's floors out).
            is_access: true,
            stuck_strikes: 0,
            depth: 0,
            reservation: None,
            });
        steps += 1;
    }
    Some((kind, steps))
}

/// The per-block job predicate, shared by both placement paths. Mine =
/// every filled block; Chop = wood only; Build = currently-empty positions
/// (placing blocks, not removing); Stockpile = none yet (B6 zones).
fn job_wanted(kind: DesignationKind, block: &Block) -> bool {
    match kind {
        DesignationKind::Mine => block.is_filled(),
        // CHOP redesign (FR10): a fell-set covers the WHOLE tree — trunk
        // (Wood) AND canopy (Leaves; cleared, no drop — the drop branch keys
        // on the block kind). Fixes the registry's "Chop-ignores-Leaves".
        DesignationKind::Chop => {
            matches!(block.kind(), BlockKind::Wood | BlockKind::Leaves)
        },
        // B5.8: a ladder rung, like Build, goes into currently-open space.
        DesignationKind::Build | DesignationKind::Ladder => !block.is_filled(),
        DesignationKind::Stockpile | DesignationKind::Zone(_) => false,
        // GATHER (row 38): forage — one job per collectible PLANT sprite
        // (the TerrainResource food allowlist; Stones/Wood/Gem/Ore stay
        // with the Mine/Chop economies). `is_directly_collectible` =
        // vanilla's own "yields without a required item" predicate, so
        // every job the scan creates is one the authoritative Collect
        // handler will actually honor.
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
pub const COORD_DEPOSIT: f32 = 1.0;
pub const COORD_DECAY: f32 = 0.95;
pub const COORD_SAT_WEIGHT: f32 = 0.75;
pub const COORD_BARK_COOLDOWN_SECS: f64 = 30.0;
pub const COORD_BARK_MIN_DIFF: f32 = 5.0;

/// The saturation field's coarse cell for a world position (euclidean division
/// so negative coordinates bucket correctly).
pub fn coord_cell(pos: Vec3<i32>) -> Vec2<i32> {
    Vec2::new(pos.x.div_euclid(COORD_CELL), pos.y.div_euclid(COORD_CELL))
}

/// bastion (FR15-TIGHTDIG, row 31): the VARIANT toggle — the
/// displacement+arc-length progress metric + the reinstated
/// committed-path steer run ONLY when `BASTION_TIGHTDIG=1` is in the
/// environment (read once). Off = today's beeline stuck-economy,
/// bit-for-bit. The paired-A/B harness (`--b58-paired`) runs one leg
/// each way on the same seed and reports the DELTA — the FR17-approved
/// interim measurement (tick-determinism is the real fix, a separate B8
/// block); the default stays OFF until the Opus gate rules on the
/// evidence.
pub fn tightdig_enabled() -> bool {
    static TIGHTDIG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TIGHTDIG.get_or_init(|| {
        std::env::var("BASTION_TIGHTDIG").is_ok_and(|v| v == "1")
    })
}

/// bastion (FR15-TIGHTDIG): the progress WINDOW length (seconds) — the
/// displacement verdict is judged once per window; STUCK_TIMEOUT (10s)
/// therefore allows ~5 consecutive no-progress verdicts before the
/// stall economy engages, comparable patience to the beeline metric.
pub const TIGHTDIG_WINDOW: f64 = 2.0;
/// Net displacement from the window anchor that counts as PROGRESS —
/// ≥ 1.0 subsumes the old 1-block reset hysteresis (sub-block wobble
/// never resets), and 1.5 tolerates routing AWAY from the beeline
/// (the false-fire this block exists to fix).
pub const TIGHTDIG_MIN_PROGRESS: f32 = 1.5;
/// A steer-target move larger than this is a SWITCH (anchor → target,
/// fetch engage/release) — re-anchor the window, don't read it as
/// stall or jump.
pub const TIGHTDIG_STEER_SWITCH: f32 = 2.0;

/// bastion (FR15-TIGHTDIG): the INPUT-SWAP measure — a synthetic
/// "distance" that drives the EXISTING watchdog branch structure
/// (improve→reset / jump→rebase / else→accrue) from the drive-owned
/// displacement window instead of beeline-to-steer, so the whole tuned
/// stall economy downstream (hysteresis, grace, queue-release, carve)
/// is reused VERBATIM — the FR15 lesson applied: change the measure's
/// INPUT, never the economy's structure. Absolute values are
/// meaningless; only the deltas steer the branches:
/// `best − 1 − ε` = a progressing window verdict (forces the reset arm),
/// `best + 5`     = a steer switch (forces the >+4 rebase arm, no accrual),
/// `best`         = no verdict yet / not progressing (forces accrual).
/// Progressing = net displacement from the window anchor ≥
/// [`TIGHTDIG_MIN_PROGRESS`] per [`TIGHTDIG_WINDOW`], AND (committed
/// path ACTIVE this tick ⇒ its waypoint index advanced within the
/// window) — displacement is measured on the COLONIST, so all three
/// steer sources (anchor / fetch / beeline-or-path) read correctly by
/// construction.
#[expect(clippy::too_many_arguments, reason = "a pure measure over drive state")]
fn tightdig_measure(
    progress_watch: &mut HashMap<Uid, (Vec3<f32>, f64, usize)>,
    last_steer: &mut HashMap<Uid, Vec3<f32>>,
    path_idx: Option<usize>,
    committed_active: bool,
    u: Uid,
    pos: Vec3<f32>,
    steer: Vec3<f32>,
    now: f64,
    best_dist: f32,
) -> f32 {
    // Fresh assignments carry best_dist = f32::MAX, where ±small deltas
    // SATURATE away (MAX − 1.5 == MAX) and no branch could ever fire —
    // clamp to a finite working base; only deltas matter.
    let base = best_dist.min(1.0e6);
    let switched = last_steer
        .get(&u)
        .is_some_and(|ls| ls.distance(steer) > TIGHTDIG_STEER_SWITCH);
    last_steer.insert(u, steer);
    let idx_now = path_idx.unwrap_or(0);
    if switched {
        progress_watch.insert(u, (pos, now, idx_now));
        return base + 5.0;
    }
    match progress_watch.get(&u).copied() {
        None => {
            progress_watch.insert(u, (pos, now, idx_now));
            base
        },
        Some((anchor, start, idx0)) => {
            if now - start < TIGHTDIG_WINDOW {
                return base;
            }
            let displaced = pos.distance(anchor);
            let s_ok = !committed_active || idx_now > idx0;
            progress_watch.insert(u, (pos, now, idx_now));
            if displaced >= TIGHTDIG_MIN_PROGRESS && s_ok {
                base - 1.0 - STUCK_EPSILON
            } else {
                // The existing accrual arm downstream counts
                // no_progress_ticks per tick under BOTH metrics — no
                // second counter here.
                base
            }
        },
    }
}

/// Arbitration cadence in server ticks (~0.5s at 30 tps): "a few Hz, not
/// every tick".
pub const ARBITRATION_INTERVAL: u64 = 15;
/// A colonist counts as arrived within this 3D distance of the job's
/// stand-at target (`block + (0.5, 0.5, 1.0)`).
const ARRIVE_DIST: f32 = 2.5;
/// Travel watchdog: release + mark unreachable after this long without
/// progress (seconds). `pub` so scenario harnesses can size their sampling
/// windows against it (see `bastion-harness`'s B4 scenario).
pub const STUCK_TIMEOUT: f32 = 10.0;
/// bastion (CASE-003 belt, persistence form): consecutive core-in-solid
/// ticks before the EMBED WATCH relocates a colonist (~1s at 30 tps —
/// instant on a human timescale, an eternity past any legitimate mining
/// transient).
pub const EMBED_PERSIST_TICKS: u32 = 30;
/// bastion (B6 HAUL): pending-haul cap per loaded colonist (throttle — the
/// generator never floods the board; more spawn as deliveries complete).
pub const HAUL_JOBS_PER_COLONIST: usize = 2;
/// Progress threshold per watchdog sample (blocks).
const STUCK_EPSILON: f32 = 0.5;
/// Walk speed factor for job travel.
const TRAVEL_SPEED: f32 = 0.8;

/// Chunks the harness (and future scenario tooling) forces to stay loaded —
/// the server unload sweep skips them. Empty in normal play.
#[derive(Default)]
pub struct BastionForceLoaded(pub HashSet<Vec2<i32>>);

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

/// The TRUE crest z of a flat-floor column: the topmost real-terrain block at
/// or above `floor_z`, scanned up to `floor_z + FLAT_SURFACE_SCAN_MAX`. Unlike
/// [`column_surface_z`] (a ±window around the paint plane that caps a tall
/// hill), this reaches the column's real top so a flat-floor pit cuts the
/// whole hill down to the shared floor. Returns `None` when the floor cell
/// itself has no real terrain at/above it in range (the column's surface is
/// already at/below the floor — nothing to dig; [`ZExtent::column_range`]
/// agrees, returning `None` for `surface < floor`).
pub fn column_flat_surface_z(terrain: &TerrainGrid, x: i32, y: i32, floor_z: i32) -> Option<i32> {
    (floor_z..=floor_z + FLAT_SURFACE_SCAN_MAX)
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
/// SAME surface (the echo-bounds invariant + an honest volume cap depend on it).
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

/// bastion (B15 / reviewer FR12): does this Mine target have a TERRAIN-ONLY
/// standable work-stance? Returns the feet-cell OFFSET from `pos` of the best
/// stance to COMMIT, or `None` when none exists (no way to stand and reach it)
/// OR the only stance is standing on an ISOLATED 1-wide block (clean-SKIP — a
/// precarious perch physics slides a colonist off before it can Arrive;
/// deferred to a future cave-in/collapse). PURE + meant to be computed ONCE
/// per arbitration cycle into a set (exactly like `is_exposed`): NO
/// per-colonist path-reachability here (that is expensive and order-sensitive
/// — the arrive/watchdog/teleport pipeline owns per-colonist reach downstream;
/// this gate only answers "does a stance EXIST"). Fixes B15: the exposure gate
/// admitted work with no standable stance (a hillside `+1`-arrival-gap cell
/// whose on-top space is a 1-wide slot, or a floating block) → claimed →
/// never Arrived → churn.
///
/// PREFERS ON-TOP (dig in place — least travel, the normal deep-dig stance)
/// and falls back to an ADJACENT-ground stance only when on-top is unusable
/// (a wedged `+1`-slot or an isolated perch) — that adjacent stance is exactly
/// what a `+1`-gap hillside block has to its open (downhill) side.
fn has_standable_stance(terrain: &TerrainGrid, pos: Vec3<i32>) -> Option<Vec3<i32>> {
    let solid = |p: Vec3<i32>| terrain.get(p).map(|b| b.is_filled()).unwrap_or(false);
    let open = |p: Vec3<i32>| terrain.get(p).map(|b| !b.is_filled()).unwrap_or(false);
    let cardinals = [
        Vec2::new(1, 0),
        Vec2::new(-1, 0),
        Vec2::new(0, 1),
        Vec2::new(0, -1),
    ];
    // 1. ON-TOP (preferred — the in-place deep-dig stance): stand ON the block,
    //    mine underfoot (the colonist then falls; the climb/teleport floor
    //    covers the landing). Valid iff:
    //    - body room above: pos+z1 (feet) AND pos+z2 (head) both open;
    //    - NOT an ISOLATED 1-wide perch (all 4 lateral sides solid-free at the
    //      block's own level) — nothing can path ONTO an isolated floater, so
    //      that is the clean-SKIP case (falls through to None below unless an
    //      adjacent stance exists);
    //    - the on-top space is NOT a 1-wide SLOT walled by higher columns (the
    //      `+1` arrival gap the capsule WEDGES in — ≥3 of the 4 lateral sides
    //      solid at the stance level pos+z1). A wedged block routes to its open
    //      side instead (step 2).
    let on_top_clear = open(pos + Vec3::unit_z()) && open(pos + Vec3::unit_z() * 2);
    let isolated = cardinals
        .into_iter()
        .all(|d| !solid(Vec3::new(pos.x + d.x, pos.y + d.y, pos.z)));
    let slot_walls = cardinals
        .into_iter()
        .filter(|d| solid(Vec3::new(pos.x + d.x, pos.y + d.y, pos.z + 1)))
        .count();
    if on_top_clear && !isolated && slot_walls < 3 {
        return Some(Vec3::unit_z());
    }
    // 2. ADJACENT-GROUND fallback: a cardinal neighbor cell at the block's own
    //    level with a solid floor below + open feet + open head — stand there
    //    and mine sideways. This is the reachable stance a wedged `+1`-gap
    //    block has downhill, and the way a reachable floating LEDGE is worked;
    //    an ISOLATED floater has none (its neighbors' floors are air) → None →
    //    clean-SKIP (no claim→unreachable churn; deferred to cave-in).
    for d in cardinals {
        let feet = Vec3::new(pos.x + d.x, pos.y + d.y, pos.z);
        if open(feet) && open(feet + Vec3::unit_z()) && solid(feet - Vec3::unit_z()) {
            return Some(Vec3::new(d.x, d.y, 0));
        }
    }
    None
}

/// The 6 axis-aligned neighbour offsets (shared by the support flood-fill).
const NEIGHBOURS6: [Vec3<i32>; 6] = [
    Vec3::new(1, 0, 0),
    Vec3::new(-1, 0, 0),
    Vec3::new(0, 1, 0),
    Vec3::new(0, -1, 0),
    Vec3::new(0, 0, 1),
    Vec3::new(0, 0, -1),
];

/// bastion (CAVE-IN v1 / FR11 Q2): the BOUNDED support check. With the block at
/// `removed_pos` about to be mined away (treated as air here), flood each solid
/// component that touched it, capped at `cap` cells. A component connected to
/// the big ground/bedrock mass blows past the cap → SUPPORTED (assumed, so a
/// large anchored mass is never spuriously collapsed — the known-limit large-
/// overhang case defers to the future global check, and Q1's eject backstops
/// any un-caught collapse). A component fully enumerated WITHIN the cap is a
/// small remnant no longer connected to ground → a FLOATING CHUNK that should
/// collapse. Returns the union of all floating cells (usually one small chunk),
/// or `None` if nothing floats.
///
/// Each severed neighbour is flooded SEPARATELY (removing `removed_pos` may cut
/// one mass into a grounded part and a floating part — a single merged flood
/// would wrongly read the whole thing as grounded). PURE (terrain via
/// `is_solid`) so it unit-tests without a `TerrainGrid` and stays deterministic.
/// `pub` so the harness's `bastion_force_collapse_check` can drive the same
/// support check deterministically (no colonist-mining timing).
pub fn floating_chunk(
    is_solid: impl Fn(Vec3<i32>) -> bool,
    removed_pos: Vec3<i32>,
    cap: usize,
) -> Option<Vec<Vec3<i32>>> {
    // Model the post-removal terrain: removed_pos reads as air.
    let solid = |p: Vec3<i32>| p != removed_pos && is_solid(p);
    let mut visited: HashSet<Vec3<i32>> = HashSet::new();
    let mut floating: Vec<Vec3<i32>> = Vec::new();
    for d in NEIGHBOURS6 {
        let start = removed_pos + d;
        if !solid(start) || visited.contains(&start) {
            continue;
        }
        // Flood this neighbour's solid component, bounded by the cap.
        let mut comp: HashSet<Vec3<i32>> = HashSet::new();
        comp.insert(start);
        let mut stack = vec![start];
        let mut grounded = false;
        while let Some(b) = stack.pop() {
            for dd in NEIGHBOURS6 {
                let n = b + dd;
                if solid(n) && comp.insert(n) {
                    if comp.len() > cap {
                        grounded = true; // big mass = connected to ground
                        break;
                    }
                    stack.push(n);
                }
            }
            if grounded {
                break;
            }
        }
        // Mark the component visited so a sibling neighbour in the SAME mass
        // doesn't re-flood it.
        visited.extend(comp.iter().copied());
        if !grounded {
            floating.extend(comp);
        }
    }
    (!floating.is_empty()).then_some(floating)
}

/// bastion (B5.6b-2): resolve a painted XY footprint + [`ZExtent`] to the
/// exact axis-aligned bounds of the per-column surface-relative volume.
/// This is what the server ECHOES to clients as the designation rect — the
/// echoed rect MUST bound every job the placement generates, or 3D
/// `cancel_region` erase misses jobs and orphans them (the echo-bounds
/// invariant, B5.6b findings §b-2). Returns `None` when no column resolves.
pub fn resolve_surface_bounds(
    terrain: &TerrainGrid,
    min_xy: Vec2<i32>,
    max_xy: Vec2<i32>,
    hint_z: i32,
    extent: ZExtent,
) -> Option<Region> {
    let mut z_min = i32::MAX;
    let mut z_max = i32::MIN;
    for y in min_xy.y..=max_xy.y {
        for x in min_xy.x..=max_xy.x {
            if let Some(s) = resolve_column_surface(terrain, x, y, hint_z, &extent)
                && let Some((lo, hi)) = extent.column_range(s)
            {
                z_min = z_min.min(lo);
                z_max = z_max.max(hi);
            }
        }
    }
    (z_min <= z_max).then(|| Region {
        min: Vec3::new(min_xy.x, min_xy.y, z_min),
        max: Vec3::new(max_xy.x, max_xy.y, z_max),
    })
}

/// bastion (B5.8-E): the trapped-detector ANNULUS scan — r 3..=6 around
/// `feet`. A shaft/small pit has ONLY walls out there; open ground has
/// level cells. Level-or-lower surfaces COUNT as egress (walk off / hop
/// down) — the first detector counted only upward steps, so any idle
/// colonist beside a town wall read as "trapped" and fired spurious carves
/// (part-e run-1). Wide pits (>7 across) evade this local test — the
/// loop-breaker covers their job-holding cases; jobless wide-pit detection
/// is a noted known-limit pending a real reachability probe. Returns
/// (has_egress, nearest rim target for an access plan).
fn egress_scan(
    terrain: &TerrainGrid,
    feet: Vec3<i32>,
    reach: i32,
) -> (bool, Option<Vec3<i32>>) {
    egress_scan_with(
        |x, y| column_surface_z(terrain, x, y, feet.z),
        feet,
        reach,
    )
}

/// The pure core of [`egress_scan`], generic over the surface probe so the
/// ±1 rise boundary is UNIT-TESTABLE without a `TerrainGrid` (reviewer F1:
/// the off-by-one this fixed hid for weeks precisely because nothing pinned
/// the boundary).
fn egress_scan_with(
    surface_of: impl Fn(i32, i32) -> Option<i32>,
    feet: Vec3<i32>,
    reach: i32,
) -> (bool, Option<Vec3<i32>>) {
    const EGRESS_RING_R: i32 = 5;
    let mut rim: Option<(i32, Vec3<i32>)> = None; // (xy dist, target)
    for dx in -(EGRESS_RING_R + 1)..=(EGRESS_RING_R + 1) {
        for dy in -(EGRESS_RING_R + 1)..=(EGRESS_RING_R + 1) {
            let d = dx.abs().max(dy.abs());
            if d < 3 {
                continue;
            }
            let Some(s) = surface_of(feet.x + dx, feet.y + dy) else {
                continue;
            };
            // Egress = a surface the colonist can STAND on: rise to stand
            // is (s+1) − feet, climbable iff ≤ reach → s ≤ feet+reach−1.
            // The original `s ≤ feet+reach` admitted rise reach+1 — one
            // too generous, and EXACTLY the b5 quarry shape (3-rise pit,
            // reach-2 novice): the detector read the unreachable rim as
            // egress and never fired, so the pit-floor digger churned
            // claims on far jobs forever (the "chop flake"'s true root).
            if s >= feet.z - 4 && s <= feet.z + reach - 1 {
                return (true, None);
            }
            if s > feet.z + reach - 1 {
                let dd = dx.abs() + dy.abs();
                if rim.as_ref().is_none_or(|(bd, _)| dd < *bd) {
                    rim = Some((dd, Vec3::new(feet.x + dx, feet.y + dy, s)));
                }
            }
        }
    }
    (false, rim.map(|(_, t)| t))
}

/// bastion (B-LIVE3): the ULTIMATE-fail-safe teleport destination — the
/// nearest real surface within a small spiral of `feet` (own column first).
/// `None` only if no column in range resolves a STANDABLE surface at all.
///
/// CASE-003 (the chokepoint-wedge root, seed-21 repro): `column_surface_z`
/// deliberately sees THROUGH non-surface-terrain solids (Wood/Leaves — so
/// designations resolve the ground UNDER a tree), which means a tree standing
/// on the returned surface can occupy the destination cell. The original
/// picker accepted it and teleported the colonist INTO the trunk — where the
/// phys terrain resolver's exhaustion path (revert to tick-start pos + zero
/// velocity) LOCKED it in place, and the stuck-watch re-fired 60s later to
/// the SAME deterministic cell (a permanent teleport→wedge loop). The accept
/// condition now requires the destination feet AND head cells to be air; an
/// occupied column is skipped and the spiral finds the next clear one. An
/// errored read (unloaded chunk) REJECTS the column — never teleport into
/// unknown space.
fn surface_teleport_dest(terrain: &TerrainGrid, feet: Vec3<i32>) -> Option<Vec3<i32>> {
    surface_teleport_dest_impl(
        |x, y| column_surface_z(terrain, x, y, feet.z + 64),
        |p| terrain.get(p).is_ok_and(|b| !b.is_filled()),
        feet,
    )
}

/// The testable core of [`surface_teleport_dest`] (`floating_chunk`'s
/// closure pattern): `surface_z(x, y)` resolves a column's surface,
/// `open(cell)` is "this cell is loaded air". ONE implementation — the
/// `TerrainGrid` wrapper above is the shipping path.
fn surface_teleport_dest_impl(
    surface_z: impl Fn(i32, i32) -> Option<i32>,
    open: impl Fn(Vec3<i32>) -> bool,
    feet: Vec3<i32>,
) -> Option<Vec3<i32>> {
    for r in 0..=8i32 {
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let (x, y) = (feet.x + dx, feet.y + dy);
                if let Some(s) = surface_z(x, y)
                    // The dest MUST be ABOVE the colonist — a teleport to
                    // the OWN column (r=0) of a pit returns the pit floor
                    // (below grade), teleporting the colonist to itself
                    // (chokepoint sealed-pit fs: tp fired but fs_out stayed
                    // false). Requiring `s ≥ feet.z` finds the surrounding
                    // pad's rim instead — always an upward exit.
                    && s + 1 > feet.z
                    // CASE-003: TRUE-STANDABLE dest — feet + head cells must
                    // be air (a tree trunk / any non-surface solid standing
                    // on the scanned surface occupies them; skip the column).
                    && open(Vec3::new(x, y, s + 1))
                    && open(Vec3::new(x, y, s + 2))
                {
                    return Some(Vec3::new(x, y, s + 1));
                }
            }
        }
    }
    None
}

/// CAVE-IN v1 (FR11): the bounded support-check cap (Q2), a collapse's fixed
/// health damage as a FRACTION of max HP (Q6 — a setback, not death; lethality
/// is a later dial), and the fear a collapse instils (Mood is 0=breakdown..
/// 1=content, so fear DROPS it).
pub const CAVEIN_SUPPORT_CAP: usize = 64;
pub const CAVEIN_DAMAGE_FRAC: f32 = 0.25;
pub const CAVEIN_FEAR: f32 = 0.25;

/// CHOP redesign (FR10): the caps bounding a single tree's fell-set walk. A
/// large oak fits comfortably; a canopy MERGED into a worldgen Wood roof (the
/// D15 residual) is CLIPPED instead of felling the building, and the height
/// band stops vertical runaway. The XY RADIUS is the real per-tree boundary:
/// forest canopies CONNECT (b5 run-1: one seed flooded 13 trees' worth to the
/// cell cap), so the walk is confined to a tree-plausible column around the
/// base — neighbouring trees are their own seeds.
/// In DENSE forest even a tight radius catches neighbouring merged canopy, so
/// per-tree sets there legitimately clip at the cap (bounded work per seed;
/// the neighbours are their own seeds and shared cells dedupe at placement) —
/// the cap is the guarantee, not a defect (b5 runs 1-2 finding).
pub const TREE_FELL_CELL_CAP: usize = 2048;
pub const TREE_FELL_HEIGHT_CAP: i32 = 40;
pub const TREE_FELL_RADIUS: i32 = 10;

/// CHOP redesign (FR10 Part 2 step 3): from a CONFIRMED tree base (the caller
/// seeds only from `tree_valid_at`-confirmed oracle positions — never a
/// building), flood the connected Wood+Leaves component: the tree's full
/// block-set (trunk + branches + canopy). Bounded by [`TREE_FELL_CELL_CAP`] +
/// [`TREE_FELL_HEIGHT_CAP`] above the base and 2 below it (roots). PURE
/// (terrain via `is_tree_block`) so it unit-tests without a `TerrainGrid`.
pub fn tree_fell_set(
    is_tree_block: impl Fn(Vec3<i32>) -> bool,
    base: Vec3<i32>,
    cell_cap: usize,
    height_cap: i32,
    radius: i32,
) -> Vec<Vec3<i32>> {
    if !is_tree_block(base) {
        return Vec::new();
    }
    let mut seen: HashSet<Vec3<i32>> = HashSet::new();
    seen.insert(base);
    let mut stack = vec![base];
    let mut cells = Vec::new();
    while let Some(p) = stack.pop() {
        cells.push(p);
        if cells.len() >= cell_cap {
            break; // clipped — never fell past the cap (D15 guard)
        }
        for d in NEIGHBOURS6 {
            let n = p + d;
            if n.z < base.z - 2
                || n.z > base.z + height_cap
                || (n.x - base.x).abs() > radius
                || (n.y - base.y).abs() > radius
            {
                continue;
            }
            if is_tree_block(n) && seen.insert(n) {
                stack.push(n);
            }
        }
    }
    cells
}

/// CAVE-IN v1 (FR11 Q1, reviewer R8/F-CAVE-1+2) + CASE-003: the nearest
/// true-standable relocation cell — MOVED to `common::bastion` so the phys
/// CENTER-SAFETY-NET (common-systems, which cannot see server code) and the
/// cave-in eject share the ONE implementation (B17 identity-by-construction).
/// Re-exported here so every existing server caller keeps its path.
pub use common::bastion::eject_dest;

/// CAVE-IN v1 (FR11 Q1/Q6, reviewer R8/F-CAVE-3): THE eject-and-injure — the
/// ONE implementation both the live mine-completion path (`Sys::run`'s
/// post-loop) and the harness's `bastion_force_collapse_check` call, so the
/// tested path and the shipping path cannot drift. Every colonist whose feet
/// stand in the collapse's crush volume (a falling column, at/below the chunk)
/// is EJECTED to the nearest true standable cell outside the crush
/// ([`eject_dest`]; `None` → left in place, safe — the collapse only REMOVES
/// rock) + INJURED (−[`CAVEIN_DAMAGE_FRAC`] of max health + a [`CAVEIN_FEAR`]
/// Mood drop). Returns the victim count. Generic over the colonist storage so
/// a `ReadStorage` (the hook) and a `WriteStorage` (the system) both fit.
pub fn cavein_eject_and_injure<'a, D>(
    cells: &[Vec3<i32>],
    terrain: &TerrainGrid,
    time: common::resources::Time,
    entities: &Entities<'a>,
    colonists: &specs::Storage<'a, comp::Colonist, D>,
    positions: &mut WriteStorage<'a, comp::Pos>,
    velocities: &mut WriteStorage<'a, comp::Vel>,
    healths: &mut WriteStorage<'a, comp::Health>,
    moods: &mut WriteStorage<'a, comp::bastion::Mood>,
) -> usize
where
    D: std::ops::Deref<Target = specs::storage::MaskedStorage<comp::Colonist>>,
{
    let crush_xy: HashSet<Vec2<i32>> =
        cells.iter().map(|c| Vec2::new(c.x, c.y)).collect();
    let chunk_min_z = cells.iter().map(|c| c.z).min().unwrap_or(i32::MAX);
    let victims: Vec<specs::Entity> = (&**entities, colonists, &*positions)
        .join()
        .filter_map(|(e, _c, p)| {
            let feet = p.0.map(|v| v.floor() as i32);
            (crush_xy.contains(&Vec2::new(feet.x, feet.y)) && feet.z <= chunk_min_z)
                .then_some(e)
        })
        .collect();
    let mut count = 0;
    for entity in victims {
        let feet = positions
            .get(entity)
            .map(|p| p.0.map(|v| v.floor() as i32))
            .unwrap_or_default();
        if let Some(dest) = eject_dest(terrain, feet, &crush_xy) {
            if let Some(pos) = positions.get_mut(entity) {
                pos.0 = dest.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0);
            }
            if let Some(vel) = velocities.get_mut(entity) {
                vel.0 = Vec3::zero();
            }
        }
        if let Some(mut health) = healths.get_mut(entity) {
            let dmg = health.maximum() * CAVEIN_DAMAGE_FRAC;
            health.change_by(comp::HealthChange {
                amount: -dmg,
                by: None,
                cause: None,
                precise: false,
                time,
                instance: rand::random(),
            });
        }
        if let Some(mood) = moods.get_mut(entity) {
            mood.0 = (mood.0 - CAVEIN_FEAR).max(0.0);
        }
        count += 1;
        info!(?feet, "bastion: CAVE-IN — colonist ejected + injured (not buried)");
    }
    count
}

/// The job board resource.
#[derive(Default)]
pub struct JobBoard {
    next_id: JobId,
    pub jobs: HashMap<JobId, Job>,
    /// bastion (B5.8): the union of placed designation volumes — the
    /// colony's terrain-claim mask. Auto carve-steps (self-rescue) is
    /// confined to this mask (expanded by the stair's own rise), so the
    /// system never carves wilderness to chase an out-of-scope target.
    /// Maintained by place (append) / cancel (exact AABB subtraction —
    /// the unit-tested `Region::subtract`).
    pub designated: Vec<Region>,
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
    egress_watch: HashMap<Uid, (Vec3<f32>, f32, bool)>,
    /// bastion (B5.8-E3): per-colonist CLAIM-CHURN watch — (anchor
    /// position, consecutive unreachable releases without leaving it). The
    /// stillness timer can't see a colonist that cycles claim→unreachable→
    /// re-claim (it reads as employed at nearly every pass, and its brief
    /// jobless windows rarely coincide with the sampling tick); the churn
    /// COUNT is the loop's own signature. Threshold → an on-the-spot
    /// annulus test → an egress request, employed or not.
    churn_watch: HashMap<Uid, (Vec3<f32>, u8)>,
    /// bastion (B5.8-E3): egress requests raised OUTSIDE the sampling pass
    /// (the churn detector fires from the every-tick upkeep loop); drained
    /// into the next egress pass, which owns one-plan-at-a-time gating.
    egress_pending: Vec<(Uid, Vec3<i32>, Vec3<i32>)>,
    /// bastion (B6, reviewer F3): consecutive seconds the access economy
    /// has been IDLE (access jobs exist, none claimed). A stale abandoned
    /// plan — e.g. a half-carved egress staircase nobody needs after the
    /// crew found another way out — would otherwise freeze one-plan-at-a-
    /// time colony-wide forever AND sit flagged unreachable on the board.
    access_idle_secs: f32,
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
    pub no_progress_ticks: u64,
    pub travel_timeouts: u64,
    pub failsafe_teleports: u64,
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
    saturation: HashMap<Vec2<i32>, f32>,
    /// bastion (FR13-REV Q4): per-colonist bark cooldown — `allowed_to_speak`
    /// is a capability check, not a rate limit, so the coordination bark
    /// carries its own cadence.
    last_bark: HashMap<Uid, f64>,
    stuck_watch: HashMap<Uid, f32>,
    /// bastion (FR15-TIGHTDIG, flag-gated): per-colonist PROGRESS WINDOW —
    /// (window anchor position, window start time, committed-path index at
    /// window start). The drive-owned displacement signal: progressing =
    /// net displacement from the anchor ≥ [`TIGHTDIG_MIN_PROGRESS`] per
    /// [`TIGHTDIG_WINDOW`], AND (committed-path active ⇒ the path index
    /// advanced). Replaces the beeline best-dist inputs at the watchdog
    /// reset sites when [`tightdig_enabled`]; board-side because
    /// `ActiveJob` is `Copy` (the stuck_watch pattern).
    progress_watch: HashMap<Uid, (Vec3<f32>, f64, usize)>,
    /// bastion (FR15-TIGHTDIG, flag-gated): per-colonist COMMITTED PATH —
    /// (waypoints, next index, the target it was computed for). The
    /// reinstated FR15 committed-path steer: computed ONCE via
    /// `bastion_full_path` (bounded, one-shot), steered waypoint-by-
    /// waypoint, invalidated when the job target moves; `None`/exhausted
    /// falls back to the plain beeline steer (today's behavior).
    path_cache: HashMap<Uid, (Vec<Vec3<i32>>, usize, Vec3<f32>)>,
    /// bastion (FR15-TIGHTDIG, flag-gated): last tick's steer target —
    /// a steer SWITCH (anchor reached → real target, fetch engaged, …)
    /// re-anchors the progress window instead of reading as stall/jump
    /// (re-expresses the old `sdist > best_dist + 4.0` rebase under the
    /// new metric, no dangling beeline reader).
    last_steer: HashMap<Uid, Vec3<f32>>,
    /// bastion (B6 HAUL): painted stockpile zones `(id, region)` — the haul
    /// destinations. Registered at placement, dropped on cancel (dependent
    /// haul jobs cancel with their zone).
    pub stockpiles: Vec<(common::bastion::ZoneId, Region)>,
    /// bastion (ZONE-0): painted ACTIVITY zones `(id, kind, region)` — the
    /// soft-magnet footprints, mirrored into [`ActivityZones`] each
    /// arbitration pass. Same lifecycle as stockpiles.
    pub activity_zones: Vec<(common::bastion::ZoneId, common::bastion::ZoneKind, Region)>,
    next_zone: common::bastion::ZoneId,
    /// bastion (B6 JOB-CORE): the reservation table — ONE item entity
    /// reserved by ONE job (the double-spend guard). Stock itself stays
    /// DERIVED from physical items (D2: never a second mutable count);
    /// this table only prevents two jobs spending one item.
    reservations: HashMap<common::bastion::ReservationId, Uid>,
    next_reservation: common::bastion::ReservationId,
    /// bastion (GATHER deposit ruling): per-colonist set of item defs its
    /// forage collects put in its bag — recorded at emit from the SAME
    /// reclaim source the authoritative handler consumes (a loot-TABLE
    /// sprite could roll a different def than we recorded; such a leftover
    /// rides the bag until a future re-roll — never lost, never duped).
    /// Drained by the end-of-forage [`JobKind::DepositRun`]; keyed by Uid
    /// so a demote/promote round-trip keeps the debt.
    gathered_defs: HashMap<Uid, std::collections::HashSet<String>>,
    /// bastion (CASE-003 belt, persistence form): consecutive ticks each
    /// colonist's capsule CORE has sat inside solid terrain. At
    /// [`EMBED_PERSIST_TICKS`] the colonist is genuinely WEDGED (the
    /// revert-locked class — the seed-21 tree teleport) and is relocated;
    /// transient core-solid states (a top-down digger settling into its own
    /// fresh 1-deep pocket) clear in a few ticks and never trip it.
    embed_watch: HashMap<Uid, u32>,
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
}

impl JobBoard {
    /// Generate jobs for a validated designation region. Returns created ids.
    /// v1 generation: Mine = every filled block in the region; Chop = every
    /// wood block; Build = every currently-empty position (the inverse of
    /// Mine — you're placing new blocks, not removing existing ones), gated
    /// on `BUILD_MATERIAL_ITEM` (B5's single-material stand-in; B6 gives Build
    /// real per-blueprint recipes); Stockpile = none yet (B6 zones).
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
        self.designated.push(region);
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
                            unreachable: false,
                            progress: 0.0,
                            required_item: matches!(
                                kind,
                                DesignationKind::Build | DesignationKind::Ladder
                            )
                            .then_some(BUILD_MATERIAL_ITEM),
                            needs_materials: false,
                            carve_attempted: false,
                            is_access: false,
                            stuck_strikes: 0,
                            // Box-top-relative depth: the descent gate's
                            // "how far below the way out".
                            depth: (region.max.z - z).clamp(0, 255) as u8,
                            reservation: None,
                        });
                        created.push(id);
                    }
                }
            }
        }
        info!(
            ?kind,
            jobs = created.len(),
            "bastion: designation placed"
        );
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
                let Some(surface) = resolve_column_surface(terrain, x, y, hint_z, &extent)
                else {
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
                            unreachable: false,
                            progress: 0.0,
                            required_item: matches!(
                                kind,
                                DesignationKind::Build | DesignationKind::Ladder
                            )
                            .then_some(BUILD_MATERIAL_ITEM),
                            needs_materials: false,
                            carve_attempted: false,
                            is_access: false,
                            stuck_strikes: 0,
                            // Per-column surface-relative depth: the
                            // descent gate's "how far below the way out".
                            depth,
                            reservation: None,
                        });
                        created.push(id);
                    }
                }
            }
        }
        if let Some((z_min, z_max)) = mask_z {
            self.designated.push(Region {
                min: Vec3::new(min_xy.x, min_xy.y, z_min),
                max: Vec3::new(max_xy.x, max_xy.y, z_max),
            });
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

    /// CHOP redesign (FR10): generate Chop jobs for a RESOLVED fell-set — the
    /// block positions of one whole tree, computed by the HANDLER via the
    /// World oracle (`get_area_trees` → `tree_valid_at` → [`tree_fell_set`]).
    /// `bastion_jobs` stays terrain-only: this just makes one job per
    /// handed-in position (re-validated against `job_wanted`, deduped), and
    /// the fell-set's tight AABB joins the claim mask exactly like a painted
    /// designation (the same AABB is echoed to the client as the per-tree
    /// outline box, so cancel-through-the-echo reaches every job).
    pub fn place_chop_cells(
        &mut self,
        terrain: &TerrainGrid,
        cells: &[Vec3<i32>],
    ) -> Vec<JobId> {
        let mut created = Vec::new();
        let occupied: HashSet<Vec3<i32>> = self.jobs.values().map(|j| j.pos).collect();
        let (mut min, mut max) = (Vec3::broadcast(i32::MAX), Vec3::broadcast(i32::MIN));
        for &pos in cells {
            let Ok(block) = terrain.get(pos) else {
                continue;
            };
            if !job_wanted(DesignationKind::Chop, block) || occupied.contains(&pos) {
                continue;
            }
            min = Vec3::partial_min(min, pos);
            max = Vec3::partial_max(max, pos);
            let id = self.next_id;
            self.next_id += 1;
            self.jobs.insert(id, Job {
                kind: common::bastion::JobKind::Designated(DesignationKind::Chop),
                work: DesignationKind::Chop.work_type(),
                pos,
                skill_floor: 0,
                claimed_by: None,
                unreachable: false,
                progress: 0.0,
                required_item: None,
                needs_materials: false,
                carve_attempted: false,
                is_access: false,
                stuck_strikes: 0,
                depth: 0,
                reservation: None,
                        });
            created.push(id);
        }
        if !created.is_empty() {
            self.designated.push(Region { min, max });
        }
        info!(jobs = created.len(), "bastion: chop fell-set placed (FR10)");
        created
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
        self.activity_zones.retain(|(_, _, r)| !r.intersects(&region));
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
            .flat_map(|r| {
                if r.intersects(&region) {
                    r.subtract(&region)
                } else {
                    vec![r]
                }
            })
            .collect();
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
            self.reservations.remove(&rid);
        }
        info!(released = released.len(), "bastion: designation cancelled");
        released
    }

    /// bastion (B6): reserve an item entity for a job. The caller stores
    /// the id on `Job.reservation`; release goes through [`Self::remove_job`]
    /// or [`Self::release_reservation`].
    pub fn reserve(&mut self, item: Uid) -> common::bastion::ReservationId {
        let id = self.next_reservation;
        self.next_reservation += 1;
        self.reservations.insert(id, item);
        id
    }

    pub fn release_reservation(&mut self, id: common::bastion::ReservationId) {
        self.reservations.remove(&id);
    }

    /// Is this item entity already reserved by any job? (Linear scan —
    /// colonies are small; the table holds at most a few dozen entries.)
    pub fn is_reserved(&self, item: Uid) -> bool {
        self.reservations.values().any(|u| *u == item)
    }

    pub fn reserved_item(
        &self,
        id: common::bastion::ReservationId,
    ) -> Option<Uid> {
        self.reservations.get(&id).copied()
    }

    /// bastion (B6): remove a job AND release its reservation — THE removal
    /// path (B17: one place, so a cancelled/moot/completed job can never
    /// leak a reservation).
    pub fn remove_job(&mut self, id: JobId) -> Option<Job> {
        let job = self.jobs.remove(&id);
        if let Some(j) = &job
            && let Some(rid) = j.reservation
        {
            self.reservations.remove(&rid);
        }
        job
    }

    /// bastion (B6): is this cell inside a stockpile footprint? XY + a
    /// tolerant z-band (items REST ON the painted surface; the paint's
    /// z-band needn't contain the resting z exactly).
    pub fn stockpile_at(&self, cell: Vec3<i32>) -> Option<common::bastion::ZoneId> {
        self.stockpiles
            .iter()
            .find(|(_, r)| {
                r.contains_point_xy(cell)
                    && cell.z >= r.min.z - 2
                    && cell.z <= r.max.z + 3
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
        }
    }
}

/// The arbitration + travel + work-execution system.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Tick>,
        Read<'a, DeltaTime>,
        Read<'a, common::resources::Time>,
        Read<'a, IdMaps>,
        ReadExpect<'a, ProgramTime>,
        Write<'a, JobBoard>,
        WriteStorage<'a, comp::Colonist>,
        // WRITE only for the B5.8 position-driven climb assist; every other
        // use reads.
        WriteStorage<'a, comp::Pos>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, ActiveJob>,
        // bastion (B-ASSET1): test-fixture goto orders (harness/arena).
        WriteStorage<'a, comp::bastion::BastionTestGoto>,
        WriteStorage<'a, comp::Agent>,
        WriteStorage<'a, comp::Inventory>,
        WriteExpect<'a, BlockChange>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, common::event::EventBus<CreateItemDropEvent>>,
        // COORDINATION-stigmergic-v1 (FR13-REV Q4): the coordination bark.
        ReadExpect<'a, common::event::EventBus<common::event::ChatEvent>>,
        ReadStorage<'a, comp::CharacterState>,
        WriteStorage<'a, comp::Vel>,
        ReadStorage<'a, comp::PhysicsState>,
        // CAVE-IN v1 (FR11 Q6): eject-and-injure a colonist caught in a
        // collapse's crush volume — health damage + a fear (Mood) drop.
        WriteStorage<'a, comp::Health>,
        WriteStorage<'a, comp::bastion::Mood>,
        // LOD-1 + B6 (nested: the flat tuple hit specs' arity ceiling):
        // the Loaded-gate's entity→npc link + rtsim data (read-only), and
        // B6's loose-drop scan + the vanilla pickup path (reuse, never a
        // second pickup mechanism).
        (
            ReadStorage<'a, common::rtsim::RtSimEntity>,
            ReadExpect<'a, crate::rtsim::RtSim>,
            ReadStorage<'a, comp::PickupItem>,
            ReadExpect<'a, common::event::EventBus<common::event::InventoryManipEvent>>,
            // ZONE-0: the activity-zone mirror the agent magnet reads.
            specs::Write<'a, common::bastion::ActivityZones>,
        ),
    );

    const NAME: &'static str = "bastion_jobs";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut EcsJob<Self>,
        (
            entities,
            tick,
            dt,
            time,
            id_maps,
            program_time,
            mut board,
            mut colonists,
            mut positions,
            uids,
            mut active_jobs,
            mut test_gotos,
            mut agents,
            mut inventories,
            mut block_change,
            terrain,
            item_drop_events,
            chat_events,
            char_states,
            mut velocities,
            physics_states,
            mut healths,
            mut moods,
            (
                rtsim_entities,
                rtsim,
                pickup_items,
                inventory_manip_events,
                mut activity_zones,
            ),
        ): Self::SystemData,
    ) {
        let mut item_drop_emitter = item_drop_events.emitter();
        let mut chat_emitter = chat_events.emitter();
        let mut inv_manip_emitter = inventory_manip_events.emitter();
        // DETRNG (B8 root fix): tick-seeded, not OS entropy — the toss
        // velocities this feeds are cosmetic scatter (drop landing spots →
        // pile merge grouping), and seeding them per-tick makes the whole
        // system reproducible under --seed (the last b5 residual:
        // stone_entities varied run-to-run). Same scatter feel in the live
        // game; deterministic everywhere.
        let mut rng = {
            use rand::SeedableRng;
            rand::rngs::StdRng::seed_from_u64(tick.0 ^ 0xBA57_10AA)
        };
        // Pre-deref so field borrows split (jobs mutably + anchors shared
        // inside the same loop).
        let board = &mut *board;

        // ── LOD-1: the LOADED-GATE ───────────────────────────────────────
        // Once `npc.mode` flips to Simulated, the ECS entity persists until
        // its deferred DeleteEvent is consumed — and this system could
        // still claim / progress / COMPLETE a job for it in that window
        // (an Arrived completion would emit an item drop for an npc the
        // rtsim tier already owns: the both-tiers dupe). Gate BOTH the
        // claim loop and the travel/work upkeep on the authoritative mode
        // (impossible-by-construction, spec §5D). Permissive defaults: an
        // entity with no rtsim link (or a stale npc id) has no Simulated
        // tier to dupe against — treat as loaded.
        let rtsim_data = rtsim.state().data();
        let is_loaded = |entity: specs::Entity| -> bool {
            rtsim_entities.get(entity).is_none_or(|re| {
                rtsim_data
                    .npcs
                    .get(*re)
                    .is_none_or(|npc| {
                        matches!(
                            npc.mode,
                            ::rtsim::data::npc::SimulationMode::Loaded
                        )
                    })
            })
        };

        // ── B5.8: climbing improves with use (Ben: climbing is a SKILL) ──
        // XP accrues while a colonist is actually in the Climb state; the
        // level feeds `scramble_reach` (agent system) — vertical traversal
        // is a GROWING capability, not a binary.
        //
        // ALSO the CLIMB ASSIST (the spec's sanctioned "relax the rules for
        // the Colonist body — they're workers, not players"): a working
        // colonist whose job target is ABOVE gets a guaranteed ascent rate
        // while (a) in the Climb state, (b) airborne against a wall
        // (mid-scramble — only reachable via a path edge the reach model
        // granted), or (c) BESIDE A LADDER block, grounded or not (a ladder
        // is climbable by construction — this makes ladder mounts and
        // pit-pillar exits deterministic; runs 3-8 showed the vanilla
        // jump→Climb entry chain is ~50% timing-flaky). Plain wall-hugging
        // on foot gets nothing, so free-climbing stays bounded by the path
        // graph and the climbing skill. Players and vanilla NPCs untouched.
        const CLIMB_XP_RATE: f32 = 1.5; // xp per second while lifted
        const CLIMB_ASSIST_VZ: f32 = 2.5; // ascent rate, blocks/s
        {
            let mut climb_iter = (
                &mut colonists,
                &char_states,
                &mut velocities,
                (&active_jobs).maybe(),
                &mut positions,
                &physics_states,
            )
                .lend_join();
            while let Some((mut colonist, cs, vel, active, pos, phys)) = climb_iter.next() {
                // B-LIVE3: the fail-safe climbs WITHOUT a job (a dispersing
                // or trapped-idle colonist has none — Ben's "climb out of
                // anywhere"). Job-driven climbs still require the target
                // ABOVE; the fail-safe climbs unconditionally upward while
                // its window lasts (expiry bounds it; on open ground the
                // trapped verdict never renews it).
                let climb_free_now = colonist.0.climb_free_until > time.0;
                let target_above = active
                    .as_ref()
                    .and_then(|a| board.jobs.get(&a.job))
                    .map(|j| j.pos.z as f32 + 1.0 > pos.0.z + 0.2);
                if !climb_free_now && target_above != Some(true) {
                    continue;
                }
                let feet = pos.0.map(|e| e.floor() as i32);
                // 5×5 XY neighborhood at feet/head height: the Chaser
                // abandons its approach anywhere up to ~2.5 blocks out
                // (runs 13/18 deadlocks — stationary just outside a
                // tighter grab), so the grab radius must cover the whole
                // stop band. Ladders are colony-built infrastructure;
                // "working the ladder" from two blocks is fine worker
                // fiction.
                // ±3 grab (was ±2): the vanilla Chaser abandons approaches
                // 1.5-2.5 blocks out, and the anchor walk-steer hands over
                // at 1.6 — a climber parked at 2.1-2.6 from the rung column
                // sat in a DEAD ZONE outside the old grab for minutes
                // (run-11 timestamps). ±3 lets the magnetism reach into the
                // whole stop band and drag them the rest of the way in.
                let mut nearest_ladder: Option<Vec3<i32>> = None;
                for dx in -3..=3i32 {
                    for dy in -3..=3i32 {
                        for dz in 0..=1i32 {
                            let p = feet + Vec3::new(dx, dy, dz);
                            if terrain.get(p).ok().and_then(|b| b.get_sprite())
                                == Some(SpriteKind::Ladder)
                                && nearest_ladder.is_none_or(|b| {
                                    (dx.abs() + dy.abs())
                                        < (b.x - feet.x).abs() + (b.y - feet.y).abs()
                                })
                            {
                                nearest_ladder = Some(p);
                            }
                        }
                    }
                }
                let beside_ladder = nearest_ladder.is_some();
                // REACH CAP on the wall/climb arms: lift only while
                // standable ground is within the colonist's scramble reach
                // below — otherwise the assist would elevator workers up
                // walls of ANY height (run-9 finding: a pit exit "passed"
                // by free-climbing the 5-block wall, bypassing the skill
                // model). Ladders are exempt: beating reach is their job.
                let reach = 2 + colonist.0.skills.climbing.level.min(1) as i32;
                let ground_within_reach = (1..=reach + 1).any(|i| {
                    terrain
                        .get(feet - Vec3::unit_z() * i)
                        .map(|b| b.is_filled())
                        .unwrap_or(false)
                });
                let climbing = matches!(cs, comp::CharacterState::Climb(_));
                let on_wall = phys.on_wall.is_some();
                // B-LIVE3 (Ben's UNIVERSAL CLIMB-OUT): a colonist under the
                // trapped fail-safe climbs ANY wall — no ladder, no reach
                // cap. Granted only by the no-egress verdict / mine-done
                // dispersal; expiry is the hysteresis; the teleport
                // backstop covers even this failing.
                let climb_free = colonist.0.climb_free_until > time.0;
                // POSITION-DRIVEN lift (runs 3-14 lesson: every vanilla
                // physics-TIMING dependency — jump→wall-contact→Climb-state
                // entry — flakes run to run; velocity nudges inherit the
                // flake. Workers on colony access move UP, period): wall
                // contact suffices, airborne not required. Head space is
                // checked so the lift can't push a body into a ceiling.
                let supported = beside_ladder
                    || ((climbing || on_wall) && (ground_within_reach || climb_free));
                if supported {
                    let head_clear = terrain
                        .get(feet + Vec3::unit_z() * 2)
                        .map(|b| !b.is_solid())
                        .unwrap_or(true);
                    if head_clear {
                        // VELOCITY-ONLY lift (B6 SOFT-0 runs 15-21): the
                        // original position-pop gets resolved straight
                        // back down by phys ground-snap when the climber
                        // stands on open floor (on_wall=false at a shaft
                        // mouth — every b58 climber was wall-pressed,
                        // which masked this since B5.8). Owning vz makes
                        // the integrator carry the ascent; and dropping
                        // the pop entirely means the climb can NEVER
                        // tunnel — phys owns all position integration
                        // (run ck-3: pop+momentum embedded a climber in a
                        // ceiling permanently, the exact hard-terrain
                        // violation the scenario asserts against). Same
                        // reach-cap/head-clear gates bound the lift.
                        vel.0.z = vel.0.z.max(CLIMB_ASSIST_VZ);
                        // DISCRETE RUNG-STEP (run 29, Ben's auto-snap
                        // backstop from the access-reliability batch): a
                        // GROUNDED climber whose velocity route gets eaten
                        // by ground physics (carved pockets, ledge lips)
                        // still takes one guaranteed 1-block step per
                        // second — the same supported/head-clear/reach
                        // gates bound it, and the step target's body space
                        // is verified clear so it can never snap into
                        // rock. Reads as mounting a rung.
                        // head_clear (feet+2) IS the step's safety proof: a
                        // 1.75-tall body stepped to feet+1 spans feet+1 ..
                        // feet+2.75 — blocks feet+1 (its current torso ✓)
                        // and feet+2 (head_clear ✓). An extra feet+3 probe
                        // blocked pocket exits one block too early (runs
                        // 32-34 stragglers under half-carved stair cells).
                        if phys.on_ground.is_some() && tick.0 % 30 == 0 {
                            pos.0.z += 1.0;
                        }
                        colonist.0.skills.climbing.add_xp(CLIMB_XP_RATE * dt.0);
                    }
                    // B6 SOFT-0 finding — LADDER MAGNETISM: the grab
                    // window is ±2 XY (the Chaser stop band, runs 13/18),
                    // so a climber can start rising 2 blocks BESIDE the
                    // ladder column; in an open pit it drifts over the
                    // rim, but under a CEILING (an interior chamber→shaft)
                    // it wedges airborne with no ground control and the
                    // watchdog kills the claim. While on the ladder arm,
                    // pull XY toward the ladder column center so the
                    // climber slides INTO the shaft as it rises. Small
                    // per-tick step; the hard terrain pass still resolves
                    // any wall contact (clip-polish per Ben's taste
                    // ruling).
                    if let Some(lp) = nearest_ladder {
                        const LADDER_MAGNET_V: f32 = 1.5; // blocks/s
                        // Pull toward the ladder's OPEN NEIGHBOR column,
                        // not the rung block itself: rungs have
                        // solid_height 1.0 (a rung is a platform), so the
                        // pillar is an impassable pole — the CLIMB space
                        // is the air column beside it (run-6 finding: the
                        // magnet parked climbers ON the bottom rung, where
                        // the rung above failed the head-check). In an
                        // open pit the climber already stands in that
                        // neighbor → no-op; in an interior shaft it pulls
                        // them off the rung into the shaft.
                        let solid = |p: Vec3<i32>| {
                            terrain.get(p).map(|b| b.is_solid()).unwrap_or(true)
                        };
                        let climb_col = [
                            Vec2::new(1, 0),
                            Vec2::new(-1, 0),
                            Vec2::new(0, 1),
                            Vec2::new(0, -1),
                        ]
                        .into_iter()
                        .map(|d| Vec3::new(lp.x + d.x, lp.y + d.y, lp.z))
                        .filter(|c| !solid(*c) && !solid(*c + Vec3::unit_z()))
                        .min_by(|a, b| {
                            let da = Vec2::new(a.x as f32 + 0.5, a.y as f32 + 0.5)
                                .distance_squared(pos.0.xy());
                            let db = Vec2::new(b.x as f32 + 0.5, b.y as f32 + 0.5)
                                .distance_squared(pos.0.xy());
                            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        // WEDGE ESCAPE (runs C2/C3, Ben's auto-snap class):
                        // the magnet can deliver a body ONTO the rung
                        // pillar when the open column lies on the far side
                        // — the climber then stands wedged between rung
                        // solids (sprites are Air-KIND, so the hard-terrain
                        // assert can't even see it) with the rung overhead
                        // failing head-clear forever. Standing inside the
                        // pillar footprint → snap to the open climb
                        // column's floor and restart the climb properly.
                        let on_pillar = terrain
                            .get(feet)
                            .ok()
                            .and_then(|b| b.get_sprite())
                            == Some(SpriteKind::Ladder);
                        if on_pillar && let Some(cc) = climb_col {
                            let solid_at = |p: Vec3<i32>| {
                                terrain
                                    .get(p)
                                    .map(|b| b.is_solid())
                                    .unwrap_or(false)
                            };
                            let mut sz = cc.z;
                            while sz > cc.z - 8
                                && !solid_at(Vec3::new(cc.x, cc.y, sz - 1))
                            {
                                sz -= 1;
                            }
                            // 31.1 (CASE-004-MAGNET, B19): the scanned
                            // floor's HEAD cell must be clear before the
                            // snap writes (sz can sit below the pair
                            // climb_col proved open at the LADDER's z).
                            // Blocked → hold position; the ledge/belt
                            // machinery stays the recovery path.
                            if !solid_at(Vec3::new(cc.x, cc.y, sz + 1)) {
                                pos.0 = Vec3::new(
                                    cc.x as f32 + 0.5,
                                    cc.y as f32 + 0.5,
                                    sz as f32,
                                );
                                vel.0 = Vec3::zero();
                            }
                        } else if let Some(cc) = climb_col {
                            let center =
                                Vec2::new(cc.x as f32 + 0.5, cc.y as f32 + 0.5);
                            let d = center - pos.0.xy();
                            let dist = d.magnitude();
                            if dist > 0.05 {
                                let step = (LADDER_MAGNET_V * dt.0).min(dist);
                                let nudge = d / dist * step;
                                // 31.1 (CASE-004-MAGNET, the confirmed
                                // BC-004 writer, B19): climb_col proved
                                // headroom at the LADDER's z — a mid-climb
                                // nudge lands at the colonist's OWN z,
                                // where the column can be pinched. Gate
                                // the write on 2-high openness at the
                                // DESTINATION cell (the exact climb_col
                                // predicate, own-z); blocked → skip the
                                // nudge entirely — never write, so no
                                // embed occurs at all (the belt stays a
                                // backstop, not the mechanism).
                                let dest = Vec3::new(
                                    (pos.0.x + nudge.x).floor() as i32,
                                    (pos.0.y + nudge.y).floor() as i32,
                                    pos.0.z.floor() as i32,
                                );
                                if !solid(dest) && !solid(dest + Vec3::unit_z()) {
                                    pos.0.x += nudge.x;
                                    pos.0.y += nudge.y;
                                }
                            }
                        }
                    }
                }
                // LEDGE SNAP — one rule kills every crest race: a HANGING
                // climber (supported by the structure, own column below is
                // air) steps onto any face-adjacent walkable ledge at its
                // current height. Covers the gauntlet's intermediate tier
                // crests AND the final rim/plateau dismount (runs 15-17:
                // drift-vs-gravity at each crest was the residual flake).
                let solid = |p: Vec3<i32>| {
                    terrain.get(p).map(|b| b.is_solid()).unwrap_or(false)
                };
                if supported && !solid(feet - Vec3::unit_z()) {
                    // Candidates at CURRENT height and ONE UP: the +1 is
                    // the crest MANTLE (Ben's confirmed live bug + the
                    // chokepoint run-35 straggler one block short at the
                    // shaft lip) — the ledge you exit onto stands a block
                    // ABOVE your hanging feet, so a same-height-only scan
                    // never sees it.
                    let snap = [0i32, 1]
                        .into_iter()
                        .flat_map(|dz| {
                            [
                                Vec2::new(1, 0),
                                Vec2::new(-1, 0),
                                Vec2::new(0, 1),
                                Vec2::new(0, -1),
                            ]
                            .into_iter()
                            .map(move |d| {
                                Vec3::new(feet.x + d.x, feet.y + d.y, feet.z + dz)
                            })
                        })
                        .find(|c| {
                            !solid(*c)
                                && !solid(*c + Vec3::unit_z())
                                && solid(*c - Vec3::unit_z())
                        });
                    if let Some(c) = snap {
                        pos.0 = Vec3::new(c.x as f32 + 0.5, c.y as f32 + 0.5, c.z as f32);
                        vel.0 = Vec3::zero();
                    }
                }
            }
        }

        // ── CREST-DISMOUNT SNAP (Ben live-test; reviewer FR + architect) ──
        // A climber rising to a ledge tops out into air the instant its feet
        // reach the target level — the lift's own gate (`target_above`, in
        // the block above) flips false right there, cutting the assist
        // exactly when the colonist must still cross the horizontal gap onto
        // the ledge. It can't finish the dismount, slips back, and oscillates
        // at the crest (the documented `ladder_pillar` failure: "gravity wins
        // the crossing"). The universal below-grade teleport DOES rescue it,
        // but only at the 60s floor — a long visible stall Ben live-flagged.
        // This makes the dismount PROACTIVE: a job-carrying colonist that has
        // RISEN to its target's crest level and is still HANGING (own column
        // below is air — mid-slip or beside the ladder) snaps onto the
        // nearest walkable dismount cell TOWARD the target — within 2 XY of
        // its feet, at the crest or one below it, head-clear + solid beneath,
        // and never FARTHER in XY from the target than it already is. Keyed
        // to the path target (never a free warp); the hanging gate means one
        // snap onto solid ground ends it (no jitter — next tick it's grounded
        // and excluded). The 60s teleport stays the ultimate backstop.
        {
            let solid = |p: Vec3<i32>| terrain.get(p).map(|b| b.is_solid()).unwrap_or(false);
            let mut dismount_iter =
                (&active_jobs, &mut positions, &mut velocities).lend_join();
            while let Some((active, pos, vel)) = dismount_iter.next() {
                let Some(job) = board.jobs.get(&active.job) else {
                    continue;
                };
                let feet = pos.0.map(|e| e.floor() as i32);
                // The walkable stance ATOP the target block (Mine/access
                // arrive = stand-on-top): a dismounting colonist's feet-block
                // sits one above the target block.
                let crest_z = job.pos.z + 1;
                // "Risen to the crest": feet at the crest (±1 — a slip just
                // under, or the topped-out air block just over). Still below,
                // or well past, is not a dismount.
                if feet.z < crest_z - 1 || feet.z > crest_z + 1 {
                    continue;
                }
                // Must be HANGING (own column below is air): a colonist
                // already standing on solid ground doesn't need the snap, and
                // this makes the snap self-terminating (grounded next tick).
                if solid(feet - Vec3::unit_z()) {
                    continue;
                }
                let tgt = Vec2::new(job.pos.x, job.pos.y);
                let feet_gap = (feet.x - tgt.x).abs().max((feet.y - tgt.y).abs());
                // Nearest-to-target walkable dismount cell within 2 XY of
                // feet, at the crest or one below it (≤1 level below), that
                // does not move the colonist AWAY from the target.
                let mut best: Option<(Vec3<i32>, i32)> = None;
                for dz in [0i32, -1] {
                    for dx in -2..=2i32 {
                        for dy in -2..=2i32 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let c = Vec3::new(feet.x + dx, feet.y + dy, crest_z + dz);
                            let walkable = !solid(c)
                                && !solid(c + Vec3::unit_z())
                                && solid(c - Vec3::unit_z());
                            if !walkable {
                                continue;
                            }
                            let gap = (c.x - tgt.x).abs().max((c.y - tgt.y).abs());
                            if gap > feet_gap {
                                continue; // never snap away from the target
                            }
                            if best.is_none_or(|(_, bg)| gap < bg) {
                                best = Some((c, gap));
                            }
                        }
                    }
                }
                if let Some((c, _)) = best {
                    pos.0 = Vec3::new(c.x as f32 + 0.5, c.y as f32 + 0.5, c.z as f32);
                    vel.0 = Vec3::zero();
                }
            }
        }

        // ── bastion (B-ASSET1): test-goto upkeep (every tick) ────────────
        // Same Goto assertion + 3D arrival + progress-watchdog semantics as
        // job travel below. Terminal states (arrived/stuck) persist on the
        // component for the harness/arena to read; the order stays attached
        // until explicitly removed. Inert when no fixture carries the comp.
        {
            let mut goto_iter = (
                &entities,
                &mut test_gotos,
                &positions,
                &uids,
                (&mut agents).maybe(),
            )
                .lend_join();
            while let Some((_, goto, pos, uid, mut agent)) = goto_iter.next() {
                if goto.arrived || goto.stuck {
                    continue;
                }
                goto.elapsed += dt.0;
                // ARRIVAL is always the REAL distance; only the watchdog
                // compare below takes the FR15-TIGHTDIG input swap (same
                // mirror-semantics as job travel, per this block's design).
                let real_dist = pos.0.distance(goto.target);
                if real_dist < ARRIVE_DIST {
                    goto.arrived = true;
                    if let Some(agent) = agent.as_deref_mut() {
                        agent.rtsim_controller.activity = None;
                    }
                    continue;
                }
                if let Some(agent) = agent.as_deref_mut() {
                    agent.rtsim_controller.activity =
                        Some(common::rtsim::NpcActivity::Goto(goto.target, TRAVEL_SPEED));
                }
                let dist = if tightdig_enabled() {
                    tightdig_measure(
                        &mut board.progress_watch,
                        &mut board.last_steer,
                        None,
                        false,
                        *uid,
                        pos.0,
                        goto.target,
                        time.0,
                        goto.best_dist,
                    )
                } else {
                    real_dist
                };
                if dist + STUCK_EPSILON < goto.best_dist {
                    goto.best_dist = dist;
                    goto.stuck_time = 0.0;
                } else {
                    goto.stuck_time += dt.0;
                    if goto.stuck_time > STUCK_TIMEOUT {
                        goto.stuck = true;
                        if let Some(agent) = agent.as_deref_mut() {
                            agent.rtsim_controller.activity = None;
                        }
                    }
                }
            }
        }

        // ── Travel + work upkeep (every tick) ───────────────────────────
        let mut to_release: Vec<specs::Entity> = Vec::new();
        // B5.8: carve-steps self-rescue requests gathered during upkeep
        // (from-feet, to-job, parent job id) — processed after the loop
        // (the board can't be restructured mid-borrow).
        let mut carve_requests: Vec<(Vec3<i32>, Vec3<i32>, JobId)> = Vec::new();
        // B5.8-E3: unreachable releases feed the claim-churn detector
        // (entity, pos, feet, reach) — processed post-loop (same borrow
        // constraint as carve_requests).
        let mut churn_events: Vec<(specs::Entity, Vec3<f32>, Vec3<i32>, i32)> = Vec::new();
        // B-LIVE3 (mine lifecycle): designations that completed their last
        // job this tick — the post-loop pass marks them done + disperses
        // below-grade miners.
        let mut done_regions: Vec<Region> = Vec::new();
        // CAVE-IN v1 (FR11): floating chunks that collapsed this tick (their
        // cells) — the post-loop pass ejects-and-injures colonists in each
        // crush volume (can't cross-join Health/Mood inside the upkeep loop).
        let mut collapses: Vec<Vec<Vec3<i32>>> = Vec::new();
        // R3 fix-2 (WAITING): a position snapshot for the queue-order test
        // (who is closer to a staged anchor) — the upkeep lend_join can't
        // re-join positions mid-iteration.
        let queue_snapshot: Vec<Vec3<f32>> = (&colonists, &positions)
            .join()
            .map(|(_, p)| p.0)
            .collect();

        // ── ZONE-0: mirror activity zones for the agent magnet ───────────
        // (Arbitration cadence; zones are few — a rewrite beats dirty
        // tracking at this size. The mirror is read-only geometry; the
        // board stays the single authority.)
        if tick.0 % ARBITRATION_INTERVAL as u64 == 5 {
            let mirror: Vec<(common::bastion::ZoneKind, Region)> = board
                .activity_zones
                .iter()
                .map(|(_, k, r)| (*k, *r))
                .collect();
            if activity_zones.0 != mirror {
                activity_zones.0 = mirror;
            }
        }

        // ── B6 HAUL: job generation (arbitration cadence, own offset) ────
        // Scan loose bastion-output drops (stone/log — the two defs the
        // colony produces; required_item is &'static so the def rides
        // there) not already inside a stockpile footprint, not reserved,
        // not already targeted. Reserve AT GENERATION (the double-spend
        // guard starts before any claim); throttled per colonist.
        if tick.0 % ARBITRATION_INTERVAL as u64 == 7 && !board.stockpiles.is_empty()
        {
            let cap = queue_snapshot.len() * HAUL_JOBS_PER_COLONIST;
            let mut pending = board
                .jobs
                .values()
                .filter(|j| matches!(j.kind, common::bastion::JobKind::Haul { .. }))
                .count();
            if pending < cap {
                let occupied: HashSet<Vec3<i32>> =
                    board.jobs.values().map(|j| j.pos).collect();
                for (pickup, ipos, iuid) in
                    (&pickup_items, &positions, &uids).join()
                {
                    if pending >= cap {
                        break;
                    }
                    let matched = match pickup.item().item_definition_id().itemdef_id()
                    {
                        Some(d) if d == MINE_DROP_ITEM => Some(MINE_DROP_ITEM),
                        Some(d) if d == CHOP_DROP_ITEM => Some(CHOP_DROP_ITEM),
                        _ => None,
                    };
                    let Some(static_def) = matched else { continue };
                    let cell = ipos.0.map(|e| e.floor() as i32);
                    if board.stockpile_at(cell).is_some()
                        || board.is_reserved(*iuid)
                        || occupied.contains(&cell)
                    {
                        continue;
                    }
                    let Some(dest) = board
                        .stockpiles
                        .iter()
                        .min_by_key(|(_, r)| {
                            let c = (r.min + r.max) / 2;
                            let d = c - cell;
                            (d.x as i64).pow(2) + (d.y as i64).pow(2) + (d.z as i64).pow(2)
                        })
                        .map(|(z, _)| *z)
                    else {
                        continue;
                    };
                    let rid = board.reserve(*iuid);
                    let id = board.next_id;
                    board.next_id += 1;
                    board.jobs.insert(id, Job {
                        kind: common::bastion::JobKind::Haul {
                            item: *iuid,
                            destination: dest,
                        },
                        work: common::bastion::WorkType::Haul,
                        pos: cell,
                        skill_floor: 0,
                        claimed_by: None,
                        unreachable: false,
                        progress: 0.0,
                        required_item: Some(static_def),
                        needs_materials: false,
                        carve_attempted: false,
                        is_access: false,
                        stuck_strikes: 0,
                        depth: 0,
                        reservation: Some(rid),
                    });
                    pending += 1;
                }
            }
        }
        // ── GATHER deposit ruling (tick-offset 9, its own arbitration
        // slot): the ONE end-of-forage stockpile trip. For each idle loaded
        // colonist still carrying recorded forage, once NO claimable Gather
        // target remains on the board and a stockpile exists, create a
        // DepositRun PRE-CLAIMED for it (nearest zone — the haul-gen
        // picker's shape) and put it straight to work; the bag was the
        // batch unit, so this fires once per forage stint, not per sprite.
        // Orphaned runs (claimant released/demoted → unclaimed) are swept
        // here first — the claim loop never re-assigns them.
        if tick.0 % ARBITRATION_INTERVAL as u64 == 9 {
            let orphans: Vec<JobId> = board
                .jobs
                .iter()
                .filter(|(_, j)| {
                    matches!(
                        j.kind,
                        common::bastion::JobKind::DepositRun { .. }
                    ) && j.claimed_by.is_none()
                })
                .map(|(id, _)| *id)
                .collect();
            for id in orphans {
                board.remove_job(id);
            }
            let gather_open = board.jobs.values().any(|j| {
                j.kind.is(DesignationKind::Gather)
                    && j.claimed_by.is_none()
                    && !j.unreachable
            });
            if !gather_open && !board.stockpiles.is_empty() {
                let mut deposit_runs: Vec<(specs::Entity, JobId)> = Vec::new();
                for (entity, _, pos, uid, ()) in (
                    &entities,
                    &colonists,
                    &positions,
                    &uids,
                    !&active_jobs,
                )
                    .join()
                {
                    if !is_loaded(entity)
                        || !board
                            .gathered_defs
                            .get(uid)
                            .is_some_and(|defs| !defs.is_empty())
                    {
                        continue;
                    }
                    let cell = pos.0.map(|e| e.floor() as i32);
                    let Some((dest, drop_cell)) = board
                        .stockpiles
                        .iter()
                        .min_by_key(|(_, r)| {
                            let c = (r.min + r.max) / 2;
                            let d = c - cell;
                            (d.x as i64).pow(2)
                                + (d.y as i64).pow(2)
                                + (d.z as i64).pow(2)
                        })
                        .map(|(z, r)| {
                            (*z, Vec3::new(
                                (r.min.x + r.max.x) / 2,
                                (r.min.y + r.max.y) / 2,
                                r.max.z,
                            ))
                        })
                    else {
                        continue;
                    };
                    let id = board.next_id;
                    board.next_id += 1;
                    board.jobs.insert(id, Job {
                        kind: common::bastion::JobKind::DepositRun {
                            destination: dest,
                        },
                        work: common::bastion::WorkType::Haul,
                        pos: drop_cell,
                        skill_floor: 0,
                        claimed_by: Some(*uid),
                        unreachable: false,
                        progress: 0.0,
                        required_item: None,
                        needs_materials: false,
                        carve_attempted: false,
                        is_access: false,
                        stuck_strikes: 0,
                        depth: 0,
                        reservation: None,
                    });
                    board.total_claims += 1;
                    info!(
                        job = id,
                        colonist = %uid,
                        zone = dest,
                        "bastion: forage deposit run created"
                    );
                    deposit_runs.push((entity, id));
                }
                for (entity, job_id) in deposit_runs {
                    let _ = active_jobs.insert(entity, ActiveJob {
                        job: job_id,
                        state: ActiveJobState::Traveling,
                        best_dist: f32::MAX,
                        stuck_time: 0.0,
                        reset_dist: f32::MAX,
                        soft_granted: false,
                        stance: Vec3::unit_z(),
                    });
                }
            }
        }
        // `Colonist`'s storage is change-tracked (synced comp) — mutable
        // multi-storage joins over it need `LendJoin`, not `Join`.
        let mut upkeep_iter = (
            &entities,
            &mut colonists,
            &positions,
            &mut active_jobs,
            (&mut agents).maybe(),
        )
            .lend_join();
        while let Some((entity, mut colonist, pos, active, agent)) = upkeep_iter.next() {
            // LOD-1 Loaded-gate: a demoting colonist (mode already flipped,
            // DeleteEvent pending) gets NO travel/progress/COMPLETION —
            // the rtsim tier owns it. The claim sweep releases its claim.
            if !is_loaded(entity) {
                continue;
            }
            let Some(job) = board.jobs.get_mut(&active.job) else {
                // Cancelled out from under the colonist → re-idle.
                to_release.push(entity);
                continue;
            };
            // B15/FR12: arrive at the COMMITTED work-stance (feet offset), not
            // always on-top. Default (0,0,1) reproduces the pre-B15
            // `job.pos + (0.5,0.5,1.0)` on-top target exactly; an adjacent
            // stance `(±1,0,0)`/`(0,±1,0)` sends the digger BESIDE a `+1`-gap
            // block it can't stand on top of. Pinned at claim (not re-picked).
            let target =
                crate::bastion_actions::approach_target(job.pos, active.stance);
            match active.state {
                ActiveJobState::Traveling => {
                    // ── B6 FETCH LEG (Build material from a stockpile) ──
                    // A claimant holding an item reservation and NOT yet
                    // carrying steers at the RESERVED ITEM, not the site;
                    // the vanilla pickup fires within reach, and `carrying`
                    // flipping hands steering back to the job. The Arrived
                    // transition is suppressed while fetching (standing at
                    // the ITEM is not arrival at the JOB). Haul jobs manage
                    // their own legs (their reservation IS the cargo).
                    let mut fetch_steer: Option<Vec3<f32>> = None;
                    if let Some(rid) = job.reservation
                        && !matches!(job.kind, common::bastion::JobKind::Haul { .. })
                    {
                        let carrying = job.required_item.is_some_and(|req| {
                            inventories.get(entity).is_some_and(|inv| {
                                inv.slots().flatten().any(|i| {
                                    i.item_definition_id().itemdef_id()
                                        == Some(req)
                                })
                            })
                        });
                        if !carrying {
                            let item_uid = board.reservations.get(&rid).copied();
                            let ipos = item_uid
                                .and_then(|u| id_maps.uid_entity(u))
                                .and_then(|ie| positions.get(ie).map(|p| p.0));
                            match (item_uid, ipos) {
                                (Some(u), Some(ip)) => {
                                    // The Chaser parks 1.5-2.5 out — emit
                                    // within the whole band.
                                    if pos.0.distance(ip) < 2.8 {
                                        crate::bastion_actions::emit_pickup(
                                            &mut inv_manip_emitter,
                                            entity,
                                            u,
                                        );
                                    }
                                    fetch_steer = Some(ip);
                                },
                                _ => {
                                    // The reserved item vanished (a player
                                    // grabbed it, a merge consumed it):
                                    // release reservation + claim; next
                                    // arbitration re-evaluates materials.
                                    board.reservations.remove(&rid);
                                    job.reservation = None;
                                    job.needs_materials = true;
                                    to_release.push(entity);
                                    continue;
                                },
                            }
                        }
                    }
                    // B5.8: moot-check DURING travel too — a carve stair (or
                    // any other edit) can consume the claimed block before
                    // the claimant arrives; without this the zombie job
                    // cycles claim→stuck→unreachable forever. Same predicate
                    // the completion re-validation uses.
                    let still_wanted = terrain
                        .get(job.pos)
                        .ok()
                        .is_some_and(|b| match job.kind {
                            common::bastion::JobKind::Designated(d) => job_wanted(d, b),
                            // Haul validity = the ITEM still exists — owned
                            // by the Haul arm, not the block moot-check.
                            // DepositRun validity = the ZONE still exists —
                            // owned by its own Arrived arm.
                            common::bastion::JobKind::Haul { .. }
                            | common::bastion::JobKind::DepositRun { .. } => true,
                        });
                    if !still_wanted {
                        info!(
                            job = active.job,
                            kind = ?job.kind,
                            pos = ?job.pos,
                            "bastion: job moot mid-travel — target block changed; dropped"
                        );
                        board.remove_job(active.job);
                        to_release.push(entity);
                        continue;
                    }
                    // 3D distance: standing on the surface above a deep job
                    // must NOT count as arrival (the watchdog handles it).
                    // B5.8-E anti-loop: repeated stuck-outs grow this job's
                    // arrival tolerance (bounded ~6.1 at 3+ strikes) — the
                    // colonist eventually WORKS THE BLOCK REMOTELY
                    // (mine-from-below) instead of looping forever on a
                    // spot it can't physically stand at.
                    let arrive =
                        ARRIVE_DIST + (job.stuck_strikes.min(3) as f32) * 1.2;
                    let dist = pos.0.distance(target);
                    if fetch_steer.is_none() && dist < arrive {
                        active.state = ActiveJobState::Arrived;
                        if let Some(agent) = agent {
                            agent.rtsim_controller.activity = None;
                        }
                        info!(
                            job = active.job,
                            pos = ?job.pos,
                            "bastion: colonist arrived at job site, working (B5)"
                        );
                    } else {
                        // B5.8: STAGED ROUTING through access anchors — an
                        // ascent beyond this colonist's reach steers to the
                        // nearest registered vertical link (ladder base)
                        // first; the climb assist does the vertical leg.
                        // Necessary because the vanilla incremental A*
                        // resets whenever the agent moves >2 blocks — a
                        // beeline-then-bob at a wall never completes the
                        // search that would find the ladder (run-10 root
                        // cause; the graph itself is proven by the
                        // bastion_vertical_tests).
                        let feet = pos.0.map(|e| e.floor() as i32);
                        let reach =
                            2 + colonist.0.skills.climbing.level.min(1) as i32;
                        let over_reach = job.pos.z - feet.z > reach;
                        // FR15: the anchor lookup is HOISTED so the
                        // no-anchor case can fall through to the committed
                        // waypoint path (a terraced route the full-path
                        // compute found) or the climb-free egress (fix-2)
                        // instead of beelining at an unreachable top target.
                        let anchor_steer = if over_reach {
                            board
                                .access_anchors
                                .iter()
                                .filter(|a| {
                                    a.z >= feet.z - 2
                                        && a.z <= job.pos.z + 2
                                        && a.xy()
                                            .map(|e| e as f32)
                                            .distance(pos.0.xy())
                                            < 24.0
                                })
                                .min_by(|a, b| {
                                    let da = a.xy().map(|e| e as f32).distance(pos.0.xy());
                                    let db = b.xy().map(|e| e as f32).distance(pos.0.xy());
                                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                                })
                                .map(|a| {
                                    let base = Vec3::new(
                                        a.x as f32 + 0.5,
                                        a.y as f32 + 0.5,
                                        a.z as f32 + 1.0,
                                    );
                                    if pos.0.xy().distance(base.xy()) > 1.6 {
                                        // Walking to the anchor base (a
                                        // GROUND-level goal — run 12 proved
                                        // an elevated steer from out here
                                        // stalls the Chaser entirely). The
                                        // Chaser stop band (1.5-2.5 out,
                                        // b58 runs 13/18) is covered by the
                                        // climb assist's ±3 ladder grab +
                                        // magnetism, not by this steer.
                                        base
                                    } else {
                                        // AT the anchor: steer straight UP
                                        // its column (the assist lifts).
                                        // B6 SOFT-0 finding: steering at
                                        // the real target here pins the
                                        // climber against the ceiling 1-2
                                        // blocks OUTSIDE an interior shaft
                                        // (chamber→1×1 ladder column) —
                                        // b58's open-pit pillars never hit
                                        // this because there was nothing
                                        // overhead. Vertical bearing keeps
                                        // the body inside the column; the
                                        // staging condition itself expires
                                        // near the top and hands steer
                                        // back to the real target.
                                        Vec3::new(base.x, base.y, target.z)
                                    }
                                })
                        } else {
                            None
                        };
                        let staged_at_anchor = anchor_steer.is_some();
                        let steer = if let Some(s) = anchor_steer {
                            s
                        } else if tightdig_enabled()
                            && let Some(u) = uids.get(entity).copied()
                        {
                            // FR15-TIGHTDIG Part 2 (flag-gated, Opus-gated
                            // block): the REINSTATED committed-path steer.
                            // The original FR15 revert traded legs because
                            // the beeline watchdog misread path-following
                            // as stall; the displacement-window measure
                            // (tightdig_measure below) owns the progress
                            // verdict now, so the steer and the metric
                            // change TOGETHER — the approved FR17 shape.
                            // One bounded full-path per target
                            // (bastion_full_path, inert since the revert),
                            // waypoint-by-waypoint; None/exhausted → the
                            // plain beeline target (pre-block behavior).
                            let stale = board.path_cache.get(&u).is_none_or(
                                |(_, _, for_t)| {
                                    for_t.distance_squared(target) > 1.0
                                },
                            );
                            if stale {
                                let cfg = common::path::TraversalConfig {
                                    node_tolerance: 1.5,
                                    slow_factor: 0.0,
                                    on_ground: true,
                                    in_liquid: false,
                                    min_tgt_dist: 1.0,
                                    can_climb: true,
                                    // Conservative base reach (per-colonist
                                    // trained reach is a tunable the paired
                                    // A/B judges — FR15 lesson: no solo
                                    // tuning).
                                    scramble_reach: 2,
                                    can_fly: false,
                                    vectored_propulsion: false,
                                    is_target_loaded: true,
                                };
                                match common::path::bastion_full_path(
                                    &*terrain,
                                    pos.0,
                                    target,
                                    &cfg,
                                ) {
                                    Some(wps) if !wps.is_empty() => {
                                        board
                                            .path_cache
                                            .insert(u, (wps, 0, target));
                                    },
                                    _ => {
                                        board.path_cache.remove(&u);
                                    },
                                }
                            }
                            let mut steer = target;
                            let mut exhausted = false;
                            if let Some((wps, idx, _)) =
                                board.path_cache.get_mut(&u)
                            {
                                while *idx < wps.len() {
                                    let wp = wps[*idx].map(|e| e as f32)
                                        + Vec3::new(0.5, 0.5, 0.0);
                                    if pos.0.xy().distance(wp.xy()) < 1.2
                                        && (pos.0.z - wp.z).abs() < 2.0
                                    {
                                        *idx += 1;
                                    } else {
                                        break;
                                    }
                                }
                                if *idx < wps.len() {
                                    steer = wps[*idx].map(|e| e as f32)
                                        + Vec3::new(0.5, 0.5, 0.0);
                                } else {
                                    // Path walked out — beeline the last
                                    // leg (arrive owns the rest).
                                    exhausted = true;
                                }
                            }
                            if exhausted {
                                board.path_cache.remove(&u);
                            }
                            steer
                        } else {
                            // (FR15 committed-waypoint drive + fix-2
                            // travel-steer: REVERTED BY MEASUREMENT after 9
                            // instrumented variants — the committed-path
                            // steer changes the distance-measure semantics
                            // under the WHOLE stuck-economy (watchdog →
                            // soft-grace → queue-release → carve-rescue →
                            // churn → teleport), which was tuned for
                            // beeline/anchor steers: every variant traded
                            // one leg for another (v8/v9: pit carve-rescue
                            // starved, fail-safe teleports 9-15 vs 0).
                            // FR15-TIGHTDIG reinstates it BEHIND THE FLAG
                            // above, paired with the displacement metric;
                            // this arm is the flag-OFF baseline, verbatim.)
                            target
                        };
                        // B6: the FETCH override wins every steer — the
                        // reserved item IS the destination until carried.
                        let steer = fetch_steer.unwrap_or(steer);
                        // R3 fix-2 (WAITING — single-file queue discipline):
                        // when staged at an anchor and ANOTHER colonist is
                        // meaningfully closer to it, WAIT — don't shove
                        // into the funnel, don't run the watchdog on the
                        // queue time. The colonist actually climbing (in
                        // or nearly in the column) never yields; promotion
                        // re-evaluates every arbitration pass.
                        // FR15: gated on ANCHOR staging specifically — a
                        // committed-WAYPOINT steer is also != target, but a
                        // crew-mate near my next waypoint is not a queue
                        // (the misfire parked whole crews in the fix-1
                        // first flight: b58 [20767,36,3] vs [2469,3,0]).
                        if staged_at_anchor {
                            let my_d = pos.0.xy().distance(steer.xy());
                            // Queue-mates only: near MY level (a pad worker
                            // strolling past the shaft TOP is not ahead of
                            // me in the climb queue — W1 over-yielded to
                            // exactly those phantoms and parked the whole
                            // chamber).
                            if my_d > 1.2
                                && queue_snapshot.iter().any(|q| {
                                    (q.z - pos.0.z).abs() <= 4.0
                                        && q.xy().distance(steer.xy()) + 0.5 < my_d
                                })
                            {
                                active.state = ActiveJobState::Waiting;
                                if let Some(agent) = agent {
                                    agent.rtsim_controller.activity = None;
                                }
                                continue;
                            }
                        }
                        // Keep the intent asserted (rtsim brain is gated off
                        // while ActiveJob exists, but agents clear activity
                        // on their own in places).
                        if let Some(agent) = agent {
                            agent.rtsim_controller.activity =
                                Some(common::rtsim::NpcActivity::Goto(steer, TRAVEL_SPEED));
                        }
                        // Watchdog: distance to the CURRENT steer target
                        // must keep improving; pacing near an unreachable
                        // site doesn't count. A large upward JUMP in the
                        // measure means the steer target switched (anchor
                        // reached → real target) — rebase, don't count it
                        // as being stuck.
                        // FR15-TIGHTDIG (flag-gated INPUT SWAP): the same
                        // three-branch machinery below runs unchanged; the
                        // MEASURE feeding it becomes the drive-owned
                        // displacement window (steer-agnostic — correct
                        // under anchor, fetch-override, and committed-path
                        // steers by construction).
                        let sdist = if tightdig_enabled()
                            && let Some(u) = uids.get(entity).copied()
                        {
                            let committed_active = anchor_steer.is_none()
                                && fetch_steer.is_none()
                                && board.path_cache.contains_key(&u);
                            let path_idx =
                                board.path_cache.get(&u).map(|(_, i, _)| *i);
                            tightdig_measure(
                                &mut board.progress_watch,
                                &mut board.last_steer,
                                path_idx,
                                committed_active,
                                u,
                                pos.0,
                                steer,
                                time.0,
                                active.best_dist,
                            )
                        } else {
                            pos.0.distance(steer)
                        };
                        if sdist + STUCK_EPSILON < active.best_dist {
                            active.best_dist = sdist;
                            // R3 fix-1 HYSTERESIS: zero the stall clock
                            // only on ≥1 block of NET progress since the
                            // last zero — sub-block wobble (magnet/hover/
                            // physics jitter clears the 0.5 EPSILON
                            // easily) must not starve the watchdog.
                            if active.reset_dist - sdist >= 1.0 {
                                active.reset_dist = sdist;
                                active.stuck_time = 0.0;
                            }
                        } else if sdist > active.best_dist + 4.0 {
                            active.best_dist = sdist;
                            active.reset_dist = sdist;
                            active.stuck_time = 0.0;
                        } else {
                            // (B6 SOFT-0 run-8/9 bisect: the ×0.2 staged
                            // queue-patience factor is REMOVED while the
                            // dead-Traveling-arm mystery is isolated —
                            // plain accrual, the run-7 configuration.)
                            active.stuck_time += dt.0;
                            // FR15 instrumentation: a tick spent NOT
                            // improving toward the current steer — the
                            // A*-bob accrues here (reported baseline).
                            board.no_progress_ticks += 1;
                            if active.stuck_time > STUCK_TIMEOUT {
                                board.travel_timeouts += 1;
                                // B6 SOFT-0 QUEUE RELEASE: a stall while
                                // STAGED at an anchor (steer != target) is
                                // usually WAITING for a single-file
                                // vertical link — not unreachability.
                                // Release to IDLE with no unreachable flag,
                                // no strikes, no carve: the job returns to
                                // the pool clean and arbitration re-hands
                                // it (often to whoever is now best-placed).
                                // The churn detector still counts these
                                // releases (flag-agnostic "cycling in
                                // place" signature), so a colonist stuck
                                // at a MIRAGE anchor still gets the
                                // humanitarian bubble — no infinite loops.
                                // (This replaces run-8's ×0.2 patience,
                                // which starved all movement: waiting is
                                // handled by RELEASING cleanly, not by
                                // stalling the watchdog.)
                                // B6 SOFT-0 (trigger a — the GRACE WINDOW):
                                // before degrading further, one
                                // soft-collision pass per assignment —
                                // most chokepoint stalls are two colonists
                                // mutually blocking; softened they squeeze
                                // past and progress resumes. This ALSO
                                // gives every claim ≥2 timeouts of real
                                // walking time before any release path
                                // (runs 12/13: a first-timeout queue
                                // release never let the Chaser start from
                                // a crowded spawn — whole crew floor-
                                // parked).
                                // AR-2 (reviewer R1 / checklist P4):
                                // DENSITY-GATE the grace — soft-collision
                                // only helps a colonist↔colonist stall, so
                                // grant the grace ONLY when another
                                // colonist is within squeeze range. A
                                // TERRAIN-blocked stall (nobody nearby)
                                // skips straight to the carve/unreachable
                                // pipeline instead of burning a zero-
                                // benefit STUCK_TIMEOUT. `soft_granted`
                                // still caps it at one attempt.
                                let blocker_near = queue_snapshot.iter().any(|q| {
                                    let d = *q - pos.0;
                                    d.xy().magnitude_squared() < 6.25 // 2.5 XY
                                        && d.z.abs() < 2.0
                                        && d.magnitude_squared() > 0.01 // not self
                                });
                                if !active.soft_granted && blocker_near {
                                    active.soft_granted = true;
                                    active.stuck_time = 0.0;
                                    colonist.0.soft_until =
                                        time.0 + SOFT_GRACE_SECS;
                                    continue;
                                }
                                // B6 SOFT-0 QUEUE RELEASE (second+ timeout,
                                // STAGED at an anchor): waiting for a
                                // single-file vertical link is not
                                // unreachability — release to IDLE with no
                                // unreachable flag, no strikes, no carve;
                                // the job returns to the pool clean and
                                // arbitration re-hands it. The churn
                                // detector still counts these (flag-
                                // agnostic), so a MIRAGE anchor still ends
                                // in the humanitarian bubble — no infinite
                                // loops.
                                // R3 fix-2 retired the mid-climb keep: the
                                // WAITING state now owns queue discipline
                                // (waiters never reach this timeout), and
                                // the hysteresis makes a REAL climb's net
                                // progress reset the clock — so a staged
                                // timeout here is a genuine stall: clean
                                // release + churn accrual.
                                // FR15: anchor-staging SPECIFICALLY (a
                                // stalled waypoint traveler takes the
                                // ordinary carve/unreachable pipeline, the
                                // pre-fix behavior for direct steers).
                                if staged_at_anchor {
                                    let feet = pos.0.map(|e| e.floor() as i32);
                                    let reach = 2
                                        + colonist.0.skills.climbing.level.min(1)
                                            as i32;
                                    churn_events.push((entity, pos.0, feet, reach));
                                    job.claimed_by = None;
                                    to_release.push(entity);
                                    continue;
                                }
                                job.claimed_by = None;
                                // B5.8-E: strike — grows the remote-work
                                // arrival tolerance (see the arrive calc).
                                job.stuck_strikes = job.stuck_strikes.saturating_add(1);
                                // B5.8: a stuck ASCENT beyond THIS
                                // colonist's climbing reach gets one
                                // auto-access attempt before the job is
                                // written off (the pit-trap, solved by the
                                // system). Scramblable rises, descents and
                                // flat approaches fail for other reasons —
                                // no terrain edits for those (also protects
                                // exact-conservation invariants from stray
                                // spoil).
                                let feet = pos.0.map(|e| e.floor() as i32);
                                let reach =
                                    2 + colonist.0.skills.climbing.level.min(1) as i32;
                                // The attempt flag is burned at PLAN time
                                // (post-loop), not here — a request skipped
                                // because another plan is pending must keep
                                // its turn for later.
                                if !job.carve_attempted
                                    && !job.is_access
                                    && job.pos.z - feet.z > reach
                                {
                                    carve_requests.push((feet, job.pos, active.job));
                                } else {
                                    // B5.8-E3: no per-colonist re-claim bar
                                    // here (E2 tried one; it leaked on
                                    // physics wobble and starved the
                                    // strike-grown remote-work convergence
                                    // that marginal sites NEED). Retries are
                                    // the mechanism; the CHURN DETECTOR
                                    // below counts these releases so a
                                    // colonist cycling in place still gets
                                    // the humanitarian bubble (~10s of
                                    // bounces) — employed or not.
                                    job.unreachable = true;
                                    churn_events.push((
                                        entity,
                                        pos.0,
                                        feet,
                                        reach,
                                    ));
                                    info!(
                                        job = active.job,
                                        pos = ?job.pos,
                                        colonist = ?feet,
                                        "bastion: job unreachable — claim released"
                                    );
                                }
                                to_release.push(entity);
                            }
                        }
                    }
                },
                ActiveJobState::Waiting => {
                    // R3 fix-2: promotion = re-enter Traveling at the
                    // arbitration cadence; Traveling's staging re-Waits if
                    // it's still not this colonist's turn. The flip-flop
                    // IS the queue-order re-check, and the watchdog fields
                    // reset each promotion so queue time never reads as
                    // stall.
                    if tick.0 % ARBITRATION_INTERVAL as u64 == 0 {
                        active.state = ActiveJobState::Traveling;
                        active.best_dist = f32::MAX;
                        active.reset_dist = f32::MAX;
                        active.stuck_time = 0.0;
                    }
                },
                ActiveJobState::Arrived => {
                    // ── GATHER deposit ruling: the end-of-forage stockpile
                    // trip — instant at arrival like Haul leg-2. Empty every
                    // recorded forage def from the bag onto the zone cell
                    // (spawn-loadout stacks of the same def ride along —
                    // colony stock either way); a zone erased mid-trip drops
                    // the job but KEEPS the recorded defs, so the next
                    // trigger pass retries against a surviving stockpile.
                    if let common::bastion::JobKind::DepositRun { destination } =
                        job.kind
                    {
                        if board
                            .stockpiles
                            .iter()
                            .any(|(z, _)| *z == destination)
                        {
                            if let Some(u) = uids.get(entity).copied() {
                                let defs = board
                                    .gathered_defs
                                    .remove(&u)
                                    .unwrap_or_default();
                                let mut total = 0u32;
                                if let Some(mut inv) = inventories.get_mut(entity)
                                {
                                    for def in &defs {
                                        total += crate::bastion_actions::deposit_all_of(
                                            &mut inv,
                                            def,
                                            job.pos,
                                            &mut item_drop_emitter,
                                            *program_time,
                                        );
                                    }
                                }
                                info!(
                                    job = active.job,
                                    zone = destination,
                                    defs = defs.len(),
                                    amount = total,
                                    "bastion: forage deposited"
                                );
                            }
                        }
                        board.remove_job(active.job);
                        to_release.push(entity);
                        continue;
                    }
                    // ── B6 HAUL: pickup + drop-off — BEFORE the block-work
                    // path (a haul's "work" is instant at each leg; no
                    // progress accumulation, no moot/tool machinery).
                    // progress encodes the leg: <0.5 = at the ITEM (leg 1),
                    // >=0.5 = at the ZONE (leg 2).
                    if let common::bastion::JobKind::Haul { item, destination } =
                        job.kind
                    {
                        let carrying = job.required_item.is_some_and(|req| {
                            inventories.get(entity).is_some_and(|inv| {
                                inv.slots().flatten().any(|i| {
                                    i.item_definition_id().itemdef_id()
                                        == Some(req)
                                })
                            })
                        });
                        if job.progress < 0.5 {
                            // LEG 1: standing at the item.
                            if id_maps.uid_entity(item).is_some() {
                                // Emit the VANILLA pickup (a re-emit against
                                // a consumed uid no-ops in the handler); the
                                // entity vanishing is the confirmation,
                                // checked next tick.
                                crate::bastion_actions::emit_pickup(
                                    &mut inv_manip_emitter,
                                    entity,
                                    item,
                                );
                                continue;
                            }
                            if carrying {
                                // Cargo aboard — LEG 2: retarget the zone's
                                // drop cell (center column, painted top).
                                if let Some((_, r)) = board
                                    .stockpiles
                                    .iter()
                                    .find(|(z, _)| *z == destination)
                                {
                                    job.pos = Vec3::new(
                                        (r.min.x + r.max.x) / 2,
                                        (r.min.y + r.max.y) / 2,
                                        r.max.z,
                                    );
                                    job.progress = 0.5;
                                    active.state = ActiveJobState::Traveling;
                                    active.best_dist = f32::MAX;
                                    active.reset_dist = f32::MAX;
                                    active.stuck_time = 0.0;
                                } else {
                                    // Zone died mid-haul (the cancel path
                                    // also sweeps these — defensive).
                                    let rid = job.reservation;
                                    if let Some(rid) = rid {
                                        board.reservations.remove(&rid);
                                    }
                                    board.jobs.remove(&active.job);
                                    to_release.push(entity);
                                }
                            } else {
                                // Item vanished and we don't hold it (a
                                // player grabbed it / merged away) — moot.
                                let rid = job.reservation;
                                if let Some(rid) = rid {
                                    board.reservations.remove(&rid);
                                }
                                board.jobs.remove(&active.job);
                                to_release.push(entity);
                            }
                            continue;
                        }
                        // LEG 2: at the zone — drop the WHOLE held stack of
                        // the cargo def (fresh colonists carry none of the
                        // bastion outputs; pile merging re-aggregates).
                        let mut dropped = 0u32;
                        if let Some(req) = job.required_item
                            && let Some(mut inv) = inventories.get_mut(entity)
                        {
                            dropped = crate::bastion_actions::deposit_all_of(
                                &mut inv,
                                req,
                                job.pos,
                                &mut item_drop_emitter,
                                *program_time,
                            );
                        }
                        info!(
                            job = active.job,
                            zone = destination,
                            amount = dropped,
                            "bastion: haul delivered"
                        );
                        let rid = job.reservation;
                        if let Some(rid) = rid {
                            board.reservations.remove(&rid);
                        }
                        board.jobs.remove(&active.job);
                        to_release.push(entity);
                        continue;
                    }
                    // B5: accumulate work, rate scaled by the relevant skill.
                    // TOOL-0 (TOOLS-UPGRADE §3): × the EQUIPPED-tool factor —
                    // bare hands/wrong tool = the slow base, a matching
                    // Pick/Axe/Hammer speeds it up by quality. Skill and
                    // tool multiply (both axes pay). The factor itself is
                    // `common::bastion::tool_factor` (pure, unit-pinned).
                    let skill_level = colonist.0.skills.level_for(job.work);
                    let tool = inventories.get(entity).and_then(|inv| {
                        inv.equipped(comp::slot::EquipSlot::ActiveMainhand)
                            .and_then(|item| match &*item.kind() {
                                comp::item::ItemKind::Tool(t) => {
                                    Some((t.kind, item.quality()))
                                },
                                _ => None,
                            })
                    });
                    job.progress += crate::bastion_actions::work_progress(
                        dt.0,
                        skill_level,
                        job.work,
                        tool,
                    );
                    if job.progress < 1.0 {
                        continue;
                    }

                    // ── GATHER (row 38): its own completion arm, like Haul —
                    // the verb is the VANILLA sprite interaction, not a
                    // terrain edit, so the completion_block machinery below
                    // (whose `None` arm would spin this job forever) never
                    // sees it. Work accrued above (Haul skill + bare-hand
                    // tool factor); at full progress, emit the authoritative
                    // Collect each tick until the SPRITE VACATES — the
                    // handler is idempotent (re-emits no-op once collected
                    // or if the block was edited this tick), and the vacated
                    // block is the confirmation, exactly like Haul leg-1's
                    // entity-vanish. Vacated-by-another-hand (a player beat
                    // us to it) completes the same way: the world state is
                    // the truth, and the handler's single consumption makes
                    // a double-yield impossible.
                    if job.kind.is(DesignationKind::Gather) {
                        let block = terrain.get(job.pos).ok().copied();
                        let collectible =
                            block.is_some_and(|b| b.is_directly_collectible());
                        if collectible {
                            // Deposit ruling: record what this collect will
                            // put in the bag — same reclaim source the
                            // handler consumes (idempotent across re-emits:
                            // a HashSet union).
                            if let Some(u) = uids.get(entity)
                                && let Some(items) = block.and_then(|b| {
                                    comp::Item::try_reclaim_from_block(
                                        b,
                                        terrain.sprite_cfg_at(job.pos),
                                    )
                                })
                            {
                                let bag =
                                    board.gathered_defs.entry(*u).or_default();
                                for (_, item) in &items {
                                    if let Some(def) = item
                                        .item_definition_id()
                                        .itemdef_id()
                                    {
                                        bag.insert(def.to_string());
                                    }
                                }
                            }
                            crate::bastion_actions::emit_collect(
                                &mut inv_manip_emitter,
                                entity,
                                job.pos,
                            );
                        } else {
                            colonist.0.skills.grant_xp(job.work, COMPLETION_XP);
                            info!(
                                job = active.job,
                                pos = ?job.pos,
                                "bastion: gathered"
                            );
                            board.remove_job(active.job);
                            to_release.push(entity);
                        }
                        continue;
                    }

                    // The world may have changed since this job was placed
                    // (vanilla mining, an explosion, another designation's
                    // completed edit): a Mine/Chop job whose block is already
                    // gone must not complete — that would conjure a drop out
                    // of empty air — and a Build job must not overwrite a
                    // block something else placed. Re-check the predicate
                    // placement used; a job it no longer holds for is moot.
                    // Checked before Build's material consumption so a moot
                    // Build job doesn't eat the material.
                    // CHOP redesign (FR10): the drop branch below needs the
                    // block's PRE-REMOVAL kind — Wood drops, Leaves clears
                    // free — so capture it with the validity read.
                    let completed_kind = terrain.get(job.pos).ok().map(|b| b.kind());
                    let still_valid = completed_kind.is_some_and(|k| match job.kind {
                        common::bastion::JobKind::Designated(d) => match d {
                            DesignationKind::Mine => {
                                terrain.get(job.pos).ok().is_some_and(|b| b.is_filled())
                            },
                            DesignationKind::Chop => {
                                matches!(k, BlockKind::Wood | BlockKind::Leaves)
                            },
                            DesignationKind::Build | DesignationKind::Ladder => {
                                terrain.get(job.pos).ok().is_some_and(|b| !b.is_filled())
                            },
                            DesignationKind::Stockpile
                            | DesignationKind::Zone(_) => false,
                            // Gather completes in its own arm above (the
                            // Haul pattern) — defensive.
                            DesignationKind::Gather => false,
                        },
                        // Haul/DepositRun complete in their own arms above —
                        // defensive.
                        common::bastion::JobKind::Haul { .. }
                        | common::bastion::JobKind::DepositRun { .. } => false,
                    });
                    if !still_valid {
                        info!(
                            job = active.job,
                            kind = ?job.kind,
                            pos = ?job.pos,
                            "bastion: job moot — target block changed under it; dropped"
                        );
                        board.remove_job(active.job);
                        to_release.push(entity);
                        continue;
                    }

                    // Defer if another system already edited this block THIS
                    // tick: completing over it would make the block's final
                    // state depend on system run order. Progress stays >=
                    // 1.0; retried next tick against the updated terrain,
                    // where the moot-check above re-decides. Checked before
                    // Build's material consumption — deferring *after*
                    // consuming would strand the material (next tick's
                    // consumption attempt finds an empty inventory and
                    // stalls the job, the item already destroyed).
                    if !block_change.can_set_block(job.pos) {
                        continue;
                    }
                    // CASE-004 (the embedding writer, IDENTIFIED by the
                    // CENTER-SAFETY-NET fire-site diagnostics): a Build
                    // completion placed SOLID rock into a cell a colonist's
                    // body occupied — tick_start == embedded_at, fractional
                    // xy, i.e. someone STANDING there when the block
                    // landed. NEVER complete a solid placement into an
                    // occupied cell: defer exactly like `can_set_block`
                    // above (progress stays >= 1.0, retried next tick; the
                    // occupant walks on within ticks — and the phys net
                    // remains the belt if any writer still slips through).
                    if job.kind.is(DesignationKind::Build)
                        && queue_snapshot.iter().any(|p| {
                            let feet = p.map(|e| e.floor() as i32);
                            feet == job.pos || feet + Vec3::unit_z() == job.pos
                        })
                    {
                        continue;
                    }

                    // Build: consume the required material from the
                    // colonist's inventory now — if it's gone (taken by a
                    // faster claimant elsewhere; single-material stand-in so
                    // this is rare), stall rather than build for free.
                    if let Some(required) = job.required_item {
                        let taken = inventories.get_mut(entity).and_then(|mut inv| {
                            let slot = inv.slots_with_id().find_map(|(slot, item)| {
                                item.as_ref()
                                    .is_some_and(|i| {
                                        i.item_definition_id().itemdef_id() == Some(required)
                                    })
                                    .then_some(slot)
                            });
                            // Consume exactly ONE unit: decrement a stack in
                            // place, remove only a lone item. The previous
                            // `inv.remove(slot)` ate the WHOLE STACK — a
                            // 6-stone stack vanished on the first ladder
                            // rung and the builder stopped being a carrier
                            // (b58 run-2 finding: rungs stuck at 1/5).
                            slot.and_then(|slot| match inv.slot_mut(slot) {
                                Some(Some(item)) if item.amount() > 1 => {
                                    item.decrease_amount(1).ok().map(|_| ())
                                },
                                Some(Some(_)) => inv.remove(slot).map(|_| ()),
                                _ => None,
                            })
                        });
                        if taken.is_none() {
                            job.progress = 0.0;
                            job.needs_materials = true;
                            to_release.push(entity);
                            continue;
                        }
                    }

                    // Complete: authoritative terrain edit (same path
                    // vanilla mining uses — never a raw chonk write) + item
                    // drop + skill XP. Plain `set` is safe here: the
                    // `can_set_block` deferral above already ruled out a
                    // same-tick collision, and nothing runs between it and
                    // this line but our own loop (job positions are unique —
                    // `place_designation` dedupes — so a later iteration
                    // can't race this block either).
                    // B-AG5-CORE: THE completion edit lives in
                    // bastion_actions (None = no terrain edit for this
                    // kind — same continue semantics as before).
                    let Some(new_block) =
                        crate::bastion_actions::completion_block(job.kind)
                    else {
                        continue;
                    };
                    block_change.set(job.pos, new_block);
                    // B5.8: a player-built ladder line registers as an
                    // access anchor too (one per column — XY dedupe), so
                    // staged routing finds it.
                    if job.kind.is(DesignationKind::Ladder)
                        && !board
                            .access_anchors
                            .iter()
                            .any(|a| a.xy().distance_squared(job.pos.xy()) < 4)
                    {
                        info!(pos = ?job.pos, "bastion: access anchor registered (built)");
                        board.access_anchors.push(job.pos);
                    }

                    if let Some(item_id) = match job.kind.designation() {
                        Some(DesignationKind::Mine) => Some(MINE_DROP_ITEM),
                        // CHOP redesign (FR10): only WOOD yields — leaves
                        // clear with no drop (yield scales with trunk size by
                        // construction: one drop per Wood block).
                        Some(DesignationKind::Chop)
                            if completed_kind == Some(BlockKind::Wood) =>
                        {
                            Some(CHOP_DROP_ITEM)
                        },
                        _ => None,
                    } {
                        // B5.5: colonist output is a player resource —
                        // persistent (no despawn timer) and mergeable
                        // (`should_merge: true`), so burst mining aggregates
                        // into piles instead of carpeting the ground with
                        // one entity per block. Gentle toss (was ±2.0
                        // horizontal): drops land close, so spawn-time
                        // merging within MAX_ITEM_MERGE_DIST actually fires.
                        crate::bastion_actions::emit_drop(
                            &mut item_drop_emitter,
                            job.pos,
                            Item::new_from_asset_expect(item_id),
                            *program_time,
                            &mut rng,
                        );
                    }

                    colonist.0.skills.grant_xp(job.work, COMPLETION_XP);
                    info!(
                        job = active.job,
                        kind = ?job.kind,
                        pos = ?job.pos,
                        "bastion: job completed"
                    );
                    let done_pos = job.pos;
                    // CAVE-IN v1 (FR11 Q2/Q3): removing this block may sever a
                    // bounded chunk from the ground mass — check AT COMPLETION
                    // on the current terrain (block_change is deferred, so
                    // floating_chunk treats done_pos as the air it's about to
                    // become). A bounded floater COLLAPSES: its cells drop to
                    // air + a resource item (the floating rock FALLS instead of
                    // hanging — closes what 2b clean-skips), and the crush
                    // volume below is queued for the post-loop eject-and-injure
                    // (nobody is ever buried). Mine only (Chop/Build/Ladder
                    // don't sever rock). A collapse cell that also had its own
                    // Mine job is handled by that job's moot re-check (the block
                    // is already air → dropped, no double-yield).
                    if job.kind.is(DesignationKind::Mine) {
                        let is_filled =
                            |p: Vec3<i32>| terrain.get(p).map(|b| b.is_filled()).unwrap_or(false);
                        if let Some(cells) =
                            floating_chunk(is_filled, done_pos, CAVEIN_SUPPORT_CAP)
                        {
                            for &cell in &cells {
                                block_change.set(cell, Block::empty());
                                item_drop_emitter.emit(CreateItemDropEvent {
                                    pos: comp::Pos(
                                        cell.map(|e| e as f32) + Vec3::broadcast(0.5),
                                    ),
                                    vel: comp::Vel(Vec3::zero()),
                                    ori: comp::Ori::default(),
                                    item: PickupItem::new(
                                        Item::new_from_asset_expect(MINE_DROP_ITEM),
                                        *program_time,
                                        true,
                                    ),
                                    loot_owner: None,
                                    persistent: true,
                                });
                            }
                            info!(
                                ?done_pos,
                                cells = cells.len(),
                                "bastion: CAVE-IN — floating chunk collapsed"
                            );
                            board.cavein_drop_cells += cells.len() as u64;
                            collapses.push(cells);
                        }
                    }
                    board.remove_job(active.job);
                    // reviewer F5 / b58-(d) fix: completing a job is the
                    // ground-truth "making progress" signal — reset the
                    // universal stuck-watch. A colonist steadily clearing
                    // blocks in a confined deep pit (small displacement, so
                    // the leash alone would false-fire the teleport) stays
                    // safe; only a colonist that completes NOTHING for the
                    // full window is teleported.
                    if let Some(u) = uids.get(entity) {
                        board.stuck_watch.remove(u);
                    }
                    // B-LIVE3 (Ben's MINE LIFECYCLE): the designation this
                    // job belonged to may just have finished — collect for
                    // the post-loop done/disperse pass (the board's job map
                    // is queryable here, but colonists/positions are
                    // mid-borrow).
                    for region in board.designated.iter() {
                        if region.contains_point(done_pos)
                            && !board.jobs.values().any(|j| {
                                !j.is_access && region.contains_point(j.pos)
                            })
                        {
                            done_regions.push(*region);
                        }
                    }
                    to_release.push(entity);
                },
            }
        }
        for entity in &to_release {
            if let Some(active) = active_jobs.get(*entity) {
                // If the job still exists and is still claimed by us, free it.
                if let Some(job) = board.jobs.get_mut(&active.job)
                    && job.claimed_by == uids.get(*entity).copied()
                    && !job.unreachable
                {
                    job.claimed_by = None;
                }
            }
            active_jobs.remove(*entity);
            // FR15-TIGHTDIG: the travel episode is over — its window,
            // committed path, and steer memory are stale (a fresh claim
            // re-anchors from scratch, exactly like best_dist = MAX).
            if let Some(u) = uids.get(*entity) {
                board.progress_watch.remove(u);
                board.path_cache.remove(u);
                board.last_steer.remove(u);
            }
            if let Some(agent) = agents.get_mut(*entity) {
                agent.rtsim_controller.activity = None;
            }
        }

        // ── B-LIVE3: MINE DONE + DISPERSE (Ben's mine lifecycle) ─────────
        // A designation whose last block just cleared is DONE: log the
        // completion, and every below-grade colonist inside it gets the
        // dispersal package — a climb-free window (the universal climb-out:
        // any wall, no ladder needed) plus a surface Goto nudge, so miners
        // LEAVE a finished pit instead of loitering at the bottom. The
        // teleport backstop covers even this failing. (Region stays in the
        // claim mask — mask semantics unchanged this pass.)
        for region in done_regions {
            board.done_count += 1;
            info!(?region, "bastion: MINE DONE — dispersing");
            let mut disp_iter =
                (&mut colonists, &positions, (&mut agents).maybe()).lend_join();
            while let Some((mut colonist, pos, agent)) = disp_iter.next() {
                let p = pos.0.map(|e| e.floor() as i32);
                if region.contains_point_xy(p) && p.z < region.max.z {
                    colonist.0.climb_free_until = time.0 + 45.0;
                    if let Some(agent) = agent {
                        // Nudge toward the surface above the region edge;
                        // if the Goto gets overwritten, climb-free +
                        // teleport still guarantee the exit.
                        let out = Vec3::new(
                            region.min.x as f32 - 2.0,
                            pos.0.y,
                            (region.max.z + 2) as f32,
                        );
                        agent.rtsim_controller.activity =
                            Some(common::rtsim::NpcActivity::Goto(out, TRAVEL_SPEED));
                    }
                }
            }
        }

        // ── CAVE-IN v1 (FR11 Q1/Q6): EJECT-AND-INJURE ────────────────────
        // THE INVARIANT that lets cave-ins coexist with the no-entombment
        // guarantee: a colonist caught in a collapse's crush volume is EJECTED
        // (nearest true standable cell OUTSIDE the falling footprint) +
        // INJURED (health damage + a fear drop), NEVER buried. Runs post-loop
        // — the eject/injure need Pos/Vel/Health/Mood writes the upkeep
        // lend_join can't hold. The SHARED `cavein_eject_and_injure` is the
        // one implementation (the harness hook calls the same fn — reviewer
        // R8/F-CAVE-3: the tested path IS the shipping path).
        for cells in collapses {
            cavein_eject_and_injure(
                &cells,
                &terrain,
                *time,
                &entities,
                &colonists,
                &mut positions,
                &mut velocities,
                &mut healths,
                &mut moods,
            );
        }

        // ── CASE-003 belt (persistence form): the EMBED WATCH ────────────
        // A colonist whose capsule CORE (±0.2 around center, torso level)
        // sits inside solid terrain for EMBED_PERSIST_TICKS consecutive
        // ticks is genuinely WEDGED — the revert-locked class (phys reverts
        // an externally-written in-wall pos to tick-start forever; the
        // seed-21 fail-safe-teleport-into-a-tree repro) — and is relocated
        // to the nearest true-standable cell (the ONE shared eject_dest;
        // None → left in place, the job-watchdog remains the slow
        // backstop). Transient core-solid states are NORMAL MINING (a
        // top-down digger settling into its own fresh 1-deep pocket clears
        // within ticks as it mines on; boundary straddles resolve next
        // tick) — PERSISTENCE is the discriminator, learned the hard way:
        // the first belt lived mid-phys with a bare center test and fired
        // on those legitimate transients every ck run. Post-phys settled
        // positions, sequential, deterministic. CENTER_NET_FIRES stays the
        // REPORTED telemetry (0 expected; any climb = a real wedge writer).
        {
            let mut embed_iter =
                (&colonists, &mut positions, &mut velocities, &uids).lend_join();
            while let Some((_, mut pos, mut vel, uid)) = embed_iter.next() {
                let core_solid = [
                    (-0.2f32, -0.2f32),
                    (-0.2, 0.2),
                    (0.2, -0.2),
                    (0.2, 0.2),
                ]
                .into_iter()
                .all(|(dx, dy)| {
                    let corner =
                        Vec3::new(pos.0.x + dx, pos.0.y + dy, pos.0.z)
                            .map(|e| e.floor() as i32)
                            + Vec3::unit_z();
                    terrain
                        .get(corner)
                        .map(|b| b.is_filled())
                        .unwrap_or(false)
                });
                if core_solid {
                    let n = board.embed_watch.entry(*uid).or_insert(0);
                    *n += 1;
                    if *n >= EMBED_PERSIST_TICKS {
                        *n = 0;
                        let feet = pos.0.map(|e| e.floor() as i32);
                        if let Some(d) =
                            eject_dest(&terrain, feet, &HashSet::new())
                        {
                            tracing::warn!(
                                embedded_at = ?pos.0,
                                relocated_to = ?d,
                                "bastion EMBED WATCH: colonist WEDGED in \
                                 terrain (persisted a full second) — \
                                 relocated; hunt the writer"
                            );
                            pos.0 = d.map(|e| e as f32)
                                + Vec3::new(0.5, 0.5, 0.0);
                            vel.0 = Vec3::zero();
                            common::bastion::CENTER_NET_FIRES.fetch_add(
                                1,
                                core::sync::atomic::Ordering::Relaxed,
                            );
                        }
                    }
                } else {
                    board.embed_watch.remove(uid);
                }
            }
        }

        // ── B5.8-E3: CLAIM-CHURN trapped detector ────────────────────────
        // A colonist cycling claim→unreachable→re-claim reads as EMPLOYED
        // at nearly every sampling pass, so the stillness timer never sees
        // it (E2's reset tweak wasn't enough; E3 run-2 showed widening the
        // accrual to employed colonists false-fires on legitimate WAITING
        // instead). The loop's own signature — consecutive unreachable
        // releases without leaving the spot — is unambiguous: count them,
        // and at the threshold run the same annulus test the stillness
        // path uses. Genuinely walled-in → egress request (drained by the
        // next egress pass, which owns one-plan-at-a-time + claim release);
        // open ground → not trapped, the retries keep converging via the
        // strike-grown arrival (a hard target is not an emergency).
        const CHURN_TRAPPED_RELEASES: u8 = 8; // ≈10s of ~1.2s bounce cycles
        for (entity, posf, feet, reach) in churn_events {
            let Some(uid) = uids.get(entity).copied() else {
                continue;
            };
            let churn = board.churn_watch.entry(uid).or_insert((posf, 0));
            // Leash 6 (matches the stillness watch, same rationale): a
            // climber hover-wobbling at a shaft mouth (magnet + falls)
            // paces 3-5 blocks — the old 3-block leash reset the count
            // every cycle and the F2 chain never reached its threshold
            // (runs A1-A3: an unreleasable mid-climb claim + a blind
            // stillness path + a never-firing churn = 600s of hover). The
            // egress_scan verdict is the false-positive guard, not the
            // leash.
            if posf.distance_squared(churn.0) > 36.0 {
                *churn = (posf, 1);
                continue;
            }
            churn.1 = churn.1.saturating_add(1);
            // (The dead persistent-churn teleport tier — churn.1 >= 16,
            // unreachable because the reset below sawtooths it 0→8→0 —
            // was REMOVED per reviewer F5. The verdict-INDEPENDENT
            // `stuck_watch` teleport below now owns the ultimate backstop,
            // closing the has_egress false-positive the old tiers missed.)
            if churn.1 < CHURN_TRAPPED_RELEASES {
                continue;
            }
            churn.1 = 0; // one shot; re-arms if the cycling continues
            // GUARDS, reviewer-F2 shape (the original anchor-PROXIMITY
            // guard was wrong: near-an-anchor ≠ usable-by-this-colonist —
            // runs 25-26's shaft straggler churned forever beside a ladder
            // it couldn't win, unrescued). The AUTHORITATIVE check is the
            // egress_scan VERDICT below — the only guard kept above it is
            // one-plan-at-a-time (a pending access plan means the rescue
            // economy is already working; round-3 showed a second bubble
            // disorders it; the F3 staleness pruner bounds how long that
            // gate can hold).
            let access_busy = board.jobs.values().any(|j| j.is_access);
            if access_busy {
                continue;
            }
            let (has_egress, rim) = egress_scan(&terrain, feet, reach);
            if !has_egress && let Some(target) = rim {
                info!(
                    ?feet,
                    "bastion: claim-churn trapped — egress requested (B5.8-E3)"
                );
                board.egress_pending.push((uid, feet, target));
            }
        }

        // ── B5.8: AUTONOMOUS ACCESS (self-rescue) ────────────────────────
        // A stuck ascent inside colony claims gets access that FITS THE
        // GEOMETRY (Ben's directive — autonomous access is the default):
        // 1. STAIRS where the claim has room — the SHARED `carve_ramp`
        //    decomposition (one lib, DF-DIG-VERBS' player verb is the other
        //    caller), switchbacking inside the access mask, refusing
        //    floorless routes.
        // 2. LADDER where it's tight or hollow — a material-free rung
        //    pillar (built from spoil) up an adjacent open column.
        // 3. Neither fits → unreachable, as before. Never touches terrain
        //    outside the claim mask (the never-chase-a-deer rule).
        // Emitted steps/rungs are ordinary jobs: arbitration assigns them
        // (nearest wins — usually the stuck colonist; the exposure gate
        // sequences stair digs bottom-up naturally).
        //
        // ONE PLAN AT A TIME: while any access job is pending, no new plan
        // is emitted (requests keep their turn — the attempt flag burns at
        // plan time). Concurrent plans overlap and dig each other's step
        // floors out (run-7's gallery of chaos); one stair serves everyone.
        let access_pending = board.jobs.values().any(|j| j.is_access);
        for (from, to, parent) in carve_requests.into_iter().take(
            if access_pending { 0 } else { 1 },
        ) {
            if let Some(job) = board.jobs.get_mut(&parent) {
                job.carve_attempted = true;
            }
            let mask = board.designated.clone();
            match plan_access(board, &terrain, &mask, from, to) {
                Some((kind, steps)) => {
                    info!(
                        parent,
                        steps,
                        ?kind,
                        "bastion: auto-access emitted (B5.8 self-rescue)"
                    );
                },
                None => {
                    if let Some(job) = board.jobs.get_mut(&parent) {
                        job.unreachable = true;
                    }
                    info!(
                        job = parent,
                        "bastion: auto-access refused (no in-claim route) — job unreachable"
                    );
                },
            }
        }

        // ── B6 SOFT-0 (trigger b): CLUSTERING RELIEF ────────────────────
        // A chokepoint IS high local density: > N other colonists within a
        // small radius → soft-collision before the pile-up deadlocks.
        // O(n²) over loaded colonists (colonies are small); ~1s cadence.
        if tick.0 % 30 == 11 {
            let snapshot: Vec<Vec3<f32>> = (&colonists, &positions)
                .join()
                .map(|(_, p)| p.0)
                .collect();
            let mut dense_iter = (&mut colonists, &positions).lend_join();
            while let Some((mut colonist, pos)) = dense_iter.next() {
                let nearby = snapshot
                    .iter()
                    .filter(|p| {
                        let d = **p - pos.0;
                        // Excludes self (distance 0 counts once — subtract
                        // below).
                        d.xy().magnitude_squared()
                            < SOFT_DENSITY_R * SOFT_DENSITY_R
                            && d.z.abs() < 2.0
                    })
                    .count()
                    .saturating_sub(1); // self
                // Skip the write when already soft well past the next
                // pass — `Colonist` is change-tracked/synced, and a
                // no-op refresh every second would re-sync the comp.
                // (The read goes through Deref, not DerefMut — unflagged.)
                if nearby >= SOFT_DENSITY_N
                    && colonist.0.soft_until < time.0 + 1.5
                {
                    colonist.0.soft_until = time.0 + SOFT_GRACE_SECS;
                }
            }
        }

        // ── B5.8-E: EMERGENCY EGRESS (the "nobody entombed" fail-safe) ──
        // Ben's live-test repro: mine a shaft, DELETE the zone — the digger
        // is stranded: no job (no watchdog trigger) and no claims (no carve
        // permission). The override: a colonist that has been stationary
        // ~20s while NOT actually working (jobless OR spinning on a claim —
        // E3) with NO reachable step up anywhere in a radius-5 ring (a
        // surviving stair/ladder step within scramble reach counts as
        // egress — no spurious carving) gets an access plan under a
        // HUMANITARIAN BUBBLE mask around them, independent of zone state.
        // The emitted steps/rungs are ordinary access jobs — the trapped
        // colonist claims them and digs/builds its own way out.
        const EGRESS_STILL_SECS: f32 = 20.0;
        const EGRESS_BUBBLE_R: i32 = 8;
        if tick.0 % 30 == 7 {
            // Colonists ACTUALLY WORKING (Arrived, progress accruing) are
            // the watchdog's problem, not the trapped detector's — clear
            // their watch so the still-timer only accrues across
            // consecutive non-working seconds (brief between-claim gaps
            // were summing to spurious triggers). Merely being EMPLOYED
            // must NOT reset it (E2: churn starved the timer) — but
            // employed colonists also must NOT ACCRUE it (E3 run-2:
            // colonists legitimately WAITING in line at a ladder/chokepoint
            // fired spurious bubbles all over parts b/c). So: Arrived
            // removes, jobless accrues, employed-Traveling FREEZES — and
            // the claim-churn looper (employed at nearly every sample) is
            // caught by its own signature in the upkeep loop instead (the
            // churn detector below feeds `egress_pending`).
            for (_, uid, active) in (&colonists, &uids, &active_jobs).join() {
                if matches!(active.state, ActiveJobState::Arrived) {
                    board.egress_watch.remove(uid);
                }
            }
            let mut egress_requests: Vec<(Uid, Vec3<i32>, Vec3<i32>)> = Vec::new();
            // Churn-detector fire (at most ONE per pass — extras are
            // dropped and re-arm through continued cycling; round-3
            // finding: batch-draining released whole crews' claims at
            // once). It carries its own annulus verdict; release the
            // stuck claim so the colonist is free for its rescue steps.
            let churn_fire = board.egress_pending.pop();
            board.egress_pending.clear();
            if let Some((uid, from, to)) = churn_fire {
                if let Some(entity) = id_maps.uid_entity(uid) {
                    if let Some(active) = active_jobs.get(entity) {
                        if let Some(job) = board.jobs.get_mut(&active.job) {
                            if job.claimed_by == Some(uid) {
                                job.claimed_by = None;
                            }
                        }
                        active_jobs.remove(entity);
                        if let Some(agent) = agents.get_mut(entity) {
                            agent.rtsim_controller.activity = None;
                        }
                    }
                    egress_requests.push((uid, from, to));
                }
            }
            let mut still_iter =
                (&mut colonists, &positions, &uids, !&active_jobs).lend_join();
            while let Some((mut colonist, pos, uid, ())) = still_iter.next() {
                let watch = board
                    .egress_watch
                    .entry(*uid)
                    .or_insert((pos.0, 0.0, false));
                // CONFINEMENT radius 6, not stillness radius 3 (chokepoint
                // ck-2 straggler): a jobless colonist PACING inside a 5-wide
                // chamber kept resetting the 3-block anchor and the trapped
                // detector never accrued. The annulus scan is the actual
                // false-positive guard (open-ground idlers read has_egress
                // and reset regardless), so the position leash only needs
                // to distinguish "roaming free" from "circling a cell".
                if pos.0.distance_squared(watch.0) > 36.0 {
                    *watch = (pos.0, 0.0, false);
                    continue;
                }
                watch.1 += 1.0; // this pass runs ~once per second
                if watch.2 || watch.1 < EGRESS_STILL_SECS {
                    continue;
                }
                let feet = pos.0.map(|e| e.floor() as i32);
                let reach = 2 + colonist.0.skills.climbing.level.min(1) as i32;
                let (has_egress, rim) = egress_scan(&terrain, feet, reach);
                if has_egress {
                    // Walkable/climbable ground within the annulus — not
                    // walled in.
                    *watch = (pos.0, 0.0, false);
                    continue;
                }
                let Some(target) = rim else {
                    // Open/flat ground — just idling, not trapped.
                    *watch = (pos.0, 0.0, false);
                    continue;
                };
                watch.2 = true;
                // B-LIVE3: the climb-free fail-safe is granted at the
                // VERDICT, per-colonist — NOT in the plan-emission loop,
                // whose one-plan-at-a-time take(0) swallowed it whenever
                // leftover access jobs existed anywhere (the sealed-pit
                // regression sat trapped 240s because of exactly that).
                // The teleport backstop keys off this window.
                colonist.0.climb_free_until = time.0 + 45.0;
                egress_requests.push((*uid, feet, target));
            }
            // ── B6 (reviewer F3): STALE ACCESS-PLAN PRUNING ──────────────
            // An abandoned plan (access jobs exist, NOBODY claims them for
            // ACCESS_STALE_SECS) is removed wholesale: it was freezing the
            // one-plan-at-a-time gate colony-wide and leaving permanent
            // unreachable flags on the board (chokepoint run-17: 12
            // leftovers after the crew exited another way). Any claim
            // resets the clock; already-built rungs/steps stay (they're
            // real structure); the plan can re-emit fresh if still needed.
            const ACCESS_STALE_SECS: f32 = 20.0;
            let access_jobs_exist = board.jobs.values().any(|j| j.is_access);
            let access_claimed =
                board.jobs.values().any(|j| j.is_access && j.claimed_by.is_some());
            if access_jobs_exist && !access_claimed {
                board.access_idle_secs += 1.0; // this pass ≈ once per second
                if board.access_idle_secs >= ACCESS_STALE_SECS {
                    let before = board.jobs.len();
                    board.jobs.retain(|_, j| !j.is_access);
                    info!(
                        pruned = before - board.jobs.len(),
                        "bastion: stale access plan abandoned (F3 pruner)"
                    );
                    board.access_idle_secs = 0.0;
                }
            } else {
                board.access_idle_secs = 0.0;
            }

            let egress_pending = board.jobs.values().any(|j| j.is_access);
            for (uid, from, to) in egress_requests
                .into_iter()
                .take(if egress_pending { 0 } else { 1 })
            {
                let bubble = [Region {
                    min: Vec3::new(
                        from.x - EGRESS_BUBBLE_R,
                        from.y - EGRESS_BUBBLE_R,
                        from.z - 2,
                    ),
                    max: Vec3::new(
                        from.x + EGRESS_BUBBLE_R,
                        from.y + EGRESS_BUBBLE_R,
                        from.z + 64,
                    ),
                }];
                match plan_access(board, &terrain, &bubble, from, to) {
                    Some((kind, steps)) => {
                        info!(
                            ?kind,
                            steps,
                            ?from,
                            "bastion: EMERGENCY EGRESS emitted (B5.8-E)"
                        );
                    },
                    None => {
                        // Retry shortly rather than burning the attempt —
                        // terrain may open up (or another colonist digs).
                        if let Some(w) = board.egress_watch.get_mut(&uid) {
                            w.1 = EGRESS_STILL_SECS - 10.0;
                            w.2 = false;
                        }
                        info!(?from, "bastion: emergency egress found no route (retry)");
                    },
                }
                // B-LIVE3 (Ben's UNIVERSAL CLIMB-OUT): every trapped
                // verdict ALSO grants the climb-free window — plan or no
                // plan, the colonist may claw its own way up any wall as
                // the fail-safe below the plan tier. 45s; the teleport
                // backstop below fires if even that stalls.
                if let Some(entity) = id_maps.uid_entity(uid)
                    && let Some(mut c) = colonists.get_mut(entity)
                {
                    c.0.climb_free_until = time.0 + 45.0;
                }
            }

            // ── B-LIVE3 / reviewer F5: the UNIVERSAL stuck teleport, the
            // ULTIMATE backstop (Ben: "if all-out fails just teleport them
            // to ground level"). VERDICT-INDEPENDENT — no has_egress gate
            // (the old tier's fatal hole: a shaft-mouth hover reads
            // has_egress=TRUE as a false positive, so the verdict-gated
            // teleport never fired and the colonist hovered forever). Pure
            // position+time: a colonist that ISN'T working (Arrived
            // resets — legitimate stationary) and hasn't moved
            // `STUCK_LEASH` blocks in `STUCK_TELEPORT_SECS` is teleported
            // to the nearest real surface — but ONLY when that surface is
            // meaningfully ELSEWHERE (guards against no-op teleporting an
            // idle colonist already standing on open ground). Loudly
            // logged: every fire means the organic tiers (Waiting →
            // climb-free → egress plan) failed and wants a look.
            // 60s: the COMPLETION-RESET (a colonist completing a job
            // clears its watch) is the real deep-dig guard — a productive
            // digger completes a block every few seconds and never
            // accrues. 60s (vs 90) rescues the sealed-pit / quarry
            // stragglers within their measurement windows while the
            // completion-reset keeps the deep dig safe (the earlier 60s
            // deep-dig regression PREDATED the completion-reset). Bounds
            // entombment tightly.
            const STUCK_TELEPORT_SECS: f32 = 60.0;
            // Working colonists are legitimately stationary — reset.
            for (_, uid, active) in (&colonists, &uids, &active_jobs).join() {
                if matches!(active.state, ActiveJobState::Arrived) {
                    board.stuck_watch.remove(uid);
                }
            }
            let mut tp_iter =
                (&colonists, &uids, &mut positions, &mut velocities).lend_join();
            while let Some((_colonist, uid, pos, vel)) = tp_iter.next() {
                let is_working = id_maps.uid_entity(*uid).and_then(|e| active_jobs.get(e))
                    .is_some_and(|a| matches!(a.state, ActiveJobState::Arrived));
                if is_working {
                    board.stuck_watch.remove(uid);
                    continue;
                }
                let feet = pos.0.map(|e| e.floor() as i32);
                // A REAL DIGGER = has a live job AND stands inside an
                // active designation (a work zone). Teleporting it yanks
                // the dig (b58-(d) over-fire). Both clauses are required
                // (AR-2 reviewer F6): `board.designated` is colony-wide and
                // does NOT shrink on claim-release, so a POSITION-only mask
                // left a jobless colonist trapped INSIDE a designation with
                // no teleport backstop — the "impossible by construction"
                // net had a scope hole (the F5 class, inside a zone). The
                // job clause closes it: a JOBLESS colonist always teleports
                // (even inside a designation), so a churn-demoted or
                // zone-orphaned colonist IS rescued. The chokepoint
                // straggler sits in the pre-carved chamber (no designation)
                // → teleports regardless. A digger's OWN job existing on
                // the board is the real "it's working here" signal.
                let is_real_digger = id_maps
                    .uid_entity(*uid)
                    .and_then(|e| active_jobs.get(e))
                    .is_some_and(|a| board.jobs.contains_key(&a.job))
                    && board.designated.iter().any(|r| r.contains_point(feet));
                if is_real_digger {
                    board.stuck_watch.remove(uid);
                    continue;
                }
                let dest = surface_teleport_dest(&terrain, feet);
                // BELOW GRADE = a real surface exists ABOVE, meaningfully
                // elsewhere (the colonist is in a pit/shaft, not on open
                // ground where dest ≈ current). NOT movement-keyed: a
                // wanderer below grade still accumulates (the e-out hole).
                let below_grade = dest.is_some_and(|d| {
                    d.map(|e| e as f32).xy().distance(pos.0.xy()) >= 3.0
                        || (d.z as f32 - pos.0.z) >= 3.0
                });
                if !below_grade {
                    // On/at a surface — not stuck (B7 feeds idle colonists).
                    board.stuck_watch.remove(uid);
                    continue;
                }
                let secs = board.stuck_watch.entry(*uid).or_insert(0.0);
                *secs += 1.0; // this pass runs ~once per second
                if *secs < STUCK_TELEPORT_SECS {
                    continue;
                }
                if let Some(d) = dest {
                    tracing::warn!(
                        ?feet,
                        ?d,
                        secs = *secs,
                        "bastion: ULTIMATE FAIL-SAFE — teleporting stuck \
                         colonist to ground (organic egress tiers failed)"
                    );
                    pos.0 = d.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0);
                    vel.0 = Vec3::zero();
                    board.stuck_watch.remove(uid);
                    // FR15 instrumentation: fix-2's success measure is this
                    // reverting to a RARE backstop (reported baseline).
                    board.failsafe_teleports += 1;
                }
            }
        }

        // ── Claim sweep: release jobs whose claimant vanished (demoted /
        // despawned) so work never leaks (standing invariant). ─────────────
        if tick.0 % ARBITRATION_INTERVAL == 3 {
            for job in board.jobs.values_mut() {
                if let Some(uid) = job.claimed_by {
                    let alive = id_maps
                        .uid_entity(uid)
                        .is_some_and(|e| active_jobs.get(e).is_some());
                    if !alive {
                        job.claimed_by = None;
                    }
                }
            }
        }

        // ── Unreachable retry: B5 makes terrain change as a *consequence* of
        // job completion (unlike B4, which only ever targeted static
        // terrain), so a block genuinely unreachable now — e.g. the one
        // fully-enclosed interior cell of a solid N³ dig, boxed in on all
        // sides by the same designation's own outer shell — can become
        // reachable once its neighbors are cleared. `unreachable` must not be
        // a permanent life sentence, or a solid volume can never fully clear.
        // Retried periodically rather than every cycle: still costs an
        // arbitration attempt + a fresh watchdog timeout if truly stuck.
        if tick.0 % (ARBITRATION_INTERVAL * 4) == 0 {
            for job in board.jobs.values_mut() {
                job.unreachable = false;
            }
        }

        // ── Arbitration (every ARBITRATION_INTERVAL ticks) ──────────────
        if tick.0 % ARBITRATION_INTERVAL != 0 {
            return;
        }

        // ── COORDINATION-stigmergic-v1 (FR13-REV): the saturation field ──
        // DECAY (per-cell independent → order-free, deterministic), prune the
        // near-zero tail so the map tracks only the live frontier; then
        // DEPOSIT for every colonist currently WORKING (Arrived) — the join
        // is sequential + entity-ordered, so the float sums are fixed-order
        // deterministic (FR13-REV Q2). Read at claim time only (a commitment
        // point) — never continuously (the Q3/B14 anti-bob split: the field
        // steers ALLOCATION; job completion is the monotonic re-flow trigger).
        board.saturation.values_mut().for_each(|v| *v *= COORD_DECAY);
        board.saturation.retain(|_, v| *v > 0.05);
        for (_colonist, active) in (&colonists, &active_jobs).join() {
            if matches!(active.state, ActiveJobState::Arrived)
                && let Some(job) = board.jobs.get(&active.job)
            {
                let cell = coord_cell(job.pos);
                *board.saturation.entry(cell).or_insert(0.0) += COORD_DEPOSIT;
            }
        }

        // B5: a Build job is only eligible for a colonist currently carrying
        // its required material (the single-material stand-in for real
        // hauling/recipes — B6). Also flag/clear `needs_materials` here so
        // the state is visible even before any colonist attempts the job.
        let any_colonist_has_material = |inventories: &WriteStorage<comp::Inventory>| {
            (&colonists, inventories).join().any(|(_, inv)| {
                inv.slots().flatten().any(|item| {
                    item.item_definition_id().itemdef_id() == Some(BUILD_MATERIAL_ITEM)
                })
            })
        };
        let material_available = any_colonist_has_material(&inventories);
        for job in board.jobs.values_mut() {
            // Keyed off required_item (not kind): B5.8's auto-access ladder
            // jobs are material-free and must not be flagged.
            if job.required_item.is_some() && job.claimed_by.is_none() {
                job.needs_materials = !material_available;
            }
        }

        // Claims are marked on the board *during* selection (atomic within
        // the pass — two idle colonists can't pick the same job); the
        // `ActiveJob` comps are inserted afterwards (can't insert while the
        // anti-join borrows the storage).
        // (entity, job, committed work-STANCE offset) — B15/FR12.
        let mut assignments: Vec<(specs::Entity, JobId, Vec3<i32>)> = Vec::new();
        // B5.8 (DF-style dig behavior, Ben's live-test requirement):
        // 1. REACHABILITY GATE — a Mine job is claimable only when EXPOSED
        //    (≥1 of its 6 neighbors non-filled): a digger can stand next to
        //    it. Interior cells unlock as the shell clears; a fresh deep
        //    dig therefore proceeds TOP-DOWN from the surface layer instead
        //    of everyone rushing (and stalling on) the deepest corner. Side
        //    effect: carve-stair steps self-sequence (only the next step is
        //    exposed). Computed once per cycle, not per colonist.
        let mut exposed: HashSet<JobId> = HashSet::new();
        // B15 / reviewer FR12: the STANDABLE set — an exposed Mine cell is only
        // CLAIMABLE if a colonist can actually STAND to work it (terrain-only,
        // once-per-cycle, alongside exposure). Value = the committed work-STANCE
        // (feet offset from job.pos) pinned at claim. Exposure ≠ standability
        // was the bug: a hillside `+1`-gap cell or a floating block passed
        // exposure, got claimed, then never Arrived → churn.
        let mut standable: HashMap<JobId, Vec3<i32>> = HashMap::new();
        for (id, job) in board.jobs.iter_mut() {
            if !job.kind.is(DesignationKind::Mine) || job.claimed_by.is_some() {
                continue;
            }
            let is_exposed = [
                Vec3::new(1, 0, 0),
                Vec3::new(-1, 0, 0),
                Vec3::new(0, 1, 0),
                Vec3::new(0, -1, 0),
                Vec3::new(0, 0, 1),
                Vec3::new(0, 0, -1),
            ]
            .into_iter()
            .any(|d| {
                terrain
                    .get(job.pos + d)
                    .map(|b| !b.is_filled())
                    .unwrap_or(true)
            });
            if is_exposed {
                exposed.insert(*id);
                // ACCESS steps (rescue rungs/stairs) are colony infrastructure
                // laid on reachable ground by construction — never standability-
                // gated (they use the on-top stance, as before). Ordinary Mine
                // cells must have a real stance; a cell with none (isolated
                // floater / walled `+1` gap) is left UNCLAIMED this cycle — NOT
                // flagged unreachable, so no claim→unreachable churn — and
                // retried each cycle as the shell opens (or deferred to
                // cave-in). `job` isn't touched here beyond the read.
                if job.is_access {
                    standable.insert(*id, Vec3::unit_z());
                } else if let Some(stance) = has_standable_stance(&terrain, job.pos) {
                    standable.insert(*id, stance);
                }
            } else {
                // Fully enclosed: flag unreachable-for-now so the audit/UI
                // reflect it (the periodic retry sweep re-tests as the dig
                // opens the shell; B4's buried-job invariant rides this).
                job.unreachable = true;
            }
        }
        // B5.8-E ACCESS-BEFORE-DESCENT (Ben's proactive fix): a dig cell
        // deeper than novice reach below its own surface is CLAIMABLE ONLY
        // once return-access exists nearby (an anchor whose base joins the
        // dig's level range) — access LEADS the descent, so an inescapable
        // hole is never created. The gate tracks the SHALLOWEST held cell;
        // a proactive plan fires for it below (and the ladder extends
        // downward as the dig deepens, one plan per ~4 layers).
        let mut descent_gated: HashSet<JobId> = HashSet::new();
        let mut descent_plan: Option<(JobId, Vec3<i32>, u8)> = None;
        for (id, job) in board.jobs.iter() {
            if !job.kind.is(DesignationKind::Mine)
                || job.is_access
                || job.depth <= 2
                || !exposed.contains(id)
            {
                continue;
            }
            let anchored = board.access_anchors.iter().any(|a| {
                (a.x - job.pos.x).abs().max((a.y - job.pos.y).abs()) <= 8
                    && a.z >= job.pos.z - 1
                    && a.z <= job.pos.z + 4
            });
            if anchored {
                continue;
            }
            descent_gated.insert(*id);
            if descent_plan
                .as_ref()
                .is_none_or(|(_, p, _)| job.pos.z > p.z)
            {
                descent_plan = Some((*id, job.pos, job.depth));
            }
        }
        // The proactive access plan for the shallowest gated layer (one
        // plan at a time, as everywhere): from the open floor ABOVE the
        // gated cell up to its column's own surface.
        if let Some((jid, jpos, jdepth)) = descent_plan
            && !board.jobs.values().any(|j| j.is_access)
        {
            let from = jpos + Vec3::unit_z();
            let to = Vec3::new(jpos.x, jpos.y, jpos.z + jdepth as i32);
            let mask = board.designated.clone();
            if let Some((kind, steps)) = plan_access(board, &terrain, &mask, from, to) {
                info!(
                    job = jid,
                    ?kind,
                    steps,
                    "bastion: proactive descent access emitted (B5.8-E)"
                );
            } else if !AUTO_LADDER_ACCESS {
                // B6-hotfix (Ben live-test, deep-dig throughput — registry
                // D16): the descent gate holds a deep cell until access LEADS
                // the descent. With the auto-ladder fallback disabled,
                // plan_access returns None wherever STAIRS can't fit (a tight
                // footprint can't switchback) and there is NO other access to
                // wait for — so holding here would strand the deep cells
                // UNMINEABLE forever (a tight pit stops at depth 2; b58 saw
                // exactly 75/150). RELEASE the gate: the deep cells become
                // claimable and the universal below-grade teleport is the
                // declared egress (entombment stays impossible by
                // construction — the gate's protective purpose is redundant
                // under that stronger backstop). STAIRS still LEAD the
                // descent wherever they DO fit (the branch above builds them
                // + registers an anchor, and an anchored cell is never gated
                // to begin with); only the can't-build-access case changes.
                // Flag-tied: flip AUTO_LADDER_ACCESS back on and the old
                // gated-descent returns with the ladders.
                descent_gated.clear();
            }
            // On None with the auto-provider ON: the gate holds and retries
            // next cycle (the frontier keeps digging its SAFE layers, the
            // ladder plan leads the descent).
        }
        // 3. DISPERSION — claims (standing + taken this pass) repel new
        //    claims within 2 XY blocks, spreading a work crew across the
        //    frontier instead of stacking on one cell.
        let mut claimed_pos: Vec<Vec3<i32>> = board
            .jobs
            .values()
            .filter(|j| j.claimed_by.is_some())
            .map(|j| j.pos)
            .collect();
        for (entity, colonist, pos, uid, ()) in (
            &entities,
            &colonists,
            &positions,
            &uids,
            !&active_jobs,
        )
            .join()
        {
            // LOD-1 Loaded-gate: never CLAIM for a demoting colonist.
            if !is_loaded(entity) {
                continue;
            }
            let carries_material = inventories.get(entity).is_some_and(|inv| {
                inv.slots().flatten().any(|item| {
                    item.item_definition_id().itemdef_id() == Some(BUILD_MATERIAL_ITEM)
                })
            });
            // Highest priority, then lowest score (distance + B5.8's
            // top-down and dispersion shaping).
            let mut best: Option<(JobId, u8, f32)> = None;
            for (id, job) in board.jobs.iter() {
                if job.claimed_by.is_some() || job.unreachable {
                    continue;
                }
                // GATHER deposit ruling: a DepositRun empties ONE specific
                // colonist's bag — created pre-claimed; an orphan (claimant
                // released/demoted) must never be re-assigned to a colonist
                // whose bag holds nothing (the trigger pass sweeps orphans
                // and re-creates for the right colonist).
                if matches!(
                    job.kind,
                    common::bastion::JobKind::DepositRun { .. }
                ) {
                    continue;
                }
                // B15 / FR12: a Mine cell is claimable only with a STANDABLE
                // stance (⊆ exposed — access always qualifies; an ordinary cell
                // needs a real stance). Replaces the bare exposure gate so
                // unstandable `+1`-gap / floating cells aren't claimed-then-
                // stuck.
                if job.kind.is(DesignationKind::Mine) && !standable.contains_key(id) {
                    continue;
                }
                // B5.8-E: held until return-access leads the descent.
                if descent_gated.contains(id) {
                    continue;
                }
                // B6: a material job with nothing in hand is claimable IF a
                // STOCKPILED loose item of the def is reservable (the fetch
                // leg); Haul jobs are exempt (their cargo IS the job target,
                // reserved at generation). Availability only — the
                // reservation itself commits WITH the claim below.
                if let Some(req) = job.required_item
                    && !carries_material
                    && !matches!(job.kind, common::bastion::JobKind::Haul { .. })
                    && !(&pickup_items, &positions, &uids).join().any(
                        |(pi, ipos, iuid)| {
                            pi.item().item_definition_id().itemdef_id() == Some(req)
                                && board
                                    .stockpile_at(ipos.0.map(|e| e.floor() as i32))
                                    .is_some()
                                && !board.is_reserved(*iuid)
                        },
                    )
                {
                    continue;
                }
                if colonist.0.skills.level_for(job.work) < job.skill_floor {
                    continue;
                }
                let priority = colonist.0.work_priorities.get(job.work);
                if priority == 0 {
                    continue;
                }
                let dist = pos.0.distance(job.pos.map(|e| e as f32));
                // B5.8: ACCESS jobs (rescue rungs/steps) are built by whoever
                // is ON SITE — a distant claimant holding a rescue-critical
                // rung starves the trapped colonist (run-12 deadlock: parked
                // bystanders claimed the pillar, the trapped digger hogged
                // the out-job, nobody moved).
                if job.is_access && dist > 16.0 {
                    continue;
                }
                // …and vertical-access construction outranks ordinary work
                // (a priority TIER, compared before score): the trapped
                // colonist takes its own rescue rungs over re-claiming the
                // unreachable job; a wall crew finishes the ladder before
                // chasing the job on top.
                let priority = if job.is_access || job.kind.is(DesignationKind::Ladder) {
                    priority.saturating_add(1)
                } else {
                    priority
                };
                // 2. TOP-DOWN — one level of height outweighs any plausible
                //    in-dig travel distance AND the dispersion penalty, so
                //    the shallowest frontier clears first (DF-style layer-
                //    by-layer). RELATIVE to the colonist and CLAMPED: an
                //    absolute-z term made any Mine job crush every Build/
                //    Ladder job in cross-kind comparison (run-12: the rung
                //    starvation root). MINE DIGS ONLY: construction goes
                //    bottom-up by nearest-first — a top-weighted rung claim
                //    is unreachable until the rungs below it exist.
                let feet_z = pos.0.z.floor() as i32;
                // B5.8-E3: access steps are EXCLUDED even though they're
                // carved as Mine jobs — an ascent staircase's steps must go
                // NEAREST-first (bottom-up from the trapped digger), and the
                // top-down bonus (−8/level upward) crushed distance so the
                // digger chased the highest shaft-face step it couldn't
                // reach instead of the adjacent bottom one (the tool0-gate
                // (e) bounce carousel).
                let depth_score =
                    if job.kind.is(DesignationKind::Mine) && !job.is_access {
                        -((job.pos.z - feet_z).clamp(-4, 4) as f32) * 8.0
                    } else {
                        0.0
                    };
                let clump_penalty = if claimed_pos.iter().any(|c| {
                    (c.x - job.pos.x).abs() < 2 && (c.y - job.pos.y).abs() < 2
                }) {
                    12.0
                } else {
                    0.0
                };
                // COORDINATION-stigmergic-v1 (FR13-REV Q1): the saturation
                // gradient — repelled from worked/crowded cells, drawn to the
                // under-served frontier. ADDITIVE alongside the in-pass clump
                // repel (the field only knows LAST cycle's work; clump_penalty
                // still prevents same-pass re-clumping — the b58 dispersion
                // gate rides on it). A near-flat field adds ~0 → today's
                // distance/top-down behavior (Q5: continuous degrade, no
                // small-job threshold).
                let sat_penalty = board
                    .saturation
                    .get(&coord_cell(job.pos))
                    .copied()
                    .unwrap_or(0.0)
                    * COORD_SAT_WEIGHT;
                let score = dist + depth_score + clump_penalty + sat_penalty;
                let better = match &best {
                    None => true,
                    Some((_, bp, bs)) => priority > *bp || (priority == *bp && score < *bs),
                };
                if better {
                    best = Some((*id, priority, score));
                }
            }
            if let Some((job_id, _, _)) = best {
                // B6: commit the FETCH reservation with the claim (scoring
                // only checked availability). If the item raced away
                // between passes, skip this claim — next arbitration
                // re-evaluates.
                let mut fetch_rid = None;
                {
                    let needs_fetch = board.jobs.get(&job_id).is_some_and(|j| {
                        j.required_item.is_some()
                            && !carries_material
                            // 34.1 (Sonnet tag-review R-B6HAUL): a RE-CLAIMED
                            // mid-fetch job already HOLDS its reservation —
                            // reserving again orphaned a second one (the
                            // `.or(fetch_rid)` kept the old id and the new
                            // one leaked, one item permanently unreservable
                            // per re-claim).
                            && j.reservation.is_none()
                            && !matches!(
                                j.kind,
                                common::bastion::JobKind::Haul { .. }
                            )
                    });
                    if needs_fetch {
                        let req = board
                            .jobs
                            .get(&job_id)
                            .and_then(|j| j.required_item);
                        let cand = (&pickup_items, &positions, &uids)
                            .join()
                            .find(|(pi, ipos, iuid)| {
                                pi.item().item_definition_id().itemdef_id() == req
                                    && board
                                        .stockpile_at(
                                            ipos.0.map(|e| e.floor() as i32),
                                        )
                                        .is_some()
                                    && !board.is_reserved(**iuid)
                            })
                            .map(|(_, _, iuid)| *iuid);
                        match cand {
                            Some(iuid) => fetch_rid = Some(board.reserve(iuid)),
                            None => continue,
                        }
                    }
                }
                let mut claimed_cell = None;
                if let Some(job) = board.jobs.get_mut(&job_id) {
                    job.claimed_by = Some(*uid);
                    job.reservation = job.reservation.or(fetch_rid);
                    claimed_pos.push(job.pos);
                    claimed_cell = Some(coord_cell(job.pos));
                    // B-LIVE4: count every claim event (initial + re-claim)
                    // for the mine-oscillation claims-per-job telemetry.
                    board.total_claims += 1;
                }
                // COORDINATION-stigmergic-v1 (FR13-REV Q4): narrate a REAL
                // flow — the colonist leaves a markedly more saturated spot
                // for an under-served one. Own per-colonist cooldown
                // (allowed_to_speak is capability, not rate-limit).
                if let Some(new_cell) = claimed_cell {
                    let here = board
                        .saturation
                        .get(&coord_cell(pos.0.map(|e| e.floor() as i32)))
                        .copied()
                        .unwrap_or(0.0);
                    let there =
                        board.saturation.get(&new_cell).copied().unwrap_or(0.0);
                    let barked = board.last_bark.get(uid).copied().unwrap_or(f64::MIN);
                    if here >= there + COORD_BARK_MIN_DIFF
                        && time.0 - barked > COORD_BARK_COOLDOWN_SECS
                    {
                        board.last_bark.insert(*uid, time.0);
                        chat_emitter.emit(common::event::ChatEvent {
                            msg: comp::UnresolvedChatMsg::npc_say(
                                *uid,
                                common::comp::Content::Plain(
                                    "Crowded here — I'll work where they're \
                                     short-handed."
                                        .into(),
                                ),
                            ),
                            from_client: false,
                        });
                    }
                }
                info!(job = job_id, colonist = %uid, "bastion: job claimed");
                // The committed stance (B15/FR12): the standable set's pinned
                // offset for a gated Mine cell; on-top (0,0,1) for everything
                // else (non-Mine jobs, and the pre-B15 default).
                let stance = standable.get(&job_id).copied().unwrap_or(Vec3::unit_z());
                assignments.push((entity, job_id, stance));
            }
        }
        for (entity, job_id, stance) in assignments {
            let _ = active_jobs.insert(entity, ActiveJob {
                job: job_id,
                state: ActiveJobState::Traveling,
                best_dist: f32::MAX,
                stuck_time: 0.0,
                reset_dist: f32::MAX,
                soft_granted: false,
                stance,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CASE-003 pin: the fail-safe picker must SKIP a column whose surface
    /// is occupied by a non-surface solid (a tree trunk standing on the
    /// scanned ground) and land on the next clear column — never inside the
    /// trunk. This is the seed-21 wedge: `column_surface_z` sees through
    /// Wood, the old picker teleported the colonist into it.
    #[test]
    fn surface_teleport_skips_occupied_column() {
        // Every column's surface is z=10; a trunk occupies (0,0,11)+(0,0,12).
        let surface_z = |_x: i32, _y: i32| Some(10);
        let open = |p: Vec3<i32>| {
            p.z > 10 && !(p.xy() == Vec2::new(0, 0) && (p.z == 11 || p.z == 12))
        };
        let dest = surface_teleport_dest_impl(surface_z, open, Vec3::new(0, 0, 5))
            .expect("clear columns exist in the spiral");
        assert_ne!(dest.xy(), Vec2::new(0, 0), "teleported into the trunk");
        assert_eq!(dest.z, 11);
    }

    /// The pit-rim guard stays: a below-grade colonist never gets its OWN
    /// pit floor (s+1 must be strictly above the feet) — the dest is the
    /// surrounding pad's rim.
    #[test]
    fn surface_teleport_requires_above_grade() {
        let surface_z = |x: i32, y: i32| if (x, y) == (0, 0) { Some(4) } else { Some(10) };
        let open = |p: Vec3<i32>| {
            p.z > if p.xy() == Vec2::new(0, 0) { 4 } else { 10 }
        };
        let dest =
            surface_teleport_dest_impl(surface_z, open, Vec3::new(0, 0, 5)).unwrap();
        assert_ne!(dest.xy(), Vec2::new(0, 0), "teleported to own pit floor");
        assert_eq!(dest.z, 11);
    }

    /// Reviewer F1 — THE ±1 BOUNDARY PIN. "Standable" means the rise to
    /// STAND on a surface, (s+1) − feet, is ≤ reach: s ≤ feet+reach−1.
    /// The original `s ≤ feet+reach` admitted rise reach+1 and read the b5
    /// quarry rim (rise 3, reach-2 novice) as escapable — the trapped
    /// detector never fired and the entrapment hid as the "chop flake"
    /// for weeks. If this test needs changing, the CLIMB MODEL changed —
    /// update scramble semantics first, not this assert.
    #[test]
    fn egress_scan_rise_boundary() {
        let feet = Vec3::new(0, 0, 100);
        let reach = 2;
        let flat_rim = |s: i32| move |_x: i32, _y: i32| Some(s);
        // Rise exactly reach (s = feet+reach−1 = 101 → stand 102 − 100 = 2):
        // climbable, EGRESS.
        assert!(egress_scan_with(flat_rim(101), feet, reach).0);
        // Rise reach+1 (s = feet+reach = 102): NOT climbable — the exact
        // off-by-one. Must read WALLED with the rim offered as a plan
        // target.
        let (has, rim) = egress_scan_with(flat_rim(102), feet, reach);
        assert!(!has);
        assert_eq!(rim.map(|r| r.z), Some(102));
        // Level ground and modest drops count as egress (walk off).
        assert!(egress_scan_with(flat_rim(100), feet, reach).0);
        assert!(egress_scan_with(flat_rim(96), feet, reach).0);
        // Below the −4 window: a sheer drop is neither egress nor a rim —
        // (false, None) = "open/flat" upstream (not trapped, no carve).
        let (has, rim) = egress_scan_with(flat_rim(90), feet, reach);
        assert!(!has);
        assert!(rim.is_none());
        // Higher reach widens the standable band by exactly one per level.
        assert!(egress_scan_with(flat_rim(102), feet, 3).0);
        assert!(!egress_scan_with(flat_rim(103), feet, 3).0);
    }

    /// GATHER (row 38) pin: the forage predicate is the FOOD allowlist ∩
    /// directly-collectible — plant sprites in; mineral sprites out (Stones
    /// is hand-collectible but belongs to the Mine economy); bare terrain
    /// out; and no OTHER kind accidentally claims a sprite cell.
    #[test]
    fn gather_job_predicate_is_the_food_allowlist() {
        use common::terrain::{Block, BlockKind, SpriteKind};
        let mush = Block::air(SpriteKind::Mushroom);
        assert!(job_wanted(DesignationKind::Gather, &mush));
        assert!(!job_wanted(
            DesignationKind::Gather,
            &Block::air(SpriteKind::Stones)
        ));
        assert!(!job_wanted(
            DesignationKind::Gather,
            &Block::new(BlockKind::Rock, vek::Rgb::new(120, 120, 120))
        ));
        assert!(!job_wanted(DesignationKind::Gather, &Block::empty()));
        // A sprite cell is air — Mine (filled-only) must not want it.
        assert!(!job_wanted(DesignationKind::Mine, &mush));
    }

    /// CAVE-IN v1 (FR11 Q2): the BOUNDED support check. Removing a block that
    /// severs a small chunk from the grounded mass reports it FLOATING; a
    /// removal inside a big grounded mass (blows past the cap) reports nothing.
    #[test]
    fn floating_chunk_support() {
        let cap = 64;
        // A floating block at (0,0,102) held only by the support (0,0,101);
        // everything at z<=100 is the grounded mass. Remove the support.
        let with_floater = |p: Vec3<i32>| p.z <= 100 || p == Vec3::new(0, 0, 102);
        assert_eq!(
            floating_chunk(with_floater, Vec3::new(0, 0, 101), cap),
            Some(vec![Vec3::new(0, 0, 102)])
        );
        // A multi-cell floater (an L of 3 at z=102) severs together.
        let l_floater = |p: Vec3<i32>| {
            p.z <= 100
                || [
                    Vec3::new(0, 0, 102),
                    Vec3::new(1, 0, 102),
                    Vec3::new(0, 1, 102),
                ]
                .contains(&p)
        };
        let mut got = floating_chunk(l_floater, Vec3::new(0, 0, 101), cap).unwrap();
        got.sort_by_key(|p| (p.x, p.y, p.z));
        assert_eq!(got, vec![
            Vec3::new(0, 0, 102),
            Vec3::new(0, 1, 102),
            Vec3::new(1, 0, 102),
        ]);
        // Inside a big grounded mass → the component blows past the cap →
        // SUPPORTED, nothing floats.
        assert_eq!(
            floating_chunk(|p: Vec3<i32>| p.z <= 100, Vec3::new(0, 0, 100), cap),
            None
        );
        // Nothing solid around the removal → nothing floats.
        assert_eq!(
            floating_chunk(|_p: Vec3<i32>| false, Vec3::new(0, 0, 100), cap),
            None
        );
    }

    /// CHOP redesign (FR10): the bounded whole-tree flood — connected
    /// component from the base, clipped by the cell cap (the D15 guard) and
    /// the height band.
    #[test]
    fn tree_fell_set_bounds() {
        // A 3-block trunk (z 10..12) + a 3-block canopy arm at z 12.
        let tree: Vec<Vec3<i32>> = vec![
            Vec3::new(0, 0, 10),
            Vec3::new(0, 0, 11),
            Vec3::new(0, 0, 12),
            Vec3::new(1, 0, 12),
            Vec3::new(0, 1, 12),
        ];
        let tset: HashSet<Vec3<i32>> = tree.iter().copied().collect();
        let is_tree = |p: Vec3<i32>| tset.contains(&p);
        let mut got = tree_fell_set(is_tree, Vec3::new(0, 0, 10), 4096, 40, 16);
        got.sort_by_key(|p| (p.x, p.y, p.z));
        let mut want = tree.clone();
        want.sort_by_key(|p| (p.x, p.y, p.z));
        assert_eq!(got, want);
        // The CELL CAP clips (never fells past it): an infinite Wood plane
        // yields exactly cap cells.
        let plane = |p: Vec3<i32>| p.z == 10;
        assert_eq!(tree_fell_set(plane, Vec3::new(0, 0, 10), 16, 40, 64).len(), 16);
        // The XY RADIUS is the per-tree boundary (forest canopies CONNECT —
        // without it one seed floods the whole forest to the cap): an
        // infinite plane with radius 2 yields exactly the 5×5 column window.
        assert_eq!(
            tree_fell_set(plane, Vec3::new(0, 0, 10), 4096, 40, 2).len(),
            25
        );
        // The HEIGHT band bounds the walk: an infinite column stops at
        // base+height_cap (and 2 below).
        let column = |p: Vec3<i32>| p.x == 0 && p.y == 0;
        let cells = tree_fell_set(column, Vec3::new(0, 0, 10), 4096, 5, 16);
        assert!(cells.iter().all(|p| p.z >= 8 && p.z <= 15));
        assert_eq!(cells.len(), 8); // z 8..=15
        // A non-tree seed yields nothing.
        assert!(tree_fell_set(is_tree, Vec3::new(9, 9, 9), 4096, 40, 16).is_empty());
    }
}
