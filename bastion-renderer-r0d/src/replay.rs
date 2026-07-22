//! BUILD-007A10.6 (part 2) — paired isolated replay and rollback proof (design
//! §16). Self-contained core: the exact-version replay identity (§16.1) and the
//! 256-tick hash/index checkpoints (§16.3). The live A/B run orchestration
//! (§16.4) and Git-blob rollback (§16.6) are harness/integration surfaces.

use sha2::{Digest, Sha256};

/// The exact-version replay identity (§16.1). V1 requires byte-exact equality of
/// every field; there is no compatibility range and no silent defaulting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayVersionIdentityV1 {
    pub source_tree_sha256: [u8; 32],
    pub executable_digest: [u8; 32],
    pub asset_package_root: [u8; 32],
    pub feature_capability_tier: [u8; 32],
    pub contract_oracle_schema: [u8; 32],
    pub bootstrap_manifest_digest: [u8; 32],
}

/// Which exact-version field mismatched (§16.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayVersionField {
    SourceTree,
    ExecutableDigest,
    AssetPackageRoot,
    FeatureCapabilityTier,
    ContractOracleSchema,
    BootstrapManifest,
}

/// `ReplayVersionMismatch` (§16.1): the first field (frozen order) that differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayVersionMismatch {
    pub field: ReplayVersionField,
}

impl ReplayVersionIdentityV1 {
    /// Verify a replay candidate against the recorded identity (§16.1). Any
    /// difference is a terminal `ReplayVersionMismatch` — cross-version replay is
    /// explicitly unsupported in V1.
    pub fn verify_exact(&self, candidate: &ReplayVersionIdentityV1) -> Result<(), ReplayVersionMismatch> {
        use ReplayVersionField::*;
        let checks = [
            (SourceTree, self.source_tree_sha256, candidate.source_tree_sha256),
            (ExecutableDigest, self.executable_digest, candidate.executable_digest),
            (AssetPackageRoot, self.asset_package_root, candidate.asset_package_root),
            (FeatureCapabilityTier, self.feature_capability_tier, candidate.feature_capability_tier),
            (ContractOracleSchema, self.contract_oracle_schema, candidate.contract_oracle_schema),
            (BootstrapManifest, self.bootstrap_manifest_digest, candidate.bootstrap_manifest_digest),
        ];
        for (field, a, b) in checks {
            if a != b {
                return Err(ReplayVersionMismatch { field });
            }
        }
        Ok(())
    }

    /// Domain-separated digest of the whole replay identity.
    #[must_use]
    pub fn identity_digest(&self) -> [u8; 32] {
        let mut p = Vec::with_capacity(32 * 6);
        for f in [
            &self.source_tree_sha256,
            &self.executable_digest,
            &self.asset_package_root,
            &self.feature_capability_tier,
            &self.contract_oracle_schema,
            &self.bootstrap_manifest_digest,
        ] {
            p.extend_from_slice(f);
        }
        crate::domain_hash("bastion/r0d/replay-identity", 1, 0, &p)
    }
}

/// Checkpoint cadence (§16.3): a hash/index checkpoint every 256 simulation ticks.
pub const CHECKPOINT_INTERVAL: u64 = 256;

/// A hash/index checkpoint (§16.3). V1 does NOT serialize whole-engine restore
/// snapshots; replay re-runs from tick zero to the checkpoint and verifies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointV1 {
    pub input_tape_offset: u64,
    pub semantic_tape_offset: u64,
    pub simulation_tick: u64,
    pub state_root: [u8; 32],
    pub record_chunk_root: [u8; 32],
}

impl CheckpointV1 {
    /// Domain-separated checkpoint digest for the checkpoint index.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut p = Vec::with_capacity(24 + 64);
        p.extend_from_slice(&self.input_tape_offset.to_le_bytes());
        p.extend_from_slice(&self.semantic_tape_offset.to_le_bytes());
        p.extend_from_slice(&self.simulation_tick.to_le_bytes());
        p.extend_from_slice(&self.state_root);
        p.extend_from_slice(&self.record_chunk_root);
        crate::domain_hash("bastion/r0d/checkpoint", 1, 0, &p)
    }
}

/// A checkpoint should be written at tick 0, every 256 ticks, and at phase
/// boundaries (§16.3). This models the cadence rule (phase boundaries are
/// signalled by the caller).
#[must_use]
pub fn checkpoint_due(tick: u64, phase_boundary: bool) -> bool {
    phase_boundary || tick % CHECKPOINT_INTERVAL == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ReplayVersionIdentityV1 {
        ReplayVersionIdentityV1 {
            source_tree_sha256: [1; 32],
            executable_digest: [2; 32],
            asset_package_root: [3; 32],
            feature_capability_tier: [4; 32],
            contract_oracle_schema: [5; 32],
            bootstrap_manifest_digest: [6; 32],
        }
    }

    #[test]
    fn exact_version_matches_itself() {
        assert!(identity().verify_exact(&identity()).is_ok());
    }

    #[test]
    fn any_field_drift_is_typed_mismatch() {
        let mut c = identity();
        c.executable_digest = [0xff; 32];
        assert_eq!(
            identity().verify_exact(&c),
            Err(ReplayVersionMismatch { field: ReplayVersionField::ExecutableDigest })
        );
    }

    #[test]
    fn first_differing_field_wins_in_frozen_order() {
        let mut c = identity();
        c.source_tree_sha256 = [0xff; 32];
        c.contract_oracle_schema = [0xff; 32];
        assert_eq!(
            identity().verify_exact(&c).unwrap_err().field,
            ReplayVersionField::SourceTree
        );
    }

    #[test]
    fn checkpoint_cadence() {
        assert!(checkpoint_due(0, false));
        assert!(checkpoint_due(256, false));
        assert!(!checkpoint_due(255, false));
        assert!(checkpoint_due(100, true)); // phase boundary forces one
    }

    #[test]
    fn checkpoint_digest_is_content_sensitive() {
        let a = CheckpointV1 { input_tape_offset: 10, semantic_tape_offset: 20, simulation_tick: 256, state_root: [7; 32], record_chunk_root: [8; 32] };
        let mut b = a;
        b.state_root = [9; 32];
        assert_ne!(a.digest(), b.digest());
    }
}
