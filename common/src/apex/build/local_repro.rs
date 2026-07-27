//! `APEX-T1.3.01` — the local reproducibility smoke contract (real packet
//! `APEX-T1.3`, section 7): one frozen profile for the same-worker
//! exact-output rebuild + host-path impurity smoke.
//!
//! The artifact boundary is the ENTIRE `packages.bastion-harness-repro`
//! Nix output — never a single ELF, never a normalized copy. Wall-clock
//! timestamps and raw host paths are DIAGNOSTIC: the canonical root is
//! computed over the record with timestamps blanked (packet section 7:
//! "excluded from equality/root fields"), and host paths appear only as
//! stable digest tokens. Packet divergence, documented: the packet's
//! `record_root` struct field is realized as [`canonical_root`] +
//! emission sidecar rather than a self-referential field — a root cannot
//! be a field of the very encoding it digests.
//!
//! [`canonical_root`]: LocalReproducibilitySmokeV1::canonical_root

use crate::apex::digest::{
    ArtifactDigestV1, ArtifactIdentityV1, DigestDomainIdV1, DigestErrorV1, ProtocolDigestV1, digest_manifest_value_v1,
};
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, MachineTextV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1,
    ManifestDecodeLimitsV1, ManifestDecodeV1, ManifestEncodeV1, ManifestErrorV1, ManifestSchemaErrorV1,
    ManifestValueV1, StructFieldsV1,
};
use crate::apex::source_closure::GitHexIdV1;

pub const LOCAL_REPRO_SMOKE_SCHEMA_V1: &str = "bastion.local-repro-smoke/v1";
pub const LOCAL_REPRO_PROFILE_V1: &str = "apex-local-repro-x86_64-linux-v1";

/// Own limits (T0.2 has no `Default`): a smoke record is small — a
/// handful of executions, digests, and short ASCII store paths.
pub const fn local_repro_limits_v1() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 1 << 20,
        max_depth: 8,
        max_nodes: 1 << 14,
        max_array_items: 256,
        max_map_entries: 32,
        max_machine_text_bytes: 4096,
        max_byte_string_bytes: 4096,
    }
}

/// Packet section 7 terminal set — SEALED u16 mapping; decode of an
/// unknown discriminant fails closed.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalReproTerminalV1 {
    Pass = 0,
    BlockSourceClosure = 1,
    BlockDerivationDrift = 2,
    BlockImpureEvaluation = 3,
    BlockConcurrentBuild = 4,
    BlockProjectCache = 5,
    BlockNonlocalBaseline = 6,
    BlockBaselineBuild = 7,
    BlockRebuild = 8,
    BlockNondeterministicOutput = 9,
    BlockOutputInventory = 10,
    BlockDiagnosticCapture = 11,
    BlockCanaryFalsePositive = 12,
    BlockCanaryFalseNegative = 13,
    BlockMutationSurvived = 14,
    BlockEvidencePartial = 15,
}

impl LocalReproTerminalV1 {
    pub const ALL: [LocalReproTerminalV1; 16] = [
        Self::Pass,
        Self::BlockSourceClosure,
        Self::BlockDerivationDrift,
        Self::BlockImpureEvaluation,
        Self::BlockConcurrentBuild,
        Self::BlockProjectCache,
        Self::BlockNonlocalBaseline,
        Self::BlockBaselineBuild,
        Self::BlockRebuild,
        Self::BlockNondeterministicOutput,
        Self::BlockOutputInventory,
        Self::BlockDiagnosticCapture,
        Self::BlockCanaryFalsePositive,
        Self::BlockCanaryFalseNegative,
        Self::BlockMutationSurvived,
        Self::BlockEvidencePartial,
    ];

    pub const fn as_u16(self) -> u16 { self as u16 }

    pub fn try_from_u16(v: u16) -> Result<Self, ManifestSchemaErrorV1> {
        Self::ALL
            .into_iter()
            .find(|t| t.as_u16() == v)
            .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown terminal"))
    }
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildExecutionKindV1 {
    Baseline = 0,
    RebuildCheck = 1,
}

impl BuildExecutionKindV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub fn try_from_u16(v: u16) -> Result<Self, ManifestSchemaErrorV1> {
        match v {
            0 => Ok(Self::Baseline),
            1 => Ok(Self::RebuildCheck),
            _ => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("unknown execution kind")),
        }
    }
}

/// One build execution. `started_at`/`finished_at` are DIAGNOSTIC ASCII
/// timestamps — blanked before the canonical root is computed. Host paths
/// never appear raw: `source_path_token`/`out_link_path_token` are
/// sha256 digests of the absolute path bytes (stable tokens that prove
/// A≠B without leaking machine layout into evidence).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildExecutionV1 {
    pub ordinal: u32,
    pub execution_kind: BuildExecutionKindV1,
    pub locally_executed: bool,
    pub source_path_token: ArtifactDigestV1,
    pub out_link_path_token: ArtifactDigestV1,
    pub exit_code: i64,
    pub log_identity: ArtifactIdentityV1,
    pub started_at: MachineTextV1,
    pub finished_at: MachineTextV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostPathEvaluationV1 {
    pub materialization_a_token: ArtifactDigestV1,
    pub materialization_b_token: ArtifactDigestV1,
    pub closure_roots_equal: bool,
    pub derivations_equal: bool,
}

/// The smoke record (packet section 7, field IDs frozen in struct order).
/// `baseline_built_this_run` realizes the packet's baseline-provenance
/// distinction (T1.3.07: `baseline_built_this_run` vs
/// `baseline_preexisting_verified`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalReproducibilitySmokeV1 {
    pub admitted_commit: GitHexIdV1,
    pub source_closure_root: ProtocolDigestV1,
    pub derivation_path: MachineTextV1,
    pub derivation_identity: ArtifactIdentityV1,
    pub output_store_path: MachineTextV1,
    pub output_nar_identity: ArtifactIdentityV1,
    pub baseline: BuildExecutionV1,
    pub baseline_built_this_run: bool,
    pub rebuilds: Vec<BuildExecutionV1>,
    pub host_path_evaluation: HostPathEvaluationV1,
    pub output_manifest_root: ProtocolDigestV1,
    pub canary_root: ProtocolDigestV1,
    pub terminal: LocalReproTerminalV1,
}

impl LocalReproducibilitySmokeV1 {
    /// The canonical record root under `LocalReproSmoke` (= 12), computed
    /// with every diagnostic timestamp blanked so wall-clock never moves
    /// the root (packet section 7's exclusion rule).
    pub fn canonical_root(&self) -> Result<ProtocolDigestV1, DigestErrorV1> {
        let mut view = self.clone();
        let blank = MachineTextV1::new("").expect("empty is ASCII");
        view.baseline.started_at = blank.clone();
        view.baseline.finished_at = blank.clone();
        for r in &mut view.rebuilds {
            r.started_at = blank.clone();
            r.finished_at = blank.clone();
        }
        digest_manifest_value_v1(DigestDomainIdV1::LocalReproSmoke, &view, &local_repro_limits_v1())
    }
}

fn err(detail: &'static str) -> ManifestSchemaErrorV1 {
    ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail(detail)
}

fn map_value(entries: Vec<(u16, ManifestValueV1)>) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    let entries = entries.into_iter().map(|(id, v)| (FieldIdV1::new(id), v)).collect();
    Ok(ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries)?))
}

fn integer_value(v: i64) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    if v >= 0 { Ok(ManifestValueV1::Unsigned(v as u64)) } else { ManifestValueV1::negative(v) }
}

fn take_unsigned(value: ManifestValueV1) -> Result<u64, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Unsigned(v) => Ok(v),
        _ => Err(err("expected unsigned")),
    }
}

fn take_integer(value: ManifestValueV1) -> Result<i64, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Unsigned(v) => i64::try_from(v).map_err(|_| err("integer out of range")),
        ManifestValueV1::Negative(v) => Ok(v),
        _ => Err(err("expected integer")),
    }
}

fn take_bool(value: ManifestValueV1) -> Result<bool, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Bool(b) => Ok(b),
        _ => Err(err("expected bool")),
    }
}

fn take_text(value: ManifestValueV1) -> Result<MachineTextV1, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::MachineText(t) => Ok(t),
        _ => Err(err("expected machine text")),
    }
}

fn take_map(value: ManifestValueV1) -> Result<StructFieldsV1, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Map(m) => Ok(StructFieldsV1::new(m)),
        _ => Err(err("expected map")),
    }
}

impl ManifestEncodeV1 for BuildExecutionV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::Unsigned(self.ordinal as u64)),
            (1, ManifestValueV1::Unsigned(self.execution_kind.as_u16() as u64)),
            (2, ManifestValueV1::Bool(self.locally_executed)),
            (3, self.source_path_token.to_manifest_value_v1()?),
            (4, self.out_link_path_token.to_manifest_value_v1()?),
            (5, integer_value(self.exit_code)?),
            (6, self.log_identity.to_manifest_value_v1()?),
            (7, ManifestValueV1::MachineText(self.started_at.clone())),
            (8, ManifestValueV1::MachineText(self.finished_at.clone())),
        ])
    }
}

impl ManifestDecodeV1 for BuildExecutionV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(value)?;
        let ordinal = u32::try_from(take_unsigned(f.take_required(FieldIdV1::new(0))?)?)
            .map_err(|_| err("ordinal out of range"))?;
        let execution_kind = BuildExecutionKindV1::try_from_u16(
            u16::try_from(take_unsigned(f.take_required(FieldIdV1::new(1))?)?).map_err(|_| err("kind out of range"))?,
        )?;
        let locally_executed = take_bool(f.take_required(FieldIdV1::new(2))?)?;
        let source_path_token = ArtifactDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(3))?)?;
        let out_link_path_token = ArtifactDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(4))?)?;
        let exit_code = take_integer(f.take_required(FieldIdV1::new(5))?)?;
        let log_identity = ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(6))?)?;
        let started_at = take_text(f.take_required(FieldIdV1::new(7))?)?;
        let finished_at = take_text(f.take_required(FieldIdV1::new(8))?)?;
        f.finish_no_unknown()?;
        Ok(Self {
            ordinal,
            execution_kind,
            locally_executed,
            source_path_token,
            out_link_path_token,
            exit_code,
            log_identity,
            started_at,
            finished_at,
        })
    }
}

impl ManifestEncodeV1 for HostPathEvaluationV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, self.materialization_a_token.to_manifest_value_v1()?),
            (1, self.materialization_b_token.to_manifest_value_v1()?),
            (2, ManifestValueV1::Bool(self.closure_roots_equal)),
            (3, ManifestValueV1::Bool(self.derivations_equal)),
        ])
    }
}

impl ManifestDecodeV1 for HostPathEvaluationV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(value)?;
        let materialization_a_token = ArtifactDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(0))?)?;
        let materialization_b_token = ArtifactDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(1))?)?;
        let closure_roots_equal = take_bool(f.take_required(FieldIdV1::new(2))?)?;
        let derivations_equal = take_bool(f.take_required(FieldIdV1::new(3))?)?;
        f.finish_no_unknown()?;
        Ok(Self { materialization_a_token, materialization_b_token, closure_roots_equal, derivations_equal })
    }
}

impl ManifestEncodeV1 for LocalReproducibilitySmokeV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(LOCAL_REPRO_SMOKE_SCHEMA_V1)?)),
            (1, ManifestValueV1::MachineText(MachineTextV1::new(LOCAL_REPRO_PROFILE_V1)?)),
            (2, ManifestValueV1::MachineText(MachineTextV1::new(self.admitted_commit.as_str())?)),
            (3, self.source_closure_root.to_manifest_value_v1()?),
            (4, ManifestValueV1::MachineText(self.derivation_path.clone())),
            (5, self.derivation_identity.to_manifest_value_v1()?),
            (6, ManifestValueV1::MachineText(self.output_store_path.clone())),
            (7, self.output_nar_identity.to_manifest_value_v1()?),
            (8, self.baseline.to_manifest_value_v1()?),
            (9, ManifestValueV1::Bool(self.baseline_built_this_run)),
            (10, ManifestValueV1::Array(
                self.rebuilds.iter().map(|r| r.to_manifest_value_v1()).collect::<Result<_, _>>()?,
            )),
            (11, self.host_path_evaluation.to_manifest_value_v1()?),
            (12, self.output_manifest_root.to_manifest_value_v1()?),
            (13, self.canary_root.to_manifest_value_v1()?),
            (14, ManifestValueV1::Unsigned(self.terminal.as_u16() as u64)),
        ])
    }
}

impl ManifestDecodeV1 for LocalReproducibilitySmokeV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(value)?;
        let schema = take_text(f.take_required(FieldIdV1::new(0))?)?;
        if schema.as_str() != LOCAL_REPRO_SMOKE_SCHEMA_V1 {
            return Err(err("wrong schema tag"));
        }
        let profile = take_text(f.take_required(FieldIdV1::new(1))?)?;
        if profile.as_str() != LOCAL_REPRO_PROFILE_V1 {
            return Err(err("unknown local-repro profile"));
        }
        let admitted_commit = GitHexIdV1::new(take_text(f.take_required(FieldIdV1::new(2))?)?.as_str())
            .map_err(|_| err("admitted commit is not 40-lower-hex"))?;
        let source_closure_root = ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(3))?)?;
        let derivation_path = take_text(f.take_required(FieldIdV1::new(4))?)?;
        let derivation_identity = ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(5))?)?;
        let output_store_path = take_text(f.take_required(FieldIdV1::new(6))?)?;
        let output_nar_identity = ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(7))?)?;
        let baseline = BuildExecutionV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(8))?)?;
        let baseline_built_this_run = take_bool(f.take_required(FieldIdV1::new(9))?)?;
        let rebuilds = match f.take_required(FieldIdV1::new(10))? {
            ManifestValueV1::Array(items) => items
                .into_iter()
                .map(BuildExecutionV1::from_manifest_value_v1)
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(err("rebuilds must be an array")),
        };
        let host_path_evaluation = HostPathEvaluationV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(11))?)?;
        let output_manifest_root = ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(12))?)?;
        let canary_root = ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(13))?)?;
        let terminal = LocalReproTerminalV1::try_from_u16(
            u16::try_from(take_unsigned(f.take_required(FieldIdV1::new(14))?)?)
                .map_err(|_| err("terminal out of range"))?,
        )?;
        f.finish_no_unknown()?;

        // Structural admission rules (packet sections 5.7 + 8/T1.3.08): a
        // PASS record must prove at least two current-run executions —
        // locally built baseline + ≥1 rebuild, or ≥2 rebuilds over a
        // verified preexisting baseline — with strictly increasing
        // ordinals and every comparison execution local.
        if terminal == LocalReproTerminalV1::Pass {
            let fresh = rebuilds.len() + usize::from(baseline_built_this_run);
            if fresh < 2 {
                return Err(err("PASS requires at least two current-run executions"));
            }
            if baseline_built_this_run && !baseline.locally_executed {
                return Err(err("baseline claimed built-this-run but not locally executed"));
            }
            if rebuilds.iter().any(|r| !r.locally_executed || r.execution_kind != BuildExecutionKindV1::RebuildCheck) {
                return Err(err("every rebuild must be a locally executed RebuildCheck"));
            }
            let mut last = baseline.ordinal;
            for r in &rebuilds {
                if r.ordinal <= last {
                    return Err(err("execution ordinals must strictly increase"));
                }
                last = r.ordinal;
            }
        }

        Ok(Self {
            admitted_commit,
            source_closure_root,
            derivation_path,
            derivation_identity,
            output_store_path,
            output_nar_identity,
            baseline,
            baseline_built_this_run,
            rebuilds,
            host_path_evaluation,
            output_manifest_root,
            canary_root,
            terminal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::hash_artifact_bytes_v1;
    use crate::apex::manifest::{decode_manifest_v1, encode_manifest_v1};

    fn digest_of(b: &[u8]) -> ArtifactDigestV1 { hash_artifact_bytes_v1(b).digest }

    fn execution(ordinal: u32, kind: BuildExecutionKindV1) -> BuildExecutionV1 {
        BuildExecutionV1 {
            ordinal,
            execution_kind: kind,
            locally_executed: true,
            source_path_token: digest_of(b"source path A"),
            out_link_path_token: digest_of(b"out link A"),
            exit_code: 0,
            log_identity: hash_artifact_bytes_v1(b"log"),
            started_at: MachineTextV1::new("2026-07-27T00:00:00Z").unwrap(),
            finished_at: MachineTextV1::new("2026-07-27T00:10:00Z").unwrap(),
        }
    }

    fn proto(domain_payload: &[u8]) -> ProtocolDigestV1 {
        use crate::apex::digest::digest_canonical_bytes_v1;
        digest_canonical_bytes_v1(DigestDomainIdV1::LocalReproSmoke, domain_payload, 1 << 20).unwrap()
    }

    fn sample() -> LocalReproducibilitySmokeV1 {
        LocalReproducibilitySmokeV1 {
            admitted_commit: GitHexIdV1::new("0123456789abcdef0123456789abcdef01234567").unwrap(),
            source_closure_root: proto(b"closure"),
            derivation_path: MachineTextV1::new("/nix/store/aaaa-bastion-harness-repro.drv").unwrap(),
            derivation_identity: hash_artifact_bytes_v1(b"drv json"),
            output_store_path: MachineTextV1::new("/nix/store/bbbb-bastion-harness-repro").unwrap(),
            output_nar_identity: hash_artifact_bytes_v1(b"nar"),
            baseline: execution(0, BuildExecutionKindV1::Baseline),
            baseline_built_this_run: true,
            rebuilds: vec![execution(1, BuildExecutionKindV1::RebuildCheck)],
            host_path_evaluation: HostPathEvaluationV1 {
                materialization_a_token: digest_of(b"path a"),
                materialization_b_token: digest_of(b"path b"),
                closure_roots_equal: true,
                derivations_equal: true,
            },
            output_manifest_root: proto(b"manifest"),
            canary_root: proto(b"canaries"),
            terminal: LocalReproTerminalV1::Pass,
        }
    }

    #[test]
    fn round_trips_canonically() {
        let record = sample();
        let limits = local_repro_limits_v1();
        let bytes = encode_manifest_v1(&record, &limits).unwrap();
        let decoded: LocalReproducibilitySmokeV1 = decode_manifest_v1(&bytes, &limits).unwrap();
        assert_eq!(decoded, record);
        assert_eq!(encode_manifest_v1(&decoded, &limits).unwrap(), bytes);
    }

    #[test]
    fn canonical_root_ignores_wall_clock_but_not_output() {
        let a = sample();
        let mut b = sample();
        b.baseline.started_at = MachineTextV1::new("2026-07-28T09:00:00Z").unwrap();
        b.rebuilds[0].finished_at = MachineTextV1::new("2026-07-28T09:20:00Z").unwrap();
        assert_eq!(a.canonical_root().unwrap(), b.canonical_root().unwrap(), "wall clock must not move the root");

        let mut c = sample();
        c.output_nar_identity = hash_artifact_bytes_v1(b"different nar");
        assert_ne!(a.canonical_root().unwrap(), c.canonical_root().unwrap(), "output identity must move the root");
    }

    #[test]
    fn pass_requires_two_current_run_executions() {
        let limits = local_repro_limits_v1();

        // Preexisting baseline + only one rebuild = one current-run
        // execution: decode must fail closed.
        let mut weak = sample();
        weak.baseline_built_this_run = false;
        let bytes = encode_manifest_v1(&weak, &limits).unwrap();
        assert!(decode_manifest_v1::<LocalReproducibilitySmokeV1>(&bytes, &limits).is_err());

        // Same shape with two rebuilds is admissible.
        let mut ok = sample();
        ok.baseline_built_this_run = false;
        ok.rebuilds.push(BuildExecutionV1 { ordinal: 2, ..execution(2, BuildExecutionKindV1::RebuildCheck) });
        let bytes = encode_manifest_v1(&ok, &limits).unwrap();
        assert!(decode_manifest_v1::<LocalReproducibilitySmokeV1>(&bytes, &limits).is_ok());

        // A blocked record with one execution is fine — the rule guards
        // PASS claims only.
        let mut blocked = weak.clone();
        blocked.terminal = LocalReproTerminalV1::BlockRebuild;
        let bytes = encode_manifest_v1(&blocked, &limits).unwrap();
        assert!(decode_manifest_v1::<LocalReproducibilitySmokeV1>(&bytes, &limits).is_ok());
    }

    #[test]
    fn unknown_terminal_and_kind_fail_closed() {
        assert!(LocalReproTerminalV1::try_from_u16(16).is_err());
        assert!(BuildExecutionKindV1::try_from_u16(2).is_err());
        let ids: std::collections::HashSet<u16> =
            LocalReproTerminalV1::ALL.iter().map(|t| t.as_u16()).collect();
        assert_eq!(ids.len(), 16);
    }

    #[test]
    fn non_rebuild_kind_in_rebuilds_fails_on_pass() {
        let limits = local_repro_limits_v1();
        let mut bad = sample();
        bad.rebuilds[0].execution_kind = BuildExecutionKindV1::Baseline;
        // Still two current-run executions, but the rebuild slot lies
        // about its kind.
        bad.rebuilds[0].ordinal = 1;
        let bytes = encode_manifest_v1(&bad, &limits).unwrap();
        assert!(decode_manifest_v1::<LocalReproducibilitySmokeV1>(&bytes, &limits).is_err());
    }
}
