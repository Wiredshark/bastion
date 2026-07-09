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

## 4c. Gotcha: shader `#include`s must be registered in Rust (not just the file)

A new `assets/voxygen/shaders/include/*.glsl` is **not** auto-discovered. Veloren's runtime shader
compiler resolves `#include <name.glsl>` through a hardcoded closure, so every include is registered
in **two** Rust places or startup panics with `Include <name> in <shader> is not defined` (crash in
`pipeline_creation.rs` during `initial_create_pipelines`, before any world loads):
1. `voxygen/src/render/renderer/shaders.rs` — add `"include.bastion_occlusion"` to the asset list.
2. `voxygen/src/render/renderer/pipeline_creation.rs` — `let bastion_occlusion =
   shaders.get("include.bastion_occlusion").unwrap();` and a `"bastion_occlusion.glsl" =>
   bastion_occlusion.0.to_owned(),` arm in the `fetch_include` match.
This bit B1.6 (figure-frag `#include <bastion_occlusion.glsl>` crashed at startup). Future
shader-touching blocks (B1.7/B1.8/B13) adding includes must do the same.

## 4d. In-game findings & known limitations (verified 2026-07-08, spectate overseer)

Framework confirmed working (compiles, runs, discards drive the fade, 55–60 fps in Solid/Reveal/
Slice at normal zoom; view-mode cycle key + chat feedback confirmed). Limitations found, all rooted
in the **approximate/stubbed masks** the block explicitly allows — real data in B2/B3 resolves them:

1. **Roof mask reveals whole caves as a circular hole.** The approximate roof mask fades the
   height-slab (`roof_low..roof_high` above focus) within `roof_radius` of the look point. Over a
   Veloren *cave* site, that thin above-ground band *is* the cave roof, so removing it exposes the
   entire cavern — reading as a big circle eating the ground. Correct-ish (it revealed what you
   looked into) but jarring. → **Roof is now off in the default Reveal preset** (opt-in via the egui
   toggle; correct on a building). Real per-room coverage (B2/B3) restricts it to genuinely enclosed
   columns.
2. **Height-proximity "cuts through" mountains.** `height_above_focus` can't distinguish a roof from
   a mountainside, so tall terrain above the focus plane fades (dithered → reads as cutting). Made
   gentler (strength 0.5, height 15/90) but inherent to the geometry-only approximation; the real
   fix is per-room/roof data, not a height heuristic.
3. **Camera→focus cutaway punches the foreground.** With stub targets at the focus, a top-down
   cutaway cylinder removes a column of foreground. → **Cutaway off in the default preset** (opt-in
   toggle); B2/B3 feed real off-focus targets so the cylinder is localized to an entity behind a
   wall.

**Default Reveal is therefore proximity-only** (soft foreground/tall-geometry fade) — clean on open
terrain and near settlements; roof + cutaway are demonstrable via the egui toggles and become the
auto-default when B2/B3 supply real inputs.

### Surface-error crash under extreme zoom (NOT occlusion; streaming/load)

Panning far as a spectator eventually crashes with `SurfaceError("Acquiring a texture failed…")` at
`voxygen/src/run.rs:230` (swapchain `get_current_texture` failure = GPU device loss / Windows TDR).
Observed with the scene at **view distance ~1567 blocks, ~10,000 loaded chunks, ~9,600 particles,
13–25 fps**. This is a GPU-load/TDR issue, not the occlusion pass (a fragment `discard` cannot lose
the surface). Root cause is the **unbounded spectator view distance** as the overseer focus streams
via `spectate_position` (B1.5) — the scene grows without a cap until the GPU stalls. **Follow-up
(streaming, not B1.6):** clamp the overseer/spectator effective view distance (or the streamed
radius) so a wide pan can't balloon the loaded set. Filed as a B1.5/B2 risk. *(QA round 2 pulled
`OVERSEER_ZOOM_MAX` from 1024 to 384 — the crash reproduced at max zoom-out, which is exactly the
~1400-block-span state; the clamp removes the trigger, the streaming cap remains the real fix.)*

### QA round 3: the airborne-focus root cause (fixed — ground glide)

Three seemingly separate reports — "PgUp/PgDn slice sits way off", "Reveal does nothing", "the
camera cuts through terrain" — plus "spectate behaves worse than a character" all shared one root
cause: **nothing ever grounded the overseer focus z**. A spectator spawns high in the air (observed
focus z ≈ 1102 over terrain at ~950), so the slice auto-placed ~150 blocks up, the proximity height
fade measured `height_above` from the sky (nothing qualified → Reveal ≡ Solid), the relight `below`
term covered the whole scene, and the camera had no floor. With a character the focus tracks the
(grounded) avatar, which is why modes "worked better" there. Fix: per-frame **ground glide** in the
Overseer camera arm (`bastion::ground_z`: chunk-meta-alt-seeded `try_find_ground`, riding water
surfaces) + a camera/sight-line lift (`BASTION_CAM_MARGIN`) so the camera can never go under
terrain — the B&W2 behavior.

Two independent shader bugs surfaced by the same round: the **relight radius** reused the proximity
distance thresholds (which the round-1 retune had pushed off-screen), covering the entire screen —
and as a flat additive term it rendered night scenes pure white. Relight now uses the localized
roof radius and is **scaled by daylight** CPU-side. And the **proximity fade** is now height ×
central-window (fade tall geometry near the view center only) instead of `max(height, distance)` —
resolving the round-1 "cuts through mountains" and round-3 "Reveal does nothing" complaints with
one formula.

### Chunk streaming with a character presence (B2)

Terrain streams around the *presence*: Spectate follows the overseer focus via `spectate_position`
(which physically moves the spectator entity — for a character that would be a teleport, so it is
spectator-only by design). With a character, panning past the avatar's view distance hits unloaded
void ("no new world chunks generate"). **B2** (overseer as a first-class presence) adds a
server-side camera anchor that streams terrain around the overseer focus without moving the avatar.

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

## 10. Shadows — DEFERRED (the single documented deferral)

**Decision: shadow-pass occlusion fade is deferred**, per the block's explicit "shadows are the one
acceptable deferral" allowance. Re-lighting (§7) already delivers the product requirement — revealed
interiors read lit-from-above, not black — so the shadow fade is polish (avoiding crisp
hidden-roof shadow lines on the revealed floor), not the readability fix.

Why deferred (concrete, not hand-waved): the directed **terrain** and **figure** shadow pipelines
(`render/pipelines/shadow.rs::ShadowPipeline::new` / `ShadowFigurePipeline::new`, and
`PointShadowPipeline`) are **depth-only** — every one sets `fragment: None`. `light-shadows-frag.glsl`
is not actually bound. To fade shadows I would have to:
1. add a whole fragment-shader stage to *each* of the 3 shadow pipelines (terrain-directed,
   figure-directed, point) — otherwise a hidden roof still casts figure/point shadows, so it'd be
   inconsistent;
2. thread `f_pos` as a new varying through `light-shadows-directed-vert`,
   `light-shadows-figure-vert`, and the point-shadow vert+geom;
3. accept that a discarding fragment shader **disables early-Z** on the shadow pass — the single most
   expensive pass — a perf regression on exactly the block flagged as most GPU-sensitive.

That is the "perf/scope blows" case. **Refine path (B2/B3):** once a cheap per-column/room roof mask
exists, add a minimal discard fragment stage to the shadow pipelines keyed on that mask (targeted, not
per-fragment occlusion math), so hidden roofs stop shadowing revealed interiors without the early-Z
cost on all geometry. Figures and particles *do* get the main-pass fade (they are not depth-only).
