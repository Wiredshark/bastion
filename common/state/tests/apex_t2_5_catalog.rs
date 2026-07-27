//! `APEX-T2.5.24` (pure subset) — the 120-case catalog acceptance runner,
//! T2.2/2.3/2.4 pattern: pin re-verified per run, TOTAL terminal coverage
//! (driven / structurally-claimed with reasons; unclaimed name FAILS),
//! everything driven through the REAL pipeline: T2.3 validate → T2.4
//! resolve → .05 compile → .07 conflicts → .08 projections → .09 cache.
//! Runtime terminals (.10-.19: wire, client bootstrap, content
//! publication, Wasmtime, lifecycle/receipt, live limit enforcement) and
//! the .04b deployment-evidence family are claimed with their step
//! deferrals — the row stays open (NEEDS-DEPLOYMENT-EVIDENCE) until those
//! land; this runner is the pure-surface gate, not row completion.

#![cfg(feature = "plugins")]

use common::apex::digest::{DigestDomainIdV1, ProtocolDigestV1, digest_canonical_bytes_v1, hash_artifact_bytes_v1};
use common::apex::manifest::{CanonicalPathV1, MachineTextV1};
use sha2::Digest;
use veloren_common_state::plugin::activation_plan::*;
use veloren_common_state::plugin::archive_profile::{ArchiveLimitsPolicyV1, CanonicalEntryV1};
use veloren_common_state::plugin::artifact_cache::*;
use veloren_common_state::plugin::manifest::*;
use veloren_common_state::plugin::resolver::*;

const CATALOG: &str = "PROJECT-BASTION-APEX-T2.5-PLUGIN-ACTIVATION-PLAN-CANARIES-v1.json";
const PIN: &str = "bbc061fa8e5bbec465dfbbf6d4e625c94d9e2babc137b818f709560de51ceb91";

fn mtext(s: &str) -> MachineTextV1 { MachineTextV1::new(s).unwrap() }

fn proto(p: &[u8]) -> ProtocolDigestV1 {
    digest_canonical_bytes_v1(DigestDomainIdV1::PluginActivationPlan, p, 1 << 20).unwrap()
}

fn mlimits() -> PluginManifestLimitsV1 {
    PluginManifestLimitsV1 {
        policy_id: mtext("apex-t2-5-catalog-mlimits-v1"),
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

fn deployment_policy() -> PluginDeploymentAdmissionPolicyV1 {
    PluginDeploymentAdmissionPolicyV1 {
        schema_version: PLUGIN_ACTIVATION_SCHEMA_VERSION_V1,
        purpose: PluginPolicyPurposeV1::TestFixture,
        archive_limits: ArchiveLimitsPolicyV1 {
            policy_id: mtext("apex-t2-5-catalog-archive-v1"),
            max_archive_bytes: 1 << 20,
            max_entry_bytes: 1 << 18,
            max_entries: 64,
            max_path_bytes: 200,
            max_manifest_bytes: 1 << 14,
        },
        manifest_limits: mlimits(),
        runtime_limits_by_mode: vec![
            PluginModeRuntimeLimitsV1 {
                mode: PluginActivationModeV1::Server,
                max_linear_memory_bytes: 1 << 26,
                max_fuel_per_event: 1 << 20,
                max_instances: 8,
            },
            PluginModeRuntimeLimitsV1 {
                mode: PluginActivationModeV1::Client,
                max_linear_memory_bytes: 1 << 26,
                max_fuel_per_event: 1 << 20,
                max_instances: 8,
            },
        ],
        legacy_admission: PluginLegacyAdmissionV1::StrictCanonicalOnly,
        conflict_decisions: vec![],
        multiplayer_local_plugin_policy: MultiplayerLocalPluginPolicyV1::RejectLocalPlugins,
        policy_owner: PluginPolicyOwnerIdV1(mtext("apex-test-operator")),
        policy_revision: 1,
    }
}

fn resolver_policy() -> PluginResolverPolicyV1 {
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

/// One plugin with one module of the given world through the REAL T2.3
/// pipeline.
fn candidate(id: &str, world: &str) -> (PluginManifestAdmissionV1, ValidatedPluginManifestV1) {
    let toml = format!(
        "manifest_version = 1\ndependencies = []\n\n[plugin]\nid = \"{id}\"\nversion = \"1.0.0\"\n\
         host_api = \"veloren:plugin@0.0.1\"\n\n[[modules]]\npath = \"modules/m.wasm\"\nworld = \"{world}\"\n\n\
         [claims]\nasset_roots = []\n\n[[claims.runtime]]\nmode = \"server\"\ncommands = []\nanimations = []\n"
    );
    let ns: Vec<CanonicalEntryV1> = ["plugin.toml", "modules/m.wasm"]
        .iter()
        .map(|p| CanonicalEntryV1 {
            path: CanonicalPathV1::new(*p).unwrap(),
            portability_key: mtext(p),
            size_bytes: 1,
            content_sha256: [7; 32],
        })
        .collect();
    let art = hash_artifact_bytes_v1(toml.as_bytes());
    let root = digest_canonical_bytes_v1(DigestDomainIdV1::PluginManifest, b"admission-policy", 1 << 20).unwrap();
    let admission = validate_plugin_manifest_v1(
        toml.as_bytes(),
        &ns,
        &art,
        &root,
        &mlimits(),
        PluginManifestEnforcementModeV1::StrictV1,
        root.clone(),
    )
    .expect("catalog candidate must validate");
    let v = match &admission {
        PluginManifestAdmissionV1::ValidatedV1(v) => (**v).clone(),
        other => panic!("{other:?}"),
    };
    (admission, v)
}

/// The standard three-plugin deployment: server-only, shared, animation.
fn deployment() -> PluginDeploymentPlanV1 {
    let (a1, v1) = candidate("x:srv", "server-plugin");
    let (a2, v2) = candidate("x:both", "plugin");
    let (a3, v3) = candidate("x:anim", "animation-plugin");
    let graph = match resolve_plugin_graph_v1(vec![a1, a2, a3], &resolver_policy()) {
        PluginResolutionTerminalV1::Resolved(g) => g,
        PluginResolutionTerminalV1::Rejected(r) => panic!("{:?}", r.errors),
    };
    compile_deployment_plan_v1(&graph, &deployment_policy(), proto(b"base-content"), &[v1, v2, v3]).unwrap()
}

fn key(id: &str) -> PluginNodeKeyV1 {
    PluginNodeKeyV1 {
        plugin_id: CanonicalPluginIdV1::parse(id, &mlimits()).unwrap(),
        plugin_version: PluginVersionV1::parse("1.0.0").unwrap(),
    }
}

fn res(kind: PluginResourceKindV1, name: &str) -> PluginResourceKeyV1 {
    PluginResourceKeyV1 { kind, name: mtext(name) }
}

fn claim(r: &PluginResourceKeyV1, k: &str) -> PluginClaimV1 {
    PluginClaimV1 { resource: r.clone(), claimant: key(k) }
}

fn decision(r: &PluginResourceKeyV1, claimants: &[&str], resolution: PluginConflictResolutionV1) -> PluginConflictDecisionV1 {
    PluginConflictDecisionV1 {
        resource: r.clone(),
        claimants: claimants.iter().map(|k| key(k)).collect(),
        resolution,
        policy_version: 1,
    }
}

fn cache_pair() -> (tempfile::TempDir, PluginArtifactCacheV1, Vec<Vec<u8>>) {
    let dir = tempfile::tempdir().unwrap();
    let payloads = vec![b"artifact-zero".to_vec(), b"artifact-one".to_vec()];
    let reqs = payloads.iter().enumerate().map(|(i, b)| (i as u32, hash_artifact_bytes_v1(b))).collect();
    let cache = PluginArtifactCacheV1::new(dir.path().join("cache"), reqs).unwrap();
    (dir, cache, payloads)
}

/// terminal → driven verdict (true = behaves as the catalog requires).
fn drive(terminal: &str) -> Option<bool> {
    Some(match terminal {
        // -- root_domains ---------------------------------------------------
        "ROOTS-PASS" => {
            let plan = deployment();
            let policy_root = deployment_policy().policy_root().unwrap();
            let droot = plan.deployment_root().unwrap();
            let server = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server).unwrap();
            let receipt = PluginActivationReceiptV1 {
                deployment_root: droot.clone(),
                mode: PluginActivationModeV1::Server,
                registrations: vec![],
                within_ceiling: true,
                shadows: vec![],
            };
            policy_root != droot && server.activation_root().is_ok() && receipt.receipt_root().is_ok()
        },
        "BLOCK-ROOT-DOMAIN-MISMATCH" => {
            // Every pair of root KINDS over the same content is distinct —
            // substituting one for another can never verify.
            let plan = deployment();
            let droot = plan.deployment_root().unwrap();
            let server = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server).unwrap();
            let aroot = server.activation_root().unwrap();
            let receipt = PluginActivationReceiptV1 {
                deployment_root: droot.clone(),
                mode: PluginActivationModeV1::Server,
                registrations: vec![],
                within_ceiling: true,
                shadows: vec![],
            };
            let rroot = receipt.receipt_root().unwrap();
            let proot = deployment_policy().policy_root().unwrap();
            let all = [&droot, &aroot, &rroot, &proot];
            (0..all.len()).all(|i| ((i + 1)..all.len()).all(|j| all[i] != all[j]))
        },
        "BLOCK-MODE-ROOT-MISMATCH" => {
            let plan = deployment();
            let s = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server).unwrap();
            let c = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Client).unwrap();
            s.activation_root().unwrap() != c.activation_root().unwrap()
        },
        "INVALID-PLAN-ROOT" => {
            // Field-omission sensitivity: every top-level plan input moves
            // the deployment root (an omitted/altered field cannot verify).
            let base = deployment().deployment_root().unwrap();
            let mut p1 = deployment();
            p1.graph_root = proto(b"other-graph");
            let mut p2 = deployment();
            p2.base_content_root = proto(b"other-base");
            let mut p3 = deployment();
            p3.nodes.pop();
            base != p1.deployment_root().unwrap()
                && base != p2.deployment_root().unwrap()
                && base != p3.deployment_root().unwrap()
        },
        "PLAN-ROOT-STABLE" => {
            let a = deployment().deployment_root().unwrap();
            let b = deployment().deployment_root().unwrap();
            a == b
        },
        "INVALID-ACTIVATION-PLAN" => {
            let mut dup_ord = deployment();
            let n = dup_ord.nodes[0].clone();
            dup_ord.nodes.insert(0, n);
            let cmd = res(PluginResourceKindV1::Command, "hello");
            let mut dup_dec = deployment();
            dup_dec.conflict_decisions = vec![
                decision(&cmd, &["x:srv", "x:both"], PluginConflictResolutionV1::Reject),
                decision(&cmd, &["x:srv", "x:both"], PluginConflictResolutionV1::Reject),
            ];
            matches!(
                validate_deployment_plan_v1(&dup_ord),
                Err(PluginPlanValidationErrorV1::InvalidActivationPlanOrdinals)
            ) && matches!(
                validate_deployment_plan_v1(&dup_dec),
                Err(PluginPlanValidationErrorV1::InvalidActivationPlanDecisions)
            )
        },
        "UNSUPPORTED-PLAN-SCHEMA" => {
            let mut p = deployment();
            p.schema_version = 9;
            matches!(validate_deployment_plan_v1(&p), Err(PluginPlanValidationErrorV1::UnsupportedPlanSchema { got: 9 }))
        },

        // -- policy_presence ------------------------------------------------
        "POLICY-PASS" => {
            // Collision-free claims + empty decision map is legal, and the
            // full explicit policy roots.
            let cmd = res(PluginResourceKindV1::Command, "solo");
            resolve_claim_conflicts_v1(&[claim(&cmd, "x:srv")], &[], &[]).unwrap().is_empty()
                && deployment_policy().policy_root().is_ok()
        },
        "POLICY-ROOT-MISMATCH" => {
            // Any payload change moves the root: a stale root cannot match
            // a mutated policy.
            let base = deployment_policy().policy_root().unwrap();
            let mut p1 = deployment_policy();
            p1.policy_revision = 2;
            let mut p2 = deployment_policy();
            p2.purpose = PluginPolicyPurposeV1::Production;
            let mut p3 = deployment_policy();
            p3.runtime_limits_by_mode[0].max_fuel_per_event += 1;
            base != p1.policy_root().unwrap()
                && base != p2.policy_root().unwrap()
                && base != p3.policy_root().unwrap()
        },
        "INVALID-DEPLOYMENT-POLICY" => {
            // A decision naming a node with no live collision (unknown /
            // no-longer-present plugin) is refused as stale policy.
            let cmd = res(PluginResourceKindV1::Command, "hello");
            let d = decision(&cmd, &["x:ghost", "x:other"], PluginConflictResolutionV1::Reject);
            matches!(
                resolve_claim_conflicts_v1(&[], &[], &[d]),
                Err(PluginConflictErrorV1::StaleDecision { .. })
            )
        },

        // -- conflict_resolution --------------------------------------------
        "CONFLICT-DECISION-REQUIRED" => {
            let a = res(PluginResourceKindV1::AssetKey, "example.thing");
            matches!(
                resolve_claim_conflicts_v1(&[claim(&a, "x:srv"), claim(&a, "x:both")], &[], &[]),
                Err(PluginConflictErrorV1::UnresolvedCollision { .. })
            )
        },
        "CONFLICT-PASS" => {
            let a = res(PluginResourceKindV1::AssetKey, "example.thing");
            let claims = [claim(&a, "x:srv"), claim(&a, "x:both")];
            let d = decision(&a, &["x:srv", "x:both"], PluginConflictResolutionV1::ExclusiveOwner {
                owner: key("x:srv"),
                displaced: vec![key("x:both")],
            });
            let resolved = resolve_claim_conflicts_v1(&claims, &[], std::slice::from_ref(&d)).unwrap();
            resolved.len() == 1 && resolve_claim_conflicts_v1(&[], &[], &[]).unwrap().is_empty()
        },
        "UNAUTHORIZED-CONFLICT-DECISION" => {
            // No decision (from any source) can authorize shadowing base
            // content — and plugins have no channel to inject decisions at
            // all (decisions exist only inside the operator policy root).
            let cmd = res(PluginResourceKindV1::Command, "give");
            let d = decision(&cmd, &["x:srv", "x:both"], PluginConflictResolutionV1::ExclusiveOwner {
                owner: key("x:srv"),
                displaced: vec![key("x:both")],
            });
            matches!(
                resolve_claim_conflicts_v1(
                    &[claim(&cmd, "x:srv"), claim(&cmd, "x:both")],
                    std::slice::from_ref(&cmd),
                    std::slice::from_ref(&d)
                ),
                Err(PluginConflictErrorV1::BaseResourceShadowingForbidden { .. })
            )
        },
        "INVALID-CONFLICT-DECISION" => {
            let cmd = res(PluginResourceKindV1::Command, "hello");
            // Displaced names a provider that is not among the claimants.
            let d = decision(&cmd, &["x:srv", "x:both"], PluginConflictResolutionV1::ExclusiveOwner {
                owner: key("x:srv"),
                displaced: vec![key("x:ghost")],
            });
            matches!(
                resolve_claim_conflicts_v1(&[claim(&cmd, "x:srv"), claim(&cmd, "x:both")], &[], &[d]),
                Err(PluginConflictErrorV1::DecisionResolutionInvalid { .. })
            )
        },
        "CONFLICT-ROOT-STABLE" => {
            let cmd = res(PluginResourceKindV1::Command, "hello");
            let anim = res(PluginResourceKindV1::Skeleton, "wave");
            let d1 = decision(&cmd, &["x:srv", "x:both"], PluginConflictResolutionV1::Reject);
            let d2 = decision(&anim, &["x:srv", "x:anim"], PluginConflictResolutionV1::OrderedConcatenate {
                combiner_id: mtext("cat"),
                providers: vec![key("x:anim"), key("x:srv")],
            });
            let mut p1 = deployment_policy();
            p1.conflict_decisions = vec![d1.clone(), d2.clone()];
            let mut p2 = deployment_policy();
            p2.conflict_decisions = vec![d2, d1];
            p1.policy_root().unwrap() == p2.policy_root().unwrap()
        },

        // -- mode_projection ------------------------------------------------
        "MODE-PROJECTION-PASS" => {
            let plan = deployment();
            let s = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server).unwrap();
            let c = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Client).unwrap();
            let srv_only = plan.nodes.iter().find(|n| n.key == key("x:srv")).unwrap().ordinal;
            // Acceptance: server-only module absent from client plan; both
            // projections tie to the one deployment root.
            s.activations.contains(&srv_only)
                && !c.activations.contains(&srv_only)
                && s.deployment_root == c.deployment_root
                && validate_mode_activation_plan_v1(&s, &plan).is_ok()
                && validate_mode_activation_plan_v1(&c, &plan).is_ok()
        },
        "INVALID-MODE-PROJECTION" => {
            let plan = deployment();
            let srv_only = plan.nodes.iter().find(|n| n.key == key("x:srv")).unwrap().ordinal;
            let mut c = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Client).unwrap();
            c.activations.push(srv_only); // server-only module in client plan
            c.activations.sort_unstable();
            let mut c2 = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Client).unwrap();
            c2.activations.pop(); // client-required module omitted
            matches!(
                validate_mode_activation_plan_v1(&c, &plan),
                Err(PluginPlanValidationErrorV1::InvalidModeProjection { .. })
            ) && matches!(
                validate_mode_activation_plan_v1(&c2, &plan),
                Err(PluginPlanValidationErrorV1::InvalidModeProjection { .. })
            )
        },
        "MODE-DEFERRED" => {
            matches!(
                compile_mode_activation_plan_v1(&deployment(), PluginActivationModeV1::SinglePlayer),
                Err(PluginActivationErrorV1::SinglePlayerPlanUnsupported)
            )
        },
        "MODE-ROOT-STABLE" => {
            let plan = deployment();
            let a = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server).unwrap();
            let b = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server).unwrap();
            a.activation_root().unwrap() == b.activation_root().unwrap()
        },
        "INVALID-MODE-ROOT" => {
            // The mode tag IS bound into the root: identical deployment +
            // identical activation set, different mode => different root
            // (a root computed without the tag would collide here).
            let droot = proto(b"deployment");
            let s = PluginActivationPlanV1 {
                mode: PluginActivationModeV1::Server,
                deployment_root: droot.clone(),
                activations: vec![0, 1],
            };
            let c = PluginActivationPlanV1 {
                mode: PluginActivationModeV1::Client,
                deployment_root: droot,
                activations: vec![0, 1],
            };
            s.activation_root().unwrap() != c.activation_root().unwrap()
        },

        // -- artifacts_cache ------------------------------------------------
        "ARTIFACT-HASH-MISMATCH" => {
            let (_d, cache, p) = cache_pair();
            let mut flipped = p[0].clone();
            flipped[0] ^= 0xff;
            matches!(cache.stage(0, &flipped), Err(ArtifactCacheErrorV1::DigestMismatch { ordinal: 0 }))
        },
        "ARTIFACT-SIZE-MISMATCH" => {
            let (_d, cache, p) = cache_pair();
            let mut long = p[0].clone();
            long.push(0);
            matches!(cache.stage(0, &p[0][..4]), Err(ArtifactCacheErrorV1::SizeMismatch { .. }))
                && matches!(cache.stage(0, &long), Err(ArtifactCacheErrorV1::SizeMismatch { .. }))
        },
        "ARTIFACT-ORDINAL-MISMATCH" => {
            // Bytes of ordinal 1 delivered under ordinal 0: refused (the
            // per-ordinal requirement digest cannot match).
            let (_d, cache, p) = cache_pair();
            matches!(cache.stage(0, &p[1]), Err(ArtifactCacheErrorV1::SizeMismatch { .. } | ArtifactCacheErrorV1::DigestMismatch { .. }))
        },
        "UNREQUESTED-ARTIFACT" => {
            let (_d, cache, p) = cache_pair();
            matches!(cache.stage(9, &p[0]), Err(ArtifactCacheErrorV1::UnrequestedArtifact { ordinal: 9 }))
        },
        "ARTIFACT-DUPLICATE" => {
            // Duplicate identical response: idempotent, no state change.
            let (_d, cache, p) = cache_pair();
            let path1 = cache.stage(0, &p[0]).unwrap();
            let path2 = cache.stage(0, &p[0]).unwrap();
            path1 == path2 && cache.open_verified(0).unwrap() == p[0]
        },
        "ARTIFACT-CONFLICT" => {
            // Same ordinal, different bytes: the second response is refused
            // and the verified staging is untouched.
            let (_d, cache, p) = cache_pair();
            cache.stage(0, &p[0]).unwrap();
            let mut other = p[0].clone();
            other[0] ^= 0xff;
            matches!(cache.stage(0, &other), Err(ArtifactCacheErrorV1::DigestMismatch { .. }))
                && cache.open_verified(0).unwrap() == p[0]
        },
        "ARTIFACT-STAGING-STABLE" => {
            // Arrival order permuted => same final verified set.
            let (_d1, c1, p) = cache_pair();
            c1.stage(0, &p[0]).unwrap();
            c1.stage(1, &p[1]).unwrap();
            let (_d2, c2, q) = cache_pair();
            c2.stage(1, &q[1]).unwrap();
            c2.stage(0, &q[0]).unwrap();
            c1.open_verified(0).unwrap() == c2.open_verified(0).unwrap()
                && c1.open_verified(1).unwrap() == c2.open_verified(1).unwrap()
        },
        "ARTIFACT-STAGING-PASS" => {
            let (_d, cache, p) = cache_pair();
            cache.stage(0, &p[0]).unwrap();
            cache.stage(1, &p[1]).unwrap();
            cache.is_staged_verified(0) && cache.is_staged_verified(1)
        },
        "ARTIFACT-SET-INCOMPLETE" => {
            let (_d, cache, p) = cache_pair();
            cache.stage(0, &p[0]).unwrap();
            matches!(cache.open_verified(1), Err(ArtifactCacheErrorV1::NotStaged { ordinal: 1 }))
        },
        "UNVERIFIED-CACHE-HIT" => {
            let (_d, cache, p) = cache_pair();
            let path = cache.stage(0, &p[0]).unwrap();
            std::fs::write(&path, b"tampered-on-disk").unwrap();
            matches!(cache.open_verified(0), Err(ArtifactCacheErrorV1::CorruptCachedArtifact { ordinal: 0 }))
        },

        _ => return None,
    })
}

/// Structurally claimed names, each with its deferral/citation reason.
const CLAIMED: &[(&str, &str)] = &[
    // policy_presence: the strict loader lives in the server crate.
    ("POLICY-UNAVAILABLE", "server::plugin_deployment_policy::strict_loader_loads_and_fails_closed: missing file / missing field are typed terminals; the policy type has no Default and every limb is mandatory, so an absent limb is unrepresentable in the typed world (T2.5.04a)"),
    ("BLOCK-TEST-POLICY-IN-PRODUCTION", "purpose is bound into policy_root (driven by POLICY-ROOT-MISMATCH: purpose flip moves the root); the production-context admission gate lands with the .14 runtime policy enforcement step"),
    ("OPERATIONAL-POLICY-UNAVAILABLE", "T2.5.04b: no operator-reviewed production policy exists yet — row carries NEEDS-DEPLOYMENT-EVIDENCE; TestFixture-purpose policies power all tests (packet: blocks row completion, not implementation)"),
    // conflict combiners: registry lands with content publication.
    ("UNKNOWN-COMBINER", "OrderedConcatenate combiner registry lands with .12 batch content generation; until then no combine executes (decisions are recorded, not applied)"),
    ("UNSUPPORTED-COMBINE-SEMANTICS", "same .12 deferral: combine legality is checked against the registered combinable-schema list when combination executes"),
    ("INVALID-CONFLICT-INPUT", "claim-inventory completeness is the .06 expansion's contract: plugin_asset_keys_v1 (veloren-common-assets) fails closed on any unclaimed publishable file, so a base collision cannot be silently omitted from a claim group built through it"),
    ("RECEIPT-CONFLICT", ".19 receipt validation: committed owner maps are checked against resolved collisions when the receipt lands"),
    // root domain: digest algorithm sealed upstream.
    ("UNSUPPORTED-DIGEST-PROFILE", "DigestAlgorithmIdV1/DigestDomainIdV1 are sealed T0.2/T0.3 enums: an unknown algorithm or domain tag fails decode in the substrate before any T2.5 code runs"),
    // mode / client lifecycle: .10-.11.
    ("LOCAL-EXTRA-PLUGIN-REJECTED", ".11 client bootstrap: MultiplayerLocalPluginPolicyV1::RejectLocalPlugins is the only constructible V1 policy value; the enforcement point is client State assembly"),
    ("INVALID-MODE-INSTANTIATION", "mode-gated instantiation checks land with .18 lifecycle (instantiation itself + limits are live: .14/.15)"),
    ("PLUGIN-WORLD-MISMATCH", "LANDED .16: declared world selects exactly one wrapper, mismatch typed for all three worlds; legacy manifests keep probing (module.rs plugin_declared_world_v1 tests)"),
    // artifact wire: .10.
    ("ARTIFACT-ROOT-MISMATCH", ".10 typed wire collector: response deployment-root check precedes cache staging"),
    ("INVALID-ARTIFACT-REQUEST", ".10 typed wire messages: request validation against the deployment requirement set"),
    // client staging: .11.
    ("CLIENT-BOOTSTRAP-INCOMPLETE", ".11 complete-before-State client bootstrap"),
    ("CLIENT-BOOTSTRAP-PASS", ".11"),
    ("CLIENT-RECEIPT-MISMATCH", ".11/.19"),
    ("CLIENT-STAGING-STABLE", ".11 (cache-level order independence driven by ARTIFACT-STAGING-STABLE)"),
    ("CLIENT-STATE-BEFORE-PLUGINS", ".11: State-before-plugins ordering becomes a typed init error"),
    ("LATE-ACTIVATION-FORBIDDEN", ".23 removes the late-load APIs; .11 closes the client lifecycle gap"),
    // content publication: .12-.13.
    ("CONTENT-ALREADY-INSTALLED", "LANDED .12: veloren-common-assets plugin_content_generation_v1::generation_installs_exactly_once_and_seals_incremental_paths — second install refused, state untouched"),
    ("CONTENT-GENERATION-INCOMPLETE", "LANDED .12 by construction: install_content_generation_v1 publishes the whole batch under ONE registry write lock (all-or-nothing extend); no partial-generation state is representable"),
    ("CONTENT-NOT-INSTALLED", "LANDED .12: content_generation_v1() getter is the governed-caller probe (None = ungoverned); asserted in the same test"),
    ("CONTENT-PUBLICATION-ABORTED", "LANDED .12: generation_refuses_to_layer_on_legacy_publication — LegacyPublicationPresent refusal changes nothing"),
    ("CONTENT-PUBLICATION-PASS", "LANDED .12: install happy path publishes the complete batch, token readable, canonical fold order preserved"),
    ("CONTENT-ROOT-MISMATCH", "generation token = deployment root is INSTALLED by both governed consumers (.12); the consumer-side token-vs-expected-root check lands with .19 receipt validation"),
    ("EARLY-ASSET-ACCESS", "OPEN: the global cache remains usable pre-install for the ungoverned legacy path; a governed-process early-access guard is .13+ enforcement (deferral, not silently dropped)"),
    ("HOT-RELOAD-FORBIDDEN", "LANDED .12 by construction: the generation seal is a OnceLock with NO clear/replace API — authoritative replacement is unrepresentable"),
    ("INCREMENTAL-PUBLICATION-FORBIDDEN", "LANDED .12: post-seal commit_prepared_tars/register_tar refuse with GenerationSealed (tested)"),
    // deployment readiness / evidence: .04b + .24 full form.
    ("CORPUS-COMPLETENESS-OVERCLAIM", ".04b archive inventory (NEEDS-DEPLOYMENT-EVIDENCE)"),
    ("CORPUS-EXCLUSIONS-MISSING", ".04b"),
    ("CORPUS-UNAVAILABLE", ".04b"),
    ("INVENTORY-BASE-CONTENT-UNAVAILABLE", ".04b"),
    ("INVENTORY-ROOT-STABLE", ".04b"),
    ("INVENTORY-STATISTICS-OVERFLOW", ".04b"),
    ("PRODUCTION-ADMISSION-BLOCKED", ".04b + .07: production admission requires an operator-reviewed policy that does not exist yet — fail-closed is the current behavior"),
    ("PRODUCTION-READINESS-PASS", ".04b: cannot pass until deployment evidence exists"),
    ("RESEARCH-SPEC-PASS", "packet research-spec acceptance is recorded in the packet itself; no runtime surface"),
    ("FAIL-CLOSED-VIOLATION", "meta-terminal: every driven family above proves the fail-closed direction (typed refusal, never a default); no code path constructs a default policy/plan/artifact on failure — see .04a loader (no Settings fallback) and the no-Default policy types"),
    // legacy: T2.2/T2.3 already landed the admission halves; the rest is .13/.21+.
    ("LEGACY-PLUGIN-REJECTED", "T2.3: StrictV1 enforcement rejects legacy manifests (manifest.rs tests: legacy_lane_is_total_and_strict_rejects); T2.2 admit_strict_canonical rejects non-canonical archives"),
    ("INVALID-LEGACY-POLICY", ".04a loader: legacy_admission must be the literal strict-canonical-only value; anything else is a typed refusal (server crate test)"),
    ("LEGACY-POLICY-PASS", ".04a loader accepts exactly the strict posture (server crate test)"),
    ("LEGACY-MIGRATION-PLAN-REQUIRED", "packet: legacy migration is an explicit operator artifact; no migration machinery lands in V1 (deferral recorded, not silently dropped)"),
    ("LEGACY-OVERLAY-INCOMPLETE", "same legacy-migration deferral"),
    ("RUNTIME-MIGRATION-FORBIDDEN", ".23: late-load/migration APIs are removed rather than policed"),
    // lifecycle/receipt: .17-.19.
    ("ACTIVATION-RECEIPT-PASS", ".17-.19 exactly-once lifecycle + receipt validation (receipt TYPES and root are landed and driven under ROOTS-PASS/BLOCK-ROOT-DOMAIN-MISMATCH)"),
    ("INVALID-ACTIVATION-RECEIPT", ".19"),
    ("RECEIPT-PLAN-MISMATCH", ".19 (the receipt root binds deployment_root + mode: driven root separation already proves substitution cannot verify)"),
    ("PLUGIN-LIFECYCLE-ABORTED", ".18"),
    ("PLUGIN-LIFECYCLE-DUPLICATE", ".18 exactly-once lifecycle"),
    ("REGISTRATION-OUTSIDE-CEILING", ".19: within_ceiling is a typed receipt field (landed); enforcement lands with the lifecycle"),
    // runtime limits / Wasmtime: .14-.16.
    ("RUNTIME-POLICY-PASS", "LANDED .14: per-mode policy ceilings applied per store (memory limiter + per-event fuel), threaded policy->wire->both consumers; live-trap fixture deferred to .18/VM"),
    ("RUNTIME-POLICY-MISMATCH", ".18/.19: receipt-level comparison of applied vs policy limits (application itself landed .14)"),
    ("PLUGIN-RUNTIME-LIMIT", "LANDED .14 mechanism: StoreLimits memory ceiling live; trapping fixture deferred to .18/VM"),
    ("PLUGIN-FUEL-EXHAUSTED", "LANDED .14 mechanism: fuel ON at the one engine, per-event refuel at every entry; burn-loop fixture deferred to .18/VM"),
    ("PLUGIN-INTERRUPTED", ".14"),
    ("AUTO-TUNING-FORBIDDEN", "PluginModeRuntimeLimitsV1 has no Default and no derivation path: limits exist only as explicit policy fields bound into policy_root (driven by POLICY-ROOT-MISMATCH: a limit change moves the root)"),
    ("UNROOTED-RUNTIME-DEFAULT", "same no-Default construction: a runtime limit that is not in the rooted policy cannot exist"),
    ("PLUGIN-COMPILE-FAILED", "LANDED .15: malformed bytes + core-module-not-component both hit the typed compile terminal (plugin_component_preflight_v1)"),
    ("PLUGIN-IMPORT-FAILED", "LANDED .15: unknown import hits the typed instantiate_pre resolution terminal (with the documented empty-instance-import subtlety)"),
    ("PLUGIN-INSTANTIATE-FAILED", "LANDED .15: instantiation isolated in new_from_prepared (compile/import impossible there by construction); failure class exercised by the declared-world tests"),
    ("PLUGIN-LINKER-CONFLICT", "LANDED .15 stage: LinkerSetupFailed terminal exists at host-API registration; a genuine collision is not externally constructible with one linker per preflight"),
    ("PLUGIN-PREFLIGHT-PASS", "LANDED .15: empty component passes preflight; every T2.1 canary module now flows through it"),
    ("PREFLIGHT-ROOT-STABLE", "preflight is pure per bytes+engine (.15); a recorded preflight ROOT artifact lands with .19 receipts"),
    ("PREFLIGHT-SIDE-EFFECT", "LANDED .15 by construction: preflight_component_v1 creates no store/instance/wrapper; nothing host-visible exists before instantiation"),
];

#[test]
fn t2_5_catalog_pins_counts_and_total_coverage() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../readme/apex");
    let bytes = std::fs::read(dir.join(CATALOG)).expect("catalog present");
    let sha: String = sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(sha, PIN, "catalog pin drift");
    let text = String::from_utf8_lossy(&bytes);
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 120);

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
        // A name must not be double-classified.
        assert!(
            !(drive(terminal).is_some() && claimed.contains(terminal)),
            "terminal {terminal} is both driven and claimed"
        );
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
    assert!(driven >= 28, "driven terminal count regressed: {driven}");
}
