//! Shared subsystem descriptors and compatibility profiles (`APEX-T0.5`).
//!
//! Fleet-authored per Ben's 2026-07-27 order (original packet/vectors
//! hallucination-class); spec at
//! `readme/apex/APEX-T0.5-FLEET-AUTHORED-SPEC-v1.md`, Opus 5 spec-reviewed,
//! Fable build-authorized. See that document for the full determinism
//! story, data-model rationale, and non-goals.

pub mod descriptor;
pub mod negotiate;
pub mod profile;
pub mod report;
pub mod rule;
pub mod slot;
pub mod transform;

pub use descriptor::{ContentProtocolVersion, NumericProtocolVersion, SubsystemDescriptorV1, WorldgenProtocolVersion};
pub use negotiate::{
    CapabilityIdV1, CapabilityRequirementErrorV1, CapabilityRequirementV1, NegotiationSelectorV1, SelectorAlgorithmV1,
    SelectorOwnerV1,
};
pub use profile::{CompatibilityProfileErrorV1, CompatibilityProfileV1, MAX_PROFILE_ENTRIES};
pub use report::{
    CompatibilityOutcomeV1, CompatibilityReportV1, CompatibilityResultV1, IncompatibilityReasonV1, InvalidInputReasonV1,
    SubsystemEvaluationInputV1, UnknownExtensionEvidenceV1, evaluate_compatibility_v1,
};
pub use rule::{
    AcceptRangeV1, AcceptSetV1, CompatibilityRuleConstructionErrorV1, CompatibilityRuleV1, ExtensionCriticalityV1,
};
pub use slot::SubsystemSlotIdV1;
pub use transform::{TransformIdV1, TransformKeyV1, TransformRegistrationErrorV1, TransformRegistryV1};
