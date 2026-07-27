//! `APEX-T1.107` (engine-list, Part 2 P2-G6): failure-seed preservation
//! and shrinking. A `FailureSeedRecordV1` is the audit trail for taking a
//! failing seed found by the paired-run oracle
//! (`bastion-harness::determinism_regression`'s `Verdict`/`FirstDivergence`)
//! and reducing it to a smaller reproducing case — WITHOUT losing proof
//! that the smaller case still reproduces the SAME bug, not a different
//! one the shrink accidentally introduced.
//!
//! Scope decision, disclosed rather than silently narrowed: `FirstDivergence`
//! lives in `bastion-harness` (it embeds `serde_json::Value`, a
//! scenario-specific comparison type, not a manifest-canonical one) and
//! that crate depends on `common`, never the reverse — so this schema
//! cannot embed `FirstDivergence` directly. `original_divergence_signature`
//! / `minimized_divergence_signature` are opaque
//! [`ArtifactIdentityV1`] digests instead; V1 defines the record's SHAPE
//! and the "same signature or the shrink is rejected" structural rule, but
//! the reduction function from a real `FirstDivergence` down to a stable
//! signature (which fields identify "the same bug" vs. which are allowed
//! to drift) is left to the `bastion-harness` integration, not built here.

use crate::apex::digest::{ArtifactIdentityV1, DigestDomainIdV1, DigestErrorV1, ProtocolDigestV1, digest_manifest_value_v1};
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, MachineTextV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1,
    ManifestDecodeLimitsV1, ManifestDecodeV1, ManifestEncodeV1, ManifestErrorV1, ManifestSchemaErrorV1,
    ManifestValueV1, StructFieldsV1,
};

pub const FAILURE_SEED_RECORD_SCHEMA_V1: &str = "bastion.failure-seed-record/v1";

pub const fn failure_seed_limits_v1() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 1 << 20,
        max_depth: 8,
        max_nodes: 1 << 12,
        max_array_items: 256,
        max_map_entries: 40,
        max_machine_text_bytes: 4096,
        max_byte_string_bytes: 4096,
    }
}

fn err(detail: &'static str) -> ManifestSchemaErrorV1 {
    ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail(detail)
}

fn map_value(entries: Vec<(u16, ManifestValueV1)>) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    let entries = entries.into_iter().map(|(id, v)| (FieldIdV1::new(id), v)).collect();
    Ok(ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries)?))
}

fn take_unsigned(v: ManifestValueV1) -> Result<u64, ManifestSchemaErrorV1> {
    match v { ManifestValueV1::Unsigned(x) => Ok(x), _ => Err(err("expected unsigned")) }
}
fn take_text(v: ManifestValueV1) -> Result<MachineTextV1, ManifestSchemaErrorV1> {
    match v { ManifestValueV1::MachineText(t) => Ok(t), _ => Err(err("expected machine text")) }
}
fn take_map(v: ManifestValueV1) -> Result<StructFieldsV1, ManifestSchemaErrorV1> {
    match v { ManifestValueV1::Map(m) => Ok(StructFieldsV1::new(m)), _ => Err(err("expected map")) }
}

macro_rules! sealed_terminal_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $val:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[repr(u16)]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $name { $($variant = $val),+ }
        impl $name {
            pub const ALL: &'static [$name] = &[$(Self::$variant),+];
            pub const fn as_u16(self) -> u16 { self as u16 }
            pub fn try_from_u16(v: u16) -> Result<Self, ManifestSchemaErrorV1> {
                Self::ALL.iter().copied().find(|t| t.as_u16() == v).ok_or_else(|| err("unknown terminal discriminant"))
            }
        }
    };
}

sealed_terminal_enum! {
    /// `NotShrunk` = the seed is preserved but no shrink attempt has
    /// landed yet. `Shrunk` = a smaller case is accepted (its signature
    /// matched). `ShrinkRejectedSignatureDrift` = a smaller case was
    /// FOUND but discarded because its divergence signature differed —
    /// recorded as evidence of the rejection, never silently dropped.
    FailureSeedTerminalV1 {
        NotShrunk = 0,
        Shrunk = 1,
        ShrinkRejectedSignatureDrift = 2,
    }
}

/// Builder-ready row binding: `FailureSeedRecordV1=(test_id,
/// original_seed, minimized_input_digest, minimized_seed,
/// shrink_trace_digest, first_divergence_signature)` — split here into
/// the original/minimized signature PAIR the row's own acceptance rule
/// needs ("a smaller case is accepted only if the exact failure signature
/// ... remain[s]"), rather than one ambiguous shared field. Structural
/// admission (decode-time, not a separate pass): `Shrunk` requires every
/// minimized_* field present AND `minimized_divergence_signature ==
/// original_divergence_signature`; `NotShrunk` requires every minimized_*
/// field absent; `ShrinkRejectedSignatureDrift` requires the minimized
/// candidate present but its signature to actually DIFFER (the rejection
/// must be provably a rejection, not an unset field masquerading as one).
#[derive(Clone, Debug, PartialEq)]
pub struct FailureSeedRecordV1 {
    pub test_id: MachineTextV1,
    pub original_seed: u64,
    pub original_divergence_signature: ArtifactIdentityV1,
    pub minimized_seed: Option<u64>,
    pub minimized_input_digest: Option<ArtifactIdentityV1>,
    pub minimized_divergence_signature: Option<ArtifactIdentityV1>,
    /// The shrink search's own trace (every candidate tried, win or lose)
    /// — always present, even under `NotShrunk`, so "no shrink attempted"
    /// and "shrink attempted, nothing accepted" stay distinguishable.
    pub shrink_trace_digest: ArtifactIdentityV1,
    pub terminal: FailureSeedTerminalV1,
}

impl FailureSeedRecordV1 {
    pub fn canonical_root(&self) -> Result<ProtocolDigestV1, DigestErrorV1> {
        digest_manifest_value_v1(DigestDomainIdV1::FailureSeedRecord, self, &failure_seed_limits_v1())
    }
}

impl ManifestEncodeV1 for FailureSeedRecordV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut entries = vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(FAILURE_SEED_RECORD_SCHEMA_V1)?)),
            (1, ManifestValueV1::MachineText(self.test_id.clone())),
            (2, ManifestValueV1::Unsigned(self.original_seed)),
            (3, self.original_divergence_signature.to_manifest_value_v1()?),
            (7, self.shrink_trace_digest.to_manifest_value_v1()?),
            (8, ManifestValueV1::Unsigned(self.terminal.as_u16() as u64)),
        ];
        if let Some(seed) = self.minimized_seed {
            entries.push((4, ManifestValueV1::Unsigned(seed)));
        }
        if let Some(digest) = &self.minimized_input_digest {
            entries.push((5, digest.to_manifest_value_v1()?));
        }
        if let Some(sig) = &self.minimized_divergence_signature {
            entries.push((6, sig.to_manifest_value_v1()?));
        }
        map_value(entries)
    }
}
impl ManifestDecodeV1 for FailureSeedRecordV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        if take_text(f.take_required(FieldIdV1::new(0))?)?.as_str() != FAILURE_SEED_RECORD_SCHEMA_V1 {
            return Err(err("wrong failure-seed-record schema tag"));
        }
        let test_id = take_text(f.take_required(FieldIdV1::new(1))?)?;
        let original_seed = take_unsigned(f.take_required(FieldIdV1::new(2))?)?;
        let original_divergence_signature =
            ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(3))?)?;
        let minimized_seed = f.take_optional(FieldIdV1::new(4))?.map(take_unsigned).transpose()?;
        let minimized_input_digest = f
            .take_optional(FieldIdV1::new(5))?
            .map(ArtifactIdentityV1::from_manifest_value_v1)
            .transpose()?;
        let minimized_divergence_signature = f
            .take_optional(FieldIdV1::new(6))?
            .map(ArtifactIdentityV1::from_manifest_value_v1)
            .transpose()?;
        let shrink_trace_digest = ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(7))?)?;
        let terminal = FailureSeedTerminalV1::try_from_u16(
            u16::try_from(take_unsigned(f.take_required(FieldIdV1::new(8))?)?)
                .map_err(|_| err("terminal discriminant out of range"))?,
        )?;
        f.finish_no_unknown()?;

        let minimized_all_present = minimized_seed.is_some()
            && minimized_input_digest.is_some()
            && minimized_divergence_signature.is_some();
        let minimized_all_absent =
            minimized_seed.is_none() && minimized_input_digest.is_none() && minimized_divergence_signature.is_none();
        match terminal {
            FailureSeedTerminalV1::NotShrunk => {
                if !minimized_all_absent {
                    return Err(err("NotShrunk must carry no minimized_* fields"));
                }
            },
            FailureSeedTerminalV1::Shrunk => {
                if !minimized_all_present {
                    return Err(err("Shrunk requires every minimized_* field"));
                }
                if minimized_divergence_signature.as_ref() != Some(&original_divergence_signature) {
                    return Err(err("Shrunk requires the minimized signature to match the original exactly"));
                }
            },
            FailureSeedTerminalV1::ShrinkRejectedSignatureDrift => {
                if !minimized_all_present {
                    return Err(err("ShrinkRejectedSignatureDrift requires every minimized_* field"));
                }
                if minimized_divergence_signature.as_ref() == Some(&original_divergence_signature) {
                    return Err(err(
                        "ShrinkRejectedSignatureDrift requires the minimized signature to actually differ",
                    ));
                }
            },
        }
        Ok(Self {
            test_id,
            original_seed,
            original_divergence_signature,
            minimized_seed,
            minimized_input_digest,
            minimized_divergence_signature,
            shrink_trace_digest,
            terminal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::hash_artifact_bytes_v1;
    use crate::apex::manifest::{decode_manifest_v1, encode_manifest_v1};

    fn text(s: &str) -> MachineTextV1 { MachineTextV1::new(s).unwrap() }

    fn not_shrunk() -> FailureSeedRecordV1 {
        FailureSeedRecordV1 {
            test_id: text("b55-deep"),
            original_seed: 12345,
            original_divergence_signature: hash_artifact_bytes_v1(b"sig-original"),
            minimized_seed: None,
            minimized_input_digest: None,
            minimized_divergence_signature: None,
            shrink_trace_digest: hash_artifact_bytes_v1(b"trace-empty"),
            terminal: FailureSeedTerminalV1::NotShrunk,
        }
    }

    fn shrunk() -> FailureSeedRecordV1 {
        let sig = hash_artifact_bytes_v1(b"sig-original");
        FailureSeedRecordV1 {
            test_id: text("b55-deep"),
            original_seed: 12345,
            original_divergence_signature: sig.clone(),
            minimized_seed: Some(77),
            minimized_input_digest: Some(hash_artifact_bytes_v1(b"minimized-input")),
            minimized_divergence_signature: Some(sig),
            shrink_trace_digest: hash_artifact_bytes_v1(b"trace-nonempty"),
            terminal: FailureSeedTerminalV1::Shrunk,
        }
    }

    #[test]
    fn record_round_trips_canonically() {
        let limits = failure_seed_limits_v1();
        for r in [not_shrunk(), shrunk()] {
            let bytes = encode_manifest_v1(&r, &limits).unwrap();
            let decoded: FailureSeedRecordV1 = decode_manifest_v1(&bytes, &limits).unwrap();
            assert_eq!(decoded, r);
            assert_eq!(encode_manifest_v1(&decoded, &limits).unwrap(), bytes);
            assert!(r.canonical_root().is_ok());
        }
    }

    /// The row's own acceptance rule: a shrink is accepted ONLY if the
    /// exact failure signature survives. A signature drift must be
    /// recorded as a REJECTION, never silently accepted as `Shrunk`.
    #[test]
    fn signature_drift_cannot_be_accepted() {
        let limits = failure_seed_limits_v1();
        let mut drifted = shrunk();
        drifted.minimized_divergence_signature = Some(hash_artifact_bytes_v1(b"sig-DIFFERENT"));
        let bytes = encode_manifest_v1(&drifted, &limits).unwrap();
        assert!(decode_manifest_v1::<FailureSeedRecordV1>(&bytes, &limits).is_err());

        // The same drifted candidate, honestly labeled as a rejection, decodes fine.
        drifted.terminal = FailureSeedTerminalV1::ShrinkRejectedSignatureDrift;
        let bytes = encode_manifest_v1(&drifted, &limits).unwrap();
        assert!(decode_manifest_v1::<FailureSeedRecordV1>(&bytes, &limits).is_ok());
    }

    #[test]
    fn partial_minimized_fields_are_rejected() {
        let limits = failure_seed_limits_v1();
        let mut partial = shrunk();
        partial.minimized_seed = None; // digest+signature present, seed missing
        let bytes = encode_manifest_v1(&partial, &limits).unwrap();
        assert!(decode_manifest_v1::<FailureSeedRecordV1>(&bytes, &limits).is_err());

        let mut leaked = not_shrunk();
        leaked.minimized_seed = Some(1); // NotShrunk must carry none
        let bytes = encode_manifest_v1(&leaked, &limits).unwrap();
        assert!(decode_manifest_v1::<FailureSeedRecordV1>(&bytes, &limits).is_err());
    }

    #[test]
    fn sealed_terminal_fails_closed() {
        assert!(FailureSeedTerminalV1::try_from_u16(3).is_err());
        assert_eq!(FailureSeedTerminalV1::ALL.len(), 3);
    }
}
