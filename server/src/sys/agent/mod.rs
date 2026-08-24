pub mod behavior_tree;
use server_agent::data::AgentEvents;
pub use server_agent::{action_nodes, attack, consts, data, util};

use crate::{
    Settings, Tick,
    sys::agent::{
        behavior_tree::{BehaviorData, BehaviorTree},
        data::{AgentData, ReadData},
    },
};
use common::{
    comp::{
        self, Agent, Alignment, CharacterState, Controller, Health, Scale,
        inventory::slot::EquipSlot, item::ItemDesc,
    },
    mounting::Volume,
};
use common_base::prof_span;
use common_ecs::{Job, Origin, ParMode, Phase, System};
use rand::{RngExt, SeedableRng, rng};
// RNG-DEEP-004/007 (determinism audit): ChaCha8Rng replaces StdRng/SmallRng —
// both are explicitly NON-portable (algorithm may change across rand versions
// / platforms), so agent behavior + helper streams could diverge
// cross-machine even with identical seeds. ChaCha8 is a named, portable,
// version-stable generator.
use rand_chacha::ChaCha8Rng;
use rayon::iter::ParallelIterator;
use specs::{LendJoin, ParJoin, Read, ReadExpect, ReadStorage, WriteStorage};
use std::cell::RefCell;

fn deterministic_agent_seed(world_seed: u32, tick: u64, uid: common::uid::Uid) -> u64 {
    // SplitMix64 finalizer over the three stable identities. This is stream
    // separation, not simulation randomness: identical (world,tick,entity)
    // inputs reproduce, while neighboring seeds and different agents do not
    // share streams.
    let mut x = (world_seed as u64).rotate_left(32)
        ^ tick.rotate_left(17)
        ^ uid.0.get()
        ^ 0xA6E3_7D91_5B4C_2F08;
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

// bastion (PATH-0): `traversal_config_for` moved into the `bastion-server`
// leaf crate (crate-split) — re-imported here so call sites are unchanged.
pub(crate) use crate::bastion_path::traversal_config_for;

/// This system will allow NPCs to modify their controller
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        AgentEvents<'a>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, common::comp::bastion::BastionTraversalOwnership>,
        ReadExpect<'a, common_state::ExecutionMode>,
        Read<'a, Tick>,
        ReadExpect<'a, Settings>,
        // ★ ROADS: the colony street set for traversal_config_for.
        Read<'a, bastion_server::bastion_jobs::JobBoard>,
    );

    const NAME: &'static str = "agent";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (
            read_data,
            events,
            mut agents,
            mut controllers,
            traversal_ownerships,
            execution_mode,
            tick,
            settings,
            bastion_board,
        ): Self::SystemData,
    ) {
        job.cpu_stats.measure(ParMode::Rayon);

        (
            &read_data.entities,
            (
                &read_data.energies,
                read_data.healths.maybe(),
                read_data.combos.maybe(),
            ),
            (
                &read_data.positions,
                &read_data.velocities,
                &read_data.orientations,
            ),
            read_data.bodies.maybe(),
            &read_data.inventories,
            (
                &read_data.char_states,
                &read_data.skill_set,
                &read_data.active_abilities,
            ),
            &read_data.physics_states,
            &read_data.uids,
            &mut agents,
            &mut controllers,
            read_data.light_emitter.maybe(),
            read_data.groups.maybe(),
            read_data.rtsim_entities.maybe(),
            (
                !&read_data.is_mounts,
                read_data.is_riders.maybe(),
                read_data.is_volume_riders.maybe(),
            ),
            traversal_ownerships.maybe(),
        )
            .par_join()
            .for_each_init(
                || {
                    prof_span!(guard, "agent rayon job");
                    guard
                },
                |_guard,
                 (
                    entity,
                    (energy, health, combo),
                    (pos, vel, ori),
                    body,
                    inventory,
                    (char_state, skill_set, active_abilities),
                    physics_state,
                    uid,
                    agent,
                    controller,
                    light_emitter,
                    group,
                    rtsim_entity,
                    (_, is_rider, is_volume_rider),
                    traversal_ownership,
                )| {
                    let mut emitters = events.get_emitters();
                    let deterministic_seed = execution_mode
                        .is_deterministic()
                        .then(|| deterministic_agent_seed(settings.world_seed, tick.0, *uid));
                    // Chaser owns hidden route state and historically drew
                    // directly from `rand::rng()` when unsticking. Give it a
                    // separate deterministic stream in harness mode so an
                    // otherwise invisible route-index mutation cannot split
                    // two same-seed runs. Live mode explicitly retains the
                    // original OS-seeded path.
                    agent.chaser.set_deterministic_seed(
                        deterministic_seed.map(|seed| seed ^ 0xC4A5_E211_0B71_5EED),
                    );
                    let mut rng = if let Some(seed) = deterministic_seed {
                        ChaCha8Rng::seed_from_u64(seed)
                    } else {
                        // Preserve live entropy. Only the deterministic
                        // harness derives an agent stream from stable inputs.
                        ChaCha8Rng::from_rng(&mut rng())
                    };

                    // The entity that is moving, if riding it's the mount, otherwise it's itself
                    let moving_entity = is_rider
                        .and_then(|is_rider| {
                            let mut mount = is_rider.mount;
                            // Find the root mount, i.e the one that's doing the moving.
                            loop {
                                let e = read_data.id_maps.uid_entity(mount)?;

                                if let Some(is_rider) = read_data.is_riders.get(e) {
                                    mount = is_rider.mount;
                                } else {
                                    return Some(e);
                                }
                            }
                        })
                        .or_else(|| {
                            is_volume_rider.and_then(|is_volume_rider| {
                                match is_volume_rider.pos.kind {
                                    Volume::Terrain => None,
                                    Volume::Entity(uid) => read_data.id_maps.uid_entity(uid),
                                }
                            })
                        })
                        .unwrap_or(entity);

                    let pos = read_data.positions.get(moving_entity).unwrap_or(pos);
                    let vel = read_data.velocities.get(moving_entity).unwrap_or(vel);
                    let moving_body = read_data.bodies.get(moving_entity);
                    let physics_state = read_data
                        .physics_states
                        .get(moving_entity)
                        .unwrap_or(physics_state);
                    let goto_writer_diag = std::env::var("BASTION_GOTO_WRITER_DIAG_UID")
                        .ok()
                        .and_then(|value| value.parse::<u64>().ok())
                        == Some(uid.0.get());

                    if crate::bastion_flight_recorder::enabled() {
                        let target = match agent.rtsim_controller.activity {
                            Some(common::rtsim::NpcActivity::Goto(target, _)) => {
                                Some([target.x, target.y, target.z])
                            },
                            _ => None,
                        };
                        crate::bastion_flight_recorder::record_writer(
                            crate::bastion_flight_recorder::WriterEvent {
                                schema: "bastion.flight-recorder.event/v1".into(),
                                tick: tick.0,
                                uid: uid.0.get(),
                                observation_sequence: 100,
                                snapshot_stage: "agent-system-pre-behavior-snapshot".into(),
                                dispatcher_dependency_proven: false,
                                writer: "agent_system_before_reset".into(),
                                move_dir: [
                                    controller.inputs.move_dir.x,
                                    controller.inputs.move_dir.y,
                                ],
                                move_z: controller.inputs.move_z,
                                target,
                                note: "controller input entering authoritative Agent behavior"
                                    .into(),
                            },
                        );
                    }

                    if goto_writer_diag {
                        tracing::info!(
                            tick = tick.0,
                            uid = uid.0.get(),
                            position = ?pos.0,
                            velocity = ?vel.0,
                            activity = ?agent.rtsim_controller.activity,
                            controller_move_dir = ?controller.inputs.move_dir,
                            controller_move_z = controller.inputs.move_z,
                            chaser_last_target = ?agent.chaser.last_target(),
                            chaser_route_target = ?agent.chaser.route_target(),
                            chaser_route_complete = ?agent.chaser.route_is_complete(),
                            chaser_state = ?agent.chaser.state(),
                            writer = "agent_system_before_reset",
                            explicit_agent_to_bastion_jobs_dependency = false,
                            "bastion: agent system writer snapshot"
                        );
                    }

                    // Stage-1 B5.8: after the validated approach, the route
                    // task is the only movement-intent owner. Do not reset or
                    // run the behavior tree here: both are Controller writes
                    // that would compete with the link task. Character
                    // behavior and physics still consume the task's existing
                    // Controller intent later in the authoritative stack.
                    if traversal_ownership
                        .is_some_and(|ownership| ownership.mode.owns_movement_intent())
                    {
                        if crate::bastion_flight_recorder::enabled() {
                            crate::bastion_flight_recorder::record_writer(
                                crate::bastion_flight_recorder::WriterEvent {
                                    schema: "bastion.flight-recorder.event/v1".into(),
                                    tick: tick.0,
                                    uid: uid.0.get(),
                                    observation_sequence: 105,
                                    snapshot_stage: "agent-link-owner-exclusion".into(),
                                    dispatcher_dependency_proven: false,
                                    writer: "bastion_traversal_task".into(),
                                    move_dir: [
                                        controller.inputs.move_dir.x,
                                        controller.inputs.move_dir.y,
                                    ],
                                    move_z: controller.inputs.move_z,
                                    target: None,
                                    note: format!(
                                        "agent deferred to link_id={} mode={:?}",
                                        traversal_ownership.unwrap().link_id,
                                        traversal_ownership.unwrap().mode
                                    ),
                                },
                            );
                        }
                        return;
                    }

                    // Hack, replace with better system when groups are more sophisticated
                    // Override alignment if in a group unless entity is owned already
                    let alignment = if matches!(
                        &read_data.alignments.get(entity),
                        &Some(Alignment::Owned(_))
                    ) {
                        read_data.alignments.get(entity).copied()
                    } else {
                        group
                            .and_then(|g| read_data.group_manager.group_info(*g))
                            .and_then(|info| read_data.uids.get(info.leader))
                            .copied()
                            .map_or_else(
                                || read_data.alignments.get(entity).copied(),
                                |uid| Some(Alignment::Owned(uid)),
                            )
                    };

                    if !matches!(
                        char_state,
                        CharacterState::LeapMelee(_) | CharacterState::Glide(_)
                    ) {
                        // Default to looking in orientation direction
                        // (can be overridden below)
                        //
                        // This definitely breaks LeapMelee, Glide and
                        // probably not only that, do we really need this at all?
                        controller.reset();
                        controller.inputs.look_dir = ori.look_dir();
                    }

                    let scale = read_data
                        .scales
                        .get(moving_entity)
                        .map_or(1.0, |Scale(s)| *s);

                    let glider_equipped = inventory
                        .equipped(EquipSlot::Glider)
                        .as_ref()
                        .is_some_and(|item| matches!(&*item.kind(), comp::item::ItemKind::Glider));

                    let is_gliding = matches!(
                        read_data.char_states.get(entity),
                        Some(CharacterState::GlideWield(_) | CharacterState::Glide(_))
                    ) && physics_state.on_ground.is_none();

                    // bastion (PATH-0): built by THE shared builder (the
                    // sequential path scheduler uses the same fn — zero
                    // mirror drift). A colonist mid-Goto is SCHEDULED:
                    // its searches run in the budgeted sequential system,
                    // never inline here.
                    let goto_scheduled = read_data.colonists.get(entity).is_some()
                        && matches!(
                            agent.rtsim_controller.activity,
                            Some(common::rtsim::NpcActivity::Goto(..))
                        );
                    let traversal_config = traversal_config_for(
                        scale,
                        moving_body,
                        physics_state,
                        read_data.colonists.get(entity),
                        goto_scheduled,
                        read_data.time.0,
                        &bastion_board.road_cells,
                    );
                    let health_fraction = health.map_or(1.0, Health::fraction);

                    // Package all this agent's data into a convenient struct
                    let data = AgentData {
                        entity: &entity,
                        rtsim_entity,
                        uid,
                        pos,
                        vel,
                        ori,
                        energy,
                        body,
                        inventory,
                        skill_set,
                        physics_state,
                        alignment: alignment.as_ref(),
                        traversal_config,
                        scale,
                        damage: health_fraction,
                        light_emitter,
                        glider_equipped,
                        is_gliding,
                        health: read_data.healths.get(entity),
                        heads: read_data.heads.get(entity),
                        char_state,
                        active_abilities,
                        combo,
                        buffs: read_data.buffs.get(entity),
                        stats: read_data.stats.get(entity),
                        cached_spatial_grid: &read_data.cached_spatial_grid,
                        msm: &read_data.msm,
                        poise: read_data.poises.get(entity),
                        stance: read_data.stances.get(entity),
                        helper_rng: RefCell::new(
                            deterministic_seed
                                .map(|seed| ChaCha8Rng::seed_from_u64(seed ^ 0x51A7_C0DE_55AA_7711)),
                        ),
                    };

                    ///////////////////////////////////////////////////////////
                    // Behavior tree
                    ///////////////////////////////////////////////////////////
                    // The behavior tree is meant to make decisions for agents
                    // *but should not* mutate any data (only action nodes
                    // should do that). Each path should lead to one (and only
                    // one) action node. This makes bugfinding much easier and
                    // debugging way easier. If you don't think so, try
                    // debugging the agent code before this MR
                    // (https://gitlab.com/veloren/veloren/-/merge_requests/1801).
                    // Each tick should arrive at one (1) action node which
                    // then determines what the agent does. If this makes you
                    // uncomfortable, consider dt the response time of the
                    // NPC. To make the tree easier to read, subtrees can be
                    // created as methods on `AgentData`. Action nodes are
                    // also methods on the `AgentData` struct. Action nodes
                    // are the only parts of this tree that should provide
                    // inputs.
                    let mut behavior_data = BehaviorData {
                        agent,
                        agent_data: data,
                        read_data: &read_data,
                        emitters: &mut emitters,
                        controller,
                        rng: &mut rng,
                    };

                    BehaviorTree::root().run(&mut behavior_data);

                    if crate::bastion_flight_recorder::enabled() {
                        let target = match behavior_data.agent.rtsim_controller.activity {
                            Some(common::rtsim::NpcActivity::Goto(target, _)) => {
                                Some([target.x, target.y, target.z])
                            },
                            _ => None,
                        };
                        crate::bastion_flight_recorder::record_writer(
                            crate::bastion_flight_recorder::WriterEvent {
                                schema: "bastion.flight-recorder.event/v1".into(),
                                tick: tick.0,
                                uid: uid.0.get(),
                                observation_sequence: 110,
                                snapshot_stage: "agent-system-post-behavior-snapshot".into(),
                                dispatcher_dependency_proven: false,
                                writer: "agent_system_after_behavior_tree".into(),
                                move_dir: [
                                    behavior_data.controller.inputs.move_dir.x,
                                    behavior_data.controller.inputs.move_dir.y,
                                ],
                                move_z: behavior_data.controller.inputs.move_z,
                                target,
                                note: "authoritative Agent/Chaser output before later systems"
                                    .into(),
                            },
                        );
                    }

                    if goto_writer_diag {
                        tracing::info!(
                            tick = tick.0,
                            uid = uid.0.get(),
                            position = ?behavior_data.agent_data.pos.0,
                            velocity = ?behavior_data.agent_data.vel.0,
                            activity = ?behavior_data.agent.rtsim_controller.activity,
                            controller_move_dir = ?behavior_data.controller.inputs.move_dir,
                            controller_move_z = behavior_data.controller.inputs.move_z,
                            chaser_last_target = ?behavior_data.agent.chaser.last_target(),
                            chaser_route_target = ?behavior_data.agent.chaser.route_target(),
                            chaser_route_complete = ?behavior_data.agent.chaser.route_is_complete(),
                            chaser_state = ?behavior_data.agent.chaser.state(),
                            writer = "agent_system_after_behavior_tree",
                            explicit_agent_to_bastion_jobs_dependency = false,
                            "bastion: agent system writer snapshot"
                        );
                    }

                    debug_assert!(controller.inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
                    debug_assert!(controller.inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
                },
            );
    }
}
