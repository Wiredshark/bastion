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

## 2026-07-10 — B5.8 (vertical mobility)

- **Spec vs build (recorded drift, sanctioned):** the spec's scramble rode
  "the existing climb machinery"; as built, colonist vertical execution is
  SERVER-ASSISTED (position-driven lift + ledge snap + dismount rules in
  `bastion_jobs`) because the vanilla jump→wall-contact→Climb-state chain
  proved ~50% timing-flaky per attempt (23-run evidence, findings §2c-2e).
  The path GRAPH is the honest authority (reach-gated edges, 3 unit
  tests); the assist only executes what the graph granted. Visual polish
  (real climb animations for assisted moves) = future work.
- **Gate drift (architect-sanctioned descope):** the multi-colonist
  climb-execution COMPOSITE outcomes in `--b58-scenario` are KNOWN-OPEN
  informational, not gating; Ben's LADDER COLLISION WAIVER shipped as the
  narrow v1 and full determinism is owned by SOFT-COLLISION (COMMITTED at
  B6, design doc in readme/). The B4 buried-job invariant now rides the
  exposure gate's proactive unreachable-flagging (same assert, new
  mechanism).
- **Auto-access materials:** auto-built access (rescue rungs/steps) is
  MATERIAL-FREE ("infrastructure from spoil"); player-placed ladders cost
  `BUILD_MATERIAL_ITEM` like Build. Deliberate asymmetry — note for B6's
  real recipe system.

## B-ASSET1 (2026-07-09)

- **Spec vs reality — asset-lab layout:** the block prompt said
  `asset-lab/vox/real/`; at block start REAL candidates lived flat in
  `vox/` (no real/). Mid-block the pilot converged on the prompt's layout
  + added `catalog.json` + per-asset `.ron` sidecars (contract v2, driven
  by this block's §9 findings). The loader supports both (catalog-first,
  legacy fallback). No doc change needed — reality converged to spec.
- **Marker authority split:** `readme/ASSET_MARKER_REGISTRY.md` is the
  single custom-band (200–255) authority; the engine mirror lives in
  `server/src/bastion_assets.rs::marker_registry` as parse-checked RON
  strings. Drift between them = fidelity findings at load (by design).
  The registry doc's byte-200 row writes the shorthand `DoorBars(())` —
  the valid StructureBlock form is `Sprite(DoorBars())`; flagged to the
  pilot (3 gate sidecars carry the same shorthand).
- **ASSET_INTEGRATION_LOG.md left UNTRACKED deliberately:** it is a
  living cross-agent coordination file (pilot appends reads, tester
  appends results) in the primary tree, like `asset-lab/` itself —
  committing it from a worktree would fork it from the live copy.
  Flagged for the architect: decide whether Ben's VC sweep adopts it.
- **ACCRETE RULE postdates this block:** FLEET_STATUS.md now requires
  each systems block to extend a B-TESTBED scenario; B-ASSET1's
  `--asset-test` IS the asset-tier standing suite (spec'd before the
  rule). B-TESTBED should absorb `--asset-test all` as a lane when it
  lands.

## B5.6b-2.1 drift notes (2026-07-10, overnight)

- **Egress reset semantics refined vs the FLEET_STATUS spec:** the spec
  (and the first implementation) said "employed colonists reset the
  trapped-detector watch". B5.8-E2 narrows that to **ARRIVED-only** (the
  colonist is actually working): a claim that bounces unreachable every
  ~1.2s kept its colonist nominally employed forever, so the employed-
  reset starved the egress net — the exact NO-INFINITE-LOOPS violation
  the net exists to prevent. Traveling colonists neither reset nor
  accrue; real movement resets via the existing position test. Paired
  with the `Job.last_bounce` claim bar (same colonist + same feet-block
  may not re-claim a job it just bounced; anyone else may).
- **Descent-gate depth threshold vs novice reach:** the gate holds Mine
  claims at depth > 2 — but a full-depth DEFAULT dig (down=2, a 3-block
  rim rise) already exceeds a climbing-0 colonist's scramble reach (2).
  Deliberately NOT widened to depth ≥ 2 (that would demand access plans
  for every default surface paint); the egress net is the designed catch
  for the climbing-0 self-trap case, now that E2 lets it fire.
  Climbing-1 colonists (reach 3) scramble out of default pits fine.
- **`WORK_DURATION_BASE` 6.0 is a stopgap:** Ben's "too fast" verdict,
  answered with a flat base bump. TOOL-0 (queued) replaces the flat bump
  with `tool_factor` — when it lands, the base constant's meaning shifts
  from "the rate" to "the no-tool floor"; re-tune then, don't stack.

## B6 SOFT-0 + live-fix batch drift notes (2026-07-10)

- **Entombment is now impossible BY CONSTRUCTION, not by verdict.** The
  final tier is a VERDICT-INDEPENDENT teleport: a colonist that isn't
  working and completes no job while not moving 6 blocks for 60s is
  teleported to the nearest real surface. This deliberately does NOT
  consult `egress_scan` — the earlier verdict-gated tiers left a
  `has_egress` FALSE-POSITIVE hole (a shaft-mouth hover reads escapable
  but isn't). The organic tiers (Waiting → climb-free → egress plan)
  are the PREFERRED path and handle the vast majority; teleport is the
  loud floor beneath them (every fire is `warn!`-logged — a fire means
  the organic path failed and wants investigation). SOFT-COLLISION-
  design §0 didn't spec a teleport tier; Ben directed it as the
  ultimate backstop and it subsumes the design's "guaranteed egress."
- **`ActiveJobState::Waiting` is new** — a third state the design didn't
  name. Queue-waiting at a single-file link is now a first-class state
  the watchdog skips entirely (queue time is not stuckness). Validated
  against DF 53.15 (DF-CHOKEPOINT-BEHAVIOR-REF): DF has no explicit
  Waiting (emergent repath) but our explicit density-promoted state is
  MORE deterministic — the reference confirms no forward path
  reservation is needed (occupancy + re-anchor suffices).
- **The reset-prone-accumulator is a recurring smell (3rd instance).**
  `stuck_time` (fixed by `reset_dist` hysteresis), `churn.1` (its
  threshold raced its own reset → the F5 dead-code teleport), and the
  original E2 employed-reset all share the shape: an accumulator whose
  reset condition can fire before its threshold, silently disabling the
  net it feeds. Flagged to the reviewer for BASTION_COMMON_ISSUES.
- **`day_length`=10 min is overseer-scoped, not global.** The TimeScale
  day mechanism was already correct (the imperceptibility was the 30-min
  base day); the overseer flag shortens the day so 4× reads as 4×.
  Vanilla sessions + the per-world meta stay untouched.
  `WORK_DURATION_BASE` is still REAL seconds (flagged in-code) — the
  per-game-time migration (TIMESCALE-DESIGN) is deferred; don't rekey
  it to TimeOfDay-delta (desyncs under MAX_DELTA_TIME per FR6).
- **The climb assist can lift WITHOUT a job (`climb_free`).** A
  dispersing or trapped-idle colonist has no ActiveJob; the assist's
  join is now `(&active_jobs).maybe()` so the fail-safe covers them.
  Job-driven climbs still require the target above; climb_free lifts
  unconditionally upward while its window lasts.

## TOOL0 + B5.8-E3 drift notes (2026-07-10, overnight)

- **The climb assist has a documented SLACK of reach+2 in enclosed
  shafts** (chimney: the reach cap measures ground below CURRENT feet,
  so repeated grabs gain reach+1, and ledge-snap adds one) — a colonist
  can EXECUTE an ascent the path GRAPH would refuse. Kept deliberately:
  it self-rescues real entrapments and only fires against walls with
  grab contact. The graph stays the PLANNING authority; the b1 scenario
  gate acknowledges the race (`ladder-chain OR exited`). If a future
  block needs execution to exactly match the graph, this is the spot.
- **Assist XP-on-use re-levels mid-scenario:** any skill pin
  (`bastion_set_colonist_climbing`) decays as the assist grants XP —
  scenario design must not assume a pinned level persists through
  climbing activity.
- **Egress semantics tightened (the annulus rise fix):** "standable"
  now means rise ≤ reach exactly. The trapped-detector consequently
  fires for default-depth (down=2) digs left by climbing-0 colonists —
  MORE egress bubbles in live play than before, by design (they were
  entombed before; Ben's "nobody entombed" invariant now actually
  holds for the novice case). The descent gate's >2 threshold stays
  (its consistency note from b-2.1 still applies).
- **E2's `Job.last_bounce` bar: added and REMOVED same-night** — it
  leaked on physics wobble and starved the strike-growth convergence.
  The churn detector (count consecutive unreachable releases in place)
  replaced it; if archaeology finds the bar referenced anywhere, it
  never reached main.
- **Harness measurement semantics:** `bastion_jobs_in_region` and
  `bastion_claimed_job_positions` EXCLUDE `is_access` jobs — designation
  invariants measure designation work; access scaffolding lives inside
  dig volumes by design. Any future caller wanting scaffolding included
  needs a new hook.

## TIMECTL drift notes (2026-07-10, overnight)

- **Spec deltas (small, deliberate):** UI-3 §3 asks for a "dimmed world /
  PAUSED tag" — shipped the tag + amber lit-button (no world dimming; a
  post-process hook is disproportionate for v1). "Faster (3–4×)" shipped
  as 4× exactly (`MAX_DELTA_TIME` clamps nothing until ~30×; physics
  fidelity, not the clamp, is the real ceiling — top speed stays a
  tuning call). Buttons render as text `II/1×/2×/4×` (glyph coverage in
  the game font beats ⏸/⏩ emoji).
- **ESC-menu unpause interplay (vanilla behavior, kept):** the vanilla
  ESC/settings menu auto-pauses on open and auto-UNPAUSES on close — a
  cluster-pause therefore doesn't survive an ESC round-trip. The HUD
  mirrors the truth each frame so the buttons never lie; unifying the
  two pause sources = backlog if Ben trips on it.
- **Multiplayer scope:** pause is singleplayer-only (vanilla mechanism);
  `/time_scale` needs Admin. In any future multiplayer overseer session
  the pause button no-ops and speed needs the admin role — fine for the
  colony-sim shape (solo god), noted for B10+.
