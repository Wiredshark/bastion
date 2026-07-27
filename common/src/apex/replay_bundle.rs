//! `APEX-T1.114` (engine-list): the authoritative replay bundle — a
//! self-contained record of one deterministic run, sufficient to re-run it
//! from `start_checkpoint_digest` through `expected_terminal_tick` without
//! any other input. Builds directly on the paired-run
//! `Verdict`/`FirstDivergence` oracle (`bastion-harness::determinism_regression`)
//! rather than replacing it: a bundle is what that oracle's failing side
//! would need to hand off for offline replay/first-divergence localization,
//! not a new comparison mechanism.
//!
//! Scope decision, disclosed rather than silently narrowed: V1 defines the
//! bundle's SCHEMA and structural admission (the "reject any identity
//! mismatch" ask) only. It does not wire an emitter into
//! `determinism_regression`'s existing failure path, and it does not
//! attempt the row's "two fresh processes per frozen execution cell" oracle
//! mutation-canary campaign — that is VM-fixture weight, not a schema
//! concern (per the local-pins/VM-fixtures split).

use crate::apex::digest::{
    ArtifactIdentityV1, DigestDomainIdV1, DigestErrorV1, ProtocolDigestV1, digest_manifest_value_v1,
};
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, MachineTextV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1,
    ManifestDecodeLimitsV1, ManifestDecodeV1, ManifestEncodeV1, ManifestErrorV1, ManifestSchemaErrorV1,
    ManifestValueV1, StructFieldsV1,
};

pub const REPLAY_RUN_MANIFEST_SCHEMA_V1: &str = "bastion.replay-run-manifest/v1";
pub const REPLAY_BUNDLE_SCHEMA_V1: &str = "bastion.replay-bundle/v1";

pub const fn replay_bundle_limits_v1() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 1 << 24,
        max_depth: 12,
        max_nodes: 1 << 20,
        // The command log is the one field expected to grow large; every
        // other array in this schema stays small.
        max_array_items: 1 << 16,
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
fn take_array(v: ManifestValueV1) -> Result<Vec<ManifestValueV1>, ManifestSchemaErrorV1> {
    match v { ManifestValueV1::Array(a) => Ok(a), _ => Err(err("expected array")) }
}

/// Builder-ready row binding, field 1 of 7: which run this bundle replays.
/// Field vocabulary matches the sibling paired-run oracle's
/// `ExecutionCellKey` (same program area, same identity discipline) minus
/// the fields that key a COMPARISON (`input_log_hash`, `cutpoint_profile`,
/// `run_ordinal`) rather than describe a single run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayRunManifestV1 {
    pub scenario_id: MachineTextV1,
    pub scenario_version: u32,
    pub seed_id: u64,
    pub worker_count: u32,
    pub schedule_seed: u64,
    pub platform_cell: MachineTextV1,
}

impl ManifestEncodeV1 for ReplayRunManifestV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(REPLAY_RUN_MANIFEST_SCHEMA_V1)?)),
            (1, ManifestValueV1::MachineText(self.scenario_id.clone())),
            (2, ManifestValueV1::Unsigned(self.scenario_version as u64)),
            (3, ManifestValueV1::Unsigned(self.seed_id)),
            (4, ManifestValueV1::Unsigned(self.worker_count as u64)),
            (5, ManifestValueV1::Unsigned(self.schedule_seed)),
            (6, ManifestValueV1::MachineText(self.platform_cell.clone())),
        ])
    }
}
impl ManifestDecodeV1 for ReplayRunManifestV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        if take_text(f.take_required(FieldIdV1::new(0))?)?.as_str() != REPLAY_RUN_MANIFEST_SCHEMA_V1 {
            return Err(err("wrong run-manifest schema tag"));
        }
        let out = Self {
            scenario_id: take_text(f.take_required(FieldIdV1::new(1))?)?,
            scenario_version: u32::try_from(take_unsigned(f.take_required(FieldIdV1::new(2))?)?)
                .map_err(|_| err("scenario_version out of range"))?,
            seed_id: take_unsigned(f.take_required(FieldIdV1::new(3))?)?,
            worker_count: u32::try_from(take_unsigned(f.take_required(FieldIdV1::new(4))?)?)
                .map_err(|_| err("worker_count out of range"))?,
            schedule_seed: take_unsigned(f.take_required(FieldIdV1::new(5))?)?,
            platform_cell: take_text(f.take_required(FieldIdV1::new(6))?)?,
        };
        f.finish_no_unknown()?;
        if out.worker_count == 0 {
            return Err(err("worker_count must be at least 1"));
        }
        Ok(out)
    }
}

/// Which source accepted a command — the "player/god/scenario" vocabulary
/// the row names. `PlayerAdmitted` carries the accepting session's
/// character id (u64: `CharacterId(i64)`'s non-negative range) so two
/// players issuing byte-identical commands remain distinguishable evidence.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum CommandSourceV1 {
    PlayerAdmitted(u64),
    God,
    Scenario,
}

/// One accepted command in canonical replay order. `command_digest` is the
/// EXACT-byte identity (never a protocol root) of the command's own
/// serialized wire bytes — a command is a leaf artifact here, not a typed
/// object this schema re-derives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalCommandEntryV1 {
    pub tick: u64,
    /// Tiebreak for same-tick commands — the order they were ADMITTED into
    /// the authoritative log, never wall-clock or arrival order.
    pub sequence_in_tick: u32,
    pub source: CommandSourceV1,
    pub command_digest: ArtifactIdentityV1,
}

impl ManifestEncodeV1 for CanonicalCommandEntryV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let (source_tag, player_char_id) = match self.source {
            CommandSourceV1::PlayerAdmitted(id) => (0u64, Some(id)),
            CommandSourceV1::God => (1, None),
            CommandSourceV1::Scenario => (2, None),
        };
        let mut entries = vec![
            (0, ManifestValueV1::Unsigned(self.tick)),
            (1, ManifestValueV1::Unsigned(self.sequence_in_tick as u64)),
            (2, ManifestValueV1::Unsigned(source_tag)),
            (4, self.command_digest.to_manifest_value_v1()?),
        ];
        if let Some(id) = player_char_id {
            entries.push((3, ManifestValueV1::Unsigned(id)));
        }
        map_value(entries)
    }
}
impl ManifestDecodeV1 for CanonicalCommandEntryV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        let tick = take_unsigned(f.take_required(FieldIdV1::new(0))?)?;
        let sequence_in_tick = u32::try_from(take_unsigned(f.take_required(FieldIdV1::new(1))?)?)
            .map_err(|_| err("sequence_in_tick out of range"))?;
        let source_tag = take_unsigned(f.take_required(FieldIdV1::new(2))?)?;
        let player_char_id = f.take_optional(FieldIdV1::new(3))?.map(take_unsigned).transpose()?;
        let command_digest = ArtifactIdentityV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(4))?)?;
        f.finish_no_unknown()?;
        let source = match (source_tag, player_char_id) {
            (0, Some(id)) => CommandSourceV1::PlayerAdmitted(id),
            (0, None) => return Err(err("PlayerAdmitted requires a character id")),
            (1, None) => CommandSourceV1::God,
            (2, None) => CommandSourceV1::Scenario,
            (1 | 2, Some(_)) => return Err(err("God/Scenario must not carry a character id")),
            _ => return Err(err("unknown command source tag")),
        };
        Ok(Self { tick, sequence_in_tick, source, command_digest })
    }
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
    /// V1 admits exactly the outcomes a schema-only bundle can honestly
    /// claim — no live replay engine exists yet to PRODUCE `ReplayVerified`.
    ReplayBundleTerminalV1 {
        BundleAssembled = 0,
        ReplayVerified = 1,
        FirstDivergenceLocalized = 2,
        IdentityMismatch = 3,
    }
}

/// Builder-ready row binding: `ReplayBundleV1=(run_manifest,
/// build_manifest_digest, content_manifest_digest, start_checkpoint_digest,
/// canonical_command_log, expected_terminal_tick, domain_tape_digests)`.
/// Each `_digest` field is domain-BOUND (its `ProtocolDigestV1.domain` is
/// checked at decode, not just its bytes) — a build-manifest digest wired
/// into the content-manifest slot is a decode-time rejection, not a latent
/// mix-up. `canonical_command_log` sorts by `(tick, sequence_in_tick)`;
/// `domain_tape_digests` sorts by `ProtocolDigestV1`'s own `Ord` with no
/// duplicate domain. Both orderings and every digest-domain binding are
/// enforced on decode — "bundle replay must be self-contained and reject
/// any identity mismatch" is a structural-admission property, not a
/// separate validation pass a caller could forget to run.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayBundleV1 {
    pub bundle_id: MachineTextV1,
    pub run_manifest: ReplayRunManifestV1,
    pub build_manifest_digest: ProtocolDigestV1,
    pub content_manifest_digest: ProtocolDigestV1,
    pub start_checkpoint_digest: ProtocolDigestV1,
    pub canonical_command_log: Vec<CanonicalCommandEntryV1>,
    pub expected_terminal_tick: u64,
    pub domain_tape_digests: Vec<ProtocolDigestV1>,
    pub terminal: ReplayBundleTerminalV1,
}

impl ReplayBundleV1 {
    pub fn canonical_root(&self) -> Result<ProtocolDigestV1, DigestErrorV1> {
        digest_manifest_value_v1(DigestDomainIdV1::ReplayBundle, self, &replay_bundle_limits_v1())
    }
}

impl ManifestEncodeV1 for ReplayBundleV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(REPLAY_BUNDLE_SCHEMA_V1)?)),
            (1, ManifestValueV1::MachineText(self.bundle_id.clone())),
            (2, self.run_manifest.to_manifest_value_v1()?),
            (3, self.build_manifest_digest.to_manifest_value_v1()?),
            (4, self.content_manifest_digest.to_manifest_value_v1()?),
            (5, self.start_checkpoint_digest.to_manifest_value_v1()?),
            (6, ManifestValueV1::Array(
                self.canonical_command_log.iter().map(|c| c.to_manifest_value_v1()).collect::<Result<_, _>>()?,
            )),
            (7, ManifestValueV1::Unsigned(self.expected_terminal_tick)),
            (8, ManifestValueV1::Array(
                self.domain_tape_digests.iter().map(|d| d.to_manifest_value_v1()).collect::<Result<_, _>>()?,
            )),
            (9, ManifestValueV1::Unsigned(self.terminal.as_u16() as u64)),
        ])
    }
}
impl ManifestDecodeV1 for ReplayBundleV1 {
    fn from_manifest_value_v1(v: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut f = take_map(v)?;
        if take_text(f.take_required(FieldIdV1::new(0))?)?.as_str() != REPLAY_BUNDLE_SCHEMA_V1 {
            return Err(err("wrong bundle schema tag"));
        }
        let bundle_id = take_text(f.take_required(FieldIdV1::new(1))?)?;
        let run_manifest = ReplayRunManifestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(2))?)?;
        let build_manifest_digest =
            ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(3))?)?;
        let content_manifest_digest =
            ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(4))?)?;
        let start_checkpoint_digest =
            ProtocolDigestV1::from_manifest_value_v1(f.take_required(FieldIdV1::new(5))?)?;
        let canonical_command_log = take_array(f.take_required(FieldIdV1::new(6))?)?
            .into_iter()
            .map(CanonicalCommandEntryV1::from_manifest_value_v1)
            .collect::<Result<Vec<_>, _>>()?;
        let expected_terminal_tick = take_unsigned(f.take_required(FieldIdV1::new(7))?)?;
        let domain_tape_digests = take_array(f.take_required(FieldIdV1::new(8))?)?
            .into_iter()
            .map(ProtocolDigestV1::from_manifest_value_v1)
            .collect::<Result<Vec<_>, _>>()?;
        let terminal =
            ReplayBundleTerminalV1::try_from_u16(u16::try_from(take_unsigned(f.take_required(FieldIdV1::new(9))?)?)
                .map_err(|_| err("terminal discriminant out of range"))?)?;
        f.finish_no_unknown()?;

        // Structural admission — the row's "reject any identity mismatch".
        if build_manifest_digest.domain != DigestDomainIdV1::BuildManifest {
            return Err(err("build_manifest_digest is not domain-bound to BuildManifest"));
        }
        if content_manifest_digest.domain != DigestDomainIdV1::SemanticContent {
            return Err(err("content_manifest_digest is not domain-bound to SemanticContent"));
        }
        if start_checkpoint_digest.domain != DigestDomainIdV1::SaveUniverseManifest {
            return Err(err("start_checkpoint_digest is not domain-bound to SaveUniverseManifest"));
        }
        let mut prev_key: Option<(u64, u32)> = None;
        for c in &canonical_command_log {
            let key = (c.tick, c.sequence_in_tick);
            if let Some(p) = prev_key
                && p >= key
            {
                return Err(err("canonical_command_log not strictly ordered by (tick, sequence_in_tick)"));
            }
            if c.tick > expected_terminal_tick {
                return Err(err("a command's tick exceeds expected_terminal_tick"));
            }
            prev_key = Some(key);
        }
        let mut prev_tape: Option<&ProtocolDigestV1> = None;
        let mut seen_domains: Vec<DigestDomainIdV1> = Vec::with_capacity(domain_tape_digests.len());
        for d in &domain_tape_digests {
            if let Some(p) = prev_tape
                && p >= d
            {
                return Err(err("domain_tape_digests not strictly ordered"));
            }
            if seen_domains.contains(&d.domain) {
                return Err(err("duplicate domain in domain_tape_digests"));
            }
            seen_domains.push(d.domain);
            prev_tape = Some(d);
        }
        Ok(Self {
            bundle_id,
            run_manifest,
            build_manifest_digest,
            content_manifest_digest,
            start_checkpoint_digest,
            canonical_command_log,
            expected_terminal_tick,
            domain_tape_digests,
            terminal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::{DigestAlgorithmIdV1, DigestBytes32V1, hash_artifact_bytes_v1};
    use crate::apex::manifest::{decode_manifest_v1, encode_manifest_v1};

    fn text(s: &str) -> MachineTextV1 { MachineTextV1::new(s).unwrap() }

    fn proto(domain: DigestDomainIdV1, seed: u8) -> ProtocolDigestV1 {
        ProtocolDigestV1 {
            algorithm: DigestAlgorithmIdV1::Sha256,
            domain,
            bytes: DigestBytes32V1::from_array([seed; 32]),
        }
    }

    fn run_manifest() -> ReplayRunManifestV1 {
        ReplayRunManifestV1 {
            scenario_id: text("bastion-net-envelope-scenario"),
            scenario_version: 1,
            seed_id: 42,
            worker_count: 1,
            schedule_seed: 7,
            platform_cell: text("x86_64-linux"),
        }
    }

    fn command(tick: u64, seq: u32, source: CommandSourceV1) -> CanonicalCommandEntryV1 {
        CanonicalCommandEntryV1 {
            tick,
            sequence_in_tick: seq,
            source,
            command_digest: hash_artifact_bytes_v1(format!("{tick}-{seq}").as_bytes()),
        }
    }

    fn bundle() -> ReplayBundleV1 {
        ReplayBundleV1 {
            bundle_id: text("bundle-0001"),
            run_manifest: run_manifest(),
            build_manifest_digest: proto(DigestDomainIdV1::BuildManifest, 1),
            content_manifest_digest: proto(DigestDomainIdV1::SemanticContent, 2),
            start_checkpoint_digest: proto(DigestDomainIdV1::SaveUniverseManifest, 3),
            canonical_command_log: vec![
                command(0, 0, CommandSourceV1::Scenario),
                command(5, 0, CommandSourceV1::PlayerAdmitted(11)),
                command(5, 1, CommandSourceV1::God),
            ],
            expected_terminal_tick: 100,
            domain_tape_digests: vec![
                proto(DigestDomainIdV1::ExecutionEvidence, 10),
                proto(DigestDomainIdV1::SemanticContent, 20),
            ],
            terminal: ReplayBundleTerminalV1::BundleAssembled,
        }
    }

    #[test]
    fn bundle_round_trips_canonically() {
        let b = bundle();
        let limits = replay_bundle_limits_v1();
        let bytes = encode_manifest_v1(&b, &limits).unwrap();
        let decoded: ReplayBundleV1 = decode_manifest_v1(&bytes, &limits).unwrap();
        assert_eq!(decoded, b);
        assert_eq!(encode_manifest_v1(&decoded, &limits).unwrap(), bytes);
        assert!(b.canonical_root().is_ok());
    }

    /// The row's "reject any identity mismatch": each digest-domain
    /// binding, command-log ordering, tape ordering/uniqueness, and the
    /// terminal-tick bound are all rejected independently at decode.
    #[test]
    fn identity_mismatches_are_rejected() {
        let limits = replay_bundle_limits_v1();
        let reject = |mutate: fn(&mut ReplayBundleV1)| {
            let mut b = bundle();
            mutate(&mut b);
            let bytes = encode_manifest_v1(&b, &limits).unwrap();
            assert!(
                decode_manifest_v1::<ReplayBundleV1>(&bytes, &limits).is_err(),
                "expected rejection"
            );
        };

        reject(|b| b.build_manifest_digest.domain = DigestDomainIdV1::SemanticContent);
        reject(|b| b.content_manifest_digest.domain = DigestDomainIdV1::BuildManifest);
        reject(|b| b.start_checkpoint_digest.domain = DigestDomainIdV1::BuildManifest);
        reject(|b| b.canonical_command_log.swap(0, 1)); // now out of order
        reject(|b| b.canonical_command_log.push(command(0, 0, CommandSourceV1::Scenario))); // duplicate key
        reject(|b| b.expected_terminal_tick = 4); // a command (tick 5) exceeds it
        reject(|b| b.domain_tape_digests.reverse()); // now out of order
        reject(|b| b.domain_tape_digests.push(proto(DigestDomainIdV1::ExecutionEvidence, 99))); // dup domain

        // A genuinely honest bundle still decodes.
        let bytes = encode_manifest_v1(&bundle(), &limits).unwrap();
        assert!(decode_manifest_v1::<ReplayBundleV1>(&bytes, &limits).is_ok());
    }

    #[test]
    fn command_source_wire_admission() {
        let limits = replay_bundle_limits_v1();
        // PlayerAdmitted requires the character id field.
        let mut b = bundle();
        b.canonical_command_log = vec![command(0, 0, CommandSourceV1::PlayerAdmitted(9))];
        let bytes = encode_manifest_v1(&b, &limits).unwrap();
        let decoded: ReplayBundleV1 = decode_manifest_v1(&bytes, &limits).unwrap();
        assert_eq!(decoded.canonical_command_log[0].source, CommandSourceV1::PlayerAdmitted(9));
    }

    #[test]
    fn sealed_terminal_fails_closed() {
        assert!(ReplayBundleTerminalV1::try_from_u16(4).is_err());
        assert_eq!(ReplayBundleTerminalV1::ALL.len(), 4);
    }
}
