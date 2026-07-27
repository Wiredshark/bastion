//! Direct transform registration and lookup (`APEX-T0.5`, spec section
//! 3.6). Deliberately does not compute or execute multi-hop transform
//! chains — the row's own anti-goal, deferred to `APEX-T4.5`/`T4.6`.

use crate::apex::digest::ArtifactIdentityV1;
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};
use crate::apex::scalar::SchemaVersion;

/// Build-time-frozen `u32` tag for one transform's semantics (build step
/// 1: "freeze... transform semantics").
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TransformIdV1(u32);

impl TransformIdV1 {
    pub const fn new(inner: u32) -> Self { Self(inner) }
    pub const fn get(self) -> u32 { self.0 }
}

/// The exact lookup key: `(transform_id, from_schema, to_schema,
/// implementation_root)`. Two entries with the same key but different
/// `implementation_root` are a conflicting registration, not a duplicate
/// (see [`TransformRegistryV1::register`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TransformKeyV1 {
    pub transform_id: TransformIdV1,
    pub from_schema: SchemaVersion,
    pub to_schema: SchemaVersion,
    pub implementation_root: ArtifactIdentityV1,
}

/// Typed, exhaustive failure for [`TransformRegistryV1::register`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformRegistrationErrorV1 {
    /// The exact same key (including `implementation_root`) was already registered.
    Duplicate,
    /// The same `(transform_id, from_schema, to_schema)` triple is already
    /// registered under a *different* `implementation_root` — a
    /// conflicting, not merely duplicate, registration.
    Conflict,
}

/// Exact-key transform lookup. No multi-hop graph planning or execution —
/// direct lookup only.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TransformRegistryV1 {
    entries: Vec<TransformKeyV1>,
}

impl TransformRegistryV1 {
    pub fn new() -> Self { Self { entries: Vec::new() } }

    pub fn register(&mut self, key: TransformKeyV1) -> Result<(), TransformRegistrationErrorV1> {
        for existing in &self.entries {
            if *existing == key {
                return Err(TransformRegistrationErrorV1::Duplicate);
            }
            if existing.transform_id == key.transform_id
                && existing.from_schema == key.from_schema
                && existing.to_schema == key.to_schema
            {
                return Err(TransformRegistrationErrorV1::Conflict);
            }
        }
        self.entries.push(key);
        Ok(())
    }

    pub fn contains(&self, key: &TransformKeyV1) -> bool { self.entries.iter().any(|e| e == key) }

    pub fn entries(&self) -> &[TransformKeyV1] { &self.entries }
}

impl ManifestEncodeV1 for TransformKeyV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(self.transform_id.get() as u64)),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(self.from_schema.get() as u64)),
            (FieldIdV1::new(3), ManifestValueV1::Unsigned(self.to_schema.get() as u64)),
            (FieldIdV1::new(4), self.implementation_root.to_manifest_value_v1()?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for TransformKeyV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let transform_id = match fields.take_required(FieldIdV1::new(1))? {
            ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => TransformIdV1::new(v as u32),
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let from_schema = match fields.take_required(FieldIdV1::new(2))? {
            ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => SchemaVersion::new(v as u32),
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let to_schema = match fields.take_required(FieldIdV1::new(3))? {
            ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => SchemaVersion::new(v as u32),
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let implementation_root = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(4))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { transform_id, from_schema, to_schema, implementation_root })
    }
}

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

    fn key(root_seed: &[u8]) -> TransformKeyV1 {
        TransformKeyV1 {
            transform_id: TransformIdV1::new(1),
            from_schema: SchemaVersion::new(1),
            to_schema: SchemaVersion::new(2),
            implementation_root: hash_artifact_bytes_v1(root_seed),
        }
    }

    #[test]
    fn register_rejects_exact_duplicate() {
        let mut reg = TransformRegistryV1::new();
        let k = key(b"impl");
        assert!(reg.register(k).is_ok());
        assert_eq!(reg.register(k), Err(TransformRegistrationErrorV1::Duplicate));
    }

    #[test]
    fn register_rejects_conflicting_implementation_root() {
        let mut reg = TransformRegistryV1::new();
        assert!(reg.register(key(b"impl-a")).is_ok());
        assert_eq!(reg.register(key(b"impl-b")), Err(TransformRegistrationErrorV1::Conflict));
    }

    #[test]
    fn register_allows_distinct_triples() {
        let mut reg = TransformRegistryV1::new();
        assert!(reg.register(key(b"impl")).is_ok());
        let mut other = key(b"impl");
        other.to_schema = SchemaVersion::new(3);
        assert!(reg.register(other).is_ok());
    }

    #[test]
    fn transform_key_round_trips() {
        let original = key(b"impl");
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: TransformKeyV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }
}
