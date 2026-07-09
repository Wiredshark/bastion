//! bastion (B-ASSET1): `--asset-test <id|all>` — the flat-plane arena dynamic
//! tests (ASSET_DYNAMIC_TEST_SPEC tier ISOLATED-DYNAMIC) plus one
//! INTEGRATED-DYNAMIC spot check on real worldgen terrain, driven through the
//! real server + real agent pathfinding (no simulated geometry proxies).
//!
//! Cast per category:
//! - Structure / TestFixture: reachability + traversal + arrival + egress +
//!   multi-occupancy + interior function point (geometric-interior target,
//!   ≥ 3 blocks from the bounds edge so ARRIVE_DIST 2.5 can't false-arrive
//!   through a wall).
//! - Defense (the palisade wall+gate line): blocked/unblocked matrix — the
//!   closed gate must never admit the outside colonist (watchdog fires); the
//!   open variant must (poses = two marker mappings of the same vox). The
//!   other three yard sides are harness-built rock fixture walls (the asset
//!   under test is the wall+gate line; rotation of the line itself is a
//!   documented follow-up).
//! - Flora: world-scale path-around (the prop assertion at 1 vox = 1 block).
//! - Prop/Item (figure-layer, 11 vox/block — handcart/gloomcap/maul/armor):
//!   load + marker-fidelity ONLY; their world integration is a
//!   sprite/item-manifest edit, excluded by this block's
//!   vanilla-asset-tree-untouched contract. Reported PASS(load-only).
//! - Creature: SKIP (later integration rung — Body/skeleton Rust work).
//!
//! `--asset-test all` runs every non-`test_*`, non-creature catalog entry.
//! `test_*` fixtures run only when named explicitly — `test_room_door_closed`
//! is the deliberate useful-FAIL demonstration (exit code honestly nonzero).
//!
//! Results: one JSON line per asset on stdout + an append-only block in
//! `readme/ASSET_INTEGRATION_LOG.md` (created on first run with the format
//! contract header the asset session reads back).

use serde::Serialize;
use specs::WorldExt;
use server::{
    CalendarMode, EditableSettings, Input, Server, Settings,
    bastion_assets::{self, AssetCategory, AssetLabCatalog, LoadedAsset, PlacementReport},
    persistence::{DatabaseSettings, SqlLogMode},
};
use std::{
    io::Write as _,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::info;
use vek::{Aabb, Vec2, Vec3};

/// Sim-seconds budget for one leg of travel (spec: arrival ≤ 30 s sim).
const ARRIVAL_BUDGET_S: f32 = 30.0;
/// Extra allowance for the 3-colonist multi-occupancy legs (shoving).
const MULTI_BUDGET_S: f32 = 45.0;

pub struct AssetTestConfig {
    pub seed: u32,
    pub tps: f64,
    /// Asset id or "all".
    pub target: String,
    pub asset_lab_dir: PathBuf,
}

#[derive(Serialize, Clone)]
struct Assertion {
    name: String,
    pass: bool,
    detail: String,
}

#[derive(Serialize)]
struct AssetResult {
    id: String,
    category: String,
    mode: String,
    fidelity_ok: bool,
    marker_checks: Vec<String>,
    blocks_placed: usize,
    sprite_cfgs_dropped: usize,
    entity_spawners_skipped: usize,
    assertions: Vec<Assertion>,
    pass: bool,
}

enum GotoOutcome {
    Arrived { elapsed: f32 },
    Stuck { elapsed: f32, best: f32 },
    Timeout { elapsed: f32, dist: f32 },
    Refused,
    Lost,
}

impl GotoOutcome {
    fn arrived(&self) -> bool { matches!(self, GotoOutcome::Arrived { .. }) }

    fn describe(&self) -> String {
        match self {
            GotoOutcome::Arrived { elapsed } => format!("arrived in {elapsed:.1}s"),
            GotoOutcome::Stuck { elapsed, best } => {
                format!("STUCK (watchdog) after {elapsed:.1}s, best dist {best:.1}")
            },
            GotoOutcome::Timeout { elapsed, dist } => {
                format!("TIMEOUT after {elapsed:.1}s, dist {dist:.1}")
            },
            GotoOutcome::Refused => "goto refused (colonist not loaded or has a job)".into(),
            GotoOutcome::Lost => "goto state lost (colonist demoted mid-order?)".into(),
        }
    }
}

pub fn run(cfg: &AssetTestConfig) -> std::process::ExitCode {
    let started = Instant::now();

    // ── Catalog + cast ───────────────────────────────────────────────────
    let catalog = AssetLabCatalog::scan(&cfg.asset_lab_dir);
    if catalog.entries.is_empty() {
        eprintln!(
            "ASSET-TEST: no assets found under {} — is --asset-lab-dir right?",
            cfg.asset_lab_dir.display()
        );
        return std::process::ExitCode::FAILURE;
    }
    let cast: Vec<_> = if cfg.target == "all" {
        catalog
            .entries
            .iter()
            .filter(|e| {
                !matches!(
                    e.category,
                    AssetCategory::Creature | AssetCategory::TestFixture
                )
            })
            .cloned()
            .collect()
    } else {
        match catalog.get(&cfg.target) {
            Some(e) => vec![e.clone()],
            None => {
                eprintln!(
                    "ASSET-TEST: '{}' not in catalog ({} entries under {})",
                    cfg.target,
                    catalog.entries.len(),
                    cfg.asset_lab_dir.display()
                );
                return std::process::ExitCode::FAILURE;
            },
        }
    };

    // ── Server boot (the b4/b5 headless recipe) ──────────────────────────
    let data_dir = std::env::temp_dir().join(format!(
        "bastion-asset-{}-{}",
        std::process::id(),
        started.elapsed().as_nanos()
    ));
    std::fs::create_dir_all(&data_dir).expect("failed to create harness data dir");
    let settings = Settings {
        gameserver_protocols: Vec::new(),
        auth_server_address: None,
        query_address: None,
        world_seed: cfg.seed,
        server_name: "bastion-harness-asset".into(),
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
    info!(elapsed = ?started.elapsed(), "asset-test: server booted");

    let dt = Duration::from_secs_f64(1.0 / cfg.tps);

    // ── Arena anchor: offset from the first site ─────────────────────────
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
    let arena_wpos = site_wpos + Vec2::new(192.0, 0.0);
    let loaded_chunks = server.bastion_force_load_area(arena_wpos, 5);
    info!(loaded_chunks, "asset-test: arena area force-loaded");

    let ax = arena_wpos.x as i32;
    let ay = arena_wpos.y as i32;
    let pad_z = ground_z(&server, ax, ay).expect("no ground at arena anchor");

    // ── Fixtures: pad + 3 colonists, verified once on the bare plane ─────
    let pad_writes = flatten_pad(&mut server, ax, ay, pad_z, 44, 28);
    info!(pad_writes, "asset-test: initial pad flatten");
    tick(&mut server, dt, 5);

    let staging = Vec3::new(ax as f32 + 0.5, (ay - 30) as f32 + 0.5, (pad_z + 2) as f32);
    let names = server.bastion_spawn_colony(staging, 3);
    tick(&mut server, dt, 60);
    let loaded_fixtures = server.bastion_colonist_states().len();
    info!(?names, loaded_fixtures, "asset-test: fixture colonists spawned");
    if loaded_fixtures < 3 {
        eprintln!("ASSET-TEST: only {loaded_fixtures}/3 fixture colonists loaded — aborting");
        return std::process::ExitCode::FAILURE;
    }

    let pad_center = Vec3::new(ax as f32 + 0.5, ay as f32 + 0.5, (pad_z + 1) as f32);
    let sanity = goto_and_wait(&mut server, dt, &names[0], pad_center, ARRIVAL_BUDGET_S);
    let sanity_back = if sanity.arrived() {
        goto_and_wait(&mut server, dt, &names[0], staging, ARRIVAL_BUDGET_S)
    } else {
        GotoOutcome::Refused
    };
    if !sanity.arrived() || !sanity_back.arrived() {
        eprintln!(
            "ASSET-TEST: bare-pad fixture sanity failed (out: {} / back: {}) — environment, not asset",
            sanity.describe(),
            sanity_back.describe()
        );
        return std::process::ExitCode::FAILURE;
    }
    info!("asset-test: bare-pad fixture sanity PASS");

    // ── Per-asset runner ─────────────────────────────────────────────────
    let mut results: Vec<AssetResult> = Vec::new();
    for entry in &cast {
        let result = run_one_asset(
            &mut server,
            dt,
            cfg,
            entry,
            &names,
            ax,
            ay,
            pad_z,
            staging,
            site_wpos,
        );
        results.push(result);
    }

    // ── Output ───────────────────────────────────────────────────────────
    let all_pass = results.iter().all(|r| r.pass);
    for r in &results {
        println!("{}", serde_json::to_string(r).expect("serializable"));
    }
    println!(
        "ASSET-TEST SUMMARY: {}/{} pass — {}",
        results.iter().filter(|r| r.pass).count(),
        results.len(),
        if all_pass { "PASS" } else { "FAIL" }
    );

    append_integration_log(&results, cfg);

    drop(server);
    let _ = std::fs::remove_dir_all(&data_dir);
    if all_pass {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn tick(server: &mut Server, dt: Duration, n: u64) {
    for _ in 0..n {
        server.tick(Input::default(), dt).expect("server tick failed");
        server.cleanup();
    }
}

/// Real-terrain-kind ground scan (the B5 canopy lesson — never `is_filled`).
fn ground_z(server: &Server, x: i32, y: i32) -> Option<i32> {
    use common::{terrain::BlockKind, vol::ReadVol};
    let terrain = server.state().terrain();
    (0..2048).rev().find(|z| {
        terrain.get(Vec3::new(x, y, *z)).is_ok_and(|b| {
            matches!(
                b.kind(),
                BlockKind::Rock
                    | BlockKind::WeakRock
                    | BlockKind::GlowingRock
                    | BlockKind::GlowingWeakRock
                    | BlockKind::Grass
                    | BlockKind::Snow
                    | BlockKind::ArtSnow
                    | BlockKind::Earth
                    | BlockKind::Sand
                    | BlockKind::Ice
            )
        })
    })
}

/// Guaranteed-flat rock slab + clear air above, centered on (ax, ay).
/// Returns buffered write count (applied next tick).
fn flatten_pad(server: &mut Server, ax: i32, ay: i32, pad_z: i32, half: i32, clear_h: i32) -> usize {
    use common::terrain::{Block, BlockKind};
    use vek::Rgb;
    let mut writes = 0usize;
    for x in (ax - half)..=(ax + half) {
        for y in (ay - half)..=(ay + half) {
            for dz in -1..=0 {
                server.state_mut().set_block(
                    Vec3::new(x, y, pad_z + dz),
                    Block::new(BlockKind::Rock, Rgb::new(120, 120, 120)),
                );
                writes += 1;
            }
            for dz in 1..=clear_h {
                server
                    .state_mut()
                    .set_block(Vec3::new(x, y, pad_z + dz), Block::empty());
                writes += 1;
            }
        }
    }
    writes
}

/// Issue a goto and tick until a terminal state (arrival / watchdog-stuck /
/// budget timeout). Clears the order before returning.
fn goto_and_wait(
    server: &mut Server,
    dt: Duration,
    name: &str,
    target: Vec3<f32>,
    budget: f32,
) -> GotoOutcome {
    if !server.bastion_goto(name, target) {
        return GotoOutcome::Refused;
    }
    loop {
        tick(server, dt, 15);
        let st = server
            .bastion_goto_states()
            .into_iter()
            .find(|s| s.0 == name);
        match st {
            Some((_, pos, target, elapsed, arrived, stuck)) => {
                if arrived {
                    server.bastion_goto_clear(Some(name));
                    return GotoOutcome::Arrived { elapsed };
                }
                if stuck {
                    let best = pos.distance(target);
                    server.bastion_goto_clear(Some(name));
                    return GotoOutcome::Stuck { elapsed, best };
                }
                if elapsed > budget {
                    let dist = pos.distance(target);
                    server.bastion_goto_clear(Some(name));
                    return GotoOutcome::Timeout { elapsed, dist };
                }
            },
            None => return GotoOutcome::Lost,
        }
    }
}

/// Issue simultaneous gotos for several colonists and wait until ALL arrive
/// (pass) or any terminal failure / budget expiry. Clears all orders.
fn goto_all_and_wait(
    server: &mut Server,
    dt: Duration,
    names: &[String],
    targets: &[Vec3<f32>],
    budget: f32,
) -> (bool, String) {
    for (name, target) in names.iter().zip(targets) {
        if !server.bastion_goto(name, *target) {
            server.bastion_goto_clear(None);
            return (false, format!("goto refused for {name}"));
        }
    }
    loop {
        tick(server, dt, 15);
        let states = server.bastion_goto_states();
        if states.len() < names.len() {
            server.bastion_goto_clear(None);
            return (false, "an order was lost (demote mid-travel?)".into());
        }
        if states.iter().all(|s| s.4) {
            let max_t = states.iter().map(|s| s.3).fold(0.0f32, f32::max);
            server.bastion_goto_clear(None);
            return (true, format!("all {} arrived by {max_t:.1}s", names.len()));
        }
        if let Some(s) = states.iter().find(|s| s.5) {
            let msg = format!("{} STUCK after {:.1}s", s.0, s.3);
            server.bastion_goto_clear(None);
            return (false, msg);
        }
        if states.iter().any(|s| s.3 > budget) {
            let unarrived: Vec<_> = states.iter().filter(|s| !s.4).map(|s| s.0.clone()).collect();
            server.bastion_goto_clear(None);
            return (false, format!("budget {budget:.0}s expired; not arrived: {unarrived:?}"));
        }
    }
}

/// Geometric interior target: walkable cell (solid below, non-solid feet+head)
/// inside the placed bounds, maximizing distance from the bounds edge (≥ 3 so
/// ARRIVE_DIST 2.5 can't false-arrive through a wall).
fn interior_target(server: &Server, report: &PlacementReport, pad_z: i32) -> Option<Vec3<f32>> {
    use common::vol::ReadVol;
    let terrain = server.state().terrain();
    let b = report.bounds;
    let mut best: Option<(i32, Vec3<i32>)> = None;
    for x in b.min.x..b.max.x {
        for y in b.min.y..b.max.y {
            let edge_dist = (x - b.min.x)
                .min(b.max.x - 1 - x)
                .min(y - b.min.y)
                .min(b.max.y - 1 - y);
            if edge_dist < 3 {
                continue;
            }
            for z in pad_z..(pad_z + 8) {
                let below = terrain.get(Vec3::new(x, y, z)).ok().copied();
                let feet = terrain.get(Vec3::new(x, y, z + 1)).ok().copied();
                let head = terrain.get(Vec3::new(x, y, z + 2)).ok().copied();
                let walkable = below.is_some_and(|b| b.is_filled())
                    && feet.is_some_and(|b| !b.is_solid())
                    && head.is_some_and(|b| !b.is_solid());
                if walkable && best.is_none_or(|(d, _)| edge_dist > d) {
                    best = Some((edge_dist, Vec3::new(x, y, z + 1)));
                }
            }
        }
    }
    best.map(|(_, p)| p.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0))
}

/// Rock fixture walls sealing three sides of the defense yard (the asset line
/// is the fourth). Pure test fixture, not part of the asset under test.
fn build_yard_walls(server: &mut Server, line_bounds: Aabb<i32>, yard_depth: i32, pad_z: i32) {
    use common::terrain::{Block, BlockKind};
    use vek::Rgb;
    let rock = Block::new(BlockKind::Rock, Rgb::new(90, 90, 90));
    let x0 = line_bounds.min.x - 1;
    let x1 = line_bounds.max.x;
    let y_south = line_bounds.min.y - yard_depth;
    for z in (pad_z + 1)..=(pad_z + 8) {
        for y in y_south..line_bounds.max.y {
            server.state_mut().set_block(Vec3::new(x0, y, z), rock);
            server.state_mut().set_block(Vec3::new(x1, y, z), rock);
        }
        for x in x0..=x1 {
            server.state_mut().set_block(Vec3::new(x, y_south, z), rock);
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn run_one_asset(
    server: &mut Server,
    dt: Duration,
    cfg: &AssetTestConfig,
    entry: &bastion_assets::AssetLabEntry,
    names: &[String],
    ax: i32,
    ay: i32,
    pad_z: i32,
    staging: Vec3<f32>,
    site_wpos: Vec2<f32>,
) -> AssetResult {
    let mut assertions: Vec<Assertion> = Vec::new();
    let mut push = |assertions: &mut Vec<Assertion>, name: &str, pass: bool, detail: String| {
        info!(asset = entry.id, name, pass, detail, "asset-test assertion");
        assertions.push(Assertion { name: name.into(), pass, detail });
    };

    if matches!(entry.category, AssetCategory::Creature) {
        return AssetResult {
            id: entry.id.clone(),
            category: format!("{:?}", entry.category),
            mode: "SKIP (creature — later integration rung)".into(),
            fidelity_ok: true,
            marker_checks: vec![],
            blocks_placed: 0,
            sprite_cfgs_dropped: 0,
            entity_spawners_skipped: 0,
            assertions: vec![],
            pass: true,
        };
    }

    let load_only = matches!(entry.category, AssetCategory::Prop | AssetCategory::Item);
    let mode = if load_only { "load-only" } else { "isolated-dynamic" };

    // Fresh arena per asset; load-only assets never touch the world.
    let placed: Result<(LoadedAsset, PlacementReport), String> = if load_only {
        bastion_assets::load_asset(entry, false).map(|l| (l, PlacementReport::default()))
    } else {
        flatten_pad(server, ax, ay, pad_z, 44, 28);
        tick(server, dt, 3);
        server.bastion_asset_place(entry, Vec3::new(ax, ay, pad_z), false, cfg.seed)
    };

    let (loaded, report) = match placed {
        Ok(v) => v,
        Err(e) => {
            push(&mut assertions, "load", false, format!("malformed/unreadable: {e}"));
            return AssetResult {
                id: entry.id.clone(),
                category: format!("{:?}", entry.category),
                mode: mode.into(),
                fidelity_ok: false,
                marker_checks: vec![],
                blocks_placed: 0,
                sprite_cfgs_dropped: 0,
                entity_spawners_skipped: 0,
                assertions,
                pass: false,
            };
        },
    };
    tick(server, dt, 3); // apply the placement BlockChange

    let marker_checks: Vec<String> = loaded
        .checks
        .iter()
        .map(|c| {
            format!(
                "byte {} x{}: expected {}, resolved {} [{}]",
                c.byte,
                c.count,
                c.expected,
                c.resolved,
                if c.ok { "ok" } else { "MISMATCH" }
            )
        })
        .collect();
    push(
        &mut assertions,
        "marker-fidelity",
        loaded.fidelity_ok,
        format!("{} distinct bytes checked", loaded.checks.len()),
    );

    if !load_only {
        match entry.category {
            AssetCategory::Structure | AssetCategory::TestFixture | AssetCategory::Other => {
                // Interior function point (geometric).
                match interior_target(server, &report, pad_z) {
                    Some(interior) => {
                        // Reachability + traversal + arrival.
                        let leg = goto_and_wait(server, dt, &names[0], interior, ARRIVAL_BUDGET_S);
                        push(&mut assertions, "reach-interior", leg.arrived(), leg.describe());
                        // Egress (only meaningful if we got in).
                        if leg.arrived() {
                            let out = goto_and_wait(server, dt, &names[0], staging, ARRIVAL_BUDGET_S);
                            push(&mut assertions, "egress", out.arrived(), out.describe());
                            // Multi-occupancy: 3 in, 3 out, same door.
                            let (in_ok, in_detail) = goto_all_and_wait(
                                server,
                                dt,
                                names,
                                &[interior, interior, interior],
                                MULTI_BUDGET_S,
                            );
                            push(&mut assertions, "multi-occupancy-in", in_ok, in_detail);
                            let outs: Vec<Vec3<f32>> = (0..names.len())
                                .map(|i| staging + Vec3::new(2.0 * i as f32, 0.0, 0.0))
                                .collect();
                            let (out_ok, out_detail) =
                                goto_all_and_wait(server, dt, names, &outs, MULTI_BUDGET_S);
                            push(&mut assertions, "multi-occupancy-out", out_ok, out_detail);
                        }
                    },
                    None => {
                        push(
                            &mut assertions,
                            "reach-interior",
                            false,
                            "no interior walkable cell ≥3 from bounds edge (sealed or solid?)"
                                .into(),
                        );
                    },
                }

                // Integrated-dynamic spot check (real worldgen terrain), only
                // for the flagship interior structure to bound wall time.
                if entry.id == "structure_housing_human_cottage" {
                    run_integrated_spot_check(
                        server, dt, cfg, entry, names, site_wpos, staging, &mut assertions,
                        &mut push,
                    );
                }
            },
            AssetCategory::Defense => {
                // Yard: fixture walls seal three sides; the line (with its
                // gate) is the north side. Outside = north of the line.
                build_yard_walls(server, report.bounds, 18, pad_z);
                tick(server, dt, 3);
                let cx = (report.bounds.min.x + report.bounds.max.x) / 2;
                let outside = Vec3::new(
                    cx as f32 + 0.5,
                    (report.bounds.max.y + 8) as f32 + 0.5,
                    (pad_z + 1) as f32,
                );
                let yard = Vec3::new(
                    cx as f32 + 0.5,
                    (report.bounds.min.y - 9) as f32 + 0.5,
                    (pad_z + 1) as f32,
                );
                // Stage the colonist outside first.
                let stage = goto_and_wait(server, dt, &names[0], outside, ARRIVAL_BUDGET_S);
                if !stage.arrived() {
                    push(&mut assertions, "defense-staging", false, stage.describe());
                } else {
                    // CLOSED: must NOT get in (watchdog stuck or timeout = pass).
                    let closed = goto_and_wait(server, dt, &names[0], yard, ARRIVAL_BUDGET_S);
                    push(
                        &mut assertions,
                        "gate-closed-blocks",
                        !closed.arrived(),
                        closed.describe(),
                    );
                    // OPEN variant: re-place open, rebuild fixtures, must get in
                    // (this also validates the yard fixture isn't leaky — a leak
                    // would have shown as closed-arrival).
                    flatten_pad(server, ax, ay, pad_z, 44, 28);
                    tick(server, dt, 3);
                    match server.bastion_asset_place(
                        entry,
                        Vec3::new(ax, ay, pad_z),
                        true,
                        cfg.seed,
                    ) {
                        Ok((_, open_report)) => {
                            build_yard_walls(server, open_report.bounds, 18, pad_z);
                            tick(server, dt, 3);
                            let restage =
                                goto_and_wait(server, dt, &names[0], outside, ARRIVAL_BUDGET_S);
                            let open = if restage.arrived() {
                                goto_and_wait(server, dt, &names[0], yard, ARRIVAL_BUDGET_S)
                            } else {
                                GotoOutcome::Refused
                            };
                            push(
                                &mut assertions,
                                "gate-open-admits",
                                open.arrived(),
                                open.describe(),
                            );
                            if open.arrived() {
                                let egress =
                                    goto_and_wait(server, dt, &names[0], outside, ARRIVAL_BUDGET_S);
                                push(
                                    &mut assertions,
                                    "gate-open-egress",
                                    egress.arrived(),
                                    egress.describe(),
                                );
                            }
                        },
                        Err(e) => {
                            push(&mut assertions, "gate-open-admits", false, format!("re-place failed: {e}"));
                        },
                    }
                }
            },
            AssetCategory::Flora => {
                let b = report.bounds;
                let beyond = Vec3::new(
                    ax as f32 + 0.5,
                    (b.max.y + 6) as f32 + 0.5,
                    (pad_z + 1) as f32,
                );
                let out = goto_and_wait(server, dt, &names[0], beyond, ARRIVAL_BUDGET_S);
                push(&mut assertions, "path-around", out.arrived(), out.describe());
                if out.arrived() {
                    let home = goto_and_wait(server, dt, &names[0], staging, ARRIVAL_BUDGET_S);
                    push(&mut assertions, "path-back", home.arrived(), home.describe());
                }
            },
            _ => {},
        }
    }

    let pass = assertions.iter().all(|a| a.pass);
    AssetResult {
        id: entry.id.clone(),
        category: format!("{:?}", entry.category),
        mode: mode.into(),
        fidelity_ok: loaded.fidelity_ok,
        marker_checks,
        blocks_placed: report.blocks_placed,
        sprite_cfgs_dropped: report.sprite_cfgs_dropped,
        entity_spawners_skipped: report.entity_spawners_skipped,
        assertions,
        pass,
    }
}

/// INTEGRATED-DYNAMIC: the same reach/egress on real, unflattened worldgen
/// terrain — the flat plane's blind spot, sampled (spec Part 2).
#[expect(clippy::too_many_arguments)]
fn run_integrated_spot_check(
    server: &mut Server,
    dt: Duration,
    cfg: &AssetTestConfig,
    entry: &bastion_assets::AssetLabEntry,
    names: &[String],
    site_wpos: Vec2<f32>,
    pad_staging: Vec3<f32>,
    assertions: &mut Vec<Assertion>,
    push: &mut impl FnMut(&mut Vec<Assertion>, &str, bool, String),
) {
    let spot = site_wpos + Vec2::new(-160.0, 96.0);
    server.bastion_force_load_area(spot, 4);
    let sx = spot.x as i32;
    let sy = spot.y as i32;
    let Some(gz) = ground_z(server, sx, sy) else {
        push(assertions, "integrated-reach", false, "no ground at integrated spot".into());
        return;
    };
    // Slope across the footprint (for the log; natural terrain, unflattened).
    let corners = [(-8, -6), (8, -6), (-8, 6), (8, 6)];
    let zs: Vec<i32> = corners
        .iter()
        .filter_map(|(dx, dy)| ground_z(server, sx + dx, sy + dy))
        .collect();
    let slope = zs.iter().max().copied().unwrap_or(gz) - zs.iter().min().copied().unwrap_or(gz);

    match server.bastion_asset_place(entry, Vec3::new(sx, sy, gz), false, cfg.seed) {
        Ok((_, report)) => {
            tick(server, dt, 3);
            let approach = Vec3::new(sx as f32 + 0.5, (report.bounds.min.y - 10) as f32, 0.0);
            let approach_z = ground_z(server, approach.x as i32, approach.y as i32).unwrap_or(gz);
            let approach = approach.with_z((approach_z + 1) as f32);
            // Walk the fixture over from the pad (may be a long leg — budget ×3).
            let travel = goto_and_wait(server, dt, &names[0], approach, ARRIVAL_BUDGET_S * 3.0);
            if !travel.arrived() {
                push(
                    assertions,
                    "integrated-reach",
                    false,
                    format!("could not stage at integrated spot: {}", travel.describe()),
                );
                return;
            }
            match interior_target(server, &report, gz) {
                Some(interior) => {
                    let leg = goto_and_wait(server, dt, &names[0], interior, ARRIVAL_BUDGET_S);
                    push(
                        assertions,
                        "integrated-reach",
                        leg.arrived(),
                        format!("slope {slope} across footprint; {}", leg.describe()),
                    );
                    if leg.arrived() {
                        let out = goto_and_wait(server, dt, &names[0], approach, ARRIVAL_BUDGET_S);
                        push(assertions, "integrated-egress", out.arrived(), out.describe());
                    }
                },
                None => push(
                    assertions,
                    "integrated-reach",
                    false,
                    format!("slope {slope}: no interior walkable cell found on natural terrain"),
                ),
            }
            // Send the fixture home for later assets.
            let _ = goto_and_wait(server, dt, &names[0], pad_staging, ARRIVAL_BUDGET_S * 3.0);
        },
        Err(e) => push(assertions, "integrated-reach", false, format!("place failed: {e}")),
    }
}

/// Append machine-readable results to `readme/ASSET_INTEGRATION_LOG.md`
/// (created with the format-contract header on first run; append-only after).
fn append_integration_log(results: &[AssetResult], cfg: &AssetTestConfig) {
    let log_path = PathBuf::from("readme/ASSET_INTEGRATION_LOG.md");
    let mut body = String::new();
    if !log_path.exists() {
        body.push_str(
            "# ASSET INTEGRATION LOG (game-side, append-only)\n\
             \n\
             Written by `bastion-harness --asset-test` (B-ASSET1). The asset session reads\n\
             this back to promote READY-pending-dynamic → READY-INTEGRATED. One dated block\n\
             per run; one JSON line per asset (schema: AssetResult in\n\
             `bastion-harness/src/asset_test.rs`).\n\
             \n\
             FORMAT CONTRACT (engine side — see docs/BASTION_BASSET1_FINDINGS.md):\n\
             - Input: flattened `.vox` under `asset-lab/vox/` (compose.py output). Sidecar\n\
             metadata optional; category inferred from id prefix.\n\
             - Byte bands (ASSET_LESSONS L3): 1–16 world-reserved (engine defaults),\n\
             32–199 literals, 200–255 gameplay markers via the engine marker registry:\n\
             200 = gate KeyholeBars (closed) / carved air (open variant),\n\
             206/207/208/209 = pressure-plate/desk/bench/bed → carved air, cells recorded\n\
             as function points. UNKNOWN 200-255 bytes fail marker fidelity — extend\n\
             `server/src/bastion_assets.rs::marker_registry` first.\n\
             - Figure-layer assets (props/items at 11 vox/block, creatures) are load-only /\n\
             SKIP here; their world integration is manifest work (a later block).\n\
             - `test_*` fixtures run only when named explicitly (deliberate-FAIL demos).\n\n",
        );
    }
    body.push_str(&format!(
        "## RUN {} · seed {} · target `{}`\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"),
        cfg.seed,
        cfg.target
    ));
    for r in results {
        body.push_str(&format!(
            "ASSET {} DYNAMIC-ISOLATED: {}\n```json\n{}\n```\n",
            r.id,
            if r.pass { "PASS" } else { "FAIL" },
            serde_json::to_string(r).expect("serializable")
        ));
    }
    body.push('\n');
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(mut f) => {
            let _ = f.write_all(body.as_bytes());
            info!(?log_path, "asset-test: integration log appended");
        },
        Err(e) => eprintln!("ASSET-TEST: could not append {}: {e}", log_path.display()),
    }
}
