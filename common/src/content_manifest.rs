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
