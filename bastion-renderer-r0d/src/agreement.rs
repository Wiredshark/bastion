//! BUILD-007A10.4 (part 2 of 2) — transcript-bound server/client agreement
//! substrate (design §10).
//!
//! - §10.1 transcript hash over the canonical Offer→Ack→Commit frames.
//! - §10.2 the linear agreement state machine (no phase/camera/capture may
//!   advance before `AgreementCommitted`); only the immediate successor
//!   transition is legal.
//! - §10.4 fragmentation: canonical content is transport-fragmentation
//!   independent; reassembly verifies unique indices, exact total length, and
//!   the whole-message digest before decoding, under the 16 MiB hard cap.
//!
//! The live TLS-inspired handshake over the real network stack is the
//! integration surface; this module is the self-contained transcript/state/
//! fragmentation core with golden vectors.

use sha2::{Digest, Sha256};

/// Fragment payload threshold (§10.4): payloads over 64 KiB are split.
pub const FRAGMENT_THRESHOLD: usize = 64 * 1024;
/// Manifest/projection hard cap (§10.4): 16 MiB each.
pub const MESSAGE_HARD_CAP: usize = 16 * 1024 * 1024;

/// The linear agreement state machine (§10.2). Discriminants are ordered; the
/// only legal transition is to the immediate successor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgreementStateV1 {
    Created,
    OfferSent,
    OfferValidated,
    ReplicationApplied,
    ClientProjectionComplete,
    AckSent,
    AckValidated,
    CommitSent,
    CommitValidated,
    AgreementCommitted,
}

impl AgreementStateV1 {
    const ORDER: [AgreementStateV1; 10] = [
        AgreementStateV1::Created,
        AgreementStateV1::OfferSent,
        AgreementStateV1::OfferValidated,
        AgreementStateV1::ReplicationApplied,
        AgreementStateV1::ClientProjectionComplete,
        AgreementStateV1::AckSent,
        AgreementStateV1::AckValidated,
        AgreementStateV1::CommitSent,
        AgreementStateV1::CommitValidated,
        AgreementStateV1::AgreementCommitted,
    ];

    fn ordinal(self) -> usize {
        Self::ORDER.iter().position(|&s| s == self).expect("in ORDER")
    }

    /// The immediate successor, or `None` at the terminal committed state.
    #[must_use]
    pub fn next(self) -> Option<AgreementStateV1> {
        Self::ORDER.get(self.ordinal() + 1).copied()
    }

    /// Only the immediate successor is a legal advance (§10.2): no skipping,
    /// no reordering, no staying. Any other transition is rejected.
    #[must_use]
    pub fn can_advance_to(self, to: AgreementStateV1) -> bool {
        self.next() == Some(to)
    }

    /// Nothing downstream (script phase, camera, readiness, capture) may proceed
    /// until both sides reach the terminal committed state (§10.2).
    #[must_use]
    pub fn is_committed(self) -> bool {
        self == AgreementStateV1::AgreementCommitted
    }
}

/// Length-frame one handshake message (`u64_le(len) || bytes`) for the
/// transcript hash — transport framing that cannot alias across message
/// boundaries.
fn frame(out: &mut Vec<u8>, msg: &[u8]) {
    out.extend_from_slice(&(msg.len() as u64).to_le_bytes());
    out.extend_from_slice(msg);
}

/// The committed transcript hash (§10.1):
/// `SHA256("bastion/r0d/agreement/v1" || frame(Offer) || frame(Ack) || frame(Commit))`.
#[must_use]
pub fn transcript_hash(offer: &[u8], ack: &[u8], commit: &[u8]) -> [u8; 32] {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"bastion/r0d/agreement/v1");
    frame(&mut buf, offer);
    frame(&mut buf, ack);
    frame(&mut buf, commit);
    Sha256::digest(&buf).into()
}

/// Typed agreement failures (§10.3/§10.4). All terminal within the run epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgreementError {
    /// Payload exceeds the 16 MiB manifest/projection cap (§10.4).
    OversizedMessage { len: usize, cap: usize },
    /// No fragments to reassemble.
    NoFragments,
    /// Fragments disagree on `fragment_count`.
    InconsistentCount,
    /// Fragments disagree on `message_digest`.
    InconsistentDigest,
    /// A fragment index was missing or duplicated.
    IndexGap { index: u32 },
    /// Reassembled length did not match the declared total.
    LengthMismatch { expected: u64, got: u64 },
    /// Whole-message digest did not match after reassembly.
    DigestMismatch,
}

/// One transport fragment (§10.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentV1 {
    pub message_digest: [u8; 32],
    pub fragment_index: u32,
    pub fragment_count: u32,
    pub total_payload_len: u64,
    pub fragment_payload: Vec<u8>,
}

/// Split a canonical message into transport fragments (§10.4). The message
/// digest binds every fragment to the whole; fragmentation is invisible to the
/// canonical content.
pub fn fragment_message(payload: &[u8]) -> Result<Vec<FragmentV1>, AgreementError> {
    if payload.len() > MESSAGE_HARD_CAP {
        return Err(AgreementError::OversizedMessage {
            len: payload.len(),
            cap: MESSAGE_HARD_CAP,
        });
    }
    let digest: [u8; 32] = Sha256::digest(payload).into();
    let chunks: Vec<&[u8]> = if payload.is_empty() {
        vec![&[][..]]
    } else {
        payload.chunks(FRAGMENT_THRESHOLD).collect()
    };
    let count = chunks.len() as u32;
    Ok(chunks
        .into_iter()
        .enumerate()
        .map(|(i, c)| FragmentV1 {
            message_digest: digest,
            fragment_index: i as u32,
            fragment_count: count,
            total_payload_len: payload.len() as u64,
            fragment_payload: c.to_vec(),
        })
        .collect())
}

/// Reassemble fragments (§10.4). Verifies consistent count/digest, unique
/// contiguous indices, exact total length, and the whole-message digest before
/// returning. Arrival order is irrelevant.
pub fn reassemble(mut fragments: Vec<FragmentV1>) -> Result<Vec<u8>, AgreementError> {
    if fragments.is_empty() {
        return Err(AgreementError::NoFragments);
    }
    let count = fragments[0].fragment_count;
    let digest = fragments[0].message_digest;
    let total = fragments[0].total_payload_len;
    if fragments.iter().any(|f| f.fragment_count != count) {
        return Err(AgreementError::InconsistentCount);
    }
    if fragments.iter().any(|f| f.message_digest != digest) {
        return Err(AgreementError::InconsistentDigest);
    }
    if fragments.len() as u32 != count {
        // Missing or duplicate fragment: sort and find the first gap for a
        // precise typed error.
        fragments.sort_by_key(|f| f.fragment_index);
        for (i, f) in fragments.iter().enumerate() {
            if f.fragment_index != i as u32 {
                return Err(AgreementError::IndexGap { index: i as u32 });
            }
        }
        return Err(AgreementError::IndexGap { index: count - 1 });
    }
    fragments.sort_by_key(|f| f.fragment_index);
    let mut out = Vec::with_capacity(total as usize);
    for (i, f) in fragments.iter().enumerate() {
        if f.fragment_index != i as u32 {
            return Err(AgreementError::IndexGap { index: i as u32 });
        }
        out.extend_from_slice(&f.fragment_payload);
    }
    if out.len() as u64 != total {
        return Err(AgreementError::LengthMismatch {
            expected: total,
            got: out.len() as u64,
        });
    }
    let got: [u8; 32] = Sha256::digest(&out).into();
    if got != digest {
        return Err(AgreementError::DigestMismatch);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    #[test]
    fn only_immediate_successor_advances() {
        use AgreementStateV1::*;
        assert!(Created.can_advance_to(OfferSent));
        assert!(!Created.can_advance_to(OfferValidated)); // no skipping
        assert!(!OfferValidated.can_advance_to(Created)); // no going back
        assert!(!OfferSent.can_advance_to(OfferSent)); // no staying
        assert_eq!(AgreementCommitted.next(), None);
        assert!(AgreementCommitted.is_committed());
        assert!(!CommitValidated.is_committed());
    }

    #[test]
    fn full_linear_walk_reaches_committed() {
        let mut s = AgreementStateV1::Created;
        while let Some(n) = s.next() {
            assert!(s.can_advance_to(n));
            s = n;
        }
        assert!(s.is_committed());
    }

    #[test]
    fn frozen_transcript_hash() {
        let h = transcript_hash(b"offer-bytes", b"ack-bytes", b"commit-bytes");
        assert_eq!(
            hex_bytes(&h),
            "65c3ce3d27eab1ea742c4bebd31a72a8d1e51dd704819b14dd0aef3c7c171bdf",
            "frozen transcript hash drift",
        );
    }

    #[test]
    fn transcript_frames_prevent_boundary_aliasing() {
        // Moving a byte across the offer/ack boundary must change the transcript
        // (length-framing defeats concatenation aliasing).
        assert_ne!(
            transcript_hash(b"ab", b"c", b"commit"),
            transcript_hash(b"a", b"bc", b"commit"),
        );
    }

    #[test]
    fn fragment_round_trip_is_order_independent() {
        let payload: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let mut frags = fragment_message(&payload).unwrap();
        assert!(frags.len() >= 3, "200KB splits into multiple 64KiB fragments");
        frags.reverse(); // arrival order scrambled
        assert_eq!(reassemble(frags).unwrap(), payload);
    }

    #[test]
    fn small_payload_is_single_fragment() {
        let frags = fragment_message(b"hello").unwrap();
        assert_eq!(frags.len(), 1);
        assert_eq!(reassemble(frags).unwrap(), b"hello");
    }

    #[test]
    fn missing_fragment_is_typed_failure() {
        let payload: Vec<u8> = (0..200_000u32).map(|i| i as u8).collect();
        let mut frags = fragment_message(&payload).unwrap();
        frags.pop(); // drop the last fragment
        assert!(matches!(
            reassemble(frags),
            Err(AgreementError::IndexGap { .. })
        ));
    }

    #[test]
    fn corrupted_fragment_fails_whole_message_digest() {
        let payload: Vec<u8> = (0..100u32).map(|i| i as u8).collect();
        let mut frags = fragment_message(&payload).unwrap();
        frags[0].fragment_payload[0] ^= 0xff; // flip a bit without touching digest
        assert_eq!(reassemble(frags), Err(AgreementError::DigestMismatch));
    }

    #[test]
    fn oversized_message_rejected() {
        // A vector at cap+1 is rejected without hashing the whole thing twice.
        let big = vec![0u8; MESSAGE_HARD_CAP + 1];
        assert_eq!(
            fragment_message(&big),
            Err(AgreementError::OversizedMessage {
                len: MESSAGE_HARD_CAP + 1,
                cap: MESSAGE_HARD_CAP
            })
        );
    }
}
