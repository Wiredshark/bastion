use crate::{chunk_serialize::SerializedChunk, client::Client, metrics::NetworkRequestMetrics};

use common_ecs::{Job, Origin, Phase, System};
use specs::{ReadExpect, ReadStorage};

/// This system will handle sending terrain to clients by
/// collecting chunks that need to be send for a single generation run and then
/// trigger a SlowJob for serialisation.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Client>,
        ReadExpect<'a, NetworkRequestMetrics>,
        ReadExpect<'a, crossbeam_channel::Receiver<SerializedChunk>>,
    );

    const NAME: &'static str = "chunk_send";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (clients, network_metrics, chunk_receiver): Self::SystemData) {
        let mut lossy = 0u64;
        let mut lossless = 0u64;
        // ★ CANONICAL SEND ORDER (BASTION_DETERMINISTIC_CHUNK_SEND=1).
        //
        // Serialization runs in `SlowJob`s batched 10 per job, each sending its
        // batch into this channel ON COMPLETION -- so `try_iter()` yields
        // chunks in THREAD-SCHEDULING ORDER and every run delivers terrain to
        // clients in a different sequence. That is the certification's bar 2:
        // membership is pinned by the deterministic barrier (measured identical
        // on 30/30 twin pairs) while the per-tick schedule differed on 31/31,
        // because the barrier pins WHICH chunks are released and not WHEN each
        // arrives. Its own doc says so -- "membership is what this pins".
        //
        // Draining the whole tick's chunks and emitting them sorted by key is
        // the same shape as `canonical_haul_pickup_order`, which already exists
        // (and is tested) for exactly this reason on the pickup side.
        //
        // Env-gated and DEFAULT OFF: this changes the live send path, so every
        // banked run stays byte-reproducible until an A/B says otherwise.
        let ordered = std::env::var_os("BASTION_DETERMINISTIC_CHUNK_SEND").is_some();
        let mut drained: Vec<crate::chunk_serialize::SerializedChunk> = Vec::new();
        if ordered {
            drained.extend(chunk_receiver.try_iter());
            // Sort by (x, y) -- a TOTAL order on the key, so the sequence is a
            // pure function of the chunk SET and not of which thread finished
            // first. `sort_by_key` is stable, and keys are unique within a
            // tick's drain, so no tie-break is needed.
            drained.sort_by_key(|sc| (sc.key.x, sc.key.y));
            // INFO, not debug: this is the arm's PRECONDITION witness. The
            // server logs at INFO, so a debug! line would be invisible and
            // "the ordering did not help" would render identically to "the
            // ordering never ran" -- the exact failure that voided two fans
            // earlier today.
            tracing::info!(n = drained.len(), "bastion: chunk send ORDERED by key");
        }
        for sc in drained.into_iter().chain(chunk_receiver.try_iter()) {
            for recipient in sc.recipients {
                if let Some(client) = clients.get(recipient)
                    && client.send_prepared(&sc.msg).is_err()
                {
                    if sc.lossy_compression {
                        lossy += 1;
                    } else {
                        lossless += 1;
                    }
                }
            }
        }
        network_metrics.chunks_served_lossy.inc_by(lossy);
        network_metrics.chunks_served_lossless.inc_by(lossless);
    }
}
