#![deny(unsafe_code)]
#![expect(
    clippy::option_map_unit_fn,
    clippy::needless_pass_by_ref_mut // until we find a better way for specs
)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(box_patterns, option_zip, const_type_name, slice_partition_dedup)]

pub mod automod;
// bastion (B-ASSET1): the --asset-arena test chamber (env-gated). Stays in
// this crate (unlike its 11 siblings below): it is an `impl Server` shim.
#[cfg(feature = "worldgen")]
pub mod bastion_arena;
// bastion: the other bastion_* modules live in the `bastion-server` leaf crate
// (crate-split — see readme/CRATE-SPLIT-BASTION-SERVER-PACKET.md); re-exported
// at their old paths so every crate::bastion_*/server::bastion_* reference
// compiles unchanged.
#[cfg(feature = "worldgen")]
pub use bastion_server::bastion_assets;
pub use bastion_server::{
    bastion_actions, bastion_chop, bastion_flat_arena, bastion_flight_recorder, bastion_jobs,
    bastion_mood, bastion_path, bastion_piles, bastion_traversal, bastion_traversal_tooling,
};
mod character_creator;
pub mod chat;
pub mod chunk_generator;
mod chunk_serialize;
pub mod client;
pub mod cmd;
pub mod connection_handler;
mod data_dir;
pub mod error;
pub mod events;
pub mod input;
pub mod location;
pub mod lod;
pub mod login_provider;
pub mod metrics;
pub mod persistence;
mod pet;
pub mod presence;
pub mod rtsim;
pub mod settings;
pub mod state_ext;
pub mod sys;
#[cfg(feature = "persistent_world")]
pub mod terrain_persistence;

#[cfg(feature = "worldgen")] mod weather;

pub mod wiring;

// Reexports
pub use crate::{
    data_dir::DEFAULT_DATA_DIR_NAME,
    error::Error,
    events::Event,
    input::Input,
    settings::{CalendarMode, EditableSettings, Settings},
};

#[cfg(feature = "persistent_world")]
use crate::terrain_persistence::TerrainPersistence;
use crate::{
    automod::AutoMod,
    chunk_generator::ChunkGenerator,
    client::Client,
    cmd::ChatCommandExt,
    connection_handler::ConnectionHandler,
    data_dir::DataDir,
    location::Locations,
    login_provider::LoginProvider,
    persistence::PersistedComponents,
    presence::{RegionSubscription, RepositionToFreeSpace},
    state_ext::StateExt,
    sys::sentinel::DeletedEntities,
};
use authc::Uuid;
use censor::Censor;
#[cfg(not(feature = "worldgen"))]
use common::grid::Grid;
#[cfg(feature = "worldgen")]
use common::terrain::CoordinateConversions;
#[cfg(feature = "worldgen")]
use common::terrain::TerrainChunkSize;
use common::{
    assets::AssetExt,
    calendar::Calendar,
    character::{CharacterId, CharacterItem},
    cmd::ServerChatCommand,
    comp::{self, ChatType, Content},
    event::{
        ClientDisconnectEvent, ClientDisconnectWithoutPersistenceEvent, EventBus, ExitIngameEvent,
        UpdateCharacterDataEvent,
    },
    link::Is,
    mounting::{Volume, VolumeRider},
    region::RegionMap,
    resources::{BattleMode, GameMode, Time, TimeOfDay},
    rtsim::RtSimEntity,
    shared_server_config::ServerConstants,
    slowjob::SlowJobPool,
    terrain::TerrainChunk,
    uid::Uid,
    vol::RectRasterableVol,
};
use common_base::prof_span;
use common_ecs::run_now;
use common_net::{
    msg::{ClientType, DisconnectReason, PlayerListUpdate, ServerGeneral, ServerInfo, ServerMsg},
    sync::WorldSyncExt,
};
use common_state::{AreasContainer, BlockDiff, BuildArea, ExecutionMode, State};
use common_systems::add_local_systems;
use metrics::{EcsSystemMetrics, GameplayMetrics, PhysicsMetrics, TickMetrics};
use network::{ListenAddr, Network, Pid};
use persistence::{
    character_loader::{CharacterLoader, CharacterUpdaterMessage},
    character_updater::CharacterUpdater,
};
use prometheus::Registry;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use settings::banlist::NormalizedIpAddr;
use specs::{
    Builder, Entity as EcsEntity, Entity, Join, LendJoin, WorldExt, shred::SendDispatcher,
};
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[cfg(not(feature = "worldgen"))]
use bastion_server::test_world::{IndexOwned, World};
use tokio::runtime::Runtime;
use tracing::{debug, error, info, trace, warn};
use vek::*;
use veloren_query_server::server::QueryServer;
pub use world::{WorldGenerateStage, civ::WorldCivStage, sim::WorldSimStage};

use crate::{
    persistence::{DatabaseSettings, SqlLogMode},
    sys::terrain,
};
use hashbrown::HashMap;
use std::sync::RwLock;

use crate::settings::Protocol;

#[cfg(feature = "plugins")]
use {
    common::uid::IdMaps,
    common_state::plugin::{PluginMgr, memory_manager::EcsWorld},
};

use crate::{chat::ChatCache, persistence::character_loader::CharacterScreenResponseKind};
use common::comp::Anchor;
#[cfg(feature = "worldgen")]
pub use world::{
    IndexOwned, World,
    sim::{DEFAULT_WORLD_MAP, DEFAULT_WORLD_SEED, FileOpts, GenOpts, WorldOpts},
};

/// Number of seconds a player must wait before they can change their battle
/// mode after each change.
///
/// TODO: Discuss time
const BATTLE_MODE_COOLDOWN: f64 = 60.0 * 5.0;

/// SpawnPoint corresponds to the default location that players are positioned
/// at if they have no waypoint. Players *should* always have a waypoint, so
/// this should basically never be used in practice.
#[derive(Copy, Clone)]
pub struct SpawnPoint(pub Vec3<f32>);

impl Default for SpawnPoint {
    fn default() -> Self { Self(Vec3::new(0.0, 0.0, 256.0)) }
}

// This is the minimum chunk range that is kept loaded around each player
// server-side. This is independent of the client's view distance and exists to
// avoid exploits such as small view distance chunk reloading and also to keep
// various mechanics working fluidly (i.e: not unloading nearby entities).
pub const MIN_VD: u32 = 6;

// Tick count used for throttling network updates — moved to the
// `bastion-server` leaf in the crate-split (its systems read it too);
// re-exported here so `crate::Tick` stays valid everywhere.
pub use bastion_server::Tick;

#[derive(Clone)]
pub struct HwStats {
    hardware_threads: u32,
    rayon_threads: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum DisconnectType {
    WithPersistence,
    WithoutPersistence,
}

// Start of Tick, used for metrics
#[derive(Copy, Clone)]
pub struct TickStart(Instant);

/// Store of BattleMode cooldowns for players while they go offline
#[derive(Clone, Default, Debug)]
pub struct BattleModeBuffer {
    map: HashMap<CharacterId, (BattleMode, Time)>,
}

impl BattleModeBuffer {
    pub fn push(&mut self, char_id: CharacterId, save: (BattleMode, Time)) {
        self.map.insert(char_id, save);
    }

    pub fn get(&self, char_id: &CharacterId) -> Option<&(BattleMode, Time)> {
        self.map.get(char_id)
    }

    pub fn pop(&mut self, char_id: &CharacterId) -> Option<(BattleMode, Time)> {
        self.map.remove(char_id)
    }
}

/// Keeps the IPs of recently logged off clients in memory, only used
/// for IP bans if the target is no longer online.
pub struct RecentClientIPs {
    pub last_addrs: schnellru::LruMap<Uuid, NormalizedIpAddr>,
}

impl Default for RecentClientIPs {
    fn default() -> Self {
        Self {
            last_addrs: schnellru::LruMap::new(schnellru::ByLength::new(1000)),
        }
    }
}

pub struct ChunkRequest {
    entity: EcsEntity,
    key: Vec2<i32>,
}

#[derive(Debug)]
pub enum ServerInitStage {
    DbMigrations,
    DbVacuum,
    WorldGen(WorldGenerateStage),
    StartingSystems,
}

pub struct Server {
    state: State,
    world: Arc<World>,
    index: IndexOwned,

    connection_handler: ConnectionHandler,

    runtime: Arc<Runtime>,

    metrics_registry: Arc<Registry>,
    chat_cache: ChatCache,
    database_settings: Arc<RwLock<DatabaseSettings>>,
    disconnect_all_clients_requested: bool,

    event_dispatcher: SendDispatcher<'static>,
    execution_mode: ExecutionMode,
}

impl Server {
    /// Create a new `Server`
    pub fn new(
        settings: Settings,
        editable_settings: EditableSettings,
        database_settings: DatabaseSettings,
        data_dir: &std::path::Path,
        report_stage: &(dyn Fn(ServerInitStage) + Send + Sync),
        runtime: Arc<Runtime>,
    ) -> Result<Self, Error> {
        prof_span!("Server::new");
        info!("Server data dir is: {}", data_dir.display());
        if settings.auth_server_address.is_none() {
            info!("Authentication is disabled");
        }

        report_stage(ServerInitStage::DbMigrations);
        // Run pending DB migrations (if any)
        debug!("Running DB migrations...");
        persistence::run_migrations(&database_settings);

        report_stage(ServerInitStage::DbVacuum);
        // Vacuum database
        debug!("Vacuuming database...");
        persistence::vacuum_database(&database_settings);

        let database_settings = Arc::new(RwLock::new(database_settings));

        let registry = Arc::new(Registry::new());
        let chunk_gen_metrics = metrics::ChunkGenMetrics::new(&registry).unwrap();
        let job_metrics = metrics::JobMetrics::new(&registry).unwrap();
        let network_request_metrics = metrics::NetworkRequestMetrics::new(&registry).unwrap();
        let player_metrics = metrics::PlayerMetrics::new(&registry).unwrap();
        let ecs_system_metrics = EcsSystemMetrics::new(&registry).unwrap();
        let tick_metrics = TickMetrics::new(&registry).unwrap();
        let physics_metrics = PhysicsMetrics::new(&registry).unwrap();
        let server_event_metrics = metrics::ServerEventMetrics::new(&registry).unwrap();
        let gameplay_metrics = GameplayMetrics::new(&registry).unwrap();
        let query_server_metrics = metrics::QueryServerMetrics::new(&registry).unwrap();

        let battlemode_buffer = BattleModeBuffer::default();

        // bastion (det-capture): opt-in deterministic rtsim RNG + serial
        // execution for a LIVE determinism capture (server-cli / voxygen),
        // gated on BASTION_DETERMINISTIC. The harness enables this directly;
        // live otherwise boots PARALLEL with OS-entropy rtsim RNG (tick_rng
        // falls back to `rand::rng()` when the flag is off) — which is exactly
        // why two live runs of the same world diverged (different colonists,
        // different wander). Must precede the execution_mode read + worldgen.
        if std::env::var_os("BASTION_DETERMINISTIC").is_some() {
            ::rtsim::enable_deterministic_rtsim();
            common::enable_deterministic_worldgen();
        }

        // DETRNG/ARCH-003: the harness sets the rtsim flag before Server::new.
        // Read it once at construction so execution policy cannot change
        // halfway through a run. Live never sets it and remains parallel.
        let execution_mode = rtsim::execution_mode();
        let pools = State::pools_with_mode(GameMode::Server, execution_mode);
        if execution_mode.is_deterministic() {
            let rayon_threads = pools.current_num_threads();
            // T0.52: the serial-vs-parallel equivalence PROBE deliberately
            // runs deterministic seeds on a multi-worker pool — the old
            // invariant (deterministic ⇒ one worker) is exactly what the
            // probe tests the engine beyond. Guard stays for normal runs.
            if std::env::var_os("BASTION_DETERMINISTIC_PARALLEL").is_some() {
                tracing::warn!(
                    rayon_threads,
                    "T0.52 PROBE RUN: deterministic seeds on a multi-worker pool — \
                     serial-vs-parallel equivalence experiment, not a shipping mode"
                );
            } else {
                assert_eq!(
                    rayon_threads, 1,
                    "ARCH-003 deterministic execution requires a one-worker Rayon pool"
                );
            }
        }

        // Load plugins before generating the world.
        #[cfg(feature = "plugins")]
        let plugin_mgr = PluginMgr::from_asset_or_default();

        debug!("Generating world, seed: {}", settings.world_seed);
        #[cfg(feature = "worldgen")]
        let (world, index) = World::generate(
            settings.world_seed,
            WorldOpts {
                seed_elements: true,
                world_file: if let Some(ref opts) = settings.map_file {
                    opts.clone()
                } else {
                    // Load default map from assets.
                    FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into())
                },
                calendar: Some(settings.calendar_mode.calendar_now()),
            },
            &pools,
            &|stage| {
                report_stage(ServerInitStage::WorldGen(stage));
            },
        );
        #[cfg(not(feature = "worldgen"))]
        let (world, index) = World::generate(settings.world_seed);

        #[cfg(feature = "worldgen")]
        let map = world.get_map_data(index.as_index_ref(), &pools);
        #[cfg(not(feature = "worldgen"))]
        let map = common_net::msg::WorldMapMsg {
            dimensions_lg: Vec2::zero(),
            max_height: 1.0,
            rgba: Grid::new(Vec2::new(1, 1), 1),
            horizons: [(vec![0], vec![0]), (vec![0], vec![0])],
            alt: Grid::new(Vec2::new(1, 1), 1),
            sites: Vec::new(),
            possible_starting_sites: Vec::new(),
            pois: Vec::new(),
            default_chunk: Arc::new(world.generate_oob_chunk()),
        };

        #[cfg(feature = "worldgen")]
        let map_size_lg = world.sim().map_size_lg();
        #[cfg(not(feature = "worldgen"))]
        let map_size_lg = world.map_size_lg();

        let lod = lod::Lod::from_world(&world, index.as_index_ref(), &pools);

        report_stage(ServerInitStage::StartingSystems);

        let mut state = State::server_with_mode(
            Arc::clone(&pools),
            map_size_lg,
            Arc::clone(&map.default_chunk),
            execution_mode,
            |dispatcher_builder| {
                add_local_systems(dispatcher_builder);
                sys::msg::add_server_systems(dispatcher_builder);
                sys::add_server_systems(dispatcher_builder);
                #[cfg(feature = "worldgen")]
                {
                    rtsim::add_server_systems(dispatcher_builder);
                    weather::add_server_systems(dispatcher_builder);
                }
            },
            #[cfg(feature = "plugins")]
            plugin_mgr,
        );
        events::register_event_busses(state.ecs_mut());
        state.ecs_mut().insert(battlemode_buffer);
        state.ecs_mut().insert(RecentClientIPs::default());
        state.ecs_mut().insert(settings.clone());
        state.ecs_mut().insert(editable_settings);
        state.ecs_mut().insert(DataDir {
            path: data_dir.to_owned(),
        });

        state.ecs_mut().insert(Vec::<ChunkRequest>::new());
        // bastion (B4): job board + harness-pinned chunk set.
        state.ecs_mut().insert(bastion_jobs::JobBoard::default());
        // bastion (PATH-0): the sequential path scheduler's state.
        state
            .ecs_mut()
            .insert(bastion_path::PathScheduler::default());
        state
            .ecs_mut()
            .insert(common::bastion::ActivityZones::default());
        state
            .ecs_mut()
            .insert(bastion_jobs::BastionForceLoaded::default());
        state
            .ecs_mut()
            .insert(EventBus::<chunk_serialize::ChunkSendEntry>::default());
        state.ecs_mut().insert(Locations::default());
        state.ecs_mut().insert(LoginProvider::new(
            settings.auth_server_address.clone(),
            Arc::clone(&runtime),
        ));
        state.ecs_mut().insert(HwStats {
            hardware_threads: num_cpus::get() as u32,
            rayon_threads: num_cpus::get() as u32,
        });
        state.ecs_mut().insert(ServerConstants {
            day_cycle_coefficient: settings.day_cycle_coefficient(),
        });
        state.ecs_mut().insert(Tick(0));
        state.ecs_mut().insert(TickStart(Instant::now()));
        state.ecs_mut().insert(job_metrics);
        state.ecs_mut().insert(network_request_metrics);
        state.ecs_mut().insert(player_metrics);
        state.ecs_mut().insert(ecs_system_metrics);
        state.ecs_mut().insert(tick_metrics);
        state.ecs_mut().insert(physics_metrics);
        state.ecs_mut().insert(server_event_metrics);
        state.ecs_mut().insert(gameplay_metrics);
        state.ecs_mut().insert(query_server_metrics);
        if settings.experimental_terrain_persistence {
            #[cfg(feature = "persistent_world")]
            {
                warn!(
                    "Experimental terrain persistence support is enabled. This feature may break, \
                     be disabled, or otherwise change under your feet at *any time*. \
                     Additionally, it is expected to be replaced in the future *without* \
                     migration or warning. You have been warned."
                );
                state
                    .ecs_mut()
                    .insert(TerrainPersistence::new(data_dir.to_owned()));
            }
            #[cfg(not(feature = "persistent_world"))]
            error!(
                "Experimental terrain persistence support was requested, but the server was not \
                 compiled with the feature. Terrain modifications will *not* be persisted."
            );
        }
        {
            let pool = state.ecs_mut().write_resource::<SlowJobPool>();
            pool.configure("CHUNK_DROP", |_n| 1);
            pool.configure("CHUNK_GENERATOR", |n| n / 2 + n / 4);
            pool.configure("CHUNK_SERIALIZER", |n| n / 2);
            pool.configure("RTSIM_SAVE", |_| 1);
            pool.configure("WEATHER", |_| 1);
        }
        state
            .ecs_mut()
            .insert(ChunkGenerator::new(chunk_gen_metrics));
        {
            let (sender, receiver) =
                crossbeam_channel::bounded::<chunk_serialize::SerializedChunk>(10_000);
            state.ecs_mut().insert(sender);
            state.ecs_mut().insert(receiver);
        }

        state.ecs_mut().insert(CharacterUpdater::new(
            Arc::<RwLock<DatabaseSettings>>::clone(&database_settings),
        )?);

        let ability_map = comp::item::tool::AbilityMap::<comp::AbilityItem>::load_expect_cloned(
            "common.abilities.ability_set_manifest",
        );
        state.ecs_mut().insert(ability_map);

        let msm = comp::inventory::item::MaterialStatManifest::load().cloned();
        state.ecs_mut().insert(msm);

        let rbm = common::recipe::RecipeBookManifest::load().cloned();
        state.ecs_mut().insert(rbm);

        state.ecs_mut().insert(CharacterLoader::new(
            Arc::<RwLock<DatabaseSettings>>::clone(&database_settings),
        )?);

        // System schedulers to control execution of systems
        state
            .ecs_mut()
            .insert(sys::PersistenceScheduler::every(Duration::from_secs(10)));

        // Region map (spatial structure for entity synchronization)
        state.ecs_mut().insert(RegionMap::new());

        // Server-only components
        state.ecs_mut().register::<RegionSubscription>();
        state.ecs_mut().register::<Client>();
        state.ecs_mut().register::<comp::Presence>();
        state.ecs_mut().register::<wiring::WiringElement>();
        state.ecs_mut().register::<wiring::Circuit>();
        state.ecs_mut().register::<Anchor>();
        state.ecs_mut().register::<comp::Pet>();
        state.ecs_mut().register::<login_provider::PendingLogin>();
        state.ecs_mut().register::<RepositionToFreeSpace>();
        state.ecs_mut().register::<RtSimEntity>();

        // Load banned words list
        let banned_words = settings.moderation.load_banned_words(data_dir);
        let censor = Arc::new(Censor::Custom(banned_words.into_iter().collect()));
        state.ecs_mut().insert(Arc::clone(&censor));

        // Init automod
        state
            .ecs_mut()
            .insert(AutoMod::new(&settings.moderation, censor));

        state.ecs_mut().insert(map);

        // bastion (FLAT-TEST-ARENA, live-path diagnostic): log the resolved
        // arena state at EVERY boot so a real singleplayer launch self-reports
        // whether the flag reached this server thread's env read (the
        // gate-green-but-inert bug's decisive signal — if this logs `false`
        // on a launch that set BASTION_FLAT_ARENA / passed --bastion-flat-arena,
        // the transport, not the override, is at fault).
        info!(
            flat_arena_enabled = bastion_flat_arena::enabled(),
            "bastion: FLAT-TEST-ARENA env check at server boot"
        );
        #[cfg(feature = "worldgen")]
        let spawn_point = SpawnPoint(if bastion_flat_arena::enabled() {
            // bastion (FLAT-TEST-ARENA): land on the slab, not at the
            // nearest town with a sim-derived (pre-override) altitude.
            bastion_flat_arena::spawn_wpos(bastion_flat_arena::world_center_wpos(&world))
        } else {
            let index = index.as_index_ref();
            // NOTE: all of these `.map(|e| e as [type])` calls should compile into no-ops,
            // but are needed to be explicit about casting (and to make the compiler stop
            // complaining)

            // Search for town defined by spawn_town server setting. If this fails, or is
            // None, set spawn to the nearest town to the centre of the world
            let center_chunk = world.sim().map_size_lg().chunks().map(i32::from) / 2;
            let spawn_chunk = world
                .civs()
                .sites()
                .filter(|site| site.is_settlement())
                .map(|site| site.center)
                .min_by_key(|site_pos| site_pos.distance_squared(center_chunk))
                .unwrap_or(center_chunk);

            world.find_accessible_pos(index, TerrainChunkSize::center_wpos(spawn_chunk), false)
        });
        #[cfg(not(feature = "worldgen"))]
        let spawn_point = if bastion_flat_arena::enabled() {
            SpawnPoint(bastion_flat_arena::spawn_wpos(
                bastion_flat_arena::world_center_wpos(&world),
            ))
        } else {
            SpawnPoint::default()
        };

        // Set the spawn point we calculated above
        state.ecs_mut().insert(spawn_point);

        // Insert a default AABB for the world
        // TODO: prevent this from being deleted
        {
            #[cfg(feature = "worldgen")]
            let size = world.sim().get_size();
            #[cfg(not(feature = "worldgen"))]
            let size = world.map_size_lg().chunks().map(u32::from);

            let world_size = size.map(|e| e as i32) * TerrainChunk::RECT_SIZE.map(|e| e as i32);
            let world_aabb = Aabb {
                min: Vec3::new(0, 0, -32768),
                max: Vec3::new(world_size.x, world_size.y, 32767),
            }
            .made_valid();

            state
                .ecs()
                .write_resource::<AreasContainer<BuildArea>>()
                .insert("world".to_string(), world_aabb)
                .expect("The initial insert should always work.");
        }

        // Insert the world into the ECS (todo: Maybe not an Arc?)
        let world = Arc::new(world);
        state.ecs_mut().insert(Arc::clone(&world));
        state.ecs_mut().insert(lod);
        state.ecs_mut().insert(index.clone());

        // Set starting time for the server.
        state.ecs_mut().write_resource::<TimeOfDay>().0 = settings.world.start_time;

        // Register trackers
        sys::sentinel::UpdateTrackers::register(state.ecs_mut());

        state.ecs_mut().insert(DeletedEntities::default());

        // Only allow clients to send us a maximum of 1 MB per uncompressed message, to
        // reduce the effectiveness of a DoS attack
        let network = Network::new_with_registry(Pid::new(), &runtime, &registry, 1 << 20);
        let (chat_cache, chat_tracker) = ChatCache::new(Duration::from_secs(60), &runtime);
        state.ecs_mut().insert(chat_tracker);

        let mut printed_quic_warning = false;
        for protocol in &settings.gameserver_protocols {
            match protocol {
                Protocol::Tcp { address } => {
                    runtime.block_on(network.listen(ListenAddr::Tcp(*address)))?;
                },
                Protocol::Quic {
                    address,
                    cert_file_path,
                    key_file_path,
                } => {
                    use rustls_pemfile::Item;
                    use std::fs;

                    match || -> Result<_, Box<dyn std::error::Error>> {
                        let key = fs::read(key_file_path)?;
                        let key = if key_file_path.extension().is_some_and(|x| x == "der") {
                            PrivateKeyDer::try_from(key).map_err(|_| "No valid pem key in file")?
                        } else {
                            debug!("convert pem key to der");
                            rustls_pemfile::read_all(&mut key.as_slice())
                                .find_map(|item| match item {
                                    Ok(Item::Pkcs1Key(v)) => Some(PrivateKeyDer::Pkcs1(v)),
                                    Ok(Item::Pkcs8Key(v)) => Some(PrivateKeyDer::Pkcs8(v)),
                                    Ok(Item::Sec1Key(v)) => Some(PrivateKeyDer::Sec1(v)),
                                    Ok(Item::Crl(_)) => None,
                                    Ok(Item::Csr(_)) => None,
                                    Ok(Item::X509Certificate(_)) => None,
                                    Ok(_) => None,
                                    Err(e) => {
                                        tracing::warn!(?e, "error while reading key_file");
                                        None
                                    },
                                })
                                .ok_or("No valid pem key in file")?
                        };
                        let cert_chain = fs::read(cert_file_path)?;
                        let cert_chain = if cert_file_path.extension().is_some_and(|x| x == "der") {
                            vec![CertificateDer::from(cert_chain)]
                        } else {
                            debug!("convert pem cert to der");
                            rustls_pemfile::certs(&mut cert_chain.as_slice())
                                .filter_map(|item| match item {
                                    Ok(cert) => Some(cert),
                                    Err(e) => {
                                        tracing::warn!(?e, "error while reading cert_file");
                                        None
                                    },
                                })
                                .collect()
                        };
                        let server_config = quinn::ServerConfig::with_single_cert(cert_chain, key)?;
                        Ok(server_config)
                    }() {
                        Ok(server_config) => {
                            runtime.block_on(
                                network.listen(ListenAddr::Quic(*address, server_config.clone())),
                            )?;

                            if !printed_quic_warning {
                                warn!(
                                    "QUIC is enabled. This is experimental and not recommended in \
                                     production"
                                );
                                printed_quic_warning = true;
                            }
                        },
                        Err(e) => {
                            error!(
                                ?e,
                                "Failed to load the TLS certificate, running without QUIC {}",
                                *address
                            );
                        },
                    }
                },
            }
        }

        if let Some(addr) = settings.query_address {
            use veloren_query_server::proto::ServerInfo;

            const QUERY_SERVER_RATELIMIT: u16 = 120;

            let (query_server_info_tx, query_server_info_rx) =
                tokio::sync::watch::channel(ServerInfo {
                    git_hash: *common::util::GIT_HASH,
                    git_timestamp: *common::util::GIT_TIMESTAMP,
                    players_count: 0,
                    player_cap: settings.max_players,
                    battlemode: settings.gameplay.battle_mode.into(),
                });
            let mut query_server =
                QueryServer::new(addr, query_server_info_rx, QUERY_SERVER_RATELIMIT);
            let query_server_metrics =
                Arc::new(Mutex::new(veloren_query_server::server::Metrics::default()));
            let query_server_metrics2 = Arc::clone(&query_server_metrics);
            runtime.spawn(async move {
                let err = query_server.run(query_server_metrics2).await.err();
                error!(?err, "Query server stopped unexpectedly");
            });
            state.ecs_mut().insert(query_server_info_tx);
            state.ecs_mut().insert(query_server_metrics);
        }

        runtime.block_on(network.listen(ListenAddr::Mpsc(14004)))?;

        let connection_handler = ConnectionHandler::new(network, &runtime);

        // Init rtsim, loading it from disk if possible
        #[cfg(feature = "worldgen")]
        {
            match rtsim::RtSim::new(
                &settings.world,
                settings.world_seed,
                index.as_index_ref(),
                &world,
                data_dir.to_owned(),
            ) {
                Ok(rtsim) => {
                    state.ecs_mut().insert(rtsim.state().data().time_of_day);
                    state.ecs_mut().insert(rtsim);
                },
                Err(err) => {
                    error!("Failed to load rtsim: {}", err);
                    return Err(Error::RtsimError(err));
                },
            }
            weather::init(&mut state);
        }

        let mut this = Self {
            state,
            world,
            index,
            connection_handler,
            runtime,

            metrics_registry: registry,
            chat_cache,
            database_settings,
            disconnect_all_clients_requested: false,

            event_dispatcher: Self::create_event_dispatcher(pools),
            execution_mode,
        };

        // bastion (B-ASSET1): the --asset-arena test chamber. Inert unless
        // the BASTION_ASSET_ARENA env var is set (voxygen sets it).
        #[cfg(feature = "worldgen")]
        this.bastion_arena_init_from_env();

        debug!(?settings, "created veloren server with");

        info!("Server version: {}", *common::util::DISPLAY_VERSION);

        Ok(this)
    }

    pub fn get_server_info(&self) -> ServerInfo {
        let settings = self.state.ecs().fetch::<Settings>();

        ServerInfo {
            name: settings.server_name.clone(),
            git_hash: *common::util::GIT_HASH,
            git_timestamp: *common::util::GIT_TIMESTAMP,
            auth_provider: settings.auth_server_address.clone(),
        }
    }

    /// Get a reference to the server's settings
    pub fn settings(&self) -> impl Deref<Target = Settings> + '_ {
        self.state.ecs().fetch::<Settings>()
    }

    /// Get a mutable reference to the server's settings
    pub fn settings_mut(&self) -> impl DerefMut<Target = Settings> + '_ {
        self.state.ecs().fetch_mut::<Settings>()
    }

    /// Get a mutable reference to the server's editable settings
    pub fn editable_settings_mut(&self) -> impl DerefMut<Target = EditableSettings> + '_ {
        self.state.ecs().fetch_mut::<EditableSettings>()
    }

    /// Get a reference to the server's editable settings
    pub fn editable_settings(&self) -> impl Deref<Target = EditableSettings> + '_ {
        self.state.ecs().fetch::<EditableSettings>()
    }

    /// Get path to the directory that the server info into
    pub fn data_dir(&self) -> impl Deref<Target = DataDir> + '_ {
        self.state.ecs().fetch::<DataDir>()
    }

    /// Get a reference to the server's game state.
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the server's game state.
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Get a reference to the server's world.
    pub fn world(&self) -> &World { &self.world }

    /// bastion (B3): spawn the player-colony starting band near `wpos` (used
    /// by the headless harness; in-game the client message drives it).
    /// Returns the roster names.
    pub fn bastion_spawn_colony(&mut self, wpos: Vec3<f32>, count: u8) -> Vec<String> {
        self.state
            .ecs()
            .write_resource::<rtsim::RtSim>()
            .bastion_spawn_colony(wpos, count)
    }

    /// Determinism-capture founding: seed the colony from an explicit tick
    /// (see [`rtsim::RtSim::bastion_spawn_colony_seeded`]).
    pub fn bastion_spawn_colony_seeded(
        &mut self,
        wpos: Vec3<f32>,
        count: u8,
        seed_tick: u64,
    ) -> Vec<String> {
        self.state
            .ecs()
            .write_resource::<rtsim::RtSim>()
            .bastion_spawn_colony_seeded(wpos, count, seed_tick)
    }

    /// bastion (SEASON-0, harness hook): (season-index, year_phase,
    /// day_of_year, days_in_year) at a given TimeOfDay under the LOADED
    /// RON config — the in-vivo derivation probe (pure of any stored
    /// state by construction).
    pub fn bastion_season_probe(&self, tod: f64) -> (u8, f64, u32, f64) {
        use common::time::{Season, SeasonConfig, day_of_year, year_phase};
        let cfg = SeasonConfig::current();
        let season = match Season::at(tod, cfg.days_in_year) {
            Season::Spring => 0,
            Season::Summer => 1,
            Season::Autumn => 2,
            Season::Winter => 3,
        };
        (
            season,
            year_phase(tod, cfg.days_in_year),
            day_of_year(tod, cfg.days_in_year),
            cfg.days_in_year,
        )
    }

    /// bastion (B7-0, harness hook): (hunger, rest, recreation, mood) for
    /// a named loaded colonist.
    pub fn bastion_colonist_needs_mood(&self, name: &str) -> Option<(f32, f32, f32, f32)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let needs = ecs.read_storage::<comp::bastion::Needs>();
        let moods = ecs.read_storage::<comp::bastion::Mood>();
        (&entities, &colonists, &needs, &moods)
            .join()
            .find(|(_, c, _, _)| c.0.name == name)
            .map(|(_, _, n, m)| (n.hunger, n.rest, n.recreation, m.0))
    }

    /// STATUS-SURFACE: harness probe for the inspector's status line + energy
    /// fraction — routed through the SAME pure classifier as the live wire
    /// fill (`bastion_jobs::colonist_status_display`), so probe and wire
    /// cannot drift. READ-ONLY.
    pub fn bastion_colonist_status(
        &self,
        name: &str,
    ) -> Option<(Option<common::comp::bastion::BastionColonistStatus>, f32)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let energies = ecs.read_storage::<comp::Energy>();
        let board = ecs.read_resource::<crate::bastion_jobs::JobBoard>();
        let tick = ecs.read_resource::<Tick>();
        (&entities, &colonists, &uids)
            .join()
            .find(|(_, c, _)| c.0.name == name)
            .map(|(e, _, uid)| {
                (
                    crate::bastion_jobs::colonist_status(&board, *uid, tick.0),
                    energies.get(e).map_or(0.0, |en| en.fraction()),
                )
            })
    }

    /// bastion (B7-0, harness hook): TEST setter for a named colonist's
    /// meters — drives the starved-case formula assert (the next mood
    /// cadence recomputes from these).
    pub fn bastion_set_needs(
        &mut self,
        name: &str,
        hunger: f32,
        rest: f32,
        recreation: f32,
    ) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let mut needs = ecs.write_storage::<comp::bastion::Needs>();
        for (_, c, n) in (&entities, &colonists, &mut needs).join() {
            if c.0.name == name {
                n.hunger = hunger;
                n.rest = rest;
                n.recreation = recreation;
                return true;
            }
        }
        false
    }

    /// bastion (B-AG3 slice 1, harness hook): set ONE value weight (±50)
    /// on a named colonist — the slice's sole writer (rolling is a later
    /// slice). Colonist storage is change-tracked: find immutably, then
    /// `get_mut` (the bastion_assign_bed_owner pattern). Unknown value
    /// names return false (the vocabulary is the locked enum).
    pub fn bastion_set_values(&mut self, name: &str, value: &str, weight: i8) -> bool {
        use common::bastion::Value;
        use specs::Join;
        let value = match value {
            "Glory" => Value::Glory,
            "Tradition" => Value::Tradition,
            "Kin" => Value::Kin,
            "Wealth" => Value::Wealth,
            "Piety" => Value::Piety,
            "Nature" => Value::Nature,
            "Craft" => Value::Craft,
            "Freedom" => Value::Freedom,
            _ => return false,
        };
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let mut colonists = ecs.write_storage::<comp::Colonist>();
        let found = (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .map(|(e, _)| e);
        if let Some(e) = found
            && let Some(mut c) = colonists.get_mut(e)
        {
            c.0.values.insert(value, weight);
            return true;
        }
        false
    }

    /// bastion (B-AG3 slice 1, harness hook): queue ONE chronicle thought
    /// for a named colonist at its own feet — the values-divergence
    /// scenario's deterministic depositor. Rides the REAL pipeline from
    /// the queue on (board `pending_thoughts` → the rtsim tick's drain →
    /// `chronicle.record` → the %11 recompute reads it back through the
    /// care weighting); only the EMITTER is synthetic (the CAVEIN leg
    /// owns the live-emitter path). Kinds limited to the thought-table
    /// set; unknown names return false.
    pub fn bastion_deposit_thought(&mut self, name: &str, kind: &str) -> bool {
        use specs::Join;
        let kind = match kind {
            "Death" => ::rtsim::data::ChronicleKind::Death,
            "CaveIn" => ::rtsim::data::ChronicleKind::CaveIn,
            "SleptInBed" => ::rtsim::data::ChronicleKind::SleptInBed,
            "SleptOnGround" => ::rtsim::data::ChronicleKind::SleptOnGround,
            _ => return false,
        };
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let positions = ecs.read_storage::<comp::Pos>();
        let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        for (_, c, p, re) in (&entities, &colonists, &positions, &rtsim_entities).join() {
            if c.0.name == name {
                board
                    .pending_thoughts
                    .push((*re, p.0.map(|v| v.floor() as i32), kind));
                return true;
            }
        }
        false
    }

    /// bastion (FOCUS-0-DERIVE, harness hook): CLEAR a named colonist's
    /// value weights. Since 43.1 rolls REAL values at generation, exact
    /// care-math fixtures (the VALUES leg) clear first, then set the one
    /// weight under test — composable with bastion_set_values.
    pub fn bastion_clear_values(&mut self, name: &str) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let mut colonists = ecs.write_storage::<comp::Colonist>();
        let found = (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .map(|(e, _)| e);
        if let Some(e) = found
            && let Some(mut c) = colonists.get_mut(e)
        {
            c.0.values.clear();
            return true;
        }
        false
    }

    /// bastion (FOCUS-0-DERIVE, harness hook): the LIVE derived need
    /// weight for a named colonist — reads their rolled values (ECS) +
    /// vanilla personality (rtsim, boolean-trait API) through the same
    /// lookup shape the mood recompute uses, then the pure derivation.
    pub fn bastion_derived_need_weight(&self, name: &str, need: &str) -> Option<f32> {
        use common::bastion::Need;
        use specs::Join;
        let need = match need {
            "Pray" => Need::Pray,
            "Socialize" => Need::Socialize,
            "Drink" => Need::Drink,
            "Craft" => Need::Craft,
            "Family" => Need::Family,
            "SeeAnimals" => Need::SeeAnimals,
            "AdmireArt" => Need::AdmireArt,
            "Learn" => Need::Learn,
            "Acquire" => Need::Acquire,
            "Fight" => Need::Fight,
            _ => return None,
        };
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
        let rtsim = ecs.read_resource::<rtsim::RtSim>();
        let data = rtsim.state().data();
        (&colonists, &rtsim_entities)
            .join()
            .find(|(c, _)| c.0.name == name)
            .and_then(|(c, re)| {
                data.npcs.get(*re).map(|npc| {
                    comp::bastion::derive_need_weight(need, &npc.personality, &c.0.values)
                })
            })
    }

    /// bastion (FOCUS-0-DERIVE, harness hook): a named colonist's
    /// boolean personality trait (the vanilla public API) — the roster
    /// correlation groups by trait independently of the weight probe.
    pub fn bastion_colonist_trait(&self, name: &str, trait_name: &str) -> Option<bool> {
        use common::rtsim::PersonalityTrait;
        use specs::Join;
        let t = match trait_name {
            "Extroverted" => PersonalityTrait::Extroverted,
            "Introverted" => PersonalityTrait::Introverted,
            "Sociable" => PersonalityTrait::Sociable,
            "Neurotic" => PersonalityTrait::Neurotic,
            _ => return None,
        };
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
        let rtsim = ecs.read_resource::<rtsim::RtSim>();
        let data = rtsim.state().data();
        (&colonists, &rtsim_entities)
            .join()
            .find(|(c, _)| c.0.name == name)
            .and_then(|(_, re)| data.npcs.get(*re).map(|npc| npc.personality.is(t)))
    }

    /// bastion (B-AG3 slice 1, harness hook): a named colonist's value
    /// weights, name-sorted (probe/round-trip verification).
    pub fn bastion_colonist_values(&self, name: &str) -> Vec<(String, i8)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let mut out: Vec<(String, i8)> = colonists
            .join()
            .find(|c| c.0.name == name)
            .map(|c| {
                c.0.values
                    .iter()
                    .map(|(v, w)| (format!("{v:?}"), *w))
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    }

    /// bastion (AUTON-0, harness hook): set a colonist's health fraction
    /// — drives the below-flee-health signal deterministically (the
    /// scenario's Flee trigger; no synthetic drive injection, the REAL
    /// signal path).
    pub fn bastion_set_health_fraction(&mut self, name: &str, fraction: f32) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let mut healths = ecs.write_storage::<comp::Health>();
        for (e, c) in (&entities, &colonists).join() {
            if c.0.name == name {
                if let Some(mut h) = healths.get_mut(e) {
                    h.set_fraction(fraction);
                    return true;
                }
            }
        }
        false
    }

    /// bastion (AUTON-0, harness hook): a colonist's current drive.
    pub fn bastion_colonist_drive(&self, name: &str) -> Option<String> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let arbiters = ecs.read_storage::<comp::bastion::Arbiter>();
        (&colonists, &arbiters)
            .join()
            .find(|(c, _)| c.0.name == name)
            .map(|(_, a)| format!("{:?}", a.current))
    }

    /// bastion (AUTON-0, harness hook): cumulative drive switches (the
    /// thrash bound reads the delta).
    pub fn bastion_drive_switches(&self) -> u64 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .drive_switches
    }

    /// bastion (RUN-0, harness hook): flip a colonist's emergency-run
    /// flag — the TEST trigger (RUN-1 owns real triggers). The governor
    /// still force-reverts it at the energy floor regardless.
    pub fn bastion_set_running(&mut self, name: &str, running: bool) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let mut colonists = ecs.write_storage::<comp::Colonist>();
        let found = (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .map(|(e, _)| e);
        if let Some(e) = found
            && let Some(mut c) = colonists.get_mut(e)
        {
            c.0.running = running;
            return true;
        }
        false
    }

    /// bastion (RUN-0, harness hook): a colonist's (energy current, max,
    /// running) — the governor's probes.
    pub fn bastion_colonist_energy(&self, name: &str) -> Option<(f32, f32, bool)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let energies = ecs.read_storage::<comp::Energy>();
        (&colonists, &energies)
            .join()
            .find(|(c, _)| c.0.name == name)
            .map(|(c, e)| (e.current(), e.maximum(), c.0.running))
    }

    /// bastion (B5.8 fixture, FABLE-003-blessed SETUP-ONLY staging hook —
    /// permitted-touch item 4): set a named colonist's energy to an absolute
    /// `target` (drained-miner staging, Ben's ruling: a trapped miner is
    /// mid-shift, not rested; FABLE-003 caps staging at target ≤ 0.1).
    /// Constraints (all four, hers): Energy-component-ONLY (no other comp is
    /// touched); SETUP-ONLY — calling after the episode marker is the
    /// INV-HARNESS-ENERGY falsifier (`energy-stage-after-episode-start`,
    /// FAIL-by-construction, same family as teleport/goto_clear); every call
    /// emits a recorder staging event when the recorder is enabled; the
    /// INV-HARNESS-ENERGY writer-inventory row is the fixture's evidence-side
    /// obligation. Mutation via the comp's own `Energy::change_by`.
    pub fn bastion_set_colonist_energy(&mut self, name: &str, target: f32) -> bool {
        use specs::LendJoin;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let mut energies = ecs.write_storage::<comp::Energy>();
        let mut done = false;
        let mut iter = (&colonists, &uids, &mut energies).lend_join();
        while let Some((c, u, mut e)) = iter.next() {
            if c.0.name == name {
                let delta = target - e.current();
                e.change_by(delta);
                if crate::bastion_flight_recorder::enabled() {
                    crate::bastion_flight_recorder::record_writer(
                        crate::bastion_flight_recorder::WriterEvent {
                            schema: "bastion.flight-recorder.writer/v1".into(),
                            tick: 0,
                            uid: u.0.get(),
                            observation_sequence: 0,
                            snapshot_stage: "harness-setup".into(),
                            dispatcher_dependency_proven: false,
                            writer: "harness-energy-stage".into(),
                            move_dir: [0.0, 0.0],
                            move_z: 0.0,
                            target: None,
                            note: format!("set_colonist_energy target={target}"),
                        },
                    );
                }
                done = true;
            }
        }
        done
    }

    /// bastion (FABLE-003 pattern, harness STAGING hook, setup-only): pin a
    /// colonist's climbing level. Falsifier preconditions must be STRUCTURAL,
    /// not spawn-lottery — colonists roll climbing 0..=1 at spawn, and the
    /// seed corpus proved a level-1 roll (cap 6 + scramble 3) legitimately
    /// exits the probe's depth-8 shaft. Constraints (INV-HARNESS-CLIMB-LEVEL):
    /// climbing skill only; scenario SETUP only (never mid-window); recorder
    /// staging event per call; the calling scenario carries a staged flag in
    /// its JSON.
    pub fn bastion_set_colonist_climb_level(&mut self, name: &str, level: u16) -> bool {
        use specs::LendJoin;
        let ecs = self.state.ecs();
        let mut staged_uid = None;
        let mut colonists = ecs.write_storage::<comp::Colonist>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let mut done = false;
        let mut iter = (&mut colonists, &uids).lend_join();
        while let Some((mut c, u)) = iter.next() {
            if c.0.name == name {
                c.0.skills.climbing = common::bastion::SkillLevel { level, xp: 0.0 };
                staged_uid = Some(*u);
                if crate::bastion_flight_recorder::enabled() {
                    crate::bastion_flight_recorder::record_writer(
                        crate::bastion_flight_recorder::WriterEvent {
                            schema: "bastion.flight-recorder.writer/v1".into(),
                            tick: 0,
                            uid: u.0.get(),
                            observation_sequence: 0,
                            snapshot_stage: "harness-setup".into(),
                            dispatcher_dependency_proven: false,
                            writer: "harness-climb-level-stage".into(),
                            move_dir: [0.0, 0.0],
                            move_z: 0.0,
                            target: None,
                            note: format!("set_colonist_climb_level level={level}"),
                        },
                    );
                }
                done = true;
            }
        }
        drop(iter);
        drop(colonists);
        drop(uids);
        // A stage must also clear any live episode snapshot — setup ticks can
        // have or_insert'ed the PRE-staging spawn roll (frozen-verify tape:
        // level=0 yet cap_blocks=6 for the whole episode).
        if let Some(uid) = staged_uid {
            ecs.write_resource::<bastion_jobs::JobBoard>()
                .staging_clear_climb_snapshot(&uid);
        }
        done
    }

    /// bastion (FARM/PROD-2, harness hook): the COLONY-TOTAL count of an
    /// item def — loose ground items PLUS every colonist's bag (the
    /// seed-conservation invariant counts both: a fetched stack lives in
    /// a bag, invisible to the ground-only counter).
    pub fn bastion_colony_item_total(&self, asset_id: &str) -> u64 {
        use specs::Join;
        let ecs = self.state.ecs();
        let items = ecs.read_storage::<comp::PickupItem>();
        let ground: u64 = (&items)
            .join()
            .filter(|pi| pi.item().item_definition_id().itemdef_id() == Some(asset_id))
            .map(|pi| pi.item().amount() as u64)
            .sum();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let inventories = ecs.read_storage::<comp::Inventory>();
        let bags: u64 = (&colonists, &inventories)
            .join()
            .flat_map(|(_, inv)| {
                inv.slots().flatten().filter_map(|it| {
                    (it.item_definition_id().itemdef_id() == Some(asset_id))
                        .then(|| it.amount() as u64)
                })
            })
            .sum();
        ground + bags
    }

    /// bastion (FARM/PROD-2, harness hook): a cell's crop growth stage
    /// (None = no sprite / no Growth attr) — the scenario's staged-
    /// growth probe.
    pub fn bastion_sprite_growth(&self, pos: Vec3<i32>) -> Option<u8> {
        use common::vol::ReadVol;
        self.state.terrain().get(pos).ok().and_then(|b| {
            b.get_attr::<common::terrain::sprite::Growth>()
                .ok()
                .map(|g| g.0)
        })
    }

    /// bastion (PATH-0, harness hook): the path scheduler's telemetry —
    /// (grants_total, peak_tick_iters, peak_wait). The scenario asserts
    /// the cap held and no requester was starved.
    pub fn bastion_path_stats(&self) -> (u64, u64, u32) {
        let s = self
            .state
            .ecs()
            .read_resource::<bastion_path::PathScheduler>();
        (s.grants_total, s.peak_tick_iters, s.peak_wait)
    }

    /// bastion (B7-2, harness hook): cumulative preempt attempts (the
    /// anti-thrash rate bound reads the delta over a window).
    pub fn bastion_preempt_attempts(&self) -> u64 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .preempt_attempts
    }

    /// bastion (B7-2, harness hook): register a bed slot DIRECTLY on the
    /// board (the completion arm's registration, callable) — preemption
    /// tests need beds to exist without re-proving the build pipeline
    /// (the BED leg owns that), and the unreachable-endure fixture needs
    /// a bed sealed inside solid rock.
    pub fn bastion_register_bed(&mut self, pos: Vec3<i32>) {
        let ecs = self.state.ecs();
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        board.beds.insert(pos, common::bastion::BedSlot {
            kind: common::bastion::BedKind::Bedroll,
            owner: None,
            occupant: None,
        });
    }

    /// bastion (B7-1, harness hook): assign bed OWNERSHIP — writes the
    /// board slot's fast lookup AND the colonist record's persistent
    /// truth (mirrored by colonist_record every loaded tick).
    pub fn bastion_assign_bed_owner(&mut self, name: &str, pos: Vec3<i32>) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let mut colonists = ecs.write_storage::<comp::Colonist>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        // Colonist's storage is change-tracked: find immutably, then
        // get_mut (the tick.rs idiom).
        let found = (&entities, &colonists, &uids)
            .join()
            .find(|(_, c, _)| c.0.name == name)
            .map(|(e, _, u)| (e, *u));
        if let Some((e, uid)) = found {
            let Some(slot) = board.beds.get_mut(&pos) else {
                return false;
            };
            slot.owner = Some(uid);
            if let Some(mut c) = colonists.get_mut(e) {
                c.0.owned_bed = Some(pos);
            }
            return true;
        }
        false
    }

    /// bastion (B7-1, harness hook): a PRE-CLAIMED RestAt job (the
    /// DepositRun insertion pattern — B7-2's preempt trigger creates
    /// these automatically later).
    pub fn bastion_assign_rest(&mut self, name: &str, bed_pos: Vec3<i32>) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let mut active_jobs = ecs.write_storage::<comp::bastion::ActiveJob>();
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        for (e, c, uid) in (&entities, &colonists, &uids).join() {
            if c.0.name == name {
                if active_jobs.contains(e) {
                    return false;
                }
                let id = board.insert_rest_job(bed_pos, *uid);
                let _ = active_jobs.insert(e, comp::bastion::ActiveJob {
                    job: id,
                    state: comp::bastion::ActiveJobState::Traveling,
                    best_dist: f32::MAX,
                    stuck_time: 0.0,
                    reset_dist: f32::MAX,
                    soft_granted: false,
                    stance: Vec3::unit_z(),
                });
                return true;
            }
        }
        false
    }

    /// bastion (B7-1, harness hook): a bed slot's (owner, occupant) as
    /// raw uid u64s, `None` if no bed there.
    pub fn bastion_bed_slot(&self, pos: Vec3<i32>) -> Option<(Option<u64>, Option<u64>)> {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        board
            .beds
            .get(&pos)
            .map(|s| (s.owner.map(|u| u.0.get()), s.occupant.map(|u| u.0.get())))
    }

    /// bastion (B7-1, harness hook): a named colonist's uid as u64.
    pub fn bastion_colonist_uid(&self, name: &str) -> Option<u64> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        (&colonists, &uids)
            .join()
            .find(|(c, _)| c.0.name == name)
            .map(|(_, u)| u.0.get())
    }

    /// bastion (M2 fixture, harness hook, READ-ONLY): traversal-task
    /// introspection — (phase_name, reserved_is_self, abort_reason). The
    /// ladder fixture asserts phase progressions on this; no writers.
    pub fn bastion_traversal_probe(
        &self,
        name: &str,
    ) -> Option<(String, bool, Option<&'static str>)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let uid = {
            let colonists = ecs.read_storage::<comp::Colonist>();
            let uids = ecs.read_storage::<common::uid::Uid>();
            (&colonists, &uids)
                .join()
                .find(|(c, _)| c.0.name == name)
                .map(|(_, u)| *u)?
        };
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        board.traversal_probe(&uid)
    }

    /// bastion (M2 fixture STAGING): replace a colonist's consumables with
    /// ONE asset-deterministic food item. Registry class 7 (nondeterministic
    /// item identity — hash AND slot) otherwise injects real behavioral
    /// divergence through the post-damage eat: byte-identical runs draw food
    /// from different slots and their downstream trajectories fork (N6 x2
    /// comparator, 6990/9002 divergent samples). Staging-only, called before
    /// the episode's start marker; the fixture asserts the result as its own
    /// precondition. Returns the number of consumables replaced.
    pub fn bastion_canonicalize_colonist_food(&mut self, name: &str) -> Option<usize> {
        use common::comp::item::{Item, ItemKind};
        use specs::Join;
        let ecs = self.state.ecs();
        let entity = {
            let colonists = ecs.read_storage::<comp::Colonist>();
            let entities = ecs.entities();
            (&entities, &colonists)
                .join()
                .find(|(_, c)| c.0.name == name)
                .map(|(e, _)| e)?
        };
        let mut inventories = ecs.write_storage::<comp::Inventory>();
        let mut inventory = inventories.get_mut(entity)?;
        let ids: Vec<_> = inventory
            .slots_with_id()
            .filter(|(_, slot)| {
                slot.as_ref()
                    .is_some_and(|item| matches!(&*item.kind(), ItemKind::Consumable { .. }))
            })
            .map(|(id, _)| id)
            .collect();
        let removed = ids.len();
        for id in ids {
            let _ = inventory.remove(id);
        }
        let _ = inventory.push(Item::new_from_asset_expect("common.items.food.cheese"));
        Some(removed)
    }

    /// bastion (M2 fixture N1B, harness read): dismount anchor of the named
    /// colonist's live traversal route (None when no task/descriptor).
    pub fn bastion_route_dismount(&self, name: &str) -> Option<Vec3<i32>> {
        use specs::Join;
        let ecs = self.state.ecs();
        let uid = {
            let colonists = ecs.read_storage::<comp::Colonist>();
            let uids = ecs.read_storage::<common::uid::Uid>();
            (&colonists, &uids)
                .join()
                .find(|(c, _)| c.0.name == name)
                .map(|(_, u)| *u)?
        };
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        board.route_dismount(&uid)
    }

    /// bastion (M2 fixture, PERMITTED TOUCH 3): emit real damage through the
    /// PRODUCTION event bus so the Apply-phase handler produces
    /// `AgentEvent::Hurt` (INV-INBOX-HURT). Writing `agent.inbox`, `Health`,
    /// or any component directly is prohibited — that would fake the
    /// interruption path. Event-emission only.
    pub fn bastion_emit_damage(&mut self, name: &str, amount: f32) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let (entity, time) = {
            let colonists = ecs.read_storage::<comp::Colonist>();
            let entities = ecs.entities();
            let time = *ecs.read_resource::<common::resources::Time>();
            let Some(entity) = (&entities, &colonists)
                .join()
                .find(|(_, c)| c.0.name == name)
                .map(|(e, _)| e)
            else {
                return false;
            };
            (entity, time)
        };
        ecs.read_resource::<common::event::EventBus<common::event::HealthChangeEvent>>()
            .emit_now(common::event::HealthChangeEvent {
                entity,
                change: comp::HealthChange {
                    amount: -amount.abs(),
                    by: None,
                    cause: Some(common::DamageSource::Falling),
                    time,
                    precise: false,
                    // Fixed instance: common::combat::next_attack_instance() made N6 nondeterministic
                    // across identical runs (the x2 comparator caught it).
                    instance: 0xBA57_10D4,
                },
            });
        true
    }

    /// bastion (F5 falsifier, harness hook, READ-ONLY): egress introspection
    /// for one colonist — (has_egress_target, owned_live_access_jobs,
    /// live_access_jobs_total). The redesigned stuckjob leg asserts its own
    /// preconditions on these; no writers.
    pub fn bastion_egress_probe(&self, name: &str) -> Option<(bool, usize, usize)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let uid = {
            let colonists = ecs.read_storage::<comp::Colonist>();
            let uids = ecs.read_storage::<common::uid::Uid>();
            (&colonists, &uids)
                .join()
                .find(|(c, _)| c.0.name == name)
                .map(|(_, u)| *u)?
        };
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        Some(board.egress_probe(uid))
    }

    /// bastion (B7-1, harness hook): the persistent owned-bed key on the
    /// colonist record (the save/load roundtrip probe).
    pub fn bastion_colonist_owned_bed(&self, name: &str) -> Option<Vec3<i32>> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        colonists
            .join()
            .find(|c| c.0.name == name)
            .and_then(|c| c.0.owned_bed)
    }

    /// bastion (B7-1, harness hook): KILL a named colonist (health to
    /// zero through the normal damage path) — the kill-while-sleeping
    /// occupancy-release assert drives this.
    pub fn bastion_kill_colonist(&mut self, name: &str) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let time = *ecs.read_resource::<common::resources::Time>();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let mut healths = ecs.write_storage::<comp::Health>();
        let found = (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .map(|(e, _)| e);
        let _ = time;
        if let Some(e) = found
            && let Some(mut health) = healths.get_mut(e)
        {
            // The real death API — plain damage is absorbed by death
            // protection; kill() zeroes health AND strips it.
            health.kill();
            return true;
        }
        false
    }

    /// bastion (SEASON-0, harness hook): the server's CURRENT TimeOfDay
    /// (the master clock the derivation reads).
    pub fn bastion_time_of_day(&self) -> f64 {
        self.state
            .ecs()
            .read_resource::<common::resources::TimeOfDay>()
            .0
    }

    /// bastion (SEASON-1, harness hook): does the named event fire on the
    /// given day-of-year, through the LOADED RON schedule (the exact query
    /// consumers will use)?
    pub fn bastion_seasonal_event(&self, day_of_year: u32, name: &str) -> bool {
        common::time::SeasonalSchedule::current().is_event_on(day_of_year, name)
    }

    /// bastion (SEASON-1, harness hook): every scheduled event name firing
    /// on the given day-of-year (name-sorted), through the loaded schedule.
    pub fn bastion_seasonal_events_on(&self, day_of_year: u32) -> Vec<String> {
        common::time::SeasonalSchedule::current()
            .events_on(day_of_year)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// bastion (B-AG2, harness hook): the archetype table's weight for
    /// (key, activity) — probes the EXACT lookup path the brain's
    /// converted gates use (`archetype_chance`).
    pub fn bastion_archetype_weight(&self, key: &str, activity: &str) -> Option<f32> {
        ::rtsim::rule::npc_ai::archetype::archetype_chance(key, activity)
    }

    /// bastion (B-AG2, harness hook): an archetype's full allowed set
    /// (name-sorted) — the same-code/different-data contrast probe.
    pub fn bastion_archetype_allowed(&self, key: &str) -> Vec<(String, f32)> {
        ::rtsim::rule::npc_ai::archetype::allowed_set(key)
    }

    /// bastion (B-AG2, harness hook): (herbalist, hunter, guard) counts in
    /// the generated rtsim population.
    pub fn bastion_profession_census(&self) -> (usize, usize, usize) {
        self.state
            .ecs()
            .read_resource::<rtsim::RtSim>()
            .bastion_profession_census()
    }

    /// bastion (HIST-0, harness hook): soak-record chronicle test events
    /// (band: 0 Routine / 1 Notable / other Legendary) through THE ONE
    /// capture entry point.
    pub fn bastion_chronicle_record_test(&mut self, band: u8, n: u32) -> u64 {
        self.state
            .ecs()
            .write_resource::<rtsim::RtSim>()
            .bastion_chronicle_record_test(band, n)
    }

    /// bastion (HIST-0, harness hook): (routine, notable, legendary)
    /// chronicle counts.
    pub fn bastion_chronicle_counts(&self) -> (usize, usize, usize) {
        self.state
            .ecs()
            .read_resource::<rtsim::RtSim>()
            .bastion_chronicle_counts()
    }

    /// bastion (HIST-0, harness hook): end-of-time sweep + the B10
    /// persistence round-trip, byte-for-byte on the chronicle.
    pub fn bastion_chronicle_roundtrip(&mut self) -> bool {
        self.state
            .ecs()
            .write_resource::<rtsim::RtSim>()
            .bastion_chronicle_roundtrip()
    }

    /// bastion (B3): the colony roster.
    pub fn bastion_colony_roster(&self) -> Vec<common::bastion::BastionColonist> {
        self.state
            .ecs()
            .read_resource::<rtsim::RtSim>()
            .bastion_colony_roster()
    }

    /// bastion (B4, harness/scenario hook): synchronously generate + insert
    /// chunks in a square around `center_wpos` and pin them against the
    /// unload sweep. Mirrors the vanilla insertion recipe (terrain insert +
    /// `TerrainChanges::new_chunks` + `rtsim.hook_load_chunk`); chunk
    /// supplements (wildlife spawns) are deliberately skipped. Returns the
    /// number of chunks generated.
    pub fn bastion_force_load_area(&mut self, center_wpos: Vec2<f32>, chunk_radius: i32) -> usize {
        use common::terrain::CoordinateConversions;
        let center = center_wpos.as_::<i32>().wpos_to_cpos();
        let mut generated = 0;
        for dy in -chunk_radius..=chunk_radius {
            for dx in -chunk_radius..=chunk_radius {
                let key = center + Vec2::new(dx, dy);
                self.state
                    .ecs()
                    .write_resource::<bastion_jobs::BastionForceLoaded>()
                    .0
                    .insert(key);
                if self.state.terrain().get_key_arc(key).is_some() {
                    continue;
                }
                // bastion (FLAT-TEST-ARENA): the same runtime override the
                // live chunk generator applies — the harness force-load
                // path honors the arena too (the ARENA leg tests through
                // here).
                let gen_result = crate::bastion_flat_arena::override_chunk(
                    crate::bastion_flat_arena::world_center_wpos(&self.world),
                    key,
                )
                .ok_or(())
                .or_else(|()| {
                    self.world.generate_chunk(
                        self.index.as_index_ref(),
                        key,
                        None,
                        // NOTE: despite the name, this closure means
                        // "cancel?" (see chunk_generator.rs's
                        // `cancel.load(..)`).
                        || false,
                        None,
                    )
                });
                let Ok((chunk, supplement)) = gen_result else {
                    continue;
                };
                let chunk = std::sync::Arc::new(chunk);
                let ecs = self.state.ecs();
                let mut terrain = ecs.write_resource::<common::terrain::TerrainGrid>();
                let mut changes = ecs.write_resource::<common_state::TerrainChanges>();
                if terrain.insert(key, chunk).is_none() {
                    changes.new_chunks.insert(key);
                    ecs.write_resource::<rtsim::RtSim>().hook_load_chunk(
                        key,
                        supplement.rtsim_max_resources,
                        &self.world,
                    );
                }
                generated += 1;
            }
        }
        generated
    }

    /// bastion (B-ASSET1): load an asset-lab asset (runs the marker-fidelity
    /// gate) and stamp it into live terrain at `origin` through the
    /// authoritative `BlockChange` path. `open_variant` selects the
    /// operable-open marker mapping (gate bars ↔ carved air). Placement
    /// proceeds even when fidelity checks fail — callers (harness/arena)
    /// decide fatality from `LoadedAsset::fidelity_ok`; a malformed vox is an
    /// `Err` (log + skip, never panic).
    #[cfg(feature = "worldgen")]
    pub fn bastion_asset_place(
        &mut self,
        entry: &bastion_assets::AssetLabEntry,
        origin: Vec3<i32>,
        open_variant: bool,
        seed: u32,
    ) -> Result<(bastion_assets::LoadedAsset, bastion_assets::PlacementReport), String> {
        let loaded = bastion_assets::load_asset(entry, open_variant)?;
        let report = bastion_assets::place_structure(
            &mut self.state,
            &self.world,
            self.index.as_index_ref(),
            &loaded,
            origin,
            seed,
        );
        Ok((loaded, report))
    }

    /// bastion (B4, harness hook): place a designation directly on the board.
    /// Returns created job ids.
    pub fn bastion_place_designation(
        &mut self,
        region: common::bastion::Region,
        kind: common::bastion::DesignationKind,
    ) -> Vec<common::bastion::JobId> {
        let ecs = self.state.ecs();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        board.place_designation(&terrain, region.normalized(), kind)
    }

    /// bastion (AUTON-3, harness hook): a named colonist's last drive
    /// scores `(work, flee, idle)` — the post-modulation urgencies the
    /// arbiter recorded at its last scoring write. THE UI-4 read surface
    /// in probe form (the B7-0-before-B9 precedent: data before display).
    pub fn bastion_colonist_last_scores(&self, name: &str) -> Option<(f32, f32, f32)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let arbiters = ecs.read_storage::<comp::bastion::Arbiter>();
        (&entities, &colonists, &arbiters)
            .join()
            .find(|(_, c, _)| c.0.name == name)
            .map(|(_, _, a)| a.last_scores)
    }

    /// bastion (CHOP-PROGRESS-INDICATOR, harness hook): the colonist's
    /// current work job + progress fraction (the inspector's "Doing" line
    /// source) — `Some((WorkType, 0..1))` while on a progress-bearing work
    /// job, `None` when idle/self-job. The scenario asserts a cutting
    /// colonist reports `Some((Chop, >0))`.
    pub fn bastion_colonist_activity(
        &self,
        name: &str,
    ) -> Option<(common::bastion::WorkType, f32)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let arbiters = ecs.read_storage::<comp::bastion::Arbiter>();
        (&entities, &colonists, &arbiters)
            .join()
            .find(|(_, c, _)| c.0.name == name)
            .and_then(|(_, _, a)| a.activity)
    }

    /// bastion (UI-5, row 62.2): the CELL-inspect resolution as a probe — the
    /// harness's "data before display" read for the Universal Debug Inspector.
    /// Resolves a world cell to the Bastion object in its XY column (job →
    /// stockpile+contents → farm → fell-set → None), exactly as the live wire
    /// path (`sys::msg::in_game::resolve_cell_inspect`) does; keep the two in
    /// lockstep. READ-ONLY.
    pub fn bastion_inspect_cell(
        &self,
        cell: Vec3<i32>,
    ) -> Option<common::comp::bastion::BastionInspectKind> {
        use common::{
            comp::bastion::{
                BastionFarmInspect, BastionFellSetInspect, BastionInspectKind, BastionJobInspect,
                BastionStockpileInspect,
            },
            vol::ReadVol,
        };
        use specs::Join;
        let ecs = self.state.ecs();
        let board = ecs.read_resource::<crate::bastion_jobs::JobBoard>();
        let id_maps = ecs.read_resource::<common::uid::IdMaps>();
        let colonists = ecs.read_storage::<comp::Colonist>();

        // 1. An active job in the clicked XY column (nearest z in a window).
        if let Some(job) = board
            .jobs
            .values()
            .filter(|j| j.pos.x == cell.x && j.pos.y == cell.y && (j.pos.z - cell.z).abs() <= 6)
            .min_by_key(|j| (j.pos.z - cell.z).abs())
        {
            let claimant = job.claimed_by.and_then(|uid| {
                id_maps
                    .uid_entity(uid)
                    .and_then(|e| colonists.get(e))
                    .map(|c| c.0.name.clone())
            });
            return Some(BastionInspectKind::Job(BastionJobInspect {
                work: job.work,
                pos: job.pos,
                progress: job.progress,
                claimant,
                unreachable: job.unreachable,
                needs_materials: job.needs_materials,
                is_access: job.is_access,
                stuck_strikes: job.stuck_strikes,
            }));
        }

        // 2. A stockpile zone → its contents (grouped by item, most first).
        if let Some(zid) = board.stockpile_at(cell)
            && let Some((_, region)) = board.stockpiles.iter().find(|(id, _)| *id == zid)
        {
            let items = ecs.read_storage::<comp::PickupItem>();
            let positions = ecs.read_storage::<comp::Pos>();
            let mut tally: Vec<(String, u32)> = Vec::new();
            let mut total = 0u32;
            for (item, pos) in (&items, &positions).join() {
                let ip = pos.0.map(|e| e.floor() as i32);
                if !region.contains_point_xy(ip) {
                    continue;
                }
                let amount = item.amount() as u32;
                total += amount;
                let def = item
                    .item()
                    .item_definition_id()
                    .itemdef_id()
                    .unwrap_or("?")
                    .to_string();
                if let Some(slot) = tally.iter_mut().find(|(d, _)| *d == def) {
                    slot.1 += amount;
                } else {
                    tally.push((def, amount));
                }
            }
            tally.sort_by(|a, b| b.1.cmp(&a.1));
            return Some(BastionInspectKind::Stockpile(BastionStockpileInspect {
                contents: tally,
                total,
            }));
        }

        // 3. A farm plot → the sampled cell's crop growth stage.
        if let Some((_, region)) = board.farms.iter().find(|(_, r)| r.contains_point_xy(cell)) {
            let growth = self.state.terrain().get(cell).ok().and_then(|b| {
                b.get_attr::<common::terrain::sprite::Growth>()
                    .ok()
                    .map(|g| g.0)
            });
            let cells = ((region.max.x - region.min.x + 1).max(0)
                * (region.max.y - region.min.y + 1).max(0)) as u32;
            return Some(BastionInspectKind::Farm(BastionFarmInspect {
                growth,
                cells,
            }));
        }

        // 4. A registered fell-set (a tree queued / mid-timber).
        if let Some(fell) = board
            .chop_fell_sets
            .values()
            .find(|cf| cf.cells.iter().any(|c| c.x == cell.x && c.y == cell.y))
        {
            return Some(BastionInspectKind::FellSet(BastionFellSetInspect {
                remaining: fell.cells.len() as u32,
                total: fell.wood_count,
            }));
        }

        None
    }

    /// bastion (AUTON-3, harness hook): the urgency-modulation
    /// personality axes for a named colonist — `(adventurous, worried,
    /// sociable_or_extroverted, introverted)`; the scenario predicts
    /// scores with the mechanism's own pub fn from these (mirror-free).
    pub fn bastion_colonist_personality4(&self, name: &str) -> Option<(bool, bool, bool, bool)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
        let rtsim = ecs.read_resource::<crate::rtsim::RtSim>();
        let data = rtsim.state().data();
        (&entities, &colonists, &rtsim_entities)
            .join()
            .find(|(_, c, _)| c.0.name == name)
            .and_then(|(_, _, re)| data.npcs.get(*re))
            .map(|npc| {
                use common::rtsim::PersonalityTrait as PT;
                (
                    npc.personality.is(PT::Adventurous),
                    npc.personality.is(PT::Worried),
                    npc.personality.is(PT::Sociable) || npc.personality.is(PT::Extroverted),
                    npc.personality.is(PT::Introverted),
                )
            })
    }

    /// bastion (AUTON-1, harness hook): queue a BUILD PLAN — intent only,
    /// no jobs (the generator pass owns job creation). Returns the plan's
    /// frozen cell count.
    pub fn bastion_queue_build_plan(&mut self, region: common::bastion::Region) -> usize {
        let ecs = self.state.ecs();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        board.queue_build_plan(&terrain, region.normalized())
    }

    /// bastion (AUTON-1, harness hook): generator telemetry —
    /// `(gen_mine_jobs, gen_build_jobs, plans_completed, open_plans,
    /// pending_mine, pending_build)`. The scenario's bound + quiescence
    /// asserts read these.
    pub fn bastion_selfgen_stats(&self) -> (u64, u64, u64, usize, usize, usize) {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        let pending_mine = board
            .jobs
            .values()
            .filter(|j| j.kind.is(common::bastion::DesignationKind::Mine))
            .count();
        let pending_build = board
            .jobs
            .values()
            .filter(|j| j.kind.is(common::bastion::DesignationKind::Build))
            .count();
        (
            board.gen_mine_jobs,
            board.gen_build_jobs,
            board.plans_completed,
            board.plans.len(),
            pending_mine,
            pending_build,
        )
    }

    /// bastion (AUTON-2, harness hook): a named colonist's rolled
    /// temperament `(conscientious, neurotic)` — READ-ONLY off the same
    /// rtsim data the stagger reads. The spiral scenario ASSIGNS its
    /// designed value-spread around the seed's actual rolls and asserts
    /// against each colonist's exact effective threshold (the first
    /// draw's lesson: personality stacks on values, so group labels
    /// can't predict who preempts — the computed threshold can).
    pub fn bastion_colonist_temperament(&self, name: &str) -> Option<(bool, bool)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
        let rtsim = ecs.read_resource::<crate::rtsim::RtSim>();
        let data = rtsim.state().data();
        (&entities, &colonists, &rtsim_entities)
            .join()
            .find(|(_, c, _)| c.0.name == name)
            .and_then(|(_, _, re)| data.npcs.get(*re))
            .map(|npc| {
                (
                    npc.personality
                        .is(common::rtsim::PersonalityTrait::Conscientious),
                    npc.personality
                        .is(common::rtsim::PersonalityTrait::Neurotic),
                )
            })
    }

    /// bastion (AUTON-2, harness hook): live Despond self-jobs on the
    /// board — READ-ONLY (Despond holds are ordinary board jobs, so the
    /// past-band "keeps re-firing" assert needs no B7-3 counter). Seen
    /// >0 in two separated windows = the breakdown staircase cycled.
    pub fn bastion_despond_jobs(&self) -> usize {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        board
            .jobs
            .values()
            .filter(|j| matches!(j.kind, common::bastion::JobKind::Despond { .. }))
            .count()
    }

    /// bastion (HIST-1, harness hook): chronicle capture vitals —
    /// `(death_entries, last_death_actor_count, theft_entries,
    /// theft_pos_ok, reports_len)`. Reports ride along so the sibling
    /// sink's continued firing is assertable (the regression-free half of
    /// the done-when).
    pub fn bastion_hist1_probe(&self) -> (usize, usize, usize, bool, usize) {
        let ecs = self.state.ecs();
        let rtsim = ecs.read_resource::<crate::rtsim::RtSim>();
        let data = rtsim.state().data();
        let mut death_count = 0;
        let mut death_actors = 0;
        let mut theft_count = 0;
        let mut theft_ok = false;
        for e in data.chronicle.events() {
            match e.kind {
                ::rtsim::data::ChronicleKind::Death => {
                    death_count += 1;
                    death_actors = e.actors.len();
                },
                ::rtsim::data::ChronicleKind::Theft => {
                    theft_count += 1;
                    theft_ok = e.pos.is_some();
                },
                _ => {},
            }
        }
        (
            death_count,
            death_actors,
            theft_count,
            theft_ok,
            data.reports.iter().count(),
        )
    }

    /// bastion (HIST-1, harness hook): fire the REAL theft hook — the
    /// same `hook_pickup_owned_sprite` the interaction handler calls —
    /// with a named colonist as the thief at its own feet. Tests the
    /// event→both-sinks binding through vanilla's own emission path.
    pub fn bastion_emit_test_theft(&mut self, name: &str) -> bool {
        use specs::Join;
        let resolved = {
            let ecs = self.state.ecs();
            let entities = ecs.entities();
            let colonists = ecs.read_storage::<comp::Colonist>();
            let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
            let positions = ecs.read_storage::<comp::Pos>();
            (&entities, &colonists, &rtsim_entities, &positions)
                .join()
                .find(|(_, c, _, _)| c.0.name == name)
                .map(|(_, _, re, pos)| {
                    (
                        common::rtsim::Actor::Npc(*re),
                        pos.0.map(|e| e.floor() as i32),
                    )
                })
        };
        let Some((actor, wpos)) = resolved else {
            return false;
        };
        let index = self.index.as_index_ref();
        self.state
            .ecs()
            .write_resource::<crate::rtsim::RtSim>()
            .hook_pickup_owned_sprite(
                &self.world,
                index,
                common::terrain::sprite::SpriteKind::Crate,
                wpos,
                actor,
            );
        true
    }

    /// bastion (49.2/B37, harness hook): board vitals for the haul-pinning
    /// scenario — `(next_id, live_reservations)`. `next_id` bumps once per
    /// job creation, so its delta counts re-emissions exactly (no racy
    /// transition polling); the reservation count proves drops FREE their
    /// items (a re-emit is only possible against an unreserved item).
    pub fn bastion_board_probe(&self) -> (u64, usize) {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        (board.probe_next_id(), board.probe_reservations())
    }

    /// bastion (B5.6b-2, harness hook): place a designation via the
    /// surface-relative path — the same per-column resolution the in-game
    /// paint message uses. Returns (created job ids, resolved echo bounds)
    /// so scenarios can assert the echo-bounds invariant and per-column
    /// coverage directly.
    pub fn bastion_place_designation_surface(
        &mut self,
        min_xy: vek::Vec2<i32>,
        max_xy: vek::Vec2<i32>,
        hint_z: i32,
        extent: common::bastion::ZExtent,
        kind: common::bastion::DesignationKind,
    ) -> (Vec<common::bastion::JobId>, Option<common::bastion::Region>) {
        let ecs = self.state.ecs();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        let bounds = bastion_jobs::resolve_surface_bounds(&terrain, min_xy, max_xy, hint_z, extent);
        let created =
            board.place_designation_surface(&terrain, min_xy, max_xy, hint_z, extent, kind);
        (created, bounds)
    }

    /// bastion (CHOP redesign FR10, harness hook): run the SAME whole-tree
    /// detection the paint handler runs ([`bastion_chop::detect_trees`] — one
    /// implementation, registry B17) over an XY footprint and place the
    /// fell-set jobs. Returns `(trees, cells, jobs created)`.
    pub fn bastion_place_chop_area(
        &mut self,
        min_xy: vek::Vec2<i32>,
        max_xy: vek::Vec2<i32>,
    ) -> (usize, usize, usize, Option<common::bastion::Region>) {
        let ecs = self.state.ecs();
        let world = ecs.read_resource::<Arc<World>>();
        let index = ecs.read_resource::<IndexOwned>();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        let trees = bastion_chop::detect_trees(&world, &index, &terrain, min_xy, max_xy);
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        let (mut cells_total, mut jobs) = (0, 0);
        for (_aabb, base, cells) in &trees {
            cells_total += cells.len();
            // CHOP-FELLING (row 51.6): ONE base-cut job per tree.
            jobs += board.place_chop_fell(&terrain, *base, cells).is_some() as usize;
        }
        (
            trees.len(),
            cells_total,
            jobs,
            trees.first().map(|(aabb, _, _)| *aabb),
        )
    }

    /// bastion (CHOP-FELLING, harness hook): place a base-cut from an
    /// EXPLICIT base — the oracle-free path for the fixture-built trees the
    /// test_world can't grow (the oracle half degrades under the stub
    /// World; the fell-set flood + placement below are the REAL shipping
    /// fns — B17: the tested path is the shipping path from the flood in).
    /// Returns `(fell-set size, wood count, size-scaled threshold, job
    /// created)` — the threshold is the deterministic size-scaling proof
    /// (`CHOP_WORK_PER_BLOCK × wood_count`), travel-free.
    pub fn bastion_place_chop_tree(&mut self, base: vek::Vec3<i32>) -> (usize, u32, f32, bool) {
        use common::vol::ReadVol;
        let ecs = self.state.ecs();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        let is_tree = |p: vek::Vec3<i32>| {
            terrain
                .get(p)
                .map(|b| {
                    matches!(
                        b.kind(),
                        common::terrain::BlockKind::Wood | common::terrain::BlockKind::Leaves
                    )
                })
                .unwrap_or(false)
        };
        let cells = bastion_jobs::tree_fell_set(
            &is_tree,
            base,
            bastion_jobs::TREE_FELL_CELL_CAP,
            bastion_jobs::TREE_FELL_HEIGHT_CAP,
            bastion_jobs::TREE_FELL_RADIUS,
        );
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        let id = board.place_chop_fell(&terrain, base, &cells);
        let (wood, threshold, created) = match id {
            Some(id) => board
                .chop_fell_sets
                .get(&id)
                .map_or((0, 0.0, true), |f| (f.wood_count, f.threshold, true)),
            None => (0, 0.0, false),
        };
        (cells.len(), wood, threshold, created)
    }

    /// bastion (CHOP-FELLING, harness probe): `(stored fell-sets, trees
    /// mid-fall, cells remaining across all falls)` — read-only.
    pub fn bastion_chop_fell_stats(&self) -> (usize, usize, usize) {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        (
            board.chop_fell_sets.len(),
            board.felling.len(),
            board.felling.iter().map(|t| t.cells.len() - t.cursor).sum(),
        )
    }

    /// bastion (B5.8, harness hook): positions of currently-claimed jobs —
    /// lets scenarios assert work-crew dispersion across the dig frontier.
    pub fn bastion_claimed_job_positions(&self) -> Vec<vek::Vec3<i32>> {
        // is_access excluded (B5.8-E3): scenarios use this to measure
        // DESIGNATION-work invariants (crew dispersion). The system's own
        // access scaffolding (stair steps/rungs) is adjacent-by-
        // construction — counting it read as "clumped claims" whenever a
        // rescue plan ran (tool0-gate: d_dispersed collapsed to ~0.43 on
        // healthy runs at the slower work pace).
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .jobs
            .values()
            .filter(|j| j.claimed_by.is_some() && !j.is_access)
            .map(|j| j.pos)
            .collect()
    }

    /// bastion (B5.8, harness hook): the sprite at a block, so scenarios can
    /// assert ladder rungs landed.
    pub fn bastion_block_sprite(&self, pos: vek::Vec3<i32>) -> Option<common::terrain::SpriteKind> {
        use common::vol::ReadVol;
        self.state
            .ecs()
            .read_resource::<common::terrain::TerrainGrid>()
            .get(pos)
            .ok()
            .and_then(|b| b.get_sprite())
    }

    /// bastion (B5.6b-2, harness hook): the per-column surface the placement
    /// path would resolve for (x, y) around `hint_z` — lets scenarios assert
    /// coverage column-by-column against the same authority.
    pub fn bastion_column_surface_z(&self, x: i32, y: i32, hint_z: i32) -> Option<i32> {
        let terrain = self
            .state
            .ecs()
            .read_resource::<common::terrain::TerrainGrid>();
        bastion_jobs::column_surface_z(&terrain, x, y, hint_z)
    }

    /// bastion (B4, harness hook): cancel designations in a region.
    pub fn bastion_cancel_designation(&mut self, region: common::bastion::Region) {
        self.state
            .ecs()
            .write_resource::<bastion_jobs::JobBoard>()
            .cancel_region(region.normalized());
    }

    /// bastion (B4, harness hook): job-board audit snapshot.
    pub fn bastion_job_audit(&self) -> common::bastion::JobAudit {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .audit()
    }

    /// bastion (B4, harness hook): per-colonist state (name, position,
    /// claimed job + travel state) for loaded colonists.
    pub fn bastion_colonist_states(&self) -> Vec<(String, Vec3<f32>, Option<(u64, bool)>)> {
        self.bastion_colonist_states_full()
            .into_iter()
            .map(|(_, n, p, j)| (n, p, j))
            .collect()
    }

    /// bastion (B6, harness hook): give every loaded colonist a UNIQUE
    /// deterministic name (`Colonist-0`, `Colonist-1`, …). Random spawn
    /// names collide (~1/24 per pair — with a 5-colonist crew that's a
    /// meaningful chance every run), and every name-keyed scenario check
    /// (fs/quarry/pit lures, ever-out sets) then tracks the wrong
    /// colonist → intermittent false failures. Call once after spawn.
    /// Returns the new names in join order.
    pub fn bastion_rename_colonists_unique(&mut self) -> Vec<String> {
        use specs::LendJoin;
        let ecs = self.state.ecs();
        let mut colonists = ecs.write_storage::<comp::Colonist>();
        let mut names = Vec::new();
        let mut i = 0;
        let mut iter = (&mut colonists).lend_join();
        while let Some(mut c) = iter.next() {
            let name = format!("Colonist-{i}");
            c.0.name = name.clone();
            names.push(name);
            i += 1;
        }
        names
    }

    /// bastion (B6, harness hook): colonist states WITH the entity `Uid` —
    /// names are randomly drawn and CAN collide (chokepoint run-23: two
    /// "Yara of the Vale"s collapsed a 5-colonist roster to 4 in every
    /// name-keyed assert). Identity-sensitive scenario tracking keys on
    /// the uid.
    pub fn bastion_colonist_states_full(
        &self,
    ) -> Vec<(u64, String, Vec3<f32>, Option<(u64, bool)>)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let positions = ecs.read_storage::<comp::Pos>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let jobs = ecs.read_storage::<comp::bastion::ActiveJob>();
        (&colonists, &positions, &uids, jobs.maybe())
            .join()
            .map(|(c, p, u, j)| {
                (
                    u.0.get(),
                    c.0.name.clone(),
                    p.0,
                    j.map(|j| {
                        (
                            j.job,
                            matches!(j.state, comp::bastion::ActiveJobState::Arrived),
                        )
                    }),
                )
            })
            .collect()
    }

    /// bastion (B5.8 ladder-fixture geometry PROBE, harness read): the emitted
    /// emergency `EmergencyTraversalKind` for a colonist by name — "CarvedStair"
    /// (walkable, Phase-1), "ConstructedLadder" (ladder_pillar — the fixture's
    /// target), or "NaturalShaft" (wrong climb kind). Read from the live route
    /// descriptor board. This is the read leg of the architect's 2-part pre-build
    /// proof that a candidate narrow shaft lands on ConstructedLadder, not a
    /// stair and not NaturalShaft. `None` = no emergency route owned yet.
    pub fn bastion_colonist_route_kind(&self, name: &str) -> Option<String> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let board = ecs.read_resource::<crate::bastion_jobs::JobBoard>();
        (&colonists, &uids)
            .join()
            .find(|(c, _)| c.0.name == name)
            .and_then(|(_, u)| board.emergency_route_descriptors.get(u))
            .map(|d| format!("{:?}", d.kind))
    }

    /// bastion (B4, harness hook): set a work priority on a colonist by
    /// name — both the rtsim record and, if loaded, the ECS mirror.
    pub fn bastion_set_work_priority(
        &mut self,
        name: &str,
        work: common::bastion::WorkType,
        priority: u8,
    ) -> bool {
        let mut found = false;
        {
            let ecs = self.state.ecs();
            let mut rtsim = ecs.write_resource::<rtsim::RtSim>();
            found |= rtsim.bastion_set_work_priority(name, work, priority);
        }
        {
            use specs::LendJoin;
            let ecs = self.state.ecs();
            let mut colonists = ecs.write_storage::<comp::Colonist>();
            let mut iter = (&mut colonists).lend_join();
            while let Some(mut colonist) = iter.next() {
                if colonist.0.name == name {
                    colonist.0.work_priorities.set(work, priority);
                    found = true;
                }
            }
        }
        found
    }

    /// bastion (B5, harness hook): give a named colonist one unit of an item
    /// (stands in for B6 hauling — lets the Build path be tested without a
    /// real logistics chain). Returns whether the colonist was found+loaded.
    pub fn bastion_give_colonist_item(&mut self, name: &str, asset_id: &str) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let entities = ecs.entities();
        let target = (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .map(|(e, _)| e);
        drop(colonists);
        let Some(entity) = target else { return false };
        let mut inventories = ecs.write_storage::<comp::Inventory>();
        // Direct single-entity access — flagged-storage restrictions only
        // bite multi-component `.join()`, not a plain `get_mut(entity)`.
        if let Some(mut inv) = inventories.get_mut(entity) {
            let _ = inv.push(common::comp::Item::new_from_asset_expect(asset_id));
            true
        } else {
            false
        }
    }

    /// bastion (B-ASSET1, harness/arena hook): order a named loaded colonist
    /// to walk to `target` (test-goto — same agent Goto mechanism as job
    /// travel, arrival/stuck readable via [`Self::bastion_goto_states`]).
    /// Refuses colonists holding a job (the job system owns their activity).
    pub fn bastion_goto(&mut self, name: &str, target: Vec3<f32>) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let entities = ecs.entities();
        let target_entity = (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .map(|(e, _)| e);
        drop(colonists);
        let Some(entity) = target_entity else {
            return false;
        };
        if ecs
            .read_storage::<comp::bastion::ActiveJob>()
            .contains(entity)
        {
            return false;
        }
        ecs.write_storage::<comp::bastion::BastionTestGoto>()
            .insert(entity, comp::bastion::BastionTestGoto::new(target))
            .is_ok()
    }

    /// bastion (B-ASSET1): every active goto order:
    /// `(name, pos, target, elapsed_s, arrived, stuck)`.
    pub fn bastion_goto_states(&self) -> Vec<(String, Vec3<f32>, Vec3<f32>, f32, bool, bool)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let gotos = ecs.read_storage::<comp::bastion::BastionTestGoto>();
        let positions = ecs.read_storage::<comp::Pos>();
        (&colonists, &gotos, &positions)
            .join()
            .map(|(c, g, p)| {
                (
                    c.0.name.clone(),
                    p.0,
                    g.target,
                    g.elapsed,
                    g.arrived,
                    g.stuck,
                )
            })
            .collect()
    }

    // bastion (B-ASSET1): the teleport-colonist helper now lives further
    // down — the B5.8 merge brought an identical-signature version that also
    // zeroes velocity and forces a chunk resync (physics would lerp long
    // teleports otherwise); asset tests call it unchanged.

    /// bastion (B-ASSET1): clear a named colonist's goto order (`None` = all).
    /// Returns how many were cleared.
    pub fn bastion_goto_clear(&mut self, name: Option<&str>) -> usize {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let entities = ecs.entities();
        let targets: Vec<_> = (&entities, &colonists)
            .join()
            .filter(|(_, c)| name.is_none_or(|n| c.0.name == n))
            .map(|(e, _)| e)
            .collect();
        drop(colonists);
        let mut gotos = ecs.write_storage::<comp::bastion::BastionTestGoto>();
        let mut agents = ecs.write_storage::<comp::Agent>();
        let mut cleared = 0;
        for e in targets {
            if gotos.remove(e).is_some() {
                cleared += 1;
                if let Some(agent) = agents.get_mut(e) {
                    agent.rtsim_controller.activity = None;
                }
            }
        }
        cleared
    }

    /// bastion (B5, harness hook): count loose dropped items near `pos`
    /// (within `radius` blocks) whose item asset id matches `asset_id`.
    pub fn bastion_count_items_near(&self, pos: Vec3<f32>, radius: f32, asset_id: &str) -> usize {
        use specs::Join;
        let ecs = self.state.ecs();
        let items = ecs.read_storage::<comp::PickupItem>();
        let positions = ecs.read_storage::<comp::Pos>();
        (&items, &positions)
            .join()
            .filter(|(item, item_pos)| {
                item_pos.0.distance_squared(pos) <= radius * radius
                    && item.item().item_definition_id().itemdef_id() == Some(asset_id)
            })
            .count()
    }

    /// bastion (B5.5, harness hook): SUM of item amounts (not entity count —
    /// piles carry counts) across loose drops near `pos` matching
    /// `asset_id`. Pass `f32::INFINITY` radius for a system-wide
    /// conservation total.
    pub fn bastion_sum_items_near(&self, pos: Vec3<f32>, radius: f32, asset_id: &str) -> u64 {
        use specs::Join;
        let ecs = self.state.ecs();
        let items = ecs.read_storage::<comp::PickupItem>();
        let positions = ecs.read_storage::<comp::Pos>();
        (&items, &positions)
            .join()
            .filter(|(item, item_pos)| {
                (radius.is_infinite() || item_pos.0.distance_squared(pos) <= radius * radius)
                    && item.item().item_definition_id().itemdef_id() == Some(asset_id)
            })
            .map(|(item, _)| item.amount() as u64)
            .sum()
    }

    /// bastion (B5.5, harness hook): total loose-drop ENTITY count (the pile
    /// aggregation bound: mining N blocks must not carpet the world with N
    /// entities).
    pub fn bastion_pickup_entity_count(&self) -> usize {
        use specs::Join;
        let ecs = self.state.ecs();
        let items = ecs.read_storage::<comp::PickupItem>();
        (&items).join().count()
    }

    /// bastion (B5.5, harness hook): set a colonist's skill level for a work
    /// type — rtsim record and, if loaded, the ECS mirror (same pattern as
    /// `bastion_set_work_priority`). Lets scale scenarios run at high work
    /// rates instead of 3 s/block.
    pub fn bastion_set_colonist_skill(
        &mut self,
        name: &str,
        work: common::bastion::WorkType,
        level: u16,
    ) -> bool {
        let mut found = false;
        {
            let ecs = self.state.ecs();
            let mut rtsim = ecs.write_resource::<rtsim::RtSim>();
            found |= rtsim.bastion_set_colonist_skill(name, work, level);
        }
        {
            use specs::LendJoin;
            let ecs = self.state.ecs();
            let mut colonists = ecs.write_storage::<comp::Colonist>();
            let mut iter = (&mut colonists).lend_join();
            while let Some(mut colonist) = iter.next() {
                if colonist.0.name == name {
                    colonist.0.skills.set_level_for(work, level);
                    found = true;
                }
            }
        }
        found
    }

    /// bastion (B5, harness hook): a named colonist's skill level+xp for the
    /// given work type.
    pub fn bastion_colonist_skill(
        &self,
        name: &str,
        work: common::bastion::WorkType,
    ) -> Option<common::bastion::SkillLevel> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        (&colonists).join().find(|c| c.0.name == name).map(|c| {
            let s = &c.0.skills;
            match work {
                common::bastion::WorkType::Mine => s.mining,
                common::bastion::WorkType::Chop => s.woodcutting,
                common::bastion::WorkType::Build => s.construction,
                common::bastion::WorkType::Haul => s.hauling,
                common::bastion::WorkType::Cook => s.cooking,
                common::bastion::WorkType::Farm => s.farming,
            }
        })
    }

    /// bastion (B5.8, harness hook): teleport a loaded colonist — scenario
    /// STAGING only (parks participants at a test site so vertical-mobility
    /// gates measure the mechanism, not cross-town goto reliability, which
    /// is a separate pre-existing weakness — see B58 findings).
    pub fn bastion_teleport_colonist(&mut self, name: &str, pos: Vec3<f32>) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let entities = ecs.entities();
        let target = (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .map(|(e, _)| e);
        drop(colonists);
        if let Some(entity) = target {
            let mut positions = ecs.write_storage::<comp::Pos>();
            let mut velocities = ecs.write_storage::<comp::Vel>();
            if let Some(p) = positions.get_mut(entity) {
                p.0 = pos;
                if let Some(v) = velocities.get_mut(entity) {
                    v.0 = Vec3::zero();
                }
                // Force a chunk-position resync so physics doesn't lerp the
                // entity across the map.
                let _ = ecs
                    .write_storage::<common::comp::ForceUpdate>()
                    .get_mut(entity)
                    .map(|f| f.update());
                return true;
            }
        }
        false
    }

    /// bastion (B5.8, harness hook): set a loaded colonist's CLIMBING
    /// movement skill (deterministic scramble-reach in scenarios).
    pub fn bastion_set_colonist_climbing(&mut self, name: &str, level: u16) -> bool {
        use specs::LendJoin;
        let ecs = self.state.ecs();
        let mut colonists = ecs.write_storage::<comp::Colonist>();
        let mut found = false;
        let mut iter = (&mut colonists).lend_join();
        while let Some(mut colonist) = iter.next() {
            if colonist.0.name == name {
                colonist.0.skills.climbing.level = level;
                found = true;
            }
        }
        found
    }

    /// bastion (B5.8, harness hook): a loaded colonist's climbing skill —
    /// scenarios assert XP accrues with use.
    pub fn bastion_colonist_climbing(&self, name: &str) -> Option<common::bastion::SkillLevel> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        (&colonists)
            .join()
            .find(|c| c.0.name == name)
            .map(|c| c.0.skills.climbing)
    }

    /// bastion (CAVE-IN v1, harness hook): a colonist's (current, maximum)
    /// health — lets a cave-in scenario assert a crush victim was INJURED
    /// (current < max) but NOT killed / NOT buried (current > 0, still alive).
    pub fn bastion_colonist_health(&self, name: &str) -> Option<(f32, f32)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let healths = ecs.read_storage::<comp::Health>();
        (&colonists, &healths)
            .join()
            .find(|(c, _)| c.0.name == name)
            .map(|(_, h)| (h.current(), h.maximum()))
    }

    /// bastion (CAVE-IN v1, harness hook): a colonist's Mood (0=breakdown..
    /// 1=content) — a cave-in scenario asserts a crush victim was FEARED (Mood
    /// dropped from the 0.6 default). Colonists always carry Mood (rtsim
    /// promote), even the synthetic harness spawns that skip Health.
    pub fn bastion_colonist_mood(&self, name: &str) -> Option<f32> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let moods = ecs.read_storage::<comp::bastion::Mood>();
        (&colonists, &moods)
            .join()
            .find(|(c, _)| c.0.name == name)
            .map(|(_, m)| m.0)
    }

    /// bastion (CAVE-IN v1, harness hook): DETERMINISTICALLY drive the same
    /// mining-remnant collapse the mine-completion path runs — treat
    /// `removed_pos` as just-mined, run the bounded support check, and if a
    /// bounded chunk floats: collapse it (cells → air) AND eject-and-injure
    /// every colonist in the crush volume (nearest safe cell outside the
    /// falling footprint + health damage + Mood fear). Returns the victim
    /// count. Reuses the SAME pure helpers + constants as the system
    /// (`floating_chunk`, `eject_dest`, `CAVEIN_*`) so there is no logic drift;
    /// this exists only so a scenario can place a victim in-crush and fire the
    /// collapse ON THAT TICK (a colonist mining it live wanders off the crush
    /// footprint before completion, so the invariant can't be pinned that way).
    pub fn bastion_force_collapse_check(&mut self, removed_pos: Vec3<i32>) -> usize {
        use common::vol::ReadVol;
        // 1. The floating chunk (floating_chunk reads removed_pos AS air).
        let cells = {
            let terrain = self
                .state
                .ecs()
                .read_resource::<common::terrain::TerrainGrid>();
            bastion_jobs::floating_chunk(
                |p| terrain.get(p).map(|b| b.is_filled()).unwrap_or(false),
                removed_pos,
                bastion_jobs::CAVEIN_SUPPORT_CAP,
            )
        };
        let Some(cells) = cells else {
            return 0;
        };
        // 2. Collapse: remove the mined block + the floating chunk.
        self.state
            .set_block(removed_pos, common::terrain::Block::empty());
        for &c in &cells {
            self.state.set_block(c, common::terrain::Block::empty());
        }
        // 3. Eject-and-injure the crush volume — the SAME shared fn the live
        //    mine-completion path runs (reviewer R8/F-CAVE-3: the tested path IS the
        //    shipping path; no parallel copy to drift).
        let ecs = self.state.ecs();
        let time = *ecs.read_resource::<common::resources::Time>();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let mut positions = ecs.write_storage::<comp::Pos>();
        let mut velocities = ecs.write_storage::<comp::Vel>();
        let mut healths = ecs.write_storage::<comp::Health>();
        let mut moods = ecs.write_storage::<comp::bastion::Mood>();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        let victims = bastion_jobs::cavein_eject_and_injure(
            &cells,
            &terrain,
            time,
            &entities,
            &colonists,
            &mut positions,
            &mut velocities,
            &mut healths,
            &mut moods,
        );
        // B7-0: queue the fear thoughts EXACTLY like the live mine-
        // completion caller — the deterministic test hook must not
        // silently skip the emitter (R8's tested-path-IS-shipping-path
        // includes the thought; the cavein leg's fear-persists assert
        // rides this).
        {
            let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
            let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
            for e in &victims {
                if let (Some(re), Some(p)) = (rtsim_entities.get(*e), positions.get(*e)) {
                    board.pending_thoughts.push((
                        *re,
                        p.0.map(|v| v.floor() as i32),
                        ::rtsim::data::ChronicleKind::CaveIn,
                    ));
                }
            }
        }
        victims.len()
    }

    /// bastion (B-LIVE3, harness hook): designations completed (mine-done
    /// lifecycle) since server start.
    pub fn bastion_done_designations(&self) -> u64 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .done_count
    }

    /// bastion (DETRNG belt, harness hook): cumulative cave-in collapse drop
    /// cells — the conservation companion for stone accounting.
    pub fn bastion_cavein_drop_cells(&self) -> u64 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .cavein_drop_cells
    }

    /// bastion (CASE-003, harness hook): total phys CENTER-SAFETY-NET fires
    /// (process-global, monotonic). REPORTED telemetry — with the writer-side
    /// bugs fixed this sits at 0; any climb marks a NEW embedding writer.
    pub fn bastion_center_net_fires(&self) -> u64 {
        common::bastion::CENTER_NET_FIRES.load(core::sync::atomic::Ordering::Relaxed)
    }

    /// bastion (FR15 instrumentation, harness hook): locomotion baseline
    /// counters `(no_progress_ticks, travel_timeouts, failsafe_teleports)` —
    /// REPORTED telemetry for the before/after fix-1/fix-2 comparison.
    pub fn bastion_locomotion_stats(&self) -> (u64, u64, u64) {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        (
            board.no_progress_ticks,
            board.travel_timeouts,
            board.failsafe_teleports,
        )
    }

    /// bastion (B5.5 deep, harness probe): full source attribution for every
    /// ultimate fail-safe teleport observed by this server.
    pub fn bastion_failsafe_events(&self) -> Vec<bastion_jobs::FailsafeTeleportEvent> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .failsafe_events
            .clone()
    }

    /// REQ-0040 harness proof that temporary humanitarian access leaves no
    /// jobs, terrain provenance, or deferred cleanup behind.
    pub fn bastion_emergency_access_stats(&self) -> (usize, usize, usize) {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        (
            board.emergency_access_jobs.len(),
            board.emergency_access_cells.len(),
            board.emergency_cleanup_pending.len()
                + board.emergency_safe_secs.len()
                + board.emergency_route_members.len()
                + board.emergency_route_targets.len()
                + board.emergency_route_mounts.len()
                + board.emergency_route_sequences.len()
                + board.bastion_traversal_tasks.len()
                + board.emergency_partial_route_entries.len()
                + board.emergency_settle_anchors.len(),
        )
    }

    /// REQ-0041 diagnostic detail for a residual emergency route. This is
    /// read-only harness telemetry: job id, owner uid, target, claimant,
    /// unreachable flag, and accumulated work progress.
    pub fn bastion_emergency_access_details(
        &self,
    ) -> Vec<(
        u64,
        u64,
        Vec3<i32>,
        Option<u64>,
        bool,
        f32,
        Option<Vec3<f32>>,
        Option<(u64, bool)>,
    )> {
        use specs::Join;
        let ecs = self.state.ecs();
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let positions = ecs.read_storage::<comp::Pos>();
        let active = ecs.read_storage::<comp::bastion::ActiveJob>();
        let mut details: Vec<_> = board
            .emergency_access_jobs
            .iter()
            .filter_map(|(id, owner)| {
                board.jobs.get(id).map(|job| {
                    let owner_state = (&uids, &positions, active.maybe())
                        .join()
                        .find(|(uid, _, _)| **uid == *owner)
                        .map(|(_, position, active)| {
                            (
                                position.0,
                                active.map(|active| {
                                    (
                                        active.job,
                                        matches!(
                                            active.state,
                                            comp::bastion::ActiveJobState::Arrived
                                        ),
                                    )
                                }),
                            )
                        });
                    (
                        *id,
                        owner.0.get(),
                        job.pos,
                        job.claimed_by.map(|uid| uid.0.get()),
                        job.unreachable,
                        job.progress,
                        owner_state.map(|state| state.0),
                        owner_state.and_then(|state| state.1),
                    )
                })
            })
            .collect();
        details.sort_by_key(|detail| detail.0);
        details
    }

    /// REQ-0046 read-only cleanup diagnostics. The aggregate third stat mixes
    /// three different lifetimes, so expose them separately together with
    /// member positions and provenance-cell ownership.
    pub fn bastion_emergency_cleanup_details(
        &self,
    ) -> (
        Vec<u64>,
        Vec<(u64, u64, Option<Vec3<f32>>, Option<Vec3<i32>>, Option<f32>)>,
        Vec<(u64, f32)>,
        Vec<(u64, Vec<Vec3<i32>>)>,
    ) {
        use specs::Join;
        let ecs = self.state.ecs();
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let positions = ecs.read_storage::<comp::Pos>();
        let mut pending: Vec<_> = board
            .emergency_cleanup_pending
            .iter()
            .map(|uid| uid.0.get())
            .collect();
        pending.sort_unstable();
        let mut members: Vec<_> = board
            .emergency_route_members
            .iter()
            .map(|(member, owner)| {
                let position = (&uids, &positions)
                    .join()
                    .find(|(uid, _)| **uid == *member)
                    .map(|(_, position)| position.0);
                (
                    member.0.get(),
                    owner.0.get(),
                    position,
                    board.egress_targets.get(member).copied(),
                    board.emergency_safe_secs.get(member).copied(),
                )
            })
            .collect();
        members.sort_by_key(|detail| detail.0);
        let mut safe: Vec<_> = board
            .emergency_safe_secs
            .iter()
            .map(|(uid, seconds)| (uid.0.get(), *seconds))
            .collect();
        safe.sort_by_key(|detail| detail.0);
        let mut cells_by_owner: HashMap<u64, Vec<Vec3<i32>>> = HashMap::new();
        for (cell, (owner, _)) in &board.emergency_access_cells {
            cells_by_owner.entry(owner.0.get()).or_default().push(*cell);
        }
        for cells in cells_by_owner.values_mut() {
            cells.sort_by_key(|cell| (cell.x, cell.y, cell.z));
        }
        let mut cells_by_owner: Vec<_> = cells_by_owner.into_iter().collect();
        cells_by_owner.sort_by_key(|detail| detail.0);
        (pending, members, safe, cells_by_owner)
    }

    /// bastion (LOD-0, harness hook): force-DEMOTE a loaded colonist — flip
    /// the rtsim mode to Simulated AND delete the live entity in one step.
    /// (A bare mode flip cannot demote: the "Load in NPCs" pass runs BEFORE
    /// the sync loop each tick and flips a Simulated npc in a loaded chunk
    /// straight back to Loaded — the real demote is chunk-unload-driven.)
    /// The per-tick save-back mirror holds the colonist's state as of the
    /// last completed tick, so the deletion loses nothing; the load pass
    /// then RE-CREATES + RE-PROMOTES from the persisted record — the exact
    /// promote-restores-persisted-state path LOD-0 must prove.
    pub fn bastion_force_demote(&mut self, name: &str) -> bool {
        use specs::Join;
        if !self
            .state
            .ecs()
            .write_resource::<rtsim::RtSim>()
            .bastion_force_demote(name)
        {
            return false;
        }
        let entity = {
            let ecs = self.state.ecs();
            let entities = ecs.entities();
            let colonists = ecs.read_storage::<comp::Colonist>();
            (&entities, &colonists)
                .join()
                .find(|(_, c)| c.0.name == name)
                .map(|(e, _)| e)
        };
        match entity {
            Some(e) => {
                if let Err(err) = self.state.delete_entity_recorded(e) {
                    tracing::warn!(?err, "bastion LOD-0: force-demote delete failed");
                    false
                } else {
                    true
                }
            },
            None => false,
        }
    }

    /// bastion (B6, harness fixture): spawn a loose persistent item drop at
    /// a position (test fixtures — e.g. seeding a stockpile with exactly one
    /// material for the reservation race test).
    pub fn bastion_spawn_item(&mut self, pos: Vec3<f32>, asset_id: &str, amount: u32) -> bool {
        self.bastion_spawn_item_class(pos, asset_id, amount, true)
    }

    /// bastion (B5.5 deep, harness fixture): spawn a loose item in an
    /// explicit lifetime/merge class. `persistent=true` is a Bastion pile
    /// with no despawn timer; `false` is vanilla timed loot. This is kept a
    /// fixture hook so the adversarial gate exercises the shipping event and
    /// item systems rather than fabricating ECS components.
    pub fn bastion_spawn_item_class(
        &mut self,
        pos: Vec3<f32>,
        asset_id: &str,
        amount: u32,
        persistent: bool,
    ) -> bool {
        let Ok(mut item) = comp::Item::new_from_asset(asset_id) else {
            return false;
        };
        if amount > 1 && item.set_amount(amount).is_err() {
            return false;
        }
        let ecs = self.state.ecs();
        let program_time = *ecs.read_resource::<common::resources::ProgramTime>();
        ecs.read_resource::<common::event::EventBus<common::event::CreateItemDropEvent>>()
            .emit_now(common::event::CreateItemDropEvent {
                pos: comp::Pos(pos),
                vel: comp::Vel(Vec3::zero()),
                ori: comp::Ori::default(),
                item: comp::PickupItem::new(item, program_time, true),
                loot_owner: None,
                persistent,
            });
        true
    }

    /// B5.5 deep control fixture: spawn vanilla timed loot with a hard,
    /// nonexistent owner for longer than the accelerated soak's wall time.
    /// This keeps ambient humanoids from moving the 37-unit expiry control
    /// into an inventory, where loose-entity lifetime components cannot be
    /// observed, without changing shipping item expiry or merge behavior.
    pub fn bastion_spawn_isolated_timed_item(
        &mut self,
        pos: Vec3<f32>,
        asset_id: &str,
        amount: u32,
    ) -> bool {
        let Ok(mut item) = comp::Item::new_from_asset(asset_id) else {
            return false;
        };
        if amount > 1 && item.set_amount(amount).is_err() {
            return false;
        }
        let Some(fake_uid) = std::num::NonZeroU64::new(u64::MAX) else {
            return false;
        };
        let ecs = self.state.ecs();
        let program_time = *ecs.read_resource::<common::resources::ProgramTime>();
        ecs.read_resource::<common::event::EventBus<common::event::CreateItemDropEvent>>()
            .emit_now(common::event::CreateItemDropEvent {
                pos: comp::Pos(pos),
                vel: comp::Vel(Vec3::zero()),
                ori: comp::Ori::default(),
                item: comp::PickupItem::new(item, program_time, true),
                loot_owner: Some(comp::LootOwner::new(
                    comp::loot_owner::LootOwnerKind::Player(common::uid::Uid(fake_uid)),
                    false,
                    600,
                    *ecs.read_resource::<common::resources::Time>(),
                )),
                persistent: false,
            });
        true
    }

    /// bastion (B5.5 deep, harness probe): item amounts/entities split by
    /// the real `BastionPile` merge class, plus lifetime-component mismatch
    /// counts. Tuple fields are `(persistent_amount, persistent_entities,
    /// timed_amount, timed_entities, persistent_with_timer,
    /// timed_without_timer)`.
    pub fn bastion_item_class_summary_near(
        &self,
        pos: Vec3<f32>,
        radius: f32,
        asset_id: &str,
    ) -> (u64, usize, u64, usize, usize, usize) {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let items = ecs.read_storage::<comp::PickupItem>();
        let positions = ecs.read_storage::<comp::Pos>();
        let piles = ecs.read_storage::<comp::bastion::BastionPile>();
        let objects = ecs.read_storage::<comp::Object>();
        let mut out = (0, 0, 0, 0, 0, 0);
        for (entity, item, item_pos) in (&entities, &items, &positions).join() {
            if item_pos.0.distance_squared(pos) > radius * radius
                || item.item().item_definition_id().itemdef_id() != Some(asset_id)
            {
                continue;
            }
            let persistent = piles.contains(entity);
            let timed = matches!(objects.get(entity), Some(comp::Object::DeleteAfter { .. }));
            if persistent {
                out.0 += item.amount() as u64;
                out.1 += 1;
                out.4 += usize::from(timed);
            } else {
                out.2 += item.amount() as u64;
                out.3 += 1;
                out.5 += usize::from(!timed);
            }
        }
        out
    }

    /// bastion (B5.5 deep, diagnostic probe): canonical snapshots of all
    /// persistent piles for one item definition. Entity ids let the harness
    /// exclude its pre-existing control piles and follow the 1,000-cell
    /// cohort through periodic merges without changing shipping components.
    pub fn bastion_persistent_item_snapshots(&self, asset_id: &str) -> Vec<(u32, u64, Vec3<f32>)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let items = ecs.read_storage::<comp::PickupItem>();
        let positions = ecs.read_storage::<comp::Pos>();
        let piles = ecs.read_storage::<comp::bastion::BastionPile>();
        let mut out: Vec<_> = (&entities, &items, &positions, &piles)
            .join()
            .filter(|(_, item, _, _)| {
                item.item().item_definition_id().itemdef_id() == Some(asset_id)
            })
            .map(|(entity, item, pos, _)| (entity.id(), item.amount() as u64, pos.0))
            .collect();
        out.sort_by_key(|(entity_id, _, _)| *entity_id);
        out
    }

    /// bastion (B5.5 deep, harness oracle): every live inventory that
    /// contains the requested item, including non-colonist RTSim actors.
    /// The tuple is `(entity_id, uid, debug_name, is_colonist, is_player,
    /// is_rtsim, amount)` and is sorted by entity id for stable evidence.
    pub fn bastion_inventory_item_snapshots(
        &self,
        asset_id: &str,
    ) -> Vec<(u32, Option<u64>, String, bool, bool, bool, u64)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let inventories = ecs.read_storage::<comp::Inventory>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let stats = ecs.read_storage::<comp::Stats>();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let players = ecs.read_storage::<comp::Player>();
        let rtsim_entities = ecs.read_storage::<common::rtsim::RtSimEntity>();
        let mut out = Vec::new();
        for (entity, inventory) in (&entities, &inventories).join() {
            let amount = inventory
                .slots()
                .flatten()
                .filter(|item| item.item_definition_id().itemdef_id() == Some(asset_id))
                .map(|item| item.amount() as u64)
                .sum::<u64>();
            if amount == 0 {
                continue;
            }
            out.push((
                entity.id(),
                uids.get(entity).map(|uid| uid.0.get()),
                stats
                    .get(entity)
                    .map(|stats| format!("{:?}", stats.name))
                    .unwrap_or_default(),
                colonists.contains(entity),
                players.contains(entity),
                rtsim_entities.contains(entity),
                amount,
            ));
        }
        out.sort_by_key(|(entity_id, _, _, _, _, _, _)| *entity_id);
        out
    }

    /// bastion (LOD-0, harness hook): the named colonist's LIVE bag
    /// inventory in canonical `(id, amount)` form — built by the SAME
    /// `colonist_record` the save-back uses (B17), for exact conservation
    /// asserts across promote cycles (no loss, no dupe).
    pub fn bastion_colonist_inventory(&self, name: &str) -> Option<Vec<(String, u32)>> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let inventories = ecs.read_storage::<comp::Inventory>();
        (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .and_then(|(e, c)| {
                crate::rtsim::tick::colonist_record(c, inventories.get(e), None, None).inventory
            })
    }

    /// Registry class 7 fixture read: observe the named colonist's live bag
    /// with the same item identity and slot fields used by the focused lazy
    /// loadout fixture. This is read-only and does not alter inventory order.
    pub fn bastion_colonist_item_observations(
        &self,
        name: &str,
    ) -> Option<Vec<crate::rtsim::tick::Class7ItemObservation>> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let inventories = ecs.read_storage::<comp::Inventory>();
        (&entities, &colonists)
            .join()
            .find(|(_, colonist)| colonist.0.name == name)
            .and_then(|(entity, _)| inventories.get(entity))
            .map(crate::rtsim::tick::class7_inventory_observations)
    }

    /// Registry class 7 fixture read: expose the exact slot the production
    /// Agent healing rule would choose for this live colonist.
    pub fn bastion_colonist_selected_healing_item(
        &self,
        name: &str,
    ) -> Option<crate::rtsim::tick::Class7ItemObservation> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let inventories = ecs.read_storage::<comp::Inventory>();
        let inventory = (&entities, &colonists)
            .join()
            .find(|(_, colonist)| colonist.0.name == name)
            .and_then(|(entity, _)| inventories.get(entity))?;
        let slot = crate::sys::agent::action_nodes::select_healing_item(inventory, true, 1.0)?;
        let item = inventory.get(slot)?;
        Some(crate::rtsim::tick::class7_item_observation(slot, item))
    }

    /// bastion (COORDINATION-stigmergic-v1, harness hook): the saturation
    /// field at a position's coarse cell.
    pub fn bastion_saturation_at(&self, pos: Vec3<i32>) -> f32 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .saturation_at(pos)
    }

    /// bastion (B-LIVE4, harness hook): cumulative job-claim events over the
    /// board's life (initial claims + re-claims after release). Snapshot
    /// before/after a dig phase → claims-per-job ratio, the mine-oscillation
    /// (in/out bob) telemetry.
    pub fn bastion_total_claims(&self) -> u64 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .total_claims
    }

    /// bastion (MINING-LIVE-FIDELITY, measure-first harness hook): per-cell
    /// audit of the REMAINING (undug) Mine jobs inside `region` —
    /// `(pos, depth, claimed, unreachable, anchored)` — so the fidelity
    /// scenario can classify WHY undug cells sit at end-of-run:
    /// descent-gate-held = deep (depth>2) + !anchored + !claimed;
    /// unreachable = enclosed-flagged; claimed = someone is (or believes
    /// they are) on it. `anchored` uses THE gate's own shared predicate
    /// ([`bastion_jobs::access_anchor_covers`]) so probe and gate cannot
    /// drift. Read-only; access-scaffolding cells excluded (they are not
    /// designated payload).
    pub fn bastion_mine_fidelity_cells(
        &self,
        region: common::bastion::Region,
    ) -> Vec<(Vec3<i32>, u8, bool, bool, bool)> {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        board
            .jobs
            .values()
            .filter(|j| {
                j.kind.is(common::bastion::DesignationKind::Mine)
                    && !j.is_access
                    && j.pos.x >= region.min.x
                    && j.pos.x <= region.max.x
                    && j.pos.y >= region.min.y
                    && j.pos.y <= region.max.y
                    && j.pos.z >= region.min.z
                    && j.pos.z <= region.max.z
            })
            .map(|j| {
                (
                    j.pos,
                    j.depth,
                    j.claimed_by.is_some(),
                    j.unreachable,
                    bastion_jobs::access_anchor_covers(&board.access_anchors, j.pos),
                )
            })
            .collect()
    }

    /// bastion (DPA fixture, harness hook): the live access-anchor list —
    /// the SHAFT-ALWAYS-ACCESSED predicate samples colonist positions
    /// against these via [`bastion_jobs::access_anchor_covers`]. Read-only.
    pub fn bastion_access_anchors(&self) -> Vec<Vec3<i32>> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .access_anchors
            .clone()
    }

    /// bastion (DPA fixture, harness hook): count of live dig-provisioned /
    /// auto-access LADDER rung jobs (`is_access && Ladder`), + how many of
    /// them are material-gated. Leg 3's stairs-preferred assert reads the
    /// first; leg 2's material-hold assert reads both. Read-only.
    pub fn bastion_ladder_access_jobs(&self) -> (usize, usize) {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        let mut total = 0usize;
        let mut gated = 0usize;
        for j in board.jobs.values() {
            if j.is_access && j.kind.is(common::bastion::DesignationKind::Ladder) {
                total += 1;
                if j.required_item.is_some() {
                    gated += 1;
                }
            }
        }
        (total, gated)
    }

    /// bastion (DPA diag, harness hook): full state of every live ACCESS
    /// job — (pos, kind-label, claimed, unreachable, needs_materials,
    /// required_item) — the leg-B "rungs never build" forensics feed.
    /// Read-only.
    pub fn bastion_access_job_dump(
        &self,
    ) -> Vec<(Vec3<i32>, String, bool, bool, bool, Option<&'static str>)> {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        board
            .jobs
            .values()
            .filter(|j| j.is_access)
            .map(|j| {
                (
                    j.pos,
                    format!("{:?}", j.kind),
                    j.claimed_by.is_some(),
                    j.unreachable,
                    j.needs_materials,
                    j.required_item,
                )
            })
            .collect()
    }

    /// bastion (DPA diag, harness hook): the claim gate's material-
    /// availability predicate, decomposed per wood item — for each loose
    /// item of `def`: (pos, in_stockpile, reserved). The rung-fetch
    /// forensics feed: shows exactly which leg of the availability check
    /// fails. Read-only.
    pub fn bastion_material_availability(
        &self,
        def: &str,
    ) -> Vec<(Vec3<f32>, bool, bool)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        let items = ecs.read_storage::<comp::PickupItem>();
        let positions = ecs.read_storage::<comp::Pos>();
        let uids = ecs.read_storage::<Uid>();
        (&items, &positions, &uids)
            .join()
            .filter(|(pi, _, _)| pi.item().item_definition_id().itemdef_id() == Some(def))
            .map(|(_, ipos, iuid)| {
                (
                    ipos.0,
                    board
                        .stockpile_at(ipos.0.map(|e| e.floor() as i32))
                        .is_some(),
                    board.is_reserved(*iuid),
                )
            })
            .collect()
    }

    /// bastion (DPA-2, harness hook): the classified access-block reason —
    /// `Some(item_def)` while the descent frontier holds on missing rung
    /// material. Read-only.
    pub fn bastion_access_block_reason(&self) -> Option<&'static str> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .access_material_missing
    }

    /// bastion (R10 N-FENCE, harness hook): the named colonist's LIVE
    /// traversal authority tuple `(link_id, epoch, member_uid)` — the
    /// fixture captures this mid-climb, forces an abort, then presents the
    /// CAPTURED (now stale) tuple through the stale-write probe. Read-only.
    pub fn bastion_traversal_authority(&self, name: &str) -> Option<(u64, u64, u64)> {
        let uid = self.bastion_colonist_uid(name)?;
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        let task = board
            .bastion_traversal_tasks_probe(common::uid::Uid(std::num::NonZeroU64::new(uid)?))?;
        Some((task.0, task.1, uid))
    }

    /// bastion (M3, read-only): the fair queue of the named colonist's
    /// route link — ordered `(member_uid, enqueue_tick)` pairs plus the
    /// link's reservation generation. `None` = not a route member or no
    /// live link container.
    pub fn bastion_traversal_queue(&self, name: &str) -> Option<(Vec<(u64, u64)>, u64)> {
        let uid = self.bastion_colonist_uid(name)?;
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        let owner =
            board.bastion_route_owner_probe(common::uid::Uid(std::num::NonZeroU64::new(uid)?))?;
        board.bastion_traversal_queue_probe(owner)
    }

    /// bastion (R10 N-FENCE, harness hook — PERMITTED TOUCH): attempt a
    /// movement write against the named colonist's controller presenting an
    /// ARBITRARY authority tuple, through THE production fence
    /// ([`bastion_traversal::fenced_movement_write`] — the tested path IS
    /// the shipping path, B17). Writes a sentinel input on acceptance.
    /// Returns `(accepted, inputs_changed_from_before)`.
    pub fn bastion_r10_stale_write_probe(
        &mut self,
        name: &str,
        link_id: u64,
        epoch: u64,
        member_uid: u64,
    ) -> Option<(bool, bool)> {
        use specs::Join;
        let member = common::uid::Uid(std::num::NonZeroU64::new(member_uid)?);
        let authority = bastion_traversal::TraversalAuthority {
            link_id,
            epoch,
            member,
        };
        let ecs = self.state.ecs();
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        let current_epoch = board.current_epoch(link_id);
        let current_member = board.bastion_traversal_current_member(link_id);
        drop(board);
        let entity = {
            let uids = ecs.read_storage::<common::uid::Uid>();
            let colonists = ecs.read_storage::<comp::Colonist>();
            let entities = ecs.entities();
            (&entities, &uids, &colonists)
                .join()
                .find(|(_, _, c)| c.0.name == name)
                .map(|(e, ..)| e)
        }?;
        let mut controllers = ecs.write_storage::<comp::Controller>();
        let controller = controllers.get_mut(entity)?;
        let before = (controller.inputs.move_dir, controller.inputs.move_z);
        // Sentinel input: distinct from any live value so acceptance is
        // observable; rejection must leave inputs byte-identical.
        let accepted = bastion_traversal::fenced_movement_write(
            current_epoch,
            current_member,
            &authority,
            controller,
            vek::Vec2::new(0.707, -0.707),
            0.5,
        );
        let after = (controller.inputs.move_dir, controller.inputs.move_z);
        Some((accepted, before != after))
    }

    /// bastion (B6 SOFT-0, harness hook): register an access anchor, as a
    /// player-designated or auto-built ladder would (scenarios that place
    /// ladder SPRITES directly bypass the designation path that normally
    /// registers the base — without an anchor, staged routing can't find
    /// the ladder and the B5.8 run-10 beeline/A*-reset failure returns).
    pub fn bastion_register_access_anchor(&mut self, pos: Vec3<i32>) {
        self.state
            .ecs()
            .write_resource::<bastion_jobs::JobBoard>()
            .access_anchors
            .push(pos);
    }

    /// bastion (avatar/playthrough hook): drive a named colonist as a scripted
    /// PLAYER avatar — write locomotion input straight into its `Controller`,
    /// the same component client input lands in, so an endurance run can
    /// exercise input->world interaction (not just the world living itself).
    /// Deterministic by construction: the caller supplies a pure-function-of-
    /// tick input, so cross-run determinism must still hold. Returns whether the
    /// colonist was found. (v1 = locomotion only; abilities are a later add.)
    pub fn bastion_set_avatar_input(
        &mut self,
        name: &str,
        move_dir: vek::Vec2<f32>,
        move_z: f32,
    ) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let entity = {
            let colonists = ecs.read_storage::<comp::Colonist>();
            let entities = ecs.entities();
            (&entities, &colonists)
                .join()
                .find(|(_, c)| c.0.name == name)
                .map(|(e, _)| e)
        };
        let Some(entity) = entity else {
            return false;
        };
        let mut controllers = ecs.write_storage::<comp::Controller>();
        match controllers.get_mut(entity) {
            Some(controller) => {
                controller.inputs.move_dir = move_dir;
                controller.inputs.move_z = move_z;
                true
            },
            None => false,
        }
    }

    /// bastion (TOOL-0, harness hook): equip an item asset into a loaded
    /// colonist's mainhand (deterministic tool-speed scenarios; whatever
    /// the swap displaces is discarded — scenarios don't care).
    pub fn bastion_equip_tool(&mut self, name: &str, asset_id: &str) -> bool {
        use specs::Join;
        let Ok(item) = comp::Item::new_from_asset(asset_id) else {
            return false;
        };
        let time = *self.state.ecs().read_resource::<common::resources::Time>();
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let entities = ecs.entities();
        let mut inventories = ecs.write_storage::<comp::Inventory>();
        for (entity, colonist) in (&entities, &colonists).join() {
            if colonist.0.name == name {
                if let Some(mut inv) = inventories.get_mut(entity) {
                    let _ = inv.replace_loadout_item(
                        comp::slot::EquipSlot::ActiveMainhand,
                        Some(item),
                        time,
                    );
                    return true;
                }
            }
        }
        false
    }

    /// bastion (TOOL-0, harness hook): the tool factor a loaded colonist's
    /// CURRENT mainhand yields for a work type — scenarios assert the
    /// curve end-to-end (equipped pick > bare hands) without timing races.
    pub fn bastion_colonist_tool_factor(
        &self,
        name: &str,
        work: common::bastion::WorkType,
    ) -> Option<f32> {
        use specs::Join;
        let ecs = self.state.ecs();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let entities = ecs.entities();
        let inventories = ecs.read_storage::<comp::Inventory>();
        (&entities, &colonists)
            .join()
            .find(|(_, c)| c.0.name == name)
            .map(|(entity, _)| {
                let tool = inventories.get(entity).and_then(|inv| {
                    inv.equipped(comp::slot::EquipSlot::ActiveMainhand)
                        .and_then(|item| match &*item.kind() {
                            comp::item::ItemKind::Tool(t) => Some((t.kind, item.quality())),
                            _ => None,
                        })
                });
                common::bastion::tool_factor(work, tool)
            })
    }

    /// bastion (B5, harness hook): the block kind at a world position (for
    /// asserting a mined hole / a placed wall).
    pub fn bastion_block_kind(&self, pos: Vec3<i32>) -> Option<common::terrain::BlockKind> {
        use common::vol::ReadVol;
        self.state.terrain().get(pos).ok().map(|b| b.kind())
    }

    /// bastion (FLAT-TEST-ARENA, harness hook): the arena's anchor — the
    /// same world-center wpos the generation override keys on, so the
    /// ARENA leg samples exactly where the slab is.
    pub fn bastion_world_center_wpos(&self) -> Vec2<u32> {
        crate::bastion_flat_arena::world_center_wpos(&self.world)
    }

    /// bastion (GATHER, harness hook): is the block at `pos` still DIRECTLY
    /// collectible? A collected sprite may stay VISIBLE (`into_collected` →
    /// `into_vacant` keeps the sprite for regrowth semantics), so
    /// sprite-presence is the wrong "was it foraged" probe — this is the
    /// exact predicate the forage scan and verb both key on.
    pub fn bastion_block_collectible(&self, pos: Vec3<i32>) -> bool {
        use common::vol::ReadVol;
        self.state
            .terrain()
            .get(pos)
            .is_ok_and(|b| b.is_directly_collectible())
    }

    /// bastion (B5, harness hook): whether any Build job in the board has
    /// `needs_materials` set (visibility into the stalled-blueprint state).
    pub fn bastion_any_job_needs_materials(&self) -> bool {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .jobs
            .values()
            .any(|j| j.needs_materials)
    }

    /// bastion (B5.5, harness hook): jobs whose target block lies inside
    /// `region` (asserting partial-erase removed exactly the erased half).
    pub fn bastion_jobs_in_region(&self, region: common::bastion::Region) -> usize {
        // is_access excluded (B5.8-E3): every caller measures DESIGNATION
        // job state (layer-clear order, flat-floor bounds, cancel
        // cleanliness). Access-plan steps live INSIDE dig volumes by
        // design (access LEADS the descent), and at the slower work pace
        // they persist long enough to straddle layer boundaries — counting
        // them latched d_top_down out of order on healthy runs.
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .jobs
            .values()
            .filter(|j| region.contains_point(j.pos) && !j.is_access)
            .count()
    }

    /// bastion (B5.5, harness hook): colonists holding an `ActiveJob` whose
    /// job id is no longer on the board. Transiently non-zero for at most
    /// one upkeep tick after a cancel; anything persisting is a leaked
    /// claim (standing invariant).
    pub fn bastion_orphaned_claims(&self) -> usize {
        use specs::Join;
        let ecs = self.state.ecs();
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        let active = ecs.read_storage::<comp::bastion::ActiveJob>();
        let colonists = ecs.read_storage::<comp::Colonist>();
        (&active, &colonists)
            .join()
            .filter(|(a, _)| !board.jobs.contains_key(&a.job))
            .count()
    }

    /// Get a reference to the Metrics Registry
    pub fn metrics_registry(&self) -> &Arc<Registry> { &self.metrics_registry }

    /// Get a reference to the Chat Cache
    pub fn chat_cache(&self) -> &ChatCache { &self.chat_cache }

    fn parse_locations(&self, character_list_data: &mut [CharacterItem]) {
        character_list_data.iter_mut().for_each(|c| {
            let name = c
                .location
                .as_ref()
                .and_then(|s| {
                    persistence::parse_waypoint(s)
                        .ok()
                        .and_then(|(waypoint, _)| waypoint.map(|w| w.get_pos()))
                })
                .and_then(|wpos| {
                    self.world
                        .get_location_name(self.index.as_index_ref(), wpos.xy().as_::<i32>())
                });
            c.location = name;
        });
    }

    /// Execute a single server tick, handle input and update the game state by
    /// the given duration.
    pub fn tick(&mut self, _input: Input, dt: Duration) -> Result<Vec<Event>, Error> {
        self.state.ecs().write_resource::<Tick>().0 += 1;
        self.state.ecs().write_resource::<TickStart>().0 = Instant::now();

        // bastion (B-ASSET1): arena upkeep (deferred fixture goto). No-op
        // when the arena resource is absent (i.e. always, outside
        // --asset-arena boots).
        #[cfg(feature = "worldgen")]
        self.bastion_arena_tick();

        // bastion (det-capture): env-gated AUTO-FOUND a colony for NON-INTERACTIVE
        // determinism runs. server-cli has no client to found a colony via the
        // normal command, so on a flat-arena boot with BASTION_AUTOFOUND_COLONY=N
        // set, force-load the slab center (tick 1) then spawn N colonists at the
        // deterministic slab-center spawn (tick 30, once chunks are in). Lets a
        // headless run capture the very colony an overseer client would found —
        // the missing piece for a rendered-equivalent determinism capture.
        #[cfg(feature = "worldgen")]
        {
            use std::sync::OnceLock;
            static N: OnceLock<Option<u8>> = OnceLock::new();
            let n = *N.get_or_init(|| {
                std::env::var("BASTION_AUTOFOUND_COLONY")
                    .ok()
                    .and_then(|s| s.parse::<u8>().ok())
                    .filter(|&n| n > 0 && bastion_flat_arena::enabled())
            });
            if let Some(n) = n {
                use std::sync::atomic::{AtomicBool, Ordering};
                static LOADED: AtomicBool = AtomicBool::new(false);
                static SPAWNED: AtomicBool = AtomicBool::new(false);
                // Force-load the flat slab on the first tick, then found the
                // colony at rtsim data.tick >= 30 — LATE enough that the world is
                // loaded and colonists promote to ACTIVE (wandering) entities, so
                // the capture exercises a MOVING colony, not a frozen spawn. This
                // is only safe because BASTION_DETERMINISTIC (required for this
                // path) makes execution serial + data.tick deterministic, so both
                // runs reach data.tick 30 identically. The founding RNG is also
                // pinned to a fixed seed_tick (0) belt-and-suspenders.
                if !LOADED.swap(true, Ordering::Relaxed) {
                    let center = bastion_flat_arena::world_center_wpos(&self.world);
                    self.bastion_force_load_area(center.map(|e| e as f32), 5);
                } else if !SPAWNED.load(Ordering::Relaxed) {
                    let dtick = self
                        .state
                        .ecs()
                        .read_resource::<rtsim::RtSim>()
                        .state()
                        .data()
                        .tick;
                    if dtick >= 30 {
                        SPAWNED.store(true, Ordering::Relaxed);
                        let center = bastion_flat_arena::world_center_wpos(&self.world);
                        let sp = bastion_flat_arena::spawn_wpos(center);
                        self.bastion_spawn_colony_seeded(sp, n, 0);
                    }
                }
            }
        }

        // Update calendar events as time changes
        // TODO: If a lot of calendar events get added, this might become expensive.
        // Maybe don't do this every tick?
        let new_calendar = self
            .state
            .ecs()
            .read_resource::<Settings>()
            .calendar_mode
            .calendar_now();
        *self.state.ecs_mut().write_resource::<Calendar>() = new_calendar;

        #[cfg(feature = "hot-site")]
        if let Ok(lib) = world::LIB.lock()
            && let Some(lib) = &*lib
        {
            static LAST_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let last_count = LAST_COUNT.load(std::sync::atomic::Ordering::Relaxed);

            let new_count = lib.reload_count();

            if new_count > last_count {
                LAST_COUNT.store(new_count, std::sync::atomic::Ordering::Relaxed);

                let count = cmd::reload_chunks_inner(self, Vec3::zero(), None, true);

                tracing::info!("Reloaded {count} chunks");
            }
        }

        // This tick function is the centre of the Veloren universe. Most server-side
        // things are managed from here, and as such it's important that it
        // stays organised. Please consult the core developers before making
        // significant changes to this code. Here is the approximate order of
        // things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state of the
        //    game
        // 2) Go through any events (timer-driven or otherwise) that need handling and
        //    apply them to the state of the game
        // 3) Go through all incoming client network communications, apply them to the
        //    game state
        // 4) Perform a single LocalState tick (i.e: update the world and entities in
        //    the world)
        // 5) Go through the terrain update queue and apply all changes to the terrain
        // 6) Send relevant state updates to all clients
        // 7) Check for persistence updates related to character data, and message the
        //    relevant entities
        // 8) Update Metrics with current data
        // 9) Finish the tick, passing control of the main thread back to the frontend

        // 1) Build up a list of events for this frame, to be passed to the frontend.
        let mut frontend_events = Vec::new();

        // 2)

        let before_new_connections = Instant::now();

        // 3) Handle inputs from clients
        self.handle_new_connections(&mut frontend_events);

        let before_state_tick = Instant::now();

        fn on_block_update(ecs: &specs::World, changes: Vec<BlockDiff>) {
            // When a resource block updates, inform rtsim. T1.12: the
            // resource-class-change test is the single authoritative
            // `BlockDiff::changes_rtsim_resource` predicate — every applied
            // diff (from `State::apply_terrain_changes`) is screened through
            // it so no resource creation/deletion escapes rtsim's ledger.
            if changes.iter().any(|c| c.changes_rtsim_resource()) {
                ecs.write_resource::<rtsim::RtSim>().hook_block_update(
                    &ecs.read_resource::<Arc<world::World>>(),
                    ecs.read_resource::<world::IndexOwned>().as_index_ref(),
                    changes,
                );
            }
        }

        // 4) Tick the server's LocalState.
        // 5) Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // in sys/terrain.rs
        let mut state_tick_metrics = Default::default();
        let server_constants = (*self.state.ecs().read_resource::<ServerConstants>()).clone();
        self.state.tick(
            dt,
            false,
            Some(&mut state_tick_metrics),
            &server_constants,
            on_block_update,
        );

        let before_handle_events = Instant::now();

        // Process any pending request to disconnect all clients, the disconnections
        // will be processed once handle_events() is called below
        let disconnect_type = self.disconnect_all_clients_if_requested();

        // Handle entity links (such as mounting)
        self.state.maintain_links();

        // Handle game events
        frontend_events.append(&mut self.handle_events());

        let before_update_terrain_and_regions = Instant::now();

        // Apply terrain changes and update the region map after processing server
        // events so that changes made by server events will be immediately
        // visible to client synchronization systems, minimizing the latency of
        // `ServerEvent` mediated effects
        self.update_region_map();
        // NOTE: apply_terrain_changes sends the *new* value since it is not being
        // synchronized during the tick.
        self.state.apply_terrain_changes(on_block_update);

        let before_sync = Instant::now();

        // 6) Synchronise clients with the new state of the world.
        sys::run_sync_systems(self.state.ecs_mut());

        // bastion (det-capture hook): per-tick authoritative colonist Pos dump
        // (raw f32 bits), gated by BASTION_AUTH_POS_LOG. The D1 arbiter — run
        // the game twice, diff these logs, find the FIRST diverging tick (or
        // prove HOLD). Sim-authoritative (server side), so it captures identical
        // state whether the server runs headless (server-cli) or under voxygen's
        // singleplayer — the "where do seeds diverge" probe for the live game.
        {
            use std::sync::OnceLock;
            use std::sync::atomic::{AtomicU64, Ordering};
            static LOG_PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
            let path = LOG_PATH
                .get_or_init(|| std::env::var_os("BASTION_AUTH_POS_LOG").map(Into::into));
            if let Some(path) = path {
                use specs::Join;
                static TICK: AtomicU64 = AtomicU64::new(0);
                let t = TICK.fetch_add(1, Ordering::Relaxed);
                let ecs = self.state.ecs();
                let uids = ecs.read_storage::<common::uid::Uid>();
                let positions = ecs.read_storage::<comp::Pos>();
                let colonists = ecs.read_storage::<comp::Colonist>();
                let mut rows: Vec<(u64, f32, f32, f32)> = (&uids, &positions, &colonists)
                    .join()
                    .map(|(uid, pos, _)| (uid.0.get(), pos.0.x, pos.0.y, pos.0.z))
                    .collect();
                rows.sort_by_key(|(uid, ..)| *uid);
                let mut line = String::new();
                for (uid, x, y, z) in &rows {
                    // raw f32 bits (decimal) so a byte-exact diff is trivial.
                    line.push_str(&format!(
                        "t {t} uid {uid} pos {} {} {}\n",
                        x.to_bits(),
                        y.to_bits(),
                        z.to_bits()
                    ));
                }
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    let _ = f.write_all(line.as_bytes());
                }
            }
        }

        let before_world_tick = Instant::now();

        // Tick the world
        self.world.tick(dt);

        let before_entity_cleanup = Instant::now();

        // In the event of a request to disconnect all players without persistence, we
        // must run the terrain system a second time after the messages to
        // perform client disconnections have been processed. This ensures that any
        // items on the ground are deleted.
        if let Some(DisconnectType::WithoutPersistence) = disconnect_type {
            run_now::<terrain::Sys>(self.state.ecs_mut());
        }

        // Hook rtsim chunk unloads
        #[cfg(feature = "worldgen")]
        {
            let mut rtsim = self.state.ecs().write_resource::<rtsim::RtSim>();
            let world = self.state.ecs().read_resource::<Arc<World>>();
            for chunk in &self.state.terrain_changes().removed_chunks {
                rtsim.hook_unload_chunk(*chunk, &world);
            }
        }

        // Prevent anchor entity chains which are not currently supported due to:
        // * potential cycles?
        // * unloading a chain could occur across an unbounded number of ticks with the
        //   current implementation.
        // * in particular, we want to be able to unload all entities in a
        //   limited number of ticks when a database error occurs and kicks all
        //   players (not quiet sure on exact time frame, since it already
        //   takes a tick after unloading all chunks for entities to despawn?),
        //   see this thread and the discussion linked from there:
        //   https://gitlab.com/veloren/veloren/-/merge_requests/2668#note_634913847
        let anchors = self.state.ecs().read_storage::<Anchor>();
        let anchored_anchor_entities: Vec<Entity> = (
            &self.state.ecs().entities(),
            &self.state.ecs().read_storage::<Anchor>(),
        )
            .join()
            .filter_map(|(_, anchor)| match anchor {
                Anchor::Entity(anchor_entity) => Some(*anchor_entity),
                _ => None,
            })
            // We allow Anchor::Entity(_) -> Anchor::Chunk(_) connections, since they can't chain further.
            //
            // NOTE: The entity with `Anchor::Entity` will unload one tick after the entity with `Anchor::Chunk`.
            .filter(|anchor_entity| match anchors.get(*anchor_entity) {
                Some(Anchor::Entity(_)) => true,
                Some(Anchor::Chunk(_)) | None => false
            })
            .collect();
        drop(anchors);

        for entity in anchored_anchor_entities {
            if cfg!(debug_assertions) {
                panic!("Entity anchor chain detected");
            }
            error!(
                "Detected an anchor entity that itself has an anchor entity - anchor chains are \
                 not currently supported. The entity's Anchor component has been deleted"
            );
            self.state.delete_component::<Anchor>(entity);
        }

        // Remove NPCs that are outside the view distances of all players
        // This is done by removing NPCs in unloaded chunks
        let to_delete = {
            let terrain = self.state.terrain();
            (
                &self.state.ecs().entities(),
                &self.state.ecs().read_storage::<comp::Pos>(),
                !&self.state.ecs().read_storage::<comp::Presence>(),
                self.state.ecs().read_storage::<Anchor>().maybe(),
                self.state.ecs().read_storage::<Is<VolumeRider>>().maybe(),
            )
                .join()
                .filter(|(_, pos, _, anchor, is_volume_rider)| {
                    let pos = is_volume_rider
                        .and_then(|is_volume_rider| match is_volume_rider.pos.kind {
                            Volume::Terrain => None,
                            Volume::Entity(e) => {
                                let e = self.state.ecs().entity_from_uid(e)?;
                                let pos = self
                                    .state
                                    .ecs()
                                    .read_storage::<comp::Pos>()
                                    .get(e)
                                    .copied()?;

                                Some(pos.0)
                            },
                        })
                        .unwrap_or(pos.0);
                    let chunk_key = terrain.pos_key(pos.map(|e| e.floor() as i32));
                    match anchor {
                        Some(Anchor::Chunk(hc)) => {
                            // Check if both this chunk and the NPCs `home_chunk` is unloaded. If
                            // so, we delete them. We check for
                            // `home_chunk` in order to avoid duplicating
                            // the entity under some circumstances.
                            terrain.get_key_real(chunk_key).is_none()
                                && terrain.get_key_real(*hc).is_none()
                        },
                        Some(Anchor::Entity(entity)) => !self.state.ecs().is_alive(*entity),
                        None => terrain.get_key_real(chunk_key).is_none(),
                    }
                })
                .map(|(entity, _, _, _, _)| entity)
                .collect::<Vec<_>>()
        };

        #[cfg(feature = "worldgen")]
        {
            let mut rtsim = self.state.ecs().write_resource::<rtsim::RtSim>();
            let rtsim_entities = self.state.ecs().read_storage();
            for entity in &to_delete {
                if let Some(rtsim_entity) = rtsim_entities.get(*entity) {
                    rtsim.hook_rtsim_entity_unload(*rtsim_entity);
                }
            }
        }

        // Actually perform entity deletion
        for entity in to_delete {
            if let Err(e) = self.state.delete_entity_recorded(entity) {
                error!(?e, "Failed to delete agent outside the terrain");
            }
        }

        if let Some(DisconnectType::WithoutPersistence) = disconnect_type {
            info!(
                "Disconnection of all players without persistence complete, signalling to \
                 persistence thread that character updates may continue to be processed"
            );
            self.state
                .ecs()
                .fetch_mut::<CharacterUpdater>()
                .disconnected_success();
        }

        // 7 Persistence updates
        let before_persistence_updates = Instant::now();

        let character_loader = self.state.ecs().read_resource::<CharacterLoader>();

        let mut character_updater = self.state.ecs().write_resource::<CharacterUpdater>();
        let updater_messages: Vec<CharacterUpdaterMessage> = character_updater.messages().collect();

        // Get character-related database responses and notify the requesting client
        character_loader
            .messages()
            .chain(updater_messages)
            .for_each(|message| match message {
                CharacterUpdaterMessage::DatabaseBatchCompletion(batch_id) => {
                    character_updater.process_batch_completion(batch_id);
                },
                CharacterUpdaterMessage::CharacterScreenResponse(response) => {
                    match response.response_kind {
                        CharacterScreenResponseKind::CharacterList(result) => match result {
                            Ok(mut character_list_data) => {
                                self.parse_locations(&mut character_list_data);
                                self.notify_client(
                                    response.target_entity,
                                    ServerGeneral::CharacterListUpdate(character_list_data),
                                )
                            },
                            Err(error) => self.notify_client(
                                response.target_entity,
                                ServerGeneral::CharacterActionError(error.to_string()),
                            ),
                        },
                        CharacterScreenResponseKind::CharacterCreation(result) => match result {
                            Ok((character_id, mut list)) => {
                                self.parse_locations(&mut list);
                                self.notify_client(
                                    response.target_entity,
                                    ServerGeneral::CharacterListUpdate(list),
                                );
                                self.notify_client(
                                    response.target_entity,
                                    ServerGeneral::CharacterCreated(character_id),
                                );
                            },
                            Err(error) => self.notify_client(
                                response.target_entity,
                                ServerGeneral::CharacterActionError(error.to_string()),
                            ),
                        },
                        CharacterScreenResponseKind::CharacterEdit(result) => match result {
                            Ok((character_id, mut list)) => {
                                self.parse_locations(&mut list);
                                self.notify_client(
                                    response.target_entity,
                                    ServerGeneral::CharacterListUpdate(list),
                                );
                                self.notify_client(
                                    response.target_entity,
                                    ServerGeneral::CharacterEdited(character_id),
                                );
                            },
                            Err(error) => self.notify_client(
                                response.target_entity,
                                ServerGeneral::CharacterActionError(error.to_string()),
                            ),
                        },
                        CharacterScreenResponseKind::CharacterData(result) => {
                            match *result {
                                Ok((character_data, skill_set_persistence_load_error)) => {
                                    let PersistedComponents {
                                        body,
                                        hardcore,
                                        stats,
                                        skill_set,
                                        inventory,
                                        waypoint,
                                        pets,
                                        active_abilities,
                                        map_marker,
                                    } = character_data;
                                    let character_data = (
                                        body,
                                        hardcore,
                                        stats,
                                        skill_set,
                                        inventory,
                                        waypoint,
                                        pets,
                                        active_abilities,
                                        map_marker,
                                    );
                                    // TODO: Does this need to be a server event? E.g. we could
                                    // just handle it here.
                                    self.state.emit_event_now(UpdateCharacterDataEvent {
                                        entity: response.target_entity,
                                        components: character_data,
                                        metadata: skill_set_persistence_load_error,
                                    })
                                },
                                Err(error) => {
                                    // We failed to load data for the character from the DB. Notify
                                    // the client to push the state back to character selection,
                                    // with the error to display
                                    self.notify_client(
                                        response.target_entity,
                                        ServerGeneral::CharacterDataLoadResult(Err(
                                            error.to_string()
                                        )),
                                    );

                                    // Clean up the entity data on the server
                                    self.state.emit_event_now(ExitIngameEvent {
                                        entity: response.target_entity,
                                    })
                                },
                            }
                        },
                    }
                },
            });

        drop(character_loader);
        drop(character_updater);

        {
            // Check for new chunks; cancel and regenerate all chunks if the asset has been
            // reloaded. Note that all of these assignments are no-ops, so the
            // only work we do here on the fast path is perform a relaxed read on an atomic.
            // boolean.
            let index = &mut self.index;
            let world = &mut self.world;
            let ecs = self.state.ecs_mut();
            let slow_jobs = ecs.write_resource::<SlowJobPool>();

            index.reload_if_changed(|index| {
                let mut chunk_generator = ecs.write_resource::<ChunkGenerator>();
                let client = ecs.read_storage::<Client>();
                let mut terrain = ecs.write_resource::<common::terrain::TerrainGrid>();
                #[cfg(feature = "worldgen")]
                let rtsim = ecs.read_resource::<rtsim::RtSim>();
                #[cfg(not(feature = "worldgen"))]
                let rtsim = ();

                // Cancel all pending chunks.
                chunk_generator.cancel_all();

                if client.is_empty() {
                    // No clients, so just clear all terrain.
                    terrain.clear();
                } else {
                    // There's at least one client, so regenerate all chunks.
                    terrain.iter().for_each(|(pos, _)| {
                        chunk_generator.generate_chunk(
                            None,
                            pos,
                            &slow_jobs,
                            Arc::clone(world),
                            &rtsim,
                            index.clone(),
                            (
                                *ecs.read_resource::<TimeOfDay>(),
                                (*ecs.read_resource::<Calendar>()).clone(),
                            ),
                        );
                    });
                }
            });
        }

        let end_of_server_tick = Instant::now();

        // 8) Update Metrics
        run_now::<sys::metrics::Sys>(self.state.ecs());

        {
            // Report timing info
            let tick_metrics = self.state.ecs().read_resource::<TickMetrics>();

            let tt = &tick_metrics.tick_time;
            tt.with_label_values(&["new connections"])
                .set((before_state_tick - before_new_connections).as_nanos() as i64);
            tt.with_label_values(&["handle server events"])
                .set((before_update_terrain_and_regions - before_handle_events).as_nanos() as i64);
            tt.with_label_values(&["update terrain and region map"])
                .set((before_sync - before_update_terrain_and_regions).as_nanos() as i64);
            tt.with_label_values(&["state"])
                .set((before_handle_events - before_state_tick).as_nanos() as i64);
            tt.with_label_values(&["world tick"])
                .set((before_entity_cleanup - before_world_tick).as_nanos() as i64);
            tt.with_label_values(&["entity cleanup"])
                .set((before_persistence_updates - before_entity_cleanup).as_nanos() as i64);
            tt.with_label_values(&["persistence_updates"])
                .set((end_of_server_tick - before_persistence_updates).as_nanos() as i64);
            for (label, duration) in state_tick_metrics.timings {
                tick_metrics
                    .state_tick_time
                    .with_label_values(&[label])
                    .set(duration.as_nanos() as i64);
            }
            tick_metrics.tick_time_hist.observe(
                end_of_server_tick
                    .duration_since(before_state_tick)
                    .as_secs_f64(),
            );
        }

        // 9) Finish the tick, pass control back to the frontend.

        Ok(frontend_events)
    }

    /// Clean up the server after a tick.
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();

        // Maintain persisted terrain
        #[cfg(feature = "persistent_world")]
        self.state
            .ecs()
            .try_fetch_mut::<TerrainPersistence>()
            .map(|mut t| t.maintain());
    }

    // Run RegionMap tick to update entity region occupancy
    fn update_region_map(&mut self) {
        prof_span!("Server::update_region_map");
        let ecs = self.state().ecs();
        ecs.write_resource::<RegionMap>().tick(
            ecs.read_storage::<comp::Pos>(),
            ecs.read_storage::<comp::Vel>(),
            ecs.read_storage::<comp::Presence>(),
            ecs.entities(),
        );
    }

    fn initialize_client(&mut self, client: connection_handler::IncomingClient) -> Entity {
        let entity = self
            .state
            .ecs_mut()
            .create_entity_synced()
            .with(client)
            .build();
        self.state
            .ecs()
            .read_resource::<metrics::PlayerMetrics>()
            .clients_connected
            .inc();
        entity
    }

    /// Disconnects all clients if requested by either an admin command or
    /// due to a persistence transaction failure and returns the processed
    /// DisconnectionType
    fn disconnect_all_clients_if_requested(&mut self) -> Option<DisconnectType> {
        let mut character_updater = self.state.ecs().fetch_mut::<CharacterUpdater>();

        let disconnect_type = self.get_disconnect_all_clients_requested(&mut character_updater);
        if let Some(disconnect_type) = disconnect_type {
            let with_persistence = disconnect_type == DisconnectType::WithPersistence;
            let clients = self.state.ecs().read_storage::<Client>();
            let entities = self.state.ecs().entities();

            info!(
                "Disconnecting all clients ({} persistence) as requested",
                if with_persistence { "with" } else { "without" }
            );
            for (_, entity) in (&clients, &entities).join() {
                info!("Emitting client disconnect event for entity: {:?}", entity);
                if with_persistence {
                    self.state.emit_event_now(ClientDisconnectEvent(
                        entity,
                        comp::DisconnectReason::Kicked,
                    ))
                } else {
                    self.state
                        .emit_event_now(ClientDisconnectWithoutPersistenceEvent(entity))
                };
            }

            self.disconnect_all_clients_requested = false;
        }

        disconnect_type
    }

    fn get_disconnect_all_clients_requested(
        &self,
        character_updater: &mut CharacterUpdater,
    ) -> Option<DisconnectType> {
        let without_persistence_requested = character_updater.disconnect_all_clients_requested();
        let with_persistence_requested = self.disconnect_all_clients_requested;

        if without_persistence_requested {
            return Some(DisconnectType::WithoutPersistence);
        };
        if with_persistence_requested {
            return Some(DisconnectType::WithPersistence);
        };
        None
    }

    /// Handle new client connections.
    fn handle_new_connections(&mut self, frontend_events: &mut Vec<Event>) {
        while let Ok(sender) = self.connection_handler.info_requester_receiver.try_recv() {
            // can fail, e.g. due to timeout or network prob.
            trace!("sending info to connection_handler");
            let _ = sender.send(connection_handler::ServerInfoPacket {
                info: self.get_server_info(),
                time: self.state.get_time(),
            });
        }

        while let Ok(incoming) = self.connection_handler.client_receiver.try_recv() {
            let entity = self.initialize_client(incoming);
            frontend_events.push(Event::ClientConnected { entity });
        }
    }

    pub fn notify_client<S>(&self, entity: EcsEntity, msg: S)
    where
        S: Into<ServerMsg>,
    {
        if let Some(client) = self.state.ecs().read_storage::<Client>().get(entity) {
            client.send_fallible(msg);
        }
    }

    pub fn notify_players(&mut self, msg: ServerGeneral) { self.state.notify_players(msg); }

    fn process_command(&mut self, entity: EcsEntity, name: String, args: Vec<String>) {
        // Find the command object and run its handler.
        if let Ok(command) = name.parse::<ServerChatCommand>() {
            command.execute(self, entity, args);
        } else {
            #[cfg(feature = "plugins")]
            {
                let mut plugin_manager = self.state.ecs().write_resource::<PluginMgr>();
                let ecs_world = EcsWorld {
                    entities: &self.state.ecs().entities(),
                    health: self.state.ecs().read_component().into(),
                    uid: self.state.ecs().read_component().into(),
                    id_maps: &self.state.ecs().read_resource::<IdMaps>().into(),
                    player: self.state.ecs().read_component().into(),
                };
                let uid = if let Some(uid) = ecs_world.uid.get(entity).copied() {
                    uid
                } else {
                    self.notify_client(
                        entity,
                        ServerGeneral::server_msg(
                            comp::ChatType::CommandError,
                            common::comp::Content::Plain(
                                "Can't get player UUID (player may be disconnected?)".to_string(),
                            ),
                        ),
                    );
                    return;
                };
                match plugin_manager.command_event(&ecs_world, &name, args.as_slice(), uid) {
                    Err(common_state::plugin::CommandResults::UnknownCommand) => self
                        .notify_client(
                            entity,
                            ServerGeneral::server_msg(
                                comp::ChatType::CommandError,
                                common::comp::Content::Plain(format!(
                                    "Unknown command '/{name}'.\nType '/help' for available \
                                     commands",
                                )),
                            ),
                        ),
                    Ok(value) => {
                        self.notify_client(
                            entity,
                            ServerGeneral::server_msg(
                                comp::ChatType::CommandInfo,
                                common::comp::Content::Plain(value.join("\n")),
                            ),
                        );
                    },
                    Err(common_state::plugin::CommandResults::PluginError(err)) => {
                        self.notify_client(
                            entity,
                            ServerGeneral::server_msg(
                                comp::ChatType::CommandError,
                                common::comp::Content::Plain(format!(
                                    "Error occurred while executing command '/{name}'.\n{err}"
                                )),
                            ),
                        );
                    },
                    Err(common_state::plugin::CommandResults::HostError(err)) => {
                        error!(?err, ?name, ?args, "Can't execute command");
                        self.notify_client(
                            entity,
                            ServerGeneral::server_msg(
                                comp::ChatType::CommandError,
                                common::comp::Content::Plain(format!(
                                    "Internal error {err:?} while executing '/{name}'.\nContact \
                                     the server administrator",
                                )),
                            ),
                        );
                    },
                }
            }
        }
    }

    fn entity_admin_role(&self, entity: EcsEntity) -> Option<comp::AdminRole> {
        self.state
            .read_component_copied::<comp::Admin>(entity)
            .map(|admin| admin.0)
    }

    pub fn number_of_players(&self) -> i64 {
        self.state.ecs().read_storage::<Client>().join().count() as i64
    }

    /// NOTE: Do *not* allow this to be called from any command that doesn't go
    /// through the CLI!
    pub fn add_admin(&mut self, username: &str, role: comp::AdminRole) {
        let mut editable_settings = self.editable_settings_mut();
        let login_provider = self.state.ecs().fetch::<LoginProvider>();
        let data_dir = self.data_dir();
        if let Some(entity) = add_admin(
            username,
            role,
            &login_provider,
            &mut editable_settings,
            &data_dir.path,
        )
        .and_then(|uuid| {
            let state = &self.state;
            (
                &state.ecs().entities(),
                &state.read_storage::<comp::Player>(),
            )
                .join()
                .find(|(_, player)| player.uuid() == uuid)
                .map(|(e, _)| e)
        }) {
            drop((data_dir, login_provider, editable_settings));
            // Add admin component if the player is ingame; if they are not, we can ignore
            // the write failure.
            self.state
                .write_component_ignore_entity_dead(entity, comp::Admin(role));
        };
    }

    /// NOTE: Do *not* allow this to be called from any command that doesn't go
    /// through the CLI!
    pub fn remove_admin(&self, username: &str) {
        let mut editable_settings = self.editable_settings_mut();
        let login_provider = self.state.ecs().fetch::<LoginProvider>();
        let data_dir = self.data_dir();
        if let Some(entity) = remove_admin(
            username,
            &login_provider,
            &mut editable_settings,
            &data_dir.path,
        )
        .and_then(|uuid| {
            let state = &self.state;
            (
                &state.ecs().entities(),
                &state.read_storage::<comp::Player>(),
            )
                .join()
                .find(|(_, player)| player.uuid() == uuid)
                .map(|(e, _)| e)
        }) {
            // Remove admin component if the player is ingame
            self.state
                .ecs()
                .write_storage::<comp::Admin>()
                .remove(entity);
        };
    }

    /// Useful for testing without a client
    /// view_distance: distance in chunks that are persisted, this acts like the
    /// player view distance so it is actually a bit farther due to a buffer
    /// zone
    #[cfg(feature = "worldgen")]
    pub fn create_centered_persister(&mut self, view_distance: u32) {
        let world_dims_chunks = self.world.sim().get_size();
        let world_dims_blocks = TerrainChunkSize::blocks(world_dims_chunks);
        // NOTE: origin is in the corner of the map
        // TODO: extend this function to have picking a random position or specifying a
        // position as options
        //let mut rng = rand::rng();
        // // Pick a random position but not to close to the edge
        // let rand_pos = world_dims_blocks.map(|e| e as i32).map(|e| e / 2 +
        // rng.random_range(-e/2..e/2 + 1));
        let pos = comp::Pos(Vec3::from(world_dims_blocks.map(|e| e as f32 / 2.0)));
        self.state
            .create_persister(pos, view_distance, &self.world, &self.index)
            .build();
    }

    /// Used by benchmarking code.
    pub fn chunks_pending(&mut self) -> bool {
        self.state_mut()
            .mut_resource::<ChunkGenerator>()
            .pending_chunks()
            .next()
            .is_some()
    }

    /// Sets the SQL log mode at runtime
    pub fn set_sql_log_mode(&mut self, sql_log_mode: SqlLogMode) {
        // Unwrap is safe here because we only perform a variable assignment with the
        // RwLock taken meaning that no panic can occur that would cause the
        // RwLock to become poisoned. This justification also means that calling
        // unwrap() on the associated read() calls for this RwLock is also safe
        // as long as no code that can panic is introduced here.
        let mut database_settings = self.database_settings.write().unwrap();
        database_settings.sql_log_mode = sql_log_mode;
        // Drop the RwLockWriteGuard to avoid performing unnecessary actions (logging)
        // with the lock taken.
        drop(database_settings);
        info!("SQL log mode changed to {:?}", sql_log_mode);
    }

    pub fn disconnect_all_clients(&mut self) {
        info!("Disconnecting all clients due to local console command");
        self.disconnect_all_clients_requested = true;
    }

    /// Headless-harness finalization hook: stop the connection worker and
    /// synchronously finish the network's existing graceful shutdown before
    /// the server-owned Tokio runtime is dropped.
    ///
    /// Normal live servers keep their non-blocking `Drop` behavior. This is
    /// intentionally narrow because deterministic harnesses must include all
    /// teardown diagnostics in their final verdict.
    pub fn shutdown_network_for_harness(&mut self) -> Result<(), String> {
        self.connection_handler.shutdown_and_wait(&self.runtime)
    }

    /// Sends the given client a message with their current battle mode and
    /// whether they can change it.
    ///
    /// This function expects the `EcsEntity` to represent a player, otherwise
    /// it will log an error.
    pub fn get_battle_mode_for(&mut self, client: EcsEntity) {
        let ecs = self.state.ecs();
        let time = ecs.read_resource::<Time>();
        let settings = ecs.read_resource::<Settings>();
        let players = ecs.read_storage::<comp::Player>();
        let get_player_result = players.get(client).ok_or_else(|| {
            error!("Can't get player component for client.");

            Content::Plain("Can't get player component for client.".to_string())
        });
        let player = match get_player_result {
            Ok(player) => player,
            Err(content) => {
                self.notify_client(
                    client,
                    ServerGeneral::server_msg(ChatType::CommandError, content),
                );
                return;
            },
        };

        let mut msg = format!("Current battle mode: {:?}.", player.battle_mode);

        if settings.gameplay.battle_mode.allow_choosing() {
            msg.push_str(" Possible to change.");
        } else {
            msg.push_str(" Global.");
        }

        if let Some(change) = player.last_battlemode_change {
            let Time(time) = *time;
            let Time(change) = change;
            let elapsed = time - change;
            let next = BATTLE_MODE_COOLDOWN - elapsed;

            if next > 0.0 {
                let notice = format!(" Next change will be available in: {:.0} seconds", next);
                msg.push_str(&notice);
            }
        }

        self.notify_client(
            client,
            ServerGeneral::server_msg(ChatType::CommandInfo, Content::Plain(msg)),
        );
    }

    /// Sets the battle mode for the given client or informs them if they are
    /// not allowed to change it.
    ///
    /// This function expects the `EcsEntity` to represent a player, otherwise
    /// it will log an error.
    pub fn set_battle_mode_for(&mut self, client: EcsEntity, battle_mode: BattleMode) {
        let ecs = self.state.ecs();
        let time = ecs.read_resource::<Time>();
        let settings = ecs.read_resource::<Settings>();

        if !settings.gameplay.battle_mode.allow_choosing() {
            self.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::localized("command-disabled-by-settings"),
                ),
            );

            return;
        }

        #[cfg(feature = "worldgen")]
        let in_town = {
            let pos = if let Some(pos) = self
                .state
                .ecs()
                .read_storage::<comp::Pos>()
                .get(client)
                .copied()
            {
                pos
            } else {
                self.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        Content::localized_with_args("command-position-unavailable", [(
                            "target", "target",
                        )]),
                    ),
                );

                return;
            };

            let wpos = pos.0.xy().map(|x| x as i32);
            let chunk_pos = wpos.wpos_to_cpos();
            self.world.civs().sites().any(|site| {
                // empirical
                const RADIUS: f32 = 9.0;
                let delta = site
                    .center
                    .map(|x| x as f32)
                    .distance(chunk_pos.map(|x| x as f32));
                delta < RADIUS
            })
        };

        #[cfg(not(feature = "worldgen"))]
        let in_town = true;

        if !in_town {
            self.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::localized("command-battlemode-intown"),
                ),
            );

            return;
        }

        let mut players = ecs.write_storage::<comp::Player>();
        let mut player = if let Some(info) = players.get_mut(client) {
            info
        } else {
            error!("Failed to get info for player.");

            return;
        };

        if let Some(Time(last_change)) = player.last_battlemode_change {
            let Time(time) = *time;
            let elapsed = time - last_change;
            if elapsed < BATTLE_MODE_COOLDOWN {
                let next = BATTLE_MODE_COOLDOWN - elapsed;

                self.notify_client(
                    client,
                    ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        Content::Plain(format!(
                            "Next change will be available in {next:.0} seconds."
                        )),
                    ),
                );

                return;
            }
        }

        if player.battle_mode == battle_mode {
            self.notify_client(
                client,
                ServerGeneral::server_msg(
                    ChatType::CommandInfo,
                    Content::localized("command-battlemode-same"),
                ),
            );

            return;
        }

        player.battle_mode = battle_mode;
        player.last_battlemode_change = Some(*time);

        self.notify_client(
            client,
            ServerGeneral::server_msg(
                ChatType::CommandInfo,
                Content::localized_with_args("command-battlemode-updated", [(
                    "battlemode",
                    format!("{battle_mode:?}"),
                )]),
            ),
        );

        drop(players);

        let uid = ecs.read_storage::<Uid>().get(client).copied().unwrap();

        self.state().notify_players(ServerGeneral::PlayerListUpdate(
            PlayerListUpdate::UpdateBattleMode(uid, battle_mode),
        ));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.state
            .notify_players(ServerGeneral::Disconnect(DisconnectReason::Shutdown));

        #[cfg(feature = "persistent_world")]
        self.state
            .ecs()
            .try_fetch_mut::<TerrainPersistence>()
            .map(|mut terrain_persistence| {
                info!("Unloading terrain persistence...");
                terrain_persistence.unload_all()
            });

        #[cfg(feature = "worldgen")]
        {
            debug!("Saving rtsim state...");
            self.state.ecs().write_resource::<rtsim::RtSim>().save(true);
        }

        // DET-TER-018 (v5 deep-pass): the recorder finalizes LAST — after
        // terrain persistence unload and the rtsim save — so the tape's
        // terminal record covers the true shutdown persistence sequence
        // instead of claiming finalization before persistence ran.
        bastion_flight_recorder::finalize();
    }
}

#[must_use]
pub fn handle_edit<T, S: settings::EditableSetting>(
    data: T,
    result: Option<(String, Result<(), settings::SettingError<S>>)>,
) -> Option<T> {
    use crate::settings::SettingError;
    let (info, result) = result?;
    match result {
        Ok(()) => {
            info!("{}", info);
            Some(data)
        },
        Err(SettingError::Io(err)) => {
            warn!(
                ?err,
                "Failed to write settings file to disk, but succeeded in memory (success message: \
                 {})",
                info,
            );
            Some(data)
        },
        Err(SettingError::Integrity(err)) => {
            error!(?err, "Encountered an error while validating the request",);
            None
        },
    }
}

/// If successful returns the Some(uuid) of the added admin
///
/// NOTE: Do *not* allow this to be called from any command that doesn't go
/// through the CLI!
#[must_use]
pub fn add_admin(
    username: &str,
    role: comp::AdminRole,
    login_provider: &LoginProvider,
    editable_settings: &mut EditableSettings,
    data_dir: &std::path::Path,
) -> Option<common::uuid::Uuid> {
    use crate::settings::EditableSetting;
    let role_ = role.into();
    match login_provider.username_to_uuid(username) {
        Ok(uuid) => handle_edit(
            uuid,
            editable_settings.admins.edit(data_dir, |admins| {
                match admins.insert(uuid, settings::AdminRecord {
                    username_when_admined: Some(username.into()),
                    date: chrono::Utc::now(),
                    role: role_,
                }) {
                    None => Some(format!(
                        "Successfully added {} ({}) as {:?}!",
                        username, uuid, role
                    )),
                    Some(old_admin) if old_admin.role == role_ => {
                        info!("{} ({}) already has role: {:?}!", username, uuid, role);
                        None
                    },
                    Some(old_admin) => Some(format!(
                        "{} ({}) role changed from {:?} to {:?}!",
                        username, uuid, old_admin.role, role
                    )),
                }
            }),
        ),
        Err(err) => {
            error!(
                ?err,
                "Could not find uuid for this name; either the user does not exist or there was \
                 an error communicating with the auth server."
            );
            None
        },
    }
}

/// If successful returns the Some(uuid) of the removed admin
///
/// NOTE: Do *not* allow this to be called from any command that doesn't go
/// through the CLI!
#[must_use]
pub fn remove_admin(
    username: &str,
    login_provider: &LoginProvider,
    editable_settings: &mut EditableSettings,
    data_dir: &std::path::Path,
) -> Option<common::uuid::Uuid> {
    use crate::settings::EditableSetting;
    match login_provider.username_to_uuid(username) {
        Ok(uuid) => handle_edit(
            uuid,
            editable_settings.admins.edit(data_dir, |admins| {
                if let Some(admin) = admins.remove(&uuid) {
                    Some(format!(
                        "Successfully removed {} ({}) with role {:?} from the admins list",
                        username, uuid, admin.role,
                    ))
                } else {
                    info!("{} ({}) is not an admin!", username, uuid);
                    None
                }
            }),
        ),
        Err(err) => {
            error!(
                ?err,
                "Could not find uuid for this name; either the user does not exist or there was \
                 an error communicating with the auth server."
            );
            None
        },
    }
}
