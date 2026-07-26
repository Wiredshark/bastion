//! Domain-separated digests and exact content identity (`APEX-T0.3`).
//!
//! Two deliberately different identities, kept mechanically hard to
//! confuse:
//!
//! 1. **Exact artifact identity** ([`artifact::ArtifactIdentityV1`]) —
//!    plain `SHA256(raw bytes)` plus the exact byte count. Protects byte
//!    integrity; no domain framing, ever.
//! 2. **Protocol/semantic root** ([`protocol::ProtocolDigestV1`]) —
//!    `SHA256` of a domain-framed preimage over `BastionManifestEncodingV1`
//!    canonical bytes. Identifies a purpose-scoped root; never verifies
//!    transferred bytes by itself.

mod algorithm;
mod artifact;
mod content;
mod domain;
mod error;
mod protocol;

pub use algorithm::{DigestAlgorithmIdV1, DigestBytes32V1};
pub use artifact::{
    ArtifactDigestV1, ArtifactIdentityV1, ArtifactReaderErrorV1, VerifiedArtifactBytesV1, hash_artifact_bytes_v1,
    hash_artifact_reader_v1, verify_artifact_bytes_v1,
};
pub use content::{ContentIdentityV1, SemanticRootV1};
pub use domain::DigestDomainIdV1;
pub use error::{ArtifactVerificationErrorCodeV1, ArtifactVerificationErrorV1, DigestErrorCodeV1, DigestErrorV1};
pub use protocol::{ProtocolDigestV1, digest_canonical_bytes_v1, digest_manifest_value_v1};

use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, MachineTextV1, StructFieldsV1,
};

fn digest_bytestring(bytes: &DigestBytes32V1) -> ManifestValueV1 { ManifestValueV1::Bytes(bytes.as_array().to_vec()) }

fn take_digest_bytestring(value: ManifestValueV1) -> Result<DigestBytes32V1, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Bytes(b) => DigestBytes32V1::try_from_slice(&b)
            .map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("digest is not 32 bytes")),
        _ => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
    }
}

impl ManifestEncodeV1 for ArtifactDigestV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(self.algorithm.as_u16() as u64)),
            (FieldIdV1::new(2), digest_bytestring(&self.bytes)),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for ArtifactDigestV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let algorithm_raw = match fields.take_required(FieldIdV1::new(1))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let algorithm = DigestAlgorithmIdV1::try_from_u16(algorithm_raw)
            .map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unsupported algorithm"))?;
        let bytes = take_digest_bytestring(fields.take_required(FieldIdV1::new(2))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { algorithm, bytes })
    }
}

impl ManifestEncodeV1 for ArtifactIdentityV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), self.digest.to_manifest_value_v1()?),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(self.size_bytes)),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for ArtifactIdentityV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let digest = ArtifactDigestV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let size_bytes = match fields.take_required(FieldIdV1::new(2))? {
            ManifestValueV1::Unsigned(v) => v,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        fields.finish_no_unknown()?;
        Ok(Self { digest, size_bytes })
    }
}

impl ManifestEncodeV1 for ProtocolDigestV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(self.algorithm.as_u16() as u64)),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(self.domain.as_u16() as u64)),
            (FieldIdV1::new(3), digest_bytestring(&self.bytes)),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for ProtocolDigestV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let algorithm_raw = match fields.take_required(FieldIdV1::new(1))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let algorithm = DigestAlgorithmIdV1::try_from_u16(algorithm_raw)
            .map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unsupported algorithm"))?;
        let domain_raw = match fields.take_required(FieldIdV1::new(2))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let domain = domain::DigestDomainIdV1::ALL
            .into_iter()
            .find(|d| d.as_u16() == domain_raw)
            .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown domain id"))?;
        let bytes = take_digest_bytestring(fields.take_required(FieldIdV1::new(3))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { algorithm, domain, bytes })
    }
}

impl ManifestEncodeV1 for SemanticRootV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::MachineText(self.schema_id.clone())),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(self.canonicalization_version as u64)),
            (FieldIdV1::new(3), self.root.to_manifest_value_v1()?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SemanticRootV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let schema_id: MachineTextV1 = match fields.take_required(FieldIdV1::new(1))? {
            ManifestValueV1::MachineText(t) => t,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let canonicalization_version = match fields.take_required(FieldIdV1::new(2))? {
            ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => v as u32,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let root = ProtocolDigestV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(3))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { schema_id, canonicalization_version, root })
    }
}

impl ManifestEncodeV1 for ContentIdentityV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut entries = vec![(FieldIdV1::new(1), self.artifact.to_manifest_value_v1()?)];
        if let Some(semantic) = &self.semantic {
            entries.push((FieldIdV1::new(2), semantic.to_manifest_value_v1()?));
        }
        let map = CanonicalFieldMapV1::try_from_entries(entries)?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for ContentIdentityV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let artifact = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let semantic = match fields.take_optional(FieldIdV1::new(2))? {
            Some(v) => Some(SemanticRootV1::from_manifest_value_v1(v)?),
            None => None,
        };
        fields.finish_no_unknown()?;
        Ok(Self { artifact, semantic })
    }
}

#[cfg(test)]
mod encoding_tests {
    use super::*;
    use crate::apex::manifest::{ManifestDecodeLimitsV1, decode_manifest_v1, encode_manifest_v1};

    fn limits() -> ManifestDecodeLimitsV1 {
        ManifestDecodeLimitsV1 {
            max_input_bytes: 4096,
            max_depth: 8,
            max_nodes: 256,
            max_array_items: 64,
            max_map_entries: 64,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 256,
        }
    }

    #[test]
    fn content_identity_round_trips_without_semantic() {
        let artifact = hash_artifact_bytes_v1(b"payload");
        let original = ContentIdentityV1 { artifact, semantic: None };
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: ContentIdentityV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn content_identity_round_trips_with_semantic() {
        let artifact = hash_artifact_bytes_v1(b"payload");
        let root = digest_canonical_bytes_v1(DigestDomainIdV1::SemanticContent, &[0xa0], 1 << 20).unwrap();
        let semantic =
            SemanticRootV1 { schema_id: MachineTextV1::new("example/v1").unwrap(), canonicalization_version: 3, root };
        let original = ContentIdentityV1 { artifact, semantic: Some(semantic) };
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: ContentIdentityV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn unknown_algorithm_id_fails_closed() {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(99)),
            (FieldIdV1::new(2), ManifestValueV1::Bytes(vec![0u8; 32])),
        ])
        .unwrap();
        let bytes = encode_manifest_v1(
            &ArtifactDigestWrapperForTest(ManifestValueV1::Map(map)),
            &limits(),
        )
        .unwrap();
        let err = decode_manifest_v1::<ArtifactDigestV1>(&bytes, &limits()).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::FieldKeyType);
    }

    /// Test-only pass-through wrapper: lets the unknown-algorithm test
    /// build an arbitrary manifest value tree without going through
    /// ArtifactDigestV1's own (correctly restrictive) constructor.
    struct ArtifactDigestWrapperForTest(ManifestValueV1);
    impl ManifestEncodeV1 for ArtifactDigestWrapperForTest {
        fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(self.0.clone()) }
    }
}
