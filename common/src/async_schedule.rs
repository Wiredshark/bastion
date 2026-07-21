//! T0.65 (master build order; T0-004 packet, step 11): token-bucket
//! admission + Deficit Round Robin scheduling for async/event work.
//!
//! THE RULES (packet):
//! - Costs are SEMANTIC UNITS (one target affected, one entity created, one
//!   inventory mutation, one terrain cell changed, one child event emitted)
//!   — NEVER measured microseconds.
//! - Correctness work is delayed / backpressured, NEVER silently dropped.
//!   (Presentation/diagnostic work may be dropped under a declared policy —
//!   not modeled here; this is the correctness path.)
//! - Deficits and token balances are SIMULATION STATE — hashable and
//!   testable, so a deterministic run reproduces them exactly.
//! - Spillover stays stable FIFO within its class.
//!
//! Determinism story (Ben's law): integer arithmetic, stable FIFO queues,
//! class iteration in sorted-key order; no wall-clock, no RNG, no
//! iteration-order dependence.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// A token bucket bounding admission burst. Balance is simulation state —
/// serialize/hash it and a replay reproduces it exactly. Time advances in
/// SIM TICKS (never wall-clock).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBucket {
    capacity: u64,
    tokens: u64,
    refill_per_tick: u64,
}

impl TokenBucket {
    pub fn new(capacity: u64, refill_per_tick: u64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_per_tick,
        }
    }

    /// Advance one sim tick (saturating at capacity).
    pub fn tick(&mut self) {
        self.tokens = (self.tokens + self.refill_per_tick).min(self.capacity);
    }

    /// Try to admit `cost` semantic units: on success debit and return
    /// true; on insufficient tokens return false — the caller BACKPRESSURES
    /// (retries next tick), never drops a correctness item.
    pub fn try_admit(&mut self, cost: u64) -> bool {
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }

    pub fn tokens(&self) -> u64 { self.tokens }
}

/// A DRR class: a stable key, a per-round quantum (semantic units), a
/// carried deficit (simulation state), and a stable-FIFO backlog.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DrrClass<T> {
    quantum: u64,
    deficit: u64,
    backlog: VecDeque<(u64, T)>,
}

/// T0.65: Deficit Round Robin over classes keyed by a stable id. Serves
/// admitted work fairly by semantic cost; within a class, strict FIFO.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeficitRoundRobin<K: Ord + Clone, T> {
    classes: BTreeMap<K, DrrClass<T>>,
}

impl<K: Ord + Clone, T> Default for DeficitRoundRobin<K, T> {
    fn default() -> Self {
        Self {
            classes: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Clone, T> DeficitRoundRobin<K, T> {
    /// Declare a class with its quantum (idempotent on quantum; keeps the
    /// existing backlog/deficit).
    pub fn declare_class(&mut self, key: K, quantum: u64) {
        self.classes
            .entry(key)
            .and_modify(|class| class.quantum = quantum)
            .or_insert(DrrClass {
                quantum,
                deficit: 0,
                backlog: VecDeque::new(),
            });
    }

    /// Enqueue `item` of `cost` semantic units into `key`'s stable-FIFO
    /// backlog (declaring the class with `default_quantum` if new).
    pub fn enqueue(&mut self, key: K, cost: u64, item: T, default_quantum: u64) {
        self.classes
            .entry(key)
            .or_insert(DrrClass {
                quantum: default_quantum,
                deficit: 0,
                backlog: VecDeque::new(),
            })
            .backlog
            .push_back((cost, item));
    }

    pub fn is_empty(&self) -> bool {
        self.classes.values().all(|class| class.backlog.is_empty())
    }

    /// Run ONE DRR round: each non-empty class (in sorted-key order) gains
    /// its quantum and serves as many head items as its deficit covers;
    /// leftover deficit carries. Returns the served items in service order
    /// (the deterministic schedule). Nothing is dropped — unserved items
    /// stay FIFO for the next round.
    pub fn run_round(&mut self) -> Vec<(K, T)> {
        let mut served = Vec::new();
        for (key, class) in self.classes.iter_mut() {
            if class.backlog.is_empty() {
                // An idle class does not hoard deficit.
                class.deficit = 0;
                continue;
            }
            class.deficit += class.quantum;
            while let Some((cost, _)) = class.backlog.front() {
                if *cost <= class.deficit {
                    class.deficit -= *cost;
                    let (_, item) = class.backlog.pop_front().expect("front just checked");
                    served.push((key.clone(), item));
                } else {
                    break;
                }
            }
        }
        served
    }
}

#[cfg(test)]
mod t0_65_tests {
    use super::*;

    #[test]
    fn t0_65_token_bucket_backpressures_never_drops() {
        let mut bucket = TokenBucket::new(10, 3);
        assert!(bucket.try_admit(8));
        assert_eq!(bucket.tokens(), 2);
        // Insufficient: refused (caller retries) — not dropped.
        assert!(!bucket.try_admit(5));
        assert_eq!(bucket.tokens(), 2);
        // Refill is sim-tick paced and saturates at capacity.
        bucket.tick(); // 5
        bucket.tick(); // 8
        assert!(bucket.try_admit(5));
        // Balance is deterministic simulation state.
        let snapshot = bucket.clone();
        bucket.tick();
        assert_ne!(bucket, snapshot);
    }

    #[test]
    fn t0_65_drr_is_fair_fifo_and_deterministic() {
        let mut drr: DeficitRoundRobin<u8, &str> = DeficitRoundRobin::default();
        // Class A: quantum 2, three unit-cost items. Class B: quantum 1,
        // two unit-cost items.
        drr.enqueue(1, 1, "a1", 2);
        drr.enqueue(1, 1, "a2", 2);
        drr.enqueue(1, 1, "a3", 2);
        drr.enqueue(2, 1, "b1", 1);
        drr.enqueue(2, 1, "b2", 1);
        // Round 1: A gets quantum 2 → serves a1,a2 (FIFO); B gets 1 → b1.
        assert_eq!(drr.run_round(), vec![(1, "a1"), (1, "a2"), (2, "b1")]);
        // Round 2: A → a3; B → b2.
        assert_eq!(drr.run_round(), vec![(1, "a3"), (2, "b2")]);
        assert!(drr.is_empty());
    }

    #[test]
    fn t0_65_drr_deficit_carries_for_costly_items() {
        let mut drr: DeficitRoundRobin<u8, &str> = DeficitRoundRobin::default();
        // Quantum 2, a cost-5 item: takes 3 rounds to accumulate deficit.
        drr.enqueue(1, 5, "big", 2);
        assert_eq!(drr.run_round(), Vec::<(u8, &str)>::new()); // deficit 2
        assert_eq!(drr.run_round(), Vec::<(u8, &str)>::new()); // deficit 4
        assert_eq!(drr.run_round(), vec![(1, "big")]); // deficit 6 >= 5
        assert!(drr.is_empty());
    }
}
