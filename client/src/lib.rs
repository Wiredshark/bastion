#![deny(unsafe_code)]
#![deny(clippy::clone_on_ref_ptr)]

pub mod addr;
pub mod error;

// Reexports
pub use crate::error::Error;
pub use authc::AuthClientError;
pub use common_net::msg::ServerInfo;
pub use specs::{
    Builder, DispatcherBuilder, Entity as EcsEntity, Join, LendJoin, ReadStorage, World, WorldExt,
};

use crate::addr::ConnectionArgs;
use byteorder::{ByteOrder, LittleEndian};
use common::apex::weather_snapshot::{
    PredictionWindSourceV1, WeatherSnapshotIdV1, WeatherSnapshotStoreV1,
};
use common::{
    character::{CharacterId, CharacterItem},
    comp::{
        self, AdminRole, CharacterState, ChatMode, ControlAction, ControlEvent, Controller,
        ControllerInputs, GroupManip, Hardcore, InputKind, InventoryAction, InventoryEvent,
        InventoryUpdateEvent, MapMarkerChange, PresenceKind, UtteranceKind,
        chat::KillSource,
        controller::CraftEvent,
        gizmos::Gizmos,
        group,
        inventory::{
            InventorySortOrder,
            item::{ItemKind, modular, tool},
        },
        invite::{InviteKind, InviteResponse},
        skills::Skill,
        slot::{EquipSlot, InvSlotId, Slot},
    },
    event::{EventBus, LocalEvent, PluginHash, UpdateCharacterMetadata},
    grid::Grid,
    link::Is,
    lod,
    map::Marker,
    mounting::{Rider, VolumePos, VolumeRider},
    outcome::Outcome,
    recipe::{ComponentRecipeBook, RecipeBookManifest},
    resources::{BattleMode, DeltaTime, GameMode, PlayerEntity, Time, TimeOfDay},
    rtsim,
    shared_server_config::ServerConstants,
    spiral::Spiral2d,
    terrain::{
        BiomeKind, CoordinateConversions, SiteKindMeta, SpriteKind, TerrainChunk, TerrainChunkSize,
        TerrainGrid, block::Block, map::MapConfig, neighbors,
    },
    trade::{PendingTrade, SitePrices, TradeAction, TradeId, TradeResult},
    uid::{IdMaps, Uid},
    vol::RectVolSize,
    weather::{CompressedWeather, SharedWeatherGrid, Weather, WeatherGrid},
};
#[cfg(feature = "tracy")] use common_base::plot;
use common_base::{prof_span, span};
use common_i18n::Content;
use common_net::{
    msg::{
        ChatTypeContext, ClientGeneral, ClientMsg, ClientRegister, DisconnectReason, InviteAnswer,
        Notification, PingMsg, PlayerInfo, PlayerListUpdate, RegisterError, ServerGeneral,
        ServerInit, ServerRegisterAnswer, SessionBindingV1, SessionRequestV1,
        server::ServerDescription,
        world_msg::{EconomyInfo, PoiInfo, SiteId},
    },
    sync::WorldSyncExt,
};

pub use common_net::msg::ClientType;
use common_state::State;
#[cfg(feature = "plugins")]
use common_state::plugin::PluginMgr;
use common_systems::add_local_systems;
use comp::BuffKind;
use hashbrown::{HashMap, HashSet};
use hickory_resolver::{
    Resolver, config::ResolverConfig, net::runtime::TokioRuntimeProvider, proto::rr::RData,
};
use image::DynamicImage;
use network::{ConnectAddr, Network, Participant, Pid, Stream};
use num::traits::FloatConst;
use rayon::prelude::*;
use rustls::client::danger::ServerCertVerified;
use specs::{Component, SystemData};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Debug,
    mem,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;
use tracing::{debug, error, trace, warn};
use vek::*;

pub const MAX_SELECTABLE_VIEW_DISTANCE: u32 = 65;

const PING_ROLLING_AVERAGE_SECS: usize = 10;

/// `APEX-T7.1` Decision 5: the prediction buffer's two limits. See
/// `Client::prediction_buffer`'s own doc for why the duration limit is
/// an entry count (approximating 500ms) rather than an exact duration.
const PREDICTION_BUFFER_CAPACITY_TICKS: usize = 128;
const PREDICTION_BUFFER_BUDGET_BYTES: usize = 64 * 1024;

/// Client frontend events.
///
/// These events are returned to the frontend that ticks the client.
#[derive(Debug)]
pub enum Event {
    Chat(comp::ChatMsg),
    GroupInventoryUpdate(comp::FrontendItem, Uid),
    InviteComplete {
        target: Uid,
        answer: InviteAnswer,
        kind: InviteKind,
    },
    TradeComplete {
        result: TradeResult,
        trade: PendingTrade,
    },
    Disconnect,
    DisconnectionNotification(u64),
    InventoryUpdated(Vec<InventoryUpdateEvent>),
    Notification(UserNotification),
    SetViewDistance(u32),
    Outcome(Outcome),
    CharacterCreated(CharacterId),
    CharacterEdited(CharacterId),
    CharacterJoined(UpdateCharacterMetadata),
    CharacterError(String),
    MapMarker(comp::MapMarkerUpdate),
    StartSpectate(Vec3<f32>),
    SpectatePosition(Vec3<f32>),
    PluginDataReceived(Vec<u8>),
    Dialogue(Uid, rtsim::Dialogue<true>),
    Gizmos(Vec<Gizmos>),
}

/// A message for the user to be displayed through the UI.
///
/// This type mirrors the [`common_net::msg::Notification`] type, but does not
/// include any data that the UI does not need.
#[derive(Debug)]
pub enum UserNotification {
    WaypointUpdated,
}

#[derive(Debug)]
pub enum ClientInitStage {
    /// A connection to the server is being created
    ConnectionEstablish,
    /// Waiting for server version
    WatingForServerVersion,
    /// We're currently authenticating with the server
    Authentication,
    /// Loading map data, site information, recipe information and other
    /// initialization data
    LoadingInitData,
    /// Prepare data received by the server to be used by the client (insert
    /// data into the ECS, render map)
    StartingClient,
}

pub struct WorldData {
    /// Just the "base" layer for LOD; currently includes colors and nothing
    /// else. In the future we'll add more layers, like shadows, rivers, and
    /// probably foliage, cities, roads, and other structures.
    pub lod_base: Grid<u32>,
    /// The "height" layer for LOD; currently includes only land altitudes, but
    /// in the future should also water depth, and probably other
    /// information as well.
    pub lod_alt: Grid<u32>,
    /// The "shadow" layer for LOD.  Includes east and west horizon angles and
    /// an approximate max occluder height, which we use to try to
    /// approximate soft and volumetric shadows.
    pub lod_horizon: Grid<u32>,
    /// A fully rendered map image for use with the map and minimap; note that
    /// this can be constructed dynamically by combining the layers of world
    /// map data (e.g. with shadow map data or river data), but at present
    /// we opt not to do this.
    ///
    /// The first two elements of the tuple are the regular and topographic maps
    /// respectively. The third element of the tuple is the world size (as a 2D
    /// grid, in chunks), and the fourth element holds the minimum height for
    /// any land chunk (i.e. the sea level) in its x coordinate, and the maximum
    /// land height above this height (i.e. the max height) in its y coordinate.
    map: (Vec<Arc<DynamicImage>>, Vec2<u16>, Vec2<f32>),
}

impl WorldData {
    pub fn chunk_size(&self) -> Vec2<u16> { self.map.1 }

    pub fn map_layers(&self) -> &Vec<Arc<DynamicImage>> { &self.map.0 }

    pub fn map_image(&self) -> &Arc<DynamicImage> { &self.map.0[0] }

    pub fn topo_map_image(&self) -> &Arc<DynamicImage> { &self.map.0[1] }

    pub fn min_chunk_alt(&self) -> f32 { self.map.2.x }

    pub fn max_chunk_alt(&self) -> f32 { self.map.2.y }

    pub fn alt_at(&self, cpos: Vec2<i32>) -> Option<f32> {
        let [a, b, _, _] = self.lod_alt.get(cpos)?.to_le_bytes();
        Some(
            (a as f32 * (1.0 / 256.0) + b as f32) * (1.0 / 256.0) * self.max_chunk_alt()
                + self.min_chunk_alt(),
        )
    }
}

pub struct SiteMarker {
    pub marker: Marker,
    pub economy: Option<EconomyInfo>,
}

struct WeatherLerp {
    old: (SharedWeatherGrid, Instant),
    new: (SharedWeatherGrid, Instant),
    old_local_wind: (Vec2<f32>, Instant),
    new_local_wind: (Vec2<f32>, Instant),
    /// `APEX-T5.4`: PRESENTATION wind. Receipt-time interpolated, exactly
    /// as before, and now barred from the prediction path — nothing that
    /// predicts reads this field.
    local_wind: Vec2<f32>,
    /// `APEX-T5.4`: the authoritative snapshots, keyed by `T0.87`'s
    /// weather epoch. This is what prediction reads.
    snapshots: WeatherSnapshotStoreV1,
    /// The most recent snapshot the server has named.
    latest_snapshot: Option<WeatherSnapshotIdV1>,
}

impl WeatherLerp {
    fn local_wind_update(&mut self, wind: Vec2<f32>, snapshot: WeatherSnapshotIdV1) {
        // The authoritative record first: it is keyed by the snapshot the
        // server named, and carries no arrival time at all.
        self.snapshots.record_v1(snapshot, wind);
        self.latest_snapshot = Some(snapshot);
        // Then the presentation record, which is the only thing that
        // knows what time the packet arrived.
        self.old_local_wind = mem::replace(&mut self.new_local_wind, (wind, Instant::now()));
    }

    /// `APEX-T5.4`: the wind a predicted frame may use.
    ///
    /// `PredictionWindSourceV1::Unavailable` when the snapshot is gone —
    /// the caller snaps rather than extrapolating, because an
    /// extrapolation's input is elapsed wall-clock time, which is the
    /// dependency this whole split removes.
    fn prediction_wind(&self) -> PredictionWindSourceV1 {
        match self.latest_snapshot {
            Some(id) => self.snapshots.wind_at_v1(id),
            None => PredictionWindSourceV1::Unavailable,
        }
    }

    fn update_local_wind(&mut self) {
        // Assumes updates are regular
        let t = (self.new_local_wind.1.elapsed().as_secs_f32()
            / self
                .new_local_wind
                .1
                .duration_since(self.old_local_wind.1)
                .as_secs_f32())
        .clamp(0.0, 1.0);

        self.local_wind = Vec2::lerp_unclamped(self.old_local_wind.0, self.new_local_wind.0, t);
    }

    fn weather_update(&mut self, weather: SharedWeatherGrid, snapshot: WeatherSnapshotIdV1) {
        self.latest_snapshot = Some(snapshot);
        self.old = mem::replace(&mut self.new, (weather, Instant::now()));
    }

    // TODO: Make improvements to this interpolation, it's main issue is assuming
    // that updates come at regular intervals.
    fn update(&mut self, to_update: &mut WeatherGrid) {
        prof_span!("WeatherLerp::update");
        self.update_local_wind();
        let old = &self.old.0;
        let new = &self.new.0;
        if new.size() == Vec2::zero() {
            return;
        }
        if to_update.size() != new.size() {
            *to_update = WeatherGrid::from(new);
        }
        if old.size() == new.size() {
            // Assumes updates are regular
            let t = (self.new.1.elapsed().as_secs_f32()
                / self.new.1.duration_since(self.old.1).as_secs_f32())
            .clamp(0.0, 1.0);

            to_update
                .iter_mut()
                .zip(old.iter().zip(new.iter()))
                .for_each(|((_, current), ((_, old), (_, new)))| {
                    *current = CompressedWeather::lerp_unclamped(old, new, t);
                    // `APEX-T5.4`: the grid is what physics and glider
                    // prediction read, so it gets the AUTHORITATIVE
                    // snapshot wind, not the receipt-time lerp. When no
                    // snapshot is retained the previous value stands —
                    // a snap — rather than an extrapolation, which is how
                    // the wall-clock dependency would get back in.
                    //
                    // The receipt-time value survives as `local_wind` for
                    // presentation only; see `Client::presentation_wind`.
                    if let Some(authoritative) = self.prediction_wind().wind_v1() {
                        current.wind = authoritative;
                    }
                });
        }
    }
}

impl Default for WeatherLerp {
    fn default() -> Self {
        let old = Instant::now();
        let new = Instant::now();
        Self {
            old: (SharedWeatherGrid::new(Vec2::zero()), old),
            new: (SharedWeatherGrid::new(Vec2::zero()), new),
            old_local_wind: (Vec2::zero(), old),
            new_local_wind: (Vec2::zero(), new),
            local_wind: Vec2::zero(),
            // 64 snapshots at the weather cadence is minutes of history:
            // far more than any replay window, and bounded so a long
            // disconnect cannot replay against a snapshot nobody has.
            snapshots: WeatherSnapshotStoreV1::new_v1(64),
            latest_snapshot: None,
        }
    }
}


/// `APEX-T3.4.20c`: one drained semantic frame -- the payload plus the
/// checkpoint binding it arrived under. `None` means unfenced.
struct DrainedFrameV1 {
    msg: ServerGeneral,
    checkpoint: Option<common_net::msg::checkpoint::CheckpointedEnvelopeContextV1>,
    sequence: u64,
}

/// `APEX-T3.4.20c`: the receiver's whole checkpoint runtime. Present only
/// when a deployment supplied a resource profile; absent means the
/// checkpoint path is off and checkpointed traffic is refused.
struct ClientCheckpointRuntimeV1 {
    profile: common_net::msg::checkpoint::CheckpointResourceProfileV1,
    chronology: common_net::msg::checkpoint::CheckpointChronologyV1,
    phase: common_net::msg::checkpoint::ClientCheckpointStateV1,
    aligner: Option<common_net::msg::checkpoint::CheckpointAlignerV1>,
    /// Frames accepted for the checkpoint in flight -- the receiver's own
    /// count, which `prepare_checkpoint_v1` must not infer.
    staged_events: u32,
}

/// Collects a committed checkpoint's records so they can be handed to the
/// ordinary per-stream handlers. Those handlers ARE the client's apply
/// step; a handler error tears the connection down, exactly as it does
/// for any other message today.
#[derive(Default)]
struct CheckpointApplyCollectorV1 {
    applied: Vec<common_net::msg::checkpoint::PreparedOpV1>,
    committed: Option<(u64, [u8; 32])>,
}

impl common_net::msg::checkpoint::CheckpointApplySinkV1 for CheckpointApplyCollectorV1 {
    fn apply_record_v1(&mut self, op: &common_net::msg::checkpoint::PreparedOpV1) {
        self.applied.push(op.clone());
    }

    fn checkpoint_committed_v1(&mut self, epoch: u64, descriptor_root: [u8; 32]) {
        self.committed = Some((epoch, descriptor_root));
    }
}

pub struct Client {
    client_type: ClientType,
    registered: bool,
    presence: Option<PresenceKind>,
    runtime: Arc<Runtime>,
    server_info: ServerInfo,
    /// `APEX-T3.3.06`: `Some` only while a `NetEnvelopeV1` attachment is
    /// active -- mirrors the server-side `Client::semantic_send_state`
    /// (client-to-server direction: this client sending `ClientGeneral`
    /// to the server). Dormant: nothing advances this yet (`T3.3.07`).
    semantic_send_state: Option<common_net::msg::SemanticSendStateV1>,
    /// Server-to-client direction (receiving `ServerGeneral`/`ServerInit`
    /// from the server). Dormant: nothing advances this yet (`T3.3.10`).
    semantic_receive_state: Option<common_net::msg::SemanticReceiveStateV1>,
    /// `T3.3.18`: redacted (reason/stream)-keyed ingress counters, the
    /// client-side counterpart of the server's own `SemanticIngressMetricsV1`
    /// resource -- process-lifetime, not per-attachment (unlike
    /// `semantic_receive_state`, this survives a resume/reconnect).
    semantic_ingress_metrics: common_net::msg::envelope::SemanticIngressMetricsV1,
    /// `APEX-T3.4.20c`: `Some` only when the deployment supplied a
    /// checkpoint resource profile -- the rollout gate, mirroring
    /// `semantic_send_state.is_some()`. There is no production profile
    /// yet (`production_checkpoint_profile_v1` refuses), so this is
    /// `None` on real traffic and every checkpointed frame is refused
    /// rather than half-handled.
    checkpoint_runtime: Option<ClientCheckpointRuntimeV1>,
    /// Localized server motd and rules
    server_description: ServerDescription,
    world_data: WorldData,
    weather: WeatherLerp,
    player_list: HashMap<Uid, PlayerInfo>,
    character_list: CharacterList,
    character_being_deleted: Option<CharacterId>,
    sites: HashMap<SiteId, SiteMarker>,
    extra_markers: Vec<Marker>,
    possible_starting_sites: Vec<SiteId>,
    pois: Vec<PoiInfo>,
    pub chat_mode: ChatMode,
    component_recipe_book: ComponentRecipeBook,
    available_recipes: HashMap<String, Option<SpriteKind>>,
    lod_zones: HashMap<Vec2<i32>, lod::Zone>,
    lod_last_requested: Option<Instant>,
    lod_pos_fallback: Option<Vec2<f32>>,
    /// `APEX-T3.6`: the physics correction generation this client is
    /// predicting under, adopted from the server's `CompSync` and echoed
    /// on every state report. Typed, so a stale value cannot pass the
    /// server's gate by comparing equal after a wrap.
    force_update_generation: common::apex::physics_generation::PhysicsGenerationV1,
    /// `APEX-T7.3a`: the client's own predicted-frame buffer, budgeted
    /// per T7.1 Decision 5. Populated once per tick, for `self.entity()`
    /// only -- predicting another entity's future inputs makes no sense,
    /// the client doesn't know them. `T7.3b` (gated separately) is the
    /// consumer that replays it; today it is populated but never
    /// replayed, so this row changes nothing about how a correction is
    /// applied.
    ///
    /// Capacity note (disclosed, not silently assumed): Decision 5 says
    /// "500ms of history, at the SERVER tick rate", but the client's own
    /// tick cadence is render-loop-driven, not fixed -- `Client::tick`
    /// runs once per frame at whatever rate the frontend calls it, so a
    /// fixed entry COUNT can only approximate a fixed DURATION. Sized
    /// generously (128 entries, ~500ms even at a 240Hz tick rate) rather
    /// than exactly; the 64KiB byte budget below is the harder limit in
    /// practice (Decision 5's own text treats budget-exceeded as the
    /// severe case -- snap and record -- and duration-exceeded as the
    /// routine one -- drop the oldest and keep going). A precise
    /// time-based eviction using each frame's own stored `time` field
    /// would remove the approximation; not attempted this row.
    prediction_buffer: common::apex::prediction_boundary::ClientPredictionBufferV1,
    /// `APEX-T7.3a` Decision 3: whether `self.entity()` was mounted
    /// (rider or volume-rider) as of the last tick. Compared against the
    /// current tick to detect a TRANSITION (entering or leaving a
    /// mount), which terminates the prediction history -- a local,
    /// client-detected boundary, not a server-issued generation, so it
    /// clears via `clear_v1` rather than `adopt_generation_v1`.
    was_mounted_last_tick: bool,
    // DET-NET-011/012 (v6, stage 1): newest server sync tick seen across
    // the replication streams (the chronology witness).
    last_server_sync_tick: u64,
    /// `APEX-T7.4` item A: correction-magnitude accounting, recorded on
    /// every `Replayed` reconciliation outcome. See
    /// `common_systems::reconciliation::CorrectionMagnitudeMetricsV1`'s
    /// own doc for the T5.1-shape reuse and its scope boundary.
    correction_magnitude_metrics: common_systems::reconciliation::CorrectionMagnitudeMetricsV1,

    role: Option<AdminRole>,
    max_group_size: u32,
    // Client has received an invite (inviter uid, time out instant)
    invite: Option<(Uid, Instant, Duration, InviteKind)>,
    group_leader: Option<Uid>,
    // Note: potentially representable as a client only component
    group_members: HashMap<Uid, group::Role>,
    // Pending invites that this client has sent out
    pending_invites: HashSet<Uid>,
    // The pending trade the client is involved in, and it's id
    pending_trade: Option<(TradeId, PendingTrade, Option<SitePrices>)>,
    waypoint: Option<String>,

    network: Option<Network>,
    participant: Option<Participant>,
    general_stream: Stream,
    ping_stream: Stream,
    register_stream: Stream,
    character_screen_stream: Stream,
    in_game_stream: Stream,
    terrain_stream: Stream,

    client_timeout: Duration,
    last_server_ping: f64,
    last_server_pong: f64,
    last_ping_delta: f64,
    ping_deltas: VecDeque<f64>,

    tick: u64,
    state: State,

    flashing_lights_enabled: bool,

    /// Terrrain view distance
    server_view_distance_limit: Option<u32>,
    view_distance: Option<u32>,
    lod_distance: f32,
    // TODO: move into voxygen
    loaded_distance: f32,

    pending_chunks: HashMap<Vec2<i32>, Instant>,
    /// bastion (B1.6): overseer god-camera terrain anchor. When set, terrain
    /// chunks are requested/retained around it instead of the entity position
    /// (the entity's immediate area is still retained). Mirrored to the server
    /// (`ClientGeneral::BastionCameraAnchor`) so request validation accepts it.
    bastion_terrain_anchor: Option<Vec3<f32>>,
    /// bastion (B2a): designations echoed back by the server (validated).
    /// Rendered as an overlay by voxygen; replaced by real job-board state in
    /// B4. B5.5: removals subtract from stored rects (AABB subtraction), so
    /// the list shrinks/splits too — `bastion_designations_rev` bumps on any
    /// change and voxygen rebuilds its overlay shapes when it moves.
    // B5.6b-2: the third field is the surface-relative extent for volume
    // rendering (`None` = legacy literal region). Erase subtraction pieces
    // inherit their parent's extent (splits are XY-shaped in practice — the
    // erase path cancels at each rect's own full z-range).
    bastion_designations: Vec<(
        common::bastion::Region,
        common::bastion::DesignationKind,
        Option<common::bastion::ZExtent>,
    )>,
    bastion_designations_rev: u64,
    /// bastion (UI-4 row 62 → UI-5 row 62.2): the latest inspector reply —
    /// (target, payload). `payload: None` = nothing Bastion-tracked sits at
    /// the target (the panel renders nothing). Overwritten per reply; the
    /// HUD polls at ~1Hz while a panel is open.
    bastion_inspect: Option<(
        comp::bastion::BastionInspectTarget,
        Option<comp::bastion::BastionInspectKind>,
    )>,
    target_time_of_day: Option<TimeOfDay>,
    dt_adjustment: f64,

    connected_server_constants: ServerConstants,
    /// Requested but not yet received plugins
    missing_plugins: HashSet<PluginHash>,
    /// Locally cached plugins needed by the server
    local_plugins: Vec<PathBuf>,
}

/// Holds data related to the current players characters, as well as some
/// additional state to handle UI.
#[derive(Debug, Default)]
pub struct CharacterList {
    pub characters: Vec<CharacterItem>,
    pub loading: bool,
}

async fn connect_quic(
    network: &Network,
    hostname: String,
    override_port: Option<u16>,
    prefer_ipv6: bool,
    validate_tls: bool,
) -> Result<network::Participant, crate::error::Error> {
    let config = if validate_tls {
        quinn::ClientConfig::try_with_platform_verifier()?
    } else {
        warn!(
            "skipping validation of server identity. There is no guarantee that the server you're \
             connected to is the one you expect to be connecting to."
        );
        #[derive(Debug)]
        struct Verifier;
        impl rustls::client::danger::ServerCertVerifier for Verifier {
            fn verify_server_cert(
                &self,
                _end_entity: &rustls::pki_types::CertificateDer<'_>,
                _intermediates: &[rustls::pki_types::CertificateDer<'_>],
                _server_name: &rustls::pki_types::ServerName<'_>,
                _ocsp_response: &[u8],
                _now: rustls::pki_types::UnixTime,
            ) -> Result<ServerCertVerified, rustls::Error> {
                Ok(ServerCertVerified::assertion())
            }

            fn verify_tls12_signature(
                &self,
                _message: &[u8],
                _cert: &rustls::pki_types::CertificateDer<'_>,
                _dss: &rustls::DigitallySignedStruct,
            ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error>
            {
                Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
            }

            fn verify_tls13_signature(
                &self,
                _message: &[u8],
                _cert: &rustls::pki_types::CertificateDer<'_>,
                _dss: &rustls::DigitallySignedStruct,
            ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error>
            {
                Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
            }

            fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
                vec![
                    rustls::SignatureScheme::RSA_PKCS1_SHA1,
                    rustls::SignatureScheme::ECDSA_SHA1_Legacy,
                    rustls::SignatureScheme::RSA_PKCS1_SHA256,
                    rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
                    rustls::SignatureScheme::RSA_PKCS1_SHA384,
                    rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
                    rustls::SignatureScheme::RSA_PKCS1_SHA512,
                    rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
                    rustls::SignatureScheme::RSA_PSS_SHA256,
                    rustls::SignatureScheme::RSA_PSS_SHA384,
                    rustls::SignatureScheme::RSA_PSS_SHA512,
                    rustls::SignatureScheme::ED25519,
                    rustls::SignatureScheme::ED448,
                ]
            }
        }

        let mut cfg = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(Verifier))
            .with_no_client_auth();
        cfg.enable_early_data = true;

        quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(cfg).unwrap(),
        ))
    };

    addr::try_connect(network, &hostname, override_port, prefer_ipv6, |a| {
        ConnectAddr::Quic(a, config.clone(), hostname.clone())
    })
    .await
}

/// `APEX-T7.3c-ii`: read the seven `RollingStateV1` fields directly from
/// `state`'s live ECS storages for `entity`. `None` if any is missing —
/// a predicted entity is expected to carry all seven (the same
/// components `character_behavior::Sys`'s live join and
/// `replay_predicted_frame_v1` both require).
fn read_rolling_state_v1(
    state: &common_state::State,
    entity: specs::Entity,
) -> Option<common_systems::character_behavior::RollingStateV1> {
    Some(common_systems::character_behavior::RollingStateV1 {
        char_state: state.read_storage::<comp::CharacterState>().get(entity)?.clone(),
        character_activity: state.read_storage::<comp::CharacterActivity>().get(entity)?.clone(),
        pos: *state.read_storage::<comp::Pos>().get(entity)?,
        vel: *state.read_storage::<comp::Vel>().get(entity)?,
        ori: *state.read_storage::<comp::Ori>().get(entity)?,
        density: *state.read_storage::<comp::Density>().get(entity)?,
        energy: *state.read_storage::<comp::Energy>().get(entity)?,
    })
}

/// `APEX-T7.3c-ii`: write a replayed `RollingStateV1` back into `state`'s
/// live ECS storages for `entity` — superseding the raw authoritative
/// snapshot `apply_comp_sync_package` already wrote verbatim (the LAW's
/// write-verbatim half, satisfied before this ever runs) with the
/// client's own not-yet-acknowledged inputs re-applied on top of it.
fn write_rolling_state_v1(
    state: &common_state::State,
    entity: specs::Entity,
    rolling: &common_systems::character_behavior::RollingStateV1,
) {
    // `CharacterState`/`CharacterActivity`/`Density`/`Energy` use
    // flagged storage (`DerefFlaggedStorage`) -- `get_mut` returns a
    // `FlaggedAccessMut`, not a plain `&mut T`, and writing through its
    // `DerefMut` needs the LOCAL BINDING itself to be `mut` (this is the
    // exact same storage-flagging `T7.3b`'s `JoinFieldMut` finding
    // covers: these are the same four fields). `Pos`/`Vel`/`Ori` are
    // plain storage and don't need it.
    if let Some(mut c) = state.ecs().write_storage::<comp::CharacterState>().get_mut(entity) {
        *c = rolling.char_state.clone();
    }
    if let Some(mut c) = state.ecs().write_storage::<comp::CharacterActivity>().get_mut(entity) {
        *c = rolling.character_activity.clone();
    }
    if let Some(c) = state.ecs().write_storage::<comp::Pos>().get_mut(entity) {
        *c = rolling.pos;
    }
    if let Some(c) = state.ecs().write_storage::<comp::Vel>().get_mut(entity) {
        *c = rolling.vel;
    }
    if let Some(c) = state.ecs().write_storage::<comp::Ori>().get_mut(entity) {
        *c = rolling.ori;
    }
    if let Some(mut c) = state.ecs().write_storage::<comp::Density>().get_mut(entity) {
        *c = rolling.density;
    }
    if let Some(mut c) = state.ecs().write_storage::<comp::Energy>().get_mut(entity) {
        *c = rolling.energy;
    }
}

impl Client {
    pub async fn new(
        addr: ConnectionArgs,
        runtime: Arc<Runtime>,
        // TODO: refactor to avoid needing to use this out parameter
        mismatched_server_info: &mut Option<ServerInfo>,
        username: &str,
        password: &str,
        locale: Option<String>,
        auth_trusted: impl FnMut(&str) -> bool,
        init_stage_update: &(dyn Fn(ClientInitStage) + Send + Sync),
        add_foreign_systems: impl Fn(&mut DispatcherBuilder) + Send + 'static,
        #[cfg_attr(not(feature = "plugins"), expect(unused_variables))] config_dir: PathBuf,
        client_type: ClientType,
    ) -> Result<Self, Error> {
        let _ = rustls::crypto::ring::default_provider().install_default(); // needs to be initialized before usage
        // Use `usize::MAX` as the output limit: we implicitly trust servers to not send
        // us too much data (TODO: should we?)
        let network = Network::new(Pid::new(), &runtime);

        init_stage_update(ClientInitStage::ConnectionEstablish);

        let mut participant = match addr {
            ConnectionArgs::Srv {
                hostname,
                prefer_ipv6,
                validate_tls,
                use_quic,
            } => {
                // Try to create a resolver backed by /etc/resolv.conf or the Windows Registry
                // first. If that fails, create a resolver being hard-coded to
                // Google's 8.8.8.8 public resolver.
                let resolver = Resolver::builder_tokio()
                    .unwrap_or_else(|error| {
                        error!(
                            "Failed to create DNS resolver using system configuration: {error:?}"
                        );
                        warn!("Falling back to a default configured resolver.");
                        Resolver::builder_with_config(
                            ResolverConfig::default(),
                            TokioRuntimeProvider::default(),
                        )
                    })
                    .build()
                    .expect(
                        "Could not get a Hickory DNS resolver, maybe you are missing some tls libs",
                    );

                let quic_service_host = format!("_veloren._udp.{hostname}");
                let quic_lookup_future = resolver.srv_lookup(quic_service_host);
                let tcp_service_host = format!("_veloren._tcp.{hostname}");
                let tcp_lookup_future = resolver.srv_lookup(tcp_service_host);
                let (quic_rr, tcp_rr) = tokio::join!(quic_lookup_future, tcp_lookup_future);

                #[derive(Eq, PartialEq)]
                enum ConnMode {
                    Quic,
                    Tcp,
                }

                // Push the results of both futures into `srv_rr`. This uses map_or_else purely
                // for side effects.
                let mut srv_rr = Vec::new();
                let () = quic_rr.map_or_else(
                    |error| {
                        warn!("QUIC SRV lookup failed: {error:?}");
                    },
                    |srv_lookup| {
                        srv_rr.extend(srv_lookup.answers().iter().filter_map(|record| {
                            if let RData::SRV(srv) = &record.data {
                                Some((ConnMode::Quic, srv.clone()))
                            } else {
                                None
                            }
                        }))
                    },
                );
                let () = tcp_rr.map_or_else(
                    |error| {
                        warn!("TCP SRV lookup failed: {error:?}");
                    },
                    |srv_lookup| {
                        srv_rr.extend(srv_lookup.answers().iter().filter_map(|record| {
                            if let RData::SRV(srv) = &record.data {
                                Some((ConnMode::Tcp, srv.clone()))
                            } else {
                                None
                            }
                        }))
                    },
                );

                // SRV records have a priority; lowest priority hosts MUST be contacted first.
                let srv_rr_slice = srv_rr.as_mut_slice();
                srv_rr_slice.sort_by_key(|(_, srv)| srv.priority);

                let mut iter = srv_rr_slice.iter();

                // This loops exits as soon as the above iter over `srv_rr_slice` is exhausted
                loop {
                    if let Some((conn_mode, srv_rr)) = iter.next() {
                        let hostname = format!("{}", srv_rr.target);
                        let port = Some(srv_rr.port);
                        let conn_result = match conn_mode {
                            ConnMode::Quic => {
                                connect_quic(&network, hostname, port, prefer_ipv6, validate_tls)
                                    .await
                            },
                            ConnMode::Tcp => {
                                addr::try_connect(
                                    &network,
                                    &hostname,
                                    port,
                                    prefer_ipv6,
                                    ConnectAddr::Tcp,
                                )
                                .await
                            },
                        };
                        match conn_result {
                            Ok(c) => break c,
                            Err(error) => {
                                warn!("Failed to connect to host {}: {error:?}", srv_rr.target)
                            },
                        }
                    } else {
                        warn!(
                            "No SRV hosts succeeded connection, falling back to direct connection"
                        );
                        // This case is also hit if no SRV host was returned from the query, so we
                        // check for QUIC/TCP preference.
                        let c = if use_quic {
                            connect_quic(&network, hostname, None, prefer_ipv6, validate_tls)
                                .await?
                        } else {
                            match addr::try_connect(
                                &network,
                                &hostname,
                                None,
                                prefer_ipv6,
                                ConnectAddr::Tcp,
                            )
                            .await
                            {
                                Ok(c) => c,
                                Err(error) => return Err(error),
                            }
                        };
                        break c;
                    }
                }
            },
            ConnectionArgs::Tcp {
                hostname,
                prefer_ipv6,
            } => {
                addr::try_connect(&network, &hostname, None, prefer_ipv6, ConnectAddr::Tcp).await?
            },
            ConnectionArgs::Quic {
                hostname,
                prefer_ipv6,
                validate_tls,
            } => {
                warn!(
                    "QUIC is enabled. This is experimental and you won't be able to connect to \
                     TCP servers unless deactivated"
                );

                connect_quic(&network, hostname, None, prefer_ipv6, validate_tls).await?
            },
            ConnectionArgs::Mpsc(id) => network.connect(ConnectAddr::Mpsc(id)).await?,
        };

        #[cfg_attr(not(feature = "plugins"), expect(unused_mut))]
        let mut stream = participant.opened().await?;
        let ping_stream = participant.opened().await?;
        let mut register_stream = participant.opened().await?;
        let character_screen_stream = participant.opened().await?;
        let in_game_stream = participant.opened().await?;
        let terrain_stream = participant.opened().await?;

        init_stage_update(ClientInitStage::WatingForServerVersion);
        register_stream.send(client_type)?;
        let server_info: ServerInfo = register_stream.recv().await?;
        if server_info.git_hash != *common::util::GIT_HASH
            || server_info.git_timestamp != *common::util::GIT_TIMESTAMP
        {
            warn!(
                "Server is running {}, you are running {}, versions might be incompatible!",
                common::util::make_display_version(server_info.git_hash, server_info.git_timestamp),
                *common::util::DISPLAY_VERSION,
            );
        }
        // Pass the server info back to the caller to ensure they can access it even
        // if this function errors.
        *mismatched_server_info = Some(server_info.clone());
        debug!("Auth Server: {:?}", server_info.auth_provider);

        ping_stream.send(PingMsg::Ping)?;

        init_stage_update(ClientInitStage::Authentication);
        // Register client
        let register_session_binding = Self::register(
            username,
            password,
            locale,
            auth_trusted,
            &server_info,
            &mut register_stream,
        )
        .await?;

        init_stage_update(ClientInitStage::LoadingInitData);
        // Wait for initial sync
        let mut ping_interval = tokio::time::interval(Duration::from_secs(1));

        // `T4.1` chunk 2b: `ServerGeneral::BootstrapManifest` always
        // precedes `GameSync` on this same stream
        // (`server/src/sys/msg/register.rs::finalize_admission`), sent via
        // the legacy path regardless of the negotiated semantic protocol
        // (chunk 2a's own routing fix -- register stream is what makes a
        // send BEFORE GameSync possible at all; general-stream would not
        // have). Received and validated BEFORE `GameSync` is even
        // awaited: `State::client` construction below this point is
        // therefore only reachable once a compatible manifest has been
        // seen, the ordering invariant this exists to enforce (`BOOT-005`
        // if the wrong message arrives or fails to decode, `BOOT-006` if
        // it decodes but a slot disagrees -- see `error.rs`).
        let bootstrap_manifest_msg: ServerGeneral = loop {
            tokio::select! {
                res = register_stream.recv() => break res?,
                _ = ping_interval.tick() => ping_stream.send(PingMsg::Ping)?,
            }
        };
        let bootstrap_manifest_wire = crate::error::expect_bootstrap_manifest(bootstrap_manifest_msg)?;
        let bootstrap_manifest = crate::error::validate_bootstrap_manifest_v1(&bootstrap_manifest_wire)?;
        // `T4.2` chunk B (`BOOT-007`): freshness admission runs AFTER slot
        // validation, on the SAME manifest, before `GameSync` is awaited --
        // no second FSM surgery, cashing in chunk A's ledger the same way
        // chunk 2a/2b cashed in chunk 1's reserved wire field. The ledger
        // is fresh per connection attempt (parked scope, per the
        // orchestrator's ruling: cross-reconnect persistence is not yet
        // needed since the client persists nothing else across
        // reconnects either -- `server_boot_id` + a per-connection floor
        // covers today's surface).
        let mut bootstrap_freshness_ledger = common::apex::bootstrap_freshness::BootstrapFreshnessLedgerV1::new(
            server_info.server_boot_id,
            register_session_binding.session_id,
            register_session_binding.epoch,
        );
        crate::error::admit_bootstrap_freshness_v1(&mut bootstrap_freshness_ledger, &bootstrap_manifest)?;

        // `APEX-T3.3.16`: V1-envelope GameSync iff negotiation selected
        // `NetEnvelopeV1` -- packet: "Legacy keeps direct GameSync;
        // certified V1 requires envelope", "mode mixing terminates". A
        // rejected/raw frame here is a hard bootstrap failure (`?`
        // propagates out), unlike a dropped later-game replication frame
        // (`handle_messages`'s own reject path just warns and drops one
        // frame) -- there is no partial/degraded way to proceed without
        // a validated initial GameSync.
        let game_sync: ServerInit = if register_session_binding.selected_semantic_protocol
            == common_net::msg::SemanticProtocolIdV1::NetEnvelopeV1
        {
            let receive_state = common_net::msg::envelope::SemanticReceiveStateV1::new(common_net::msg::ActiveSessionBindingV1 {
                server_boot_id: server_info.server_boot_id,
                session_id: register_session_binding.session_id,
                epoch: register_session_binding.epoch,
            });
            let raw: Vec<u8> = loop {
                tokio::select! {
                    res = register_stream.recv::<Vec<u8>>() => break res?,
                    _ = ping_interval.tick() => ping_stream.send(PingMsg::Ping)?,
                }
            };
            // The causality half is discarded here -- this `receive_state`
            // is a throwaway, local-to-bootstrap value (the real,
            // persistent one is built later once `Client` exists), so
            // committing a snapshot watermark to it would be pointless.
            Self::validate_semantic_frame_v1::<ServerInit>(
                &raw,
                &receive_state,
                common_net::msg::envelope::SemanticStreamIdV1::Bootstrap,
            )
            .map_err(|reject| Error::Other(format!("semantic V1 GameSync envelope rejected: {reject:?}")))?
            .0
        } else {
            loop {
                tokio::select! {
                    res = register_stream.recv() => break res?,
                    _ = ping_interval.tick() => ping_stream.send(PingMsg::Ping)?,
                }
            }
        };
        let ServerInit::GameSync {
            server_boot_id: game_sync_server_boot_id,
            entity_package,
            time_of_day,
            max_group_size,
            client_timeout,
            world_map,
            recipe_book,
            component_recipe_book,
            material_stats,
            ability_map,
            server_constants,
            description,
            active_plugins: _active_plugins,
            // APEX-T2.5.11: consumed by the acquisition-before-State flow
            // when it lands; a Some from a newer server is noted, not an
            // error (legacy hash path still runs).
            plugin_deployment: _plugin_deployment,
            role,
            session_binding: game_sync_session_binding,
        } = game_sync;

        // APEX-T3.1.12: compare GameSync's boot ID against the ServerInfo
        // observation before constructing State/PlayerEntity/plugin
        // readiness -- a server restart between registration and this
        // bootstrap message must not mix state across incarnations.
        crate::error::check_game_sync_boot_scope(server_info.server_boot_id, game_sync_server_boot_id)?;
        // APEX-T3.2: same shape, new field -- RegisterAnswer and GameSync
        // must carry the identical SessionBindingV1, checked before
        // constructing State (spec section 3.5, canaries SES-045/046).
        crate::error::check_session_binding_equality(register_session_binding, game_sync_session_binding)?;
        // APEX-T3.3.06: "accepted binding initializes" -- mirrors
        // server/src/sys/msg/register.rs::finalize_admission's own reset,
        // computed from the just-verified GameSync binding (identical to
        // register_session_binding per the equality check above).
        let semantic_state_binding = (game_sync_session_binding.selected_semantic_protocol
            == common_net::msg::SemanticProtocolIdV1::NetEnvelopeV1)
            .then(|| common_net::msg::ActiveSessionBindingV1 {
                server_boot_id: game_sync_server_boot_id,
                session_id: game_sync_session_binding.session_id,
                epoch: game_sync_session_binding.epoch,
            });

        // APEX-T2.5.11 — acquisition BEFORE State: when the server sent a
        // typed deployment summary, every client-active artifact is
        // verified into the cache and the plugin manager is built from
        // those verified files BEFORE `State::client` exists. No
        // `load_server_plugin` after State on this path — and per
        // `RejectLocalPlugins`, local extra plugins are NOT loaded when a
        // deployment governs the session.
        #[cfg(feature = "plugins")]
        let deployment_plugin_mgr: Option<common_state::plugin::PluginMgr> = match &_plugin_deployment {
            None => None,
            Some(summary) => {
                use common::apex::digest::{
                    ArtifactDigestV1, ArtifactIdentityV1, DigestAlgorithmIdV1, DigestBytes32V1,
                };
                use common_net::msg::plugin_artifact::PluginArtifactRequestV1;
                use common_state::plugin::artifact_cache::PluginArtifactCacheV1;
                // APEX-T2.5.22 — schema/completeness refusals BEFORE any
                // acquisition: every client-active ordinal must have
                // exactly one requirement (an incomplete or duplicated
                // set is a typed init error, never a partial bootstrap).
                for ordinal in &summary.client_activations {
                    let n = summary.requirements.iter().filter(|r| r.ordinal == *ordinal).count();
                    if n != 1 {
                        return Err(Error::Other(format!(
                            "plugin deployment summary incomplete: ordinal {ordinal} has {n} requirements"
                        )));
                    }
                }
                // Wire bytes are never trusted as identity: the cache
                // re-verifies size+digest on stage AND on read.
                let reqs: Vec<(u32, ArtifactIdentityV1)> = summary
                    .requirements
                    .iter()
                    .filter(|r| summary.client_activations.contains(&r.ordinal))
                    .map(|r| {
                        (r.ordinal, ArtifactIdentityV1 {
                            digest: ArtifactDigestV1 {
                                algorithm: DigestAlgorithmIdV1::Sha256,
                                bytes: DigestBytes32V1::from_array(r.digest),
                            },
                            size_bytes: r.size_bytes,
                        })
                    })
                    .collect();
                let cache =
                    PluginArtifactCacheV1::new(config_dir.join("plugin_artifact_cache_v1"), reqs.clone())
                        .map_err(|e| Error::Other(format!("plugin artifact cache: {e:?}")))?;
                let missing: Vec<u32> =
                    reqs.iter().map(|(o, _)| *o).filter(|o| !cache.is_staged_verified(*o)).collect();
                if !missing.is_empty() {
                    tracing::info!(?missing, "requesting deployment artifacts before State");
                    stream.send(ClientGeneral::RequestPluginArtifacts(PluginArtifactRequestV1 {
                        deployment_root: summary.deployment_root,
                        ordinals: missing.clone(),
                    }))?;
                    let mut outstanding: std::collections::BTreeSet<u32> = missing.into_iter().collect();
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
                    while !outstanding.is_empty() {
                        let msg: ServerGeneral = tokio::select! {
                            res = stream.recv() => res?,
                            _ = ping_interval.tick() => { ping_stream.send(PingMsg::Ping)?; continue; },
                            _ = tokio::time::sleep_until(deadline) => {
                                return Err(Error::Other(format!(
                                    "plugin artifact acquisition timed out; outstanding: {outstanding:?}"
                                )));
                            },
                        };
                        match msg {
                            ServerGeneral::PluginArtifactData(resp) => {
                                let ordinal = resp.descriptor.ordinal;
                                cache.stage(ordinal, &resp.bytes).map_err(|e| {
                                    Error::Other(format!("plugin artifact {ordinal} refused: {e:?}"))
                                })?;
                                outstanding.remove(&ordinal);
                            },
                            ServerGeneral::Disconnect(reason) => {
                                return Err(Error::Other(format!(
                                    "server disconnected during plugin acquisition: {reason:?}"
                                )));
                            },
                            other => {
                                // Disclosed .11 limitation: general-stream
                                // traffic in this brief pre-State window is
                                // dropped with a log (buffer-and-replay is
                                // .12+ scope). Only entered when the server
                                // opted into V1 deployments.
                                tracing::warn!(
                                    "dropping general message during plugin acquisition: {}",
                                    core::any::type_name_of_val(&other)
                                );
                            },
                        }
                    }
                }
                let paths: Vec<(u32, PathBuf)> = reqs
                    .iter()
                    .map(|(o, _)| {
                        cache
                            .verified_path(*o)
                            .map(|p| (*o, p))
                            .map_err(|e| Error::Other(format!("plugin artifact {o} unavailable: {e:?}")))
                    })
                    .collect::<Result<_, _>>()?;
                let expected: Vec<(u32, [u8; 32])> =
                    reqs.iter().map(|(o, a)| (*o, *a.digest.bytes.as_array())).collect();
                Some(
                    common_state::plugin::PluginMgr::from_deployment_paths_v1(
                        paths,
                        &expected,
                        summary.deployment_root,
                        Some(common_state::plugin::module::PluginStoreLimitsV1 {
                            max_linear_memory_bytes: summary.client_runtime_limits.max_linear_memory_bytes,
                            max_fuel_per_event: summary.client_runtime_limits.max_fuel_per_event,
                        }),
                        Some(summary.client_runtime_limits.max_instances),
                        Some(summary.command_owners.iter().cloned().collect()),
                        Some(summary.skeleton_owners.iter().cloned().collect()),
                    )
                    .map_err(|e| Error::Other(format!("deployment plugin batch failed: {e:?}")))?,
                )
            },
        };

        init_stage_update(ClientInitStage::StartingClient);
        #[cfg(feature = "plugins")]
        let deployment_governs = _plugin_deployment.is_some();
        // Spawn in a blocking thread (leaving the network thread free).  This is mostly
        // useful for bots.
        let mut task = tokio::task::spawn_blocking(move || {
            let map_size_lg =
                common::terrain::MapSizeLg::new(world_map.dimensions_lg).map_err(|_| {
                    Error::Other(format!(
                        "Server sent bad world map dimensions: {:?}",
                        world_map.dimensions_lg,
                    ))
                })?;
            let sea_level = world_map.default_chunk.get_min_z() as f32;

            // Initialize `State`
            let pools = State::pools(GameMode::Client);
            let mut state = State::client(
                pools,
                map_size_lg,
                world_map.default_chunk,
                // TODO: Add frontend systems
                |dispatch_builder| {
                    add_local_systems(dispatch_builder);
                    add_foreign_systems(dispatch_builder);
                },
                // APEX-T2.5.11: a governed deployment supplies the fully
                // verified manager; otherwise the legacy local-asset path
                // runs exactly as before.
                //
                // APEX (feature-invariance): the ARGUMENT is unconditional —
                // only its VALUE is feature-gated. A `#[cfg]` on the argument
                // itself made this call's arity depend on a feature, which
                // broke the moment cargo unified features across a combined
                // server+client build.
                {
                    #[cfg(feature = "plugins")]
                    {
                        common_state::StatePluginsV1::new(
                            deployment_plugin_mgr.unwrap_or_else(
                                common_state::plugin::PluginMgr::from_asset_or_default,
                            ),
                        )
                    }
                    #[cfg(not(feature = "plugins"))]
                    {
                        common_state::StatePluginsV1::none()
                    }
                },
            )
            .map_err(|e| Error::Other(format!("state construction failed: {e:?}")))?;

            #[cfg_attr(not(feature = "plugins"), expect(unused_mut))]
            let mut missing_plugins: Vec<PluginHash> = Vec::new();
            #[cfg_attr(not(feature = "plugins"), expect(unused_mut))]
            let mut local_plugins: Vec<PathBuf> = Vec::new();
            // APEX-T2.5.11: the legacy hash-list path runs ONLY when no
            // deployment governs the session (RejectLocalPlugins + no
            // late-load: a governed client's plugin set is exactly the
            // verified deployment, already complete before State).
            #[cfg(feature = "plugins")]
            if !deployment_governs {
                let already_present = state.ecs().read_resource::<PluginMgr>().plugin_list();
                for hash in _active_plugins.iter() {
                    if !already_present.contains(hash) {
                        // look in config_dir first (cache)
                        if let Ok(local_path) = common_state::plugin::find_cached(&config_dir, hash)
                        {
                            local_plugins.push(local_path);
                        } else {
                            //tracing::info!("cache not found {local_path:?}");
                            tracing::info!("Server requires plugin {hash:x?}");
                            missing_plugins.push(*hash);
                        }
                    }
                }
            }
            // Client-only components
            state.ecs_mut().register::<comp::Last<CharacterState>>();
            let entity = state.ecs_mut().apply_entity_package(entity_package);
            *state.ecs_mut().write_resource() = time_of_day;
            *state.ecs_mut().write_resource() = PlayerEntity(Some(entity));
            state.ecs_mut().insert(material_stats);
            state.ecs_mut().insert(ability_map);
            state.ecs_mut().insert(recipe_book);

            let map_size = map_size_lg.chunks();
            let max_height = world_map.max_height;
            let rgba = world_map.rgba;
            let alt = world_map.alt;
            if rgba.size() != map_size.map(|e| e as i32) {
                return Err(Error::Other("Server sent a bad world map image".into()));
            }
            if alt.size() != map_size.map(|e| e as i32) {
                return Err(Error::Other("Server sent a bad altitude map.".into()));
            }
            let [west, east] = world_map.horizons;
            let scale_angle = |a: u8| (a as f32 / 255.0 * <f32 as FloatConst>::FRAC_PI_2()).tan();
            let scale_height = |h: u8| h as f32 / 255.0 * max_height;
            let scale_height_big = |h: u32| (h >> 3) as f32 / 8191.0 * max_height;

            debug!("Preparing image...");
            let unzip_horizons = |(angles, heights): &(Vec<_>, Vec<_>)| {
                (
                    angles.iter().copied().map(scale_angle).collect::<Vec<_>>(),
                    heights
                        .iter()
                        .copied()
                        .map(scale_height)
                        .collect::<Vec<_>>(),
                )
            };
            let horizons = [unzip_horizons(&west), unzip_horizons(&east)];

            // Redraw map (with shadows this time).
            let mut world_map_rgba = vec![0u32; rgba.size().product() as usize];
            let mut world_map_topo = vec![0u32; rgba.size().product() as usize];
            let mut map_config = common::terrain::map::MapConfig::orthographic(
                map_size_lg,
                core::ops::RangeInclusive::new(0.0, max_height),
            );
            map_config.horizons = Some(&horizons);
            let rescale_height = |h: f32| h / max_height;
            let bounds_check = |pos: Vec2<i32>| {
                pos.reduce_partial_min() >= 0
                    && pos.x < map_size.x as i32
                    && pos.y < map_size.y as i32
            };
            fn sample_pos(
                map_config: &MapConfig,
                pos: Vec2<i32>,
                alt: &Grid<u32>,
                rgba: &Grid<u32>,
                map_size: &Vec2<u16>,
                map_size_lg: &common::terrain::MapSizeLg,
                max_height: f32,
            ) -> common::terrain::map::MapSample {
                let rescale_height = |h: f32| h / max_height;
                let scale_height_big = |h: u32| (h >> 3) as f32 / 8191.0 * max_height;
                let bounds_check = |pos: Vec2<i32>| {
                    pos.reduce_partial_min() >= 0
                        && pos.x < map_size.x as i32
                        && pos.y < map_size.y as i32
                };
                let MapConfig {
                    gain,
                    is_contours,
                    is_height_map,
                    is_stylized_topo,
                    ..
                } = *map_config;
                let mut is_contour_line = false;
                let mut is_border = false;
                let (rgb, alt, downhill_wpos) = if bounds_check(pos) {
                    let posi = pos.y as usize * map_size.x as usize + pos.x as usize;
                    let [r, g, b, _a] = rgba[pos].to_le_bytes();
                    let is_water = r == 0 && b > 102 && g < 77;
                    let alti = alt[pos];
                    // Compute contours (chunks are assigned in the river code below)
                    let altj = rescale_height(scale_height_big(alti));
                    let contour_interval = 150.0;
                    let chunk_contour = (altj * gain / contour_interval) as u32;

                    // Compute downhill.
                    let downhill = {
                        let mut best = -1;
                        let mut besth = alti;
                        for nposi in neighbors(*map_size_lg, posi) {
                            let nbh = alt.raw()[nposi];
                            let nalt = rescale_height(scale_height_big(nbh));
                            let nchunk_contour = (nalt * gain / contour_interval) as u32;
                            if !is_contour_line && chunk_contour > nchunk_contour {
                                is_contour_line = true;
                            }
                            let [nr, ng, nb, _na] = rgba.raw()[nposi].to_le_bytes();
                            let n_is_water = nr == 0 && nb > 102 && ng < 77;

                            if !is_border && is_water && !n_is_water {
                                is_border = true;
                            }

                            if nbh < besth {
                                besth = nbh;
                                best = nposi as isize;
                            }
                        }
                        best
                    };
                    let downhill_wpos = if downhill < 0 {
                        None
                    } else {
                        Some(
                            Vec2::new(
                                (downhill as usize % map_size.x as usize) as i32,
                                (downhill as usize / map_size.x as usize) as i32,
                            ) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
                        )
                    };
                    (Rgb::new(r, g, b), alti, downhill_wpos)
                } else {
                    (Rgb::zero(), 0, None)
                };
                let alt = f64::from(rescale_height(scale_height_big(alt)));
                let wpos = pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
                let downhill_wpos =
                    downhill_wpos.unwrap_or(wpos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32));
                let is_path = rgb.r == 0x37 && rgb.g == 0x29 && rgb.b == 0x23;
                let rgb = rgb.map(|e: u8| e as f64 / 255.0);
                let is_water = rgb.r == 0.0 && rgb.b > 0.4 && rgb.g < 0.3;

                let rgb = if is_height_map {
                    if is_path {
                        // Path color is Rgb::new(0x37, 0x29, 0x23)
                        Rgb::new(0.9, 0.9, 0.63)
                    } else if is_water {
                        Rgb::new(0.23, 0.47, 0.53)
                    } else if is_contours && is_contour_line {
                        // Color contour lines
                        Rgb::new(0.15, 0.15, 0.15)
                    } else {
                        // Color hill shading
                        let lightness = (alt + 0.2).min(1.0);
                        Rgb::new(lightness, 0.9 * lightness, 0.5 * lightness)
                    }
                } else if is_stylized_topo {
                    if is_path {
                        Rgb::new(0.9, 0.9, 0.63)
                    } else if is_water {
                        if is_border {
                            Rgb::new(0.10, 0.34, 0.50)
                        } else {
                            Rgb::new(0.23, 0.47, 0.63)
                        }
                    } else if is_contour_line {
                        Rgb::new(0.25, 0.25, 0.25)
                    } else {
                        // Stylized colors
                        Rgb::new(
                            (rgb.r + 0.25).min(1.0),
                            (rgb.g + 0.23).min(1.0),
                            (rgb.b + 0.10).min(1.0),
                        )
                    }
                } else {
                    Rgb::new(rgb.r, rgb.g, rgb.b)
                }
                .map(|e| (e * 255.0) as u8);
                common::terrain::map::MapSample {
                    rgb,
                    alt,
                    downhill_wpos,
                    connections: None,
                }
            }
            // Generate standard shaded map
            map_config.is_shaded = true;
            map_config.generate(
                |pos| {
                    sample_pos(
                        &map_config,
                        pos,
                        &alt,
                        &rgba,
                        &map_size,
                        &map_size_lg,
                        max_height,
                    )
                },
                |wpos| {
                    let pos = wpos.wpos_to_cpos();
                    rescale_height(if bounds_check(pos) {
                        scale_height_big(alt[pos])
                    } else {
                        0.0
                    })
                },
                |pos, (r, g, b, a)| {
                    world_map_rgba[pos.y * map_size.x as usize + pos.x] =
                        u32::from_le_bytes([r, g, b, a]);
                },
            );
            // Generate map with topographical lines and stylized colors
            map_config.is_contours = true;
            map_config.is_stylized_topo = true;
            map_config.generate(
                |pos| {
                    sample_pos(
                        &map_config,
                        pos,
                        &alt,
                        &rgba,
                        &map_size,
                        &map_size_lg,
                        max_height,
                    )
                },
                |wpos| {
                    let pos = wpos.wpos_to_cpos();
                    rescale_height(if bounds_check(pos) {
                        scale_height_big(alt[pos])
                    } else {
                        0.0
                    })
                },
                |pos, (r, g, b, a)| {
                    world_map_topo[pos.y * map_size.x as usize + pos.x] =
                        u32::from_le_bytes([r, g, b, a]);
                },
            );
            let make_raw = |rgb| -> Result<_, Error> {
                let mut raw = vec![0u8; 4 * world_map_rgba.len()];
                LittleEndian::write_u32_into(rgb, &mut raw);
                Ok(Arc::new(
                    DynamicImage::ImageRgba8({
                        // Should not fail if the dimensions are correct.
                        let map =
                            image::ImageBuffer::from_raw(u32::from(map_size.x), u32::from(map_size.y), raw);
                        map.ok_or_else(|| Error::Other("Server sent a bad world map image".into()))?
                    })
                    // Flip the image, since Voxygen uses an orientation where rotation from
                    // positive x axis to positive y axis is counterclockwise around the z axis.
                    .flipv(),
                ))
            };
            let lod_base = rgba;
            let lod_alt = alt;
            let world_map_rgb_img = make_raw(&world_map_rgba)?;
            let world_map_topo_img = make_raw(&world_map_topo)?;
            let world_map_layers = vec![world_map_rgb_img, world_map_topo_img];
            let horizons = (west.0, west.1, east.0, east.1)
                .into_par_iter()
                .map(|(wa, wh, ea, eh)| u32::from_le_bytes([wa, wh, ea, eh]))
                .collect::<Vec<_>>();
            let lod_horizon = horizons;
            let map_bounds = Vec2::new(sea_level, max_height);
            debug!("Done preparing image...");

            Ok((
                state,
                lod_base,
                lod_alt,
                Grid::from_raw(map_size.map(|e| e as i32), lod_horizon),
                (world_map_layers, map_size, map_bounds),
                world_map.sites,
                world_map.possible_starting_sites,
                world_map.pois,
                component_recipe_book,
                max_group_size,
                client_timeout,
                missing_plugins,
                local_plugins,
                role,
            ))
        });

        let (
            state,
            lod_base,
            lod_alt,
            lod_horizon,
            world_map,
            sites,
            possible_starting_sites,
            pois,
            component_recipe_book,
            max_group_size,
            client_timeout,
            missing_plugins,
            local_plugins,
            role,
        ) = loop {
            tokio::select! {
                res = &mut task => break res.expect("Client thread should not panic")?,
                _ = ping_interval.tick() => ping_stream.send(PingMsg::Ping)?,
            }
        };
        let missing_plugins_set = missing_plugins.iter().cloned().collect();
        if !missing_plugins.is_empty() {
            stream.send(ClientGeneral::RequestPlugins(missing_plugins))?;
        }
        ping_stream.send(PingMsg::Ping)?;

        debug!("Initial sync done");

        Ok(Self {
            client_type,
            registered: true,
            presence: None,
            runtime,
            server_info,
            semantic_send_state: semantic_state_binding.map(common_net::msg::SemanticSendStateV1::new),
            semantic_receive_state: semantic_state_binding.map(common_net::msg::SemanticReceiveStateV1::new),
            semantic_ingress_metrics: common_net::msg::envelope::SemanticIngressMetricsV1::new(),
            checkpoint_runtime: None,
            server_description: description,
            world_data: WorldData {
                lod_base,
                lod_alt,
                lod_horizon,
                map: world_map,
            },
            weather: WeatherLerp::default(),
            player_list: HashMap::new(),
            character_list: CharacterList::default(),
            character_being_deleted: None,
            sites: sites
                .iter()
                .filter_map(|m| {
                    Some((m.site?, SiteMarker {
                        marker: m.clone(),
                        economy: None,
                    }))
                })
                .collect(),
            extra_markers: sites.iter().filter(|m| m.site.is_none()).cloned().collect(),
            possible_starting_sites,
            pois,
            component_recipe_book,
            available_recipes: HashMap::default(),
            chat_mode: ChatMode::default(),

            lod_zones: HashMap::new(),
            lod_last_requested: None,
            lod_pos_fallback: None,

            force_update_generation: common::apex::physics_generation::PhysicsGenerationV1::NEVER_CORRECTED,
            prediction_buffer: common::apex::prediction_boundary::ClientPredictionBufferV1::new(
                PREDICTION_BUFFER_CAPACITY_TICKS,
                PREDICTION_BUFFER_BUDGET_BYTES,
            ),
            was_mounted_last_tick: false,
            last_server_sync_tick: 0,
            correction_magnitude_metrics: common_systems::reconciliation::CorrectionMagnitudeMetricsV1::new(),

            role,
            max_group_size,
            invite: None,
            group_leader: None,
            group_members: HashMap::new(),
            pending_invites: HashSet::new(),
            pending_trade: None,
            waypoint: None,

            network: Some(network),
            participant: Some(participant),
            general_stream: stream,
            ping_stream,
            register_stream,
            character_screen_stream,
            in_game_stream,
            terrain_stream,

            client_timeout,

            last_server_ping: 0.0,
            last_server_pong: 0.0,
            last_ping_delta: 0.0,
            ping_deltas: VecDeque::new(),

            tick: 0,
            state,

            flashing_lights_enabled: true,

            server_view_distance_limit: None,
            view_distance: None,
            lod_distance: 4.0,
            loaded_distance: 0.0,

            pending_chunks: HashMap::new(),
            bastion_terrain_anchor: None,
            bastion_designations: Vec::new(),
            bastion_designations_rev: 0,
            bastion_inspect: None,
            target_time_of_day: None,
            dt_adjustment: 1.0,

            connected_server_constants: server_constants,
            missing_plugins: missing_plugins_set,
            local_plugins,
        })
    }

    /// Request a state transition to `ClientState::Registered`.
    async fn register(
        username: &str,
        password: &str,
        locale: Option<String>,
        mut auth_trusted: impl FnMut(&str) -> bool,
        server_info: &ServerInfo,
        register_stream: &mut Stream,
    ) -> Result<SessionBindingV1, Error> {
        // Authentication
        let token_or_username = match &server_info.auth_provider {
            Some(addr) => {
                // Query whether this is a trusted auth server
                if auth_trusted(addr) {
                    let (scheme, authority) = match addr.split_once("://") {
                        Some((s, a)) => (s, a),
                        None => return Err(Error::AuthServerUrlInvalid(addr.to_string())),
                    };

                    let scheme = match scheme.parse::<authc::Scheme>() {
                        Ok(s) => s,
                        Err(_) => return Err(Error::AuthServerUrlInvalid(addr.to_string())),
                    };

                    let authority = match authority.parse::<authc::Authority>() {
                        Ok(a) => a,
                        Err(_) => return Err(Error::AuthServerUrlInvalid(addr.to_string())),
                    };

                    Ok(authc::AuthClient::new(scheme, authority)?
                        .sign_in(username, password)
                        .await?
                        .serialize())
                } else {
                    Err(Error::AuthServerNotTrusted)
                }
            },
            None => Ok(username.to_owned()),
        }?;

        debug!("Registering client...");

        register_stream.send(ClientRegister {
            // APEX-T3.1.08: echo exactly the boot ID observed in ServerInfo.
            expected_server_boot_id: server_info.server_boot_id,
            // APEX-T3.2: always `New` for now -- client-side resume-on-
            // reconnect (storing a prior `SessionBindingV1` across a
            // voxygen-level retry and threading it back in here) is a
            // deliberate follow-up, not wired in this pass; the server-side
            // `Resume` path is fully real and integration-tested via the
            // harness independent of this client ever sending it (spec
            // section 5's own scope note).
            session_request: SessionRequestV1::New,
            // APEX-T3.3.05: no V1 sender exists yet (lands in T3.3.07) --
            // always Legacy for now, matching every real server this
            // client talks to advertising Legacy in its supported set
            // (row status doc requirement 2: golden path unaffected).
            requested_semantic_protocol: common_net::msg::envelope::SemanticProtocolIdV1::Legacy,
            token_or_username,
            locale,
        })?;

        match register_stream.recv::<ServerRegisterAnswer>().await? {
            Err(RegisterError::AuthError(err)) => Err(Error::AuthErr(err)),
            Err(RegisterError::InvalidCharacter) => Err(Error::InvalidCharacter),
            Err(RegisterError::NotOnWhitelist) => Err(Error::NotOnWhitelist),
            Err(RegisterError::Kicked(err)) => Err(Error::Kicked(err)),
            Err(RegisterError::Banned(info)) => Err(Error::Banned(info)),
            Err(RegisterError::TooManyPlayers) => Err(Error::TooManyPlayers),
            Err(RegisterError::ServerBootMismatch { current, received }) => {
                Err(Error::ServerBootMismatch { server_info: current, game_sync: received })
            },
            Err(RegisterError::UnknownSession) => Err(Error::UnknownSession),
            Err(RegisterError::SessionPrincipalMismatch) => Err(Error::SessionPrincipalMismatch),
            Err(RegisterError::SessionExpired) => Err(Error::SessionExpired),
            Err(RegisterError::ConnectionEpochMismatch { current, expected }) => {
                Err(Error::ConnectionEpochMismatch { current, expected })
            },
            Err(RegisterError::ConnectionEpochExhausted) => Err(Error::ConnectionEpochExhausted),
            Err(RegisterError::SessionClientTypeMismatch { session, requested }) => {
                Err(Error::SessionClientTypeMismatch { session, requested })
            },
            Err(RegisterError::OlderAttemptSuperseded) => Err(Error::OlderAttemptSuperseded),
            Err(RegisterError::IncompatibleSemanticProtocol) => Err(Error::IncompatibleSemanticProtocol),
            Err(RegisterError::SemanticProtocolModeSwitch) => Err(Error::SemanticProtocolModeSwitch),
            Ok(admission) => {
                debug!("Client registered successfully.");
                Ok(admission.binding())
            },
        }
    }

    /// `APEX-T3.3.07`: envelope, digest, sequence, and send one
    /// post-auth `ClientGeneral` over the negotiated `NetEnvelopeV1`
    /// wire mode. Dormant in this tree today -- `register()` always
    /// requests `Legacy` (`T3.3.05`'s own row status doc), so
    /// `semantic_send_state` is always `None` and this is never actually
    /// reached by the live client; built and tested against a
    /// synthetically-attached state instead (this function's own test
    /// module).
    ///
    /// Packet's own failure list ("No attachment, exhaustion, encode
    /// failure, send failure after allocation"): the first three are
    /// treated the same way this function's sibling
    /// (`send_msg_err`'s existing `!verified` branch, just above the call
    /// site) already treats a dropped message -- logged, message
    /// silently not sent, connection not torn down for what is a local
    /// bookkeeping problem, not a network fault. Only the underlying
    /// `stream.send` failure -- a genuine transport-layer event -- is
    /// propagated as `Err`, and only AFTER the sequence was already
    /// consumed (packet: "sequence is consumed before send and never
    /// reused after failure" -- `SemanticSendStateV1::allocate_sequence`
    /// advances the cursor unconditionally before any encode/send is
    /// attempted, so a failed send never gets retried with the same
    /// sequence value).
    fn send_semantic_v1(&mut self, msg: ClientGeneral) -> Result<(), network::StreamError> {
        use common_net::msg::envelope::{SemanticRouteV1, encode_payload_v1, net_envelope_profile_root_v1, payload_digest_v1};

        let Some(send_state) = self.semantic_send_state.as_mut() else {
            warn!("send_semantic_v1 called with no active NetEnvelopeV1 attachment; dropping message: {msg:?}");
            return Ok(());
        };
        let binding = send_state.binding();
        let semantic_stream = msg.semantic_stream();
        let payload_schema = msg.payload_schema();

        // packet's `command_id` field is dormant until `T3.5` -- this
        // sender never sets it (spec section 5.6: `Some` is rejected by
        // the receiver anyway).
        let sequence = match send_state.allocate_sequence(semantic_stream) {
            Ok(seq) => seq,
            Err(_exhausted) => {
                warn!(?semantic_stream, "semantic send sequence exhausted (u64::MAX reached); dropping message");
                return Ok(());
            },
        };

        let payload_bytes = encode_payload_v1(&msg);
        let profile_root = net_envelope_profile_root_v1();
        let payload_encoding = common_net::msg::envelope::SemanticPayloadEncodingV1::Bincode2LegacySerde;
        let payload_digest = payload_digest_v1(profile_root, payload_schema, payload_encoding, &payload_bytes);
        let header = common_net::msg::envelope::NetEnvelopeHeaderV1 {
            profile_root,
            server_boot_id: binding.server_boot_id,
            session_id: binding.session_id,
            connection_epoch: binding.epoch,
            direction: common_net::msg::envelope::SemanticDirectionV1::ClientToServer,
            semantic_stream,
            sequence,
            causality: common_net::msg::envelope::SemanticCausalityV1 { producer_tick: None, snapshot: None },
            payload_schema,
            payload_encoding,
            payload_len: payload_bytes.len() as u64,
            payload_digest,
            command_id: None,
            checkpoint: None,
        };
        let frame = common_net::msg::envelope::SemanticWireFrameV1 { header, payload_bytes };

        let limits = common::apex::manifest::ManifestDecodeLimitsV1 {
            max_input_bytes: 1 << 20,
            max_depth: 8,
            max_nodes: 64,
            max_array_items: 16,
            max_map_entries: 16,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 1 << 20,
        };
        let frame_bytes = common::apex::manifest::encode_manifest_v1(&frame, &limits)
            .expect("frame construction above is always within limits and always well-formed");

        // The T0.2-encoded frame bytes are carried as an opaque Vec<u8>
        // through the EXISTING (bincode-legacy) stream framing, per
        // packet section 7.3: "carried as an opaque byte vector through
        // the existing stream framing" -- never a second, competing wire
        // protocol.
        let stream = match semantic_stream {
            common_net::msg::envelope::SemanticStreamIdV1::CharacterScreen => &mut self.character_screen_stream,
            common_net::msg::envelope::SemanticStreamIdV1::InGame => &mut self.in_game_stream,
            common_net::msg::envelope::SemanticStreamIdV1::Terrain => &mut self.terrain_stream,
            common_net::msg::envelope::SemanticStreamIdV1::General => &mut self.general_stream,
            common_net::msg::envelope::SemanticStreamIdV1::Bootstrap => &mut self.register_stream,
        };
        stream.send(frame_bytes)
    }

    /// `T3.3.10`: client-side, server-to-client counterpart of the
    /// server's own `validate_semantic_frame_v1`
    /// (`server/src/sys/msg/mod.rs`) -- decodes and validates one raw
    /// semantic wire frame BEFORE any local ECS/frontend mutation,
    /// returning the fully checked payload. Pure: takes
    /// `receive_state` by immutable reference, so it structurally cannot
    /// itself commit the receive cursor (packet: "cursor does not
    /// advance on validation failure; it advances before handler call").
    /// Dormant in this tree today for the same reason `send_semantic_v1`
    /// is -- `T3.3.05`'s negotiation always resolves `Legacy`.
    ///
    /// `T3.3.16`: genericized over the payload type (was hardcoded to
    /// `ServerGeneral`) -- every check here (profile root, boot,
    /// session, epoch, direction, stream route, sequence, payload
    /// length, digest, command-id) is payload-independent, and
    /// `ServerInit::GameSync` needs the identical validation this row's
    /// own acceptance gate leans on ("no post-auth V1 payload is
    /// accepted before correctly bound GameSync"). Existing call sites
    /// keep working unchanged (`ServerGeneral` is still inferred from
    /// their own surrounding context); only the direct unit tests below
    /// needed an explicit turbofish once inference had nothing left to
    /// pin the type from.
    ///
    /// `T3.3.17`: also returns the frame's own `SemanticCausalityV1`
    /// alongside the decoded payload -- this function stays pure (takes
    /// `receive_state` by `&`, never commits `highest_snapshot`, same
    /// "cursor does not advance on validation failure" discipline the
    /// sequence check already follows), so the caller needs the
    /// causality back to call `SemanticReceiveStateV1::commit_snapshot`
    /// itself, strictly after `advance_expected` succeeds (mirrors
    /// exactly how the caller already commits the sequence cursor).
    fn validate_semantic_frame_v1<T>(
        raw: &[u8],
        receive_state: &common_net::msg::envelope::SemanticReceiveStateV1,
        expected_physical_stream: common_net::msg::envelope::SemanticStreamIdV1,
    ) -> Result<
        (
            T,
            common_net::msg::envelope::SemanticCausalityV1,
            Option<common_net::msg::checkpoint::CheckpointedEnvelopeContextV1>,
        ),
        common_net::msg::envelope::SemanticEnvelopeRejectV1,
    >
    where
        T: common_net::msg::envelope::SemanticRouteV1 + serde::de::DeserializeOwned,
    {
        use common_net::msg::envelope::{
            NetEnvelopeHeaderV1, SemanticDirectionV1, SemanticEnvelopeRejectV1, SemanticWireFrameV1, decode_payload_exact_v1,
            net_envelope_profile_root_v1, payload_digest_v1, production_causality_profile_v1, validate_causality_against_profile_v1,
        };

        let limits = common::apex::manifest::ManifestDecodeLimitsV1 {
            max_input_bytes: 1 << 20,
            max_depth: 8,
            max_nodes: 64,
            max_array_items: 16,
            max_map_entries: 16,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 1 << 20,
        };
        let frame: SemanticWireFrameV1 = common::apex::manifest::decode_manifest_v1(raw, &limits)
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
        if header.direction != SemanticDirectionV1::ServerToClient {
            return Err(SemanticEnvelopeRejectV1::WrongDirection);
        }
        if header.semantic_stream != expected_physical_stream {
            return Err(SemanticEnvelopeRejectV1::StreamRouteMismatch);
        }
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
        // Dormant until T3.5 (packet section 5.6): `Some` is
        // unconditionally rejected today, never partially trusted.
        if header.command_id.is_some() {
            return Err(SemanticEnvelopeRejectV1::CommandIdUnsupported);
        }
        // `T3.3.17`: structural profile check (declared domain,
        // schema's own requirement) first, THEN the session-local
        // monotonicity check -- a frame naming an undeclared domain is
        // rejected before this session's own history is even
        // consulted. Both unreachable on real traffic today (production
        // profile: no declared domains, every schema optional).
        validate_causality_against_profile_v1(&header.causality, header.payload_schema, &production_causality_profile_v1())?;
        if let Some(snapshot) = &header.causality.snapshot {
            if !receive_state.snapshot_is_fresh(snapshot) {
                return Err(SemanticEnvelopeRejectV1::StaleSnapshot);
            }
        }

        let decoded: T = decode_payload_exact_v1(&frame.payload_bytes)?;
        if decoded.semantic_stream() != header.semantic_stream || decoded.payload_schema() != header.payload_schema {
            return Err(SemanticEnvelopeRejectV1::StreamRouteMismatch);
        }

        Ok((decoded, header.causality, header.checkpoint))
    }

    fn send_msg_err<S>(&mut self, msg: S) -> Result<(), network::StreamError>
    where
        S: Into<ClientMsg>,
    {
        prof_span!("send_msg_err");
        let msg: ClientMsg = msg.into();
        #[cfg(debug_assertions)]
        {
            const C_TYPE: ClientType = ClientType::Game;
            let verified = msg.verify(C_TYPE, self.registered, self.presence);

            // Due to the fact that character loading is performed asynchronously after
            // initial connect it is possible to receive messages after a character load
            // error while in the wrong state.
            if !verified {
                warn!(
                    "Received ClientType::Game message when not in game (Registered: {} Presence: \
                     {:?}), dropping message: {:?} ",
                    self.registered, self.presence, msg
                );
                return Ok(());
            }
        }
        match msg {
            ClientMsg::Type(msg) => self.register_stream.send(msg),
            ClientMsg::Register(msg) => self.register_stream.send(msg),
            ClientMsg::General(msg) => {
                // APEX-T3.3.07: V1 routes/encodes/sequences/frames/sends
                // through send_semantic_v1; Legacy falls through to the
                // unchanged code below it (packet: "Legacy branch remains
                // during rollout"). Checked first, before any of the
                // existing physical-stream matching, so a V1 attachment
                // never touches the legacy path at all.
                if self.semantic_send_state.is_some() {
                    return self.send_semantic_v1(msg);
                }
                #[cfg(feature = "tracy")]
                let (mut ingame, mut terrain) = (0.0, 0.0);
                let stream = match msg {
                    ClientGeneral::RequestCharacterList
                    | ClientGeneral::CreateCharacter { .. }
                    | ClientGeneral::EditCharacter { .. }
                    | ClientGeneral::DeleteCharacter(_)
                    | ClientGeneral::Character(_, _)
                    | ClientGeneral::Spectate(_) => &mut self.character_screen_stream,
                    // Only in game
                    ClientGeneral::ControllerInputs(_)
                    | ClientGeneral::ControlEvent(_)
                    | ClientGeneral::ControlAction(_)
                    | ClientGeneral::SetViewDistance(_)
                    | ClientGeneral::BreakBlock(_)
                    | ClientGeneral::PlaceBlock(_, _)
                    | ClientGeneral::ExitInGame
                    | ClientGeneral::PlayerPhysics { .. }
                    | ClientGeneral::UnlockSkill(_)
                    | ClientGeneral::RequestSiteInfo(_)
                    | ClientGeneral::RequestPlayerPhysics { .. }
                    | ClientGeneral::RequestLossyTerrainCompression { .. }
                    | ClientGeneral::UpdateMapMarker(_)
                    | ClientGeneral::SpectatePosition(_)
                    | ClientGeneral::SpectateEntity(_)
                    | ClientGeneral::BastionCameraAnchor(_)
                    | ClientGeneral::BastionPlaceDesignation { .. }
                    | ClientGeneral::BastionApplyInfluence { .. }
                    | ClientGeneral::BastionContextAction { .. }
                    | ClientGeneral::BastionSpawnColony { .. }
                    | ClientGeneral::BastionCancelDesignation { .. }
                    | ClientGeneral::BastionInspect { .. }
                    | ClientGeneral::SetBattleMode(_) => {
                        #[cfg(feature = "tracy")]
                        {
                            ingame = 1.0;
                        }
                        &mut self.in_game_stream
                    },
                    // Terrain
                    ClientGeneral::TerrainChunkRequest { .. }
                    | ClientGeneral::LodZoneRequest { .. } => {
                        #[cfg(feature = "tracy")]
                        {
                            terrain = 1.0;
                        }
                        &mut self.terrain_stream
                    },
                    // Always possible
                    ClientGeneral::ChatMsg(_)
                    | ClientGeneral::Command(_, _)
                    | ClientGeneral::Terminate
                    | ClientGeneral::RequestPlugins(_)
                    | ClientGeneral::RequestPluginArtifacts(_)
                    // T3.4.19: the commit ack rides General, mirroring its
                    // semantic routing.
                    | ClientGeneral::CheckpointCommitAck(_)
                    // W3 renderer-bench: out-of-band diagnostics ride
                    // General, mirroring their semantic routing.
                    | ClientGeneral::RendererBenchReady
                    | ClientGeneral::RendererBenchProjectionAck(_) => &mut self.general_stream,
                };
                #[cfg(feature = "tracy")]
                {
                    plot!("ingame_sends", ingame);
                    plot!("terrain_sends", terrain);
                }
                stream.send(msg)
            },
            ClientMsg::Ping(msg) => self.ping_stream.send(msg),
        }
    }

    pub fn request_player_physics(&mut self, server_authoritative: bool) {
        self.send_msg(ClientGeneral::RequestPlayerPhysics {
            server_authoritative,
        })
    }

    /// W3 renderer-bench: tell the server this client is in-session and
    /// able to receive bench announces (voxygen calls this from the
    /// session readiness hook; the headless ackbot after spectate).
    pub fn renderer_bench_ready(&mut self) {
        self.send_msg(ClientGeneral::RendererBenchReady);
    }

    /// W3 renderer-bench: one announce → one ack. The projection root is
    /// computed from the CLIENT's replicated ECS at receipt time (a
    /// wall-coupled observation — recorded beside the tape, never inside
    /// run identity), keyed by the synced semantic id, never runtime ids.
    fn handle_renderer_bench_announce(
        &mut self,
        ann: common::renderer_bench::BenchFrameAnnounceV1,
    ) {
        use common::renderer_bench as rb;
        use std::sync::OnceLock;
        static ACK_ENABLED: OnceLock<bool> = OnceLock::new();
        let enabled = *ACK_ENABLED.get_or_init(|| {
            std::env::var("BASTION_RENDERER_BENCH_ACK").as_deref() == Ok("1")
        });
        if !enabled {
            return;
        }
        let schema = rb::oracle_schema_hash();
        // Mirror the server's arena-origin math (mm_to_blocks).
        let origin = Vec3::new(
            ann.arena_origin_mm[0] as f32,
            ann.arena_origin_mm[1] as f32,
            ann.arena_origin_mm[2] as f32,
        ) / 1000.0;
        let mut owners: Vec<(u32, (Vec<u8>, [u8; 32]))> = {
            use specs::Join;
            let ecs = self.state.ecs();
            let bench_ids = ecs.read_storage::<comp::bastion::RendererBenchEntityId>();
            let positions = ecs.read_storage::<comp::Pos>();
            (&bench_ids, &positions)
                .join()
                .map(|(id, pos)| {
                    let mm_v = (pos.0 - origin) * 1000.0;
                    let mm = [mm_v.x as i32, mm_v.y as i32, mm_v.z as i32];
                    (id.0, rb::client_projection_owner(&schema, id.0, mm))
                })
                .collect()
        };
        owners.sort_by_key(|(id, _)| *id);
        let entities_resolved = owners.len() as u32;
        let owner_entries: Vec<(Vec<u8>, [u8; 32])> =
            owners.into_iter().map(|(_, e)| e).collect();
        let client_projection_root =
            rb::domain_root(&schema, rb::Domain::ClientProjection, &owner_entries);
        self.send_msg(ClientGeneral::RendererBenchProjectionAck(
            rb::BenchProjectionAckV1 {
                frame_index: ann.frame_index,
                sim_tick: ann.sim_tick,
                frame_root_echo: ann.frame_root,
                client_projection_root,
                entities_resolved,
            },
        ));
    }

    pub fn request_lossy_terrain_compression(&mut self, lossy_terrain_compression: bool) {
        self.send_msg(ClientGeneral::RequestLossyTerrainCompression {
            lossy_terrain_compression,
        })
    }

    fn send_msg<S>(&mut self, msg: S)
    where
        S: Into<ClientMsg>,
    {
        let res = self.send_msg_err(msg);
        if let Err(e) = res {
            warn!(
                ?e,
                "connection to server no longer possible, couldn't send msg"
            );
        }
    }

    /// Request a state transition to `ClientState::Character`.
    pub fn request_character(
        &mut self,
        character_id: CharacterId,
        view_distances: common::ViewDistances,
    ) {
        let view_distances = self.set_view_distances_local(view_distances);
        self.send_msg(ClientGeneral::Character(character_id, view_distances));

        if let Some(character) = self
            .character_list
            .characters
            .iter()
            .find(|x| x.character.id == Some(character_id))
        {
            self.waypoint = character.location.clone();
        }

        // Assume we are in_game unless server tells us otherwise
        self.presence = Some(PresenceKind::Character(character_id));
    }

    /// Request a state transition to `ClientState::Spectate`.
    pub fn request_spectate(&mut self, view_distances: common::ViewDistances) {
        let view_distances = self.set_view_distances_local(view_distances);
        self.send_msg(ClientGeneral::Spectate(view_distances));

        self.presence = Some(PresenceKind::Spectator);
    }

    /// Load the current players character list
    pub fn load_character_list(&mut self) {
        self.character_list.loading = true;
        self.send_msg(ClientGeneral::RequestCharacterList);
    }

    /// New character creation
    pub fn create_character(
        &mut self,
        alias: String,
        mainhand: Option<String>,
        offhand: Option<String>,
        body: comp::Body,
        hardcore: bool,
        start_site: Option<SiteId>,
    ) {
        self.character_list.loading = true;
        self.send_msg(ClientGeneral::CreateCharacter {
            alias,
            mainhand,
            offhand,
            body,
            hardcore,
            start_site,
        });
    }

    pub fn edit_character(&mut self, alias: String, id: CharacterId, body: comp::Body) {
        self.character_list.loading = true;
        self.send_msg(ClientGeneral::EditCharacter { alias, id, body });
    }

    /// Character deletion
    pub fn delete_character(&mut self, character_id: CharacterId) {
        // Pre-emptively remove the character to be deleted from the character list as
        // character deletes are processed asynchronously by the server so we can't rely
        // on a timely response to update the character list
        if let Some(pos) = self
            .character_list
            .characters
            .iter()
            .position(|x| x.character.id == Some(character_id))
        {
            self.character_list.characters.remove(pos);
        }
        self.send_msg(ClientGeneral::DeleteCharacter(character_id));
    }

    /// Send disconnect message to the server
    pub fn logout(&mut self) {
        debug!("Sending logout from server");
        self.send_msg(ClientGeneral::Terminate);
        self.registered = false;
        self.presence = None;
    }

    /// Request a state transition to `ClientState::Registered` from an ingame
    /// state.
    pub fn request_remove_character(&mut self) {
        self.chat_mode = ChatMode::World;
        self.send_msg(ClientGeneral::ExitInGame);
    }

    pub fn set_view_distances(&mut self, view_distances: common::ViewDistances) {
        let view_distances = self.set_view_distances_local(view_distances);
        self.send_msg(ClientGeneral::SetViewDistance(view_distances));
    }

    /// Clamps provided view distances, locally sets the terrain view distance
    /// in the client's properties and returns the clamped values for the
    /// caller to send to the server.
    fn set_view_distances_local(
        &mut self,
        view_distances: common::ViewDistances,
    ) -> common::ViewDistances {
        let view_distances = common::ViewDistances {
            terrain: view_distances
                .terrain
                .clamp(1, MAX_SELECTABLE_VIEW_DISTANCE),
            entity: view_distances.entity.max(1),
        };
        self.view_distance = Some(view_distances.terrain);
        view_distances
    }

    pub fn set_lod_distance(&mut self, lod_distance: u32) {
        let lod_distance = lod_distance.clamp(0, 1000) as f32 / lod::ZONE_SIZE as f32;
        self.lod_distance = lod_distance;
    }

    pub fn set_flashing_lights_enabled(&mut self, flashing_lights_enabled: bool) {
        self.flashing_lights_enabled = flashing_lights_enabled;
    }

    pub fn use_slot(&mut self, slot: Slot) {
        self.control_action(ControlAction::InventoryAction(InventoryAction::Use(slot)))
    }

    pub fn swap_slots(&mut self, a: Slot, b: Slot) {
        match (a, b) {
            (Slot::Overflow(o), Slot::Inventory(inv))
            | (Slot::Inventory(inv), Slot::Overflow(o)) => {
                self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                    InventoryEvent::OverflowMove(o, inv),
                )));
            },
            (Slot::Overflow(_), _) | (_, Slot::Overflow(_)) => {},
            (Slot::Equip(equip), slot) | (slot, Slot::Equip(equip)) => self.control_action(
                ControlAction::InventoryAction(InventoryAction::Swap(equip, slot)),
            ),
            (Slot::Inventory(inv1), Slot::Inventory(inv2)) => {
                self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                    InventoryEvent::Swap(inv1, inv2),
                )))
            },
        }
    }

    pub fn drop_slot(&mut self, slot: Slot) {
        match slot {
            Slot::Equip(equip) => {
                self.control_action(ControlAction::InventoryAction(InventoryAction::Drop(equip)))
            },
            Slot::Inventory(inv) => self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::InventoryEvent(InventoryEvent::Drop(inv)),
            )),
            Slot::Overflow(o) => self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::InventoryEvent(InventoryEvent::OverflowDrop(o)),
            )),
        }
    }

    pub fn sort_inventory(&mut self, sort_order: InventorySortOrder) {
        self.control_action(ControlAction::InventoryAction(InventoryAction::Sort(
            sort_order,
        )));
    }

    pub fn perform_trade_action(&mut self, action: TradeAction) {
        if let Some((id, _, _)) = self.pending_trade {
            if let TradeAction::Decline = action {
                self.pending_trade.take();
            }
            self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::PerformTradeAction(id, action),
            ));
        }
    }

    pub fn is_dead(&self) -> bool { self.current::<comp::Health>().is_some_and(|h| h.is_dead) }

    pub fn is_gliding(&self) -> bool {
        self.current::<CharacterState>()
            .is_some_and(|cs| matches!(cs, CharacterState::Glide(_)))
    }

    pub fn split_swap_slots(&mut self, a: Slot, b: Slot) {
        match (a, b) {
            (Slot::Overflow(_), _) | (_, Slot::Overflow(_)) => {},
            (Slot::Equip(equip), slot) | (slot, Slot::Equip(equip)) => self.control_action(
                ControlAction::InventoryAction(InventoryAction::Swap(equip, slot)),
            ),
            (Slot::Inventory(inv1), Slot::Inventory(inv2)) => {
                self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                    InventoryEvent::SplitSwap(inv1, inv2),
                )))
            },
        }
    }

    pub fn split_drop_slot(&mut self, slot: Slot) {
        match slot {
            Slot::Equip(equip) => {
                self.control_action(ControlAction::InventoryAction(InventoryAction::Drop(equip)))
            },
            Slot::Inventory(inv) => self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::InventoryEvent(InventoryEvent::SplitDrop(inv)),
            )),
            Slot::Overflow(o) => self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::InventoryEvent(InventoryEvent::OverflowSplitDrop(o)),
            )),
        }
    }

    pub fn pick_up(&mut self, entity: EcsEntity) {
        // Get the health component from the entity

        if let Some(uid) = self.state.read_component_copied(entity) {
            // If we're dead, exit before sending the message
            if self.is_dead() {
                return;
            }

            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::Pickup(uid),
            )));
        }
    }

    pub fn do_pet(&mut self, target_entity: EcsEntity) {
        if self.is_dead() {
            return;
        }

        if let Some(target_uid) = self.state.read_component_copied(target_entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InteractWith {
                target: target_uid,
                kind: common::interaction::InteractionKind::Pet,
            }))
        }
    }

    pub fn npc_interact(&mut self, npc_entity: EcsEntity) {
        // If we're dead, exit before sending message
        if self.is_dead() {
            return;
        }

        if let Some(uid) = self.state.read_component_copied(npc_entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Interact(uid)));
        }
    }

    pub fn player_list(&self) -> &HashMap<Uid, PlayerInfo> { &self.player_list }

    pub fn character_list(&self) -> &CharacterList { &self.character_list }

    pub fn server_info(&self) -> &ServerInfo { &self.server_info }

    pub fn server_description(&self) -> &ServerDescription { &self.server_description }

    pub fn world_data(&self) -> &WorldData { &self.world_data }

    pub fn component_recipe_book(&self) -> &ComponentRecipeBook { &self.component_recipe_book }

    pub fn client_type(&self) -> &ClientType { &self.client_type }

    pub fn available_recipes(&self) -> &HashMap<String, Option<SpriteKind>> {
        &self.available_recipes
    }

    pub fn lod_zones(&self) -> &HashMap<Vec2<i32>, lod::Zone> { &self.lod_zones }

    /// Set the fallback position used for loading LoD zones when the client
    /// entity does not have a position.
    pub fn set_lod_pos_fallback(&mut self, pos: Vec2<f32>) { self.lod_pos_fallback = Some(pos); }

    pub fn craft_recipe(
        &mut self,
        recipe: &str,
        slots: Vec<(u32, InvSlotId)>,
        craft_sprite: Option<(VolumePos, SpriteKind)>,
        amount: u32,
    ) -> bool {
        let (can_craft, has_sprite) = if let Some(inventory) = self
            .state
            .ecs()
            .read_storage::<comp::Inventory>()
            .get(self.entity())
        {
            let rbm = self.state.ecs().read_resource::<RecipeBookManifest>();
            let (can_craft, required_sprite) = inventory.can_craft_recipe(recipe, 1, &rbm);
            let has_sprite =
                required_sprite.is_none_or(|s| Some(s) == craft_sprite.map(|(_, s)| s));
            (can_craft, has_sprite)
        } else {
            (false, false)
        };
        if can_craft && has_sprite {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::CraftRecipe {
                    craft_event: CraftEvent::Simple {
                        recipe: recipe.to_string(),
                        slots,
                        amount,
                    },
                    craft_sprite: craft_sprite.map(|(pos, _)| pos),
                },
            )));
            true
        } else {
            false
        }
    }

    /// Checks if the item in the given slot can be salvaged.
    pub fn can_salvage_item(&self, slot: InvSlotId) -> bool {
        self.inventories()
            .get(self.entity())
            .and_then(|inv| inv.get(slot))
            .is_some_and(|item| item.is_salvageable())
    }

    /// Salvage the item in the given inventory slot. `salvage_pos` should be
    /// the location of a relevant crafting station within range of the player.
    pub fn salvage_item(&mut self, slot: InvSlotId, salvage_pos: VolumePos) -> bool {
        let is_salvageable = self.can_salvage_item(slot);
        if is_salvageable {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::CraftRecipe {
                    craft_event: CraftEvent::Salvage(slot),
                    craft_sprite: Some(salvage_pos),
                },
            )));
        }
        is_salvageable
    }

    /// Crafts modular weapon from components in the provided slots.
    /// `sprite_pos` should be the location of the necessary crafting station in
    /// range of the player.
    /// Returns whether or not the networking event was sent (which is based on
    /// whether the player has two modular components in the provided slots)
    pub fn craft_modular_weapon(
        &mut self,
        primary_component: InvSlotId,
        secondary_component: InvSlotId,
        sprite_pos: Option<VolumePos>,
    ) -> bool {
        let inventories = self.inventories();
        let inventory = inventories.get(self.entity());

        enum ModKind {
            Primary,
            Secondary,
        }

        // Closure to get inner modular component info from item in a given slot
        let mod_kind = |slot| match inventory
            .and_then(|inv| inv.get(slot).map(|item| item.kind()))
            .as_deref()
        {
            Some(ItemKind::ModularComponent(modular::ModularComponent::ToolPrimaryComponent {
                ..
            })) => Some(ModKind::Primary),
            Some(ItemKind::ModularComponent(
                modular::ModularComponent::ToolSecondaryComponent { .. },
            )) => Some(ModKind::Secondary),
            _ => None,
        };

        if let (Some(ModKind::Primary), Some(ModKind::Secondary)) =
            (mod_kind(primary_component), mod_kind(secondary_component))
        {
            drop(inventories);
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::CraftRecipe {
                    craft_event: CraftEvent::ModularWeapon {
                        primary_component,
                        secondary_component,
                    },
                    craft_sprite: sprite_pos,
                },
            )));
            true
        } else {
            false
        }
    }

    pub fn craft_modular_weapon_component(
        &mut self,
        toolkind: tool::ToolKind,
        material: InvSlotId,
        modifier: Option<InvSlotId>,
        slots: Vec<(u32, InvSlotId)>,
        sprite_pos: Option<VolumePos>,
    ) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
            InventoryEvent::CraftRecipe {
                craft_event: CraftEvent::ModularWeaponPrimaryComponent {
                    toolkind,
                    material,
                    modifier,
                    slots,
                },
                craft_sprite: sprite_pos,
            },
        )));
    }

    /// Repairs the item in the given inventory slot. `sprite_pos` should be
    /// the location of a relevant crafting station within range of the player.
    pub fn repair_item(&mut self, item: Slot, sprite_pos: VolumePos) -> bool {
        let is_repairable = {
            let inventories = self.inventories();
            let inventory = inventories.get(self.entity());
            inventory.is_some_and(|inv| {
                if let Some(item) = match item {
                    Slot::Equip(equip_slot) => inv.equipped(equip_slot),
                    Slot::Inventory(invslot) => inv.get(invslot),
                    Slot::Overflow(_) => None,
                } {
                    item.has_durability()
                } else {
                    false
                }
            })
        };
        if is_repairable {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::CraftRecipe {
                    craft_event: CraftEvent::Repair(item),
                    craft_sprite: Some(sprite_pos),
                },
            )));
        }
        is_repairable
    }

    fn update_available_recipes(&mut self) {
        let rbm = self.state.ecs().read_resource::<RecipeBookManifest>();
        let inventories = self.state.ecs().read_storage::<comp::Inventory>();
        if let Some(inventory) = inventories.get(self.entity()) {
            self.available_recipes = inventory
                .recipes_iter()
                .cloned()
                .filter_map(|name| {
                    let (can_craft, required_sprite) = inventory.can_craft_recipe(&name, 1, &rbm);
                    if can_craft {
                        Some((name, required_sprite))
                    } else {
                        None
                    }
                })
                .collect();
        }
    }

    /// Unstable, likely to be removed in a future release
    pub fn sites(&self) -> &HashMap<SiteId, SiteMarker> { &self.sites }

    pub fn markers(&self) -> impl Iterator<Item = &Marker> {
        self.sites
            .values()
            .map(|s| &s.marker)
            .chain(self.extra_markers.iter())
    }

    pub fn possible_starting_sites(&self) -> &[SiteId] { &self.possible_starting_sites }

    /// Unstable, likely to be removed in a future release
    pub fn pois(&self) -> &Vec<PoiInfo> { &self.pois }

    pub fn enable_lantern(&mut self) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::EnableLantern));
    }

    pub fn disable_lantern(&mut self) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::DisableLantern));
    }

    pub fn toggle_sprite_light(&mut self, pos: VolumePos, enable: bool) {
        self.control_action(ControlAction::InventoryAction(
            InventoryAction::ToggleSpriteLight(pos, enable),
        ));
    }

    pub fn help_downed(&mut self, target_entity: EcsEntity) {
        if self.is_dead() {
            return;
        }

        if let Some(target_uid) = self.state.read_component_copied(target_entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InteractWith {
                target: target_uid,
                kind: common::interaction::InteractionKind::HelpDowned,
            }))
        }
    }

    pub fn remove_buff(&mut self, buff_id: BuffKind) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::RemoveBuff(
            buff_id,
        )));
    }

    pub fn leave_stance(&mut self) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::LeaveStance));
    }

    pub fn unlock_skill(&mut self, skill: Skill) {
        self.send_msg(ClientGeneral::UnlockSkill(skill));
    }

    pub fn max_group_size(&self) -> u32 { self.max_group_size }

    pub fn invite(&self) -> Option<(Uid, Instant, Duration, InviteKind)> { self.invite }

    pub fn group_info(&self) -> Option<(String, Uid)> {
        self.group_leader.map(|l| ("Group".into(), l)) // TODO
    }

    pub fn group_members(&self) -> &HashMap<Uid, group::Role> { &self.group_members }

    pub fn pending_invites(&self) -> &HashSet<Uid> { &self.pending_invites }

    pub fn pending_trade(&self) -> &Option<(TradeId, PendingTrade, Option<SitePrices>)> {
        &self.pending_trade
    }

    pub fn is_trading(&self) -> bool { self.pending_trade.is_some() }

    pub fn send_invite(&mut self, invitee: Uid, kind: InviteKind) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InitiateInvite(
            invitee, kind,
        )))
    }

    pub fn accept_invite(&mut self) {
        // Clear invite
        self.invite.take();
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InviteResponse(
            InviteResponse::Accept,
        )));
    }

    pub fn decline_invite(&mut self) {
        // Clear invite
        self.invite.take();
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InviteResponse(
            InviteResponse::Decline,
        )));
    }

    pub fn leave_group(&mut self) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::GroupManip(
            GroupManip::Leave,
        )));
    }

    pub fn kick_from_group(&mut self, uid: Uid) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::GroupManip(
            GroupManip::Kick(uid),
        )));
    }

    pub fn assign_group_leader(&mut self, uid: Uid) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::GroupManip(
            GroupManip::AssignLeader(uid),
        )));
    }

    pub fn is_riding(&self) -> bool {
        self.state
            .ecs()
            .read_storage::<Is<Rider>>()
            .get(self.entity())
            .is_some()
            || self
                .state
                .ecs()
                .read_storage::<Is<VolumeRider>>()
                .get(self.entity())
                .is_some()
    }

    pub fn is_lantern_enabled(&self) -> bool {
        self.state
            .ecs()
            .read_storage::<comp::LightEmitter>()
            .get(self.entity())
            .is_some()
    }

    pub fn mount(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.read_component_copied(entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Mount(uid)));
        }
    }

    /// Mount a block at a `VolumePos`.
    pub fn mount_volume(&mut self, volume_pos: VolumePos) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::MountVolume(
            volume_pos,
        )));
    }

    pub fn unmount(&mut self) { self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Unmount)); }

    pub fn set_pet_stay(&mut self, entity: EcsEntity, stay: bool) {
        if let Some(uid) = self.state.read_component_copied(entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::SetPetStay(
                uid, stay,
            )));
        }
    }

    pub fn give_up(&mut self) {
        if comp::is_downed(self.current().as_ref(), self.current().as_ref()) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::GiveUp));
        }
    }

    pub fn respawn(&mut self) -> bool {
        if self.current::<comp::Health>().is_some_and(|h| h.is_dead) {
            // Hardcore characters cannot respawn, kick them to character selection
            if self.current::<Hardcore>().is_some() {
                self.request_remove_character();
            } else {
                self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Respawn));
            }
            true
        } else {
            false
        }
    }

    pub fn map_marker_event(&mut self, event: MapMarkerChange) {
        self.send_msg(ClientGeneral::UpdateMapMarker(event));
    }

    /// Set the current position to spectate, returns true if the client's
    /// player has a Pos component to write to.
    pub fn spectate_position(&mut self, pos: Vec3<f32>) -> bool {
        let write = if let Some(position) = self
            .state
            .ecs()
            .write_storage::<comp::Pos>()
            .get_mut(self.entity())
        {
            position.0 = pos;
            true
        } else {
            false
        };
        if write {
            self.send_msg(ClientGeneral::SpectatePosition(pos));
        }
        write
    }

    /// bastion (B1.6): set/clear the god-camera terrain anchor. Unlike
    /// [`Self::spectate_position`] this never moves the entity — it only
    /// changes where terrain streams from, for an *embodied* overseer.
    ///
    /// Hysteresis: the loaded disc stays put while the camera pans inside it —
    /// re-centering every pan step kept a freshly-missing chunk next to the
    /// center at all times, which collapsed the fog/detail radius and rendered
    /// the whole view as LoD (QA round 5). The anchor re-centers onto the
    /// focus only once it strays past ~35% of the view radius (XY only — the
    /// ground-glide focus z wobbles constantly).
    pub fn bastion_set_terrain_anchor(&mut self, anchor: Option<Vec3<f32>>) {
        let changed = match (self.bastion_terrain_anchor, anchor) {
            (None, None) => false,
            (Some(a), Some(b)) => {
                let vd_blocks =
                    self.view_distance.unwrap_or(10) as f32 * TerrainChunkSize::RECT_SIZE.x as f32;
                a.xy().distance_squared(b.xy()) > (vd_blocks * 0.35).powi(2)
            },
            _ => true,
        };
        if changed {
            self.bastion_terrain_anchor = anchor;
            self.send_msg(ClientGeneral::BastionCameraAnchor(anchor));
        }
    }

    /// bastion (B2a): designations the server has validated and echoed.
    pub fn bastion_designations(
        &self,
    ) -> &[(
        common::bastion::Region,
        common::bastion::DesignationKind,
        Option<common::bastion::ZExtent>,
    )] {
        &self.bastion_designations
    }

    /// bastion (B5.5): bumps whenever the designation list changes in ANY
    /// way (add, erase-subtract, split). Voxygen rebuilds its overlay shapes
    /// when this moves — index-based incremental sync can't express removal.
    pub fn bastion_designations_rev(&self) -> u64 { self.bastion_designations_rev }

    /// bastion (UI-4 row 62 → UI-5 row 62.2): request one object's inspector
    /// payload — a colonist entity or a world cell (the HUD sends on selection
    /// + ~1Hz while its panel is open).
    pub fn bastion_inspect_request(&mut self, target: comp::bastion::BastionInspectTarget) {
        self.send_msg(ClientGeneral::BastionInspect { target });
    }

    /// bastion (UI-4 → UI-5): the latest inspector reply — `(target, payload)`;
    /// `payload: None` = nothing Bastion-tracked sits at the target.
    pub fn bastion_inspect(
        &self,
    ) -> Option<&(
        comp::bastion::BastionInspectTarget,
        Option<comp::bastion::BastionInspectKind>,
    )> {
        self.bastion_inspect.as_ref()
    }

    /// bastion (B2a): paint a designation region (server validates + echoes).
    /// B5.6b-2: `z_extent: Some(_)` switches to the surface-relative path —
    /// `region`'s XY is the footprint, `max.z` the paint-plane hint; the
    /// server resolves per-column surfaces and echoes exact bounds. `None`
    /// sends the region literally (legacy semantics).
    pub fn bastion_place_designation(
        &mut self,
        region: common::bastion::Region,
        kind: common::bastion::DesignationKind,
        z_extent: Option<common::bastion::ZExtent>,
    ) {
        self.send_msg(ClientGeneral::BastionPlaceDesignation {
            region,
            kind,
            z_extent,
        });
    }

    /// bastion (B2a): apply a divine influence (stub until B13).
    pub fn bastion_apply_influence(
        &mut self,
        target: Vec3<f32>,
        kind: common::bastion::InfluenceKind,
    ) {
        self.send_msg(ClientGeneral::BastionApplyInfluence { target, kind });
    }

    /// bastion (B2a): send a context-menu verb (stub until B3/B4).
    pub fn bastion_context_action(
        &mut self,
        target: common::bastion::ContextTarget,
        verb: common::bastion::ContextVerb,
    ) {
        self.send_msg(ClientGeneral::BastionContextAction { target, verb });
    }

    /// bastion (B3): found the player colony near `pos`.
    pub fn bastion_spawn_colony(&mut self, pos: Vec3<f32>, count: u8) {
        self.send_msg(ClientGeneral::BastionSpawnColony { pos, count });
    }

    /// bastion (B4): cancel designations in a region (releases claims).
    pub fn bastion_cancel_designation(&mut self, region: common::bastion::Region) {
        self.send_msg(ClientGeneral::BastionCancelDesignation { region });
    }

    pub fn start_spectate_entity(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.read_component_copied(entity) {
            self.send_msg(ClientGeneral::SpectateEntity(Some(uid)));
        } else {
            warn!("Spectating entity without a `Uid` component");
        }
    }

    pub fn stop_spectate_entity(&mut self) { self.send_msg(ClientGeneral::SpectateEntity(None)); }

    /// Checks whether a player can swap their weapon+ability `Loadout` settings
    /// and sends the `ControlAction` event that signals to do the swap.
    pub fn swap_loadout(&mut self) { self.control_action(ControlAction::SwapEquippedWeapons) }

    /// Determine whether the player is wielding, if they're even capable of
    /// being in a wield state.
    pub fn is_wielding(&self) -> Option<bool> {
        self.state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| cs.is_wield())
    }

    pub fn toggle_wield(&mut self) {
        match self.is_wielding() {
            Some(true) => self.control_action(ControlAction::Unwield),
            Some(false) => self.control_action(ControlAction::Wield),
            None => warn!("Can't toggle wield, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_sit(&mut self) {
        let is_sitting = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::Sit));

        match is_sitting {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Sit),
            None => warn!("Can't toggle sit, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_crawl(&mut self) {
        let is_crawling = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::Crawl));

        match is_crawling {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Crawl),
            None => warn!("Can't toggle crawl, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_dance(&mut self) {
        let is_dancing = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::Dance));

        match is_dancing {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Dance),
            None => warn!("Can't toggle dance, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn utter(&mut self, kind: UtteranceKind) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Utterance(kind)));
    }

    pub fn toggle_sneak(&mut self) {
        let is_sneaking = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(CharacterState::is_stealthy);

        match is_sneaking {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Sneak),
            None => warn!("Can't toggle sneak, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_glide(&mut self) {
        let using_glider = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::GlideWield(_) | CharacterState::Glide(_)));

        match using_glider {
            Some(true) => self.control_action(ControlAction::Unwield),
            Some(false) => self.control_action(ControlAction::GlideWield),
            None => warn!("Can't toggle glide, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn cancel_climb(&mut self) {
        let is_climbing = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::Climb(_)));

        match is_climbing {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => {},
            None => warn!("Can't stop climbing, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn handle_input(
        &mut self,
        input: InputKind,
        pressed: bool,
        select_pos: Option<Vec3<f32>>,
        target_entity: Option<EcsEntity>,
    ) {
        if pressed {
            self.control_action(ControlAction::StartInput {
                input,
                target_entity: target_entity.and_then(|e| self.state.read_component_copied(e)),
                select_pos,
            });
        } else {
            self.control_action(ControlAction::CancelInput { input });
        }
    }

    pub fn activate_portal(&mut self, portal: EcsEntity) {
        if let Some(portal_uid) = self.state.read_component_copied(portal) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::ActivatePortal(
                portal_uid,
            )));
        }
    }

    fn control_action(&mut self, control_action: ControlAction) {
        if let Some(controller) = self
            .state
            .ecs()
            .write_storage::<Controller>()
            .get_mut(self.entity())
        {
            controller.push_action(control_action);
        }
        self.send_msg(ClientGeneral::ControlAction(control_action));
    }

    fn control_event(&mut self, control_event: ControlEvent) {
        if let Some(controller) = self
            .state
            .ecs()
            .write_storage::<Controller>()
            .get_mut(self.entity())
        {
            controller.push_event(control_event.clone());
        }
        self.send_msg(ClientGeneral::ControlEvent(control_event));
    }

    pub fn view_distance(&self) -> Option<u32> { self.view_distance }

    pub fn server_view_distance_limit(&self) -> Option<u32> { self.server_view_distance_limit }

    pub fn loaded_distance(&self) -> f32 { self.loaded_distance }

    pub fn position(&self) -> Option<Vec3<f32>> {
        self.state
            .read_storage::<comp::Pos>()
            .get(self.entity())
            .map(|v| v.0)
    }

    /// Returns Weather::default if no player position exists.
    pub fn weather_at_player(&self) -> Weather {
        self.position()
            .map(|p| {
                let mut weather = self.state.weather_at(p.xy());
                weather.wind = self.weather.local_wind;
                weather
            })
            .unwrap_or_default()
    }

    pub fn current_chunk(&self) -> Option<Arc<TerrainChunk>> {
        let chunk_pos = Vec2::from(self.position()?)
            .map2(TerrainChunkSize::RECT_SIZE, |e: f32, sz| {
                (e as u32).div_euclid(sz) as i32
            });

        self.state.terrain().get_key_arc(chunk_pos).cloned()
    }

    /// Get spiral of chunks around the client with given radius, paired with
    /// each chunk's coordinate on the chunk grid
    pub fn chunks_around(&self, radius: i32) -> Option<Vec<(Arc<TerrainChunk>, Vec2<i32>)>> {
        let chunk_pos = Vec2::from(self.position()?)
            .map2(TerrainChunkSize::RECT_SIZE, |e: f32, sz| {
                (e as u32).div_euclid(sz) as i32
            });

        Some(
            Spiral2d::with_radius(radius)
                .filter_map(|coord| {
                    let pos = chunk_pos + coord;
                    self.state
                        .terrain()
                        .get_key_arc(pos)
                        .map(|chunk| (Arc::clone(chunk), pos))
                })
                .collect(),
        )
    }

    pub fn current<C>(&self) -> Option<C>
    where
        C: Component + Clone,
    {
        self.state.read_storage::<C>().get(self.entity()).cloned()
    }

    pub fn current_biome(&self) -> BiomeKind {
        match self.current_chunk() {
            Some(chunk) => chunk.meta().biome(),
            _ => BiomeKind::Void,
        }
    }

    pub fn current_site(&self) -> SiteKindMeta {
        let mut player_alt = 0.0;
        if let Some(position) = self.current::<comp::Pos>() {
            player_alt = position.0.z;
        }
        let mut terrain_alt = 0.0;
        let mut site = None;
        if let Some(chunk) = self.current_chunk() {
            terrain_alt = chunk.meta().alt();
            site = chunk.meta().site();
        }
        if player_alt < terrain_alt - 40.0 {
            if let Some(SiteKindMeta::Dungeon(dungeon)) = site {
                SiteKindMeta::Dungeon(dungeon)
            } else {
                SiteKindMeta::Cave
            }
        } else {
            site.unwrap_or_default()
        }
    }

    pub fn request_site_economy(&mut self, id: SiteId) {
        self.send_msg(ClientGeneral::RequestSiteInfo(id))
    }

    pub fn inventories(&self) -> ReadStorage<'_, comp::Inventory> { self.state.read_storage() }

    /// Send a chat message to the server.
    pub fn send_chat(&mut self, message: String) {
        self.send_msg(ClientGeneral::ChatMsg(comp::Content::Plain(message)));
    }

    /// Send a command to the server.
    pub fn send_command(&mut self, name: String, args: Vec<String>) {
        self.send_msg(ClientGeneral::Command(name, args));
    }

    /// Remove all cached terrain
    pub fn clear_terrain(&mut self) {
        self.state.clear_terrain();
        self.pending_chunks.clear();
    }

    pub fn place_block(&mut self, pos: Vec3<i32>, block: Block) {
        self.send_msg(ClientGeneral::PlaceBlock(pos, block));
    }

    pub fn remove_block(&mut self, pos: Vec3<i32>) {
        self.send_msg(ClientGeneral::BreakBlock(pos));
    }

    pub fn collect_block(&mut self, pos: Vec3<i32>) {
        self.control_action(ControlAction::InventoryAction(InventoryAction::Collect(
            pos,
        )));
    }

    pub fn perform_dialogue(&mut self, target: EcsEntity, dialogue: rtsim::Dialogue) {
        if let Some(target_uid) = self.state.read_component_copied(target) {
            // TODO: Add a way to do send-only chat
            // if let Some(msg) = dialogue.message().cloned() {
            //     self.send_msg(ClientGeneral::ChatMsg(msg));
            // }
            self.control_event(ControlEvent::Dialogue(target_uid, dialogue));
        }
    }

    pub fn do_talk(&mut self, tgt: Option<EcsEntity>) {
        if let Some(controller) = self
            .state
            .ecs()
            .write_storage::<comp::Controller>()
            .get_mut(self.entity())
        {
            controller.push_action(ControlAction::Talk(
                tgt.and_then(|tgt| self.state.read_component_copied(tgt)),
            ));
        }
    }

    pub fn change_ability(&mut self, slot: usize, new_ability: comp::ability::AuxiliaryAbility) {
        let auxiliary_key = self
            .inventories()
            .get(self.entity())
            .map_or((None, None), |inv| {
                let tool_kind = |slot| {
                    inv.equipped(slot).and_then(|item| match &*item.kind() {
                        ItemKind::Tool(tool) => Some(tool.kind),
                        _ => None,
                    })
                };

                (
                    tool_kind(EquipSlot::ActiveMainhand),
                    tool_kind(EquipSlot::ActiveOffhand),
                )
            });

        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::ChangeAbility {
            slot,
            auxiliary_key,
            new_ability,
        }))
    }

    pub fn waypoint(&self) -> &Option<String> { &self.waypoint }

    pub fn set_battle_mode(&mut self, battle_mode: BattleMode) {
        self.send_msg(ClientGeneral::SetBattleMode(battle_mode));
    }

    pub fn get_battle_mode(&self) -> BattleMode {
        let Some(uid) = self.uid() else {
            error!("Client entity does not have a Uid component");

            return BattleMode::PvP;
        };

        let Some(player_info) = self.player_list.get(&uid) else {
            error!("Client does not have PlayerInfo for its Uid");

            return BattleMode::PvP;
        };

        let Some(ref character_info) = player_info.character else {
            error!("Client does not have CharacterInfo for its PlayerInfo");

            return BattleMode::PvP;
        };

        character_info.battle_mode
    }

    /// Execute a single client tick, handle input and update the game state by
    /// the given duration.
    pub fn tick(&mut self, inputs: ControllerInputs, dt: Duration) -> Result<Vec<Event>, Error> {
        span!(_guard, "tick", "Client::tick");
        // This tick function is the centre of the Veloren universe. Most client-side
        // things are managed from here, and as such it's important that it
        // stays organised. Please consult the core developers before making
        // significant changes to this code. Here is the approximate order of
        // things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state of the
        //    game
        // 2) Handle messages from the server
        // 3) Go through any events (timer-driven or otherwise) that need handling and
        //    apply them to the state of the game
        // 4) Perform a single LocalState tick (i.e: update the world and entities in
        //    the world)
        // 5) Go through the terrain update queue and apply all changes to the terrain
        // 6) Sync information to the server
        // 7) Finish the tick, passing actions of the main thread back to the frontend

        // 1) Handle input from frontend.
        // Pass character actions from frontend input to the player's entity.
        if self.presence.is_some() {
            prof_span!("handle and send inputs");
            if let Err(e) = self
                .state
                .ecs()
                .write_storage::<Controller>()
                .entry(self.entity())
                .map(|entry| {
                    entry
                        .or_insert_with(Controller::default)
                        .inputs = inputs.clone();
                })
            {
                let entry = self.entity();
                error!(
                    ?e,
                    ?entry,
                    "Couldn't access controller component on client entity"
                );
            }
            self.send_msg_err(ClientGeneral::ControllerInputs(Box::new(inputs)))?;
        }

        // 2) Build up a list of events for this frame, to be passed to the frontend.
        let mut frontend_events = Vec::new();

        // Prepare for new events
        {
            prof_span!("Last<CharacterState> comps update");
            let ecs = self.state.ecs();
            let mut last_character_states = ecs.write_storage::<comp::Last<CharacterState>>();
            for (entity, _, character_state) in (
                &ecs.entities(),
                &ecs.read_storage::<comp::Body>(),
                &ecs.read_storage::<CharacterState>(),
            )
                .join()
            {
                if let Some(l) = last_character_states
                    .entry(entity)
                    .ok()
                    .map(|l| l.or_insert_with(|| comp::Last(character_state.clone())))
                    // TODO: since this just updates when the variant changes we should
                    // just store the variant to avoid the clone overhead
                    .filter(|l| !character_state.same_variant(&l.0))
                {
                    *l = comp::Last(character_state.clone());
                }
            }
        }

        // Handle new messages from the server.
        frontend_events.append(&mut self.handle_new_messages()?);

        // 3) Update client local data
        // Check if the invite has timed out and remove if so
        if self
            .invite
            .is_some_and(|(_, timeout, dur, _)| timeout.elapsed() > dur)
        {
            self.invite = None;
        }

        // Lerp the clientside weather.
        self.weather.update(&mut self.state.weather_grid_mut());

        if let Some(target_tod) = self.target_time_of_day {
            let mut tod = self.state.ecs_mut().write_resource::<TimeOfDay>();
            tod.0 = target_tod.0;
            self.target_time_of_day = None;
        }

        // Save dead hardcore character ids to avoid displaying in the character list
        // while the server is still in the process of deleting the character
        if self.current::<Hardcore>().is_some()
            && self.is_dead()
            && let Some(PresenceKind::Character(character_id)) = self.presence
        {
            self.character_being_deleted = Some(character_id);
        }

        // `APEX-T7.3a`: snapshot the CURRENT Controller for self.entity()
        // before state.tick() runs -- character_behavior::Sys drains it
        // (take_actions()) as part of this same tick, so this is the
        // exact value the transition this tick will consume.
        let pre_tick_controller = self
            .presence
            .is_some()
            .then(|| self.state.read_storage::<Controller>().get(self.entity()).cloned())
            .flatten();

        // 4) Tick the client's LocalState
        self.state.tick(
            Duration::from_secs_f64(dt.as_secs_f64() * self.dt_adjustment),
            true,
            None,
            &self.connected_server_constants,
            |_, _| {},
        );

        // `APEX-T7.3a`: Decision 3 -- a mount/carry transition terminates
        // the prediction history, checked before the capture below so a
        // frame predicted UNDER the new mount state never lands in a
        // buffer whose earlier entries predicted the old one.
        if self.presence.is_some() {
            let entity = self.entity();
            let is_mounted = self.state.read_storage::<Is<Rider>>().get(entity).is_some()
                || self.state.read_storage::<Is<VolumeRider>>().get(entity).is_some();
            if is_mounted != self.was_mounted_last_tick {
                self.prediction_buffer.clear_v1();
            }
            self.was_mounted_last_tick = is_mounted;

            // `APEX-T7.3a`: record this tick's predicted frame -- Time
            // and DeltaTime are read AFTER state.tick() because
            // State::tick advances both BEFORE dispatching systems
            // (common/state/src/state.rs), so their post-tick values are
            // exactly what character_behavior::Sys read this tick.
            if let Some(controller) = pre_tick_controller {
                let time = *self.state.ecs().read_resource::<Time>();
                let dt = *self.state.ecs().read_resource::<DeltaTime>();
                // Decision 2's world revision. `weather`: the same
                // latest_snapshot pattern the PlayerPhysics report above
                // already uses (APEX-T5.2). `touched_chunks`: DISCLOSED
                // approximation -- the entity's own chunk only, not
                // common_systems::phys::Sys's actual per-tick query
                // footprint (that system exposes no touched-chunk
                // tracking to capture from here without instrumenting a
                // hot path, out of T7.3a's scope). Conservative, not
                // exact: the entity's own chunk is always a SUBSET of
                // what the real query touches, so this can only cause
                // EXTRA invalidations on an unrelated chunk unload, never
                // a missed one -- the safe direction per Decision 2's own
                // "snap rather than substitute" preference.
                let weather = self.weather.latest_snapshot.unwrap_or(WeatherSnapshotIdV1::from_sequence_v1(0));
                let touched_chunks = self
                    .state
                    .read_storage::<comp::Pos>()
                    .get(entity)
                    .map(|pos| vec![pos.0.xy().as_::<i32>().wpos_to_cpos()])
                    .unwrap_or_default();
                let world_revision =
                    common::apex::prediction_boundary::WorldRevisionV1 { weather, touched_chunks };
                // `APEX-T7.3c-ii`: baseline-stamping, read at the same
                // moment as everything else this frame captures.
                // `last_server_sync_tick` and `tick` are both already
                // maintained by this client for other reasons (DET-NET-011/
                // 012's chronology witness, and the tick counter itself) --
                // this costs two extra reads, no new bookkeeping.
                let alignment = common::apex::prediction_boundary::FrameAlignmentV1 {
                    baseline_sync_tick: self.last_server_sync_tick,
                    ordinal: self.tick,
                };
                let outcome = self.prediction_buffer.push_v1(
                    common::apex::prediction_boundary::PredictedFrameV1 {
                        controller,
                        dt,
                        time,
                        world_revision,
                        alignment,
                    },
                );
                if let common::apex::prediction_boundary::PushOutcomeV1::BudgetExceeded {
                    attempted_bytes,
                    budget_bytes,
                } = outcome
                {
                    tracing::warn!(
                        attempted_bytes,
                        budget_bytes,
                        "T7.1 Decision 5: prediction buffer budget exceeded, clearing (snap, recorded)"
                    );
                    self.prediction_buffer.clear_v1();
                }
            }
        }

        // TODO: avoid emitting these in the first place OR actually use outcomes
        // generated locally on the client (if they can be deduplicated from
        // ones that the server generates or if the client can reliably generate
        // them (e.g. syncing skipping character states past certain
        // stages might skip points where outcomes are generated, however we might not
        // care about this?) and the server doesn't need to send them)
        let _ = self.state.ecs().fetch::<EventBus<Outcome>>().recv_all();

        // 5) Terrain
        self.tick_terrain()?;

        // Send a ping to the server once every second
        if self.state.get_program_time() - self.last_server_ping > 1. {
            self.send_msg_err(PingMsg::Ping)?;
            self.last_server_ping = self.state.get_program_time();
        }

        // 6) Update the server about the player's physics attributes.
        if self.presence.is_some()
            && let (Some(pos), Some(vel), Some(ori)) = (
                self.state.read_storage().get(self.entity()).cloned(),
                self.state.read_storage().get(self.entity()).cloned(),
                self.state.read_storage().get(self.entity()).cloned(),
            )
        {
            self.in_game_stream.send(ClientGeneral::PlayerPhysics {
                pos,
                vel,
                ori,
                physics_generation: self.force_update_generation,
                // APEX-T5.2: the snapshot this frame was predicted under.
                // Unknown before the first weather packet, which is a
                // real state and is reported as snapshot 0 rather than
                // as a guess.
                weather_snapshot: self
                    .weather
                    .latest_snapshot
                    .unwrap_or(WeatherSnapshotIdV1::from_sequence_v1(0)),
            })?;
        }

        /*
        // Output debug metrics
        if log_enabled!(Level::Info) && self.tick % 600 == 0 {
            let metrics = self
                .state
                .terrain()
                .iter()
                .fold(ChonkMetrics::default(), |a, (_, c)| a + c.get_metrics());
            info!("{:?}", metrics);
        }
        */

        // 7) Finish the tick, pass control back to the frontend.
        self.tick += 1;
        Ok(frontend_events)
    }

    /// Clean up the client after a tick.
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    /// Handles terrain addition and removal.
    ///
    /// Removes old terrain chunks outside the view distance.
    /// Sends requests for missing chunks within the view distance.
    fn tick_terrain(&mut self) -> Result<(), Error> {
        let entity_pos = self
            .state
            .read_storage::<comp::Pos>()
            .get(self.entity())
            .cloned();
        // bastion (B1.6): when the god-camera anchor is set, terrain streams
        // around *it* instead of the avatar (the avatar's area is still
        // retained below so its local physics keeps ground under it).
        let pos = self.bastion_terrain_anchor.map(comp::Pos).or(entity_pos);
        if let (Some(pos), Some(view_distance)) = (pos, self.view_distance) {
            prof_span!("terrain");
            let chunk_pos = self.state.terrain().pos_key(pos.0.map(|e| e as i32));
            // bastion: second retention center — the avatar — when anchored.
            let avatar_chunk_pos = self
                .bastion_terrain_anchor
                .and(entity_pos)
                .map(|p| self.state.terrain().pos_key(p.0.map(|e| e as i32)));

            // Remove chunks that are too far from the player.
            let mut chunks_to_remove = Vec::new();
            self.state.terrain().iter().for_each(|(key, _)| {
                // Subtract 2 from the offset before computing squared magnitude
                // 1 for the chunks needed bordering other chunks for meshing
                // 1 as a buffer so that if the player moves back in that direction the chunks
                //   don't need to be reloaded
                // Take the minimum of the adjusted difference vs the view_distance + 1 to
                //   prevent magnitude_squared from overflowing

                let too_far = |center: Vec2<i32>| {
                    (center - key)
                        .map(|e: i32| (e.unsigned_abs()).saturating_sub(2).min(view_distance + 1))
                        .magnitude_squared()
                        > view_distance.pow(2)
                };
                if too_far(chunk_pos) && avatar_chunk_pos.is_none_or(too_far) {
                    chunks_to_remove.push(key);
                }
            });
            for key in chunks_to_remove {
                self.state.remove_chunk(key);
            }

            let mut current_tick_send_chunk_requests = 0;
            // Request chunks from the server.
            self.loaded_distance = ((view_distance * TerrainChunkSize::RECT_SIZE.x) as f32).powi(2);
            // +1 so we can find a chunk that's outside the vd for better fog
            for dist in 0..view_distance as i32 + 1 {
                // Only iterate through chunks that need to be loaded for circular vd
                // The (dist - 2) explained:
                // -0.5 because a chunk is visible if its corner is within the view distance
                // -0.5 for being able to move to the corner of the current chunk
                // -1 because chunks are not meshed if they don't have all their neighbors
                //     (notice also that view_distance is decreased by 1)
                //     (this subtraction on vd is omitted elsewhere in order to provide
                //     a buffer layer of loaded chunks)
                let top = if 2 * (dist - 2).max(0).pow(2) > (view_distance - 1).pow(2) as i32 {
                    ((view_distance - 1).pow(2) as f32 - (dist - 2).pow(2) as f32)
                        .sqrt()
                        .round() as i32
                        + 1
                } else {
                    dist
                };

                let mut skip_mode = false;
                for i in -top..top + 1 {
                    let keys = [
                        chunk_pos + Vec2::new(dist, i),
                        chunk_pos + Vec2::new(i, dist),
                        chunk_pos + Vec2::new(-dist, i),
                        chunk_pos + Vec2::new(i, -dist),
                    ];

                    for key in keys.iter() {
                        let dist_to_player = (TerrainGrid::key_chunk(*key).map(|x| x as f32)
                            + TerrainChunkSize::RECT_SIZE.map(|x| x as f32) / 2.0)
                            .distance_squared(pos.0.into());

                        let terrain = self.state.terrain();
                        if let Some(chunk) = terrain.get_key_arc(*key) {
                            if !skip_mode && !terrain.contains_key_real(*key) {
                                let chunk = Arc::clone(chunk);
                                drop(terrain);
                                self.state.insert_chunk(*key, chunk);
                            }
                        } else {
                            drop(terrain);
                            if !skip_mode && !self.pending_chunks.contains_key(key) {
                                const TOTAL_PENDING_CHUNKS_LIMIT: usize = 12;
                                const CURRENT_TICK_PENDING_CHUNKS_LIMIT: usize = 2;
                                // bastion (B1.6): a god-camera re-center swaps
                                // in a large crescent of missing chunks at
                                // once; vanilla's walking-pace throttle takes
                                // ages to fill it, so allow more in flight.
                                let (total_limit, tick_limit) =
                                    if self.bastion_terrain_anchor.is_some() {
                                        (TOTAL_PENDING_CHUNKS_LIMIT * 4, 8)
                                    } else {
                                        (
                                            TOTAL_PENDING_CHUNKS_LIMIT,
                                            CURRENT_TICK_PENDING_CHUNKS_LIMIT,
                                        )
                                    };
                                if self.pending_chunks.len() < total_limit
                                    && current_tick_send_chunk_requests < tick_limit
                                {
                                    self.send_msg_err(ClientGeneral::TerrainChunkRequest {
                                        key: *key,
                                    })?;
                                    current_tick_send_chunk_requests += 1;
                                    self.pending_chunks.insert(*key, Instant::now());
                                } else {
                                    skip_mode = true;
                                }
                            }

                            if dist_to_player < self.loaded_distance {
                                self.loaded_distance = dist_to_player;
                            }
                        }
                    }
                }
            }
            self.loaded_distance = self.loaded_distance.sqrt()
                - ((TerrainChunkSize::RECT_SIZE.x as f32 / 2.0).powi(2)
                    + (TerrainChunkSize::RECT_SIZE.y as f32 / 2.0).powi(2))
                .sqrt();

            // If chunks are taking too long, assume they're no longer pending.
            let now = Instant::now();
            self.pending_chunks
                .retain(|_, created| now.duration_since(*created) < Duration::from_secs(3));
        }

        if let Some(lod_pos) = pos.map(|p| p.0.xy()).or(self.lod_pos_fallback) {
            // Manage LoD zones
            let lod_zone = lod_pos.map(|e| lod::from_wpos(e as i32));

            // Request LoD zones that are in range
            if self
                .lod_last_requested
                .is_none_or(|i| i.elapsed() > Duration::from_secs(5))
                && let Some(rpos) = Spiral2d::new()
                    .take((1 + self.lod_distance.ceil() as i32 * 2).pow(2) as usize)
                    .filter(|rpos| !self.lod_zones.contains_key(&(lod_zone + *rpos)))
                    .min_by_key(|rpos| rpos.magnitude_squared())
                    .filter(|rpos| {
                        rpos.map(|e| e as f32).magnitude() < (self.lod_distance - 0.5).max(0.0)
                    })
            {
                self.send_msg_err(ClientGeneral::LodZoneRequest {
                    key: lod_zone + rpos,
                })?;
                self.lod_last_requested = Some(Instant::now());
            }

            // Cull LoD zones out of range
            self.lod_zones.retain(|p, _| {
                (*p - lod_zone).map(|e| e as f32).magnitude_squared() < self.lod_distance.powi(2)
            });
        }

        Ok(())
    }

    fn handle_server_msg(
        &mut self,
        frontend_events: &mut Vec<Event>,
        msg: ServerGeneral,
    ) -> Result<(), Error> {
        prof_span!("handle_server_msg");
        match msg {
            ServerGeneral::Disconnect(reason) => match reason {
                DisconnectReason::Shutdown => return Err(Error::ServerShutdown),
                DisconnectReason::Kicked(reason) => return Err(Error::Kicked(reason)),
                DisconnectReason::Banned(info) => return Err(Error::Banned(info)),
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Init(list)) => {
                // DET-NET-015: wire payload is now a Uid-sorted Vec; rebuild the
                // local lookup map from it.
                self.player_list = list.into_iter().collect()
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Add(uid, player_info)) => {
                if let Some(old_player_info) = self.player_list.insert(uid, player_info.clone()) {
                    warn!(
                        "Received msg to insert {} with uid {} into the player list but there was \
                         already an entry for {} with the same uid that was overwritten!",
                        player_info.player_alias, uid, old_player_info.player_alias
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Moderator(uid, moderator)) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    player_info.is_moderator = moderator;
                } else {
                    warn!(
                        "Received msg to update admin status of uid {}, but they were not in the \
                         list.",
                        uid
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::SelectedCharacter(
                uid,
                char_info,
            )) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    player_info.character = Some(char_info);
                } else {
                    warn!(
                        "Received msg to update character info for uid {}, but they were not in \
                         the list.",
                        uid
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::ExitCharacter(uid)) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    if player_info.character.is_none() {
                        debug!(?player_info.player_alias, ?uid, "Received PlayerListUpdate::ExitCharacter for a player who wasnt ingame");
                    }
                    player_info.character = None;
                } else {
                    debug!(
                        ?uid,
                        "Received PlayerListUpdate::ExitCharacter for a nonexitent player"
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Remove(uid)) => {
                // Instead of removing players, mark them as offline because we need to
                // remember the names of disconnected players in chat.
                //
                // TODO: consider alternatives since this leads to an ever growing list as
                // players log out and in. Keep in mind we might only want to
                // keep only so many messages in chat the history. We could
                // potentially use an ID that's more persistent than the Uid.
                // One of the reasons we don't just store the string of the player name
                // into the message is to make alias changes reflected in older messages.

                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    if player_info.is_online {
                        player_info.is_online = false;
                    } else {
                        warn!(
                            "Received msg to remove uid {} from the player list by they were \
                             already marked offline",
                            uid
                        );
                    }
                } else {
                    warn!(
                        "Received msg to remove uid {} from the player list by they weren't in \
                         the list!",
                        uid
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Alias(uid, new_name)) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    player_info.player_alias = new_name;
                } else {
                    warn!(
                        "Received msg to alias player with uid {} to {} but this uid is not in \
                         the player list",
                        uid, new_name
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::UpdateBattleMode(
                uid,
                battle_mode,
            )) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    if let Some(ref mut character_info) = player_info.character {
                        character_info.battle_mode = battle_mode;
                    } else {
                        warn!(
                            "Received msg to update battle mode of uid {} to {:?} but this player \
                             does not have a character",
                            uid, battle_mode
                        );
                    }
                } else {
                    warn!(
                        "Received msg to update battle mode of uid {} to {:?} but this uid is not \
                         in the player list",
                        uid, battle_mode
                    );
                }
            },
            ServerGeneral::ChatMsg(m) => frontend_events.push(Event::Chat(m)),
            ServerGeneral::ChatMode(m) => {
                self.chat_mode = m;
            },
            ServerGeneral::SetPlayerEntity(uid) => {
                if let Some(entity) = self.state.ecs().entity_from_uid(uid) {
                    let old_player_entity = mem::replace(
                        &mut *self.state.ecs_mut().write_resource(),
                        PlayerEntity(Some(entity)),
                    );
                    if let Some(old_entity) = old_player_entity.0 {
                        // Transfer controller to the new entity.
                        let mut controllers = self.state.ecs().write_storage::<Controller>();
                        if let Some(controller) = controllers.remove(old_entity)
                            && let Err(e) = controllers.insert(entity, controller)
                        {
                            error!(
                                ?e,
                                "Failed to insert controller when setting new player entity!"
                            );
                        }
                    }
                    if let Some(presence) = self.presence {
                        self.presence = Some(match presence {
                            PresenceKind::Spectator => PresenceKind::Spectator,
                            PresenceKind::LoadingCharacter(_) => PresenceKind::Possessor,
                            PresenceKind::Character(_) => PresenceKind::Possessor,
                            PresenceKind::Possessor => PresenceKind::Possessor,
                            // bastion (ROW-COLONY-PRESENCE): a colony
                            // presence has no `Client`, so no client ever
                            // runs this code for one -- kept exhaustive.
                            PresenceKind::Colony => PresenceKind::Possessor,
                        });
                    }
                    // Clear pending trade
                    self.pending_trade = None;
                } else {
                    return Err(Error::Other("Failed to find entity from uid.".into()));
                }
            },
            ServerGeneral::TimeOfDay(time_of_day, calendar, new_time, time_scale) => {
                self.target_time_of_day = Some(time_of_day);
                *self.state.ecs_mut().write_resource() = calendar;
                *self.state.ecs_mut().write_resource() = time_scale;
                let mut time = self.state.ecs_mut().write_resource::<Time>();
                // Avoid side-eye from Einstein
                // If new time from server is at least 5 seconds ahead, replace client time.
                // Otherwise try to slightly twean client time (by 1%) to keep it in line with
                // server time.
                self.dt_adjustment = if new_time.0 > time.0 + 5.0 {
                    *time = new_time;
                    1.0
                } else if new_time.0 > time.0 {
                    1.01
                } else {
                    0.99
                };
            },
            ServerGeneral::EntitySync(entity_sync_package) => {
                // DET-NET-011 (v6, stage 1): cross-stream chronology witness —
                // a stamped package whose server tick regresses against the
                // newest seen is logged (0 = unstamped legacy, skipped).
                if entity_sync_package.sync_tick != 0 {
                    if entity_sync_package.sync_tick < self.last_server_sync_tick {
                        tracing::warn!(
                            got = entity_sync_package.sync_tick,
                            newest = self.last_server_sync_tick,
                            "DET-NET-011: EntitySync arrived with a regressed server tick"
                        );
                    }
                    self.last_server_sync_tick =
                        self.last_server_sync_tick.max(entity_sync_package.sync_tick);
                }
                let uid = self.uid();
                self.state
                    .ecs_mut()
                    .apply_entity_sync_package(entity_sync_package, uid);
            },
            ServerGeneral::CompSync(comp_sync_package, physics_generation) => {
                // DET-NET-012 (v6, stage 1): same chronology witness.
                let sync_tick = comp_sync_package.sync_tick;
                if sync_tick != 0 {
                    if sync_tick < self.last_server_sync_tick {
                        tracing::warn!(
                            got = sync_tick,
                            newest = self.last_server_sync_tick,
                            "DET-NET-012: CompSync arrived with a regressed server tick"
                        );
                    }
                    self.last_server_sync_tick = self.last_server_sync_tick.max(sync_tick);
                }
                self.force_update_generation = physics_generation;

                // `APEX-T7.3c-ii`: snapshot the client's OWN current
                // belief BEFORE the authoritative write below overwrites
                // it -- reconciliation needs both "what I believed" and
                // "what the server just said" for the same tick, and
                // this is the only point at which the first of those is
                // still readable.
                let own_entity_touched = self
                    .uid()
                    .is_some_and(|uid| comp_sync_package.comp_updates.iter().any(|(u, _)| *u == uid.0));
                let pre_sync_rolling = (own_entity_touched && self.presence.is_some())
                    .then(|| read_rolling_state_v1(&self.state, self.entity()))
                    .flatten();

                self.state
                    .ecs_mut()
                    .apply_comp_sync_package(comp_sync_package);

                if let Some(current) = pre_sync_rolling {
                    let entity = self.entity();
                    if let Some(authoritative) = read_rolling_state_v1(&self.state, entity) {
                        let outcome = {
                            let read_data =
                                common_systems::character_behavior::ReadData::fetch(self.state.ecs());
                            let id_maps = specs::Read::<IdMaps>::fetch(self.state.ecs());
                            common_systems::reconciliation::reconcile_v1(
                                &read_data,
                                &id_maps,
                                entity,
                                &mut self.prediction_buffer,
                                &current,
                                &authoritative,
                                sync_tick,
                                physics_generation,
                                |chunk| self.state.terrain().get_key_arc(chunk).is_some(),
                                |snapshot| {
                                    matches!(
                                        self.weather.snapshots.wind_at_v1(snapshot),
                                        common::apex::weather_snapshot::PredictionWindSourceV1::Snapshot(_)
                                    )
                                },
                            )
                        };
                        match outcome {
                            common_systems::reconciliation::ReconciliationOutcomeV1::StaleCorrection {
                                buffer_generation,
                                got_generation,
                            } => {
                                // `APEX-T7.4` item A's own required test,
                                // live: an out-of-order/duplicate CompSync
                                // is rejected here, before the buffer was
                                // ever touched -- nothing to undo.
                                tracing::debug!(
                                    ?buffer_generation,
                                    ?got_generation,
                                    "stale CompSync generation rejected, prediction history untouched"
                                );
                            },
                            common_systems::reconciliation::ReconciliationOutcomeV1::Agreed { .. } => {},
                            common_systems::reconciliation::ReconciliationOutcomeV1::Replayed {
                                final_rolling,
                                position_correction_distance,
                                ..
                            } => {
                                self.correction_magnitude_metrics.record_correction_v1(position_correction_distance);
                                write_rolling_state_v1(&self.state, entity, &final_rolling);
                            },
                            common_systems::reconciliation::ReconciliationOutcomeV1::Snapped { .. } => {},
                        }
                    }
                }
            },
            ServerGeneral::CreateEntity(entity_package) => {
                self.state.ecs_mut().apply_entity_package(entity_package);
            },
            ServerGeneral::DeleteEntity(entity_uid) => {
                if self.uid() != Some(entity_uid) {
                    self.state
                        .ecs_mut()
                        .delete_entity_and_clear_uid_mapping(entity_uid);
                }
            },
            ServerGeneral::Notification(n) => {
                let Notification::WaypointSaved { location_name } = n.clone();
                self.waypoint = Some(location_name);

                frontend_events.push(Event::Notification(UserNotification::WaypointUpdated));
            },
            ServerGeneral::PluginData(d) => {
                let plugin_len = d.len();
                tracing::info!(?plugin_len, "plugin data");
                frontend_events.push(Event::PluginDataReceived(d));
            },
            // APEX-T2.5.10: typed artifact wire is defined but DORMANT
            // until the .11 bootstrap consumes it through the verified
            // collector. Ignore-with-warning, never panic on wire input.
            ServerGeneral::PluginArtifactData(r) => {
                tracing::warn!(
                    ordinal = r.descriptor.ordinal,
                    "PluginArtifactData before the T2.5.11 bootstrap path is active; ignoring"
                );
            },
            ServerGeneral::SetPlayerRole(role) => {
                debug!(?role, "Updating client role");
                self.role = role;
            },
            // W3 renderer-bench: compute this client's ClientProjection
            // root from ITS replicated view and ack. Gated per-process on
            // BASTION_RENDERER_BENCH_ACK=1; inert otherwise.
            ServerGeneral::RendererBenchFrame(ann) => {
                self.handle_renderer_bench_announce(ann);
            },
            _ => unreachable!("Not a general msg"),
        }
        Ok(())
    }

    fn handle_server_in_game_msg(
        &mut self,
        frontend_events: &mut Vec<Event>,
        msg: ServerGeneral,
    ) -> Result<(), Error> {
        prof_span!("handle_server_in_game_msg");
        match msg {
            ServerGeneral::GroupUpdate(change_notification) => {
                use comp::group::ChangeNotification::*;
                // Note: we use a hashmap since this would not work with entities outside
                // the view distance
                match change_notification {
                    Added(uid, role) => {
                        // Check if this is a newly formed group by looking for absence of
                        // other non pet group members
                        if !matches!(role, group::Role::Pet)
                            && !self
                                .group_members
                                .values()
                                .any(|r| !matches!(r, group::Role::Pet))
                        {
                            frontend_events
                                // TODO: localise
                                .push(Event::Chat(comp::ChatType::Meta.into_plain_msg(
                                    "Type /g or /group to chat with your group members",
                                )));
                        }
                        if let Some(player_info) = self.player_list.get(&uid) {
                            frontend_events.push(Event::Chat(
                                // TODO: localise, uses deprecated personalize_alias
                                #[expect(deprecated, reason = "i18n alias")]
                                comp::ChatType::GroupMeta("Group".into()).into_plain_msg(format!(
                                    "[{}] joined group",
                                    self.personalize_alias(uid, player_info.player_alias.clone())
                                )),
                            ));
                        }
                        if self.group_members.insert(uid, role) == Some(role) {
                            warn!(
                                "Received msg to add uid {} to the group members but they were \
                                 already there",
                                uid
                            );
                        }
                    },
                    Removed(uid) => {
                        if let Some(player_info) = self.player_list.get(&uid) {
                            frontend_events.push(Event::Chat(
                                // TODO: localise, uses deprecated personalize_alias
                                #[expect(deprecated, reason = "i18n alias")]
                                comp::ChatType::GroupMeta("Group".into()).into_plain_msg(format!(
                                    "[{}] left group",
                                    self.personalize_alias(uid, player_info.player_alias.clone())
                                )),
                            ));
                            frontend_events.push(Event::MapMarker(
                                comp::MapMarkerUpdate::GroupMember(uid, MapMarkerChange::Remove),
                            ));
                        }
                        if self.group_members.remove(&uid).is_none() {
                            warn!(
                                "Received msg to remove uid {} from group members but by they \
                                 weren't in there!",
                                uid
                            );
                        }
                    },
                    NewLeader(leader) => {
                        self.group_leader = Some(leader);
                    },
                    NewGroup { leader, members } => {
                        self.group_leader = Some(leader);
                        self.group_members = members.into_iter().collect();
                        // Currently add/remove messages treat client as an implicit member
                        // of the group whereas this message explicitly includes them so to
                        // be consistent for now we will remove the client from the
                        // received hashset
                        if let Some(uid) = self.uid() {
                            self.group_members.remove(&uid);
                        }
                        frontend_events.push(Event::MapMarker(comp::MapMarkerUpdate::ClearGroup));
                    },
                    NoGroup => {
                        self.group_leader = None;
                        self.group_members = HashMap::new();
                        frontend_events.push(Event::MapMarker(comp::MapMarkerUpdate::ClearGroup));
                    },
                }
            },
            ServerGeneral::Invite {
                inviter,
                timeout,
                kind,
            } => {
                self.invite = Some((inviter, Instant::now(), timeout, kind));
            },
            ServerGeneral::InvitePending(uid) => {
                if !self.pending_invites.insert(uid) {
                    warn!("Received message about pending invite that was already pending");
                }
            },
            ServerGeneral::InviteComplete {
                target,
                answer,
                kind,
            } => {
                if !self.pending_invites.remove(&target) {
                    warn!(
                        "Received completed invite message for invite that was not in the list of \
                         pending invites"
                    )
                }
                frontend_events.push(Event::InviteComplete {
                    target,
                    answer,
                    kind,
                });
            },
            ServerGeneral::GroupInventoryUpdate(item, uid) => {
                frontend_events.push(Event::GroupInventoryUpdate(item, uid));
            },
            // Cleanup for when the client goes back to the `presence = None`
            ServerGeneral::ExitInGameSuccess => {
                self.presence = None;
                self.clean_state();
            },
            ServerGeneral::InventoryUpdate(inventory, events) => {
                let mut update_inventory = false;
                for event in events.iter() {
                    match event {
                        InventoryUpdateEvent::BlockCollectFailed { .. } => {},
                        InventoryUpdateEvent::EntityCollectFailed { .. } => {},
                        _ => update_inventory = true,
                    }
                }
                if update_inventory {
                    // Push the updated inventory component to the client
                    // FIXME: Figure out whether this error can happen under normal gameplay,
                    // if not find a better way to handle it, if so maybe consider kicking the
                    // client back to login?
                    let entity = self.entity();
                    if let Err(e) = self
                        .state
                        .ecs_mut()
                        .write_storage()
                        .insert(entity, inventory)
                    {
                        warn!(
                            ?e,
                            "Received an inventory update event for client entity, but this \
                             entity was not found... this may be a bug."
                        );
                    }
                }

                self.update_available_recipes();

                frontend_events.push(Event::InventoryUpdated(events));
            },
            ServerGeneral::Dialogue(sender, dialogue) => {
                frontend_events.push(Event::Dialogue(sender, dialogue));
            },
            ServerGeneral::SetViewDistance(vd) => {
                self.view_distance = Some(vd);
                frontend_events.push(Event::SetViewDistance(vd));
                // If the server is correcting client vd selection we assume this is the max
                // allowed view distance.
                self.server_view_distance_limit = Some(vd);
            },
            ServerGeneral::Outcomes(outcomes) => {
                frontend_events.extend(outcomes.into_iter().map(Event::Outcome))
            },
            ServerGeneral::Knockback(impulse) => {
                self.state
                    .ecs()
                    .read_resource::<EventBus<LocalEvent>>()
                    .emit_now(LocalEvent::ApplyImpulse {
                        entity: self.entity(),
                        impulse,
                    });
            },
            ServerGeneral::UpdatePendingTrade(id, trade, pricing) => {
                trace!("UpdatePendingTrade {:?} {:?}", id, trade);
                self.pending_trade = Some((id, trade, pricing));
            },
            ServerGeneral::FinishedTrade(result) => {
                if let Some((_, trade, _)) = self.pending_trade.take() {
                    frontend_events.push(Event::TradeComplete { result, trade })
                }
            },
            ServerGeneral::SiteEconomy(economy) => {
                if let Some(rich) = self.sites.get_mut(&economy.id) {
                    rich.economy = Some(economy);
                }
            },
            ServerGeneral::MapMarker(event) => {
                frontend_events.push(Event::MapMarker(event));
            },
            ServerGeneral::WeatherUpdate(weather, snapshot) => {
                self.weather.weather_update(weather, snapshot);
            },
            ServerGeneral::LocalWindUpdate(wind, snapshot) => {
                self.weather.local_wind_update(wind, snapshot);
            },
            ServerGeneral::SpectatePosition(pos) => {
                frontend_events.push(Event::SpectatePosition(pos));
            },
            ServerGeneral::UpdateRecipes => {
                self.update_available_recipes();
            },
            ServerGeneral::Gizmos(gizmos) => frontend_events.push(Event::Gizmos(gizmos)),
            ServerGeneral::BastionDesignation {
                region,
                kind,
                z_extent,
            } => {
                // bastion (B2a): server-validated designation echo — kept for
                // the overlay render. B4 replaces this list with job-board
                // state.
                self.bastion_designations.push((region, kind, z_extent));
                self.bastion_designations_rev += 1;
            },
            ServerGeneral::BastionDesignationRemoved { region } => {
                // bastion (B5.5): subtract the erased region from every
                // stored rect (exact 3D AABB subtraction, ≤6 pieces each);
                // untouched rects pass through, covered rects vanish.
                let old = std::mem::take(&mut self.bastion_designations);
                for (r, kind, extent) in old {
                    if r.intersects(&region) {
                        self.bastion_designations
                            .extend(r.subtract(&region).into_iter().map(|p| (p, kind, extent)));
                    } else {
                        self.bastion_designations.push((r, kind, extent));
                    }
                }
                self.bastion_designations_rev += 1;
            },
            ServerGeneral::BastionInspectInfo { target, payload } => {
                // bastion (UI-4): the inspector reply — latest wins (the
                // HUD shows one panel; stale replies for other targets
                // are simply overwritten).
                self.bastion_inspect = Some((target, payload));
            },
            _ => unreachable!("Not a in_game message"),
        }
        Ok(())
    }

    fn handle_server_terrain_msg(&mut self, msg: ServerGeneral) -> Result<(), Error> {
        prof_span!("handle_server_terrain_mgs");
        match msg {
            ServerGeneral::TerrainChunkUpdate { key, chunk } => {
                if let Some(chunk) = chunk.ok().and_then(|c| c.to_chunk()) {
                    self.state.insert_chunk(key, Arc::new(chunk));
                }
                self.pending_chunks.remove(&key);
            },
            ServerGeneral::LodZoneUpdate { key, zone } => {
                self.lod_zones.insert(key, zone);
                self.lod_last_requested = None;
            },
            ServerGeneral::TerrainBlockUpdates(blocks) => {
                // DET-NET-014: the payload is now a position-sorted Vec; apply
                // in that canonical order (was a HashMap drained in seed order).
                if let Some(blocks) = blocks.decompress() {
                    blocks.into_iter().for_each(|(pos, block)| {
                        self.state.set_block(pos, block);
                    });
                }
            },
            _ => unreachable!("Not a terrain message"),
        }
        Ok(())
    }

    fn handle_server_character_screen_msg(
        &mut self,
        events: &mut Vec<Event>,
        msg: ServerGeneral,
    ) -> Result<(), Error> {
        prof_span!("handle_server_character_screen_msg");
        match msg {
            ServerGeneral::CharacterListUpdate(character_list) => {
                self.character_list.characters = character_list;
                if self.character_being_deleted.is_some() {
                    if let Some(pos) = self
                        .character_list
                        .characters
                        .iter()
                        .position(|x| x.character.id == self.character_being_deleted)
                    {
                        self.character_list.characters.remove(pos);
                    } else {
                        self.character_being_deleted = None;
                    }
                }
                self.character_list.loading = false;
            },
            ServerGeneral::CharacterActionError(error) => {
                warn!("CharacterActionError: {:?}.", error);
                events.push(Event::CharacterError(error));
            },
            ServerGeneral::CharacterDataLoadResult(Ok(metadata)) => {
                trace!("Handling join result by server");
                events.push(Event::CharacterJoined(metadata));
            },
            ServerGeneral::CharacterDataLoadResult(Err(error)) => {
                trace!("Handling join error by server");
                self.presence = None;
                self.clean_state();
                events.push(Event::CharacterError(error));
            },
            ServerGeneral::CharacterCreated(character_id) => {
                events.push(Event::CharacterCreated(character_id));
            },
            ServerGeneral::CharacterEdited(character_id) => {
                events.push(Event::CharacterEdited(character_id));
            },
            ServerGeneral::CharacterSuccess => debug!("client is now in ingame state on server"),
            ServerGeneral::SpectatorSuccess(spawn_point) => {
                events.push(Event::StartSpectate(spawn_point));
                debug!("client is now in ingame state on server");
            },
            _ => unreachable!("Not a character_screen msg"),
        }
        Ok(())
    }

    fn handle_ping_msg(&mut self, msg: PingMsg) -> Result<(), Error> {
        prof_span!("handle_ping_msg");
        match msg {
            PingMsg::Ping => {
                self.send_msg_err(PingMsg::Pong)?;
            },
            PingMsg::Pong => {
                self.last_server_pong = self.state.get_program_time();
                self.last_ping_delta = self.state.get_program_time() - self.last_server_ping;

                // Maintain the correct number of deltas for calculating the rolling average
                // ping. The client sends a ping to the server every second so we should be
                // receiving a pong reply roughly every second.
                while self.ping_deltas.len() > PING_ROLLING_AVERAGE_SECS - 1 {
                    self.ping_deltas.pop_front();
                }
                self.ping_deltas.push_back(self.last_ping_delta);
            },
        }
        Ok(())
    }

    /// `T3.3.10`: V1/Legacy receive-helper selector for one semantic
    /// stream, the client-side counterpart of the server's
    /// `try_recv_all_dispatch` (`server/src/sys/msg/mod.rs`). Not a
    /// generic free function taking a handler closure like the
    /// server's: the client's streams AND its handler methods both live
    /// on `self`, so a closure capturing `self` for the handler call
    /// would conflict with the sibling `&mut self.<stream>` /
    /// `&mut self.semantic_receive_state` borrows in the same call
    /// expression. Draining into a `Vec` first (no handler calls while
    /// `self`'s fields are borrowed), then handling each message
    /// afterward, sidesteps that entirely -- `handle_messages` below
    /// does the actual per-message dispatch to `self.handle_server_*`.
    /// Cursor advance still happens strictly before this function
    /// returns each frame to the caller (packet: "cursor does not
    /// advance on validation failure; it advances before handler
    /// call") -- only the RELATIVE ORDER of "handler runs for frame N"
    /// vs. "cursor already advanced past frame N+1" changes from the
    /// server's per-frame interleaving to a batch-then-handle split,
    /// which only matters if a handler errors mid-batch; that error
    /// tears down the whole connection either way (`Result<u64, Error>`
    /// propagates out of `handle_messages` to the caller's disconnect
    /// path), so a handful of already-validated-but-now-orphaned cursor
    /// advances on a connection that's about to die are inert.
    fn drain_semantic_stream_v1(
        stream: &mut Stream,
        receive_state: &mut Option<common_net::msg::envelope::SemanticReceiveStateV1>,
        semantic_stream: common_net::msg::envelope::SemanticStreamIdV1,
        metrics: &common_net::msg::envelope::SemanticIngressMetricsV1,
    ) -> Result<Vec<DrainedFrameV1>, Error> {
        let mut out = Vec::new();
        while let Some(raw) = stream.try_recv::<Vec<u8>>()? {
            let Some(state) = receive_state.as_ref() else {
                warn!("received a semantic V1 frame with no active attachment; dropping");
                continue;
            };
            match Self::validate_semantic_frame_v1(&raw, state, semantic_stream) {
                Ok((decoded, causality, checkpoint)) => {
                    let receive_state_mut = receive_state.as_mut().expect("checked Some above");
                    let advance_result = receive_state_mut.advance_expected(semantic_stream);
                    if advance_result.is_err() {
                        warn!("semantic receive sequence exhausted; dropping message");
                        metrics.record_terminal(
                            common_net::msg::envelope::SemanticProtocolTerminalV1::SequenceExhausted,
                            semantic_stream,
                        );
                        continue;
                    }
                    // `T3.3.17`: commit the snapshot watermark strictly
                    // AFTER the sequence cursor advance succeeds, same
                    // "cursor does not advance on validation failure; it
                    // advances before handler call" ordering already
                    // used for sequence.
                    if let Some(snapshot) = causality.snapshot {
                        receive_state_mut.commit_snapshot(snapshot);
                    }
                    let sequence = receive_state_mut.next_expected_for(semantic_stream).get() - 1;
                    out.push(DrainedFrameV1 { msg: decoded, checkpoint, sequence });
                },
                Err(reject) => {
                    warn!(?reject, "semantic ingress rejected a frame");
                    metrics.record_reject(&reject, semantic_stream);
                },
            }
        }
        Ok(out)
    }


    /// `APEX-T3.4.20c`: one step of the checkpoint runtime. Pure over the
    /// runtime (no `&self`), so the caller can hold it out of `Client`
    /// while it runs. Returns the records to hand to the ordinary
    /// per-stream handlers -- empty while a checkpoint is still aligning
    /// -- plus the receipt to acknowledge once it commits.
    #[expect(clippy::type_complexity)]
    fn checkpoint_step_v1(
        rt: &mut ClientCheckpointRuntimeV1,
        expected_binding: common_net::msg::envelope::ActiveSessionBindingV1,
        stream: common_net::msg::envelope::SemanticStreamIdV1,
        frame: DrainedFrameV1,
    ) -> Result<
        (
            Vec<(common_net::msg::envelope::SemanticStreamIdV1, ServerGeneral)>,
            Option<common_net::msg::checkpoint::CheckpointCommitReceiptV1>,
        ),
        Error,
    > {
        use common_net::msg::checkpoint::{
            CheckpointAlignerV1, CheckpointParticipantV1, CheckpointParticipationV1, commit_checkpoint_v1,
            prepare_checkpoint_v1, validate_checkpoint_context_v1,
        };

        let fail = |what: &str| Error::Other(format!("checkpoint: {what}"));

        match frame.msg {
            ServerGeneral::CheckpointBegin(open) => {
                let context = frame.checkpoint.ok_or_else(|| fail("Begin without checkpoint context"))?;
                if rt.aligner.is_none() {
                    // The descriptor names the session it belongs to; a
                    // checkpoint for another binding is not ours to align.
                    if open.descriptor.binding != expected_binding {
                        return Err(fail("descriptor binding is not this session"));
                    }
                    let root = open
                        .descriptor
                        .descriptor_root_v1()
                        .map_err(|e| fail(&format!("descriptor root: {e:?}")))?;
                    let aligner = CheckpointAlignerV1::open_v1(open.descriptor.clone(), root)
                        .map_err(|e| fail(&format!("descriptor refused: {e:?}")))?;
                    rt.phase
                        .begin_alignment_v1(open.begin.epoch)
                        .map_err(|e| fail(&format!("phase: {e:?}")))?;
                    rt.aligner = Some(aligner);
                    rt.staged_events = 0;
                }
                let aligner = rt.aligner.as_mut().expect("opened above");
                validate_checkpoint_context_v1(
                    CheckpointParticipationV1::CheckpointControl,
                    Some(&context),
                    open.begin.epoch,
                    aligner.descriptor_root(),
                )
                .map_err(|e| fail(&format!("Begin context: {e:?}")))?;
                aligner
                    .accept_begin_v1(&open.begin)
                    .map_err(|e| fail(&format!("Begin refused: {e:?}")))?;
                rt.staged_events += 1;
                Ok((Vec::new(), None))
            },
            ServerGeneral::CheckpointBarrier(barrier) => {
                let context = frame.checkpoint.ok_or_else(|| fail("Barrier without checkpoint context"))?;
                let aligner = rt.aligner.as_mut().ok_or_else(|| fail("Barrier with no checkpoint open"))?;
                validate_checkpoint_context_v1(
                    CheckpointParticipationV1::CheckpointControl,
                    Some(&context),
                    barrier.epoch,
                    aligner.descriptor_root(),
                )
                .map_err(|e| fail(&format!("Barrier context: {e:?}")))?;
                aligner
                    .accept_barrier_v1(&barrier)
                    .map_err(|e| fail(&format!("Barrier refused: {e:?}")))?;
                rt.staged_events += 1;
                if !aligner.is_complete() {
                    return Ok((Vec::new(), None));
                }

                let descriptor = aligner.descriptor().clone();
                let descriptor_root = aligner.descriptor_root();
                let staged = aligner
                    .take_apply_sequence_v1()
                    .map_err(|e| fail(&format!("alignment incomplete: {e:?}")))?;
                let prepared = prepare_checkpoint_v1(
                    &descriptor,
                    descriptor_root,
                    staged,
                    rt.staged_events,
                    &rt.profile,
                    &rt.chronology,
                )
                .map_err(|e| fail(&format!("prepare refused: {e:?}")))?;
                rt.phase
                    .mark_prepared_v1(descriptor.epoch)
                    .map_err(|e| fail(&format!("phase: {e:?}")))?;

                let mut sink = CheckpointApplyCollectorV1::default();
                let receipt = commit_checkpoint_v1(prepared, &mut rt.chronology, &mut sink);
                rt.phase
                    .mark_committed_v1(receipt.epoch)
                    .map_err(|e| fail(&format!("phase: {e:?}")))?;
                rt.aligner = None;
                rt.staged_events = 0;

                let out = sink
                    .applied
                    .into_iter()
                    .map(|op| (op.stream, std::sync::Arc::try_unwrap(op.payload).unwrap_or_else(|arc| (*arc).clone())))
                    .collect();
                Ok((out, Some(receipt)))
            },
            msg => match frame.checkpoint {
                Some(context) => {
                    let aligner = rt.aligner.as_mut().ok_or_else(|| fail("checkpointed data with no checkpoint open"))?;
                    aligner
                        .accept_data_v1(stream, frame.sequence, &context, std::sync::Arc::new(msg))
                        .map_err(|e| fail(&format!("data refused: {e:?}")))?;
                    rt.staged_events += 1;
                    Ok((Vec::new(), None))
                },
                // Unfenced traffic: legal only outside a checkpoint. Inside
                // one it would be applied out of the aligned order, which is
                // the interleave the fence exists to prevent.
                None if rt.phase.may_apply_directly_v1() => Ok((vec![(stream, msg)], None)),
                None if msg.participation_v1() == CheckpointParticipationV1::OutOfBandDiagnostic => {
                    Ok((vec![(stream, msg)], None))
                },
                None => Err(fail("unfenced data arrived inside an open checkpoint")),
            },
        }
    }

    /// `T3.4.20c`: runs the checkpoint runtime over one drained frame,
    /// then acknowledges a commit. With the path inactive, a checkpointed
    /// frame is refused rather than half-handled.
    fn checkpoint_intercept_v1(
        &mut self,
        stream: common_net::msg::envelope::SemanticStreamIdV1,
        frame: DrainedFrameV1,
    ) -> Result<Vec<(common_net::msg::envelope::SemanticStreamIdV1, ServerGeneral)>, Error> {
        let Some(mut rt) = self.checkpoint_runtime.take() else {
            if frame.checkpoint.is_some()
                || matches!(frame.msg, ServerGeneral::CheckpointBegin(_) | ServerGeneral::CheckpointBarrier(_))
            {
                return Err(Error::Other(
                    "checkpoint: a fenced frame arrived with the checkpoint path inactive".to_owned(),
                ));
            }
            return Ok(vec![(stream, frame.msg)]);
        };
        let Some(expected_binding) = self.semantic_receive_state.as_ref().map(|state| state.binding()) else {
            self.checkpoint_runtime = Some(rt);
            return Err(Error::Other("checkpoint: no active attachment to bind a checkpoint to".to_owned()));
        };
        let stepped = Self::checkpoint_step_v1(&mut rt, expected_binding, stream, frame);
        self.checkpoint_runtime = Some(rt);
        let (out, ack) = stepped?;
        if let Some(receipt) = ack {
            self.send_msg(ClientGeneral::CheckpointCommitAck(receipt));
        }
        Ok(out)
    }

    fn dispatch_by_stream_v1(
        &mut self,
        frontend_events: &mut Vec<Event>,
        stream: common_net::msg::envelope::SemanticStreamIdV1,
        msg: ServerGeneral,
    ) -> Result<(), Error> {
        use common_net::msg::envelope::SemanticStreamIdV1 as S;
        match stream {
            S::Bootstrap | S::General => self.handle_server_msg(frontend_events, msg),
            S::CharacterScreen => self.handle_server_character_screen_msg(frontend_events, msg),
            S::InGame => self.handle_server_in_game_msg(frontend_events, msg),
            S::Terrain => self.handle_server_terrain_msg(msg),
        }
    }

    fn handle_messages(&mut self, frontend_events: &mut Vec<Event>) -> Result<u64, Error> {
        use common_net::msg::envelope::SemanticStreamIdV1;

        let mut cnt = 0;
        #[cfg(feature = "tracy")]
        let (mut terrain_cnt, mut ingame_cnt) = (0, 0);
        loop {
            let cnt_start = cnt;

            if self.semantic_receive_state.is_some() {
                for msg in Self::drain_semantic_stream_v1(
                    &mut self.general_stream,
                    &mut self.semantic_receive_state,
                    SemanticStreamIdV1::General,
                    &self.semantic_ingress_metrics,
                )? {
                    cnt += 1;
                    for (stream, out) in self.checkpoint_intercept_v1(SemanticStreamIdV1::General, msg)? {
                        self.dispatch_by_stream_v1(frontend_events, stream, out)?;
                    }
                }
            } else {
                while let Some(msg) = self.general_stream.try_recv()? {
                    cnt += 1;
                    self.handle_server_msg(frontend_events, msg)?;
                }
            }
            while let Some(msg) = self.ping_stream.try_recv()? {
                cnt += 1;
                self.handle_ping_msg(msg)?;
            }
            if self.semantic_receive_state.is_some() {
                for msg in Self::drain_semantic_stream_v1(
                    &mut self.character_screen_stream,
                    &mut self.semantic_receive_state,
                    SemanticStreamIdV1::CharacterScreen,
                    &self.semantic_ingress_metrics,
                )? {
                    cnt += 1;
                    for (stream, out) in self.checkpoint_intercept_v1(SemanticStreamIdV1::CharacterScreen, msg)? {
                        self.dispatch_by_stream_v1(frontend_events, stream, out)?;
                    }
                }
            } else {
                while let Some(msg) = self.character_screen_stream.try_recv()? {
                    cnt += 1;
                    self.handle_server_character_screen_msg(frontend_events, msg)?;
                }
            }
            if self.semantic_receive_state.is_some() {
                for msg in Self::drain_semantic_stream_v1(
                    &mut self.in_game_stream,
                    &mut self.semantic_receive_state,
                    SemanticStreamIdV1::InGame,
                    &self.semantic_ingress_metrics,
                )? {
                    cnt += 1;
                    #[cfg(feature = "tracy")]
                    {
                        ingame_cnt += 1;
                    }
                    for (stream, out) in self.checkpoint_intercept_v1(SemanticStreamIdV1::InGame, msg)? {
                        self.dispatch_by_stream_v1(frontend_events, stream, out)?;
                    }
                }
            } else {
                while let Some(msg) = self.in_game_stream.try_recv()? {
                    cnt += 1;
                    #[cfg(feature = "tracy")]
                    {
                        ingame_cnt += 1;
                    }
                    self.handle_server_in_game_msg(frontend_events, msg)?;
                }
            }
            if self.semantic_receive_state.is_some() {
                for msg in Self::drain_semantic_stream_v1(
                    &mut self.terrain_stream,
                    &mut self.semantic_receive_state,
                    SemanticStreamIdV1::Terrain,
                    &self.semantic_ingress_metrics,
                )? {
                    cnt += 1;
                    #[cfg(feature = "tracy")]
                    {
                        if let ServerGeneral::TerrainChunkUpdate { chunk, .. } = &msg.msg {
                            terrain_cnt += chunk.as_ref().map(|x| x.approx_len()).unwrap_or(0);
                        }
                    }
                    for (stream, out) in self.checkpoint_intercept_v1(SemanticStreamIdV1::Terrain, msg)? {
                        self.dispatch_by_stream_v1(frontend_events, stream, out)?;
                    }
                }
            } else {
                while let Some(msg) = self.terrain_stream.try_recv()? {
                    cnt += 1;
                    #[cfg(feature = "tracy")]
                    {
                        if let ServerGeneral::TerrainChunkUpdate { chunk, .. } = &msg {
                            terrain_cnt += chunk.as_ref().map(|x| x.approx_len()).unwrap_or(0);
                        }
                    }
                    self.handle_server_terrain_msg(msg)?;
                }
            }

            if cnt_start == cnt {
                #[cfg(feature = "tracy")]
                {
                    plot!("terrain_recvs", terrain_cnt as f64);
                    plot!("ingame_recvs", ingame_cnt as f64);
                }
                return Ok(cnt);
            }
        }
    }

    /// Handle new server messages.
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        prof_span!("handle_new_messages");
        let mut frontend_events = Vec::new();

        // Check that we have an valid connection.
        // Use the last ping time as a 1s rate limiter, we only notify the user once per
        // second
        if self.state.get_program_time() - self.last_server_ping > 1. {
            let duration_since_last_pong = self.state.get_program_time() - self.last_server_pong;

            // Dispatch a notification to the HUD warning they will be kicked in {n} seconds
            const KICK_WARNING_AFTER_REL_TO_TIMEOUT_FRACTION: f64 = 0.75;
            if duration_since_last_pong
                >= (self.client_timeout.as_secs() as f64
                    * KICK_WARNING_AFTER_REL_TO_TIMEOUT_FRACTION)
                && self.state.get_program_time() - duration_since_last_pong > 0.
            {
                frontend_events.push(Event::DisconnectionNotification(
                    (self.state.get_program_time() - duration_since_last_pong).round() as u64,
                ));
            }
        }

        let msg_count = self.handle_messages(&mut frontend_events)?;

        if msg_count == 0
            && self.state.get_program_time() - self.last_server_pong
                > self.client_timeout.as_secs() as f64
        {
            return Err(Error::ServerTimeout);
        }

        // ignore network events
        while let Some(res) = self
            .participant
            .as_mut()
            .and_then(|p| p.try_fetch_event().transpose())
        {
            let event = res?;
            trace!(?event, "received network event");
        }

        Ok(frontend_events)
    }

    pub fn entity(&self) -> EcsEntity {
        self.state
            .ecs()
            .read_resource::<PlayerEntity>()
            .0
            .expect("Client::entity should always have PlayerEntity be Some")
    }

    pub fn uid(&self) -> Option<Uid> { self.state.read_component_copied(self.entity()) }

    pub fn presence(&self) -> Option<PresenceKind> { self.presence }

    pub fn registered(&self) -> bool { self.registered }

    pub fn get_tick(&self) -> u64 { self.tick }

    pub fn get_ping_ms(&self) -> f64 { self.last_ping_delta * 1000.0 }

    pub fn get_ping_ms_rolling_avg(&self) -> f64 {
        let mut total_weight = 0.;
        let pings = self.ping_deltas.len() as f64;
        (self
            .ping_deltas
            .iter()
            .enumerate()
            .fold(0., |acc, (i, ping)| {
                let weight = i as f64 + 1. / pings;
                total_weight += weight;
                acc + (weight * ping)
            })
            / total_weight)
            * 1000.0
    }

    /// Get a reference to the client's runtime thread pool. This pool should be
    /// used for any computationally expensive operations that run outside
    /// of the main thread (i.e., threads that block on I/O operations are
    /// exempt).
    pub fn runtime(&self) -> &Arc<Runtime> { &self.runtime }

    /// Get a reference to the client's game state.
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the client's game state.
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Returns an iterator over the aliases of all the online players on the
    /// server
    pub fn players(&self) -> impl Iterator<Item = &str> {
        self.player_list()
            .values()
            .filter_map(|player_info| player_info.is_online.then_some(&*player_info.player_alias))
    }

    /// Return true if this client is a moderator on the server
    pub fn is_moderator(&self) -> bool { self.role.is_some() }

    pub fn role(&self) -> &Option<AdminRole> { &self.role }

    /// Clean client ECS state
    fn clean_state(&mut self) {
        // Clear pending trade
        self.pending_trade = None;

        let client_uid = self.uid().expect("Client doesn't have a Uid!!!");

        // Clear ecs of all entities
        self.state.ecs_mut().delete_all();
        self.state.ecs_mut().maintain();
        self.state.ecs_mut().insert(IdMaps::default());

        // Recreate client entity with Uid
        let entity_builder = self.state.ecs_mut().create_entity();
        entity_builder
            .world
            .write_resource::<IdMaps>()
            .add_entity(client_uid, entity_builder.entity);

        let entity = entity_builder.with(client_uid).build();
        self.state.ecs().write_resource::<PlayerEntity>().0 = Some(entity);
    }

    /// Change player alias to "You" if client belongs to matching player
    // TODO: move this to voxygen or i18n-helpers and properly localize there
    // or what's better, just remove completely, it won't properly work with
    // localization anyway.
    #[deprecated = "this function doesn't localize"]
    fn personalize_alias(&self, uid: Uid, alias: String) -> String {
        let client_uid = self.uid().expect("Client doesn't have a Uid!!!");
        if client_uid == uid {
            "You".to_string()
        } else {
            alias
        }
    }

    /// Get important information from client that is necessary for message
    /// localisation
    pub fn lookup_msg_context(&self, msg: &comp::ChatMsg) -> ChatTypeContext {
        let mut result = ChatTypeContext {
            you: self.uid().expect("Client doesn't have a Uid!!!"),
            player_info: HashMap::new(),
            entity_name: HashMap::new(),
        };

        let name_of_uid = |uid| {
            let ecs = self.state().ecs();
            let id_maps = ecs.read_resource::<common::uid::IdMaps>();
            id_maps.uid_entity(uid).and_then(|e| {
                ecs.read_storage::<comp::Stats>()
                    .get(e)
                    .map(|s| s.name.clone())
            })
        };

        let mut add_data_of = |uid| {
            match self.player_list.get(uid) {
                Some(player_info) => {
                    result.player_info.insert(*uid, player_info.clone());
                },
                None => {
                    result.entity_name.insert(
                        *uid,
                        name_of_uid(*uid).unwrap_or_else(|| Content::Plain("<?>".to_string())),
                    );
                },
            };
        };

        match &msg.chat_type {
            comp::ChatType::Online(uid) | comp::ChatType::Offline(uid) => add_data_of(uid),
            comp::ChatType::Kill(kill_source, victim) => {
                add_data_of(victim);

                match kill_source {
                    KillSource::Player(attacker_uid, _) => {
                        add_data_of(attacker_uid);
                    },
                    KillSource::NonPlayer(_, _)
                    | KillSource::FallDamage
                    | KillSource::Suicide
                    | KillSource::NonExistent(_)
                    | KillSource::Other => (),
                };
            },
            comp::ChatType::Tell(from, to) | comp::ChatType::NpcTell(from, to) => {
                add_data_of(from);
                add_data_of(to);
            },
            comp::ChatType::Say(uid)
            | comp::ChatType::Region(uid)
            | comp::ChatType::World(uid)
            | comp::ChatType::NpcSay(uid)
            | comp::ChatType::Group(uid, _)
            | comp::ChatType::Faction(uid, _)
            | comp::ChatType::Npc(uid) => add_data_of(uid),
            comp::ChatType::CommandError
            | comp::ChatType::CommandInfo
            | comp::ChatType::FactionMeta(_)
            | comp::ChatType::GroupMeta(_)
            | comp::ChatType::Meta => (),
        };
        result
    }

    /// Execute a single client tick:
    /// - handles messages from the server
    /// - sends physics update
    /// - requests chunks
    ///
    /// The game state is purposefully not simulated to reduce the overhead of
    /// running the client. This method is for use in testing a server with
    /// many clients connected.
    #[cfg(feature = "tick_network")]
    #[expect(clippy::needless_collect)] // False positive
    pub fn tick_network(&mut self, dt: Duration) -> Result<(), Error> {
        span!(_guard, "tick_network", "Client::tick_network");
        // Advance state time manually since we aren't calling `State::tick`
        self.state
            .ecs()
            .write_resource::<common::resources::ProgramTime>()
            .0 += dt.as_secs_f64();

        let time_scale = *self
            .state
            .ecs()
            .read_resource::<common::resources::TimeScale>();
        self.state
            .ecs()
            .write_resource::<common::resources::Time>()
            .0 += dt.as_secs_f64() * time_scale.0;

        // Handle new messages from the server.
        self.handle_new_messages()?;

        // 5) Terrain
        self.tick_terrain()?;
        let empty = Arc::new(TerrainChunk::new(
            0,
            Block::empty(),
            Block::empty(),
            common::terrain::TerrainChunkMeta::void(),
        ));
        let mut terrain = self.state.terrain_mut();
        // Replace chunks with empty chunks to save memory
        let to_clear = terrain
            .iter()
            .filter_map(|(key, chunk)| (chunk.sub_chunks_len() != 0).then(|| key))
            .collect::<Vec<_>>();
        to_clear.into_iter().for_each(|key| {
            terrain.insert(key, Arc::clone(&empty));
        });
        drop(terrain);

        // Send a ping to the server once every second
        if self.state.get_program_time() - self.last_server_ping > 1. {
            self.send_msg_err(PingMsg::Ping)?;
            self.last_server_ping = self.state.get_program_time();
        }

        // 6) Update the server about the player's physics attributes.
        if self.presence.is_some() {
            if let (Some(pos), Some(vel), Some(ori)) = (
                self.state.read_storage().get(self.entity()).cloned(),
                self.state.read_storage().get(self.entity()).cloned(),
                self.state.read_storage().get(self.entity()).cloned(),
            ) {
                self.in_game_stream.send(ClientGeneral::PlayerPhysics {
                    pos,
                    vel,
                    ori,
                    physics_generation: self.force_update_generation,
                })?;
            }
        }

        // 7) Finish the tick, pass control back to the frontend.
        self.tick += 1;

        Ok(())
    }

    /// another plugin data received, is this the last one
    pub fn plugin_received(&mut self, hash: PluginHash) -> usize {
        if !self.missing_plugins.remove(&hash) {
            tracing::warn!(?hash, "received unrequested plugin");
        }
        self.missing_plugins.len()
    }

    /// true if missing_plugins is not empty
    pub fn are_plugins_missing(&self) -> bool { !self.missing_plugins.is_empty() }

    /// extract list of locally cached plugins to load
    pub fn take_local_plugins(&mut self) -> Vec<PathBuf> { std::mem::take(&mut self.local_plugins) }
}

impl Drop for Client {
    fn drop(&mut self) {
        trace!("Dropping client");
        if self.registered {
            if let Err(e) = self.send_msg_err(ClientGeneral::Terminate) {
                warn!(
                    ?e,
                    "Error during drop of client, couldn't send disconnect package, is the \
                     connection already closed?",
                );
            }
        } else {
            trace!("no disconnect msg necessary as client wasn't registered")
        }

        tokio::task::block_in_place(|| {
            if let Err(e) = self
                .runtime
                .block_on(self.participant.take().unwrap().disconnect())
            {
                warn!(?e, "error when disconnecting, couldn't send all data");
            }
        });
        //explicitly drop the network here while the runtime is still existing
        drop(self.network.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client_i18n::LocalizationHandle;

    #[test]
    /// THIS TEST VERIFIES THE CONSTANT API.
    /// CHANGING IT WILL BREAK 3rd PARTY APPLICATIONS (please extend) which
    /// needs to be informed (or fixed)
    ///  - torvus: https://gitlab.com/veloren/torvus
    ///
    /// CONTACT @Core Developer BEFORE MERGING CHANGES TO THIS TEST
    fn constant_api_test() {
        use common::clock::Clock;
        use voxygen_i18n_helpers::localize_chat_message;

        const SPT: f64 = 1.0 / 60.0;

        let runtime = Arc::new(Runtime::new().unwrap());
        let runtime2 = Arc::clone(&runtime);
        let username = "Foo";
        let password = "Bar";
        let auth_server = "auth.veloren.net";
        let veloren_client: Result<Client, Error> = runtime.block_on(Client::new(
            ConnectionArgs::Tcp {
                hostname: "127.0.0.1:9000".to_owned(),
                prefer_ipv6: false,
            },
            runtime2,
            &mut None,
            username,
            password,
            None,
            |suggestion: &str| suggestion == auth_server,
            &|_| {},
            |_| {},
            PathBuf::default(),
            ClientType::ChatOnly,
        ));
        let localisation = LocalizationHandle::load_expect("en");

        let _ = veloren_client.map(|mut client| {
            //clock
            let mut clock = Clock::new(Duration::from_secs_f64(SPT));

            //tick
            let events_result: Result<Vec<Event>, Error> =
                client.tick(ControllerInputs::default(), clock.game_dt());

            //chat functionality
            client.send_chat("foobar".to_string());

            let _ = events_result.map(|mut events| {
                // event handling
                if let Some(event) = events.pop() {
                    match event {
                        Event::Chat(msg) => {
                            let msg: comp::ChatMsg = msg;
                            let _s: String = localize_chat_message(
                                &msg,
                                &client.lookup_msg_context(&msg),
                                &localisation.read(),
                                true,
                            )
                            .1;
                        },
                        Event::Disconnect => {},
                        Event::DisconnectionNotification(_) => {
                            debug!("Will be disconnected soon! :/")
                        },
                        Event::Notification(notification) => {
                            let notification: UserNotification = notification;
                            debug!("Notification: {:?}", notification);
                        },
                        _ => {},
                    }
                };
            });

            client.cleanup();
            clock.tick();
        });
    }

    // `T3.3.10`: `validate_semantic_frame_v1` is pure (no `Stream`
    // needed), so it gets the same direct unit-test treatment as the
    // server's own `validate_semantic_frame_v1` (T3.3.08). The stateful
    // `drain_semantic_stream_v1` wrapper is NOT independently tested
    // here, for the same reason `send_semantic_v1` (`T3.3.07`) never
    // was: both need a live `Stream` backed by a real `network::
    // Participant`, which this crate has no lightweight way to
    // construct (`constant_api_test` above is the one place that tries,
    // via a real TCP dial that's expected to fail without a listening
    // server). Its own "no mutation on reject" guarantee is structural,
    // not runtime-tested: `validate_semantic_frame_v1` takes
    // `receive_state` by `&` (immutable) reference, so no code path
    // through it, reject or accept, can touch cursor state.
    //
    // Packet's own test list ("Duplicate replication/terrain/inventory/
    // Bastion, wrong route, old GameSync, handler error"): "old
    // GameSync" is this tree's `StaleEpoch` check under a different
    // name -- `ConnectionEpoch` IS the mechanism tied to when `GameSync`
    // last ran (T3.2), so a stale epoch and a stale GameSync are the
    // same rejection, not two. "Handler error" needs the live-`Stream`
    // harness noted above and is deferred with it.
    use common_net::msg::{
        ServerGeneral,
        envelope::{
            ActiveSessionBindingV1, SemanticCausalityV1, SemanticEnvelopeRejectV1, SemanticPayloadEncodingV1,
            SemanticReceiveStateV1, SemanticRouteV1, SemanticStreamIdV1, encode_payload_v1,
        },
    };
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use std::num::NonZeroU64;
    use vek::Vec2;

    fn recv_binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([11; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([12; 16])).unwrap(),
            epoch: ConnectionEpoch::new(7).unwrap(),
        }
    }

    fn recv_state() -> SemanticReceiveStateV1 { SemanticReceiveStateV1::new(recv_binding()) }

    fn recv_frame_bytes(b: ActiveSessionBindingV1, sequence: u64, msg: &ServerGeneral) -> Vec<u8> {
        recv_frame_bytes_checkpointed(b, sequence, msg, None)
    }

    fn recv_frame_bytes_checkpointed(
        b: ActiveSessionBindingV1,
        sequence: u64,
        msg: &ServerGeneral,
        checkpoint: Option<common_net::msg::checkpoint::CheckpointedEnvelopeContextV1>,
    ) -> Vec<u8> {
        let payload_bytes = encode_payload_v1(msg);
        let profile_root = common_net::msg::envelope::net_envelope_profile_root_v1();
        let payload_schema = msg.payload_schema();
        let payload_encoding = SemanticPayloadEncodingV1::Bincode2LegacySerde;
        let payload_digest = common_net::msg::envelope::payload_digest_v1(
            profile_root,
            payload_schema,
            payload_encoding,
            &payload_bytes,
        );
        let header = common_net::msg::envelope::NetEnvelopeHeaderV1 {
            profile_root,
            server_boot_id: b.server_boot_id,
            session_id: b.session_id,
            connection_epoch: b.epoch,
            direction: common_net::msg::envelope::SemanticDirectionV1::ServerToClient,
            semantic_stream: msg.semantic_stream(),
            sequence: NonZeroU64::new(sequence).unwrap(),
            causality: SemanticCausalityV1 { producer_tick: None, snapshot: None },
            payload_schema,
            payload_encoding,
            payload_len: payload_bytes.len() as u64,
            payload_digest,
            command_id: None,
            checkpoint,
        };
        let frame = common_net::msg::envelope::SemanticWireFrameV1 { header, payload_bytes };
        let limits = common::apex::manifest::ManifestDecodeLimitsV1 {
            max_input_bytes: 1 << 20,
            max_depth: 8,
            max_nodes: 64,
            max_array_items: 16,
            max_map_entries: 16,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 1 << 20,
        };
        common::apex::manifest::encode_manifest_v1(&frame, &limits).unwrap()
    }

    /// One easy-to-construct representative `ServerGeneral` per stream,
    /// matching the packet's own named example classes: General stands
    /// in for "replication" traffic, InGame for "inventory/Bastion"
    /// (both route InGame per `envelope.rs`'s own classification,
    /// T3.3.04), Terrain and CharacterScreen for themselves.
    fn representative_messages() -> [ServerGeneral; 4] {
        [
            ServerGeneral::UpdateRecipes,
            ServerGeneral::CharacterSuccess,
            ServerGeneral::ExitInGameSuccess,
            ServerGeneral::TerrainChunkUpdate { key: Vec2::new(0, 0), chunk: Err(()) },
        ]
    }

    #[test]
    fn receive_semantic_v1_valid_frame_is_accepted_for_every_stream() {
        let state = recv_state();
        for msg in representative_messages() {
            let raw = recv_frame_bytes(recv_binding(), 1, &msg);
            let (decoded, _causality, _checkpoint) = Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()).unwrap();
            assert_eq!(decoded.semantic_stream(), msg.semantic_stream());
        }
    }

    #[test]
    fn receive_semantic_v1_duplicate_sequence_is_rejected_for_every_stream() {
        // "Duplicate replication/terrain/inventory/Bastion": duplicate-
        // sequence rejection is stream-agnostic in this tree's
        // validation pipeline, proven here across all four streams'
        // representative message kinds rather than assumed from one.
        for msg in representative_messages() {
            let mut state = recv_state();
            state.advance_expected(msg.semantic_stream()).unwrap(); // next_expected is now 2
            let raw = recv_frame_bytes(recv_binding(), 1, &msg); // stale: already-consumed value
            assert_eq!(
                Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()).unwrap_err(),
                SemanticEnvelopeRejectV1::DuplicateSequence
            );
        }
    }

    #[test]
    fn receive_semantic_v1_sequence_gap_is_rejected_with_exact_values() {
        let state = recv_state();
        let msg = ServerGeneral::UpdateRecipes;
        let raw = recv_frame_bytes(recv_binding(), 5, &msg); // expected 1, received 5
        assert_eq!(
            Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()).unwrap_err(),
            SemanticEnvelopeRejectV1::SequenceGap { expected: 1, received: 5 }
        );
    }

    #[test]
    fn receive_semantic_v1_wrong_route_is_rejected() {
        let state = recv_state();
        let msg = ServerGeneral::UpdateRecipes; // declares InGame
        let raw = recv_frame_bytes(recv_binding(), 1, &msg);
        assert_eq!(
            Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, SemanticStreamIdV1::Terrain).unwrap_err(),
            SemanticEnvelopeRejectV1::StreamRouteMismatch
        );
    }

    #[test]
    fn receive_semantic_v1_old_game_sync_ie_stale_epoch_is_rejected() {
        let state = recv_state(); // epoch 7
        let mut stale = recv_binding();
        stale.epoch = ConnectionEpoch::new(6).unwrap(); // frame from before the last GameSync-triggered epoch bump
        let msg = ServerGeneral::UpdateRecipes;
        let raw = recv_frame_bytes(stale, 1, &msg);
        assert_eq!(
            Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()).unwrap_err(),
            SemanticEnvelopeRejectV1::StaleEpoch
        );
    }

    #[test]
    fn receive_semantic_v1_wrong_direction_is_rejected() {
        let state = recv_state();
        let msg = ServerGeneral::UpdateRecipes;
        let payload_bytes = encode_payload_v1(&msg);
        let profile_root = common_net::msg::envelope::net_envelope_profile_root_v1();
        let payload_schema = msg.payload_schema();
        let payload_encoding = SemanticPayloadEncodingV1::Bincode2LegacySerde;
        let payload_digest = common_net::msg::envelope::payload_digest_v1(
            profile_root,
            payload_schema,
            payload_encoding,
            &payload_bytes,
        );
        let b = recv_binding();
        let header = common_net::msg::envelope::NetEnvelopeHeaderV1 {
            profile_root,
            server_boot_id: b.server_boot_id,
            session_id: b.session_id,
            connection_epoch: b.epoch,
            direction: common_net::msg::envelope::SemanticDirectionV1::ClientToServer, // wrong: client is RECEIVING
            semantic_stream: msg.semantic_stream(),
            sequence: NonZeroU64::new(1).unwrap(),
            causality: SemanticCausalityV1 { producer_tick: None, snapshot: None },
            payload_schema,
            payload_encoding,
            payload_len: payload_bytes.len() as u64,
            payload_digest,
            command_id: None,
            checkpoint: None,
        };
        let frame = common_net::msg::envelope::SemanticWireFrameV1 { header, payload_bytes };
        let limits = common::apex::manifest::ManifestDecodeLimitsV1 {
            max_input_bytes: 1 << 20,
            max_depth: 8,
            max_nodes: 64,
            max_array_items: 16,
            max_map_entries: 16,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 1 << 20,
        };
        let raw = common::apex::manifest::encode_manifest_v1(&frame, &limits).unwrap();
        assert_eq!(
            Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()).unwrap_err(),
            SemanticEnvelopeRejectV1::WrongDirection
        );
    }

    /// `T3.3.16`'s own "raw V1" test case: a V1-negotiated attachment
    /// must never silently accept a raw (non-enveloped) frame as if it
    /// were a valid semantic payload -- "mode mixing terminates". Raw
    /// bincode-legacy bytes (exactly what a `Legacy` session would have
    /// sent for the same payload instead) are not valid T0.2 manifest
    /// bytes at all, so decode fails outright -- a pre-existing gap in
    /// this generic function's own coverage (every other test here
    /// exercises a validly-ENCODED-but-otherwise-wrong frame), closed
    /// here since this row's packet names it explicitly.
    #[test]
    fn receive_semantic_v1_raw_legacy_bytes_are_rejected_not_silently_accepted() {
        let state = recv_state();
        let raw_legacy_bytes = encode_payload_v1(&ServerGeneral::UpdateRecipes);
        assert_eq!(
            Client::validate_semantic_frame_v1::<ServerGeneral>(&raw_legacy_bytes, &state, SemanticStreamIdV1::General).unwrap_err(),
            SemanticEnvelopeRejectV1::EnvelopeDecodeFailure
        );
    }

    /// `receive_semantic_no_mutation`: proves the structural guarantee
    /// noted at the top of this section -- a rejected frame cannot have
    /// advanced the cursor, because `validate_semantic_frame_v1` only
    /// ever receives `receive_state` by immutable reference.
    #[test]
    fn receive_semantic_no_mutation_on_reject_leaves_cursor_unchanged() {
        let state = recv_state();
        let before = state.next_expected_for(SemanticStreamIdV1::General);
        let msg = ServerGeneral::UpdateRecipes;
        let raw = recv_frame_bytes(recv_binding(), 99, &msg); // a gap, guaranteed reject
        assert!(Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()).is_err());
        assert_eq!(state.next_expected_for(SemanticStreamIdV1::General), before);
    }

    /// `APEX-T3.3.18` "redacted metrics" test (packet names `cargo test
    /// -p veloren-client semantic_metrics_redaction`; same aspirational-
    /// vs-actual module-path gap noted server-side -- this crate's
    /// tests live in a flat `tests` module, not a `semantic_metrics`
    /// one). Client-side twin of the server's `rejected_traffic_
    /// liveness_and_redacted_metrics`: a real reject feeds
    /// `SemanticIngressMetricsV1` with nothing but a code and a stream.
    #[test]
    fn client_rejected_traffic_liveness_and_redacted_metrics() {
        let state = recv_state();
        let metrics = common_net::msg::envelope::SemanticIngressMetricsV1::new();
        let msg = ServerGeneral::UpdateRecipes; // routes InGame
        let raw = recv_frame_bytes(recv_binding(), 99, &msg); // sequence gap
        let Err(reject) = Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()) else {
            panic!("fixture must reject (sequence gap)");
        };
        metrics.record_reject(&reject, msg.semantic_stream());

        assert_eq!(state.next_expected_for(msg.semantic_stream()).get(), 1, "rejected traffic must not move the cursor");
        assert_eq!(metrics.snapshot(), vec![("sequence_gap", SemanticStreamIdV1::InGame, 1)]);
    }

    /// `APEX-T3.4.20c`: the live receive path. These drive the real
    /// `Client` code -- `validate_semantic_frame_v1` under the exact
    /// decode limits production uses, and `checkpoint_step_v1` over a
    /// whole checkpoint -- not a harness reimplementation of them.
    mod checkpoint_receive_v1 {
        use super::*;
        use common_net::msg::checkpoint::{
            CheckpointBarrierV1, CheckpointBeginV1, CheckpointChronologyV1,
            CheckpointDescriptorV1, CheckpointOrdinalV1, CheckpointProfilePurposeV1, CheckpointResourceProfileV1,
            CheckpointStreamOpenV1, CheckpointedEnvelopeContextV1, ClientCheckpointStateV1,
            REQUIRED_CHECKPOINT_STREAMS_V1, StreamCheckpointPlanV1, TranscriptEntryV1,
            global_transcript_root_v1, stream_transcript_root_v1,
        };

        const EPOCH: u64 = 3;

        fn records() -> Vec<(SemanticStreamIdV1, u64, ServerGeneral)> {
            vec![
                (SemanticStreamIdV1::InGame, 1, ServerGeneral::CharacterSuccess),
                (SemanticStreamIdV1::InGame, 2, ServerGeneral::UpdateRecipes),
                (SemanticStreamIdV1::Terrain, 3, ServerGeneral::ExitInGameSuccess),
            ]
        }

        fn entry_of(sequence: u64, ordinal: u64, msg: &ServerGeneral) -> (TranscriptEntryV1, u64) {
            let bytes = encode_payload_v1(msg);
            let digest = common_net::msg::envelope::payload_digest_v1(
                common_net::msg::envelope::net_envelope_profile_root_v1(),
                msg.payload_schema(),
                SemanticPayloadEncodingV1::Bincode2LegacySerde,
                &bytes,
            );
            (
                TranscriptEntryV1 {
                    sequence,
                    ordinal: CheckpointOrdinalV1(ordinal),
                    payload_kind: msg.payload_schema().as_u16(),
                    payload_digest: *digest.as_array(),
                },
                bytes.len() as u64,
            )
        }

        /// The descriptor the server would have planned for `records()`,
        /// with every stream fenced from its own sequence 1.
        fn descriptor() -> CheckpointDescriptorV1 {
            let binding = recv_binding();
            let mut plans = Vec::with_capacity(5);
            let mut all = Vec::new();
            for stream in REQUIRED_CHECKPOINT_STREAMS_V1 {
                let mut entries = Vec::new();
                let mut bytes = 0;
                let mut sequence = 1;
                for (_, ordinal, msg) in records().iter().filter(|(s, _, _)| *s == stream) {
                    sequence += 1;
                    let (entry, len) = entry_of(sequence, *ordinal, msg);
                    bytes += len;
                    entries.push(entry);
                }
                let n = entries.len() as u32;
                plans.push(StreamCheckpointPlanV1 {
                    stream,
                    begin_sequence: 1,
                    first_data_sequence: (n > 0).then_some(2),
                    last_data_sequence: (n > 0).then_some(1 + n as u64),
                    barrier_sequence: 2 + n as u64,
                    data_record_count: n,
                    payload_bytes: bytes,
                    stream_transcript_root: stream_transcript_root_v1(&binding, EPOCH, stream, &entries).unwrap(),
                });
                all.extend(entries);
            }
            CheckpointDescriptorV1 {
                schema_version: 1,
                binding,
                epoch: EPOCH,
                parent_epoch: EPOCH - 1,
                resource_profile_root: [1; 32],
                apply_policy_root: [2; 32],
                egress_order_policy_root: [3; 32],
                data_record_count: records().len() as u32,
                ordinal_max: records().len() as u64,
                payload_bytes: plans.iter().map(|p| p.payload_bytes).sum(),
                global_transcript_root: global_transcript_root_v1(&binding, EPOCH, &all).unwrap(),
                streams: plans.try_into().unwrap(),
                bootstrap_manifest_root: None,
            }
        }

        fn runtime() -> ClientCheckpointRuntimeV1 {
            let mut chronology = CheckpointChronologyV1::new();
            chronology.commit_epoch_v1(EPOCH - 1);
            ClientCheckpointRuntimeV1 {
                profile: CheckpointResourceProfileV1 {
                    profile_id: "apex-t3-4-client-test-v1".to_owned(),
                    purpose: CheckpointProfilePurposeV1::TestFixture,
                    max_records_per_checkpoint: 8,
                    max_payload_bytes_per_checkpoint: 1 << 16,
                    max_payload_bytes_per_stream: [1 << 16; 5],
                    max_staged_events: 32,
                    max_prepared_ops: 8,
                },
                chronology,
                phase: ClientCheckpointStateV1::new(64),
                aligner: None,
                staged_events: 0,
            }
        }

        fn ctx(root: [u8; 32], ordinal: Option<u64>) -> CheckpointedEnvelopeContextV1 {
            CheckpointedEnvelopeContextV1 {
                epoch: EPOCH,
                ordinal: ordinal.map(CheckpointOrdinalV1),
                descriptor_root: root,
            }
        }

        fn frame(msg: ServerGeneral, checkpoint: Option<CheckpointedEnvelopeContextV1>, sequence: u64) -> DrainedFrameV1 {
            DrainedFrameV1 { msg, checkpoint, sequence }
        }

        /// A fenced frame must survive the EXACT decode limits the live
        /// receive path uses -- the header grew by a nested map, and a
        /// limit that rejected it would make checkpoints undeliverable.
        #[test]
        fn a_fenced_frame_survives_the_live_decode_limits() {
            let msg = ServerGeneral::UpdateRecipes;
            let context = ctx([0xC7; 32], Some(4));
            let raw = recv_frame_bytes_checkpointed(recv_binding(), 1, &msg, Some(context));
            let state = recv_state();
            let (decoded, _causality, carried) =
                Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()).unwrap();
            assert_eq!(format!("{decoded:?}"), format!("{msg:?}"));
            assert_eq!(carried, Some(context), "the checkpoint binding must survive the wire");

            // ...and an unfenced frame still decodes to no context
            let raw = recv_frame_bytes(recv_binding(), 1, &msg);
            let (_, _, none) =
                Client::validate_semantic_frame_v1::<ServerGeneral>(&raw, &state, msg.semantic_stream()).unwrap();
            assert_eq!(none, None);
        }

        /// `CKPT-020`: a descriptor names the session it belongs to, and
        /// a checkpoint planned for another binding is not ours to align.
        #[test]
        fn a_descriptor_for_another_session_is_refused() {
            let descriptor = descriptor();
            let root = descriptor.descriptor_root_v1().unwrap();
            let mut rt = runtime();
            let other = ActiveSessionBindingV1 {
                server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([90; 16])).unwrap(),
                session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([91; 16])).unwrap(),
                epoch: ConnectionEpoch::new(7).unwrap(),
            };
            let open = CheckpointStreamOpenV1 {
                begin: CheckpointBeginV1 { epoch: EPOCH, stream: SemanticStreamIdV1::General, descriptor_root: root },
                descriptor,
            };
            assert!(
                Client::checkpoint_step_v1(
                    &mut rt,
                    other,
                    SemanticStreamIdV1::General,
                    frame(ServerGeneral::CheckpointBegin(Box::new(open)), Some(ctx(root, None)), 1),
                )
                .is_err()
            );
            assert!(rt.aligner.is_none(), "a refused descriptor must not open an aligner");
        }

        /// The whole point: the client applies NOTHING until the last
        /// stream is fenced, then applies the set in canonical order.
        #[test]
        fn nothing_is_applied_until_the_last_barrier() {
            let descriptor = descriptor();
            let root = descriptor.descriptor_root_v1().unwrap();
            let mut rt = runtime();

            for stream in REQUIRED_CHECKPOINT_STREAMS_V1 {
                let open = CheckpointStreamOpenV1 {
                    begin: CheckpointBeginV1 { epoch: EPOCH, stream, descriptor_root: root },
                    descriptor: descriptor.clone(),
                };
                let (out, ack) = Client::checkpoint_step_v1(
                    &mut rt,
                    recv_binding(),
                    stream,
                    frame(ServerGeneral::CheckpointBegin(Box::new(open)), Some(ctx(root, None)), 1),
                )
                .unwrap();
                assert!(out.is_empty() && ack.is_none());
            }

            for stream in REQUIRED_CHECKPOINT_STREAMS_V1 {
                let mut sequence = 1;
                for (_, ordinal, msg) in records().into_iter().filter(|(s, _, _)| *s == stream) {
                    sequence += 1;
                    let (out, ack) = Client::checkpoint_step_v1(
                        &mut rt,
                        recv_binding(),
                        stream,
                        frame(msg, Some(ctx(root, Some(ordinal))), sequence),
                    )
                    .unwrap();
                    assert!(out.is_empty() && ack.is_none(), "staged, never applied");
                }
            }

            // unfenced traffic inside an open checkpoint is refused, not
            // quietly applied out of order
            assert!(
                Client::checkpoint_step_v1(
                    &mut rt,
                    recv_binding(),
                    SemanticStreamIdV1::General,
                    frame(ServerGeneral::UpdateRecipes, None, 99)
                )
                .is_err()
            );

            let mut applied = Vec::new();
            let mut receipt = None;
            for plan in descriptor.streams.iter() {
                let barrier = CheckpointBarrierV1 {
                    epoch: EPOCH,
                    stream: plan.stream,
                    descriptor_root: root,
                    data_record_count: plan.data_record_count,
                    payload_bytes: plan.payload_bytes,
                    last_data_sequence: plan.last_data_sequence,
                    stream_transcript_root: plan.stream_transcript_root,
                };
                let (out, ack) = Client::checkpoint_step_v1(
                    &mut rt,
                    recv_binding(),
                    plan.stream,
                    frame(ServerGeneral::CheckpointBarrier(barrier), Some(ctx(root, None)), plan.barrier_sequence),
                )
                .unwrap();
                if out.is_empty() {
                    assert!(ack.is_none(), "no receipt before the checkpoint is whole");
                } else {
                    applied = out;
                    receipt = ack;
                }
            }

            // phase-major, ordinal-minor, each record on its own stream
            let shape: Vec<(SemanticStreamIdV1, String)> =
                applied.iter().map(|(s, m)| (*s, format!("{m:?}"))).collect();
            assert_eq!(
                shape,
                vec![
                    (SemanticStreamIdV1::InGame, format!("{:?}", ServerGeneral::CharacterSuccess)),
                    (SemanticStreamIdV1::InGame, format!("{:?}", ServerGeneral::UpdateRecipes)),
                    (SemanticStreamIdV1::Terrain, format!("{:?}", ServerGeneral::ExitInGameSuccess)),
                ]
            );
            let receipt = receipt.expect("a committed checkpoint is acknowledged");
            assert_eq!(receipt.epoch, EPOCH);
            assert_eq!(receipt.descriptor_root, root);
            assert_eq!(receipt.applied_records, 3);
            assert_eq!(rt.chronology.committed_epoch(), EPOCH);
            // ...and the receiver is Idle again, so ordinary traffic flows
            let (out, _) = Client::checkpoint_step_v1(
                &mut rt,
                recv_binding(),
                SemanticStreamIdV1::General,
                frame(ServerGeneral::UpdateRecipes, None, 100),
            )
            .unwrap();
            assert_eq!(out.len(), 1);
        }
    }

}

#[cfg(test)]
mod weather_prediction_split_v1 {
    use super::*;

    fn lerp_with(snapshot: u64, wind: Vec2<f32>) -> WeatherLerp {
        let mut lerp = WeatherLerp::default();
        // Two arrivals, so the presentation lerp has a real interval to
        // interpolate over rather than dividing by zero. The two calls
        // must be separated by a REAL sleep, not just consecutive
        // statements: back-to-back Instant::now() calls can register a
        // near-zero (or exactly zero) calibration duration, which makes
        // `update_local_wind`'s ratio saturate its `.clamp(0.0, 1.0)` to
        // 1.0 on the very first call -- the presentation value would
        // already sit at the endpoint before any test-body timing even
        // runs, silently defeating the "moves after elapsed time"
        // assertion below regardless of what it measures afterward.
        lerp.local_wind_update(Vec2::zero(), WeatherSnapshotIdV1::from_sequence_v1(snapshot - 1));
        std::thread::sleep(std::time::Duration::from_millis(5));
        lerp.local_wind_update(wind, WeatherSnapshotIdV1::from_sequence_v1(snapshot));
        lerp
    }

    /// **`T5.4`'s acceptance criterion, on the live type.** Receipt
    /// timing varies; the prediction input does not.
    ///
    /// The two `update_local_wind` calls are separated by real elapsed
    /// time, which is what moves the presentation lerp. The test asserts
    /// the presentation value ACTUALLY moved, so it cannot pass by the
    /// delay having had no effect — and asserts the prediction wind did
    /// not.
    #[test]
    fn receipt_timing_moves_presentation_wind_and_never_prediction_wind() {
        let mut lerp = lerp_with(7, Vec2::new(3.0, -1.0));

        let prediction_before = lerp.prediction_wind().wind_v1();
        lerp.update_local_wind();
        let presentation_before = lerp.local_wind;

        std::thread::sleep(std::time::Duration::from_millis(40));

        lerp.update_local_wind();
        let presentation_after = lerp.local_wind;
        let prediction_after = lerp.prediction_wind().wind_v1();

        assert_ne!(
            presentation_before, presentation_after,
            "the elapsed time had no effect on presentation wind, so this test would pass              against a broken split"
        );
        assert_eq!(
            prediction_before, prediction_after,
            "receipt timing reached the prediction input"
        );
        assert_eq!(prediction_after, Some(Vec2::new(3.0, -1.0)), "prediction wind is the snapshot's own value");
    }

    /// The glider reads the WeatherGrid, so the grid is what must carry
    /// authoritative wind. This is the reroute itself, asserted.
    #[test]
    fn the_simulation_grid_receives_snapshot_wind_not_the_lerp() {
        let mut lerp = lerp_with(4, Vec2::new(9.0, 0.0));
        lerp.weather_update(
            SharedWeatherGrid::new(Vec2::new(2, 2)),
            WeatherSnapshotIdV1::from_sequence_v1(4),
        );
        lerp.old = lerp.new.clone();

        let mut grid = WeatherGrid::new(Vec2::new(2, 2));
        lerp.update(&mut grid);

        for (_, cell) in grid.iter() {
            assert_eq!(
                cell.wind,
                Vec2::new(9.0, 0.0),
                "the grid carries the receipt-time lerp instead of the snapshot's wind"
            );
        }
        // NOT asserting that presentation differs here: once the lerp
        // reaches t=1.0 the two legitimately coincide. Asserting a
        // coincidence would make this test fail for a correct reason.
        // The property under test is that the GRID takes the snapshot
        // value, and that is asserted above; the timing independence is
        // the previous test's job.
    }

    /// With no snapshot retained the previous grid value stands — a snap.
    /// Never an extrapolation, whose input would be elapsed wall-clock
    /// time and would put the dependency straight back.
    #[test]
    fn a_missing_snapshot_snaps_rather_than_extrapolating() {
        let lerp = WeatherLerp::default();
        assert_eq!(lerp.prediction_wind(), PredictionWindSourceV1::Unavailable);
        assert_eq!(lerp.prediction_wind().wind_v1(), None);
    }
}
