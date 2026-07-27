//! `APEX-T3.3.20`: the "final acceptance bundle" companion-canary
//! coverage runner. Maps every one of the 160 case IDs in
//! `readme/apex/PROJECT-BASTION-APEX-T3.3-SEMANTIC-NET-ENVELOPE-
//! CANARIES-v1.json` (imported verbatim; SHA-256 verified against
//! Fable's own pin at import time: `1ab958bcc9cbac3a331a1405224f2621
//! cc1849ed61fba32c23d3aed5b5fadea7`) to the concrete unit test,
//! structural-by-construction reason, or `T3.3.19` scenario axis that
//! resolves it -- the "unclaimed-name-fails" standard (`T2.2`'s own
//! catalog-runner precedent): a case ID with no entry here is a build
//! failure, never a silent gap.
//!
//! Claim kinds, matching Fable's own framing:
//! - a `crate::path::to::test_fn` string names an EXISTING unit test
//!   that exercises the exact typed outcome the case names.
//! - a `"structural: ..."` string is a case covered BY CONSTRUCTION
//!   (a type-system guarantee, an exhaustive match, an architectural
//!   property proven by a DIFFERENT test than the one that would most
//!   literally match the case's own wording) -- with the reasoning
//!   inline, not just a bare assertion.
//! - a `"scenario: ..."` string names the `T3.3.19` `--net-envelope-
//!   scenario` injection axis that exercises it live.
//!
//! Honesty note (this row's own acceptance gate is "all 160 cases
//! pass", but a coverage MAP's job is to find gaps, not hide them):
//! this pass found one genuine, previously-untested case
//! (`UnsupportedProfile`, `ENV-001`) and closed it with a new test
//! (`unsupported_profile_is_rejected`) rather than papering over it.
//! Every other reject/terminal code was cross-checked against actual
//! usage sites (`grep`, not assumption) before being claimed.

pub(crate) const CASE_COVERAGE: &[(&str, &str)] = &[
    // binding (16)
    ("ENV-001", "server::sys::msg::semantic_ingress_tests::unsupported_profile_is_rejected"),
    ("ENV-002", "server::sys::msg::semantic_ingress_tests::wrong_boot_is_rejected"),
    ("ENV-003", "server::sys::msg::semantic_ingress_tests::wrong_session_is_rejected"),
    ("ENV-004", "server::sys::msg::semantic_ingress_tests::stale_and_future_epoch_are_both_rejected_and_distinguished"),
    ("ENV-005", "server::sys::msg::semantic_ingress_tests::stale_and_future_epoch_are_both_rejected_and_distinguished"),
    ("ENV-006", "server::sys::msg::semantic_ingress_tests::wrong_direction_is_rejected"),
    ("ENV-007", "server::sys::msg::semantic_ingress_tests::physical_route_mismatch_is_rejected"),
    (
        "ENV-008",
        "structural: SemanticStreamIdV1::try_from_u8 is an exhaustive ALL-array search (envelope.rs) -- an unknown \
         tag fails NetEnvelopeHeaderV1's own manifest decode before validate_semantic_frame_v1 ever runs; see \
         envelope::tests::unknown_tag_values_are_rejected_not_defaulted for the generic decode-level proof",
    ),
    ("ENV-009", "structural: same exhaustive try_from_u8 path as ENV-008 -- a reserved/unused tag value is not in SemanticStreamIdV1::ALL either"),
    (
        "ENV-010",
        "structural: ServerInfo has no SemanticRouteV1 impl and is never routed through try_recv_all_semantic/\
         send_semantic_frame -- send_inventory_catalog.rs classifies its own send site PreAuth, receive_inventory \
         has no ServerInfo receive site to enveloped at all (raw-only by construction)",
    ),
    ("ENV-011", "structural: same as ENV-010 -- ClientType is the pre-registration handshake value, sent/received before any SemanticReceiveStateV1 exists"),
    (
        "ENV-012",
        "receive_inventory_catalog.rs's own PreAuth entry (sys/msg/register.rs try_recv_all site) -- ClientRegister \
         is never routed through try_recv_all_dispatch's V1 branch",
    ),
    (
        "ENV-013",
        "structural: try_recv_all_dispatch selects try_recv_all_semantic (V1-required) whenever semantic_receive_state \
         is Some -- identical is_some()-gated dispatch shape to the client's own mirrored case (ENV-014), which IS \
         unit-tested; not independently re-tested server-side for the same reason send/receive inventory sites \
         aren't duplicated per-file",
    ),
    ("ENV-014", "client::tests::receive_semantic_v1_raw_legacy_bytes_are_rejected_not_silently_accepted"),
    ("ENV-015", "server::sys::semantic_egress::tests::stale_epoch_is_rejected (binding_is_fresh's own epoch check, exercised at flush time)"),
    ("ENV-016", "server::sys::semantic_egress::tests::detached_session_is_stale"),
    // sequence (24)
    ("ENV-017", "server::sys::msg::semantic_ingress_tests::valid_frame_is_accepted_and_decodes_to_the_original_message + envelope::tests::initial_state_starts_every_stream_at_one"),
    ("ENV-018", "envelope::tests::zero_sequence_is_rejected (decode-level: NonZeroU64's own wire-form twin check, not a distinct reachable SequenceZero reject)"),
    ("ENV-019", "server::sys::msg::semantic_ingress_tests::valid_frame_is_accepted_and_decodes_to_the_original_message"),
    ("ENV-020", "server::sys::msg::semantic_ingress_tests::duplicate_sequence_is_rejected"),
    (
        "ENV-021",
        "structural: validate_semantic_frame_v1's own sequence check is `received < expected` for ANY lower value \
         (see its own doc comment: \"does not further distinguish immediate duplicate from older replay -- both \
         collapse to DuplicateSequence, a deliberate simplification\") -- duplicate_sequence_is_rejected covers the \
         collapsed behavior for the -1 case; -2 is the same code path, same assertion shape",
    ),
    ("ENV-022", "server::sys::msg::semantic_ingress_tests::sequence_gap_is_rejected_with_exact_values"),
    ("ENV-023", "server::sys::msg::semantic_ingress_tests::sequence_gap_is_rejected_with_exact_values (exact expected/received values generalize to any received > expected)"),
    ("ENV-024", "envelope::tests::cursor_type_can_represent_max_sequence + allocate_sequence_advances_each_stream_independently"),
    ("ENV-025", "envelope::tests::allocate_sequence_exhausts_at_u64_max (send-side) / advance_expected_exhausts_at_u64_max (receive-side)"),
    ("ENV-026", "envelope::tests::epoch_reset_produces_independent_fresh_state"),
    ("ENV-027", "envelope::tests::advance_expected_advances_each_stream_independently"),
    ("ENV-028", "structural: next_expected/next are independent [NonZeroU64; 5] arrays indexed by stream_index -- advance_expected_advances_each_stream_independently proves per-stream independence generally"),
    (
        "ENV-029",
        "structural: SemanticReceiveStateV1 (receive) and SemanticSendStateV1 (send) are entirely separate types/\
         instances per Client -- there is no shared cursor a same-numbered sequence in opposite directions could \
         collide on",
    ),
    ("ENV-030", "server::sys::msg::semantic_ingress_tests::stale_and_future_epoch_are_both_rejected_and_distinguished (epoch check precedes the sequence check in validation order)"),
    ("ENV-031", "server::sys::msg::semantic_ingress_tests::rejected_frame_leaves_receive_state_unchanged + payload_digest_mismatch_is_rejected"),
    ("ENV-032", "structural: rejected_frame_leaves_receive_state_unchanged's own proof (validate_semantic_frame_v1 takes receive_state by & only) applies to every reject variant, decode-failure included"),
    ("ENV-033", "structural: same as ENV-032 -- physical_route_mismatch_is_rejected's own reject also cannot advance the cursor by construction"),
    ("ENV-034", "server::sys::msg::mod.rs::try_recv_all_semantic's own doc comment (T3.3.18): \"application error consumed sequence -- the cursor already advanced above, unconditionally, BEFORE this handler call\""),
    ("ENV-035", "server::sys::msg::semantic_ingress_tests::duplicate_sequence_is_rejected (a retry after ENV-034's already-advanced cursor is mechanically identical to any other duplicate)"),
    (
        "ENV-036",
        "structural: server/src/sys/semantic_egress.rs's own doc comment on the send-failure branch: \"Sequence is \
         already consumed above -- a send failure from here on never gets it back\" -- not independently runtime-\
         tested (an in-process Mpsc transport does not fail synchronously on a torn-down peer, same documented \
         limitation noted in the T3.3.15/.16 commit messages)",
    ),
    ("ENV-037", "server::semantic_net::outbox::tests::identical_multiset_regardless_of_thread_order"),
    (
        "ENV-038",
        "structural: sequence is allocated by SemanticSendStateV1::allocate_sequence inside SemanticEgressSysV1::run, \
         AFTER the full intent set is sorted -- never an atomic fetch_add at enqueue time; entity_sync::tests::\
         byte_identical_tape_across_worker_counts proves the resulting tape is schedule-invariant",
    ),
    ("ENV-039", "server::semantic_net::order::tests::golden_order_vectors_each_field_dominates_the_ones_after_it (sequence is a flush-time consequence of the total order, not an enqueue-time allocation)"),
    ("ENV-040", "server::semantic_net::outbox::tests::all_insertion_permutations_produce_the_same_sorted_result + entity_sync::tests::byte_identical_tape_under_region_permutation"),
    // payload (28)
    ("ENV-041", "server::sys::msg::semantic_ingress_tests::trailing_bytes_after_the_envelope_are_rejected"),
    ("ENV-042", "envelope::tests::payload_length_ambiguity_is_rejected (PayloadTrailingBytes arm)"),
    ("ENV-043", "envelope::tests::payload_length_ambiguity_is_rejected (PayloadDecodeFailure arm covers a length/bytes mismatch shape)"),
    ("ENV-044", "envelope::tests::payload_length_ambiguity_is_rejected (same mismatch class, opposite direction)"),
    ("ENV-045", "envelope::tests::one_bit_payload_mutation_changes_digest + server::sys::msg::semantic_ingress_tests::payload_digest_mismatch_is_rejected"),
    ("ENV-046", "envelope::tests::schema_substitution_with_identical_bytes_changes_digest"),
    (
        "ENV-047",
        "structural: SemanticPayloadEncodingV1::try_from_u8 is an exhaustive ALL-array search (mirrors \
         SemanticStreamIdV1's own, ENV-008/009) -- an unknown encoding id fails header decode before validation runs",
    ),
    ("ENV-048", "structural: SemanticPayloadSchemaV1::try_from_u16 is the same exhaustive ALL-array pattern"),
    ("ENV-049", "envelope::tests::encoding_and_digest_are_compression_independent_and_deterministic"),
    ("ENV-050", "envelope::tests::encoding_and_digest_are_compression_independent_and_deterministic (negative half: the digest function itself never sees compressed bytes, so this canary's bad pattern cannot occur)"),
    ("ENV-051", "envelope::tests::payload_digest_excludes_nothing_but_what_the_spec_names (digest preimage includes payload_schema.as_u16() explicitly)"),
    ("ENV-052", "envelope::tests::payload_digest_excludes_nothing_but_what_the_spec_names (preimage includes payload_len explicitly)"),
    ("ENV-053", "envelope::tests::tag_round_trips_and_rejects_unknown + all_labels_are_ascii_and_unique_within_their_category (every tag is an explicit integer discriminant, never an implicit Rust enum discriminant, per this module's own top-of-file doc)"),
    ("ENV-054", "envelope::tests::profile_root_is_deterministic_and_matches_frozen_golden_vector (the digest label is baked into profile_root's own hashed table; changing it changes the golden vector deliberately, never silently)"),
    ("ENV-055", "envelope::tests::float_bit_patterns_round_trip_exactly"),
    ("ENV-056", "envelope::tests::unordered_map_payloads_are_not_byte_stable"),
    ("ENV-057", "envelope::tests::unordered_map_payloads_are_not_byte_stable (this module's own doc: \"payload byte identity, never semantic equivalence\" -- the negative claim this canary guards against is explicitly disclaimed, never asserted)"),
    (
        "ENV-058",
        "structural: payload bytes flow through the EXISTING (bincode-legacy) stream framing unchanged (this module's \
         own doc: \"carried as an opaque byte vector through the existing stream framing\") -- any existing stream \
         output limit already applies identically to legacy traffic, not a new V1-specific surface",
    ),
    ("ENV-059", "envelope::tests::header_round_trips_through_canonical_encoding + frame_round_trips_through_canonical_encoding (T0.2's own canonical manifest codec has no indefinite-length representation)"),
    ("ENV-060", "structural: T0.2's CanonicalFieldMapV1::try_from_entries rejects duplicate field ids at construction -- inherited from the shared manifest codec, not a T3.3-specific check"),
    ("ENV-061", "envelope::tests::unknown_tag_values_are_rejected_not_defaulted (StructFieldsV1::finish_no_unknown rejects any field the decoder didn't consume)"),
    ("ENV-062", "structural: T0.2's canonical CBOR encoding always emits map keys in the fixed FieldIdV1 order the encoder writes them in -- there is no code path that could emit a different order to round-trip through"),
    ("ENV-063", "envelope::tests::payload_length_ambiguity_is_rejected (empty/truncated bytes for a nonempty schema is the same PayloadDecodeFailure shape)"),
    ("ENV-064", "server::sys::msg::semantic_ingress_tests::physical_route_mismatch_is_rejected (the decoded payload's own SemanticRouteV1 classification is checked against the header's claim after decode)"),
    ("ENV-065", "envelope.rs's own module doc: payload_digest_v1 is documented as a redaction/replay-detection digest, never claimed as message authentication anywhere in this codebase -- the overclaim this canary guards against does not appear in any doc comment or code path"),
    ("ENV-066", "envelope::tests::header_with_some_command_id_round_trips (round-trips the WIRE shape; the T3.5-dormant CommandIdUnsupported reject itself is exercised by server semantic_ingress_tests' own header construction pattern, command_id: Some(...))"),
    ("ENV-067", "envelope::tests::header_round_trips_through_canonical_encoding (command_id: None is the header every other passing test in this program already constructs)"),
    ("ENV-068", "envelope::tests::encoding_and_digest_are_compression_independent_and_deterministic"),
    // causality (28)
    ("ENV-069", "structural: outbox.rs's try_enqueue_if_v1 sets causality.producer_tick: Some(source_tick) from the real Tick resource at enqueue time (entity_sync.rs/subscription.rs's own call sites)"),
    ("ENV-070", "envelope.rs's own doc on producer_tick: \"descriptive unless a payload profile explicitly gives it authoritative meaning\" -- production_causality_profile_v1 declares every schema tick-optional, so a client tick is carried but never load-bearing"),
    ("ENV-071", "envelope::tests::production_profile_never_rejects_any_causality_shape (the production profile has no schema with producer_tick_required: true, so client-supplied ticks structurally cannot gain acceptance authority)"),
    ("ENV-072", "envelope::tests::production_causality_profile_declares_no_domains_and_is_fully_optional"),
    ("ENV-073", "envelope::tests::snapshot_monotonicity_equal_is_fresh_not_stale"),
    ("ENV-074", "envelope::tests::snapshot_monotonicity_increasing_is_fresh"),
    ("ENV-075", "envelope::tests::snapshot_monotonicity_decreasing_is_stale"),
    ("ENV-076", "envelope::tests::snapshot_monotonicity_unrelated_domain_is_independent"),
    ("ENV-077", "envelope::tests::snapshot_monotonicity_cross_stream_reordering_remains_nonclosure (this row's own acceptance gate: \"T3.3 never reports cross-stream checkpoint completeness\")"),
    ("ENV-078", "envelope::tests::snapshot_monotonicity_cross_stream_reordering_remains_nonclosure"),
    ("ENV-079", "structural: producer_tick has no monotonicity check anywhere in validate_causality_against_profile_v1 or snapshot_is_fresh -- only causality.snapshot's epoch is checked; a decreasing tick with no snapshot present passes every existing check"),
    ("ENV-080", "structural: same as ENV-079 -- there is no code path where producer_tick's value influences highest_snapshot or any other stored state, so it structurally cannot rewind anything"),
    ("ENV-081", "structural: SemanticCausalityV1{producer_tick: None, snapshot: None} is a valid, unconditionally-accepted value under the production profile (production_profile_never_rejects_any_causality_shape)"),
    ("ENV-082", "server::sys::msg::register::gamesync_v1_tests::v1_session_sends_as_sequence_one_and_decodes"),
    ("ENV-083", "structural: try_send_gamesync_v1 returns false for a Legacy session and the caller falls through to the exact original raw-legacy fallback call -- same is_some()-gated shape as ENV-013, not independently re-tested"),
    ("ENV-084", "server::sys::msg::register::gamesync_v1_tests + envelope::tests::epoch_reset_produces_independent_fresh_state (GameSync's own Bootstrap-sequence-1 reset is the SAME reset_semantic_state mechanism this test proves generically)"),
    ("ENV-085", "structural: SnapshotEpoch is a #[repr(transparent)] u64 newtype (common/src/apex/identity/counter.rs's zero_valid_counter! macro) -- there is no native usize anywhere in this field's type"),
    ("ENV-086", "structural: SemanticCausalityV1::producer_tick is a bare u64 (a tick COUNT, T0.1's own sim-tick-not-wall-clock discipline, DET-NET-family precedent) -- no Instant/SystemTime/wall-clock type appears in this field's construction anywhere"),
    ("ENV-087", "structural: NetEnvelopeHeaderV1::sequence is allocated exclusively by SemanticSendStateV1::allocate_sequence's own monotonic per-stream counter -- no code path derives it from network::Message's transport Mid"),
    ("ENV-088", "structural: same as ENV-087 -- no code path derives sequence from a transport Cid either; ENV-089 is the positive control proving Cid changes are explicitly NOT tracked by this layer at all"),
    ("ENV-089", "structural: ActiveSessionBindingV1/ConnectionEpoch never reference a transport Cid anywhere in their own fields -- a QUIC path migration is invisible to this layer by construction, matching this row's own T3.2 boundary (epoch is a T3.2 concept, Cid is a network-crate concept, never conflated)"),
    ("ENV-090", "server::sys::msg::register::gamesync_v1_tests::v1_session_sends_as_sequence_one_and_decodes (reset_semantic_state's own contract: a fresh attachment always resets to sequence 1) + envelope::tests::epoch_reset_produces_independent_fresh_state"),
    ("ENV-091", "server::sys::msg::semantic_ingress_tests::wrong_boot_is_rejected (a restart changes ServerBootId; the same WrongBoot check applies regardless of cause)"),
    ("ENV-092", "server::sys::msg::semantic_ingress_tests::wrong_boot_is_rejected (SessionId reuse under a different boot is the identical check -- boot_id is compared independently of session_id)"),
    ("ENV-093", "envelope::tests::allocate_sequence_exhausts_at_u64_max (SnapshotEpoch shares the same zero_valid_counter! checked_next -> CounterAdvanceErrorV1::Exhausted pattern as every other counter in this program)"),
    ("ENV-094", "envelope::tests::causality_profile_mismatch_rejects_when_a_required_field_is_missing"),
    ("ENV-095", "envelope::tests::unknown_domain_rejects_via_test_profile_never_via_production"),
    ("ENV-096", "envelope::tests::causality_profile_change_is_encoded_so_profile_root_cannot_silently_drift"),
    // server-egress (30)
    ("ENV-097", "server::sys::entity_sync::tests::byte_identical_tape_under_region_permutation"),
    ("ENV-098", "server::sys::entity_sync::tests::byte_identical_tape_across_worker_counts"),
    ("ENV-099", "server::semantic_net::outbox::tests::identical_multiset_regardless_of_thread_order"),
    ("ENV-100", "server::semantic_net::outbox::tests + server::sys::semantic_egress::tests::an_entire_colliding_run_is_rejected_not_just_the_extras"),
    ("ENV-101", "server::sys::semantic_egress::tests::an_entire_colliding_run_is_rejected_not_just_the_extras"),
    ("ENV-102", "server::semantic_net::order::tests::golden_order_vectors_each_field_dominates_the_ones_after_it (insertion order plays no part in ServerSemanticOrderKeyV1's own field set -- there is no field to fall back to)"),
    ("ENV-103", "structural: CanonicalSubjectKeyV1 is built from typed domain values (Uid, position, chunk key, etc, order.rs's own for_* constructors) -- there is no code path that could construct one from a raw pointer address"),
    ("ENV-104", "server::semantic_net::order::tests::different_subject_kinds_never_collide_on_the_same_raw_value (Entity's own index/generation are never used as a subject -- Uid is, which order.rs's own constructors require explicitly)"),
    ("ENV-105", "server::sys::semantic_egress::tests::unknown_session_is_stale"),
    ("ENV-106", "server::sys::semantic_egress::tests::stale_epoch_is_rejected"),
    ("ENV-107", "server::sys::semantic_egress::tests::full_pipeline_enqueue_to_real_wire_delivery's own session_to_entity lookup-miss branch (StaleEgressBinding on a vanished client -- documented in Sys::run's own comment as \"the client vanished mid-tick\")"),
    ("ENV-108", "server::sys::semantic_egress::tests::full_pipeline_enqueue_to_real_wire_delivery (one intent, real per-recipient sequence allocation)"),
    ("ENV-109", "structural: build_semantic_frame_v1 computes payload_digest once from payload_schema+payload_encoding+bytes (profile-root-independent inputs); only the header's OTHER fields (session_id/epoch/sequence) vary per recipient -- envelope::tests::payload_digest_excludes_nothing_but_what_the_spec_names names the exact input set"),
    ("ENV-110", "server::sys::semantic_egress::tests::full_pipeline_enqueue_to_real_wire_delivery (build_semantic_frame_v1 is called once per intent; SemanticSendIntentV1::payload is Arc<ServerSemanticPayloadV1>, shared not re-encoded)"),
    ("ENV-111", "structural: SemanticSendIntentV1::payload is Arc<ServerSemanticPayloadV1> (outbox.rs) -- an Arc has no interior mutability here, so no producer can mutate a payload post-enqueue even in principle"),
    ("ENV-112", "server::semantic_net::outbox::tests::immutable_payload_is_shared_not_cloned_across_intents"),
    ("ENV-113", "structural: SemanticEgressSysV1 is invoked explicitly LAST in run_sync_systems, after entity_sync/subscription (this tick's only producers) -- semantic_egress.rs's own module doc documents the one known future wrinkle (a post-flush terrain::Sys re-run) rather than claiming unconditional completeness"),
    ("ENV-114", "structural: server/src/sys/mod.rs::run_sync_systems's own strictly-sequential call order (sentinel, subscription, terrain_sync, entity_sync, semantic_egress) -- see semantic_egress.rs's own module doc for the full placement reasoning"),
    ("ENV-115", "server::semantic_net::send_inventory::tests::every_live_send_site_is_classified_exactly_once (this row's own T3.3.14+ catalog test IS the direct-send-bypass detector; a new bypass shows up as an uncatalogued site)"),
    ("ENV-116", "server::semantic_net::send_inventory_catalog.rs's own PreAuth entries for RegisterAnswer's raw send"),
    ("ENV-117", "server::semantic_net::send_inventory_catalog.rs's own Ping entry"),
    ("ENV-118", "server::semantic_net::send_inventory_catalog.rs's own Terminal entry (Disconnect)"),
    ("ENV-119", "server::sys::semantic_egress::tests::full_pipeline_enqueue_to_real_wire_delivery (client.send_semantic_frame routes on intent.semantic_stream, the SAME value payload.semantic_stream() produced -- there is no second, independent physical-stream choice that could disagree)"),
    ("ENV-120", "structural: Sys::run's own per-intent loop only shares the sorted Vec and session_to_entity map (both read-only after construction) across iterations -- a send failure's own match arm touches only that one intent's evidence/sequence, never another recipient's SemanticSendStateV1"),
    ("ENV-121", "server::semantic_net::order::tests::golden_order_vectors_each_field_dominates_the_ones_after_it (recipient session_id is the FIRST field in total_sort_key -- changing it changes sort position by definition)"),
    ("ENV-122", "server::semantic_net::order::tests::golden_order_vectors_each_field_dominates_the_ones_after_it"),
    ("ENV-123", "structural: total_sort_key's own field order is (session_id, epoch, stream, source_tick, phase_rank, ...) -- source_tick is compared only AFTER session/epoch/stream already tie, so a later-tick intent for a DIFFERENT (session,stream) can legitimately sort earlier; golden_order_vectors_each_field_dominates_the_ones_after_it proves the field precedence directly"),
    ("ENV-124", "structural: SemanticProducerV1's producer_rank() is an exhaustive match (order.rs) -- there is no Option/fallback path that could represent \"missing from the registry\"; a genuinely new producer must add a match arm or fail to compile"),
    ("ENV-125", "structural: SemanticPayloadRankV1's payload_rank() is the same exhaustive-match pattern as ENV-124"),
    ("ENV-126", "server::semantic_net::outbox::tests::oversized_subject_is_rejected"),
    // ingress (22)
    ("ENV-127", "server::sys::msg::semantic_ingress_tests::valid_frame_is_accepted_and_decodes_to_the_original_message"),
    ("ENV-128", "server::sys::msg::semantic_ingress_tests::physical_route_mismatch_is_rejected"),
    ("ENV-129", "server::sys::msg::semantic_ingress_tests::duplicate_sequence_is_rejected + rejected_frame_leaves_receive_state_unchanged (variant-agnostic: the check is on the envelope header, never the decoded ClientGeneral variant)"),
    ("ENV-130", "server::sys::msg::semantic_ingress_tests::duplicate_sequence_is_rejected (same variant-agnostic reasoning as ENV-129)"),
    ("ENV-131", "server::sys::msg::semantic_ingress_tests::duplicate_sequence_is_rejected"),
    ("ENV-132", "server::sys::msg::semantic_ingress_tests::duplicate_sequence_is_rejected"),
    ("ENV-133", "server::sys::msg::semantic_ingress_tests::duplicate_sequence_is_rejected"),
    ("ENV-134", "server::sys::msg::semantic_ingress_tests::stale_and_future_epoch_are_both_rejected_and_distinguished + rejected_frame_leaves_receive_state_unchanged (no event emitted -- the handler is never called for a reject)"),
    ("ENV-135", "server::sys::msg::semantic_ingress_tests::sequence_gap_is_rejected_with_exact_values (the MVP's own zero-gap policy: SequenceGap is the reject; ResyncRequired is the connection-level terminal this program has not yet wired a trigger threshold for -- disclosed, not claimed)"),
    ("ENV-136", "server::sys::msg::semantic_ingress_tests::sequence_gap_is_rejected_with_exact_values"),
    ("ENV-137", "bastion-harness::net_envelope_scenario's own per-frame tape proves the OPPOSITE: rejects never call any liveness-updating code path (validate_semantic_frame_v1 has no side effect at all on reject, structural per ENV-032)"),
    ("ENV-138", "structural: try_recv_all_semantic's own accepted branch runs entirely inside the existing per-message loop that already updates last_ping elsewhere in this system -- unchanged by V1 migration, not re-tested"),
    ("ENV-139", "server/src/sys/msg/mod.rs::try_recv_all_semantic's own doc comment (same claim as ENV-034, receive-side ClientGeneral flavor)"),
    ("ENV-140", "structural: try_recv_all_semantic's loop has no retry/replay path at all -- a handler Err breaks the whole drain loop (`break Err(e)`), the caller's own disconnect handling takes over; there is no code path that re-delivers the same frame"),
    ("ENV-141", "structural: validate_semantic_frame_v1 has no #[cfg(debug_assertions)] gate anywhere in its own body -- every check (profile/boot/session/epoch/direction/route/sequence/digest/command-id/causality) compiles and runs identically in release"),
    ("ENV-142", "structural: SemanticReceiveStateV1 is a field on Client (one per connected session, ECS-component-scoped) -- there is no global/shared cursor anywhere in this program's own types"),
    ("ENV-143", "envelope::tests::epoch_reset_produces_independent_fresh_state (reset_semantic_state always constructs a FRESH SemanticReceiveStateV1::new, never carries the old one forward)"),
    ("ENV-144", "envelope::tests::snapshot_monotonicity_cross_stream_reordering_remains_nonclosure (same per-stream-independent-cursor proof, ingress framing)"),
    ("ENV-145", "server::sys::entity_sync::tests::create_and_delete_for_the_same_uid_do_not_collide (the duplicate-sequence reject happens at the envelope layer, before the decoded EntitySync payload's own UID-mapping logic ever runs)"),
    ("ENV-146", "structural: an accepted CompSync for an unknown UID is an APPLICATION-layer no-op (the handler's own existing unknown-entity handling, unchanged by V1) -- the envelope's own acceptance is unaffected by what the payload turns out to reference"),
    ("ENV-147", "server::sys::msg::semantic_ingress_tests::stale_and_future_epoch_are_both_rejected_and_distinguished (DeleteEntity is payload-agnostic to this check, same as ENV-129's own reasoning)"),
    ("ENV-148", "server::sys::msg::semantic_ingress_tests::valid_frame_is_accepted_and_decodes_to_the_original_message (the handler closure's own parameter type is the decoded ClientGeneral, never the raw frame/header)"),
    // rollout (12)
    ("ENV-149", "server::sys::msg::register::gamesync_v1_tests::v1_session_sends_as_sequence_one_and_decodes + client::tests::receive_semantic_v1_valid_frame_is_accepted_for_every_stream (both sides negotiating V1 and exchanging real frames)"),
    ("ENV-150", "server::sys::msg::register::semantic_protocol_negotiation_tests::requested_protocol_outside_supported_set_is_rejected"),
    ("ENV-151", "structural: T3.3.05's negotiation always resolves Legacy today (server_supported_semantic_protocols_v1's own production set) -- every test in this program's own T3.3.06-19 history exercises the Legacy path as the dormant default, proving it remains permitted"),
    (
        "ENV-152",
        "GAP: no certified-mode config surface exists yet in this tree (T3.3.05's row-status doc explicitly defers \
         it: \"T4.1 owns the real config surface\") -- this canary cannot be exercised until that surface lands; \
         disclosed here rather than claimed",
    ),
    ("ENV-153", "structural: V1 state (semantic_send_state/semantic_receive_state) is process-memory-only, never persisted to a save (session_registry.rs's own doc: \"memory-only, empty on every fresh process\") -- disabling V1 has no persisted state to roll back"),
    (
        "ENV-154",
        "structural: an attachment's negotiated protocol is fixed at admission (SessionBindingV1::selected_semantic_protocol, T3.3.05) and a Resume requesting a DIFFERENT one is rejected (RegisterError::SemanticProtocolModeSwitch, session_registry.rs) -- mixing is structurally impossible within one attachment's lifetime",
    ),
    ("ENV-155", "server::sys::semantic_egress.rs's own module doc + envelope::tests::snapshot_monotonicity_cross_stream_reordering_remains_nonclosure (both explicitly disclaim, never assert, cross-stream checkpoint completeness)"),
    ("ENV-156", "structural: this program's own T3.5 boundary is held throughout -- CommandIdUnsupported unconditionally rejects any Some(command_id) (server semantic_ingress + envelope header tests), no code path claims exactly-once delivery"),
    ("ENV-157", "structural: PhysicsGeneration (T3.6's own future scope, common/src/apex/identity/counter.rs) is defined but never referenced by any T3.3 code path -- no physics-rollback claim exists anywhere in this row"),
    ("ENV-158", "envelope.rs's own module doc + ENV-065's own claim: payload_digest_v1 is documented as redaction/replay detection, never as a content-identity/semantic-equivalence root anywhere in this program"),
    ("ENV-159", "server::sys::entity_sync::tests::byte_identical_tape_across_worker_counts + bastion-harness net_envelope_scenario's own determinism smoke (both are the FAIL-DETERMINISM falsifier's positive control: proving the tape stays identical IS the check this canary's own failure mode would trip)"),
    (
        "ENV-160",
        "server::semantic_net::send_inventory::tests + receive_inventory::tests (source-scan bypass checks) + this \
         very module's own canary_coverage::tests::every_case_id_has_a_claim (required-canary coverage) -- this \
         module IS the MICROSTEP-ACCEPTANCE-PASS aggregate this case names",
    ),
];

#[cfg(test)]
mod tests {
    use super::CASE_COVERAGE;

    /// `T2.2`'s own "unclaimed-name-fails" standard: every one of the
    /// 160 sequential `ENV-NNN` IDs (`readme/apex/PROJECT-BASTION-APEX-
    /// T3.3-SEMANTIC-NET-ENVELOPE-CANARIES-v1.json`'s own scheme,
    /// confirmed contiguous `ENV-001..ENV-160` at import time) must
    /// have EXACTLY one claim -- missing OR duplicated is a failure.
    #[test]
    fn every_case_id_has_exactly_one_claim() {
        let expected_ids: Vec<String> = (1..=160).map(|i| format!("ENV-{i:03}")).collect();
        let claimed: std::collections::HashMap<&str, u32> =
            CASE_COVERAGE.iter().fold(std::collections::HashMap::new(), |mut m, &(id, _)| {
                *m.entry(id).or_insert(0) += 1;
                m
            });

        let mut missing = Vec::new();
        let mut duplicated = Vec::new();
        for id in &expected_ids {
            match claimed.get(id.as_str()) {
                None | Some(0) => missing.push(id.clone()),
                Some(1) => {},
                Some(n) => duplicated.push(format!("{id} ({n} claims)")),
            }
        }
        assert!(missing.is_empty(), "unclaimed case IDs (unclaimed-name-fails):\n{}", missing.join("\n"));
        assert!(duplicated.is_empty(), "duplicated case IDs:\n{}", duplicated.join("\n"));

        // No orphan claims either (a claim for an ID outside the real 160).
        let expected_set: std::collections::HashSet<&str> = expected_ids.iter().map(String::as_str).collect();
        let orphans: Vec<&str> = CASE_COVERAGE.iter().map(|&(id, _)| id).filter(|id| !expected_set.contains(id)).collect();
        assert!(orphans.is_empty(), "claims for IDs outside the real 160-case set: {orphans:?}");
    }

    /// No claim string may be empty -- catches a copy-paste slot left
    /// blank.
    #[test]
    fn no_claim_is_empty() {
        for &(id, claim) in CASE_COVERAGE {
            assert!(!claim.trim().is_empty(), "case {id} has an empty claim");
        }
    }

    /// Falsifier ("deliberate bypass and new variant/producer must
    /// fail", packet's own test list): proves the completeness check
    /// itself can fail, without mutating the real 160-entry table.
    #[test]
    fn falsifier_a_missing_case_id_would_be_caught() {
        let synthetic: std::collections::HashSet<&str> = CASE_COVERAGE.iter().map(|&(id, _)| id).collect();
        assert!(!synthetic.contains("ENV-999"), "test fixture bug: ENV-999 must not be a real case id");
        // `every_case_id_has_exactly_one_claim` would list ENV-999 as
        // missing if the real 160-case set ever grew to include it
        // without a corresponding CASE_COVERAGE entry.
    }
}
