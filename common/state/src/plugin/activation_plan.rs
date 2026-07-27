//! `APEX-T2.5.02/.03` — deployment/activation plan, policy, and receipt
//! types with domain-separated roots (REAL packet §7 via Fable's brief;
//! 120-case catalog pin `bbc061fa…` in-repo).
//!
//! MECHANISM ONLY: every production VALUE fails closed —
//! `PluginPolicyPurposeV1::TestFixture` powers tests, `Production`
//! readiness is its own typed terminal. One deployment generation per
//! `State` construction; no hot-add. Roots: deployment/activation/policy/
//! receipt all under `PluginActivationPlan` (= 3, pre-registered) with
//! distinct SCHEMA TAGS bound into each payload, so no payload of one
//! kind can substitute for another even inside the shared domain
//! (packet .03 acceptance: "mode payloads cannot share an activation
//! root").

use super::manifest::PluginManifestLimitsV1;
use super::resolver::{PluginNodeKeyV1, ResolvedPluginGraphV1};
use common::apex::digest::{
    ArtifactIdentityV1, DigestDomainIdV1, ProtocolDigestV1, digest_manifest_value_v1,
};
use common::apex::manifest::{CanonicalFieldMapV1, CanonicalPathV1, FieldIdV1, MachineTextV1, ManifestEncodeV1, ManifestValueV1};

pub const PLUGIN_ACTIVATION_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PluginPolicyPurposeV1 {
    Production,
    TestFixture,
}

/// Packet §7: the only V1 runtime legacy posture.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PluginLegacyAdmissionV1 {
    StrictCanonicalOnly,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MultiplayerLocalPluginPolicyV1 {
    /// V1: multiplayer clients activate ONLY the server-derived Client
    /// projection; local extra plugins are rejected.
    RejectLocalPlugins,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PluginActivationModeV1 {
    Server,
    Client,
    /// Deferred: `GameMode::Singleplayer` is unused live; the projection
    /// exists in the type space, no plan is compiled for it in V1.
    SinglePlayer,
}

impl PluginActivationModeV1 {
    pub fn tag(self) -> u64 {
        match self {
            Self::Server => 0,
            Self::Client => 1,
            Self::SinglePlayer => 2,
        }
    }
}

/// Per-mode Wasmtime runtime limits — VALUES are NEEDS-DESIGN; the type
/// requires every field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginModeRuntimeLimitsV1 {
    pub mode: PluginActivationModeV1,
    pub max_linear_memory_bytes: u64,
    pub max_fuel_per_event: u64,
    pub max_instances: u32,
}

/// The resource namespace a conflict lives in.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PluginResourceKindV1 {
    Command,
    Body,
    Skeleton,
    AssetKey,
}

impl PluginResourceKindV1 {
    fn tag(&self) -> u64 {
        match self {
            Self::Command => 0,
            Self::Body => 1,
            Self::Skeleton => 2,
            Self::AssetKey => 3,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PluginResourceKeyV1 {
    pub kind: PluginResourceKindV1,
    pub name: MachineTextV1,
}

/// Packet §7 exact variants. `OrderedConcatenate` is legal ONLY for a
/// registered combinable schema — the combiner id names it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginConflictResolutionV1 {
    Reject,
    ExclusiveOwner { owner: PluginNodeKeyV1, displaced: Vec<PluginNodeKeyV1> },
    OrderedConcatenate { combiner_id: MachineTextV1, providers: Vec<PluginNodeKeyV1> },
}

/// One operator decision for one OBSERVED collision (collision-free sets
/// admit an empty decision list).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginConflictDecisionV1 {
    pub resource: PluginResourceKeyV1,
    pub claimants: Vec<PluginNodeKeyV1>,
    pub resolution: PluginConflictResolutionV1,
    pub policy_version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginPolicyOwnerIdV1(pub MachineTextV1);

/// The strict admission policy (packet §7). EVERY field mandatory; the
/// strict loader (`load_plugin_deployment_policy_strict_v1`, server
/// crate, .04+) is the ONLY production entry — never a serde-default
/// `Settings` field (live `Settings::load` fails open on parse errors,
/// the exact hazard the row forbids).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginDeploymentAdmissionPolicyV1 {
    pub schema_version: u32,
    pub purpose: PluginPolicyPurposeV1,
    pub archive_limits: super::archive_profile::ArchiveLimitsPolicyV1,
    pub manifest_limits: PluginManifestLimitsV1,
    pub runtime_limits_by_mode: Vec<PluginModeRuntimeLimitsV1>,
    pub legacy_admission: PluginLegacyAdmissionV1,
    pub conflict_decisions: Vec<PluginConflictDecisionV1>,
    pub multiplayer_local_plugin_policy: MultiplayerLocalPluginPolicyV1,
    pub policy_owner: PluginPolicyOwnerIdV1,
    pub policy_revision: u64,
}

/// One node of the deployment plan: exact artifact per ordinal.
#[derive(Clone, Debug)]
pub struct PluginDeploymentNodeV1 {
    pub ordinal: u32,
    pub key: PluginNodeKeyV1,
    pub artifact: ArtifactIdentityV1,
    pub archive_semantic_root: ProtocolDigestV1,
    pub manifest_root: ProtocolDigestV1,
    pub modules: Vec<PluginModuleDeploymentV1>,
}

#[derive(Clone, Debug)]
pub struct PluginModuleDeploymentV1 {
    pub path: CanonicalPathV1,
    pub declared_world: super::manifest::PluginModuleWorldV1,
}

/// The single shared deployment plan (packet §5.1: ONE per State
/// construction, byte-identical client/server).
#[derive(Clone, Debug)]
pub struct PluginDeploymentPlanV1 {
    pub schema_version: u32,
    pub graph_root: ProtocolDigestV1,
    pub policy_root: ProtocolDigestV1,
    pub base_content_root: ProtocolDigestV1,
    pub nodes: Vec<PluginDeploymentNodeV1>,
    pub conflict_decisions: Vec<PluginConflictDecisionV1>,
}

/// A mode projection tied to the deployment root.
#[derive(Clone, Debug)]
pub struct PluginActivationPlanV1 {
    pub mode: PluginActivationModeV1,
    pub deployment_root: ProtocolDigestV1,
    /// Ordinal-ordered subset of deployment nodes active in this mode.
    pub activations: Vec<u32>,
}

/// Frozen after the one lifecycle pass: actual registrations vs ceilings.
#[derive(Clone, Debug)]
pub struct PluginActivationReceiptV1 {
    pub deployment_root: ProtocolDigestV1,
    pub mode: PluginActivationModeV1,
    pub registrations: Vec<(u32, PluginResourceKeyV1)>,
    pub within_ceiling: bool,
    pub shadows: Vec<PluginResourceKeyV1>,
}

/// Artifact wire types (packet §7): ordinal + digest + size + bytes;
/// local paths / times / hostnames / transport order NEVER enter roots.
#[derive(Clone, Debug)]
pub struct PluginArtifactRequirementV1 {
    pub deployment_root: ProtocolDigestV1,
    pub ordinal: u32,
    pub artifact: ArtifactIdentityV1,
}

#[derive(Clone, Debug)]
pub struct PluginArtifactRequestV1 {
    pub deployment_root: ProtocolDigestV1,
    pub ordinal: u32,
    pub digest: ProtocolDigestV1,
}

// ---------------------------------------------------------------------------
// .03 — domain-separated roots. Shared domain 3, distinct schema tags.
// ---------------------------------------------------------------------------

const SCHEMA_POLICY: &str = "bastion.plugin-deployment-policy/v1";
const SCHEMA_PLAN: &str = "bastion.plugin-deployment-plan/v1";
const SCHEMA_ACTIVATION: &str = "bastion.plugin-activation-plan/v1";
const SCHEMA_RECEIPT: &str = "bastion.plugin-activation-receipt/v1";

struct W(ManifestValueV1);
impl ManifestEncodeV1 for W {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, common::apex::manifest::ManifestCodecErrorV1> {
        Ok(self.0.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginActivationErrorV1 {
    CanonicalizationFailure,
    NonAsciiIdentity,
}

fn text(s: &str) -> Result<ManifestValueV1, PluginActivationErrorV1> {
    Ok(ManifestValueV1::MachineText(
        MachineTextV1::new(s).map_err(|_| PluginActivationErrorV1::NonAsciiIdentity)?,
    ))
}

fn fmap(entries: Vec<(u16, ManifestValueV1)>) -> Result<ManifestValueV1, PluginActivationErrorV1> {
    CanonicalFieldMapV1::try_from_entries(entries.into_iter().map(|(i, v)| (FieldIdV1::new(i), v)).collect())
        .map(ManifestValueV1::Map)
        .map_err(|_| PluginActivationErrorV1::CanonicalizationFailure)
}

fn digest_bytes(d: &ProtocolDigestV1) -> ManifestValueV1 { ManifestValueV1::Bytes(d.bytes.as_array().to_vec()) }

fn key_value(k: &PluginNodeKeyV1) -> Result<ManifestValueV1, PluginActivationErrorV1> {
    fmap(vec![(0, text(k.plugin_id.as_str())?), (1, text(&k.plugin_version.get().to_string())?)])
}

fn resource_value(r: &PluginResourceKeyV1) -> Result<ManifestValueV1, PluginActivationErrorV1> {
    fmap(vec![(0, ManifestValueV1::Unsigned(r.kind.tag())), (1, ManifestValueV1::MachineText(r.name.clone()))])
}

fn resolution_value(r: &PluginConflictResolutionV1) -> Result<ManifestValueV1, PluginActivationErrorV1> {
    match r {
        PluginConflictResolutionV1::Reject => fmap(vec![(0, ManifestValueV1::Unsigned(0))]),
        PluginConflictResolutionV1::ExclusiveOwner { owner, displaced } => fmap(vec![
            (0, ManifestValueV1::Unsigned(1)),
            (1, key_value(owner)?),
            (2, ManifestValueV1::Array(displaced.iter().map(key_value).collect::<Result<_, _>>()?)),
        ]),
        PluginConflictResolutionV1::OrderedConcatenate { combiner_id, providers } => fmap(vec![
            (0, ManifestValueV1::Unsigned(2)),
            (1, ManifestValueV1::MachineText(combiner_id.clone())),
            (2, ManifestValueV1::Array(providers.iter().map(key_value).collect::<Result<_, _>>()?)),
        ]),
    }
}

fn decision_value(d: &PluginConflictDecisionV1) -> Result<ManifestValueV1, PluginActivationErrorV1> {
    fmap(vec![
        (0, resource_value(&d.resource)?),
        (1, ManifestValueV1::Array(d.claimants.iter().map(key_value).collect::<Result<_, _>>()?)),
        (2, resolution_value(&d.resolution)?),
        (3, ManifestValueV1::Unsigned(d.policy_version)),
    ])
}

fn root_of(value: ManifestValueV1) -> Result<ProtocolDigestV1, PluginActivationErrorV1> {
    let limits = super::archive_profile::plugin_archive_limits_v1();
    digest_manifest_value_v1(DigestDomainIdV1::PluginActivationPlan, &W(value), &limits)
        .map_err(|_| PluginActivationErrorV1::CanonicalizationFailure)
}

impl PluginDeploymentAdmissionPolicyV1 {
    /// `PluginDeploymentPolicyRootV1` — sorted conflict decisions;
    /// purpose is bound in (a TestFixture policy can never share a root
    /// with a Production one).
    pub fn policy_root(&self) -> Result<ProtocolDigestV1, PluginActivationErrorV1> {
        let mut decisions = self.conflict_decisions.clone();
        decisions.sort_by(|a, b| a.resource.cmp(&b.resource));
        let mut mode_limits = self.runtime_limits_by_mode.clone();
        mode_limits.sort_by_key(|m| m.mode.tag());
        let limits_v: Vec<ManifestValueV1> = mode_limits
            .iter()
            .map(|m| {
                fmap(vec![
                    (0, ManifestValueV1::Unsigned(m.mode.tag())),
                    (1, ManifestValueV1::Unsigned(m.max_linear_memory_bytes)),
                    (2, ManifestValueV1::Unsigned(m.max_fuel_per_event)),
                    (3, ManifestValueV1::Unsigned(m.max_instances as u64)),
                ])
            })
            .collect::<Result<_, _>>()?;
        let top = fmap(vec![
            (0, text(SCHEMA_POLICY)?),
            (1, ManifestValueV1::Unsigned(self.schema_version as u64)),
            (2, ManifestValueV1::Unsigned(matches!(self.purpose, PluginPolicyPurposeV1::TestFixture) as u64)),
            (3, ManifestValueV1::MachineText(self.archive_limits.policy_id.clone())),
            (4, ManifestValueV1::MachineText(self.manifest_limits.policy_id.clone())),
            (5, ManifestValueV1::Array(limits_v)),
            (6, ManifestValueV1::Unsigned(0)), // legacy_admission: StrictCanonicalOnly = 0 (sealed)
            (7, ManifestValueV1::Array(decisions.iter().map(decision_value).collect::<Result<_, _>>()?)),
            (8, ManifestValueV1::Unsigned(0)), // multiplayer: RejectLocalPlugins = 0 (sealed)
            (9, ManifestValueV1::MachineText(self.policy_owner.0.clone())),
            (10, ManifestValueV1::Unsigned(self.policy_revision)),
        ])?;
        root_of(top)
    }
}

impl PluginDeploymentPlanV1 {
    /// `PluginDeploymentPlanRootV1` — binds graph, policy, base content,
    /// exact artifacts by ordinal, and every conflict decision.
    pub fn deployment_root(&self) -> Result<ProtocolDigestV1, PluginActivationErrorV1> {
        let nodes: Vec<ManifestValueV1> = self
            .nodes
            .iter()
            .map(|n| {
                let modules: Vec<ManifestValueV1> = n
                    .modules
                    .iter()
                    .map(|m| fmap(vec![(0, text(m.path.as_str())?), (1, ManifestValueV1::Unsigned(world_tag(m.declared_world)))]))
                    .collect::<Result<_, _>>()?;
                fmap(vec![
                    (0, ManifestValueV1::Unsigned(n.ordinal as u64)),
                    (1, key_value(&n.key)?),
                    (2, ManifestValueV1::Bytes(n.artifact.digest.bytes.as_array().to_vec())),
                    (3, ManifestValueV1::Unsigned(n.artifact.size_bytes)),
                    (4, digest_bytes(&n.archive_semantic_root)),
                    (5, digest_bytes(&n.manifest_root)),
                    (6, ManifestValueV1::Array(modules)),
                ])
            })
            .collect::<Result<_, _>>()?;
        let mut decisions = self.conflict_decisions.clone();
        decisions.sort_by(|a, b| a.resource.cmp(&b.resource));
        let top = fmap(vec![
            (0, text(SCHEMA_PLAN)?),
            (1, ManifestValueV1::Unsigned(self.schema_version as u64)),
            (2, digest_bytes(&self.graph_root)),
            (3, digest_bytes(&self.policy_root)),
            (4, digest_bytes(&self.base_content_root)),
            (5, ManifestValueV1::Array(nodes)),
            (6, ManifestValueV1::Array(decisions.iter().map(decision_value).collect::<Result<_, _>>()?)),
        ])?;
        root_of(top)
    }
}

fn world_tag(w: super::manifest::PluginModuleWorldV1) -> u64 {
    use super::manifest::PluginModuleWorldV1 as W;
    match w {
        W::Plugin => 0,
        W::ServerPlugin => 1,
        W::AnimationPlugin => 2,
    }
}

impl PluginActivationPlanV1 {
    /// `PluginActivationPlanRootV1` — (deployment_root, mode tag,
    /// ordinal-ordered activations). Distinct schema tag + mode tag bound
    /// in: two modes can NEVER share an activation root even over the
    /// same deployment (packet .03 acceptance).
    pub fn activation_root(&self) -> Result<ProtocolDigestV1, PluginActivationErrorV1> {
        let top = fmap(vec![
            (0, text(SCHEMA_ACTIVATION)?),
            (1, digest_bytes(&self.deployment_root)),
            (2, ManifestValueV1::Unsigned(self.mode.tag())),
            (3, ManifestValueV1::Array(self.activations.iter().map(|&o| ManifestValueV1::Unsigned(o as u64)).collect())),
        ])?;
        root_of(top)
    }
}

impl PluginActivationReceiptV1 {
    pub fn receipt_root(&self) -> Result<ProtocolDigestV1, PluginActivationErrorV1> {
        let regs: Vec<ManifestValueV1> = self
            .registrations
            .iter()
            .map(|(ord, r)| fmap(vec![(0, ManifestValueV1::Unsigned(*ord as u64)), (1, resource_value(r)?)]))
            .collect::<Result<_, _>>()?;
        let shadows: Vec<ManifestValueV1> = self.shadows.iter().map(resource_value).collect::<Result<_, _>>()?;
        let top = fmap(vec![
            (0, text(SCHEMA_RECEIPT)?),
            (1, digest_bytes(&self.deployment_root)),
            (2, ManifestValueV1::Unsigned(self.mode.tag())),
            (3, ManifestValueV1::Array(regs)),
            (4, ManifestValueV1::Bool(self.within_ceiling)),
            (5, ManifestValueV1::Array(shadows)),
        ])?;
        root_of(top)
    }
}

/// .05 seam (typed early so .02's acceptance holds — later steps consume
/// roots, not loose vectors): compile the deployment plan as a PURE
/// function of graph + policy + base content root.
pub fn compile_deployment_plan_v1(
    graph: &ResolvedPluginGraphV1,
    policy: &PluginDeploymentAdmissionPolicyV1,
    base_content_root: ProtocolDigestV1,
) -> Result<PluginDeploymentPlanV1, PluginActivationErrorV1> {
    let policy_root = policy.policy_root()?;
    let nodes = graph
        .nodes
        .iter()
        .map(|n| PluginDeploymentNodeV1 {
            ordinal: n.ordinal,
            key: n.key.clone(),
            artifact: n.archive_artifact.clone(),
            archive_semantic_root: n.archive_semantic_root.clone(),
            manifest_root: n.manifest_root.clone(),
            modules: Vec::new(), // populated by .05's manifest join; typed seam now
        })
        .collect();
    Ok(PluginDeploymentPlanV1 {
        schema_version: PLUGIN_ACTIVATION_SCHEMA_VERSION_V1,
        graph_root: graph.graph_root.clone(),
        policy_root,
        base_content_root,
        nodes,
        conflict_decisions: policy.conflict_decisions.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::archive_profile::ArchiveLimitsPolicyV1;
    use super::*;
    use common::apex::digest::digest_canonical_bytes_v1;

    fn mtext(s: &str) -> MachineTextV1 { MachineTextV1::new(s).unwrap() }

    fn proto(p: &[u8]) -> ProtocolDigestV1 {
        digest_canonical_bytes_v1(DigestDomainIdV1::PluginActivationPlan, p, 1 << 20).unwrap()
    }

    fn fixture_policy() -> PluginDeploymentAdmissionPolicyV1 {
        PluginDeploymentAdmissionPolicyV1 {
            schema_version: PLUGIN_ACTIVATION_SCHEMA_VERSION_V1,
            purpose: PluginPolicyPurposeV1::TestFixture,
            archive_limits: ArchiveLimitsPolicyV1 {
                policy_id: mtext("apex-t2-5-testfixture-archive-v1"),
                max_archive_bytes: 1 << 20,
                max_entry_bytes: 1 << 18,
                max_entries: 64,
                max_path_bytes: 200,
                max_manifest_bytes: 1 << 14,
            },
            manifest_limits: PluginManifestLimitsV1 {
                policy_id: mtext("apex-t2-5-testfixture-manifest-v1"),
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
            runtime_limits_by_mode: vec![
                PluginModeRuntimeLimitsV1 {
                    mode: PluginActivationModeV1::Client,
                    max_linear_memory_bytes: 1 << 26,
                    max_fuel_per_event: 1 << 20,
                    max_instances: 8,
                },
                PluginModeRuntimeLimitsV1 {
                    mode: PluginActivationModeV1::Server,
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

    #[test]
    fn four_root_kinds_are_mutually_distinct() {
        let policy = fixture_policy();
        let policy_root = policy.policy_root().unwrap();
        let plan = PluginDeploymentPlanV1 {
            schema_version: 1,
            graph_root: proto(b"graph"),
            policy_root: policy_root.clone(),
            base_content_root: proto(b"base"),
            nodes: vec![],
            conflict_decisions: vec![],
        };
        let deployment_root = plan.deployment_root().unwrap();
        let activation = PluginActivationPlanV1 {
            mode: PluginActivationModeV1::Server,
            deployment_root: deployment_root.clone(),
            activations: vec![],
        };
        let activation_root = activation.activation_root().unwrap();
        let receipt = PluginActivationReceiptV1 {
            deployment_root: deployment_root.clone(),
            mode: PluginActivationModeV1::Server,
            registrations: vec![],
            within_ceiling: true,
            shadows: vec![],
        };
        let receipt_root = receipt.receipt_root().unwrap();
        let all = [&policy_root, &deployment_root, &activation_root, &receipt_root];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j], "root kinds {i}/{j} must never collide (schema tags bound in)");
            }
        }
    }

    #[test]
    fn mode_roots_never_shared_and_purpose_separates_policies() {
        let plan_root = proto(b"deployment");
        let server = PluginActivationPlanV1 {
            mode: PluginActivationModeV1::Server,
            deployment_root: plan_root.clone(),
            activations: vec![0, 1],
        };
        let client = PluginActivationPlanV1 {
            mode: PluginActivationModeV1::Client,
            deployment_root: plan_root,
            activations: vec![0, 1],
        };
        assert_ne!(
            server.activation_root().unwrap(),
            client.activation_root().unwrap(),
            "same deployment + same activations, different mode => different root"
        );

        let fixture = fixture_policy();
        let mut production = fixture.clone();
        production.purpose = PluginPolicyPurposeV1::Production;
        assert_ne!(
            fixture.policy_root().unwrap(),
            production.policy_root().unwrap(),
            "TestFixture and Production can never share a policy root"
        );
    }

    #[test]
    fn policy_root_is_order_invariant_over_decisions_and_mode_limits() {
        let key = |id: &str| PluginNodeKeyV1 {
            plugin_id: super::super::manifest::CanonicalPluginIdV1::parse(id, &fixture_policy().manifest_limits).unwrap(),
            plugin_version: super::super::manifest::PluginVersionV1::parse("1.0.0").unwrap(),
        };
        let d1 = PluginConflictDecisionV1 {
            resource: PluginResourceKeyV1 { kind: PluginResourceKindV1::Command, name: mtext("hello") },
            claimants: vec![key("x:a"), key("x:b")],
            resolution: PluginConflictResolutionV1::Reject,
            policy_version: 1,
        };
        let d2 = PluginConflictDecisionV1 {
            resource: PluginResourceKeyV1 { kind: PluginResourceKindV1::AssetKey, name: mtext("a.thing") },
            claimants: vec![key("x:a"), key("x:c")],
            resolution: PluginConflictResolutionV1::ExclusiveOwner { owner: key("x:a"), displaced: vec![key("x:c")] },
            policy_version: 1,
        };
        let mut p1 = fixture_policy();
        p1.conflict_decisions = vec![d1.clone(), d2.clone()];
        let mut p2 = fixture_policy();
        p2.conflict_decisions = vec![d2, d1];
        p2.runtime_limits_by_mode.reverse();
        assert_eq!(p1.policy_root().unwrap(), p2.policy_root().unwrap(), "declaration order never moves the policy root");
    }

    #[test]
    fn compile_is_pure_and_root_binds_policy() {
        use super::super::resolver::*;
        let graph = ResolvedPluginGraphV1 {
            resolver_version: 1,
            policy_root: proto(b"resolver-policy"),
            candidate_set_root: proto(b"candidates"),
            nodes: vec![],
            edges: vec![],
            graph_root: proto(b"graph"),
        };
        let policy = fixture_policy();
        let plan1 = compile_deployment_plan_v1(&graph, &policy, proto(b"base")).unwrap();
        let plan2 = compile_deployment_plan_v1(&graph, &policy, proto(b"base")).unwrap();
        assert_eq!(plan1.deployment_root().unwrap(), plan2.deployment_root().unwrap(), "pure function");

        let mut policy2 = policy.clone();
        policy2.policy_revision = 2;
        let plan3 = compile_deployment_plan_v1(&graph, &policy2, proto(b"base")).unwrap();
        assert_ne!(
            plan1.deployment_root().unwrap(),
            plan3.deployment_root().unwrap(),
            "a policy change moves EVERY deployment root"
        );
    }
}
