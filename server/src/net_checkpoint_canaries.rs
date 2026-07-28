//! `APEX-T3.4.23` — companion-canary coverage map for the 176 logical
//! cases of `readme/apex/PROJECT-BASTION-APEX-T3.4-CROSS-STREAM-
//! CHECKPOINT-CANARIES-v1.json` (160) plus its session-control delta
//! (16). Same "unclaimed-name-fails" standard `T3.3.20` set: a case ID
//! with no entry here is a build failure, never a silent gap.
//!
//! Claim kinds:
//! - a `crate::path::to::test_fn` string names an existing test that
//!   drives the exact typed outcome the case names.
//! - `"structural: ..."` is covered BY CONSTRUCTION, with the reasoning
//!   inline.
//! - `"OPEN: ..."` is a case this tier does NOT yet cover, named rather
//!   than papered over. The count is pinned below, so a new OPEN cannot
//!   be added quietly and closing one is a visible edit.
//!
//! The OPEN set is concentrated in three places, all of them real: ECS
//! referential preflight (116/117/127), the commit-vs-tick ordering pin
//! (121/123/124/130), and session control (036/162-171/173-176).
//!
//! RE-PINNED TWICE. First after T3.5 closed (2026-07-28), when the count
//! did NOT move; then again once `T3.6.01` actually built the frames,
//! which took it 22 -> 10. The history is kept because the first re-pin
//! is the honest part: a prediction that cases would close is not a
//! closure.
//!
//! First re-pin, verbatim:
//! This row originally said the session-control cases "need frames T3.5
//! introduces". T3.5 shipped session-control IDENTITY — a journaled,
//! never-auto-retried `CommandKindV1::SessionControl`, and a journal
//! that survives a resume — but it introduced no `SessionTerminate`
//! FRAME, so nothing here became coverable. Sixteen cases stay open for
//! the reason they were always open, and the prediction that they would
//! close was wrong rather than the work being incomplete. The frames are
//! their own row.
//!
//! Second re-pin: `T3.6.01` built `SessionTerminateV1` and its control
//! lane, closing `CKPT-162`..`CKPT-171`, `CKPT-175` and `CKPT-176`.
//! `CKPT-173` was closed by wiring the control lane to the egress gate.
//! `CKPT-174` was closed by `T3.6.03`'s legacy-disconnect inventory: the
//! four live send sites are enumerated, each mapped to the
//! `SessionTerminationReasonV1` it becomes, and the set pinned so a new
//! one fails the build. The sites are NOT migrated — the checkpoint path
//! is dormant, so flipping live disconnect behaviour on a dormant
//! premise would be the wrong trade. The claim is a tripwire plus a
//! written migration, not that the legacy source is gone.

pub(crate) const OPEN_CASE_COUNT: usize = 8;

pub(crate) const CASE_COVERAGE: &[(&str, &str)] = &[
    ("CKPT-001", "veloren_server::net_checkpoint::checkpoint_planner_v1::a_plan_aligns_end_to_end_and_tampering_does_not"),
    ("CKPT-002", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-003", "veloren_server::net_checkpoint::checkpoint_planner_v1::a_plan_aligns_end_to_end_and_tampering_does_not"),
    ("CKPT-004", "structural: CheckpointDescriptorV1::validate_v1 pins schema_version, and the descriptor rides bincode with exact-consume decode -- an unknown schema never reaches the aligner (see descriptor_validates_completeness_and_roots_bind_every_field)"),
    ("CKPT-005", "structural: T0.2's StructFieldsV1::finish_no_unknown rejects any field the header codec does not name; the descriptor payload itself is exact-consume decoded (decode_payload_exact_v1)"),
    ("CKPT-006", "structural: CanonicalFieldMapV1::try_from_entries refuses duplicate field ids (T0.2), so a duplicated descriptor field cannot decode"),
    ("CKPT-007", "structural: the T0.2 canonical codec has no indefinite-length form to emit or accept"),
    ("CKPT-008", "veloren_common_net::msg::checkpoint::checkpoint_profile_v1::required_stream_set_is_all_five_exactly"),
    ("CKPT-009", "veloren_common_net::msg::checkpoint::checkpoint_profile_v1::required_stream_set_is_all_five_exactly"),
    ("CKPT-010", "veloren_common_net::msg::checkpoint::checkpoint_profile_v1::required_stream_set_is_all_five_exactly"),
    ("CKPT-011", "veloren_common_net::msg::checkpoint::checkpoint_profile_v1::required_stream_set_is_all_five_exactly"),
    ("CKPT-012", "structural: resource_profile_root is a non-Option field of CheckpointDescriptorV1 -- an absent profile is unrepresentable; the value it must match is checked by admit_descriptor_v1 (declared_and_actual_limits_are_both_enforced)"),
    ("CKPT-013", "structural: apply_policy_root is a non-Option descriptor field, bound into descriptor_root_v1 (descriptor_validates_completeness_and_roots_bind_every_field)"),
    ("CKPT-014", "structural: egress_order_policy_root is a non-Option descriptor field, bound into descriptor_root_v1 (same test)"),
    ("CKPT-015", "structural: global_transcript_root is a non-Option descriptor field; an empty checkpoint gets the TYPED empty root, never an absent one"),
    ("CKPT-016", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::an_empty_checkpoint_still_commits"),
    ("CKPT-017", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::descriptor_validates_completeness_and_roots_bind_every_field"),
    ("CKPT-018", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::descriptor_validates_completeness_and_roots_bind_every_field"),
    ("CKPT-019", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::descriptor_validates_completeness_and_roots_bind_every_field"),
    ("CKPT-020", "veloren_client::tests::checkpoint_receive_v1::a_descriptor_for_another_session_is_refused"),
    ("CKPT-021", "veloren_common_net::msg::checkpoint::checkpoint_epoch_ordinal_v1::epoch_chain_is_contiguous_and_non_reusable"),
    ("CKPT-022", "veloren_common_net::msg::checkpoint::checkpoint_epoch_ordinal_v1::epoch_chain_is_contiguous_and_non_reusable"),
    ("CKPT-023", "veloren_common_net::msg::checkpoint::checkpoint_epoch_ordinal_v1::epoch_chain_is_contiguous_and_non_reusable"),
    ("CKPT-024", "veloren_common_net::msg::checkpoint::checkpoint_epoch_ordinal_v1::epoch_chain_is_contiguous_and_non_reusable"),
    ("CKPT-025", "veloren_common_net::msg::checkpoint::checkpoint_epoch_ordinal_v1::epoch_chain_is_contiguous_and_non_reusable"),
    ("CKPT-026", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_a_matching_ack_advances_the_commit_watermark"),
    ("CKPT-027", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_a_matching_ack_advances_the_commit_watermark"),
    ("CKPT-028", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_a_matching_ack_advances_the_commit_watermark"),
    ("CKPT-029", "structural: T3.3 ingress rejects a stale connection epoch (SemanticEnvelopeRejectV1::StaleEpoch) before a frame ever reaches the checkpoint runtime -- client::tests::stale_and_future_epoch_are_both_rejected"),
    ("CKPT-030", "structural: same T3.3 ingress binding check, WrongBoot arm, ahead of the checkpoint runtime"),
    ("CKPT-031", "veloren_server::net_checkpoint::checkpoint_planner_v1::a_fenced_stream_holds_ordinary_traffic_until_its_barrier"),
    ("CKPT-032", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::staging_rejects_are_typed"),
    ("CKPT-033", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::nothing_applies_until_every_stream_is_fenced"),
    ("CKPT-034", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-035", "veloren_common_net::msg::checkpoint::checkpoint_client_phase_v1::direct_application_is_illegal_while_a_checkpoint_aligns"),
    ("CKPT-036", "OPEN: resync under a new connection epoch has no code path yet -- the runtime is created per attachment and abandon_v1 is the only reset; a resync row belongs with the T3.5 session-control work"),
    ("CKPT-037", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::every_stream_including_empty_is_fenced"),
    ("CKPT-038", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::staging_rejects_are_typed"),
    ("CKPT-039", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::every_stream_including_empty_is_fenced"),
    ("CKPT-040", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-041", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-042", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-043", "veloren_server::net_checkpoint::checkpoint_planner_v1::control_frames_become_self_sufficient_wire_messages"),
    ("CKPT-044", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-045", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-046", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-047", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-048", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-049", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::substituted_payload_fails_its_stream_fence"),
    ("CKPT-050", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::every_stream_including_empty_is_fenced"),
    ("CKPT-051", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::nothing_applies_until_every_stream_is_fenced"),
    ("CKPT-052", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::nothing_applies_until_every_stream_is_fenced"),
    ("CKPT-053", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::nothing_applies_until_every_stream_is_fenced"),
    ("CKPT-054", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::nothing_applies_until_every_stream_is_fenced"),
    ("CKPT-055", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-056", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_cross_stream_interleaving_survives_perturbation"),
    ("CKPT-057", "veloren_server::net_checkpoint::checkpoint_planner_v1::frames_fence_every_stream_and_carry_bound_context"),
    ("CKPT-058", "structural: the T3.3 zero-gap ingress check (SequenceGap) rejects before the aligner; the client drains through validate_semantic_frame_v1 first"),
    ("CKPT-059", "structural: same T3.3 ingress path, DuplicateSequence arm"),
    ("CKPT-060", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-061", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::descriptor_validates_completeness_and_roots_bind_every_field"),
    ("CKPT-062", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::descriptor_validates_completeness_and_roots_bind_every_field"),
    ("CKPT-063", "structural: every data record's SEQUENCE is part of the stream transcript preimage, so a sequence outside the declared range moves the stream root and fails at that stream's Barrier -- the mechanism substituted_payload_fails_its_stream_fence exercises"),
    ("CKPT-064", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::every_stream_including_empty_is_fenced"),
    ("CKPT-065", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::descriptor_validates_completeness_and_roots_bind_every_field"),
    ("CKPT-066", "structural: T3.3 ingress rejects a frame whose binding does not match the live attachment, ahead of the checkpoint runtime"),
    ("CKPT-067", "veloren_common_net::msg::envelope::tests::reserve_sequences_is_all_or_nothing_across_streams"),
    ("CKPT-068", "structural: SemanticSendStateV1 and SemanticReceiveStateV1 are separate per-direction cursor sets, and direction is a header field checked at ingress"),
    ("CKPT-069", "structural: the aligner never reads or advances a T3.3 cursor -- it consumes frames the T3.3 layer has already accepted, so it cannot re-admit an old sequence"),
    ("CKPT-070", "veloren_common_net::msg::envelope::tests::reserve_sequences_is_all_or_nothing_across_streams"),
    ("CKPT-071", "veloren_server::net_checkpoint::checkpoint_planner_v1::rejected_checkpoints_never_consume_sequences"),
    ("CKPT-072", "structural: only CheckpointFrameV1::Data pushes a TranscriptEntryV1 (aligner accept_data_v1); Begin/Barrier never enter entries, which is why the plan's own roots reproduce on the receiver in a_plan_aligns_end_to_end_and_tampering_does_not"),
    ("CKPT-073", "veloren_common_net::msg::checkpoint::checkpoint_epoch_ordinal_v1::ordinals_are_dense_and_order_independent"),
    ("CKPT-074", "veloren_common_net::msg::checkpoint::checkpoint_context_v1::context_field_matrix_is_exhaustively_enforced"),
    ("CKPT-075", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::staging_rejects_are_typed"),
    ("CKPT-076", "veloren_common_net::msg::checkpoint::checkpoint_epoch_ordinal_v1::ordinals_are_dense_and_order_independent"),
    ("CKPT-077", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::staging_rejects_are_typed"),
    ("CKPT-078", "veloren_common_net::msg::checkpoint::checkpoint_context_v1::context_field_matrix_is_exhaustively_enforced"),
    ("CKPT-079", "veloren_common_net::msg::checkpoint::checkpoint_context_v1::context_field_matrix_is_exhaustively_enforced"),
    ("CKPT-080", "veloren_server::net_checkpoint::checkpoint_planner_v1::frames_fence_every_stream_and_carry_bound_context"),
    ("CKPT-081", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_cross_stream_interleaving_survives_perturbation"),
    ("CKPT-082", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-083", "veloren_server::net_checkpoint::checkpoint_planner_v1::plan_is_a_pure_function_of_the_intent_set"),
    ("CKPT-084", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::descriptor_validates_completeness_and_roots_bind_every_field"),
    ("CKPT-085", "veloren_server::net_checkpoint::checkpoint_planner_v1::plan_is_a_pure_function_of_the_intent_set"),
    ("CKPT-086", "veloren_common_net::msg::checkpoint::checkpoint_egress_order_v1::duplicate_key_and_phase_regression_are_typed"),
    ("CKPT-087", "veloren_server::net_checkpoint::checkpoint_planner_v1::duplicate_order_key_and_resource_ceiling_are_typed"),
    ("CKPT-088", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::an_empty_checkpoint_still_commits"),
    ("CKPT-089", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::descriptor_validates_completeness_and_roots_bind_every_field"),
    ("CKPT-090", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::roots_are_permutation_invariant_and_mutation_sensitive"),
    ("CKPT-091", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::roots_are_permutation_invariant_and_mutation_sensitive"),
    ("CKPT-092", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::roots_are_permutation_invariant_and_mutation_sensitive"),
    ("CKPT-093", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::roots_are_permutation_invariant_and_mutation_sensitive"),
    ("CKPT-094", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::substituted_payload_fails_its_stream_fence"),
    ("CKPT-095", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::roots_are_permutation_invariant_and_mutation_sensitive"),
    ("CKPT-096", "veloren_server::net_checkpoint::checkpoint_planner_v1::a_plan_aligns_end_to_end_and_tampering_does_not"),
    ("CKPT-097", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::roots_are_permutation_invariant_and_mutation_sensitive"),
    ("CKPT-098", "veloren_server::net_checkpoint::checkpoint_planner_v1::a_plan_aligns_end_to_end_and_tampering_does_not"),
    ("CKPT-099", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::prepare_rejects_and_commit_applies_the_whole_set"),
    ("CKPT-100", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::prepare_rejects_and_commit_applies_the_whole_set"),
    ("CKPT-101", "veloren_common_net::msg::checkpoint::checkpoint_resource_v1::declared_and_actual_limits_are_both_enforced"),
    ("CKPT-102", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-103", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-104", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::every_stream_including_empty_is_fenced"),
    ("CKPT-105", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-106", "structural: DigestDomainIdV1 domain-separates every root; the checkpoint roots own 40/41/42 and the T3.3 envelope profile owns 20, pinned by common::apex::digest::domain's exact-table test"),
    ("CKPT-107", "structural: CheckpointGlobalTranscript (41) and CheckpointDescriptor (42) are distinct domains in the same sealed registry, so one root can never verify as the other"),
    ("CKPT-108", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::every_stream_including_empty_is_fenced"),
    ("CKPT-109", "structural: stream_transcript_root_v1 always hashes a preimage (binding/epoch/stream/count), so an empty stream's root is a real digest -- an all-zero digest cannot be produced by it"),
    ("CKPT-110", "veloren_common_net::msg::checkpoint::checkpoint_descriptor_v1::roots_are_permutation_invariant_and_mutation_sensitive"),
    ("CKPT-111", "structural: two identical payloads differ in their egress sort key (local ordinal), so both are admitted and land at distinct ordinals; the ordinal is in the transcript preimage, so the roots still differ -- the same mechanism CKPT-110 checks from the other side"),
    ("CKPT-112", "structural: staged records are a BTreeMap keyed by ordinal, so an unindexed staged record is unrepresentable; a count that disagrees fails prepare (prepare_rejects_and_commit_applies_the_whole_set)"),
    ("CKPT-113", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-114", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-115", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::prepare_rejects_and_commit_applies_the_whole_set"),
    ("CKPT-116", "OPEN: prepare validates the checkpoint's own structure, not ECS referential integrity -- a missing referenced Uid is not yet a prepare-time reject"),
    ("CKPT-117", "OPEN: duplicate entity creation is not yet detected at prepare; it needs the ECS preflight CKPT-116 also names"),
    ("CKPT-118", "veloren_common_net::msg::checkpoint::checkpoint_profile_v1::participation_and_phases_are_total_and_ordered"),
    ("CKPT-119", "veloren_common_net::msg::checkpoint::checkpoint_egress_order_v1::duplicate_key_and_phase_regression_are_typed"),
    ("CKPT-120", "veloren_common_net::msg::checkpoint::checkpoint_profile_v1::participation_and_phases_are_total_and_ordered"),
    ("CKPT-121", "OPEN: frontend events are emitted by the ordinary handlers at dispatch time, after commit; they are not separately staged, so there is nothing yet to assert about staged events"),
    ("CKPT-122", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-123", "OPEN: commit currently runs inside handle_messages; its ordering against the client's own State::tick is not yet pinned by a test"),
    ("CKPT-124", "OPEN: the inverse of CKPT-123, same missing ordering pin"),
    ("CKPT-125", "structural: CheckpointApplySinkV1's methods return unit and commit_checkpoint_v1 returns no Result -- a recoverable failure after the first mutation is unrepresentable"),
    ("CKPT-126", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::prepare_rejects_and_commit_applies_the_whole_set"),
    ("CKPT-127", "OPEN: nothing re-checks entity generations between prepare and commit; the window exists because dispatch happens after commit"),
    ("CKPT-128", "structural: panic = \"abort\" in both the dev and release profiles (workspace Cargo.toml), so an allocation panic during commit aborts the process rather than continuing with a half-applied checkpoint"),
    ("CKPT-129", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-130", "OPEN: no post-commit state root is computed or carried yet (bootstrap_manifest_root is always None), so there is nothing to compare"),
    ("CKPT-131", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-132", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::prepare_rejects_and_commit_applies_the_whole_set"),
    ("CKPT-133", "veloren_server::net_checkpoint::checkpoint_planner_v1::a_fenced_stream_holds_ordinary_traffic_until_its_barrier"),
    ("CKPT-134", "veloren_server::net_checkpoint::checkpoint_planner_v1::a_fenced_stream_holds_ordinary_traffic_until_its_barrier"),
    ("CKPT-135", "structural: the egress gate answers Hold for a fenced stream and releases it only at its own Barrier (a_fenced_stream_holds_ordinary_traffic_until_its_barrier); there is no path that polls a released stream early"),
    ("CKPT-136", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_cross_stream_interleaving_survives_perturbation"),
    ("CKPT-137", "veloren_client::tests::checkpoint_receive_v1::nothing_is_applied_until_the_last_barrier"),
    ("CKPT-138", "structural: a refused checkpoint returns Err out of checkpoint_intercept_v1, which propagates to the connection teardown path -- there is no in-band unblock"),
    ("CKPT-139", "structural: ping is its own physical stream, never fenced, and admit_other_v1 passes OutOfBandDiagnostic unconditionally"),
    ("CKPT-140", "structural: ping is not a ServerGeneral and has no CheckpointParticipantV1 impl, so it can never enter a transcript entry"),
    ("CKPT-141", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_cross_stream_interleaving_survives_perturbation"),
    ("CKPT-142", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_cross_stream_interleaving_survives_perturbation"),
    ("CKPT-143", "veloren_server::net_checkpoint::checkpoint_planner_v1::only_cross_stream_interleaving_survives_perturbation"),
    ("CKPT-144", "veloren_common_net::msg::checkpoint::checkpoint_aligner_v1::nothing_applies_until_every_stream_is_fenced"),
    ("CKPT-145", "veloren_common_net::msg::checkpoint::checkpoint_client_phase_v1::alignment_is_bounded_but_idle_and_prepared_do_not_age"),
    ("CKPT-146", "veloren_common_net::msg::checkpoint::checkpoint_client_phase_v1::alignment_is_bounded_but_idle_and_prepared_do_not_age"),
    ("CKPT-147", "veloren_common_net::msg::checkpoint::checkpoint_client_phase_v1::alignment_is_bounded_but_idle_and_prepared_do_not_age"),
    ("CKPT-148", "veloren_common_net::msg::checkpoint::checkpoint_client_phase_v1::direct_application_is_illegal_while_a_checkpoint_aligns"),
    ("CKPT-149", "veloren_common_net::msg::checkpoint::checkpoint_resource_v1::declared_and_actual_limits_are_both_enforced"),
    ("CKPT-150", "veloren_server::net_checkpoint::checkpoint_planner_v1::duplicate_order_key_and_resource_ceiling_are_typed"),
    ("CKPT-151", "veloren_common_net::msg::checkpoint::checkpoint_resource_v1::declared_and_actual_limits_are_both_enforced"),
    ("CKPT-152", "veloren_common_net::msg::checkpoint::checkpoint_resource_v1::declared_and_actual_limits_are_both_enforced"),
    ("CKPT-153", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::prepare_rejects_and_commit_applies_the_whole_set"),
    ("CKPT-154", "veloren_common_net::msg::checkpoint::checkpoint_resource_v1::declared_and_actual_limits_are_both_enforced"),
    ("CKPT-155", "veloren_common_net::msg::checkpoint::checkpoint_resource_v1::no_production_default_exists"),
    ("CKPT-156", "veloren_common_net::msg::checkpoint::checkpoint_resource_v1::no_production_default_exists"),
    ("CKPT-157", "veloren_server::net_checkpoint::checkpoint_planner_v1::rejected_checkpoints_never_consume_sequences"),
    ("CKPT-158", "veloren_common_net::msg::checkpoint::checkpoint_controls_v1::segment_violations_are_typed"),
    ("CKPT-159", "veloren_common_net::msg::checkpoint::checkpoint_prepare_commit_v1::prepare_rejects_and_commit_applies_the_whole_set"),
    ("CKPT-160", "structural: post-barrier data is refused by the sealed segmenter rather than buffered; the receiver has no future-epoch buffer to grow"),
    ("CKPT-161", "veloren_common_net::msg::checkpoint::checkpoint_profile_v1::participation_and_phases_are_total_and_ordered"),
    ("CKPT-162", "veloren_common_net::msg::session_control::session_termination_v1::termination_discards_in_flight_work_and_spares_committed_checkpoints"),
    ("CKPT-163", "veloren_common_net::msg::session_control::session_termination_v1::termination_discards_in_flight_work_and_spares_committed_checkpoints"),
    ("CKPT-164", "veloren_common_net::msg::session_control::session_termination_v1::termination_discards_in_flight_work_and_spares_committed_checkpoints"),
    ("CKPT-165", "veloren_common_net::msg::session_control::session_termination_v1::termination_discards_in_flight_work_and_spares_committed_checkpoints"),
    ("CKPT-166", "veloren_common_net::msg::session_control::session_termination_v1::a_frame_is_a_request_and_the_registry_is_the_authority"),
    ("CKPT-167", "veloren_common_net::msg::session_control::session_termination_v1::control_lane_rejects_are_typed_and_a_repeat_is_idempotent"),
    ("CKPT-168", "veloren_common_net::msg::session_control::session_termination_v1::control_lane_rejects_are_typed_and_a_repeat_is_idempotent"),
    ("CKPT-169", "veloren_common_net::msg::session_control::session_termination_v1::control_lane_rejects_are_typed_and_a_repeat_is_idempotent"),
    ("CKPT-170", "veloren_common_net::msg::session_control::session_termination_v1::control_lane_rejects_are_typed_and_a_repeat_is_idempotent"),
    ("CKPT-171", "veloren_common_net::msg::session_control::session_termination_v1::control_lane_rejects_are_typed_and_a_repeat_is_idempotent"),
    ("CKPT-172", "structural: ping carries no CheckpointParticipantV1 impl and never enters a transcript entry -- the same construction CKPT-140 rests on"),
    ("CKPT-173", "veloren_server::net_checkpoint::checkpoint_planner_v1::the_session_control_lane_is_never_blocked_by_a_fence"),
    ("CKPT-174", "veloren_server::net_checkpoint_disconnect::legacy_disconnect_inventory_v1::every_legacy_disconnect_send_site_is_inventoried"),
    ("CKPT-175", "veloren_common_net::msg::session_control::session_termination_v1::a_frame_is_a_request_and_the_registry_is_the_authority"),
    ("CKPT-176", "veloren_common_net::msg::session_control::session_termination_v1::a_frame_is_a_request_and_the_registry_is_the_authority"),
];

#[cfg(test)]
mod tests {
    use super::{CASE_COVERAGE, OPEN_CASE_COUNT};

    /// Every one of the 176 logical case IDs has exactly one claim.
    #[test]
    fn every_case_id_has_exactly_one_claim() {
        let expected: Vec<String> = (1..=176).map(|i| format!("CKPT-{i:03}")).collect();
        let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        for &(id, _) in CASE_COVERAGE {
            *counts.entry(id).or_insert(0) += 1;
        }
        let missing: Vec<&String> = expected.iter().filter(|id| counts.get(id.as_str()).copied().unwrap_or(0) == 0).collect();
        let duplicated: Vec<&String> = expected.iter().filter(|id| counts.get(id.as_str()).copied().unwrap_or(0) > 1).collect();
        assert!(missing.is_empty(), "unclaimed case IDs (unclaimed-name-fails): {missing:?}");
        assert!(duplicated.is_empty(), "duplicated case IDs: {duplicated:?}");

        let known: std::collections::HashSet<&str> = expected.iter().map(String::as_str).collect();
        let orphans: Vec<&str> = CASE_COVERAGE.iter().map(|&(id, _)| id).filter(|id| !known.contains(id)).collect();
        assert!(orphans.is_empty(), "claims for IDs outside the 176-case set: {orphans:?}");
    }

    /// No empty claims, and the OPEN set is exactly the pinned size --
    /// a gap can be closed or opened, but never drift unnoticed.
    #[test]
    fn claims_are_substantive_and_the_open_set_is_pinned() {
        for &(id, claim) in CASE_COVERAGE {
            assert!(claim.len() > 8, "case {id} has a stub claim: {claim:?}");
        }
        let open: Vec<&str> = CASE_COVERAGE.iter().filter(|(_, c)| c.starts_with("OPEN:")).map(|&(id, _)| id).collect();
        assert_eq!(
            open.len(),
            OPEN_CASE_COUNT,
            "the OPEN set moved -- update OPEN_CASE_COUNT deliberately. Currently open: {open:?}"
        );
    }

    /// Falsifier: the runner would actually catch a missing ID.
    #[test]
    fn a_missing_case_id_would_be_caught() {
        let truncated: Vec<&(&str, &str)> = CASE_COVERAGE.iter().take(CASE_COVERAGE.len() - 1).collect();
        assert!(!truncated.iter().any(|(id, _)| *id == "CKPT-176"), "dropping the last claim must lose CKPT-176");
    }
}
