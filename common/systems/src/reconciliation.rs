//! `APEX-T7.3c-ii` — wiring the divergence metric (`T7.3c-i`) and the
//! replay primitive (`T7.3b`) together against a real `CompSync`
//! arrival.
//!
//! **The composition, stated once because it is the whole row:** a
//! `CompSync(N)` first TRIMS every buffered frame whose baseline is
//! older than `N` — their inputs are already reflected in `N`'s own
//! snapshot, so replaying them again would double-apply an
//! already-acknowledged input, regardless of agree/diverge. Only THEN
//! does the quantization metric decide whether anything further is
//! needed: if the client's current rolling state already agrees with
//! `N` within tolerance, nothing else happens — the trim was the whole
//! correction. If it diverges, every remaining (unacknowledged) frame
//! must be replayable or the buffer snaps; if all are replayable, they
//! are replayed forward from `N`'s fresh, verbatim baseline, in
//! `alignment.ordinal` order (the buffer's own insertion order, per
//! `ClientPredictionBufferV1::unacknowledged_v1`'s doc).
//!
//! This module is deliberately NOT a `Client` method. Every function
//! here takes its inputs as parameters, matching `character_behavior::
//! replay_predicted_frame_v1`'s own shape — `Client` has no headless
//! test harness (verified before this was written: its one `#[cfg(test)]`
//! block attempts a real TCP connection and silently no-ops when it
//! fails), so the decision logic lives where it can be tested directly,
//! and the actual `CompSync` handler in `client/src/lib.rs` becomes a
//! thin call site supplying live fields to these functions.

use crate::character_behavior::{ReadData, RollingStateV1, replay_predicted_frame_v1};
use common::{
    apex::{
        prediction_boundary::{ClientPredictionBufferV1, NotReplayableV1, ReplayContextV1},
        reconciliation_metric::{ComparableStateV1, DivergenceReasonV1, check_agreement_v1},
        weather_snapshot::WeatherSnapshotIdV1,
    },
    comp::character_state::{CharacterStateEventSinkV1, OutputEvents},
    uid::IdMaps,
};
use specs::{Entity, Read};
use vek::Vec2;

/// A [`RollingStateV1`] and a [`ComparableStateV1`] carry the identical
/// seven fields — this is the one place that says so, rather than
/// leaving the two shapes to drift in silent agreement.
fn comparable_v1(rolling: &RollingStateV1) -> ComparableStateV1 {
    ComparableStateV1 {
        char_state: rolling.char_state.clone(),
        character_activity: rolling.character_activity.clone(),
        pos: rolling.pos,
        vel: rolling.vel,
        ori: rolling.ori,
        density: rolling.density,
        energy: rolling.energy,
    }
}

/// What happened when a `CompSync(sync_tick)` was reconciled against the
/// client's prediction buffer. In every variant the buffer has already
/// been trimmed of acknowledged-by-implication entries by the time this
/// is returned — `trimmed` reports how many, in whichever variant it
/// appears.
#[derive(Debug)]
pub enum ReconciliationOutcomeV1 {
    /// The client's own current rolling state already agreed with the
    /// authoritative snapshot within the reviewed tolerances — no
    /// replay was needed. The authoritative values already stand,
    /// written verbatim by `apply_comp_sync_package` before this ever
    /// ran (the LAW's write-verbatim half, already true by
    /// construction).
    Agreed { trimmed: usize },
    /// Diverged, and every unacknowledged frame was replayable — they
    /// were replayed forward from the fresh authoritative baseline.
    /// `final_rolling` is what the caller should write into the live
    /// ECS components, superseding the raw authoritative snapshot with
    /// the client's own not-yet-acknowledged inputs re-applied on top
    /// of it. `sink_counts` is the throwaway event sink's
    /// captured-and-discarded counts, summed across every replayed
    /// frame — never delivered to a live bus, counted so the discard is
    /// never silent.
    Replayed {
        trimmed: usize,
        reason: DivergenceReasonV1,
        replayed: usize,
        final_rolling: RollingStateV1,
        sink_counts: Vec<(&'static str, usize)>,
    },
    /// Diverged, and at least one unacknowledged frame was NOT
    /// replayable (`WorldRevisionV1::replayable_against_v1`'s
    /// first-reason). The buffer is cleared entirely — a stale frame
    /// blocking replay makes every frame after it suspect too, since
    /// they were predicted assuming the blocked one would replay
    /// cleanly. The authoritative snapshot, already written verbatim,
    /// stands as-is; nothing further is written by this outcome.
    Snapped { trimmed: usize, divergence: DivergenceReasonV1, not_replayable: NotReplayableV1 },
}

/// Reconcile the client's prediction buffer against one `CompSync`'s
/// authoritative snapshot for `entity`.
///
/// `current_rolling` is the client's own belief about its state RIGHT
/// NOW, before this `CompSync` arrived — the caller's job to have
/// tracked (typically: whatever the live ECS components already say,
/// since the client predicts every tick regardless of network
/// arrivals). `authoritative` is the just-applied `CompSync` snapshot,
/// read back from the same live components AFTER
/// `apply_comp_sync_package` wrote it verbatim.
#[expect(clippy::too_many_arguments)]
pub fn reconcile_v1(
    read_data: &ReadData,
    id_maps: &Read<IdMaps>,
    entity: Entity,
    buffer: &mut ClientPredictionBufferV1,
    current_rolling: &RollingStateV1,
    authoritative: &RollingStateV1,
    sync_tick: u64,
    chunk_is_loaded: impl Fn(Vec2<i32>) -> bool,
    weather_is_retained: impl Fn(WeatherSnapshotIdV1) -> bool,
) -> ReconciliationOutcomeV1 {
    let trimmed = buffer.trim_acknowledged_v1(sync_tick);

    let current_comparable = comparable_v1(current_rolling);
    let authoritative_comparable = comparable_v1(authoritative);

    let reason = match check_agreement_v1(&current_comparable, &authoritative_comparable) {
        Ok(()) => return ReconciliationOutcomeV1::Agreed { trimmed },
        Err(reason) => reason,
    };

    // Collected up front rather than iterated in place: the loop below
    // needs `buffer.clear_v1()` (a mutable borrow) reachable from inside
    // a loop over its own contents, which an in-place iterator borrow
    // does not allow. `replay_predicted_frame_v1` only needs each
    // frame's own data, never the buffer itself, so nothing is lost by
    // detaching the frames from it first.
    let frames: Vec<_> = buffer.unacknowledged_v1(sync_tick).cloned().collect();

    for frame in &frames {
        if let Err(not_replayable) =
            frame.world_revision.replayable_against_v1(&chunk_is_loaded, &weather_is_retained)
        {
            buffer.clear_v1();
            return ReconciliationOutcomeV1::Snapped { trimmed, divergence: reason, not_replayable };
        }
    }

    let mut rolling = authoritative.clone();
    let mut sink_counts: Vec<(&'static str, usize)> = Vec::new();
    let mut replayed = 0usize;
    for frame in &frames {
        let sink = CharacterStateEventSinkV1::default();
        {
            let mut emitters = sink.emitters();
            let mut local_events = Vec::new();
            let mut output_events = OutputEvents::new(&mut local_events, &mut emitters);
            replay_predicted_frame_v1(
                read_data,
                id_maps,
                entity,
                &mut rolling,
                frame,
                &mut output_events,
                ReplayContextV1,
            );
        }
        sink_counts.extend(sink.drain_counts_v1());
        replayed += 1;
    }

    ReconciliationOutcomeV1::Replayed { trimmed, reason, replayed, final_rolling: rolling, sink_counts }
}
