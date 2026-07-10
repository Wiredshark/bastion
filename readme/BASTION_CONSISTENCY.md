# Project Bastion — doc/code consistency audit (append-only)

Contradictions between the design doc / prior findings docs and what's
actually implemented, found while building each block. Not all of these are
bugs — some are intentional simplifications for the block's scope — but
each is a place a future reader could reasonably expect different behavior
than what's there. Append new findings; never edit or remove an old entry
even if a later block resolves it (note the resolution as a *new* entry
instead, referencing the old one).

## B5 — Work execution

- **Build material sourcing**: `readme/veloren-colony-rts-build-report.md`
  §B5 (line 647) says "a blueprint voxel is a ghost until a colonist hauls
  the required material (**ties into B6**) and completes construction" —
  i.e. the design doc assumes B6's hauling system exists before Build jobs
  can ever complete. B5 as actually shipped does **not** wait for B6: it
  uses a single-material stand-in (`common::bastion::BUILD_MATERIAL_ITEM`,
  a colonist must simply be *carrying* one unit in `Inventory`, sourced in
  the harness via a direct `bastion_give_colonist_item` injection, not any
  in-game hauling action) so the Done-when criterion ("a wall blueprint
  with materials present gets constructed... without materials, it stalls
  ...and raises a needs-materials job") could be verified without B6
  existing yet. This is a deliberate, scope-correct simplification — B5's
  design-doc section itself says "Touches: ... item spawning" not "requires
  B6" in its own Touches/Approach breakdown, only the one aside does — but
  it means anyone reading only the design doc's B5 section would expect
  Build to be unreachable/untestable until B6 lands, which is not the case.
  See `readme/BASTION_BACKLOG.md`'s B5 section for the plan to replace the
  stand-in with real hauling once B6 exists.
- **Build "ghost" visual**: the same design-doc passage describes a
  blueprint voxel as "a ghost until... construction, replacing ghost with
  real voxel" — implying a distinct, player-visible pending-construction
  render state. No such visual exists: a Build designation's target
  position renders as whatever it already was (typically empty air) right
  up until the job completes, at which point it snaps directly to the
  final solid block. There is no in-between "ghost/blueprint outline"
  state. Not covered by B5's own Done-when criteria (which are all
  headless/mechanical: "gets constructed," "stalls... and raises a
  needs-materials job" — no visual requirement), so this isn't a B5 gate
  failure, but it's a real gap between the design doc's description and
  what a player would see. Likely belongs to whichever block does
  designation-overlay polish (B9, per B4's findings doc §5 "per-job status
  render is B9's").
- **Chop = "fells a tree"**: design doc §B5 Done-when says "Chop
  designation fells a tree and yields logs," read most naturally as one
  designation → one whole-tree action. The actual implementation (carried
  from B4, unchanged in B5) generates one Chop job **per `BlockKind::Wood`
  voxel** in the painted region — for a real multi-block tree trunk this
  produces many jobs, several of which are unreachable by a ground-walking
  colonist with no climb/ramp model (see `readme/BASTION_BACKLOG.md`'s B5
  "ADD" entry on the base-interaction verb this actually needs). B4's own
  code comments and B5's harness test comments already flag this as a
  known, out-of-scope gap — this entry exists so it's also visible from
  the doc-consistency side, not just buried in test-geometry comments.

## B5.5 — Zone deletion + pile aggregation (2026-07-09)

- **Block prompt vs. Veloren reality (resolved in Bastion's favor, worth
  recording)**: the B5.5 prompt describes building "a single pile entity
  carrying a count" as if new machinery — Veloren's `PickupItem` already
  IS that (a `Vec<Item>` with exact `amount()` accounting and
  conservation-exact `try_merge`); it simply never fired for B5 drops
  because `should_merge` was `false` (an anti-DoS flag documented as
  "currently only used for inventory dropped items"). B5.5 is therefore a
  flag + persistence-class wrapper, not a new pile system. No doc change
  needed; noted so future readers don't go looking for a bespoke pile
  entity type.
- **Design doc §B5 "stone, wood, ore per block type — Veloren already maps
  block/sprite → loot"**: still unimplemented (B5 ships flat
  stones-for-any-block, logs-for-wood; see the B5 consistency entry).
  B5.5's aggregation is loot-type-agnostic, so the eventual per-block loot
  mapping slots in without touching pile code. Drift unchanged, flagged
  again only because B5.5 touched the drop path.
- **Prompt's "reuse an existing crate/heap-like object model if one
  fits"**: surveyed `comp::body::item::Body` — nothing heap-like exists
  (item bodies mirror item kinds). Tier-scaling the item mesh
  (`comp::Scale`) is the shipped stand-in; real heap meshes belong to the
  asset pipeline (flagged in the backlog, and exactly the kind of gap the
  concurrent asset-lab session's tooling is meant to fill — coordination
  point for the architect).

## B5.6 — Zone visuals (2026-07-09, PRE-BUILD SCOPE FLAG — block NOT started)

- **Prompt framing vs. actual scope (flagged for architect decision).**
  `readme/B5.6-zone-visuals-prompt.md` calls itself a "Small,
  almost-entirely-CLIENT-SIDE patch block." Reading the code, the block is
  really THREE tractable items + TWO that need net-new infrastructure:
  - **Tractable now** (existing seams): Part 1 *outline* draping — the
    photographed floating-overlay bug (`bastion_region_outline` in
    `voxygen/src/session/mod.rs:701` draws 4 flat `DebugShape::Line`s at
    `max.z + 0.15`, a single flat height → floats on slopes). Fix = sample
    `client.state().terrain()` height along each edge (ReadVol, already in
    scope) and emit conformed segments. Part 3 toggle (new `GameInput` +
    3-state enum). Part 4 pile tiers (extend `server/src/bastion_piles.rs`).
  - **Needs new infrastructure** (NOT a small patch):
    - Part 1 *fills* + Part 2 *volumetric* rendering: the debug pipeline has
      `Quad` internally so a translucent conformed-surface/volume shape is
      *feasible*, but it requires a NEW `DebugShape` variant that carries
      pre-conformed terrain-sampled geometry (the debug mesh builders in
      `scene/debug.rs` have no terrain access) + confirming the debug pass
      blends alpha. That is real rendering-infra work, not parameterization.
    - Part 2 *volume-SELECTION UX* ("drag footprint then scroll/vertical-drag
      to set depth level-by-level; precision numeric panel"): there is **no
      designation z-extent model** to drive. `common::bastion::Region` is
      just `min`/`max` `Vec3<i32>`; painting hardcodes `min.z - 2 .. max.z`
      (`session/mod.rs:789`). Depth-selection is net-new *interaction +
      data-model* work (arguably design, adjacent to B6 mine-zone depth),
      not a client visual tweak.
  - **The whole block's correctness is VISUAL** — its Done-when is
    screenshot-gated (drape-on-hill/pit, volume reads countable depth). That
    can only be verified by rebuilding voxygen (~6 min each) and looking;
    headless scenarios only prove "sim unaffected."
- **Suggested resolution (flagged for architect):** split B5.6 into
  **B5.6a** = outline draping (fixes the photographed bug) + visuals toggle
  + pile tier scaling (all tractable, fast, high-value), and **B5.6b** =
  conformed *fills* + *volumetric* zone rendering + the *volume-selection
  UX/z-extent model* (a real rendering+interaction block; the z-extent part
  may want a design pass first, and it overlaps the §3v mine-zone-depth and
  §3w boundary-overlay work). B5.6a can be built immediately on
  confirmation. Recorded, not auto-actioned — the runner did NOT unilaterally
  rescope-and-merge, since that would fake B5.6's Done-when.

## B5.6a — Zone visuals (2026-07-09, built)

- **Prompt "small patch" framing — resolved by the approved split.** The
  B5.6 pre-build flag (see the prior entry) is now actioned: B5.6a = the
  tractable subset (outline draping, visuals toggle, pile tiers), built this
  session; B5.6b = fills/volumes/volume-selection (z-extent model decided by
  the architect in `readme/B5.6-zone-visuals-prompt.md`). No remaining
  doc/code contradiction on scope.
- **`Scale`-as-visual reused, consistent with B5.5.** B5.6a's pile tier
  scaling uses the synced `comp::Scale` exactly as B5.5's basic version did
  (`server/src/bastion_piles.rs`). Note (not a contradiction, a caveat for
  the eventual asset pass): `Scale` also scales the entity's *physics
  collider*, so "pile visual scaling" isn't purely visual — a real heap
  mesh/body (asset pipeline) would separate visual size from collider.
  Recorded so B5.6b / the asset session don't assume Scale is cosmetic-only.
- **B5 scenario robustness (repo reality vs. run-log claims).** Prior
  run-log entries report `--b5-scenario` "3/3" / "13/13". Measured this
  session: that holds on a QUIET machine (B5.5-tag and B5.6a-branch both
  6/6), but the scenario drops to ~65% under CPU load (timing-sensitive
  colonist arrival). Not a contradiction in the code, but the "passes
  reliably" claim is load-dependent — flagged so future sessions run gate
  scenarios on a quiet machine and consider hardening the scenario
  (backlog). No auto-fix applied.

## B5.6b — Zone-management UI (2026-07-09, EXPLORE + scope recommendation)

- **Scope: B5.6b bundles ~4 blocks; recommend the architect tag sequenced
  sub-blocks so `main` advances incrementally.** The block as specified is
  the full zone-management UI (conformed fills + color-blend + labels; the
  z_extent data model; volumetric rendering + depth rings; volume-selection
  UX; clickable zones → radial Delete/Modify-depth/Edit-mode drag-handles;
  erase-by-type wire protocol). Each is a real subsystem. Feasibility is
  CONFIRMED (debug pipeline alpha-blends → conformed fills need no new
  pipeline; z_extent job-gen is a bounded per-cell-surface refactor of the
  existing triple loop) and a full verified build plan + z_extent design is
  in `docs/BASTION_B5.6b_FINDINGS.md`. Recommended split (each merges/tags):
  **b-1** fills+colors+blend+labels+SUBTLE (client-only, low-risk, the
  headline visual); **b-2** z_extent model + volumetric + volume-selection
  UX (also fixes B5.MINE-COVERAGE — surface-relative z closes the
  slope-coverage gap); **b-3** zone interaction/edit-mode; **b-4**
  erase-by-type. Not auto-actioned — the runner did NOT self-rescope-and-tag
  (that would fake B5.6b's Done-when); flagged for the architect to formalize
  (as the B5.6→B5.6a/B5.6b split was). `bastion/block-B5.6b` branch holds the
  run-log start + findings/plan; `main` untouched at `bastion-block-B5.6a`.

## B5.6b-1 — Zone fills (2026-07-09, built)

- **Prompt's "new DebugShape variant carrying pre-conformed geometry" —
  implemented as specified.** `docs/BASTION_B5.6b_FINDINGS.md` predicted the
  debug pass alpha-blends (`BlendState::ALPHA_BLENDING`) and that a conformed
  geometry needs terrain access the debug mesh builders lack; both held.
  `DebugShape::ConformedTris` carries session-sampled draped triangles;
  colour+alpha via `set_context`. No new pipeline needed — matches the
  finding.
- **Caveat recorded (not a doc contradiction, a v1 fidelity note):** fills
  are LIT by the debug frag shader (they're not a flat UI tint). See backlog.
  The spec says "terrain-conformed tinted fill" — the lit look satisfies
  "conformed + tinted"; a perfectly flat tint would need an unlit overlay
  path. Flagged so B5.6b-2/asset work knows the debug pass lights overlays.
- **z_extent NOT touched in b-1** (correct per the split — b-1 is surface
  fills only; z_extent + volumetric is b-2). The fills use the existing
  designation footprint (`Region` XY); no data-model change. Consistent.

## B5.6b-1 — post-verify note (2026-07-09)

- **Queue root-cause list vs. shipped fix (bug 1):** the re-cut queue's
  description of the overlay bug also names "slice-clamp direction" and
  "rebuild not triggered on view-mode change" as roots. The shipped fix
  addressed the canopy walk-down (Wood/Leaves via is_filled — the §5
  gotcha); slice/rev/visuals rebuild triggers already existed from B5.6a.
  Ben's eyeball re-verify PASSED, so no open bug — but note: a V-cycle
  view-MODE change does not itself rebuild overlays (they depend on the
  mode only via slice_z). If mode-coupled overlay staleness reappears,
  add view-mode to the rebuild triggers (one field in the sync check).
  Watch item, flagged not fixed.

## 2026-07-09 — B-MAP1 (overseer minimap)

- **Prompt vs build (recorded drift, not auto-corrected):**
  `readme/B-MAP1-overseer-minimap-prompt.md` prescribes GPU render-to-texture
  per chunk via the B1 ortho camera. Verified against the repo: voxygen's
  conrod UI consumes CPU images only (`ui/graphic/mod.rs` atlas), so literal
  RTT needs a bespoke offscreen wgpu pass PLUS readback anyway; meanwhile the
  vanilla `VoxelMinimap` already implements the per-chunk-tile +
  trickled-jobs + terrain-edit-invalidation architecture the technique
  actually wants. Built CPU voxel-scan tiles + hillshade on that seam (reuse
  rule §2a); the block gate is outcome-based and met. Suggested resolution:
  architect either blesses CPU tiles as standing approach or queues an RTT
  upgrade block (backlog entry exists).
- **Prompt vs repo:** the prompt says tile invalidation is "shared with
  B5.6a's draping cache — same trigger". In the repo, B5.6a's draped
  overlays have NO terrain-edit invalidation (known watch-item "overlay
  terrain-edit restaling" in the run log); they rebuild on
  designation-rev/slice/visuals change only. The shareable mechanism is the
  client-side `TerrainChanges` stream, of which the minimap is now the FIRST
  consumer. Flagged for the architect; backlog entry restates the overlay
  gap.

## 2026-07-09 — B5.6b-2 (z_extent + volumetric + volume-UX)

- **Spec vs build (recorded drift, deliberate):** the spec's volumetric
  render ("top + subtle walls + depth rings", "volume reads correctly on a
  slope") is built as ABSOLUTE-z rings over the echoed AABB + corner
  posts, not per-column surface-following rings. Rationale in the findings
  AS-BUILT §design-decision: per-column rings re-sampled at overlay
  rebuild would sag into the dig mid-excavation (the sampler reads CURRENT
  terrain); the AABB matches cancel/erase box semantics and stays put.
  Cost: on slopes the ring stack shows the volume envelope (relief +
  depth). PREVIEW rings during the drag ARE per-column (sampled fresh).
  Architect: bless or queue the per-column upgrade (backlog entry exists).
- **Docs drift, resolved by schema guard:** future-work §3e/§3m/§3q/§3z
  carry 7/8/9-kind purpose lists; frameworks §2's 8-kind list is now
  LOCKED as `common::bastion::Purpose` with a unit test naming §2 the
  edit-first authority. Purpose is live but unconsumed until B6 —
  consistent by construction hereafter.
- **Terminology note:** "N levels deep" counts SURFACE-INCLUSIVE levels
  (down=2 → "3 levels deep" = surface + 2 below), matching the volume the
  zone actually claims; the wire `ZExtent{down,up}` counts offsets. One
  label authority: `Tools::z_extent_label`.
