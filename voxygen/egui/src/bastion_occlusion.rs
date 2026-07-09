//! bastion (B1.6): the overseer occlusion & transparency debug panel.
//!
//! Self-contained plain data (no dependency on voxygen's `bastion::occlusion`
//! types — this crate sits below voxygen). The host reads the current state
//! into [`BastionOcclusionEguiState`] each frame, this window edits a copy, and
//! any change is emitted as `EguiAction::SetBastionOcclusion` for the host to
//! apply back to the scene. Structured to drop into the B9 settings tab.

use crate::{EguiAction, EguiActions};
use egui::{Context, Slider, Window};

/// A flat snapshot of the overseer occlusion controls, passed host→panel and
/// (on edit) panel→host. `view_mode`: 0 = Solid, 1 = Reveal, 2 = Slice.
#[derive(Clone, Copy, PartialEq)]
pub struct BastionOcclusionEguiState {
    pub view_mode: u32,
    pub strength: f32,
    pub relight_strength: f32,
    pub cutaway_radius: f32,
    pub fade_band: f32,
    pub slice_enabled: bool,
    pub proximity_enabled: bool,
    pub cutaway_enabled: bool,
    pub roof_enabled: bool,
    pub has_slice: bool,
}

pub fn draw_bastion_occlusion_window(
    ctx: &Context,
    open: &mut bool,
    egui_actions: &mut EguiActions,
    state: Option<BastionOcclusionEguiState>,
) {
    let Some(cur) = state else {
        // Not in overseer mode — nothing to control.
        *open = false;
        return;
    };
    let mut s = cur;
    Window::new("Overseer Occlusion")
        .open(open)
        .default_width(260.0)
        .show(ctx, |ui| {
            ui.label("View mode");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut s.view_mode, 0, "Solid");
                ui.selectable_value(&mut s.view_mode, 1, "Reveal");
                ui.selectable_value(&mut s.view_mode, 2, "Slice");
            });
            ui.separator();
            ui.label("Behaviors (gate the preset)");
            ui.checkbox(&mut s.roof_enabled, "Roof reveal");
            ui.checkbox(&mut s.cutaway_enabled, "Camera → target cutaway");
            ui.checkbox(&mut s.proximity_enabled, "Proximity / height fade");
            ui.add_enabled(
                s.has_slice,
                egui::Checkbox::new(&mut s.slice_enabled, "Manual slice"),
            );
            ui.separator();
            ui.add(
                Slider::new(&mut s.strength, 0.0..=1.0)
                    .clamping(egui::SliderClamping::Always)
                    .text("Transparency strength"),
            );
            ui.add(
                Slider::new(&mut s.relight_strength, 0.0..=1.5)
                    .clamping(egui::SliderClamping::Always)
                    .text("Interior relight"),
            );
            ui.add(
                Slider::new(&mut s.cutaway_radius, 1.0..=24.0)
                    .clamping(egui::SliderClamping::Always)
                    .text("Cutaway radius"),
            );
            ui.add(
                Slider::new(&mut s.fade_band, 0.5..=32.0)
                    .clamping(egui::SliderClamping::Always)
                    .text("Slice fade band"),
            );
        });

    if s != cur {
        egui_actions
            .actions
            .push(EguiAction::SetBastionOcclusion(s));
    }
}
