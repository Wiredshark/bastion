//! `BastionManifestEncodingV1` encoding for lifecycle identities
//! (`APEX-T0.4`, packet section 7.8).
//!
//! UUID identities encode as a definite CBOR byte string of length 16;
//! counters encode as a preferred unsigned integer. The decoder rejects
//! alternate UUID versions/variants and any noncanonical CBOR through
//! `APEX-T0.2`'s own strict decoder — this module only adds the
//! identity-specific length/version/variant checks on top.

use crate::apex::manifest::{ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1, ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1};

use super::counter::{ConnectionEpoch, PhysicsGeneration, SaveEpoch, SnapshotEpoch};
use super::opaque::{CommandId, ServerBootId, SessionId, UniverseBranchId};

fn opaque_to_value(uuid: &uuid::Uuid) -> ManifestValueV1 { ManifestValueV1::Bytes(uuid.as_bytes().to_vec()) }

fn opaque_from_value<T>(value: ManifestValueV1, from_uuid: impl FnOnce(uuid::Uuid) -> Result<T, super::error::IdentityDecodeErrorV1>) -> Result<T, ManifestSchemaErrorV1> {
    let ManifestValueV1::Bytes(b) = value else {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("expected a 16-byte bytestring"));
    };
    if b.len() != 16 {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("UUID field must be exactly 16 bytes"));
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&b);
    let uuid = uuid::Uuid::from_bytes(arr);
    from_uuid(uuid).map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("invalid UUID version/variant"))
}

macro_rules! impl_opaque_manifest_codec {
    ($ty:ty) => {
        impl ManifestEncodeV1 for $ty {
            fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(opaque_to_value(self.as_uuid())) }
        }
        impl ManifestDecodeV1 for $ty {
            fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
                opaque_from_value(value, <$ty>::from_uuid_v4)
            }
        }
    };
}

impl_opaque_manifest_codec!(ServerBootId);
impl_opaque_manifest_codec!(SessionId);
impl_opaque_manifest_codec!(CommandId);
impl_opaque_manifest_codec!(UniverseBranchId);

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
    fn uuid_field_is_16_byte_cbor_bytestring() {
        let mut source = FixedRandomBytesSourceV1([0x91, 0x91, 0x08, 0xf7, 0x52, 0xd1, 0x13, 0x20, 0x9b, 0xac, 0xf8, 0x47, 0xdb, 0x41, 0x48, 0xa8]);
        let id = ServerBootId::generate(&mut source).unwrap();
        let bytes = encode_manifest_v1(&id, &limits()).unwrap();
        // 0x50 = major2 (bytes), length 16, immediate form.
        assert_eq!(bytes[0], 0x50);
        assert_eq!(bytes.len(), 17);
        let decoded: ServerBootId = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(decoded.as_uuid(), id.as_uuid());
    }

    #[test]
    fn exact_uuid_v4_cbor_bytestring_vector() {
        // uuid_v4 vector: bytes_hex 919108f752d143209bacf847db4148a8 ->
        // cbor_bytestring_hex 50919108f752d143209bacf847db4148a8.
        let bytes: [u8; 16] = [0x91, 0x91, 0x08, 0xf7, 0x52, 0xd1, 0x43, 0x20, 0x9b, 0xac, 0xf8, 0x47, 0xdb, 0x41, 0x48, 0xa8];
        let uuid = uuid::Uuid::from_bytes(bytes);
        let id = ServerBootId::from_uuid_v4(uuid).unwrap();
        let encoded = encode_manifest_v1(&id, &limits()).unwrap();
        let hex: String = encoded.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "50919108f752d143209bacf847db4148a8");
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
