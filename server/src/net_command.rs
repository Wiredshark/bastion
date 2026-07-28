//! `APEX-T3.5.08` — the server side of command idempotency: one journal
//! per session, the carriage check, and the exactly-once execution seam
//! wired into a single admission call the ingress path will make once
//! the command path is activated.
//!
//! Dormant: `SemanticEnvelopeRejectV1::CommandIdUnsupported` still
//! refuses every `Some(command_id)` at ingress, so nothing here is on a
//! live route yet. Activation is gated by `admit_command_activation_v1`,
//! which refuses today for named reasons.

use common_net::msg::ClientGeneral;
use common_net::msg::command::{
    ClientCommandOutboxV1, CommandCarriageErrorV1, CommandDescriptorV1, CommandExecutionV1, CommandJournalV1,
    CommandOutcomeV1, CommandReceiptV1, CommandRefusalV1, JournalErrorV1, ReceiptErrorV1,
    command_descriptor_from_frame_v1, execute_command_once_v1,
};
use common_net::msg::envelope::ActiveSessionBindingV1;
use common::apex::identity::CommandId;

/// What a command frame did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandIngressV1 {
    /// Not a command at all — the caller handles it the ordinary way.
    NotACommand,
    /// A command, with its receipt to send back. `executed` says whether
    /// the work actually ran; a replay reports `false` and the ORIGINAL
    /// outcome.
    Handled { receipt: CommandReceiptV1, executed: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandIngressErrorV1 {
    Carriage(CommandCarriageErrorV1),
    Journal(JournalErrorV1),
    Receipt(ReceiptErrorV1),
}

/// One session's command state. Created only when the deployment
/// supplies a journal capacity — the same "no invented production value"
/// rule the checkpoint profile follows.
#[derive(Debug, Clone)]
pub struct SessionCommandRuntimeV1 {
    journal: CommandJournalV1,
}

impl SessionCommandRuntimeV1 {
    pub fn new(binding: ActiveSessionBindingV1, journal_capacity: usize) -> Self {
        Self { journal: CommandJournalV1::new(binding, journal_capacity) }
    }

    pub fn binding(&self) -> ActiveSessionBindingV1 { self.journal.binding() }

    pub fn retired_floor(&self) -> u64 { self.journal.retired_floor() }

    /// Retires a command once the client acknowledges its terminal.
    pub fn retire_v1(&mut self, sequence: u64) -> Result<u64, CommandIngressErrorV1> {
        self.journal.retire_v1(sequence).map_err(CommandIngressErrorV1::Journal)
    }

    /// Admits one already-envelope-validated frame. The carriage check
    /// runs FIRST, so a command id on a query is refused before any
    /// journal slot is spent, and the work runs at most once ever.
    pub fn admit_frame_v1<F>(
        &mut self,
        command_id: Option<CommandId>,
        sequence: u64,
        effect_epoch: u64,
        payload: &ClientGeneral,
        payload_digest: [u8; 32],
        execute: F,
    ) -> Result<CommandIngressV1, CommandIngressErrorV1>
    where
        F: FnOnce(&CommandDescriptorV1) -> CommandOutcomeV1,
    {
        let binding = self.journal.binding();
        let descriptor = command_descriptor_from_frame_v1(binding, command_id, payload, payload_digest)
            .map_err(CommandIngressErrorV1::Carriage)?;
        let Some(descriptor) = descriptor else {
            return Ok(CommandIngressV1::NotACommand);
        };

        let execution =
            execute_command_once_v1(&mut self.journal, &descriptor, sequence, || execute(&descriptor))
                .map_err(CommandIngressErrorV1::Journal)?;
        let receipt = CommandReceiptV1::for_command_v1(&descriptor, execution.outcome(), effect_epoch)
            .map_err(CommandIngressErrorV1::Receipt)?;
        Ok(CommandIngressV1::Handled {
            receipt,
            executed: matches!(execution, CommandExecutionV1::Executed(_)),
        })
    }
}

/// `APEX-T3.5.08` — production admission for the command path, in the
/// same shape `T3.4.24` uses: every blocker is named, not just the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandActivationBlockerV1 {
    /// No deployment-supplied journal capacity or retry budget.
    NoProductionCommandProfile,
    /// The checkpoint tier this rides on is not activatable either.
    CheckpointPathInactive,
    /// Ingress still refuses every command id by construction.
    IngressRefusesCommandIds,
}

/// Refuses activation and says why. `IngressRefusesCommandIds` is the
/// structural one: both ingress paths return `CommandIdUnsupported` for
/// any `Some(command_id)`, so no command can reach this module until
/// that rejection is deliberately removed.
pub fn admit_command_activation_v1(
    client_type: common_net::msg::ClientType,
) -> Result<(), Vec<CommandActivationBlockerV1>> {
    let mut blockers = vec![
        CommandActivationBlockerV1::NoProductionCommandProfile,
        CommandActivationBlockerV1::IngressRefusesCommandIds,
    ];
    if crate::net_checkpoint::admit_checkpoint_activation_v1(client_type).is_err() {
        blockers.push(CommandActivationBlockerV1::CheckpointPathInactive);
    }
    Err(blockers)
}

/// Client-side twin: hand a receipt to the outbox that issued the
/// command. Kept here so the round trip has one place to read.
pub fn apply_receipt_to_outbox_v1(
    outbox: &mut ClientCommandOutboxV1,
    receipt: &CommandReceiptV1,
) -> Result<CommandOutcomeV1, CommandIngressErrorV1> {
    outbox.accept_receipt_v1(receipt).map_err(CommandIngressErrorV1::Receipt)
}

#[cfg(test)]
mod command_ingress_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use common_net::msg::command::{CommandKindV1, CommandRefusalV1};
    use common_net::msg::ServerGeneral;
    use common_net::msg::envelope::SemanticStreamIdV1;
    use std::cell::Cell;

    fn binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    /// Client issues, retries, server executes once, receipt clears the
    /// outbox — the whole loop through the real seams of both sides.
    #[test]
    fn the_round_trip_executes_once_and_clears_the_outbox() {
        let mut outbox = ClientCommandOutboxV1::new(binding(), 4);
        let mut server = SessionCommandRuntimeV1::new(binding(), 8);
        let runs = Cell::new(0u32);

        let payload = ClientGeneral::Terminate;
        let digest = [7u8; 32];
        let sent = outbox.issue_v1(CommandKindV1::SessionControl, digest).unwrap();

        let mut receipts = Vec::new();
        for _ in 0..3 {
            let handled = server
                .admit_frame_v1(Some(sent.descriptor.command_id), sent.sequence, 1, &payload, digest, |_| {
                    runs.set(runs.get() + 1);
                    CommandOutcomeV1::Applied { result_digest: [5; 32] }
                })
                .unwrap();
            match handled {
                CommandIngressV1::Handled { receipt, executed } => {
                    receipts.push((receipt, executed));
                },
                CommandIngressV1::NotACommand => panic!("Terminate is a command"),
            }
            // the client resends the SAME descriptor while unacknowledged
            let _ = outbox.retry_v1(&sent.descriptor.command_id);
        }
        assert_eq!(runs.get(), 1, "three deliveries, one execution");
        assert_eq!(receipts.iter().filter(|(_, executed)| *executed).count(), 1);
        assert!(receipts.iter().all(|(r, _)| r.outcome == CommandOutcomeV1::Applied { result_digest: [5; 32] }));

        // any of the identical receipts clears the outbox, exactly once
        let (receipt, _) = receipts[2];
        assert_eq!(
            apply_receipt_to_outbox_v1(&mut outbox, &receipt).unwrap(),
            CommandOutcomeV1::Applied { result_digest: [5; 32] }
        );
        assert_eq!(outbox.pending_len(), 0);
    }

    /// Carriage errors are caught before a journal slot is spent.
    #[test]
    fn a_query_carrying_a_command_id_never_reaches_the_journal() {
        let mut server = SessionCommandRuntimeV1::new(binding(), 1);
        let runs = Cell::new(0u32);
        let work = |_: &CommandDescriptorV1| {
            runs.set(runs.get() + 1);
            CommandOutcomeV1::Refused { reason: CommandRefusalV1::Unprocessable }
        };

        let id = common_net::msg::command::DerivedCommandIdSourceV1::id_for_ordinal_v1(&binding(), 1).unwrap();
        assert_eq!(
            server
                .admit_frame_v1(Some(id), 1, 1, &ClientGeneral::RequestCharacterList, [1; 32], work)
                .unwrap_err(),
            CommandIngressErrorV1::Carriage(CommandCarriageErrorV1::UnexpectedCommandId)
        );
        assert_eq!(
            server.admit_frame_v1(None, 1, 1, &ClientGeneral::Terminate, [1; 32], work).unwrap_err(),
            CommandIngressErrorV1::Carriage(CommandCarriageErrorV1::MissingCommandId)
        );
        assert_eq!(runs.get(), 0);

        // ...and a plain query passes straight through
        assert_eq!(
            server.admit_frame_v1(None, 1, 1, &ClientGeneral::RequestCharacterList, [1; 32], work).unwrap(),
            CommandIngressV1::NotACommand
        );
        assert_eq!(runs.get(), 0);
        // the journal slot is still free for a real command
        assert!(matches!(
            server.admit_frame_v1(Some(id), 1, 1, &ClientGeneral::Terminate, [1; 32], work).unwrap(),
            CommandIngressV1::Handled { executed: true, .. }
        ));
        assert_eq!(runs.get(), 1);
    }


    /// `T3.5.13`: a command result is checkpointed data on the canonical
    /// egress, and it applies AFTER the effect it reports.
    #[test]
    fn a_command_result_is_checkpointed_data_that_applies_after_its_effect() {
        use common_net::msg::checkpoint::{CheckpointApplyPhaseV1, CheckpointParticipantV1, CheckpointParticipationV1};
        use common_net::msg::command::{
            CommandDescriptorV1, CommandKindV1, CommandOutcomeV1, CommandPublicationV1,
        };
        use common_net::msg::envelope::SemanticRouteV1;

        let descriptor = CommandDescriptorV1 {
            binding: binding(),
            command_id: common::apex::identity::CommandId::generate(&mut FixedRandomBytesSourceV1([5; 16]))
                .unwrap(),
            kind: CommandKindV1::ControlAction,
            request_digest: [1; 32],
        };
        let published = CommandPublicationV1::publish_v1(
            &descriptor,
            1,
            CommandOutcomeV1::Applied { result_digest: [2; 32] },
            4,
        )
        .unwrap();
        let msg = ServerGeneral::CommandResult(published);

        // CMD-128/129: canonical egress, and inside a checkpoint
        assert_eq!(msg.semantic_stream(), SemanticStreamIdV1::General);
        assert_eq!(msg.participation_v1(), CheckpointParticipationV1::CheckpointedData);

        // CMD-130/131: the result applies after the effect's own records.
        // Every phase an effect can use ranks at or below OrderedEvent.
        let result_phase = msg.apply_phase_v1().unwrap();
        assert_eq!(result_phase, CheckpointApplyPhaseV1::OrderedEvent);
        // Stated over the WHOLE phase set rather than a few sample
        // payloads: no phase ranks above the result's, so no effect can
        // apply after the result that reports it. Equal rank (another
        // OrderedEvent) falls to the ordinal, which the checkpoint's own
        // canonical order already fixes.
        assert!(
            CheckpointApplyPhaseV1::ALL.iter().all(|phase| phase.rank() <= result_phase.rank()),
            "OrderedEvent must be the last phase for this claim to hold"
        );
    }

    /// The command path is refused for every client type today, and the
    /// refusal names each reason.
    #[test]
    fn command_activation_is_refused_with_named_blockers() {
        use common_net::msg::ClientType;

        for client_type in [ClientType::Game, ClientType::ChatOnly, ClientType::SilentSpectator] {
            let blockers = admit_command_activation_v1(client_type).unwrap_err();
            assert!(blockers.contains(&CommandActivationBlockerV1::NoProductionCommandProfile));
            assert!(blockers.contains(&CommandActivationBlockerV1::IngressRefusesCommandIds));
            assert!(
                blockers.contains(&CommandActivationBlockerV1::CheckpointPathInactive),
                "{client_type:?}: the command path cannot outrun the checkpoint path it rides on"
            );
        }
    }
}

/// `APEX-T3.5.15` — async workflows. Character create/edit/delete leave
/// the tick and come back from a worker, which is where exactly-once
/// usually dies: the response gets keyed by ECS entity (`CMD-106`), a
/// retry queues a second worker action (`CMD-107`), a late response
/// lands on a newer command (`CMD-109`), or a dropped channel leaves the
/// command executable forever (`CMD-110`).
///
/// `CommandContextV1` is what travels with the work. Its `effect_id` is
/// the command's own identity root, so it is stable across every retry
/// by construction (`CMD-115`) and cannot be confused with an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandContextV1 {
    pub descriptor: CommandDescriptorV1,
    pub sequence: u64,
    pub effect_id: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowErrorV1 {
    /// This command already has a worker action in flight (`CMD-107`).
    AlreadyQueued,
    /// No workflow under that effect id — a response that would land on
    /// the wrong command (`CMD-109`) or on a closed session (`CMD-108`).
    Unknown,
    /// A workflow cannot be dropped while its effect may still commit
    /// (`CMD-114`).
    StillInFlight,
    NonCanonical,
}

/// A worker's answer. There is no "success" variant a lost channel or a
/// panic can synthesise: those map to `Refused`, which is a real
/// terminal the client can see and reason about (`CMD-110`, `CMD-111`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerAnswerV1 {
    Completed(CommandOutcomeV1),
    ChannelLost,
    Panicked,
    TimedOut,
}

#[derive(Debug, Clone, Default)]
pub struct PendingWorkflowsV1 {
    pending: std::collections::BTreeMap<[u8; 32], CommandContextV1>,
}

impl CommandContextV1 {
    pub fn for_command_v1(descriptor: &CommandDescriptorV1, sequence: u64) -> Result<Self, WorkflowErrorV1> {
        let effect_id = descriptor.identity_root_v1().map_err(|_| WorkflowErrorV1::NonCanonical)?;
        Ok(Self { descriptor: *descriptor, sequence, effect_id })
    }
}

impl PendingWorkflowsV1 {
    pub fn new() -> Self { Self::default() }

    pub fn len(&self) -> usize { self.pending.len() }

    pub fn is_empty(&self) -> bool { self.pending.is_empty() }

    pub fn is_pending_v1(&self, effect_id: &[u8; 32]) -> bool { self.pending.contains_key(effect_id) }

    /// Queues the worker action for a command. A retry of the same
    /// command does NOT queue a second one.
    pub fn enqueue_v1(&mut self, context: CommandContextV1) -> Result<(), WorkflowErrorV1> {
        if self.pending.contains_key(&context.effect_id) {
            return Err(WorkflowErrorV1::AlreadyQueued);
        }
        self.pending.insert(context.effect_id, context);
        Ok(())
    }

    /// Resolves a worker's answer back to the command that asked for it,
    /// keyed by effect id — never by entity, never by arrival order.
    /// Every answer, including a lost channel or a panic, produces a
    /// terminal outcome, so no command is left permanently executable.
    pub fn resolve_v1(
        &mut self,
        effect_id: &[u8; 32],
        answer: WorkerAnswerV1,
    ) -> Result<(CommandContextV1, CommandOutcomeV1), WorkflowErrorV1> {
        let context = self.pending.remove(effect_id).ok_or(WorkflowErrorV1::Unknown)?;
        let outcome = match answer {
            WorkerAnswerV1::Completed(outcome) => outcome,
            WorkerAnswerV1::ChannelLost | WorkerAnswerV1::Panicked => {
                CommandOutcomeV1::Refused { reason: CommandRefusalV1::Unprocessable }
            },
            WorkerAnswerV1::TimedOut => CommandOutcomeV1::Refused { reason: CommandRefusalV1::PreconditionFailed },
        };
        Ok((context, outcome))
    }

    /// Drops every workflow belonging to a session that has closed. A
    /// workflow still in flight is NOT dropped: its effect may still
    /// commit, and forgetting it is how a late answer gets misattributed.
    pub fn close_session_v1(&mut self, session: &ActiveSessionBindingV1) -> Result<usize, WorkflowErrorV1> {
        let doomed: Vec<[u8; 32]> = self
            .pending
            .iter()
            .filter(|(_, c)| c.descriptor.binding.session_id == session.session_id)
            .map(|(k, _)| *k)
            .collect();
        if !doomed.is_empty() {
            return Err(WorkflowErrorV1::StillInFlight);
        }
        Ok(self.pending.len())
    }
}

#[cfg(test)]
mod command_workflow_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use common_net::msg::command::{CommandKindV1, CommandRefusalV1};

    fn binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn descriptor(seed: u8) -> CommandDescriptorV1 {
        CommandDescriptorV1 {
            binding: binding(),
            command_id: CommandId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            kind: CommandKindV1::CharacterLifecycle,
            request_digest: [1; 32],
        }
    }

    /// `CMD-107`/`CMD-115`: a retry does not queue a second worker
    /// action, because the effect id is the command's identity.
    #[test]
    fn a_retry_does_not_queue_a_second_worker_action() {
        let mut workflows = PendingWorkflowsV1::new();
        let d = descriptor(10);
        let first = CommandContextV1::for_command_v1(&d, 1).unwrap();
        let retry = CommandContextV1::for_command_v1(&d, 1).unwrap();
        assert_eq!(first.effect_id, retry.effect_id, "the effect id must be stable across retries");

        workflows.enqueue_v1(first).unwrap();
        assert_eq!(workflows.enqueue_v1(retry).unwrap_err(), WorkflowErrorV1::AlreadyQueued);
        assert_eq!(workflows.len(), 1);
    }

    /// `CMD-109`/`CMD-110`/`CMD-111`: answers are keyed by effect id, and
    /// every answer terminates the command — a lost channel or a panic
    /// can never become a synthetic success.
    #[test]
    fn every_worker_answer_terminates_and_none_invents_success() {
        let mut workflows = PendingWorkflowsV1::new();
        let d = descriptor(11);
        let context = CommandContextV1::for_command_v1(&d, 1).unwrap();
        workflows.enqueue_v1(context).unwrap();

        // an answer for a workflow nobody queued lands nowhere
        assert_eq!(
            workflows.resolve_v1(&[0xAA; 32], WorkerAnswerV1::Completed(CommandOutcomeV1::Applied { result_digest: [1; 32] })).unwrap_err(),
            WorkflowErrorV1::Unknown
        );

        let (resolved, outcome) = workflows.resolve_v1(&context.effect_id, WorkerAnswerV1::ChannelLost).unwrap();
        assert_eq!(resolved.descriptor.command_id, d.command_id);
        assert_eq!(outcome, CommandOutcomeV1::Refused { reason: CommandRefusalV1::Unprocessable });
        assert!(workflows.is_empty(), "a resolved workflow is no longer pending");

        // ...and a late second answer cannot be misattributed
        assert_eq!(
            workflows.resolve_v1(&context.effect_id, WorkerAnswerV1::Completed(CommandOutcomeV1::Applied { result_digest: [2; 32] })).unwrap_err(),
            WorkflowErrorV1::Unknown
        );

        for (answer, expected) in [
            (WorkerAnswerV1::Panicked, CommandRefusalV1::Unprocessable),
            (WorkerAnswerV1::TimedOut, CommandRefusalV1::PreconditionFailed),
        ] {
            let d = descriptor(12);
            let c = CommandContextV1::for_command_v1(&d, 1).unwrap();
            workflows.enqueue_v1(c).unwrap();
            let (_, outcome) = workflows.resolve_v1(&c.effect_id, answer).unwrap();
            assert_eq!(outcome, CommandOutcomeV1::Refused { reason: expected });
        }
    }

    /// `CMD-114`: a workflow whose effect may still commit is not
    /// dropped when the session closes.
    #[test]
    fn an_in_flight_workflow_is_not_dropped_by_session_close() {
        let mut workflows = PendingWorkflowsV1::new();
        let d = descriptor(13);
        let context = CommandContextV1::for_command_v1(&d, 1).unwrap();
        workflows.enqueue_v1(context).unwrap();

        assert_eq!(workflows.close_session_v1(&binding()).unwrap_err(), WorkflowErrorV1::StillInFlight);
        assert_eq!(workflows.len(), 1, "the workflow must survive until its effect is settled");

        workflows.resolve_v1(&context.effect_id, WorkerAnswerV1::TimedOut).unwrap();
        assert_eq!(workflows.close_session_v1(&binding()).unwrap(), 0);
    }
}
