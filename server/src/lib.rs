#![deny(unsafe_code)]
#![expect(
    clippy::option_map_unit_fn,
    clippy::needless_pass_by_ref_mut // until we find a better way for specs
)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(box_patterns, option_zip, const_type_name, slice_partition_dedup)]

pub mod apex_certificate;
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
    bastion_actions, bastion_chop, bastion_entity_event_log, bastion_flat_arena,
    bastion_flight_recorder, bastion_founding_preset, bastion_jobs, bastion_mood, bastion_path,
    bastion_piles, bastion_traversal, bastion_traversal_tooling,
};
pub mod bootstrap_freshness_minter;
mod character_creator;
pub mod chat;
pub mod chunk_generator;
mod chunk_serialize;
pub mod client;
pub mod cmd;
pub mod connection_handler;
pub mod content_epoch;
mod data_dir;
pub mod error;
pub mod events;
pub mod input;
pub mod location;
pub mod lod;
pub mod login_provider;
pub mod metrics;
pub mod net_checkpoint;
pub mod net_command;
mod net_command_bypass;
mod net_command_canaries;
mod net_checkpoint_canaries;
mod net_checkpoint_disconnect;
pub mod persistence;
mod pet;
pub mod plugin_deployment_policy;
pub mod presence;
pub mod rtsim;
pub mod physics_cohort;
pub mod save_inventory;
pub mod save_migration;
pub mod save_universe;
pub mod semantic_net;
pub mod session_registry;
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
use common::apex::identity::{OsRandomBytesSourceV1, ServerBootId};
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

    /// APEX-T3.1: identifies this live server-process incarnation. Fresh on
    /// every `Server::new`, never persisted, never part of authoritative
    /// simulation state (see the ECS-resource insertion site for the same
    /// caveat repeated where it matters for determinism boundary tests).
    server_boot_id: ServerBootId,
}

/// `E11-6b`: the ordering half of `Server::bastion_persistent_item_snapshots`,
/// extracted so the claim -- output order is a function of `Uid`, not of
/// join/allocator iteration order -- is directly testable without a live
/// ECS world. The `entity.id()`/amount/position fields ride along
/// unsorted-on; only the `Uid` decides the order, and it is dropped from
/// the output because it was never part of the caller's contract.
fn sort_persistent_item_snapshots_by_uid_v1(
    mut items: Vec<(u32, u64, Vec3<f32>, Uid)>,
) -> Vec<(u32, u64, Vec3<f32>)> {
    items.sort_by_key(|(_, _, _, uid)| *uid);
    items.into_iter().map(|(entity_id, amount, pos, _)| (entity_id, amount, pos)).collect()
}

#[cfg(test)]
mod bastion_persistent_item_snapshots_v1 {
    use super::*;

    fn uid(n: u64) -> Uid { Uid(std::num::NonZeroU64::new(n).unwrap()) }

    /// The row's falsifier: the SAME set of `Uid`-tagged rows, presented
    /// in two DIFFERENT pre-sort orders (standing in for two different
    /// allocator/join iteration orders across two runs with the same
    /// live entities), must sort to the IDENTICAL output. Before `E11-6b`
    /// the pre-sort order was itself the (entity-id) sort key, so this
    /// would have been vacuous; it is real here because the two input
    /// orders below are not already `Uid`-sorted.
    #[test]
    fn output_order_is_a_function_of_uid_not_of_pre_sort_order() {
        let row = |entity_id: u32, amount: u64, x: f32, uid_n: u64| {
            (entity_id, amount, Vec3::new(x, 0.0, 0.0), uid(uid_n))
        };
        // entity ids deliberately DISAGREE with uid order (3's uid is
        // smallest, 1's is largest) -- a leftover entity-id sort would
        // produce a visibly different order than a uid sort here.
        let forward = vec![row(1, 10, 1.0, 30), row(2, 20, 2.0, 20), row(3, 30, 3.0, 10)];
        let shuffled = vec![row(3, 30, 3.0, 10), row(1, 10, 1.0, 30), row(2, 20, 2.0, 20)];

        let a = sort_persistent_item_snapshots_by_uid_v1(forward);
        let b = sort_persistent_item_snapshots_by_uid_v1(shuffled);

        assert_eq!(a, b, "the same Uid set in a different pre-sort order produced a different result");
        assert_eq!(
            a,
            vec![(3, 30, Vec3::new(3.0, 0.0, 0.0)), (2, 20, Vec3::new(2.0, 0.0, 0.0)), (1, 10, Vec3::new(1.0, 0.0, 0.0))],
            "output must be uid-ascending, not entity-id-ascending"
        );
    }

    /// Non-vacuity: sorting by the OLD key (entity id) on these same
    /// fixtures gives a DIFFERENT order than the uid sort above -- proof
    /// the two orderings are not accidentally coincident on this data,
    /// which would make the falsifier above pass for the wrong reason.
    #[test]
    fn entity_id_order_and_uid_order_genuinely_differ_on_this_fixture() {
        let row = |entity_id: u32, amount: u64, x: f32, uid_n: u64| {
            (entity_id, amount, Vec3::new(x, 0.0, 0.0), uid(uid_n))
        };
        let mut by_entity_id = vec![row(1, 10, 1.0, 30), row(2, 20, 2.0, 20), row(3, 30, 3.0, 10)];
        by_entity_id.sort_by_key(|(entity_id, _, _, _)| *entity_id);
        let entity_id_order: Vec<u32> = by_entity_id.iter().map(|(id, _, _, _)| *id).collect();

        let by_uid = sort_persistent_item_snapshots_by_uid_v1(vec![
            row(1, 10, 1.0, 30),
            row(2, 20, 2.0, 20),
            row(3, 30, 3.0, 10),
        ]);
        let uid_order: Vec<u32> = by_uid.iter().map(|(id, _, _)| *id).collect();

        assert_ne!(entity_id_order, uid_order, "fixture's entity-id and uid orders coincide -- strengthen it");
    }
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

        // APEX-T3.1.04: generate the boot ID first -- before any durable or
        // externally visible startup work (DB migrations/vacuum, plugin
        // publication, worldgen, persistence, listeners). A failed attempt
        // must not have touched any of that; a retry (a fresh `Server::new`
        // call) generates a new ID rather than reusing a partially-failed one.
        let server_boot_id =
            ServerBootId::generate(&mut OsRandomBytesSourceV1).map_err(Error::from)?;
        info!("Server boot ID: {}", server_boot_id.to_text_v1());

        // `APEX-T4.1-CONTENT-LIVE`: ContentManifest::build had zero live
        // callers before this row (only its own test module constructed
        // one) -- computed ONCE, here, at boot, and cached for the
        // server's whole lifetime. This must never be re-invoked
        // per-connection: `bootstrap_manifest_v1`
        // (`server/src/sys/msg/register.rs`) runs on EVERY client
        // admission and reads this cached value, never recomputes it.
        // `common::content_manifest::build_from_asset_tree_v1`'s own doc
        // names the measured, one-time cost and the scoping decision
        // (default asset tree only, `VELOREN_ASSETS_OVERRIDE` not
        // merged). A walk failure is logged and treated as "content
        // identity absent" (`None`), same discipline
        // `WorldBaselineInputV1`'s other un-derived slots already use --
        // never a reason to fail server boot over a compatibility-check
        // amenity.
        let content_manifest = match common::content_manifest::build_from_asset_tree_v1(format!("{:x}", *common::util::GIT_HASH), vec![], vec![]) {
            Ok(manifest) => {
                info!(files = manifest.files.len(), "content manifest built");
                Some(manifest)
            },
            Err(e) => {
                warn!(?e, "failed to build content manifest (content identity will be absent, not fabricated)");
                None
            },
        };
        let content_protocol_root =
            content_manifest.as_ref().and_then(common::content_manifest::content_protocol_version_v1);

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

        // APEX-T2.5.11: strict deployment compile (policy-file opt-in).
        // Missing policy file = Legacy (byte-identical live behavior);
        // present-but-invalid policy or a failed compile REFUSES startup
        // (the .04a loader-trap rule: never fall back on a broken policy).
        // MUST run before any plugin/asset publication: a Deployed state
        // publishes through the ONE-TIME content generation (T2.5.12),
        // which refuses to layer on prior legacy publication.
        #[cfg(feature = "plugins")]
        let plugin_deployment = {
            let mut plugins_dir = (*common::assets::ASSETS_PATH).clone();
            plugins_dir.push("plugins");
            crate::plugin_deployment_policy::init_plugin_deployment_v1(data_dir, &plugins_dir)
                .map_err(|e| Error::Other(format!("plugin deployment init failed (fail-closed): {e:?}")))?
        };

        // Load plugins before generating the world. Deployed = the server's
        // own manager is built from the SAME verified deployment artifacts
        // through the one-time generation; Legacy = the exact old path.
        //
        // APEX-T2.5.13: this ordering IS the step's contract — in Deployed
        // mode the complete plan is compiled and the sealed content
        // generation installed BEFORE `World::generate` runs below, and any
        // failure aborts startup. Worldgen therefore always observes a
        // frozen, complete content generation, never a prefix (permutation
        // invariance of the compile itself is proven in
        // common-state::plugin::deployment tests; the canonical fold order
        // is DET-AST-034's sort). The world-baseline fixture ("discovery
        // permutations yield same world baseline") belongs to the VM
        // fixture lane.
        #[cfg(feature = "plugins")]
        let plugin_mgr = match &plugin_deployment {
            crate::plugin_deployment_policy::PluginDeploymentStateV1::Deployed {
                summary,
                server_runtime_limits,
                server_artifact_paths,
                ..
            } => PluginMgr::from_deployment_paths_v1(
                server_artifact_paths.clone(),
                &summary.requirements.iter().map(|r| (r.ordinal, r.digest)).collect::<Vec<_>>(),
                summary.deployment_root,
                Some(common_state::plugin::module::PluginStoreLimitsV1 {
                    max_linear_memory_bytes: server_runtime_limits.max_linear_memory_bytes,
                    max_fuel_per_event: server_runtime_limits.max_fuel_per_event,
                }),
                Some(server_runtime_limits.max_instances),
                Some(summary.command_owners.iter().cloned().collect()),
                Some(summary.skeleton_owners.iter().cloned().collect()),
            )
            .map_err(|e| Error::Other(format!("deployment plugin batch failed (fail-closed): {e:?}")))?,
            crate::plugin_deployment_policy::PluginDeploymentStateV1::Legacy => {
                PluginMgr::from_asset_or_default()
            },
        };

        debug!("Generating world, seed: {}", settings.world_seed);
        #[cfg(feature = "worldgen")]
        // `APEX-T4-PV`: hoisted into a named local so the worldgen
        // vocabulary can be derived FROM THE ACTUAL OPTIONS this server
        // generated with, rather than from a reconstruction of them.
        // One construction, two consumers -- the same shape as
        // `map_geometry_root` below.
        #[cfg(feature = "worldgen")]
        let world_opts = WorldOpts {
            seed_elements: true,
            world_file: if let Some(ref opts) = settings.map_file {
                opts.clone()
            } else {
                // Load default map from assets.
                FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into())
            },
            calendar: Some(settings.calendar_mode.calendar_now()),
        };
        // `APEX-T4-PV`: the frozen worldgen vocabulary's protocol root.
        // `loaded_map_digest` is None because every live path here is
        // either Generate or LoadAsset -- the asset case identifies its
        // map by PATH (its content rides the content root), so no byte
        // digest is owed. A future operator-supplied map FILE would owe
        // one, and `map_source_is_honest()` is what detects its absence.
        #[cfg(feature = "worldgen")]
        let worldgen_protocol_root = {
            let gen_opts = world_opts.world_file.gen_opts().unwrap_or_default();
            world::apex_worldgen_vocabulary::WorldgenVocabularyV1::from_opts_v1(
                settings.world_seed,
                &world_opts.world_file,
                &gen_opts,
                world_opts.seed_elements,
                world_opts.calendar.as_ref(),
                None,
            )
            .protocol_root_v1()
            .ok()
        };
        #[cfg(feature = "worldgen")]
        let (world, index) = World::generate(
            settings.world_seed,
            world_opts,
            &pools,
            &|stage| {
                report_stage(ServerInitStage::WorldGen(stage));
            },
        );
        #[cfg(not(feature = "worldgen"))]
        let (world, index) = World::generate(settings.world_seed);

        #[cfg(feature = "worldgen")]
        let map = world.get_map_data(index.as_index_ref(), &pools);
        // `APEX-T4.3`: computed here, right after `map` is built and
        // before it's moved into an ECS resource below (`RtSim::new`,
        // called much later, cannot borrow it by then) -- one
        // computation, two consumers (this root, and `map` itself for
        // the real bootstrap send).
        #[cfg(feature = "worldgen")]
        let map_geometry_root = common_net::msg::world_msg::world_map_geometry_root_v1(&map);
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
            // APEX (feature-invariance): the argument is unconditional;
            // only its value is feature-gated.
            {
                #[cfg(feature = "plugins")]
                {
                    common_state::StatePluginsV1::new(plugin_mgr)
                }
                #[cfg(not(feature = "plugins"))]
                {
                    common_state::StatePluginsV1::none()
                }
            },
        )
        .map_err(|e| Error::Other(format!("state construction failed: {e:?}")))?;
        events::register_event_busses(state.ecs_mut());
        // APEX-T3.1.05: same Copy value as the Server field above, inserted
        // once here and never mutated afterward (systems read it via
        // ReadExpect<ServerBootId>, never write it).
        state.ecs_mut().insert(server_boot_id);
        // `APEX-T4.1-CONTENT-LIVE`: the content protocol root computed
        // ONCE above, inserted once here alongside `ServerBootId` --
        // `bootstrap_manifest_v1` (`server/src/sys/msg/register.rs`)
        // reads this via `ReadExpect` on every client admission rather
        // than recomputing the asset-tree walk per connection.
        state.ecs_mut().insert(content_protocol_root);
        // `APEX-T4.2` chunk B: the per-boot bootstrap freshness minter,
        // inserted once here alongside `ServerBootId` (never reset, never
        // reinserted -- one minter per process, same lifetime as the boot
        // ID it chains manifests to).
        state.ecs_mut().insert(crate::bootstrap_freshness_minter::BootstrapFreshnessMinterV1::default());
        // APEX-T3.2: memory-only, empty on every fresh process (canary
        // SES-105) -- inserted once here alongside ServerBootId, never
        // persisted/reloaded from a save.
        state.ecs_mut().insert(crate::session_registry::SessionRegistry::new());
        // APEX-T3.3.11: memory-only outbox, empty on every fresh process --
        // nothing enqueues into it yet (no producer migration has happened;
        // T3.3.13/14) and nothing drains it yet (no SemanticEgressSysV1;
        // T3.3.15).
        state.ecs_mut().insert(crate::semantic_net::outbox::ServerSemanticOutboxV1::new());
        // APEX-T3.3.18: memory-only, redacted-by-construction ingress
        // counters (keyed by (terminal/reject code, physical stream)
        // only) -- process lifetime, never persisted.
        state.ecs_mut().insert(common_net::msg::envelope::SemanticIngressMetricsV1::new());
        // APEX-T5.1: cohort membership and its per-cohort report counters.
        state.ecs_mut().insert(crate::physics_cohort::PhysicsCohortRegistryV1::new());
        state.ecs_mut().insert(crate::physics_cohort::PhysicsCohortMetricsV1::new());
        #[cfg(feature = "plugins")]
        state.ecs_mut().insert(plugin_deployment);
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
        // bastion (READBACK-PREREG.md): the next-tick mine readback queue.
        // MANUALLY inserted: `Write<T: Default>`'s auto-setup does NOT run
        // under this dispatcher -- the first pitread leg panicked in shred
        // ("Tried to fetch resource ... MineReadbackQueue ... does not
        // exist") one tick after boot, so the auto-insert assumption is
        // MEASURED false here, not merely doubted.
        state
            .ecs_mut()
            .insert(bastion_jobs::MineReadbackQueue::default());
        // Same measured lesson, second instance (chain11 cookdiag panicked
        // identically on PendingSeedItems one leg after the resource split).
        state
            .ecs_mut()
            .insert(bastion_jobs::PendingSeedItems::default());
        state
            .ecs_mut()
            .insert(bastion_jobs::TradePriceBook::default());
        state.ecs_mut().insert(bastion_jobs::DivineFavor::default());
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
            // ROW #89 DIAGNOSTIC (registered 80d5923470): 12 generator threads
            // + 8 serializer + rayon's n contend with the ONE main tick loop for
            // 16 cores at boot. That is the only surviving candidate for the
            // measured 1.27-1.54x early-boot tick-rate variation, after every
            // client-side hypothesis was excluded. Pinning to 1 makes chunk
            // generation ~12x slower in wall time -- this is a DIAGNOSTIC ARM,
            // never a candidate fix.
            if std::env::var_os("BASTION_PIN_CHUNKGEN").is_some() {
                pool.configure("CHUNK_GENERATOR", |_| 1);
                tracing::warn!(
                    "bastion: row89 CHUNK_GENERATOR PINNED to 1 thread —                      diagnostic arm, generation is deliberately serialised"
                );
            } else {
                pool.configure("CHUNK_GENERATOR", |n| n / 2 + n / 4);
            }
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

        // `APEX-T4.6` chunk 3b: the save-universe epoch commit's own
        // character-DB staging needs `db_dir` (via `VACUUM INTO` on its
        // OWN read-only connection, per the orchestrator's connection-
        // discipline requirement -- never `CharacterUpdater`'s
        // connection). Reuses the SAME `Arc` already handed to
        // `CharacterUpdater`/`CharacterLoader` below, not a new type;
        // `rtsim/tick.rs`'s periodic-save system reads it as its own
        // `SystemData` field.
        state
            .ecs_mut()
            .insert(Arc::<RwLock<DatabaseSettings>>::clone(&database_settings));

        let ability_map = comp::item::tool::AbilityMap::<comp::AbilityItem>::load_expect_cloned(
            "common.abilities.ability_set_manifest",
        );
        state.ecs_mut().insert(ability_map);

        let msm = comp::inventory::item::MaterialStatManifest::load().cloned();
        state.ecs_mut().insert(msm);

        let rbm = common::recipe::RecipeBookManifest::load().cloned();
        state.ecs_mut().insert(rbm);

        // T0.72: the admission barrier's watcher set. Constructed once
        // here (after the manifests above are already warm in the asset
        // cache) so its own `.load()` calls are cache hits, not fresh
        // parses.
        state
            .ecs_mut()
            .insert(crate::content_epoch::ContentWatchers::new());

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
        // #96: the line above reported ONE gate, and item 15 was voided by a
        // DIFFERENT one. `BASTION_FLAT_ARENA_WALLED=1` produced no wall in the
        // world even though the attestation proved the var reached the process
        // and two unit tests proved `wall_cells()` emits a closed hollow ring —
        // because both tests call `wall_cells()` DIRECTLY and neither exercises
        // the `walled()` gate or the write into a chunk. A green gate beside an
        // inert feature is precisely what this emit exists to catch, so it now
        // carries every arena gate and, crucially, the CELL COUNTS.
        //
        // The counts are what make it diagnostic rather than merely descriptive:
        // `feature_cells` is the total `resourced_feature_cells` will try to
        // write, and `wall_cells` is the ring's share of it. So
        //   walled=false                  -> the GATE is the bug
        //   walled=true, wall_cells=0     -> the generator is the bug
        //   walled=true, wall_cells>0     -> the WRITE is the bug, and the
        //                                    survey that found no rock at the
        //                                    ring is pointing at chunk.set
        // Three outcomes, distinguished before a single further run.
        #[cfg(feature = "worldgen")]
        {
            // #99: the COUNTS are computed only when the arena is on. They are
            // the expensive half — `resourced_feature_cells` builds a Vec of
            // every feature cell — and on an ordinary launch that Vec is
            // fixture geometry for a fixture that is not running. Cheap at
            // boot, but computing fixture geometry in a non-fixture launch is
            // exactly the kind of thing a later reader mistakes for meaning.
            //
            // The FLAGS stay unconditional. That is the whole point of this
            // line: it has to self-report on the launch where the operator
            // expected the arena and did not get it, and a line that only
            // prints when the arena worked could never say so.
            //
            // `Option`, not 0: a real zero and a not-computed zero must never
            // render identically — the same rule the hostile-proximity census
            // follows for its uid fields. `None` here means NOT MEASURED.
            let counts = bastion_flat_arena::enabled().then(|| {
                let centre = bastion_flat_arena::world_center_wpos(&world);
                (
                    bastion_flat_arena::resourced_feature_cells(centre).len(),
                    bastion_flat_arena::wall_cells(
                        centre,
                        bastion_flat_arena::WALL_RADIUS,
                        bastion_flat_arena::WALL_HEIGHT,
                    )
                    .len(),
                )
            });
            info!(
                flat_arena_enabled = bastion_flat_arena::enabled(),
                resourced = bastion_flat_arena::resourced(),
                walled = bastion_flat_arena::walled(),
                pit_depth = bastion_flat_arena::pit_depth(),
                shaft_depth = bastion_flat_arena::shaft_depth(),
                feature_cells = ?counts.map(|c| c.0),
                wall_cells = ?counts.map(|c| c.1),
                "bastion: FLAT-TEST-ARENA env check at server boot"
            );
        }
        #[cfg(not(feature = "worldgen"))]
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
            // T0.68: two settlements exactly equidistant from center_chunk
            // previously fell through to whichever `Civs::sites()` (a
            // `Store::values()` iteration, no id exposed at this call
            // site) happened to visit first. The site's own center is a
            // deterministic, already-available tiebreak -- no two
            // settlements share a center, so (x, y) fully disambiguates.
            let spawn_chunk = world
                .civs()
                .sites()
                .filter(|site| site.is_settlement())
                .map(|site| site.center)
                .min_by_key(|site_pos| {
                    common::decision_key::DecisionKeyV1::nearest(
                        (),
                        site_pos.distance_squared(center_chunk),
                        (site_pos.x, site_pos.y),
                        (0i32, 0i32),
                    )
                })
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
                map_geometry_root,
                worldgen_protocol_root,
                content_protocol_root,
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

            server_boot_id,
        };

        // bastion (B-ASSET1): the --asset-arena test chamber. Inert unless
        // the BASTION_ASSET_ARENA env var is set (voxygen sets it).
        #[cfg(feature = "worldgen")]
        this.bastion_arena_init_from_env();

        debug!(?settings, "created veloren server with");

        info!("Server version: {}", *common::util::DISPLAY_VERSION);

        // driver-12 follow-on (2026-08-09, Opus/Fable "ports shipment"):
        // always-on, boot-time-only -- distinct from the gated periodic
        // BASTION_DECAY_JOIN_DIAG emit in bastion_jobs.rs. Every live run
        // now reports which mood config actually loaded without needing a
        // special env var, closing the "wrong-tree asset resolution"
        // hypothesis class for good (that hypothesis took a harness-only
        // re-derivation to rule out for driver-12; this makes it a one-line
        // log read instead).
        {
            let mood_cfg = common::bastion::MoodConfig::current();
            info!(
                hunger_decay_per_sec = mood_cfg.hunger.decay_per_sec,
                rest_decay_per_sec = mood_cfg.rest.decay_per_sec,
                "bastion effective mood config"
            );
            // Item 8 (endurance run) pre-flight, Opus's ruling: "effective
            // config is EMITTED, not inferred -- needs rates, thresholds."
            // The line above had the decay rates but never the interrupt
            // thresholds those rates are measured against -- without both,
            // a run's own crossing times can't be read from its log at
            // all, only guessed at from the RON default (per-colonist
            // stagger further adjusts this base value; this is the
            // baseline the packet asks be named from live config, not
            // memory).
            info!(
                hunger_interrupt = mood_cfg.hunger.interrupt,
                hunger_comfort = mood_cfg.hunger.comfort,
                rest_interrupt = mood_cfg.rest.interrupt,
                rest_comfort = mood_cfg.rest.comfort,
                recreation_interrupt = mood_cfg.recreation.interrupt,
                recreation_comfort = mood_cfg.recreation.comfort,
                "bastion effective mood config: need interrupt/comfort thresholds"
            );
        }

        // ROW-ITEM6-WITNESS-PACKET part A1 (Opus's second flag,
        // 2026-08-10): forced here, unconditionally, at boot -- the
        // OnceLock-cached accessors otherwise only log on FIRST use inside
        // the F3 pass, which in practice fires in every run that reaches
        // arbitration but is not GUARANTEED to on every code path. A run
        // whose effective config can't be reconstructed after the fact is
        // a void run for calibration purposes; forcing the read (and the
        // log) here makes the line unconditional, same reasoning as the
        // mood-config log immediately above.
        let _ = bastion_jobs::access_stale_secs();
        let _ = bastion_jobs::access_stall_secs();

        // ITEM8-V4 (checklist entry 5, "effective config emitted, not
        // inferred"): F6's threshold is DERIVED (from access_stall_secs,
        // not itself env-tunable), so it doesn't go through
        // env_threshold_secs_or_refuse's own unconditional log -- emitted
        // here instead, same "at boot, unconditionally" discipline as
        // the mood config above, so a killed server's log still carries
        // the value a future reader would otherwise have to recompute.
        info!(
            generic_claim_leak_secs = bastion_jobs::generic_claim_leak_secs(),
            colony_terminal_zero_streak_samples = bastion_jobs::COLONY_TERMINAL_ZERO_STREAK_SAMPLES,
            "bastion effective ITEM8-V4 config (F6 backstop threshold, sentinel S1 streak)"
        );

        Ok(this)
    }

    /// APEX-T3.1: this live server-process incarnation's boot identity.
    /// Public (not secret), not proof of authentication, excluded from
    /// authoritative simulation state/RNG keys, never persisted.
    pub fn server_boot_id(&self) -> ServerBootId { self.server_boot_id }

    pub fn get_server_info(&self) -> ServerInfo {
        let settings = self.state.ecs().fetch::<Settings>();

        ServerInfo {
            server_boot_id: self.server_boot_id,
            name: settings.server_name.clone(),
            git_hash: *common::util::GIT_HASH,
            git_timestamp: *common::util::GIT_TIMESTAMP,
            auth_provider: settings.auth_server_address.clone(),
            supported_semantic_protocols: common_net::msg::server_supported_semantic_protocols_v1(),
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
        let names = self
            .state
            .ecs()
            .write_resource::<rtsim::RtSim>()
            .bastion_spawn_colony(wpos, count);
        self.bastion_found_colony_seed_stock(wpos);
        self.bastion_found_colony_presence(wpos);
        names
    }

    /// Determinism-capture founding: seed the colony from an explicit tick
    /// (see [`rtsim::RtSim::bastion_spawn_colony_seeded`]).
    pub fn bastion_spawn_colony_seeded(
        &mut self,
        wpos: Vec3<f32>,
        count: u8,
        seed_tick: u64,
    ) -> Vec<String> {
        let names = self
            .state
            .ecs()
            .write_resource::<rtsim::RtSim>()
            .bastion_spawn_colony_seeded(wpos, count, seed_tick);
        self.bastion_found_colony_seed_stock(wpos);
        self.bastion_found_colony_presence(wpos);
        names
    }

    /// bastion (#105, DECISIONS-FOR-BEN: FOUNDING SEED STOCK): the shared
    /// half of colony founding both spawn paths above call -- a persistent
    /// loose drop (same mechanism as a player's own `/dropall true`, item
    /// 6's own instrument) so it becomes eligible for the B6 haul-to-
    /// stockpile pipeline the moment a stockpile is designated nearby.
    /// Deliberately at the Server layer, not inside `RtSim`: RtSim has no
    /// ECS item-drop/event-bus access, and every founding that reaches the
    /// Server API (live or determinism-capture) should carry the same
    /// starting stock -- one entry point, not a live-only special case.
    ///
    /// TWO PRODUCERS BY DESIGN, NOT A DOUBLE-FIRE RISK: this one, and the
    /// twin at `sys/msg/in_game.rs`'s `rtsim.bastion_spawn_colony` call
    /// site (the live `BastionSpawnColony` client-message path, which
    /// bypasses this method entirely -- see that call site's own doc for
    /// why the fix originally missed it). The two entry points are
    /// disjoint by construction: a live client founding is handled
    /// entirely inside that system and never reaches `Server::
    /// bastion_spawn_colony`/`_seeded`; every OTHER caller of THIS method
    /// -- the harness's ~60 scenario call sites, `bastion_arena.rs`'s
    /// "fixture" staging spawn (a live but non-client-message admin path),
    /// determinism-capture code -- is a direct Rust call that never goes
    /// through the client-message system at all. One founding call, one
    /// path, one producer -- grep both sites before assuming otherwise.
    fn bastion_found_colony_seed_stock(&mut self, wpos: Vec3<f32>) {
        self.bastion_spawn_item(
            wpos,
            bastion_jobs::FARM_SEED_ITEM,
            bastion_jobs::FOUNDING_SEED_STOCK,
        );
    }

    /// bastion (ROW-COLONY-PRESENCE, DECISIONS #106): mints the
    /// server-owned `PresenceKind::Colony` that keeps founded colonists in
    /// `SimulationMode::Loaded` with no client connected -- the finding
    /// item 8's endurance run surfaced (colonists demoted to `Simulated`
    /// ~20s after every founding to date, invisible only because every
    /// prior live leg happened to have a client present the whole time).
    ///
    /// View distance = 1 chunk (a 3x3 chunk block, 96x96 blocks centered on
    /// the founding position): every designation this arc has ever painted
    /// (stockpile/farm/bed, each a handful of blocks) sits well inside a
    /// single 32-block chunk of the founding point, so radius 1 already
    /// carries comfortable margin for haul paths beyond the plots
    /// themselves. Kept at the SMALLEST nonzero radius deliberately -- the
    /// packet's own debt note: held chunks per colony are bounded for one
    /// colony but grow UNBOUNDED across many, and item 40 (multi-colony)
    /// is where that cost gets revisited, not here.
    ///
    /// Same "two producers by design" shape as
    /// `bastion_found_colony_seed_stock` above, called from the same two
    /// call sites for the same reason (the live `BastionSpawnColony`
    /// message bypasses this method entirely) -- see that method's doc.
    pub(crate) fn bastion_found_colony_presence(&mut self, wpos: Vec3<f32>) {
        #[cfg(feature = "worldgen")]
        {
            // ★ VIEW DISTANCE IS NOW A KNOB (BASTION_COLONY_PRESENCE_VD).
            //
            // 1 is a 3x3 chunk area, which loads NINE chunks and then nothing
            // ever again -- measured: a headless run promoted 9 chunks across 3
            // ticks and stayed flat for the remaining 7,100. That is a real
            // determinism result on a trivial sample, and a trivial sample is
            // exactly how a vacuous green happens.
            //
            // A client uses 6 (13x13 = 169 chunks), so raising this makes the
            // headless arm a COMPARABLE exercise to the driven one instead of a
            // token one. Default 1 keeps every existing run byte-identical.
            let colony_presence_vd: u32 = std::env::var("BASTION_COLONY_PRESENCE_VD")
                .ok()
                .and_then(|v| v.parse().ok())
                .filter(|&v| v > 0)
                .unwrap_or(1);
            #[allow(non_snake_case)]
            let COLONY_PRESENCE_VIEW_DISTANCE: u32 = colony_presence_vd;
            self.state
                .create_colony_presence(
                    comp::Pos(wpos),
                    COLONY_PRESENCE_VIEW_DISTANCE,
                    &self.world,
                    &self.index,
                )
                .build();
        }
        #[cfg(not(feature = "worldgen"))]
        let _ = wpos;
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

    /// bastion (AUTON-2 unification, site 4/6, Fixture 2 cross-
    /// attribution test, harness hook, 2026-08-09): a PRE-CLAIMED
    /// Despond job at a chosen `until` (the `bastion_assign_rest`
    /// pattern, `insert_despond_job` instead of `insert_rest_job`) --
    /// bypasses the mood-driven breakdown roll entirely, so a
    /// determinism-by-construction test can put TWO colonists into
    /// distinct, KNOWN breakdown deadlines without depending on RNG
    /// timing to line them up.
    pub fn bastion_force_despond(&mut self, name: &str, until: f64) -> bool {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let uids = ecs.read_storage::<common::uid::Uid>();
        let positions = ecs.read_storage::<comp::Pos>();
        let mut active_jobs = ecs.write_storage::<comp::bastion::ActiveJob>();
        let mut board = ecs.write_resource::<bastion_jobs::JobBoard>();
        for (e, c, uid, pos) in (&entities, &colonists, &uids, &positions).join() {
            if c.0.name == name {
                if active_jobs.contains(e) {
                    return false;
                }
                let feet = pos.0.map(|v| v.floor() as i32);
                let id = board.insert_despond_job(feet, *uid, until);
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
        let (entity, time, uid) = {
            let colonists = ecs.read_storage::<comp::Colonist>();
            let entities = ecs.entities();
            let uids = ecs.read_storage::<common::uid::Uid>();
            let time = *ecs.read_resource::<common::resources::Time>();
            let Some(entity) = (&entities, &colonists)
                .join()
                .find(|(_, c)| c.0.name == name)
                .map(|(e, _)| e)
            else {
                return false;
            };
            (entity, time, uids.get(entity).copied())
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
                    // T0.85 (E5-B): the root-cause fix retires the
                    // hardcoded constant this comment used to explain --
                    // common::combat::next_attack_instance() (a process-
                    // global counter) made N6 nondeterministic across
                    // identical runs (the x2 comparator caught it), worked
                    // around here with a fixed value instead of fixing the
                    // counter itself. Now genuinely deterministic
                    // (world-scoped derivation from the target's own uid +
                    // sim time), so the workaround is no longer needed.
                    instance: uid.map_or(0, |uid| {
                        common::combat::derive_attack_instance("server/bastion-emit-damage", None, uid, time, 0)
                    }),
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

    /// bastion (design-review diagnostic, 2026-07-30, Fable-directed): the
    /// AUTON-0 drive gate discriminator -- ALL job claiming/execution is
    /// gated on `arb.current == Drive::Work`, and the Idle->Work
    /// transition requires `work_available` (an unclaimed, non-unreachable
    /// job exists). A colony whose only jobs are marked `unreachable`
    /// never enters Work at all -- not "won't claim," "won't wake."
    /// Returns `(drive as debug string, work_score, flee_score,
    /// idle_score)` so a scenario can sample whether the colonist ever
    /// reaches Work, and if not, whether the work urgency itself is zero
    /// (a signal problem) or nonzero-but-losing (a hysteresis/threshold
    /// problem).
    pub fn bastion_colonist_drive_scores(&self, name: &str) -> Option<(String, f32, f32, f32)> {
        use specs::Join;
        let ecs = self.state.ecs();
        let entities = ecs.entities();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let arbiters = ecs.read_storage::<comp::bastion::Arbiter>();
        (&entities, &colonists, &arbiters)
            .join()
            .find(|(_, c, _)| c.0.name == name)
            .map(|(_, _, a)| {
                let (w, f, i) = a.last_scores;
                (format!("{:?}", a.current), w, f, i)
            })
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
        if let Some((id, job)) = board
            .jobs
            .iter()
            .filter(|(_, j)| {
                j.pos.x == cell.x && j.pos.y == cell.y && (j.pos.z - cell.z).abs() <= 6
            })
            .min_by_key(|(_, j)| (j.pos.z - cell.z).abs())
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
                blocked_by: board.blocked_by(job.pos),
                benched_since_tick: board.benched_since.get(id).copied(),
                // ROW B′: a direct field read off `job`, no lookup --
                // cheaper than Row B's HashMap probe it replaces.
                benched_until_tick: job.benched_until_tick,
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

    /// bastion (AUTON-2 unification, FIXTURE 2, harness hook,
    /// 2026-08-08): current sim seconds (`common::resources::Time`).
    /// MEASURED reason this exists: the live Despond job is visible for
    /// too short a window to catch by polling between ticks -- the
    /// eat-carve-out's own precondition (hunger already below
    /// interrupt, structurally required to drop mood below
    /// `break_minor` in the first place) makes it eligible to fire on
    /// the very next arbitration pass after the job is created, and
    /// per-tick polling still missed it. `until` is deterministic
    /// (`time_at_roll + despond_secs`), so reading the sim clock at the
    /// roll and computing the expectation analytically sidesteps the
    /// race entirely rather than trying to win it.
    pub fn bastion_sim_time(&self) -> f64 {
        self.state.ecs().read_resource::<common::resources::Time>().0
    }

    /// bastion (AUTON-2 unification, FIXTURE 2 "DESPOND-RESUME
    /// DETERMINISM", harness hook, 2026-08-08): the `until` deadline of
    /// the named colonist's live Despond job, if one exists. Read-only.
    /// The fixture's own falsifier requires this byte-identical across a
    /// resume ("same deadline, no re-roll") -- reads the raw `f64`, not
    /// a rounded/derived form, so a re-roll's ULP drift is visible.
    pub fn bastion_despond_until(&self, uid: u64) -> Option<f64> {
        use common::uid::Uid;
        let target = Uid(std::num::NonZeroU64::new(uid)?);
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        // bastion (#101): well-defined only if a colonist holds AT MOST ONE live
        // `Despond` job. `insert_despond_job` does NO de-duplication — it
        // allocates and inserts unconditionally — so the uniqueness is ENTIRELY
        // the callers': the harness hook refuses when the colonist already holds
        // an active job, and the live path creates from `PendingNeed::Despond`
        // behind the arbiter's `pending_self_job`, a single `Option<JobId>`.
        // ★ Both are arguments about code ELSEWHERE. This turns them into a test.
        debug_assert!(
            board
                .jobs
                .values()
                .filter(|j| matches!(j.kind, common::bastion::JobKind::Despond { .. })
                    && j.claimed_by == Some(target))
                .count()
                <= 1,
            "bastion_despond_until: colonist {target:?} holds MORE THAN ONE live Despond job — \
             `find_map` returns whichever the hasher visited first. The callers are supposed to \
             make this impossible (`pending_self_job` is a single Option); if this fires, that is \
             a JobBoard bug and NOT something a sort would fix."
        );
        board.jobs.values().find_map(|j| match j.kind {
            common::bastion::JobKind::Despond { until } if j.claimed_by == Some(target) => {
                Some(until)
            },
            _ => None,
        })
    }

    /// bastion (AUTON-2 unification, site 4/6, Fixture 2 cross-
    /// attribution test, harness hook, 2026-08-09): `uid`'s Despond
    /// deadline whether ACTIVELY held OR SUSPENDED (`suspended_for`) --
    /// distinct from `bastion_despond_until` above, which only sees the
    /// active case. Reads by uid EXPLICITLY (never "nearest" or "any"),
    /// which is the whole property a cross-attribution test needs to
    /// see hold: colonist A's query can never surface colonist B's job,
    /// because nothing here scans without matching the specific uid.
    pub fn bastion_despond_until_any(&self, uid: u64) -> Option<f64> {
        use common::uid::Uid;
        let target = Uid(std::num::NonZeroU64::new(uid)?);
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        // bastion (#101): the STRICTLY BROADER predicate of the pair — it matches
        // on `claimed_by` OR `suspended_for`, so it is the more exposed of the
        // two and can match twice even where the sibling matches once (one
        // claimed job plus one suspended job for the same colonist).
        //
        // ★★ The existing comment here defends a DIFFERENT property: "colonist
        // A's query can never surface colonist B's job". True, and about
        // cross-colonist leakage — it says nothing about ONE colonist matching
        // TWICE, which is the hazard `find_map` actually has. A comment that
        // answers an adjacent question reads, at a glance, as though the
        // question were settled.
        debug_assert!(
            board
                .jobs
                .values()
                .filter(|j| matches!(j.kind, common::bastion::JobKind::Despond { .. })
                    && (j.claimed_by == Some(target) || j.suspended_for == Some(target)))
                .count()
                <= 1,
            "bastion_despond_until_any: colonist {target:?} matches MORE THAN ONE live Despond \
             job across the claimed/suspended union — `find_map` returns whichever the hasher \
             visited first. Broader predicate than the sibling, so this can fire where that one \
             does not."
        );
        board.jobs.values().find_map(|j| match j.kind {
            common::bastion::JobKind::Despond { until }
                if j.claimed_by == Some(target) || j.suspended_for == Some(target) =>
            {
                Some(until)
            },
            _ => None,
        })
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

    /// bastion (mechanism-2 terrain-reachability probe, harness hook,
    /// 2026-07-30): OFFLINE (unbounded-budget), READ-ONLY answer to
    /// "does a walkable route exist from `from` to a valid standing
    /// position adjacent to `target`" -- see
    /// [`bastion_jobs::offline_reachability_probe`] for the full design
    /// rationale, including the corrected three-tier (step/jump/scramble)
    /// model matching the live pathfinder's actual `neighbors` fn, and the
    /// `probe_incomplete` discipline. Flattened to a plain tuple (matching
    /// this file's other harness-hook shapes): `(standable_target,
    /// path_exists_step, path_exists_jump, path_exists_scramble,
    /// probe_incomplete, columns_visited_step, columns_visited_jump,
    /// columns_visited_scramble)`.
    pub fn bastion_offline_reachability_probe(
        &self,
        from: vek::Vec3<i32>,
        target: vek::Vec3<i32>,
        node_cap: usize,
    ) -> (
        Option<vek::Vec3<i32>>,
        bool,
        bool,
        bool,
        bool,
        u32,
        u32,
        u32,
    ) {
        let ecs = self.state.ecs();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        let r = bastion_jobs::offline_reachability_probe(&terrain, from, target, node_cap);
        (
            r.standable_target,
            r.path_exists_step,
            r.path_exists_jump,
            r.path_exists_scramble,
            r.probe_incomplete,
            r.columns_visited_step,
            r.columns_visited_jump,
            r.columns_visited_scramble,
        )
    }

    /// bastion (chop-oracle ground-truth audit, harness hook, 2026-07-30):
    /// does a trunk-plus-canopy (Wood with Leaves above it, same column)
    /// physically exist anywhere in this XY footprint, independent of
    /// [`Self::bastion_place_chop_area`]'s own detection pipeline -- see
    /// [`bastion_chop::detect_trees_ground_truth`] for why it must not
    /// reuse that pipeline's candidate/validity gates, and why the
    /// predicate is Wood-then-Leaves rather than either block alone.
    /// Flattened to a plain tuple (matching this file's other harness-hook
    /// return shapes) rather than the crate-internal enum, so callers
    /// outside `bastion-server` don't need that type's path:
    /// `(witness (wood_pos, leaves_pos) on a hit, unreachable_columns,
    /// total_columns)`. `unreachable_columns > 0` means the scan could not
    /// examine part of the footprint (unloaded terrain / no altitude
    /// sample) -- "couldn't look," distinct from "looked, found nothing."
    pub fn bastion_chop_ground_truth(
        &self,
        min_xy: vek::Vec2<i32>,
        max_xy: vek::Vec2<i32>,
    ) -> (Option<(vek::Vec3<i32>, vek::Vec3<i32>)>, u32, u32) {
        let ecs = self.state.ecs();
        let world = ecs.read_resource::<Arc<World>>();
        let index = ecs.read_resource::<IndexOwned>();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        match bastion_chop::detect_trees_ground_truth(&world, &index, &terrain, min_xy, max_xy) {
            bastion_chop::TreeGroundTruthOutcome::Found(w) => {
                (Some((w.wood_pos, w.leaves_pos)), 0, 0)
            },
            bastion_chop::TreeGroundTruthOutcome::NotFound => (None, 0, 0),
            bastion_chop::TreeGroundTruthOutcome::ScanIncomplete {
                unreachable_columns,
                total_columns,
            } => (None, unreachable_columns, total_columns),
        }
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
                // ITEM 14: guarding reads the MELEE skill, matching
                // `ColonistSkills::level_for` — the two must agree or a
                // harness query and the arbitration gate would disagree about
                // the same colonist's competence.
                common::bastion::WorkType::Guard => s.melee,
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
        let uids = ecs.read_storage::<common::uid::Uid>();
        let mut positions = ecs.write_storage::<comp::Pos>();
        let mut velocities = ecs.write_storage::<comp::Vel>();
        let mut healths = ecs.write_storage::<comp::Health>();
        let mut moods = ecs.write_storage::<comp::bastion::Mood>();
        let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
        // The harness hook must feed the SAME bus the live caller does —
        // "the tested path IS the shipping path" is this function's own rule,
        // and damage that skips the bus skips death.
        let health_events =
            ecs.read_resource::<common::event::EventBus<common::event::HealthChangeEvent>>();
        let victims = bastion_jobs::cavein_eject_and_injure(
            &cells,
            &terrain,
            time,
            &entities,
            &colonists,
            &uids,
            &mut positions,
            &mut velocities,
            &mut healths,
            &health_events,
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

    /// CARVE-CASCADE PROBE (mechanism 1, predictions A/B — Opus,
    /// 2026-07-30): `(frontier_completes_max, abort_resets_max,
    /// abort_ceiling_max, access_emissions_max, members_seen)`.
    ///
    /// **Read the RESETS against the CEILING, never either alone.** A low
    /// ceiling with HIGH resets is the cascade signature: the per-episode
    /// bound is satisfied at every step because `frontier-complete` keeps
    /// clearing it. A low ceiling with ZERO resets is a genuinely healthy
    /// run. Reporting only the ceiling would call both of those healthy,
    /// which is why prediction B measured as a ceiling is a false
    /// all-clear.
    ///
    /// `members_seen` is a TRUE COUNT: it unions all four maps, so a member
    /// that hit an abort without completing a frontier or emitting a plan is
    /// still counted. (It chained only two maps until 2026-07-30 and was a
    /// silent lower bound — undercounting precisely the members whose
    /// bound-1 behaviour this probe exists to observe.)
    ///
    /// The four *maxima* remain four INDEPENDENT maxima and need not describe
    /// the same member; they answer "was there a big cascade somewhere", not
    /// "was it one cascade". Per-member rows are required before the A/B
    /// reading table can be evaluated — see
    /// `readme/CARVE-CASCADE-DIAGNOSIS-seed61.md`.
    ///
    /// Maxima folded over members in SORTED order so no hash-iteration
    /// order reaches the output. Pure telemetry; `JobBoard` feeds no
    /// canonical hash (checked), so this cannot move the 72/72
    /// determinism baseline.
    pub fn bastion_cascade_probe(&self) -> (u32, u32, u32, u32, u32) {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        let fold_max = |m: &hashbrown::HashMap<common::uid::Uid, u32>| -> u32 {
            let mut vals: Vec<(u64, u32)> = m.iter().map(|(u, v)| (u.0.get(), *v)).collect();
            vals.sort_unstable();
            vals.into_iter().map(|(_, v)| v).max().unwrap_or(0)
        };
        // `members_seen` unions ALL FOUR maps. Chaining only completes and
        // emissions (as this did) makes any member that reached an ABORT
        // without ever completing a frontier or emitting a plan invisible —
        // i.e. it undercounts exactly the members whose bound-1 behaviour the
        // carve-cascade row studies, turning a count into a silent lower
        // bound. Reported by the harness as a count, so it must be one.
        let mut members: Vec<u64> = board
            .cascade_frontier_completes
            .keys()
            .chain(board.cascade_abort_resets.keys())
            .chain(board.cascade_abort_max.keys())
            .chain(board.cascade_access_emissions.keys())
            .map(|u| u.0.get())
            .collect();
        members.sort_unstable();
        members.dedup();
        (
            fold_max(&board.cascade_frontier_completes),
            fold_max(&board.cascade_abort_resets),
            fold_max(&board.cascade_abort_max),
            fold_max(&board.cascade_access_emissions),
            members.len() as u32,
        )
    }
    /// bastion (mechanism-2 friction instrument, harness hook, 2026-07-30):
    /// the TAIL signature -- the highest travel-timeout count any single
    /// job POSITION accumulated this run. A target retried many times that
    /// never resolves reads high here even if `travel_timeouts` (the raw
    /// total) is unremarkable; ambient one-off friction spread across many
    /// different targets does not. Always-on counter, read-only, no world
    /// writes -- cannot re-roll a seed's outcome.
    pub fn bastion_max_same_target_timeouts(&self) -> u32 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .timeout_counts_by_pos
            .values()
            .max()
            .copied()
            .unwrap_or(0)
    }

    /// bastion (mechanism-2 friction instrument, harness hook, 2026-07-30):
    /// travel-timeout count for ONE specific job position -- lets the
    /// harness compute the ATTRIBUTION metric Fable's fan data called for
    /// (magnitude alone doesn't discriminate pass/fail; "did the job THIS
    /// timeout fired on ultimately complete" might). 0 if this position
    /// never timed out.
    pub fn bastion_timeout_count_for_pos(&self, job_pos: vek::Vec3<i32>) -> u32 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .timeout_counts_by_pos
            .get(&job_pos)
            .copied()
            .unwrap_or(0)
    }

    /// bastion (BUILD-INSTRUMENT-SPEC/CHOP-INSTRUMENT-SPEC, harness hook,
    /// 2026-08-08): `(required_item, has_reservation)` for the job at an
    /// EXACT position, if one exists there. `BastionInspectKind::Job`
    /// (the general-purpose inspect payload, also client-facing) doesn't
    /// carry these two `Job` fields; both specs ask for them "already
    /// fields on `Job` -- no new engine state, a JOIN" -- this is that
    /// join's missing half, added as its own minimal getter rather than
    /// widening the shared inspect struct's shape. Exact-position match
    /// (not `bastion_inspect_cell`'s nearby-column search): every call
    /// site knows the job's own designated position precisely (mine
    /// cells, chop bases, build slots are all placed at literal
    /// coordinates the caller already has). Read-only, no world writes.
    pub fn bastion_job_material_info(
        &self,
        pos: vek::Vec3<i32>,
    ) -> Option<(Option<&'static str>, bool)> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .jobs
            .values()
            .find(|j| j.pos == pos)
            .map(|j| (j.required_item, j.reservation.is_some()))
    }

    /// bastion (AUTON2-STEP1 watchdog-defect trace, harness hook,
    /// 2026-08-08): the numeric `JobId` at an EXACT position, so a caller
    /// can gate `BASTION_SDIST_TRACE_JOB=<id>` on a self-job it doesn't
    /// know the id of in advance (self-jobs like RestAt get a FRESH id
    /// per retry -- there is no static id to hardcode). Same exact-match
    /// pattern as `bastion_job_material_info` (every call site already
    /// knows the job's own designated position). Read-only.
    pub fn bastion_job_id_at(&self, pos: vek::Vec3<i32>) -> Option<u64> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .jobs
            .iter()
            .find(|(_, j)| j.pos == pos)
            .map(|(id, _)| *id)
    }

    /// bastion (AUTON-2 UNIFICATION, FIXTURE 1 "TRY-TO-ORPHAN", harness
    /// hook, 2026-08-08; corrected per Fable DECISIONS #72, 2026-08-09):
    /// the settle invariant, `AUTON2-ACCEPTANCE-FIXTURES.md` -- for
    /// every self-job (`is_labor_hold_self_job`: `RestAt`/`EatFrom`/
    /// `Despond`) with `claimed_by == None`, it is EITHER suspended for
    /// a LIVE owner (`suspended_for`, site 4/6's ownership-across-
    /// release field) OR genuinely orphaned. `claimed_by` alone can't
    /// tell these apart -- it's `None` for both a suspended job (owned,
    /// waiting to reclaim) and a plain orphan; `suspended_for` is what
    /// distinguishes them, same predicate the orphan sweep itself uses
    /// (`bastion_jobs.rs`, the `orphans` filter). Owner liveness here
    /// checks DEATH only (not load-state) -- this function is a READ-
    /// ONLY diagnostic snapshot, never the removal path itself, so an
    /// unloaded-but-alive owner reads as a harmless false-positive here
    /// rather than a dangerous false-negative; the sweep's own removal
    /// decision (which DOES check load-state) is the actual gate.
    /// Returns the violating positions (empty = invariant holds).
    /// Settle-time only (one pass over `board.jobs`), per the fixture's
    /// own budget -- never call this per-tick.
    pub fn bastion_settle_invariant_violations(&self) -> Vec<vek::Vec3<i32>> {
        let ecs = self.state.ecs();
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        let id_maps = ecs.read_resource::<common::uid::IdMaps>();
        let healths = ecs.read_storage::<comp::Health>();
        board
            .jobs
            .values()
            .filter(|j| {
                bastion_jobs::is_labor_hold_self_job(&j.kind)
                    && j.claimed_by.is_none()
                    && {
                        // Death-only owner_alive here (no load-state check)
                        // -- see this function's own doc for why; the
                        // TIE-BREAK logic itself is shared with the orphan
                        // sweep via the same pure predicate.
                        let owner_alive = j.suspended_for.is_some_and(|owner| {
                            id_maps.uid_entity(owner).is_some_and(|e| {
                                !healths.get(e).is_some_and(|h| h.is_dead || h.should_die())
                            })
                        });
                        bastion_jobs::settle_invariant_violation(j.suspended_for, owner_alive)
                    }
            })
            .map(|j| j.pos)
            .collect()
    }

    /// bastion (AUTON-2 unification, FIXTURE 1's invariant LIVE IN
    /// EVERY SCENARIO, harness hook, 2026-08-08): the CUMULATIVE count
    /// (`JobBoard::settle_invariant_violations`, incremented at the
    /// existing orphan sweep's own ~2 Hz cadence -- every scenario that
    /// ticks the server, and production, exercise it automatically, no
    /// per-scenario wiring). Distinct from `bastion_settle_
    /// invariant_violations` above (a live snapshot at ONE instant);
    /// this counts every settle-time pass across the WHOLE run, so a
    /// violation that self-heals before the next harness poll is still
    /// counted. Pre-unification this is EXPECTED to be nonzero in any
    /// scenario exercising a self-job release path (see the field's
    /// own doc) -- becomes the real regression bar (expected 0) once
    /// GUARD-6 lands.
    pub fn bastion_settle_invariant_violation_count(&self) -> u64 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .settle_invariant_violations
    }

    /// bastion (TRAVEL-ROW-SPEC §4.1, harness hook, 2026-08-08): closest
    /// approach ever achieved, for every job position that has incurred at
    /// least one travel timeout -- one value per position in
    /// `timeout_counts_by_pos`, read from `min_distance_to_target` (which
    /// is written for every actively-traveled target, not just ones that
    /// timed out; filtering to the timeout set is what makes this the
    /// travel-failure discriminator rather than a general locomotion
    /// stat). Small ⇒ the colonist got close, so the target was reachable
    /// and travel failed (UNREACHED). Large ⇒ never approached
    /// (UNREACHABLE). §4.2's threshold is derived from this distribution,
    /// not guessed. Read-only, no world writes, zero added hot-path cost
    /// (`min_distance_to_target` is already maintained every tick this
    /// reads at settle).
    /// ORDER (#84, 2026-08-17): iterated in KEY ORDER, not hash order.
    /// `timeout_counts_by_pos` is a `HashMap`, so iterating its keys yields an
    /// unspecified order; this list is serialized into the harness JSON and
    /// `holdcheck` compares lists WHOLE ("a reordered list is a change"), so an
    /// unsorted collect reported a wave MOVER on every permutation with no
    /// content change at all -- measured at 16/48 seeds on wave31->wave32, ALL
    /// of them order-only. Sorting by position makes the sequence a function of
    /// the CONTENT, which is what a baseline has to be.
    pub fn bastion_travel_timeout_min_distances(&self) -> Vec<f32> {
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        let mut keyed: Vec<(vek::Vec3<i32>, f32)> = board
            .timeout_counts_by_pos
            .keys()
            .filter_map(|pos| board.min_distance_to_target.get(pos).copied().map(|d| (*pos, d)))
            .collect();
        // Sorted by POSITION, not by the distance value: two positions can share
        // a distance, and sorting by a non-unique key leaves the tie's order
        // hash-dependent -- the same defect one step further in.
        keyed.sort_unstable_by_key(|(pos, _)| (pos.x, pos.y, pos.z));
        keyed.into_iter().map(|(_, d)| d).collect()
    }

    /// bastion (TRAVEL-ROW-SPEC §4.1, harness hook, 2026-08-08): (job
    /// position, colonist's actual position at that job's most recent
    /// travel timeout) for every position that has incurred at least one
    /// travel timeout. Diagnosis input (lets an offline probe start from
    /// where a failing attempt actually stood, per `last_timeout_pos`'s
    /// own field doc) -- not the classification input, see
    /// `bastion_travel_timeout_min_distances` for that. Read-only.
    pub fn bastion_travel_timeout_last_positions(
        &self,
    ) -> Vec<(vek::Vec3<i32>, vek::Vec3<f32>)> {
        // ORDER (#84, 2026-08-17): sorted by job position -- see
        // `bastion_travel_timeout_min_distances` for the full reasoning. This
        // accessor feeds THREE harness JSON fields (b5_travel_timeout_last_
        // positions, and b5_self_job_reachability_probe via main.rs:3707), so
        // its order escaped into all three; the third was found by the DATA,
        // not by the read.
        let board = self.state.ecs().read_resource::<bastion_jobs::JobBoard>();
        let mut out: Vec<(vek::Vec3<i32>, vek::Vec3<f32>)> = board
            .timeout_counts_by_pos
            .keys()
            .filter_map(|pos| board.last_timeout_pos.get(pos).map(|lp| (*pos, *lp)))
            .collect();
        out.sort_unstable_by_key(|(pos, _)| (pos.x, pos.y, pos.z));
        out
    }

    /// bastion (task #55, harness hook, 2026-07-30): how many designation
    /// Regions are currently recorded as blocked. The acceptance test's
    /// edge-trigger check: this must reach exactly 1 (not re-increment
    /// every tick) once a designation is genuinely blocked, and must go
    /// back to 0 once that designation is cancelled.
    pub fn bastion_blocked_regions_count(&self) -> usize {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .blocked_regions
            .len()
    }

    /// bastion (task #55, harness hook, 2026-07-30): the blocking cell for
    /// whichever blocked Region (if any) contains `pos` -- the same lookup
    /// the inspector uses, exposed directly so the harness can assert on a
    /// designation's blocking cell without going through the full
    /// `BastionInspectKind` payload.
    pub fn bastion_blocked_by(&self, pos: vek::Vec3<i32>) -> Option<vek::Vec3<i32>> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .blocked_by(pos)
    }

    /// bastion (task #61, attribution, 2026-08-03): EVERY mechanism that
    /// has recorded a block covering `pos` -- see
    /// `JobBoard::blocked_sources`'s doc (a scalar first-match would
    /// silently hide a second producer on the same region).
    pub fn bastion_blocked_sources(&self, pos: vek::Vec3<i32>) -> Vec<&'static str> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .blocked_sources(pos)
    }

    /// bastion (task #59, starvation measurement, harness hook,
    /// 2026-07-30): `(starvation_cycles, starvation_crowded_cycles,
    /// cycles_since_last_claim)` for one job position -- see
    /// `JobBoard::starvation_cycles`'s doc for the hypothesis this tests.
    /// All zero if the position was never open/unclaimed during
    /// arbitration (e.g. always claimed instantly, or never designated).
    pub fn bastion_starvation_stats(&self, pos: vek::Vec3<i32>) -> (u32, u32, u32, u32) {
        let board = self
            .state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>();
        (
            board.starvation_cycles.get(&pos).copied().unwrap_or(0),
            board
                .starvation_crowded_cycles
                .get(&pos)
                .copied()
                .unwrap_or(0),
            board.cycles_since_last_claim.get(&pos).copied().unwrap_or(0),
            // task #59 aging mechanism-level check: "times offered."
            board.claims_by_pos.get(&pos).copied().unwrap_or(0),
        )
    }

    /// bastion (observability row, DECISIONS #49, harness hook,
    /// 2026-08-04): the access-plan state the corpus had zero visibility
    /// into (75 fields, none of them this) despite the self-rescue path
    /// firing in most seeds -- see `JobBoard::access_plan_calls`'s own
    /// doc for what each number means. Flattened tuple: `(self_rescue_
    /// calls, self_rescue_emissions, emergency_calls, emergency_
    /// emissions, proactive_descent_calls, proactive_descent_emissions,
    /// self_rescue_starved_by_access_pending, access_pending_true_ticks,
    /// live_is_access_count)`. All zero-defaulted, never `None` -- a
    /// scenario that never exercises access-planning at all reads as
    /// all-zero, not absent, so a corpus-wide zero must be checked
    /// against whether `is_access` jobs EVER existed before reading it
    /// as "the mechanism never fires."
    ///
    /// CORRECTION (2026-08-09, Opus, caught auditing #60's falsifier):
    /// that existence check is field `.7` (`access_pending_true_ticks`,
    /// an ACCUMULATOR -- incremented every tick any `is_access` job was
    /// on the board, never reset), NOT the last field `.8`
    /// (`live_is_access_count`, a SNAPSHOT -- `board.jobs` at the moment
    /// of THIS read only). On wave32's 48 seeds, `.8 > 0` undercounted
    /// `.7 > 0` 14-to-34: 20 seeds ran access-planning for real but had
    /// no `is_access` job left on the board at the specific tick the
    /// harness happened to sample. `ever` is not `now` -- use `.7` to
    /// ask whether access-planning ever ran; `.8` only answers whether
    /// it is running AT THIS INSTANT (a different, also legitimate,
    /// question -- not this one).
    ///
    /// NOT a duplicate of `bastion_cascade_probe`'s `access_emissions_max`
    /// (Opus's catch, 2026-08-04): that field is a PER-MEMBER MAXIMUM
    /// over `cascade_access_emissions` (the emergency call site only) --
    /// answers "what's the worst single member's count". This field's
    /// `emergency_emissions` is a TOTAL across all members and calls --
    /// answers "how many times overall", and additionally covers the
    /// self_rescue and proactive_descent sites `cascade_probe` doesn't
    /// touch at all. Different questions, kept separate on purpose --
    /// see `bastion_cascade_probe`'s own doc for the per-member reading.
    pub fn bastion_access_plan_stats(
        &self,
    ) -> (u32, u32, u32, u32, u32, u32, u32, u64, u32) {
        let board = self
            .state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>();
        let get = |m: &hashbrown::HashMap<&'static str, u32>, k: &str| {
            m.get(k).copied().unwrap_or(0)
        };
        (
            get(&board.access_plan_calls, "self_rescue"),
            get(&board.access_plan_emissions, "self_rescue"),
            get(&board.access_plan_calls, "emergency"),
            get(&board.access_plan_emissions, "emergency"),
            get(&board.access_plan_calls, "proactive_descent"),
            get(&board.access_plan_emissions, "proactive_descent"),
            board.self_rescue_starved_by_access_pending,
            board.access_pending_true_ticks,
            board.jobs.values().filter(|j| j.is_access).count() as u32,
        )
    }

    /// bastion (#70, ROW60-F3-CORPUS-FIELDS-PACKET, harness hook,
    /// 2026-08-09): the F3 stale-access-plan pruner's branch-dwell
    /// accumulators -- the corpus's only route to this data, since the
    /// wave-fan transport carries stdout JSON only and discards stderr,
    /// where the live `F3-BRANCH` diagnostic writes. See
    /// `JobBoard::b5_f3_ticks_branch_a`'s own doc for what each field
    /// means. Flattened: `(ticks_branch_a, ticks_branch_b,
    /// ticks_branch_c, transitions, idle_peak, prunes_fired,
    /// stalled_peak, stalled_final)`. All pure accumulators, zero-defaulted,
    /// never gated -- DIAGNOSTICS, not verdict terms; must never enter the
    /// harness's `clauses` vec. `stalled_peak` added for ITEM 2 (Opus's
    /// catch, 2026-08-10): without it the fan cannot set
    /// `ACCESS_STALL_SECS` from measured seeds at all. `stalled_final`
    /// added the same day (Opus's second catch, WAVE33-RESULTS.md): the
    /// peak alone can't distinguish "stalled then recovered" from "still
    /// stalling when the run ended" -- this is `access_stalled_secs`'s
    /// value at whatever moment this is called, which resolves that.
    pub fn bastion_f3_prune_stats(&self) -> (u64, u64, u64, u32, f32, u32, f32, f32) {
        let board = self
            .state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>();
        (
            board.b5_f3_ticks_branch_a,
            board.b5_f3_ticks_branch_b,
            board.b5_f3_ticks_branch_c,
            board.b5_f3_transitions,
            board.b5_f3_idle_peak,
            board.b5_f3_prunes_fired,
            board.b5_f3_stalled_peak,
            board.access_stalled_secs,
        )
    }

    /// bastion (DECISIONS #89, ROW69-OPTION-B-PACKET, harness hook,
    /// 2026-08-09): the reservation-capacity row's feature-acceptance
    /// measures. Flattened: `(eat_completions_distinct,
    /// stack_reserved_units_max)`. See `JobBoard::
    /// b5_eat_completions_distinct`'s own doc. Both pure accumulators,
    /// zero-defaulted, never gated -- DIAGNOSTICS, not verdict terms;
    /// must never enter the harness's `clauses` vec.
    /// bastion (entity-event-log stage-1 floor gate, self-attestation field,
    /// Opus's catch, 2026-08-10): a paired determinism-floor run with the
    /// chassis enabled but zero producers wired changes no other
    /// gameplay-visible field, so the two arms are otherwise
    /// indistinguishable from "the env var never reached the harness."
    /// `event_count`'s `Option` return renders as absent in one arm and
    /// present (even at `0`) in the other -- presence is the witness the
    /// number can't yet provide. Not gated on ECS state (the log is a
    /// process-global, not a board resource); pure passthrough, DIAGNOSTIC
    /// only, must never enter the harness's `clauses` vec.
    pub fn bastion_eelog_event_count(&self) -> Option<u64> { bastion_entity_event_log::event_count() }

    /// bastion (entity-event-log Measure 0 export, Opus's request,
    /// 2026-08-10): "the aggregate-late law landing on the instrument
    /// built to escape it" -- `bastion_eelog_event_count` is a run-total
    /// and cannot answer either of the pilot's two registered customers
    /// (Measure 0's cluster-vs-uniform question over ticks; seed 69's
    /// per-event queue-position detail), so this crosses entities
    /// deliberately and exports per-event data instead. Flattened for the
    /// harness's JSON: `(tick, subject_uid, job_id, reason_debug,
    /// queue_position)`. `cap` bounds the list (density budget -- this is
    /// per-event data crossing into the corpus, not a per-tick emission);
    /// the paired `truncated` bool must be surfaced beside it, same
    /// self-accounting law as the ring's own flag -- a silently-capped
    /// list must never render identically to a complete one. Not gated on
    /// ECS state; pure passthrough, DIAGNOSTIC only, must never enter the
    /// harness's `clauses` vec.
    pub fn bastion_eelog_released_events(
        &self,
        cap: usize,
    ) -> (Vec<(u64, u64, u64, String, Option<usize>)>, bool) {
        let (records, truncated) = bastion_entity_event_log::released_events(cap);
        let flattened = records
            .into_iter()
            .map(|r| (r.tick, r.subject.0.get(), r.job, format!("{:?}", r.reason), r.queue_position))
            .collect();
        (flattened, truncated)
    }

    /// PRECONDITION for the eat/stack pair (instrument debt, ebc2b5a053).
    /// `b5_eat_completions_distinct` and `b5_stack_reserved_units_max` are ZERO
    /// on all 48 seeds, which reads two ways the fields cannot separate: the
    /// stack path RAN and nothing stacked, or NO RESERVATION WAS EVER MADE.
    /// `next_reservation` is a monotonic id counter -- it already answers this
    /// and only needed exposing. 0 => the zeros are VACUOUS.
    pub fn bastion_reservation_total(&self) -> u64 {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .reservation_total()
    }

    pub fn bastion_eat_stack_stats(&self) -> (u32, u32) {
        let board = self
            .state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>();
        (
            board.b5_eat_completions_distinct.len() as u32,
            board.b5_stack_reserved_units_max,
        )
    }

    /// bastion (ROW-ITEM6-WITNESS-PACKET, harness hook, 2026-08-10): the
    /// item-6 pickup-refusal witness -- board accumulators, not
    /// flight-recorder events, so a corpus fan (stdout JSON only) can
    /// finally see item 6 at all. Flattened: `(refused_pile_protected,
    /// refused_ambient_disabled, refused_ambient_uids_distinct,
    /// refused_loot_owned_colonist, refused_loot_owned_ambient,
    /// pile_pickup_by_member, pile_pickup_by_nonmember)`. The first two are
    /// FLAT (Opus's ruling: a colonist/ambient split on either would read
    /// 0 by construction of the branch that guards it -- see `JobBoard::
    /// b5_pickup_refused_pile_protected`'s own doc). `refused_loot_owned_*`
    /// stays split (real signal there). `refused_ambient_uids_distinct` is
    /// the count half of the timing-race witness -- see
    /// `bastion_item6_ambient_refusal_recheck` for the other half, which
    /// needs a separate call (it does a live storage read, not a stored
    /// counter). All pure accumulators, zero-defaulted, never gated --
    /// DIAGNOSTICS, not verdict terms; must never enter the harness's
    /// `clauses` vec.
    pub fn bastion_item6_witness_stats(&self) -> (u32, u32, u32, u32, u32, u32, u32) {
        let board = self
            .state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>();
        (
            board.b5_pickup_refused_pile_protected,
            board.b5_pickup_refused_ambient_disabled,
            board.b5_pickup_refused_ambient_uids.len() as u32,
            board.b5_pickup_refused_loot_owned_colonist,
            board.b5_pickup_refused_loot_owned_ambient,
            board.b5_pile_pickup_by_member,
            board.b5_pile_pickup_by_nonmember,
        )
    }

    /// bastion (ROW-ITEM6-WITNESS-PACKET, timing-race witness, ruling
    /// 2026-08-10): the deferred half of the `ambient-loot-disabled`
    /// check that replaced Opus's withdrawn same-instant `_colonist`
    /// split. `board.b5_pickup_refused_ambient_uids` records every
    /// picker `Uid` refused as ambient, with the TICK of its first
    /// refusal; THIS accessor cross-references those uids against
    /// colonist status NOW, at call time -- a genuinely different instant
    /// than the branch predicate that recorded them, which is what makes
    /// "was this uid a colonist later" a real question instead of a
    /// tautology (a same-instant re-read of the same component the
    /// branch already gated on cannot vary, see the withdrawn design's
    /// own doc). Flattened: `(distinct_uids, later_colonist)`.
    /// `later_colonist > 0` means a picker refused as ambient is a
    /// colonist by the time this is called -- for a scenario where
    /// colony membership is fixed at startup (no mid-run recruitment),
    /// that is decisive evidence of the membership-timing race Fable's
    /// wave33-mover hypothesis predicted; if membership CAN change
    /// mid-run, cross-check the recorded tick against when membership
    /// changed before concluding a race (recruitment produces the same
    /// signature with a large delta; a race produces a small one).
    /// Call once, at whatever point the caller considers "later" (a
    /// corpus fan calls this at run end).
    pub fn bastion_item6_ambient_refusal_recheck(&self) -> (u32, u32) {
        let ecs = self.state.ecs();
        let board = ecs.read_resource::<bastion_jobs::JobBoard>();
        let id_maps = ecs.read_resource::<common::uid::IdMaps>();
        let colonists = ecs.read_storage::<comp::Colonist>();
        let distinct = board.b5_pickup_refused_ambient_uids.len() as u32;
        let later_colonist = board
            .b5_pickup_refused_ambient_uids
            .keys()
            .filter(|uid| {
                id_maps
                    .uid_entity(**uid)
                    .is_some_and(|e| colonists.contains(e))
            })
            .count() as u32;
        (distinct, later_colonist)
    }

    /// bastion (ARB-ATTEMPT-01 step 2, batch item 1, harness hook,
    /// 2026-08-04): `to_release` outcome counts by classified reason --
    /// see `JobBoard::release_reason_counts`'s own doc for the zero-vs-
    /// absent caveat. Flattened: `(other, timed_out, completed,
    /// removed_externally, target_changed)`. `target_changed` (step 2b,
    /// found closing seed 66's `Other:1` site-scan gap) is the 4th
    /// discovered producer -- a job's target block changing mid-travel.
    /// Four of 26 producers are named yet (still scoped, not exhaustive)
    /// -- a nonzero `other` on a DIFFERENT seed means that seed exercises
    /// a still-undiscovered producer, not a bug.
    pub fn bastion_release_reason_counts(&self) -> (u32, u32, u32, u32, u32) {
        use bastion_jobs::ReleaseReason;
        let board = self
            .state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>();
        let get = |r: ReleaseReason| board.release_reason_counts.get(&r).copied().unwrap_or(0);
        (
            get(ReleaseReason::Other),
            get(ReleaseReason::TimedOut),
            get(ReleaseReason::Completed),
            get(ReleaseReason::RemovedExternally),
            get(ReleaseReason::TargetChanged),
        )
    }

    /// bastion (mechanism-2 friction instrument, harness hook, 2026-07-30):
    /// EVERY job position that ever timed out this run, with its count --
    /// unlike `bastion_timeout_count_for_pos` (single position) or the
    /// still-open-cell diag (undefined for passing seeds, since a pass has
    /// no open cells), this enumeration exists for EVERY seed regardless of
    /// outcome, because a position can time out one or more times and still
    /// go on to complete. That's the point: it's the only friction-location
    /// signal Fable's structural-position test can use without repeating
    /// the same tautology on still-open-cell positions (undefined for
    /// passes) one level down. Read-only, no world writes.
    pub fn bastion_all_timeout_positions(&self) -> Vec<(vek::Vec3<i32>, u32)> {
        let mut v: Vec<(vek::Vec3<i32>, u32)> = self
            .state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .timeout_counts_by_pos
            .iter()
            .map(|(pos, count)| (*pos, *count))
            .collect();
        // Opus's E14-5a lesson (2026-07-30): a HashMap-order Vec reaching
        // JSON as an array makes run-to-run diffs noisy for a reason that
        // has nothing to do with the simulation (hashbrown's per-process
        // random iteration seed). Count descending, position ascending as
        // tiebreak, so the array is canonical and a diff means something.
        v.sort_unstable_by(|a, b| {
            b.1.cmp(&a.1)
                .then((a.0.x, a.0.y, a.0.z).cmp(&(b.0.x, b.0.y, b.0.z)))
        });
        v
    }

    /// bastion (mechanism-2 friction instrument, harness hook, 2026-07-30):
    /// total TGT-DRIFT (Chaser astar-reset) events this run. On the record
    /// per Fable's ruling: this does NOT discriminate ambient friction from
    /// the failure-tail signature (fires at similar rates in passing and
    /// failing runs) -- report it, never gate on it alone.
    pub fn bastion_drift_events_total(&self) -> u64 {
        self.state
            .ecs()
            .read_resource::<bastion_path::PathScheduler>()
            .drift_events_total
    }

    /// bastion (mechanism-2 terrain probe, harness hook, 2026-07-30): the
    /// closest approach EVER achieved toward `job_pos`, across every claim
    /// attempt -- a pure position measurement sharing no dependency with
    /// `has_standable_stance`, so it can discriminate what a path-exists
    /// probe built on that predicate structurally cannot (arrived-close vs.
    /// never-got-near). `None` if this position never had an active
    /// traveler.
    pub fn bastion_min_distance_to_target(&self, job_pos: vek::Vec3<i32>) -> Option<f32> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .min_distance_to_target
            .get(&job_pos)
            .copied()
    }

    /// bastion (mechanism-2 terrain probe, harness hook, 2026-07-30): the
    /// colonist's actual position at the moment of `job_pos`'s most recent
    /// travel timeout -- lets the offline reachability probe run from
    /// where a failing attempt actually stood, not just from spawn.
    /// `None` if this position never timed out.
    pub fn bastion_last_timeout_pos(&self, job_pos: vek::Vec3<i32>) -> Option<vek::Vec3<f32>> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .last_timeout_pos
            .get(&job_pos)
            .copied()
    }

    /// bastion (mechanism-2 terrain probe, harness hook, 2026-07-30): the
    /// Chaser's own route-diagnostic state at EVERY travel timeout on
    /// `job_pos`, in order -- `(route_exists, route_complete,
    /// route_next_idx)` per timeout. Fable's refinement: `route_next_idx`
    /// PINNED across the sequence means stuck at one waypoint;
    /// ADVANCING means real route progress that still times out -- a
    /// different failure than getting stuck. Empty if this position
    /// never timed out.
    pub fn bastion_timeout_route_states(
        &self,
        job_pos: vek::Vec3<i32>,
    ) -> Vec<(bool, Option<bool>, Option<usize>)> {
        self.state
            .ecs()
            .read_resource::<bastion_jobs::JobBoard>()
            .timeout_route_states
            .get(&job_pos)
            .cloned()
            .unwrap_or_default()
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
        // `E11-6b`: sorted by `Uid`, not the raw `entity.id()` this
        // returns -- `entity.id()` is an allocator SLOT, reused across
        // despawn/respawn and dependent on join iteration order, exactly
        // `E11-6a`'s named hazard (same class DET-PHY-005 fixed for
        // physics candidates). The tuple keeps `entity.id()` as its
        // first field for callers that already depend on that shape;
        // only the SORT KEY changes -- extracted to
        // `sort_persistent_item_snapshots_by_uid_v1` so the ordering
        // claim is directly testable without a live ECS world.
        let uids = ecs.read_storage::<Uid>();
        let unsorted: Vec<_> = (&entities, &items, &positions, &piles, &uids)
            .join()
            .filter(|(_, item, _, _, _)| {
                item.item().item_definition_id().itemdef_id() == Some(asset_id)
            })
            .map(|(entity, item, pos, _, uid)| (entity.id(), item.amount() as u64, pos.0, *uid))
            .collect();
        sort_persistent_item_snapshots_by_uid_v1(unsorted)
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

    /// bastion (batch prep, 2026-08-04, Fable-directed): the SOLID-CELL
    /// QUERY -- every cell within `region` that's currently FILLED terrain,
    /// independent of whether a job exists there at all. `mine_cell_diag`
    /// (bastion-harness) iterates `board.jobs.values()`, i.e. JOB-BEARING
    /// cells only -- its membership rule silently drops any solid cell
    /// that never got a job (a designation gap) AND any solid cell whose
    /// job was already removed while the block itself never actually
    /// cleared (a phantom-completion mismatch) -- burned batch items 61
    /// and 90 twice today by conflating "no job here" with "nothing solid
    /// here". This fn asks the terrain directly, not the job board, so it
    /// can't inherit that membership bug. READ-ONLY (`terrain.get` only),
    /// no world writes, no job-board access at all -- can't move any
    /// existing baseline. Bounded to `region`'s own volume; a caller
    /// passing an unbounded region is responsible for the cost.
    pub fn bastion_solid_cells_in_region(
        &self,
        region: common::bastion::Region,
    ) -> Vec<vek::Vec3<i32>> {
        use common::vol::ReadVol;
        let terrain = self.state.terrain();
        let mut out = Vec::new();
        for x in region.min.x..=region.max.x {
            for y in region.min.y..=region.max.y {
                for z in region.min.z..=region.max.z {
                    let p = vek::Vec3::new(x, y, z);
                    if terrain.get(p).is_ok_and(|b| b.is_filled()) {
                        out.push(p);
                    }
                }
            }
        }
        out
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

        // T0.72: the ONE named content-epoch admission barrier, first
        // thing every tick, before any system runs. Certified/harness
        // runs lock the epoch by construction: DeterministicSerial skips
        // the check entirely rather than gating on a separate flag.
        if !self.execution_mode.is_deterministic() {
            let ecs = self.state.ecs();
            let mut watchers = ecs.write_resource::<content_epoch::ContentWatchers>();
            let mut epoch = ecs.write_resource::<common::resources::ContentEpoch>();
            let changed = watchers.poll_and_admit(&mut epoch);
            if !changed.is_empty() {
                info!(?changed, epoch = epoch.0, "content epoch advanced");
            }
        }

        // bastion (B-ASSET1): arena upkeep (deferred fixture goto). No-op
        // when the arena resource is absent (i.e. always, outside
        // --asset-arena boots).
        #[cfg(feature = "worldgen")]
        self.bastion_arena_tick();

        // bastion (ITEM 29): refresh the trade price book — the leaf crate
        // cannot see world's site economies, so the SERVER mirrors
        // (site pos, food-per-wood ratio) into an ECS resource at a slow
        // cadence. Prices move with the economy sim; 600 ticks keeps the
        // book honest without paying the site walk per tick.
        #[cfg(feature = "worldgen")]
        self.bastion_trade_price_tick();

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
                    // ★ REAL-TERRAIN AUTOFOUND (BASTION_AUTOFOUND_REAL_TERRAIN=1).
                    //
                    // This filter required the FLAT ARENA, which made a headless
                    // terrain-determinism run impossible: the arena is
                    // PRE-GENERATED so nothing is ever promoted, and real
                    // terrain has something to generate but no requester,
                    // because without the arena no colony is founded and so no
                    // `Presence` exists to ask for chunks. Measured: a headless
                    // real-terrain arm emitted 7,200 census lines and promoted
                    // ZERO, with `autofound colony founded` appearing 0 times.
                    //
                    // Neither half of the machinery is actually arena-specific.
                    // `world_center_wpos` is `sim().get_size() / 2 * RECT_SIZE`
                    // -- the WORLD centre, deterministic for any world -- and
                    // the founding preset resolves its datum from TERRAIN. Only
                    // `spawn_wpos`'s hardcoded FLAT_ARENA_Z is, and the branch
                    // below resolves ground height instead when the arena is off.
                    //
                    // Default unchanged: without the new flag the arena
                    // requirement stands exactly as before, byte-for-byte.
                    .filter(|&n| {
                        n > 0
                            && (bastion_flat_arena::enabled()
                                || std::env::var_os("BASTION_AUTOFOUND_REAL_TERRAIN").is_some())
                    })
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
                        // On the flat arena the spawn z is the slab constant. On
                        // REAL terrain that constant is meaningless, so resolve
                        // the ground from the terrain the force-load above just
                        // brought in. A colony spawned at the wrong z either
                        // falls or is entombed, and either way promotes nothing
                        // -- which would reproduce the very VOID this branch
                        // exists to remove.
                        let sp_opt = if bastion_flat_arena::enabled() {
                            Some(bastion_flat_arena::spawn_wpos(center))
                        } else {
                            use bastion_server::bastion_founding_preset as preset;
                            let xy = Vec2::new(center.x as i32, center.y as i32);
                            let ground = {
                                let ecs = self.state.ecs();
                                let terrain =
                                    ecs.read_resource::<common::terrain::TerrainGrid>();
                                preset::resolve_datum(&terrain, xy, 0)
                            };
                            match ground {
                                Some(z) => Some(Vec3::new(
                                    center.x as f32 + 0.5,
                                    center.y as f32 + 0.5,
                                    z as f32 + 1.0,
                                )),
                                None => {
                                    // LOUD: an unresolved datum means the
                                    // force-load did not deliver the centre
                                    // chunk. Spawning anyway would produce a
                                    // silent zero-promotion run that looks
                                    // deterministic for the wrong reason.
                                    //
                                    // ★ `None` here, NOT an early return: the
                                    // first version returned from `tick()`,
                                    // which would have skipped the ENTIRE
                                    // remaining tick -- a far wider blast
                                    // radius than "do not found a colony". The
                                    // compiler caught it (tick returns Result),
                                    // and the narrow form is what was wanted
                                    // anyway.
                                    tracing::warn!(
                                        ?center,
                                        "bastion: real-terrain autofound could not resolve ground                                          datum -- SKIPPING the spawn rather than founding at a                                          guessed z"
                                    );
                                    None
                                },
                            }
                        };
                        if let Some(sp) = sp_opt {
                            self.bastion_autofound_found(sp, n);
                        }
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

        // `E11-1b` (premise-checked, CLOSED negative -- orchestrator-ruled):
        // this drain processes `CharacterUpdaterMessage`s in raw crossbeam
        // `try_iter` arrival order, the same shape as `ChunkGenerator`'s
        // pre-fix completion channel (see `chunk_generator.rs`'s
        // `recv_new_chunks_sorted`/`recv_new_chunks_deterministic` --
        // request-tick stamp + hold + due-release + sort-by-key). That
        // pattern does NOT apply here today, traced per handler:
        //   - `DatabaseBatchCompletion` -> `process_batch_completion` is a
        //     `HashMap::retain` keyed on its own `batch_id`; two calls with
        //     different ids commute regardless of order.
        //   - `CharacterScreenResponse` handlers (`CharacterList`,
        //     `CharacterCreation`, `CharacterEdit`, `CharacterData` ->
        //     `handle_loaded_character_data`) write ONLY to that message's
        //     own `target_entity`'s own components/subscription and send to
        //     that entity's own connection -- never shared/authoritative
        //     state another player's simulation outcome depends on. Unlike
        //     a chunk (shared world state every viewer depends on), this is
        //     ordinary per-connection arrival ordering, not a determinism
        //     hazard.
        //   - A universal sort key doesn't even exist today: the variants
        //     are heterogeneous and don't uniformly carry a `CharacterId`
        //     (`CharacterList`/`DatabaseBatchCompletion` have none at all;
        //     `CharacterCreation`/`CharacterEdit` carry one only inside
        //     `Ok(..)`) -- building one would mean threading a new request-
        //     sequence field through `CharacterLoaderRequestKind` and
        //     `CharacterUpdaterAction` for zero current consumers.
        // ARMED TRIGGER: the moment any handler here starts writing shared/
        // authoritative state beyond its own response's entity, this
        // reasoning no longer holds and the chunk-gen stamp+hold+due-
        // release+sort mechanism (cited above, already implemented once)
        // becomes MANDATORY, not optional -- start from this trace, not
        // from scratch.
        //
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
            // `APEX-T4.6` chunk 3b: `RtSim::save` also needs the
            // character DB's directory now -- read via the same
            // `Arc<RwLock<DatabaseSettings>>` resource
            // `server/src/lib.rs`'s construction already inserts.
            let db_dir = self
                .state
                .ecs()
                .read_resource::<Arc<RwLock<DatabaseSettings>>>()
                .read()
                .expect("DatabaseSettings RwLock was poisoned")
                .db_dir
                .clone();
            self.state
                .ecs()
                .write_resource::<rtsim::RtSim>()
                .save(true, &db_dir);
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

impl Server {
    /// bastion: the WHOLE autofound founding — spawn, adoption/preset
    /// placement, seed-food — EXTRACTED from `tick()` after the frozen-phase
    /// guard (t0_27, fixed 20,000-char window) failed FOUR times today on
    /// founding-side additions (seed-food inline, its explanatory comment, the
    /// comment spelling a landmark, adoption inline). With the block out of
    /// `tick()` entirely, founding edits can never consume that window again.
    fn bastion_autofound_found(&mut self, sp: Vec3<f32>, n: u8) {

                        // ADOPT-A-TOWN (mode A): all logic lives in the
                        // helper (t0_27 window discipline).
                        let adoption = self.bastion_adoption(sp);
                        let sp = adoption.as_ref().map(|(asp, ..)| *asp).unwrap_or(sp);
                        tracing::info!(?sp, arena = bastion_flat_arena::enabled(),
                            adopted = adoption.is_some(),
                            "bastion: autofound spawn resolved");
                        self.bastion_spawn_colony_seeded(sp, n, 0);
                        // THE PRESET, TOO — otherwise this path spawns
                        // colonists into a world with NO WORK, and every
                        // scored counter reads zero. Two deterministic runs
                        // then "match" at 0 == 0, which is vacuous on exactly
                        // the numbers a determinism capture exists to compare.
                        //
                        // Same placement authority the live handler uses
                        // (`place_preset`), so the captured colony is the one
                        // an overseer would found rather than a lookalike.
                        // MODE A placement in a helper (t0_27 discipline).
                        if let Some((_, town_origin, plots)) = adoption {
                            Self::bastion_adopt_place(
                                self.state.ecs(),
                                town_origin,
                                &plots,
                                sp.z.floor() as i32,
                            );
                        } else {
                            use bastion_server::bastion_founding_preset as preset;
                            let origin_xy = preset::origin_xy(sp);
                            let ecs = self.state.ecs();
                            let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
                            let datum =
                                preset::resolve_datum(&terrain, origin_xy, sp.z.floor() as i32);
                            if let Some(datum_z) = datum {
                                let origin = Vec3::new(origin_xy.x, origin_xy.y, datum_z);
                                let mut board =
                                    ecs.write_resource::<bastion_server::bastion_jobs::JobBoard>();
                                let (roles, jobs) =
                                    preset::place_preset(&mut board, &terrain, origin);
                                tracing::info!(
                                    ?origin,
                                    elements = %preset::roles_summary(&roles),
                                    complete = preset::preset_is_complete(&roles),
                                    jobs,
                                    "bastion: autofound colony founded (deterministic path)"
                                );
                                // ITEM 11 fixture lever (see the fn's doc).
                                Self::bastion_seed_food(ecs, origin);
                                Self::bastion_seed_materials(ecs, origin);
                            } else {
                                tracing::warn!(
                                    ?origin_xy,
                                    "bastion: autofound could not resolve a datum -- no preset \
                                     placed"
                                );
                            }
                        }
                        // AND THE PRESENCE — the last uncontrolled input.
                        //
                        // Without it, the ONLY thing keeping the colony's
                        // chunks loaded is a connected client, and work
                        // cannot proceed on unloaded terrain. The client
                        // arrives at a wall-clock-dependent tick (measured:
                        // `Accepting Tcp` at log line 262 vs 246 across two
                        // otherwise byte-identical runs), so the whole work
                        // trajectory started from a different offset and no
                        // determinism comparison was possible.
                        //
                        // A server-owned presence loads them with no client
                        // at all, which is what makes a driverless
                        // determinism capture possible.
                        self.bastion_found_colony_presence(sp);
                        tracing::info!(
                            pos = ?sp,
                            "bastion: autofound colony presence created (no client needed)"
                        );
                        
    }

    /// bastion (ADOPT-A-TOWN): the full mode-A decision — flag check, site
    /// search, datum re-anchor — EXTRACTED from `tick()` because the previous
    /// inline version pushed `before_state_tick` past t0_27's 20,000-char
    /// window, exactly as the seed-food block did before it. Returns the
    /// re-anchored spawn, the town origin, and the mapped plots; `None` =
    /// mode B, byte-identical founding.
    fn bastion_adoption(
        &self,
        sp: Vec3<f32>,
    ) -> Option<(
        Vec3<f32>,
        Vec2<i32>,
        Vec<(common::bastion::DesignationKind, Vec2<i32>, Vec2<i32>)>,
    )> {
        if std::env::var_os("BASTION_ADOPT_TOWN").is_none() {
            return None;
        }
        #[cfg(not(feature = "worldgen"))]
        {
            // A named refusal, not silence: the test_world shim has no sites.
            tracing::warn!(
                "bastion: BASTION_ADOPT_TOWN set but this build has no worldgen —                  adoption is impossible here, falling back to the founding preset"
            );
            None
        }
        #[cfg(feature = "worldgen")]
        {
            let (town_origin, plots) = Self::bastion_adoptable_town_plots(
                self.index.as_index_ref(),
                sp.xy().map(|e| e as i32),
                // 4096: the first live leg MEASURED the nearest adoptable
                // site at 1,224 blocks against a 1,024 radius — 200 short.
                // The re-anchor moves the whole founding to the town, so a
                // wide radius costs nothing but the scan.
                4096,
            )?;
            // Re-anchor the WHOLE founding at the town — via worldgen's own
            // APPROXIMATE altitude, never loaded terrain. The first version
            // called `resolve_datum` at the town, where no chunk exists at
            // founding tick, got None, and silently fell back to the ORIGINAL
            // spawn: colonists, presence and chunk-loading all stayed 1,100
            // blocks from the adopted plots, which therefore waited forever
            // (the WAITING witness read min_loaded=false for an entire leg).
            // `get_alt_approx` is sim data — terrain-independent by
            // construction, which is the property this decision needs.
            let asp = self
                .world()
                .sim()
                .get_alt_approx(town_origin)
                .map(|alt| {
                    Vec3::new(
                        town_origin.x as f32 + 0.5,
                        town_origin.y as f32 + 0.5,
                        alt + 2.0,
                    )
                })
                .unwrap_or(sp);
            Some((asp, town_origin, plots))
        }
    }

    /// bastion (ADOPT-A-TOWN): mode-A placement — adopted designations
    /// replace the preset, through the SAME `place_designation_surface`
    /// authority the paint path uses. Extracted for the same t0_27 reason.
    fn bastion_adopt_place(
        ecs: &specs::World,
        town_origin: Vec2<i32>,
        plots: &[(common::bastion::DesignationKind, Vec2<i32>, Vec2<i32>)],
        hint_z: i32,
    ) {
        // ★ DEFERRED, not immediate (2026-08-20). The first live leg found the
        // town, mapped 5 plots — and placed ZERO jobs of every kind, because
        // placement ran at founding tick against terrain that had not streamed
        // in yet ("unloaded" and "unsuitable" render identically to a surface
        // resolve). Item 18 solved this exact problem for save-restore:
        // `pending_restore` holds (Region, kind) rows and the bastion tick
        // drains each once BOTH its corners are loaded, through
        // `place_designation` — the same authority everything else uses. The
        // adopted plots ride that path; the z-band brackets the town datum so
        // the volume covers each plot's actual surface.
        let mut board = ecs.write_resource::<bastion_server::bastion_jobs::JobBoard>();
        let mut queued = 0usize;
        for (kind, min, max) in plots {
            // SURFACE queue, not the volume one: the volume z-band buried the
            // jobs (unreachable_job=8, colony idle 12,000 ticks). The surface
            // drain resolves each column's own top, which is what a town plot
            // actually is.
            board
                .pending_adopt_surface
                .push((*min, *max, hint_z, *kind));
            queued += 1;
        }
        tracing::info!(
            ?town_origin,
            plots = plots.len(),
            queued,
            "bastion: ADOPT-A-TOWN founded into an existing settlement              (designations QUEUED, placed as terrain loads)"
        );
        drop(board);
        // The survival window still needs food: the fixture lever applies
        // identically here.
        Self::bastion_seed_food(ecs, Vec3::new(town_origin.x, town_origin.y, hint_z));
        Self::bastion_seed_materials(ecs, Vec3::new(town_origin.x, town_origin.y, hint_z));
    }

    /// bastion (ADOPT-A-TOWN mode A, 2026-08-20): find the nearest worldgen
    /// site holding at least one MAPPED plot (FarmField/House/Barn — the
    /// charter's own mapping) within `radius` of `near`, and return OWNED
    /// data: the site's origin plus each mapped plot as
    /// (colony designation kind, world-space min XY, world-space max XY).
    ///
    /// OWNED on purpose: the caller must drop the `IndexRef` borrow before
    /// `bastion_spawn_colony_seeded(&mut self)` — returning references here
    /// would wedge the founding sequence into a borrow conflict.
    ///
    /// "Adoptable = has ≥1 mapped plot" deliberately sidesteps site-kind
    /// taxonomy: the bars care about structures a colony can USE, not about
    /// what worldgen calls the settlement.
    #[cfg(feature = "worldgen")]
    fn bastion_adoptable_town_plots(
        index: world::IndexRef,
        near: Vec2<i32>,
        radius: i32,
    ) -> Option<(Vec2<i32>, Vec<(common::bastion::DesignationKind, Vec2<i32>, Vec2<i32>)>)>
    {
        use common::bastion::DesignationKind as D;
        use world::site::plot::PlotKind;
        let map_kind = |k: &PlotKind| match k {
            PlotKind::FarmField(_) => Some(D::Farm),
            PlotKind::House(_) => Some(D::Bed),
            PlotKind::Barn(_) => Some(D::Stockpile),
            _ => None,
        };
        let (site, d2) = index
            .sites
            .iter()
            .filter_map(|(_, site)| {
                let has_mapped = site.plots().any(|p| map_kind(p.kind()).is_some());
                has_mapped.then(|| (site, site.origin.distance_squared(near)))
            })
            .min_by_key(|(_, d2)| *d2)?;
        if d2 > radius * radius {
            tracing::warn!(
                ?near,
                radius,
                nearest_d = (d2 as f32).sqrt() as i32,
                "bastion: ADOPT-A-TOWN VOID — nearest adoptable site is outside                  the search radius (a worldgen fact, not a feature failure)"
            );
            return None;
        }
        let plots = site
            .plots()
            .filter_map(|p| {
                let kind = map_kind(p.kind())?;
                let b = p.find_bounds();
                // Tile-space -> world-space; +1/-1 keeps the max INCLUSIVE.
                let min = site.tile_wpos(b.min);
                let max = site.tile_wpos(b.max + 1) - 1;
                Some((kind, min, max))
            })
            .collect::<Vec<_>>();
        Some((site.origin, plots))
    }

    /// bastion (ITEM 11 fixture lever, 2026-08-20): `BASTION_SEED_FOOD=<n>`
    /// drops `n` food items at a founding colony so hunger is not the binding
    /// constraint. Default OFF — absent the var this is never called into.
    ///
    /// ★★ EXTRACTED FROM `tick()`, AND ITS COMMENT KEPT OUT OF `tick()` TOO.
    /// The frozen-phase-order test scans a FIXED 20,000-char window of that
    /// function, so anything there — code OR comments — can push a later
    /// landmark out of view. This block cost the guard twice before it moved:
    /// once as ~60 lines of code, then again as the ~15-line comment
    /// explaining the first fix. A third failure came from the comment
    /// SPELLING a landmark's name, which a plain `find(needle)` cannot tell
    /// from the landmark itself.
    ///
    /// So: inside `tick()`, one line. Everything else lives here, where
    /// length is free.
    ///
    /// EXTRACTED from `tick()` deliberately: inline, it pushed a phase landmark
    /// out of the frozen-phase-order test's fixed 20,000-char window. That test
    /// protects the ORDER of tick phases, and a long fixture block near the top
    /// of `tick()` degrades it silently. (Outside `tick()` this doc is safe —
    /// the test only scans that function's window.)
    /// bastion (ITEM 29): mirror each priced site's (position, food-per-wood
    /// ratio) into the `TradePriceBook` ECS resource the mission generator
    /// reads. The ratio is the site's OWN `SitePrices` (bar 2's audit); z
    /// comes from sim alt (terrain-independent — the adoption lesson).
    #[cfg(feature = "worldgen")]
    fn bastion_trade_price_tick(&mut self) {
        let tick = self.state.ecs().read_resource::<Tick>().0;
        if tick % 600 != 23 {
            return;
        }
        let entries: Vec<(Vec3<i32>, f32)> = {
            let ecs = self.state.ecs();
            let rtsim = ecs.read_resource::<rtsim::RtSim>();
            let data = rtsim.state().data();
            let index = self.index.as_index_ref();
            data.sites
                .iter()
                .filter_map(|(_, site)| {
                    let ws = site.world_site?;
                    let prices = index.get_site_prices(ws.id())?;
                    let food = prices
                        .values
                        .get(&common::trade::Good::Food)
                        .copied()
                        .unwrap_or(0.0);
                    let wood = prices
                        .values
                        .get(&common::trade::Good::Wood)
                        .copied()
                        .unwrap_or(0.0);
                    // Degenerate prices are REPORTED by exclusion, never
                    // normalised (the prereg's VOID branch).
                    if food <= 0.0 || wood <= 0.0 {
                        return None;
                    }
                    let wpos = site.wpos;
                    let alt = self.world.sim().get_alt_approx(wpos)?;
                    Some((Vec3::new(wpos.x, wpos.y, alt as i32 + 2), wood / food))
                })
                .collect()
        };
        if !entries.is_empty() {
            let mut book = self
                .state
                .ecs()
                .write_resource::<bastion_jobs::TradePriceBook>();
            if book.0.len() != entries.len() {
                // Bar 2's audit rides here: the ratio must be the SITE'S,
                // not a constant — a spread of one distinct value is the
                // FEW-DISCRETE-VALUES flag (the first minted ratio was a
                // suspiciously round 1.0).
                let mut ratios: Vec<f32> = entries.iter().map(|(_, r)| *r).collect();
                ratios.sort_by(|a, b| a.total_cmp(b));
                let distinct = ratios
                    .windows(2)
                    .filter(|w| (w[1] - w[0]).abs() > 1e-6)
                    .count()
                    + 1;
                tracing::info!(
                    sites = entries.len(),
                    ratio_min = ratios.first().copied().unwrap_or(0.0),
                    ratio_max = ratios.last().copied().unwrap_or(0.0),
                    ratio_distinct = distinct,
                    "bastion: ITEM 29 trade price book refreshed"
                );
            }
            book.0 = entries;
        }
    }

    /// bastion (ADOPT bar 2): FIXTURE lever — seed building materials the
    /// way `bastion_seed_food` seeds food. An adopted town's 1,519
    /// designations refused 12,152/12,152 claim checks at the materials
    /// gate (the colony arrives with NOTHING and even access-job generation
    /// starves behind the refusals). Whether adoption SHIPS with a starter
    /// cache is a design ruling (banked for Ben); this env unblocks the
    /// "colonists work adopted infrastructure" bar meanwhile.
    fn bastion_seed_materials(ecs: &specs::World, origin: Vec3<i32>) {
        let Ok(n) = std::env::var("BASTION_SEED_MATERIALS") else {
            return;
        };
        let n: u32 = n.parse().unwrap_or(0);
        // Deferred: at an adopted town the origin chunk is UNLOADED at
        // founding — direct emits landed in the void (food_stock=0 all
        // leg while the witness said seeded=64). The board drain delivers
        // when the chunk loads; on a loaded flat arena that's ~same tick.
        // Its OWN resource, never the JobBoard: the founding caller holds
        // the board mutably and a second borrow here panicked the server
        // on boot (atomic_refcell, chain10 cookdiag VOID).
        {
            let mut q = ecs.write_resource::<crate::bastion_jobs::PendingSeedItems>();
            q.0.push((
                origin + Vec3::new(2, 0, 0),
                common::bastion::BUILD_MATERIAL_ITEM.to_string(),
                n,
            ));
            // ITEM 29: half as many LOGS beside the stones — the trade
            // mission's sellable lot (CHOP_DROP_ITEM), so a trade leg can
            // fixture "wood to sell" the same way builds fixture stone.
            q.0.push((
                origin + Vec3::new(4, 0, 0),
                common::bastion::CHOP_DROP_ITEM.to_string(),
                (n / 2).max(1) as u32,
            ));
        }
        tracing::warn!(
            seeded = n,
            ?origin,
            "bastion: BASTION_SEED_MATERIALS active — QUEUED for chunk-load delivery (FIXTURE lever; no balance number changed)"
        );
    }

    fn bastion_seed_food(ecs: &specs::World, origin: Vec3<i32>) {
        let Ok(n) = std::env::var("BASTION_SEED_FOOD") else {
            return;
        };
        let n: u32 = n.parse().unwrap_or(0);
        // Deferred via the board queue (see bastion_seed_materials — the
        // adopted-origin unloaded-chunk lesson applies identically here).
        ecs.write_resource::<crate::bastion_jobs::PendingSeedItems>()
            .0
            .push((origin, "common.items.food.mushroom".to_string(), n));
        tracing::warn!(
            seeded = n,
            ?origin,
            "bastion: BASTION_SEED_FOOD active — colony supplied so hunger is not the              binding constraint (FIXTURE lever; no balance number changed)"
        );
    }
}
