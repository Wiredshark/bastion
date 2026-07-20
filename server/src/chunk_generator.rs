use crate::metrics::ChunkGenMetrics;
#[cfg(feature = "worldgen")]
use crate::rtsim::RtSim;
#[cfg(not(feature = "worldgen"))]
use crate::test_world::{IndexOwned, World};
use common::{
    calendar::Calendar, generation::ChunkSupplement, resources::TimeOfDay, slowjob::SlowJobPool,
    terrain::TerrainChunk,
};
use hashbrown::{HashMap, hash_map::Entry};
use rayon::iter::ParallelIterator;
use specs::Entity as EcsEntity;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vek::*;
#[cfg(feature = "worldgen")]
use world::{IndexOwned, World};

type ChunkGenPayload = Result<(TerrainChunk, ChunkSupplement), Option<EcsEntity>>;
type ChunkGenResult = (Vec2<i32>, ChunkGenPayload);

struct Pending {
    cancel: Arc<AtomicBool>,
    /// bastion ENGOPT4 stage 2: the tick this chunk was requested — the
    /// deterministic barrier releases it exactly `delay` ticks later.
    request_tick: u64,
}

pub struct ChunkGenerator {
    chunk_tx: crossbeam_channel::Sender<ChunkGenResult>,
    chunk_rx: crossbeam_channel::Receiver<ChunkGenResult>,
    pending_chunks: HashMap<Vec2<i32>, Pending>,
    /// bastion ENGOPT4 stage 2: results that ARRIVED before their release
    /// tick — held so apply timing is a pure function of (request tick,
    /// delay), not of generation speed. Live mode drains this empty.
    arrived: HashMap<Vec2<i32>, ChunkGenPayload>,
    current_tick: u64,
    metrics: Arc<ChunkGenMetrics>,
}
impl ChunkGenerator {
    pub fn new(metrics: ChunkGenMetrics) -> Self {
        let (chunk_tx, chunk_rx) = crossbeam_channel::unbounded();
        Self {
            chunk_tx,
            chunk_rx,
            pending_chunks: HashMap::new(),
            arrived: HashMap::new(),
            current_tick: 0,
            metrics: Arc::new(metrics),
        }
    }

    pub fn generate_chunk(
        &mut self,
        entity: Option<EcsEntity>,
        key: Vec2<i32>,
        slowjob_pool: &SlowJobPool,
        world: Arc<World>,
        #[cfg(feature = "worldgen")] rtsim: &RtSim,
        #[cfg(not(feature = "worldgen"))] _rtsim: &(),
        index: IndexOwned,
        time: (TimeOfDay, Calendar),
    ) {
        let v = if let Entry::Vacant(v) = self.pending_chunks.entry(key) {
            v
        } else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        v.insert(Pending {
            cancel: Arc::clone(&cancel),
            request_tick: self.current_tick,
        });
        let chunk_tx = self.chunk_tx.clone();
        self.metrics.chunks_requested.inc();

        // Get state for this chunk from rtsim
        #[cfg(feature = "worldgen")]
        let rtsim_resources = Some(rtsim.get_chunk_resources(key));
        #[cfg(not(feature = "worldgen"))]
        let rtsim_resources = None;

        slowjob_pool.spawn("CHUNK_GENERATOR", move || {
            let index = index.as_index_ref();
            // bastion (FLAT-TEST-ARENA): the runtime arena override — a
            // flat slab inside the radius when BASTION_FLAT_ARENA is set,
            // the real generator otherwise (a single cold branch when
            // the flag is absent).
            let payload = crate::bastion_flat_arena::override_chunk(
                crate::bastion_flat_arena::world_center_wpos(&world),
                key,
            )
            .map(Ok)
            .unwrap_or_else(|| {
                world.generate_chunk(index, key, rtsim_resources, || cancel.load(Ordering::Relaxed), Some(time))
            })
                // FIXME: Since only the first entity who cancels a chunk is notified, we end up
                // delaying chunk re-requests for up to 3 seconds for other clients, which isn't
                // great.  We *could* store all the other requesting clients here, but it could
                // bloat memory a lot.  Currently, this isn't much of an issue because we rarely
                // have large numbers of pending chunks, so most of them are likely to be nearby an
                // actual player most of the time, but that will eventually change.  In the future,
                // some solution that always pushes chunk updates to players (rather than waiting
                // for explicit requests) should adequately solve this kind of issue.
                .map_err(|_| entity);
            let _ = chunk_tx.send((key, payload));
        });
    }

    pub fn recv_new_chunk(&mut self) -> Option<ChunkGenResult> {
        // Make sure chunk wasn't cancelled and if it was check to see if there are more
        // chunks to receive
        while let Ok((key, res)) = self.chunk_rx.try_recv() {
            if self.pending_chunks.remove(&key).is_some() {
                self.metrics.chunks_served.inc();
                // TODO: do anything else if res is an Err?
                return Some((key, res));
            }
        }

        None
    }


    /// bastion ENGOPT4 stage 2: advance the generator's tick clock (called
    /// once per tick from the terrain system before draining).
    pub fn note_tick(&mut self, tick: u64) { self.current_tick = tick; }

    /// bastion ENGOPT4 stage 2 — the DETERMINISTIC APPLY BARRIER (harness
    /// mode; the DETRNG precedent): a chunk requested at tick T applies at
    /// exactly T + `delay`, regardless of generation speed. Early arrivals
    /// are HELD in `arrived`; due-but-absent chunks are WAITED for
    /// (blocking recv, loud bounded cap — the harness is sim-paced, so
    /// blocking trades wall time for byte-identity). Live mode never calls
    /// this. Probe evidence for the design: stages 1+1b converged
    /// terrain-content outcomes across VMs (mf_completion byte-equal) but
    /// per-tick apply MEMBERSHIP still shifted agent timing (teleports
    /// 10v17) — membership is what this pins.
    pub fn recv_new_chunks_deterministic(&mut self, delay: u64) -> Vec<ChunkGenResult> {
        while let Ok((key, res)) = self.chunk_rx.try_recv() {
            if self.pending_chunks.contains_key(&key) {
                self.arrived.insert(key, res);
            }
        }
        let due: Vec<Vec2<i32>> = self
            .pending_chunks
            .iter()
            .filter(|(_, p)| p.request_tick.saturating_add(delay) <= self.current_tick)
            .map(|(k, _)| *k)
            .collect();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        for key in &due {
            while !self.arrived.contains_key(key) {
                if std::time::Instant::now() > deadline {
                    tracing::error!(
                        ?key,
                        "deterministic chunk barrier: due chunk never arrived within the \
                         cap — releasing partial set (this run's determinism is COMPROMISED)"
                    );
                    break;
                }
                match self
                    .chunk_rx
                    .recv_timeout(std::time::Duration::from_millis(50))
                {
                    Ok((k, res)) => {
                        if self.pending_chunks.contains_key(&k) {
                            self.arrived.insert(k, res);
                        }
                    },
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {},
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                }
            }
        }
        let mut out: Vec<ChunkGenResult> = Vec::new();
        for key in due {
            if let Some(res) = self.arrived.remove(&key) {
                self.pending_chunks.remove(&key);
                self.metrics.chunks_served.inc();
                out.push((key, res));
            }
        }
        out.sort_unstable_by_key(|(key, _)| (key.x, key.y));
        out
    }

    /// bastion ENGINE-OPT-4 (ARCH-003, ledger "stable-sort authoritative
    /// completions"): drain EVERY completed chunk available this tick and
    /// return them in DETERMINISTIC order (chunk key, lexicographic). The
    /// raw channel yields completion-time order — scheduling-dependent (the
    /// same-platform triple-divergence diagnosis: three identical AMD Rome
    /// VMs, three different simulations). With this, within-tick apply
    /// order is a pure function of the completed SET. Per-tick MEMBERSHIP
    /// (which chunks completed by now) remains wall-time-dependent — the
    /// harness-mode apply-barrier question, tracked as this block's second
    /// stage. Cancel-stale rejection (the pending_chunks epoch-equivalent)
    /// is preserved unchanged.
    pub fn recv_new_chunks_sorted(&mut self) -> Vec<ChunkGenResult> {
        let mut out = Vec::new();
        let held: Vec<Vec2<i32>> = self.arrived.keys().copied().collect();
        for key in held {
            if self.pending_chunks.remove(&key).is_some() {
                if let Some(res) = self.arrived.remove(&key) {
                    self.metrics.chunks_served.inc();
                    out.push((key, res));
                }
            }
        }
        while let Ok((key, res)) = self.chunk_rx.try_recv() {
            if self.pending_chunks.remove(&key).is_some() {
                self.metrics.chunks_served.inc();
                out.push((key, res));
            }
        }
        out.sort_unstable_by_key(|(key, _)| (key.x, key.y));
        out
    }

    pub fn pending_chunks(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.pending_chunks.keys().copied()
    }

    pub fn par_pending_chunks(&self) -> impl rayon::iter::ParallelIterator<Item = Vec2<i32>> + '_ {
        self.pending_chunks.par_keys().copied()
    }

    pub fn cancel_if_pending(&mut self, key: Vec2<i32>) {
        if let Some(pending) = self.pending_chunks.remove(&key) {
            pending.cancel.store(true, Ordering::Relaxed);
            self.arrived.remove(&key);
            self.metrics.chunks_canceled.inc();
        }
    }

    pub fn cancel_all(&mut self) {
        let metrics = Arc::clone(&self.metrics);
        self.pending_chunks.drain().for_each(|(_, pending)| {
            pending.cancel.store(true, Ordering::Relaxed);
            metrics.chunks_canceled.inc();
        });
        self.arrived.clear();
    }
}

// bastion ENGINE-OPT-4 falsifier: the sorted drain's output order must be a
// pure function of the completed SET — invariant under completion-order
// permutation (the raw channel's order is scheduling-dependent; three
// identical AMD Rome VMs produced three different simulations through it).
#[cfg(test)]
mod tests {
    use super::*;
    use common::slowjob::SlowJobPool;
    use std::sync::Arc;

    fn pool() -> SlowJobPool {
        SlowJobPool::new(
            2,
            10,
            Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap()),
        )
    }

    /// Stage-2 barrier semantics: an early arrival is HELD until its
    /// deterministic due tick (request_tick + delay); the due set releases
    /// sorted. Apply membership = f(request stream), not generation speed.
    #[test]
    fn engopt4_barrier_holds_early_arrivals_until_due() {
        let mut generator = ChunkGenerator::new(
            crate::metrics::ChunkGenMetrics::new(&prometheus::Registry::new()).unwrap(),
        );
        let a = Vec2::new(5, 5);
        let b = Vec2::new(-3, 2);
        generator.note_tick(0);
        for key in [a, b] {
            generator.pending_chunks.insert(key, Pending {
                cancel: Arc::new(AtomicBool::new(false)),
                request_tick: 0,
            });
        }
        // Both results arrive IMMEDIATELY (generation "instant").
        generator.chunk_tx.send((a, Err(None))).unwrap();
        generator.chunk_tx.send((b, Err(None))).unwrap();
        // Before the due tick: nothing may release, arrivals are held.
        generator.note_tick(10);
        assert!(generator.recv_new_chunks_deterministic(30).is_empty());
        assert_eq!(generator.arrived.len(), 2, "early arrivals must be held");
        // At the due tick: both release, in key order.
        generator.note_tick(30);
        let released: Vec<Vec2<i32>> = generator
            .recv_new_chunks_deterministic(30)
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(released, vec![b, a], "due set releases in sorted key order");
        assert!(generator.pending_chunks.is_empty());
    }

    #[test]
    fn engopt4_sorted_drain_is_completion_order_invariant() {
        let _ = pool();
        let keys: Vec<Vec2<i32>> = vec![
            Vec2::new(3, -1),
            Vec2::new(-2, 7),
            Vec2::new(0, 0),
            Vec2::new(3, 4),
            Vec2::new(-2, -9),
        ];
        let mut reference: Option<Vec<Vec2<i32>>> = None;
        // Feed the SAME completed set in several different completion orders
        // (rotations + a reversal — the permutation family the A* falsifier
        // established); the drained order must be identical every time.
        for perm in 0..6 {
            let mut generator = ChunkGenerator::new(crate::metrics::ChunkGenMetrics::new(
                &prometheus::Registry::new(),
            )
            .unwrap());
            let mut order = keys.clone();
            order.rotate_left(perm % keys.len());
            if perm >= keys.len() {
                order.reverse();
            }
            for key in &order {
                // Register as pending (the stale-gate) then complete directly
                // through the channel — the drain path under test.
                generator.pending_chunks.insert(*key, Pending {
                    cancel: Arc::new(AtomicBool::new(false)),
                    request_tick: 0,
                });
                generator
                    .chunk_tx
                    .send((*key, Err(None)))
                    .expect("test channel send");
            }
            let drained: Vec<Vec2<i32>> = generator
                .recv_new_chunks_sorted()
                .into_iter()
                .map(|(key, _)| key)
                .collect();
            assert_eq!(drained.len(), keys.len(), "every completion drained");
            match &reference {
                None => reference = Some(drained),
                Some(reference) => assert_eq!(
                    &drained, reference,
                    "drain order must be invariant under completion-order permutation (perm {perm})"
                ),
            }
        }
        // And the order is the deterministic key order, explicitly.
        let mut expected = keys;
        expected.sort_unstable_by_key(|k| (k.x, k.y));
        assert_eq!(reference.unwrap(), expected);
    }
}
