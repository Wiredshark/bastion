use super::Args;
use common::{
    comp::{self, CharacterState, PhysicsState, Pos, Vel},
    resources::{Time, TimeOfDay},
    terrain::{Block, BlockKind},
};
use serde::Serialize;
use serde_json::Value;
use server::{
    CalendarMode, EditableSettings, Input, Server, Settings, SpawnPoint,
    bastion_boot_cache::{self, Origin, Status},
    persistence::{DatabaseSettings, SqlLogMode},
};
use sha2::Digest;
use specs::{Join, WorldExt};
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::Path,
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use vek::Rgb;

static RUN_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ColonistSample {
    uid: u64,
    name: String,
    pos: [f32; 3],
    vel: [f32; 3],
    character_state: String,
    on_ground: bool,
    on_wall: Option<[f32; 3]>,
    active_job: Option<(u64, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct TrajectorySample {
    tick: u64,
    sim_time: f64,
    time_of_day: f64,
    rtsim_tick: u64,
    colonists: Vec<ColonistSample>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct Outcome {
    seed: u32,
    ticks: u64,
    spawn_point: [f32; 3],
    rtsim_tick: u64,
    rtsim_npcs: usize,
    rtsim_sites: usize,
    rtsim_data_sha256: String,
    loaded_terrain_sha256: String,
    loaded_entities: usize,
    loaded_colonists: usize,
    final_sample: TrajectorySample,
}

#[derive(Serialize)]
struct LegMetadata {
    status: Status,
    boot_wall_millis: u128,
    force_load_wall_millis: u128,
    tape_samples: usize,
    cached_chunk_copy_on_write_clean: bool,
}

#[derive(Serialize)]
struct PairVerdict {
    name: String,
    seed: u32,
    deterministic: bool,
    fresh_origin_valid: bool,
    restored_origin_valid: bool,
    outcome_equal: bool,
    trajectory_equal: bool,
    nonempty_trajectory: bool,
    first_divergence: Option<FirstDivergence>,
    fresh: LegMetadata,
    restored: LegMetadata,
}

#[derive(Serialize)]
struct FirstDivergence {
    source: &'static str,
    record: usize,
    path: String,
    fresh: Value,
    restored: Value,
}

#[derive(Serialize)]
struct Verdict {
    schema: &'static str,
    executable_sha256: String,
    source_head: &'static str,
    target_arch: &'static str,
    target_os: &'static str,
    normalized_fields: [&'static str; 1],
    pairs: Vec<PairVerdict>,
    deterministic: bool,
    gate_pass: bool,
}

struct Leg {
    status: Status,
    boot_wall: Duration,
    force_load_wall: Duration,
    outcome: Outcome,
    tape_bytes: Vec<u8>,
    sample_count: usize,
    cached_chunk_copy_on_write_clean: bool,
    rtsim_data_ron: Vec<u8>,
}

fn sample(server: &Server, tick: u64) -> TrajectorySample {
    let ecs = server.state().ecs();
    let entities = ecs.entities();
    let uids = ecs.read_storage::<common::uid::Uid>();
    let colonists = ecs.read_storage::<comp::Colonist>();
    let positions = ecs.read_storage::<Pos>();
    let velocities = ecs.read_storage::<Vel>();
    let states = ecs.read_storage::<CharacterState>();
    let physics = ecs.read_storage::<PhysicsState>();
    let jobs = ecs.read_storage::<comp::bastion::ActiveJob>();
    let mut observed = (
        &entities,
        &uids,
        &colonists,
        &positions,
        &velocities,
        &states,
        &physics,
    )
        .join()
        .map(
            |(entity, uid, colonist, pos, vel, state, physics)| ColonistSample {
                uid: uid.0.get(),
                name: colonist.0.name.clone(),
                pos: [pos.0.x, pos.0.y, pos.0.z],
                vel: [vel.0.x, vel.0.y, vel.0.z],
                character_state: format!("{state:?}"),
                on_ground: physics.on_ground.is_some(),
                on_wall: physics.on_wall.map(|normal| [normal.x, normal.y, normal.z]),
                active_job: jobs
                    .get(entity)
                    .map(|job| (job.job, format!("{:?}", job.state))),
            },
        )
        .collect::<Vec<_>>();
    observed.sort_by_key(|colonist| colonist.uid);
    let rtsim = ecs.read_resource::<server::rtsim::RtSim>();
    TrajectorySample {
        tick,
        sim_time: ecs.read_resource::<Time>().0,
        time_of_day: ecs.read_resource::<TimeOfDay>().0,
        rtsim_tick: rtsim.state().data().tick,
        colonists: observed,
    }
}

fn append_jsonl(bytes: &mut Vec<u8>, value: &impl Serialize) {
    serde_json::to_writer(&mut *bytes, value).expect("trajectory sample is serializable");
    bytes.push(b'\n');
}

fn run_leg(args: &Args, seed: u32) -> Leg {
    let id = RUN_ID.fetch_add(1, Ordering::Relaxed);
    let data_dir = std::env::temp_dir().join(format!(
        "bastion-boot-cache-{}-{seed}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&data_dir).expect("create boot-cache leg data directory");
    let settings = Settings {
        gameserver_protocols: Vec::new(),
        auth_server_address: None,
        query_address: None,
        world_seed: seed,
        server_name: "bastion-boot-cache-proof".into(),
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
            .thread_name("bastion-boot-cache-proof")
            .build()
            .expect("build boot-cache proof runtime"),
    );
    let boot_started = Instant::now();
    let mut server = Server::new(
        settings,
        editable_settings,
        database_settings,
        &data_dir,
        &|_| {},
        runtime,
    )
    .expect("boot cache proof server construction");
    let boot_wall = boot_started.elapsed();
    let status = server.bastion_boot_cache_status().clone();

    let spawn = server.state().ecs().read_resource::<SpawnPoint>().0;
    let force_started = Instant::now();
    let loaded_chunks = server.bastion_force_load_area(spawn.xy(), 1);
    let force_load_wall = force_started.elapsed();
    assert_eq!(
        loaded_chunks, 9,
        "boot-cache proof must load its 3x3 premise"
    );
    server.bastion_spawn_colony(spawn, 3);

    let mut tape_bytes = Vec::new();
    append_jsonl(&mut tape_bytes, &sample(&server, 0));
    let dt = Duration::from_secs_f64(1.0 / args.tps);
    for tick in 1..=args.boot_cache_proof_ticks {
        server
            .tick(Input::default(), dt)
            .expect("boot-cache proof server tick");
        server.cleanup();
        append_jsonl(&mut tape_bytes, &sample(&server, tick));
    }
    let final_sample = sample(&server, args.boot_cache_proof_ticks);
    let (outcome, rtsim_data_ron) = {
        let ecs = server.state().ecs();
        let rtsim = ecs.read_resource::<server::rtsim::RtSim>();
        let data = rtsim.state().data();
        let rtsim_data_ron =
            bastion_boot_cache::rtsim_data_ron(&data).expect("serialize proof RTSim data");
        (
            Outcome {
                seed,
                ticks: args.boot_cache_proof_ticks,
                spawn_point: [spawn.x, spawn.y, spawn.z],
                rtsim_tick: data.tick,
                rtsim_npcs: data.npcs.npcs.len(),
                rtsim_sites: data.sites.sites.len(),
                rtsim_data_sha256: hex::encode(sha2::Sha256::digest(&rtsim_data_ron)),
                loaded_terrain_sha256: bastion_boot_cache::terrain_grid_sha256(
                    &ecs.read_resource::<common::terrain::TerrainGrid>(),
                ),
                loaded_entities: ecs.entities().join().count(),
                loaded_colonists: final_sample.colonists.len(),
                final_sample,
            },
            rtsim_data_ron,
        )
    };
    // Mutation happens only after all compared observations are frozen. Its
    // sole purpose is to prove that TerrainGrid's Arc::make_mut path cannot
    // alter the pristine chunk retained by the boot template.
    let cached_chunk_before = bastion_boot_cache::cached_chunks_sha256();
    let mutation_pos = spawn.map(|value| value.floor() as i32);
    let old_block = server.state().get_block(mutation_pos);
    let replacement = if old_block.is_some_and(|block| block.is_solid()) {
        Block::empty()
    } else {
        Block::new(BlockKind::Rock, Rgb::new(120, 120, 120))
    };
    server.state_mut().set_block(mutation_pos, replacement);
    let cached_chunk_after = bastion_boot_cache::cached_chunks_sha256();
    let cached_chunk_copy_on_write_clean =
        cached_chunk_before.is_some() && cached_chunk_before == cached_chunk_after;
    let sample_count = tape_bytes.iter().filter(|byte| **byte == b'\n').count();
    drop(server);
    remove_dir_all_retry(&data_dir);
    Leg {
        status,
        boot_wall,
        force_load_wall,
        outcome,
        tape_bytes,
        sample_count,
        cached_chunk_copy_on_write_clean,
        rtsim_data_ron,
    }
}

fn remove_dir_all_retry(path: &Path) {
    for attempt in 0..5 {
        match fs::remove_dir_all(path) {
            Ok(()) => return,
            Err(error) if attempt == 4 => {
                eprintln!(
                    "boot-cache proof cleanup warning for {}: {error}",
                    path.display()
                )
            },
            Err(_) => std::thread::sleep(Duration::from_millis(250)),
        }
    }
}

fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    writer.write_all(bytes)?;
    writer.flush()
}

fn first_json_diff(path: &str, fresh: &Value, restored: &Value) -> Option<(String, Value, Value)> {
    match (fresh, restored) {
        (Value::Object(fresh), Value::Object(restored)) => {
            let mut keys = fresh.keys().chain(restored.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
                let child_path = format!("{path}.{key}");
                match (fresh.get(key), restored.get(key)) {
                    (Some(fresh), Some(restored)) => {
                        if let Some(diff) = first_json_diff(&child_path, fresh, restored) {
                            return Some(diff);
                        }
                    },
                    (fresh, restored) => {
                        return Some((
                            child_path,
                            fresh.cloned().unwrap_or(Value::Null),
                            restored.cloned().unwrap_or(Value::Null),
                        ));
                    },
                }
            }
            None
        },
        (Value::Array(fresh), Value::Array(restored)) => {
            let len = fresh.len().max(restored.len());
            for index in 0..len {
                let child_path = format!("{path}[{index}]");
                match (fresh.get(index), restored.get(index)) {
                    (Some(fresh), Some(restored)) => {
                        if let Some(diff) = first_json_diff(&child_path, fresh, restored) {
                            return Some(diff);
                        }
                    },
                    (fresh, restored) => {
                        return Some((
                            child_path,
                            fresh.cloned().unwrap_or(Value::Null),
                            restored.cloned().unwrap_or(Value::Null),
                        ));
                    },
                }
            }
            None
        },
        _ if fresh == restored => None,
        _ => Some((path.to_owned(), fresh.clone(), restored.clone())),
    }
}

fn first_divergence(fresh: &Leg, restored: &Leg) -> Option<FirstDivergence> {
    for (record, (fresh_line, restored_line)) in fresh
        .tape_bytes
        .split(|byte| *byte == b'\n')
        .zip(restored.tape_bytes.split(|byte| *byte == b'\n'))
        .enumerate()
    {
        if fresh_line == restored_line {
            continue;
        }
        let fresh_value = serde_json::from_slice(fresh_line).unwrap_or(Value::Null);
        let restored_value = serde_json::from_slice(restored_line).unwrap_or(Value::Null);
        let (path, fresh_value, restored_value) = first_json_diff(
            "$",
            &fresh_value,
            &restored_value,
        )
        .unwrap_or(("$".to_owned(), fresh_value, restored_value));
        return Some(FirstDivergence {
            source: "trajectory",
            record,
            path,
            fresh: fresh_value,
            restored: restored_value,
        });
    }
    if fresh.tape_bytes.len() != restored.tape_bytes.len() {
        return Some(FirstDivergence {
            source: "trajectory",
            record: fresh.sample_count.min(restored.sample_count),
            path: "$.record_count".to_owned(),
            fresh: Value::from(fresh.sample_count as u64),
            restored: Value::from(restored.sample_count as u64),
        });
    }
    let fresh_outcome = serde_json::to_value(&fresh.outcome).unwrap();
    let restored_outcome = serde_json::to_value(&restored.outcome).unwrap();
    let record = fresh.sample_count.saturating_sub(1);
    first_json_diff("$", &fresh_outcome, &restored_outcome).map(|(path, fresh, restored)| {
        FirstDivergence {
            source: "outcome",
            record,
            path,
            fresh,
            restored,
        }
    })
}

fn run_pair(args: &Args, root: &Path, name: String, seed: u32) -> PairVerdict {
    bastion_boot_cache::clear();
    let fresh = run_leg(args, seed);
    let restored = run_leg(args, seed);
    let pair_dir = root.join(&name);
    fs::create_dir_all(&pair_dir).expect("create boot-cache pair evidence directory");
    write_bytes(&pair_dir.join("fresh-trajectory.jsonl"), &fresh.tape_bytes)
        .expect("write fresh trajectory");
    write_bytes(
        &pair_dir.join("restored-trajectory.jsonl"),
        &restored.tape_bytes,
    )
    .expect("write restored trajectory");
    write_bytes(
        &pair_dir.join("fresh-outcome.json"),
        &serde_json::to_vec_pretty(&fresh.outcome).unwrap(),
    )
    .expect("write fresh outcome");
    write_bytes(
        &pair_dir.join("restored-outcome.json"),
        &serde_json::to_vec_pretty(&restored.outcome).unwrap(),
    )
    .expect("write restored outcome");
    write_bytes(&pair_dir.join("fresh-rtsim.ron"), &fresh.rtsim_data_ron)
        .expect("write fresh RTSim state");
    write_bytes(
        &pair_dir.join("restored-rtsim.ron"),
        &restored.rtsim_data_ron,
    )
    .expect("write restored RTSim state");

    let fresh_origin_valid = fresh.status.origin == Origin::Fresh;
    let restored_origin_valid = restored.status.origin == Origin::Restored;
    let outcome_equal = fresh.outcome == restored.outcome;
    let trajectory_equal = fresh.tape_bytes == restored.tape_bytes;
    let nonempty_trajectory = fresh.sample_count == (args.boot_cache_proof_ticks + 1) as usize
        && restored.sample_count == fresh.sample_count
        && fresh.outcome.loaded_colonists > 0
        && restored.outcome.loaded_colonists > 0;
    let cached_chunk_copy_on_write_clean =
        fresh.cached_chunk_copy_on_write_clean && restored.cached_chunk_copy_on_write_clean;
    let first_divergence = first_divergence(&fresh, &restored);
    PairVerdict {
        name,
        seed,
        deterministic: fresh_origin_valid
            && restored_origin_valid
            && outcome_equal
            && trajectory_equal
            && nonempty_trajectory
            && cached_chunk_copy_on_write_clean,
        fresh_origin_valid,
        restored_origin_valid,
        outcome_equal,
        trajectory_equal,
        nonempty_trajectory,
        first_divergence,
        fresh: LegMetadata {
            status: fresh.status,
            boot_wall_millis: fresh.boot_wall.as_millis(),
            force_load_wall_millis: fresh.force_load_wall.as_millis(),
            tape_samples: fresh.sample_count,
            cached_chunk_copy_on_write_clean: fresh.cached_chunk_copy_on_write_clean,
        },
        restored: LegMetadata {
            status: restored.status,
            boot_wall_millis: restored.boot_wall.as_millis(),
            force_load_wall_millis: restored.force_load_wall.as_millis(),
            tape_samples: restored.sample_count,
            cached_chunk_copy_on_write_clean: restored.cached_chunk_copy_on_write_clean,
        },
    }
}

pub(super) fn run(args: &Args, output_dir: &Path) -> ExitCode {
    if output_dir.exists() {
        eprintln!(
            "BOOT CACHE PROOF: refusing to overwrite existing output {}",
            output_dir.display()
        );
        return ExitCode::from(2);
    }
    if args.boot_cache_proof_seeds.is_empty() {
        eprintln!("BOOT CACHE PROOF: at least one seed is required");
        return ExitCode::from(2);
    }
    fs::create_dir_all(output_dir).expect("create boot-cache proof output");
    let exe = std::env::current_exe().expect("locate boot-cache proof executable");
    let executable_sha256 =
        bastion_boot_cache::executable_sha256(&exe).expect("hash boot-cache proof executable");
    bastion_boot_cache::enable(executable_sha256.clone()).expect("enable exact-key boot cache");

    let mut pairs = Vec::new();
    let primary = args.boot_cache_proof_seeds[0];
    pairs.push(run_pair(
        args,
        output_dir,
        format!("seed-{primary}-x1"),
        primary,
    ));
    pairs.push(run_pair(
        args,
        output_dir,
        format!("seed-{primary}-x2"),
        primary,
    ));
    for seed in args.boot_cache_proof_seeds.iter().copied().skip(1) {
        pairs.push(run_pair(
            args,
            output_dir,
            format!("seed-{seed}-corpus"),
            seed,
        ));
    }
    let deterministic = pairs.iter().all(|pair| pair.deterministic);
    let verdict = Verdict {
        schema: "bastion.boot-cache-proof/v1",
        executable_sha256,
        source_head: env!("BASTION_BUILD_SHA"),
        target_arch: std::env::consts::ARCH,
        target_os: std::env::consts::OS,
        normalized_fields: ["wall_unix_millis"],
        pairs,
        deterministic,
        gate_pass: deterministic,
    };
    write_bytes(
        &output_dir.join("verdict.json"),
        &serde_json::to_vec_pretty(&verdict).unwrap(),
    )
    .expect("write boot-cache verdict");
    println!("{}", serde_json::to_string(&verdict).unwrap());
    for pair in &verdict.pairs {
        let speedup = if pair.restored.boot_wall_millis == 0 {
            f64::INFINITY
        } else {
            pair.fresh.boot_wall_millis as f64 / pair.restored.boot_wall_millis as f64
        };
        println!(
            "BOOT CACHE PAIR {} seed={} deterministic={} fresh={}ms restored={}ms \
             speedup={speedup:.2}x",
            pair.name,
            pair.seed,
            pair.deterministic,
            pair.fresh.boot_wall_millis,
            pair.restored.boot_wall_millis,
        );
        if let Some(divergence) = &pair.first_divergence {
            println!(
                "BOOT CACHE FIRST DIVERGENCE source={} record={} path={} fresh={} restored={}",
                divergence.source,
                divergence.record,
                divergence.path,
                divergence.fresh,
                divergence.restored,
            );
        }
    }
    println!(
        "BOOT CACHE FRESH/RESTORED: {}",
        if deterministic { "PASS" } else { "FAIL" }
    );
    bastion_boot_cache::disable();
    if deterministic {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
