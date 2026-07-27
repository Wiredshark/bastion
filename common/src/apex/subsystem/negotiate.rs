//! Capability negotiation: catalog, requirement, and explicit selector
//! ownership (`APEX-T0.5`, spec section 3.6).
//!
//! Determinism story: `NegotiationSelectorV1` makes the owner, algorithm,
//! and version explicit, encoded, checked fields — two runs given the
//! same profile and catalog input always select the same outcome, and a
//! selector disagreement between peers is a typed `InvalidInput`, never a
//! silent default to one side's preference.

use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};
use crate::apex::scalar::SchemaVersion;

/// A capability identifier: build-time-frozen `u32` tag. Not a closed enum
/// (unlike [`super::slot::SubsystemSlotIdV1`]) — capabilities are an
/// open, growing catalog owned by individual subsystems, not a small
/// vocabulary this row itself enumerates.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CapabilityIdV1(u32);

impl CapabilityIdV1 {
    pub const fn new(inner: u32) -> Self { Self(inner) }
    pub const fn get(self) -> u32 { self.0 }
}

/// Typed, exhaustive failure for [`CapabilityRequirementV1::new`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityRequirementErrorV1 {
    EmptyCatalog,
    DuplicateCapability(CapabilityIdV1),
}

/// A non-empty, duplicate-free, canonically sorted set of required
/// capabilities. Checked at construction — an empty or duplicate-bearing
/// requirement cannot be built.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRequirementV1 {
    catalog: Vec<CapabilityIdV1>,
}

impl CapabilityRequirementV1 {
    pub fn new(mut catalog: Vec<CapabilityIdV1>) -> Result<Self, CapabilityRequirementErrorV1> {
        if catalog.is_empty() {
            return Err(CapabilityRequirementErrorV1::EmptyCatalog);
        }
        catalog.sort();
        for w in catalog.windows(2) {
            if w[0] == w[1] {
                return Err(CapabilityRequirementErrorV1::DuplicateCapability(w[0]));
            }
        }
        Ok(Self { catalog })
    }

    pub fn catalog(&self) -> &[CapabilityIdV1] { &self.catalog }
}

/// Which side's preference wins a negotiated-capability decision. Explicit
/// by construction — never inferred from message arrival order.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u16)]
pub enum SelectorOwnerV1 {
    ServerAuthoritative = 1,
    ClientPreferred = 2,
}

impl SelectorOwnerV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const ALL: [SelectorOwnerV1; 2] = [Self::ServerAuthoritative, Self::ClientPreferred];

    pub fn try_from_u16(raw: u16) -> Option<Self> { Self::ALL.into_iter().find(|s| s.as_u16() == raw) }
}

/// The deterministic algorithm a [`SelectorOwnerV1`] applies once it owns
/// the decision. Closed vocabulary, extensible by appending a new variant
/// (same convention as [`crate::apex::digest::DigestDomainIdV1`]).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u16)]
pub enum SelectorAlgorithmV1 {
    HighestMutualVersion = 1,
    ExactMatchOnly = 2,
}

impl SelectorAlgorithmV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const ALL: [SelectorAlgorithmV1; 2] = [Self::HighestMutualVersion, Self::ExactMatchOnly];

    pub fn try_from_u16(raw: u16) -> Option<Self> { Self::ALL.into_iter().find(|s| s.as_u16() == raw) }
}

/// Explicit owner/algorithm/version for a negotiated-capability decision.
/// Two profiles whose rules agree but whose selectors disagree must never
/// silently fall back to one side — see spec section 2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NegotiationSelectorV1 {
    pub owner: SelectorOwnerV1,
    pub algorithm: SelectorAlgorithmV1,
    pub version: SchemaVersion,
}

impl ManifestEncodeV1 for CapabilityRequirementV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let items = self.catalog.iter().map(|c| ManifestValueV1::Unsigned(c.get() as u64)).collect();
        Ok(ManifestValueV1::Array(items))
    }
}

impl ManifestDecodeV1 for CapabilityRequirementV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Array(items) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut catalog = Vec::with_capacity(items.len());
        for item in items {
            match item {
                ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => catalog.push(CapabilityIdV1::new(v as u32)),
                _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
            }
        }
        Self::new(catalog).map_err(|e| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail(match e {
            CapabilityRequirementErrorV1::EmptyCatalog => "empty capability catalog",
            CapabilityRequirementErrorV1::DuplicateCapability(_) => "duplicate capability id",
        }))
    }
}

impl ManifestEncodeV1 for NegotiationSelectorV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(self.owner.as_u16() as u64)),
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(self.algorithm.as_u16() as u64)),
            (FieldIdV1::new(3), ManifestValueV1::Unsigned(self.version.get() as u64)),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for NegotiationSelectorV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let owner_raw = match fields.take_required(FieldIdV1::new(1))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let owner = SelectorOwnerV1::try_from_u16(owner_raw)
            .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown selector owner"))?;
        let algorithm_raw = match fields.take_required(FieldIdV1::new(2))? {
            ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        let algorithm = SelectorAlgorithmV1::try_from_u16(algorithm_raw)
            .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown selector algorithm"))?;
        let version = match fields.take_required(FieldIdV1::new(3))? {
            ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => SchemaVersion::new(v as u32),
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
        };
        fields.finish_no_unknown()?;
        Ok(Self { owner, algorithm, version })
    }
}

#[cfg(test)]
mod tests {
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
    fn capability_requirement_rejects_empty() {
        assert_eq!(CapabilityRequirementV1::new(vec![]), Err(CapabilityRequirementErrorV1::EmptyCatalog));
    }

    #[test]
    fn capability_requirement_rejects_duplicates() {
        let a = CapabilityIdV1::new(1);
        assert_eq!(CapabilityRequirementV1::new(vec![a, a]), Err(CapabilityRequirementErrorV1::DuplicateCapability(a)));
    }

    #[test]
    fn capability_requirement_sorts_canonically() {
        let req = CapabilityRequirementV1::new(vec![CapabilityIdV1::new(5), CapabilityIdV1::new(1), CapabilityIdV1::new(3)]).unwrap();
        assert_eq!(req.catalog(), &[CapabilityIdV1::new(1), CapabilityIdV1::new(3), CapabilityIdV1::new(5)]);
    }

    #[test]
    fn capability_requirement_round_trips() {
        let original = CapabilityRequirementV1::new(vec![CapabilityIdV1::new(2), CapabilityIdV1::new(9)]).unwrap();
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: CapabilityRequirementV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn negotiation_selector_round_trips() {
        let original = NegotiationSelectorV1 {
            owner: SelectorOwnerV1::ServerAuthoritative,
            algorithm: SelectorAlgorithmV1::HighestMutualVersion,
            version: SchemaVersion::new(1),
        };
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: NegotiationSelectorV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn selector_owner_and_algorithm_try_from_u16_round_trip_and_reject_unknown() {
        for o in SelectorOwnerV1::ALL {
            assert_eq!(SelectorOwnerV1::try_from_u16(o.as_u16()), Some(o));
        }
        assert_eq!(SelectorOwnerV1::try_from_u16(9999), None);
        for a in SelectorAlgorithmV1::ALL {
            assert_eq!(SelectorAlgorithmV1::try_from_u16(a.as_u16()), Some(a));
        }
        assert_eq!(SelectorAlgorithmV1::try_from_u16(9999), None);
    }
}
