# BASTION_ARCHITECTURE — how the built systems actually work

The living "how does this all work" map for Project Bastion (Veloren fork →
autonomous god-game colony sim). Written for a fresh session with no memory:
read this + the design doc (`readme/veloren-colony-rts-build-report.md`) + the
run log (`docs/BASTION_RUN_LOG.md`) and you can continue building correctly.
Every block updates this file as part of its work (append/edit the relevant
system section; keep it truthful over tidy).

Created 2026-07-09 as a retroactive catch-up covering B0–B5 (all merged).
Sources: the findings docs (`docs/BASTION_B*_FINDINGS.md`), the code itself,
and the git history — locations below are verified against the repo, not
guessed. Line numbers drift; symbols are the anchor.

---

## 1. Pillars & invariants (in force, enforced by tests or convention)

- **Influence, not command** (design doc §1a): the player designates, sets
  policy, and spends god-power; colonists decide and act autonomously. Any
  "tell unit X to do Y now" feature must become designation/policy/god-power.
- **Overseer = invisible player entity, NEVER spectator** (design doc §4
  standing directive): the god camera anchors to a real player entity with
  correct chunk streaming. Two behavioral rules: (1) the world ignores the
  anchor (no aggro/greet/collide — `BastionGodAnchor` marker + behavior-tree
  invulnerable-target drops); (2) the anchor cannot die (permanent
  `BuffKind::Invulnerability` while god mode is on). Mortality is B12's
  (possession) concern only. Past sessions repeatedly reached for spectator
  mode — do not; it is the source of the streaming/sync problems.
- **The loaded↔simulated boundary**: colonists are rtsim NPCs first; the
  loaded ECS entity is a *projection* that exists only while their chunk is
  loaded. All colonist identity/skills persist rtsim-side and survive
  promote/demote round-trips and full restarts.
- **Invariant-first testing** (design doc §7): bit-exact determinism is NOT
  the gate (rtsim rules seed RNG from OS entropy — see WI-DET). The gates are
  invariants: no item dupe/loss, no double-claims, entity counts return to
  baseline, bounded tick time, no panics. Aggregate determinism (counts +
  clocks at fixed seed) is measured-OK at B0 and re-checked when sim systems
  change.
- **Additive & gated, never break vanilla**: everything Bastion adds is
  namespaced (`bastion`-prefixed crates/modules/comps/flags) and vanilla
  voxygen/server-cli must keep building and booting unchanged (checked at
  every block gate).
- **serde-ready by construction**: every new bastion type derives
  Serialize/Deserialize the day it is born (B10 persistence ground rule); new
  rtsim fields are `#[serde(default)]` so old `data.dat` saves keep loading.
- **`bastion/main` only advances by fully-tested, tagged blocks** — tags
  `bastion-block-<N>` are the rollback map (see
  `readme/BASTION_RESTORE_LEDGER.md`).

## 2. The systems, in dependency order

### 2.1 B0 — Baseline + headless harness (`bastion-harness/`)

**What:** a custom test-driver crate that boots the real `veloren-server`
in-process (no network, no GPU, no client) and drives it tick-by-tick, faster
than real time (~8.7×), asserting invariants. It is how ALL simulation logic
is gated; voxygen is only for visual/UX verification.

**How it works:** mirrors the singleplayer recipe — build `server::Settings`
in code (`gameserver_protocols: vec![]`, `auth_server_address: None`,
`calendar_mode: None`, seeded world), `Server::new(...)` with a **fresh temp
data_dir per run** (rtsim loads `<data_dir>/rtsim/data.dat` if present —
stale dirs poison reproducibility), then loop
`server.tick(Input::default(), fixed_dt)` + `server.cleanup()`. dt is fixed
1/30 s and the loop never sleeps; N server ticks = N rtsim ticks.

**Where:** `bastion-harness/src/main.rs`. Modes: `--ticks N` (baseline
aggregate dump), `--verify` (two isolated child processes, diff aggregates →
DETERMINISM OK/DIVERGED), `--colony N` (B3 roster), `--b4-scenario`,
`--b5-scenario` (block gates, JSON result line + PASS/FAIL exit code).
Deep detail: `docs/BASTION_HARNESS.md`, `docs/BASTION_B0_FINDINGS.md`.

**Baseline facts that still matter:** upstream `master` @ `bfef92fc…`
(content-verified; local history shares no ancestry with upstream — see
`BASELINE.md` graft note). `common/build.rs` requires `git log` to succeed.
Toolchain: pinned nightly, `x86_64-pc-windows-gnu`; the built exes need the
mingw runtime DLLs next to them or on PATH (`BASTION_B1_FINDINGS.md` §5b).

### 2.2 B1 / B1.5 / B1.6 / B1.7 — the overseer camera stack (voxygen)

**What:** a Black&White2-style top-down orthographic god camera with
Z-slicing and a 4-mode occlusion framework, entered via
`--bastion-overseer` (`voxygen/src/cli.rs`) and toggled with F9.

**Key pieces & where:**
- `CameraMode::Overseer` — `voxygen/src/scene/camera.rs`. Swaps only the
  projection for an orthographic pair (half-height `v = dist·tan(fov/2)` so
  `dist` still means zoom); bypasses the terrain-collision ray (a god camera
  slicing underground must not collide). Reversed-depth conventions per
  vanilla.
- **Input contexts** — `voxygen/src/bastion/input.rs`:
  `InputContext { Menu, Overseer, Avatar }` with static
  `ContextScheme { owned, suppressed }` tables, applied at the single
  physical-key→`GameInput` fan-out point (`Window::map_input`,
  `voxygen/src/window.rs`). Menu = strict vanilla passthrough. This is why
  overseer keys can share physical keys with vanilla (Q/E = Slot10/Interact)
  without conflicts. B9 adds per-context rebind overrides here.
- **Camera feel** — grab-drag pan (plane-anchored, 1:1, inertia on release),
  right-drag orbit (pitch clamped 20°–89°), zoom-to-cursor — all through
  `Camera::dependents()` unprojection; see `docs/BASTION_CAMERA.md` and
  `BASTION_B1_5_FINDINGS.md` §3 for the math. Raw mouse arms live in
  `voxygen/src/session/mod.rs` (~line 2096, `bastion_overseer_active` gated,
  click-vs-drag disambiguated by a 6px slop).
- **Occlusion / view modes** — `Globals` uniform tail
  (`voxygen/src/render/pipelines/mod.rs` + `assets/voxygen/shaders/include/
  globals.glsl`) carries mode bits + params; a shared
  `bastion_occlusion.glsl` include computes an alpha composed from soft-slice
  / proximity / cutaway / roof-reveal, applied as **ordered Bayer dithered
  discard** (the opaque passes can't alpha-blend). Slice keys: PageUp/Down;
  top-down snap: End; view-mode cycle: V. Deep detail:
  `docs/BASTION_VIEWMODES.md`, `BASTION_B1_6_FINDINGS.md`.
- **B1.7** (inside B1.6's commits): ortho near-plane extension
  (`OVERSEER_BEHIND` = 768 blocks behind the camera plane — see Gotchas) and
  terrain-anchor hysteresis for LoD stability.
- **Chunk streaming**: the overseer anchor entity's `Pos` follows the camera
  focus, so chunks load under what you look at (the invisible-player-anchor
  design; presence carries `bastion_terrain_anchor`).

**Tested by:** in-game QA rounds (documented in the findings/test docs);
the harness does not exercise voxygen.

### 2.3 B2a — Overseer interaction surface

**What:** the tool palette, right-click radial menu, selection, and the
designation paint→server→echo→overlay loop. This is the channel every later
player-facing verb rides.

**Where:**
- Shared types: `common/src/bastion.rs` — `Region` (inclusive AABB,
  `normalized()`, volume cap `MAX_DESIGNATION_VOLUME`), `DesignationKind
  { Mine, Chop, Build, Stockpile }`, `ContextVerb` (incl. `FoundColony`),
  `InfluenceKind`, `ContextTarget`.
- Client→server messages (`ClientGeneral::…` in `common/net`):
  `BastionPlaceDesignation`, `BastionCancelDesignation` (B4),
  `BastionApplyInfluence`, `BastionContextAction`, `BastionSpawnColony` (B3),
  `BastionCameraAnchor` (B1.6). Server validation lives in
  `server/src/sys/msg/in_game.rs`; board mutations are deferred to after the
  parallel message loop (the job board can't be touched inside the par join).
- Echo path: `ServerGeneral::BastionDesignation { region, kind }` → client
  stores in `bastion_designations` (`client/src/lib.rs`) → session syncs to
  `scene.debug` line-rect overlays (`voxygen/src/scene/debug.rs`).
- Tools state: `voxygen/src/bastion/tools.rs` — `ToolMode { Pan, Inspect,
  Designate(kind) }`, `GodMode { God, Free }`, `target_allowed()` stub (B2b's
  enforcement point). T cycles tools, G toggles god mode
  (`voxygen/src/game_input.rs` `Bastion*` variants; defaults in
  `voxygen/src/settings/control.rs`).
- Radial menu + palette: conrod HUD (`voxygen/src/hud/bastion.rs`,
  `hud/bastion_radial.rs`) — **not egui** (egui is hard-gated behind a debug
  toggle; gameplay UI can't live there).
- Selection: `comp::bastion::BastionSelected` marker
  (`common/src/comp/bastion.rs`); click/box-select via
  `bastion::cursor_ray`/`unproject_to_world_plane`
  (`voxygen/src/bastion/mod.rs`); feeds the B1.6 cutaway targets.

### 2.4 B3 — Colonists + the god anchor

**What:** the colonist entity model and the §4 anchor guarantees.

**How colonists work:** a colonist is an rtsim `Npc`
(`rtsim/src/data/npc.rs`) with `#[serde(default)] bastion_colonist:
Option<BastionColonist>`. `BastionColonist` (`common/src/bastion.rs`) holds
name/backstory/`ColonistSkills`/`WorkPriorities` — all persisted rtsim-side,
so the roster works headlessly and survives restarts. When a colonist's
chunk loads, vanilla promotion (`server/src/rtsim/tick.rs`, promote at the
`SimulationMode::Loaded` flip) creates the ECS entity; a decoration pass in
the same tick system inserts the synced `comp::Colonist` mirror
(`common/src/comp/bastion.rs`, registered in
`common/net/src/synced_components.rs`) and overrides `Stats::name`. Demote
(chunk unload) deletes the ECS entity; the rtsim record persists.
`PlayerColony`, `Needs`, `Mood` comps exist but are server-side only until
B7/B-AG4.

**God anchor:** `comp::bastion::BastionGodAnchor` marker + permanent
`BuffKind::Invulnerability` applied in the `BastionCameraAnchor` handler on
god-mode entry, removed on exit. The vanilla behavior tree already drops
invulnerable targets, giving no-aggro nearly for free.

**Spawning:** `Server::bastion_spawn_colony(pos, n)` (pub, harness-callable)
+ in-game via radial `FoundColony` → `ClientGeneral::BastionSpawnColony`.

### 2.5 B4 — Designation → job board → arbitration → pathing

**What:** the heart of Slice B. Painted designations become block-level jobs;
idle colonists claim them (priority → distance, atomically within a pass) and
walk there autonomously.

**Where:** `server/src/bastion_jobs.rs` (all of it), job types in
`common/src/bastion.rs` (`Job`, `JobId`, `WorkType`, `JobAudit`), per-entity
claim state in `common/src/comp/bastion.rs` (`ActiveJob`,
`ActiveJobState { Traveling, Arrived }`).

**The loop, per server tick** (`bastion_jobs::Sys`, Phase::Create):
1. **Travel/work upkeep** (every tick, `LendJoin` over colonists+ActiveJob):
   Traveling → assert `NpcActivity::Goto(target, TRAVEL_SPEED)` into
   `Agent::rtsim_controller.activity` (executed by the vanilla agent behavior
   tree with real traversal); arrival = **3D** distance <
   `ARRIVE_DIST`(2.5) from `job.pos + (0.5,0.5,1.0)`; a progress-based
   watchdog (best-distance must improve ≥ `STUCK_EPSILON` within
   `STUCK_TIMEOUT`=10 s) releases the claim and marks the job `unreachable`.
   Arrived → B5's work tick (below).
2. **Claim sweep** (staggered, tick % 15 == 3): release claims whose
   claimant entity vanished (demote/despawn) — work never leaks.
3. **Unreachable retry** (every 60 ticks): clear all `unreachable` flags —
   B5 makes terrain change as a consequence of completion, so unreachability
   is not permanent (e.g. an enclosed dig cell opens up when neighbors clear).
4. **Arbitration** (every `ARBITRATION_INTERVAL`=15 ticks): per idle
   colonist, best = highest `WorkPriorities` priority (0 = never) → nearest;
   Build jobs require the colonist to be carrying `BUILD_MATERIAL_ITEM`;
   `skill_floor` enforced; claims marked on the board during selection so two
   colonists can't pick the same job in one pass.

**The rtsim clobber gate:** `server/src/rtsim/tick.rs` (~line 737) skips
copying `npc.controller.activity` into the agent while the entity has an
`ActiveJob` — otherwise the rtsim brain would overwrite the job travel
intent every tick.

**Cancel:** `JobBoard::cancel_region` removes jobs + returns released
claimant uids; colonists re-idle within one upkeep tick (their job id no
longer resolves).

**Headless colony testing:** `Server::bastion_force_load_area(center,
radius)` synchronously generates + inserts chunks and pins them in
`BastionForceLoaded` (the unload sweep skips pinned chunks) so colonists
promote without a client. NOTE: `Read<T: Default>` does NOT auto-register
resources through the dispatcher — `JobBoard`/`BastionForceLoaded` are
inserted explicitly in `Server::new`.

### 2.6 B5 — Work execution (dig/chop/build, drops, XP)

**What:** Arrived colonists actually do the work: progress accumulates at
`work_rate(skill)` ((1 + 0.2·level)/3 s), and completion applies the terrain
edit, emits the item drop, and grants XP.

**How completion works** (in `bastion_jobs.rs`'s Arrived arm), in order:
1. **Re-validation**: the placement predicate is re-checked against live
   `TerrainGrid` (Mine: still filled; Chop: still Wood; Build: still empty).
   A job whose block changed under it is *moot*: removed with no loot/no XP
   (checked before Build consumes material). This plus placement-time
   dedupe (one job per block position, ever — `place_designation` skips
   occupied positions) kills the repaint/overlap item-dupe class.
2. **Same-tick deferral**: `BlockChange::can_set_block` — if another system
   already edited this block this tick, retry next tick (never let system
   run order decide the final block state).
3. **Build material gate**: consume one `BUILD_MATERIAL_ITEM` from the
   colonist's own `Inventory` (`slots_with_id` + `remove`); if missing,
   stall: reset progress, set `needs_materials`, release. Arbitration also
   pre-gates (only material-carriers may claim Build jobs) and sweeps the
   `needs_materials` flag for unclaimed Build jobs each cycle.
4. **Terrain edit**: `BlockChange::set` (`common/state/src/state.rs`) — the
   same authoritative path vanilla mining uses; applied by
   `apply_terrain_changes_internal` at tick end. NEVER raw chonk writes.
5. **Item drop**: emit `CreateItemDropEvent` (`MINE_DROP_ITEM` stones /
   `CHOP_DROP_ITEM` logs; consts in `common/src/bastion.rs`) — handled by
   `Server::create_item_drop` (`server/src/state_ext.rs`: merge-with-nearby
   first, else spawn a `PickupItem` entity with a 300 s `DeleteAfter`).
6. **XP**: `ColonistSkills::grant_xp(work, 8.0)`; flat 20 XP/level
   (`SkillLevel::add_xp`), feeding back into `work_rate`.

**The colonist loot gate:** vanilla Humanoid NPCs opportunistically target
and pick up nearby item drops (`is_valid_target` in
`server/agent/src/action_nodes.rs`). That silently consumed every B5 drop
the instant it spawned. Gated off for entities with `comp::Colonist` (new
`ReadData::colonists` field in `server/agent/src/data.rs`). B6 decides
whether hauling reuses this path selectively or does deliberate pickup.

**Harness gate:** `--b5-scenario` — quarry-pit mine (27 jobs, with a carved
exit ramp — see Gotchas), single-block chop, two-phase Build
(with-material completes / material-consumed stalls + `needs_materials`),
drop counts conservation-checked, XP asserted, zero-input soak. Debug/state
hooks for all of this live as `Server::bastion_*` methods in
`server/src/lib.rs` (~lines 860–1010).

### 2.7 B5.5 — Zone deletion + item-drop pile aggregation (patch block)

**What:** the two gaps the first live demo exposed — no way to delete a
painted zone, and mined areas carpeting into one item entity per block.

**Zone deletion:** the server side already existed
(`JobBoard::cancel_region` + `ClientGeneral::BastionCancelDesignation`,
B4). B5.5 adds the UI + overlay: `ToolMode::Erase`
(`voxygen/src/bastion/tools.rs`, T-cycle; same drag as designate, red
preview, sends the cancel message) and `RadialAction::DeleteZone`
(`voxygen/src/hud/bastion.rs`; client-resolved — one cancel per painted
rect containing the clicked block; never crosses the wire as a verb). The
cancel handler echoes `ServerGeneral::BastionDesignationRemoved{region}`;
the client (`client/src/lib.rs`) subtracts it from stored rects via exact
3D AABB subtraction (`Region::subtract` in `common/src/bastion.rs`, ≤6
pieces, unit-tested for volume conservation) and bumps
`bastion_designations_rev`; voxygen rebuilds ALL overlay shapes on rev
change (incremental index sync can't express removal/splits).

**Pile aggregation (merge-never-delete):** the pebble-carpet root cause
was `PickupItem::should_merge == false` on B5 drops — Veloren's
conservation-exact pile machinery (`PickupItem` = `Vec<Item>` with
`amount()`, `try_merge`, spawn-time + periodic merging in
`server/src/sys/item.rs`) existed all along and never fired. Colonist
drops now emit `persistent: true` (new `CreateItemDropEvent` field; all
vanilla emitters pass `false`): **no `DeleteAfter` despawn timer**
(colonist output is a player resource — the old 300 s timer was a latent
loss bug), a `comp::bastion::BastionPile` marker (server-side), 
`should_merge: true`, and a gentle toss so spawn merging lands.
**Merge-class separation**: `get_nearby_mergeable_items` only pairs
entities whose `BastionPile` presence matches — a persistent pile must
never merge into a timed vanilla drop (it would inherit the despawn =
silent loss) nor grant vanilla loot immortality. `bastion_piles::Sys`
(`server/src/bastion_piles.rs`) tier-scales piles by amount via the
synced `Scale` comp (1.0/1.35/1.7) so heaps read bigger as they grow.

**B6 interface (deliberate):** piles are ordinary `PickupItem` entities +
marker — enumerable/claimable exactly like drops; hauling one pile of 47
is one trip. Partial-pickup/split APIs don't exist; B6 decides.

**Harness gate:** `--b55-scenario` — partial erase (exactly the erased
half's jobs removed, zero orphaned claims via the new
`bastion_orphaned_claims` hook, remainder keeps working), whole-zone
delete (board empty, all idle), then a 200-block slab mined with **exact**
conservation (`bastion_sum_items_near` == 200) through merges AND through
the soak, with the entity count bounded (25 piles observed). The B5
scenario's drop assertions switched to amount sums.

### 2.8 B5.6a — Zone visuals: terrain draping + toggle + pile tiers (client-side patch)

**What:** fixes the photographed floating-overlay bug and adds a
designation-visuals toggle + richer pile visuals. (Split from B5.6; the
fills/volumes/volume-selection remainder is B5.6b — see the design doc and
`BASTION_CONSISTENCY.md`.)

- **Overlay draping** (`voxygen/src/bastion/mod.rs`): `overlay_surface_z`
  (the single overlay-height authority — `ground_z`, clamped to the active
  Z-slice) + `draped_rect_outline` (samples the surface along a rect's
  perimeter, returns conformed line segments). `bastion_region_outline`
  (`session/mod.rs`) emits these instead of 4 flat lines at the pick-plane z
  (which floated on slopes). All overlay callers drape: paint + box-select
  previews (coarse `step 2.0` during drag) and the committed designation
  overlay (`step 1.0`, rebuilt on rev **and Z-slice** change). **This is the
  reusable overlay-renderer seam** B5.6b (fills/volumes) and §3w (colony
  boundary) reuse — keep `overlay_surface_z` the one height authority.
- **Visuals toggle** (`voxygen/src/bastion/tools.rs` `VisualsMode`, key
  **H**): On / Subtle (dimmed outlines) / Off (hidden). Purely visual —
  designations stay active; a paint/erase tool auto-reveals while active.
- **Pile tiers** (`server/src/bastion_piles.rs`): 5-step growth curve with a
  great-mound plateau cap; count never touched (conservation exact). Note:
  uses the synced `comp::Scale`, which also scales the physics collider — a
  real heap mesh (asset pipeline) would decouple visual size from collider.
- **Not done** (judgment call, logged): erase-by-type filter — not cheap on
  existing seams (needs a protocol change). Area-erase already exists (B5.5
  erase-drag).

**Gate:** visual-correctness is screenshot-verified in-game (draping on a
hill + a pit across Z-slice modes); headless B4/B5/B5.5 confirm zero sim
impact (client-only + a pile-scale tweak). **Standing note:** `--b5-scenario`
is timing-flaky under machine load — run gate scenarios on a quiet machine
(it was 6/6 at both the B5.5 tag and this branch when quiet).

<<<<<<< HEAD
### 2.9 B5.6b — Zone-management UI (SPLIT into sub-blocks b-1..b-4)

B5.6 was too large for one block; a builder scope-flag split it (architect-
blessed): **b-1** fills+colors+blend+labels+SUBTLE (built); **b-2** z_extent
model + volumetric + volume-selection UX (also closes B5.MINE-COVERAGE);
**b-3** zone click-select + radial (Delete/Modify-depth/Edit-mode drag
handles); **b-4** erase-by-type wire filter. Plan:
`docs/BASTION_B5.6b_FINDINGS.md`.

**B5.6b-1 (built):** terrain-conformed translucent zone **fills** in the
kind-color legend, overlap-blended, with centroid **labels**; SUBTLE=border
only. Client-only.
- `DebugShape::ConformedTris(Vec<[Vec3;3]>)` (`scene/debug.rs`) — pre-
  conformed geometry rendered with the per-shape context color; the debug
  pass already alpha-blends (`render/pipelines/debug.rs`
  `BlendState::ALPHA_BLENDING`), so no new pipeline. NOTE: the debug frag
  *lights* it — fills are a lit tint, not a flat UI fill (backlog).
- `bastion::draped_fill_tris` (`voxygen/src/bastion/mod.rs`) — samples the
  visible surface (`overlay_surface_z`, slice-aware) at each footprint corner
  once, emits 2 draped tris/cell. The **reusable conformed-fill utility**
  (b-2 volumes + §3w boundary reuse it; `overlay_surface_z` is the one height
  authority).
- Colors: `bastion::tools::{zone_rgb, zone_border_color, zone_fill_color}`.
  Fills low-alpha → overlaps composite to a blended color.
- Labels: `Hud::bastion_set_zone_labels` → conrod `Text` with
  `.position_ingame(centroid)` (world-anchored, like overhead nametags),
  fed by `session::bastion_sync_designations` (ON mode only; empty in
  SUBTLE/OFF).
- **Gate:** in-game — fills drape + colored + overlaps blend + labels +
  SUBTLE=border-only; headless B4/B5/B5.5 unaffected (voxygen-only diff).

### 2.10 B-MAP1 — Overseer minimap (founds the map/overlay layer)

**What:** the god's minimap — rendered top-down terrain tiles (WoW-addon
technique), a zoomable pyramid blending into the worldgen map, overlay
pin/layers, and click/drag/scroll navigation. Replaces the vanilla minimap
ONLY while the overseer HUD is active; flagless boots keep vanilla
bit-identical.

**Where:** `voxygen/src/hud/bastion_minimap.rs` (all of it), drawn from
`hud/mod.rs` instead of vanilla `MiniMap` when `bastion.active`. Session
handles `BastionMinimapJump/Pan` (→ `camera.set_focus_pos`; XY only — the
overseer focus glide re-rides `ground_z`). Zone colors come from
`bastion::tools::zone_rgb` — b-1's one color legend; the map applies its own
alpha (map footprints and in-world fills agree by construction).

- **Tile engine** (`BastionMinimapTiles`): per-chunk 32×32 tiles (1
  texel/block) scanned straight down from sky or the active Z-slice out of
  the real loaded voxels (buildings/trees/digs are terrain voxels, so they
  appear as themselves); per-texel heights drive an NW-light hillshade.
  Tiles build off-thread on the `IMAGE_PROCESSING` SlowJobPool via
  `KeyedJobs` keyed `(chunk, slice_rev)` — renders trickle, stale tiles keep
  showing until replaced, the frame never blocks. **Invalidation:**
  `TerrainChanges::modified_blocks` (any modified block re-renders that
  chunk's tile — dig a pit, the map updates in ~a second) and slice changes
  (slice below ground = the map shows that level; the mining framework's
  slice-aware map comes free). Composite: a chunk-grid-anchored 512²
  (16-chunk) window texture; panning moves only the widget's source
  rectangle, re-anchoring recomposites from cache.
- **Two-tier pyramid:** worldgen map (1 texel/chunk) always drawn beneath;
  the tile layer alpha-fades out as the view widens past the 512-block
  window (colony/district = tiles, region/world = worldgen). Zoom is
  px-per-block, scroll-stepped, world-fit … 4 px/block; level label shows
  Colony/District/Region/World.
- **The §3s pin/layer API** (the part future blocks reuse): `MinimapLayer
  { Colonists, Zones, Piles, Frustum, Alerts }` with per-layer toggle chips;
  built-in providers query client state directly (zones from
  `client.bastion_designations()` tinted via `tools::zone_rgb`; colonists
  from synced `comp::Colonist`+`Pos` with `comp::BastionSelected` highlight;
  piles from synced `comp::PickupItem`+`Scale`; frustum from
  `unproject_to_world_plane` of the 4 screen corners). **External systems
  (territory §3w, trade routes, dominion, threat alerts) add pins without
  touching the widget:** push `MinimapPin { wpos, color, size, halo }` into
  `hud.bastion_minimap.extra_pins` each frame (drawn on the Alerts layer),
  or add a `MinimapLayer` variant + provider for a first-class toggleable
  layer. Icon art for chips/pins is a wishlist item for the asset lab.
- **Navigation:** click = jump the god camera there; drag = continuous pan
  (world moves with the cursor); scroll = zoom. North-up always.

**Not done (recorded):** GPU ortho-RTT tile source (the prompt's literal
technique — the conrod UI only consumes CPU images, so RTT needs a bespoke
offscreen pass + readback; the tile cache/API is source-agnostic and CPU
voxel-scan tiles meet the gate — see `docs/BASTION_BMAP1_FINDINGS.md` §2 and
the consistency log), fullscreen map view, alert providers (hook only),
minimap-window dragging in overseer mode.

**Tested by:** compile + vanilla-regression gates headlessly; visual gate
(tiles recognizable, dig-updates, overlay accuracy, click-jump) in-game —
`docs/BASTION_BMAP1_TEST.md`.

## 3. Build methodology (how blocks land)

Per-block cycle: **checkpoint** (clean tree, branch `bastion/block-<N>`, log
start SHA) → **explore** (verify real symbols; write
`docs/BASTION_<N>_FINDINGS.md`) → **build** (small labeled commits, additive
+ gated) → **self-test** (cargo green; harness scenario asserting the
block's Done-when + standing invariants; zero-input soak from B4 on; vanilla
boots) → **merge no-ff + tag** `bastion-block-<N>` or **rollback** (main
untouched, branch kept for diagnosis) → **bookkeeping** (append to
`readme/BASTION_BACKLOG.md`, `readme/BASTION_RESTORE_LEDGER.md`,
`readme/BASTION_CONSISTENCY.md`; update this file; run-log entry).
Full protocol: `readme/MEGA-PROMPT-autonomous-batch-builder.md`.

## 4. Key reused Veloren machinery (wrap, don't reinvent)

| Machinery | Where | Bastion use |
|---|---|---|
| rtsim NPCs + promote/demote | `rtsim/`, `server/src/rtsim/tick.rs` | colonists ARE rtsim NPCs; the loaded entity is a projection |
| `NpcActivity::Goto` → agent behavior tree | `common/src/rtsim.rs`, `server/agent/` | job travel with real traversal, no new pathing code |
| `BlockChange` | `common/state/src/state.rs` | all terrain edits (authoritative, mesh/rtsim hooks fire) |
| `CreateItemDropEvent` → `create_item_drop` | `server/src/state_ext.rs` | all item drops (merge + despawn semantics for free) |
| `BuffKind::Invulnerability` | `common/src/comp/buff.rs` | god-anchor immortality + no-aggro (tree drops invuln targets) |
| `Inventory` | `common/src/comp/inventory/` | Build material stand-in; B6 hauling |
| synced-components x-macro | `common/net/src/synced_components.rs` | `Colonist` comp sync (one list entry propagates everywhere) |
| scene debug shapes | `voxygen/src/scene/debug.rs` | designation overlays, colonist markers, select boxes |
| conrod HUD events | `voxygen/src/hud/` | radial menu, palette, info lines (egui is debug-gated — unusable) |

## 5. Gotchas & standing hazards (bite list — check before they bite again)

- **Vertical reachability is the recurring trap.** Arrival is 3D within 2.5
  of `block+(0,0,1)` and agents can't climb: (a) freestanding structures
  ≥2 tall have unreachable upper/lower blocks, (b) an enclosed dig pit
  traps its digger (B5's mine test carves an explicit exit ramp), (c) tall
  trees can't be per-voxel chopped (needs a base-interaction verb),
  (d) a single-level slab forced across sloped terrain buries/floats
  blocks (B5.5's Part 2 stalled at 8/200 until the terraform fully
  determined the geometry: under-fill + working level + headroom + ring).
  Backlog has the mechanism-level fix ideas; the mining framework
  (`BASTION-SYSTEM-FRAMEWORKS.md` §6) owns the real solution. Any new
  test geometry or designation feature must respect this.
- **`ground_z`-style scans must filter to real terrain kinds** (Rock/Earth/
  Grass/Sand/…): `is_filled()` counts tree Wood/Leaves, returning canopy
  height and placing things in mid-air (bit B5's chop/build tests).
- **Pick rays start ~768 blocks behind the camera** (`OVERSEER_BEHIND`,
  B1.7 ortho near extension): any ray-parameter range check must budget for
  it (silently broke B2a entity picking).
- **New shader `#include`s need TWO Rust registrations**
  (`renderer/shaders.rs` asset list + `pipeline_creation.rs` fetch match) or
  startup panics (bit B1.6).
- **Flagged/synced comp storages** (`Colonist`, etc.): multi-storage mutable
  iteration needs `LendJoin` (`.lend_join()` + `while let`), not `.join()`;
  single-entity `get_mut` is fine. Guard bindings need `mut`.
- **This project's rand needs `rand::RngExt`** for `random_range`/`random`
  (not the std `Rng` methods) — recurring compile stumble.
- **`Read<T: Default>` does not auto-register ECS resources** via the server
  dispatcher path — insert explicitly in `Server::new` (bit B4).
- **`bastion_force_load_area`'s `should_continue` closure means "cancel?"** —
  return `false` to proceed (passing `|| true` cancels every chunk).
- **docs/ vs readme/ split**: findings/test docs + run log live in `docs/`
  (older convention); bookkeeping + architect inputs live in `readme/`
  (append-only). Check both on startup.
- **Concurrent sessions share this working tree** (asset-lab session:
  `asset-lab/`, `readme/ASSET_*`, `readme/COMPONENT_*`, `MASTER-*`, etc.).
  NEVER `git add -A`; stage Bastion files explicitly by path.
- **The B4 harness scenario asserts *ever*-arrived/*ever*-unreachable**
  cumulatively across its window (B5 made `Arrived` transient); don't
  "simplify" it back to instantaneous sampling.
- **mingw runtime DLLs** must be exe-adjacent or on PATH for the gnu-
  toolchain exes (Entry-Point-Not-Found otherwise); ReShade on this machine
  eats the `Home` key (why top-down snap is `End`).
- **Retro-tag fuzziness**: B1.6/B1.7 share one tag commit; `bastion-block-B5`
  was moved once (documented in the restore ledger) — the ledger, not tag
  dates, is the rollback truth.

## 6. State & pointers (update every block)

**Done (merged + tagged):** B0, B1, B1.5, B1.6(+B1.7), B2a, B3, B4, B5,
B5.5, B5.6a.
**Done also:** B5.6a (tagged), B5.6b-1 (zone fills+colors+blend+labels+SUBTLE
— this block). **Next (per the updated queue):** B5.6b-2 (z_extent model +
volumetric + volume-selection UX; also closes B5.MINE-COVERAGE), b-3 (zone
interaction/edit-mode), b-4 (erase-by-type), plus B5.7/B5.8/B5.9,
B-UNDERGROUND, B-CAM-FOLLOW, then B6 (stockpiles/hauling). Independents
(B-MAP1, B-ASSET1, B-TESTBED) floatable.

| Need | Read |
|---|---|
| Block queue + protocol | `readme/MEGA-PROMPT-autonomous-batch-builder.md` |
| Block specs (Done-when) | `readme/veloren-colony-rts-build-report.md` |
| System frameworks (zones/mining/animation/testing) | `readme/BASTION-SYSTEM-FRAMEWORKS.md` |
| What happened when | `docs/BASTION_RUN_LOG.md` |
| Per-block real symbols | `docs/BASTION_B*_FINDINGS.md` |
| Rollback map | `readme/BASTION_RESTORE_LEDGER.md` |
| Known debts/ideas | `readme/BASTION_BACKLOG.md` |
| Doc/code drift | `readme/BASTION_CONSISTENCY.md` |
| Harness usage | `docs/BASTION_HARNESS.md` |
| Camera/view-mode internals | `docs/BASTION_CAMERA.md`, `docs/BASTION_VIEWMODES.md` |
| Agency / DF depth / divine politics | the bibles + gap ledger in `readme/` |
