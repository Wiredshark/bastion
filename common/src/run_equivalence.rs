//! T0.63 (master build order; T0-004 packet, step 9): run equivalence.
//!
//! Two runs are EQUIVALENT when, together:
//! 1. final authoritative domain hashes match (the [`FinalStateCertificate`]
//!    authoritative surface),
//! 2. every REQUIRED causal edge exists in both,
//! 3. exactly-once / conservation invariants match, and
//! 4. independent-event multisets match within a declared tolerance.
//!
//! Runs do NOT need identical topological order of INDEPENDENT events — a
//! canonical topological normalization would be a display aid only, never a
//! runtime-order claim. (Byte-identity, which the mf VM pairs prove, is a
//! STRONGER property than this; equivalence is the weaker contract that
//! still certifies correctness when legal schedule perturbation reorders
//! independent work — e.g. the T0.52 parallel probe or the T0.64 fuzzer.)
//!
//! Determinism story (Ben's law): a pure comparison over sorted/keyed
//! collections; no RNG, no wall-clock, no iteration-order dependence.

use crate::causal_record::CausalId;
use crate::state_hash::FinalStateCertificate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The comparable summary of one run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSummary {
    pub certificate: FinalStateCertificate,
    /// Causal edges (cause → effect) this run asserts happened.
    pub causal_edges: BTreeSet<(CausalId, CausalId)>,
    /// Named conserved quantities (items created − destroyed, ...) — must
    /// match EXACTLY between equivalent runs.
    pub conservation: BTreeMap<String, i64>,
    /// Independent-event multiset: event-kind label → count. Order-free by
    /// construction; compared within tolerance.
    pub independent_events: BTreeMap<String, u64>,
}

/// The per-event-kind absolute-count tolerance for independent events.
/// Zero = exact; a legal-schedule perturbation that changes only *which*
/// independent event fired first must not change the COUNT, so the default
/// is exact and tolerance is opt-in per kind.
#[derive(Clone, Debug, Default)]
pub struct EquivalenceTolerance {
    pub per_kind: BTreeMap<String, u64>,
}

/// Why two runs are not equivalent — the minimal violating evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EquivalenceVerdict {
    Equivalent,
    /// Final authoritative hashes differ (seed/tick/durable composite).
    HashMismatch,
    /// Required causal edges present in one run but not the other.
    MissingCausalEdges(Vec<(CausalId, CausalId)>),
    /// Named conserved quantities that differ.
    ConservationMismatch(Vec<String>),
    /// Independent-event kinds whose counts differ beyond tolerance.
    IndependentMultisetMismatch(Vec<String>),
}

/// The REQUIRED causal edges an equivalence check demands exist in both
/// runs (a subset of each run's full edge set — the load-bearing ones).
pub fn check_equivalence(
    a: &RunSummary,
    b: &RunSummary,
    required_edges: &BTreeSet<(CausalId, CausalId)>,
    tolerance: &EquivalenceTolerance,
) -> EquivalenceVerdict {
    // 1. Final authoritative hashes.
    if !a.certificate.authoritative_matches(&b.certificate) {
        return EquivalenceVerdict::HashMismatch;
    }
    // 2. Required causal edges exist in BOTH.
    let missing: Vec<(CausalId, CausalId)> = required_edges
        .iter()
        .filter(|edge| !(a.causal_edges.contains(edge) && b.causal_edges.contains(edge)))
        .copied()
        .collect();
    if !missing.is_empty() {
        return EquivalenceVerdict::MissingCausalEdges(missing);
    }
    // 3. Conservation invariants match exactly.
    let mut conservation_diffs: Vec<String> = Vec::new();
    for key in a.conservation.keys().chain(b.conservation.keys()) {
        if a.conservation.get(key) != b.conservation.get(key)
            && !conservation_diffs.contains(key)
        {
            conservation_diffs.push(key.clone());
        }
    }
    conservation_diffs.sort();
    conservation_diffs.dedup();
    if !conservation_diffs.is_empty() {
        return EquivalenceVerdict::ConservationMismatch(conservation_diffs);
    }
    // 4. Independent-event multisets within tolerance.
    let mut multiset_diffs: Vec<String> = Vec::new();
    for key in a
        .independent_events
        .keys()
        .chain(b.independent_events.keys())
    {
        let ca = a.independent_events.get(key).copied().unwrap_or(0);
        let cb = b.independent_events.get(key).copied().unwrap_or(0);
        let allowed = tolerance.per_kind.get(key).copied().unwrap_or(0);
        if ca.abs_diff(cb) > allowed && !multiset_diffs.contains(key) {
            multiset_diffs.push(key.clone());
        }
    }
    multiset_diffs.sort();
    multiset_diffs.dedup();
    if !multiset_diffs.is_empty() {
        return EquivalenceVerdict::IndependentMultisetMismatch(multiset_diffs);
    }
    EquivalenceVerdict::Equivalent
}

#[cfg(test)]
mod t0_63_tests {
    use super::*;
    use crate::state_hash::{DomainHash, IntegrityHash};

    fn cert(durable: u8) -> FinalStateCertificate {
        FinalStateCertificate {
            schema: "bastion/final-state-certificate/v1".to_string(),
            world_seed: 7,
            tick: 100,
            durable_composite: DomainHash([durable; 32]),
            rebuildable_integrity: IntegrityHash([0; 32]),
            domain_hashes: Vec::new(),
        }
    }

    fn summary(durable: u8) -> RunSummary {
        RunSummary {
            certificate: cert(durable),
            causal_edges: [(CausalId(1), CausalId(2)), (CausalId(2), CausalId(3))]
                .into_iter()
                .collect(),
            conservation: [("items".to_string(), 5)].into_iter().collect(),
            independent_events: [("chatter".to_string(), 10)].into_iter().collect(),
        }
    }

    #[test]
    fn t0_63_equivalent_runs_pass() {
        let required = [(CausalId(1), CausalId(2))].into_iter().collect();
        assert_eq!(
            check_equivalence(&summary(1), &summary(1), &required, &Default::default()),
            EquivalenceVerdict::Equivalent
        );
    }

    #[test]
    fn t0_63_each_dimension_fails_distinctly() {
        let required: BTreeSet<_> = [(CausalId(1), CausalId(2))].into_iter().collect();
        // Hash mismatch.
        assert_eq!(
            check_equivalence(&summary(1), &summary(2), &required, &Default::default()),
            EquivalenceVerdict::HashMismatch
        );
        // Missing required edge.
        let mut b = summary(1);
        b.causal_edges.remove(&(CausalId(1), CausalId(2)));
        assert_eq!(
            check_equivalence(&summary(1), &b, &required, &Default::default()),
            EquivalenceVerdict::MissingCausalEdges(vec![(CausalId(1), CausalId(2))])
        );
        // Conservation mismatch (items 5 vs 6) — an empty required set so
        // the check reaches conservation.
        let mut b = summary(1);
        b.conservation.insert("items".to_string(), 6);
        assert_eq!(
            check_equivalence(&summary(1), &b, &BTreeSet::new(), &Default::default()),
            EquivalenceVerdict::ConservationMismatch(vec!["items".to_string()])
        );
        // Independent multiset beyond tolerance.
        let mut b = summary(1);
        b.independent_events.insert("chatter".to_string(), 14);
        assert_eq!(
            check_equivalence(&summary(1), &b, &BTreeSet::new(), &Default::default()),
            EquivalenceVerdict::IndependentMultisetMismatch(vec!["chatter".to_string()])
        );
        // ...but within a declared per-kind tolerance, equivalent.
        let tolerance = EquivalenceTolerance {
            per_kind: [("chatter".to_string(), 5)].into_iter().collect(),
        };
        assert_eq!(
            check_equivalence(&summary(1), &b, &BTreeSet::new(), &tolerance),
            EquivalenceVerdict::Equivalent
        );
    }
}
