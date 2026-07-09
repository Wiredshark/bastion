# B5.6a findings — outline draping + visuals toggle + pile tiers

Spec: `readme/B5.6-zone-visuals-prompt.md` (Parts 1-outlines, 3-toggle,
4-piles; the approved B5.6a split — see `readme/BASTION_CONSISTENCY.md` B5.6
entry). Fills + volumetric + volume-selection UX are B5.6b. Built 2026-07-09
on `bastion/block-B5.6a` (start `eb8984e`). Almost entirely client-side.

## 1. The floating-overlay bug — root cause confirmed

`voxygen/src/session/mod.rs` `bastion_region_outline` drew 4 flat
`DebugShape::Line`s at `z = max.z + 0.15`. The region's z comes from the
paint: `min.z-2 .. max.z` where `max.z` = the pick-plane height =
`bastion_plane_z()` = the active Z-slice height **or the flat camera-focus
`z`**. So with no slice active, the whole rectangle is drawn at one flat
camera-focus height → it floats over any sloped terrain
(`readme/evidence-b56-floating-selection-bleabrolm.png`). Ben's F9/slice
lead is consistent: toggling the slice changed `bastion_plane_z()`, moving
the flat rectangle to a height that happened to align better that session.

## 2. The fix — terrain draping (reusable)

New `voxygen/src/bastion/mod.rs`:
- `overlay_surface_z(terrain, xy, z_hint, slice_z)` — the single height
  authority. Normally `ground_z` (the existing B1.5 surface-glide helper);
  while a Z-slice is active it clamps to `slice_z` (the occlusion pass
  discards terrain above the slice, so the *visible* top is the cut — the
  overlay must sit on that, not the true ground, or it floats above the
  sliced surface).
- `draped_rect_outline(terrain, min_xy, max_xy, z_hint, slice_z, hover,
  step)` — walks the 4 perimeter edges at `step`-block strides, samples the
  surface at each, returns conformed world-space line segments. The caller
  emits each as a debug line.

`bastion_region_outline` now calls this (immutable client/terrain borrow to
collect segments, then mutable scene borrow to emit shapes — the borrows
must not overlap). Three callers, each draping:
- paint preview (`bastion_paint_update`) and box-select preview
  (`bastion_boxsel_update`): live drag, coarse `step = 2.0` (cheap
  per-mouse-move rebuild).
- committed designation overlay (`bastion_sync_designations`): per-cell
  `step = 1.0`, built on rev change **and now on Z-slice change** (the
  draped surface re-clamps when slicing — satisfies the Done-when's "correct
  across all slice modes / after toggles").

**SEAM for B5.6b + §3w:** `overlay_surface_z` is the shared height sampler;
conformed *fills* emit draped quads over the interior grid using the same
sampler, and the colony-boundary overlay reuses `draped_rect_outline`. Keep
one height authority so all overlays agree.

## 3. Visuals toggle (Part 3)

`voxygen/src/bastion/tools.rs` `VisualsMode { On, Subtle, Off }` (default
On). New `GameInput::BastionCycleVisuals` bound to **H** (overseer context
owns it; avatar context suppresses it — same pattern as T/G). Handler in the
session cycles the mode + marks the overlay dirty. Purely visual:
- Off → overlays not built (designations stay fully active — sim untouched).
- Subtle → outlines dimmed (`line_alpha 0.45`).
- On → full alpha (fills/volumes join in B5.6b).
Auto-reveal: while a Designate/Erase tool is active, effective mode is forced
to On even if set to Off (you always see what you paint); tool-select marks
the overlay dirty so it rebuilds on the transition. The live paint/box-select
previews always render (active drag), independent of the mode.

## 4. Pile tiers (Part 4)

`server/src/bastion_piles.rs` `tier_scale`: five steps (0.8 / 1.1 / 1.45 /
1.8) at counts 1–5 / 6–20 / 21–60 / 61–150, then a **plateau cap** 2.15 at
150+ (the count keeps rising; the visual stops). Read from
`PickupItem::amount()`; the scale never feeds back into the count
(conservation exact). Applied via the synced `comp::Scale` as before, so the
client re-renders larger with zero client change.

## 5. Judgment call — erase-by-type SKIPPED (Part 1b), area-erase already exists

Ben approved erase filters "only if cheap on existing seams." Assessment:
- **Area-erase already exists**: the B5.5 Erase tool IS a drag-rectangle
  that sends `BastionCancelDesignation{region}` → `cancel_region` removes all
  jobs in the box. Dragging Erase over an area is the area-erase.
- **Erase-by-type is NOT cheap**: `cancel_region` is region-based (removes
  every job in the box regardless of kind), and the removal echo
  (`BastionDesignationRemoved{region}`) makes the client subtract the region
  from *all* rects regardless of kind. A type filter therefore needs a
  wire-protocol change (the cancel message + removal echo gain a
  `Option<DesignationKind>`), a kind-filtered server cancel, kind-filtered
  client subtraction, and a tool-filter UI selector — a cross-cutting feature
  across 4–5 files + the protocol, not a cheap seam-ride. **Skipped**;
  logged to `readme/BASTION_BACKLOG.md` for a future block (natural fit with
  B9's colony HUD / tool polish).

## 6. Watch-items / deferred

- **Terrain-edit restaling**: the committed overlay drapes on the surface at
  build time (rev/slice change). If terrain is dug *under* a standing zone,
  the outline keeps its original height until the next rebuild — arguably
  fine (shows the original footprint), but the prompt's "rebuild on terrain
  edit under the zone" is not wired (needs a client terrain-change signal).
  Backlogged.
- **Debug-shape count**: a large zone's draped perimeter is ~P line shapes
  (one per cell). Fine for the dozens-of-zones scale; if it bites at scale,
  batch segments into a single polyline shape (a debug-pipeline addition).
- **Scale-lerp**: pile tier changes snap (discrete `Scale`); a brief
  client-side lerp ("shouldn't pop") is polish, not wired. Backlogged.
