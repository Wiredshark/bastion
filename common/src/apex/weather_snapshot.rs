//! `APEX-T5.4` — tick-owned weather input.
//!
//! Wall-clock receipt timing must not change gameplay-prediction wind
//! state.
//!
//! **The failure, read at tip.** `WeatherLerp::update_local_wind`
//! (`client/src/lib.rs`) lerps on `Instant::elapsed()` since a packet
//! arrived, over a denominator that is the interval between the last two
//! ARRIVALS — its own comment concedes `// Assumes updates are regular`.
//! Under jitter that assumption fails, and `local_wind` reaches glider
//! steering through air flow (`common/src/states/glide.rs`, the
//! `lateral_wind_speed` slerp). Two clients receiving identical weather
//! packets with different jitter therefore predict different glides.
//!
//! **The split is by PURPOSE, and it is enforced by the type system
//! rather than by a rule.** [`PredictionWindV1`] carries the snapshot it
//! came from; [`PresentationWindV1`] does not and cannot acquire one.
//! There is no `From` between them, no comparison between them, and no
//! accessor that turns one into the other. A caller cannot politely be
//! asked to keep them apart and then forget — the conversion does not
//! exist. A `compile_fail` doctest pins that, because a missing `impl` is
//! the kind of guarantee that gets added back by a well-meaning patch.
//!
//! **A missing snapshot is [`PredictionWindSourceV1::Unavailable`], not
//! an extrapolation.** Extrapolating is exactly how the wall-clock
//! dependency gets back in through a side door: an extrapolation's input
//! is elapsed time. The caller must snap or go non-predictive.
//!
//! **Scope.** This row lands the types, the store and their laws. The
//! live client reroute is NOT done here and is recorded in
//! [`WEATHER_PREDICTION_LEAKS`]: it needs the snapshot id to travel on
//! the wire, which is `T5.2`'s environment reference and a wire-version
//! bump that batches at the tier boundary. Nothing below should be read
//! as saying the live glider path is fixed — it is not, and the canary
//! says where it still leaks.

use serde::{Deserialize, Serialize};
use vek::Vec2;

/// Identity of one authoritative weather snapshot.
///
/// Server-issued and monotone. Opaque: there is no arithmetic on it,
/// because "the snapshot two before this one" is not a thing a consumer
/// is entitled to compute — it either has the snapshot or it does not.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeatherSnapshotIdV1(u64);

impl WeatherSnapshotIdV1 {
    pub const fn from_sequence_v1(sequence: u64) -> Self { Self(sequence) }

    pub const fn sequence_v1(self) -> u64 { self.0 }
}

/// Wind that MAY drive prediction.
///
/// Carries the snapshot it came from, so a replay uses the same wind the
/// original did. Construction requires a snapshot id, which is what makes
/// "prediction wind with no provenance" unrepresentable.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PredictionWindV1 {
    snapshot: WeatherSnapshotIdV1,
    wind: Vec2<f32>,
}

impl PredictionWindV1 {
    pub const fn new_v1(snapshot: WeatherSnapshotIdV1, wind: Vec2<f32>) -> Self {
        Self { snapshot, wind }
    }

    pub const fn snapshot_v1(self) -> WeatherSnapshotIdV1 { self.snapshot }

    pub const fn wind_v1(self) -> Vec2<f32> { self.wind }
}

/// Wind for rendering only.
///
/// Receipt-time interpolated, exactly as today, and barred from the
/// prediction path by having no route into [`PredictionWindV1`]. Two
/// clients are EXPECTED to disagree here; that is what it is for.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PresentationWindV1 {
    wind: Vec2<f32>,
}

impl PresentationWindV1 {
    pub const fn new_v1(wind: Vec2<f32>) -> Self { Self { wind } }

    pub const fn wind_v1(self) -> Vec2<f32> { self.wind }
}

/// The result of asking for the prediction wind at a snapshot.
///
/// ```compile_fail
/// # use veloren_common::apex::weather_snapshot::*;
/// # use vek::Vec2;
/// let presentation = PresentationWindV1::new_v1(Vec2::new(1.0, 0.0));
/// // T5.4: there is deliberately no conversion. If this ever compiles,
/// // presentation wind can reach the prediction path again and the row
/// // has been undone.
/// let prediction: PredictionWindV1 = presentation.into();
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PredictionWindSourceV1 {
    /// The snapshot is retained and its wind is available.
    Snapshot(PredictionWindV1),
    /// The snapshot is not retained. The caller must snap or go
    /// non-predictive. It must NOT extrapolate: an extrapolation's input
    /// is elapsed wall-clock time, which is the dependency this row
    /// exists to remove.
    Unavailable,
}

impl PredictionWindSourceV1 {
    pub const fn wind_v1(self) -> Option<Vec2<f32>> {
        match self {
            Self::Snapshot(wind) => Some(wind.wind_v1()),
            Self::Unavailable => None,
        }
    }
}

/// Recent authoritative snapshots, retained by id.
///
/// A bounded ring: retaining everything would make a long-disconnected
/// client's replay succeed against a snapshot nobody else still has, and
/// a silent unbounded buffer is its own bug. Eviction is by lowest id, so
/// what falls out is what is oldest, not what was touched least recently
/// — the latter would make retention depend on access timing, which is
/// the same class of dependency as the one being removed.
#[derive(Clone, Debug)]
pub struct WeatherSnapshotStoreV1 {
    capacity: usize,
    snapshots: Vec<(WeatherSnapshotIdV1, Vec2<f32>)>,
}

impl WeatherSnapshotStoreV1 {
    pub fn new_v1(capacity: usize) -> Self {
        Self { capacity: capacity.max(1), snapshots: Vec::new() }
    }

    /// Record an authoritative snapshot. Re-recording an id REPLACES it:
    /// the server is the authority on what a snapshot contains, and a
    /// store that kept the first value would let a correction be ignored.
    pub fn record_v1(&mut self, id: WeatherSnapshotIdV1, wind: Vec2<f32>) {
        match self.snapshots.iter_mut().find(|(existing, _)| *existing == id) {
            Some(slot) => slot.1 = wind,
            None => {
                self.snapshots.push((id, wind));
                self.snapshots.sort_by_key(|(id, _)| *id);
                while self.snapshots.len() > self.capacity {
                    self.snapshots.remove(0);
                }
            },
        }
    }

    /// The prediction wind for a snapshot, or `Unavailable`.
    ///
    /// Takes no time argument at all. That is the row's acceptance
    /// criterion expressed as a signature: a function that cannot see the
    /// clock cannot depend on it.
    pub fn wind_at_v1(&self, id: WeatherSnapshotIdV1) -> PredictionWindSourceV1 {
        self.snapshots
            .iter()
            .find(|(existing, _)| *existing == id)
            .map(|(id, wind)| PredictionWindSourceV1::Snapshot(PredictionWindV1::new_v1(*id, *wind)))
            .unwrap_or(PredictionWindSourceV1::Unavailable)
    }

    pub fn retained_v1(&self) -> usize { self.snapshots.len() }

    pub fn oldest_retained_v1(&self) -> Option<WeatherSnapshotIdV1> {
        self.snapshots.first().map(|(id, _)| *id)
    }
}

/// Where receipt-time wind still reaches prediction in the live client.
///
/// Recorded rather than implied: this row lands the types, not the
/// reroute, and an artifact that said nothing here would read as though
/// the leak were closed.
pub const WEATHER_PREDICTION_LEAKS: &[(&str, &str)] = &[
    (
        "client/src/lib.rs::WeatherLerp::update_local_wind",
        "lerps on Instant::elapsed() since packet arrival and writes the result into every \
         weather cell's wind; still live, and still the source of the prediction input",
    ),
    (
        "common/src/states/glide.rs (lateral_wind_speed slerp)",
        "consumes air flow derived from that wind to steer the glider, so jitter between two \
         clients becomes a different predicted glide",
    ),
];

/// What the live reroute is waiting on. A value, not a comment, so `T5.2`
/// cannot assume this row already moved the client.
pub const SNAPSHOT_ID_TRAVELS_ON_THE_WIRE: bool = false;

#[cfg(test)]
mod weather_snapshot_v1 {
    use super::*;

    fn id(n: u64) -> WeatherSnapshotIdV1 { WeatherSnapshotIdV1::from_sequence_v1(n) }

    /// A receipt-time interpolator, exactly the shape the client has
    /// today: `t` is elapsed-since-arrival over the interval between the
    /// last two arrivals. Used to MODEL jitter so the row's acceptance
    /// criterion can be stated as a test.
    fn presentation_wind(
        old: Vec2<f32>,
        new: Vec2<f32>,
        elapsed_since_arrival: f32,
        interval_between_arrivals: f32,
    ) -> PresentationWindV1 {
        let t = (elapsed_since_arrival / interval_between_arrivals).clamp(0.0, 1.0);
        PresentationWindV1::new_v1(Vec2::lerp_unclamped(old, new, t))
    }

    /// **The row's acceptance criterion.** Receipt delay varies wildly;
    /// the snapshot sequence does not; the prediction inputs are equal.
    ///
    /// The presentation winds computed from the same delays are also
    /// checked to actually DIFFER, so the test cannot pass by the jitter
    /// having had no effect — that would make the green a lottery.
    #[test]
    fn varying_receipt_delay_does_not_change_the_prediction_input() {
        let mut store = WeatherSnapshotStoreV1::new_v1(8);
        store.record_v1(id(1), Vec2::new(1.0, 0.0));
        store.record_v1(id(2), Vec2::new(3.0, 0.0));

        let baseline = store.wind_at_v1(id(2));
        let mut presentation = Vec::new();
        for (elapsed, interval) in [(0.001, 0.1), (0.05, 0.1), (0.099, 0.1), (0.4, 0.5)] {
            // Same snapshot sequence, wildly different receipt timing.
            assert_eq!(
                store.wind_at_v1(id(2)),
                baseline,
                "the prediction input moved with receipt timing"
            );
            presentation.push(presentation_wind(
                Vec2::new(1.0, 0.0),
                Vec2::new(3.0, 0.0),
                elapsed,
                interval,
            ));
        }

        assert!(
            presentation.windows(2).any(|pair| pair[0] != pair[1]),
            "the modelled jitter had no effect on presentation wind either, so this test would \
             pass against a broken split"
        );
    }

    /// Two clients may disagree about presentation wind and may not
    /// disagree about prediction wind. Stated as one test because the two
    /// halves are the same claim.
    #[test]
    fn presentation_may_differ_between_clients_while_prediction_may_not() {
        let mut a = WeatherSnapshotStoreV1::new_v1(4);
        let mut b = WeatherSnapshotStoreV1::new_v1(4);
        a.record_v1(id(7), Vec2::new(2.0, -1.0));
        b.record_v1(id(7), Vec2::new(2.0, -1.0));

        assert_eq!(a.wind_at_v1(id(7)), b.wind_at_v1(id(7)));

        let client_a = presentation_wind(Vec2::zero(), Vec2::new(2.0, -1.0), 0.01, 0.1);
        let client_b = presentation_wind(Vec2::zero(), Vec2::new(2.0, -1.0), 0.09, 0.1);
        assert_ne!(client_a, client_b, "presentation wind is allowed — expected — to differ");
    }

    /// A dropped snapshot produces the fallback, not an interpolation.
    /// `wind_v1` returns `None` rather than a plausible number, because a
    /// plausible number is what a caller silently uses.
    #[test]
    fn a_dropped_snapshot_produces_the_fallback_not_an_interpolation() {
        let mut store = WeatherSnapshotStoreV1::new_v1(2);
        store.record_v1(id(1), Vec2::new(1.0, 0.0));
        store.record_v1(id(2), Vec2::new(2.0, 0.0));
        store.record_v1(id(3), Vec2::new(3.0, 0.0));

        // 1 was evicted. Its neighbours are both present, which is
        // exactly when an interpolating implementation would invent a
        // value for it.
        assert_eq!(store.wind_at_v1(id(1)), PredictionWindSourceV1::Unavailable);
        assert_eq!(store.wind_at_v1(id(1)).wind_v1(), None);
        assert_eq!(store.oldest_retained_v1(), Some(id(2)));
    }

    /// A snapshot never seen at all is also `Unavailable` — the store
    /// does not distinguish "evicted" from "never arrived", because the
    /// caller's correct action is the same for both and a distinction
    /// invites a different one.
    #[test]
    fn a_snapshot_that_never_arrived_is_unavailable_too() {
        let store = WeatherSnapshotStoreV1::new_v1(4);
        assert_eq!(store.wind_at_v1(id(99)), PredictionWindSourceV1::Unavailable);
    }

    /// Eviction is by lowest id, not by access. Retention that depended
    /// on access timing would be the same class of wall-clock dependency
    /// this row removes, one level up.
    #[test]
    fn eviction_is_by_age_not_by_access() {
        let mut store = WeatherSnapshotStoreV1::new_v1(3);
        for n in 1..=3 {
            store.record_v1(id(n), Vec2::new(n as f32, 0.0));
        }
        // Touch the oldest repeatedly; it must still be the one evicted.
        for _ in 0..5 {
            let _ = store.wind_at_v1(id(1));
        }
        store.record_v1(id(4), Vec2::new(4.0, 0.0));

        assert_eq!(store.wind_at_v1(id(1)), PredictionWindSourceV1::Unavailable);
        assert_eq!(store.oldest_retained_v1(), Some(id(2)));
        assert_eq!(store.retained_v1(), 3);
    }

    /// Snapshots arriving out of order are retained in id order, so what
    /// falls out of a full store does not depend on arrival order either.
    #[test]
    fn out_of_order_arrival_does_not_change_what_is_retained() {
        let mut forwards = WeatherSnapshotStoreV1::new_v1(2);
        let mut backwards = WeatherSnapshotStoreV1::new_v1(2);
        for n in [1u64, 2, 3] {
            forwards.record_v1(id(n), Vec2::new(n as f32, 0.0));
        }
        for n in [3u64, 2, 1] {
            backwards.record_v1(id(n), Vec2::new(n as f32, 0.0));
        }

        assert_eq!(forwards.oldest_retained_v1(), backwards.oldest_retained_v1());
        assert_eq!(forwards.wind_at_v1(id(3)), backwards.wind_at_v1(id(3)));
        assert_eq!(forwards.wind_at_v1(id(1)), PredictionWindSourceV1::Unavailable);
        assert_eq!(backwards.wind_at_v1(id(1)), PredictionWindSourceV1::Unavailable);
    }

    /// The server is the authority on a snapshot's contents: re-recording
    /// an id replaces it. A store that kept the first value would let a
    /// correction be silently ignored.
    #[test]
    fn re_recording_a_snapshot_replaces_it() {
        let mut store = WeatherSnapshotStoreV1::new_v1(4);
        store.record_v1(id(5), Vec2::new(1.0, 1.0));
        store.record_v1(id(5), Vec2::new(9.0, 9.0));

        assert_eq!(store.retained_v1(), 1, "the correction was stored as a second snapshot");
        assert_eq!(
            store.wind_at_v1(id(5)).wind_v1(),
            Some(Vec2::new(9.0, 9.0)),
            "the correction was ignored"
        );
    }

    /// Prediction wind keeps the snapshot it was predicted UNDER, so a
    /// replay can use the same wind the original did rather than
    /// whatever is current.
    #[test]
    fn prediction_wind_carries_its_provenance() {
        let mut store = WeatherSnapshotStoreV1::new_v1(4);
        store.record_v1(id(11), Vec2::new(4.0, 0.0));
        store.record_v1(id(12), Vec2::new(5.0, 0.0));

        let PredictionWindSourceV1::Snapshot(predicted) = store.wind_at_v1(id(11)) else {
            panic!("snapshot 11 is retained");
        };
        assert_eq!(predicted.snapshot_v1(), id(11));
        // Later snapshots do not retroactively change what was predicted.
        assert_eq!(predicted.wind_v1(), Vec2::new(4.0, 0.0));
    }

    /// The live leak is recorded with its location, and the row does not
    /// claim to have closed it.
    #[test]
    fn the_live_leak_is_named_and_not_claimed_closed() {
        assert!(
            !SNAPSHOT_ID_TRAVELS_ON_THE_WIRE,
            "if the snapshot id now travels on the wire, the client reroute is unblocked and \
             WEATHER_PREDICTION_LEAKS must be re-derived rather than left stale"
        );
        assert_eq!(WEATHER_PREDICTION_LEAKS.len(), 2);
        for (site, why) in WEATHER_PREDICTION_LEAKS {
            assert!(site.contains("::") || site.contains(".rs"), "{site} is not a location");
            assert!(why.len() > 40, "{site} is asserted rather than evidenced: {why:?}");
        }
    }
}
