# B5.6b-1 self-test results — zone fills + colors + blend + labels + SUBTLE

Run: 2026-07-09, branch `bastion/block-B5.6b-1` (`de86387..51ddf22`), first
formalized B5.6b sub-block (architect-blessed split). Result: **PASS**
(in-game verified by Ben, including the three fix-before-tag demo bugs).

## Compiles
`cargo check` voxygen + common green (first try on the fills build);
voxygen exe built and live-tested repeatedly.

## Unit tests
`cargo test -p veloren-common --lib bastion` — **6/6** (Region suite incl.
the B5.6a erase-fix reproductions; `contains_point_xy` exercised via the
radial matching).

## Headless invariants (client-only block → sim unaffected)
`--b4` / `--b5` / `--b55`: **3/3 PASS** on a quiet machine, re-run after the
final fix commit (it touched `common` — additive method only). Soak rides
the B5.5 scenario tail as usual.

## In-game visual QA — VERIFIED BY BEN
The b-1 deliverables:
- **Fills**: terrain-conformed translucent ground fills in the kind-color
  legend (Mine orange / Chop green / Build blue / Stockpile purple).
- **Overlap blend**: overlapping zones alpha-composite to a mixed color.
- **Labels**: world-anchored "Mine 1"-style labels at zone centroids.
- **SUBTLE**: border-only; ON = fill+border+label; OFF = nothing.

Three God-mode-demo bugs found by Ben's live test, fixed on-branch
(`51ddf22`), and **eyeball re-verified**:
1. **Overlays climbed trees** — `overlay_surface_z` now walks down past
   canopy Wood/Leaves + under-canopy air to real terrain kinds (liquid
   surface kept for the water glide). Camera `ground_z` glide untouched.
2. **Radial Delete-zone dead near zone centers** — the new labels hit-tested
   as conrod widgets and `bastion_cursor_over_widget` swallowed clicks;
   labels are now input-transparent (`graphics_for(window)`). Zone matching
   also hardened to XY footprint (`Region::contains_point_xy`) — the clicked
   surface block's z routinely falls outside the rect's paint-time z-band on
   slopes (same z-fragility class as the B5.6a erase fix).
3. **Grab-drag pan off-center** — the anchor plane was the camera-focus z;
   `bastion_begin_grab` now two-pass-samples the (canopy-safe) terrain
   height under the cursor; active Z-slice still overrides.

## Consistency note
The queue's root-cause summary for bug 1 also names "slice-clamp direction +
rebuild not triggered on view-mode change"; the shipped fix addressed the
canopy walk (+ the B5.6a-era slice/rev/visuals rebuild triggers already in
place), and Ben's re-verify passed. View-mode (V-cycle) change does not
itself trigger an overlay rebuild — overlays only depend on it via slice_z —
recorded as a watch item, not an open bug (see BASTION_CONSISTENCY).

## Standing invariants
No panics; no sim impact (client-only + one additive common method);
conservation untouched; vanilla input paths untouched (labels passthrough is
overseer-HUD-only).
