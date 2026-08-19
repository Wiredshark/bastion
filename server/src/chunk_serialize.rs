use crate::client::PreparedMsg;
use specs::Entity;
use vek::Vec2;

/// Sending a chunk to the user works the following way:
/// A system like `msg::terrain` `terrain` or `terrain_sync` either decide to
/// trigger chunk generation, or if the chunk already exists
/// push a `ChunkSendQueue` to the eventbus.
/// The `chunk_serialize` system will coordinate serializing via a SlowJob
/// outside of the tick. On the next tick, the `chunk_send` system will pick up
/// finished chunks.
///
/// Deferring allows us to remove code duplication and maybe serialize ONCE,
/// send to MULTIPLE clients
/// TODO: store a urgent flag and seperate even more, 5 ticks vs 5 seconds
#[derive(Debug, PartialEq, Eq)]
pub struct ChunkSendEntry {
    pub(crate) entity: Entity,
    pub(crate) chunk_key: Vec2<i32>,
}

pub struct SerializedChunk {
    pub(crate) lossy_compression: bool,
    pub(crate) msg: PreparedMsg,
    pub(crate) recipients: Vec<Entity>,
    /// ★ THE CHUNK'S OWN KEY, carried so the CONSUMER can order by it.
    ///
    /// Serialization runs in `SlowJob`s batched 10 per job, and each job sends
    /// its batch into the channel ON COMPLETION -- so `chunk_send` consumed
    /// chunks in thread-scheduling order and clients received terrain in a
    /// different order on every run. Without the key there was nothing to sort
    /// by, so the nondeterminism was unfixable at the consumer.
    ///
    /// This is the certification's bar 2 ("twin runs state-identical INCLUDING
    /// chunk timing"): membership is pinned by the deterministic barrier --
    /// measured identical on 30/30 twin pairs -- while the per-tick schedule
    /// differed on 31/31, because the barrier pins WHICH chunks are released
    /// and not WHEN each reaches a client. Its own doc says so: "membership is
    /// what this pins."
    pub(crate) key: Vec2<i32>,
}
