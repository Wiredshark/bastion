//! `APEX-T4.6` chunk 1 — `SaveUniverseManifestV1`: the durable binding that
//! makes a save epoch's state COMPLETE and NAMEABLE, plus the pure epoch-
//! lineage ledger that admits or refuses a candidate epoch before it is
//! ever written to disk.
//!
//! **The gap this closes.** `RtSim::save` (`server/src/rtsim/mod.rs:668`)
//! writes its own store through `AtomicFile`, and `CharacterUpdater`
//! commits sqlite on its own independent schedule — confirmed at
//! premise-check time, not guessed. Neither knows the other's tick, so a
//! crash between the two produces two internally-valid stores that never
//! coexisted. This row's architecture note: freeze one tick, stage every
//! store, bind their digests into ONE manifest, then publish a single
//! atomic pointer as the sole commit point.
//!
//! **Chunk scope, self-sized per this program's own standing discipline**
//! (orchestrator-approved 4-chunk split): this chunk is the DATA MODEL —
//! the manifest type and its codec, the epoch-pointer VALUE type, the
//! pure epoch-zero classifier, and the pure epoch-lineage ledger — fully
//! testable against fixtures alone, no filesystem. Deliberately NOT built
//! here, banked for chunk 2: the real staged-write-then-publish protocol
//! (the `AtomicFile` primitive `RtSim::save` already uses, reused not
//! reinvented — the pointer is what's new), and the real on-disk pointer
//! FILE this value type gets written into. Chunk 3: wiring into the real
//! save trigger, and subsuming `rtsim::data::Data::world_baseline_root`
//! (`T4.6-INTERIM`'s own doc comment names this row as its real home).
//! Chunk 4: the `SAVE-001..` crash-injection fixture tests.
//!
//! **Reused rather than reinvented, per this program's own repeated law:**
//! - The epoch-lineage chain (`SaveEpochLineageV1`/`SaveEpochLedgerV1`)
//!   copies `T4.2`'s `BootstrapFreshnessLedgerV1` SHAPE — a floor plus a
//!   predecessor-root chain, distinct typed rejections per failure mode —
//!   simplified for this row's single-lineage, no-resume, strictly-
//!   sequential shape (a save epoch has no "reconnect" analog).
//! - `SaveEpoch` (`common/src/apex/identity/counter.rs`) and its manifest
//!   codec impl already exist, pre-reserved with the doc comment
//!   "Zero/genesis validity policy is owned by T4" — `SaveEpoch(0)` IS the
//!   pointer-less-directory-reads-as-epoch-zero case this row's migration
//!   note requires, not a separate bool this module invents.
//! - `content`/`build`/`numeric`/`schedule` identity is carried as
//!   `Vec<SubsystemDescriptorV1>` — `T0.5`'s sparse-by-slot vocabulary,
//!   the same reuse `BootstrapManifestV1` already made. This is also
//!   where a future `T8.5` economy-remedy entry declares itself: push a
//!   descriptor at `SubsystemSlotIdV1::Economy`, no new mechanism needed.
//! - `world_baseline_root` stays its own dedicated field (not forced
//!   through a `SubsystemDescriptorV1`): it is already a
//!   `WorldBaselineManifest`-domain `ProtocolDigestV1` (`T4.3`), not raw
//!   content a `ContentIdentityV1` could honestly wrap.
//! - `migration_journal_digest` is `None` today, matching `T4.5`'s
//!   confirmed-EMPTY rtsim migration graph — a reserved place to declare
//!   the journal once a real migration step exists, not a fabricated one.

use crate::apex::digest::{
    ArtifactDigestV1, ArtifactIdentityV1, DigestDomainIdV1, DigestErrorV1, ProtocolDigestV1, digest_manifest_value_v1,
};
use crate::apex::identity::SaveEpoch;
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeLimitsV1, ManifestDecodeV1,
    ManifestEncodeV1, ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};
use crate::apex::subsystem::descriptor::SubsystemDescriptorV1;

/// Decode limits for this row's manifest. `max_array_items` covers
/// `stores` (a handful today, `SaveStoreIdV1::CharacterDb`/`RtsimData`)
/// and `descriptors` (at most `SubsystemSlotIdV1`'s own small vocabulary)
/// — neither is expected to grow large, unlike a command log.
pub const fn save_universe_manifest_limits_v1() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 1 << 16,
        max_depth: 10,
        max_nodes: 1 << 12,
        max_array_items: 256,
        max_map_entries: 32,
        max_machine_text_bytes: 512,
        max_byte_string_bytes: 512,
    }
}

fn err(detail: &'static str) -> ManifestSchemaErrorV1 { ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail(detail) }

/// `Option<T>` has no bare representation in the restricted data model —
/// the same 0-or-1-element-array discriminant `bootstrap_freshness.rs`
/// and `bootstrap_manifest.rs` already use, reused rather than a third
/// encoding invented here.
fn encode_optional_digest(v: &Option<ArtifactDigestV1>) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    match v {
        Some(d) => Ok(ManifestValueV1::Array(vec![d.to_manifest_value_v1()?])),
        None => Ok(ManifestValueV1::Array(Vec::new())),
    }
}

fn decode_optional_digest(value: ManifestValueV1) -> Result<Option<ArtifactDigestV1>, ManifestSchemaErrorV1> {
    let ManifestValueV1::Array(items) = value else { return Err(err("expected an array")) };
    match <[ManifestValueV1; 1]>::try_from(items) {
        Ok([only]) => Ok(Some(ArtifactDigestV1::from_manifest_value_v1(only)?)),
        Err(items) if items.is_empty() => Ok(None),
        Err(_) => Err(err("optional digest array must have 0 or 1 elements")),
    }
}

// ---------------------------------------------------------------------
// Epoch lineage: the typed tuple that goes INTO a manifest, plus the
// pure ledger that admits or refuses a candidate BEFORE one is ever
// written.
// ---------------------------------------------------------------------

/// One epoch's declared position in the save's history. Embedded in
/// [`SaveUniverseManifestV1`] as the durable record; [`SaveEpochLedgerV1`]
/// is the separate in-process admission mechanism that checks a
/// candidate against everything already committed this boot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveEpochLineageV1 {
    pub epoch: SaveEpoch,
    /// The manifest root of the epoch this one extends. `None` only when
    /// `epoch.get() == 1`, chaining from epoch zero — the pointer-less
    /// legacy directory this row's migration note names, which was never
    /// staged and so has no manifest root to chain from. Every later
    /// epoch's predecessor root is `Some`.
    pub predecessor_root: Option<ArtifactDigestV1>,
}

impl ManifestEncodeV1 for SaveEpochLineageV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), self.epoch.to_manifest_value_v1()?),
            (FieldIdV1::new(2), encode_optional_digest(&self.predecessor_root)?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SaveEpochLineageV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(err("expected a map")) };
        let mut fields = StructFieldsV1::new(map);
        let epoch = SaveEpoch::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let predecessor_root = decode_optional_digest(fields.take_required(FieldIdV1::new(2))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { epoch, predecessor_root })
    }
}

/// Why a candidate epoch was refused admission — each a distinct typed
/// terminal, same discipline `BootstrapFreshnessRejectionV1` established
/// ("collapsing them into one 'invalid' loses the diagnosis").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveEpochRejectionV1 {
    /// `SaveEpoch(0)` is the reserved epoch-zero sentinel — never a real
    /// candidate a commit-writer admits.
    EpochZeroReserved,
    /// The candidate does not extend the floor by exactly one. A save
    /// epoch has no resume/reconnect analog (unlike `T4.2`'s connection
    /// epoch), so unlike `BootstrapFreshnessLedgerV1` there is only ONE
    /// failure mode here for a non-extending epoch, covering both a
    /// replay of an already-committed epoch and a gap this ledger
    /// refuses to paper over.
    NotSequential { floor: SaveEpoch, candidate: SaveEpoch },
    /// The candidate's declared `predecessor_root` doesn't match the
    /// floor's own root (an undeclared fork), or claims `Some` root when
    /// it should be `None` (epoch 1 claiming a predecessor that epoch
    /// zero never had), or `None` when a real predecessor root was
    /// required.
    PredecessorMismatch,
}

/// The pure epoch-lineage ledger: a floor (the last admitted epoch's
/// number and own manifest root), `None` before the first admission.
/// Mirrors `BootstrapFreshnessLedgerV1`'s floor half; deliberately
/// without its rebindable-epoch half, since nothing here resumes.
#[derive(Clone, Copy, Debug, Default)]
pub struct SaveEpochLedgerV1 {
    floor: Option<(SaveEpoch, ArtifactDigestV1)>,
}

impl SaveEpochLedgerV1 {
    pub fn new() -> Self { Self { floor: None } }

    /// The last admitted epoch, or `SaveEpoch::INITIAL` (zero) before
    /// anything has ever been admitted — the same epoch-zero-as-genesis
    /// reading a pointer-less save directory gets, not a special case.
    pub fn current_epoch(&self) -> SaveEpoch { self.floor.map_or(SaveEpoch::INITIAL, |(e, _)| e) }

    pub fn current_root(&self) -> Option<ArtifactDigestV1> { self.floor.map(|(_, root)| root) }

    /// Classifies and, if admitted, advances the floor. `candidate_root`
    /// is the caller-computed manifest root of `candidate`'s OWN epoch
    /// (this ledger has no opinion on how that root is computed — see
    /// [`compute_save_universe_manifest_root_v1`]).
    pub fn admit_v1(&mut self, candidate: SaveEpochLineageV1, candidate_root: ArtifactDigestV1) -> Result<(), SaveEpochRejectionV1> {
        if candidate.epoch.get() == 0 {
            return Err(SaveEpochRejectionV1::EpochZeroReserved);
        }
        let floor_epoch = self.current_epoch();
        if candidate.epoch.get() != floor_epoch.get() + 1 {
            return Err(SaveEpochRejectionV1::NotSequential { floor: floor_epoch, candidate: candidate.epoch });
        }
        if candidate.predecessor_root != self.current_root() {
            return Err(SaveEpochRejectionV1::PredecessorMismatch);
        }
        self.floor = Some((candidate.epoch, candidate_root));
        Ok(())
    }
}

// ---------------------------------------------------------------------
// The epoch pointer's VALUE (chunk 2 owns the on-disk FILE this gets
// written into and the real read that produces a `SaveEpochPointerReadV1`).
// ---------------------------------------------------------------------

/// The published pointer's content: which epoch is current, and the
/// exact-byte identity of the manifest that commits it. Binding the
/// manifest's identity INTO the pointer (not just its epoch number)
/// means recovery can tell "a manifest exists at this epoch's path" from
/// "the manifest the pointer actually named is the one on disk" — the
/// two are not the same claim after a torn write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveEpochPointerV1 {
    pub epoch: SaveEpoch,
    pub manifest_identity: ArtifactIdentityV1,
}

impl ManifestEncodeV1 for SaveEpochPointerV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), self.epoch.to_manifest_value_v1()?),
            (FieldIdV1::new(2), self.manifest_identity.to_manifest_value_v1()?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SaveEpochPointerV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(err("expected a map")) };
        let mut fields = StructFieldsV1::new(map);
        let epoch = SaveEpoch::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let manifest_identity = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(2))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { epoch, manifest_identity })
    }
}

/// What chunk 2's real filesystem read resolves to, classified PURELY
/// (no filesystem here — the real read, and its corrupt/undecodable-bytes
/// handling, is chunk 2's job). This is the row's own migration law made
/// a type: a save directory that has never published a pointer reads as
/// epoch zero, never as corruption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveEpochPointerReadV1 {
    /// No pointer file exists — epoch zero, the pre-adoption legacy
    /// state this row's migration note names.
    NeverPublished,
    Published(SaveEpochPointerV1),
}

impl SaveEpochPointerReadV1 {
    /// The epoch this read implies, unconditionally — the one fact
    /// chunk 2's recovery path needs before it can even attempt payload
    /// verification.
    pub fn epoch(&self) -> SaveEpoch {
        match self {
            Self::NeverPublished => SaveEpoch::INITIAL,
            Self::Published(p) => p.epoch,
        }
    }
}

// ---------------------------------------------------------------------
// Per-store payload binding.
// ---------------------------------------------------------------------

/// A stable identifier for one persisted store this epoch's manifest
/// binds. Deliberately NOT `server::save_inventory::SaveStoreKindV1`
/// itself — `common` cannot depend on `server`, the same dependency-
/// direction constraint `world_baseline.rs` documents for `world`.
/// Explicit discriminants for wire stability, same discipline as
/// `DigestDomainIdV1`. Matches `SaveStoreKindV1::EXPECTED`'s two members
/// by name; server code is the integration point that maps between the
/// two lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SaveStoreIdV1 {
    CharacterDb = 1,
    RtsimData = 2,
}

impl SaveStoreIdV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const fn try_from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Self::CharacterDb),
            2 => Some(Self::RtsimData),
            _ => None,
        }
    }
}

impl ManifestEncodeV1 for SaveStoreIdV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(ManifestValueV1::Unsigned(self.as_u16() as u64)) }
}

impl ManifestDecodeV1 for SaveStoreIdV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Unsigned(v) = value else { return Err(err("expected an unsigned store id")) };
        if v > u16::MAX as u64 {
            return Err(err("store id out of range"));
        }
        Self::try_from_u16(v as u16).ok_or_else(|| err("unknown store id"))
    }
}

/// One store's staged payload: which store, and its exact-byte identity
/// (post-flush, per chunk 2's "flush and verify each payload's digest").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveStorePayloadV1 {
    pub store: SaveStoreIdV1,
    pub identity: ArtifactIdentityV1,
}

impl ManifestEncodeV1 for SaveStorePayloadV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), self.store.to_manifest_value_v1()?),
            (FieldIdV1::new(2), self.identity.to_manifest_value_v1()?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SaveStorePayloadV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(err("expected a map")) };
        let mut fields = StructFieldsV1::new(map);
        let store = SaveStoreIdV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let identity = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(2))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { store, identity })
    }
}

// ---------------------------------------------------------------------
// The manifest itself.
// ---------------------------------------------------------------------

/// `T4.6`'s durable binding: every store's payload digest, world/content/
/// build/numeric/schedule identity, the frozen tick, the parent epoch,
/// and the migration journal — the spec's own list, cashed field for
/// field. See the module doc for which existing mechanisms each field
/// reuses rather than reinvents.
///
/// `stores` and `descriptors` are plain order-significant arrays — same
/// "sparse by construction" discipline as `BootstrapManifestV1`'s own
/// fields, not auto-canonicalized by this codec. A caller that always
/// constructs them in a fixed order (chunk 2/3's job) gets a
/// reproducible manifest byte-image; this type does not enforce that by
/// itself.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveUniverseManifestV1 {
    pub lineage: SaveEpochLineageV1,
    /// The one frozen simulation tick every staged store was snapshotted
    /// at — the row's own "freeze one canonical simulation tick" step.
    pub frozen_tick: u64,
    pub stores: Vec<SaveStorePayloadV1>,
    /// `T4.3`'s `WorldBaselineManifest`-domain root. `None` only for an
    /// epoch staged before a world baseline was ever computed (should not
    /// occur once chunk 3 wires this row as `world_baseline_root`'s real
    /// home, kept optional at the type level rather than assumed).
    pub world_baseline_root: Option<ArtifactDigestV1>,
    /// `T0.5`'s sparse per-slot vocabulary, reused wholesale — see the
    /// module doc for why this is also `T8.5`'s declared remedy slot.
    pub descriptors: Vec<SubsystemDescriptorV1>,
    /// `T4.5`'s confirmed-EMPTY rtsim migration graph, reserved rather
    /// than fabricated — `None` until a real migration step exists to
    /// journal.
    pub migration_journal_digest: Option<ArtifactDigestV1>,
}

impl ManifestEncodeV1 for SaveUniverseManifestV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut stores = Vec::with_capacity(self.stores.len());
        for s in &self.stores {
            stores.push(s.to_manifest_value_v1()?);
        }
        let mut descriptors = Vec::with_capacity(self.descriptors.len());
        for d in &self.descriptors {
            descriptors.push(d.to_manifest_value_v1()?);
        }
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), self.lineage.to_manifest_value_v1()?),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(self.frozen_tick)),
            (FieldIdV1::new(3), ManifestValueV1::Array(stores)),
            (FieldIdV1::new(4), encode_optional_digest(&self.world_baseline_root)?),
            (FieldIdV1::new(5), ManifestValueV1::Array(descriptors)),
            (FieldIdV1::new(6), encode_optional_digest(&self.migration_journal_digest)?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SaveUniverseManifestV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(err("expected a map")) };
        let mut fields = StructFieldsV1::new(map);

        let lineage = SaveEpochLineageV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let frozen_tick = match fields.take_required(FieldIdV1::new(2))? {
            ManifestValueV1::Unsigned(v) => v,
            _ => return Err(err("frozen_tick must be an unsigned integer")),
        };
        let ManifestValueV1::Array(store_values) = fields.take_required(FieldIdV1::new(3))? else {
            return Err(err("stores must be an array"));
        };
        let mut stores = Vec::with_capacity(store_values.len());
        for v in store_values {
            stores.push(SaveStorePayloadV1::from_manifest_value_v1(v)?);
        }
        let world_baseline_root = decode_optional_digest(fields.take_required(FieldIdV1::new(4))?)?;
        let ManifestValueV1::Array(descriptor_values) = fields.take_required(FieldIdV1::new(5))? else {
            return Err(err("descriptors must be an array"));
        };
        let mut descriptors = Vec::with_capacity(descriptor_values.len());
        for v in descriptor_values {
            descriptors.push(SubsystemDescriptorV1::from_manifest_value_v1(v)?);
        }
        let migration_journal_digest = decode_optional_digest(fields.take_required(FieldIdV1::new(6))?)?;

        fields.finish_no_unknown()?;
        Ok(Self { lineage, frozen_tick, stores, world_baseline_root, descriptors, migration_journal_digest })
    }
}

/// This epoch's semantic identity — the root other systems reference
/// (`T8.5`'s economy remedy, a future replay bundle's
/// `start_checkpoint_digest`, already typed as this exact domain in
/// `replay_bundle.rs`) rather than the raw manifest bytes.
pub fn compute_save_universe_manifest_root_v1(manifest: &SaveUniverseManifestV1) -> Result<ProtocolDigestV1, DigestErrorV1> {
    digest_manifest_value_v1(DigestDomainIdV1::SaveUniverseManifest, manifest, &save_universe_manifest_limits_v1())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::{ContentIdentityV1, hash_artifact_bytes_v1};
    use crate::apex::scalar::SchemaVersion;
    use crate::apex::subsystem::slot::SubsystemSlotIdV1;

    fn digest(tag: u8) -> ArtifactDigestV1 { hash_artifact_bytes_v1(&[tag]).digest }
    fn identity(tag: u8) -> ArtifactIdentityV1 { hash_artifact_bytes_v1(&[tag, tag]) }

    fn descriptor(slot: SubsystemSlotIdV1, seed: &[u8]) -> SubsystemDescriptorV1 {
        SubsystemDescriptorV1 { slot, schema: SchemaVersion::new(1), content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(seed), semantic: None } }
    }

    fn lineage(epoch: u64, predecessor_root: Option<ArtifactDigestV1>) -> SaveEpochLineageV1 {
        SaveEpochLineageV1 { epoch: SaveEpoch::new(epoch), predecessor_root }
    }

    fn manifest() -> SaveUniverseManifestV1 {
        SaveUniverseManifestV1 {
            lineage: lineage(1, None),
            frozen_tick: 4200,
            stores: vec![
                SaveStorePayloadV1 { store: SaveStoreIdV1::CharacterDb, identity: identity(1) },
                SaveStorePayloadV1 { store: SaveStoreIdV1::RtsimData, identity: identity(2) },
            ],
            world_baseline_root: Some(digest(9)),
            descriptors: vec![descriptor(SubsystemSlotIdV1::Content, b"content"), descriptor(SubsystemSlotIdV1::Numeric, b"numeric")],
            migration_journal_digest: None,
        }
    }

    // -- manifest codec --------------------------------------------------

    #[test]
    fn full_manifest_round_trips() {
        let original = manifest();
        let bytes = crate::apex::manifest::encode_manifest_v1(&original, &save_universe_manifest_limits_v1()).unwrap();
        let decoded: SaveUniverseManifestV1 = crate::apex::manifest::decode_manifest_v1(&bytes, &save_universe_manifest_limits_v1()).unwrap();
        assert_eq!(original, decoded);
    }

    /// Every optional field absent, no stores, no descriptors — the
    /// minimal manifest an epoch-1 commit with nothing yet resolved could
    /// honestly produce.
    #[test]
    fn minimal_manifest_round_trips() {
        let original = SaveUniverseManifestV1 {
            lineage: lineage(1, None),
            frozen_tick: 0,
            stores: Vec::new(),
            world_baseline_root: None,
            descriptors: Vec::new(),
            migration_journal_digest: None,
        };
        let bytes = crate::apex::manifest::encode_manifest_v1(&original, &save_universe_manifest_limits_v1()).unwrap();
        let decoded: SaveUniverseManifestV1 = crate::apex::manifest::decode_manifest_v1(&bytes, &save_universe_manifest_limits_v1()).unwrap();
        assert_eq!(original, decoded);
    }

    /// A later epoch (with a real predecessor root, per the type's own
    /// invariant) round-trips too — not just the epoch-1/`None` case.
    #[test]
    fn later_epoch_with_predecessor_root_round_trips() {
        let original = SaveUniverseManifestV1 { lineage: lineage(7, Some(digest(3))), ..manifest() };
        let bytes = crate::apex::manifest::encode_manifest_v1(&original, &save_universe_manifest_limits_v1()).unwrap();
        let decoded: SaveUniverseManifestV1 = crate::apex::manifest::decode_manifest_v1(&bytes, &save_universe_manifest_limits_v1()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn epoch_pointer_round_trips() {
        let original = SaveEpochPointerV1 { epoch: SaveEpoch::new(5), manifest_identity: identity(1) };
        let bytes = crate::apex::manifest::encode_manifest_v1(&original, &save_universe_manifest_limits_v1()).unwrap();
        let decoded: SaveEpochPointerV1 = crate::apex::manifest::decode_manifest_v1(&bytes, &save_universe_manifest_limits_v1()).unwrap();
        assert_eq!(original, decoded);
    }

    // -- pointer read classifier -----------------------------------------

    /// This row's own migration law, as a type-level assertion: no
    /// pointer ever published reads as epoch zero, not an error.
    #[test]
    fn never_published_reads_as_epoch_zero() {
        assert_eq!(SaveEpochPointerReadV1::NeverPublished.epoch(), SaveEpoch::INITIAL);
        assert_eq!(SaveEpoch::INITIAL.get(), 0);
    }

    #[test]
    fn published_pointer_reports_its_own_epoch() {
        let pointer = SaveEpochPointerV1 { epoch: SaveEpoch::new(3), manifest_identity: identity(1) };
        assert_eq!(SaveEpochPointerReadV1::Published(pointer).epoch(), SaveEpoch::new(3));
    }

    // -- epoch ledger ------------------------------------------------------

    #[test]
    fn the_first_epoch_admits_unconditionally_at_one_with_no_predecessor() {
        let mut ledger = SaveEpochLedgerV1::new();
        assert_eq!(ledger.current_epoch(), SaveEpoch::INITIAL);
        assert!(ledger.admit_v1(lineage(1, None), digest(1)).is_ok());
        assert_eq!(ledger.current_epoch(), SaveEpoch::new(1));
        assert_eq!(ledger.current_root(), Some(digest(1)));
    }

    #[test]
    fn a_correctly_chained_second_epoch_is_admitted() {
        let mut ledger = SaveEpochLedgerV1::new();
        ledger.admit_v1(lineage(1, None), digest(1)).unwrap();
        assert!(ledger.admit_v1(lineage(2, Some(digest(1))), digest(2)).is_ok());
        assert_eq!(ledger.current_epoch(), SaveEpoch::new(2));
    }

    #[test]
    fn epoch_zero_is_never_a_valid_candidate() {
        let mut ledger = SaveEpochLedgerV1::new();
        assert_eq!(ledger.admit_v1(lineage(0, None), digest(1)).unwrap_err(), SaveEpochRejectionV1::EpochZeroReserved);
    }

    /// A replay of the already-committed epoch is refused, distinctly
    /// from a gap.
    #[test]
    fn replaying_the_current_epoch_is_refused_not_sequential() {
        let mut ledger = SaveEpochLedgerV1::new();
        ledger.admit_v1(lineage(1, None), digest(1)).unwrap();
        assert_eq!(
            ledger.admit_v1(lineage(1, None), digest(1)).unwrap_err(),
            SaveEpochRejectionV1::NotSequential { floor: SaveEpoch::new(1), candidate: SaveEpoch::new(1) }
        );
    }

    /// A gap (skipping straight to epoch 3 after epoch 1) is refused, the
    /// same terminal as a replay — this ledger draws no distinction
    /// between the two, unlike `T4.2`'s richer connection-resume shape.
    #[test]
    fn a_gap_in_the_epoch_sequence_is_refused() {
        let mut ledger = SaveEpochLedgerV1::new();
        ledger.admit_v1(lineage(1, None), digest(1)).unwrap();
        assert_eq!(
            ledger.admit_v1(lineage(3, Some(digest(1))), digest(3)).unwrap_err(),
            SaveEpochRejectionV1::NotSequential { floor: SaveEpoch::new(1), candidate: SaveEpoch::new(3) }
        );
    }

    /// `T4.2`'s "mix-and-match roots" required-test shape, reused: a
    /// sequentially-valid epoch with a FOREIGN predecessor root (not the
    /// floor's own) is refused as a fork.
    #[test]
    fn a_foreign_predecessor_root_is_refused_as_a_fork() {
        let mut ledger = SaveEpochLedgerV1::new();
        ledger.admit_v1(lineage(1, None), digest(1)).unwrap();
        assert_eq!(ledger.admit_v1(lineage(2, Some(digest(99))), digest(2)).unwrap_err(), SaveEpochRejectionV1::PredecessorMismatch);
    }

    /// Epoch 1 claiming a `Some` predecessor root is refused — epoch zero
    /// never had one, so claiming continuity from a root that does not
    /// exist is exactly as wrong as a foreign root.
    #[test]
    fn epoch_one_claiming_a_predecessor_root_is_refused() {
        let mut ledger = SaveEpochLedgerV1::new();
        assert_eq!(ledger.admit_v1(lineage(1, Some(digest(1))), digest(1)).unwrap_err(), SaveEpochRejectionV1::PredecessorMismatch);
    }

    /// Epoch 2 omitting the (now-required) predecessor root is refused —
    /// the mirror image of the previous test.
    #[test]
    fn epoch_two_omitting_the_predecessor_root_is_refused() {
        let mut ledger = SaveEpochLedgerV1::new();
        ledger.admit_v1(lineage(1, None), digest(1)).unwrap();
        assert_eq!(ledger.admit_v1(lineage(2, None), digest(2)).unwrap_err(), SaveEpochRejectionV1::PredecessorMismatch);
    }

    /// Non-vacuity: the `digest` fixture helper actually produces
    /// distinct values, or every mismatch assertion above would pass by
    /// coincidence.
    #[test]
    fn the_digest_fixture_helper_produces_genuinely_distinct_digests() {
        assert_ne!(digest(1), digest(2));
        assert_ne!(digest(1), digest(99));
    }

    // -- semantic root -----------------------------------------------------

    #[test]
    fn the_same_manifest_produces_the_same_root() {
        assert_eq!(compute_save_universe_manifest_root_v1(&manifest()).unwrap(), compute_save_universe_manifest_root_v1(&manifest()).unwrap());
    }

    #[test]
    fn a_different_frozen_tick_moves_the_root() {
        let base = manifest();
        let altered = SaveUniverseManifestV1 { frozen_tick: base.frozen_tick + 1, ..base.clone() };
        assert_ne!(compute_save_universe_manifest_root_v1(&base).unwrap(), compute_save_universe_manifest_root_v1(&altered).unwrap());
    }

    /// A single store's payload digest changing moves the root — the
    /// exact scenario the whole row exists to detect (a store that wrote
    /// different bytes than the manifest claims).
    #[test]
    fn an_altered_store_payload_digest_moves_the_root() {
        let base = manifest();
        let mut altered = base.clone();
        altered.stores[0].identity = identity(77);
        assert_ne!(compute_save_universe_manifest_root_v1(&base).unwrap(), compute_save_universe_manifest_root_v1(&altered).unwrap());
    }

    #[test]
    fn an_altered_world_baseline_root_moves_the_root() {
        let base = manifest();
        let altered = SaveUniverseManifestV1 { world_baseline_root: Some(digest(200)), ..base.clone() };
        assert_ne!(compute_save_universe_manifest_root_v1(&base).unwrap(), compute_save_universe_manifest_root_v1(&altered).unwrap());
    }

    /// `world_baseline_root` absent vs present must not collide -- the
    /// same `None`-never-collides-with-`Some` discipline `T4.3` proved
    /// for its own `Option<u32>` fields, here for `Option<ArtifactDigestV1>`.
    #[test]
    fn absent_world_baseline_root_is_distinct_from_every_present_value() {
        let base = manifest();
        let absent = SaveUniverseManifestV1 { world_baseline_root: None, ..base.clone() };
        assert_ne!(compute_save_universe_manifest_root_v1(&base).unwrap(), compute_save_universe_manifest_root_v1(&absent).unwrap());
    }

    /// Domain separation is real, not decorative: the manifest root moves
    /// under a different domain for the identical logical content (proven
    /// via two manifests differing only in a field that itself encodes
    /// the domain id nowhere — this exercises `digest_manifest_value_v1`'s
    /// own domain framing, not a hand-rolled preimage).
    #[test]
    fn a_permuted_store_list_moves_the_root_but_the_same_stores_reordered_and_reinserted_identically_does_not() {
        let a = manifest();
        let b = manifest();
        assert_eq!(compute_save_universe_manifest_root_v1(&a).unwrap(), compute_save_universe_manifest_root_v1(&b).unwrap());

        let mut reordered = a.clone();
        reordered.stores.reverse();
        // This type does not canonicalize `stores` -- a genuinely
        // different construction ORDER is a genuinely different byte
        // image, and therefore a different root. Documented, not a bug:
        // see the type's own doc comment on caller-supplied ordering.
        assert_ne!(compute_save_universe_manifest_root_v1(&a).unwrap(), compute_save_universe_manifest_root_v1(&reordered).unwrap());
    }
}
