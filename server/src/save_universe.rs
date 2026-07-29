//! `APEX-T4.6` chunk 2 — the real staged-write-then-publish commit
//! protocol: per-store payload staging, manifest write, and the single
//! atomic pointer rename that is the sole commit point.
//!
//! **Chunk scope, self-sized per the orchestrator-approved 4-chunk
//! split.** This chunk is the store-agnostic MECHANISM, fully testable
//! against synthetic byte payloads on a `tempdir` -- deliberately NOT
//! wired into `RtSim::save`/`CharacterUpdater`'s real save trigger yet
//! (chunk 3's job), and does not subsume `rtsim::data::Data::
//! world_baseline_root` (also chunk 3, per the orchestrator's ruling:
//! "never remove the old path before the new one is the actual reader").
//! Garbage-collecting superseded epochs is BANKED, not built here --
//! "policy-safe acknowledgment" and retention counts are deployment
//! values this program's own rule says a builder doesn't invent (same
//! law as `T4.5`'s four `PendingRuling` policies); the hook point is
//! named in this module's doc, no number attached.
//!
//! **Reused, not reinvented.** Per-store staging uses the exact
//! `AtomicFile` primitive `RtSim::save` already writes through
//! (`server/src/rtsim/mod.rs:729`) -- the pointer file is the only NEW
//! primitive this row adds. `common::apex::save_universe` owns every
//! type this module writes to disk; this module is pure I/O orchestration
//! around that data model, adding no parallel schema.
//!
//! **Layout.** Given a root directory (chunk 3 places it as a sibling of
//! `saves/`/`rtsim/` under the server's existing `data_dir`):
//! ```text
//! <root>/epochs/<epoch>/manifest.bin
//! <root>/epochs/<epoch>/payload-<store-id>.bin
//! <root>/pointer.bin
//! ```
//! Epoch directories are never reused (a fresh directory per epoch), so
//! staged payloads and the manifest are written with
//! `OverwriteBehavior::DisallowOverwrite` -- a second staging attempt at
//! the same epoch is a bug this module refuses to paper over. The
//! pointer path IS reused every commit, so it alone uses
//! `OverwriteBehavior::AllowOverwrite`; its atomic rename is the row's
//! one and only commit point.
//!
//! **Both directions of the row's acceptance criterion hold structurally,
//! not by convention:** [`recover_v1`] never inspects any epoch directory
//! other than the one the CURRENT pointer names, so staged-but-uncommitted
//! state is invisible to it ("complete state without a pointer is
//! inactive"); and it refuses (a typed error, never a partial return) the
//! moment the manifest's own identity or any payload's identity fails to
//! match what was claimed ("a manifest without complete state is
//! unreadable").

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use atomicwrites::{AtomicFile, OverwriteBehavior};
use common::apex::{
    digest::{ArtifactIdentityV1, ArtifactReaderErrorV1, hash_artifact_bytes_v1, hash_artifact_reader_v1},
    identity::SaveEpoch,
    manifest::{ManifestCodecErrorV1, ManifestSchemaErrorV1, decode_manifest_v1, encode_manifest_v1},
    save_universe::{
        SaveEpochPointerReadV1, SaveEpochPointerV1, SaveStoreIdV1, SaveUniverseManifestV1, save_universe_manifest_limits_v1,
    },
};

/// A generous streaming ceiling, not a real limit: staged payloads
/// (a character DB snapshot, an rtsim blob) are expected to be at most a
/// few hundred MB in any real deployment. This exists so a hostile or
/// runaway stream cannot exhaust memory unbounded, per
/// `hash_artifact_reader_v1`'s own contract -- it is not a production
/// value this row invents, it is `hash_artifact_reader_v1`'s existing
/// safety valve given a deliberately high ceiling.
const MAX_PAYLOAD_BYTES: u64 = 1 << 34;

fn io_from_atomic<E: Into<io::Error>>(e: atomicwrites::Error<E>) -> io::Error {
    match e {
        atomicwrites::Error::Internal(io_err) => io_err,
        atomicwrites::Error::User(user_err) => user_err.into(),
    }
}

/// The root directory layout this row's staged commits live under. See
/// the module doc for the exact path shape.
#[derive(Clone, Debug)]
pub struct SaveUniverseLayoutV1 {
    root: PathBuf,
}

impl SaveUniverseLayoutV1 {
    pub fn new(root: PathBuf) -> Self { Self { root } }

    pub fn root(&self) -> &Path { &self.root }

    fn epoch_dir(&self, epoch: SaveEpoch) -> PathBuf { self.root.join("epochs").join(epoch.get().to_string()) }

    fn manifest_path(&self, epoch: SaveEpoch) -> PathBuf { self.epoch_dir(epoch).join("manifest.bin") }

    fn payload_path(&self, epoch: SaveEpoch, store: SaveStoreIdV1) -> PathBuf {
        self.epoch_dir(epoch).join(format!("payload-{}.bin", store.as_u16()))
    }

    fn pointer_path(&self) -> PathBuf { self.root.join("pointer.bin") }
}

/// Every way staging or committing an epoch can fail. `Io` covers both
/// this module's own filesystem calls and `AtomicFile`'s own internal
/// failures (unwrapped via [`io_from_atomic`] rather than left nested).
#[derive(Debug)]
pub enum SaveUniverseCommitErrorV1 {
    Io(io::Error),
    ManifestEncode(ManifestCodecErrorV1),
}

impl core::fmt::Display for SaveUniverseCommitErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::ManifestEncode(e) => write!(f, "manifest encode error: {e:?}"),
        }
    }
}

impl std::error::Error for SaveUniverseCommitErrorV1 {}

/// Stages one store's payload for `epoch`: writes `write` through
/// `AtomicFile` (the same primitive `RtSim::save` already uses), then
/// re-opens and hashes the FLUSHED file -- the manifest's claimed digest
/// is always derived from bytes actually on disk post-flush, never from
/// what the writer merely intended to write.
pub fn stage_payload_v1<F>(
    layout: &SaveUniverseLayoutV1,
    epoch: SaveEpoch,
    store: SaveStoreIdV1,
    write: F,
) -> Result<common::apex::save_universe::SaveStorePayloadV1, SaveUniverseCommitErrorV1>
where
    F: FnOnce(&mut File) -> io::Result<()>,
{
    let path = layout.payload_path(epoch, store);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(SaveUniverseCommitErrorV1::Io)?;
    }
    AtomicFile::new(&path, OverwriteBehavior::DisallowOverwrite)
        .write(write)
        .map_err(io_from_atomic)
        .map_err(SaveUniverseCommitErrorV1::Io)?;

    let mut file = File::open(&path).map_err(SaveUniverseCommitErrorV1::Io)?;
    let identity = hash_artifact_reader_v1(&mut file, MAX_PAYLOAD_BYTES)
        .map_err(|e| SaveUniverseCommitErrorV1::Io(reader_error_to_io(e)))?;
    Ok(common::apex::save_universe::SaveStorePayloadV1 { store, identity })
}

fn reader_error_to_io(e: ArtifactReaderErrorV1) -> io::Error {
    match e {
        ArtifactReaderErrorV1::Io(io_err) => io_err,
        ArtifactReaderErrorV1::Digest(digest_err) => io::Error::other(format!("{digest_err}")),
    }
}

/// Writes `manifest` for `epoch`, flushed through `AtomicFile`, and
/// returns the exact-byte identity of what actually landed on disk --
/// the same "verify post-flush" discipline [`stage_payload_v1`] uses,
/// and exactly the identity [`publish_pointer_v1`]'s pointer must bind.
pub fn write_manifest_v1(
    layout: &SaveUniverseLayoutV1,
    epoch: SaveEpoch,
    manifest: &SaveUniverseManifestV1,
) -> Result<ArtifactIdentityV1, SaveUniverseCommitErrorV1> {
    let bytes = encode_manifest_v1(manifest, &save_universe_manifest_limits_v1()).map_err(SaveUniverseCommitErrorV1::ManifestEncode)?;
    let path = layout.manifest_path(epoch);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(SaveUniverseCommitErrorV1::Io)?;
    }
    AtomicFile::new(&path, OverwriteBehavior::DisallowOverwrite)
        .write(|f| f.write_all(&bytes))
        .map_err(io_from_atomic)
        .map_err(SaveUniverseCommitErrorV1::Io)?;
    Ok(hash_artifact_bytes_v1(&bytes))
}

/// Publishes the epoch pointer -- the row's sole commit point. Unlike
/// staged payloads and the manifest, the pointer path IS reused every
/// commit, so this is the one write in the whole protocol that allows
/// overwrite; its atomic rename is what makes an epoch "the" current one.
pub fn publish_pointer_v1(layout: &SaveUniverseLayoutV1, pointer: SaveEpochPointerV1) -> Result<(), SaveUniverseCommitErrorV1> {
    let bytes = encode_manifest_v1(&pointer, &save_universe_manifest_limits_v1()).map_err(SaveUniverseCommitErrorV1::ManifestEncode)?;
    let path = layout.pointer_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(SaveUniverseCommitErrorV1::Io)?;
    }
    AtomicFile::new(&path, OverwriteBehavior::AllowOverwrite)
        .write(|f| f.write_all(&bytes))
        .map_err(io_from_atomic)
        .map_err(SaveUniverseCommitErrorV1::Io)?;
    Ok(())
}

/// Writes the manifest, then publishes the pointer that commits it --
/// the two steps the spec names as adjacent ("write and flush... then
/// atomically publish"), composed once here so a caller cannot
/// accidentally publish a pointer to a manifest it never actually wrote.
pub fn commit_epoch_v1(layout: &SaveUniverseLayoutV1, manifest: &SaveUniverseManifestV1) -> Result<SaveEpochPointerV1, SaveUniverseCommitErrorV1> {
    let epoch = manifest.lineage.epoch;
    let manifest_identity = write_manifest_v1(layout, epoch, manifest)?;
    let pointer = SaveEpochPointerV1 { epoch, manifest_identity };
    publish_pointer_v1(layout, pointer)?;
    Ok(pointer)
}

/// Every way recovery can fail. Each variant is the exact spot the row's
/// two-directional acceptance criterion bites: a decode failure or an
/// identity mismatch anywhere in the chain refuses recovery rather than
/// returning a partial or best-effort result.
#[derive(Debug)]
pub enum SaveUniverseRecoveryErrorV1 {
    Io(io::Error),
    PointerDecode(ManifestSchemaErrorV1),
    ManifestDecode(ManifestSchemaErrorV1),
    /// The manifest file's own bytes don't match the identity the
    /// pointer named for it -- a torn or corrupted manifest write.
    ManifestIdentityMismatch { expected: ArtifactIdentityV1, actual: ArtifactIdentityV1 },
    /// A store the manifest lists has no payload file on disk at all.
    PayloadMissing { store: SaveStoreIdV1 },
    /// A store payload's on-disk bytes don't match the identity the
    /// manifest claims for it.
    PayloadIdentityMismatch { store: SaveStoreIdV1, expected: ArtifactIdentityV1, actual: ArtifactIdentityV1 },
}

impl core::fmt::Display for SaveUniverseRecoveryErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::PointerDecode(e) => write!(f, "pointer decode error: {e:?}"),
            Self::ManifestDecode(e) => write!(f, "manifest decode error: {e:?}"),
            Self::ManifestIdentityMismatch { .. } => write!(f, "manifest identity does not match the pointer"),
            Self::PayloadMissing { store } => write!(f, "store {store:?} has no payload file for the current epoch"),
            Self::PayloadIdentityMismatch { store, .. } => write!(f, "store {store:?}'s payload identity does not match the manifest"),
        }
    }
}

impl std::error::Error for SaveUniverseRecoveryErrorV1 {}

/// Reads the pointer file, classifying its absence as
/// [`SaveEpochPointerReadV1::NeverPublished`] (this row's own migration
/// law: a pointer-less directory is epoch zero, never corruption) rather
/// than an I/O error. A pointer file that EXISTS but fails to decode is a
/// different problem -- a real error, not silently treated as absent.
pub fn read_pointer_v1(layout: &SaveUniverseLayoutV1) -> Result<SaveEpochPointerReadV1, SaveUniverseRecoveryErrorV1> {
    match fs::read(layout.pointer_path()) {
        Ok(bytes) => {
            let pointer: SaveEpochPointerV1 =
                decode_manifest_v1(&bytes, &save_universe_manifest_limits_v1()).map_err(SaveUniverseRecoveryErrorV1::PointerDecode)?;
            Ok(SaveEpochPointerReadV1::Published(pointer))
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(SaveEpochPointerReadV1::NeverPublished),
        Err(e) => Err(SaveUniverseRecoveryErrorV1::Io(e)),
    }
}

/// What a recovery attempt resolves to. `EpochZero` is not an error --
/// see [`read_pointer_v1`]'s own doc comment.
#[derive(Debug, PartialEq)]
pub enum SaveUniverseRecoveryV1 {
    EpochZero,
    Recovered { manifest: SaveUniverseManifestV1 },
}

/// The full recovery path: read the pointer, then admit ONLY a manifest
/// whose own identity matches what the pointer named AND whose every
/// listed store payload matches the manifest's own claim. Never scans
/// any epoch directory the current pointer doesn't name -- staged or
/// superseded epochs are simply invisible here, which is what makes
/// "complete state without a pointer is inactive" true structurally
/// rather than by discipline.
pub fn recover_v1(layout: &SaveUniverseLayoutV1) -> Result<SaveUniverseRecoveryV1, SaveUniverseRecoveryErrorV1> {
    let pointer = match read_pointer_v1(layout)? {
        SaveEpochPointerReadV1::NeverPublished => return Ok(SaveUniverseRecoveryV1::EpochZero),
        SaveEpochPointerReadV1::Published(p) => p,
    };

    let manifest_path = layout.manifest_path(pointer.epoch);
    let manifest_bytes = fs::read(&manifest_path).map_err(SaveUniverseRecoveryErrorV1::Io)?;
    let manifest_identity = hash_artifact_bytes_v1(&manifest_bytes);
    if manifest_identity != pointer.manifest_identity {
        return Err(SaveUniverseRecoveryErrorV1::ManifestIdentityMismatch { expected: pointer.manifest_identity, actual: manifest_identity });
    }
    let manifest: SaveUniverseManifestV1 =
        decode_manifest_v1(&manifest_bytes, &save_universe_manifest_limits_v1()).map_err(SaveUniverseRecoveryErrorV1::ManifestDecode)?;

    for store_payload in &manifest.stores {
        let payload_path = layout.payload_path(pointer.epoch, store_payload.store);
        let mut file = File::open(&payload_path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                SaveUniverseRecoveryErrorV1::PayloadMissing { store: store_payload.store }
            } else {
                SaveUniverseRecoveryErrorV1::Io(e)
            }
        })?;
        let actual = hash_artifact_reader_v1(&mut file, MAX_PAYLOAD_BYTES).map_err(|e| SaveUniverseRecoveryErrorV1::Io(reader_error_to_io(e)))?;
        if actual != store_payload.identity {
            return Err(SaveUniverseRecoveryErrorV1::PayloadIdentityMismatch {
                store: store_payload.store,
                expected: store_payload.identity,
                actual,
            });
        }
    }

    Ok(SaveUniverseRecoveryV1::Recovered { manifest })
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apex::digest::{ContentIdentityV1, hash_artifact_bytes_v1};
    use common::apex::save_universe::{SaveEpochLineageV1, SaveStorePayloadV1};
    use common::apex::scalar::SchemaVersion;
    use common::apex::subsystem::descriptor::SubsystemDescriptorV1;
    use common::apex::subsystem::slot::SubsystemSlotIdV1;

    fn layout() -> (tempfile::TempDir, SaveUniverseLayoutV1) {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = SaveUniverseLayoutV1::new(dir.path().to_owned());
        (dir, layout)
    }

    fn descriptor(seed: &[u8]) -> SubsystemDescriptorV1 {
        SubsystemDescriptorV1 {
            slot: SubsystemSlotIdV1::Content,
            schema: SchemaVersion::new(1),
            content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(seed), semantic: None },
        }
    }

    fn manifest_for(epoch: u64, predecessor_root: Option<common::apex::digest::ArtifactDigestV1>, stores: Vec<SaveStorePayloadV1>) -> SaveUniverseManifestV1 {
        SaveUniverseManifestV1 {
            lineage: SaveEpochLineageV1 { epoch: SaveEpoch::new(epoch), predecessor_root },
            frozen_tick: 100 * epoch,
            stores,
            world_baseline_root: None,
            descriptors: vec![descriptor(b"chunk2")],
            migration_journal_digest: None,
        }
    }

    // -- staging -----------------------------------------------------------

    #[test]
    fn a_staged_payload_is_readable_back_with_the_bytes_it_was_given() {
        let (_dir, layout) = layout();
        let payload = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"rtsim-bytes")).unwrap();
        assert_eq!(payload.store, SaveStoreIdV1::RtsimData);
        assert_eq!(payload.identity, hash_artifact_bytes_v1(b"rtsim-bytes"));
    }

    /// Staging the SAME store at the SAME epoch twice is refused -- an
    /// epoch directory is never reused, per the module's own layout
    /// contract.
    #[test]
    fn restaging_the_same_store_at_the_same_epoch_is_refused() {
        let (_dir, layout) = layout();
        stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::CharacterDb, |f| f.write_all(b"first")).unwrap();
        let err = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::CharacterDb, |f| f.write_all(b"second"));
        assert!(err.is_err(), "restaging must be refused, not silently overwrite");
    }

    /// The two stores at the same epoch land at distinct paths and don't
    /// collide with each other.
    #[test]
    fn two_stores_at_the_same_epoch_stage_independently() {
        let (_dir, layout) = layout();
        let a = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::CharacterDb, |f| f.write_all(b"db")).unwrap();
        let b = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"rtsim")).unwrap();
        assert_ne!(a.identity, b.identity);
    }

    // -- manifest + pointer commit ------------------------------------------

    #[test]
    fn commit_then_recover_round_trips_the_manifest() {
        let (_dir, layout) = layout();
        let payload = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"rtsim-v1")).unwrap();
        let manifest = manifest_for(1, None, vec![payload]);

        let pointer = commit_epoch_v1(&layout, &manifest).unwrap();
        assert_eq!(pointer.epoch, SaveEpoch::new(1));

        match recover_v1(&layout).unwrap() {
            SaveUniverseRecoveryV1::Recovered { manifest: recovered } => assert_eq!(recovered, manifest),
            SaveUniverseRecoveryV1::EpochZero => panic!("expected Recovered after a real commit"),
        }
    }

    /// The row's own migration law, exercised end to end through this
    /// module's real filesystem read (not just `common`'s pure
    /// classifier): a directory that never had anything committed reads
    /// as epoch zero, not an error.
    #[test]
    fn an_untouched_directory_recovers_as_epoch_zero() {
        let (_dir, layout) = layout();
        assert_eq!(recover_v1(&layout).unwrap(), SaveUniverseRecoveryV1::EpochZero);
    }

    /// A second epoch, correctly chained, supersedes the first as the
    /// current pointer -- recovery only ever sees the LATEST commit.
    #[test]
    fn a_second_committed_epoch_supersedes_the_first() {
        let (_dir, layout) = layout();
        let p1 = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-1")).unwrap();
        let m1 = manifest_for(1, None, vec![p1]);
        commit_epoch_v1(&layout, &m1).unwrap();
        let m1_root = hash_artifact_bytes_v1(&encode_manifest_v1(&m1, &save_universe_manifest_limits_v1()).unwrap()).digest;

        let p2 = stage_payload_v1(&layout, SaveEpoch::new(2), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-2")).unwrap();
        let m2 = manifest_for(2, Some(m1_root), vec![p2]);
        commit_epoch_v1(&layout, &m2).unwrap();

        match recover_v1(&layout).unwrap() {
            SaveUniverseRecoveryV1::Recovered { manifest } => assert_eq!(manifest.lineage.epoch, SaveEpoch::new(2)),
            SaveUniverseRecoveryV1::EpochZero => panic!("expected epoch 2"),
        }
    }

    // -- the two-directional acceptance criterion ----------------------------

    /// "A manifest without complete state must be unreadable": a
    /// committed pointer whose manifest names a store payload that was
    /// never actually staged is refused, not silently treated as
    /// partially loaded.
    #[test]
    fn a_manifest_naming_a_missing_payload_is_refused() {
        let (_dir, layout) = layout();
        // Fabricate a manifest claiming a payload that was never staged.
        let phantom = SaveStorePayloadV1 { store: SaveStoreIdV1::CharacterDb, identity: hash_artifact_bytes_v1(b"never-written") };
        let manifest = manifest_for(1, None, vec![phantom]);
        commit_epoch_v1(&layout, &manifest).unwrap();

        let err = recover_v1(&layout).unwrap_err();
        assert!(matches!(err, SaveUniverseRecoveryErrorV1::PayloadMissing { store: SaveStoreIdV1::CharacterDb }));
    }

    /// A staged payload whose bytes were corrupted after staging (so its
    /// on-disk digest no longer matches the manifest's claim) is refused,
    /// not silently loaded as if nothing happened.
    #[test]
    fn a_corrupted_payload_is_refused() {
        let (_dir, layout) = layout();
        let payload = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"original-bytes")).unwrap();
        let manifest = manifest_for(1, None, vec![payload]);
        commit_epoch_v1(&layout, &manifest).unwrap();

        // Corrupt the staged payload file directly, bypassing this
        // module's own write path -- simulating bitrot/a torn write that
        // slipped past the atomic rename.
        std::fs::write(layout.payload_path(SaveEpoch::new(1), SaveStoreIdV1::RtsimData), b"corrupted!!").unwrap();

        let err = recover_v1(&layout).unwrap_err();
        assert!(matches!(err, SaveUniverseRecoveryErrorV1::PayloadIdentityMismatch { store: SaveStoreIdV1::RtsimData, .. }));
    }

    /// A manifest file corrupted after commit (its bytes no longer match
    /// the identity the pointer named) is refused before even attempting
    /// to decode it as a manifest.
    #[test]
    fn a_corrupted_manifest_is_refused() {
        let (_dir, layout) = layout();
        let payload = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"bytes")).unwrap();
        let manifest = manifest_for(1, None, vec![payload]);
        commit_epoch_v1(&layout, &manifest).unwrap();

        std::fs::write(layout.manifest_path(SaveEpoch::new(1)), b"not-a-real-manifest").unwrap();

        let err = recover_v1(&layout).unwrap_err();
        assert!(matches!(err, SaveUniverseRecoveryErrorV1::ManifestIdentityMismatch { .. }));
    }

    /// "Complete state without a pointer must be inactive": staging a
    /// full, internally-consistent epoch's payloads and manifest WITHOUT
    /// ever calling `publish_pointer_v1` must still recover as epoch
    /// zero -- recovery never discovers an epoch directory on its own.
    #[test]
    fn a_fully_staged_but_never_published_epoch_is_inactive() {
        let (_dir, layout) = layout();
        let payload = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"staged-only")).unwrap();
        let manifest = manifest_for(1, None, vec![payload]);
        // Deliberately write the manifest but never call
        // commit_epoch_v1/publish_pointer_v1 -- the manifest exists,
        // fully self-consistent, but nothing points at it.
        write_manifest_v1(&layout, SaveEpoch::new(1), &manifest).unwrap();

        assert_eq!(recover_v1(&layout).unwrap(), SaveUniverseRecoveryV1::EpochZero);
    }

    /// A pointer file that exists but fails to decode (garbage bytes) is
    /// a real error, not silently treated the same as "never published".
    #[test]
    fn an_undecodable_pointer_file_is_a_real_error_not_epoch_zero() {
        let (_dir, layout) = layout();
        std::fs::create_dir_all(layout.root()).unwrap();
        std::fs::write(layout.pointer_path(), b"garbage-not-a-pointer").unwrap();

        let err = recover_v1(&layout).unwrap_err();
        assert!(matches!(err, SaveUniverseRecoveryErrorV1::PointerDecode(_)));
    }
}
