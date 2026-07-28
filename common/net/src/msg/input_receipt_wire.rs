//! `APEX-T5.3` step 2 — the input receipt's WIRE form.
//!
//! [`common::apex::input_receipt::InputReceiptV1`] is the in-memory type.
//! It is deliberately not `Serialize`: its probes wrap
//! `DigestBytes32V1`, a `T0.3` identity type that has no serde impl and
//! should not acquire one just because one message wants to carry it.
//! Adding serde to a sealed identity type to satisfy a transport is how
//! an identity becomes something the wire can assert rather than
//! something the receiver recomputes.
//!
//! So the wire carries plain bytes and the receiver RECONSTRUCTS the
//! typed receipt from them. That is the program's recompute-don't-trust
//! rule applied at its smallest scale: nothing arrives already typed.

use common::apex::{
    input_receipt::{CorrectionReasonV1, InputReceiptV1, SequenceStateV1},
    physics_generation::PhysicsGenerationV1,
    probe::{ExactProbeV1, QuantizationPolicyV1, QuantizedProbeV1},
};
use serde::{Deserialize, Serialize};

/// `SequenceStateV1` on the wire.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SequenceStateWireV1 {
    Accepted(u64),
    Rejected { sequence: u64, reason: CorrectionReasonWireV1 },
}

/// `CorrectionReasonV1` on the wire. A separate enum rather than a serde
/// derive on the original, so a reason added in `common` cannot silently
/// change the wire vocabulary — it has to be added here too, which is
/// where `T3.3`'s frozen tag table will notice it.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CorrectionReasonWireV1 {
    None,
    PositionRejected,
    StaleGeneration,
    UnacceptedSequence,
    ServerAuthorityTaken,
}

impl CorrectionReasonWireV1 {
    pub const fn to_typed_v1(self) -> CorrectionReasonV1 {
        match self {
            Self::None => CorrectionReasonV1::None,
            Self::PositionRejected => CorrectionReasonV1::PositionRejected,
            Self::StaleGeneration => CorrectionReasonV1::StaleGeneration,
            Self::UnacceptedSequence => CorrectionReasonV1::UnacceptedSequence,
            Self::ServerAuthorityTaken => CorrectionReasonV1::ServerAuthorityTaken,
        }
    }

    pub const fn from_typed_v1(reason: CorrectionReasonV1) -> Self {
        match reason {
            CorrectionReasonV1::None => Self::None,
            CorrectionReasonV1::PositionRejected => Self::PositionRejected,
            CorrectionReasonV1::StaleGeneration => Self::StaleGeneration,
            CorrectionReasonV1::UnacceptedSequence => Self::UnacceptedSequence,
            CorrectionReasonV1::ServerAuthorityTaken => Self::ServerAuthorityTaken,
        }
    }
}

/// One input receipt, as bytes.
///
/// The probe digests travel as raw arrays. The receiver does not get a
/// `ExactProbeV1` off the wire — it gets 32 bytes and builds one, which
/// is the only construction path that keeps "this is an exact probe"
/// a statement the receiver makes rather than one the sender claims.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputReceiptWireV1 {
    pub sequence: SequenceStateWireV1,
    pub server_tick: u64,
    pub generation: PhysicsGenerationV1,
    pub correction: CorrectionReasonWireV1,
    pub exact_digest: [u8; 32],
    pub quantised_policy: u16,
    pub quantised_digest: [u8; 32],
}

impl InputReceiptWireV1 {
    pub fn from_typed_v1(receipt: InputReceiptV1) -> Self {
        Self {
            sequence: match receipt.sequence {
                SequenceStateV1::Accepted(sequence) => SequenceStateWireV1::Accepted(sequence),
                SequenceStateV1::Rejected { sequence, reason } => SequenceStateWireV1::Rejected {
                    sequence,
                    reason: CorrectionReasonWireV1::from_typed_v1(reason),
                },
            },
            server_tick: receipt.server_tick,
            generation: receipt.generation,
            correction: CorrectionReasonWireV1::from_typed_v1(receipt.correction),
            exact_digest: *receipt.exact.digest_v1().as_array(),
            quantised_policy: receipt.quantised.policy_v1().version_v1(),
            quantised_digest: *receipt.quantised.digest_v1().as_array(),
        }
    }

    /// Rebuild the typed receipt from the bytes.
    ///
    /// Total: every wire value maps to a typed one, so a malformed
    /// receipt is a decode failure at the envelope rather than a
    /// half-built receipt here.
    pub fn to_typed_v1(self) -> InputReceiptV1 {
        InputReceiptV1 {
            sequence: match self.sequence {
                SequenceStateWireV1::Accepted(sequence) => SequenceStateV1::Accepted(sequence),
                SequenceStateWireV1::Rejected { sequence, reason } => SequenceStateV1::Rejected {
                    sequence,
                    reason: reason.to_typed_v1(),
                },
            },
            server_tick: self.server_tick,
            generation: self.generation,
            correction: self.correction.to_typed_v1(),
            exact: ExactProbeV1::from_digest_bytes_v1(self.exact_digest),
            quantised: QuantizedProbeV1::from_digest_bytes_v1(
                QuantizationPolicyV1::from_version_v1(self.quantised_policy),
                self.quantised_digest,
            ),
        }
    }
}

#[cfg(test)]
mod input_receipt_wire_v1 {
    use super::*;

    fn typed() -> InputReceiptV1 {
        InputReceiptV1 {
            sequence: SequenceStateV1::Accepted(41),
            server_tick: 900,
            generation: PhysicsGenerationV1::from_legacy_counter_v1(3),
            correction: CorrectionReasonV1::None,
            exact: ExactProbeV1::of_bytes_v1(b"authoritative state"),
            quantised: QuantizedProbeV1::of_bytes_v1(
                QuantizationPolicyV1::from_version_v1(1),
                b"quantised observation",
            ),
        }
    }

    /// The round trip is lossless through the REAL encoder the messages
    /// use, not just through the conversion functions — a conversion that
    /// round-trips in memory but not through bincode would still be a
    /// broken transport.
    #[test]
    fn a_receipt_round_trips_through_the_wire_encoder() {
        let before = typed();
        let wire = InputReceiptWireV1::from_typed_v1(before);
        let bytes = bincode::serde::encode_to_vec(wire, bincode::config::legacy())
            .expect("receipt encodes");
        let (decoded, _): (InputReceiptWireV1, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::legacy())
                .expect("receipt decodes");
        assert_eq!(decoded, wire);
        assert_eq!(decoded.to_typed_v1(), before);
    }

    /// A rejected sequence carries its reason across the wire; the reason
    /// is what tells a client whether it was stale or forged.
    #[test]
    fn a_rejection_keeps_its_reason() {
        let before = InputReceiptV1 {
            sequence: SequenceStateV1::Rejected {
                sequence: 7,
                reason: CorrectionReasonV1::StaleGeneration,
            },
            correction: CorrectionReasonV1::StaleGeneration,
            ..typed()
        };
        let after = InputReceiptWireV1::from_typed_v1(before).to_typed_v1();
        assert_eq!(after, before);
        assert!(!after.sequence.accepted_v1());
    }

    /// The quantisation policy survives. Losing it would let two probes
    /// taken under different tolerance policies compare as though they
    /// were the same measurement.
    #[test]
    fn the_quantisation_policy_survives_the_wire() {
        let before = InputReceiptV1 {
            quantised: QuantizedProbeV1::of_bytes_v1(
                QuantizationPolicyV1::from_version_v1(9),
                b"observation",
            ),
            ..typed()
        };
        let after = InputReceiptWireV1::from_typed_v1(before).to_typed_v1();
        assert_eq!(after.quantised.policy_v1().version_v1(), 9);
        assert_eq!(after, before);
    }

    /// Every correction reason maps both ways. A reason added to the
    /// typed enum without a wire variant fails to compile here, which is
    /// the point of not deriving serde on the original.
    #[test]
    fn every_correction_reason_maps_both_ways() {
        for reason in [
            CorrectionReasonV1::None,
            CorrectionReasonV1::PositionRejected,
            CorrectionReasonV1::StaleGeneration,
            CorrectionReasonV1::UnacceptedSequence,
            CorrectionReasonV1::ServerAuthorityTaken,
        ] {
            assert_eq!(
                CorrectionReasonWireV1::from_typed_v1(reason).to_typed_v1(),
                reason
            );
        }
    }
}
