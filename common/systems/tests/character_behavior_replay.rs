#[cfg(test)]
mod tests {
    use common::{
        SkillSetBuilder,
        apex::{
            prediction_boundary::{PredictedFrameV1, ReplayContextV1, WorldRevisionV1},
            weather_snapshot::WeatherSnapshotIdV1,
        },
        comp::{
            Alignment, CharacterActivity, CharacterState, Controller, ControllerInputs, Energy,
            Ori, PhysicsState, Pos, Stats, Vel,
            character_state::{CharacterStateEventSinkV1, OutputEvents},
            item::MaterialStatManifest,
            tool::AbilityMap,
        },
        event::KnockbackEvent,
        event::EventBus,
        resources::{DeltaTime, GameMode, Time},
        states::idle,
        terrain::{MapSizeLg, TerrainChunk},
        uid::{IdMaps, Uid},
    };
    use common_ecs::dispatch;
    use common_state::State;
    use rand::rng;
    use specs::{Builder, Entity, Read, SystemData, WorldExt};
    use std::{num::NonZeroU64, sync::Arc};
    use vek::{Vec2, Vec3};
    use veloren_common_systems::character_behavior::{self, ReadData, RollingStateV1};

    const DEFAULT_WORLD_CHUNKS_LG: MapSizeLg =
        if let Ok(map_size_lg) = MapSizeLg::new(Vec2 { x: 1, y: 1 }) {
            map_size_lg
        } else {
            panic!("Default world chunk size does not satisfy required invariants.");
        };

    /// Mirrors `common/systems/tests/character_state.rs`'s `setup()` --
    /// same `State` construction, same resource inserts, plus one more:
    /// `EventBus<KnockbackEvent>` is inserted explicitly because
    /// `CharacterStateEvents`'s fields are ALL `Option<Read<EventBus<_>>>`
    /// (so a bare-`character_behavior::Sys`-only dispatcher setup, unlike
    /// the full game dispatcher, never auto-registers any of them) and
    /// this file's sink-vs-live comparison needs a REAL bus to prove is
    /// untouched, not an absent one.
    fn setup() -> State {
        let pools = State::pools(GameMode::Server);
        let mut state = State::new(
            GameMode::Server,
            pools,
            DEFAULT_WORLD_CHUNKS_LG,
            Arc::new(TerrainChunk::water(0)),
            |dispatch_builder| {
                dispatch::<character_behavior::Sys>(dispatch_builder, &[]);
            },
            common_state::StatePluginsV1::none(),
        )
        .expect("test State construction is legacy-mode and cannot fail");
        state.ecs_mut().insert(MaterialStatManifest::load().cloned());
        state.ecs_mut().insert(AbilityMap::load().cloned());
        state.ecs_mut().insert(EventBus::<KnockbackEvent>::default());
        state.ecs_mut().read_resource::<Time>();
        state.ecs_mut().read_resource::<DeltaTime>();
        state
    }

    /// Only the components `replay_predicted_frame_v1` actually reads
    /// from ECS storage (`Uid`/`Mass`/`Body`/`PhysicsState`/`Stats`/
    /// `SkillSet`) -- everything `RollingStateV1` carries (char_state,
    /// pos, vel, ori, density, energy) is supplied by the caller, not
    /// stored on this entity, because `JoinFieldMut::Owned`/the plain
    /// `&mut` rolling fields never touch ECS storage for those.
    fn create_entity(state: &mut State) -> (Entity, common::comp::Body) {
        let body = common::comp::Body::Humanoid(common::comp::humanoid::Body::random_with(
            &mut rng(),
            &common::comp::humanoid::Species::Human,
        ));
        let skill_set = SkillSetBuilder::default().build();
        let entity = state
            .ecs_mut()
            .create_entity()
            .with(body.mass())
            .with(body)
            .with(skill_set)
            .with(PhysicsState::default())
            .with(Stats::empty(body))
            .with(Uid(NonZeroU64::new(1).unwrap()))
            .build();
        (entity, body)
    }

    fn baseline_rolling(body: common::comp::Body) -> RollingStateV1 {
        RollingStateV1 {
            char_state: CharacterState::Idle(idle::Data::default()),
            character_activity: CharacterActivity::default(),
            pos: Pos(Vec3::zero()),
            vel: Vel::default(),
            ori: Ori::default(),
            density: body.density(),
            energy: Energy::new(body),
        }
    }

    fn frame(move_dir: Vec2<f32>) -> PredictedFrameV1 {
        let mut controller = Controller::default();
        controller.inputs = ControllerInputs {
            move_dir,
            ..Default::default()
        };
        PredictedFrameV1 {
            controller,
            dt: DeltaTime(1.0 / 30.0),
            time: Time(0.0),
            world_revision: WorldRevisionV1 {
                weather: WeatherSnapshotIdV1::from_sequence_v1(0),
                touched_chunks: Vec::new(),
            },
            alignment: common::apex::prediction_boundary::FrameAlignmentV1 {
                baseline_sync_tick: 0,
                ordinal: 0,
            },
        }
    }

    fn replay_once(state: &State, entity: Entity, rolling: &mut RollingStateV1, frame: &PredictedFrameV1) {
        let ecs = state.ecs();
        let read_data = ReadData::fetch(ecs);
        let id_maps = Read::<IdMaps>::fetch(ecs);
        let sink = CharacterStateEventSinkV1::default();
        let mut emitters = sink.emitters();
        let mut local_events = Vec::new();
        let mut output_events = OutputEvents::new(&mut local_events, &mut emitters);
        character_behavior::replay_predicted_frame_v1(
            &read_data,
            &id_maps,
            entity,
            rolling,
            frame,
            &mut output_events,
            ReplayContextV1,
        );
    }

    /// `APEX-T7.3b` acceptance, Decision-1 falsification: a
    /// `TransitionInput` field moves the replayed output, an
    /// `AmbientAccess` field does not -- proven against the REAL
    /// `replay_predicted_frame_v1` primitive and the REAL `Idle`
    /// character state, not a toy model.
    ///
    /// `move_dir` (part of `Controller`, `PREDICTION_FIELD_ROLES`
    /// classifies `controller`/`inputs` `TransitionInput`) drives
    /// `basic_move`'s `update.vel` directly (`states/utils.rs`,
    /// `data.dt.0 * accel * data.inputs.move_dir`), so a nonzero vs.
    /// zero `move_dir` MUST move the result.
    ///
    /// `alignment` (`PREDICTION_FIELD_ROLES` classifies it
    /// `AmbientAccess`) is verified, not assumed, never read by any
    /// helper `Idle::behavior()` actually calls (`handle_skating`,
    /// `handle_orientation`, `handle_move`, `handle_jump`,
    /// `handle_wield`, `handle_climb`, `handle_wallrun`,
    /// `leave_stance`) -- grepped for `data.alignment`/`data.combo`
    /// across all seven and found zero references before writing this
    /// assertion.
    #[test]
    fn input_mutation_moves_the_replayed_output_ambient_mutation_does_not() {
        let mut state = setup();
        let (entity, body) = create_entity(&mut state);

        let mut rolling_zero_move = baseline_rolling(body);
        replay_once(&state, entity, &mut rolling_zero_move, &frame(Vec2::zero()));

        let mut rolling_nonzero_move = baseline_rolling(body);
        replay_once(
            &state,
            entity,
            &mut rolling_nonzero_move,
            &frame(Vec2::new(1.0, 0.0)),
        );

        assert_ne!(
            rolling_zero_move.vel, rolling_nonzero_move.vel,
            "controller.inputs.move_dir is TransitionInput -- changing it must move the \
             replayed output"
        );

        state
            .ecs()
            .write_storage::<Alignment>()
            .insert(entity, Alignment::Wild)
            .expect("entity was just built and is alive");
        let mut rolling_with_alignment = baseline_rolling(body);
        replay_once(
            &state,
            entity,
            &mut rolling_with_alignment,
            &frame(Vec2::zero()),
        );

        assert_eq!(
            rolling_zero_move, rolling_with_alignment,
            "alignment is AmbientAccess and Idle's behavior() call chain never reads it -- \
             changing it must not move the replayed output"
        );
    }

    /// `APEX-T7.3b` acceptance, the other half Fable's ruling asked for:
    /// prove the sink captures what is routed through it, and prove the
    /// live bus a real system would drain stays untouched by the SAME
    /// call. This exercises the real `emit_server` -> `EmitExt::emit`
    /// -> `Emitter::emit` (buffer) -> `Emitter::drop` (flush)
    /// machinery `event_emitters!` generates -- the identical path a
    /// character state's `output_events.emit_server(...)` call would
    /// take -- rather than depending on a specific `Idle` sub-branch
    /// (which would need a fully-equipped `Inventory` fixture to reach
    /// its one emitting path, `attempt_swap_equipped_weapons`,
    /// disproportionate machinery for what this test is proving).
    #[test]
    fn sink_captures_what_a_replay_emits_and_the_live_bus_stays_untouched() {
        let state = setup();

        let sink = CharacterStateEventSinkV1::default();
        {
            let mut emitters = sink.emitters();
            let mut local_events = Vec::new();
            let mut output_events = OutputEvents::new(&mut local_events, &mut emitters);
            // Same call a live `character_behavior::Sys` poise-stun
            // block makes (`output_events.emit_server(KnockbackEvent {
            // .. })`) -- the only difference is which buses `emitters`
            // was constructed from.
            output_events.emit_server(KnockbackEvent {
                entity: state.ecs().entities().create(),
                impulse: Vec3::zero(),
            });
            // `emitters` (and its buffered-but-unflushed `Emitter`s)
            // drops here -- that drop is what flushes into the SINK's
            // own buses, not the live ones.
        }

        let counts = sink.drain_counts_v1();
        let knockback_count = counts
            .iter()
            .find(|(name, _)| *name == "knockback")
            .map(|(_, n)| *n)
            .unwrap_or(0);
        assert_eq!(
            knockback_count, 1,
            "the sink must have captured the one KnockbackEvent routed through its emitters"
        );
        assert!(
            counts.iter().filter(|(name, _)| *name != "knockback").all(|(_, n)| *n == 0),
            "no other channel was emitted through, none should be counted: {counts:?}"
        );

        let live_bus = state.ecs().fetch::<EventBus<KnockbackEvent>>();
        let live_count = live_bus.recv_all().count();
        assert_eq!(
            live_count, 0,
            "the live KnockbackEvent bus a real system would drain must be untouched -- the \
             emission above was routed entirely into the throwaway sink"
        );
    }
}
