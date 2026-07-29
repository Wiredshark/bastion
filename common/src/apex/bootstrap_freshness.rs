//! `APEX-T4.2` — bootstrap freshness: an authentic but *stale* bootstrap
//! manifest must not be applicable.
//!
//! **The gap this closes.** `T4.1`'s `check_game_sync_boot_scope` rejects
//! cross-restart mixing (`ServerBootId`), which is one axis. Nothing binds
//! a manifest to a *sequence* within one boot, so a replayed authentic
//! manifest from earlier in the same boot is indistinguishable from a
//! current one.
//!
//! **This module reuses `T3.5`'s monotone-sequence-with-floor PATTERN,
//! not its literal type.** `CommandJournalV1` (`common/net/src/msg/
//! command.rs`) tracks a `binding` (rebindable on resume) separate from a
//! `retired_floor`, and rejects a sequence at-or-below the floor as a
//! replay, an epoch below the binding's as a superseded attachment. The
//! failure mode here is identical (an authentic-but-superseded artifact
//! replaying), so [`BootstrapFreshnessLedgerV1`] is shaped the same way —
//! a rebindable epoch plus a floor — deliberately NOT the full command
//! journal (no in-flight/terminal/capacity machinery: a bootstrap manifest
//! is not a repeated, acked exchange, it is admitted once per connection
//! attempt).
//!
//! **Chunk scope, self-sized per this program's own standing discipline**
//! (`T4.1` chunk 1's own precedent: "landing it first and self-sizing the
//! wire-ordering work separately"). This chunk lands the DATA MODEL — the
//! typed freshness tuple, wired into [`crate::apex::bootstrap_manifest::
//! BootstrapManifestV1`]'s reserved slot — and the PURE ledger mechanism,
//! fully testable against fixtures alone. Deliberately NOT built here,
//! banked for a follow-up chunk: the live nonce/transcript handshake
//! binding (so possession of a recorded manifest alone is insufficient —
//! today's ledger only proves *ordering*, not liveness), the declared-
//! reset escape hatch for the predecessor-fork rejection (today ANY fork
//! rejects, unconditionally — correct but not yet permissive), the real
//! per-boot sequence counter + root chain on the server, and real
//! cross-reconnect floor persistence on the client (today's ledger is a
//! plain struct with no attached lifetime scope; wiring it into `Client`/
//! `Server` state is the follow-up's job, same split as `T4.1`'s own
//! carrier-then-wiring chunks).

use crate::apex::digest::ArtifactDigestV1;
use crate::apex::identity::{ConnectionEpoch, ServerBootId, SessionId, SnapshotEpoch};
use crate::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1, ManifestDecodeV1, ManifestEncodeV1,
    ManifestErrorV1, ManifestSchemaErrorV1, ManifestValueV1, StructFieldsV1,
};

/// The freshness/sequence-binding tuple `T4.1`'s `freshness_reserved`
/// field was reserved for. `sequence` is a plain `u64` (not a dedicated
/// counter newtype), matching `CommandJournalV1`'s own choice — the
/// validity/ordering rules live in the ledger below, not the type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootstrapFreshnessV1 {
    pub server_boot_id: ServerBootId,
    pub session_id: SessionId,
    pub epoch: ConnectionEpoch,
    /// Monotone WITHIN THE BOOT — one global counter spanning every
    /// session's manifests this boot has issued, not a per-session
    /// counter. Zero is invalid (mirrors `CommandJournalV1::SequenceZero`).
    pub sequence: u64,
    pub snapshot_epoch: SnapshotEpoch,
    /// The digest of the immediately preceding manifest issued this boot
    /// (across every session) — `None` only for the very first manifest
    /// a boot ever issues. Chains every manifest into one global sequence
    /// so a fork (a manifest that doesn't extend what the ledger already
    /// admitted) is detectable even if its own sequence number looks
    /// plausible in isolation.
    pub predecessor_root: Option<ArtifactDigestV1>,
}

fn encode_optional_root(root: &Option<ArtifactDigestV1>) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    match root {
        Some(r) => Ok(ManifestValueV1::Array(vec![r.to_manifest_value_v1()?])),
        None => Ok(ManifestValueV1::Array(Vec::new())),
    }
}

fn decode_optional_root(value: ManifestValueV1) -> Result<Option<ArtifactDigestV1>, ManifestSchemaErrorV1> {
    let ManifestValueV1::Array(items) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
    match <[ManifestValueV1; 1]>::try_from(items) {
        Ok([only]) => Ok(Some(ArtifactDigestV1::from_manifest_value_v1(only)?)),
        Err(items) if items.is_empty() => Ok(None),
        Err(_) => Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("optional predecessor root array must have 0 or 1 elements")),
    }
}

impl ManifestEncodeV1 for BootstrapFreshnessV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), self.server_boot_id.to_manifest_value_v1()?),
            (FieldIdV1::new(2), self.session_id.to_manifest_value_v1()?),
            (FieldIdV1::new(3), self.epoch.to_manifest_value_v1()?),
            (FieldIdV1::new(4), ManifestValueV1::Unsigned(self.sequence)),
            (FieldIdV1::new(5), self.snapshot_epoch.to_manifest_value_v1()?),
            (FieldIdV1::new(6), encode_optional_root(&self.predecessor_root)?),
        ])?;
        Ok(ManifestValueV1::Map(map))
    }
}

impl ManifestDecodeV1 for BootstrapFreshnessV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let ManifestValueV1::Map(map) = value else { return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType)) };
        let mut fields = StructFieldsV1::new(map);
        let server_boot_id = ServerBootId::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let session_id = SessionId::from_manifest_value_v1(fields.take_required(FieldIdV1::new(2))?)?;
        let epoch = ConnectionEpoch::from_manifest_value_v1(fields.take_required(FieldIdV1::new(3))?)?;
        let sequence = match fields.take_required(FieldIdV1::new(4))? {
            ManifestValueV1::Unsigned(v) => v,
            _ => return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::FieldKeyType).detail("sequence must be an unsigned integer")),
        };
        let snapshot_epoch = SnapshotEpoch::from_manifest_value_v1(fields.take_required(FieldIdV1::new(5))?)?;
        let predecessor_root = decode_optional_root(fields.take_required(FieldIdV1::new(6))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { server_boot_id, session_id, epoch, sequence, snapshot_epoch, predecessor_root })
    }
}

/// Why a candidate freshness tuple was refused — each a distinct typed
/// terminal per this row's own spec ("collapsing them into one 'invalid'
/// loses the diagnosis").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapFreshnessRejectionV1 {
    /// Sequence zero is not a sequence (mirrors `JournalErrorV1::SequenceZero`).
    SequenceZero,
    /// A different `(ServerBootId, SessionId)` than the ledger tracks —
    /// not a freshness question, a different lineage entirely.
    ForeignLineage,
    /// The candidate's own epoch is behind the ledger's current (possibly
    /// resume-advanced) epoch — a stale manifest replayed after a
    /// legitimate reconnect moved the ledger forward ("freeze").
    Freeze { ledger_epoch: ConnectionEpoch, candidate_epoch: ConnectionEpoch },
    /// Sequence at or below the floor — a replay of an earlier-in-boot
    /// manifest ("rollback").
    Rollback { floor: u64, candidate: u64 },
    /// The candidate's `predecessor_root` doesn't chain from the floor's
    /// own root — an undeclared fork ("mix-and-match roots"). Today this
    /// is unconditional; the declared-reset escape hatch is banked for
    /// the follow-up chunk alongside the live handshake binding.
    PredecessorFork,
}

/// The floor: the last admitted freshness tuple's `(sequence, own content
/// root)`. `None` before the very first admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FreshnessFloorV1 {
    sequence: u64,
    root: ArtifactDigestV1,
}

/// `T4.2`'s ledger: a rebindable current epoch plus a floor, mirroring
/// `CommandJournalV1`'s own `binding`/`retired_floor` split. Pure and
/// in-process — see the module doc for what real persistence this chunk
/// deliberately does not yet wire up.
#[derive(Clone, Copy, Debug)]
pub struct BootstrapFreshnessLedgerV1 {
    server_boot_id: ServerBootId,
    session_id: SessionId,
    epoch: ConnectionEpoch,
    floor: Option<FreshnessFloorV1>,
}

impl BootstrapFreshnessLedgerV1 {
    pub fn new(server_boot_id: ServerBootId, session_id: SessionId, epoch: ConnectionEpoch) -> Self {
        Self { server_boot_id, session_id, epoch, floor: None }
    }

    pub fn current_epoch(&self) -> ConnectionEpoch { self.epoch }

    pub fn floor_sequence(&self) -> Option<u64> { self.floor.map(|f| f.sequence) }

    /// A resume advances the ledger's own tracked epoch, exactly
    /// `CommandJournalV1::rebind_epoch_v1`'s shape: an epoch behind the
    /// current one cannot rebind (a superseded reconnect attempt), the
    /// floor is untouched either way.
    pub fn rebind_epoch_v1(&mut self, epoch: ConnectionEpoch) -> Result<(), BootstrapFreshnessRejectionV1> {
        if epoch.get() < self.epoch.get() {
            return Err(BootstrapFreshnessRejectionV1::Freeze { ledger_epoch: self.epoch, candidate_epoch: epoch });
        }
        self.epoch = epoch;
        Ok(())
    }

    /// Classifies and, if admitted, advances the floor. `candidate_root`
    /// is the caller-computed content root of `candidate`'s OWN manifest
    /// (this ledger has no opinion on how that root is computed — see
    /// `hash_artifact_bytes_v1` for the existing project convention).
    pub fn admit_v1(
        &mut self,
        candidate: BootstrapFreshnessV1,
        candidate_root: ArtifactDigestV1,
    ) -> Result<(), BootstrapFreshnessRejectionV1> {
        if candidate.sequence == 0 {
            return Err(BootstrapFreshnessRejectionV1::SequenceZero);
        }
        if candidate.server_boot_id != self.server_boot_id || candidate.session_id != self.session_id {
            return Err(BootstrapFreshnessRejectionV1::ForeignLineage);
        }
        if candidate.epoch.get() < self.epoch.get() {
            return Err(BootstrapFreshnessRejectionV1::Freeze { ledger_epoch: self.epoch, candidate_epoch: candidate.epoch });
        }
        if let Some(floor) = self.floor {
            if candidate.sequence <= floor.sequence {
                return Err(BootstrapFreshnessRejectionV1::Rollback { floor: floor.sequence, candidate: candidate.sequence });
            }
            if candidate.predecessor_root != Some(floor.root) {
                return Err(BootstrapFreshnessRejectionV1::PredecessorFork);
            }
        }
        self.floor = Some(FreshnessFloorV1 { sequence: candidate.sequence, root: candidate_root });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::hash_artifact_bytes_v1;
    use crate::apex::identity::FixedRandomBytesSourceV1;
    use crate::apex::manifest::{ManifestDecodeLimitsV1, decode_manifest_v1, encode_manifest_v1};

    fn limits() -> ManifestDecodeLimitsV1 {
        ManifestDecodeLimitsV1 {
            max_input_bytes: 4096,
            max_depth: 8,
            max_nodes: 256,
            max_array_items: 32,
            max_map_entries: 32,
            max_machine_text_bytes: 128,
            max_byte_string_bytes: 128,
        }
    }

    fn boot(seed: u8) -> ServerBootId { ServerBootId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap() }
    fn session(seed: u8) -> SessionId { SessionId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap() }
    fn root(tag: u8) -> ArtifactDigestV1 { hash_artifact_bytes_v1(&[tag]).digest }

    fn tuple(
        server_boot_id: ServerBootId,
        session_id: SessionId,
        epoch: u64,
        sequence: u64,
        predecessor_root: Option<ArtifactDigestV1>,
    ) -> BootstrapFreshnessV1 {
        BootstrapFreshnessV1 {
            server_boot_id,
            session_id,
            epoch: ConnectionEpoch::new(epoch).unwrap(),
            sequence,
            snapshot_epoch: SnapshotEpoch::new(0),
            predecessor_root,
        }
    }

    #[test]
    fn round_trips_through_the_manifest_codec() {
        let original = tuple(boot(1), session(2), 1, 5, Some(root(9)));
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: BootstrapFreshnessV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trips_with_no_predecessor_root() {
        let original = tuple(boot(3), session(4), 1, 1, None);
        let bytes = encode_manifest_v1(&original, &limits()).unwrap();
        let decoded: BootstrapFreshnessV1 = decode_manifest_v1(&bytes, &limits()).unwrap();
        assert_eq!(decoded, original);
    }

    /// The very first manifest a ledger ever sees always admits, with no
    /// floor to compare against and no predecessor root required.
    #[test]
    fn the_first_manifest_admits_unconditionally() {
        let b = boot(10);
        let s = session(11);
        let mut ledger = BootstrapFreshnessLedgerV1::new(b, s, ConnectionEpoch::new(1).unwrap());
        assert_eq!(ledger.floor_sequence(), None);
        assert!(ledger.admit_v1(tuple(b, s, 1, 1, None), root(1)).is_ok());
        assert_eq!(ledger.floor_sequence(), Some(1));
    }

    /// `T4.2`'s "rollback" required test: a replayed EARLIER sequence is
    /// refused, distinctly, after a fresher one has already been admitted.
    #[test]
    fn rollback_replaying_an_earlier_valid_manifest_is_refused() {
        let b = boot(20);
        let s = session(21);
        let mut ledger = BootstrapFreshnessLedgerV1::new(b, s, ConnectionEpoch::new(1).unwrap());
        let r1 = root(1);
        ledger.admit_v1(tuple(b, s, 1, 5, None), r1).unwrap();
        ledger.admit_v1(tuple(b, s, 1, 9, Some(r1)), root(2)).unwrap();

        // Replay the FIRST manifest (sequence 5) again -- authentic bytes,
        // but stale relative to the floor (now 9).
        let replay = tuple(b, s, 1, 5, None);
        assert_eq!(
            ledger.admit_v1(replay, r1).unwrap_err(),
            BootstrapFreshnessRejectionV1::Rollback { floor: 9, candidate: 5 }
        );
        // A duplicate of the floor itself (sequence == floor) is also a
        // rollback, not a silent no-op accept.
        let replay_current = tuple(b, s, 1, 9, Some(r1));
        assert_eq!(
            ledger.admit_v1(replay_current, root(2)).unwrap_err(),
            BootstrapFreshnessRejectionV1::Rollback { floor: 9, candidate: 9 }
        );
    }

    /// `T4.2`'s "freeze" required test: replaying the CURRENT manifest
    /// after the ledger's own epoch has advanced (a legitimate resume)
    /// is refused as `Freeze`, a distinct terminal from `Rollback` even
    /// though the sequence itself doesn't regress.
    #[test]
    fn freeze_replaying_the_current_manifest_after_the_epoch_advances_is_refused() {
        let b = boot(30);
        let s = session(31);
        let mut ledger = BootstrapFreshnessLedgerV1::new(b, s, ConnectionEpoch::new(1).unwrap());
        let admitted = tuple(b, s, 1, 3, None);
        ledger.admit_v1(admitted, root(1)).unwrap();

        // A legitimate resume moves the ledger's epoch forward.
        ledger.rebind_epoch_v1(ConnectionEpoch::new(2).unwrap()).unwrap();
        assert_eq!(ledger.current_epoch(), ConnectionEpoch::new(2).unwrap());

        // The exact same (authentic) manifest, still carrying epoch 1,
        // replayed after the epoch moved on.
        assert_eq!(
            ledger.admit_v1(admitted, root(1)).unwrap_err(),
            BootstrapFreshnessRejectionV1::Freeze { ledger_epoch: ConnectionEpoch::new(2).unwrap(), candidate_epoch: ConnectionEpoch::new(1).unwrap() }
        );
    }

    /// `rebind_epoch_v1` itself must reject a stale rebind attempt (a
    /// superseded reconnect), same shape as `CommandJournalV1::
    /// SupersededAttachment`.
    #[test]
    fn rebind_epoch_rejects_a_stale_epoch() {
        let mut ledger = BootstrapFreshnessLedgerV1::new(boot(40), session(41), ConnectionEpoch::new(3).unwrap());
        assert_eq!(
            ledger.rebind_epoch_v1(ConnectionEpoch::new(2).unwrap()).unwrap_err(),
            BootstrapFreshnessRejectionV1::Freeze { ledger_epoch: ConnectionEpoch::new(3).unwrap(), candidate_epoch: ConnectionEpoch::new(2).unwrap() }
        );
        assert_eq!(ledger.current_epoch(), ConnectionEpoch::new(3).unwrap(), "a rejected rebind must not move the epoch");
    }

    /// `T4.2`'s "mix-and-match roots" required test: a manifest with a
    /// genuinely higher sequence but a FOREIGN predecessor root (doesn't
    /// chain from the floor) is refused as a fork, distinctly from a
    /// simple sequence rollback.
    #[test]
    fn mix_and_match_a_valid_manifest_with_a_foreign_predecessor_is_refused() {
        let b = boot(50);
        let s = session(51);
        let mut ledger = BootstrapFreshnessLedgerV1::new(b, s, ConnectionEpoch::new(1).unwrap());
        ledger.admit_v1(tuple(b, s, 1, 1, None), root(1)).unwrap();

        // Sequence 2 genuinely advances the floor, but claims a
        // predecessor root that is NOT the floor's own root (root(99)
        // instead of root(1)) -- a fork, not a legitimate continuation.
        let forked = tuple(b, s, 1, 2, Some(root(99)));
        assert_eq!(ledger.admit_v1(forked, root(2)).unwrap_err(), BootstrapFreshnessRejectionV1::PredecessorFork);
    }

    /// A tuple naming a different `(ServerBootId, SessionId)` than the
    /// ledger tracks is refused as `ForeignLineage`, never silently
    /// compared against an unrelated floor.
    #[test]
    fn a_foreign_lineage_is_refused_not_compared_against_the_floor() {
        let mut ledger = BootstrapFreshnessLedgerV1::new(boot(60), session(61), ConnectionEpoch::new(1).unwrap());
        let foreign = tuple(boot(62), session(63), 1, 1, None);
        assert_eq!(ledger.admit_v1(foreign, root(1)).unwrap_err(), BootstrapFreshnessRejectionV1::ForeignLineage);
    }

    #[test]
    fn sequence_zero_is_refused_even_as_the_first_admission() {
        let mut ledger = BootstrapFreshnessLedgerV1::new(boot(70), session(71), ConnectionEpoch::new(1).unwrap());
        let zero = tuple(boot(70), session(71), 1, 0, None);
        assert_eq!(ledger.admit_v1(zero, root(1)).unwrap_err(), BootstrapFreshnessRejectionV1::SequenceZero);
    }

    /// Non-vacuity / positive control: a genuinely fresh manifest that
    /// correctly chains from the floor's own root is admitted.
    #[test]
    fn a_genuinely_fresh_correctly_chained_manifest_is_admitted() {
        let b = boot(80);
        let s = session(81);
        let mut ledger = BootstrapFreshnessLedgerV1::new(b, s, ConnectionEpoch::new(1).unwrap());
        let r1 = root(1);
        ledger.admit_v1(tuple(b, s, 1, 1, None), r1).unwrap();
        assert!(ledger.admit_v1(tuple(b, s, 1, 2, Some(r1)), root(2)).is_ok());
        assert_eq!(ledger.floor_sequence(), Some(2));
    }

    /// Non-vacuity check: `root(1)` and `root(2)` (the fixture helper used
    /// throughout this module) must actually differ, or every
    /// `PredecessorFork` assertion above would pass by coincidence rather
    /// than because the comparison is real.
    #[test]
    fn the_root_fixture_helper_produces_genuinely_distinct_digests() {
        assert_ne!(root(1), root(2));
        assert_ne!(root(1), root(99));
    }
}
