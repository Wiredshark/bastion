use common::{clock::Clock, match_some};
use crossbeam_channel::{Receiver, Sender, TryRecvError, bounded, unbounded};
use i18n::LocalizationHandle;
use rand::seq::IteratorRandom;
use server::{
    Error as ServerError, Event, Input, Server, ServerInitStage,
    persistence::{DatabaseSettings, SqlLogMode},
    settings::server_description::ServerDescription,
};

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::runtime::Runtime;
use tracing::{error, info, trace, warn};

mod singleplayer_world;
pub use singleplayer_world::{SingleplayerWorld, SingleplayerWorlds};

const TPS: u64 = 30;

/// Used to start and stop the background thread running the server
/// when in singleplayer mode.
pub struct Singleplayer {
    _server_thread: JoinHandle<()>,
    stop_server_s: Sender<()>,
    pub receiver: Receiver<Result<(), ServerError>>,
    pub init_stage_receiver: Receiver<ServerInitStage>,
    // Wether the server is stopped or not
    paused: Arc<AtomicBool>,
}

impl Singleplayer {
    /// Returns wether or not the server is paused
    pub fn is_paused(&self) -> bool { self.paused.load(Ordering::SeqCst) }

    /// Pauses if true is passed and unpauses if false (Does nothing if in that
    /// state already)
    pub fn pause(&self, state: bool) { self.paused.store(state, Ordering::SeqCst); }
}

impl Drop for Singleplayer {
    fn drop(&mut self) {
        // Ignore the result
        let _ = self.stop_server_s.send(());
    }
}

#[derive(Default)]
pub enum SingleplayerState {
    #[default]
    None,
    Init(SingleplayerWorlds),
    Running(Singleplayer),
}

impl SingleplayerState {
    pub fn init() -> Self {
        let dir = common_base::userdata_dir();

        Self::Init(SingleplayerWorlds::load(&dir))
    }

    pub fn run(
        &mut self,
        runtime: &Arc<Runtime>,
        selected_language: &String,
        i18n: &LocalizationHandle,
        bastion_overseer: bool,
    ) {
        if let Self::Init(worlds) = self {
            let Some(world) = worlds.current() else {
                error!("Failed to get the current world.");
                return;
            };
            let server_data_dir = world.path.clone();

            let mut settings = server::Settings::singleplayer(&server_data_dir);
            let mut editable_settings = server::EditableSettings::singleplayer(&server_data_dir);

            let i18n = i18n.read();
            let motd = ["hud-chat-singleplayer-motd1", "hud-chat-singleplayer-motd2"]
                .iter()
                .choose(&mut rand::rng())
                .expect("Message of the day don't wanna play.");

            editable_settings.server_description.descriptions.insert(
                selected_language.to_string(),
                ServerDescription {
                    motd: i18n.get_msg(motd).to_string(),
                    rules: None,
                },
            );

            let file_opts = if let Some(gen_opts) = &world.gen_opts
                && !world.is_generated
            {
                server::FileOpts::Save(world.map_path.clone(), gen_opts.clone())
            } else {
                if !world.is_generated && world.gen_opts.is_none() {
                    world.copy_default_world();
                }
                server::FileOpts::Load(world.map_path.clone())
            };

            settings.map_file = Some(file_opts);
            settings.world_seed = world.seed;
            settings.day_length = world.day_length;
            // bastion (B-LIVE2, Ben's "the day is the same speed"): the
            // TimeScale mechanism already multiplies the day advance
            // (state.rs — landed with TIMECTL); the imperceptibility was
            // the BASE day being 30 real-minutes (per-world meta), so 4×
            // still crawled. In overseer/colony mode the TIMESCALE-DESIGN
            // target is a 10-minute day at 1× (→ 2.5 min at 4×, visibly
            // fast). Flag-scoped: the world meta stays vanilla and
            // vanilla sessions are untouched.
            if bastion_overseer {
                settings.day_length = 10.0;
            }

            let (stop_server_s, stop_server_r) = unbounded();

            let (server_stage_tx, server_stage_rx) = unbounded();

            // Create server

            // Relative to data_dir
            const PERSISTENCE_DB_DIR: &str = "saves";

            let database_settings = DatabaseSettings {
                db_dir: server_data_dir.join(PERSISTENCE_DB_DIR),
                sql_log_mode: SqlLogMode::Disabled, /* Voxygen doesn't take in command-line
                                                     * arguments
                                                     * so SQL logging can't be enabled for
                                                     * singleplayer without changing this line
                                                     * manually */
            };

            let paused = Arc::new(AtomicBool::new(false));
            let paused1 = Arc::clone(&paused);

            let (result_sender, result_receiver) = bounded(1);

            let builder = thread::Builder::new().name("singleplayer-server-thread".into());
            let runtime = Arc::clone(runtime);
            let thread = builder
                .spawn(move || {
                    trace!("starting singleplayer server thread");

                    let (server, init_result) = match Server::new(
                        settings,
                        editable_settings,
                        database_settings,
                        &server_data_dir,
                        &|init_stage| {
                            let _ = server_stage_tx.send(init_stage);
                        },
                        runtime,
                    ) {
                        Ok(server) => (Some(server), Ok(())),
                        Err(err) => (None, Err(err)),
                    };

                    match (result_sender.send(init_result), server) {
                        (Err(e), _) => warn!(
                            ?e,
                            "Failed to send singleplayer server initialization result. Most \
                             likely the channel was closed by cancelling server creation. \
                             Stopping Server"
                        ),
                        (Ok(()), None) => (),
                        (Ok(()), Some(server)) => run_server(server, stop_server_r, paused1),
                    }

                    trace!("ending singleplayer server thread");
                })
                .unwrap();

            *self = SingleplayerState::Running(Singleplayer {
                _server_thread: thread,
                stop_server_s,
                init_stage_receiver: server_stage_rx,
                receiver: result_receiver,
                paused,
            });
        } else {
            error!("SingleplayerState::run was called, but singleplayer is already running!");
        }
    }

    /// bastion (B-ASSET1): boot the asset render arena — a THROWAWAY
    /// singleplayer world (fresh temp data dir each boot, default map asset,
    /// fixed seed, no persistence expectations) with the arena env vars set
    /// so the embedded server prepares the inspection pad. The vanilla `run`
    /// path is untouched; this never reads or writes the user's worlds.
    pub fn run_bastion_arena(
        &mut self,
        runtime: &Arc<Runtime>,
        asset_id: &str,
        asset_lab_dir: &std::path::Path,
    ) {
        if matches!(self, Self::Running(_)) {
            error!("run_bastion_arena called, but singleplayer is already running");
            return;
        }
        let server_data_dir =
            std::env::temp_dir().join(format!("bastion-asset-arena-{}", std::process::id()));
        if let Err(e) = std::fs::create_dir_all(&server_data_dir) {
            error!(?e, "could not create arena data dir");
            return;
        }

        // The env-var transport to the embedded server (read once in
        // Server::new; deliberately not a Settings field — Settings persists
        // to settings.ron and a transient CLI mode must not pollute it).
        // SAFETY: called from the main thread before the server thread (the
        // only reader) is spawned; no concurrent env access at this point.
        #[expect(unsafe_code)]
        unsafe {
            std::env::set_var("BASTION_ASSET_ARENA", asset_id);
            std::env::set_var(
                "BASTION_ASSET_LAB_DIR",
                asset_lab_dir
                    .canonicalize()
                    .unwrap_or_else(|_| asset_lab_dir.to_path_buf()),
            );
        }

        let mut settings = server::Settings::singleplayer(&server_data_dir);
        settings.map_file = None; // default pre-generated map asset
        settings.world_seed = 1337;
        let mut editable_settings = server::EditableSettings::singleplayer(&server_data_dir);
        editable_settings.server_description.descriptions.insert(
            "en".to_string(),
            ServerDescription {
                motd: "Bastion asset arena — /bastion_arena next|prev|fixture|dismiss".to_string(),
                rules: None,
            },
        );

        let database_settings = DatabaseSettings {
            db_dir: server_data_dir.join("saves"),
            sql_log_mode: SqlLogMode::Disabled,
        };

        let (stop_server_s, stop_server_r) = unbounded();
        let (server_stage_tx, server_stage_rx) = unbounded();
        let paused = Arc::new(AtomicBool::new(false));
        let paused1 = Arc::clone(&paused);
        let (result_sender, result_receiver) = bounded(1);

        let builder = thread::Builder::new().name("singleplayer-server-thread".into());
        let runtime = Arc::clone(runtime);
        let thread = builder
            .spawn(move || {
                trace!("starting bastion asset-arena server thread");
                let (server, init_result) = match Server::new(
                    settings,
                    editable_settings,
                    database_settings,
                    &server_data_dir,
                    &|init_stage| {
                        let _ = server_stage_tx.send(init_stage);
                    },
                    runtime,
                ) {
                    Ok(server) => (Some(server), Ok(())),
                    Err(err) => (None, Err(err)),
                };
                match (result_sender.send(init_result), server) {
                    (Err(e), _) => warn!(?e, "Failed to send arena server init result"),
                    (Ok(()), None) => (),
                    (Ok(()), Some(server)) => run_server(server, stop_server_r, paused1),
                }
                trace!("ending bastion asset-arena server thread");
            })
            .unwrap();

        *self = SingleplayerState::Running(Singleplayer {
            _server_thread: thread,
            stop_server_s,
            init_stage_receiver: server_stage_rx,
            receiver: result_receiver,
            paused,
        });
    }

    pub fn as_running(&self) -> Option<&Singleplayer> {
        match_some!(self, SingleplayerState::Running(s) => s)
    }

    pub fn as_init(&self) -> Option<&SingleplayerWorlds> {
        match_some!(self, SingleplayerState::Init(s) => s)
    }

    pub fn is_running(&self) -> bool { matches!(self, SingleplayerState::Running(_)) }
}

fn run_server(mut server: Server, stop_server_r: Receiver<()>, paused: Arc<AtomicBool>) {
    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::new(Duration::from_secs_f64(1.0 / TPS as f64));

    loop {
        // Check any event such as stopping and pausing
        match stop_server_r.try_recv() {
            Ok(()) => break,
            Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => (),
        }

        // Wait for the next tick.
        clock.tick();

        // R0D D1-replay (leg-12 fix): the client CONNECT tick is wall-timed,
        // so cross-run world histories diverged. In capture mode, AUTO-PAUSE
        // exactly when the first player appears (0 -> 1 transition, one-shot);
        // the session unpauses on entry and anchors the capture clock — every
        // run's post-anchor history is then tick-aligned to the client.
        {
            use std::sync::atomic::AtomicBool as AB;
            static R0D_ANCHORED: AB = AB::new(false);
            if crate::render::bastion_r0d::capture_config().is_some()
                && server.number_of_players() >= 1
                && !R0D_ANCHORED.swap(true, Ordering::SeqCst)
            {
                server.bastion_r0d_mark_anchor();
                paused.store(true, Ordering::SeqCst);
                info!("r0d: server auto-paused at first-player anchor");
            }
        }

        // Skip updating the server if it's paused
        if paused.load(Ordering::SeqCst) && server.number_of_players() < 2 {
            continue;
        } else if server.number_of_players() > 1 {
            paused.store(false, Ordering::SeqCst);
        }

        let events = server
            .tick(
                Input::default(),
                // DET-CLK-006 (determinism audit, DOMAIN ROOT): the
                // authoritative tick duration is the DECLARED fixed step,
                // never wall-clock game_dt() — host load/pauses/scheduling
                // must not reach authoritative state (Time/physics/AI/
                // persistence). `clock` remains the PACER (clock.tick()
                // still sleeps to target TPS) and a diagnostics source.
                // INTENDED live-behavior change per the full-determinism
                // mandate: an overloaded host now slows SIM TIME (fixed-
                // step, one tick per loop) instead of free-running larger
                // dts through gameplay.
                Duration::from_secs_f64(1.0 / TPS as f64),
            )
            .expect("Failed to tick server!");

        for event in events {
            match event {
                Event::ClientConnected { .. } => info!("Client connected!"),
                Event::ClientDisconnected { .. } => info!("Client disconnected!"),
                Event::Chat { entity: _, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick.
        server.cleanup();
    }
}
