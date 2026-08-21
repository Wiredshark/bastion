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
use vek::Vec3;

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

    /// Certification-only flat arena boot: a fresh fixed-seed world with one
    /// deterministic colony fixture. The CLI flag is explicit, the data root
    /// is process-unique, and normal singleplayer persistence is untouched.
    pub fn run_bastion_flat_arena(&mut self, runtime: &Arc<Runtime>) {
        if matches!(self, Self::Running(_)) {
            error!("run_bastion_flat_arena called, but singleplayer is already running");
            return;
        }
        server::bastion_enable_renderer_certification_determinism();
        let server_data_dir =
            std::env::temp_dir().join(format!("bastion-flat-arena-{}", std::process::id()));
        if let Err(e) = std::fs::create_dir_all(&server_data_dir) {
            error!(?e, "could not create flat arena data dir");
            return;
        }

        let mut settings = server::Settings::singleplayer(&server_data_dir);
        settings.map_file = None;
        settings.world_seed = 1337;
        let editable_settings = server::EditableSettings::singleplayer(&server_data_dir);
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
                trace!("starting bastion flat-arena server thread");
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
                    Ok(mut server) => {
                        let center = server.bastion_world_center_wpos();
                        let fixture_position = server::bastion_flat_arena::spawn_wpos(center);
                        let r1d_scale_smoke = std::env::var_os("BASTION_R1D_SCALE_SMOKE").is_some();
                        // Normal certification remains bounded to 64. The
                        // explicit R1D closure lane may request the already
                        // accepted 512-visible policy ceiling.
                        let maximum_figure_count = if r1d_scale_smoke { 512 } else { 64 };
                        let figure_count = std::env::var("BASTION_R1BC_FIGURE_COUNT")
                            .ok()
                            .and_then(|value| value.parse::<u16>().ok())
                            .filter(|count| (1..=maximum_figure_count).contains(count))
                            .unwrap_or(1);
                        let r1d_tier_smoke = std::env::var_os("BASTION_R1D_TIER_SMOKE").is_some();
                        let r1d_group_smoke = std::env::var_os("BASTION_R1D_GROUP_SMOKE").is_some();
                        let fixture = if figure_count == 1 {
                            server.bastion_spawn_colony(fixture_position, 1)
                        } else {
                            // The population certification lane must exercise
                            // genuinely compatible figures. Repeating the
                            // existing one-colonist fixture at a stable grid
                            // preserves the same deterministic body/equipment
                            // source while giving every figure a distinct
                            // world position and server identity.
                            let width = if r1d_scale_smoke { 32_u16 } else { 8_u16 };
                            (0..figure_count)
                                .flat_map(|ordinal| {
                                    let (x, y) = if r1d_scale_smoke {
                                        // Sixteen explicit rows of 32 people.
                                        // Presentation declarations, not these
                                        // positions, own group membership.
                                        let row = ordinal / width;
                                        let column = ordinal % width;
                                        let depth = f32::from(row) * 6.0;
                                        let lateral = (f32::from(column) - 15.5) * 2.0;
                                        (depth + lateral, depth - lateral)
                                    } else if r1d_group_smoke {
                                        // Two declared fixture-owned groups:
                                        // a near wedge and a middle-distance
                                        // four-column grid. Membership comes
                                        // from the explicit presentation
                                        // declaration, never these positions.
                                        if ordinal < 12 {
                                            let local = i32::from(ordinal);
                                            let row = (local + 1) / 2;
                                            let side = if local == 0 {
                                                0.0
                                            } else if local % 2 == 1 {
                                                -1.0
                                            } else {
                                                1.0
                                            };
                                            let depth = row as f32 * 1.8;
                                            let lateral = side * row as f32 * 1.8;
                                            (depth + lateral, depth - lateral)
                                        } else {
                                            let local = i32::from(ordinal - 12);
                                            let row = local / 4;
                                            let column = local % 4;
                                            let depth = 26.0 + row as f32 * 2.5;
                                            let lateral = (column as f32 - 1.5) * 2.2;
                                            (depth + lateral, depth - lateral)
                                        }
                                    } else if r1d_tier_smoke {
                                        // Four depth bands follow the declared
                                        // capture camera's +X/+Y view axis.
                                        // Lateral offsets keep bodies visibly
                                        // distinct without changing semantic
                                        // tier selection by enumeration order.
                                        let band = usize::from(ordinal / 6).min(3);
                                        let depth = [0.0_f32, 9.0, 22.0, 40.0][band];
                                        let lateral = (f32::from(ordinal % 6) - 2.5) * 1.6;
                                        (depth + lateral, depth - lateral)
                                    } else {
                                        (
                                            f32::from(ordinal % width) * 2.5,
                                            f32::from(ordinal / width) * 2.5,
                                        )
                                    };
                                    server.bastion_spawn_colony(
                                        fixture_position + Vec3::new(x, y, 0.0),
                                        1,
                                    )
                                })
                                .collect()
                        };
                        info!(
                            ?fixture,
                            figure_count,
                            r1d_tier_smoke,
                            r1d_group_smoke,
                            r1d_scale_smoke,
                            world_seed = 1337,
                            ?fixture_position,
                            "bastion: capture flat-arena fixture declared"
                        );
                        (Some(server), Ok(()))
                    },
                    Err(err) => (None, Err(err)),
                };
                match (result_sender.send(init_result), server) {
                    (Err(e), _) => warn!(?e, "Failed to send flat arena server init result"),
                    (Ok(()), None) => (),
                    (Ok(()), Some(server)) => run_server(server, stop_server_r, paused1),
                }
                trace!("ending bastion flat-arena server thread");
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
    let continuous_streaming_measurement =
        match crate::render::bastion_r0d::continuous_streaming_measurement_selected_v1() {
            Ok(selected) => selected,
            Err(fault) => {
                // Row-81 exclusion (3) FIXED: an invalid declaration used
                // to `return` here — killing the EMBEDDED SERVER (and the
                // whole session with it) over a diagnostic env typo. The
                // fault is recorded (any certification leg that expected
                // the measurement sees it and refuses on its own), the
                // error is loud, and the game keeps running unmeasured.
                crate::render::bastion_r0d::record_certification_fixture_fault_v1(fault);
                error!(
                    fault,
                    "bastion: invalid streaming-measurement declaration — measurement DISABLED, \
                     server continues (fault recorded for the certification path)"
                );
                false
            },
        };
    let certification_freeze_tick =
        crate::render::bastion_r0d::certification_freeze_tick_for_runtime_v1(
            std::env::var_os("BASTION_FLAT_ARENA").is_some(),
            crate::render::bastion_r0d::absolute_time_capture_selected(),
            continuous_streaming_measurement,
        );
    if certification_freeze_tick.is_some() || continuous_streaming_measurement {
        crate::render::bastion_r0d::reset_certification_server_latch_v1();
    }
    // W5 ops mode (BASTION_R0D_FREEZE_AFTER_LOGIN=1): the freeze counts
    // down from the moment a client is PRESENT instead of from boot. The
    // fixed-tick default lost a race against login on a loaded host — the
    // server froze mid-login and the client timed out (measured three
    // times tonight). Default unchanged: exact-tick freeze semantics.
    let freeze_after_login = std::env::var_os("BASTION_R0D_FREEZE_AFTER_LOGIN").is_some();
    let mut deferred_freeze_target: Option<u64> = if freeze_after_login {
        None // resolves when the first client arrives
    } else {
        certification_freeze_tick
    };
    let mut certification_weather_fixture =
        match crate::r1f_weather::certification_fixture_declaration() {
            crate::r1f_weather::CertificationFixtureDeclarationV1::Disabled => None,
            crate::r1f_weather::CertificationFixtureDeclarationV1::Requested(kind) => {
                Some((kind, None, false))
            },
            crate::r1f_weather::CertificationFixtureDeclarationV1::Invalid => {
                // Same repair as the streaming-measurement arm above: fail
                // LOUD, never fatal — a diagnostic env typo must not kill
                // the embedded server. The fault is recorded; any
                // certification leg that expected the fixture refuses on
                // its own evidence.
                let fault = "R1F_WEATHER_FIXTURE_INVALID_DECLARATION";
                crate::render::bastion_r0d::record_certification_fixture_fault_v1(fault);
                error!(
                    fault,
                    "bastion: invalid flat-arena weather fixture declaration — fixture \
                     DISABLED, server continues (fault recorded)"
                );
                None
            },
        };
    let mut completed_ticks = 0_u64;

    loop {
        // Check any event such as stopping and pausing
        match stop_server_r.try_recv() {
            Ok(()) => break,
            Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => (),
        }

        // Wait for the next tick.
        clock.tick();

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
        if let Some((kind, requested_zone_generation, acknowledged)) =
            &mut certification_weather_fixture
            && !*acknowledged
        {
            use server::bastion_weather_fixture::{
                BastionWeatherFixtureKindV1, BastionWeatherFixtureStepV1,
            };
            let server_kind = match *kind {
                crate::r1f_weather::CertificationFixtureKindV1::Clear => {
                    BastionWeatherFixtureKindV1::Clear
                },
                crate::r1f_weather::CertificationFixtureKindV1::Rain => {
                    BastionWeatherFixtureKindV1::Rain
                },
                crate::r1f_weather::CertificationFixtureKindV1::Storm => {
                    BastionWeatherFixtureKindV1::Storm
                },
            };
            match server.bastion_weather_fixture_step_v1(server_kind, *requested_zone_generation) {
                BastionWeatherFixtureStepV1::WaitingForWeatherJob => {},
                BastionWeatherFixtureStepV1::Queued { zone_generation } => {
                    *requested_zone_generation = Some(zone_generation);
                    info!(
                        ?kind,
                        zone_generation, "bastion: authoritative flat-arena weather fixture queued"
                    );
                },
                BastionWeatherFixtureStepV1::QueueGenerationOverflow => {
                    let fault = "R1F_WEATHER_FIXTURE_QUEUE_GENERATION_OVERFLOW";
                    crate::render::bastion_r0d::record_certification_fixture_fault_v1(fault);
                    error!(fault, "bastion: weather fixture queue generation overflow");
                    break;
                },
                BastionWeatherFixtureStepV1::WaitingForAuthoritativeSnapshot {
                    requested_zone_generation,
                    completed_zone_generation,
                    observed_kind,
                    observed_rain,
                } => {
                    trace!(
                        ?kind,
                        requested_zone_generation,
                        completed_zone_generation,
                        ?observed_kind,
                        observed_rain,
                        "bastion: awaiting authoritative flat-arena weather snapshot"
                    );
                },
                BastionWeatherFixtureStepV1::Acknowledged {
                    observed_kind,
                    observed_rain,
                } => {
                    *acknowledged = true;
                    info!(
                        ?kind,
                        ?observed_kind,
                        observed_rain,
                        "bastion: authoritative flat-arena weather fixture acknowledged"
                    );
                },
            }
        }
        let Some(next_completed_tick) = completed_ticks.checked_add(1) else {
            error!("bastion: renderer certification server tick overflow");
            break;
        };
        completed_ticks = next_completed_tick;
        if freeze_after_login
            && certification_freeze_tick.is_some()
            && deferred_freeze_target.is_none()
            && server.number_of_players() > 0
        {
            deferred_freeze_target =
                certification_freeze_tick.map(|f| completed_ticks.saturating_add(f));
            info!(
                ?deferred_freeze_target,
                completed_ticks, "bastion: deferred certification freeze armed (client present)"
            );
        }
        if deferred_freeze_target == Some(completed_ticks) {
            if certification_weather_fixture
                .as_ref()
                .is_some_and(|(_, _, acknowledged)| !acknowledged)
            {
                let fault = "R1F_WEATHER_FIXTURE_ACK_MISSING_BEFORE_FREEZE";
                crate::render::bastion_r0d::record_certification_fixture_fault_v1(fault);
                error!(
                    fault,
                    completed_ticks,
                    "bastion: weather fixture was not authoritative before certification freeze"
                );
                break;
            }
            paused.store(true, Ordering::SeqCst);
            info!(
                completed_ticks,
                "bastion: renderer certification simulation frozen"
            );
        }
        if (certification_freeze_tick.is_some() || continuous_streaming_measurement)
            && let Err(error) =
                crate::render::bastion_r0d::record_certification_server_tick_for_runtime_v1(
                    completed_ticks,
                    certification_freeze_tick.is_some(),
                )
        {
            error!(
                ?error,
                completed_ticks, "bastion: renderer certification server latch rejected tick"
            );
            break;
        }
    }
}
