//! `APEX-T3.6` — physics correction generations. When the server forces
//! a client's position, everything the client predicted before that
//! correction is void. Today the boundary between "before" and "after"
//! is a bare `u64` counter on `ForceUpdate` that advances with
//! `wrapping_add`, and the client echoes it back on every
//! `PlayerPhysics` frame. A wrap makes a stale generation compare equal
//! to a live one, which is exactly the door this row closes.
//!
//! Relationship to `T3.5`, kept deliberately distinct: `PlayerPhysics`
//! is a `LatestState` payload — newest wins, never journaled. The
//! generation is the OUTER guard that decides which "newest" is even
//! eligible. Latest-state answers "of the frames I may apply, which is
//! freshest"; the generation answers "is this frame from the world I am
//! still in". Collapsing the two would let a newest-but-stale frame win.

use serde::{Deserialize, Serialize};

/// A physics correction generation. Monotone per entity; every advance
/// invalidates the client prediction history that preceded it.
///
/// Generation `0` is "no correction has ever been forced", which is the
/// value a fresh `ForceUpdate` carries and the value a client that has
/// never been corrected echoes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PhysicsGenerationV1(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsGenerationErrorV1 {
    /// The generation space is exhausted. Refusing is the whole point:
    /// wrapping would make a stale generation compare equal to a live
    /// one, and a corrected-away prediction could replay.
    Exhausted,
}

impl PhysicsGenerationV1 {
    pub const NEVER_CORRECTED: Self = Self(0);

    pub const fn from_legacy_counter_v1(counter: u64) -> Self { Self(counter) }

    pub const fn as_legacy_counter_v1(self) -> u64 { self.0 }

    /// CHECKED, never wrapping (`T3.6` step 4).
    pub fn advance_v1(self) -> Result<Self, PhysicsGenerationErrorV1> {
        self.0.checked_add(1).map(Self).ok_or(PhysicsGenerationErrorV1::Exhausted)
    }

    pub const fn is_never_corrected(self) -> bool { self.0 == 0 }
}

/// What a client frame or a prediction-history entry is stamped with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsStampV1 {
    pub generation: PhysicsGenerationV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsAdmitV1 {
    /// Same generation as the server's: eligible for latest-state
    /// arbitration.
    Eligible,
    /// From before the newest correction: refused outright.
    StaleGeneration { server: u64, got: u64 },
    /// Newer than anything the server has issued — a client cannot mint
    /// a generation.
    ForgedGeneration { server: u64, got: u64 },
}

/// The server's view of one entity's correction generation, and the
/// admission rule for client state reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicsCorrectionStateV1 {
    generation: PhysicsGenerationV1,
}

impl PhysicsCorrectionStateV1 {
    pub fn new() -> Self { Self { generation: PhysicsGenerationV1::NEVER_CORRECTED } }

    pub fn from_legacy_counter_v1(counter: u64) -> Self {
        Self { generation: PhysicsGenerationV1::from_legacy_counter_v1(counter) }
    }

    pub fn generation(&self) -> PhysicsGenerationV1 { self.generation }

    /// Forces a correction, advancing the generation. Every prediction
    /// the client made under the old generation is void from here.
    pub fn force_correction_v1(&mut self) -> Result<PhysicsGenerationV1, PhysicsGenerationErrorV1> {
        self.generation = self.generation.advance_v1()?;
        Ok(self.generation)
    }

    /// Admits a client state report. A report is eligible only if it
    /// carries the CURRENT generation: older means it was computed
    /// before a correction the client had not seen, newer means the
    /// client invented one.
    pub fn admit_report_v1(&self, stamp: PhysicsStampV1) -> PhysicsAdmitV1 {
        let server = self.generation.as_legacy_counter_v1();
        let got = stamp.generation.as_legacy_counter_v1();
        match got.cmp(&server) {
            std::cmp::Ordering::Equal => PhysicsAdmitV1::Eligible,
            std::cmp::Ordering::Less => PhysicsAdmitV1::StaleGeneration { server, got },
            std::cmp::Ordering::Greater => PhysicsAdmitV1::ForgedGeneration { server, got },
        }
    }
}

/// The client's prediction history, stamped by generation. On a
/// correction the entries from older generations are dropped, so they
/// can never be replayed into the new one.
#[derive(Debug, Clone, Default)]
pub struct PredictionHistoryV1<T> {
    generation: PhysicsGenerationV1,
    entries: std::collections::VecDeque<(PhysicsGenerationV1, T)>,
    capacity: usize,
}

impl<T> PredictionHistoryV1<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            generation: PhysicsGenerationV1::NEVER_CORRECTED,
            entries: std::collections::VecDeque::new(),
            capacity,
        }
    }

    pub fn generation(&self) -> PhysicsGenerationV1 { self.generation }

    pub fn len(&self) -> usize { self.entries.len() }

    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// Records a prediction under the current generation.
    pub fn push_v1(&mut self, entry: T) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back((self.generation, entry));
    }

    /// Adopts a server correction. Entries from any older generation are
    /// dropped, because they describe a world the server has overruled.
    /// Returns how many were invalidated.
    pub fn adopt_generation_v1(&mut self, generation: PhysicsGenerationV1) -> usize {
        if generation <= self.generation {
            return 0;
        }
        let before = self.entries.len();
        self.entries.retain(|(stamped, _)| *stamped >= generation);
        self.generation = generation;
        before - self.entries.len()
    }

    /// The entries still eligible to replay: only those stamped with the
    /// current generation.
    pub fn replayable_v1(&self) -> impl Iterator<Item = &T> {
        let generation = self.generation;
        self.entries.iter().filter(move |(stamped, _)| *stamped == generation).map(|(_, entry)| entry)
    }
}

#[cfg(test)]
mod physics_generation_v1 {
    use super::*;

    /// Step 4: advancement is checked. At the ceiling it REFUSES rather
    /// than wrapping — the behaviour `ForceUpdate::update`'s
    /// `wrapping_add` has today.
    #[test]
    fn advancement_is_checked_and_never_wraps() {
        let g = PhysicsGenerationV1::NEVER_CORRECTED;
        assert!(g.is_never_corrected());
        let next = g.advance_v1().unwrap();
        assert_eq!(next.as_legacy_counter_v1(), 1);

        let ceiling = PhysicsGenerationV1::from_legacy_counter_v1(u64::MAX);
        assert_eq!(ceiling.advance_v1().unwrap_err(), PhysicsGenerationErrorV1::Exhausted);
        // and the wrap it replaces would have compared EQUAL to a fresh
        // generation, which is the whole hazard
        assert_ne!(u64::MAX.wrapping_add(1), u64::MAX);
        assert_eq!(u64::MAX.wrapping_add(1), PhysicsGenerationV1::NEVER_CORRECTED.as_legacy_counter_v1());
    }

    /// A report from before the newest correction is refused, and one
    /// from a generation the server never issued is refused too.
    #[test]
    fn only_the_current_generation_is_eligible() {
        let mut server = PhysicsCorrectionStateV1::new();
        let stamp = |n| PhysicsStampV1 { generation: PhysicsGenerationV1::from_legacy_counter_v1(n) };

        assert_eq!(server.admit_report_v1(stamp(0)), PhysicsAdmitV1::Eligible);

        server.force_correction_v1().unwrap();
        assert_eq!(server.generation().as_legacy_counter_v1(), 1);
        assert_eq!(server.admit_report_v1(stamp(0)), PhysicsAdmitV1::StaleGeneration { server: 1, got: 0 });
        assert_eq!(server.admit_report_v1(stamp(1)), PhysicsAdmitV1::Eligible);
        assert_eq!(server.admit_report_v1(stamp(2)), PhysicsAdmitV1::ForgedGeneration { server: 1, got: 2 });
    }

    /// The acceptance criterion, stated directly: no corrected-away
    /// prediction can replay into a newer generation.
    #[test]
    fn corrected_away_predictions_cannot_replay_into_a_newer_generation() {
        let mut history: PredictionHistoryV1<u32> = PredictionHistoryV1::new(8);
        for step in 0..5 {
            history.push_v1(step);
        }
        assert_eq!(history.len(), 5);
        assert_eq!(history.replayable_v1().count(), 5);

        // the server forces a correction
        let mut server = PhysicsCorrectionStateV1::new();
        let corrected = server.force_correction_v1().unwrap();
        let dropped = history.adopt_generation_v1(corrected);
        assert_eq!(dropped, 5, "every pre-correction prediction is invalidated");
        assert!(history.is_empty());
        assert_eq!(history.replayable_v1().count(), 0);

        // predictions made after the correction are replayable again
        history.push_v1(99);
        assert_eq!(history.replayable_v1().copied().collect::<Vec<_>>(), vec![99]);

        // adopting the same or an older generation changes nothing
        assert_eq!(history.adopt_generation_v1(corrected), 0);
        assert_eq!(history.adopt_generation_v1(PhysicsGenerationV1::NEVER_CORRECTED), 0);
        assert_eq!(history.replayable_v1().count(), 1);
    }

    /// Reconnect: a client that comes back mid-stream adopts whatever
    /// generation the server is on, and its old history does not survive
    /// the transition.
    #[test]
    fn a_generation_transition_across_reconnect_drops_the_old_history() {
        let mut server = PhysicsCorrectionStateV1::new();
        for _ in 0..3 {
            server.force_correction_v1().unwrap();
        }
        assert_eq!(server.generation().as_legacy_counter_v1(), 3);

        // the client was predicting under generation 0 and never saw the
        // corrections
        let mut history: PredictionHistoryV1<u32> = PredictionHistoryV1::new(8);
        history.push_v1(1);
        history.push_v1(2);
        assert_eq!(
            server.admit_report_v1(PhysicsStampV1 { generation: history.generation() }),
            PhysicsAdmitV1::StaleGeneration { server: 3, got: 0 }
        );

        // on resume it adopts the server's generation
        assert_eq!(history.adopt_generation_v1(server.generation()), 2);
        assert_eq!(history.generation(), server.generation());
        assert_eq!(
            server.admit_report_v1(PhysicsStampV1 { generation: history.generation() }),
            PhysicsAdmitV1::Eligible
        );
    }

    /// `T3.6` step 2 rests on this: swapping the raw `u64` for the typed
    /// generation on the wire must not change a single byte, or the
    /// "typed, not reformatted" claim is false and the change needs a
    /// protocol bump it was not given.
    #[test]
    fn the_typed_generation_serialises_exactly_like_the_counter_it_replaces() {
        for counter in [0u64, 1, 7, 4096, u64::MAX] {
            let generation = PhysicsGenerationV1::from_legacy_counter_v1(counter);
            let typed = bincode::serde::encode_to_vec(generation, bincode::config::legacy())
                .expect("generation encodes");
            let raw = bincode::serde::encode_to_vec(counter, bincode::config::legacy())
                .expect("counter encodes");
            assert_eq!(typed, raw, "counter {counter} must be byte-identical on the wire");
        }
    }

    /// Legacy conversion is lossless in both directions during the
    /// migration window.
    #[test]
    fn legacy_counter_conversion_round_trips() {
        for counter in [0u64, 1, 7, u64::MAX] {
            let generation = PhysicsGenerationV1::from_legacy_counter_v1(counter);
            assert_eq!(generation.as_legacy_counter_v1(), counter);
        }
        assert_eq!(
            PhysicsCorrectionStateV1::from_legacy_counter_v1(9).generation().as_legacy_counter_v1(),
            9
        );
    }
}
