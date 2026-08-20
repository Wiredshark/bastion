//! W3 renderer-bench: drains the bench system's announce outbox to every
//! in-game client (spectators included — they carry `Presence` too).
//! Lives in the server crate because `bastion-server` cannot see `Client`.
//! See `readme/renderer-bench/W3-LAUNCH-PACKET.md`.

use crate::client::Client;
use common::{comp::Presence, renderer_bench::RendererBenchNetOutbox};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::ServerGeneral;
use specs::{Join, ReadStorage, Write};

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Write<'a, RendererBenchNetOutbox>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Presence>,
    );

    const NAME: &'static str = "renderer_bench_net";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (mut outbox, clients, presences): Self::SystemData) {
        if outbox.announces.is_empty() {
            return;
        }
        for ann in outbox.announces.drain(..) {
            let mut msg = Some(ServerGeneral::RendererBenchFrame(ann));
            let mut lazy_msg = None;
            for (client, _) in (&clients, &presences).join() {
                if let Some(msg) = msg.take() {
                    lazy_msg = Some(client.prepare(msg));
                }
                if let Some(msg) = lazy_msg.as_ref() {
                    let _ = client.send_prepared(msg);
                }
            }
        }
    }
}
