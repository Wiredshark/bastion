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
use veloren_client::{Client, ClientType, WorldExt, addr::ConnectionArgs};

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
    let target = Vec3::new(x, y, z);
    // The client's presence() flips to Spectator OPTIMISTICALLY inside
    // request_spectate — server confirmation is Event::StartSpectate.
    // Messages sent before that are dropped by the server's not-in-game
    // guard, which is exactly how the first two legs parked at world
    // spawn while reporting local success.
    let mut server_ingame_at: Option<u64> = None;
    let mut ready_sent = false;
    let max_ticks = max_seconds * TPS;
    for tick in 0..max_ticks {
        let events = match client.tick(comp::ControllerInputs::default(), clock.game_dt()) {
            Ok(ev) => ev,
            Err(e) => {
                println!("ackbot: tick error {e:?} — leg VOID");
                std::process::exit(2);
            },
        };
        for ev in events {
            if let veloren_client::Event::StartSpectate(spawn) = ev {
                println!("ackbot: server CONFIRMED spectate (spawn={spawn:?}) tick={tick}");
                server_ingame_at = Some(tick);
            }
        }
        if let Some(t0) = server_ingame_at {
            // Latest-state and cheap: keep re-sending for the whole run
            // so the server-side position can never silently stay stale.
            if (tick - t0) % 15 == 0 {
                client.spectate_position(target);
            }
            // Readiness starts the run: grant a grace window after the
            // confirmed transition so the position + region subscription
            // have settled server-side first.
            if !ready_sent && tick - t0 >= 60 {
                client.renderer_bench_ready();
                ready_sent = true;
                println!("ackbot: readiness sent tick={tick} (t0={t0})");
            }
        }
        // Discriminating census: bench entities vs ANY synced entity.
        // bench=0 with pos>0 means sync works but the component does not
        // apply; pos=0 means no entity sync reaches this client at all.
        if ready_sent && tick % 30 == 0 {
            use specs::Join;
            let ecs = client.state().ecs();
            let bench = ecs
                .read_storage::<comp::bastion::RendererBenchEntityId>()
                .join()
                .count();
            let pos_entities = ecs.read_storage::<comp::Pos>().join().count();
            println!(
                "ackbot: census tick={tick} bench={bench} pos_entities={pos_entities} my_pos={:?}",
                client.position()
            );
        }
        client.cleanup();
        clock.tick();
    }
    if !ready_sent {
        println!(
            "ackbot: server never confirmed spectate (confirmed_at={server_ingame_at:?}, \
             local_pos={:?}) — leg VOID",
            client.position()
        );
        std::process::exit(2);
    }
    println!("ackbot: done ({max_seconds}s)");
}
