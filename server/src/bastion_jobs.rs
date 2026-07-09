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
        MINE_DROP_ITEM, Region,
    },
    comp,
    comp::{
        Item,
        bastion::{ActiveJob, ActiveJobState},
        item::PickupItem,
    },
    event::CreateItemDropEvent,
    resources::{DeltaTime, ProgramTime},
    terrain::{Block, BlockKind, TerrainGrid},
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
/// speeds this up (see `work_rate`).
const WORK_DURATION_BASE: f32 = 3.0;
/// Work-rate skill bonus: +20% speed per skill level.
const WORK_SKILL_BONUS: f32 = 0.2;
/// Flat completion XP grant (design doc: "grant skill XP on completion").
const COMPLETION_XP: f32 = 8.0;

fn work_rate(skill_level: u16) -> f32 {
    (1.0 + skill_level as f32 * WORK_SKILL_BONUS) / WORK_DURATION_BASE
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

/// The job board resource.
#[derive(Default)]
pub struct JobBoard {
    next_id: JobId,
    pub jobs: HashMap<JobId, Job>,
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
                    let wanted = match kind {
                        DesignationKind::Mine => block.is_filled(),
                        DesignationKind::Chop => matches!(block.kind(), BlockKind::Wood),
                        DesignationKind::Build => !block.is_filled(),
                        // B6: stockpile zones generate their own job type.
                        DesignationKind::Stockpile => false,
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
                            progress: 0.0,
                            required_item: matches!(kind, DesignationKind::Build)
                                .then_some(BUILD_MATERIAL_ITEM),
                            needs_materials: false,
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

/// The arbitration + travel + work-execution system.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Tick>,
        Read<'a, DeltaTime>,
        Read<'a, IdMaps>,
        ReadExpect<'a, ProgramTime>,
        Write<'a, JobBoard>,
        WriteStorage<'a, comp::Colonist>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, ActiveJob>,
        // bastion (B-ASSET1): test-fixture goto orders (harness/arena).
        WriteStorage<'a, comp::bastion::BastionTestGoto>,
        WriteStorage<'a, comp::Agent>,
        WriteStorage<'a, comp::Inventory>,
        WriteExpect<'a, BlockChange>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, common::event::EventBus<CreateItemDropEvent>>,
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
            program_time,
            mut board,
            mut colonists,
            positions,
            uids,
            mut active_jobs,
            mut test_gotos,
            mut agents,
            mut inventories,
            mut block_change,
            terrain,
            item_drop_events,
        ): Self::SystemData,
    ) {
        let mut item_drop_emitter = item_drop_events.emitter();
        let mut rng = rand::rng();

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
                            "bastion: colonist arrived at job site, working (B5)"
                        );
                    } else {
                        // Keep the intent asserted (rtsim brain is gated off
                        // while ActiveJob exists, but agents clear activity
                        // on their own in places).
                        if let Some(agent) = agent {
                            agent.rtsim_controller.activity =
                                Some(common::rtsim::NpcActivity::Goto(target, TRAVEL_SPEED));
                        }
                        // Watchdog: distance-to-target must keep improving;
                        // pacing near an unreachable site doesn't count.
                        if dist + STUCK_EPSILON < active.best_dist {
                            active.best_dist = dist;
                            active.stuck_time = 0.0;
                        } else {
                            active.stuck_time += dt.0;
                            if active.stuck_time > STUCK_TIMEOUT {
                                job.claimed_by = None;
                                job.unreachable = true;
                                info!(
                                    job = active.job,
                                    pos = ?job.pos,
                                    "bastion: job unreachable — claim released"
                                );
                                to_release.push(entity);
                            }
                        }
                    }
                },
                ActiveJobState::Arrived => {
                    // B5: accumulate work, rate scaled by the relevant skill.
                    let skill_level = colonist.0.skills.level_for(job.work);
                    job.progress += dt.0 * work_rate(skill_level);
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
                        DesignationKind::Build => !b.is_filled(),
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
                    if job.kind == DesignationKind::Build {
                        let taken = inventories.get_mut(entity).and_then(|mut inv| {
                            let slot = inv.slots_with_id().find_map(|(slot, item)| {
                                item.as_ref()
                                    .is_some_and(|i| {
                                        i.item_definition_id().itemdef_id()
                                            == Some(BUILD_MATERIAL_ITEM)
                                    })
                                    .then_some(slot)
                            });
                            slot.and_then(|slot| inv.remove(slot))
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
                        DesignationKind::Stockpile => continue,
                    };
                    block_change.set(job.pos, new_block);

                    if let Some(item_id) = match job.kind {
                        DesignationKind::Mine => Some(MINE_DROP_ITEM),
                        DesignationKind::Chop => Some(CHOP_DROP_ITEM),
                        DesignationKind::Build | DesignationKind::Stockpile => None,
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
                    board.jobs.remove(&active.job);
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
            if job.kind == DesignationKind::Build && job.claimed_by.is_none() {
                job.needs_materials = !material_available;
            }
        }

        // Claims are marked on the board *during* selection (atomic within
        // the pass — two idle colonists can't pick the same job); the
        // `ActiveJob` comps are inserted afterwards (can't insert while the
        // anti-join borrows the storage).
        let mut assignments: Vec<(specs::Entity, JobId)> = Vec::new();
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
            // Highest priority, then nearest.
            let mut best: Option<(JobId, u8, f32)> = None;
            for (id, job) in board.jobs.iter() {
                if job.claimed_by.is_some() || job.unreachable {
                    continue;
                }
                if job.kind == DesignationKind::Build && !carries_material {
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
                assignments.push((entity, job_id));
            }
        }
        for (entity, job_id) in assignments {
            let _ = active_jobs.insert(entity, ActiveJob {
                job: job_id,
                state: ActiveJobState::Traveling,
                best_dist: f32::MAX,
                stuck_time: 0.0,
            });
        }
    }
}
