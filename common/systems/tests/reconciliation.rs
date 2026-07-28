#[cfg(test)]
mod tests {
    use common::{
        SkillSetBuilder,
        apex::{
            prediction_boundary::{
                ClientPredictionBufferV1, FrameAlignmentV1, PredictedFrameV1, WorldRevisionV1,
            },
            reconciliation_metric::{POS_TOLERANCE_V1, VEL_TOLERANCE_V1},
            weather_snapshot::WeatherSnapshotIdV1,
        },
        comp::{
            CharacterActivity, CharacterState, Controller, ControllerInputs, Energy, Ori,
            PhysicsState, Pos, Stats, Vel,
            item::MaterialStatManifest,
            tool::AbilityMap,
        },
        event::{EventBus, KnockbackEvent},
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
    use veloren_common_systems::{
        character_behavior::{self, ReadData, RollingStateV1},
        reconciliation::{ReconciliationOutcomeV1, reconcile_v1},
    };

    const DEFAULT_WORLD_CHUNKS_LG: MapSizeLg =
        if let Ok(map_size_lg) = MapSizeLg::new(Vec2 { x: 1, y: 1 }) {
            map_size_lg
        } else {
            panic!("Default world chunk size does not satisfy required invariants.");
        };

    /// Same shape as `character_behavior_replay.rs`'s `setup()`.
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

    fn frame(move_dir: Vec2<f32>, baseline_sync_tick: u64, ordinal: u64) -> PredictedFrameV1 {
        let mut controller = Controller::default();
        controller.inputs = ControllerInputs { move_dir, ..Default::default() };
        PredictedFrameV1 {
            controller,
            dt: DeltaTime(1.0 / 30.0),
            time: Time(0.0),
            world_revision: WorldRevisionV1 {
                weather: WeatherSnapshotIdV1::from_sequence_v1(0),
                touched_chunks: Vec::new(),
            },
            alignment: FrameAlignmentV1 { baseline_sync_tick, ordinal },
        }
    }

    /// `APEX-T7.3c-ii` acceptance: a sub-tolerance perturbation between
    /// the client's current belief and the authoritative snapshot does
    /// NOT fire a replay -- the damping proven through `reconcile_v1`
    /// itself, not just the metric it calls.
    #[test]
    fn sub_tolerance_perturbation_agrees_and_only_trims() {
        let mut state = setup();
        let (entity, body) = create_entity(&mut state);
        let ecs = state.ecs();
        let read_data = ReadData::fetch(ecs);
        let id_maps = Read::<IdMaps>::fetch(ecs);

        let mut buffer = ClientPredictionBufferV1::new(16, 64 * 1024);
        // Acknowledged by CompSync(10): baseline 5 < 10.
        buffer.push_v1(frame(Vec2::zero(), 5, 0));
        // Still unacknowledged: baseline 10 >= 10.
        buffer.push_v1(frame(Vec2::zero(), 10, 1));

        let current = baseline_rolling(body);
        let mut authoritative = baseline_rolling(body);
        authoritative.vel.0.x += VEL_TOLERANCE_V1 * 0.5;

        let outcome = reconcile_v1(
            &read_data,
            &id_maps,
            entity,
            &mut buffer,
            &current,
            &authoritative,
            10,
            |_| true,
            |_| true,
        );

        match outcome {
            ReconciliationOutcomeV1::Agreed { trimmed } => assert_eq!(trimmed, 1),
            other => panic!("expected Agreed, got {other:?}"),
        }
        assert_eq!(buffer.len(), 1, "only the unacknowledged frame should survive the trim");
    }

    /// `APEX-T7.3c-ii` acceptance: a supra-tolerance divergence, with
    /// every unacknowledged frame replayable, fires a replay -- not a
    /// snap.
    #[test]
    fn supra_tolerance_with_valid_world_revision_replays() {
        let mut state = setup();
        let (entity, body) = create_entity(&mut state);
        let ecs = state.ecs();
        let read_data = ReadData::fetch(ecs);
        let id_maps = Read::<IdMaps>::fetch(ecs);

        let mut buffer = ClientPredictionBufferV1::new(16, 64 * 1024);
        buffer.push_v1(frame(Vec2::zero(), 10, 0));

        let current = baseline_rolling(body);
        let mut authoritative = baseline_rolling(body);
        authoritative.pos.0.x += POS_TOLERANCE_V1 * 2.0;

        let outcome = reconcile_v1(
            &read_data,
            &id_maps,
            entity,
            &mut buffer,
            &current,
            &authoritative,
            10,
            |_| true,
            |_| true,
        );

        match outcome {
            ReconciliationOutcomeV1::Replayed { trimmed, replayed, .. } => {
                assert_eq!(trimmed, 0);
                assert_eq!(replayed, 1);
            },
            other => panic!("expected Replayed, got {other:?}"),
        }
    }

    /// `APEX-T7.3c-ii` acceptance: a supra-tolerance divergence where an
    /// unacknowledged frame is NOT replayable (its weather snapshot is
    /// gone) snaps instead of replaying -- the buffer is cleared rather
    /// than left holding frames that can never legitimately replay.
    #[test]
    fn supra_tolerance_with_gone_weather_snapshot_snaps() {
        let mut state = setup();
        let (entity, body) = create_entity(&mut state);
        let ecs = state.ecs();
        let read_data = ReadData::fetch(ecs);
        let id_maps = Read::<IdMaps>::fetch(ecs);

        let mut buffer = ClientPredictionBufferV1::new(16, 64 * 1024);
        buffer.push_v1(frame(Vec2::zero(), 10, 0));

        let current = baseline_rolling(body);
        let mut authoritative = baseline_rolling(body);
        authoritative.pos.0.x += POS_TOLERANCE_V1 * 2.0;

        let outcome = reconcile_v1(
            &read_data,
            &id_maps,
            entity,
            &mut buffer,
            &current,
            &authoritative,
            10,
            |_| true,
            |_| false, // no weather snapshot is retained
        );

        match outcome {
            ReconciliationOutcomeV1::Snapped { trimmed, .. } => assert_eq!(trimmed, 0),
            other => panic!("expected Snapped, got {other:?}"),
        }
        assert!(buffer.is_empty(), "a snap clears the buffer entirely");
    }

    /// `APEX-T7.3c-ii` acceptance, the live-path form of T3.6's
    /// "corrected-away predictions cannot replay" (physics_generation.rs
    /// tests it at the generation level; this tests it at the
    /// baseline-stamping level `reconcile_v1` actually uses). An
    /// acknowledged-by-implication frame (baseline < sync_tick) with a
    /// LARGE `move_dir` sits alongside an unacknowledged frame with a
    /// ZERO `move_dir`. If the old frame were replayed, `basic_move`
    /// would move `vel.x` by a large, easily-distinguished amount; if it
    /// is correctly excluded, only the zero-effect frame replays and
    /// `vel.x` stays close to the authoritative baseline.
    #[test]
    fn corrected_away_history_cannot_replay_on_the_live_path() {
        let mut state = setup();
        let (entity, body) = create_entity(&mut state);
        let ecs = state.ecs();
        let read_data = ReadData::fetch(ecs);
        let id_maps = Read::<IdMaps>::fetch(ecs);

        let mut buffer = ClientPredictionBufferV1::new(16, 64 * 1024);
        // Acknowledged by CompSync(10): would move vel.x by a lot if it
        // were (wrongly) replayed.
        buffer.push_v1(frame(Vec2::new(1.0, 0.0), 5, 0));
        // Unacknowledged: zero-effect input.
        buffer.push_v1(frame(Vec2::zero(), 10, 1));

        let current = baseline_rolling(body);
        let mut authoritative = baseline_rolling(body);
        authoritative.pos.0.x += POS_TOLERANCE_V1 * 2.0;

        let outcome = reconcile_v1(
            &read_data,
            &id_maps,
            entity,
            &mut buffer,
            &current,
            &authoritative,
            10,
            |_| true,
            |_| true,
        );

        let ReconciliationOutcomeV1::Replayed { trimmed, replayed, final_rolling, .. } = outcome
        else {
            panic!("expected Replayed, got {outcome:?}");
        };
        assert_eq!(trimmed, 1, "the baseline-5 frame must be trimmed, not replayed");
        assert_eq!(replayed, 1, "only the baseline-10 frame is a replay candidate");
        assert_eq!(buffer.len(), 1, "only the unacknowledged frame survives");
        // A humanoid's base_accel is on the order of tens of units/s^2;
        // one tick's worth of the OLD frame's move_dir=(1,0) input would
        // move vel.x by something on the order of accel * dt, easily
        // exceeding 0.1 -- far above what the zero-input replayed frame
        // could produce. Bounding well under that is the proof the old
        // frame's input never reached this result.
        assert!(
            (final_rolling.vel.0.x - authoritative.vel.0.x).abs() < 0.1,
            "vel.x moved as if the acknowledged-away frame's move_dir=(1,0) input replayed: \
             final={:?} authoritative={:?}",
            final_rolling.vel.0.x,
            authoritative.vel.0.x
        );
    }
}
