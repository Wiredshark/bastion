# B5.6a self-test results — zone visuals: draping + toggle + pile tiers

Run: 2026-07-09, branch `bastion/block-B5.6a` (`eb8984e..`), gate per the
approved B5.6a split (Parts 1-outlines, 3-toggle, 4-piles; fills/volumes are
B5.6b) + standing invariants. Result: **PASS** (in-game verified by Ben).

## Compiles
`cargo check`: veloren-common, veloren-voxygen, veloren-server — green.
Voxygen + server-cli binaries built.

## Unit tests
`cargo test -p veloren-common --lib bastion` — **6/6**, including the two
that reproduce and pin the erase fix:
- `erase_by_xy_removes_regardless_of_z_misalignment` — the naive `subtract`
  misses a z-misaligned erase (reproduces the bug); `clip_xy` + `subtract`
  removes cleanly (the fix).
- `erase_partial_xy_leaves_remainder_at_correct_z` — partial erase leaves the
  un-brushed remainder, at the designation's own z.

## Headless invariants (client-only block → assert sim UNAFFECTED)
`--b4` / `--b5` / `--b55` scenarios, **3/3 each = 9/9** on a quiet machine
(the earlier B5 flake was root-caused as environmental machine-load — see the
consistency note; B5.5-tag and this branch both run 6/6 clean when quiet).
The block is client-side (voxygen) + one additive `common` method
(`clip_xy`, never called by the server) + a pile-`Scale` tweak, so zero sim
impact — confirmed. B5.5's soak tail (600 ticks) passed within its scenario.

## Vanilla regression
`veloren-server-cli` flagless boot: clean, 0 panics (verified this session at
the B5.6a code state). The diff since — client-side + additive `common`
method — does not touch server-cli's boot path (mega-prompt: "don't
full-rebuild unless the diff touches shared paths"), and the harness
exercised the full server tick 9× with 0 panics.

## In-game visual QA — VERIFIED BY BEN (the block's core deliverable)
- **Draping** (the photographed bug): PASS — Ben's screenshot shows Mine-zone
  outlines hugging the excavation rim and stepping down slopes, no floating.
  Verified across the excavation and sloped terrain.
- **Visuals toggle (H)**: PASS (after the auto-reveal fix) — cycles
  On → Subtle → Off; Off genuinely hides committed overlays; the active
  paint preview still shows while dragging.
- **Erase**: PASS (after the XY-clip fix) — erasing after moving/zooming the
  camera removes the overlay cleanly; partial erase leaves only the un-brushed
  part. (Was: z-misaligned cancel silently missed, leaving a surface-draped
  remnant that looked like the whole zone.)
- Pile tier growth: shipped (5-step curve + plateau cap, server-side);
  conservation exact (count read-only). Visual tier plateau not stress-tested
  in-game this pass (headless conservation covers the count invariant).

Ben's verdict (2026-07-09): "yes they all worked."

## Bugs found + fixed at the gate (Ben's first live test)
1. H toggle no-op — auto-reveal forced overlays On whenever a designate tool
   was selected; removed it. (H fires only `BastionCycleVisuals` in overseer;
   Greet/Fly, the other H bindings, are already suppressed there.)
2. Erase left overlay/jobs behind — erase drag's z came from the camera
   pick-plane; z-misaligned cancel missed. Fixed via `Region::clip_xy`
   (XY-match at each rect's own z). See `docs/BASTION_B5.6a_FINDINGS.md` §7.

## Standing invariants
No panics. No sim impact (client-only + additive). Conservation exact
(pile scale is count-read-only). Erase claim-release is the proven B5.5 AABB
path, now z-robust. Vanilla untouched.

## Deferred (logged to backlog; not B5.6a)
- Fills + volumetric + volume-selection UX + erase-by-type → **B5.6b** (its
  own session; RimWorld zone-UI reference captured).
- Mine-coverage gap (colonists leave some designated cells) → **B5.MINE-
  COVERAGE** investigation (possible 5th vertical-reachability bite).
- Terrain-edit overlay restaling; pile scale-lerp — backlog.
