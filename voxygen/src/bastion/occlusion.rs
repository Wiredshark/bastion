//! bastion: the unified overseer occlusion & transparency system (B1.6).
//!
//! B1 gave the overseer a hard Z-slice (`discard` if height > `slice_z`). B1.6
//! generalizes that single hook into **one** framework: a uniform block plus a
//! shared shader alpha function, driven four composable ways. See
//! `docs/BASTION_B1_6_FINDINGS.md` and `assets/voxygen/shaders/include/
//! bastion_occlusion.glsl` (the shader half).
//!
//! The four behaviors are all the *same fragment operation* — a dithered
//! screen-door discard whose alpha is a function instead of a constant:
//! - **Slice** — B1's manual cut, upgraded to a smooth fade band.
//! - **Proximity** — geometry fades by height above / distance from the focus.
//! - **Cutaway** — geometry between the camera and tracked targets fades.
//! - **Roof reveal** — geometry in a slab above the focus (near it) fades.
//!
//! This module owns the CPU-side state + the packing into the `Globals`
//! occlusion fields; the alpha math lives in the shader.

use vek::*;

/// Mode bitmask bits (mirror `bastion_occlusion.glsl`). Compose freely.
pub mod mode {
    pub const SOLID: u32 = 0;
    pub const SLICE: u32 = 1;
    pub const PROXIMITY: u32 = 2;
    pub const CUTAWAY: u32 = 4;
    pub const ROOF: u32 = 8;
}

/// Max cutaway targets (matches the `bastion_occ_targets[4]` shader array).
pub const MAX_TARGETS: usize = 4;

/// The player-facing view mode cycled by the view-mode key. Each is a preset
/// over the composable mode bits; per-mode toggles + the slider still tweak the
/// underlying `Occlusion` fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Vanilla look — nothing hidden.
    Solid,
    /// The smart default: roof reveal + camera-to-target cutaway + a gentle
    /// proximity fade (the "auto" occlusion).
    #[default]
    Reveal,
    /// Manual cross-section: the soft Z-slice (+ interior relight).
    Slice,
}

impl ViewMode {
    pub fn next(self) -> Self {
        match self {
            ViewMode::Solid => ViewMode::Reveal,
            ViewMode::Reveal => ViewMode::Slice,
            ViewMode::Slice => ViewMode::Solid,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Solid => "Solid",
            ViewMode::Reveal => "Reveal",
            ViewMode::Slice => "Slice",
        }
    }
}

/// CPU-side occlusion state. Lives on the `Scene`; packed into `Globals` each
/// frame. Overseer-only — outside overseer mode the scene packs
/// [`OcclusionUniform::solid`] so vanilla/char-select are untouched.
#[derive(Clone, Debug)]
pub struct Occlusion {
    pub view_mode: ViewMode,
    /// Per-mode enable toggles (let the debug panel flip one behavior without
    /// leaving the current view preset).
    pub slice_enabled: bool,
    pub proximity_enabled: bool,
    pub cutaway_enabled: bool,
    pub roof_enabled: bool,

    /// Active manual slice height (world Z). `None` = no manual slice set.
    pub slice_z: Option<f32>,
    /// Smooth fade band below the slice / reveal edges (blocks).
    pub fade_band: f32,

    /// Proximity fade: 0..1 overall strength (the transparency slider).
    pub strength: f32,
    /// Height above focus at which fading starts / is complete (blocks).
    pub height_start: f32,
    pub height_end: f32,
    /// XY distance from focus at which fading starts / is complete (blocks).
    pub dist_start: f32,
    pub dist_end: f32,

    /// Cutaway cylinder radius (blocks).
    pub cutaway_radius: f32,
    /// Roof-reveal slab above focus (blocks).
    pub roof_low: f32,
    pub roof_high: f32,
    /// Interior re-lighting strength (added top-down fill).
    pub relight_strength: f32,

    /// Cutaway targets, absolute world positions. Stubbed this block (focus +
    /// debug markers); B2 feeds hovered/selected entities, B3 colonists.
    pub targets: Vec<Vec3<f32>>,
}

impl Default for Occlusion {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::default(),
            slice_enabled: true,
            proximity_enabled: true,
            cutaway_enabled: true,
            roof_enabled: true,
            slice_z: None,
            fade_band: 6.0,
            strength: 0.85,
            height_start: 8.0,
            height_end: 40.0,
            dist_start: 48.0,
            dist_end: 160.0,
            cutaway_radius: 6.0,
            roof_low: 3.0,
            roof_high: 14.0,
            relight_strength: 0.5,
            targets: Vec::new(),
        }
    }
}

impl Occlusion {
    /// The active mode bitmask, combining the view-mode preset with the
    /// per-mode toggles.
    pub fn active_mode(&self) -> u32 {
        let preset = match self.view_mode {
            ViewMode::Solid => mode::SOLID,
            // "Reveal" is the auto occlusion: roof + cutaway + a gentle
            // proximity readability layer.
            ViewMode::Reveal => mode::ROOF | mode::CUTAWAY | mode::PROXIMITY,
            // "Slice" is the manual cross-section (+ a touch of proximity).
            ViewMode::Slice => mode::SLICE | mode::PROXIMITY,
        };
        let mut m = preset;
        // Per-mode toggles gate the bits (they can only *remove* from the
        // preset, so a toggle never turns on a behavior the preset excludes —
        // keeps the mental model simple).
        if !self.slice_enabled {
            m &= !mode::SLICE;
        }
        if !self.proximity_enabled {
            m &= !mode::PROXIMITY;
        }
        if !self.cutaway_enabled {
            m &= !mode::CUTAWAY;
        }
        if !self.roof_enabled {
            m &= !mode::ROOF;
        }
        // The slice bit only means anything once a slice height exists.
        if self.slice_z.is_none() {
            m &= !mode::SLICE;
        }
        m
    }

    /// Pack into the GPU uniform for a given camera focus.
    ///
    /// `focus_off` is `focus.trunc()` (matches `Globals.focus_off`); targets
    /// are emitted in `f_pos` space (`world - focus_off`) so the shader avoids
    /// huge-float subtraction.
    pub fn to_uniform(&self, focus_pos: Vec3<f32>) -> OcclusionUniform {
        let focus_off = focus_pos.map(|e| e.trunc());
        let mode = self.active_mode();

        let mut targets = [[0.0f32; 4]; MAX_TARGETS];
        let mut count = 0u32;
        for t in self.targets.iter().take(MAX_TARGETS) {
            let rel = *t - focus_off;
            targets[count as usize] = [rel.x, rel.y, rel.z, 1.0];
            count += 1;
        }

        OcclusionUniform {
            mode: [mode, count, 0, 0],
            a: [
                self.slice_z.unwrap_or(f32::MAX),
                self.fade_band.max(0.001),
                focus_pos.z,
                self.strength.clamp(0.0, 1.0),
            ],
            b: [
                self.height_start,
                self.height_end.max(self.height_start + 0.001),
                self.dist_start,
                self.dist_end.max(self.dist_start + 0.001),
            ],
            c: [
                self.cutaway_radius.max(0.001),
                self.roof_low,
                self.roof_high.max(self.roof_low + 0.001),
                self.relight_strength.max(0.0),
            ],
            targets,
        }
    }
}

/// The packed GPU form — copied field-for-field into `Globals`. Layout mirrors
/// `bastion_occlusion.glsl`.
#[derive(Copy, Clone, Debug)]
pub struct OcclusionUniform {
    pub mode: [u32; 4],
    pub a: [f32; 4],
    pub b: [f32; 4],
    pub c: [f32; 4],
    pub targets: [[f32; 4]; MAX_TARGETS],
}

impl OcclusionUniform {
    /// The "nothing hidden" uniform (mode 0) — vanilla look. Used outside
    /// overseer mode and in the char-select scene.
    pub fn solid() -> Self {
        Self {
            mode: [mode::SOLID, 0, 0, 0],
            a: [f32::MAX, 1.0, 0.0, 0.0],
            b: [1.0, 2.0, 1.0, 2.0],
            c: [1.0, 1.0, 2.0, 0.0],
            targets: [[0.0; 4]; MAX_TARGETS],
        }
    }
}
