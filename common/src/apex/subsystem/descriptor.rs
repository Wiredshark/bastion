//! `SubsystemDescriptorV1` and the typed optional protocol-root newtypes
//! (`APEX-T0.5`, spec sections 3.2-3.3).

use crate::apex::digest::{ContentIdentityV1, ProtocolDigestV1};
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};
use crate::apex::scalar::SchemaVersion;

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

/// Declares one typed protocol-root newtype carrying a
/// [`ProtocolDigestV1`]. One distinct type per protocol root a
/// downstream row actually needs — never a bare digest reused across
/// subsystems (spec section 3.3: "without... ambiguous class
/// semantics").
///
/// **AMENDED `T4-PV`, 2026-07-29 — the payload widened from
/// `ProtocolVersion` (u32) to `ProtocolDigestV1` (32 bytes + domain).**
///
/// Original decision, recorded verbatim because how a boundary was
/// arrived at is part of what it means: these were `ProtocolVersion`
/// newtypes, and `T4.3` shipped them that way while explicitly banking
/// "the REAL derivation ... from an honest frozen vocabulary" for a
/// later chunk.
///
/// That later chunk is `T4-PV`, and its premise-check found the two
/// halves could not meet: `T4.3` ruled the derivation must follow
/// `net_envelope_profile_root_v1`'s pattern, which returns 32 bytes,
/// while the field could hold 32 BITS. Truncating to fit would let two
/// different vocabularies collide into one root — a save adopted
/// against a world that no longer exists, which is the silent failure
/// `T4.3` exists to prevent. The `u32` had no independent meaning to
/// lose: every construction site in the tree was a hand-written `1` or
/// `2` in a test, i.e. a placeholder for exactly this derivation.
///
/// Orchestrator-ruled 2026-07-29: widen. All three widen together —
/// a mixed-width triple is worse than either uniform state.
macro_rules! protocol_root_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        // No serde derive, unlike the pre-amendment u32 form: these now
        // carry a ProtocolDigestV1, whose own type deliberately has no
        // serde either. The manifest codec is this program's canonical
        // encoding for identity, and a second uncontrolled one is how a
        // root acquires two byte-forms that disagree.
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(ProtocolDigestV1);

        impl $name {
            pub const fn new(inner: ProtocolDigestV1) -> Self { Self(inner) }
            pub const fn get(self) -> ProtocolDigestV1 { self.0 }
        }

        impl ManifestEncodeV1 for $name {
            fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
                self.0.to_manifest_value_v1()
            }
        }

        impl ManifestDecodeV1 for $name {
            fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
                Ok(Self(ProtocolDigestV1::from_manifest_value_v1(value)?))
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
    use crate::apex::digest::{DigestDomainIdV1, digest_canonical_bytes_v1, hash_artifact_bytes_v1};
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
        let w = WorldgenProtocolVersion::new(digest_canonical_bytes_v1(DigestDomainIdV1::WorldgenProtocolRoot, &[7], 64).expect("test digest"));
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
