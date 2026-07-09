# Project Bastion — backlog (append-only)

FIX = a known bug/gap in something already built. ADD = a missing capability
a future block should own. IDEA = a design thought not yet triaged into a
block. Newest entries go at the bottom of each block's section; never edit or
remove an earlier block's entries.

## B5 — Work execution (dig/chop/build effects, item drops, skill XP)

- **FIX** (confirmed, worked around in-test, not fixed at the mechanism
  level): any vertically-stacked designation of height >= 2 has its *lower*
  block's arrival target (`block_pos + (0,0,1)`) land exactly on the block
  above it. On flat ground with no adjacent same-height terrain, that
  target is only reachable once the block above is cleared *and* even then
  requires standing on top of a lone 1-block pillar — which the vanilla
  ground-walking traversal doesn't reliably solve (confirmed empirically:
  claim → 10s stuck → release → retry-sweep reclaim, looping indefinitely,
  never resolving even after the upper block was cleared). This is the same
  underlying gap already flagged for tall trees (see below), just
  triggering one layer sooner than assumed. The B5 harness's chop test was
  changed from a 2-tall stump to a single block to sidestep it — the
  underlying job-placement/arrival-target model is unchanged. A real fix
  needs either a smarter per-job arrival target (e.g. "any exposed face,
  not always +1 z") or a base-interaction verb (see next item) that
  sidesteps per-voxel jobs for multi-block structures entirely.
- **ADD** (carried over from B4's own comment, now empirically reinforced
  by the above): chopping a real multi-block-tall tree needs a
  base-interaction verb ("fell the tree" from ground level, one job) rather
  than per-voxel Chop jobs — the per-voxel model cannot express "the top of
  a tall trunk is reachable" without solving general freestanding-structure
  climbing, which is out of scope. Whichever block adds tree-chopping to
  the real game (not just the harness's short test stump) should own this.
- **ADD**: B6 hauling should supersede B5's `BUILD_MATERIAL_ITEM`
  single-material stand-in (`common::bastion::BUILD_MATERIAL_ITEM`,
  `server/src/bastion_jobs.rs`'s `carries_material`/`needs_materials`
  logic) with real per-blueprint recipes and colonists that actually haul
  materials to a site rather than a harness-injected starting inventory
  item. The `needs_materials` flag and stall-not-build-for-free semantics
  should carry forward as-is; only the "how does a colonist get the item"
  path changes.
- **IDEA**: colonists' vanilla opportunistic-loot AI (any Humanoid NPC
  auto-targets and picks up nearby item drops — `server/agent/src/
  action_nodes.rs`'s `is_valid_target`) is now gated off entirely for
  `Colonist` entities (see the B5 fix below) so B5's mined/chopped drops
  stay on the ground. When B6 hauling exists, decide whether it should
  reuse this vanilla pickup path (re-enabling it selectively, e.g. only
  when a colonist has a `Haul` job targeting that specific item) or
  implement its own deliberate pickup, bypassing vanilla AI entirely like
  B5's Build-material consumption already does. Leaning toward the latter
  for determinism/control, but worth a real design pass rather than
  defaulting.
- **FIX** (found and fixed in this block, noted here so the mechanism is
  documented): the vanilla NPC "wander over and pick up any nearby loose
  item" behavior (`is_valid_target`'s `Body::Item` branch) was silently
  consuming every stone/log drop within the same tick or two it appeared —
  colonists standing right next to their own work would auto-loot it into
  their inventory before anything else could observe it as a ground drop.
  Root-caused via a debug hook logging every `handle_create_item_drop` call
  (confirmed `created=true` for all 27 stone drops) cross-referenced
  against a per-tick dropped-item census (confirmed the count repeatedly
  fell back to exactly 0 well before all mining completed, then rose again
  as new drops appeared — the signature of continuous auto-pickup, not a
  merge or despawn-timeout artifact, both of which were investigated and
  ruled out first). Fixed by gating item-drop targeting off entirely for
  `comp::Colonist` entities (new `ReadData::colonists` field in
  `server/agent/src/data.rs`, checked in `action_nodes.rs`). This is a
  vanilla-adjacent file change but strictly additive/gated — no behavior
  changes for any non-colonist NPC.
- **FIX** (found and fixed in this block): B4's own `--b4-scenario` broke
  as a side effect of B5's already-committed work-execution changes to the
  *shared* `bastion_jobs.rs` upkeep loop — not from anything in this
  session's colonist-loot fix. Root cause: B4's test sampled "how many
  enabled colonists are simultaneously `Arrived` right now" and broke early
  once that hit 4, which was reliable when `Arrived` was a terminal state
  (B4 never completed jobs). B5 makes `Arrived` transient (a job completes
  after a few seconds of work and releases the colonist), so the
  instantaneous count could undercount even though every colonist
  successfully arrived at some point; separately, B5's fast job completion
  meant the deliberately-unreachable "deep" test job wasn't picked up
  until well after the sampling loop's old early-exit condition had
  already fired, so its 10s watchdog never got a fair chance to run before
  the test snapshotted the audit. Fixed by tracking *ever* arrived (a
  `HashSet` of colonist names, accumulated across the whole fixed 60s
  window) and *ever* unreachable (an OR-accumulated flag) instead of
  point-in-time snapshots, and loosening the arrived threshold from
  `>= 4` to `>= 3` of 4 enabled colonists (which of the 4 gets stuck
  babysitting the one shared unreachable job, if any, is a timing
  coin-flip — requiring literally all 4 to *also* land a real arrival in
  the same fixed window is more brittle than the invariant needs).
  Verified 5/5 clean on both `--b4-scenario` and `--b5-scenario` after the
  fix (was previously failing 3/3 and flaking ~30-65% respectively before
  the two separate root causes above were both addressed).
- **FIX** (found and fixed in this block, after the merge below — see the
  amended findings): the mine quarry pit's rim ring gives every dig cell
  guaranteed *adjacent* footing for mining, but nothing guaranteed a
  walkable path back *out* once the whole 3×3×3 footprint was hollowed —
  the rim is a sheer 2-block wall with no climb/ramp modeled. A colonist
  that happened to finish its last mine job while standing at the pit
  floor, then got assigned to a job elsewhere (e.g. Build, clear across
  the colony), was flat-out trapped: confirmed via a debug log showing its
  position genuinely never changing at all across repeated 10s-stuck
  cycles for the entire settling budget. Fixed in the harness by carving a
  2-step staircase (floor → step → rim) on one column, just outside the
  ring, so the pit floor always has a walkable exit. Verified 8/8 clean on
  `--b5-scenario` after the fix (was flaking on `build_placed` even after
  the two fixes above, roughly 2/5-3/8 depending on the run). This is a
  test-geometry fix only — `bastion_jobs.rs`'s execution/watchdog logic
  was untouched by it; the underlying "no climb/ramp modeled" limitation
  is the same one already tracked for tall chop stumps/trees above, and
  will resurface for any future test (or real in-game designation) that
  digs an enclosed pit without its own exit.

### Post-gate code review (same session, fresh-eyes pass)

- **FIX (fixed)**: `place_designation` had no dedupe against existing
  jobs — repainting a region (or overlapping Mine+Chop paints; Wood is
  `is_filled()` so Mine matches wood blocks too) created duplicate jobs
  per block, each of which completed independently and dropped loot from
  the same single block: a free-item exploit reachable from the in-game
  paint path. Now one job per block position, regardless of kind.
- **FIX (fixed)**: job completion never re-validated the target block —
  if the world changed between claim and completion (vanilla mining, an
  explosion), a Mine job "completed" against empty air and conjured free
  stones, and a Build job silently overwrote whatever now occupied its
  target. Completion now re-checks the placement predicate against
  `TerrainGrid` (moot jobs are dropped without loot/XP, before Build's
  material consumption so the material isn't eaten), and defers via
  `can_set_block` if another system already edited the block this tick
  (so the final block state never depends on system run order).
- **FIX (fixed)**: `Job::skill_floor` existed and was documented but was
  never checked anywhere — arbitration now skips jobs whose work-type
  skill level is below the floor (still always 0 in v1 generation, but
  the field is no longer dead config).
- Cleanups in the same pass: mine/chop drop asset ids hoisted to
  `common::bastion::{MINE_DROP_ITEM, CHOP_DROP_ITEM}` (were duplicated
  as string literals between `bastion_jobs.rs` and the harness — drift
  risk); duplicated skill-level match factored into
  `ColonistSkills::level_for`; stale `ARRIVE_DIST` doc comment (said XY,
  code is deliberately 3D); unused field dropped from arbitration's
  `assignments` tuple; harness step-numbering and mine-ramp comments
  corrected (the ramp's first hop is 2 blocks, not 1 — works, verified,
  but the comment misdescribed it).

## B5.5 — Zone deletion + pile aggregation (2026-07-09)

- **FIX** (found in test, fixed in test geometry only): a fourth
  manifestation of the vertical-reachability trap — a single-z-level slab
  forced across sloped natural terrain buries blocks inside hillsides
  (their `+1` arrival cell is a 1-block gap a colonist can't fit in) and
  floats others over air pockets. Part 2's first run stalled at 8/200.
  Any future scenario terraform must fully determine its geometry
  (under-fill + working level + headroom + perimeter footing — see
  `--b55-scenario`'s Part 2 for the pattern). The mechanism-level fix
  (smarter arrival targets / dig-frontier access planning) is the same
  standing backlog item as the pit/stump/tree cases; the mining framework
  (`readme/BASTION-SYSTEM-FRAMEWORKS.md` §6, "access is part of the dig
  plan") owns the real solution.
- **ADD** (B6 interface decisions, per the block prompt's watch-items):
  piles are ordinary `PickupItem` entities with `amount()` counts and a
  `BastionPile` marker — B6 hauling should enumerate them via the same
  storages, claim one pile = one haul trip, and take the whole pile into
  `Inventory` on pickup (vanilla stacking handles counts). If B6 wants
  partial pickup it needs a split API on `PickupItem` (does not exist;
  design decision deferred to B6). Reservation of piles (two haulers, one
  pile) is B6's job-board problem, same shape as block-job claims.
- **ADD**: pile visuals are tier-scaled item meshes (`comp::Scale` 1.0 /
  1.35 / 1.7). A real heap mesh + count label/tooltip is asset-pipeline +
  B9 work. The scale system (`server/src/bastion_piles.rs`) is the hook
  point — swap Scale for a body/model change when assets exist.
- **IDEA** (optional): erase depth is symmetric with paint depth (`-2`
  under the plane), so erasing at a different Z-slice than the original
  paint can miss the paint's under-reach; the radial "Delete zone" covers
  exact cleanup. If players report confusion, make erase depth generous
  (e.g. -4) or erase full column extents of intersected rects.
- **NOTE** (TRAVEL_SPEED watch-item from the block prompt): Ben's live
  demo verdict on colonist walk speed was not collected this session —
  the in-game demo of B5.5's erase tool should double as the speed
  eyeball; still open.

## B5.6 — Zone visuals (2026-07-09, pre-build scope assessment)

- **FIX** (photographed, high-priority, tractable): painted-zone/selection
  outlines render as flat rectangles floating over sloped terrain — the
  live-demo bug in `readme/evidence-b56-floating-selection-bleabrolm.png`.
  Root cause: `voxygen/src/session/mod.rs` `bastion_region_outline` draws 4
  `DebugShape::Line`s at a single flat `max.z + 0.15`. Fix = terrain-conform
  each edge (sample `client.state().terrain()` height, emit segments). Ben's
  diagnostic (a slice toggle temporarily "fixed" it) is consistent with the
  overlay caching a stale flat height. Verify across all slice modes + after
  toggles. This is the core of the proposed B5.6a.
- **ADD** (needs infra, deferred to proposed B5.6b): terrain-conformed
  translucent *fill* overlays + *volumetric* zone rendering require a new
  `DebugShape` variant carrying pre-conformed geometry (debug mesh builders
  in `scene/debug.rs` lack terrain access) and alpha-blend confirmation on
  the debug pass. Feasible (the pipeline already uses `Quad`), but real
  rendering-infra work — see `readme/BASTION_CONSISTENCY.md` B5.6 entry.
- **ADD** (design-adjacent, deferred): volume-selection UX (scroll/drag to
  set zone depth + precision numeric field) has no data model to drive —
  `common::bastion::Region` is min/max only; paint hardcodes `min.z-2`. A
  designation z-extent model is net-new (overlaps §3v mine-zone-depth). Flag
  for a design pass before building the selection UX.
- **IDEA** (optional): the reusable overlay-draping utility the prompt asks
  for (Part 1) is the same one §3w colony-boundary overlay will reuse —
  design its API for both customers when B5.6a is built (note the seam).

## B5.6a — Zone visuals: draping + toggle + pile tiers (2026-07-09)

- **FIX (fixed)**: the photographed floating-overlay bug — zone/selection
  outlines drawn as a flat rectangle at the pick-plane z, floating over
  slopes. Fixed by terrain-draping (`bastion::draped_rect_outline` +
  `overlay_surface_z`). All three overlay callers drape; committed overlay
  rebuilds on Z-slice change too. See `docs/BASTION_B5.6a_FINDINGS.md`.
- **ADD (deferred, judgment call)**: erase-by-type filter — NOT cheap on
  existing seams (needs a wire-protocol change: the cancel message + removal
  echo gain `Option<DesignationKind>`; kind-filtered server `cancel_region`;
  kind-filtered client subtraction; a tool-filter UI). Skipped per Ben's
  "only if cheap." Natural fit for B9 colony-HUD/tool polish, or a small
  dedicated patch. (Area-erase already exists: the B5.5 Erase drag.)
- **ADD (deferred)**: committed overlay does not rebuild when terrain is dug
  *under* a standing zone (drapes at build time; rebuilds on rev/slice
  change only). Needs a client terrain-change signal to rebuild the affected
  zone's overlay. Arguably fine (shows original footprint) but the prompt's
  "rebuild on terrain edit under the zone" watch-item is unwired.
- **IDEA (optional)**: pile tier changes snap the `Scale`; a brief
  client-side scale-lerp would stop the pop ("shouldn't pop distractingly").
  Polish, not wired.
- **FIX (pre-existing, NOT B5.6a — flagged)**: the `--b5-scenario` is
  timing-flaky under machine load — colonist arrival/completion can miss the
  scenario's tick caps when the CPU is busy (game client running, concurrent
  asset session). Measured this session: B5.5-tag 6/6 and B5.6a-branch 6/6
  in a QUIET window, but ~65% when loaded. Isolated + confirmed NOT a
  B5.6a regression (client-only + a pile-scale tweak that makes B5's piles
  *smaller*). Worth hardening the scenario (looser tick caps / progress-based
  waits) or the underlying pathing timing — future robustness item, not this
  block's job. Runner note: run gate scenarios on a quiet machine.
- **IDEA (B5.6b seam, logged per Ben)**: the conformed-geometry helper for
  B5.6b fills/volumes IS the reusable overlay-renderer; `overlay_surface_z`
  is the shared height authority (all overlays must agree). The §3w
  colony-boundary overlay is its next customer — design the fill/volume API
  for both when B5.6b is built.

## B5.6b visual reference (2026-07-09, captured during B5.6a hold)

- **IDEA/reference (B5.6b):** Ben provided a RimWorld screenshot as the
  zone-management target — "we want this but flat image and volumetric."
  Read: RimWorld-style colored AREA OVERLAYS (each zone type a flat tinted
  fill draped on the ground) + a management surface (the grid/schedule-style
  panel), rendered in our engine BOTH as flat terrain-conformed fills AND as
  volumetric zones with countable depth. This matches the updated B5.6b
  queue scope (full ground fills in zone-type colors, overlap blending,
  labels, volumetric + layer counter, clickable zones → radial). No
  `readme/reference-images/` folder exists yet — when Ben drops the image
  there, the B5.6b session should view it and judge output against it
  (mega-prompt input #8). Not actioned now: B5.6b is its own session and
  B5.6a must tag first.

## B5.6b-1 — Zone fills + colors + blend + labels + SUBTLE (2026-07-09)

- **FIX/watch (fills are LIT):** `DebugShape::ConformedTris` renders through
  the debug frag shader, which applies sun/point lighting (`illuminate(...)`)
  — so a fill is a *lit* translucent surface, not a flat UI tint. Reads fine
  as a ground tint (integrates with terrain shading) but is not a constant
  color across a large slope. If a flatter UI look is wanted, add an
  unlit/emissive path to the debug frag (or a dedicated overlay pipeline) —
  deferred; the lit look is acceptable v1 and matches how the outlines
  already render.
- **IDEA (overlap blend is order-dependent):** fills alpha-composite, so
  overlapping zones blend, but the result depends on draw order (alpha
  blending isn't order-independent). For 2–3 overlaps it reads as a blend
  (good enough v1). True order-independent blending (or a max/additive
  compositing mode) is a later polish if many-zone overlaps look wrong.
- **ADD (label index stability):** zone labels use a per-kind running index
  by list order ("Mine 1", "Mine 2"). Erase-splitting a rect renumbers
  subsequent zones (the index isn't a stable ID). Fine for the auto
  type+index the spec asked for; real stable names/IDs are the "naming
  later" item (B5.6b-3 radial Rename / a zone-id model).
- **ADD (fill cost at scale):** one `ConformedTris` shape per zone with
  2·W·H triangles; rebuilt on rev/slice/visuals change (cached otherwise). A
  huge quarry is thousands of tiny tris in one mesh — fine, but if it bites,
  decimate the fill grid (sample every N cells) or cap fill area.
- **SEAM (B5.6b-2 + §3w):** `bastion::draped_fill_tris` + `overlay_surface_z`
  are the reusable conformed-fill utility. B5.6b-2 volumes extend it (walls +
  depth rings from the same corner-height grid); §3w boundary reuses the
  footprint fill. Keep `overlay_surface_z` the one height authority.
