#[cfg(feature = "plugins")]
use crate::plugin::PluginMgr;
#[cfg(feature = "plugins")]
use crate::plugin::memory_manager::EcsWorld;
use crate::{BuildArea, NoDurabilityArea};
#[cfg(feature = "plugins")]
use common::uid::IdMaps;
use common::{
    calendar::Calendar,
    comp::{self, gizmos::RtsimGizmos},
    event::{BonkEvent, EventBus, LocalEvent},
    interaction,
    link::Is,
    mounting::{Mount, Rider, VolumeRider, VolumeRiders},
    outcome::Outcome,
    resources::{
        ContentEpoch, DeltaTime, EntitiesDiedLastTick, GameMode, PlayerEntity,
        PlayerPhysicsSettings, ProgramTime, Time, TimeOfDay, TimeScale,
    },
    shared_server_config::ServerConstants,
    slowjob::SlowJobPool,
    terrain::{Block, MapSizeLg, TerrainChunk, TerrainGrid, sprite::SpriteAdjecencyRequirement},
    tether,
    time::DayPeriod,
    trade::Trades,
    util::Dir2,
    vol::{ReadVol, WriteVol},
    weather::{Weather, WeatherGrid},
};
use common_base::{prof_span, span};
use common_ecs::{PhysicsMetrics, SysMetrics};
use common_net::sync::{WorldSyncExt, interpolation as sync_interp};
use core::{convert::identity, time::Duration};
use hashbrown::{HashMap, HashSet};
use rayon::{ThreadPool, ThreadPoolBuilder};
use specs::{
    Component, DispatcherBuilder, Entity as EcsEntity, WorldExt,
    prelude::Resource,
    shred::{Fetch, FetchMut, SendDispatcher},
    storage::{MaskedStorage as EcsMaskedStorage, Storage as EcsStorage},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use timer_queue::TimerQueue;
use vek::*;

/// At what point should we stop speeding up physics to compensate for lag? If
/// we speed physics up too fast, we'd skip important physics events like
/// collisions. This constant determines the upper limit. If delta time exceeds
/// this value, the game's physics will begin to produce time lag. Ideally, we'd
/// avoid such a situation.
const MAX_DELTA_TIME: f32 = 1.0;
/// convert seconds to milliseconds to use in TimerQueue
const SECONDS_TO_MILLISECONDS: f64 = 1000.0;

#[derive(Default)]
pub struct BlockChange {
    blocks: HashMap<Vec3<i32>, Block>,
}

impl BlockChange {
    pub fn set(&mut self, pos: Vec3<i32>, block: Block) { self.blocks.insert(pos, block); }

    pub fn try_set(&mut self, pos: Vec3<i32>, block: Block) -> Option<()> {
        if !self.blocks.contains_key(&pos) {
            self.blocks.insert(pos, block);
            Some(())
        } else {
            None
        }
    }

    /// Check if the block at given position `pos` has already been modified
    /// this tick.
    pub fn can_set_block(&self, pos: Vec3<i32>) -> bool { !self.blocks.contains_key(&pos) }

    pub fn clear(&mut self) { self.blocks.clear(); }
}

#[derive(Default)]
pub struct ScheduledBlockChange {
    changes: TimerQueue<HashMap<Vec3<i32>, Block>>,
    outcomes: TimerQueue<HashMap<Vec3<i32>, Block>>,
    last_poll_time: u64,
}
impl ScheduledBlockChange {
    pub fn set(&mut self, pos: Vec3<i32>, block: Block, replace_time: f64) {
        let timer = self.changes.insert(
            (replace_time * SECONDS_TO_MILLISECONDS) as u64,
            HashMap::new(),
        );
        self.changes.get_mut(timer).insert(pos, block);
    }

    pub fn outcome_set(&mut self, pos: Vec3<i32>, block: Block, replace_time: f64) {
        let outcome_timer = self.outcomes.insert(
            (replace_time * SECONDS_TO_MILLISECONDS) as u64,
            HashMap::new(),
        );
        self.outcomes.get_mut(outcome_timer).insert(pos, block);
    }
}

#[derive(Default)]
pub struct TerrainChanges {
    pub new_chunks: HashSet<Vec2<i32>>,
    pub modified_chunks: HashSet<Vec2<i32>>,
    pub removed_chunks: HashSet<Vec2<i32>>,
    pub modified_blocks: HashMap<Vec3<i32>, Block>,
}

impl TerrainChanges {
    pub fn clear(&mut self) {
        self.new_chunks.clear();
        self.modified_chunks.clear();
        self.removed_chunks.clear();
    }
}

#[derive(Clone)]
pub struct BlockDiff {
    pub wpos: Vec3<i32>,
    pub old: Block,
    pub new: Block,
}

impl BlockDiff {
    /// T1.12 (Bastion conservation cluster): the RTSim resource-conservation
    /// predicate — true iff this diff changes the block's rtsim resource
    /// CLASS (a `Some`↔`None` transition, or one class to another). This is
    /// the SINGLE authority deciding whether a block change must be forwarded
    /// to `RtSim::hook_block_update`: a resource block that is mined, grown,
    /// or otherwise reclassified without this returning `true` would silently
    /// desync rtsim's resource ledger (an untracked deletion or creation).
    /// Two sprites of the SAME class (e.g. `Stones` vs `Stones2`, both
    /// `Stone`) is NOT a class change and correctly needs no rtsim hook.
    ///
    /// Determinism (Ben's law): a pure total function of `(old, new)` blocks
    /// — no state, no RNG, no wall-clock. Centralising it here means every
    /// forwarding decision reads ONE predicate, pinned against drift by
    /// `t1_12_resource_change_predicate`.
    pub fn changes_rtsim_resource(&self) -> bool {
        self.old.get_rtsim_resource() != self.new.get_rtsim_resource()
    }
}

/// A type used to represent game state stored on both the client and the
/// server. This includes things like entity components, terrain data, and
/// global states like weather, time of day, etc.
pub struct State {
    ecs: specs::World,
    // Avoid lifetime annotation by storing a thread pool instead of the whole dispatcher
    thread_pool: Arc<ThreadPool>,
    dispatcher: SendDispatcher<'static>,
    execution_mode: ExecutionMode,
}

pub type Pools = Arc<ThreadPool>;

/// Selects how ECS systems and nested Rayon work execute.
///
/// The live game uses [`Parallel`](Self::Parallel). The Bastion headless
/// harness opts into [`DeterministicSerial`](Self::DeterministicSerial) at
/// boot so identical inputs have one system/entity execution order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionMode {
    Parallel,
    DeterministicSerial,
}

impl ExecutionMode {
    pub const fn is_deterministic(self) -> bool { matches!(self, Self::DeterministicSerial) }
}

impl State {
    pub fn pools(game_mode: GameMode) -> Pools {
        Self::pools_with_mode(game_mode, ExecutionMode::Parallel)
    }

    /// Build the pool used by dispatch, nested Rayon iterators, worldgen, and
    /// slow jobs. A one-worker pool removes work-stealing only for the
    /// deterministic harness; live construction remains unchanged.
    pub fn pools_with_mode(game_mode: GameMode, execution_mode: ExecutionMode) -> Pools {
        let (thread_name_infix, is_main_task) = match game_mode {
            GameMode::Server => ("s", true),
            GameMode::Client => ("c", true),
            // Note: We don't currently use `Singleplayer`. When we do, server-side tasks should be
            // deprioritised in favour of things that sit on the main thread!
            GameMode::Singleplayer => ("sp", false),
        };

        let is_first_error = Arc::new(AtomicBool::new(true));
        let set_priority = move || {
            use thread_priority::*;
            let priority = if is_main_task {
                // These threads are critical for the main tick loop, so need a higher priority
                ThreadPriority::Crossplatform(TryFrom::try_from(50).unwrap())
            } else {
                ThreadPriority::Min
            };
            let res = cfg_select! {
                target_os = "linux" => std::thread::current().set_priority_and_policy(
                    ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin),
                    priority,
                ),
                _ => std::thread::current().set_priority(priority),
            };
            if let Err(err) = res
                && is_first_error.swap(false, Ordering::Relaxed)
            {
                tracing::warn!(
                    "Unable to set priority/schedule policy for dispatcher pool thread: {err}"
                );
            }
        };

        // T0.52 (T0-004 packet, step 5): BASTION_DETERMINISTIC_PARALLEL runs
        // deterministic mode on a MULTI-worker pool with parallel dispatch —
        // the serial-vs-parallel equivalence probe. If the stamped bus,
        // per-entity disjoint writes, and keyed draws hold, a parallel
        // deterministic run must be byte-identical to the serial one; any
        // divergence names a real schedule-order authority leak.
        let deterministic_parallel = std::env::var_os("BASTION_DETERMINISTIC_PARALLEL").is_some();
        let num_threads = if execution_mode.is_deterministic() && !deterministic_parallel {
            1
        } else if execution_mode.is_deterministic() && deterministic_parallel {
            // T0.64 (T0-004 packet, step 10): the legal-schedule FUZZER —
            // BASTION_SCHEDULE_SEED varies the WORKER COUNT (a declared
            // scheduling freedom: any worker count is legal, shred still
            // enforces the phase manifest's dependencies). A campaign runs
            // one serial baseline + K parallel legs at seed-derived thread
            // counts and asserts every leg is byte-identical to serial; a
            // diverging seed is the minimal repro (the shrink is already a
            // single seed). Unset seed = full num_cpus (the plain probe).
            match std::env::var("BASTION_SCHEDULE_SEED").ok().and_then(|s| s.parse::<u64>().ok()) {
                Some(seed) => {
                    let max = num_cpus::get().max(2);
                    // Seed-derived worker count in [2, max].
                    2 + (seed as usize % (max - 1).max(1))
                },
                None => num_cpus::get().max(common::consts::MIN_RECOMMENDED_RAYON_THREADS),
            }
        } else {
            num_cpus::get().max(common::consts::MIN_RECOMMENDED_RAYON_THREADS)
        };

        Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .thread_name(move |i| format!("rayon-{}-{}", thread_name_infix, i))
                .spawn_handler(|thread| {
                    let mut b = std::thread::Builder::new();
                    if let Some(name) = thread.name() {
                        b = b.name(name.to_owned());
                    }
                    if let Some(stack_size) = thread.stack_size() {
                        b = b.stack_size(stack_size);
                    }
                    let set_priority = set_priority.clone();
                    b.spawn(move || {
                        set_priority();
                        thread.run()
                    })?;
                    Ok(())
                })
                .build()
                .unwrap(),
        )
    }

    /// Create a new `State` in client mode.
    pub fn client(
        pools: Pools,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        add_systems: impl Fn(&mut DispatcherBuilder),
        #[cfg(feature = "plugins")] plugin_mgr: PluginMgr,
    ) -> Self {
        Self::new(
            GameMode::Client,
            pools,
            map_size_lg,
            default_chunk,
            add_systems,
            #[cfg(feature = "plugins")]
            plugin_mgr,
        )
    }

    /// Create a new `State` in server mode.
    pub fn server(
        pools: Pools,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        add_systems: impl Fn(&mut DispatcherBuilder),
        #[cfg(feature = "plugins")] plugin_mgr: PluginMgr,
    ) -> Self {
        Self::server_with_mode(
            pools,
            map_size_lg,
            default_chunk,
            ExecutionMode::Parallel,
            add_systems,
            #[cfg(feature = "plugins")]
            plugin_mgr,
        )
    }

    /// Create server state with an explicit execution policy. This is kept
    /// separate from [`State::server`] so every existing live caller remains
    /// parallel by default.
    pub fn server_with_mode(
        pools: Pools,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        execution_mode: ExecutionMode,
        add_systems: impl Fn(&mut DispatcherBuilder),
        #[cfg(feature = "plugins")] plugin_mgr: PluginMgr,
    ) -> Self {
        Self::new_with_mode(
            GameMode::Server,
            pools,
            map_size_lg,
            default_chunk,
            execution_mode,
            add_systems,
            #[cfg(feature = "plugins")]
            plugin_mgr,
        )
    }

    pub fn new(
        game_mode: GameMode,
        pools: Pools,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        add_systems: impl Fn(&mut DispatcherBuilder),
        #[cfg(feature = "plugins")] plugin_mgr: PluginMgr,
    ) -> Self {
        Self::new_with_mode(
            game_mode,
            pools,
            map_size_lg,
            default_chunk,
            ExecutionMode::Parallel,
            add_systems,
            #[cfg(feature = "plugins")]
            plugin_mgr,
        )
    }

    fn new_with_mode(
        game_mode: GameMode,
        pools: Pools,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        execution_mode: ExecutionMode,
        add_systems: impl Fn(&mut DispatcherBuilder),
        #[cfg(feature = "plugins")] plugin_mgr: PluginMgr,
    ) -> Self {
        prof_span!(guard, "create dispatcher");
        // DET-ECS-007: new dispatcher = new phase-barrier schedule.
        common_ecs::begin_schedule();
        let mut dispatch_builder =
            DispatcherBuilder::<'static, 'static>::new().with_pool(Arc::clone(&pools));
        // TODO: Consider alternative ways to do this
        add_systems(&mut dispatch_builder);
        let dispatcher = dispatch_builder
            .build()
            .try_into_sendable()
            .unwrap_or_else(|_| panic!("Thread local systems not allowed"));
        drop(guard);

        Self {
            ecs: Self::setup_ecs_world(
                game_mode,
                Arc::clone(&pools),
                map_size_lg,
                default_chunk,
                execution_mode,
                #[cfg(feature = "plugins")]
                plugin_mgr,
            ),
            thread_pool: pools,
            dispatcher,
            execution_mode,
        }
    }

    /// Creates ecs world and registers all the common components and resources
    // TODO: Split up registering into server and client (e.g. move
    // EventBus<ServerEvent> to the server)
    fn setup_ecs_world(
        game_mode: GameMode,
        thread_pool: Arc<ThreadPool>,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        execution_mode: ExecutionMode,
        #[cfg(feature = "plugins")] mut plugin_mgr: PluginMgr,
    ) -> specs::World {
        prof_span!("State::setup_ecs_world");
        let mut ecs = specs::World::new();
        // Uids for sync
        ecs.register_sync_marker();
        // bastion (B2a): overseer selection marker (client-side use; harmless
        // to register everywhere, never synced).
        ecs.register::<comp::BastionSelected>();
        // bastion (B3): colonists + god-anchor. `Colonist` is synced (see
        // common-net synced_components); the rest are server-side.
        ecs.register::<comp::Colonist>();
        ecs.register::<comp::PlayerColony>();
        ecs.register::<comp::bastion::Needs>();
        ecs.register::<comp::bastion::Mood>();
        // bastion (AUTON-0): the per-colonist drive arbiter.
        ecs.register::<comp::bastion::Arbiter>();
        ecs.register::<comp::bastion::ConstructedLadderTraversal>();
        ecs.register::<comp::bastion::BastionTraversalOwnership>();
        ecs.register::<comp::BastionGodAnchor>();
        // bastion (B4): colonist job assignment (server-side).
        ecs.register::<comp::bastion::ActiveJob>();
        // bastion (B5.5): persistent item-pile marker (server-side).
        ecs.register::<comp::bastion::BastionPile>();
        // bastion (B-ASSET1): test-fixture goto order (server-side).
        ecs.register::<comp::bastion::BastionTestGoto>();
        // Register server -> all clients synced components.
        ecs.register::<comp::Body>();
        ecs.register::<comp::Hardcore>();
        ecs.register::<comp::body::parts::Heads>();
        ecs.register::<comp::Player>();
        ecs.register::<comp::Stats>();
        ecs.register::<comp::SkillSet>();
        ecs.register::<comp::ActiveAbilities>();
        ecs.register::<comp::Buffs>();
        ecs.register::<comp::Auras>();
        ecs.register::<comp::EnteredAuras>();
        ecs.register::<comp::Energy>();
        ecs.register::<comp::Combo>();
        ecs.register::<comp::Health>();
        ecs.register::<comp::Poise>();
        ecs.register::<comp::CanBuild>();
        ecs.register::<comp::LightEmitter>();
        ecs.register::<comp::PickupItem>();
        ecs.register::<comp::ThrownItem>();
        ecs.register::<comp::Scale>();
        ecs.register::<Is<Mount>>();
        ecs.register::<Is<Rider>>();
        ecs.register::<Is<VolumeRider>>();
        ecs.register::<Is<tether::Leader>>();
        ecs.register::<Is<tether::Follower>>();
        ecs.register::<Is<interaction::Interactor>>();
        ecs.register::<interaction::Interactors>();
        ecs.register::<comp::Mass>();
        ecs.register::<comp::Density>();
        ecs.register::<comp::Collider>();
        ecs.register::<comp::Sticky>();
        ecs.register::<comp::Immovable>();
        ecs.register::<comp::CharacterState>();
        ecs.register::<comp::CharacterActivity>();
        ecs.register::<comp::Object>();
        ecs.register::<comp::Group>();
        ecs.register::<comp::Shockwave>();
        ecs.register::<comp::ShockwaveHitEntities>();
        ecs.register::<comp::projectile::ProjectileHitEntities>();
        ecs.register::<comp::Beam>();
        ecs.register::<comp::Arcing>();
        ecs.register::<comp::Pool>();
        ecs.register::<comp::Alignment>();
        ecs.register::<comp::LootOwner>();
        ecs.register::<comp::Admin>();
        ecs.register::<comp::Stance>();
        ecs.register::<comp::Teleporting>();
        ecs.register::<comp::GizmoSubscriber>();
        ecs.register::<comp::FrontendMarker>();

        // Register components send from clients -> server
        ecs.register::<comp::Controller>();

        // Register components send directly from server -> all but one client
        ecs.register::<comp::PhysicsState>();

        // Register components synced from client -> server -> all other clients
        ecs.register::<comp::Pos>();
        ecs.register::<comp::Vel>();
        ecs.register::<comp::Ori>();
        ecs.register::<comp::Inventory>();

        // Register common unsynced components
        ecs.register::<comp::PreviousPhysCache>();
        ecs.register::<comp::PosVelOriDefer>();

        // Register client-local components
        // TODO: only register on the client
        ecs.register::<comp::LightAnimation>();
        ecs.register::<sync_interp::InterpBuffer<comp::Pos>>();
        ecs.register::<sync_interp::InterpBuffer<comp::Vel>>();
        ecs.register::<sync_interp::InterpBuffer<comp::Ori>>();

        // Register server-local components
        // TODO: only register on the server
        ecs.register::<comp::Last<comp::Pos>>();
        ecs.register::<comp::Last<comp::Vel>>();
        ecs.register::<comp::Last<comp::Ori>>();
        ecs.register::<comp::Agent>();
        ecs.register::<comp::WaypointArea>();
        ecs.register::<comp::ForceUpdate>();
        ecs.register::<comp::InventoryUpdateBuffer>();
        ecs.register::<comp::Waypoint>();
        ecs.register::<comp::MapMarker>();
        ecs.register::<comp::Projectile>();
        ecs.register::<comp::Melee>();
        ecs.register::<comp::ItemDrops>();
        ecs.register::<comp::ChatMode>();
        ecs.register::<comp::Faction>();
        ecs.register::<comp::invite::Invite>();
        ecs.register::<comp::invite::PendingInvites>();
        ecs.register::<VolumeRiders>();
        ecs.register::<common::combat::DeathEffects>();
        ecs.register::<common::combat::RiderEffects>();
        ecs.register::<comp::SpectatingEntity>();

        // Register synced resources used by the ECS.
        ecs.insert(TimeOfDay(0.0));
        ecs.insert(Calendar::default());
        ecs.insert(WeatherGrid::new(Vec2::zero()));
        ecs.insert(Time(0.0));
        ecs.insert(ProgramTime(0.0));
        ecs.insert(TimeScale(1.0));
        // T0.72: only the server's admission barrier ever advances this;
        // the client never runs that barrier, so it stays at 0 there.
        ecs.insert(ContentEpoch::default());

        // Register unsynced resources used by the ECS.
        ecs.insert(DeltaTime(0.0));
        ecs.insert(PlayerEntity(None));
        ecs.insert(TerrainGrid::new(map_size_lg, default_chunk).unwrap());
        ecs.insert(BlockChange::default());
        ecs.insert(ScheduledBlockChange::default());
        ecs.insert(crate::special_areas::AreasContainer::<BuildArea>::default());
        ecs.insert(crate::special_areas::AreasContainer::<NoDurabilityArea>::default());
        ecs.insert(TerrainChanges::default());
        ecs.insert(EventBus::<LocalEvent>::default());
        ecs.insert(game_mode);
        ecs.insert(execution_mode);
        ecs.insert(EventBus::<Outcome>::default());
        ecs.insert(common::CachedSpatialGrid::default());
        ecs.insert(EntitiesDiedLastTick::default());
        ecs.insert(RtsimGizmos::default());

        if execution_mode.is_deterministic() {
            ecs.insert(SlowJobPool::new_inline(10_000, thread_pool));
        } else {
            let num_cpu = num_cpus::get() as u64;
            let slow_limit = (num_cpu / 2 + num_cpu / 4).max(1);
            tracing::trace!(?slow_limit, "Slow Thread limit");
            ecs.insert(SlowJobPool::new(slow_limit, 10_000, thread_pool));
        }

        // TODO: only register on the server
        ecs.insert(comp::group::GroupManager::default());
        ecs.insert(SysMetrics::default());
        ecs.insert(PhysicsMetrics::default());
        ecs.insert(Trades::default());
        ecs.insert(PlayerPhysicsSettings::default());
        ecs.insert(VolumeRiders::default());

        // Load plugins from asset directory
        #[cfg(feature = "plugins")]
        ecs.insert({
            let ecs_world = EcsWorld {
                entities: &ecs.entities(),
                health: ecs.read_component().into(),
                uid: ecs.read_component().into(),
                id_maps: &ecs.read_resource::<IdMaps>().into(),
                player: ecs.read_component().into(),
            };
            // APEX-T2.5.18: exactly-once ORDERED activation. Governed
            // managers are fail-closed — a hook failure ABORTS State
            // construction (the log-and-empty fallback is gone on that
            // path; a governed session never silently runs pluginless).
            // Legacy managers keep the exact old fallback behavior.
            let mut plugin_mgr = plugin_mgr;
            match plugin_mgr.activate_v1(&ecs_world, game_mode) {
                Ok(()) => {
                    // APEX-T2.5.19: governed sessions validate ACTUAL
                    // registrations against declared manifest claims —
                    // an undeclared registration aborts initialization.
                    if plugin_mgr.is_governed() {
                        if let Err(e) = plugin_mgr.registration_receipt_input_v1() {
                            panic!(
                                "APEX-T2.5.19 undeclared plugin registration (fail-closed): {e:?}"
                            );
                        }
                    }
                    plugin_mgr
                },
                Err(e) if plugin_mgr.is_governed() => {
                    // Startup abort (packet: no active-game rollback). On
                    // the client this unwinds through spawn_blocking as a
                    // typed JoinError; on the server it kills startup.
                    panic!("APEX-T2.5.18 governed plugin activation failed (fail-closed): {e:?}");
                },
                Err(e) => {
                    tracing::debug!(?e, "Failed to run plugin init");
                    tracing::info!("Plugins disabled, enable debug logging for more information.");
                    PluginMgr::default()
                },
            }
        });

        ecs
    }

    /// Register a component with the state's ECS.
    #[must_use]
    pub fn with_component<T: Component>(mut self) -> Self
    where
        <T as Component>::Storage: Default,
    {
        self.ecs.register::<T>();
        self
    }

    /// Write a component attributed to a particular entity, ignoring errors.
    ///
    /// This should be used *only* when we can guarantee that the rest of the
    /// code does not rely on the insert having succeeded (meaning the
    /// entity is no longer alive!).
    ///
    /// Returns None if the entity was dead or there was no previous entry for
    /// this component; otherwise, returns Some(old_component).
    pub fn write_component_ignore_entity_dead<C: Component>(
        &mut self,
        entity: EcsEntity,
        comp: C,
    ) -> Option<C> {
        self.ecs
            .write_storage()
            .insert(entity, comp)
            .ok()
            .and_then(identity)
    }

    /// Delete a component attributed to a particular entity.
    pub fn delete_component<C: Component>(&mut self, entity: EcsEntity) -> Option<C> {
        self.ecs.write_storage().remove(entity)
    }

    /// Read a component attributed to a particular entity.
    pub fn read_component_cloned<C: Component + Clone>(&self, entity: EcsEntity) -> Option<C> {
        self.ecs.read_storage().get(entity).cloned()
    }

    /// Read a component attributed to a particular entity.
    pub fn read_component_copied<C: Component + Copy>(&self, entity: EcsEntity) -> Option<C> {
        self.ecs.read_storage().get(entity).copied()
    }

    /// # Panics
    /// Panics if `EventBus<E>` is borrowed
    pub fn emit_event_now<E>(&self, event: E)
    where
        EventBus<E>: Resource,
    {
        self.ecs.write_resource::<EventBus<E>>().emit_now(event)
    }

    /// Given mutable access to the resource R, assuming the resource
    /// component exists (this is already the behavior of functions like `fetch`
    /// and `write_component_ignore_entity_dead`).  Since all of our resources
    /// are generated up front, any failure here is definitely a code bug.
    pub fn mut_resource<R: Resource>(&mut self) -> &mut R {
        self.ecs.get_mut::<R>().expect(
            "Tried to fetch an invalid resource even though all our resources should be known at \
             compile time.",
        )
    }

    /// Get a read-only reference to the storage of a particular component type.
    pub fn read_storage<C: Component>(&self) -> EcsStorage<'_, C, Fetch<'_, EcsMaskedStorage<C>>> {
        self.ecs.read_storage::<C>()
    }

    /// Get a reference to the internal ECS world.
    pub fn ecs(&self) -> &specs::World { &self.ecs }

    /// Get a mutable reference to the internal ECS world.
    pub fn ecs_mut(&mut self) -> &mut specs::World { &mut self.ecs }

    /// Admit a server-required plugin already available on disk, running its
    /// load hook exactly once against the live ECS.
    ///
    /// DET-PLG-003: late-admitted client plugins previously skipped their load
    /// hook (see [`PluginMgr::load_server_plugin`]). This wraps the manager
    /// call so the caller does not have to reconstruct the plugin ECS view or
    /// look up the game mode — both are read from this `State`, so a
    /// cached/downloaded plugin follows the same activation path as one present
    /// in the asset directory at construction.
    #[cfg(feature = "plugins")]
    pub fn load_server_plugin(
        &self,
        path: std::path::PathBuf,
    ) -> Result<common::event::PluginHash, crate::plugin::errors::PluginError> {
        let ecs = &self.ecs;
        let mode = *ecs.read_resource::<GameMode>();
        let ecs_world = EcsWorld {
            entities: &ecs.entities(),
            health: ecs.read_component().into(),
            uid: ecs.read_component().into(),
            id_maps: &ecs.read_resource::<IdMaps>().into(),
            player: ecs.read_component().into(),
        };
        ecs.write_resource::<PluginMgr>()
            .load_server_plugin(path, &ecs_world, mode)
    }

    /// Cache a server-delivered plugin's bytes and admit it, running its load
    /// hook exactly once (DET-PLG-003; see [`State::load_server_plugin`]).
    #[cfg(feature = "plugins")]
    pub fn cache_server_plugin(
        &self,
        base_dir: &std::path::Path,
        data: Vec<u8>,
    ) -> Result<common::event::PluginHash, crate::plugin::errors::PluginError> {
        let ecs = &self.ecs;
        let mode = *ecs.read_resource::<GameMode>();
        let ecs_world = EcsWorld {
            entities: &ecs.entities(),
            health: ecs.read_component().into(),
            uid: ecs.read_component().into(),
            id_maps: &ecs.read_resource::<IdMaps>().into(),
            player: ecs.read_component().into(),
        };
        ecs.write_resource::<PluginMgr>()
            .cache_server_plugin(base_dir, data, &ecs_world, mode)
    }

    pub fn thread_pool(&self) -> &Arc<ThreadPool> { &self.thread_pool }

    /// Get a reference to the `TerrainChanges` structure of the state. This
    /// contains information about terrain state that has changed since the
    /// last game tick.
    pub fn terrain_changes(&self) -> Fetch<'_, TerrainChanges> { self.ecs.read_resource() }

    /// Get a reference the current in-game weather grid.
    pub fn weather_grid(&self) -> Fetch<'_, WeatherGrid> { self.ecs.read_resource() }

    /// Get a mutable reference the current in-game weather grid.
    pub fn weather_grid_mut(&mut self) -> FetchMut<'_, WeatherGrid> { self.ecs.write_resource() }

    /// Get the current weather at a position in worldspace.
    pub fn weather_at(&self, pos: Vec2<f32>) -> Weather {
        self.weather_grid().get_interpolated(pos)
    }

    /// Get the max weather near a position in worldspace.
    pub fn max_weather_near(&self, pos: Vec2<f32>) -> Weather {
        self.weather_grid().get_max_near(pos)
    }

    /// Get the current in-game time of day.
    ///
    /// Note that this should not be used for physics, animations or other such
    /// localised timings.
    pub fn get_time_of_day(&self) -> f64 { self.ecs.read_resource::<TimeOfDay>().0 }

    /// Get the current in-game day period (period of the day/night cycle)
    pub fn get_day_period(&self) -> DayPeriod { self.get_time_of_day().into() }

    /// Get the current in-game time.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_time(&self) -> f64 { self.ecs.read_resource::<Time>().0 }

    /// Get the current true in-game time, unaffected by time_scale.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_program_time(&self) -> f64 { self.ecs.read_resource::<ProgramTime>().0 }

    /// Get the current delta time.
    pub fn get_delta_time(&self) -> f32 { self.ecs.read_resource::<DeltaTime>().0 }

    /// Get a reference to this state's terrain.
    pub fn terrain(&self) -> Fetch<'_, TerrainGrid> { self.ecs.read_resource() }

    /// Get a reference to this state's terrain.
    pub fn slow_job_pool(&self) -> Fetch<'_, SlowJobPool> { self.ecs.read_resource() }

    /// Get a writable reference to this state's terrain.
    pub fn terrain_mut(&self) -> FetchMut<'_, TerrainGrid> { self.ecs.write_resource() }

    /// Get a block in this state's terrain.
    pub fn get_block(&self, pos: Vec3<i32>) -> Option<Block> {
        self.terrain().get(pos).ok().copied()
    }

    /// Set a block in this state's terrain.
    pub fn set_block(&self, pos: Vec3<i32>, block: Block) {
        self.ecs.write_resource::<BlockChange>().set(pos, block);
    }

    /// Set a block in this state's terrain (used to delete temporary summoned
    /// sprites after a timeout).
    pub fn schedule_set_block(
        &self,
        pos: Vec3<i32>,
        block: Block,
        sprite_block: Block,
        replace_time: f64,
    ) {
        self.ecs
            .write_resource::<ScheduledBlockChange>()
            .set(pos, block, replace_time);
        self.ecs
            .write_resource::<ScheduledBlockChange>()
            .outcome_set(pos, sprite_block, replace_time);
    }

    /// Check if the block at given position `pos` has already been modified
    /// this tick.
    pub fn can_set_block(&self, pos: Vec3<i32>) -> bool {
        self.ecs.read_resource::<BlockChange>().can_set_block(pos)
    }

    /// Removes every chunk of the terrain.
    pub fn clear_terrain(&mut self) -> usize {
        let removed_chunks = &mut self.ecs.write_resource::<TerrainChanges>().removed_chunks;

        self.terrain_mut()
            .drain()
            .map(|(key, _)| {
                removed_chunks.insert(key);
            })
            .count()
    }

    /// Insert the provided chunk into this state's terrain.
    pub fn insert_chunk(&mut self, key: Vec2<i32>, chunk: Arc<TerrainChunk>) {
        if self
            .ecs
            .write_resource::<TerrainGrid>()
            .insert(key, chunk)
            .is_some()
        {
            self.ecs
                .write_resource::<TerrainChanges>()
                .modified_chunks
                .insert(key);
        } else {
            self.ecs
                .write_resource::<TerrainChanges>()
                .new_chunks
                .insert(key);
        }
    }

    /// Remove the chunk with the given key from this state's terrain, if it
    /// exists.
    pub fn remove_chunk(&mut self, key: Vec2<i32>) -> bool {
        if self
            .ecs
            .write_resource::<TerrainGrid>()
            .remove(key)
            .is_some()
        {
            self.ecs
                .write_resource::<TerrainChanges>()
                .removed_chunks
                .insert(key);

            true
        } else {
            false
        }
    }

    // Apply terrain changes
    pub fn apply_terrain_changes(&self, block_update: impl FnMut(&specs::World, Vec<BlockDiff>)) {
        self.apply_terrain_changes_internal(false, block_update);
    }

    /// `during_tick` is true if and only if this is called from within
    /// [State::tick].
    ///
    /// This only happens if [State::tick] is asked to update terrain itself
    /// (using `update_terrain: true`).  [State::tick] is called from within
    /// both the client and the server ticks, right after handling terrain
    /// messages; currently, client sets it to true and server to false.
    fn apply_terrain_changes_internal(
        &self,
        during_tick: bool,
        mut block_update: impl FnMut(&specs::World, Vec<BlockDiff>),
    ) {
        span!(
            _guard,
            "apply_terrain_changes",
            "State::apply_terrain_changes"
        );
        let mut terrain = self.ecs.write_resource::<TerrainGrid>();
        let mut modified_blocks =
            std::mem::take(&mut self.ecs.write_resource::<BlockChange>().blocks);

        let mut scheduled_changes = self.ecs.write_resource::<ScheduledBlockChange>();
        let current_time: f64 = self.ecs.read_resource::<Time>().0 * SECONDS_TO_MILLISECONDS;
        let current_time = current_time as u64;
        // This is important as the poll function has a debug assert that the new poll
        // is at a more recent time than the old poll. As Time is synced between server
        // and client, there is a chance that client dt can get slightly ahead of a
        // server update, so we do not want to panic in that scenario.
        if scheduled_changes.last_poll_time < current_time {
            scheduled_changes.last_poll_time = current_time;
            // DET-TER-013 (v5 deep-pass, disposition): the same-cell
            // precedence here is IMPLICIT but TOTAL and deterministic —
            // batches poll in (due-time, insertion-sequence) order from the
            // timer queue, and extend() overwrites, so the LATEST-due
            // scheduled write wins a cell, and any scheduled write beats the
            // live tick's earlier entry for that cell (scheduled-over-live).
            // Each batch is a position-keyed map (no intra-batch duplicate
            // cells), so batch-internal iteration order cannot matter.
            // Declared here so the policy is explicit; a stamped
            // per-operation schedule remains optional hardening.
            while let Some(changes) = scheduled_changes.changes.poll(current_time) {
                modified_blocks.extend(changes.iter());
            }
            let outcome = self.ecs.read_resource::<EventBus<Outcome>>();
            while let Some(outcomes) = scheduled_changes.outcomes.poll(current_time) {
                for (pos, block) in outcomes.into_iter() {
                    if let Some(sprite) = block.get_sprite() {
                        outcome.emit_now(Outcome::SpriteDelete { pos, sprite });
                    }
                }
            }
        }
        // Apply block modifications
        // Only include in `TerrainChanges` if successful
        let mut updated_blocks = Vec::with_capacity(modified_blocks.len());

        // All positions that should recieve a block update.
        let mut block_updates = HashSet::<Vec3<i32>>::default();

        modified_blocks.retain(|wpos, new| {
            let res = terrain.map(*wpos, |old| {
                updated_blocks.push(BlockDiff {
                    wpos: *wpos,
                    old,
                    new: *new,
                });
                *new
            });

            if let (&Ok(old), true) = (&res, during_tick) {
                // NOTE: If the changes are applied during the tick, we push the *old* value as
                // the modified block (since it otherwise can't be recovered after the tick).
                // Otherwise, the changes will be applied after the tick, so we push the *new*
                // value.
                *new = old;
            }

            if let (&Ok(old), false) = (&res, during_tick) {
                let h = old
                    .get_sprite()
                    .and_then(|s| s.solid_height())
                    .unwrap_or(1.0)
                    .max(
                        new.get_sprite()
                            .and_then(|s| s.solid_height())
                            .unwrap_or(1.0),
                    )
                    .ceil() as i32;

                block_updates.extend((-1..=h + 1).map(|z| wpos + Vec3::unit_z() * z).chain(
                    (0..=h).flat_map(|z| {
                        Dir2::ALL
                            .iter()
                            .map(move |d| wpos + Vec3::unit_z() * z + d.to_vec2())
                    }),
                ));
            };

            res.is_ok()
        });

        if !updated_blocks.is_empty() {
            block_update(&self.ecs, updated_blocks);
        }

        // Only do block updates not during the tick since that's when actual
        // terrain changes are applied.
        //
        // Clients will get these changes since they're just normal block updates
        // next tick.
        if !during_tick {
            prof_span!(_guard, "Indirectly modified sprites");

            // Collects all blocks that are neighbors with a modified block,
            // where the `adjecency_requirement` is no longer upheld.
            let indirectly_modified = block_updates
                .into_iter()
                // Filter for blocks that have an adjecency requirement.
                .filter_map(|wpos| {
                    let block = terrain.get(wpos).ok()?;
                    Some((wpos, block.get_sprite()?.adjecency_requirement()?, block))
                })
                // Check if said adjecency requirement is upheld.
                .filter(|(wpos, adjecency_requirement, block)| {
                    let rot_mat = block.rotation_mat();
                    // Tries to find a solid block for the given adjecent block.
                    let find_solid = |adj: Vec3<i32>| {
                        let wpos = wpos + adj;

                        let res = terrain.get(wpos).copied().unwrap_or(Block::empty());

                        // Don't check for sprites if we're checking for a block
                        // directly above.
                        let not_above = adj.z <= 0 || adj.x != 0 || adj.y != 0;

                        if not_above && !res.is_solid() {
                            // Sprites can be taller than 1 block.
                            for z in 1..=Block::MAX_HEIGHT.ceil() as i32 {
                                if let Ok(block) = terrain.get(wpos - Vec3::unit_z() * z)
                                    && let Some(sprite) = block.get_sprite()
                                    && let Some(h) = sprite.solid_height()
                                    && h.ceil() as i32 > z
                                {
                                    return *block;
                                }
                            }
                        }

                        res
                    };

                    // Same as `find_solid` but first rotates with the sprites rotation
                    // and mirroring.
                    let rel_solid = |adj: Vec3<i32>| find_solid(rot_mat * adj);

                    let valid = match adjecency_requirement {
                        SpriteAdjecencyRequirement::AllSolid(v) => {
                            v.iter().all(|v| rel_solid(*v).is_solid())
                        },
                        SpriteAdjecencyRequirement::AnySolid(v) => {
                            v.iter().any(|v| rel_solid(*v).is_solid())
                        },
                    };

                    !valid
                })
                .map(|(wpos, _, block)| (wpos, block))
                .collect::<Vec<_>>();

            // If the sprite is bonkable, bonk it.
            let bonk_event_bus = self.ecs.write_resource::<EventBus<BonkEvent>>();
            let mut bonk_emitter = bonk_event_bus.emitter();

            let mut block_change = self.ecs.write_resource::<BlockChange>();

            for (wpos, block) in indirectly_modified {
                if block.is_bonkable() {
                    bonk_emitter.emit(BonkEvent {
                        pos: wpos.as_::<f32>() + 0.5,
                        // TODO: Pass who destroyed the block?
                        owner: None,
                        target: None,
                    });
                } else {
                    block_change.blocks.insert(wpos, block.into_vacant());
                }
            }
        }

        self.ecs.write_resource::<TerrainChanges>().modified_blocks = modified_blocks;
    }

    /// Execute a single tick, simulating the game state by the given duration.
    pub fn tick(
        &mut self,
        dt: Duration,
        update_terrain: bool,
        mut metrics: Option<&mut StateTickMetrics>,
        server_constants: &ServerConstants,
        block_update: impl FnMut(&specs::World, Vec<BlockDiff>),
    ) {
        span!(_guard, "tick", "State::tick");

        // Timing code for server metrics
        macro_rules! section_span {
            ($guard:ident, $label:literal) => {
                span!(span_guard, $label);
                let metrics_guard = metrics.as_mut().map(|m| MetricsGuard::new($label, m));
                let $guard = (span_guard, metrics_guard);
            };
        }

        // Change the time accordingly.
        // T0.8 (master build order, Run 10 — the consistency half): under a
        // hitch (dt beyond MAX_DELTA_TIME) the OLD code advanced Time /
        // TimeOfDay / ProgramTime by the FULL frame dt while DeltaTime (and
        // so physics displacement) clamped — sim clocks raced ahead of the
        // world they timestamp, firing timers across un-simulated time. All
        // clocks now lag TOGETHER by the same clamp ("start lagging" as the
        // original comment intended). Byte-identical whenever
        // dt × time_scale <= MAX_DELTA_TIME — i.e. every harness tick and
        // every normal live frame. Bounded fixed SUBSTEPS (re-running the
        // dispatcher to consume the lag) remain the high-surface second
        // half, deliberately not smuggled into this change.
        let time_scale = self.ecs.read_resource::<TimeScale>().0;
        let scaled_dt = (dt.as_secs_f64() * time_scale).min(f64::from(MAX_DELTA_TIME));
        self.ecs.write_resource::<TimeOfDay>().0 +=
            scaled_dt * server_constants.day_cycle_coefficient;
        self.ecs.write_resource::<Time>().0 += scaled_dt;
        self.ecs.write_resource::<ProgramTime>().0 +=
            dt.as_secs_f64().min(f64::from(MAX_DELTA_TIME));

        // Update delta time.
        // Beyond a delta time of MAX_DELTA_TIME, start lagging to avoid skipping
        // important physics events.
        self.ecs.write_resource::<DeltaTime>().0 = scaled_dt as f32;

        section_span!(guard, "run systems");
        // T0.52: the parallel-equivalence probe uses the PARALLEL dispatcher
        // under deterministic seeds (see pools_with_mode).
        let deterministic_parallel =
            std::env::var_os("BASTION_DETERMINISTIC_PARALLEL").is_some();
        if self.execution_mode.is_deterministic() && !deterministic_parallel {
            // `dispatch_seq` fixes the Specs/Shred system order. Running it
            // inside this State's one-worker pool also captures nested
            // `par_join`/`par_iter` calls; otherwise sequential dispatch from
            // the main thread would fall back to Rayon's global pool.
            let pool = Arc::clone(&self.thread_pool);
            let dispatcher = &mut self.dispatcher;
            let ecs = &self.ecs;
            pool.install(move || dispatcher.dispatch_seq(ecs));
        } else {
            self.dispatcher.dispatch(&self.ecs);
        }
        drop(guard);

        self.maintain_ecs();

        if update_terrain {
            self.apply_terrain_changes_internal(true, block_update);
        }

        // Process local events
        section_span!(guard, "process local events");

        let outcomes = self.ecs.read_resource::<EventBus<Outcome>>();
        let mut outcomes_emitter = outcomes.emitter();

        let events = self.ecs.read_resource::<EventBus<LocalEvent>>().recv_all();
        for event in events {
            let mut velocities = self.ecs.write_storage::<comp::Vel>();
            let physics = self.ecs.read_storage::<comp::PhysicsState>();
            match event {
                LocalEvent::Jump(entity, impulse) => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0.z = impulse + physics.get(entity).map_or(0.0, |ps| ps.ground_vel.z);
                    }
                },
                LocalEvent::ApplyImpulse { entity, impulse } => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0 = impulse;
                    }
                },
                LocalEvent::Boost {
                    entity,
                    vel: extra_vel,
                } => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0 += extra_vel;
                    }
                },
                LocalEvent::CreateOutcome(outcome) => {
                    outcomes_emitter.emit(outcome);
                },
            }
        }
        drop(guard);
    }

    pub fn maintain_ecs(&mut self) {
        span!(_guard, "maintain ecs");
        self.ecs.maintain();
    }

    /// Clean up the state after a tick.
    pub fn cleanup(&mut self) {
        span!(_guard, "cleanup", "State::cleanup");
        // Clean up data structures from the last tick.
        self.ecs.write_resource::<TerrainChanges>().clear();
    }
}

// Timing code for server metrics
#[derive(Default)]
pub struct StateTickMetrics {
    pub timings: Vec<(&'static str, Duration)>,
}

impl StateTickMetrics {
    fn add(&mut self, label: &'static str, dur: Duration) {
        // Check for duplicates!
        debug_assert!(
            self.timings.iter().all(|(l, _)| *l != label),
            "Duplicate label in state tick metrics {label}"
        );
        self.timings.push((label, dur));
    }
}

struct MetricsGuard<'a> {
    start: Instant,
    label: &'static str,
    metrics: &'a mut StateTickMetrics,
}

impl<'a> MetricsGuard<'a> {
    fn new(label: &'static str, metrics: &'a mut StateTickMetrics) -> Self {
        Self {
            start: Instant::now(),
            label,
            metrics,
        }
    }
}

impl Drop for MetricsGuard<'_> {
    fn drop(&mut self) { self.metrics.add(self.label, self.start.elapsed()); }
}

#[cfg(test)]
mod t1_12_tests {
    use super::BlockDiff;
    use common::terrain::{Block, sprite::SpriteKind};
    use vek::Vec3;

    fn diff(old: Block, new: Block) -> BlockDiff {
        BlockDiff {
            wpos: Vec3::zero(),
            old,
            new,
        }
    }

    #[test]
    fn t1_12_resource_change_predicate() {
        let stones = Block::air(SpriteKind::Stones); // rtsim resource: Stone
        let stones2 = Block::air(SpriteKind::Stones2); // ALSO Stone
        let empty = Block::air(SpriteKind::Empty); // no rtsim resource
        let iron = Block::air(SpriteKind::Iron); // rtsim resource: Ore

        // Sanity on the underlying classifier the predicate rides.
        assert!(stones.get_rtsim_resource().is_some());
        assert_eq!(stones.get_rtsim_resource(), stones2.get_rtsim_resource());
        assert!(empty.get_rtsim_resource().is_none());

        // A resource block mined to nothing IS a class change → must forward.
        assert!(diff(stones, empty).changes_rtsim_resource());
        // Nothing growing INTO a resource IS a class change → must forward.
        assert!(diff(empty, iron).changes_rtsim_resource());
        // One class to a DIFFERENT class → must forward.
        assert!(diff(stones, iron).changes_rtsim_resource());

        // Two sprites of the SAME class → NOT a class change → no hook.
        assert!(!diff(stones, stones2).changes_rtsim_resource());
        // A pure non-resource edit → no hook.
        assert!(!diff(empty, empty).changes_rtsim_resource());
        // Identity → no hook.
        assert!(!diff(iron, iron).changes_rtsim_resource());
    }
}
