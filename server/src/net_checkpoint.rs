//! `APEX-T3.4.09` — the server checkpoint planner: one frozen plan per
//! recipient per flush. Pure over a frozen intent set; the T3.3 egress
//! seam supplies the intents and consumes the plan. Dormant until
//! activation, like the T3.3 V1 path it builds on.

use common::apex::digest::DigestBytes32V1;
use common_net::msg::ServerGeneral;
use common_net::msg::checkpoint::{
    CheckpointApplyPhaseV1, CheckpointDescriptorV1, CheckpointOrdinalV1, CheckpointParticipantV1,
    CheckpointBarrierV1, CheckpointBeginV1, CheckpointParticipationV1, CheckpointResourceProfileV1,
    CheckpointedEnvelopeContextV1,
    REQUIRED_CHECKPOINT_STREAMS_V1, StreamCheckpointPlanV1, TranscriptEntryV1,
    global_transcript_root_v1, stream_transcript_root_v1, validate_checkpoint_context_v1,
};
use common_net::msg::checkpoint::{AlignErrorV1, CheckpointAlignerV1, CheckpointCommitReceiptV1, CheckpointStreamOpenV1};
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
        control: CheckpointBeginV1,
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
        control: CheckpointBarrierV1,
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

impl CheckpointFrameV1 {
    /// `T3.4.20c`: the control message this frame puts on the wire, if
    /// any. Data frames carry their payload directly (already an `Arc`),
    /// so there is nothing to build for them. Every Begin ships the whole
    /// descriptor — physical streams have no cross-stream arrival order,
    /// so whichever Begin lands first must be able to open the receiver.
    pub fn control_message_v1(&self, descriptor: &CheckpointDescriptorV1) -> Option<ServerGeneral> {
        match self {
            Self::Begin { control, .. } => Some(ServerGeneral::CheckpointBegin(Box::new(
                CheckpointStreamOpenV1 { begin: control.clone(), descriptor: descriptor.clone() },
            ))),
            Self::Barrier { control, .. } => Some(ServerGeneral::CheckpointBarrier(control.clone())),
            Self::Data { .. } => None,
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
        frames.push(CheckpointFrameV1::Begin {
            stream,
            sequence: stream_plan.begin_sequence,
            context: control,
            control: CheckpointBeginV1 { epoch, stream, descriptor_root: plan.descriptor_root },
        });
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
        frames.push(CheckpointFrameV1::Barrier {
            stream,
            sequence: stream_plan.barrier_sequence,
            context: control,
            control: CheckpointBarrierV1 {
                epoch,
                stream,
                descriptor_root: plan.descriptor_root,
                data_record_count: stream_plan.data_record_count,
                payload_bytes: stream_plan.payload_bytes,
                last_data_sequence: stream_plan.last_data_sequence,
                stream_transcript_root: stream_plan.stream_transcript_root,
            },
        });
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

/// `APEX-T3.4.14` — what a plan's own frames prove when driven through a
/// real receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointCompletenessV1 {
    pub descriptor_root: [u8; 32],
    pub frames: usize,
    pub apply_order: Vec<(CheckpointOrdinalV1, CheckpointApplyPhaseV1)>,
}

/// `T3.4.14`: completeness is verified by RECEIVING, not by inspecting.
/// The plan's frames are driven through a fresh aligner, which recomputes
/// every root from the payloads themselves and refuses to yield anything
/// until all five streams are fenced. A plan that cannot be aligned is
/// not a checkpoint.
pub fn verify_plan_completeness_v1(
    plan: &RecipientCheckpointPlanV1,
) -> Result<CheckpointCompletenessV1, AlignErrorV1> {
    let frames = checkpoint_frames_v1(plan).map_err(|_| AlignErrorV1::Incomplete)?;
    let mut aligner = CheckpointAlignerV1::open_v1(plan.descriptor.clone(), plan.descriptor_root)?;
    for frame in &frames {
        match frame {
            CheckpointFrameV1::Begin { control, .. } => aligner.accept_begin_v1(control)?,
            CheckpointFrameV1::Data { stream, sequence, context, payload, .. } => {
                aligner.accept_data_v1(*stream, *sequence, context, Arc::clone(payload))?
            },
            CheckpointFrameV1::Barrier { control, .. } => aligner.accept_barrier_v1(control)?,
        }
    }
    if !aligner.is_complete() {
        return Err(AlignErrorV1::Incomplete);
    }
    let applied = aligner.take_apply_sequence_v1()?;
    Ok(CheckpointCompletenessV1 {
        descriptor_root: plan.descriptor_root,
        frames: frames.len(),
        apply_order: applied.iter().map(|r| (r.ordinal, r.phase)).collect(),
    })
}


/// `APEX-T3.4.13` — while a checkpoint is in flight, a stream carries
/// that checkpoint and nothing else. Ordinary traffic is HELD, not
/// dropped and not interleaved: interleaving would put unordinaled data
/// inside a fenced segment, which the receiver cannot align.
#[derive(Debug, Clone, Default)]
pub struct CheckpointEgressGateV1 {
    /// Epoch of the checkpoint fencing each stream, if any.
    fenced: [Option<u64>; 5],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EgressAdmitV1 {
    Send,
    /// Hold until this stream's Barrier goes out; the sender must not
    /// reorder around it.
    Hold,
    Reject(&'static str),
}

impl CheckpointEgressGateV1 {
    pub fn new() -> Self { Self::default() }

    pub fn is_quiescent(&self) -> bool { self.fenced.iter().all(|f| f.is_none()) }

    pub fn fenced_epoch(&self, stream: SemanticStreamIdV1) -> Option<u64> { self.fenced[stream_slot_v1(stream)] }

    /// Fences every stream for this plan's epoch. A plan cannot open over
    /// a checkpoint that has not finished emitting.
    pub fn open_for_plan_v1(&mut self, plan: &RecipientCheckpointPlanV1) -> Result<(), CheckpointPlanErrorV1> {
        if !self.is_quiescent() {
            return Err(CheckpointPlanErrorV1::PlanInvariant("checkpoint opened while another is in flight"));
        }
        self.fenced = [Some(plan.descriptor.epoch); 5];
        Ok(())
    }

    /// Releases one stream; called as its Barrier is emitted.
    pub fn close_stream_v1(&mut self, stream: SemanticStreamIdV1, epoch: u64) -> Result<(), CheckpointPlanErrorV1> {
        let slot = stream_slot_v1(stream);
        match self.fenced[slot] {
            Some(open) if open == epoch => {
                self.fenced[slot] = None;
                Ok(())
            },
            _ => Err(CheckpointPlanErrorV1::PlanInvariant("barrier for a stream this gate has not fenced")),
        }
    }

    /// A checkpoint's own frame passes only on the stream and epoch it
    /// was planned for.
    pub fn admit_frame_v1(&self, frame: &CheckpointFrameV1) -> EgressAdmitV1 {
        match self.fenced[stream_slot_v1(frame.stream())] {
            Some(open) if open == frame.context().epoch => EgressAdmitV1::Send,
            Some(_) => EgressAdmitV1::Reject("frame from another checkpoint epoch"),
            None => EgressAdmitV1::Reject("checkpoint frame on an unfenced stream"),
        }
    }

    /// Everything else the server wants to send while the fence is up.
    /// Diagnostics are out-of-band by construction and always pass;
    /// checkpointed data waits; a foreign control frame is a bug.
    pub fn admit_other_v1(&self, stream: SemanticStreamIdV1, payload: &ServerGeneral) -> EgressAdmitV1 {
        let participation = payload.participation_v1();
        if participation == CheckpointParticipationV1::OutOfBandDiagnostic {
            return EgressAdmitV1::Send;
        }
        match self.fenced[stream_slot_v1(stream)] {
            None => EgressAdmitV1::Send,
            Some(_) => match participation {
                CheckpointParticipationV1::CheckpointedData => EgressAdmitV1::Hold,
                CheckpointParticipationV1::CheckpointControl => {
                    EgressAdmitV1::Reject("control payload inside a fenced segment")
                },
                CheckpointParticipationV1::OutOfBandDiagnostic => EgressAdmitV1::Send,
            },
        }
    }
}

/// Emits a plan through the gate: every frame is admitted, and each
/// stream is released exactly at its own Barrier. Returns the frames in
/// send order with the gate left quiescent.
pub fn emit_through_gate_v1(
    gate: &mut CheckpointEgressGateV1,
    plan: &RecipientCheckpointPlanV1,
) -> Result<Vec<CheckpointFrameV1>, CheckpointPlanErrorV1> {
    let frames = checkpoint_frames_v1(plan)?;
    gate.open_for_plan_v1(plan)?;
    for frame in &frames {
        if gate.admit_frame_v1(frame) != EgressAdmitV1::Send {
            return Err(CheckpointPlanErrorV1::PlanInvariant("gate refused a frame of its own plan"));
        }
        if matches!(frame, CheckpointFrameV1::Barrier { .. }) {
            gate.close_stream_v1(frame.stream(), frame.context().epoch)?;
        }
    }
    if !gate.is_quiescent() {
        return Err(CheckpointPlanErrorV1::PlanInvariant("a stream was left fenced after emission"));
    }
    Ok(frames)
}


/// `APEX-T3.4.19` — per-recipient commit watermark. The server does not
/// take "acknowledged" on trust: an ack must name the epoch, the
/// descriptor root and the record count of the checkpoint that is
/// actually outstanding, or it is refused and the watermark stands.
#[derive(Debug, Clone, Copy, Default)]
pub struct RecipientCommitWatermarkV1 {
    committed_epoch: u64,
    outstanding: Option<CheckpointCommitReceiptV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitAckErrorV1 {
    NothingOutstanding,
    EpochMismatch,
    RootMismatch,
    RecordCountMismatch,
    ParentMismatch,
    AlreadyOutstanding,
}

impl RecipientCommitWatermarkV1 {
    pub fn new() -> Self { Self::default() }

    pub fn committed_epoch(&self) -> u64 { self.committed_epoch }

    pub fn outstanding_epoch(&self) -> Option<u64> { self.outstanding.map(|r| r.epoch) }

    /// Records what this recipient must ack. One checkpoint may be
    /// outstanding at a time — the same rule the egress gate enforces.
    pub fn expect_commit_v1(&mut self, plan: &RecipientCheckpointPlanV1) -> Result<(), CommitAckErrorV1> {
        if self.outstanding.is_some() {
            return Err(CommitAckErrorV1::AlreadyOutstanding);
        }
        if plan.descriptor.parent_epoch != self.committed_epoch {
            return Err(CommitAckErrorV1::ParentMismatch);
        }
        self.outstanding = Some(CheckpointCommitReceiptV1 {
            epoch: plan.descriptor.epoch,
            parent_epoch: plan.descriptor.parent_epoch,
            descriptor_root: plan.descriptor_root,
            applied_records: plan.descriptor.data_record_count,
        });
        Ok(())
    }

    /// Accepts an ack only if it matches the outstanding checkpoint in
    /// every field. The watermark advances on success and on nothing else.
    pub fn accept_ack_v1(&mut self, ack: &CheckpointCommitReceiptV1) -> Result<u64, CommitAckErrorV1> {
        let expected = self.outstanding.ok_or(CommitAckErrorV1::NothingOutstanding)?;
        if ack.epoch != expected.epoch {
            return Err(CommitAckErrorV1::EpochMismatch);
        }
        if ack.parent_epoch != expected.parent_epoch {
            return Err(CommitAckErrorV1::ParentMismatch);
        }
        if ack.descriptor_root != expected.descriptor_root {
            return Err(CommitAckErrorV1::RootMismatch);
        }
        if ack.applied_records != expected.applied_records {
            return Err(CommitAckErrorV1::RecordCountMismatch);
        }
        self.committed_epoch = expected.epoch;
        self.outstanding = None;
        Ok(self.committed_epoch)
    }
}


/// `APEX-T3.4.22` — perturbation harness. One knob per way a checkpoint
/// can arrive wrong, plus the one way it can legitimately arrive
/// differently (cross-stream interleaving), driven through the real
/// receiver. Seeded and parameterizable so it can be cranked later
/// rather than rewritten.
#[derive(Debug, Clone)]
pub enum CheckpointPerturbationV1 {
    /// The plan's own send order.
    None,
    /// Streams interleaved arbitrarily, each stream's own order intact.
    /// This one must still ALIGN -- physical streams have no cross-stream
    /// arrival order, and the design exists to tolerate that.
    InterleaveStreams { seed: u64 },
    /// One stream's frames reversed: per-stream FIFO is load-bearing.
    ReverseWithinStream,
    DropFrame { index: usize },
    DuplicateFrame { index: usize },
    /// A data frame's payload replaced with a different one of the same
    /// size class, leaving every declared count intact.
    SwapPayload { index: usize, replacement: ServerGeneral },
    ForgeOrdinal { index: usize, ordinal: u64 },
    ForeignEpoch { index: usize },
}

fn perturbed_frames_v1(frames: Vec<CheckpointFrameV1>, perturbation: &CheckpointPerturbationV1) -> Vec<CheckpointFrameV1> {
    use CheckpointPerturbationV1 as P;
    match perturbation {
        P::None => frames,
        P::InterleaveStreams { seed } => {
            // Round-robin the per-stream queues in a seed-chosen order:
            // every stream's own sequence order survives, nothing else does.
            let mut queues: Vec<Vec<CheckpointFrameV1>> = REQUIRED_CHECKPOINT_STREAMS_V1
                .into_iter()
                .map(|stream| frames.iter().filter(|f| f.stream() == stream).cloned().collect())
                .collect();
            let mut state = *seed | 1;
            let mut out = Vec::with_capacity(frames.len());
            while queues.iter().any(|q| !q.is_empty()) {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let pick = (state >> 33) as usize % queues.len();
                if let Some(frame) = queues[pick].first().cloned() {
                    queues[pick].remove(0);
                    out.push(frame);
                }
            }
            out
        },
        P::ReverseWithinStream => {
            let mut out = Vec::with_capacity(frames.len());
            for stream in REQUIRED_CHECKPOINT_STREAMS_V1 {
                let mut of: Vec<CheckpointFrameV1> = frames.iter().filter(|f| f.stream() == stream).cloned().collect();
                of.reverse();
                out.extend(of);
            }
            out
        },
        P::DropFrame { index } => {
            let mut out = frames;
            if *index < out.len() {
                out.remove(*index);
            }
            out
        },
        P::DuplicateFrame { index } => {
            let mut out = frames;
            if let Some(frame) = out.get(*index).cloned() {
                out.insert(*index + 1, frame);
            }
            out
        },
        P::SwapPayload { index, replacement } => {
            let mut out = frames;
            if let Some(CheckpointFrameV1::Data { payload, .. }) = out.get_mut(*index) {
                *payload = Arc::new(replacement.clone());
            }
            out
        },
        P::ForgeOrdinal { index, ordinal } => {
            let mut out = frames;
            if let Some(CheckpointFrameV1::Data { context, .. }) = out.get_mut(*index) {
                context.ordinal = Some(CheckpointOrdinalV1(*ordinal));
            }
            out
        },
        P::ForeignEpoch { index } => {
            let mut out = frames;
            if let Some(frame) = out.get_mut(*index) {
                let epoch = match frame {
                    CheckpointFrameV1::Begin { context, .. }
                    | CheckpointFrameV1::Data { context, .. }
                    | CheckpointFrameV1::Barrier { context, .. } => {
                        context.epoch += 1;
                        context.epoch
                    },
                };
                match frame {
                    CheckpointFrameV1::Begin { control, .. } => control.epoch = epoch,
                    CheckpointFrameV1::Barrier { control, .. } => control.epoch = epoch,
                    CheckpointFrameV1::Data { .. } => {},
                }
            }
            out
        },
    }
}

/// Drives a plan's frames through a fresh receiver under one
/// perturbation. `Ok` means the checkpoint still aligned and committed.
pub fn drive_perturbed_checkpoint_v1(
    plan: &RecipientCheckpointPlanV1,
    perturbation: &CheckpointPerturbationV1,
) -> Result<CheckpointCompletenessV1, AlignErrorV1> {
    let frames = perturbed_frames_v1(checkpoint_frames_v1(plan).map_err(|_| AlignErrorV1::Incomplete)?, perturbation);
    let mut aligner = CheckpointAlignerV1::open_v1(plan.descriptor.clone(), plan.descriptor_root)?;
    for frame in &frames {
        match frame {
            CheckpointFrameV1::Begin { control, .. } => aligner.accept_begin_v1(control)?,
            CheckpointFrameV1::Data { stream, sequence, context, payload, .. } => {
                aligner.accept_data_v1(*stream, *sequence, context, Arc::clone(payload))?
            },
            CheckpointFrameV1::Barrier { control, .. } => aligner.accept_barrier_v1(control)?,
        }
    }
    if !aligner.is_complete() {
        return Err(AlignErrorV1::Incomplete);
    }
    let applied = aligner.take_apply_sequence_v1()?;
    Ok(CheckpointCompletenessV1 {
        descriptor_root: plan.descriptor_root,
        frames: frames.len(),
        apply_order: applied.iter().map(|r| (r.ordinal, r.phase)).collect(),
    })
}


/// `APEX-T3.4.24` — production admission. The checkpoint path may only
/// be activated when every precondition holds; each failure is NAMED, so
/// a refusal says what is missing rather than just "no".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointActivationBlockerV1 {
    /// No deployment-supplied production resource profile exists.
    NoProductionProfile,
    /// The canary coverage map still has cases this tier does not cover.
    UncoveredCanaryCases(usize),
    /// The supplied profile budgets a stream this client type can never
    /// legally receive.
    ProfileIllegalForClientType,
}

/// Admits activation for one client type, or returns EVERY blocker that
/// applies — not just the first, so one pass names the whole gap.
pub fn admit_checkpoint_activation_v1(
    client_type: common_net::msg::ClientType,
) -> Result<CheckpointResourceProfileV1, Vec<CheckpointActivationBlockerV1>> {
    use common_net::msg::checkpoint::{production_checkpoint_profile_v1, validate_profile_for_client_type_v1};

    let mut blockers = Vec::new();
    if crate::net_checkpoint_canaries::OPEN_CASE_COUNT > 0 {
        blockers.push(CheckpointActivationBlockerV1::UncoveredCanaryCases(
            crate::net_checkpoint_canaries::OPEN_CASE_COUNT,
        ));
    }
    match production_checkpoint_profile_v1() {
        Err(_) => {
            blockers.push(CheckpointActivationBlockerV1::NoProductionProfile);
            Err(blockers)
        },
        Ok(profile) => {
            if validate_profile_for_client_type_v1(&profile, client_type).is_err() {
                blockers.push(CheckpointActivationBlockerV1::ProfileIllegalForClientType);
            }
            if blockers.is_empty() { Ok(profile) } else { Err(blockers) }
        },
    }
}

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
        intent_payload(stream, local_ordinal, ServerGeneral::UpdateRecipes)
    }

    fn intent_payload(stream: SemanticStreamIdV1, local_ordinal: u32, payload: ServerGeneral) -> SemanticSendIntentV1 {
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
            payload: Arc::new(payload),
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

    /// `T3.4.24`: production activation is refused, and the refusal
    /// names every blocker that applies.
    #[test]
    fn production_activation_is_refused_with_named_blockers() {
        use common_net::msg::ClientType;

        for client_type in [
            ClientType::Game,
            ClientType::ChatOnly,
            ClientType::SilentSpectator,
            ClientType::Bot { privileged: false },
            ClientType::Bot { privileged: true },
        ] {
            let blockers = admit_checkpoint_activation_v1(client_type).unwrap_err();
            assert!(
                blockers.contains(&CheckpointActivationBlockerV1::NoProductionProfile),
                "{client_type:?}: production has no profile to activate with"
            );
            assert!(
                blockers.iter().any(|b| matches!(b, CheckpointActivationBlockerV1::UncoveredCanaryCases(n) if *n > 0)),
                "{client_type:?}: the OPEN canary set must block activation while it is nonempty"
            );
        }
        // The two blockers are independent: closing one alone does not
        // admit activation.
        assert!(crate::net_checkpoint_canaries::OPEN_CASE_COUNT > 0);
    }

    /// `T3.4.22`: the perturbation table. Cross-stream interleaving is
    /// the ONE reordering a checkpoint must survive; every other
    /// perturbation must be caught.
    #[test]
    fn only_cross_stream_interleaving_survives_perturbation() {
        use CheckpointPerturbationV1 as P;

        let mut state = SemanticSendStateV1::new(binding());
        let plan = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            1,
            0,
            vec![
                intent_payload(SemanticStreamIdV1::InGame, 1, ServerGeneral::CharacterSuccess),
                intent_payload(SemanticStreamIdV1::InGame, 2, ServerGeneral::UpdateRecipes),
                intent_payload(SemanticStreamIdV1::Terrain, 3, ServerGeneral::ExitInGameSuccess),
            ],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();

        let clean = drive_perturbed_checkpoint_v1(&plan, &P::None).unwrap();

        // Cross-stream arrival order is not observable in the outcome.
        for seed in [1u64, 2, 3, 7, 11, 101, 65537, u64::MAX] {
            let shuffled = drive_perturbed_checkpoint_v1(&plan, &P::InterleaveStreams { seed }).unwrap();
            assert_eq!(shuffled, clean, "seed {seed}: interleaving must not move the outcome");
        }

        // Everything else is caught.
        let frames = checkpoint_frames_v1(&plan).unwrap();
        let data_index = frames
            .iter()
            .position(|f| matches!(f, CheckpointFrameV1::Data { .. }))
            .expect("the plan has data frames");
        for perturbation in [
            P::ReverseWithinStream,
            P::DropFrame { index: 0 },
            P::DropFrame { index: data_index },
            P::DuplicateFrame { index: data_index },
            P::SwapPayload { index: data_index, replacement: ServerGeneral::ExitInGameSuccess },
            P::ForgeOrdinal { index: data_index, ordinal: 3 },
            P::ForeignEpoch { index: 0 },
        ] {
            assert!(
                drive_perturbed_checkpoint_v1(&plan, &perturbation).is_err(),
                "{perturbation:?} must not align"
            );
        }
    }

    /// `T3.4.20c`: every control frame becomes a wire message that routes
    /// back to its OWN stream, and every Begin is self-sufficient.
    #[test]
    fn control_frames_become_self_sufficient_wire_messages() {
        use common_net::msg::envelope::SemanticRouteV1;

        let mut state = SemanticSendStateV1::new(binding());
        let plan = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            2,
            1,
            vec![intent(SemanticStreamIdV1::InGame, 1)],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();

        let frames = checkpoint_frames_v1(&plan).unwrap();
        let mut begins = 0;
        let mut barriers = 0;
        for frame in &frames {
            match frame.control_message_v1(&plan.descriptor) {
                None => assert!(matches!(frame, CheckpointFrameV1::Data { .. })),
                Some(msg) => {
                    assert_eq!(msg.semantic_stream(), frame.stream(), "a fence must route to its own stream");
                    match &msg {
                        ServerGeneral::CheckpointBegin(open) => {
                            begins += 1;
                            // self-sufficient: the whole descriptor rides along
                            assert_eq!(open.descriptor, plan.descriptor);
                            assert_eq!(open.begin.descriptor_root, plan.descriptor_root);
                        },
                        ServerGeneral::CheckpointBarrier(b) => {
                            barriers += 1;
                            assert_eq!(b.descriptor_root, plan.descriptor_root);
                        },
                        _ => panic!("a control frame produced a non-control message"),
                    }
                },
            }
        }
        assert_eq!((begins, barriers), (5, 5));
    }

    /// `T3.4.19`: the watermark advances only on an ack that matches the
    /// outstanding checkpoint in every field.
    #[test]
    fn only_a_matching_ack_advances_the_commit_watermark() {
        let mut state = SemanticSendStateV1::new(binding());
        let plan = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            1,
            0,
            vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::Terrain, 2)],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();

        let mut wm = RecipientCommitWatermarkV1::new();
        assert_eq!(wm.committed_epoch(), 0);
        let good = CheckpointCommitReceiptV1 {
            epoch: 1,
            parent_epoch: 0,
            descriptor_root: plan.descriptor_root,
            applied_records: 2,
        };
        // nothing outstanding yet: an ack cannot invent one
        assert_eq!(wm.accept_ack_v1(&good).unwrap_err(), CommitAckErrorV1::NothingOutstanding);

        wm.expect_commit_v1(&plan).unwrap();
        assert_eq!(wm.outstanding_epoch(), Some(1));
        assert_eq!(wm.expect_commit_v1(&plan).unwrap_err(), CommitAckErrorV1::AlreadyOutstanding);

        for (bad, want) in [
            (CheckpointCommitReceiptV1 { epoch: 2, ..good }, CommitAckErrorV1::EpochMismatch),
            (CheckpointCommitReceiptV1 { parent_epoch: 9, ..good }, CommitAckErrorV1::ParentMismatch),
            (CheckpointCommitReceiptV1 { descriptor_root: [0xCC; 32], ..good }, CommitAckErrorV1::RootMismatch),
            (CheckpointCommitReceiptV1 { applied_records: 1, ..good }, CommitAckErrorV1::RecordCountMismatch),
        ] {
            assert_eq!(wm.accept_ack_v1(&bad).unwrap_err(), want);
            assert_eq!(wm.committed_epoch(), 0, "a refused ack must not advance the watermark");
            assert_eq!(wm.outstanding_epoch(), Some(1), "and must not clear the outstanding checkpoint");
        }

        assert_eq!(wm.accept_ack_v1(&good).unwrap(), 1);
        assert_eq!(wm.outstanding_epoch(), None);
        // the same ack cannot be replayed to advance anything twice
        assert_eq!(wm.accept_ack_v1(&good).unwrap_err(), CommitAckErrorV1::NothingOutstanding);

        // the next checkpoint must chain off the committed epoch
        let orphan = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            3,
            2,
            vec![intent(SemanticStreamIdV1::InGame, 3)],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();
        assert_eq!(wm.expect_commit_v1(&orphan).unwrap_err(), CommitAckErrorV1::ParentMismatch);
    }

    /// `T3.4.14`: completeness proven by RECEIVING the plan's own frames.
    #[test]
    fn a_plan_aligns_end_to_end_and_tampering_does_not() {
        use common_net::msg::checkpoint::CheckpointApplyPhaseV1 as Ph;

        let mut state = SemanticSendStateV1::new(binding());
        let mut plan = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            5,
            4,
            vec![
                intent_payload(SemanticStreamIdV1::InGame, 1, ServerGeneral::CharacterSuccess),
                intent_payload(SemanticStreamIdV1::InGame, 2, ServerGeneral::UpdateRecipes),
                intent_payload(SemanticStreamIdV1::Terrain, 3, ServerGeneral::ExitInGameSuccess),
            ],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();

        let proof = verify_plan_completeness_v1(&plan).unwrap();
        assert_eq!(proof.descriptor_root, plan.descriptor_root);
        assert_eq!(proof.frames, 13);
        assert_eq!(
            proof.apply_order,
            vec![
                (CheckpointOrdinalV1(1), Ph::CharacterState),
                (CheckpointOrdinalV1(2), Ph::InGameState),
                (CheckpointOrdinalV1(3), Ph::InGameState),
            ]
        );

        // A payload swapped after planning keeps every count and sequence
        // the descriptor declares -- only the recomputed transcript moves.
        plan.intents[1].payload = Arc::new(ServerGeneral::ExitInGameSuccess);
        assert_eq!(verify_plan_completeness_v1(&plan), Err(AlignErrorV1::StreamRootMismatch));

        // And a descriptor edited away from its own root cannot even open.
        let mut retampered = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            6,
            5,
            vec![intent_payload(SemanticStreamIdV1::InGame, 1, ServerGeneral::CharacterSuccess)],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();
        retampered.descriptor.streams[2].stream_transcript_root = [0xEE; 32];
        assert!(verify_plan_completeness_v1(&retampered).is_err());
    }

    /// `T3.4.13`: a fenced stream carries its checkpoint and nothing
    /// else, and every stream is released exactly at its own Barrier.
    #[test]
    fn a_fenced_stream_holds_ordinary_traffic_until_its_barrier() {
        let mut state = SemanticSendStateV1::new(binding());
        let plan = reserve_and_plan_recipient_checkpoint_v1(
            &mut state,
            7,
            6,
            vec![intent(SemanticStreamIdV1::InGame, 1), intent(SemanticStreamIdV1::Terrain, 2)],
            &profile(),
            [1; 32],
            [2; 32],
        )
        .unwrap();

        let mut gate = CheckpointEgressGateV1::new();
        assert!(gate.is_quiescent());
        // before the fence, ordinary data flows
        assert_eq!(gate.admit_other_v1(SemanticStreamIdV1::InGame, &ServerGeneral::UpdateRecipes), EgressAdmitV1::Send);

        gate.open_for_plan_v1(&plan).unwrap();
        // ...and while it is up, that same data is HELD, not dropped
        for stream in REQUIRED_CHECKPOINT_STREAMS_V1 {
            assert_eq!(gate.admit_other_v1(stream, &ServerGeneral::UpdateRecipes), EgressAdmitV1::Hold);
            assert_eq!(gate.fenced_epoch(stream), Some(7));
        }
        // a second checkpoint cannot open over one still in flight
        assert!(gate.open_for_plan_v1(&plan).is_err());
        // frames of another epoch are refused even on a fenced stream
        let frames = checkpoint_frames_v1(&plan).unwrap();
        let mut foreign = frames[0].clone();
        if let CheckpointFrameV1::Begin { context, .. } = &mut foreign {
            context.epoch = 8;
        }
        assert!(matches!(gate.admit_frame_v1(&foreign), EgressAdmitV1::Reject(_)));

        // releasing is per-stream and epoch-matched
        gate.close_stream_v1(SemanticStreamIdV1::InGame, 7).unwrap();
        assert_eq!(gate.admit_other_v1(SemanticStreamIdV1::InGame, &ServerGeneral::UpdateRecipes), EgressAdmitV1::Send);
        assert_eq!(gate.admit_other_v1(SemanticStreamIdV1::Terrain, &ServerGeneral::UpdateRecipes), EgressAdmitV1::Hold);
        assert!(gate.close_stream_v1(SemanticStreamIdV1::InGame, 7).is_err(), "a stream cannot be released twice");
        assert!(!gate.is_quiescent());

        // the whole emission leaves the gate quiescent again
        let mut fresh = CheckpointEgressGateV1::new();
        let emitted = emit_through_gate_v1(&mut fresh, &plan).unwrap();
        assert_eq!(emitted.len(), frames.len());
        assert!(fresh.is_quiescent());
        assert_eq!(fresh.admit_other_v1(SemanticStreamIdV1::InGame, &ServerGeneral::UpdateRecipes), EgressAdmitV1::Send);
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
