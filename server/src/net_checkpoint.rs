//! `APEX-T3.4.09` — the server checkpoint planner: one frozen plan per
//! recipient per flush. Pure over a frozen intent set; the T3.3 egress
//! seam supplies the intents and consumes the plan. Dormant until
//! activation, like the T3.3 V1 path it builds on.

use common::apex::digest::DigestBytes32V1;
use common_net::msg::ServerGeneral;
use common_net::msg::checkpoint::{
    CheckpointApplyPhaseV1, CheckpointDescriptorV1, CheckpointOrdinalV1, CheckpointParticipantV1,
    CheckpointParticipationV1, CheckpointResourceProfileV1, CheckpointedEnvelopeContextV1,
    REQUIRED_CHECKPOINT_STREAMS_V1, StreamCheckpointPlanV1, TranscriptEntryV1,
    global_transcript_root_v1, stream_transcript_root_v1, validate_checkpoint_context_v1,
};
use common_net::msg::envelope::{ActiveSessionBindingV1, SemanticRouteV1, SemanticSendStateV1, SemanticStreamIdV1};
use crate::semantic_net::outbox::SemanticSendIntentV1;
use std::num::NonZeroU64;
use std::sync::Arc;

#[derive(Debug)]
pub enum CheckpointPlanErrorV1 {
    ProducerAfterFlush,
    DuplicateOrderKey,
    PlanInvariant(&'static str),
    ResourceExceeded(&'static str),
    RootFailure,
    SequenceExhausted,
}

/// One recipient's complete checkpoint: the descriptor plus the ordered
/// data records, ready for Begin/Data/Barrier emission.
#[derive(Debug)]
pub struct RecipientCheckpointPlanV1 {
    pub recipient: ActiveSessionBindingV1,
    pub descriptor: CheckpointDescriptorV1,
    pub descriptor_root: [u8; 32],
    /// (ordinal, stream, sequence) in ordinal order.
    pub records: Vec<(CheckpointOrdinalV1, SemanticStreamIdV1, u64)>,
    /// The admitted intents in ordinal order: index `n` is ordinal `n+1`,
    /// so frames are derivable from the plan alone.
    pub intents: Vec<SemanticSendIntentV1>,
}

fn payload_digest_of(intent: &SemanticSendIntentV1) -> [u8; 32] {
    let bytes = common_net::msg::envelope::encode_payload_v1(&*intent.payload);
    let d = common_net::msg::envelope::payload_digest_v1(
        common_net::msg::envelope::net_envelope_profile_root_v1(),
        intent.payload.payload_schema(),
        common_net::msg::envelope::SemanticPayloadEncodingV1::Bincode2LegacySerde,
        &bytes,
    );
    *d.as_array()
}

/// Slot of a stream in `REQUIRED_CHECKPOINT_STREAMS_V1`, which is also
/// its cursor slot in `SemanticSendStateV1` — the two orders agree, and
/// `cursor_slots_match_the_required_stream_order` pins that.
fn stream_slot_v1(stream: SemanticStreamIdV1) -> usize {
    REQUIRED_CHECKPOINT_STREAMS_V1.iter().position(|s| *s == stream).expect("the five streams are total")
}

/// Sequence-INDEPENDENT admission: every reject that must land before a
/// cursor moves, so a refused checkpoint never consumes sequences.
/// Returns the sorted intents plus per-stream record counts and bytes.
fn admit_intent_set_v1(
    mut intents: Vec<SemanticSendIntentV1>,
    profile: &CheckpointResourceProfileV1,
) -> Result<(Vec<SemanticSendIntentV1>, [u32; 5], [u64; 5]), CheckpointPlanErrorV1> {
    use CheckpointPlanErrorV1 as E;

    intents.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
    if intents.windows(2).any(|w| w[0].total_sort_key() == w[1].total_sort_key()) {
        return Err(E::DuplicateOrderKey);
    }
    // Control/diagnostic payloads never carry an ordinal, so they are not
    // checkpoint DATA and must not enter the transcript.
    if intents
        .iter()
        .any(|i| i.payload.participation_v1() != CheckpointParticipationV1::CheckpointedData)
    {
        return Err(E::PlanInvariant("non-data payload in checkpoint intent set"));
    }

    let mut counts = [0u32; 5];
    let mut bytes = [0u64; 5];
    for intent in &intents {
        let slot = stream_slot_v1(intent.semantic_stream);
        counts[slot] += 1;
        bytes[slot] += common_net::msg::envelope::encode_payload_v1(&*intent.payload).len() as u64;
    }
    for slot in 0..5 {
        if bytes[slot] > profile.max_payload_bytes_per_stream[slot] {
            return Err(E::ResourceExceeded("payload_bytes_per_stream"));
        }
    }
    if intents.len() as u64 > u64::from(profile.max_records_per_checkpoint) {
        return Err(E::ResourceExceeded("records_per_checkpoint"));
    }
    if bytes.iter().sum::<u64>() > profile.max_payload_bytes_per_checkpoint {
        return Err(E::ResourceExceeded("payload_bytes_per_checkpoint"));
    }
    Ok((intents, counts, bytes))
}

/// Per-stream sequence demand of a checkpoint: Begin + data + Barrier.
/// Every stream is fenced, so even an empty one demands two.
pub fn sequence_demand_v1(counts: &[u32; 5]) -> [u64; 5] { std::array::from_fn(|i| u64::from(counts[i]) + 2) }

/// `T3.4.10`: reserve the whole plan's sequences from the live send
/// cursors, then plan against them. Admission runs FIRST, so a rejected
/// checkpoint leaves every cursor untouched; the reservation itself is
/// all-or-nothing. Past the reservation only a root failure can abort,
/// and that range is then burned rather than reused (T3.3's own "a
/// sequence is consumed before send and never reused after failure").
pub fn reserve_and_plan_recipient_checkpoint_v1(
    send_state: &mut SemanticSendStateV1,
    epoch: u64,
    parent_epoch: u64,
    intents: Vec<SemanticSendIntentV1>,
    profile: &CheckpointResourceProfileV1,
    apply_policy_root: [u8; 32],
    egress_order_policy_root: [u8; 32],
) -> Result<RecipientCheckpointPlanV1, CheckpointPlanErrorV1> {
    let recipient = send_state.binding();
    let (intents, counts, _) = admit_intent_set_v1(intents, profile)?;
    let first_sequences = send_state
        .reserve_sequences_v1(sequence_demand_v1(&counts))
        .map_err(|_| CheckpointPlanErrorV1::SequenceExhausted)?;
    plan_recipient_checkpoint_v1(
        recipient,
        epoch,
        parent_epoch,
        intents,
        first_sequences,
        profile,
        apply_policy_root,
        egress_order_policy_root,
    )
}

/// Plan ONE recipient from its frozen intents. Ordinals follow the T3.3
/// total sort key, so the plan is a pure function of the intent SET.
/// Each stream's sequences run contiguously from its OWN reserved base
/// in `first_sequences`: Begin, that stream's data, Barrier.
pub fn plan_recipient_checkpoint_v1(
    recipient: ActiveSessionBindingV1,
    epoch: u64,
    parent_epoch: u64,
    intents: Vec<SemanticSendIntentV1>,
    first_sequences: [NonZeroU64; 5],
    profile: &CheckpointResourceProfileV1,
    apply_policy_root: [u8; 32],
    egress_order_policy_root: [u8; 32],
) -> Result<RecipientCheckpointPlanV1, CheckpointPlanErrorV1> {
    use CheckpointPlanErrorV1 as E;

    let (intents, _, _) = admit_intent_set_v1(intents, profile)?;

    // Ordinals are global across the recipient, assigned in sort order.
    let ordered: Vec<(CheckpointOrdinalV1, &SemanticSendIntentV1)> = intents
        .iter()
        .enumerate()
        .map(|(i, intent)| (CheckpointOrdinalV1(i as u64 + 1), intent))
        .collect();

    let mut stream_plans: Vec<StreamCheckpointPlanV1> = Vec::with_capacity(5);
    let mut records: Vec<(CheckpointOrdinalV1, SemanticStreamIdV1, u64)> = Vec::with_capacity(ordered.len());
    let mut all_entries: Vec<TranscriptEntryV1> = Vec::with_capacity(ordered.len());
    let mut total_bytes = 0u64;

    for (idx, stream) in REQUIRED_CHECKPOINT_STREAMS_V1.into_iter().enumerate() {
        let mut seq = first_sequences[idx].get();
        let begin_sequence = seq;
        seq += 1;
        let mut entries: Vec<TranscriptEntryV1> = Vec::new();
        let mut first_data = None;
        let mut last_data = None;
        let mut bytes = 0u64;
        for (ordinal, intent) in ordered.iter().filter(|(_, i)| i.semantic_stream == stream) {
            let this = seq;
            seq += 1;
            first_data.get_or_insert(this);
            last_data = Some(this);
            let payload_bytes = common_net::msg::envelope::encode_payload_v1(&*intent.payload).len() as u64;
            bytes += payload_bytes;
            let e = TranscriptEntryV1 {
                sequence: this,
                ordinal: *ordinal,
                payload_kind: intent.payload.payload_schema() as u16,
                payload_digest: payload_digest_of(intent),
            };
            entries.push(e.clone());
            all_entries.push(e);
            records.push((*ordinal, stream, this));
        }
        let barrier_sequence = seq;

        total_bytes += bytes;
        stream_plans.push(StreamCheckpointPlanV1 {
            stream,
            begin_sequence,
            first_data_sequence: first_data,
            last_data_sequence: last_data,
            barrier_sequence,
            data_record_count: entries.len() as u32,
            payload_bytes: bytes,
            stream_transcript_root: stream_transcript_root_v1(&recipient, epoch, stream, &entries)
                .map_err(|_| E::RootFailure)?,
        });
    }

    // Ceilings were enforced in admission, before any reservation; the
    // declared preflight below re-checks them against the built
    // descriptor.
    let record_count = ordered.len() as u32;
    let descriptor = CheckpointDescriptorV1 {
        schema_version: 1,
        binding: recipient,
        epoch,
        parent_epoch,
        resource_profile_root: profile.profile_root_v1().map_err(|_| E::RootFailure)?,
        apply_policy_root,
        egress_order_policy_root,
        data_record_count: record_count,
        ordinal_max: record_count as u64,
        payload_bytes: total_bytes,
        global_transcript_root: global_transcript_root_v1(&recipient, epoch, &all_entries)
            .map_err(|_| E::RootFailure)?,
        streams: stream_plans.try_into().map_err(|_| E::PlanInvariant("stream plan count"))?,
        bootstrap_manifest_root: None,
    };
    profile
        .admit_descriptor_v1(&descriptor)
        .map_err(|_| E::ResourceExceeded("declared preflight"))?;
    let descriptor_root = descriptor.descriptor_root_v1().map_err(|_| E::RootFailure)?;

    Ok(RecipientCheckpointPlanV1 { recipient, descriptor, descriptor_root, records, intents })
}

/// The apply phase of a planned record, for the client-side aligner.
pub fn record_apply_phase_v1(intent: &SemanticSendIntentV1) -> Option<CheckpointApplyPhaseV1> {
    intent.payload.apply_phase_v1()
}

/// `T3.4.11` — one emitted frame of a recipient's checkpoint. Every
/// frame carries its checkpoint context, so an unbound checkpoint frame
/// is unrepresentable; only Data carries an ordinal.
#[derive(Debug, Clone)]
pub enum CheckpointFrameV1 {
    Begin {
        stream: SemanticStreamIdV1,
        sequence: u64,
        context: CheckpointedEnvelopeContextV1,
    },
    Data {
        stream: SemanticStreamIdV1,
        sequence: u64,
        context: CheckpointedEnvelopeContextV1,
        apply_phase: CheckpointApplyPhaseV1,
        payload: Arc<ServerGeneral>,
    },
    Barrier {
        stream: SemanticStreamIdV1,
        sequence: u64,
        context: CheckpointedEnvelopeContextV1,
    },
}

impl CheckpointFrameV1 {
    pub fn stream(&self) -> SemanticStreamIdV1 {
        match self {
            Self::Begin { stream, .. } | Self::Data { stream, .. } | Self::Barrier { stream, .. } => *stream,
        }
    }

    pub fn sequence(&self) -> u64 {
        match self {
            Self::Begin { sequence, .. } | Self::Data { sequence, .. } | Self::Barrier { sequence, .. } => *sequence,
        }
    }

    pub fn context(&self) -> &CheckpointedEnvelopeContextV1 {
        match self {
            Self::Begin { context, .. } | Self::Data { context, .. } | Self::Barrier { context, .. } => context,
        }
    }

    pub fn participation(&self) -> CheckpointParticipationV1 {
        match self {
            Self::Data { .. } => CheckpointParticipationV1::CheckpointedData,
            _ => CheckpointParticipationV1::CheckpointControl,
        }
    }
}

/// `T3.4.11`: expand a plan into its send-ordered frames — stream by
/// stream in canonical order, and within a stream Begin, its data in
/// ordinal order, Barrier. Each frame's context is validated against
/// the plan's own epoch and descriptor root before it is emitted, so
/// emission cannot produce a frame the receiver would have to reject.
pub fn checkpoint_frames_v1(plan: &RecipientCheckpointPlanV1) -> Result<Vec<CheckpointFrameV1>, CheckpointPlanErrorV1> {
    use CheckpointPlanErrorV1 as E;

    let epoch = plan.descriptor.epoch;
    let control = CheckpointedEnvelopeContextV1 { epoch, ordinal: None, descriptor_root: plan.descriptor_root };
    let mut frames = Vec::with_capacity(plan.records.len() + 10);

    for stream_plan in plan.descriptor.streams.iter() {
        let stream = stream_plan.stream;
        frames.push(CheckpointFrameV1::Begin { stream, sequence: stream_plan.begin_sequence, context: control });
        for (ordinal, record_stream, sequence) in plan.records.iter().filter(|(_, s, _)| *s == stream) {
            let intent = plan
                .intents
                .get(ordinal.0 as usize - 1)
                .ok_or(E::PlanInvariant("record ordinal has no intent"))?;
            if intent.semantic_stream != *record_stream {
                return Err(E::PlanInvariant("record stream disagrees with its intent"));
            }
            frames.push(CheckpointFrameV1::Data {
                stream,
                sequence: *sequence,
                context: CheckpointedEnvelopeContextV1 { epoch, ordinal: Some(*ordinal), descriptor_root: plan.descriptor_root },
                // Data participation is already proven by admission, so a
                // missing phase here is a broken classification, not input.
                apply_phase: record_apply_phase_v1(intent).ok_or(E::PlanInvariant("checkpointed data without an apply phase"))?,
                payload: Arc::clone(&intent.payload),
            });
        }
        frames.push(CheckpointFrameV1::Barrier { stream, sequence: stream_plan.barrier_sequence, context: control });
    }

    for frame in &frames {
        validate_checkpoint_context_v1(frame.participation(), Some(frame.context()), epoch, plan.descriptor_root)
            .map_err(|_| E::PlanInvariant("emitted frame failed its own context validation"))?;
    }
    Ok(frames)
}

/// Unused import guard: the descriptor root is a raw 32-byte value here,
/// converted at the T0.3 boundary by callers that need a typed digest.
pub fn descriptor_root_bytes_v1(root: [u8; 32]) -> DigestBytes32V1 { DigestBytes32V1::from_array(root) }

#[cfg(test)]
mod checkpoint_planner_v1 {
    use super::*;
    use crate::semantic_net::order::{SemanticPayloadRankV1, SemanticProducerV1, phase_rank};
    use crate::semantic_net::outbox::{CanonicalSubjectKeyV1, ServerSemanticOrderKeyV1};
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use common_ecs::Phase;
    use common_net::msg::checkpoint::CheckpointProfilePurposeV1;
    use common_net::msg::envelope::SemanticCausalityV1;

    fn binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn profile() -> CheckpointResourceProfileV1 {
        CheckpointResourceProfileV1 {
            profile_id: "apex-t3-4-planner-test-v1".to_owned(),
            purpose: CheckpointProfilePurposeV1::TestFixture,
            max_records_per_checkpoint: 16,
            max_payload_bytes_per_checkpoint: 1 << 20,
            max_payload_bytes_per_stream: [1 << 20; 5],
            max_staged_events: 16,
            max_prepared_ops: 16,
        }
    }

    fn intent(stream: SemanticStreamIdV1, local_ordinal: u32) -> SemanticSendIntentV1 {
        SemanticSendIntentV1 {
            recipient: binding(),
            semantic_stream: stream,
            causality: SemanticCausalityV1 { producer_tick: Some(1), snapshot: None },
            order_key: ServerSemanticOrderKeyV1 {
                source_tick: 1,
                phase_rank: phase_rank(Phase::Create),
                producer_rank: SemanticProducerV1::EntitySync.producer_rank(),
                payload_rank: SemanticPayloadRankV1::CompSync.payload_rank(),
                subject: CanonicalSubjectKeyV1::for_singleton("planner-test"),
                local_ordinal,
            },
            payload: Arc::new(ServerGeneral::UpdateRecipes),
        }
    }

    const BASE: [NonZeroU64; 5] = [NonZeroU64::new(1).expect("1 is nonzero"); 5];

    #[test]
    fn plan_is_a_pure_function_of_the_intent_set() {
        let mk = || vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::Terrain, 2), intent(SemanticStreamIdV1::InGame, 0)];
        let a = plan_recipient_checkpoint_v1(binding(), 1, 0, mk(), BASE, &profile(), [1; 32], [2; 32]).unwrap();
        let mut reversed = mk();
        reversed.reverse();
        let b = plan_recipient_checkpoint_v1(binding(), 1, 0, reversed, BASE, &profile(), [1; 32], [2; 32]).unwrap();
        assert_eq!(a.descriptor_root, b.descriptor_root, "input order must not move the plan");
        assert_eq!(a.records, b.records);

        assert_eq!(a.descriptor.streams.len(), 5);
        assert_eq!(a.descriptor.data_record_count, 3);
        assert_eq!(a.descriptor.ordinal_max, 3);
        // every stream is fenced: 5 Begins + 5 Barriers + 3 data = 13
        assert_eq!(a.descriptor.streams.iter().map(|s| s.barrier_sequence - s.begin_sequence + 1).sum::<u64>(), 13);
        // per-stream bases: each stream runs from its OWN cursor
        assert!(a.descriptor.streams.iter().all(|s| s.begin_sequence == 1));
        // empty streams still get Begin+Barrier with a typed root
        let empty: Vec<_> = a.descriptor.streams.iter().filter(|s| s.data_record_count == 0).collect();
        assert_eq!(empty.len(), 3);
        assert!(empty.iter().all(|s| s.barrier_sequence == s.begin_sequence + 1));
    }

    #[test]
    fn duplicate_order_key_and_resource_ceiling_are_typed() {
        let dup = vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::InGame, 1)];
        assert!(matches!(
            plan_recipient_checkpoint_v1(binding(), 1, 0, dup, BASE, &profile(), [1; 32], [2; 32]),
            Err(CheckpointPlanErrorV1::DuplicateOrderKey)
        ));

        let mut tight = profile();
        tight.max_records_per_checkpoint = 1;
        let two = vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::InGame, 2)];
        assert!(matches!(
            plan_recipient_checkpoint_v1(binding(), 1, 0, two, BASE, &tight, [1; 32], [2; 32]),
            Err(CheckpointPlanErrorV1::ResourceExceeded(_))
        ));
    }

    /// `T3.4.10`: the reservation is whole-plan, and a REJECTED plan
    /// must not consume a single sequence.
    #[test]
    fn rejected_checkpoints_never_consume_sequences() {
        let cursors = |s: &SemanticSendStateV1| REQUIRED_CHECKPOINT_STREAMS_V1.map(|st| s.next_for(st).get());

        let mut state = SemanticSendStateV1::new(binding());
        let plan = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            1,
            0,
            vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::Terrain, 2), intent(SemanticStreamIdV1::InGame, 0)],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();
        // demand consumed exactly: 2 everywhere, +2 InGame, +1 Terrain
        assert_eq!(cursors(&state), [3, 3, 5, 3, 4]);
        assert!(plan.descriptor.streams.iter().all(|s| s.begin_sequence == 1));

        // A second checkpoint starts where the first stopped -- no reuse.
        let after_first = cursors(&state);
        let plan2 = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            2,
            1,
            vec![intent(SemanticStreamIdV1::InGame, 5)],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();
        let ingame = plan2.descriptor.streams.iter().find(|s| s.stream == SemanticStreamIdV1::InGame).unwrap();
        assert_eq!(ingame.begin_sequence, after_first[2]);
        assert_eq!(cursors(&state), [5, 5, 8, 5, 6]);

        // Every admission reject leaves ALL cursors frozen.
        let frozen = cursors(&state);
        let mut tight = profile();
        tight.max_records_per_checkpoint = 1;
        for (intents, profile) in [
            (vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::InGame, 1)], profile()),
            (vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::InGame, 2)], tight),
        ] {
            assert!(reserve_and_plan_recipient_checkpoint_v1(&mut state, 3, 2, intents, &profile, [1; 32], [2; 32]).is_err());
            assert_eq!(cursors(&state), frozen, "a refused checkpoint must not move a cursor");
        }

        // Exhaustion on ONE stream aborts the whole reservation.
        let mut edge = SemanticSendStateV1::new(binding());
        edge.reserve_sequences_v1([0, 0, u64::MAX - 1, 0, 0]).unwrap();
        assert!(matches!(
            reserve_and_plan_recipient_checkpoint_v1(&mut edge, 1, 0, vec![], &profile(), [1; 32], [2; 32]),
            Err(CheckpointPlanErrorV1::SequenceExhausted)
        ));
        assert_eq!(cursors(&edge), [1, 1, u64::MAX, 1, 1]);
    }

    /// `T3.4.11`: emission is fenced, ordered, and self-validating.
    #[test]
    fn frames_fence_every_stream_and_carry_bound_context() {
        let mut state = SemanticSendStateV1::new(binding());
        let plan = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            4,
            3,
            vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::Terrain, 2), intent(SemanticStreamIdV1::InGame, 0)],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();
        let frames = checkpoint_frames_v1(&plan).unwrap();

        assert_eq!(frames.len(), 13, "5 Begins + 5 Barriers + 3 data");
        // stream-canonical order, and within a stream Begin < data < Barrier
        let stream_order: Vec<SemanticStreamIdV1> = frames.iter().map(|f| f.stream()).collect();
        let mut seen: Vec<SemanticStreamIdV1> = Vec::new();
        for stream in &stream_order {
            if seen.last() != Some(stream) {
                assert!(!seen.contains(stream), "a stream's frames must not be interleaved");
                seen.push(*stream);
            }
        }
        assert_eq!(seen, REQUIRED_CHECKPOINT_STREAMS_V1.to_vec());
        for stream in REQUIRED_CHECKPOINT_STREAMS_V1 {
            let of: Vec<&CheckpointFrameV1> = frames.iter().filter(|f| f.stream() == stream).collect();
            assert!(matches!(of.first().unwrap(), CheckpointFrameV1::Begin { .. }));
            assert!(matches!(of.last().unwrap(), CheckpointFrameV1::Barrier { .. }));
            let seqs: Vec<u64> = of.iter().map(|f| f.sequence()).collect();
            assert!(seqs.windows(2).all(|w| w[1] == w[0] + 1), "sequences must be contiguous within a stream");
            assert!(of[1..of.len() - 1].iter().all(|f| matches!(f, CheckpointFrameV1::Data { .. })));
        }

        // context binding: data carries its ordinal, control never does,
        // and every frame is bound to this epoch and descriptor root
        let ordinals: Vec<u64> = frames
            .iter()
            .filter_map(|f| match f {
                CheckpointFrameV1::Data { context, .. } => Some(context.ordinal.unwrap().0),
                _ => None,
            })
            .collect();
        assert_eq!(ordinals.len(), 3);
        assert_eq!(ordinals.iter().copied().collect::<std::collections::BTreeSet<_>>(), [1, 2, 3].into());
        for frame in &frames {
            assert_eq!(frame.context().epoch, 4);
            assert_eq!(frame.context().descriptor_root, plan.descriptor_root);
            if !matches!(frame, CheckpointFrameV1::Data { .. }) {
                assert!(frame.context().ordinal.is_none(), "control frames carry no ordinal");
            }
        }
    }

    /// The cursor array's slot order and `REQUIRED_CHECKPOINT_STREAMS_V1`
    /// must agree, or demand would be reserved on the wrong stream.
    #[test]
    fn cursor_slots_match_the_required_stream_order() {
        for (slot, stream) in REQUIRED_CHECKPOINT_STREAMS_V1.into_iter().enumerate() {
            let mut state = SemanticSendStateV1::new(binding());
            let mut demand = [0u64; 5];
            demand[slot] = 7;
            state.reserve_sequences_v1(demand).unwrap();
            assert_eq!(state.next_for(stream).get(), 8, "slot {slot} must be {stream:?}");
            assert_eq!(stream_slot_v1(stream), slot);
        }
    }
}
