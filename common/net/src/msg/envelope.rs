//! Semantic network envelope: protocol tags, the frozen
//! `NET_ENVELOPE_PROFILE_V1` vocabulary, and pinned payload codec/digest
//! (`APEX-T3.3`, steps `T3.3.01`-`T3.3.03`).
//!
//! These steps add the shared protocol-visible vocabulary and payload
//! codec only -- no send, receive, sequencing, or cross-stream barrier
//! lands here (packet section 8's non-goals for both steps). Nothing in
//! the live client/server call graph constructs or calls these yet; they
//! are inert until a later `T3.3.0x` step wires them in.
//!
//! Determinism story: every protocol-visible tag uses an explicit integer
//! discriminant frozen by this module (never a Rust enum's implicit
//! discriminant, per the packet's own adversarial requirement), and the
//! whole tag vocabulary is bound into one frozen `profile_root` digest
//! registered through `APEX-T0.5`'s subsystem-descriptor machinery
//! (`SubsystemSlotIdV1::NetEnvelope`) -- the same content-identity
//! discipline every other subsystem root in this program uses, not a
//! bespoke one-off hash. Payload bytes are encoded exactly once with the
//! pinned `bincode::config::legacy()` (`SemanticPayloadEncodingV1::Bincode2LegacySerde`,
//! matching the existing wire codec's own pin in `network/src/message.rs`)
//! and decoded with the same exact-consume discipline `T3.3.02` gave the
//! outer frame -- payload byte identity, never semantic equivalence
//! (packet section 5.5's own disclaimer, proven false-if-untrue by this
//! module's `unordered_map_payloads_are_not_byte_stable` negative test).

use std::num::NonZeroU64;

use bincode::config::legacy;
use bincode::serde::{decode_from_slice, encode_to_vec};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use sha2::{Digest as _, Sha256};

use common::apex::digest::{
    ArtifactIdentityV1, ContentIdentityV1, DigestBytes32V1, DigestDomainIdV1, SemanticRootV1, digest_canonical_bytes_v1,
    hash_artifact_bytes_v1,
};
use common::apex::identity::{CommandId, ConnectionEpoch, ServerBootId, SessionId, SnapshotEpoch};
use common::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, MachineTextV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1,
    ManifestEncodeV1, ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};
use common::apex::scalar::SchemaVersion;
use common::apex::subsystem::{SubsystemDescriptorV1, SubsystemSlotIdV1};

use crate::msg::client::ClientGeneral;
use crate::msg::server::{ServerGeneral, ServerInit};

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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
// T3.4.20b: serialized as its FROZEN tag, never as a variant index -- a
// wire payload carrying a stream must move with `as_u8`, not with
// declaration order.
#[serde(into = "u8", try_from = "u8")]
pub enum SemanticStreamIdV1 {
    Bootstrap = 1,
    CharacterScreen = 2,
    InGame = 3,
    General = 4,
    Terrain = 5,
}

impl From<SemanticStreamIdV1> for u8 {
    fn from(s: SemanticStreamIdV1) -> u8 { s.as_u8() }
}

impl TryFrom<u8> for SemanticStreamIdV1 {
    type Error = &'static str;

    fn try_from(raw: u8) -> Result<Self, Self::Error> {
        Self::try_from_u8(raw).ok_or("unknown semantic stream tag")
    }
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
            // APEX-T5.2 (the T5 tier's single wire bump): both payload
            // schemas CHANGED — `ClientGeneral::PlayerPhysics` gained a
            // weather-snapshot reference, `ServerGeneral`'s weather
            // messages gained the snapshot id they belong to, and
            // `ServerGeneral::InputReceipt` is new. The label is where a
            // payload schema's version lives, so it moves here and the
            // frozen table moves with it.
            //
            // FOUND WHILE DOING THIS, and worth more than the bump: the
            // frozen table digests the TAG VOCABULARY, not the payload
            // CONTENTS. Changing a variant's shape does not move
            // `profile_root` on its own — this label bump is what makes
            // the change visible, and nothing forces a future author to
            // remember it. See `payload_schema_labels_carry_the_wire_version`.
            Self::ClientGeneral => "bastion/net-envelope/payload-schema/client-general/v2",
            Self::ServerGeneral => "bastion/net-envelope/payload-schema/server-general/v2",
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

/// Packet section 5.9/`T3.3.05`: the narrow post-auth wire mode one
/// attachment selects at registration. `T4.1`'s `BootstrapManifestV1`
/// later subsumes this into a fuller negotiation; until then this is
/// deliberately small -- exactly the two modes that exist.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SemanticProtocolIdV1 {
    Legacy = 1,
    NetEnvelopeV1 = 2,
}

impl SemanticProtocolIdV1 {
    pub const fn as_u8(self) -> u8 { self as u8 }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Legacy => "bastion/net-envelope/semantic-protocol/legacy/v1",
            Self::NetEnvelopeV1 => "bastion/net-envelope/semantic-protocol/net-envelope-v1/v1",
        }
    }

    pub const ALL: [SemanticProtocolIdV1; 2] = [Self::Legacy, Self::NetEnvelopeV1];

    pub fn try_from_u8(raw: u8) -> Option<Self> { Self::ALL.into_iter().find(|p| p.as_u8() == raw) }
}

/// The server's currently-fixed advertised set, always sorted ascending
/// by tag. `T3.3.05` builds the negotiation mechanism, not a
/// certified-mode config surface (packet section 5.9: "`T4.1` later
/// subsumes this narrow negotiation") -- both modes are always
/// advertised today, so `IncompatibleSemanticProtocol` is a real,
/// tested, but currently-dormant rejection (no live client requests
/// anything but `Legacy` until `T3.3.07`).
pub fn server_supported_semantic_protocols_v1() -> Vec<SemanticProtocolIdV1> {
    vec![SemanticProtocolIdV1::Legacy, SemanticProtocolIdV1::NetEnvelopeV1]
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
    /// `T3.4.20`: present exactly when this frame belongs to a
    /// checkpoint. Absent means unfenced traffic; the receiver decides
    /// what that is allowed to be, per participation class.
    pub checkpoint: Option<super::checkpoint::CheckpointedEnvelopeContextV1>,
}

/// Packet section 7.3. `payload_bytes` is carried as an opaque byte
/// vector through the existing stream framing.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticWireFrameV1 {
    pub header: NetEnvelopeHeaderV1,
    pub payload_bytes: Vec<u8>,
}

// `T3.3.07`: canonical `BastionManifestEncodingV1` (T0.2) encoding for the
// envelope frame -- packet section 7.3: "SemanticWireFrameV1 is encoded
// with the already-required deterministic T0.2 codec". Built on
// `T0.4.6`'s tagged opaque-identity codec (ServerBootId/SessionId/
// ConnectionEpoch/CommandId) and reuses `DigestBytes32V1::try_from_slice`
// (T0.3) for the two digest fields -- no bespoke byte handling here,
// every primitive is the shared, already-tested one.

fn digest32_to_value(d: &DigestBytes32V1) -> ManifestValueV1 { ManifestValueV1::Bytes(d.as_array().to_vec()) }

fn digest32_from_value(value: ManifestValueV1) -> Result<DigestBytes32V1, ManifestSchemaErrorV1> {
    let ManifestValueV1::Bytes(b) = value else {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("expected a 32-byte bytestring"));
    };
    DigestBytes32V1::try_from_slice(&b).map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("digest must be exactly 32 bytes"))
}

/// `T3.4.20`: the checkpoint context as three canonical fields —
/// epoch, optional ordinal, descriptor root.
fn checkpoint_ctx_to_value(
    ctx: &super::checkpoint::CheckpointedEnvelopeContextV1,
) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    let mut entries = vec![
        (FieldIdV1::new(1), ManifestValueV1::Unsigned(ctx.epoch)),
        (FieldIdV1::new(3), ManifestValueV1::Bytes(ctx.descriptor_root.to_vec())),
    ];
    if let Some(ordinal) = ctx.ordinal {
        entries.push((FieldIdV1::new(2), ManifestValueV1::Unsigned(ordinal.0)));
    }
    Ok(ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries)?))
}

fn checkpoint_ctx_from_value(
    value: ManifestValueV1,
) -> Result<super::checkpoint::CheckpointedEnvelopeContextV1, ManifestSchemaErrorV1> {
    let ManifestValueV1::Map(map) = value else {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("checkpoint context must be a map"));
    };
    let mut fields = StructFieldsV1::new(map);
    let epoch = match fields.take_required(FieldIdV1::new(1))? {
        ManifestValueV1::Unsigned(v) => v,
        _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
    };
    let ordinal = match fields.take_optional(FieldIdV1::new(2))? {
        Some(ManifestValueV1::Unsigned(v)) => Some(super::checkpoint::CheckpointOrdinalV1(v)),
        Some(_) => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        None => None,
    };
    let descriptor_root = match fields.take_required(FieldIdV1::new(3))? {
        ManifestValueV1::Bytes(b) => <[u8; 32]>::try_from(b.as_slice())
            .map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("descriptor root must be exactly 32 bytes"))?,
        _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
    };
    fields.finish_no_unknown()?;
    Ok(super::checkpoint::CheckpointedEnvelopeContextV1 { epoch, ordinal, descriptor_root })
}

impl ManifestEncodeV1 for SnapshotDomainId {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(ManifestValueV1::Unsigned(self.0 as u64)) }
}

impl ManifestDecodeV1 for SnapshotDomainId {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        match value {
            ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => Ok(Self(v as u32)),
            _ => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        }
    }
}

impl ManifestEncodeV1 for SemanticSnapshotRefV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), self.domain.to_manifest_value_v1()?),
            (FieldIdV1::new(2), self.epoch.to_manifest_value_v1()?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SemanticSnapshotRefV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let domain = SnapshotDomainId::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let epoch = SnapshotEpoch::from_manifest_value_v1(fields.take_required(FieldIdV1::new(2))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { domain, epoch })
    }
}

impl ManifestEncodeV1 for SemanticCausalityV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut entries = Vec::new();
        if let Some(tick) = self.producer_tick {
            entries.push((FieldIdV1::new(1), ManifestValueV1::Unsigned(tick)));
        }
        if let Some(snapshot) = &self.snapshot {
            entries.push((FieldIdV1::new(2), snapshot.to_manifest_value_v1()?));
        }
        let map = CanonicalFieldMapV1::try_from_entries(entries)?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SemanticCausalityV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let producer_tick = match fields.take_optional(FieldIdV1::new(1))? {
            Some(ManifestValueV1::Unsigned(v)) => Some(v),
            Some(_) => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
            None => None,
        };
        let snapshot = match fields.take_optional(FieldIdV1::new(2))? {
            Some(v) => Some(SemanticSnapshotRefV1::from_manifest_value_v1(v)?),
            None => None,
        };
        fields.finish_no_unknown()?;
        Ok(Self { producer_tick, snapshot })
    }
}

impl ManifestEncodeV1 for NetEnvelopeHeaderV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut entries = vec![
            (FieldIdV1::new(1), digest32_to_value(&self.profile_root)),
            (FieldIdV1::new(2), self.server_boot_id.to_manifest_value_v1()?),
            (FieldIdV1::new(3), self.session_id.to_manifest_value_v1()?),
            (FieldIdV1::new(4), self.connection_epoch.to_manifest_value_v1()?),
            (FieldIdV1::new(5), ManifestValueV1::Unsigned(self.direction.as_u8() as u64)),
            (FieldIdV1::new(6), ManifestValueV1::Unsigned(self.semantic_stream.as_u8() as u64)),
            (FieldIdV1::new(7), ManifestValueV1::Unsigned(self.sequence.get())),
            (FieldIdV1::new(8), self.causality.to_manifest_value_v1()?),
            (FieldIdV1::new(9), ManifestValueV1::Unsigned(self.payload_schema.as_u16() as u64)),
            (FieldIdV1::new(10), ManifestValueV1::Unsigned(self.payload_encoding.as_u8() as u64)),
            (FieldIdV1::new(11), ManifestValueV1::Unsigned(self.payload_len)),
            (FieldIdV1::new(12), digest32_to_value(&self.payload_digest)),
        ];
        if let Some(command_id) = &self.command_id {
            entries.push((FieldIdV1::new(13), command_id.to_manifest_value_v1()?));
        }
        if let Some(checkpoint) = &self.checkpoint {
            entries.push((FieldIdV1::new(14), checkpoint_ctx_to_value(checkpoint)?));
        }
        let map = CanonicalFieldMapV1::try_from_entries(entries)?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for NetEnvelopeHeaderV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let profile_root = digest32_from_value(fields.take_required(FieldIdV1::new(1))?)?;
        let server_boot_id = ServerBootId::from_manifest_value_v1(fields.take_required(FieldIdV1::new(2))?)?;
        let session_id = SessionId::from_manifest_value_v1(fields.take_required(FieldIdV1::new(3))?)?;
        let connection_epoch = ConnectionEpoch::from_manifest_value_v1(fields.take_required(FieldIdV1::new(4))?)?;
        let direction = match fields.take_required(FieldIdV1::new(5))? {
            ManifestValueV1::Unsigned(v) if v <= u8::MAX as u64 => SemanticDirectionV1::try_from_u8(v as u8)
                .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown direction tag"))?,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let semantic_stream = match fields.take_required(FieldIdV1::new(6))? {
            ManifestValueV1::Unsigned(v) if v <= u8::MAX as u64 => SemanticStreamIdV1::try_from_u8(v as u8)
                .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown stream tag"))?,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let sequence = match fields.take_required(FieldIdV1::new(7))? {
            ManifestValueV1::Unsigned(v) => {
                NonZeroU64::new(v).ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("sequence zero"))?
            },
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let causality = SemanticCausalityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(8))?)?;
        let payload_schema = match fields.take_required(FieldIdV1::new(9))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => SemanticPayloadSchemaV1::try_from_u16(v as u16)
                .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown payload schema tag"))?,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let payload_encoding = match fields.take_required(FieldIdV1::new(10))? {
            ManifestValueV1::Unsigned(v) if v <= u8::MAX as u64 => SemanticPayloadEncodingV1::try_from_u8(v as u8)
                .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown payload encoding tag"))?,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let payload_len = match fields.take_required(FieldIdV1::new(11))? {
            ManifestValueV1::Unsigned(v) => v,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let payload_digest = digest32_from_value(fields.take_required(FieldIdV1::new(12))?)?;
        let command_id = match fields.take_optional(FieldIdV1::new(13))? {
            Some(v) => Some(CommandId::from_manifest_value_v1(v)?),
            None => None,
        };
        let checkpoint = match fields.take_optional(FieldIdV1::new(14))? {
            Some(v) => Some(checkpoint_ctx_from_value(v)?),
            None => None,
        };
        fields.finish_no_unknown()?;
        Ok(Self {
            profile_root,
            server_boot_id,
            session_id,
            connection_epoch,
            direction,
            semantic_stream,
            sequence,
            causality,
            payload_schema,
            payload_encoding,
            payload_len,
            payload_digest,
            command_id,
            checkpoint,
        })
    }
}

impl ManifestEncodeV1 for SemanticWireFrameV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), self.header.to_manifest_value_v1()?),
            (FieldIdV1::new(2), ManifestValueV1::Bytes(self.payload_bytes.clone())),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SemanticWireFrameV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let header = NetEnvelopeHeaderV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let ManifestValueV1::Bytes(payload_bytes) = fields.take_required(FieldIdV1::new(2))? else {
            return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("expected a bytestring payload"));
        };
        fields.finish_no_unknown()?;
        Ok(Self { header, payload_bytes })
    }
}

/// `T3.3.06`: the boot/session/epoch triple every semantic cursor is
/// keyed to (packet's own repeated requirement: "all keys include
/// boot/session/epoch/direction/stream"). Distinct from
/// `server::SessionBindingV1` -- that type is the wire-echoed admission
/// binding (T3.2); this one additionally carries `server_boot_id` because
/// a cursor must never survive a server restart even if `session_id`
/// somehow collided across boot incarnations (it can't, opaque UUIDv4,
/// but the key is defense in depth per the packet's own phrasing, not
/// this row inventing new paranoia).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActiveSessionBindingV1 {
    pub server_boot_id: ServerBootId,
    pub session_id: SessionId,
    pub epoch: ConnectionEpoch,
}

const FIRST_SEQUENCE: NonZeroU64 = NonZeroU64::new(1).expect("1 is nonzero");

/// Packet section 7.6. One send cursor per semantic stream (`[General,
/// Bootstrap, CharacterScreen, InGame, Terrain]` in `SemanticStreamIdV1`
/// tag order), owned by whichever side is sending in this direction.
/// `T3.3.06` only creates this and its reset constructor -- no sender
/// exists yet that consumes/advances it (`T3.3.07`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSendStateV1 {
    pub binding: ActiveSessionBindingV1,
    next: [NonZeroU64; 5],
}

impl SemanticSendStateV1 {
    /// `T3.3.06`'s "reset constructor keyed by `ActiveSessionBindingV1`":
    /// every one of the five per-stream cursors starts at `1` (packet
    /// section 8: "Five cursor domains per direction start at one").
    /// Called on a freshly-accepted binding and again whenever the epoch
    /// advances ("higher epoch replaces") -- never partially reset.
    pub fn new(binding: ActiveSessionBindingV1) -> Self { Self { binding, next: [FIRST_SEQUENCE; 5] } }

    pub fn binding(&self) -> ActiveSessionBindingV1 { self.binding }

    pub fn next_for(&self, stream: SemanticStreamIdV1) -> NonZeroU64 { self.next[stream_index(stream)] }

    /// `T3.3.07`: consumes and returns this stream's current sequence,
    /// advancing the cursor for next time -- packet's own ordering rule,
    /// "sequence is consumed before send and never reused after failure":
    /// the cursor already reflects the NEXT value the instant this
    /// returns, regardless of whether the caller's subsequent encode/send
    /// actually succeeds. `CounterAdvanceErrorV1::Exhausted` reuses
    /// `T0.4`'s existing checked-counter error rather than a bespoke one.
    pub fn allocate_sequence(&mut self, stream: SemanticStreamIdV1) -> Result<NonZeroU64, common::apex::identity::CounterAdvanceErrorV1> {
        let idx = stream_index(stream);
        let current = self.next[idx];
        let advanced = current.checked_add(1).ok_or(common::apex::identity::CounterAdvanceErrorV1::Exhausted)?;
        self.next[idx] = advanced;
        Ok(current)
    }

    /// `T3.4.10`: reserves `counts[i]` consecutive sequences on every
    /// stream at once, returning each stream's first reserved value.
    /// All-or-nothing: if ANY stream would exhaust, no cursor moves --
    /// a checkpoint plan must never be able to consume part of its
    /// sequence range. A count of zero reserves nothing and reports the
    /// cursor unchanged.
    pub fn reserve_sequences_v1(&mut self, counts: [u64; 5]) -> Result<[NonZeroU64; 5], common::apex::identity::CounterAdvanceErrorV1> {
        let first = self.next;
        let mut advanced = self.next;
        for idx in 0..5 {
            let end = first[idx]
                .get()
                .checked_add(counts[idx])
                .ok_or(common::apex::identity::CounterAdvanceErrorV1::Exhausted)?;
            advanced[idx] = NonZeroU64::new(end).expect("first is nonzero and counts are non-negative");
        }
        self.next = advanced;
        Ok(first)
    }

    /// Test-only: constructs a state with an arbitrary starting cursor
    /// per stream, so exhaustion at the real `u64::MAX` boundary can be
    /// tested directly against `allocate_sequence` itself rather than
    /// reasoned about indirectly (looping `u64::MAX` times is not a real
    /// option).
    #[cfg(test)]
    fn with_cursors_for_test(binding: ActiveSessionBindingV1, next: [NonZeroU64; 5]) -> Self { Self { binding, next } }
}

/// Packet section 7.6. The receive-side twin of [`SemanticSendStateV1`].
/// `highest_snapshot`/`terminal` are dormant per sections 5.7/T3.4 and
/// T3.3.08+ respectively -- carried now so the reset shape is frozen,
/// never written to by this step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticReceiveStateV1 {
    pub binding: ActiveSessionBindingV1,
    next_expected: [NonZeroU64; 5],
    pub highest_snapshot: std::collections::BTreeMap<SnapshotDomainId, SnapshotEpoch>,
    pub terminal: Option<SemanticProtocolTerminalV1>,
}

impl SemanticReceiveStateV1 {
    pub fn new(binding: ActiveSessionBindingV1) -> Self {
        Self { binding, next_expected: [FIRST_SEQUENCE; 5], highest_snapshot: std::collections::BTreeMap::new(), terminal: None }
    }

    pub fn binding(&self) -> ActiveSessionBindingV1 { self.binding }

    pub fn next_expected_for(&self, stream: SemanticStreamIdV1) -> NonZeroU64 { self.next_expected[stream_index(stream)] }

    /// `T3.3.08`: commits acceptance of the sequence this stream was
    /// expecting -- called only AFTER every other envelope/payload check
    /// has already passed (packet: "cursor does not advance on
    /// validation failure; it advances before handler call"). The
    /// zero-gap MVP (packet section 5.4) means the caller has already
    /// confirmed `received == next_expected_for(stream)` before calling
    /// this; this method's own job is only to advance the cursor for
    /// next time, which can itself exhaust at `u64::MAX` even though the
    /// just-accepted value was valid.
    pub fn advance_expected(&mut self, stream: SemanticStreamIdV1) -> Result<(), common::apex::identity::CounterAdvanceErrorV1> {
        let idx = stream_index(stream);
        let advanced = self.next_expected[idx].checked_add(1).ok_or(common::apex::identity::CounterAdvanceErrorV1::Exhausted)?;
        self.next_expected[idx] = advanced;
        Ok(())
    }

    /// `T3.3.17`: packet's own "lower same-stream/domain snapshots
    /// reject" -- local monotonic non-decreasing constraint per domain
    /// (`highest_snapshot` is keyed by domain only, shared across every
    /// semantic stream this attachment carries -- T3.3.01's own doc
    /// calls that shape frozen, so this uses it exactly as declared
    /// rather than adding stream-scoping to the key). Equal is accepted
    /// (non-decreasing, not strictly-increasing -- the packet says
    /// "lower ... reject", not "equal ... reject"). A domain seen for
    /// the first time always passes -- nothing to compare against yet.
    /// Pure: mirrors `next_expected_for` (check) being separate from
    /// `advance_expected`/`commit_snapshot` (commit) -- "cursor does
    /// not advance on validation failure."
    pub fn snapshot_is_fresh(&self, snapshot: &SemanticSnapshotRefV1) -> bool {
        self.highest_snapshot.get(&snapshot.domain).is_none_or(|&highest| snapshot.epoch >= highest)
    }

    /// `T3.3.17`: commits acceptance of a snapshot ref -- called only
    /// AFTER `snapshot_is_fresh` has already passed, same "advance only
    /// after validation" discipline `advance_expected` follows for
    /// sequence. Monotonic non-decreasing: only ever raises a domain's
    /// watermark (an accepted EQUAL epoch is a correct no-op here).
    pub fn commit_snapshot(&mut self, snapshot: SemanticSnapshotRefV1) {
        self.highest_snapshot
            .entry(snapshot.domain)
            .and_modify(|highest| {
                if snapshot.epoch > *highest {
                    *highest = snapshot.epoch;
                }
            })
            .or_insert(snapshot.epoch);
    }

    /// Test-only twin of `SemanticSendStateV1::with_cursors_for_test`.
    #[cfg(test)]
    fn with_cursors_for_test(binding: ActiveSessionBindingV1, next_expected: [NonZeroU64; 5]) -> Self {
        Self { binding, next_expected, highest_snapshot: std::collections::BTreeMap::new(), terminal: None }
    }
}

const fn stream_index(stream: SemanticStreamIdV1) -> usize {
    match stream {
        SemanticStreamIdV1::Bootstrap => 0,
        SemanticStreamIdV1::CharacterScreen => 1,
        SemanticStreamIdV1::InGame => 2,
        SemanticStreamIdV1::General => 3,
        SemanticStreamIdV1::Terrain => 4,
    }
}

/// Packet section 7.9's connection-level terminal outcomes (as opposed to
/// `SemanticEnvelopeRejectV1`'s per-frame rejection reasons). Dormant
/// until `T3.3.08`'s ingress validation actually sets `.terminal` on a
/// `SemanticReceiveStateV1` -- added now, in full, for the same reason
/// `SemanticEnvelopeRejectV1` was: one frozen name set for every later
/// step to reference, not each inventing its own subset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProtocolTerminalV1 {
    ResyncRequired,
    SequenceExhausted,
    ApplicationError,
    ProtocolViolation,
    SendFailedAfterSequenceAllocated,
}

impl SemanticProtocolTerminalV1 {
    /// `T3.3.18`: stable, field-independent code -- the metrics label
    /// and terminal-catalog artifact key. Every variant here is already
    /// fieldless, but the code is a SEPARATE, deliberately-frozen string
    /// (never `{:?}`) so a `Debug` reformatting elsewhere in this file
    /// can never silently change a metrics label's cardinality/spelling.
    pub const fn code(self) -> &'static str {
        match self {
            Self::ResyncRequired => "resync_required",
            Self::SequenceExhausted => "sequence_exhausted",
            Self::ApplicationError => "application_error",
            Self::ProtocolViolation => "protocol_violation",
            Self::SendFailedAfterSequenceAllocated => "send_failed_after_sequence_allocated",
        }
    }

    pub const ALL: [SemanticProtocolTerminalV1; 5] = [
        Self::ResyncRequired,
        Self::SequenceExhausted,
        Self::ApplicationError,
        Self::ProtocolViolation,
        Self::SendFailedAfterSequenceAllocated,
    ];

    /// `T3.3.18`: "add protocol disconnect mapping" -- every connection-
    /// level terminal maps to exactly one EXISTING `DisconnectReason`
    /// (no new variants invented; the row's own compatibility note says
    /// "disconnect variants follow negotiated wire version", meaning
    /// this mapping exists ALONGSIDE the untouched Legacy disconnect
    /// paths, not replacing them). `ProtocolViolation` -- a client that
    /// sent malformed/deliberately-invalid traffic -- maps to `Kicked`,
    /// the same reason this codebase already uses for other deliberate-
    /// misbehavior disconnects (`register.rs`'s own "logged in from
    /// another location" kick). Every other terminal here is a
    /// transient/resource condition, not evidence of bad faith, so it
    /// maps to `NetworkError` (the existing catch-all this codebase
    /// already uses for non-graceful, non-malicious disconnects).
    pub const fn disconnect_reason(self) -> common::comp::DisconnectReason {
        use common::comp::DisconnectReason as D;
        match self {
            Self::ProtocolViolation => D::Kicked,
            Self::ResyncRequired | Self::SequenceExhausted | Self::ApplicationError | Self::SendFailedAfterSequenceAllocated => {
                D::NetworkError
            },
        }
    }
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

/// Packet section 7.9. Typed rejection reasons a receiver can name for a
/// semantic frame. Frozen vocabulary, added in full now (matching the
/// tag-enum pattern above) even though most variants are unreachable
/// until later `T3.3.0x` steps wire in ingress validation -- every
/// variant is exported, so nothing here is genuinely dead code, and later
/// steps get one already-agreed set of terminal names instead of each
/// inventing its own subset piecemeal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticEnvelopeRejectV1 {
    UnsupportedProfile,
    WrongBoot,
    WrongSession,
    StaleEpoch,
    FutureEpoch,
    WrongDirection,
    UnknownStream,
    StreamRouteMismatch,
    SequenceZero,
    DuplicateSequence,
    ReplaySequence,
    SequenceGap { expected: u64, received: u64 },
    SequenceExhausted,
    PayloadEncodingUnsupported,
    PayloadSchemaUnsupported,
    PayloadLengthMismatch,
    PayloadDigestMismatch,
    EnvelopeDecodeFailure,
    EnvelopeTrailingBytes,
    PayloadDecodeFailure,
    PayloadTrailingBytes,
    StaleSnapshot,
    CommandIdUnsupported,
    NoActiveAttachment,
    StaleEgressBinding,
    DuplicateOrderKey,
    OrderKeyTooLarge,
    /// `T3.3.15`: added when the egress owner's own encode step turned
    /// out to be genuinely fallible (unlike `T3.3.07`'s client-side
    /// `send_semantic_v1`, which treats its own encode as impossible-to-
    /// fail-by-construction and `.expect()`s it -- that function has
    /// exactly one, already-validated call shape; egress instead encodes
    /// arbitrary payloads from many different producers, so a genuine
    /// per-intent reject path is warranted here where it wasn't there).
    EncodeFailure,
    /// `T3.3.17`: `causality.snapshot`'s domain is not in the active
    /// `NetEnvelopeCausalityProfileV1`'s declared set. Production
    /// declares no domains ("leave snapshot absent until a producer has
    /// defined epochs"), so this is unreachable on real traffic today --
    /// only a test profile with a non-empty declared set exercises it.
    UnknownDomain,
    /// `T3.3.17`: the frame's causality does not satisfy its payload
    /// schema's declared requirement in the active
    /// `NetEnvelopeCausalityProfileV1` (e.g. a schema declared
    /// tick-required arrived tickless). Production declares every
    /// schema fully optional, so this is unreachable on real traffic
    /// today -- only a test profile with a required field exercises it.
    CausalityProfileMismatch,
}

impl SemanticEnvelopeRejectV1 {
    /// `T3.3.18`: stable, field-independent code -- the metrics label
    /// and terminal-catalog artifact key. `SequenceGap`'s own
    /// `expected`/`received` values are per-frame data (unbounded
    /// cardinality), never metrics cardinality -- every instance of it
    /// shares this one code regardless of its fields.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedProfile => "unsupported_profile",
            Self::WrongBoot => "wrong_boot",
            Self::WrongSession => "wrong_session",
            Self::StaleEpoch => "stale_epoch",
            Self::FutureEpoch => "future_epoch",
            Self::WrongDirection => "wrong_direction",
            Self::UnknownStream => "unknown_stream",
            Self::StreamRouteMismatch => "stream_route_mismatch",
            Self::SequenceZero => "sequence_zero",
            Self::DuplicateSequence => "duplicate_sequence",
            Self::ReplaySequence => "replay_sequence",
            Self::SequenceGap { .. } => "sequence_gap",
            Self::SequenceExhausted => "sequence_exhausted",
            Self::PayloadEncodingUnsupported => "payload_encoding_unsupported",
            Self::PayloadSchemaUnsupported => "payload_schema_unsupported",
            Self::PayloadLengthMismatch => "payload_length_mismatch",
            Self::PayloadDigestMismatch => "payload_digest_mismatch",
            Self::EnvelopeDecodeFailure => "envelope_decode_failure",
            Self::EnvelopeTrailingBytes => "envelope_trailing_bytes",
            Self::PayloadDecodeFailure => "payload_decode_failure",
            Self::PayloadTrailingBytes => "payload_trailing_bytes",
            Self::StaleSnapshot => "stale_snapshot",
            Self::CommandIdUnsupported => "command_id_unsupported",
            Self::NoActiveAttachment => "no_active_attachment",
            Self::StaleEgressBinding => "stale_egress_binding",
            Self::DuplicateOrderKey => "duplicate_order_key",
            Self::OrderKeyTooLarge => "order_key_too_large",
            Self::EncodeFailure => "encode_failure",
            Self::UnknownDomain => "unknown_domain",
            Self::CausalityProfileMismatch => "causality_profile_mismatch",
        }
    }

    /// One representative instance per variant (`SequenceGap` gets
    /// placeholder field values -- its own fields are not part of its
    /// code's identity). Used for the terminal-catalog artifact and the
    /// completeness/uniqueness tests below.
    pub const ALL: [SemanticEnvelopeRejectV1; 30] = [
        Self::UnsupportedProfile,
        Self::WrongBoot,
        Self::WrongSession,
        Self::StaleEpoch,
        Self::FutureEpoch,
        Self::WrongDirection,
        Self::UnknownStream,
        Self::StreamRouteMismatch,
        Self::SequenceZero,
        Self::DuplicateSequence,
        Self::ReplaySequence,
        Self::SequenceGap { expected: 0, received: 0 },
        Self::SequenceExhausted,
        Self::PayloadEncodingUnsupported,
        Self::PayloadSchemaUnsupported,
        Self::PayloadLengthMismatch,
        Self::PayloadDigestMismatch,
        Self::EnvelopeDecodeFailure,
        Self::EnvelopeTrailingBytes,
        Self::PayloadDecodeFailure,
        Self::PayloadTrailingBytes,
        Self::StaleSnapshot,
        Self::CommandIdUnsupported,
        Self::NoActiveAttachment,
        Self::StaleEgressBinding,
        Self::DuplicateOrderKey,
        Self::OrderKeyTooLarge,
        Self::EncodeFailure,
        Self::UnknownDomain,
        Self::CausalityProfileMismatch,
    ];
}

/// `T3.3.18`: "server/client counters keyed by reason/stream" --
/// shared by both sides (this codebase's existing Prometheus-backed
/// `PlayerMetrics`/`NetworkRequestMetrics` are server-only
/// infrastructure the client has no equivalent of; this type is
/// deliberately independent of that, usable identically on either
/// side). Redacted BY CONSTRUCTION, not by discipline: the key is
/// exactly `(&'static str, SemanticStreamIdV1)` -- a fixed, bounded-
/// cardinality label pair that structurally cannot hold a payload
/// byte, a token, a chat string, or a session/principal identifier
/// (packet's own acceptance gate: "logs contain no token/chat/command/
/// payload bytes"; this type can't violate that even if misused, there
/// is no field to put one in).
#[derive(Default)]
pub struct SemanticIngressMetricsV1 {
    counts: std::sync::Mutex<std::collections::HashMap<(&'static str, SemanticStreamIdV1), u64>>,
}

impl SemanticIngressMetricsV1 {
    pub fn new() -> Self { Self::default() }

    pub fn record_reject(&self, reject: &SemanticEnvelopeRejectV1, stream: SemanticStreamIdV1) {
        self.record(reject.code(), stream);
    }

    pub fn record_terminal(&self, terminal: SemanticProtocolTerminalV1, stream: SemanticStreamIdV1) {
        self.record(terminal.code(), stream);
    }

    fn record(&self, code: &'static str, stream: SemanticStreamIdV1) {
        *self.counts.lock().expect("semantic ingress metrics mutex poisoned").entry((code, stream)).or_insert(0) += 1;
    }

    /// A point-in-time copy -- the "metrics snapshot" evidence artifact.
    /// Sorted for deterministic output (never iteration-order-dependent
    /// `HashMap` order).
    pub fn snapshot(&self) -> Vec<(&'static str, SemanticStreamIdV1, u64)> {
        let mut out: Vec<_> =
            self.counts.lock().expect("semantic ingress metrics mutex poisoned").iter().map(|(&(code, stream), &n)| (code, stream, n)).collect();
        out.sort_by_key(|&(code, stream, _)| (code, stream.as_u8()));
        out
    }
}

/// Packet section 7.10's own outcome classification for one evidence
/// entry -- reuses the two ALREADY-frozen terminal vocabularies
/// wholesale (`SemanticEnvelopeRejectV1` for per-frame rejects,
/// `SemanticProtocolTerminalV1` for connection-level terminals) instead
/// of inventing parallel variant names for the same concepts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticFrameVerdictV1 {
    Sent,
    Rejected(SemanticEnvelopeRejectV1),
    Terminal(SemanticProtocolTerminalV1),
}

/// Packet section 7.10. Added in `T3.3.15` -- described in the packet's
/// own shared-vocabulary section 7 alongside every other type in this
/// module, but never actually landed in `T3.3.01`'s "exact enums/structs
/// from Section 7" pass; `T3.3.15`'s own "records evidence" step
/// (algorithm step 9) is the first to actually need it, so it completes
/// `T3.3.01`'s contract here rather than blocking on going back to that
/// step. Same class of gap `T0.4.6` closed for `T0.4`.
///
/// "Do not record tokens, chat text, command arguments, or payload
/// bytes in ordinary logs" (packet's own words) -- this shape has no
/// field that could hold any of those; `payload_digest` is the only
/// payload-derived value, and a digest is not the payload.
#[derive(Clone, Copy, Debug)]
pub struct SemanticFrameEvidenceV1 {
    pub tick_observed: u64,
    pub direction: SemanticDirectionV1,
    pub stream: SemanticStreamIdV1,
    pub session_id: SessionId,
    pub connection_epoch: ConnectionEpoch,
    pub sequence: u64,
    pub payload_schema: SemanticPayloadSchemaV1,
    pub payload_digest: DigestBytes32V1,
    pub verdict: SemanticFrameVerdictV1,
}

/// Packet section 7.1/T3.3.03: encodes `payload` with the pinned
/// `SemanticPayloadEncodingV1::Bincode2LegacySerde` codec -- the exact
/// same `bincode::config::legacy()` pin `network/src/message.rs` already
/// uses for the outer frame, reused rather than reinvented so payload
/// bytes and outer-frame bytes never silently drift onto different
/// codecs.
pub fn encode_payload_v1<T: Serialize>(payload: &T) -> Vec<u8> {
    encode_to_vec(payload, legacy()).expect("bincode legacy serde encoding of an owned value is infallible")
}

/// `T3.3.03`: the payload-level twin of `T3.3.02`'s outer exact-consume
/// decode -- bincode decodes exactly one value and reports how many bytes
/// it consumed; a short consume (trailing bytes) is a distinct typed
/// rejection from a decode failure, never silently accepted.
pub fn decode_payload_exact_v1<T: DeserializeOwned>(payload_bytes: &[u8]) -> Result<T, SemanticEnvelopeRejectV1> {
    match decode_from_slice(payload_bytes, legacy()) {
        Ok((value, consumed)) if consumed == payload_bytes.len() => Ok(value),
        Ok(_) => Err(SemanticEnvelopeRejectV1::PayloadTrailingBytes),
        Err(_) => Err(SemanticEnvelopeRejectV1::PayloadDecodeFailure),
    }
}

/// Packet section 7.5/`T3.3.04`: one shared classification a sender and a
/// receiver both call, so `common/net/src/msg/client.rs::send_msg_err`'s
/// physical-stream match and `server/src/client.rs::prepare`'s physical-
/// stream match (packet section 3.4's named route-drift risk) have a
/// single semantic source of truth to be checked against, instead of each
/// independently deciding a variant's stream. This step does not migrate
/// either call site (packet's own "keep legacy matches until migrated");
/// it adds the registry and the exhaustive tests that prove it agrees
/// with both legacy matches today.
pub trait SemanticRouteV1 {
    fn semantic_stream(&self) -> SemanticStreamIdV1;
    fn payload_schema(&self) -> SemanticPayloadSchemaV1;
}

/// Mirrors `client/src/lib.rs::send_msg_err`'s physical-stream match
/// exactly (that match is itself exhaustive with no wildcard arm, so it
/// is the authoritative current variant-to-stream mapping this trait impl
/// must match; `T3.3.04`'s test suite checks this directly rather than
/// trusting a paraphrase).
impl SemanticRouteV1 for ClientGeneral {
    fn semantic_stream(&self) -> SemanticStreamIdV1 {
        use ClientGeneral as C;
        match self {
            C::RequestCharacterList
            | C::CreateCharacter { .. }
            | C::EditCharacter { .. }
            | C::DeleteCharacter(_)
            | C::Character(_, _)
            | C::Spectate(_) => SemanticStreamIdV1::CharacterScreen,
            C::ControllerInputs(_)
            | C::ControlEvent(_)
            | C::ControlAction(_)
            | C::SetViewDistance(_)
            | C::BreakBlock(_)
            | C::PlaceBlock(_, _)
            | C::ExitInGame
            | C::PlayerPhysics { .. }
            | C::UnlockSkill(_)
            | C::RequestSiteInfo(_)
            | C::RequestPlayerPhysics { .. }
            | C::RequestLossyTerrainCompression { .. }
            | C::UpdateMapMarker(_)
            | C::SpectatePosition(_)
            | C::SpectateEntity(_)
            | C::BastionCameraAnchor(_)
            | C::BastionPlaceDesignation { .. }
            | C::BastionApplyInfluence { .. }
            | C::BastionContextAction { .. }
            | C::BastionSpawnColony { .. }
            | C::BastionCancelDesignation { .. }
            | C::BastionInspect { .. }
            | C::SetBattleMode(_) => SemanticStreamIdV1::InGame,
            C::TerrainChunkRequest { .. } | C::LodZoneRequest { .. } => SemanticStreamIdV1::Terrain,
            // APEX MERGE: T2.5.10's RequestPluginArtifacts joins the
            // General stream, mirroring client/src/lib.rs's own physical
            // routing for it (the authoritative mapping this impl tracks).
            C::ChatMsg(_)
            | C::Command(_, _)
            | C::Terminate
            | C::RequestPlugins(_)
            | C::RequestPluginArtifacts(_)
            // T3.4.19: the commit ack rides the General stream, which is
            // never itself blocked by a checkpoint's own data fence.
            | C::CheckpointCommitAck(_) => SemanticStreamIdV1::General,
        }
    }

    fn payload_schema(&self) -> SemanticPayloadSchemaV1 { SemanticPayloadSchemaV1::ClientGeneral }
}

/// Mirrors `server/src/client.rs::prepare`'s physical-stream match
/// exactly (also exhaustive, no wildcard; the OTHER match in that file at
/// lines ~99-149 is dead code inside a `/* ... */` comment block, not a
/// second live route -- confirmed by reading `send`, which calls
/// `prepare`/`send_prepared`, never the commented-out block).
impl SemanticRouteV1 for ServerGeneral {
    fn semantic_stream(&self) -> SemanticStreamIdV1 {
        use ServerGeneral as S;
        match self {
            // T3.4.20b: a fence belongs to the stream it names, not to a
            // fixed one -- all five streams carry their own Begin/Barrier.
            S::CheckpointBegin(b) => b.begin.stream,
            // T3.5.13: results ride General, the canonical egress every
            // session has.
            S::CommandResult(_) => SemanticStreamIdV1::General,
            S::CheckpointBarrier(b) => b.stream,
            S::CharacterDataLoadResult(_)
            | S::CharacterListUpdate(_)
            | S::CharacterActionError(_)
            | S::CharacterCreated(_)
            | S::CharacterEdited(_)
            | S::CharacterSuccess
            | S::SpectatorSuccess(_) => SemanticStreamIdV1::CharacterScreen,
            S::GroupUpdate(_)
            | S::Invite { .. }
            | S::InvitePending(_)
            | S::InviteComplete { .. }
            | S::ExitInGameSuccess
            | S::InventoryUpdate(_, _)
            | S::GroupInventoryUpdate(_, _)
            | S::Dialogue(_, _)
            | S::SetViewDistance(_)
            | S::Outcomes(_)
            | S::Knockback(_)
            | S::SiteEconomy(_)
            | S::UpdatePendingTrade(_, _, _)
            | S::FinishedTrade(_)
            | S::MapMarker(_)
            | S::WeatherUpdate(..)
            | S::LocalWindUpdate(..)
            // APEX-T5.3: a receipt is data about a frame already sent,
            // classified with the rest of the in-game data stream.
            | S::InputReceipt(_)
            | S::SpectatePosition(_)
            | S::UpdateRecipes
            | S::Gizmos(_)
            | S::BastionDesignation { .. }
            | S::BastionDesignationRemoved { .. }
            | S::BastionInspectInfo { .. } => SemanticStreamIdV1::InGame,
            S::TerrainChunkUpdate { .. } | S::LodZoneUpdate { .. } | S::TerrainBlockUpdates(_) => {
                SemanticStreamIdV1::Terrain
            },
            S::PlayerListUpdate(_)
            | S::ChatMsg(_)
            | S::ChatMode(_)
            | S::SetPlayerEntity(_)
            | S::TimeOfDay(_, _, _, _)
            | S::EntitySync(_)
            | S::CompSync(_, _)
            | S::CreateEntity(_)
            | S::DeleteEntity(_)
            | S::Disconnect(_)
            | S::Notification(_)
            | S::SetPlayerRole(_)
            | S::PluginData(_)
            // APEX MERGE: T2.5.10's PluginArtifactData joins the General
            // stream, mirroring server/src/client.rs::prepare's own
            // physical routing for it.
            | S::PluginArtifactData(_) => SemanticStreamIdV1::General,
            // `T4.1` chunk 2a: rides the SAME stream as `GameSync`
            // (`SemanticRouteV1 for ServerInit` below) on purpose --
            // this row's whole point is a guaranteed pre-`GameSync`
            // ordering, and only same-stream sequencing can actually
            // guarantee that once this message is wired through the
            // V1-envelope pipeline (chunk 2b). Sent via the plain legacy
            // path today, so this classification is not yet load-bearing
            // -- it is the honest FUTURE-correct answer, not a guess.
            S::BootstrapManifest(_) => SemanticStreamIdV1::Bootstrap,
        }
    }

    fn payload_schema(&self) -> SemanticPayloadSchemaV1 { SemanticPayloadSchemaV1::ServerGeneral }
}

/// `ServerInit`'s single `GameSync` variant routes through the register
/// stream today (`ServerMsg::Init(m) => PreparedMsg::new(0, ..,
/// &self.register_stream_params)` in `server/src/client.rs::prepare`) --
/// the same physical stream `ServerMsg::Info`/`RegisterAnswer` use, which
/// this row's `SemanticStreamIdV1::Bootstrap` names.
impl SemanticRouteV1 for ServerInit {
    fn semantic_stream(&self) -> SemanticStreamIdV1 { SemanticStreamIdV1::Bootstrap }

    fn payload_schema(&self) -> SemanticPayloadSchemaV1 { SemanticPayloadSchemaV1::ServerInit }
}

/// `T3.3.17`: one payload schema's declared causality requirement --
/// whether a receiver must reject a frame of this schema for lacking
/// `producer_tick`/`snapshot`. Part of [`NetEnvelopeCausalityProfileV1`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CausalityRequirementV1 {
    pub payload_schema: SemanticPayloadSchemaV1,
    pub producer_tick_required: bool,
    pub snapshot_required: bool,
}

/// `T3.3.17`: "snapshot-domain profiles" -- an EXTENSION of the
/// existing `NET_ENVELOPE_PROFILE_V1` vocabulary (not new `T3.4`
/// semantics; `T3.4` still fully owns snapshot PRODUCTION/watermark
/// semantics under its own later profile root): the declared set of
/// [`SnapshotDomainId`]s a receiver should accept in `causality.snapshot`,
/// and each payload schema's causality requirement.
/// [`production_causality_profile_v1`] is the ONE frozen instance the
/// real live path ever uses (empty domain set, every schema fully
/// optional -- both `UnknownDomain` and `CausalityProfileMismatch` are
/// therefore unreachable on real traffic today, matching this row's own
/// "leave snapshot absent until a producer has defined epochs"); tests
/// construct their own instances directly to exercise both rejects and
/// the profile-immutability guard without touching the frozen
/// production values.
#[derive(Clone, Debug)]
pub struct NetEnvelopeCausalityProfileV1 {
    pub declared_domains: Vec<SnapshotDomainId>,
    pub requirements: Vec<CausalityRequirementV1>,
}

/// The frozen production instance -- encoded into
/// `net_envelope_profile_table_bytes_v1` below (categories 5/6), so
/// changing it without also changing the surrounding hashed table is
/// impossible: the table IS the profile, and any edit to either
/// category changes `net_envelope_profile_root_v1()`. Empty domain set,
/// every schema optional -- the mechanism is real and tested, but
/// nothing on real traffic can trip either of `T3.3.17`'s new rejects
/// today.
pub fn production_causality_profile_v1() -> NetEnvelopeCausalityProfileV1 {
    NetEnvelopeCausalityProfileV1 {
        declared_domains: Vec::new(),
        requirements: SemanticPayloadSchemaV1::ALL
            .map(|payload_schema| CausalityRequirementV1 { payload_schema, producer_tick_required: false, snapshot_required: false })
            .to_vec(),
    }
}

/// `T3.3.17`: checks one frame's causality against the active profile's
/// declared vocabulary/requirements -- pure, and independent of any
/// session-local monotonicity state (that's
/// `SemanticReceiveStateV1::snapshot_is_fresh`, a SEPARATE, session-
/// scoped check the caller applies afterward, never conflated with this
/// structural, profile-level one). Order: domain membership first (a
/// frame naming an undeclared domain is a profile violation, checked
/// before anything session-local could even apply), then the schema's
/// own requirement.
pub fn validate_causality_against_profile_v1(
    causality: &SemanticCausalityV1,
    payload_schema: SemanticPayloadSchemaV1,
    profile: &NetEnvelopeCausalityProfileV1,
) -> Result<(), SemanticEnvelopeRejectV1> {
    if let Some(snapshot) = &causality.snapshot {
        if !profile.declared_domains.contains(&snapshot.domain) {
            return Err(SemanticEnvelopeRejectV1::UnknownDomain);
        }
    }
    if let Some(requirement) = profile.requirements.iter().find(|r| r.payload_schema == payload_schema) {
        if requirement.producer_tick_required && causality.producer_tick.is_none() {
            return Err(SemanticEnvelopeRejectV1::CausalityProfileMismatch);
        }
        if requirement.snapshot_required && causality.snapshot.is_none() {
            return Err(SemanticEnvelopeRejectV1::CausalityProfileMismatch);
        }
    }
    Ok(())
}

fn encode_domain_category(buf: &mut Vec<u8>, category: u8, domains: &[SnapshotDomainId]) {
    buf.push(category);
    buf.extend_from_slice(&(domains.len() as u16).to_be_bytes());
    let mut sorted: Vec<u32> = domains.iter().map(|d| d.get()).collect();
    sorted.sort_unstable();
    for d in sorted {
        buf.extend_from_slice(&d.to_be_bytes());
    }
}

fn encode_causality_requirement_category(buf: &mut Vec<u8>, category: u8, requirements: &[CausalityRequirementV1]) {
    buf.push(category);
    buf.extend_from_slice(&(requirements.len() as u16).to_be_bytes());
    let mut sorted = requirements.to_vec();
    sorted.sort_by_key(|r| r.payload_schema.as_u16());
    for r in sorted {
        buf.extend_from_slice(&r.payload_schema.as_u16().to_be_bytes());
        buf.push(r.producer_tick_required as u8);
        buf.push(r.snapshot_required as u8);
    }
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
    // `T3.3.17`: categories 5/6 are the "snapshot-domain profiles" --
    // the production `NetEnvelopeCausalityProfileV1` is part of this
    // same frozen table, so changing it (declaring a domain, requiring
    // a field) is impossible without also changing `profile_root`.
    let causality_profile = production_causality_profile_v1();
    encode_domain_category(&mut buf, 5, &causality_profile.declared_domains);
    encode_causality_requirement_category(&mut buf, 6, &causality_profile.requirements);
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

    use common::apex::manifest::{ManifestDecodeLimitsV1, decode_manifest_v1, encode_manifest_v1};

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
        let check = |labels: Vec<&str>| {
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
            // Recomputed at the APEX merge: `DigestDomainIdV1::NetEnvelopeProfile`
            // moved 12 -> 20 to settle a cross-lane collision, and the domain
            // id is part of the canonical preimage, so the root moved with it.
            //
            // Recomputed again for APEX-T5.2, the T5 tier's single wire bump:
            // both payload-schema labels moved v1 -> v2 because both payload
            // schemas changed (PlayerPhysics gained a weather-snapshot
            // reference; the weather messages gained the snapshot id they
            // belong to; ServerGeneral::InputReceipt is new). One recompute,
            // one reason, spent once for the whole tier.
            "sha256:0f1bb6139b9c18cd286991d43528daa3252297c07d1e6f52347a01883e551746",
            "NET_ENVELOPE_PROFILE_V1 table changed -- recompute and update this golden vector deliberately, \
             it must never drift silently"
        );
    }

    /// `APEX-T5.2` finding, recorded as a test because a comment would
    /// not survive the next author.
    ///
    /// The frozen table digests the TAG VOCABULARY. It does NOT digest
    /// the payload schemas' CONTENTS — `ClientGeneral` and
    /// `ServerGeneral` are opaque to it. So adding a field to a message,
    /// or a variant to an enum, does not move `profile_root` by itself.
    /// The payload-schema LABEL is the only place a payload version is
    /// recorded, which makes bumping it the whole of the wire-version
    /// mechanism — and nothing forces a future author to remember.
    ///
    /// This test pins the labels so that a payload change made WITHOUT a
    /// label bump still leaves this assertion describing v2 while the
    /// messages have moved on. It narrows the gap; it does not close it,
    /// and saying so is the point.
    #[test]
    fn payload_schema_labels_carry_the_wire_version() {
        assert!(
            SemanticPayloadSchemaV1::ClientGeneral.label().ends_with("/v2"),
            "ClientGeneral's payload schema label is the wire version for every client message;              if the payload changed, this label must change with it"
        );
        assert!(
            SemanticPayloadSchemaV1::ServerGeneral.label().ends_with("/v2"),
            "ServerGeneral's payload schema label is the wire version for every server message"
        );
        // ServerInit did not change in T5.2 and must not be bumped along
        // for the ride — a version that moves without a reason teaches
        // readers that versions are noise.
        assert!(SemanticPayloadSchemaV1::ServerInit.label().ends_with("/v1"));
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

    // T3.3.03's own test list: one-bit mutation, schema substitution,
    // length ambiguity, compression independence, float bit preservation,
    // unordered-map byte drift.

    #[test]
    fn payload_round_trips_exactly() {
        let bytes = encode_payload_v1(&("abc".to_string(), 42u32, -7i64));
        let decoded: (String, u32, i64) = decode_payload_exact_v1(&bytes).unwrap();
        assert_eq!(decoded, ("abc".to_string(), 42u32, -7i64));
    }

    /// One-bit mutation: flipping a single payload bit must change the
    /// digest (never leave it, or the whole point of the digest is moot).
    #[test]
    fn one_bit_payload_mutation_changes_digest() {
        let root = net_envelope_profile_root_v1();
        let bytes = encode_payload_v1(&"abc".to_string());
        let a = payload_digest_v1(root, SemanticPayloadSchemaV1::ClientGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, &bytes);
        let mut mutated = bytes.clone();
        let last = mutated.len() - 1;
        mutated[last] ^= 0x01;
        let b = payload_digest_v1(root, SemanticPayloadSchemaV1::ClientGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, &mutated);
        assert_ne!(a.as_array(), b.as_array());
    }

    /// Schema substitution: identical bytes under a different declared
    /// `SemanticPayloadSchemaV1` must digest differently -- a receiver
    /// cannot be tricked into accepting bytes meant for one payload enum
    /// as if they were another just because the raw bytes happen to match.
    #[test]
    fn schema_substitution_with_identical_bytes_changes_digest() {
        let root = net_envelope_profile_root_v1();
        let bytes = encode_payload_v1(&"abc".to_string());
        let as_client = payload_digest_v1(root, SemanticPayloadSchemaV1::ClientGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, &bytes);
        let as_server = payload_digest_v1(root, SemanticPayloadSchemaV1::ServerGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, &bytes);
        assert_ne!(as_client.as_array(), as_server.as_array());
    }

    /// Length ambiguity: `decode_payload_exact_v1` is the payload-level
    /// twin of `T3.3.02`'s outer exact-consume decode -- short or long
    /// buffers around a valid encoding must be rejected, not silently
    /// truncated/padded.
    #[test]
    fn payload_length_ambiguity_is_rejected() {
        let bytes = encode_payload_v1(&"abc".to_string());

        let mut with_trailing = bytes.clone();
        with_trailing.push(0xFF);
        assert_eq!(decode_payload_exact_v1::<String>(&with_trailing), Err(SemanticEnvelopeRejectV1::PayloadTrailingBytes));

        let truncated = &bytes[..bytes.len() - 1];
        assert_eq!(decode_payload_exact_v1::<String>(truncated), Err(SemanticEnvelopeRejectV1::PayloadDecodeFailure));
    }

    /// Compression independence: `encode_payload_v1`/`payload_digest_v1`
    /// never compress -- calling either twice on the same input is
    /// byte-for-byte deterministic regardless of what a later transport
    /// stage does to the bytes afterward.
    #[test]
    fn encoding_and_digest_are_compression_independent_and_deterministic() {
        let payload = vec![0u8; 10_000]; // Highly compressible, deliberately.
        let a = encode_payload_v1(&payload);
        let b = encode_payload_v1(&payload);
        assert_eq!(a, b);
        let root = net_envelope_profile_root_v1();
        let digest_a = payload_digest_v1(root, SemanticPayloadSchemaV1::ClientGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, &a);
        let digest_b = payload_digest_v1(root, SemanticPayloadSchemaV1::ClientGeneral, SemanticPayloadEncodingV1::Bincode2LegacySerde, &b);
        assert_eq!(digest_a.as_array(), digest_b.as_array());
    }

    /// Float bit preservation: the pinned legacy Bincode/Serde codec must
    /// round-trip raw IEEE-754 bits exactly, including a NaN payload whose
    /// bit pattern would NOT compare equal under plain `==` -- this test
    /// compares `to_bits()`, not the float values themselves, so it would
    /// fail honestly rather than passing by accident on a NaN that never
    /// actually got checked.
    #[test]
    fn float_bit_patterns_round_trip_exactly() {
        let values: [f64; 4] = [0.0, -0.0, f64::NAN, f64::INFINITY];
        for v in values {
            let bytes = encode_payload_v1(&v);
            let decoded: f64 = decode_payload_exact_v1(&bytes).unwrap();
            assert_eq!(decoded.to_bits(), v.to_bits(), "bit pattern mismatch for {v}");
        }
    }

    /// Unordered-map byte drift: this is a NEGATIVE test proving packet
    /// section 5.5's own disclaimer is real, not just written down --
    /// payload byte identity does NOT prove semantic equivalence for a
    /// `HashMap`-backed payload, because std `HashMap` iteration order is
    /// not canonicalized by this codec. Two structurally-identical maps
    /// CAN encode to different bytes; this module makes no claim
    /// otherwise, and this test is the falsifiable proof of that.
    #[test]
    fn unordered_map_payloads_are_not_byte_stable() {
        use std::collections::HashMap;
        // A HashMap large enough, with keys spread across enough hash
        // buckets, that RandomState's per-process random seed makes
        // insertion/iteration order vary run-to-run in practice -- the
        // same non-determinism axis packet section 5.5 disclaims.
        let mut map: HashMap<u32, u32> = HashMap::new();
        for i in 0..64u32 {
            map.insert(i, i * 7);
        }
        let a = encode_payload_v1(&map);
        // Same logical content, independently rebuilt -- a fresh HashMap
        // instance gets its own random iteration order.
        let mut map2: HashMap<u32, u32> = HashMap::new();
        for i in 0..64u32 {
            map2.insert(i, i * 7);
        }
        let b = encode_payload_v1(&map2);
        // Two independently-seeded HashMaps with 64 entries have a
        // negligible (not zero, but astronomically small) chance of
        // sharing an iteration order -- this is the actual "byte drift"
        // the packet's test name asks for, demonstrated rather than
        // merely asserted in a doc comment.
        assert_ne!(a, b, "two independently-built HashMaps coincidentally encoded identically (extremely unlikely -- rerun to confirm before treating as a real regression)");
        // The invariant this module actually guarantees, despite the byte
        // drift above: semantically-equal content still decodes equal.
        let decoded_a: HashMap<u32, u32> = decode_payload_exact_v1(&a).unwrap();
        let decoded_b: HashMap<u32, u32> = decode_payload_exact_v1(&b).unwrap();
        assert_eq!(decoded_a, decoded_b, "semantically equal maps must still decode equal regardless of byte order");
    }

    // T3.3.04's own test list: every variant on correct and every wrong
    // stream, plus a new-variant compile/test canary. The exhaustive
    // `match` (no wildcard arm) in each `SemanticRouteV1` impl above is
    // itself the compile-time half of that canary -- it already failed to
    // compile once during this step until every current variant was
    // covered, and will fail again the moment a new variant is added to
    // `ClientGeneral`/`ServerGeneral`/`ServerInit` without being routed
    // here. These tests are the runtime half: representative variants
    // from every one of the four semantic streams, confirming
    // `semantic_stream()` lands on the RIGHT one and therefore (since it
    // is a total function returning exactly one value) implicitly not any
    // of the three wrong ones.

    #[test]
    fn client_general_route_covers_every_stream() {
        use vek::Vec2;
        assert_eq!(ClientGeneral::RequestCharacterList.semantic_stream(), SemanticStreamIdV1::CharacterScreen);
        assert_eq!(ClientGeneral::ExitInGame.semantic_stream(), SemanticStreamIdV1::InGame);
        assert_eq!(ClientGeneral::TerrainChunkRequest { key: Vec2::new(0, 0) }.semantic_stream(), SemanticStreamIdV1::Terrain);
        assert_eq!(ClientGeneral::Terminate.semantic_stream(), SemanticStreamIdV1::General);
        assert_eq!(ClientGeneral::Terminate.payload_schema(), SemanticPayloadSchemaV1::ClientGeneral);
    }

    #[test]
    fn server_general_route_covers_every_stream() {
        assert_eq!(ServerGeneral::CharacterSuccess.semantic_stream(), SemanticStreamIdV1::CharacterScreen);
        assert_eq!(ServerGeneral::ExitInGameSuccess.semantic_stream(), SemanticStreamIdV1::InGame);
        assert_eq!(ServerGeneral::UpdateRecipes.semantic_stream(), SemanticStreamIdV1::InGame);
        assert_eq!(
            ServerGeneral::Disconnect(crate::msg::server::DisconnectReason::Shutdown).semantic_stream(),
            SemanticStreamIdV1::General
        );
        assert_eq!(ServerGeneral::CharacterSuccess.payload_schema(), SemanticPayloadSchemaV1::ServerGeneral);
    }

    /// `ServerInit` has one variant (`GameSync`), so its
    /// `SemanticRouteV1::semantic_stream` impl is a constant function of
    /// the type, never inspecting `&self` -- matching
    /// `server/src/client.rs::prepare`'s `ServerMsg::Init` arm, which
    /// also routes unconditionally to the register stream (this row's
    /// `Bootstrap` tag). Constructing a full `GameSync` (entity package,
    /// world map, recipe books, ...) just to call a method that ignores
    /// its argument would test nothing this doesn't already prove;
    /// `cargo check` compiling the impl's `SemanticStreamIdV1::Bootstrap`
    /// body is the actual proof here, not a runtime assertion.
    const _: fn(&ServerInit) -> SemanticStreamIdV1 = <ServerInit as SemanticRouteV1>::semantic_stream;

    // T3.3.05: SemanticProtocolIdV1 tag registry + the server's fixed
    // advertised set.

    #[test]
    fn semantic_protocol_tags_are_unique_and_explicit() {
        let ids: HashSet<u8> = SemanticProtocolIdV1::ALL.iter().map(|p| p.as_u8()).collect();
        assert_eq!(ids.len(), SemanticProtocolIdV1::ALL.len());
        assert_eq!(SemanticProtocolIdV1::Legacy.as_u8(), 1);
        assert_eq!(SemanticProtocolIdV1::NetEnvelopeV1.as_u8(), 2);
        for p in SemanticProtocolIdV1::ALL {
            assert_eq!(SemanticProtocolIdV1::try_from_u8(p.as_u8()), Some(p));
        }
        assert_eq!(SemanticProtocolIdV1::try_from_u8(0), None);
        assert_eq!(SemanticProtocolIdV1::try_from_u8(3), None);
    }

    #[test]
    fn supported_protocols_are_sorted_and_include_legacy() {
        let supported = server_supported_semantic_protocols_v1();
        let mut sorted = supported.clone();
        sorted.sort();
        assert_eq!(supported, sorted, "advertised set must already be sorted ascending");
        // Row status doc requirement 2 (before/after wire-compat delta):
        // Legacy must stay advertised until a real certified-mode config
        // surface exists (T4.1), or the live client (which always
        // requests Legacy today) would start failing to register.
        assert!(supported.contains(&SemanticProtocolIdV1::Legacy));
    }

    // T3.3.06's own test list: initial state, epoch reset, old state
    // inaccessible, max sequence.

    fn test_binding(epoch: u64) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut common::apex::identity::FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut common::apex::identity::FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(epoch).unwrap(),
        }
    }

    #[test]
    fn initial_state_starts_every_stream_at_one() {
        let binding = test_binding(1);
        let send = SemanticSendStateV1::new(binding);
        let recv = SemanticReceiveStateV1::new(binding);
        for stream in SemanticStreamIdV1::ALL {
            assert_eq!(send.next_for(stream).get(), 1);
            assert_eq!(recv.next_expected_for(stream).get(), 1);
        }
        assert_eq!(send.binding(), binding);
        assert_eq!(recv.binding(), binding);
        assert!(recv.highest_snapshot.is_empty());
        assert!(recv.terminal.is_none());
    }

    /// "Higher epoch replaces": a fresh reset at a higher epoch is a
    /// full, independent state -- never derived from or merged with
    /// whatever the previous epoch's state happened to hold.
    #[test]
    fn epoch_reset_produces_independent_fresh_state() {
        let epoch1_binding = test_binding(1);
        let epoch1_state = SemanticSendStateV1::new(epoch1_binding);

        let epoch2_binding = test_binding(2);
        let epoch2_state = SemanticSendStateV1::new(epoch2_binding);

        assert_ne!(epoch1_state.binding().epoch, epoch2_state.binding().epoch);
        // Both start at 1 regardless of epoch value -- "per-attachment"
        // (this row's own title), not a running total across epochs.
        for stream in SemanticStreamIdV1::ALL {
            assert_eq!(epoch1_state.next_for(stream), epoch2_state.next_for(stream));
        }
    }

    // T3.3.07's own test list: independent cursors, max (exhaustion).
    // "All routes" is T3.3.04's own SemanticRouteV1 coverage, reused
    // rather than re-proven here. "Send failure"/"mixed mode" are
    // structural guarantees of this row's code shape (send_semantic_v1
    // mutates the cursor before any encode/send is attempted, so a
    // failed send can never retry the same sequence; send_msg_err's V1
    // check is a single is_some() branch, so one Client instance can
    // never send SOME messages via Legacy and others via V1) rather than
    // independently unit-tested against a full mock Client -- matching
    // T3.3.06's own established scope (test the state types directly,
    // not a full live Client fixture, for code nothing live reaches yet).

    #[test]
    fn allocate_sequence_advances_each_stream_independently() {
        let mut state = SemanticSendStateV1::new(test_binding(1));
        let first_ingame = state.allocate_sequence(SemanticStreamIdV1::InGame).unwrap();
        assert_eq!(first_ingame.get(), 1);
        // A different stream's cursor is untouched by the InGame allocation above.
        assert_eq!(state.next_for(SemanticStreamIdV1::General).get(), 1);
        assert_eq!(state.next_for(SemanticStreamIdV1::CharacterScreen).get(), 1);

        let second_ingame = state.allocate_sequence(SemanticStreamIdV1::InGame).unwrap();
        assert_eq!(second_ingame.get(), 2);
        // Still untouched.
        assert_eq!(state.next_for(SemanticStreamIdV1::General).get(), 1);

        let first_general = state.allocate_sequence(SemanticStreamIdV1::General).unwrap();
        assert_eq!(first_general.get(), 1);
        // InGame's cursor is unaffected by General's own allocation.
        assert_eq!(state.next_for(SemanticStreamIdV1::InGame).get(), 3);
    }

    #[test]
    fn reserve_sequences_is_all_or_nothing_across_streams() {
        let mut state = SemanticSendStateV1::new(test_binding(1));
        let first = state.reserve_sequences_v1([2, 2, 4, 2, 3]).unwrap();
        assert_eq!(first.map(|f| f.get()), [1; 5]);
        assert_eq!(state.next_for(SemanticStreamIdV1::InGame).get(), 5);
        assert_eq!(state.next_for(SemanticStreamIdV1::Terrain).get(), 4);
        // A second reservation starts exactly where the first ended --
        // reserved ranges never overlap.
        let second = state.reserve_sequences_v1([1, 0, 1, 0, 0]).unwrap();
        assert_eq!(second[2].get(), 5);
        // Zero reserves nothing.
        assert_eq!(second[1].get(), 3);
        assert_eq!(state.next_for(SemanticStreamIdV1::CharacterScreen).get(), 3);

        // One exhausting stream aborts the WHOLE reservation: a
        // sequential implementation would have advanced Bootstrap first.
        let max = NonZeroU64::new(u64::MAX).unwrap();
        let mut edge = SemanticSendStateV1::with_cursors_for_test(test_binding(1), [FIRST_SEQUENCE, FIRST_SEQUENCE, max, FIRST_SEQUENCE, FIRST_SEQUENCE]);
        assert_eq!(
            edge.reserve_sequences_v1([2, 2, 2, 2, 2]).unwrap_err(),
            common::apex::identity::CounterAdvanceErrorV1::Exhausted
        );
        for stream in [SemanticStreamIdV1::Bootstrap, SemanticStreamIdV1::CharacterScreen, SemanticStreamIdV1::General, SemanticStreamIdV1::Terrain] {
            assert_eq!(edge.next_for(stream).get(), 1, "no cursor may move when the reservation fails");
        }
        assert_eq!(edge.next_for(SemanticStreamIdV1::InGame), max);
    }

    #[test]
    fn allocate_sequence_exhausts_at_u64_max() {
        let max = NonZeroU64::new(u64::MAX).unwrap();
        let mut state = SemanticSendStateV1::with_cursors_for_test(test_binding(1), [max; 5]);

        // Exhaustion is per-stream: General is at MAX and fails...
        assert_eq!(
            state.allocate_sequence(SemanticStreamIdV1::General).unwrap_err(),
            common::apex::identity::CounterAdvanceErrorV1::Exhausted
        );
        // ...never panics, and never silently wraps back to a reused
        // value (the cursor stays at MAX, not 0 or 1).
        assert_eq!(state.next_for(SemanticStreamIdV1::General), max);

        // ...and every other stream, ALSO at MAX, independently fails
        // too -- exhaustion isn't accidentally scoped to just the one
        // stream tested above.
        for stream in SemanticStreamIdV1::ALL {
            assert!(state.allocate_sequence(stream).is_err());
        }
    }

    // T3.3.08: SemanticReceiveStateV1::advance_expected -- receive-side
    // twin of allocate_sequence's own independent-cursors/exhaustion tests.

    #[test]
    fn advance_expected_advances_each_stream_independently() {
        let mut state = SemanticReceiveStateV1::new(test_binding(1));
        assert_eq!(state.next_expected_for(SemanticStreamIdV1::InGame).get(), 1);
        state.advance_expected(SemanticStreamIdV1::InGame).unwrap();
        assert_eq!(state.next_expected_for(SemanticStreamIdV1::InGame).get(), 2);
        // Other streams untouched.
        assert_eq!(state.next_expected_for(SemanticStreamIdV1::General).get(), 1);
        assert_eq!(state.next_expected_for(SemanticStreamIdV1::Terrain).get(), 1);
    }

    #[test]
    fn advance_expected_exhausts_at_u64_max() {
        let max = NonZeroU64::new(u64::MAX).unwrap();
        let mut state = SemanticReceiveStateV1::with_cursors_for_test(test_binding(1), [max; 5]);
        assert_eq!(
            state.advance_expected(SemanticStreamIdV1::General).unwrap_err(),
            common::apex::identity::CounterAdvanceErrorV1::Exhausted
        );
        // Never silently wraps.
        assert_eq!(state.next_expected_for(SemanticStreamIdV1::General), max);
    }

    /// "Old state inaccessible": nothing about `SemanticSendStateV1`
    /// exposes a way to recover a PRIOR reset's state from a NEW one --
    /// the type itself has no history, only the one `binding` it was
    /// constructed with. This is a structural proof (the type has no such
    /// field/method), not a runtime one.
    #[test]
    fn state_carries_no_history_beyond_its_own_binding() {
        let a = SemanticSendStateV1::new(test_binding(1));
        let b = SemanticSendStateV1::new(test_binding(2));
        // Overwriting `a` with `b` (the live wiring's actual `Option<..> =
        // Some(new_state)` pattern) drops `a` entirely -- there is no
        // shared storage between two `SemanticSendStateV1` values a
        // caller could accidentally read stale data through.
        let mut slot = Some(a);
        assert_eq!(slot.as_ref().unwrap().binding().epoch, a.binding().epoch);
        slot = Some(b);
        assert_eq!(slot.unwrap().binding().epoch, b.binding().epoch);
    }

    /// "Max sequence": `NonZeroU64` (not `u64`) is the cursor type
    /// specifically so `u64::MAX` is a representable, valid cursor value
    /// -- exhaustion is a real, checkable condition (`T3.3.07`+'s
    /// `SequenceExhausted` terminal), not a type-level impossibility this
    /// row would need extra machinery to detect.
    #[test]
    fn cursor_type_can_represent_max_sequence() {
        let max = NonZeroU64::new(u64::MAX).expect("u64::MAX is nonzero");
        assert_eq!(max.get(), u64::MAX);
        assert_eq!(FIRST_SEQUENCE.get(), 1);
    }

    #[test]
    fn stream_index_is_injective_over_all_five_streams() {
        let indices: HashSet<usize> = SemanticStreamIdV1::ALL.iter().map(|s| stream_index(*s)).collect();
        assert_eq!(indices.len(), SemanticStreamIdV1::ALL.len());
        for i in &indices {
            assert!(*i < 5);
        }
    }

    // T3.3.07: `NetEnvelopeHeaderV1`/`SemanticWireFrameV1`'s canonical
    // `BastionManifestEncodingV1` (T0.2) encoding -- packet section 7.3's
    // "encoded with the already-required deterministic T0.2 codec"
    // (`T0.4.6`'s tagged opaque-identity codec is what makes this
    // possible; those fields round-trip through their own, separately
    // tested, manifest codec).

    fn wide_limits() -> ManifestDecodeLimitsV1 {
        ManifestDecodeLimitsV1 {
            max_input_bytes: 4096,
            max_depth: 8,
            max_nodes: 64,
            max_array_items: 16,
            max_map_entries: 16,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 256,
        }
    }

    fn sample_header() -> NetEnvelopeHeaderV1 {
        use common::apex::identity::FixedRandomBytesSourceV1;
        NetEnvelopeHeaderV1 {
            profile_root: net_envelope_profile_root_v1(),
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([0x11; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([0x22; 16])).unwrap(),
            connection_epoch: ConnectionEpoch::new(7).unwrap(),
            direction: SemanticDirectionV1::ClientToServer,
            semantic_stream: SemanticStreamIdV1::General,
            sequence: NonZeroU64::new(3).unwrap(),
            causality: SemanticCausalityV1 { producer_tick: Some(42), snapshot: None },
            payload_schema: SemanticPayloadSchemaV1::ClientGeneral,
            payload_encoding: SemanticPayloadEncodingV1::Bincode2LegacySerde,
            payload_len: 5,
            payload_digest: payload_digest_v1(
                net_envelope_profile_root_v1(),
                SemanticPayloadSchemaV1::ClientGeneral,
                SemanticPayloadEncodingV1::Bincode2LegacySerde,
                b"hello",
            ),
            command_id: None,
            checkpoint: None,
        }
    }

    /// `T3.4.20`: the checkpoint context survives the canonical codec
    /// intact, and absent stays absent — an unfenced frame must never
    /// decode into a checkpointed one.
    #[test]
    fn checkpoint_context_round_trips_and_absence_is_preserved() {
        use super::super::checkpoint::{CheckpointOrdinalV1, CheckpointedEnvelopeContextV1};

        let plain = sample_header();
        let bytes = encode_manifest_v1(&plain, &wide_limits()).unwrap();
        let decoded: NetEnvelopeHeaderV1 = decode_manifest_v1(&bytes, &wide_limits()).unwrap();
        assert_eq!(decoded.checkpoint, None);

        for ordinal in [None, Some(CheckpointOrdinalV1(9))] {
            let mut header = sample_header();
            header.checkpoint =
                Some(CheckpointedEnvelopeContextV1 { epoch: 4, ordinal, descriptor_root: [0xA5; 32] });
            let bytes = encode_manifest_v1(&header, &wide_limits()).unwrap();
            let decoded: NetEnvelopeHeaderV1 = decode_manifest_v1(&bytes, &wide_limits()).unwrap();
            assert_eq!(decoded, header);
            assert_eq!(decoded.checkpoint.unwrap().ordinal, ordinal);
            // a fenced frame is not byte-identical to an unfenced one
            assert_ne!(bytes, encode_manifest_v1(&plain, &wide_limits()).unwrap());
        }
    }

    #[test]
    fn header_round_trips_through_canonical_encoding() {
        let header = sample_header();
        let bytes = encode_manifest_v1(&header, &wide_limits()).unwrap();
        let decoded: NetEnvelopeHeaderV1 = decode_manifest_v1(&bytes, &wide_limits()).unwrap();
        assert_eq!(decoded, header);
    }

    #[test]
    fn frame_round_trips_through_canonical_encoding() {
        let frame = SemanticWireFrameV1 { header: sample_header(), payload_bytes: b"hello".to_vec() };
        let bytes = encode_manifest_v1(&frame, &wide_limits()).unwrap();
        let decoded: SemanticWireFrameV1 = decode_manifest_v1(&bytes, &wide_limits()).unwrap();
        assert_eq!(decoded, frame);
    }

    /// `command_id: Some(_)` round-trips too -- the optional 13th field is
    /// exercised in both directions, not just its absence.
    #[test]
    fn header_with_some_command_id_round_trips() {
        use common::apex::identity::FixedRandomBytesSourceV1;
        let mut header = sample_header();
        header.command_id = Some(CommandId::generate(&mut FixedRandomBytesSourceV1([0x33; 16])).unwrap());
        let bytes = encode_manifest_v1(&header, &wide_limits()).unwrap();
        let decoded: NetEnvelopeHeaderV1 = decode_manifest_v1(&bytes, &wide_limits()).unwrap();
        assert_eq!(decoded, header);
    }

    /// `causality.snapshot: Some(_)` round-trips too.
    #[test]
    fn header_with_snapshot_causality_round_trips() {
        let mut header = sample_header();
        header.causality.snapshot =
            Some(SemanticSnapshotRefV1 { domain: SnapshotDomainId::new(9), epoch: SnapshotEpoch::new(3) });
        let bytes = encode_manifest_v1(&header, &wide_limits()).unwrap();
        let decoded: NetEnvelopeHeaderV1 = decode_manifest_v1(&bytes, &wide_limits()).unwrap();
        assert_eq!(decoded, header);
    }

    /// Every enum tag field (`direction`/`semantic_stream`/`payload_schema`/
    /// `payload_encoding`) rejects an out-of-range value rather than
    /// silently accepting or panicking. Targeted at the four tag fields
    /// specifically -- NOT a blind whole-blob byte-flip: `profile_root`
    /// and `payload_digest` are opaque 32-byte hashes with no internal
    /// validity structure, so ANY byte value is legitimately valid for
    /// them, and a byte-flip test across the whole encoding would
    /// (correctly) find "different but still valid" headers there and
    /// produce a false failure -- confirmed by actually hitting exactly
    /// that false positive before narrowing this test's scope.
    #[test]
    fn unknown_tag_values_are_rejected_not_defaulted() {
        fn with_field(base: &NetEnvelopeHeaderV1, field: u16, value: ManifestValueV1) -> Vec<u8> {
            let ManifestValueV1::Map(map) = base.to_manifest_value_v1().unwrap() else { panic!("expected a map") };
            let new_entries: Vec<_> = map
                .into_entries()
                .into_iter()
                .map(|(id, v)| if id == FieldIdV1::new(field) { (id, value.clone()) } else { (id, v) })
                .collect();
            let mutated = CanonicalFieldMapV1::try_from_entries(new_entries).unwrap();
            encode_manifest_v1(&RawWrapper(ManifestValueV1::Map(mutated)), &wide_limits()).unwrap()
        }

        let header = sample_header();
        // direction (field 5): 0 and 3 are both outside {1, 2}.
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&with_field(&header, 5, ManifestValueV1::Unsigned(0)), &wide_limits()).is_err());
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&with_field(&header, 5, ManifestValueV1::Unsigned(3)), &wide_limits()).is_err());
        // semantic_stream (field 6): 0 and 6 are both outside {1..=5}.
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&with_field(&header, 6, ManifestValueV1::Unsigned(0)), &wide_limits()).is_err());
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&with_field(&header, 6, ManifestValueV1::Unsigned(6)), &wide_limits()).is_err());
        // payload_schema (field 9): 0 and 4 are both outside {1, 2, 3}.
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&with_field(&header, 9, ManifestValueV1::Unsigned(0)), &wide_limits()).is_err());
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&with_field(&header, 9, ManifestValueV1::Unsigned(4)), &wide_limits()).is_err());
        // payload_encoding (field 10): 0 and 2 are both outside {1}.
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&with_field(&header, 10, ManifestValueV1::Unsigned(0)), &wide_limits()).is_err());
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&with_field(&header, 10, ManifestValueV1::Unsigned(2)), &wide_limits()).is_err());

        // Sanity: the untouched header still decodes fine (proves
        // with_field's own round-trip plumbing is correct, not just that
        // every mutation happens to fail for an unrelated reason).
        let good = encode_manifest_v1(&header, &wide_limits()).unwrap();
        assert_eq!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&good, &wide_limits()).unwrap(), header);
    }

    /// Zero sequence is rejected -- the manifest path's own twin of
    /// `NonZeroU64`'s type-level guarantee (packet's `SEQUENCE-ZERO`
    /// terminal exists precisely because the wire form has no type-system
    /// help; this decoder must supply the check by hand).
    #[test]
    fn zero_sequence_is_rejected() {
        // Build a header value tree directly (bypassing NetEnvelopeHeaderV1's
        // own always-valid constructor) with sequence forced to 0, proving
        // the DECODER checks this, not just relying on callers never
        // constructing a zero sequence.
        let header = sample_header();
        let value = header.to_manifest_value_v1().unwrap();
        let ManifestValueV1::Map(map) = value else { panic!("expected a map") };
        let new_entries: Vec<_> = map
            .into_entries()
            .into_iter()
            .map(|(id, v)| if id == FieldIdV1::new(7) { (id, ManifestValueV1::Unsigned(0)) } else { (id, v) })
            .collect();
        let mutated_map = CanonicalFieldMapV1::try_from_entries(new_entries).unwrap();
        let mutated_value = RawWrapper(ManifestValueV1::Map(mutated_map));
        let bytes = encode_manifest_v1(&mutated_value, &wide_limits()).unwrap();
        assert!(decode_manifest_v1::<NetEnvelopeHeaderV1>(&bytes, &wide_limits()).is_err());
    }

    /// Test-only pass-through wrapper (same pattern used throughout
    /// `common/src/apex/`): lets a hostile test build an arbitrary
    /// manifest value tree without going through the real checked
    /// constructor.
    struct RawWrapper(ManifestValueV1);
    impl ManifestEncodeV1 for RawWrapper {
        fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(self.0.clone()) }
    }

    /// `APEX-T3.3.17` tests (`cargo test -p veloren-common-net
    /// envelope::snapshot_monotonicity`). Packet's own test list:
    /// "Absent/equal/increasing/decreasing/unrelated domain;
    /// cross-stream reordering remains nonclosure."
    fn snapshot_state() -> SemanticReceiveStateV1 {
        SemanticReceiveStateV1::new(ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut common::apex::identity::FixedRandomBytesSourceV1([31; 16])).unwrap(),
            session_id: SessionId::generate(&mut common::apex::identity::FixedRandomBytesSourceV1([32; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        })
    }

    fn snap(domain: u32, epoch: u64) -> SemanticSnapshotRefV1 {
        SemanticSnapshotRefV1 { domain: SnapshotDomainId::new(domain), epoch: SnapshotEpoch::new(epoch) }
    }

    #[test]
    fn snapshot_monotonicity_absent_domain_always_fresh() {
        // "Absent": no producer has ever populated `snapshot` for this
        // domain (or at all) -- a domain seen for the first time has
        // nothing to compare against, so it always passes.
        let state = snapshot_state();
        assert!(state.snapshot_is_fresh(&snap(1, 0)));
        assert!(state.snapshot_is_fresh(&snap(1, u64::MAX)));
    }

    #[test]
    fn snapshot_monotonicity_equal_is_fresh_not_stale() {
        // "Equal": the packet says "lower ... reject", not "equal ...
        // reject" -- non-decreasing, not strictly-increasing.
        let mut state = snapshot_state();
        state.commit_snapshot(snap(1, 5));
        assert!(state.snapshot_is_fresh(&snap(1, 5)));
    }

    #[test]
    fn snapshot_monotonicity_increasing_is_fresh() {
        let mut state = snapshot_state();
        state.commit_snapshot(snap(1, 5));
        assert!(state.snapshot_is_fresh(&snap(1, 6)));
    }

    #[test]
    fn snapshot_monotonicity_decreasing_is_stale() {
        let mut state = snapshot_state();
        state.commit_snapshot(snap(1, 5));
        assert!(!state.snapshot_is_fresh(&snap(1, 4)));
    }

    #[test]
    fn snapshot_monotonicity_unrelated_domain_is_independent() {
        // A different domain's own watermark never affects this one --
        // `highest_snapshot` is keyed by domain, so domain 2 seeing
        // epoch 100 must not make domain 1's epoch 1 look stale.
        let mut state = snapshot_state();
        state.commit_snapshot(snap(2, 100));
        assert!(state.snapshot_is_fresh(&snap(1, 1)));
    }

    #[test]
    fn snapshot_monotonicity_commit_only_ever_raises_the_watermark() {
        // An accepted (fresh) EQUAL epoch must not lower/reset anything
        // -- `commit_snapshot` itself is a no-op below the current high.
        let mut state = snapshot_state();
        state.commit_snapshot(snap(1, 5));
        state.commit_snapshot(snap(1, 5));
        assert!(!state.snapshot_is_fresh(&snap(1, 4)), "watermark must still be 5, not reset to 5 again from a lower path");
    }

    /// "Cross-stream reordering remains nonclosure" (packet's own test
    /// name): `highest_snapshot` is keyed by domain ONLY (T3.3.01's own
    /// frozen shape), shared across every semantic stream on this one
    /// attachment -- proven here by committing a domain's watermark
    /// while `next_expected_for` on a COMPLETELY DIFFERENT stream never
    /// moves. This is the negative half of this row's acceptance gate
    /// ("T3.3 never reports cross-stream checkpoint completeness"):
    /// accepting a snapshot says nothing about, and never advances, any
    /// OTHER stream's own independent sequence cursor -- the two axes
    /// (per-stream sequence, per-domain snapshot epoch) are provably
    /// orthogonal, not a disguised cross-stream watermark.
    #[test]
    fn snapshot_monotonicity_cross_stream_reordering_remains_nonclosure() {
        let mut state = snapshot_state();
        let general_seq_before = state.next_expected_for(SemanticStreamIdV1::General);
        let terrain_seq_before = state.next_expected_for(SemanticStreamIdV1::Terrain);

        state.commit_snapshot(snap(1, 9));

        assert_eq!(state.next_expected_for(SemanticStreamIdV1::General), general_seq_before);
        assert_eq!(state.next_expected_for(SemanticStreamIdV1::Terrain), terrain_seq_before);
        // The domain watermark itself IS shared across streams (by
        // construction, per T3.3.01's frozen key shape) -- a lower
        // epoch is stale regardless of which stream would carry it.
        // Sharing the watermark is not the same as reporting
        // completeness: nothing here claims stream General having seen
        // domain 1 means Terrain has "caught up" to anything.
        assert!(!state.snapshot_is_fresh(&snap(1, 8)));
    }

    /// `APEX-T3.3.17` "snapshot-domain profiles" tests -- Fable's
    /// resolution of the row's own ambiguous terms: `UnknownDomain` and
    /// `CausalityProfileMismatch` are unreachable on real traffic today
    /// (production declares no domains, every schema optional), so
    /// these exercise them via test-constructed profiles, never the
    /// frozen production one.

    #[test]
    fn production_causality_profile_declares_no_domains_and_is_fully_optional() {
        let profile = production_causality_profile_v1();
        assert!(profile.declared_domains.is_empty());
        assert_eq!(profile.requirements.len(), SemanticPayloadSchemaV1::ALL.len());
        assert!(profile.requirements.iter().all(|r| !r.producer_tick_required && !r.snapshot_required));
    }

    #[test]
    fn production_profile_never_rejects_any_causality_shape() {
        let profile = production_causality_profile_v1();
        for schema in SemanticPayloadSchemaV1::ALL {
            assert!(validate_causality_against_profile_v1(&SemanticCausalityV1 { producer_tick: None, snapshot: None }, schema, &profile).is_ok());
            assert!(
                validate_causality_against_profile_v1(&SemanticCausalityV1 { producer_tick: Some(1), snapshot: None }, schema, &profile).is_ok()
            );
        }
    }

    #[test]
    fn unknown_domain_rejects_via_test_profile_never_via_production() {
        // Production: no declared domains, so ANY snapshot's domain is
        // "unknown" -- but production is never asked to validate a
        // `Some(snapshot)` on real traffic (no producer sets one).
        let declared = NetEnvelopeCausalityProfileV1 { declared_domains: vec![SnapshotDomainId::new(1)], requirements: Vec::new() };
        let causality = SemanticCausalityV1 { producer_tick: None, snapshot: Some(snap(2, 0)) };
        assert_eq!(
            validate_causality_against_profile_v1(&causality, SemanticPayloadSchemaV1::ServerGeneral, &declared).unwrap_err(),
            SemanticEnvelopeRejectV1::UnknownDomain
        );
        // The SAME domain, declared -- accepted (structurally; session-
        // local monotonicity is a separate check, not exercised here).
        let causality_declared_domain = SemanticCausalityV1 { producer_tick: None, snapshot: Some(snap(1, 0)) };
        assert!(validate_causality_against_profile_v1(&causality_declared_domain, SemanticPayloadSchemaV1::ServerGeneral, &declared).is_ok());
    }

    #[test]
    fn causality_profile_mismatch_rejects_when_a_required_field_is_missing() {
        let profile = NetEnvelopeCausalityProfileV1 {
            declared_domains: Vec::new(),
            requirements: vec![CausalityRequirementV1 {
                payload_schema: SemanticPayloadSchemaV1::ServerGeneral,
                producer_tick_required: true,
                snapshot_required: false,
            }],
        };
        let tickless = SemanticCausalityV1 { producer_tick: None, snapshot: None };
        assert_eq!(
            validate_causality_against_profile_v1(&tickless, SemanticPayloadSchemaV1::ServerGeneral, &profile).unwrap_err(),
            SemanticEnvelopeRejectV1::CausalityProfileMismatch
        );
        let ticked = SemanticCausalityV1 { producer_tick: Some(7), snapshot: None };
        assert!(validate_causality_against_profile_v1(&ticked, SemanticPayloadSchemaV1::ServerGeneral, &profile).is_ok());
        // A DIFFERENT schema with no declared requirement is unaffected.
        assert!(validate_causality_against_profile_v1(&tickless, SemanticPayloadSchemaV1::ClientGeneral, &profile).is_ok());
    }

    /// "Causality profile changes without envelope profile change"
    /// (Fable's canary framing): proves the requirement-category
    /// encoding is content-sensitive, mirroring `different_tag_
    /// orderings_would_produce_different_roots`'s own encode-function-
    /// level pattern exactly -- since `net_envelope_profile_table_
    /// bytes_v1` calls this same encoder on the production profile, a
    /// changed requirement is structurally incapable of leaving
    /// `profile_root` unchanged.
    #[test]
    fn causality_profile_change_is_encoded_so_profile_root_cannot_silently_drift() {
        let unrequired = [CausalityRequirementV1 {
            payload_schema: SemanticPayloadSchemaV1::ClientGeneral,
            producer_tick_required: false,
            snapshot_required: false,
        }];
        let required = [CausalityRequirementV1 {
            payload_schema: SemanticPayloadSchemaV1::ClientGeneral,
            producer_tick_required: true,
            snapshot_required: false,
        }];
        let mut buf_a = Vec::new();
        encode_causality_requirement_category(&mut buf_a, 6, &unrequired);
        let mut buf_b = Vec::new();
        encode_causality_requirement_category(&mut buf_b, 6, &required);
        assert_ne!(buf_a, buf_b);

        // Same proof for the declared-domain category.
        let mut buf_c = Vec::new();
        encode_domain_category(&mut buf_c, 5, &[]);
        let mut buf_d = Vec::new();
        encode_domain_category(&mut buf_d, 5, &[SnapshotDomainId::new(1)]);
        assert_ne!(buf_c, buf_d);
    }

    /// `APEX-T3.3.18` tests (`cargo test -p veloren-common-net
    /// envelope::terminal_codes`). Packet's own test list: "One per
    /// terminal, redaction, rejected traffic liveness, application
    /// error consumed sequence" -- the last two are ingress-pipeline
    /// concerns, closed on the server/client sides where the pipeline
    /// actually lives, not here.

    #[test]
    fn terminal_codes_are_unique_and_the_pinned_count_forces_this_test_to_be_touched_on_growth() {
        let codes: HashSet<&str> = SemanticProtocolTerminalV1::ALL.iter().map(|t| t.code()).collect();
        assert_eq!(codes.len(), SemanticProtocolTerminalV1::ALL.len());
        assert_eq!(SemanticProtocolTerminalV1::ALL.len(), 5, "a new terminal variant needs its own disconnect_reason mapping too");
    }

    #[test]
    fn reject_codes_are_unique_and_the_pinned_count_forces_this_test_to_be_touched_on_growth() {
        let codes: HashSet<&str> = SemanticEnvelopeRejectV1::ALL.iter().map(|r| r.code()).collect();
        assert_eq!(codes.len(), SemanticEnvelopeRejectV1::ALL.len());
        assert_eq!(SemanticEnvelopeRejectV1::ALL.len(), 30);
    }

    /// "One per terminal": every `SemanticProtocolTerminalV1` maps to
    /// exactly the `DisconnectReason` its own doc comment names.
    #[test]
    fn every_terminal_maps_to_its_documented_disconnect_reason() {
        use common::comp::DisconnectReason as D;
        assert!(matches!(SemanticProtocolTerminalV1::ResyncRequired.disconnect_reason(), D::NetworkError));
        assert!(matches!(SemanticProtocolTerminalV1::SequenceExhausted.disconnect_reason(), D::NetworkError));
        assert!(matches!(SemanticProtocolTerminalV1::ApplicationError.disconnect_reason(), D::NetworkError));
        assert!(matches!(SemanticProtocolTerminalV1::ProtocolViolation.disconnect_reason(), D::Kicked));
        assert!(matches!(SemanticProtocolTerminalV1::SendFailedAfterSequenceAllocated.disconnect_reason(), D::NetworkError));
    }

    #[test]
    fn metrics_record_and_snapshot_roundtrip() {
        let metrics = SemanticIngressMetricsV1::new();
        metrics.record_reject(&SemanticEnvelopeRejectV1::StaleEpoch, SemanticStreamIdV1::General);
        metrics.record_reject(&SemanticEnvelopeRejectV1::StaleEpoch, SemanticStreamIdV1::General);
        metrics.record_reject(&SemanticEnvelopeRejectV1::SequenceGap { expected: 3, received: 9 }, SemanticStreamIdV1::Terrain);
        metrics.record_terminal(SemanticProtocolTerminalV1::ProtocolViolation, SemanticStreamIdV1::General);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot, vec![
            ("protocol_violation", SemanticStreamIdV1::General, 1),
            ("sequence_gap", SemanticStreamIdV1::Terrain, 1),
            ("stale_epoch", SemanticStreamIdV1::General, 2),
        ]);
    }

    /// `SequenceGap`'s own field values (arbitrary `u64`s, unbounded)
    /// never leak into the metrics key -- two DIFFERENT `expected`/
    /// `received` pairs collapse into the same one bucket, exactly the
    /// "field values are per-frame data, never metrics cardinality"
    /// guarantee `code()`'s own doc comment states.
    #[test]
    fn metrics_redaction_collapses_field_carrying_variants_to_one_bucket() {
        let metrics = SemanticIngressMetricsV1::new();
        metrics.record_reject(&SemanticEnvelopeRejectV1::SequenceGap { expected: 1, received: 2 }, SemanticStreamIdV1::General);
        metrics.record_reject(&SemanticEnvelopeRejectV1::SequenceGap { expected: 999, received: 1_000_000 }, SemanticStreamIdV1::General);
        assert_eq!(metrics.snapshot(), vec![("sequence_gap", SemanticStreamIdV1::General, 2)]);
    }
}
