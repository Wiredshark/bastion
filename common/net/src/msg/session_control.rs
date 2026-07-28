//! `APEX-T3.6.01` — session termination as a fenced control frame. Both
//! earlier tiers point here: T3.4 left sixteen session-control cases open
//! (`CKPT-036`, `CKPT-162`..`CKPT-176`) because no terminate FRAME
//! existed, and T3.5 left server-side teardown open (`CMD-084`) for the
//! same reason.
//!
//! Two rules shape the design:
//! - A termination frame is a REQUEST, never an authority. The T3.2
//!   session registry decides when a session is closed; this state
//!   machine does not consider itself terminated until the registry
//!   confirms (`CKPT-176`).
//! - Terminating mid-checkpoint DISCARDS the aligning or prepared work
//!   rather than committing part of it (`CKPT-163`..`CKPT-165`), while a
//!   checkpoint that already committed stays committed (`CKPT-166`).

use super::envelope::ActiveSessionBindingV1;
use serde::{Deserialize, Serialize};

/// Why a session is ending. Typed: a reason is a tag the receiver can act
/// on, never prose it has to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(into = "u8", try_from = "u8")]
pub enum SessionTerminationReasonV1 {
    ClientRequested = 1,
    ServerShutdown = 2,
    Kicked = 3,
    Banned = 4,
    /// The transport went away without a reason frame (`CKPT-175`).
    TransportClosed = 5,
}

impl SessionTerminationReasonV1 {
    pub const ALL: [Self; 5] = [
        Self::ClientRequested,
        Self::ServerShutdown,
        Self::Kicked,
        Self::Banned,
        Self::TransportClosed,
    ];

    pub const fn as_u8(self) -> u8 { self as u8 }

    pub fn try_from_u8(raw: u8) -> Option<Self> { Self::ALL.into_iter().find(|r| r.as_u8() == raw) }
}

impl From<SessionTerminationReasonV1> for u8 {
    fn from(r: SessionTerminationReasonV1) -> u8 { r.as_u8() }
}

impl TryFrom<u8> for SessionTerminationReasonV1 {
    type Error = &'static str;

    fn try_from(raw: u8) -> Result<Self, Self::Error> {
        Self::try_from_u8(raw).ok_or("unknown termination reason tag")
    }
}

/// One termination request on the control lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTerminateV1 {
    pub binding: ActiveSessionBindingV1,
    /// Control-lane sequence, monotone per session and separate from the
    /// semantic streams' own cursors.
    pub control_sequence: u64,
    pub reason: SessionTerminationReasonV1,
}

/// What accepting a termination does to work in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationEffectV1 {
    /// Nothing was in flight.
    NothingInFlight,
    /// A checkpoint was aligning; its staged records are dropped
    /// unapplied.
    DiscardAligning,
    /// A checkpoint was prepared but not committed; the prepared set is
    /// dropped unapplied.
    DiscardPrepared,
    /// The frame repeats one already accepted; nothing changes.
    AlreadyRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlErrorV1 {
    /// Boot, session or connection epoch does not match (`CKPT-167`).
    WrongBinding,
    /// At or below the last accepted control sequence (`CKPT-168`).
    StaleOrReplay,
    /// Skips the next expected control sequence (`CKPT-169`).
    SequenceGap { expected: u64, got: u64 },
    /// Reuses an accepted sequence with a different payload (`CKPT-171`).
    SequenceConflict,
}

/// What the aligner was doing when termination arrived. Mirrors
/// `ClientCheckpointPhaseV1` without depending on it, so the control
/// lane can be reasoned about on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InFlightWorkV1 {
    None,
    Aligning,
    Prepared,
}

#[derive(Debug, Clone)]
pub struct SessionTerminationStateV1 {
    binding: ActiveSessionBindingV1,
    next_control_sequence: u64,
    accepted: Option<SessionTerminateV1>,
    /// Set only when the T3.2 registry confirms; a frame alone never
    /// flips this.
    registry_closed: bool,
    committed_epoch: u64,
}

impl SessionTerminationStateV1 {
    pub fn new(binding: ActiveSessionBindingV1, committed_epoch: u64) -> Self {
        Self { binding, next_control_sequence: 1, accepted: None, registry_closed: false, committed_epoch }
    }

    /// A session is terminated when the REGISTRY says so, not when a
    /// frame says so.
    pub fn is_terminated(&self) -> bool { self.registry_closed }

    pub fn requested(&self) -> Option<SessionTerminateV1> { self.accepted }

    /// Checkpoints that committed before termination stay committed.
    pub fn committed_epoch(&self) -> u64 { self.committed_epoch }

    pub fn next_control_sequence(&self) -> u64 { self.next_control_sequence }

    /// Accepts a termination request and reports what it discards. Does
    /// NOT close the session: the caller takes this to the T3.2 registry
    /// and calls `confirm_registry_closed_v1` when that succeeds.
    pub fn accept_v1(
        &mut self,
        frame: &SessionTerminateV1,
        in_flight: InFlightWorkV1,
    ) -> Result<TerminationEffectV1, ControlErrorV1> {
        if frame.binding != self.binding {
            return Err(ControlErrorV1::WrongBinding);
        }
        if let Some(accepted) = self.accepted {
            if frame.control_sequence == accepted.control_sequence {
                // Same sequence: identical is an idempotent repeat,
                // different content is a conflict.
                return if *frame == accepted {
                    Ok(TerminationEffectV1::AlreadyRequested)
                } else {
                    Err(ControlErrorV1::SequenceConflict)
                };
            }
            if frame.control_sequence < accepted.control_sequence {
                return Err(ControlErrorV1::StaleOrReplay);
            }
        }
        if frame.control_sequence < self.next_control_sequence {
            return Err(ControlErrorV1::StaleOrReplay);
        }
        if frame.control_sequence > self.next_control_sequence {
            return Err(ControlErrorV1::SequenceGap {
                expected: self.next_control_sequence,
                got: frame.control_sequence,
            });
        }

        self.accepted = Some(*frame);
        self.next_control_sequence += 1;
        Ok(match in_flight {
            InFlightWorkV1::None => TerminationEffectV1::NothingInFlight,
            InFlightWorkV1::Aligning => TerminationEffectV1::DiscardAligning,
            InFlightWorkV1::Prepared => TerminationEffectV1::DiscardPrepared,
        })
    }

    /// The transport vanishing is a termination with no frame behind it.
    /// It still goes through the registry.
    pub fn transport_closed_v1(&mut self) -> TerminationEffectV1 {
        if self.accepted.is_none() {
            self.accepted = Some(SessionTerminateV1 {
                binding: self.binding,
                control_sequence: self.next_control_sequence,
                reason: SessionTerminationReasonV1::TransportClosed,
            });
            self.next_control_sequence += 1;
        }
        TerminationEffectV1::NothingInFlight
    }

    /// The registry has closed this session. Only now is it terminated.
    pub fn confirm_registry_closed_v1(&mut self) { self.registry_closed = true; }
}

#[cfg(test)]
mod session_termination_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding_at(epoch: u64) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(epoch).unwrap(),
        }
    }

    fn frame(binding: ActiveSessionBindingV1, sequence: u64, reason: SessionTerminationReasonV1) -> SessionTerminateV1 {
        SessionTerminateV1 { binding, control_sequence: sequence, reason }
    }

    /// `CKPT-162`..`CKPT-166`: what termination discards, and what it
    /// leaves alone.
    #[test]
    fn termination_discards_in_flight_work_and_spares_committed_checkpoints() {
        let b = binding_at(1);

        // idle
        let mut idle = SessionTerminationStateV1::new(b, 4);
        assert_eq!(
            idle.accept_v1(&frame(b, 1, SessionTerminationReasonV1::ClientRequested), InFlightWorkV1::None).unwrap(),
            TerminationEffectV1::NothingInFlight
        );

        // aligning, and prepared
        for (work, effect) in [
            (InFlightWorkV1::Aligning, TerminationEffectV1::DiscardAligning),
            (InFlightWorkV1::Prepared, TerminationEffectV1::DiscardPrepared),
        ] {
            let mut state = SessionTerminationStateV1::new(b, 4);
            assert_eq!(
                state.accept_v1(&frame(b, 1, SessionTerminationReasonV1::ClientRequested), work).unwrap(),
                effect
            );
            // a committed checkpoint is untouched either way
            assert_eq!(state.committed_epoch(), 4);
        }
    }

    /// `CKPT-167`..`CKPT-171`: the control lane's own typed rejects, and
    /// idempotency of a repeated terminal frame.
    #[test]
    fn control_lane_rejects_are_typed_and_a_repeat_is_idempotent() {
        let b = binding_at(1);
        let mut state = SessionTerminationStateV1::new(b, 0);

        // wrong binding
        assert_eq!(
            state
                .accept_v1(&frame(binding_at(2), 1, SessionTerminationReasonV1::ClientRequested), InFlightWorkV1::None)
                .unwrap_err(),
            ControlErrorV1::WrongBinding
        );
        // a gap
        assert_eq!(
            state.accept_v1(&frame(b, 5, SessionTerminationReasonV1::ClientRequested), InFlightWorkV1::None).unwrap_err(),
            ControlErrorV1::SequenceGap { expected: 1, got: 5 }
        );

        let accepted = frame(b, 1, SessionTerminationReasonV1::ClientRequested);
        state.accept_v1(&accepted, InFlightWorkV1::None).unwrap();

        // CKPT-170: an exact duplicate changes nothing
        assert_eq!(
            state.accept_v1(&accepted, InFlightWorkV1::Aligning).unwrap(),
            TerminationEffectV1::AlreadyRequested
        );
        // CKPT-171: the same sequence with a different payload conflicts
        assert_eq!(
            state.accept_v1(&frame(b, 1, SessionTerminationReasonV1::Kicked), InFlightWorkV1::None).unwrap_err(),
            ControlErrorV1::SequenceConflict
        );
        // CKPT-168: an older sequence is a replay
        let mut later = SessionTerminationStateV1::new(b, 0);
        later.accept_v1(&frame(b, 1, SessionTerminationReasonV1::ClientRequested), InFlightWorkV1::None).unwrap();
        assert_eq!(later.next_control_sequence(), 2);
        assert_eq!(
            later.accept_v1(&frame(b, 0, SessionTerminationReasonV1::ClientRequested), InFlightWorkV1::None).unwrap_err(),
            ControlErrorV1::StaleOrReplay
        );
    }

    /// `CKPT-175`/`CKPT-176`: a transport that vanishes still terminates,
    /// and a reason frame is never itself the authority.
    #[test]
    fn a_frame_is_a_request_and_the_registry_is_the_authority() {
        let b = binding_at(1);
        let mut state = SessionTerminationStateV1::new(b, 3);

        state.accept_v1(&frame(b, 1, SessionTerminationReasonV1::ClientRequested), InFlightWorkV1::None).unwrap();
        assert!(state.requested().is_some(), "the request is recorded");
        assert!(!state.is_terminated(), "a frame alone must not close the session");

        state.confirm_registry_closed_v1();
        assert!(state.is_terminated());
        assert_eq!(state.committed_epoch(), 3, "termination does not un-commit a checkpoint");

        // CKPT-175: no frame at all, and it still terminates through the
        // same registry step
        let mut dropped = SessionTerminationStateV1::new(b, 0);
        dropped.transport_closed_v1();
        assert_eq!(dropped.requested().unwrap().reason, SessionTerminationReasonV1::TransportClosed);
        assert!(!dropped.is_terminated());
        dropped.confirm_registry_closed_v1();
        assert!(dropped.is_terminated());
    }

    #[test]
    fn reason_tags_are_explicit_and_total() {
        for reason in SessionTerminationReasonV1::ALL {
            assert_eq!(SessionTerminationReasonV1::try_from_u8(reason.as_u8()), Some(reason));
        }
        assert_eq!(SessionTerminationReasonV1::try_from_u8(0), None);
        assert_eq!(SessionTerminationReasonV1::try_from_u8(6), None);
    }
}
