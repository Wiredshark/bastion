//! `CompatibilityReportV1` and the evaluator (`APEX-T0.5`, spec section
//! 3.7). Walks a profile against supplied descriptors/catalogs/registry
//! and produces one typed result per rule, in slot-tag order, without
//! short-circuiting.

use crate::apex::manifest::VariantTagV1;

use super::descriptor::SubsystemDescriptorV1;
use super::negotiate::{CapabilityIdV1, NegotiationSelectorV1};
use super::profile::CompatibilityProfileV1;
use super::rule::{CompatibilityRuleV1, ExtensionCriticalityV1};
use super::slot::SubsystemSlotIdV1;
use super::transform::TransformRegistryV1;

/// Why a rule evaluated `Incompatible` -- well-formed input, simply not
/// satisfied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IncompatibilityReasonV1 {
    ContentMismatch,
    SchemaNotInAcceptSet,
    SchemaOutOfAcceptRange,
    CapabilityMissing,
    TransformNotAuthorized,
    UnknownCriticalExtension,
}

/// Why a rule evaluated `InvalidInput` -- structurally well-formed and
/// successfully decoded, but the evaluator cannot resolve it in context
/// (spec section 3.7 clarification: never a decode-time malformation,
/// which is a hard decode failure before evaluation ever runs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvalidInputReasonV1 {
    NoDescriptorForSlot,
    SelectorDisagreement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompatibilityOutcomeV1 {
    Compatible,
    Incompatible(IncompatibilityReasonV1),
    InvalidInput(InvalidInputReasonV1),
}

/// Evidence retained for an `Unknown` rule regardless of outcome --
/// populated whenever the evaluated rule was `Unknown`, never inferred
/// from an absent field (spec section 3.7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownExtensionEvidenceV1 {
    pub tag: VariantTagV1,
    pub criticality: ExtensionCriticalityV1,
    pub raw_payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityResultV1 {
    pub slot: SubsystemSlotIdV1,
    pub outcome: CompatibilityOutcomeV1,
    pub unknown_extension: Option<UnknownExtensionEvidenceV1>,
}

/// One entry per profile rule, sorted by slot tag (inherited from
/// [`CompatibilityProfileV1`]'s own canonical order), never
/// short-circuited -- every rule is evaluated even after an earlier one
/// is `Incompatible`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityReportV1(Vec<CompatibilityResultV1>);

impl CompatibilityReportV1 {
    pub fn results(&self) -> &[CompatibilityResultV1] { &self.0 }

    pub fn is_fully_compatible(&self) -> bool {
        self.0.iter().all(|r| matches!(r.outcome, CompatibilityOutcomeV1::Compatible))
    }
}

/// Everything the evaluator needs beyond the profile itself: what the
/// peer/environment actually declared. Not part of the frozen wire
/// contract (spec section 5) -- an in-process evaluation input, not a
/// manifest-encoded type.
#[derive(Clone, Debug, Default)]
pub struct SubsystemEvaluationInputV1 {
    /// One descriptor per slot the peer/environment actually declares;
    /// sparse by construction -- a slot with no matching descriptor is
    /// exactly the `InvalidInputReasonV1::NoDescriptorForSlot` case.
    pub descriptors: Vec<SubsystemDescriptorV1>,
    pub local_selector: Option<NegotiationSelectorV1>,
    pub peer_selector: Option<NegotiationSelectorV1>,
    pub peer_capabilities: Vec<CapabilityIdV1>,
    pub transform_registry: TransformRegistryV1,
}

impl SubsystemEvaluationInputV1 {
    fn descriptor_for(&self, slot: SubsystemSlotIdV1) -> Option<&SubsystemDescriptorV1> {
        self.descriptors.iter().find(|d| d.slot == slot)
    }
}

/// Walks `profile` against `input` and produces a full report. Never
/// stops at the first mismatch (build step 7).
pub fn evaluate_compatibility_v1(profile: &CompatibilityProfileV1, input: &SubsystemEvaluationInputV1) -> CompatibilityReportV1 {
    let mut results = Vec::with_capacity(profile.entries().len());
    for (slot, rule) in profile.entries() {
        let (outcome, unknown_extension) = evaluate_one_rule(*slot, rule, input);
        results.push(CompatibilityResultV1 { slot: *slot, outcome, unknown_extension });
    }
    CompatibilityReportV1(results)
}

fn evaluate_one_rule(
    slot: SubsystemSlotIdV1,
    rule: &CompatibilityRuleV1,
    input: &SubsystemEvaluationInputV1,
) -> (CompatibilityOutcomeV1, Option<UnknownExtensionEvidenceV1>) {
    match rule {
        CompatibilityRuleV1::Exact { content } => match input.descriptor_for(slot) {
            Some(d) if d.content == *content => (CompatibilityOutcomeV1::Compatible, None),
            Some(_) => (CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::ContentMismatch), None),
            None => (CompatibilityOutcomeV1::InvalidInput(InvalidInputReasonV1::NoDescriptorForSlot), None),
        },
        CompatibilityRuleV1::AcceptSet(set) => match input.descriptor_for(slot) {
            Some(d) if set.schemas().contains(&d.schema) => (CompatibilityOutcomeV1::Compatible, None),
            Some(_) => (CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::SchemaNotInAcceptSet), None),
            None => (CompatibilityOutcomeV1::InvalidInput(InvalidInputReasonV1::NoDescriptorForSlot), None),
        },
        CompatibilityRuleV1::AcceptRange(range) => match input.descriptor_for(slot) {
            Some(d) if range.contains(d.schema) => (CompatibilityOutcomeV1::Compatible, None),
            Some(_) => (CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::SchemaOutOfAcceptRange), None),
            None => (CompatibilityOutcomeV1::InvalidInput(InvalidInputReasonV1::NoDescriptorForSlot), None),
        },
        CompatibilityRuleV1::NegotiatedCapability { requirement } => {
            match (input.local_selector, input.peer_selector) {
                (Some(local), Some(peer)) if local == peer => {
                    let satisfied = requirement.catalog().iter().all(|c| input.peer_capabilities.contains(c));
                    if satisfied {
                        (CompatibilityOutcomeV1::Compatible, None)
                    } else {
                        (CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::CapabilityMissing), None)
                    }
                },
                _ => (CompatibilityOutcomeV1::InvalidInput(InvalidInputReasonV1::SelectorDisagreement), None),
            }
        },
        CompatibilityRuleV1::DirectTransform { key } => {
            if input.transform_registry.contains(key) {
                (CompatibilityOutcomeV1::Compatible, None)
            } else {
                (CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::TransformNotAuthorized), None)
            }
        },
        CompatibilityRuleV1::ProvenanceOnly => (CompatibilityOutcomeV1::Compatible, None),
        CompatibilityRuleV1::Unknown { tag, criticality, raw_payload } => {
            let evidence = UnknownExtensionEvidenceV1 { tag: *tag, criticality: *criticality, raw_payload: raw_payload.clone() };
            match criticality {
                ExtensionCriticalityV1::Critical => {
                    (CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::UnknownCriticalExtension), Some(evidence))
                },
                ExtensionCriticalityV1::Noncritical => (CompatibilityOutcomeV1::Compatible, Some(evidence)),
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::{ContentIdentityV1, hash_artifact_bytes_v1};
    use crate::apex::scalar::SchemaVersion;
    use crate::apex::subsystem::negotiate::{SelectorAlgorithmV1, SelectorOwnerV1};
    use crate::apex::subsystem::rule::AcceptRangeV1;
    use crate::apex::subsystem::transform::{TransformIdV1, TransformKeyV1};

    fn descriptor(slot: SubsystemSlotIdV1, seed: &[u8]) -> SubsystemDescriptorV1 {
        SubsystemDescriptorV1 {
            slot,
            schema: SchemaVersion::new(1),
            content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(seed), semantic: None },
        }
    }

    #[test]
    fn never_short_circuits_evaluates_every_rule_even_after_a_mismatch() {
        let d = descriptor(SubsystemSlotIdV1::Worldgen, b"actual");
        let wrong = CompatibilityRuleV1::Exact { content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"wrong"), semantic: None } };
        let ok = CompatibilityRuleV1::ProvenanceOnly;
        let profile = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Worldgen, wrong), (SubsystemSlotIdV1::Content, ok)]).unwrap();
        let input = SubsystemEvaluationInputV1 { descriptors: vec![d], ..Default::default() };
        let report = evaluate_compatibility_v1(&profile, &input);
        assert_eq!(report.results().len(), 2, "both rules must be evaluated, not stopped at the first mismatch");
        assert!(matches!(report.results()[0].outcome, CompatibilityOutcomeV1::Incompatible(_)));
        assert!(matches!(report.results()[1].outcome, CompatibilityOutcomeV1::Compatible));
    }

    #[test]
    fn wrong_artifact_bytes_is_incompatible_never_invalid_input() {
        let d = descriptor(SubsystemSlotIdV1::Worldgen, b"actual");
        let rule = CompatibilityRuleV1::Exact { content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"other"), semantic: None } };
        let profile = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Worldgen, rule)]).unwrap();
        let input = SubsystemEvaluationInputV1 { descriptors: vec![d], ..Default::default() };
        let report = evaluate_compatibility_v1(&profile, &input);
        assert_eq!(report.results()[0].outcome, CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::ContentMismatch));
    }

    #[test]
    fn missing_descriptor_is_invalid_input() {
        let rule = CompatibilityRuleV1::Exact { content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"x"), semantic: None } };
        let profile = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Worldgen, rule)]).unwrap();
        let input = SubsystemEvaluationInputV1::default();
        let report = evaluate_compatibility_v1(&profile, &input);
        assert_eq!(report.results()[0].outcome, CompatibilityOutcomeV1::InvalidInput(InvalidInputReasonV1::NoDescriptorForSlot));
    }

    #[test]
    fn selector_drift_is_invalid_input_not_a_silent_fallback() {
        let requirement = crate::apex::subsystem::negotiate::CapabilityRequirementV1::new(vec![CapabilityIdV1::new(1)]).unwrap();
        let rule = CompatibilityRuleV1::NegotiatedCapability { requirement };
        let profile = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Numeric, rule)]).unwrap();
        let input = SubsystemEvaluationInputV1 {
            local_selector: Some(NegotiationSelectorV1 {
                owner: SelectorOwnerV1::ServerAuthoritative,
                algorithm: SelectorAlgorithmV1::HighestMutualVersion,
                version: SchemaVersion::new(1),
            }),
            peer_selector: Some(NegotiationSelectorV1 {
                owner: SelectorOwnerV1::ServerAuthoritative,
                algorithm: SelectorAlgorithmV1::ExactMatchOnly, // disagreement
                version: SchemaVersion::new(1),
            }),
            peer_capabilities: vec![CapabilityIdV1::new(1)],
            ..Default::default()
        };
        let report = evaluate_compatibility_v1(&profile, &input);
        assert_eq!(report.results()[0].outcome, CompatibilityOutcomeV1::InvalidInput(InvalidInputReasonV1::SelectorDisagreement));
    }

    #[test]
    fn unauthorized_transform_is_incompatible_not_panic() {
        let key = TransformKeyV1 {
            transform_id: TransformIdV1::new(1),
            from_schema: SchemaVersion::new(1),
            to_schema: SchemaVersion::new(2),
            implementation_root: hash_artifact_bytes_v1(b"impl"),
        };
        let rule = CompatibilityRuleV1::DirectTransform { key };
        let profile = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Numeric, rule)]).unwrap();
        let input = SubsystemEvaluationInputV1::default(); // empty registry -- key absent
        let report = evaluate_compatibility_v1(&profile, &input);
        assert_eq!(report.results()[0].outcome, CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::TransformNotAuthorized));
    }

    #[test]
    fn critical_unknown_is_incompatible_noncritical_unknown_is_compatible_with_evidence_retained() {
        let critical = CompatibilityRuleV1::Unknown {
            tag: VariantTagV1::new(9001),
            criticality: ExtensionCriticalityV1::Critical,
            raw_payload: vec![0xaa],
        };
        let noncritical = CompatibilityRuleV1::Unknown {
            tag: VariantTagV1::new(9002),
            criticality: ExtensionCriticalityV1::Noncritical,
            raw_payload: vec![0xbb],
        };
        let profile = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Worldgen, critical), (SubsystemSlotIdV1::Content, noncritical)]).unwrap();
        let report = evaluate_compatibility_v1(&profile, &SubsystemEvaluationInputV1::default());

        assert_eq!(report.results()[0].outcome, CompatibilityOutcomeV1::Incompatible(IncompatibilityReasonV1::UnknownCriticalExtension));
        assert_eq!(report.results()[0].unknown_extension.as_ref().unwrap().raw_payload, vec![0xaa]);

        assert_eq!(report.results()[1].outcome, CompatibilityOutcomeV1::Compatible);
        let evidence = report.results()[1].unknown_extension.as_ref().expect("evidence must be retained, not dropped");
        assert_eq!(evidence.raw_payload, vec![0xbb]);
        assert_eq!(evidence.tag.get(), 9002);
    }

    #[test]
    fn accept_range_boundary_is_evaluated_correctly() {
        let mut d = descriptor(SubsystemSlotIdV1::Numeric, b"x");
        d.schema = SchemaVersion::new(5);
        let range = AcceptRangeV1::new(SchemaVersion::new(1), SchemaVersion::new(5)).unwrap();
        let rule = CompatibilityRuleV1::AcceptRange(range);
        let profile = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Numeric, rule)]).unwrap();
        let input = SubsystemEvaluationInputV1 { descriptors: vec![d], ..Default::default() };
        let report = evaluate_compatibility_v1(&profile, &input);
        assert_eq!(report.results()[0].outcome, CompatibilityOutcomeV1::Compatible, "max bound is inclusive");
    }
}
