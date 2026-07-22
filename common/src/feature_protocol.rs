//! T1.1 (master build order; T1-001 packet, step 1): the feature
//! protocol-declaration fitness gate.
//!
//! Every cross-system feature carries a [`FeatureProtocolDecl`] — the
//! executable mirror of the human-readable shared-engine registry. A
//! feature may not register a command without an owner, authoritative
//! domains, declared clocks, a transaction boundary, lifecycle /
//! persistence / LOD policies, and acceptance tests. [`validate`] enforces
//! that completeness; a CI/test golden over the registry is the gate
//! (the T0.12/25/48 fitness-function pattern).
//!
//! Decls-first: declarations DESCRIBE existing features and change no
//! runtime behavior; registration is later made to depend on them.
//!
//! Determinism story (Ben's law): pure data + pure validation over
//! sorted/keyed collections; no runtime effect, no RNG, no wall-clock.

use serde::{Deserialize, Serialize};

/// Stable feature identity (the registry key).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FeatureId(pub String);

/// The owning module (single-owner authority).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleId(pub String);

/// A command kind the feature admits.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CommandKind(pub String);

/// An acceptance-test contract id the feature must satisfy.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TestContractId(pub String);

/// The authoritative domains a feature may write — physical authority stays
/// in these owners (JobBoard is coordination, not a world database).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuthorityDomain {
    Terrain,
    Inventory,
    Ecs,
    Rtsim,
    JobBoardCoordination,
    Persistence,
    /// R0D §3A.7 (append-only, reviewer-approved): renderer PRESENTATION
    /// authority — snapshots, selection, captures. A feature holding only this
    /// domain can never write Terrain/Inventory/Ecs/Rtsim/coordination/
    /// persistence state; the renderer-r0d validator rule enforces that its
    /// authoritative_domains equal exactly [RendererPresentation].
    RendererPresentation,
}

/// The clock domains a feature reads (T0.4). Wall is diagnostic-only and
/// may not gate gameplay — a decl listing Wall as a gameplay clock is a
/// determinism-law violation flagged at validation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ClockDomain {
    Sim,
    World,
    Program,
    Wall,
    /// R0D §3A.7 (append-only, reviewer-approved): the render-frame clock —
    /// permitted only for presentation/capture progression, never to gate
    /// gameplay (the renderer-r0d validator enforces this scoping).
    RenderFrame,
}

/// How the feature's effects commit.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionBoundary {
    /// Purely local, single-command unit of work.
    InProcessUnitOfWork,
    /// Local commit + async release through a recoverable outbox.
    Outbox,
    /// Multi-participant custody via an orchestrated saga.
    Saga,
}

/// Server-issued capability the feature's commands require (T1.11).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CapabilityKind {
    Observe,
    Designate,
    FoundColony,
    SculptTerrain,
    Possess,
    ApplyInfluence,
    CastPower,
    AdminDebug,
}

/// Named policy ids (string tokens; the registry defines their meaning).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyId(pub String);

/// The observability contract — a feature that mutates authoritative state
/// must record (the recorder substrate from T0.56).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityContract {
    pub records_causal: bool,
    pub emits_command_status: bool,
}

/// T1.1: the per-feature protocol declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureProtocolDecl {
    pub feature: FeatureId,
    pub owner_module: ModuleId,
    pub authoritative_domains: Vec<AuthorityDomain>,
    pub command_types: Vec<CommandKind>,
    pub required_capabilities: Vec<CapabilityKind>,
    pub clock_domains: Vec<ClockDomain>,
    pub transaction_boundary: TransactionBoundary,
    pub lifecycle_policy: PolicyId,
    pub persistence_policy: PolicyId,
    pub lod_policy: PolicyId,
    pub observability: ObservabilityContract,
    pub acceptance_tests: Vec<TestContractId>,
}

/// A completeness violation the fitness gate reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolViolation {
    /// A field required whenever the feature admits commands is missing.
    MissingField {
        feature: String,
        field: &'static str,
    },
    /// Two declarations claim the same feature id (owner ambiguity).
    DuplicateFeature(String),
    /// A gameplay feature lists Wall as a clock (determinism-law breach).
    WallClockGameplay(String),
}

/// Validate ONE declaration's completeness (a feature that admits commands
/// must declare every protocol dimension).
pub fn validate(decl: &FeatureProtocolDecl) -> Vec<ProtocolViolation> {
    let mut violations = Vec::new();
    let feature = decl.feature.0.clone();
    let mut require = |ok: bool, field: &'static str, out: &mut Vec<ProtocolViolation>| {
        if !ok {
            out.push(ProtocolViolation::MissingField {
                feature: feature.clone(),
                field,
            });
        }
    };
    if decl.command_types.is_empty() {
        // A feature that admits no commands still needs an owner, but the
        // command-driven requirements don't apply.
        require(!decl.owner_module.0.is_empty(), "owner_module", &mut violations);
        return violations;
    }
    require(!decl.owner_module.0.is_empty(), "owner_module", &mut violations);
    require(
        !decl.authoritative_domains.is_empty(),
        "authoritative_domains",
        &mut violations,
    );
    require(!decl.clock_domains.is_empty(), "clock_domains", &mut violations);
    require(
        !decl.lifecycle_policy.0.is_empty(),
        "lifecycle_policy",
        &mut violations,
    );
    require(
        !decl.persistence_policy.0.is_empty(),
        "persistence_policy",
        &mut violations,
    );
    require(!decl.lod_policy.0.is_empty(), "lod_policy", &mut violations);
    require(
        decl.observability.emits_command_status,
        "observability.emits_command_status",
        &mut violations,
    );
    require(
        !decl.acceptance_tests.is_empty(),
        "acceptance_tests",
        &mut violations,
    );
    // Determinism-law: Wall is diagnostic-only; a gameplay feature may not
    // key on it. (A feature legitimately needing wall time for host safety
    // declares it via a diagnostic channel, not the gameplay clock list.)
    if decl.clock_domains.contains(&ClockDomain::Wall) {
        violations.push(ProtocolViolation::WallClockGameplay(feature));
    }
    violations
}

/// Validate a whole registry: per-decl completeness + no duplicate feature
/// ids. Returns violations sorted for a stable golden.
pub fn validate_registry(decls: &[FeatureProtocolDecl]) -> Vec<ProtocolViolation> {
    let mut violations = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for decl in decls {
        if !seen.insert(decl.feature.0.clone()) {
            violations.push(ProtocolViolation::DuplicateFeature(decl.feature.0.clone()));
        }
        violations.extend(validate(decl));
    }
    violations
}

#[cfg(test)]
mod t1_1_tests {
    use super::*;

    fn complete(feature: &str) -> FeatureProtocolDecl {
        FeatureProtocolDecl {
            feature: FeatureId(feature.to_string()),
            owner_module: ModuleId("bastion_jobs".to_string()),
            authoritative_domains: vec![AuthorityDomain::Terrain, AuthorityDomain::Inventory],
            command_types: vec![CommandKind("Dig".to_string())],
            required_capabilities: vec![CapabilityKind::Designate],
            clock_domains: vec![ClockDomain::Sim],
            transaction_boundary: TransactionBoundary::InProcessUnitOfWork,
            lifecycle_policy: PolicyId("job-lifecycle-v1".to_string()),
            persistence_policy: PolicyId("rtsim-save-v1".to_string()),
            lod_policy: PolicyId("loaded-gate-v1".to_string()),
            observability: ObservabilityContract {
                records_causal: true,
                emits_command_status: true,
            },
            acceptance_tests: vec![TestContractId("mf-scenario".to_string())],
        }
    }

    #[test]
    fn t1_1_complete_declaration_passes() {
        assert!(validate(&complete("terrain-work")).is_empty());
    }

    #[test]
    fn t1_1_incomplete_declaration_names_each_missing_field() {
        let mut decl = complete("bad");
        decl.clock_domains.clear();
        decl.acceptance_tests.clear();
        decl.observability.emits_command_status = false;
        let violations = validate(&decl);
        assert!(violations.contains(&ProtocolViolation::MissingField {
            feature: "bad".to_string(),
            field: "clock_domains",
        }));
        assert!(violations.contains(&ProtocolViolation::MissingField {
            feature: "bad".to_string(),
            field: "acceptance_tests",
        }));
        assert!(violations.contains(&ProtocolViolation::MissingField {
            feature: "bad".to_string(),
            field: "observability.emits_command_status",
        }));
    }

    #[test]
    fn t1_1_wall_clock_gameplay_is_rejected() {
        let mut decl = complete("wall-gated");
        decl.clock_domains = vec![ClockDomain::Wall];
        assert!(validate(&decl).contains(&ProtocolViolation::WallClockGameplay(
            "wall-gated".to_string()
        )));
    }

    #[test]
    fn t1_1_registry_rejects_duplicate_owner() {
        let decls = vec![complete("dup"), complete("dup")];
        assert!(
            validate_registry(&decls).contains(&ProtocolViolation::DuplicateFeature(
                "dup".to_string()
            ))
        );
    }

    /// The STARTER registry — god powers, quests, terrain work, inventory
    /// transfers — declared (describing existing features, no behavior
    /// change) and gated for completeness. New cross-system features append
    /// here; the gate keeps every one honest.
    #[test]
    fn t1_1_starter_registry_is_complete() {
        let registry = vec![
            complete("terrain-work"),
            FeatureProtocolDecl {
                feature: FeatureId("inventory-transfer".to_string()),
                owner_module: ModuleId("events::inventory_manip".to_string()),
                authoritative_domains: vec![AuthorityDomain::Inventory],
                command_types: vec![CommandKind("Pickup".to_string())],
                required_capabilities: vec![],
                clock_domains: vec![ClockDomain::Sim],
                transaction_boundary: TransactionBoundary::InProcessUnitOfWork,
                lifecycle_policy: PolicyId("item-lifecycle-v1".to_string()),
                persistence_policy: PolicyId("none-v1".to_string()),
                lod_policy: PolicyId("loaded-gate-v1".to_string()),
                observability: ObservabilityContract {
                    records_causal: false,
                    emits_command_status: true,
                },
                acceptance_tests: vec![TestContractId("loot-auth".to_string())],
            },
            FeatureProtocolDecl {
                feature: FeatureId("god-powers".to_string()),
                owner_module: ModuleId("bastion_god".to_string()),
                authoritative_domains: vec![AuthorityDomain::Terrain, AuthorityDomain::Rtsim],
                command_types: vec![CommandKind("CastPower".to_string())],
                required_capabilities: vec![CapabilityKind::CastPower],
                clock_domains: vec![ClockDomain::Sim, ClockDomain::World],
                transaction_boundary: TransactionBoundary::Outbox,
                lifecycle_policy: PolicyId("power-lifecycle-v1".to_string()),
                persistence_policy: PolicyId("rtsim-save-v1".to_string()),
                lod_policy: PolicyId("global-v1".to_string()),
                observability: ObservabilityContract {
                    records_causal: true,
                    emits_command_status: true,
                },
                acceptance_tests: vec![TestContractId("god-power-scenario".to_string())],
            },
            FeatureProtocolDecl {
                feature: FeatureId("quests".to_string()),
                owner_module: ModuleId("rtsim::quest".to_string()),
                authoritative_domains: vec![AuthorityDomain::Rtsim],
                command_types: vec![CommandKind("AcceptQuest".to_string())],
                required_capabilities: vec![],
                clock_domains: vec![ClockDomain::World],
                transaction_boundary: TransactionBoundary::InProcessUnitOfWork,
                lifecycle_policy: PolicyId("quest-lifecycle-v1".to_string()),
                persistence_policy: PolicyId("rtsim-save-v1".to_string()),
                lod_policy: PolicyId("rtsim-v1".to_string()),
                observability: ObservabilityContract {
                    records_causal: false,
                    emits_command_status: true,
                },
                acceptance_tests: vec![TestContractId("quest-timeout".to_string())],
            },
        ];
        assert_eq!(validate_registry(&registry), Vec::new());
    }
}
