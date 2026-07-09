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
            // Off by default — their B1.6 masks are approximate/stubbed and
            // artifact as an always-on default; tick them in the egui panel to
            // demo, they rejoin the auto-default with real data in B2/B3.
            cutaway_enabled: false,
            roof_enabled: false,
            slice_z: None,
            fade_band: 6.0,
            // Reveal = tall geometry *near the view center* fades (see the
            // proximity block in bastion_occlusion.glsl: height fade × central
            // window). Strong enough to be clearly visible on canopy/cliffs
            // over the focus, while the window keeps the distant panorama
            // solid (QA round 1: "cuts through mountains"; QA round 3:
            // "reveal does nothing").
            strength: 0.6,
            height_start: 12.0,
            height_end: 60.0,
            // The central window, as a *fraction of the on-screen view radius*
            // (zoom-scaled at pack time): fully active around the focus,
            // easing off toward the screen edge so background terrain stays
            // untouched.
            dist_start: 0.55,
            dist_end: 1.0,
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
            // "Reveal" = a gentle height-proximity readability layer (tall
            // foreground softly fades so you see the ground). ROOF and CUTAWAY
            // are *not* in the default preset because their B1.6 masks are
            // approximate/stubbed and produce artifacts as an always-on
            // default — roof's height-slab-in-a-radius reveals a whole cave as
            // a circular hole, and a camera→focus cutaway punches the
            // foreground. Both are opt-in via the egui toggles (demonstrable on
            // a building), and rejoin the auto-default once B2/B3 feed real
            // per-room coverage + hovered/colonist targets.
            ViewMode::Reveal => mode::PROXIMITY,
            // "Slice" is the manual cross-section (+ a touch of proximity).
            ViewMode::Slice => mode::SLICE | mode::PROXIMITY,
        };
        // Solid is always truly solid, whatever the toggles say.
        if self.view_mode == ViewMode::Solid {
            return mode::SOLID;
        }
        // Per-behavior toggles compose *on top of* the preset — they can add a
        // behavior (e.g. tick Roof in the egui panel to demo roof reveal on a
        // building) or remove one. Defaults keep roof/cutaway off so the
        // Reveal preset stays proximity-only.
        let mut m = preset;
        let set = |m: u32, bit: u32, on: bool| if on { m | bit } else { m & !bit };
        m = set(m, mode::PROXIMITY, self.proximity_enabled);
        m = set(m, mode::ROOF, self.roof_enabled);
        m = set(m, mode::CUTAWAY, self.cutaway_enabled);
        // The slice is *preset-gated*: it only cuts in the Slice view mode
        // (where its toggle can still disable it), never composes into Reveal.
        // Without this, a leftover slice_z made Reveal == Slice — a hard
        // ground cut in both, and the view-mode key appeared to do nothing.
        m = set(
            m,
            mode::SLICE,
            preset & mode::SLICE != 0 && self.slice_enabled && self.slice_z.is_some(),
        );
        m
    }

    /// Pack into the GPU uniform for a given camera focus.
    ///
    /// `focus_off` is `focus.trunc()` (matches `Globals.focus_off`); targets
    /// are emitted in `f_pos` space (`world - focus_off`) so the shader avoids
    /// huge-float subtraction. `view_radius` is the on-screen ortho half-extent
    /// (blocks) — the proximity window is expressed as a fraction of it so it
    /// tracks the zoom instead of a fixed block distance. `daylight` (0 night →
    /// 1 midday) scales the interior relight: it's an additive linear-light
    /// term, so unscaled it blows the night scene out to white.
    pub fn to_uniform(
        &self,
        focus_pos: Vec3<f32>,
        view_radius: f32,
        daylight: f32,
    ) -> OcclusionUniform {
        let focus_off = focus_pos.map(|e| e.trunc());
        let mode = self.active_mode();

        let mut targets = [[0.0f32; 4]; MAX_TARGETS];
        let mut count = 0u32;
        for t in self.targets.iter().take(MAX_TARGETS) {
            let rel = *t - focus_off;
            targets[count as usize] = [rel.x, rel.y, rel.z, 1.0];
            count += 1;
        }

        let vr = view_radius.max(1.0);
        let dist_start = self.dist_start * vr;
        let dist_end = (self.dist_end * vr).max(dist_start + 0.001);

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
                dist_start,
                dist_end,
            ],
            c: [
                self.cutaway_radius.max(0.001),
                self.roof_low,
                self.roof_high.max(self.roof_low + 0.001),
                self.relight_strength.max(0.0) * daylight.clamp(0.0, 1.0),
            ],
            // Roof reveal gets its own "near the look point" radius (a fraction
            // of the view) so it stays localized instead of sharing the
            // (disabled-by-default) proximity distance range.
            d: [vr * 0.55, 0.0, 0.0, 0.0],
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
    pub d: [f32; 4],
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
            d: [1.0, 0.0, 0.0, 0.0],
            targets: [[0.0; 4]; MAX_TARGETS],
        }
    }
}
