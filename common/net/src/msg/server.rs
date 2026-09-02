use super::{
    ClientType, CompressedData, EcsCompPacket, PingMsg, QuadPngEncoding, TriPngEncoding,
    WidePacking, WireChonk, world_msg::EconomyInfo,
};
use crate::sync;
use common::{
    apex::identity::{ConnectionEpoch, ServerBootId, SessionId},
    calendar::Calendar,
    character::{self, CharacterItem},
    comp::{
        self, AdminRole, Content, body::Gender, gizmos::Gizmos, invite::InviteKind,
        item::MaterialStatManifest,
    },
    event::{PluginHash, UpdateCharacterMetadata},
    lod,
    outcome::Outcome,
    recipe::{ComponentRecipeBook, RecipeBookManifest},
    resources::{BattleMode, Time, TimeOfDay, TimeScale},
    rtsim,
    shared_server_config::ServerConstants,
    terrain::{Block, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    trade::{PendingTrade, SitePrices, TradeId, TradeResult},
    uid::Uid,
    uuid::Uuid,
    weather::SharedWeatherGrid,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::warn;
use vek::*;

///This struct contains all messages the server might send (on different
/// streams though)
#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum ServerMsg {
    /// Basic info about server, send ONCE, clients need it to Register
    Info(ServerInfo),
    /// Initial data package, send BEFORE Register ONCE. Not Register relevant
    Init(Box<ServerInit>),
    /// Result to `ClientMsg::Register`. send ONCE
    RegisterAnswer(ServerRegisterAnswer),
    /// Msg that can be send ALWAYS as soon as client is registered, e.g. `Chat`
    General(ServerGeneral),
    Ping(PingMsg),
}

/*
2nd Level Enums
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// APEX-T3.1: identifies this live server-process incarnation, sent
    /// before authentication so the client can echo it back in
    /// `ClientRegister` and the server can reject a stale post-restart
    /// registration before running `login_provider.verify`.
    pub server_boot_id: ServerBootId,
    pub name: String,
    pub git_hash: u32,
    pub git_timestamp: i64,
    pub auth_provider: Option<String>,
    /// `APEX-T3.3.05`: the semantic-protocol modes this server currently
    /// accepts, always sorted ascending by tag
    /// (`server_supported_semantic_protocols_v1()`). A client echoes one
    /// of these back in `ClientRegister.requested_semantic_protocol`.
    pub supported_semantic_protocols: Vec<crate::msg::envelope::SemanticProtocolIdV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerDescription {
    pub motd: String,
    pub rules: Option<String>,
}

/// Reponse To ClientType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerInit {
    GameSync {
        /// APEX-T3.1.11: repeats the same `ServerInfo` boot ID so the
        /// client can reject bootstrap state mixed across a server
        /// restart between registration and this message, before
        /// constructing `State::client`.
        server_boot_id: ServerBootId,
        entity_package: sync::EntityPackage<EcsCompPacket>,
        role: Option<AdminRole>,
        time_of_day: TimeOfDay,
        max_group_size: u32,
        client_timeout: Duration,
        world_map: crate::msg::world_msg::WorldMapMsg,
        recipe_book: RecipeBookManifest,
        component_recipe_book: ComponentRecipeBook,
        material_stats: MaterialStatManifest,
        ability_map: comp::item::tool::AbilityMap,
        server_constants: ServerConstants,
        description: ServerDescription,
        active_plugins: Vec<PluginHash>,
        /// APEX-T3.2: repeats the same `SessionBindingV1` `RegisterAnswer`
        /// carried, so the client can reject bootstrap state mixed across
        /// a session admitted for one binding but synced under another --
        /// the same "repeat and check equality before constructing State"
        /// pattern `server_boot_id` above already establishes (spec
        /// section 3.5, canaries SES-045/046).
        session_binding: SessionBindingV1,
        /// APEX-T2.5.11 wire half: the typed deployment summary. `None` =
        /// explicit legacy mode (the `active_plugins` hash path). The
        /// acquisition-before-State client flow consumes this when the
        /// .11 bootstrap lands; until then servers send `None`.
        plugin_deployment: Option<crate::msg::plugin_artifact::PluginDeploymentSummaryV1>,
    },
}

/// APEX-T3.2: identifies exactly one server-issued session attachment.
/// Carried by `RegisterAnswer`'s success arm and repeated in `GameSync`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBindingV1 {
    pub session_id: SessionId,
    pub epoch: ConnectionEpoch,
    /// `APEX-T3.3.05`: the semantic protocol this session negotiated at
    /// admission, fixed for the session's lifetime (a `Resume` requesting
    /// a different one is rejected -- `RegisterError::SemanticProtocolModeSwitch`).
    /// Reuses `T3.2`'s existing `RegisterAnswer`/`GameSync` binding-echo
    /// equality check for free -- no new equality code was written for
    /// this field.
    pub selected_semantic_protocol: crate::msg::envelope::SemanticProtocolIdV1,
}

/// APEX-T3.2: `RegisterAnswer`'s success payload -- distinguishes a
/// brand-new session from a resumed one from a same-principal
/// replacement, all three carrying the binding the client must echo back
/// for the `GameSync` equality check (spec section 3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionAdmissionV1 {
    Created { binding: SessionBindingV1 },
    Resumed { binding: SessionBindingV1 },
    Replaced { binding: SessionBindingV1 },
}

impl SessionAdmissionV1 {
    pub fn binding(&self) -> SessionBindingV1 {
        match self {
            Self::Created { binding } | Self::Resumed { binding } | Self::Replaced { binding } => *binding,
        }
    }
}

pub type ServerRegisterAnswer = Result<SessionAdmissionV1, RegisterError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedTerrainChunk {
    DeflatedChonk(CompressedData<TerrainChunk>),
    QuadPng(WireChonk<QuadPngEncoding<4>, WidePacking<true>, TerrainChunkMeta, TerrainChunkSize>),
    TriPng(WireChonk<TriPngEncoding<false>, WidePacking<true>, TerrainChunkMeta, TerrainChunkSize>),
}

impl SerializedTerrainChunk {
    pub fn approx_len(&self) -> usize {
        match self {
            SerializedTerrainChunk::DeflatedChonk(data) => data.data.len(),
            SerializedTerrainChunk::QuadPng(data) => data.data.data.len(),
            SerializedTerrainChunk::TriPng(data) => data.data.data.len(),
        }
    }

    pub fn via_heuristic(chunk: &TerrainChunk, lossy_compression: bool) -> Self {
        if lossy_compression && (chunk.get_max_z() - chunk.get_min_z() <= 128) {
            Self::quadpng(chunk)
        } else {
            Self::deflate(chunk)
        }
    }

    pub fn deflate(chunk: &TerrainChunk) -> Self {
        Self::DeflatedChonk(CompressedData::compress(chunk, 1))
    }

    pub fn quadpng(chunk: &TerrainChunk) -> Self {
        if let Some(wc) = WireChonk::from_chonk(QuadPngEncoding(), WidePacking(), chunk) {
            Self::QuadPng(wc)
        } else {
            warn!("Image encoding failure occurred, falling back to deflate");
            Self::deflate(chunk)
        }
    }

    pub fn tripng(chunk: &TerrainChunk) -> Self {
        if let Some(wc) = WireChonk::from_chonk(TriPngEncoding(), WidePacking(), chunk) {
            Self::TriPng(wc)
        } else {
            warn!("Image encoding failure occurred, falling back to deflate");
            Self::deflate(chunk)
        }
    }

    pub fn to_chunk(&self) -> Option<TerrainChunk> {
        match self {
            Self::DeflatedChonk(chonk) => chonk.decompress(),
            Self::QuadPng(wc) => wc.to_chonk(),
            Self::TriPng(wc) => wc.to_chonk(),
        }
    }
}

/// Messages sent from the server to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerGeneral {
    //Character Screen related
    /// Result of loading character data
    CharacterDataLoadResult(Result<UpdateCharacterMetadata, String>),
    /// A list of characters belonging to the a authenticated player was sent
    CharacterListUpdate(Vec<CharacterItem>),
    /// An error occurred while creating or deleting a character
    CharacterActionError(String),
    /// A new character was created
    CharacterCreated(character::CharacterId),
    CharacterEdited(character::CharacterId),
    CharacterSuccess,
    SpectatorSuccess(Vec3<f32>),
    //Ingame related
    GroupUpdate(comp::group::ChangeNotification<Uid>),
    /// Indicate to the client that they are invited to join a group
    Invite {
        inviter: Uid,
        timeout: Duration,
        kind: InviteKind,
    },
    /// Indicate to the client that their sent invite was not invalid and is
    /// currently pending
    InvitePending(Uid),
    /// Update the HUD of the clients in the group
    GroupInventoryUpdate(comp::FrontendItem, Uid),
    /// Note: this could potentially include all the failure cases such as
    /// inviting yourself in which case the `InvitePending` message could be
    /// removed and the client could consider their invite pending until
    /// they receive this message Indicate to the client the result of their
    /// invite
    InviteComplete {
        target: Uid,
        answer: InviteAnswer,
        kind: InviteKind,
    },
    /// Trigger cleanup for when the client goes back to the `Registered` state
    /// from an ingame state
    ExitInGameSuccess,
    InventoryUpdate(comp::Inventory, Vec<comp::InventoryUpdateEvent>),
    Dialogue(Uid, rtsim::Dialogue<true>),
    /// NOTE: The client can infer that entity view distance will be at most the
    /// terrain view distance that we send here (and if lower it won't be
    /// modified). So we just need to send the terrain VD back to the client
    /// if corrections are made.
    SetViewDistance(u32),
    Outcomes(Vec<Outcome>),
    Knockback(Vec3<f32>),
    // Ingame related AND terrain stream
    TerrainChunkUpdate {
        key: Vec2<i32>,
        chunk: Result<SerializedTerrainChunk, ()>,
    },
    LodZoneUpdate {
        key: Vec2<i32>,
        zone: lod::Zone,
    },
    // DET-NET-014 (v6 deep-pass, High): the wire payload is a POSITION-SORTED
    // Vec, not a HashMap. A HashMap serializes in process-seed iteration order,
    // so equivalent terrain updates encoded to different bytes (breaking exact
    // wire evidence) and applied on the client in different order. The sorted
    // Vec is canonical by construction.
    TerrainBlockUpdates(CompressedData<Vec<(Vec3<i32>, Block)>>),
    // Always possible
    PlayerListUpdate(PlayerListUpdate),
    /// A message to go into the client chat box. The client is responsible for
    /// formatting the message and turning it into a speech bubble.
    ChatMsg(comp::ChatMsg),
    ChatMode(comp::ChatMode),
    SetPlayerEntity(Uid),
    TimeOfDay(TimeOfDay, Calendar, Time, TimeScale),
    EntitySync(sync::EntitySyncPackage),
    CompSync(
        sync::CompSyncPackage<EcsCompPacket>,
        common::apex::physics_generation::PhysicsGenerationV1,
    ),
    CreateEntity(sync::EntityPackage<EcsCompPacket>),
    DeleteEntity(Uid),
    Disconnect(DisconnectReason),
    /// `APEX-T3.4.20b`: opens this stream's fenced segment for a
    /// checkpoint epoch. Control, never data: it carries no ordinal.
    CheckpointBegin(Box<super::checkpoint::CheckpointStreamOpenV1>),
    /// `APEX-T3.4.20b`: seals it, declaring exactly what crossed so the
    /// receiver can check its own transcript against the claim.
    CheckpointBarrier(super::checkpoint::CheckpointBarrierV1),
    /// `APEX-T3.5.13`: one command's terminal result, published inside
    /// the same checkpoint as the effect it reports. Checkpointed data,
    /// not out-of-band chatter (`CMD-128`, `CMD-129`).
    CommandResult(super::command::CommandPublicationV1),
    /// Send a popup notification such as "Waypoint Saved"
    Notification(Notification),
    UpdatePendingTrade(TradeId, PendingTrade, Option<SitePrices>),
    FinishedTrade(TradeResult),
    /// Economic information about sites
    SiteEconomy(EconomyInfo),
    MapMarker(comp::MapMarkerUpdate),
    /// `APEX-T5.2`: the grid PLUS the identity of the snapshot it is.
    /// The id is `T0.87`'s weather generation epoch — the counter
    /// incremented exactly once at the single named adoption point — not
    /// a second counter minted for the wire. Two identities for one
    /// snapshot is the confusion this program exists to prevent.
    WeatherUpdate(SharedWeatherGrid, common::apex::weather_snapshot::WeatherSnapshotIdV1),
    /// `APEX-T5.2`: local wind plus the snapshot it belongs to. A client
    /// that cannot name the snapshot a wind came from cannot replay
    /// against it, which is `T5.4`'s whole point.
    LocalWindUpdate(Vec2<f32>, common::apex::weather_snapshot::WeatherSnapshotIdV1),
    /// `APEX-T5.3`: the server's receipt for one input frame, in wire
    /// form. The client rebuilds the typed receipt rather than receiving
    /// one already typed.
    InputReceipt(crate::msg::input_receipt_wire::InputReceiptWireV1),
    /// Suggest the client to spectate a position. Called after client has
    /// requested teleport etc.
    SpectatePosition(Vec3<f32>),
    /// Plugin data requested from the server
    PluginData(Vec<u8>),
    /// APEX-T2.5.10: one typed artifact (root/ordinal/digest/size +
    /// bytes); transport order carries no meaning. `PluginData` remains
    /// for explicit legacy mode only.
    PluginArtifactData(crate::msg::plugin_artifact::PluginArtifactResponseV1),
    /// Update the list of available recipes. Usually called after a new recipe
    /// is acquired
    UpdateRecipes,
    SetPlayerRole(Option<AdminRole>),
    Gizmos(Vec<Gizmos>),
    /// bastion (B2a): a validated designation echoed back to the placing
    /// overseer, so the client can render the region overlay. B4 replaces the
    /// echo with real job-board state.
    BastionDesignation {
        region: common::bastion::Region,
        kind: common::bastion::DesignationKind,
        /// B5.6b-2: `Some` for surface-relative placements (`region` holds
        /// the exact resolved bounds); the client keeps it so the volume
        /// rendering can count levels. `None` for legacy literal regions.
        z_extent: Option<common::bastion::ZExtent>,
    },
    /// bastion (B5.5): a cancelled/erased designation region echoed back so
    /// the client subtracts it from its stored overlay rects.
    BastionDesignationRemoved {
        region: common::bastion::Region,
    },
    /// bastion (UI-4 row 62 → UI-5 row 62.2): one inspected object's
    /// payload, the reply to `ClientGeneral::BastionInspect` (request/response
    /// on selection — a single-target on-demand query, not comp-sync). The
    /// echoed `target` lets the client match the reply to its live selection;
    /// `payload` is `None` when nothing Bastion-tracked sits at the target.
    BastionInspectInfo {
        target: comp::bastion::BastionInspectTarget,
        payload: Option<comp::bastion::BastionInspectKind>,
    },
    /// `APEX-T4.1` chunk 2a: a total, classified compatibility report the
    /// client can validate before applying `ServerInit::GameSync`'s bulk
    /// state. Sent (`server/src/sys/msg/register.rs`'s `finalize_admission`)
    /// immediately before `GameSync` in the SAME admission call -- ordering
    /// is this row's whole mechanism, so this send site is not incidental.
    /// Reception/validation ordering enforcement on the client is a
    /// separate, follow-up chunk (`T4.1` chunk 2b); this chunk is the
    /// EMISSION only, and is dormant in practice today: `T3.3.05`'s own
    /// doc notes no live client requests anything but the `Legacy`
    /// semantic protocol yet, and this message rides the same gate.
    BootstrapManifest(crate::msg::bootstrap_manifest_wire::BootstrapManifestWireV1),
    /// W3 renderer-bench (`readme/renderer-bench/W3-LAUNCH-PACKET.md`):
    /// one cadence frame's announce, sent to every in-game client
    /// (spectators included). Always compiled — no feature gate — and
    /// inert unless a bench run is armed on the server.
    RendererBenchFrame(common::renderer_bench::BenchFrameAnnounceV1),
    /// bastion (ZONE ASSIGNMENT, Ben 2026-09-01): the whole assignment list,
    /// (colonist, the zone's region, set by hand?), sent on every change and
    /// every 600 ticks as a fallback, so a zone can SHOW who works it.
    /// LAST in the enum on purpose: appending keeps every older discriminant.
    BastionAssignments {
        entries: Vec<(common::uid::Uid, common::bastion::Region, bool)>,
    },
}

impl ServerGeneral {
    // TODO: Don't use `Into<Content>` since this treats all strings as plaintext,
    // properly localise server messages
    pub fn server_msg(chat_type: comp::ChatType<String>, content: impl Into<Content>) -> Self {
        ServerGeneral::ChatMsg(chat_type.into_msg(content.into()))
    }
}

/*
end of 2nd level Enums
*/

/// Inform the client of updates to the player list.
///
/// Note: Before emiting any of these, check if the current
/// [`veloren_client::Client::client_type`] wants to emit login events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerListUpdate {
    // DET-NET-014/015 (v6 deep-pass, High): a Uid-sorted Vec, not a HashMap.
    // A HashMap serializes in process-seed order, so the initial player list
    // encoded to different wire bytes run-to-run and the client initialized in
    // that order. The sorted Vec is canonical by construction.
    Init(Vec<(Uid, PlayerInfo)>),
    Add(Uid, PlayerInfo),
    SelectedCharacter(Uid, CharacterInfo),
    ExitCharacter(Uid),
    Moderator(Uid, bool),
    Remove(Uid),
    Alias(Uid, String),
    UpdateBattleMode(Uid, BattleMode),
}

impl PlayerListUpdate {
    /// DET-NET-015: build the initial player-list update with a canonical,
    /// wire-stable ordering — sorted by Uid. Callers hold the player list as a
    /// HashMap (built in ECS-join / process-hash order), so constructing `Init`
    /// through this helper (rather than collecting the HashMap directly) is what
    /// makes the serialized bytes — and the order the client initializes its
    /// player list — identical run-to-run.
    pub fn init_canonical(player_list: impl IntoIterator<Item = (Uid, PlayerInfo)>) -> Self {
        let mut list: Vec<(Uid, PlayerInfo)> = player_list.into_iter().collect();
        list.sort_unstable_by_key(|(uid, _)| uid.0);
        PlayerListUpdate::Init(list)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub is_moderator: bool,
    pub is_online: bool,
    pub player_alias: String,
    pub character: Option<CharacterInfo>,
    pub uuid: Uuid,
}

/// used for localisation, filled by client and used by i18n code
pub struct ChatTypeContext {
    pub you: Uid,
    pub player_info: HashMap<Uid, PlayerInfo>,
    pub entity_name: HashMap<Uid, Content>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    /// The name of specific character, not to be mistaken for player's alias.
    ///
    /// We use Content here as for all names, but any character name provided
    /// directly from a client will be `Content::Plain`
    pub name: Content,
    pub gender: Option<Gender>,
    pub battle_mode: BattleMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InviteAnswer {
    Accepted,
    Declined,
    TimedOut,
}

/// A message that should be displayed to the player, possibly with data to
/// update the client.
///
/// See [`veloren_client::UserNotification`] for the stripped down version,
/// which the client sends to the UI after removing (and using) any data that is
/// not relevant to rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notification {
    WaypointSaved { location_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BanInfo {
    pub reason: String,
    /// Unix timestamp at which the ban will expire
    pub until: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisconnectReason {
    /// Server shut down
    Shutdown,
    /// Client was kicked
    Kicked(String),
    Banned(BanInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegisterError {
    AuthError(String),
    Banned(BanInfo),
    Kicked(String),
    InvalidCharacter,
    NotOnWhitelist,
    TooManyPlayers,
    /// APEX-T3.1.10: the client's `ClientRegister.expected_server_boot_id`
    /// does not match this process's current boot ID -- typically because
    /// the server restarted between sending `ServerInfo` and receiving
    /// this registration. Distinct from every other terminal here so
    /// callers can offer a full reconnect rather than an auth/ban/kick UX.
    ServerBootMismatch {
        current: ServerBootId,
        received: ServerBootId,
    },
    /// APEX-T3.2 (`UNKNOWN-SESSION`, SES-027): `Resume`'s locator does not
    /// name any session record this process holds (never existed, or was
    /// purged after expiry).
    UnknownSession,
    /// APEX-T3.2 (`SESSION-PRINCIPAL-MISMATCH`, SES-028): the
    /// freshly-authenticated principal differs from the one the resumed
    /// session record was issued to. Never merged into `UnknownSession`
    /// -- a distinct terminal so client UX can tell "no such session"
    /// from "that session isn't yours".
    SessionPrincipalMismatch,
    /// APEX-T3.2 (`SESSION-EXPIRED`, SES-030/031): the detached record's
    /// retention window has elapsed (boundary-inclusive: exactly
    /// `expires_at` counts as expired).
    SessionExpired,
    /// APEX-T3.2 (`STALE-CONNECTION-EPOCH`/`FUTURE-CONNECTION-EPOCH`,
    /// SES-032/033): `Resume.expected_epoch` does not equal the record's
    /// current epoch. One variant, `current`/`expected` both carried, so
    /// the client can tell which direction the mismatch went without a
    /// second terminal.
    ConnectionEpochMismatch {
        current: ConnectionEpoch,
        expected: ConnectionEpoch,
    },
    /// APEX-T3.2 (`CONNECTION-EPOCH-EXHAUSTED`, SES-034): the session's
    /// epoch counter is already at `u64::MAX`; no further attachment can
    /// be admitted for it (never silently wraps -- `ConnectionEpoch::
    /// checked_next` is `checked`, not `wrapping`).
    ConnectionEpochExhausted,
    /// APEX-T3.2 (`SESSION-CLIENT-TYPE-MISMATCH`, SES-035-038): `Resume`
    /// requested a different `ClientType` than the session was created
    /// with (e.g. a `Game` session resumed as `ChatOnly`).
    SessionClientTypeMismatch {
        session: ClientType,
        requested: ClientType,
    },
    /// APEX-T3.2 (`OLDER-ATTEMPT-SUPERSEDED`, SES-054/055): a same-principal
    /// race within one admission phase -- this attempt's `attempt_seq` was
    /// smaller than another attempt already committed for the same
    /// principal in the same phase. Distinct from `SessionBootMismatch`'s
    /// `NewerLogin` disconnect (that path is for an already-committed
    /// active session from an *earlier* phase being replaced by a new
    /// one); this is two intents racing within the *same* phase, neither
    /// of which had committed yet when the race was resolved.
    OlderAttemptSuperseded,
    /// `APEX-T3.3.05`: `ClientRegister.requested_semantic_protocol` is not
    /// in this server's currently-advertised set. Currently dormant in
    /// live traffic (this tree's client always requests `Legacy`, this
    /// tree's server always advertises both `Legacy` and
    /// `NetEnvelopeV1`) -- real and tested, not reachable by the live
    /// client/server pair today.
    IncompatibleSemanticProtocol,
    /// `APEX-T3.3.05`: a `Resume` requested a different
    /// `SemanticProtocolIdV1` than the session was originally negotiated
    /// with. Packet section 5.9: "one attachment may never mix modes" --
    /// the semantic-protocol twin of `SessionClientTypeMismatch` above.
    SemanticProtocolModeSwitch,
}

impl ServerMsg {
    pub fn verify(
        &self,
        c_type: ClientType,
        registered: bool,
        presence: Option<comp::PresenceKind>,
    ) -> bool {
        match self {
            ServerMsg::Info(_) | ServerMsg::Init(_) | ServerMsg::RegisterAnswer(_) => {
                !registered && presence.is_none()
            },
            ServerMsg::General(g) => {
                registered
                    && match g {
                        //Character Screen related
                        ServerGeneral::CharacterDataLoadResult(_)
                        | ServerGeneral::CharacterListUpdate(_)
                        | ServerGeneral::CharacterActionError(_)
                        | ServerGeneral::CharacterEdited(_)
                        | ServerGeneral::CharacterCreated(_) => {
                            c_type != ClientType::ChatOnly && presence.is_none()
                        },
                        ServerGeneral::CharacterSuccess | ServerGeneral::SpectatorSuccess(_) => {
                            c_type == ClientType::Game && presence.is_none()
                        },
                        //Ingame related
                        ServerGeneral::GroupUpdate(_)
                        | ServerGeneral::Invite { .. }
                        | ServerGeneral::InvitePending(_)
                        | ServerGeneral::InviteComplete { .. }
                        | ServerGeneral::ExitInGameSuccess
                        | ServerGeneral::InventoryUpdate(_, _)
                        | ServerGeneral::GroupInventoryUpdate(_, _)
                        | ServerGeneral::Dialogue(_, _)
                        | ServerGeneral::TerrainChunkUpdate { .. }
                        | ServerGeneral::TerrainBlockUpdates(_)
                        | ServerGeneral::SetViewDistance(_)
                        | ServerGeneral::Outcomes(_)
                        | ServerGeneral::Knockback(_)
                        | ServerGeneral::UpdatePendingTrade(_, _, _)
                        | ServerGeneral::FinishedTrade(_)
                        | ServerGeneral::SiteEconomy(_)
                        | ServerGeneral::MapMarker(_)
                        | ServerGeneral::WeatherUpdate(..)
                        | ServerGeneral::LocalWindUpdate(..)
                        | ServerGeneral::InputReceipt(_)
                        | ServerGeneral::SpectatePosition(_)
                        | ServerGeneral::UpdateRecipes
                        | ServerGeneral::Gizmos(_)
                        | ServerGeneral::BastionDesignation { .. }
                        | ServerGeneral::BastionDesignationRemoved { .. }
                        | ServerGeneral::BastionAssignments { .. }
                        | ServerGeneral::BastionInspectInfo { .. }
                        // W3 renderer-bench: announces reach any in-game
                        // observer, spectators included.
                        | ServerGeneral::RendererBenchFrame(_) => {
                            c_type == ClientType::Game && presence.is_some()
                        },
                        // Always possible
                        ServerGeneral::PlayerListUpdate(_)
                        | ServerGeneral::ChatMsg(_)
                        | ServerGeneral::ChatMode(_)
                        | ServerGeneral::SetPlayerEntity(_)
                        | ServerGeneral::TimeOfDay(_, _, _, _)
                        | ServerGeneral::EntitySync(_)
                        | ServerGeneral::CompSync(_, _)
                        | ServerGeneral::CreateEntity(_)
                        | ServerGeneral::DeleteEntity(_)
                        | ServerGeneral::Disconnect(_)
                        | ServerGeneral::Notification(_)
                        | ServerGeneral::SetPlayerRole(_)
                        | ServerGeneral::LodZoneUpdate { .. } => true,
                        ServerGeneral::PluginData(_) => true,
                        // T3.4.20b: fences are session-scoped, not
                        // presence-scoped -- a checkpoint can span the
                        // character screen as readily as in-game.
                        ServerGeneral::CheckpointBegin(_) | ServerGeneral::CheckpointBarrier(_) => true,
                        // A command result is meaningful only to a
                        // registered session, which `registered` above
                        // already requires.
                        ServerGeneral::CommandResult(_) => true,
                        ServerGeneral::PluginArtifactData(_) => true,
                        // Sent in the same admission call as GameSync,
                        // before presence is established -- same
                        // session-scoped-not-presence-scoped reasoning as
                        // the checkpoint messages above.
                        ServerGeneral::BootstrapManifest(_) => true,
                    }
            },
            ServerMsg::Ping(_) => true,
        }
    }
}

impl From<comp::ChatMsg> for ServerGeneral {
    fn from(v: comp::ChatMsg) -> Self { ServerGeneral::ChatMsg(v) }
}

impl From<ServerInfo> for ServerMsg {
    fn from(o: ServerInfo) -> ServerMsg { ServerMsg::Info(o) }
}

impl From<ServerInit> for ServerMsg {
    fn from(o: ServerInit) -> ServerMsg { ServerMsg::Init(Box::new(o)) }
}

impl From<ServerRegisterAnswer> for ServerMsg {
    fn from(o: ServerRegisterAnswer) -> ServerMsg { ServerMsg::RegisterAnswer(o) }
}

impl From<ServerGeneral> for ServerMsg {
    fn from(o: ServerGeneral) -> ServerMsg { ServerMsg::General(o) }
}

impl From<PingMsg> for ServerMsg {
    fn from(o: PingMsg) -> ServerMsg { ServerMsg::Ping(o) }
}

/// DET-NET-014: canonicalize terrain block updates into a position-sorted Vec
/// for the wire. The source is a HashMap<Vec3<i32>, Block> whose iteration order
/// rides the process hash seed, so building the compressed `TerrainBlockUpdates`
/// payload through this helper is what makes the serialized bytes byte-canonical
/// and the client apply them in a deterministic order.
pub fn canonical_terrain_block_updates(
    blocks: impl IntoIterator<Item = (Vec3<i32>, Block)>,
) -> Vec<(Vec3<i32>, Block)> {
    let mut list: Vec<(Vec3<i32>, Block)> = blocks.into_iter().collect();
    list.sort_unstable_by_key(|(p, _)| (p.x, p.y, p.z));
    list
}

#[cfg(test)]
mod det_net_wire_order_tests {
    use super::*;
    use std::num::NonZeroU64;

    fn uid(n: u64) -> Uid { Uid(NonZeroU64::new(n).unwrap()) }

    fn dummy_info(alias: &str) -> PlayerInfo {
        PlayerInfo {
            is_moderator: false,
            is_online: true,
            player_alias: alias.to_string(),
            character: None,
            uuid: Uuid::nil(),
        }
    }

    /// NET-01 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof):
    /// `PlayerListUpdate::init_canonical` emits the initial player list in a
    /// canonical Uid-sorted order regardless of the caller's HashMap iteration
    /// order (DET-NET-015). Without it the wire bytes — and the order the client
    /// initializes its player list — would vary per process hash seed. There was
    /// no executable evidence for this contract; the sort was inline in the
    /// register system.
    #[test]
    fn player_list_init_is_uid_sorted_and_input_order_independent() {
        // The same three players supplied in two DIFFERENT input orders (as a
        // HashMap's per-process iteration order would).
        let order_a = vec![
            (uid(5), dummy_info("e")),
            (uid(1), dummy_info("a")),
            (uid(3), dummy_info("c")),
        ];
        let order_b = vec![
            (uid(3), dummy_info("c")),
            (uid(5), dummy_info("e")),
            (uid(1), dummy_info("a")),
        ];

        let (va, vb) = match (
            PlayerListUpdate::init_canonical(order_a),
            PlayerListUpdate::init_canonical(order_b),
        ) {
            (PlayerListUpdate::Init(va), PlayerListUpdate::Init(vb)) => (va, vb),
            _ => panic!("init_canonical must produce PlayerListUpdate::Init"),
        };

        // Canonical: strictly ascending by Uid.
        let uids_a: Vec<u64> = va.iter().map(|(u, _)| u.0.get()).collect();
        assert_eq!(
            uids_a,
            vec![1, 3, 5],
            "initial player list is not Uid-sorted (DET-NET-015): {uids_a:?}"
        );

        // Input-order-independent: the two source orderings produce identical
        // wire ordering. A regression that collected the HashMap directly would
        // encode per-process-order bytes and fail this.
        let uids_b: Vec<u64> = vb.iter().map(|(u, _)| u.0.get()).collect();
        assert_eq!(
            uids_a, uids_b,
            "player list wire order depends on input/HashMap order — DET-NET-015 regressed"
        );
    }

    /// NET-02 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof):
    /// `canonical_terrain_block_updates` emits block updates in a canonical
    /// (x,y,z)-sorted order regardless of the caller's HashMap iteration order
    /// (DET-NET-014). The sort was inline in the terrain_sync system with no
    /// executable evidence.
    #[test]
    fn terrain_block_updates_are_position_sorted_and_input_order_independent() {
        let b = Block::empty();
        // The same block set supplied in two DIFFERENT input orders.
        let order_a = vec![
            (Vec3::new(2, 0, 0), b),
            (Vec3::new(0, 1, 0), b),
            (Vec3::new(0, 0, 3), b),
            (Vec3::new(0, 0, 1), b),
        ];
        let order_b = vec![
            (Vec3::new(0, 0, 1), b),
            (Vec3::new(0, 0, 3), b),
            (Vec3::new(0, 1, 0), b),
            (Vec3::new(2, 0, 0), b),
        ];
        let va = canonical_terrain_block_updates(order_a);
        let vb = canonical_terrain_block_updates(order_b);

        // Canonical: sorted by (x, y, z).
        let pa: Vec<(i32, i32, i32)> = va.iter().map(|(p, _)| (p.x, p.y, p.z)).collect();
        assert_eq!(
            pa,
            vec![(0, 0, 1), (0, 0, 3), (0, 1, 0), (2, 0, 0)],
            "terrain block updates not position-sorted (DET-NET-014): {pa:?}"
        );

        // Input-order-independent.
        let pb: Vec<(i32, i32, i32)> = vb.iter().map(|(p, _)| (p.x, p.y, p.z)).collect();
        assert_eq!(
            pa, pb,
            "terrain block-update wire order depends on input/HashMap order — DET-NET-014 regressed"
        );
    }
}

/// APEX-T3.1.06/.11: bincode-legacy round-trip for the fields T3.1 added to
/// live wire messages -- the same config `network/src/message.rs` uses,
/// not a synthetic one.
#[cfg(test)]
mod apex_t3_1_wire_tests {
    use super::*;
    use common::apex::identity::{FixedRandomBytesSourceV1, ServerBootId};

    fn fixed_boot_id() -> ServerBootId {
        ServerBootId::generate(&mut FixedRandomBytesSourceV1([0x42; 16])).unwrap()
    }

    #[test]
    fn server_info_round_trips_with_boot_id() {
        let info = ServerInfo {
            server_boot_id: fixed_boot_id(),
            name: "test".into(),
            git_hash: 0,
            git_timestamp: 0,
            auth_provider: None,
            supported_semantic_protocols: crate::msg::server_supported_semantic_protocols_v1(),
        };
        let bytes = bincode::serde::encode_to_vec(&info, bincode::config::legacy()).unwrap();
        let (decoded, _): (ServerInfo, usize) = bincode::serde::decode_from_slice(&bytes, bincode::config::legacy()).unwrap();
        assert_eq!(decoded.server_boot_id, info.server_boot_id);
    }

    #[test]
    fn register_error_boot_mismatch_round_trips() {
        let current = fixed_boot_id();
        let received = ServerBootId::generate(&mut FixedRandomBytesSourceV1([0x99; 16])).unwrap();
        assert_ne!(current, received);
        let err: ServerRegisterAnswer = Err(RegisterError::ServerBootMismatch { current, received });
        let bytes = bincode::serde::encode_to_vec(&err, bincode::config::legacy()).unwrap();
        let (decoded, _): (ServerRegisterAnswer, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::legacy()).unwrap();
        match decoded {
            Err(RegisterError::ServerBootMismatch { current: c, received: r }) => {
                assert_eq!(c, current);
                assert_eq!(r, received);
            },
            other => panic!("expected ServerBootMismatch, got {other:?}"),
        }
    }
}
