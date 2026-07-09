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
| **Proximity / height** | Foreground floor near the focus stays solid; tall/background geometry fades — the always-on readability layer | `strength` (slider), `height_start/end`, `dist_start/end` |
| **Cutaway** | Geometry between the camera and tracked targets fades so targets show through walls (Diablo-style) | `cutaway_radius`, `targets[]` |
| **Roof / interior reveal** | Geometry in a slab above the focus, near it, fades so you can see into buildings (RimWorld-style) | `roof_low/high` |

**Interior re-lighting** adds a soft top-down fill over exposed interior surfaces (brightest on
floors) so revealed rooms read *lit-from-above*, not black — `relight_strength`.

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

`fade_band 6`, `strength 0.5`, `height_start/end 20/120`, `dist_start/end 0.72/1.15` (as a
*fraction of the on-screen view radius*, so the distance vignette tracks zoom instead of a fixed block
distance), `cutaway_radius 6`, `roof_low/high 3/14`, `relight_strength 0.5`. Defaults are deliberately
gentle because the overseer sees a wide area; B1.8's colony-scale focus policy can push them harder.
All live-editable in the egui panel.
