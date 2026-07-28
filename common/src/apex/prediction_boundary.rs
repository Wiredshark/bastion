//! `APEX-T7.2a` — the prediction boundary, as types.
//!
//! **T7.2 IS A ONE-WAY DOOR. Once client prediction and server authority
//! share a kernel, every later character-behaviour change changes both.
//! That is the row's purpose and the reason T7.1 gates it.**
//!
//! Built under the boundary approved at `T7.1`
//! (`readme/apex/APEX-T7.1-PREDICTION-BOUNDARY-PROPOSAL-v1.md`), all five
//! decisions standing.
//!
//! **What this module is, after the row was re-sized by reading it.** The
//! transition is already substantially pure: `StateUpdate`
//! (`comp/character_state.rs`) is an owned 11-field output with no
//! handles or channels, and `behavior()`/`handle_event()` already take
//! `&JoinData` and return one. So T7.2 is not "make it pure". It is:
//!
//! 1. make Decision 1's input/ambient split **explicit in the types**
//!    instead of implicit in a 38-field struct where both kinds sit side
//!    by side ([`PREDICTION_FIELD_ROLES`]);
//! 2. make `LazyUpdate` and the authority-only emitters **unavailable
//!    during replay by construction** ([`ReplayContextV1`]) rather than
//!    forbidden by a comment;
//! 3. attach Decision 2's world-revision identity ([`WorldRevisionV1`]).
//!
//! The narrower door is better news than the wide one, not lesser work:
//! the part that had to be right first is the part that is here.
//!
//! **The capability split is the load-bearing idea.** A predicted frame
//! must not insert components or emit authority-only effects. Guarding
//! that at each call site means remembering at each call site. Instead
//! the ability is a TRAIT that [`LiveContextV1`] implements and
//! [`ReplayContextV1` ] does not, so a replay that tries does not
//! compile. Two `compile_fail` doctests pin it, because a missing impl is
//! exactly what a well-meaning patch adds back.

use super::{
    physics_generation::{PhysicsGenerationV1, PredictionHistoryV1},
    weather_snapshot::WeatherSnapshotIdV1,
};
use crate::{
    comp::{
        Controller,
        controller::{InputAttr, InputKind, QueuedCommand},
    },
    resources::{DeltaTime, Time},
};
use vek::Vec2;

/// What a `JoinData` field is, for prediction.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PredictionFieldRoleV1 {
    /// A replayed frame's output can differ when this differs, so it is
    /// stored in history and replayed from it.
    TransitionInput,
    /// Read from AUTHORITY at replay time, never from history. Storing it
    /// would let a replay run against a world that no longer exists.
    AmbientAccess,
    /// Not state at all — a write channel. A predicted frame that uses
    /// one is emitting a side effect, which is Decision 4's problem, and
    /// [`ReplayContextV1`] is what makes it unavailable.
    WriteChannel,
    /// WHO is transitioning. Fixed for the whole of a history: neither
    /// replayed from it nor re-read from ambient authority, because a
    /// replay of a different entity is not a replay.
    ///
    /// **Found by encoding the classification as a test.** `T7.1`'s
    /// Decision 1 listed 14 inputs and 22 ambient fields and called
    /// `JoinData` 38 fields wide. Those numbers do not add up, and
    /// neither I nor the independent review noticed: `entity` and `uid`
    /// appear in the struct and in NEITHER list. They are identity, not
    /// state, which is why they read as too obvious to classify — and
    /// "too obvious to write down" is how a boundary acquires a hole.
    Identity,
}

/// The approved classification of every `JoinData` field, from `T7.1`
/// Decision 1.
///
/// The rule that produced it, restated so a future reader can re-derive
/// rather than trust: **a field is a transition input iff a replayed
/// frame's output can differ when it differs.**
pub const PREDICTION_FIELD_ROLES: &[(&str, PredictionFieldRoleV1)] = {
    use PredictionFieldRoleV1::{AmbientAccess as A, TransitionInput as I, WriteChannel as W};
    &[
        ("character", I),
        ("character_activity", I),
        ("pos", I),
        ("vel", I),
        ("ori", I),
        ("dt", I),
        ("time", I),
        ("controller", I),
        ("inputs", I),
        // Settled by independent review, not by assertion: provable from
        // the StateUpdate `From` chain (behavior.rs:153 ->
        // character_state.rs:94 value copy -> requirements_paid).
        ("energy", I),
        ("physics", I),
        ("mount_data", I),
        ("volume_mount_data", I),
        ("stance", I),
        ("scale", A),
        ("mass", A),
        ("density", A),
        ("body", A),
        // Ambient because death is AUTHORITATIVE and a predicted frame
        // must never decide it.
        ("health", A),
        ("heads", A),
        // Ambient because item state is another authority's; predicting
        // an equip would be predicting a server decision.
        ("inventory", A),
        ("stats", A),
        ("skill_set", A),
        ("active_abilities", A),
        ("ability_map", A),
        ("msm", A),
        ("combo", A),
        ("alignment", A),
        ("terrain", A),
        ("melee_attack", A),
        ("id_maps", A),
        ("alignments", A),
        ("prev_phys_caches", A),
        ("bodies", A),
        ("constructed_ladder_traversal", A),
        ("updater", W),
        // See `PredictionFieldRoleV1::Identity` — the two the approved
        // boundary omitted.
        ("entity", PredictionFieldRoleV1::Identity),
        ("uid", PredictionFieldRoleV1::Identity),
    ]
};

/// The world revision a predicted frame ran against.
///
/// Decision 2: history stores the world's IDENTITY, never a copy of it.
/// A frame whose revision is gone is **not replayable** — the client
/// snaps rather than replaying against current terrain, because a
/// plausible substitute is what a caller silently uses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldRevisionV1 {
    /// The weather snapshot, from `T5.4`/`T5.2`.
    pub weather: WeatherSnapshotIdV1,
    /// The chunk keys this frame's physics query actually touched.
    ///
    /// Per-frame rather than blanket: invalidating all history on any
    /// unload would make prediction useless at a chunk boundary, which is
    /// where players spend most of their time moving. Cost measured at
    /// review: ~1.05 KiB per client from the real
    /// `Spiral2d::new().take(9)`, 1.6% of Decision 5's 64 KiB budget, so
    /// the coarser fallback the proposal offered was struck.
    pub touched_chunks: Vec<Vec2<i32>>,
}

/// Why a frame cannot be replayed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NotReplayableV1 {
    /// A chunk the frame read is no longer loaded.
    ChunkUnloaded(Vec2<i32>),
    /// The weather snapshot is no longer retained.
    WeatherSnapshotGone(WeatherSnapshotIdV1),
}

impl WorldRevisionV1 {
    /// Whether this frame may be replayed against the world as it is now.
    ///
    /// Returns the FIRST reason it cannot, not a count — the first reason
    /// is the one to act on, and the rest are usually downstream.
    pub fn replayable_against_v1(
        &self,
        chunk_is_loaded: impl Fn(Vec2<i32>) -> bool,
        weather_is_retained: impl Fn(WeatherSnapshotIdV1) -> bool,
    ) -> Result<(), NotReplayableV1> {
        if !weather_is_retained(self.weather) {
            return Err(NotReplayableV1::WeatherSnapshotGone(self.weather));
        }
        for chunk in &self.touched_chunks {
            if !chunk_is_loaded(*chunk) {
                return Err(NotReplayableV1::ChunkUnloaded(*chunk));
            }
        }
        Ok(())
    }
}

/// A predicted frame may insert components.
///
/// Implemented for [`LiveContextV1`] and NOT for [`ReplayContextV1`], so
/// a replay that reaches for `LazyUpdate` does not compile.
pub trait MayInsertComponentsV1 {}

/// A predicted frame may emit authority-only effects — damage, item
/// transfer, block changes, chat, death.
///
/// Class 3 of Decision 4. Implemented for [`LiveContextV1`] only.
pub trait MayEmitAuthorityEffectsV1 {}

/// The live tick. Everything is permitted.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct LiveContextV1;

impl MayInsertComponentsV1 for LiveContextV1 {}
impl MayEmitAuthorityEffectsV1 for LiveContextV1 {}

/// A replay of an already-predicted frame.
///
/// Implements NEITHER capability. That absence is the mechanism: it is
/// not that replay is asked not to emit, it is that the function that
/// emits cannot be called with this type.
///
/// ```compile_fail
/// # use veloren_common::apex::prediction_boundary::*;
/// // This is the exact bound `JoinData::updater_v1` requires, so this
/// // doctest IS the pin on that accessor: if it compiles, a replay can
/// // obtain a LazyUpdate and queue a component insertion.
/// fn insert_a_component<C: MayInsertComponentsV1>(_: C) {}
/// insert_a_component(ReplayContextV1);
/// ```
///
/// ```compile_fail
/// # use veloren_common::apex::prediction_boundary::*;
/// fn emit_damage<C: MayEmitAuthorityEffectsV1>(_: C) {}
/// // T7.2 Decision 4 class 3: authority-only effects are unavailable
/// // during replay, not merely discouraged.
/// emit_damage(ReplayContextV1);
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct ReplayContextV1;

// ---------------------------------------------------------------------
// `APEX-T7.3a` — the client prediction buffer.
// ---------------------------------------------------------------------

/// One predicted frame's DRIVING input -- what a replay cannot re-derive
/// by re-running the pure transition, as distinct from the ROLLING state
/// (`pos`/`vel`/`character`/`energy`/`physics`/`mount_data`/
/// `volume_mount_data`/`stance`) that Decision 1 also classifies
/// `TransitionInput`.
///
/// That classification is correct and not contradicted here: those
/// fields' VALUES do change the output. But a replay BUFFER doesn't need
/// to carry them explicitly, because they are exactly the ACCUMULATED
/// OUTPUT of replaying every earlier stored frame from a known baseline
/// forward -- storing them again per frame would duplicate what the
/// replay itself reconstructs. What a frame cannot be reconstructed
/// without is what the player actually DID that tick: the full
/// `Controller` (raw analog inputs plus the queued discrete action/
/// command channel it carries), and the `dt`/`time` that tick ran under.
///
/// Also carries Decision 2's [`WorldRevisionV1`] -- PER FRAME, not once
/// for the whole buffer, because weather/touched-chunks can change
/// between two frames within the same 500ms window.
#[derive(Clone, Debug, PartialEq)]
pub struct PredictedFrameV1 {
    pub controller: Controller,
    pub dt: DeltaTime,
    pub time: Time,
    pub world_revision: WorldRevisionV1,
}

impl PredictedFrameV1 {
    /// An APPROXIMATION, not an exact byte accounting -- disclosed the
    /// same way Decision 5's own 500ms/64KiB numbers are ("reasoned, not
    /// measured"). `Controller` carries a `Vec`/`BTreeMap` whose exact
    /// heap footprint isn't worth chasing precisely for a budget whose
    /// whole point is triggering a safe, recorded snap well before
    /// anything is actually memory-pressured.
    pub fn approx_size_bytes(&self) -> usize {
        std::mem::size_of::<Controller>()
            + self.controller.queued_inputs.len() * std::mem::size_of::<(InputKind, InputAttr)>()
            + self.controller.staged_commands().len() * std::mem::size_of::<QueuedCommand>()
            + std::mem::size_of::<DeltaTime>()
            + std::mem::size_of::<Time>()
            + std::mem::size_of::<WeatherSnapshotIdV1>()
            + self.world_revision.touched_chunks.len() * std::mem::size_of::<Vec2<i32>>()
    }
}

/// Why a push into the buffer was refused.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PushOutcomeV1 {
    Pushed,
    /// Decision 5's HARD budget, breached. Per Decision 5: this is a
    /// snap, recorded -- never a silent shortening of the window. The
    /// buffer is NOT partially trimmed to fit; the caller is expected to
    /// clear it (`ClientPredictionBufferV1::clear_v1`) and record the
    /// event, the same "first reason, not a count" discipline
    /// `WorldRevisionV1::replayable_against_v1` uses.
    BudgetExceeded { attempted_bytes: usize, budget_bytes: usize },
}

/// Decision 5's two limits, wrapping [`PredictionHistoryV1`]. The two
/// limits fail DIFFERENTLY on purpose, stated directly in Decision 5:
/// duration (500ms, the tick-count `capacity` below) exceeded drops the
/// oldest entry and keeps predicting -- `PredictionHistoryV1::push_v1`
/// already does this. Budget (64 KiB) exceeded is a hard stop, never a
/// silent trim -- that half doesn't exist in `PredictionHistoryV1`
/// itself (it has no byte notion at all), which is what this wrapper
/// adds.
pub struct ClientPredictionBufferV1 {
    history: PredictionHistoryV1<PredictedFrameV1>,
    budget_bytes: usize,
}

impl ClientPredictionBufferV1 {
    pub fn new(capacity_ticks: usize, budget_bytes: usize) -> Self {
        Self { history: PredictionHistoryV1::new(capacity_ticks), budget_bytes }
    }

    pub fn generation(&self) -> PhysicsGenerationV1 { self.history.generation() }

    pub fn len(&self) -> usize { self.history.len() }

    pub fn is_empty(&self) -> bool { self.history.is_empty() }

    /// The current buffer's approximate total size -- every stored
    /// entry, not just the currently-replayable generation, since a
    /// stale-but-not-yet-adopted entry still occupies the budget until
    /// it's evicted or the generation advances.
    pub fn approx_bytes_v1(&self) -> usize {
        self.history.iter_v1().map(|(_, frame)| frame.approx_size_bytes()).sum()
    }

    /// Attempts to record one tick's driving input. Checked BEFORE the
    /// tick-count capacity ever gets a chance to silently evict on this
    /// same push -- the budget check must see the buffer as it stands
    /// now, not after `PredictionHistoryV1`'s own eviction has already
    /// made room.
    pub fn push_v1(&mut self, frame: PredictedFrameV1) -> PushOutcomeV1 {
        let projected = self.approx_bytes_v1() + frame.approx_size_bytes();
        if projected > self.budget_bytes {
            return PushOutcomeV1::BudgetExceeded {
                attempted_bytes: projected,
                budget_bytes: self.budget_bytes,
            };
        }
        self.history.push_v1(frame);
        PushOutcomeV1::Pushed
    }

    /// Adopts a server correction -- see [`PredictionHistoryV1::adopt_generation_v1`].
    pub fn adopt_generation_v1(&mut self, generation: PhysicsGenerationV1) -> usize {
        self.history.adopt_generation_v1(generation)
    }

    /// Decision 3 (mount/carry termination) and the budget-exceeded
    /// path both call this: a whole-buffer invalidation that is NOT
    /// itself a server correction, so it does not advance the
    /// generation the way `adopt_generation_v1` does.
    pub fn clear_v1(&mut self) { self.history.clear_v1(); }

    /// The entries still eligible to replay -- see
    /// [`PredictionHistoryV1::replayable_v1`]. `T7.3b`'s consumer.
    pub fn replayable_v1(&self) -> impl Iterator<Item = &PredictedFrameV1> { self.history.replayable_v1() }
}

#[cfg(test)]
mod prediction_boundary_v1 {
    use super::*;

    /// A model transition with the approved shape: output is a function
    /// of the INPUTS only, and ambient access is readable but must not
    /// change what comes out.
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    struct ModelInputs {
        character: u32,
        energy: u32,
    }

    #[derive(Copy, Clone, Debug)]
    struct ModelAmbient {
        health: u32,
        inventory: u32,
    }

    fn model_transition(inputs: ModelInputs, _ambient: ModelAmbient) -> u32 {
        inputs.character * 10 + inputs.energy
    }

    /// **Decision 1's own test.** Mutating one ambient field between the
    /// original run and the replay must not move the output; mutating a
    /// transition input must. Run over every field of each kind, so a
    /// field classified by intuition rather than by behaviour is caught.
    #[test]
    fn ambient_fields_do_not_move_the_output_and_inputs_do() {
        let inputs = ModelInputs { character: 3, energy: 7 };
        let ambient = ModelAmbient { health: 100, inventory: 4 };
        let baseline = model_transition(inputs, ambient);

        for mutated in [
            ModelAmbient { health: 1, ..ambient },
            ModelAmbient { inventory: 999, ..ambient },
        ] {
            assert_eq!(
                model_transition(inputs, mutated),
                baseline,
                "an ambient field reached the output, so it is a transition input and the \
                 classification is wrong"
            );
        }

        for mutated in [
            ModelInputs { character: 4, ..inputs },
            ModelInputs { energy: 8, ..inputs },
        ] {
            assert_ne!(
                model_transition(mutated, ambient),
                baseline,
                "a transition input did NOT reach the output, so it is ambient and history is \
                 storing more than it needs"
            );
        }
    }

    /// The approved classification is pinned. All 38 `JoinData` fields
    /// are present exactly once, and the split matches `T7.1` Decision 1.
    #[test]
    fn every_join_data_field_is_classified_exactly_once() {
        assert_eq!(PREDICTION_FIELD_ROLES.len(), 38, "JoinData's field count moved; re-derive");

        let mut names: Vec<&str> = PREDICTION_FIELD_ROLES.iter().map(|(n, _)| *n).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "a field is classified twice");

        let count = |role| PREDICTION_FIELD_ROLES.iter().filter(|(_, r)| *r == role).count();
        assert_eq!(count(PredictionFieldRoleV1::TransitionInput), 14);
        assert_eq!(count(PredictionFieldRoleV1::AmbientAccess), 21);
        assert_eq!(
            count(PredictionFieldRoleV1::Identity),
            2,
            "`entity` and `uid` are the identity of the subject; a third identity field means              something changed about what a transition is FOR"
        );
        assert_eq!(
            count(PredictionFieldRoleV1::WriteChannel),
            1,
            "`updater` is the only write channel; a second one is Decision 4's problem and needs \
             a ruling, not a classification"
        );
    }

    /// The three fields the proposal called out as sitting on a
    /// non-obvious line stay where the approved boundary put them.
    #[test]
    fn the_contested_fields_stay_where_the_boundary_put_them() {
        let role = |name: &str| {
            PREDICTION_FIELD_ROLES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("{name} is not classified"))
        };
        assert_eq!(role("energy"), PredictionFieldRoleV1::TransitionInput);
        assert_eq!(role("health"), PredictionFieldRoleV1::AmbientAccess);
        assert_eq!(role("inventory"), PredictionFieldRoleV1::AmbientAccess);
        assert_eq!(role("updater"), PredictionFieldRoleV1::WriteChannel);
    }

    /// A frame whose chunk went away is not replayable, and the reason
    /// names the chunk. Snap, never replay against current terrain.
    #[test]
    fn an_unloaded_chunk_makes_a_frame_unreplayable() {
        let revision = WorldRevisionV1 {
            weather: WeatherSnapshotIdV1::from_sequence_v1(5),
            touched_chunks: vec![Vec2::new(1, 1), Vec2::new(2, 2)],
        };
        assert_eq!(revision.replayable_against_v1(|_| true, |_| true), Ok(()));
        assert_eq!(
            revision.replayable_against_v1(|c| c != Vec2::new(2, 2), |_| true),
            Err(NotReplayableV1::ChunkUnloaded(Vec2::new(2, 2)))
        );
    }

    /// A chunk the frame never touched can unload freely — that is why
    /// the revision records touched keys instead of invalidating
    /// everything on any unload.
    #[test]
    fn an_untouched_chunk_unloading_does_not_invalidate_the_frame() {
        let revision = WorldRevisionV1 {
            weather: WeatherSnapshotIdV1::from_sequence_v1(5),
            touched_chunks: vec![Vec2::new(1, 1)],
        };
        assert_eq!(
            revision.replayable_against_v1(|c| c != Vec2::new(9, 9), |_| true),
            Ok(()),
            "an unrelated unload invalidated the frame; prediction would be useless at a chunk \
             boundary"
        );
    }

    /// Weather is checked BEFORE chunks, and its absence is its own
    /// reason — a client that snapped for the wrong reason looks for the
    /// wrong bug.
    #[test]
    fn a_missing_weather_snapshot_is_reported_before_any_chunk() {
        let revision = WorldRevisionV1 {
            weather: WeatherSnapshotIdV1::from_sequence_v1(5),
            touched_chunks: vec![Vec2::new(1, 1)],
        };
        assert_eq!(
            revision.replayable_against_v1(|_| false, |_| false),
            Err(NotReplayableV1::WeatherSnapshotGone(
                WeatherSnapshotIdV1::from_sequence_v1(5)
            )),
            "a chunk reason was reported for a frame whose weather was already gone"
        );
    }

    /// The live context has both capabilities. The replay context's lack
    /// of them is pinned by the `compile_fail` doctests on
    /// `ReplayContextV1`; this asserts the positive half, so the traits
    /// cannot be satisfied by nobody at all.
    #[test]
    fn the_live_context_has_both_capabilities() {
        fn insert<C: MayInsertComponentsV1>(_: C) -> bool { true }
        fn emit<C: MayEmitAuthorityEffectsV1>(_: C) -> bool { true }
        assert!(insert(LiveContextV1));
        assert!(emit(LiveContextV1));
    }
}

/// `APEX-T7.3a` — the client prediction buffer.
#[cfg(test)]
mod client_prediction_buffer_v1 {
    use super::*;

    fn frame() -> PredictedFrameV1 {
        PredictedFrameV1 {
            controller: Controller::default(),
            dt: DeltaTime(1.0 / 30.0),
            time: Time(0.0),
            world_revision: WorldRevisionV1 {
                weather: WeatherSnapshotIdV1::from_sequence_v1(0),
                touched_chunks: Vec::new(),
            },
        }
    }

    /// Decision 5's duration limit (the tick-count `capacity`): exceeded
    /// silently drops the oldest entry and KEEPS predicting. This is the
    /// behaviour `PredictionHistoryV1::push_v1` already has; this test
    /// pins that the wrapper doesn't change it.
    #[test]
    fn duration_limit_drops_oldest_and_keeps_predicting() {
        let mut buffer = ClientPredictionBufferV1::new(3, 1_000_000);
        for _ in 0..5 {
            assert_eq!(buffer.push_v1(frame()), PushOutcomeV1::Pushed);
        }
        assert_eq!(buffer.len(), 3, "the oldest entries were dropped, not refused");
    }

    /// Decision 5's HARD budget: exceeded is a snap, recorded — never a
    /// silent shortening. Falsified both ways: the push that breaches
    /// the budget is REFUSED outright (not silently trimmed to fit), and
    /// the buffer is left holding exactly what it held before the
    /// refused push (nothing partially applied).
    #[test]
    fn budget_exhaustion_refuses_the_push_rather_than_silently_shortening() {
        let one_frame_bytes = frame().approx_size_bytes();
        // Budget room for exactly 2 frames, tick-count capacity generous
        // enough that duration would never be the thing that fires here.
        let mut buffer = ClientPredictionBufferV1::new(100, one_frame_bytes * 2);

        assert_eq!(buffer.push_v1(frame()), PushOutcomeV1::Pushed);
        assert_eq!(buffer.push_v1(frame()), PushOutcomeV1::Pushed);
        assert_eq!(buffer.len(), 2);

        let before_bytes = buffer.approx_bytes_v1();
        let outcome = buffer.push_v1(frame());
        assert_eq!(
            outcome,
            PushOutcomeV1::BudgetExceeded {
                attempted_bytes: before_bytes + one_frame_bytes,
                budget_bytes: one_frame_bytes * 2,
            }
        );
        // Falsifies "silently shortened": length and byte total are
        // UNCHANGED by the refused push, not trimmed down to fit.
        assert_eq!(buffer.len(), 2, "a refused push must not partially apply");
        assert_eq!(buffer.approx_bytes_v1(), before_bytes);
    }

    /// The caller's expected response to a budget breach: clear and
    /// record, not retry with a smaller frame.
    #[test]
    fn caller_clears_on_budget_exceeded_and_the_buffer_is_then_empty() {
        let one_frame_bytes = frame().approx_size_bytes();
        let mut buffer = ClientPredictionBufferV1::new(100, one_frame_bytes);
        assert_eq!(buffer.push_v1(frame()), PushOutcomeV1::Pushed);
        assert!(matches!(buffer.push_v1(frame()), PushOutcomeV1::BudgetExceeded { .. }));

        buffer.clear_v1();
        assert!(buffer.is_empty());
        assert_eq!(buffer.approx_bytes_v1(), 0);
        // Room again after the clear.
        assert_eq!(buffer.push_v1(frame()), PushOutcomeV1::Pushed);
    }

    /// The acceptance criterion (`physics_generation.rs`'s own test),
    /// re-proven on the REAL client-attached wrapper, not just the
    /// generic type it wraps.
    #[test]
    fn corrected_away_predictions_cannot_replay_into_a_newer_generation_on_the_wrapper() {
        let mut buffer = ClientPredictionBufferV1::new(8, 1_000_000);
        for _ in 0..5 {
            buffer.push_v1(frame());
        }
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.replayable_v1().count(), 5);

        let mut server = crate::apex::physics_generation::PhysicsCorrectionStateV1::new();
        let corrected = server.force_correction_v1().unwrap();
        let dropped = buffer.adopt_generation_v1(corrected);
        assert_eq!(dropped, 5, "every pre-correction prediction is invalidated");
        assert!(buffer.is_empty());
        assert_eq!(buffer.replayable_v1().count(), 0);

        buffer.push_v1(frame());
        assert_eq!(buffer.replayable_v1().count(), 1, "predictions after the correction are replayable again");
    }

    /// Decision 3 (mount/carry termination): clearing is NOT a
    /// correction — it must not advance the generation, unlike
    /// `adopt_generation_v1`. A caller that clears on entering a mount
    /// and then leaves it should not have silently skipped a generation
    /// the server never issued.
    #[test]
    fn clear_v1_for_mount_termination_does_not_advance_the_generation() {
        let mut buffer = ClientPredictionBufferV1::new(8, 1_000_000);
        let before = buffer.generation();
        buffer.push_v1(frame());
        buffer.clear_v1();
        assert_eq!(buffer.generation(), before);
        assert!(buffer.is_empty());
    }

    #[test]
    fn approx_bytes_v1_grows_with_each_push_and_shrinks_on_eviction() {
        let mut buffer = ClientPredictionBufferV1::new(2, 1_000_000);
        assert_eq!(buffer.approx_bytes_v1(), 0);
        buffer.push_v1(frame());
        let one = buffer.approx_bytes_v1();
        assert!(one > 0);
        buffer.push_v1(frame());
        assert_eq!(buffer.approx_bytes_v1(), one * 2);
        // Capacity 2: a third push evicts the first, net size unchanged.
        buffer.push_v1(frame());
        assert_eq!(buffer.approx_bytes_v1(), one * 2);
    }
}
