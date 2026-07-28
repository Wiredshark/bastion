//! `APEX-T3.4.09` — the server checkpoint planner: one frozen plan per
//! recipient per flush. Pure over a frozen intent set; the T3.3 egress
//! seam supplies the intents and consumes the plan. Dormant until
//! activation, like the T3.3 V1 path it builds on.

use common::apex::digest::DigestBytes32V1;
use common_net::msg::checkpoint::{
    CheckpointApplyPhaseV1, CheckpointDescriptorV1, CheckpointOrdinalV1, CheckpointParticipantV1,
    CheckpointParticipationV1, CheckpointResourceProfileV1, REQUIRED_CHECKPOINT_STREAMS_V1,
    StreamCheckpointPlanV1, TranscriptEntryV1, global_transcript_root_v1, stream_transcript_root_v1,
};
use common_net::msg::envelope::{ActiveSessionBindingV1, SemanticRouteV1, SemanticStreamIdV1};
use crate::semantic_net::outbox::SemanticSendIntentV1;

#[derive(Debug)]
pub enum CheckpointPlanErrorV1 {
    ProducerAfterFlush,
    DuplicateOrderKey,
    PlanInvariant(&'static str),
    ResourceExceeded(&'static str),
    RootFailure,
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

/// Plan ONE recipient from its frozen intents. Ordinals follow the T3.3
/// total sort key, so the plan is a pure function of the intent SET.
/// Sequences are reserved contiguously from `first_sequence`: Begin,
/// then that stream's data, then Barrier, stream by stream in canonical
/// order (T3.4.10 owns whole-plan reservation across recipients).
pub fn plan_recipient_checkpoint_v1(
    recipient: ActiveSessionBindingV1,
    epoch: u64,
    parent_epoch: u64,
    mut intents: Vec<SemanticSendIntentV1>,
    first_sequence: u64,
    profile: &CheckpointResourceProfileV1,
    apply_policy_root: [u8; 32],
    egress_order_policy_root: [u8; 32],
) -> Result<RecipientCheckpointPlanV1, CheckpointPlanErrorV1> {
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

    // Ordinals are global across the recipient, assigned in sort order.
    let ordered: Vec<(CheckpointOrdinalV1, &SemanticSendIntentV1)> = intents
        .iter()
        .enumerate()
        .map(|(i, intent)| (CheckpointOrdinalV1(i as u64 + 1), intent))
        .collect();

    let mut seq = first_sequence;
    let mut stream_plans: Vec<StreamCheckpointPlanV1> = Vec::with_capacity(5);
    let mut records: Vec<(CheckpointOrdinalV1, SemanticStreamIdV1, u64)> = Vec::with_capacity(ordered.len());
    let mut all_entries: Vec<TranscriptEntryV1> = Vec::with_capacity(ordered.len());
    let mut total_bytes = 0u64;

    for (idx, stream) in REQUIRED_CHECKPOINT_STREAMS_V1.into_iter().enumerate() {
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
        seq += 1;

        if bytes > profile.max_payload_bytes_per_stream[idx] {
            return Err(E::ResourceExceeded("payload_bytes_per_stream"));
        }
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

    let record_count = ordered.len() as u32;
    if record_count > profile.max_records_per_checkpoint {
        return Err(E::ResourceExceeded("records_per_checkpoint"));
    }
    if total_bytes > profile.max_payload_bytes_per_checkpoint {
        return Err(E::ResourceExceeded("payload_bytes_per_checkpoint"));
    }

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

    Ok(RecipientCheckpointPlanV1 { recipient, descriptor, descriptor_root, records })
}

/// The apply phase of a planned record, for the client-side aligner.
pub fn record_apply_phase_v1(intent: &SemanticSendIntentV1) -> Option<CheckpointApplyPhaseV1> {
    intent.payload.apply_phase_v1()
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
    use common_net::msg::ServerGeneral;
    use common_net::msg::checkpoint::CheckpointProfilePurposeV1;
    use common_net::msg::envelope::SemanticCausalityV1;
    use std::sync::Arc;

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

    #[test]
    fn plan_is_a_pure_function_of_the_intent_set() {
        let mk = || vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::Terrain, 2), intent(SemanticStreamIdV1::InGame, 0)];
        let a = plan_recipient_checkpoint_v1(binding(), 1, 0, mk(), 1, &profile(), [1; 32], [2; 32]).unwrap();
        let mut reversed = mk();
        reversed.reverse();
        let b = plan_recipient_checkpoint_v1(binding(), 1, 0, reversed, 1, &profile(), [1; 32], [2; 32]).unwrap();
        assert_eq!(a.descriptor_root, b.descriptor_root, "input order must not move the plan");
        assert_eq!(a.records, b.records);

        // every stream is fenced: 5 Begins + 5 Barriers + 3 data = 13 sequences
        assert_eq!(a.descriptor.streams.len(), 5);
        assert_eq!(a.descriptor.data_record_count, 3);
        assert_eq!(a.descriptor.ordinal_max, 3);
        let last_barrier = a.descriptor.streams.iter().map(|s| s.barrier_sequence).max().unwrap();
        assert_eq!(last_barrier, 13);
        // empty streams still get Begin+Barrier with a typed root
        let empty: Vec<_> = a.descriptor.streams.iter().filter(|s| s.data_record_count == 0).collect();
        assert_eq!(empty.len(), 3);
        assert!(empty.iter().all(|s| s.barrier_sequence == s.begin_sequence + 1));
    }

    #[test]
    fn duplicate_order_key_and_resource_ceiling_are_typed() {
        let dup = vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::InGame, 1)];
        assert!(matches!(
            plan_recipient_checkpoint_v1(binding(), 1, 0, dup, 1, &profile(), [1; 32], [2; 32]),
            Err(CheckpointPlanErrorV1::DuplicateOrderKey)
        ));

        let mut tight = profile();
        tight.max_records_per_checkpoint = 1;
        let two = vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::InGame, 2)];
        assert!(matches!(
            plan_recipient_checkpoint_v1(binding(), 1, 0, two, 1, &tight, [1; 32], [2; 32]),
            Err(CheckpointPlanErrorV1::ResourceExceeded(_))
        ));
    }
}
