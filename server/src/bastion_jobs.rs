//! bastion (B4): the job board — designations become jobs, colonists claim
//! jobs by priority/skill/distance and walk to them. No work *effects* yet
//! (B5); the loop ends at "arrived at job site, ready to work".
//!
//! Design (see design doc §B4 + `docs/BASTION_B4_FINDINGS.md`):
//! - [`JobBoard`] is a server ECS resource; jobs are block-level tasks
//!   generated from painted designations (`common::bastion::Region`).
//! - [`Sys`] runs every tick for travel upkeep and every
//!   [`ARBITRATION_INTERVAL`] ticks for claim arbitration.
//! - Colonist movement reuses the loaded-agent rtsim intent path: the system
//!   writes `NpcActivity::Goto` into `Agent::rtsim_controller`, which the
//!   vanilla behavior tree executes with real traversal. While a colonist has
//!   an [`comp::bastion::ActiveJob`], the rtsim brain's activity is *not*
//!   synced over it (gate in `rtsim::tick`).

use crate::Tick;
use common::{
    bastion::{DesignationKind, Job, JobAudit, JobId, Region},
    comp,
    comp::bastion::{ActiveJob, ActiveJobState},
    resources::DeltaTime,
    terrain::TerrainGrid,
    uid::{IdMaps, Uid},
    vol::ReadVol,
};
use common_ecs::{Job as EcsJob, Origin, Phase, System};
use hashbrown::{HashMap, HashSet};
use specs::{Entities, Join, LendJoin, Read, ReadStorage, Write, WriteStorage};
use tracing::info;
use vek::*;

/// Arbitration cadence in server ticks (~0.5s at 30 tps): "a few Hz, not
/// every tick".
pub const ARBITRATION_INTERVAL: u64 = 15;
/// A colonist counts as arrived within this XY distance of the job block.
const ARRIVE_DIST: f32 = 2.5;
/// Travel watchdog: release + mark unreachable after this long without
/// progress (seconds).
const STUCK_TIMEOUT: f32 = 10.0;
/// Progress threshold per watchdog sample (blocks).
const STUCK_EPSILON: f32 = 0.5;
/// Walk speed factor for job travel.
const TRAVEL_SPEED: f32 = 0.8;

/// Chunks the harness (and future scenario tooling) forces to stay loaded —
/// the server unload sweep skips them. Empty in normal play.
#[derive(Default)]
pub struct BastionForceLoaded(pub HashSet<Vec2<i32>>);

/// The job board resource.
#[derive(Default)]
pub struct JobBoard {
    next_id: JobId,
    pub jobs: HashMap<JobId, Job>,
}

impl JobBoard {
    /// Generate jobs for a validated designation region. Returns created ids.
    /// v1 generation: Mine = every filled block in the region; Chop = every
    /// wood block; Build/Stockpile = none yet (B5 blueprints / B6 zones).
    pub fn place_designation(
        &mut self,
        terrain: &TerrainGrid,
        region: Region,
        kind: DesignationKind,
    ) -> Vec<JobId> {
        let mut created = Vec::new();
        let work = kind.work_type();
        for z in region.min.z..=region.max.z {
            for y in region.min.y..=region.max.y {
                for x in region.min.x..=region.max.x {
                    let pos = Vec3::new(x, y, z);
                    let Ok(block) = terrain.get(pos) else {
                        continue;
                    };
                    let wanted = match kind {
                        DesignationKind::Mine => block.is_filled(),
                        DesignationKind::Chop => matches!(
                            block.kind(),
                            common::terrain::BlockKind::Wood
                        ),
                        // B5/B6: blueprints and stockpile zones generate
                        // their own job types; nothing to do yet.
                        DesignationKind::Build | DesignationKind::Stockpile => false,
                    };
                    if wanted {
                        let id = self.next_id;
                        self.next_id += 1;
                        self.jobs.insert(id, Job {
                            kind,
                            work,
                            pos,
                            skill_floor: 0,
                            claimed_by: None,
                            unreachable: false,
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

    /// Cancel all jobs inside a region. Returns the uids whose claims were
    /// released (their `ActiveJob` comps are cleared by the system within
    /// one cycle because the job id no longer exists).
    pub fn cancel_region(&mut self, region: Region) -> Vec<Uid> {
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

/// The arbitration + travel system.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Tick>,
        Read<'a, DeltaTime>,
        Read<'a, IdMaps>,
        Write<'a, JobBoard>,
        ReadStorage<'a, comp::Colonist>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, ActiveJob>,
        WriteStorage<'a, comp::Agent>,
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
            id_maps,
            mut board,
            colonists,
            positions,
            uids,
            mut active_jobs,
            mut agents,
        ): Self::SystemData,
    ) {
        // ── Travel upkeep (every tick) ──────────────────────────────────
        let mut to_release: Vec<(specs::Entity, Option<JobId>)> = Vec::new();
        for (entity, _colonist, pos, active, agent) in (
            &entities,
            &colonists,
            &positions,
            &mut active_jobs,
            (&mut agents).maybe(),
        )
            .join()
        {
            let Some(job) = board.jobs.get_mut(&active.job) else {
                // Cancelled out from under the colonist → re-idle.
                to_release.push((entity, None));
                continue;
            };
            let target = job.pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 1.0);
            match active.state {
                ActiveJobState::Traveling => {
                    // 3D distance: standing on the surface above a deep job
                    // must NOT count as arrival (the watchdog handles it).
                    let dist = pos.0.distance(target);
                    if dist < ARRIVE_DIST {
                        active.state = ActiveJobState::Arrived;
                        if let Some(agent) = agent {
                            agent.rtsim_controller.activity = None;
                        }
                        info!(
                            job = active.job,
                            pos = ?job.pos,
                            "bastion: colonist arrived at job site, ready to work (B5)"
                        );
                    } else {
                        // Keep the intent asserted (rtsim brain is gated off
                        // while ActiveJob exists, but agents clear activity
                        // on their own in places).
                        if let Some(agent) = agent {
                            agent.rtsim_controller.activity =
                                Some(common::rtsim::NpcActivity::Goto(target, TRAVEL_SPEED));
                        }
                        // Watchdog: no progress for too long → unreachable.
                        if pos.0.distance_squared(active.last_pos) < STUCK_EPSILON.powi(2) {
                            active.stuck_time += dt.0;
                            if active.stuck_time > STUCK_TIMEOUT {
                                job.claimed_by = None;
                                job.unreachable = true;
                                info!(
                                    job = active.job,
                                    pos = ?job.pos,
                                    "bastion: job unreachable — claim released"
                                );
                                to_release.push((entity, None));
                            }
                        } else {
                            active.last_pos = pos.0;
                            active.stuck_time = 0.0;
                        }
                    }
                },
                ActiveJobState::Arrived => {
                    // B5 hooks work execution here; for now hold position.
                },
            }
        }
        for (entity, _) in &to_release {
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

        // ── Arbitration (every ARBITRATION_INTERVAL ticks) ──────────────
        if tick.0 % ARBITRATION_INTERVAL != 0 {
            return;
        }
        // Claims are marked on the board *during* selection (atomic within
        // the pass — two idle colonists can't pick the same job); the
        // `ActiveJob` comps are inserted afterwards (can't insert while the
        // anti-join borrows the storage).
        let mut assignments: Vec<(specs::Entity, JobId, Vec3<f32>)> = Vec::new();
        for (entity, colonist, pos, uid, ()) in (
            &entities,
            &colonists,
            &positions,
            &uids,
            !&active_jobs,
        )
            .join()
        {
            // Highest priority, then nearest.
            let mut best: Option<(JobId, u8, f32)> = None;
            for (id, job) in board.jobs.iter() {
                if job.claimed_by.is_some() || job.unreachable {
                    continue;
                }
                let priority = colonist.0.work_priorities.get(job.work);
                if priority == 0 {
                    continue;
                }
                let dist = pos.0.distance(job.pos.map(|e| e as f32));
                let better = match &best {
                    None => true,
                    Some((_, bp, bd)) => priority > *bp || (priority == *bp && dist < *bd),
                };
                if better {
                    best = Some((*id, priority, dist));
                }
            }
            if let Some((job_id, _, _)) = best {
                if let Some(job) = board.jobs.get_mut(&job_id) {
                    job.claimed_by = Some(*uid);
                }
                info!(job = job_id, colonist = %uid, "bastion: job claimed");
                assignments.push((entity, job_id, pos.0));
            }
        }
        for (entity, job_id, pos) in assignments {
            let _ = active_jobs.insert(entity, ActiveJob {
                job: job_id,
                state: ActiveJobState::Traveling,
                last_pos: pos,
                stuck_time: 0.0,
            });
        }
    }
}
