//! BUILD-007A10.8 (part 2) — clean shutdown, fault terminals, and crash
//! recovery (design §19).
//!
//! Self-contained substrate: the 12-stage declared shutdown order as a strict
//! gate (§19.1) — no lower stage publishes success while an earlier required
//! acknowledgement is missing — plus the typed fault terminals (§19.2/§19.4)
//! that always retain evidence and never publish success. The live wgpu drain /
//! worker joins / filesystem sync are the integration surface.

/// The 12 declared shutdown stages (§19.1), in strict order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShutdownStage {
    FreezeInputs = 1,
    FreezeRequests = 2,
    TerminalizeUnsubmitted = 3,
    SubmitFinalGpuWork = 4,
    DriveQueueCompletion = 5,
    TerminalizeReadbacks = 6,
    JoinWorkers = 7,
    ClientFlushAndAck = 8,
    ServerStopAndAck = 9,
    CloseNetwork = 10,
    FlushArtifacts = 11,
    PublishCommit = 12,
}

impl ShutdownStage {
    const ORDER: [ShutdownStage; 12] = [
        ShutdownStage::FreezeInputs,
        ShutdownStage::FreezeRequests,
        ShutdownStage::TerminalizeUnsubmitted,
        ShutdownStage::SubmitFinalGpuWork,
        ShutdownStage::DriveQueueCompletion,
        ShutdownStage::TerminalizeReadbacks,
        ShutdownStage::JoinWorkers,
        ShutdownStage::ClientFlushAndAck,
        ShutdownStage::ServerStopAndAck,
        ShutdownStage::CloseNetwork,
        ShutdownStage::FlushArtifacts,
        ShutdownStage::PublishCommit,
    ];

    fn ordinal(self) -> usize {
        Self::ORDER.iter().position(|&s| s == self).expect("in ORDER")
    }

    fn next(self) -> Option<ShutdownStage> {
        Self::ORDER.get(self.ordinal() + 1).copied()
    }
}

/// Typed fault terminals (§19.2/§19.4). Each retains semantic evidence and can
/// never become a success verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultTerminal {
    /// `R0D_INVALID_EVIDENCE_DEVICE_LOST` (§19.2).
    DeviceLost,
    /// `R0D_INVALID_EVIDENCE_SURFACE_LOST` (§19.2).
    SurfaceLost,
    /// Map/readback failure => invalid evidence (§19.2).
    MapReadbackFailure,
    /// `R0D_INVALID_EVIDENCE_INFRA_TIMEOUT` — outer harness kill (§19.3).
    InfraTimeout,
    /// `R0D_FAIL_SHUTDOWN` — panic / duplicate terminal / unjoined worker /
    /// unsynced file / missing ack / post-terminal write (§19.4).
    ShutdownFail,
}

/// Shutdown-sequence errors (§19.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownError {
    /// Attempted to skip a stage or advance out of order.
    OutOfOrder { from: ShutdownStage, to: ShutdownStage },
    /// Reached a stage that requires an acknowledgement not yet recorded.
    MissingAck { stage: ShutdownStage },
    /// The sequence already faulted or was interrupted.
    AlreadyTerminal,
}

/// The final verdict of a shutdown sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownVerdict {
    /// Clean shutdown: all 12 stages, both acks, no fault/interrupt.
    Success,
    /// A typed fault terminal — evidence retained, never success.
    InvalidOrFailed(FaultTerminal),
    /// SIGINT once-only interrupt (§19.4): incomplete, never success.
    Interrupted,
}

/// A strict 12-stage shutdown gate (§19.1). Advances one stage at a time; the
/// client/server close acks (stages 8/9) are required before any later stage,
/// and only a clean full traversal can publish success.
#[derive(Clone, Copy, Debug)]
pub struct ShutdownSequenceV1 {
    stage: ShutdownStage,
    client_ack: bool,
    server_ack: bool,
    fault: Option<FaultTerminal>,
    interrupted: bool,
}

impl Default for ShutdownSequenceV1 {
    fn default() -> Self {
        Self {
            stage: ShutdownStage::FreezeInputs,
            client_ack: false,
            server_ack: false,
            fault: None,
            interrupted: false,
        }
    }
}

impl ShutdownSequenceV1 {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn stage(self) -> ShutdownStage {
        self.stage
    }

    fn is_terminal(self) -> bool {
        self.fault.is_some() || self.interrupted
    }

    /// Record the client close ack (produced at stage 8).
    pub fn record_client_ack(&mut self) {
        self.client_ack = true;
    }

    /// Record the server close ack (produced at stage 9).
    pub fn record_server_ack(&mut self) {
        self.server_ack = true;
    }

    /// Advance to the immediate next stage (§19.1). Rejects out-of-order jumps,
    /// and refuses to move past a stage whose required ack is missing.
    pub fn advance(&mut self, to: ShutdownStage) -> Result<(), ShutdownError> {
        if self.is_terminal() {
            return Err(ShutdownError::AlreadyTerminal);
        }
        if self.stage.next() != Some(to) {
            return Err(ShutdownError::OutOfOrder { from: self.stage, to });
        }
        // No lower stage may proceed if an earlier required ack is missing.
        if to > ShutdownStage::ClientFlushAndAck && !self.client_ack {
            return Err(ShutdownError::MissingAck { stage: ShutdownStage::ClientFlushAndAck });
        }
        if to > ShutdownStage::ServerStopAndAck && !self.server_ack {
            return Err(ShutdownError::MissingAck { stage: ShutdownStage::ServerStopAndAck });
        }
        self.stage = to;
        Ok(())
    }

    /// Record a typed fault (§19.2/§19.4). Once faulted the run can never succeed.
    pub fn fault(&mut self, f: FaultTerminal) {
        if self.fault.is_none() {
            self.fault = Some(f);
        }
    }

    /// SIGINT once-only transition to Interrupted (§19.4).
    pub fn interrupt(&mut self) {
        self.interrupted = true;
    }

    /// The final verdict. Success requires reaching stage 12 with both acks and
    /// no fault/interrupt; otherwise a typed terminal (§19.1/§19.4).
    #[must_use]
    pub fn verdict(self) -> ShutdownVerdict {
        if let Some(f) = self.fault {
            return ShutdownVerdict::InvalidOrFailed(f);
        }
        if self.interrupted {
            return ShutdownVerdict::Interrupted;
        }
        if self.stage == ShutdownStage::PublishCommit && self.client_ack && self.server_ack {
            ShutdownVerdict::Success
        } else {
            ShutdownVerdict::InvalidOrFailed(FaultTerminal::ShutdownFail)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive a clean sequence, recording acks at stages 8/9.
    fn clean_run() -> ShutdownSequenceV1 {
        let mut s = ShutdownSequenceV1::new();
        for &stage in ShutdownStage::ORDER.iter().skip(1) {
            if stage == ShutdownStage::CloseNetwork {
                // Acks were produced at 8/9, before advancing past them.
            }
            if stage > ShutdownStage::ClientFlushAndAck && !s.client_ack {
                s.record_client_ack();
            }
            if stage > ShutdownStage::ServerStopAndAck && !s.server_ack {
                s.record_server_ack();
            }
            s.advance(stage).unwrap();
        }
        s
    }

    #[test]
    fn clean_full_traversal_succeeds() {
        assert_eq!(clean_run().verdict(), ShutdownVerdict::Success);
    }

    #[test]
    fn out_of_order_advance_rejected() {
        let mut s = ShutdownSequenceV1::new();
        assert_eq!(
            s.advance(ShutdownStage::JoinWorkers),
            Err(ShutdownError::OutOfOrder { from: ShutdownStage::FreezeInputs, to: ShutdownStage::JoinWorkers })
        );
    }

    #[test]
    fn missing_client_ack_blocks_progress_past_stage_8() {
        let mut s = ShutdownSequenceV1::new();
        // Walk cleanly up to stage 8 (ClientFlushAndAck).
        for &stage in &ShutdownStage::ORDER[1..8] {
            s.advance(stage).unwrap();
        }
        assert_eq!(s.stage(), ShutdownStage::ClientFlushAndAck);
        // Without the client ack, cannot advance to stage 9.
        assert_eq!(
            s.advance(ShutdownStage::ServerStopAndAck),
            Err(ShutdownError::MissingAck { stage: ShutdownStage::ClientFlushAndAck })
        );
    }

    #[test]
    fn incomplete_sequence_is_not_success() {
        let mut s = ShutdownSequenceV1::new();
        s.advance(ShutdownStage::FreezeRequests).unwrap();
        assert_eq!(s.verdict(), ShutdownVerdict::InvalidOrFailed(FaultTerminal::ShutdownFail));
    }

    #[test]
    fn device_loss_can_never_succeed() {
        let mut s = clean_run();
        s.fault(FaultTerminal::DeviceLost);
        assert_eq!(s.verdict(), ShutdownVerdict::InvalidOrFailed(FaultTerminal::DeviceLost));
        // Further faults do not overwrite the first.
        s.fault(FaultTerminal::ShutdownFail);
        assert_eq!(s.verdict(), ShutdownVerdict::InvalidOrFailed(FaultTerminal::DeviceLost));
    }

    #[test]
    fn interrupt_is_never_success() {
        let mut s = ShutdownSequenceV1::new();
        s.interrupt();
        assert_eq!(s.verdict(), ShutdownVerdict::Interrupted);
        // A faulted-then-interrupted run reports the fault (fault checked first).
        assert_eq!(s.advance(ShutdownStage::FreezeRequests), Err(ShutdownError::AlreadyTerminal));
    }
}
