//! `APEX` — the exact/quantised probe pair, built ONCE.
//!
//! Three rows need the same two probes: `T5.3` (input receipts), `T6.2`
//! (raw and semantic numeric probes) and `T8.1` (economy determinism).
//! The dependency map's rule is that three consumers is the threshold for
//! building a thing once, and naming its consumers in its own doc — so
//! they are named here rather than in a map nobody reads at the moment
//! they matter.
//!
//! **The invariant, and why it is a type and not a sentence.** A
//! quantised observation must never be able to certify exact execution.
//! Documentation asking politely is not enough: the mistake it prevents
//! is one line of plausible-looking code, written by someone who has both
//! values in scope and wants a boolean. So:
//!
//! - [`ExactProbeV1`] and [`QuantizedProbeV1`] are distinct types,
//! - there is no `From` in either direction,
//! - neither implements a comparison that accepts the other,
//! - and [`QuantizedProbeV1`] carries its POLICY VERSION inside its
//!   identity, so two quantised probes taken under different tolerance
//!   policies are not equal even when their digests are.
//!
//! That last point is the one a documented rule always loses: a
//! quantisation policy changes, the digests happen to coincide, and a
//! comparison silently starts meaning something else.
//!
//! Lineage: the same move as `T3.5`'s commit sink (methods return unit,
//! so a recoverable mid-commit failure is unrepresentable) and `T3.4`'s
//! `production_checkpoint_profile_v1` (returns an error, because there is
//! no production profile to invent).

use super::digest::{DigestBytes32V1, hash_artifact_bytes_v1};

/// Hash of the raw authoritative bits.
///
/// **This is the one that can certify.** Equality here means the bytes
/// were the same.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ExactProbeV1(DigestBytes32V1);

impl ExactProbeV1 {
    pub fn of_bytes_v1(bytes: &[u8]) -> Self {
        Self(hash_artifact_bytes_v1(bytes).digest.bytes)
    }

    pub const fn digest_v1(&self) -> &DigestBytes32V1 { &self.0 }
}

/// Which tolerance policy a quantised probe was taken under.
///
/// Part of the probe's identity rather than metadata beside it. Two
/// probes taken under different policies describe different questions,
/// and a type that let them compare equal would answer the wrong one.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct QuantizationPolicyV1(u16);

impl QuantizationPolicyV1 {
    pub const fn from_version_v1(version: u16) -> Self { Self(version) }

    pub const fn version_v1(self) -> u16 { self.0 }
}

/// Hash of a quantised observation, for tolerance analysis.
///
/// **This one can never certify exact execution.** A match means two runs
/// agreed *within a policy's tolerance*, which is a strictly weaker claim
/// than byte equality and is not convertible into it.
///
/// ```compile_fail
/// # use veloren_common::apex::probe::*;
/// let quantised = QuantizedProbeV1::of_bytes_v1(QuantizationPolicyV1::from_version_v1(1), b"x");
/// // A quantised observation must not become an exact one.
/// let exact: ExactProbeV1 = quantised.into();
/// ```
///
/// ```compile_fail
/// # use veloren_common::apex::probe::*;
/// let exact = ExactProbeV1::of_bytes_v1(b"x");
/// let quantised = QuantizedProbeV1::of_bytes_v1(QuantizationPolicyV1::from_version_v1(1), b"x");
/// // ...and the two must not be comparable to each other in either
/// // direction, or "they matched" becomes ambiguous.
/// let _ = exact == quantised;
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct QuantizedProbeV1 {
    policy: QuantizationPolicyV1,
    digest: DigestBytes32V1,
}

impl QuantizedProbeV1 {
    /// The policy version is folded into the hashed bytes, so a probe
    /// cannot be re-labelled with a different policy after the fact.
    pub fn of_bytes_v1(policy: QuantizationPolicyV1, quantised_bytes: &[u8]) -> Self {
        let mut buf = Vec::with_capacity(quantised_bytes.len() + 2);
        buf.extend_from_slice(&policy.version_v1().to_be_bytes());
        buf.extend_from_slice(quantised_bytes);
        Self { policy, digest: hash_artifact_bytes_v1(&buf).digest.bytes }
    }

    pub const fn policy_v1(&self) -> QuantizationPolicyV1 { self.policy }

    pub const fn digest_v1(&self) -> &DigestBytes32V1 { &self.digest }
}

/// What a probe pair says about two runs.
///
/// The interesting variant is [`Self::HiddenRawDrift`]: quantised probes
/// agree and exact probes do not. That is the case the whole pair exists
/// to surface — gameplay tolerance masking a real bit-level divergence —
/// and it has its own name so it cannot be reported as "matched".
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProbeComparisonV1 {
    /// Both agree. The strongest statement available.
    Identical,
    /// Exact probes agree; quantised do not. Either the quantisation
    /// policies differ, or the quantiser is not a function of the bytes.
    QuantiserDisagreesOnIdenticalBytes,
    /// Quantised agree; exact do not. Real divergence, masked.
    HiddenRawDrift,
    /// Neither agrees.
    Divergent,
    /// The quantised probes were taken under different policies, so they
    /// are not comparable at all. Not a match and not a mismatch —
    /// collapsing it into either would be an answer to a question nobody
    /// asked.
    IncomparablePolicies,
}

/// Compare two probe pairs. The ONLY sanctioned way to relate an exact
/// probe to a quantised one: through a result type that keeps the two
/// answers separate.
pub fn compare_probes_v1(
    left: (ExactProbeV1, QuantizedProbeV1),
    right: (ExactProbeV1, QuantizedProbeV1),
) -> ProbeComparisonV1 {
    if left.1.policy_v1() != right.1.policy_v1() {
        return ProbeComparisonV1::IncomparablePolicies;
    }
    match (left.0 == right.0, left.1 == right.1) {
        (true, true) => ProbeComparisonV1::Identical,
        (true, false) => ProbeComparisonV1::QuantiserDisagreesOnIdenticalBytes,
        (false, true) => ProbeComparisonV1::HiddenRawDrift,
        (false, false) => ProbeComparisonV1::Divergent,
    }
}

impl ProbeComparisonV1 {
    /// Whether this comparison certifies exact execution.
    ///
    /// Exactly one variant does. Written as a match rather than a
    /// `matches!` so adding a variant forces a decision about it instead
    /// of defaulting it to "does not certify" — which is the safe default
    /// and therefore the one that hides a mistake.
    pub const fn certifies_exact_execution_v1(self) -> bool {
        match self {
            Self::Identical => true,
            Self::QuantiserDisagreesOnIdenticalBytes
            | Self::HiddenRawDrift
            | Self::Divergent
            | Self::IncomparablePolicies => false,
        }
    }
}

/// The three rows that consume this pair. Named here so a fourth
/// consumer is a deliberate addition rather than a quiet one.
pub const PROBE_CONSUMERS: [&str; 3] = [
    "T5.3 input receipts",
    "T6.2 raw and semantic numeric probes",
    "T8.1 economy determinism",
];

#[cfg(test)]
mod probe_v1 {
    use super::*;

    fn policy(v: u16) -> QuantizationPolicyV1 { QuantizationPolicyV1::from_version_v1(v) }

    /// **The tier's non-vacuity case.** Quantised probes match, exact
    /// probes do not. If the two probes were not genuinely independent,
    /// this could not be constructed at all.
    #[test]
    fn hidden_raw_drift_is_its_own_answer_and_certifies_nothing() {
        // Two runs whose raw bits differ in the last place but whose
        // quantised observation is the same bucket.
        let raw_a = 1.000_000_1_f32.to_bits().to_be_bytes();
        let raw_b = 1.000_000_2_f32.to_bits().to_be_bytes();
        assert_ne!(raw_a, raw_b, "the fixture's raw bytes must differ or the test is vacuous");

        // A quantiser to 3 decimal places puts both in one bucket.
        let bucket = |v: f32| ((v * 1000.0).round() as i64).to_be_bytes();
        let quantised_a = bucket(1.000_000_1);
        let quantised_b = bucket(1.000_000_2);
        assert_eq!(quantised_a, quantised_b, "the fixture's quantised bytes must agree");

        let comparison = compare_probes_v1(
            (
                ExactProbeV1::of_bytes_v1(&raw_a),
                QuantizedProbeV1::of_bytes_v1(policy(1), &quantised_a),
            ),
            (
                ExactProbeV1::of_bytes_v1(&raw_b),
                QuantizedProbeV1::of_bytes_v1(policy(1), &quantised_b),
            ),
        );

        assert_eq!(comparison, ProbeComparisonV1::HiddenRawDrift);
        assert!(!comparison.certifies_exact_execution_v1());
    }

    /// The only variant that certifies is `Identical`, and it certifies
    /// because the EXACT probes matched.
    #[test]
    fn exactly_one_comparison_certifies_exact_execution() {
        let pair = (
            ExactProbeV1::of_bytes_v1(b"same"),
            QuantizedProbeV1::of_bytes_v1(policy(1), b"same"),
        );
        assert_eq!(compare_probes_v1(pair, pair), ProbeComparisonV1::Identical);
        assert!(compare_probes_v1(pair, pair).certifies_exact_execution_v1());

        for other in [
            ProbeComparisonV1::QuantiserDisagreesOnIdenticalBytes,
            ProbeComparisonV1::HiddenRawDrift,
            ProbeComparisonV1::Divergent,
            ProbeComparisonV1::IncomparablePolicies,
        ] {
            assert!(!other.certifies_exact_execution_v1(), "{other:?} certified exact execution");
        }
    }

    /// A quantised probe's policy is part of its identity: the same
    /// bytes under two policies are two different probes, and comparing
    /// them is `IncomparablePolicies` rather than a mismatch.
    #[test]
    fn a_policy_change_makes_probes_incomparable_not_merely_unequal() {
        let under_one = QuantizedProbeV1::of_bytes_v1(policy(1), b"observation");
        let under_two = QuantizedProbeV1::of_bytes_v1(policy(2), b"observation");
        assert_ne!(under_one, under_two);
        assert_ne!(
            under_one.digest_v1(),
            under_two.digest_v1(),
            "the policy version is not folded into the digest, so a probe could be re-labelled"
        );

        let exact = ExactProbeV1::of_bytes_v1(b"bytes");
        assert_eq!(
            compare_probes_v1((exact, under_one), (exact, under_two)),
            ProbeComparisonV1::IncomparablePolicies,
            "a policy change was reported as a divergence, which would send someone hunting a \
             bug that is really a policy edit"
        );
    }

    /// A quantiser that is not a function of the bytes is a distinct
    /// finding from a divergence — the raw bits agreed.
    #[test]
    fn a_quantiser_disagreeing_on_identical_bytes_is_its_own_answer() {
        let exact = ExactProbeV1::of_bytes_v1(b"identical");
        let comparison = compare_probes_v1(
            (exact, QuantizedProbeV1::of_bytes_v1(policy(3), b"a")),
            (exact, QuantizedProbeV1::of_bytes_v1(policy(3), b"b")),
        );
        assert_eq!(comparison, ProbeComparisonV1::QuantiserDisagreesOnIdenticalBytes);
        assert!(!comparison.certifies_exact_execution_v1());
    }

    /// Divergence on both axes is reported as divergence, not as hidden
    /// drift — the masking claim requires the quantised probes to agree.
    #[test]
    fn divergence_on_both_axes_is_not_reported_as_hidden_drift() {
        let comparison = compare_probes_v1(
            (
                ExactProbeV1::of_bytes_v1(b"a"),
                QuantizedProbeV1::of_bytes_v1(policy(1), b"a"),
            ),
            (
                ExactProbeV1::of_bytes_v1(b"b"),
                QuantizedProbeV1::of_bytes_v1(policy(1), b"b"),
            ),
        );
        assert_eq!(comparison, ProbeComparisonV1::Divergent);
    }

    /// The consumer list is real, so a fourth consumer is deliberate.
    #[test]
    fn the_consumer_list_names_three_rows() {
        assert_eq!(PROBE_CONSUMERS.len(), 3);
        for consumer in PROBE_CONSUMERS {
            assert!(consumer.starts_with('T'), "{consumer} is not a row");
        }
    }
}
