# Bastion overseer camera (B1)

The top-down orthographic "overseer" view over the world — the god's-eye camera every later block
renders through. Additive and flag-gated: without the flag, voxygen is bit-identical vanilla.

## Launching

```powershell
target\debug\veloren-voxygen.exe --bastion-overseer
# or: $env:BASTION_OVERSEER = "1"; target\debug\veloren-voxygen.exe
```

Start singleplayer normally; once the character spawns, the camera switches to the overseer view
centered on it. The flag also arms the in-session toggle:

| Key | Action |
|---|---|
| `F9` | Toggle overseer ↔ vanilla third-person (for comparison; only armed with the flag) |
| `W A S D` | Pan the ground target across the map (speed scales with zoom) |
| mouse scroll | Zoom (orthographic scale; clamped) |
| `Q` / `E` | Rotate the view in smooth 90° steps |
| `PgUp` / `PgDn` | Z-slice cursor: first press activates the slice near the focus height, holding moves it up/down |
| `0` (CycleCamera) | Escape hatch: drops back to vanilla third-person |

While the overseer camera is active: mouse-look is disabled (fixed oblique pitch), and
`Primary`/`Secondary`/`Interact`/hotbar-slot-10 avatar actions are consumed as no-ops so Q/E and
clicks don't puppet the character (B2 hangs inspect/designate off these instead).

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
| `OVERSEER_PITCH` | `voxygen/src/scene/camera.rs` | 60° | Fixed oblique look-down angle (90° = straight down, reads poorly) |
| `OVERSEER_ZOOM_MIN/MAX` | `camera.rs` | 24 / 1024 | Zoom (`dist`) clamp; ortho half-height = `dist·tan(fov/2)` |
| `OVERSEER_START_DIST` | `camera.rs` | 192 | Zoom on entering overseer mode |
| `BASTION_PAN_FACTOR` | `voxygen/src/session/mod.rs` | 1.0 | Pan speed = `dist × factor` units/s |
| `BASTION_SLICE_RATE` | `session/mod.rs` | 16.0 | Slice speed while held, blocks/s |

## How it works (for B2+)

- `CameraMode::Overseer` (`camera.rs`) selects an **orthographic reversed-depth projection** in
  `compute_dependents_helper` and skips the terrain-collision ray entirely; everything downstream
  (globals, culling, shaders) consumes the same `Dependents` as vanilla.
- The slice height travels as `Globals.bastion_slice_z` (the former `globals_dummy` pad slot —
  std140 layout unchanged; `f32::MAX` = off). `Scene::set_bastion_slice_z` is the API; the value is
  force-disabled outside overseer mode at the single `Globals::new` call site.
- Fragment shaders (`terrain-frag`, `sprite-frag`, `fluid-frag/*`) discard when
  `f_pos.z + focus_off.z > bastion_slice_z`.
- **B2 hooks:** screen→world picking under this camera should use `Camera::dependents()`
  (`view_mat_inv`/`proj_mat_inv` — both valid for ortho); the consumed-as-no-op
  `Primary`/`Secondary`/`Interact` arms in `session/mod.rs` are the natural place to route
  inspect/designate; the slice height is the natural "active work layer" for designation painting.
- **Known limitation (B2/B3):** chunk loading follows the *player entity*, not the camera — pan far
  enough and you see LoD terrain instead of voxels. The spectator pathway
  (`client.spectate_position`) is the intended fix when colonies replace the hero.
