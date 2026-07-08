# Bastion overseer camera (B1 + B1.5)

The top-down orthographic "overseer" view over the world — the god's-eye camera every later block
renders through — with Black & White 2 camera feel and a per-mode input-context system (B1.5).
Additive and flag-gated: without the flag, voxygen is bit-identical vanilla.

## Launching

```powershell
target\debug\veloren-voxygen.exe --bastion-overseer
# or: $env:BASTION_OVERSEER = "1"; target\debug\veloren-voxygen.exe
```

> **Windows launch note (gnu toolchain):** the build needs the mingw runtime DLLs
> (`libgcc_s_seh-1.dll`, `libstdc++-6.dll`, `libwinpthread-1.dll`) resolvable at launch — either on
> `PATH` or copied next to the exe — or Windows shows "Entry Point Not Found" when singleplayer
> starts. See `docs/BASTION_B1_FINDINGS.md` §5b.

After spawning into a world (Enter World, or Spectate World), the camera drops into the overseer
view automatically. Verified in-game evidence lives in `docs/evidence-b1-*.jpg`:
top-down ortho over the generated world, 90° rotate, zoom-out, Z-slice cut + restore, and the F9
toggle back to vanilla perspective (measured 60 fps / 4 ms frame time).

Start singleplayer normally; once the character spawns, the camera switches to the overseer view
centered on it.

## Input contexts (§3b, B1.5)

Three modes own three control schemes; the active context filters the key→`GameInput` fan-out at
the window chokepoint (`bastion::input`), so schemes can share physical keys without collisions:

- **Overseer** (god mode, default with the flag): the controls below. Avatar verbs
  (`Primary`/`Secondary`/`Interact`/hotbar/`Roll`/…) are suppressed at the source — the old HUD
  "Q steals slot 10" bug class is gone by construction. The overseer also owns the *cursor*: it
  stays free/visible (the HUD's grab logic yields).
- **Avatar** (stub until B12 wires real possession): exactly vanilla controls (third-person for a
  character, freefly for a spectator); overseer keys are suppressed. `F9` swaps Overseer ⇄ Avatar —
  one atomic context switch, the same call B12's embody/release will make.
- **Menu** (vanilla passthrough): active whenever the flag is off or no session runs; every play-
  state transition resets to it.

WASD is deliberately live in both Overseer and Avatar (pans the god camera / moves the body).
Rebinding UI is deferred to B9; the per-context scheme tables in `bastion::input` are the data
model its per-mode tabs will edit.

## Overseer controls (B&W2 feel, B1.5)

| Input | Action |
|---|---|
| **left-drag** | **Grab-drag pan**: the grabbed world point stays locked under the cursor; release throws with eased inertia |
| **right-drag** | **Free orbit**: continuous yaw + pitch (clamped 20°–89°), damped |
| mouse scroll | **Zoom to cursor**: eased ortho dolly toward the point under the cursor |
| `W A S D` | Pan fallback (speed scales with zoom) |
| `Q` / `E` | Optional 90° yaw steps (keeps current pitch) |
| `End` | Snap to top-down (nearest 90° yaw, near-vertical pitch). Not `Home`: ReShade's overlay claims it |
| `PgUp` / `PgDn` | Z-slice cursor: first press activates near the focus height, holding moves it |
| `F9` | Context switch: Overseer ⇄ Avatar |

## Z-slice

The slice hides every terrain/sprite/fluid fragment above the active height, revealing interiors
and underground — the DF-style "peer down into the world" control. `PgUp`/`PgDn` activate and move
it; leaving overseer mode (F9/`0`) clears it. v1 is a hard clip; known quirks, all acceptable for
B1 and revisitable later:

- Entities (figures), distant LoD terrain, and particles are not sliced.
- Geometry hidden by the slice still casts shadows (shadow-map shaders are unsliced).
- Interiors exposed by the slice are lit as if the roof were still there (no light rebake).

## Tunables

| Constant | Where | Default | Meaning |
|---|---|---|---|
| `OVERSEER_PITCH` | `voxygen/src/scene/camera.rs` | 60° | Default pitch on entering overseer mode |
| `OVERSEER_PITCH_MIN/MAX` | `camera.rs` | 20° / 89° | Free-pitch swoop range (true 90° degenerates plane picking) |
| `OVERSEER_ZOOM_MIN/MAX` | `camera.rs` | 24 / 1024 | Zoom (`dist`) clamp; ortho half-height = `dist·tan(fov/2)` |
| `OVERSEER_START_DIST` | `camera.rs` | 192 | Zoom on entering overseer mode |
| `BASTION_PAN_FACTOR` | `voxygen/src/session/mod.rs` | 1.0 | WASD pan speed = `dist × factor` units/s |
| `BASTION_SLICE_RATE` | `session/mod.rs` | 16.0 | Slice speed while held, blocks/s |
| `BASTION_ORBIT_SENS` | `session/mod.rs` | 0.0035 | Orbit radians per pixel of right-drag |
| `BASTION_PAN_DAMP` | `session/mod.rs` | 5.0 | Inertia decay rate (1/s) after grab release |
| `BASTION_GRAB_MAX_STEP` | `session/mod.rs` | 512 | Per-frame grab translation clamp (grazing-angle safety) |

## How it works (for B2 / B12)

- `CameraMode::Overseer` (`camera.rs`) selects an **orthographic reversed-depth projection** in
  `compute_dependents_helper` and skips the terrain-collision ray entirely; everything downstream
  (globals, culling, shaders) consumes the same `Dependents` as vanilla.
- The slice height travels as `Globals.bastion_slice_z` (the former `globals_dummy` pad slot —
  std140 layout unchanged; `f32::MAX` = off). `Scene::set_bastion_slice_z` is the API; the value is
  force-disabled outside overseer mode at the single `Globals::new` call site.
- Fragment shaders (`terrain-frag`, `sprite-frag`, `fluid-frag/*`) discard when
  `f_pos.z + focus_off.z > bastion_slice_z`.
- **Picking:** `bastion::unproject_to_world_plane(camera, cursor_px, res, plane_z)` — cursor →
  world on a horizontal plane, exact for the ortho projection (`view_mat_inv * proj_mat_inv`, no
  perspective divide, `focus.trunc()` offset re-added). Grab-drag, zoom-to-cursor, and (B2)
  designation painting all share it; the active slice height is the natural work layer.
- **B2 hooks:** the Overseer context suppresses `Primary`/`Secondary`/`Interact` at the fan-out;
  B2's inspect/designate tools claim those raw mouse events in the session (the grab-drag arm shows
  the pattern) and their `GameInput`s get added to `OVERSEER_SCHEME.owned`.
- **B12 hooks:** embody = the `bastion_exit_overseer`/`bastion_enter_overseer` pair, i.e. one
  atomic context swap to `Avatar` (exactly vanilla bindings) and back — plus the server-side
  controller handoff that block owns.
- **Streaming:** in overseer mode the per-tick spectator sync sends the camera **focus** to
  `client.spectate_position`, so terrain streams under the view — no pan-to-LoD wall. Character
  (hero) sessions still stream around the character until B3 removes the hero.
