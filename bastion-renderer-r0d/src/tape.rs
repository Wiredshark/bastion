//! BUILD-007A10.6 (part 1) — semantic checkpoint tapes and first divergence
//! (design §14).
//!
//! A canonical semantic tape is the closed-total-order sequence of typed
//! records committed by every authority, hashed into an RFC 9162 Merkle tree so
//! roots reproduce independently and divergence localizes by binary search.
//!
//! - §14.1 closed domain-rank registry (16 frozen ranks); unknown ranks
//!   rejected.
//! - §14.2 total order `(tick, frame_or_zero, domain_rank, authority_rank,
//!   owner_digest, leaf_kind_rank, local_ordinal)`; duplicate total keys
//!   rejected.
//! - §14.3 RFC 9162 tree: `leaf = SHA256(0x00 || record)`, `node = SHA256(0x01
//!   || left || right)`; chunks of ≤256 records.
//! - §14.6 first-divergence: compare final roots, binary-search chunk roots,
//!   then records in the first differing chunk.

use sha2::{Digest, Sha256};

/// Frozen semantic domain ranks (§14.1). The numeric rank is part of the total
/// order and the Merkle identity, so the mapping is closed and versioned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DomainRank {
    Bootstrap = 1,
    TickInput = 2,
    ServerSemantic = 3,
    ReplicationGeneration = 4,
    ClientApplied = 5,
    Agreement = 6,
    SceneProjection = 7,
    FigureKey = 8,
    AssetLifecycle = 9,
    Readiness = 10,
    CameraFrame = 11,
    RenderSelection = 12,
    DrawPassShadowRain = 13,
    StructuralCapture = 14,
    ArtifactPublication = 15,
    ShutdownVerdict = 16,
}

impl DomainRank {
    /// Resolve a numeric rank against the closed registry (§14.1). An unknown
    /// required rank is rejected, never silently accepted.
    pub fn from_rank(rank: u16) -> Option<DomainRank> {
        use DomainRank::*;
        Some(match rank {
            1 => Bootstrap,
            2 => TickInput,
            3 => ServerSemantic,
            4 => ReplicationGeneration,
            5 => ClientApplied,
            6 => Agreement,
            7 => SceneProjection,
            8 => FigureKey,
            9 => AssetLifecycle,
            10 => Readiness,
            11 => CameraFrame,
            12 => RenderSelection,
            13 => DrawPassShadowRain,
            14 => StructuralCapture,
            15 => ArtifactPublication,
            16 => ShutdownVerdict,
            _ => return None,
        })
    }
}

/// The closed total-order key of a tape record (§14.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TapeKeyV1 {
    pub simulation_tick: u64,
    pub render_frame_or_zero: u64,
    pub domain_rank: u16,
    pub authority_rank: u16,
    pub owner_digest: [u8; 32],
    pub leaf_kind_rank: u16,
    pub local_ordinal: u64,
}

/// One typed semantic record: its total-order key plus canonical payload bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapeRecordV1 {
    pub key: TapeKeyV1,
    pub payload: Vec<u8>,
}

impl TapeRecordV1 {
    /// Canonical record bytes: the total-order key fields (frozen order) then
    /// the length-framed payload. This is the exact preimage the Merkle leaf
    /// hashes.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let k = &self.key;
        let mut b = Vec::with_capacity(64 + self.payload.len());
        b.extend_from_slice(&k.simulation_tick.to_le_bytes());
        b.extend_from_slice(&k.render_frame_or_zero.to_le_bytes());
        b.extend_from_slice(&k.domain_rank.to_le_bytes());
        b.extend_from_slice(&k.authority_rank.to_le_bytes());
        b.extend_from_slice(&k.owner_digest);
        b.extend_from_slice(&k.leaf_kind_rank.to_le_bytes());
        b.extend_from_slice(&k.local_ordinal.to_le_bytes());
        b.extend_from_slice(&(self.payload.len() as u64).to_le_bytes());
        b.extend_from_slice(&self.payload);
        b
    }
}

/// Tape finalization failures (§14.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TapeError {
    /// Two records share a total key (§14.2).
    DuplicateKey { key: TapeKeyV1 },
    /// A record used a domain rank outside the closed registry (§14.1).
    UnknownDomainRank { rank: u16 },
}

/// RFC 9162 leaf hash: `SHA256(0x00 || record_bytes)`.
#[must_use]
pub fn leaf_hash(record_bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([0x00]);
    h.update(record_bytes);
    h.finalize().into()
}

fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([0x01]);
    h.update(left);
    h.update(right);
    h.finalize().into()
}

/// RFC 9162 Merkle Tree Hash over the given leaf preimages (§14.3). Empty input
/// hashes the empty string; a single leaf uses the leaf hash; otherwise the
/// tree splits at the largest power of two strictly below the count.
#[must_use]
pub fn merkle_root(records: &[Vec<u8>]) -> [u8; 32] {
    match records.len() {
        0 => Sha256::digest([]).into(),
        1 => leaf_hash(&records[0]),
        n => {
            let mut k = 1;
            while k * 2 < n {
                k *= 2;
            }
            let left = merkle_root(&records[..k]);
            let right = merkle_root(&records[k..]);
            node_hash(&left, &right)
        },
    }
}

/// Max records per chunk (§14.3).
pub const CHUNK_MAX: usize = 256;

/// A finalized canonical tape: sorted records, chunk roots, and final root.
#[derive(Clone, Debug)]
pub struct FinalizedTapeV1 {
    records: Vec<TapeRecordV1>,
    chunk_roots: Vec<[u8; 32]>,
    final_root: [u8; 32],
}

impl FinalizedTapeV1 {
    /// Finalize a set of locally-gathered records (§14.2/§14.3): validate
    /// ranks, sort by the total key, reject duplicate keys, then hash into
    /// chunk roots (≤256 records each) and the final RFC 9162 root over all
    /// leaves.
    pub fn finalize(mut records: Vec<TapeRecordV1>) -> Result<Self, TapeError> {
        for r in &records {
            if DomainRank::from_rank(r.key.domain_rank).is_none() {
                return Err(TapeError::UnknownDomainRank {
                    rank: r.key.domain_rank,
                });
            }
        }
        records.sort_by(|a, b| a.key.cmp(&b.key));
        for w in records.windows(2) {
            if w[0].key == w[1].key {
                return Err(TapeError::DuplicateKey { key: w[0].key });
            }
        }
        let leaves: Vec<Vec<u8>> = records.iter().map(TapeRecordV1::canonical_bytes).collect();
        let chunk_roots: Vec<[u8; 32]> = leaves.chunks(CHUNK_MAX).map(|c| merkle_root(c)).collect();
        let final_root = merkle_root(&leaves);
        Ok(Self {
            records,
            chunk_roots,
            final_root,
        })
    }

    #[must_use]
    pub fn final_root(&self) -> [u8; 32] { self.final_root }

    #[must_use]
    pub fn chunk_roots(&self) -> &[[u8; 32]] { &self.chunk_roots }

    #[must_use]
    pub fn len(&self) -> usize { self.records.len() }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.records.is_empty() }
}

/// First-divergence report (§14.6): the exact record position and context where
/// two tapes first differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererFirstDivergenceV1 {
    pub chunk_index: usize,
    pub record_index_in_chunk: usize,
    pub key: TapeKeyV1,
    pub expected_record: Option<Vec<u8>>,
    pub actual_record: Option<Vec<u8>>,
}

/// Locate the first divergence between two finalized tapes (§14.6): equal final
/// roots means identical; otherwise scan chunk roots for the first differing
/// chunk, then the first differing record inside it.
#[must_use]
pub fn first_divergence(
    expected: &FinalizedTapeV1,
    actual: &FinalizedTapeV1,
) -> Option<RendererFirstDivergenceV1> {
    if expected.final_root == actual.final_root {
        return None;
    }
    let nchunks = expected.chunk_roots.len().max(actual.chunk_roots.len());
    for ci in 0..nchunks {
        let er = expected.chunk_roots.get(ci);
        let ar = actual.chunk_roots.get(ci);
        if er == ar {
            continue;
        }
        // First differing chunk: compare its records.
        let start = ci * CHUNK_MAX;
        for i in 0..CHUNK_MAX {
            let e = expected.records.get(start + i);
            let a = actual.records.get(start + i);
            match (e, a) {
                (Some(er), Some(ar)) if er == ar => continue,
                (None, None) => break,
                _ => {
                    let key = e
                        .map(|r| r.key)
                        .or_else(|| a.map(|r| r.key))
                        .expect("one side present");
                    return Some(RendererFirstDivergenceV1 {
                        chunk_index: ci,
                        record_index_in_chunk: i,
                        key,
                        expected_record: e.map(TapeRecordV1::canonical_bytes),
                        actual_record: a.map(TapeRecordV1::canonical_bytes),
                    });
                },
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    #[test]
    fn domain_registry_is_closed() {
        assert_eq!(DomainRank::from_rank(1), Some(DomainRank::Bootstrap));
        assert_eq!(DomainRank::from_rank(16), Some(DomainRank::ShutdownVerdict));
        assert_eq!(DomainRank::from_rank(0), None);
        assert_eq!(DomainRank::from_rank(17), None);
    }

    #[test]
    fn rfc9162_root_matches_manual_construction() {
        // 3 leaves: MTH = node(node(leaf0,leaf1), leaf2) per the largest-power-of-2
        // split (k=2).
        let recs = vec![vec![0xa0u8], vec![0xa1u8], vec![0xa2u8]];
        let l0 = leaf_hash(&recs[0]);
        let l1 = leaf_hash(&recs[1]);
        let l2 = leaf_hash(&recs[2]);
        let expected = node_hash(&node_hash(&l0, &l1), &l2);
        assert_eq!(merkle_root(&recs), expected);
    }

    #[test]
    fn empty_and_single_roots() {
        assert_eq!(merkle_root(&[]), <[u8; 32]>::from(Sha256::digest([])));
        assert_eq!(merkle_root(&[vec![0x00]]), leaf_hash(&[0x00]));
    }

    fn rec(tick: u64, domain: u16, ord: u64, payload: u8) -> TapeRecordV1 {
        TapeRecordV1 {
            key: TapeKeyV1 {
                simulation_tick: tick,
                render_frame_or_zero: 0,
                domain_rank: domain,
                authority_rank: 0,
                owner_digest: [domain as u8; 32],
                leaf_kind_rank: 0,
                local_ordinal: ord,
            },
            payload: vec![payload],
        }
    }

    #[test]
    fn finalize_sorts_and_is_producer_order_independent() {
        let a = FinalizedTapeV1::finalize(vec![rec(2, 3, 0, 9), rec(1, 3, 0, 8), rec(1, 2, 0, 7)])
            .unwrap();
        let b = FinalizedTapeV1::finalize(vec![rec(1, 2, 0, 7), rec(2, 3, 0, 9), rec(1, 3, 0, 8)])
            .unwrap();
        assert_eq!(a.final_root(), b.final_root());
    }

    #[test]
    fn duplicate_total_key_rejected() {
        let e = FinalizedTapeV1::finalize(vec![rec(1, 3, 5, 1), rec(1, 3, 5, 2)]).unwrap_err();
        assert!(matches!(e, TapeError::DuplicateKey { .. }));
    }

    #[test]
    fn unknown_domain_rank_rejected() {
        assert_eq!(
            FinalizedTapeV1::finalize(vec![rec(1, 99, 0, 1)]).unwrap_err(),
            TapeError::UnknownDomainRank { rank: 99 }
        );
    }

    #[test]
    fn identical_tapes_have_no_divergence() {
        let a = FinalizedTapeV1::finalize(vec![rec(1, 2, 0, 1), rec(2, 3, 0, 2)]).unwrap();
        let b = FinalizedTapeV1::finalize(vec![rec(1, 2, 0, 1), rec(2, 3, 0, 2)]).unwrap();
        assert_eq!(first_divergence(&a, &b), None);
    }

    #[test]
    fn first_divergence_localizes_the_differing_record() {
        let a = FinalizedTapeV1::finalize(vec![rec(1, 2, 0, 1), rec(2, 3, 0, 2), rec(3, 3, 0, 3)])
            .unwrap();
        // Second record payload differs (9 vs 2).
        let b = FinalizedTapeV1::finalize(vec![rec(1, 2, 0, 1), rec(2, 3, 0, 9), rec(3, 3, 0, 3)])
            .unwrap();
        let d = first_divergence(&a, &b).expect("tapes differ");
        assert_eq!(d.record_index_in_chunk, 1);
        assert_eq!(d.key.simulation_tick, 2);
        assert_ne!(d.expected_record, d.actual_record);
    }

    #[test]
    fn frozen_tape_root() {
        let tape = FinalizedTapeV1::finalize(vec![rec(1, 2, 0, 1), rec(2, 3, 0, 2)]).unwrap();
        assert_eq!(
            hex_bytes(&tape.final_root()),
            "20a4871973688ef190d6f37f351c3dc52c66a32680aaa4e8191a45f8d067928a",
            "frozen tape root drift",
        );
    }
}
