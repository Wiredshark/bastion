//! Live-playthrough driver (LIVE-PLAYTHROUGH-PREP.md): drives the real
//! `ClientGeneral::Bastion*` wire path against a real hosted server-cli
//! instance, exactly what a player's client would send -- never harness
//! scenario injection. Actions come from a line-oriented script file so an
//! LLM player can decide the next script from the previous run's log
//! instead of everything being fixed up front.
//!
//! Script grammar (one command per line, `#` comments, blank lines skipped):
//!   wait <ticks>                        -- ITEM 5: waits on the server-
//!       tracking `Time` resource (sim seconds = ticks / NOMINAL_TPS), not
//!       a raw client tick count -- the two clocks can drift under load.
//!       Bounded two ways, logged under DIFFERENT diagnoses: consecutive
//!       `client.tick()` errors (the engine's own `ServerTimeout`
//!       liveness check -- `Time` itself cannot detect a dead server, it
//!       advances locally every tick regardless) and a spin count
//!       underneath as a cheap absolute ceiling. Either firing makes the
//!       wait VOID, not short. Every wait logs both the requested ticks
//!       and the sim-time span actually covered.
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
//!   cmd <name> <args...>                -- raw chat command (e.g. `cmd
//!       give_item common.items.food.mushroom 50`, `cmd dropall`) --
//!       requires the connecting player hold the needed admin role
//!       (`server-cli admin add <user> admin`)

use common::{
    ViewDistances,
    bastion::{DesignationKind, Region},
    clock::Clock,
    comp::{self, bastion::BastionInspectTarget, body::humanoid::Body},
    resources::Time,
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
// ITEM 5 (ROW-WAIT-SERVER-AUTHORITATIVE-PACKET): `Wait(n)` waits on the
// server-tracking `Time` resource, not the client's own tick loop counter --
// those are different clocks, free to drift under load.
const NOMINAL_TPS: f64 = TPS as f64;
// Liveness signal, corrected 2026-08-10 (Opus's catch, verified against
// the client's own Time-advancement code before landing): `Time` cannot
// detect a stopped server -- it advances locally every tick regardless of
// server responsiveness (`State::tick`, `common/state/src/state.rs`), the
// server only steers it toward its own value. The real signal is
// `client.tick()` returning `Err(Error::ServerTimeout)` -- the engine's
// own `client_timeout`-based liveness check, not one invented here.
// Consecutive tick errors (not a lone one, which could be transient) trip
// VOID; `WAIT_SPIN_CAP_MULTIPLIER` stays underneath as a cheap absolute
// ceiling for whatever this somehow misses.
const WAIT_SPIN_CAP_MULTIPLIER: u64 = 20;

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

#[derive(Debug)]
enum ScriptCmd {
    Wait(u64),
    Anchor,
    /// FOUNDING PRESET acceptance (2026-08-12): the optional position is the
    /// FOUNDING TARGET, not the player's body. Packet §3.1 — "God TARGETS F via
    /// the overseer founding action" — the god aims, they do not have to stand
    /// on it, so `current_pos` was the driver's simplification rather than the
    /// UI's semantics. `None` keeps every existing script byte-identical.
    ///
    /// This is what makes §8 B1 testable LIVE: pass a z that differs from the
    /// column's first air cell and the emitted `datum=` must still resolve from
    /// TERRAIN. On the flat arena the player always settles to the datum, so
    /// standing where you found can never discriminate the two (smoke F-1).
    Spawn(u8, Option<Vec3<f32>>),
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
    // AUTON-2 MILESTONE LIVE SESSION (2026-08-09): a generic chat-command
    // send -- e.g. `cmd give_item common.items.food.mushroom 50` then
    // `cmd dropall` to place food on the ground for colonists to find.
    // Reuses `Client::send_command`, the same wire path a real player's
    // chat bar uses; requires the connecting player to hold the needed
    // role (`server-cli admin add <user> admin`), same as any other
    // admin-gated command.
    Cmd(String, Vec<String>),
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

/// THE VERB TABLE this driver accepts — the capability half of the build
/// fingerprint (F3).
///
/// A stale driver is not the defect; a stale driver that fails SILENTLY is.
/// The `no_overflow` binary of 2026-08-11 took `spawn 8 x y z`, kept the
/// count and discarded the coordinates, so nine distinct lattice origins all
/// became the anchor and the census read as "all nine points are identical".
/// It was caught by noticing a missing log field — luck, not method.
///
/// This table is printed at startup so any evidence log can be attributed to
/// a build, and `verb_table_matches_the_parser` drives every entry through
/// the REAL parser so the declaration cannot drift from it. A table written
/// beside the parser that nothing checks would go stale exactly as the
/// binary did.
pub const SCRIPT_VERBS: &[&str] = &[
    "wait",
    "anchor",
    "spawn",
    "designate",
    "cancel",
    "inspect_cell",
    "list_designations",
    "survey",
    "note",
    "cmd",
];

fn parse_script(path: &str) -> Vec<ScriptCmd> {
    let text = fs::read_to_string(path).expect("failed to read script file");
    parse_script_text(&text)
}

fn parse_script_text(text: &str) -> Vec<ScriptCmd> {
    let mut cmds = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let verb = parts.next().unwrap();
        // THE TABLE IS THE CONTRACT, not a comment about it. Gating here
        // makes `SCRIPT_VERBS` load-bearing: a verb removed from it stops
        // parsing, so the declaration cannot quietly under-report what this
        // binary accepts. Without this the table could drift silently in
        // the one direction that matters — exactly the failure mode the
        // stale driver had.
        if !SCRIPT_VERBS.contains(&verb) {
            panic!(
                "unknown script verb at line {lineno}: {verb} (this build declares: {})",
                SCRIPT_VERBS.join(",")
            );
        }
        let rest: Vec<&str> = parts.collect();
        let cmd = match verb {
            "wait" => ScriptCmd::Wait(rest[0].parse().expect("bad wait ticks")),
            "anchor" => ScriptCmd::Anchor,
            // `spawn <n>` founds at the player; `spawn <n> <x> <y> <z>` founds at
            // an explicitly TARGETED position (see ScriptCmd::Spawn).
            "spawn" => {
                let count = rest[0].parse().expect("bad spawn count");
                let target = match rest.len() {
                    1 => None,
                    4 => Some(Vec3::new(
                        rest[1].parse().unwrap_or_else(|_| panic!("bad spawn x at line {lineno}")),
                        rest[2].parse().unwrap_or_else(|_| panic!("bad spawn y at line {lineno}")),
                        rest[3].parse().unwrap_or_else(|_| panic!("bad spawn z at line {lineno}")),
                    )),
                    n => panic!("spawn takes <count> or <count> <x> <y> <z> at line {lineno}, got {n} args"),
                };
                ScriptCmd::Spawn(count, target)
            },
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
            "cmd" => {
                let name = rest.first().unwrap_or_else(|| panic!("missing command name at line {lineno}")).to_string();
                ScriptCmd::Cmd(name, rest[1..].iter().map(|s| s.to_string()).collect())
            },
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

    // F3 · BUILD FINGERPRINT, FIRST — before the script is even parsed.
    //
    // Every evidence log this driver has ever written was, strictly,
    // unattributed: nothing in it said which binary produced it. A stale
    // build that silently discarded arguments once turned nine distinct
    // origins into one and read as a consistent census. The commit and the
    // verb table go in the log so a reader can tell WHICH driver spoke, and
    // so a capability the script relies on is visible rather than assumed.
    //
    // CAVEAT, stated where it lives: `GIT_HASH` derives from
    // `VELOREN_GIT_VERSION`, which `common::util` lets a RUNTIME env var
    // override. Unset (as in every run here) it reports the build; set, it
    // would lie. It is the strongest identity this crate exposes, and its
    // limit is named rather than papered over.
    log.log(&format!(
        "driver build={:08x} built_at={} verbs={}",
        *common::util::GIT_HASH,
        *common::util::GIT_TIMESTAMP,
        SCRIPT_VERBS.join(",")
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
                    // bastion (fixture row): the driver's view distance is the
                    // ONE structural difference between a client-connected leg
                    // and the headless colony-presence leg, which uses
                    // COLONY_PRESENCE_VIEW_DISTANCE = 1. At the default 6 a
                    // client loads a 13x13 chunk area against presence's 3x3 --
                    // ~18x more world -- so the two configurations were never
                    // the same colony, and F2 measured them as if they were.
                    // Env-settable so ONE binary can run both ends of that
                    // comparison; unset = 6, the historical default, unchanged.
                    let vd = std::env::var("BASTION_DRIVER_VIEW_DISTANCE")
                        .ok()
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(6);
                    log.log(&format!("requesting view distance terrain={vd} entity={vd}"));
                    client.request_character(id, ViewDistances {
                        terrain: vd,
                        entity: vd,
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

    // ITEM 5: the server-tracking `Time` resource -- hard-resynced if the
    // client falls more than 5s behind, otherwise tweaked at ~1%/tick
    // (client/src/lib.rs's own dt_adjustment). Reachable today with no
    // protocol change; this is a read of state the client already
    // maintains, not a new signal.
    fn server_time(client: &Client) -> f64 {
        client.state().ecs().read_resource::<Time>().0
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
                // ITEM 5 (ROW-WAIT-SERVER-AUTHORITATIVE-PACKET): wait on
                // the server-tracking `Time` resource, not a raw client
                // tick count -- the two clocks are free to drift under
                // load, which is the root of every timing confusion the
                // food arc hit.
                //
                // Opus's catch (2026-08-10), verified before landing:
                // `Time` cannot be used as a liveness signal -- both the
                // regular tick path (`State::tick`,
                // `common/state/src/state.rs`, `write_resource::<Time>().0
                // += scaled_dt` every tick) and the harness-only
                // `tick_network` path advance `Time` LOCALLY every tick,
                // unconditionally; the server only STEERS it (hard resync
                // past 5s divergence, ~1% tween otherwise) via
                // `TimeOfDay`. A dead server's `Time` keeps moving at the
                // local rate with nothing behind it -- a "no sim advance"
                // check would never fire against the exact condition it
                // exists to catch, and would certify a void run as clean.
                //
                // The real liveness signal: `client.tick()` itself already
                // returns `Err(Error::ServerTimeout)` when the engine's own
                // `client_timeout` elapses with no server messages
                // (client/src/lib.rs, `handle_messages` -> the
                // `msg_count == 0` timeout check) -- a signal already
                // computed for us, not one we invent. Consecutive tick
                // errors (not a single one, which could be transient) trip
                // VOID; the spin count stays underneath as a cheap
                // absolute ceiling, logged under its own distinct reason
                // so "server died" and "rate model is wrong" never
                // collapse into one diagnosis.
                const CONSECUTIVE_TICK_ERROR_LIMIT: u32 = 3;
                let start = server_time(&client);
                let target = start + (n as f64) / NOMINAL_TPS;
                let spin_cap = n.saturating_mul(WAIT_SPIN_CAP_MULTIPLIER).max(1);
                let mut spins = 0u64;
                let mut consecutive_tick_errors = 0u32;
                let mut last_tick_error: Option<String> = None;
                #[derive(Debug)]
                enum WaitVoidReason {
                    ServerUnresponsive,
                    SpinCeiling,
                }
                let mut void_reason: Option<WaitVoidReason> = None;
                while server_time(&client) < target {
                    if spins >= spin_cap {
                        void_reason = Some(WaitVoidReason::SpinCeiling);
                        break;
                    }
                    match client.tick(comp::ControllerInputs::default(), clock.game_dt()) {
                        Ok(events) => {
                            consecutive_tick_errors = 0;
                            for event in events {
                                if let Event::Chat(m) = &event {
                                    log.log(&format!("[chat] {m:?}"));
                                }
                            }
                        },
                        Err(e) => {
                            log.log(&format!("tick error during wait: {e:?}"));
                            last_tick_error = Some(format!("{e:?}"));
                            consecutive_tick_errors += 1;
                            if consecutive_tick_errors >= CONSECUTIVE_TICK_ERROR_LIMIT {
                                void_reason = Some(WaitVoidReason::ServerUnresponsive);
                                break;
                            }
                        },
                    }
                    client.cleanup();
                    clock.tick();
                    spins += 1;
                }
                if let Some(p) = own_pos(&client) {
                    current_pos = p;
                }
                let end = server_time(&client);
                match void_reason {
                    Some(WaitVoidReason::ServerUnresponsive) => log.log(&format!(
                        "VOID: server unresponsive ({consecutive_tick_errors} consecutive tick errors, last: {}) -- wait {n} ticks (target sim {start:.2}..{target:.2}), stuck at sim {end:.2} after {spins} spins; pos now {current_pos:?}",
                        last_tick_error.unwrap_or_default()
                    )),
                    Some(WaitVoidReason::SpinCeiling) => log.log(&format!(
                        "VOID: absolute spin ceiling ({spin_cap}) hit -- wait {n} ticks (target sim {start:.2}..{target:.2}) reached only sim {end:.2}; rate model likely wrong, not a dead server; pos now {current_pos:?}"
                    )),
                    None => log.log(&format!(
                        "waited {n} ticks -> sim {start:.2}..{end:.2} in {spins} client spins; pos now {current_pos:?}"
                    )),
                }
            },
            ScriptCmd::Anchor => {
                client.bastion_set_terrain_anchor(Some(current_pos));
                log.log(&format!("sent BastionCameraAnchor at {current_pos:?}"));
            },
            ScriptCmd::Spawn(count, target) => {
                // The TARGET is what the god aims at; the player's body is only
                // the default. Logged distinctly so a scored run can tell which
                // it was without re-reading the script.
                let pos = target.unwrap_or(current_pos);
                client.bastion_spawn_colony(pos, count);
                log.log(&format!(
                    "sent BastionSpawnColony pos={pos:?} count={count} targeted={}",
                    target.is_some()
                ));
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
            ScriptCmd::Cmd(name, cmd_args) => {
                log.log(&format!("sent chat command /{name} {cmd_args:?}"));
                client.send_command(name, cmd_args);
                // One tick round-trip so the resulting chat feedback (or
                // error) lands in the log before the next script line.
                match client.tick(comp::ControllerInputs::default(), clock.game_dt()) {
                    Ok(events) => {
                        for event in events {
                            if let Event::Chat(m) = &event {
                                log.log(&format!("[chat] {m:?}"));
                            }
                        }
                    },
                    Err(e) => {
                        log.log(&format!("tick error after cmd: {e:?}"));
                    },
                }
                client.cleanup();
                clock.tick();
            },
        }
    }

    log.log("=== script complete, disconnecting ===");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D3 · **THE DECLARATION CANNOT DRIFT FROM THE PARSER.**
    ///
    /// Every verb in `SCRIPT_VERBS` is driven through the REAL parser. A
    /// verb in the table the parser rejects fails here; a verb the parser
    /// gained without the table fails the companion test below. A
    /// capability list that nothing checks would go stale exactly as the
    /// 2026-08-11 binary did — that is the whole defect this guards.
    #[test]
    fn verb_table_matches_the_parser() {
        // A minimal VALID line per verb. Written as script text and parsed,
        // not asserted about.
        let sample = |verb: &str| -> String {
            match verb {
                "wait" => "wait 1".into(),
                "anchor" => "anchor".into(),
                "spawn" => "spawn 8".into(),
                "designate" => "designate chop 1 2 3 4 5 6".into(),
                "cancel" => "cancel 1 2 3 4 5 6".into(),
                "inspect_cell" => "inspect_cell 1 2 3".into(),
                "list_designations" => "list_designations".into(),
                "survey" => "survey 1 2 3 4 5 6 7".into(),
                "note" => "note hello".into(),
                "cmd" => "cmd dropall".into(),
                other => panic!(
                    "SCRIPT_VERBS lists `{other}` but this test has no sample line for it -- \
                     the table grew and its check did not"
                ),
            }
        };
        for verb in SCRIPT_VERBS {
            let parsed = parse_script_text(&sample(verb));
            assert_eq!(
                parsed.len(),
                1,
                "declared verb `{verb}` must parse to exactly one command"
            );
        }
    }

    /// D3b · a verb NOT in the table must be rejected, or the table would be
    /// a floor rather than the contract.
    #[test]
    #[should_panic(expected = "unknown script verb")]
    fn an_undeclared_verb_is_rejected() { parse_script_text("teleport 1 2 3"); }

    /// D2 · **THE EXACT CAPABILITY WHOSE SILENT ABSENCE VOIDED A CENSUS.**
    /// Four arguments must yield a TARGETED spawn and one must not.
    #[test]
    fn spawn_targeting_is_carried_not_discarded() {
        match parse_script_text("spawn 8 15184.5 15984.5 419.0").as_slice() {
            [ScriptCmd::Spawn(8, Some(target))] => {
                assert_eq!(target.x, 15184.5);
                assert_eq!(target.y, 15984.5);
                assert_eq!(target.z, 419.0);
            },
            other => panic!("four args must target; got {other:?}"),
        }
        match parse_script_text("spawn 8").as_slice() {
            [ScriptCmd::Spawn(8, None)] => {},
            other => panic!("one arg must not target; got {other:?}"),
        }
    }

    /// D2b · **ARGUMENTS ARE REFUSED, NOT IGNORED.** The stale binary took
    /// the count and dropped the rest, which is why nine origins collapsed
    /// into one with nothing in the log to show for it. An arity the parser
    /// cannot honour must fail LOUDLY, at its line.
    #[test]
    #[should_panic(expected = "spawn takes")]
    fn a_spawn_arity_the_parser_cannot_honour_panics() {
        parse_script_text("spawn 8 15184.5 15984.5");
    }
}
