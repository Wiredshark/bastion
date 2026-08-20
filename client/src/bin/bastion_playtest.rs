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
// ACK BARRIER: bound on waiting for a command acknowledgement. Generous --
// it exists to stop an unbounded hang, not to time the server.
const ACK_SPIN_CAP: u32 = 600;

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
    InspectColonists,
    InspectColony,
    /// ARC 2 item 12: request colonist chronicles (the entity-log player view).
    InspectChronicle,
    CountItems,
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

/// ACK BARRIER (ack-barrier row): tick until the server's designation
/// revision moves off `rev_before`, or the bound expires.
///
/// Returns whether the transition was OBSERVED. A timeout is reported by the
/// caller as `ACK-TIMEOUT` and never as silence -- an unobserved ack and a
/// fast one must not render identically, which is the same rule the inspect
/// reply-matching had to learn.
fn await_designation_rev(client: &mut Client, clock: &mut Clock, rev_before: u64) -> bool {
    for _ in 0..ACK_SPIN_CAP {
        let _ = client.tick(comp::ControllerInputs::default(), driver_dt(clock));
        client.cleanup();
        driver_pace(clock, client);
        if client.bastion_designations_rev() != rev_before {
            return true;
        }
    }
    false
}

/// DROP WITNESS row: how many loose `PickupItem` entities the CLIENT can see.
///
/// `PickupItem` is a synced component (`impl NetSync for PickupItem`), so a
/// dropped item is observable from here. `dropall` emits no chat reply and was
/// therefore treated as unobservable -- but the observable was an ENTITY, not a
/// message.
fn count_pickup_items(client: &Client) -> usize {
    use specs::Join;
    let ecs = client.state().ecs();
    let items = ecs.read_storage::<comp::PickupItem>();
    (&items).join().count()
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
        "cookstation" => DesignationKind::CookStation,
        "farm" => DesignationKind::Farm,
        // ITEM 14: both guard assignment types are paintable from a script,
        // because bar 2 requires BOTH to land in a live run and a fixture that
        // can only paint one would score half the axis and look complete.
        "guardpost" => DesignationKind::GuardPost,
        "patrol" => DesignationKind::PatrolPoint,
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
    "inspect_colonists",
    "inspect_colony",
    "inspect_chronicle",
    "count_items",
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
            "inspect_colonists" => ScriptCmd::InspectColonists,
            "inspect_colony" => ScriptCmd::InspectColony,
            "inspect_chronicle" => ScriptCmd::InspectChronicle,
            "count_items" => ScriptCmd::CountItems,
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


// ═══ BASTION_TICK_DRIVEN_DRIVER — the OPT-IN arm for row #89 bar 2 ═══
//
// Bar 2 fails on its timing clause and four candidate causes were eliminated by
// measurement, every one a wall-clock READ inside a loop. The cause is the loop
// itself: this driver and the server are two independently WALL-PACED loops, so
// the mapping spin -> server-tick is a function of the wall clock and no amount
// of gating individual reads can remove it.
//
// This arm converts BOTH halves of the coupling, because converting one is
// worse than converting neither -- a half-fix leaves the other half live and
// produces a REDUCED-divergence number that reads like progress and proves
// nothing (the same trap this file's own line-452 comment names for the
// join-hold fix: "expected to REDUCE, not necessarily eliminate").
//
//   (a) STEP SIZE: `client.tick(.., dt)` gets a FIXED 1/TPS instead of
//       `driver_dt(&clock)`, so the client's simulation advances in equal steps
//       rather than wall-sized ones.
//   (b) PACING: the loop waits for the server-derived sim clock to advance one
//       tick instead of sleeping to a wall deadline.
//
// Default OFF: unset, every call is byte-identical to before and every banked
// baseline stands.
//
// ★ SCOPED HONESTLY: a tick-driven driver is NOT what a real player runs. This
// arm measures whether the ENGINE can hold a fingerprint when its client stops
// being wall-paced -- it does not claim the shipped client behaves this way.
fn tick_driven_driver() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        let on = std::env::var_os("BASTION_TICK_DRIVEN_DRIVER").is_some();
        // ★★ KNOWN BROKEN, TWICE MEASURED — REFUSES rather than yielding data.
        //
        // Attempt 1 (sleep until server Time advances): starved its own message
        // pump — Time only advances in the client when the client is ticked.
        // Attempt 2 (PUMP while waiting): fixed the starvation (0 pace-exhaustion
        // warns) and the leg STILL died at the join — 86 census emits vs 11,317,
        // anchor-origin guard fired.
        //
        // THE MECHANISM, and it is the one I briefly talked myself out of:
        // this file's budgets are SPIN COUNTS that silently encode WALL-TIME
        // assumptions. `POS_WAIT_TICKS = TPS * 15` means "15 seconds" only
        // because each spin sleeps ~1/TPS of WALL time. Slave the spin to the
        // SERVER's clock and 450 spins no longer span 15 wall seconds, so a
        // network-timed arrival (the `Pos` component) misses a budget that was
        // never really counting spins.
        //
        // Every such budget (POS_WAIT_TICKS, ACK_SPIN_CAP, WAIT_SPIN_CAP_
        // MULTIPLIER, ...) would have to be re-expressed in sim time. THAT is
        // why option 1 is a restructure and not a flag — confirmed by two
        // attempts, not asserted.
        assert!(
            !on,
            "BASTION_TICK_DRIVEN_DRIVER is KNOWN BROKEN (twice measured) and refuses              to run: this driver's spin-count budgets encode WALL-TIME assumptions              (POS_WAIT_TICKS = TPS*15 means 15s only because each spin sleeps 1/TPS              of wall time). Slaving spins to the server clock makes network-timed              arrivals miss those budgets — the join fails with the anchor-origin              guard. A correct arm must re-express every budget in SIM TIME. See              BAR2-CAUSE-IS-STRUCTURAL.md."
        );
        on
    })
}

/// Fixed step under the arm; wall-derived otherwise.
fn driver_dt(clock: &Clock) -> std::time::Duration {
    if tick_driven_driver() {
        std::time::Duration::from_secs_f64(1.0 / TPS as f64)
    } else {
        // ★ NOT `driver_dt(&clock)`. A blanket text replace rewrote this line
        // into a self-call, making the DEFAULT path infinitely recursive — and
        // `cargo check` passed, because unbounded recursion is not a compile
        // error. Only running it would have found it. Scripted rewrites destroy
        // meaning silently; this is the second time in one edit that the same
        // replace corrupted the very helper it was introducing.
        clock.game_dt()
    }
}

/// Last observed server sim time, in microseconds, for the tick-driven pace.
static LAST_SIM_US: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Advance one step: wait on the SERVER's clock under the arm, the wall clock
/// otherwise. Falls back to the wall pace if sim time does not advance within a
/// bounded number of polls, so a stalled server cannot hang the driver
/// silently -- and says so, because a silent fallback would make a wall-paced
/// run indistinguishable from a tick-driven one.
fn driver_pace(clock: &mut Clock, client: &mut Client) {
    use std::sync::atomic::Ordering;
    if !tick_driven_driver() {
        clock.tick();
        return;
    }
    // ★ PUMP WHILE WAITING. The first version SLEPT until the server-derived
    // `Time` advanced -- but `Time` only advances IN THE CLIENT when the client
    // is ticked and the network pumped, so it waited on a value its own waiting
    // prevented from arriving (measured: one leg died on the anchor guard with
    // 128 census emits vs 11,317; another fell back 326 times).
    //
    // Ticking inside the wait breaks that circularity: the loop advances the
    // client until the SERVER's clock has moved one tick, so a driver spin is
    // slaved to a server tick instead of to a wall deadline. Spin-count budgets
    // (ACK_SPIN_CAP, ...) still count OUTER spins and simply cover more client
    // ticks each -- more generous, not broken.
    let now_us = |c: &Client| (c.state().ecs().read_resource::<Time>().0 * 1_000_000.0) as u64;
    let start = LAST_SIM_US.load(Ordering::Relaxed);
    let t0 = now_us(client);
    if start == 0 {
        LAST_SIM_US.store(t0, Ordering::Relaxed);
        return;
    }
    let target = start + (1_000_000.0 / TPS as f64) as u64;
    let dt = std::time::Duration::from_secs_f64(1.0 / TPS as f64);
    for pump in 0..600u32 {
        if now_us(client) >= target {
            LAST_SIM_US.store(now_us(client), Ordering::Relaxed);
            return;
        }
        let _ = client.tick(comp::ControllerInputs::default(), dt);
        client.cleanup();
        if pump == 599 {
            tracing::warn!(
                start_us = start,
                target_us = target,
                "bastion: TICK-DRIVEN pace exhausted 600 pumps without the server                  clock advancing; this leg is NOT tick-driven and must be scored VOID"
            );
            LAST_SIM_US.store(now_us(client), Ordering::Relaxed);
            return;
        }
    }
}

fn main() {
    // ★★ WITHOUT THIS, EVERY `tracing::*` EMIT IN `client/src/lib.rs` IS
    // INVISIBLE (2026-08-19). This binary had no subscriber, so 14 client-side
    // diagnostics went nowhere — including the row89 chunk-request witness
    // whose own comment says it exists "so the arm can declare itself VOID
    // instead". It could never have done that. An instrument with no consumer
    // is not an instrument.
    //
    // stderr, because the runner already captures it to driverout-<tag>.log and
    // the playtest's own `log.log()` file must stay parseable.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .try_init();
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
        match client.tick(comp::ControllerInputs::default(), driver_dt(&clock)) {
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
        driver_pace(&mut clock, &mut client);
        ticks += 1;

        // #89 (1bcd1d251c): the JOIN is the session boundary where
        // nondeterminism enters. This loop is paced by `clock.tick()` -- WALL
        // CLOCK -- and fires `request_character` on whichever spin the
        // character list happens to arrive on, which is network/disk timed.
        // The resulting `InitializeCharacterEvent` therefore lands on a
        // DIFFERENT SERVER TICK between runs, and every downstream count
        // inherits the offset. Measured: the view-distance grant is the FIRST
        // divergence in 25 of 42 same-seed twin pairs, beating all fourteen
        // colony families combined.
        //
        // The hold removes the ARRIVAL JITTER from the join: instead of
        // "request as soon as the list lands", request at a FIXED spin count
        // that is comfortably past any list-load. Default 0 = today's
        // behaviour, byte-identical, so every banked baseline stands until an
        // arm opts in.
        //
        // ★ SCOPED HONESTLY: this pins the CLIENT side of the boundary. If the
        // server is running uncapped, the server tick reached by spin N still
        // varies with server speed -- so this is expected to REDUCE, not
        // necessarily eliminate, join-tick variance. Which it does is the
        // measurement, not an assumption.
        let join_hold: u64 = std::env::var("BASTION_JOIN_HOLD_TICKS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if !requested_join && ticks >= join_hold {
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

    // bastion: WAIT ON THE CONDITION, NOT ON A CLOCK.
    //
    // This was a fixed `TPS * 2` spin followed by ONE read of `Pos` with a
    // silent `unwrap_or_else(|| Vec3::zero())`. When the component had not
    // arrived inside that fixed window the driver anchored the god-camera at
    // the WORLD ORIGIN: the server then generated and streamed a 7x7 block of
    // terrain at chunk (0, 0) that nothing ever looked at, while the colony
    // itself got only its baseline chunks.
    //
    // Measured on the banked corpus: this fired in 41 of 68 runs, and one twin
    // pair SPLIT across it -- twin1 anchored at the colony, twin2 at the
    // origin, a 154-chunk difference in promoted terrain that had been read as
    // engine nondeterminism. The two outcomes are indistinguishable in a
    // scored log, so the corpus carried two populations under one label.
    //
    // There is no neutral position to fall back to: the origin is not a
    // degraded answer, it is a DIFFERENT EXPERIMENT. So the failure path
    // refuses rather than guesses, and exits non-zero so a run that could not
    // be anchored can never be scored as one that was.
    const POS_WAIT_TICKS: u64 = TPS * 15;
    // ★ PLANT, for the red-demonstration this fix owes. The race fires only
    // when `Pos` arrives later than the old fixed `TPS * 2` spin, which depends
    // on server boot speed -- on fresh hosts it arrived at tick 47-48 in 6 of 6
    // runs, so a whole VM fan demonstrated nothing and was scored VOID rather
    // than green. Waiting for the condition to occur by luck is not a test.
    //
    // `BASTION_PLANT_POS_DELAY=<ticks>` withholds `Pos` from the driver for the
    // first N ticks -- the stage this fix protects -- so a value above TPS*2
    // forces exactly the case the old code got wrong. INERT when unset.
    let plant_pos_delay: u64 = std::env::var("BASTION_PLANT_POS_DELAY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if plant_pos_delay > 0 {
        log.log(&format!(
            "PLANT ACTIVE: BASTION_PLANT_POS_DELAY={plant_pos_delay} -- Pos is withheld from the \
             driver for the first {plant_pos_delay} ticks. This run is PLANTED and must never be \
             scored as a live measurement."
        ));
    }
    let mut warmed: u64 = 0;
    let anchored = loop {
        if warmed >= plant_pos_delay {
            if let Some(p) = own_pos(&client) {
                break Some(p);
            }
        }
        if warmed >= POS_WAIT_TICKS {
            break None;
        }
        let _ = client.tick(comp::ControllerInputs::default(), driver_dt(&clock));
        client.cleanup();
        driver_pace(&mut clock, &mut client);
        warmed += 1;
    };
    let Some(mut current_pos) = anchored else {
        log.log(&format!(
            "VOID: no Pos component after {POS_WAIT_TICKS} ticks ({}s) -- REFUSING to anchor at the \
             world origin. Anchoring at Vec3::zero() streams terrain at chunk (0, 0) and silently \
             makes this run a different condition from one anchored at the colony.",
            POS_WAIT_TICKS / TPS
        ));
        std::process::exit(3);
    };
    // SELF-SCORING WITNESS. The old code spun a fixed `TPS * 2` and then read
    // once, so it had `Pos` exactly when it arrived within that window. That
    // makes `warmed > TPS * 2` the precise counterfactual: those are the runs
    // the old driver would have anchored at the world origin. Emitting it here
    // means every future run carries its own red-demonstration and the defect
    // rate is countable without re-deriving it from geometry.
    let old_would_have_failed = warmed > TPS * 2;
    log.log(&format!(
        "anchor precondition: Pos arrived after {warmed} ticks (old fixed spin was {}, ceiling {POS_WAIT_TICKS}); \
         OLD BEHAVIOUR WOULD HAVE {}",
        TPS * 2,
        if old_would_have_failed {
            "ANCHORED AT THE WORLD ORIGIN -- this run is one the old driver got wrong"
        } else {
            "matched this run"
        }
    ));
    // Terrain warm-up, unchanged in length: Pos arriving early must not shorten
    // the settle the painting path depends on.
    for _ in warmed..(TPS * 2) {
        let _ = client.tick(comp::ControllerInputs::default(), driver_dt(&clock));
        client.cleanup();
        driver_pace(&mut clock, &mut client);
    }
    // Re-read after the settle so the anchor is the SETTLED position, exactly as
    // before -- the wait above changed only whether `Pos` exists, not when it is
    // sampled. Breaking out of the wait early must not sample a falling player.
    if let Some(p) = own_pos(&client) {
        current_pos = p;
    }
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
                    match client.tick(comp::ControllerInputs::default(), driver_dt(&clock)) {
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
                    driver_pace(&mut clock, &mut client);
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
                // ACK BARRIER: return only once the server's designation
                // revision has actually moved. `bastion_designations_rev`
                // bumps on any change (its own doc comment), so it is the
                // acknowledgement -- and nothing consumed it as one before.
                //
                // Sampling 87ms after SENDING a designation read the OLD
                // count and looked exactly like a refusal. A guess that is
                // too short does not error; it reads as a result.
                let rev_before = client.bastion_designations_rev();
                client.bastion_place_designation(region, kind, None);
                log.log(&format!("sent BastionPlaceDesignation kind={kind:?} region={region:?}"));
                let acked = await_designation_rev(&mut client, &mut clock, rev_before);
                log.log(&format!(
                    "designate ACK rev {rev_before} -> {} ({})",
                    client.bastion_designations_rev(),
                    if acked { "observed" } else { "ACK-TIMEOUT" }
                ));
            },
            ScriptCmd::Cancel(region) => {
                client.bastion_cancel_designation(region);
                log.log(&format!("sent BastionCancelDesignation region={region:?}"));
            },
            ScriptCmd::InspectCell(pos) => {
                client.bastion_inspect_request(BastionInspectTarget::Cell(pos));
                // One tick round-trip to receive the echoed reply.
                let _ = client.tick(comp::ControllerInputs::default(), driver_dt(&clock));
                client.cleanup();
                driver_pace(&mut clock, &mut client);
                log.log(&format!("inspect_cell {pos:?} -> {:?}", client.bastion_inspect()));
            },
            ScriptCmd::InspectColonists => {
                // ARC 2 item 9. The inspector's ENTITY arm is what the HUD
                // uses, and nothing automated had ever exercised it -- the
                // driver could only inspect CELLS. `Colonist` is not a
                // network-synced component, so the client cannot enumerate
                // colonists directly; it does exactly what the HUD does --
                // sends a Uid and lets the SERVER decide whether anything
                // Bastion-tracked is there. That keeps this on the shipping
                // path rather than inventing a second resolver.
                let uids: Vec<common::uid::Uid> = {
                    use specs::Join;
                    let ecs = client.state().ecs();
                    let uid_store = ecs.read_storage::<common::uid::Uid>();
                    let mut v: Vec<common::uid::Uid> = (&uid_store).join().copied().collect();
                    // Deterministic order so two samples are comparable and
                    // the log is diffable between legs.
                    v.sort_by_key(|u| u.0);
                    v
                };
                let mut found = 0usize;
                for uid in uids {
                    client.bastion_inspect_request(BastionInspectTarget::Entity(uid));
                    // THE REPLY MUST ANSWER THIS REQUEST. `bastion_inspect()`
                    // is a single latest-reply slot, so reading it after a
                    // fixed one-tick wait returns whatever happened to be
                    // there -- which, when the round trip takes longer than a
                    // tick, is the PREVIOUS uid's payload. That produced a
                    // convincing lie on the first run: distinct uids reporting
                    // the same colonist and one uid reporting two different
                    // colonists across samples, which reads exactly like a
                    // server resolver ignoring its target. The reply carries
                    // its OWN target; match on it instead of trusting arrival
                    // order.
                    let mut matched = false;
                    for _ in 0..60 {
                        let _ = client.tick(comp::ControllerInputs::default(), driver_dt(&clock));
                        client.cleanup();
                        driver_pace(&mut clock, &mut client);
                        if let Some((BastionInspectTarget::Entity(got), _)) =
                            client.bastion_inspect()
                            && *got == uid
                        {
                            matched = true;
                            break;
                        }
                    }
                    if !matched {
                        // Silence here would be indistinguishable from "not a
                        // colonist", so it is logged as its own outcome.
                        log.log(&format!("INSPECT uid={} NO-REPLY-MATCHED", uid.0));
                        continue;
                    }
                    if let Some((_, Some(common::comp::bastion::BastionInspectKind::Colonist(p)))) =
                        client.bastion_inspect()
                    {
                        found += 1;
                        // One line per colonist, key=value so a scorer can
                        // diff FIELDS between two samples rather than eyeball
                        // a Debug blob.
                        log.log(&format!(
                            // `ownership` and `mood_explanation` are logged as
                            // PRESENT/ABSENT rather than in full: the server
                            // builds both as Some(..), so the question this
                            // line has to answer is whether they arrive at
                            // all. Their absence from this format is what let
                            // me report "they were None" about fields I had
                            // never looked at -- silence in a log is evidence
                            // about the LOGGER, not about the field.
                            // ITEM 13: `health` is printed as the raw Option debug
                            // (`Some(0.7143)` / `None`) rather than flattened to a
                            // number, for the reason stated on the field itself: a
                            // missing component must not be readable as 0.0/dead.
                            //
                            // ★ `conscientious` and `neurotic` are added here having
                            // ALREADY been on the wire — the payload carries 14
                            // fields and this line printed 12. That gap is the exact
                            // trap the comment above warns about, and I walked into
                            // it: I read THIS FORMAT STRING to conclude what the
                            // payload contained. The conclusion happened to be right
                            // for `health`; it was wrong for these two.
                            "INSPECT uid={} name={} hunger={:.4} rest={:.4} recreation={:.4} \
                             energy={:.4} health={:?} mood={:.4} drive={:?} scores={:?} \
                             activity={:?} status={:?} ownership={} mood_expl={} \
                             consc={} neur={} skills={:?} traits={:?}                              desires={:?} bravery={:.2}",
                            uid.0,
                            p.name,
                            p.hunger,
                            p.rest,
                            p.recreation,
                            p.energy,
                            p.health,
                            p.mood,
                            p.drive,
                            p.last_scores,
                            p.activity,
                            p.status,
                            if p.ownership.is_some() { "SOME" } else { "NONE" },
                            if p.mood_explanation.is_some() {
                                "SOME"
                            } else {
                                "NONE"
                            },
                            p.conscientious,
                            p.neurotic,
                            p.skills,
                            p.traits,
                            p.desires,
                            p.guard_bravery
                        ));
                        // ITEM 23 (morale events): the THOUGHT breakdown in
                        // full. mood_expl=SOME proved ARRIVAL; scoring item
                        // 23's bar ("a planted event produces a mood step")
                        // needs the CONTENTS -- which thought, what magnitude.
                        // One line per colonist, empty printed as thoughts=0,
                        // so no-thoughts and not-requested cannot conflate.
                        if let Some(me) = &p.mood_explanation {
                            log.log(&format!(
                                "MOODX uid={} total={:.4} thoughts={} {:?}",
                                uid.0,
                                me.total_mood,
                                me.thoughts.len(),
                                me.thoughts
                                    .iter()
                                    .map(|t| (t.thought_id, t.base_magnitude))
                                    .collect::<Vec<_>>()
                            ));
                        }
                    }
                }
                log.log(&format!("inspect_colonists -> {found} colonist payload(s)"));
            },
            ScriptCmd::CountItems => {
                use specs::Join;
                let ecs = client.state().ecs();
                let items = ecs.read_storage::<comp::PickupItem>();
                let positions = ecs.read_storage::<comp::Pos>();
                let mut lines: Vec<String> = Vec::new();
                for (item, pos) in (&items, &positions).join() {
                    lines.push(format!(
                        "{}@({:.1},{:.1},{:.1})",
                        item.item()
                            .item_definition_id()
                            .itemdef_id()
                            .unwrap_or("?"),
                        pos.0.x,
                        pos.0.y,
                        pos.0.z
                    ));
                }
                let n = lines.len();
                drop(items);
                drop(positions);
                log.log(&format!("ITEMS count={n} {}", lines.join(" ")));
            },
            // ARC 2 item 12: chronicle round trip for EVERY colonist uid the
            // client can see. Per the prereg: print enabled and truncated
            // ALWAYS -- an empty row list with enabled=false says nothing
            // about the entity, and the logger omitting the flag would
            // recreate the exact conflation the payload exists to prevent.
            ScriptCmd::InspectChronicle => {
                let uids: Vec<common::uid::Uid> = {
                    use specs::Join;
                    let state = client.state();
                    let ecs = state.ecs();
                    let colonists = ecs.read_storage::<comp::Colonist>();
                    let uid_storage = ecs.read_storage::<common::uid::Uid>();
                    (&colonists, &uid_storage)
                        .join()
                        .map(|(_, uid)| *uid)
                        .collect()
                };
                log.log(&format!("CHRONICLE requesting {} colonists", uids.len()));
                for uid in uids {
                    client.bastion_inspect_request(BastionInspectTarget::Chronicle(uid));
                    let mut got = None;
                    for _ in 0..60 {
                        let _ =
                            client.tick(comp::ControllerInputs::default(), driver_dt(&clock));
                        client.cleanup();
                        driver_pace(&mut clock, &mut client);
                        if let Some((BastionInspectTarget::Chronicle(u), payload)) =
                            client.bastion_inspect()
                            && *u == uid
                        {
                            got = Some(payload.clone());
                            break;
                        }
                    }
                    match got {
                        Some(Some(common::comp::bastion::BastionInspectKind::Chronicle(c))) => {
                            log.log(&format!(
                                "CHRONICLE uid={} enabled={} truncated={} rows={}",
                                uid.0.get(),
                                c.enabled,
                                c.truncated,
                                c.events.len()
                            ));
                            for row in &c.events {
                                log.log(&format!(
                                    "CHRONICLE-ROW uid={} tick={} kind={} actor={:?}",
                                    uid.0.get(),
                                    row.tick,
                                    row.kind,
                                    row.actor
                                ));
                            }
                        },
                        Some(other) => {
                            log.log(&format!("CHRONICLE uid={} WRONG-KIND {other:?}", uid.0.get()))
                        },
                        None => log.log(&format!("CHRONICLE uid={} NO-REPLY", uid.0.get())),
                    }
                }
            },
            ScriptCmd::InspectColony => {
                // ARC 2 item 10. Same protocol, same reply-matching
                // discipline as inspect_colonists -- the single-slot API
                // makes "read after one tick" quietly wrong.
                client.bastion_inspect_request(BastionInspectTarget::Colony);
                let mut got = None;
                for _ in 0..60 {
                    let _ = client.tick(comp::ControllerInputs::default(), driver_dt(&clock));
                    client.cleanup();
                    driver_pace(&mut clock, &mut client);
                    if let Some((BastionInspectTarget::Colony, payload)) = client.bastion_inspect()
                    {
                        got = Some(payload.clone());
                        break;
                    }
                }
                match got {
                    Some(Some(common::comp::bastion::BastionInspectKind::Colony(c))) => {
                        log.log(&format!(
                            // blocked_materials printed ALWAYS: a field the
                            // logger omits is indistinguishable from a field
                            // the server never sent, and this session has hit
                            // that exact shape three times (ownership,
                            // mood_explanation, and this one).
                            "COLONY tick={} colonists={} food_stock={} jobs_total={} jobs_claimed={} jobs_unreachable={} designations={} blocked_materials={}",
                            c.tick,
                            c.colonists,
                            c.food_stock,
                            c.jobs_total,
                            c.jobs_claimed,
                            c.jobs_unreachable,
                            c.designations,
                            c.jobs_blocked_materials
                        ));
                    },
                    Some(other) => log.log(&format!("COLONY WRONG-KIND {other:?}")),
                    None => log.log("COLONY NO-REPLY-MATCHED"),
                }
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
                // ★ AN UNLOADED CELL IS NOT AIR. Both scans below read the
                // terrain with `.map(|b| b.is_filled()).unwrap_or(false)`, and
                // `TerrainGrid::get` returns Err for a chunk that is not
                // loaded. So "not filled" silently covered two different
                // worlds: a cell the client has SEEN and found empty, and a
                // cell the client has NEVER SEEN. The first is evidence; the
                // second is absence of evidence, and they rendered identically
                // in every one of the 94 banked logs carrying a survey.
                //
                // The consequence is not cosmetic: an unloaded column reports
                // "no surface in range", and an unloaded run beneath a surface
                // counts toward `empty_run`, which is what promotes a column to
                // an OVERHANG CANDIDATE. Unloaded terrain could therefore
                // manufacture the very finding the survey exists to make.
                //
                // Counted, not fixed by clamping: the survey still reports what
                // it saw, but a reader can now tell which kind of nothing it
                // was. [[null-needs-a-couldnt-happen-witness]]
                let mut columns_unloaded = 0;
                let mut cells_unloaded = 0;
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        columns_scanned += 1;
                        let mut surface = None;
                        let mut z = ztop;
                        let mut column_saw_unloaded = false;
                        while z >= zbot {
                            match terrain.get(Vec3::new(x, y, z)) {
                                Ok(b) => {
                                    if b.is_filled() {
                                        surface = Some(z);
                                        break;
                                    }
                                },
                                Err(_) => {
                                    cells_unloaded += 1;
                                    column_saw_unloaded = true;
                                },
                            }
                            z -= 1;
                        }
                        if column_saw_unloaded {
                            columns_unloaded += 1;
                        }
                        let Some(sz) = surface else {
                            columns_no_surface += 1;
                            continue;
                        };
                        let mut empty_run = 0;
                        let mut run_unloaded = 0;
                        let mut zz = sz - 1;
                        while zz >= zbot {
                            match terrain.get(Vec3::new(x, y, zz)) {
                                Ok(b) if b.is_filled() => break,
                                Ok(_) => {},
                                Err(_) => {
                                    run_unloaded += 1;
                                    cells_unloaded += 1;
                                },
                            }
                            empty_run += 1;
                            zz -= 1;
                        }
                        if empty_run >= gap {
                            // Carry the unloaded count INTO the candidate, so a
                            // candidate built out of unseen cells is visible as
                            // such at the point it is quoted, not only in a
                            // summary line further down.
                            candidates.push((x, y, sz, empty_run, run_unloaded));
                        }
                    }
                }
                drop(terrain);
                let tainted = candidates.iter().filter(|c| c.4 > 0).count();
                log.log(&format!(
                    "survey [{x0},{y0}]-[{x1},{y1}] z[{zbot},{ztop}] gap>={gap}: \
                     {columns_scanned} columns, {columns_no_surface} with no surface \
                     in range, {} overhang candidates: {candidates:?} \
                     | UNLOADED: {columns_unloaded} columns touched unloaded terrain, \
                     {cells_unloaded} cells unseen, {tainted} candidates rest on unseen cells{}",
                    candidates.len(),
                    if columns_unloaded > 0 || tainted > 0 {
                        " <- READ THE UNLOADED FIGURES BEFORE THE CANDIDATES"
                    } else {
                        " (every cell in range was actually observed)"
                    }
                ));
            },
            ScriptCmd::Note(text) => {
                log.log(&format!("[note] {text}"));
            },
            ScriptCmd::Cmd(name, cmd_args) => {
                log.log(&format!("sent chat command /{name} {cmd_args:?}"));
                let watch_items = name == "dropall";
                client.send_command(name, cmd_args);
                // ACK BARRIER: spin until the server's chat reply for THIS
                // command arrives, instead of a single speculative tick.
                //
                // One tick was not enough: `give_item` was acknowledged 660ms
                // AFTER the following `dropall` had already been sent, so the
                // drop emptied an inventory that had not yet received the
                // items -- and the run reported "the drop produced nothing".
                // DROP WITNESS row: `dropall` emits NO chat reply, so the
                // chat barrier below can never observe it. Its real effect is
                // ENTITIES appearing, and PickupItem is synced -- so for that
                // one command the barrier watches the item count instead.
                let items_before = count_pickup_items(&client);
                let mut acked = false;
                for _ in 0..ACK_SPIN_CAP {
                    if watch_items && count_pickup_items(&client) > items_before {
                        acked = true;
                        log.log(&format!(
                            "dropall witnessed: items {items_before} -> {}",
                            count_pickup_items(&client)
                        ));
                        break;
                    }
                    match client.tick(comp::ControllerInputs::default(), driver_dt(&clock)) {
                        Ok(events) => {
                            for event in events {
                                if let Event::Chat(m) = &event {
                                    log.log(&format!("[chat] {m:?}"));
                                    // Any CommandInfo/CommandError is the
                                    // server having PROCESSED the command --
                                    // success and refusal both count, because
                                    // the barrier is about ordering, not
                                    // about the command succeeding.
                                    if matches!(
                                        m.chat_type,
                                        comp::ChatType::CommandInfo
                                            | comp::ChatType::CommandError
                                    ) {
                                        acked = true;
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            log.log(&format!("tick error after cmd: {e:?}"));
                        },
                    }
                    client.cleanup();
                    driver_pace(&mut clock, &mut client);
                    if acked {
                        break;
                    }
                }
                log.log(&format!(
                    "cmd ACK {}",
                    if acked { "observed" } else { "ACK-TIMEOUT" }
                ));
                match Ok::<Vec<Event>, ()>(Vec::new()) {
                    Ok(_events) => {},
                    Err(()) => {},
                }
                client.cleanup();
                driver_pace(&mut clock, &mut client);
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
                "inspect_colonists" => "inspect_colonists".into(),
                "inspect_colony" => "inspect_colony".into(),
                "count_items" => "count_items".into(),
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
