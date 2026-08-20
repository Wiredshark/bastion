//! W3 renderer-bench ack bot (`readme/renderer-bench/W3-LAUNCH-PACKET.md`):
//! a headless SPECTATOR client for the integrated replication proof. It
//! connects, spectates at the arena origin, signals readiness, and then
//! just ticks — the ack path itself lives in `veloren-client` (gated on
//! `BASTION_RENDERER_BENCH_ACK=1`, which the runner must set for this
//! process): every announce the server sends comes back as a projection
//! ack, and the server's tape records them. The TAPE is the evidence;
//! this binary only has to exist, be present, and stay alive.
//!
//! Usage: rbench_ackbot <hostname> <username> <x> <y> <z> <max_seconds>
//!   x/y/z: arena origin in BLOCKS (the runner derives it from the
//!   fixture's arena_origin_mm / 1000).
//!   The username must already hold the admin role on the target server
//!   (spectate is moderator-gated): `server-cli admin add <username> admin`.

use common::{ViewDistances, clock::Clock, comp};
use std::{sync::Arc, time::Duration};
use tokio::runtime::Runtime;
use vek::Vec3;
use veloren_client::{Client, ClientType, addr::ConnectionArgs};

const TPS: u64 = 30;

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: rbench_ackbot <hostname> <username> <x> <y> <z> <max_seconds>";
    let hostname = args.next().expect(usage);
    let username = args.next().expect(usage);
    let x: f32 = args.next().expect(usage).parse().expect("x: f32");
    let y: f32 = args.next().expect(usage).parse().expect("y: f32");
    let z: f32 = args.next().expect(usage).parse().expect("z: f32");
    let max_seconds: u64 = args.next().expect(usage).parse().expect("max_seconds: u64");

    // PRECONDITION above result (house law): the ack gate must be ON in
    // THIS process or the whole leg silently proves nothing.
    let ack_gate = std::env::var("BASTION_RENDERER_BENCH_ACK").unwrap_or_default();
    println!("PRECONDITION ackbot build={:08x} ack_gate={ack_gate}", *common::util::GIT_HASH);
    assert_eq!(ack_gate, "1", "BASTION_RENDERER_BENCH_ACK=1 must be set for the ackbot");

    let runtime = Arc::new(Runtime::new().unwrap());
    let addr = ConnectionArgs::Tcp { prefer_ipv6: false, hostname };
    let mut client = runtime
        .block_on(Client::new(
            addr,
            Arc::clone(&runtime),
            &mut None,
            &username,
            "",
            None,
            |_| true,
            &|stage| println!("[init] {stage:?}"),
            |_| {},
            Default::default(),
            ClientType::Game,
        ))
        .expect("ackbot: failed to connect");
    println!("ackbot: connected");

    client.request_spectate(ViewDistances { terrain: 6, entity: 6 });

    let mut clock = Clock::new(Duration::from_secs_f64(1.0 / TPS as f64));
    let mut ready_sent = false;
    let max_ticks = max_seconds * TPS;
    for tick in 0..max_ticks {
        if let Err(e) = client.tick(comp::ControllerInputs::default(), clock.game_dt()) {
            println!("ackbot: tick error {e:?} — leg VOID");
            std::process::exit(2);
        }
        if !ready_sent && matches!(client.presence(), Some(comp::PresenceKind::Spectator)) {
            client.renderer_bench_ready();
            client.spectate_position(Vec3::new(x, y, z));
            ready_sent = true;
            println!("ackbot: SPECTATING at ({x},{y},{z}) tick={tick} — readiness sent");
        }
        client.cleanup();
        clock.tick();
    }
    if !ready_sent {
        println!("ackbot: never reached spectator presence — leg VOID");
        std::process::exit(2);
    }
    println!("ackbot: done ({max_seconds}s)");
}
