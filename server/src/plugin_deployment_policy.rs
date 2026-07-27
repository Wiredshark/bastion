//! `APEX-T2.5.04a` — the STRICT deployment-policy loader.
//!
//! `server_config/plugin_deployment_policy.ron` → typed
//! `PluginDeploymentAdmissionPolicyV1`, through a raw all-fields-required
//! mirror and the checked constructors. DELIBERATELY not a field on
//! `Settings`: live `Settings::load` falls back to defaults on parse
//! failure — the exact fail-open the row forbids. Missing file, parse
//! error, or invalid content are each a TYPED terminal; there is no
//! default policy object anywhere in this module.

#![cfg(feature = "plugins")]

use common::apex::manifest::MachineTextV1;
use common_state::plugin::activation_plan::*;
use common_state::plugin::archive_profile::ArchiveLimitsPolicyV1;
use common_state::plugin::manifest::{CanonicalPluginIdV1, PluginManifestLimitsV1, PluginVersionV1};
use common_state::plugin::resolver::PluginNodeKeyV1;
use serde::Deserialize;
use std::path::Path;

pub const POLICY_FILE: &str = "server_config/plugin_deployment_policy.ron";

#[derive(Debug)]
pub enum PluginPolicyLoadErrorV1 {
    /// No policy file: plugin-enabled startup must refuse
    /// (`BLOCK-ADMISSION-POLICY-MISSING` family).
    PolicyFileMissing { path: String },
    PolicyRead { detail: String },
    PolicyParse { detail: String },
    UnsupportedSchemaVersion { got: u32 },
    PolicyInvalid { detail: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPolicy {
    schema_version: u32,
    purpose: String,
    archive_limits: RawArchiveLimits,
    manifest_limits: RawManifestLimits,
    runtime_limits_by_mode: Vec<RawModeLimits>,
    legacy_admission: String,
    conflict_decisions: Vec<RawConflictDecision>,
    multiplayer_local_plugin_policy: String,
    policy_owner: String,
    policy_revision: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawArchiveLimits {
    policy_id: String,
    max_archive_bytes: u64,
    max_entry_bytes: u64,
    max_entries: u64,
    max_path_bytes: u64,
    max_manifest_bytes: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifestLimits {
    policy_id: String,
    max_manifest_bytes: u64,
    max_plugin_id_bytes: u16,
    max_display_name_bytes: u16,
    max_module_count: u32,
    max_dependency_count: u32,
    max_runtime_claim_modes: u8,
    max_command_claims_per_mode: u32,
    max_animation_claims_per_mode: u32,
    max_asset_root_count: u32,
    max_runtime_key_bytes: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModeLimits {
    mode: String,
    max_linear_memory_bytes: u64,
    max_fuel_per_event: u64,
    max_instances: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConflictDecision {
    kind: String,
    name: String,
    claimants: Vec<RawNodeKey>,
    resolution: RawResolution,
    policy_version: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNodeKey {
    plugin_id: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum RawResolution {
    Reject,
    ExclusiveOwner { owner: RawNodeKey, displaced: Vec<RawNodeKey> },
    OrderedConcatenate { combiner_id: String, providers: Vec<RawNodeKey> },
}

fn invalid(detail: impl Into<String>) -> PluginPolicyLoadErrorV1 {
    PluginPolicyLoadErrorV1::PolicyInvalid { detail: detail.into() }
}

fn mtext(s: &str, what: &str) -> Result<MachineTextV1, PluginPolicyLoadErrorV1> {
    MachineTextV1::new(s).map_err(|_| invalid(format!("{what}: non-ASCII")))
}

fn mode(s: &str) -> Result<PluginActivationModeV1, PluginPolicyLoadErrorV1> {
    match s {
        "server" => Ok(PluginActivationModeV1::Server),
        "client" => Ok(PluginActivationModeV1::Client),
        "single-player" => Ok(PluginActivationModeV1::SinglePlayer),
        other => Err(invalid(format!("unknown mode {other:?}"))),
    }
}

fn node_key(raw: &RawNodeKey, limits: &PluginManifestLimitsV1) -> Result<PluginNodeKeyV1, PluginPolicyLoadErrorV1> {
    Ok(PluginNodeKeyV1 {
        plugin_id: CanonicalPluginIdV1::parse(&raw.plugin_id, limits)
            .map_err(|e| invalid(format!("claimant id {:?}: {e:?}", raw.plugin_id)))?,
        plugin_version: PluginVersionV1::parse(&raw.version)
            .map_err(|e| invalid(format!("claimant version {:?}: {e:?}", raw.version)))?,
    })
}

/// THE strict loader (packet §7). The only production entry point for a
/// deployment policy.
pub fn load_plugin_deployment_policy_strict_v1(
    data_dir: &Path,
) -> Result<PluginDeploymentAdmissionPolicyV1, PluginPolicyLoadErrorV1> {
    let path = data_dir.join(POLICY_FILE);
    if !path.is_file() {
        return Err(PluginPolicyLoadErrorV1::PolicyFileMissing { path: path.display().to_string() });
    }
    let raw_text = std::fs::read_to_string(&path)
        .map_err(|e| PluginPolicyLoadErrorV1::PolicyRead { detail: e.to_string() })?;
    let raw: RawPolicy = ron::from_str(&raw_text)
        .map_err(|e| PluginPolicyLoadErrorV1::PolicyParse { detail: e.to_string() })?;

    if raw.schema_version != PLUGIN_ACTIVATION_SCHEMA_VERSION_V1 {
        return Err(PluginPolicyLoadErrorV1::UnsupportedSchemaVersion { got: raw.schema_version });
    }
    let purpose = match raw.purpose.as_str() {
        "production" => PluginPolicyPurposeV1::Production,
        "test-fixture" => PluginPolicyPurposeV1::TestFixture,
        other => return Err(invalid(format!("unknown purpose {other:?}"))),
    };
    if raw.legacy_admission != "strict-canonical-only" {
        return Err(invalid("legacy_admission must be \"strict-canonical-only\" (the only V1 posture)"));
    }
    if raw.multiplayer_local_plugin_policy != "reject-local-plugins" {
        return Err(invalid("multiplayer_local_plugin_policy must be \"reject-local-plugins\" (V1)"));
    }

    let manifest_limits = PluginManifestLimitsV1 {
        policy_id: mtext(&raw.manifest_limits.policy_id, "manifest policy_id")?,
        max_manifest_bytes: raw.manifest_limits.max_manifest_bytes,
        max_plugin_id_bytes: raw.manifest_limits.max_plugin_id_bytes,
        max_display_name_bytes: raw.manifest_limits.max_display_name_bytes,
        max_module_count: raw.manifest_limits.max_module_count,
        max_dependency_count: raw.manifest_limits.max_dependency_count,
        max_runtime_claim_modes: raw.manifest_limits.max_runtime_claim_modes,
        max_command_claims_per_mode: raw.manifest_limits.max_command_claims_per_mode,
        max_animation_claims_per_mode: raw.manifest_limits.max_animation_claims_per_mode,
        max_asset_root_count: raw.manifest_limits.max_asset_root_count,
        max_runtime_key_bytes: raw.manifest_limits.max_runtime_key_bytes,
    };

    let kind = |s: &str| -> Result<PluginResourceKindV1, PluginPolicyLoadErrorV1> {
        match s {
            "command" => Ok(PluginResourceKindV1::Command),
            "body" => Ok(PluginResourceKindV1::Body),
            "skeleton" => Ok(PluginResourceKindV1::Skeleton),
            "asset-key" => Ok(PluginResourceKindV1::AssetKey),
            other => Err(invalid(format!("unknown resource kind {other:?}"))),
        }
    };
    let mut conflict_decisions = Vec::with_capacity(raw.conflict_decisions.len());
    for d in &raw.conflict_decisions {
        let claimants: Vec<PluginNodeKeyV1> =
            d.claimants.iter().map(|c| node_key(c, &manifest_limits)).collect::<Result<_, _>>()?;
        if claimants.len() < 2 {
            return Err(invalid("a conflict decision needs at least two claimants"));
        }
        let resolution = match &d.resolution {
            RawResolution::Reject => PluginConflictResolutionV1::Reject,
            RawResolution::ExclusiveOwner { owner, displaced } => {
                let owner = node_key(owner, &manifest_limits)?;
                let displaced: Vec<PluginNodeKeyV1> =
                    displaced.iter().map(|c| node_key(c, &manifest_limits)).collect::<Result<_, _>>()?;
                if !claimants.contains(&owner) {
                    return Err(invalid("ExclusiveOwner.owner must be one of the claimants"));
                }
                if displaced.is_empty() || displaced.iter().any(|k| !claimants.contains(k)) {
                    return Err(invalid("ExclusiveOwner.displaced must name the displaced claimants exactly"));
                }
                PluginConflictResolutionV1::ExclusiveOwner { owner, displaced }
            },
            RawResolution::OrderedConcatenate { combiner_id, providers } => PluginConflictResolutionV1::OrderedConcatenate {
                combiner_id: mtext(combiner_id, "combiner_id")?,
                providers: providers.iter().map(|c| node_key(c, &manifest_limits)).collect::<Result<_, _>>()?,
            },
        };
        conflict_decisions.push(PluginConflictDecisionV1 {
            resource: PluginResourceKeyV1 { kind: kind(&d.kind)?, name: mtext(&d.name, "resource name")? },
            claimants,
            resolution,
            policy_version: d.policy_version,
        });
    }

    Ok(PluginDeploymentAdmissionPolicyV1 {
        schema_version: raw.schema_version,
        purpose,
        archive_limits: ArchiveLimitsPolicyV1 {
            policy_id: mtext(&raw.archive_limits.policy_id, "archive policy_id")?,
            max_archive_bytes: raw.archive_limits.max_archive_bytes,
            max_entry_bytes: raw.archive_limits.max_entry_bytes,
            max_entries: raw.archive_limits.max_entries,
            max_path_bytes: raw.archive_limits.max_path_bytes,
            max_manifest_bytes: raw.archive_limits.max_manifest_bytes,
        },
        manifest_limits,
        runtime_limits_by_mode: raw
            .runtime_limits_by_mode
            .iter()
            .map(|m| {
                Ok(PluginModeRuntimeLimitsV1 {
                    mode: mode(&m.mode)?,
                    max_linear_memory_bytes: m.max_linear_memory_bytes,
                    max_fuel_per_event: m.max_fuel_per_event,
                    max_instances: m.max_instances,
                })
            })
            .collect::<Result<_, PluginPolicyLoadErrorV1>>()?,
        legacy_admission: PluginLegacyAdmissionV1::StrictCanonicalOnly,
        conflict_decisions,
        multiplayer_local_plugin_policy: MultiplayerLocalPluginPolicyV1::RejectLocalPlugins,
        policy_owner: PluginPolicyOwnerIdV1(mtext(&raw.policy_owner, "policy_owner")?),
        policy_revision: raw.policy_revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"(
    schema_version: 1,
    purpose: "test-fixture",
    archive_limits: (
        policy_id: "apex-t2-5-fixture-archive-v1",
        max_archive_bytes: 1048576,
        max_entry_bytes: 262144,
        max_entries: 64,
        max_path_bytes: 200,
        max_manifest_bytes: 16384,
    ),
    manifest_limits: (
        policy_id: "apex-t2-5-fixture-manifest-v1",
        max_manifest_bytes: 16384,
        max_plugin_id_bytes: 64,
        max_display_name_bytes: 64,
        max_module_count: 8,
        max_dependency_count: 8,
        max_runtime_claim_modes: 3,
        max_command_claims_per_mode: 8,
        max_animation_claims_per_mode: 8,
        max_asset_root_count: 4,
        max_runtime_key_bytes: 64,
    ),
    runtime_limits_by_mode: [
        (mode: "server", max_linear_memory_bytes: 67108864, max_fuel_per_event: 1048576, max_instances: 8),
        (mode: "client", max_linear_memory_bytes: 67108864, max_fuel_per_event: 1048576, max_instances: 8),
    ],
    legacy_admission: "strict-canonical-only",
    conflict_decisions: [],
    multiplayer_local_plugin_policy: "reject-local-plugins",
    policy_owner: "apex-test-operator",
    policy_revision: 1,
)"#;

    fn write_policy(dir: &Path, content: &str) {
        let p = dir.join(POLICY_FILE);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn strict_loader_loads_and_fails_closed() {
        let dir = tempfile::tempdir().unwrap();

        // Missing file = typed missing, never a default object.
        assert!(matches!(
            load_plugin_deployment_policy_strict_v1(dir.path()),
            Err(PluginPolicyLoadErrorV1::PolicyFileMissing { .. })
        ));

        write_policy(dir.path(), GOOD);
        let policy = load_plugin_deployment_policy_strict_v1(dir.path()).unwrap();
        assert_eq!(policy.purpose, PluginPolicyPurposeV1::TestFixture);
        assert!(policy.policy_root().is_ok());

        // Parse failure = typed parse error (the anti-Settings-fallback
        // property: no default object on ANY failure path).
        write_policy(dir.path(), "not ron at all (((");
        assert!(matches!(
            load_plugin_deployment_policy_strict_v1(dir.path()),
            Err(PluginPolicyLoadErrorV1::PolicyParse { .. })
        ));

        // Unknown field is rejected (deny_unknown_fields).
        write_policy(dir.path(), &GOOD.replace("policy_revision: 1,", "policy_revision: 1, extra: 2,"));
        assert!(matches!(
            load_plugin_deployment_policy_strict_v1(dir.path()),
            Err(PluginPolicyLoadErrorV1::PolicyParse { .. })
        ));

        // Missing field is rejected (all fields mandatory).
        write_policy(dir.path(), &GOOD.replace("policy_revision: 1,", ""));
        assert!(matches!(
            load_plugin_deployment_policy_strict_v1(dir.path()),
            Err(PluginPolicyLoadErrorV1::PolicyParse { .. })
        ));

        // Wrong schema version typed.
        write_policy(dir.path(), &GOOD.replace("schema_version: 1", "schema_version: 9"));
        assert!(matches!(
            load_plugin_deployment_policy_strict_v1(dir.path()),
            Err(PluginPolicyLoadErrorV1::UnsupportedSchemaVersion { got: 9 })
        ));

        // Owner-not-a-claimant invariant bites.
        let bad_owner = GOOD.replace(
            "conflict_decisions: [],",
            r#"conflict_decisions: [(
                kind: "command", name: "hello",
                claimants: [(plugin_id: "x:a", version: "1.0.0"), (plugin_id: "x:b", version: "1.0.0")],
                resolution: ExclusiveOwner(owner: (plugin_id: "x:z", version: "1.0.0"), displaced: [(plugin_id: "x:b", version: "1.0.0")]),
                policy_version: 1,
            )],"#,
        );
        write_policy(dir.path(), &bad_owner);
        assert!(matches!(
            load_plugin_deployment_policy_strict_v1(dir.path()),
            Err(PluginPolicyLoadErrorV1::PolicyInvalid { .. })
        ));
    }
}

// ---------------------------------------------------------------------------
// APEX-T2.5.11 — the server's live deployment state + startup init.
// ---------------------------------------------------------------------------

use common_net::msg::plugin_artifact::{PluginArtifactDescriptorV1, PluginDeploymentSummaryV1};
use common_state::plugin::deployment::{PluginDeploymentCompileErrorV1, compile_deployment_from_archives_v1};

/// The compiled deployment as an ECS resource. `Legacy` = no policy file
/// (today's behavior, GameSync sends `None`); `Deployed` = a strict
/// compile succeeded at startup.
pub enum PluginDeploymentStateV1 {
    Legacy,
    Deployed {
        summary: PluginDeploymentSummaryV1,
        /// Ordinal-sorted serving set.
        artifacts: Vec<(u32, std::sync::Arc<Vec<u8>>)>,
    },
}

impl PluginDeploymentStateV1 {
    pub fn summary(&self) -> Option<PluginDeploymentSummaryV1> {
        match self {
            Self::Legacy => None,
            Self::Deployed { summary, .. } => Some(summary.clone()),
        }
    }

    pub fn artifact(&self, ordinal: u32) -> Option<&std::sync::Arc<Vec<u8>>> {
        match self {
            Self::Legacy => None,
            Self::Deployed { artifacts, .. } => {
                artifacts.iter().find(|(o, _)| *o == ordinal).map(|(_, bytes)| bytes)
            },
        }
    }

    pub fn deployment_root_bytes(&self) -> Option<[u8; 32]> {
        self.summary().map(|s| s.deployment_root)
    }
}

#[derive(Debug)]
pub enum PluginDeploymentInitErrorV1 {
    /// Policy file present but unusable — NEVER falls back to legacy
    /// (the .04a loader-trap rule); the server must refuse to start.
    Policy(PluginPolicyLoadErrorV1),
    ArchiveDirUnreadable { detail: String },
    Compile(PluginDeploymentCompileErrorV1),
}

/// Startup init. Missing policy file = `Legacy` (byte-identical live
/// behavior); present-but-invalid policy or any compile failure = HARD
/// startup error. `plugins_dir` is the same directory `PluginMgr`
/// discovers from (`<assets>/plugins/*.plugin.tar`).
pub fn init_plugin_deployment_v1(
    data_dir: &Path,
    plugins_dir: &Path,
) -> Result<PluginDeploymentStateV1, PluginDeploymentInitErrorV1> {
    use PluginDeploymentInitErrorV1 as E;
    let policy = match load_plugin_deployment_policy_strict_v1(data_dir) {
        Err(PluginPolicyLoadErrorV1::PolicyFileMissing { .. }) => return Ok(PluginDeploymentStateV1::Legacy),
        Err(other) => return Err(E::Policy(other)),
        Ok(policy) => policy,
    };

    // Collect archive bytes, filename-sorted for reproducible refusal
    // messages (the compile itself is input-order-invariant, proven).
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    if plugins_dir.is_dir() {
        for entry in std::fs::read_dir(plugins_dir).map_err(|e| E::ArchiveDirUnreadable { detail: e.to_string() })? {
            let entry = entry.map_err(|e| E::ArchiveDirUnreadable { detail: e.to_string() })?;
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(".plugin.tar")) {
                paths.push(path);
            }
        }
    }
    paths.sort();
    let archives: Vec<Vec<u8>> = paths
        .iter()
        .map(|p| std::fs::read(p).map_err(|e| E::ArchiveDirUnreadable { detail: format!("{}: {e}", p.display()) }))
        .collect::<Result<_, _>>()?;

    // Base content root: V1 uses the policy root as the stand-in until a
    // real base-content identity exists (recorded NEEDS-DEPLOYMENT-
    // EVIDENCE with the .04b family) — deliberate, disclosed, and it
    // still moves the deployment root whenever the policy moves.
    let base_content_root = policy.policy_root().map_err(|e| {
        E::Compile(PluginDeploymentCompileErrorV1::ActivationError(e))
    })?;
    let compiled =
        compile_deployment_from_archives_v1(&archives, &policy, base_content_root).map_err(E::Compile)?;

    let requirements: Vec<PluginArtifactDescriptorV1> = compiled
        .plan
        .nodes
        .iter()
        .map(|n| PluginArtifactDescriptorV1 {
            deployment_root: *compiled.deployment_root.bytes.as_array(),
            ordinal: n.ordinal,
            digest: *n.artifact.digest.bytes.as_array(),
            size_bytes: n.artifact.size_bytes,
        })
        .collect();
    let client_activation_root = compiled
        .client_plan
        .activation_root()
        .map_err(|e| E::Compile(PluginDeploymentCompileErrorV1::ActivationError(e)))?;
    let summary = PluginDeploymentSummaryV1 {
        deployment_root: *compiled.deployment_root.bytes.as_array(),
        requirements,
        client_activations: compiled.client_plan.activations.clone(),
        client_activation_root: *client_activation_root.bytes.as_array(),
    };
    Ok(PluginDeploymentStateV1::Deployed {
        summary,
        artifacts: compiled.artifacts.into_iter().map(|(o, b)| (o, std::sync::Arc::new(b))).collect(),
    })
}

#[cfg(test)]
mod deployment_state_tests {
    use super::*;

    #[test]
    fn missing_policy_is_legacy_and_invalid_policy_is_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let plugins = dir.path().join("plugins");
        // No policy file at all => Legacy, summary None.
        let state = init_plugin_deployment_v1(dir.path(), &plugins).unwrap();
        assert!(matches!(state, PluginDeploymentStateV1::Legacy));
        assert!(state.summary().is_none());

        // Present-but-broken policy => HARD error, never Legacy.
        let p = dir.path().join(POLICY_FILE);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "((((not ron").unwrap();
        assert!(matches!(
            init_plugin_deployment_v1(dir.path(), &plugins),
            Err(PluginDeploymentInitErrorV1::Policy(PluginPolicyLoadErrorV1::PolicyParse { .. }))
        ));
    }
}
