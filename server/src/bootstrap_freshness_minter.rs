//! `APEX-T4.2` chunk B: the per-boot bootstrap-sequence-and-root-chain
//! minter -- server-side counterpart to
//! `common::apex::bootstrap_freshness::BootstrapFreshnessLedgerV1`'s
//! client-side floor.
//!
//! ONE instance per server process, inserted alongside `ServerBootId`
//! (`server/src/lib.rs`) and never reset for the process's lifetime --
//! the sequence is monotone WITHIN THE BOOT, spanning every session's
//! manifests this boot issues, not a per-session counter (`T4.2`'s own
//! spec).

use common::apex::digest::ArtifactDigestV1;

#[derive(Debug)]
pub struct BootstrapFreshnessMinterV1 {
    next_sequence: u64,
    current_root: Option<ArtifactDigestV1>,
}

impl Default for BootstrapFreshnessMinterV1 {
    /// Sequence starts at 1 -- zero is reserved
    /// (`BootstrapFreshnessRejectionV1::SequenceZero`). No predecessor
    /// root: the first manifest this boot ever issues has none to chain
    /// from.
    fn default() -> Self { Self { next_sequence: 1, current_root: None } }
}

impl BootstrapFreshnessMinterV1 {
    /// The `(sequence, predecessor_root)` pair for the NEXT manifest this
    /// boot issues. Call BEFORE building that manifest's freshness tuple
    /// (`predecessor_root` must be embedded in it), then call
    /// [`Self::commit_v1`] with that manifest's own content root once it
    /// is fully assembled -- the two-step split exists because the root
    /// can only be computed AFTER the tuple this call produces is
    /// embedded in the manifest (the standard hash-chain construction: a
    /// block's hash covers its own prev-hash pointer).
    pub fn next_v1(&mut self) -> (u64, Option<ArtifactDigestV1>) {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        (sequence, self.current_root)
    }

    /// Commits the just-minted manifest's own content root as the new
    /// chain head. Must follow exactly one [`Self::next_v1`] call, with
    /// the root of the manifest that pair was embedded in -- skipping
    /// this (e.g. on an encode failure) leaves the chain head unmoved, so
    /// the NEXT `next_v1()` re-offers the SAME `predecessor_root`. That is
    /// correct, not a bug: a failed mint attempt burns a sequence number
    /// (gaps are fine -- `BootstrapFreshnessLedgerV1::admit_v1` only
    /// requires a STRICTLY GREATER sequence, never adjacency) but never
    /// enters the chain, because nothing was actually sent.
    pub fn commit_v1(&mut self, root: ArtifactDigestV1) { self.current_root = Some(root); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apex::digest::hash_artifact_bytes_v1;

    fn root(tag: u8) -> ArtifactDigestV1 { hash_artifact_bytes_v1(&[tag]).digest }

    #[test]
    fn the_first_mint_has_no_predecessor_and_sequence_one() {
        let mut minter = BootstrapFreshnessMinterV1::default();
        let (sequence, predecessor_root) = minter.next_v1();
        assert_eq!(sequence, 1);
        assert_eq!(predecessor_root, None);
    }

    /// The core chain property: each mint's predecessor is exactly the
    /// PREVIOUS mint's committed root, and sequence strictly increases.
    #[test]
    fn successive_mints_chain_and_advance_the_sequence() {
        let mut minter = BootstrapFreshnessMinterV1::default();

        let (seq1, pred1) = minter.next_v1();
        assert_eq!(pred1, None);
        minter.commit_v1(root(1));

        let (seq2, pred2) = minter.next_v1();
        assert_eq!(pred2, Some(root(1)));
        minter.commit_v1(root(2));

        let (seq3, pred3) = minter.next_v1();
        assert_eq!(pred3, Some(root(2)));

        assert!(seq1 < seq2 && seq2 < seq3, "sequence must strictly increase across mints");
    }

    /// A mint whose caller never commits (e.g. an encode failure) leaves
    /// the chain head unmoved -- the NEXT mint re-offers the SAME
    /// predecessor, and the burned sequence number is never reused.
    #[test]
    fn an_uncommitted_mint_leaves_the_chain_head_unmoved_and_burns_the_sequence() {
        let mut minter = BootstrapFreshnessMinterV1::default();
        minter.next_v1();
        minter.commit_v1(root(1));

        let (failed_seq, failed_pred) = minter.next_v1();
        assert_eq!(failed_pred, Some(root(1)));
        // Deliberately no commit_v1 here -- simulating an encode failure.

        let (next_seq, next_pred) = minter.next_v1();
        assert_ne!(next_seq, failed_seq, "the failed attempt's sequence must never be reissued");
        assert_eq!(next_pred, Some(root(1)), "an uncommitted attempt must not move the chain head");
    }
}
