use std::{
    io,
    num::NonZeroU64,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use super::{
    CommandResults,
    errors::{PluginModuleError, PluginPreflightErrorV1},
    memory_manager::{EcsAccessManager, EcsWorld},
};
use hashbrown::{HashMap, HashSet};
use tokio::io::AsyncWrite;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, HasSelf, Linker},
};
use wasmtime_wasi::{
    HostMonotonicClock, HostWallClock, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView,
    cli::{IsTerminal, StdoutStream},
    p2::Pollable,
};

pub(crate) mod types_mod {
    wasmtime::component::bindgen!({
        path: "../../plugin/wit/veloren.wit",
        world: "common-types",
    });
}

wasmtime::component::bindgen!({
    path: "../../plugin/wit/veloren.wit",
    world: "plugin",
    with: {
        "veloren:plugin/types@0.0.1": types_mod::veloren::plugin::types,
        "veloren:plugin/information@0.0.1.entity": Entity,
    },
});

mod animation_plugin {
    wasmtime::component::bindgen!({
        path: "../../plugin/wit/veloren.wit",
        world: "animation-plugin",
        with: {
            "veloren:plugin/types@0.0.1": super::types_mod::veloren::plugin::types,
        },
    });
}

mod server_plugin {
    wasmtime::component::bindgen!({
        path: "../../plugin/wit/veloren.wit",
        world: "server-plugin",
        with: {
            "veloren:plugin/types@0.0.1": super::types_mod::veloren::plugin::types,
            "veloren:plugin/information@0.0.1.entity": super::Entity,
        },
    });
}

pub struct Entity {
    uid: common::uid::Uid,
}

pub use animation::Body;
use exports::veloren::plugin::animation;
pub use types_mod::veloren::plugin::types::{
    self, CharacterState, Dependency, Skeleton, Transform,
};
use veloren::plugin::{actions, information};

type StoreType = wasmtime::Store<WasiHostCtx>;

/// This enum abstracts over the different types of plugins we defined
enum PluginWrapper {
    Full(Plugin),
    Animation(animation_plugin::AnimationPlugin),
    Server(server_plugin::ServerPlugin),
}

impl PluginWrapper {
    fn load_event<S: wasmtime::AsContextMut>(
        &self,
        store: S,
        mode: common::resources::GameMode,
    ) -> wasmtime::Result<()>
    where
        <S as wasmtime::AsContext>::Data: std::marker::Send,
    {
        let mode = match mode {
            common::resources::GameMode::Server => types::GameMode::Server,
            common::resources::GameMode::Client => types::GameMode::Client,
            common::resources::GameMode::Singleplayer => types::GameMode::SinglePlayer,
        };
        match self {
            PluginWrapper::Full(pl) => pl.veloren_plugin_events().call_load(store, mode),
            PluginWrapper::Animation(pl) => pl.veloren_plugin_events().call_load(store, mode),
            PluginWrapper::Server(pl) => pl.veloren_plugin_events().call_load(store, mode),
        }
    }

    fn command_event<S: wasmtime::AsContextMut>(
        &self,
        store: S,
        name: &str,
        args: &[String],
        player: types::Uid,
    ) -> wasmtime::Result<Result<Vec<String>, String>>
    where
        <S as wasmtime::AsContext>::Data: std::marker::Send,
    {
        match self {
            PluginWrapper::Full(pl) => pl
                .veloren_plugin_server_events()
                .call_command(store, name, args, player),
            PluginWrapper::Animation(_) => Ok(Err("not implemented".into())),
            PluginWrapper::Server(pl) => pl
                .veloren_plugin_server_events()
                .call_command(store, name, args, player),
        }
    }

    fn player_join_event(
        &self,
        store: &mut StoreType,
        name: &str,
        uuid: (types::Uid, types::Uid),
    ) -> wasmtime::Result<types::JoinResult> {
        match self {
            PluginWrapper::Full(pl) => pl
                .veloren_plugin_server_events()
                .call_join(store, name, uuid),
            PluginWrapper::Animation(_) => Ok(types::JoinResult::None),
            PluginWrapper::Server(pl) => pl
                .veloren_plugin_server_events()
                .call_join(store, name, uuid),
        }
    }

    fn create_body(&self, store: &mut StoreType, bodytype: i32) -> Option<animation::Body> {
        match self {
            PluginWrapper::Full(pl) => {
                let body_iface = pl.veloren_plugin_animation().body();
                body_iface.call_constructor(store, bodytype).ok()
            },
            PluginWrapper::Animation(pl) => {
                let body_iface = pl.veloren_plugin_animation().body();
                body_iface.call_constructor(store, bodytype).ok()
            },
            PluginWrapper::Server(_) => None,
        }
    }

    fn update_skeleton(
        &self,
        store: &mut StoreType,
        body: animation::Body,
        dep: types::Dependency,
        time: f32,
    ) -> Option<types::Skeleton> {
        match self {
            PluginWrapper::Full(pl) => {
                let body_iface = pl.veloren_plugin_animation().body();
                body_iface.call_update_skeleton(store, body, dep, time).ok()
            },
            PluginWrapper::Animation(pl) => {
                let body_iface = pl.veloren_plugin_animation().body();
                body_iface.call_update_skeleton(store, body, dep, time).ok()
            },
            PluginWrapper::Server(_) => None,
        }
    }
}

/// This structure represent the WASM State of the plugin.
pub struct PluginModule {
    ecs: Arc<EcsAccessManager>,
    plugin: PluginWrapper,
    store: Mutex<wasmtime::Store<WasiHostCtx>>,
    name: String,
    /// APEX-T2.5.14: per-event fuel budget (u64::MAX = legacy unlimited).
    fuel_per_event: u64,
}

struct WasiHostCtx {
    preview2_ctx: WasiCtx,
    preview2_table: wasmtime::component::ResourceTable,
    ecs: Arc<EcsAccessManager>,
    registered_commands: HashSet<String>,
    registered_bodies: HashMap<String, types::BodyIndex>,
    /// APEX-T2.5.14: per-store resource ceilings (memory/table growth),
    /// from the deployment policy for governed modules; unlimited for
    /// legacy modules (behavior-preserving).
    limits: wasmtime::StoreLimits,
}

/// APEX-T2.5.14 — the per-store slice of the deployment policy's
/// per-mode runtime limits. No `Default`: legacy modules pass `None`
/// explicitly (recorded as unlimited), governed modules carry policy
/// values.
#[derive(Clone, Copy, Debug)]
pub struct PluginStoreLimitsV1 {
    pub max_linear_memory_bytes: u64,
    pub max_fuel_per_event: u64,
}

impl WasiView for WasiHostCtx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.preview2_ctx,
            table: &mut self.preview2_table,
        }
    }
}

impl information::Host for WasiHostCtx {}

impl types::Host for WasiHostCtx {}

impl actions::Host for WasiHostCtx {
    fn register_command(&mut self, name: String) {
        tracing::info!("Plugin registers /{name}");
        self.registered_commands.insert(name);
    }

    fn player_send_message(&mut self, uid: actions::Uid, text: String) {
        tracing::info!("Plugin sends message {text} to player {uid:?}");
    }

    fn register_animation(&mut self, name: String, id: types::BodyIndex) {
        let _ = self.registered_bodies.insert(name, id);
    }
}

impl information::HostEntity for WasiHostCtx {
    fn find_entity(
        &mut self,
        uid: actions::Uid,
    ) -> Result<wasmtime::component::Resource<information::Entity>, types::Error> {
        self.ctx()
            .table
            .push(Entity {
                uid: common::uid::Uid(NonZeroU64::new(uid).ok_or(types::Error::RuntimeError)?),
            })
            .map_err(|_err| types::Error::RuntimeError)
    }

    fn health(
        &mut self,
        self_: wasmtime::component::Resource<information::Entity>,
    ) -> Result<information::Health, types::Error> {
        let uid = self
            .ctx()
            .table
            .get(&self_)
            .map_err(|_err| types::Error::RuntimeError)?
            .uid;
        self.ecs.with(|world| {
            let world = world.ok_or(types::Error::EcsPointerNotAvailable)?;
            let player = world
                .id_maps
                .uid_entity(uid)
                .ok_or(types::Error::EcsEntityNotFound)?;
            world
                .health
                .get(player)
                .map(|health| information::Health {
                    current: health.current(),
                    base_max: health.base_max(),
                    maximum: health.maximum(),
                })
                .ok_or(types::Error::EcsComponentNotFound)
        })
    }

    fn name(
        &mut self,
        self_: wasmtime::component::Resource<information::Entity>,
    ) -> Result<String, types::Error> {
        let uid = self
            .ctx()
            .table
            .get(&self_)
            .map_err(|_err| types::Error::RuntimeError)?
            .uid;
        self.ecs.with(|world| {
            let world = world.ok_or(types::Error::EcsPointerNotAvailable)?;
            let player = world
                .id_maps
                .uid_entity(uid)
                .ok_or(types::Error::EcsEntityNotFound)?;
            Ok(world
                .player
                .get(player)
                .ok_or(types::Error::EcsComponentNotFound)?
                .alias
                .to_owned())
        })
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<information::Entity>,
    ) -> wasmtime::Result<()> {
        Ok(self.ctx().table.delete(rep).map(|_entity| ())?)
    }
}

struct InfoStream(String);

impl AsyncWrite for InfoStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        tracing::info!("{}: {}", self.0, String::from_utf8_lossy(buf));
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[wasmtime_wasi::async_trait]
impl Pollable for InfoStream {
    async fn ready(&mut self) {}
}

struct ErrorStream(String);

impl AsyncWrite for ErrorStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        tracing::error!("{}: {}", self.0, String::from_utf8_lossy(buf));
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[wasmtime_wasi::async_trait]
impl Pollable for ErrorStream {
    async fn ready(&mut self) {}
}

struct LogStream(String, tracing::Level);

impl IsTerminal for LogStream {
    fn is_terminal(&self) -> bool { true }
}

impl StdoutStream for LogStream {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        if self.1 == tracing::Level::INFO {
            Box::new(InfoStream(self.0.clone()))
        } else {
            Box::new(ErrorStream(self.0.clone()))
        }
    }
}

// DET-PLG-002 (determinism audit): deny the host wall/monotonic clocks to
// plugin hooks and inject deterministic replacements.
//
// Wasmtime's default `WasiCtxBuilder` wires the *host* clocks into every plugin
// store. That leaves plugin hook behaviour dependent on launch time,
// command-arrival delay, scheduler stalls, and machine load even when the
// accepted game history and plugin traversal order are identical — an ambient
// nondeterministic input, the time-domain twin of RNG-P3-001. We replace both
// clocks so no host time can enter a hook. The WASI contract only requires the
// wall clock to report Unix time and the monotonic clock to be non-decreasing;
// both hold here.

/// Fixed Unix timestamp (seconds) reported to every plugin's wall clock. Any
/// constant works; a frozen value means the wall clock is a pure function of
/// nothing external, so it cannot carry real time into a hook.
const PLUGIN_WALL_CLOCK_UNIX_SECS: u64 = 1_600_000_000; // 2020-09-13T12:26:40Z

/// Nanoseconds the deterministic monotonic clock advances per read. A nonzero
/// step keeps the clock strictly increasing, so a guest that waits on elapsed
/// monotonic time makes deterministic progress instead of stalling on a frozen
/// value, while any two identical replays observe an identical read sequence.
const PLUGIN_MONOTONIC_STEP_NANOS: u64 = 1_000;

/// Frozen wall clock: always reports the same Unix instant (see DET-PLG-002).
struct DeterministicWallClock;

impl HostWallClock for DeterministicWallClock {
    fn resolution(&self) -> Duration { Duration::from_secs(1) }

    fn now(&self) -> Duration { Duration::from_secs(PLUGIN_WALL_CLOCK_UNIX_SECS) }
}

/// Monotonic clock driven by a per-read counter rather than host time (see
/// DET-PLG-002). Each read returns the current value and advances by a fixed
/// step; the sequence is a pure function of the guest's own (already
/// deterministic) execution, so it replays identically.
struct DeterministicMonotonicClock {
    now_nanos: AtomicU64,
}

impl DeterministicMonotonicClock {
    fn new() -> Self {
        Self {
            now_nanos: AtomicU64::new(0),
        }
    }
}

impl HostMonotonicClock for DeterministicMonotonicClock {
    fn resolution(&self) -> u64 { PLUGIN_MONOTONIC_STEP_NANOS }

    fn now(&self) -> u64 {
        self.now_nanos
            .fetch_add(PLUGIN_MONOTONIC_STEP_NANOS, Ordering::Relaxed)
    }
}

/// `APEX-T2.5.14` — THE process Wasmtime runtime: one explicitly
/// configured `Engine` shared by every module, replacing the old
/// engine-per-module construction whose `Config` defaults were
/// unrecorded. Every non-default knob lives HERE, in one auditable
/// place; per-module code can no longer make config decisions at all.
/// (`Engine` is `Send + Sync` and internally shared by design —
/// wasmtime documents cross-store engine sharing as the intended use.)
pub struct PluginRuntimeV1 {
    engine: Engine,
}

/// The explicit V1 config: component model ON; everything else is
/// wasmtime's documented default, deliberately unmodified — recorded by
/// this constant's existence rather than scattered per call site.
pub const PLUGIN_RUNTIME_CONFIG_TAG_V1: &str = "bastion.plugin-runtime/v1:component-model+fuel";

impl PluginRuntimeV1 {
    fn new() -> Result<Self, wasmtime::Error> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        // APEX-T2.5.14: fuel metering is ON for every module (one engine,
        // one config — no governed/legacy engine split). Legacy modules
        // get u64::MAX fuel so their behavior is preserved; the metering
        // overhead is the disclosed cost of a single shared engine.
        config.consume_fuel(true);
        Ok(Self { engine: Engine::new(&config)? })
    }

    pub fn engine(&self) -> &Engine { &self.engine }
}

/// The one runtime, constructed on first use. A construction failure is
/// remembered (typed) — every subsequent module creation fails the same
/// way rather than retrying its own private engine.
pub fn plugin_runtime_v1() -> Result<&'static PluginRuntimeV1, PluginModuleError> {
    static RUNTIME: std::sync::OnceLock<Result<PluginRuntimeV1, String>> = std::sync::OnceLock::new();
    RUNTIME
        .get_or_init(|| PluginRuntimeV1::new().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| PluginModuleError::RuntimeUnavailable { detail: e.clone() })
}

/// `APEX-T2.5.15` — a component that has passed the FULL preflight:
/// compiled, host linker built, and every import resolved/typechecked
/// via `instantiate_pre`. Holding one has no side effects; actual
/// instantiation, wrapper construction, and load hooks remain separate
/// fallible stages.
pub struct PreparedPluginModuleV1 {
    name: String,
    instance_pre: wasmtime::component::InstancePre<WasiHostCtx>,
}

impl PreparedPluginModuleV1 {
    pub fn name(&self) -> &str { &self.name }
}

/// `APEX-T2.5.15` — the preflight: exact bytes → component → import
/// resolution/typecheck, each stage a distinct typed terminal. Runs
/// BEFORE any store, instance, wrapper, or publication exists.
pub fn preflight_component_v1(
    name: &str,
    wasm_data: &[u8],
) -> Result<PreparedPluginModuleV1, PluginPreflightErrorV1> {
    let engine = plugin_runtime_v1()
        .map_err(|e| PluginPreflightErrorV1::RuntimeUnavailable { detail: format!("{e:?}") })?
        .engine();
    let component = Component::from_binary(engine, wasm_data).map_err(|e| {
        PluginPreflightErrorV1::CompileFailed { module: name.to_owned(), detail: e.to_string() }
    })?;
    let mut linker: Linker<WasiHostCtx> = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|e| {
        PluginPreflightErrorV1::LinkerSetupFailed { module: name.to_owned(), detail: e.to_string() }
    })?;
    Plugin::add_to_linker::<_, HasSelf<_>>(&mut linker, |x| x).map_err(|e| {
        PluginPreflightErrorV1::LinkerSetupFailed { module: name.to_owned(), detail: e.to_string() }
    })?;
    let instance_pre = linker.instantiate_pre(&component).map_err(|e| {
        PluginPreflightErrorV1::ImportResolutionFailed { module: name.to_owned(), detail: e.to_string() }
    })?;
    Ok(PreparedPluginModuleV1 { name: name.to_owned(), instance_pre })
}

impl PluginModule {
    /// This function takes bytes from a WASM File and compile them.
    /// `limits = None` = legacy/ungoverned (unlimited, recorded);
    /// `Some` = the deployment policy's per-mode ceilings.
    pub fn new(
        name: String,
        wasm_data: &[u8],
        limits: Option<PluginStoreLimitsV1>,
    ) -> Result<Self, PluginModuleError> {
        // APEX-T2.5.15: even the single-module path goes through the
        // full preflight — there is no instantiate-without-preflight.
        // (Legacy path: no declared world, probing preserved.)
        let prepared = preflight_component_v1(&name, wasm_data).map_err(PluginModuleError::Preflight)?;
        Self::new_from_prepared(&prepared, limits, None)
    }

    /// `APEX-T2.5.15/.16` — instantiate from a PREFLIGHTED component:
    /// store setup + `InstancePre::instantiate` + declared-world wrapper
    /// selection (`expected_world = None` = legacy probing). Compile and
    /// import failures are impossible here by construction.
    pub fn new_from_prepared(
        prepared: &PreparedPluginModuleV1,
        limits: Option<PluginStoreLimitsV1>,
        expected_world: Option<super::manifest::PluginModuleWorldV1>,
    ) -> Result<Self, PluginModuleError> {
        let name = prepared.name.clone();
        let ecs = Arc::new(EcsAccessManager::default());

        // APEX-T2.5.14: the SHARED engine — no per-module Config exists.
        let engine = plugin_runtime_v1()?.engine().clone();
        // create a WASI environment (std implementing system calls)
        // RNG-P3-001 (determinism audit): seed the WASI *insecure* random
        // deterministically by plugin identity. WASI's own contract splits
        // `wasi:random/random` (secure — MUST stay unpredictable; untouched
        // here) from `wasi:random/insecure` + `insecure-seed` (explicitly
        // documented as non-crypto and permitted to be deterministic) — so
        // this is in-contract and does not weaken the sandbox: crypto
        // consumers must use the secure interface, which keeps OS entropy.
        let insecure_seed = common::state_hash::stable_hash_u64(
            "bastion/domain/wasi-insecure-random/v1",
            &name,
        );
        let wasi = WasiCtxBuilder::new()
            .stdout(LogStream(name.clone(), tracing::Level::INFO))
            .stderr(LogStream(name.clone(), tracing::Level::ERROR))
            .insecure_random_seed(insecure_seed as u128)
            // DET-PLG-002: replace the host clocks with deterministic ones so
            // plugin hooks cannot observe real wall/monotonic time.
            .wall_clock(DeterministicWallClock)
            .monotonic_clock(DeterministicMonotonicClock::new())
            .build();
        let store_limits = match &limits {
            Some(l) => wasmtime::StoreLimitsBuilder::new()
                .memory_size(l.max_linear_memory_bytes as usize)
                .build(),
            None => wasmtime::StoreLimitsBuilder::new().build(), // unlimited (legacy)
        };
        let host_ctx = WasiHostCtx {
            preview2_ctx: wasi,
            preview2_table: wasmtime_wasi::ResourceTable::new(),
            ecs: Arc::clone(&ecs),
            registered_commands: HashSet::new(),
            registered_bodies: HashMap::new(),
            limits: store_limits,
        };
        // the store contains all data of a wasm instance
        let mut store = Store::new(&engine, host_ctx);
        store.limiter(|ctx| &mut ctx.limits);
        let fuel_per_event = limits.map_or(u64::MAX, |l| l.max_fuel_per_event);
        store
            .set_fuel(fuel_per_event)
            .expect("fuel metering enabled by PluginRuntimeV1 construction");

        // APEX-T2.5.15: instantiate from the PREFLIGHTED InstancePre —
        // compile and import resolution already happened before any
        // store existed; only instantiation itself can fail here.
        let instance =
            prepared.instance_pre.instantiate(&mut store).map_err(PluginModuleError::Wasmtime)?;

        // APEX-T2.5.16: a module whose manifest DECLARED a world gets
        // exactly that wrapper — a missing/mismatched export is a typed
        // terminal, never a silent fallback to a different world.
        // Probing survives ONLY for legacy manifests (no declaration).
        let plugin = match expected_world {
            Some(super::manifest::PluginModuleWorldV1::Plugin) => Plugin::new(&mut store, &instance)
                .map(PluginWrapper::Full)
                .map_err(|e| PluginModuleError::WorldMismatch {
                    module: name.clone(),
                    declared: "plugin",
                    detail: e.to_string(),
                })?,
            Some(super::manifest::PluginModuleWorldV1::AnimationPlugin) => {
                animation_plugin::AnimationPlugin::new(&mut store, &instance)
                    .map(PluginWrapper::Animation)
                    .map_err(|e| PluginModuleError::WorldMismatch {
                        module: name.clone(),
                        declared: "animation-plugin",
                        detail: e.to_string(),
                    })?
            },
            Some(super::manifest::PluginModuleWorldV1::ServerPlugin) => {
                server_plugin::ServerPlugin::new(&mut store, &instance)
                    .map(PluginWrapper::Server)
                    .map_err(|e| PluginModuleError::WorldMismatch {
                        module: name.clone(),
                        declared: "server-plugin",
                        detail: e.to_string(),
                    })?
            },
            None => match Plugin::new(&mut store, &instance) {
                Ok(pl) => Ok(PluginWrapper::Full(pl)),
                Err(_) => match animation_plugin::AnimationPlugin::new(&mut store, &instance) {
                    Ok(pl) => Ok(PluginWrapper::Animation(pl)),
                    Err(_) => server_plugin::ServerPlugin::new(&mut store, &instance)
                        .map(PluginWrapper::Server),
                },
            }
            .map_err(PluginModuleError::Wasmtime)?,
        };

        Ok(Self {
            plugin,
            ecs,
            store: store.into(),
            name,
            fuel_per_event,
        })
    }

    pub fn name(&self) -> &str { &self.name }

    /// APEX-T2.5.19 — the ACTUAL registrations this module's load hooks
    /// made, as canonical sorted sets (receipt input; compared against
    /// the manifest's declared claims).
    pub fn actual_registrations_v1(&mut self) -> (Vec<String>, Vec<String>) {
        let store = self.store.get_mut().unwrap();
        let mut commands: Vec<String> = store.data().registered_commands.iter().cloned().collect();
        commands.sort_unstable();
        let mut bodies: Vec<String> = store.data().registered_bodies.keys().cloned().collect();
        bodies.sort_unstable();
        (commands, bodies)
    }

    /// APEX-T2.5.14: `max_fuel_per_event` semantics — every host-invoked
    /// event starts from a full per-event budget (a well-behaved event
    /// can never be starved by an earlier one; a runaway event traps at
    /// ITS OWN ceiling). Called at the head of every public entry point.
    fn refuel(&mut self) {
        self.store
            .get_mut()
            .unwrap()
            .set_fuel(self.fuel_per_event)
            .expect("fuel metering enabled by PluginRuntimeV1 construction");
    }

    // Implementation of the commands called from veloren and provided in plugins
    pub fn load_event(
        &mut self,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<(), PluginModuleError> {
        self.refuel();
        self.ecs
            .execute_with(ecs, || {
                self.plugin.load_event(self.store.get_mut().unwrap(), mode)
            })
            .map_err(PluginModuleError::Wasmtime)
    }

    pub fn command_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        args: &[String],
        player: common::uid::Uid,
    ) -> Result<Vec<String>, CommandResults> {
        self.refuel();
        if !self
            .store
            .get_mut()
            .unwrap()
            .data()
            .registered_commands
            .contains(name)
        {
            return Err(CommandResults::UnknownCommand);
        }
        self.ecs.execute_with(ecs, || {
            match self.plugin.command_event(
                self.store.get_mut().unwrap(),
                name,
                args,
                player.0.into(),
            ) {
                Err(err) => Err(CommandResults::HostError(err)),
                Ok(result) => result.map_err(CommandResults::PluginError),
            }
        })
    }

    pub fn player_join_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        uuid: common::uuid::Uuid,
    ) -> types::JoinResult {
        self.refuel();
        self.ecs.execute_with(ecs, || {
            match self.plugin.player_join_event(
                self.store.get_mut().unwrap(),
                name,
                uuid.as_u64_pair(),
            ) {
                Ok(value) => {
                    tracing::info!("JoinResult {value:?}");
                    value
                },
                Err(err) => {
                    tracing::error!("join_event: {err:?}");
                    types::JoinResult::None
                },
            }
        })
    }

    pub fn create_body(&mut self, bodytype: &str) -> Option<animation::Body> {
        self.refuel();
        let store = self.store.get_mut().unwrap();
        let bodytype = store.data().registered_bodies.get(bodytype).copied();
        bodytype.and_then(|bd| self.plugin.create_body(store, bd))
    }

    pub fn update_skeleton(
        &mut self,
        body: &animation::Body,
        dep: &types::Dependency,
        time: f32,
    ) -> Option<types::Skeleton> {
        self.refuel();
        self.plugin
            .update_skeleton(self.store.get_mut().unwrap(), *body, *dep, time)
    }
}

/// `APEX-T2.5.15` — component preflight canaries: each stage's terminal
/// driven with a real (wat-built) fixture; no store or instance exists
/// at any point in these tests.
#[cfg(test)]
mod plugin_component_preflight_v1 {
    use super::*;

    #[test]
    fn preflight_stages_produce_their_own_terminals() {
        // Malformed bytes: compile terminal.
        assert!(matches!(
            preflight_component_v1("junk", b"not wasm at all"),
            Err(PluginPreflightErrorV1::CompileFailed { .. })
        ));

        // A valid but EMPTY component (no imports, no exports): preflight
        // PASSES — imports resolve trivially. (Missing exports are the
        // wrapper stage's business, .16.)
        let empty = wat::parse_str("(component)").unwrap();
        assert!(preflight_component_v1("empty", &empty).is_ok());

        // Core wasm module (not a component): compile-stage refusal under
        // the component-model config.
        let core = wat::parse_str("(module)").unwrap();
        assert!(matches!(
            preflight_component_v1("core", &core),
            Err(PluginPreflightErrorV1::CompileFailed { .. })
        ));

        // Unknown import: instantiate_pre resolution terminal — the
        // exact class that used to surface only at live instantiation.
        // NB: an EMPTY instance import is trivially satisfiable (observed
        // directly — wasmtime provides it implicitly), so the fixture
        // must import real content for resolution to have work to do.
        let unknown_import = wat::parse_str(
            r#"(component (import "nonexistent:pkg/iface@0.0.1" (instance (export "f" (func)))))"#,
        )
        .unwrap();
        assert!(matches!(
            preflight_component_v1("ghost", &unknown_import),
            Err(PluginPreflightErrorV1::ImportResolutionFailed { .. })
        ));
    }
}

/// `APEX-T2.5.16` — declared-world enforcement canaries.
#[cfg(test)]
mod plugin_declared_world_v1 {
    use super::*;

    #[test]
    fn declared_world_is_enforced_and_legacy_probes() {
        let empty = wat::parse_str("(component)").unwrap();
        let prepared = preflight_component_v1("w", &empty).unwrap();

        // Declared world + missing exports = the typed mismatch terminal,
        // for every world — never a fallback to another wrapper.
        for world in [
            super::super::manifest::PluginModuleWorldV1::Plugin,
            super::super::manifest::PluginModuleWorldV1::ServerPlugin,
            super::super::manifest::PluginModuleWorldV1::AnimationPlugin,
        ] {
            assert!(matches!(
                PluginModule::new_from_prepared(&prepared, None, Some(world)),
                Err(PluginModuleError::WorldMismatch { .. })
            ));
        }
        // Legacy (no declaration): probing path, generic wasmtime error —
        // byte-compatible with the old behavior class.
        assert!(matches!(
            PluginModule::new_from_prepared(&prepared, None, None),
            Err(PluginModuleError::Wasmtime(_))
        ));
    }

    #[test]
    fn world_extraction_reads_v1_and_ignores_legacy() {
        let v1 = b"manifest_version = 1\n[[modules]]\npath = \"m.wasm\"\nworld = \"server-plugin\"\n";
        let worlds = super::super::extract_declared_worlds_v1(v1).unwrap();
        assert_eq!(
            worlds.get(std::path::Path::new("m.wasm")),
            Some(&super::super::manifest::PluginModuleWorldV1::ServerPlugin)
        );
        assert!(super::super::extract_declared_worlds_v1(b"name = \"old\"\nmodules = []\n").is_none());
        assert!(super::super::extract_declared_worlds_v1(b"\xff\xfe not utf8").is_none());
    }
}
