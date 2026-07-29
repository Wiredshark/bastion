//! `APEX-T8.5` — `EconomicNumericProtocolV1`: the decision row that
//! consumes T8.1/T8.3/T8.4's lane evidence and picks the smallest remedy
//! rung the *measured* evidence justifies, per
//! `readme/apex/APEX-T8-TIER-SPEC-FLEET-v1.md`'s own ladder (section
//! `T8.5`).
//!
//! **Why this is code and not prose.** The tier spec forbids choosing a
//! remedy "by preference" (T8.4's acceptance criterion, reused here). A
//! rung picked by a hand-written sentence cannot be re-run once T8.2
//! (still pending, Fable-deferred) lands real cross-platform evidence; a
//! rung DERIVED from a typed [`EconomicSensitivityEvidenceV1`] can be
//! re-evaluated by constructing a new evidence value and re-running
//! [`economic_numeric_protocol_rung_for_evidence_v1`] -- no prose to
//! re-read and re-interpret.
//!
//! **The cache-vs-history boundary, declared before any re-derivation
//! rung is chosen (the row's own hard constraint).** Rung 1 -- the rung
//! [`t8_5_current_decision_v1`] actually selects, see below -- needs no
//! re-derivation at all, so this boundary is not load-bearing for TODAY's
//! decision. It is declared anyway, as the row asks, because it is
//! exactly "the evidence that would force a higher rung": every rung from
//! 2 upward re-derives *something*, and re-deriving a `History` field
//! instead of a `Cache` field is precisely the silent-world-rewrite this
//! row exists to prevent. [`ECONOMIC_CACHE_HISTORY_BOUNDARY_V1`] is that
//! declaration, one entry per numeric-bearing `Economy` field
//! (`world/src/site/economy/mod.rs`), each with the concrete evidence
//! that justifies its class.
//!
//! **Why `common` carries field names as opaque `&'static str`, not a
//! `world::site::economy::Economy` reference.** Same boundary
//! `world_baseline.rs` already documents: `world` depends on `common`,
//! never the reverse. The classification is data, not a type the field
//! could be checked against structurally -- `world`'s own code is where a
//! future reader confirms a listed field still means what this table
//! says it means.
//!
//! **The manifest slot.** `T4.6`'s `SaveUniverseManifestV1.descriptors`
//! (`common/src/apex/save_universe.rs:383`) already reserves
//! `SubsystemSlotIdV1::Economy` for exactly this declaration ("a future
//! `T8.5` economy-remedy entry declares itself: push a descriptor at
//! `SubsystemSlotIdV1::Economy`, no new mechanism needed" -- that
//! module's own doc, written ahead of this row). [`t8_5_descriptor_v1`]
//! builds that descriptor; nothing here adds a second mechanism.

use crate::apex::digest::{ArtifactIdentityV1, ContentIdentityV1, hash_artifact_bytes_v1};
use crate::apex::scalar::SchemaVersion;
use crate::apex::subsystem::{SubsystemDescriptorV1, SubsystemSlotIdV1};

/// The ladder, `readme/apex/APEX-T8-TIER-SPEC-FLEET-v1.md` `T8.5`,
/// cheapest first. Explicit discriminants (never declaration order), same
/// discipline as every other frozen vocabulary in this program
/// ([`SubsystemSlotIdV1`], `DigestDomainIdV1`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EconomicNumericProtocolRungV1 {
    /// Certify determinism within one compiler/profile/target cell.
    /// Changes nothing. Sufficient exactly when no lane found a live
    /// order dependency, no lane found unbounded model sensitivity, and
    /// (if T8.2 has run) no cross-platform divergence.
    SameProfileCertification = 1,
    /// Fix a proven **transactional** order dependency (T8.3's first
    /// class: A-then-B genuinely differs) with a canonical traversal
    /// order. Changes generated worlds.
    CanonicalOrder = 2,
    /// Fix a proven **reduction-rounding** order dependency (T8.3's
    /// second class: float summation order) or a T8.2 cross-platform
    /// arithmetic divergence with a fixed accumulation order or
    /// phase-boundary quantisation step. Changes economic values.
    PhaseBoundaryQuantisation = 3,
    /// A T8.4 branch-crossing finding: some input difference flips a
    /// modelled decision, not just a magnitude. Needs a redesigned
    /// price-response kernel, not a numeric fix. Changes model behaviour.
    DeterministicPriceResponseKernel = 4,
    /// A T8.4 unbounded-sensitivity finding: the model is chaotic under
    /// floats and no canonical order or quantisation step converges it.
    /// Changes the save format.
    FixedDecimalStoredState = 5,
    /// Abandon regeneration; store the generated economy directly.
    /// Never evidence-triggered by this row's derivation -- a policy
    /// call the ladder still names, but which no measured sensitivity
    /// alone justifies choosing.
    PersistedGeneratedBaseline = 6,
}

impl EconomicNumericProtocolRungV1 {
    pub const fn as_u8(self) -> u8 { self as u8 }

    pub const ALL: [EconomicNumericProtocolRungV1; 6] = [
        Self::SameProfileCertification,
        Self::CanonicalOrder,
        Self::PhaseBoundaryQuantisation,
        Self::DeterministicPriceResponseKernel,
        Self::FixedDecimalStoredState,
        Self::PersistedGeneratedBaseline,
    ];
}

/// The three lanes' relevant findings, typed rather than prose. `T8.2`'s
/// field is `Option<bool>` -- deliberately, not defaulted to `false` --
/// because that lane has not run (Fable-deferred, "an environment
/// question I'll size separately"); `None` means "untested", never
/// "negative". Conflating the two is exactly the "not yet derived" vs
/// "fabricated" mistake this program's other frozen types already refuse
/// to make (`WorldBaselineInputV1::worldgen`, same discipline).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EconomicSensitivityEvidenceV1 {
    /// `T8.2`, not yet run. `Some(true)` forces at least
    /// [`EconomicNumericProtocolRungV1::PhaseBoundaryQuantisation`];
    /// `None` or `Some(false)` add no constraint -- rung 1's own
    /// definition is scoped to "one compiler/profile/target cell", so an
    /// untested cross-platform lane does not retroactively invalidate it.
    pub cross_platform_divergence_found: Option<bool>,
    /// `T8.3`: a permutation genuinely changed an outcome (stock
    /// consumed, customer served first), not just a rounding artifact.
    pub transactional_order_dependency_found: bool,
    /// `T8.3`: a permutation changed only a float-summation result.
    pub reduction_rounding_order_dependency_found: bool,
    /// `T8.4`: any swept field's sensitivity curve grew without bound
    /// over the swept horizon.
    pub model_sensitivity_unbounded: bool,
    /// `T8.4`: any swept field's perturbation flipped a modelled branch
    /// (the unstable-threshold inventory is non-empty).
    pub model_sensitivity_crosses_branch: bool,
}

/// Pure decision table, evidence to rung. Each `true`/`Some(true)`
/// ratchets the rung UP to at least the named floor; nothing here ever
/// lowers it, and nothing here ever reaches rung 6 -- see that variant's
/// own doc for why.
pub fn economic_numeric_protocol_rung_for_evidence_v1(evidence: EconomicSensitivityEvidenceV1) -> EconomicNumericProtocolRungV1 {
    use EconomicNumericProtocolRungV1::*;
    let mut rung = SameProfileCertification;
    if evidence.transactional_order_dependency_found {
        rung = rung.max(CanonicalOrder);
    }
    if evidence.reduction_rounding_order_dependency_found || evidence.cross_platform_divergence_found == Some(true) {
        rung = rung.max(PhaseBoundaryQuantisation);
    }
    if evidence.model_sensitivity_crosses_branch {
        rung = rung.max(DeterministicPriceResponseKernel);
    }
    if evidence.model_sensitivity_unbounded {
        rung = rung.max(FixedDecimalStoredState);
    }
    rung
}

/// One `Economy` (`world/src/site/economy/mod.rs`) field's classification
/// against the row's hard constraint: *re-derive only declared caches,
/// never path-dependent stocks, population, or history*.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EconomicStateClassV1 {
    /// Safe to re-derive: either an unconditional per-tick overwrite (no
    /// old value is ever read) or a bounded-memory smoothing term whose
    /// own perturbation-sensitivity evidence shows it decays to zero
    /// rather than accumulating.
    Cache,
    /// Never safe to re-derive: an accumulator whose current value
    /// depends on the full sequence of prior ticks, with no decay back
    /// to a fixed point.
    History,
}

#[derive(Clone, Copy, Debug)]
pub struct EconomicFieldClassificationV1 {
    pub field_name: &'static str,
    pub class: EconomicStateClassV1,
    /// The concrete evidence for this row's own future reader, not a
    /// repeat of the field's doc comment in `world`.
    pub evidence: &'static str,
}

const fn field(field_name: &'static str, class: EconomicStateClassV1, evidence: &'static str) -> EconomicFieldClassificationV1 {
    EconomicFieldClassificationV1 { field_name, class, evidence }
}

/// The declared boundary. `neighbors[].id` (site-adjacency topology, not
/// an economic magnitude) is deliberately absent -- it is `T4.3`'s
/// site-graph/world-baseline root's concern, not this row's; only
/// `neighbors[].last_values`/`last_supplies` (the remembered trade
/// snapshot) are in scope here.
pub const ECONOMIC_CACHE_HISTORY_BOUNDARY_V1: &[EconomicFieldClassificationV1] = &[
    field(
        "stocks",
        EconomicStateClassV1::History,
        "accumulator (+=/-= across ticks); T8.4 stock-sensitivity curve: a 1-ULP perturbation is still measurable for ~200 phases before decaying to 0.0 -- present, not instantaneous",
    ),
    field(
        "pop",
        EconomicStateClassV1::History,
        "T8.4 population-sensitivity curve: a perturbation persists at constant magnitude (~3.8e-6) across all 200 swept phases, never decaying -- the strongest path-dependence evidence of any field",
    ),
    field(
        "surplus",
        EconomicStateClassV1::Cache,
        "T8.3 structural-finding test: self.surplus = demand.map(...) is an unconditional per-tick overwrite in collect_deliveries; no old value is ever read, proven not merely assumed",
    ),
    field(
        "marginal_surplus",
        EconomicStateClassV1::Cache,
        "code read, context.rs tick(): self.marginal_surplus = demand.map(|g, demand| supply[g] - demand) is the same unconditional-overwrite shape as surplus, same file region -- not yet given its own dedicated falsifier test",
    ),
    field(
        "labor_values",
        EconomicStateClassV1::Cache,
        "code read, mod.rs tick(): self.labor_values = total_labor_values.map(...) wholesale-replaces the map each tick from a within-tick-fresh local, never blending in the prior self.labor_values",
    ),
    field(
        "values",
        EconomicStateClassV1::Cache,
        "T8.4 price-sensitivity curve (Good::Food, +1e-3 quantisation unit, warmup=2): perturbation decays to exactly 0.0 by phase 200 -- bounded-memory smoothing (smooth=0.8), not an accumulator",
    ),
    field(
        "labors",
        EconomicStateClassV1::Cache,
        "T8.4 demand-sensitivity curve (labors[Banker], +1e-3): perturbation decays to exactly 0.0 by phase ~200 -- same bounded-memory smoothing shape as values",
    ),
    field(
        "orders",
        EconomicStateClassV1::Cache,
        "per-tick transient message buffer, drained every phase (context.rs tick(), orders.drain()); Economy carries no Serialize/Deserialize derive today, so this never crosses a save/regen boundary to begin with",
    ),
    field(
        "deliveries",
        EconomicStateClassV1::Cache,
        "per-tick transient message buffer, drained every phase (context.rs tick(), trade.deliveries.drain()); same never-persisted status as orders",
    ),
    field(
        "neighbors[].last_values / last_supplies",
        EconomicStateClassV1::Cache,
        "T8.3 delivery-collection test: std::mem::swap(&mut n.last_values, &mut d.prices) is a last-observed-snapshot overwrite, proven dead-code-today under current reachability -- not an accumulator even when live",
    ),
];

/// Everything this row's decision needs to be replayable: the evidence it
/// was derived from, the derived rung, and a stated escalation contract
/// so "what would change this" is data, not a promise.
#[derive(Clone, Debug)]
pub struct EconomicNumericProtocolDecisionV1 {
    pub evidence: EconomicSensitivityEvidenceV1,
    pub rung: EconomicNumericProtocolRungV1,
}

pub fn economic_numeric_protocol_decision_v1(evidence: EconomicSensitivityEvidenceV1) -> EconomicNumericProtocolDecisionV1 {
    EconomicNumericProtocolDecisionV1 { evidence, rung: economic_numeric_protocol_rung_for_evidence_v1(evidence) }
}

/// The evidence this row was actually closed with: T8.1's per-phase
/// hashing infrastructure exists and is fixture-tested; T8.3 swept site
/// order (null, 2000-phase experiment), scarce-allocation order (order-
/// independent, both symmetric and asymmetric), and delivery collection
/// (last-writer path proven dead-code-today) with no transactional or
/// reduction-rounding finding surviving trace; T8.4 swept all six named
/// fields (price, stock/demand, surplus, population, smoothing) and found
/// every curve bounded, none unbounded, none crossing a branch. T8.2 has
/// not run.
pub fn t8_5_current_evidence_v1() -> EconomicSensitivityEvidenceV1 {
    EconomicSensitivityEvidenceV1 {
        cross_platform_divergence_found: None,
        transactional_order_dependency_found: false,
        reduction_rounding_order_dependency_found: false,
        model_sensitivity_unbounded: false,
        model_sensitivity_crosses_branch: false,
    }
}

/// The row's own ruled decision, from [`t8_5_current_evidence_v1`]:
/// [`EconomicNumericProtocolRungV1::SameProfileCertification`], no
/// remedy needed. Full credit close per the row's own stated bar: "if
/// the honest verdict is 'rung 1, no remedy needed, here is the evidence
/// and here is what would change it,' that is a full-credit close."
pub fn t8_5_current_decision_v1() -> EconomicNumericProtocolDecisionV1 { economic_numeric_protocol_decision_v1(t8_5_current_evidence_v1()) }

fn push_u8(buf: &mut Vec<u8>, v: u8) { buf.push(v); }
fn push_option_bool(buf: &mut Vec<u8>, v: Option<bool>) {
    match v {
        None => buf.push(0),
        Some(false) => buf.push(1),
        Some(true) => buf.push(2),
    }
}
fn push_bool(buf: &mut Vec<u8>, v: bool) { buf.push(if v { 1 } else { 0 }); }

/// Canonical, fixed-width preimage -- same discipline as
/// `world_baseline_preimage_v1`: every field fixed-width so no two
/// distinct decisions can collide.
fn economic_numeric_protocol_decision_preimage_v1(decision: &EconomicNumericProtocolDecisionV1) -> Vec<u8> {
    let mut buf = Vec::new();
    push_u8(&mut buf, decision.rung.as_u8());
    push_option_bool(&mut buf, decision.evidence.cross_platform_divergence_found);
    push_bool(&mut buf, decision.evidence.transactional_order_dependency_found);
    push_bool(&mut buf, decision.evidence.reduction_rounding_order_dependency_found);
    push_bool(&mut buf, decision.evidence.model_sensitivity_unbounded);
    push_bool(&mut buf, decision.evidence.model_sensitivity_crosses_branch);
    buf
}

pub fn economic_numeric_protocol_decision_identity_v1(decision: &EconomicNumericProtocolDecisionV1) -> ArtifactIdentityV1 {
    hash_artifact_bytes_v1(&economic_numeric_protocol_decision_preimage_v1(decision))
}

/// `T4.6`'s manifest slot, ridden not reinvented: a `SubsystemDescriptorV1`
/// at `SubsystemSlotIdV1::Economy`, ready to push into a
/// `SaveUniverseManifestV1.descriptors` array.
pub fn t8_5_descriptor_v1(decision: &EconomicNumericProtocolDecisionV1) -> SubsystemDescriptorV1 {
    SubsystemDescriptorV1 {
        slot: SubsystemSlotIdV1::Economy,
        schema: SchemaVersion::new(1),
        content: ContentIdentityV1 { artifact: economic_numeric_protocol_decision_identity_v1(decision), semantic: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_findings() -> EconomicSensitivityEvidenceV1 {
        EconomicSensitivityEvidenceV1 {
            cross_platform_divergence_found: None,
            transactional_order_dependency_found: false,
            reduction_rounding_order_dependency_found: false,
            model_sensitivity_unbounded: false,
            model_sensitivity_crosses_branch: false,
        }
    }

    #[test]
    fn no_findings_at_all_selects_rung_1() {
        assert_eq!(
            economic_numeric_protocol_rung_for_evidence_v1(no_findings()),
            EconomicNumericProtocolRungV1::SameProfileCertification
        );
    }

    #[test]
    fn an_untested_cross_platform_lane_does_not_force_a_higher_rung() {
        let evidence = EconomicSensitivityEvidenceV1 { cross_platform_divergence_found: None, ..no_findings() };
        assert_eq!(
            economic_numeric_protocol_rung_for_evidence_v1(evidence),
            EconomicNumericProtocolRungV1::SameProfileCertification,
            "rung 1 is scoped to one profile; an UNRUN cross-platform lane must not be treated as a negative finding"
        );
    }

    #[test]
    fn a_positive_cross_platform_finding_forces_at_least_quantisation() {
        let evidence = EconomicSensitivityEvidenceV1 { cross_platform_divergence_found: Some(true), ..no_findings() };
        assert_eq!(
            economic_numeric_protocol_rung_for_evidence_v1(evidence),
            EconomicNumericProtocolRungV1::PhaseBoundaryQuantisation
        );
    }

    #[test]
    fn a_negative_cross_platform_finding_does_not_force_a_higher_rung() {
        let evidence = EconomicSensitivityEvidenceV1 { cross_platform_divergence_found: Some(false), ..no_findings() };
        assert_eq!(economic_numeric_protocol_rung_for_evidence_v1(evidence), EconomicNumericProtocolRungV1::SameProfileCertification);
    }

    #[test]
    fn a_transactional_order_finding_forces_canonical_order() {
        let evidence = EconomicSensitivityEvidenceV1 { transactional_order_dependency_found: true, ..no_findings() };
        assert_eq!(economic_numeric_protocol_rung_for_evidence_v1(evidence), EconomicNumericProtocolRungV1::CanonicalOrder);
    }

    #[test]
    fn a_reduction_rounding_finding_forces_quantisation() {
        let evidence = EconomicSensitivityEvidenceV1 { reduction_rounding_order_dependency_found: true, ..no_findings() };
        assert_eq!(economic_numeric_protocol_rung_for_evidence_v1(evidence), EconomicNumericProtocolRungV1::PhaseBoundaryQuantisation);
    }

    #[test]
    fn a_branch_crossing_finding_forces_the_kernel_rung() {
        let evidence = EconomicSensitivityEvidenceV1 { model_sensitivity_crosses_branch: true, ..no_findings() };
        assert_eq!(
            economic_numeric_protocol_rung_for_evidence_v1(evidence),
            EconomicNumericProtocolRungV1::DeterministicPriceResponseKernel
        );
    }

    #[test]
    fn an_unbounded_sensitivity_finding_forces_fixed_decimal_state() {
        let evidence = EconomicSensitivityEvidenceV1 { model_sensitivity_unbounded: true, ..no_findings() };
        assert_eq!(economic_numeric_protocol_rung_for_evidence_v1(evidence), EconomicNumericProtocolRungV1::FixedDecimalStoredState);
    }

    /// Rung 6 is never evidence-triggered -- it is the ladder's own named
    /// policy-only option, not a derivation outcome. Even the worst
    /// simultaneous finding across every axis must not reach it.
    #[test]
    fn rung_6_is_never_selected_by_the_evidence_derivation() {
        let worst = EconomicSensitivityEvidenceV1 {
            cross_platform_divergence_found: Some(true),
            transactional_order_dependency_found: true,
            reduction_rounding_order_dependency_found: true,
            model_sensitivity_unbounded: true,
            model_sensitivity_crosses_branch: true,
        };
        assert_ne!(economic_numeric_protocol_rung_for_evidence_v1(worst), EconomicNumericProtocolRungV1::PersistedGeneratedBaseline);
        assert_eq!(economic_numeric_protocol_rung_for_evidence_v1(worst), EconomicNumericProtocolRungV1::FixedDecimalStoredState);
    }

    #[test]
    fn multiple_findings_take_the_highest_forced_rung_not_the_last_checked() {
        // model_sensitivity_unbounded (rung 5) is checked after
        // transactional_order_dependency_found (rung 2) in the function
        // body -- this proves the result is a max, not "whichever branch
        // runs last wins".
        let evidence = EconomicSensitivityEvidenceV1 { transactional_order_dependency_found: true, model_sensitivity_unbounded: true, ..no_findings() };
        assert_eq!(economic_numeric_protocol_rung_for_evidence_v1(evidence), EconomicNumericProtocolRungV1::FixedDecimalStoredState);
    }

    #[test]
    fn the_current_decision_is_rung_1_with_no_remedy_needed() {
        let decision = t8_5_current_decision_v1();
        assert_eq!(decision.rung, EconomicNumericProtocolRungV1::SameProfileCertification);
        assert_eq!(decision.evidence.cross_platform_divergence_found, None, "T8.2 has not run; the decision must say so, not assume a negative");
    }

    #[test]
    fn rung_tags_are_frozen_and_unique() {
        use std::collections::HashSet;
        let tags: HashSet<u8> = EconomicNumericProtocolRungV1::ALL.iter().map(|r| r.as_u8()).collect();
        assert_eq!(tags.len(), EconomicNumericProtocolRungV1::ALL.len());
        assert_eq!(EconomicNumericProtocolRungV1::SameProfileCertification.as_u8(), 1);
        assert_eq!(EconomicNumericProtocolRungV1::CanonicalOrder.as_u8(), 2);
        assert_eq!(EconomicNumericProtocolRungV1::PhaseBoundaryQuantisation.as_u8(), 3);
        assert_eq!(EconomicNumericProtocolRungV1::DeterministicPriceResponseKernel.as_u8(), 4);
        assert_eq!(EconomicNumericProtocolRungV1::FixedDecimalStoredState.as_u8(), 5);
        assert_eq!(EconomicNumericProtocolRungV1::PersistedGeneratedBaseline.as_u8(), 6);
    }

    #[test]
    fn the_same_decision_hashes_identically_across_runs() {
        let a = economic_numeric_protocol_decision_identity_v1(&t8_5_current_decision_v1());
        let b = economic_numeric_protocol_decision_identity_v1(&t8_5_current_decision_v1());
        assert_eq!(a, b);
    }

    #[test]
    fn a_different_rung_moves_the_decision_identity() {
        let rung1 = economic_numeric_protocol_decision_identity_v1(&t8_5_current_decision_v1());
        let rung2 = economic_numeric_protocol_decision_identity_v1(&EconomicNumericProtocolDecisionV1 {
            evidence: EconomicSensitivityEvidenceV1 { transactional_order_dependency_found: true, ..t8_5_current_evidence_v1() },
            rung: EconomicNumericProtocolRungV1::CanonicalOrder,
        });
        assert_ne!(rung1, rung2);
    }

    #[test]
    fn an_untested_and_a_negative_cross_platform_lane_produce_different_decision_identities() {
        // The distinction is real, not decorative: a future decision built
        // from a genuinely negative T8.2 result must not be
        // indistinguishable from today's pending-lane decision.
        let untested = economic_numeric_protocol_decision_identity_v1(&t8_5_current_decision_v1());
        let negative = economic_numeric_protocol_decision_identity_v1(&economic_numeric_protocol_decision_v1(EconomicSensitivityEvidenceV1 {
            cross_platform_divergence_found: Some(false),
            ..t8_5_current_evidence_v1()
        }));
        assert_ne!(untested, negative);
    }

    #[test]
    fn the_descriptor_rides_the_economy_slot_t46_already_reserved() {
        let descriptor = t8_5_descriptor_v1(&t8_5_current_decision_v1());
        assert_eq!(descriptor.slot, SubsystemSlotIdV1::Economy);
    }

    #[test]
    fn the_descriptor_moves_when_the_decision_moves() {
        let a = t8_5_descriptor_v1(&t8_5_current_decision_v1());
        let escalated = economic_numeric_protocol_decision_v1(EconomicSensitivityEvidenceV1 {
            model_sensitivity_unbounded: true,
            ..t8_5_current_evidence_v1()
        });
        let b = t8_5_descriptor_v1(&escalated);
        assert_ne!(a.content.artifact, b.content.artifact);
    }

    #[test]
    fn every_boundary_entry_has_nonempty_evidence() {
        for entry in ECONOMIC_CACHE_HISTORY_BOUNDARY_V1 {
            assert!(!entry.field_name.is_empty());
            assert!(!entry.evidence.is_empty(), "{} must cite concrete evidence, not an unsupported classification", entry.field_name);
        }
    }

    #[test]
    fn the_two_path_dependent_history_fields_are_exactly_stocks_and_pop() {
        let history_fields: Vec<&str> = ECONOMIC_CACHE_HISTORY_BOUNDARY_V1
            .iter()
            .filter(|e| matches!(e.class, EconomicStateClassV1::History))
            .map(|e| e.field_name)
            .collect();
        assert_eq!(history_fields, vec!["stocks", "pop"], "if this changes, a rung >= 2 decision must re-examine what it would be safe to re-derive");
    }

    #[test]
    fn boundary_field_names_are_unique() {
        use std::collections::HashSet;
        let names: HashSet<&str> = ECONOMIC_CACHE_HISTORY_BOUNDARY_V1.iter().map(|e| e.field_name).collect();
        assert_eq!(names.len(), ECONOMIC_CACHE_HISTORY_BOUNDARY_V1.len(), "duplicate field classification");
    }
}
