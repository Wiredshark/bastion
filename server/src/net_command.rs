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
    CommandOutcomeV1, CommandReceiptV1, JournalErrorV1, ReceiptErrorV1, command_descriptor_from_frame_v1,
    execute_command_once_v1,
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
