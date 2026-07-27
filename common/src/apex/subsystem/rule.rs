//! `CompatibilityRuleV1` — tagged, invalid combinations unrepresentable
//! (`APEX-T0.5`, spec section 3.4/3.7).
//!
//! Wire envelope (this row's own layering, not T0.2's core codec):
//! `Map { 0: VariantTagV1 as Unsigned, 1: ExtensionCriticalityV1 as
//! Unsigned, 2: payload }`. The six foundational variants always encode
//! `criticality = Critical` (the value is inert for them today — every
//! build implementing `APEX-T0.5` at all recognizes them — but present
//! for wire-envelope uniformity and forward schema evolution). A
//! genuinely unrecognized tag decodes to [`CompatibilityRuleV1::Unknown`],
//! never a hard decode error by itself — only *this row's evaluator*
//! (not the codec) decides fail-closed vs tolerate, per `criticality`
//! (spec section 3.7).
//!
//! Forward-compatibility convention, honestly recorded rather than
//! silently dropped: any *future* variant this build does not yet know
//! about must wire its payload (field 2) as a raw `Bytes` leaf, not a
//! nested structured value — this is the only way an old decoder can
//! preserve unrecognized content byte-exactly without a
//! `ManifestDecodeLimitsV1` value to re-parse a nested subtree with
//! (`from_manifest_value_v1` intentionally has no limits parameter; see
//! `common/src/apex/manifest/mod.rs`). A future variant author who sends
//! a raw structured payload instead of a `Bytes` leaf makes their
//! extension undecodable by any build that predates it — a decode
//! failure, never a silently wrong/lossy round-trip. This build's own
//! six known variants are unaffected: their payload shapes are decided
//! locally by this same decoder, not reconstructed from an opaque blob.

use crate::apex::digest::ContentIdentityV1;
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1, VariantTagV1,
};
use crate::apex::scalar::SchemaVersion;

use super::negotiate::CapabilityRequirementV1;
use super::transform::TransformKeyV1;

/// Whether an unrecognized [`CompatibilityRuleV1::Unknown`] fails the
/// containing slot closed or is tolerated with evidence retained (spec
/// section 3.7).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u16)]
pub enum ExtensionCriticalityV1 {
    Critical = 0,
    Noncritical = 1,
}

impl ExtensionCriticalityV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const fn try_from_u16(raw: u16) -> Option<Self> {
        match raw {
            0 => Some(Self::Critical),
            1 => Some(Self::Noncritical),
            _ => None,
        }
    }
}

/// Checked-nonempty, checked-duplicate-free, canonically sorted set of
/// acceptable schema versions. Private field: the only way to build one
/// is [`AcceptSetV1::new`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptSetV1 {
    schemas: Vec<SchemaVersion>,
}

/// Checked `min <= max` schema range. Private fields: the only way to
/// build one is [`AcceptRangeV1::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptRangeV1 {
    min: SchemaVersion,
    max: SchemaVersion,
}

/// Typed, exhaustive failure for [`AcceptSetV1::new`]/[`AcceptRangeV1::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityRuleConstructionErrorV1 {
    EmptyAcceptSet,
    DuplicateSchemaInAcceptSet(SchemaVersion),
    InvertedAcceptRange { min: SchemaVersion, max: SchemaVersion },
}

impl AcceptSetV1 {
    pub fn new(mut schemas: Vec<SchemaVersion>) -> Result<Self, CompatibilityRuleConstructionErrorV1> {
        if schemas.is_empty() {
            return Err(CompatibilityRuleConstructionErrorV1::EmptyAcceptSet);
        }
        schemas.sort();
        for w in schemas.windows(2) {
            if w[0] == w[1] {
                return Err(CompatibilityRuleConstructionErrorV1::DuplicateSchemaInAcceptSet(w[0]));
            }
        }
        Ok(Self { schemas })
    }

    pub fn schemas(&self) -> &[SchemaVersion] { &self.schemas }
}

impl AcceptRangeV1 {
    pub fn new(min: SchemaVersion, max: SchemaVersion) -> Result<Self, CompatibilityRuleConstructionErrorV1> {
        if min > max {
            return Err(CompatibilityRuleConstructionErrorV1::InvertedAcceptRange { min, max });
        }
        Ok(Self { min, max })
    }

    pub fn min(&self) -> SchemaVersion { self.min }

    pub fn max(&self) -> SchemaVersion { self.max }

    pub fn contains(&self, v: SchemaVersion) -> bool { self.min <= v && v <= self.max }
}

/// Tagged compatibility rule. Every variant except `ProvenanceOnly` and
/// `Unknown` carries exactly the fields it needs — no shared
/// Option-everything struct, per spec section 3.4.
#[derive(Clone, Debug, PartialEq)]
pub enum CompatibilityRuleV1 {
    Exact { content: ContentIdentityV1 },
    AcceptSet(AcceptSetV1),
    AcceptRange(AcceptRangeV1),
    NegotiatedCapability { requirement: CapabilityRequirementV1 },
    DirectTransform { key: TransformKeyV1 },
    /// Informational only, never gates compatibility.
    ProvenanceOnly,
    /// A rule variant this build's enum cannot name. See module docs.
    Unknown { tag: VariantTagV1, criticality: ExtensionCriticalityV1, raw_payload: Vec<u8> },
}

const TAG_EXACT: u16 = 1;
const TAG_ACCEPT_SET: u16 = 2;
const TAG_ACCEPT_RANGE: u16 = 3;
const TAG_NEGOTIATED_CAPABILITY: u16 = 4;
const TAG_DIRECT_TRANSFORM: u16 = 5;
const TAG_PROVENANCE_ONLY: u16 = 6;

impl ManifestEncodeV1 for CompatibilityRuleV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let (tag, criticality, payload): (u16, ExtensionCriticalityV1, ManifestValueV1) = match self {
            Self::Exact { content } => (TAG_EXACT, ExtensionCriticalityV1::Critical, content.to_manifest_value_v1()?),
            Self::AcceptSet(set) => {
                let items = set.schemas.iter().map(|s| ManifestValueV1::Unsigned(s.get() as u64)).collect();
                (TAG_ACCEPT_SET, ExtensionCriticalityV1::Critical, ManifestValueV1::Array(items))
            },
            Self::AcceptRange(range) => {
                let map = CanonicalFieldMapV1::try_from_entries(vec![
                    (FieldIdV1::new(1), ManifestValueV1::Unsigned(range.min.get() as u64)),
                    (FieldIdV1::new(2), ManifestValueV1::Unsigned(range.max.get() as u64)),
                ])?;
                (TAG_ACCEPT_RANGE, ExtensionCriticalityV1::Critical, ManifestValueV1::Map(map))
            },
            Self::NegotiatedCapability { requirement } => {
                (TAG_NEGOTIATED_CAPABILITY, ExtensionCriticalityV1::Critical, requirement.to_manifest_value_v1()?)
            },
            Self::DirectTransform { key } => (TAG_DIRECT_TRANSFORM, ExtensionCriticalityV1::Critical, key.to_manifest_value_v1()?),
            Self::ProvenanceOnly => (TAG_PROVENANCE_ONLY, ExtensionCriticalityV1::Critical, ManifestValueV1::Bytes(Vec::new())),
            Self::Unknown { tag, criticality, raw_payload } => (tag.get(), *criticality, ManifestValueV1::Bytes(raw_payload.clone())),
        };
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(tag as u64)),
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(criticality.as_u16() as u64)),
            (FieldIdV1::new(2), payload),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for CompatibilityRuleV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let tag_raw = match fields.take_required(FieldIdV1::new(0))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let criticality_raw = match fields.take_required(FieldIdV1::new(1))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let criticality = ExtensionCriticalityV1::try_from_u16(criticality_raw)
            .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown extension criticality"))?;
        let payload = fields.take_required(FieldIdV1::new(2))?;
        fields.finish_no_unknown()?;

        let rule = match tag_raw {
            TAG_EXACT => Self::Exact { content: ContentIdentityV1::from_manifest_value_v1(payload)? },
            TAG_ACCEPT_SET => {
                let ManifestValueV1::Array(items) = payload else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
                let mut schemas = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => schemas.push(SchemaVersion::new(v as u32)),
                        _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
                    }
                }
                let set = AcceptSetV1::new(schemas)
                    .map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("invalid AcceptSet wire content"))?;
                Self::AcceptSet(set)
            },
            TAG_ACCEPT_RANGE => {
                let ManifestValueV1::Map(inner_map) = payload else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
                let mut inner = StructFieldsV1::new(inner_map);
                let min = match inner.take_required(FieldIdV1::new(1))? {
                    ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => SchemaVersion::new(v as u32),
                    _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
                };
                let max = match inner.take_required(FieldIdV1::new(2))? {
                    ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => SchemaVersion::new(v as u32),
                    _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
                };
                inner.finish_no_unknown()?;
                let range = AcceptRangeV1::new(min, max)
                    .map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("invalid AcceptRange wire content: min > max"))?;
                Self::AcceptRange(range)
            },
            TAG_NEGOTIATED_CAPABILITY => {
                Self::NegotiatedCapability { requirement: CapabilityRequirementV1::from_manifest_value_v1(payload)? }
            },
            TAG_DIRECT_TRANSFORM => Self::DirectTransform { key: TransformKeyV1::from_manifest_value_v1(payload)? },
            TAG_PROVENANCE_ONLY => Self::ProvenanceOnly,
            unrecognized => {
                let ManifestValueV1::Bytes(raw_payload) = payload else {
                    return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)
                        .detail("unrecognized rule variant's payload must be a raw Bytes leaf for forward-compatible pass-through"));
                };
                Self::Unknown { tag: VariantTagV1::new(unrecognized), criticality, raw_payload }
            },
        };
        Ok(rule)
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

    #[test]
    fn accept_set_rejects_empty() {
        assert_eq!(AcceptSetV1::new(vec![]), Err(CompatibilityRuleConstructionErrorV1::EmptyAcceptSet));
    }

    #[test]
    fn accept_set_rejects_duplicates() {
        let v = SchemaVersion::new(1);
        assert_eq!(AcceptSetV1::new(vec![v, v]), Err(CompatibilityRuleConstructionErrorV1::DuplicateSchemaInAcceptSet(v)));
    }

    #[test]
    fn accept_range_rejects_inverted() {
        let min = SchemaVersion::new(5);
        let max = SchemaVersion::new(1);
        assert_eq!(AcceptRangeV1::new(min, max), Err(CompatibilityRuleConstructionErrorV1::InvertedAcceptRange { min, max }));
    }

    #[test]
    fn accept_range_allows_equal_bounds() {
        let v = SchemaVersion::new(3);
        assert!(AcceptRangeV1::new(v, v).is_ok());
    }

    #[test]
    fn every_known_variant_round_trips() {
        let cases = vec![
            CompatibilityRuleV1::Exact { content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"x"), semantic: None } },
            CompatibilityRuleV1::AcceptSet(AcceptSetV1::new(vec![SchemaVersion::new(1), SchemaVersion::new(3)]).unwrap()),
            CompatibilityRuleV1::AcceptRange(AcceptRangeV1::new(SchemaVersion::new(1), SchemaVersion::new(5)).unwrap()),
            CompatibilityRuleV1::NegotiatedCapability {
                requirement: super::super::negotiate::CapabilityRequirementV1::new(vec![super::super::negotiate::CapabilityIdV1::new(1)]).unwrap(),
            },
            CompatibilityRuleV1::DirectTransform {
                key: TransformKeyV1 {
                    transform_id: super::super::transform::TransformIdV1::new(1),
                    from_schema: SchemaVersion::new(1),
                    to_schema: SchemaVersion::new(2),
                    implementation_root: hash_artifact_bytes_v1(b"impl"),
                },
            },
            CompatibilityRuleV1::ProvenanceOnly,
            CompatibilityRuleV1::Unknown {
                tag: VariantTagV1::new(9999),
                criticality: ExtensionCriticalityV1::Noncritical,
                raw_payload: vec![1, 2, 3],
            },
        ];
        for original in cases {
            let bytes = encode_manifest_v1(&original, &limits()).unwrap();
            let decoded: CompatibilityRuleV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
            assert_eq!(original, decoded, "round-trip mismatch");
        }
    }

    #[test]
    fn unrecognized_tag_decodes_to_unknown_not_a_hard_error() {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(777)),
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(ExtensionCriticalityV1::Critical.as_u16() as u64)),
            (FieldIdV1::new(2), ManifestValueV1::Bytes(vec![0xde, 0xad])),
        ])
        .unwrap();
        let bytes = encode_manifest_v1(&RawWrapper(ManifestValueV1::Map(map)), &limits()).unwrap();
        let decoded: CompatibilityRuleV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        match decoded {
            CompatibilityRuleV1::Unknown { tag, criticality, raw_payload } => {
                assert_eq!(tag.get(), 777);
                assert_eq!(criticality, ExtensionCriticalityV1::Critical);
                assert!(!raw_payload.is_empty());
            },
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    /// Non-vacuity for the forward-compatibility convention documented in
    /// this module's header: an unrecognized tag whose payload is a raw
    /// structured value (not a `Bytes` leaf) cannot be safely preserved
    /// without a `ManifestDecodeLimitsV1` to re-parse it with, so it must
    /// fail decode rather than silently drop/misrepresent the content.
    #[test]
    fn unrecognized_tag_with_non_bytes_payload_fails_decode() {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(778)),
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(ExtensionCriticalityV1::Noncritical.as_u16() as u64)),
            (FieldIdV1::new(2), ManifestValueV1::Array(vec![ManifestValueV1::Unsigned(1)])),
        ])
        .unwrap();
        let bytes = encode_manifest_v1(&RawWrapper(ManifestValueV1::Map(map)), &limits()).unwrap();
        let err = decode_manifest_v1::<CompatibilityRuleV1>(&bytes, &limits()).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::FieldKeyType);
    }

    /// Non-vacuity / decode-vs-InvalidInput boundary (spec section 3.7
    /// clarification): a KNOWN variant (`AcceptRange`) whose wire content
    /// violates its own construction invariant (`min > max`) -- content
    /// that could never be produced by calling `AcceptRange::new` locally
    /// -- must fail decode entirely, not decode into some in-memory shape
    /// later flagged `InvalidInput` by the evaluator.
    #[test]
    fn known_variant_with_invalid_wire_content_is_a_decode_failure() {
        let inner = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(5)), // min = 5
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(1)), // max = 1, inverted
        ])
        .unwrap();
        let outer = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(TAG_ACCEPT_RANGE as u64)),
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(ExtensionCriticalityV1::Critical.as_u16() as u64)),
            (FieldIdV1::new(2), ManifestValueV1::Map(inner)),
        ])
        .unwrap();
        let bytes = encode_manifest_v1(&RawWrapper(ManifestValueV1::Map(outer)), &limits()).unwrap();
        let err = decode_manifest_v1::<CompatibilityRuleV1>(&bytes, &limits()).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::FieldKeyType);
    }

    /// An `AcceptSet` with an empty wire-encoded array is the same
    /// decode-failure class as the inverted `AcceptRange` above --
    /// checked here directly since `CompatibilityProfileV1` (spec section
    /// 3.5) is what proves this poisons a whole *profile's* decode, not
    /// just one rule.
    #[test]
    fn empty_accept_set_wire_content_is_a_decode_failure() {
        let outer = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(TAG_ACCEPT_SET as u64)),
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(ExtensionCriticalityV1::Critical.as_u16() as u64)),
            (FieldIdV1::new(2), ManifestValueV1::Array(vec![])),
        ])
        .unwrap();
        let bytes = encode_manifest_v1(&RawWrapper(ManifestValueV1::Map(outer)), &limits()).unwrap();
        let err = decode_manifest_v1::<CompatibilityRuleV1>(&bytes, &limits()).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::FieldKeyType);
    }

    struct RawWrapper(ManifestValueV1);
    impl ManifestEncodeV1 for RawWrapper {
        fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(self.0.clone()) }
    }
}
