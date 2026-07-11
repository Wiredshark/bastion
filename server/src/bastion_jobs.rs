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
        MINE_DROP_ITEM, Region, ZExtent, tool_factor,
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
use rand::RngExt as _;
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

fn work_rate(skill_level: u16) -> f32 {
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
            None => ladder_pillar(terrain, mask, from, to.z)
                .map(|cells| (cells, DesignationKind::Ladder)),
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
            kind,
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
        DesignationKind::Chop => matches!(block.kind(), BlockKind::Wood),
        // B5.8: a ladder rung, like Build, goes into currently-open space.
        DesignationKind::Build | DesignationKind::Ladder => !block.is_filled(),
        DesignationKind::Stockpile => false,
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
            if let Some(s) = column_surface_z(terrain, x, y, hint_z)
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
/// `None` only if no column in range resolves a surface at all.
fn surface_teleport_dest(terrain: &TerrainGrid, feet: Vec3<i32>) -> Option<Vec3<i32>> {
    for r in 0..=8i32 {
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                if let Some(s) =
                    column_surface_z(terrain, feet.x + dx, feet.y + dy, feet.z + 64)
                    // The dest MUST be ABOVE the colonist — a teleport to
                    // the OWN column (r=0) of a pit returns the pit floor
                    // (below grade), teleporting the colonist to itself
                    // (chokepoint sealed-pit fs: tp fired but fs_out stayed
                    // false). Requiring `s ≥ feet.z` finds the surrounding
                    // pad's rim instead — always an upward exit.
                    && s + 1 > feet.z
                {
                    return Some(Vec3::new(feet.x + dx, feet.y + dy, s + 1));
                }
            }
        }
    }
    None
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
    stuck_watch: HashMap<Uid, f32>,
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
                            kind,
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
                let Some(surface) = column_surface_z(terrain, x, y, hint_z) else {
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
                            kind,
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

    /// Cancel all jobs inside a region. Returns the uids whose claims were
    /// released (their `ActiveJob` comps are cleared by the system within
    /// one cycle because the job id no longer exists).
    pub fn cancel_region(&mut self, region: Region) -> Vec<Uid> {
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
            !inside
        });
        info!(released = released.len(), "bastion: designation cancelled");
        released
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
        ReadStorage<'a, comp::CharacterState>,
        WriteStorage<'a, comp::Vel>,
        ReadStorage<'a, comp::PhysicsState>,
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
            char_states,
            mut velocities,
            physics_states,
        ): Self::SystemData,
    ) {
        let mut item_drop_emitter = item_drop_events.emitter();
        let mut rng = rand::rng();
        // Pre-deref so field borrows split (jobs mutably + anchors shared
        // inside the same loop).
        let board = &mut *board;

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
                            pos.0 = Vec3::new(
                                cc.x as f32 + 0.5,
                                cc.y as f32 + 0.5,
                                sz as f32,
                            );
                            vel.0 = Vec3::zero();
                        } else if let Some(cc) = climb_col {
                            let center =
                                Vec2::new(cc.x as f32 + 0.5, cc.y as f32 + 0.5);
                            let d = center - pos.0.xy();
                            let dist = d.magnitude();
                            if dist > 0.05 {
                                let step = (LADDER_MAGNET_V * dt.0).min(dist);
                                let nudge = d / dist * step;
                                pos.0.x += nudge.x;
                                pos.0.y += nudge.y;
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

        // ── bastion (B-ASSET1): test-goto upkeep (every tick) ────────────
        // Same Goto assertion + 3D arrival + progress-watchdog semantics as
        // job travel below. Terminal states (arrived/stuck) persist on the
        // component for the harness/arena to read; the order stays attached
        // until explicitly removed. Inert when no fixture carries the comp.
        {
            let mut goto_iter = (&mut test_gotos, &positions, (&mut agents).maybe()).lend_join();
            while let Some((goto, pos, mut agent)) = goto_iter.next() {
                if goto.arrived || goto.stuck {
                    continue;
                }
                goto.elapsed += dt.0;
                let dist = pos.0.distance(goto.target);
                if dist < ARRIVE_DIST {
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
        // R3 fix-2 (WAITING): a position snapshot for the queue-order test
        // (who is closer to a staged anchor) — the upkeep lend_join can't
        // re-join positions mid-iteration.
        let queue_snapshot: Vec<Vec3<f32>> = (&colonists, &positions)
            .join()
            .map(|(_, p)| p.0)
            .collect();
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
            let Some(job) = board.jobs.get_mut(&active.job) else {
                // Cancelled out from under the colonist → re-idle.
                to_release.push(entity);
                continue;
            };
            let target = job.pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 1.0);
            match active.state {
                ActiveJobState::Traveling => {
                    // B5.8: moot-check DURING travel too — a carve stair (or
                    // any other edit) can consume the claimed block before
                    // the claimant arrives; without this the zombie job
                    // cycles claim→stuck→unreachable forever. Same predicate
                    // the completion re-validation uses.
                    let still_wanted = terrain
                        .get(job.pos)
                        .ok()
                        .is_some_and(|b| job_wanted(job.kind, b));
                    if !still_wanted {
                        info!(
                            job = active.job,
                            kind = ?job.kind,
                            pos = ?job.pos,
                            "bastion: job moot mid-travel — target block changed; dropped"
                        );
                        board.jobs.remove(&active.job);
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
                    if dist < arrive {
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
                        let steer = if job.pos.z - feet.z > reach {
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
                                .unwrap_or(target)
                        } else {
                            target
                        };
                        // R3 fix-2 (WAITING — single-file queue discipline):
                        // when staged at an anchor and ANOTHER colonist is
                        // meaningfully closer to it, WAIT — don't shove
                        // into the funnel, don't run the watchdog on the
                        // queue time. The colonist actually climbing (in
                        // or nearly in the column) never yields; promotion
                        // re-evaluates every arbitration pass.
                        if steer != target {
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
                        let sdist = pos.0.distance(steer);
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
                            if active.stuck_time > STUCK_TIMEOUT {
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
                                if !active.soft_granted {
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
                                if steer != target {
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
                    job.progress +=
                        dt.0 * work_rate(skill_level) * tool_factor(job.work, tool);
                    if job.progress < 1.0 {
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
                    let still_valid = terrain.get(job.pos).ok().is_some_and(|b| match job.kind {
                        DesignationKind::Mine => b.is_filled(),
                        DesignationKind::Chop => matches!(b.kind(), BlockKind::Wood),
                        DesignationKind::Build | DesignationKind::Ladder => !b.is_filled(),
                        DesignationKind::Stockpile => false,
                    });
                    if !still_valid {
                        info!(
                            job = active.job,
                            kind = ?job.kind,
                            pos = ?job.pos,
                            "bastion: job moot — target block changed under it; dropped"
                        );
                        board.jobs.remove(&active.job);
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
                    let new_block = match job.kind {
                        DesignationKind::Mine | DesignationKind::Chop => Block::empty(),
                        DesignationKind::Build => Block::new(BlockKind::Rock, Rgb::new(150, 150, 150)),
                        // B5.8: the native climbable ladder sprite — the
                        // vertical link pathfinding knows about.
                        DesignationKind::Ladder => Block::air(SpriteKind::Ladder),
                        DesignationKind::Stockpile => continue,
                    };
                    block_change.set(job.pos, new_block);
                    // B5.8: a player-built ladder line registers as an
                    // access anchor too (one per column — XY dedupe), so
                    // staged routing finds it.
                    if job.kind == DesignationKind::Ladder
                        && !board
                            .access_anchors
                            .iter()
                            .any(|a| a.xy().distance_squared(job.pos.xy()) < 4)
                    {
                        info!(pos = ?job.pos, "bastion: access anchor registered (built)");
                        board.access_anchors.push(job.pos);
                    }

                    if let Some(item_id) = match job.kind {
                        DesignationKind::Mine => Some(MINE_DROP_ITEM),
                        DesignationKind::Chop => Some(CHOP_DROP_ITEM),
                        DesignationKind::Build
                        | DesignationKind::Stockpile
                        | DesignationKind::Ladder => None,
                    } {
                        // B5.5: colonist output is a player resource —
                        // persistent (no despawn timer) and mergeable
                        // (`should_merge: true`), so burst mining aggregates
                        // into piles instead of carpeting the ground with
                        // one entity per block. Gentle toss (was ±2.0
                        // horizontal): drops land close, so spawn-time
                        // merging within MAX_ITEM_MERGE_DIST actually fires.
                        item_drop_emitter.emit(CreateItemDropEvent {
                            pos: comp::Pos(job.pos.map(|e| e as f32) + Vec3::broadcast(0.5)),
                            vel: comp::Vel(
                                (Vec2::unit_x()
                                    .rotated_z(rng.random::<f32>() * std::f32::consts::TAU)
                                    * 0.5)
                                    .with_z(rng.random_range(2.0..4.0)),
                            ),
                            ori: comp::Ori::default(),
                            item: PickupItem::new(
                                Item::new_from_asset_expect(item_id),
                                *program_time,
                                true,
                            ),
                            loot_owner: None,
                            persistent: true,
                        });
                    }

                    colonist.0.skills.grant_xp(job.work, COMPLETION_XP);
                    info!(
                        job = active.job,
                        kind = ?job.kind,
                        pos = ?job.pos,
                        "bastion: job completed"
                    );
                    let done_pos = job.pos;
                    board.jobs.remove(&active.job);
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
                // INSIDE AN ACTIVE DESIGNATION = a work zone, so a
                // below-grade colonist there is a digger (idle between
                // blocks, or working) — teleporting it yanks the dig
                // (b58-(d) over-fire). A TRAPPED colonist is outside any
                // designation: a chokepoint straggler sits in the
                // pre-carved chamber (not a designation), and an
                // entombed colonist's zone was DELETED — both correctly
                // still teleport. The mask distinguishes "in a work zone"
                // from "stuck in dead space" without a reachability
                // verdict; a genuinely-stuck DIGGER is demoted to jobless
                // by the churn detector (claim released) → its zone
                // reference is gone → it teleports on the next pass.
                if board.designated.iter().any(|r| r.contains_point(feet)) {
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
        let mut assignments: Vec<(specs::Entity, JobId)> = Vec::new();
        // B5.8 (DF-style dig behavior, Ben's live-test requirement):
        // 1. REACHABILITY GATE — a Mine job is claimable only when EXPOSED
        //    (≥1 of its 6 neighbors non-filled): a digger can stand next to
        //    it. Interior cells unlock as the shell clears; a fresh deep
        //    dig therefore proceeds TOP-DOWN from the surface layer instead
        //    of everyone rushing (and stalling on) the deepest corner. Side
        //    effect: carve-stair steps self-sequence (only the next step is
        //    exposed). Computed once per cycle, not per colonist.
        let mut exposed: HashSet<JobId> = HashSet::new();
        for (id, job) in board.jobs.iter_mut() {
            if job.kind != DesignationKind::Mine || job.claimed_by.is_some() {
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
            if job.kind != DesignationKind::Mine
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
            }
            // On None: the gate holds and this retries next cycle (the
            // frontier keeps digging its SAFE layers meanwhile).
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
                if job.kind == DesignationKind::Mine && !exposed.contains(id) {
                    continue;
                }
                // B5.8-E: held until return-access leads the descent.
                if descent_gated.contains(id) {
                    continue;
                }
                if job.required_item.is_some() && !carries_material {
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
                let priority = if job.is_access || job.kind == DesignationKind::Ladder {
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
                    if job.kind == DesignationKind::Mine && !job.is_access {
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
                let score = dist + depth_score + clump_penalty;
                let better = match &best {
                    None => true,
                    Some((_, bp, bs)) => priority > *bp || (priority == *bp && score < *bs),
                };
                if better {
                    best = Some((*id, priority, score));
                }
            }
            if let Some((job_id, _, _)) = best {
                if let Some(job) = board.jobs.get_mut(&job_id) {
                    job.claimed_by = Some(*uid);
                    claimed_pos.push(job.pos);
                }
                info!(job = job_id, colonist = %uid, "bastion: job claimed");
                assignments.push((entity, job_id));
            }
        }
        for (entity, job_id) in assignments {
            let _ = active_jobs.insert(entity, ActiveJob {
                job: job_id,
                state: ActiveJobState::Traveling,
                best_dist: f32::MAX,
                stuck_time: 0.0,
                reset_dist: f32::MAX,
                soft_granted: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
