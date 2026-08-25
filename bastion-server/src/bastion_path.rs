//! bastion (PATH-0, row 45): the SEQUENTIAL path scheduler — the global
//! per-tick pathfinding budget, deterministic by construction.
//!
//! WHY THIS EXISTS (the spec's change-1 determinism argument): colonist
//! searches used to run inline inside the agent system's `.par_join()`.
//! Determinism held only because searches shared no state; a budget the
//! parallel closures COMPETE for would be order-dependent under the
//! non-deterministic join. So searches for colonist job-travel
//! (`NpcActivity::Goto`) are lifted OUT: the agent tick builds configs
//! with `search_allowed: false` (holding the pre-existing Pending stance
//! when routeless) and THIS system — sequential, Uid-ordered — runs the
//! searches under [`PATH_TICK_ITER_CAP`], via the same incremental
//! `find_path`/`astar.poll` machinery, reused wholesale through
//! [`common::path::Chaser::search_step`].
//!
//! NO STARVATION BY CONSTRUCTION (the packet's load-bearing guardrail):
//! the drain is a CURSOR'D ROUND-ROBIN over the Uid-sorted candidate set
//! — each tick resumes granting after the last-served Uid and wraps, so
//! under sustained contention every requester is served within
//! ceil(total_planned_iters / CAP) ticks. Deferral is bounded; denial is
//! impossible (a request persists — it is simply the visible
//! routeless+Goto state — until granted). The scenario MEASURES this
//! (peak wait) rather than assuming it.
//!
//! SCOPE: colonist Goto searches only — the N-scaling load. Combat/flee
//! pathing (rare, latency-critical) stays inline-vanilla, as does all
//! vanilla-NPC pathing (`search_allowed: true` — behavior untouched).
//! `bastion_full_path` (TIGHTDIG, flag-OFF) is outside the budget by
//! its own rollout status. NOT the ARCH-003 determinism fix (that seam
//! — ambient rng in shared pathing code — persists sequentially and is
//! owned elsewhere); this scheduler is determinism-FRIENDLY plumbing.

use common::{
    comp::{self, Body},
    path::TraversalConfig,
    rtsim::NpcActivity,
    terrain::TerrainGrid,
    uid::Uid,
};
use common_ecs::{Job as EcsJob, Origin, Phase, System};
use specs::{Entities, Join, ReadExpect, ReadStorage, Write, WriteStorage};
use vek::Vec3;

// (moved from veloren-server sys/agent/mod.rs in the crate-split; the one
// bastion caller is below at the scheduler grant loop, the vanilla-agent
// caller re-imports from here)
/// bastion (PATH-0): THE traversal-config builder — extracted from the
/// agent tick so the sequential path scheduler builds byte-identical
/// configs (zero mirror drift; the scheduler passes the colonist's OWN
/// body/physics/scale — colonists are never mounted). `search_allowed`
/// is decided here too: a colonist whose current activity is a job-travel
/// Goto never searches inline (the scheduler owns those searches —
/// PATH-0's budget covers the N-scaling load); vanilla NPCs and
/// colonists in any other state (combat, flee) search inline exactly as
/// before.
pub fn traversal_config_for(
    scale: f32,
    moving_body: Option<&Body>,
    physics_state: &comp::PhysicsState,
    colonist: Option<&comp::Colonist>,
    goto_scheduled: bool,
    now: f64,
    // ★ ROADS: the colony's street columns (JobBoard::road_cells).
    // Colonists' searches price walk edges onto these at ROAD_FACTOR;
    // vanilla NPCs get them too — a villager also takes the road. An
    // `Arc` clone per config: a refcount, not a copy.
    roads: &std::sync::Arc<std::collections::HashSet<vek::Vec2<i32>>>,
    walls: &std::sync::Arc<std::collections::HashSet<vek::Vec2<i32>>>,
    interiors: &std::sync::Arc<std::collections::HashSet<vek::Vec2<i32>>>,
) -> TraversalConfig {
    // This controls how picky NPCs are about their pathfinding.
    // Giants are larger and so can afford to be less precise
    // when trying to move around the world
    // (especially since they would otherwise get stuck on
    // obstacles that smaller entities would not).
    let node_tolerance = scale * 1.5;
    let slow_factor = moving_body.map_or(0.0, |b| 1.0 - 1.0 / (1.0 + b.base_accel() * 0.01));
    TraversalConfig {
        node_tolerance,
        slow_factor,
        on_ground: physics_state.on_ground.is_some(),
        in_liquid: physics_state.in_liquid().is_some(),
        min_tgt_dist: scale * moving_body.map_or(1.0, |body| body.max_radius()),
        can_climb: moving_body.is_some_and(Body::can_climb),
        // bastion (B5.8): vertical reach for colony workers
        // only (vanilla NPC pathing unchanged), mapped from
        // the colonist's CLIMBING movement skill — novices
        // manage 2-block faces, level 1+ unlocks the 3-up
        // scramble edges. The skill grows with use.
        scramble_reach: match colonist {
            Some(c) if moving_body.is_some_and(Body::can_climb) => {
                2 + (c.0.skills.climbing.level.min(1) as u8)
            },
            _ => 0,
        },
        can_fly: moving_body.is_some_and(|b| b.fly_thrust().is_some()),
        vectored_propulsion: moving_body.is_some_and(|b| b.vectored_propulsion()),
        is_target_loaded: true,
        search_allowed: !goto_scheduled,
        // ★ CLIMB BANS (Ben: "infinite climb loops... try a secondary
        // route"): live (unexpired) failed-climb columns priced out of
        // this colonist's searches. Vanilla NPCs pass none.
        climb_ban: colonist
            .map(|c| {
                c.0.climb_bans
                    .iter()
                    .filter(|(_, until)| now < *until)
                    .map(|(col, _)| *col)
                    .collect()
            })
            .unwrap_or_default(),
        road_cells: std::sync::Arc::clone(roads),
        wall_margin_cells: std::sync::Arc::clone(walls),
        interior_cells: std::sync::Arc::clone(interiors),
        // V4 natural routes: colonists carry a uid-seeded shimmer so each
        // villager favours their own line through town (deterministic —
        // same colonist, same walk). Vanilla NPCs keep 0 = laser optimal.
        route_jitter_seed: colonist
            .map(|c| {
                c.0.name
                    .bytes()
                    .fold(0xcbf29ce484222325u64, |h, b| {
                        (h ^ b as u64).wrapping_mul(0x100000001b3)
                    })
                    .max(1)
            })
            .unwrap_or(0),
    }
}

/// The global per-tick pathfinding iteration cap (in `astar.poll`
/// iteration units — the same 250..750 per-call budgets `find_path`
/// already hands the poll, summed). 3000 = four worst-case (Longest)
/// searches or twelve fresh (Small) ones per tick; a single search can
/// never exceed the cap (750 < 3000), so a lone requester is always
/// granted immediately.
pub const PATH_TICK_ITER_CAP: u64 = 3000;

/// The scheduler's state + REPORTED telemetry (harness hook reads it).
#[derive(Default)]
pub struct PathScheduler {
    /// Round-robin cursor: the last-served Uid — next tick's drain
    /// starts strictly after it (wrapping), so contention rotates
    /// fairly instead of starving high Uids.
    cursor: Option<u64>,
    /// Ticks each currently-waiting requester has been deferred
    /// (removed on grant or when the need disappears). BTreeMap — every
    /// ordering in this system is STRUCTURAL (Uid-sorted), never a hash
    /// map's iteration order (the Opus determinism property (a)).
    waits: std::collections::BTreeMap<u64, u32>,
    /// Telemetry: total grants, the worst per-tick iteration spend, and
    /// the worst deferral any requester ever saw.
    pub grants_total: u64,
    pub peak_tick_iters: u64,
    pub peak_wait: u32,
    /// Last observed Goto target per requester — drift² > 2.0 between
    /// consecutive observations is EXACTLY the Chaser's astar-reset
    /// trigger (path.rs `last_search_tgt` wipe), a TGT-DRIFT event.
    /// Ben/Fable-directed 2026-07-30: promoted from env-gated-only
    /// (Sonnet's original tgt-stability test) to always-on tracking, since
    /// [`Self::drift_events_total`] needs it every run, not just under
    /// BASTION_LEGC_DIAG. Tiny (bounded by active-requester count), so
    /// making it unconditional costs nothing measurable.
    pub diag_last_tgt: std::collections::BTreeMap<u64, Vec3<f32>>,
    /// bastion (mechanism-2 friction instrument, Ben/Fable-directed,
    /// 2026-07-30): always-on TGT-DRIFT count — cheap counter, distinct
    /// from the verbose BASTION_LEGC_DIAG log line (still env-gated; the
    /// log path costs real time and would perturb timing, the counter
    /// costs an increment). NOTE (on the record per Fable's ruling): drift
    /// alone does NOT discriminate ambient friction from the failure-tail
    /// signature -- it fires at similar rates in both passing and failing
    /// runs. Sustained TIMEOUT count (`JobBoard::timeout_counts_by_pos`)
    /// is the signal; never gate anything on this field alone.
    pub drift_events_total: u64,
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainGrid>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, comp::Colonist>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, comp::Body>,
        ReadStorage<'a, comp::PhysicsState>,
        ReadStorage<'a, comp::Scale>,
        WriteStorage<'a, comp::Agent>,
        Write<'a, PathScheduler>,
        specs::Read<'a, common::resources::Time>,
        specs::Read<'a, crate::bastion_jobs::JobBoard>,
    );

    const NAME: &'static str = "bastion_path";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut EcsJob<Self>,
        (
            entities,
            terrain,
            uids,
            colonists,
            positions,
            bodies,
            physics_states,
            scales,
            mut agents,
            mut sched,
            time,
            board,
        ): Self::SystemData,
    ) {
        // Candidates: colonists whose agent is mid-Goto with a routeless
        // chaser — the request IS this visible state (pull model: no
        // queue mutation from the parallel tick, nothing to desync).
        let mut cands: Vec<(u64, specs::Entity, Vec3<f32>, Option<f32>)> =
            (&entities, &uids, &colonists, &agents)
                .join()
                .filter_map(
                    |(entity, uid, _, agent)| match agent.rtsim_controller.activity {
                        Some(NpcActivity::Goto(tgt, _)) if agent.chaser.needs_search() => Some((
                            uid.0.get(),
                            entity,
                            tgt,
                            agent.rtsim_controller.path_endpoint_tolerance(tgt),
                        )),
                        _ => None,
                    },
                )
                .collect();
        // Sonnet's tgt-stability test, ALWAYS ON since 2026-07-30 (was
        // env-gated): a drift² > 2.0 between consecutive candidate
        // observations for the same uid is the exact condition that wipes
        // the Chaser's partial A* — counted every run
        // (`drift_events_total`) so a restart storm is directly countable
        // against grant volume without needing the env var. The verbose
        // per-event LOG line stays behind BASTION_LEGC_DIAG (real time
        // cost, would perturb timing); the counter increment does not.
        let legc_diag_log = std::env::var_os("BASTION_LEGC_DIAG").is_some();
        for (uid64, _, tgt, _) in &cands {
            // Copy `prev` out to an owned value immediately -- the
            // borrow of `sched.diag_last_tgt` must end here so the
            // mutable `drift_events_total` increment below is legal.
            if let Some(prev) = sched.diag_last_tgt.get(uid64).copied() {
                let d2 = tgt.distance_squared(prev);
                if d2 > 2.0 {
                    sched.drift_events_total += 1;
                    if legc_diag_log {
                        tracing::info!(
                            uid = uid64,
                            d2,
                            from = ?prev,
                            to = ?tgt,
                            "bastion LEGC-DIAG: TGT-DRIFT (astar-reset trigger)"
                        );
                    }
                }
            }
            sched.diag_last_tgt.insert(*uid64, *tgt);
        }
        // Sweep waits for needs that disappeared (arrived/reassigned) —
        // only live requesters accrue deferral.
        sched
            .waits
            .retain(|k, _| cands.iter().any(|(id, ..)| id == k));
        if cands.is_empty() {
            return;
        }
        // Uid-sorted + cursor rotation = the deterministic round-robin.
        cands.sort_unstable_by_key(|(id, ..)| *id);
        let start = sched
            .cursor
            .map(|c| cands.partition_point(|(id, ..)| *id <= c))
            .unwrap_or(0);
        let mut used: u64 = 0;
        for i in 0..cands.len() {
            let (uid64, entity, tgt, endpoint_tolerance) = cands[(start + i) % cands.len()];
            let planned = agents
                .get(entity)
                .map(|a| a.chaser.planned_iters())
                .unwrap_or(750);
            if used + planned > PATH_TICK_ITER_CAP {
                // Deferred — bounded by the rotation, never denied.
                let w = sched.waits.entry(uid64).or_insert(0);
                *w += 1;
                let w = *w;
                sched.peak_wait = sched.peak_wait.max(w);
                continue;
            }
            let (Some(pos), Some(phys)) = (positions.get(entity), physics_states.get(entity))
            else {
                continue;
            };
            let mut cfg = traversal_config_for(
                scales.get(entity).map_or(1.0, |s| s.0),
                bodies.get(entity),
                phys,
                colonists.get(entity),
                // The scheduler IS the search context; the flag only
                // gates inline chase searches.
                false,
                time.0,
                &board.road_cells,
                &board.wall_margin_cells,
                &board.interior_cells,
            );
            if let Some(endpoint_tolerance) = endpoint_tolerance {
                cfg.node_tolerance = cfg.node_tolerance.min(endpoint_tolerance);
            }
            if let Some(agent) = agents.get_mut(entity) {
                agent.chaser.search_step(&*terrain, pos.0, tgt, &cfg);
                // bastion ledger #180: debit ACTUAL expansions, not the
                // planned estimate — a trivial search no longer eats a
                // 250-iter slot, so more colonists are served per tick.
                // Admission (above) still projects with `planned`, and
                // actual <= planned per step, so the cap holds.
                used += agent.chaser.last_search_consumed().min(planned);
            } else {
                used += planned;
            }
            sched.grants_total += 1;
            sched.cursor = Some(uid64);
            sched.waits.remove(&uid64);
        }
        sched.peak_tick_iters = sched.peak_tick_iters.max(used);
    }
}
