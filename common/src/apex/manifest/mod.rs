//! `BastionManifestEncodingV1`: restricted RFC 8949 core-deterministic CBOR
//! (`APEX-T0.2`).
//!
//! Determinism story: one accepted [`value::ManifestValueV1`] maps to
//! exactly one byte string, and decode only accepts that one byte string
//! back (`decode::decode_canonical_value_v1` re-encodes and byte-compares
//! before returning). No floats, tags, null/undefined, indefinite lengths,
//! or native-width integers can enter the model at all, so there is no
//! platform, library-version, or HashMap-iteration-order axis for the
//! output bytes to vary along. See
//! `readme/apex/BASTION-MANIFEST-ENCODING-v1.md` for the full normative
//! profile and `common/tests/apex_manifest_encoding_v1.rs` for conformance
//! against the program's golden-vector corpus.
//!
//! Only this module may reach into CBOR byte layout
//! (`encode`/`decode`/`error` submodules are private to the crate; nothing
//! outside `common::apex::manifest` constructs or inspects raw bytes).

mod encode;
mod decode;
mod error;
mod path;
mod text;
mod value;

pub use error::{ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeErrorV1, ManifestErrorV1, ManifestSchemaErrorV1};
pub use path::CanonicalPathV1;
pub use text::MachineTextV1;
pub use value::{
    ArraySemanticsV1, CanonicalFieldMapV1, CanonicalSortKeyV1, FieldIdV1, ManifestDecodeLimitsV1, ManifestValueV1,
    StructFieldsV1, VariantTagV1,
};

/// The frozen codec identity string. Not a digest — `APEX-T0.3` defines
/// domain-separated content roots on top of this encoding.
pub const BASTION_MANIFEST_ENCODING_V1: &str = "bastion.manifest-cbor.rfc8949-core/v1";

/// Implemented by a domain DTO that can become a canonical manifest value.
pub trait ManifestEncodeV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1>;
}

/// Implemented by a domain DTO that can be reconstructed from a decoded,
/// already-canonical manifest value.
pub trait ManifestDecodeV1: Sized {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1>;
}

/// Encodes `value` to canonical `BastionManifestEncodingV1` bytes.
pub fn encode_manifest_v1<T: ManifestEncodeV1>(
    value: &T,
    limits: &ManifestDecodeLimitsV1,
) -> Result<Vec<u8>, ManifestCodecErrorV1> {
    let tree = value.to_manifest_value_v1()?;
    encode::encode_manifest_value_v1(&tree, limits)
}

/// Decodes canonical `BastionManifestEncodingV1` bytes into `T`, rejecting
/// any byte sequence that is not the unique canonical encoding of an
/// accepted value, then applying `T`'s own schema validation.
pub fn decode_manifest_v1<T: ManifestDecodeV1>(
    bytes: &[u8],
    limits: &ManifestDecodeLimitsV1,
) -> Result<T, ManifestDecodeErrorV1> {
    let tree = decode::decode_canonical_value_v1(bytes, limits)?;
    T::from_manifest_value_v1(tree)
}

/// Encodes a bare [`ManifestValueV1`] tree directly, without a schema type.
/// Exposed (rather than only the private `encode` module) so golden-vector
/// conformance tests can check raw value/byte pairs, per packet T0.2.05's
/// evidence gate ("Rust bytes equal hand/RFC vectors and independent
/// oracle bytes") — this is still the same restricted encoder every
/// `ManifestEncodeV1` impl uses, not a generic byte-writer escape hatch.
pub fn encode_value_bytes_v1(value: &ManifestValueV1) -> Result<Vec<u8>, ManifestCodecErrorV1> {
    let mut out = Vec::new();
    encode::encode_value_v1(&mut out, value)?;
    Ok(out)
}

/// Decodes a bare [`ManifestValueV1`] tree directly (see
/// [`encode_value_bytes_v1`] for why this exists).
pub fn decode_value_bytes_v1(bytes: &[u8], limits: &ManifestDecodeLimitsV1) -> Result<ManifestValueV1, ManifestDecodeErrorV1> {
    decode::decode_canonical_value_v1(bytes, limits)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Example {
        a: u64,
        b: bool,
    }

    impl ManifestEncodeV1 for Example {
        fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
            let map = CanonicalFieldMapV1::try_from_entries(vec![
                (FieldIdV1::new(0), ManifestValueV1::Unsigned(self.a)),
                (FieldIdV1::new(1), ManifestValueV1::Bool(self.b)),
            ])?;
            Ok(ManifestValueV1::Map(map))
        }
    }

    impl ManifestDecodeV1 for Example {
        fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
            let ManifestValueV1::Map(map) = value else {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType));
            };
            let mut fields = StructFieldsV1::new(map);
            let a = match fields.take_required(FieldIdV1::new(0))? {
                ManifestValueV1::Unsigned(v) => v,
                _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
            };
            let b = match fields.take_required(FieldIdV1::new(1))? {
                ManifestValueV1::Bool(v) => v,
                _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
            };
            fields.finish_no_unknown()?;
            Ok(Example { a, b })
        }
    }

    fn test_limits() -> ManifestDecodeLimitsV1 {
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
    fn encode_decode_round_trip() {
        let original = Example { a: 42, b: true };
        let limits = test_limits();
        let bytes = encode_manifest_v1(&original, &limits).unwrap();
        let decoded: Example = decode_manifest_v1(&bytes, &limits).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn decode_rejects_unknown_field() {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(1)),
            (FieldIdV1::new(1), ManifestValueV1::Bool(true)),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(9)),
        ])
        .unwrap();
        let limits = test_limits();
        let bytes = encode::encode_manifest_value_v1(&ManifestValueV1::Map(map), &limits).unwrap();
        let err = decode_manifest_v1::<Example>(&bytes, &limits).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::UnknownField);
    }

    #[test]
    fn decode_rejects_missing_required_field() {
        let map = CanonicalFieldMapV1::try_from_entries(vec![(FieldIdV1::new(0), ManifestValueV1::Unsigned(1))]).unwrap();
        let limits = test_limits();
        let bytes = encode::encode_manifest_value_v1(&ManifestValueV1::Map(map), &limits).unwrap();
        let err = decode_manifest_v1::<Example>(&bytes, &limits).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::MissingRequiredField);
    }

    #[test]
    fn codec_identity_is_frozen() {
        assert_eq!(BASTION_MANIFEST_ENCODING_V1, "bastion.manifest-cbor.rfc8949-core/v1");
    }
}
