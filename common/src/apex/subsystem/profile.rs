//! `CompatibilityProfileV1` — canonical typed-array of unique slot rules
//! (`APEX-T0.5`, spec section 3.5).

use crate::apex::manifest::{
    ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1, ManifestErrorV1,
    ManifestSchemaErrorV1, ManifestValueV1,
};

use super::rule::CompatibilityRuleV1;
use super::slot::SubsystemSlotIdV1;

/// Max entries a single profile may declare (build step 4: "complete
/// cardinality and bounds validation"). One profile has at most one rule
/// per known slot, so this bound is generous, not load-bearing on its
/// own — [`CompatibilityProfileV1::new`]'s duplicate-slot check is what
/// actually enforces uniqueness.
pub const MAX_PROFILE_ENTRIES: usize = 256;

/// Typed, exhaustive failure for [`CompatibilityProfileV1::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityProfileErrorV1 {
    DuplicateSlot(SubsystemSlotIdV1),
    TooManyEntries { count: usize, max: usize },
}

/// Checked-unique-per-slot, canonically slot-tag-ordered set of
/// `(SubsystemSlotIdV1, CompatibilityRuleV1)` rules. Canonical iteration
/// order is slot-tag order, not insertion order — this is what makes
/// [`super::report::CompatibilityReportV1`]'s total order possible
/// without a separate sort step hiding non-determinism.
#[derive(Clone, Debug, PartialEq)]
pub struct CompatibilityProfileV1 {
    entries: Vec<(SubsystemSlotIdV1, CompatibilityRuleV1)>,
}

impl CompatibilityProfileV1 {
    pub fn new(mut entries: Vec<(SubsystemSlotIdV1, CompatibilityRuleV1)>) -> Result<Self, CompatibilityProfileErrorV1> {
        if entries.len() > MAX_PROFILE_ENTRIES {
            return Err(CompatibilityProfileErrorV1::TooManyEntries { count: entries.len(), max: MAX_PROFILE_ENTRIES });
        }
        entries.sort_by_key(|(slot, _)| slot.as_u16());
        for w in entries.windows(2) {
            if w[0].0 == w[1].0 {
                return Err(CompatibilityProfileErrorV1::DuplicateSlot(w[0].0));
            }
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[(SubsystemSlotIdV1, CompatibilityRuleV1)] { &self.entries }
}

impl ManifestEncodeV1 for CompatibilityProfileV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut items = Vec::with_capacity(self.entries.len());
        for (slot, rule) in &self.entries {
            let pair = ManifestValueV1::Array(vec![ManifestValueV1::Unsigned(slot.as_u16() as u64), rule.to_manifest_value_v1()?]);
            items.push(pair);
        }
        Ok(ManifestValueV1::Array(items))
    }
}

impl ManifestDecodeV1 for CompatibilityProfileV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Array(items) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut entries = Vec::with_capacity(items.len());
        for item in items {
            let ManifestValueV1::Array(pair) = item else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
            let [slot_value, rule_value] = <[ManifestValueV1; 2]>::try_from(pair)
                .map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("profile entry must be a 2-element array"))?;
            let slot_raw = match slot_value {
                ManifestValueV1::Unsigned(v) if v <= u16::MAX as u64 => v as u16,
                _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
            };
            let slot = SubsystemSlotIdV1::try_from_u16(slot_raw)
                .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown subsystem slot id"))?;
            // Rule decode failure (including a known variant with invalid
            // wire content -- spec section 3.7) propagates here and fails
            // the whole profile decode; no partial profile is ever built
            // from a partially-untrusted entry list.
            let rule = CompatibilityRuleV1::from_manifest_value_v1(rule_value)?;
            entries.push((slot, rule));
        }
        Self::new(entries).map_err(|e| {
            ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail(match e {
                CompatibilityProfileErrorV1::DuplicateSlot(_) => "duplicate slot in profile",
                CompatibilityProfileErrorV1::TooManyEntries { .. } => "profile exceeds max entries",
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::hash_artifact_bytes_v1;
    use crate::apex::manifest::{
        CanonicalFieldMapV1, FieldIdV1, ManifestDecodeLimitsV1, decode_manifest_v1, encode_manifest_v1,
    };
    use crate::apex::digest::ContentIdentityV1;

    fn limits() -> ManifestDecodeLimitsV1 {
        ManifestDecodeLimitsV1 {
            max_input_bytes: 8192,
            max_depth: 8,
            max_nodes: 512,
            max_array_items: 128,
            max_map_entries: 128,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 256,
        }
    }

    fn exact_rule(seed: &[u8]) -> CompatibilityRuleV1 {
        CompatibilityRuleV1::Exact { content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(seed), semantic: None } }
    }

    #[test]
    fn rejects_duplicate_slot() {
        let entries = vec![(SubsystemSlotIdV1::Worldgen, exact_rule(b"a")), (SubsystemSlotIdV1::Worldgen, exact_rule(b"b"))];
        assert_eq!(CompatibilityProfileV1::new(entries), Err(CompatibilityProfileErrorV1::DuplicateSlot(SubsystemSlotIdV1::Worldgen)));
    }

    #[test]
    fn canonical_order_is_slot_tag_not_insertion() {
        let entries = vec![(SubsystemSlotIdV1::Build, exact_rule(b"build")), (SubsystemSlotIdV1::Worldgen, exact_rule(b"worldgen"))];
        let profile = CompatibilityProfileV1::new(entries).unwrap();
        let slots: Vec<_> = profile.entries().iter().map(|(s, _)| *s).collect();
        assert_eq!(slots, vec![SubsystemSlotIdV1::Worldgen, SubsystemSlotIdV1::Build]);
    }

    #[test]
    fn multi_slot_profile_round_trips() {
        let entries = vec![
            (SubsystemSlotIdV1::Worldgen, exact_rule(b"worldgen")),
            (SubsystemSlotIdV1::Content, exact_rule(b"content")),
            (SubsystemSlotIdV1::Numeric, exact_rule(b"numeric")),
        ];
        let original = CompatibilityProfileV1::new(entries).unwrap();
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: CompatibilityProfileV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }

    /// Byte-flip canary (spec section 6, "T1.2.05"-style pattern reused
    /// for T0.5): mutating one artifact byte flips the profile's encoding.
    #[test]
    fn byte_flip_changes_encoding() {
        let a = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Worldgen, exact_rule(b"a"))]).unwrap();
        let b = CompatibilityProfileV1::new(vec![(SubsystemSlotIdV1::Worldgen, exact_rule(b"b"))]).unwrap();
        assert_ne!(encode_manifest_v1(&a, &limits()).unwrap(), encode_manifest_v1(&b, &limits()).unwrap());
    }

    /// Path-order canary: the same logical set built in a different
    /// insertion order encodes identically (canonical order is slot-tag,
    /// never insertion order).
    #[test]
    fn insertion_order_does_not_affect_encoding() {
        let forward = CompatibilityProfileV1::new(vec![
            (SubsystemSlotIdV1::Worldgen, exact_rule(b"w")),
            (SubsystemSlotIdV1::Build, exact_rule(b"b")),
        ])
        .unwrap();
        let backward = CompatibilityProfileV1::new(vec![
            (SubsystemSlotIdV1::Build, exact_rule(b"b")),
            (SubsystemSlotIdV1::Worldgen, exact_rule(b"w")),
        ])
        .unwrap();
        assert_eq!(encode_manifest_v1(&forward, &limits()).unwrap(), encode_manifest_v1(&backward, &limits()).unwrap());
    }

    /// Non-vacuity for the profile-poisons-whole-decode claim (spec
    /// section 3.7): a profile array with one otherwise-valid entry and
    /// one entry whose rule has invalid wire content must fail decode
    /// entirely -- never a partial profile.
    #[test]
    fn one_poisoned_rule_fails_the_whole_profile_decode() {
        let good_entry = ManifestValueV1::Array(vec![
            ManifestValueV1::Unsigned(SubsystemSlotIdV1::Worldgen.as_u16() as u64),
            exact_rule(b"good").to_manifest_value_v1().unwrap(),
        ]);
        let poisoned_rule = ManifestValueV1::Map(
            CanonicalFieldMapV1::try_from_entries(vec![
                (FieldIdV1::new(0), ManifestValueV1::Unsigned(2)), // AcceptSet tag
                (FieldIdV1::new(1), ManifestValueV1::Unsigned(0)), // Critical
                (FieldIdV1::new(2), ManifestValueV1::Array(vec![])), // empty -> invalid
            ])
            .unwrap(),
        );
        let poisoned_entry =
            ManifestValueV1::Array(vec![ManifestValueV1::Unsigned(SubsystemSlotIdV1::Content.as_u16() as u64), poisoned_rule]);
        let array = ManifestValueV1::Array(vec![good_entry, poisoned_entry]);

        struct RawWrapper(ManifestValueV1);
        impl ManifestEncodeV1 for RawWrapper {
            fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> { Ok(self.0.clone()) }
        }
        let bytes = encode_manifest_v1(&RawWrapper(array), &limits()).unwrap();
        let err = decode_manifest_v1::<CompatibilityProfileV1>(&bytes, &limits());
        assert!(err.is_err(), "one poisoned rule must fail the whole profile's decode, not a partial profile");
    }
}
