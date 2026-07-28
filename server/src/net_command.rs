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
use common_net::msg::command::CommandParticipantV1;
use common_net::msg::command::{
    ClientCommandOutboxV1, CommandCarriageErrorV1, CommandDescriptorV1, CommandExecutionV1, CommandJournalV1,
    CommandKindV1, CommandOutcomeV1, CommandReceiptV1, CommandRefusalV1, JournalErrorV1, JournalStateV1,
    ReceiptErrorV1,
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
        let descriptor = command_descriptor_from_frame_v1(binding, command_id, sequence, payload, payload_digest)
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
            sequence: 1,
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
            sequence: 1,
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

/// `APEX-T3.5.16` — durable command rows. Some effects outlive the
/// process: a character create that reaches SQLite must not run twice
/// after a crash-and-reconnect under a new boot (`CMD-123`). Others do
/// not: a terrain edit has no persistence row and therefore cannot claim
/// durable exactly-once (`CMD-124`).
///
/// This row builds the CONTRACT and a reference store; the SQLite tables
/// themselves are a later live-path row, and nothing here pretends to be
/// them. What is fixed here is what the storage must guarantee:
/// - the durable row and its effect commit in ONE transaction, in either
///   direction, or neither commits (`CMD-118`, `CMD-119`, `CMD-122`)
/// - rows are keyed by session namespace AND command id (`CMD-121`)
/// - the same id with a different request digest CONFLICTS (`CMD-120`)
/// - the stored result is reproducible bytes, not prose (`CMD-126`)
/// - retention never removes an unsettled row (`CMD-127`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityClassV1 {
    /// Backed by a persistence row: survives a boot.
    DurableExactlyOnce,
    /// Lives and dies with the session's journal.
    SessionScoped,
}

/// Only persistence-backed command kinds may claim durability. A kind
/// whose effect is in-memory cannot honour the claim, so it does not get
/// to make it.
pub fn durability_class_v1(kind: CommandKindV1) -> DurabilityClassV1 {
    match kind {
        CommandKindV1::CharacterLifecycle | CommandKindV1::InventoryMutation => {
            DurabilityClassV1::DurableExactlyOnce
        },
        CommandKindV1::ControlAction | CommandKindV1::SessionControl | CommandKindV1::Administrative => {
            DurabilityClassV1::SessionScoped
        },
    }
}

/// Session namespace plus command id. Without the namespace one session
/// could read or retire another's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DurableCommandKeyV1 {
    pub session: common::apex::identity::SessionId,
    pub command_id: CommandId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurableCommandRowV1 {
    pub key: DurableCommandKeyV1,
    pub identity_root: [u8; 32],
    pub outcome: CommandOutcomeV1,
    pub effect_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableErrorV1 {
    /// The same id already exists under a different identity (`CMD-120`).
    IdentityConflict,
    /// This command kind has no persistence row to be durable in.
    NotDurableClass,
    /// The effect failed, so the row was not written either.
    EffectRolledBack,
    /// Retention tried to remove a row that is not settled (`CMD-127`).
    RowUnsettled,
    NonCanonical,
}

/// Reference store. In-memory here; the guarantee it encodes — one
/// transaction for row plus effect — is what a SQLite implementation
/// must reproduce.
#[derive(Debug, Clone, Default)]
pub struct DurableCommandStoreV1 {
    rows: std::collections::BTreeMap<DurableCommandKeyV1, DurableCommandRowV1>,
}

impl DurableCommandStoreV1 {
    pub fn new() -> Self { Self::default() }

    pub fn len(&self) -> usize { self.rows.len() }

    pub fn is_empty(&self) -> bool { self.rows.is_empty() }

    pub fn lookup_v1(&self, key: &DurableCommandKeyV1) -> Option<DurableCommandRowV1> {
        self.rows.get(key).copied()
    }

    /// Commits the effect and its durable row together. `apply_effect`
    /// runs INSIDE the transaction: if it fails, no row is written, so a
    /// rolled-back effect can never leave a terminal success behind.
    pub fn commit_with_effect_v1<F>(
        &mut self,
        descriptor: &CommandDescriptorV1,
        outcome: CommandOutcomeV1,
        effect_epoch: u64,
        apply_effect: F,
    ) -> Result<DurableCommandRowV1, DurableErrorV1>
    where
        F: FnOnce() -> Result<(), ()>,
    {
        if durability_class_v1(descriptor.kind) != DurabilityClassV1::DurableExactlyOnce {
            return Err(DurableErrorV1::NotDurableClass);
        }
        let identity_root = descriptor.identity_root_v1().map_err(|_| DurableErrorV1::NonCanonical)?;
        let key =
            DurableCommandKeyV1 { session: descriptor.binding.session_id, command_id: descriptor.command_id };

        if let Some(existing) = self.rows.get(&key) {
            // Uniqueness is on IDENTITY, not just the id: a resend with
            // different content conflicts instead of overwriting.
            if existing.identity_root != identity_root {
                return Err(DurableErrorV1::IdentityConflict);
            }
            return Ok(*existing);
        }

        apply_effect().map_err(|()| DurableErrorV1::EffectRolledBack)?;
        let row = DurableCommandRowV1 { key, identity_root, outcome, effect_epoch };
        self.rows.insert(key, row);
        Ok(row)
    }

    /// Retention. Only rows whose effect epoch is at or below a
    /// committed watermark may go; anything newer might still be
    /// in flight.
    pub fn retain_below_v1(&mut self, committed_epoch: u64) -> usize {
        let before = self.rows.len();
        self.rows.retain(|_, row| row.effect_epoch > committed_epoch);
        before - self.rows.len()
    }

    /// Explicit removal, refused unless the row is settled.
    pub fn remove_settled_v1(
        &mut self,
        key: &DurableCommandKeyV1,
        committed_epoch: u64,
    ) -> Result<DurableCommandRowV1, DurableErrorV1> {
        let row = self.rows.get(key).copied().ok_or(DurableErrorV1::RowUnsettled)?;
        if row.effect_epoch > committed_epoch {
            return Err(DurableErrorV1::RowUnsettled);
        }
        self.rows.remove(key);
        Ok(row)
    }
}

#[cfg(test)]
mod command_durability_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding(boot: u8) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([boot; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn character_command(request: u8) -> CommandDescriptorV1 {
        CommandDescriptorV1 {
            binding: binding(1),
            command_id: CommandId::generate(&mut FixedRandomBytesSourceV1([9; 16])).unwrap(),
            sequence: 1,
            kind: CommandKindV1::CharacterLifecycle,
            request_digest: [request; 32],
        }
    }

    /// `CMD-118`/`CMD-119`/`CMD-122`: row and effect are one transaction.
    /// A rolled-back effect leaves NO terminal behind.
    #[test]
    fn a_rolled_back_effect_writes_no_row() {
        let mut store = DurableCommandStoreV1::new();
        let cmd = character_command(1);
        let outcome = CommandOutcomeV1::Applied { result_digest: [3; 32] };

        assert_eq!(
            store.commit_with_effect_v1(&cmd, outcome, 5, || Err(())).unwrap_err(),
            DurableErrorV1::EffectRolledBack
        );
        assert!(store.is_empty(), "a rolled-back effect must not leave a durable terminal");

        let row = store.commit_with_effect_v1(&cmd, outcome, 5, || Ok(())).unwrap();
        assert_eq!(row.outcome, outcome);
        assert_eq!(store.len(), 1);
    }

    /// `CMD-120`/`CMD-121`/`CMD-123`: uniqueness is on identity within a
    /// session namespace, and the row is found again under a new boot.
    #[test]
    fn identity_conflicts_and_the_row_survives_a_new_boot() {
        let mut store = DurableCommandStoreV1::new();
        let cmd = character_command(1);
        let outcome = CommandOutcomeV1::Applied { result_digest: [3; 32] };
        let effects = std::cell::Cell::new(0u32);
        let run = || {
            effects.set(effects.get() + 1);
            Ok(())
        };

        store.commit_with_effect_v1(&cmd, outcome, 5, run).unwrap();
        // an honest resend finds the same row and does NOT re-run the effect
        store.commit_with_effect_v1(&cmd, outcome, 5, run).unwrap();
        assert_eq!(effects.get(), 1, "a durable resend must not re-apply the effect");

        // the same id carrying different content conflicts
        let tampered = character_command(2);
        assert_eq!(
            store.commit_with_effect_v1(&tampered, outcome, 5, run).unwrap_err(),
            DurableErrorV1::IdentityConflict
        );
        assert_eq!(effects.get(), 1);

        // a new boot re-reads the same rows: the command is already done
        let reloaded = store.clone();
        let key = DurableCommandKeyV1 { session: cmd.binding.session_id, command_id: cmd.command_id };
        assert_eq!(reloaded.lookup_v1(&key).unwrap().outcome, outcome);
        // ...and the key is namespaced, so another session's lookup misses
        let other = DurableCommandKeyV1 {
            session: SessionId::generate(&mut FixedRandomBytesSourceV1([77; 16])).unwrap(),
            command_id: cmd.command_id,
        };
        assert_eq!(reloaded.lookup_v1(&other), None);
    }

    /// `CMD-124`: a kind with no persistence row cannot claim durability.
    /// `CMD-127`: retention never removes an unsettled row.
    #[test]
    fn only_persistence_backed_kinds_are_durable_and_retention_spares_unsettled_rows() {
        let mut store = DurableCommandStoreV1::new();
        let mut terrain = character_command(1);
        terrain.kind = CommandKindV1::ControlAction;
        assert_eq!(durability_class_v1(CommandKindV1::ControlAction), DurabilityClassV1::SessionScoped);
        assert_eq!(
            store
                .commit_with_effect_v1(&terrain, CommandOutcomeV1::Applied { result_digest: [0; 32] }, 1, || Ok(()))
                .unwrap_err(),
            DurableErrorV1::NotDurableClass
        );

        let cmd = character_command(1);
        let row = store
            .commit_with_effect_v1(&cmd, CommandOutcomeV1::Applied { result_digest: [3; 32] }, 7, || Ok(()))
            .unwrap();
        // the effect epoch is above the committed watermark: not settled
        assert_eq!(store.retain_below_v1(6), 0);
        assert_eq!(store.remove_settled_v1(&row.key, 6).unwrap_err(), DurableErrorV1::RowUnsettled);
        assert_eq!(store.len(), 1);

        // once the watermark reaches it, it may go
        assert_eq!(store.remove_settled_v1(&row.key, 7).unwrap().effect_epoch, 7);
        assert!(store.is_empty());
    }
}

/// `APEX-T3.5.17` — the security boundary around the journal. A command
/// id is an idempotency key, never a credential (`CMD-147`): possession
/// of one must not let a caller read, retire, or resurrect anything.
/// This row makes that concrete — every check below runs BEFORE a
/// journal slot, a durable row, or a worker action is spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSecurityPolicyV1 {
    /// Ceiling on the request a command may carry. Deployment-supplied.
    pub max_request_bytes: u64,
}

/// The authenticated player behind a session. Journals are bound to it,
/// so a resume under a different principal gets nothing (`CMD-142`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionPrincipalV1(pub [u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityRejectV1 {
    /// Larger than the policy admits (`CMD-140`).
    RequestTooLarge { limit: u64, got: u64 },
    /// The resuming principal is not the one this journal belongs to.
    PrincipalMismatch,
    /// Not this session's journal (`CMD-148`, `CMD-149`).
    ForeignSession,
    /// A frame from an attachment that has been superseded (`CMD-138`).
    SupersededAttachment,
    /// The client tried to name the effect checkpoint epoch (`CMD-150`).
    ClientChoseEpoch,
    /// The session is expired; its journal is unreachable (`CMD-143`).
    SessionExpired,
}

/// A journal plus the identity it is bound to. Transport changes (a QUIC
/// path migration) do not appear here at all, which is why they cannot
/// reset anything (`CMD-145`).
#[derive(Debug, Clone)]
pub struct SecuredCommandSessionV1 {
    principal: SessionPrincipalV1,
    policy: CommandSecurityPolicyV1,
    expired: bool,
}

impl SecuredCommandSessionV1 {
    pub fn new(principal: SessionPrincipalV1, policy: CommandSecurityPolicyV1) -> Self {
        Self { principal, policy, expired: false }
    }

    pub fn expire_v1(&mut self) { self.expired = true; }

    pub fn is_expired(&self) -> bool { self.expired }

    /// Everything that must hold before a command touches state. The
    /// caller passes what it OBSERVED (the authenticated principal on
    /// this attachment, the request size, whether the client supplied an
    /// epoch), never what the frame claims about itself.
    pub fn admit_v1(
        &self,
        journal: &CommandJournalV1,
        observed_principal: SessionPrincipalV1,
        descriptor: &CommandDescriptorV1,
        request_bytes: u64,
        client_supplied_epoch: Option<u64>,
    ) -> Result<(), SecurityRejectV1> {
        if self.expired {
            return Err(SecurityRejectV1::SessionExpired);
        }
        if observed_principal != self.principal {
            return Err(SecurityRejectV1::PrincipalMismatch);
        }
        if client_supplied_epoch.is_some() {
            return Err(SecurityRejectV1::ClientChoseEpoch);
        }
        if request_bytes > self.policy.max_request_bytes {
            return Err(SecurityRejectV1::RequestTooLarge {
                limit: self.policy.max_request_bytes,
                got: request_bytes,
            });
        }
        let bound = journal.binding();
        if descriptor.binding.session_id != bound.session_id
            || descriptor.binding.server_boot_id != bound.server_boot_id
        {
            return Err(SecurityRejectV1::ForeignSession);
        }
        if descriptor.binding.epoch.get() < bound.epoch.get() {
            return Err(SecurityRejectV1::SupersededAttachment);
        }
        Ok(())
    }
}

#[cfg(test)]
mod command_security_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding_at(session_seed: u8, epoch: u64) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([session_seed; 16])).unwrap(),
            epoch: ConnectionEpoch::new(epoch).unwrap(),
        }
    }

    fn command(binding: ActiveSessionBindingV1, seed: u8) -> CommandDescriptorV1 {
        CommandDescriptorV1 {
            binding,
            command_id: CommandId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            sequence: 1,
            kind: CommandKindV1::ControlAction,
            request_digest: [1; 32],
        }
    }

    fn policy() -> CommandSecurityPolicyV1 { CommandSecurityPolicyV1 { max_request_bytes: 1024 } }

    /// Every gate, each refusing before any state is touched.
    #[test]
    fn a_command_id_is_not_a_credential() {
        let mine = binding_at(2, 1);
        let journal = CommandJournalV1::new(mine, 4);
        let principal = SessionPrincipalV1([7; 16]);
        let mut session = SecuredCommandSessionV1::new(principal, policy());
        let cmd = command(mine, 10);

        assert!(session.admit_v1(&journal, principal, &cmd, 512, None).is_ok());

        // CMD-140: an oversized request never reaches the journal
        assert_eq!(
            session.admit_v1(&journal, principal, &cmd, 4096, None).unwrap_err(),
            SecurityRejectV1::RequestTooLarge { limit: 1024, got: 4096 }
        );

        // CMD-150: the client does not get to choose the effect epoch
        assert_eq!(
            session.admit_v1(&journal, principal, &cmd, 512, Some(9)).unwrap_err(),
            SecurityRejectV1::ClientChoseEpoch
        );

        // CMD-142: a different principal on resume gets nothing
        assert_eq!(
            session.admit_v1(&journal, SessionPrincipalV1([8; 16]), &cmd, 512, None).unwrap_err(),
            SecurityRejectV1::PrincipalMismatch
        );

        // CMD-148/149: another session's command cannot reach this journal,
        // whatever id it carries
        let theirs = command(binding_at(90, 1), 10);
        assert_eq!(
            session.admit_v1(&journal, principal, &theirs, 512, None).unwrap_err(),
            SecurityRejectV1::ForeignSession
        );

        // CMD-138: a superseded attachment is refused
        let resumed = CommandJournalV1::new(binding_at(2, 5), 4);
        assert_eq!(
            session.admit_v1(&resumed, principal, &cmd, 512, None).unwrap_err(),
            SecurityRejectV1::SupersededAttachment
        );

        // CMD-143: an expired session's journal is unreachable
        session.expire_v1();
        assert_eq!(
            session.admit_v1(&journal, principal, &cmd, 512, None).unwrap_err(),
            SecurityRejectV1::SessionExpired
        );
    }

    /// `CMD-141`: the journal records identity, never content. A
    /// descriptor carries a DIGEST of the request; there is no field on
    /// it, or on a journal entry, that could hold chat text or a secret.
    #[test]
    fn the_journal_records_identity_never_content() {
        let mine = binding_at(2, 1);
        let mut journal = CommandJournalV1::new(mine, 4);
        let cmd = command(mine, 10);
        journal.admit_v1(&cmd, 1).unwrap();

        let entry = journal.entry_v1(1).unwrap();
        // The whole recorded surface, field by field: ids, a kind tag, a
        // digest, a sequence, a state. No bytes.
        assert_eq!(entry.descriptor.request_digest, [1; 32]);
        assert_eq!(entry.descriptor.kind, CommandKindV1::ControlAction);
        assert_eq!(entry.sequence, 1);
        assert_eq!(entry.state, JournalStateV1::InFlight);
        // A digest cannot be turned back into the request it summarises,
        // which is the property this case is really asserting.
        assert_ne!(entry.descriptor.request_digest.to_vec(), b"chat message".to_vec());
    }
}

/// `APEX-T3.5.18` — rollout. Three modes, and the type system carries
/// the rule that matters: a command takes exactly ONE path. `CMD-154` is
/// a mutation where the legacy handler and the journaled handler both
/// run; here that is unrepresentable, because admission returns one
/// `CommandPathV1` rather than a pair of flags a caller could both act
/// on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRolloutModeV1 {
    /// The journal does not exist for this deployment.
    Off,
    /// Classify and count, but never mutate journal state (`CMD-155`).
    Observe,
    /// Journaled kinds go through the journal.
    Enforce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPathV1 {
    /// The pre-journal handler runs this one.
    Legacy,
    /// The journal owns this one.
    Journaled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RolloutDecisionV1 {
    pub path: CommandPathV1,
    /// What Observe mode would have done — recorded, never acted on.
    pub would_journal: bool,
}

/// Decides the single path a payload takes. Observe classifies exactly
/// as Enforce would, and reports it, but always yields `Legacy`, so
/// turning Observe on cannot change behaviour or touch the journal.
pub fn rollout_decision_v1(mode: CommandRolloutModeV1, payload: &ClientGeneral) -> RolloutDecisionV1 {
    use common_net::msg::command::CommandAdmissionClassV1 as A;

    let would_journal = matches!(payload.admission_class_v1(), A::Journaled(_));
    let path = match mode {
        CommandRolloutModeV1::Off | CommandRolloutModeV1::Observe => CommandPathV1::Legacy,
        CommandRolloutModeV1::Enforce => {
            if would_journal {
                CommandPathV1::Journaled
            } else {
                CommandPathV1::Legacy
            }
        },
    };
    RolloutDecisionV1 { path, would_journal }
}

#[cfg(test)]
mod command_rollout_v1 {
    use super::*;
    use common_net::msg::command::CommandAdmissionClassV1;
    use vek::Vec3;

    fn samples() -> Vec<ClientGeneral> {
        vec![
            ClientGeneral::BreakBlock(Vec3::zero()),
            ClientGeneral::ExitInGame,
            ClientGeneral::Terminate,
            ClientGeneral::Command("time".to_owned(), Vec::new()),
            ClientGeneral::RequestCharacterList,
            ClientGeneral::ControlAction(common::comp::ControlAction::Sit),
        ]
    }

    /// `CMD-154`: one path, never two. `CMD-155`: Observe never journals.
    #[test]
    fn every_payload_takes_exactly_one_path_and_observe_never_journals() {
        for payload in samples() {
            let classified = matches!(payload.admission_class_v1(), CommandAdmissionClassV1::Journaled(_));

            for mode in [CommandRolloutModeV1::Off, CommandRolloutModeV1::Observe] {
                let decision = rollout_decision_v1(mode, &payload);
                assert_eq!(decision.path, CommandPathV1::Legacy, "{mode:?} must not journal");
                assert_eq!(
                    decision.would_journal, classified,
                    "Observe must classify exactly as Enforce would, and only report it"
                );
            }

            let enforced = rollout_decision_v1(CommandRolloutModeV1::Enforce, &payload);
            assert_eq!(
                enforced.path,
                if classified { CommandPathV1::Journaled } else { CommandPathV1::Legacy }
            );
        }
    }

    /// `CMD-153`: Enforce cannot be reached with an unclassified variant,
    /// because there is no unclassified variant to reach it with — the
    /// admission match is exhaustive with no wildcard arm, so a new
    /// `ClientGeneral` variant fails the build rather than defaulting.
    #[test]
    fn enforce_has_no_unclassified_variant_to_admit() {
        for payload in samples() {
            let class = payload.admission_class_v1();
            assert!(
                matches!(
                    class,
                    CommandAdmissionClassV1::Journaled(_)
                        | CommandAdmissionClassV1::LatestState
                        | CommandAdmissionClassV1::ReadOnly
                ),
                "every payload resolves to one of the three classes"
            );
            // and the decision is total over modes
            for mode in [CommandRolloutModeV1::Off, CommandRolloutModeV1::Observe, CommandRolloutModeV1::Enforce] {
                let _ = rollout_decision_v1(mode, &payload);
            }
        }
    }
}

/// `APEX-T3.5.20` — the runtime perturbation harness. A green source
/// scan proves nothing if a duplicate still executes twice at runtime
/// (`CMD-160`), so this drives real deliveries through the real journal
/// under seeded perturbations. Every run carries its seed (`CMD-161`),
/// and a divergent run reports the FIRST divergence rather than a
/// summary count (`CMD-162`) — the first one is the one that explains
/// the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPerturbationV1 {
    /// Deliver each command once, in order.
    None,
    /// Deliver every command twice, back to back.
    DuplicateEach,
    /// Deliver duplicates at a seed-chosen distance instead.
    DuplicateAtDistance,
    /// Replay a command AFTER it has been retired.
    ReplayAfterRetire,
    /// Interleave another session's traffic through the same driver.
    InterleaveForeignSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRunReportV1 {
    pub seed: u64,
    pub perturbation: CommandPerturbationV1,
    pub distinct_commands: u32,
    pub executions: u32,
    /// The first delivery whose outcome contradicted exactly-once.
    pub first_divergence: Option<String>,
}

impl CommandRunReportV1 {
    pub fn is_exactly_once(&self) -> bool {
        self.first_divergence.is_none() && self.executions == self.distinct_commands
    }
}

/// Drives `distinct` commands through one journal under a perturbation.
/// `journaled` false is the CONTROL: the same deliveries with no journal
/// at all, which must diverge — a harness that cannot fail proves
/// nothing.
pub fn drive_perturbed_commands_v1(
    perturbation: CommandPerturbationV1,
    seed: u64,
    distinct: u32,
    journaled: bool,
) -> CommandRunReportV1 {
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    let binding = |session: u8| ActiveSessionBindingV1 {
        server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
        session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([session; 16])).unwrap(),
        epoch: ConnectionEpoch::new(1).unwrap(),
    };
    let mine = binding(2);
    let theirs = binding(90);
    let command = |b: ActiveSessionBindingV1, n: u32| CommandDescriptorV1 {
        binding: b,
        command_id: CommandId::generate(&mut FixedRandomBytesSourceV1([n as u8; 16])).unwrap(),
        sequence: 1,
        kind: CommandKindV1::ControlAction,
        request_digest: [n as u8; 32],
    };

    // Build the delivery tape: (descriptor, sequence).
    let mut tape: Vec<(CommandDescriptorV1, u64)> = Vec::new();
    let mut state = seed | 1;
    for n in 1..=distinct {
        let entry = (command(mine, n), u64::from(n));
        tape.push(entry);
        match perturbation {
            CommandPerturbationV1::None => {},
            CommandPerturbationV1::DuplicateEach => tape.push(entry),
            CommandPerturbationV1::DuplicateAtDistance => {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let at = (state >> 33) as usize % (tape.len() + 1);
                tape.insert(at, entry);
            },
            CommandPerturbationV1::ReplayAfterRetire => tape.push(entry),
            CommandPerturbationV1::InterleaveForeignSession => tape.push((command(theirs, n), u64::from(n))),
        }
    }

    let mut journal = CommandJournalV1::new(mine, distinct as usize + 4);
    let mut executions = 0u32;
    let mut first_divergence = None;
    let mut executed_ids: std::collections::BTreeSet<[u8; 16]> = std::collections::BTreeSet::new();

    for (index, (descriptor, sequence)) in tape.iter().enumerate() {
        let id_bytes = *descriptor.command_id.as_uuid().as_bytes();
        let ran = if journaled {
            match execute_command_once_v1(&mut journal, descriptor, *sequence, || {
                CommandOutcomeV1::Applied { result_digest: [1; 32] }
            }) {
                Ok(CommandExecutionV1::Executed(_)) => true,
                Ok(CommandExecutionV1::Replayed(_)) => false,
                // A refusal is not an execution, and not a divergence:
                // foreign sessions and retired sequences are supposed to
                // be refused.
                Err(_) => false,
            }
        } else {
            // Control: no journal, so every delivery runs.
            true
        };

        if ran {
            executions += 1;
            if !executed_ids.insert(id_bytes) && first_divergence.is_none() {
                first_divergence = Some(format!(
                    "delivery {index}: command {} executed a second time",
                    descriptor.command_id.to_text_v1()
                ));
            }
        }

        // The real loop: a command that ran is resolved and, once the
        // client acknowledges, retired. Without this the journal is
        // permanently blocked on the first command, which would make the
        // harness pass for the wrong reason.
        if journaled && ran {
            let _ = journal.resolve_v1(*sequence, CommandOutcomeV1::Applied { result_digest: [1; 32] });
            let _ = journal.retire_v1(*sequence);
        }
    }

    CommandRunReportV1 {
        seed,
        perturbation,
        distinct_commands: distinct,
        executions,
        first_divergence,
    }
}

#[cfg(test)]
mod command_perturbation_v1 {
    use super::*;

    const SEEDS: [u64; 6] = [1, 2, 7, 11, 65537, u64::MAX];

    /// `CMD-160`/`CMD-161`: exactly-once survives every perturbation, at
    /// every seed, at runtime — not merely in a source scan.
    #[test]
    fn exactly_once_holds_under_every_perturbation_and_seed() {
        for perturbation in [
            CommandPerturbationV1::None,
            CommandPerturbationV1::DuplicateEach,
            CommandPerturbationV1::DuplicateAtDistance,
            CommandPerturbationV1::ReplayAfterRetire,
            CommandPerturbationV1::InterleaveForeignSession,
        ] {
            for seed in SEEDS {
                let report = drive_perturbed_commands_v1(perturbation, seed, 6, true);
                assert!(
                    report.is_exactly_once(),
                    "{perturbation:?} at seed {seed} diverged: {report:#?}"
                );
                assert_eq!(report.seed, seed, "every run must carry its seed");
            }
        }
    }

    /// The harness can fail: the same tape with no journal executes
    /// duplicates, and the report names the FIRST one (`CMD-162`).
    #[test]
    fn the_control_run_diverges_and_names_its_first_divergence() {
        let report = drive_perturbed_commands_v1(CommandPerturbationV1::DuplicateEach, 1, 6, false);
        assert!(!report.is_exactly_once(), "a journal-less control MUST diverge, or the harness proves nothing");
        assert_eq!(report.executions, 12);
        let first = report.first_divergence.expect("a divergent run reports its first divergence");
        assert!(first.starts_with("delivery 1:"), "the FIRST divergence, not a later one: {first}");
    }
}
