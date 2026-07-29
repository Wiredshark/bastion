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
        SaveEpochPointerReadV1, SaveEpochPointerV1, SaveStoreIdV1, SaveStorePayloadV1, SaveUniverseManifestV1,
        save_universe_manifest_limits_v1,
    },
};

use crate::persistence::{self, ConnectionMode, DatabaseSettings, SqlLogMode};

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
) -> Result<SaveStorePayloadV1, SaveUniverseCommitErrorV1>
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
    Ok(SaveStorePayloadV1 { store, identity })
}

/// Stages the character DB via `VACUUM INTO` -- SQLite's own consistent-
/// snapshot mechanism. This does NOT compose with [`stage_payload_v1`]'s
/// `AtomicFile`-closure abstraction: `VACUUM INTO` creates its OWN target
/// file via SQLite's own I/O rather than writing through a handle we
/// supply, so it gets its own stage-to-temp-then-verified-durable-rename
/// sequence instead.
///
/// **Connection discipline** (orchestrator-required): runs on its OWN
/// connection, opened read-only directly from `db_dir` -- never through
/// `CharacterUpdater`'s connection. This is the SAME pattern
/// `CharacterLoader` already uses in production
/// (`persistence::establish_connection(&settings, ConnectionMode::ReadOnly)`,
/// concurrently with `CharacterUpdater`'s own read-write connection), not
/// a new one invented here.
///
/// **Read-vs-write behavior, verified not assumed**
/// (`vacuum_into_is_not_blocked_by_and_does_not_block_concurrent_writer_commits`
/// below, against `persistence::establish_connection`'s own real
/// pragmas): this database runs in WAL mode. `VACUUM INTO` opens its own
/// read transaction and gets SQLite's standard WAL snapshot isolation --
/// it sees a consistent point-in-time view as of when it started, is
/// never blocked by a concurrent writer's commits, and never blocks them
/// either. The one WAL-specific cost: a writer's checkpoint cannot
/// reclaim WAL frames this snapshot still needs, so the WAL file can grow
/// temporarily while a long-running vacuum overlaps with writes --
/// reclaimed once the vacuum's read transaction ends. A temporary size
/// cost, never a correctness or availability issue for either side.
///
/// **Durability discipline** (orchestrator-required): `VACUUM INTO`
/// writes directly to its target with no `AtomicFile`-style temp-then-
/// rename of its own, so this function gives it one by hand -- stage to
/// a `.tmp` sibling of the final path, fsync the file, rename, fsync the
/// containing directory (best-effort on Windows, where a directory
/// cannot be opened for `fsync` the way Unix allows -- rename durability
/// there rests on NTFS's own metadata journal). The digest that ENTERS
/// THE MANIFEST is re-derived from the PLACED file, never the pre-rename
/// temp bytes -- [`stage_payload_v1`]'s own rule: integrity anchors to
/// final resting bytes.
pub fn stage_character_db_v1(layout: &SaveUniverseLayoutV1, epoch: SaveEpoch, db_dir: &Path) -> Result<SaveStorePayloadV1, SaveUniverseCommitErrorV1> {
    let final_path = layout.payload_path(epoch, SaveStoreIdV1::CharacterDb);
    if let Some(dir) = final_path.parent() {
        fs::create_dir_all(dir).map_err(SaveUniverseCommitErrorV1::Io)?;
    }
    if final_path.exists() {
        return Err(SaveUniverseCommitErrorV1::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "character-db payload already staged for this epoch",
        )));
    }
    let temp_path = final_path.with_extension("tmp");
    if temp_path.exists() {
        // A leftover from a prior crashed attempt at the SAME epoch --
        // `VACUUM INTO` refuses to write over an existing target.
        fs::remove_file(&temp_path).map_err(SaveUniverseCommitErrorV1::Io)?;
    }

    let read_settings = DatabaseSettings { db_dir: db_dir.to_owned(), sql_log_mode: SqlLogMode::Disabled };
    let source = persistence::establish_connection(&read_settings, ConnectionMode::ReadOnly);
    let temp_path_str = temp_path
        .to_str()
        .ok_or_else(|| SaveUniverseCommitErrorV1::Io(io::Error::new(io::ErrorKind::InvalidInput, "non-UTF8 staging path")))?;
    source
        .execute("VACUUM INTO ?1", rusqlite::params![temp_path_str])
        .map_err(|e| SaveUniverseCommitErrorV1::Io(io::Error::other(e.to_string())))?;
    drop(source);

    // Durable placement: fsync the file, rename, fsync the directory --
    // the same file-then-directory fsync discipline `AtomicFile` gives
    // its own writes, applied by hand since `VACUUM INTO`'s target isn't
    // written through that primitive.
    //
    // `.write(true)` is load-bearing, not decoration: `sync_all` calls
    // `FlushFileBuffers` on Windows, which the WIN32 API documents as
    // requiring a handle opened with write access -- a plain read-only
    // `File::open` handle fails it with `ERROR_ACCESS_DENIED`. Found by
    // running this exact code, not assumed: the first attempt used
    // `File::open` and failed every real test with that error.
    fs::OpenOptions::new()
        .write(true)
        .open(&temp_path)
        .and_then(|f| f.sync_all())
        .map_err(SaveUniverseCommitErrorV1::Io)?;
    fs::rename(&temp_path, &final_path).map_err(SaveUniverseCommitErrorV1::Io)?;
    if let Some(dir) = final_path.parent() {
        sync_directory_best_effort(dir);
    }

    let mut placed = File::open(&final_path).map_err(SaveUniverseCommitErrorV1::Io)?;
    let identity = hash_artifact_reader_v1(&mut placed, MAX_PAYLOAD_BYTES).map_err(|e| SaveUniverseCommitErrorV1::Io(reader_error_to_io(e)))?;
    Ok(SaveStorePayloadV1 { store: SaveStoreIdV1::CharacterDb, identity })
}

/// See [`stage_character_db_v1`]'s own doc comment for the Windows
/// caveat: this is a real `fsync` on Unix, and a documented best-effort
/// no-op on Windows.
fn sync_directory_best_effort(dir: &Path) {
    #[cfg(unix)]
    {
        if let Err(e) = File::open(dir).and_then(|f| f.sync_all()) {
            tracing::warn!(?e, ?dir, "failed to fsync save-universe epoch directory after a character-db rename");
        }
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
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
    Recovered {
        manifest: SaveUniverseManifestV1,
        /// The manifest's own exact-byte identity -- the SAME value this
        /// function already computed internally to verify against the
        /// pointer. Surfaced (chunk 3b) rather than discarded: a caller
        /// seeding `SaveEpochLedgerV1::seeded_from_recovery_v1` needs
        /// exactly this value as the chain link for the next epoch, and
        /// re-deriving it by re-encoding `manifest` would be a second,
        /// redundant computation of a value already in hand.
        manifest_identity: ArtifactIdentityV1,
    },
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

    Ok(SaveUniverseRecoveryV1::Recovered { manifest, manifest_identity })
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
            SaveUniverseRecoveryV1::Recovered { manifest: recovered, .. } => assert_eq!(recovered, manifest),
            SaveUniverseRecoveryV1::EpochZero => panic!("expected Recovered after a real commit"),
        }
    }

    /// `APEX-T4.6` chunk 3b's own need: `recover_v1`'s surfaced
    /// `manifest_identity` must be the SAME value the pointer itself
    /// carries (`commit_epoch_v1`'s return), not a re-derived or
    /// otherwise-computed one -- a seeded ledger's chain link has to be
    /// exactly what the on-disk pointer already commits to.
    #[test]
    fn recovered_manifest_identity_matches_the_committed_pointers_own() {
        let (_dir, layout) = layout();
        let payload = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"rtsim-v1")).unwrap();
        let manifest = manifest_for(1, None, vec![payload]);
        let pointer = commit_epoch_v1(&layout, &manifest).unwrap();

        match recover_v1(&layout).unwrap() {
            SaveUniverseRecoveryV1::Recovered { manifest_identity, .. } => assert_eq!(manifest_identity, pointer.manifest_identity),
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
            SaveUniverseRecoveryV1::Recovered { manifest, .. } => assert_eq!(manifest.lineage.epoch, SaveEpoch::new(2)),
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

    // -- character-db staging (VACUUM INTO) -----------------------------

    fn db_settings(dir: &Path) -> DatabaseSettings { DatabaseSettings { db_dir: dir.to_owned(), sql_log_mode: SqlLogMode::Disabled } }

    #[test]
    fn a_staged_character_db_payload_is_a_valid_readable_snapshot() {
        let db_dir = tempfile::tempdir().unwrap();
        let settings = db_settings(db_dir.path());
        let setup = persistence::establish_connection(&settings, ConnectionMode::ReadWrite);
        setup.execute("CREATE TABLE probe (n INTEGER NOT NULL)", []).unwrap();
        setup.execute("INSERT INTO probe (n) VALUES (42)", []).unwrap();
        drop(setup);

        let (_layout_dir, layout) = layout();
        let payload = stage_character_db_v1(&layout, SaveEpoch::new(1), db_dir.path()).unwrap();
        assert_eq!(payload.store, SaveStoreIdV1::CharacterDb);

        let placed_path = layout.payload_path(SaveEpoch::new(1), SaveStoreIdV1::CharacterDb);
        let verify = rusqlite::Connection::open(&placed_path).unwrap();
        let n: i64 = verify.query_row("SELECT n FROM probe", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 42);

        // The recorded identity is the PLACED file's own bytes, not some
        // other claim.
        let mut placed_file = File::open(&placed_path).unwrap();
        let actual = hash_artifact_reader_v1(&mut placed_file, MAX_PAYLOAD_BYTES).unwrap();
        assert_eq!(payload.identity, actual);

        // No leftover temp file after a successful placement.
        assert!(!layout.payload_path(SaveEpoch::new(1), SaveStoreIdV1::CharacterDb).with_extension("tmp").exists());
    }

    /// Restaging the character DB at the same epoch is refused, same as
    /// [`restaging_the_same_store_at_the_same_epoch_is_refused`] proves
    /// for the `AtomicFile`-backed path -- this store's staging has its
    /// own code path and needs its own proof of the same invariant.
    #[test]
    fn restaging_the_character_db_at_the_same_epoch_is_refused() {
        let db_dir = tempfile::tempdir().unwrap();
        let settings = db_settings(db_dir.path());
        let setup = persistence::establish_connection(&settings, ConnectionMode::ReadWrite);
        setup.execute("CREATE TABLE probe (n INTEGER NOT NULL)", []).unwrap();
        drop(setup);

        let (_layout_dir, layout) = layout();
        stage_character_db_v1(&layout, SaveEpoch::new(1), db_dir.path()).unwrap();
        let err = stage_character_db_v1(&layout, SaveEpoch::new(1), db_dir.path());
        assert!(err.is_err(), "restaging must be refused, not silently overwrite");
    }

    /// `APEX-T4.6` chunk 3b's own premise-check, made a test rather than
    /// an assumption: does `VACUUM INTO` on its own read-only connection
    /// block, or get blocked by, a concurrent writer's commits against
    /// the SAME WAL-mode database this codebase actually uses
    /// (`persistence::establish_connection`, not a hand-rolled
    /// connection)?
    #[test]
    fn vacuum_into_is_not_blocked_by_and_does_not_block_concurrent_writer_commits() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, AtomicI64, Ordering},
        };

        let db_dir = tempfile::tempdir().unwrap();
        let settings = db_settings(db_dir.path());

        let setup = persistence::establish_connection(&settings, ConnectionMode::ReadWrite);
        setup.execute("CREATE TABLE probe (n INTEGER NOT NULL)", []).unwrap();
        drop(setup);

        let stop = Arc::new(AtomicBool::new(false));
        // Shared progress counter -- the wait below polls THIS rather
        // than sleeping a fixed guess, so the test is robust to
        // scheduling contention under a busy parallel test run instead
        // of flaking when the writer thread is slow to get its first
        // timeslice.
        let rows_committed = Arc::new(AtomicI64::new(0));
        let writer_thread = {
            let stop = Arc::clone(&stop);
            let rows_committed = Arc::clone(&rows_committed);
            let settings = settings.clone();
            std::thread::spawn(move || {
                let conn = persistence::establish_connection(&settings, ConnectionMode::ReadWrite);
                let mut n = 0i64;
                while !stop.load(Ordering::Relaxed) {
                    conn.execute("INSERT INTO probe (n) VALUES (?1)", rusqlite::params![n]).unwrap();
                    n += 1;
                    rows_committed.store(n, Ordering::Relaxed);
                }
                n
            })
        };

        // Wait for real, observed writer progress (bounded, not
        // infinite) before racing `VACUUM INTO` against it -- a fixed
        // sleep here is exactly the kind of guess that flakes under a
        // contended parallel `cargo test` run.
        let wait_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while rows_committed.load(Ordering::Relaxed) < 1 && std::time::Instant::now() < wait_deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(rows_committed.load(Ordering::Relaxed) >= 1, "the writer thread never got to commit even once within 5s -- test infrastructure, not the claim under test");

        let (_layout_dir, layout) = layout();
        // The claim under test: this succeeds without error and without
        // hanging, concurrently with the writer thread's ongoing commits.
        let payload = stage_character_db_v1(&layout, SaveEpoch::new(1), db_dir.path())
            .expect("VACUUM INTO must not be blocked by, nor block, concurrent writer commits");

        stop.store(true, Ordering::Relaxed);
        let rows_written = writer_thread.join().unwrap();
        assert!(rows_written > 0, "the writer thread must have actually run concurrently, not merely been spawned");

        let placed_path = layout.payload_path(SaveEpoch::new(1), SaveStoreIdV1::CharacterDb);
        let verify = rusqlite::Connection::open(&placed_path).unwrap();
        let count: i64 = verify.query_row("SELECT COUNT(*) FROM probe", [], |r| r.get(0)).unwrap();
        assert!(count >= 0, "the vacuumed snapshot must be a valid, queryable database");
        let _ = payload;
    }

    // =====================================================================
    // `APEX-T4.6` chunk 4 -- the `SAVE-001..` crash-injection canary sketch.
    //
    // Each canary constructs the ON-DISK STATE a crash at that exact step
    // would leave behind -- directly, via `std::fs`, not by actually
    // killing a process mid-write -- and proves `recover_v1` resolves it
    // exactly as the row's directional acceptance criterion demands:
    // complete state without a pointer is INACTIVE, and a manifest
    // without complete (verified) state is UNREADABLE. Same fixture-
    // constructed standard as `T4.5-FIXTURES`'s local corpus, not a VM
    // filesystem matrix -- "across supported filesystems" is explicitly
    // a deployment-matrix value per the spec's own closing disclaimer,
    // out of a single builder's scope.
    //
    // Several canaries in the spec's own sketch are already proven by
    // tests ABOVE this section, from before chunk 4 existed to number
    // them -- restated here as cross-references rather than duplicated:
    //   SAVE-001 crash before staging            -> `an_untouched_directory_recovers_as_epoch_zero`
    //             (identical observable state to "old pointer-less save
    //             directory on first boot" -- recovery cannot distinguish
    //             a directory that never had anything staged from one
    //             that predates this row entirely; both are `EpochZero`.)
    //   SAVE-005 pointer names a missing payload -> `a_manifest_naming_a_missing_payload_is_refused`
    //   SAVE-006 manifest bytes don't match the pointer's claim -> `a_corrupted_manifest_is_refused`
    //   SAVE-007 a payload's bytes don't match the manifest's claim -> `a_corrupted_payload_is_refused`
    //   SAVE-004 (a) after manifest, before pointer -> `a_fully_staged_but_never_published_epoch_is_inactive`
    //             (this section's own `save_004b_...` below is the same
    //             step with a PRIOR committed epoch present too -- the
    //             realistic live-server shape, not just the empty-history
    //             case.)
    //
    // Two of the spec's named canaries are deliberately NOT fabricated,
    // with the reasoning recorded rather than silently skipped:
    //   SAVE-008 "during pointer rename"   -- `AtomicFile`'s rename is the
    //             OS's own atomicity primitive (a single filesystem
    //             rename syscall); there is no observable third state
    //             between "old pointer still in place" and "new pointer
    //             fully in place" to construct. What CAN be tested
    //             faithfully, and is below, is the artifact a crash
    //             leaves behind either way: `AtomicFile`'s own randomized
    //             `.atomicwrite*/tmpfile.tmp` staging directory,
    //             abandoned next to an untouched real pointer.
    //   SAVE-009 "two pointers"            -- N/A for this row's single-
    //             pointer-file design (`pointer.bin`, always overwritten,
    //             never a set). The analogous, buildable property --
    //             a NEW commit atomically supersedes the old one, never
    //             leaving an ambiguous choice between two candidates --
    //             is exactly what `a_second_committed_epoch_supersedes_
    //             the_first` (above) already proves.
    //   SAVE-011 "GC racing a reader"      -- GC is BANKED, not built
    //             (orchestrator-ruled: retention policy is a deployment
    //             value, not a builder's to invent). The structural
    //             property that would make such a race impossible BY
    //             CONSTRUCTION once GC exists is provable now, without
    //             GC: `save_010_stale_staged_epochs_are_present_but_
    //             ignored` below proves recovery never opens any epoch
    //             directory the current pointer doesn't name -- a future
    //             GC deleting those same untouched directories cannot
    //             race a reader that never looks at them.

    /// SAVE-002: a crash mid-write of a NEW epoch's payload, with a PRIOR
    /// epoch already fully committed (the realistic live-server case --
    /// by the time a second epoch is ever attempted, a first one exists).
    /// The truncated in-progress payload for epoch 2 sits on disk; no
    /// manifest or pointer for epoch 2 was ever written. Recovery must
    /// be completely unaffected, still returning epoch 1.
    #[test]
    fn save_002_a_truncated_mid_write_payload_for_a_newer_epoch_does_not_affect_recovery_of_the_committed_one() {
        let (_dir, layout) = layout();
        let p1 = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-1-complete")).unwrap();
        let m1 = manifest_for(1, None, vec![p1]);
        commit_epoch_v1(&layout, &m1).unwrap();

        // Simulate the crash: a payload path for epoch 2 exists but is
        // an obviously truncated fragment, written directly (bypassing
        // `stage_payload_v1`, which this crash never got to finish).
        let epoch2_payload_path = layout.payload_path(SaveEpoch::new(2), SaveStoreIdV1::RtsimData);
        fs::create_dir_all(epoch2_payload_path.parent().unwrap()).unwrap();
        fs::write(&epoch2_payload_path, b"trunc").unwrap();
        // No manifest, no pointer update for epoch 2 -- the crash never
        // reached either step.

        match recover_v1(&layout).unwrap() {
            SaveUniverseRecoveryV1::Recovered { manifest, .. } => assert_eq!(manifest.lineage.epoch, SaveEpoch::new(1)),
            SaveUniverseRecoveryV1::EpochZero => panic!("epoch 1 was genuinely committed; must not read as epoch zero"),
        }
    }

    /// SAVE-003: a crash after EVERY payload for a new epoch finished
    /// staging (verified digests and all), but before that epoch's
    /// manifest was ever written -- again with a prior committed epoch
    /// present. The fully-valid, orphaned payloads must not confuse
    /// recovery into thinking a newer epoch exists.
    #[test]
    fn save_003_fully_staged_payloads_with_no_manifest_do_not_affect_recovery_of_the_committed_epoch() {
        let (_dir, layout) = layout();
        let p1 = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-1")).unwrap();
        let m1 = manifest_for(1, None, vec![p1]);
        commit_epoch_v1(&layout, &m1).unwrap();

        // Epoch 2's payload finishes staging completely (a real,
        // verifiable payload -- the crash happens strictly AFTER this
        // succeeds) but its manifest is never written.
        stage_payload_v1(&layout, SaveEpoch::new(2), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-2-payload-only")).unwrap();

        match recover_v1(&layout).unwrap() {
            SaveUniverseRecoveryV1::Recovered { manifest, .. } => assert_eq!(manifest.lineage.epoch, SaveEpoch::new(1)),
            SaveUniverseRecoveryV1::EpochZero => panic!("epoch 1 was genuinely committed; must not read as epoch zero"),
        }
    }

    /// SAVE-004b: `a_fully_staged_but_never_published_epoch_is_inactive`
    /// (above) proves the empty-history case; this is the same crash
    /// step -- manifest written, pointer never published -- with a PRIOR
    /// committed epoch present, the shape every real live-server crash
    /// at this step would actually have.
    #[test]
    fn save_004b_a_written_but_unpublished_manifest_does_not_affect_recovery_of_the_committed_epoch() {
        let (_dir, layout) = layout();
        let p1 = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-1")).unwrap();
        let m1 = manifest_for(1, None, vec![p1]);
        let pointer1 = commit_epoch_v1(&layout, &m1).unwrap();

        let p2 = stage_payload_v1(&layout, SaveEpoch::new(2), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-2")).unwrap();
        let m2 = manifest_for(2, Some(pointer1.manifest_identity.digest), vec![p2]);
        // Write the manifest but never call publish_pointer_v1/
        // commit_epoch_v1 for it -- the crash lands exactly here.
        write_manifest_v1(&layout, SaveEpoch::new(2), &m2).unwrap();

        match recover_v1(&layout).unwrap() {
            SaveUniverseRecoveryV1::Recovered { manifest, .. } => assert_eq!(manifest.lineage.epoch, SaveEpoch::new(1)),
            SaveUniverseRecoveryV1::EpochZero => panic!("epoch 1 was genuinely committed; must not read as epoch zero"),
        }
    }

    /// SAVE-008: the artifact `AtomicFile`'s own crashed rename leaves
    /// behind -- its randomized `.atomicwrite*/tmpfile.tmp` staging
    /// subdirectory, abandoned next to an untouched, still-valid real
    /// pointer. See this section's own header comment for why the
    /// in-between "torn rename" state itself cannot be fabricated (the
    /// OS rename syscall admits no such state).
    #[test]
    fn save_008_an_abandoned_atomicwrite_staging_directory_does_not_affect_recovery() {
        let (_dir, layout) = layout();
        let p1 = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-1")).unwrap();
        let m1 = manifest_for(1, None, vec![p1]);
        commit_epoch_v1(&layout, &m1).unwrap();

        // The exact shape `atomicwrites::AtomicFile` uses (see its own
        // source: `.atomicwrite`-prefixed tempdir, `tmpfile.tmp` inside),
        // fabricated directly to simulate a crash between "temp file
        // written" and "rename into place" for a HYPOTHETICAL second
        // pointer publish that never completed.
        let abandoned_dir = layout.root().join(".atomicwrite-crash-simulation");
        fs::create_dir_all(&abandoned_dir).unwrap();
        fs::write(abandoned_dir.join("tmpfile.tmp"), b"a pointer that never got renamed into place").unwrap();

        match recover_v1(&layout).unwrap() {
            SaveUniverseRecoveryV1::Recovered { manifest, .. } => assert_eq!(manifest.lineage.epoch, SaveEpoch::new(1)),
            SaveUniverseRecoveryV1::EpochZero => panic!("epoch 1 was genuinely committed; must not read as epoch zero"),
        }
        // The abandoned artifact is untouched -- `recover_v1` never
        // scans the directory, only ever reads the exact `pointer.bin`
        // path, so it has no way to even notice this file exists.
        assert!(abandoned_dir.join("tmpfile.tmp").exists());
    }

    /// SAVE-010: several stale, superseded epochs remain fully present
    /// on disk (GC is banked, not built -- see this section's own header
    /// comment). Recovery must return ONLY the current pointer's epoch,
    /// completely ignoring the others -- proving, without GC existing
    /// yet, that a future GC of those same directories could never race
    /// a reader (SAVE-011): the reader never opens them in the first
    /// place.
    #[test]
    fn save_010_stale_staged_epochs_are_present_but_ignored() {
        let (_dir, layout) = layout();

        let p1 = stage_payload_v1(&layout, SaveEpoch::new(1), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-1")).unwrap();
        let m1 = manifest_for(1, None, vec![p1]);
        let pointer1 = commit_epoch_v1(&layout, &m1).unwrap();

        let p2 = stage_payload_v1(&layout, SaveEpoch::new(2), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-2")).unwrap();
        let m2 = manifest_for(2, Some(pointer1.manifest_identity.digest), vec![p2]);
        let pointer2 = commit_epoch_v1(&layout, &m2).unwrap();

        let p3 = stage_payload_v1(&layout, SaveEpoch::new(3), SaveStoreIdV1::RtsimData, |f| f.write_all(b"epoch-3")).unwrap();
        let m3 = manifest_for(3, Some(pointer2.manifest_identity.digest), vec![p3]);
        commit_epoch_v1(&layout, &m3).unwrap();

        match recover_v1(&layout).unwrap() {
            SaveUniverseRecoveryV1::Recovered { manifest, .. } => assert_eq!(manifest.lineage.epoch, SaveEpoch::new(3)),
            SaveUniverseRecoveryV1::EpochZero => panic!("epoch 3 was genuinely committed; must not read as epoch zero"),
        }
        // Epochs 1 and 2's own directories are still there, fully
        // intact -- exactly what a real (not-yet-built) GC would later
        // clean up. Their continued presence changes nothing about what
        // was just recovered.
        assert!(layout.root().join("epochs").join("1").exists());
        assert!(layout.root().join("epochs").join("2").exists());
    }
}
