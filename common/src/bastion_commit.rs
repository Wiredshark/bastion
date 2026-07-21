//! T1.8 (master build order; T1-001 packet, step 7): the typed
//! `BastionCommitQueue`.
//!
//! Producers plan commits from IMMUTABLE snapshots (never mutating live
//! state during planning). The queue stable-sorts by
//! (generation, target identity, kind) — a deterministic total order, never
//! arrival — detects conflicts (two commits touching the same target),
//! validates authority generations, and hands the survivors to the EXISTING
//! owners (terrain / inventory / lifecycle / RTSim) to commit. JobBoard
//! stays COORDINATION truth; this queue is not another world database.
//!
//! Determinism story (Ben's law): stable total-order sort ending in stable
//! identity, deterministic conflict resolution (lowest generation wins the
//! target), pure validation; no RNG, no wall-clock.

use serde::{Deserialize, Serialize};

/// A stable commit target identity — a terrain cell hash, an item uid, a
/// job id, ... — NEVER a recycling ECS entity id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CommitTarget(pub u64);

/// Which authority a Bastion commit writes through.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BastionCommitKind {
    TerrainWork,
    ItemTransfer,
    ConsumeAndBuild,
    NeedAction,
    Lifecycle,
}

/// One planned commit. `generation` is the command/job authority generation
/// the commit was planned against; `target` is what it writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionCommit {
    pub kind: BastionCommitKind,
    pub generation: u64,
    pub target: CommitTarget,
}

/// Why a commit was rejected at drain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitRejection {
    /// The authority generation moved since planning (stale snapshot).
    StaleGeneration {
        target: CommitTarget,
        planned: u64,
        current: Option<u64>,
    },
    /// Another commit already claimed this target this drain (conflict —
    /// the lower-generation, then lower-kind commit wins).
    Conflict { target: CommitTarget },
}

/// T1.8: the typed commit queue. Pure state (serialize/hash it).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BastionCommitQueue {
    pending: Vec<BastionCommit>,
}

impl BastionCommitQueue {
    pub fn plan(&mut self, commit: BastionCommit) { self.pending.push(commit); }

    pub fn is_empty(&self) -> bool { self.pending.is_empty() }

    /// Drain: stable-sort by (generation, target, kind), reject stale
    /// generations, resolve target conflicts (the first survivor in sorted
    /// order — lowest generation — wins). Returns (committable in order,
    /// rejections). Committable commits are then applied by the caller
    /// through the EXISTING owners.
    #[expect(clippy::type_complexity, reason = "the two drain lanes")]
    pub fn drain(
        &mut self,
        current_generation: impl Fn(CommitTarget) -> Option<u64>,
    ) -> (Vec<BastionCommit>, Vec<CommitRejection>) {
        let mut batch = core::mem::take(&mut self.pending);
        batch.sort_by_key(|commit| (commit.generation, commit.target, commit.kind));
        let mut committable = Vec::new();
        let mut rejections = Vec::new();
        let mut claimed = std::collections::BTreeSet::new();
        for commit in batch {
            // Generation validation (the acceptance-predicate barrier).
            let current = current_generation(commit.target);
            if current != Some(commit.generation) {
                rejections.push(CommitRejection::StaleGeneration {
                    target: commit.target,
                    planned: commit.generation,
                    current,
                });
                continue;
            }
            // Conflict: another survivor already claimed this target.
            if !claimed.insert(commit.target) {
                rejections.push(CommitRejection::Conflict {
                    target: commit.target,
                });
                continue;
            }
            committable.push(commit);
        }
        (committable, rejections)
    }
}

#[cfg(test)]
mod t1_8_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn commit(kind: BastionCommitKind, generation: u64, target: u64) -> BastionCommit {
        BastionCommit {
            kind,
            generation,
            target: CommitTarget(target),
        }
    }

    #[test]
    fn t1_8_drains_in_stable_order_not_arrival() {
        let generations: BTreeMap<u64, u64> = [(10, 1), (20, 1), (30, 2)].into_iter().collect();
        let current = |t: CommitTarget| generations.get(&t.0).copied();
        let mut queue = BastionCommitQueue::default();
        // Arrival scrambled; distinct targets, all generations current.
        queue.plan(commit(BastionCommitKind::Lifecycle, 2, 30));
        queue.plan(commit(BastionCommitKind::TerrainWork, 1, 20));
        queue.plan(commit(BastionCommitKind::TerrainWork, 1, 10));
        let (committable, rejections) = queue.drain(current);
        assert!(rejections.is_empty());
        // Sorted by (generation, target): (1,10), (1,20), (2,30).
        let targets: Vec<u64> = committable.iter().map(|c| c.target.0).collect();
        assert_eq!(targets, vec![10, 20, 30]);
    }

    #[test]
    fn t1_8_rejects_stale_generation_and_conflicts() {
        let generations: BTreeMap<u64, u64> = [(10, 5)].into_iter().collect();
        let current = |t: CommitTarget| generations.get(&t.0).copied();
        let mut queue = BastionCommitQueue::default();
        // Stale: planned gen 4, current 5.
        queue.plan(commit(BastionCommitKind::TerrainWork, 4, 10));
        let (committable, rejections) = queue.drain(&current);
        assert!(committable.is_empty());
        assert_eq!(rejections, vec![CommitRejection::StaleGeneration {
            target: CommitTarget(10),
            planned: 4,
            current: Some(5),
        }]);

        // Conflict: two current commits for the same target — lowest-kind
        // (sorted first) wins, the other is a Conflict rejection.
        let mut queue = BastionCommitQueue::default();
        queue.plan(commit(BastionCommitKind::ItemTransfer, 5, 10));
        queue.plan(commit(BastionCommitKind::NeedAction, 5, 10));
        let (committable, rejections) = queue.drain(&current);
        assert_eq!(committable.len(), 1);
        assert_eq!(committable[0].kind, BastionCommitKind::ItemTransfer); // sorts before NeedAction
        assert_eq!(rejections, vec![CommitRejection::Conflict {
            target: CommitTarget(10),
        }]);
    }
}
