//! `APEX-T2.4` — canonical exact plugin dependency DAG (REAL packet
//! `PROJECT-BASTION-APEX-MICROSTEP-APEX-T2.4-CANONICAL-PLUGIN-DEPENDENCY-
//! DAG.md`; 80-case canary pin `2dc0bf14…`; domains `PluginCandidateSet`
//! = 18 and `PluginResolvedGraph` = 19, row-order registered).
//!
//! One pure batch resolver: strict-validated candidates in → canonical
//! Kahn (BTreeSet ready-set, ascending `PluginNodeKeyV1`) → resolved
//! graph with ordinal lists and domain-separated roots, or a typed
//! report with a DETERMINISTIC cycle witness (residual-sorted DFS,
//! rotate-min — packet section 9.4). Exact `(id, version)` satisfaction
//! only (policy 5.3); one version per plugin ID (policy 5.1); the ENTIRE
//! admitted set resolves — root selection/pruning is `T2.5`'s (5.2).

use super::manifest::{
    CanonicalPluginIdV1, PluginManifestAdmissionV1, PluginVersionV1, ValidatedPluginManifestV1,
    recompute_manifest_root,
};
use common::apex::digest::{
    DigestDomainIdV1, ProtocolDigestV1, digest_manifest_value_v1,
};
use common::apex::manifest::{CanonicalFieldMapV1, FieldIdV1, MachineTextV1, ManifestEncodeV1, ManifestValueV1};

pub const PLUGIN_RESOLVER_VERSION_V1: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PluginNodeKeyV1 {
    pub plugin_id: CanonicalPluginIdV1,
    pub plugin_version: PluginVersionV1,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PluginVersionMultiplicityPolicyV1 {
    SingleVersionPerPluginId,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PluginReadyOrderV1 {
    AscendingNodeKey,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PluginCycleWitnessPolicyV1 {
    ResidualSortedDfsRotateMinV1,
}

/// Mandatory injected limits (packet 5.5: no production defaults here).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginResolverLimitsV1 {
    pub max_node_count: u32,
    pub max_edge_count: u32,
    pub max_error_count: u32,
    pub max_cycle_witness_nodes: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginResolverPolicyV1 {
    pub resolver_version: u32,
    pub multiplicity: PluginVersionMultiplicityPolicyV1,
    pub ready_order: PluginReadyOrderV1,
    pub cycle_witness: PluginCycleWitnessPolicyV1,
    pub limits: PluginResolverLimitsV1,
    pub policy_root: ProtocolDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginDependencyEdgeV1 {
    pub dependency: PluginNodeKeyV1,
    pub dependent: PluginNodeKeyV1,
}

#[derive(Clone, Debug)]
pub struct ResolvedPluginNodeV1 {
    pub ordinal: u32,
    pub key: PluginNodeKeyV1,
    pub manifest_root: ProtocolDigestV1,
    pub archive_artifact: common::apex::digest::ArtifactIdentityV1,
    pub archive_semantic_root: ProtocolDigestV1,
    /// Ascending, duplicate-free ordinals of this node's dependencies.
    pub dependency_ordinals: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CycleWitnessV1 {
    pub nodes: Vec<PluginNodeKeyV1>,
}

#[derive(Clone, Debug)]
pub struct ResolvedPluginGraphV1 {
    pub resolver_version: u32,
    pub policy_root: ProtocolDigestV1,
    pub candidate_set_root: ProtocolDigestV1,
    pub nodes: Vec<ResolvedPluginNodeV1>,
    pub edges: Vec<PluginDependencyEdgeV1>,
    pub graph_root: ProtocolDigestV1,
}

/// Packet section 8 error families (the resolver-reachable set).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginResolutionErrorV1 {
    LegacyCandidateNotResolvable,
    ResolverPolicyInvalid,
    CandidateLimitExceeded,
    EdgeLimitExceeded,
    ErrorLimitExceeded,
    CycleWitnessLimitExceeded,
    DuplicateCandidate { key: PluginNodeKeyV1 },
    ConflictingCandidateArtifact { key: PluginNodeKeyV1 },
    ConflictingCandidateManifest { key: PluginNodeKeyV1 },
    CandidateAdmissionPolicyMismatch,
    MultipleVersionsForPluginId { plugin_id: String },
    InvalidCandidateManifestRoot { key: PluginNodeKeyV1 },
    MissingDependency { dependent: PluginNodeKeyV1, required: PluginNodeKeyV1 },
    DependencyVersionMismatch { dependent: PluginNodeKeyV1, required: PluginNodeKeyV1, available: Vec<PluginVersionV1> },
    SelfDependency { key: PluginNodeKeyV1 },
    DuplicateEdge { dependency: PluginNodeKeyV1, dependent: PluginNodeKeyV1 },
    IndegreeOverflow,
    DependencyCycle { witness: CycleWitnessV1, residual_root: ProtocolDigestV1 },
    GraphCanonicalizationFailure,
}

#[derive(Clone, Debug)]
pub struct PluginResolutionReportV1 {
    pub policy_root: ProtocolDigestV1,
    pub candidate_set_root: Option<ProtocolDigestV1>,
    /// Sorted, bounded by `limits.max_error_count`.
    pub errors: Vec<PluginResolutionErrorV1>,
}

#[derive(Clone, Debug)]
pub enum PluginResolutionTerminalV1 {
    Resolved(Box<ResolvedPluginGraphV1>),
    Rejected(Box<PluginResolutionReportV1>),
}

struct W(ManifestValueV1);
impl ManifestEncodeV1 for W {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, common::apex::manifest::ManifestCodecErrorV1> {
        Ok(self.0.clone())
    }
}

fn text(s: &str) -> ManifestValueV1 {
    ManifestValueV1::MachineText(MachineTextV1::new(s).expect("resolver identities are ASCII by construction"))
}

fn digest_value(v: &ProtocolDigestV1) -> ManifestValueV1 {
    ManifestValueV1::Bytes(v.bytes.as_array().to_vec())
}

fn fmap(entries: Vec<(u16, ManifestValueV1)>) -> ManifestValueV1 {
    ManifestValueV1::Map(
        CanonicalFieldMapV1::try_from_entries(entries.into_iter().map(|(i, v)| (FieldIdV1::new(i), v)).collect())
            .expect("static field ids are unique"),
    )
}

fn key_value(k: &PluginNodeKeyV1) -> ManifestValueV1 {
    fmap(vec![(0, text(k.plugin_id.as_str())), (1, text(&k.plugin_version.get().to_string()))])
}

fn domain_root(domain: DigestDomainIdV1, value: ManifestValueV1) -> Result<ProtocolDigestV1, PluginResolutionErrorV1> {
    let limits = super::archive_profile::plugin_archive_limits_v1();
    digest_manifest_value_v1(domain, &W(value), &limits).map_err(|_| PluginResolutionErrorV1::GraphCanonicalizationFailure)
}

/// The one pure batch resolver (packet T2.4.14). Never touches Wasmtime,
/// ECS, caches, or global assets — candidates in, terminal out.
pub fn resolve_plugin_graph_v1(
    candidates: Vec<PluginManifestAdmissionV1>,
    policy: &PluginResolverPolicyV1,
) -> PluginResolutionTerminalV1 {
    let reject = |candidate_set_root: Option<ProtocolDigestV1>, mut errors: Vec<PluginResolutionErrorV1>| {
        // §5.4: aggregate up to the injected limit; exceeding it is ITSELF
        // recorded, never silently dropped.
        if errors.len() > policy.limits.max_error_count as usize {
            errors.truncate((policy.limits.max_error_count as usize).saturating_sub(1));
            errors.push(PluginResolutionErrorV1::ErrorLimitExceeded);
        }
        PluginResolutionTerminalV1::Rejected(Box::new(PluginResolutionReportV1 {
            policy_root: policy.policy_root.clone(),
            candidate_set_root,
            errors,
        }))
    };

    if policy.resolver_version != PLUGIN_RESOLVER_VERSION_V1 {
        return reject(None, vec![PluginResolutionErrorV1::ResolverPolicyInvalid]);
    }
    if candidates.len() as u32 > policy.limits.max_node_count {
        return reject(None, vec![PluginResolutionErrorV1::CandidateLimitExceeded]);
    }

    // §9.1 — candidate admission: legacy rejected as a class; sorted;
    // repeated keys and multi-version IDs rejected; carried roots
    // re-verified (never trusted).
    let mut validated: Vec<ValidatedPluginManifestV1> = Vec::with_capacity(candidates.len());
    for c in candidates {
        match c {
            PluginManifestAdmissionV1::ValidatedV1(v) => validated.push(*v),
            PluginManifestAdmissionV1::ObservedLegacyV0(_) => {
                return reject(None, vec![PluginResolutionErrorV1::LegacyCandidateNotResolvable]);
            },
        }
    }
    validated.sort_by(|a, b| {
        a.plugin_id
            .as_str()
            .cmp(b.plugin_id.as_str())
            .then_with(|| a.plugin_version.get().cmp(b.plugin_version.get()))
    });
    let mut admission_errors = Vec::new();
    // Every candidate must have been admitted under ONE manifest policy
    // (PDG-030: mixed admission-policy roots cannot resolve together).
    if validated.windows(2).any(|p| p[0].admission_policy_root != p[1].admission_policy_root) {
        admission_errors.push(PluginResolutionErrorV1::CandidateAdmissionPolicyMismatch);
    }
    for pair in validated.windows(2) {
        if pair[0].plugin_id == pair[1].plugin_id {
            if pair[0].plugin_version == pair[1].plugin_version {
                let key = PluginNodeKeyV1 {
                    plugin_id: pair[1].plugin_id.clone(),
                    plugin_version: pair[1].plugin_version.clone(),
                };
                // Same key: distinguish exact duplicate from CONFLICTING
                // content under one identity (PDG-023/024 — the sharper
                // finding outranks the generic duplicate).
                if pair[0].archive_artifact != pair[1].archive_artifact {
                    admission_errors.push(PluginResolutionErrorV1::ConflictingCandidateArtifact { key });
                } else if pair[0].manifest_root != pair[1].manifest_root {
                    admission_errors.push(PluginResolutionErrorV1::ConflictingCandidateManifest { key });
                } else {
                    admission_errors.push(PluginResolutionErrorV1::DuplicateCandidate { key });
                }
            } else {
                admission_errors.push(PluginResolutionErrorV1::MultipleVersionsForPluginId {
                    plugin_id: pair[1].plugin_id.as_str().to_owned(),
                });
            }
        }
    }
    for v in &validated {
        match recompute_manifest_root(v) {
            Ok(root) if root == v.manifest_root => {},
            _ => admission_errors.push(PluginResolutionErrorV1::InvalidCandidateManifestRoot {
                key: PluginNodeKeyV1 { plugin_id: v.plugin_id.clone(), plugin_version: v.plugin_version.clone() },
            }),
        }
    }
    if !admission_errors.is_empty() {
        return reject(None, admission_errors);
    }

    // §9.1.6 — candidate-set root under PluginCandidateSet (= 18).
    let candidate_items: Vec<ManifestValueV1> = validated
        .iter()
        .map(|v| {
            fmap(vec![
                (0, key_value(&PluginNodeKeyV1 {
                    plugin_id: v.plugin_id.clone(),
                    plugin_version: v.plugin_version.clone(),
                })),
                (1, digest_value(&v.manifest_root)),
            ])
        })
        .collect();
    let candidate_set_root = match domain_root(DigestDomainIdV1::PluginCandidateSet, ManifestValueV1::Array(candidate_items)) {
        Ok(r) => r,
        Err(e) => return reject(None, vec![e]),
    };

    // §9.2 — edges (dependency → dependent), exact-key satisfaction only,
    // missing aggregated sorted+bounded (§5.4).
    let index: std::collections::BTreeMap<&str, usize> =
        validated.iter().enumerate().map(|(i, v)| (v.plugin_id.as_str(), i)).collect();
    let key_of = |v: &ValidatedPluginManifestV1| PluginNodeKeyV1 {
        plugin_id: v.plugin_id.clone(),
        plugin_version: v.plugin_version.clone(),
    };
    let mut dep_errors = Vec::new();
    let mut edges: Vec<(usize, usize)> = Vec::new(); // (dependency idx, dependent idx)
    for (di, v) in validated.iter().enumerate() {
        for dep in &v.dependencies {
            if dep.plugin_id == v.plugin_id {
                dep_errors.push(PluginResolutionErrorV1::SelfDependency { key: key_of(v) });
                continue;
            }
            match index.get(dep.plugin_id.as_str()) {
                Some(&target) if validated[target].plugin_version == dep.version => edges.push((target, di)),
                Some(&target) => dep_errors.push(PluginResolutionErrorV1::DependencyVersionMismatch {
                    dependent: key_of(v),
                    required: PluginNodeKeyV1 { plugin_id: dep.plugin_id.clone(), plugin_version: dep.version.clone() },
                    available: vec![validated[target].plugin_version.clone()],
                }),
                None => dep_errors.push(PluginResolutionErrorV1::MissingDependency {
                    dependent: key_of(v),
                    required: PluginNodeKeyV1 { plugin_id: dep.plugin_id.clone(), plugin_version: dep.version.clone() },
                }),
            }
        }
    }
    if edges.len() as u32 > policy.limits.max_edge_count {
        return reject(Some(candidate_set_root), vec![PluginResolutionErrorV1::EdgeLimitExceeded]);
    }
    if !dep_errors.is_empty() {
        dep_errors.sort_by_key(|e| format!("{e:?}"));
        return reject(Some(candidate_set_root), dep_errors);
    }

    // §9.3 — canonical Kahn: BTreeSet ready-set keyed by index (validated
    // is key-sorted, so index order IS ascending node-key order).
    let n = validated.len();
    // Duplicate edges cannot arise from a valid T2.3 manifest (deps are
    // deduped there) — one reaching here is an invariant violation and is
    // REJECTED, not silently deduped (PDG-039).
    {
        let mut seen = std::collections::BTreeSet::new();
        for &(dep, dependent) in &edges {
            if !seen.insert((dep, dependent)) {
                return reject(
                    Some(candidate_set_root),
                    vec![PluginResolutionErrorV1::DuplicateEdge {
                        dependency: key_of(&validated[dep]),
                        dependent: key_of(&validated[dependent]),
                    }],
                );
            }
        }
    }
    let mut indegree = vec![0u32; n];
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(dep, dependent) in &edges {
        outgoing[dep].push(dependent);
        indegree[dependent] = match indegree[dependent].checked_add(1) {
            Some(v) => v,
            None => return reject(Some(candidate_set_root), vec![PluginResolutionErrorV1::IndegreeOverflow]),
        };
    }
    for out in &mut outgoing {
        out.sort_unstable();
    }
    let mut ready: std::collections::BTreeSet<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut ordinal_of = vec![u32::MAX; n];
    let mut emitted = 0u32;
    while let Some(&node) = ready.iter().next() {
        ready.remove(&node);
        ordinal_of[node] = emitted;
        emitted += 1;
        for &dependent in &outgoing[node] {
            indegree[dependent] -= 1;
            if indegree[dependent] == 0 {
                ready.insert(dependent);
            }
        }
    }

    if (emitted as usize) < n {
        // §9.4 — deterministic cycle witness over the residual subgraph.
        let residual: Vec<usize> = (0..n).filter(|&i| ordinal_of[i] == u32::MAX).collect();
        let residual_set: std::collections::BTreeSet<usize> = residual.iter().copied().collect();
        let witness = cycle_witness(&residual, &residual_set, &outgoing);
        if witness.len() as u32 > policy.limits.max_cycle_witness_nodes {
            return reject(Some(candidate_set_root), vec![PluginResolutionErrorV1::CycleWitnessLimitExceeded]);
        }
        let residual_items: Vec<ManifestValueV1> =
            residual.iter().map(|&i| key_value(&key_of(&validated[i]))).collect();
        let residual_root = match domain_root(DigestDomainIdV1::PluginResolvedGraph, ManifestValueV1::Array(residual_items)) {
            Ok(r) => r,
            Err(e) => return reject(Some(candidate_set_root), vec![e]),
        };
        return reject(
            Some(candidate_set_root),
            vec![PluginResolutionErrorV1::DependencyCycle {
                witness: CycleWitnessV1 { nodes: witness.into_iter().map(|i| key_of(&validated[i])).collect() },
                residual_root,
            }],
        );
    }

    // §9.5 — resolved nodes in ordinal order + sorted edges + graph root.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| ordinal_of[i]);
    let mut nodes = Vec::with_capacity(n);
    for &i in &order {
        let v = &validated[i];
        let mut dependency_ordinals: Vec<u32> = v
            .dependencies
            .iter()
            .map(|d| ordinal_of[index[d.plugin_id.as_str()]])
            .collect();
        dependency_ordinals.sort_unstable();
        dependency_ordinals.dedup();
        nodes.push(ResolvedPluginNodeV1 {
            ordinal: ordinal_of[i],
            key: key_of(v),
            manifest_root: v.manifest_root.clone(),
            archive_artifact: v.archive_artifact.clone(),
            archive_semantic_root: v.archive_semantic_root.clone(),
            dependency_ordinals,
        });
    }
    let mut edge_objs: Vec<PluginDependencyEdgeV1> = edges
        .iter()
        .map(|&(dep, dependent)| PluginDependencyEdgeV1 {
            dependency: key_of(&validated[dep]),
            dependent: key_of(&validated[dependent]),
        })
        .collect();
    edge_objs.sort_by(|a, b| a.dependency.cmp(&b.dependency).then_with(|| a.dependent.cmp(&b.dependent)));
    edge_objs.dedup();

    let node_items: Vec<ManifestValueV1> = nodes
        .iter()
        .map(|node| {
            fmap(vec![
                (0, ManifestValueV1::Unsigned(node.ordinal as u64)),
                (1, key_value(&node.key)),
                (2, digest_value(&node.manifest_root)),
                (3, ManifestValueV1::Bytes(node.archive_artifact.digest.bytes.as_array().to_vec())),
                (4, digest_value(&node.archive_semantic_root)),
                (5, ManifestValueV1::Array(
                    node.dependency_ordinals.iter().map(|&o| ManifestValueV1::Unsigned(o as u64)).collect(),
                )),
            ])
        })
        .collect();
    let edge_items: Vec<ManifestValueV1> = edge_objs
        .iter()
        .map(|e| fmap(vec![(0, key_value(&e.dependency)), (1, key_value(&e.dependent))]))
        .collect();
    let graph_value = fmap(vec![
        (0, ManifestValueV1::Unsigned(PLUGIN_RESOLVER_VERSION_V1 as u64)),
        (1, digest_value(&policy.policy_root)),
        (2, digest_value(&candidate_set_root)),
        (3, ManifestValueV1::Array(node_items)),
        (4, ManifestValueV1::Array(edge_items)),
    ]);
    let graph_root = match domain_root(DigestDomainIdV1::PluginResolvedGraph, graph_value) {
        Ok(r) => r,
        Err(e) => return reject(Some(candidate_set_root), vec![e]),
    };

    PluginResolutionTerminalV1::Resolved(Box::new(ResolvedPluginGraphV1 {
        resolver_version: PLUGIN_RESOLVER_VERSION_V1,
        policy_root: policy.policy_root.clone(),
        candidate_set_root,
        nodes,
        edges: edge_objs,
        graph_root,
    }))
}

/// §9.4 — three-color DFS over the residual subgraph, smallest unvisited
/// key first, neighbors ascending; first gray-edge closes the cycle;
/// prefix removed; rotated so the smallest key leads. Pure function of
/// the residual SET — input order can never move the witness.
fn cycle_witness(
    residual: &[usize],
    residual_set: &std::collections::BTreeSet<usize>,
    outgoing: &[Vec<usize>],
) -> Vec<usize> {
    #[derive(Copy, Clone, PartialEq)]
    enum Color {
        White,
        Gray,
        Black,
    }
    let max = outgoing.len();
    let mut color = vec![Color::White; max];
    for &start in residual {
        if color[start] != Color::White {
            continue;
        }
        // Iterative DFS with an explicit stack of (node, neighbor-iter pos).
        let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
        color[start] = Color::Gray;
        while let Some(&mut (node, ref mut pos)) = stack.last_mut() {
            let neighbors: Vec<usize> =
                outgoing[node].iter().copied().filter(|t| residual_set.contains(t)).collect();
            if *pos < neighbors.len() {
                let next = neighbors[*pos];
                *pos += 1;
                match color[next] {
                    Color::Gray => {
                        // Cycle found: from `next` to the top of the stack.
                        let cycle_start = stack.iter().position(|&(n, _)| n == next).expect("gray node is on stack");
                        let mut cycle: Vec<usize> = stack[cycle_start..].iter().map(|&(n, _)| n).collect();
                        // Rotate so the smallest node key (= smallest index,
                        // since validated is key-sorted) leads.
                        let min_pos = cycle
                            .iter()
                            .enumerate()
                            .min_by_key(|&(_, &n)| n)
                            .map(|(p, _)| p)
                            .expect("cycle nonempty");
                        cycle.rotate_left(min_pos);
                        return cycle;
                    },
                    Color::White => {
                        color[next] = Color::Gray;
                        stack.push((next, 0));
                    },
                    Color::Black => {},
                }
            } else {
                color[node] = Color::Black;
                stack.pop();
            }
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::super::archive_profile::CanonicalEntryV1;
    use super::super::manifest::*;
    use super::*;
    use common::apex::digest::{digest_canonical_bytes_v1, hash_artifact_bytes_v1};
    use common::apex::manifest::CanonicalPathV1;

    fn limits() -> PluginManifestLimitsV1 {
        PluginManifestLimitsV1 {
            policy_id: MachineTextV1::new("apex-t2-4-test-limits-v1").unwrap(),
            max_manifest_bytes: 1 << 14,
            max_plugin_id_bytes: 64,
            max_display_name_bytes: 64,
            max_module_count: 8,
            max_dependency_count: 8,
            max_runtime_claim_modes: 3,
            max_command_claims_per_mode: 8,
            max_animation_claims_per_mode: 8,
            max_asset_root_count: 4,
            max_runtime_key_bytes: 64,
        }
    }

    fn policy() -> PluginResolverPolicyV1 {
        PluginResolverPolicyV1 {
            resolver_version: PLUGIN_RESOLVER_VERSION_V1,
            multiplicity: PluginVersionMultiplicityPolicyV1::SingleVersionPerPluginId,
            ready_order: PluginReadyOrderV1::AscendingNodeKey,
            cycle_witness: PluginCycleWitnessPolicyV1::ResidualSortedDfsRotateMinV1,
            limits: PluginResolverLimitsV1 {
                max_node_count: 64,
                max_edge_count: 256,
                max_error_count: 16,
                max_cycle_witness_nodes: 16,
            },
            policy_root: digest_canonical_bytes_v1(DigestDomainIdV1::PluginResolvedGraph, b"test-policy", 1 << 20)
                .unwrap(),
        }
    }

    /// Build a candidate through the REAL T2.3 pipeline.
    fn candidate(id: &str, version: &str, deps: &[(&str, &str)]) -> PluginManifestAdmissionV1 {
        // Top-level arrays must precede any table header in TOML.
        let mut toml = String::from("manifest_version = 1\nmodules = []\ndependencies = [\n");
        for (did, dver) in deps {
            toml.push_str(&format!("  {{ id = \"{did}\", version = \"{dver}\" }},\n"));
        }
        toml.push_str(&format!(
            "]\n\n[plugin]\nid = \"{id}\"\nversion = \"{version}\"\nhost_api = \"veloren:plugin@0.0.1\"\n\n"
        ));
        toml.push_str("[claims]\nasset_roots = []\n\n");
        toml.push_str("[[claims.runtime]]\nmode = \"server\"\ncommands = []\nanimations = []\n");
        let ns = vec![CanonicalEntryV1 {
            path: CanonicalPathV1::new("plugin.toml").unwrap(),
            portability_key: MachineTextV1::new("plugin.toml").unwrap(),
            size_bytes: toml.len() as u64,
            content_sha256: [7; 32],
        }];
        let art = hash_artifact_bytes_v1(toml.as_bytes());
        let root = digest_canonical_bytes_v1(DigestDomainIdV1::PluginManifest, b"archive-root", 1 << 20).unwrap();
        validate_plugin_manifest_v1(
            toml.as_bytes(),
            &ns,
            &art,
            &root,
            &limits(),
            PluginManifestEnforcementModeV1::StrictV1,
            root.clone(),
        )
        .expect("test candidate must validate")
    }

    fn graph_root_of(candidates: Vec<PluginManifestAdmissionV1>) -> ProtocolDigestV1 {
        match resolve_plugin_graph_v1(candidates, &policy()) {
            PluginResolutionTerminalV1::Resolved(g) => g.graph_root.clone(),
            PluginResolutionTerminalV1::Rejected(r) => panic!("{:?}", r.errors),
        }
    }

    #[test]
    fn diamond_resolves_with_canonical_ordinals_and_permutation_invariance() {
        let mk = || {
            vec![
                candidate("x:a", "1.0.0", &[]),
                candidate("x:b", "1.0.0", &[("x:a", "1.0.0")]),
                candidate("x:c", "1.0.0", &[("x:a", "1.0.0")]),
                candidate("x:d", "1.0.0", &[("x:b", "1.0.0"), ("x:c", "1.0.0")]),
            ]
        };
        let g = match resolve_plugin_graph_v1(mk(), &policy()) {
            PluginResolutionTerminalV1::Resolved(g) => g,
            PluginResolutionTerminalV1::Rejected(r) => panic!("{:?}", r.errors),
        };
        let names: Vec<&str> = g.nodes.iter().map(|n| n.key.plugin_id.as_str()).collect();
        assert_eq!(names, vec!["x:a", "x:b", "x:c", "x:d"], "Kahn + ascending-key ready order");
        assert_eq!(g.nodes[3].dependency_ordinals, vec![1, 2]);
        assert_eq!(g.edges.len(), 4);

        let base = graph_root_of(mk());
        let mut reordered = mk();
        reordered.reverse();
        assert_eq!(base, graph_root_of(reordered));
        let mut shuffled = mk();
        shuffled.swap(0, 2);
        shuffled.swap(1, 3);
        assert_eq!(base, graph_root_of(shuffled));
    }

    #[test]
    fn admission_and_dependency_errors_bite() {
        let p = policy();
        let r = resolve_plugin_graph_v1(vec![candidate("x:a", "1.0.0", &[]), candidate("x:a", "1.0.0", &[])], &p);
        assert!(matches!(r, PluginResolutionTerminalV1::Rejected(ref rep)
            if matches!(rep.errors[0], PluginResolutionErrorV1::DuplicateCandidate { .. })));

        let r = resolve_plugin_graph_v1(vec![candidate("x:a", "1.0.0", &[]), candidate("x:a", "2.0.0", &[])], &p);
        assert!(matches!(r, PluginResolutionTerminalV1::Rejected(ref rep)
            if matches!(rep.errors[0], PluginResolutionErrorV1::MultipleVersionsForPluginId { .. })));

        let r = resolve_plugin_graph_v1(
            vec![candidate("x:a", "1.0.0", &[("x:gone", "1.0.0"), ("x:also-gone", "1.0.0")])],
            &p,
        );
        match r {
            PluginResolutionTerminalV1::Rejected(rep) => {
                assert_eq!(rep.errors.len(), 2);
                assert!(rep.errors.iter().all(|e| matches!(e, PluginResolutionErrorV1::MissingDependency { .. })));
                assert!(rep.candidate_set_root.is_some(), "candidate-set root computed before dep validation");
            },
            other => panic!("{other:?}"),
        }

        let r = resolve_plugin_graph_v1(
            vec![candidate("x:a", "2.0.0", &[]), candidate("x:b", "1.0.0", &[("x:a", "1.0.0")])],
            &p,
        );
        assert!(matches!(r, PluginResolutionTerminalV1::Rejected(ref rep)
            if matches!(&rep.errors[0], PluginResolutionErrorV1::DependencyVersionMismatch { available, .. }
                if available.len() == 1)));

        let legacy = validate_plugin_manifest_v1(
            b"name = \"old\"\n",
            &[],
            &hash_artifact_bytes_v1(b"x"),
            &p.policy_root,
            &limits(),
            PluginManifestEnforcementModeV1::ObserveLegacy,
            p.policy_root.clone(),
        )
        .unwrap();
        let r = resolve_plugin_graph_v1(vec![legacy], &p);
        assert!(matches!(r, PluginResolutionTerminalV1::Rejected(ref rep)
            if matches!(rep.errors[0], PluginResolutionErrorV1::LegacyCandidateNotResolvable)));

        let mut tampered = match candidate("x:a", "1.0.0", &[]) {
            PluginManifestAdmissionV1::ValidatedV1(v) => v,
            _ => unreachable!(),
        };
        tampered.manifest_root = p.policy_root.clone();
        let r = resolve_plugin_graph_v1(vec![PluginManifestAdmissionV1::ValidatedV1(tampered)], &p);
        assert!(matches!(r, PluginResolutionTerminalV1::Rejected(ref rep)
            if matches!(rep.errors[0], PluginResolutionErrorV1::InvalidCandidateManifestRoot { .. })));
    }

    #[test]
    fn cycle_witness_is_deterministic_and_rotate_min() {
        let mk = || {
            vec![
                candidate("x:a", "1.0.0", &[("x:c", "1.0.0")]),
                candidate("x:b", "1.0.0", &[("x:a", "1.0.0")]),
                candidate("x:c", "1.0.0", &[("x:b", "1.0.0")]),
                candidate("x:standalone", "1.0.0", &[]),
            ]
        };
        let witness_of = |cands: Vec<PluginManifestAdmissionV1>| match resolve_plugin_graph_v1(cands, &policy()) {
            PluginResolutionTerminalV1::Rejected(rep) => match &rep.errors[0] {
                PluginResolutionErrorV1::DependencyCycle { witness, .. } => {
                    witness.nodes.iter().map(|k| k.plugin_id.as_str().to_owned()).collect::<Vec<_>>()
                },
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        };
        let w1 = witness_of(mk());
        let mut reordered = mk();
        reordered.reverse();
        let w2 = witness_of(reordered);
        assert_eq!(w1, w2, "witness is a pure function of the residual set");
        assert_eq!(w1[0], "x:a", "rotate-min: smallest key leads");
        assert_eq!(w1.len(), 3, "standalone node is not in the witness");
    }
}
