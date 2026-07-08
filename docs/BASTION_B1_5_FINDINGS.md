# Bastion B1.5 — Findings (input contexts, B&W2 camera feel, streaming)

Verified against `bastion/main` @ B1 complete. Read with `BASTION_B1_FINDINGS.md` (B1 symbols) and
`BASTION_CAMERA.md`. Line numbers drift; symbols anchor.

## 1. The input chokepoint (where the context layer goes)

The **single fan-out point** for physical key/button → `GameInput` is
`Window::map_input(KeyMouse, &mut ControlSettings, ...) -> Option<MappedInput>`
(`voxygen/src/window.rs:1265`): it resolves `controls.get_associated_game_inputs(&key)` — the
`inverse_keybindings` HashSet — and its two call sites push one `Event::InputUpdate(gi, state)` per
associated GameInput:

- keyboard: `handle_window_event` → `WindowEvent::KeyboardInput` (window.rs:812-867)
- mouse buttons: `WindowEvent::MouseInput` (window.rs:765-784) — **only when the cursor is
  grabbed** ("Mouse input not mapped to input if it is not grabbed"); the raw
  `Event::MouseButton(button, state)` is *always* pushed (line 784).

**Context filter inserted here** (both push loops): a `bastion::input::InputContextState` stored on
`Window` drops GameInputs that are not live in the active context, *before* the HUD or session ever
see them. This kills the whole "two schemes share one key" class: the HUD can't steal `Q` in
Overseer because `Slot10` never fires there. B1's `Hud::bastion_overseer_active` gates and the
session's no-op guard arm are deleted in favor of this.

Related free-cursor facts that shape the B&W2 controls:
- `WindowEvent::MouseWheel → Event::Zoom` is **gated on `cursor_grabbed`** (window.rs:880) — the
  overseer runs with a *free* cursor, so a bastion condition must open that gate.
- Free-cursor motion arrives as `Event::CursorMove(delta)` (raw deltas, window.rs:735) and the
  absolute position is tracked in `Window::cursor_position` (physical px, window.rs:877; accessor
  added). Grabbed motion is `Event::CursorPan` — not used by the overseer.
- `Window::grab_cursor(bool)` (window.rs:921) flips grab + pointer visibility. Overseer = free,
  Avatar/vanilla in-game = grabbed.

## 2. Context model (§3b)

`voxygen/src/bastion/input.rs`:
- `InputContext { Menu (default), Overseer, Avatar }`. **Menu = strict passthrough** (vanilla
  bit-identical; it is the active context whenever `--bastion-overseer` wasn't passed).
- Context is **derived, not duplicated**: `Overseer` iff the flag is on and the camera is in
  `CameraMode::Overseer`; `Avatar` iff flag on and any other camera mode (the B12 stub — vanilla
  third-person controls driving the character); else `Menu`. The session syncs the derived context
  into `Window` when it changes; the swap is one enum write = atomic whole-table switch.
- Each context has a `ContextScheme { owned, suppressed }` static table. WASD is deliberately in
  no suppression list (shared: pans the god camera / moves the body). Overseer suppresses the
  avatar verb set (Primary/Secondary/Interact/Slot1-10/Roll/Mount/... plus `CycleCamera`,
  `ToggleCursor`, `SpectateViewpoint` to protect camera/cursor invariants); Avatar suppresses the
  overseer keys (`BastionRotate*/Slice*/SnapTopDown`). `BastionToggleOverseer` (F9) is live in both
  — it is the mode switch. B9's rebind tabs add a per-context `overrides: HashMap<GameInput,
  KeyMouse>` field to `ContextScheme`; nothing else needs to change.

## 3. Camera-feel math (all through `Camera::dependents()`)

- **Unproject:** `inv = (proj_mat * view_mat).inverted()`; two clip points `(ndc, z=1)` /
  `(ndc, z=0)` (reversed depth) → ray; intersect plane `z = plane_z`. One caveat from B1: the view
  matrix translates by `-focus.fract()` only — unprojected coords are **relative to
  `focus.trunc()`** (`focus_off`), so add it back for world space. Well-conditioned for ortho at
  pitch ≥ 20° (ray ∥ camera forward; `dir.z ≠ 0`).
- **Grab-drag:** on press, pick anchor on plane `z = slice.unwrap_or(focus.z)`; per frame, delta =
  anchor − point-under-cursor-now, apply via `force_focus_pos` (instant — the lock must be 1:1;
  easing belongs to release only). Release inertia: EMA'd drag velocity decays as
  `vel *= exp(−damp·dt)`.
- **Orbit:** right-drag accumulates `Event::CursorMove` deltas → `set_orientation` targets (camera
  lerp = damping). Pitch clamp 20°–89°; yaw unclamped/continuous. Snap-to-top-down = new
  `GameInput::BastionSnapTopDown` (Home, unbound in vanilla).
- **Zoom-to-cursor:** `Event::Zoom` (wheel) → `f = tgt_dist_new / tgt_dist_old` (multiplicative,
  clamped in `Camera::zoom_by`), then `tgt_focus = p + (tgt_focus − p)·f` where `p` = point under
  cursor on the focus plane; both dist and focus ease via the camera's existing interpolation.
  Needed two tiny additive getters: `get_tgt_dist()`, `get_tgt_focus()`.

## 4. Streaming (mostly already exists)

`session/mod.rs:772-781` **already** calls `client.spectate_position(cam_pos)` every tick when
`presence == PresenceKind::Spectator` (sends `ClientGeneral::SpectatePosition`, writes the client
entity `Pos` — client/src/lib.rs:1950). So spectator sessions already stream — but around
**`cam_pos`** (the boom position, up to ~`dist` away from what you're looking at). The B1.5 change:
in overseer mode pass the **focus** (the ground point under the view) instead. Character-presence
sessions still stream around the character only — full camera-streaming for the hero-less colony
arrives with B3 (documented limitation, unchanged from B1).

## 4b. Machine/verification discoveries

- **ReShade is installed on this machine** (Vulkan implicit layer). Its overlay toggle defaults to
  `Home` and, while the overlay is open, ReShade **blocks all input to the game** — it also ignores
  *injected* keystrokes for its toggle, so scripted verification can't close it. Consequences:
  `BastionSnapTopDown` default moved `Home` → `End`; scripted runs disable implicit layers via
  `VK_LOADER_LAYERS_DISABLE=~implicit~`.
- Scripted **orbit** verification must inject *relative* mouse motion (`mouse_event(MOUSEEVENTF_MOVE)`)
  — `SetCursorPos` teleports produce `WindowEvent::CursorMoved` (grab-drag sees them via the tracked
  absolute position) but no `DeviceEvent::MouseMotion`, which is what `Event::CursorMove`/orbit
  consumes — same raw-motion channel as vanilla mouse-look.

## 5. HUD interception notes

- The HUD consumes hotbar keys inside `Hud::handle_event` via `try_hotbar_slot_from_input` (two
  sites, hud/mod.rs ~4963 and ~5155) — B1's boolean gate there is **removed**; suppression now
  happens upstream at the window fan-out.
- The HUD does **not** handle raw `Event::MouseButton` — conrod widget clicks arrive via
  `Event::Ui`. So the session sees every mouse press even over HUD widgets; grab-drag therefore
  gates on conrod's `widget_under_mouse != ui.window` ("cursor over a real widget") to avoid
  grabbing the world through a button click.
