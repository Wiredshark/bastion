//! Exact artifact identity and verification (`APEX-T0.3`, packet sections
//! 5.2, 7.3-7.4).
//!
//! `ArtifactDigestV1 = SHA256(exact artifact bytes)`. No domain prefix,
//! filename, path, media type, timestamp, or decompressed representation
//! enters this calculation (negative canary "artifact-must-remain-plain-
//! sha256").

use sha2::{Digest, Sha256};

use super::algorithm::{DigestAlgorithmIdV1, DigestBytes32V1};
use super::error::{ArtifactVerificationErrorCodeV1, ArtifactVerificationErrorV1, DigestErrorCodeV1, DigestErrorV1};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ArtifactDigestV1 {
    pub algorithm: DigestAlgorithmIdV1,
    pub bytes: DigestBytes32V1,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ArtifactIdentityV1 {
    pub digest: ArtifactDigestV1,
    pub size_bytes: u64,
}

/// Standard exact-byte identity: one pass over `bytes`, no metadata mixed
/// in. Matches ordinary `sha256sum` over the same bytes.
pub fn hash_artifact_bytes_v1(bytes: &[u8]) -> ArtifactIdentityV1 {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out: [u8; 32] = hasher.finalize().into();
    ArtifactIdentityV1 {
        digest: ArtifactDigestV1 { algorithm: DigestAlgorithmIdV1::Sha256, bytes: DigestBytes32V1::from_array(out) },
        size_bytes: bytes.len() as u64,
    }
}

/// Streaming variant for large inputs. Enforces `max_bytes` while reading
/// (declared/observed size is checked incrementally, not only after a full
/// read), so a hostile unbounded stream cannot exhaust memory before the
/// limit is noticed.
pub fn hash_artifact_reader_v1<R: std::io::Read>(reader: &mut R, max_bytes: u64) -> Result<ArtifactIdentityV1, ArtifactReaderErrorV1> {
    let mut hasher = Sha256::new();
    let mut total: u64 = 0;
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf).map_err(ArtifactReaderErrorV1::Io)?;
        if n == 0 {
            break;
        }
        total = total
            .checked_add(n as u64)
            .ok_or(ArtifactReaderErrorV1::Digest(DigestErrorV1::new(DigestErrorCodeV1::SizeOverflow)))?;
        if total > max_bytes {
            return Err(ArtifactReaderErrorV1::Digest(DigestErrorV1::new(DigestErrorCodeV1::InputTooLarge)));
        }
        hasher.update(&buf[..n]);
    }
    let out: [u8; 32] = hasher.finalize().into();
    Ok(ArtifactIdentityV1 {
        digest: ArtifactDigestV1 { algorithm: DigestAlgorithmIdV1::Sha256, bytes: DigestBytes32V1::from_array(out) },
        size_bytes: total,
    })
}

/// `hash_artifact_reader_v1`'s error type: I/O failures are not part of the
/// closed `DigestErrorCodeV1` registry (which has no IO variant — it is
/// for pure-value digest operations), so this stays a small dedicated enum
/// rather than stretching that registry's numeric codes to also mean
/// "reader broke".
#[derive(Debug)]
pub enum ArtifactReaderErrorV1 {
    Digest(DigestErrorV1),
    Io(std::io::Error),
}

impl core::fmt::Display for ArtifactReaderErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Digest(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ArtifactReaderErrorV1 {}

/// A private token proving `bytes` were verified against `identity` at
/// construction time. Only [`verify_artifact_bytes_v1`] can produce one —
/// a pathname or prior verification elsewhere is never treated as proof
/// that later-opened bytes are the bytes that were checked.
pub struct VerifiedArtifactBytesV1<'a> {
    bytes: &'a [u8],
    identity: ArtifactIdentityV1,
}

impl<'a> VerifiedArtifactBytesV1<'a> {
    pub fn bytes(&self) -> &'a [u8] { self.bytes }

    pub fn identity(&self) -> ArtifactIdentityV1 { self.identity }
}

/// Verifies `bytes` against `expected` (size checked before digest, so the
/// two failure modes are distinguishable) and returns a verified token
/// only on exact match. No parser or consumer callback runs on failure.
pub fn verify_artifact_bytes_v1<'a>(
    bytes: &'a [u8],
    expected: &ArtifactIdentityV1,
) -> Result<VerifiedArtifactBytesV1<'a>, ArtifactVerificationErrorV1> {
    if bytes.len() as u64 != expected.size_bytes {
        return Err(ArtifactVerificationErrorV1::new(ArtifactVerificationErrorCodeV1::SizeMismatch));
    }
    let actual = hash_artifact_bytes_v1(bytes);
    if actual.digest.bytes.as_array() != expected.digest.bytes.as_array() || actual.digest.algorithm != expected.digest.algorithm {
        return Err(ArtifactVerificationErrorV1::new(ArtifactVerificationErrorCodeV1::DigestMismatch));
    }
    Ok(VerifiedArtifactBytesV1 { bytes, identity: *expected })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8; 32]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

    #[test]
    fn nist_empty_and_abc_vectors() {
        let empty = hash_artifact_bytes_v1(b"");
        assert_eq!(hex(empty.digest.bytes.as_array()), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(empty.size_bytes, 0);

        let abc = hash_artifact_bytes_v1(b"abc");
        assert_eq!(hex(abc.digest.bytes.as_array()), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(abc.size_bytes, 3);
    }

    #[test]
    fn canonical_cbor_field_map_vector() {
        // {"field":1,"value":"abc"} encoded as a1 01 63 616263 (from T0.2's own vectors).
        let payload = [0xa1u8, 0x01, 0x63, 0x61, 0x62, 0x63];
        let id = hash_artifact_bytes_v1(&payload);
        assert_eq!(hex(id.digest.bytes.as_array()), "177fd783140007221ea31840f21ceb3a3ac551dfc9fcb8df3e8df7463b27674d");
        assert_eq!(id.size_bytes, 6);
    }

    #[test]
    fn chunk_size_independence() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let whole = hash_artifact_bytes_v1(&data);
        for chunk_size in [1usize, 7, 4096] {
            let mut hasher = Sha256::new();
            for chunk in data.chunks(chunk_size) {
                hasher.update(chunk);
            }
            let out: [u8; 32] = hasher.finalize().into();
            assert_eq!(&out, whole.digest.bytes.as_array(), "chunk_size={chunk_size}");
        }
    }

    #[test]
    fn reader_matches_bytes_and_respects_limit() {
        let data = b"abc".repeat(1000);
        let mut cursor = std::io::Cursor::new(&data);
        let via_reader = hash_artifact_reader_v1(&mut cursor, u64::MAX).unwrap();
        let via_bytes = hash_artifact_bytes_v1(&data);
        assert_eq!(via_reader.digest.bytes.as_array(), via_bytes.digest.bytes.as_array());
        assert_eq!(via_reader.size_bytes, data.len() as u64);

        let mut cursor2 = std::io::Cursor::new(&data);
        let err = hash_artifact_reader_v1(&mut cursor2, 10).unwrap_err();
        assert!(matches!(err, ArtifactReaderErrorV1::Digest(e) if e.code == DigestErrorCodeV1::InputTooLarge));
    }

    #[test]
    fn one_byte_append_or_truncate_changes_digest() {
        let base = hash_artifact_bytes_v1(b"abc");
        let appended = hash_artifact_bytes_v1(b"abcd");
        let truncated = hash_artifact_bytes_v1(b"ab");
        assert_ne!(base.digest.bytes.as_array(), appended.digest.bytes.as_array());
        assert_ne!(base.digest.bytes.as_array(), truncated.digest.bytes.as_array());
    }

    #[test]
    fn verify_correct_bytes_succeeds() {
        let bytes = b"hello world";
        let identity = hash_artifact_bytes_v1(bytes);
        let token = verify_artifact_bytes_v1(bytes, &identity).unwrap();
        assert_eq!(token.bytes(), bytes);
        assert_eq!(token.identity().size_bytes, identity.size_bytes);
    }

    #[test]
    fn verify_same_length_wrong_bytes_is_digest_mismatch() {
        let identity = hash_artifact_bytes_v1(b"hello world");
        let err = match verify_artifact_bytes_v1(b"HELLO WORLD", &identity) {
            Err(e) => e,
            Ok(_) => panic!("expected DigestMismatch"),
        };
        assert_eq!(err.code, ArtifactVerificationErrorCodeV1::DigestMismatch);
    }

    #[test]
    fn verify_correct_digest_wrong_declared_size_is_size_mismatch() {
        let identity = hash_artifact_bytes_v1(b"hello world");
        let wrong_size = ArtifactIdentityV1 { size_bytes: identity.size_bytes + 1, ..identity };
        let err = match verify_artifact_bytes_v1(b"hello world", &wrong_size) {
            Err(e) => e,
            Ok(_) => panic!("expected SizeMismatch"),
        };
        assert_eq!(err.code, ArtifactVerificationErrorCodeV1::SizeMismatch);
    }

    #[test]
    fn verify_empty_artifact() {
        let identity = hash_artifact_bytes_v1(b"");
        assert!(verify_artifact_bytes_v1(b"", &identity).is_ok());
    }
}
