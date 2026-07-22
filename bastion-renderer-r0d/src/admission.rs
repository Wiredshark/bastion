//! BUILD-007A10.0 — the W0 source-authority manifest and its typed classifier.

use crate::{BlobMap, domain_hash, push_sorted_vec, push_str};
use std::collections::{BTreeMap, BTreeSet};

/// The 17-field W0 source-authority manifest (design §3.3). It binds the
/// immutable integration base so that any later admitted build can be checked
/// against it byte-for-byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererW0AdmissionV2 {
    pub source_commit: String,
    pub source_parent_set: Vec<String>,
    pub source_tree_sha256: String,
    pub branch_name: String,
    pub worktree_path_hash: [u8; 32],
    pub clean_status: bool,
    pub submodule_status: bool,
    pub rust_toolchain: String,
    pub cargo_lock_sha256: String,
    pub package_set: Vec<String>,
    pub feature_set: Vec<String>,
    /// Sorted, slash-normalized paths R0D may see already present.
    pub allowed_existing_paths: Vec<String>,
    /// Sorted paths R0D must never author (e.g. `common/src/comp/mod.rs`).
    pub forbidden_paths: Vec<String>,
    /// Sorted namespace prefixes R0D may create.
    pub allowed_new_namespaces: Vec<String>,
    /// Pre-edit blob hashes of every existing path, keyed by path (sorted).
    pub base_blob_sha256_by_path: BlobMap,
    /// Sorted lease identifiers this build owns.
    pub ownership_leases: Vec<String>,
    /// `true` = the collision scan found no conflicting owner.
    pub collision_scan_result: bool,
}

/// The observed candidate source state validated against a manifest.
#[derive(Clone, Debug, Default)]
pub struct CandidateSourceState {
    pub branch_name: String,
    pub source_commit: String,
    /// Tracked paths reported dirty/modified.
    pub dirty_paths: Vec<String>,
    /// Every path present in the candidate worktree.
    pub present_paths: Vec<String>,
    /// Observed blob hash by path.
    pub blob_sha256_by_path: BlobMap,
    /// Leases the candidate is holding.
    pub held_leases: Vec<String>,
}

/// Distinct typed source-authority failures. Human prose is diagnostic and
/// excluded from equality — the variant plus the offending item is the stable
/// identity, so `R0D_SOURCE_AUTHORITY_MISMATCH` is machine-classifiable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum R0dSourceAuthorityMismatch {
    DirtyFile {
        path: String,
    },
    ExtraPath {
        path: String,
    },
    ChangedBaseBlob {
        path: String,
        expected: String,
        actual: String,
    },
    BranchDrift {
        expected_branch: String,
        expected_commit: String,
        actual_branch: String,
        actual_commit: String,
    },
    LeaseCollision {
        lease: String,
    },
}

impl RendererW0AdmissionV2 {
    /// Validate a candidate. Checks run in a fixed deterministic order and the
    /// first failure wins; there is no best-effort continuation. Every scan is
    /// byte-sorted so the returned failure is a pure function of the inputs.
    pub fn validate(
        &self,
        cand: &CandidateSourceState,
    ) -> Result<(), R0dSourceAuthorityMismatch> {
        // 1. Clean status — no dirty tracked file.
        if self.clean_status {
            let mut dirty: Vec<&String> = cand.dirty_paths.iter().collect();
            dirty.sort();
            if let Some(p) = dirty.first() {
                return Err(R0dSourceAuthorityMismatch::DirtyFile {
                    path: (*p).clone(),
                });
            }
        }

        // 2. Path boundary — every present path must be allowed and not forbidden.
        let allowed: BTreeSet<&String> = self.allowed_existing_paths.iter().collect();
        let forbidden: BTreeSet<&String> = self.forbidden_paths.iter().collect();
        let mut present: Vec<&String> = cand.present_paths.iter().collect();
        present.sort();
        for p in present {
            if forbidden.contains(p) || !(allowed.contains(p) || self.is_allowed_new(p)) {
                return Err(R0dSourceAuthorityMismatch::ExtraPath { path: p.clone() });
            }
        }

        // 3. Base blobs unchanged (BTreeMap iterates path-sorted).
        for (path, expected) in &self.base_blob_sha256_by_path {
            if let Some(actual) = cand.blob_sha256_by_path.get(path) {
                if actual != expected {
                    return Err(R0dSourceAuthorityMismatch::ChangedBaseBlob {
                        path: path.clone(),
                        expected: expected.clone(),
                        actual: actual.clone(),
                    });
                }
            }
        }

        // 4. Branch drift — the candidate must sit on the admitted branch/commit.
        if cand.branch_name != self.branch_name || cand.source_commit != self.source_commit {
            return Err(R0dSourceAuthorityMismatch::BranchDrift {
                expected_branch: self.branch_name.clone(),
                expected_commit: self.source_commit.clone(),
                actual_branch: cand.branch_name.clone(),
                actual_commit: cand.source_commit.clone(),
            });
        }

        // 5. Lease collision — every held lease must be one this build owns.
        let owned: BTreeSet<&String> = self.ownership_leases.iter().collect();
        let mut held: Vec<&String> = cand.held_leases.iter().collect();
        held.sort();
        for l in held {
            if !owned.contains(l) {
                return Err(R0dSourceAuthorityMismatch::LeaseCollision { lease: l.clone() });
            }
        }

        Ok(())
    }

    fn is_allowed_new(&self, path: &str) -> bool {
        self.allowed_new_namespaces
            .iter()
            .any(|ns| path == ns || path.starts_with(&format!("{ns}/")))
    }

    /// The admission digest over the canonical, byte-sorted manifest fields.
    #[must_use]
    pub fn admission_digest(&self) -> [u8; 32] {
        domain_hash("bastion/r0d/manifest/v1/sha256", 1, 0, &self.canonical_bytes())
    }

    /// Deterministic manifest serialization: length-framed fields in a fixed
    /// order, every collection byte-sorted so insertion order can never leak in.
    /// (007A10.1 upgrades this to canonical CBOR; a fixed length-framed encoding
    /// is sufficient and stable for the W0 admission digest.)
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        push_str(&mut b, &self.source_commit);
        push_sorted_vec(&mut b, &self.source_parent_set);
        push_str(&mut b, &self.source_tree_sha256);
        push_str(&mut b, &self.branch_name);
        b.extend_from_slice(&self.worktree_path_hash);
        b.push(u8::from(self.clean_status));
        b.push(u8::from(self.submodule_status));
        push_str(&mut b, &self.rust_toolchain);
        push_str(&mut b, &self.cargo_lock_sha256);
        push_sorted_vec(&mut b, &self.package_set);
        push_sorted_vec(&mut b, &self.feature_set);
        push_sorted_vec(&mut b, &self.allowed_existing_paths);
        push_sorted_vec(&mut b, &self.forbidden_paths);
        push_sorted_vec(&mut b, &self.allowed_new_namespaces);
        b.extend_from_slice(&(self.base_blob_sha256_by_path.len() as u64).to_le_bytes());
        for (p, h) in &self.base_blob_sha256_by_path {
            push_str(&mut b, p);
            push_str(&mut b, h);
        }
        push_sorted_vec(&mut b, &self.ownership_leases);
        b.push(u8::from(self.collision_scan_result));
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> RendererW0AdmissionV2 {
        let mut blobs = BTreeMap::new();
        blobs.insert("common/src/comp/mod.rs".to_string(), "aaaa".to_string());
        blobs.insert("common/src/lib.rs".to_string(), "bbbb".to_string());
        RendererW0AdmissionV2 {
            source_commit: "5d0dc72b9a".to_string(),
            source_parent_set: vec!["p0".to_string()],
            source_tree_sha256: "1494e6fdcc53".to_string(),
            branch_name: "bastion/renderer-r0d-w0-v2".to_string(),
            worktree_path_hash: [7u8; 32],
            clean_status: true,
            submodule_status: true,
            rust_toolchain: "stable-x86_64".to_string(),
            cargo_lock_sha256: "cccc".to_string(),
            package_set: vec!["common".to_string(), "bastion-renderer-r0d".to_string()],
            feature_set: vec![],
            allowed_existing_paths: vec![
                "common/src/comp/mod.rs".to_string(),
                "common/src/lib.rs".to_string(),
            ],
            forbidden_paths: vec!["common/src/comp/mod.rs".to_string()],
            allowed_new_namespaces: vec!["bastion-renderer-r0d".to_string()],
            base_blob_sha256_by_path: blobs,
            ownership_leases: vec!["renderer-r0d".to_string()],
            collision_scan_result: true,
        }
    }

    /// A candidate that passes every check (used as the mutation baseline).
    /// Note the forbidden `comp/mod.rs` is intentionally NOT present here — a
    /// clean base must not carry it (DC-003).
    fn clean_candidate(m: &RendererW0AdmissionV2) -> CandidateSourceState {
        CandidateSourceState {
            branch_name: m.branch_name.clone(),
            source_commit: m.source_commit.clone(),
            dirty_paths: vec![],
            present_paths: vec![
                "common/src/lib.rs".to_string(),
                "bastion-renderer-r0d/src/lib.rs".to_string(),
            ],
            blob_sha256_by_path: {
                let mut m2 = BTreeMap::new();
                m2.insert("common/src/lib.rs".to_string(), "bbbb".to_string());
                m2
            },
            held_leases: vec!["renderer-r0d".to_string()],
        }
    }

    #[test]
    fn clean_candidate_admits() {
        let m = manifest();
        assert_eq!(m.validate(&clean_candidate(&m)), Ok(()));
    }

    #[test]
    fn dirty_file_forces_typed_failure() {
        let m = manifest();
        let mut c = clean_candidate(&m);
        c.dirty_paths.push("common/src/lib.rs".to_string());
        assert_eq!(
            m.validate(&c),
            Err(R0dSourceAuthorityMismatch::DirtyFile {
                path: "common/src/lib.rs".to_string()
            })
        );
    }

    #[test]
    fn extra_path_forces_typed_failure() {
        let m = manifest();
        let mut c = clean_candidate(&m);
        c.present_paths.push("voxygen/src/render/rogue.rs".to_string());
        assert_eq!(
            m.validate(&c),
            Err(R0dSourceAuthorityMismatch::ExtraPath {
                path: "voxygen/src/render/rogue.rs".to_string()
            })
        );
    }

    #[test]
    fn changed_base_blob_forces_typed_failure() {
        let m = manifest();
        let mut c = clean_candidate(&m);
        c.blob_sha256_by_path
            .insert("common/src/lib.rs".to_string(), "deadbeef".to_string());
        assert_eq!(
            m.validate(&c),
            Err(R0dSourceAuthorityMismatch::ChangedBaseBlob {
                path: "common/src/lib.rs".to_string(),
                expected: "bbbb".to_string(),
                actual: "deadbeef".to_string(),
            })
        );
    }

    #[test]
    fn branch_drift_forces_typed_failure() {
        let m = manifest();
        let mut c = clean_candidate(&m);
        c.source_commit = "ffffffff".to_string();
        assert_eq!(
            m.validate(&c),
            Err(R0dSourceAuthorityMismatch::BranchDrift {
                expected_branch: m.branch_name.clone(),
                expected_commit: m.source_commit.clone(),
                actual_branch: c.branch_name.clone(),
                actual_commit: "ffffffff".to_string(),
            })
        );
    }

    #[test]
    fn lease_collision_forces_typed_failure() {
        let m = manifest();
        let mut c = clean_candidate(&m);
        c.held_leases.push("someone-elses-lease".to_string());
        assert_eq!(
            m.validate(&c),
            Err(R0dSourceAuthorityMismatch::LeaseCollision {
                lease: "someone-elses-lease".to_string()
            })
        );
    }

    #[test]
    fn admission_digest_is_stable_and_state_sensitive() {
        let m = manifest();
        // Deterministic: identical manifests hash identically.
        assert_eq!(m.admission_digest(), manifest().admission_digest());
        // State-sensitive: any field change moves the digest.
        let mut m2 = manifest();
        m2.source_commit = "different".to_string();
        assert_ne!(m.admission_digest(), m2.admission_digest());
        // Collection order can NOT leak in.
        let mut m3 = manifest();
        m3.package_set.reverse();
        assert_eq!(m.admission_digest(), m3.admission_digest());
    }
}
