use common_net::synced_components::Heads;
use specs::{
    Entities, LazyUpdate, LendJoin, Read, ReadExpect, ReadStorage, SystemData, WriteStorage, shred,
};

use common::{
    apex::prediction_boundary::{PredictedFrameV1, ReplayContextV1},
    comp::{
        self, ActiveAbilities, Beam, Body, CharacterActivity, CharacterState, Combo, Controller,
        Density, Energy, Health, Inventory, InventoryManip, Mass, Melee, Ori, PhysicsState, Poise,
        Pos, PreviousPhysCache, Scale, SkillSet, Stance, StateUpdate, Stats, Vel,
        character_state::{CharacterStateEvents, OutputEvents},
        inventory::item::{MaterialStatManifest, tool::AbilityMap},
    },
    event::{self, EventBus, KnockbackEvent, LocalEvent},
    link::Is,
    mounting::{Rider, VolumeRider},
    outcome::Outcome,
    resources::{DeltaTime, Time},
    states::{
        behavior::{JoinData, JoinFieldMut, JoinStruct},
        idle,
    },
    terrain::TerrainGrid,
    uid::{IdMaps, Uid},
};
use common_ecs::{Job, Origin, Phase, System};

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    events: CharacterStateEvents<'a>,
    local_bus: Read<'a, EventBus<LocalEvent>>,
    dt: Read<'a, DeltaTime>,
    time: Read<'a, Time>,
    lazy_update: Read<'a, LazyUpdate>,
    healths: ReadStorage<'a, Health>,
    heads: ReadStorage<'a, Heads>,
    bodies: ReadStorage<'a, Body>,
    masses: ReadStorage<'a, Mass>,
    scales: ReadStorage<'a, Scale>,
    physics_states: ReadStorage<'a, PhysicsState>,
    melee_attacks: ReadStorage<'a, Melee>,
    beams: ReadStorage<'a, Beam>,
    uids: ReadStorage<'a, Uid>,
    is_riders: ReadStorage<'a, Is<Rider>>,
    is_volume_riders: ReadStorage<'a, Is<VolumeRider>>,
    stats: ReadStorage<'a, Stats>,
    skill_sets: ReadStorage<'a, SkillSet>,
    active_abilities: ReadStorage<'a, ActiveAbilities>,
    msm: ReadExpect<'a, MaterialStatManifest>,
    ability_map: ReadExpect<'a, AbilityMap>,
    combos: ReadStorage<'a, Combo>,
    alignments: ReadStorage<'a, comp::Alignment>,
    terrain: ReadExpect<'a, TerrainGrid>,
    inventories: ReadStorage<'a, Inventory>,
    stances: ReadStorage<'a, Stance>,
    prev_phys_caches: ReadStorage<'a, PreviousPhysCache>,
    constructed_ladder_traversals: ReadStorage<'a, comp::bastion::ConstructedLadderTraversal>,
}

/// ## Character Behavior System
/// Passes `JoinData` to `CharacterState`'s `behavior` handler fn's. Receives a
/// `StateUpdate` in return and performs updates to ECS Components from that.
#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, CharacterActivity>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Density>,
        WriteStorage<'a, Energy>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, Poise>,
        Read<'a, EventBus<Outcome>>,
        Read<'a, IdMaps>,
    );

    const NAME: &'static str = "character_behavior";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            read_data,
            mut character_states,
            mut character_activities,
            mut positions,
            mut velocities,
            mut orientations,
            mut densities,
            mut energies,
            mut controllers,
            mut poises,
            outcomes,
            id_maps,
        ): Self::SystemData,
    ) {
        let mut local_emitter = read_data.local_bus.emitter();
        let mut outcomes_emitter = outcomes.emitter();
        let mut emitters = read_data.events.get_emitters();

        let mut local_events = Vec::new();
        let mut output_events = OutputEvents::new(&mut local_events, &mut emitters);

        let join = (
            &read_data.entities,
            &read_data.uids,
            &mut character_states,
            &mut character_activities,
            &mut positions,
            &mut velocities,
            &mut orientations,
            &read_data.masses,
            &mut densities,
            &mut energies,
            read_data.inventories.maybe(),
            &mut controllers,
            read_data.healths.maybe(),
            read_data.heads.maybe(),
            (
                &read_data.bodies,
                &read_data.physics_states,
                read_data.scales.maybe(),
                &read_data.stats,
                &read_data.skill_sets,
                read_data.active_abilities.maybe(),
                read_data.is_riders.maybe(),
                read_data.constructed_ladder_traversals.maybe(),
            ),
            read_data.combos.maybe(),
        )
            .lend_join();
        join.for_each(|comps| {
            let (
                entity,
                uid,
                mut char_state,
                character_activity,
                pos,
                vel,
                ori,
                mass,
                density,
                energy,
                inventory,
                controller,
                health,
                heads,
                (
                    body,
                    physics,
                    scale,
                    stat,
                    skill_set,
                    active_abilities,
                    is_rider,
                    constructed_ladder_traversal,
                ),
                combo,
            ) = comps;
            // Being dead overrides all other states
            if health.is_some_and(|h| h.is_dead) {
                // Do nothing
                return;
            }

            // Remove components that entity should not have if not in relevant char state
            if !char_state.is_melee_attack() {
                read_data.lazy_update.remove::<Melee>(entity);
            }
            if !char_state.is_beam_attack() {
                read_data.lazy_update.remove::<Beam>(entity);
            }

            // Enter stunned state if poise damage is enough
            if let Some(mut poise) = poises.get_mut(entity) {
                let was_wielded = char_state.is_wield();
                let poise_state = poise.poise_state();
                let pos = pos.0;
                if let (Some((stunned_state, stunned_duration)), impulse_strength) =
                    poise_state.poise_effect(was_wielded)
                {
                    // Reset poise if there is some stunned state to apply
                    poise.reset(*read_data.time, stunned_duration);
                    if !comp::is_downed(health, Some(&char_state)) {
                        *char_state = stunned_state;
                    }
                    outcomes_emitter.emit(Outcome::PoiseChange {
                        pos,
                        state: poise_state,
                    });
                    if let Some(impulse_strength) = impulse_strength {
                        output_events.emit_server(KnockbackEvent {
                            entity,
                            impulse: impulse_strength * *poise.knockback(),
                        });
                    }
                }
            }

            // Controller actions
            let actions = controller.take_actions();

            let mut join_struct = JoinStruct {
                entity,
                uid,
                char_state: JoinFieldMut::Live(char_state),
                character_activity: JoinFieldMut::Live(character_activity),
                pos,
                vel,
                ori,
                scale,
                mass,
                density: JoinFieldMut::Live(density),
                energy: JoinFieldMut::Live(energy),
                inventory,
                controller,
                health,
                heads,
                body,
                physics,
                melee_attack: read_data.melee_attacks.get(entity),
                beam: read_data.beams.get(entity),
                stat,
                skill_set,
                active_abilities,
                combo,
                alignment: read_data.alignments.get(entity),
                terrain: &read_data.terrain,
                mount_data: read_data.is_riders.get(entity),
                volume_mount_data: read_data.is_volume_riders.get(entity),
                stance: read_data.stances.get(entity),
                id_maps: &id_maps,
                alignments: &read_data.alignments,
                prev_phys_caches: &read_data.prev_phys_caches,
                bodies: &read_data.bodies,
                constructed_ladder_traversal,
            };

            for action in actions {
                let j = JoinData::new(
                    &join_struct,
                    &read_data.lazy_update,
                    &read_data.dt,
                    &read_data.time,
                    &read_data.msm,
                    &read_data.ability_map,
                );
                let state_update = j.character.handle_event(&j, &mut output_events, action);
                Self::publish_state_update(&mut join_struct, state_update, &mut output_events);
            }

            // Mounted occurs after control actions have been handled
            // If mounted, character state is controlled by mount
            if is_rider.is_some() && !join_struct.char_state.can_perform_mounted() {
                // TODO: A better way to swap between mount inputs and rider inputs
                *join_struct.char_state = CharacterState::Idle(idle::Data::default());
                return;
            }

            let j = JoinData::new(
                &join_struct,
                &read_data.lazy_update,
                &read_data.dt,
                &read_data.time,
                &read_data.msm,
                &read_data.ability_map,
            );

            let state_update = j.character.behavior(&j, &mut output_events);
            Self::publish_state_update(&mut join_struct, state_update, &mut output_events);
        });

        local_emitter.append_vec(local_events);
    }
}

impl Sys {
    fn publish_state_update(
        join: &mut JoinStruct,
        state_update: StateUpdate,
        output_events: &mut OutputEvents,
    ) {
        // Here we check for equality with the previous value of these components before
        // updating them so that the modification detection will not be
        // triggered unnecessarily. This is important for minimizing updates
        // sent to the clients (and thus keeping bandwidth usage down).
        //
        // TODO: if checking equality is expensive for char_state use optional field in
        // StateUpdate
        if *join.char_state != state_update.character {
            *join.char_state = state_update.character
        }
        if *join.character_activity != state_update.character_activity {
            *join.character_activity = state_update.character_activity
        }
        if *join.density != state_update.density {
            *join.density = state_update.density
        }
        if *join.energy != state_update.energy {
            *join.energy = state_update.energy;
        };

        // These components use a different type of change detection.
        *join.pos = state_update.pos;
        *join.vel = state_update.vel;
        *join.ori = state_update.ori;

        for (input, attr) in state_update.queued_inputs {
            join.controller.queued_inputs.insert(input, attr);
        }
        for input in state_update.removed_inputs {
            join.controller.queued_inputs.remove(&input);
        }
        if state_update.swap_equipped_weapons {
            output_events.emit_server(event::InventoryManipEvent(
                join.entity,
                InventoryManip::SwapEquippedWeapons,
            ));
        }
    }
}

// ---------------------------------------------------------------------
// `APEX-T7.3b` -- the replay primitive.
// ---------------------------------------------------------------------

/// `APEX-T7.3b`: the client prediction rolling state -- the subset of
/// `JoinStruct`'s fields Decision 1 classifies `TransitionInput` that
/// a replay reconstructs by replaying frames forward, as distinct from
/// `PredictedFrameV1`'s DRIVING input, which it cannot re-derive. See
/// that type's doc comment for the split; this struct is the other
/// half of it.
#[derive(Clone, Debug, PartialEq)]
pub struct RollingStateV1 {
    pub char_state: CharacterState,
    pub character_activity: CharacterActivity,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub density: Density,
    pub energy: Energy,
}

/// `APEX-T7.3b`: replay ONE predicted frame's transition off the live
/// tick, through the SAME `behavior()`/`handle_event()` dispatch
/// `Sys::run` above uses -- same code on both paths, so they cannot
/// diverge by construction. That sameness is the guarantee `T7.1`/
/// `T7.2` exist to buy; this function is what spends it.
///
/// Ambient fields are read fresh from `read_data`/`id_maps` (Decision
/// 1: ambient is never replayed from history -- it is authority at
/// call time, the same storages the live tick reads). `rolling`
/// carries the reconstructed rolling state and is updated in place
/// rather than returned, matching what replaying a frame actually IS:
/// advancing an accumulator, not producing a detached result.
/// `frame` supplies the one tick's driving input Decision 1 says a
/// replay cannot re-derive. `output_events` is caller-owned so the
/// caller can inspect a throwaway sink's counts after this returns
/// (see `CharacterStateEventSinkV1` and `WorldRevisionV1`, whose
/// `replayable_against_v1` check is the CALLER's responsibility
/// before reaching here -- this function does not re-check it).
///
/// Panics if `entity` lacks `Uid`/`Mass`/`Body`/`PhysicsState`/
/// `Stats`/`SkillSet` -- the same invariant the live join enforces by
/// construction: those five are joined un-`maybe()`'d in `Sys::run`,
/// so no live entity reaches `behavior()`/`handle_event()` without
/// them either, and a predicted entity is necessarily one that did.
///
/// Deliberately does NOT run the three live-only blocks `Sys::run`
/// wraps this same dispatch in: the dead-check, the `LazyUpdate`
/// `Melee`/`Beam` removal, and the poise-stun block. None of the
/// three is `TransitionInput`-driven (they read `Health`/`Poise`,
/// which this primitive treats as ordinary ambient reads at most, via
/// `JoinData`, never as a decision point of their own), and a replay
/// that decided "you're dead now" or applied a fresh stun would BE
/// the authority-only decision Decision 4 forbids, not a replay of
/// one.
///
/// Known, disclosed gap inherited unchanged from the live path, not
/// introduced here: `mount_data`/`volume_mount_data`/`stance` are
/// classified `TransitionInput` by `PREDICTION_FIELD_ROLES`, but
/// `Sys::run` reads all three live from `read_data` every tick rather
/// than threading them through history, and this primitive matches
/// that -- `PredictedFrameV1`'s schema does not carry them (`T7.3a`
/// is already closed). Extending it is out of this row's scope.
pub fn replay_predicted_frame_v1(
    read_data: &ReadData,
    id_maps: &Read<IdMaps>,
    entity: specs::Entity,
    rolling: &mut RollingStateV1,
    frame: &PredictedFrameV1,
    output_events: &mut OutputEvents,
    _ctx: ReplayContextV1,
) {
    let uid = read_data.uids.get(entity).expect(
        "APEX-T7.3b: replayed entity must have Uid, the same invariant the live join enforces",
    );
    let mass = read_data.masses.get(entity).expect(
        "APEX-T7.3b: replayed entity must have Mass, the same invariant the live join enforces",
    );
    let body = read_data.bodies.get(entity).expect(
        "APEX-T7.3b: replayed entity must have Body, the same invariant the live join enforces",
    );
    let physics = read_data.physics_states.get(entity).expect(
        "APEX-T7.3b: replayed entity must have PhysicsState, the same invariant the live join \
         enforces",
    );
    let stat = read_data.stats.get(entity).expect(
        "APEX-T7.3b: replayed entity must have Stats, the same invariant the live join enforces",
    );
    let skill_set = read_data.skill_sets.get(entity).expect(
        "APEX-T7.3b: replayed entity must have SkillSet, the same invariant the live join \
         enforces",
    );

    let mut controller = frame.controller.clone();
    let actions = controller.take_actions();

    let mut join_struct = JoinStruct {
        entity,
        uid,
        char_state: JoinFieldMut::Owned(&mut rolling.char_state),
        character_activity: JoinFieldMut::Owned(&mut rolling.character_activity),
        pos: &mut rolling.pos,
        vel: &mut rolling.vel,
        ori: &mut rolling.ori,
        scale: read_data.scales.get(entity),
        mass,
        density: JoinFieldMut::Owned(&mut rolling.density),
        energy: JoinFieldMut::Owned(&mut rolling.energy),
        inventory: read_data.inventories.get(entity),
        controller: &mut controller,
        health: read_data.healths.get(entity),
        heads: read_data.heads.get(entity),
        body,
        physics,
        melee_attack: read_data.melee_attacks.get(entity),
        beam: read_data.beams.get(entity),
        stat,
        skill_set,
        active_abilities: read_data.active_abilities.get(entity),
        combo: read_data.combos.get(entity),
        alignment: read_data.alignments.get(entity),
        terrain: &read_data.terrain,
        mount_data: read_data.is_riders.get(entity),
        volume_mount_data: read_data.is_volume_riders.get(entity),
        stance: read_data.stances.get(entity),
        id_maps,
        alignments: &read_data.alignments,
        prev_phys_caches: &read_data.prev_phys_caches,
        bodies: &read_data.bodies,
        constructed_ladder_traversal: read_data.constructed_ladder_traversals.get(entity),
    };

    let is_rider = join_struct.mount_data;

    for action in actions {
        let j = JoinData::new(
            &join_struct,
            &read_data.lazy_update,
            &frame.dt,
            &frame.time,
            &read_data.msm,
            &read_data.ability_map,
        );
        let state_update = j.character.handle_event(&j, output_events, action);
        Sys::publish_state_update(&mut join_struct, state_update, output_events);
    }

    // Mounted occurs after control actions have been handled, mirroring
    // `Sys::run` exactly -- `is_rider` is ambient (read at call time,
    // not from history), so this check is safe on the replay path too.
    if is_rider.is_some() && !join_struct.char_state.can_perform_mounted() {
        *join_struct.char_state = CharacterState::Idle(idle::Data::default());
        return;
    }

    let j = JoinData::new(
        &join_struct,
        &read_data.lazy_update,
        &frame.dt,
        &frame.time,
        &read_data.msm,
        &read_data.ability_map,
    );
    let state_update = j.character.behavior(&j, output_events);
    Sys::publish_state_update(&mut join_struct, state_update, output_events);
}
