//! bastion: the input-context system (design doc §3b, built in B1.5).
//!
//! Three interaction modes own three separate control schemes instead of
//! fighting over one keymap:
//!
//! - [`InputContext::Menu`] — vanilla. Strict passthrough; active whenever the
//!   `--bastion-overseer` flag is off (regression safety) or no session runs.
//! - [`InputContext::Overseer`] — god mode. B&W2 camera controls + (from B2)
//!   designation/influence tools. Avatar verbs are suppressed, which is what
//!   structurally kills the B1 "HUD steals Q for hotbar slot 10" bug class.
//! - [`InputContext::Avatar`] — embodied (B12; stubbed since B1.5): controls
//!   are exactly vanilla Veloren, overseer keys are suppressed.
//!
//! The active context lives on [`crate::window::Window`] and filters the
//! physical-key → `GameInput` fan-out *at the source* (`Window::map_input`'s
//! push sites), before the HUD or session ever see an event. Switching context
//! is a single enum write — the whole binding table swaps atomically.
//!
//! Movement (WASD) is deliberately shared across Overseer/Avatar: it pans the
//! god camera in one and moves the body in the other. Everything else is
//! mode-specific.
//!
//! Rebinding: no UI yet (deliberate; B9). The per-context [`ContextScheme`]
//! tables are the data model a per-mode keybind tab edits later — B9 adds an
//! `overrides: HashMap<GameInput, KeyMouse>` field per context and nothing
//! else has to change.

use crate::game_input::GameInput;
use hashbrown::HashSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputContext {
    /// Vanilla passthrough (also: no session / flag off).
    #[default]
    Menu,
    /// God mode: the overseer camera and its tools own the scheme.
    Overseer,
    /// Embodied in a body with exactly vanilla controls (B12 stub).
    Avatar,
}

/// A context's control scheme. `suppressed` are GameInputs from *other*
/// schemes that must not fire while this context is active (they share
/// physical keys or would act on the wrong consumer).
pub struct ContextScheme {
    /// Inputs this context owns/uses — documentation + future rebind-UI
    /// grouping (B9 shows one tab per context listing exactly these).
    pub owned: &'static [GameInput],
    pub suppressed: &'static [GameInput],
}

/// Menu / vanilla: nothing suppressed, everything routes exactly as upstream.
pub const MENU_SCHEME: ContextScheme = ContextScheme {
    owned: &[],
    suppressed: &[],
};

/// Overseer (god mode). WASD pans; mouse grab-drags/orbits/zooms; PgUp/PgDn
/// slice; Home snaps top-down; F9 embodies (stub). Avatar verbs are dead —
/// including `Slot10`/`Interact`, whose physical keys (Q/E) the overseer owns
/// for rotation, and `CycleCamera`/`ToggleCursor`/`SpectateViewpoint`, which
/// would break the overseer camera/free-cursor invariants.
pub const OVERSEER_SCHEME: ContextScheme = ContextScheme {
    owned: &[
        GameInput::MoveForward,
        GameInput::MoveBack,
        GameInput::MoveLeft,
        GameInput::MoveRight,
        GameInput::BastionToggleOverseer,
        GameInput::BastionRotateLeft,
        GameInput::BastionRotateRight,
        GameInput::BastionSliceUp,
        GameInput::BastionSliceDown,
        GameInput::BastionSnapTopDown,
    ],
    suppressed: &[
        GameInput::Primary,
        GameInput::Secondary,
        GameInput::Block,
        GameInput::Roll,
        GameInput::Jump,
        GameInput::WallJump,
        GameInput::SwimUp,
        GameInput::SwimDown,
        GameInput::Glide,
        GameInput::Fly,
        GameInput::Sneak,
        GameInput::CancelClimb,
        GameInput::ToggleWield,
        GameInput::SwapLoadout,
        GameInput::Sit,
        GameInput::Crawl,
        GameInput::Dance,
        GameInput::Greet,
        GameInput::Mount,
        GameInput::StayFollow,
        GameInput::Interact,
        GameInput::Trade,
        GameInput::ToggleLantern,
        GameInput::GiveUp,
        GameInput::Respawn,
        GameInput::FreeLook,
        GameInput::AutoWalk,
        GameInput::ToggleWalk,
        GameInput::CameraClamp,
        GameInput::ZoomLock,
        GameInput::Slot1,
        GameInput::Slot2,
        GameInput::Slot3,
        GameInput::Slot4,
        GameInput::Slot5,
        GameInput::Slot6,
        GameInput::Slot7,
        GameInput::Slot8,
        GameInput::Slot9,
        GameInput::Slot10,
        GameInput::PreviousSlot,
        GameInput::NextSlot,
        GameInput::CurrentSlot,
        GameInput::Select,
        GameInput::CycleCamera,
        GameInput::ToggleCursor,
        GameInput::SpectateViewpoint,
        GameInput::SpectateSpeedBoost,
    ],
};

/// Avatar (embodied; stub until B12 wires real possession): exactly vanilla,
/// with the overseer-only keys dead. `BastionToggleOverseer` stays live — it
/// is the release/embody switch.
pub const AVATAR_SCHEME: ContextScheme = ContextScheme {
    owned: &[
        GameInput::MoveForward,
        GameInput::MoveBack,
        GameInput::MoveLeft,
        GameInput::MoveRight,
        GameInput::BastionToggleOverseer,
    ],
    suppressed: &[
        GameInput::BastionRotateLeft,
        GameInput::BastionRotateRight,
        GameInput::BastionSliceUp,
        GameInput::BastionSliceDown,
        GameInput::BastionSnapTopDown,
    ],
};

pub fn scheme(context: InputContext) -> &'static ContextScheme {
    match context {
        InputContext::Menu => &MENU_SCHEME,
        InputContext::Overseer => &OVERSEER_SCHEME,
        InputContext::Avatar => &AVATAR_SCHEME,
    }
}

/// The active context + its precomputed suppression set. Lives on `Window`
/// so the key→GameInput fan-out can filter at the single chokepoint.
#[derive(Default)]
pub struct InputContextState {
    active: InputContext,
    suppressed: HashSet<GameInput>,
}

impl InputContextState {
    pub fn active(&self) -> InputContext { self.active }

    /// Atomic whole-scheme swap (one call; no key-by-key toggling).
    pub fn set(&mut self, context: InputContext) {
        if self.active != context {
            self.active = context;
            self.suppressed = scheme(context).suppressed.iter().copied().collect();
        }
    }

    /// Is this GameInput live under the active context? Called by the window
    /// fan-out for every candidate binding of a pressed key.
    pub fn is_live(&self, input: GameInput) -> bool { !self.suppressed.contains(&input) }

    /// The overseer runs with a free (ungrabbed) cursor but still needs the
    /// mouse wheel to reach the game as `Event::Zoom`.
    pub fn wheel_while_free(&self) -> bool { self.active == InputContext::Overseer }
}
