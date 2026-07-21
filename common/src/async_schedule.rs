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
        let (served, _spent) = self.run_round_bounded(u64::MAX);
        served
    }

    /// T0.66: a round bounded by a total semantic-unit `budget` — the
    /// hierarchical layer caps a domain's per-round spend this way. Returns
    /// (served items, units spent). A class stops mid-service once the
    /// shared budget is exhausted; its deficit carries, nothing is dropped.
    pub fn run_round_bounded(&mut self, budget: u64) -> (Vec<(K, T)>, u64) {
        let mut served = Vec::new();
        let mut spent = 0u64;
        for (key, class) in self.classes.iter_mut() {
            if class.backlog.is_empty() {
                // An idle class does not hoard deficit.
                class.deficit = 0;
                continue;
            }
            class.deficit = class.deficit.saturating_add(class.quantum);
            while let Some((cost, _)) = class.backlog.front() {
                if *cost <= class.deficit && spent + *cost <= budget {
                    class.deficit -= *cost;
                    spent += *cost;
                    let (_, item) = class.backlog.pop_front().expect("front just checked");
                    served.push((key.clone(), item));
                } else {
                    break;
                }
            }
        }
        (served, spent)
    }
}

/// T0.66 (master build order; T0-004 packet, step 12): the six scheduling
/// domains — REUSED VERBATIM from the T0.12 dispatcher-manifest groupings
/// (zero new taxonomy, Fork 2 ruling). Global budget → these domain
/// budgets → class queues → stable owner queues.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SchedulingDomain {
    Path,
    Events,
    Jobs,
    Rtsim,
    Terrain,
    PersistenceApply,
}

/// One domain's budget policy + its inner (class, owner) DRR.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DomainBudget<T> {
    /// Semantic units this domain may spend per global round. DEFAULTS to
    /// `u64::MAX` (unbounded) — today's behavior, no throttling; tuning is
    /// deferred to a real fixture (Fork 2 ruling).
    quantum: u64,
    /// The DRR over (class id, stable owner id) within the domain.
    inner: DeficitRoundRobin<(u32, u64), T>,
}

/// T0.66: hierarchical DRR — global budget distributed across the six
/// domains (sorted order), each domain's grant capping its inner
/// (class, owner) DRR round. Wall-duration telemetry is diagnostic/host-
/// safety only and NEVER decides what committed here.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HierarchicalDrr<T> {
    domains: BTreeMap<SchedulingDomain, DomainBudget<T>>,
    /// Total semantic units across all domains per round; `u64::MAX`
    /// (unbounded) by default.
    global_quantum: u64,
}

impl<T> Default for HierarchicalDrr<T> {
    fn default() -> Self {
        Self {
            domains: BTreeMap::new(),
            global_quantum: u64::MAX,
        }
    }
}

impl<T> HierarchicalDrr<T> {
    pub fn set_global_quantum(&mut self, quantum: u64) { self.global_quantum = quantum; }

    /// Declare a domain with its per-round quantum (unbounded = u64::MAX).
    pub fn declare_domain(&mut self, domain: SchedulingDomain, quantum: u64) {
        self.domains
            .entry(domain)
            .and_modify(|budget| budget.quantum = quantum)
            .or_insert(DomainBudget {
                quantum,
                inner: DeficitRoundRobin::default(),
            });
    }

    /// Enqueue into a domain's (class, owner) queue (declaring the domain
    /// unbounded + the inner class with `default_quantum` if new).
    pub fn enqueue(
        &mut self,
        domain: SchedulingDomain,
        class: u32,
        owner: u64,
        cost: u64,
        item: T,
        default_quantum: u64,
    ) {
        self.domains
            .entry(domain)
            .or_insert(DomainBudget {
                quantum: u64::MAX,
                inner: DeficitRoundRobin::default(),
            })
            .inner
            .enqueue((class, owner), cost, item, default_quantum);
    }

    pub fn is_empty(&self) -> bool {
        self.domains.values().all(|budget| budget.inner.is_empty())
    }

    /// Run one global round: domains in sorted order, each granted
    /// min(its quantum, remaining global budget), running its inner DRR
    /// bounded by that grant. Returns served items as
    /// (domain, (class, owner), item) in the deterministic schedule order.
    #[expect(clippy::type_complexity, reason = "the hierarchical service tuple")]
    pub fn run_round(&mut self) -> Vec<(SchedulingDomain, (u32, u64), T)> {
        let mut served = Vec::new();
        let mut global_remaining = self.global_quantum;
        for (domain, budget) in self.domains.iter_mut() {
            let grant = budget.quantum.min(global_remaining);
            let (items, spent) = budget.inner.run_round_bounded(grant);
            global_remaining = global_remaining.saturating_sub(spent);
            for (key, item) in items {
                served.push((*domain, key, item));
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

    #[test]
    fn t0_66_unbounded_default_serves_everything_deterministically() {
        let mut drr: HierarchicalDrr<&str> = HierarchicalDrr::default();
        // Enqueue across two domains, all unbounded — everything serves in
        // sorted domain order, one round.
        drr.enqueue(SchedulingDomain::Jobs, 0, 1, 1, "job-a", u64::MAX);
        drr.enqueue(SchedulingDomain::Path, 0, 1, 1, "path-a", u64::MAX);
        drr.enqueue(SchedulingDomain::Jobs, 0, 2, 1, "job-b", u64::MAX);
        let served = drr.run_round();
        // Path < Jobs in the enum order; within Jobs, owner 1 then 2 (stable key).
        assert_eq!(
            served,
            vec![
                (SchedulingDomain::Path, (0, 1), "path-a"),
                (SchedulingDomain::Jobs, (0, 1), "job-a"),
                (SchedulingDomain::Jobs, (0, 2), "job-b"),
            ]
        );
        assert!(drr.is_empty());
    }

    #[test]
    fn t0_66_global_budget_caps_and_carries_never_drops() {
        let mut drr: HierarchicalDrr<&str> = HierarchicalDrr::default();
        drr.set_global_quantum(2); // only 2 semantic units per round
        drr.declare_domain(SchedulingDomain::Path, u64::MAX);
        drr.enqueue(SchedulingDomain::Path, 0, 1, 1, "p1", u64::MAX);
        drr.enqueue(SchedulingDomain::Path, 0, 2, 1, "p2", u64::MAX);
        drr.enqueue(SchedulingDomain::Path, 0, 3, 1, "p3", u64::MAX);
        // Round 1: budget 2 → p1, p2. p3 carries (never dropped).
        let round1 = drr.run_round();
        assert_eq!(round1.len(), 2);
        assert!(!drr.is_empty());
        // Round 2: p3 serves.
        let round2 = drr.run_round();
        assert_eq!(round2, vec![(SchedulingDomain::Path, (0, 3), "p3")]);
        assert!(drr.is_empty());
    }
}
