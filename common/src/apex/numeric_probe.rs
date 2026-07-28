//! `APEX-T6.2` — raw and semantic numeric probes.
//!
//! Hidden raw divergence must be visible even when gameplay tolerance
//! masks it.
//!
//! Structurally the twin of `T5.3`, and it SHARES that row's types
//! rather than re-deriving them: [`super::probe::ExactProbeV1`] is the
//! raw probe and [`super::probe::QuantizedProbeV1`] is the semantic one,
//! with the same prohibition — a semantic match can never certify exact
//! execution, and no conversion exists in either direction.
//!
//! What this module adds is the part `T5.3` does not need: **how the
//! bytes going into those probes are produced**.
//!
//! **`to_bits()`, never `==`.** Float equality hides exactly the
//! differences a raw probe exists to find: `-0.0 == 0.0` is true and
//! their bit patterns differ, and every NaN compares unequal to itself
//! regardless of payload. [`RawSampleV1`] therefore stores bits, and it
//! is constructed from an `f32`/`f64` rather than from a `u32`, so a
//! caller cannot hand it a number it did not derive from a float.
//!
//! **A quantisation policy is four decisions, not a scale factor.**
//! Scale, rounding mode, saturation and an explicit non-finite policy —
//! [`QuantizationSpecV1`] names all four, because the ones that get
//! forgotten (what does a NaN quantise to? what happens at the
//! saturation edge?) are the ones that make two runs disagree for
//! reasons that have nothing to do with the simulation. The spec's
//! version is what `T5.3`'s `QuantizationPolicyV1` carries, so a policy
//! edit is visible in every probe taken under it.
//!
//! **A probe is taken at a PHASE, and probes from different phases are
//! not comparable.** A probe taken after physics and one taken after
//! character behaviour describe different worlds; reporting their
//! difference as a divergence is a false positive that costs a day.
//! [`ProbePhaseV1`] is part of the sample set's identity.
//!
//! **The probe observes and does not participate.** Nothing here returns
//! a quantised VALUE — only digests and a first-difference report. There
//! is no accessor that hands a caller a semantic number it could feed
//! back into simulation, because that is how a probe becomes the thing it
//! measures.

use super::probe::{
    ExactProbeV1, ProbeComparisonV1, QuantizationPolicyV1, QuantizedProbeV1, compare_probes_v1,
};

/// The tick phase a probe was taken at.
///
/// Part of a sample set's identity: comparing across phases is a false
/// divergence, and this type is what makes it impossible rather than
/// discouraged.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ProbePhaseV1 {
    /// Before any system has run this tick.
    TickStart,
    /// After character behaviour, before physics.
    AfterCharacterBehaviour,
    /// After the physics tick.
    AfterPhysics,
    /// After every system, before sync.
    TickEnd,
}

impl ProbePhaseV1 {
    pub const ALL: [Self; 4] =
        [Self::TickStart, Self::AfterCharacterBehaviour, Self::AfterPhysics, Self::TickEnd];

    pub const fn label(self) -> &'static str {
        match self {
            Self::TickStart => "tick-start",
            Self::AfterCharacterBehaviour => "after-character-behaviour",
            Self::AfterPhysics => "after-physics",
            Self::TickEnd => "tick-end",
        }
    }
}

/// One sampled float, stored as bits.
///
/// Constructed from a float, never from a raw word: a caller cannot hand
/// this a `u32` it computed some other way and have it counted as a
/// sample.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RawSampleV1(u64);

impl RawSampleV1 {
    pub fn of_f32_v1(value: f32) -> Self { Self(u64::from(value.to_bits())) }

    pub fn of_f64_v1(value: f64) -> Self { Self(value.to_bits()) }

    pub const fn bits_v1(self) -> u64 { self.0 }
}

/// How the tolerance quantiser behaves at each of the four decisions
/// that actually cause disagreement.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct QuantizationSpecV1 {
    /// The policy version. Folded into every probe taken under this
    /// spec, so a policy edit is visible rather than silent.
    pub policy: QuantizationPolicyV1,
    /// Multiplier applied before rounding. `1000` = three decimals.
    pub scale: i64,
    pub rounding: RoundingModeV1,
    /// Values whose scaled magnitude exceeds this saturate to it.
    pub saturation: i64,
    pub non_finite: NonFinitePolicyV1,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RoundingModeV1 {
    /// Ties away from zero. Named because `f32::round` does this and
    /// "round" alone does not say which of six behaviours is meant.
    NearestTiesAwayFromZero,
    TowardZero,
}

/// What a non-finite value quantises to.
///
/// There is no `Passthrough` variant. A NaN that reaches the quantiser
/// unchanged compares unequal to itself, which makes a semantic probe
/// non-reflexive — the probe would report a divergence between a run and
/// itself.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NonFinitePolicyV1 {
    /// NaN and both infinities map to distinct reserved sentinels, so
    /// "this field went NaN" survives quantisation as a fact rather than
    /// being smeared into a number.
    ReservedSentinels,
}

/// The reserved sentinels. Chosen at the extremes of `i64` so no scaled
/// finite value can collide with them; `saturation` is validated against
/// that below.
const SENTINEL_NAN: i64 = i64::MAX;
const SENTINEL_POS_INF: i64 = i64::MAX - 1;
const SENTINEL_NEG_INF: i64 = i64::MIN;

/// Why a spec was rejected.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum QuantizationSpecErrorV1 {
    /// A scale of zero or below collapses every value to one bucket, and
    /// a semantic probe that always matches is worse than no probe: it
    /// looks like evidence.
    NonPositiveScale(i64),
    /// The saturation bound reaches the reserved sentinels, so a finite
    /// value could quantise to "NaN".
    SaturationCollidesWithSentinels(i64),
}

impl QuantizationSpecV1 {
    /// Validate, rather than trusting. Both rejections describe a spec
    /// that would produce confidently wrong answers instead of failing.
    pub const fn validated_v1(self) -> Result<Self, QuantizationSpecErrorV1> {
        if self.scale <= 0 {
            return Err(QuantizationSpecErrorV1::NonPositiveScale(self.scale));
        }
        if self.saturation >= SENTINEL_POS_INF || self.saturation <= SENTINEL_NEG_INF {
            return Err(QuantizationSpecErrorV1::SaturationCollidesWithSentinels(self.saturation));
        }
        Ok(self)
    }

    /// Quantise one sample. Private on purpose — see the module doc's
    /// "observes and does not participate": no caller gets a semantic
    /// VALUE out of this module, only digests.
    fn quantise(self, value: f64) -> i64 {
        if value.is_nan() {
            return SENTINEL_NAN;
        }
        if value.is_infinite() {
            return if value.is_sign_positive() { SENTINEL_POS_INF } else { SENTINEL_NEG_INF };
        }
        let scaled = value * self.scale as f64;
        let rounded = match self.rounding {
            RoundingModeV1::NearestTiesAwayFromZero => scaled.round(),
            RoundingModeV1::TowardZero => scaled.trunc(),
        };
        let bound = self.saturation as f64;
        (rounded.clamp(-bound, bound)) as i64
    }
}

/// A named field's raw sample. The name is what a first-difference
/// report can point at.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct NamedSampleV1 {
    pub field: &'static str,
    pub raw: RawSampleV1,
}

/// One tick's samples at one phase, in a stable order.
///
/// Order is the caller's responsibility and is preserved exactly: the
/// row requires a stable component and entity order, and silently
/// sorting here would hide a caller whose order is not stable.
/// [`Self::order_is_stable_v1`] is how a caller checks itself.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericSampleSetV1 {
    phase: ProbePhaseV1,
    samples: Vec<NamedSampleV1>,
}

impl NumericSampleSetV1 {
    pub fn new_v1(phase: ProbePhaseV1, samples: Vec<NamedSampleV1>) -> Self {
        Self { phase, samples }
    }

    pub const fn phase_v1(&self) -> ProbePhaseV1 { self.phase }

    pub fn len_v1(&self) -> usize { self.samples.len() }

    /// Whether two sample sets present their fields in the same order.
    /// A caller with an unstable component order fails HERE rather than
    /// producing a divergence report about a field that merely moved.
    pub fn order_matches_v1(&self, other: &Self) -> bool {
        self.samples.len() == other.samples.len()
            && self.samples.iter().zip(&other.samples).all(|(a, b)| a.field == b.field)
    }

    /// Raw probe: the phase, then every field's name and BITS.
    pub fn raw_probe_v1(&self) -> ExactProbeV1 {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.phase.label().as_bytes());
        for sample in &self.samples {
            bytes.extend_from_slice(sample.field.as_bytes());
            bytes.extend_from_slice(&sample.raw.bits_v1().to_be_bytes());
        }
        ExactProbeV1::of_bytes_v1(&bytes)
    }

    /// Semantic probe under a validated spec.
    pub fn semantic_probe_v1(&self, spec: QuantizationSpecV1) -> QuantizedProbeV1 {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.phase.label().as_bytes());
        for sample in &self.samples {
            bytes.extend_from_slice(sample.field.as_bytes());
            bytes.extend_from_slice(
                &spec.quantise(f64::from(f32::from_bits(sample.raw.bits_v1() as u32))).to_be_bytes(),
            );
        }
        QuantizedProbeV1::of_bytes_v1(spec.policy, &bytes)
    }
}

/// What comparing two sample sets found.
#[derive(Clone, Debug, PartialEq)]
pub enum NumericProbeReportV1 {
    /// The two sets were taken at different phases and describe
    /// different worlds. Not a divergence.
    PhaseMismatch { left: ProbePhaseV1, right: ProbePhaseV1 },
    /// The two sets present their fields in different orders, so the
    /// caller's sampling order is not stable and no comparison is
    /// meaningful.
    UnstableFieldOrder,
    /// Both probes, plus the FIRST field whose raw bits differ. `None`
    /// means no field differed.
    Compared {
        probes: ProbeComparisonV1,
        first_differing_field: Option<&'static str>,
    },
}

/// Compare two sample sets at the digest level and report the first
/// differing field.
///
/// The first field, not a count: a count says how much moved, the first
/// field says what moved, and the rest is usually downstream of it.
pub fn compare_sample_sets_v1(
    left: &NumericSampleSetV1,
    right: &NumericSampleSetV1,
    spec: QuantizationSpecV1,
) -> NumericProbeReportV1 {
    if left.phase != right.phase {
        return NumericProbeReportV1::PhaseMismatch { left: left.phase, right: right.phase };
    }
    if !left.order_matches_v1(right) {
        return NumericProbeReportV1::UnstableFieldOrder;
    }

    let first_differing_field = left
        .samples
        .iter()
        .zip(&right.samples)
        .find(|(a, b)| a.raw != b.raw)
        .map(|(a, _)| a.field);

    NumericProbeReportV1::Compared {
        probes: compare_probes_v1(
            (left.raw_probe_v1(), left.semantic_probe_v1(spec)),
            (right.raw_probe_v1(), right.semantic_probe_v1(spec)),
        ),
        first_differing_field,
    }
}

/// The row's process requirement, carried as a value.
///
/// The quantisation policy is a gameplay-tolerance judgement, not a
/// numerics detail, and the row says it must be separately reviewed.
/// Nothing in this module ships a default spec for that reason: a
/// convenient default is how an unreviewed policy becomes the one
/// everybody uses.
///
/// **RULED 2026-07-28** (`APEX-T6.2`, doubling as `APEX-T7.3c`'s
/// divergence-metric spec -- one ruling, two consumers). The law and
/// its ruling are recorded beside the flag they flip, resolution-
/// policies style (`server::save_migration::RESOLUTION_LAW_V1`): a
/// ruling without its question is an instruction nobody can re-derive.
pub const QUANTIZATION_LAW_V1: &str = "quantization decides WHETHER states agree -- never WHAT \
     gets written; a correction always applies the authoritative values verbatim, no quantized \
     value ever feeds state";

/// The three field classes the law resolves into, and the non-finite
/// rule that applies across all of them.
pub const QUANTIZATION_RULING_V1: &str = "\
    (1) DISCRETE/SEMANTIC fields (CharacterState variant/kind, stance, wield, mount state): \
    EXACT equality, any mismatch is a divergence -- branch-driving discrete state has no \
    meaningful tolerance. \
    (2) CONTINUOUS physics (position/velocity/orientation): quantized comparison against named, \
    reviewed tolerance constants (apex::reconciliation_metric::{POS,VEL,ORI}_TOLERANCE_V1), \
    chosen below player perception AND below gameplay effect. \
    (3) ACCUMULATORS (energy/health-family): exact if integer-backed, tolerance at display \
    precision (0.01) if float. \
    NON-FINITE: any NaN/inf in a compared field is its own divergence reason -- never \
    quantized, never sentinel-mapped; the non-reflexivity trap this module's own semantic \
    probe closes for the determinism-audit case stays closed here too. \
    SEMANTICS: diverged iff any exact-class field differs OR any quantized-class field exceeds \
    tolerance; the FIRST differing field is recorded, not a count.";

pub const QUANTIZATION_POLICY_REVIEWED: bool = true;

#[cfg(test)]
mod numeric_probe_v1 {
    use super::*;

    fn spec(version: u16, scale: i64) -> QuantizationSpecV1 {
        QuantizationSpecV1 {
            policy: QuantizationPolicyV1::from_version_v1(version),
            scale,
            rounding: RoundingModeV1::NearestTiesAwayFromZero,
            saturation: 1_000_000_000,
            non_finite: NonFinitePolicyV1::ReservedSentinels,
        }
        .validated_v1()
        .expect("fixture spec is valid")
    }

    fn set(phase: ProbePhaseV1, fields: &[(&'static str, f32)]) -> NumericSampleSetV1 {
        NumericSampleSetV1::new_v1(
            phase,
            fields
                .iter()
                .map(|(field, value)| NamedSampleV1 {
                    field,
                    raw: RawSampleV1::of_f32_v1(*value),
                })
                .collect(),
        )
    }

    /// **The row's step 5, built as a canary rather than an aspiration.**
    /// Semantic probes match, raw probes differ. If this cannot be
    /// constructed the two probes are not independent and the semantic
    /// one is decorative.
    #[test]
    fn semantic_probes_match_while_raw_probes_differ() {
        let a = set(ProbePhaseV1::AfterPhysics, &[("vel.x", 1.000_000_1), ("vel.y", 0.0)]);
        let b = set(ProbePhaseV1::AfterPhysics, &[("vel.x", 1.000_000_2), ("vel.y", 0.0)]);
        let spec = spec(1, 1000);

        // Both halves of the premise, asserted rather than assumed.
        assert_ne!(a.raw_probe_v1(), b.raw_probe_v1(), "the fixture's raw bits do not differ");
        assert_eq!(
            a.semantic_probe_v1(spec),
            b.semantic_probe_v1(spec),
            "the fixture's quantised observations do not agree"
        );

        let NumericProbeReportV1::Compared { probes, first_differing_field } =
            compare_sample_sets_v1(&a, &b, spec)
        else {
            panic!("same phase and same order should compare");
        };
        assert_eq!(probes, ProbeComparisonV1::HiddenRawDrift);
        assert!(!probes.certifies_exact_execution_v1());
        assert_eq!(first_differing_field, Some("vel.x"));
    }

    /// `-0.0` and `0.0` are equal as floats and different as bits. The
    /// raw probe exists to see this, so it is tested directly.
    #[test]
    fn the_raw_probe_sees_negative_zero() {
        #[expect(clippy::neg_zero)]
        let negative = -0.0_f32;
        assert!(negative == 0.0, "the premise: these compare equal as floats");
        assert_ne!(
            RawSampleV1::of_f32_v1(negative),
            RawSampleV1::of_f32_v1(0.0),
            "to_bits did not distinguish -0.0 from 0.0"
        );

        let a = set(ProbePhaseV1::TickEnd, &[("z", negative)]);
        let b = set(ProbePhaseV1::TickEnd, &[("z", 0.0)]);
        assert_ne!(a.raw_probe_v1(), b.raw_probe_v1());
    }

    /// A NaN must not make the semantic probe non-reflexive. With a
    /// passthrough policy the probe would report a run as diverging from
    /// itself; the reserved sentinel is what prevents that, and the type
    /// has no passthrough variant to choose instead.
    #[test]
    fn a_nan_does_not_make_the_semantic_probe_disagree_with_itself() {
        let spec = spec(1, 1000);
        let with_nan = set(ProbePhaseV1::AfterPhysics, &[("x", f32::NAN), ("y", 1.0)]);

        assert_eq!(with_nan.semantic_probe_v1(spec), with_nan.semantic_probe_v1(spec));
        let NumericProbeReportV1::Compared { probes, first_differing_field } =
            compare_sample_sets_v1(&with_nan, &with_nan, spec)
        else {
            panic!("same set compared to itself");
        };
        assert_eq!(probes, ProbeComparisonV1::Identical);
        assert_eq!(first_differing_field, None);
    }

    /// Infinities keep their sign through quantisation, and neither
    /// collides with NaN. "This went to +inf" and "this went to -inf" are
    /// different findings.
    #[test]
    fn the_two_infinities_and_nan_stay_distinct() {
        let spec = spec(1, 1000);
        let pos = set(ProbePhaseV1::TickEnd, &[("x", f32::INFINITY)]);
        let neg = set(ProbePhaseV1::TickEnd, &[("x", f32::NEG_INFINITY)]);
        let nan = set(ProbePhaseV1::TickEnd, &[("x", f32::NAN)]);

        assert_ne!(pos.semantic_probe_v1(spec), neg.semantic_probe_v1(spec));
        assert_ne!(pos.semantic_probe_v1(spec), nan.semantic_probe_v1(spec));
        assert_ne!(neg.semantic_probe_v1(spec), nan.semantic_probe_v1(spec));
    }

    /// A probe taken at a different phase is a different measurement.
    /// Reported as such, never as a divergence.
    #[test]
    fn probes_from_different_phases_are_not_a_divergence() {
        let spec = spec(1, 1000);
        let after_physics = set(ProbePhaseV1::AfterPhysics, &[("x", 1.0)]);
        let tick_end = set(ProbePhaseV1::TickEnd, &[("x", 1.0)]);

        assert_eq!(
            compare_sample_sets_v1(&after_physics, &tick_end, spec),
            NumericProbeReportV1::PhaseMismatch {
                left: ProbePhaseV1::AfterPhysics,
                right: ProbePhaseV1::TickEnd,
            }
        );
        // Even with identical VALUES, the phase difference reaches the
        // digest, so a caller ignoring the report still cannot conclude
        // agreement.
        assert_ne!(after_physics.raw_probe_v1(), tick_end.raw_probe_v1());
    }

    /// An unstable field order is reported as such rather than as a
    /// divergence about whichever field happened to move.
    #[test]
    fn an_unstable_field_order_is_reported_not_diagnosed() {
        let spec = spec(1, 1000);
        let a = set(ProbePhaseV1::AfterPhysics, &[("x", 1.0), ("y", 2.0)]);
        let b = set(ProbePhaseV1::AfterPhysics, &[("y", 2.0), ("x", 1.0)]);
        assert_eq!(
            compare_sample_sets_v1(&a, &b, spec),
            NumericProbeReportV1::UnstableFieldOrder
        );
    }

    /// The first differing field is the FIRST one, not any one.
    #[test]
    fn the_report_names_the_first_differing_field() {
        let spec = spec(1, 1000);
        let a = set(ProbePhaseV1::TickStart, &[("a", 1.0), ("b", 2.0), ("c", 3.0)]);
        let b = set(ProbePhaseV1::TickStart, &[("a", 1.0), ("b", 9.0), ("c", 9.0)]);

        let NumericProbeReportV1::Compared { first_differing_field, .. } =
            compare_sample_sets_v1(&a, &b, spec)
        else {
            panic!("same phase and order");
        };
        assert_eq!(first_differing_field, Some("b"));
    }

    /// A spec that would always match, or one whose saturation reaches
    /// the sentinels, is rejected. Both would produce confidently wrong
    /// answers rather than failing.
    #[test]
    fn a_spec_that_cannot_discriminate_is_rejected() {
        let base = QuantizationSpecV1 {
            policy: QuantizationPolicyV1::from_version_v1(1),
            scale: 0,
            rounding: RoundingModeV1::TowardZero,
            saturation: 1000,
            non_finite: NonFinitePolicyV1::ReservedSentinels,
        };
        assert_eq!(
            base.validated_v1(),
            Err(QuantizationSpecErrorV1::NonPositiveScale(0))
        );

        let colliding = QuantizationSpecV1 { scale: 1000, saturation: i64::MAX, ..base };
        assert_eq!(
            colliding.validated_v1(),
            Err(QuantizationSpecErrorV1::SaturationCollidesWithSentinels(i64::MAX))
        );
    }

    /// A policy edit is visible in every probe taken under it, and two
    /// policies are incomparable rather than divergent.
    #[test]
    fn a_policy_edit_is_visible_and_not_reported_as_divergence() {
        let a = set(ProbePhaseV1::AfterPhysics, &[("x", 1.234_5)]);
        let one = a.semantic_probe_v1(spec(1, 1000));
        let two = a.semantic_probe_v1(spec(2, 1000));
        assert_ne!(one, two, "the same observation under two policies produced one probe");

        assert_eq!(
            compare_probes_v1((a.raw_probe_v1(), one), (a.raw_probe_v1(), two)),
            ProbeComparisonV1::IncomparablePolicies
        );
    }

    /// The row's process requirement is a value, not a comment: the flag
    /// says the policy HAS been reviewed, and the ruling is recorded
    /// beside it, not left implicit. This test previously asserted the
    /// opposite (`!QUANTIZATION_POLICY_REVIEWED`) — inverted by hand on
    /// the ruling landing, exactly as that assertion said to when it
    /// still held.
    #[test]
    fn the_quantization_policy_is_reviewed_and_its_ruling_is_recorded() {
        assert!(
            QUANTIZATION_POLICY_REVIEWED,
            "flip this back only if the ruling below is retracted, not merely loosened"
        );
        assert!(QUANTIZATION_LAW_V1.len() > 40, "the governing law is too vague to rule from");
        assert!(
            QUANTIZATION_RULING_V1.contains("DISCRETE"),
            "the ruling must cover the discrete/semantic class"
        );
        assert!(
            QUANTIZATION_RULING_V1.contains("CONTINUOUS"),
            "the ruling must cover the continuous-physics class"
        );
        assert!(
            QUANTIZATION_RULING_V1.contains("ACCUMULATOR"),
            "the ruling must cover the accumulator class"
        );
        assert!(
            QUANTIZATION_RULING_V1.contains("NON-FINITE"),
            "the ruling must cover the non-finite rule"
        );
    }

    /// Every phase has a distinct label, since the label is what enters
    /// the digest.
    #[test]
    fn every_phase_label_is_distinct() {
        let mut labels: Vec<&str> = ProbePhaseV1::ALL.iter().map(|p| p.label()).collect();
        labels.sort_unstable();
        let before = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), before, "two phases share a label, so their digests collide");
    }
}
