//! `APEX-T2.4.17` — the 80-case catalog acceptance runner (the T2.2/T2.3
//! pattern): pin re-verified per run, total terminal coverage
//! (driven / structurally-claimed with reasons; unclaimed fails), error
//! families driven through the REAL resolver over REAL T2.3-validated
//! candidates, and the ROOT-MISMATCH family proven as SENSITIVITY (every
//! named mutation MUST move the corresponding root).

#![cfg(feature = "plugins")]

use common::apex::digest::{DigestDomainIdV1, ProtocolDigestV1, digest_canonical_bytes_v1, hash_artifact_bytes_v1};
use common::apex::manifest::{CanonicalPathV1, MachineTextV1};
use sha2::Digest;
use veloren_common_state::plugin::archive_profile::CanonicalEntryV1;
use veloren_common_state::plugin::manifest::*;
use veloren_common_state::plugin::resolver::*;

const CATALOG: &str = "PROJECT-BASTION-APEX-T2.4-PLUGIN-DEPENDENCY-DAG-CANARIES-v1.json";
const PIN: &str = "2dc0bf140d7d015f6d4ded62d18c9240976bd85a3fe05f6669bfcee815f1d821";

fn mlimits() -> PluginManifestLimitsV1 {
    PluginManifestLimitsV1 {
        policy_id: MachineTextV1::new("apex-t2-4-catalog-mlimits-v1").unwrap(),
        max_manifest_bytes: 1 << 14,
        max_plugin_id_bytes: 64,
        max_display_name_bytes: 64,
        max_module_count: 8,
        max_dependency_count: 16,
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
        policy_root: digest_canonical_bytes_v1(DigestDomainIdV1::PluginResolvedGraph, b"catalog-policy", 1 << 20)
            .unwrap(),
    }
}

fn candidate(id: &str, version: &str, deps: &[(&str, &str)]) -> PluginManifestAdmissionV1 {
    let mut toml = String::from("manifest_version = 1\nmodules = []\ndependencies = [\n");
    for (did, dver) in deps {
        toml.push_str(&format!("  {{ id = \"{did}\", version = \"{dver}\" }},\n"));
    }
    toml.push_str(&format!(
        "]\n\n[plugin]\nid = \"{id}\"\nversion = \"{version}\"\nhost_api = \"veloren:plugin@0.0.1\"\n\n"
    ));
    toml.push_str("[claims]\nasset_roots = []\n\n[[claims.runtime]]\nmode = \"server\"\ncommands = []\nanimations = []\n");
    let ns = vec![CanonicalEntryV1 {
        path: CanonicalPathV1::new("plugin.toml").unwrap(),
        portability_key: MachineTextV1::new("plugin.toml").unwrap(),
        size_bytes: toml.len() as u64,
        content_sha256: [7; 32],
    }];
    let art = hash_artifact_bytes_v1(toml.as_bytes());
    let root = digest_canonical_bytes_v1(DigestDomainIdV1::PluginManifest, b"admission-policy", 1 << 20).unwrap();
    validate_plugin_manifest_v1(
        toml.as_bytes(),
        &ns,
        &art,
        &root,
        &mlimits(),
        PluginManifestEnforcementModeV1::StrictV1,
        root.clone(),
    )
    .expect("catalog candidate must validate")
}

fn unbox(c: PluginManifestAdmissionV1) -> ValidatedPluginManifestV1 {
    match c {
        PluginManifestAdmissionV1::ValidatedV1(v) => *v,
        other => panic!("{other:?}"),
    }
}

fn first_error(r: PluginResolutionTerminalV1) -> PluginResolutionErrorV1 {
    match r {
        PluginResolutionTerminalV1::Rejected(rep) => rep.errors[0].clone(),
        other => panic!("expected rejection, got {other:?}"),
    }
}

fn resolved_root(cands: Vec<PluginManifestAdmissionV1>, p: &PluginResolverPolicyV1) -> ProtocolDigestV1 {
    match resolve_plugin_graph_v1(cands, p) {
        PluginResolutionTerminalV1::Resolved(g) => g.graph_root.clone(),
        PluginResolutionTerminalV1::Rejected(r) => panic!("{:?}", r.errors),
    }
}

fn diamond() -> Vec<PluginManifestAdmissionV1> {
    vec![
        candidate("x:a", "1.0.0", &[]),
        candidate("x:b", "1.0.0", &[("x:a", "1.0.0")]),
        candidate("x:c", "1.0.0", &[("x:a", "1.0.0")]),
        candidate("x:d", "1.0.0", &[("x:b", "1.0.0"), ("x:c", "1.0.0")]),
    ]
}

/// terminal → driven verdict (true = behaves as the catalog requires).
fn drive(terminal: &str) -> Option<bool> {
    use PluginResolutionErrorV1 as E;
    let p = policy();
    Some(match terminal {
        "RESOLVED" => matches!(resolve_plugin_graph_v1(diamond(), &p), PluginResolutionTerminalV1::Resolved(_)),
        "PERMUTATION-CAMPAIGN-PASS" => {
            let base = resolved_root(diamond(), &p);
            let mut r1 = diamond();
            r1.reverse();
            let mut r2 = diamond();
            r2.swap(0, 3);
            r2.swap(1, 2);
            base == resolved_root(r1, &p) && base == resolved_root(r2, &p)
        },
        "DEPENDENCY-CYCLE" => {
            let cands = vec![
                candidate("x:a", "1.0.0", &[("x:c", "1.0.0")]),
                candidate("x:b", "1.0.0", &[("x:a", "1.0.0")]),
                candidate("x:c", "1.0.0", &[("x:b", "1.0.0")]),
            ];
            matches!(first_error(resolve_plugin_graph_v1(cands, &p)), E::DependencyCycle { .. })
        },
        "MISSING-DEPENDENCIES" => matches!(
            first_error(resolve_plugin_graph_v1(vec![candidate("x:a", "1.0.0", &[("x:gone", "1.0.0")])], &p)),
            E::MissingDependency { .. }
        ),
        "DEPENDENCY-VERSION-MISMATCH" => matches!(
            first_error(resolve_plugin_graph_v1(
                vec![candidate("x:a", "2.0.0", &[]), candidate("x:b", "1.0.0", &[("x:a", "1.0.0")])],
                &p
            )),
            E::DependencyVersionMismatch { .. }
        ),
        "DUPLICATE-CANDIDATE" => matches!(
            first_error(resolve_plugin_graph_v1(
                vec![candidate("x:a", "1.0.0", &[]), candidate("x:a", "1.0.0", &[])],
                &p
            )),
            E::DuplicateCandidate { .. }
        ),
        "MULTIPLE-VERSIONS-FOR-PLUGIN-ID" => matches!(
            first_error(resolve_plugin_graph_v1(
                vec![candidate("x:a", "1.0.0", &[]), candidate("x:a", "2.0.0", &[])],
                &p
            )),
            E::MultipleVersionsForPluginId { .. }
        ),
        "LEGACY-CANDIDATE-NOT-RESOLVABLE" => {
            let legacy = validate_plugin_manifest_v1(
                b"name = \"old\"\n",
                &[],
                &hash_artifact_bytes_v1(b"x"),
                &p.policy_root,
                &mlimits(),
                PluginManifestEnforcementModeV1::ObserveLegacy,
                p.policy_root.clone(),
            )
            .unwrap();
            matches!(first_error(resolve_plugin_graph_v1(vec![legacy], &p)), E::LegacyCandidateNotResolvable)
        },
        "SELF-DEPENDENCY" => {
            // Unreachable from a T2.3-valid candidate; drive the defensive
            // check via a tampered dependencies list.
            let mut v = unbox(candidate("x:a", "1.0.0", &[("x:other", "1.0.0")]));
            let other = unbox(candidate("x:other", "1.0.0", &[]));
            v.dependencies[0].plugin_id = v.plugin_id.clone();
            v.dependencies[0].version = v.plugin_version.clone();
            // Recompute root so admission passes and the self-dep check is
            // what actually bites.
            v.manifest_root = recompute_manifest_root(&v).unwrap();
            matches!(
                first_error(resolve_plugin_graph_v1(
                    vec![
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(v)),
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(other))
                    ],
                    &p
                )),
                E::SelfDependency { .. }
            )
        },
        "DUPLICATE-EDGE" => {
            let mut v = unbox(candidate("x:a", "1.0.0", &[("x:b", "1.0.0")]));
            let b = unbox(candidate("x:b", "1.0.0", &[]));
            let dup = v.dependencies[0].clone();
            v.dependencies.push(dup);
            v.manifest_root = recompute_manifest_root(&v).unwrap();
            matches!(
                first_error(resolve_plugin_graph_v1(
                    vec![
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(v)),
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(b))
                    ],
                    &p
                )),
                E::DuplicateEdge { .. }
            )
        },
        "INVALID-CANDIDATE-ROOT" => {
            let mut v = unbox(candidate("x:a", "1.0.0", &[]));
            v.manifest_root = p.policy_root.clone(); // wrong domain AND wrong bytes
            matches!(
                first_error(resolve_plugin_graph_v1(vec![PluginManifestAdmissionV1::ValidatedV1(Box::new(v))], &p)),
                E::InvalidCandidateManifestRoot { .. }
            )
        },
        "CONFLICTING-CANDIDATE-ARTIFACT" => {
            let a1 = unbox(candidate("x:a", "1.0.0", &[]));
            let mut a2 = a1.clone();
            a2.archive_artifact = hash_artifact_bytes_v1(b"different bytes");
            matches!(
                first_error(resolve_plugin_graph_v1(
                    vec![
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(a1)),
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(a2))
                    ],
                    &p
                )),
                E::ConflictingCandidateArtifact { .. }
            )
        },
        "CONFLICTING-CANDIDATE-MANIFEST" => {
            // Same key + same artifact, different manifest content root: a
            // recomputable different root needs different content — use a
            // dependency delta and re-root both.
            let a1 = unbox(candidate("x:a", "1.0.0", &[]));
            let mut a2 = unbox(candidate("x:a", "1.0.0", &[("x:b", "1.0.0")]));
            a2.archive_artifact = a1.archive_artifact.clone();
            a2.manifest_root = recompute_manifest_root(&a2).unwrap();
            let b = unbox(candidate("x:b", "1.0.0", &[]));
            matches!(
                first_error(resolve_plugin_graph_v1(
                    vec![
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(a1)),
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(a2)),
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(b))
                    ],
                    &p
                )),
                E::ConflictingCandidateManifest { .. }
            )
        },
        "POLICY-MISMATCH" => {
            let a = unbox(candidate("x:a", "1.0.0", &[]));
            let mut b = unbox(candidate("x:b", "1.0.0", &[]));
            b.admission_policy_root =
                digest_canonical_bytes_v1(DigestDomainIdV1::PluginManifest, b"other-policy", 1 << 20).unwrap();
            matches!(
                first_error(resolve_plugin_graph_v1(
                    vec![
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(a)),
                        PluginManifestAdmissionV1::ValidatedV1(Box::new(b))
                    ],
                    &p
                )),
                E::CandidateAdmissionPolicyMismatch
            )
        },
        "LIMIT-EXCEEDED" => {
            let mut small = p.clone();
            small.limits.max_node_count = 1;
            matches!(
                first_error(resolve_plugin_graph_v1(diamond(), &small)),
                E::CandidateLimitExceeded
            )
        },
        "EDGE-LIMIT-EXCEEDED" => {
            let mut small = p.clone();
            small.limits.max_edge_count = 1;
            matches!(first_error(resolve_plugin_graph_v1(diamond(), &small)), E::EdgeLimitExceeded)
        },
        "ERROR-LIMIT-EXCEEDED" => {
            let mut small = p.clone();
            small.limits.max_error_count = 2;
            let cands = vec![candidate(
                "x:a",
                "1.0.0",
                &[("x:g1", "1.0.0"), ("x:g2", "1.0.0"), ("x:g3", "1.0.0"), ("x:g4", "1.0.0")],
            )];
            match resolve_plugin_graph_v1(cands, &small) {
                PluginResolutionTerminalV1::Rejected(rep) => {
                    rep.errors.len() == 2 && matches!(rep.errors.last(), Some(E::ErrorLimitExceeded))
                },
                other => panic!("{other:?}"),
            }
        },
        "ROOT-MISMATCH" => {
            // Sensitivity family: every named dimension MUST move a root.
            let base = resolved_root(diamond(), &p);
            // Node content change.
            let mut altered = diamond();
            altered[0] = candidate("x:a", "1.0.1", &[]);
            altered[1] = candidate("x:b", "1.0.0", &[("x:a", "1.0.1")]);
            altered[2] = candidate("x:c", "1.0.0", &[("x:a", "1.0.1")]);
            let moved_content = resolved_root(altered, &p) != base;
            // Policy root change.
            let mut p2 = p.clone();
            p2.policy_root =
                digest_canonical_bytes_v1(DigestDomainIdV1::PluginResolvedGraph, b"other", 1 << 20).unwrap();
            let moved_policy = resolved_root(diamond(), &p2) != base;
            // Candidate omission.
            let subset: Vec<_> = diamond().into_iter().take(3).collect();
            let moved_subset = resolved_root(subset, &p) != base;
            moved_content && moved_policy && moved_subset
        },
        _ => return None,
    })
}

/// Structurally claimed names, each with its reason.
const CLAIMED: &[(&str, &str)] = &[
    ("SIDE-EFFECT-VIOLATION", "resolve_plugin_graph_v1 is a pure function: no Wasmtime/PluginMgr/register_tar/load_event/fs/network types are importable from resolver.rs (PDG-069..075 all foreclosed by signature + module imports)"),
    ("INDEGREE-OVERFLOW", "checked_add on u32 indegree; unreachable while max_edge_count <= u32::MAX but the checked path exists and rejects (PDG-041)"),
    ("FEATURE-OFF-PASS", "resolver module is behind the plugins feature; feature-off workspace build verified by the standard check pipeline (PDG-079, T2.4.19)"),
    ("DEFERRED-TO-T2.5", "packet 5.2: root selection, deployment admission, and conflict policy recorded as T2.5 handoffs, not closed"),
];

#[test]
fn t2_4_catalog_pins_counts_and_total_coverage() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../readme/apex");
    let bytes = std::fs::read(dir.join(CATALOG)).expect("catalog present");
    let sha: String = sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(sha, PIN, "catalog pin drift");
    let text = String::from_utf8_lossy(&bytes);
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 80);

    let claimed: std::collections::BTreeSet<&str> = CLAIMED.iter().map(|(n, _)| *n).collect();
    let mut driven = 0usize;
    let mut failed = Vec::new();
    let mut unclaimed = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for case in cases {
        let terminal = case["expected_terminal"].as_str().unwrap();
        if !seen.insert(terminal.to_owned()) {
            continue;
        }
        match drive(terminal) {
            Some(true) => driven += 1,
            Some(false) => failed.push(terminal.to_owned()),
            None => {
                if !claimed.contains(terminal) {
                    unclaimed.push(terminal.to_owned());
                }
            },
        }
    }
    assert!(failed.is_empty(), "driven terminals that FAILED: {failed:?}");
    assert!(unclaimed.is_empty(), "unclaimed catalog terminals: {unclaimed:?}");
    assert!(driven >= 15, "driven terminal count regressed: {driven}");
}
