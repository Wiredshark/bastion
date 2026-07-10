//! bastion (B2a): overseer tool-palette state + the God/Free ruleset hook.
//!
//! The interaction surface is identical in both rulesets (§3c); only the
//! *rules* differ. `target_allowed` is the restriction hook — stubbed
//! permissive here, enforced in B2b (colony membership from B3, favor/
//! cooldown metering from B13).

use common::bastion::{DesignationKind, ZExtent};

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
    pub const ALL: [ToolMode; 8] = [
        ToolMode::Pan,
        ToolMode::Inspect,
        ToolMode::Designate(DesignationKind::Mine),
        ToolMode::Designate(DesignationKind::Chop),
        ToolMode::Designate(DesignationKind::Build),
        ToolMode::Designate(DesignationKind::Stockpile),
        // B5.8: ladders — a 1-column upward designation (drag a spot, the
        // up-extent sets the height; kind default = 4 rungs).
        ToolMode::Designate(DesignationKind::Ladder),
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
        // B5.8: ladder — wood-rung tan, distinct from Chop's leaf green.
        DesignationKind::Ladder => [0.85, 0.7, 0.35],
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

/// bastion (B5.6b-2): UI clamp for the depth selection. The server's real
/// cap is `MAX_DESIGNATION_VOLUME` (validated per footprint); this just
/// keeps the stepper/scroll sane. 32 matches the volume cap's z-span.
pub const Z_EXTENT_MAX_DOWN: u16 = 32;
pub const Z_EXTENT_MAX_UP: u16 = 8;

/// Live overseer interaction state (session-owned).
#[derive(Default)]
pub struct Tools {
    pub tool: ToolMode,
    pub god_mode: GodMode,
    /// bastion (B5.6b-2): the current designation depth (surface-relative
    /// [`ZExtent`]) the paint path sends. Scroll-while-painting and the tool
    /// panel's stepper both edit THIS field — the two selection paths are
    /// synced by construction. Persists across paints; reset to the kind
    /// default when the designate kind changes (see the session's tool
    /// cycling).
    pub z_extent: ZExtent,
    /// bastion (B5.6b-2.1, Ben's flat-floor mode): when set, the paint's
    /// depth measures from the CLICKED plane and every column digs to that
    /// one shared absolute level — flat, square pit bottoms (quarries /
    /// foundations / plazas) instead of slope-following. The `floor_z` is
    /// derived at paint time (plane − down); this flag is the mode toggle
    /// on the depth stepper.
    pub flat_floor: bool,
}

impl Tools {
    /// Step the depth selection: positive = deeper (down+), negative =
    /// shallower; below down=0 the steps extend UPWARD instead (up+), so one
    /// axis scrolls through the whole sensible range:
    /// `up=MAX_UP .. up=0/down=0 .. down=MAX_DOWN`.
    pub fn step_z_extent(&mut self, steps: i32) {
        let signed = self.z_extent.down as i32 - self.z_extent.up as i32 + steps;
        if signed >= 0 {
            self.z_extent.down = (signed as u16).min(Z_EXTENT_MAX_DOWN);
            self.z_extent.up = 0;
        } else {
            self.z_extent.down = 0;
            self.z_extent.up = ((-signed) as u16).min(Z_EXTENT_MAX_UP);
        }
    }

    /// The live counter string for the depth UX ("3 levels deep").
    pub fn z_extent_label(&self) -> String {
        let base = match (self.z_extent.down, self.z_extent.up) {
            (d, 0) => format!("{} levels deep", d + 1),
            (0, u) => format!("{} levels up", u + 1),
            (d, u) => format!("{} levels ({} down, {} up)", d as u32 + 1 + u as u32, d, u),
        };
        if self.flat_floor {
            format!("{base} · FLAT floor")
        } else {
            base
        }
    }
}
