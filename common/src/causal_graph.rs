//! T0.59 + T0.62 (master build order; T0-004 packet, step 9): the causal
//! oracle. Compile recorder records into a DAG and check the packet's
//! causal invariants, returning the MINIMAL violating slice — not an exact
//! total-trace match, not a full model checker.
//!
//! Edges are Lamport-style happens-before (parent/causation/ack), NOT
//! vector clocks — a single-process authoritative server doesn't need them.
//!
//! Invariants (packet): no causal cycle; every causation target exists (or
//! is explicitly external); ordering rules such as commit-follows-
//! validation and ack-follows-admission hold; every node carries a terminal
//! outcome (a missing terminal is itself a failure — enforced by
//! [`CausalRecord`]'s non-optional `outcome`, checked here for the
//! never-recorded case via the caller's expected-node set).
//!
//! Determinism story (Ben's law): pure graph analysis over sorted/keyed
//! collections; no RNG, no wall-clock.

use crate::causal_record::{CausalId, CausalOutcome, CausalRecord};
use std::collections::{BTreeMap, BTreeSet};

/// One directed happens-before edge (cause → effect).
pub type CausalEdge = (CausalId, CausalId);

/// A compiled causal DAG.
#[derive(Clone, Debug, Default)]
pub struct CausalGraph {
    /// span_id → (terminal outcome, whether its cause is declared external).
    nodes: BTreeMap<CausalId, (CausalOutcome, bool)>,
    edges: BTreeSet<CausalEdge>,
}

/// The minimal violating evidence a check returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CausalViolation {
    /// A causal cycle — the node ids on it, in a canonical rotation.
    Cycle(Vec<CausalId>),
    /// An effect whose declared cause exists in no node and is not marked
    /// external.
    DanglingCausation { effect: CausalId, cause: CausalId },
    /// A declared ordering rule violated (the two nodes out of order).
    Ordering {
        rule: &'static str,
        earlier: CausalId,
        later: CausalId,
    },
    /// An expected node produced no record (missing terminal).
    MissingTerminal(CausalId),
}

impl CausalGraph {
    /// Compile records into the DAG. `external_causes` are causation ids the
    /// caller declares legitimately outside this trace (client input, ...).
    pub fn from_records(
        records: &[CausalRecord],
        external_causes: &BTreeSet<CausalId>,
    ) -> Self {
        let mut nodes = BTreeMap::new();
        let mut edges = BTreeSet::new();
        for record in records {
            let external = record
                .causation_id
                .is_some_and(|cause| external_causes.contains(&cause));
            nodes.insert(record.span_id, (record.outcome, external));
            if let Some(parent) = record.parent_span {
                edges.insert((parent, record.span_id));
            }
            if let Some(cause) = record.causation_id {
                edges.insert((cause, record.span_id));
            }
            for link in &record.links {
                edges.insert((*link, record.span_id));
            }
        }
        Self { nodes, edges }
    }

    /// Check all invariants; return the FIRST minimal violating slice (the
    /// packet's minimal-slice contract), or `None` if the graph is valid.
    pub fn check(&self) -> Option<CausalViolation> {
        // 1. Dangling causation: every edge source is a node or external.
        for (source, effect) in &self.edges {
            if !self.nodes.contains_key(source) {
                let external = self
                    .nodes
                    .get(effect)
                    .is_some_and(|(_, is_external)| *is_external);
                if !external {
                    return Some(CausalViolation::DanglingCausation {
                        effect: *effect,
                        cause: *source,
                    });
                }
            }
        }
        // 2. Cycle detection (DFS with a recursion stack).
        if let Some(cycle) = self.find_cycle() {
            return Some(CausalViolation::Cycle(cycle));
        }
        None
    }

    /// Check a declared ordering rule: `earlier` must reach `later` (a
    /// happens-before path), else the two are out of order. Used for the
    /// packet's commit-follows-validation / ack-follows-admission /
    /// compensation-follows-failure / demotion-save-before-delete rules.
    pub fn require_before(
        &self,
        rule: &'static str,
        earlier: CausalId,
        later: CausalId,
    ) -> Option<CausalViolation> {
        if self.reaches(earlier, later) {
            None
        } else {
            Some(CausalViolation::Ordering {
                rule,
                earlier,
                later,
            })
        }
    }

    /// Every expected node must have produced a record (a missing terminal
    /// is itself an invariant failure).
    pub fn require_present(&self, expected: &BTreeSet<CausalId>) -> Option<CausalViolation> {
        expected
            .iter()
            .find(|id| !self.nodes.contains_key(id))
            .map(|id| CausalViolation::MissingTerminal(*id))
    }

    fn reaches(&self, from: CausalId, to: CausalId) -> bool {
        let mut stack = vec![from];
        let mut seen = BTreeSet::new();
        while let Some(node) = stack.pop() {
            if node == to {
                return true;
            }
            if !seen.insert(node) {
                continue;
            }
            for (source, effect) in &self.edges {
                if *source == node {
                    stack.push(*effect);
                }
            }
        }
        false
    }

    fn find_cycle(&self) -> Option<Vec<CausalId>> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            Visiting,
            Done,
        }
        let mut marks: BTreeMap<CausalId, Mark> = BTreeMap::new();
        // Iterative DFS carrying the path so a found cycle is the slice.
        for &start in self.nodes.keys() {
            if marks.get(&start).is_some() {
                continue;
            }
            let mut path: Vec<CausalId> = Vec::new();
            let mut stack: Vec<(CausalId, bool)> = vec![(start, false)];
            while let Some((node, exiting)) = stack.pop() {
                if exiting {
                    marks.insert(node, Mark::Done);
                    path.pop();
                    continue;
                }
                if marks.get(&node) == Some(&Mark::Done) {
                    continue;
                }
                marks.insert(node, Mark::Visiting);
                path.push(node);
                stack.push((node, true));
                for (source, effect) in &self.edges {
                    if *source != node {
                        continue;
                    }
                    match marks.get(effect) {
                        Some(Mark::Visiting) => {
                            // Cycle: slice from the effect's position in path.
                            let at = path.iter().position(|n| n == effect).unwrap_or(0);
                            return Some(path[at..].to_vec());
                        },
                        Some(Mark::Done) => {},
                        None => stack.push((*effect, false)),
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod t0_59_tests {
    use super::*;
    use crate::causal_record::derive_span_id;
    use crate::state_hash::RecorderSchemaRef;

    fn record(span: CausalId, cause: Option<CausalId>, outcome: CausalOutcome) -> CausalRecord {
        CausalRecord {
            schema: RecorderSchemaRef {
                family: 1,
                major: 1,
                minor: 0,
                patch: 0,
            },
            run_id: 1,
            trace_id: CausalId(1),
            span_id: span,
            parent_span: None,
            links: Vec::new(),
            tick: 0,
            phase_ordinal: 0,
            sequence: 0,
            kind: 0,
            causation_id: cause,
            correlation_id: None,
            actor: None,
            target: None,
            pre_hash: None,
            post_hash: None,
            outcome,
        }
    }

    #[test]
    fn t0_62_valid_chain_passes_and_ordering_holds() {
        let a = CausalId(1);
        let b = CausalId(2);
        let c = CausalId(3);
        let records = [
            record(a, None, CausalOutcome::Ok),
            record(b, Some(a), CausalOutcome::Ok),
            record(c, Some(b), CausalOutcome::Ok),
        ];
        let graph = CausalGraph::from_records(&records, &BTreeSet::new());
        assert_eq!(graph.check(), None);
        // commit (c) follows validation (a) via the chain.
        assert_eq!(graph.require_before("commit-after-validation", a, c), None);
        // reverse ordering is a violation.
        assert!(matches!(
            graph.require_before("bad", c, a),
            Some(CausalViolation::Ordering { .. })
        ));
    }

    #[test]
    fn t0_59_dangling_cycle_and_missing_are_caught() {
        let a = CausalId(1);
        let b = CausalId(2);
        // Dangling: b's cause (99) is neither a node nor external.
        let dangling = [record(b, Some(CausalId(99)), CausalOutcome::Ok)];
        let graph = CausalGraph::from_records(&dangling, &BTreeSet::new());
        assert_eq!(
            graph.check(),
            Some(CausalViolation::DanglingCausation {
                effect: b,
                cause: CausalId(99),
            })
        );
        // ...but a declared-external cause is fine.
        let graph = CausalGraph::from_records(
            &dangling,
            &[CausalId(99)].into_iter().collect(),
        );
        assert_eq!(graph.check(), None);
        // Cycle: a↔b.
        let cyclic = [
            record(a, Some(b), CausalOutcome::Ok),
            record(b, Some(a), CausalOutcome::Ok),
        ];
        let graph = CausalGraph::from_records(&cyclic, &BTreeSet::new());
        assert!(matches!(graph.check(), Some(CausalViolation::Cycle(_))));
        // Missing terminal: expected node c produced no record.
        let graph = CausalGraph::from_records(
            &[record(a, None, CausalOutcome::Ok)],
            &BTreeSet::new(),
        );
        let _ = derive_span_id(1, 1, 1, 1, 1); // determinism helper exists
        assert_eq!(
            graph.require_present(&[a, CausalId(3)].into_iter().collect()),
            Some(CausalViolation::MissingTerminal(CausalId(3)))
        );
    }
}
