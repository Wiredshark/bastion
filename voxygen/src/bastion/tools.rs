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
