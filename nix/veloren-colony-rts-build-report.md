# Project Bastion — Turning Veloren into a Top-Down Dwarf-Fortress / RimWorld RTS

**A build & test directive for Claude Code**
Version 1.0 · Architect-authored design doc · Target: fork of `veloren/veloren` (Rust, GPL-3)

---

## 0. How to use this document

This is written for the architect→builder workflow: this document is the **architect artifact**. Each build block (B0–B11) in §6 is scoped so it can be lifted more-or-less whole into an **isolated Claude Code builder session** against the fork. Blocks declare their own objective, the files they touch, the approach, and — critically — a **Done-when** contract with concrete tests. Builders should not proceed past a block's Done-when gate.

Ordering is deliberate. B0–B2 establish the shell (build + camera + input) so you can *see* the world top-down before touching simulation. B3–B8 build the colony simulation. B9–B11 are the UI/persistence/scenario layer that make it a game rather than a tech demo. §9 defines the vertical slices if you want a playable thing sooner than block-linear order allows.

**Naming:** the working title is *Project Bastion*. New crates/modules use the `bastion_` / `bastion::` prefix so a `grep` cleanly separates our code from upstream Veloren, which matters for merging upstream changes later.

---

## 1. Executive summary — the core thesis

Veloren is **not one program**. It is a headless simulation stack (`common` + `world` + `rtsim` + `server`) with a *swappable* frontend (`voxygen`). Veloren's own architecture documentation explicitly says a frontend could render the world "using isometric graphics, Dwarf Fortress style layered 2D graphics, or any other representation one could imagine," and anticipates a Legends-Viewer-style history browser. We are not fighting the engine's grain; we are building the thing its authors left the door open for.

That gives us a **two-track transformation**, and the tracks are largely independent:

1. **Frontend track** — reconfigure `voxygen`'s third-person, avatar-centric camera and input into a **top-down orthographic RTS camera** with **selection + command + designation** input. No simulation changes required to *look* like an RTS.

2. **Simulation track** — invert the control model. Vanilla Veloren = *one* player avatar + ambient NPCs. We want *no* player avatar and *many* player-directed colonists, plus a **designation → job → task** loop (mine/chop/build/haul/cook/fight) with **needs, stockpiles, and mood**. This is a colony-sim layer bolted onto two substrates Veloren already ships:
   - the **ECS** (`specs`, struct-of-arrays) on the `server` for *loaded*, high-resolution, per-colonist simulation, and
   - **`rtsim`** (real-time sim) for *low-resolution* world-scale simulation of everything off-screen — factions, migration, economy, resource depletion — which already exists and is exactly the Dwarf-Fortress "the world lives without you" layer.

The single most important design decision in this whole document: **colonists you are actively managing live in the ECS; the wider world lives in rtsim; and the boundary between them is `SimulationMode` (loaded ↔ simulated), which Veloren already implements.** Do not reinvent world simulation. Wrap it.

---

## 2. What Veloren actually is (architecture map)

Grounded facts about the codebase, so builders reference real crates and symbols rather than guessing.

**Crate structure** (independent Rust crates, parallel-compilable):

| Crate | Role | Our use |
|---|---|---|
| `common` | Shared types, ECS **components**, physics, terrain (voxel "chonks"), items | Heavy reuse + new components |
| `common/src/rtsim` | Interface types between rtsim and the game (`NpcId`, `SiteId`, `Actor`, `NpcAction`, `NpcActivity`, `RtSimController`) | The bridge for our colony ↔ world coupling |
| `world` | Procedural worldgen + site/civ generation; runnable **headless** | Reuse for embark map + starting site |
| `rtsim` (`veloren_rtsim`) | Real-time, low-resolution simulation of the **entire** world, even unloaded chunks. Rule/event based (`impl Rule`, `OnTick`, `npc_ai.rs`). Flat-table data model, **not** ECS, for cheap persistence | Extend for world-scale colony effects, threats, migration |
| `server` | Authoritative game state, ECS tick, event injection, plugins | Where our per-colonist job/needs systems live |
| `client` | Headless, display-agnostic client library; `handle_input` entry point | Add colony commands to the input surface |
| `voxygen` | Default 3D frontend (wgpu). Camera in `voxygen/src/scene/`, input bindings in `voxygen/src/settings/control.rs` (`GameInput` enum), UI in `voxygen/src/ui/` | Camera + input + HUD rework |
| `network` | Transport (Tcp/Udp/Quic/Mpsc) | Untouched (singleplayer uses Mpsc) |
| `server-cli` | Headless server frontend | Our test harness anchor |

**ECS:** `specs`, struct-of-arrays for cache coherency. Entities are dynamic component bags; components live in `common/src/comp/`. Systems tick on the server.

**Input flow** (know this cold — B2 depends on it): `voxygen` reads winit events → maps via `GameInput` (bindings in `voxygen/src/settings/control.rs`) → calls `client.handle_input(...)` in `client/src/lib.rs` → sends to `server` over the network layer. Terrain edits already exist client-side (`client.remove_block(pos)`), which is the seed of our mining/building.

**rtsim specifics** (the colony-sim goldmine):
- Simulates the whole world efficiently; tick rate varies by distance from loaded regions.
- Deliberately **does not** handle items, combat, fine movement, physics, or small-scale spawning — those are the ECS/server's job. This is *exactly* the division of labour a colony sim wants.
- NPCs carry a `SimulationMode` (loaded as a physical ECS entity vs. abstractly simulated).
- Roadmap already targets factions, diplomacy, population growth, migration, economy, resource-depletion tracking (depletion already done). We inherit momentum here.

**Reality anchors for the builder** (not obstacles — planning inputs):
- **Compile times & assets.** This is a large Rust workspace with git-lfs assets. Cold builds are long; iteration discipline (per-crate builds, `cargo check`, headless tests) is mandatory. B0 exists to make this bearable.
- **License.** GPL-3. Any distributed fork must ship source under GPL-3. Fine for a personal/creative project; just know it.
- **UI toolkit.** voxygen's in-game UI has historically been `conrod`-based (`voxygen/src/ui/`). Verify the current toolkit in-tree before B9 and match it; do not introduce a second UI framework unless a block explicitly says to.

---

## 3. Target design — what "top-down DF / RimWorld RTS" means, concretely

Pin the target so builders don't drift. The game we are building has these properties:

**Camera & view.** Fixed top-down / high-oblique orthographic view over a bounded embark region. Pan, zoom, 90°-step rotate. A **Z-layer / depth slice** control (DF's most important interaction) so the player can dig down and view lower layers — Veloren is fully 3D voxel, so "cut away everything above layer Z" is a render filter, not new geometry.

**Unit of play.** The player controls a **colony**: a starting band of ~5–8 colonists (Veloren humanoid `Body` reused). No single hero avatar. The camera is detached from any entity.

**Embody (possession).** At any time the player can **jump into any colonist and play it normally** — dropping from the top-down RTS view into stock Veloren third/first-person control (WASD, mouse-look, real combat, interaction), then releasing back to autonomous AI and the RTS view. This is a first-class feature, not a debug tool. Crucially it costs almost nothing to build: vanilla Veloren already *is* "control one humanoid," so possession is a **mode switch** over machinery we deliberately kept alive, not new gameplay. See B12.

**Core loop (RimWorld/DF spine):**
1. Player **designates** work on the world (dig this rock, chop these trees, build a wall here, haul this there, create a stockpile zone, create a room).
2. Colonists have **jobs/skills** with per-colonist **priorities** (RimWorld work-tab grid). They **claim** designations that match their allowed work, pathfind, and **execute** (edit terrain, carry items, construct).
3. Colonists have **needs** (hunger, rest, recreation, mood/`sanity`). Unmet needs → debuffs → breakdowns.
4. **Stockpiles & production chains**: raw resource → hauled to stockpile → workshop job → product. Start shallow (wood/stone/food), deepen later.
5. **Threats**: bandit raids (rtsim already models bandits), wildlife, weather/seasons. Player switches to **RTS control** — select colonists, direct them to fight/flee/take cover — reusing Veloren's real combat.
6. The **world persists and lives** off-screen via rtsim (other factions grow, trade caravans arrive, resources deplete, history accrues → a Legends-style chronicle).

**Explicit non-goals for v1** (cut scope ruthlessly): multiplayer, the solo-hero RPG campaign/quest progression, mounted/airship content. Feature-flag them off; don't delete (keeps upstream merges sane). **Note: direct first/third-person play is explicitly *in* scope — but as *possession* of a colonist (see B12), reusing vanilla avatar controls, not as a separate hero character.**

---

## 4. Transformation strategy — two tracks, one boundary

```
        FRONTEND TRACK                         SIMULATION TRACK
 ┌──────────────────────────┐        ┌──────────────────────────────────┐
 │ voxygen                  │        │ server (ECS, loaded colonists)    │
 │  • RTS ortho camera (B1) │  cmds  │  • Colonist comp + needs (B3,B7)  │
 │  • Z-slice render (B1)   │ ─────► │  • Designation/Job systems (B4)   │
 │  • Selection/box (B2)    │        │  • Work execution: dig/build (B5) │
 │  • Designation tools (B2)│ ◄───── │  • Stockpiles/hauling/items (B6)  │
 │  • Colony HUD (B9)       │ state  │  • Threats/combat/RTS ctrl (B8)   │
 └──────────────────────────┘        └───────────────┬──────────────────┘
                                                      │ SimulationMode
                                                      │ (loaded ↔ simulated)
                                              ┌───────▼──────────────────┐
                                              │ rtsim (world-scale)       │
                                              │  • factions, migration    │
                                              │  • economy, depletion     │
                                              │  • threats spawn, history │
                                              └───────────────────────────┘
```

**The boundary rule.** Anything inside the active embark region and currently relevant = ECS entity on the server (full fidelity). Anything outside, or dormant = rtsim record (cheap). Colonists that wander off the map edge, caravans that leave, enemies that retreat → demote to `SimulationMode::Simulated`. This is how you get a DF-scale living world without simulating 10,000 entities at 60 Hz.

---

## 5. Gap analysis — reuse / bend / build / discard

**Reuse as-is:** voxel terrain + chonks; terrain block add/remove; humanoid bodies & animation; item/inventory system; pathfinding primitives (verify current impl — `common` has movement/nav); combat & health/stats; rtsim persistence; worldgen & site generation; networking (singleplayer Mpsc).

**Bend (reconfigure existing):** camera (3rd-person → ortho top-down); input (`GameInput` avatar controls → RTS selection/command); NPC AI patterns in `rtsim/src/rule/npc_ai.rs` → colonist task AI; `RtSimController`/`comp::Controller` (already the "abstract intent → action" interface — perfect for job execution).

**Build new:** `Colonist` component (needs/skills/priorities); designation model + placement; job board / claim / arbitration; stockpile zones & hauling; work-execution systems that emit terrain edits & item moves; RTS selection state & orders; colony HUD; Z-slice render filter; embark/scenario setup.

**Discard/flag-off (do not delete):** the hero character-creation *flow*; questing; trade UI (repurpose later). **Explicitly keep** the avatar-control machinery itself — vanilla camera, input bindings, and controller routing — because Embody (B12) reuses all of it to let the player jump into any colonist. The thing we drop is the *assumption that there must be exactly one, permanently player-controlled hero*, not the ability to control a humanoid.

---

## 6. Build blocks

Each block: **Objective · Touches · Approach · Done-when (tests).** Builders work one block per session.

### B0 — Fork, baseline build, and the headless test harness
**Objective:** Reproducible build of vanilla, plus a fast headless loop so simulation logic can be tested without the renderer.
**Touches:** repo root, CI/scripts, `server-cli`, `world` examples.
**Approach:**
- Fork `veloren/veloren`. Pin to a known-good tagged release commit (check `releases`), record the exact SHA in `BASELINE.md`. Ensure git-lfs assets pull.
- Get `cargo build --bin veloren-voxygen --bin veloren-server-cli` green. Run vanilla singleplayer once to confirm.
- Stand up a **headless sim harness**: a small binary/example that boots `world` + `rtsim` + a minimal `server` **without voxygen**, seeds a fixed world, ticks N times, and dumps state. This is where 90% of colony-sim testing will happen (fast, deterministic, no GPU).
- Add a `bastion-check` script: `cargo check -p veloren-common -p veloren-server -p veloren-rtsim` for the fast inner loop.
**Done-when:**
- `cargo build` (both bins) succeeds on a clean checkout; SHA recorded.
- Vanilla singleplayer launches and renders a world.
- Headless harness ticks a seeded world 1000 rtsim ticks and prints a deterministic NPC/site count twice in a row (determinism check).

### B1 — Top-down orthographic RTS camera + Z-slice
**Objective:** See the world top-down with pan/zoom/rotate and a depth-slice control. Vanilla camera still selectable behind a feature flag.
**Touches:** `voxygen/src/scene/camera.rs` (+ wherever the projection/view matrices feed the renderer), a new `bastion` camera mode, terrain draw path for the slice filter.
**Approach:**
- Add `CameraMode::Rts` alongside existing modes. Orthographic projection; fixed pitch (start ~55–70° oblique — pure 90° top-down reads poorly with voxel walls); target = a ground point, not an entity.
- Controls: WASD / edge-scroll pan; scroll = zoom (clamp); Q/E = 90° rotate; PgUp/PgDn (or `[`/`]`) = **Z-slice** cursor.
- **Z-slice render:** discard/gray voxels above the current slice Z so digging down is visible. Cheapest correct approach: a uniform `max_z` passed to the terrain shader that clips fragments above it; refine later to a fade band.
**Done-when:**
- Launch flag puts you in ortho top-down over a generated world; pan/zoom/rotate feel responsive (>50 fps on a mid GPU).
- Z-slice cursor hides everything above the chosen layer and reveals interior/underground voxels.
- Vanilla 3rd-person mode still works when flag off (no regressions).

### B2 — Selection & command input (RTS control surface)
**Objective:** Click-select an entity, drag-box multi-select, right-click to issue a ground order; a designation cursor mode. This is plumbing, not yet behavior.
**Touches:** `voxygen/src/settings/control.rs` (`GameInput` additions), voxygen input handling, `client/src/lib.rs` (`handle_input` → new colony commands), a new `bastion` message type through `network`/`server`, a `Selectable`/`Selected` component in `common/src/comp`.
**Approach:**
- Add `GameInput` variants: `Select`, `SelectAdd`, `BoxSelectDrag`, `CommandMove`, `DesignateApply`, `DesignateCancel`, plus a designation-tool cycle.
- Screen→world ray/pick: reuse voxygen's existing block/entity targeting (the `build_target`/`nearest_block` machinery referenced in input handling) to resolve clicks to a voxel or entity.
- Drag-box: track down/up screen coords, project frustum slice, mark ECS entities with `Selected`.
- New client→server messages: `IssueOrder { targets: Vec<Uid>, order }` and `PlaceDesignation { region, kind }`. Server validates & stores; behavior arrives in later blocks. For now, orders just log server-side and move a debug marker.
**Done-when:**
- Left-click selects a highlighted entity; drag-box selects several; Shift adds.
- Right-click on ground with a selection sends a `CommandMove` the server logs with correct world coords.
- A designation tool paints a marked region the server receives and echoes back for render (colored overlay).

### B3 — Colonist entity model & starting colony
**Objective:** Define a colonist and spawn a player-faction starting band. No jobs yet — just entities that exist, are yours, and are selectable.
**Touches:** `common/src/comp/` (new `Colonist`, `Needs`, `Skills`, `WorkPriorities`, `Faction`/ownership tag), server spawn logic, rtsim actor linkage.
**Approach:**
- `Colonist` component: name, backstory flags, `Skills { mining, construction, cooking, melee, ... : level+xp }`, `WorkPriorities` (per work-type 0–4, RimWorld-style), `Mood`.
- `Needs { hunger, rest, recreation }` as 0.0–1.0 clocks (decay handled in B7).
- Reuse humanoid `Body`, `Health`, `Stats`. Tag with a `PlayerColony` ownership marker so selection/orders only affect yours.
- Spawn 5–8 at embark near the chosen site; register each as an rtsim `Actor` so they can demote to simulated when off-map.
**Done-when:**
- Headless harness spawns a colony of N colonists with randomized names/skills; dumps a roster.
- In voxygen, the band appears top-down, is box-selectable, and non-colony NPCs are visibly distinct (color/marker).
- A colonist that leaves the loaded region demotes to `SimulationMode::Simulated` and re-promotes on return (log-verified).

### B4 — Designation → Job board → claim/arbitration
**Objective:** The colony-sim heart. Player designations become jobs; colonists claim jobs by priority/skill/distance and path to them. Still no work *effect* (that's B5) — the loop up to "arrived at job site, ready to work" must be solid first.
**Touches:** new `bastion_jobs` module in `server`; `common` designation/job types; colonist AI system; integrate with `Controller`/`RtSimController` intent interface.
**Approach:**
- **Designation kinds** (v1): `Mine(voxel)`, `Chop(tree)`, `Build(blueprint_voxel)`, `Haul(item→zone)`, `Deconstruct`. Stored as a spatial set on the server; mirrored to client for overlay (B2 already carries the channel).
- **Job board:** each designation spawns one or more `Job`s. A `Job` has kind, location, required work-type, optional skill floor, and a `claimed_by: Option<Entity>`.
- **Arbitration system** (ECS system, runs a few Hz not every tick): for each idle colonist, pick the highest-priority reachable unclaimed job matching an allowed work-type, tie-broken by distance then skill. Claim atomically. Release on interruption/failure.
- **Pathing:** drive the colonist via the existing controller/movement so it walks to the job site. Reuse Veloren nav; if 3D nav is heavy, constrain to a navmesh over walkable surface + Z-slice.
**Done-when:**
- Headless: place 20 mine designations + 5 colonists; every colonist claims a distinct nearest job, no double-claims, all reach their sites; unreachable jobs stay unclaimed and are logged.
- Cancelling a designation releases its claim and re-idles the colonist within one arbitration cycle.
- Priorities honored: a colonist with mining=0 (disabled) never claims a mine job.

### B5 — Work execution: dig, chop, build, deconstruct
**Objective:** Jobs actually change the world. Mining removes voxels and yields stone; chopping fells trees → logs; building consumes materials and places voxels.
**Touches:** `bastion_jobs` execution systems, terrain edit path (reuse `remove_block`/block-add), item spawning, skill XP.
**Approach:**
- On "arrived + working," run a **work tick**: accumulate progress (rate scaled by skill), and on completion apply the terrain edit and **emit item drops** (stone, wood, ore per block type — Veloren already maps block/sprite → loot).
- **Build** is the inverse: a blueprint voxel is a ghost until a colonist hauls the required material (ties into B6) and completes construction, replacing ghost with real voxel.
- Grant skill XP on completion; feed back into rates.
- Guard terrain edits through the server's authoritative terrain events so chunk meshing/render updates fire (don't bypass into raw chonk writes).
**Done-when:**
- Designate a 3×3×3 dig; colonists mine it out; the hole appears in the top-down view with the Z-slice; stone items appear on the ground.
- Chop designation fells a tree and yields logs.
- A wall blueprint with materials present gets constructed into solid voxels; without materials, it stalls as a ghost and raises a "needs materials" job.

### B6 — Stockpiles, items, hauling
**Objective:** Resource logistics. Zones where items collect; colonists haul loose items into them; production/build jobs pull from them.
**Touches:** `common` item system (reuse), stockpile zone type, hauling jobs, inventory on colonists (reuse `Inventory`).
**Approach:**
- **Stockpile zone:** a designated ground rectangle with optional filters (what it accepts). Server tracks contents as an aggregate + physical item entities within.
- **Hauling job** auto-generation: any loose acceptable item not in a stockpile spawns a haul job (throttled/prioritized). Colonist picks up (into `Inventory`), walks, deposits.
- **Reservation:** build/production jobs *reserve* stockpile items so two jobs don't grab the same log.
- Expose stockpile totals to the HUD channel for B9.
**Done-when:**
- Mined stone auto-generates haul jobs; colonists carry stone into a stockpile; loose count → stockpile count conserves total (no item duplication/loss — assert in headless test).
- A build job reserves and consumes stockpiled material; reservation prevents double-spend under two concurrent builds (headless stress test with 2 builds, 1 log → exactly one completes).

### B7 — Needs, mood, and the idle/AI loop
**Objective:** Colonists are alive: they eat, sleep, idle sensibly, and degrade under neglect. Reuse rtsim NPC-AI patterns for the behavior tree.
**Touches:** needs decay system, need-satisfaction jobs (eat from stockpile, sleep in bed), mood/`Mood` system, idle behavior, `rtsim/src/rule/npc_ai.rs` patterns adapted.
**Approach:**
- **Decay:** hunger/rest/recreation tick down over game-time. Thresholds create high-priority *self* jobs (find food → eat; find bed → sleep) that preempt work.
- **Beds/food** as buildable/placeable objects (beds = build job from B5; food = item in stockpile from B6).
- **Mood** aggregates need states + events (comfortable room, saw a corpse, ate well) → modifiers; below a floor → **breakdown** state (wander/refuse work) as the tension mechanic.
- **Idle:** when no jobs, colonists don't freeze — wander to a rally point, socialize, or do lowest-priority passive work.
**Done-when:**
- Headless: run 2 in-game days; colonists eat and sleep on schedule; with food/beds present, mood stays stable; remove all food and a starvation debuff + mood drop appears within the expected window.
- A colonist mid-mining who drops below the rest threshold releases its job, sleeps, then resumes work — verified in log ordering.

### B8 — Threats, combat, and RTS control
**Objective:** External pressure + direct player command in combat. Reuse Veloren's real combat; spawn threats via rtsim.
**Touches:** rtsim threat spawning (bandits already modeled), enemy AI (existing agent code), RTS order set (`Attack`, `Move`, `Hold`, `Flee`), combat integration for colonists.
**Approach:**
- **Raids:** an rtsim rule periodically sends a bandit party toward the colony; on entering the loaded region they **promote** to ECS entities and use existing hostile agent AI.
- **RTS control:** extend B2 orders with `Attack(target)`, `Hold`, `Move-attack`. Selected colonists draw weapons (reuse combat states) and engage; unselected keep working or flee per a colony alarm toggle.
- **Alarm state:** a colony-wide "draft/undraft" so the player can yank everyone off work into defense (RimWorld draft).
**Done-when:**
- A scripted raid spawns off-map, pathes in, promotes to loaded entities, and attacks.
- Drafting + `Attack` order makes selected colonists fight and deal/take real damage; undraft returns them to the job board.
- A defeated raid's survivors retreat off-map and demote back to simulated (no leak of loaded entities — assert entity count returns to baseline).

### B9 — Colony HUD & control panels
**Objective:** The information + control layer that makes it playable: colonist list/inspector, RimWorld-style **work-priority grid**, designation toolbar, stockpile/resource readout, alerts, Z-slice indicator.
**Touches:** `voxygen/src/ui/` (match the in-tree UI toolkit — verify conrod vs. successor first), new `bastion` HUD widgets, state channel from server.
**Approach:**
- **Bottom toolbar:** designation tools (dig/chop/build/haul/zone), each entering a paint mode wired to B2.
- **Colonist bar + inspector:** portraits/health/mood; click → panel with needs, skills, current job, equipment.
- **Work tab:** grid of colonists × work-types with editable 0–4 priorities feeding B3/B4.
- **Alerts/log:** "colonist starving," "under attack," "construction stalled: no materials," + a scrolling event log (proto Legends chronicle).
- **Resource readout:** stockpile totals from B6.
**Done-when:**
- All designation tools are placeable from the toolbar and reflect in-world overlays.
- Editing a work priority in the grid changes claiming behavior live (set mining→0, colonists stop taking mine jobs).
- Inspector shows correct live needs/mood/job for the selected colonist; alerts fire on starvation and raid.

### B10 — Persistence (save/load the colony + world)
**Objective:** Save and reload a colony and its living world.
**Touches:** rtsim persistence (already exists), server DB/save for ECS colony state (colonists, needs, skills, priorities, designations, stockpiles, blueprints), save/load UI hook.
**Approach:**
- Piggyback rtsim's flat-table persistence for world-scale state.
- Serialize the colony ECS slice: colonist components, job board, designations, stockpile contents & reservations, in-progress blueprints. Use `serde` on the new `bastion` types from the start (design B3–B8 types serde-ready).
- Load path: restore world+rtsim, then rehydrate ECS colony, then re-derive transient job claims (safer to recompute claims than persist them).
**Done-when:**
- Save mid-game (colonists working, half-built wall, stocked stockpile), quit, reload → identical roster, needs, designations, stockpile counts, and the half-built wall resumes.
- No entity duplication or orphaned jobs after reload (headless assert on counts).

### B11 — Embark, worldgen tuning, starting scenario
**Objective:** A front-to-back new-game flow: generate/pick a world, choose an embark site, drop the starting colony with starting resources.
**Touches:** `world` site selection (reuse), a new embark screen (replaces character creation flow — flag off vanilla creation), scenario/starting-inventory config.
**Approach:**
- **Embark screen:** show the worldgen map (reuse Veloren's map render), let the player pick a region/site (reuse `world` site data: biome, resources, threats nearby). Legends/history preview optional stretch.
- **Scenario config** (RON, matching Veloren's data conventions): starting colonist count/skills, starting items, difficulty (raid cadence).
- **Drop-in:** generate the local chunks, spawn colony + starting stockpile, hand control to the RTS view.
**Done-when:**
- New game → map → pick site → land with colony + starting resources in the top-down view, no hero/character-creation step.
- Different sites yield materially different starts (biome/resources/threat proximity differ as previewed).

### B12 — Embody / Possession ("jump into any colonist and play normally")
**Objective:** The player can select any colonist and take direct first/third-person control using Veloren's stock avatar controls, then release back to autonomous AI and the top-down RTS view — seamlessly, with no window where both the AI and the player drive the same entity.
**Depends on:** B1 (we kept the vanilla camera mode), B2 (input modes + selection), B3 (colonists exist). Best experienced after B4–B5 (there's real work to drop into) and B7 (AI to suspend/resume). Can technically land right after Slice A.
**Touches:** voxygen camera-mode switch (reuse vanilla third/first-person from B1), input-mode switch (reuse vanilla `GameInput` bindings kept in `control.rs`), a new `Possessing`/`PlayerControlled` marker in `common/src/comp`, server-side control-handoff (who drives the entity's `comp::Controller`), suspend/resume of that colonist's job-AI (B4/B7), a HUD "Embody / Release" affordance (B9).
**Approach:**
- Possession is a **mode switch, not new gameplay** — this is the whole reason the doc insists on flagging vanilla off rather than deleting it. On **Embody(colonist E):**
  1. Ensure E is `SimulationMode::Loaded` (an active-colony colonist already is).
  2. Attach `Possessing` and, **server-authoritatively**, route player input into E's `comp::Controller` *instead of* the job-AI. The server is the single point that flips the driver, so there is never a frame where both drive E.
  3. **Suspend E's autonomous AI:** the job-arbitration system skips possessed colonists, and E's current claimed job is **released** (or explicitly paused) so no other colonist double-claims it and no orphan claim is left behind.
  4. voxygen switches to the vanilla third-person (default) / first-person camera bound to E and enables vanilla movement/combat/interaction bindings. It now plays exactly like stock singleplayer Veloren for that one entity.
- While embodied: **needs/mood keep decaying** (you can starve or exhaust yourself), **damage is real** (same entity), and the **rest of the colony keeps running under AI** while rtsim keeps ticking the wider world. Possession does not pause the game.
- On **Release** (hotkey, e.g. `Backspace`/`Tab`, or the HUD button again): server hands E's `Controller` back to the job-AI, camera snaps back to RTS at E's position, RTS input restored; E resumes claiming jobs next arbitration cycle.
- **Edge cases to handle explicitly:** death while possessed → auto-release to RTS + alert; possession supersedes draft (B8) for that colonist; switching straight from one possessed colonist to another = release-then-embody in a single action; releasing a colonist that has wandered mid-task.
**Done-when:**
- Select a colonist → Embody → camera drops to third-person on it and stock WASD/mouse-look/attack/interact drive it identically to vanilla singleplayer.
- The moment of possession releases/pauses that colonist's job with **no double-claim** by another colonist (headless + in-game assert).
- Needs continue to change and damage lands while embodied (eat something / take a hit and see it reflected in the inspector after release).
- Release returns to the top-down RTS view at that location and the colonist resumes autonomous work within one arbitration cycle.
- Embody→Release leaves entity count, controller ownership, and the job board consistent: exactly one driver at all times, zero orphaned claims, zero duplicate controllers (assert in a headless possess/release stress loop).

---

## 7. Cross-cutting testing strategy

The engine's headless-simulatable design is the biggest testing asset. Exploit it.

**Tier 1 — Headless deterministic sim tests (the workhorse).** Every simulation block (B3–B8, B10) ships tests that boot the B0 harness (world+rtsim+server, no voxygen), seed a fixed RNG + fixed world, run K ticks, and assert on dumped state. Determinism is a *requirement*: same seed → same result twice. This catches logic bugs in seconds without a GPU. Prioritize **conservation invariants** (items neither created nor destroyed except by explicit rules), **no double-claims**, **entity-count returns to baseline** after load/unload cycles.

**Tier 2 — Property / stress tests.** Randomized job floods, concurrent reservations, mass promote/demote across the loaded boundary. Assert invariants hold under load. Especially important for B4/B6/B8 where races live.

**Tier 3 — Visual smoke tests (voxygen).** For frontend blocks (B1/B2/B9): scripted launch into a fixed scenario, verify camera modes, overlays, and HUD render without panics; capture a screenshot for manual eyeball. Automate the "does it launch and not crash" gate even if the visual check is human.

**Tier 4 — Vertical-slice playtests.** After each vertical slice (§9), a human plays 15–30 min against a checklist: can I dig, build, stock, feed, defend, save/load? Log friction.

**Regression guard for upstream merges.** Because we keep vanilla behind flags, keep a CI job that builds and boots **vanilla** mode too, so a Bastion change that breaks upstream is caught early.

**Per-block gate.** No block is "done" until its Done-when tests are green in CI. Builders paste test output into the session as proof before moving on.

---

## 8. Risk & gotcha register

These are planning inputs for the builder, not reasons to stop.

1. **Compile-time drag.** Large workspace. Mitigation: per-crate `cargo check`, `mold`/`lld` linker, `sccache`, keep new code in small crates. Budget B0 to set this up; it pays for itself in every later block.
2. **rtsim is intentionally low-resolution.** Do **not** try to run per-colonist hunger/pathing in rtsim — that's the ECS/server's job. rtsim handles world-scale only. Keeping this boundary clean is the difference between a performant game and a swamp. (This is the #1 way this project goes wrong.)
3. **3D nav under a 2.5D control scheme.** Veloren's world is fully 3D; colony-sim pathing assumptions are more 2.5D. Constrain to walkable-surface navmesh + Z-slice; don't expose full 3D flight/climb to colonists in v1.
4. **The loaded↔simulated boundary (`SimulationMode`).** Promotion/demotion is where entities leak or duplicate. Every block that crosses it needs a baseline-entity-count assertion (see Tier-1 tests).
5. **Terrain edits must go through authoritative events**, not raw chonk writes, or meshing/render/persistence desync. B5 must respect this.
6. **UI toolkit drift.** Verify the current in-tree UI framework before B9; do not add a second one.
7. **Item conservation.** Hauling/reservation/production is the classic dupe-bug farm. Conservation invariants in Tier-1 tests are mandatory, not optional.
8. **Scope creep via Veloren's existing RPG systems.** Quests, trade, mounts, magic will tempt you. Flag them off; integrate deliberately later, never incidentally.
9. **GPL-3.** Any distribution obligations apply. Note it in the repo; not a code risk, a release-planning one.
10. **Determinism vs. `specs` iteration order / floating point.** Seed all RNG explicitly and avoid order-dependent float accumulation in sim logic, or Tier-1 determinism tests will flap.
11. **Dual-driver hazard (possession, B12).** The one way Embody goes wrong: the job-AI and the player both drive the same `Controller` and fight for it. Mitigation is structural — control handoff is **server-authoritative and single-point**, the possessed colonist's job is released on Embody, and arbitration skips possessed colonists. Every possess/release must assert *exactly one driver* and a clean job-board handoff. Handle death-while-possessed by auto-releasing to the RTS view rather than leaving a dead entity "controlled."

---

## 9. Suggested milestone ordering (vertical slices)

Block-linear order is safe but slow to "playable." These slices reach a playable artifact sooner by cutting across blocks:

- **Slice A — "I can see and select" (B0 → B1 → B2, B3-lite).** Top-down world, a visible colony you can box-select and order to move. No work yet. First dopamine hit; validates the whole frontend track.
- **Slice B — "I can dig and build" (B4 → B5 → B6-lite → minimal B9 toolbar).** Designate → colonists mine/chop/build/haul → stockpile grows. This is the core loop; if this feels good, the project is real.
- **Slice C — "It's a colony" (B7 → B6-full → B9-full).** Needs, mood, work-priority grid, alerts. Now it's RimWorld-shaped.
- **Slice D — "It's under threat and it persists" (B8 → B10).** Raids, RTS defense, save/load.
- **Slice E — "It's a new game each time" (B11 + rtsim world-life polish).** Embark, scenarios, living world & chronicle.

- **Embody (B12) is orthogonal.** It depends only on B1–B3, so it *can* land as early as the end of Slice A — but it's most satisfying once there's real work to drop into, so recommend building it alongside Slice C. It's also a cheap, high-impact demo: because it reuses vanilla control machinery, the effort is mode-switching and clean AI handoff, not new gameplay.

Recommend building **Slice A and B fully before touching C+**, because they de-risk both tracks (frontend and the sim boundary) and produce something demonstrably fun to iterate on.

---

## Appendix A — Key files & symbols to anchor on

- Camera: `voxygen/src/scene/camera.rs` (+ projection feeding the renderer).
- Input bindings: `voxygen/src/settings/control.rs` (`GameInput` enum, `default_binding`).
- Client input entry: `client/src/lib.rs` → `handle_input`, `remove_block`.
- ECS components: `common/src/comp/` (add `Colonist`, `Needs`, `Skills`, `WorkPriorities`, `Selected`, `PlayerColony`, `Possessing`/`PlayerControlled`).
- Control routing / possession: `comp::Controller` (the intent interface player input and job-AI both target — the single point B12 flips), plus vanilla camera/input kept behind the `bastion` flag.
- rtsim interface: `common/src/rtsim/` (`NpcId`, `SiteId`, `Actor`, `NpcAction`, `NpcActivity`, `RtSimController`, `SimulationMode`).
- rtsim rules/AI: `rtsim/src/rule/` (`npc_ai.rs`, `simulate_npcs.rs`), `impl Rule`, `OnTick`.
- Terrain/sprites: `common/src/terrain/` (`sprite.rs`), chonk types.
- UI: `voxygen/src/ui/` (verify toolkit).
- Server binary/harness: `server-cli/`, `server/`.
- Worldgen: `world/` (site/civ generation, map).

*(Verify exact paths against the pinned baseline SHA at the start of each block — the tree moves.)*

## Appendix B — Glossary (Bastion terms)

- **Colonist:** player-owned ECS humanoid with needs/skills/priorities.
- **Designation:** a player-painted intent on the world (dig/chop/build/haul/zone).
- **Job:** a claimable unit of work derived from a designation or a need.
- **Work-type:** category a colonist may be enabled/prioritized for (mining, construction, hauling, cooking, melee…).
- **Stockpile:** a zone that collects filtered items; the logistics hub.
- **Loaded vs. Simulated:** ECS full-fidelity entity vs. rtsim abstract record; governed by `SimulationMode`.
- **Slice (Z-slice):** the render depth cut for viewing/digging layers.
- **Draft/Alarm:** colony-wide toggle pulling colonists from work into combat control.
- **Embody / Possession:** taking direct first/third-person control of a single colonist using vanilla Veloren controls, suspending its AI, and releasing back to the RTS view (B12).

---

*End of directive. Build B0 first; do not skip the harness.*
