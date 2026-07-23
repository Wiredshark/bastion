use crate::{
    comp::{
        BuffKind, ability,
        inventory::{
            InventorySortOrder,
            item::tool::ToolKind,
            slot::{EquipSlot, InvSlotId, Slot},
        },
        invite::{InviteKind, InviteResponse},
    },
    mounting::VolumePos,
    rtsim,
    trade::{TradeAction, TradeId},
    uid::Uid,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use specs::Component;
use std::{collections::BTreeMap, num::NonZeroU32};
use vek::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryEvent {
    Pickup(Uid),
    Swap(InvSlotId, InvSlotId),
    SplitSwap(InvSlotId, InvSlotId),
    Drop(InvSlotId),
    SplitDrop(InvSlotId),
    Sort(InventorySortOrder),
    CraftRecipe {
        craft_event: CraftEvent,
        craft_sprite: Option<VolumePos>,
    },
    OverflowMove(usize, InvSlotId),
    OverflowDrop(usize),
    OverflowSplitDrop(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryAction {
    Swap(EquipSlot, Slot),
    Drop(EquipSlot),
    Use(Slot),
    Sort(InventorySortOrder),
    Collect(Vec3<i32>),
    // TODO: Not actually inventory-related: refactor to allow sprite interaction without
    // inventory manipulation!
    ToggleSpriteLight(VolumePos, bool),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryManip {
    Pickup(Uid),
    Collect {
        sprite_pos: Vec3<i32>,
        /// If second field is `true`, item will be consumed on collection.
        required_item: Option<(InvSlotId, bool)>,
    },
    Use(Slot),
    Swap(Slot, Slot),
    SplitSwap(Slot, Slot),
    Drop(Slot),
    SplitDrop(Slot),
    Sort(InventorySortOrder),
    CraftRecipe {
        craft_event: CraftEvent,
        craft_sprite: Option<VolumePos>,
    },
    SwapEquippedWeapons,
    Delete(InvSlotId, NonZeroU32),
}

impl From<InventoryEvent> for InventoryManip {
    fn from(inv_event: InventoryEvent) -> Self {
        match inv_event {
            InventoryEvent::Pickup(pickup) => Self::Pickup(pickup),
            InventoryEvent::Swap(inv1, inv2) => {
                Self::Swap(Slot::Inventory(inv1), Slot::Inventory(inv2))
            },
            InventoryEvent::SplitSwap(inv1, inv2) => {
                Self::SplitSwap(Slot::Inventory(inv1), Slot::Inventory(inv2))
            },
            InventoryEvent::Drop(inv) => Self::Drop(Slot::Inventory(inv)),
            InventoryEvent::SplitDrop(inv) => Self::SplitDrop(Slot::Inventory(inv)),
            InventoryEvent::Sort(sort_order) => Self::Sort(sort_order),
            InventoryEvent::CraftRecipe {
                craft_event,
                craft_sprite,
            } => Self::CraftRecipe {
                craft_event,
                craft_sprite,
            },
            InventoryEvent::OverflowMove(o, inv) => {
                Self::Swap(Slot::Overflow(o), Slot::Inventory(inv))
            },
            InventoryEvent::OverflowDrop(o) => Self::Drop(Slot::Overflow(o)),
            InventoryEvent::OverflowSplitDrop(o) => Self::SplitDrop(Slot::Overflow(o)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CraftEvent {
    Simple {
        recipe: String,
        slots: Vec<(u32, InvSlotId)>,
        amount: u32,
    },
    Salvage(InvSlotId),
    // TODO: Maybe look at making this more general when there are more modular recipes?
    ModularWeapon {
        primary_component: InvSlotId,
        secondary_component: InvSlotId,
    },
    // TODO: Maybe try to consolidate into another? Otherwise eventually make more general.
    ModularWeaponPrimaryComponent {
        toolkind: ToolKind,
        material: InvSlotId,
        modifier: Option<InvSlotId>,
        slots: Vec<(u32, InvSlotId)>,
    },
    Repair(Slot),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupManip {
    Leave,
    Kick(Uid),
    AssignLeader(Uid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, strum::EnumString)]
pub enum UtteranceKind {
    Calm,
    Angry,
    Surprised,
    Hurt,
    Greeting,
    Scream,
    Ambush,
    /* Death,
     * TODO: Wait for more post-death features (i.e. animations) before implementing death
     * sounds */
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlEvent {
    //ToggleLantern,
    EnableLantern,
    DisableLantern,
    Interact(Uid),
    InitiateInvite(Uid, InviteKind),
    InviteResponse(InviteResponse),
    PerformTradeAction(TradeId, TradeAction),
    Mount(Uid),
    MountVolume(VolumePos),
    Unmount,
    SetPetStay(Uid, bool),
    InventoryEvent(InventoryEvent),
    GroupManip(GroupManip),
    RemoveBuff(BuffKind),
    LeaveStance,
    GiveUp,
    Respawn,
    Utterance(UtteranceKind),
    ChangeAbility {
        slot: usize,
        auxiliary_key: ability::AuxiliaryKey,
        new_ability: ability::AuxiliaryAbility,
    },
    ActivatePortal(Uid),
    InteractWith {
        target: Uid,
        kind: crate::interaction::InteractionKind,
    },
    Dialogue(Uid, rtsim::Dialogue),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlAction {
    SwapEquippedWeapons,
    InventoryAction(InventoryAction),
    Wield,
    GlideWield,
    Unwield,
    Sit,
    Crawl,
    Dance,
    Sneak,
    Stand,
    Talk(Option<Uid>),
    StartInput {
        input: InputKind,
        target_entity: Option<Uid>,
        // Some inputs need a selected position, such as mining
        select_pos: Option<Vec3<f32>>,
    },
    CancelInput {
        input: InputKind,
    },
}

impl ControlAction {
    pub fn basic_input(input: InputKind) -> Self {
        ControlAction::StartInput {
            input,
            target_entity: None,
            select_pos: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Ord, PartialOrd)]
#[repr(u32)]
pub enum InputKind {
    Primary = 0,
    Secondary = 1,
    Block = 2,
    Ability(usize) = 3,
    Roll = 4,
    Jump = 5,
    Fly = 6,
    WallJump = 7,
}

impl InputKind {
    pub fn is_ability(self) -> bool {
        matches!(
            self,
            Self::Primary | Self::Secondary | Self::Ability(_) | Self::Block
        )
    }
}

impl From<InputKind> for Option<ability::AbilityInput> {
    fn from(input: InputKind) -> Option<ability::AbilityInput> {
        use ability::AbilityInput;
        match input {
            InputKind::Block => Some(AbilityInput::Guard),
            InputKind::Primary => Some(AbilityInput::Primary),
            InputKind::Secondary => Some(AbilityInput::Secondary),
            InputKind::Roll => Some(AbilityInput::Movement),
            InputKind::Ability(index) => Some(AbilityInput::Auxiliary(index)),
            InputKind::Jump | InputKind::WallJump | InputKind::Fly => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputAttr {
    pub select_pos: Option<Vec3<f32>>,
    pub target_entity: Option<Uid>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControllerInputs {
    pub move_dir: Vec2<f32>,
    pub move_z: f32, /* z axis (not combined with move_dir because they may have independent
                      * limits) */
    pub look_dir: Dir,
    pub break_block_pos: Option<Vec3<f32>>,
    /// Attempt to enable strafing.
    /// Currently, setting this to false will *not* disable strafing during a
    /// wielding character state.
    pub strafing: bool,
}

/// T0.22 (master build order; ledger #190): which scheduled consumer a
/// staged command is addressed to — `Control` = `controller::Sys` (control
/// events), `Behavior` = `character_behavior::Sys` (control actions). The
/// phase tag plus the producer-local sequence is the envelope's identity;
/// tick and actor are implicit (the frame stamp and the owning entity), and
/// producer rank is carried by the DECLARED system schedule (T0.12/14/20
/// edges) that fixes push order.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandPhase {
    Control,
    Behavior,
}

/// T0.22: the tagged command payload — one channel for what were two
/// unrelated vecs, so CROSS-CHANNEL relative order is preserved by
/// construction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommandPayload {
    Event(ControlEvent),
    Action(ControlAction),
}

/// T0.22: one staged command in the envelope channel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueuedCommand {
    pub phase: CommandPhase,
    /// Producer-local sequence within the current frame — monotonic across
    /// BOTH phases (the cross-channel ordering witness).
    pub seq: u32,
    pub payload: CommandPayload,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub inputs: ControllerInputs,
    pub queued_inputs: BTreeMap<InputKind, InputAttr>,
    // T0.21 (ledger #151) + T0.22 (ledger #190): ONE private sequenced
    // command channel. Producers stage through the push_* API (which tags
    // phase + sequence); each scheduled consumer drains ITS phase from the
    // shared frame at its dispatch point (`drain_events` = Control,
    // `take_actions` = Behavior), exactly-once per phase, advancing the
    // frame stamp. The type enforces what the T0.20 dispatcher edge
    // declares, and the single channel preserves cross-channel relative
    // order by construction.
    commands: Vec<QueuedCommand>,
    /// Producer-local sequence counter for the current frame.
    #[serde(skip)]
    seq: u32,
    /// Frame generation — incremented by each scheduled drain; staleness
    /// checks and pins read it.
    #[serde(skip)]
    frame: u64,
}

impl ControllerInputs {
    /// Sanitize inputs to avoid clients sending bad data.
    pub fn sanitize(&mut self) {
        self.move_dir = if self.move_dir.map(|e| e.is_finite()).reduce_and() {
            self.move_dir / self.move_dir.magnitude().max(1.0)
        } else {
            Vec2::zero()
        };
        self.move_z = if self.move_z.is_finite() {
            self.move_z.clamped(-1.0, 1.0)
        } else {
            0.0
        };
    }

    /// Updates Controller inputs with new version received from the client
    pub fn update_with_new(&mut self, new: Self) {
        self.move_dir = new.move_dir;
        self.move_z = new.move_z;
        self.look_dir = new.look_dir;
        self.break_block_pos = new.break_block_pos;
    }
}

impl Controller {
    /// Sets all inputs to default
    pub fn reset(&mut self) {
        self.inputs = Default::default();
        self.queued_inputs = Default::default();
    }

    pub fn clear_events(&mut self) {
        self.commands
            .retain(|command| command.phase != CommandPhase::Control);
    }

    fn stage(&mut self, phase: CommandPhase, payload: CommandPayload) {
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);
        self.commands.push(QueuedCommand { phase, seq, payload });
    }

    /// T0.21/T0.22: the Control-phase consumer's scheduled drain — takes
    /// this phase's entries from the shared frame (exactly-once per phase)
    /// and advances the generation.
    pub fn drain_events(&mut self) -> Vec<ControlEvent> {
        self.frame += 1;
        self.commands
            .extract_if(.., |command| command.phase == CommandPhase::Control)
            .filter_map(|command| match command.payload {
                CommandPayload::Event(event) => Some(event),
                CommandPayload::Action(_) => None,
            })
            .collect()
    }

    /// T0.21/T0.22: the Behavior-phase consumer's scheduled drain — takes
    /// this phase's entries from the shared frame (exactly-once per phase)
    /// and advances the generation.
    pub fn take_actions(&mut self) -> Vec<ControlAction> {
        self.frame += 1;
        self.commands
            .extract_if(.., |command| command.phase == CommandPhase::Behavior)
            .filter_map(|command| match command.payload {
                CommandPayload::Action(action) => Some(action),
                CommandPayload::Event(_) => None,
            })
            .collect()
    }

    /// Wholesale action replacement (rider→mount redirection): drops the
    /// staged Behavior entries and stages the replacements in order.
    pub fn set_actions(&mut self, actions: Vec<ControlAction>) {
        self.commands
            .retain(|command| command.phase != CommandPhase::Behavior);
        for action in actions {
            self.stage(CommandPhase::Behavior, CommandPayload::Action(action));
        }
    }

    /// Selective extraction (rider→mount input redirection) — explicit API
    /// so partial drains remain visible operations.
    pub fn extract_actions_if(
        &mut self,
        mut pred: impl FnMut(&mut ControlAction) -> bool,
    ) -> Vec<ControlAction> {
        self.commands
            .extract_if(.., |command| match &mut command.payload {
                CommandPayload::Action(action) => pred(action),
                CommandPayload::Event(_) => false,
            })
            .filter_map(|command| match command.payload {
                CommandPayload::Action(action) => Some(action),
                CommandPayload::Event(_) => None,
            })
            .collect()
    }

    pub fn has_queued_events(&self) -> bool {
        self.commands
            .iter()
            .any(|command| command.phase == CommandPhase::Control)
    }

    /// T0.22: read-only view of the staged envelope in producer order — the
    /// cross-channel ordering witness (tests/diagnostics).
    pub fn staged_commands(&self) -> &[QueuedCommand] { &self.commands }

    pub fn frame(&self) -> u64 { self.frame }

    pub fn push_event(&mut self, event: ControlEvent) {
        self.stage(CommandPhase::Control, CommandPayload::Event(event));
    }

    pub fn push_utterance(&mut self, utterance: UtteranceKind) {
        self.push_event(ControlEvent::Utterance(utterance));
    }

    pub fn push_invite_response(&mut self, invite_response: InviteResponse) {
        self.push_event(ControlEvent::InviteResponse(invite_response));
    }

    pub fn push_initiate_invite(&mut self, uid: Uid, invite: InviteKind) {
        self.push_event(ControlEvent::InitiateInvite(uid, invite));
    }

    pub fn push_action(&mut self, action: ControlAction) {
        self.stage(CommandPhase::Behavior, CommandPayload::Action(action));
    }

    pub fn push_basic_input(&mut self, input: InputKind) {
        self.push_action(ControlAction::basic_input(input));
    }

    pub fn push_cancel_input(&mut self, input: InputKind) {
        self.push_action(ControlAction::CancelInput { input });
    }
}

impl Component for Controller {
    type Storage = specs::VecStorage<Self>;
}

// T0.21: the frame contract — staging never advances the generation,
// scheduled drains do, and a drained frame is gone (exactly-once).
#[cfg(test)]
mod t0_21_tests {
    use super::*;

    #[test]
    fn t0_21_command_frames_are_exactly_once() {
        let mut c = Controller::default();
        assert_eq!(c.frame(), 0);
        c.push_event(ControlEvent::EnableLantern);
        c.push_event(ControlEvent::DisableLantern);
        c.push_basic_input(InputKind::Jump);
        assert_eq!(c.frame(), 0, "staging must not advance the frame");
        assert!(c.has_queued_events());

        let events = c.drain_events();
        assert_eq!(events.len(), 2);
        assert_eq!(c.frame(), 1, "a drain advances the generation");
        assert!(c.drain_events().is_empty(), "a drained frame is gone");
        assert_eq!(c.frame(), 2);

        let actions = c.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(c.frame(), 3);
        assert!(c.take_actions().is_empty(), "exactly-once consumption");

        // Post-drain staging lands in the NEXT frame, untouched by the
        // previous consumer.
        c.push_event(ControlEvent::EnableLantern);
        assert!(c.has_queued_events());
        assert_eq!(c.drain_events().len(), 1);
    }

    /// T0.22: the single channel preserves CROSS-CHANNEL relative order
    /// (the seq witness), and each phase drains exactly its own entries.
    #[test]
    fn t0_22_envelope_preserves_cross_channel_order() {
        let mut c = Controller::default();
        c.push_event(ControlEvent::EnableLantern);
        c.push_basic_input(InputKind::Jump);
        c.push_event(ControlEvent::DisableLantern);

        let seqs: Vec<(CommandPhase, u32)> = c
            .staged_commands()
            .iter()
            .map(|q| (q.phase, q.seq))
            .collect();
        assert_eq!(seqs, vec![
            (CommandPhase::Control, 0),
            (CommandPhase::Behavior, 1),
            (CommandPhase::Control, 2),
        ]);

        // Phase drains are disjoint views of the one frame.
        let events = c.drain_events();
        assert_eq!(events, vec![
            ControlEvent::EnableLantern,
            ControlEvent::DisableLantern
        ]);
        let actions = c.take_actions();
        assert_eq!(actions.len(), 1);
        assert!(c.staged_commands().is_empty());
    }
}

/// INP-01 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof): DET-INP —
/// input-SELECTION determinism. When several player inputs are queued in the
/// same tick, the state machine picks which to act on via
/// `queued_inputs.keys().next()` (the canonically-lowest InputKind — see
/// common/src/states/utils.rs, `handle_ability`/interrupt selection). Because
/// `queued_inputs` is a `BTreeMap<InputKind, _>`, that selection is a pure
/// function of the queued input SET, independent of the order the inputs were
/// inserted / received over the network. If the container ever regressed to a
/// HashMap (which would still compile), WHICH ability fires when two are queued
/// the same tick would become non-deterministic — a live-desync bug. The inline
/// BTreeMap had no executable evidence pinning that contract.
///
/// This is the COMMON-side (server-authoritative) input-determinism surface —
/// distinct from the voxygen keybinding UI (which key maps to which action is a
/// cosmetic client setting, not a determinism input). No graphics build needed.
#[cfg(test)]
mod inp_det_tests {
    use super::*;

    const A: InputAttr = InputAttr {
        select_pos: None,
        target_entity: None,
    };

    // Build a controller with the given inputs queued in the given order.
    fn queue(order: &[InputKind]) -> Controller {
        let mut c = Controller::default();
        for k in order {
            c.queued_inputs.insert(*k, A);
        }
        c
    }

    #[test]
    fn inp_queued_input_selection_is_insertion_order_independent() {
        use InputKind::*;
        // The SAME input set, inserted forward vs reversed.
        let set = [Primary, Block, Ability(2), Jump, Fly];
        let mut rev = set.to_vec();
        rev.reverse();
        let fwd = queue(&set);
        let bwd = queue(&rev);

        // Full processing order (BTreeMap iteration) is identical across
        // insertion orders AND equals the canonical InputKind order.
        let keys_fwd: Vec<InputKind> = fwd.queued_inputs.keys().copied().collect();
        let keys_bwd: Vec<InputKind> = bwd.queued_inputs.keys().copied().collect();
        assert_eq!(
            keys_fwd, keys_bwd,
            "iteration order must not depend on insertion order"
        );
        assert_eq!(
            keys_fwd,
            vec![Primary, Block, Ability(2), Jump, Fly],
            "must be canonical InputKind order (Primary<Block<Ability<Jump<Fly)"
        );

        // The SELECTED input (`.keys().next()` — what the state machine acts
        // on) is the canonical min regardless of insertion order.
        assert_eq!(fwd.queued_inputs.keys().next(), Some(&Primary));
        assert_eq!(bwd.queued_inputs.keys().next(), Some(&Primary));
    }

    #[test]
    fn inp_selection_is_non_vacuous() {
        use InputKind::*;
        // A DIFFERENT set selects a DIFFERENT input — the contract carries
        // information, it is not a trivially-constant answer.
        let s1 = queue(&[Secondary, Jump]);
        let s2 = queue(&[Block, Fly]);
        assert_eq!(s1.queued_inputs.keys().next(), Some(&Secondary));
        assert_eq!(s2.queued_inputs.keys().next(), Some(&Block));
        assert_ne!(
            s1.queued_inputs.keys().next(),
            s2.queued_inputs.keys().next()
        );
        // Ability(usize) orders between Block and Roll (variant discriminant),
        // and by its payload within — so Ability(0) is the min here.
        let s3 = queue(&[Jump, Ability(0), Roll]);
        assert_eq!(s3.queued_inputs.keys().next(), Some(&Ability(0)));
    }
}
