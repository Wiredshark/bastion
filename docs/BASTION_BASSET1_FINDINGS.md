# BASTION B-ASSET1 FINDINGS — asset integration harness + render arena

Block spec: `readme/B-ASSET1-integration-render-arena-prompt.md`. Dynamic test contract:
`readme/ASSET_DYNAMIC_TEST_SPEC.md`. Asset-side ground truth: `readme/ASSET_GAMEPLAY_MARKERS.md`,
`readme/ASSET_SYSTEM_GUIDE.md`, `asset-lab/CATALOG.md`.
Explored 2026-07-09 (read-only recon while B5.6b-1 held the tree). All symbols verified in-repo.

## 1. The ingestion path (Part 1 seams)

### 1a. Structure loading — `common/src/terrain/structure.rs`
- `Structure` = `center: Vec3<i32>` + `Arc<BaseStructure<StructureBlock>>` + `custom_indices: [Option<StructureBlock>; 256]`.
- `BaseStructure` = `Dyna<Option<NonZeroU8>>` volume + `palette: [B; 256]`.
- `load_base_structure(dot_vox_data, to_block)` (line ~170, **pub(crate)**): stores each voxel's
  palette index as `voxel.i + 1` (`NonZeroU8`), palette color at `(i + 1)`. Default block for
  unmapped bytes: `StructureBlock::Filled(BlockKind::Misc, palette_color)`.
- `StructuresGroup` asset loader: RON `Vec<StructureSpec>` → `{specifier, center: [i32;3], custom_indices: HashMap<u8, StructureBlock>}`,
  merged over `default_custom_indices()` (bytes 1–16, the world-reserved band; matches
  ASSET_GAMEPLAY_MARKERS §1 exactly).

### 1b. THE INDEX-SHIFT QUESTION (welded-gate-class trap, engine side)
Engine stores volume index = `dot_vox::Voxel.i + 1` and palette slot `(i+1)` ← dot_vox palette `[i]`.
Whether RON `custom_indices` key K hits raw-XYZI-byte K or K±1 depends on the dot_vox crate's
(v5.2.0, Cargo.lock:2164) `Voxel.i` convention (raw byte vs pre-shifted). The asset side authored
markers as **raw XYZI bytes** (voxlib.py). Ground truth pinned empirically at build time (BEFORE
trusting any marker): load `world/structure/spots/gnarling_totem.vox` + its RON (byte 217 →
`Filled(GlowingRock, …)`) and assert which index resolves. The block's marker-fidelity assert then
guards every asset-lab byte the same way. **Measured, not reasoned from docs** — result recorded in
§7.1 when the first build task runs.

### 1c. StructureBlock → terrain Block — `world/src/block.rs:207 block_from_structure(...)`
Signature: `(index: IndexRef, sblock, pos, structure_pos: Vec2, structure_seed: u32, sample: &ColumnSample, block_for_sprite: Fn()->Block, calendar, units: &Vec2<Vec2<i32>>) -> Option<(Block, Option<SpriteCfg>, Option<&str /*entity path*/>)>`.
- Needs worldgen context: `IndexRef`, `ColumnSample` (grass/leaf recolor), `units` (rotation basis).
- Runtime availability: `Server` owns `world: Arc<World>` + `index: IndexOwned` (private fields,
  `server/src/lib.rs:244-245`) — reachable from `impl Server` bastion hooks. ColumnSample at runtime
  via the barn's own pattern: `land.column_sample(wpos2d, index)` (`world/src/site/plot/barn.rs:42`).

### 1d. Placement is worldgen-time only today — THE SEAM FINDING
- Spot path: `world/src/layer/spot.rs:536 apply_spots_to` → `canvas.blit_structure(origin, structure, seed, units, true)`
  (`world/src/canvas.rs:246`): per-column top-down z sweep, `block_from_structure` per voxel,
  sprite-cfg via `canvas.set_sprite_cfg`, EntitySpawner voxels queue NPC spawns, snow layering.
  Rotation units from `UnitChooser::new(seed)`.
- Plot path (barn.ron route, `world/src/site/plot/barn.rs:108`): `PrefabStructure::load_group` →
  `painter.prim(Primitive::Prefab) ... .fill(Fill::Prefab(structure, origin, seed))` at site render.
- **No runtime structure stamping exists anywhere** (checked `server/src/cmd.rs` — only sprite
  spawning + build-mode). Runtime placement for B-ASSET1 = new bastion seam: iterate the Structure
  volume, convert via the REAL `block_from_structure`, write via `state_mut().set_block` (the same
  authoritative `BlockChange` path B5 work-execution uses; applied at tick end; mesh/rtsim hooks fire).
- **Consequence for B-AG6/autonomous building** (the spec asked): catalog assets CAN be placed at
  runtime through this new seam; the spot/plot pipeline itself resists runtime use (canvas/painter
  are generation-context types). Autonomous building should reuse THIS block's placement fn, not spots.
- Runtime differences documented: (a) SpriteCfg-carrying blocks — worldgen stores cfg in CHUNK META
  (`world/src/canvas.rs:201` → `chunk.meta_mut().set_sprite_cfg_at`); no runtime server write path
  exists → v1 places the plain sprite + logs the cfg drop (backlog: runtime sprite-cfg seam pairs
  with the operable-state block); (b) `EntitySpawner` voxels — worldgen spawns NPCs; the loader
  logs + skips (world-layer scope; creatures are a later rung); (c) snow layering skipped.

## 2. Asset-lab reality (loader input format — the asset session reads this back)

Brief said `asset-lab/vox/real/`; reality (2026-07-09):
- REAL candidates: flattened `.vox` directly in `asset-lab/vox/` (e.g. `structure_housing_human_cottage.vox`).
- Components: `asset-lab/vox/components/<id>.vox` + `<id>.meta.json` (dims, voxels, sha256, markers,
  kind). Marker declaration example (gate): `"markers": {"200": "KeyholeBars(\"lockpick\") / Sprite(DoorBars) via custom_indices"}` — free text, byte → intent.
- Compositions: `asset-lab/manifests/<id>.json` `{id, kind, desc, placements:[{component, offset:[x,y,z], rot∈{0,90,180,270}}]}`;
  **compose.py already pre-flattens compositions to a single .vox in vox/** (castle_fellgate.vox etc.).
  → LOADER DECISION: ingest flattened `.vox` (+ meta/manifest for metadata); per-component placement
  not needed (the flatten step already preserves marker bands — compose.py asserts, ASSET_LESSONS L3).
  Simpler, chosen, documented.
- Marker band law (ASSET_LESSONS L3): bytes 1–16 world semantics, 32–199 literals, **200–255 gameplay
  markers**. Known assignments: 200 gate KeyholeBars/DoorBars, 206 pressure-plate, 207 desk,
  208 bench, 209 bed (`gen/asset17_monastery.py:42`, `gen/asset15_operables.py:84`, `gen/asset02_palisade.py:89`).
- **Format assumption the loader imposes** (documented for the asset session): marker semantics must
  be machine-readable. The loader ships a bastion marker registry (byte → StructureBlock) for the
  known band; per-asset `meta.json` `markers` keys select which bytes are asserted. Free-text values
  are matched against the registry; unknown marker bytes = load-time warning + Filled(Misc) fallback
  (never panic). Function points for dynamic tests derive from marker cells (bed/bench/desk/plate)
  + geometric door detection (2w×3h air gap scan, function_check.py convention) where no marker exists.

## 3. Test asset picks (Part 2 cast)

| asset | file | why | expected |
|---|---|---|---|
| cottage | vox/structure_housing_human_cottage.vox | interior structure; byte 11 MaybeChest interior corner | PASS (reach/traverse/arrive/egress/multi-occ/interior) |
| palisade line | vox/defense_palisade_line_demo.vox (+ components) | wall+gate; byte 200 KeyholeBars | PASS closed-blocks/open-passes (poses = mappings) |
| handcart | vox/prop_handcart.vox | prop | PASS (path around, no over-blocking) |
| smithy | vox/structure_production_smithy.vox | production interior | PASS |
| door-closed room | vox/test_room_door_closed.vox | **the useful FAIL**: closed oak-door pose seals the room → interior function point unreachable | FAIL with reason |
| door-open room | vox/test_room_door_open.vox | the FAIL's control twin | PASS |

Integrated-dynamic spot-check: cottage on real worldgen terrain (site-adjacent, real slope), reachability re-run.

## 4. Harness plan (Part 2 mechanics — mirrors --b4/--b5 scenario patterns)

- Boot recipe: identical to b4/b5 scenario fns (`bastion-harness/src/main.rs:298 ff`): temp data dir,
  `Settings{gameserver_protocols: vec![], auth: None, map_file: None, seed}`, 2-thread tokio,
  `Server::new`, `tick(Input::default(), 1/30s)` closure.
- Arena pad: anchor offset from first site; `bastion_force_load_area` (pins chunks); flatten a pad
  via `state_mut().set_block` (Rock fill below, air above) — `ground_z` MUST filter real terrain
  kinds (the B5 canopy lesson); pad guarantees sill ≤1 and flat approach; vertical-reachability trap
  avoided by design (everything at pad level).
- Goto primitive (new): `comp::bastion::BastionTestGoto` + per-tick assert of
  `NpcActivity::Goto(target, TRAVEL_SPEED)` into `Agent::rtsim_controller.activity` (exactly
  bastion_jobs' Traveling arm) + arrival (3D < ARRIVE_DIST of target) + progress watchdog
  (STUCK_EPSILON/STUCK_TIMEOUT mirrors). Requires extending the rtsim clobber gate
  (`server/src/rtsim/tick.rs` ~737: currently `ActiveJob`-only) to also hold while `BastionTestGoto`
  present. Additive, inert unless the comp is inserted (harness/arena only). serde-ready.
- Reachability measured behaviorally (arrived-within-budget / stuck-watchdog / timeout+reason) —
  the agent's internal A* is not observable server-side; DEVIATION from the spec's "A* completes"
  wording, same invariant in practice.
- CLI: `--asset-test <id|all>` → per-assertion PASS/FAIL + reasons, one JSON line per asset +
  summary line; append machine-readable block to `readme/ASSET_INTEGRATION_LOG.md` (append-only;
  created once by this block). Exit code = all-pass. Assertions per category per
  ASSET_DYNAMIC_TEST_SPEC §per-type.
- Marker fidelity gate runs at LOAD (before any placement): every declared marker byte resolves to
  the registry-intended StructureBlock through the REAL Structure::get path (catches index-shift).
- Malformed-asset policy: log + skip + FAIL that asset id with reason; never panic (spec Part 1).

## 5. Arena plan (Part 3 seams)

- `voxygen/src/cli.rs`: `--asset-arena [asset-id]` (+ optional `--asset-lab-dir`). Flows via
  `global_state.args` (the `bastion_overseer` pattern; arena implies overseer).
- Auto-start: main menu seam at `voxygen/src/menu/main/mod.rs:487` (`MainMenuEvent::StartSingleplayer`
  → `global_state.singleplayer.run(...)`) — arena flag triggers the same path programmatically with
  a THROWAWAY world (temp data dir, default map asset, no persistence; regenerate each boot) instead
  of `SingleplayerWorlds::current()` (`voxygen/src/singleplayer/mod.rs:69 ff`). Vanilla path untouched.
- Server side of the arena: env-var transport (`BASTION_ASSET_ARENA=<id>`, `BASTION_ASSET_LAB_DIR`)
  read once at server boot — deliberately NOT a `server::Settings` field (Settings persists to
  settings.ron; a transient CLI mode must not pollute saved settings). Setup after world ready:
  force-load + flatten pad + load catalog + place asset + move spawn/anchor to pad.
- Controls (B2a message pattern, `common/net` ClientGeneral + `server/src/sys/msg/in_game.rs`
  validation): cycle-asset next/prev, spawn fixture colonist at pad edge (walks to asset interior
  via BastionTestGoto), despawn. Keys in the overseer input context (session/mod.rs key arms
  pattern). B5.6b-1 landed new code in `voxygen/src/bastion/` — symbols re-verified post-merge
  before editing.

## 6. Vanilla-regression guarantees

- All loading behind the flag/env: flag-off boot loads nothing from asset-lab, zero new asset-tree
  files, no vanilla manifest edits (`assets/` untouched entirely — loader reads asset-lab/ directly
  from disk at runtime; that's what keeps the vanilla asset tree byte-identical).
- New comp + message variants additive; `Read<T: Default>` resource-registration gotcha applies to
  any new resource (insert in `Server::new` explicitly).
- rand usage: `rand::RngExt` (project idiom), LendJoin for multi-storage mutation.
- Line-ending discipline per the B5.6b-1 hygiene lesson: no text-mode script edits; new files LF.

## 7. Pre-verified facts + open questions

1. dot_vox `Voxel.i` convention (§1b): **RESOLVED — MEASURED**. The `bastion_dot_vox_index_convention`
   unit test (structure.rs) passes: the totem's GlowingRock voxels carry raw `Voxel.i == 216` for RON
   byte 217, i.e. authored/RON/meta "byte" == `voxel.i + 1` == the engine slot. Asset-lab byte 200
   hits engine slot 200 with NO translation. The test pins this permanently.
2. SpriteCfg at runtime: RESOLVED — worldgen chunk meta only (§1d-a); v1 drops cfg with log.
3. KeyholeBars pathing solidity: **VERIFIED end-to-end** — `path.rs:755 walkable()` requires
   `!is_solid()` at feet+head; `Block::is_solid` (block.rs:504) for sprite blocks =
   `solid_height().is_some()`; `SpriteKind::KeyholeBars.solid_height() = 1.0`
   (sprite/mod.rs:769-773). Closed gate blocks A*. The FAIL-pair room's closed door is solid panel
   voxels — blocks regardless.
4. Runtime "open" pose = alternate custom_indices mapping byte 200 → carved air (Hollow) — same vox,
   two mappings, no asset-side edits.
5. Pad flatten cost: **RESOLVED — MEASURED**: 237k buffered `set_block`s in ~0.03 s; applied within a
   few ticks with no issue. Non-problem.
6. `--asset-test all` wall time: boot ~13 s + per-asset legs a few sim-seconds each (8.7× real-time).

## 8. As-built results + first-live-run lessons (2026-07-09)

- **Cottage FULL PASS (7/7)**: marker fidelity (byte 11 → MaybeChest through the real pipeline),
  1295 blocks placed, reach-interior 4.2s, egress 3.7s, multi-occupancy 3-in 12.6s / 3-out 5.2s,
  integrated-dynamic on natural terrain (slope 3 across footprint) reach 2.8s + egress 2.4s.
- **Lesson (bit run #1): fixed-height pad clearing leaves natural cliff walls** standing inside the
  pad on sloped anchors (colonist STUCK at dist 14.8 mid-pad). Fix: `pick_flat_anchor` (worldgen-sim
  interpolated altitude probe over candidate rings — no chunk loads) + `survey_pad` (real-terrain
  min/max after force-load) + adaptive clear height. Both shared in `server::bastion_assets` (the
  arena uses them too).
- **Lesson: never walk fixtures cross-country** to integrated sites (STUCK 337 blocks out — natural
  terrain traversal isn't the subject). `Server::bastion_teleport_colonist` stages directly.
- **`all` cast scoping**: base-state world-layer assets only. Pose-state files (gate-closed castles,
  sprung traps, raised bridges) rightly fail reach — pose MATRICES pair with the future
  operable-state block; they stay runnable by explicit id. Figure-layer props/items (11 vox/block:
  handcart, gloomcap, maul, armor) are load+fidelity-only — their world integration is
  sprite/item-manifest work, excluded by the vanilla-asset-tree-untouched contract. The world-scale
  path-around assertion runs on flora (rowan, 1 vox = 1 block).
- **Arena controls**: `/bastion_arena next|prev|fixture|dismiss|info` chat command (Admin — 
  singleplayer grants it) instead of new keybinds — command handlers get `&mut Server` directly,
  killing a net-message + input-context + EventBus plumbing chain. Keybinds = backlog nicety.
- **Arena transport**: `BASTION_ASSET_ARENA` / `BASTION_ASSET_LAB_DIR` env vars set by voxygen
  before spawning the singleplayer server thread; arena world = throwaway temp dir, default map,
  seed 1337, spawn point moved onto the pad.

## 9. Full-suite results (--asset-test all + FAIL pair, 2026-07-09 — see readme/ASSET_INTEGRATION_LOG.md)

**26/34 pass; every failure is the fidelity contract catching a real byte-semantics gap:**
- **Wall+gate (defense_palisade_line_demo): FULL PASS** — byte 200 ×8 → KeyholeBars verified;
  closed BLOCKS (watchdog STUCK at best dist 11.3), open ADMITS (2.4s) + egress (2.5s).
  8 unlock SpriteCfgs dropped (documented limitation).
- **Cottage: 7/7** (incl. integrated-dynamic, slope 3). **Flora ×4** (rowan/pine/sapling/snag):
  leaf/fruit world-band bytes verified, path-around/back PASS. **Armor/items/props ×15:
  load-only PASS.**
- **Useful FAIL demonstrated**: `test_room_door_closed` → reach-interior FAIL "STUCK (watchdog)
  after 13.5s, best dist 4.5" (exit 1); its control `test_room_door_open` → 5/5 PASS.
- **CONTRACT CATCHES (for the asset session to reconcile — the block's purpose):**
  1. **Undeclared marker bytes** (asset catalog grew mid-block): 201/202 (smithy v1+v2),
     210 (workshop_carpenter), 211 (mason ×60), 212 (smelter), 213 (tannery), 217 (quarry hall ×13
     + terracotta_set_demo ×22 — presumably GlowingRock per the gnarling precedent). Each fails
     fidelity with "UNKNOWN-MARKER (declare in marker_registry)" — extend
     `server/src/bastion_assets.rs::marker_registry` with agreed semantics, then they pass.
  2. **Byte-8 carve drift (REAL semantic trap)**: quarry hall uses byte 8 ×2227 as interior
     carve-air (the dwarves/entrance.ron convention — but that is a PER-STRUCTURE RON override!).
     Default band maps 8 → `Fruit` (apple/beehive-chance, else keep-existing — NOT carved). On the
     flat pad the tests passed by luck (air already everywhere); on natural terrain the interior
     would not carve. Asset side should re-author carve cells to byte 16 (Hollow) or the registry
     needs a per-asset override mechanism. NOT silently fixed — flagged here + consistency log.
  3. Figure-layer glow bytes (14–16) resolve as world-band Palm/Hollow in load-only checks
     (maul/gloomcap/waystone) — semantically fine (figure-layer assets never take the Structure
     path; the sprite pipeline owns those bands) but the load-only report wording should not
     imply world-band intent. Cosmetic; noted.

## 10. Contract v2 + the graduation sweep (post-resume, isolated worktree, 2026-07-09)

**§9's catches drove the pilot to build the machine-readable contract this block asked for**
(`readme/ASSET_MARKER_REGISTRY.md` opens by citing them): `asset-lab/vox/real/` + `catalog.json`
(category, placement class, arena cast, authored marker CELL COORDINATES, per asset) + per-asset
`.ron` custom_indices sidecars; bytes 200–219 formally allocated; byte-8 carve re-authored to 16;
a content-side staging gate (`asset-lab/gen/markers.py`) now blocks undeclared bytes at source.

**Engine side (this block, contract v2):** catalog-first scanner (legacy fallback kept); sidecar
RON parsed through the game's own StructureBlock deserializer (parse failure = fidelity finding);
`marker_registry` mirrors the authority doc as parse-checked RON strings (sidecars override);
EXACT-CELL fidelity (authored coordinates vs census cells, voxel-for-voxel; >128-cell bytes fall
back to count-only); cast-driven suite dispatch incl. the `work-marker` variant (reach the actual
authored crafting cell); integration-log path anchored to the asset-lab parent (worktree-safe).

**Graduation sweep: 61/64 PASS** (readme/ASSET_INTEGRATION_LOG.md holds per-asset JSON). All
workshops reach their authored work cells with exact-cell matches; flora/structures/faith/depot
green; registry bytes 201–219 resolve as declared; byte-8→16 fix verified. The 3 fails share ONE
root cause: gate sidecars write `200: DoorBars(())` — not a StructureBlock variant; valid form
`Sprite(DoorBars())` (registry doc row carries the same shorthand). Reported to the pilot
(same-day-fix loop); gate DYNAMICS verified via the registry fallback meanwhile — after a
teleport-staging fix (idle fixtures WANDER between poses; one got walled into the yard during the
open-pose rebuild): all three gates closed-block/open-admit/open-egress PASS.

**Deferred-with-reasons** (backlog): figure-scale props' declared casts (11 vox/block — world
placement would be building-sized; needs world-scale versions or the sprite-manifest rung);
`sprite_ladder_rope`'s climb cast (B5.8 verbs not yet in main); pose matrices (DF-MECH).
