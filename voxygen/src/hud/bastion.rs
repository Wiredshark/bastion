//! bastion (B2a): HUD state for the overseer interaction surface — the tool
//! palette, the contextual radial menu, and the selection info line.
//!
//! Deliberately conrod (not egui): egui rendering is hard-gated behind the
//! debug toggle (`settings.interface.toggle_egui_debug`), and this is
//! gameplay UI. B9's colony HUD re-skins these; B2a is the framework.

use crate::bastion::tools::{GodMode, ToolMode};
use common::bastion::{ContextTarget, ContextVerb, InfluenceKind};
use vek::*;

/// One radial entry's action.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RadialAction {
    Verb(ContextVerb),
    Influence(InfluenceKind),
    /// bastion (B5.5): delete the whole painted designation rect(s) under
    /// the clicked block. Client-resolved (the client knows its echoed
    /// rects) — never crosses the wire as a verb; the session sends one
    /// `BastionCancelDesignation` per containing rect.
    DeleteZone,
}

impl RadialAction {
    pub fn label(&self) -> &'static str {
        match self {
            RadialAction::Verb(v) => v.label(),
            RadialAction::Influence(k) => k.label(),
            RadialAction::DeleteZone => "Delete zone",
        }
    }

    /// Shown greyed: exists on the menu, but its block hasn't landed yet.
    pub fn stubbed(&self) -> bool {
        match self {
            RadialAction::Verb(v) => v.stubbed(),
            RadialAction::Influence(_) => false,
            RadialAction::DeleteZone => false,
        }
    }
}

/// An open radial menu.
pub struct BastionRadial {
    /// Context title ("Tree", "Rock", "Ground", "Entity <uid>").
    pub title: String,
    /// What the verbs act on.
    pub target: ContextTarget,
    /// World point of the context (influences aim here).
    pub point: Vec3<f32>,
    /// All actions for this context (draw shows a pie of the first few +
    /// "More…" expanding to the full list).
    pub actions: Vec<RadialAction>,
    /// "More…" was clicked — show the dense list.
    pub expanded: bool,
    /// Conrod screen position, pinned on the first draw after opening.
    pub pinned: Option<[f64; 2]>,
}

impl BastionRadial {
    pub fn new(
        title: String,
        target: ContextTarget,
        point: Vec3<f32>,
        actions: Vec<RadialAction>,
    ) -> Self {
        Self {
            title,
            target,
            point,
            actions,
            expanded: false,
            pinned: None,
        }
    }
}

/// Hud-side mirror of the overseer interaction state (set by the session
/// every frame; the HUD draws from it and answers with `hud::Event`s).
#[derive(Default)]
pub struct BastionHudState {
    /// Overseer context active (nothing draws otherwise).
    pub active: bool,
    pub tool: ToolMode,
    pub god_mode: GodMode,
    /// Info line for the current selection.
    pub selected_info: Option<String>,
    pub radial: Option<BastionRadial>,
    /// bastion (B5.6b-1): world-anchored zone labels ("Mine 1"), each at a
    /// world centroid with its kind colour. Set by the session's overlay
    /// sync (ON mode only; empty in SUBTLE/OFF). Drawn via `position_ingame`.
    pub zone_labels: Vec<(Vec3<f32>, String, [f32; 4])>,
    /// bastion (B-MAP1): active Z-slice, mirrored so the minimap tiles can
    /// render the slice level (below-ground slice → below-ground map).
    pub slice_z: Option<f32>,
    /// bastion (B5.6b-2): the live depth counter during a designate drag
    /// ("3 levels deep"), world-anchored at the drag cursor. `None` when no
    /// drag is active. Drawn exactly like `zone_labels`.
    pub paint_label: Option<(Vec3<f32>, String, [f32; 4])>,
    /// bastion (B5.6b-2): the current depth selection as a label for the
    /// tool panel's precision stepper (mirrored from the session's `Tools`,
    /// so the stepper and scroll-while-painting stay synced by construction).
    pub z_extent_label: String,
    /// bastion (B5.6b-2.1): flat-floor mode active (mirrored; drives the
    /// stepper's mode-toggle button state).
    pub flat_floor: bool,
}

/// How many actions the pie shows before overflowing into "More…".
pub const RADIAL_PIE_MAX: usize = 5;
