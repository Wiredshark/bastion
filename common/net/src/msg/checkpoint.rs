//! `APEX-T3.4` — cross-stream checkpoint watermarks: participation
//! classification, global chronology, apply phases.

use super::envelope::SemanticStreamIdV1;
use super::{ClientGeneral, ServerGeneral, ServerInit};
use serde::{Deserialize, Serialize};

/// Every checkpoint requires all five streams; an empty stream is fenced
/// by its own Begin/Barrier rather than omitted.
pub const REQUIRED_CHECKPOINT_STREAMS_V1: [SemanticStreamIdV1; 5] = [
    SemanticStreamIdV1::Bootstrap,
    SemanticStreamIdV1::CharacterScreen,
    SemanticStreamIdV1::InGame,
    SemanticStreamIdV1::General,
    SemanticStreamIdV1::Terrain,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CheckpointParticipationV1 {
    CheckpointedData,
    CheckpointControl,
    OutOfBandDiagnostic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CheckpointApplyPhaseV1 {
    IdentityLifecycle = 1,
    Components = 2,
    PlayerBinding = 3,
    CharacterState = 4,
    InGameState = 5,
    TerrainBase = 6,
    TerrainDelta = 7,
    OrderedEvent = 8,
}

impl CheckpointApplyPhaseV1 {
    pub const ALL: [Self; 8] = [
        Self::IdentityLifecycle,
        Self::Components,
        Self::PlayerBinding,
        Self::CharacterState,
        Self::InGameState,
        Self::TerrainBase,
        Self::TerrainDelta,
        Self::OrderedEvent,
    ];

    pub const fn rank(self) -> u16 { self as u16 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointProfileErrorV1 {
    UnregisteredCheckpointPayload,
    InvalidRequiredStreamSet,
    AuthoritativePingForbidden,
}

/// Classification is exhaustive over both server enums: a new variant
/// fails to compile rather than defaulting to data.
pub trait CheckpointParticipantV1 {
    fn participation_v1(&self) -> CheckpointParticipationV1;
    fn apply_phase_v1(&self) -> Option<CheckpointApplyPhaseV1>;
}

impl CheckpointParticipantV1 for ServerGeneral {
    fn participation_v1(&self) -> CheckpointParticipationV1 {
        use CheckpointParticipationV1 as P;
        use ServerGeneral as S;
        match self {
            S::PlayerListUpdate(_)
            | S::ChatMode(_)
            | S::SetPlayerEntity(_)
            | S::TimeOfDay(_, _, _, _)
            | S::EntitySync(_)
            | S::CompSync(_, _)
            | S::CreateEntity(_)
            | S::DeleteEntity(_)
            | S::CharacterDataLoadResult(_)
            | S::CharacterListUpdate(_)
            | S::CharacterActionError(_)
            | S::CharacterCreated(_)
            | S::CharacterEdited(_)
            | S::CharacterSuccess
            | S::SpectatorSuccess(_)
            | S::GroupUpdate(_)
            | S::Invite { .. }
            | S::InvitePending(_)
            | S::InviteComplete { .. }
            | S::ExitInGameSuccess
            | S::InventoryUpdate(_, _)
            | S::GroupInventoryUpdate(_, _)
            | S::Dialogue(_, _)
            | S::SetViewDistance(_)
            | S::Outcomes(_)
            | S::Knockback(_)
            | S::SiteEconomy(_)
            | S::UpdatePendingTrade(_, _, _)
            | S::FinishedTrade(_)
            | S::MapMarker(_)
            | S::WeatherUpdate(_)
            | S::LocalWindUpdate(_)
            | S::SpectatePosition(_)
            | S::UpdateRecipes
            | S::Gizmos(_)
            | S::TerrainChunkUpdate { .. }
            | S::LodZoneUpdate { .. }
            | S::TerrainBlockUpdates(_)
            | S::ChatMsg(_)
            | S::Notification(_)
            | S::SetPlayerRole(_)
            | S::PluginData(_)
            | S::PluginArtifactData(_)
            | S::BastionDesignation { .. }
            | S::BastionDesignationRemoved { .. }
            | S::BastionInspectInfo { .. } => P::CheckpointedData,
            S::Disconnect(_) => P::CheckpointControl,
        }
    }

    fn apply_phase_v1(&self) -> Option<CheckpointApplyPhaseV1> {
        use CheckpointApplyPhaseV1 as Ph;
        use ServerGeneral as S;
        Some(match self {
            S::CreateEntity(_) | S::DeleteEntity(_) => Ph::IdentityLifecycle,
            S::EntitySync(_) | S::CompSync(_, _) => Ph::Components,
            S::SetPlayerEntity(_) | S::PlayerListUpdate(_) | S::SetPlayerRole(_) | S::ChatMode(_) => Ph::PlayerBinding,
            S::CharacterDataLoadResult(_)
            | S::CharacterListUpdate(_)
            | S::CharacterActionError(_)
            | S::CharacterCreated(_)
            | S::CharacterEdited(_)
            | S::CharacterSuccess
            | S::SpectatorSuccess(_) => Ph::CharacterState,
            S::TerrainChunkUpdate { .. } | S::LodZoneUpdate { .. } => Ph::TerrainBase,
            S::TerrainBlockUpdates(_) => Ph::TerrainDelta,
            S::Outcomes(_) | S::ChatMsg(_) | S::Notification(_) | S::Knockback(_) | S::Dialogue(_, _) => {
                Ph::OrderedEvent
            },
            S::Disconnect(_) => return None,
            _ => Ph::InGameState,
        })
    }
}

impl CheckpointParticipantV1 for ServerInit {
    fn participation_v1(&self) -> CheckpointParticipationV1 {
        match self {
            ServerInit::GameSync { .. } => CheckpointParticipationV1::CheckpointedData,
        }
    }

    fn apply_phase_v1(&self) -> Option<CheckpointApplyPhaseV1> {
        Some(CheckpointApplyPhaseV1::IdentityLifecycle)
    }
}

/// C2S payloads never participate: checkpoints are server-authored.
impl CheckpointParticipantV1 for ClientGeneral {
    fn participation_v1(&self) -> CheckpointParticipationV1 { CheckpointParticipationV1::OutOfBandDiagnostic }

    fn apply_phase_v1(&self) -> Option<CheckpointApplyPhaseV1> { None }
}

pub fn validate_required_stream_set_v1(streams: &[SemanticStreamIdV1]) -> Result<(), CheckpointProfileErrorV1> {
    let mut seen: Vec<u8> = streams.iter().map(|s| s.as_u8()).collect();
    seen.sort_unstable();
    seen.dedup();
    let mut want: Vec<u8> = REQUIRED_CHECKPOINT_STREAMS_V1.iter().map(|s| s.as_u8()).collect();
    want.sort_unstable();
    if seen == want { Ok(()) } else { Err(CheckpointProfileErrorV1::InvalidRequiredStreamSet) }
}

#[cfg(test)]
mod checkpoint_profile_v1 {
    use super::*;

    #[test]
    fn participation_and_phases_are_total_and_ordered() {
        // Control payloads carry no apply phase; data payloads always do.
        let disconnect = ServerGeneral::Disconnect(super::super::DisconnectReason::Shutdown);
        assert_eq!(disconnect.participation_v1(), CheckpointParticipationV1::CheckpointControl);
        assert!(disconnect.apply_phase_v1().is_none());

        let data = ServerGeneral::UpdateRecipes;
        assert_eq!(data.participation_v1(), CheckpointParticipationV1::CheckpointedData);
        assert!(data.apply_phase_v1().is_some());

        // Phase ranks are strictly increasing in declared order.
        let ranks: Vec<u16> = CheckpointApplyPhaseV1::ALL.iter().map(|p| p.rank()).collect();
        assert!(ranks.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(ranks.len(), 8);
    }

    #[test]
    fn required_stream_set_is_all_five_exactly() {
        assert!(validate_required_stream_set_v1(&REQUIRED_CHECKPOINT_STREAMS_V1).is_ok());
        assert_eq!(
            validate_required_stream_set_v1(&REQUIRED_CHECKPOINT_STREAMS_V1[..4]),
            Err(CheckpointProfileErrorV1::InvalidRequiredStreamSet)
        );
        // Duplicates collapse; a repeated stream is not a valid 5-set.
        let dup = [
            SemanticStreamIdV1::Bootstrap,
            SemanticStreamIdV1::Bootstrap,
            SemanticStreamIdV1::CharacterScreen,
            SemanticStreamIdV1::InGame,
            SemanticStreamIdV1::General,
        ];
        assert_eq!(
            validate_required_stream_set_v1(&dup),
            Err(CheckpointProfileErrorV1::InvalidRequiredStreamSet)
        );
    }
}

/// `APEX-T3.4.02` — global chronology. Epoch starts at 1 per binding
/// (0 = "no checkpoint committed"); data ordinals are dense 1..=N within
/// one epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CheckpointOrdinalV1(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointChronologyErrorV1 {
    EpochZero,
    EpochGap { expected: u64, got: u64 },
    EpochStale { committed: u64, got: u64 },
    ParentMismatch { expected: u64, got: u64 },
    OrdinalZero,
    OrdinalGap { expected: u64, got: u64 },
    OrdinalDuplicate { ordinal: u64 },
}

/// Per-binding chronology cursor. A new binding starts committed = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CheckpointChronologyV1 {
    committed_epoch: u64,
}

impl CheckpointChronologyV1 {
    pub fn new() -> Self { Self { committed_epoch: 0 } }

    pub fn committed_epoch(&self) -> u64 { self.committed_epoch }

    /// Admits exactly `committed + 1` with `parent == committed`.
    pub fn validate_epoch_v1(&self, epoch: u64, parent_epoch: u64) -> Result<(), CheckpointChronologyErrorV1> {
        use CheckpointChronologyErrorV1 as E;
        if epoch == 0 {
            return Err(E::EpochZero);
        }
        if epoch <= self.committed_epoch {
            return Err(E::EpochStale { committed: self.committed_epoch, got: epoch });
        }
        if epoch != self.committed_epoch + 1 {
            return Err(E::EpochGap { expected: self.committed_epoch + 1, got: epoch });
        }
        if parent_epoch != self.committed_epoch {
            return Err(E::ParentMismatch { expected: self.committed_epoch, got: parent_epoch });
        }
        Ok(())
    }

    /// Only a committed checkpoint advances the cursor.
    pub fn commit_epoch_v1(&mut self, epoch: u64) { self.committed_epoch = epoch; }
}

/// Validates a whole epoch's ordinal transcript: dense 1..=N, no gap, no
/// duplicate, order-independent (input may arrive in any order).
pub fn validate_ordinals_v1(ordinals: &[CheckpointOrdinalV1]) -> Result<(), CheckpointChronologyErrorV1> {
    use CheckpointChronologyErrorV1 as E;
    let mut sorted: Vec<u64> = ordinals.iter().map(|o| o.0).collect();
    sorted.sort_unstable();
    for (i, &o) in sorted.iter().enumerate() {
        if o == 0 {
            return Err(E::OrdinalZero);
        }
        if i > 0 && o == sorted[i - 1] {
            return Err(E::OrdinalDuplicate { ordinal: o });
        }
        let expected = i as u64 + 1;
        if o != expected {
            return Err(E::OrdinalGap { expected, got: o });
        }
    }
    Ok(())
}

#[cfg(test)]
mod checkpoint_epoch_ordinal_v1 {
    use super::*;

    #[test]
    fn epoch_chain_is_contiguous_and_non_reusable() {
        use CheckpointChronologyErrorV1 as E;
        let mut c = CheckpointChronologyV1::new();
        assert_eq!(c.validate_epoch_v1(0, 0), Err(E::EpochZero));
        assert_eq!(c.validate_epoch_v1(2, 0), Err(E::EpochGap { expected: 1, got: 2 }));
        assert_eq!(c.validate_epoch_v1(1, 7), Err(E::ParentMismatch { expected: 0, got: 7 }));
        c.validate_epoch_v1(1, 0).unwrap();
        c.commit_epoch_v1(1);
        // replay of a committed epoch is stale, not a gap
        assert_eq!(c.validate_epoch_v1(1, 0), Err(E::EpochStale { committed: 1, got: 1 }));
        c.validate_epoch_v1(2, 1).unwrap();
    }

    #[test]
    fn ordinals_are_dense_and_order_independent() {
        use CheckpointChronologyErrorV1 as E;
        let ord = |v: &[u64]| v.iter().map(|&x| CheckpointOrdinalV1(x)).collect::<Vec<_>>();
        assert!(validate_ordinals_v1(&ord(&[])).is_ok());
        assert!(validate_ordinals_v1(&ord(&[3, 1, 2])).is_ok());
        assert_eq!(validate_ordinals_v1(&ord(&[0, 1])), Err(E::OrdinalZero));
        assert_eq!(validate_ordinals_v1(&ord(&[1, 3])), Err(E::OrdinalGap { expected: 2, got: 3 }));
        assert_eq!(validate_ordinals_v1(&ord(&[1, 2, 2])), Err(E::OrdinalDuplicate { ordinal: 2 }));
    }
}

/// `APEX-T3.4.03` — apply-order policy. Ordinals are assigned by the
/// egress total sort, then applied in phase order; the policy is
/// equality-critical, so its root is bound into the descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointOrderErrorV1 {
    DuplicateOrderKey,
    UnknownPhase,
    PhaseRegression { previous: u16, got: u16 },
    PolicyMismatch,
}

/// One data record as the planner sees it: its egress sort key decides
/// the ordinal, its phase decides when it applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointRecordV1 {
    pub egress_sort_key: Vec<u8>,
    pub stream: SemanticStreamIdV1,
    pub phase: CheckpointApplyPhaseV1,
    pub payload_digest: [u8; 32],
}

/// Assigns dense ordinals by egress sort key. Input order is irrelevant;
/// duplicate keys are a producer bug, not a tie to break.
pub fn assign_ordinals_v1(
    mut records: Vec<CheckpointRecordV1>,
) -> Result<Vec<(CheckpointOrdinalV1, CheckpointRecordV1)>, CheckpointOrderErrorV1> {
    records.sort_by(|a, b| a.egress_sort_key.cmp(&b.egress_sort_key));
    if records.windows(2).any(|w| w[0].egress_sort_key == w[1].egress_sort_key) {
        return Err(CheckpointOrderErrorV1::DuplicateOrderKey);
    }
    Ok(records
        .into_iter()
        .enumerate()
        .map(|(i, r)| (CheckpointOrdinalV1(i as u64 + 1), r))
        .collect())
}

/// Apply order = phase rank, then ordinal. Verifies a proposed apply
/// sequence never regresses a phase.
pub fn validate_apply_order_v1(
    applied: &[(CheckpointOrdinalV1, CheckpointApplyPhaseV1)],
) -> Result<(), CheckpointOrderErrorV1> {
    let mut prev = (0u16, 0u64);
    for (ord, phase) in applied {
        let cur = (phase.rank(), ord.0);
        if cur.0 < prev.0 {
            return Err(CheckpointOrderErrorV1::PhaseRegression { previous: prev.0, got: cur.0 });
        }
        if cur <= prev {
            return Err(CheckpointOrderErrorV1::DuplicateOrderKey);
        }
        prev = cur;
    }
    Ok(())
}

/// The canonical apply sequence for a record set.
pub fn canonical_apply_sequence_v1(
    assigned: &[(CheckpointOrdinalV1, CheckpointRecordV1)],
) -> Vec<(CheckpointOrdinalV1, CheckpointApplyPhaseV1)> {
    let mut out: Vec<(CheckpointOrdinalV1, CheckpointApplyPhaseV1)> =
        assigned.iter().map(|(o, r)| (*o, r.phase)).collect();
    out.sort_by_key(|(o, p)| (p.rank(), o.0));
    out
}

#[cfg(test)]
mod checkpoint_egress_order_v1 {
    use super::*;

    fn rec(key: &[u8], phase: CheckpointApplyPhaseV1) -> CheckpointRecordV1 {
        CheckpointRecordV1 {
            egress_sort_key: key.to_vec(),
            stream: SemanticStreamIdV1::InGame,
            phase,
            payload_digest: [0; 32],
        }
    }

    #[test]
    fn same_intent_set_yields_same_tape_regardless_of_input_order() {
        use CheckpointApplyPhaseV1 as Ph;
        let mk = || {
            vec![
                rec(b"c", Ph::Components),
                rec(b"a", Ph::OrderedEvent),
                rec(b"b", Ph::IdentityLifecycle),
            ]
        };
        let forward = assign_ordinals_v1(mk()).unwrap();
        let mut reversed_input = mk();
        reversed_input.reverse();
        let reversed = assign_ordinals_v1(reversed_input).unwrap();
        assert_eq!(forward, reversed);
        // ordinal follows the sort key, not arrival
        assert_eq!(forward[0].1.egress_sort_key, b"a".to_vec());
        assert_eq!(forward[0].0, CheckpointOrdinalV1(1));

        // apply sequence is phase-major, ordinal-minor
        let seq = canonical_apply_sequence_v1(&forward);
        assert!(validate_apply_order_v1(&seq).is_ok());
        assert_eq!(seq[0].1, Ph::IdentityLifecycle);
        assert_eq!(seq.last().unwrap().1, Ph::OrderedEvent);
    }

    #[test]
    fn duplicate_key_and_phase_regression_are_typed() {
        use CheckpointApplyPhaseV1 as Ph;
        assert_eq!(
            assign_ordinals_v1(vec![rec(b"x", Ph::Components), rec(b"x", Ph::InGameState)]),
            Err(CheckpointOrderErrorV1::DuplicateOrderKey)
        );
        let bad = vec![
            (CheckpointOrdinalV1(1), Ph::Components),
            (CheckpointOrdinalV1(2), Ph::IdentityLifecycle),
        ];
        assert!(matches!(
            validate_apply_order_v1(&bad),
            Err(CheckpointOrderErrorV1::PhaseRegression { .. })
        ));
    }
}

/// `APEX-T3.4.04/.05` — descriptor + transcript roots.
use common::apex::digest::{DigestDomainIdV1, ProtocolDigestV1, digest_canonical_bytes_v1};

const ROOT_INPUT_LIMIT: u64 = 1 << 22;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamCheckpointPlanV1 {
    pub stream: SemanticStreamIdV1,
    pub begin_sequence: u64,
    pub first_data_sequence: Option<u64>,
    pub last_data_sequence: Option<u64>,
    pub barrier_sequence: u64,
    pub data_record_count: u32,
    pub payload_bytes: u64,
    pub stream_transcript_root: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointDescriptorV1 {
    pub schema_version: u32,
    pub binding: super::envelope::ActiveSessionBindingV1,
    pub epoch: u64,
    pub parent_epoch: u64,
    pub resource_profile_root: [u8; 32],
    pub apply_policy_root: [u8; 32],
    pub egress_order_policy_root: [u8; 32],
    pub data_record_count: u32,
    pub ordinal_max: u64,
    pub payload_bytes: u64,
    pub global_transcript_root: [u8; 32],
    /// One plan per required stream, in REQUIRED_CHECKPOINT_STREAMS_V1 order.
    pub streams: [StreamCheckpointPlanV1; 5],
    pub bootstrap_manifest_root: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointDescriptorErrorV1 {
    DescriptorIncomplete,
    DescriptorInvariant,
    NonCanonical,
    PolicyMismatch,
    StreamRootMismatch,
    GlobalRootMismatch,
}

/// One data record's transcript entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptEntryV1 {
    pub sequence: u64,
    pub ordinal: CheckpointOrdinalV1,
    pub payload_kind: u16,
    pub payload_digest: [u8; 32],
}

fn root_of(domain: DigestDomainIdV1, preimage: &[u8]) -> Result<[u8; 32], CheckpointDescriptorErrorV1> {
    digest_canonical_bytes_v1(domain, preimage, ROOT_INPUT_LIMIT)
        .map(|d: ProtocolDigestV1| *d.bytes.as_array())
        .map_err(|_| CheckpointDescriptorErrorV1::NonCanonical)
}

/// Stream root over (sequence, ordinal, kind, digest), sequence-ordered.
/// An empty stream has a typed empty root, never an absent one.
pub fn stream_transcript_root_v1(
    binding: &super::envelope::ActiveSessionBindingV1,
    epoch: u64,
    stream: SemanticStreamIdV1,
    entries: &[TranscriptEntryV1],
) -> Result<[u8; 32], CheckpointDescriptorErrorV1> {
    let mut sorted = entries.to_vec();
    sorted.sort_by_key(|e| e.sequence);
    let mut p = Vec::with_capacity(64 + sorted.len() * 56);
    p.extend_from_slice(binding.session_id.as_uuid().as_bytes());
    p.extend_from_slice(&binding.epoch.get().to_be_bytes());
    p.extend_from_slice(&epoch.to_be_bytes());
    p.push(stream.as_u8());
    p.extend_from_slice(&(sorted.len() as u64).to_be_bytes());
    for e in &sorted {
        p.extend_from_slice(&e.sequence.to_be_bytes());
        p.extend_from_slice(&e.ordinal.0.to_be_bytes());
        p.extend_from_slice(&e.payload_kind.to_be_bytes());
        p.extend_from_slice(&e.payload_digest);
    }
    root_of(DigestDomainIdV1::CheckpointStreamTranscript, &p)
}

/// Global root over the whole epoch's records, ordinal-ordered.
pub fn global_transcript_root_v1(
    binding: &super::envelope::ActiveSessionBindingV1,
    epoch: u64,
    entries: &[TranscriptEntryV1],
) -> Result<[u8; 32], CheckpointDescriptorErrorV1> {
    let mut sorted = entries.to_vec();
    sorted.sort_by_key(|e| e.ordinal.0);
    let mut p = Vec::with_capacity(64 + sorted.len() * 48);
    p.extend_from_slice(binding.session_id.as_uuid().as_bytes());
    p.extend_from_slice(&binding.epoch.get().to_be_bytes());
    p.extend_from_slice(&epoch.to_be_bytes());
    p.extend_from_slice(&(sorted.len() as u64).to_be_bytes());
    for e in &sorted {
        p.extend_from_slice(&e.ordinal.0.to_be_bytes());
        p.extend_from_slice(&e.payload_kind.to_be_bytes());
        p.extend_from_slice(&e.payload_digest);
    }
    root_of(DigestDomainIdV1::CheckpointGlobalTranscript, &p)
}

impl CheckpointDescriptorV1 {
    /// Structural completeness: all five streams present in canonical
    /// order, per-stream counts/bytes summing to the global figures,
    /// barrier after begin, ordinal_max matching the record count.
    pub fn validate_v1(&self) -> Result<(), CheckpointDescriptorErrorV1> {
        use CheckpointDescriptorErrorV1 as E;
        let order: Vec<u8> = self.streams.iter().map(|s| s.stream.as_u8()).collect();
        let want: Vec<u8> = REQUIRED_CHECKPOINT_STREAMS_V1.iter().map(|s| s.as_u8()).collect();
        if order != want {
            return Err(E::DescriptorIncomplete);
        }
        let mut records = 0u64;
        let mut bytes = 0u64;
        for s in &self.streams {
            if s.barrier_sequence <= s.begin_sequence {
                return Err(E::DescriptorInvariant);
            }
            match (s.first_data_sequence, s.last_data_sequence, s.data_record_count) {
                (None, None, 0) => {},
                (Some(f), Some(l), n) if n > 0 && f <= l && f > s.begin_sequence && l < s.barrier_sequence => {},
                _ => return Err(E::DescriptorInvariant),
            }
            records += s.data_record_count as u64;
            bytes += s.payload_bytes;
        }
        if records != self.data_record_count as u64 || bytes != self.payload_bytes {
            return Err(E::DescriptorInvariant);
        }
        if self.ordinal_max != records {
            return Err(E::DescriptorInvariant);
        }
        if self.epoch == 0 || self.parent_epoch + 1 != self.epoch {
            return Err(E::DescriptorInvariant);
        }
        Ok(())
    }

    pub fn descriptor_root_v1(&self) -> Result<[u8; 32], CheckpointDescriptorErrorV1> {
        self.validate_v1()?;
        let mut p = Vec::with_capacity(512);
        p.extend_from_slice(&self.schema_version.to_be_bytes());
        p.extend_from_slice(self.binding.session_id.as_uuid().as_bytes());
        p.extend_from_slice(&self.binding.epoch.get().to_be_bytes());
        p.extend_from_slice(&self.epoch.to_be_bytes());
        p.extend_from_slice(&self.parent_epoch.to_be_bytes());
        p.extend_from_slice(&self.resource_profile_root);
        p.extend_from_slice(&self.apply_policy_root);
        p.extend_from_slice(&self.egress_order_policy_root);
        p.extend_from_slice(&self.data_record_count.to_be_bytes());
        p.extend_from_slice(&self.ordinal_max.to_be_bytes());
        p.extend_from_slice(&self.payload_bytes.to_be_bytes());
        p.extend_from_slice(&self.global_transcript_root);
        for s in &self.streams {
            p.push(s.stream.as_u8());
            p.extend_from_slice(&s.begin_sequence.to_be_bytes());
            p.extend_from_slice(&s.first_data_sequence.unwrap_or(0).to_be_bytes());
            p.extend_from_slice(&s.last_data_sequence.unwrap_or(0).to_be_bytes());
            p.extend_from_slice(&s.barrier_sequence.to_be_bytes());
            p.extend_from_slice(&s.data_record_count.to_be_bytes());
            p.extend_from_slice(&s.payload_bytes.to_be_bytes());
            p.extend_from_slice(&s.stream_transcript_root);
        }
        match self.bootstrap_manifest_root {
            Some(r) => {
                p.push(1);
                p.extend_from_slice(&r);
            },
            None => p.push(0),
        }
        root_of(DigestDomainIdV1::CheckpointDescriptor, &p)
    }
}

#[cfg(test)]
mod checkpoint_descriptor_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding() -> super::super::envelope::ActiveSessionBindingV1 {
        super::super::envelope::ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn entry(seq: u64, ord: u64) -> TranscriptEntryV1 {
        TranscriptEntryV1 { sequence: seq, ordinal: CheckpointOrdinalV1(ord), payload_kind: 7, payload_digest: [ord as u8; 32] }
    }

    fn plan(stream: SemanticStreamIdV1, n: u32, root: [u8; 32]) -> StreamCheckpointPlanV1 {
        StreamCheckpointPlanV1 {
            stream,
            begin_sequence: 1,
            first_data_sequence: if n > 0 { Some(2) } else { None },
            last_data_sequence: if n > 0 { Some(1 + n as u64) } else { None },
            barrier_sequence: 2 + n as u64,
            data_record_count: n,
            payload_bytes: n as u64 * 10,
            stream_transcript_root: root,
        }
    }

    fn descriptor(counts: [u32; 5]) -> CheckpointDescriptorV1 {
        let total: u32 = counts.iter().sum();
        let streams: Vec<StreamCheckpointPlanV1> = REQUIRED_CHECKPOINT_STREAMS_V1
            .iter()
            .zip(counts)
            .map(|(s, n)| plan(*s, n, [0; 32]))
            .collect();
        CheckpointDescriptorV1 {
            schema_version: 1,
            binding: binding(),
            epoch: 1,
            parent_epoch: 0,
            resource_profile_root: [1; 32],
            apply_policy_root: [2; 32],
            egress_order_policy_root: [3; 32],
            data_record_count: total,
            ordinal_max: total as u64,
            payload_bytes: total as u64 * 10,
            global_transcript_root: [4; 32],
            streams: streams.try_into().unwrap(),
            bootstrap_manifest_root: None,
        }
    }

    #[test]
    fn roots_are_permutation_invariant_and_mutation_sensitive() {
        let b = binding();
        let entries = vec![entry(2, 1), entry(3, 2), entry(4, 3)];
        let mut shuffled = entries.clone();
        shuffled.reverse();
        let s1 = stream_transcript_root_v1(&b, 1, SemanticStreamIdV1::InGame, &entries).unwrap();
        let s2 = stream_transcript_root_v1(&b, 1, SemanticStreamIdV1::InGame, &shuffled).unwrap();
        assert_eq!(s1, s2, "input order must not move the stream root");

        // omission, duplication, and stream identity all move it
        assert_ne!(s1, stream_transcript_root_v1(&b, 1, SemanticStreamIdV1::InGame, &entries[..2]).unwrap());
        let mut dup = entries.clone();
        dup.push(entry(4, 3));
        assert_ne!(s1, stream_transcript_root_v1(&b, 1, SemanticStreamIdV1::InGame, &dup).unwrap());
        assert_ne!(s1, stream_transcript_root_v1(&b, 1, SemanticStreamIdV1::Terrain, &entries).unwrap());
        // empty stream has a typed root, not an absent one
        assert!(stream_transcript_root_v1(&b, 1, SemanticStreamIdV1::Terrain, &[]).is_ok());

        let g1 = global_transcript_root_v1(&b, 1, &entries).unwrap();
        assert_eq!(g1, global_transcript_root_v1(&b, 1, &shuffled).unwrap());
        assert_ne!(g1, s1, "domain separation: global root != stream root");
        assert_ne!(g1, global_transcript_root_v1(&b, 2, &entries).unwrap());
    }

    #[test]
    fn descriptor_validates_completeness_and_roots_bind_every_field() {
        use CheckpointDescriptorErrorV1 as E;
        let d = descriptor([1, 0, 2, 0, 1]);
        d.validate_v1().unwrap();
        let root = d.descriptor_root_v1().unwrap();

        // every scalar field moves the root
        for mutate in [
            (|x: &mut CheckpointDescriptorV1| x.payload_bytes += 1) as fn(&mut CheckpointDescriptorV1),
            |x| x.global_transcript_root[0] ^= 1,
            |x| x.apply_policy_root[0] ^= 1,
            |x| x.resource_profile_root[0] ^= 1,
            |x| x.egress_order_policy_root[0] ^= 1,
            |x| x.streams[0].stream_transcript_root[0] ^= 1,
            |x| x.bootstrap_manifest_root = Some([9; 32]),
        ] {
            let mut m = d.clone();
            mutate(&mut m);
            // payload_bytes mutation breaks the invariant; the rest re-root
            match m.descriptor_root_v1() {
                Ok(r) => assert_ne!(r, root),
                Err(E::DescriptorInvariant) => {},
                Err(e) => panic!("{e:?}"),
            }
        }

        // wrong stream order = incomplete
        let mut bad = d.clone();
        bad.streams.swap(0, 1);
        assert_eq!(bad.validate_v1(), Err(E::DescriptorIncomplete));
        // counts must sum
        let mut bad2 = d.clone();
        bad2.data_record_count += 1;
        assert_eq!(bad2.validate_v1(), Err(E::DescriptorInvariant));
        // epoch chain
        let mut bad3 = d.clone();
        bad3.parent_epoch = 5;
        assert_eq!(bad3.validate_v1(), Err(E::DescriptorInvariant));
    }
}

/// `APEX-T3.4.06` — in-line stream boundaries: every required stream
/// carries Begin then Data* then Barrier, so an empty stream is
/// explicitly fenced rather than ambiguously absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointBeginV1 {
    pub epoch: u64,
    pub stream: SemanticStreamIdV1,
    pub descriptor_root: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointBarrierV1 {
    pub epoch: u64,
    pub stream: SemanticStreamIdV1,
    pub descriptor_root: [u8; 32],
    pub data_record_count: u32,
    pub payload_bytes: u64,
    pub last_data_sequence: Option<u64>,
    pub stream_transcript_root: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamSegmentEventV1 {
    Begin(CheckpointBeginV1),
    Data { sequence: u64, ordinal: CheckpointOrdinalV1, payload_bytes: u64 },
    Barrier(CheckpointBarrierV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamSegmentErrorV1 {
    DataBeforeBegin,
    DuplicateBegin,
    EarlyBarrier,
    DuplicateBarrier,
    BarrierPlanMismatch,
    DescriptorRootMismatch,
    WrongStream,
    EpochMismatch,
}

/// Per-stream FIFO segmenter, one per (epoch, stream). Arrival order on a
/// fenced stream is send order.
#[derive(Debug, Clone)]
pub struct StreamSegmenterV1 {
    epoch: u64,
    stream: SemanticStreamIdV1,
    descriptor_root: [u8; 32],
    begun: bool,
    sealed: bool,
    records: u32,
    bytes: u64,
    last_sequence: Option<u64>,
}

impl StreamSegmenterV1 {
    pub fn new(epoch: u64, stream: SemanticStreamIdV1, descriptor_root: [u8; 32]) -> Self {
        Self { epoch, stream, descriptor_root, begun: false, sealed: false, records: 0, bytes: 0, last_sequence: None }
    }

    pub fn is_sealed(&self) -> bool { self.sealed }

    pub fn observed(&self) -> (u32, u64, Option<u64>) { (self.records, self.bytes, self.last_sequence) }

    pub fn accept_v1(&mut self, event: &StreamSegmentEventV1) -> Result<(), StreamSegmentErrorV1> {
        use StreamSegmentErrorV1 as E;
        match event {
            StreamSegmentEventV1::Begin(b) => {
                if b.stream != self.stream {
                    return Err(E::WrongStream);
                }
                if b.epoch != self.epoch {
                    return Err(E::EpochMismatch);
                }
                if b.descriptor_root != self.descriptor_root {
                    return Err(E::DescriptorRootMismatch);
                }
                if self.begun {
                    return Err(E::DuplicateBegin);
                }
                self.begun = true;
                Ok(())
            },
            StreamSegmentEventV1::Data { sequence, payload_bytes, .. } => {
                if !self.begun {
                    return Err(E::DataBeforeBegin);
                }
                if self.sealed {
                    return Err(E::DuplicateBarrier);
                }
                self.records += 1;
                self.bytes += payload_bytes;
                self.last_sequence = Some(*sequence);
                Ok(())
            },
            StreamSegmentEventV1::Barrier(b) => {
                if b.stream != self.stream {
                    return Err(E::WrongStream);
                }
                if b.epoch != self.epoch {
                    return Err(E::EpochMismatch);
                }
                if b.descriptor_root != self.descriptor_root {
                    return Err(E::DescriptorRootMismatch);
                }
                if !self.begun {
                    return Err(E::EarlyBarrier);
                }
                if self.sealed {
                    return Err(E::DuplicateBarrier);
                }
                // Barrier must match what actually crossed this stream.
                if b.data_record_count != self.records
                    || b.payload_bytes != self.bytes
                    || b.last_data_sequence != self.last_sequence
                {
                    return Err(E::BarrierPlanMismatch);
                }
                self.sealed = true;
                Ok(())
            },
        }
    }
}

#[cfg(test)]
mod checkpoint_controls_v1 {
    use super::*;

    fn begin(stream: SemanticStreamIdV1) -> StreamSegmentEventV1 {
        StreamSegmentEventV1::Begin(CheckpointBeginV1 { epoch: 1, stream, descriptor_root: [5; 32] })
    }

    fn data(seq: u64, ord: u64) -> StreamSegmentEventV1 {
        StreamSegmentEventV1::Data { sequence: seq, ordinal: CheckpointOrdinalV1(ord), payload_bytes: 10 }
    }

    fn barrier(stream: SemanticStreamIdV1, n: u32, bytes: u64, last: Option<u64>) -> StreamSegmentEventV1 {
        StreamSegmentEventV1::Barrier(CheckpointBarrierV1 {
            epoch: 1,
            stream,
            descriptor_root: [5; 32],
            data_record_count: n,
            payload_bytes: bytes,
            last_data_sequence: last,
            stream_transcript_root: [6; 32],
        })
    }

    #[test]
    fn every_stream_including_empty_is_fenced() {
        for stream in REQUIRED_CHECKPOINT_STREAMS_V1 {
            let mut seg = StreamSegmenterV1::new(1, stream, [5; 32]);
            seg.accept_v1(&begin(stream)).unwrap();
            seg.accept_v1(&data(2, 1)).unwrap();
            seg.accept_v1(&data(3, 2)).unwrap();
            seg.accept_v1(&barrier(stream, 2, 20, Some(3))).unwrap();
            assert!(seg.is_sealed());

            let mut empty = StreamSegmenterV1::new(1, stream, [5; 32]);
            empty.accept_v1(&begin(stream)).unwrap();
            empty.accept_v1(&barrier(stream, 0, 0, None)).unwrap();
            assert!(empty.is_sealed());
            assert_eq!(empty.observed(), (0, 0, None));
        }
    }

    #[test]
    fn segment_violations_are_typed() {
        use StreamSegmentErrorV1 as E;
        let s = SemanticStreamIdV1::InGame;
        let mut a = StreamSegmenterV1::new(1, s, [5; 32]);
        assert_eq!(a.accept_v1(&data(2, 1)), Err(E::DataBeforeBegin));

        let mut b = StreamSegmenterV1::new(1, s, [5; 32]);
        b.accept_v1(&begin(s)).unwrap();
        assert_eq!(b.accept_v1(&begin(s)), Err(E::DuplicateBegin));

        let mut c = StreamSegmenterV1::new(1, s, [5; 32]);
        assert_eq!(c.accept_v1(&barrier(s, 0, 0, None)), Err(E::EarlyBarrier));

        let mut d = StreamSegmenterV1::new(1, s, [5; 32]);
        d.accept_v1(&begin(s)).unwrap();
        d.accept_v1(&barrier(s, 0, 0, None)).unwrap();
        assert_eq!(d.accept_v1(&barrier(s, 0, 0, None)), Err(E::DuplicateBarrier));
        assert_eq!(d.accept_v1(&data(2, 1)), Err(E::DuplicateBarrier));

        let mut e = StreamSegmenterV1::new(1, s, [5; 32]);
        e.accept_v1(&begin(s)).unwrap();
        e.accept_v1(&data(2, 1)).unwrap();
        assert_eq!(e.accept_v1(&barrier(s, 5, 50, Some(9))), Err(E::BarrierPlanMismatch));

        let mut f = StreamSegmenterV1::new(1, s, [5; 32]);
        assert_eq!(f.accept_v1(&begin(SemanticStreamIdV1::Terrain)), Err(E::WrongStream));
        let mut g = StreamSegmenterV1::new(2, s, [5; 32]);
        assert_eq!(g.accept_v1(&begin(s)), Err(E::EpochMismatch));
        let mut h = StreamSegmenterV1::new(1, s, [9; 32]);
        assert_eq!(h.accept_v1(&begin(s)), Err(E::DescriptorRootMismatch));
    }
}

/// `APEX-T3.4.07` — checkpoint context carried alongside a frame.
/// Data frames MUST carry epoch + ordinal + descriptor root; control
/// frames carry epoch + root and MUST NOT carry an ordinal; diagnostics
/// carry none. Unbound checkpoint data is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckpointedEnvelopeContextV1 {
    pub epoch: u64,
    pub ordinal: Option<CheckpointOrdinalV1>,
    pub descriptor_root: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointContextErrorV1 {
    MissingContext,
    IllegalOrdinal,
    MissingOrdinal,
    RootMismatch,
    EpochMismatch,
    ForbiddenContext,
}

/// Validates a frame context against its participation class and the
/// active descriptor.
pub fn validate_checkpoint_context_v1(
    participation: CheckpointParticipationV1,
    context: Option<&CheckpointedEnvelopeContextV1>,
    active_epoch: u64,
    active_descriptor_root: [u8; 32],
) -> Result<(), CheckpointContextErrorV1> {
    use CheckpointContextErrorV1 as E;
    use CheckpointParticipationV1 as P;
    match (participation, context) {
        (P::OutOfBandDiagnostic, None) => Ok(()),
        (P::OutOfBandDiagnostic, Some(_)) => Err(E::ForbiddenContext),
        (_, None) => Err(E::MissingContext),
        (class, Some(c)) => {
            if c.epoch != active_epoch {
                return Err(E::EpochMismatch);
            }
            if c.descriptor_root != active_descriptor_root {
                return Err(E::RootMismatch);
            }
            match (class, c.ordinal) {
                (P::CheckpointedData, Some(o)) if o.0 > 0 => Ok(()),
                (P::CheckpointedData, Some(_)) => Err(E::IllegalOrdinal),
                (P::CheckpointedData, None) => Err(E::MissingOrdinal),
                (P::CheckpointControl, None) => Ok(()),
                (P::CheckpointControl, Some(_)) => Err(E::IllegalOrdinal),
                (P::OutOfBandDiagnostic, _) => unreachable!("diagnostic handled above"),
            }
        },
    }
}

#[cfg(test)]
mod checkpoint_context_v1 {
    use super::*;

    const ROOT: [u8; 32] = [7; 32];

    fn ctx(epoch: u64, ordinal: Option<u64>, root: [u8; 32]) -> CheckpointedEnvelopeContextV1 {
        CheckpointedEnvelopeContextV1 { epoch, ordinal: ordinal.map(CheckpointOrdinalV1), descriptor_root: root }
    }

    #[test]
    fn context_field_matrix_is_exhaustively_enforced() {
        use CheckpointContextErrorV1 as E;
        use CheckpointParticipationV1 as P;
        let ok = |p, c: Option<CheckpointedEnvelopeContextV1>| {
            validate_checkpoint_context_v1(p, c.as_ref(), 1, ROOT)
        };

        // data: needs epoch + nonzero ordinal + matching root
        assert!(ok(P::CheckpointedData, Some(ctx(1, Some(1), ROOT))).is_ok());
        assert_eq!(ok(P::CheckpointedData, None), Err(E::MissingContext));
        assert_eq!(ok(P::CheckpointedData, Some(ctx(1, None, ROOT))), Err(E::MissingOrdinal));
        assert_eq!(ok(P::CheckpointedData, Some(ctx(1, Some(0), ROOT))), Err(E::IllegalOrdinal));
        assert_eq!(ok(P::CheckpointedData, Some(ctx(2, Some(1), ROOT))), Err(E::EpochMismatch));
        assert_eq!(ok(P::CheckpointedData, Some(ctx(1, Some(1), [9; 32]))), Err(E::RootMismatch));

        // control: epoch + root, ordinal forbidden
        assert!(ok(P::CheckpointControl, Some(ctx(1, None, ROOT))).is_ok());
        assert_eq!(ok(P::CheckpointControl, Some(ctx(1, Some(1), ROOT))), Err(E::IllegalOrdinal));
        assert_eq!(ok(P::CheckpointControl, None), Err(E::MissingContext));

        // diagnostic: no context at all
        assert!(ok(P::OutOfBandDiagnostic, None).is_ok());
        assert_eq!(ok(P::OutOfBandDiagnostic, Some(ctx(1, None, ROOT))), Err(E::ForbiddenContext));
    }
}
