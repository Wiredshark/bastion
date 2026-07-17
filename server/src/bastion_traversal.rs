//! Stage-1 B5.8 route-owned off-mesh traversal task.
//!
//! This module is an extraction of the former emergency mount transaction,
//! not a parallel locomotion stack. It owns link identity, reservation,
//! phase/liveness, and interruption/release. Existing Agent/Chaser owns the
//! validated approach; Controller, CharacterState::Climb/Stand, contact and
//! physics remain authoritative execution.

use crate::bastion_jobs::EmergencyTraversalKind;
use common::{
    bastion::JobId,
    comp::bastion::{BastionTraversalMode, BastionTraversalOwnership},
    uid::Uid,
};
use vek::{Vec3, Vec3 as VekVec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BastionTraversalReject {
    LinkAlreadyReserved,
    StaleTerrainRevision,
    InvalidPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BastionTraversalInterruption {
    ContactLost,
    ExternalRelocation,
    AgentInbox,
    RtsimAction,
    ControllerEvent,
    Tether,
    Mount,
    Interpolation,
}

impl BastionTraversalInterruption {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::ContactLost => "authoritative-contact-lost",
            Self::ExternalRelocation => "external-relocation",
            Self::AgentInbox => "agent-inbox-interruption",
            Self::RtsimAction => "rtsim-action-interruption",
            Self::ControllerEvent => "controller-event-interruption",
            Self::Tether => "tether-interruption",
            Self::Mount => "mount-interruption",
            Self::Interpolation => "interpolation-interruption",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BastionTraversalPhase {
    LinkApproach,
    QueuedForLink,
    Reserved,
    TraversingEntry,
    TraversingLink,
    TraversingTopExit,
    FrontierWork,
    ConfirmingExitRelease,
    ConfirmingExitTraversal,
    ConfirmingExit,
    Complete,
    Abort,
}

impl BastionTraversalPhase {
    pub(crate) fn mode(self) -> Option<BastionTraversalMode> {
        match self {
            Self::LinkApproach => Some(BastionTraversalMode::LinkApproach),
            Self::QueuedForLink => Some(BastionTraversalMode::QueuedForLink),
            Self::Reserved | Self::TraversingEntry => Some(BastionTraversalMode::Reserved),
            Self::TraversingLink | Self::TraversingTopExit => {
                Some(BastionTraversalMode::TraversingLink)
            },
            Self::FrontierWork => Some(BastionTraversalMode::FrontierWork),
            Self::ConfirmingExitRelease | Self::ConfirmingExitTraversal | Self::ConfirmingExit => {
                Some(BastionTraversalMode::ConfirmingExit)
            },
            Self::Complete | Self::Abort => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BastionTraversalPurpose {
    FullExit,
    ConstructionFrontier(JobId),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BastionTraversalTask {
    pub link_id: u64,
    pub terrain_revision: u64,
    pub reserved_member: Uid,
    pub entry: Vec3<i32>,
    pub exit: Vec3<i32>,
    pub owner: Uid,
    pub purpose: BastionTraversalPurpose,
    pub traversal_kind: EmergencyTraversalKind,
    pub mount: Vec3<i32>,
    pub top_z: i32,
    pub phase: BastionTraversalPhase,
    pub started_tick: u64,
    pub phase_tick: u64,
    pub last_progress_tick: u64,
    pub stable_samples: u8,
    pub best_z: f32,
    pub best_exit_distance: f32,
    pub exit_target: Option<Vec3<i32>>,
    pub exit_started_tick: u64,
    pub exit_stable_samples: u8,
    pub abort_reason: Option<&'static str>,
    pub ladder_contact: Option<Vec3<i32>>,
    pub wall_contact: Option<VekVec3<f32>>,
    pub traversal_started_tick: u64,
    pub stable_window_started_tick: u64,
    pub last_stable_sample_tick: u64,
}

impl BastionTraversalTask {
    pub(crate) fn transition(
        &mut self,
        next: BastionTraversalPhase,
        tick: u64,
    ) -> Result<(), BastionTraversalReject> {
        let allowed = matches!(
            (self.phase, next),
            (
                BastionTraversalPhase::LinkApproach,
                BastionTraversalPhase::QueuedForLink
            ) | (
                BastionTraversalPhase::LinkApproach,
                BastionTraversalPhase::Reserved
            ) | (
                BastionTraversalPhase::QueuedForLink,
                BastionTraversalPhase::Reserved
            ) | (
                BastionTraversalPhase::Reserved,
                BastionTraversalPhase::LinkApproach
            ) | (
                BastionTraversalPhase::Reserved,
                BastionTraversalPhase::TraversingEntry
            ) | (
                BastionTraversalPhase::TraversingEntry,
                BastionTraversalPhase::TraversingLink
            ) | (
                BastionTraversalPhase::TraversingLink,
                BastionTraversalPhase::TraversingTopExit
            ) | (
                BastionTraversalPhase::TraversingTopExit,
                BastionTraversalPhase::FrontierWork
            ) | (
                BastionTraversalPhase::TraversingTopExit,
                BastionTraversalPhase::ConfirmingExitRelease
            ) | (
                BastionTraversalPhase::ConfirmingExitRelease,
                BastionTraversalPhase::ConfirmingExitTraversal
            ) | (
                BastionTraversalPhase::ConfirmingExitTraversal,
                BastionTraversalPhase::ConfirmingExit
            ) | (
                BastionTraversalPhase::ConfirmingExit,
                BastionTraversalPhase::Complete
            ) | (
                BastionTraversalPhase::FrontierWork,
                BastionTraversalPhase::TraversingLink
            )
        );
        if !allowed {
            return Err(BastionTraversalReject::InvalidPhase);
        }
        self.phase = next;
        self.phase_tick = tick;
        Ok(())
    }

    pub(crate) fn reserve(&mut self, member: Uid, tick: u64) -> Result<(), BastionTraversalReject> {
        if self.phase != BastionTraversalPhase::QueuedForLink {
            return Err(BastionTraversalReject::InvalidPhase);
        }
        if self.reserved_member != member {
            return Err(BastionTraversalReject::LinkAlreadyReserved);
        }
        self.phase = BastionTraversalPhase::Reserved;
        self.phase_tick = tick;
        Ok(())
    }

    pub(crate) fn validate_terrain_revision(
        &mut self,
        observed: u64,
        tick: u64,
    ) -> Result<(), BastionTraversalReject> {
        if observed == self.terrain_revision {
            Ok(())
        } else {
            self.abort("stale-terrain-revision", tick);
            Err(BastionTraversalReject::StaleTerrainRevision)
        }
    }

    pub(crate) fn interrupt(&mut self, interruption: BastionTraversalInterruption, tick: u64) {
        self.abort(interruption.reason(), tick);
    }

    pub(crate) fn complete(&mut self, tick: u64) {
        self.phase = BastionTraversalPhase::Complete;
        self.phase_tick = tick;
    }

    pub(crate) fn ownership(self) -> Option<BastionTraversalOwnership> {
        self.phase.mode().map(|mode| BastionTraversalOwnership {
            link_id: self.link_id,
            route_owner: self.owner,
            reserved_member: self.reserved_member,
            mode,
            terrain_revision: self.terrain_revision,
        })
    }

    pub(crate) fn abort(&mut self, reason: &'static str, tick: u64) {
        self.abort_reason = Some(reason);
        self.phase = BastionTraversalPhase::Abort;
        self.phase_tick = tick;
    }

    pub(crate) fn reservation_matches(self, member: Uid) -> bool {
        self.reserved_member == member && self.phase != BastionTraversalPhase::Abort
    }
}
