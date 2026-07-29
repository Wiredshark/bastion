//! `APEX-T4.1` chunk 2a — `BootstrapManifestV1`'s WIRE form.
//!
//! `common::apex::bootstrap_manifest::BootstrapManifestV1` is a
//! `T0.5`-native type: it round-trips through the MANIFEST-VALUE codec
//! ([`common::apex::manifest::ManifestEncodeV1`]/`ManifestDecodeV1`), not
//! serde — the same choice every other `T0.5` subsystem type makes,
//! because `SubsystemDescriptorV1` carries `ContentIdentityV1`, a `T0.3`
//! SEALED IDENTITY type that deliberately has no serde impl. Same
//! principle `T5.3`'s `InputReceiptWireV1` documents: adding serde to a
//! sealed identity to satisfy a transport is how an identity becomes
//! something the wire ASSERTS rather than something the receiver
//! RECOMPUTES.
//!
//! So the wire carries the manifest-encoded BYTES, and the receiver
//! reconstructs the typed manifest via `decode_manifest_v1` — the
//! manifest codec's own checked decode already IS the recompute step;
//! this wrapper's only job is getting the bytes from sender to receiver
//! through the ordinary bincode `ServerGeneral` pipeline.

use common::apex::bootstrap_manifest::BootstrapManifestV1;
use common::apex::manifest::{
    ManifestCodecErrorV1, ManifestDecodeErrorV1, ManifestDecodeLimitsV1, decode_manifest_v1, encode_manifest_v1,
};
use serde::{Deserialize, Serialize};

/// Decode limits for a bootstrap manifest. `T0.5`'s own
/// `MAX_PROFILE_ENTRIES` is 256 (one profile has at most one rule per
/// known slot); these are sized generously above that so a legitimate
/// manifest never trips them, while still bounding what a corrupt or
/// hostile sender can make the receiver allocate.
pub fn bootstrap_manifest_limits_v1() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 65536,
        max_depth: 8,
        max_nodes: 4096,
        max_array_items: 512,
        max_map_entries: 512,
        max_machine_text_bytes: 256,
        max_byte_string_bytes: 4096,
    }
}

/// [`BootstrapManifestV1`], as bytes. Private field: the only way to
/// build one is [`BootstrapManifestWireV1::from_typed_v1`], so a caller
/// cannot hand-assemble bytes that never passed through the real
/// encoder.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BootstrapManifestWireV1 {
    manifest_bytes: Vec<u8>,
}

impl BootstrapManifestWireV1 {
    pub fn from_typed_v1(manifest: &BootstrapManifestV1) -> Result<Self, ManifestCodecErrorV1> {
        Ok(Self { manifest_bytes: encode_manifest_v1(manifest, &bootstrap_manifest_limits_v1())? })
    }

    /// Rebuild the typed manifest from the bytes -- full schema
    /// validation (`T0.5`'s slot/rule/profile checks) runs here, not at
    /// construction, so a malformed manifest is a decode failure at this
    /// boundary rather than a half-built value downstream.
    pub fn to_typed_v1(&self) -> Result<BootstrapManifestV1, ManifestDecodeErrorV1> {
        decode_manifest_v1(&self.manifest_bytes, &bootstrap_manifest_limits_v1())
    }
}

#[cfg(test)]
mod bootstrap_manifest_wire_v1 {
    use super::*;
    use common::apex::digest::{ContentIdentityV1, hash_artifact_bytes_v1};
    use common::apex::scalar::SchemaVersion;
    use common::apex::subsystem::{SubsystemDescriptorV1, SubsystemSlotIdV1};

    fn typed() -> BootstrapManifestV1 {
        BootstrapManifestV1 {
            descriptors: vec![SubsystemDescriptorV1 {
                slot: SubsystemSlotIdV1::NetEnvelope,
                schema: SchemaVersion::new(1),
                content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"wire-schema"), semantic: None },
            }],
            peer_selector: None,
            peer_capabilities: Vec::new(),
            freshness: None,
        }
    }

    /// The round trip is lossless through the REAL bincode encoder the
    /// messages use, not just through the conversion functions — a
    /// conversion that round-trips in memory but not through bincode
    /// would still be a broken transport.
    #[test]
    fn a_manifest_round_trips_through_the_wire_encoder() {
        let before = typed();
        let wire = BootstrapManifestWireV1::from_typed_v1(&before).unwrap();
        let bytes = bincode::serde::encode_to_vec(&wire, bincode::config::legacy()).expect("wire encodes");
        let (decoded, _): (BootstrapManifestWireV1, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::legacy()).expect("wire decodes");
        assert_eq!(decoded, wire);
        assert_eq!(decoded.to_typed_v1().unwrap(), before);
    }

    /// An empty manifest (the row's own dormant-today state -- no live
    /// client requests anything but `Legacy` yet) round-trips too.
    #[test]
    fn an_empty_manifest_round_trips() {
        let before = BootstrapManifestV1::default();
        let wire = BootstrapManifestWireV1::from_typed_v1(&before).unwrap();
        assert_eq!(wire.to_typed_v1().unwrap(), before);
    }

    /// Corrupted wire bytes fail decode rather than producing a
    /// half-built manifest.
    #[test]
    fn corrupted_bytes_fail_decode_not_silently() {
        let mut wire = BootstrapManifestWireV1::from_typed_v1(&typed()).unwrap();
        for byte in wire.manifest_bytes.iter_mut() {
            *byte ^= 0xff;
        }
        assert!(wire.to_typed_v1().is_err(), "corrupted manifest bytes must fail decode, not decode into garbage");
    }
}
