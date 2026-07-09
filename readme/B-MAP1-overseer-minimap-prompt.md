# B-MAP1 — The Overseer Minimap (rendered top-down tiles, WoW-addon style)

> **For Ben:** paste into a game-build session (independent, client-side; any time after B5.6 — it reuses
> the overlay-draping/overlay infrastructure). Standard protocol: branch → build → test → merge → tag
> `bastion-block-BMAP1`.

## WHY
The vanilla minimap is designed for a third-person RPG player: tight zoom, low detail, player-centric. The
overseer needs a GOD'S minimap — and eventually the §3s "map IS the interface" layer starts here. The
technique is the proven WoW-addon approach: **render the real world top-down into cached tiles, composite a
zoomable pyramid, draw overlays on top.**

## Part 1 — Rendered tile pyramid
- **Near zoom (the colony):** render actual terrain top-down into per-chunk tiles — orthographic,
  straight-down captures (the B1 ortho camera machinery is EXACTLY this; render-to-texture per chunk).
  Real voxel imagery: you see your actual buildings, trees, dig sites — not color blobs.
- **Cache + invalidate:** tiles cached; re-render a chunk's tile ONLY on terrain-edit events under it (B5
  already emits these — the B5.6 draping cache uses the same trigger; share the mechanism). Budget:
  re-renders trickled, never hitching the frame.
- **Far zoom (region/world):** below a zoom threshold, blend to the existing worldgen map data (Veloren
  has a world map) — near tiles for detail, worldgen for scale. Two-tier, seamless-enough crossfade.
- **Zoom control:** scroll on the minimap steps through levels (colony ~few chunks → district → region →
  world); a size toggle (small/large/fullscreen map view can come later — minimap first).

## Part 2 — Overlays (the god's information layer)
On top of the tiles, toggleable pins/layers (respect the B5.6 ON/SUBTLE/OFF philosophy):
- **Colonists** — dots (color by state later; dots now), selected colonist highlighted.
- **Designations/zones** — tinted footprints matching their in-world colors (reuse zone data straight;
  this is the draped overlay's 2D projection).
- **Piles/stockpiles** — small markers.
- **Camera frustum** — a rectangle showing what the main view sees (the classic RTS minimap element).
- **Alerts** (hook for later: threats, breaches) — the §3s icon-language starts as these pin types; keep
  the pin API open (future: boundary contour §3w, territory, trade routes).
- **Click-to-move:** clicking the minimap jumps/pans the god camera there. Drag = continuous pan. This is
  the single biggest usability win — the minimap becomes navigation, not decoration.

## GATE
- Minimap shows real rendered terrain at colony zoom (screenshot: buildings/trees recognizable); zoom
  steps to worldgen scale smoothly; tile re-render on dig (mine a patch → minimap updates within seconds,
  no hitch); overlays toggle; click-to-jump works; overlay positions accurate against the world (paint a
  zone, its minimap footprint matches). Vanilla flagless boot: untouched vanilla minimap. Tag + bookkeeping.

## WATCH-ITEMS
- Tile render cost/format (resolution per chunk tile — pick pragmatic, e.g. 2–4 px/block near zoom;
  document).
- Underground: when the camera/Z-slice is below ground, the minimap should follow (render the slice level,
  not the surface) — if nontrivial, ship surface-only and backlog slice-aware tiles (the mining framework
  will want them).
- This block founds the map/overlay layer — log the pin/layer API for §3s consumers (territory, routes,
  dominion) in the architecture doc.
