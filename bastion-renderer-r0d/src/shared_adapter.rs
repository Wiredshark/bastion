//! BUILD-007A10.10 — shared determinism-substrate adapters (design §3A).
//!
//! The renderer program REUSES the engine's shared substrate
//! (`common::state_hash`, `content_manifest`, `async_work`, `causal_record`,
//! `run_equivalence`, `feature_protocol`) rather than creating independent
//! equivalents. This module is the adapter layer:
//!
//! - §3A.2 `RendererR0DAdmissionV2`: composition over `FinalStateCertificate` +
//!   `ContentManifest` + eight renderer domain roots. Renderer roots are
//!   compared exactly but NEVER inserted into the authoritative
//!   `durable_composite` (rule 1/2) — enforced by construction: this type holds
//!   the certificate by value and offers no mutation of it.
//! - §3A.3 the hash/identity adapter: only VALIDATED canonical bytes may become
//!   a renderer `DomainHash` — the function signature accepts
//!   `ValidatedCanonicalBytesV1`, not raw bytes, so the "validate → canonical
//!   CBOR → DomainHasher" pipeline is type-enforced. `stable_hash_u64` over
//!   derived `Hash` impls is NOT used for renderer identity.
//! - §3A.5 `RendererAsyncOwnerTableV1`: full owner digests → collision-checked
//!   contiguous `u64` ordinals (starting at 1) feeding
//!   `AsyncOwnerKey.stable_owner`. No digest truncation as identity.
//! - §3A.6 `RendererCausalDigestV1` full causal authority + a collision-checked
//!   `CausalId` alias bijection; plus the equivalence gate: shared
//!   `check_equivalence` with a ZERO-tolerance guard for R0D-required kinds,
//!   AND exact renderer tape/structural root equality.
//! - §3A.7 staged feature-protocol fitness rule for `renderer-r0d`. The
//!   append-only `AuthorityDomain::RendererPresentation` / `ClockDomain::
//!   RenderFrame` enum additions edit the SHARED `common/src/feature_protocol.rs`
//!   and per the design "require shared-engine review before implementation" —
//!   they are NOT made here. The staged rule enforces everything expressible in
//!   the current vocabulary: a renderer feature declares NO existing write
//!   authority (Terrain/Inventory/Ecs/Rtsim/JobBoardCoordination/Persistence),
//!   never lists `Wall`, and must record causal.

use std::collections::BTreeMap;

use common::async_work::{AsyncOwnerKey, AsyncPurpose};
use common::causal_record::CausalId;
use common::content_manifest::ContentManifest;
use common::feature_protocol::{AuthorityDomain, ClockDomain, FeatureProtocolDecl};
use common::run_equivalence::{
    EquivalenceTolerance, EquivalenceVerdict, RunSummary, check_equivalence,
};
use common::state_hash::{DomainHash, DomainHasher, FinalStateCertificate, RecorderSchemaRef};

use crate::cbor::ValidatedCanonicalBytesV1;

// ----------------------------------------------------------------------------
// §3A.3 hash/identity adapter
// ----------------------------------------------------------------------------

/// The §3A.3 identity procedure, type-enforced: typed value → strict validation
/// → Core Deterministic CBOR → `DomainHasher(label)` → full `DomainHash`.
/// Accepting only [`ValidatedCanonicalBytesV1`] makes "feed only validated
/// canonical bytes; retain domain labels" a compile-time property.
#[must_use]
pub fn renderer_domain_hash(label: &str, canonical: &ValidatedCanonicalBytesV1) -> DomainHash {
    let mut h = DomainHasher::new(label);
    h.field(canonical.as_bytes());
    h.finish()
}

// ----------------------------------------------------------------------------
// §3A.2 renderer admission composition
// ----------------------------------------------------------------------------

/// Typed schema-compatibility failure (§3A.2 rule 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchemaMismatch {
    pub expected: RecorderSchemaRef,
    pub actual: RecorderSchemaRef,
}

/// Run-030 composed admission (§3A.2). `authoritative.durable_composite`
/// remains owned by the simulation determinism program; the eight renderer
/// roots live beside it and are compared exactly, never folded in.
#[derive(Clone, Debug)]
pub struct RendererR0DAdmissionV2 {
    pub schema: RecorderSchemaRef,
    pub authoritative: FinalStateCertificate,
    pub content: ContentManifest,
    pub renderer_extract_root: DomainHash,
    pub replication_projection_root: DomainHash,
    pub canonical_selection_root: DomainHash,
    pub asset_package_root: DomainHash,
    pub structural_visual_root: DomainHash,
    pub artifact_index_root: DomainHash,
    pub causal_tape_root: DomainHash,
    pub environment_policy_digest: DomainHash,
}

impl RendererR0DAdmissionV2 {
    /// §3A.2 rule 5: any schema-family or major-version mismatch is typed and
    /// terminal (delegates to the shared `RecorderSchemaRef::can_read`).
    pub fn check_schema(&self, reader: RecorderSchemaRef) -> Result<(), SchemaMismatch> {
        if reader.can_read(self.schema) {
            Ok(())
        } else {
            Err(SchemaMismatch { expected: reader, actual: self.schema })
        }
    }

    /// Exact comparison of the renderer surface (§3A.2 rule 2): authoritative
    /// certificate equality via the SHARED authoritative_matches (seed/tick/
    /// durable composite), then byte-exact equality of every renderer root.
    /// Renderer mismatches are never hidden by any tolerance.
    #[must_use]
    pub fn matches_exactly(&self, other: &RendererR0DAdmissionV2) -> bool {
        self.schema == other.schema
            && self.authoritative.authoritative_matches(&other.authoritative)
            && self.renderer_extract_root == other.renderer_extract_root
            && self.replication_projection_root == other.replication_projection_root
            && self.canonical_selection_root == other.canonical_selection_root
            && self.asset_package_root == other.asset_package_root
            && self.structural_visual_root == other.structural_visual_root
            && self.artifact_index_root == other.artifact_index_root
            && self.causal_tape_root == other.causal_tape_root
            && self.environment_policy_digest == other.environment_policy_digest
    }

    /// Admission digest over the composition (frozen field order), via the
    /// shared `DomainHasher`. The durable composite participates as an OPAQUE
    /// field value — this digest is a new presentation-side root, never written
    /// back into authoritative state.
    #[must_use]
    pub fn admission_digest(&self) -> DomainHash {
        let mut h = DomainHasher::new("bastion/r0d/admission-v2/sha256");
        h.field(&self.schema.family.to_le_bytes());
        h.field(&self.schema.major.to_le_bytes());
        h.field(&self.schema.minor.to_le_bytes());
        h.field(&self.schema.patch.to_le_bytes());
        h.field(&self.authoritative.world_seed.to_le_bytes());
        h.field(&self.authoritative.tick.to_le_bytes());
        h.field(&self.authoritative.durable_composite.0);
        for root in [
            &self.renderer_extract_root,
            &self.replication_projection_root,
            &self.canonical_selection_root,
            &self.asset_package_root,
            &self.structural_visual_root,
            &self.artifact_index_root,
            &self.causal_tape_root,
            &self.environment_policy_digest,
        ] {
            h.field(&root.0);
        }
        h.finish()
    }
}

// ----------------------------------------------------------------------------
// §3A.5 async owner table
// ----------------------------------------------------------------------------

/// Owner-table failures (§3A.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnerTableError {
    DuplicateOwnerDigest { digest: [u8; 32] },
}

/// The frozen full-digest ↔ owner-ordinal bijection (§3A.5): sort full 32-byte
/// digests, reject duplicates, assign contiguous `u64` ordinals from 1, publish
/// in agreement. The ordinal is used ONLY inside the shared async substrate —
/// no digest truncation is ever identity.
#[derive(Clone, Debug)]
pub struct RendererAsyncOwnerTableV1 {
    by_ordinal: Vec<[u8; 32]>,
    to_ordinal: BTreeMap<[u8; 32], u64>,
}

impl RendererAsyncOwnerTableV1 {
    pub fn assign(mut digests: Vec<[u8; 32]>) -> Result<Self, OwnerTableError> {
        digests.sort();
        let mut to_ordinal = BTreeMap::new();
        for (i, d) in digests.iter().enumerate() {
            if to_ordinal.insert(*d, (i + 1) as u64).is_some() {
                return Err(OwnerTableError::DuplicateOwnerDigest { digest: *d });
            }
        }
        Ok(Self { by_ordinal: digests, to_ordinal })
    }

    #[must_use]
    pub fn ordinal_of(&self, digest: &[u8; 32]) -> Option<u64> {
        self.to_ordinal.get(digest).copied()
    }

    #[must_use]
    pub fn digest_of(&self, ordinal: u64) -> Option<&[u8; 32]> {
        if ordinal == 0 {
            return None;
        }
        self.by_ordinal.get((ordinal - 1) as usize)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_ordinal.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_ordinal.is_empty()
    }

    /// Build the shared-substrate owner key for a renderer owner (§3A.5):
    /// `stable_owner` is the table ordinal, never a truncated digest.
    #[must_use]
    pub fn owner_key(
        &self,
        digest: &[u8; 32],
        incarnation: u64,
        purpose: AsyncPurpose,
    ) -> Option<AsyncOwnerKey> {
        self.ordinal_of(digest).map(|stable_owner| AsyncOwnerKey {
            stable_owner,
            incarnation,
            purpose,
        })
    }

    /// The published table digest (order-independent by construction: the table
    /// is sorted).
    #[must_use]
    pub fn table_digest(&self) -> DomainHash {
        let mut h = DomainHasher::new("bastion/r0d/async-owner-table/sha256");
        h.field(&(self.by_ordinal.len() as u64).to_le_bytes());
        for d in &self.by_ordinal {
            h.field(d);
        }
        h.finish()
    }
}

// ----------------------------------------------------------------------------
// §3A.6 causal digest + CausalId alias bijection
// ----------------------------------------------------------------------------

/// The full renderer causal authority (§3A.6):
/// `SHA256(domain || run_epoch || scenario_digest || tick || frame || phase_tag
///        || producer_digest || local_sequence || kind_tag)`
/// via the shared length-framed `DomainHasher`.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn renderer_causal_digest(
    run_epoch: u64,
    scenario_digest: &[u8; 32],
    tick: u64,
    frame: u64,
    phase_tag: u16,
    producer_digest: &[u8; 32],
    local_sequence: u64,
    kind_tag: u16,
) -> [u8; 32] {
    let mut h = DomainHasher::new("bastion/r0d/causal/v1/sha256");
    h.field(&run_epoch.to_le_bytes());
    h.field(scenario_digest);
    h.field(&tick.to_le_bytes());
    h.field(&frame.to_le_bytes());
    h.field(&phase_tag.to_le_bytes());
    h.field(producer_digest);
    h.field(&local_sequence.to_le_bytes());
    h.field(&kind_tag.to_le_bytes());
    h.finish().0
}

/// Alias-table failure: two full causal digests mapped to one `CausalId` — the
/// collision canary (§3A.6). Diagnostics may not index by the alias until the
/// bijection is proven collision-free for the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalAliasCollision {
    pub id: CausalId,
    pub first: [u8; 32],
    pub second: [u8; 32],
}

/// A run-local `CausalId` alias table with collision proof (§3A.6). First-
/// divergence records always carry the FULL digest; the u64 alias only indexes
/// diagnostics after this bijection holds.
#[derive(Clone, Debug, Default)]
pub struct CausalAliasTableV1 {
    by_id: BTreeMap<u64, [u8; 32]>,
}

impl CausalAliasTableV1 {
    /// Register a full digest under its compact alias. The alias is the first 8
    /// LE bytes of the digest — acceptable ONLY because a collision is a typed
    /// terminal, never silent.
    pub fn register(&mut self, full: [u8; 32]) -> Result<CausalId, CausalAliasCollision> {
        let id = u64::from_le_bytes(full[..8].try_into().expect("8 bytes"));
        match self.by_id.get(&id) {
            None => {
                self.by_id.insert(id, full);
                Ok(CausalId(id))
            }
            Some(existing) if *existing == full => Ok(CausalId(id)),
            Some(existing) => Err(CausalAliasCollision { id: CausalId(id), first: *existing, second: full }),
        }
    }

    #[must_use]
    pub fn full_digest(&self, id: CausalId) -> Option<&[u8; 32]> {
        self.by_id.get(&id.0)
    }
}

// ----------------------------------------------------------------------------
// §3A.6 equivalence gate
// ----------------------------------------------------------------------------

/// A tolerance entry for an R0D-required renderer event kind is forbidden
/// (§3A.6). R0D kinds are namespaced `r0d/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForbiddenTolerance {
    pub kinds: Vec<String>,
}

/// Guard the tolerance map (§3A.6): any `r0d/`-namespaced kind with a nonzero
/// tolerance is rejected before comparison.
pub fn guard_r0d_zero_tolerance(t: &EquivalenceTolerance) -> Result<(), ForbiddenTolerance> {
    let kinds: Vec<String> = t
        .per_kind
        .iter()
        .filter(|(k, &v)| k.starts_with("r0d/") && v > 0)
        .map(|(k, _)| k.clone())
        .collect();
    if kinds.is_empty() { Ok(()) } else { Err(ForbiddenTolerance { kinds }) }
}

/// The renderer admission verdict (§3A.6): shared authoritative equivalence
/// must pass AND exact renderer tape comparison must pass AND structural visual
/// equality must pass. Layered strictly — a renderer mismatch cannot be hidden
/// by `RunSummary` tolerance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RendererEquivalence {
    Equivalent,
    ToleranceForbidden(ForbiddenTolerance),
    AuthoritativeMismatch(EquivalenceVerdict),
    RendererTapeMismatch,
    StructuralVisualMismatch,
}

/// Run the three-layer gate (§3A.6).
#[must_use]
pub fn renderer_equivalence(
    a: &RunSummary,
    b: &RunSummary,
    required_edges: &std::collections::BTreeSet<(CausalId, CausalId)>,
    tolerance: &EquivalenceTolerance,
    tape_roots: (&DomainHash, &DomainHash),
    structural_roots: (&DomainHash, &DomainHash),
) -> RendererEquivalence {
    if let Err(f) = guard_r0d_zero_tolerance(tolerance) {
        return RendererEquivalence::ToleranceForbidden(f);
    }
    let shared = check_equivalence(a, b, required_edges, tolerance);
    if shared != EquivalenceVerdict::Equivalent {
        return RendererEquivalence::AuthoritativeMismatch(shared);
    }
    if tape_roots.0 != tape_roots.1 {
        return RendererEquivalence::RendererTapeMismatch;
    }
    if structural_roots.0 != structural_roots.1 {
        return RendererEquivalence::StructuralVisualMismatch;
    }
    RendererEquivalence::Equivalent
}

// ----------------------------------------------------------------------------
// §3A.7 staged feature-protocol fitness rule
// ----------------------------------------------------------------------------

/// Staged renderer-feature violations (§3A.7). Expressible in the CURRENT
/// shared vocabulary; the `AuthorityDomain::RendererPresentation` /
/// `ClockDomain::RenderFrame` enum additions await shared-engine review and are
/// NOT made by this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RendererFeatureViolation {
    /// The renderer feature declared an existing WRITE authority domain —
    /// forbidden: its only authority is renderer presentation.
    ForbiddenWriteAuthority(AuthorityDomain),
    /// `Wall` is rejected outright for the renderer feature.
    WallClock,
    /// The renderer feature must record causal.
    MissingCausalRecording,
}

/// Validate the `renderer-r0d` feature declaration under the staged rule
/// (§3A.7): no Terrain/Inventory/Ecs/Rtsim/JobBoardCoordination/Persistence
/// write authority, no Wall clock, causal recording required. When the shared
/// enum extension lands, this tightens to exactly `[RendererPresentation]`.
pub fn validate_renderer_feature(decl: &FeatureProtocolDecl) -> Vec<RendererFeatureViolation> {
    let mut v = Vec::new();
    for d in &decl.authoritative_domains {
        v.push(RendererFeatureViolation::ForbiddenWriteAuthority(*d));
    }
    if decl.clock_domains.contains(&ClockDomain::Wall) {
        v.push(RendererFeatureViolation::WallClock);
    }
    if !decl.observability.records_causal {
        v.push(RendererFeatureViolation::MissingCausalRecording);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::feature_protocol::{
        FeatureId, ModuleId, ObservabilityContract, PolicyId, TransactionBoundary,
    };
    use common::state_hash::IntegrityHash;
    use std::collections::BTreeSet;

    fn dh(b: u8) -> DomainHash {
        DomainHash([b; 32])
    }

    fn cert(seed: u32, tick: u64, composite: u8) -> FinalStateCertificate {
        FinalStateCertificate {
            schema: "bastion/final-state-certificate/v1".to_string(),
            world_seed: seed,
            tick,
            durable_composite: dh(composite),
            rebuildable_integrity: IntegrityHash([0; 32]),
        }
    }

    fn schema() -> RecorderSchemaRef {
        RecorderSchemaRef { family: 40, major: 1, minor: 0, patch: 0 }
    }

    fn admission(composite: u8, extract: u8) -> RendererR0DAdmissionV2 {
        RendererR0DAdmissionV2 {
            schema: schema(),
            authoritative: cert(7, 100, composite),
            content: ContentManifest::build(
                "bastion/content-manifest/v1",
                "5d0dc72b9a",
                vec![],
                vec![],
                vec![],
            ),
            renderer_extract_root: dh(extract),
            replication_projection_root: dh(2),
            canonical_selection_root: dh(3),
            asset_package_root: dh(4),
            structural_visual_root: dh(5),
            artifact_index_root: dh(6),
            causal_tape_root: dh(7),
            environment_policy_digest: dh(8),
        }
    }

    #[test]
    fn hash_adapter_only_accepts_validated_canonical_bytes() {
        // A canonical CBOR uint payload validates; its DomainHash is stable and
        // label-separated (this is the §3A.3 pipeline end-to-end).
        let canonical = ValidatedCanonicalBytesV1::validate(&[0x05]).unwrap();
        let a = renderer_domain_hash("bastion/r0d/test/v1/sha256", &canonical);
        let b = renderer_domain_hash("bastion/r0d/test/v1/sha256", &canonical);
        let c = renderer_domain_hash("bastion/r0d/other/v1/sha256", &canonical);
        assert_eq!(a, b);
        assert_ne!(a, c, "domain label separates");
        // Non-canonical bytes (nonpreferred integer encoding) cannot even be
        // constructed into the input type.
        assert!(ValidatedCanonicalBytesV1::validate(&[0x18, 0x05]).is_err());
    }

    #[test]
    fn admission_schema_mismatch_is_typed_terminal() {
        let adm = admission(1, 1);
        assert!(adm.check_schema(schema()).is_ok());
        let mut wrong_major = schema();
        wrong_major.major = 2;
        assert!(adm.check_schema(wrong_major).is_err());
        let mut newer_minor = schema();
        newer_minor.minor = 3; // additive: still readable
        assert!(adm.check_schema(newer_minor).is_ok());
    }

    #[test]
    fn admission_compares_exactly_and_digests_are_sensitive() {
        let a = admission(1, 1);
        assert!(a.matches_exactly(&admission(1, 1)));
        // A renderer-root difference fails exactly (no tolerance).
        assert!(!a.matches_exactly(&admission(1, 9)));
        // An authoritative composite difference fails via the SHARED check.
        assert!(!a.matches_exactly(&admission(9, 1)));
        assert_ne!(a.admission_digest(), admission(1, 9).admission_digest());
    }

    #[test]
    fn owner_table_is_sorted_contiguous_and_feeds_owner_key() {
        let t = RendererAsyncOwnerTableV1::assign(vec![[3; 32], [1; 32], [2; 32]]).unwrap();
        assert_eq!(t.ordinal_of(&[1; 32]), Some(1));
        assert_eq!(t.ordinal_of(&[3; 32]), Some(3));
        assert_eq!(t.digest_of(2), Some(&[2; 32]));
        assert_eq!(t.digest_of(0), None);
        let k = t.owner_key(&[2; 32], 5, AsyncPurpose(9)).unwrap();
        assert_eq!(k.stable_owner, 2);
        assert_eq!(k.incarnation, 5);
        // Insertion order cannot change the published table digest.
        let t2 = RendererAsyncOwnerTableV1::assign(vec![[1; 32], [2; 32], [3; 32]]).unwrap();
        assert_eq!(t.table_digest(), t2.table_digest());
        // Duplicates are typed terminal.
        assert!(RendererAsyncOwnerTableV1::assign(vec![[1; 32], [1; 32]]).is_err());
    }

    #[test]
    fn causal_digest_is_tuple_sensitive_and_alias_collisions_are_typed() {
        let a = renderer_causal_digest(1, &[2; 32], 10, 0, 3, &[4; 32], 0, 7);
        let b = renderer_causal_digest(1, &[2; 32], 10, 0, 3, &[4; 32], 0, 8);
        assert_ne!(a, b, "kind tag separates");
        let mut table = CausalAliasTableV1::default();
        let id = table.register(a).unwrap();
        assert_eq!(table.full_digest(id), Some(&a));
        // Same digest re-registers fine.
        assert_eq!(table.register(a).unwrap(), id);
        // A different digest with the same first-8 bytes collides — typed.
        let mut forged = b;
        forged[..8].copy_from_slice(&a[..8]);
        assert!(table.register(forged).is_err());
    }

    fn summary(composite: u8, events: &[(&str, u64)]) -> RunSummary {
        RunSummary {
            certificate: cert(7, 100, composite),
            causal_edges: BTreeSet::new(),
            conservation: BTreeMap::new(),
            independent_events: events.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        }
    }

    #[test]
    fn equivalence_gate_layers_strictly() {
        let edges = BTreeSet::new();
        let tol = EquivalenceTolerance::default();
        let a = summary(1, &[]);
        // All layers pass.
        assert_eq!(
            renderer_equivalence(&a, &summary(1, &[]), &edges, &tol, (&dh(9), &dh(9)), (&dh(8), &dh(8))),
            RendererEquivalence::Equivalent
        );
        // Authoritative mismatch caught by the SHARED layer.
        assert!(matches!(
            renderer_equivalence(&a, &summary(2, &[]), &edges, &tol, (&dh(9), &dh(9)), (&dh(8), &dh(8))),
            RendererEquivalence::AuthoritativeMismatch(EquivalenceVerdict::HashMismatch)
        ));
        // Tape mismatch caught even when authoritative passes.
        assert_eq!(
            renderer_equivalence(&a, &summary(1, &[]), &edges, &tol, (&dh(9), &dh(1)), (&dh(8), &dh(8))),
            RendererEquivalence::RendererTapeMismatch
        );
        // Structural mismatch is the final layer.
        assert_eq!(
            renderer_equivalence(&a, &summary(1, &[]), &edges, &tol, (&dh(9), &dh(9)), (&dh(8), &dh(1))),
            RendererEquivalence::StructuralVisualMismatch
        );
    }

    #[test]
    fn r0d_tolerance_entries_are_forbidden() {
        let mut tol = EquivalenceTolerance::default();
        tol.per_kind.insert("r0d/camera-frame".to_string(), 1);
        tol.per_kind.insert("gameplay/ambient".to_string(), 5); // allowed
        let err = guard_r0d_zero_tolerance(&tol).unwrap_err();
        assert_eq!(err.kinds, vec!["r0d/camera-frame".to_string()]);
        // Zero-valued r0d entry is fine (exact).
        let mut tol2 = EquivalenceTolerance::default();
        tol2.per_kind.insert("r0d/camera-frame".to_string(), 0);
        assert!(guard_r0d_zero_tolerance(&tol2).is_ok());
        // And the gate short-circuits on the forbidden tolerance.
        let a = summary(1, &[]);
        assert!(matches!(
            renderer_equivalence(&a, &a.clone(), &BTreeSet::new(), &tol, (&dh(9), &dh(9)), (&dh(8), &dh(8))),
            RendererEquivalence::ToleranceForbidden(_)
        ));
    }

    fn renderer_decl(domains: Vec<AuthorityDomain>, clocks: Vec<ClockDomain>, causal: bool) -> FeatureProtocolDecl {
        FeatureProtocolDecl {
            feature: FeatureId("renderer-r0d".to_string()),
            owner_module: ModuleId("bastion-renderer-r0d".to_string()),
            authoritative_domains: domains,
            command_types: vec![],
            required_capabilities: vec![],
            clock_domains: clocks,
            transaction_boundary: TransactionBoundary::InProcessUnitOfWork,
            lifecycle_policy: PolicyId("r0d-run-epoch".to_string()),
            persistence_policy: PolicyId("none".to_string()),
            lod_policy: PolicyId("r0d-canonical".to_string()),
            observability: ObservabilityContract { records_causal: causal, emits_command_status: false },
            acceptance_tests: vec![],
        }
    }

    #[test]
    fn staged_renderer_feature_rule() {
        // Clean: no write authority, Sim clock only, causal recorded.
        assert!(validate_renderer_feature(&renderer_decl(vec![], vec![ClockDomain::Sim], true)).is_empty());
        // Any existing write authority is forbidden.
        let v = validate_renderer_feature(&renderer_decl(vec![AuthorityDomain::Ecs], vec![ClockDomain::Sim], true));
        assert_eq!(v, vec![RendererFeatureViolation::ForbiddenWriteAuthority(AuthorityDomain::Ecs)]);
        // Wall is rejected; missing causal recording is rejected.
        let v = validate_renderer_feature(&renderer_decl(vec![], vec![ClockDomain::Wall], false));
        assert!(v.contains(&RendererFeatureViolation::WallClock));
        assert!(v.contains(&RendererFeatureViolation::MissingCausalRecording));
    }
}
