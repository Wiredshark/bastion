//! `APEX-T1.4.01/.02` — fresh-environment rebuild-pair contracts (REAL
//! packet `PROJECT-BASTION-APEX-MICROSTEP-APEX-T1.4-FRESH-ENVIRONMENT-
//! REBUILD-PAIR.md`, section 8; canary pin `7970d960…` verified).
//!
//! Claim boundary, packet section 1: two fresh VMs under ONE cloud
//! account are `PairPassSameTrustDomain`, never "clean-room"/"any
//! party". NAR equality is the authoritative oracle; logs, hostnames,
//! instance IDs, and times are run details (timestamps are blanked in
//! every canonical root, the T1.3 pattern).
//!
//! Documented realizations of packet gaps (referenced but never defined
//! in section 8 — fleet-authored minimal forms, cross-review targets):
//! `DependencySubstitutionPolicyV1` (policy 6: pinned digest-verified
//! dependencies only), `NetworkPhasePolicyV1` (policy 14: substituters
//! allowed before the final derivation, offline during it), and
//! `OutputMismatchV1` (first-mismatch evidence: which comparison field
//! broke, at which member/offset). The packet's embedded `record_root`
//! fields are realized as `canonical_root()` methods + emission sidecars
//! (established T1.3 divergence — a root cannot be a field of the very
//! encoding it digests).

use crate::apex::digest::{
    ArtifactIdentityV1, DigestDomainIdV1, DigestErrorV1, ProtocolDigestV1, digest_manifest_value_v1,
};
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, MachineTextV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1,
    ManifestDecodeLimitsV1, ManifestDecodeV1, ManifestEncodeV1, ManifestErrorV1, ManifestSchemaErrorV1,
    ManifestValueV1, StructFieldsV1,
};
use crate::apex::source_closure::GitHexIdV1;

pub const FRESH_BUILDER_PROFILE_SCHEMA_V1: &str = "bastion.fresh-builder-profile/v1";
pub const FRESH_BUILDER_RUN_SCHEMA_V1: &str = "bastion.fresh-builder-run/v1";
pub const FRESH_REBUILD_PAIR_SCHEMA_V1: &str = "bastion.fresh-rebuild-pair/v1";

pub const fn fresh_rebuild_limits_v1() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 1 << 20,
        max_depth: 10,
        max_nodes: 1 << 14,
        max_array_items: 256,
        max_map_entries: 40,
        max_machine_text_bytes: 4096,
        max_byte_string_bytes: 4096,
    }
}

macro_rules! sealed_u16_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $val:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[repr(u16)]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $name { $($variant = $val),+ }
        impl $name {
            pub const ALL: &'static [$name] = &[$(Self::$variant),+];
            pub const fn as_u16(self) -> u16 { self as u16 }
            pub fn try_from_u16(v: u16) -> Result<Self, ManifestSchemaErrorV1> {
                Self::ALL.iter().copied().find(|t| t.as_u16() == v).ok_or_else(|| {
                    ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)
                        .detail(concat!("unknown ", stringify!($name), " discriminant"))
                })
            }
        }
    };
}

sealed_u16_enum! {
    /// Packet policy 6 — V1 admits exactly one posture.
    DependencySubstitutionPolicyV1 { PinnedDigestVerifiedOnly = 0 }
}
sealed_u16_enum! {
    /// Packet policy 14 — V1 admits exactly one posture.
    NetworkPhasePolicyV1 { SubstituteThenOfflineFinal = 0 }
}
sealed_u16_enum! {
    /// Packet section 8.5, order-frozen. Unknown discriminants fail closed.
    FreshBuilderTerminalV1 {
        BuildPass = 0, PrerequisiteMissing = 1, IsolationCheckFailed = 2,
        SourceClosureMismatch = 3, BuilderProfileMismatch = 4, DerivationMismatch = 5,
        FinalOutputNotCold = 6, FinalOutputSubstituted = 7, ProjectCacheEnabled = 8,
        BuildFailed = 9, EvidenceWriteFailed = 10, EvidenceUploadFailed = 11,
        InfrastructureTimeout = 12,
    }
}
sealed_u16_enum! {
    /// Packet section 8.5, order-frozen. `PairPass` is reserved for a
    /// FUTURE independent-trust-domain profile — the V1 controller may
    /// only ever emit `PairPassSameTrustDomain` (packet policy 12).
    FreshRebuildPairTerminalV1 {
        PairPass = 0, PairPassSameTrustDomain = 1, PrerequisiteMissing = 2,
        SharedBuilderInstance = 3, SharedWritableStore = 4, BuilderProfileMismatch = 5,
        SourceClosureMismatch = 6, BuildDefinitionMismatch = 7, DerivationMismatch = 8,
        BuildAFailed = 9, BuildBFailed = 10, EvidenceIncomplete = 11, OutputMismatch = 12,
        ComparatorFailed = 13, InfrastructureIncomplete = 14, RetryWashedMismatch = 15,
        PosthocNormalizationAttempted = 16, CanaryOracleIncomplete = 17,
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
fn take_bool(v: ManifestValueV1) -> Result<bool, ManifestSchemaErrorV1> {
    match v { ManifestValueV1::Bool(b) => Ok(b), _ => Err(err("expected bool")) }
}
fn take_text(v: ManifestValueV1) -> Result<MachineTextV1, ManifestSchemaErrorV1> {
    match v { ManifestValueV1::MachineText(t) => Ok(t), _ => Err(err("expected machine text")) }
}
fn take_map(v: ManifestValueV1) -> Result<StructFieldsV1, ManifestSchemaErrorV1> {
    match v { ManifestValueV1::Map(m) => Ok(StructFieldsV1::new(m)), _ => Err(err("expected map")) }
}
fn take_array(v: ManifestValueV1) -> Result<Vec<ManifestValueV1>, ManifestSchemaErrorV1> {
    match v { ManifestValueV1::Array(a) => Ok(a), _ => Err(err("expected array")) }
}
fn take_u16_enum<T>(v: ManifestValueV1, f: fn(u16) -> Result<T, ManifestSchemaErrorV1>) -> Result<T, ManifestSchemaErrorV1> {
    f(u16::try_from(take_unsigned(v)?).map_err(|_| err("discriminant out of range"))?)
}

/// Packet section 8.1. `allowed_substituters` are kept in the packet's
/// canonical order (sorted by digest bytes then recorded URI) — enforced
/// by the constructor-side sort in [`FreshBuilderProfileV1::normalize`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubstituterV1 {
    pub identity: ArtifactIdentityV1,
    pub uri: MachineTextV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshBuilderProfileV1 {
    pub profile_id: MachineTextV1,
    pub target_system: MachineTextV1,
    pub target_triple: MachineTextV1,
    pub builder_image: ArtifactIdentityV1,
    pub nix_cli: ArtifactIdentityV1,
    pub nix_config_root: ProtocolDigestV1,
    pub allowed_substituters: Vec<SubstituterV1>,
    pub final_derivation_must_build_locally: bool,
    pub dependency_substitution_policy: DependencySubstitutionPolicyV1,
    pub network_phase_policy: NetworkPhasePolicyV1,
    pub max_builds_per_instance: u32,
}

impl FreshBuilderProfileV1 {
    pub fn normalize(&mut self) {
        self.allowed_substituters.sort_by(|a, b| {
            a.identity
                .digest
                .bytes
                .as_array()
                .cmp(b.identity.digest.bytes.as_array())
                .then_with(|| a.uri.as_str().as_bytes().cmp(b.uri.as_str().as_bytes()))
        });
    }

    pub fn canonical_root(&self) -> Result<ProtocolDigestV1, DigestErrorV1> {
        digest_manifest_value_v1(DigestDomainIdV1::FreshBuilderProfile, self, &fresh_rebuild_limits_v1())
    }
}

/// Packet section 8.2 — evidence, not secrets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuilderIsolationEvidenceV1 {
    pub builder_instance_id: MachineTextV1,
    pub provider_instance_identity: ArtifactIdentityV1,
    pub boot_identity: ArtifactIdentityV1,
    pub rootfs_identity: ArtifactIdentityV1,
    pub writable_store_identity: ArtifactIdentityV1,
    pub workspace_identity: ArtifactIdentityV1,
    pub shared_writable_mounts: Vec<MachineTextV1>,
    pub project_cache_detected: bool,
    pub final_output_preexisting: bool,
}

/// Packet section 8.3. Timestamps are diagnostic — blanked in the
/// canonical root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshBuilderRunV1 {
    pub pair_id: MachineTextV1,
    pub builder_ordinal: u8,
    pub invocation_id: MachineTextV1,
    pub builder_profile_root: ProtocolDigestV1,
    pub isolation: BuilderIsolationEvidenceV1,
    pub admitted_commit: GitHexIdV1,
    pub source_closure_root: ProtocolDigestV1,
    pub build_definition_root: ProtocolDigestV1,
    pub derivation_path: MachineTextV1,
    pub derivation_identity: ArtifactIdentityV1,
    pub final_output_store_path: MachineTextV1,
    pub final_output_locally_built: bool,
    pub final_output_substituted: bool,
    pub nar_hash_reported_by_nix: MachineTextV1,
    pub nar_size_reported_by_nix: u64,
    pub nar_artifact: ArtifactIdentityV1,
    pub reference_set_root: ProtocolDigestV1,
    pub output_file_manifest_root: ProtocolDigestV1,
    pub build_log: ArtifactIdentityV1,
    pub started_at_utc: MachineTextV1,
    pub finished_at_utc: MachineTextV1,
    pub terminal: FreshBuilderTerminalV1,
}

impl FreshBuilderRunV1 {
    pub fn canonical_root(&self) -> Result<ProtocolDigestV1, DigestErrorV1> {
        let mut view = self.clone();
        let blank = MachineTextV1::new("").expect("empty is ASCII");
        view.started_at_utc = blank.clone();
        view.finished_at_utc = blank;
        digest_manifest_value_v1(DigestDomainIdV1::FreshBuilderRun, &view, &fresh_rebuild_limits_v1())
    }
}

/// Fleet-authored realization of the packet's undefined `OutputMismatchV1`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputMismatchV1 {
    /// Which comparison broke first: "nar-hash" | "nar-size" |
    /// "reference-set" | "nar-bytes" | "file-manifest".
    pub comparison: MachineTextV1,
    /// First differing member (relative path, reference, or byte offset
    /// rendered as decimal ASCII for "nar-bytes").
    pub first_differing_member: MachineTextV1,
}

/// Packet section 8.4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshRebuildPairV1 {
    pub pair_id: MachineTextV1,
    pub profile_root: ProtocolDigestV1,
    pub run_a: FreshBuilderRunV1,
    pub run_b: FreshBuilderRunV1,
    pub source_closure_equal: bool,
    pub build_definition_equal: bool,
    pub derivation_equal: bool,
    pub builders_isolated: bool,
    pub final_outputs_local: bool,
    pub nar_hash_equal: bool,
    pub nar_size_equal: bool,
    pub reference_set_equal: bool,
    pub exact_nar_bytes_equal: bool,
    pub first_mismatch: Option<OutputMismatchV1>,
    pub canary_campaign_root: ProtocolDigestV1,
    pub terminal: FreshRebuildPairTerminalV1,
}

impl FreshRebuildPairV1 {
    pub fn canonical_root(&self) -> Result<ProtocolDigestV1, DigestErrorV1> {
        let mut view = self.clone();
        let blank = MachineTextV1::new("").expect("empty is ASCII");
        for run in [&mut view.run_a, &mut view.run_b] {
            run.started_at_utc = blank.clone();
            run.finished_at_utc = blank.clone();
        }
        digest_manifest_value_v1(DigestDomainIdV1::FreshRebuildPair, &view, &fresh_rebuild_limits_v1())
    }
}

impl ManifestEncodeV1 for SubstituterV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, self.identity.to_manifest_value_v1()?),
            (1, ManifestValueV1::MachineText(self.uri.clone())),
        ])
    }
}
impl ManifestDecodeV1 for SubstituterV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        let identity = ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(0))?)?;
        let uri = take_text(f.take_required(FieldIdV1::new(1))?)?;
        f.finish_no_unknown()?;
        Ok(Self { identity, uri })
    }
}

impl ManifestEncodeV1 for FreshBuilderProfileV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(FRESH_BUILDER_PROFILE_SCHEMA_V1)?)),
            (1, ManifestValueV1::MachineText(self.profile_id.clone())),
            (2, ManifestValueV1::MachineText(self.target_system.clone())),
            (3, ManifestValueV1::MachineText(self.target_triple.clone())),
            (4, self.builder_image.to_manifest_value_v1()?),
            (5, self.nix_cli.to_manifest_value_v1()?),
            (6, self.nix_config_root.to_manifest_value_v1()?),
            (7, ManifestValueV1::Array(
                self.allowed_substituters.iter().map(|s| s.to_manifest_value_v1()).collect::<Result<_, _>>()?,
            )),
            (8, ManifestValueV1::Bool(self.final_derivation_must_build_locally)),
            (9, ManifestValueV1::Unsigned(self.dependency_substitution_policy.as_u16() as u64)),
            (10, ManifestValueV1::Unsigned(self.network_phase_policy.as_u16() as u64)),
            (11, ManifestValueV1::Unsigned(self.max_builds_per_instance as u64)),
        ])
    }
}
impl ManifestDecodeV1 for FreshBuilderProfileV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        if take_text(f.take_required(FieldIdV1::new(0))?)?.as_str() != FRESH_BUILDER_PROFILE_SCHEMA_V1 {
            return Err(err("wrong profile schema tag"));
        }
        let out = Self {
            profile_id: take_text(f.take_required(FieldIdV1::new(1))?)?,
            target_system: take_text(f.take_required(FieldIdV1::new(2))?)?,
            target_triple: take_text(f.take_required(FieldIdV1::new(3))?)?,
            builder_image: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(4))?)?,
            nix_cli: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(5))?)?,
            nix_config_root: ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(6))?)?,
            allowed_substituters: take_array(f.take_required(FieldIdV1::new(7))?)?
                .into_iter()
                .map(SubstituterV1::from_manifest_value_v1)
                .collect::<Result<Vec<_>, _>>()?,
            final_derivation_must_build_locally: take_bool(f.take_required(FieldIdV1::new(8))?)?,
            dependency_substitution_policy: take_u16_enum(
                f.take_required(FieldIdV1::new(9))?,
                DependencySubstitutionPolicyV1::try_from_u16,
            )?,
            network_phase_policy: take_u16_enum(
                f.take_required(FieldIdV1::new(10))?,
                NetworkPhasePolicyV1::try_from_u16,
            )?,
            max_builds_per_instance: u32::try_from(take_unsigned(f.take_required(FieldIdV1::new(11))?)?)
                .map_err(|_| err("max_builds out of range"))?,
        };
        f.finish_no_unknown()?;
        // Canonical-order admission: a wire profile whose substituters are
        // out of the packet's declared order is NOT canonical.
        let mut sorted = out.clone();
        sorted.normalize();
        if sorted.allowed_substituters != out.allowed_substituters {
            return Err(err("allowed_substituters not in canonical order"));
        }
        Ok(out)
    }
}

impl ManifestEncodeV1 for BuilderIsolationEvidenceV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(self.builder_instance_id.clone())),
            (1, self.provider_instance_identity.to_manifest_value_v1()?),
            (2, self.boot_identity.to_manifest_value_v1()?),
            (3, self.rootfs_identity.to_manifest_value_v1()?),
            (4, self.writable_store_identity.to_manifest_value_v1()?),
            (5, self.workspace_identity.to_manifest_value_v1()?),
            (6, ManifestValueV1::Array(
                self.shared_writable_mounts.iter().map(|t| ManifestValueV1::MachineText(t.clone())).collect(),
            )),
            (7, ManifestValueV1::Bool(self.project_cache_detected)),
            (8, ManifestValueV1::Bool(self.final_output_preexisting)),
        ])
    }
}
impl ManifestDecodeV1 for BuilderIsolationEvidenceV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        let out = Self {
            builder_instance_id: take_text(f.take_required(FieldIdV1::new(0))?)?,
            provider_instance_identity: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(1))?)?,
            boot_identity: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(2))?)?,
            rootfs_identity: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(3))?)?,
            writable_store_identity: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(4))?)?,
            workspace_identity: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(5))?)?,
            shared_writable_mounts: take_array(f.take_required(FieldIdV1::new(6))?)?
                .into_iter()
                .map(take_text)
                .collect::<Result<Vec<_>, _>>()?,
            project_cache_detected: take_bool(f.take_required(FieldIdV1::new(7))?)?,
            final_output_preexisting: take_bool(f.take_required(FieldIdV1::new(8))?)?,
        };
        f.finish_no_unknown()?;
        Ok(out)
    }
}

impl ManifestEncodeV1 for FreshBuilderRunV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(FRESH_BUILDER_RUN_SCHEMA_V1)?)),
            (1, ManifestValueV1::MachineText(self.pair_id.clone())),
            (2, ManifestValueV1::Unsigned(self.builder_ordinal as u64)),
            (3, ManifestValueV1::MachineText(self.invocation_id.clone())),
            (4, self.builder_profile_root.to_manifest_value_v1()?),
            (5, self.isolation.to_manifest_value_v1()?),
            (6, ManifestValueV1::MachineText(MachineTextV1::new(self.admitted_commit.as_str())?)),
            (7, self.source_closure_root.to_manifest_value_v1()?),
            (8, self.build_definition_root.to_manifest_value_v1()?),
            (9, ManifestValueV1::MachineText(self.derivation_path.clone())),
            (10, self.derivation_identity.to_manifest_value_v1()?),
            (11, ManifestValueV1::MachineText(self.final_output_store_path.clone())),
            (12, ManifestValueV1::Bool(self.final_output_locally_built)),
            (13, ManifestValueV1::Bool(self.final_output_substituted)),
            (14, ManifestValueV1::MachineText(self.nar_hash_reported_by_nix.clone())),
            (15, ManifestValueV1::Unsigned(self.nar_size_reported_by_nix)),
            (16, self.nar_artifact.to_manifest_value_v1()?),
            (17, self.reference_set_root.to_manifest_value_v1()?),
            (18, self.output_file_manifest_root.to_manifest_value_v1()?),
            (19, self.build_log.to_manifest_value_v1()?),
            (20, ManifestValueV1::MachineText(self.started_at_utc.clone())),
            (21, ManifestValueV1::MachineText(self.finished_at_utc.clone())),
            (22, ManifestValueV1::Unsigned(self.terminal.as_u16() as u64)),
        ])
    }
}
impl ManifestDecodeV1 for FreshBuilderRunV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        if take_text(f.take_required(FieldIdV1::new(0))?)?.as_str() != FRESH_BUILDER_RUN_SCHEMA_V1 {
            return Err(err("wrong run schema tag"));
        }
        let out = Self {
            pair_id: take_text(f.take_required(FieldIdV1::new(1))?)?,
            builder_ordinal: {
                let o = take_unsigned(f.take_required(FieldIdV1::new(2))?)?;
                if o > 1 {
                    return Err(err("builder_ordinal must be 0 or 1"));
                }
                o as u8
            },
            invocation_id: take_text(f.take_required(FieldIdV1::new(3))?)?,
            builder_profile_root: ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(4))?)?,
            isolation: BuilderIsolationEvidenceV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(5))?)?,
            admitted_commit: GitHexIdV1::new(take_text(f.take_required(FieldIdV1::new(6))?)?.as_str())
                .map_err(|_| err("admitted commit not 40-lower-hex"))?,
            source_closure_root: ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(7))?)?,
            build_definition_root: ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(8))?)?,
            derivation_path: take_text(f.take_required(FieldIdV1::new(9))?)?,
            derivation_identity: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(10))?)?,
            final_output_store_path: take_text(f.take_required(FieldIdV1::new(11))?)?,
            final_output_locally_built: take_bool(f.take_required(FieldIdV1::new(12))?)?,
            final_output_substituted: take_bool(f.take_required(FieldIdV1::new(13))?)?,
            nar_hash_reported_by_nix: take_text(f.take_required(FieldIdV1::new(14))?)?,
            nar_size_reported_by_nix: take_unsigned(f.take_required(FieldIdV1::new(15))?)?,
            nar_artifact: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(16))?)?,
            reference_set_root: ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(17))?)?,
            output_file_manifest_root: ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(18))?)?,
            build_log: ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(19))?)?,
            started_at_utc: take_text(f.take_required(FieldIdV1::new(20))?)?,
            finished_at_utc: take_text(f.take_required(FieldIdV1::new(21))?)?,
            terminal: take_u16_enum(f.take_required(FieldIdV1::new(22))?, FreshBuilderTerminalV1::try_from_u16)?,
        };
        f.finish_no_unknown()?;
        // Packet policy 5/7: a run claiming BuildPass must have actually
        // executed the final derivation locally, unsubstituted.
        if out.terminal == FreshBuilderTerminalV1::BuildPass
            && (!out.final_output_locally_built || out.final_output_substituted)
        {
            return Err(err("BuildPass requires local unsubstituted final output"));
        }
        Ok(out)
    }
}

impl ManifestEncodeV1 for OutputMismatchV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(self.comparison.clone())),
            (1, ManifestValueV1::MachineText(self.first_differing_member.clone())),
        ])
    }
}
impl ManifestDecodeV1 for OutputMismatchV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        let comparison = take_text(f.take_required(FieldIdV1::new(0))?)?;
        let first_differing_member = take_text(f.take_required(FieldIdV1::new(1))?)?;
        f.finish_no_unknown()?;
        Ok(Self { comparison, first_differing_member })
    }
}

impl ManifestEncodeV1 for FreshRebuildPairV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut entries = vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(FRESH_REBUILD_PAIR_SCHEMA_V1)?)),
            (1, ManifestValueV1::MachineText(self.pair_id.clone())),
            (2, self.profile_root.to_manifest_value_v1()?),
            (3, self.run_a.to_manifest_value_v1()?),
            (4, self.run_b.to_manifest_value_v1()?),
            (5, ManifestValueV1::Bool(self.source_closure_equal)),
            (6, ManifestValueV1::Bool(self.build_definition_equal)),
            (7, ManifestValueV1::Bool(self.derivation_equal)),
            (8, ManifestValueV1::Bool(self.builders_isolated)),
            (9, ManifestValueV1::Bool(self.final_outputs_local)),
            (10, ManifestValueV1::Bool(self.nar_hash_equal)),
            (11, ManifestValueV1::Bool(self.nar_size_equal)),
            (12, ManifestValueV1::Bool(self.reference_set_equal)),
            (13, ManifestValueV1::Bool(self.exact_nar_bytes_equal)),
            (15, self.canary_campaign_root.to_manifest_value_v1()?),
            (16, ManifestValueV1::Unsigned(self.terminal.as_u16() as u64)),
        ];
        if let Some(m) = &self.first_mismatch {
            entries.push((14, m.to_manifest_value_v1()?));
        }
        map_value(entries)
    }
}
impl ManifestDecodeV1 for FreshRebuildPairV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        if take_text(f.take_required(FieldIdV1::new(0))?)?.as_str() != FRESH_REBUILD_PAIR_SCHEMA_V1 {
            return Err(err("wrong pair schema tag"));
        }
        let pair_id = take_text(f.take_required(FieldIdV1::new(1))?)?;
        let profile_root = ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(2))?)?;
        let run_a = FreshBuilderRunV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(3))?)?;
        let run_b = FreshBuilderRunV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(4))?)?;
        let source_closure_equal = take_bool(f.take_required(FieldIdV1::new(5))?)?;
        let build_definition_equal = take_bool(f.take_required(FieldIdV1::new(6))?)?;
        let derivation_equal = take_bool(f.take_required(FieldIdV1::new(7))?)?;
        let builders_isolated = take_bool(f.take_required(FieldIdV1::new(8))?)?;
        let final_outputs_local = take_bool(f.take_required(FieldIdV1::new(9))?)?;
        let nar_hash_equal = take_bool(f.take_required(FieldIdV1::new(10))?)?;
        let nar_size_equal = take_bool(f.take_required(FieldIdV1::new(11))?)?;
        let reference_set_equal = take_bool(f.take_required(FieldIdV1::new(12))?)?;
        let exact_nar_bytes_equal = take_bool(f.take_required(FieldIdV1::new(13))?)?;
        let first_mismatch = f
            .take_optional(FieldIdV1::new(14))?
            .map(OutputMismatchV1::from_manifest_value_v1)
            .transpose()?;
        let canary_campaign_root = ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(15))?)?;
        let terminal =
            take_u16_enum(f.take_required(FieldIdV1::new(16))?, FreshRebuildPairTerminalV1::try_from_u16)?;
        f.finish_no_unknown()?;

        // Structural admission (packet sections 1/6): a pass claim must be
        // backed by EVERY comparison bit; V1 may only claim the
        // same-trust-domain pass; a recorded mismatch forbids a pass.
        let is_pass = matches!(
            terminal,
            FreshRebuildPairTerminalV1::PairPass | FreshRebuildPairTerminalV1::PairPassSameTrustDomain
        );
        if terminal == FreshRebuildPairTerminalV1::PairPass {
            return Err(err("V1 controller may only claim PairPassSameTrustDomain (packet policy 12)"));
        }
        if is_pass {
            let all = source_closure_equal
                && build_definition_equal
                && derivation_equal
                && builders_isolated
                && final_outputs_local
                && nar_hash_equal
                && nar_size_equal
                && reference_set_equal
                && exact_nar_bytes_equal;
            if !all {
                return Err(err("pair pass requires every comparison bit true"));
            }
            if first_mismatch.is_some() {
                return Err(err("pair pass cannot carry a mismatch record"));
            }
            if run_a.terminal != FreshBuilderTerminalV1::BuildPass || run_b.terminal != FreshBuilderTerminalV1::BuildPass {
                return Err(err("pair pass requires both runs BuildPass"));
            }
            if run_a.builder_ordinal == run_b.builder_ordinal {
                return Err(err("runs must have distinct ordinals"));
            }
            if run_a.isolation.builder_instance_id.as_str() == run_b.isolation.builder_instance_id.as_str() {
                return Err(err("pair pass on a shared builder instance"));
            }
        }
        Ok(Self {
            pair_id,
            profile_root,
            run_a,
            run_b,
            source_closure_equal,
            build_definition_equal,
            derivation_equal,
            builders_isolated,
            final_outputs_local,
            nar_hash_equal,
            nar_size_equal,
            reference_set_equal,
            exact_nar_bytes_equal,
            first_mismatch,
            canary_campaign_root,
            terminal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::{digest_canonical_bytes_v1, hash_artifact_bytes_v1};
    use crate::apex::manifest::{decode_manifest_v1, encode_manifest_v1};

    fn proto(payload: &[u8]) -> ProtocolDigestV1 {
        digest_canonical_bytes_v1(DigestDomainIdV1::FreshRebuildPair, payload, 1 << 20).unwrap()
    }

    fn art(b: &[u8]) -> ArtifactIdentityV1 { hash_artifact_bytes_v1(b) }

    fn text(s: &str) -> MachineTextV1 { MachineTextV1::new(s).unwrap() }

    fn isolation(id: &str) -> BuilderIsolationEvidenceV1 {
        BuilderIsolationEvidenceV1 {
            builder_instance_id: text(id),
            provider_instance_identity: art(id.as_bytes()),
            boot_identity: art(b"boot"),
            rootfs_identity: art(b"rootfs"),
            writable_store_identity: art(id.as_bytes()),
            workspace_identity: art(id.as_bytes()),
            shared_writable_mounts: vec![],
            project_cache_detected: false,
            final_output_preexisting: false,
        }
    }

    fn run(ordinal: u8, instance: &str) -> FreshBuilderRunV1 {
        FreshBuilderRunV1 {
            pair_id: text("pair-0001"),
            builder_ordinal: ordinal,
            invocation_id: text("inv"),
            builder_profile_root: proto(b"profile"),
            isolation: isolation(instance),
            admitted_commit: GitHexIdV1::new("0123456789abcdef0123456789abcdef01234567").unwrap(),
            source_closure_root: proto(b"closure"),
            build_definition_root: proto(b"drv-def"),
            derivation_path: text("/nix/store/aaaa.drv"),
            derivation_identity: art(b"drv"),
            final_output_store_path: text("/nix/store/bbbb-out"),
            final_output_locally_built: true,
            final_output_substituted: false,
            nar_hash_reported_by_nix: text("sha256-abc"),
            nar_size_reported_by_nix: 42,
            nar_artifact: art(b"nar"),
            reference_set_root: proto(b"refs"),
            output_file_manifest_root: proto(b"manifest"),
            build_log: art(b"log"),
            started_at_utc: text("2026-07-27T08:00:00Z"),
            finished_at_utc: text("2026-07-27T09:00:00Z"),
            terminal: FreshBuilderTerminalV1::BuildPass,
        }
    }

    fn pair() -> FreshRebuildPairV1 {
        FreshRebuildPairV1 {
            pair_id: text("pair-0001"),
            profile_root: proto(b"profile"),
            run_a: run(0, "vm-a"),
            run_b: run(1, "vm-b"),
            source_closure_equal: true,
            build_definition_equal: true,
            derivation_equal: true,
            builders_isolated: true,
            final_outputs_local: true,
            nar_hash_equal: true,
            nar_size_equal: true,
            reference_set_equal: true,
            exact_nar_bytes_equal: true,
            first_mismatch: None,
            canary_campaign_root: proto(b"canaries"),
            terminal: FreshRebuildPairTerminalV1::PairPassSameTrustDomain,
        }
    }

    #[test]
    fn pair_round_trips_canonically() {
        let p = pair();
        let limits = fresh_rebuild_limits_v1();
        let bytes = encode_manifest_v1(&p, &limits).unwrap();
        let decoded: FreshRebuildPairV1 = decode_manifest_v1(&bytes, &limits).unwrap();
        assert_eq!(decoded, p);
        assert_eq!(encode_manifest_v1(&decoded, &limits).unwrap(), bytes);
    }

    #[test]
    fn pass_admission_bites() {
        let limits = fresh_rebuild_limits_v1();

        // One false comparison bit under a pass claim fails decode.
        let mut washed = pair();
        washed.exact_nar_bytes_equal = false;
        let bytes = encode_manifest_v1(&washed, &limits).unwrap();
        assert!(decode_manifest_v1::<FreshRebuildPairV1>(&bytes, &limits).is_err());

        // Same-instance pair cannot pass.
        let mut shared = pair();
        shared.run_b.isolation.builder_instance_id = text("vm-a");
        let bytes = encode_manifest_v1(&shared, &limits).unwrap();
        assert!(decode_manifest_v1::<FreshRebuildPairV1>(&bytes, &limits).is_err());

        // V1 may never claim the unqualified PairPass.
        let mut overclaim = pair();
        overclaim.terminal = FreshRebuildPairTerminalV1::PairPass;
        let bytes = encode_manifest_v1(&overclaim, &limits).unwrap();
        assert!(decode_manifest_v1::<FreshRebuildPairV1>(&bytes, &limits).is_err());

        // An honest mismatch record decodes fine.
        let mut red = pair();
        red.exact_nar_bytes_equal = false;
        red.terminal = FreshRebuildPairTerminalV1::OutputMismatch;
        red.first_mismatch = Some(OutputMismatchV1 {
            comparison: text("nar-bytes"),
            first_differing_member: text("4096"),
        });
        let bytes = encode_manifest_v1(&red, &limits).unwrap();
        assert!(decode_manifest_v1::<FreshRebuildPairV1>(&bytes, &limits).is_ok());
    }

    #[test]
    fn run_admission_and_root_semantics() {
        let limits = fresh_rebuild_limits_v1();

        // BuildPass with a substituted final output fails decode.
        let mut subbed = run(0, "vm-a");
        subbed.final_output_substituted = true;
        let bytes = encode_manifest_v1(&subbed, &limits).unwrap();
        assert!(decode_manifest_v1::<FreshBuilderRunV1>(&bytes, &limits).is_err());

        // Wall clock never moves a canonical root; NAR identity does.
        let a = run(0, "vm-a");
        let mut b = run(0, "vm-a");
        b.finished_at_utc = text("2026-07-28T00:00:00Z");
        assert_eq!(a.canonical_root().unwrap(), b.canonical_root().unwrap());
        let mut c = run(0, "vm-a");
        c.nar_artifact = art(b"different nar");
        assert_ne!(a.canonical_root().unwrap(), c.canonical_root().unwrap());
    }

    #[test]
    fn profile_substituter_order_is_canonical() {
        let limits = fresh_rebuild_limits_v1();
        let s1 = SubstituterV1 { identity: art(b"cache-b"), uri: text("https://b.example") };
        let s2 = SubstituterV1 { identity: art(b"cache-a"), uri: text("https://a.example") };
        let mut profile = FreshBuilderProfileV1 {
            profile_id: text("apex-fresh-pair-x86_64-linux-v1"),
            target_system: text("x86_64-linux"),
            target_triple: text("x86_64-unknown-linux-gnu"),
            builder_image: art(b"bastion-golden-nix"),
            nix_cli: art(b"nix-2.24.9"),
            nix_config_root: proto(b"nix-config"),
            allowed_substituters: vec![s1.clone(), s2.clone()],
            final_derivation_must_build_locally: true,
            dependency_substitution_policy: DependencySubstitutionPolicyV1::PinnedDigestVerifiedOnly,
            network_phase_policy: NetworkPhasePolicyV1::SubstituteThenOfflineFinal,
            max_builds_per_instance: 1,
        };
        // Unsorted profile encodes, but its wire form is rejected at decode.
        let bytes = encode_manifest_v1(&profile, &limits).unwrap();
        let round: Result<FreshBuilderProfileV1, _> = decode_manifest_v1(&bytes, &limits);
        let expect_reject = {
            let mut sorted = profile.clone();
            sorted.normalize();
            sorted.allowed_substituters != profile.allowed_substituters
        };
        assert_eq!(round.is_err(), expect_reject, "decode must reject exactly when order is non-canonical");
        profile.normalize();
        let bytes = encode_manifest_v1(&profile, &limits).unwrap();
        let decoded: FreshBuilderProfileV1 = decode_manifest_v1(&bytes, &limits).unwrap();
        assert_eq!(decoded, profile);
        assert!(profile.canonical_root().is_ok());
    }

    #[test]
    fn sealed_enums_fail_closed() {
        assert!(FreshBuilderTerminalV1::try_from_u16(13).is_err());
        assert!(FreshRebuildPairTerminalV1::try_from_u16(18).is_err());
        assert!(DependencySubstitutionPolicyV1::try_from_u16(1).is_err());
        assert!(NetworkPhasePolicyV1::try_from_u16(1).is_err());
        assert_eq!(FreshBuilderTerminalV1::ALL.len(), 13);
        assert_eq!(FreshRebuildPairTerminalV1::ALL.len(), 18);
    }
}
