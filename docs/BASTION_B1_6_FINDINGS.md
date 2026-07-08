# Bastion B1.6 — Findings (unified occlusion & transparency)

Verified against `bastion/main` @ B1.5 complete. Read with `BASTION_B1_FINDINGS.md` (the B1 slice hook)
and `BASTION_CAMERA.md`. Line numbers drift; symbols anchor.

## 1. What B1.6 generalizes

B1 added one hook: each of `terrain-frag`, `sprite-frag`, `fluid-frag/{cheap,shiny}` starts `main()`
with `if (f_pos.z + focus_off.z > bastion_slice_z) discard;`, fed by a single `Globals.bastion_slice_z`
f32 (`f32::MAX` = off), gated to overseer mode in `scene/mod.rs` at the `Globals::new` call.
B1.6 replaces the constant threshold with a **function**: one uniform block + one shared alpha
function, driven four ways (soft-slice / proximity / cutaway / roof-reveal), composed by `min` alpha.

## 2. The transparency mechanism: screen-door dither, not blending

**Critical constraint:** terrain/sprite/fluid/figure/particle are **opaque passes** (depth write, no
alpha blend). You cannot "fade" by lowering output alpha — it won't blend. That is exactly why B1 used
`discard`. So B1.6's soft fade is **ordered dithered discard** (screen-door transparency): compute
alpha 0..1, `discard` the fragment when `alpha < bayer(gl_FragCoord)` using a 4×4 Bayer matrix. Cheap,
needs no pipeline/blend-state change, works identically in every opaque pass and in the shadow pass.
(Veloren already ships a `dither()` in `random.glsl`, but it's gated behind
`EXPERIMENTAL_DISCARDTRANSPARENCY`; we ship our own unconditional Bayer in the bastion include.)

## 3. Passes and where the hook goes (all have `f_pos` in scope)

| Pass | File | Insert point |
|---|---|---|
| Terrain | `terrain-frag.glsl:74` | replace the B1 slice `discard` at top of `main` |
| Sprite | `sprite-frag.glsl:54` | replace the B1 slice `discard` |
| Fluid (cheap) | `fluid-frag/cheap.glsl` | replace the B1 slice `discard` |
| Fluid (shiny) | `fluid-frag/shiny.glsl` | replace the B1 slice `discard` |
| Figure | `figure-frag.glsl:92` | new discard at top of `main` (`f_pos` present; existing dither uses it) |
| Particle | `particle-frag.glsl:39` | new discard at top of `main` (`f_pos`, `f_norm` present) |
| **Shadow (directed)** | `light-shadows-directed-vert.glsl` + `light-shadows-frag.glsl` | **hard**: vert computes `f_pos` but does not pass it to the (empty) frag; needs a new `out vec3 f_pos` varying + matching `in`, then dither-discard in the frag. Figures cast via `light-shadows-figure-vert.glsl`. |

Re-lighting is added where each surface pass writes color: terrain at `surf_color` (terrain-frag:550,
output :578), likewise sprite/figure. Add a top-down fill term before the final write.

## 4. Uniform layout (std140, Rust `Globals` in `render/pipelines/mod.rs`)

`Globals` currently ends `... screen_fade: f32, bastion_slice_z: f32` — one clean `vec4`
`[sprite_render_distance, player_ori, screen_fade, bastion_slice_z]`. B1.6 replaces the last slot with
a pad and appends the block on fresh 16-byte boundaries (all rows are vec4/uvec4, so bytemuck `Pod`
stays valid and the `size % 16 == 0` assert holds):

```
// tail of Globals (GLSL names in globals.glsl mirror these):
float screen_fade;
float _bastion_pad;                 // was bastion_slice_z
uvec4 bastion_occ_mode;             // .x mode bitmask, .y target_count
vec4  bastion_occ_a;                // .x slice_z(world), .y fade_band, .z focus_z(world), .w prox_strength
vec4  bastion_occ_b;                // .x height_start, .y height_end, .z dist_start, .w dist_end
vec4  bastion_occ_c;                // .x cutaway_radius, .y roof_low, .z roof_high, .w relight_strength
vec4  bastion_occ_targets[4];       // .xyz target in f_pos space (world - focus_off), .w enabled
```

Mode bits: `SLICE=1, PROXIMITY=2, CUTAWAY=4, ROOF=8` (compose freely; `0` = Solid = return 1).
`target_count` ≤ 4.

**Coordinate spaces (precision-safe, reuse existing globals — no huge-float subtraction):** vertex
shaders emit `f_pos = world - focus_off`. So: fragment world z = `f_pos.z + focus_off.z`; fragment
relative to focus in XY = `f_pos.xy - focus_pos.xy` (`focus_pos` global is `fract(focus)`); camera in
`f_pos` space = `cam_pos.xyz + focus_pos.xyz` (`cam_pos` is the camera's offset from focus). Targets
are passed pre-subtracted (`target_world - focus_off`) from Rust to live in `f_pos` space.

## 5. Shared alpha function (`assets/voxygen/shaders/include/bastion_occlusion.glsl`)

`float bastion_occlusion_alpha(vec3 f_pos)` → 0 (hidden) .. 1 (solid), `min`-composed across active
modes. `bool bastion_occlusion_discard(vec3 f_pos, vec2 frag_coord)` wraps it with the Bayer test.
`vec3 bastion_relight_add(vec3 f_pos, vec3 f_norm)` returns the interior fill. Solid (mode 0) short-
circuits to 1 / no fill — the vanilla-look regression check.

## 6. Roof mask: the cheap/approximate signal (no in-shader upward sampling)

There is **no cheap per-column coverage** available in-shader (terrain frags have `f_pos`+normal, no
vertical voxel sampling; `view_distance.z/.w` are global sea-level/max-height, not per-column). Per the
block's allowance, B1.6 uses an **approximate geometric mask**: "roof" = geometry in a height *slab*
above the focus plane (`roof_low..roof_high` above `focus_z`) **and** near the focus in XY (so distant
hills/mountains stay solid). This visibly reveals a building's roof/upper walls when you look into it,
at zero sampling cost. **Refine path (B2/B3):** feed a real per-column/room coverage bit (from the
server's structure/room data or a client-side chunk heightmap) into `bastion_occ` to restrict the mask
to genuinely enclosed columns. Documented as a stub.

## 7. Re-lighting plan (do it properly)

Revealed interiors go dark because the (unsliced) roof still shadows them and they lose skylight. Fix =
inject a **soft top-down fill** over exposed interior surfaces: `fill = relight_strength · below ·
near · up`, where `below` = fragment at/under the slice (or under focus for roof-reveal), `near` =
close to focus in XY, `up = clamp(f_norm.z·0.5+0.5)` (brightest on floors/upward faces → reads as
lit-from-above, RimWorld-style). Added to `surf_color` in terrain/sprite/figure frags. This *also*
covers the case where shadow-pass fade is deferred — interiors stay readable regardless.

## 8. Targets — stubbed (mark for B2/B3)

`bastion_occ_targets` this block = the **camera focus point** + up to 3 debug markers offset around it
(so cutaway is demonstrable). **Replacement path:** B2 feeds hovered/`Selected` entity world positions;
B3 feeds colonist positions. The Rust side already funnels targets through one
`BastionOcclusion::targets: Vec<Vec3<f32>>` — B2/B3 just populate it.

## 9. Controls / plumbing

- Params live on a new `scene` field (`BastionOcclusion` in the `bastion` module) and are packed into
  `Globals::new` (the single f32 arg becomes the packed block). Gated to overseer mode exactly like the
  B1 slice; Solid/`f32::MAX`-equivalent (mode 0) everywhere else, so vanilla + char-select unaffected.
- A **view-mode cycle** key (`GameInput::BastionCycleViewMode`, default `V`) and per-mode toggles +
  a strength slider go through the **B1.5 Overseer input context** (added to `OVERSEER_SCHEME.owned`),
  and are surfaced in the **egui debug panel** (behind `egui-ui`), structured to move to the B9
  settings tab. The B1.5 `BastionSliceUp/Down` keys keep driving `slice_z`.

## 10. Shadows — attempt, else the single documented deferral

Threading `f_pos` through the directed-shadow vert→frag and dither-discarding is the one genuinely hard
pass (shared vert across terrain; separate figure vert; a geom shader for point lights). Plan: attempt
the directed (sun/moon) terrain shadow first; if it destabilizes or costs too much, **defer shadows
only** and rely on re-lighting for interior readability — recorded here as the sole acceptable
deferral, never a silent drop.
