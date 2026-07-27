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
    /// .05 join: a resolved node has no supplied manifest whose RECOMPUTED
    /// root matches the node's `manifest_root`. A stale or tampered
    /// manifest object is indistinguishable from a missing one — both
    /// refuse the whole plan.
    ManifestMissingForNode { ordinal: u32 },
    /// A supplied manifest failed its own root recomputation.
    ManifestRecomputeFailure,
    /// .08: V1 compiles no SinglePlayer projection — the mode exists in
    /// the type space only until live semantics are defined.
    SinglePlayerPlanUnsupported,
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

/// .05 — compile the deployment plan as a PURE function of graph +
/// policy + base content root + the validated manifests. The manifest
/// join is by RECOMPUTED root equality, never by plugin-id lookup: each
/// supplied manifest re-derives its own `manifest_semantic_root` and a
/// node joins only the manifest whose recomputed root equals the node's
/// `manifest_root` (a manifest object mutated after admission fails the
/// join instead of smuggling different modules under an admitted root).
pub fn compile_deployment_plan_v1(
    graph: &ResolvedPluginGraphV1,
    policy: &PluginDeploymentAdmissionPolicyV1,
    base_content_root: ProtocolDigestV1,
    manifests: &[super::manifest::ValidatedPluginManifestV1],
) -> Result<PluginDeploymentPlanV1, PluginActivationErrorV1> {
    let policy_root = policy.policy_root()?;
    let recomputed: Vec<(ProtocolDigestV1, &super::manifest::ValidatedPluginManifestV1)> = manifests
        .iter()
        .map(|m| {
            super::manifest::recompute_manifest_root(m)
                .map(|root| (root, m))
                .map_err(|_| PluginActivationErrorV1::ManifestRecomputeFailure)
        })
        .collect::<Result<_, _>>()?;
    let nodes = graph
        .nodes
        .iter()
        .map(|n| {
            let manifest = recomputed
                .iter()
                .find(|(root, _)| *root == n.manifest_root)
                .map(|(_, m)| *m)
                .ok_or(PluginActivationErrorV1::ManifestMissingForNode { ordinal: n.ordinal })?;
            Ok(PluginDeploymentNodeV1 {
                ordinal: n.ordinal,
                key: n.key.clone(),
                artifact: n.archive_artifact.clone(),
                archive_semantic_root: n.archive_semantic_root.clone(),
                manifest_root: n.manifest_root.clone(),
                modules: manifest
                    .modules
                    .iter()
                    .map(|md| PluginModuleDeploymentV1 { path: md.path.clone(), declared_world: md.world })
                    .collect(),
            })
        })
        .collect::<Result<_, PluginActivationErrorV1>>()?;
    Ok(PluginDeploymentPlanV1 {
        schema_version: PLUGIN_ACTIVATION_SCHEMA_VERSION_V1,
        graph_root: graph.graph_root.clone(),
        policy_root,
        base_content_root,
        nodes,
        conflict_decisions: policy.conflict_decisions.clone(),
    })
}

/// .08 — the pure mode projection: ordinal-ordered subset of deployment
/// nodes with at least one module whose declared world runs in `mode`.
/// Node order comes from the resolver's canonical ordinals — never from
/// discovery, transport, or container order.
pub fn compile_mode_activation_plan_v1(
    plan: &PluginDeploymentPlanV1,
    mode: PluginActivationModeV1,
) -> Result<PluginActivationPlanV1, PluginActivationErrorV1> {
    if matches!(mode, PluginActivationModeV1::SinglePlayer) {
        return Err(PluginActivationErrorV1::SinglePlayerPlanUnsupported);
    }
    let mut activations: Vec<u32> = plan
        .nodes
        .iter()
        .filter(|n| n.modules.iter().any(|m| world_active_in_mode(m.declared_world, mode)))
        .map(|n| n.ordinal)
        .collect();
    activations.sort_unstable();
    Ok(PluginActivationPlanV1 { mode, deployment_root: plan.deployment_root()?, activations })
}

// ---------------------------------------------------------------------------
// .07 — operator-owned conflict compilation. Claims arrive pre-expanded
// to exact resource keys (.06 for assets, manifest runtime claims for
// commands/animations); NOTHING here may resolve by container order.
// ---------------------------------------------------------------------------

/// One exact claim: this plugin publishes/owns this resource.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PluginClaimV1 {
    pub resource: PluginResourceKeyV1,
    pub claimant: PluginNodeKeyV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginConflictErrorV1 {
    /// ≥2 claimants and no operator decision — never auto-resolved.
    UnresolvedCollision { resource: PluginResourceKeyV1, claimants: Vec<PluginNodeKeyV1> },
    /// V1: no decision can authorize shadowing a base-game resource.
    BaseResourceShadowingForbidden { resource: PluginResourceKeyV1, claimant: PluginNodeKeyV1 },
    /// A decision for a resource with NO current collision — the policy
    /// no longer matches the deployment; refuse instead of ignoring.
    StaleDecision { resource: PluginResourceKeyV1 },
    /// The decision's recorded claimant set differs from the observed one.
    DecisionClaimantMismatch { resource: PluginResourceKeyV1 },
    /// Resolution internally inconsistent with the observed claimants.
    DecisionResolutionInvalid { resource: PluginResourceKeyV1, detail: &'static str },
    /// The operator ruled this collision unacceptable: deployment refused.
    OperatorRejectedCollision { resource: PluginResourceKeyV1 },
    /// Two decisions naming the same resource: ambiguous policy, refused
    /// (picking either would be container-order resolution).
    DuplicateDecision { resource: PluginResourceKeyV1 },
}

/// .08 — which module worlds run in which mode. SinglePlayer is a schema
/// placeholder: no V1 plan is compiled for it.
fn world_active_in_mode(world: super::manifest::PluginModuleWorldV1, mode: PluginActivationModeV1) -> bool {
    use super::manifest::PluginModuleWorldV1 as W;
    match world {
        W::Plugin => matches!(mode, PluginActivationModeV1::Server | PluginActivationModeV1::Client),
        W::ServerPlugin => matches!(mode, PluginActivationModeV1::Server),
        W::AnimationPlugin => matches!(mode, PluginActivationModeV1::Client),
    }
}

/// A collision the operator resolved; consumed by mode projection (.08+).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginResolvedCollisionV1 {
    pub resource: PluginResourceKeyV1,
    pub claimants: Vec<PluginNodeKeyV1>,
    pub resolution: PluginConflictResolutionV1,
}

/// Pure conflict compilation. Errors are reported in canonical resource
/// order — input permutation can never move the outcome (acceptance: no
/// collision resolved by container order).
pub fn resolve_claim_conflicts_v1(
    claims: &[PluginClaimV1],
    base_resources: &[PluginResourceKeyV1],
    decisions: &[PluginConflictDecisionV1],
) -> Result<Vec<PluginResolvedCollisionV1>, PluginConflictErrorV1> {
    let mut claims: Vec<PluginClaimV1> = claims.to_vec();
    claims.sort();
    claims.dedup(); // the same plugin claiming a resource in two modes is one claim

    // Base shadowing first: unconditional, decision-independent.
    for c in &claims {
        if base_resources.contains(&c.resource) {
            return Err(PluginConflictErrorV1::BaseResourceShadowingForbidden {
                resource: c.resource.clone(),
                claimant: c.claimant.clone(),
            });
        }
    }

    // Group into collisions (sorted input => groups and members ordered).
    let mut collisions: Vec<(PluginResourceKeyV1, Vec<PluginNodeKeyV1>)> = Vec::new();
    for c in &claims {
        match collisions.last_mut() {
            Some((res, members)) if *res == c.resource => members.push(c.claimant.clone()),
            _ => collisions.push((c.resource.clone(), vec![c.claimant.clone()])),
        }
    }
    collisions.retain(|(_, members)| members.len() >= 2);

    // Every decision must match a live collision exactly; every collision
    // must have a decision; no resource may carry two decisions.
    let mut decision_resources: Vec<&PluginResourceKeyV1> = decisions.iter().map(|d| &d.resource).collect();
    decision_resources.sort();
    for pair in decision_resources.windows(2) {
        if pair[0] == pair[1] {
            return Err(PluginConflictErrorV1::DuplicateDecision { resource: pair[0].clone() });
        }
    }
    let mut resolved = Vec::with_capacity(collisions.len());
    for (resource, claimants) in &collisions {
        let decision = decisions
            .iter()
            .find(|d| d.resource == *resource)
            .ok_or_else(|| PluginConflictErrorV1::UnresolvedCollision {
                resource: resource.clone(),
                claimants: claimants.clone(),
            })?;
        let mut recorded = decision.claimants.clone();
        recorded.sort();
        if recorded != *claimants {
            return Err(PluginConflictErrorV1::DecisionClaimantMismatch { resource: resource.clone() });
        }
        let invalid = |detail| PluginConflictErrorV1::DecisionResolutionInvalid { resource: resource.clone(), detail };
        match &decision.resolution {
            PluginConflictResolutionV1::Reject => {
                return Err(PluginConflictErrorV1::OperatorRejectedCollision { resource: resource.clone() });
            },
            PluginConflictResolutionV1::ExclusiveOwner { owner, displaced } => {
                if !claimants.contains(owner) {
                    return Err(invalid("owner is not a claimant"));
                }
                let mut expected: Vec<PluginNodeKeyV1> =
                    claimants.iter().filter(|k| *k != owner).cloned().collect();
                expected.sort();
                let mut got = displaced.clone();
                got.sort();
                if got != expected {
                    return Err(invalid("displaced must be exactly the non-owner claimants"));
                }
            },
            PluginConflictResolutionV1::OrderedConcatenate { providers, .. } => {
                let mut got = providers.clone();
                got.sort();
                if got != *claimants {
                    return Err(invalid("providers must be exactly the claimant set"));
                }
            },
        }
        resolved.push(PluginResolvedCollisionV1 {
            resource: resource.clone(),
            claimants: claimants.clone(),
            resolution: decision.resolution.clone(),
        });
    }
    for d in decisions {
        if !collisions.iter().any(|(res, _)| *res == d.resource) {
            return Err(PluginConflictErrorV1::StaleDecision { resource: d.resource.clone() });
        }
    }
    Ok(resolved)
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
        let plan1 = compile_deployment_plan_v1(&graph, &policy, proto(b"base"), &[]).unwrap();
        let plan2 = compile_deployment_plan_v1(&graph, &policy, proto(b"base"), &[]).unwrap();
        assert_eq!(plan1.deployment_root().unwrap(), plan2.deployment_root().unwrap(), "pure function");

        let mut policy2 = policy.clone();
        policy2.policy_revision = 2;
        let plan3 = compile_deployment_plan_v1(&graph, &policy2, proto(b"base"), &[]).unwrap();
        assert_ne!(
            plan1.deployment_root().unwrap(),
            plan3.deployment_root().unwrap(),
            "a policy change moves EVERY deployment root"
        );
    }

    /// .05 — real T2.3 validate → real T2.4 resolve → compile join.
    #[test]
    fn deployment_join_fills_modules_by_recomputed_root_and_refuses_otherwise() {
        use super::super::archive_profile::CanonicalEntryV1;
        use super::super::manifest::*;
        use super::super::resolver::*;
        use common::apex::digest::hash_artifact_bytes_v1;
        use common::apex::manifest::CanonicalPathV1;

        let toml = "manifest_version = 1\ndependencies = []\n\n[plugin]\nid = \"x:solo\"\nversion = \"1.0.0\"\n\
                    host_api = \"veloren:plugin@0.0.1\"\n\n[[modules]]\npath = \"modules/solo.wasm\"\n\
                    world = \"server-plugin\"\n\n[claims]\nasset_roots = []\n\n\
                    [[claims.runtime]]\nmode = \"server\"\ncommands = []\nanimations = []\n";
        let ns: Vec<CanonicalEntryV1> = ["plugin.toml", "modules/solo.wasm"]
            .iter()
            .map(|p| CanonicalEntryV1 {
                path: CanonicalPathV1::new(*p).unwrap(),
                portability_key: mtext(p),
                size_bytes: 1,
                content_sha256: [7; 32],
            })
            .collect();
        let art = hash_artifact_bytes_v1(toml.as_bytes());
        let archive_root = proto(b"archive-root");
        let admission = validate_plugin_manifest_v1(
            toml.as_bytes(),
            &ns,
            &art,
            &archive_root,
            &fixture_policy().manifest_limits,
            PluginManifestEnforcementModeV1::StrictV1,
            archive_root.clone(),
        )
        .unwrap();
        let validated = match &admission {
            PluginManifestAdmissionV1::ValidatedV1(v) => (**v).clone(),
            other => panic!("{other:?}"),
        };
        let resolver_policy = PluginResolverPolicyV1 {
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
            policy_root: proto(b"resolver-policy"),
        };
        let graph = match resolve_plugin_graph_v1(vec![admission], &resolver_policy) {
            PluginResolutionTerminalV1::Resolved(g) => g,
            PluginResolutionTerminalV1::Rejected(r) => panic!("{:?}", r.errors),
        };

        let policy = fixture_policy();
        let plan =
            compile_deployment_plan_v1(&graph, &policy, proto(b"base"), std::slice::from_ref(&validated)).unwrap();
        assert_eq!(plan.nodes.len(), 1);
        assert_eq!(plan.nodes[0].modules.len(), 1, "the join must fill modules from the validated manifest");
        assert_eq!(plan.nodes[0].modules[0].path.as_str(), "modules/solo.wasm");
        assert_eq!(plan.nodes[0].modules[0].declared_world, PluginModuleWorldV1::ServerPlugin);

        // No manifest supplied => typed refusal, never an empty-module node.
        assert!(matches!(
            compile_deployment_plan_v1(&graph, &policy, proto(b"base"), &[]),
            Err(PluginActivationErrorV1::ManifestMissingForNode { ordinal: 0 })
        ));

        // A manifest mutated AFTER admission recomputes to a different
        // root and fails the join — modules can't be smuggled under an
        // admitted root.
        let mut tampered = validated;
        tampered.modules.clear();
        assert!(matches!(
            compile_deployment_plan_v1(&graph, &policy, proto(b"base"), &[tampered]),
            Err(PluginActivationErrorV1::ManifestMissingForNode { ordinal: 0 })
        ));
    }

    /// .07 — no collision resolved by container order.
    #[test]
    fn plugin_conflict_policy_v1_refuses_everything_but_exact_operator_decisions() {
        let key = |id: &str| PluginNodeKeyV1 {
            plugin_id: super::super::manifest::CanonicalPluginIdV1::parse(id, &fixture_policy().manifest_limits)
                .unwrap(),
            plugin_version: super::super::manifest::PluginVersionV1::parse("1.0.0").unwrap(),
        };
        let res = |kind: PluginResourceKindV1, name: &str| PluginResourceKeyV1 { kind, name: mtext(name) };
        let cmd = res(PluginResourceKindV1::Command, "hello");
        let anim = res(PluginResourceKindV1::Skeleton, "wave");
        let claim = |r: &PluginResourceKeyV1, k: &str| PluginClaimV1 { resource: r.clone(), claimant: key(k) };
        let decision = |r: &PluginResourceKeyV1, claimants: &[&str], resolution| PluginConflictDecisionV1 {
            resource: r.clone(),
            claimants: claimants.iter().map(|k| key(k)).collect(),
            resolution,
            policy_version: 1,
        };

        // Plugin/plugin command collision with an exact ExclusiveOwner
        // decision resolves — and is permutation-invariant.
        let claims = [claim(&cmd, "x:a"), claim(&cmd, "x:b"), claim(&anim, "x:a")];
        let mut permuted = claims.to_vec();
        permuted.reverse();
        let decisions = [
            decision(&cmd, &["x:a", "x:b"], PluginConflictResolutionV1::ExclusiveOwner {
                owner: key("x:a"),
                displaced: vec![key("x:b")],
            }),
        ];
        let r1 = resolve_claim_conflicts_v1(&claims, &[], &decisions).unwrap();
        let r2 = resolve_claim_conflicts_v1(&permuted, &[], &decisions).unwrap();
        assert_eq!(r1, r2, "claim container order can never move the outcome");
        assert_eq!(r1.len(), 1, "the solo animation claim is no collision");
        assert_eq!(r1[0].resource, cmd);

        // No decision => unresolved, never first/last-wins.
        assert!(matches!(
            resolve_claim_conflicts_v1(&claims, &[], &[]),
            Err(PluginConflictErrorV1::UnresolvedCollision { .. })
        ));
        // Base/plugin: no decision can authorize shadowing base.
        assert!(matches!(
            resolve_claim_conflicts_v1(&claims, std::slice::from_ref(&anim), &decisions),
            Err(PluginConflictErrorV1::BaseResourceShadowingForbidden { .. })
        ));
        // Claimant drift => mismatch (stale set recorded in the policy).
        let drifted = [claim(&cmd, "x:a"), claim(&cmd, "x:c")];
        assert!(matches!(
            resolve_claim_conflicts_v1(&drifted, &[], &decisions),
            Err(PluginConflictErrorV1::DecisionClaimantMismatch { .. })
        ));
        // Decision without a live collision => stale.
        assert!(matches!(
            resolve_claim_conflicts_v1(&[claim(&anim, "x:a")], &[], &decisions),
            Err(PluginConflictErrorV1::StaleDecision { .. })
        ));
        // Operator Reject terminal.
        assert!(matches!(
            resolve_claim_conflicts_v1(&claims, &[], &[decision(&cmd, &["x:a", "x:b"], PluginConflictResolutionV1::Reject)]),
            Err(PluginConflictErrorV1::OperatorRejectedCollision { .. })
        ));
        // Two decisions on one resource => ambiguous, refused.
        let dup = [
            decision(&cmd, &["x:a", "x:b"], PluginConflictResolutionV1::Reject),
            decision(&cmd, &["x:a", "x:b"], PluginConflictResolutionV1::ExclusiveOwner {
                owner: key("x:a"),
                displaced: vec![key("x:b")],
            }),
        ];
        assert!(matches!(
            resolve_claim_conflicts_v1(&claims, &[], &dup),
            Err(PluginConflictErrorV1::DuplicateDecision { .. })
        ));
        // OrderedConcatenate must name the claimant set exactly.
        assert!(matches!(
            resolve_claim_conflicts_v1(&claims, &[], &[decision(
                &cmd,
                &["x:a", "x:b"],
                PluginConflictResolutionV1::OrderedConcatenate { combiner_id: mtext("cat"), providers: vec![key("x:a")] }
            )]),
            Err(PluginConflictErrorV1::DecisionResolutionInvalid { .. })
        ));
    }

    /// .08 — mode projections.
    #[test]
    fn plugin_mode_activation_plans_v1_split_by_declared_world() {
        use super::super::manifest::{CanonicalPluginIdV1, PluginModuleWorldV1, PluginVersionV1};
        use common::apex::digest::hash_artifact_bytes_v1;
        use common::apex::manifest::CanonicalPathV1;

        let node = |ordinal: u32, id: &str, worlds: &[PluginModuleWorldV1]| PluginDeploymentNodeV1 {
            ordinal,
            key: PluginNodeKeyV1 {
                plugin_id: CanonicalPluginIdV1::parse(id, &fixture_policy().manifest_limits).unwrap(),
                plugin_version: PluginVersionV1::parse("1.0.0").unwrap(),
            },
            artifact: hash_artifact_bytes_v1(id.as_bytes()),
            archive_semantic_root: proto(id.as_bytes()),
            manifest_root: proto(id.as_bytes()),
            modules: worlds
                .iter()
                .map(|w| PluginModuleDeploymentV1 { path: CanonicalPathV1::new("m.wasm").unwrap(), declared_world: *w })
                .collect(),
        };
        let plan = PluginDeploymentPlanV1 {
            schema_version: PLUGIN_ACTIVATION_SCHEMA_VERSION_V1,
            graph_root: proto(b"graph"),
            policy_root: fixture_policy().policy_root().unwrap(),
            base_content_root: proto(b"base"),
            nodes: vec![
                node(0, "x:server-only", &[PluginModuleWorldV1::ServerPlugin]),
                node(1, "x:both", &[PluginModuleWorldV1::Plugin]),
                node(2, "x:anim", &[PluginModuleWorldV1::AnimationPlugin]),
            ],
            conflict_decisions: vec![],
        };

        let server = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server).unwrap();
        let client = compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Client).unwrap();
        assert_eq!(server.activations, vec![0, 1], "server-only + shared");
        assert_eq!(client.activations, vec![1, 2], "server-only module absent from the client plan");
        assert_eq!(server.deployment_root, client.deployment_root, "both projections tie to ONE deployment");
        assert_ne!(server.activation_root().unwrap(), client.activation_root().unwrap());
        // Stable: recompilation reproduces the roots byte-identically.
        assert_eq!(
            compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::Server)
                .unwrap()
                .activation_root()
                .unwrap(),
            server.activation_root().unwrap()
        );
        assert!(matches!(
            compile_mode_activation_plan_v1(&plan, PluginActivationModeV1::SinglePlayer),
            Err(PluginActivationErrorV1::SinglePlayerPlanUnsupported)
        ));
    }
}
