//! `APEX-T4.4` — non-authoritative existing-save inventory.
//!
//! Diagnose what is on disk without certifying it and without writing to
//! it. The three stores this build persists have no shared descriptor, so
//! there is no artifact today that answers "what is in this save
//! directory" without opening each store with the current binary's
//! assumptions — which is exactly what you cannot do to a save the
//! current binary is failing to load.
//!
//! Two refusals hold the row up.
//!
//! **It never infers a common tick or checkpoint.** Consistency is
//! [`SaveConsistencyV1`], which has ONE variant. A `Coherent` variant
//! would be produced by a coherent-LOOKING fixture and would then be read
//! as "the stores agree", which nothing here checks. `T4.6` is what
//! creates cross-store coherence; until it exists, an inventory that
//! claimed it would be lying.
//!
//! **It does not open a store with the current binary's types.**
//! `Data::from_reader` rejects anything that is not `CURRENT_VERSION`, so
//! it cannot tell a version-11 save from garbage; the inventory uses
//! `Data::probe_version_v1`, which reads the version field alone and
//! therefore reports a FUTURE version rather than failing on it. SQLite
//! is opened `mode=ro&immutable=1`, which is what keeps the read from
//! creating a `-wal`/`-shm` sidecar next to a save being diagnosed.

use common::apex::digest::{ArtifactIdentityV1, hash_artifact_bytes_v1};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A store this build actually persists.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SaveStoreKindV1 {
    /// `saves/db.sqlite` — character and player persistence, migrated by
    /// refinery.
    CharacterDb,
    /// `rtsim/data.dat` — msgpack rtsim world state, carrying its own
    /// `version` field.
    RtsimData,
    /// `rtsim/data.ron_backup*` — a file the loader moved aside after a
    /// failed read. Its PRESENCE is the diagnosis: it means a previous
    /// boot could not load rtsim and started fresh.
    RtsimBackup,
    /// `terrain/chunk_X_Y.dat` — persisted per-chunk block diffs.
    TerrainChunk,
}

impl SaveStoreKindV1 {
    /// The stores whose absence is worth reporting. Backups and terrain
    /// chunks are not here: a save with no rtsim backup is a save that
    /// never failed, and a world with no edited chunks has nothing to
    /// persist. Reporting those as "missing" would manufacture findings.
    pub const EXPECTED: [Self; 2] = [Self::CharacterDb, Self::RtsimData];
}

/// Stores the `T4` tier spec names that this build does not persist.
///
/// Recorded rather than silently dropped: an inventory that simply
/// omitted them would read as "there are none", and the next reader would
/// have to re-derive that from the whole server. Each entry states how it
/// was established.
pub const NOT_PERSISTED_BY_THIS_BUILD: &[(&str, &str)] = &[
    (
        "map",
        "the world map is regenerated from the world seed at boot; no map artifact is written to \
         the data dir (see the LOD/map path, which serves from generated state)",
    ),
    (
        "replay/evidence",
        "APEX evidence records are written by the harness under its own output root, not into a \
         server save directory; they are not part of what a save's coherence would be about",
    ),
];

/// What the artifact itself declares about its version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FoundVersionV1 {
    /// Read out of the artifact's own bytes. This may be a version this
    /// binary cannot load — that is the point.
    Declared(u32),
    /// The artifact carries no version this row can read without opening
    /// it with the current binary's types.
    Undeclared,
    /// The artifact exists but could not be read far enough to find one.
    Unreadable(String),
}

/// One artifact found on disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveArtifactV1 {
    pub kind: SaveStoreKindV1,
    /// Path relative to the inventoried root, forward-slashed.
    pub relative_path: String,
    /// Content identity of the bytes AS FOUND, via `T0.3`. Digesting the
    /// raw file rather than a parsed form is what makes the record
    /// meaningful for an artifact this binary cannot parse.
    pub identity: ArtifactIdentityV1,
    pub found_version: FoundVersionV1,
}

/// Cross-store consistency.
///
/// One variant, deliberately. Adding a second would let a coherent-
/// looking fixture certify coherence that nothing in this row checks; the
/// spec's own falsifier is that a coherent save must STILL report
/// `Unverified`, and with one variant that is unrepresentable rather than
/// merely tested.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SaveConsistencyV1 {
    Unverified,
}

impl SaveConsistencyV1 {
    pub const ALL: [Self; 1] = [Self::Unverified];
}

/// One row of refinery's applied-migration history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedMigrationV1 {
    pub version: i32,
    pub name: String,
    pub checksum: String,
}

/// The character db's migration state, as found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationHistoryV1 {
    /// Read from `refinery_schema_history`, in applied order.
    Applied(Vec<AppliedMigrationV1>),
    /// The database opened but carries no refinery history table — a
    /// pre-refinery or hand-made file.
    NoHistoryTable,
    /// There is no character db to read.
    NoDatabase,
    /// The file exists and SQLite refused it. The string is the driver's
    /// own message, not an interpretation of it.
    Unreadable(String),
}

/// The whole report. A description of a directory, not a verdict on it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveInventoryV1 {
    pub root: String,
    /// Sorted by (kind, path) so two inventories of the same directory
    /// compare equal regardless of readdir order.
    pub artifacts: Vec<SaveArtifactV1>,
    pub missing: Vec<SaveStoreKindV1>,
    pub migrations: MigrationHistoryV1,
    pub consistency: SaveConsistencyV1,
}

impl SaveInventoryV1 {
    pub fn artifacts_of(&self, kind: SaveStoreKindV1) -> impl Iterator<Item = &SaveArtifactV1> {
        self.artifacts.iter().filter(move |a| a.kind == kind)
    }

    /// `T4.5`'s input: the multiset of content identities in this save,
    /// sorted. Two saves with the same index hold the same bytes, which
    /// is the only equality a corpus needs and the only one this row is
    /// entitled to assert.
    pub fn corpus_index_v1(&self) -> Vec<(SaveStoreKindV1, ArtifactIdentityV1)> {
        let mut index: Vec<_> =
            self.artifacts.iter().map(|a| (a.kind, a.identity.clone())).collect();
        index.sort_by(|a, b| {
            a.0.cmp(&b.0).then_with(|| a.1.digest.bytes.as_array().cmp(b.1.digest.bytes.as_array()))
        });
        index
    }
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn artifact(root: &Path, path: &Path, kind: SaveStoreKindV1) -> Option<SaveArtifactV1> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Some(SaveArtifactV1 {
                kind,
                relative_path: relative(root, path),
                identity: hash_artifact_bytes_v1(&[]),
                found_version: FoundVersionV1::Unreadable(err.to_string()),
            });
        },
    };

    let found_version = match kind {
        // The probe reads the version field alone, so a save from a
        // FUTURE version reports its number instead of failing — which is
        // the entire reason this row cannot use `Data::from_reader`.
        SaveStoreKindV1::RtsimData | SaveStoreKindV1::RtsimBackup => {
            match rtsim::data::Data::probe_version_v1(bytes.as_slice()) {
                Some(version) => FoundVersionV1::Declared(version),
                None => FoundVersionV1::Unreadable(
                    "not a readable msgpack named-map encoding".to_owned(),
                ),
            }
        },
        // SQLite's own `user_version` header word (offset 60, big-endian).
        // Read from the header rather than from a query: it costs no
        // connection, and refinery's state is reported separately.
        SaveStoreKindV1::CharacterDb => match bytes.get(60..64) {
            Some(word) => FoundVersionV1::Declared(u32::from_be_bytes([
                word[0], word[1], word[2], word[3],
            ])),
            None => FoundVersionV1::Unreadable("file is shorter than a SQLite header".to_owned()),
        },
        // Chunk files carry no version word of their own; their format is
        // pinned by the terrain-persistence code, not by the file.
        SaveStoreKindV1::TerrainChunk => FoundVersionV1::Undeclared,
    };

    Some(SaveArtifactV1 {
        kind,
        relative_path: relative(root, path),
        identity: hash_artifact_bytes_v1(&bytes),
        found_version,
    })
}

/// Read refinery's applied-migration history WITHOUT writing.
///
/// `mode=ro` alone is not enough for a WAL database: SQLite would want to
/// create the `-shm` sidecar next to a save we are supposed to be leaving
/// untouched. `immutable=1` is the flag that makes the read genuinely
/// non-mutating.
fn migration_history(db: &Path) -> MigrationHistoryV1 {
    if !db.is_file() {
        return MigrationHistoryV1::NoDatabase;
    }

    let uri = format!("file:{}?mode=ro&immutable=1", db.display().to_string().replace('\\', "/"));
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI;
    let connection = match rusqlite::Connection::open_with_flags(&uri, flags) {
        Ok(connection) => connection,
        Err(err) => return MigrationHistoryV1::Unreadable(err.to_string()),
    };

    let mut statement = match connection
        .prepare("SELECT version, name, checksum FROM refinery_schema_history ORDER BY version")
    {
        Ok(statement) => statement,
        // A missing table and a corrupt file are different findings and
        // must not collapse into one.
        Err(err) => {
            return if err.to_string().contains("no such table") {
                MigrationHistoryV1::NoHistoryTable
            } else {
                MigrationHistoryV1::Unreadable(err.to_string())
            };
        },
    };

    let rows = statement.query_map([], |row| {
        Ok(AppliedMigrationV1 {
            version: row.get(0)?,
            name: row.get(1)?,
            checksum: row.get(2)?,
        })
    });

    match rows.and_then(|rows| rows.collect::<Result<Vec<_>, _>>()) {
        Ok(applied) => MigrationHistoryV1::Applied(applied),
        Err(err) => MigrationHistoryV1::Unreadable(err.to_string()),
    }
}

/// Inventory a server data directory. Read-only on every path, including
/// no repair-on-read: a corrupt artifact is reported corrupt and left
/// exactly as found.
pub fn inventory_save_dir_v1(root: &Path) -> SaveInventoryV1 {
    let mut artifacts = Vec::new();

    let db = root.join("saves").join("db.sqlite");
    if db.is_file() {
        artifacts.extend(artifact(root, &db, SaveStoreKindV1::CharacterDb));
    }

    let rtsim_dir = root.join("rtsim");
    if let Ok(entries) = fs::read_dir(&rtsim_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let kind = if name == "data.dat" {
                SaveStoreKindV1::RtsimData
            } else if name.starts_with("data.ron_backup") {
                SaveStoreKindV1::RtsimBackup
            } else {
                continue;
            };
            artifacts.extend(artifact(root, &path, kind));
        }
    }

    let terrain_dir = root.join("terrain");
    if let Ok(entries) = fs::read_dir(&terrain_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            if path.is_file() && name.starts_with("chunk_") && name.ends_with(".dat") {
                artifacts.extend(artifact(root, &path, SaveStoreKindV1::TerrainChunk));
            }
        }
    }

    artifacts.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.relative_path.cmp(&b.relative_path)));

    let missing = SaveStoreKindV1::EXPECTED
        .into_iter()
        .filter(|kind| !artifacts.iter().any(|a| a.kind == *kind))
        .collect();

    SaveInventoryV1 {
        root: root.to_string_lossy().replace('\\', "/"),
        artifacts,
        missing,
        migrations: migration_history(&db),
        consistency: SaveConsistencyV1::Unverified,
    }
}

#[cfg(test)]
mod save_inventory_v1 {
    use super::*;
    use std::collections::BTreeMap;

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("fixture path has a parent")).expect("mkdir");
        fs::write(path, bytes).expect("fixture write");
    }

    /// An rtsim save declaring `version`, encoded the way the real writer
    /// encodes (named map), so the probe is exercised against the real
    /// shape rather than a convenient one.
    fn rtsim_bytes(version: u32) -> Vec<u8> {
        #[derive(serde::Serialize)]
        struct FakeSave {
            version: u32,
            nature: BTreeMap<String, u32>,
            some_future_field: Vec<u8>,
        }
        let mut buf = Vec::new();
        rmp_serde::encode::write_named(&mut buf, &FakeSave {
            version,
            nature: BTreeMap::new(),
            some_future_field: vec![1, 2, 3],
        })
        .expect("fixture encode");
        buf
    }

    /// Every file under `root`, digested. The no-write test's instrument.
    fn snapshot(root: &Path) -> BTreeMap<String, [u8; 32]> {
        fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, [u8; 32]>) {
            let Ok(entries) = fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else if let Ok(bytes) = fs::read(&path) {
                    out.insert(
                        relative(root, &path),
                        *hash_artifact_bytes_v1(&bytes).digest.bytes.as_array(),
                    );
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// A store that simply is not there is reported missing, not
    /// inferred, and not created.
    #[test]
    fn a_missing_store_is_reported_and_not_created() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write(root, "rtsim/data.dat", &rtsim_bytes(10));

        let report = inventory_save_dir_v1(root);
        assert_eq!(report.missing, vec![SaveStoreKindV1::CharacterDb]);
        assert_eq!(report.migrations, MigrationHistoryV1::NoDatabase);
        assert!(!root.join("saves").exists(), "the inventory created a store it was diagnosing");
    }

    /// A truncated artifact produces a record, not a failure: the digest
    /// of what IS there is the most useful thing about a broken save.
    #[test]
    fn a_truncated_store_still_produces_a_record() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let full = rtsim_bytes(10);
        write(root, "rtsim/data.dat", &full[..full.len() / 3]);

        let report = inventory_save_dir_v1(root);
        let found: Vec<_> = report.artifacts_of(SaveStoreKindV1::RtsimData).collect();
        assert_eq!(found.len(), 1, "the truncated file vanished from the report");
        assert!(
            matches!(found[0].found_version, FoundVersionV1::Unreadable(_)),
            "a truncated msgpack save reported a version anyway: {:?}",
            found[0].found_version
        );
        assert!(found[0].identity.size_bytes > 0, "the bytes that ARE there were not digested");
    }

    /// A save from a version this binary cannot load reports THAT
    /// VERSION. `Data::from_reader` would only be able to say "no".
    #[test]
    fn a_future_version_store_reports_its_version() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let future = rtsim::data::CURRENT_VERSION + 7;
        write(root, "rtsim/data.dat", &rtsim_bytes(future));

        let report = inventory_save_dir_v1(root);
        let found: Vec<_> = report.artifacts_of(SaveStoreKindV1::RtsimData).collect();
        assert_eq!(found[0].found_version, FoundVersionV1::Declared(future));
        assert!(
            rtsim::data::Data::from_reader(rtsim_bytes(future).as_slice()).is_err(),
            "the fixture no longer represents an unloadable save, so this test proves nothing"
        );
    }

    /// The probe's documented contract for an absent field: version 0,
    /// which is what its absence has always meant in this format. Claimed
    /// in `probe_version_v1`'s doc, so it is asserted rather than assumed.
    #[test]
    fn an_rtsim_save_with_no_version_field_reads_as_version_zero() {
        #[derive(serde::Serialize)]
        struct NoVersion {
            nature: BTreeMap<String, u32>,
        }
        let mut buf = Vec::new();
        rmp_serde::encode::write_named(&mut buf, &NoVersion { nature: BTreeMap::new() })
            .expect("fixture encode");

        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write(root, "rtsim/data.dat", &buf);

        let report = inventory_save_dir_v1(root);
        let found: Vec<_> = report.artifacts_of(SaveStoreKindV1::RtsimData).collect();
        assert_eq!(found[0].found_version, FoundVersionV1::Declared(0));
    }

    /// Two rtsim files in one directory is a real state — the loader
    /// creates it by moving a failed save aside. Both are reported, and
    /// the backup's presence is not silently merged into the live one.
    #[test]
    fn two_rtsim_files_are_both_reported() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write(root, "rtsim/data.dat", &rtsim_bytes(10));
        write(root, "rtsim/data.ron_backup", &rtsim_bytes(4));
        write(root, "rtsim/data.ron_backup_1", &rtsim_bytes(3));

        let report = inventory_save_dir_v1(root);
        assert_eq!(report.artifacts_of(SaveStoreKindV1::RtsimData).count(), 1);
        let backups: Vec<_> = report.artifacts_of(SaveStoreKindV1::RtsimBackup).collect();
        assert_eq!(backups.len(), 2, "a backup was dropped: {:?}", report.artifacts);
        assert_eq!(backups[0].found_version, FoundVersionV1::Declared(4));
        assert_eq!(backups[1].found_version, FoundVersionV1::Declared(3));
    }

    /// Stores that disagree produce a report and no verdict.
    #[test]
    fn disagreeing_stores_produce_a_report_and_no_verdict() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write(root, "rtsim/data.dat", &rtsim_bytes(2));
        // A "database" that is not one. The inventory records what it
        // found and what SQLite said; it does not decide anything.
        write(root, "saves/db.sqlite", b"this is not a sqlite file at all");

        let report = inventory_save_dir_v1(root);
        assert!(report.missing.is_empty(), "both stores are present");
        assert!(
            matches!(report.migrations, MigrationHistoryV1::Unreadable(_)),
            "a non-database was not reported as unreadable: {:?}",
            report.migrations
        );
        assert_eq!(report.consistency, SaveConsistencyV1::Unverified);
    }

    /// The spec's falsifier. A save whose stores look perfectly coherent
    /// must STILL report `Unverified`, or the field means nothing — and
    /// with one variant it cannot report anything else.
    #[test]
    fn a_coherent_looking_save_is_still_unverified() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write(root, "rtsim/data.dat", &rtsim_bytes(rtsim::data::CURRENT_VERSION));
        write(root, "terrain/chunk_0_0.dat", b"chunk bytes");

        let report = inventory_save_dir_v1(root);
        assert_eq!(report.consistency, SaveConsistencyV1::Unverified);
        assert_eq!(
            SaveConsistencyV1::ALL.len(),
            1,
            "a second consistency variant appeared. Certifying cross-store coherence is T4.6's \
             job; a variant here would let a coherent-LOOKING save claim it"
        );
    }

    /// No writes on any path, including no repair-on-read. Asserted by
    /// digesting the whole tree either side of the call rather than by
    /// reading the code.
    #[test]
    fn inventorying_writes_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write(root, "rtsim/data.dat", &rtsim_bytes(10));
        write(root, "rtsim/data.ron_backup", b"garbage");
        write(root, "saves/db.sqlite", b"not a database");
        write(root, "terrain/chunk_1_2.dat", b"chunk bytes");

        let before = snapshot(root);
        let _ = inventory_save_dir_v1(root);
        let after = snapshot(root);
        assert_eq!(
            before, after,
            "the inventory changed the directory it was diagnosing (a new -wal/-shm sidecar \
             counts: it is a write into someone's save)"
        );
    }

    /// The corpus index is a function of the bytes, so the same save
    /// indexes identically however it was walked.
    #[test]
    fn the_corpus_index_is_a_function_of_the_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write(root, "rtsim/data.dat", &rtsim_bytes(10));
        write(root, "terrain/chunk_0_0.dat", b"a");
        write(root, "terrain/chunk_9_9.dat", b"b");

        let index = inventory_save_dir_v1(root).corpus_index_v1();
        assert_eq!(index.len(), 3);
        assert_eq!(index, inventory_save_dir_v1(root).corpus_index_v1());

        // And it is sensitive: one changed byte moves the index.
        write(root, "terrain/chunk_9_9.dat", b"c");
        assert_ne!(index, inventory_save_dir_v1(root).corpus_index_v1());
    }

    /// Build a real SQLite database at `saves/db.sqlite`, in WAL mode —
    /// the mode where a careless read-only open creates a `-shm` sidecar.
    fn write_database(root: &Path, with_history: bool) {
        let path = root.join("saves").join("db.sqlite");
        fs::create_dir_all(path.parent().expect("saves has a parent")).expect("mkdir");
        let connection = rusqlite::Connection::open(&path).expect("fixture db");
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .expect("fixture journal mode");
        connection
            .execute_batch("CREATE TABLE character (id INTEGER PRIMARY KEY, alias TEXT)")
            .expect("fixture schema");
        if with_history {
            connection
                .execute_batch(
                    "CREATE TABLE refinery_schema_history (version INT4 PRIMARY KEY, name \
                     VARCHAR(255), applied_on VARCHAR(255), checksum VARCHAR(255));
                     INSERT INTO refinery_schema_history VALUES (2, 'second', '', 'cksum-2');
                     INSERT INTO refinery_schema_history VALUES (1, 'first', '', 'cksum-1');",
                )
                .expect("fixture history");
        }
        drop(connection);
    }

    /// The migration history is read back in applied order from a real
    /// database. Without this the `Applied` arm was never executed and
    /// the row's most delicate path — an actual connection — rested on
    /// the code reading correctly.
    #[test]
    fn refinery_history_is_read_in_version_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write_database(root, true);

        let report = inventory_save_dir_v1(root);
        let MigrationHistoryV1::Applied(applied) = report.migrations else {
            panic!("a real database with a history table read as {:?}", report.migrations);
        };
        assert_eq!(applied.len(), 2);
        // Inserted 2-then-1; reported 1-then-2, because the query orders.
        assert_eq!(applied[0].version, 1);
        assert_eq!(applied[0].name, "first");
        assert_eq!(applied[1].checksum, "cksum-2");
    }

    /// A database with no refinery table is a distinct finding from a
    /// corrupt one, and must not collapse into `Unreadable`.
    #[test]
    fn a_database_without_refinery_state_is_not_reported_as_corrupt() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write_database(root, false);

        let report = inventory_save_dir_v1(root);
        assert_eq!(report.migrations, MigrationHistoryV1::NoHistoryTable);
        assert!(report.missing.is_empty() || report.missing == vec![SaveStoreKindV1::RtsimData]);
    }

    /// The no-write claim against a REAL database. This is where it could
    /// actually fail: a WAL-mode file opened read-only WITHOUT
    /// `immutable=1` makes SQLite create a `-shm` next to the save.
    #[test]
    fn reading_a_real_wal_database_creates_no_sidecar() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write_database(root, true);
        write(root, "rtsim/data.dat", &rtsim_bytes(rtsim::data::CURRENT_VERSION));

        let before = snapshot(root);
        let report = inventory_save_dir_v1(root);
        let after = snapshot(root);

        assert!(
            matches!(report.migrations, MigrationHistoryV1::Applied(_)),
            "the connection did not actually read, so this test proves nothing: {:?}",
            report.migrations
        );
        assert_eq!(
            before.keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>(),
            "reading the database added or removed files in the save directory"
        );
        assert_eq!(before, after, "reading the database changed bytes in the save directory");
    }

    /// The stores the tier spec named but this build does not persist are
    /// recorded with the evidence for saying so, not silently omitted.
    #[test]
    fn stores_this_build_does_not_persist_are_named_with_reasons() {
        assert!(!NOT_PERSISTED_BY_THIS_BUILD.is_empty());
        for (store, why) in NOT_PERSISTED_BY_THIS_BUILD {
            assert!(
                why.len() > 40,
                "{store} is excluded from the inventory on an assertion rather than evidence: \
                 {why:?}"
            );
        }
    }
}
