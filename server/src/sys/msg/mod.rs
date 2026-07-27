pub mod character_screen;
pub mod general;
pub mod gizmos;
pub mod in_game;
pub mod network_events;
pub mod ping;
pub mod register;
pub mod terrain;

use crate::{
    client::Client,
    sys::{loot, pets},
};
use common_ecs::{System, dispatch};
use common_net::msg::{
    ClientGeneral,
    envelope::{
        NetEnvelopeHeaderV1, SemanticDirectionV1, SemanticEnvelopeRejectV1, SemanticReceiveStateV1, SemanticRouteV1,
        SemanticStreamIdV1, SemanticWireFrameV1, decode_payload_exact_v1, net_envelope_profile_root_v1, payload_digest_v1,
    },
};
use serde::de::DeserializeOwned;
use specs::DispatcherBuilder;
use tracing::warn;

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    //run ping after general, as its super fast anyway. also don't get duplicate
    // disconnect then.
    dispatch::<gizmos::Sys>(dispatch_builder, &[]);
    dispatch::<character_screen::Sys>(dispatch_builder, &[]);
    dispatch::<general::Sys>(dispatch_builder, &[]);
    dispatch::<in_game::Sys>(dispatch_builder, &[]);
    dispatch::<ping::Sys>(dispatch_builder, &[&general::Sys::sys_name()]);
    dispatch::<register::Sys>(dispatch_builder, &[]);
    dispatch::<terrain::Sys>(dispatch_builder, &[]);
    dispatch::<pets::Sys>(dispatch_builder, &[]);
    dispatch::<loot::Sys>(dispatch_builder, &[]);
    dispatch::<network_events::Sys>(dispatch_builder, &[]);
}

/// handles all send msg and calls a handle fn
/// Aborts when a error occurred returns cnt of successful msg otherwise
pub(crate) fn try_recv_all<M, F>(
    client: &mut Client,
    stream_id: u8,
    mut f: F,
) -> Result<u64, crate::error::Error>
where
    M: DeserializeOwned,
    F: FnMut(&Client, M) -> Result<(), crate::error::Error>,
{
    let mut cnt = 0u64;
    loop {
        let msg = match client.recv(stream_id) {
            Ok(Some(msg)) => msg,
            Ok(None) => break Ok(cnt),
            Err(e) => break Err(e.into()),
        };
        if let Err(e) = f(client, msg) {
            break Err(e);
        }
        cnt += 1;
    }
}

fn semantic_manifest_limits() -> common::apex::manifest::ManifestDecodeLimitsV1 {
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

/// `APEX-T3.3.08`: the full envelope/payload validation pipeline for one
/// received frame's raw bytes, WITHOUT committing any cursor state or
/// calling any handler -- a pure function, directly unit-testable
/// (`try_recv_all_semantic` below is the thin stateful wrapper around
/// it). Packet's own list, in order: exact-decode frame, validate
/// profile/binding/direction/physical route/sequence, verify payload,
/// exact-decode `ClientGeneral`, verify shared route.
pub(crate) fn validate_semantic_frame_v1(
    raw: &[u8],
    receive_state: &SemanticReceiveStateV1,
    expected_physical_stream: SemanticStreamIdV1,
) -> Result<ClientGeneral, SemanticEnvelopeRejectV1> {
    let frame: SemanticWireFrameV1 = common::apex::manifest::decode_manifest_v1(raw, &semantic_manifest_limits())
        .map_err(|_| SemanticEnvelopeRejectV1::EnvelopeDecodeFailure)?;
    let header: &NetEnvelopeHeaderV1 = &frame.header;

    if header.profile_root != net_envelope_profile_root_v1() {
        return Err(SemanticEnvelopeRejectV1::UnsupportedProfile);
    }
    let binding = receive_state.binding();
    if header.server_boot_id != binding.server_boot_id {
        return Err(SemanticEnvelopeRejectV1::WrongBoot);
    }
    if header.session_id != binding.session_id {
        return Err(SemanticEnvelopeRejectV1::WrongSession);
    }
    if header.connection_epoch != binding.epoch {
        return Err(if header.connection_epoch.get() < binding.epoch.get() {
            SemanticEnvelopeRejectV1::StaleEpoch
        } else {
            SemanticEnvelopeRejectV1::FutureEpoch
        });
    }
    if header.direction != SemanticDirectionV1::ClientToServer {
        return Err(SemanticEnvelopeRejectV1::WrongDirection);
    }
    // "Physical route": the frame's own declared semantic_stream must
    // match the physical stream it actually arrived on -- a General
    // frame cannot claim to be Terrain traffic just because it says so.
    if header.semantic_stream != expected_physical_stream {
        return Err(SemanticEnvelopeRejectV1::StreamRouteMismatch);
    }
    // Zero-gap MVP (packet section 5.4): equal accepts, less is a
    // duplicate/replay, greater is a gap. This tree does not further
    // distinguish "immediate duplicate" from "older replay" -- both
    // collapse to DuplicateSequence, a deliberate simplification the
    // packet's own two-way split (received<expected -> reject) does not
    // require going beyond.
    let expected_seq = receive_state.next_expected_for(header.semantic_stream);
    if header.sequence < expected_seq {
        return Err(SemanticEnvelopeRejectV1::DuplicateSequence);
    }
    if header.sequence > expected_seq {
        return Err(SemanticEnvelopeRejectV1::SequenceGap { expected: expected_seq.get(), received: header.sequence.get() });
    }
    if header.payload_len != frame.payload_bytes.len() as u64 {
        return Err(SemanticEnvelopeRejectV1::PayloadLengthMismatch);
    }
    let recomputed_digest =
        payload_digest_v1(header.profile_root, header.payload_schema, header.payload_encoding, &frame.payload_bytes);
    if recomputed_digest.as_array() != header.payload_digest.as_array() {
        return Err(SemanticEnvelopeRejectV1::PayloadDigestMismatch);
    }
    // Dormant until T3.5 (packet section 5.6): `Some` is unconditionally
    // rejected today, never partially trusted.
    if header.command_id.is_some() {
        return Err(SemanticEnvelopeRejectV1::CommandIdUnsupported);
    }

    let decoded: ClientGeneral = decode_payload_exact_v1(&frame.payload_bytes)?;
    // "Verify shared route": the DECODED payload's own SemanticRouteV1
    // classification (T3.3.04) must agree with what the header claimed
    // -- a header cannot say "General/ClientGeneral" while its payload
    // bytes actually decode to a TerrainChunkRequest.
    if decoded.semantic_stream() != header.semantic_stream || decoded.payload_schema() != header.payload_schema {
        return Err(SemanticEnvelopeRejectV1::StreamRouteMismatch);
    }

    Ok(decoded)
}

/// `APEX-T3.3.08`: "one reusable pre-mutation gate" -- validates and
/// decodes every frame received on `stream_id` before the handler ever
/// sees it, committing the receive cursor only after every check passes
/// and strictly before the handler is called (packet: "cursor does not
/// advance on validation failure; it advances before handler call;
/// handler error terminates and is never replayed"). Reached via
/// `try_recv_all_dispatch` below, wired into the four semantic receive
/// systems (general/character_screen/in_game/terrain) as of `T3.3.09`.
///
/// A rejected frame is dropped and draining continues (packet's own
/// "rejected traffic liveness" test concern, and its acceptance gate
/// "every reject leaves state/events unchanged" -- true here since
/// neither the cursor nor the handler is ever reached for a reject). A
/// `NoActiveAttachment` frame (no live `NetEnvelopeV1` state on this
/// `Client`) is treated the same way, not specially. Only a genuine
/// handler error terminates the drain loop, matching `try_recv_all`'s
/// own existing contract exactly.
pub(crate) fn try_recv_all_semantic<F>(
    client: &mut Client,
    stream_id: u8,
    expected_semantic_stream: SemanticStreamIdV1,
    mut handler: F,
) -> Result<u64, crate::error::Error>
where
    F: FnMut(&Client, ClientGeneral) -> Result<(), crate::error::Error>,
{
    let mut cnt = 0u64;
    loop {
        let raw: Vec<u8> = match client.recv(stream_id) {
            Ok(Some(msg)) => msg,
            Ok(None) => break Ok(cnt),
            Err(e) => break Err(e.into()),
        };
        let Some(receive_state) = client.semantic_receive_state() else {
            warn!("received a semantic V1 frame with no active attachment; dropping");
            continue;
        };
        match validate_semantic_frame_v1(&raw, receive_state, expected_semantic_stream) {
            Ok(decoded) => {
                let advance_result = client
                    .semantic_receive_state_mut()
                    .expect("checked Some above; nothing else can clear this state mid-loop")
                    .advance_expected(expected_semantic_stream);
                if advance_result.is_err() {
                    warn!("semantic receive sequence exhausted; dropping message");
                    continue;
                }
                if let Err(e) = handler(client, decoded) {
                    break Err(e);
                }
                cnt += 1;
            },
            Err(reject) => {
                warn!(?reject, "semantic ingress rejected a frame");
            },
        }
    }
}

/// `T3.3.09`: per-client V1/Legacy receive-helper selector, shared by
/// all four semantic receive systems (general/character_screen/in_game/
/// terrain). `client.semantic_receive_state()` is `Some` exactly when
/// T3.2's handshake negotiated the V1 semantic protocol for this
/// connection (T3.3.05: negotiation always resolves to `Legacy` today,
/// so this always takes the `else` branch live -- both arms are already
/// fully wired and tested, waiting on T3.3.05's own eventual negotiation
/// change to go live). Both arms share the exact same `handler` closure
/// unchanged: the decoded `ClientGeneral` payload follows the existing
/// handler/deferred path either way (packet: "preserve handlers; ensure
/// envelope acceptance occurs first").
pub(crate) fn try_recv_all_dispatch<F>(
    client: &mut Client,
    stream_id: u8,
    semantic_stream: SemanticStreamIdV1,
    handler: F,
) -> Result<u64, crate::error::Error>
where
    F: FnMut(&Client, ClientGeneral) -> Result<(), crate::error::Error>,
{
    if client.semantic_receive_state().is_some() {
        try_recv_all_semantic(client, stream_id, semantic_stream, handler)
    } else {
        try_recv_all(client, stream_id, handler)
    }
}

#[cfg(test)]
mod semantic_ingress_tests {
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use common_net::msg::envelope::{
        ActiveSessionBindingV1, SemanticCausalityV1, SemanticPayloadEncodingV1, encode_payload_v1,
    };
    use std::num::NonZeroU64;

    use super::*;

    fn binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(5).unwrap(),
        }
    }

    fn receive_state() -> SemanticReceiveStateV1 { SemanticReceiveStateV1::new(binding()) }

    /// A minimal, valid `ClientGeneral` for `General`/`ChatMsg` -- picked
    /// for the simplest constructible variant (see T3.3.04's own route
    /// tests for why `ChatMsg`/`Terminate` were the easy picks there too).
    fn sample_msg() -> ClientGeneral { ClientGeneral::Terminate }

    fn valid_frame_bytes(b: ActiveSessionBindingV1, sequence: u64) -> Vec<u8> {
        let msg = sample_msg();
        let payload_bytes = encode_payload_v1(&msg);
        let profile_root = net_envelope_profile_root_v1();
        let payload_schema = msg.payload_schema();
        let payload_encoding = SemanticPayloadEncodingV1::Bincode2LegacySerde;
        let payload_digest = payload_digest_v1(profile_root, payload_schema, payload_encoding, &payload_bytes);
        let header = NetEnvelopeHeaderV1 {
            profile_root,
            server_boot_id: b.server_boot_id,
            session_id: b.session_id,
            connection_epoch: b.epoch,
            direction: SemanticDirectionV1::ClientToServer,
            semantic_stream: msg.semantic_stream(),
            sequence: NonZeroU64::new(sequence).unwrap(),
            causality: SemanticCausalityV1 { producer_tick: None, snapshot: None },
            payload_schema,
            payload_encoding,
            payload_len: payload_bytes.len() as u64,
            payload_digest,
            command_id: None,
        };
        let frame = SemanticWireFrameV1 { header, payload_bytes };
        common::apex::manifest::encode_manifest_v1(&frame, &semantic_manifest_limits()).unwrap()
    }

    #[test]
    fn valid_frame_is_accepted_and_decodes_to_the_original_message() {
        let state = receive_state();
        let raw = valid_frame_bytes(binding(), 1);
        let decoded = validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).unwrap();
        assert!(matches!(decoded, ClientGeneral::Terminate));
    }

    #[test]
    fn wrong_boot_is_rejected() {
        let state = receive_state();
        let mut wrong = binding();
        wrong.server_boot_id = ServerBootId::generate(&mut FixedRandomBytesSourceV1([99; 16])).unwrap();
        let raw = valid_frame_bytes(wrong, 1);
        assert_eq!(
            validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::WrongBoot
        );
    }

    #[test]
    fn wrong_session_is_rejected() {
        let state = receive_state();
        let mut wrong = binding();
        wrong.session_id = SessionId::generate(&mut FixedRandomBytesSourceV1([99; 16])).unwrap();
        let raw = valid_frame_bytes(wrong, 1);
        assert_eq!(
            validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::WrongSession
        );
    }

    #[test]
    fn stale_and_future_epoch_are_both_rejected_and_distinguished() {
        let state = receive_state(); // binding().epoch = 5
        let mut stale = binding();
        stale.epoch = ConnectionEpoch::new(4).unwrap();
        assert_eq!(
            validate_semantic_frame_v1(&valid_frame_bytes(stale, 1), &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::StaleEpoch
        );

        let mut future = binding();
        future.epoch = ConnectionEpoch::new(6).unwrap();
        assert_eq!(
            validate_semantic_frame_v1(&valid_frame_bytes(future, 1), &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::FutureEpoch
        );
    }

    /// "Duplicate": a sequence at or below what's already been consumed.
    #[test]
    fn duplicate_sequence_is_rejected() {
        let mut state = receive_state();
        state.advance_expected(SemanticStreamIdV1::General).unwrap(); // next_expected is now 2
        let raw = valid_frame_bytes(binding(), 1); // stale: the already-consumed value
        assert_eq!(
            validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::DuplicateSequence
        );
    }

    /// "Gap": a sequence ahead of what's expected, with the exact
    /// expected/received pair carried in the terminal.
    #[test]
    fn sequence_gap_is_rejected_with_exact_values() {
        let state = receive_state();
        let raw = valid_frame_bytes(binding(), 5); // expected 1, received 5
        assert_eq!(
            validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::SequenceGap { expected: 1, received: 5 }
        );
    }

    /// "Digest": the payload bytes were tampered with after the digest
    /// was computed -- decoded via direct manifest reconstruction (not
    /// available through the normal constructors, which always compute a
    /// correct digest) to prove the DECODER independently recomputes and
    /// checks it, rather than trusting the header's own claim.
    #[test]
    fn payload_digest_mismatch_is_rejected() {
        let state = receive_state();
        let b = binding();
        let msg = sample_msg();
        let payload_bytes = encode_payload_v1(&msg);
        let profile_root = net_envelope_profile_root_v1();
        let payload_schema = msg.payload_schema();
        let payload_encoding = SemanticPayloadEncodingV1::Bincode2LegacySerde;
        // Digest of the WRONG bytes -- a tampered payload with a stale digest.
        let wrong_digest = payload_digest_v1(profile_root, payload_schema, payload_encoding, b"not the real payload");
        let header = NetEnvelopeHeaderV1 {
            profile_root,
            server_boot_id: b.server_boot_id,
            session_id: b.session_id,
            connection_epoch: b.epoch,
            direction: SemanticDirectionV1::ClientToServer,
            semantic_stream: msg.semantic_stream(),
            sequence: NonZeroU64::new(1).unwrap(),
            causality: SemanticCausalityV1 { producer_tick: None, snapshot: None },
            payload_schema,
            payload_encoding,
            payload_len: payload_bytes.len() as u64,
            payload_digest: wrong_digest,
            command_id: None,
        };
        let frame = SemanticWireFrameV1 { header, payload_bytes };
        let raw = common::apex::manifest::encode_manifest_v1(&frame, &semantic_manifest_limits()).unwrap();
        assert_eq!(
            validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::PayloadDigestMismatch
        );
    }

    /// "Trailing": extra bytes appended after a complete, otherwise-valid
    /// envelope frame must fail decode, not silently truncate.
    #[test]
    fn trailing_bytes_after_the_envelope_are_rejected() {
        let state = receive_state();
        let mut raw = valid_frame_bytes(binding(), 1);
        raw.push(0xFF);
        assert_eq!(
            validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::EnvelopeDecodeFailure
        );
    }

    /// "Route": the frame's declared `semantic_stream` doesn't match the
    /// physical stream it actually arrived on.
    #[test]
    fn physical_route_mismatch_is_rejected() {
        let state = receive_state();
        let raw = valid_frame_bytes(binding(), 1); // declares General
        assert_eq!(
            validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::Terrain).unwrap_err(),
            SemanticEnvelopeRejectV1::StreamRouteMismatch
        );
    }

    /// A rejected frame cannot mutate anything -- not just tested at
    /// runtime but structurally guaranteed: `validate_semantic_frame_v1`
    /// takes `receive_state: &SemanticReceiveStateV1` (an IMMUTABLE
    /// reference), so there is no code path through this function, reject
    /// or accept, that could touch cursor state even if it tried. This
    /// test is the concrete proof the type signature's promise holds.
    #[test]
    fn rejected_frame_leaves_receive_state_unchanged() {
        let state = receive_state();
        let before = state.next_expected_for(SemanticStreamIdV1::General);
        let raw = valid_frame_bytes(binding(), 99); // a gap, guaranteed reject
        assert!(validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).is_err());
        assert_eq!(state.next_expected_for(SemanticStreamIdV1::General), before);
    }

    #[test]
    fn wrong_direction_is_rejected() {
        let state = receive_state();
        let b = binding();
        let msg = sample_msg();
        let payload_bytes = encode_payload_v1(&msg);
        let profile_root = net_envelope_profile_root_v1();
        let payload_schema = msg.payload_schema();
        let payload_encoding = SemanticPayloadEncodingV1::Bincode2LegacySerde;
        let payload_digest = payload_digest_v1(profile_root, payload_schema, payload_encoding, &payload_bytes);
        let header = NetEnvelopeHeaderV1 {
            profile_root,
            server_boot_id: b.server_boot_id,
            session_id: b.session_id,
            connection_epoch: b.epoch,
            direction: SemanticDirectionV1::ServerToClient, // wrong: server is RECEIVING
            semantic_stream: msg.semantic_stream(),
            sequence: NonZeroU64::new(1).unwrap(),
            causality: SemanticCausalityV1 { producer_tick: None, snapshot: None },
            payload_schema,
            payload_encoding,
            payload_len: payload_bytes.len() as u64,
            payload_digest,
            command_id: None,
        };
        let frame = SemanticWireFrameV1 { header, payload_bytes };
        let raw = common::apex::manifest::encode_manifest_v1(&frame, &semantic_manifest_limits()).unwrap();
        assert_eq!(
            validate_semantic_frame_v1(&raw, &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::WrongDirection
        );
    }
}
