//! `SubsystemDescriptorV1` and the typed optional protocol-root newtypes
//! (`APEX-T0.5`, spec sections 3.2-3.3).

use crate::apex::digest::ContentIdentityV1;
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};
use crate::apex::scalar::{ProtocolVersion, SchemaVersion};

use super::slot::SubsystemSlotIdV1;

/// One descriptor identifies one subsystem artifact at one schema
/// version. `content` reuses [`ContentIdentityV1`] verbatim — the row's
/// own explicit anti-goal is "without duplicated identity fields".
#[derive(Clone, Debug, PartialEq)]
pub struct SubsystemDescriptorV1 {
    pub slot: SubsystemSlotIdV1,
    pub schema: SchemaVersion,
    pub content: ContentIdentityV1,
}

impl ManifestEncodeV1 for SubsystemDescriptorV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(self.slot.as_u16() as u64)),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(self.schema.get() as u64)),
            (FieldIdV1::new(3), self.content.to_manifest_value_v1()?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for SubsystemDescriptorV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let slot_raw = match fields.take_required(FieldIdV1::new(1))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let slot = SubsystemSlotIdV1::try_from_u16(slot_raw)
            .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown subsystem slot id"))?;
        let schema = match fields.take_required(FieldIdV1::new(2))? {
            ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => SchemaVersion::new(v as u32),
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let content = ContentIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(3))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { slot, schema, content })
    }
}

/// Declares one typed optional protocol-root newtype delegating to
/// [`ProtocolVersion`] (`APEX-T0.1`). One distinct type per protocol root
/// a downstream row actually needs — never a bare `Option<ProtocolVersion>`
/// reused across subsystems (spec section 3.3: "without... ambiguous
/// class semantics").
macro_rules! protocol_root_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(ProtocolVersion);

        impl $name {
            pub const fn new(inner: ProtocolVersion) -> Self { Self(inner) }
            pub const fn get(self) -> ProtocolVersion { self.0 }
        }

        impl ManifestEncodeV1 for $name {
            fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
                Ok(ManifestValueV1::Unsigned(self.0.get() as u64))
            }
        }

        impl ManifestDecodeV1 for $name {
            fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
                match value {
                    ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => Ok(Self(ProtocolVersion::new(v as u32))),
                    _ => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
                }
            }
        }
    };
}

protocol_root_newtype!(
    /// `APEX-T4.3`'s worldgen protocol root.
    WorldgenProtocolVersion
);
protocol_root_newtype!(
    /// `APEX-T4.1`/`APEX-T4.3`'s content protocol root.
    ContentProtocolVersion
);
protocol_root_newtype!(
    /// `APEX-T4.3`/`APEX-T6.1`'s numeric protocol root.
    NumericProtocolVersion
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::hash_artifact_bytes_v1;
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
    fn descriptor_round_trips() {
        let original = SubsystemDescriptorV1 {
            slot: SubsystemSlotIdV1::Worldgen,
            schema: SchemaVersion::new(3),
            content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"worldgen"), semantic: None },
        };
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: SubsystemDescriptorV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn descriptor_rejects_unknown_slot_id() {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(9999)),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(1)),
            (
                FieldIdV1::new(3),
                ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"x"), semantic: None }.to_manifest_value_v1().unwrap(),
            ),
        ])
        .unwrap();
        let bytes = encode_manifest_v1(&RawWrapper(ManifestValueV1::Map(map)), &limits()).unwrap();
        let err = decode_manifest_v1::<SubsystemDescriptorV1>(&bytes, &limits()).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::FieldKeyType);
    }

    #[test]
    fn protocol_root_newtypes_round_trip_and_are_mutually_distinct_types() {
        let w = WorldgenProtocolVersion::new(ProtocolVersion::new(7));
        let bytes = encode_manifest_v1(&w, &limits()).unwrap();
        let decoded: WorldgenProtocolVersion = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(w, decoded);
        // Compile-time property (not a runtime assertion): there is no
        // `From<WorldgenProtocolVersion> for NumericProtocolVersion` and no
        // shared inner-field access outside this module, so a worldgen root
        // cannot be silently substituted for a numeric root at a call site
        // expecting `NumericProtocolVersion`.
    }

    /// Test-only pass-through wrapper, same pattern as `common/src/apex/digest/mod.rs`'s
    /// `ArtifactDigestWrapperForTest`: lets a hostile test build an arbitrary
    /// manifest value tree without going through the real checked constructor.
    struct RawWrapper(ManifestValueV1);
    impl ManifestEncodeV1 for RawWrapper {
        fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(self.0.clone()) }
    }
}
