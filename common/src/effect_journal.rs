//! T1.2 + T1.4 (master build order; T1-001 packet, step 4): the prepared
//! effect journal — a deterministic per-command unit of work, NOT permanent
//! event sourcing.
//!
//! Two phases: PREPARE stages every effect (component ops, lazy ops,
//! terrain ops, child events, publications) and does ALL fallible
//! validation — version expectations checked against current authoritative
//! generations. COMMIT on a [`PreparedEffectJournal`] is non-fallible
//! except process termination: allocations and conflict checks already
//! happened, so nothing between the last validation and the authoritative
//! write can fail. Fallible validation before the first authoritative
//! mutation, always (the shared gate).
//!
//! Rejects general 2PC — EventBus / network / persistence actors / terrain
//! are not all transactional resource managers. Async and durable effects
//! release through outboxes / typed saga steps AFTER the local commit, not
//! inside it. The type enforces this: only a `PreparedEffectJournal` can be
//! committed, and it is produced solely by successful [`prepare`].
//!
//! Determinism story (Ben's law): validation is a pure check over
//! sorted expectations; the commit applies staged ops in stable order; no
//! RNG, no wall-clock, no iteration-order dependence.

use crate::command_protocol::CommandId;
use serde::{Deserialize, Serialize};

/// A version expectation the commit's validity depends on: `target`'s
/// authoritative generation must equal `expected_generation` at prepare
/// time (the owner moved on ⇒ reject, the acceptance-predicate barrier).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionExpectation {
    pub target: u64,
    pub expected_generation: u64,
}

/// T1.2: the staged effect journal for one command. Generic over the
/// caller's concrete op types (component / terrain / event payloads).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectJournal<Comp, Terr, Ev> {
    pub command: CommandId,
    pub expected: Vec<VersionExpectation>,
    pub component_ops: Vec<Comp>,
    pub lazy_ops: Vec<Comp>,
    pub terrain_ops: Vec<Terr>,
    pub child_events: Vec<Ev>,
    pub publications: Vec<Ev>,
}

impl<Comp, Terr, Ev> EffectJournal<Comp, Terr, Ev> {
    pub fn new(command: CommandId) -> Self {
        Self {
            command,
            expected: Vec::new(),
            component_ops: Vec::new(),
            lazy_ops: Vec::new(),
            terrain_ops: Vec::new(),
            child_events: Vec::new(),
            publications: Vec::new(),
        }
    }

    pub fn expect(&mut self, target: u64, expected_generation: u64) -> &mut Self {
        self.expected.push(VersionExpectation {
            target,
            expected_generation,
        });
        self
    }
}

/// Why prepare failed (before any authoritative mutation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrepareError {
    /// An expected target's generation moved (the owner-moved-on barrier).
    StaleExpectation {
        target: u64,
        expected: u64,
        actual: Option<u64>,
    },
}

/// T1.4: a journal whose fallible validation has PASSED — commit is
/// non-fallible. Constructed only by [`prepare`]; the inner journal is
/// consumed by [`PreparedEffectJournal::commit`].
#[derive(Clone, Debug)]
pub struct PreparedEffectJournal<Comp, Terr, Ev>(EffectJournal<Comp, Terr, Ev>);

/// PREPARE: validate every version expectation against current
/// authoritative generations. Returns the prepared journal (commit
/// non-fallible) or the first stale expectation (nothing was mutated).
pub fn prepare<Comp, Terr, Ev>(
    journal: EffectJournal<Comp, Terr, Ev>,
    current_generation: impl Fn(u64) -> Option<u64>,
) -> Result<PreparedEffectJournal<Comp, Terr, Ev>, PrepareError> {
    for expectation in &journal.expected {
        let actual = current_generation(expectation.target);
        if actual != Some(expectation.expected_generation) {
            return Err(PrepareError::StaleExpectation {
                target: expectation.target,
                expected: expectation.expected_generation,
                actual,
            });
        }
    }
    Ok(PreparedEffectJournal(journal))
}

impl<Comp, Terr, Ev> PreparedEffectJournal<Comp, Terr, Ev> {
    /// COMMIT: apply the staged authoritative ops (component + terrain)
    /// via the caller's owners, then hand the post-commit events and
    /// publications back for OUTBOX release (never inside the commit). This
    /// method itself cannot fail — validation already happened.
    pub fn commit(
        self,
        mut apply_component: impl FnMut(Comp),
        mut apply_terrain: impl FnMut(Terr),
    ) -> CommittedEffects<Ev> {
        let journal = self.0;
        for op in journal.component_ops {
            apply_component(op);
        }
        for op in journal.lazy_ops {
            apply_component(op);
        }
        for op in journal.terrain_ops {
            apply_terrain(op);
        }
        CommittedEffects {
            command: journal.command,
            child_events: journal.child_events,
            publications: journal.publications,
        }
    }
}

/// The post-commit effects to release through outboxes / the command
/// journal — NOT applied inside the authoritative commit.
#[derive(Clone, Debug)]
pub struct CommittedEffects<Ev> {
    pub command: CommandId,
    pub child_events: Vec<Ev>,
    pub publications: Vec<Ev>,
}

#[cfg(test)]
mod t1_2_tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn t1_4_prepare_validates_before_any_mutation_then_commit_is_infallible() {
        let generations: BTreeMap<u64, u64> = [(1, 5), (2, 9)].into_iter().collect();
        let current = |target| generations.get(&target).copied();

        let mut journal: EffectJournal<&str, &str, &str> = EffectJournal::new(CommandId(1));
        journal.expect(1, 5).expect(2, 9);
        journal.component_ops.push("set-hp");
        journal.terrain_ops.push("dig");
        journal.publications.push("hp-changed");

        let prepared = prepare(journal, current).expect("expectations current");
        let mut applied = Vec::new();
        let mut terrain = Vec::new();
        let committed = prepared.commit(|op| applied.push(op), |op| terrain.push(op));
        assert_eq!(applied, vec!["set-hp"]);
        assert_eq!(terrain, vec!["dig"]);
        // Publications come back for OUTBOX release, not applied in commit.
        assert_eq!(committed.publications, vec!["hp-changed"]);
    }

    #[test]
    fn t1_2_stale_expectation_rejects_with_nothing_mutated() {
        let generations: BTreeMap<u64, u64> = [(1, 6)].into_iter().collect(); // moved 5→6
        let current = |target| generations.get(&target).copied();
        let mut journal: EffectJournal<&str, &str, &str> = EffectJournal::new(CommandId(1));
        journal.expect(1, 5); // stale
        journal.component_ops.push("would-corrupt");
        let result = prepare(journal, current);
        assert_eq!(
            result.err(),
            Some(PrepareError::StaleExpectation {
                target: 1,
                expected: 5,
                actual: Some(6),
            })
        );
        // The op was never applied — validate-before-mutate held.
    }
}
