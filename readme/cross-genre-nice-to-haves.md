# Project Bastion — Cross-Genre "Nice-to-Have" Feature Options

**Status: OPTIONAL / future development.** None of this is load-bearing or queued. This is a curated menu of
features borrowed from other genres (god games, RimWorld, city-builders, RTS, Minecraft), **filtered for one
test:** *does it amplify "autonomous god-sim over a living voxel world," or dilute it?* Adopt the ones that
make the depth **legible, paced, and consequential**; avoid the ones that pull toward another genre's
identity. Everything here is reinterpreted through Pillar §1a (influence, not command) and obeys the
loaded↔simulated LOD law.

**Context (why the filter matters):** research shows the *deep-sim, 3D, DF-souled god game* niche is
essentially empty — the genre's survivors are mostly 2D or shallow. Bastion's moat is **simulation depth**.
So borrow features that *widen that moat*, not ones that make Bastion the fourth WorldBox.

Tags: **ADOPT** (strong fit, worth doing) · **ADAPT** (fits with reframing) · **AVOID** (dilutes identity).

---

## The top 5 (highest amplification-per-effort — if you only ever do a few)

1. **Director / Storyteller (from RimWorld) — ADOPT. The single highest-value borrow.**
   RimWorld's genius isn't colonists — it's the **AI Storyteller** that *paces* dramatic events (raids,
   disasters, opportunities) based on how you're doing, to keep the story tense rather than random. For an
   *autonomous* god game, a director that decides *when* to send a raid / drought / migrant wave / rival-god
   provocation — tuned to keep the world **eventful but not doomed** — is exactly what defeats the "stable
   but boring" failure mode the Tier-1b soak guards against. It turns "the colony survives untended" into
   "the colony survives untended *and something interesting keeps happening.*" **God-game reframe:** the
   **rival gods (Divine Politics) ARE the storytellers** — their scheming *is* the event pacing. Design as a
   god-flavored director. *This is the thing that makes the autonomous world worth watching.*

2. **Over-godding penalty (from Reus) — ADAPT. Makes restraint a strategy.**
   Reus's core tension: shower your people with resources and they turn greedy/violent; keep them a little
   humble and challenged and they thrive. A **feedback mechanic that punishes over-intervention** is exactly
   what keeps a god game from being a hollow power-fantasy sandbox. **Reframe into the faith economy:** a
   people made too comfortable grow **decadent/faithless**; a people who face *managed* hardship stay
   **devout**. Divine *restraint* becomes a real, rewarded strategy. Folds into the Divine Politics faith
   loop.

3. **Mind/trait-modifier god-powers (from WorldBox) — ADAPT. Your B13 power set, but deeper.**
   WorldBox's powers are "apply a trait and watch it ripple" — Bless, Curse, Madness, Plague, Inspiration.
   Because Bastion creatures have **real minds + mind-LOD**, these are almost free to add and *far* deeper
   than the genre standard: "Madness" isn't a flag, it's a real mood/mind override that **spreads through the
   relationship graph**; "Plague" rides the same systems. Adopt the *pattern* (powers = mind/trait
   modifiers); skip the silly ones (UFOs, Crabzilla) unless you want them in **Free mode**. This is B13.

4. **Terrain raise/lower (from Populous) — ADAPT. The most iconic missing god-verb.**
   Populous's founding mechanic is raising/lowering land to shape where people can settle. You already have
   the primitive (`MakeVolume`/block edit). "Raise a hill / lower a valley / flatten ground for my people"
   is the most *recognizable* god power and it's cheap given the substrate. Marquee B13 power.

5. **Alerts + jump-to-event & legibility overlays (from RTS + city-builders) — ADOPT. How a god keeps up.**
   RTS excels at *drawing attention* across a big map ("under attack!" + click to jump). City-builders excel
   at *heatmap overlays* (coverage, happiness, crime). An autonomous god watching a self-running world needs
   both: an **alert system with camera-jump** (raid begins / colonist dying / rival god acted — wire to
   B1.8's map fly-to) and **overlays** (faith / mood / needs-coverage on the god map, reusing the occlusion/
   overlay tech). This is how the invisible sim becomes *legible* — a chronic god-game weakness Bastion can
   just solve. Fold into B9.

---

## The rest, by genre

### God games (Populous, Black & White, From Dust, Reus, WorldBox, Universim)
- **ADOPT — Creation/destruction disasters** (meteor, storm, plague, flood): the wrath half of the loop. You
  have Explosion/Lightning/WeatherZone; ledger has the rest. Make overuse of wrath *cost* faith/loyalty (ties
  Reus penalty).
- **ADAPT — "Inspire rebellion / sow discord"** (WorldBox players blast villages with unrest to start wars):
  a diplomacy-tier god-power. Fits Divine Politics DP5 (incite holy war, split a faction).
- **ADOPT (as a design value) — "run it and watch"**: WorldBox players timelapse the world at max speed and
  watch what emerges. That's the **soak test as entertainment.** Consider an explicit **observe/timelapse
  mode** — the watchable-while-untended world *is* the product.
- **AVOID — Universim-style tech tree / research progression**: pulls toward management/4X. Bastion's
  "progression" is the world's *history and faith*, not a civ research tree. Don't bolt it on.

### RimWorld
- **ADOPT — Social event feed**: surface B-AG3 relationships as a readable log ("X and Y became lovers," "A
  holds a grudge against B"). Nearly free given the substrate; enormously charming. Pairs with DF-LOG/
  Chronicle.
- **ALREADY HAVE — mood/tantrum spiral** (B7 + B-AG3), **work-priority grid** (B3/B4).

### City-builders (Cities: Skylines, Banished, Micropolis)
- **ADOPT — legibility overlays** (covered in top-5 #5).
- **ALREADY HAVE (concept) — zoning as designation** (DF-ZONES).
- **AVOID — direct infrastructure micromanagement** (draw roads/pipes/traffic): management texture that
  fights god-detachment. Colonists build their *own* roads (B-AG6 settlement growth); the god doesn't draw
  them.

### RTS (OpenRA, StarCraft lineage)
- **ADOPT — alerts / jump-to-event** (covered in top-5 #5).
- **AVOID (core) — unit micro, base-building-as-player, APM**: the whole design is explicitly *not*
  StarCraft. Already fenced off; keep it that way.

### Minecraft
- **AVOID (mostly)** — first-person creative free-building is the opposite axis from a god-sim, and Veloren's
  own devs rejected free-building for world coherence. The joy of watching a structure rise block-by-block
  you *already get* when a colonist builds in loaded view.
- **ADAPT (eventual) — redstone-style logic → DF mechanisms**: player-designated trigger→link→effect logic
  (already the DF-MECH cluster). A fun late toy; fits the god-designation model.

---

## Traps — the specific case (see also DF Gap Ledger §E)
- Veloren has **no player-buildable trap system**, but its **dungeons likely contain authored hazard
  content** (spike pits, fire jets, triggered damage). **If those hazards exist as reusable placeable/
  scriptable objects, the *effect* half of a trap is already built** — you'd harvest the hazard entity
  rather than write spike-damage from scratch. What remains to build: the **player-designation + trigger +
  wiring** half (place → link to plate/lever → fire autonomously on raiders).
- **Action item before the DF-TRAP design pass:** grep the dungeon/site-generation assets to confirm whether
  those hazards are **reusable objects** (cheap — harvest them) or **baked-in geometry** (build from
  scratch). This meaningfully changes DF-TRAP's cost.
- **God-game reframe (from ledger §E):** traps are *pre-placed policy* — designate in peacetime, colonists
  build+wire, they trigger **autonomously** on raiders (B8). Cleaner for the pillar than real-time trap
  control.
- **Build once:** traps + mechanisms + operable terrain share the **trigger→link→effect** engine (DF-MECH).

---

## The unifying principle (why this list is shaped the way it is)
Bastion's advantage is **depth of simulation.** The borrows that fit make that depth **legible** (overlays,
alerts, social feed), **paced** (the director), **consequential** (over-godding penalty, mind-powers), and
**shapeable** (terrain verbs). The ones to avoid (tech trees, free-building, unit micro, infrastructure
management) each pull toward *another* genre's identity and blur the one thing no other god game currently is:
a beautiful, deep, 3D, DF-souled living world you preside over. **Deepen into the niche; don't wander out of
it.**

*End — optional menu. Promote any item to a real design pass + block when the core game is proven and you
want it.*
