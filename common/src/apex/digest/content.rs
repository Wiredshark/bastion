//! Semantic roots and combined content identity (`APEX-T0.3`, packet
//! sections 5.5, 7.2).
//!
//! `ContentIdentityV1.semantic` defaults to `None` and stays `None` until
//! an owning row freezes a concrete semantic schema/canonicalization
//! (negative canary "semantic-never-integrity": a semantic match can only
//! ever *supplement* exact artifact verification, never substitute for
//! it — there is no API here that accepts a `SemanticRootV1` in place of
//! an `ArtifactIdentityV1` check).

use super::artifact::ArtifactIdentityV1;
use super::protocol::ProtocolDigestV1;
use crate::apex::manifest::MachineTextV1;

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticRootV1 {
    pub schema_id: MachineTextV1,
    pub canonicalization_version: u32,
    pub root: ProtocolDigestV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContentIdentityV1 {
    pub artifact: ArtifactIdentityV1,
    pub semantic: Option<SemanticRootV1>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::algorithm::DigestAlgorithmIdV1;
    use crate::apex::digest::artifact::{ArtifactDigestV1, hash_artifact_bytes_v1};
    use crate::apex::digest::domain::DigestDomainIdV1;
    use crate::apex::digest::protocol::digest_canonical_bytes_v1;

    #[test]
    fn semantic_defaults_to_none() {
        let identity = hash_artifact_bytes_v1(b"content");
        let content = ContentIdentityV1 { artifact: identity, semantic: None };
        assert!(content.semantic.is_none());
    }

    #[test]
    fn semantic_root_is_additive_not_a_substitute() {
        // A ContentIdentityV1 with a semantic root still carries its own
        // independent artifact identity -- there is no code path that
        // derives artifact identity FROM the semantic root.
        let artifact = hash_artifact_bytes_v1(b"payload bytes");
        let root = digest_canonical_bytes_v1(DigestDomainIdV1::SemanticContent, &[0xa0], 1 << 20).unwrap();
        let semantic =
            SemanticRootV1 { schema_id: MachineTextV1::new("example/v1").unwrap(), canonicalization_version: 1, root };
        let content = ContentIdentityV1 { artifact, semantic: Some(semantic) };
        assert_eq!(content.artifact.digest.algorithm, DigestAlgorithmIdV1::Sha256);
        assert!(content.semantic.is_some());
    }

    #[test]
    fn artifact_digest_is_never_domain_framed() {
        // Negative canary "artifact-must-remain-plain-sha256": the exact
        // artifact digest for a payload must equal plain SHA-256 of that
        // payload, never the domain-separated protocol digest of it.
        let payload = [0xa0u8];
        let artifact = hash_artifact_bytes_v1(&payload);
        let protocol = digest_canonical_bytes_v1(DigestDomainIdV1::BootstrapManifest, &payload, 1 << 20).unwrap();
        let plain = ArtifactDigestV1 { algorithm: DigestAlgorithmIdV1::Sha256, bytes: artifact.digest.bytes };
        assert_ne!(plain.bytes.as_array(), protocol.bytes.as_array());
    }
}
