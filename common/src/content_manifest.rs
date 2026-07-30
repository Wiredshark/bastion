//! T0.57 (master build order; T0-004 packet, step 8 family): a sorted
//! per-file content manifest + root hash — NOT source-commit-only, not
//! directory mtimes, not one opaque archive hash. On a mismatch the diff
//! reports changed PATHS, not just "the hash differs".
//!
//! T0.54 (same family): a project-owned in-toto-style provenance sidecar —
//! the statement SHAPE (source commit, materials + digests, tool/run
//! identity), without importing a full SLSA/SPDX stack.
//!
//! Determinism story (Ben's law): the manifest root is a fold over
//! key-SORTED entries; identical content yields an identical root
//! regardless of directory-walk order.

use crate::state_hash::{DomainHash, DomainHasher};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One file's canonical content entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentEntry {
    /// Repo-relative path, forward-slashed (platform-independent).
    pub path: String,
    pub content_hash: DomainHash,
}

/// T0.57: the content manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentManifest {
    /// e.g. `bastion/content-manifest/v1`.
    pub schema: String,
    pub source_commit: String,
    pub feature_flags: Vec<String>,
    pub locale_set: Vec<String>,
    /// Path-sorted (enforced by [`ContentManifest::build`]).
    pub files: Vec<ContentEntry>,
    pub root: DomainHash,
}

impl ContentManifest {
    pub fn build(
        schema: impl Into<String>,
        source_commit: impl Into<String>,
        mut feature_flags: Vec<String>,
        mut locale_set: Vec<String>,
        mut files: Vec<ContentEntry>,
    ) -> Self {
        feature_flags.sort();
        locale_set.sort();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let mut hasher = DomainHasher::new("bastion/content-manifest/v1/sha256");
        for flag in &feature_flags {
            hasher.field(flag.as_bytes());
        }
        for locale in &locale_set {
            hasher.field(locale.as_bytes());
        }
        for entry in &files {
            hasher.field(entry.path.as_bytes());
            hasher.field(&entry.content_hash.0);
        }
        let root = hasher.finish();
        Self {
            schema: schema.into(),
            source_commit: source_commit.into(),
            feature_flags,
            locale_set,
            files,
            root,
        }
    }

    /// Diff against another manifest: the changed/added/removed PATHS (not
    /// "the hash differs"). Empty iff the file sets are byte-equal.
    pub fn changed_paths(&self, other: &ContentManifest) -> Vec<String> {
        use std::collections::BTreeMap;
        let mine: BTreeMap<&str, &DomainHash> = self
            .files
            .iter()
            .map(|e| (e.path.as_str(), &e.content_hash))
            .collect();
        let theirs: BTreeMap<&str, &DomainHash> = other
            .files
            .iter()
            .map(|e| (e.path.as_str(), &e.content_hash))
            .collect();
        let mut changed = Vec::new();
        for (path, hash) in &mine {
            match theirs.get(path) {
                Some(other_hash) if other_hash == hash => {},
                _ => changed.push((*path).to_string()),
            }
        }
        for path in theirs.keys() {
            if !mine.contains_key(path) {
                changed.push((*path).to_string());
            }
        }
        changed.sort();
        changed.dedup();
        changed
    }
}

/// `APEX-T4.1-CONTENT-LIVE`: the live construction `ContentManifest::build`
/// itself was always missing a caller for -- walks the real, on-disk asset
/// tree at [`common_assets::ASSETS_PATH`] and folds every regular file's
/// content through [`ContentManifest::build`]'s own path-sorted hash.
///
/// **Deliberately plain `std::fs`, not `assets_manager`'s `Source`
/// abstraction.** `Source` addresses assets by `(id, ext)`, not by path,
/// and `common_assets::FileSystem` merges `VELOREN_ASSETS_OVERRIDE` into
/// that addressing. Replicating the override-merge precedence correctly
/// through `Source::read_dir` would be real, separate work; this walk
/// covers `ASSETS_PATH` (the default tree) only. A server actually
/// running under `VELOREN_ASSETS_OVERRIDE` would therefore get a content
/// root that does not reflect the override -- named here as the row's
/// own scoping decision, not discovered later as a silent gap.
///
/// **Symlinks are skipped, not followed.** A cycle would hang the walk;
/// this tree's own real assets are not expected to contain any (LFS
/// files are smudged into real blobs by git-lfs, never left as
/// symlinks), so skipping is a safe default rather than a hazard this
/// row leaves unnamed.
///
/// **Affordability, measured not assumed** (this row's own required gate
/// before wiring anything live -- and the reason this function reads +
/// hashes files IN PARALLEL rather than during the directory walk
/// itself). The first cut was single-threaded, one syscall-bound file at
/// a time: **115.7 seconds** against this tree's real 10,610 files
/// (~415MB), measured on a cold run -- dominated by per-file open/read/
/// close latency, not raw hashing throughput, so a faster hasher would
/// not have fixed it. Splitting the walk into "collect paths" (cheap)
/// then "read+hash every path with `rayon`" (parallel) measured
/// **492ms** on a subsequent (likely warm-cache) run -- a ~235x
/// improvement. Honestly: a true cold-cache boot may cost more than
/// 492ms (disk bandwidth rather than cache-hit latency becomes the
/// floor), but even a naive bound of 415MB over a slow disk lands in the
/// low single digits of seconds, not anywhere near the original 115s --
/// see `t4_1_content_live_real_asset_tree_walk_completes_in_bounded_time`
/// below for the bound this program actually enforces. This is a
/// ONE-TIME, boot-scoped cost regardless -- callers must compute it
/// exactly once at server start and cache the result; it must never be
/// re-invoked per-connection (`server/src/sys/msg/register.rs`'s own
/// `bootstrap_manifest_v1` sends the resulting descriptor on EVERY
/// client admission, which is exactly the call site that would turn a
/// one-time cost into a per-player one if this were called there
/// directly instead of read from a cache).
pub fn build_from_asset_tree_v1(
    source_commit: impl Into<String>,
    feature_flags: Vec<String>,
    locale_set: Vec<String>,
) -> std::io::Result<ContentManifest> {
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    let root = &*common_assets::ASSETS_PATH;
    // Phase 1: collect every regular-file path. Pure directory listing,
    // no file content read -- cheap relative to phase 2, kept separate
    // so phase 2 can parallelize purely over already-known paths rather
    // than interleaving directory syscalls with file reads on one thread.
    let mut paths = Vec::new();
    collect_file_paths_v1(root, &mut paths)?;

    // Phase 2: read + hash every file IN PARALLEL. Measured, not assumed
    // affordable (this row's own required gate): a single-threaded first
    // cut took ~116s against the real tree (10,610 files, ~415MB) --
    // dominated by per-file open/read/close syscall latency, not raw
    // hashing throughput, so spreading the I/O across threads is the
    // right fix, not a faster hasher. See
    // `t4_1_content_live_real_asset_tree_walk_completes_in_bounded_time`
    // for the parallel version's own measured bound.
    let files: Vec<ContentEntry> = paths
        .par_iter()
        .map(|path| -> std::io::Result<ContentEntry> {
            let bytes = std::fs::read(path)?;
            let mut hasher = DomainHasher::new("bastion/content-manifest/file/v1/sha256");
            hasher.field(&bytes);
            let relative = path.strip_prefix(root).expect("collected path is always under root").to_string_lossy().replace('\\', "/");
            Ok(ContentEntry { path: relative, content_hash: hasher.finish() })
        })
        .collect::<std::io::Result<Vec<_>>>()?;

    Ok(ContentManifest::build("bastion/content-manifest/v1", source_commit, feature_flags, locale_set, files))
}

fn collect_file_paths_v1(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_symlink() {
            continue;
        } else if file_type.is_dir() {
            collect_file_paths_v1(&path, out)?;
        } else if file_type.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// Derives `T4.1`/`T4-PV`'s `ContentProtocolVersion` from a real,
/// already-built [`ContentManifest`]'s own root -- domain-separated the
/// same way every other protocol root in this program is (`net_envelope_
/// profile_root_v1`'s own pattern, per `T4-PV`'s own ruling). `None` only
/// if the domain-separation hash itself fails, which cannot happen for a
/// fixed 32-byte input under this domain's generous limit -- kept
/// `Option` rather than `unwrap`ped so a caller mirrors the same
/// "absent, not fabricated" discipline `WorldBaselineInputV1`'s other
/// slots already use, instead of panicking a server boot on an
/// unreachable error path.
pub fn content_protocol_version_v1(
    manifest: &ContentManifest,
) -> Option<crate::apex::subsystem::descriptor::ContentProtocolVersion> {
    crate::apex::digest::digest_canonical_bytes_v1(crate::apex::digest::DigestDomainIdV1::ContentProtocolRoot, &manifest.root.0, 1 << 20)
        .ok()
        .map(crate::apex::subsystem::descriptor::ContentProtocolVersion::new)
}

/// T0.54: one referenced material (input) with its digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceMaterial {
    pub uri: String,
    pub digest: DomainHash,
}

/// T0.54: the in-toto-style provenance statement SHAPE for a build artifact
/// or research doc — source commit, materials + digests, tool/run identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceStatement {
    /// e.g. `bastion/provenance/v1`.
    pub schema: String,
    /// The subject artifact this statement is about (path or logical name).
    pub subject: String,
    pub subject_digest: DomainHash,
    pub source_commit: String,
    pub materials: Vec<ProvenanceMaterial>,
    /// Tool + run identity (builder name, run id) — provenance, never a
    /// gameplay clock.
    pub tool_identity: String,
    pub run_id: u64,
}

#[cfg(test)]
mod t0_57_tests {
    use super::*;

    fn entry(path: &str, byte: u8) -> ContentEntry {
        ContentEntry {
            path: path.to_string(),
            content_hash: DomainHash([byte; 32]),
        }
    }

    #[test]
    fn t0_57_root_is_walk_order_free() {
        let a = ContentManifest::build("s", "abc", vec![], vec![], vec![
            entry("b.rs", 2),
            entry("a.rs", 1),
        ]);
        let b = ContentManifest::build("s", "abc", vec![], vec![], vec![
            entry("a.rs", 1),
            entry("b.rs", 2),
        ]);
        assert_eq!(a.root, b.root, "directory-walk order must not affect the root");
    }

    #[test]
    fn t0_57_diff_reports_changed_paths() {
        let base = ContentManifest::build("s", "c1", vec![], vec![], vec![
            entry("a.rs", 1),
            entry("b.rs", 2),
            entry("c.rs", 3),
        ]);
        let changed = ContentManifest::build("s", "c2", vec![], vec![], vec![
            entry("a.rs", 1),   // unchanged
            entry("b.rs", 9),   // changed
            entry("d.rs", 4),   // added (c.rs removed)
        ]);
        assert_eq!(base.changed_paths(&changed), vec![
            "b.rs".to_string(),
            "c.rs".to_string(),
            "d.rs".to_string(),
        ]);
        assert!(base.changed_paths(&base).is_empty());
    }
}

#[cfg(test)]
mod t4_1_content_live_tests {
    use super::*;

    /// `APEX-T4.1-CONTENT-LIVE`'s own affordability gate, run against the
    /// REAL checked-out asset tree, not a synthetic fixture -- measured,
    /// not assumed. 30 seconds is deliberately generous (this row's own
    /// measured run was well under that on ordinary local disk), but a
    /// real regression -- the tree growing by orders of magnitude, or a
    /// caller accidentally invoking this per-connection instead of once
    /// at boot -- fails loudly here rather than silently degrading a
    /// live server's connection latency.
    #[test]
    fn t4_1_content_live_real_asset_tree_walk_completes_in_bounded_time() {
        let started = std::time::Instant::now();
        let manifest = build_from_asset_tree_v1("test", vec![], vec![]).expect("the real asset tree is readable");
        let elapsed = started.elapsed();
        println!("content manifest: {} files in {:?}", manifest.files.len(), elapsed);
        assert!(manifest.files.len() > 1000, "the real asset tree should contain far more than 1000 files; got {}", manifest.files.len());
        assert!(elapsed.as_secs() < 30, "asset-tree content walk took {elapsed:?}, exceeding the one-time boot-cost bound");
    }

    /// The real walk's root is stable across two runs against the SAME
    /// unmodified tree -- proves the function is a pure, deterministic
    /// function of on-disk content, not of walk order or timing.
    #[test]
    fn t4_1_content_live_real_asset_tree_walk_is_deterministic() {
        let a = build_from_asset_tree_v1("test", vec![], vec![]).expect("walk 1");
        let b = build_from_asset_tree_v1("test", vec![], vec![]).expect("walk 2");
        assert_eq!(a.root, b.root);
        assert_eq!(a.files, b.files);
    }

    /// Every entry's path is forward-slashed and repo-relative (no
    /// leading `assets/`, since the walk root itself IS the assets
    /// directory) -- the platform-independence `ContentEntry::path`'s own
    /// doc comment requires.
    #[test]
    fn t4_1_content_live_paths_are_forward_slashed_and_relative() {
        let manifest = build_from_asset_tree_v1("test", vec![], vec![]).expect("walk");
        for entry in &manifest.files {
            assert!(!entry.path.contains('\\'), "path must be forward-slashed: {:?}", entry.path);
            assert!(!entry.path.starts_with('/'), "path must be relative, not absolute: {:?}", entry.path);
            assert!(!entry.path.starts_with("assets/"), "path must not double up the walk root: {:?}", entry.path);
        }
    }
}
