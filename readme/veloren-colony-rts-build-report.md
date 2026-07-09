# Project Bastion — Turning Veloren into an Autonomous God-Game Colony Sim

**A build & test directive for Claude Code**
Version 2.1 · Architect-authored design doc · Target: fork of `veloren/veloren` (Rust, GPL-3)
Lineage: Dwarf Fortress / RimWorld (autonomy & policy) + Black & White / From Dust / Populous (indirect divinity). **Explicitly *not* StarCraft-style unit micro.**
*v2.1 changelog: agents now DO things. Added B-AG5 (world-verb **action library** — gather→build→produce — on the principle of **one library, two drivers**: a verb is defined once and invoked by either a player designation→colonist job OR an NPC's own drive, so colonist work and autonomous NPC life share one codebase) + B-AG6 (**generative systems**: autonomous village growth + deep DF **reproduction/genealogy** with kin graphs & inherited traits — the loop that makes the world grow, not just decline). Both LOD-aware (settlement growth as rtsim events when unwatched, real voxels when loaded; full genealogy only for tracked/loaded lineages). Agency Bible §5c authors world verbs & generative systems. v2.0: the DF Mind (B-AG3/B-AG4).*

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

**Camera & view (Black & White 2 feel).** A top-down / high-oblique orthographic **overseer** camera over a bounded embark region. The feel target is **Black & White 2**, not a stiff RTS cam: **grab-drag panning** (mouse-grab the terrain and pull the world under a fixed cursor), plus WASD as a shared fallback; **free continuous orbit + pitch** (swoop from near-top-down down to a low oblique horizon) with a quick **snap-to-top-down** for reading the fortress; smooth **zoom that dollies from whole-region to near-ground**; and **inertia/easing** so it feels alive. A **Z-layer / depth slice** control (DF's most important interaction, already built in B1) cuts away everything above a layer so the player can dig down and read the colony in cross-section — Veloren is fully 3D voxel, so this is a render filter, not new geometry. Chunk streaming must follow the *camera* (via `client.spectate_position`), not a hero entity, so panning never hits LoD terrain.

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

## 3b. Input contexts — separate control schemes per mode (first-class system)

There are **three distinct interaction modes**, and each owns its own control scheme rather than fighting over one keymap. (The B1 builder already hit this: it had to gate `Q` because the HUD claimed it for a hotbar slot — that's the smell of two schemes sharing one binding table. Fix it structurally now, before B2 piles more god-mode keys on.)

The modes:
1. **Overseer (god mode)** — the default. B&W2 grab-drag / orbit / zoom / depth-slice, plus designation and (later) divine-influence tools. This is the game you play.
2. **Avatar (embodied)** — active only while possessing a colonist (B12). **Controls are *exactly* vanilla Veloren** (WASD, mouse-look, real combat, hotbar) so it feels native. Entering an avatar swaps the *whole* control context to vanilla; releasing swaps back to Overseer.
3. **Menu / vanilla** — untouched, for regression safety and menus.

**The system (build in B1.5, before B2's tools):** a `bastion` **InputContext** layer sits above `GameInput`. The active context owns a **binding table**; switching mode swaps the table wholesale (not key-by-key), and routes input to the right consumer (god camera/tools vs. the possessed entity's `comp::Controller`). Rules:
- **Movement keys (WASD) are shared** across Overseer and Avatar by design — they pan the god camera in one, move the body in the other. **Everything else is mode-specific**, so no more HUD-vs-camera key collisions.
- Context transitions are **clean and atomic**: on Embody, suspend god-mode bindings and hand input to the avatar (vanilla); on Release, the reverse. This dovetails with B12's server-authoritative control handoff — one mode change, not a scatter of per-key toggles.
- The HUD must **not** consume keys that belong to the active context (the B1 `Q` gate becomes a general rule: the context layer arbitrates, HUD yields in Overseer mode).

**Rebinding (recommendation, decided):** **do not build a rebind UI yet** — hardcode good defaults. But structure the binding tables **per-context from the start** so a settings screen with **separate keybind tabs per mode** (Overseer / Avatar) drops in cleanly at B9 with no data-model rework. This is the cheap-now, no-retrofit path.

---

## 3c. God mode vs. Free mode — two rulesets, one interaction surface (canon)

The player picks which game they're playing, via one setting. The interaction surface (left-drag pan,
left-click select, right-click radial menu, tool palette; §B2a) is **identical** in both — only the *rules*
around direct control differ. This is how the "kitchen sink" of interaction coexists with the autonomy
pillar (§1a): direct control is a **divine intervention you spend**, not a persistent RTS mode.

- **God mode (default — the real game).** Two restrictions keep it a *god game*:
  1. **Target restriction** — you may select/force-act only on entities **under your influence** (your
     colony). The wider world you affect *indirectly* (designations, god-powers), never by direct command.
  2. **Metering** — force-actions (force-move, force-do-job, and to a degree possess) cost something. A
     **toggle** picks the limiter: a **favor meter** (draws from the B13 favor/faith economy) **or** a
     **cooldown**. Both exist; the player chooses the discipline.
  The distinction that matters: *free, unlimited unit-commanding is an RTS; a god who can occasionally force
  a hand is a god game.* Same verbs — the meter/cooldown is what makes it the latter.

- **Free mode (sandbox / creative).** Both restrictions lift: unlimited force-action and possession over
  **any** entity, no cost. For testing, building showcases, or playing god without constraints.

**Implementation note:** the God/Free toggle and the target-restriction check are **stubbed at the input
layer in B2a** (no teeth until B3 spawns a colony and B13 adds favor), then enforced in **B2b**. The favor⇄
cooldown limiter toggle likewise lands with B2b/B13. Building the hook early means no rework when the colony
and economy exist.

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
- **STATUS: DONE.** `CameraMode::Overseer`, ortho reversed-depth projection, `Globals.bastion_slice_z` shader slice, `--bastion-overseer` flag, F9 toggle, WASD/Q-E/PgUp-PgDn. 60 fps @ 4K. See `BASTION_CAMERA.md`.

### B1.5 — Overseer control layer: input contexts + Black & White 2 camera feel + streaming
**Objective:** Build the **overseer control layer, done right** — (a) a `bastion` **input-context system** so god-mode and avatar-mode own separate control schemes (§3b), and (b) inside it, make the overseer camera *feel* like **Black & White 2** — grab the world and pull it, swoop freely from top-down to low oblique, zoom toward the cursor with inertia — and (c) stream chunks under the **camera** so panning never decays to LoD. No simulation, no interaction surface (that's B2). Supersedes B1's placeholder WASD/90°-snap controls.
**Why before B2:** designation-painting and inspection want a solid, grabbable, correctly-picking world *and* a control-mode system to hang tools on. The B1 builder already hit a HUD-vs-camera key collision (`Q`); the context system fixes that class of bug structurally before more god-mode keys land.
**Touches (verified from B1 findings):** a new `bastion` **InputContext** layer above `game_input.rs`/`control.rs` (per-mode binding tables); `voxygen/src/scene/camera.rs` (`CameraMode::Overseer`, free yaw+pitch, zoom easing); `scene/mod.rs` (overseer `maintain` arm); `session/mod.rs` (grab-drag + orbit input routed through the active context, replace 90° snaps); and `client.spectate_position(pos)` for streaming under the focus.
**Approach:**
- **Input-context system (§3b) — build this first, it's the foundation:** an `InputContext` enum (`Overseer`, `Avatar`, `Menu`) with a per-context binding table. The active context owns which bindings are live and where input routes (god camera/tools vs. a possessed entity's `comp::Controller`). **WASD is shared** across Overseer/Avatar by design; **everything else is context-specific.** Context switches are **atomic** (swap the whole table, not key-by-key). The HUD must yield keys the active context owns (generalize the B1 `Q` gate). Structure binding tables per-context now so a rebind UI with per-mode tabs drops in at B9 — but **do not build the rebind UI yet**; hardcode good defaults.
- **Grab-drag panning (the B&W signature):** on drag, screen→world–pick the ground/slice point under the cursor (via `Camera::dependents()` `proj_mat_inv`/`view_mat_inv`, intersect the active slice/ground plane) and pan so that world point stays locked under the cursor. Add release **inertia** that eases out.
- **Free orbit + pitch:** replace the 90° yaw snaps with continuous yaw, and add continuous **pitch** from near-top-down (~89°) down to a low oblique horizon (~20–25°). Orbit on right-drag (or modifier). Smooth/damped, with an optional **snap-to-top-down** for DF-style reading.
- **Zoom-to-cursor with easing:** scroll dollies ortho scale from whole-region to near-ground, eased, zooming *toward the point under the cursor* (B&W style), not screen center.
- **Stream under the camera:** call `client.spectate_position(focus)` each frame so terrain streams under the overseer focus, killing the pan-to-LoD problem the B1 findings flagged.
- **Keep the Z-slice correct at any pitch/orbit;** keep everything `bastion`-gated so vanilla + the existing overseer flag stay intact.
**Done-when:**
- **Input contexts:** switching context swaps the whole binding table atomically; WASD works in both Overseer (pans) and Avatar (would move a body) while other keys are mode-specific; the HUD no longer steals context-owned keys (the `Q` collision is gone by construction, not by special-case). A stubbed `Avatar` context can be toggled for testing even before B12 wires real possession.
- Grab-drag pulls the world under the cursor with the grabbed point staying locked to it; release carries eased inertia — reads like B&W2.
- Continuous yaw + pitch from top-down to low swoop, smooth, no forced snapping (snap-to-top-down available on demand).
- Zoom eases from whole-region to near-ground, toward the cursor.
- Panning far keeps full-detail terrain (chunks stream under the focus via `spectate_position`) — no LoD wall.
- Z-slice still cuts correctly at any camera angle; vanilla + prior overseer flag intact; >50 fps held.

### B2a — Overseer interaction surface (the "kitchen sink" input layer)
**Objective:** The full overseer interaction surface, faithful to the pillar: **left-drag pans** (B1.5),
**left-click selects/inspects**, and **right-click opens a contextual radial menu** — the primary "affect the
world" verb surface — plus a **tool palette** to pin a persistent mode (pan / inspect / designate-paint).
This is plumbing + menu framework; menu entries are **server-echo stubs** until B3/B4/B13 give them behavior.
**God mode vs. Free mode (canon — see §3c):** the interaction surface is the same in both; the *rules* differ.
In **God mode** (default) you may only select/act on entities **under your influence** (your colony), and
force-actions are **metered** (favor⇄cooldown toggle). In **Free mode** (sandbox) those restrictions lift.
B2a **stubs** the God/Free toggle and the target-restriction check at the input layer (no teeth until B3
spawns a colony + B13 adds favor) — but wires the hook so B2b/B3 can enforce it without rework.
**Touches (verified seams from B1.5 closeout):**
- `voxygen/src/bastion/` — add tool-mode state + the radial-menu widget; add tool `GameInput`s to
  **`OVERSEER_SCHEME.owned`**, reclaiming the currently-suppressed **Primary/Secondary/Interact** slots.
- Picking: reuse **`unproject_to_world_plane`** (slice height = active work layer) and the **grab-drag
  raw-mouse pattern** with the **`bastion_cursor_over_widget`** HUD gate — all handed over by B1.5.
- `common/src/comp` — a `Selectable`/`Selected` marker (inspection/HUD + feeds B1.6 cutaway targets).
- Client→server `bastion` messages (stubbed payloads): `PlaceDesignation { region, kind }`,
  `ApplyInfluence { target, kind }`, `ContextAction { target, verb }`. Server validates + echoes; no real
  behavior yet, and **no free per-unit command verb** (force-action lives in B2b, metered).
**Approach:**
- **Cursor defaults:** empty space left-drag = pan (unchanged); left-click on unit/tile = **select/inspect**
  (mark `Selected`, populate HUD, feed B1.6 cutaway); right-click = **contextual radial menu**.
- **Radial menu framework (radial + "More…"):** a fast pie of the top ~6 context actions with a **"More…"**
  wedge that expands to a dense list for crowded contexts (full colonist policy, god-power palette). Context
  resolves from what's under the cursor: rock → *mine*; tree → *chop*; ground → *build/stockpile/bless/rain*;
  colonist → *set policy / Embody / Force Action* (last two are B2b/B12, shown but stubbed/greyed).
- **Tool palette:** pin pan / inspect / designate-paint as a persistent mode; designate-paint drag-marks a
  region → `PlaceDesignation` (server echoes for overlay render). Everything routes through the Overseer
  input context.
**Done-when:**
- Left-drag still pans (B1.5 intact); **left-click selects/inspects** a unit/tile (HUD shows detail, marker
  set); designate-paint marks a region the server echoes back as an overlay.
- **Right-click opens the radial menu** context-appropriate to what's under the cursor; "More…" expands to a
  list; selecting an entry sends the stubbed `ContextAction`/`ApplyInfluence` with correct world coords.
- The God/Free toggle + target-restriction hook exist at the input layer (stubbed — God mode will later gate
  targets to the colony); tool palette switches modes; all within the Overseer context, `bastion`-gated,
  vanilla intact.
- Reclaimed Primary/Secondary/Interact fire the new tools (the B1.5 "suppressed slots" TODO is closed).

### B2b — Force-action & possess as metered god powers (after B3)
**Objective:** The sanctioned **direct-control escapes**, framed as *divine intervention you spend*, not RTS
micro: **force-move / force-do-job** on a colonist, and the **Embody** entry point (B12). Per your decision,
this is a **two-ruleset** feature toggled by God/Free mode.
**Depends on:** B3 (colonists to act on) + a favor stub (B13). Slots in after B3; before then the menu entries
from B2a are shown-but-stubbed.
**Approach:**
- **God mode (default):** force-actions may target **only entities under your influence**, and are **metered**
  — a **toggle** picks the limiter: a **favor meter** (draws from the B13 economy) *or* a **cooldown**. Same
  verbs, disciplined. This preserves the autonomous-god-game pillar (a god who *occasionally* forces a hand,
  not a general issuing free orders).
- **Free mode (sandbox):** target restriction and cost both lift — unlimited force-action + possess over
  anything. For testing, showcases, or pure sandbox play.
- Verbs: `ForceMove(target, pos)`, `ForceJob(target, job)`, `Embody(target)` (hands to B12's context swap +
  server-authoritative controller handoff). All server-authoritative; force-action suspends the colonist's
  autonomous AI for the duration then returns it (like a mini-embody), never leaving an orphaned job.
**Done-when:**
- In **God mode**: force-move/force-job work **only** on colony entities; the active limiter (favor or
  cooldown, per toggle) is spent/enforced and shown in the HUD; the forced colonist resumes autonomy cleanly
  after (no orphaned job, single driver).
- **Embody** from the radial menu drops into the entity via B12's path and releases back to Overseer.
- In **Free mode**: the same verbs work on any entity with no cost; toggling back to God mode re-imposes
  target restriction + metering.
- Invariant: force-action never double-drives an entity (one controller at a time) and never dupes/loses a job.

### B1.6 — Overseer occlusion & transparency system (all four view modes, one framework)
**Objective:** Generalize B1's hard Z-slice into **one occlusion framework** that drives four composable
view behaviors, so the player rarely needs the manual cut. All four are the *same fragment operation*
(`discard`→`fade` with a smarter threshold) — build the general machinery once, then each mode is a cheap
parameterization. Entity/roof inputs that don't exist yet are **stubbed** (real data arrives with B2 hover/
selection and B3 colonists). Can be built now; slot it right after B1.5 (it depends on the overseer camera +
input contexts, not on simulation).
**The four modes (compose via a bitmask):**
1. **Solid** — nothing hidden (vanilla look).
2. **Soft slice (manual)** — B1's hard cut upgraded to a smooth fade band. The deliberate "read the fortress in cross-section" tool.
3. **Proximity / height transparency (ambient)** — foreground floor near the focus stays visible while geometry fades by **height above the focus plane** and/or **distance from focus**, driven by a strength slider. The always-on readability layer.
4. **Automatic occlusion** — two behaviors that need no manual input: **roof/interior reveal** (RimWorld-style: fade the roof over enclosed spaces you peer into) + **camera-to-target cutaway** (Diablo-style: fade geometry between the camera and tracked entities). This is the "smart" default once B2/B3 feed real targets.
**Touches (generalizes B1's slice hook):**
- Shader globals: expand `Globals.bastion_slice_z` into a **`bastion_occlusion`** block (mode bitmask, `focus_z`, `fade_band`, height/distance falloff + strength, `slice_z`, `target_count`+`targets[]`+`cutaway_radius`, roof/reveal params) in `assets/voxygen/shaders/include/globals.glsl` + `voxygen/src/render/pipelines/mod.rs` (respect std140).
- One shared **`bastion_occlusion_alpha(f_pos, world_pos)`** function in a shared shader include, returning visibility 0..1. Every fragment pass multiplies/discards by it — the single chokepoint.
- Apply across **all passes**: `terrain-frag`, `sprite-frag`, `fluid-frag` (cheap+shiny), **`figure-frag`**, **`particle-frag`**, and the **shadow pass** (so hidden roofs don't cast onto revealed interiors). *Shadows are the hardest pass — attempt them; if perf/scope blows, shadows are the one acceptable deferral, documented.*
- Per-frame params: `scene/mod.rs` computes `focus_z`, mode, falloffs, and the tracked-target array; `session/mod.rs` adds a **view-mode cycle** key + slider/toggles, routed through the **B1.5 Overseer input context**.
- **Interior re-lighting (do it properly — decided):** revealed/exposed interior voxels must look *lit*, not black. Inject a soft top-down fill light over revealed regions (RimWorld reads as "lit from above") rather than a flat ambient boost. This is the highest-effort part of the block.
- **Roof/enclosure mask:** a cheap "is this column covered above the focus plane?" signal. Prefer a precomputed/approximate coverage signal over expensive in-shader upward sampling. Approximate is fine this block; refine when B2/B3 land.
- **Stubbed tracking (replace later):** cutaway `targets[]` = camera focus + debug markers now → hovered/selected entities (B2) + colonists (B3) later. Mark stubs explicitly.
**Controls (via B1.5 Overseer context):** a **view-mode cycle** key (Solid → Reveal → Slice) *and* a **transparency slider + per-mode toggles** — exposed now via the egui debug panel (already available behind `egui-ui`), structured to move into the B9 settings tab. (Product owner wants both.)
**Done-when:**
- One `bastion_occlusion` uniform + one shared alpha function drive all modes; adding/removing a mode is a parameterization, not a new pass.
- **Solid** = vanilla look. **Soft-slice** = B1's cut with a smooth fade band (no hard aliased edge). **Proximity/height transparency** = foreground floor near focus visible while tall/background geometry fades by a working slider.
- **Roof/interior reveal** visibly works on a test building (approximate mask acceptable), and revealed interiors are **re-lit and readable**, not black.
- **Camera-to-target cutaway**: geometry between camera and a stubbed target fades so the target shows through walls; composes across multiple targets.
- Fade applies across terrain, sprites, fluid, **figures, and particles** (shadows too, or shadows documented as the single deferral).
- View-mode cycle key + transparency/toggle debug panel work through the Overseer context; params structured for the B9 settings tab. Vanilla + overseer flag intact.
- **Perf:** this is the most GPU-sensitive block so far — measure and report fps in each mode; keep the alpha function cheap (no expensive per-fragment upward sampling for the roof mask). Hold >50 fps.

### B1.7 — Overseer LoD & frustum tuning (make the zoomed-out view whole and beautiful)
**Objective:** Fix two distinct zoom-out artifacts so the overseer view reads as a continuous, beautiful
world at any zoom: (1) the **hard bottom cut-off / black wedges**, and (2) the **too-aggressive LoD /
detail falloff** from the overseer's altitude+zoom. Target look: **crisp streamed terrain near the focus,
melting smoothly into Veloren's existing distant LoD/map-terrain backdrop, extended to the horizon with a
soft distance fade** — no hard edge, no detail ring, no shimmer. **Build order:** independent of B1.6;
recommended *next* since it fixes a visible artifact in the primary view.
**Diagnosis (two different causes — don't conflate):**
- The straight-line bottom cut + black triangles = **orthographic frustum/far-plane + rendered-region
  edge**, NOT streaming/LoD. In perspective, distance melts into fog; in an ortho top-down cam, geometry
  outside the box/planes simply stops flat and you see the void. Fix in the **camera/render extent**, not
  the streamer.
- The washed-out, shimmering (moiré/contour) distance IS **LoD**, but firing early because thresholds are
  tuned for a **ground-level player**, not a hoisted, zoomed camera.
**Touches (verify against B1/B1.5 findings):**
- `voxygen/src/scene/camera.rs` — ortho **near/far planes scale with zoom**; far plane + ground extent must
  cover the visible box so there's no void wedge.
- Distant terrain: Veloren's **LoD/map-terrain system** (e.g. `voxygen/src/scene/lod.rs` + `world` map
  data) — ensure it renders **under the overseer cam to the horizon** (it may be culled or unrequested in
  this mode).
- View distance: the client terrain view-distance request — **raise/scale with overseer zoom** (bounded for
  perf), alongside the `spectate_position` focus from B1.5.
- Distance fade: reuse Veloren's fog/horizon fade, but drive it by **world-distance-from-focus** (ortho has
  no natural depth-fog), so crisp→LoD→horizon is a smooth melt, not a cut.
- Shimmer: at the overseer's grazing ortho angle the low-detail LoD terrain aliases — apply a mip/detail
  bias or flatten far normals to kill the moiré.
**Approach (my recommended answers, baked in):**
- **Far look = crisp near, graceful LoD backdrop far** — reuse the *existing* LoD/map terrain as the
  distant field; don't build a new system. Near focus streams crisp chunks; the far field is cheap LoD.
- **Edge = extend to horizon with distance fade** (no hard "tabletop" border). A deliberate bounded map
  edge is a valid *later* option if defined embark borders are ever wanted — deferred.
- Scale crisp view-distance + LoD transition distances with zoom so you never see a hard detail ring or the
  region-edge seam; extend the ortho far plane + a ground skirt so no black wedge appears; apply the
  world-distance fade so distant LoD melts into the horizon.
- **Bound by fps:** at max zoom prefer more LoD backdrop over more crisp chunks (crisp near, cheap far).
**Done-when:**
- **No black wedge / hard bottom cut** at any zoom or pan; the world reads continuous to the horizon.
- Zooming out shows crisp terrain near focus **melting smoothly into distant LoD terrain**, then a soft
  horizon fade — no hard detail ring, no region-edge seam.
- The **moiré/contour shimmer** on distant terrain is gone or strongly reduced.
- **fps held at max zoom** (report numbers); memory bounded (crisp view distance capped sensibly).
- Vanilla + overseer flag intact; Z-slice / B1.6 occlusion still correct at all zooms.

### B1.8 — Camera navigation: map fly-to + surface / underground elevation modes
**Objective:** Two things that make the overseer camera actually navigable in 3D: (1) **map fly-to** — press
`M`, click a location, and the camera **smoothly flies** there (B&W style); and (2) **two elevation
modes** that make a 3D fortress legible instead of a clipping mess:
- **Surface mode (above ground):** the camera focus **rides the terrain heightmap** — panning across a
  mountain lifts the camera *over* it (never clips through), and underground is never shown. The surface is
  the floor. Default outdoor view; occlusion default = B1.6 **auto-reveal/cutaway** (roofs/trees/cliffs in
  front fade).
- **Underground mode:** the focus drops to a **free depth cursor** and the world reads like a Dwarf Fortress
  cross-section — everything **above** the working layer hidden, everything at/below **revealed and re-lit**
  (this *is* B1.6's slice + interior relight, now the primary view, not an occasional tool).
This is not new tech — it's a **focus-height policy** on the B1.5 camera, feeding the B1.6 occlusion modes.
**Depends on:** B1.5 (camera, input contexts, `spectate_position`) and **B1.6** (slice + reveal + relight —
underground mode reuses it). Independent of B1.7. Build after B1.6.
**Design decisions (baked in):**
- **Map click → smooth eased fly-to** (not a snap): move focus over the target, surface-clamp Z on arrival,
  stream via `spectate_position` en route so there's no LoD pop / black edge on landing.
- **Mode switch = both:** a manual **toggle key** for intent, **plus smart auto** — pushing the depth cursor
  below the surface (or descending past terrain) flips to underground; rising back above flips to surface.
  Never stuck.
- **Underground depth = free depth cursor** (PgUp/PgDn fly to any layer) **+ snap conveniences**: snap to the
  dug floor under the cursor, snap to the deepest dug layer. Free-fly is primary; snaps keep it anchored.
- **Surface→underground transition = ease/descend through terrain** — focus Z eases down with the slice
  engaging progressively (feels like descending into the fortress), reversed on ascent. No hard cut.
**Touches (verify against B1.5/B1.6 findings):**
- `voxygen/src/scene/camera.rs` — focus-Z **policy** (surface-clamp vs. free-depth), easing for both fly-to
  and the descend transition.
- **Terrain height lookup** under the focus XY (cheap): the client terrain column height / `world.sim()`
  column, or a downward sample — for surface-clamp. Must be cheap (runs per frame while panning).
- `session/mod.rs` — mode toggle key, underground depth cursor (reuse B1 PgUp/PgDn), auto-switch logic, and
  routing the depth into B1.6's slice params; all via the B1.5 Overseer input context.
- **Map UI** in `voxygen/src/hud/` (conrod map/minimap widget already on `M`) — intercept a click, resolve
  to world XY, trigger fly-to. Reuse the existing widget; don't rebuild the map.
- `client.spectate_position(focus)` — stream under the focus during fly-to and mode transitions.
- Integrate with **B1.6**: surface mode selects the auto-reveal occlusion preset; underground selects the
  slice-above + reveal/relight-below preset.
**Done-when:**
- `M` opens the map; **clicking a location smoothly flies** the overseer there (eased), lands surface-clamped,
  terrain streamed — no LoD pop, no black edge on arrival.
- **Surface mode:** panning across a mountain **lifts the camera over it** — no clipping through terrain;
  underground is never visible.
- **Underground mode:** the free depth cursor flies to any layer; snap-to-dug-floor / snap-to-deepest work;
  everything above the working layer is hidden, at/below is revealed and **re-lit** (via B1.6).
- **Switching:** the manual toggle works; smart-auto flips when the depth cursor/descent crosses the surface
  and back; never lands in a broken state.
- **Transition** eases through the terrain (descend feel), not a hard cut.
- Vanilla + overseer flag intact; fps held (report numbers); works with B1.6 occlusion and B1.7 LoD.

### B1.9 — Tilt-shift post-process (the miniature-diorama capstone)
**Objective:** A **tilt-shift** post-process that gives the overseer view the miniature-diorama look
(Cities: Skylines / god-game signature) — the visual that sells "I'm a god looking down at a living little
world." Pure **post-process**: self-contained, touches no simulation/streaming/occlusion, cheap relative to
everything else. Last of the camera-polish run; build after B1.6/B1.8 so it can reuse their focus field.
**The ortho caveat (shapes the implementation):** true tilt-shift blurs by depth, but the overseer camera is
**orthographic** with a flat, non-perspective depth buffer, so naive depth-based blur looks wrong. Drive the
blur by a **screen-space focus band centered on the focus point's screen projection** (this is what
tilt-shift photographically is, and it's robust across orbit/pitch), **optionally modulated by
focus-plane distance** — reusing the focus field B1.6/B1.8 already compute rather than inventing one.
**Design decisions (baked in):**
- **Strength = slider (subtle → strong).** Adjustable band width + blur strength, from light cinematic hint
  to full toy-diorama.
- **Auto-scale with zoom:** strong when zoomed out (god view), eases off as you zoom toward ground level —
  prevents the up-close nausea a constant blur causes.
- **Miniature color boost = separate toggle** in the same pass (slight saturation/contrast bump — the other
  half of the toy-model illusion). Blur slider always; color-boost on/off.
- **Overseer-only:** off in vanilla and in the Avatar (embodied) context.
**Touches (verify against B1.6/B1.8 findings):**
- Veloren's **post-process pass** (`assets/voxygen/shaders/postprocess-frag.*` + the post/clean pipeline in
  `voxygen/src/render/pipelines/`) — add the tilt-shift blur (separable Gaussian, two-pass, or a cheap
  approximation) + optional color grade. Reuse/extend the existing post stage; don't add a parallel one.
- Uniforms: band center (= focus point's screen projection), band width, blur strength, zoom-scale factor,
  color-boost toggle. Reuse the **focus field already in `Globals`** (from B1.6) for band centering and the
  optional focus-distance modulation.
- Controls: toggle + sliders via the B1.5 Overseer context / egui debug panel now, structured for the B9
  settings tab.
**Done-when:**
- Tilt-shift toggle on → miniature-diorama look; the **strength slider** ramps subtle → strong (band width +
  blur).
- The **sharp focus band tracks the focus/center**; blur ramps above and below it.
- **Auto-scales with zoom** (strong out, eases in near ground); toggles cleanly off.
- The **color-boost toggle** works in the same pass (saturation/contrast).
- Effect is **overseer-only** (off in vanilla + Avatar); composes with B1.6 occlusion, B1.7 LoD, B1.8 modes.
- fps held (report numbers; the blur kernel is bounded/eased by zoom).

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

### B-AG1 — Loaded fidelity: make promoted NPCs honor their rtsim lives (the "nobody stands around" fix)
**Objective:** Kill the "NPCs just stand around" problem for the **entire population at once**, before any
per-species authoring. The cause (see Agency Bible §1): when a rich rtsim NPC promotes to a loaded ECS
entity, its high-level intent (`NpcAction`/`NpcActivity` via `RtSimController`) collapses into a generic idle
`agent` behavior. Fix: make the **loaded `agent` faithfully continue the rtsim plan** — the trader keeps
trading, the guard patrols, the hunter hunts, the raid party keeps marching — instead of resetting to idle.
**This is the single highest-leverage agency block; do it first.**
**Depends on:** nothing new (rtsim + agent already exist). Can run in parallel with the camera/sim tracks.
**Touches:** the loaded `agent` AI (voxygen/server agent code), the rtsim→loaded handoff at promotion,
`common/src/rtsim` (`NpcAction`/`NpcActivity`/`RtSimController`), `rtsim/src/rule/npc_ai.rs`.
**Approach (rtsim-safe — Agency Bible §0 law):**
- On promotion (`SimulationMode::Simulated → Loaded`), read the NPC's **current rtsim activity/goal** and
  have the loaded agent *resume it* (travel to site/station, patrol beat, hunt, march) rather than defaulting
  to idle-wander. One brain (rtsim intent), two fidelities (abstract vs. physical).
- **Assume nothing** (the rtsim law): the NPC's target/home/faction may be gone or changed at any tick —
  every continuation must **degrade gracefully** (re-plan, pick a new target, wander only as a last resort),
  never freeze or panic. A behavior that assumes its target persists is the same "standing around" bug in a
  new coat.
- Keep it **cheap and general** — a type-agnostic "resume your rtsim intent" bridge, not per-species logic
  yet (that's B-AG2). This one fix should visibly animate most of the world.
**Done-when:**
- In the loaded view, promoted townsfolk resume their **daily task/phase** (go to station/tavern/home) rather
  than spawning idle; travellers keep travelling; wild predators/prey resume hunting/grazing; a raid party
  keeps marching.
- **Graceful failure:** deleting/moving an NPC's target mid-action does not freeze or crash it — it re-plans
  or falls back (log-verified).
- **Boundary invariant:** repeated promote/demote cycles leave no idle "stuck" NPCs and no entity leak
  (harness-asserted). Measurable drop in idle-standing NPCs vs. baseline.
- Vanilla behavior for NPCs with genuinely no rtsim intent is unchanged; fps unaffected (it's the same agents,
  smarter).

### B-AG2 — Agency depth: per-archetype behavior from the Agency Bible
**Objective:** Now that loaded NPCs *act* (B-AG1), **deepen** their agency to DF level per the **Agency Bible**
— purpose, home/territory, faction, and who/what/how they interact — starting with the **flagship archetypes**
and expanding group by group. This is content + systems, authored **tendency-first**.
**Depends on:** B-AG1 (agents must express intent before you enrich it). Uses the Agency Bible as its design
source. Pairs with B7 (needs/AI patterns) and B8 (raids consume the raider archetype).
**Touches:** `rtsim/src/rule/npc_ai.rs` (+ new rules) as the behavior authoring home; `NpcAction`/
`NpcActivity`; `assets/common/entity/wild` + `common/src/comp/body.rs` (the type inventory / stats source of
truth); the loaded agent bridge from B-AG1.
**Approach:**
- Implement the **four flagship archetypes** first (Agency Bible §3): Townsperson (faction/site/profession/
  daily rhythm), Wolf (territorial pack predator), Deer (herd/graze/flee/migrate), Wyvern+Raider (roaming apex
  / lair-based raid). Each authored as **tendencies with graceful failure** (Bible §0/§5), driven through
  rtsim intent, expressed by the loaded agent.
- Prove the schema end-to-end on the flagships (predator hunts prey; prey herds/flees; townsfolk run daily
  routines + trade; raiders muster→march→raid→retreat), then **expand group by group** through the Agency
  Bible §4 inventory (map each body type to a flagship template + deviations).
- **Ecosystem tendency (Bible §5.4):** predator/prey/faction relations should tend toward equilibrium
  (populations and factions wax/wane) — this is what makes it read DF-alive, not scripted. Feed rtsim's
  wildlife-population and repopulation systems.
**Done-when:**
- The four flagship archetypes behave per the Bible in both simulated and loaded tiers (verify a hunt, a herd
  flee, a townsperson's day, a raid cycle).
- Expansion pass(es) apply the templates to the §4 groups; **no body type is fully inert** (even critters
  forage/flock/scatter).
- **Tendency-safe:** every behavior handles missing target/home/faction gracefully (no freeze/crash) — the
  rtsim law holds under the Tier-1b soak.
- **Ecosystem invariant (Tier-1b):** over a long zero-input soak, populations/factions shift plausibly
  (predation, migration, faction activity) without runaway or extinction-to-zero, and the event log shows
  varied NPC life — the world visibly *lives*.

### B-AG3 — The Mind: DF-style personality → values → memory → thoughts → emotions → mood
**Objective:** Give every creature a **Dwarf Fortress–style inner life** that *drives behavior* — the crown
jewel of the DF transfer. Implement the **causal pipeline** (Agency Bible §5b): an event, filtered through an
individual's **personality facets**, **values**, and **memory**, produces an **emotion**, which feeds a
running **mood** that at extremes triggers breakdown/tantrum/elation. Two NPCs with different personalities
experience the same event differently — that's the whole point.
**Depends on:** B7 (needs/mood substrate), B3 (colonist/NPC entities), the Veloren sentiment system (relationship
seed). Pairs with B-AG1/B-AG2 (agency consumes the mind). Feeds B-AG4 (the inspector displays it).
**Touches:** new `bastion_mind` model in `common` (personality, values, memory, relationships, thoughts, mood);
rtsim data (mind summary persists in the flat-table model); `rtsim/src/rule/npc_ai.rs` + agent AI (behavior
*consumes* mood/values/grudges); the loaded↔simulated promotion path (mind LOD).
**Approach (rtsim-safe + LOD-critical):**
- **Data model (Agency Bible §5b.2):** personality facets (start ~a dozen, expandable toward DF's ~50; set at
  creation, stable), weighted values, a **decaying** memory log with **persistent** items (grudges/bonds/
  trauma), per-actor relationships (deepening Veloren sentiment), thoughts (source+intensity+decay), mood
  aggregate → states. All `serde`-ready (B10).
- **The pipeline:** an event + personality + values + memory → an emotion of some intensity → mood update.
  Thoughts decay; grudges persist (the deliberate asymmetry). Author every step as a **tendency** that
  degrades gracefully if inputs are missing (Bible §0 law).
- **Mind LOD (Agency Bible §5b.3 — obey or it breaks the engine):** *every* creature has the full model, but
  it runs at **level-of-detail mirroring the body loaded/simulated split**. Simulated tier = a **cheap summary**
  (dominant mood, key relationships, grudges) on throttled rtsim ticks. Loaded/selected/possessed = **full
  resolution** (all facets, live thought generation). Promote the mind to full-res on load/inspect, demote to
  summary when unwatched, persisting durable parts. **Do NOT run full minds every tick for thousands of NPCs**
  (main doc gotcha #1). Animals get the same model with a lighter personality/values profile — a lighter
  parameterization, not a lesser system.
- **It must drive behavior:** mood gates work (a breaking colonist refuses jobs — ties to B7); values/
  personality bias job and social choices; grudges alter who aids whom in a fight; a god force-action (B2b)
  leaves a resentful thought. Wire these consumers, or it's cosmetic.
**Done-when:**
- Two NPCs with different personalities produce **different thoughts** from the **same** event (headless-verify
  the pipeline output differs by personality/values).
- Thoughts **decay** over time; a grudge/bond **persists** across a long soak and across loaded↔simulated
  cycles (no reset).
- **Mind LOD holds:** full minds run only for loaded/selected/possessed NPCs; simulated NPCs carry the cheap
  summary; **Tier-1b soak stays performant** with thousands of NPCs (no per-tick full-mind blowup — assert fps/
  tick-time bounded).
- The mind **drives behavior**: a mood breakdown changes what a colonist does; a grudge changes a combat/aid
  decision (log-verified).
- Promotion/inspect populates a full mind; demotion persists durable parts (no dupe/loss across the boundary).

### B-AG4 — DF-style unit inspector (select any NPC, see everything)
**Objective:** Selecting any NPC opens a **full Dwarf Fortress–style unit sheet** — Thoughts, Personality,
Relationships, Needs, Health, Skills/Labors — surfacing the mind (B-AG3) and agency (B-AG1/2). This panel is
also the project's **honesty check and build checklist**: each tab corresponds to a system that must exist to
fill it. An empty tab means that system isn't real yet — you can't fake it.
**Depends on:** B2a (selection), B3 (skills), B7 (needs), B-AG3 (mind), B8/combat (health). Tabs light up as
their systems come online — build the shell early, populate progressively.
**Touches:** `voxygen` HUD (per §2a: prototype in **egui** for speed, port keepers to **conrod**; menus use
iced); the state channel from server; read models for mind/needs/skills/relationships/health.
**Approach:**
- A tabbed inspector opened on select (B2a) / on demand:
  - **Thoughts** — recent emotional events with source + intensity + current mood/state (from B-AG3).
  - **Personality** — the facet profile + values (what they care about).
  - **Relationships** — friends/rivals/kin/lovers/grudges with sentiment (from B-AG3 + Veloren sentiment).
  - **Needs** — hunger/rest/recreation + social/creative (from B7).
  - **Health** — bodypart-level condition/wounds (deepen Veloren's real health/combat).
  - **Skills / Labors** — skills + the RimWorld-style work-priority grid (from B3/B4).
- On open, **promote the NPC's mind to full-res** (B-AG3) so the sheet is fully populated; demote on close.
- Works for **any** NPC/creature (animals show the lighter mind profile), not just your colony — inspecting the
  world is core to the DF experience and the god fantasy.
- Structure read models so tabs render whatever data exists and clearly show "system not yet built" for
  unfilled tabs during development.
**Done-when:**
- Selecting any NPC opens the unit sheet; each **built** system's tab is populated with live, correct data
  (thoughts update, needs tick, relationships/grudges show, skills/priorities editable where applicable).
- Inspecting **promotes the mind to full-res** so Thoughts/Personality are fully populated even for a
  previously-simulated NPC; closing demotes cleanly.
- Works on colonists **and** wild creatures/other-faction NPCs (animals show the lighter profile).
- Unbuilt tabs render a clear "not yet implemented" state (the panel doubles as the visible build checklist);
  vanilla + prior blocks intact; fps unaffected by opening the sheet.

### B-AG5 — World-verb action library + NPC drives (agents that DO things on their own)
**Objective:** Give agents a shared **action library** of world-affecting verbs — chop, mine, forage, hunt,
fish, build, farm, craft — and let **NPCs invoke them from their own drives**, not only when the player
designates work. This is what makes the world *productive* and DF-alive: a woodcutter fells trees because
it's his profession; a village commissions a house because it's growing.
**The core principle (Agency Bible §5c.1) — one action library, two drivers:** each verb is defined **once**
with one authoritative world-effect, callable by **either** a player designation→colonist job (B4/B5/B6)
**or** an NPC's own drive. Same verb, same effect, different reason. **Do not** build NPC actions as a
separate system from colonist work — that's two divergent codebases; unify them.
**Depends on:** B4/B5 (the colonist job/work-execution path these verbs share), B-AG1 (agents express intent),
B-AG3 (minds/professions produce drives). Reuses §2a primitives heavily.
**Touches:** a new shared `bastion_actions` module (`common`/`server`) that both the job system and NPC AI
call; the terrain-edit + item/loot paths (gathering/construction); Veloren's recipe/crafting + town crafting
stations (production); `rtsim/src/rule/npc_ai.rs` (NPC drives → self-designated tasks).
**Approach (dependency-ordered per Agency Bible §5c.2):**
1. **Gathering** first (foundation): chop/mine/forage/hunt/fish as library verbs, reusing terrain edit +
   block/sprite→loot. Both colonists (B5) and NPCs call these.
2. **Construction:** build houses/walls/roads via `MakeBlock`/`MakeVolume` + the blueprint→build flow.
3. **Production/crafting:** workshops, farming (plant→tend→harvest), cooking, smithing via the item/recipe
   system + crafting stations. Consumes gathered/grown inputs.
4. **NPC drives (Agency Bible §5c.5):** an NPC's profession + mind + needs + site context produce a drive
   that **self-designates** a task into the same job/task machinery colonists use. Tendency-first: no
   resource → relocate/idle-differently, never freeze.
**Done-when:**
- Each verb family (gather→build→produce) works when invoked by a **player designation** AND by an **NPC
  drive**, through the **same** `bastion_actions` path (verified both ways).
- A woodcutter NPC autonomously fells trees near its village and the logs enter the economy; a builder NPC
  raises a structure; a farmer tends a field — all with **zero player input**.
- Conservation holds (no item dupe/loss); graceful failure holds (no resources → no freeze); Tier-1b soak
  shows NPCs autonomously producing/building over time.
- Colonist work (B4/B5/B6) still uses the same library — no divergence between colonist and NPC action code.

### B-AG6 — Generative systems: autonomous settlement growth + reproduction & genealogy (the world grows)
**Objective:** Close the loop that makes the world **self-sustaining and generative** — villages **grow and
build on their own**, and populations **reproduce** with **deep DF-style kinship, families, and inherited
traits**. Without this, every population only declines; with it, the world lives forward: dynasties form,
factions grow, herds swell, the map's story accrues.
**Depends on:** B-AG5 (the build/gather verbs growth uses), B-AG3 (the mind whose traits are inherited +
relationships that form pairings), rtsim repopulation queue + wildlife-population + site-generation (extend,
don't reinvent). The capstone of the agency track.
**Touches:** rtsim site-growth + repopulation rules; a `bastion` kinship/genealogy model (`common`, serde,
persisted in rtsim's flat tables); B-AG3 mind (trait inheritance); B-AG4 inspector (relationships/ancestry).
**Approach (LOD-critical — Agency Bible §5c.3/§5c.4):**
- **Autonomous settlement growth:** villages expand at the **simulated tier** as low-res rtsim **site-growth
  events** (population↑/resources↑ → site gains a structure); when **loaded**, growth manifests as **actual
  placed voxels** via B-AG5 build verbs. Reconcile at the boundary (rtsim "grew a house" ↔ real geometry).
  **Never** place blocks for hundreds of offscreen villages every tick (gotcha #1).
- **Reproduction:** humanoids form pairings via mind relationships (§5b lovers/spouses) and have **children**
  who inherit a blend of parent **personality facets + values + traits** (B-AG3) and join the **kin graph**;
  animals breed when population+food allow (feeds ecosystem equilibrium). Build on rtsim's repopulation queue
  (delayed, not instant).
- **Genealogy LOD:** **full kin graphs / family trees / trait inheritance** run **full-res for tracked/loaded
  lineages**; **distant bloodlines persist as compact ancestry records** (parent links + key traits),
  promotable on inspect. Delivers "grandson of the founder who slew the wyvern" without holding every tree in
  memory always.
- **Ties in:** kinship surfaces in the B-AG4 inspector; a parent's death creates grief thoughts + persistent
  memory (B-AG3); births feed the population dynamics of B-AG2's ecosystem.
**Done-when:**
- **Settlement growth:** over a Tier-1b soak, NPC villages **grow** (population + structures) autonomously —
  as rtsim events when unwatched, as real construction when loaded — without per-tick offscreen block placement
  (assert bounded cost).
- **Reproduction:** colonists/NPCs pair and have children who **inherit** blended personality/values/traits
  (verify inheritance in the child's mind); animals breed; populations **sustain or grow** rather than only
  declining (Tier-1b: no extinction-to-zero, no runaway).
- **Genealogy:** the kin graph forms; a multi-generation lineage is inspectable (B-AG4), with distant
  ancestry as compact records that promote on inspect; a founder's descendants are traceable.
- LOD holds (full-res only for tracked/loaded lineages); grief/memory ties fire on kin death; all serde-ready;
  Tier-1b performant with thousands of NPCs breeding/growing.

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
  4. voxygen switches the **InputContext to `Avatar`** (built in B1.5) — a single atomic context swap to the vanilla third-person (default) / first-person camera and **exactly vanilla Veloren bindings** (movement/combat/interaction/hotbar). It now plays like stock singleplayer for that one entity. Release swaps the context back to `Overseer`. No per-key juggling — the context system handles the whole scheme change.
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
