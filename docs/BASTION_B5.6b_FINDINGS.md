# B5.6b findings + build plan — the zone-management UI

Spec: `readme/B5.6-zone-visuals-prompt.md` §B5.6b. Explored 2026-07-09 on
`bastion/block-B5.6b` (start `7e6e119`, off `bastion-block-B5.6a`).

**Status: EXPLORE done, feasibility confirmed, build plan written. NOT yet
built — B5.6b is a large multi-subsystem "its own session" block (architect's
words); this was a long continuation thread, so the disciplined call was to
de-risk with a verified plan and leave `main` green rather than start a
build that couldn't be completed+gated cleanly.** The next fresh-budget
session builds from this plan. Recommend building as the sequenced
sub-blocks below so `main` advances incrementally (each merges/tags on its
own) instead of one giant unmergeable branch.

## Confirmed feasibility (the two things that were in doubt)

1. **Translucent conformed FILLS are buildable — no new render pipeline.**
   The debug pipeline already alpha-blends:
   `voxygen/src/render/pipelines/debug.rs:138` —
   `blend: Some(wgpu::BlendState::ALPHA_BLENDING)`. So a new `DebugShape`
   variant carrying pre-conformed triangle/quad geometry (built by a helper
   WITH terrain access, in the session where `client.state().terrain()` is
   available) renders translucent fills directly. The debug mesh builders in
   `scene/debug.rs` have no terrain access, so the geometry must be sampled
   in the session (like B5.6a's `draped_rect_outline`) and passed in as a
   vertex list — hence a `DebugShape::Mesh(Vec<[Vec3<f32>;3]>)`-style variant
   (or a `ConformedFill(Vec<quad>)`), colored via the existing per-shape
   context color (`set_context`, which carries an RGBA — alpha already flows).
2. **z_extent job-gen is a bounded per-cell refactor.**
   `server/src/bastion_jobs.rs::place_designation` is a triple loop
   `for z in region.min.z..=max.z { for y { for x { predicate }}}`. The
   z_extent model changes this to: `for (x,y) in footprint { surface =
   find_surface(terrain, x, y); for z in surface-down ..= surface+up {
   predicate }}`. `place_designation` already has `&TerrainGrid`. This also
   fixes B5.MINE-COVERAGE's likely cause (the current plane-relative z-band
   misses blocks on slopes — surface-relative resolves per column).

## The z_extent data model (architect-decided; the schema to lock)

- `common::bastion`: add `struct ZExtent { down: u16, up: u16 }` (serde,
  Copy). Surface-relative. Put it on the wire + on the stored designation.
- **Where it lives on the wire:** `ClientGeneral::BastionPlaceDesignation`
  currently carries `{ region, kind }`. Change to carry the **XY footprint**
  (a `Region` with the z-range meaning "hint plane", or better an explicit
  `footprint: Aabr<i32>` + `surface_hint: i32`) + `z_extent: ZExtent`. The
  server resolves per-cell. Keep `Region` for the echo/overlay (the client
  already stores `Vec<(Region, DesignationKind)>`); the stored Region's z can
  be the resolved bounding z (min surface-down .. max surface+up) for
  overlay/erase AABB math, with the per-cell truth re-derivable.
- **Defaults preserve current semantics:** default `ZExtent` per kind so a
  flat paint matches today — Mine `{down: 2, up: 0}` (≈ current `plane-2..
  plane` on flat ground), Chop/Stockpile/Build `{down: 0..2, up: 0}`. On
  slopes it now correctly follows the surface (the intended fix, not a
  regression). Existing harness scenarios call
  `Server::bastion_place_designation(Region, kind)` directly — KEEP that
  Region-based entry (it becomes "explicit volume, no surface resolution")
  and ADD a footprint+z_extent entry the in-game paint uses. That way the
  B4/B5/B5.5 scenarios are untouched (their hand-built geometry still works)
  and only the client paint path gains z_extent. (B5.8's gate will later
  remove the hand-patched scenario geometry once vertical mobility lands.)

## Sequenced sub-block plan (recommend the architect tag each)

**B5.6b-1 — conformed FILLS + colors + blend + labels + SUBTLE** (client-only,
low-risk, the headline RimWorld-colored-zones visual Ben asked for; NO data
model needed — fills are surface-draped over the existing footprint):
- New `DebugShape` variant for pre-conformed translucent geometry + a
  `bastion::draped_fill(terrain, footprint, slice_z, hover)` helper (the
  reusable overlay-renderer utility; `overlay_surface_z` stays the one height
  authority). Emit a draped quad per footprint cell.
- Zone-type colors (Mine/Chop/Build/Stockpile each a legend color); overlap =
  order-independent color blend (alpha compositing handles this naturally if
  each zone's fill draws with moderate alpha).
- Centroid labels ("Mine 2") — camera-facing, distance-scaled. Check the HUD
  for a world-anchored text mechanism (nametags use one — `scene`/`hud`
  overhead text); reuse it.
- Wire into `bastion_sync_designations`: ON = fill+border+label,
  SUBTLE = border only (already draped from B5.6a), OFF = nothing.
- Gate: hill/pit colored fills screenshot; overlap blend; labels; SUBTLE=border
  only; B4/B5/B5.5 green (client-only → automatic); vanilla clean.

**B5.6b-2 — z_extent model + VOLUMETRIC rendering + volume-selection UX:**
- Land the `ZExtent` schema + footprint+z_extent paint path + per-cell server
  resolution (above). Headless: assert defaults reproduce current job counts
  on flat ground + correctly cover a slope (this doubles as the
  B5.MINE-COVERAGE fix + coverage assertion).
- Volumetric render: top face + subtle walls + depth rings every N levels
  (countable), clip with Z-slice. Reuse the conformed-geometry helper.
- Volume-selection UX: scroll/hold-drag sets depth with live ring preview +
  "N levels" counter; precision numeric field; both synced.

**B5.6b-3 — zone INTERACTION (the management layer):**
- Click-select a zone (pick against stored footprints — reuse
  `unproject_to_world_plane` / the block pick; test point-in-footprint).
- Right-click selected → radial: Delete (B5.5), Modify depth (reopens the
  b-2 depth UX, live preview, claims released/created per the erase rules),
  Edit mode.
- Edit mode: drag handles at footprint edges/corners; live conformed preview;
  commit/cancel; shrink releases claims (proven B5.5 AABB subtraction /
  B5.6a `clip_xy`), grow generates jobs; same-cycle claim-release assertion.

**B5.6b-4 — ERASE-BY-TYPE** (wire-protocol kind filter, promoted from B5.6a):
- `ClientGeneral::BastionCancelDesignation` + `ServerGeneral::
  BastionDesignationRemoved` gain `Option<DesignationKind>`; `cancel_region`
  filters by kind; client subtraction filters by kind; Erase tool UI gains
  the filter selector. Gate: overlapping Mine+Chop, filter=Chop → only Chop
  erased, Mine untouched.

## Reused seams (verified this session / carried from B5.6a)

- Overlay draping: `voxygen/src/bastion/mod.rs::draped_rect_outline` +
  `overlay_surface_z` (the height authority). Fills reuse the same sampler.
- Overlay sync + rebuild triggers (rev/slice/visuals dirty):
  `session/mod.rs::bastion_sync_designations`.
- Client designation store: `client/src/lib.rs` `bastion_designations:
  Vec<(Region, DesignationKind)>` + `bastion_designations_rev`.
- Radial menu: `voxygen/src/hud/bastion.rs` `RadialAction` + the session's
  `bastion_open_radial` / `HudEvent::BastionRadialPick` handler.
- Erase AABB math: `common::bastion::Region::{subtract, clip_xy}` (unit-tested).
- Debug shapes: `scene/debug.rs` `DebugShape` + `add_shape`/`set_context`
  (RGBA context color, alpha honored) / `remove_shape`; pipeline blends.
- `VisualsMode` (On/Subtle/Off): `voxygen/src/bastion/tools.rs`.

## Gotchas for the builder

- Fill cost: one draped quad per footprint cell × zones. Cache like the
  outline (rebuild on rev/slice change); a large quarry is hundreds of quads
  but they're tiny and static. Batch into ONE mesh shape per zone if the
  per-shape overhead bites.
- Labels: world-anchored text must hide in SUBTLE/OFF and not fight the
  conrod HUD; find the existing overhead-nametag path rather than a new one.
- z_extent surface-relative is ALSO the B5.MINE-COVERAGE fix — coordinate:
  building b-2 likely closes that investigation. Don't double-build.
- Keep the Region-based `bastion_place_designation` for the harness; add the
  footprint+z_extent path for the client (don't break the 9/9 scenarios).
