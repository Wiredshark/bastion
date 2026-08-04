//! Live-playthrough driver (LIVE-PLAYTHROUGH-PREP.md): drives the real
//! `ClientGeneral::Bastion*` wire path against a real hosted server-cli
//! instance, exactly what a player's client would send -- never harness
//! scenario injection. Actions come from a line-oriented script file so an
//! LLM player can decide the next script from the previous run's log
//! instead of everything being fixed up front.
//!
//! Script grammar (one command per line, `#` comments, blank lines skipped):
//!   wait <ticks>
//!   anchor                              -- terrain anchor at current pos
//!   spawn <count>                       -- found colony at current pos
//!   designate <kind> <x0> <y0> <z0> <x1> <y1> <z1>
//!       kind in: mine chop build stockpile ladder gather bed farm
//!   cancel <x0> <y0> <z0> <x1> <y1> <z1>
//!   inspect_cell <x> <y> <z>
//!   list_designations
//!   survey <x0> <y0> <x1> <y1> <ztop> <zbot> <gap>
//!       for each (x,y) in the 2D box: walk down from ztop, find the
//!       topmost filled cell (the "surface"); a column is flagged an
//!       OVERHANG CANDIDATE if there are >= <gap> consecutive unfilled
//!       cells immediately beneath that surface before hitting solid
//!       ground again (or the scan reaches zbot without finding any) --
//!       the same terrain data a real client's renderer reads, not a
//!       harness-only view.
//!   note <free text>                    -- marker only, logged verbatim

use common::{
    ViewDistances,
    bastion::{DesignationKind, Region},
    clock::Clock,
    comp::{self, bastion::BastionInspectTarget, body::humanoid::Body},
    terrain::TerrainGrid,
    vol::ReadVol,
};
use common_net::msg::ServerInfo;
use std::{
    fs,
    io::Write,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Runtime;
use tracing::warn;
use vek::Vec3;
use veloren_client::{Client, ClientType, Event, WorldExt, addr::ConnectionArgs};

const TPS: u64 = 30;

fn ts() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

struct Logger {
    file: fs::File,
}

impl Logger {
    fn new(path: &str) -> Self {
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("failed to open playtest log file");
        Self { file }
    }

    fn log(&mut self, line: &str) {
        println!("{line}");
        let _ = writeln!(self.file, "[{}] {}", ts(), line);
    }
}

enum ScriptCmd {
    Wait(u64),
    Anchor,
    Spawn(u8),
    Designate(DesignationKind, Region),
    Cancel(Region),
    InspectCell(Vec3<i32>),
    ListDesignations,
    Survey {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        ztop: i32,
        zbot: i32,
        gap: i32,
    },
    Note(String),
}

fn parse_kind(s: &str) -> Option<DesignationKind> {
    Some(match s {
        "mine" => DesignationKind::Mine,
        "chop" => DesignationKind::Chop,
        "build" => DesignationKind::Build,
        "stockpile" => DesignationKind::Stockpile,
        "ladder" => DesignationKind::Ladder,
        "gather" => DesignationKind::Gather,
        "bed" => DesignationKind::Bed,
        "farm" => DesignationKind::Farm,
        _ => return None,
    })
}

fn parse_region(parts: &[&str]) -> Option<Region> {
    if parts.len() != 6 {
        return None;
    }
    let n: Vec<i32> = parts.iter().map(|p| p.parse().ok()).collect::<Option<_>>()?;
    Some(
        Region {
            min: Vec3::new(n[0], n[1], n[2]),
            max: Vec3::new(n[3], n[4], n[5]),
        }
        .normalized(),
    )
}

fn parse_script(path: &str) -> Vec<ScriptCmd> {
    let text = fs::read_to_string(path).expect("failed to read script file");
    let mut cmds = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let verb = parts.next().unwrap();
        let rest: Vec<&str> = parts.collect();
        let cmd = match verb {
            "wait" => ScriptCmd::Wait(rest[0].parse().expect("bad wait ticks")),
            "anchor" => ScriptCmd::Anchor,
            "spawn" => ScriptCmd::Spawn(rest[0].parse().expect("bad spawn count")),
            "designate" => {
                let kind = parse_kind(rest[0]).unwrap_or_else(|| panic!("bad kind at line {lineno}: {}", rest[0]));
                let region = parse_region(&rest[1..]).unwrap_or_else(|| panic!("bad region at line {lineno}"));
                ScriptCmd::Designate(kind, region)
            },
            "cancel" => {
                let region = parse_region(&rest).unwrap_or_else(|| panic!("bad region at line {lineno}"));
                ScriptCmd::Cancel(region)
            },
            "inspect_cell" => {
                let n: Vec<i32> = rest.iter().map(|p| p.parse().unwrap()).collect();
                ScriptCmd::InspectCell(Vec3::new(n[0], n[1], n[2]))
            },
            "list_designations" => ScriptCmd::ListDesignations,
            "survey" => {
                let n: Vec<i32> = rest.iter().map(|p| p.parse().unwrap()).collect();
                ScriptCmd::Survey {
                    x0: n[0],
                    y0: n[1],
                    x1: n[2],
                    y1: n[3],
                    ztop: n[4],
                    zbot: n[5],
                    gap: n[6],
                }
            },
            "note" => ScriptCmd::Note(rest.join(" ")),
            other => panic!("unknown script verb at line {lineno}: {other}"),
        };
        cmds.push(cmd);
    }
    cmds
}

fn main() {
    let mut args = std::env::args().skip(1);
    let server = args.next().unwrap_or_else(|| "localhost".to_string());
    let username = args.next().unwrap_or_else(|| "bastion_llm_player".to_string());
    let script_path = args.next().expect("usage: bastion_playtest <server> <username> <script_file> [log_file]");
    let log_path = args
        .next()
        .unwrap_or_else(|| "bastion-test-evidence/live-playthrough/driver.log".to_string());

    if let Some(parent) = std::path::Path::new(&log_path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut log = Logger::new(&log_path);
    log.log(&format!(
        "=== bastion_playtest starting: server={server} username={username} script={script_path} ==="
    ));

    let script = parse_script(&script_path);
    log.log(&format!("parsed {} script commands", script.len()));

    let runtime = Arc::new(Runtime::new().unwrap());
    let addr = ConnectionArgs::Tcp {
        prefer_ipv6: false,
        hostname: server,
    };

    let mut server_info: Option<ServerInfo> = None;
    let mut client = runtime
        .block_on(Client::new(
            addr,
            Arc::clone(&runtime),
            &mut server_info,
            &username,
            "",
            None,
            |_| true,
            &|stage| println!("[init] {stage:?}"),
            |_| {},
            Default::default(),
            ClientType::Game,
        ))
        .expect("failed to connect to server");
    log.log(&format!("connected. server_info={:?}", client.server_info()));

    client.create_character(
        username.clone(),
        Some("common.items.weapons.sword.starter".to_string()),
        None,
        Body {
            species: comp::body::humanoid::Species::Human,
            body_type: comp::body::humanoid::BodyType::Male,
            hair_style: 0,
            beard: 0,
            eyes: 0,
            accessory: 0,
            hair_color: 0,
            skin: 0,
            eye_color: 0,
            height_scale: u8::MAX / 2,
        }
        .into(),
        false,
        None,
    );
    client.load_character_list();

    let mut clock = Clock::new(Duration::from_secs_f64(1.0 / TPS as f64));
    let mut requested_join = false;
    let mut ticks: u64 = 0;
    const JOIN_TIMEOUT_TICKS: u64 = TPS * 60;

    loop {
        match client.tick(comp::ControllerInputs::default(), clock.game_dt()) {
            Ok(events) => {
                for event in events {
                    if let Event::Chat(m) = &event {
                        log.log(&format!("[chat] {m:?}"));
                    }
                }
            },
            Err(e) => {
                log.log(&format!("tick error while joining: {e:?}"));
                panic!("tick error before reaching in-game");
            },
        }
        client.cleanup();
        clock.tick();
        ticks += 1;

        if !requested_join {
            let list = client.character_list();
            if !list.loading && !list.characters.is_empty() {
                if let Some(id) = list.characters[0].character.id {
                    client.request_character(id, ViewDistances {
                        terrain: 6,
                        entity: 6,
                    });
                    requested_join = true;
                    log.log(&format!("requested character join id={id:?}"));
                }
            }
        }

        if matches!(client.presence(), Some(comp::PresenceKind::Character(_))) {
            break;
        }

        if ticks > JOIN_TIMEOUT_TICKS {
            panic!("timed out waiting to reach in-game (character list never populated / never entered Character presence)");
        }
    }
    log.log(&format!("in-game. entity={:?}", client.entity()));

    fn own_pos(client: &Client) -> Option<Vec3<f32>> {
        client
            .state()
            .ecs()
            .read_storage::<comp::Pos>()
            .get(client.entity())
            .map(|p| p.0)
    }

    // Let terrain load a moment before reading position / painting.
    for _ in 0..(TPS * 2) {
        let _ = client.tick(comp::ControllerInputs::default(), clock.game_dt());
        client.cleanup();
        clock.tick();
    }

    let mut current_pos = own_pos(&client).unwrap_or_else(|| {
        warn!("no Pos component readable yet; defaulting to origin");
        Vec3::zero()
    });
    log.log(&format!("player pos at script start: {current_pos:?}"));

    for cmd in script {
        match cmd {
            ScriptCmd::Wait(n) => {
                for _ in 0..n {
                    match client.tick(comp::ControllerInputs::default(), clock.game_dt()) {
                        Ok(events) => {
                            for event in events {
                                if let Event::Chat(m) = &event {
                                    log.log(&format!("[chat] {m:?}"));
                                }
                            }
                        },
                        Err(e) => {
                            log.log(&format!("tick error during wait: {e:?}"));
                        },
                    }
                    client.cleanup();
                    clock.tick();
                }
                if let Some(p) = own_pos(&client) {
                    current_pos = p;
                }
                log.log(&format!("waited {n} ticks; pos now {current_pos:?}"));
            },
            ScriptCmd::Anchor => {
                client.bastion_set_terrain_anchor(Some(current_pos));
                log.log(&format!("sent BastionCameraAnchor at {current_pos:?}"));
            },
            ScriptCmd::Spawn(count) => {
                client.bastion_spawn_colony(current_pos, count);
                log.log(&format!("sent BastionSpawnColony pos={current_pos:?} count={count}"));
            },
            ScriptCmd::Designate(kind, region) => {
                client.bastion_place_designation(region, kind, None);
                log.log(&format!("sent BastionPlaceDesignation kind={kind:?} region={region:?}"));
            },
            ScriptCmd::Cancel(region) => {
                client.bastion_cancel_designation(region);
                log.log(&format!("sent BastionCancelDesignation region={region:?}"));
            },
            ScriptCmd::InspectCell(pos) => {
                client.bastion_inspect_request(BastionInspectTarget::Cell(pos));
                // One tick round-trip to receive the echoed reply.
                let _ = client.tick(comp::ControllerInputs::default(), clock.game_dt());
                client.cleanup();
                clock.tick();
                log.log(&format!("inspect_cell {pos:?} -> {:?}", client.bastion_inspect()));
            },
            ScriptCmd::ListDesignations => {
                log.log(&format!(
                    "designations (rev={}): {:?}",
                    client.bastion_designations_rev(),
                    client.bastion_designations()
                ));
            },
            ScriptCmd::Survey {
                x0,
                y0,
                x1,
                y1,
                ztop,
                zbot,
                gap,
            } => {
                let terrain = client.state().ecs().read_resource::<TerrainGrid>();
                let mut candidates = Vec::new();
                let mut columns_scanned = 0;
                let mut columns_no_surface = 0;
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        columns_scanned += 1;
                        let mut surface = None;
                        let mut z = ztop;
                        while z >= zbot {
                            if terrain
                                .get(Vec3::new(x, y, z))
                                .map(|b| b.is_filled())
                                .unwrap_or(false)
                            {
                                surface = Some(z);
                                break;
                            }
                            z -= 1;
                        }
                        let Some(sz) = surface else {
                            columns_no_surface += 1;
                            continue;
                        };
                        let mut empty_run = 0;
                        let mut zz = sz - 1;
                        while zz >= zbot {
                            let filled = terrain
                                .get(Vec3::new(x, y, zz))
                                .map(|b| b.is_filled())
                                .unwrap_or(false);
                            if filled {
                                break;
                            }
                            empty_run += 1;
                            zz -= 1;
                        }
                        if empty_run >= gap {
                            candidates.push((x, y, sz, empty_run));
                        }
                    }
                }
                drop(terrain);
                log.log(&format!(
                    "survey [{x0},{y0}]-[{x1},{y1}] z[{zbot},{ztop}] gap>={gap}: \
                     {columns_scanned} columns, {columns_no_surface} with no surface \
                     in range, {} overhang candidates: {candidates:?}",
                    candidates.len()
                ));
            },
            ScriptCmd::Note(text) => {
                log.log(&format!("[note] {text}"));
            },
        }
    }

    log.log("=== script complete, disconnecting ===");
}
