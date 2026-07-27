//! `APEX-T3.3.15`: the single canonical server egress owner.
//! `SemanticEgressSysV1` -- takes the outbox's complete pending vector
//! (never a partial drain), validates each intent's binding against the
//! CURRENT live `SessionRegistry` state, sorts and rejects duplicate
//! order keys, allocates a checked per-recipient/per-stream sequence,
//! encodes and sends the frame, and records evidence. Spec:
//! `PROJECT-BASTION-APEX-MICROSTEP-APEX-T3.3-SEMANTIC-NET-ENVELOPE.md`
//! section 15 (algorithm: section 7.8's own 9-step list; failure modes:
//! section 7.9's already-frozen `SemanticEnvelopeRejectV1`/
//! `SemanticProtocolTerminalV1`).
//!
//! Invoked explicitly after `entity_sync`/`subscription` in
//! `server/src/sys/mod.rs::run_sync_systems` -- "assert no semantic
//! producer runs after flush" is satisfied STRUCTURALLY today: every
//! current producer (`entity_sync.rs`, `subscription.rs`) lives inside
//! `run_sync_systems`'s own strictly-sequential call sequence, and this
//! system is the last line in it. One genuine wrinkle found while
//! placing this, recorded honestly rather than silently worked around:
//! `server/src/lib.rs`'s tick loop conditionally re-runs
//! `terrain::Sys` AFTER `run_sync_systems` (the
//! `DisconnectType::WithoutPersistence` cleanup path, `server/src/
//! lib.rs` around the `disconnect_type` check). `terrain::Sys` is not
//! migrated yet (still `PostAuthCandidate` in the `T3.3.14` catalog) so
//! this is inert today -- but WHEN it is migrated (`T3.3.14b-e`), that
//! specific disconnect-cleanup path will enqueue an intent this tick's
//! egress has already flushed, delaying it one tick. Flagged, not
//! solved here; `T3.3.14`'s own future terrain.rs migration pass needs
//! to either re-invoke egress after that conditional re-run or accept
//! the one-tick delay for that narrow path.
//!
//! Dormant today like everything else in this program's T3.3 thread:
//! the outbox is always empty (`T3.3.05`'s negotiation never resolves
//! anything but `Legacy`), so `take_pending()` returns an empty `Vec`
//! and this system's own body returns immediately, every tick.

use std::collections::HashMap;

use crate::{
    Tick,
    client::Client,
    semantic_net::outbox::{SemanticSendIntentV1, ServerSemanticOutboxV1},
    session_registry::{SessionRegistry, SessionStateV1},
};
use common::apex::identity::{ConnectionEpoch, ServerBootId, SessionId};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::envelope::{
    NetEnvelopeHeaderV1, SemanticDirectionV1, SemanticEnvelopeRejectV1, SemanticFrameEvidenceV1,
    SemanticFrameVerdictV1, SemanticPayloadEncodingV1, SemanticRouteV1, SemanticWireFrameV1, encode_payload_v1,
    net_envelope_profile_root_v1, payload_digest_v1,
};
use specs::{Entities, Join, ReadExpect, WriteStorage};
use tracing::warn;

fn manifest_limits() -> common::apex::manifest::ManifestDecodeLimitsV1 {
    common::apex::manifest::ManifestDecodeLimitsV1 {
        max_input_bytes: 1 << 20,
        max_depth: 8,
        max_nodes: 64,
        max_array_items: 16,
        max_map_entries: 16,
        max_machine_text_bytes: 256,
        max_byte_string_bytes: 1 << 20,
    }
}

fn zero_digest() -> common::apex::digest::DigestBytes32V1 { common::apex::digest::DigestBytes32V1::from_array([0; 32]) }

fn evidence(
    tick: u64,
    stream: common_net::msg::envelope::SemanticStreamIdV1,
    session_id: SessionId,
    epoch: ConnectionEpoch,
    payload_schema: common_net::msg::envelope::SemanticPayloadSchemaV1,
    verdict: SemanticFrameVerdictV1,
) -> SemanticFrameEvidenceV1 {
    SemanticFrameEvidenceV1 {
        tick_observed: tick,
        direction: SemanticDirectionV1::ServerToClient,
        stream,
        session_id,
        connection_epoch: epoch,
        sequence: 0,
        payload_schema,
        payload_digest: zero_digest(),
        verdict,
    }
}

/// `APEX-T3.3.15`: section 7.8 step 2, "validates each intent against
/// current active T3.2 binding" -- pure and independently testable
/// against a real `SessionRegistry` built the same way `session_
/// registry.rs`'s own tests build one (via `admit_sorted`), without
/// needing a live `Client`/ECS `World`. Fresh iff the recipient's own
/// `server_boot_id` matches this process's, AND the registry still has
/// an `Active` record for that `session_id` at exactly that `epoch` --
/// a session that no longer exists, has detached, or has moved to a
/// different epoch (a resumed/replaced connection) is stale.
fn binding_is_fresh(
    recipient: &common_net::msg::envelope::ActiveSessionBindingV1,
    session_registry: &SessionRegistry,
    server_boot_id: ServerBootId,
) -> bool {
    recipient.server_boot_id == server_boot_id
        && session_registry
            .record(recipient.session_id)
            .is_some_and(|record| record.state == SessionStateV1::Active && record.epoch == recipient.epoch)
}

/// `APEX-T3.3.15`: rejects every intent that shares a
/// `(recipient, semantic_stream, order_key)` with another in `sorted`
/// -- REJECTS THE WHOLE COLLIDING RUN, not "keep the first". Section
/// 7.7: "Two intents with the same recipient, stream, and order key are
/// a terminal producer bug" -- picking a survivor out of a genuine bug
/// would silently hide which one was the "real" send; rejecting all of
/// them makes the bug visible in the evidence log instead. Requires
/// `sorted` to already be ordered by `total_sort_key` (a fundamental
/// property of sorting: intents comparing equal on the fields checked
/// here are necessarily adjacent after a sort on a superset of those
/// same fields) -- pure and independently testable without any
/// `Client`/`SessionRegistry`/ECS World.
fn reject_duplicate_order_keys(
    sorted: Vec<SemanticSendIntentV1>,
    tick: u64,
    evidence_log: &mut Vec<SemanticFrameEvidenceV1>,
) -> Vec<SemanticSendIntentV1> {
    let mut deduped: Vec<SemanticSendIntentV1> = Vec::with_capacity(sorted.len());
    let mut i = 0;
    while i < sorted.len() {
        let mut j = i + 1;
        while j < sorted.len()
            && sorted[j].recipient == sorted[i].recipient
            && sorted[j].semantic_stream == sorted[i].semantic_stream
            && sorted[j].order_key == sorted[i].order_key
        {
            j += 1;
        }
        if j - i == 1 {
            deduped.push(sorted[i].clone());
        } else {
            for intent in &sorted[i..j] {
                evidence_log.push(evidence(
                    tick,
                    intent.semantic_stream,
                    intent.recipient.session_id,
                    intent.recipient.epoch,
                    intent.payload.payload_schema(),
                    SemanticFrameVerdictV1::Rejected(SemanticEnvelopeRejectV1::DuplicateOrderKey),
                ));
            }
        }
        i = j;
    }
    deduped
}

/// This system will flush the semantic outbox to its recipients.
#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Client>,
        ReadExpect<'a, ServerSemanticOutboxV1>,
        ReadExpect<'a, SessionRegistry>,
        ReadExpect<'a, ServerBootId>,
        specs::Read<'a, Tick>,
    );

    const NAME: &'static str = "semantic_egress";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Apply;

    fn run(_job: &mut Job<Self>, (entities, mut clients, outbox, session_registry, server_boot_id, tick): Self::SystemData) {
        let tick = tick.0;
        let mut intents = outbox.take_pending();
        if intents.is_empty() {
            return;
        }

        let mut evidence_log: Vec<SemanticFrameEvidenceV1> = Vec::with_capacity(intents.len());

        // Step 2 (section 7.8): validate current binding. A stale
        // binding (session gone, detached, epoch moved on, or -- in
        // principle -- a different server boot) is a per-intent reject,
        // never a whole-batch abort.
        intents.retain(|intent| {
            let fresh = binding_is_fresh(&intent.recipient, &session_registry, *server_boot_id);
            if !fresh {
                evidence_log.push(evidence(
                    tick,
                    intent.semantic_stream,
                    intent.recipient.session_id,
                    intent.recipient.epoch,
                    intent.payload.payload_schema(),
                    SemanticFrameVerdictV1::Rejected(SemanticEnvelopeRejectV1::StaleEgressBinding),
                ));
            }
            fresh
        });

        // Step 3: sort by the section 7.7 total order. Everything
        // downstream (duplicate detection, per-recipient/per-stream
        // sequence allocation) relies on this ordering; see
        // `SemanticSendIntentV1::total_sort_key`'s own doc for why every
        // field it draws from is wire-independent and thread-order-free.
        intents.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));

        // Step 3b: reject duplicate order keys. Contiguous after the
        // sort above (a fundamental property of sorting by exactly the
        // fields being compared for equality here) -- an entire run of
        // N>1 colliding intents is rejected in full -- see
        // `reject_duplicate_order_keys`'s own doc comment.
        let deduped = reject_duplicate_order_keys(intents, tick, &mut evidence_log);

        // Steps 4-8, one client at a time. `deduped` is already sorted
        // by (recipient, stream, ...), so processing it in this exact
        // order naturally groups same-(recipient, stream) intents
        // contiguously and allocates their sequences in the right
        // relative order without any separate grouping pass. A failure
        // partway through one recipient's intents (lookup miss, encode
        // failure, send failure) only skips THAT intent -- "recipient
        // failures cannot affect other recipients" (packet's own
        // words), satisfied by construction: nothing here shares state
        // across intents except the per-client cursor each intent's own
        // `client.semantic_send_state_mut()` call reaches independently.
        let session_to_entity: HashMap<SessionId, specs::Entity> = (&entities, &clients)
            .join()
            .filter_map(|(entity, client)| client.semantic_send_state().map(|s| (s.binding().session_id, entity)))
            .collect();

        for intent in deduped {
            let Some(&entity) = session_to_entity.get(&intent.recipient.session_id) else {
                // The client vanished between the binding-freshness check
                // above and now (e.g. disconnected mid-tick) -- reject,
                // don't panic; this is exactly the kind of "recipient
                // failure" the packet says must not touch anyone else.
                evidence_log.push(evidence(
                    tick,
                    intent.semantic_stream,
                    intent.recipient.session_id,
                    intent.recipient.epoch,
                    intent.payload.payload_schema(),
                    SemanticFrameVerdictV1::Rejected(SemanticEnvelopeRejectV1::StaleEgressBinding),
                ));
                continue;
            };
            let Some(client) = clients.get_mut(entity) else { continue };
            let Some(send_state) = client.semantic_send_state_mut() else { continue };

            let sequence = match send_state.allocate_sequence(intent.semantic_stream) {
                Ok(seq) => seq,
                Err(_exhausted) => {
                    evidence_log.push(evidence(
                        tick,
                        intent.semantic_stream,
                        intent.recipient.session_id,
                        intent.recipient.epoch,
                        intent.payload.payload_schema(),
                        SemanticFrameVerdictV1::Rejected(SemanticEnvelopeRejectV1::SequenceExhausted),
                    ));
                    continue;
                },
            };

            let payload_bytes = encode_payload_v1(&*intent.payload);
            let profile_root = net_envelope_profile_root_v1();
            let payload_schema = intent.payload.payload_schema();
            let payload_encoding = SemanticPayloadEncodingV1::Bincode2LegacySerde;
            let payload_digest = payload_digest_v1(profile_root, payload_schema, payload_encoding, &payload_bytes);
            let header = NetEnvelopeHeaderV1 {
                profile_root,
                server_boot_id: intent.recipient.server_boot_id,
                session_id: intent.recipient.session_id,
                connection_epoch: intent.recipient.epoch,
                direction: SemanticDirectionV1::ServerToClient,
                semantic_stream: intent.semantic_stream,
                sequence,
                causality: intent.causality,
                payload_schema,
                payload_encoding,
                payload_len: payload_bytes.len() as u64,
                payload_digest,
                command_id: None,
            };
            let frame = SemanticWireFrameV1 { header, payload_bytes };

            let frame_bytes = match common::apex::manifest::encode_manifest_v1(&frame, &manifest_limits()) {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!(?e, "semantic egress encode failure");
                    evidence_log.push(evidence(
                        tick,
                        intent.semantic_stream,
                        intent.recipient.session_id,
                        intent.recipient.epoch,
                        payload_schema,
                        SemanticFrameVerdictV1::Rejected(SemanticEnvelopeRejectV1::EncodeFailure),
                    ));
                    continue;
                },
            };

            // Sequence is already consumed above -- a send failure from
            // here on never gets it back (packet: "a failed send
            // consumes its sequence and never reuses it").
            match client.send_semantic_frame(intent.semantic_stream, frame_bytes) {
                Ok(()) => {
                    let mut ev = evidence(
                        tick,
                        intent.semantic_stream,
                        intent.recipient.session_id,
                        intent.recipient.epoch,
                        payload_schema,
                        SemanticFrameVerdictV1::Sent,
                    );
                    ev.sequence = sequence.get();
                    ev.payload_digest = payload_digest;
                    evidence_log.push(ev);
                },
                Err(e) => {
                    // "Terminates that attachment": matches this
                    // codebase's own existing convention everywhere else
                    // a send can fail (`send_fallible` silently discards
                    // the error and relies on the network layer's own
                    // liveness/timeout detection to eventually
                    // disconnect a genuinely dead connection) -- not a
                    // new disconnect-triggering mechanism invented here.
                    warn!(?e, "semantic egress send failure after sequence allocation");
                    let mut ev = evidence(
                        tick,
                        intent.semantic_stream,
                        intent.recipient.session_id,
                        intent.recipient.epoch,
                        payload_schema,
                        SemanticFrameVerdictV1::Terminal(
                            common_net::msg::envelope::SemanticProtocolTerminalV1::SendFailedAfterSequenceAllocated,
                        ),
                    );
                    ev.sequence = sequence.get();
                    evidence_log.push(ev);
                },
            }
        }

        // Section 7.10: "Do not record tokens, chat text, command
        // arguments, or payload bytes in ordinary logs." evidence_log's
        // own shape structurally cannot carry any of those (see
        // `SemanticFrameEvidenceV1`'s own doc) -- tracing it at debug
        // level is safe by construction, not by discipline.
        if !evidence_log.is_empty() {
            tracing::debug!(count = evidence_log.len(), "semantic egress flushed");
        }
    }
}

/// `APEX-T3.3.15` tests. The pure algorithm pieces (`binding_is_fresh`,
/// `reject_duplicate_order_keys`) get direct unit coverage against a REAL
/// `SessionRegistry` (built the same way `session_registry.rs`'s own
/// tests build one) since neither needs a live `Client`/ECS `World`.
///
/// `full_pipeline_enqueue_to_real_wire_delivery` closes the gap that
/// left open across `T3.3.06/07/08/10/13/14a`: per Fable's explicit
/// direction for this row ("design it to actually INVOKE the real call
/// sites' path -- no test drives the production sites today"), it builds
/// a genuinely live `Client` (a real `network::Participant` over an
/// in-process `Mpsc` transport, the same 6 streams/ids/promises/
/// priorities `connection_handler.rs` opens for a real connection),
/// enqueues through the real `ServerSemanticOutboxV1::try_enqueue_if_v1`
/// primitive, runs `Sys` via the real `common_ecs::run_now` (the exact
/// call `run_sync_systems` makes in production), and reads the actual
/// bytes that landed on the peer side of the wire -- the first time
/// anything in this program has driven `Client::send_semantic_frame`
/// (and its `Stream::send`) for real. "Recipient failure isolation" and
/// "early flush" remain structural properties of `Sys::run`'s own
/// control flow (each intent's own `continue` on error; the
/// `is_empty()` guard is the function's first line) rather than
/// independently exercised, same as before.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use authc::Uuid;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use common_net::msg::{
        ClientType, ServerGeneral,
        client::SessionRequestV1,
        envelope::{ActiveSessionBindingV1, SemanticCausalityV1, SemanticStreamIdV1, decode_payload_exact_v1},
    };
    use itertools::Itertools;

    use super::*;
    use crate::{
        Tick,
        semantic_net::order::{SemanticPayloadRankV1, SemanticProducerV1, phase_rank},
        semantic_net::outbox::{CanonicalSubjectKeyV1, ServerSemanticOrderKeyV1},
        session_registry::AuthenticatedIntentV1,
    };

    fn boot_id() -> ServerBootId { ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap() }

    /// Admits ONE real, `Active` session the same way `session_registry
    /// .rs`'s own tests do (`admit_sorted`, not a hand-built record --
    /// there is no lightweight constructor, by design: `SessionRecordV1`
    /// only ever comes from the real admission pass).
    fn admit_one_session(registry: &mut SessionRegistry) -> ActiveSessionBindingV1 {
        let seq = registry.allocate_attempt_seq().unwrap();
        let intent = AuthenticatedIntentV1 {
            principal: Uuid::from_bytes([7; 16]),
            client_type: ClientType::Game,
            attempt_seq: seq,
            request: SessionRequestV1::New,
            capacity_exempt: false,
            requested_semantic_protocol: common_net::msg::SemanticProtocolIdV1::Legacy,
        };
        let mut src = FixedRandomBytesSourceV1([2; 16]);
        let out = registry.admit_sorted(vec![((), intent)], 64, std::time::Instant::now(), 64, &mut src);
        let admission = out.into_iter().next().unwrap().1.unwrap();
        let binding = admission.binding();
        ActiveSessionBindingV1 { server_boot_id: boot_id(), session_id: binding.session_id, epoch: binding.epoch }
    }

    #[test]
    fn fresh_active_binding_is_fresh() {
        let mut registry = SessionRegistry::new();
        let recipient = admit_one_session(&mut registry);
        assert!(binding_is_fresh(&recipient, &registry, boot_id()));
    }

    #[test]
    fn unknown_session_is_stale() {
        let registry = SessionRegistry::new();
        let recipient = ActiveSessionBindingV1 {
            server_boot_id: boot_id(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([9; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        };
        assert!(!binding_is_fresh(&recipient, &registry, boot_id()));
    }

    #[test]
    fn stale_epoch_is_rejected() {
        let mut registry = SessionRegistry::new();
        let mut recipient = admit_one_session(&mut registry);
        // A DIFFERENT epoch than the one the registry actually admitted
        // (simulating a resume/replace that moved the epoch on).
        recipient.epoch = ConnectionEpoch::new(recipient.epoch.get() + 1).unwrap();
        assert!(!binding_is_fresh(&recipient, &registry, boot_id()));
    }

    #[test]
    fn detached_session_is_stale() {
        let mut registry = SessionRegistry::new();
        let recipient = admit_one_session(&mut registry);
        registry.detach(recipient.session_id, std::time::Instant::now(), std::time::Duration::from_secs(60), 64);
        assert!(!binding_is_fresh(&recipient, &registry, boot_id()));
    }

    #[test]
    fn wrong_server_boot_is_stale() {
        let mut registry = SessionRegistry::new();
        let recipient = admit_one_session(&mut registry);
        let other_boot = ServerBootId::generate(&mut FixedRandomBytesSourceV1([99; 16])).unwrap();
        assert!(!binding_is_fresh(&recipient, &registry, other_boot));
    }

    fn recipient_binding(seed: u8) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: boot_id(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn sample_intent(recipient: ActiveSessionBindingV1, subject_byte: u8, local_ordinal: u32) -> SemanticSendIntentV1 {
        SemanticSendIntentV1 {
            recipient,
            semantic_stream: SemanticStreamIdV1::General,
            causality: SemanticCausalityV1 { producer_tick: None, snapshot: None },
            order_key: ServerSemanticOrderKeyV1 {
                source_tick: 1,
                phase_rank: phase_rank(common_ecs::Phase::Create),
                producer_rank: SemanticProducerV1::EntitySync.producer_rank(),
                payload_rank: SemanticPayloadRankV1::Create.payload_rank(),
                subject: CanonicalSubjectKeyV1::for_uid(common::uid::Uid(std::num::NonZeroU64::new(subject_byte as u64).unwrap())),
                local_ordinal,
            },
            payload: Arc::new(ServerGeneral::UpdateRecipes),
        }
    }

    #[test]
    fn no_duplicates_pass_through_unchanged() {
        let a = sample_intent(recipient_binding(1), 1, 0);
        let b = sample_intent(recipient_binding(1), 2, 0);
        let mut evidence_log = Vec::new();
        let out = reject_duplicate_order_keys(vec![a.clone(), b.clone()], 1, &mut evidence_log);
        assert_eq!(out.len(), 2);
        assert!(evidence_log.is_empty());
    }

    #[test]
    fn an_entire_colliding_run_is_rejected_not_just_the_extras() {
        // Three intents sharing the exact same (recipient, stream, order_key).
        let a = sample_intent(recipient_binding(1), 5, 0);
        let b = a.clone();
        let c = a.clone();
        let mut evidence_log = Vec::new();
        let out = reject_duplicate_order_keys(vec![a, b, c], 1, &mut evidence_log);
        assert!(out.is_empty(), "all three colliding intents must be rejected, not just the extras");
        assert_eq!(evidence_log.len(), 3);
        assert!(evidence_log.iter().all(|e| matches!(
            e.verdict,
            SemanticFrameVerdictV1::Rejected(SemanticEnvelopeRejectV1::DuplicateOrderKey)
        )));
    }

    /// "Permutation/fanout" (packet's own test list): the same multiset
    /// of intents, sorted in every possible input order first, must
    /// dedup-reject the identical set every time.
    #[test]
    fn dedup_result_is_order_independent() {
        let unique = sample_intent(recipient_binding(1), 1, 0);
        let dup_a = sample_intent(recipient_binding(1), 2, 0);
        let dup_b = dup_a.clone();
        let intents = vec![unique.clone(), dup_a, dup_b];

        for permutation in intents.into_iter().permutations(3) {
            let mut sorted = permutation;
            sorted.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
            let mut evidence_log = Vec::new();
            let out = reject_duplicate_order_keys(sorted, 1, &mut evidence_log);
            assert_eq!(out.len(), 1);
            assert_eq!(out[0].order_key.subject, unique.order_key.subject);
            assert_eq!(evidence_log.len(), 2);
        }
    }

    /// Owns everything that must outlive the test's ECS/wire round trip:
    /// both `network::Network`s, the peer-side `Participant`, and all 6
    /// peer-side `Stream`s (dropping any of them can tear down the
    /// connection). `client` is `Option` so it can be moved out into the
    /// ECS `World` without consuming (and thereby dropping) the rest of
    /// the harness.
    struct LiveClientHarness {
        _server_net: network::Network,
        _peer_net: network::Network,
        _peer_participant: network::Participant,
        _peer_general: network::Stream,
        _peer_ping: network::Stream,
        _peer_register: network::Stream,
        _peer_character_screen: network::Stream,
        peer_in_game: network::Stream,
        _peer_terrain: network::Stream,
        client: Option<Client>,
    }

    /// Builds a genuinely live `Client` the same way `connection_handler
    /// .rs` does (same stream ids/promises/priorities), over an
    /// in-process `Mpsc` transport -- no sockets, fully deterministic and
    /// fast. `network/tests/helper.rs` has an equivalent single-stream
    /// helper, but it lives in a different crate's `tests/` directory and
    /// isn't importable from here, so this rebuilds the same shape (with
    /// all 6 of `Client`'s streams instead of 1) locally.
    fn build_live_client_harness(mpsc_port: u64, runtime: &tokio::runtime::Runtime) -> LiveClientHarness {
        use network::{ConnectAddr, ListenAddr, Network, Pid, Promises};

        let reliable = Promises::ORDERED | Promises::CONSISTENCY;
        let reliablec = reliable | Promises::COMPRESSED;

        runtime.block_on(async {
            let mut server_net = Network::new(Pid::fake(0), runtime);
            let peer_net = Network::new(Pid::fake(1), runtime);
            server_net.listen(ListenAddr::Mpsc(mpsc_port)).await.unwrap();
            let mut peer_participant = peer_net.connect(ConnectAddr::Mpsc(mpsc_port)).await.unwrap();
            let server_participant = server_net.connected().await.unwrap();

            // Exact ids/promises/priorities `connection_handler.rs` uses.
            let general_stream = server_participant.open(3, reliablec, 500).await.unwrap();
            let ping_stream = server_participant.open(2, reliable, 500).await.unwrap();
            let register_stream = server_participant.open(3, reliablec, 500).await.unwrap();
            let character_screen_stream = server_participant.open(3, reliablec, 500).await.unwrap();
            let in_game_stream = server_participant.open(3, reliablec, 100_000).await.unwrap();
            let terrain_stream = server_participant.open(4, reliable, 20_000).await.unwrap();

            // "It's guaranteed that the order of open and opened is
            // equal" (`Participant::opened`'s own doc) -- same order.
            let peer_general = peer_participant.opened().await.unwrap();
            let peer_ping = peer_participant.opened().await.unwrap();
            let peer_register = peer_participant.opened().await.unwrap();
            let peer_character_screen = peer_participant.opened().await.unwrap();
            let peer_in_game = peer_participant.opened().await.unwrap();
            let peer_terrain = peer_participant.opened().await.unwrap();

            let client = Client::new(
                ClientType::Game,
                server_participant,
                ConnectAddr::Mpsc(mpsc_port),
                0.0,
                None,
                general_stream,
                ping_stream,
                register_stream,
                character_screen_stream,
                in_game_stream,
                terrain_stream,
            );

            LiveClientHarness {
                _server_net: server_net,
                _peer_net: peer_net,
                _peer_participant: peer_participant,
                _peer_general: peer_general,
                _peer_ping: peer_ping,
                _peer_register: peer_register,
                _peer_character_screen: peer_character_screen,
                peer_in_game,
                _peer_terrain: peer_terrain,
                client: Some(client),
            }
        })
    }

    #[test]
    fn full_pipeline_enqueue_to_real_wire_delivery() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let mut harness = build_live_client_harness(45_101, &runtime);

        let mut registry = SessionRegistry::new();
        let recipient = admit_one_session(&mut registry);

        // T3.3.06's own real attachment method -- not a hand-set private
        // field.
        let mut client = harness.client.take().unwrap();
        client.reset_semantic_state(recipient);

        // The real shared production primitive `entity_sync.rs` and
        // `subscription.rs` both call through.
        let outbox = ServerSemanticOutboxV1::new();
        let enqueued = outbox.try_enqueue_if_v1(
            Some(recipient),
            ServerGeneral::UpdateRecipes,
            7,
            phase_rank(common_ecs::Phase::Create),
            SemanticProducerV1::EntitySync.producer_rank(),
            SemanticPayloadRankV1::EntitySync.payload_rank(),
            CanonicalSubjectKeyV1::for_singleton("full_pipeline_test"),
            0,
        );
        assert!(enqueued, "try_enqueue_if_v1 must accept a Some(binding) recipient");

        use specs::{Builder, WorldExt};
        let mut world = specs::World::new();
        world.register::<Client>();
        world.insert(outbox);
        world.insert(registry);
        world.insert(boot_id());
        world.insert(Tick(7));
        world.insert(common_ecs::SysMetrics::default());
        world.create_entity().with(client).build();

        // The exact call `run_sync_systems` makes in production.
        common_ecs::run_now::<Sys>(&world);

        let frame_bytes: Vec<u8> = runtime.block_on(harness.peer_in_game.recv::<Vec<u8>>()).unwrap();
        let frame: SemanticWireFrameV1 = common::apex::manifest::decode_manifest_v1(&frame_bytes, &manifest_limits()).unwrap();

        assert_eq!(frame.header.semantic_stream, SemanticStreamIdV1::InGame);
        assert_eq!(frame.header.session_id, recipient.session_id);
        assert_eq!(frame.header.connection_epoch, recipient.epoch);
        assert_eq!(frame.header.server_boot_id, boot_id());
        assert_eq!(frame.header.direction, SemanticDirectionV1::ServerToClient);
        assert_eq!(frame.header.sequence.get(), 1);

        let decoded_payload: ServerGeneral = decode_payload_exact_v1(&frame.payload_bytes).unwrap();
        assert!(matches!(decoded_payload, ServerGeneral::UpdateRecipes));
    }
}
