# Bastion B1 — Findings (overseer camera + Z-slice, real symbols)

Verified against `bastion/main` (baseline `bastion-baseline`, see `BASELINE.md`). Line numbers
drift; symbols are the anchor. Read together with `docs/BASTION_B0_FINDINGS.md`.

## 1. Camera architecture (`voxygen/src/scene/camera.rs`)

- `CameraMode { FirstPerson = 0, ThirdPerson = 1, Freefly = 2 }` — voxygen-local enum, **not**
  serialized anywhere, discriminant is passed to shaders as `Globals.cam_mode: u32`. We add
  `Overseer = 3` (bastion).
- `Camera` state: `focus`/`tgt_focus: Vec3` (interpolated look-at point), `ori: Vec3`
  (x=yaw, y=pitch clamped ±π/2, z=roll), `dist`/`tgt_dist` (boom length), `fov`, `aspect`, `mode`.
- `Camera::compute_dependents_full` → `compute_dependents_near/far` cast **terrain rays** to pull
  the camera in front of occluding voxels (`terrain.ray(...)` at camera.rs:407/453). Overseer must
  **bypass** this (a god camera slicing underground must not collide) → direct
  `compute_dependents_helper(self.dist)`.
- `compute_dependents_helper` (camera.rs:472) builds:
  - `view_mat` = translation(-dist·ẑ) · rot_z/x/y(ori) · rot(−90° about x) · translation(−focus.fract())
    — world is Z-up, view space is wgpu-style; the focus *integer* part goes to the shader as
    `focus_off` instead (precision trick — see §3).
  - `proj_mat` = `perspective_rh_zo_general(fov, aspect, 1/FAR, 1/NEAR)` — **reversed depth**
    (1→0); `proj_mat_treeculler` = same non-reversed (treeculler can't handle inverted planes).
  - **Overseer swaps only this projection** for an orthographic pair:
    half-height `v = dist · tan(fov/2)` (so `dist` keeps meaning "zoom" continuously),
    `Mat4::orthographic_rh_zo(FrustumPlanes { left/right: ∓v·aspect, bottom/top: ∓v, near, far })`,
    with near/far swapped for the reversed-depth main matrix.
- Exhaustive matches that need an `Overseer` arm (compiler enforces):
  `interp_time` (camera.rs:649), `set_mode` (:720), `next_mode` (:750 — Overseer must NOT join the
  vanilla cycle), `Camera::new` dist init (:320), plus `zoom_by` (:537, ThirdPerson-only guard —
  add Overseer arm with ortho clamp) and `zoom_switch` (:551 — must ignore Overseer; gate at the
  caller instead).

## 2. Where the camera feeds the renderer

- `Scene::maintain` (`voxygen/src/scene/mod.rs`): picks `viewpoint_pos` per mode (:721-751 —
  Freefly arm is the detached-camera precedent: it *doesn't* set focus from the entity; the session
  moves focus directly), calls `camera.compute_dependents`, then builds the uniform block at
  **`Globals::new(...)` (scene/mod.rs:975-1010)**. Second call site: `scene/simple.rs:265`
  (char-select scene — pass the disabled sentinel there).
- `Globals` struct: `voxygen/src/render/pipelines/mod.rs:36-79`; GLSL mirror:
  `assets/voxygen/shaders/include/globals.glsl:4-36`. **Key layout fact:** the Rust struct ends
  `..., screen_fade: f32, globals_dummy: f32` (pad to 16 B); the GLSL block ends at `screen_fade`.
  So `globals_dummy` → `bastion_slice_z` + appending `float bastion_slice_z;` after `screen_fade`
  in GLSL is a **layout-neutral** change (std140 pad slot becomes a real field). Disabled sentinel:
  `f32::MAX` (compare `f_pos.z + focus_off.z > bastion_slice_z` is then always false).
- `cam_mode` already reaches shaders (figure-frag.glsl:95,105 branch on it for first/third-person
  player rendering; Overseer=3 falls outside both branches, which is correct for a detached view).

## 3. Terrain shader / slice filter

- Shaders are **GLSL assets** compiled at runtime via shaderc (`assets/voxygen/shaders/`), hot-
  reloadable; all include `globals.glsl`.
- World-space height in fragment shaders is `f_pos.z + focus_off.z` (positions are focus-offset
  relative — see the commented discard at terrain-frag.glsl:76 which demonstrates exactly this).
- Slice targets (v1): `terrain-frag.glsl`, `sprite-frag.glsl`, `fluid-frag/cheap.glsl`,
  `fluid-frag/shiny.glsl` — one `discard` line near the top of `main`. Explicitly **not** sliced in
  v1: `figure-*` (entities), `lod-terrain-*` (distant terrain), `*-shadow-*` (shadow casters —
  clipped geometry still casts shadows; visual quirk, note in docs), `particle`, `rope`, `debug`.

## 4. Input & mode plumbing

- `GameInput` enum: `voxygen/src/game_input.rs` (strum `EnumIter`/`AsRefStr`;
  `get_localization_key` = variant name; missing i18n just shows the raw key — safe to add
  variants). Defaults: `ControlSettings::default_binding` (`voxygen/src/settings/control.rs`).
  Multiple GameInputs may share a physical key (dispatch fans out via `inverse_keybindings`;
  `GameInput::can_share_bindings` only drives settings-UI conflict warnings).
- Key availability: `Q` = Slot10, `E` = Interact (we *share* them — overseer consumes, vanilla
  modes ignore ours), `PageUp`/`PageDown` unbound, `F9` unbound (F7 egui, F8 mute, F10 settings).
- Session-side camera control: big `GameInput` match in `SessionState::handle_events`
  (`voxygen/src/session/mod.rs:1275` CycleCamera etc.); per-frame movement dispatch matches
  camera mode at session/mod.rs:1579-1618 — **Freefly arm (:1589) is the template for overseer
  panning** (`key_state` dir → `camera.set_focus_pos`). Zoom/scroll arrives at
  `Scene::handle_input_event` (`scene/mod.rs:433 Event::Zoom` → `zoom_switch`; overseer branches
  to its own clamped ortho zoom), mouse-pan at :428 (`Event::CursorPan` → suppressed in overseer;
  pitch is fixed, yaw only via 90° steps).
- Mode entry: `voxygen/src/cli.rs::Args` (clap; airshipper only relies on `--server` and
  `ListWgpuBackends` — additive flags are safe) → stored as `GlobalState.args`
  (`voxygen/src/lib.rs:81`) → reachable from `SessionState`. Launch flag: `--bastion-overseer`.
  In-session toggle (only active when the flag was passed): `GameInput::BastionToggleOverseer`,
  default `F9`.

## 5. Risks / quirks to watch (evaluated during this block)

1. **Fog vs. camera distance:** fog is computed from `cam_pos` distance; a long ortho boom (big
   `dist`) pushes the whole ground plane toward the fog range. Mitigation: moderate zoom clamp
   (`dist` ≈ 24..1024) + oblique pitch; revisit if visuals demand a fog offset uniform.
2. **Shadows under ortho:** directed-shadow fitting (scene/mod.rs:1078+) derives from the camera
   frustum in clip space; the math is generic over invertible projections but was only ever fed
   perspective ones. If shadows glitch in overseer, the v1 answer is "known quirk, disable shadows"
   not a shadow-pipeline rework.
3. **Chunk loading follows the player entity, not the camera.** Panning past the loaded radius
   shows LoD terrain (vanilla behavior for Freefly spectate too). Fine for B1; B2+ can reuse the
   spectator pathway (`client.spectate_position(pos)` at client/src/lib.rs:1950 writes the
   entity `Pos` directly when presence is Spectator) to stream chunks under the camera.
4. **Depth precision:** ortho depth is linear; NEAR/FAR = 0.0625/524288 gives ~6 cm worst-case
   steps — acceptable; tighten the overseer far plane if z-fighting appears.
5. `img_export.rs` and `scene/simple.rs` construct cameras with fixed vanilla modes — untouched.

## 6. Corrections vs. the block prompt

- There is no `PlayState::render(&self, drawer, settings)`-shaped seam we need; everything routes
  through `Scene::maintain` + `Globals`, which is cleaner than expected.
- "Terrain shader uniform" is not a terrain-pipeline-local uniform: the right place is the shared
  `u_globals` block (set 0, binding 0) because sprites and fluids must slice too, and the dummy pad
  slot makes it free.
