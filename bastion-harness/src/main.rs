//! Project Bastion — B0 headless simulation harness.
//!
//! Boots the real Veloren simulation stack (`world` + `rtsim` + `server`) with
//! no voxygen, no GPU, and no network clients, ticks it a fixed number of
//! times faster than real-time, and dumps aggregate state as JSON on stdout
//! (logs go to stderr). `--verify` runs the same seed twice in isolated child
//! processes and diffs the aggregates, reporting `DETERMINISM: OK`/`DIVERGED`.
//!
//! This is infrastructure, not gameplay: it reuses the exact server
//! construction path of `server-cli`/singleplayer (see
//! `docs/BASTION_B0_FINDINGS.md`) and asserts nothing by itself. Later blocks
//! hang their Tier-1 assertions off the `Summary` it produces.

use clap::Parser;
use common::resources::Time;
use serde::{Deserialize, Serialize};
use server::{
    CalendarMode, EditableSettings, Input, Server, Settings,
    persistence::{DatabaseSettings, SqlLogMode},
};
use specs::{Join, WorldExt};
use std::{
    path::PathBuf,
    process::{Command, ExitCode},
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::info;

#[derive(Parser)]
#[command(name = "bastion-harness", about)]
struct Args {
    /// World seed (`server::Settings::world_seed`); also seeds rtsim data
    /// generation.
    #[arg(long, default_value_t = 1337)]
    seed: u32,

    /// Number of server ticks to run. One server tick == one rtsim tick.
    #[arg(long, default_value_t = 1000)]
    ticks: u64,

    /// Fixed simulated ticks-per-second used to derive dt (the harness never
    /// sleeps; wall-clock speed is limited only by CPU).
    #[arg(long, default_value_t = 30.0)]
    tps: f64,

    /// Run the same seed twice in two isolated child processes, diff the
    /// aggregate dumps, and report DETERMINISM: OK/DIVERGED (exit 0/1).
    #[arg(long)]
    verify: bool,

    /// Server data directory. Defaults to a fresh temp dir per run, which is
    /// required for reproducibility: rtsim *loads* `<data_dir>/rtsim/data.dat`
    /// if it exists instead of generating from the seed.
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// bastion (B3): spawn a starting colony of N colonists near the first
    /// site after boot, tick as usual, then dump the roster as a second JSON
    /// line on stdout (after the Summary line).
    #[arg(long, default_value_t = 0)]
    colony: u8,

    /// bastion (B4): run the job-board acceptance scenario (force-load an
    /// area, spawn 5 colonists, 20 mine designations + 1 unreachable,
    /// arbitration/travel/priority/cancel assertions + a zero-input soak).
    /// Prints one JSON result line; exit code reflects pass/fail.
    #[arg(long)]
    b4_scenario: bool,
}

/// Aggregate state dump. Deliberately coarse: aggregates are far more stable
/// than exact positions/trajectories, and rtsim's per-tick rules currently
/// seed RNG from OS entropy (see BASTION_B0_FINDINGS.md §4), so exact state is
/// not expected to reproduce. Every field here must be a pure function of the
/// simulation (no wall-clock, no absolute paths) so `--verify` can compare
/// dumps byte-for-byte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Summary {
    seed: u32,
    tick_count: u64,
    /// rtsim's own tick counter (`rtsim::data::Data::tick`).
    rtsim_tick: u64,
    rtsim_npc_count: usize,
    rtsim_site_count: usize,
    rtsim_faction_count: usize,
    rtsim_report_count: usize,
    /// Entities in the loaded server ECS. With no clients connected no chunks
    /// load, so this stays near zero — it exists to catch entity leaks across
    /// the loaded<->simulated boundary in later blocks.
    loaded_entity_count: usize,
    /// `common::resources::Time` (sim seconds since server start).
    sim_time: f64,
    /// `common::resources::TimeOfDay` (game-world seconds).
    time_of_day: f64,
    /// bastion (B3): rtsim NPCs carrying a colonist record. 0 unless
    /// `--colony` was passed (children in `--verify` never pass it).
    colonist_count: usize,
}

fn main() -> ExitCode {
    let args = Args::parse();

    // Logs to stderr so stdout carries exactly one line of JSON.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    if args.b4_scenario {
        b4_scenario(&args)
    } else if args.verify {
        verify(&args)
    } else {
        let (summary, roster) = run_once(&args);
        println!(
            "{}",
            serde_json::to_string(&summary).expect("Summary is always serializable")
        );
        if let Some(roster) = roster {
            // bastion (B3): the colony roster as a second stdout line.
            println!(
                "{}",
                serde_json::to_string(&roster).expect("roster is always serializable")
            );
        }
        ExitCode::SUCCESS
    }
}

/// Boot world + rtsim + server headlessly, tick, and summarize.
fn run_once(args: &Args) -> (Summary, Option<Vec<common::bastion::BastionColonist>>) {
    let started = Instant::now();

    let (data_dir, ephemeral) = match &args.data_dir {
        Some(dir) => (dir.clone(), false),
        None => {
            let dir = std::env::temp_dir().join(format!(
                "bastion-harness-{}-{}",
                std::process::id(),
                started.elapsed().as_nanos() // distinct per call within a process
            ));
            (dir, true)
        },
    };
    std::fs::create_dir_all(&data_dir).expect("failed to create harness data dir");
    info!(?data_dir, "harness data dir");

    // The minimal headless settings recipe (verified against
    // `Settings::singleplayer` and `server-cli`): no sockets, no auth, no UDP
    // query server, no wall-clock calendar. `map_file: None` loads the default
    // pre-generated map asset, so boot cost is site/civ generation only.
    let settings = Settings {
        gameserver_protocols: Vec::new(),
        auth_server_address: None,
        query_address: None,
        world_seed: args.seed,
        server_name: "bastion-harness".into(),
        map_file: None,
        max_view_distance: None,
        calendar_mode: CalendarMode::None,
        ..Settings::default()
    };
    let editable_settings = EditableSettings::singleplayer(&data_dir);
    let database_settings = DatabaseSettings {
        db_dir: data_dir.join("saves"),
        sql_log_mode: SqlLogMode::Disabled,
    };

    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("bastion-harness-tokio")
            .build()
            .expect("failed to build tokio runtime"),
    );

    let mut server = Server::new(
        settings,
        editable_settings,
        database_settings,
        &data_dir,
        &|stage| info!(?stage, "server init"),
        runtime,
    )
    .expect("failed to create headless server");
    info!(
        elapsed = ?started.elapsed(),
        "server (world + rtsim) booted headlessly"
    );

    // bastion (B3): spawn the starting colony before ticking, near the first
    // site (position is nominal headlessly — no chunks load without clients).
    if args.colony > 0 {
        let spawn_pos = {
            let ecs = server.state().ecs();
            let rtsim = ecs.read_resource::<server::rtsim::RtSim>();
            let data = rtsim.state().data();
            data.sites
                .sites
                .values()
                .next()
                .map(|s| vek::Vec3::new(s.wpos.x as f32, s.wpos.y as f32, 300.0))
                .unwrap_or_else(|| vek::Vec3::new(16384.0, 16384.0, 300.0))
        };
        let names = server.bastion_spawn_colony(spawn_pos, args.colony);
        info!(?names, "harness spawned starting colony");
    }

    // Tick as fast as the CPU allows: fixed dt, no frame pacing, no sleeping.
    let dt = Duration::from_secs_f64(1.0 / args.tps);
    let tick_started = Instant::now();
    for i in 0..args.ticks {
        server
            .tick(Input::default(), dt)
            .expect("server tick failed");
        server.cleanup();
        if (i + 1) % 250 == 0 {
            info!(tick = i + 1, elapsed = ?tick_started.elapsed(), "ticking");
        }
    }
    let ticked = tick_started.elapsed();
    info!(
        ticks = args.ticks,
        elapsed = ?ticked,
        sim_seconds = args.ticks as f64 * dt.as_secs_f64(),
        speedup = (args.ticks as f64 * dt.as_secs_f64()) / ticked.as_secs_f64().max(f64::EPSILON),
        "tick loop done"
    );

    let summary = {
        let ecs = server.state().ecs();
        let rtsim = ecs.read_resource::<server::rtsim::RtSim>();
        let data = rtsim.state().data();
        Summary {
            seed: args.seed,
            tick_count: args.ticks,
            rtsim_tick: data.tick,
            rtsim_npc_count: data.npcs.npcs.len(),
            rtsim_site_count: data.sites.sites.len(),
            rtsim_faction_count: data.factions.factions.len(),
            rtsim_report_count: data.reports.reports.len(),
            loaded_entity_count: ecs.entities().join().count(),
            sim_time: ecs.read_resource::<Time>().0,
            time_of_day: data.time_of_day.0,
            colonist_count: data
                .npcs
                .npcs
                .values()
                .filter(|n| n.bastion_colonist.is_some())
                .count(),
        }
    };

    let roster = (args.colony > 0).then(|| server.bastion_colony_roster());

    drop(server);

    if ephemeral {
        // Best-effort: rtsim's save thread may still be flushing on Windows.
        for attempt in 0..3 {
            match std::fs::remove_dir_all(&data_dir) {
                Ok(()) => break,
                Err(e) if attempt == 2 => {
                    info!(?data_dir, ?e, "could not remove temp data dir (harmless)")
                },
                Err(_) => std::thread::sleep(Duration::from_millis(500)),
            }
        }
    }

    (summary, roster)
}

/// bastion (B4): the job-board acceptance scenario (design doc §B4 Done-when).
fn b4_scenario(args: &Args) -> ExitCode {
    use common::{
        bastion::{DesignationKind, Region, WorkType},
        vol::ReadVol,
    };
    use vek::{Vec2, Vec3};

    let started = Instant::now();
    let data_dir = std::env::temp_dir().join(format!(
        "bastion-b4-{}-{}",
        std::process::id(),
        started.elapsed().as_nanos()
    ));
    std::fs::create_dir_all(&data_dir).expect("failed to create harness data dir");
    let settings = Settings {
        gameserver_protocols: Vec::new(),
        auth_server_address: None,
        query_address: None,
        world_seed: args.seed,
        server_name: "bastion-harness-b4".into(),
        map_file: None,
        max_view_distance: None,
        calendar_mode: CalendarMode::None,
        ..Settings::default()
    };
    let editable_settings = EditableSettings::singleplayer(&data_dir);
    let database_settings = DatabaseSettings {
        db_dir: data_dir.join("saves"),
        sql_log_mode: SqlLogMode::Disabled,
    };
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("bastion-harness-tokio")
            .build()
            .expect("failed to build tokio runtime"),
    );
    let mut server = Server::new(
        settings,
        editable_settings,
        database_settings,
        &data_dir,
        &|stage| info!(?stage, "server init"),
        runtime,
    )
    .expect("failed to create headless server");
    info!(elapsed = ?started.elapsed(), "b4: server booted");

    let dt = Duration::from_secs_f64(1.0 / args.tps);
    let tick = |server: &mut Server, n: u64| {
        for _ in 0..n {
            server.tick(Input::default(), dt).expect("server tick failed");
            server.cleanup();
        }
    };

    // 1. Pick a flat-ish anchor: the first site's position.
    let site_wpos: Vec2<f32> = {
        let ecs = server.state().ecs();
        let rtsim = ecs.read_resource::<server::rtsim::RtSim>();
        let data = rtsim.state().data();
        data.sites
            .sites
            .values()
            .next()
            .map(|s| s.wpos.map(|e| e as f32))
            .unwrap_or_else(|| Vec2::new(16384.0, 16384.0))
    };

    // 2. Force-load the area (also pins it against the unload sweep).
    let loaded = server.bastion_force_load_area(site_wpos, 5);
    info!(loaded, "b4: force-loaded area");

    // Ground scan helper: highest filled block z at a column.
    let ground_z = |server: &Server, x: i32, y: i32| -> Option<i32> {
        let terrain = server.state().terrain();
        // NOTE: world altitudes commonly exceed 1000 blocks.
        (0..2048).rev().find(|z| {
            terrain
                .get(Vec3::new(x, y, *z))
                .is_ok_and(|b| b.is_filled())
        })
    };
    let cx = site_wpos.x as i32;
    let cy = site_wpos.y as i32;
    let cz = ground_z(&server, cx, cy).expect("no ground at site center");

    // 3. Spawn the band on the surface.
    let names = server.bastion_spawn_colony(
        Vec3::new(site_wpos.x, site_wpos.y, cz as f32 + 2.0),
        5,
    );
    tick(&mut server, 60);
    let states = server.bastion_colonist_states();
    let colonists_loaded = states.len();

    // 4. Priority test target: first colonist never mines.
    let disabled = names.first().cloned().unwrap_or_default();
    server.bastion_set_work_priority(&disabled, WorkType::Mine, 0);

    // 5. One deep unreachable job FIRST, directly under an *enabled*
    // colonist's current position (they wander during the settle ticks) so
    // it is guaranteed nearest → claimed → travel-watchdogged unreachable.
    // Then 20 reachable mine designations in a ring.
    let deep = {
        let states = server.bastion_colonist_states();
        let anchor = states
            .iter()
            .find(|(n, _, _)| *n != disabled)
            .map(|(_, p, _)| *p)
            .unwrap_or(Vec3::new(site_wpos.x, site_wpos.y, cz as f32));
        Vec3::new(anchor.x as i32, anchor.y as i32, anchor.z as i32 - 8)
    };
    let deep_jobs = server
        .bastion_place_designation(Region { min: deep, max: deep }, DesignationKind::Mine)
        .len();
    let mut placed = 0;
    for i in 0..20 {
        let ang = std::f64::consts::TAU * i as f64 / 20.0;
        let r = 14.0 + (i % 4) as f64 * 3.0;
        let x = cx + (r * ang.cos()) as i32;
        let y = cy + (r * ang.sin()) as i32;
        if let Some(z) = ground_z(&server, x, y) {
            let b = Vec3::new(x, y, z);
            placed += server
                .bastion_place_designation(Region { min: b, max: b }, DesignationKind::Mine)
                .len();
        }
    }
    info!(placed, deep_jobs, "b4: designations placed");

    // 6. Run: up to 60s sim; sample invariants; early-exit when 4 arrived.
    let mut claims_always_distinct = true;
    let mut disabled_never_claimed = true;
    let mut arrived = 0;
    for _ in 0..60 {
        tick(&mut server, 30);
        let audit = server.bastion_job_audit();
        claims_always_distinct &= audit.claims_distinct;
        let states = server.bastion_colonist_states();
        if states
            .iter()
            .any(|(n, _, j)| *n == disabled && j.is_some())
        {
            disabled_never_claimed = false;
        }
        arrived = states
            .iter()
            .filter(|(n, _, j)| *n != disabled && matches!(j, Some((_, true))))
            .count();
        if arrived >= 4 {
            break;
        }
    }
    let audit_mid = server.bastion_job_audit();

    // 7. Cancel everything left; claims must release within one arb cycle.
    server.bastion_cancel_designation(Region {
        min: Vec3::new(cx - 64, cy - 64, cz - 64),
        max: Vec3::new(cx + 64, cy + 64, cz + 64),
    });
    tick(&mut server, 30);
    let audit_after_cancel = server.bastion_job_audit();
    let states_after_cancel = server.bastion_colonist_states();
    let all_idle_after_cancel = states_after_cancel.iter().all(|(_, _, j)| j.is_none());

    // 8. Zero-input soak: keep ticking, no panics, bounded tick time.
    let soak_ticks: u64 = 600;
    let soak_started = Instant::now();
    tick(&mut server, soak_ticks);
    let soak_elapsed = soak_started.elapsed();
    let avg_tick_ms = soak_elapsed.as_secs_f64() * 1000.0 / soak_ticks as f64;

    let result = serde_json::json!({
        "b4_colonists_loaded": colonists_loaded,
        "b4_jobs_placed": placed,
        "b4_claims_always_distinct": claims_always_distinct,
        "b4_arrived_enabled": arrived,
        "b4_priority_honored": disabled_never_claimed,
        "b4_unreachable_marked": audit_mid.unreachable >= 1,
        "b4_cancel_cleared_jobs": audit_after_cancel.total == 0,
        "b4_all_idle_after_cancel": all_idle_after_cancel,
        "b4_soak_avg_tick_ms": avg_tick_ms,
    });
    let pass = colonists_loaded == 5
        && placed >= 18
        && claims_always_distinct
        && arrived >= 4
        && disabled_never_claimed
        && audit_mid.unreachable >= 1
        && audit_after_cancel.total == 0
        && all_idle_after_cancel
        && avg_tick_ms < 100.0;
    println!("{}", result);
    println!("B4 SCENARIO: {}", if pass { "PASS" } else { "FAIL" });

    drop(server);
    let _ = std::fs::remove_dir_all(&data_dir);
    if pass { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// Run the same configuration twice in isolated child processes and diff the
/// aggregate dumps. Child processes (rather than two in-process runs)
/// guarantee zero shared state: fresh asset caches, fresh globals, fresh
/// in-process network registry.
fn verify(args: &Args) -> ExitCode {
    let exe = std::env::current_exe().expect("failed to locate own executable");

    let mut dumps = Vec::new();
    for run in 1..=2 {
        info!(run, "starting verification run");
        let output = Command::new(&exe)
            .args([
                "--seed",
                &args.seed.to_string(),
                "--ticks",
                &args.ticks.to_string(),
                "--tps",
                &args.tps.to_string(),
            ])
            .stderr(std::process::Stdio::inherit())
            .output()
            .expect("failed to spawn child harness process");
        if !output.status.success() {
            eprintln!("child run {run} failed with {}", output.status);
            return ExitCode::from(2);
        }
        let stdout = String::from_utf8(output.stdout).expect("child stdout was not UTF-8");
        let summary: Summary = serde_json::from_str(stdout.trim())
            .expect("child stdout was not a valid Summary JSON line");
        dumps.push(summary);
    }

    let (a, b) = (&dumps[0], &dumps[1]);
    println!("run 1: {}", serde_json::to_string(a).unwrap());
    println!("run 2: {}", serde_json::to_string(b).unwrap());
    if a == b {
        println!("DETERMINISM: OK");
        ExitCode::SUCCESS
    } else {
        // Field-by-field diff via the JSON representation.
        let (ja, jb) = (
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap(),
        );
        for (key, va) in ja.as_object().unwrap() {
            let vb = &jb[key];
            if va != vb {
                println!("  {key}: run1={va} run2={vb}");
            }
        }
        println!("DETERMINISM: DIVERGED");
        ExitCode::FAILURE
    }
}
