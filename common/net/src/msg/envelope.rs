//! Semantic network envelope: protocol tags and the frozen
//! `NET_ENVELOPE_PROFILE_V1` vocabulary (`APEX-T3.3`, step `T3.3.01`).
//!
//! This step adds the shared protocol-visible vocabulary only -- no send,
//! receive, sequencing, or cross-stream barrier lands here (packet
//! section 8, `T3.3.01`'s own explicit non-goals). Nothing in the live
//! client/server call graph constructs these types yet; they are inert
//! until a later `T3.3.0x` step wires them in.
//!
//! Determinism story: every protocol-visible tag uses an explicit integer
//! discriminant frozen by this module (never a Rust enum's implicit
//! discriminant, per the packet's own adversarial requirement), and the
//! whole tag vocabulary is bound into one frozen `profile_root` digest
//! registered through `APEX-T0.5`'s subsystem-descriptor machinery
//! (`SubsystemSlotIdV1::NetEnvelope`) -- the same content-identity
//! discipline every other subsystem root in this program uses, not a
//! bespoke one-off hash.

use std::num::NonZeroU64;

use sha2::{Digest as _, Sha256};

use common::apex::digest::{
    ArtifactIdentityV1, ContentIdentityV1, DigestBytes32V1, DigestDomainIdV1, SemanticRootV1, digest_canonical_bytes_v1,
    hash_artifact_bytes_v1,
};
use common::apex::identity::{CommandId, ConnectionEpoch, ServerBootId, SessionId, SnapshotEpoch};
use common::apex::manifest::MachineTextV1;
use common::apex::scalar::SchemaVersion;
use common::apex::subsystem::{SubsystemDescriptorV1, SubsystemSlotIdV1};

/// Packet section 7.1. Which side originated the frame.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SemanticDirectionV1 {
    ClientToServer = 1,
    ServerToClient = 2,
}

impl SemanticDirectionV1 {
    pub const fn as_u8(self) -> u8 { self as u8 }

    pub const fn label(self) -> &'static str {
        match self {
            Self::ClientToServer => "bastion/net-envelope/direction/client-to-server/v1",
            Self::ServerToClient => "bastion/net-envelope/direction/server-to-client/v1",
        }
    }

    pub const ALL: [SemanticDirectionV1; 2] = [Self::ClientToServer, Self::ServerToClient];

    pub fn try_from_u8(raw: u8) -> Option<Self> { Self::ALL.into_iter().find(|d| d.as_u8() == raw) }
}

/// Packet section 7.1/3.1. An application-level semantic stream, stable
/// and independent from the physical transport stream ID that carries it
/// (packet section 10.4's named attack: physical stream IDs must never be
/// mistaken for semantic stream identity).
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SemanticStreamIdV1 {
    Bootstrap = 1,
    CharacterScreen = 2,
    InGame = 3,
    General = 4,
    Terrain = 5,
}

impl SemanticStreamIdV1 {
    pub const fn as_u8(self) -> u8 { self as u8 }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Bootstrap => "bastion/net-envelope/stream/bootstrap/v1",
            Self::CharacterScreen => "bastion/net-envelope/stream/character-screen/v1",
            Self::InGame => "bastion/net-envelope/stream/in-game/v1",
            Self::General => "bastion/net-envelope/stream/general/v1",
            Self::Terrain => "bastion/net-envelope/stream/terrain/v1",
        }
    }

    pub const ALL: [SemanticStreamIdV1; 5] = [Self::Bootstrap, Self::CharacterScreen, Self::InGame, Self::General, Self::Terrain];

    pub fn try_from_u8(raw: u8) -> Option<Self> { Self::ALL.into_iter().find(|s| s.as_u8() == raw) }
}

/// Packet section 7.1. Which payload enum a frame's bytes decode as.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SemanticPayloadSchemaV1 {
    ClientGeneral = 1,
    ServerGeneral = 2,
    ServerInit = 3,
}

impl SemanticPayloadSchemaV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const fn label(self) -> &'static str {
        match self {
            Self::ClientGeneral => "bastion/net-envelope/payload-schema/client-general/v1",
            Self::ServerGeneral => "bastion/net-envelope/payload-schema/server-general/v1",
            Self::ServerInit => "bastion/net-envelope/payload-schema/server-init/v1",
        }
    }

    pub const ALL: [SemanticPayloadSchemaV1; 3] = [Self::ClientGeneral, Self::ServerGeneral, Self::ServerInit];

    pub fn try_from_u16(raw: u16) -> Option<Self> { Self::ALL.into_iter().find(|s| s.as_u16() == raw) }
}

/// Packet section 7.1. How `payload_bytes` is encoded. `Bincode2LegacySerde`
/// is the only registered value in V1 -- a future encoding requires a new
/// registered ID, never a silent replacement (same discipline as
/// `DigestAlgorithmIdV1`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SemanticPayloadEncodingV1 {
    Bincode2LegacySerde = 1,
}

impl SemanticPayloadEncodingV1 {
    pub const fn as_u8(self) -> u8 { self as u8 }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Bincode2LegacySerde => "bastion/net-envelope/payload-encoding/bincode2-legacy-serde/v1",
        }
    }

    pub const ALL: [SemanticPayloadEncodingV1; 1] = [Self::Bincode2LegacySerde];

    pub fn try_from_u8(raw: u8) -> Option<Self> { Self::ALL.into_iter().find(|e| e.as_u8() == raw) }
}

/// Dormant per packet sections 5.7/7.2: `APEX-T3.4` fully owns
/// snapshot-domain semantics and the real variant vocabulary. This row
/// only needs a stable, comparable identifier so the field has a type
/// today -- never a sealed enum here, since inventing domain names ahead
/// of the row that actually owns them is exactly the kind of speculative
/// vocabulary `T0.5`'s own slot registry doc comment explicitly disclaims
/// ("no slot is invented speculatively").
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SnapshotDomainId(u32);

impl SnapshotDomainId {
    pub const fn new(raw: u32) -> Self { Self(raw) }

    pub const fn get(self) -> u32 { self.0 }
}

/// Packet section 7.2. `T3.3` carries this; `T3.4` defines production and
/// cross-stream watermark semantics (packet section 5.7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSnapshotRefV1 {
    pub domain: SnapshotDomainId,
    pub epoch: SnapshotEpoch,
}

/// Packet section 7.2. `producer_tick` is descriptive unless a payload
/// profile explicitly gives it authoritative meaning (packet's own
/// caveat, not asserted by this type).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticCausalityV1 {
    pub producer_tick: Option<u64>,
    pub snapshot: Option<SemanticSnapshotRefV1>,
}

/// Packet section 7.3. Framed once with the required `T0.2` deterministic
/// codec at a later step; this step only defines the shape.
#[derive(Clone, Debug, PartialEq)]
pub struct NetEnvelopeHeaderV1 {
    pub profile_root: DigestBytes32V1,
    pub server_boot_id: ServerBootId,
    pub session_id: SessionId,
    pub connection_epoch: ConnectionEpoch,
    pub direction: SemanticDirectionV1,
    pub semantic_stream: SemanticStreamIdV1,
    pub sequence: NonZeroU64,
    pub causality: SemanticCausalityV1,
    pub payload_schema: SemanticPayloadSchemaV1,
    pub payload_encoding: SemanticPayloadEncodingV1,
    pub payload_len: u64,
    pub payload_digest: DigestBytes32V1,
    pub command_id: Option<CommandId>,
}

/// Packet section 7.3. `payload_bytes` is carried as an opaque byte
/// vector through the existing stream framing.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticWireFrameV1 {
    pub header: NetEnvelopeHeaderV1,
    pub payload_bytes: Vec<u8>,
}

const PAYLOAD_DIGEST_MAGIC: &[u8] = b"bastion/net-payload/v1\0";

/// Packet section 7.4:
/// `H("bastion/net-payload/v1\0" || profile_root || payload_schema_u16_be
///   || payload_encoding_u8 || payload_len_u64_be || payload_bytes)`.
/// Compression bytes and physical stream metadata are excluded by
/// construction -- the caller passes exact uncompressed payload bytes.
pub fn payload_digest_v1(
    profile_root: DigestBytes32V1,
    payload_schema: SemanticPayloadSchemaV1,
    payload_encoding: SemanticPayloadEncodingV1,
    payload_bytes: &[u8],
) -> DigestBytes32V1 {
    let mut preimage =
        Vec::with_capacity(PAYLOAD_DIGEST_MAGIC.len() + 32 + 2 + 1 + 8 + payload_bytes.len());
    preimage.extend_from_slice(PAYLOAD_DIGEST_MAGIC);
    preimage.extend_from_slice(profile_root.as_array());
    preimage.extend_from_slice(&payload_schema.as_u16().to_be_bytes());
    preimage.push(payload_encoding.as_u8());
    preimage.extend_from_slice(&(payload_bytes.len() as u64).to_be_bytes());
    preimage.extend_from_slice(payload_bytes);

    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    let out: [u8; 32] = hasher.finalize().into();
    DigestBytes32V1::from_array(out)
}

fn encode_tag_category(buf: &mut Vec<u8>, category: u8, entries: &[(u16, &'static str)]) {
    buf.push(category);
    buf.extend_from_slice(&(entries.len() as u16).to_be_bytes());
    for &(discriminant, label) in entries {
        buf.extend_from_slice(&discriminant.to_be_bytes());
        let label_bytes = label.as_bytes();
        buf.extend_from_slice(&(label_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(label_bytes);
    }
}

/// The frozen, independently-recomputable byte table this row's
/// `profile_root` is a digest of: every registered tag, in every category,
/// as `(discriminant, ascii label)` pairs in ascending-discriminant order.
/// Adding, removing, or relabeling any tag changes this table's bytes and
/// therefore `net_envelope_profile_root_v1()` -- exactly the "frozen
/// vocabulary" guarantee `T3.3.01`'s acceptance gate requires.
fn net_envelope_profile_table_bytes_v1() -> Vec<u8> {
    let mut buf = Vec::new();
    encode_tag_category(&mut buf, 1, &SemanticDirectionV1::ALL.map(|d| (d.as_u8() as u16, d.label())));
    encode_tag_category(&mut buf, 2, &SemanticStreamIdV1::ALL.map(|s| (s.as_u8() as u16, s.label())));
    encode_tag_category(&mut buf, 3, &SemanticPayloadSchemaV1::ALL.map(|s| (s.as_u16(), s.label())));
    encode_tag_category(&mut buf, 4, &SemanticPayloadEncodingV1::ALL.map(|e| (e.as_u8() as u16, e.label())));
    buf
}

/// The `NET_ENVELOPE_PROFILE_V1` registration artifact -- registered
/// through `APEX-T0.5`'s subsystem-descriptor machinery
/// (`SubsystemSlotIdV1::NetEnvelope`) rather than a bespoke one-off hash,
/// per `T3.3.01`'s "Register `NET_ENVELOPE_PROFILE_V1` through T0.5"
/// instruction.
pub fn net_envelope_profile_descriptor_v1() -> SubsystemDescriptorV1 {
    let table = net_envelope_profile_table_bytes_v1();
    let artifact: ArtifactIdentityV1 = hash_artifact_bytes_v1(&table);
    let root = digest_canonical_bytes_v1(DigestDomainIdV1::NetEnvelopeProfile, &table, 1 << 16)
        .expect("frozen tag table is far under the 64KiB limit");
    let semantic = SemanticRootV1 {
        schema_id: MachineTextV1::new("bastion/net-envelope-profile/v1").expect("static ASCII schema id"),
        canonicalization_version: 1,
        root,
    };
    SubsystemDescriptorV1 {
        slot: SubsystemSlotIdV1::NetEnvelope,
        schema: SchemaVersion::new(1),
        content: ContentIdentityV1 { artifact, semantic: Some(semantic) },
    }
}

/// The exact 32-byte `profile_root` value every `NetEnvelopeHeaderV1`
/// carries -- the `semantic` root's digest bytes from
/// [`net_envelope_profile_descriptor_v1`], never the plain artifact
/// digest (this row's vocabulary identity is domain-separated content
/// identity, not raw-byte artifact identity -- see
/// `common/src/apex/digest/mod.rs`'s module doc for why the two are kept
/// mechanically distinct).
pub fn net_envelope_profile_root_v1() -> DigestBytes32V1 {
    net_envelope_profile_descriptor_v1()
        .content
        .semantic
        .expect("net_envelope_profile_descriptor_v1 always sets a semantic root")
        .root
        .bytes
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn direction_tags_are_unique_and_explicit() {
        let ids: HashSet<u8> = SemanticDirectionV1::ALL.iter().map(|d| d.as_u8()).collect();
        assert_eq!(ids.len(), SemanticDirectionV1::ALL.len());
        assert_eq!(SemanticDirectionV1::ClientToServer.as_u8(), 1);
        assert_eq!(SemanticDirectionV1::ServerToClient.as_u8(), 2);
    }

    #[test]
    fn stream_tags_are_unique_and_explicit() {
        let ids: HashSet<u8> = SemanticStreamIdV1::ALL.iter().map(|s| s.as_u8()).collect();
        assert_eq!(ids.len(), SemanticStreamIdV1::ALL.len());
        assert_eq!(SemanticStreamIdV1::Bootstrap.as_u8(), 1);
        assert_eq!(SemanticStreamIdV1::CharacterScreen.as_u8(), 2);
        assert_eq!(SemanticStreamIdV1::InGame.as_u8(), 3);
        assert_eq!(SemanticStreamIdV1::General.as_u8(), 4);
        assert_eq!(SemanticStreamIdV1::Terrain.as_u8(), 5);
    }

    #[test]
    fn payload_schema_tags_are_unique_and_explicit() {
        let ids: HashSet<u16> = SemanticPayloadSchemaV1::ALL.iter().map(|s| s.as_u16()).collect();
        assert_eq!(ids.len(), SemanticPayloadSchemaV1::ALL.len());
        assert_eq!(SemanticPayloadSchemaV1::ClientGeneral.as_u16(), 1);
        assert_eq!(SemanticPayloadSchemaV1::ServerGeneral.as_u16(), 2);
        assert_eq!(SemanticPayloadSchemaV1::ServerInit.as_u16(), 3);
    }

    #[test]
    fn payload_encoding_tags_are_unique_and_explicit() {
        let ids: HashSet<u8> = SemanticPayloadEncodingV1::ALL.iter().map(|e| e.as_u8()).collect();
        assert_eq!(ids.len(), SemanticPayloadEncodingV1::ALL.len());
        assert_eq!(SemanticPayloadEncodingV1::Bincode2LegacySerde.as_u8(), 1);
    }

    #[test]
    fn all_labels_are_ascii_and_unique_within_their_category() {
        let mut check = |labels: Vec<&str>| {
            assert!(labels.iter().all(|l| l.is_ascii()));
            let set: HashSet<&str> = labels.iter().copied().collect();
            assert_eq!(set.len(), labels.len(), "duplicate label in {labels:?}");
        };
        check(SemanticDirectionV1::ALL.iter().map(|d| d.label()).collect());
        check(SemanticStreamIdV1::ALL.iter().map(|s| s.label()).collect());
        check(SemanticPayloadSchemaV1::ALL.iter().map(|s| s.label()).collect());
        check(SemanticPayloadEncodingV1::ALL.iter().map(|e| e.label()).collect());
    }

    #[test]
    fn tag_round_trips_and_rejects_unknown() {
        for d in SemanticDirectionV1::ALL {
            assert_eq!(SemanticDirectionV1::try_from_u8(d.as_u8()), Some(d));
        }
        assert_eq!(SemanticDirectionV1::try_from_u8(0), None);
        assert_eq!(SemanticDirectionV1::try_from_u8(3), None);

        for s in SemanticStreamIdV1::ALL {
            assert_eq!(SemanticStreamIdV1::try_from_u8(s.as_u8()), Some(s));
        }
        assert_eq!(SemanticStreamIdV1::try_from_u8(0), None);
        assert_eq!(SemanticStreamIdV1::try_from_u8(6), None);

        for s in SemanticPayloadSchemaV1::ALL {
            assert_eq!(SemanticPayloadSchemaV1::try_from_u16(s.as_u16()), Some(s));
        }
        assert_eq!(SemanticPayloadSchemaV1::try_from_u16(0), None);
        assert_eq!(SemanticPayloadSchemaV1::try_from_u16(4), None);

        for e in SemanticPayloadEncodingV1::ALL {
            assert_eq!(SemanticPayloadEncodingV1::try_from_u8(e.as_u8()), Some(e));
        }
        assert_eq!(SemanticPayloadEncodingV1::try_from_u8(0), None);
        assert_eq!(SemanticPayloadEncodingV1::try_from_u8(2), None);
    }

    /// `T3.3.01`'s objective acceptance gate: independent runs emit
    /// byte-identical header vectors. `profile_root` is the one piece of
    /// this step's output that is computed (not a literal constant), so
    /// this is the golden-vector proof -- same call, twice, byte-identical,
    /// plus a frozen exact value so a future accidental table edit is
    /// caught even if both calls in this test agree with each other.
    #[test]
    fn profile_root_is_deterministic_and_matches_frozen_golden_vector() {
        let a = net_envelope_profile_root_v1();
        let b = net_envelope_profile_root_v1();
        assert_eq!(a.as_array(), b.as_array());
        assert_eq!(
            a.to_human_v1(),
            "sha256:14a435666ca650d1e72e900196d3f6dbbdbf49da067fe87a47cfdc7a067a1cbb",
            "NET_ENVELOPE_PROFILE_V1 table changed -- recompute and update this golden vector deliberately, \
             it must never drift silently"
        );
    }

    #[test]
    fn profile_descriptor_registers_under_net_envelope_slot() {
        let descriptor = net_envelope_profile_descriptor_v1();
        assert_eq!(descriptor.slot, SubsystemSlotIdV1::NetEnvelope);
        assert_eq!(descriptor.schema.get(), 1);
        assert_eq!(descriptor.content.semantic.as_ref().unwrap().root.domain, DigestDomainIdV1::NetEnvelopeProfile);
    }

    /// Reordering the tag categories or the variants within a category
    /// must change the table bytes (and therefore the root) -- otherwise
    /// two semantically different vocabularies could collide, which would
    /// defeat the whole point of freezing this table.
    #[test]
    fn different_tag_orderings_would_produce_different_roots() {
        let mut buf_normal = Vec::new();
        encode_tag_category(&mut buf_normal, 1, &[(1, "a"), (2, "b")]);
        let mut buf_swapped = Vec::new();
        encode_tag_category(&mut buf_swapped, 1, &[(2, "b"), (1, "a")]);
        assert_ne!(buf_normal, buf_swapped);
    }

    #[test]
    fn payload_digest_excludes_nothing_but_what_the_spec_names() {
        let root = net_envelope_profile_root_v1();
        let a = payload_digest_v1(root, SemanticPayloadSchemaV1::ClientGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, b"same payload");
        let b = payload_digest_v1(root, SemanticPayloadSchemaV1::ClientGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, b"same payload");
        assert_eq!(a.as_array(), b.as_array());

        // Different schema -> different digest, same bytes.
        let c = payload_digest_v1(root, SemanticPayloadSchemaV1::ServerGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, b"same payload");
        assert_ne!(a.as_array(), c.as_array());

        // Different payload bytes -> different digest.
        let d = payload_digest_v1(root, SemanticPayloadSchemaV1::ClientGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, b"different payload!");
        assert_ne!(a.as_array(), d.as_array());
    }
}
