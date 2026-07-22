pub mod interactable;
pub mod settings_change;
mod target;

use std::{cell::RefCell, collections::HashSet, rc::Rc, result::Result, time::Duration};

use itertools::Itertools;
#[cfg(not(target_os = "macos"))]
use mumble_link::SharedLink;
use ordered_float::OrderedFloat;
use specs::WorldExt;
use tracing::{error, info};
use vek::*;

use client::{self, Client};
use common::{
    CachedSpatialGrid,
    comp::{
        self, CharacterActivity, CharacterState, ChatType, Content, Fluid, InputKind,
        InventoryUpdateEvent, Pos, PresenceKind, Stats, UtteranceKind, Vel,
        inventory::slot::{EquipSlot, Slot},
        invite::InviteKind,
        item::{ItemDesc, tool::ToolKind},
    },
    consts::MAX_MOUNT_RANGE,
    event::UpdateCharacterMetadata,
    link::Is,
    mounting::{Mount, VolumePos},
    outcome::Outcome,
    recipe::{self, RecipeBookManifest},
    terrain::{Block, BlockKind},
    trade::TradeResult,
    util::{Dir, Plane},
    vol::ReadVol,
};
use common_base::{prof_span, span};
use common_net::{msg::server::InviteAnswer, sync::WorldSyncExt};

use crate::{
    Direction, GlobalState, PlayState, PlayStateResult,
    audio::sfx::SfxEvent,
    bastion,
    cmd::run_command,
    error::Error,
    game_input::GameInput,
    hud::{
        AutoPressBehavior, DebugInfo, Event as HudEvent, Hud, HudCollectFailedReason, HudInfo,
        LootMessage, PersistedHudState, PromptDialogSettings,
    },
    key_state::KeyState,
    menu::{char_selection::CharSelectionState, main::get_client_msg_error},
    render::{Drawer, GlobalsBindGroup},
    scene::{CameraMode, DebugShapeId, Scene, SceneData, camera},
    session::target::ray_entities,
    settings::Settings,
    window::{AnalogGameInput, Event},
};
use hashbrown::HashMap;
use interactable::{BlockInteraction, EntityInteraction, Interactable, get_interactables};
use settings_change::Language::ChangeLanguage;
use target::targets_under_cursor;
#[cfg(feature = "egui-ui")]
use voxygen_egui::EguiDebugInfo;

/** The zoom scroll delta that is considered an "intent"
    to zoom, rather than the accidental zooming that Zoom Lock
    is supposed to help.
    This is used for both [AutoPressBehaviors::Toggle] and [AutoPressBehaviors::Auto].

    This value should likely differ between trackpad scrolling
    and various mouse wheels, but we just choose a reasonable
    default.

    All the mice I have can only scroll at |delta|=15 no matter
    how fast, I guess the default should be less than that so
    it gets seen. This could possibly be a user setting changed
    only in a config file; it's too minor to put in the GUI.
    If a player reports that their scroll wheel is apparently not
    working, this value may be to blame (i.e. their intent to scroll
    is not being detected at a low enough scroll speed).
*/
const ZOOM_LOCK_SCROLL_DELTA_INTENT: f32 = 14.0;

/// The action to perform after a tick
enum TickAction {
    // Continue executing
    Continue,
    // Disconnected (i.e. go to main menu)
    Disconnect,
}

#[derive(Default)]
pub struct PlayerDebugLines {
    pub chunk_normal: Option<DebugShapeId>,
    pub wind: Option<DebugShapeId>,
    pub fluid_vel: Option<DebugShapeId>,
    pub vel: Option<DebugShapeId>,
}

pub struct SessionState {
    scene: Scene,
    pub(crate) client: Rc<RefCell<Client>>,
    metadata: UpdateCharacterMetadata,
    pub(crate) hud: Hud,
    key_state: KeyState,
    inputs: comp::ControllerInputs,
    inputs_state: HashSet<GameInput>,
    selected_block: Block,
    walk_forward_dir: Vec2<f32>,
    walk_right_dir: Vec2<f32>,
    free_look: bool,
    freecam_pos: Vec3<f32>,
    auto_walk: bool,
    walking_speed: bool,
    camera_clamp: bool,
    zoom_lock: bool,
    is_aiming: bool,
    pub(crate) target_entity: Option<specs::Entity>,
    pub(crate) selected_entity: Option<(specs::Entity, std::time::Instant)>,
    pub(crate) viewpoint_entity: Option<specs::Entity>,
    interactables: interactable::Interactables,
    #[cfg(not(target_os = "macos"))]
    mumble_link: SharedLink,
    hitboxes: HashMap<specs::Entity, DebugShapeId>,
    lines: PlayerDebugLines,
    tracks: HashMap<Vec2<i32>, Vec<DebugShapeId>>,
    gizmos: Vec<(DebugShapeId, common::resources::Time, bool)>,
    /// bastion: `--bastion-overseer` was passed but the overseer camera entry
    /// is deferred until the player entity has a position to focus on.
    bastion_pending_overseer: bool,
    /// bastion: active grab-drag (B&W2 pan) — the world point under the
    /// cursor at grab time, which must stay locked under the cursor.
    bastion_grab: Option<BastionGrab>,
    /// bastion: smoothed pan velocity; carries eased inertia after release.
    bastion_pan_vel: Vec2<f32>,
    /// bastion: right-button orbit (free yaw + pitch) in progress.
    bastion_orbiting: bool,
    /// bastion (B2a): tool palette + God/Free ruleset state.
    bastion_tools: bastion::tools::Tools,
    /// bastion (B2a): cursor position at LMB/RMB press — release within a few
    /// pixels is a *click* (select / radial) instead of a drag (pan / orbit).
    bastion_lmb_down: Option<Vec2<f32>>,
    bastion_rmb_down: Option<Vec2<f32>>,
    /// bastion (B2a): in-progress designate-paint drag.
    bastion_paint: Option<BastionPaint>,
    /// bastion (B3): in-progress Inspect-tool box-select drag (same shape as
    /// a paint drag; different release semantics).
    bastion_boxsel: Option<BastionPaint>,
    /// bastion (B2a/B3): the current selection (mirrors the
    /// `BastionSelected` ECS markers; multiple via box-select).
    bastion_selected: Vec<specs::Entity>,
    /// bastion (B3): overhead marker shapes for loaded colonists.
    bastion_colonist_markers: HashMap<specs::Entity, DebugShapeId>,
    /// bastion (UI-4.1): world-space highlight rings under the SELECTED
    /// colonists — a flat ground disc that tracks each one, so a picked
    /// colonist reads at a glance in the world (not just the HUD line).
    /// Keyed by entity, synced against `bastion_selected` (the same
    /// marker-sync shape as the overhead markers).
    bastion_selection_rings: HashMap<specs::Entity, DebugShapeId>,
    /// bastion (B2a): how many echoed designations already have overlay
    /// shapes, + those shapes (debug-pipeline line rectangles).
    /// Last-seen `Client::bastion_designations_rev` (B5.5: rebuild-on-rev).
    bastion_designation_synced: u64,
    bastion_designation_shapes: Vec<DebugShapeId>,
    /// bastion (UI-4 row 62 → UI-5 row 62.2): the inspector request throttle —
    /// the last (target, send time); re-sends immediately on target change,
    /// else at ~1Hz while the same object stays inspected.
    bastion_inspect_sent: Option<(
        common::comp::bastion::BastionInspectTarget,
        std::time::Instant,
    )>,
    /// bastion (UI-5, row 62.2): the world CELL the overseer clicked to inspect
    /// a non-colonist object (a job / stockpile / farm / fell-set). Set by an
    /// empty-handed left-click that hits no colonist; cleared when a colonist
    /// is selected or the click hits nothing Bastion-tracked.
    bastion_inspect_cell: Option<vek::Vec3<i32>>,
    /// bastion (B5.6a): the Z-slice height the draped overlay was built
    /// against — a slice toggle re-clamps the draped surface, so the overlay
    /// rebuilds when this changes even if the rev didn't.
    bastion_designation_slice: Option<f32>,
    /// bastion (B5.6a): the designation-visuals display mode.
    bastion_visuals: crate::bastion::tools::VisualsMode,
    /// bastion (B5.6a): force a designation-overlay rebuild next frame (set
    /// when the visuals mode toggles, since that isn't captured by rev/slice).
    bastion_designation_dirty: bool,
}

/// bastion: state of an overseer grab-drag.
#[derive(Clone, Copy)]
struct BastionGrab {
    /// World point grabbed (on `plane_z`).
    anchor: Vec3<f32>,
    /// Height of the picking plane (active slice, else focus ground height),
    /// frozen for the drag so the lock is stable.
    plane_z: f32,
}

/// bastion (B2a): state of a designate-paint drag.
struct BastionPaint {
    /// World point of the initial press (on `plane_z`).
    anchor: Vec3<f32>,
    /// Current drag corner (same plane).
    current: Vec3<f32>,
    /// Frozen picking-plane height for the drag.
    plane_z: f32,
    /// Live preview outline (rebuilt as the corner moves).
    shapes: Vec<DebugShapeId>,
}

// bastion: overseer camera feel tunables (B1/B1.5; see docs/BASTION_CAMERA.md)
/// Pan speed of the overseer ground target, in units/s per unit of zoom
/// (`dist`) — zoomed out pans proportionally faster. (WASD fallback pan.)
const BASTION_PAN_FACTOR: f32 = 1.0;
/// Z-slice movement speed while PgUp/PgDn is held, in blocks/s.
const BASTION_SLICE_RATE: f32 = 16.0;
/// Minimum clearance the overseer camera (and its sight line to the focus)
/// keeps above the terrain surface, in blocks (B&W2: never under the world).
const BASTION_CAM_MARGIN: f32 = 6.0;
/// Free-orbit sensitivity, radians per pixel of right-drag.
const BASTION_ORBIT_SENS: f32 = 0.0035;
/// Exponential decay rate (1/s) of the grab-release inertia.
const BASTION_PAN_DAMP: f32 = 5.0;
/// Inertia below this speed (units/s) is considered stopped.
const BASTION_PAN_STOP: f32 = 1.0;
/// Per-frame safety clamp on grab-drag translation, to keep grazing-angle
/// picks from teleporting the camera.
const BASTION_GRAB_MAX_STEP: f32 = 512.0;

/// Represents an active game session (i.e., the one being played).
impl SessionState {
    /// Create a new `SessionState`.
    pub fn new(
        global_state: &mut GlobalState,
        metadata: UpdateCharacterMetadata,
        client: Rc<RefCell<Client>>,
        persisted_state: Rc<RefCell<PersistedHudState>>,
    ) -> Self {
        // Create a scene for this session. The scene handles visible elements of the
        // game world.
        let mut scene = Scene::new(
            global_state.window.renderer_mut(),
            &mut global_state.lazy_init,
            &client.borrow(),
            &global_state.settings,
        );
        scene
            .camera_mut()
            .set_fov_deg(global_state.settings.graphics.fov);
        client
            .borrow_mut()
            .set_lod_distance(global_state.settings.graphics.lod_distance);
        #[cfg(not(target_os = "macos"))]
        let mut mumble_link = SharedLink::new("veloren", "veloren-voxygen");
        {
            let mut client = client.borrow_mut();
            client.request_player_physics(global_state.settings.networking.player_physics_behavior);
            client.request_lossy_terrain_compression(
                global_state.settings.networking.lossy_terrain_compression,
            );
            #[cfg(not(target_os = "macos"))]
            if let Some(uid) = client.uid() {
                let identiy = if let Some(info) = client.player_list().get(&uid) {
                    format!("{}-{}", info.player_alias, uid)
                } else {
                    format!("unknown-{}", uid)
                };
                mumble_link.set_identity(&identiy);
                // TODO: evaluate context
            }
        }
        let hud = Hud::new(global_state, persisted_state, &client.borrow());
        let walk_forward_dir = scene.camera().forward_xy();
        let walk_right_dir = scene.camera().right_xy();

        Self {
            scene,
            client,
            key_state: KeyState::default(),
            inputs: comp::ControllerInputs::default(),
            inputs_state: HashSet::new(),
            hud,
            selected_block: Block::new(BlockKind::Misc, Rgb::broadcast(255)),
            walk_forward_dir,
            walk_right_dir,
            free_look: false,
            freecam_pos: Vec3::zero(),
            auto_walk: false,
            walking_speed: false,
            camera_clamp: false,
            zoom_lock: false,
            is_aiming: false,
            target_entity: None,
            selected_entity: None,
            viewpoint_entity: None,
            interactables: Default::default(),
            #[cfg(not(target_os = "macos"))]
            mumble_link,
            hitboxes: HashMap::new(),
            metadata,
            tracks: HashMap::new(),
            lines: Default::default(),
            gizmos: Vec::new(),
            bastion_pending_overseer: global_state.args.bastion_overseer,
            bastion_grab: None,
            bastion_pan_vel: Vec2::zero(),
            bastion_orbiting: false,
            bastion_tools: Default::default(),
            bastion_lmb_down: None,
            bastion_rmb_down: None,
            bastion_paint: None,
            bastion_boxsel: None,
            bastion_selected: Vec::new(),
            bastion_colonist_markers: HashMap::new(),
            bastion_selection_rings: HashMap::new(),
            bastion_designation_synced: 0,
            bastion_designation_shapes: Vec::new(),
            bastion_inspect_sent: None,
            bastion_inspect_cell: None,
            bastion_designation_slice: None,
            bastion_visuals: crate::bastion::tools::VisualsMode::default(),
            bastion_designation_dirty: false,
        }
    }

    // bastion: overseer camera mode switching (B1) + input contexts (B1.5)
    fn bastion_overseer_active(&self) -> bool {
        self.scene.camera().get_mode() == CameraMode::Overseer
    }

    /// bastion: the active input context (design doc §3b) is *derived* — a
    /// pure function of the launch flag and camera mode — so it can never
    /// desync from what the player sees. Syncing it into the window is the
    /// atomic whole-scheme swap.
    fn bastion_context(&self, global_state: &GlobalState) -> bastion::input::InputContext {
        if !global_state.args.bastion_overseer {
            bastion::input::InputContext::Menu
        } else if self.bastion_overseer_active() {
            bastion::input::InputContext::Overseer
        } else {
            // Any non-overseer camera while the flag is on is the (stubbed
            // until B12) embodied mode: exactly vanilla controls.
            bastion::input::InputContext::Avatar
        }
    }

    fn bastion_sync_context(&self, global_state: &mut GlobalState) {
        let context = self.bastion_context(global_state);
        global_state.window.set_bastion_context(context);
    }

    fn bastion_enter_overseer(&mut self, global_state: &mut GlobalState) {
        let pos = self.client.borrow().position();
        let camera = self.scene.camera_mut();
        camera.set_mode(CameraMode::Overseer);
        // Snap yaw to the nearest 90° step and set the default oblique pitch.
        let yaw = (camera.get_orientation().x / core::f32::consts::FRAC_PI_2).round()
            * core::f32::consts::FRAC_PI_2;
        camera.set_orientation_instant(Vec3::new(yaw, camera::OVERSEER_PITCH, 0.0));
        if let Some(pos) = pos {
            camera.force_focus_pos(pos);
        }
        // God mode runs with a free, visible cursor (grab-drag needs it).
        global_state.window.grab_cursor(false);
        self.bastion_sync_context(global_state);
    }

    fn bastion_exit_overseer(&mut self, global_state: &mut GlobalState) {
        self.scene.set_bastion_slice_z(None);
        self.bastion_grab = None;
        self.bastion_orbiting = false;
        self.bastion_pan_vel = Vec2::zero();
        // "Avatar = exactly vanilla": what vanilla means depends on presence —
        // third-person for a character body, freefly for a spectator.
        let mode = if self.client.borrow().presence() == Some(PresenceKind::Spectator) {
            CameraMode::Freefly
        } else {
            CameraMode::ThirdPerson
        };
        let camera = self.scene.camera_mut();
        camera.set_mode(mode);
        // Restore the vanilla default boom length and a level pitch.
        camera.set_distance(10.0);
        camera.set_orientation_instant(Vec3::new(camera.get_orientation().x, 0.0, 0.0));
        global_state.window.grab_cursor(true);
        self.bastion_sync_context(global_state);
    }

    /// bastion: world point under the cursor on the horizontal plane
    /// `z = plane_z` (grab-drag / zoom-to-cursor picking).
    fn bastion_point_under_cursor(
        &self,
        global_state: &GlobalState,
        plane_z: f32,
    ) -> Option<Vec3<f32>> {
        let cursor = global_state.window.cursor_position();
        let res = global_state
            .window
            .renderer()
            .resolution()
            .map(|e| e as f32);
        crate::bastion::unproject_to_world_plane(
            self.scene.camera(),
            Vec2::new(cursor.x as f32, cursor.y as f32),
            res,
            plane_z,
        )
    }

    fn bastion_begin_grab(&mut self, global_state: &GlobalState) {
        // Grab on the active slice plane if one is set (that's the layer the
        // player is reading). Otherwise (B5.6b-1 fix): anchor on the TERRAIN
        // HEIGHT UNDER THE CURSOR, not the camera-focus plane — grabbing a
        // hilltop while the focus rides a valley put the anchor plane far
        // below the grabbed surface, so the point visibly slid out from
        // under the cursor ("pan off-center"). Two refinement passes:
        // unproject on the focus plane for an approximate XY, sample the
        // (canopy-safe) surface there, re-unproject on that height — good to
        // within a block even on steep slopes.
        let plane_z = self.scene.bastion_slice_z().unwrap_or_else(|| {
            let mut z = self.scene.camera().get_focus_pos().z;
            let client = self.client.borrow();
            let terrain = client.state().terrain();
            for _ in 0..2 {
                let Some(p) = self.bastion_point_under_cursor(global_state, z) else {
                    break;
                };
                z = crate::bastion::overlay_surface_z(&terrain, p.xy(), z, None);
            }
            z
        });
        if let Some(anchor) = self.bastion_point_under_cursor(global_state, plane_z) {
            self.bastion_grab = Some(BastionGrab { anchor, plane_z });
            self.bastion_pan_vel = Vec2::zero();
        }
    }

    /// bastion (B2a): the picking-plane height for interaction (active slice,
    /// else focus ground height) — same rule as grab-drag.
    fn bastion_plane_z(&self) -> f32 {
        self.scene
            .bastion_slice_z()
            .unwrap_or_else(|| self.scene.camera().get_focus_pos().z)
    }

    /// bastion (B2a): nearest entity under the cursor (ray/cylinder test
    /// against entity positions; radius from body size).
    fn bastion_pick_entity(&self, global_state: &GlobalState) -> Option<specs::Entity> {
        let cursor = global_state.window.cursor_position();
        let res = global_state
            .window
            .renderer()
            .resolution()
            .map(|e| e as f32);
        let (origin, dir) = bastion::cursor_ray(
            self.scene.camera(),
            Vec2::new(cursor.x as f32, cursor.y as f32),
            res,
        )?;
        use specs::Join;
        let dir = dir.normalized();
        let client = self.client.borrow();
        let ecs = client.state().ecs();
        let positions = ecs.read_storage::<comp::Pos>();
        let bodies = ecs.read_storage::<comp::Body>();
        let scales = ecs.read_storage::<comp::Scale>();
        let entities = ecs.entities();
        let mut best: Option<(specs::Entity, f32)> = None;
        for (entity, pos) in (&entities, &positions).join() {
            // Aim at the torso, not the feet.
            let rel = pos.0 + Vec3::unit_z() * 0.8 - origin;
            let t = rel.dot(dir);
            // NOTE: the ray origin (NDC z=1) sits OVERSEER_BEHIND (768)
            // blocks behind the camera plane (B1.7 ortho near extension), so
            // the reachable world starts around t ≈ 768 — the cap must
            // account for that or every entity is silently rejected.
            if !(0.0..=2000.0).contains(&t) {
                continue;
            }
            let radius = bodies.get(entity).map_or(0.6, |b| b.max_radius())
                * scales.get(entity).map_or(1.0, |s| s.0)
                + 0.75;
            if (rel - dir * t).magnitude_squared() < radius * radius
                && best.is_none_or(|(_, bt)| t < bt)
            {
                best = Some((entity, t));
            }
        }
        best.map(|(e, _)| e)
    }

    /// bastion (B2a): the terrain block under the cursor (plane pick refined
    /// to the ground surface under that column).
    fn bastion_cursor_block(&self, global_state: &GlobalState) -> Option<Vec3<i32>> {
        let p = self.bastion_point_under_cursor(global_state, self.bastion_plane_z())?;
        let client = self.client.borrow();
        let terrain = client.state().terrain();
        let gz = bastion::ground_z(&terrain, p.xy(), p.z).unwrap_or(p.z);
        // ground_z is the air cell above the surface; the solid block is below.
        Some(Vec3::new(
            p.x.floor() as i32,
            p.y.floor() as i32,
            (gz - 1.0).floor() as i32,
        ))
    }

    /// bastion (B2a/B3): replace the selection set. Maintains the
    /// `BastionSelected` ECS markers (which also feed the B1.6 cutaway
    /// targets), the HUD info line, and a chat line for durable feedback.
    fn bastion_select_set(&mut self, targets: Vec<specs::Entity>) {
        // UI-5: any explicit selection supersedes a cell-inspect (the caller
        // re-sets it after, for the empty-click path).
        self.bastion_inspect_cell = None;
        let info = {
            let client = self.client.borrow();
            let ecs = client.state().ecs();
            let mut sel = ecs.write_storage::<comp::BastionSelected>();
            for prev in &self.bastion_selected {
                sel.remove(*prev);
            }
            for e in &targets {
                let _ = sel.insert(*e, comp::BastionSelected);
            }
            match targets.len() {
                0 => None,
                1 => {
                    let e = targets[0];
                    // Colonists show their roster name; anything else its uid.
                    let who = ecs
                        .read_storage::<comp::Colonist>()
                        .get(e)
                        .map(|c| c.0.name.clone())
                        .or_else(|| {
                            ecs.read_storage::<common::uid::Uid>()
                                .get(e)
                                .map(|u| format!("entity {u}"))
                        })
                        .unwrap_or_else(|| "?".into());
                    let hp = ecs
                        .read_storage::<comp::Health>()
                        .get(e)
                        .map(|h| format!(" — health {:.0}%", h.fraction() * 100.0))
                        .unwrap_or_default();
                    Some(format!("Selected: {who}{hp}"))
                },
                n => Some(format!("Selected: {n} units")),
            }
        };
        self.bastion_selected = targets;
        if let Some(info) = &info {
            self.hud
                .new_message(ChatType::CommandInfo.into_plain_msg(info.clone()));
        }
        self.hud.bastion_set_selected(info);
    }

    /// bastion (B3): begin/finish the Inspect-tool box-select drag.
    fn bastion_boxsel_begin(&mut self, global_state: &GlobalState) {
        let plane_z = self.bastion_plane_z();
        if let Some(anchor) = self.bastion_point_under_cursor(global_state, plane_z) {
            self.bastion_boxsel = Some(BastionPaint {
                anchor,
                current: anchor,
                plane_z,
                shapes: Vec::new(),
            });
        }
    }

    fn bastion_boxsel_update(&mut self, global_state: &GlobalState) {
        let Some(plane_z) = self.bastion_boxsel.as_ref().map(|p| p.plane_z) else {
            return;
        };
        let Some(current) = self.bastion_point_under_cursor(global_state, plane_z) else {
            return;
        };
        let (old_shapes, anchor) = {
            let sel = self.bastion_boxsel.as_mut().unwrap();
            sel.current = current;
            (std::mem::take(&mut sel.shapes), sel.anchor)
        };
        for id in old_shapes {
            self.scene.debug.remove_shape(id);
        }
        let min = Vec3::partial_min(anchor, current);
        let max = Vec3::partial_max(anchor, current);
        // Live drag preview: coarse stride keeps per-mouse-move rebuild cheap.
        let shapes = self.bastion_region_outline(min, max, [0.3, 0.95, 1.0, 0.9], 2.0);
        if let Some(sel) = self.bastion_boxsel.as_mut() {
            sel.shapes = shapes;
        }
    }

    fn bastion_boxsel_finish(&mut self, global_state: &GlobalState) {
        let Some(sel) = self.bastion_boxsel.take() else {
            return;
        };
        for id in sel.shapes {
            self.scene.debug.remove_shape(id);
        }
        let min: Vec2<f32> = Vec2::partial_min(sel.anchor.xy(), sel.current.xy());
        let max: Vec2<f32> = Vec2::partial_max(sel.anchor.xy(), sel.current.xy());
        // A tiny drag is a click: fall back to single-entity pick.
        if (max - min).magnitude_squared() < 2.0f32.powi(2) {
            // UI-5 (row 62.2): a colonist under the cursor selects it (UI-4);
            // an empty-handed click inspects whatever Bastion object sits in
            // that world column (a job / stockpile / farm / fell-set) — the
            // "click → show" pattern widened past colonists.
            let cell = sel.current.map(|e| e.floor() as i32);
            match self.bastion_pick_entity(global_state) {
                Some(e) => self.bastion_select_set(vec![e]),
                None => {
                    self.bastion_select_set(Vec::new());
                    self.bastion_inspect_cell = Some(cell);
                },
            }
            return;
        }
        let targets: Vec<specs::Entity> = {
            use specs::Join;
            let client = self.client.borrow();
            let ecs = client.state().ecs();
            let colonists = ecs.read_storage::<comp::Colonist>();
            let positions = ecs.read_storage::<comp::Pos>();
            let entities = ecs.entities();
            (&entities, &colonists, &positions)
                .join()
                .filter(|(_, _, pos)| {
                    let p = pos.0.xy();
                    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y
                })
                .map(|(e, _, _)| e)
                .collect()
        };
        self.bastion_select_set(targets);
    }

    /// bastion (B3): keep an overhead marker above every loaded colonist so
    /// the colony reads as *yours* top-down (B9 re-skins this).
    fn bastion_sync_colonist_markers(&mut self) {
        use specs::Join;
        let live: Vec<(specs::Entity, Vec3<f32>)> = {
            let client = self.client.borrow();
            let ecs = client.state().ecs();
            let colonists = ecs.read_storage::<comp::Colonist>();
            let positions = ecs.read_storage::<comp::Pos>();
            let entities = ecs.entities();
            (&entities, &colonists, &positions)
                .join()
                .map(|(e, _, pos)| (e, pos.0))
                .collect()
        };
        let live_set: std::collections::HashSet<specs::Entity> =
            live.iter().map(|(e, _)| *e).collect();
        let stale: Vec<(specs::Entity, DebugShapeId)> = self
            .bastion_colonist_markers
            .iter()
            .filter(|(e, _)| !live_set.contains(e))
            .map(|(e, id)| (*e, *id))
            .collect();
        for (e, id) in stale {
            self.bastion_colonist_markers.remove(&e);
            self.scene.debug.remove_shape(id);
        }
        for (e, pos) in live {
            let id = match self.bastion_colonist_markers.get(&e) {
                Some(id) => *id,
                None => {
                    let id = self
                        .scene
                        .debug
                        .add_shape(crate::scene::DebugShape::Cylinder {
                            radius: 0.35,
                            height: 0.4,
                        });
                    self.bastion_colonist_markers.insert(e, id);
                    id
                },
            };
            self.scene.debug.set_context(
                id,
                [pos.x, pos.y, pos.z + 2.4, 0.0],
                [0.25, 0.95, 1.0, 0.9],
                [0.0, 0.0, 0.0, 1.0],
            );
        }
    }

    /// bastion (UI-4.1): a flat highlight ring under each SELECTED colonist,
    /// tracking it as it moves. Mirrors `bastion_sync_colonist_markers`
    /// exactly (add/reuse/remove a per-entity debug shape), but keyed on the
    /// selection set instead of all colonists, and drawn as a wide, thin
    /// ground disc (a flat `Cylinder`) in a warm gold — distinct from the
    /// cyan overhead marker — so the picked colonist reads at a glance in
    /// the world. Deselecting or a vanished entity drops its ring.
    fn bastion_sync_selection_rings(&mut self) {
        use specs::Join;
        let sel: Vec<(specs::Entity, Vec3<f32>)> = {
            let client = self.client.borrow();
            let ecs = client.state().ecs();
            let positions = ecs.read_storage::<comp::Pos>();
            // Only the selected entities that are still LOADED colonists
            // (a vanished/despawned one is dropped below).
            let colonists = ecs.read_storage::<comp::Colonist>();
            self.bastion_selected
                .iter()
                .filter_map(|&e| {
                    (colonists.contains(e))
                        .then(|| positions.get(e).map(|p| (e, p.0)))
                        .flatten()
                })
                .collect()
        };
        let live_set: std::collections::HashSet<specs::Entity> =
            sel.iter().map(|(e, _)| *e).collect();
        let stale: Vec<(specs::Entity, DebugShapeId)> = self
            .bastion_selection_rings
            .iter()
            .filter(|(e, _)| !live_set.contains(e))
            .map(|(e, id)| (*e, *id))
            .collect();
        for (e, id) in stale {
            self.bastion_selection_rings.remove(&e);
            self.scene.debug.remove_shape(id);
        }
        for (e, pos) in sel {
            let id = match self.bastion_selection_rings.get(&e) {
                Some(id) => *id,
                None => {
                    let id = self
                        .scene
                        .debug
                        .add_shape(crate::scene::DebugShape::Cylinder {
                            radius: 0.7,
                            height: 0.05,
                        });
                    self.bastion_selection_rings.insert(e, id);
                    id
                },
            };
            // At the feet (pos.z), a hair up so it rides the ground without
            // z-fighting. Warm gold, semi-opaque.
            self.scene.debug.set_context(
                id,
                [pos.x, pos.y, pos.z + 0.05, 0.0],
                [1.0, 0.85, 0.3, 0.85],
                [0.0, 0.0, 0.0, 1.0],
            );
        }
    }

    /// bastion (B2a): open the contextual radial menu for whatever is under
    /// the cursor (entity beats block; block kind picks the lead verb).
    fn bastion_open_radial(&mut self, global_state: &GlobalState) {
        use common::bastion::{ContextTarget, ContextVerb, InfluenceKind};

        use crate::hud::bastion::{BastionRadial, RadialAction};
        if let Some(entity) = self.bastion_pick_entity(global_state) {
            let (uid, pos) = {
                let client = self.client.borrow();
                let ecs = client.state().ecs();
                (
                    ecs.read_storage::<common::uid::Uid>().get(entity).copied(),
                    ecs.read_storage::<comp::Pos>().get(entity).map(|p| p.0),
                )
            };
            if let (Some(uid), Some(pos)) = (uid, pos) {
                self.hud.bastion_open_radial(BastionRadial::new(
                    format!("Entity {uid}"),
                    ContextTarget::Entity(uid),
                    pos,
                    vec![
                        RadialAction::Verb(ContextVerb::Inspect),
                        RadialAction::Verb(ContextVerb::SetPolicy),
                        RadialAction::Verb(ContextVerb::Embody),
                        RadialAction::Verb(ContextVerb::ForceAction),
                    ],
                ));
                return;
            }
        }
        if let Some(block) = self.bastion_cursor_block(global_state) {
            let (kind, in_zone) = {
                let client = self.client.borrow();
                let terrain = client.state().terrain();
                (
                    terrain.get(block).ok().copied().map(|b| b.kind()),
                    // B5.5: inside a painted designation rect → offer
                    // whole-zone deletion.
                    client
                        .bastion_designations()
                        .iter()
                        .any(|(r, _, _)| r.contains_point_xy(block)),
                )
            };
            use common::terrain::BlockKind;
            // Lead verb from what's under the cursor; the rest behind it.
            let (title, mut actions) = match kind {
                Some(BlockKind::Wood) | Some(BlockKind::Leaves) => {
                    ("Tree", vec![RadialAction::Verb(ContextVerb::Chop)])
                },
                Some(BlockKind::Rock) | Some(BlockKind::WeakRock) => {
                    ("Rock", vec![RadialAction::Verb(ContextVerb::Mine)])
                },
                _ => ("Ground", vec![]),
            };
            if in_zone {
                // Lead position: deleting the zone you clicked is the most
                // likely intent when a zone is under the cursor.
                actions.insert(0, RadialAction::DeleteZone);
            }
            for a in [
                RadialAction::Verb(ContextVerb::Build),
                RadialAction::Verb(ContextVerb::Stockpile),
                RadialAction::Verb(ContextVerb::Mine),
                RadialAction::Verb(ContextVerb::Chop),
                // B3: colony founding lives on the ground context.
                RadialAction::Verb(ContextVerb::FoundColony),
                RadialAction::Influence(InfluenceKind::Bless),
                RadialAction::Influence(InfluenceKind::Rain),
            ] {
                if !actions.contains(&a) {
                    actions.push(a);
                }
            }
            let point = block.map(|e| e as f32) + Vec3::new(0.5, 0.5, 1.0);
            self.hud.bastion_open_radial(BastionRadial::new(
                title.to_string(),
                ContextTarget::Block(block),
                point,
                actions,
            ));
        }
    }

    /// bastion (B2a → B5.6a): rectangle outline for a region, DRAPED onto the
    /// terrain surface (was 4 flat lines at the region's pick-plane z, which
    /// floats over sloped ground — the photographed bug). Samples the visible
    /// surface per-cell (`bastion::draped_rect_outline`, slice-aware) and
    /// emits conformed line segments. `step` = sample stride (coarser for
    /// live drag previews, per-cell for committed overlays). Returns shape
    /// ids.
    fn bastion_region_outline(
        &mut self,
        min: Vec3<f32>,
        max: Vec3<f32>,
        color: [f32; 4],
        step: f32,
    ) -> Vec<DebugShapeId> {
        // Sample all conformed segments first (immutable client borrow), then
        // emit shapes (mutable scene borrow) — the two borrows must not overlap.
        let slice_z = self.scene.bastion_slice_z();
        let segs = {
            let client = self.client.borrow();
            let terrain = client.state().terrain();
            crate::bastion::draped_rect_outline(
                &terrain,
                min.xy(),
                max.xy(),
                // Hint the ground search with whichever region face is higher
                // (paint plane / region top) — ground_z converges regardless.
                min.z.max(max.z),
                slice_z,
                0.2,
                step,
            )
        };
        segs.into_iter()
            .map(|seg| {
                let id = self
                    .scene
                    .debug
                    .add_shape(crate::scene::DebugShape::Line(seg, 0.15));
                self.scene
                    .debug
                    .set_context(id, [0.0; 4], color, [0.0, 0.0, 0.0, 1.0]);
                id
            })
            .collect()
    }

    /// bastion (B2a): begin/refresh/finish the designate-paint drag.
    fn bastion_paint_begin(&mut self, global_state: &GlobalState) {
        let plane_z = self.bastion_plane_z();
        if let Some(anchor) = self.bastion_point_under_cursor(global_state, plane_z) {
            self.bastion_paint = Some(BastionPaint {
                anchor,
                current: anchor,
                plane_z,
                shapes: Vec::new(),
            });
        }
    }

    fn bastion_paint_update(&mut self, global_state: &GlobalState) {
        let Some(plane_z) = self.bastion_paint.as_ref().map(|p| p.plane_z) else {
            return;
        };
        let Some(current) = self.bastion_point_under_cursor(global_state, plane_z) else {
            return;
        };
        let (old_shapes, anchor) = {
            let paint = self.bastion_paint.as_mut().unwrap();
            paint.current = current;
            (std::mem::take(&mut paint.shapes), paint.anchor)
        };
        for id in old_shapes {
            self.scene.debug.remove_shape(id);
        }
        let min = Vec3::partial_min(anchor, current);
        let max = Vec3::partial_max(anchor, current);
        // B5.5: the erase brush previews red; placement previews yellow.
        let designate = matches!(
            self.bastion_tools.tool,
            crate::bastion::tools::ToolMode::Designate(_)
        );
        let color = if self.bastion_tools.tool == crate::bastion::tools::ToolMode::Erase {
            [1.0, 0.25, 0.2, 0.9]
        } else {
            [1.0, 1.0, 0.3, 0.9]
        };
        let mut shapes = self.bastion_region_outline(min, max, color, 2.0);
        // B5.6b-2: the drag half of the volume-selection UX — depth rings
        // below (and above) the draped surface outline, one per selected
        // level, bottom/top ring emphasized so the extent is countable while
        // you scroll. Shifted copies of the SAME conformed segments: at
        // paint time the surface sample is fresh, so shifted rings are the
        // per-column surface-relative volume the server will resolve.
        let extent = self.bastion_tools.z_extent;
        if designate && (extent.down > 0 || extent.up > 0) {
            let slice_z = self.scene.bastion_slice_z();
            let segs = {
                let client = self.client.borrow();
                let terrain = client.state().terrain();
                crate::bastion::draped_rect_outline(
                    &terrain,
                    min.xy(),
                    max.xy(),
                    min.z.max(max.z),
                    slice_z,
                    0.2,
                    2.0,
                )
            };
            let mut ring = |offset: f32, alpha: f32| {
                for seg in &segs {
                    let id = self.scene.debug.add_shape(crate::scene::DebugShape::Line(
                        [
                            seg[0] + Vec3::unit_z() * offset,
                            seg[1] + Vec3::unit_z() * offset,
                        ],
                        0.12,
                    ));
                    self.scene.debug.set_context(
                        id,
                        [0.0; 4],
                        [color[0], color[1], color[2], alpha],
                        [0.0, 0.0, 0.0, 1.0],
                    );
                    shapes.push(id);
                }
            };
            for lvl in 1..=extent.down {
                let a = if lvl == extent.down { 0.85 } else { 0.3 };
                ring(-(lvl as f32), a);
            }
            for lvl in 1..=extent.up {
                let a = if lvl == extent.up { 0.85 } else { 0.3 };
                ring(lvl as f32, a);
            }
        }
        if let Some(paint) = self.bastion_paint.as_mut() {
            paint.shapes = shapes;
        }
        // Live "N levels" counter at the drag cursor (designate only).
        self.hud.bastion_set_paint_label(designate.then(|| {
            (
                current + Vec3::unit_z() * 2.0,
                self.bastion_tools.z_extent_label(),
                color,
            )
        }));
    }

    /// bastion (B5.6b-2): switch the pinned tool, resetting the depth
    /// selection to the kind default when the DESIGNATE KIND changes (a
    /// custom mine depth shouldn't silently carry into stockpile painting).
    fn bastion_set_tool(&mut self, tool: crate::bastion::tools::ToolMode) {
        use crate::bastion::tools::ToolMode;
        let old_kind = match self.bastion_tools.tool {
            ToolMode::Designate(k) => Some(k),
            _ => None,
        };
        if let ToolMode::Designate(k) = tool
            && old_kind != Some(k)
        {
            self.bastion_tools.z_extent = common::bastion::ZExtent::default_for(k);
        }
        self.bastion_tools.tool = tool;
    }

    /// bastion (TIME-CONTROLS): the ONE sim-speed setter — the HUD buttons
    /// and the hotkeys both land here. `None` pauses (the singleplayer pause
    /// halts the server loop — the world visibly freezes); `Some(scale)`
    /// unpauses and sets the server `TimeScale` via the admin chat command
    /// (singleplayer grants Admin; `TimeScale` multiplies the ENTIRE sim's
    /// DeltaTime, so 2× runs everything — physics, agents, jobs — at 2×).
    /// Pause and scale are independent: pausing does NOT touch the scale, so
    /// resume returns to the pre-pause speed.
    fn bastion_set_sim_speed(&mut self, global_state: &GlobalState, speed: Option<f32>) {
        match speed {
            None => {
                #[cfg(feature = "singleplayer")]
                global_state.pause();
                #[cfg(not(feature = "singleplayer"))]
                let _ = global_state;
            },
            Some(s) => {
                #[cfg(feature = "singleplayer")]
                global_state.unpause();
                #[cfg(not(feature = "singleplayer"))]
                let _ = global_state;
                let current = self
                    .client
                    .borrow()
                    .state()
                    .ecs()
                    .read_resource::<common::resources::TimeScale>()
                    .0 as f32;
                if (current - s).abs() > 0.001 {
                    self.client
                        .borrow_mut()
                        .send_command("time_scale".into(), vec![format!("{s}")]);
                }
            },
        }
    }

    /// bastion (TIME-CONTROLS): step the speed ladder (⏸ ← 1× ↔ 2× ↔ 4×).
    /// Stepping DOWN from 1× pauses; stepping UP while paused resumes at 1×.
    fn bastion_step_sim_speed(&mut self, global_state: &GlobalState, up: bool) {
        const LADDER: [f32; 3] = [1.0, 2.0, 4.0];
        let paused = global_state.paused();
        let next = if paused {
            if up { Some(LADDER[0]) } else { None }
        } else {
            let current = self
                .client
                .borrow()
                .state()
                .ecs()
                .read_resource::<common::resources::TimeScale>()
                .0 as f32;
            // Nearest rung (a chat-set ×3 steps to ×4 or ×2 sensibly).
            let i = LADDER
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (*a - current)
                        .abs()
                        .partial_cmp(&(*b - current).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            if up {
                Some(LADDER[(i + 1).min(LADDER.len() - 1)])
            } else if i == 0 {
                None // below 1× = pause
            } else {
                Some(LADDER[i - 1])
            }
        };
        self.bastion_set_sim_speed(global_state, next);
    }

    /// Finish the paint drag: `Some(kind)` places a designation, `None`
    /// (B5.5, the erase tool) cancels designations in the region.
    fn bastion_paint_finish(&mut self, kind: Option<common::bastion::DesignationKind>) {
        let Some(paint) = self.bastion_paint.take() else {
            return;
        };
        for id in paint.shapes {
            self.scene.debug.remove_shape(id);
        }
        // B5.6b-2: the live level-counter label ends with the drag.
        self.hud.bastion_set_paint_label(None);
        // B5.6b-2: placement is SURFACE-RELATIVE — the drag defines only the
        // XY footprint (+ the paint plane as a z hint); the server resolves
        // each column against its own terrain surface and applies the
        // selected z_extent (default = the old plane-2..=plane depth). This
        // replaces the old flat `min.z - 2` expansion, which cut zones off
        // on slopes (B5.MINE-COVERAGE). Erase is unaffected: it matches by
        // XY footprint below.
        let min_f: Vec3<f32> = Vec3::partial_min(paint.anchor, paint.current);
        let max_f: Vec3<f32> = Vec3::partial_max(paint.anchor, paint.current);
        let region = common::bastion::Region {
            min: min_f.map(|e| e.floor() as i32),
            max: max_f.map(|e| e.floor() as i32),
        }
        .normalized();
        match kind {
            Some(kind) => {
                // B5.6b-2.1: flat-floor mode derives the shared absolute
                // floor from the TERRAIN SURFACE under the drag minus the
                // stepper depth — every column bottoms out at one level
                // (flat, square). Ben's live find: deriving from the PICK
                // PLANE (region.max.z) put the floor ABOVE every column's
                // surface whenever the camera plane sat above the ground
                // (any angled drag on slopes) → the server resolved zero
                // columns → "no terrain surface under the footprint"
                // false-reject at perfectly valid volumes. Sample the real
                // surface at the drag rect's center (the one height
                // authority, slice-aware); the plane stays a fallback.
                // CHOP redesign (FR10): an Area2D kind paints a PURE XY
                // footprint — no volume, no extent on the wire. The server
                // resolves whole trees rooted in it and echoes per-tree boxes.
                if kind.footprint_mode() == common::bastion::FootprintMode::Area2D {
                    self.client
                        .borrow_mut()
                        .bastion_place_designation(region, kind, None);
                } else {
                    let mut extent = self.bastion_tools.z_extent;
                    if self.bastion_tools.flat_floor {
                        let center = Vec2::new(
                            (region.min.x + region.max.x) as f32 / 2.0 + 0.5,
                            (region.min.y + region.max.y) as f32 / 2.0 + 0.5,
                        );
                        let surface = bastion::overlay_surface_z(
                            &self.client.borrow().state().terrain(),
                            center,
                            region.max.z as f32,
                            self.scene.bastion_slice_z(),
                        )
                        .floor() as i32;
                        extent.floor_z = Some(surface - extent.down as i32);
                    }
                    self.client
                        .borrow_mut()
                        .bastion_place_designation(region, kind, Some(extent));
                }
            },
            None => {
                // B5.6a erase fix: the drag's z came from the camera
                // pick-plane, which need not align with where a designation
                // was painted — a z-misaligned cancel silently missed,
                // leaving the overlay AND the jobs behind. Instead, match
                // designations by XY footprint and cancel the XY-intersection
                // at each rect's OWN z (can't miss in z; partial-erase leaves
                // the un-brushed remainder). Empty brush over bare ground →
                // nothing cancelled.
                let targets: Vec<common::bastion::Region> = {
                    let client = self.client.borrow();
                    client
                        .bastion_designations()
                        .iter()
                        .filter_map(|(r, _, _)| r.clip_xy(region.min.xy(), region.max.xy()))
                        .collect()
                };
                let mut client = self.client.borrow_mut();
                for t in targets {
                    client.bastion_cancel_designation(t);
                }
            },
        }
    }

    /// bastion (UI-4, row 62): the unit-inspector pump — when exactly one
    /// colonist is selected, request its payload from the server
    /// (immediately on target change, ~1Hz while held) and mirror the
    /// latest matching reply into the HUD's plain-text block; anything
    /// else (no selection, multi-select, non-colonist reply) clears it.
    /// READ-ONLY end to end — the panel writes no sim state.
    fn bastion_sync_inspector(&mut self) {
        use common::comp::bastion::{BastionInspectKind as Kind, BastionInspectTarget as Tgt};
        // Prefer a single selected colonist (the UI-4 path); else a clicked
        // world cell (UI-5 — a job / stockpile / farm / fell-set). Multi-select,
        // or nothing at all, clears the panel.
        let target = if self.bastion_selected.len() == 1 {
            let client = self.client.borrow();
            client
                .state()
                .ecs()
                .read_storage::<common::uid::Uid>()
                .get(self.bastion_selected[0])
                .copied()
                .map(Tgt::Entity)
        } else if self.bastion_selected.is_empty() {
            self.bastion_inspect_cell.map(Tgt::Cell)
        } else {
            None
        };
        let Some(target) = target else {
            self.bastion_inspect_sent = None;
            self.hud.bastion_set_inspect(Vec::new());
            return;
        };
        let stale = self
            .bastion_inspect_sent
            .is_none_or(|(t, at)| t != target || at.elapsed().as_secs_f32() > 1.0);
        if stale {
            self.client.borrow_mut().bastion_inspect_request(target);
            self.bastion_inspect_sent = Some((target, std::time::Instant::now()));
        }
        let lines = {
            let client = self.client.borrow();
            match client.bastion_inspect() {
                Some((t, Some(kind))) if *t == target => match kind {
                    Kind::Colonist(p) => {
                        let mut traits: Vec<&str> = Vec::new();
                        if p.personality4.0 {
                            traits.push("Adventurous");
                        }
                        if p.personality4.1 {
                            traits.push("Worried");
                        }
                        if p.personality4.2 {
                            traits.push("Sociable");
                        }
                        if p.personality4.3 {
                            traits.push("Introverted");
                        }
                        if p.conscientious {
                            traits.push("Conscientious");
                        }
                        if p.neurotic {
                            traits.push("Neurotic");
                        }
                        let mut lines = vec![
                            format!("- {} -", p.name),
                            format!(
                                "Drive: {:?}  (W {:.2} / F {:.2} / I {:.2})",
                                p.drive, p.last_scores.0, p.last_scores.1, p.last_scores.2
                            ),
                            // CHOP-PROGRESS-INDICATOR (row 51.61): the current
                            // work job + its % — a base-cut (or any work) reads
                            // as PROGRESSING here before it completes.
                            match p.activity {
                                Some((wt, frac)) => {
                                    format!("Doing: {:?} {:.0}%", wt, frac * 100.0)
                                },
                                None => "Doing: (idle)".to_string(),
                            },
                            // STATUS-SURFACE: energy joins the meters — it
                            // gates climbing now.
                            format!(
                                "Hunger {:.2}  Rest {:.2}  Rec {:.2}  Energy {:.2}",
                                p.hunger, p.rest, p.recreation, p.energy
                            ),
                            format!("Mood {:.2}", p.mood),
                            if traits.is_empty() {
                                "Traits: (none)".to_string()
                            } else {
                                format!("Traits: {}", traits.join(", "))
                            },
                        ];
                        // STATUS-SURFACE: the designed-wait/rescue status,
                        // right under the name — "sits there, looks broken"
                        // now reads as its actual state; NO line means
                        // nothing designed is holding them.
                        if let Some(status) = p.status {
                            use common::comp::bastion::BastionColonistStatus as S;
                            lines.insert(1, match status {
                                S::RestingToClimb => format!(
                                    "Status: Resting to climb (energy {:.0}%)",
                                    p.energy * 100.0
                                ),
                                S::WaitingForLadder => {
                                    "Status: Waiting for ladder (queued)".to_string()
                                },
                                S::RescueImminent => {
                                    "Status: Rescue imminent".to_string()
                                },
                                S::Replanning => {
                                    "Status: Replanning route".to_string()
                                },
                            });
                        }
                        lines
                    },
                    Kind::Job(j) => vec![
                        format!("- Job: {:?} -", j.work),
                        format!("Progress {:.0}%", j.progress * 100.0),
                        match &j.claimant {
                            Some(name) => format!("Worker: {name}"),
                            None => "Worker: (unclaimed)".to_string(),
                        },
                        format!("at {}, {}, {}", j.pos.x, j.pos.y, j.pos.z),
                        {
                            let mut flags: Vec<&str> = Vec::new();
                            if j.unreachable {
                                flags.push("unreachable");
                            }
                            if j.needs_materials {
                                flags.push("needs materials");
                            }
                            if j.is_access {
                                flags.push("access");
                            }
                            if j.stuck_strikes > 0 {
                                flags.push("stuck");
                            }
                            if flags.is_empty() {
                                "Status: ok".to_string()
                            } else {
                                format!("Status: {}", flags.join(", "))
                            }
                        },
                    ],
                    Kind::Stockpile(s) => {
                        let mut lines = vec![
                            "- Stockpile -".to_string(),
                            format!("{} item(s) total", s.total),
                        ];
                        if s.contents.is_empty() {
                            lines.push("(empty — waiting for hauls)".to_string());
                        } else {
                            for (def, n) in s.contents.iter().take(6) {
                                let short = def.rsplit('.').next().unwrap_or(def);
                                lines.push(format!("  {short} x{n}"));
                            }
                        }
                        lines
                    },
                    Kind::Farm(f) => vec![
                        "- Farm plot -".to_string(),
                        format!("{} cell(s)", f.cells),
                        match f.growth {
                            Some(g) => format!("Crop growth: {g}"),
                            None => "Untilled / no crop here".to_string(),
                        },
                    ],
                    Kind::FellSet(fs) => vec![
                        "- Tree (marked to fell) -".to_string(),
                        format!("{} / {} cells standing", fs.remaining, fs.total),
                    ],
                },
                // Reply pending, stale, or an empty target (payload: None) —
                // show nothing, never crash.
                _ => Vec::new(),
            }
        };
        self.hud.bastion_set_inspect(lines);
    }

    /// bastion (B2a/B5.5): keep the designation overlay in sync with the
    /// client's rect list. Rebuild-on-revision (not incremental): erase can
    /// remove or SPLIT stored rects, which an append-only index can't
    /// express — and the list is dozens of rects at most, so a full rebuild
    /// on change is trivially cheap.
    fn bastion_sync_designations(&mut self) {
        // Rebuild triggers (B5.5 rev; B5.6a slice + visuals mode): the draped
        // overlay depends on the slice plane and the visuals setting, so a
        // slice toggle or a visuals-mode change must rebuild even when the
        // designation list itself (rev) is unchanged.
        let rev = self.client.borrow().bastion_designations_rev();
        let slice_z = self.scene.bastion_slice_z();
        // B5.6a: OFF hides the COMMITTED overlays. The live paint/erase drag
        // still shows its own preview rectangle (separate shapes, always
        // drawn — see bastion_paint_update), so "you can always see what
        // you're painting" holds without an auto-reveal that forced overlays
        // back On whenever a designate tool was merely selected (the bug that
        // made H look like a no-op: after painting, the tool stays Mine, so
        // Off never took effect).
        let visuals = self.bastion_visuals;
        let up_to_date = rev == self.bastion_designation_synced
            && slice_z == self.bastion_designation_slice
            && !self.bastion_designation_dirty;
        if up_to_date {
            return;
        }
        self.bastion_designation_synced = rev;
        self.bastion_designation_slice = slice_z;
        self.bastion_designation_dirty = false;

        for id in std::mem::take(&mut self.bastion_designation_shapes) {
            self.scene.debug.remove_shape(id);
        }
        // OFF: overlays hidden entirely (designations stay fully active — this
        // is visual-only). Nothing to (re)build; drop any labels too.
        if visuals.is_off() {
            self.hud.bastion_set_zone_labels(Vec::new());
            return;
        }
        use crate::bastion::tools::{VisualsMode, zone_border_color, zone_fill_color};
        // B5.6b-1: ON = fill + border + label; SUBTLE = border only (dimmed).
        let subtle = visuals == VisualsMode::Subtle;
        let alpha = visuals.line_alpha();
        let list = self.client.borrow().bastion_designations().to_vec();
        let mut kind_counts: HashMap<common::bastion::DesignationKind, u32> = HashMap::new();
        let mut labels: Vec<(Vec3<f32>, String, [f32; 4])> = Vec::new();
        for (region, kind, _extent) in list {
            // Border — always (both ON and SUBTLE), draped, kind-coloured.
            let border = self.bastion_region_outline(
                region.min.map(|e| e as f32),
                region.max.map(|e| e as f32 + 1.0),
                zone_border_color(kind, alpha),
                1.0,
            );
            self.bastion_designation_shapes.extend(border);
            // Per-kind running index for the label ("Mine 1", "Mine 2", …).
            let idx = {
                let c = kind_counts.entry(kind).or_insert(0);
                *c += 1;
                *c
            };
            if subtle {
                continue; // SUBTLE = border only: no fill, no label.
            }
            // B5.6b-2: VOLUMETRIC rendering (ON only) — zones spanning more
            // than one level draw countable depth rings: one flat ring per
            // level boundary at the region's ABSOLUTE z-levels (the echoed
            // bounds are the exact resolved volume bounds — box semantics,
            // same as cancel/erase), plus 4 corner posts as subtle walls.
            // Absolute (not surface-shifted) so the rings stay put as the
            // dig progresses — mid-excavation the remaining volume reads
            // from inside the pit. Depth-tested like all debug shapes:
            // underground rings appear as terrain is dug or sliced away.
            let levels = region.max.z - region.min.z + 1;
            if levels > 1 {
                let [r, g, b] = crate::bastion::tools::zone_rgb(kind);
                let (min_f, max_f) = (
                    region.min.map(|e| e as f32),
                    region.max.map(|e| e as f32 + 1.0),
                );
                let corners = [
                    Vec2::new(min_f.x, min_f.y),
                    Vec2::new(max_f.x, min_f.y),
                    Vec2::new(max_f.x, max_f.y),
                    Vec2::new(min_f.x, max_f.y),
                ];
                let mut lines: Vec<([Vec3<f32>; 2], f32)> = Vec::new();
                // Rings at each level bottom (the top face is the draped
                // border above); the floor ring reads strongest. Slice-clip:
                // skip rings above the slice plane (their terrain is hidden).
                for z in region.min.z..=region.max.z {
                    let zf = z as f32;
                    if slice_z.is_some_and(|s| zf > s) {
                        continue;
                    }
                    let a = if z == region.min.z { 0.8 } else { 0.3 };
                    for i in 0..4 {
                        let (c0, c1) = (corners[i], corners[(i + 1) % 4]);
                        lines.push(([Vec3::new(c0.x, c0.y, zf), Vec3::new(c1.x, c1.y, zf)], a));
                    }
                }
                // Corner posts: floor to top face (subtle walls, v1),
                // clamped to the slice; slice below the floor = no posts.
                let top = slice_z.map_or(max_f.z, |s| max_f.z.min(s + 1.0));
                if top > min_f.z {
                    for c in corners {
                        lines.push((
                            [Vec3::new(c.x, c.y, min_f.z), Vec3::new(c.x, c.y, top)],
                            0.35,
                        ));
                    }
                }
                for (seg, a) in lines {
                    let id = self
                        .scene
                        .debug
                        .add_shape(crate::scene::DebugShape::Line(seg, 0.1));
                    self.scene
                        .debug
                        .set_context(id, [0.0; 4], [r, g, b, a * alpha], [0.0, 0.0, 0.0, 1.0]);
                    self.bastion_designation_shapes.push(id);
                }
            }
            // Fill — a terrain-conformed translucent area (ON only). Sample
            // (immutable terrain borrow) then emit one ConformedTris shape.
            let tris = {
                let client = self.client.borrow();
                let terrain = client.state().terrain();
                crate::bastion::draped_fill_tris(
                    &terrain,
                    region.min.xy(),
                    region.max.xy(),
                    region.max.z as f32,
                    slice_z,
                    0.1, // just under the outline's 0.2 hover
                )
            };
            if !tris.is_empty() {
                let id = self
                    .scene
                    .debug
                    .add_shape(crate::scene::DebugShape::ConformedTris(tris));
                self.scene
                    .debug
                    .set_context(id, [0.0; 4], zone_fill_color(kind), [0.0, 0.0, 0.0, 1.0]);
                self.bastion_designation_shapes.push(id);
            }
            // Label at the footprint centroid, on the surface (ON only).
            let cx = (region.min.x + region.max.x) as f32 / 2.0;
            let cy = (region.min.y + region.max.y) as f32 / 2.0;
            let cz = {
                let client = self.client.borrow();
                let terrain = client.state().terrain();
                crate::bastion::overlay_surface_z(
                    &terrain,
                    Vec2::new(cx, cy),
                    region.max.z as f32,
                    slice_z,
                )
            };
            // B5.6b-2: volumetric zones state their depth on the label.
            let text = if levels > 1 {
                format!("{} {} · {} levels", kind.label(), idx, levels)
            } else {
                format!("{} {}", kind.label(), idx)
            };
            labels.push((
                Vec3::new(cx, cy, cz + 1.2),
                text,
                zone_border_color(kind, 1.0),
            ));
        }
        self.hud.bastion_set_zone_labels(labels);
    }

    /// bastion: scroll zoom, eased, dollying toward the point under the
    /// cursor (B&W2 style) instead of the screen center.
    fn bastion_zoom_to_cursor(&mut self, global_state: &GlobalState, delta: f32) {
        let old_dist = self.scene.camera().get_tgt_dist();
        let picked =
            self.bastion_point_under_cursor(global_state, self.scene.camera().get_tgt_focus().z);
        let camera = self.scene.camera_mut();
        // Multiplicative dolly, clamped inside zoom_by by the overseer zoom
        // limits. NOTE the wheel arrives pre-scaled ~±15 per notch (see the
        // X11-parity factor in window.rs), so 0.01·dist ≈ ±15% per notch —
        // ~14 eased notches across the min→max range. (Was 0.02/±30%: QA said
        // "way too fast".)
        camera.zoom_by(delta * old_dist * 0.01, None);
        let f = camera.get_tgt_dist() / old_dist;
        if let Some(p) = picked
            && (f - 1.0).abs() > f32::EPSILON
        {
            // Keep the picked point stationary on screen: shrink/grow the
            // focus→point offset by the zoom factor. Both dist and focus
            // interpolate, so the motion arrives eased.
            let tgt = camera.get_tgt_focus();
            camera.set_focus_pos(p + (tgt - p) * f);
        }
    }

    fn stop_auto_walk(&mut self) {
        self.auto_walk = false;
        self.hud.auto_walk(false);
        self.key_state.auto_walk = false;
    }

    /// Possibly lock the camera zoom depending on the current behaviour, and
    /// the current inputs if in the Auto state.
    fn maybe_auto_zoom_lock(
        &mut self,
        zoom_lock_enabled: bool,
        zoom_lock_behavior: AutoPressBehavior,
    ) {
        if let AutoPressBehavior::Auto = zoom_lock_behavior {
            // to add Analog detection, update the condition rhs with a check for
            // MovementX/Y event from the last tick
            self.zoom_lock = zoom_lock_enabled && self.should_auto_zoom_lock();
        } else {
            // it's intentional that the HUD notification is not shown in this case:
            // refresh session from Settings HUD checkbox change
            self.zoom_lock = zoom_lock_enabled;
        }
    }

    /// Gets the entity that is the current viewpoint, and a bool if the client
    /// is allowed to edit it's data.
    fn viewpoint_entity(&self) -> (specs::Entity, bool) {
        self.viewpoint_entity
            .map(|e| (e, false))
            .unwrap_or_else(|| (self.client.borrow().entity(), true))
    }

    fn controlling_char(&self) -> bool {
        self.viewpoint_entity.is_none()
            && self
                .client
                .borrow()
                .presence()
                .is_some_and(|p| p.controlling_char())
    }

    /// Tick the session (and the client attached to it).
    fn tick(
        &mut self,
        dt: Duration,
        global_state: &mut GlobalState,
        outcomes: &mut Vec<Outcome>,
    ) -> Result<TickAction, Error> {
        span!(_guard, "tick", "Session::tick");

        let mut client = self.client.borrow_mut();
        self.scene.maintain_debug_hitboxes(
            &client,
            &global_state.settings,
            &mut self.hitboxes,
            &mut self.tracks,
            &mut self.gizmos,
        );
        self.scene.maintain_debug_vectors(&client, &mut self.lines);
        let pos = client.position().unwrap_or_default();

        #[cfg(not(target_os = "macos"))]
        {
            // Update mumble positional audio
            let ori = client
                .state()
                .read_storage::<comp::Ori>()
                .get(client.entity())
                .map_or_else(comp::Ori::default, |o| *o);
            let front = ori.look_dir().to_vec();
            let top = ori.up().to_vec();
            // converting from veloren z = height axis, to mumble y = height axis
            let player_pos = mumble_link::Position {
                position: [pos.x, pos.z, pos.y],
                front: [front.x, front.z, front.y],
                top: [top.x, top.z, top.y],
            };
            self.mumble_link.update(player_pos, player_pos);
        }

        for event in client.tick(self.inputs.clone(), dt)? {
            match event {
                client::Event::Chat(m) => {
                    self.hud.new_message(m);
                },
                client::Event::GroupInventoryUpdate(item, uid) => {
                    self.hud.new_loot_message(LootMessage {
                        amount: item.amount(),
                        item,
                        taken_by: uid,
                    });
                },
                client::Event::InviteComplete {
                    target,
                    answer,
                    kind,
                } => {
                    let target_name = match client.player_list().get(&target) {
                        Some(info) => info.player_alias.clone(),
                        None => match client.state().ecs().entity_from_uid(target) {
                            Some(entity) => {
                                let stats = client.state().read_storage::<Stats>();
                                stats
                                    .get(entity)
                                    .map_or(format!("<entity {}>", target), |e| {
                                        global_state.i18n.read().get_content(&e.name)
                                    })
                            },
                            None => format!("<uid {}>", target),
                        },
                    };

                    let msg_key = match (kind, answer) {
                        (InviteKind::Group, InviteAnswer::Accepted) => "hud-group-invite-accepted",
                        (InviteKind::Group, InviteAnswer::Declined) => "hud-group-invite-declined",
                        (InviteKind::Group, InviteAnswer::TimedOut) => "hud-group-invite-timed_out",
                        (InviteKind::Trade, InviteAnswer::Accepted) => "hud-trade-invite-accepted",
                        (InviteKind::Trade, InviteAnswer::Declined) => "hud-trade-invite-declined",
                        (InviteKind::Trade, InviteAnswer::TimedOut) => "hud-trade-invite-timed_out",
                    };

                    let msg = global_state
                        .i18n
                        .read()
                        .get_msg_ctx(msg_key, &i18n::fluent_args! { "target" => target_name });

                    self.hud.new_message(ChatType::Meta.into_plain_msg(msg));
                },
                client::Event::TradeComplete { result, trade: _ } => {
                    self.hud.clear_cursor();
                    self.hud
                        .new_message(ChatType::Meta.into_msg(Content::localized(match result {
                            TradeResult::Completed => "hud-trade-result-completed",
                            TradeResult::Declined => "hud-trade-result-declined",
                            TradeResult::NotEnoughSpace => "hud-trade-result-nospace",
                        })));
                },
                client::Event::InventoryUpdated(inv_events) => {
                    let sfx_triggers = self.scene.sfx_mgr.triggers.read();

                    for inv_event in inv_events {
                        let sfx_trigger_item =
                            sfx_triggers.0.get_key_value(&SfxEvent::from(&inv_event));

                        global_state.audio.emit_ui_sfx(sfx_trigger_item, None, None);

                        match inv_event {
                            InventoryUpdateEvent::BlockCollectFailed { pos, reason } => {
                                self.hud.add_failed_block_pickup(
                                    // TODO: Possibly support volumes.
                                    VolumePos::terrain(pos),
                                    HudCollectFailedReason::from_server_reason(
                                        &reason,
                                        client.state().ecs(),
                                    ),
                                );
                            },
                            InventoryUpdateEvent::EntityCollectFailed {
                                entity: uid,
                                reason,
                            } => {
                                if let Some(entity) = client.state().ecs().entity_from_uid(uid) {
                                    self.hud.add_failed_entity_pickup(
                                        entity,
                                        HudCollectFailedReason::from_server_reason(
                                            &reason,
                                            client.state().ecs(),
                                        ),
                                    );
                                }
                            },
                            InventoryUpdateEvent::Collected(item) => {
                                global_state.profile.tutorial.event_collect();
                                self.hud.new_loot_message(LootMessage {
                                    amount: item.amount(),
                                    item,
                                    taken_by: client.uid().expect("Client doesn't have a Uid!!!"),
                                });
                            },
                            _ => {},
                        };
                    }
                },
                client::Event::Dialogue(sender_uid, dialogue) => {
                    if let Some(sender) = client.state().ecs().entity_from_uid(sender_uid) {
                        self.hud.dialogue(sender, pos, dialogue, global_state);
                    }
                },
                client::Event::Disconnect => return Ok(TickAction::Disconnect),
                client::Event::DisconnectionNotification(time) => {
                    self.hud
                        .new_message(ChatType::CommandError.into_msg(match time {
                            0 => Content::localized("hud-chat-goodbye"),
                            _ => Content::localized_with_args("hud-chat-connection_lost", [(
                                "time", time,
                            )]),
                        }));
                },
                client::Event::Notification(n) => {
                    global_state.profile.tutorial.event_notification(&n);
                    self.hud.new_notification(n);
                },
                client::Event::SetViewDistance(_vd) => {},
                client::Event::Outcome(outcome) => {
                    global_state
                        .profile
                        .tutorial
                        .event_outcome(&client, &outcome);
                    outcomes.push(outcome);
                },
                client::Event::CharacterCreated(_) => {},
                client::Event::CharacterEdited(_) => {},
                client::Event::CharacterError(_) => {},
                client::Event::CharacterJoined(_) => {
                    self.scene.music_mgr.reset_track(&mut global_state.audio);
                },
                client::Event::MapMarker(event) => {
                    self.hud
                        .persisted_state
                        .borrow_mut()
                        .location_markers
                        .update(event);
                },
                client::Event::StartSpectate(spawn_point) => {
                    let server_name = &client.server_info().name;
                    let spawn_point = global_state
                        .profile
                        .get_spectate_position(server_name)
                        .unwrap_or(spawn_point);

                    client
                        .state()
                        .ecs()
                        .write_storage()
                        .insert(client.entity(), Pos(spawn_point))
                        .expect("This shouldn't exist");

                    self.scene.camera_mut().force_focus_pos(spawn_point);
                },
                client::Event::SpectatePosition(pos) => {
                    self.scene.camera_mut().force_focus_pos(pos);
                },
                client::Event::PluginDataReceived(data) => {
                    tracing::warn!("Received plugin data at wrong time {}", data.len());
                },
                client::Event::Gizmos(gizmos) => {
                    self.gizmos.retain(|gizmos| {
                        let keep = gizmos.2;
                        if !keep {
                            self.scene.debug.remove_shape(gizmos.0);
                        }
                        keep
                    });
                    for gizmos in gizmos {
                        let mut add_shape = |shape, pos: Vec3<f32>| {
                            let id = self.scene.debug.add_shape(shape);
                            self.scene.debug.set_context(
                                id,
                                pos.with_w(0.0).into_array(),
                                gizmos.color.map(|c| c as f32 / 255.0).into_array(),
                                [0.0, 0.0, 0.0, 1.0],
                            );
                            self.gizmos.push((
                                id,
                                gizmos.end_time.unwrap_or(common::resources::Time(
                                    client.state().get_time() + 1.0,
                                )),
                                gizmos.end_time.is_some(),
                            ));
                        };
                        match gizmos.shape {
                            comp::gizmos::Shape::Sphere(sphere) => {
                                add_shape(
                                    crate::scene::DebugShape::CapsulePrism {
                                        p0: Vec2::zero(),
                                        p1: Vec2::zero(),
                                        radius: sphere.radius,
                                        // Well, we need to put something in
                                        // there...
                                        head_ratio: 1.0,
                                        height: sphere.radius * 2.0,
                                    },
                                    sphere.center,
                                );
                            },
                            comp::gizmos::Shape::LineStrip(lines) => {
                                for (a, b) in lines.into_iter().tuple_windows::<(_, _)>() {
                                    add_shape(
                                        crate::scene::DebugShape::Line([Vec3::zero(), b - a], 0.1),
                                        a,
                                    );
                                }
                            },
                        }
                    }
                },
            }
        }

        Ok(TickAction::Continue)
    }

    /// Clean up the session (and the client attached to it) after a tick.
    pub fn cleanup(&mut self) { self.client.borrow_mut().cleanup(); }

    fn should_auto_zoom_lock(&self) -> bool {
        let inputs_state = &self.inputs_state;
        for input in inputs_state {
            match input {
                GameInput::Primary
                | GameInput::Secondary
                | GameInput::Block
                | GameInput::MoveForward
                | GameInput::MoveLeft
                | GameInput::MoveRight
                | GameInput::MoveBack
                | GameInput::Jump
                | GameInput::WallJump
                | GameInput::Roll
                | GameInput::Sneak
                | GameInput::AutoWalk
                | GameInput::SwimUp
                | GameInput::SwimDown
                | GameInput::SwapLoadout
                | GameInput::ToggleWield
                | GameInput::Slot1
                | GameInput::Slot2
                | GameInput::Slot3
                | GameInput::Slot4
                | GameInput::Slot5
                | GameInput::Slot6
                | GameInput::Slot7
                | GameInput::Slot8
                | GameInput::Slot9
                | GameInput::Slot10
                | GameInput::CurrentSlot
                | GameInput::SpectateViewpoint
                | GameInput::SpectateSpeedBoost => return true,
                _ => (),
            }
        }
        false
    }
}

impl PlayState for SessionState {
    fn enter(&mut self, global_state: &mut GlobalState, _: Direction) {
        // Trap the cursor.
        global_state.window.grab_cursor(true);

        self.client.borrow_mut().clear_terrain();

        // Send startup commands to the server
        if global_state.settings.send_logon_commands {
            for cmd in &global_state.settings.logon_commands {
                self.client.borrow_mut().send_chat(cmd.to_string());
            }
        }

        #[cfg(feature = "discord")]
        {
            // Update the Discord activity on client initialization
            #[cfg(feature = "singleplayer")]
            let singleplayer = global_state.singleplayer.is_running();
            #[cfg(not(feature = "singleplayer"))]
            let singleplayer = false;

            if singleplayer {
                global_state.discord.join_singleplayer();
            } else {
                global_state
                    .discord
                    .join_server(self.client.borrow().server_info().name.clone());
            }
        }
    }

    fn tick(&mut self, global_state: &mut GlobalState, events: Vec<Event>) -> PlayStateResult {
        span!(_guard, "tick", "<Session as PlayState>::tick");
        // R0D Phase III: flag-gated auto-capture driver (no-op unless the
        // BASTION_R0D_CAPTURE_* env config is present). Once every requested
        // capture hash has been written, shut the session down cleanly.
        // §17.3: entity-present legs pause the sim once the trace stabilizes,
        // freezing the world exactly so warm-capture identity applies.
        // D1-replay: the first session tick releases the anchor pause — the
        // world resumes tick-aligned to the client in every run.
        #[cfg(feature = "singleplayer")]
        if crate::render::bastion_r0d::should_unpause_on_entry() {
            global_state.unpause();
        }
        #[cfg(feature = "singleplayer")]
        if crate::render::bastion_r0d::should_pause_sim_now(
            crate::render::bastion_r0d::capture_session_frames(),
        ) {
            global_state.pause();
        }
        // D1-replay mode keys captures to authoritative SIM TIME (fixed dt),
        // so two runs capture at identical ticks regardless of wall pacing.
        let r0d_sim_time = self.client.borrow().state().get_time();
        // R0D diagnostic (leg-22 finding: a 90-colonist frame rendered as 4
        // near-identical greens with zero non-green pixels): log the client's
        // figure count once/sec in capture mode, to distinguish "entities not
        // reaching the spectator render path" (structural) from "camera not
        // framing them" (mechanical). Cheap; capture-mode only.
        if crate::render::bastion_r0d::capture_config().is_some() {
            use std::sync::atomic::{AtomicU64, Ordering};
            static LAST: AtomicU64 = AtomicU64::new(0);
            let sec = r0d_sim_time as u64;
            if LAST.swap(sec, Ordering::SeqCst) != sec {
                let cl = self.client.borrow();
                let entity_count = {
                    use specs::Join;
                    let ecs = cl.state().ecs();
                    (&ecs.entities()).join().count()
                };
                tracing::info!(
                    target: "bastion_r0d",
                    "R0D-DIAG t={sec} figures={} figures_visible={} ecs_entities={entity_count}",
                    self.scene.figure_mgr().figure_count(),
                    self.scene.figure_mgr().figure_count_visible(),
                );
            }
        }
        if crate::render::bastion_r0d::drive_capture(
            global_state.window.renderer_mut(),
            r0d_sim_time,
        ) {
            return PlayStateResult::Shutdown;
        }
        // TODO: let mut client = self.client.borrow_mut();
        // TODO: can this be a method on the session or are there borrowcheck issues?
        let (client_presence, client_type, client_registered) = {
            let client = self.client.borrow();
            (
                client.presence(),
                *client.client_type(),
                client.registered(),
            )
        };

        if let Some(presence) = client_presence {
            let camera = self.scene.camera_mut();

            // Clamp camera's vertical angle if the toggle is enabled
            if self.camera_clamp {
                let mut cam_dir = camera.get_orientation();
                let cam_dir_clamp =
                    (global_state.settings.gameplay.camera_clamp_angle as f32).to_radians();
                cam_dir.y = cam_dir.y.clamp(-cam_dir_clamp, cam_dir_clamp);
                camera.set_orientation(cam_dir);
            }

            let client = self.client.borrow();
            let player_entity = client.entity();

            let dt = global_state.clock.real_dt().as_secs_f32();

            #[cfg(feature = "discord")]
            if global_state.discord.is_active()
                && let Some(chunk) = client.current_chunk()
                && let Some(location_name) = chunk.meta().name()
            {
                global_state
                    .discord
                    .update_location(location_name, client.current_site());
            }

            if global_state.settings.gameplay.bow_zoom {
                let mut fov_scaling = 1.0;
                if let Some(comp::CharacterState::ChargedRanged(cr)) = client
                    .state()
                    .read_storage::<comp::CharacterState>()
                    .get(player_entity)
                    && cr.charge_frac() > 0.5
                {
                    fov_scaling -= 3.0 * cr.charge_frac() / 5.0;
                }
                camera.set_fixate(fov_scaling);
            } else {
                camera.set_fixate(1.0);
            }

            // Compute camera data
            camera.compute_dependents(&client.state().terrain());
            let camera::Dependents {
                cam_pos, cam_dir, ..
            } = self.scene.camera().dependents();
            let focus_pos = self.scene.camera().get_focus_pos();
            let focus_off = focus_pos.map(|e| e.trunc());
            let cam_pos = cam_pos + focus_off;

            let (is_aiming, aim_dir_offset) = {
                let is_aiming = client
                    .state()
                    .read_storage::<comp::CharacterState>()
                    .get(player_entity)
                    .map(|cs| cs.is_wield())
                    .unwrap_or(false);

                (
                    is_aiming,
                    if is_aiming && self.scene.camera().get_mode() == CameraMode::ThirdPerson {
                        Vec3::unit_z() * 0.025
                    } else {
                        Vec3::zero()
                    },
                )
            };
            self.is_aiming = is_aiming;

            let can_build = client
                .state()
                .read_storage::<comp::CanBuild>()
                .get(player_entity)
                .map_or_else(|| false, |cb| cb.enabled);

            let active_mine_tool: Option<ToolKind> = if client.is_wielding() == Some(true) {
                client
                    .inventories()
                    .get(player_entity)
                    .and_then(|inv| inv.equipped(EquipSlot::ActiveMainhand))
                    .and_then(|item| item.tool_info())
                    .filter(|tool_kind| matches!(tool_kind, ToolKind::Pick | ToolKind::Shovel))
            } else {
                None
            };

            // Check to see whether we're aiming at anything
            let (build_target, collect_target, entity_target, mine_target, terrain_target) =
                targets_under_cursor(
                    &client,
                    cam_pos,
                    cam_dir,
                    can_build,
                    active_mine_tool,
                    self.viewpoint_entity().0,
                );

            match get_interactables(
                &client,
                collect_target,
                entity_target,
                mine_target,
                &self.scene,
            ) {
                Ok(input_map) => {
                    for (_, inter) in input_map.values() {
                        global_state.profile.tutorial.event_find_interactable(inter);
                    }

                    let entities = input_map
                        .values()
                        .filter_map(|(_, interactable)| {
                            if let Interactable::Entity { entity, .. } = interactable {
                                Some(*entity)
                            } else {
                                None
                            }
                        })
                        .collect::<HashSet<_>>();
                    self.interactables = interactable::Interactables {
                        input_map,
                        entities,
                    };
                },
                Err(error) => {
                    tracing::trace!(?error, "Getting interactables failed");
                    self.interactables = Default::default()
                },
            }

            drop(client);

            self.maybe_auto_zoom_lock(
                global_state.settings.gameplay.zoom_lock,
                global_state.settings.gameplay.zoom_lock_behavior,
            );

            if presence == PresenceKind::Spectator {
                // bastion (B1.5): in overseer mode, stream terrain under the
                // *focus* — the ground point being looked at — rather than
                // the camera boom position (which trails up to `dist` behind
                // it). This is what keeps grab-drag panning on full-detail
                // terrain instead of hitting a LoD wall.
                let stream_pos = if self.bastion_overseer_active() {
                    self.scene.camera().get_focus_pos()
                } else {
                    cam_pos
                };
                let mut client = self.client.borrow_mut();
                if client.spectate_position(stream_pos) {
                    let server_name = &client.server_info().name;
                    global_state.profile.set_spectate_position(
                        server_name,
                        Some(self.scene.camera().get_focus_pos()),
                    );
                }
            } else {
                // bastion (B1.6): embodied overseer — stream terrain around
                // the god camera via the terrain anchor (spectate_position
                // would teleport the avatar). Cleared on leaving the view.
                let anchor = self
                    .bastion_overseer_active()
                    .then(|| self.scene.camera().get_focus_pos());
                self.client.borrow_mut().bastion_set_terrain_anchor(anchor);
            }

            // Set break_block_pos based on currently selected block
            self.inputs.break_block_pos = if let Some(mt) = mine_target {
                self.scene.set_select_pos(Some(mt.position_int()));
                Some(mt.position)
            } else if let Some(bt) = build_target {
                self.scene.set_select_pos(Some(bt.position_int()));
                None
            } else if let Some(ct) = collect_target {
                self.scene.set_select_pos(Some(ct.position_int()));
                None
            } else {
                self.scene.set_select_pos(None);
                None
            };

            // filled block in line of sight
            let default_select_pos = terrain_target.map(|tt| tt.position);

            // Throw out distance info, it will be useful in the future
            self.target_entity = entity_target.map(|t| t.kind.0);

            let controlling_char = self.controlling_char();

            // Handle window events.
            for event in events {
                // Pass all events to the ui first.
                {
                    let client = self.client.borrow();
                    let inventories = client.inventories();
                    let inventory = inventories.get(client.entity());
                    if self
                        .hud
                        .handle_event(event.clone(), global_state, inventory)
                    {
                        continue;
                    }
                }
                match event {
                    Event::Close => {
                        return PlayStateResult::Shutdown;
                    },
                    Event::InputUpdate(input, state)
                        if state != self.inputs_state.contains(&input) =>
                    {
                        if !self.inputs_state.insert(input) {
                            self.inputs_state.remove(&input);
                        }
                        match input {
                            // bastion note (B1.5): the old no-op guard for
                            // Primary/Secondary/Interact is gone — the input-
                            // context layer suppresses avatar verbs at the
                            // window fan-out while the Overseer context is
                            // active (see bastion::input).
                            GameInput::BastionToggleOverseer if state => {
                                // The context switch (Overseer ⇄ Avatar stub).
                                // Only functional behind the launch flag so
                                // vanilla sessions are bit-identical.
                                if global_state.args.bastion_overseer {
                                    if self.bastion_overseer_active() {
                                        self.bastion_exit_overseer(global_state);
                                    } else {
                                        self.bastion_enter_overseer(global_state);
                                    }
                                }
                            },
                            GameInput::BastionRotateLeft | GameInput::BastionRotateRight
                                if state && self.bastion_overseer_active() =>
                            {
                                // Optional 90°-step yaw (free orbit lives on
                                // right-drag); keeps the current free pitch.
                                let step = if input == GameInput::BastionRotateLeft {
                                    core::f32::consts::FRAC_PI_2
                                } else {
                                    -core::f32::consts::FRAC_PI_2
                                };
                                let camera = self.scene.camera_mut();
                                let ori = camera.get_tgt_orientation();
                                let yaw = (ori.x / core::f32::consts::FRAC_PI_2).round()
                                    * core::f32::consts::FRAC_PI_2
                                    + step;
                                camera.set_orientation(Vec3::new(yaw, ori.y, 0.0));
                            },
                            GameInput::BastionSnapTopDown
                                if state && self.bastion_overseer_active() =>
                            {
                                // DF-style reading view: nearest 90° yaw,
                                // near-vertical pitch (eased by the camera
                                // orientation lerp).
                                let camera = self.scene.camera_mut();
                                let yaw = (camera.get_tgt_orientation().x
                                    / core::f32::consts::FRAC_PI_2)
                                    .round()
                                    * core::f32::consts::FRAC_PI_2;
                                camera.set_orientation(Vec3::new(
                                    yaw,
                                    camera::OVERSEER_PITCH_MAX,
                                    0.0,
                                ));
                            },
                            GameInput::BastionSliceUp | GameInput::BastionSliceDown
                                if state && self.bastion_overseer_active() =>
                            {
                                // First press activates the slice near the
                                // focus; movement happens per-frame while held
                                // (see the Overseer arm in the camera match).
                                // Also flip to the Slice view mode so the manual
                                // cut is actually composited by the occlusion
                                // framework (B1.6).
                                if self.scene.bastion_slice_z().is_none() {
                                    let z = self.scene.camera().get_focus_pos().z + 2.0;
                                    self.scene.set_bastion_slice_z(Some(z));
                                    self.scene.bastion_occlusion_mut().view_mode =
                                        bastion::occlusion::ViewMode::Slice;
                                }
                            },
                            GameInput::BastionCycleViewMode
                                if state && self.bastion_overseer_active() =>
                            {
                                // B1.6: cycle Solid → Reveal → Slice.
                                let occ = self.scene.bastion_occlusion_mut();
                                occ.view_mode = occ.view_mode.next();
                                let label = occ.view_mode.label();
                                // Entering Slice with no cut set yet: activate
                                // it just above the focus (same as a first
                                // PgUp/PgDn press) so the mode visibly does
                                // something instead of looking like Reveal.
                                if occ.view_mode == bastion::occlusion::ViewMode::Slice
                                    && self.scene.bastion_slice_z().is_none()
                                {
                                    let z = self.scene.camera().get_focus_pos().z + 2.0;
                                    self.scene.set_bastion_slice_z(Some(z));
                                }
                                self.hud.new_message(
                                    ChatType::CommandInfo
                                        .into_plain_msg(format!("Overseer view: {label}")),
                                );
                            },
                            GameInput::BastionCycleTool
                                if state && self.bastion_overseer_active() =>
                            {
                                // B2a: cycle the pinned tool.
                                self.bastion_set_tool(self.bastion_tools.tool.next());
                                // B5.6a: tool change can flip overlay
                                // auto-reveal (paint/erase reveals while
                                // OFF) — rebuild.
                                self.bastion_designation_dirty = true;
                                let label = self.bastion_tools.tool.label();
                                self.hud.new_message(
                                    ChatType::CommandInfo
                                        .into_plain_msg(format!("Overseer tool: {label}")),
                                );
                            },
                            GameInput::BastionCycleVisuals
                                if state && self.bastion_overseer_active() =>
                            {
                                // B5.6a: cycle designation-visuals ON/SUBTLE/
                                // OFF. Visual-only — designations stay fully
                                // active; the overlay rebuilds next frame.
                                self.bastion_visuals = self.bastion_visuals.next();
                                self.bastion_designation_dirty = true;
                                let label = self.bastion_visuals.label();
                                self.hud.new_message(
                                    ChatType::CommandInfo
                                        .into_plain_msg(format!("Designation {label}")),
                                );
                            },
                            GameInput::BastionToggleGodMode
                                if state && self.bastion_overseer_active() =>
                            {
                                // B2a: God/Free ruleset toggle (stub — teeth
                                // in B2b when the colony + favor exist).
                                self.bastion_tools.god_mode = self.bastion_tools.god_mode.toggled();
                                let label = self.bastion_tools.god_mode.label();
                                self.hud.new_message(ChatType::CommandInfo.into_plain_msg(
                                    format!("Overseer ruleset: {label} (enforced from B2b)"),
                                ));
                            },
                            GameInput::BastionPauseToggle
                                if state && self.bastion_overseer_active() =>
                            {
                                // TIME-CONTROLS: Space = pause toggle. Same
                                // state the HUD buttons drive; resume keeps
                                // the pre-pause scale.
                                let target = if global_state.paused() {
                                    Some(
                                        self.client
                                            .borrow()
                                            .state()
                                            .ecs()
                                            .read_resource::<common::resources::TimeScale>()
                                            .0 as f32,
                                    )
                                } else {
                                    None
                                };
                                self.bastion_set_sim_speed(global_state, target);
                            },
                            GameInput::BastionSpeedUp | GameInput::BastionSpeedDown
                                if state && self.bastion_overseer_active() =>
                            {
                                // TIME-CONTROLS: +/− step the speed ladder
                                // (below 1× = pause).
                                self.bastion_step_sim_speed(
                                    global_state,
                                    input == GameInput::BastionSpeedUp,
                                );
                            },
                            GameInput::Primary => {
                                self.walking_speed = false;
                                let mut client = self.client.borrow_mut();
                                // Building inputs take precedence... but only if there's an active
                                // building target.
                                if let Some(build_target) = build_target.filter(|_| state) {
                                    client.remove_block(build_target.position_int());
                                } else {
                                    client.handle_input(
                                        InputKind::Primary,
                                        state,
                                        default_select_pos,
                                        self.target_entity,
                                    );
                                }
                            },
                            GameInput::Secondary => {
                                self.walking_speed = false;
                                let mut client = self.client.borrow_mut();
                                if let Some(build_target) = build_target.filter(|_| state) {
                                    let selected_pos = build_target.kind.0;
                                    client.place_block(
                                        selected_pos.map(|p| p.floor() as i32),
                                        self.selected_block,
                                    );
                                } else {
                                    client.handle_input(
                                        InputKind::Secondary,
                                        state,
                                        default_select_pos,
                                        self.target_entity,
                                    );
                                }
                            },
                            GameInput::Block => {
                                self.walking_speed = false;
                                self.client.borrow_mut().handle_input(
                                    InputKind::Block,
                                    state,
                                    default_select_pos,
                                    self.target_entity,
                                );
                            },
                            GameInput::Roll => {
                                self.walking_speed = false;
                                let mut client = self.client.borrow_mut();
                                if can_build {
                                    if state
                                        && let Some(block) = build_target.and_then(|bt| {
                                            client
                                                .state()
                                                .terrain()
                                                .get(bt.position_int())
                                                .ok()
                                                .copied()
                                        })
                                    {
                                        self.selected_block = block;
                                    }
                                } else if controlling_char {
                                    global_state.profile.tutorial.event_roll();
                                    client.handle_input(
                                        InputKind::Roll,
                                        state,
                                        default_select_pos,
                                        self.target_entity,
                                    );
                                }
                            },
                            GameInput::GiveUp => {
                                self.key_state.give_up = state.then_some(0.0).filter(|_| {
                                    let client = self.client.borrow();
                                    comp::is_downed(
                                        client.current().as_ref(),
                                        client.current().as_ref(),
                                    )
                                });
                            },
                            GameInput::Respawn => {
                                self.walking_speed = false;
                                self.stop_auto_walk();
                                if state && self.client.borrow_mut().respawn() {
                                    global_state.profile.tutorial.event_respawn();
                                    self.scene.screen_fade = -0.5;
                                }
                            },
                            GameInput::Jump => {
                                self.walking_speed = false;
                                global_state.profile.tutorial.event_jump();
                                self.client.borrow_mut().handle_input(
                                    InputKind::Jump,
                                    state,
                                    default_select_pos,
                                    self.target_entity,
                                );
                            },
                            GameInput::WallJump => {
                                self.walking_speed = false;
                                self.client.borrow_mut().handle_input(
                                    InputKind::WallJump,
                                    state,
                                    default_select_pos,
                                    self.target_entity,
                                );
                            },
                            GameInput::SwimUp => {
                                self.key_state.swim_up = state;
                            },
                            GameInput::SwimDown => {
                                self.key_state.swim_down = state;
                            },
                            GameInput::Sit => {
                                if state && controlling_char {
                                    self.stop_auto_walk();
                                    self.client.borrow_mut().toggle_sit();
                                }
                            },
                            GameInput::Crawl => {
                                if state && controlling_char {
                                    self.stop_auto_walk();
                                    self.client.borrow_mut().toggle_crawl();
                                }
                            },
                            GameInput::Dance => {
                                if state && controlling_char {
                                    self.stop_auto_walk();
                                    self.client.borrow_mut().toggle_dance();
                                }
                            },
                            GameInput::Greet => {
                                if state {
                                    self.client.borrow_mut().utter(UtteranceKind::Greeting);
                                }
                            },
                            GameInput::Sneak => {
                                let is_trading = self.client.borrow().is_trading();
                                if state && !is_trading && controlling_char {
                                    self.stop_auto_walk();
                                    self.client.borrow_mut().toggle_sneak();
                                }
                            },
                            GameInput::CancelClimb => {
                                if state && controlling_char {
                                    self.client.borrow_mut().cancel_climb();
                                }
                            },
                            GameInput::MoveForward => {
                                if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                                    self.stop_auto_walk();
                                }
                                self.key_state.up = state
                            },
                            GameInput::MoveBack => {
                                if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                                    self.stop_auto_walk();
                                }
                                self.key_state.down = state
                            },
                            GameInput::MoveLeft => {
                                if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                                    self.stop_auto_walk();
                                }
                                self.key_state.left = state
                            },
                            GameInput::MoveRight => {
                                if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                                    self.stop_auto_walk();
                                }
                                self.key_state.right = state
                            },
                            GameInput::Glide => {
                                self.walking_speed = false;
                                let is_trading = self.client.borrow().is_trading();
                                if state && !is_trading && controlling_char {
                                    if global_state.settings.gameplay.stop_auto_walk_on_input {
                                        self.stop_auto_walk();
                                    }
                                    self.client.borrow_mut().toggle_glide();
                                    global_state.profile.tutorial.event_open_glider();
                                }
                            },
                            GameInput::Fly => {
                                // Not sure where to put comment, but I noticed
                                // when testing flight.
                                //
                                // Syncing of inputs between mounter and mountee
                                // broke with controller change
                                self.key_state.fly ^= state;
                                self.client.borrow_mut().handle_input(
                                    InputKind::Fly,
                                    self.key_state.fly,
                                    default_select_pos,
                                    self.target_entity,
                                );
                            },
                            GameInput::ToggleWield => {
                                if state && controlling_char {
                                    let mut client = self.client.borrow_mut();
                                    if client.is_wielding().is_some_and(|b| !b) {
                                        self.walking_speed = false;
                                    }
                                    client.toggle_wield();
                                }
                            },
                            GameInput::SwapLoadout => {
                                if state && controlling_char {
                                    self.client.borrow_mut().swap_loadout();
                                }
                            },
                            GameInput::ToggleLantern if state && controlling_char => {
                                let mut client = self.client.borrow_mut();
                                if client.is_lantern_enabled() {
                                    client.disable_lantern();
                                } else {
                                    global_state.profile.tutorial.event_lantern();
                                    client.enable_lantern();
                                }
                            },
                            GameInput::Mount if state && controlling_char => {
                                let mut client = self.client.borrow_mut();
                                if client.is_riding() {
                                    client.unmount();
                                } else if let Some((_, interactable)) =
                                    self.interactables.input_map.get(&GameInput::Mount)
                                {
                                    match interactable {
                                        Interactable::Block { volume_pos, .. } => {
                                            client.mount_volume(*volume_pos)
                                        },
                                        Interactable::Entity { entity, .. } => {
                                            client.mount(*entity)
                                        },
                                    }
                                }
                            },
                            GameInput::StayFollow if state => {
                                let mut client = self.client.borrow_mut();
                                let player_pos = client
                                    .state()
                                    .read_storage::<Pos>()
                                    .get(client.entity())
                                    .copied();

                                let mut close_pet = None;
                                if let Some(player_pos) = player_pos {
                                    let positions = client.state().read_storage::<Pos>();
                                    close_pet = client.state().ecs().read_resource::<CachedSpatialGrid>().0
                                        .in_circle_aabr(player_pos.0.xy(), MAX_MOUNT_RANGE)
                                        .filter(|e|
                                            *e != client.entity()
                                        )
                                        .filter(|e|
                                            matches!(client.state().ecs().read_storage::<comp::Alignment>().get(*e),
                                                Some(comp::Alignment::Owned(owner)) if Some(*owner) == client.uid())
                                        )
                                        .filter(|e|
                                            client.state().ecs().read_storage::<Is<Mount>>().get(*e).is_none()
                                        )
                                        .min_by_key(|e| {
                                            OrderedFloat(positions
                                                .get(*e)
                                                .map_or(MAX_MOUNT_RANGE * MAX_MOUNT_RANGE, |x| {
                                                    player_pos.0.distance_squared(x.0)
                                                }
                                            ))
                                        });
                                }
                                if let Some(pet_entity) = close_pet
                                    && client
                                        .state()
                                        .read_storage::<Is<Mount>>()
                                        .get(pet_entity)
                                        .is_none()
                                {
                                    let is_staying = client
                                        .state()
                                        .read_storage::<CharacterActivity>()
                                        .get(pet_entity)
                                        .is_some_and(|activity| activity.is_pet_staying);
                                    client.set_pet_stay(pet_entity, !is_staying);
                                }
                            },
                            GameInput::Interact => {
                                if state {
                                    let mut client = self.client.borrow_mut();
                                    if let Some((_, interactable)) =
                                        self.interactables.input_map.get(&GameInput::Interact)
                                    {
                                        match interactable {
                                            Interactable::Block {
                                                volume_pos,
                                                block,
                                                interaction,
                                                ..
                                            } => {
                                                match interaction {
                                                    BlockInteraction::Collect { .. }
                                                    | BlockInteraction::Unlock { .. } => {
                                                        if block.is_directly_collectible() {
                                                            match volume_pos.kind {
                                                                common::mounting::Volume::Terrain => {
                                                                    client.collect_block(volume_pos.pos);
                                                                }
                                                                common::mounting::Volume::Entity(_) => {
                                                                    // TODO: Do we want to implement this?
                                                                },
                                                            }
                                                        }
                                                    },
                                                    BlockInteraction::Craft(tab) => {
                                                        self.hud.show.open_crafting_tab(
                                                            *tab,
                                                            block
                                                                .get_sprite()
                                                                .map(|s| (*volume_pos, s)),
                                                        )
                                                    },
                                                    BlockInteraction::Mine(_)
                                                    | BlockInteraction::Mount => {},
                                                    BlockInteraction::Read(content) => {
                                                        match volume_pos.kind {
                                                            common::mounting::Volume::Terrain => {
                                                                self.hud.show_content_bubble(
                                                                    volume_pos.pos.as_()
                                                                        + Vec3::new(
                                                                            0.5,
                                                                            0.5,
                                                                            block.solid_height()
                                                                                * 0.75,
                                                                        ),
                                                                    content.clone(),
                                                                )
                                                            },
                                                            // Signs on volume entities are not
                                                            // currently supported
                                                            common::mounting::Volume::Entity(_) => {
                                                            },
                                                        }
                                                    },
                                                    BlockInteraction::LightToggle(enable) => {
                                                        client.toggle_sprite_light(
                                                            *volume_pos,
                                                            *enable,
                                                        );
                                                    },
                                                }
                                            },
                                            Interactable::Entity {
                                                entity,
                                                interaction,
                                                ..
                                            } => {
                                                // NOTE: Keep this match exhaustive.
                                                match interaction {
                                                    EntityInteraction::HelpDowned => {
                                                        client.help_downed(*entity)
                                                    },
                                                    EntityInteraction::PickupItem => {
                                                        client.pick_up(*entity)
                                                    },
                                                    EntityInteraction::ActivatePortal => {
                                                        client.activate_portal(*entity)
                                                    },
                                                    EntityInteraction::Pet => {
                                                        client.do_pet(*entity)
                                                    },
                                                    EntityInteraction::Talk => {
                                                        client.npc_interact(*entity)
                                                    },
                                                    EntityInteraction::CampfireSit
                                                    | EntityInteraction::Trade
                                                    | EntityInteraction::StayFollow
                                                    | EntityInteraction::Mount => {},
                                                }
                                            },
                                        }
                                    }
                                }
                            },
                            GameInput::Trade => {
                                if state
                                    && controlling_char
                                    && let Some((_, Interactable::Entity { entity, .. })) =
                                        self.interactables.input_map.get(&GameInput::Trade)
                                {
                                    let mut client = self.client.borrow_mut();
                                    if let Some(uid) = client.state().ecs().uid_from_entity(*entity)
                                    {
                                        let name = client
                                            .player_list()
                                            .get(&uid)
                                            .map(|info| info.player_alias.clone())
                                            .unwrap_or_else(|| {
                                                let stats = client.state().read_storage::<Stats>();
                                                stats.get(*entity).map_or(
                                                    format!("<entity {:?}>", uid),
                                                    |e| {
                                                        global_state
                                                            .i18n
                                                            .read()
                                                            .get_content(&e.name)
                                                    },
                                                )
                                            });

                                        self.hud.new_message(ChatType::Meta.into_msg(
                                            Content::localized_with_args(
                                                "hud-trade-invite_sent",
                                                [("playername", name)],
                                            ),
                                        ));

                                        client.send_invite(uid, InviteKind::Trade)
                                    };
                                }
                            },
                            GameInput::FreeLook => {
                                let hud = &mut self.hud;
                                global_state.settings.gameplay.free_look_behavior.update(
                                    state,
                                    &mut self.free_look,
                                    |b| hud.free_look(b),
                                );
                                let camera = self.scene.camera_mut();
                                let ori = camera.get_orientation();
                                if self.free_look {
                                    self.freecam_pos = ori;
                                } else {
                                    camera.set_orientation_instant(self.freecam_pos);
                                }
                            },
                            GameInput::AutoWalk => {
                                let hud = &mut self.hud;
                                global_state.settings.gameplay.auto_walk_behavior.update(
                                    state,
                                    &mut self.auto_walk,
                                    |b| hud.auto_walk(b),
                                );

                                self.key_state.auto_walk =
                                    self.auto_walk && !self.client.borrow().is_gliding();
                            },
                            GameInput::ZoomIn => {
                                if state {
                                    if self.zoom_lock {
                                        self.hud.zoom_lock_reminder();
                                    } else {
                                        self.scene.handle_input_event(
                                            Event::Zoom(-30.0),
                                            &self.client.borrow(),
                                        );
                                    }
                                }
                            },
                            GameInput::ZoomOut => {
                                if state {
                                    if self.zoom_lock {
                                        self.hud.zoom_lock_reminder();
                                    } else {
                                        self.scene.handle_input_event(
                                            Event::Zoom(30.0),
                                            &self.client.borrow(),
                                        );
                                    }
                                }
                            },
                            GameInput::ZoomLock => {
                                if state {
                                    global_state.settings.gameplay.zoom_lock ^= true;

                                    self.hud
                                        .zoom_lock_toggle(global_state.settings.gameplay.zoom_lock);
                                }
                            },
                            GameInput::CameraClamp => {
                                let hud = &mut self.hud;
                                global_state.settings.gameplay.camera_clamp_behavior.update(
                                    state,
                                    &mut self.camera_clamp,
                                    |b| hud.camera_clamp(b),
                                );
                            },
                            GameInput::CycleCamera if state => {
                                // Prevent accessing camera modes which aren't available in
                                // multiplayer unless you are an
                                // admin. This is an easily bypassed clientside check.
                                // The server should do its own filtering of which entities are
                                // sent to clients to
                                // prevent abuse.
                                let camera = self.scene.camera_mut();
                                let client = self.client.borrow();
                                camera.next_mode(
                                    client.is_moderator(),
                                    (client.presence() != Some(PresenceKind::Spectator))
                                        || self.viewpoint_entity.is_some(),
                                );
                            },
                            GameInput::Select => {
                                if !state {
                                    self.selected_entity =
                                        self.target_entity.map(|e| (e, std::time::Instant::now()));
                                }
                            },
                            GameInput::AcceptGroupInvite if state => {
                                let mut client = self.client.borrow_mut();
                                if client.invite().is_some() {
                                    client.accept_invite();
                                }
                            },
                            GameInput::DeclineGroupInvite if state => {
                                let mut client = self.client.borrow_mut();
                                if client.invite().is_some() {
                                    client.decline_invite();
                                }
                            },
                            GameInput::SpectateViewpoint if state => {
                                let mut client = self.client.borrow_mut();
                                if self.viewpoint_entity.is_some() {
                                    client.stop_spectate_entity();
                                    self.viewpoint_entity = None;
                                    self.scene.camera_mut().set_mode(CameraMode::Freefly);
                                    let mut ori = self.scene.camera().get_orientation();
                                    // Remove any roll that could have possibly been set to the
                                    // camera as a result of spectating.
                                    ori.z = 0.0;
                                    self.scene.camera_mut().set_orientation(ori);
                                } else if let Some(target_entity) = entity_target
                                    && self.scene.camera().get_mode() == CameraMode::Freefly
                                {
                                    // Notify the server that we start spectating an entity so
                                    // we get viewpoint specific component packages.
                                    client.start_spectate_entity(target_entity.kind.0);

                                    self.viewpoint_entity = Some(target_entity.kind.0);
                                    self.scene.camera_mut().set_mode(CameraMode::FirstPerson);
                                }
                            },
                            GameInput::ToggleWalk if state => {
                                global_state
                                    .settings
                                    .gameplay
                                    .walking_speed_behavior
                                    .update(state, &mut self.walking_speed, |_| {});
                            },
                            _ => {},
                        }
                    },
                    // bastion (B1.5): overseer mouse controls — grab-drag pan,
                    // free orbit + pitch, zoom-to-cursor. The overseer runs
                    // with a free cursor, so these consume the *raw* mouse
                    // events (mouse→GameInput mapping is grab-gated upstream).
                    Event::MouseButton(button, mb_state) if self.bastion_overseer_active() => {
                        let pressed = mb_state == winit::event::ElementState::Pressed;
                        let cursor = {
                            let c = global_state.window.cursor_position();
                            Vec2::new(c.x as f32, c.y as f32)
                        };
                        // Release within this many pixels of the press is a
                        // *click* (select / radial), not a drag (pan / orbit).
                        const CLICK_SLOP_SQ: f32 = 36.0;
                        match button {
                            winit::event::MouseButton::Left => {
                                if pressed {
                                    // Clicks on HUD widgets are for the HUD.
                                    if !self.hud.bastion_cursor_over_widget() {
                                        // A world-press dismisses an open
                                        // radial menu.
                                        self.hud.bastion_close_radial();
                                        self.bastion_lmb_down = Some(cursor);
                                        match self.bastion_tools.tool {
                                            // B2a: designate tool paints
                                            // instead of panning. B5.5: the
                                            // erase tool uses the same drag.
                                            bastion::tools::ToolMode::Designate(_)
                                            | bastion::tools::ToolMode::Erase => {
                                                self.bastion_paint_begin(global_state);
                                            },
                                            // B3: inspect tool box-selects.
                                            bastion::tools::ToolMode::Inspect => {
                                                self.bastion_boxsel_begin(global_state);
                                            },
                                            bastion::tools::ToolMode::Pan => {
                                                self.bastion_begin_grab(global_state);
                                            },
                                        }
                                    }
                                } else {
                                    if let bastion::tools::ToolMode::Designate(kind) =
                                        self.bastion_tools.tool
                                        && self.bastion_paint.is_some()
                                    {
                                        self.bastion_paint_finish(Some(kind));
                                    } else if self.bastion_tools.tool
                                        == bastion::tools::ToolMode::Erase
                                        && self.bastion_paint.is_some()
                                    {
                                        // B5.5: erase = same drag, cancel op.
                                        self.bastion_paint_finish(None);
                                    } else if self.bastion_boxsel.is_some() {
                                        // B3: box-select release (tiny drag
                                        // falls back to single pick).
                                        self.bastion_boxsel_finish(global_state);
                                    } else if let Some(down) = self.bastion_lmb_down
                                        && down.distance_squared(cursor) < CLICK_SLOP_SQ
                                        && !self.hud.bastion_cursor_over_widget()
                                    {
                                        // B2a: left-click = select/inspect
                                        // (entity under cursor, or clear).
                                        let picked = self.bastion_pick_entity(global_state);
                                        self.bastion_select_set(picked.into_iter().collect());
                                    }
                                    self.bastion_lmb_down = None;
                                    // Release: keep the tracked velocity as
                                    // inertia (applied per-frame with decay).
                                    self.bastion_grab = None;
                                }
                            },
                            winit::event::MouseButton::Right => {
                                if pressed {
                                    self.bastion_rmb_down = Some(cursor);
                                    self.bastion_orbiting = true;
                                } else {
                                    self.bastion_orbiting = false;
                                    if let Some(down) = self.bastion_rmb_down.take()
                                        && down.distance_squared(cursor) < CLICK_SLOP_SQ
                                        && !self.hud.bastion_cursor_over_widget()
                                    {
                                        // B2a: right-click = contextual
                                        // radial menu.
                                        self.bastion_open_radial(global_state);
                                    }
                                }
                            },
                            _ => {},
                        }
                    },
                    Event::CursorMove(delta)
                        if self.bastion_orbiting && self.bastion_overseer_active() =>
                    {
                        // Free orbit: continuous yaw, pitch clamped to the
                        // overseer swoop range; damping comes from the
                        // camera's orientation interpolation.
                        let invert_y = if global_state.window.mouse_y_inversion {
                            -1.0
                        } else {
                            1.0
                        };
                        let camera = self.scene.camera_mut();
                        let ori = camera.get_tgt_orientation();
                        let yaw = ori.x - delta.x * BASTION_ORBIT_SENS;
                        let pitch = (ori.y + delta.y * BASTION_ORBIT_SENS * invert_y)
                            .clamp(camera::OVERSEER_PITCH_MIN, camera::OVERSEER_PITCH_MAX);
                        camera.set_orientation(Vec3::new(yaw, pitch, 0.0));
                    },
                    Event::Zoom(delta) if self.bastion_overseer_active() => {
                        // B5.6b-2: scrolling DURING a designate drag adjusts
                        // the zone depth (one notch = one level) instead of
                        // zooming — the drag-in-space half of the volume-
                        // selection UX. The wheel arrives pre-scaled ~±15
                        // per notch (window.rs X11-parity factor).
                        if self.bastion_paint.is_some()
                            && matches!(
                                self.bastion_tools.tool,
                                crate::bastion::tools::ToolMode::Designate(_)
                            )
                        {
                            // Scroll down = dig deeper, up = shallower (then
                            // upward) — physical direction matches the zone.
                            let steps = if delta < 0.0 { 1 } else { -1 };
                            self.bastion_tools.step_z_extent(steps);
                            self.bastion_paint_update(global_state);
                        } else {
                            // Must precede the vanilla zoom_lock arm: zooming
                            // while WASD-panning is core to the overseer feel.
                            self.bastion_zoom_to_cursor(global_state, delta);
                        }
                    },
                    Event::AnalogGameInput(input) => match input {
                        AnalogGameInput::MovementX(v) => {
                            self.key_state.analog_matrix.x = v;
                        },
                        AnalogGameInput::MovementY(v) => {
                            self.key_state.analog_matrix.y = v;
                        },
                        other => {
                            self.scene.handle_input_event(
                                Event::AnalogGameInput(other),
                                &self.client.borrow(),
                            );
                        },
                    },

                    // TODO: Localise
                    Event::ScreenshotMessage(screenshot_msg) => self
                        .hud
                        .new_message(ChatType::CommandInfo.into_plain_msg(screenshot_msg)),

                    Event::Zoom(delta) if self.zoom_lock => {
                        // only fire this Hud event when player has "intent" to zoom
                        if delta.abs() > ZOOM_LOCK_SCROLL_DELTA_INTENT {
                            self.hud.zoom_lock_reminder();
                        }
                    },

                    // Pass all other events to the scene
                    event => {
                        if let Event::Zoom(delta) = &event {
                            global_state.profile.tutorial.event_zoom(*delta);
                        }

                        self.scene.handle_input_event(event, &self.client.borrow());
                    }, // TODO: Do something if the event wasn't handled?
                }
            }

            // Talk to entities when we are in dialogue with them
            if let Some(tgt) = self.hud.current_dialogue() {
                let mut client = self.client.borrow_mut();
                client.do_talk(Some(tgt));
                // Turn to face the entity when in first-person mode
                if matches!(
                    self.scene.camera().get_mode(),
                    camera::CameraMode::FirstPerson
                ) && let Some(activity) = client
                    .state()
                    .read_storage::<CharacterActivity>()
                    .get(client.entity())
                    && let Some(dir) = activity.look_dir
                {
                    let ori = Vec3::new(dir.x.atan2(dir.y), -dir.z.atan(), 0.0);
                    self.scene.camera_mut().lerp_toward(ori, dt, 2.5);
                }
            }

            if let Some(viewpoint_entity) = self.viewpoint_entity
                && !self
                    .client
                    .borrow()
                    .state()
                    .ecs()
                    .read_storage::<Pos>()
                    .contains(viewpoint_entity)
            {
                self.client.borrow_mut().stop_spectate_entity();
                self.viewpoint_entity = None;
                self.scene.camera_mut().set_mode(CameraMode::Freefly);
            }

            let (viewpoint_entity, mutable_viewpoint) = self.viewpoint_entity();

            // Get the current state of movement related inputs
            let input_vec = self.key_state.dir_vec();
            if input_vec.magnitude_squared() > 0.5f32.powi(2) {
                global_state.profile.tutorial.event_move()
            }
            let (axis_right, axis_up) = (input_vec[0], input_vec[1]);

            if let Some(ref mut timer) = self.key_state.give_up {
                use crate::key_state::GIVE_UP_HOLD_TIME;
                *timer += dt;

                if *timer > GIVE_UP_HOLD_TIME {
                    self.client.borrow_mut().give_up();
                }
            }

            if mutable_viewpoint {
                // If auto-gliding, point camera into the wind
                if let Some(dir) = self
                    .auto_walk
                    .then_some(self.client.borrow())
                    .filter(|client| client.is_gliding())
                    .and_then(|client| {
                        let ecs = client.state().ecs();
                        let entity = client.entity();
                        let fluid = ecs
                            .read_storage::<comp::PhysicsState>()
                            .get(entity)?
                            .in_fluid?;
                        let vel = *ecs.read_storage::<Vel>().get(entity)?;
                        let free_look = self.free_look;
                        let dir_forward_xy = self.scene.camera().forward_xy();
                        let dir_right = self.scene.camera().right();

                        auto_glide(fluid, vel, free_look, dir_forward_xy, dir_right)
                    })
                {
                    self.key_state.auto_walk = false;
                    self.inputs.move_dir = Vec2::zero();
                    self.inputs.look_dir = dir;
                } else {
                    self.key_state.auto_walk = self.auto_walk;
                    if !self.free_look {
                        self.walk_forward_dir = self.scene.camera().forward_xy();
                        self.walk_right_dir = self.scene.camera().right_xy();

                        let client = self.client.borrow();

                        let holding_ranged = client
                            .inventories()
                            .get(player_entity)
                            .and_then(|inv| inv.equipped(EquipSlot::ActiveMainhand))
                            .and_then(|item| item.tool_info())
                            .is_some_and(|tool_kind| {
                                matches!(
                                    tool_kind,
                                    ToolKind::Bow
                                        | ToolKind::Staff
                                        | ToolKind::Sceptre
                                        | ToolKind::Throwable
                                )
                            })
                            || client
                                .current::<CharacterState>()
                                .is_some_and(|char_state| {
                                    matches!(char_state, CharacterState::Throw(_))
                                });

                        let dir = if is_aiming
                            && holding_ranged
                            && self.scene.camera().get_mode() == CameraMode::ThirdPerson
                        {
                            // Shoot ray from camera focus forwards and get the point it hits an
                            // entity or terrain. The ray starts from the camera focus point
                            // so that the player won't aim at things behind them, in front of the
                            // camera.
                            let ray_start = self.scene.camera().get_focus_pos();
                            let entity_ray_end = ray_start + cam_dir * 1000.0;
                            let terrain_ray_end = ray_start + cam_dir * 1000.0;

                            let aim_point = {
                                // Get the distance to nearest entity and terrain
                                let entity_dist =
                                    ray_entities(&client, ray_start, entity_ray_end, 1000.0).0;
                                let terrain_ray_distance = client
                                    .state()
                                    .terrain()
                                    .ray(ray_start, terrain_ray_end)
                                    .max_iter(1000)
                                    .until(Block::is_solid)
                                    .cast()
                                    .0;

                                // Return the hit point of whichever was smaller
                                ray_start + cam_dir * entity_dist.min(terrain_ray_distance)
                            };

                            // Get player orientation
                            let ori = client
                                .state()
                                .read_storage::<comp::Ori>()
                                .get(player_entity)
                                .copied()
                                .unwrap();
                            // Get player scale
                            let scale = client
                                .state()
                                .read_storage::<comp::Scale>()
                                .get(player_entity)
                                .copied()
                                .unwrap_or(comp::Scale(1.0));
                            // Get player body offsets
                            let body = client
                                .state()
                                .read_storage::<comp::Body>()
                                .get(player_entity)
                                .copied()
                                .unwrap();
                            let body_offsets = body.projectile_offsets(ori.look_vec(), scale.0);

                            // Get direction from player character to aim point
                            let player_pos = client
                                .state()
                                .read_storage::<Pos>()
                                .get(player_entity)
                                .copied()
                                .unwrap();

                            drop(client);
                            aim_point - (player_pos.0 + body_offsets)
                        } else {
                            cam_dir + aim_dir_offset
                        };

                        self.inputs.look_dir = Dir::from_unnormalized(dir).unwrap();
                    }
                }
                self.inputs.strafing = matches!(
                    self.scene.camera().get_mode(),
                    camera::CameraMode::FirstPerson
                );

                // Auto camera mode
                if global_state.settings.gameplay.auto_camera
                    && matches!(
                        self.scene.camera().get_mode(),
                        camera::CameraMode::ThirdPerson | camera::CameraMode::FirstPerson
                    )
                    && input_vec.magnitude_squared() > 0.0
                {
                    let camera = self.scene.camera_mut();
                    let ori = camera.get_orientation();
                    camera.set_orientation_instant(Vec3::new(
                        ori.x
                            + input_vec.x
                                * (3.0 - input_vec.y * 1.5 * if is_aiming { 1.5 } else { 1.0 })
                                * dt,
                        std::f32::consts::PI * if is_aiming { 0.015 } else { 0.1 },
                        0.0,
                    ));
                }

                self.inputs.move_z =
                    self.key_state.swim_up as i32 as f32 - self.key_state.swim_down as i32 as f32;
            }

            // bastion: deferred overseer entry (launch flag) once the player
            // entity exists to take the initial focus from.
            if self.bastion_pending_overseer && self.client.borrow().position().is_some() {
                self.bastion_pending_overseer = false;
                self.bastion_enter_overseer(global_state);
            }
            // R0D capture mode: `client.position()` never resolves for a
            // silent_spectator session (no avatar entity ever gets a Pos
            // component), so the launch-flag overseer entry above never
            // fires and the camera is left at Camera::new's raw default
            // (near world origin, unrelated to the flat-arena's actual
            // world-center spawn) — the leg-21 finding: a solid-black-then-
            // solid-green degenerate view with no visible terrain/entities.
            // Fix: explicitly spectate-position + enter overseer at the
            // flat-arena's world-center wpos (1024 chunks/side * 32
            // blocks/chunk / 2 = 16384 — MapSizeLg::new((10,10)) is the
            // engine default and this value has been externally observed
            // identical across every seed/run in the campaign, confirming
            // it's a fixed build constant, not seed-derived). One-shot.
            // R0D dynamic-capture camera (Ben-directed, leg-24 fix): a fixed
            // spectator point aimed at world origin never framed the
            // colonists (figures drawn but off-screen). Instead, EACH capture
            // tick anchor the camera to a live colonist: pick the LOWEST-Uid
            // alive bodied entity — deterministic (the target is a pure
            // function of authoritative state; in the flat arena the only
            // bodied entities are the spawned colonists) — and orbit the
            // camera on it with a fixed chase distance/angle, so a walking
            // colonist is centered and large in every frame. In Freefly the
            // focus is the look-at point and dist is how far back the eye
            // sits, so the target stays centered regardless of orbit angle.
            if crate::render::bastion_r0d::capture_config().is_some() {
                let target = {
                    use specs::Join;
                    let cl = self.client.borrow();
                    let ecs = cl.state().ecs();
                    let uids = ecs.read_storage::<common::uid::Uid>();
                    let positions = ecs.read_storage::<comp::Pos>();
                    let bodies = ecs.read_storage::<comp::Body>();
                    (&uids, &positions, &bodies)
                        .join()
                        .min_by_key(|(uid, _, _)| uid.0.get())
                        .map(|(_, pos, _)| pos.0)
                };
                if let Some(tgt) = target {
                    let camera = self.scene.camera_mut();
                    camera.set_mode(CameraMode::Freefly);
                    camera.set_distance(6.0);
                    // Chase angle: yaw 45°, modest downward pitch onto the
                    // colonist. Frozen — never wall-time-driven.
                    camera.set_orientation_instant(Vec3::new(
                        core::f32::consts::FRAC_PI_4,
                        0.3,
                        0.0,
                    ));
                    // Look at the colonist's torso (feet at pos, +1 block up).
                    camera.force_focus_pos(Vec3::new(tgt.x, tgt.y, tgt.z + 1.0));
                    self.bastion_sync_context(global_state);
                }
            }
            // bastion: keep the derived input context synced into the window
            // fan-out filter (idempotent one-enum write; covers every camera-
            // mode transition path).
            self.bastion_sync_context(global_state);
            // bastion (B1.5): grab-drag update — keep the grabbed world point
            // locked under the cursor; when released, carry eased inertia.
            if self.bastion_overseer_active() {
                if let Some(grab) = self.bastion_grab {
                    if let Some(under_cursor) =
                        self.bastion_point_under_cursor(global_state, grab.plane_z)
                    {
                        let delta = (grab.anchor - under_cursor).with_z(0.0);
                        let delta = if delta.magnitude_squared()
                            > BASTION_GRAB_MAX_STEP * BASTION_GRAB_MAX_STEP
                        {
                            delta.normalized() * BASTION_GRAB_MAX_STEP
                        } else {
                            delta
                        };
                        let focus = self.scene.camera().get_focus_pos();
                        self.scene.camera_mut().force_focus_pos(focus + delta);
                        // Track a smoothed velocity so release feels thrown,
                        // but holding still before release stops dead.
                        if dt > f32::EPSILON {
                            let instantaneous = delta.xy() / dt;
                            let blend = (dt * 15.0).min(1.0);
                            self.bastion_pan_vel =
                                Lerp::lerp(self.bastion_pan_vel, instantaneous, blend);
                        }
                    }
                } else if self.bastion_pan_vel.magnitude_squared()
                    > BASTION_PAN_STOP * BASTION_PAN_STOP
                {
                    let focus = self.scene.camera().get_focus_pos();
                    self.scene
                        .camera_mut()
                        .force_focus_pos(focus + Vec3::from(self.bastion_pan_vel) * dt);
                    self.bastion_pan_vel *= (-BASTION_PAN_DAMP * dt).exp();
                } else {
                    self.bastion_pan_vel = Vec2::zero();
                }
            }
            // bastion: move the active Z-slice while PgUp/PgDn is held.
            if self.bastion_overseer_active()
                && let Some(slice) = self.scene.bastion_slice_z()
            {
                let slice_dir = self.inputs_state.contains(&GameInput::BastionSliceUp) as i32
                    - self.inputs_state.contains(&GameInput::BastionSliceDown) as i32;
                if slice_dir != 0 {
                    self.scene.set_bastion_slice_z(Some(
                        slice + slice_dir as f32 * BASTION_SLICE_RATE * dt,
                    ));
                }
            }
            // bastion (B2a/B3): interaction-surface upkeep — live paint/box
            // previews, designation-echo overlay, colonist markers, HUD state.
            if self.bastion_overseer_active() {
                if self.bastion_paint.is_some() {
                    self.bastion_paint_update(global_state);
                }
                if self.bastion_boxsel.is_some() {
                    self.bastion_boxsel_update(global_state);
                }
                self.bastion_sync_designations();
                self.bastion_sync_colonist_markers();
                self.bastion_sync_selection_rings();
                self.bastion_sync_inspector();
            } else if !self.bastion_colonist_markers.is_empty()
                || !self.bastion_selection_rings.is_empty()
            {
                // Left overseer: drop every colony debug shape (markers +
                // selection rings) so nothing lingers in third-person.
                let ids: Vec<DebugShapeId> = self
                    .bastion_colonist_markers
                    .drain()
                    .map(|(_, id)| id)
                    .chain(self.bastion_selection_rings.drain().map(|(_, id)| id))
                    .collect();
                for id in ids {
                    self.scene.debug.remove_shape(id);
                }
            }
            // TIME-CONTROLS: the HUD cluster mirrors the TRUTH each frame —
            // the singleplayer pause flag + the synced TimeScale resource —
            // so a pause/scale change from ANY path (buttons, hotkeys, chat
            // /time_scale, the ESC menu's auto-pause) moves the buttons.
            let sim_scale = self
                .client
                .borrow()
                .state()
                .ecs()
                .read_resource::<common::resources::TimeScale>()
                .0 as f32;
            self.hud.bastion_sync(
                self.bastion_overseer_active(),
                self.bastion_tools.tool,
                self.bastion_tools.god_mode,
                self.scene.bastion_slice_z(),
                self.bastion_tools.z_extent_label(),
                self.bastion_tools.flat_floor,
                global_state.paused(),
                sim_scale,
            );

            match self.scene.camera().get_mode() {
                CameraMode::FirstPerson | CameraMode::ThirdPerson => {
                    if mutable_viewpoint {
                        // Move the player character based on their walking direction.
                        // This could be different from the camera direction if free look is
                        // enabled.
                        self.inputs.move_dir =
                            self.walk_right_dir * axis_right + self.walk_forward_dir * axis_up;
                    }
                },
                CameraMode::Freefly => {
                    // Move the camera freely in 3d space. Apply acceleration so that
                    // the movement feels more natural and controlled.
                    const FREEFLY_SPEED: f32 = 50.0;
                    const FREEFLY_SPEED_BOOST: f32 = 5.0;

                    let forward = self.scene.camera().forward().with_z(0.0).normalized();
                    let right = self.scene.camera().right().with_z(0.0).normalized();
                    let up = Vec3::unit_z();
                    let up_axis = self.key_state.swim_up as i32 as f32
                        - self.key_state.swim_down as i32 as f32;

                    let dir = (right * axis_right + forward * axis_up + up * up_axis).normalized();

                    let speed = FREEFLY_SPEED
                        * if self.inputs_state.contains(&GameInput::SpectateSpeedBoost) {
                            FREEFLY_SPEED_BOOST
                        } else {
                            1.0
                        };

                    let pos = self.scene.camera().get_focus_pos();
                    self.scene
                        .camera_mut()
                        .set_focus_pos(pos + dir * dt * speed);

                    // Do not apply any movement to the player character
                    self.inputs.move_dir = Vec2::zero();
                },
                CameraMode::Overseer => {
                    // bastion: WASD pans the ground target across the map (XY
                    // only); speed scales with zoom so travel feels constant
                    // on screen.
                    let camera = self.scene.camera();
                    let dir = camera.right_xy() * axis_right + camera.forward_xy() * axis_up;
                    let dir = if dir.magnitude_squared() > 1.0 {
                        dir.normalized()
                    } else {
                        dir
                    };
                    let speed = camera.get_distance() * BASTION_PAN_FACTOR;
                    let pos = camera.get_focus_pos();
                    self.scene
                        .camera_mut()
                        .set_focus_pos(pos + Vec3::from(dir) * dt * speed);

                    // bastion: B&W2 ground glide. The focus rides the terrain
                    // surface — so the slice, reveal and relight all reference
                    // the actual ground instead of whatever altitude the
                    // focus happened to start at (the spectator spawns high in
                    // the air) — and the camera lifts so neither it nor its
                    // sight line to the focus ever dips under terrain.
                    {
                        let camera = self.scene.camera();
                        let tgt = camera.get_tgt_focus();
                        let dist = camera.get_tgt_dist();
                        let fwd = camera.forward();
                        let mut focus = tgt;
                        {
                            let client = self.client.borrow();
                            let terrain = client.state().terrain();
                            if let Some(g) = bastion::ground_z(&terrain, tgt.xy(), tgt.z) {
                                focus.z = g + 1.0;
                            }
                            let cam = focus - fwd * dist;
                            let mut lift = 0.0_f32;
                            for t in [0.5, 1.0] {
                                let p = focus + (cam - focus) * t;
                                if let Some(g) = bastion::ground_z(&terrain, p.xy(), p.z) {
                                    lift = lift.max(g + BASTION_CAM_MARGIN - p.z);
                                }
                            }
                            focus.z += lift.max(0.0);
                        }
                        if (focus.z - tgt.z).abs() > 0.01 {
                            self.scene.camera_mut().set_focus_pos(focus);
                        }
                    }

                    // Do not apply any movement to the player character.
                    self.inputs.move_dir = Vec2::zero();
                },
            };

            let mut outcomes = Vec::new();

            // Runs if either in a multiplayer server or the singleplayer server is unpaused
            if !global_state.paused() {
                // Perform an in-game tick.
                match self.tick(global_state.clock.game_dt(), global_state, &mut outcomes) {
                    Ok(TickAction::Continue) => {}, // Do nothing
                    Ok(TickAction::Disconnect) => return PlayStateResult::Pop, // Go to main menu
                    Err(Error::ClientError(error)) => {
                        error!("[session] Failed to tick the scene: {:?}", error);
                        global_state.info_message =
                            Some(get_client_msg_error(error, None, &global_state.i18n.read()));

                        return PlayStateResult::Pop;
                    },
                    Err(err) => {
                        global_state.info_message = Some(
                            global_state
                                .i18n
                                .read()
                                .get_msg("common-connection_lost")
                                .into_owned(),
                        );
                        error!("[session] Failed to tick the scene: {:?}", err);

                        return PlayStateResult::Pop;
                    },
                }
            }

            if self.walking_speed {
                self.key_state.speed_mul = global_state.settings.gameplay.walking_speed;
            } else {
                self.key_state.speed_mul = 1.0;
            }

            // Recompute dependents just in case some input modified the camera
            self.scene
                .camera_mut()
                .compute_dependents(&self.client.borrow().state().terrain());

            // Generate debug info, if needed
            // (it iterates through enough data that we might
            // as well avoid it unless we need it).
            let debug_info = global_state.settings.interface.toggle_debug.then(|| {
                let client = self.client.borrow();
                let ecs = client.state().ecs();
                let client_entity = client.entity();
                let coordinates = ecs.read_storage::<Pos>().get(viewpoint_entity).cloned();
                let velocity = ecs.read_storage::<Vel>().get(viewpoint_entity).cloned();
                let ori = ecs
                    .read_storage::<comp::Ori>()
                    .get(viewpoint_entity)
                    .cloned();
                // NOTE: at the time of writing, it will always output default
                // look_dir in Specate mode, because Controller isn't synced
                let look_dir = if viewpoint_entity == client_entity {
                    self.inputs.look_dir
                } else {
                    ecs.read_storage::<comp::Controller>()
                        .get(viewpoint_entity)
                        .map(|c| c.inputs.look_dir)
                        .unwrap_or_default()
                };
                let in_fluid = ecs
                    .read_storage::<comp::PhysicsState>()
                    .get(viewpoint_entity)
                    .and_then(|state| state.in_fluid);
                let character_state = ecs
                    .read_storage::<comp::CharacterState>()
                    .get(viewpoint_entity)
                    .cloned();

                DebugInfo {
                    tps: global_state.clock.stats().average_tps,
                    frame_time: global_state.clock.stats().average_busy_dt,
                    frame_variance: global_state.clock.stats().average_variance,
                    ping_ms: self.client.borrow().get_ping_ms_rolling_avg(),
                    coordinates,
                    velocity,
                    ori,
                    look_dir,
                    character_state,
                    in_fluid,
                    num_chunks: self.scene.terrain().chunk_count() as u32,
                    num_lights: self.scene.lights().len() as u32,
                    num_visible_chunks: self.scene.terrain().visible_chunk_count() as u32,
                    num_shadow_chunks: self.scene.terrain().shadow_chunk_count() as u32,
                    num_figures: self.scene.figure_mgr().figure_count() as u32,
                    num_figures_visible: self.scene.figure_mgr().figure_count_visible() as u32,
                    num_particles: self.scene.particle_mgr().particle_count() as u32,
                    num_particles_visible: self.scene.particle_mgr().particle_count_visible()
                        as u32,
                    current_track: self.scene.music_mgr().current_track(),
                    current_artist: self.scene.music_mgr().current_artist(),
                    active_channels: global_state.audio.get_num_active_channels(),
                    audio_cpu_usage: global_state.audio.get_cpu_usage(),
                }
            });

            let inverted_interactable_map = self.interactables.inverted_map();

            // Extract HUD events ensuring the client borrow gets dropped.
            let mut hud_events = self.hud.maintain(
                &self.client.borrow(),
                global_state,
                &debug_info,
                self.scene.camera(),
                global_state.clock.real_dt(),
                HudInfo {
                    is_aiming,
                    active_mine_tool,
                    is_first_person: matches!(
                        self.scene.camera().get_mode(),
                        camera::CameraMode::FirstPerson
                    ),
                    viewpoint_entity,
                    mutable_viewpoint,
                    target_entity: self.target_entity,
                    selected_entity: self.selected_entity,
                    persistence_load_error: self.metadata.skill_set_persistence_load_error,
                    key_state: &self.key_state,
                },
                inverted_interactable_map,
            );

            // Maintain egui (debug interface)
            #[cfg(feature = "egui-ui")]
            if global_state.settings.interface.egui_enabled() {
                let settings_change = global_state.egui_state.maintain(
                    &mut self.client.borrow_mut(),
                    &mut self.scene,
                    global_state.window.window(),
                    debug_info.map(|debug_info| EguiDebugInfo {
                        frame_time: debug_info.frame_time,
                        ping_ms: debug_info.ping_ms,
                    }),
                    &global_state.settings,
                );

                if let Some(settings_change) = settings_change {
                    settings_change.process(global_state, self);
                }
            }

            // Look for changes in the localization files
            if global_state.i18n.reloaded() {
                hud_events.push(HudEvent::SettingsChange(
                    ChangeLanguage(Box::new(global_state.i18n.read().metadata().clone())).into(),
                ));
            }

            let mut has_repaired = false;
            let sfx_triggers = self.scene.sfx_mgr.triggers.read();
            // Maintain the UI.
            for event in hud_events {
                match event {
                    HudEvent::SendMessage(msg) => {
                        // TODO: Handle result
                        self.client.borrow_mut().send_chat(msg);
                    },
                    // bastion (B2a): interaction-surface HUD events.
                    HudEvent::BastionSelectTool(tool) => {
                        self.bastion_set_tool(tool);
                    },
                    HudEvent::BastionStepZExtent(steps) => {
                        // B5.6b-2: the tool panel's precision stepper steps
                        // the same depth field as scroll-while-painting.
                        self.bastion_tools.step_z_extent(steps);
                    },
                    HudEvent::BastionToggleFlatFloor => {
                        // B5.6b-2.1: slope-following ↔ flat-floor digging.
                        self.bastion_tools.flat_floor = !self.bastion_tools.flat_floor;
                    },
                    HudEvent::BastionSetSimSpeed(speed) => {
                        // TIME-CONTROLS: the HUD cluster's click — same
                        // setter the hotkeys use.
                        self.bastion_set_sim_speed(global_state, speed);
                    },
                    HudEvent::BastionToggleGodMode => {
                        self.bastion_tools.god_mode = self.bastion_tools.god_mode.toggled();
                    },
                    // bastion (B-MAP1): minimap navigation. Only XY moves —
                    // the overseer focus glide re-rides the terrain surface
                    // (ground_z) on its own, so z corrects next frame.
                    HudEvent::BastionMinimapJump(wpos2) => {
                        if self.bastion_overseer_active() {
                            let camera = self.scene.camera_mut();
                            let f = camera.get_tgt_focus();
                            camera.set_focus_pos(Vec3::new(wpos2.x, wpos2.y, f.z));
                        }
                    },
                    HudEvent::BastionMinimapPan(delta) => {
                        if self.bastion_overseer_active() {
                            let camera = self.scene.camera_mut();
                            let f = camera.get_tgt_focus();
                            camera.set_focus_pos(Vec3::new(f.x + delta.x, f.y + delta.y, f.z));
                        }
                    },
                    HudEvent::BastionRadialPick {
                        action,
                        target,
                        point,
                    } => {
                        // The target-restriction hook (§3c) — permissive stub
                        // until B2b gives God mode teeth.
                        if bastion::tools::target_allowed(self.bastion_tools.god_mode, None) {
                            let mut client = self.client.borrow_mut();
                            match action {
                                // B3: founding the colony is a real verb now.
                                crate::hud::bastion::RadialAction::Verb(
                                    common::bastion::ContextVerb::FoundColony,
                                ) => {
                                    client.bastion_spawn_colony(point, 6);
                                },
                                crate::hud::bastion::RadialAction::Verb(verb) => {
                                    client.bastion_context_action(target, verb);
                                },
                                crate::hud::bastion::RadialAction::Influence(kind) => {
                                    client.bastion_apply_influence(point, kind);
                                },
                                // B5.5: delete every painted rect containing
                                // the clicked block — resolved client-side
                                // from the echoed designation list, one
                                // cancel per rect.
                                crate::hud::bastion::RadialAction::DeleteZone => {
                                    if let common::bastion::ContextTarget::Block(block) = target {
                                        let rects: Vec<common::bastion::Region> = client
                                            .bastion_designations()
                                            .iter()
                                            .filter(|(r, _, _)| r.contains_point_xy(block))
                                            .map(|(r, _, _)| *r)
                                            .collect();
                                        for r in rects {
                                            client.bastion_cancel_designation(r);
                                        }
                                    }
                                },
                            }
                        }
                    },
                    HudEvent::SendCommand(name, args) => {
                        match run_command(self, global_state, &name, args) {
                            Ok(Some(info)) => {
                                self.hud.new_message(ChatType::CommandInfo.into_msg(info))
                            },
                            Ok(None) => {}, // Server will provide an info message
                            Err(error) => {
                                self.hud.new_message(ChatType::CommandError.into_msg(error))
                            },
                        };
                    },
                    HudEvent::CharacterSelection => {
                        global_state.audio.stop_all_music();
                        global_state.audio.stop_all_ambience();
                        global_state.audio.stop_all_sfx();
                        self.client.borrow_mut().request_remove_character()
                    },
                    HudEvent::Logout => {
                        self.client.borrow_mut().logout();
                        // Stop all sounds
                        // TODO: Abstract this behavior to all instances of PlayStateResult::Pop
                        // somehow
                        global_state.audio.stop_all_ambience();
                        global_state.audio.stop_all_sfx();
                        return PlayStateResult::Pop;
                    },
                    HudEvent::Quit => {
                        return PlayStateResult::Shutdown;
                    },

                    HudEvent::RemoveBuff(buff_id) => {
                        self.client.borrow_mut().remove_buff(buff_id);
                    },
                    HudEvent::LeaveStance => self.client.borrow_mut().leave_stance(),
                    HudEvent::UnlockSkill(skill) => {
                        self.client.borrow_mut().unlock_skill(skill);
                    },
                    HudEvent::UseSlot {
                        slot,
                        bypass_dialog,
                    } => {
                        let mut move_allowed = true;

                        if !bypass_dialog
                            && let Some(inventory) = self
                                .client
                                .borrow()
                                .state()
                                .ecs()
                                .read_storage::<comp::Inventory>()
                                .get(self.client.borrow().entity())
                        {
                            match slot {
                                Slot::Inventory(inv_slot) => {
                                    let slot_deficit = inventory.free_after_equip(inv_slot);
                                    if slot_deficit < 0 {
                                        self.hud.set_prompt_dialog(PromptDialogSettings::new(
                                            global_state.i18n.read().get_content(
                                                &Content::localized_with_args(
                                                    "hud-bag-use_slot_equip_drop_items",
                                                    [(
                                                        "slot_deficit",
                                                        slot_deficit.unsigned_abs() as u64,
                                                    )],
                                                ),
                                            ),
                                            HudEvent::UseSlot {
                                                slot,
                                                bypass_dialog: true,
                                            },
                                            None,
                                        ));
                                        move_allowed = false;
                                    }
                                },
                                Slot::Equip(equip_slot) => {
                                    // Ensure there is a free slot that is not provided by the
                                    // item being unequipped
                                    let free_slots =
                                        inventory.free_slots_minus_equipped_item(equip_slot);
                                    if free_slots > 0 {
                                        let slot_deficit = inventory.free_after_unequip(equip_slot);
                                        if slot_deficit < 0 {
                                            self.hud.set_prompt_dialog(PromptDialogSettings::new(
                                                global_state.i18n.read().get_content(
                                                    &Content::localized_with_args(
                                                        "hud-bag-use_slot_unequip_drop_items",
                                                        [(
                                                            "slot_deficit",
                                                            slot_deficit.unsigned_abs() as u64,
                                                        )],
                                                    ),
                                                ),
                                                HudEvent::UseSlot {
                                                    slot,
                                                    bypass_dialog: true,
                                                },
                                                None,
                                            ));
                                            move_allowed = false;
                                        }
                                    } else {
                                        move_allowed = false;
                                    }
                                },
                                Slot::Overflow(_) => {},
                            }
                        };

                        if move_allowed {
                            self.client.borrow_mut().use_slot(slot);
                        }
                    },
                    HudEvent::SwapEquippedWeapons => {
                        self.client.borrow_mut().swap_loadout();
                    },
                    HudEvent::SwapSlots {
                        slot_a,
                        slot_b,
                        bypass_dialog,
                    } => {
                        let mut move_allowed = true;
                        if !bypass_dialog
                            && let Some(inventory) = self
                                .client
                                .borrow()
                                .state()
                                .ecs()
                                .read_storage::<comp::Inventory>()
                                .get(self.client.borrow().entity())
                        {
                            match (slot_a, slot_b) {
                                (Slot::Inventory(inv_slot), Slot::Equip(equip_slot))
                                | (Slot::Equip(equip_slot), Slot::Inventory(inv_slot)) => {
                                    if !inventory.can_swap(inv_slot, equip_slot) {
                                        move_allowed = false;
                                    } else {
                                        let slot_deficit =
                                            inventory.free_after_swap(equip_slot, inv_slot);
                                        if slot_deficit < 0 {
                                            self.hud.set_prompt_dialog(PromptDialogSettings::new(
                                                global_state.i18n.read().get_content(
                                                    &Content::localized_with_args(
                                                        "hud-bag-swap_slots_drop_items",
                                                        [(
                                                            "slot_deficit",
                                                            slot_deficit.unsigned_abs() as u64,
                                                        )],
                                                    ),
                                                ),
                                                HudEvent::SwapSlots {
                                                    slot_a,
                                                    slot_b,
                                                    bypass_dialog: true,
                                                },
                                                None,
                                            ));
                                            move_allowed = false;
                                        }
                                    }
                                },
                                _ => {},
                            }
                        }
                        if move_allowed {
                            self.client.borrow_mut().swap_slots(slot_a, slot_b);
                        }
                    },
                    HudEvent::SelectExpBar(skillgroup) => {
                        global_state.settings.interface.xp_bar_skillgroup = skillgroup;
                    },
                    HudEvent::SplitSwapSlots {
                        slot_a,
                        slot_b,
                        bypass_dialog,
                    } => {
                        let mut move_allowed = true;
                        if !bypass_dialog
                            && let Some(inventory) = self
                                .client
                                .borrow()
                                .state()
                                .ecs()
                                .read_storage::<comp::Inventory>()
                                .get(self.client.borrow().entity())
                        {
                            match (slot_a, slot_b) {
                                (Slot::Inventory(inv_slot), Slot::Equip(equip_slot))
                                | (Slot::Equip(equip_slot), Slot::Inventory(inv_slot)) => {
                                    if !inventory.can_swap(inv_slot, equip_slot) {
                                        move_allowed = false;
                                    } else {
                                        let slot_deficit =
                                            inventory.free_after_swap(equip_slot, inv_slot);
                                        if slot_deficit < 0 {
                                            self.hud.set_prompt_dialog(PromptDialogSettings::new(
                                                global_state.i18n.read().get_content(
                                                    &Content::localized_with_args(
                                                        "hud-bag-split_swap_slots_drop_items",
                                                        [(
                                                            "slot_deficit",
                                                            slot_deficit.unsigned_abs() as u64,
                                                        )],
                                                    ),
                                                ),
                                                HudEvent::SwapSlots {
                                                    slot_a,
                                                    slot_b,
                                                    bypass_dialog: true,
                                                },
                                                None,
                                            ));
                                            move_allowed = false;
                                        }
                                    }
                                },
                                _ => {},
                            }
                        };
                        if move_allowed {
                            self.client.borrow_mut().split_swap_slots(slot_a, slot_b);
                        }
                    },
                    HudEvent::DropSlot(x) => {
                        let mut client = self.client.borrow_mut();
                        client.drop_slot(x);
                        if let Slot::Equip(EquipSlot::Lantern) = x {
                            client.disable_lantern();
                        }
                    },
                    HudEvent::SplitDropSlot(x) => {
                        let mut client = self.client.borrow_mut();
                        client.split_drop_slot(x);
                        if let Slot::Equip(EquipSlot::Lantern) = x {
                            client.disable_lantern();
                        }
                    },
                    HudEvent::SortInventory(sort_order) => {
                        self.client.borrow_mut().sort_inventory(sort_order);
                    },
                    HudEvent::ChangeHotbarState(state) => {
                        let client = self.client.borrow();

                        let server_name = &client.server_info().name;
                        // If we are changing the hotbar state this CANNOT be None.
                        let character_id = match client.presence().unwrap() {
                            PresenceKind::Character(id) => Some(id),
                            PresenceKind::LoadingCharacter(id) => Some(id),
                            PresenceKind::Spectator => {
                                unreachable!("HUD adaption in Spectator mode!")
                            },
                            PresenceKind::Possessor => None,
                        };

                        // Get or update the ServerProfile.
                        global_state.profile.set_hotbar_slots(
                            server_name,
                            character_id,
                            state.slots,
                        );

                        global_state
                            .profile
                            .save_to_file_warn(&global_state.config_dir);

                        info!("Event! -> ChangedHotbarState")
                    },
                    HudEvent::TradeAction(action) => {
                        self.client.borrow_mut().perform_trade_action(action);
                    },
                    HudEvent::Ability { idx, state } => {
                        self.client.borrow_mut().handle_input(
                            InputKind::Ability(idx),
                            state,
                            default_select_pos,
                            self.target_entity,
                        );
                    },

                    HudEvent::RequestSiteInfo(id) => {
                        self.client.borrow_mut().request_site_economy(id);
                    },

                    HudEvent::CraftRecipe {
                        recipe_name: recipe,
                        craft_sprite,
                        amount,
                    } => {
                        let slots = {
                            let client = self.client.borrow();

                            if let Some(inventory) = client
                                .state()
                                .ecs()
                                .read_storage::<comp::Inventory>()
                                .get(client.entity())
                            {
                                let rbm =
                                    client.state().ecs().read_resource::<RecipeBookManifest>();
                                if let Some(recipe) = inventory.get_recipe(&recipe, &rbm) {
                                    recipe.inventory_contains_ingredients(inventory, 1).ok()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };
                        if let Some(slots) = slots {
                            self.client.borrow_mut().craft_recipe(
                                &recipe,
                                slots,
                                craft_sprite,
                                amount,
                            );
                        }
                    },

                    HudEvent::CraftModularWeapon {
                        primary_slot,
                        secondary_slot,
                        craft_sprite,
                    } => {
                        self.client.borrow_mut().craft_modular_weapon(
                            primary_slot,
                            secondary_slot,
                            craft_sprite,
                        );
                    },

                    HudEvent::CraftModularWeaponComponent {
                        toolkind,
                        material,
                        modifier,
                        craft_sprite,
                    } => {
                        let additional_slots = {
                            let client = self.client.borrow();
                            let item_id = |slot| {
                                client
                                    .inventories()
                                    .get(client.entity())
                                    .and_then(|inv| inv.get(slot))
                                    .and_then(|item| {
                                        item.item_definition_id().itemdef_id().map(String::from)
                                    })
                            };
                            if let Some(material_id) = item_id(material) {
                                let key = recipe::ComponentKey {
                                    toolkind,
                                    material: material_id,
                                    modifier: modifier.and_then(item_id),
                                };
                                if let Some(recipe) = client.component_recipe_book().get(&key) {
                                    client.inventories().get(client.entity()).and_then(|inv| {
                                        recipe.inventory_contains_additional_ingredients(inv).ok()
                                    })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };
                        if let Some(additional_slots) = additional_slots {
                            self.client.borrow_mut().craft_modular_weapon_component(
                                toolkind,
                                material,
                                modifier,
                                additional_slots,
                                craft_sprite,
                            );
                        }
                    },
                    HudEvent::SalvageItem { slot, salvage_pos } => {
                        self.client.borrow_mut().salvage_item(slot, salvage_pos);
                    },
                    HudEvent::RepairItem { item, sprite_pos } => {
                        if !has_repaired {
                            let sfx_trigger_item = sfx_triggers
                                .0
                                .get_key_value(&SfxEvent::from(&InventoryUpdateEvent::Craft));
                            global_state.audio.emit_ui_sfx(sfx_trigger_item, None, None);
                            has_repaired = true
                        };
                        self.client.borrow_mut().repair_item(item, sprite_pos);
                    },
                    HudEvent::InviteMember(uid) => {
                        self.client.borrow_mut().send_invite(uid, InviteKind::Group);
                    },
                    HudEvent::AcceptInvite => {
                        self.client.borrow_mut().accept_invite();
                    },
                    HudEvent::DeclineInvite => {
                        self.client.borrow_mut().decline_invite();
                    },
                    HudEvent::KickMember(uid) => {
                        self.client.borrow_mut().kick_from_group(uid);
                    },
                    HudEvent::LeaveGroup => {
                        self.client.borrow_mut().leave_group();
                    },
                    HudEvent::AssignLeader(uid) => {
                        self.client.borrow_mut().assign_group_leader(uid);
                    },
                    HudEvent::ChangeAbility(slot, new_ability) => {
                        self.client.borrow_mut().change_ability(slot, new_ability);
                    },
                    HudEvent::SettingsChange(settings_change) => {
                        settings_change.process(global_state, self);
                    },
                    HudEvent::AcknowledgePersistenceLoadError => {
                        self.metadata.skill_set_persistence_load_error = None;
                    },
                    HudEvent::MapMarkerEvent(event) => {
                        self.client.borrow_mut().map_marker_event(event);
                    },
                    HudEvent::Dialogue(target, dialogue) => {
                        self.client.borrow_mut().perform_dialogue(target, dialogue);
                    },
                    HudEvent::SetBattleMode(mode) => {
                        self.client.borrow_mut().set_battle_mode(mode);
                    },
                }
            }

            {
                let client = self.client.borrow();
                let scene_data = SceneData {
                    client: &client,
                    state: client.state(),
                    viewpoint_entity,
                    mutable_viewpoint: mutable_viewpoint || self.free_look,
                    // Only highlight if interactable
                    target_entities: &self.interactables.entities,
                    loaded_distance: client.loaded_distance(),
                    terrain_view_distance: client.view_distance().unwrap_or(1),
                    entity_view_distance: client
                        .view_distance()
                        .unwrap_or(1)
                        .min(global_state.settings.graphics.entity_view_distance),
                    tick: client.get_tick(),
                    gamma: global_state.settings.graphics.gamma,
                    exposure: global_state.settings.graphics.exposure,
                    ambiance: global_state.settings.graphics.ambiance,
                    mouse_smoothing: global_state.settings.gameplay.smooth_pan_enable,
                    sprite_render_distance: global_state.settings.graphics.sprite_render_distance
                        as f32,
                    particles_enabled: global_state.settings.graphics.particles_enabled,
                    weapon_trails_enabled: global_state.settings.graphics.weapon_trails_enabled,
                    flashing_lights_enabled: global_state
                        .settings
                        .graphics
                        .render_mode
                        .flashing_lights_enabled,
                    figure_lod_render_distance: global_state
                        .settings
                        .graphics
                        .figure_lod_render_distance
                        as f32,
                    is_aiming,
                    interpolated_time_of_day: self.scene.interpolated_time_of_day,
                    wind_vel: self.scene.wind_vel,
                };

                // Runs if either in a multiplayer server or the singleplayer server is unpaused
                if !global_state.paused() {
                    self.scene.maintain(
                        global_state.window.renderer_mut(),
                        &mut global_state.audio,
                        &scene_data,
                        &client,
                        &global_state.settings,
                        global_state.settings.interface.minimap_face_north,
                    );

                    // Process outcomes from client
                    for outcome in outcomes {
                        self.scene
                            .handle_outcome(&outcome, &scene_data, &mut global_state.audio);
                        self.hud.handle_outcome(&outcome, &scene_data, global_state);
                    }
                }
            }

            // Clean things up after the tick.
            self.cleanup();

            PlayStateResult::Continue
        } else if client_registered && client_presence.is_none() {
            // If the client cannot enter the game but spectate, pop the play state instead
            // of going back to character selection.
            if client_type.can_spectate() && !client_type.can_enter_character() {
                // Go back to the main menu state
                PlayStateResult::Pop
            } else {
                PlayStateResult::Switch(Box::new(CharSelectionState::new(
                    global_state,
                    Rc::clone(&self.client),
                    Rc::clone(&self.hud.persisted_state),
                )))
            }
        } else {
            error!("Client not in the expected state, exiting session play state");
            PlayStateResult::Pop
        }
    }

    fn name(&self) -> &'static str { "Session" }

    fn capped_fps(&self) -> bool { false }

    fn globals_bind_group(&self) -> &GlobalsBindGroup { self.scene.global_bind_group() }

    /// Render the session to the screen.
    ///
    /// This method should be called once per frame.
    fn render(&self, drawer: &mut Drawer<'_>, settings: &Settings) {
        span!(_guard, "render", "<Session as PlayState>::render");

        let client = self.client.borrow();

        let (viewpoint_entity, mutable_viewpoint) = self.viewpoint_entity();

        let scene_data = SceneData {
            client: &client,
            state: client.state(),
            viewpoint_entity,
            mutable_viewpoint,
            // Only highlight if interactable
            target_entities: &self.interactables.entities,
            loaded_distance: client.loaded_distance(),
            terrain_view_distance: client.view_distance().unwrap_or(1),
            entity_view_distance: client
                .view_distance()
                .unwrap_or(1)
                .min(settings.graphics.entity_view_distance),
            tick: client.get_tick(),
            gamma: settings.graphics.gamma,
            exposure: settings.graphics.exposure,
            ambiance: settings.graphics.ambiance,
            mouse_smoothing: settings.gameplay.smooth_pan_enable,
            sprite_render_distance: settings.graphics.sprite_render_distance as f32,
            figure_lod_render_distance: settings.graphics.figure_lod_render_distance as f32,
            particles_enabled: settings.graphics.particles_enabled,
            weapon_trails_enabled: settings.graphics.weapon_trails_enabled,
            flashing_lights_enabled: settings.graphics.render_mode.flashing_lights_enabled,
            is_aiming: self.is_aiming,
            interpolated_time_of_day: self.scene.interpolated_time_of_day,
            wind_vel: self.scene.wind_vel,
        };

        // Render world
        self.scene.render(
            drawer,
            client.state(),
            viewpoint_entity,
            client.get_tick(),
            &scene_data,
        );

        if let Some(mut volumetric_pass) = drawer.volumetric_pass() {
            // Clouds
            prof_span!("clouds");
            volumetric_pass.draw_clouds();
        }
        if let Some(mut transparent_pass) = drawer.transparent_pass() {
            // Trails
            prof_span!("trails");
            if let Some(mut trail_drawer) = transparent_pass.draw_trails() {
                self.scene
                    .trail_mgr()
                    .render(&mut trail_drawer, &scene_data);
            }
        }
        // Bloom (call does nothing if bloom is off)
        {
            prof_span!("bloom");
            drawer.run_bloom_passes()
        }
        // PostProcess and UI
        {
            prof_span!("post-process and ui");
            let mut third_pass = drawer.third_pass();
            third_pass.draw_postprocess();
            // Draw the UI to the screen
            if let Some(mut ui_drawer) = third_pass.draw_ui() {
                self.hud.render(&mut ui_drawer);
            }; // Note: this semicolon is needed for the third_pass borrow to be
            // dropped before it's lifetime ends
        }
    }

    fn egui_enabled(&self) -> bool { true }
}

// TODO: Can probably be exported in some way for AI, somehow
fn auto_glide(
    fluid: Fluid,
    vel: Vel,
    free_look: bool,
    dir_forward_xy: Vec2<f32>,
    dir_right: Vec3<f32>,
) -> Option<Dir> {
    let Vel(rel_flow) = fluid.relative_flow(&vel);

    let is_wind_downwards = rel_flow.z.is_sign_negative();

    let dir = if free_look {
        if is_wind_downwards {
            Vec3::from(-rel_flow.xy())
        } else {
            -rel_flow
        }
    } else if is_wind_downwards {
        dir_forward_xy.into()
    } else {
        let windwards = rel_flow * dir_forward_xy.dot(rel_flow.xy()).signum();
        Plane::from(Dir::new(dir_right)).projection(windwards)
    };

    Dir::from_unnormalized(dir)
}
