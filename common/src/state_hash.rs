//! T0.53 (master build order; T0-004 packet, step 6): canonical domain-hash
//! interfaces — versioned per-domain roots plus one composite, never one
//! monolithic hash and never XOR-folding.
//!
//! THE RULES (packet):
//! - Every domain hashes with a DOMAIN-SEPARATION LABEL (e.g.
//!   `bastion/domain/jobs/v1/sha256`) — the label includes the algorithm so
//!   a future hash swap is a clean version bump.
//! - Hashes are over CANONICAL LOGICAL STATE, never memory layout. Wall
//!   time, pointers, thread ids, and queue-arrival timestamps are excluded
//!   from authoritative roots by construction (they never enter a
//!   [`DomainHasher`]).
//! - Rebuildable indexes (caches) get their own SEPARATE integrity root —
//!   a distinct TYPE here, so folding one into the durable composite is a
//!   compile error, not a review catch.
//!
//! DEVIATION (disclosed): the packet names BLAKE3; the workspace already
//! ships sha2 and adds no new dependency for this substrate. The labels
//! carry the algorithm, so adopting blake3 later is a per-domain version
//! bump, not a schema break.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A finalized authoritative domain root.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomainHash(pub [u8; 32]);

/// A finalized REBUILDABLE-INDEX root — deliberately a different type from
/// [`DomainHash`]: integrity roots are compared against fresh rebuilds,
/// never folded into the durable composite.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntegrityHash(pub [u8; 32]);

impl core::fmt::Display for DomainHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// An in-progress domain hash, bound to its domain-separation label at
/// construction (length-prefixed so label/content splits are unambiguous).
pub struct DomainHasher {
    inner: Sha256,
}

impl DomainHasher {
    /// `label` convention: `bastion/domain/<name>/v<N>/sha256`.
    pub fn new(label: &str) -> Self {
        let mut inner = Sha256::new();
        inner.update((label.len() as u64).to_le_bytes());
        inner.update(label.as_bytes());
        Self { inner }
    }

    /// Feed one canonical field (length-prefixed — field boundaries are
    /// part of the hash, so `"ab" + "c"` never equals `"a" + "bc"`).
    pub fn field(&mut self, bytes: &[u8]) -> &mut Self {
        self.inner.update((bytes.len() as u64).to_le_bytes());
        self.inner.update(bytes);
        self
    }

    pub fn finish(self) -> DomainHash { DomainHash(self.inner.finalize().into()) }

    pub fn finish_integrity(self) -> IntegrityHash {
        IntegrityHash(self.inner.finalize().into())
    }
}

/// T0.53: the per-phase state hash — versioned schema, per-domain roots,
/// one composite computed over the LABEL-SORTED domain list (insertion
/// order can never leak into the composite).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhaseStateHash {
    /// e.g. `bastion/phase-state-hash/v1`.
    pub hash_schema: String,
    pub tick: u64,
    /// The named phase this hash was captured at.
    pub phase: String,
    /// (domain label, root), sorted by label.
    pub domains: Vec<(String, DomainHash)>,
    pub composite: DomainHash,
}

impl PhaseStateHash {
    pub fn compose(
        hash_schema: impl Into<String>,
        tick: u64,
        phase: impl Into<String>,
        mut domains: Vec<(String, DomainHash)>,
    ) -> Self {
        domains.sort_by(|a, b| a.0.cmp(&b.0));
        let mut composite = DomainHasher::new("bastion/composite/v1/sha256");
        for (label, hash) in &domains {
            composite.field(label.as_bytes());
            composite.field(&hash.0);
        }
        Self {
            hash_schema: hash_schema.into(),
            tick,
            phase: phase.into(),
            domains,
            composite: composite.finish(),
        }
    }
}

#[cfg(test)]
mod t0_53_tests {
    use super::*;

    #[test]
    fn t0_53_labels_separate_and_fields_are_prefix_safe() {
        // Same bytes, different domain labels → different roots.
        let mut a = DomainHasher::new("bastion/domain/jobs/v1/sha256");
        a.field(b"payload");
        let mut b = DomainHasher::new("bastion/domain/terrain/v1/sha256");
        b.field(b"payload");
        assert_ne!(a.finish(), b.finish());
        // Field boundaries are hashed: "ab"+"c" != "a"+"bc".
        let mut x = DomainHasher::new("l");
        x.field(b"ab").field(b"c");
        let mut y = DomainHasher::new("l");
        y.field(b"a").field(b"bc");
        assert_ne!(x.finish(), y.finish());
    }

    #[test]
    fn t0_53_composite_is_insertion_order_free() {
        let d1 = ("bastion/domain/jobs/v1/sha256".to_string(), {
            let mut h = DomainHasher::new("bastion/domain/jobs/v1/sha256");
            h.field(b"j");
            h.finish()
        });
        let d2 = ("bastion/domain/terrain/v1/sha256".to_string(), {
            let mut h = DomainHasher::new("bastion/domain/terrain/v1/sha256");
            h.field(b"t");
            h.finish()
        });
        let forward =
            PhaseStateHash::compose("s", 1, "p", vec![d1.clone(), d2.clone()]);
        let reversed = PhaseStateHash::compose("s", 1, "p", vec![d2, d1]);
        assert_eq!(forward.composite, reversed.composite);
        assert_eq!(forward.domains, reversed.domains);
    }
}
