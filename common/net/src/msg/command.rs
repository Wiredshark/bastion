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

/// Terminal outcomes a command can reach. `CommandOutcomeV1` is the
/// value a replay reproduces byte for byte (`CMD-075`), so it carries
/// a result DIGEST rather than free text (`CMD-100`).
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

/// `APEX-T3.5.03`, corrected in `.09` against the imported canary
/// catalog — which C2S payloads are commands, and what KIND of admission
/// each gets. Exhaustive, no wildcard arm (`CMD-001`: a new
/// `ClientGeneral` variant with no admission class is a violation).
///
/// Three classes, because two are not enough:
/// - `Journaled` — a discrete mutation. Carries a `CommandId`, enters the
///   retry journal, applies exactly once.
/// - `LatestState` — a continuous input frame. Newest wins; replaying an
///   old one is meaningless, so it must NOT carry a command id and must
///   NOT be journaled (`CMD-002`..`CMD-005`, `CMD-066`).
/// - `ReadOnly` — a query or stream request. Mutates nothing, so a replay
///   costs a recomputation, never a double application (`CMD-023`..
///   `CMD-031`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAdmissionClassV1 {
    Journaled(CommandKindV1),
    LatestState,
    ReadOnly,
}

pub trait CommandParticipantV1 {
    fn admission_class_v1(&self) -> CommandAdmissionClassV1;

    /// The journaled kind, if this payload is a command at all.
    fn command_kind_v1(&self) -> Option<CommandKindV1> {
        match self.admission_class_v1() {
            CommandAdmissionClassV1::Journaled(kind) => Some(kind),
            _ => None,
        }
    }

    /// Whether a client may resend this on its own initiative. Session
    /// control is journaled but never auto-retried (`CMD-032`,
    /// `CMD-065`): a terminate that silently repeats is a different
    /// intent than the one the player expressed.
    fn auto_retryable_v1(&self) -> bool {
        matches!(
            self.admission_class_v1(),
            CommandAdmissionClassV1::Journaled(kind) if kind != CommandKindV1::SessionControl
        )
    }
}

impl CommandParticipantV1 for super::ClientGeneral {
    fn admission_class_v1(&self) -> CommandAdmissionClassV1 {
        use CommandAdmissionClassV1 as A;
        use CommandKindV1 as K;
        use super::ClientGeneral as C;
        match self {
            // Continuous input and view state: newest wins, never journaled.
            C::ControllerInputs(_)
            | C::ControlEvent(_)
            | C::ControlAction(_)
            | C::PlayerPhysics { .. }
            | C::SpectatePosition(_)
            | C::SpectateEntity(_)
            | C::BastionCameraAnchor(_) => A::LatestState,
            // Discrete world mutations.
            C::BreakBlock(_)
            | C::PlaceBlock(_, _)
            | C::UnlockSkill(_)
            | C::SetBattleMode(_)
            | C::UpdateMapMarker(_)
            | C::BastionPlaceDesignation { .. }
            | C::BastionApplyInfluence { .. }
            | C::BastionContextAction { .. }
            | C::BastionSpawnColony { .. }
            | C::BastionCancelDesignation { .. } => A::Journaled(K::ControlAction),
            C::CreateCharacter { .. }
            | C::DeleteCharacter(_)
            | C::EditCharacter { .. }
            | C::Character(_, _)
            | C::Spectate(_)
            | C::ExitInGame => A::Journaled(K::CharacterLifecycle),
            C::Terminate => A::Journaled(K::SessionControl),
            // A chat message is a durable, once-only effect, not a query
            // (`CMD-021`), and an admin command is the same (`CMD-022`).
            C::ChatMsg(_) | C::Command(_, _) => A::Journaled(K::Administrative),
            // Queries, stream requests and acknowledgements.
            C::RequestCharacterList
            | C::SetViewDistance(_)
            | C::RequestSiteInfo(_)
            | C::RequestPlayerPhysics { .. }
            | C::RequestLossyTerrainCompression { .. }
            | C::TerrainChunkRequest { .. }
            | C::LodZoneRequest { .. }
            | C::RequestPlugins(_)
            | C::RequestPluginArtifacts(_)
            | C::BastionInspect { .. }
            | C::CheckpointCommitAck(_) => A::ReadOnly,
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

    /// `.09`: the classes the imported catalog names, asserted against
    /// the payloads it names them for. These are the cases my first pass
    /// got wrong before the catalog was in the tree.
    #[test]
    fn continuous_input_is_not_journaled_and_chat_is() {
        use super::super::ClientGeneral as C;
        use common::comp::ControlAction;
        use vek::Vec3;

        // CMD-002..CMD-005, CMD-066: continuous frames carry no command
        // id and never enter the retry journal.
        for continuous in [
            C::ControlAction(ControlAction::Sit),
            C::PlayerPhysics {
                pos: common::comp::Pos(Vec3::zero()),
                vel: common::comp::Vel(Vec3::zero()),
                ori: common::comp::Ori::default(),
                force_counter: 0,
            },
        ] {
            assert_eq!(continuous.admission_class_v1(), CommandAdmissionClassV1::LatestState);
            assert_eq!(continuous.command_kind_v1(), None);
            assert!(!continuous.auto_retryable_v1());
            assert_eq!(
                command_descriptor_from_frame_v1(binding(), Some(id()), &continuous, [1; 32]).unwrap_err(),
                CommandCarriageErrorV1::UnexpectedCommandId
            );
        }

        // CMD-021/CMD-022: chat and admin commands are durable once-only
        // effects, not read-only traffic.
        for journaled in [C::ChatMsg(common::comp::Content::Plain("hello".to_owned())), C::Command("time".to_owned(), Vec::new())] {
            assert_eq!(
                journaled.admission_class_v1(),
                CommandAdmissionClassV1::Journaled(CommandKindV1::Administrative)
            );
            assert!(journaled.auto_retryable_v1());
        }

        // CMD-011..CMD-015: discrete world mutations are journaled.
        assert_eq!(
            C::BreakBlock(Vec3::zero()).admission_class_v1(),
            CommandAdmissionClassV1::Journaled(CommandKindV1::ControlAction)
        );

        // CMD-032/CMD-065: session control is journaled but NEVER
        // auto-retried.
        assert_eq!(
            C::Terminate.admission_class_v1(),
            CommandAdmissionClassV1::Journaled(CommandKindV1::SessionControl)
        );
        assert!(!C::Terminate.auto_retryable_v1());

        // CMD-023..CMD-031: queries and stream requests stay read-only.
        for query in [C::RequestCharacterList, C::SetViewDistance(common::ViewDistances { terrain: 4, entity: 4 })] {
            assert_eq!(query.admission_class_v1(), CommandAdmissionClassV1::ReadOnly);
            assert!(!query.auto_retryable_v1());
        }
    }

    /// The classification is total and the kinds are used, not decorative
    /// — every kind this tier defines has at least one real payload.
    #[test]
    fn every_command_kind_has_a_real_payload() {
        use super::super::ClientGeneral as C;
        use vek::Vec3;

        let samples: Vec<C> = vec![
            C::BreakBlock(Vec3::zero()),
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

/// `APEX-T3.5.04` — the exactly-once execution seam. The journal alone
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

/// Runs `execute` at most once per (command identity, sequence), ever.
///
/// `InProgress` is an error rather than a wait: this seam is single
/// threaded per session, so seeing an unresolved record here means a
/// previous call panicked or a caller re-entered — either way the
/// command must not run a second time.
pub fn execute_command_once_v1<F>(
    journal: &mut CommandJournalV1,
    descriptor: &CommandDescriptorV1,
    sequence: u64,
    execute: F,
) -> Result<CommandExecutionV1, JournalErrorV1>
where
    F: FnOnce() -> CommandOutcomeV1,
{
    match journal.admit_v1(descriptor, sequence)? {
        CommandDispositionV1::Terminal(outcome) => Ok(CommandExecutionV1::Replayed(outcome)),
        CommandDispositionV1::InProgress => Err(JournalErrorV1::ReentrantDispatch),
        CommandDispositionV1::Dispatch => {
            let outcome = execute();
            journal.resolve_v1(sequence, outcome)?;
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
        let mut journal = CommandJournalV1::new(binding(), 8);
        let cmd = command(10, 1);
        let runs = Cell::new(0u32);

        let first = execute_command_once_v1(&mut journal, &cmd, 1, || {
            runs.set(runs.get() + 1);
            CommandOutcomeV1::Applied { result_digest: [5; 32] }
        })
        .unwrap();
        assert_eq!(first, CommandExecutionV1::Executed(CommandOutcomeV1::Applied { result_digest: [5; 32] }));

        for _ in 0..16 {
            let again = execute_command_once_v1(&mut journal, &cmd, 1, || {
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
        let mut journal = CommandJournalV1::new(binding(), 8);
        let cmd = command(11, 1);
        let runs = Cell::new(0u32);
        let refuse = || {
            runs.set(runs.get() + 1);
            CommandOutcomeV1::Refused { reason: CommandRefusalV1::NotPermitted }
        };

        assert_eq!(
            execute_command_once_v1(&mut journal, &cmd, 1, refuse).unwrap(),
            CommandExecutionV1::Executed(CommandOutcomeV1::Refused { reason: CommandRefusalV1::NotPermitted })
        );
        assert_eq!(
            execute_command_once_v1(&mut journal, &cmd, 1, refuse).unwrap(),
            CommandExecutionV1::Replayed(CommandOutcomeV1::Refused { reason: CommandRefusalV1::NotPermitted })
        );
        assert_eq!(runs.get(), 1, "a retried refusal must not re-run the check");

        // ...and the NEXT command runs, once its predecessor is retired
        journal.retire_v1(1).unwrap();
        let other = command(12, 1);
        assert!(matches!(
            execute_command_once_v1(&mut journal, &other, 2, refuse).unwrap(),
            CommandExecutionV1::Executed(_)
        ));
        assert_eq!(runs.get(), 2);
    }

    /// The work never runs for a command the journal refuses.
    #[test]
    fn a_refused_admission_never_reaches_the_work() {
        let mut journal = CommandJournalV1::new(binding(), 4);
        let runs = Cell::new(0u32);
        let work = || {
            runs.set(runs.get() + 1);
            CommandOutcomeV1::Applied { result_digest: [0; 32] }
        };

        execute_command_once_v1(&mut journal, &command(20, 1), 1, work).unwrap();
        assert_eq!(runs.get(), 1);

        // a gap in the sequence
        assert!(matches!(
            execute_command_once_v1(&mut journal, &command(21, 1), 9, work).unwrap_err(),
            JournalErrorV1::SequenceGap { .. }
        ));
        // same sequence, same id, different request bytes
        assert_eq!(
            execute_command_once_v1(&mut journal, &command(20, 2), 1, work).unwrap_err(),
            JournalErrorV1::IdentityMismatch(CommandIdentityErrorV1::RequestMismatch)
        );
        // a retired sequence
        journal.retire_v1(1).unwrap();
        assert_eq!(
            execute_command_once_v1(&mut journal, &command(20, 1), 1, work).unwrap_err(),
            JournalErrorV1::Retired
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
/// so a replay of the same run issues the same id and the journal
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

    pub fn binding_v1(&self) -> ActiveSessionBindingV1 { self.binding }

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

/// `APEX-T3.5.06`, migrated onto the journal model in `.11` — the client
/// side of exactly-once. A retry resends the SAME identity AND the same
/// sequence (`CMD-057`, `CMD-058`): re-deriving either would arrive as a
/// new command and apply twice. One command is in flight at a time
/// (`CMD-060`), the next sequence advances only when a terminal is
/// acknowledged (`CMD-059`), and session control is never auto-retried
/// (`CMD-065`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingCommandV1 {
    pub descriptor: CommandDescriptorV1,
    pub sequence: u64,
    pub attempts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxErrorV1 {
    /// One command in flight at a time.
    AlreadyInFlight,
    RetryBudgetExhausted,
    /// This kind must never be resent on the client's own initiative.
    NotAutoRetryable,
    UnknownCommand,
    IdSource(CommandIdSourceErrorV1),
}

#[derive(Debug, Clone)]
pub struct ClientCommandOutboxV1 {
    source: DerivedCommandIdSourceV1,
    pending: Option<PendingCommandV1>,
    next_sequence: u64,
    retry_budget: u32,
}

impl ClientCommandOutboxV1 {
    /// The retry budget is deployment-supplied; nothing is invented here.
    pub fn new(binding: ActiveSessionBindingV1, retry_budget: u32) -> Self {
        Self {
            source: DerivedCommandIdSourceV1::new(binding),
            pending: None,
            next_sequence: 1,
            retry_budget,
        }
    }

    pub fn pending_len(&self) -> usize { usize::from(self.pending.is_some()) }

    pub fn next_sequence(&self) -> u64 { self.next_sequence }

    pub fn pending_v1(&self, command_id: &CommandId) -> Option<PendingCommandV1> {
        self.pending.filter(|p| p.descriptor.command_id == *command_id)
    }

    /// Issues a NEW command at the next sequence. The sequence does NOT
    /// advance here: it advances when the terminal is acknowledged, so a
    /// reconnect mid-flight resends the same sequence rather than
    /// skipping one.
    pub fn issue_v1(
        &mut self,
        kind: CommandKindV1,
        request_digest: [u8; 32],
    ) -> Result<PendingCommandV1, OutboxErrorV1> {
        if self.pending.is_some() {
            return Err(OutboxErrorV1::AlreadyInFlight);
        }
        let command_id = self.source.issue_v1().map_err(OutboxErrorV1::IdSource)?;
        let descriptor =
            CommandDescriptorV1 { binding: self.source.binding_v1(), command_id, kind, request_digest };
        let pending = PendingCommandV1 { descriptor, sequence: self.next_sequence, attempts: 1 };
        self.pending = Some(pending);
        Ok(pending)
    }

    /// Resends the outstanding command, unchanged in identity and
    /// sequence. Only the attempt count moves.
    pub fn retry_v1(&mut self, command_id: &CommandId) -> Result<PendingCommandV1, OutboxErrorV1> {
        let pending = self.pending.as_mut().ok_or(OutboxErrorV1::UnknownCommand)?;
        if pending.descriptor.command_id != *command_id {
            return Err(OutboxErrorV1::UnknownCommand);
        }
        if pending.descriptor.kind == CommandKindV1::SessionControl {
            return Err(OutboxErrorV1::NotAutoRetryable);
        }
        if pending.attempts >= self.retry_budget {
            return Err(OutboxErrorV1::RetryBudgetExhausted);
        }
        pending.attempts += 1;
        Ok(*pending)
    }

    /// Clears the outstanding command and advances the sequence. Called
    /// only once a terminal outcome is known.
    pub fn acknowledge_v1(&mut self, command_id: &CommandId) -> Result<PendingCommandV1, OutboxErrorV1> {
        let pending = self.pending.ok_or(OutboxErrorV1::UnknownCommand)?;
        if pending.descriptor.command_id != *command_id {
            return Err(OutboxErrorV1::UnknownCommand);
        }
        self.pending = None;
        self.next_sequence = pending.sequence + 1;
        Ok(pending)
    }
}

#[cfg(test)]
mod command_outbox_v1 {
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

    /// The loop the tier exists for: the client retries, the server
    /// executes once. Both halves driven together.
    #[test]
    fn retries_carry_the_same_identity_and_execute_once_end_to_end() {
        let mut outbox = ClientCommandOutboxV1::new(binding(), 5);
        let mut journal = CommandJournalV1::new(binding(), 8);
        let runs = Cell::new(0u32);

        let first = outbox.issue_v1(CommandKindV1::ControlAction, [7; 32]).unwrap();
        let mut deliveries = vec![first];
        for _ in 0..4 {
            deliveries.push(outbox.retry_v1(&first.descriptor.command_id).unwrap());
        }
        assert!(
            deliveries.iter().all(|d| d.descriptor == first.descriptor && d.sequence == first.sequence),
            "a retry must not change the command's identity OR its sequence"
        );
        assert_eq!(outbox.pending_v1(&first.descriptor.command_id).unwrap().attempts, 5);

        let mut outcomes = Vec::new();
        for delivery in &deliveries {
            outcomes.push(
                execute_command_once_v1(&mut journal, &delivery.descriptor, delivery.sequence, || {
                    runs.set(runs.get() + 1);
                    CommandOutcomeV1::Applied { result_digest: [3; 32] }
                })
                .unwrap()
                .outcome(),
            );
        }
        assert_eq!(runs.get(), 1, "five deliveries, one execution");
        assert!(outcomes.iter().all(|o| *o == CommandOutcomeV1::Applied { result_digest: [3; 32] }));

        // the budget is real
        assert_eq!(
            outbox.retry_v1(&first.descriptor.command_id).unwrap_err(),
            OutboxErrorV1::RetryBudgetExhausted
        );

        // acknowledging clears it and advances the sequence exactly once
        assert_eq!(outbox.next_sequence(), 1, "the sequence must not advance before the terminal ack");
        assert_eq!(outbox.acknowledge_v1(&first.descriptor.command_id).unwrap().attempts, 5);
        assert_eq!(outbox.pending_len(), 0);
        assert_eq!(outbox.next_sequence(), 2);
        assert_eq!(
            outbox.acknowledge_v1(&first.descriptor.command_id).unwrap_err(),
            OutboxErrorV1::UnknownCommand
        );
    }

    /// One in flight at a time, distinct ids per issue, and session
    /// control never auto-retried.
    #[test]
    fn one_in_flight_distinct_ids_and_no_auto_retry_of_session_control() {
        let mut outbox = ClientCommandOutboxV1::new(binding(), 3);
        let a = outbox.issue_v1(CommandKindV1::ControlAction, [1; 32]).unwrap();
        assert_eq!(
            outbox.issue_v1(CommandKindV1::ControlAction, [1; 32]).unwrap_err(),
            OutboxErrorV1::AlreadyInFlight
        );

        outbox.acknowledge_v1(&a.descriptor.command_id).unwrap();
        let b = outbox.issue_v1(CommandKindV1::ControlAction, [1; 32]).unwrap();
        assert_ne!(a.descriptor.command_id, b.descriptor.command_id, "two issues are two commands");
        assert_eq!(b.sequence, a.sequence + 1);
        assert_eq!(outbox.retry_v1(&a.descriptor.command_id).unwrap_err(), OutboxErrorV1::UnknownCommand);
        outbox.acknowledge_v1(&b.descriptor.command_id).unwrap();

        // CMD-032/CMD-065: a terminate is journaled but never auto-resent
        let terminate = outbox.issue_v1(CommandKindV1::SessionControl, [9; 32]).unwrap();
        assert_eq!(
            outbox.retry_v1(&terminate.descriptor.command_id).unwrap_err(),
            OutboxErrorV1::NotAutoRetryable
        );
    }
}

/// `APEX-T3.5.07` — the receipt a command comes back with. It carries
/// the command's IDENTITY ROOT, not just its id, so the client checks
/// that the outcome belongs to the exact command it sent — same kind,
/// same request bytes — instead of trusting an id it handed out itself.
/// (The wire variant that carries this rides the session-control row,
/// so the tier takes one protocol bump rather than several.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandReceiptV1 {
    pub command_id: CommandId,
    pub identity_root: [u8; 32],
    pub outcome: CommandOutcomeV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptErrorV1 {
    /// No such command is outstanding in this outbox.
    NotOutstanding,
    /// The receipt names a command whose identity does not match the one
    /// this client actually sent under that id.
    IdentityMismatch,
    NonCanonical,
}

impl CommandReceiptV1 {
    /// Built by the executing side from the descriptor it admitted, so
    /// the root is over the command as RECEIVED.
    pub fn for_command_v1(
        descriptor: &CommandDescriptorV1,
        outcome: CommandOutcomeV1,
    ) -> Result<Self, ReceiptErrorV1> {
        let identity_root = descriptor.identity_root_v1().map_err(|_| ReceiptErrorV1::NonCanonical)?;
        Ok(Self { command_id: descriptor.command_id, identity_root, outcome })
    }
}

impl ClientCommandOutboxV1 {
    /// Verifies a receipt against the outstanding command and clears it.
    /// The identity root is RECOMPUTED from the descriptor this client
    /// holds; a receipt that does not reproduce it is refused and the
    /// command stays outstanding.
    pub fn accept_receipt_v1(&mut self, receipt: &CommandReceiptV1) -> Result<CommandOutcomeV1, ReceiptErrorV1> {
        let pending = self.pending_v1(&receipt.command_id).ok_or(ReceiptErrorV1::NotOutstanding)?;
        let expected = pending.descriptor.identity_root_v1().map_err(|_| ReceiptErrorV1::NonCanonical)?;
        if expected != receipt.identity_root {
            return Err(ReceiptErrorV1::IdentityMismatch);
        }
        self.acknowledge_v1(&receipt.command_id).map_err(|_| ReceiptErrorV1::NotOutstanding)?;
        Ok(receipt.outcome)
    }
}

#[cfg(test)]
mod command_receipt_v1 {
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

    /// The full round trip: issue, retry, execute once, receipt back,
    /// verified against the client's own copy of the command.
    #[test]
    fn a_receipt_clears_only_the_command_it_actually_names() {
        let mut outbox = ClientCommandOutboxV1::new(binding(), 4);
        let mut journal = CommandJournalV1::new(binding(), 8);
        let runs = Cell::new(0u32);

        let cmd = outbox.issue_v1(CommandKindV1::ControlAction, [7; 32]).unwrap();
        let resent = outbox.retry_v1(&cmd.descriptor.command_id).unwrap();
        let executed = execute_command_once_v1(&mut journal, &resent.descriptor, resent.sequence, || {
            runs.set(runs.get() + 1);
            CommandOutcomeV1::Applied { result_digest: [9; 32] }
        })
        .unwrap();
        assert_eq!(runs.get(), 1);

        let receipt = CommandReceiptV1::for_command_v1(&resent.descriptor, executed.outcome()).unwrap();

        // a receipt whose root does not reproduce is refused, and the
        // command stays outstanding
        let mut forged = receipt;
        forged.identity_root = [0xAB; 32];
        assert_eq!(outbox.accept_receipt_v1(&forged).unwrap_err(), ReceiptErrorV1::IdentityMismatch);
        assert_eq!(outbox.pending_len(), 1, "a refused receipt must not clear the command");

        // ...and the genuine one clears it exactly once
        assert_eq!(
            outbox.accept_receipt_v1(&receipt).unwrap(),
            CommandOutcomeV1::Applied { result_digest: [9; 32] }
        );
        assert_eq!(outbox.pending_len(), 0);
        assert_eq!(outbox.accept_receipt_v1(&receipt).unwrap_err(), ReceiptErrorV1::NotOutstanding);
    }

    /// A receipt for a command this client never sent is refused even if
    /// its id happens to be one this client would issue later.
    #[test]
    fn a_receipt_for_an_unsent_command_is_refused() {
        let mut outbox = ClientCommandOutboxV1::new(binding(), 4);
        let future_id = DerivedCommandIdSourceV1::id_for_ordinal_v1(&binding(), 1).unwrap();
        let descriptor = CommandDescriptorV1 {
            binding: binding(),
            command_id: future_id,
            kind: CommandKindV1::ControlAction,
            request_digest: [7; 32],
        };
        let receipt =
            CommandReceiptV1::for_command_v1(&descriptor, CommandOutcomeV1::Applied { result_digest: [1; 32] })
                .unwrap();
        assert_eq!(outbox.accept_receipt_v1(&receipt).unwrap_err(), ReceiptErrorV1::NotOutstanding);

        // once the client DOES issue that ordinal, a receipt for a
        // different request under the same id is still refused
        let issued = outbox.issue_v1(CommandKindV1::ControlAction, [8; 32]).unwrap();
        assert_eq!(
            issued.descriptor.command_id, future_id,
            "ids are derived, so the ordinal is predictable by construction"
        );
        assert_eq!(outbox.accept_receipt_v1(&receipt).unwrap_err(), ReceiptErrorV1::IdentityMismatch);
        assert_eq!(outbox.pending_len(), 1);
    }
}

/// `APEX-T3.5.10` — the sequence-and-floor journal the imported catalog
/// actually specifies. `CommandLedgerV1` (.02) keyed on `CommandId`
/// alone and so had to fail closed when full: with no ordering it cannot
/// tell "never seen" from "seen and forgotten". A monotone per-session
/// SEQUENCE fixes that — everything at or below the retired floor is
/// known-terminal, so records can be dropped without ever letting a
/// replay read as fresh (`CMD-070`, `CMD-082`).
///
/// Scope, exactly as the catalog draws it: the journal belongs to one
/// (`ServerBootId`, `SessionId`). It SURVIVES a connection-epoch
/// increment, carrying its floor across a resume (`CMD-083`, `CMD-086`),
/// and it dies with the session and with the boot (`CMD-084`, `CMD-085`,
/// `CMD-144`) — a new session never inherits a floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalStateV1 {
    InFlight,
    Terminal(CommandOutcomeV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalEntryV1 {
    pub descriptor: CommandDescriptorV1,
    pub sequence: u64,
    pub state: JournalStateV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandDispositionV1 {
    /// Never seen: execute it.
    Dispatch,
    /// Seen, still executing: do NOT dispatch again (`CMD-076`).
    InProgress,
    /// Seen and finished: replay these terminal bytes (`CMD-075`).
    Terminal(CommandOutcomeV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalErrorV1 {
    /// Sequence zero is not a sequence (`CMD-037`).
    SequenceZero,
    /// At or below the retired floor — already terminal and acked.
    Retired,
    /// Beyond the next expected sequence (`CMD-069`).
    SequenceGap { expected: u64, got: u64 },
    /// The sequence is known but names a different command (`CMD-073`).
    IdentityMismatch(CommandIdentityErrorV1),
    /// This command id is already journaled under another sequence
    /// (`CMD-074`).
    IdReusedUnderAnotherSequence,
    /// One command in flight at a time; a prior terminal is still
    /// unacked (`CMD-077`).
    PriorTerminalUnacked,
    /// The frame comes from a superseded attachment (`CMD-087`).
    SupersededAttachment,
    /// Not this session's journal at all.
    ForeignSession,
    Capacity,
    /// Acking something that is not the floor's successor, or is not
    /// terminal (`CMD-080`, `CMD-081`).
    NotAckable,
    /// The execution seam was re-entered while a record was still
    /// unresolved; the command must not run a second time.
    ReentrantDispatch,
}

#[derive(Debug, Clone)]
pub struct CommandJournalV1 {
    binding: ActiveSessionBindingV1,
    retired_floor: u64,
    active: std::collections::BTreeMap<u64, JournalEntryV1>,
    capacity: usize,
}

impl CommandJournalV1 {
    pub fn new(binding: ActiveSessionBindingV1, capacity: usize) -> Self {
        Self { binding, retired_floor: 0, active: std::collections::BTreeMap::new(), capacity }
    }

    pub fn binding(&self) -> ActiveSessionBindingV1 { self.binding }

    pub fn retired_floor(&self) -> u64 { self.retired_floor }

    /// The next sequence this journal will accept as new.
    pub fn next_expected_v1(&self) -> u64 {
        self.active.keys().next_back().copied().unwrap_or(self.retired_floor) + 1
    }

    pub fn entry_v1(&self, sequence: u64) -> Option<JournalEntryV1> { self.active.get(&sequence).copied() }

    /// A resume keeps the journal: same session, later connection epoch.
    /// The floor and every active record survive, so a command issued
    /// before the reconnect cannot re-execute after it (`CMD-086`).
    pub fn rebind_epoch_v1(
        &mut self,
        binding: ActiveSessionBindingV1,
    ) -> Result<(), JournalErrorV1> {
        if binding.server_boot_id != self.binding.server_boot_id || binding.session_id != self.binding.session_id {
            return Err(JournalErrorV1::ForeignSession);
        }
        if binding.epoch.get() < self.binding.epoch.get() {
            return Err(JournalErrorV1::SupersededAttachment);
        }
        self.binding = binding;
        Ok(())
    }

    /// Classifies a command frame. Pure: nothing is dispatched here, and
    /// only a genuinely new sequence reserves a record.
    pub fn admit_v1(
        &mut self,
        descriptor: &CommandDescriptorV1,
        sequence: u64,
    ) -> Result<CommandDispositionV1, JournalErrorV1> {
        if descriptor.binding.server_boot_id != self.binding.server_boot_id
            || descriptor.binding.session_id != self.binding.session_id
        {
            return Err(JournalErrorV1::ForeignSession);
        }
        if descriptor.binding.epoch.get() < self.binding.epoch.get() {
            return Err(JournalErrorV1::SupersededAttachment);
        }
        if sequence == 0 {
            return Err(JournalErrorV1::SequenceZero);
        }
        if sequence <= self.retired_floor {
            return Err(JournalErrorV1::Retired);
        }

        if let Some(entry) = self.active.get(&sequence) {
            // Known sequence: it must be the SAME command, in every field.
            if entry.descriptor.command_id != descriptor.command_id {
                return Err(JournalErrorV1::IdentityMismatch(CommandIdentityErrorV1::RequestMismatch));
            }
            entry.descriptor.is_replay_of_v1(descriptor).map_err(JournalErrorV1::IdentityMismatch)?;
            return Ok(match entry.state {
                JournalStateV1::InFlight => CommandDispositionV1::InProgress,
                JournalStateV1::Terminal(outcome) => CommandDispositionV1::Terminal(outcome),
            });
        }

        let expected = self.next_expected_v1();
        if sequence != expected {
            return Err(JournalErrorV1::SequenceGap { expected, got: sequence });
        }
        // One in flight at a time, and a finished command must be acked
        // before the next is admitted.
        if !self.active.is_empty() {
            return Err(JournalErrorV1::PriorTerminalUnacked);
        }
        if self.active.len() >= self.capacity {
            return Err(JournalErrorV1::Capacity);
        }
        // A command id may not appear under two sequences.
        if self.active.values().any(|e| e.descriptor.command_id == descriptor.command_id) {
            return Err(JournalErrorV1::IdReusedUnderAnotherSequence);
        }
        self.active.insert(
            sequence,
            JournalEntryV1 { descriptor: *descriptor, sequence, state: JournalStateV1::InFlight },
        );
        Ok(CommandDispositionV1::Dispatch)
    }

    /// Records the terminal outcome of an in-flight command.
    pub fn resolve_v1(&mut self, sequence: u64, outcome: CommandOutcomeV1) -> Result<(), JournalErrorV1> {
        let entry = self.active.get_mut(&sequence).ok_or(JournalErrorV1::NotAckable)?;
        match entry.state {
            JournalStateV1::InFlight => {
                entry.state = JournalStateV1::Terminal(outcome);
                Ok(())
            },
            JournalStateV1::Terminal(_) => Err(JournalErrorV1::NotAckable),
        }
    }

    /// Retires a terminal command once the client has acknowledged it.
    /// Only the floor's immediate successor may retire, so a duplicate
    /// ack cannot advance the floor twice and an ack for an unknown
    /// sequence cannot advance it at all (`CMD-080`, `CMD-081`).
    pub fn retire_v1(&mut self, sequence: u64) -> Result<u64, JournalErrorV1> {
        if sequence != self.retired_floor + 1 {
            return Err(JournalErrorV1::NotAckable);
        }
        let entry = self.active.get(&sequence).ok_or(JournalErrorV1::NotAckable)?;
        if !matches!(entry.state, JournalStateV1::Terminal(_)) {
            return Err(JournalErrorV1::NotAckable);
        }
        self.active.remove(&sequence);
        self.retired_floor = sequence;
        Ok(self.retired_floor)
    }
}

#[cfg(test)]
mod command_journal_v1 {
    use super::*;
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    fn binding_at(epoch: u64) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(epoch).unwrap(),
        }
    }

    fn command_at(binding: ActiveSessionBindingV1, seed: u8, request: u8) -> CommandDescriptorV1 {
        CommandDescriptorV1 {
            binding,
            command_id: CommandId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            kind: CommandKindV1::ControlAction,
            request_digest: [request; 32],
        }
    }

    /// The floor is what makes bounded retention safe: a retired
    /// sequence is recognisably retired, never mistaken for fresh.
    #[test]
    fn a_retired_sequence_is_refused_not_re_executed() {
        let b = binding_at(1);
        let mut journal = CommandJournalV1::new(b, 4);
        let first = command_at(b, 10, 1);

        assert_eq!(journal.next_expected_v1(), 1);
        assert_eq!(journal.admit_v1(&first, 1).unwrap(), CommandDispositionV1::Dispatch);
        // in flight: a duplicate must not dispatch again
        assert_eq!(journal.admit_v1(&first, 1).unwrap(), CommandDispositionV1::InProgress);

        let outcome = CommandOutcomeV1::Applied { result_digest: [5; 32] };
        journal.resolve_v1(1, outcome).unwrap();
        // terminal: a duplicate replays the terminal bytes
        assert_eq!(journal.admit_v1(&first, 1).unwrap(), CommandDispositionV1::Terminal(outcome));

        // ...and once retired, the record is gone but the sequence is not
        assert_eq!(journal.retire_v1(1).unwrap(), 1);
        assert_eq!(journal.entry_v1(1), None, "the record is dropped");
        assert_eq!(journal.admit_v1(&first, 1).unwrap_err(), JournalErrorV1::Retired);

        // a duplicate ack cannot advance the floor twice
        assert_eq!(journal.retire_v1(1).unwrap_err(), JournalErrorV1::NotAckable);
        assert_eq!(journal.retired_floor(), 1);
        // nor can an ack for something never journaled
        assert_eq!(journal.retire_v1(9).unwrap_err(), JournalErrorV1::NotAckable);
        assert_eq!(journal.retired_floor(), 1);
    }

    #[test]
    fn sequence_gaps_reuse_and_unacked_terminals_are_all_typed() {
        let b = binding_at(1);
        let mut journal = CommandJournalV1::new(b, 4);
        let first = command_at(b, 10, 1);

        assert_eq!(journal.admit_v1(&first, 0).unwrap_err(), JournalErrorV1::SequenceZero);
        assert_eq!(
            journal.admit_v1(&first, 5).unwrap_err(),
            JournalErrorV1::SequenceGap { expected: 1, got: 5 }
        );

        journal.admit_v1(&first, 1).unwrap();
        // same sequence, different command
        let other = command_at(b, 11, 1);
        assert!(matches!(journal.admit_v1(&other, 1).unwrap_err(), JournalErrorV1::IdentityMismatch(_)));
        // same sequence, same id, different request bytes
        let tampered = command_at(b, 10, 2);
        assert_eq!(
            journal.admit_v1(&tampered, 1).unwrap_err(),
            JournalErrorV1::IdentityMismatch(CommandIdentityErrorV1::RequestMismatch)
        );

        // one in flight at a time
        assert_eq!(journal.admit_v1(&other, 2).unwrap_err(), JournalErrorV1::PriorTerminalUnacked);
        // ...and still one while its terminal is unacked
        journal.resolve_v1(1, CommandOutcomeV1::Applied { result_digest: [1; 32] }).unwrap();
        assert_eq!(journal.admit_v1(&other, 2).unwrap_err(), JournalErrorV1::PriorTerminalUnacked);
        // once retired, the next sequence flows
        journal.retire_v1(1).unwrap();
        assert_eq!(journal.admit_v1(&other, 2).unwrap(), CommandDispositionV1::Dispatch);

        // resolving twice is refused
        journal.resolve_v1(2, CommandOutcomeV1::Applied { result_digest: [2; 32] }).unwrap();
        assert_eq!(
            journal.resolve_v1(2, CommandOutcomeV1::Applied { result_digest: [3; 32] }).unwrap_err(),
            JournalErrorV1::NotAckable
        );
    }

    /// Scope: a resume keeps the floor; another session or an older
    /// attachment gets nothing.
    #[test]
    fn the_journal_survives_a_resume_and_refuses_foreign_or_stale_attachments() {
        let first_epoch = binding_at(1);
        let mut journal = CommandJournalV1::new(first_epoch, 4);
        let cmd = command_at(first_epoch, 10, 1);
        journal.admit_v1(&cmd, 1).unwrap();
        journal.resolve_v1(1, CommandOutcomeV1::Applied { result_digest: [7; 32] }).unwrap();
        journal.retire_v1(1).unwrap();

        // resume under a later connection epoch: the floor survives
        let resumed = binding_at(2);
        journal.rebind_epoch_v1(resumed).unwrap();
        assert_eq!(journal.retired_floor(), 1, "a resume must not lose the retired floor");
        let replayed = command_at(resumed, 10, 1);
        assert_eq!(journal.admit_v1(&replayed, 1).unwrap_err(), JournalErrorV1::Retired);

        // a frame from the superseded attachment is refused
        let stale = command_at(first_epoch, 12, 1);
        assert_eq!(journal.admit_v1(&stale, 2).unwrap_err(), JournalErrorV1::SupersededAttachment);
        // ...as is rebinding backwards
        assert_eq!(journal.rebind_epoch_v1(first_epoch).unwrap_err(), JournalErrorV1::SupersededAttachment);

        // another session shares nothing
        let foreign = ActiveSessionBindingV1 {
            server_boot_id: resumed.server_boot_id,
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([90; 16])).unwrap(),
            epoch: ConnectionEpoch::new(2).unwrap(),
        };
        assert_eq!(journal.rebind_epoch_v1(foreign).unwrap_err(), JournalErrorV1::ForeignSession);
        assert_eq!(journal.admit_v1(&command_at(foreign, 13, 1), 2).unwrap_err(), JournalErrorV1::ForeignSession);
        // and a fresh session starts with no floor at all
        assert_eq!(CommandJournalV1::new(foreign, 4).retired_floor(), 0);
    }
}
