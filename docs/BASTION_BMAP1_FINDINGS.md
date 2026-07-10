# BASTION_BMAP1_FINDINGS — Overseer minimap (verified seams + approach)

Block: B-MAP1 (`readme/B-MAP1-overseer-minimap-prompt.md`). Branch
`bastion/block-BMAP1` off `bastion/main` @ `de86387`, built in an isolated
worktree (`.claude/worktrees/bmap1`) — the primary tree carried a live
B5.6b-1 session. Client-side only.

## 1. The big find: vanilla `VoxelMinimap` IS the tile machinery

`voxygen/src/hud/minimap.rs` already implements the WoW-tile architecture the
brief prescribes, CPU-side:

- **Per-chunk tiles** — `MinimapColumn` per chunk key: 32×32 color grids
  computed from actual voxel data (`Block::get_color`), one grid per z-level
  (player-centric ceiling heuristics).
- **Trickled builds** — `KeyedJobs<Vec2<i32>, MinimapColumn>` on the
  `SlowJobPool` ("IMAGE_PROCESSING"): per-chunk jobs off-thread, results
  polled — never hitches the frame. (`KeyedJobs::spawn(Some(pool), key, ...)`
  returns `Some((key, result))` only when the job has finished; call it every
  maintain until it yields.)
- **Terrain-edit invalidation** — `add_chunks_to_replace()` reads
  `client.state().terrain_changes().modified_blocks` (`common_state::
  TerrainChanges`) and marks the owning chunk dirty when a block's
  `is_terrain()` flipped. This is the same client-side edit stream the brief
  meant by "B5 already emits these"; NOTE the B5.6a draped overlay does NOT
  yet consume it (run-log watch-item "overlay terrain-edit restaling" — see
  BACKLOG), so the minimap is the first consumer, not the second.
- **Compositing** — a 256×256 RgbaImage window centered on the player chunk,
  re-blitted per-pixel (HashMap lookup per pixel — slow but shipped) on chunk
  cross / z change / new chunk, uploaded via `ui.replace_graphic`.
- **Worldgen underlay** — the minimap widget already draws `world_map`
  (`hud/mod.rs:1349`, `(Vec<Rotations>, Vec2<u32>)`, 1 texel/chunk) UNDER the
  voxel image, both zoomed via conrod `source_rectangle`. The two-tier
  near/far structure exists; only the crossfade is missing.

## 2. Approach decision (recorded drift from the prompt's RTT prescription)

The prompt prescribes GPU render-to-texture per chunk via the B1 ortho
camera. Verified against the code: voxygen's conrod UI consumes CPU images
only (`ui/graphic/mod.rs` atlas of `DynamicImage`s; `Graphic::Image`) — a GPU
tile would need a bespoke offscreen render path through the wgpu `Drawer`
(per-tile globals bind groups, pass scheduling) PLUS readback to CPU to enter
the UI atlas anyway. Meanwhile trees/buildings/dig sites are terrain voxels,
so CPU color sampling shows them faithfully. Per the §2a reuse ground rule
("wrap, don't reinvent") B-MAP1 extends the proven VoxelMinimap machinery
instead:

- fork (not modify) it into a bastion tile engine with **hillshading**
  (per-cell surface height → gradient normal → NW-lit lambert) so tiles read
  as rendered relief, not flat color blobs;
- **surface/slice scan** replacing the per-z layer stack (below);
- chunk-blit compositing (no per-pixel HashMap), chunk-grid-anchored window
  (recomposite on window re-anchor, not every chunk cross).

GPU ortho-RTT tiles stay in the backlog as a visual upgrade path; the tile
cache/invalidate/pyramid architecture is renderer-agnostic, so swapping the
tile source later does not disturb the API. Gate items are outcome-based
(recognizable buildings/trees, no-hitch updates) and are met by shaded voxel
tiles. Logged in `readme/BASTION_CONSISTENCY.md`.

**Memory note driving the layer redesign:** vanilla stores a 32×32 grid per
z-level per chunk (fine for its 8×8-chunk window). The overseer window is
16×16 chunks; per-z storage would be O(100s of MB). But the overseer has an
explicit Z-slice (mirrors the main view) instead of vanilla's implicit
player-z + ceiling heuristics — so each chunk needs exactly ONE grid for the
current mode: top-down scan from sky (no slice) or from `slice_z` (cutaway,
matching the B1.6 occlusion slice). Slice change → invalidate all tiles
(trickled rebuild). 32×32 × (rgba + i16 height) ≈ 6 KB/chunk ≈ 1.5 MB total.
This also resolves the underground watch-item: slice below ground → tiles
show that level.

## 3. Verified symbols (the seams the build rides)

| Need | Symbol | Where |
|---|---|---|
| overseer active | `self.bastion.active` in Hud (mirrored by `Hud::bastion_sync(active, tool, god_mode)` from session each frame) | `voxygen/src/hud/mod.rs:5731`, session calls it |
| session overseer flag | `SessionState::bastion_overseer_active()` | `voxygen/src/session/mod.rs:305` |
| camera in HUD | `Hud::update(..., camera: &Camera, ...)` | `voxygen/src/hud/mod.rs:1529` |
| camera focus (jump/pan target) | `Camera::set_focus_pos` (eased via tgt), `get_tgt_focus`, `get_focus_pos` | `voxygen/src/scene/camera.rs:741` |
| frustum→ground | `bastion::unproject_to_world_plane(camera, cursor_px, screen_res, plane_z)` — ortho-exact | `voxygen/src/bastion/mod.rs:125` |
| slice state | `scene.bastion_slice_z() -> Option<f32>` (session uses it at `mod.rs:863`) | scene |
| terrain-edit stream | `client.state().terrain_changes().modified_blocks` | `common_state` |
| trickle pool | `KeyedJobs::new("IMAGE_PROCESSING")` + `SlowJobPool` resource | `voxygen/src/ui/mod.rs` (KeyedJobs), ECS resource |
| zones | `client.bastion_designations() -> &Vec<(Region, DesignationKind)>` + `bastion_designations_rev()` | `client/src/lib.rs:2019` |
| zone colors | Mine `[1,.6,.1]` Chop `[.2,.9,.2]` Build `[.3,.6,1]` Stockpile `[.8,.3,.9]` — currently inline in `bastion_sync_designations` | `voxygen/src/session/mod.rs:894` → extracted to `bastion::designation_color` this block |
| colonists (client) | `comp::Colonist` IS net-synced (`synced_components.rs:60`) → join with `Pos` in client ECS | common/net |
| selected colonists | `comp::BastionSelected` markers are inserted into the CLIENT ecs by the session (`session/mod.rs:469`) — HUD reads storage directly | client ECS |
| piles (client) | `comp::PickupItem` IS net-synced (`item: PickupItem` in x-macro) + `Scale` (tier) → ground-item pins with zero protocol changes. `BastionPile` marker itself is server-only — pins show ALL ground items (acceptable: stray loot is god-relevant; see BACKLOG for a synced-marker refinement) | common/net |
| click/scroll/drag on a map widget | conrod `ui.widget_input(id).clicks()/.scrolls()/.drags()` — working reference in the big map | `voxygen/src/hud/map.rs:392-452` |
| world map underlay | `self.world_map` in Hud (`(Vec<Rotations>, Vec2<u32>)`, worldsize in chunks) | `hud/mod.rs:1349` |
| plain UI image upload | `ui.add_graphic(Graphic::Image(...)) -> ImageId`, `ui.replace_graphic` — north-up map needs no `Rotations` | `voxygen/src/ui/mod.rs:212` |
| ground height (jump z) | `bastion::ground_z` — session's focus glide already re-rides terrain, so jump only needs XY | `voxygen/src/bastion/mod.rs:22` |

## 4. Design (what gets built)

New file `voxygen/src/hud/bastion_minimap.rs`:

- **`BastionMinimapTiles`** (engine, owned by Hud): chunk-key → `Tile { colors:
  [Rgba<u8>; 1024], heights: [i16; 1024] }` via KeyedJobs; dirty-set fed by
  `terrain_changes.modified_blocks` + slice-rev bumps; chunk-grid-anchored
  512×512 composite (16×16 chunks) with hillshade at blit; re-anchor when the
  camera-focus chunk nears the window edge; `maintain(client, ui, focus_xy,
  slice_z)` called from `Hud::update` ONLY when `bastion.active`.
- **`BastionMiniMap`** (conrod widget): worldgen underlay (north-up) +
  tile image with zoom crossfade + overlay pins + interactions
  (scroll = zoom steps, click = jump, drag = pan) + layer-toggle chips.
- **Pin/layer API (§3s foundation)**: `MinimapLayer { Colonists, Zones,
  Piles, Frustum, Alerts }` + `MinimapPin { wpos, kind: PinKind, color,
  emphasis }`; providers fill `Vec<MinimapPin>` per layer; documented in
  BASTION_ARCHITECTURE.md for territory/routes/dominion reuse.
- **Zoom model**: `zoom` = display px per block, log-stepped by scroll,
  clamped [world-fit … 4.0]; tile layer alpha fades 1→0 across the
  district→region band (worldgen map beneath at all zooms).

Touched files: `hud/mod.rs` (module reg, field, maintain, widget branch —
vanilla `MiniMap` drawn UNLESS `bastion.active`; two new `Event` variants
`BastionMinimapJump(Vec2<f32>)` / `BastionMinimapPan(Vec2<f32>)`),
`session/mod.rs` (consume the two events → `camera.set_focus_pos`; pass
slice_z into `bastion_sync`; designation colors via new helper),
`bastion/mod.rs` (`designation_color` helper). Nothing else — vanilla
paths untouched; flagless boot renders the vanilla minimap bit-identically.

## 5. Watch-items carried into build

- `--b5-scenario` timing-flaky under machine load; 4 concurrent sessions live
  in the primary tree — run gates when quiet, and this block is client-only
  (headless gates = regression only).
- Shared cargo target dir (`E:\veloren-master\target` via CARGO_TARGET_DIR;
  disk can't fit a second) — builds serialize against other sessions' cargo
  locks; a worktree build replaces `target/debug/veloren-voxygen.exe`
  (rebuild before any main-tree live test).
- Conrod widget-per-pin: fine at colony scale (dozens of colonists/piles);
  re-evaluate if §3s layers multiply pin counts.

## 6. Coordination hold (2026-07-09, architect directive)

Architect ordered the shared-tree sequence: B5.6b-1 merges/tags (delayed —
three eyeball fixes folded in), then B-ASSET1 builds+merges, THEN B-MAP1.
Machine stays quiet for Ben's live testing until then — my `cargo check` was
stopped mid-run (code compiles are UNVERIFIED as of this entry). All B-MAP1
work so far lives in the isolated worktree/branch; the shared checkout was
never touched. When the tree is ours: re-base `bastion/block-BMAP1` onto
post-BASSET1 `bastion/main` (commits are small + additive; expect trivial
conflicts in `voxygen/src/bastion/mod.rs` (designation_color addition) and
the append-only readme docs), THEN run the full gate.

**Directive inherited:** b-1 changes the shared `overlay_surface_z`/
`ground_z` sampler to FILTER to real terrain kinds (exclude tree
Wood/Leaves — it was climbing trees). B-MAP1's relationship to that fix,
recorded so nobody "fixes" the minimap into blandness later:

- The minimap does NOT call `ground_z`/`overlay_surface_z` anywhere. The
  tile scan is a top-down COLOR capture: canopy color and canopy height are
  INTENTIONAL there (a rendered top-down view shows trees; heights feed
  hillshade so trees read as relief lumps). That is display truth, not the
  placement bug class the b-1 fix targets — nothing is placed or draped at
  tile heights.
- Click-to-jump inherits the post-fix sampler automatically: the jump only
  sets focus XY and the session's existing focus glide re-rides the (fixed)
  `ground_z`.
- Rebase check: if the sampler signatures changed, B-MAP1 compiles clean
  anyway (zero direct uses) — verify at rebase.

## 7. Rebase plan for the merge slot (pre-solved 2026-07-09; order: b-1 -> B-MAP1)

Promotion: Ben moved B-MAP1 ahead of B5.6b-2; B-ASSET1 stood down. On the
architect's "tree is yours" ping, bastion/main = post-B5.6b-1 + a docs
commit. Read-only preview of b-1's branch (through 98ffc2b) determined every
conflict in advance:

- **Color authority collision (the real one):** b-1 added `tools::zone_rgb`
  (+ `zone_border_color`/`zone_fill_color`) and REMOVED the inline session
  color match — the same lines my `bastion::designation_color` refactor
  touched. Resolution: THEIRS WINS (landed first). Drop my designation_color
  commit content entirely; rework the minimap zone footprints to
  `tools::zone_rgb(kind)` + map-tuned alpha 0.32. One legend, zero drift —
  and their Stockpile tweak ([0.85,0.35,0.95]) flows to the map for free.
- **session/mod.rs `bastion_sync_designations`:** take theirs wholesale
  (fills + labels rewrite); my only surviving session edits are the
  bastion_sync(+slice_z) call and the two minimap event handlers.
- **hud/bastion.rs:** both add a field after `radial` (their zone_labels, my
  slice_z) — keep both.
- **hud/mod.rs:** disjoint areas (their label drawing; my minimap branch) —
  textual adjacency only.
- **BASTION_ARCHITECTURE.md:** they took §2.9 — renumber mine to §2.10.
- **Append-only docs (BACKLOG/CONSISTENCY/RUN_LOG):** union both blocks,
  theirs first (chronological).
- **Line endings:** stray CRLFs already normalized (b-1's lesson).

After rebase: cargo check, exe build, live gate (§2 of the TEST doc), merge
--no-ff, tag bastion-block-BMAP1, ledger + run-log PASS.
