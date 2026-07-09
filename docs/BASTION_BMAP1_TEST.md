# BASTION_BMAP1_TEST — gate record

Block: B-MAP1 (overseer minimap). Branch `bastion/block-BMAP1` (base
`de86387`), isolated worktree `.claude/worktrees/bmap1`.

## 1. Compile gate — PASS (2026-07-09)

- `cargo check -p veloren-voxygen` → **green, zero warnings** (shared target
  dir; `Finished dev profile in 37.79s` after the workspace-crate rebuild).
  One fix during verification: `specs::LendJoin` import for
  `Storage::maybe()` (this fork's join traits — a known recurring stumble,
  see the architecture gotcha list).
- Diff surface: `voxygen/` ONLY (hud + bastion module + session wiring) +
  docs. `common/`, `common/net`, `server/`, `rtsim/` untouched — the
  headless sim gates (B4/B5/B5.5 scenarios, soak, invariants) are unaffected
  by construction; no protocol or comp changes (pile pins ride the
  already-synced `PickupItem`/`Scale`; colonist pins ride the synced
  `Colonist`).
- Vanilla compile path: voxygen lib+bin type-check covers the vanilla HUD
  path (the bastion minimap is only constructed as an empty struct at Hud
  init; all behavior is gated on `bastion.active`, which only
  `--bastion-overseer` + F9 can set).

## 2. Live visual gate — PENDING (needs the machine slot; one builder at a
time per the architect's ordering, and building replaces the shared
`target/debug/veloren-voxygen.exe`)

Checklist (run with `--bastion-overseer`, F9 into overseer):

1. **Tiles render real terrain** at colony zoom — buildings/trees/dig sites
   recognizable, hillshaded relief (screenshot for the run log).
2. **Zoom pyramid**: scroll steps Colony → District → Region → World; tile
   layer crossfades into the worldgen map with no hard pop; level label
   updates.
3. **Dig invalidation**: paint a Mine zone, let colonists dig (or edit
   terrain) → the minimap tile updates within seconds, **no frame hitch**
   while updating.
4. **Overlay accuracy**: painted zone footprints on the minimap match their
   in-world draped overlays in position and color (same
   `designation_color`); colonist dots track colonists; selected colonist
   shows the halo; piles show gold markers sized by tier.
5. **Camera frustum** rectangle matches what the main view shows (pan/orbit
   and watch it follow).
6. **Click-to-jump**: click a far point on the minimap → god camera glides
   there (focus re-rides ground). **Drag-to-pan**: world moves with the
   cursor. **Layer chips** C/Z/P/F/! toggle their layers.
7. **Z-slice**: slice below ground (PageDown) → minimap re-renders to the
   slice level (trickled; old tiles keep showing until replaced).
8. **Vanilla flagless boot**: launch WITHOUT `--bastion-overseer` → the
   vanilla minimap appears and behaves stock (voxel map, zoom buttons,
   markers); no bastion widget, no extra maintain cost.

Result: to be appended after the live run.
