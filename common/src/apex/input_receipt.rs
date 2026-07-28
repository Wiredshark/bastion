//! `APEX-T5.3` — input receipts and dual prediction probes.
//!
//! Attribute the first divergence to one FIELD, and never let a quantised
//! observation certify exact execution.
//!
//! The probe pair itself is [`super::probe`], built once for its three
//! consumers. This module is the receipt: what the server says about one
//! input frame, and how a client's record is compared against it.
//!
//! **Two orderings are load-bearing and are enforced, not documented.**
//!
//! 1. **Generation is checked before any probe comparison.** A report
//!    from a generation older than the one in force was computed against
//!    a world the server has already corrected away; comparing its probes
//!    produces a divergence that is not a bug, and chasing it wastes the
//!    exact attention this row exists to focus. `T3.6`'s
//!    `PhysicsGenerationV1` is the authority, and
//!    [`ReceiptComparisonV1::StaleGeneration`] short-circuits before the
//!    probes are looked at.
//! 2. **Sequence acceptance is checked before that.** A receipt for a
//!    sequence the server never accepted describes nothing; its probes
//!    are whatever the client happened to compute.
//!
//! **The report is a FIRST MISMATCH, not a count.** A count says how much
//! is wrong; the first mismatch says what went wrong, and everything
//! after it is usually downstream of it. Same discipline as `T3.5.20`'s
//! perturbation harness.

use super::{
    physics_generation::PhysicsGenerationV1,
    probe::{ExactProbeV1, ProbeComparisonV1, QuantizedProbeV1, compare_probes_v1},
};

/// Why the server corrected a client.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CorrectionReasonV1 {
    /// No correction; the client's frame stood.
    None,
    /// The client's position was outside what the server allows.
    PositionRejected,
    /// The frame arrived for a generation the server had superseded.
    StaleGeneration,
    /// The frame's sequence was never accepted.
    UnacceptedSequence,
    /// The server took authority for a reason outside this frame (an
    /// admin force, a mount, a death).
    ServerAuthorityTaken,
}

/// What the server did with an input frame's sequence.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SequenceStateV1 {
    Accepted(u64),
    Rejected { sequence: u64, reason: CorrectionReasonV1 },
}

impl SequenceStateV1 {
    pub const fn sequence_v1(self) -> u64 {
        match self {
            Self::Accepted(sequence) | Self::Rejected { sequence, .. } => sequence,
        }
    }

    pub const fn accepted_v1(self) -> bool { matches!(self, Self::Accepted(_)) }
}

/// One input frame's receipt.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct InputReceiptV1 {
    pub sequence: SequenceStateV1,
    pub server_tick: u64,
    pub generation: PhysicsGenerationV1,
    pub correction: CorrectionReasonV1,
    pub exact: ExactProbeV1,
    pub quantised: QuantizedProbeV1,
}

/// The FIELD a comparison first disagreed on.
///
/// Ordered as the comparison checks them, so the variant also says how
/// far the comparison got.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ReceiptFieldV1 {
    Sequence,
    Acceptance,
    Generation,
    ServerTick,
    CorrectionReason,
    Probes,
}

/// The result of comparing a client's record against the server's
/// receipt.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ReceiptComparisonV1 {
    /// Every field agreed and the probes certify exact execution.
    Agreed,
    /// The client's frame was for a generation the server had already
    /// superseded. Reported BEFORE any probe comparison, because the
    /// probes of a superseded frame are expected to differ and are not
    /// evidence of anything.
    StaleGeneration { client: PhysicsGenerationV1, server: PhysicsGenerationV1 },
    /// The server never accepted this sequence, so there is nothing to
    /// compare probes against.
    SequenceNotAccepted { sequence: u64, reason: CorrectionReasonV1 },
    /// The first field the two records disagreed on. For
    /// [`ReceiptFieldV1::Probes`] the probe answer is carried, so
    /// "hidden raw drift" is not flattened into "mismatch".
    FirstMismatch { field: ReceiptFieldV1, probes: Option<ProbeComparisonV1> },
}

/// Compare a client's record against the server's receipt.
///
/// Order is the point. Sequence acceptance, then generation, then the
/// cheap scalar fields, then the probes. A caller cannot reorder these
/// because the order lives here rather than at every call site.
pub fn compare_receipts_v1(
    client: InputReceiptV1,
    server: InputReceiptV1,
) -> ReceiptComparisonV1 {
    if client.sequence.sequence_v1() != server.sequence.sequence_v1() {
        return ReceiptComparisonV1::FirstMismatch {
            field: ReceiptFieldV1::Sequence,
            probes: None,
        };
    }

    // (2) Sequence acceptance first: a receipt for an unaccepted sequence
    // describes nothing.
    if let SequenceStateV1::Rejected { sequence, reason } = server.sequence {
        return ReceiptComparisonV1::SequenceNotAccepted { sequence, reason };
    }
    if !client.sequence.accepted_v1() {
        return ReceiptComparisonV1::FirstMismatch {
            field: ReceiptFieldV1::Acceptance,
            probes: None,
        };
    }

    // (1) Then generation, still before any probe is touched.
    if client.generation != server.generation {
        return ReceiptComparisonV1::StaleGeneration {
            client: client.generation,
            server: server.generation,
        };
    }

    if client.server_tick != server.server_tick {
        return ReceiptComparisonV1::FirstMismatch {
            field: ReceiptFieldV1::ServerTick,
            probes: None,
        };
    }
    if client.correction != server.correction {
        return ReceiptComparisonV1::FirstMismatch {
            field: ReceiptFieldV1::CorrectionReason,
            probes: None,
        };
    }

    let probes = compare_probes_v1(
        (client.exact, client.quantised),
        (server.exact, server.quantised),
    );
    if probes.certifies_exact_execution_v1() {
        ReceiptComparisonV1::Agreed
    } else {
        ReceiptComparisonV1::FirstMismatch {
            field: ReceiptFieldV1::Probes,
            probes: Some(probes),
        }
    }
}

impl ReceiptComparisonV1 {
    /// Whether this comparison certifies that the client executed
    /// exactly what the server did.
    ///
    /// A `match` rather than a `matches!`: a new variant must be given a
    /// decision rather than silently inheriting `false`, which is the
    /// safe default and therefore the one that hides an omission.
    pub const fn certifies_exact_execution_v1(self) -> bool {
        match self {
            Self::Agreed => true,
            Self::StaleGeneration { .. }
            | Self::SequenceNotAccepted { .. }
            | Self::FirstMismatch { .. } => false,
        }
    }
}

/// The `PROBE` canary sketch, with what covers each.
pub const PROBE_CANARIES: [(&str, &str); 6] = [
    (
        "PROBE-001 quantised match with exact mismatch",
        "compare_probes_v1 returns HiddenRawDrift, which certifies nothing; the fixture asserts \
         its raw bytes really differ and its quantised bytes really agree",
    ),
    (
        "PROBE-002 exact match with quantised mismatch",
        "reported as QuantiserDisagreesOnIdenticalBytes — a quantiser that is not a function of \
         the bytes, not a divergence",
    ),
    (
        "PROBE-003 receipt for a stale generation",
        "generation is compared BEFORE any probe, and StaleGeneration short-circuits; the test \
         gives the two records deliberately different probes to prove the probes were not read",
    ),
    (
        "PROBE-004 receipt for an unaccepted sequence",
        "SequenceNotAccepted short-circuits ahead of generation and probes alike",
    ),
    (
        "PROBE-005 first-mismatch report truncated to a count",
        "the comparison returns a FIELD, and there is no count in the type at all — a count is \
         unrepresentable rather than discouraged",
    ),
    (
        "PROBE-006 a probe type converted into the other",
        "no From exists in either direction and neither is comparable to the other; two \
         compile_fail doctests in apex::probe pin both",
    ),
];

#[cfg(test)]
mod input_receipt_v1 {
    use super::{super::probe::QuantizationPolicyV1, *};
    use CorrectionReasonV1::PositionRejected;
    use ReceiptComparisonV1::{Agreed, FirstMismatch, SequenceNotAccepted, StaleGeneration};

    fn generation(n: u64) -> PhysicsGenerationV1 { PhysicsGenerationV1::from_legacy_counter_v1(n) }

    fn receipt(
        sequence: SequenceStateV1,
        tick: u64,
        gen_n: u64,
        correction: CorrectionReasonV1,
        bytes: &[u8],
    ) -> InputReceiptV1 {
        InputReceiptV1 {
            sequence,
            server_tick: tick,
            generation: generation(gen_n),
            correction,
            exact: ExactProbeV1::of_bytes_v1(bytes),
            quantised: QuantizedProbeV1::of_bytes_v1(
                QuantizationPolicyV1::from_version_v1(1),
                bytes,
            ),
        }
    }

    /// The acceptance path.
    #[test]
    fn an_agreeing_accepted_frame_certifies() {
        let r = receipt(SequenceStateV1::Accepted(7), 100, 3, CorrectionReasonV1::None, b"state");
        assert_eq!(compare_receipts_v1(r, r), Agreed);
        assert!(compare_receipts_v1(r, r).certifies_exact_execution_v1());
    }

    /// `PROBE-004`. The rejection path short-circuits ahead of everything
    /// else: an unaccepted sequence's probes are whatever the client
    /// happened to compute.
    #[test]
    fn an_unaccepted_sequence_short_circuits_before_generation_and_probes() {
        let client = receipt(SequenceStateV1::Accepted(9), 100, 3, CorrectionReasonV1::None, b"client state");
        let server = receipt(
            SequenceStateV1::Rejected { sequence: 9, reason: PositionRejected },
            // Deliberately different tick, generation and probes: if any
            // of them were consulted first, this test would report them.
            999,
            42,
            PositionRejected,
            b"server state",
        );

        assert_eq!(compare_receipts_v1(client, server), SequenceNotAccepted {
            sequence: 9,
            reason: PositionRejected
        });
    }

    /// `PROBE-003`. Generation is checked BEFORE any probe. The two
    /// records are given deliberately different probes, so a comparison
    /// that read them would report a probe mismatch instead — which is
    /// how the test proves the ordering rather than assuming it.
    #[test]
    fn a_stale_generation_is_reported_before_any_probe_is_compared() {
        let client = receipt(SequenceStateV1::Accepted(4), 50, 2, CorrectionReasonV1::None, b"old world");
        let server = receipt(SequenceStateV1::Accepted(4), 50, 5, CorrectionReasonV1::None, b"new world");

        assert_eq!(compare_receipts_v1(client, server), StaleGeneration {
            client: generation(2),
            server: generation(5),
        });
        assert!(!compare_receipts_v1(client, server).certifies_exact_execution_v1());
    }

    /// `PROBE-001` at the receipt level: the tier's non-vacuity case
    /// survives the trip through a receipt. Quantised agree, exact do
    /// not, and the answer is carried rather than flattened.
    #[test]
    fn hidden_raw_drift_reaches_the_caller_as_hidden_raw_drift() {
        let policy = QuantizationPolicyV1::from_version_v1(1);
        let mut client = receipt(SequenceStateV1::Accepted(1), 10, 1, CorrectionReasonV1::None, b"raw a");
        let mut server = receipt(SequenceStateV1::Accepted(1), 10, 1, CorrectionReasonV1::None, b"raw b");
        // Same quantised bucket, different raw bits.
        let bucket = QuantizedProbeV1::of_bytes_v1(policy, b"one bucket");
        client.quantised = bucket;
        server.quantised = bucket;
        assert_ne!(client.exact, server.exact);

        assert_eq!(compare_receipts_v1(client, server), FirstMismatch {
            field: ReceiptFieldV1::Probes,
            probes: Some(ProbeComparisonV1::HiddenRawDrift),
        });
        assert!(!compare_receipts_v1(client, server).certifies_exact_execution_v1());
    }

    /// `PROBE-005`. The report names the FIRST field, and it names the
    /// earliest one when several disagree — otherwise it would be a
    /// report about whichever field the comparison happened to reach.
    #[test]
    fn the_report_names_the_earliest_disagreeing_field() {
        // Tick, correction reason and probes all disagree at once.
        let client = receipt(SequenceStateV1::Accepted(2), 10, 1, CorrectionReasonV1::None, b"a");
        let server = receipt(SequenceStateV1::Accepted(2), 11, 1, PositionRejected, b"b");

        assert_eq!(compare_receipts_v1(client, server), FirstMismatch {
            field: ReceiptFieldV1::ServerTick,
            probes: None,
        });

        // With the tick agreeing, the next field is reported — not the
        // probes, which also disagree.
        let server = receipt(SequenceStateV1::Accepted(2), 10, 1, PositionRejected, b"b");
        assert_eq!(compare_receipts_v1(client, server), FirstMismatch {
            field: ReceiptFieldV1::CorrectionReason,
            probes: None,
        });
    }

    /// A sequence mismatch is reported as such, ahead of acceptance:
    /// comparing two records about different frames is not a divergence.
    #[test]
    fn records_about_different_sequences_are_a_sequence_mismatch() {
        let client = receipt(SequenceStateV1::Accepted(1), 10, 1, CorrectionReasonV1::None, b"a");
        let server = receipt(SequenceStateV1::Accepted(2), 10, 1, CorrectionReasonV1::None, b"a");
        assert_eq!(compare_receipts_v1(client, server), FirstMismatch {
            field: ReceiptFieldV1::Sequence,
            probes: None,
        });
    }

    /// A client that thinks it was rejected while the server accepted is
    /// an acceptance disagreement, not a probe divergence.
    #[test]
    fn disagreement_about_acceptance_is_its_own_field() {
        let client = InputReceiptV1 {
            sequence: SequenceStateV1::Rejected { sequence: 3, reason: PositionRejected },
            ..receipt(SequenceStateV1::Accepted(3), 10, 1, CorrectionReasonV1::None, b"a")
        };
        let server = receipt(SequenceStateV1::Accepted(3), 10, 1, CorrectionReasonV1::None, b"a");
        assert_eq!(compare_receipts_v1(client, server), FirstMismatch {
            field: ReceiptFieldV1::Acceptance,
            probes: None,
        });
    }

    /// The field order is the comparison order, so a reader can tell how
    /// far a comparison got from the variant alone.
    #[test]
    fn the_field_order_is_the_comparison_order() {
        let mut fields = [
            ReceiptFieldV1::Probes,
            ReceiptFieldV1::Sequence,
            ReceiptFieldV1::CorrectionReason,
            ReceiptFieldV1::Acceptance,
            ReceiptFieldV1::ServerTick,
            ReceiptFieldV1::Generation,
        ];
        fields.sort();
        assert_eq!(fields, [
            ReceiptFieldV1::Sequence,
            ReceiptFieldV1::Acceptance,
            ReceiptFieldV1::Generation,
            ReceiptFieldV1::ServerTick,
            ReceiptFieldV1::CorrectionReason,
            ReceiptFieldV1::Probes,
        ]);
    }

    /// Every canary in the sketch says what covers it.
    #[test]
    fn every_probe_canary_states_its_coverage() {
        for (name, why) in PROBE_CANARIES {
            assert!(why.len() > 40, "{name} is claimed without evidence: {why:?}");
        }
    }
}
