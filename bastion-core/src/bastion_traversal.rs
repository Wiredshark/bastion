//! Stage-1 B5.8 route-owned off-mesh traversal task.
//!
//! This module is an extraction of the former emergency mount transaction,
//! not a parallel locomotion stack. It owns link identity, reservation,
//! phase/liveness, and interruption/release. Existing Agent/Chaser owns the
//! validated approach; Controller, CharacterState::Climb/Stand, contact and
//! physics remain authoritative execution.

use crate::bastion_jobs_core::EmergencyTraversalKind;
use common::{
    bastion::JobId,
    comp::bastion::{BastionTraversalMode, BastionTraversalOwnership},
    uid::Uid,
};
use vek::{Vec3, Vec3 as VekVec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BastionTraversalReject {
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
pub struct TraversalAuthority {
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
pub fn fenced_movement_write(
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
pub enum BastionTraversalInterruption {
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
    pub fn reason(self) -> &'static str {
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
pub enum BastionTraversalPhase {
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
    pub fn mode(self) -> Option<BastionTraversalMode> {
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
pub enum BastionTraversalPurpose {
    FullExit,
    ConstructionFrontier(JobId),
}

#[derive(Clone, Copy, Debug)]
pub struct BastionTraversalTask {
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
    pub fn transition(
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

    pub fn reserve(&mut self, member: Uid, tick: u64) -> Result<(), BastionTraversalReject> {
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

    pub fn validate_terrain_revision(
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

    pub fn interrupt(&mut self, interruption: BastionTraversalInterruption, tick: u64) {
        self.abort(interruption.reason(), tick);
    }

    pub fn complete(&mut self, tick: u64) {
        self.phase = BastionTraversalPhase::Complete;
        self.phase_tick = tick;
    }

    pub fn ownership(self) -> Option<BastionTraversalOwnership> {
        self.phase.mode().map(|mode| BastionTraversalOwnership {
            link_id: self.link_id,
            route_owner: self.owner,
            reserved_member: self.reserved_member,
            mode,
            terrain_revision: self.terrain_revision,
            // M3 queue fields default empty here — the task doesn't know
            // the queue; the ONE live attach site enriches them from the
            // board's `TraversalLink` (tooling snapshots read them raw).
            queue_position: None,
            queue_enqueue_tick: None,
            reservation_generation: 0,
            queue_len: 0,
        })
    }

    pub fn abort(&mut self, reason: &'static str, tick: u64) {
        // #110 gate 1: abort was SILENT -- state flipped with no emit, so a
        // run full of created-then-aborted transactions logged identically to
        // a run with none, and "never engages" could not be told from "engages
        // and churns". Emit BEFORE overwriting phase: the phase it aborted
        // FROM is the diagnostic payload.
        if std::env::var_os("BASTION_EGRESS_DIAG").is_some() {
            tracing::info!(
                kind = ?self.traversal_kind,
                aborted_from = ?self.phase,
                reason,
                tick,
                "bastion: traversal transaction aborted"
            );
        }
        self.abort_reason = Some(reason);
        self.phase = BastionTraversalPhase::Abort;
        self.phase_tick = tick;
    }

    pub fn reservation_matches(self, member: Uid) -> bool {
        self.reserved_member == member && self.phase != BastionTraversalPhase::Abort
    }

    /// bastion (R10): the authority tuple this task's writers present.
    pub fn authority(&self) -> TraversalAuthority {
        TraversalAuthority {
            link_id: self.link_id,
            epoch: self.epoch,
            member: self.reserved_member,
        }
    }
}

/// bastion (M3): one queue ticket. The fair-order key is
/// `(enqueue_tick, uid)` — UID is the TIEBREAK only. This is R9's direct
/// fix for the lowest-UID-alone head selection: with UID-alone, a low-uid
/// member that cancels and reacquires jumps back to the FRONT and starves
/// everyone behind it; with the tick-first key, a re-enqueue (which goes
/// through the leave path first) gets a NEW tick and the back of the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraversalQueueTicket {
    pub member: Uid,
    pub enqueue_tick: u64,
}

impl TraversalQueueTicket {
    pub fn key(self) -> (u64, u64) {
        (self.enqueue_tick, self.member.0.get())
    }
}

/// bastion (M3): the persistent traversal link — link identity that
/// OUTLIVES any one task, so a queue can exist before any task does
/// (today's task-carried `link_id` dies with its task). Capacity stays 1
/// for M3 (ML-2/3 raises it later without a type change). Entry / exit /
/// terrain-revision deliberately live on the route DESCRIPTOR (same
/// owner key) — one source of truth, no duplicate to drift. The MONOTONE
/// fencing epoch lives in the JobBoard's `link_epochs` store (R10), not
/// here: an empty link container may be pruned, the epoch never resets.
#[derive(Clone, Debug)]
pub struct TraversalLink {
    /// Simultaneous same-direction traversers allowed. M3: always 1.
    pub capacity: u8,
    /// Bumps on every HEAD identity change (election/handover) —
    /// inspection/recorder metadata, complementary to R10's fencing
    /// `epoch` (which advances only on release-class events and is the
    /// safety-bearing counter).
    pub reservation_generation: u64,
    /// Kept sorted by the fair key; insert is `partition_point` so equal
    /// ticks resolve by uid deterministically.
    pub queue: Vec<TraversalQueueTicket>,
}

impl Default for TraversalLink {
    fn default() -> Self {
        Self {
            capacity: 1,
            reservation_generation: 0,
            queue: Vec::new(),
        }
    }
}

impl TraversalLink {
    /// Idempotent: a member already queued keeps its ORIGINAL ticket
    /// (returns false). Fair re-enqueue semantics come from the caller
    /// dequeuing first (the single leave path), never from re-keying here.
    pub fn enqueue(&mut self, member: Uid, tick: u64) -> bool {
        if self.queue.iter().any(|t| t.member == member) {
            return false;
        }
        let ticket = TraversalQueueTicket {
            member,
            enqueue_tick: tick,
        };
        let at = self.queue.partition_point(|t| t.key() <= ticket.key());
        self.queue.insert(at, ticket);
        true
    }

    pub fn dequeue(&mut self, member: Uid) -> bool {
        match self.queue.iter().position(|t| t.member == member) {
            Some(at) => {
                self.queue.remove(at);
                true
            },
            None => false,
        }
    }

    pub fn head(&self) -> Option<Uid> {
        self.queue.first().map(|t| t.member)
    }

    /// 0 = head. `None` = not queued.
    pub fn position(&self, member: Uid) -> Option<usize> {
        self.queue.iter().position(|t| t.member == member)
    }

    pub fn ticket(&self, member: Uid) -> Option<TraversalQueueTicket> {
        self.queue.iter().copied().find(|t| t.member == member)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Probe-shaped copy of the queue: `(member uid, enqueue_tick)` in
    /// fair order.
    pub fn snapshot(&self) -> Vec<(u64, u64)> {
        self.queue
            .iter()
            .map(|t| (t.member.0.get(), t.enqueue_tick))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod m3_queue_tests {
    use super::*;

    fn uid(n: u64) -> Uid {
        Uid(std::num::NonZeroU64::new(n).unwrap())
    }

    #[test]
    fn fair_order_is_tick_first_uid_tiebreak() {
        let mut link = TraversalLink::default();
        // Higher uid enqueued EARLIER goes first (tick beats uid — the
        // R9 fix; min-UID would have elected 3).
        assert!(link.enqueue(uid(9), 100));
        assert!(link.enqueue(uid(3), 200));
        assert_eq!(link.head(), Some(uid(9)));
        // Same tick: uid is the tiebreak.
        assert!(link.enqueue(uid(5), 200));
        assert_eq!(link.position(uid(3)), Some(1));
        assert_eq!(link.position(uid(5)), Some(2));
        assert_eq!(link.len(), 3);
    }

    #[test]
    fn reenqueue_after_leave_goes_to_the_back() {
        let mut link = TraversalLink::default();
        link.enqueue(uid(1), 10); // low uid, front
        link.enqueue(uid(7), 20);
        assert_eq!(link.head(), Some(uid(1)));
        // Cancel/reacquire: uid 1 leaves, re-enqueues LATER — it must NOT
        // return to the front (the exact starvation UID-alone permits).
        assert!(link.dequeue(uid(1)));
        assert_eq!(link.head(), Some(uid(7)));
        assert!(link.enqueue(uid(1), 30));
        assert_eq!(link.head(), Some(uid(7)));
        assert_eq!(link.position(uid(1)), Some(1));
        assert_eq!(link.ticket(uid(1)).unwrap().enqueue_tick, 30);
    }

    #[test]
    fn enqueue_is_idempotent_keeping_the_original_ticket() {
        let mut link = TraversalLink::default();
        assert!(link.enqueue(uid(4), 50));
        // A repeat enqueue (same member, later tick) is a no-op — the
        // original ticket stands; only a real leave re-keys.
        assert!(!link.enqueue(uid(4), 999));
        assert_eq!(link.ticket(uid(4)).unwrap().enqueue_tick, 50);
        assert_eq!(link.len(), 1);
        assert!(link.dequeue(uid(4)));
        assert!(!link.dequeue(uid(4)));
        assert!(link.is_empty());
    }

    #[test]
    fn capacity_defaults_to_one_for_m3() {
        // ML-2/3 raises this; M3 must not sneak capacity-N.
        assert_eq!(TraversalLink::default().capacity, 1);
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
