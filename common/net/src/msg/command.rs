//! `APEX-T3.5` — command identity and exactly-once application. A
//! command's identity is its `CommandId` bound to the session that
//! issued it and to the REQUEST BYTES it carried; a replay is only a
//! replay if all three agree. Same-id-different-request is a conflict,
//! not a duplicate, and is refused rather than silently re-applied.
//!
//! Dormant: `CommandIdUnsupported` still rejects every `Some(command_id)`
//! at both ingress paths, so nothing here is on a live route yet.

use common::apex::digest::{DigestDomainIdV1, ProtocolDigestV1, digest_canonical_bytes_v1};
use common::apex::identity::CommandId;
use serde::{Deserialize, Serialize};

use super::envelope::ActiveSessionBindingV1;

const COMMAND_ROOT_INPUT_LIMIT: u64 = 1 << 20;

/// The frozen set of command kinds. Explicit tags: a command's kind is
/// protocol-visible identity, never a Rust discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u16)]
#[serde(into = "u16", try_from = "u16")]
pub enum CommandKindV1 {
    /// A client-issued gameplay control action.
    ControlAction = 1,
    /// A client-issued inventory/trade mutation.
    InventoryMutation = 2,
    /// A client-issued character-lifecycle request.
    CharacterLifecycle = 3,
    /// A client-issued session-control request (terminate/resync).
    SessionControl = 4,
    /// An administrative command routed through the chat command path.
    Administrative = 5,
}

impl CommandKindV1 {
    pub const ALL: [Self; 5] = [
        Self::ControlAction,
        Self::InventoryMutation,
        Self::CharacterLifecycle,
        Self::SessionControl,
        Self::Administrative,
    ];

    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const fn label(self) -> &'static str {
        match self {
            Self::ControlAction => "bastion/command/control-action/v1",
            Self::InventoryMutation => "bastion/command/inventory-mutation/v1",
            Self::CharacterLifecycle => "bastion/command/character-lifecycle/v1",
            Self::SessionControl => "bastion/command/session-control/v1",
            Self::Administrative => "bastion/command/administrative/v1",
        }
    }

    pub fn try_from_u16(raw: u16) -> Option<Self> { Self::ALL.into_iter().find(|k| k.as_u16() == raw) }
}

impl From<CommandKindV1> for u16 {
    fn from(k: CommandKindV1) -> u16 { k.as_u16() }
}

impl TryFrom<u16> for CommandKindV1 {
    type Error = &'static str;

    fn try_from(raw: u16) -> Result<Self, Self::Error> {
        Self::try_from_u16(raw).ok_or("unknown command kind tag")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandIdentityErrorV1 {
    NonCanonical,
    BindingMismatch,
    KindMismatch,
    RequestMismatch,
}

/// One command's identity. The request digest is part of it: a resend
/// must carry the SAME request, or it is a different command wearing a
/// used id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptorV1 {
    pub binding: ActiveSessionBindingV1,
    pub command_id: CommandId,
    pub kind: CommandKindV1,
    pub request_digest: [u8; 32],
}

impl CommandDescriptorV1 {
    /// Domain-separated identity root, recomputed by both sides — never
    /// taken from the wire.
    pub fn identity_root_v1(&self) -> Result<[u8; 32], CommandIdentityErrorV1> {
        let mut p = Vec::with_capacity(128);
        p.extend_from_slice(self.binding.session_id.as_uuid().as_bytes());
        p.extend_from_slice(&self.binding.epoch.get().to_be_bytes());
        p.extend_from_slice(self.binding.server_boot_id.as_uuid().as_bytes());
        p.extend_from_slice(self.command_id.as_uuid().as_bytes());
        p.extend_from_slice(&self.kind.as_u16().to_be_bytes());
        p.extend_from_slice(&self.request_digest);
        digest_canonical_bytes_v1(DigestDomainIdV1::CommandDescriptor, &p, COMMAND_ROOT_INPUT_LIMIT)
            .map(|d: ProtocolDigestV1| *d.bytes.as_array())
            .map_err(|_| CommandIdentityErrorV1::NonCanonical)
    }

    /// Whether `other` is a genuine REPLAY of this command: same
    /// binding, same id, same kind, same request bytes. Anything else
    /// that reuses the id is a conflict, and the error says which field
    /// broke — a caller must not collapse them into one "invalid".
    pub fn is_replay_of_v1(&self, other: &Self) -> Result<(), CommandIdentityErrorV1> {
        if self.binding != other.binding {
            return Err(CommandIdentityErrorV1::BindingMismatch);
        }
        if self.kind != other.kind {
            return Err(CommandIdentityErrorV1::KindMismatch);
        }
        if self.request_digest != other.request_digest {
            return Err(CommandIdentityErrorV1::RequestMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod command_identity_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding(seed: u8) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([seed + 1; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn descriptor() -> CommandDescriptorV1 {
        CommandDescriptorV1 {
            binding: binding(1),
            command_id: CommandId::generate(&mut FixedRandomBytesSourceV1([9; 16])).unwrap(),
            kind: CommandKindV1::ControlAction,
            request_digest: [7; 32],
        }
    }

    #[test]
    fn command_kind_tags_are_explicit_and_total() {
        for kind in CommandKindV1::ALL {
            assert_eq!(CommandKindV1::try_from_u16(kind.as_u16()), Some(kind));
        }
        assert_eq!(CommandKindV1::try_from_u16(0), None);
        assert_eq!(CommandKindV1::try_from_u16(6), None);
        let labels: std::collections::HashSet<&str> = CommandKindV1::ALL.iter().map(|k| k.label()).collect();
        assert_eq!(labels.len(), CommandKindV1::ALL.len(), "duplicate command kind label");
    }

    /// Identity binds every field: change any one and the root moves.
    #[test]
    fn identity_root_binds_binding_id_kind_and_request() {
        let base = descriptor();
        let root = base.identity_root_v1().unwrap();
        assert_eq!(root, descriptor().identity_root_v1().unwrap(), "identity must be reproducible");

        let mut other_binding = base;
        other_binding.binding = binding(50);
        let mut other_kind = base;
        other_kind.kind = CommandKindV1::Administrative;
        let mut other_request = base;
        other_request.request_digest = [8; 32];
        let mut other_id = base;
        other_id.command_id = CommandId::generate(&mut FixedRandomBytesSourceV1([11; 16])).unwrap();

        for mutated in [other_binding, other_kind, other_request, other_id] {
            assert_ne!(mutated.identity_root_v1().unwrap(), root);
        }
    }

    /// Reusing an id with a different request is a CONFLICT, and the
    /// error names the field that broke.
    #[test]
    fn a_reused_id_with_different_content_is_a_conflict_not_a_duplicate() {
        let base = descriptor();
        assert!(base.is_replay_of_v1(&descriptor()).is_ok());

        let mut request = base;
        request.request_digest = [8; 32];
        assert_eq!(base.is_replay_of_v1(&request), Err(CommandIdentityErrorV1::RequestMismatch));

        let mut kind = base;
        kind.kind = CommandKindV1::SessionControl;
        assert_eq!(base.is_replay_of_v1(&kind), Err(CommandIdentityErrorV1::KindMismatch));

        let mut other = base;
        other.binding = binding(50);
        assert_eq!(base.is_replay_of_v1(&other), Err(CommandIdentityErrorV1::BindingMismatch));
    }
}

/// `APEX-T3.5.02` — per-session command ledger. Exactly-once, not
/// at-least-once: a resolved command replays its ORIGINAL outcome, and
/// the ledger never forgets an id it has admitted. It is bounded, and
/// exhaustion is a typed refusal rather than an eviction — evicting a
/// resolved entry would silently downgrade the guarantee, because a
/// later replay of the forgotten id would read as fresh and apply twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandOutcomeV1 {
    Applied { result_digest: [u8; 32] },
    Refused { reason: CommandRefusalV1 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandRefusalV1 {
    NotPermitted,
    Unprocessable,
    PreconditionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAdmitV1 {
    /// Never seen: the caller may execute it.
    Fresh,
    /// Admitted earlier and still executing — the caller must not start
    /// a second execution.
    InFlight,
    /// Already resolved: the caller returns THIS outcome, unchanged.
    Resolved(CommandOutcomeV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLedgerErrorV1 {
    BindingMismatch,
    /// The id is known but the command is not the same command.
    Conflict(CommandIdentityErrorV1),
    /// The bounded window is full.
    WindowExhausted,
    UnknownCommand,
    AlreadyResolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LedgerEntryV1 {
    descriptor: CommandDescriptorV1,
    outcome: Option<CommandOutcomeV1>,
}

#[derive(Debug, Clone)]
pub struct CommandLedgerV1 {
    binding: ActiveSessionBindingV1,
    capacity: usize,
    entries: std::collections::BTreeMap<CommandId, LedgerEntryV1>,
}

impl CommandLedgerV1 {
    /// `capacity` is deployment-supplied, like every other limit in this
    /// program — there is no invented default.
    pub fn new(binding: ActiveSessionBindingV1, capacity: usize) -> Self {
        Self { binding, capacity, entries: std::collections::BTreeMap::new() }
    }

    pub fn binding(&self) -> ActiveSessionBindingV1 { self.binding }

    pub fn len(&self) -> usize { self.entries.len() }

    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    pub fn resolved_count(&self) -> usize { self.entries.values().filter(|e| e.outcome.is_some()).count() }

    /// Classifies a command against everything this session has seen.
    /// Admitting a fresh command RESERVES its id, so a concurrent
    /// duplicate cannot also read as fresh.
    pub fn admit_v1(&mut self, descriptor: &CommandDescriptorV1) -> Result<CommandAdmitV1, CommandLedgerErrorV1> {
        if descriptor.binding != self.binding {
            return Err(CommandLedgerErrorV1::BindingMismatch);
        }
        if let Some(entry) = self.entries.get(&descriptor.command_id) {
            entry.descriptor.is_replay_of_v1(descriptor).map_err(CommandLedgerErrorV1::Conflict)?;
            return Ok(match entry.outcome {
                Some(outcome) => CommandAdmitV1::Resolved(outcome),
                None => CommandAdmitV1::InFlight,
            });
        }
        if self.entries.len() >= self.capacity {
            return Err(CommandLedgerErrorV1::WindowExhausted);
        }
        self.entries.insert(descriptor.command_id, LedgerEntryV1 { descriptor: *descriptor, outcome: None });
        Ok(CommandAdmitV1::Fresh)
    }

    /// Records the outcome of a command admitted as `Fresh`. Resolving
    /// twice is refused: an outcome is written once and then only read.
    pub fn resolve_v1(
        &mut self,
        command_id: CommandId,
        outcome: CommandOutcomeV1,
    ) -> Result<(), CommandLedgerErrorV1> {
        let entry = self.entries.get_mut(&command_id).ok_or(CommandLedgerErrorV1::UnknownCommand)?;
        if entry.outcome.is_some() {
            return Err(CommandLedgerErrorV1::AlreadyResolved);
        }
        entry.outcome = Some(outcome);
        Ok(())
    }

    pub fn outcome_of_v1(&self, command_id: &CommandId) -> Option<CommandOutcomeV1> {
        self.entries.get(command_id).and_then(|e| e.outcome)
    }
}

#[cfg(test)]
mod command_ledger_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding(seed: u8) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([seed + 1; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn command(seed: u8, request: u8) -> CommandDescriptorV1 {
        CommandDescriptorV1 {
            binding: binding(1),
            command_id: CommandId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            kind: CommandKindV1::ControlAction,
            request_digest: [request; 32],
        }
    }

    /// The whole guarantee: a replay never executes twice, and reads back
    /// the ORIGINAL outcome.
    #[test]
    fn a_resolved_command_replays_its_original_outcome() {
        let mut ledger = CommandLedgerV1::new(binding(1), 4);
        let cmd = command(10, 1);

        assert_eq!(ledger.admit_v1(&cmd).unwrap(), CommandAdmitV1::Fresh);
        // reserved immediately: a concurrent duplicate must not also be Fresh
        assert_eq!(ledger.admit_v1(&cmd).unwrap(), CommandAdmitV1::InFlight);

        let outcome = CommandOutcomeV1::Applied { result_digest: [3; 32] };
        ledger.resolve_v1(cmd.command_id, outcome).unwrap();
        assert_eq!(ledger.admit_v1(&cmd).unwrap(), CommandAdmitV1::Resolved(outcome));
        assert_eq!(ledger.outcome_of_v1(&cmd.command_id), Some(outcome));

        // an outcome is written once, then only read
        assert_eq!(
            ledger
                .resolve_v1(cmd.command_id, CommandOutcomeV1::Refused { reason: CommandRefusalV1::NotPermitted })
                .unwrap_err(),
            CommandLedgerErrorV1::AlreadyResolved
        );
        assert_eq!(ledger.admit_v1(&cmd).unwrap(), CommandAdmitV1::Resolved(outcome), "the outcome must not move");
    }

    #[test]
    fn conflicts_binding_and_exhaustion_are_typed() {
        let mut ledger = CommandLedgerV1::new(binding(1), 2);
        let cmd = command(10, 1);
        ledger.admit_v1(&cmd).unwrap();

        // same id, different request bytes
        let mut forged = cmd;
        forged.request_digest = [2; 32];
        assert_eq!(
            ledger.admit_v1(&forged).unwrap_err(),
            CommandLedgerErrorV1::Conflict(CommandIdentityErrorV1::RequestMismatch)
        );

        // a command bound to a different session
        let mut foreign = command(11, 1);
        foreign.binding = binding(50);
        assert_eq!(ledger.admit_v1(&foreign).unwrap_err(), CommandLedgerErrorV1::BindingMismatch);

        // resolving something never admitted
        assert_eq!(
            ledger
                .resolve_v1(command(99, 1).command_id, CommandOutcomeV1::Applied { result_digest: [0; 32] })
                .unwrap_err(),
            CommandLedgerErrorV1::UnknownCommand
        );

        // the window is bounded and fails CLOSED -- it never forgets an
        // id to make room, which would let a replay read as fresh
        ledger.admit_v1(&command(11, 1)).unwrap();
        ledger.resolve_v1(command(10, 1).command_id, CommandOutcomeV1::Applied { result_digest: [1; 32] }).unwrap();
        ledger.resolve_v1(command(11, 1).command_id, CommandOutcomeV1::Applied { result_digest: [2; 32] }).unwrap();
        assert_eq!(ledger.admit_v1(&command(12, 1)).unwrap_err(), CommandLedgerErrorV1::WindowExhausted);
        assert_eq!(ledger.resolved_count(), 2);
        // ...and the oldest resolved command is still remembered exactly
        assert_eq!(
            ledger.admit_v1(&command(10, 1)).unwrap(),
            CommandAdmitV1::Resolved(CommandOutcomeV1::Applied { result_digest: [1; 32] })
        );
    }
}
