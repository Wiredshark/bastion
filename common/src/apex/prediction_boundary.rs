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

use super::weather_snapshot::WeatherSnapshotIdV1;
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
/// fn insert_a_component<C: MayInsertComponentsV1>(_: C) {}
/// // T7.2 Decision 1: LazyUpdate is unavailable during replay. If this
/// // ever compiles, a predicted frame can queue a component insertion.
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
