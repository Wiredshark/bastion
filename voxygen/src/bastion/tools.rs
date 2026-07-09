//! bastion (B2a): overseer tool-palette state + the God/Free ruleset hook.
//!
//! The interaction surface is identical in both rulesets (§3c); only the
//! *rules* differ. `target_allowed` is the restriction hook — stubbed
//! permissive here, enforced in B2b (colony membership from B3, favor/
//! cooldown metering from B13).

use common::bastion::DesignationKind;

/// The pinned interaction tool (the palette). `Pan` is the cursor default:
/// drag pans, click selects. `Designate` turns left-drag into region paint;
/// `Erase` (B5.5) is the same drag but cancels designations in the region.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum ToolMode {
    #[default]
    Pan,
    Inspect,
    Designate(DesignationKind),
    Erase,
}

impl ToolMode {
    pub const ALL: [ToolMode; 7] = [
        ToolMode::Pan,
        ToolMode::Inspect,
        ToolMode::Designate(DesignationKind::Mine),
        ToolMode::Designate(DesignationKind::Chop),
        ToolMode::Designate(DesignationKind::Build),
        ToolMode::Designate(DesignationKind::Stockpile),
        ToolMode::Erase,
    ];

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ToolMode::Pan => "Pan",
            ToolMode::Inspect => "Inspect",
            ToolMode::Designate(k) => k.label(),
            ToolMode::Erase => "Erase",
        }
    }
}

/// bastion (B5.6a): designation-visuals display mode — purely VISUAL, zero
/// sim impact (designations stay fully active in every mode). `On` = full
/// overlays (outlines now; fills/volumes in B5.6b); `Subtle` = dimmed thin
/// outlines only (situational awareness without clutter); `Off` = nothing
/// rendered (pure colony-watching). Painting/erasing auto-reveals (see the
/// session's tool handling) so you can always see what you paint.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum VisualsMode {
    #[default]
    On,
    Subtle,
    Off,
}

impl VisualsMode {
    pub fn next(self) -> Self {
        match self {
            VisualsMode::On => VisualsMode::Subtle,
            VisualsMode::Subtle => VisualsMode::Off,
            VisualsMode::Off => VisualsMode::On,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            VisualsMode::On => "Visuals: On",
            VisualsMode::Subtle => "Visuals: Subtle",
            VisualsMode::Off => "Visuals: Off",
        }
    }

    /// Nothing rendered.
    pub fn is_off(&self) -> bool { matches!(self, VisualsMode::Off) }

    /// Alpha multiplier for overlay lines (Subtle dims them).
    pub fn line_alpha(&self) -> f32 {
        match self {
            VisualsMode::On => 1.0,
            VisualsMode::Subtle => 0.45,
            VisualsMode::Off => 0.0,
        }
    }
}

/// bastion (B5.6b-1): the zone-type colour legend — one RGB per designation
/// kind (Mine/Chop/Build/Stockpile). Borders draw it near-opaque; fills draw
/// it low-alpha so overlapping zones alpha-composite into a visibly blended
/// colour. One legend so the outline, fill, and label all agree.
pub fn zone_rgb(kind: DesignationKind) -> [f32; 3] {
    match kind {
        DesignationKind::Mine => [1.0, 0.6, 0.1],
        DesignationKind::Chop => [0.2, 0.9, 0.2],
        DesignationKind::Build => [0.3, 0.6, 1.0],
        DesignationKind::Stockpile => [0.85, 0.35, 0.95],
    }
}

/// Border colour (draped outline) for a zone kind, scaled by an alpha
/// multiplier (SUBTLE dims via `VisualsMode::line_alpha`).
pub fn zone_border_color(kind: DesignationKind, alpha_mul: f32) -> [f32; 4] {
    let [r, g, b] = zone_rgb(kind);
    [r, g, b, 0.9 * alpha_mul]
}

/// Fill colour (translucent conformed area) for a zone kind. Low alpha so
/// overlaps blend and the terrain reads through.
pub fn zone_fill_color(kind: DesignationKind) -> [f32; 4] {
    let [r, g, b] = zone_rgb(kind);
    [r, g, b, 0.22]
}

/// God mode (the real game: colony-only targets, metered force-actions) vs
/// Free mode (sandbox: no restrictions). Stub in B2a; teeth in B2b.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum GodMode {
    #[default]
    God,
    Free,
}

impl GodMode {
    pub fn toggled(self) -> Self {
        match self {
            GodMode::God => GodMode::Free,
            GodMode::Free => GodMode::God,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            GodMode::God => "God mode",
            GodMode::Free => "Free mode",
        }
    }
}

/// THE target-restriction hook (§3c). B2a: permissive stub — there is no
/// colony yet (B3) and no favor economy (B13). B2b replaces the body with:
/// God mode → target must be under the player's influence (colony member),
/// and force-actions must pass the favor⇄cooldown meter; Free mode → always
/// true.
pub fn target_allowed(_mode: GodMode, _is_colony_member: Option<bool>) -> bool { true }

/// Live overseer interaction state (session-owned).
#[derive(Default)]
pub struct Tools {
    pub tool: ToolMode,
    pub god_mode: GodMode,
}
