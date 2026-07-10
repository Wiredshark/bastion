# Project Bastion — Asset Pipeline & Design Session: MASTER COLLATION

**Purpose:** this doc collates a long design session (2026-07-09) whose history isn't fully in memory. It's
the index + narrative of what was decided, what each companion doc contains, the asset-pilot results, and the
current state — so nothing is lost. Read this first to reorient; follow the pointers into the detailed docs.

---

## 1. Where the project stands (build side)

- **Fork:** Veloren (Rust voxel RPG, GPL-3) → autonomous god-game colony sim. Repo `E:\veloren-master`,
  all design docs in `E:\veloren-master\readme\`.
- **Workflow:** Ben = architect (authors design docs in chat) → pastes into isolated Claude Code builder
  sessions. `bastion/main` advances only by tested + tagged blocks; everything reversible.
- **Build status — LIVE SOURCE is `docs/BASTION_RUN_LOG.md` + `BASTION_ARCHITECTURE.md §6`; this line is a
  snapshot that goes stale, so trust those two, not this.** As of 2026-07-09: `bastion/main` green at
  `bastion-block-B5.6a`; **B5.6b-1** (terrain-conformed zone fills/colors/labels) built + its live-demo bugs
  fixed, tag imminent. Merged chain: B0–B5, B5.5, B5.6a. `readme/BASTION_ARCHITECTURE.md` EXISTS (living
  B0–B5.6a map). Queue: B5.6b sub-blocks (b-1 fills → **b-2** z_extent/volumetric → **B5.8** vertical mobility
  → b-3/b-4) → **B5.10** gait (walk-default/sprint-reserved) → **B6** (stockpiles/hauling + individual
  stat-scaled carry). Independent: B-MAP1 (minimap), B-ASSET1 (asset integration), B-TESTBED.
- **Vertical-reachability trap: 4 bites** (B5 ×3, B5.5 ×1) — B5.8 is the fix; mine framework supersedes
  with planned access later.
- **B5 landed with 3 bugs found + fixed**, all one root cause (no climb/ramp modeling for vertical terrain
  beyond ARRIVE_DIST ~2.5 blocks): (1) colonists auto-looted their own drops via vanilla Humanoid pickup AI
  → gated pickup off for `comp::Colonist`; (2) B4 regression from B5's shared `bastion_jobs.rs` changes
  (Arrived became transient) → B4 harness now tracks cumulative ever-arrived/ever-unreachable; (3) mine-pit
  exit-ramp trap (colonist stuck in hollowed pit) → harness carves a staircase. **Backlog item flagged:
  tall-structure / vertical reachability (climb/ramp modeling) — a real mechanism gap, promote near B6/B7.**
- **A concurrent agent is modifying game systems + the asset tooling** — the two workstreams stay isolated
  (asset work writes only `asset-lab/` + `readme/`; the B5 session correctly left the concurrent asset work
  untouched and uncommitted). Confirmed working discipline.
- **DEFERRED (do first next build session):** the B5 **in-game visual demo** — paint Mine/Chop/Build and
  watch it render live. Ben has still never watched the loop render; highest-value five minutes.

## 2. The design docs (companions — what each holds)

- **`veloren-colony-rts-build-report.md`** (v2.2) — the master design doc. Block queue B0→B13 + phases.
  Key canon: §1a influence-not-command pillar; §3d the **Control/Embodiment Spectrum** (5 lenses:
  Autonomous god → DF-manage → RTS-command → god-embodied RPG avatar → mortal-RPG capstone — "one living
  world, many lenses"); §4 the overseer = invisible-player-entity-not-spectator directive (built + working;
  god mode = world-ignores-avatar + can't-die).
- **`agency-bible.md`** — the NPC mind: facets/values(+conflicts), memory-drift, FOCUS, relationships,
  mind-LOD. The DF "soul."
- **`df-feature-gap-ledger.md`** — DF systems tagged DONE/SUBSTRATE/GAP with DF-* IDs + tiers. §E: traps +
  mechanisms + operable terrain = ONE trigger→link→effect engine; Veloren has no player trap system.
- **`divine-politics-bible.md`** — trade + diplomacy + war as one faith-modulated system; competing gods.
- **`cross-genre-nice-to-haves.md`** — borrows from god games/RimWorld/city-builders/RTS, Adopt/Adapt/Avoid.
  Top: RimWorld director (rival gods as storytellers), Reus over-godding penalty, WorldBox mind-powers,
  Populous terrain raise/lower, alerts+overlays.
- **`comprehensive-feature-gap-analysis.md`** — the master union of gaps across all three genres + blind
  spots. Flags **legibility as a PILLAR** (god games fail here), day/night+seasons as gameplay, death/burial,
  terrain power, god-moral layer ("world remembers you").
- **`future-work-and-deferred-ideas.md`** — THE CATCH-ALL. Fluid physics, dialogue/voice, autonomous building
  (template catalog, carrying-capacity, site-suitability), autonomous civ advancement (tech-as-history),
  **and the entire asset pipeline (§3e–§3m)** — see §4 below.
- **`MEGA-PROMPT-autonomous-batch-builder.md`** — the build-queue runner (one block at a time, checkpoint +
  self-test + commit-or-rollback). Has per-block bookkeeping + consistency-audit directives.

## 3. Design decisions locked THIS session (beyond the asset pipeline)

- **The Embodiment Spectrum** (design doc §3d) — 5 lenses god→mortal on ONE sim. God-embodied RPG (extends
  B12 possession) + divine quest-giving; mortal-RPG capstone (no player-god, the final exam for the agency
  systems, arguably the most marketable output). Mount-&-Blade layer as long-term elaboration.
- **Autonomous civilizational advancement** (future-work §3f) — tech as *world-history*, NOT a management
  tree. Civilizations advance on their own; tech level selects which building-catalog tier they build; a god
  shepherds it. (Player-micromanaged research = still AVOID; autonomous advancement = good fit.)
- **The four walls framing** (future-work §3g) — content / simulation / design-fit / legibility. The asset
  unlock lifts ONLY the content wall; test every "can we now do X?" against which wall blocked it.
- **Autonomous building** (future-work §3c) — template catalog (race-keyed, reusing Veloren's culture-distinct
  structures), ascending tiers (fixed→parameterized→composed), carrying-capacity bounds (not magic caps),
  site-suitability placement. Walls/gates = LINE-ENCLOSE placement; player-designated first, smart
  autonomous fortification later; operable gates later.
- **Experimental feature-flagging** — foundational-but-risky systems (worldgen for islands/biomes, fluid)
  live behind flags on separate paths; base game never at risk; promote to default after isolated proof.
  Islands flagged as a strong design-fit (god genre's native geography, ties ships + carrying-capacity +
  divine territory).
- **Ships / biomes / islands** — ship *assets* now viable (airship substrate exists); *new biomes/islands* =
  worldgen changes, not assets. "Which wall" tagging applies.
- **Control spectrum generalizes across ALL colony domains** (future-work §3q) — construction, governance,
  military, economy each playable at autonomous/policy/direct level, player chooses involvement per domain.
  Construction's 3 modes (automatic / zoning / direct-placement) are the worked example. Governance = "ruler
  stuff," with the **god ≠ ruler** distinction (god influences; a mortal ruler decides) resolved by mode
  (autonomous AI-ruler-you-influence / manage-some-decisions / be-the-ruler via embodiment). **STANDING
  GUARDRAIL: autonomous is the default + soul; manage/direct are optional depth, never mandatory — or the
  game drifts into 4X/management (AVOID).**
- **Zone ↔ asset shared taxonomy** (future-work §3e-schema) — zone types = the asset structure-purpose
  enumeration (residential→housing, industrial→production, religious→faith, etc.). Lock the shared vocabulary
  NOW (cheap); build the zoning system later (B-AG6 + DF-ZONES). Soft-preference, not iron law. Next asset
  session tags `purpose` from this zone-compatible vocabulary.
- **World connective tissue** (future-work §3s) — the inter-settlement layer: territory/region tracking (the
  substrate, extends rtsim factions, could come earliest) → roads/bridges (autonomous road-building; roads
  channel trade+armies; bridges = chokepoints) → spatialized Divine Politics (trade on physical routes,
  territorial war) → sea lanes (gated on naval) → daughter settlements. Constraints: the map IS the interface
  (legibility), world-tier abstract LOD, everything-must-function, god-influence angle.
- **World-sim framework research** (future-work §3t) — deep-search catalog of steals: CK3 de-jure/de-facto +
  casus-belli (wars always legible) + opinion sums; DF overlapping claims + time-scrub political map + ages +
  history-continues-during-play; Dominions' dominion-spread (faith as territorial field — THE competing-gods
  mechanic); Distant Worlds' autonomous private economy (purest influence-not-command economy); X4/Elite-BGS
  physical-vs-abstract economy per LOD; Victoria pops; Syx requirement-based soft annexation + knowledge
  regression (the §3f anti-runaway); Total War supply/attrition (roads matter militarily) + admin scaling
  (anti-blob); Qud's conflicting historical accounts (unreliable chronicle); Nemesis-style recurring rivals.
  Meta-lesson: every framework splits abstract-world-tier from concrete-local — exactly Bastion's existing
  loaded↔simulated architecture; everything slots in.

## 4. THE ASSET PIPELINE — the session's biggest build-out (future-work §3e–§3m)

The arc: "Claude can make voxel art" → a fully-specified, self-verifying, human-correctable,
animation-capable, component-based content factory. Read future-work §3e–§3m for detail. Summary:

- **§3e Pipeline + maturity roadmap** — reference-ground → generate → verify → iterate. Roadmap: palette-ramp
  extraction → parts library → parameterized generators → stat-linked creatures (DF forgotten-beast style).
- **§3f Autonomous civ advancement** (tech-as-history, above).
- **§3g The four walls** — what the content unlock does/doesn't open.
- **§3i Delegation** — every asset tagged `READY` (existing system consumes it) vs `NEEDS:<system>` (inert
  until code exists). The tagged catalog is the interface between content pipeline + build queue; finishing a
  system unlocks an asset batch.
- **§3j Functional harness** — STYLE (looks right) vs FUNCTION (works right) are orthogonal. STATIC checks
  (geometry, content-side) + DYNAMIC checks (real NPC paths in with collision, game-side via B0/B4 harness).
  Full test taxonomy: reachability/traversal/arrival/egress/multi-occupancy/interior-function + determinism,
  save-load, LOD/mesh, placement, reserved-index, collision-coherence, scale, performance. READY = static +
  dynamic + soak.
- **§3k Debug mode** — toggleable in-world overlays: provenance (vanilla vs Claude-generated, via a `source`
  schema field — ADD EARLY), identity labels, navmesh/collision, agent-state. Build early (dev infra).
- **§3l Animation** — SKELETAL, code-defined per body family; parts are bones. Generate-to-skeleton =
  inherit animation FREE; new body plan = new skeleton = code (NEEDS:animation).
- **§3m Component system + human editor** — each big-asset chunk = a persisted, addressable, individually-
  manipulable asset; composition manifests position them (mirrors the game's parts+manifest pattern); a
  registry = the parts library. ONE mechanism = big-asset chunking + parts library + variation. Human GUI
  editor (`editor.html`) fixes mistakes by hand; a change log Claude reads back to verify + learn from Ben's
  corrections.

## 5. The asset PILOT — what actually happened (results, since memory lacks this)

A pilot ran against the real repo. Key outcomes:
- **Census done** (`readme/*TAXONOMY*`): ~4,700 `.vox` files, MagicaVoxel v150/v200, per-file RGBA palettes
  (no global palette), TWO layers (voxygen figure ~8–25 vox/block vs. world 1 vox=1 block). ASSET-vs-WORLDGEN
  split confirmed: towns/dungeons/caves are PROCEDURAL code; hand-authored .vox = creatures/armor/weapons/
  sprites/spot-prefabs. **Creatures are COMPONENTIZED** (part-`.vox` folders + per-family RON manifests with
  offsets + skeleton/animation sets) — ~15 body families. Structures use reserved palette indices +
  custom_indices markers (chest/spawner/door/hollow). Spot/plot prefab pattern (barn.vox) = the .vox-native
  path for hand-authored buildings.
- **A viewer was built** (`asset-lab/viewer.html`) — orbit/zoom, asset list, metadata panel, humanoid-scale
  ghost + ground-grid toggles.
- **First asset: a human timber cottage** — STYLE-CHECKED 9/9 (real palette, chimney, windows, 17 colors) BUT
  **too small — a colonist couldn't fit inside** (the scale ghost showed it). This was the pivotal finding:
  **style and function are orthogonal; we were only checking style.** It forced the function harness + the
  dimensional framework baseline (interior clearance ≥ 3 blocks, door ≥ colonist height, keyed to the real
  collision box). Prototyped `asset_style_check.py` (caught flat shading via the ramp axis) and
  `asset_function_check.py` (FAILED the too-small house, PASSED a corrected one) — both validated in the
  design session.

## 6. The prompts (which to paste when)

- **`MASTER-asset-tooling-prompt.md`** — THE ONE TO USE for asset tooling. Builds the component system +
  harnesses + human editor + animation via a 9-rung progressive test ladder, ISOLATED from the concurrent
  build agent (writes only to `asset-lab/` + `readme/`, never touches game files or main branch). Research-
  first. **Runs AUTONOMOUSLY** — no per-step approval; checkpoints each rung/asset, self-verifies with the
  harnesses, only stops on unfixable failure / forbidden game-file edit / context limit. Ben reviews wherever
  it lands. **Maintains `readme/ASSET_SYSTEM_GUIDE.md`** so future amnesiac sessions understand the asset
  system. Supersedes the earlier `asset-lab-claude-code-prompt.md`, `asset-generator-prompt.md`, and
  `component-system-prompt.md` (archival stepping-stones).
- **`MEGA-PROMPT-autonomous-batch-builder.md`** — the game-build-queue runner (separate workstream, the other
  agent / manual B5+). One block at a time, checkpoint + self-test + commit-or-rollback + tag. **Maintains
  `readme/BASTION_ARCHITECTURE.md`** so future amnesiac sessions understand how the game's systems work.

**Two living system guides (the "how it all works" maps, written BY the autonomous sessions FOR future
sessions):** `readme/BASTION_ARCHITECTURE.md` (game systems) + `readme/ASSET_SYSTEM_GUIDE.md` (asset system).
A fresh session reads its guide + the design docs + logs and can continue correctly with zero memory.

## 7. Standing principles that govern everything (don't lose these)

- **Which wall?** Before "can we do X now?" — is X blocked by content (asset unlock helps), simulation (hard
  systems, unhelped), design-fit (wrong genre, unhelped), or legibility (harder with more content)? Only
  content just fell.
- **Style ≠ function.** Every asset needs BOTH a style pass and a function pass. Pretty-but-unusable is a fail.
- **Static ≠ dynamic.** Measure geometry (cheap, content-side) AND prove a real NPC paths in (truth,
  game-side). READY needs both + soak.
- **Build once, many uses.** Recurring: trigger→link→effect (traps/mechanisms/gates); Hazard Events
  (timber/flood/lava/rockfall); the component system (chunking/parts-library/variation); the world-verb
  library (colonist jobs + NPC drives).
- **Generate-to-skeleton, inherit animation.** New creatures in existing families animate free; new body
  plans need code.
- **TEST vs REAL, logged, append-only.** Every asset tagged; every generation/edit/rejection logged; nothing
  overwritten.
- **Isolation from the concurrent agent** — asset tooling writes only `asset-lab/` + `readme/`, never game
  files, never main branch, never `git add -A`.
- **Legibility is a pillar, not a feature.** A deep sim nobody can read is a failed game.
- **Content unlock aims at density/variety of the world you already want — not at bolting on off-genre
  features.**

## 8. Open watch-items / next actions

- **DO FIRST next build session:** the deferred B5 **in-game visual demo** (paint Mine/Chop/Build, watch it
  render live) — Ben has never watched the loop render. Then B6 (stockpiles/items/hauling).
- **Vertical reachability gap (climb/ramp modeling)** — the root cause behind all 3 B5 bugs; no path
  modeling for terrain beyond ARRIVE_DIST (~2.5 blocks) up or down. A real mechanism gap, currently
  worked around in test geometry. **Promote to a real backlog item near B6/B7** (hauling + multi-level
  structures will hit it hard).
- **Confirm the colonist collision box** by research before trusting the asset function harness numbers.
- **The dynamic-pathing asset tests are game-side** — spec'd by the asset tooling, run by the B0/B4 harness
  on integration. Wire when an asset is first integrated.
- **Next build session's FIRST action:** the one-time catch-up doc pass → `readme/BASTION_ARCHITECTURE.md`
  documenting B0–B5, before building B6 (per the mega-prompt).
- **Next asset session's FIRST action:** catch up on the pilot's state → `readme/ASSET_SYSTEM_GUIDE.md`,
  then continue the ladder/generation. Tag asset `purpose` from the zone-compatible vocabulary.
- **God-anchor aggro live-fire** still pending (fold into B8).
- **`docs/` vs `readme/` split** — findings in `docs/`, design + bookkeeping in `readme/`; sessions check both.

## 9. THE README CHECKLIST — every file that belongs in `E:\veloren-master\readme\` (definitive)

**Core design docs (7):**
1. `veloren-colony-rts-build-report.md` — master design doc v2.2 (blocks, embodiment spectrum, pillars)
2. `agency-bible.md` — the NPC mind
3. `df-feature-gap-ledger.md` — DF-* backlog
4. `divine-politics-bible.md` — trade/diplomacy/war/competing gods
5. `cross-genre-nice-to-haves.md` — Adopt/Adapt/Avoid
6. `comprehensive-feature-gap-analysis.md` — master gap union + blind spots
7. `future-work-and-deferred-ideas.md` — THE catch-all (§3a–§3x: fluid, dialogue, autonomous building,
   civ advancement, the ENTIRE asset pipeline, embodiment, founding, harness, visual-check, control
   spectrum, custom-creature unlocks, world tissue, framework research, action animations, 3D zones +
   mining, colony boundary, site-prep + roads + terrain-following selection)

**Reference / orientation (2):**
8. `MASTER-COLLATION-index.md` — THIS doc; read-first reorientation
9. `BASTION-SYSTEM-FRAMEWORKS.md` — one-stop frameworks reference (points into future-work)

**Prompts (3 active):**
10. `MEGA-PROMPT-autonomous-batch-builder.md` — game build queue (UPDATED: B5 merged, B5.5 inserted,
    frameworks doc added to inputs) — **the one to paste for the next build session**
11. `MASTER-asset-tooling-prompt.md` — asset workstream (autonomous, isolated, catch-up-doc first)
12. `B5.5-zone-delete-drop-aggregation-prompt.md` — the patch block (or let the runner pick it up)

**Harness prototypes (2, reference implementations for the asset sessions):**
13. `asset_style_check.py` · 14. `asset_function_check.py`

**Archive (superseded — keep in `readme/archive/` or delete; the MASTER supersedes all three):**
`asset-lab-claude-code-prompt.md`, `asset-generator-prompt.md`, `component-system-prompt.md`,
(`bastion-loop-runner.ps1` — parked, loop abandoned in favor of manual/batch sessions)

**Generated BY sessions (should already exist in-repo; verify presence, don't overwrite):**
`BASTION_BACKLOG.md`, `BASTION_RESTORE_LEDGER.md`, `BASTION_CONSISTENCY.md`, `ASSET_GENERATION_LOG.md`,
`COMPONENT_REGISTRY.md`, `COMPONENT_SYSTEM_LOG.md`, `HUMAN_EDIT_LOG.md`, `ASSET_REJECTION_LOG.md`,
`ASSET_STYLE_GUIDE.md`, `ASSET_GAMEPLAY_MARKERS.md`, `ANIMATION_RESEARCH.md`, `ASSET_SYSTEM_GUIDE.md`,
taxonomy/census docs, `ASSET_DYNAMIC_TEST_SPEC.md`; plus the pending `BASTION_ARCHITECTURE.md`
(next build session's FIRST action creates it).

*This collation is the reorientation point. Everything above lives in the companion docs in `readme/`.*
