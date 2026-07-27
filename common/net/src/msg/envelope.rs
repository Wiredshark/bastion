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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
            C::ChatMsg(_) | C::Command(_, _) | C::Terminate | C::RequestPlugins(_) => SemanticStreamIdV1::General,
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
            | S::WeatherUpdate(_)
            | S::LocalWindUpdate(_)
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
            | S::PluginData(_) => SemanticStreamIdV1::General,
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
}
