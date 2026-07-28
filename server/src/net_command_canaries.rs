//! `APEX-T3.5.22` — companion-canary coverage map for the 162 cases of
//! `readme/apex/PROJECT-BASTION-APEX-T3.5-COMMAND-IDEMPOTENCY-CANARIES-v1.json`
//! (pin-verified at import: sha256 01d280e7f215f8315b5b1d79c1229e0eaeed2ba591042cb5eca1d72a579a888e).
//! Same "unclaimed-name-fails" standard `T3.3.20` set and `T3.4.23`
//! followed: a case ID with no entry here is a build failure.
//!
//! Claim kinds: a `crate::path::to::test_fn` names a test that drives the
//! exact outcome; `"structural: ..."` is covered by construction with the
//! reasoning inline; `"OPEN: ..."` is a case this tier does NOT cover,
//! named rather than papered over, with the count pinned below.
//!
//! The OPEN set is concentrated where it should be: client reconnect
//! lifecycle (055/056/061/062/067), the live SQLite and character-worker
//! wiring (116/125/152), server-side session teardown (084), snapshot
//! staleness (136).

pub(crate) const OPEN_CASE_COUNT: usize = 9;

pub(crate) const CASE_COVERAGE: &[(&str, &str)] = &[
    ("CMD-001", "structural: admission_class_v1 matches ClientGeneral exhaustively with NO wildcard arm, so a new variant fails the build rather than defaulting to a class (veloren_server::net_command::command_rollout_v1::enforce_has_no_unclassified_variant_to_admit)"),
    ("CMD-002", "veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-003", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-004", "veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-005", "veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-006", "veloren_common_net::msg::command::command_carriage_v1::every_command_kind_has_a_real_payload"),
    ("CMD-007", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-008", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-009", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-010", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-011", "veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-012", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-013", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-014", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-015", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-016", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-017", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-018", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-019", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-020", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-021", "veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-022", "veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-023", "veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-024", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-025", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-026", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-027", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-028", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-029", "veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-030", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-031", "structural: same exhaustive match arm as the case above it in `CommandParticipantV1 for ClientGeneral`; the arm is asserted by veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is"),
    ("CMD-032", "veloren_common_net::msg::command::command_outbox_v1::one_in_flight_distinct_ids_and_no_auto_retry_of_session_control"),
    ("CMD-033", "structural: CommandId is a T0.4 opaque id whose constructor validates the UUIDv4 invariant; a non-v4 value cannot be built or decoded"),
    ("CMD-034", "veloren_common_net::msg::command::command_id_source_v1::ids_are_derived_not_drawn_and_never_collide_across_sessions"),
    ("CMD-035", "veloren_common_net::msg::command::command_outbox_v1::retries_carry_the_same_identity_and_execute_once_end_to_end"),
    ("CMD-036", "veloren_common_net::msg::command::command_journal_v1::the_journal_survives_a_resume_and_refuses_foreign_or_stale_attachments"),
    ("CMD-037", "veloren_common_net::msg::command::command_journal_v1::sequence_gaps_reuse_and_unacked_terminals_are_all_typed"),
    ("CMD-038", "structural: next_expected_v1 saturates at u64::MAX and a journal at the top admits nothing further, so the sequence cannot wrap into retired territory"),
    ("CMD-039", "structural: every field of the identity preimage is written with to_be_bytes; there is no native-endian path (veloren_common_net::msg::command::command_identity_v1::identity_root_binds_binding_id_kind_and_request)"),
    ("CMD-040", "veloren_common_net::msg::command::command_identity_v1::command_kind_tags_are_explicit_and_total"),
    ("CMD-041", "veloren_common_net::msg::command::command_identity_v1::identity_root_binds_binding_id_kind_and_request"),
    ("CMD-042", "structural: request_digest IS the T3.3 payload digest, whose preimage binds the payload SCHEMA tag alongside the bytes"),
    ("CMD-043", "structural: same T3.3 payload digest, over the exact canonical payload bytes the envelope carried"),
    ("CMD-044", "structural: payload_digest_v1's preimage includes the payload length before the bytes, so a length change moves the digest"),
    ("CMD-045", "veloren_common_net::msg::command::command_journal_v1::sequence_gaps_reuse_and_unacked_terminals_are_all_typed"),
    ("CMD-046", "veloren_common_net::msg::command::command_journal_v1::sequence_gaps_reuse_and_unacked_terminals_are_all_typed"),
    ("CMD-047", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-048", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-049", "veloren_common_net::msg::command::command_journal_v1::the_journal_survives_a_resume_and_refuses_foreign_or_stale_attachments"),
    ("CMD-050", "structural: T3.3 ingress rejects a future ConnectionEpoch (SemanticEnvelopeRejectV1::FutureEpoch) before a frame reaches the command path at all"),
    ("CMD-051", "veloren_common_net::msg::command::command_carriage_v1::command_id_carriage_is_required_exactly_on_commands"),
    ("CMD-052", "structural: the payload is decoded with decode_payload_exact_v1, which refuses trailing bytes"),
    ("CMD-053", "structural: issue_v1 records the pending command BEFORE returning it, so there is no window where a send has happened and the retained descriptor has not"),
    ("CMD-054", "structural: PendingCommandV1 is Copy and its descriptor holds a digest, not the payload; there is nothing retained for a client to mutate"),
    ("CMD-055", "OPEN: the outbox has no reconnect handling yet — a boot change should drop pending commands, and nothing enforces that"),
    ("CMD-056", "OPEN: same missing reconnect handling for a SessionId change"),
    ("CMD-057", "veloren_common_net::msg::command::command_outbox_v1::retries_carry_the_same_identity_and_execute_once_end_to_end"),
    ("CMD-058", "veloren_common_net::msg::command::command_outbox_v1::retries_carry_the_same_identity_and_execute_once_end_to_end"),
    ("CMD-059", "veloren_common_net::msg::command::command_outbox_v1::retries_carry_the_same_identity_and_execute_once_end_to_end"),
    ("CMD-060", "veloren_common_net::msg::command::command_outbox_v1::one_in_flight_distinct_ids_and_no_auto_retry_of_session_control"),
    ("CMD-061", "OPEN: same-session reconnect must preserve the pending command; the outbox does not model reconnect at all yet"),
    ("CMD-062", "OPEN: an explicit logout should clear the outbox, which needs the same missing reconnect/lifecycle seam"),
    ("CMD-063", "veloren_common_net::msg::command::command_result_intake_v1::a_duplicate_result_is_recognised_not_surfaced_twice"),
    ("CMD-064", "veloren_common_net::msg::command::command_publication_v1::a_result_is_not_real_until_its_checkpoint_commits"),
    ("CMD-065", "veloren_common_net::msg::command::command_outbox_v1::one_in_flight_distinct_ids_and_no_auto_retry_of_session_control"),
    ("CMD-066", "structural: a LatestState payload never enters the outbox, so there is nothing for a retry loop to resend (veloren_common_net::msg::command::command_carriage_v1::continuous_input_is_not_journaled_and_chat_is)"),
    ("CMD-067", "OPEN: client-side durable retry state across a process restart is not built; the outbox is in-memory"),
    ("CMD-068", "veloren_common_net::msg::command::command_journal_v1::a_retired_sequence_is_refused_not_re_executed"),
    ("CMD-069", "veloren_common_net::msg::command::command_journal_v1::sequence_gaps_reuse_and_unacked_terminals_are_all_typed"),
    ("CMD-070", "veloren_common_net::msg::command::command_journal_v1::a_retired_sequence_is_refused_not_re_executed"),
    ("CMD-071", "veloren_common_net::msg::command::command_execution_v1::a_command_delivered_many_times_executes_exactly_once"),
    ("CMD-072", "veloren_common_net::msg::command::command_journal_v1::sequence_gaps_reuse_and_unacked_terminals_are_all_typed"),
    ("CMD-073", "veloren_common_net::msg::command::command_journal_v1::sequence_gaps_reuse_and_unacked_terminals_are_all_typed"),
    ("CMD-074", "structural: the sequence is inside the identity root, so the same id under another sequence is a DIFFERENT identity and cannot replay (veloren_common_net::msg::command::command_identity_v1::identity_root_binds_binding_id_kind_and_request)"),
    ("CMD-075", "veloren_common_net::msg::command::command_journal_v1::a_retired_sequence_is_refused_not_re_executed"),
    ("CMD-076", "veloren_common_net::msg::command::command_journal_v1::a_retired_sequence_is_refused_not_re_executed"),
    ("CMD-077", "veloren_common_net::msg::command::command_journal_v1::sequence_gaps_reuse_and_unacked_terminals_are_all_typed"),
    ("CMD-078", "veloren_common_net::msg::command::command_receipt_v1::a_receipt_clears_only_the_command_it_actually_names"),
    ("CMD-079", "veloren_common_net::msg::command::command_receipt_v1::a_receipt_clears_only_the_command_it_actually_names"),
    ("CMD-080", "veloren_common_net::msg::command::command_journal_v1::a_retired_sequence_is_refused_not_re_executed"),
    ("CMD-081", "veloren_common_net::msg::command::command_journal_v1::a_retired_sequence_is_refused_not_re_executed"),
    ("CMD-082", "veloren_common_net::msg::command::command_journal_v1::a_retired_sequence_is_refused_not_re_executed"),
    ("CMD-083", "veloren_common_net::msg::command::command_journal_v1::the_journal_survives_a_resume_and_refuses_foreign_or_stale_attachments"),
    ("CMD-084", "veloren_common_net::msg::session_control::session_termination_v1::a_frame_is_a_request_and_the_registry_is_the_authority"),
    ("CMD-085", "structural: the journal is bound to a ServerBootId and refuses a descriptor carrying another, so it cannot cross a boot (veloren_common_net::msg::command::command_journal_v1::the_journal_survives_a_resume_and_refuses_foreign_or_stale_attachments)"),
    ("CMD-086", "veloren_common_net::msg::command::command_journal_v1::the_journal_survives_a_resume_and_refuses_foreign_or_stale_attachments"),
    ("CMD-087", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-088", "veloren_server::net_command::command_durability_v1::a_rolled_back_effect_writes_no_row"),
    ("CMD-089", "veloren_common_net::msg::command::command_execution_v1::a_refused_admission_never_reaches_the_work"),
    ("CMD-090", "veloren_common_net::msg::command::command_journal_v1::a_retired_sequence_is_refused_not_re_executed"),
    ("CMD-091", "veloren_common_net::msg::command::command_publication_v1::a_result_is_not_real_until_its_checkpoint_commits"),
    ("CMD-092", "veloren_server::net_command::command_ingress_v1::a_command_result_is_checkpointed_data_that_applies_after_its_effect"),
    ("CMD-093", "veloren_common_net::msg::command::command_publication_v1::a_result_is_not_real_until_its_checkpoint_commits"),
    ("CMD-094", "structural: the prepared set is ordered by the T3.4 canonical apply order (phase rank, then ordinal), which is a pure function of the record set"),
    ("CMD-095", "structural: no wall-clock value appears in a command descriptor, receipt, publication or journal entry; ordering is by sequence and checkpoint epoch"),
    ("CMD-096", "veloren_server::net_command::command_workflow_v1::a_retry_does_not_queue_a_second_worker_action"),
    ("CMD-097", "structural: CheckpointApplySinkV1's methods return unit and commit_checkpoint_v1 returns no Result, so a recoverable error after the first mutation is unrepresentable"),
    ("CMD-098", "veloren_server::net_command::command_workflow_v1::every_worker_answer_terminates_and_none_invents_success"),
    ("CMD-099", "veloren_common_net::msg::command::command_execution_v1::a_refusal_is_recorded_and_replayed_like_any_other_outcome"),
    ("CMD-100", "structural: CommandOutcomeV1 carries a result DIGEST and a typed refusal reason; there is no free-text field for unbounded content"),
    ("CMD-101", "veloren_common_net::msg::command::command_publication_v1::a_result_without_a_real_effect_epoch_is_refused"),
    ("CMD-102", "veloren_common_net::msg::command::command_execution_v1::a_command_delivered_many_times_executes_exactly_once"),
    ("CMD-103", "veloren_server::net_command::command_workflow_v1::a_retry_does_not_queue_a_second_worker_action"),
    ("CMD-104", "veloren_server::net_command::command_workflow_v1::a_retry_does_not_queue_a_second_worker_action"),
    ("CMD-105", "veloren_server::net_command::command_workflow_v1::a_retry_does_not_queue_a_second_worker_action"),
    ("CMD-106", "veloren_server::net_command::command_workflow_v1::a_retry_does_not_queue_a_second_worker_action"),
    ("CMD-107", "veloren_server::net_command::command_workflow_v1::a_retry_does_not_queue_a_second_worker_action"),
    ("CMD-108", "veloren_server::net_command::command_workflow_v1::an_in_flight_workflow_is_not_dropped_by_session_close"),
    ("CMD-109", "veloren_server::net_command::command_workflow_v1::every_worker_answer_terminates_and_none_invents_success"),
    ("CMD-110", "veloren_server::net_command::command_workflow_v1::every_worker_answer_terminates_and_none_invents_success"),
    ("CMD-111", "veloren_server::net_command::command_workflow_v1::every_worker_answer_terminates_and_none_invents_success"),
    ("CMD-112", "veloren_common_net::msg::command::command_execution_v1::a_refusal_is_recorded_and_replayed_like_any_other_outcome"),
    ("CMD-113", "veloren_server::net_command::command_workflow_v1::every_worker_answer_terminates_and_none_invents_success"),
    ("CMD-114", "veloren_server::net_command::command_workflow_v1::an_in_flight_workflow_is_not_dropped_by_session_close"),
    ("CMD-115", "veloren_server::net_command::command_workflow_v1::a_retry_does_not_queue_a_second_worker_action"),
    ("CMD-116", "OPEN: ordering the async terminal against the persistence response needs the live character-worker wiring, which is a later live-path row"),
    ("CMD-117", "veloren_server::net_command::command_durability_v1::identity_conflicts_and_the_row_survives_a_new_boot"),
    ("CMD-118", "veloren_server::net_command::command_durability_v1::a_rolled_back_effect_writes_no_row"),
    ("CMD-119", "veloren_server::net_command::command_durability_v1::a_rolled_back_effect_writes_no_row"),
    ("CMD-120", "veloren_server::net_command::command_durability_v1::identity_conflicts_and_the_row_survives_a_new_boot"),
    ("CMD-121", "veloren_server::net_command::command_durability_v1::identity_conflicts_and_the_row_survives_a_new_boot"),
    ("CMD-122", "veloren_server::net_command::command_durability_v1::a_rolled_back_effect_writes_no_row"),
    ("CMD-123", "veloren_server::net_command::command_durability_v1::identity_conflicts_and_the_row_survives_a_new_boot"),
    ("CMD-124", "veloren_server::net_command::command_durability_v1::only_persistence_backed_kinds_are_durable_and_retention_spares_unsettled_rows"),
    ("CMD-125", "OPEN: the real SQLite migration and its uniqueness constraint are a later live-path row; the reference store fixes what that migration must guarantee, and claims nothing about the schema itself"),
    ("CMD-126", "structural: DurableCommandRowV1 stores the outcome, whose Applied arm is a reproducible result digest rather than rendered text"),
    ("CMD-127", "veloren_server::net_command::command_durability_v1::only_persistence_backed_kinds_are_durable_and_retention_spares_unsettled_rows"),
    ("CMD-128", "veloren_server::net_command::command_ingress_v1::a_command_result_is_checkpointed_data_that_applies_after_its_effect"),
    ("CMD-129", "veloren_server::net_command::command_ingress_v1::a_command_result_is_checkpointed_data_that_applies_after_its_effect"),
    ("CMD-130", "veloren_server::net_command::command_ingress_v1::a_command_result_is_checkpointed_data_that_applies_after_its_effect"),
    ("CMD-131", "veloren_server::net_command::command_ingress_v1::a_command_result_is_checkpointed_data_that_applies_after_its_effect"),
    ("CMD-132", "veloren_common_net::msg::command::command_publication_v1::a_result_is_not_real_until_its_checkpoint_commits"),
    ("CMD-133", "veloren_common_net::msg::command::command_result_intake_v1::a_duplicate_result_is_recognised_not_surfaced_twice"),
    ("CMD-134", "structural: a result carries a digest and a typed reason, never localized text, so a replay cannot change what it says"),
    ("CMD-135", "veloren_common_net::msg::command::command_publication_v1::a_result_without_a_real_effect_epoch_is_refused"),
    ("CMD-136", "OPEN: results do not carry a SnapshotEpoch yet; the T3.3 causality field is still dormant, so there is nothing to check staleness against"),
    ("CMD-137", "veloren_common_net::msg::command::command_publication_v1::a_result_is_not_real_until_its_checkpoint_commits"),
    ("CMD-138", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-139", "veloren_common_net::msg::command::command_journal_v1::sequence_gaps_reuse_and_unacked_terminals_are_all_typed"),
    ("CMD-140", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-141", "veloren_server::net_command::command_security_v1::the_journal_records_identity_never_content"),
    ("CMD-142", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-143", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-144", "veloren_common_net::msg::command::command_journal_v1::the_journal_survives_a_resume_and_refuses_foreign_or_stale_attachments"),
    ("CMD-145", "structural: no transport identity appears in the journal, the security session or the descriptor, so a path migration has nothing to reset"),
    ("CMD-146", "veloren_common_net::msg::command::command_outbox_v1::retries_carry_the_same_identity_and_execute_once_end_to_end"),
    ("CMD-147", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-148", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-149", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-150", "veloren_server::net_command::command_security_v1::a_command_id_is_not_a_credential"),
    ("CMD-151", "veloren_common_net::msg::command::command_result_intake_v1::a_result_from_another_session_is_refused"),
    ("CMD-152", "OPEN: a restart cannot currently distinguish failed-before-effect from unknown; that needs the durable row to be consulted on boot, which is the live wiring row CMD-125 also waits on"),
    ("CMD-153", "veloren_server::net_command::command_rollout_v1::enforce_has_no_unclassified_variant_to_admit"),
    ("CMD-154", "veloren_server::net_command::command_rollout_v1::every_payload_takes_exactly_one_path_and_observe_never_journals"),
    ("CMD-155", "veloren_server::net_command::command_rollout_v1::every_payload_takes_exactly_one_path_and_observe_never_journals"),
    ("CMD-156", "veloren_server::net_command_bypass::command_bypass_scan_v1::every_bypass_surface_file_is_classified"),
    ("CMD-157", "veloren_server::net_command_bypass::command_bypass_scan_v1::every_bypass_surface_file_is_classified"),
    ("CMD-158", "veloren_server::net_command_bypass::command_bypass_scan_v1::all_three_named_surfaces_are_scanned"),
    ("CMD-159", "structural: readme/apex/APEX-T3.5-EVIDENCE-BUNDLE-v1.json carries the catalog's sha256 and byte count in its canary_catalog block, and net_command_canaries::tests::the_catalog_this_map_covers_is_the_pinned_file fails if either moves"),
    ("CMD-160", "veloren_server::net_command::command_perturbation_v1::exactly_once_holds_under_every_perturbation_and_seed"),
    ("CMD-161", "veloren_server::net_command::command_perturbation_v1::exactly_once_holds_under_every_perturbation_and_seed"),
    ("CMD-162", "veloren_server::net_command::command_perturbation_v1::the_control_run_diverges_and_names_its_first_divergence"),
];

#[cfg(test)]
mod tests {
    use super::{CASE_COVERAGE, OPEN_CASE_COUNT};

    #[test]
    fn every_case_id_has_exactly_one_claim() {
        let expected: Vec<String> = (1..=162).map(|i| format!("CMD-{i:03}")).collect();
        let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        for &(id, _) in CASE_COVERAGE {
            *counts.entry(id).or_insert(0) += 1;
        }
        let missing: Vec<&String> =
            expected.iter().filter(|id| counts.get(id.as_str()).copied().unwrap_or(0) == 0).collect();
        let duplicated: Vec<&String> =
            expected.iter().filter(|id| counts.get(id.as_str()).copied().unwrap_or(0) > 1).collect();
        assert!(missing.is_empty(), "unclaimed case IDs (unclaimed-name-fails): {missing:?}");
        assert!(duplicated.is_empty(), "duplicated case IDs: {duplicated:?}");

        let known: std::collections::HashSet<&str> = expected.iter().map(String::as_str).collect();
        let orphans: Vec<&str> =
            CASE_COVERAGE.iter().map(|&(id, _)| id).filter(|id| !known.contains(id)).collect();
        assert!(orphans.is_empty(), "claims for IDs outside the 162-case set: {orphans:?}");
    }

    #[test]
    fn claims_are_substantive_and_the_open_set_is_pinned() {
        for &(id, claim) in CASE_COVERAGE {
            assert!(claim.len() > 8, "case {id} has a stub claim: {claim:?}");
        }
        let open: Vec<&str> =
            CASE_COVERAGE.iter().filter(|(_, c)| c.starts_with("OPEN:")).map(|&(id, _)| id).collect();
        assert_eq!(
            open.len(),
            OPEN_CASE_COUNT,
            "the OPEN set moved -- update OPEN_CASE_COUNT deliberately. Currently open: {open:?}"
        );
    }

    /// The catalog file the map claims to cover is the pinned one.
    #[test]
    fn the_catalog_this_map_covers_is_the_pinned_file() {
        // Through the program's own T0.3 artifact-identity seam, not a
        // bespoke sha2 call: exact-byte identity is exactly what that
        // function is for, and it also pins the size.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("server has a parent")
            .join("readme/apex/PROJECT-BASTION-APEX-T3.5-COMMAND-IDEMPOTENCY-CANARIES-v1.json");
        let bytes = std::fs::read(&path).expect("the imported catalog is in the tree");
        let identity = common::apex::digest::hash_artifact_bytes_v1(&bytes);
        assert_eq!(
            identity.digest.bytes.to_human_v1(),
            "sha256:01d280e7f215f8315b5b1d79c1229e0eaeed2ba591042cb5eca1d72a579a888e",
            "the catalog changed under this coverage map; re-verify the pin before touching claims"
        );
        assert_eq!(identity.size_bytes, 11308, "the pinned catalog is 11308 bytes");
    }
}
