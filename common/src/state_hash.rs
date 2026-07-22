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

/// DET-ADD-008 (determinism audit): a STABLE `std::hash::Hasher` for
/// semantic u64 IDs that must survive a toolchain / library upgrade.
/// `std::hash::DefaultHasher` (SipHash) is explicitly NOT stable across Rust
/// versions, so item hashes / terrain-revision IDs computed with it could
/// silently change on an upgrade even though a single frozen executable
/// stays consistent. This backs the standard `Hasher` interface with the
/// SAME Sha256 primitive `DomainHasher` uses (a fixed standard), so it drops
/// into any existing `value.hash(&mut h)` site while yielding a stable u64.
/// It is a SEPARATE type from `DomainHasher` (no `finish()` collision on the
/// domain API). Determinism: a pure function of the fed bytes.
pub struct StableHasher(Sha256);

impl StableHasher {
    /// `label` domain-separates independent hash spaces (length-prefixed).
    pub fn new(label: &str) -> Self {
        let mut inner = Sha256::new();
        inner.update((label.len() as u64).to_le_bytes());
        inner.update(label.as_bytes());
        StableHasher(inner)
    }
}

impl std::hash::Hasher for StableHasher {
    fn write(&mut self, bytes: &[u8]) { self.0.update(bytes); }

    fn finish(&self) -> u64 {
        // Clone so finishing does not consume the in-progress digest (the
        // `Hasher` contract allows finish() to be called mid-stream).
        let digest = self.0.clone().finalize();
        u64::from_le_bytes(
            digest[..8]
                .try_into()
                .expect("a sha256 digest is 32 bytes, always >= 8"),
        )
    }
}

/// DET-ADD-008: stable u64 hash of a single `Hash` value (convenience over
/// [`StableHasher`]); use `StableHasher` directly for multi-field hashes.
pub fn stable_hash_u64<H: std::hash::Hash + ?Sized>(label: &str, value: &H) -> u64 {
    use std::hash::Hasher as _;
    let mut h = StableHasher::new(label);
    value.hash(&mut h);
    h.finish()
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

/// T0.58 (T0-004 packet, step 8 family): the versioned schema reference
/// every recorder/hash record family carries. Published schemas are
/// immutable; event-kind ids are never reused; unknown kinds are preserved
/// opaque by consumers.
///
/// Version discipline (the packet's rules): additive field = MINOR;
/// rename-with-transform = MINOR; meaning/units/identity change = MAJOR;
/// doc or transform bugfix = PATCH.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RecorderSchemaRef {
    /// Family ordinal (append-only registry; a family id is never reused).
    pub family: u16,
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl RecorderSchemaRef {
    /// Whether a reader built for `self` can consume a record of `other`:
    /// same family, same major, reader minor >= record minor not required
    /// (additive fields are skippable) — majors must match exactly.
    pub fn can_read(self, other: RecorderSchemaRef) -> bool {
        self.family == other.family && self.major == other.major
    }
}

#[cfg(test)]
mod t0_58_tests {
    use super::RecorderSchemaRef;

    #[test]
    fn t0_58_compat_is_family_and_major_bound() {
        let reader = RecorderSchemaRef {
            family: 3,
            major: 2,
            minor: 5,
            patch: 0,
        };
        let mut record = reader;
        record.minor = 9; // additive fields: readable (skippable)
        assert!(reader.can_read(record));
        record.major = 3; // meaning change: unreadable without a transform
        assert!(!reader.can_read(record));
        record.major = 2;
        record.family = 4; // families never alias
        assert!(!reader.can_read(record));
    }
}

/// T0.55 (T0-004 packet, step 7): the four state categories a Merkle
/// domain tree covers. Each category is its OWN root; the durable root is
/// the authoritative one, the rebuildable-index root is compared against a
/// fresh rebuild and never folded into durable.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DomainCategory {
    /// The authoritative durable data (rtsim save, jobs, inventories).
    Durable,
    /// The loaded projection (ECS mirror of durable state).
    LoadedProjection,
    /// The representation mapping (rtsim id ↔ ecs entity).
    RepresentationMapping,
    /// Rebuildable indexes (caches) — separate integrity root, per the
    /// type-separation rule.
    RebuildableIndex,
}

impl DomainCategory {
    fn label(self) -> &'static str {
        match self {
            DomainCategory::Durable => "bastion/category/durable/v1/sha256",
            DomainCategory::LoadedProjection => "bastion/category/loaded-projection/v1/sha256",
            DomainCategory::RepresentationMapping => {
                "bastion/category/representation-mapping/v1/sha256"
            },
            DomainCategory::RebuildableIndex => "bastion/category/rebuildable-index/v1/sha256",
        }
    }
}

/// T0.55: one Merkle leaf — a stable key (`npc/<stable_actor_id>`,
/// `site/<site_id>`) and its canonical content hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleLeaf {
    pub key: String,
    pub hash: DomainHash,
}

/// Compute a category root from its leaves — sorted by stable key so
/// insertion/iteration order can never affect the root (the packet's
/// canonical-order rule).
pub fn category_root(category: DomainCategory, mut leaves: Vec<MerkleLeaf>) -> DomainHash {
    leaves.sort_by(|a, b| a.key.cmp(&b.key));
    let mut hasher = DomainHasher::new(category.label());
    for leaf in &leaves {
        hasher.field(leaf.key.as_bytes());
        hasher.field(&leaf.hash.0);
    }
    hasher.finish()
}

/// T0.61 (T0-004 packet, step 7): the final-state certificate — a
/// TEST/REPLAY artifact captured at the final authoritative phase, NOT a
/// save replacement. Each domain defines its own canonical leaves; there is
/// no universal serialization across domains. The rebuildable-index
/// integrity root stays SEPARATE from the durable composite.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalStateCertificate {
    /// e.g. `bastion/final-state-certificate/v1`.
    pub schema: String,
    pub world_seed: u32,
    pub tick: u64,
    /// The authoritative composite over Durable + LoadedProjection +
    /// RepresentationMapping (label-sorted via PhaseStateHash).
    pub durable_composite: DomainHash,
    /// The rebuildable-index root — compared against a fresh rebuild,
    /// distinct from the durable composite (a different field, not folded).
    pub rebuildable_integrity: IntegrityHash,
    /// E1 (engine-emission, determinism tool suite): the DIAGNOSTIC per-domain
    /// breakdown — `(domain label, root)` pairs, stored LABEL-SORTED so
    /// insertion order can never leak in (the same canonical-order rule as
    /// [`PhaseStateHash`]/[`category_root`]). This attributes WHICH domain
    /// moved when the composite changes, so the expected-change classifier can
    /// enforce `allowed_domains`/`forbidden_domains` (DECLARED_SCOPE_EXCEEDED).
    /// It is DIAGNOSTIC ONLY: excluded from [`Self::authoritative_matches`], so
    /// the durable composite stays the sole equivalence surface and adding or
    /// refining this breakdown never perturbs an existing `durable_composite`.
    /// `#[serde(default)]` keeps it additive — a v1 certificate JSON without
    /// the field still deserializes (an empty breakdown).
    #[serde(default)]
    pub domain_hashes: Vec<(String, DomainHash)>,
}

impl FinalStateCertificate {
    /// Build a certificate, storing `domain_hashes` LABEL-SORTED so insertion
    /// order can never affect the diagnostic breakdown. `durable_composite` is
    /// passed in already-computed (over whatever canonical leaves the scenario
    /// defines) and is NOT recomputed from `domain_hashes` — the two are
    /// independent by design so this additive breakdown cannot shift a frozen
    /// composite baseline.
    pub fn new(
        schema: impl Into<String>,
        world_seed: u32,
        tick: u64,
        durable_composite: DomainHash,
        rebuildable_integrity: IntegrityHash,
        mut domain_hashes: Vec<(String, DomainHash)>,
    ) -> Self {
        domain_hashes.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            schema: schema.into(),
            world_seed,
            tick,
            durable_composite,
            rebuildable_integrity,
            domain_hashes,
        }
    }

    /// Whether another certificate matches on the AUTHORITATIVE surface
    /// (seed, tick, durable composite) — the replay-equivalence check. The
    /// rebuildable integrity is compared separately and only against a
    /// fresh rebuild, never used to reject a replay. `domain_hashes` is
    /// diagnostic and deliberately NOT compared here.
    pub fn authoritative_matches(&self, other: &FinalStateCertificate) -> bool {
        self.world_seed == other.world_seed
            && self.tick == other.tick
            && self.durable_composite == other.durable_composite
    }

    /// The diagnostic breakdown as an ordered slice of `(label, root)` — the
    /// classifier reads this to see which domain roots moved between two runs.
    pub fn domain_hashes(&self) -> &[(String, DomainHash)] { &self.domain_hashes }
}

#[cfg(test)]
mod t0_55_tests {
    use super::*;

    fn leaf(key: &str, byte: u8) -> MerkleLeaf {
        let mut hasher = DomainHasher::new("bastion/domain/npc/v1/sha256");
        hasher.field(&[byte]);
        MerkleLeaf {
            key: key.to_string(),
            hash: hasher.finish(),
        }
    }

    #[test]
    fn t0_55_category_root_is_key_order_free() {
        let a = category_root(DomainCategory::Durable, vec![
            leaf("npc/2", 20),
            leaf("npc/1", 10),
            leaf("site/1", 30),
        ]);
        let b = category_root(DomainCategory::Durable, vec![
            leaf("site/1", 30),
            leaf("npc/1", 10),
            leaf("npc/2", 20),
        ]);
        assert_eq!(a, b, "leaf insertion order must not affect the category root");
        // Different category label → different root for the same leaves.
        let c = category_root(DomainCategory::LoadedProjection, vec![
            leaf("npc/1", 10),
            leaf("npc/2", 20),
            leaf("site/1", 30),
        ]);
        assert_ne!(a, c);
    }

    #[test]
    fn t0_61_certificate_authoritative_match_excludes_integrity() {
        let cert = |durable: u8, integrity: u8| FinalStateCertificate {
            schema: "bastion/final-state-certificate/v1".to_string(),
            world_seed: 7,
            tick: 100,
            durable_composite: DomainHash([durable; 32]),
            rebuildable_integrity: IntegrityHash([integrity; 32]),
            domain_hashes: Vec::new(),
        };
        // Same authoritative surface, DIFFERENT rebuildable integrity →
        // still an authoritative match (indexes are rebuilt, not replayed).
        assert!(cert(1, 9).authoritative_matches(&cert(1, 5)));
        // Different durable composite → no match.
        assert!(!cert(1, 9).authoritative_matches(&cert(2, 9)));
    }

    #[test]
    fn e1_domain_hashes_are_sorted_and_diagnostic_only() {
        let dh = |b: u8| DomainHash([b; 32]);
        // `new` stores the breakdown LABEL-SORTED regardless of insertion order.
        let forward = FinalStateCertificate::new(
            "bastion/final-state-certificate/v1",
            7,
            100,
            dh(1),
            IntegrityHash([0; 32]),
            vec![
                ("bastion/domain/terrain/v1/sha256".to_string(), dh(30)),
                ("bastion/domain/jobs/v1/sha256".to_string(), dh(20)),
            ],
        );
        let reversed = FinalStateCertificate::new(
            "bastion/final-state-certificate/v1",
            7,
            100,
            dh(1),
            IntegrityHash([0; 32]),
            vec![
                ("bastion/domain/jobs/v1/sha256".to_string(), dh(20)),
                ("bastion/domain/terrain/v1/sha256".to_string(), dh(30)),
            ],
        );
        assert_eq!(
            forward.domain_hashes(),
            reversed.domain_hashes(),
            "insertion order must not affect the stored breakdown"
        );
        assert_eq!(forward.domain_hashes()[0].0, "bastion/domain/jobs/v1/sha256");

        // The breakdown is DIAGNOSTIC: two certs with an IDENTICAL durable
        // composite but DIFFERENT domain_hashes still match authoritatively —
        // the composite is the sole equivalence surface.
        let no_breakdown = FinalStateCertificate::new(
            "bastion/final-state-certificate/v1",
            7,
            100,
            dh(1),
            IntegrityHash([0; 32]),
            Vec::new(),
        );
        assert!(forward.authoritative_matches(&no_breakdown));

        // Additive-schema: a v1 JSON WITHOUT the field deserializes (empty
        // breakdown), and a round-trip with the field is stable.
        let v1_json = r#"{"schema":"bastion/final-state-certificate/v1","world_seed":7,"tick":100,"durable_composite":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"rebuildable_integrity":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#;
        let legacy: FinalStateCertificate =
            serde_json::from_str(v1_json).expect("v1 cert without domain_hashes must deserialize");
        assert!(legacy.domain_hashes().is_empty());
        let round: FinalStateCertificate =
            serde_json::from_str(&serde_json::to_string(&forward).unwrap()).unwrap();
        assert_eq!(round.domain_hashes(), forward.domain_hashes());
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

#[cfg(test)]
mod det_add_008_tests {
    use super::*;

    #[test]
    fn stable_hash_u64_is_deterministic_label_separated_and_value_sensitive() {
        // Deterministic: same (label, value) → same u64, every call.
        assert_eq!(
            stable_hash_u64("l", &"hello"),
            stable_hash_u64("l", &"hello")
        );
        // Label domain-separates: identical value under a different label
        // must (with overwhelming probability) differ.
        assert_ne!(
            stable_hash_u64("a", &"hello"),
            stable_hash_u64("b", &"hello")
        );
        // Value-sensitive.
        assert_ne!(
            stable_hash_u64("l", &"hello"),
            stable_hash_u64("l", &"world")
        );
        // Multi-field via StableHasher directly matches the same fields fed
        // as a tuple through the convenience fn (the terrain-revision shape).
        use std::hash::{Hash, Hasher};
        let mut h = StableHasher::new("m");
        1u64.hash(&mut h);
        2u64.hash(&mut h);
        assert_eq!(h.finish(), stable_hash_u64("m", &(1u64, 2u64)));
    }
}
