# Project Bastion — Turning Veloren into an Autonomous God-Game Colony Sim

**A build & test directive for Claude Code**
Version 1.2 · Architect-authored design doc · Target: fork of `veloren/veloren` (Rust, GPL-3)
Lineage: Dwarf Fortress / RimWorld (autonomy & policy) + Black & White / From Dust / Populous (indirect divinity). **Explicitly *not* StarCraft-style unit micro.**
*v1.2 changelog: reworked §7 to invariant-first testing after B0 exploration confirmed rtsim per-tick RNG is OS-seeded (added Deterministic Mode as work item WI-DET); added §2a "what already exists (verified)" — Veloren ships most god-power/spawn/zone/possession primitives as server ops, shifting B3/B6/B8/B12/B13 toward wiring rather than building.*

---

## 0. How to use this document

This is written for the architect→builder workflow: this document is the **architect artifact**. Each build block (B0–B13) in §6 is scoped so it can be lifted more-or-less whole into an **isolated Claude Code builder session** against the fork. Blocks declare their own objective, the files they touch, the approach, and — critically — a **Done-when** contract with concrete tests. Builders should not proceed past a block's Done-when gate.

**Read Pillar §1a first.** This is an *autonomous god game*: the world plays itself and the player *influences* rather than commands. That pillar overrides any older "command/order/RTS" phrasing elsewhere.

Ordering is deliberate. B0–B2 establish the overseer shell (build + camera + inspect/designate input) so you can *see* the world top-down before touching simulation. B3–B8 build the **autonomous** colony simulation. B9–B13 add UI, persistence, scenario, possession (B12), and the divine-influence layer (B13) that turns a self-running sim into a god game. §9 defines the vertical slices if you want a playable thing sooner than block-linear order allows.

**Naming:** the working title is *Project Bastion*. New crates/modules use the `bastion_` / `bastion::` prefix so a `grep` cleanly separates our code from upstream Veloren, which matters for merging upstream changes later.

---

## 1. Executive summary — the core thesis

Veloren is **not one program**. It is a headless simulation stack (`common` + `world` + `rtsim` + `server`) with a *swappable* frontend (`voxygen`). Veloren's own architecture documentation explicitly says a frontend could render the world "using isometric graphics, Dwarf Fortress style layered 2D graphics, or any other representation one could imagine," and anticipates a Legends-Viewer-style history browser. We are not fighting the engine's grain; we are building the thing its authors left the door open for.

That gives us a **two-track transformation**, and the tracks are largely independent:

1. **Frontend track** — reconfigure `voxygen`'s third-person, avatar-centric camera and input into a **top-down orthographic overseer camera** with **inspection + designation + indirect-influence** input. No simulation changes required to *look* the part.

2. **Simulation track** — invert the control model. Vanilla Veloren = *one* player avatar + ambient NPCs. We want *no* player avatar and *many* **fully autonomous** colonists that the player **influences but does not command**, plus a **designation → job → task** loop (mine/chop/build/haul/cook/fight) with **needs, stockpiles, and mood**. This is a colony-sim layer bolted onto two substrates Veloren already ships:
   - the **ECS** (`specs`, struct-of-arrays) on the `server` for *loaded*, high-resolution, per-colonist simulation, and
   - **`rtsim`** (real-time sim) for *low-resolution* world-scale simulation of everything off-screen — factions, migration, economy, resource depletion — which already exists and is exactly the Dwarf-Fortress "the world lives without you" layer.

The single most important design decision in this whole document: **colonists you are actively managing live in the ECS; the wider world lives in rtsim; and the boundary between them is `SimulationMode` (loaded ↔ simulated), which Veloren already implements.** Do not reinvent world simulation. Wrap it.

---

## 1a. DESIGN PILLAR — the world plays itself (autonomy first, divinity not command)

**This is the defining constraint and it overrides anything elsewhere in the doc that contradicts it.**

Project Bastion is an **autonomous god game**, not a real-time strategy game. The distinction is not cosmetic — it dictates the entire control model:

- **The world is self-running.** Colonists, wildlife, factions, ecosystems, weather, and economy all pursue their own goals and continue to evolve **with zero player input**. If the player walks away, the colony still hunts, farms, builds, quarrels, defends itself, thrives, or dies. The game must be *watchable* the way Dwarf Fortress and a fish tank are watchable.
- **The player is a god / overseer, not a general.** The player does **not** select-and-command units StarCraft-style as the primary loop. The player acts *indirectly*: setting **policy** (work priorities, zones, schedules, laws), placing **designations** (intentions the colony fulfils on its own terms), and wielding **divine influence** (shaping terrain, seeding or withholding resources, blessings/curses, weather, calamities — the Black & White / From Dust / Populous vocabulary). See **B13**.
- **Autonomy is the substrate; influence rides on top.** Veloren already gives us autonomy for free: rtsim runs the world headless, and ECS colonists run their own AI. Our job is *not* to build a command layer — it's to build good autonomous agents and then a rich set of **indirect levers** over them.
- **Direct control is the exception, deliberately friction-ful.** Two escape hatches exist and are *bounded*: (a) a **draft/nudge** for emergencies (B8) that is costly/limited so it can't become the main loop, and (b) **Embody/possession** (B12), where the god descends into a single mortal and plays it directly. Neither is the default verb. The default verb is *influence*.
- **Success test:** a **zero-input soak** (§7, Tier 1b) where the sim runs for many in-game days untouched and remains stable *and* eventful — no stagnation, no death-spiral, no crash. If the world can't play itself well, nothing else matters.

Read every block below through this lens: wherever an older passage says "command," "order," or "RTS control," interpret it as *indirect influence* unless it is explicitly the bounded draft (B8) or Embody (B12).

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
- **UI toolkit is a hybrid (verified).** voxygen uses **three** UI stacks: `conrod` for the in-game HUD
  (`voxygen/src/hud/`, e.g. `Text::new(..).set(id, ui_widgets)`, `conrod_id`); a Veloren-forked **`iced`**
  (`voxygen/src/ui/ice/`, `IcedUi`/`IcedRenderer`) for menus; and **`egui`** behind the `egui-ui` feature
  (`voxygen/egui/`, `veloren_voxygen_egui`) for debug windows. **Implication for B9:** prototype the
  colony HUD fast in **egui** (trivial to add windows/panels), then port the keepers to **conrod** for the
  polished in-game look. Don't add a *fourth* framework.

## 2a. What already exists in vanilla (VERIFIED — this collapses much of the build into "wire, don't build")

Exploration of `common/src/cmd.rs` (96 `ServerChatCommand` variants), `veloren_server/events/player.rs`,
and `veloren_server/rtsim/tick.rs` confirms Veloren already ships server-side operations for most of what
Bastion needs. **These are real, tested code paths behind admin commands — reuse the underlying
event/operation, not the chat command string.** This is the single biggest de-risking finding in the doc.

- **Terrain & object editing (→ B5 mining/building, B13 terrain powers):** `MakeBlock`, `MakeSprite`,
  `MakeVolume`, `Object`, `RemoveLights`, plus client-side `remove_block`. Authoritative terrain-edit
  paths already exist and correctly trigger meshing/persistence.
- **Calamity / combat powers (→ B8, B13):** **`Explosion`** and **`Lightning`** are already implemented
  server operations. Smite-the-raiders is a wire-up, not a new system.
- **Weather (→ B13 weather powers):** a real **`WeatherGrid`** resource is fed into the rtsim tick, and
  **`WeatherZone`** already sets weather over an area. Rain-on-drought hooks straight into `WeatherGrid`.
- **Time control (→ B11, pacing):** `Time` (set time of day) and **`TimeScale`** (speed up / slow down
  the whole sim) already exist — useful for both gameplay and the Tier-1b soak (fast-forward days).
- **Zones / areas (→ B4 designations, B6 stockpiles):** **`AreaAdd` / `AreaList` / `AreaRemove`** define
  typed named areas; `Build` / `PermitBuild` / `RevokeBuild` gate build permission by area; `Safezone`
  exists. A real foundation for designation regions and stockpile zones.
- **Colonist spawning & configuration (→ B3):** `Spawn`, `MakeNpc`, `IntoNpc`, `Body`, `Scale`, `Buff`,
  `Health`, `GiveItem`, `Kit`, `SkillPoint`, `SkillPreset`, plus `CreateNpcEvent`/`NpcBuilder` in the
  server tick. Spawning a configured starting band is assembling existing pieces.
- **rtsim inspection & control (→ B0 harness, B9 HUD, B7 AI):** `RtsimInfo`, `RtsimNpc`, `RtsimChunk`,
  `RtsimTp`, `RtsimPurge` expose rtsim NPCs directly — handy for harness dumps and HUD readouts.
- **Possession already exists (→ B12, big win):** `PresenceKind::Possessor` and a server-side possession
  handler in `events/player.rs` that swaps the player's presence onto another entity **and already
  includes an item-duplication guard**. B12 is largely *reuse this proven pathway* rather than build one.
- **Entity command routing:** `Sudo` makes an entity execute a command; `Goto`/`Tp` move entities —
  useful primitives when wiring autonomous behaviour and debugging.

**What is genuinely still a build (not pre-existing):** the *autonomous colony layer itself* — the
`Colonist`/needs/skills model, the designation→job→**self-arbitration** loop, stockpile/hauling logistics,
the policy layer, the overseer camera/input, the favor economy, and From-Dust-style **fluid/material
flow** (Veloren has entity-in-fluid physics via `comp::fluid_dynamics`, and water exists as terrain, but
**not** a From Dust terrain-fluid solver — that's the one B13 power that's a real build).

**Net effect on scope:** B3, B6, B8, and especially **B13 shift from "build" toward "wire existing server
ops to overseer influence input + the favor economy."** The hard, novel work concentrates in **B4**
(autonomous job arbitration) and **B7** (autonomous AI/needs) — which is where it should be, since those
are what make the world *play itself*.

---

## 3. Target design — what "autonomous god-game colony sim" means, concretely

Pin the target so builders don't drift. Read this section against Pillar §1a: **the world is autonomous; the player influences, it does not command.** The game we are building has these properties:

**Camera & view.** Fixed top-down / high-oblique orthographic **overseer** view over a bounded embark region. Pan, zoom, 90°-step rotate. A **Z-layer / depth slice** control (DF's most important interaction) so the player can peer down and view lower layers — Veloren is fully 3D voxel, so "cut away everything above layer Z" is a render filter, not new geometry.

**Unit of play.** The player oversees a **colony** of ~5–8 autonomous colonists (Veloren humanoid `Body` reused) that grows over time. No single hero avatar; the camera is detached from any entity. Colonists are **agents with their own goals**, not selectable units awaiting orders — the player shapes *what the colony wants and can do*, then watches it act.

**Player agency = influence, in three tiers (weakest/most-diegetic first):**
1. **Policy** — standing rules the colony self-organizes around: work-priority grid, zones/stockpiles, schedules, restrictions, thresholds ("keep 20 food in reserve"). Set once, obeyed continuously. This is the primary loop.
2. **Designation** — one-off *intentions* painted on the world (mine here, build this, chop that). The colony fulfils them autonomously, in its own order, subject to its priorities — you state the *what*, never the *who/when/how*.
3. **Divine influence (B13)** — direct god-powers over the *world* rather than the *people*: shape terrain (From Dust–style material flow — you have a reference for this already), seed/withhold resources, summon rain or drought, bless/curse a colonist or a site, trigger or avert calamities. You act on the environment and the colony *responds*.

**Bounded direct control (deliberately not the main verb):**
- **Draft / nudge (B8)** — an emergency lever (defend! flee to here!) that is *costly or cooldown-limited* so it can't degrade into StarCraft micro.
- **Embody / possession (B12)** — the god descends into a single mortal and plays it normally in stock Veloren third/first-person (WASD, mouse-look, real combat), then releases back to autonomy. First-class feature, near-free to build because vanilla Veloren already *is* "control one humanoid."

**Core loop (autonomy-first):**
1. The colony **runs itself** — colonists pursue needs, claim jobs, defend, socialize, reproduce, age, and die on their own; the wider world (factions, wildlife, economy, weather) evolves via rtsim.
2. The player **watches**, reads the story, and identifies pressure points.
3. The player **adjusts policy, paints designations, or spends divine influence** to steer outcomes — never to puppet individuals.
4. The world **absorbs the influence and plays on**; consequences ripple (a blessed harvest, a diverted river, a repelled raid, a schism). History accrues → a Legends-style chronicle.
5. Rarely, in a crisis or on a whim, the player **drafts** or **embodies** — then hands control back.

**Explicit non-goals for v1** (cut scope ruthlessly): multiplayer; the solo-hero RPG campaign/quest progression; mounted/airship content; **and StarCraft-style select-and-command as a core loop** (only the bounded draft/Embody exist). Feature-flag them off; don't delete (keeps upstream merges sane). Direct first/third-person play *is* in scope, but only as Embody (B12).

---

## 4. Transformation strategy — two tracks, one boundary

```
     OVERSEER FRONTEND                    AUTONOMOUS SIMULATION (runs w/o input)
 ┌──────────────────────────┐        ┌──────────────────────────────────┐
 │ voxygen                  │influence│ server (ECS, loaded colonists)    │
 │  • ortho overseer cam(B1)│ policy │  • Colonist AI + needs (B3,B7)    │
 │  • Z-slice render (B1)   │ desig. │  • Designation/Job systems (B4)   │
 │  • inspect/select (B2)   │ ─────► │  • Work execution: dig/build (B5) │
 │  • designation tools (B2)│        │  • Stockpiles/hauling/items (B6)  │
 │  • god-power tools (B13) │ ◄───── │  • Autonomous defense (B8)        │
 │  • colony HUD (B9)       │ state  │  • God-power effects (B13)        │
 │  • Embody handoff (B12)  │        └───────────────┬──────────────────┘
 └──────────────────────────┘                        │ SimulationMode
   (player NEVER micromanages;                        │ (loaded ↔ simulated)
    only influences + rarely        ┌─────────────────▼──────────────────┐
    drafts/embodies)                │ rtsim (world-scale, always ticking)  │
                                    │  • factions, migration, diplomacy    │
                                    │  • economy, resource depletion       │
                                    │  • threats spawn, weather, history   │
                                    └──────────────────────────────────────┘
```

Note the arrows: player→sim carries **influence** (policy, designation, god-powers), never per-unit commands. The sim runs whether or not any influence arrives.

**The boundary rule.** Anything inside the active embark region and currently relevant = ECS entity on the server (full fidelity, autonomous AI). Anything outside, or dormant = rtsim record (cheap, abstract sim). Colonists that wander off the map edge, caravans that leave, enemies that retreat → demote to `SimulationMode::Simulated`. This is how you get a DF-scale living world that *keeps living on its own* without simulating 10,000 entities at 60 Hz.

---

## 5. Gap analysis — reuse / bend / build / discard

**Reuse as-is:** voxel terrain + chonks; terrain block add/remove; humanoid bodies & animation; item/inventory system; pathfinding primitives (verify current impl — `common` has movement/nav); combat & health/stats; rtsim persistence; worldgen & site generation; networking (singleplayer Mpsc).

**Bend (reconfigure existing):** camera (3rd-person → ortho overseer); input (`GameInput` avatar controls → inspect + designate + influence); NPC AI patterns in `rtsim/src/rule/npc_ai.rs` → **autonomous** colonist task AI; `RtSimController`/`comp::Controller` (already the "abstract intent → action" interface — perfect for self-directed job execution); rtsim rules → world-scale god-power effects + autonomous world life.

**Build new:** `Colonist` component (needs/skills/priorities/goals); designation model + placement; job board / claim / **autonomous** arbitration; stockpile zones & hauling; work-execution systems that emit terrain edits & item moves; **policy layer** (work grid, zones, thresholds); **divine-influence / god-power layer** (terrain shaping, resource seeding, weather, blessings, calamities — B13); bounded draft + Embody handoff; colony HUD; Z-slice render filter; embark/scenario setup.

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

### B1 — Top-down orthographic overseer camera + Z-slice
**Objective:** See the world top-down with pan/zoom/rotate and a depth-slice control. Vanilla camera still selectable behind a feature flag.
**Touches:** `voxygen/src/scene/camera.rs` (+ wherever the projection/view matrices feed the renderer), a new `bastion` camera mode, terrain draw path for the slice filter.
**Approach:**
- Add `CameraMode::Overseer` alongside existing modes. Orthographic projection; fixed pitch (start ~55–70° oblique — pure 90° top-down reads poorly with voxel walls); target = a ground point, not an entity.
- Controls: WASD / edge-scroll pan; scroll = zoom (clamp); Q/E = 90° rotate; PgUp/PgDn (or `[`/`]`) = **Z-slice** cursor.
- **Z-slice render:** discard/gray voxels above the current slice Z so digging down is visible. Cheapest correct approach: a uniform `max_z` passed to the terrain shader that clips fragments above it; refine later to a fade band.
**Done-when:**
- Launch flag puts you in ortho top-down over a generated world; pan/zoom/rotate feel responsive (>50 fps on a mid GPU).
- Z-slice cursor hides everything above the chosen layer and reveals interior/underground voxels.
- Vanilla 3rd-person mode still works when flag off (no regressions).

### B2 — Inspect, designate & influence input (the overseer control surface)
**Objective:** The *non-command* control surface. Click to **inspect** an entity/tile, drag-box to **inspect a group**, and a **designation/paint** cursor mode to express intentions on the world. This is plumbing, not yet behavior. Per Pillar §1a there is **no primary move/attack command** — selection exists for *inspection and designation targeting*, not to puppet units. (A bounded emergency nudge is added later in B8, deliberately separate.)
**Touches:** `voxygen/src/settings/control.rs` (`GameInput` additions), voxygen input handling, `client/src/lib.rs` (`handle_input` → new influence messages), a new `bastion` message type through `network`/`server`, a `Selectable`/`Selected` component in `common/src/comp`.
**Approach:**
- Add `GameInput` variants: `Inspect`, `InspectAdd`, `BoxInspectDrag`, `DesignateApply`, `DesignateCancel`, a designation-tool cycle, and a reserved `InfluenceApply` (for B13 god-powers). **Do not** add a general `CommandMove` as a core verb.
- Screen→world ray/pick: reuse voxygen's existing block/entity targeting (the `build_target`/`nearest_block` machinery referenced in input handling) to resolve clicks to a voxel or entity.
- Drag-box: track down/up screen coords, project frustum slice, mark ECS entities with `Selected` **for inspection/HUD only**.
- New client→server messages: `PlaceDesignation { region, kind }` and `ApplyInfluence { region_or_target, kind }` (kind stubbed until B13). Server validates & stores; behavior arrives in later blocks. No per-unit order message in v1.
**Done-when:**
- Left-click inspects a highlighted entity (HUD shows its detail); drag-box inspects several; Shift adds to the inspection set.
- A designation tool paints a marked region; the server receives it with correct world coords and echoes it back for render (colored overlay).
- The reserved `ApplyInfluence` message round-trips with correct coords (payload stubbed) so B13 can hang real god-powers off it.

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

### B8 — Threats & **autonomous** defense (+ a bounded emergency draft)
**Objective:** External pressure that the colony **handles on its own**, plus a single, deliberately-limited emergency lever for the player. Per Pillar §1a this is **not** an RTS combat-control block: the default is that colonists assess and respond to danger themselves. Reuse Veloren's real combat; spawn threats via rtsim.
**Touches:** rtsim threat spawning (bandits already modeled), enemy AI (existing agent code), a colonist **defense-AI policy** (fight/flee/muster thresholds), one bounded `Draft` toggle, combat integration for colonists.
**Approach:**
- **Reuse-verified:** hostile spawning, real combat, and calamity primitives (`Explosion`, `Lightning`) already exist; bandits are already rtsim-modeled. This block wires them to autonomous defense policy, not new combat code.
- **Raids:** an rtsim rule periodically sends a bandit party toward the colony; on entering the loaded region they **promote** to ECS entities and use existing hostile agent AI.
- **Autonomous defense (the default):** colonists have a defense policy — combat-capable ones muster to a **rally zone** and engage; vulnerable ones flee to safety/indoors; the militia disperses back to work when the threat clears. The player shapes this by **policy** (who is drafted-capable, rally-zone placement, courage/flee thresholds), *not* by ordering individuals.
- **Bounded emergency draft (the exception):** a single colony-wide `Draft` toggle (RimWorld-style) that mans defenses now and, optionally, a *rally-point move* for the militia. It is intentionally coarse (whole-militia, one rally point) and can carry a cost/cooldown so it can't become StarCraft micro. There is **no** per-unit attack-move order.
- **Divine intervention** over combat is the *preferred* heavy lever and lives in B13 (smite raiders, raise a wall, panic the enemy) — reinforcing "influence, not command."
**Done-when:**
- A scripted raid spawns off-map, pathes in, promotes to loaded entities, and attacks — and with **zero player input** the colony musters, fights or flees per policy, and (if it wins) returns to work. This autonomous path is the primary Done-when.
- The `Draft` toggle mans defenses and can send the militia to one rally point; untoggling returns everyone to the job board. No per-unit command exists.
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
- **Drop-in:** generate the local chunks, spawn colony + starting stockpile, hand control to the overseer view.
**Done-when:**
- New game → map → pick site → land with colony + starting resources in the top-down view, no hero/character-creation step.
- Different sites yield materially different starts (biome/resources/threat proximity differ as previewed).

### B12 — Embody / Possession ("jump into any colonist and play normally")
**Objective:** The player can select any colonist and take direct first/third-person control using Veloren's stock avatar controls, then release back to autonomous AI and the top-down overseer view — seamlessly, with no window where both the AI and the player drive the same entity.
**Depends on:** B1 (we kept the vanilla camera mode), B2 (input modes + selection), B3 (colonists exist). Best experienced after B4–B5 (there's real work to drop into) and B7 (AI to suspend/resume). Can technically land right after Slice A.
**Touches:** voxygen camera-mode switch (reuse vanilla third/first-person from B1), input-mode switch (reuse vanilla `GameInput` bindings kept in `control.rs`), a new `Possessing`/`PlayerControlled` marker in `common/src/comp`, server-side control-handoff (who drives the entity's `comp::Controller`), suspend/resume of that colonist's job-AI (B4/B7), a HUD "Embody / Release" affordance (B9).
**Approach:**
- **Reuse-verified:** Veloren already has possession — `PresenceKind::Possessor` plus a server handler in
  `veloren_server/events/player.rs` that swaps the player's presence onto another entity and **already
  guards against item duplication**. Start from that pathway; B12 is mostly *adapting the existing
  possess flow to colonists + clean job-AI handoff*, not building possession from zero. Confirm the current
  command/trigger surface in-tree (the possess entry point may be a command or event).
- Possession is a **mode switch, not new gameplay** — this is the whole reason the doc insists on flagging vanilla off rather than deleting it. On **Embody(colonist E):**
  1. Ensure E is `SimulationMode::Loaded` (an active-colony colonist already is).
  2. Attach `Possessing` and, **server-authoritatively**, route player input into E's `comp::Controller` *instead of* the job-AI. The server is the single point that flips the driver, so there is never a frame where both drive E.
  3. **Suspend E's autonomous AI:** the job-arbitration system skips possessed colonists, and E's current claimed job is **released** (or explicitly paused) so no other colonist double-claims it and no orphan claim is left behind.
  4. voxygen switches to the vanilla third-person (default) / first-person camera bound to E and enables vanilla movement/combat/interaction bindings. It now plays exactly like stock singleplayer Veloren for that one entity.
- While embodied: **needs/mood keep decaying** (you can starve or exhaust yourself), **damage is real** (same entity), and the **rest of the colony keeps running under AI** while rtsim keeps ticking the wider world. Possession does not pause the game.
- On **Release** (hotkey, e.g. `Backspace`/`Tab`, or the HUD button again): server hands E's `Controller` back to the job-AI, camera snaps back to the overseer view at E's position, overseer input restored; E resumes claiming jobs next arbitration cycle.
- **Edge cases to handle explicitly:** death while possessed → auto-release to the overseer view + alert; possession supersedes draft (B8) for that colonist; switching straight from one possessed colonist to another = release-then-embody in a single action; releasing a colonist that has wandered mid-task.
**Done-when:**
- Select a colonist → Embody → camera drops to third-person on it and stock WASD/mouse-look/attack/interact drive it identically to vanilla singleplayer.
- The moment of possession releases/pauses that colonist's job with **no double-claim** by another colonist (headless + in-game assert).
- Needs continue to change and damage lands while embodied (eat something / take a hit and see it reflected in the inspector after release).
- Release returns to the top-down overseer view at that location and the colonist resumes autonomous work within one arbitration cycle.
- Embody→Release leaves entity count, controller ownership, and the job board consistent: exactly one driver at all times, zero orphaned claims, zero duplicate controllers (assert in a headless possess/release stress loop).

### B13 — Divine influence / god-power layer (the thing that makes it a *god* game)
**Objective:** Give the player a palette of **indirect** powers that act on the *world and conditions*, which the autonomous colony then reacts to — the Black & White / From Dust / Populous vocabulary. This is the payoff of Pillar §1a: the primary way the player *changes* outcomes (beyond policy/designation) without ever puppeting a unit.
**Depends on:** B1/B2 (overseer cam + the reserved `ApplyInfluence` message), B3–B7 (a colony to affect), rtsim rules (for world-scale effects). Terrain powers share the edit path from B5.
**Touches:** a `bastion_influence` module (server), god-power definitions + costs in `common`, rtsim rules for world-scale/persistent effects, voxygen god-power toolbar (B9), the `ApplyInfluence` channel from B2, an **influence-economy** resource (mana/faith/favor).
**Approach — the power palette (v1 set; build the framework + 3–4 powers first, then expand):**
- **Reuse-verified (this block is mostly wiring):** most god-powers already exist as server operations
  behind admin commands — call the underlying event/operation, not the chat string. `MakeBlock`,
  `MakeSprite`, `MakeVolume`, `Object` (terrain/objects); **`Explosion`**, **`Lightning`** (calamity);
  **`WeatherZone`** + the `WeatherGrid` resource (weather); `Time`/`TimeScale` (time); `Buff`
  (blessings/curses); `Spawn`/`GiveItem` (resource seeding). B13's real new work is the **favor economy**,
  the **overseer targeting UI**, and routing these through `ApplyInfluence` — plus the one genuinely novel
  power below (fluid flow).
- **Terrain shaping (From Dust–style).** Raise/lower land, carve a channel, drop a rock plug, trigger material flow (lava/water/sand). Raise/lower/carve reuse the authoritative terrain-edit path from B5 (`MakeBlock`/`MakeVolume`). **The From Dust *material-flow solver itself is the one real build here*** — Veloren has entity-in-fluid physics (`comp::fluid_dynamics`) and water-as-terrain, but not a terrain fluid-flow simulation. *You already have a From Dust material-flow reference; prove it out standalone first, then port.*
- **Resource seeding / withholding.** Bless a tile to grow food, surface an ore vein, spawn game animals (reuse `Spawn`/`MakeSprite`/`GiveItem`); or blight a field. Emits items/sprites the colony then autonomously harvests.
- **Weather & season nudges.** Call rain onto a drought, clear a storm, bring an early frost — write to the existing **`WeatherGrid`** resource (as `WeatherZone` does).
- **Blessings / curses on a colonist or site.** Temporary buffs (vigor, courage, inspiration → faster work / better mood) or afflictions — reuse the existing **`Buff`** system. Persistent ones tracked in rtsim.
- **Calamity / intervention.** Smite raiders with **`Lightning`**/**`Explosion`** (already implemented), panic an enemy party (flip their AI to flee), raise a defensive wall (`MakeVolume`). The *preferred* combat lever over the B8 draft — because it's influence on the world, not command of units.
- **Influence economy.** Powers cost **favor/faith** (name TBD) that accrues from the colony thriving/worshipping and regenerates over time — so the player is a god with *limits*, choosing when to intervene, not an omnipotent micromanager. This constraint is what keeps the game autonomous rather than a puppet show. **This is genuinely new** and is the design core of the block.
**Design guardrails:** every power acts on **environment or conditions**, never as a direct "unit, do X" order (that would violate Pillar §1a — the only direct control is Embody). Powers should produce *situations the autonomous colony responds to*, and their consequences should ripple through rtsim (a diverted river changes farms downstream; a blessed harvest shifts the economy).
**Done-when:**
- The framework applies a power at a targeted region/entity via `ApplyInfluence`, spends favor, and shows cooldown/cost in the HUD.
- **Terrain shape** raises/lowers/carves voxels through the authoritative edit path (mesh + persistence update correctly); a carved channel actually routes water/material.
- **Resource seed** makes food/ore appear and the colony *autonomously* harvests it with no further input (ties the god-power back into autonomy).
- **Blessing** measurably changes an autonomous colonist's behavior (e.g. courage buff makes it hold instead of flee in B8); **calamity** (smite/panic) alters a raid's outcome.
- Favor depletes and regenerates correctly; with zero favor, powers are unavailable but the colony still runs fully autonomously (the game never *requires* god input — §7 soak test still passes).

---

## 7. Cross-cutting testing strategy

The engine's headless-simulatable design is the biggest testing asset. Exploit it.

> **Determinism reality (verified in B0 exploration).** Veloren is deterministic at **worldgen**
> (`rtsim::Data::generate` seeds a `SmallRng` from the world seed), but its **per-tick rtsim rules**
> (`npc_ai`, `migrate`, `cleanup`, …) seed their RNGs from **OS entropy, not the world seed**. So the
> same seed produces an identical *starting* world and then **diverges the moment it ticks**. `specs`
> parallel iteration + floating point can add further nondeterminism. Therefore the test strategy is
> **invariant-first, not bit-exact-replay-first**. Exact replay is an opt-in capability (Deterministic
> Mode, WI-DET below), not the default gate — and that's fine, because a *living* world should have
> run-to-run variety anyway.

**Tier 1 — Headless invariant sim tests (the workhorse).** Every simulation block (B3–B8, B13, B10)
ships tests that boot the B0 harness (world+rtsim+server, no voxygen), seed a fixed world, run K ticks,
and assert on **invariants that must hold regardless of run-to-run variety** — this is the robust gate
given the RNG reality above. Prioritize: **conservation** (items neither created nor destroyed except by
explicit rules), **no double-claims** on jobs, **entity-count returns to baseline** after load/unload and
promote/demote cycles, **bounded** tick time / memory / entity counts, and **no panics**. These catch
logic bugs in seconds without a GPU and don't depend on determinism.

**Tier 1b — The zero-input autonomy soak (THE headline test).** This is the test that proves the design
pillar. Boot the headless harness with a full colony + world, then **apply no player input at all** and
run for many in-game days (target: 30+ days, then an overnight multi-season run). Assert the world is
both **stable and eventful** — all as invariants, none requiring determinism:
- *No crash, no runaway* — entity counts, memory, and tick time stay bounded; no promote/demote leak; no item dupe.
- *No death-spiral* — a reasonably-provisioned colony survives on its own (feeds, rests, defends, repairs) without the player.
- *No stagnation* — meaningful events keep happening (births/deaths, raids fought, buildings finished, resources gathered/depleted, factions acting); the event log is non-empty and varied.
- *(Optional, only under Deterministic Mode)* — same seed reproduces the same 30-day history twice. Use this for regression diffing, not as the standing gate.
If Tier 1b fails, the game is not a god game — it's a puppet that dies when you look away. Treat a Tier 1b regression as a release blocker. Every sim block (B3–B8, B13's autonomy paths) must keep it green.

**WI-DET — Deterministic Mode (a scheduled work item, not a block).** A fork-local capability for when you
*do* need exact replay (regression diffing, bug repro, CI snapshot tests). Scope: thread a single
seeded, **tick-indexed** RNG (derived from the world seed) through the rtsim rules instead of OS entropy,
and single-thread the tick loop in this mode. Gate it behind a `bastion-deterministic` feature/flag so
**normal play keeps its run-to-run freshness**. Land WI-DET **before Slice C** (before heavy sim-behavior
testing piles up), because retrofitting determinism after B4–B8 add more entropy sources is far more
painful. It's optional to *ship*, but the invariant tests (Tier 1/1b) are mandatory and do not need it.

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
10. **Determinism is NOT free (verified).** Confirmed in B0 exploration: worldgen is seeded from the
    world seed, but rtsim's per-tick rules (`npc_ai`, `migrate`, `cleanup`) seed RNG from **OS entropy**,
    so runs diverge after tick 0; `specs` parallel iteration + floats add more. **Do not build the test
    strategy on bit-exact replay.** Use **invariant-first** tests (Tier 1/1b) as the standing gate, and
    treat exact replay as an opt-in **Deterministic Mode (WI-DET, §7)** to be landed before Slice C for
    regression diffing only. A `DETERMINISM: DIVERGED` result from the B0 harness is expected and correct,
    not a bug.
11. **Dual-driver hazard (possession, B12).** The one way Embody goes wrong: the job-AI and the player both drive the same `Controller` and fight for it. Mitigation is structural — control handoff is **server-authoritative and single-point**, the possessed colonist's job is released on Embody, and arbitration skips possessed colonists. Every possess/release must assert *exactly one driver* and a clean job-board handoff. Handle death-while-possessed by auto-releasing to the overseer view rather than leaving a dead entity "controlled."
12. **Autonomy balance — stagnation vs. death-spiral (Pillar §1a).** An autonomous world can fail two ways: it flatlines (nothing happens, boring) or it collapses the instant it's left alone (unwinnable, feels broken). Tune colonist AI and rtsim so a provisioned colony is self-sustaining but still pressured. Tier-1b soak is the guardrail; check it after every AI/needs/threat change. Expose the key rates (need decay, raid cadence, resource regen) as tunable config so balance is data, not code.
13. **Micro-creep — the RTS temptation.** The gravitational pull of "just add a move order / an attack-move / a build-here-now button" is strong and will quietly turn this back into StarCraft. Guard the pillar: the only direct control is the bounded B8 draft and B12 Embody. If a feature request is "let the player tell a specific unit to do a specific thing right now," it must instead become **policy, designation, or god-power**. Put this rule in the repo README so future-you doesn't erode it.
14. **God-power griefing the sim.** Divine powers (B13) edit terrain and spawn resources; misused they can dupe items, strand colonists (carve the ground from under them), or break pathing/persistence. Route every power through the same authoritative edit/spawn paths as B5/B6, respect conservation invariants, and fuzz-test powers in Tier-2.

---

## 9. Suggested milestone ordering (vertical slices)

Block-linear order is safe but slow to "playable." These slices reach a playable artifact sooner by cutting across blocks:

- **Slice A — "I can see it" (B0 → B1 → B2, B3-lite).** Top-down overseer world, a visible colony you can inspect (click/box) and paint designations on. No commands, no work yet. First dopamine hit; validates the frontend track.
- **Slice B — "It builds itself when I point" (B4 → B5 → B6-lite → minimal B9 toolbar).** Designate → colonists *autonomously* mine/chop/build/haul → stockpile grows, all self-directed. This is the core loop; if watching them fulfil your intentions on their own feels good, the project is real.
- **Slice C — "It's a living colony" (B7 → B6-full → B9-full).** Needs, mood, work-priority grid (policy), alerts. **First real Tier-1b soak here** — leave it running and confirm it survives and stays eventful. Now it's RimWorld-shaped and self-running.
- **Slice D — "It defends itself and persists" (B8 → B10).** Autonomous defense + bounded draft, save/load. Soak must survive raids untouched.
- **Slice E — "I am its god" (B13).** Divine influence: terrain shaping, resource seeding, weather, blessings, calamity, favor economy. This is the slice that turns a self-running colony sim into a *god game* — build the From Dust terrain-shaping power first (you have the reference).
- **Slice F — "It's a new world each time" (B11 + rtsim world-life polish).** Embark, scenarios, living world & chronicle.

- **Embody (B12) is orthogonal.** Depends only on B1–B3, so it *can* land as early as the end of Slice A — but it's most satisfying once there's real work to drop into. Recommend building it alongside Slice C/E. Cheap, high-impact: it reuses vanilla control machinery, so the effort is the mode-switch and clean AI handoff, not new gameplay.

Recommend building **Slice A and B fully before C+**, because they de-risk both tracks (frontend and the autonomy/boundary sim). The **autonomy soak (Tier-1b) becomes a standing gate from Slice C onward** — the world must keep playing itself through every later slice.

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
- **Server operations to reuse (verified):** `common/src/cmd.rs` (`ServerChatCommand`, 96 variants incl. `MakeBlock`, `MakeSprite`, `MakeVolume`, `Object`, `Explosion`, `Lightning`, `WeatherZone`, `Time`, `TimeScale`, `AreaAdd/List/Remove`, `Spawn`, `MakeNpc`, `Buff`, `GiveItem`, `RtsimInfo/Npc`) — trace each to its server-side handler and call that, not the chat string.
- **Possession (verified):** `veloren_server/events/player.rs` (possess handler, `PresenceKind::Possessor`, item-dupe guard).
- **Weather:** `common` `weather::WeatherGrid` (resource fed into the rtsim tick in `veloren_server/rtsim/tick.rs`).
- **Server tick / rtsim integration:** `veloren_server/rtsim/tick.rs` (rtsim ticks within server tick; `CreateNpcEvent`, `NpcBuilder`, `SimulationMode`, `WeatherGrid`, `time_of_day`).
- **UI (verified hybrid):** in-game HUD `voxygen/src/hud/` (conrod); menus `voxygen/src/ui/ice/` (`IcedUi`); debug windows `voxygen/egui/` / `veloren_voxygen_egui` (behind `egui-ui`). Prototype HUD in egui, port to conrod.
- Fluid: `common` `comp::fluid_dynamics` (entity-in-fluid physics only — NOT a terrain fluid-flow solver; From Dust flow is a real build).
- Server binary/harness: `server-cli/`, `server/`.
- Worldgen: `world/` (site/civ generation, map).

*(Verify exact paths against the pinned baseline SHA at the start of each block — the tree moves.)*

## Appendix B — Glossary (Bastion terms)

- **Autonomy (Pillar §1a):** the defining property — the world runs and evolves with zero player input; the player influences, never commands. The Tier-1b soak proves it.
- **Overseer view:** the top-down orthographic god's-eye camera; the default view. (Older passages saying "RTS view" mean this.)
- **Influence (three tiers):** the player's *indirect* agency — **policy** (standing rules), **designation** (one-off intents), and **divine influence / god-powers** (acting on the world). The opposite of unit command.
- **Colonist:** an *autonomous* player-affiliated ECS humanoid with needs/skills/priorities/goals — an agent, not a unit.
- **Designation:** a player-painted intent on the world (dig/chop/build/haul/zone) that the colony fulfils on its own terms.
- **Policy:** standing rules the colony self-organizes around (work grid, zones, schedules, thresholds); the primary loop.
- **Job:** a claimable unit of work autonomously derived from a designation or a need.
- **Work-type:** category a colonist may be enabled/prioritized for (mining, construction, hauling, cooking, melee…).
- **Stockpile:** a zone that collects filtered items; the logistics hub.
- **God-power / Divine influence (B13):** an indirect power acting on world/conditions (terrain shaping, resource seeding, weather, blessing/curse, calamity), costing **favor**, which the autonomous colony then reacts to.
- **Favor (a.k.a. faith):** the influence-economy resource that gates god-powers; accrues from a thriving colony, so the god has limits.
- **Loaded vs. Simulated:** ECS full-fidelity entity vs. rtsim abstract record; governed by `SimulationMode`.
- **Slice (Z-slice):** the render depth cut for viewing/digging layers.
- **Draft (B8):** the *bounded* emergency lever mustering the militia to defend — deliberately coarse so it can't become RTS micro. Not a per-unit command.
- **Embody / Possession (B12):** the god descending into one mortal — direct first/third-person control via vanilla Veloren controls, suspending its AI, then releasing back to autonomy and the overseer view.

---

*End of directive. Build B0 first; do not skip the harness. Read Pillar §1a before every block — the world must play itself.*
