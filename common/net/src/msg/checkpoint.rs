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
