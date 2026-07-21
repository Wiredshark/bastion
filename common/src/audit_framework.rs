//! T1.9 (master build order; T1-001 packet, step 8): the layered parent
//! audit framework — NOT one full scan every tick.
//!
//! Three tiers by cost/cadence:
//! - EveryTick: cheap structural invariants (job ↔ ActiveJob symmetry,
//!   claimant uniqueness, reservation inverse links, pending-commit
//!   generations).
//! - Periodic: heavier reachability (item ownership, side-table
//!   reachability, terrain closure, projection mappings).
//! - HarnessFull: exhaustive (double-entry conservation, causal partial
//!   order).
//!
//! Violations carry STABLE codes (never renumbered — the recorder-kind-id
//! discipline) and are RECORDED, never silently repaired; production repair
//! is an explicit command issued AFTER the violation is recorded.
//!
//! Determinism story (Ben's law): a pure taxonomy + collection; the audit
//! observes and records, it never mutates authoritative state, so it cannot
//! itself perturb a deterministic run.

use serde::{Deserialize, Serialize};

/// The audit cadence tier.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditTier {
    EveryTick,
    Periodic,
    HarnessFull,
}

/// A stable audit violation code. The `u16` values are an APPEND-ONLY
/// registry — never renumbered, never reused (a golden pin guards this).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u16)]
pub enum AuditCode {
    // EveryTick (0x00xx)
    JobActiveJobAsymmetry = 0x0001,
    ClaimantNotUnique = 0x0002,
    ReservationInverseBroken = 0x0003,
    PendingCommitGenerationStale = 0x0004,
    // Periodic (0x01xx)
    ItemOwnershipOrphan = 0x0101,
    SideTableUnreachable = 0x0102,
    TerrainClosureBroken = 0x0103,
    ProjectionMappingBroken = 0x0104,
    // HarnessFull (0x02xx)
    ConservationImbalance = 0x0201,
    CausalOrderViolation = 0x0202,
}

impl AuditCode {
    /// The tier this code belongs to (its check runs at this cadence).
    pub fn tier(self) -> AuditTier {
        match self {
            AuditCode::JobActiveJobAsymmetry
            | AuditCode::ClaimantNotUnique
            | AuditCode::ReservationInverseBroken
            | AuditCode::PendingCommitGenerationStale => AuditTier::EveryTick,
            AuditCode::ItemOwnershipOrphan
            | AuditCode::SideTableUnreachable
            | AuditCode::TerrainClosureBroken
            | AuditCode::ProjectionMappingBroken => AuditTier::Periodic,
            AuditCode::ConservationImbalance | AuditCode::CausalOrderViolation => {
                AuditTier::HarnessFull
            },
        }
    }

    /// Every registered code — the append-only registry (order is the
    /// stable numbering, guarded by the pin).
    pub fn all() -> &'static [AuditCode] {
        &[
            AuditCode::JobActiveJobAsymmetry,
            AuditCode::ClaimantNotUnique,
            AuditCode::ReservationInverseBroken,
            AuditCode::PendingCommitGenerationStale,
            AuditCode::ItemOwnershipOrphan,
            AuditCode::SideTableUnreachable,
            AuditCode::TerrainClosureBroken,
            AuditCode::ProjectionMappingBroken,
            AuditCode::ConservationImbalance,
            AuditCode::CausalOrderViolation,
        ]
    }
}

/// One recorded violation — code + human detail. RECORDED, never repaired
/// in place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditViolation {
    pub code: AuditCode,
    pub detail: String,
}

/// A collector for one audit pass at a given tier. Runs only the checks
/// whose code belongs to `tier` (or a cheaper tier — a HarnessFull pass
/// includes EveryTick + Periodic checks).
#[derive(Clone, Debug, Default)]
pub struct AuditReport {
    pub violations: Vec<AuditViolation>,
}

impl AuditReport {
    /// Record a violation (the ONLY action an audit takes — no repair).
    pub fn record(&mut self, code: AuditCode, detail: impl Into<String>) {
        self.violations.push(AuditViolation {
            code,
            detail: detail.into(),
        });
    }

    pub fn is_clean(&self) -> bool { self.violations.is_empty() }
}

/// Whether a check at `code` should run during a pass at `pass_tier`
/// (a heavier pass subsumes cheaper tiers).
pub fn runs_in_pass(code: AuditCode, pass_tier: AuditTier) -> bool {
    code.tier() <= pass_tier
}

#[cfg(test)]
mod t1_9_tests {
    use super::*;

    #[test]
    fn t1_9_codes_are_stable_and_tiered() {
        // Stable numbering (append-only registry — never renumber).
        assert_eq!(AuditCode::JobActiveJobAsymmetry as u16, 0x0001);
        assert_eq!(AuditCode::ItemOwnershipOrphan as u16, 0x0101);
        assert_eq!(AuditCode::ConservationImbalance as u16, 0x0201);
        // No duplicate numeric codes.
        let mut seen = std::collections::BTreeSet::new();
        for code in AuditCode::all() {
            assert!(seen.insert(*code as u16), "duplicate audit code {code:?}");
        }
        assert_eq!(seen.len(), 10);
    }

    #[test]
    fn t1_9_pass_tier_subsumes_cheaper_checks() {
        // An every-tick pass runs only the cheap checks.
        assert!(runs_in_pass(
            AuditCode::ClaimantNotUnique,
            AuditTier::EveryTick
        ));
        assert!(!runs_in_pass(
            AuditCode::ConservationImbalance,
            AuditTier::EveryTick
        ));
        // A harness-full pass runs everything.
        assert!(runs_in_pass(
            AuditCode::ClaimantNotUnique,
            AuditTier::HarnessFull
        ));
        assert!(runs_in_pass(
            AuditCode::ConservationImbalance,
            AuditTier::HarnessFull
        ));
    }

    #[test]
    fn t1_9_report_records_never_repairs() {
        let mut report = AuditReport::default();
        assert!(report.is_clean());
        report.record(AuditCode::ReservationInverseBroken, "job 7 has no inverse");
        assert!(!report.is_clean());
        assert_eq!(report.violations[0].code, AuditCode::ReservationInverseBroken);
        // The report only holds records — there is no repair method on it.
    }
}
