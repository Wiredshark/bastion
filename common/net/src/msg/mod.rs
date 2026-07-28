pub mod checkpoint;
pub mod command;
pub mod session_control;
pub mod client;
pub mod compression;
pub mod ecs_packet;
pub mod envelope;
pub mod plugin_artifact;
pub mod server;
pub mod world_msg;

// Reexports
pub use self::{
    client::{ClientGeneral, ClientMsg, ClientRegister, ClientType, SessionRequestV1},
    compression::{
        CompressedData, GridLtrPacking, PackingFormula, QuadPngEncoding, TriPngEncoding,
        VoxelImageEncoding, WidePacking, WireChonk,
    },
    ecs_packet::EcsCompPacket,
    envelope::{
        ActiveSessionBindingV1, NetEnvelopeHeaderV1, SemanticCausalityV1, SemanticDirectionV1, SemanticEnvelopeRejectV1,
        SemanticPayloadEncodingV1, SemanticPayloadSchemaV1, SemanticProtocolIdV1, SemanticProtocolTerminalV1,
        SemanticReceiveStateV1, SemanticSendStateV1, SemanticSnapshotRefV1, SemanticStreamIdV1, SemanticWireFrameV1,
        SnapshotDomainId, decode_payload_exact_v1, encode_payload_v1, net_envelope_profile_root_v1, payload_digest_v1,
        server_supported_semantic_protocols_v1,
    },
    server::{
        CharacterInfo, ChatTypeContext, DisconnectReason, InviteAnswer, Notification, PlayerInfo,
        PlayerListUpdate, RegisterError, SerializedTerrainChunk, ServerGeneral, ServerInfo,
        ServerInit, ServerMsg, ServerRegisterAnswer, SessionAdmissionV1, SessionBindingV1,
    },
    world_msg::WorldMapMsg,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PingMsg {
    Ping,
    Pong,
}
