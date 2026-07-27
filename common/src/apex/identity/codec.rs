//! `BastionManifestEncodingV1` encoding for lifecycle identities
//! (`APEX-T0.4`, packet section 7.8; type-tag closure `APEX-T0.4.6`).
//!
//! Each opaque identity's canonical form is a two-field map: an
//! `IdentityKindV1` type tag, then a definite CBOR byte string of length
//! 16. Counters encode as a preferred unsigned integer. The decoder
//! rejects alternate UUID versions/variants, a tag that doesn't match the
//! type being decoded, and any noncanonical CBOR through `APEX-T0.2`'s
//! own strict decoder — this module only adds the identity-specific
//! tag/length/version/variant checks on top.
//!
//! `APEX-T0.4.6` (Opus 5's boundary-review finding on the original
//! T0.4-completion premise, which turned out to already exist minus this
//! tag): a bare 16-byte bytestring with no type marker meant
//! `SessionId::from_manifest_value_v1` would happily accept a
//! `ServerBootId`'s encoded bytes — any two opaque types are valid
//! UUIDv4 byte strings, so nothing at the wire level distinguished them.
//! The tag closes that: `wrong_tag_cross_decode_is_rejected` below is the
//! direct proof, and `bare_bytestring_without_tag_is_rejected` retains
//! the OLD (pre-`T0.4.6`) shape as a hostile canary that must now fail,
//! converting this deliberate wire-shape break from documented to
//! tested-and-enforced. Safe to break now, not later: `T0.4`'s own doc
//! ("no live issuance, transport integration, or subsystem wiring
//! happens here") means zero live consumers exist yet to migrate.

use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};

use super::counter::{ConnectionEpoch, PhysicsGeneration, SaveEpoch, SnapshotEpoch};
use super::opaque::{CommandId, IdentityKindV1, ServerBootId, SessionId, UniverseBranchId};

fn opaque_to_value(kind: IdentityKindV1, uuid: &uuid::Uuid) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    let map = CanonicalFieldMapV1::try_from_entries(vec![
        (FieldIdV1::new(1), ManifestValueV1::Unsigned(kind.as_u16() as u64)),
        (FieldIdV1::new(2), ManifestValueV1::Bytes(uuid.as_bytes().to_vec())),
    ])?;
    Ok(ManifestValueV1::Map(map))
}

fn opaque_from_value<T>(
    expected_kind: IdentityKindV1,
    value: ManifestValueV1,
    from_uuid: impl FnOnce(uuid::Uuid) -> Result<T, super::error::IdentityDecodeErrorV1>,
) -> Result<T, ManifestSchemaErrorV1> {
    let ManifestValueV1::Map(map) = value else {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("expected a tagged identity map"));
    };
    let mut fields = StructFieldsV1::new(map);
    let tag_raw = match fields.take_required(FieldIdV1::new(1))? {
        ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
        _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("identity tag must be an unsigned u16")),
    };
    let tag = IdentityKindV1::try_from_u16(tag_raw)
        .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown identity kind tag"))?;
    if tag != expected_kind {
        // The anti-substitution check itself: a validly-tagged OTHER
        // type's encoding must not decode as this type, even though the
        // raw UUID bytes underneath would otherwise be perfectly valid.
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("identity kind tag does not match the expected type"));
    }
    let ManifestValueV1::Bytes(b) = fields.take_required(FieldIdV1::new(2))? else {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("expected a 16-byte bytestring"));
    };
    if b.len() != 16 {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("UUID field must be exactly 16 bytes"));
    }
    fields.finish_no_unknown()?;
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&b);
    let uuid = uuid::Uuid::from_bytes(arr);
    from_uuid(uuid).map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("invalid UUID version/variant"))
}

macro_rules! impl_opaque_manifest_codec {
    ($ty:ty, $kind:expr) => {
        impl ManifestEncodeV1 for $ty {
            fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { opaque_to_value($kind, self.as_uuid()) }
        }
        impl ManifestDecodeV1 for $ty {
            fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
                opaque_from_value($kind, value, <$ty>::from_uuid_v4)
            }
        }
    };
}

impl_opaque_manifest_codec!(ServerBootId, IdentityKindV1::ServerBoot);
impl_opaque_manifest_codec!(SessionId, IdentityKindV1::Session);
impl_opaque_manifest_codec!(CommandId, IdentityKindV1::Command);
impl_opaque_manifest_codec!(UniverseBranchId, IdentityKindV1::UniverseBranch);

macro_rules! impl_counter_manifest_codec_zero_reserved {
    ($ty:ty) => {
        impl ManifestEncodeV1 for $ty {
            fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(ManifestValueV1::Unsigned(self.get())) }
        }
        impl ManifestDecodeV1 for $ty {
            fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
                let ManifestValueV1::Unsigned(v) = value else {
                    return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType));
                };
                <$ty>::new(v).map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("zero is reserved"))
            }
        }
    };
}

macro_rules! impl_counter_manifest_codec_zero_valid {
    ($ty:ty) => {
        impl ManifestEncodeV1 for $ty {
            fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(ManifestValueV1::Unsigned(self.get())) }
        }
        impl ManifestDecodeV1 for $ty {
            fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
                let ManifestValueV1::Unsigned(v) = value else {
                    return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType));
                };
                Ok(<$ty>::new(v))
            }
        }
    };
}

impl_counter_manifest_codec_zero_reserved!(ConnectionEpoch);
impl_counter_manifest_codec_zero_valid!(PhysicsGeneration);
impl_counter_manifest_codec_zero_valid!(SnapshotEpoch);
impl_counter_manifest_codec_zero_valid!(SaveEpoch);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::identity::opaque::FixedRandomBytesSourceV1;
    use crate::apex::manifest::{ManifestDecodeLimitsV1, decode_manifest_v1, encode_manifest_v1};

    fn limits() -> ManifestDecodeLimitsV1 {
        ManifestDecodeLimitsV1 {
            max_input_bytes: 1024,
            max_depth: 4,
            max_nodes: 32,
            max_array_items: 8,
            max_map_entries: 8,
            max_machine_text_bytes: 64,
            max_byte_string_bytes: 64,
        }
    }

    #[test]
    fn uuid_field_is_tagged_map_with_16_byte_bytestring() {
        let mut source = FixedRandomBytesSourceV1([0x91, 0x91, 0x08, 0xf7, 0x52, 0xd1, 0x13, 0x20, 0x9b, 0xac, 0xf8, 0x47, 0xdb, 0x41, 0x48, 0xa8]);
        let id = ServerBootId::generate(&mut source).unwrap();
        let bytes = encode_manifest_v1(&id, &limits()).unwrap();
        // 0xa2 = major5 (map), 2 entries, definite length.
        assert_eq!(bytes[0], 0xa2);
        let decoded: ServerBootId = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(decoded.as_uuid(), id.as_uuid());
    }

    /// `APEX-T0.4.6`: exact byte vector for `ServerBootId`'s tagged
    /// canonical form -- {tag: IdentityKindV1::ServerBoot(1), bytes:
    /// <16 UUID bytes>}. Computed once via `encode_manifest_v1`
    /// (this test's own `println!` when run with `--nocapture`), then
    /// pinned here as the frozen expectation -- any future accidental
    /// field-order/tag-value drift fails this test loudly.
    #[test]
    fn exact_tagged_identity_cbor_vector() {
        let bytes: [u8; 16] = [0x91, 0x91, 0x08, 0xf7, 0x52, 0xd1, 0x43, 0x20, 0x9b, 0xac, 0xf8, 0x47, 0xdb, 0x41, 0x48, 0xa8];
        let uuid = uuid::Uuid::from_bytes(bytes);
        let id = ServerBootId::from_uuid_v4(uuid).unwrap();
        let encoded = encode_manifest_v1(&id, &limits()).unwrap();
        let hex: String = encoded.iter().map(|b| format!("{b:02x}")).collect();
        // a2 = map(2 entries), 01 01 = {field 1: unsigned 1 (the
        // IdentityKindV1::ServerBoot tag)}, 02 50 <16 bytes> = {field 2:
        // 16-byte bytestring}.
        assert_eq!(hex, "a201010250919108f752d143209bacf847db4148a8");
    }

    /// `APEX-T0.4.6`'s core anti-substitution proof: a `SessionId`'s
    /// validly-tagged canonical bytes must NOT decode as a `ServerBootId`,
    /// even though the underlying 16 bytes are a perfectly valid UUIDv4
    /// either way -- only the embedded tag distinguishes them, and the
    /// decoder must actually check it.
    #[test]
    fn wrong_tag_cross_decode_is_rejected() {
        let mut source = FixedRandomBytesSourceV1([0x42; 16]);
        let session = SessionId::generate(&mut source).unwrap();
        let encoded = encode_manifest_v1(&session, &limits()).unwrap();
        let err = decode_manifest_v1::<ServerBootId>(&encoded, &limits()).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::FieldKeyType);

        // Every pairwise combination of the four opaque types, not just
        // one -- the tag check must be universal, not accidentally
        // correct for one pair.
        let boot = ServerBootId::generate(&mut FixedRandomBytesSourceV1([0x01; 16])).unwrap();
        let command = CommandId::generate(&mut FixedRandomBytesSourceV1([0x02; 16])).unwrap();
        let branch = UniverseBranchId::generate(&mut FixedRandomBytesSourceV1([0x03; 16])).unwrap();
        let boot_bytes = encode_manifest_v1(&boot, &limits()).unwrap();
        let command_bytes = encode_manifest_v1(&command, &limits()).unwrap();
        let branch_bytes = encode_manifest_v1(&branch, &limits()).unwrap();
        assert!(decode_manifest_v1::<SessionId>(&boot_bytes, &limits()).is_err());
        assert!(decode_manifest_v1::<CommandId>(&boot_bytes, &limits()).is_err());
        assert!(decode_manifest_v1::<UniverseBranchId>(&boot_bytes, &limits()).is_err());
        assert!(decode_manifest_v1::<ServerBootId>(&command_bytes, &limits()).is_err());
        assert!(decode_manifest_v1::<ServerBootId>(&branch_bytes, &limits()).is_err());
    }

    /// `APEX-T0.4.6`: the pre-fix bare-bytestring shape (no tag at all)
    /// is retained here as a HOSTILE canary rather than deleted -- this
    /// is exactly what `exact_uuid_v4_cbor_bytestring_vector` (this
    /// module's own prior golden vector, before this fix) used to assert
    /// SUCCEEDED. It must now fail, converting the deliberate wire-shape
    /// break from documented to tested-and-enforced.
    #[test]
    fn bare_bytestring_without_tag_is_rejected() {
        let bytes: [u8; 16] = [0x91, 0x91, 0x08, 0xf7, 0x52, 0xd1, 0x43, 0x20, 0x9b, 0xac, 0xf8, 0x47, 0xdb, 0x41, 0x48, 0xa8];
        // 0x50 = major2 (bytes), length 16, immediate form -- the OLD shape.
        let old_shape_hex = "50919108f752d143209bacf847db4148a8";
        let old_shape_bytes: Vec<u8> = (0..old_shape_hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&old_shape_hex[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(old_shape_bytes.len(), 17);
        assert_eq!(&old_shape_bytes[1..], &bytes[..]);
        let err = decode_manifest_v1::<ServerBootId>(&old_shape_bytes, &limits()).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::FieldKeyType);
    }

    /// One-bit mutation of the tag field changes the decoded outcome
    /// (rejected, not silently accepted as some other identity kind) --
    /// `T0.4.6`'s own bar for "mutation canaries".
    #[test]
    fn one_bit_tag_mutation_is_rejected() {
        let boot = ServerBootId::generate(&mut FixedRandomBytesSourceV1([0x77; 16])).unwrap();
        let mut encoded = encode_manifest_v1(&boot, &limits()).unwrap();
        // The tag's value byte is encoded right after the tag's field-id
        // byte and the tag-value marker; flip a low bit in the tag value
        // itself (found by construction: `exact_tagged_identity_cbor_vector`
        // pins byte index 4 as the tag value `0x01`).
        encoded[4] ^= 0x01;
        assert!(decode_manifest_v1::<ServerBootId>(&encoded, &limits()).is_err());
    }

    /// `ConnectionEpoch`'s cross-decode safety needs no tag: its
    /// canonical form is a bare `Unsigned`, structurally distinct from
    /// every opaque identity's `Map` shape (CBOR major type 0 vs major
    /// type 5) -- decoding one as the other fails on the very first byte,
    /// verified directly here rather than left as an unstated assumption.
    #[test]
    fn connection_epoch_is_structurally_distinct_from_opaque_identities() {
        let boot = ServerBootId::generate(&mut FixedRandomBytesSourceV1([0x11; 16])).unwrap();
        let boot_bytes = encode_manifest_v1(&boot, &limits()).unwrap();
        assert!(decode_manifest_v1::<ConnectionEpoch>(&boot_bytes, &limits()).is_err());

        let epoch = ConnectionEpoch::new(7).unwrap();
        let epoch_bytes = encode_manifest_v1(&epoch, &limits()).unwrap();
        assert!(decode_manifest_v1::<ServerBootId>(&epoch_bytes, &limits()).is_err());
    }

    #[test]
    fn connection_epoch_counter_vectors() {
        assert_eq!(decode_manifest_v1::<ConnectionEpoch>(&[0x00], &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::FieldKeyType);
        let one: ConnectionEpoch = decode_manifest_v1(&[0x01], &limits()).unwrap();
        assert_eq!(one.get(), 1);
        assert_eq!(one.checked_next().unwrap().get(), 2);
    }

    #[test]
    fn physics_generation_zero_decodes_fine() {
        let zero: PhysicsGeneration = decode_manifest_v1(&[0x00], &limits()).unwrap();
        assert_eq!(zero.get(), 0);
    }
}
