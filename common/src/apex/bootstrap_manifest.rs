//! `APEX-T4.1` — `BootstrapManifestV1`: the wire-encodable half of a total,
//! classified compatibility check a client can run BEFORE applying
//! `ServerInit::GameSync`'s bulk state.
//!
//! **The gap this closes.** `GameSync` is applied wholesale; the client
//! constructs `State::client` from it with no prior agreement on wire,
//! content, plugin, schedule or numeric protocol. `ServerInfo.rules`
//! (`common/net/src/msg/server.rs:74`) is human-facing text, not a
//! machine-checkable identity.
//!
//! **This module reuses `T0.5`'s subsystem-compatibility machinery
//! WHOLESALE and adds no parallel classification or report mechanism.**
//! [`crate::apex::subsystem::report::evaluate_compatibility_v1`] already
//! IS the total, never-short-circuits evaluator this row's acceptance
//! criterion asks for; [`crate::apex::subsystem::rule::CompatibilityRuleV1`]
//! already carries `Exact` (equality-critical), `NegotiatedCapability`
//! (negotiated, with the selection algorithm's own agreement enforced
//! structurally via `NegotiationSelectorV1` equality inside the
//! evaluator), and `ProvenanceOnly` (recorded, never compared) — the
//! exact three-way split this row's spec describes, already built,
//! already tested. Writing a second, parallel classification enum here
//! would be exactly the "parallel vocabulary" the row's own spec forbids.
//!
//! **What this module actually adds, and why it is needed at all:**
//! [`crate::apex::subsystem::report::SubsystemEvaluationInputV1`] is, by
//! its own doc comment, "not part of the frozen wire contract... an
//! in-process evaluation input, not a manifest-encoded type." Something
//! has to carry the server's declared descriptors, negotiation selector,
//! and capability set over the actual wire. [`BootstrapManifestV1`] is
//! that carrier: a manifest-encodable subset of
//! `SubsystemEvaluationInputV1` (everything the SENDER can supply --
//! `transform_registry` and `local_selector` are receiver-local and never
//! travel), plus [`BootstrapManifestV1::to_evaluation_input_v1`] to join
//! the two back together on receipt.
//!
//! **Wire message integration (`T4.1` migration steps 2-4 — emit
//! immediately before `GameSync`, validate before `State::client`
//! construction) is a separate, follow-up chunk.** This module is the
//! row's mechanism, fully testable against fixtures alone; landing it
//! first and self-sizing the wire-ordering work separately is the
//! standing discipline for this program.

use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};
use crate::apex::subsystem::negotiate::{CapabilityIdV1, NegotiationSelectorV1};
use crate::apex::subsystem::report::SubsystemEvaluationInputV1;
use crate::apex::subsystem::descriptor::SubsystemDescriptorV1;
use crate::apex::subsystem::transform::TransformRegistryV1;

/// What a server can supply about its own bootstrap identity, encoded for
/// the wire. See the module doc for why this exists as a separate type
/// from [`SubsystemEvaluationInputV1`] rather than making that type
/// itself manifest-encodable.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BootstrapManifestV1 {
    /// One descriptor per slot the sender declares. Sparse by
    /// construction, same as `SubsystemEvaluationInputV1::descriptors` --
    /// a slot the evaluator's profile checks but this list omits is
    /// exactly `InvalidInputReasonV1::NoDescriptorForSlot`, not a
    /// separate error path this module needs to invent.
    pub descriptors: Vec<SubsystemDescriptorV1>,
    /// The sender's own selector for `NegotiatedCapability` rules, if it
    /// has one. Absent from a build that has none to offer.
    pub peer_selector: Option<NegotiationSelectorV1>,
    pub peer_capabilities: Vec<CapabilityIdV1>,
    /// `APEX-T4.2` (banked design note, not yet built): the freshness/
    /// sequence-binding tuple -- `ServerBootId`/`SessionId`/
    /// `ConnectionEpoch` (already available via `T0.4`/`T3.2`), a
    /// bootstrap sequence monotone within the boot, the snapshot epoch,
    /// and the predecessor root. `T4.2`'s own spec sizes this to the
    /// SAME pre-`GameSync` message this row already sends, so THIS field
    /// is that reservation: the wire field ID is claimed now, so `T4.2`
    /// populates it rather than needing a second pre-`GameSync` message
    /// inserted later (a second surgery on the same connection ordering
    /// invariant `T4.1`'s chunk 2 establishes). The TYPE is deliberately
    /// not guessed -- `T4.2` has not been ruled -- so this is opaque
    /// bytes, not a typed struct, until that row builds and populates
    /// it. Always `None` today.
    pub freshness_reserved: Option<Vec<u8>>,
}

impl BootstrapManifestV1 {
    /// Joins the wire-carried half with what only the receiver knows
    /// locally (its own selector preference, its own transform registry),
    /// producing exactly what
    /// [`crate::apex::subsystem::report::evaluate_compatibility_v1`]
    /// consumes.
    pub fn to_evaluation_input_v1(
        &self,
        local_selector: Option<NegotiationSelectorV1>,
        transform_registry: TransformRegistryV1,
    ) -> SubsystemEvaluationInputV1 {
        SubsystemEvaluationInputV1 {
            descriptors: self.descriptors.clone(),
            local_selector,
            peer_selector: self.peer_selector,
            peer_capabilities: self.peer_capabilities.clone(),
            transform_registry,
        }
    }
}

fn encode_capability(id: CapabilityIdV1) -> ManifestValueV1 { ManifestValueV1::Unsigned(id.get() as u64) }

fn decode_capability(value: ManifestValueV1) -> Result<CapabilityIdV1, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Unsigned(v) if v <= u32::MAX as u64 => Ok(CapabilityIdV1::new(v as u32)),
        _ => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)),
    }
}

/// `Option<T>` has no direct `ManifestValueV1` representation (the
/// restricted data model has no null/undefined kind, by design -- see
/// `common/src/apex/manifest/value.rs`'s module doc). Encoded as a
/// 0-or-1-element array: present is unambiguous from absent without
/// adding a discriminant field, and the array-length check IS the
/// discriminant.
fn encode_optional_selector(selector: &Option<NegotiationSelectorV1>) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    match selector {
        Some(s) => Ok(ManifestValueV1::Array(vec![s.to_manifest_value_v1()?])),
        None => Ok(ManifestValueV1::Array(Vec::new())),
    }
}

fn decode_optional_selector(value: ManifestValueV1) -> Result<Option<NegotiationSelectorV1>, ManifestSchemaErrorV1> {
    let ManifestValueV1::Array(items) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
    match <[ManifestValueV1; 1]>::try_from(items) {
        Ok([only]) => Ok(Some(NegotiationSelectorV1::from_manifest_value_v1(only)?)),
        Err(items) if items.is_empty() => Ok(None),
        Err(_) => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("optional selector array must have 0 or 1 elements")),
    }
}

/// Same 0-or-1-element-array encoding as [`encode_optional_selector`], for
/// the `T4.2` reservation's opaque byte payload.
fn encode_optional_bytes(bytes: &Option<Vec<u8>>) -> ManifestValueV1 {
    match bytes {
        Some(b) => ManifestValueV1::Array(vec![ManifestValueV1::Bytes(b.clone())]),
        None => ManifestValueV1::Array(Vec::new()),
    }
}

fn decode_optional_bytes(value: ManifestValueV1) -> Result<Option<Vec<u8>>, ManifestSchemaErrorV1> {
    let ManifestValueV1::Array(items) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
    match <[ManifestValueV1; 1]>::try_from(items) {
        Ok([ManifestValueV1::Bytes(b)]) => Ok(Some(b)),
        Ok([_]) => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("freshness reservation must be a Bytes leaf")),
        Err(items) if items.is_empty() => Ok(None),
        Err(_) => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("optional bytes array must have 0 or 1 elements")),
    }
}

impl ManifestEncodeV1 for BootstrapManifestV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut descriptors = Vec::with_capacity(self.descriptors.len());
        for d in &self.descriptors {
            descriptors.push(d.to_manifest_value_v1()?);
        }
        let capabilities = self.peer_capabilities.iter().copied().map(encode_capability).collect();
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Array(descriptors)),
            (FieldIdV1::new(2), encode_optional_selector(&self.peer_selector)?),
            (FieldIdV1::new(3), ManifestValueV1::Array(capabilities)),
            (FieldIdV1::new(4), encode_optional_bytes(&self.freshness_reserved)),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for BootstrapManifestV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);

        let ManifestValueV1::Array(descriptor_values) = fields.take_required(FieldIdV1::new(1))? else {
            return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType));
        };
        let mut descriptors = Vec::with_capacity(descriptor_values.len());
        for v in descriptor_values {
            descriptors.push(SubsystemDescriptorV1::from_manifest_value_v1(v)?);
        }

        let peer_selector = decode_optional_selector(fields.take_required(FieldIdV1::new(2))?)?;

        let ManifestValueV1::Array(capability_values) = fields.take_required(FieldIdV1::new(3))? else {
            return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType));
        };
        let mut peer_capabilities = Vec::with_capacity(capability_values.len());
        for v in capability_values {
            peer_capabilities.push(decode_capability(v)?);
        }

        let freshness_reserved = decode_optional_bytes(fields.take_required(FieldIdV1::new(4))?)?;

        fields.finish_no_unknown()?;
        Ok(Self { descriptors, peer_selector, peer_capabilities, freshness_reserved })
    }
}

#[cfg(test)]
mod bootstrap_manifest_v1 {
    use super::*;
    use crate::apex::digest::{ContentIdentityV1, hash_artifact_bytes_v1};
    use crate::apex::manifest::{ManifestDecodeLimitsV1, decode_manifest_v1, encode_manifest_v1};
    use crate::apex::scalar::SchemaVersion;
    use crate::apex::subsystem::negotiate::{SelectorAlgorithmV1, SelectorOwnerV1};
    use crate::apex::subsystem::slot::SubsystemSlotIdV1;

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

    fn descriptor(slot: SubsystemSlotIdV1, seed: &[u8]) -> SubsystemDescriptorV1 {
        SubsystemDescriptorV1 {
            slot,
            schema: SchemaVersion::new(1),
            content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(seed), semantic: None },
        }
    }

    fn selector() -> NegotiationSelectorV1 {
        NegotiationSelectorV1 {
            owner: SelectorOwnerV1::ServerAuthoritative,
            algorithm: SelectorAlgorithmV1::HighestMutualVersion,
            version: SchemaVersion::new(1),
        }
    }

    /// A manifest with every field populated round-trips.
    #[test]
    fn full_manifest_round_trips() {
        let original = BootstrapManifestV1 {
            descriptors: vec![descriptor(SubsystemSlotIdV1::NetEnvelope, b"wire"), descriptor(SubsystemSlotIdV1::Build, b"linux")],
            peer_selector: Some(selector()),
            peer_capabilities: vec![CapabilityIdV1::new(1), CapabilityIdV1::new(7)],
            freshness_reserved: Some(vec![1, 2, 3]),
        };
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: BootstrapManifestV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }

    /// An entirely empty manifest (no descriptors, no selector, no
    /// capabilities) round-trips -- the absent-selector case is not just
    /// theoretically representable, it is exercised.
    #[test]
    fn empty_manifest_round_trips() {
        let original = BootstrapManifestV1::default();
        assert_eq!(original.peer_selector, None);
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: BootstrapManifestV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(original, decoded);
    }

    /// The optional-selector array must reject an array of the wrong
    /// arity rather than silently taking the first element -- 2+ items
    /// is malformed input, not "present, extra data ignored".
    #[test]
    fn optional_selector_rejects_wrong_arity() {
        let two_items = ManifestValueV1::Array(vec![
            selector().to_manifest_value_v1().unwrap(),
            selector().to_manifest_value_v1().unwrap(),
        ]);
        let err = decode_optional_selector(two_items).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::FieldKeyType);
    }

    /// `T4.2`'s reservation round-trips in both states: present (once a
    /// future row populates it) and absent (today's actual state).
    #[test]
    fn freshness_reservation_round_trips_present_and_absent() {
        let with_reservation = BootstrapManifestV1 { freshness_reserved: Some(vec![9, 9, 9]), ..Default::default() };
        let bytes = encode_manifest_v1(&with_reservation, &limits()).unwrap();
        let decoded: BootstrapManifestV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(decoded.freshness_reserved, Some(vec![9, 9, 9]));

        let without = BootstrapManifestV1::default();
        assert_eq!(without.freshness_reserved, None);
        let bytes = encode_manifest_v1(&without, &limits()).unwrap();
        let decoded: BootstrapManifestV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(decoded.freshness_reserved, None);
    }

    /// Byte-flip canary (same T0.5 pattern): mutating one descriptor's
    /// content changes the manifest's wire encoding.
    #[test]
    fn byte_flip_changes_encoding() {
        let a = BootstrapManifestV1 { descriptors: vec![descriptor(SubsystemSlotIdV1::Content, b"a")], ..Default::default() };
        let b = BootstrapManifestV1 { descriptors: vec![descriptor(SubsystemSlotIdV1::Content, b"b")], ..Default::default() };
        assert_ne!(encode_manifest_v1(&a, &limits()).unwrap(), encode_manifest_v1(&b, &limits()).unwrap());
    }

    /// `to_evaluation_input_v1` carries every wire field across
    /// unchanged, and correctly attaches the receiver-local fields
    /// (`local_selector`, `transform_registry`) that never travelled.
    #[test]
    fn to_evaluation_input_joins_wire_and_local_fields() {
        let manifest = BootstrapManifestV1 {
            descriptors: vec![descriptor(SubsystemSlotIdV1::Numeric, b"numeric")],
            peer_selector: Some(selector()),
            peer_capabilities: vec![CapabilityIdV1::new(3)],
            freshness_reserved: None,
        };
        let local = NegotiationSelectorV1 {
            owner: SelectorOwnerV1::ClientPreferred,
            algorithm: SelectorAlgorithmV1::ExactMatchOnly,
            version: SchemaVersion::new(2),
        };
        let input = manifest.to_evaluation_input_v1(Some(local), TransformRegistryV1::new());

        assert_eq!(input.descriptors, manifest.descriptors);
        assert_eq!(input.peer_selector, manifest.peer_selector);
        assert_eq!(input.peer_capabilities, manifest.peer_capabilities);
        assert_eq!(input.local_selector, Some(local));
    }

    /// This module's own acceptance criterion: a manifest built through
    /// [`BootstrapManifestV1`], round-tripped through the wire encoding,
    /// and joined via `to_evaluation_input_v1`, is evaluated CORRECTLY by
    /// T0.5's unmodified `evaluate_compatibility_v1` -- proving the two
    /// layers actually compose rather than merely type-checking together.
    #[test]
    fn wire_round_tripped_manifest_evaluates_correctly_through_t0_5() {
        use crate::apex::subsystem::profile::CompatibilityProfileV1;
        use crate::apex::subsystem::report::{CompatibilityOutcomeV1, evaluate_compatibility_v1};
        use crate::apex::subsystem::rule::CompatibilityRuleV1;

        let matching = descriptor(SubsystemSlotIdV1::NetEnvelope, b"wire-schema");
        let mismatched_slot_client_expects = ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"expected-content"), semantic: None };

        let manifest = BootstrapManifestV1 {
            descriptors: vec![matching.clone(), descriptor(SubsystemSlotIdV1::Content, b"server-content")],
            peer_selector: None,
            peer_capabilities: Vec::new(),
            freshness_reserved: None,
        };
        let bytes = encode_manifest_v1(&manifest, &limits()).unwrap();
        let decoded: BootstrapManifestV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        let input = decoded.to_evaluation_input_v1(None, TransformRegistryV1::new());

        let profile = CompatibilityProfileV1::new(vec![
            (SubsystemSlotIdV1::NetEnvelope, CompatibilityRuleV1::Exact { content: matching.content }),
            (SubsystemSlotIdV1::Content, CompatibilityRuleV1::Exact { content: mismatched_slot_client_expects }),
        ])
        .unwrap();
        let report = evaluate_compatibility_v1(&profile, &input);
        let outcome_for = |slot: SubsystemSlotIdV1| {
            report.results().iter().find(|r| r.slot == slot).map(|r| r.outcome.clone()).expect("slot in report")
        };

        assert!(!report.is_fully_compatible());
        assert_eq!(outcome_for(SubsystemSlotIdV1::NetEnvelope), CompatibilityOutcomeV1::Compatible);
        assert!(matches!(outcome_for(SubsystemSlotIdV1::Content), CompatibilityOutcomeV1::Incompatible(_)));
    }
}
