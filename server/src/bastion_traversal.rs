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

/// bastion (R10): the fencing-token authority tuple a movement writer must
/// present — distributed-lock fencing-token prior art. Captured at task
/// creation (ADOPTING the link's current epoch — never advancing it, which
/// would fence the acquirer's own writes); the epoch store advances only on
/// release-class events, so any writer holding a tuple from BEFORE a
/// release/abort/reacquire/re-election presents a stale epoch and its write
/// becomes a logged no-op by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TraversalAuthority {
    pub link_id: u64,
    pub epoch: u64,
    pub member: Uid,
}

/// bastion (R10): THE validity predicate — pure (unit-pinned truth table
/// below; the release-decision extraction's discipline). A write is valid
/// iff the presented epoch equals the link's CURRENT epoch AND the
/// presenter is the currently-reserved member (`None` = no live
/// reservation → nothing may write).
pub(crate) fn authority_valid(
    current_epoch: u64,
    current_member: Option<Uid>,
    a: &TraversalAuthority,
) -> bool {
    a.epoch == current_epoch && current_member == Some(a.member)
}

/// bastion (R10): the fenced movement write — THE one choke point every
/// owned traversal movement-writer calls (the amended seam: the owned
/// writers bypass sys/agent, so the fence lives here at the bastion write
/// sites; vanilla writes stay suppressed by the existing suppressor).
/// Validate-then-write: a stale tuple is a LOGGED NO-OP (`false`, both
/// tuples in the log line) — never a panic, never blocking the current
/// owner's valid write. Current state is passed by VALUE (the caller reads
/// the board) so this stays borrow-clean and pure-testable.
pub(crate) fn fenced_movement_write(
    current_epoch: u64,
    current_member: Option<Uid>,
    authority: &TraversalAuthority,
    controller: &mut common::comp::Controller,
    move_dir: vek::Vec2<f32>,
    move_z: f32,
) -> bool {
    if authority_valid(current_epoch, current_member, authority) {
        controller.inputs.move_dir = move_dir;
        controller.inputs.move_z = move_z;
        true
    } else {
        tracing::info!(
            presented_link = authority.link_id,
            presented_epoch = authority.epoch,
            presented_member = authority.member.0.get(),
            current_epoch,
            current_member = current_member.map(|m| m.0.get()),
            "bastion R10: stale-authority movement write REJECTED (no-op)"
        );
        // R10 (recorder v2): the rejection event — both tuples on tape (the
        // forensics field R10 promises). The recorder is env-gated and this
        // arm never fires in the non-race case, so this costs nothing live.
        crate::bastion_flight_recorder::record_writer(
            crate::bastion_flight_recorder::WriterEvent {
                schema: "bastion.flight-recorder.event/v2".into(),
                tick: 0,
                uid: authority.member.0.get(),
                observation_sequence: 310,
                snapshot_stage: "r10-fence-rejection".into(),
                dispatcher_dependency_proven: false,
                writer: "r10_fence".into(),
                move_dir: [move_dir.x, move_dir.y],
                move_z,
                target: None,
                note: format!(
                    "stale-write-rejected: presented=(link {}, epoch {}, member {}) vs \
                     current=(epoch {}, member {:?})",
                    authority.link_id,
                    authority.epoch,
                    authority.member.0.get(),
                    current_epoch,
                    current_member.map(|m| m.0.get()),
                ),
            },
        );
        false
    }
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
    /// bastion (R10): the epoch this task was created under (adopt-on-
    /// acquire from the JobBoard's `link_epochs`). The task's writers
    /// present `TraversalAuthority { link_id, epoch, member }`; a release-
    /// class event advances the store and orphans this value by design.
    pub epoch: u64,
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

    /// bastion (R10): the authority tuple this task's writers present.
    pub(crate) fn authority(&self) -> TraversalAuthority {
        TraversalAuthority {
            link_id: self.link_id,
            epoch: self.epoch,
            member: self.reserved_member,
        }
    }
}

#[cfg(test)]
mod r10_tests {
    use super::*;

    /// R10: the pure predicate's full truth table — epoch match × member
    /// match × live-reservation presence (the N-FENCE fixture drives the
    /// same predicate through the live helper; this pins the logic).
    #[test]
    fn authority_valid_truth_table() {
        let m1 = Uid(std::num::NonZeroU64::new(11).unwrap());
        let m2 = Uid(std::num::NonZeroU64::new(22).unwrap());
        let a = TraversalAuthority {
            link_id: 7,
            epoch: 3,
            member: m1,
        };
        // Valid: epoch current + member reserved.
        assert!(authority_valid(3, Some(m1), &a));
        // Stale epoch (a release advanced the store).
        assert!(!authority_valid(4, Some(m1), &a));
        // Right epoch, wrong member (re-election handed the link over).
        assert!(!authority_valid(3, Some(m2), &a));
        // No live reservation at all — nothing may write.
        assert!(!authority_valid(3, None, &a));
        // Both stale: still just false (no panic path exists).
        assert!(!authority_valid(9, Some(m2), &a));
        // Epoch from the FUTURE (store reset bug shape) is equally invalid:
        // equality, not ordering — a fencing token is exact.
        assert!(!authority_valid(2, Some(m1), &a));
    }
}
