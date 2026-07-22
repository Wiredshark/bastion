//! BUILD-007A10.8 (part 1) — deterministic artifact identity and durable
//! publication (design §15).
//!
//! Self-contained substrate: the canonical logical-path grammar (§15.1), the
//! artifact descriptor + logical-path-sorted index (§15.2), and the RUN-COMMIT
//! admission identity (§15.3) — a run exists for admission only when a valid
//! RUN-COMMIT references the exact index and terminal digests. The actual
//! flush/sync/rename durability protocol (§15.3 steps / §15.4 platform tiers) is
//! the filesystem integration surface.

/// Typed logical-path failures (§15.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalPathError {
    Empty,
    TooLong { len: usize, max: usize },
    SegmentTooLong { len: usize, max: usize },
    EmptyOrDotSegment,
    IllegalChar,
    TrailingDotOrSpace,
    WindowsReservedName { name: String },
    AbsoluteOrDrivePrefix,
}

const PATH_MAX: usize = 240;
const SEGMENT_MAX: usize = 64;

const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Validate a canonical logical path (§15.1): lowercase ASCII, slash-separated,
/// <=240 bytes total and <=64 per segment; no empty/`.`/`..` segments, no
/// backslash/colon/control/trailing-dot-space, no Windows reserved basename
/// (case-insensitive), no absolute/drive/UNC prefix.
pub fn validate_logical_path(path: &str) -> Result<(), LogicalPathError> {
    if path.is_empty() {
        return Err(LogicalPathError::Empty);
    }
    if path.len() > PATH_MAX {
        return Err(LogicalPathError::TooLong { len: path.len(), max: PATH_MAX });
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(LogicalPathError::AbsoluteOrDrivePrefix);
    }
    let b = path.as_bytes();
    if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        return Err(LogicalPathError::AbsoluteOrDrivePrefix);
    }
    for &c in b {
        if c < 0x20 || c == 0x7f {
            return Err(LogicalPathError::IllegalChar);
        }
        if c == b'\\' || c == b':' || c >= 0x80 || c.is_ascii_uppercase() {
            return Err(LogicalPathError::IllegalChar);
        }
    }
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(LogicalPathError::EmptyOrDotSegment);
        }
        if seg.len() > SEGMENT_MAX {
            return Err(LogicalPathError::SegmentTooLong { len: seg.len(), max: SEGMENT_MAX });
        }
        if seg.ends_with('.') || seg.ends_with(' ') {
            return Err(LogicalPathError::TrailingDotOrSpace);
        }
        let stem = seg.split('.').next().unwrap_or(seg);
        if WINDOWS_RESERVED.contains(&stem) {
            return Err(LogicalPathError::WindowsReservedName { name: stem.to_string() });
        }
    }
    Ok(())
}

/// Whether an artifact is canonical evidence or a diagnostic (§15.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactClass {
    Canonical,
    Diagnostic,
}

/// One artifact descriptor (§15.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDescriptorV1 {
    pub logical_path: String,
    pub media_type: String,
    pub role_tag: u16,
    pub size: u64,
    pub sha256: [u8; 32],
    pub producer_authority: u16,
    pub semantic_frame_token: u64,
    pub class: ArtifactClass,
}

/// Publication-index failures (§15.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublicationError {
    InvalidPath { path: String, reason: LogicalPathError },
    DuplicatePath { path: String },
}

/// A sorted publication index (§15.2). Entries sort by logical-path bytes, never
/// directory enumeration order; the digest binds the sorted descriptor set.
#[derive(Clone, Debug)]
pub struct PublicationIndexV1 {
    descriptors: Vec<ArtifactDescriptorV1>,
}

impl PublicationIndexV1 {
    /// Build the index (§15.2): validate every logical path, reject duplicates,
    /// sort by logical-path bytes.
    pub fn build(mut descriptors: Vec<ArtifactDescriptorV1>) -> Result<Self, PublicationError> {
        for d in &descriptors {
            validate_logical_path(&d.logical_path)
                .map_err(|reason| PublicationError::InvalidPath { path: d.logical_path.clone(), reason })?;
        }
        descriptors.sort_by(|a, b| a.logical_path.as_bytes().cmp(b.logical_path.as_bytes()));
        for w in descriptors.windows(2) {
            if w[0].logical_path == w[1].logical_path {
                return Err(PublicationError::DuplicatePath { path: w[0].logical_path.clone() });
            }
        }
        Ok(Self { descriptors })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    /// Domain-separated index digest over the sorted descriptors (§15.2).
    #[must_use]
    pub fn index_digest(&self) -> [u8; 32] {
        let mut p = Vec::new();
        p.extend_from_slice(&(self.descriptors.len() as u64).to_le_bytes());
        for d in &self.descriptors {
            p.extend_from_slice(&(d.logical_path.len() as u64).to_le_bytes());
            p.extend_from_slice(d.logical_path.as_bytes());
            p.extend_from_slice(&(d.media_type.len() as u64).to_le_bytes());
            p.extend_from_slice(d.media_type.as_bytes());
            p.extend_from_slice(&d.role_tag.to_le_bytes());
            p.extend_from_slice(&d.size.to_le_bytes());
            p.extend_from_slice(&d.sha256);
            p.extend_from_slice(&d.producer_authority.to_le_bytes());
            p.extend_from_slice(&d.semantic_frame_token.to_le_bytes());
            p.push(match d.class {
                ArtifactClass::Canonical => 0,
                ArtifactClass::Diagnostic => 1,
            });
        }
        crate::domain_hash("bastion/r0d/publication-index", 1, 0, &p)
    }
}

/// The RUN-COMMIT admission marker (§15.3): binds the exact index and terminal
/// digests. Published last; a run is admissible only when a valid commit matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunCommitV1 {
    pub index_digest: [u8; 32],
    pub terminal_digest: [u8; 32],
}

impl RunCommitV1 {
    /// A run exists for admission only when the RUN-COMMIT references the EXACT
    /// index and terminal digests (§15.3). Any mismatch => not admitted.
    #[must_use]
    pub fn admits(&self, index_digest: &[u8; 32], terminal_digest: &[u8; 32]) -> bool {
        &self.index_digest == index_digest && &self.terminal_digest == terminal_digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_publication_path() {
        assert!(validate_logical_path("r0d/v1/arena/0123456789ab/server/42-7/color.png").is_ok());
    }

    #[test]
    fn rejects_each_bad_path() {
        use LogicalPathError::*;
        assert_eq!(validate_logical_path(""), Err(Empty));
        assert_eq!(validate_logical_path("a\\b"), Err(IllegalChar));
        assert_eq!(validate_logical_path("ab:c"), Err(IllegalChar)); // colon not in drive position
        assert_eq!(validate_logical_path("A/b"), Err(IllegalChar));
        assert_eq!(validate_logical_path("a/../b"), Err(EmptyOrDotSegment));
        assert_eq!(validate_logical_path("a//b"), Err(EmptyOrDotSegment));
        assert_eq!(validate_logical_path("a/b."), Err(TrailingDotOrSpace));
        assert_eq!(validate_logical_path("/abs"), Err(AbsoluteOrDrivePrefix));
        assert_eq!(validate_logical_path("c:/x"), Err(AbsoluteOrDrivePrefix));
        assert_eq!(
            validate_logical_path("a/nul/b"),
            Err(WindowsReservedName { name: "nul".to_string() })
        );
        assert!(matches!(validate_logical_path(&"x".repeat(241)), Err(TooLong { .. })));
        assert!(matches!(
            validate_logical_path(&format!("a/{}/b", "x".repeat(65))),
            Err(SegmentTooLong { .. })
        ));
    }

    fn desc(path: &str, size: u64) -> ArtifactDescriptorV1 {
        ArtifactDescriptorV1 {
            logical_path: path.to_string(),
            media_type: "application/octet-stream".to_string(),
            role_tag: 1,
            size,
            sha256: [size as u8; 32],
            producer_authority: 0,
            semantic_frame_token: 0,
            class: ArtifactClass::Canonical,
        }
    }

    #[test]
    fn index_is_path_sorted_and_order_independent() {
        let a = PublicationIndexV1::build(vec![desc("r0d/b", 2), desc("r0d/a", 1)]).unwrap();
        let b = PublicationIndexV1::build(vec![desc("r0d/a", 1), desc("r0d/b", 2)]).unwrap();
        assert_eq!(a.index_digest(), b.index_digest());
    }

    #[test]
    fn duplicate_logical_path_rejected() {
        assert_eq!(
            PublicationIndexV1::build(vec![desc("r0d/a", 1), desc("r0d/a", 2)]).unwrap_err(),
            PublicationError::DuplicatePath { path: "r0d/a".to_string() }
        );
    }

    #[test]
    fn run_commit_admits_only_exact_digests() {
        let idx = PublicationIndexV1::build(vec![desc("r0d/a", 1)]).unwrap();
        let index_digest = idx.index_digest();
        let terminal_digest = [0x9u8; 32];
        let commit = RunCommitV1 { index_digest, terminal_digest };
        assert!(commit.admits(&index_digest, &terminal_digest));
        assert!(!commit.admits(&[0; 32], &terminal_digest)); // wrong index
        assert!(!commit.admits(&index_digest, &[0; 32])); // wrong terminal
    }
}
