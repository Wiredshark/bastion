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

/// `APEX-T3.5.03` — which C2S payloads ARE commands. Exhaustive, no
/// wildcard arm: a new `ClientGeneral` variant must be classified
/// deliberately, the same discipline `CheckpointParticipantV1` holds on
/// the other direction. A command is a request that MUTATES; a query or
/// a stream request is not one, and carrying a command id on it is a
/// protocol error rather than harmless decoration.
pub trait CommandParticipantV1 {
    fn command_kind_v1(&self) -> Option<CommandKindV1>;
}

impl CommandParticipantV1 for super::ClientGeneral {
    fn command_kind_v1(&self) -> Option<CommandKindV1> {
        use CommandKindV1 as K;
        use super::ClientGeneral as C;
        match self {
            C::ControllerInputs(_)
            | C::ControlEvent(_)
            | C::ControlAction(_)
            | C::BreakBlock(_)
            | C::PlaceBlock(_, _)
            | C::PlayerPhysics { .. }
            | C::UnlockSkill(_)
            | C::SetBattleMode(_)
            | C::UpdateMapMarker(_)
            | C::SpectatePosition(_)
            | C::SpectateEntity(_)
            | C::BastionCameraAnchor(_)
            | C::BastionPlaceDesignation { .. }
            | C::BastionApplyInfluence { .. }
            | C::BastionContextAction { .. }
            | C::BastionSpawnColony { .. }
            | C::BastionCancelDesignation { .. } => Some(K::ControlAction),
            C::CreateCharacter { .. }
            | C::DeleteCharacter(_)
            | C::EditCharacter { .. }
            | C::Character(_, _)
            | C::Spectate(_)
            | C::ExitInGame => Some(K::CharacterLifecycle),
            C::Terminate => Some(K::SessionControl),
            C::Command(_, _) => Some(K::Administrative),
            // Queries, stream requests and acknowledgements mutate no
            // authoritative state: replaying one costs a recomputation,
            // never a double application.
            C::RequestCharacterList
            | C::SetViewDistance(_)
            | C::RequestSiteInfo(_)
            | C::RequestPlayerPhysics { .. }
            | C::RequestLossyTerrainCompression { .. }
            | C::TerrainChunkRequest { .. }
            | C::LodZoneRequest { .. }
            | C::ChatMsg(_)
            | C::RequestPlugins(_)
            | C::RequestPluginArtifacts(_)
            | C::BastionInspect { .. }
            | C::CheckpointCommitAck(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCarriageErrorV1 {
    /// A command payload arrived without a command id: it cannot be
    /// deduplicated, so it cannot be admitted.
    MissingCommandId,
    /// A non-command payload carried a command id.
    UnexpectedCommandId,
}

/// Builds a command's identity from a frame that has ALREADY passed
/// envelope validation. The request digest is the header's own payload
/// digest — the identity is over the bytes that actually arrived, not a
/// separately claimed hash.
pub fn command_descriptor_from_frame_v1(
    binding: ActiveSessionBindingV1,
    command_id: Option<CommandId>,
    payload: &super::ClientGeneral,
    payload_digest: [u8; 32],
) -> Result<Option<CommandDescriptorV1>, CommandCarriageErrorV1> {
    match (payload.command_kind_v1(), command_id) {
        (Some(kind), Some(command_id)) => {
            Ok(Some(CommandDescriptorV1 { binding, command_id, kind, request_digest: payload_digest }))
        },
        (Some(_), None) => Err(CommandCarriageErrorV1::MissingCommandId),
        (None, Some(_)) => Err(CommandCarriageErrorV1::UnexpectedCommandId),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
mod command_carriage_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn id() -> CommandId { CommandId::generate(&mut FixedRandomBytesSourceV1([3; 16])).unwrap() }

    /// A command id belongs on a command and nowhere else, in both
    /// directions.
    #[test]
    fn command_id_carriage_is_required_exactly_on_commands() {
        use super::super::ClientGeneral as C;

        let mutating = C::Terminate;
        let query = C::RequestCharacterList;
        assert_eq!(mutating.command_kind_v1(), Some(CommandKindV1::SessionControl));
        assert_eq!(query.command_kind_v1(), None);

        let descriptor = command_descriptor_from_frame_v1(binding(), Some(id()), &mutating, [4; 32]).unwrap().unwrap();
        assert_eq!(descriptor.kind, CommandKindV1::SessionControl);
        // identity is over the bytes that arrived, not a separate claim
        assert_eq!(descriptor.request_digest, [4; 32]);

        assert_eq!(
            command_descriptor_from_frame_v1(binding(), None, &mutating, [4; 32]).unwrap_err(),
            CommandCarriageErrorV1::MissingCommandId
        );
        assert_eq!(
            command_descriptor_from_frame_v1(binding(), Some(id()), &query, [4; 32]).unwrap_err(),
            CommandCarriageErrorV1::UnexpectedCommandId
        );
        assert_eq!(command_descriptor_from_frame_v1(binding(), None, &query, [4; 32]).unwrap(), None);
    }

    /// The classification is total and the kinds are used, not decorative
    /// — every kind this tier defines has at least one real payload.
    #[test]
    fn every_command_kind_has_a_real_payload() {
        use super::super::ClientGeneral as C;
        use common::comp::ControlAction;

        let samples: Vec<C> = vec![
            C::ControlAction(ControlAction::Sit),
            C::ExitInGame,
            C::Terminate,
            C::Command("time".to_owned(), Vec::new()),
        ];
        let kinds: std::collections::BTreeSet<CommandKindV1> =
            samples.iter().filter_map(|c| c.command_kind_v1()).collect();
        assert_eq!(
            kinds,
            [
                CommandKindV1::ControlAction,
                CommandKindV1::CharacterLifecycle,
                CommandKindV1::SessionControl,
                CommandKindV1::Administrative,
            ]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
        );
        // InventoryMutation has no ClientGeneral payload of its own yet:
        // inventory changes ride ControlEvent today. Named here so the
        // kind is not silently unreachable.
        assert!(!kinds.contains(&CommandKindV1::InventoryMutation));
    }
}

/// `APEX-T3.5.04` — the exactly-once execution seam. The ledger alone
/// can be misused (admit, then forget to resolve, or execute on a
/// `Resolved`); this wraps both into one call so double execution is
/// unrepresentable at the API: the work is an `FnOnce` the function
/// only invokes on a genuinely fresh command, and its outcome is
/// recorded before it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandExecutionV1 {
    /// The work ran now.
    Executed(CommandOutcomeV1),
    /// A replay: the ORIGINAL outcome, replayed without re-running.
    Replayed(CommandOutcomeV1),
}

impl CommandExecutionV1 {
    pub fn outcome(self) -> CommandOutcomeV1 {
        match self {
            Self::Executed(o) | Self::Replayed(o) => o,
        }
    }
}

/// Runs `execute` at most once per command identity, ever.
///
/// `InFlight` is an error rather than a wait: this seam is single
/// threaded per session, so seeing an unresolved entry here means a
/// previous call panicked or a caller re-entered — either way the
/// command must not run a second time.
pub fn execute_command_once_v1<F>(
    ledger: &mut CommandLedgerV1,
    descriptor: &CommandDescriptorV1,
    execute: F,
) -> Result<CommandExecutionV1, CommandLedgerErrorV1>
where
    F: FnOnce() -> CommandOutcomeV1,
{
    match ledger.admit_v1(descriptor)? {
        CommandAdmitV1::Resolved(outcome) => Ok(CommandExecutionV1::Replayed(outcome)),
        CommandAdmitV1::InFlight => Err(CommandLedgerErrorV1::AlreadyResolved),
        CommandAdmitV1::Fresh => {
            let outcome = execute();
            ledger.resolve_v1(descriptor.command_id, outcome)?;
            Ok(CommandExecutionV1::Executed(outcome))
        },
    }
}

#[cfg(test)]
mod command_execution_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use std::cell::Cell;

    fn binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn command(seed: u8, request: u8) -> CommandDescriptorV1 {
        CommandDescriptorV1 {
            binding: binding(),
            command_id: CommandId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            kind: CommandKindV1::ControlAction,
            request_digest: [request; 32],
        }
    }

    /// The tier's whole claim, stated as a count: N deliveries of the
    /// same command produce exactly ONE execution.
    #[test]
    fn a_command_delivered_many_times_executes_exactly_once() {
        let mut ledger = CommandLedgerV1::new(binding(), 8);
        let cmd = command(10, 1);
        let runs = Cell::new(0u32);

        let first = execute_command_once_v1(&mut ledger, &cmd, || {
            runs.set(runs.get() + 1);
            CommandOutcomeV1::Applied { result_digest: [5; 32] }
        })
        .unwrap();
        assert_eq!(first, CommandExecutionV1::Executed(CommandOutcomeV1::Applied { result_digest: [5; 32] }));

        for _ in 0..16 {
            let again = execute_command_once_v1(&mut ledger, &cmd, || {
                runs.set(runs.get() + 1);
                CommandOutcomeV1::Applied { result_digest: [99; 32] }
            })
            .unwrap();
            assert_eq!(
                again,
                CommandExecutionV1::Replayed(CommandOutcomeV1::Applied { result_digest: [5; 32] }),
                "a replay returns the ORIGINAL outcome, not a fresh one"
            );
        }
        assert_eq!(runs.get(), 1, "the work must have run exactly once across 17 deliveries");
    }

    /// A refusal is an outcome too: it is recorded and replayed, so a
    /// client cannot retry its way past a refusal.
    #[test]
    fn a_refusal_is_recorded_and_replayed_like_any_other_outcome() {
        let mut ledger = CommandLedgerV1::new(binding(), 8);
        let cmd = command(11, 1);
        let runs = Cell::new(0u32);
        let refuse = || {
            runs.set(runs.get() + 1);
            CommandOutcomeV1::Refused { reason: CommandRefusalV1::NotPermitted }
        };

        assert_eq!(
            execute_command_once_v1(&mut ledger, &cmd, refuse).unwrap(),
            CommandExecutionV1::Executed(CommandOutcomeV1::Refused { reason: CommandRefusalV1::NotPermitted })
        );
        assert_eq!(
            execute_command_once_v1(&mut ledger, &cmd, refuse).unwrap(),
            CommandExecutionV1::Replayed(CommandOutcomeV1::Refused { reason: CommandRefusalV1::NotPermitted })
        );
        assert_eq!(runs.get(), 1, "a retried refusal must not re-run the check");

        // ...and a DIFFERENT command still runs
        let other = command(12, 1);
        assert!(matches!(
            execute_command_once_v1(&mut ledger, &other, refuse).unwrap(),
            CommandExecutionV1::Executed(_)
        ));
        assert_eq!(runs.get(), 2);
    }

    /// The work never runs for a command the ledger refuses.
    #[test]
    fn a_refused_admission_never_reaches_the_work() {
        let mut ledger = CommandLedgerV1::new(binding(), 1);
        let runs = Cell::new(0u32);
        let work = || {
            runs.set(runs.get() + 1);
            CommandOutcomeV1::Applied { result_digest: [0; 32] }
        };

        execute_command_once_v1(&mut ledger, &command(20, 1), work).unwrap();
        assert_eq!(runs.get(), 1);

        // window full
        assert_eq!(
            execute_command_once_v1(&mut ledger, &command(21, 1), work).unwrap_err(),
            CommandLedgerErrorV1::WindowExhausted
        );
        // same id, different request bytes
        assert_eq!(
            execute_command_once_v1(&mut ledger, &command(20, 2), work).unwrap_err(),
            CommandLedgerErrorV1::Conflict(CommandIdentityErrorV1::RequestMismatch)
        );
        assert_eq!(runs.get(), 1, "no refused admission may reach the work");
    }
}

/// `APEX-T3.5.05` — where command ids come from. A client that draws
/// them from OS entropy makes two runs of the same scenario emit
/// different ids, which is exactly the class of nondeterminism this
/// program exists to remove. Ids are DERIVED instead: from the session
/// binding and a monotone per-session counter, through the same
/// domain-separated digest everything else in this program uses.
///
/// The result is still an opaque UUIDv4-shaped id — no structure is
/// readable from it — but it is a pure function of (binding, ordinal),
/// so a replay of the same run issues the same id and the ledger
/// recognises it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedCommandIdSourceV1 {
    binding: ActiveSessionBindingV1,
    next_ordinal: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandIdSourceErrorV1 {
    Exhausted,
    NonCanonical,
}

impl DerivedCommandIdSourceV1 {
    /// Ordinals start at 1: zero is reserved for "no command issued",
    /// the same convention the checkpoint epoch chain uses.
    pub fn new(binding: ActiveSessionBindingV1) -> Self { Self { binding, next_ordinal: 1 } }

    pub fn next_ordinal(&self) -> u64 { self.next_ordinal }

    /// Derives the id for an ordinal WITHOUT consuming it — the pure
    /// core, so a test (or a replaying client) can ask what the id for
    /// ordinal N is without moving the counter.
    pub fn id_for_ordinal_v1(
        binding: &ActiveSessionBindingV1,
        ordinal: u64,
    ) -> Result<CommandId, CommandIdSourceErrorV1> {
        let mut p = Vec::with_capacity(64);
        p.extend_from_slice(binding.server_boot_id.as_uuid().as_bytes());
        p.extend_from_slice(binding.session_id.as_uuid().as_bytes());
        p.extend_from_slice(&binding.epoch.get().to_be_bytes());
        p.extend_from_slice(&ordinal.to_be_bytes());
        let digest = digest_canonical_bytes_v1(DigestDomainIdV1::CommandDescriptor, &p, COMMAND_ROOT_INPUT_LIMIT)
            .map(|d: ProtocolDigestV1| *d.bytes.as_array())
            .map_err(|_| CommandIdSourceErrorV1::NonCanonical)?;
        let mut seed = [0u8; 16];
        seed.copy_from_slice(&digest[..16]);
        CommandId::generate(&mut common::apex::identity::FixedRandomBytesSourceV1(seed))
            .map_err(|_| CommandIdSourceErrorV1::NonCanonical)
    }

    /// Consumes the next ordinal and returns its id.
    pub fn issue_v1(&mut self) -> Result<CommandId, CommandIdSourceErrorV1> {
        let ordinal = self.next_ordinal;
        let id = Self::id_for_ordinal_v1(&self.binding, ordinal)?;
        self.next_ordinal = ordinal.checked_add(1).ok_or(CommandIdSourceErrorV1::Exhausted)?;
        Ok(id)
    }
}

#[cfg(test)]
mod command_id_source_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding(seed: u8, epoch: u64) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([seed + 1; 16])).unwrap(),
            epoch: ConnectionEpoch::new(epoch).unwrap(),
        }
    }

    /// Determinism story, stated as a test: the same run issues the same
    /// ids, and no two sessions or epochs share one.
    #[test]
    fn ids_are_derived_not_drawn_and_never_collide_across_sessions() {
        let mut a = DerivedCommandIdSourceV1::new(binding(1, 1));
        let mut b = DerivedCommandIdSourceV1::new(binding(1, 1));
        let run_a: Vec<CommandId> = (0..8).map(|_| a.issue_v1().unwrap()).collect();
        let run_b: Vec<CommandId> = (0..8).map(|_| b.issue_v1().unwrap()).collect();
        assert_eq!(run_a, run_b, "two runs of the same session must issue the same ids");

        // distinct within a session
        let unique: std::collections::BTreeSet<&CommandId> = run_a.iter().collect();
        assert_eq!(unique.len(), run_a.len());

        // a different connection epoch of the SAME session shares none
        let mut later = DerivedCommandIdSourceV1::new(binding(1, 2));
        let run_later: Vec<CommandId> = (0..8).map(|_| later.issue_v1().unwrap()).collect();
        assert!(run_a.iter().all(|id| !run_later.contains(id)), "a new epoch must not reissue an old epoch's ids");

        // ...and so does a different session
        let mut other = DerivedCommandIdSourceV1::new(binding(40, 1));
        let run_other: Vec<CommandId> = (0..8).map(|_| other.issue_v1().unwrap()).collect();
        assert!(run_a.iter().all(|id| !run_other.contains(id)));
    }

    /// The counter is the only state: asking for ordinal N never moves it.
    #[test]
    fn deriving_an_ordinal_does_not_consume_it() {
        let b = binding(1, 1);
        let mut source = DerivedCommandIdSourceV1::new(b);
        assert_eq!(source.next_ordinal(), 1);

        let peeked = DerivedCommandIdSourceV1::id_for_ordinal_v1(&b, 1).unwrap();
        assert_eq!(source.next_ordinal(), 1, "peeking must not consume");
        assert_eq!(source.issue_v1().unwrap(), peeked);
        assert_eq!(source.next_ordinal(), 2);

        // a replaying client re-derives the id of a command it already sent
        assert_eq!(DerivedCommandIdSourceV1::id_for_ordinal_v1(&b, 1).unwrap(), peeked);
    }
}
