//! BUILD-007A10.15 (pure core) — deterministic render-pass graph (design
//! §13.7 / RES-051 FrameGraph prior art; Bastion's deterministic topological
//! order and hash identity are project-owned requirements).
//!
//! - `RendererPassGraphManifestV1`: declared pass nodes + EXPLICIT hazard
//!   edges. Implicit synchronization never orders the canonical graph.
//! - Canonical topological order: Kahn's algorithm with a MIN-QUEUE over
//!   `(pass_rank, pass_digest)` — insertion order and simultaneous-ready ties
//!   are deterministically resolved, and a cycle is a typed terminal.
//! - `PassExecutionRecordV1` conformance: an observed execution order is valid
//!   iff it is exactly the canonical order — pass skipping, reordering, or
//!   unknown passes are typed failures.
//!
//! The live drawer hooks that feed real execution records are the voxygen
//! seam; this module is the golden-testable authority.

use std::collections::BTreeMap;

/// One declared pass node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassNodeV1 {
    /// Frozen rank (primary order key). Distinct passes may share a rank;
    /// the digest breaks ties deterministically.
    pub pass_rank: u16,
    pub pass_digest: [u8; 32],
    pub name: String,
}

/// An explicit hazard edge: `before` must execute before `after`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HazardEdgeV1 {
    pub before: [u8; 32],
    pub after: [u8; 32],
}

/// Typed graph failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PassGraphError {
    DuplicatePassDigest { digest: [u8; 32] },
    /// An edge references a pass digest not in the node set.
    UndeclaredPass { digest: [u8; 32] },
    /// The hazard edges form a cycle — no canonical order exists.
    CycleDetected { remaining: usize },
}

/// Typed execution-conformance failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionMismatch {
    UnknownPass { digest: [u8; 32] },
    OutOfOrder { position: usize, expected: [u8; 32], observed: [u8; 32] },
    MissingPasses { count: usize },
    ExtraPasses { count: usize },
}

/// The declared pass-graph manifest.
#[derive(Clone, Debug)]
pub struct RendererPassGraphManifestV1 {
    nodes: BTreeMap<[u8; 32], PassNodeV1>,
    edges: Vec<HazardEdgeV1>,
    canonical_order: Vec<[u8; 32]>,
}

impl RendererPassGraphManifestV1 {
    /// Build and validate the manifest: reject duplicate digests and edges
    /// referencing undeclared passes, then compute the canonical topological
    /// order via Kahn with a min-queue over `(pass_rank, pass_digest)`. Node
    /// and edge INSERTION order can never affect the result.
    pub fn build(
        nodes: Vec<PassNodeV1>,
        edges: Vec<HazardEdgeV1>,
    ) -> Result<Self, PassGraphError> {
        let mut map: BTreeMap<[u8; 32], PassNodeV1> = BTreeMap::new();
        for n in nodes {
            if map.insert(n.pass_digest, n.clone()).is_some() {
                return Err(PassGraphError::DuplicatePassDigest { digest: n.pass_digest });
            }
        }
        for e in &edges {
            for d in [e.before, e.after] {
                if !map.contains_key(&d) {
                    return Err(PassGraphError::UndeclaredPass { digest: d });
                }
            }
        }
        // Kahn with deterministic min-queue keyed (rank, digest).
        let mut indegree: BTreeMap<[u8; 32], usize> = map.keys().map(|d| (*d, 0)).collect();
        // Successor lists in sorted key order (BTreeMap-derived, insertion-free).
        let mut succ: BTreeMap<[u8; 32], Vec<[u8; 32]>> = BTreeMap::new();
        {
            // Dedup edges canonically so a repeated edge cannot double-count.
            let mut sorted_edges: Vec<(&HazardEdgeV1, ())> = edges.iter().map(|e| (e, ())).collect();
            sorted_edges.sort_by(|a, b| (a.0.before, a.0.after).cmp(&(b.0.before, b.0.after)));
            sorted_edges.dedup_by(|a, b| a.0 == b.0);
            for (e, ()) in sorted_edges {
                succ.entry(e.before).or_default().push(e.after);
                *indegree.get_mut(&e.after).expect("validated") += 1;
            }
        }
        let mut ready: std::collections::BTreeSet<(u16, [u8; 32])> = indegree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(d, _)| (map[d].pass_rank, *d))
            .collect();
        let mut order = Vec::with_capacity(map.len());
        while let Some(&(rank, digest)) = ready.iter().next() {
            ready.remove(&(rank, digest));
            order.push(digest);
            if let Some(next) = succ.get(&digest) {
                for &n in next {
                    let deg = indegree.get_mut(&n).expect("validated");
                    *deg -= 1;
                    if *deg == 0 {
                        ready.insert((map[&n].pass_rank, n));
                    }
                }
            }
        }
        if order.len() != map.len() {
            return Err(PassGraphError::CycleDetected { remaining: map.len() - order.len() });
        }
        Ok(Self { nodes: map, edges, canonical_order: order })
    }

    #[must_use]
    pub fn canonical_order(&self) -> &[[u8; 32]] {
        &self.canonical_order
    }

    /// Domain-separated manifest digest over nodes (sorted), edges (sorted),
    /// and the canonical order.
    #[must_use]
    pub fn manifest_digest(&self) -> [u8; 32] {
        let mut p = Vec::new();
        p.extend_from_slice(&(self.nodes.len() as u64).to_le_bytes());
        for n in self.nodes.values() {
            p.extend_from_slice(&n.pass_rank.to_le_bytes());
            p.extend_from_slice(&n.pass_digest);
            p.extend_from_slice(&(n.name.len() as u64).to_le_bytes());
            p.extend_from_slice(n.name.as_bytes());
        }
        let mut es: Vec<([u8; 32], [u8; 32])> = self.edges.iter().map(|e| (e.before, e.after)).collect();
        es.sort();
        es.dedup();
        p.extend_from_slice(&(es.len() as u64).to_le_bytes());
        for (b, a) in es {
            p.extend_from_slice(&b);
            p.extend_from_slice(&a);
        }
        for d in &self.canonical_order {
            p.extend_from_slice(d);
        }
        crate::domain_hash("bastion/r0d/pass-graph", 1, 0, &p)
    }

    /// Check an observed execution tape against the canonical order: it must
    /// match EXACTLY — no skip, reorder, unknown, or extra pass.
    pub fn check_execution(&self, observed: &[[u8; 32]]) -> Result<(), ExecutionMismatch> {
        for d in observed {
            if !self.nodes.contains_key(d) {
                return Err(ExecutionMismatch::UnknownPass { digest: *d });
            }
        }
        for (i, (exp, obs)) in self.canonical_order.iter().zip(observed.iter()).enumerate() {
            if exp != obs {
                return Err(ExecutionMismatch::OutOfOrder { position: i, expected: *exp, observed: *obs });
            }
        }
        use std::cmp::Ordering::*;
        match observed.len().cmp(&self.canonical_order.len()) {
            Less => Err(ExecutionMismatch::MissingPasses { count: self.canonical_order.len() - observed.len() }),
            Greater => Err(ExecutionMismatch::ExtraPasses { count: observed.len() - self.canonical_order.len() }),
            Equal => Ok(()),
        }
    }
}

/// The frozen voxygen pass-rank registry (Phase II seam; the drawer's declared
/// order). Ranks are spaced so future passes can slot between without renumber.
pub mod voxygen_ranks {
    pub const RAIN_OCCLUSION: u16 = 10;
    pub const SHADOW: u16 = 20;
    pub const FIRST: u16 = 30;
    pub const VOLUMETRIC: u16 = 40;
    pub const TRANSPARENT: u16 = 50;
    pub const BLOOM: u16 = 60;
    pub const THIRD: u16 = 70;
    // LIVE-EVIDENCE CORRECTION (first lavapipe cert run, 2026-07-22): the
    // ui_premultiply passes execute AFTER the third pass begins (observed on
    // every frame of both A/B runs), so the declared rank order follows the
    // real drawer, not the earlier source-reading guess.
    pub const UI_PREMULTIPLY: u16 = 80;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(rank: u16, d: u8, name: &str) -> PassNodeV1 {
        PassNodeV1 { pass_rank: rank, pass_digest: [d; 32], name: name.to_string() }
    }

    fn edge(b: u8, a: u8) -> HazardEdgeV1 {
        HazardEdgeV1 { before: [b; 32], after: [a; 32] }
    }

    #[test]
    fn insertion_permutation_cannot_change_order_or_digest() {
        let nodes = vec![node(30, 3, "first"), node(10, 1, "rain"), node(20, 2, "shadow")];
        let edges = vec![edge(1, 3), edge(2, 3)];
        let a = RendererPassGraphManifestV1::build(nodes.clone(), edges.clone()).unwrap();
        let mut nodes_rev = nodes;
        nodes_rev.reverse();
        let mut edges_rev = edges;
        edges_rev.reverse();
        let b = RendererPassGraphManifestV1::build(nodes_rev, edges_rev).unwrap();
        assert_eq!(a.canonical_order(), b.canonical_order());
        assert_eq!(a.manifest_digest(), b.manifest_digest());
        // Order respects ranks: rain(10) shadow(20) first(30).
        assert_eq!(a.canonical_order(), &[[1; 32], [2; 32], [3; 32]]);
    }

    #[test]
    fn simultaneous_ready_tie_resolves_by_rank_then_digest() {
        // Two rank-20 passes ready at once: digest breaks the tie.
        let nodes = vec![node(20, 9, "b-pass"), node(20, 4, "a-pass")];
        let m = RendererPassGraphManifestV1::build(nodes, vec![]).unwrap();
        assert_eq!(m.canonical_order(), &[[4; 32], [9; 32]]);
        // A hazard edge overrides rank order deterministically.
        let nodes = vec![node(20, 9, "b"), node(20, 4, "a")];
        let m = RendererPassGraphManifestV1::build(nodes, vec![edge(9, 4)]).unwrap();
        assert_eq!(m.canonical_order(), &[[9; 32], [4; 32]], "edge forces 9 before 4");
    }

    #[test]
    fn cycle_and_undeclared_are_typed() {
        let nodes = vec![node(10, 1, "a"), node(20, 2, "b")];
        assert_eq!(
            RendererPassGraphManifestV1::build(nodes.clone(), vec![edge(1, 2), edge(2, 1)]).unwrap_err(),
            PassGraphError::CycleDetected { remaining: 2 }
        );
        assert_eq!(
            RendererPassGraphManifestV1::build(nodes.clone(), vec![edge(1, 7)]).unwrap_err(),
            PassGraphError::UndeclaredPass { digest: [7; 32] }
        );
        let mut dup = nodes;
        dup.push(node(30, 1, "a-again"));
        assert!(matches!(
            RendererPassGraphManifestV1::build(dup, vec![]).unwrap_err(),
            PassGraphError::DuplicatePassDigest { .. }
        ));
    }

    #[test]
    fn duplicate_edge_does_not_wedge_the_indegree() {
        let nodes = vec![node(10, 1, "a"), node(20, 2, "b")];
        // The same hazard declared twice must still yield a complete order.
        let m = RendererPassGraphManifestV1::build(nodes, vec![edge(1, 2), edge(1, 2)]).unwrap();
        assert_eq!(m.canonical_order(), &[[1; 32], [2; 32]]);
    }

    #[test]
    fn execution_conformance_is_exact() {
        let nodes = vec![node(10, 1, "a"), node(20, 2, "b"), node(30, 3, "c")];
        let m = RendererPassGraphManifestV1::build(nodes, vec![]).unwrap();
        assert!(m.check_execution(&[[1; 32], [2; 32], [3; 32]]).is_ok());
        assert_eq!(
            m.check_execution(&[[2; 32], [1; 32], [3; 32]]).unwrap_err(),
            ExecutionMismatch::OutOfOrder { position: 0, expected: [1; 32], observed: [2; 32] }
        );
        assert_eq!(
            m.check_execution(&[[1; 32], [2; 32]]).unwrap_err(),
            ExecutionMismatch::MissingPasses { count: 1 }
        );
        assert_eq!(
            m.check_execution(&[[9; 32]]).unwrap_err(),
            ExecutionMismatch::UnknownPass { digest: [9; 32] }
        );
    }
}
