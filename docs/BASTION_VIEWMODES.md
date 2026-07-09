# Bastion overseer view modes — occlusion & transparency (B1.6)

One framework generalizes B1's hard Z-slice into four composable, always-cheap view behaviors so the
player rarely needs the manual cut. All four are the **same fragment operation**: a dithered
screen-door discard whose alpha is a function of position (`bastion_occlusion_alpha`), plus an
interior re-lighting fill. See `docs/BASTION_B1_6_FINDINGS.md` for the internals; this doc is the
user/《B9》-facing reference.

## The four behaviors (compose by `min` alpha)

| Behavior | What it does | Params |
|---|---|---|
| **Slice** | B1's manual cut, upgraded to a smooth fade band (no aliased edge) | `slice_z` (PgUp/PgDn), `fade_band` |
| **Proximity / height** | Tall geometry *near the view center* fades so the ground around the focus stays readable; the height fade is windowed by distance-from-focus, so the distant panorama (background mountains) stays solid | `strength` (slider), `height_start/end`, `dist_start/end` (the window, as fractions of the view radius) |
| **Cutaway** | Geometry between the camera and tracked targets fades so targets show through walls (Diablo-style) | `cutaway_radius`, `targets[]` |
| **Roof / interior reveal** | Geometry in a slab above the focus, near it, fades so you can see into buildings (RimWorld-style) | `roof_low/high` |

**Interior re-lighting** adds a soft top-down fill over exposed interior surfaces (brightest on
floors) so revealed rooms read *lit-from-above*, not black — `relight_strength`. It is localized to
the roof-reveal radius and **scaled by daylight** on the CPU: it's an additive linear-light term, so
unscaled it blew night scenes out to white (QA round 3).

**Ground glide (B&W2):** in overseer mode the camera focus rides the terrain surface every frame
(water surfaces, not seabeds), and the camera lifts so neither it nor its sight line to the focus
dips under terrain. This is also what anchors the whole occlusion frame of reference: the slice
auto-places at ground level, and the proximity height fade measures from the ground — in spectate
the focus otherwise floats at the spawn altitude and every mode silently degrades (QA round 3:
slice plane hundreds of blocks up, Reveal a no-op).

## View modes (the cycle key presets)

`V` (`GameInput::BastionCycleViewMode`, Overseer context) cycles:

- **Solid** — mode 0, nothing hidden (vanilla look).
- **Reveal** — a gentle proximity readability layer by default (tall foreground softly fades so you
  see the ground). Roof-reveal + cutaway are **opt-in toggles** in the egui panel this block (their
  approximate/stubbed masks artifact as an always-on default — see `BASTION_B1_6_FINDINGS.md` §4d);
  they rejoin the auto-default when B2/B3 feed real per-room coverage + hovered/colonist targets.
- **Slice** — manual cross-section + proximity; `PgUp`/`PgDn` drive the slice height (and auto-select
  this mode).

Toggle semantics: **roof** and **cutaway** compose on top of any non-Solid preset (add *or* remove —
that's how you demo them); **proximity** likewise. The **slice** is *preset-gated*: it only cuts in
the Slice view mode (its toggle can disable it there, but a leftover slice height never bleeds into
Reveal — that bled once and made Reveal == Slice, a hard ground cut in both). Cycling into Slice
with no cut set auto-places it just above the focus. **Solid** is always truly solid.

## Controls

| Input | Action |
|---|---|
| `V` | Cycle Solid → Reveal → Slice |
| `PgUp` / `PgDn` | Move the manual slice height (activates Slice) |
| egui **Debug Control → Overseer Occlusion** | View-mode buttons, per-behavior checkboxes, and sliders: transparency strength, interior relight, cutaway radius, slice fade band |

The egui panel is behind the default `egui-ui` feature; open the Debug Control window (F3 area / egui
toggle) and tick **Overseer Occlusion**. The panel snapshots live scene state and applies edits back
via `EguiAction::SetBastionOcclusion` — the same flat data model a **B9 settings tab** will bind to
(view-mode + toggles + sliders), no rework needed.

## Stubbed inputs (replace in B2/B3)

- **Cutaway targets** are stubbed to the camera focus + 2 debug markers (so cutaway is demonstrable).
  **B2** feeds hovered/`Selected` entity positions; **B3** feeds colonist positions — both just push
  into `Occlusion::targets: Vec<Vec3<f32>>`.
- **Roof mask** is an approximate geometric slab (height above focus × near focus in XY), because no
  cheap per-column coverage exists in-shader. **B2/B3** refine it with real per-column/room coverage
  data; the shader reads it from the same `bastion_occ` block.

## Passes covered

Terrain, sprites, fluid (cheap + shiny), **figures**, and **particles** all fade through the one
shared function. **Shadows are deferred** (the single documented deferral): the directed/point/figure
shadow pipelines are depth-only (`fragment: None`), so fading them means adding fragment stages to 3
pipelines and losing early-Z on the heaviest pass — and re-lighting already makes interiors readable.
See findings §10 for the refine path.

## Performance

The alpha function is arithmetic-only (no texture/voxel sampling; the roof mask is a couple of
`smoothstep`s, not upward sampling), added at the *top* of each frag as an early screen-door discard —
so hidden fragments cost *less* than before (discarded pre-shading). Measured fps per mode is recorded
in the block's report / `BASTION.md`.

## Tunables (defaults, `voxygen/src/bastion/occlusion.rs`)

`fade_band 6`, `strength 0.6`, `height_start/end 12/60`, `dist_start/end 0.55/1.0` (the central
window, as a *fraction of the on-screen view radius*, so it tracks zoom instead of a fixed block
distance), `cutaway_radius 6`, `roof_low/high 3/14`, `relight_strength 0.5` (scaled by daylight at
pack time). B1.8's colony-scale focus policy can push them harder. All live-editable in the egui
panel.

## Embodied overseer (character presence) — the primary mode

With a **character**, the overseer view now works fully (QA round 4 made character presence the
primary way to play):

- **Terrain streams around the camera** via the *god-camera terrain anchor*
  (`ClientGeneral::BastionCameraAnchor` → `Presence::bastion_terrain_anchor`): the client requests
  and retains chunks around the overseer focus (the avatar's area is retained too), and the server
  validates requests, distributes newly generated chunks, and keeps chunks loaded around the anchor
  as if it were a second player position. Unlike `spectate_position` it never moves the avatar.
  The anchor uses **hysteresis** — the loaded disc stays put while the camera pans inside it and
  re-centers only when the focus strays past ~35% of the view radius. (Re-centering every pan step
  kept a missing chunk next to the center at all times, which collapsed the fog/detail radius and
  rendered the whole view as LoD — QA round 5.) Chunk-request throttles are also raised (48 in
  flight / 8 per tick vs vanilla's walking-pace 12 / 2) so a re-centered crescent fills quickly.
- **The avatar is hidden** while in overseer view (client-side figure cull); `F9` (Avatar mode)
  brings it back. NOTE: the character still *exists* in the world — it can be attacked while you're
  god-viewing. B2's overseer presence handles the body properly (safety/removal).
- **Entities don't yet sync around a far anchor** — NPCs run server-side in anchored chunks, but
  the client only receives entity updates around the avatar (region subscription). B2 extends the
  subscription to the anchor.
