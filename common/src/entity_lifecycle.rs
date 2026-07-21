//! T2.4/T2.7 + T2.8 + T2.9 (master build order; T2 lifecycle group,
//! reuse-first cluster): entity incarnation guards, the reason-coded
//! lifecycle event, and the loaded-gate tri-state.
//!
//! These REUSE the T0-004/T1 substrate directly rather than inventing
//! parallel schemes:
//! - the incarnation guard is [`crate::async_work::AsyncOwnerKey`]'s
//!   incarnation pattern applied to entity targeting,
//! - the reason-coded lifecycle event mirrors T1.10's terminal-status shape
//!   and feeds T0.56 causal records,
//! - determinism-by-construction: pure types + pure validation, sim-only,
//!   no RNG, no wall-clock.

use serde::{Deserialize, Serialize};

/// T2.4/T2.7: an entity's incarnation stamp — WHO (stable uid) in WHICH
/// life and ownership epoch. A target reference must match the current
/// incarnation to be valid (the AsyncOwnerKey barrier, applied to entity
/// targeting: a reference to a dead-and-respawned or reowned entity is
/// stale by construction).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityIncarnation {
    /// The permanent, generation-safe identity (never a recycling ECS id).
    pub stable_uid: u64,
    /// Bumped on death/respawn/recreation.
    pub life_generation: u64,
    /// Bumped on ownership change (mount, possession, pet transfer).
    pub ownership_generation: u64,
}

/// The state a targeting command expects its target to be in.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedTargetState {
    Any,
    Alive,
    Loaded,
}

/// Why an incarnation-guarded target reference was rejected.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IncarnationMismatch {
    /// No entity with that stable uid exists now.
    Gone,
    /// The entity died/respawned since the reference was taken.
    StaleLife,
    /// Ownership changed since the reference was taken.
    StaleOwnership,
    /// The target isn't in the expected state.
    WrongState,
}

/// T2.4/T2.7: validate a target reference against the live incarnation +
/// expected state. This is the ONLY authoritative resolution path — the
/// same barrier the async acceptance predicate uses, applied to entities.
pub fn resolve_target(
    reference: EntityIncarnation,
    current: Option<EntityIncarnation>,
    expected: ExpectedTargetState,
    is_in_state: impl FnOnce(ExpectedTargetState) -> bool,
) -> Result<(), IncarnationMismatch> {
    let Some(current) = current else {
        return Err(IncarnationMismatch::Gone);
    };
    if current.stable_uid != reference.stable_uid
        || current.life_generation != reference.life_generation
    {
        return Err(IncarnationMismatch::StaleLife);
    }
    if current.ownership_generation != reference.ownership_generation {
        return Err(IncarnationMismatch::StaleOwnership);
    }
    if expected != ExpectedTargetState::Any && !is_in_state(expected) {
        return Err(IncarnationMismatch::WrongState);
    }
    Ok(())
}

/// T2.8: the one aggregate lifecycle transition, reason-coded — spawn,
/// promotion, demotion, unload, death, despawn, mount, possession, restart.
/// Feeds T0.56 causal records (as the `kind`) and carries a stable reason
/// so telemetry never has to infer WHY a transition happened.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleTransition {
    Spawn,
    Promotion,
    Demotion,
    Unload,
    Death,
    Despawn,
    Mount,
    Possession,
    Restart,
}

/// A reason-coded lifecycle event — the subject's incarnation + the
/// transition + a stable reason code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub subject: EntityIncarnation,
    pub transition: LifecycleTransition,
    /// A stable reason code (never renumbered — the audit-code discipline).
    pub reason: u16,
    pub tick: u64,
}

/// T2.9: the loaded-gate TRI-STATE — a boolean "loaded?" conflated two
/// genuinely different failures (an NPC that is legitimately unloaded vs
/// one whose RTSim↔ECS linkage is missing or stale). The reconciliation
/// loop (T2.11) acts differently on each.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadedLinkage {
    /// Known-loaded: an ECS entity exists and the IdMaps link resolves.
    KnownLoaded,
    /// Legitimately unloaded (Simulated) — no ECS entity expected.
    Unloaded,
    /// Broken: the NPC is marked Loaded but the ECS link is missing or
    /// stale — a reconciliation target, not a normal state.
    MissingOrStaleLink,
}

impl LoadedLinkage {
    /// Whether this linkage needs reconciliation (T2.11).
    pub fn needs_reconciliation(self) -> bool {
        matches!(self, LoadedLinkage::MissingOrStaleLink)
    }
}

#[cfg(test)]
mod t2_lifecycle_tests {
    use super::*;

    fn inc(uid: u64, life: u64, owner: u64) -> EntityIncarnation {
        EntityIncarnation {
            stable_uid: uid,
            life_generation: life,
            ownership_generation: owner,
        }
    }

    #[test]
    fn t2_4_incarnation_guard_rejects_stale_references() {
        let reference = inc(7, 1, 0);
        // Current, in state → Ok.
        assert_eq!(
            resolve_target(reference, Some(inc(7, 1, 0)), ExpectedTargetState::Loaded, |_| true),
            Ok(())
        );
        // Gone.
        assert_eq!(
            resolve_target(reference, None, ExpectedTargetState::Any, |_| true),
            Err(IncarnationMismatch::Gone)
        );
        // Respawned (life bumped).
        assert_eq!(
            resolve_target(reference, Some(inc(7, 2, 0)), ExpectedTargetState::Any, |_| true),
            Err(IncarnationMismatch::StaleLife)
        );
        // Reowned.
        assert_eq!(
            resolve_target(reference, Some(inc(7, 1, 5)), ExpectedTargetState::Any, |_| true),
            Err(IncarnationMismatch::StaleOwnership)
        );
        // Wrong state.
        assert_eq!(
            resolve_target(reference, Some(inc(7, 1, 0)), ExpectedTargetState::Loaded, |_| false),
            Err(IncarnationMismatch::WrongState)
        );
    }

    #[test]
    fn t2_9_tri_state_flags_only_broken_links() {
        assert!(!LoadedLinkage::KnownLoaded.needs_reconciliation());
        assert!(!LoadedLinkage::Unloaded.needs_reconciliation());
        assert!(LoadedLinkage::MissingOrStaleLink.needs_reconciliation());
    }

    #[test]
    fn t2_8_lifecycle_event_is_reason_coded_and_serializable() {
        let event = LifecycleEvent {
            subject: inc(7, 1, 0),
            transition: LifecycleTransition::Promotion,
            reason: 0x0001,
            tick: 42,
        };
        // Round-trips (persistable telemetry).
        let bytes = serde_json::to_vec(&event).unwrap();
        let decoded: LifecycleEvent = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, event);
        assert_eq!(decoded.transition, LifecycleTransition::Promotion);
    }
}
