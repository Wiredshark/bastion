//! `APEX-T2.5.09` — verify-before-cache artifact staging.
//!
//! Wrong bytes must never reach a parser or Wasmtime: every artifact is
//! verified (membership → size → digest) BEFORE it is written to its
//! final path, writes go through a temp file + rename, and every read
//! back re-verifies the digest (a corrupt cache hit is a typed terminal,
//! not a loaded plugin). The cache is disposable — deleting it costs a
//! re-download, never correctness.

use common::apex::digest::{ArtifactIdentityV1, hash_artifact_bytes_v1};
use std::path::PathBuf;

#[derive(Debug)]
pub enum ArtifactCacheErrorV1 {
    /// Ordinal not in the deployment's requirement set.
    UnrequestedArtifact { ordinal: u32 },
    /// Cheap check first: byte count differs from the requirement.
    SizeMismatch { ordinal: u32, expected: u64, got: u64 },
    /// Full digest differs from the requirement.
    DigestMismatch { ordinal: u32 },
    /// A cached file re-read later no longer matches its digest.
    CorruptCachedArtifact { ordinal: u32 },
    /// Artifact not staged yet.
    NotStaged { ordinal: u32 },
    Io { detail: String },
}

fn io_err(e: std::io::Error) -> ArtifactCacheErrorV1 {
    ArtifactCacheErrorV1::Io { detail: e.to_string() }
}

/// One deployment's staging cache. Constructed FROM the deployment plan's
/// requirement set — bytes for anything else are unrequested by
/// construction.
pub struct PluginArtifactCacheV1 {
    root: PathBuf,
    requirements: Vec<(u32, ArtifactIdentityV1)>,
}

impl PluginArtifactCacheV1 {
    pub fn new(
        root: PathBuf,
        requirements: Vec<(u32, ArtifactIdentityV1)>,
    ) -> Result<Self, ArtifactCacheErrorV1> {
        std::fs::create_dir_all(&root).map_err(io_err)?;
        Ok(Self { root, requirements })
    }

    fn requirement(&self, ordinal: u32) -> Result<&ArtifactIdentityV1, ArtifactCacheErrorV1> {
        self.requirements
            .iter()
            .find(|(o, _)| *o == ordinal)
            .map(|(_, a)| a)
            .ok_or(ArtifactCacheErrorV1::UnrequestedArtifact { ordinal })
    }

    fn final_path(&self, artifact: &ArtifactIdentityV1) -> PathBuf {
        // Content-addressed: duplicate stages of identical bytes are
        // idempotent; distinct ordinals sharing bytes share one file.
        let mut name = String::with_capacity(68);
        for b in artifact.digest.bytes.as_array() {
            name.push_str(&format!("{b:02x}"));
        }
        name.push_str(".bin");
        self.root.join(name)
    }

    /// Verify then publish: membership → size → digest → temp write →
    /// rename. Failure at any step leaves no final-path file behind.
    pub fn stage(&self, ordinal: u32, bytes: &[u8]) -> Result<PathBuf, ArtifactCacheErrorV1> {
        let expected = self.requirement(ordinal)?;
        if bytes.len() as u64 != expected.size_bytes {
            return Err(ArtifactCacheErrorV1::SizeMismatch {
                ordinal,
                expected: expected.size_bytes,
                got: bytes.len() as u64,
            });
        }
        if hash_artifact_bytes_v1(bytes) != *expected {
            return Err(ArtifactCacheErrorV1::DigestMismatch { ordinal });
        }
        let final_path = self.final_path(expected);
        let tmp = self.root.join(format!("tmp-{ordinal}-{}", std::process::id()));
        std::fs::write(&tmp, bytes).map_err(io_err)?;
        std::fs::rename(&tmp, &final_path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            io_err(e)
        })?;
        Ok(final_path)
    }

    /// The ONLY read path: re-verifies the digest so corrupt/partial
    /// cache contents surface as typed terminals, never as loader input.
    pub fn open_verified(&self, ordinal: u32) -> Result<Vec<u8>, ArtifactCacheErrorV1> {
        let expected = self.requirement(ordinal)?;
        let path = self.final_path(expected);
        if !path.is_file() {
            return Err(ArtifactCacheErrorV1::NotStaged { ordinal });
        }
        let bytes = std::fs::read(&path).map_err(io_err)?;
        if hash_artifact_bytes_v1(&bytes) != *expected {
            return Err(ArtifactCacheErrorV1::CorruptCachedArtifact { ordinal });
        }
        Ok(bytes)
    }

    /// True only if staged AND currently verifying.
    pub fn is_staged_verified(&self, ordinal: u32) -> bool {
        self.open_verified(ordinal).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_artifact_cache_v1_verifies_before_and_after_publication() {
        let dir = tempfile::tempdir().unwrap();
        let good = b"wasm-bytes-good".to_vec();
        let other = b"other-artifact-bytes".to_vec();
        let reqs = vec![(0u32, hash_artifact_bytes_v1(&good)), (1u32, hash_artifact_bytes_v1(&other))];
        let cache = PluginArtifactCacheV1::new(dir.path().join("cache"), reqs).unwrap();

        // Wrong ordinal (unrequested) refused before any write.
        assert!(matches!(cache.stage(7, &good), Err(ArtifactCacheErrorV1::UnrequestedArtifact { ordinal: 7 })));
        // Truncated ("partial write" on the wire) refused by size.
        assert!(matches!(
            cache.stage(0, &good[..4]),
            Err(ArtifactCacheErrorV1::SizeMismatch { ordinal: 0, .. })
        ));
        // Same size, different bytes refused by digest.
        let mut flipped = good.clone();
        flipped[0] ^= 0xff;
        assert!(matches!(cache.stage(0, &flipped), Err(ArtifactCacheErrorV1::DigestMismatch { ordinal: 0 })));
        // Nothing reached a final path; reads say NotStaged.
        assert!(matches!(cache.open_verified(0), Err(ArtifactCacheErrorV1::NotStaged { ordinal: 0 })));

        // Good bytes stage and read back verified.
        let final_path = cache.stage(0, &good).unwrap();
        assert_eq!(cache.open_verified(0).unwrap(), good);
        // Duplicate stage is idempotent.
        assert_eq!(cache.stage(0, &good).unwrap(), final_path);

        // Corrupt cache hit: mutate the published file on disk — the read
        // path must refuse it, never hand it to a loader.
        std::fs::write(&final_path, b"corrupted-after-publication!").unwrap();
        assert!(matches!(
            cache.open_verified(0),
            Err(ArtifactCacheErrorV1::CorruptCachedArtifact { ordinal: 0 })
        ));
        assert!(!cache.is_staged_verified(0));

        // Ordinal 1 unaffected throughout.
        cache.stage(1, &other).unwrap();
        assert_eq!(cache.open_verified(1).unwrap(), other);
    }
}
