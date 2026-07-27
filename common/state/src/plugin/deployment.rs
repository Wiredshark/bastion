//! `APEX-T2.5.11` (server half, pure core) — compile a live deployment
//! from raw archive bytes through the FULL strict pipeline:
//! T2.2 `admit_strict_canonical` → T2.3 `validate_plugin_manifest_v1` →
//! T2.4 `resolve_plugin_graph_v1` → .05 `compile_deployment_plan_v1` →
//! .08 mode projections. This is also the closure of T2.2's PAR-C14
//! rollout gate: the rollout policy value is the loaded deployment
//! policy's archive `policy_id` — strict admission stops being test-only
//! exactly when a real operator policy exists.
//!
//! Pure: bytes + policy in, compiled deployment out. The server's
//! startup wiring (policy-file discovery, fail-closed rules, resource
//! insertion) lives in the server crate; everything testable is here.

use super::activation_plan::*;
use super::archive_profile::{ArchiveRejectV1, admit_strict_canonical};
use super::manifest::{
    PluginManifestAdmissionV1, PluginManifestEnforcementModeV1, PluginManifestErrorExtV1, ValidatedPluginManifestV1,
    validate_plugin_manifest_v1,
};
use super::resolver::*;
use common::apex::digest::{DigestDomainIdV1, ProtocolDigestV1, digest_canonical_bytes_v1, hash_artifact_bytes_v1};
use std::io::Read;

#[derive(Debug)]
pub enum PluginDeploymentCompileErrorV1 {
    /// Archive (by input index) failed strict admission.
    ArchiveRejected { index: usize, reject: ArchiveRejectV1 },
    /// Archive admitted but its manifest bytes could not be re-read.
    ManifestUnreadable { index: usize },
    /// Manifest failed T2.3 validation.
    ManifestRejected { index: usize, error: PluginManifestErrorExtV1 },
    /// A strict deployment cannot contain a legacy-observed manifest.
    LegacyManifestInStrictDeployment { index: usize },
    /// T2.4 resolution rejected the candidate set.
    ResolutionRejected { report: PluginResolutionReportV1 },
    /// Plan compilation / root derivation failure.
    ActivationError(PluginActivationErrorV1),
    /// A resolved node's artifact matched none of the input archives —
    /// impossible unless the pipeline is broken; refuse loudly.
    ArtifactUnmatched { ordinal: u32 },
    CanonicalizationFailure,
}

/// One fully compiled deployment: the shared plan, both mode
/// projections, and the exact ordinal→bytes artifact set for serving.
pub struct CompiledDeploymentV1 {
    pub plan: PluginDeploymentPlanV1,
    pub deployment_root: ProtocolDigestV1,
    pub server_plan: PluginActivationPlanV1,
    pub client_plan: PluginActivationPlanV1,
    /// Ordinal-sorted `(ordinal, archive bytes)` — the serving set.
    pub artifacts: Vec<(u32, Vec<u8>)>,
}

fn manifest_bytes_of(archive: &[u8], manifest_path: &str) -> Option<Vec<u8>> {
    // tar-rs already reconciled against the canonical scan during strict
    // admission, so reading by path here cannot disagree with admission.
    let mut ar = tar::Archive::new(archive);
    for entry in ar.entries().ok()? {
        let mut entry = entry.ok()?;
        let path = entry.path().ok()?;
        if path.to_str() == Some(manifest_path) {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).ok()?;
            return Some(bytes);
        }
    }
    None
}

/// The resolver policy a deployment compile runs under: fixed V1
/// mechanism knobs, with `policy_root` derived from the DEPLOYMENT
/// policy root so a policy change moves the graph root too (nothing
/// hidden from the evidence chain).
fn resolver_policy_for(policy_root: &ProtocolDigestV1) -> Result<PluginResolverPolicyV1, PluginDeploymentCompileErrorV1> {
    Ok(PluginResolverPolicyV1 {
        resolver_version: PLUGIN_RESOLVER_VERSION_V1,
        multiplicity: PluginVersionMultiplicityPolicyV1::SingleVersionPerPluginId,
        ready_order: PluginReadyOrderV1::AscendingNodeKey,
        cycle_witness: PluginCycleWitnessPolicyV1::ResidualSortedDfsRotateMinV1,
        limits: PluginResolverLimitsV1 {
            max_node_count: 256,
            max_edge_count: 1024,
            max_error_count: 32,
            max_cycle_witness_nodes: 32,
        },
        policy_root: digest_canonical_bytes_v1(
            DigestDomainIdV1::PluginResolvedGraph,
            policy_root.bytes.as_array(),
            1 << 20,
        )
        .map_err(|_| PluginDeploymentCompileErrorV1::CanonicalizationFailure)?,
    })
}

/// The full strict compile. `base_content_root` is the base-game content
/// identity the deployment is built against (supplied by the caller —
/// the server owns what "base content" means).
pub fn compile_deployment_from_archives_v1(
    archives: &[Vec<u8>],
    policy: &PluginDeploymentAdmissionPolicyV1,
    base_content_root: ProtocolDigestV1,
) -> Result<CompiledDeploymentV1, PluginDeploymentCompileErrorV1> {
    use PluginDeploymentCompileErrorV1 as E;
    let policy_root = policy.policy_root().map_err(E::ActivationError)?;

    let mut admissions: Vec<PluginManifestAdmissionV1> = Vec::with_capacity(archives.len());
    let mut validated: Vec<ValidatedPluginManifestV1> = Vec::with_capacity(archives.len());
    for (index, bytes) in archives.iter().enumerate() {
        // PAR-C14 closure: the rollout policy value is the operator
        // policy's archive policy id.
        let strict = admit_strict_canonical(
            bytes,
            &policy.archive_limits,
            Some(policy.archive_limits.policy_id.as_str()),
        )
        .map_err(|reject| E::ArchiveRejected { index, reject })?;
        let manifest_bytes = manifest_bytes_of(bytes, strict.manifest.manifest_path.as_str())
            .ok_or(E::ManifestUnreadable { index })?;
        let admission = validate_plugin_manifest_v1(
            &manifest_bytes,
            &strict.namespace,
            &strict.artifact,
            &strict.semantic_root,
            &policy.manifest_limits,
            PluginManifestEnforcementModeV1::StrictV1,
            policy_root.clone(),
        )
        .map_err(|error| E::ManifestRejected { index, error })?;
        match &admission {
            PluginManifestAdmissionV1::ValidatedV1(v) => validated.push((**v).clone()),
            _ => return Err(E::LegacyManifestInStrictDeployment { index }),
        }
        admissions.push(admission);
    }

    let graph = match resolve_plugin_graph_v1(admissions, &resolver_policy_for(&policy_root)?) {
        PluginResolutionTerminalV1::Resolved(g) => g,
        PluginResolutionTerminalV1::Rejected(report) => return Err(E::ResolutionRejected { report: *report }),
    };
    let plan =
        compile_deployment_plan_v1(&graph, policy, base_content_root, &validated).map_err(E::ActivationError)?;
    validate_deployment_plan_v1(&plan)
        .map_err(|_| E::ActivationError(PluginActivationErrorV1::CanonicalizationFailure))?;
    let deployment_root = plan.deployment_root().map_err(E::ActivationError)?;
    let server_plan = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server).map_err(E::ActivationError)?;
    let client_plan = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Client).map_err(E::ActivationError)?;

    // Ordinal → archive bytes, joined by ARTIFACT IDENTITY recomputed
    // from the input bytes (never by input position).
    let mut artifacts = Vec::with_capacity(plan.nodes.len());
    for node in &plan.nodes {
        let bytes = archives
            .iter()
            .find(|b| hash_artifact_bytes_v1(b) == node.artifact)
            .ok_or(E::ArtifactUnmatched { ordinal: node.ordinal })?;
        artifacts.push((node.ordinal, bytes.clone()));
    }
    artifacts.sort_by_key(|(o, _)| *o);

    Ok(CompiledDeploymentV1 { plan, deployment_root, server_plan, client_plan, artifacts })
}

#[cfg(test)]
mod tests {
    use super::super::archive_profile::pack_canonical;
    use super::*;
    use common::apex::manifest::{CanonicalPathV1, MachineTextV1};

    fn mtext(s: &str) -> MachineTextV1 { MachineTextV1::new(s).unwrap() }

    fn fixture_policy() -> PluginDeploymentAdmissionPolicyV1 {
        PluginDeploymentAdmissionPolicyV1 {
            schema_version: PLUGIN_ACTIVATION_SCHEMA_VERSION_V1,
            purpose: PluginPolicyPurposeV1::TestFixture,
            archive_limits: super::super::archive_profile::ArchiveLimitsPolicyV1 {
                policy_id: mtext("apex-t2-5-11-fixture-archive-v1"),
                max_archive_bytes: 1 << 20,
                max_entry_bytes: 1 << 18,
                max_entries: 64,
                max_path_bytes: 200,
                max_manifest_bytes: 1 << 14,
            },
            manifest_limits: super::super::manifest::PluginManifestLimitsV1 {
                policy_id: mtext("apex-t2-5-11-fixture-manifest-v1"),
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
            },
            runtime_limits_by_mode: vec![PluginModeRuntimeLimitsV1 {
                mode: PluginActivationModeV1::Server,
                max_linear_memory_bytes: 1 << 26,
                max_fuel_per_event: 1 << 20,
                max_instances: 8,
            }],
            legacy_admission: PluginLegacyAdmissionV1::StrictCanonicalOnly,
            conflict_decisions: vec![],
            multiplayer_local_plugin_policy: MultiplayerLocalPluginPolicyV1::RejectLocalPlugins,
            policy_owner: PluginPolicyOwnerIdV1(mtext("apex-test-operator")),
            policy_revision: 1,
        }
    }

    fn fixture_archive(id: &str, world: &str) -> Vec<u8> {
        let toml = format!(
            "manifest_version = 1\ndependencies = []\n\n[plugin]\nid = \"{id}\"\nversion = \"1.0.0\"\n\
             host_api = \"veloren:plugin@0.0.1\"\n\n[[modules]]\npath = \"modules/m.wasm\"\nworld = \"{world}\"\n\n\
             [claims]\nasset_roots = []\n\n[[claims.runtime]]\nmode = \"server\"\ncommands = []\nanimations = []\n"
        );
        pack_canonical(
            &[
                (CanonicalPathV1::new("plugin.toml").unwrap(), toml.as_bytes()),
                (CanonicalPathV1::new("modules/m.wasm").unwrap(), b"\0wasm-fixture"),
            ],
            &fixture_policy().archive_limits,
        )
        .unwrap()
    }

    #[test]
    fn strict_pipeline_compiles_real_archives_end_to_end() {
        let base = digest_canonical_bytes_v1(DigestDomainIdV1::PluginActivationPlan, b"base", 1 << 20).unwrap();
        let archives = vec![fixture_archive("x:srv", "server-plugin"), fixture_archive("x:both", "plugin")];
        let compiled = compile_deployment_from_archives_v1(&archives, &fixture_policy(), base.clone()).unwrap();
        assert_eq!(compiled.plan.nodes.len(), 2);
        assert_eq!(compiled.artifacts.len(), 2);
        // Server sees both, client only the shared-world plugin.
        assert_eq!(compiled.server_plan.activations.len(), 2);
        assert_eq!(compiled.client_plan.activations.len(), 1);
        // Deterministic: recompile (archives permuted) => identical root
        // and identical serving set.
        let permuted = vec![archives[1].clone(), archives[0].clone()];
        let again = compile_deployment_from_archives_v1(&permuted, &fixture_policy(), base.clone()).unwrap();
        assert_eq!(compiled.deployment_root, again.deployment_root, "archive input order can never move the root");
        assert_eq!(compiled.artifacts, again.artifacts);

        // A policy change moves the deployment root.
        let mut policy2 = fixture_policy();
        policy2.policy_revision = 2;
        let moved = compile_deployment_from_archives_v1(&archives, &policy2, base).unwrap();
        assert_ne!(compiled.deployment_root, moved.deployment_root);

        // Garbage archive is a typed refusal naming the input.
        let mut broken = archives.clone();
        broken.push(b"not a tar at all".to_vec());
        assert!(matches!(
            compile_deployment_from_archives_v1(&broken, &fixture_policy(), digest_canonical_bytes_v1(DigestDomainIdV1::PluginActivationPlan, b"base", 1 << 20).unwrap()),
            Err(PluginDeploymentCompileErrorV1::ArchiveRejected { index: 2, .. })
        ));
    }
}
