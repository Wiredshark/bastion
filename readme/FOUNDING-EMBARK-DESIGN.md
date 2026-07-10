# Project Bastion — Founding / Embark Design v0.1

**The design corpus for the new-game arc: how a player goes from "empty world" to "my colony exists here."**
Companion to the main build report (B11), the future-work ledger (§3n), and the control-spectrum canon (§3q/§3d).
This document is the **B11 design pass that §3n explicitly flags for** ("Flag for B11's design pass: it's an
integration of site-suitability (surfaced) + B3 spawning + B1.8 survey camera + the embark UI"). It consolidates
and expands the B11 sketch and §3n; it **contradicts neither** — both are compatible sketches this fills in.

**Isolation note:** this is a NEW design doc. It appends to the corpus; it rewrites nothing. Where it goes beyond
B11's build-report Done-when, treat this as the fuller intent §3n asked for, not a correction.

---

## 0. The one thing to get right first — this is a *god*-game embark, not a pawn-select

Every colony-sim we steal from puts the player *inside* the colony: in RimWorld you **are** the colonists' will,
in DF you compose and command seven dwarves, in Banished you are the village's hand. **Bastion inverts this**
(Pillar §1a, divine-politics-bible §7): the player is a **god above** an autonomous colony of NPCs with real
minds (agency-bible §5b). So the embark cannot be "spec your pawns and drive them." It must be **"choose where
your people settle, who they are as a founding flock, and under what conditions — then let them live."**

That inversion decides every design call below:
- **We steal the *information* and *pacing* of colony-sim embark** (the site readout, the scenario presets, the
  vulnerable-first-year arc) — those are genre-agnostic and excellent.
- **We reject the *micro-authorship*** (hand-editing each pawn's skills/backstory, buying items by the point).
  A god sets a flock's *character*, not each soul's stat line. Individual depth still exists (minds are real),
  but it's **surfaced for inspection, not authored by the player** — the DF-manage layer lets you *look and
  re-roll*, it never demands you build seven people by hand.
- **The founding act itself is a divine act** — the very first thing on the God-Powers menu is arguably "settle
  my people *here*" (Populous' whole fantasy). Embark is where the god game *begins*, so it must feel like a
  god choosing, not a manager provisioning.

This is the north star for the whole flow: **surface everything, author little, command nothing.**

---

## 1. What the canon does (steal table)

| Game | Site selection | Band / loadout | Starting conditions | What Bastion steals | What Bastion rejects |
|---|---|---|---|---|---|
| **DF embark** | World map → region tile → local map preview (biome, surroundings, neighbors, aquifer/evil warnings) | 7 dwarves + a **point-buy** pool for skills, items, animals; embark profiles savable | Civ choice, neighbor proximity, savagery/evil of the tile | The **local-preview-before-commit**, the **savable profile**, warnings surfaced pre-commit | Point-buy micro; hand-assigning each dwarf's skills |
| **RimWorld** | World globe → tile with a rich **readout** (biome, terrain, growing period, rainfall, stone types, river, road, temperature band) | **Scenario** (Crashlanded / Rich Explorer / Lost Tribe / Naked Brutality) sets pawn count + gear; pawns **re-rollable** | **Storyteller + difficulty** (raid cadence, event intensity) chosen separately from scenario | The **tile readout panel**, **scenario presets**, **difficulty decoupled from scenario**, **re-roll not hand-edit** | Being the pawns' will; pausing/commanding directly |
| **Banished** | (no site pick) | Fixed starting families | **Easy/Medium/Hard** = family count + resources + starting season | **Difficulty = starting-generosity dial** (band size, resources, season), the cleanest difficulty model | No site choice at all |
| **Frostpunk / Against the Storm** | Scenario-fixed or node-pick on a campaign map | Scenario-set band + resources | Scenario *is* the difficulty + narrative frame | **Scenario-as-framed-start** (a start has a *premise*, not just numbers) | Linear campaign gating |
| **Populous / From Dust / B&W** | God **shapes** the land to make a site viable, then **guides** followers to it | The followers are a flock, not units | The land's hostility is the difficulty | The **god-founding fantasy**: raise land / guide the flock / the site is something you *make* good | — (this is the frame we adopt) |

**The synthesis:** RimWorld's *readout + scenario + decoupled difficulty*, DF's *preview-and-warn before commit +
savable profile*, Banished's *difficulty-as-generosity-dial*, wrapped in Populous' *god-founding frame*. Site
choice matters (DF/RW), the band is set at flock-character level not pawn level (god frame), difficulty is a
clean generosity+pressure dial (Banished/RW storyteller).

---

## 2. The arc (the real flow)

Six stages. §3n lists five; this splits "the founding band + starting conditions" into distinct authored stages
and adds the god-framing/embodiment fork at the end. Each stage names the pieces it reuses.

```
  [1] NEW GAME          →  choose to found (god) or arrive (mortal-RPG); pick/generate world
        │
  [2] WORLD SURVEY      →  B1.8 fly-to camera over the worldgen map; roam, read regions
        │
  [3] SITE SELECTION    →  suitability SURFACED (§3 below) — the same score the AI uses, shown to you
        │                   pick an empty site; optional god-terraform to improve it (Populous)
  [4] FOUNDING BAND     →  set the flock: size, character, professions, starting faith/devotion
        │                   (inspect/re-roll individuals — optional depth, never mandatory)
  [5] STARTING          →  scenario premise + difficulty: resources/tools, raid cadence, season,
      CONDITIONS            site harshness, starting favor, rival-god contest on this site
        │
  [6] DROP-IN           →  generate local chunks, spawn founders + stockpile at the chosen site,
                            hand to the overseer view (or descend into a body — embodiment fork)
```

### Stage 1 — New game / world
- **World:** generate fresh from a seed or pick an existing save (reuse Veloren worldgen; determinism holds at
  worldgen per the build-report §7 note — the *starting* world is reproducible from a seed, so an embark seed is
  a shareable "start").
- **The founding-vs-arrival fork (embodiment spectrum §3h):** the very first choice frames the whole game.
  - **Found (god mode, default):** you will choose a site and settle a fresh flock. Stages 2–6 as written.
  - **Arrive (mortal-RPG capstone, Mode B §3h):** there is no player-god; you drop into an *already-young*
    colony (autonomously founded by rtsim) as a mortal. Skips stages 3–5's authoring; the world already chose.
    Correctly **last** to build (§3h) — flag as a Stage-1 branch, not a v1 requirement.
- **B3's found-in-existing-town** remains selectable as the **proven placeholder start** until Stage 3 exists —
  ship it as "quick start (existing town)" so there's always a working entry while the real embark lands.

### Stage 2 — World survey (the B1.8 camera, pointed at worldgen)
- Reuse **B1.8's map/fly-to** and the **terrain-height sampler** (§3n; the same sampler doing camera
  surface-follow, ground-follow overlays, carrying-capacity, and now suitability — "one sampler, many uses").
- The god **roams the world map** — worldgen map render (reuse Veloren's), fly to a region, drop toward the
  surface to inspect. This is the god surveying creation before choosing. It's *reconnaissance*, not yet
  commitment. Legends/history preview of nearby factions is a stretch (B11 build-report already flags it optional).

### Stage 3 — Site selection (the heart — see §3 for the UX)
- Pick an **empty site** (not an existing town — that's the whole point of §3n vs B3).
- **Suitability is surfaced** here: the same score the autonomous-settlement system uses to place AI colonies is
  shown to the player as they hover/consider candidate sites. Detailed in §3.
- **Optional god-terraform (Populous):** before committing, the god may reshape the site to improve it — raise a
  flood-prone basin, carve a channel to water, flatten a build pad. This is a **preview of B13 terrain powers**
  used at embark; it makes "the site is something you *make* good" real and ties embark to the god-powers layer
  from the first minute. Gate behind starting-favor so it isn't free omnipotence (see §4).

### Stage 4 — The founding band (flock composition, god-framed)
See §4 for the full loadout model. The band is set at **flock-character level** (size, professions mix, faith/
devotion, a scenario-implied personality tilt), with **individual inspect + re-roll** as optional DF-manage depth.

### Stage 5 — Starting conditions & difficulty
See §5. Scenario premise + a clean difficulty dial (generosity + pressure), plus the god-game-specific starts:
**starting favor** and **whether a rival god contests this site** (divine-politics hook).

### Stage 6 — Drop-in
- Generate local chunks at the chosen site; **spawn the founding band as fresh entities at the selected empty
  location** (a modification of B3's spawn logic — spawn founders at a point, don't promote town NPCs — exactly
  as §3n specifies). Place the starting stockpile. Hand to the **overseer view** (or, on the embodiment fork,
  drop into a body via B12).
- The vulnerable first days begin: too few hands, no walls, night coming — the colony-sim's core hook, now
  actually reachable because you started from nothing.

---

## 3. Suitability surfacing — the UX crown (the one genuinely novel piece)

**The principle (§3n, load-bearing):** *autonomous-placement and player-founding are the SAME scoring — one used
by the AI, one shown to you. Build the scorer once.* The AI already needs a site-suitability function to place
its own colonies and bound growth (carrying-capacity, §3q). Embark **surfaces that exact function**. This is the
integration seam: don't build a separate "embark score."

### 3.1 What the score is made of (reuse the existing sampler)
The suitability score is a weighted composite over the candidate site's neighborhood, sampled with **B1.8's
terrain-height sampler** and rtsim/world data:
- **Flatness / buildability** — how much terrain-prep a settlement needs (the sampler's core output; low prep =
  high score, ties §3q "prefer low-prep sites").
- **Water proximity** — distance to fresh water (farming, drinking).
- **Resource access** — nearby ore density (the "ore survey" sampler pointed down, §3n), wood (forest cover),
  arable land, stone.
- **Biome / climate** — growing period, temperature band, rainfall (RimWorld's readout, from Veloren's temp
  field + calendar/season).
- **Threat proximity** — nearby monster lairs, hostile factions, raider bands (rtsim data — the raid-source
  distance).
- **Culture-fit** — does the biome/region suit the founding people's origin (a desert people founding in tundra
  scores lower; ties §3n "culture-fit" and the race-keyed asset pools).
- **Hazard flags** — flood-basin, aquifer, avalanche/landslide risk (ties §3y hazard engine + "this basin floods"
  from the flood-drainage note at §3n).

### 3.2 How to present it (three legibility layers, cheap→rich)
Surface the **same numbers the AI sees**, translated for a human. Three layers, each shippable independently:

1. **The verdict line (cheapest, ship first).** A one-line human summary on hover: *"Flat, near water, near ore —
   good site."* / *"Rich soil but a raider camp is close."* / *"This basin floods."* Direct from §3n. This is the
   MVP — a single sentence generated from the top-contributing score terms.
2. **The readout panel (RimWorld tier).** A small panel breaking the score into its factors with a bar/rating
   each (Buildability ●●●○○, Water ●●●●○, Ore ●●○○○, Threat ▲ close, Climate: temperate/long growing season,
   Hazards: flood-risk). Shows *why* the verdict says what it says.
3. **The suitability heat-overlay (god-tier, best).** A **top-down heatmap** painted over the survey map — green
   where the AI would want to build, red where it wouldn't — so the god *sees* the good ground at a glance, the
   way a god should. This is the same overlay the AI's placement reasoning would visualize; it doubles as a debug
   view for the autonomous placer. Toggle per-factor (show just water, just threat) reusing the toggleable-overlay
   pattern (§3n/design-doc overlays).

### 3.3 Honesty rules (don't lie to the player, don't trivialize the choice)
- **Surface uncertainty, don't hide it.** rtsim tends toward equilibrium and can't promise the future (agency-
  bible §0). A site "near a raider camp" is a *tendency* to danger, not a scripted doom. The readout should read
  as **assessment, not prophecy** ("raiders nearby — expect pressure" not "you will be raided on day 3").
- **No single "correct" site.** The score has trade-offs by design (safe-but-poor vs rich-but-exposed); the god
  chooses a *character* of start, not a right answer. Difficulty (§5) can weight this (a hard start offers only
  marginal sites).
- **The score is the AI's real reasoning, not a facade.** If the surfaced verdict and the AI's placement ever
  diverge, that's a bug in the shared scorer — the whole point is they're the same function. This is a built-in
  correctness check.

---

## 4. The embark-loadout model (the founding band, god-framed)

**Design stance:** you set the **flock**, not the pawns. Three control-spectrum depths (§3q applied to embark),
autonomous-by-default:

- **Preset (autonomous / default):** pick a **scenario** (§5) and it fully specifies the band — count, profession
  mix, starting faith, gear. One click, you're founding. This is the soul; most players never go deeper.
- **Shape (DF-manage / optional):** adjust the flock's **character** at policy level, not per-person:
  band **size** (3 devout founders vs 12 mixed pioneers), **profession mix** (weight toward farmers / crafters /
  guards / a balanced spread), **origin culture** (sets biome culture-fit + asset palette + starting values
  bias), **starting devotion** (a fervent flock vs a wavering one — matters immediately for god-powers, §GPC).
- **Inspect & re-roll (DF-manage depth, never mandatory):** open any individual founder and **see their mind**
  (agency-bible §5b: facets, values, needs, a starting relationship or two) and **re-roll** if you dislike the
  draw (RimWorld's model, not DF's point-buy). You **never author** a mind by hand — you accept or re-roll what
  the generator produced. This keeps founders as *real autonomous individuals* (the whole agency thesis) while
  giving the player the RimWorld satisfaction of a band they chose.

### 4.1 What a founding band carries
- **The founders themselves** — N fresh humanoid colonists (B3 spawn, at the chosen site), each with a full mind
  (§5b), a profession, and a **starting faith state = devout to *you*** (your home flock, divine-politics-bible
  §4 — "worships you natively"). Devotion level is a scenario/shape input.
- **Starting inventory / tools** — RON scenario config (B11 build-report already specifies "starting-inventory
  config" matching Veloren's data conventions): tools, seeds, a few days of food, building material stand-in
  (until B6 hauling supersedes the B5 `BUILD_MATERIAL_ITEM`, per the backlog).
- **Starting stockpile** — placed at drop-in (Stage 6).
- **Starting favor** — the god's opening divine-power budget (see §4.2).
- **Optional starting structure** — a scenario may seed a single starter (a shrine, a campfire, a cart) — ties
  §3h's "found a shrine at the falls" and gives the flock an anchor.

### 4.2 The god's own starting conditions (the god-game-specific loadout)
Unique to Bastion — the *god* has a loadout too:
- **Starting favor** — how much divine power you begin with (gates the Stage-3 terraform and early god-powers).
  A harder scenario starts you favor-poor; the Populous "raise the land to save them" fantasy costs favor, so
  spending it at embark means less for the first crisis — a real opening decision.
- **Starting worship / favor-regen** — favor accrues from the colony thriving/worshipping (B13). A devout flock
  (set in §4/Shape) means faster early favor. Devotion is thus both a colony stat and your power supply.
- **Rival-god contest (divine-politics hook, DP-tier / later):** a scenario may place your embark **inside a
  rival god's sphere** — a nearby faction already worships another deity, contesting this region's faith. Off by
  default (needs DP4 rival gods); flagged so the embark data model *reserves the slot* now.

---

## 5. Starting conditions & difficulty

Decouple **scenario** (the premise/frame) from **difficulty** (the generosity+pressure dial) — RimWorld's clean
split, and it maps perfectly onto Bastion's autonomy pillar.

### 5.1 Scenario presets (the premise — sets the band + a narrative frame)
Each is a RON config (B11) specifying band size, profession mix, starting devotion, gear, starting favor, and a
one-line premise. Starter set (steal the RimWorld archetypes, re-skinned for a god game):
- **The Faithful Few** *(≈ Crashlanded / default)* — a small balanced flock of devout founders, modest gear.
  The canonical "watch it grow from nothing" start.
- **The Pilgrimage** *(≈ Lost Tribe)* — a larger flock, primitive gear, high devotion. More hands, less tech —
  a people arriving to settle in your name.
- **The Chosen One** *(≈ Rich Explorer)* — very few founders (2–3), good gear, high starting favor. A god
  lavishing power on a tiny seed. Hard by band-size, easy by god-power.
- **The Faithless Frontier** *(≈ Naked Brutality)* — one or two founders, nothing, low devotion, near-zero
  favor. The expert start: prove the colony *and* earn the faith.

Scenarios are **savable/editable profiles** (DF's embark-profile idea) — the "Shape" layer (§4) is really just
editing a scenario in place.

### 5.2 Difficulty dial (generosity + pressure — Banished's model, RimWorld's decoupling)
Two independent axes, both **data, not code** (the build-report's tuning-config discipline, §7 point 12):
- **Generosity (starting slack):** starting resource quantity, band size bonus, starting favor, site quality
  offered (easy = suitable sites plentiful; hard = only marginal ground). Banished's family-count-and-resources
  dial, generalized.
- **Pressure (world hostility):** raid cadence (B8/B11's existing knob), threat proximity weighting in site
  offerings, hazard-event frequency (§3y), season harshness at start (start in winter = hard). Ties directly to
  the existing raid-cadence config the B11 build-report already names.
- **Storyteller analog (later):** the pressure axis can grow into a RimWorld-storyteller pacing model (curated
  event cadence vs pure random) — flag as future, not v1.

### 5.3 The guardrail (Pillar §1a — difficulty must not break autonomy)
Even the **hardest** start must remain a **self-running world** — the Tier-1b zero-input soak must still pass at
max difficulty (the game never *requires* god input, build-report §7). Difficulty makes the colony's odds worse;
it must never make the colony *unable to act on its own*. A hard start is a colony likely to *fail autonomously*,
not one that *freezes waiting for the player*. Test every difficulty preset against Tier-1b.

---

## 6. God vs mortal — how embark reads across the embodiment spectrum (§3h)

The same embark machinery serves all five embodiment lenses; the arc above is the **god-founding** case. Note the
others so the data model doesn't have to be rebuilt for them:
- **God (Autonomous/Manage/Command):** the full arc (§2). The god surveys, chooses, shapes, and settles. The
  control-spectrum depth (§4) is *which* embark stages the player leans into.
- **God-embodied (Mode A, §3h):** identical embark, but Stage 6 drops the god **into a body** (B12) at the site
  instead of the overseer view — you walk your own founding. Same flow, different drop-in camera.
- **Mortal-RPG (Mode B, §3h — capstone, last):** the **Arrive** fork (Stage 1). No site authoring — an rtsim
  colony founded *itself* (autonomous settlement growth, §5c of agency-bible), and you drop in as a mortal
  resident. Embark degenerates to "pick which young colony / where in the world to spawn." This is why the
  autonomous placer and the surfaced scorer must be the same function (§3): in Mode B the *AI* does the founding
  the player does in god mode, using the identical machinery.

**Consequence for build order:** build the god-founding arc (§2) on the shared scorer, and Mode B's arrival
becomes mostly free — it's the same worldgen + placement + drop-in with the authoring stages skipped.

---

## 7. Sequenced build slice for B11

Decomposed so each sub-block has an independent Done-when and ships value alone. Ordered by dependency and by
"working entry point at every step." The through-line: **build the shared suitability scorer first (it's owed to
the AI anyway), surface it, then wire spawning and the screen.**

- **B11.0 — Founding-spawn primitive (the unblock).** Modify B3's spawn to **spawn a fresh founding band at an
  arbitrary empty world location** (not by promoting town NPCs). Headless-testable, no UI.
  *Done-when:* the harness spawns N fresh colonists + a stockpile at a given coordinate on empty ground; they
  have full minds and start devout; entity-count/conservation invariants hold; the existing found-in-town path
  still works as the placeholder.
- **B11.1 — The shared suitability scorer.** Build the site-suitability function (§3.1) as a **shared module the
  autonomous placer and embark both call** (the §3n "build once" seam). Pure function over world/rtsim data +
  the B1.8 sampler; no UI yet.
  *Done-when:* given a site, returns a factored score + top-contributor terms; the autonomous-settlement placer
  uses it to place AI colonies; a headless test shows AI placement and a queried "embark verdict" agree for the
  same site (the built-in correctness check, §3.3).
- **B11.2 — Suitability surfacing UX.** The verdict line → readout panel → heat-overlay (§3.2), built in that
  order; ship after the verdict line alone works.
  *Done-when:* surveying the map, hovering a site shows the verdict line and readout; the heat-overlay paints
  good/bad ground; numbers match B11.1's scorer exactly.
- **B11.3 — Survey camera over worldgen.** Wire B1.8's fly-to/map camera to the worldgen map for Stage 2 roaming
  and Stage 3 site-picking (reuse; mostly integration).
  *Done-when:* new-game → survey the worldgen map, fly to regions, pick a site coordinate that feeds B11.0/B11.1.
- **B11.4 — Scenario + difficulty config.** RON scenario presets (§5.1) + the two-axis difficulty dial (§5.2) as
  tunable data. Wires band composition + starting inventory/favor + raid cadence.
  *Done-when:* selecting a scenario+difficulty yields materially different starts (band size, gear, favor, raid
  cadence, site-quality offered differ as configured); all presets pass the Tier-1b soak (§5.3).
- **B11.5 — The embark screen.** The UI tying Stages 1–6: new game → (found/arrive fork) → survey/pick site
  (suitability shown) → scenario+difficulty → confirm band → drop-in. Replaces character creation (flag vanilla
  creation off, per B11 build-report), with the "quick start (existing town)" placeholder always available.
  *Done-when:* B11 build-report's Done-when — "new game → map → pick site → land with colony + starting resources
  in the top-down view, no hero/character-creation step," and "different sites yield materially different starts."
- **B11.6 — Founding-band inspect & re-roll (optional depth).** The DF-manage layer (§4): open a founder, see
  their mind (needs B-AG3/B-AG4), re-roll. Ships after minds+inspector exist.
  *Done-when:* a founder's mind is inspectable pre-drop; re-roll produces a new valid mind; the player never has
  to touch it to found (autonomous default preserved).
- **B11.7 — Embark terraform (Populous preview, optional).** Allow Stage-3 god-terraform of the chosen site,
  favor-costed. Depends on B13 terrain powers.
  *Done-when:* the god can raise/flatten/carve the candidate site at embark, spending starting favor, and the
  suitability readout updates to reflect the change.
- **B11.8 — Mortal-arrival fork (capstone, last).** The Stage-1 "Arrive" branch (§6): drop into an rtsim-founded
  young colony as a mortal. Depends on autonomous settlement growth (agency-bible §5c) + B12 + the mortal-RPG
  readiness of minds/dialogue (§3h — correctly last).
  *Done-when:* new-game → arrive → spawn as a mortal in a colony the AI founded via the same scorer; no authoring
  stages; the world was already alive when you arrived.

**Dependency spine:** B11.0 + B11.1 (independent, build first) → B11.2/B11.3 (surface + survey) → B11.4/B11.5
(config + screen = the shippable v1 embark) → B11.6/B11.7 (optional depth) → B11.8 (capstone).
**v1 = B11.0–B11.5.** Everything after is enrichment over a working god-founding.

---

## 8. Open questions / dependencies / flags

- **Suitability weights are tuning data, not code** (§7-point-12 discipline). The composite weights (§3.1) must be
  RON-tunable so "what makes a good site" is balance, not a recompile. Owed to both the AI and embark.
- **Culture-fit needs the race-keyed asset/culture data** (§3n, asset pools) to be meaningful — until then,
  culture-fit is a stubbed constant. Flag, don't block.
- **Rival-god-contest embark (§4.2)** reserves a data slot now but can't function until DP4 (rival gods). Model
  it, gate it off.
- **Starting-material stand-in:** the founding band's build material rides the B5 `BUILD_MATERIAL_ITEM` stand-in
  until B6 hauling supersedes it (backlog) — embark scenario config should express materials in a form that
  survives that swap.
- **Determinism:** worldgen is deterministic from seed (build-report §7), so an **embark seed is a shareable
  start** — worth exposing (RimWorld/DF both let players share seeds). rtsim diverges once ticking, which is fine.
- **No contradictions found** with B11 (build-report §6), §3n, §3q, or the bibles. This doc is the consolidation
  §3n requested; it supersedes nothing and rewrites nothing. If B11's build-report Done-when and this doc's
  sub-block Done-whens ever conflict, this doc is the fuller intent and the build-report line is the minimal gate.

*End of Founding / Embark Design v0.1 — the arc from empty world to "my people live here," god-framed.*
