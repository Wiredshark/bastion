#[cfg(test)]
mod tests {
    use common::{
        SkillSetBuilder,
        apex::{
            physics_generation::PhysicsGenerationV1,
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
            PhysicsGenerationV1::NEVER_CORRECTED,
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
            PhysicsGenerationV1::NEVER_CORRECTED,
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
            PhysicsGenerationV1::NEVER_CORRECTED,
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
            PhysicsGenerationV1::NEVER_CORRECTED,
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

    // -- `APEX-T7.4` item A: stale-generation rejection ----------------------

    /// The row's own required test, live: a `physics_generation` OLDER
    /// than what the buffer has already adopted is rejected -- and the
    /// buffer is left in EXACTLY the state it started in (same length,
    /// same generation, same entries), not merely "not cleared".
    #[test]
    fn a_stale_generation_is_rejected_without_touching_the_buffer() {
        let mut state = setup();
        let (entity, body) = create_entity(&mut state);
        let ecs = state.ecs();
        let read_data = ReadData::fetch(ecs);
        let id_maps = Read::<IdMaps>::fetch(ecs);

        let mut buffer = ClientPredictionBufferV1::new(16, 64 * 1024);
        buffer.push_v1(frame(Vec2::zero(), 10, 0));
        // Advance the buffer to generation 2 first, so generation 1 is
        // genuinely stale (older), not merely "not yet reached".
        buffer.adopt_generation_v1(PhysicsGenerationV1::from_legacy_counter_v1(2));
        buffer.push_v1(frame(Vec2::zero(), 20, 0));
        let generation_before = buffer.generation();
        let len_before = buffer.len();

        let current = baseline_rolling(body);
        let mut authoritative = baseline_rolling(body);
        // Divergent on purpose -- proves the rejection happens BEFORE
        // the agreement check even runs, not because nothing diverged.
        authoritative.pos.0.x += POS_TOLERANCE_V1 * 2.0;

        let outcome = reconcile_v1(
            &read_data,
            &id_maps,
            entity,
            &mut buffer,
            &current,
            &authoritative,
            10,
            PhysicsGenerationV1::from_legacy_counter_v1(1),
            |_| true,
            |_| true,
        );

        match outcome {
            ReconciliationOutcomeV1::StaleCorrection { buffer_generation, got_generation } => {
                assert_eq!(buffer_generation, generation_before);
                assert_eq!(got_generation, PhysicsGenerationV1::from_legacy_counter_v1(1));
            },
            other => panic!("expected StaleCorrection, got {other:?}"),
        }
        assert_eq!(buffer.generation(), generation_before, "generation must not move on a rejected stale correction");
        assert_eq!(buffer.len(), len_before, "not one entry may be trimmed or dropped on a rejected stale correction");
    }

    /// A genuinely NEWER generation is adopted (not rejected): entries
    /// from the old generation are dropped by `adopt_generation_v1`
    /// before the rest of `reconcile_v1` ever runs, so a buffer holding
    /// only pre-correction entries has nothing left to replay.
    #[test]
    fn a_newer_generation_is_adopted_and_invalidates_older_entries() {
        let mut state = setup();
        let (entity, body) = create_entity(&mut state);
        let ecs = state.ecs();
        let read_data = ReadData::fetch(ecs);
        let id_maps = Read::<IdMaps>::fetch(ecs);

        let mut buffer = ClientPredictionBufferV1::new(16, 64 * 1024);
        // Every entry is captured under generation 0 (NEVER_CORRECTED),
        // the buffer's own starting generation.
        buffer.push_v1(frame(Vec2::zero(), 10, 0));
        assert_eq!(buffer.generation(), PhysicsGenerationV1::NEVER_CORRECTED);

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
            PhysicsGenerationV1::from_legacy_counter_v1(1),
            |_| true,
            |_| true,
        );

        assert_eq!(buffer.generation(), PhysicsGenerationV1::from_legacy_counter_v1(1), "the newer generation must be adopted");
        assert!(buffer.is_empty(), "every generation-0 entry must be invalidated by the jump to generation 1");
        match outcome {
            ReconciliationOutcomeV1::Replayed { replayed, .. } => {
                assert_eq!(replayed, 0, "nothing survived the generation jump to replay");
            },
            other => panic!("expected Replayed (with zero frames replayed), got {other:?}"),
        }
    }

    // -- `APEX-T7.4` item A: correction-magnitude recording -------------------

    /// `position_correction_distance` is the distance between what the
    /// client believed BEFORE this correction and what replay concluded
    /// AFTER it -- checked against a value computed independently here,
    /// not merely "present".
    #[test]
    fn position_correction_distance_matches_the_actual_correction() {
        let mut state = setup();
        let (entity, body) = create_entity(&mut state);
        let ecs = state.ecs();
        let read_data = ReadData::fetch(ecs);
        let id_maps = Read::<IdMaps>::fetch(ecs);

        let mut buffer = ClientPredictionBufferV1::new(16, 64 * 1024);
        // A zero-effect frame: `final_rolling` should land at (very
        // close to) the authoritative baseline, so the correction
        // distance is dominated by `current` vs `authoritative`, a
        // value this test controls directly.
        buffer.push_v1(frame(Vec2::zero(), 10, 0));

        let current = baseline_rolling(body);
        let mut authoritative = baseline_rolling(body);
        authoritative.pos.0.x += POS_TOLERANCE_V1 * 2.0;
        let expected_distance = current.pos.0.distance(authoritative.pos.0);

        let outcome = reconcile_v1(
            &read_data,
            &id_maps,
            entity,
            &mut buffer,
            &current,
            &authoritative,
            10,
            PhysicsGenerationV1::NEVER_CORRECTED,
            |_| true,
            |_| true,
        );

        let ReconciliationOutcomeV1::Replayed { position_correction_distance, final_rolling, .. } = outcome else {
            panic!("expected Replayed, got {outcome:?}");
        };
        // A zero-move_dir frame should leave pos essentially unchanged
        // from the authoritative baseline, so the recorded distance
        // should be very close to the independently-computed value.
        assert!(
            (position_correction_distance - expected_distance).abs() < 1e-3,
            "recorded={position_correction_distance} expected={expected_distance} final_pos={:?}",
            final_rolling.pos
        );
    }

    #[test]
    fn correction_magnitude_metrics_summary_reflects_recorded_corrections() {
        use veloren_common_systems::reconciliation::CorrectionMagnitudeMetricsV1;

        let metrics = CorrectionMagnitudeMetricsV1::new();
        assert_eq!(metrics.summary_v1(), (0, None), "no mean before anything is recorded, not a division-by-zero placeholder");

        metrics.record_correction_v1(1.0);
        metrics.record_correction_v1(3.0);
        let (count, mean) = metrics.summary_v1();
        assert_eq!(count, 2);
        assert!((mean.unwrap() - 2.0).abs() < 1e-9, "mean of 1.0 and 3.0 must be 2.0, got {mean:?}");
    }

    // -- `APEX-T7.4` item C: the row's own acceptance criterion, tested as a
    // determinism property -----------------------------------------------
    //
    // "The same correction plus the same input history, run twice, produces
    // the same final state -- and, separately, produces the same EVENT set,
    // since a replay that silently re-emits is the failure mode this row is
    // most likely to ship with." (`APEX-T7-TIER-SPEC-FLEET-v1.md`, T7.4's
    // own required-tests text.) The third required test named there ("a
    // stale correction must be rejected without touching history") is
    // ALREADY item A's own `a_stale_generation_is_rejected_without_touching_
    // the_buffer` above -- not duplicated here.
    //
    // Scope note, disclosed rather than silently assumed: these fixtures'
    // frames are zero-effect (`move_dir: Vec2::zero()`), so `sink_counts`
    // is expected empty on both runs. The determinism property under test
    // is "identical inputs produce identical outputs", which holds
    // regardless of whether the set happens to be empty or not -- an empty
    // set compared for equality is still a real equality check, just not
    // one that exercises a NONZERO event count. `reconcile_v1` has no
    // ambient/wall-clock/random inputs (verified: it takes every input as a
    // parameter, per this module's own doc), so nothing in its own
    // implementation depends on which case is exercised.

    /// Builds two INDEPENDENT, but constructed-identically, buffers and
    /// scenarios, and runs `reconcile_v1` once against each -- proving the
    /// determinism property directly (two separate computations from the
    /// same inputs agree), not merely "ran without changing between two
    /// calls on the same mutable buffer" (which `adopt_generation_v1`'s own
    /// idempotency could make trivially true even if something else were
    /// silently stateful).
    #[test]
    fn running_reconcile_v1_twice_with_identical_inputs_produces_the_same_final_state_and_event_set() {
        fn run_once() -> ReconciliationOutcomeV1 {
            let mut state = setup();
            let (entity, body) = create_entity(&mut state);
            let ecs = state.ecs();
            let read_data = ReadData::fetch(ecs);
            let id_maps = Read::<IdMaps>::fetch(ecs);

            let mut buffer = ClientPredictionBufferV1::new(16, 64 * 1024);
            buffer.push_v1(frame(Vec2::zero(), 10, 0));
            buffer.push_v1(frame(Vec2::zero(), 20, 1));

            let current = baseline_rolling(body);
            let mut authoritative = baseline_rolling(body);
            authoritative.pos.0.x += POS_TOLERANCE_V1 * 2.0;

            reconcile_v1(
                &read_data,
                &id_maps,
                entity,
                &mut buffer,
                &current,
                &authoritative,
                10,
                PhysicsGenerationV1::NEVER_CORRECTED,
                |_| true,
                |_| true,
            )
        }

        let first = run_once();
        let second = run_once();

        let (
            ReconciliationOutcomeV1::Replayed {
                trimmed: trimmed_a,
                replayed: replayed_a,
                final_rolling: final_a,
                sink_counts: sink_a,
                position_correction_distance: distance_a,
                ..
            },
            ReconciliationOutcomeV1::Replayed {
                trimmed: trimmed_b,
                replayed: replayed_b,
                final_rolling: final_b,
                sink_counts: sink_b,
                position_correction_distance: distance_b,
                ..
            },
        ) = (first, second)
        else {
            panic!("expected both runs to Replay");
        };

        // Same final state.
        assert_eq!(trimmed_a, trimmed_b);
        assert_eq!(replayed_a, replayed_b);
        assert_eq!(final_a.pos.0, final_b.pos.0);
        assert_eq!(final_a.vel.0, final_b.vel.0);
        assert_eq!(final_a.ori, final_b.ori);
        assert_eq!(final_a.char_state, final_b.char_state);
        assert_eq!(distance_a, distance_b);
        // Same event set (see the scope note above: expected empty here,
        // still a real equality check on two independently-computed
        // vectors).
        assert_eq!(sink_a, sink_b);
    }

    /// The same property under a genuinely NEWER generation (an adopted
    /// correction, not just a routine same-generation `CompSync`) --
    /// proving determinism holds across the generation-adoption branch
    /// `item A` added, not just the pre-existing trim/replay path.
    #[test]
    fn running_reconcile_v1_twice_after_a_generation_adoption_produces_the_same_final_state() {
        fn run_once() -> ReconciliationOutcomeV1 {
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

            reconcile_v1(
                &read_data,
                &id_maps,
                entity,
                &mut buffer,
                &current,
                &authoritative,
                10,
                PhysicsGenerationV1::from_legacy_counter_v1(1),
                |_| true,
                |_| true,
            )
        }

        let first = run_once();
        let second = run_once();
        match (first, second) {
            (ReconciliationOutcomeV1::Replayed { replayed: a, .. }, ReconciliationOutcomeV1::Replayed { replayed: b, .. }) => {
                assert_eq!(a, b);
                assert_eq!(a, 0, "the generation jump invalidates the single generation-0 entry, so nothing replays either time");
            },
            other => panic!("expected both runs to Replay (with zero frames replayed), got {other:?}"),
        }
    }
}
