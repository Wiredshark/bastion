# Project Bastion — Agency Bible v0.1

**The design corpus for DF-level NPC agency.** Companion to the main build report; grows in passes.
This document answers *what each creature/NPC is and wants*. The main doc's agency blocks (B-AG1, B-AG2)
answer *how the engine runs it*. Keep the two separate.

Goal (from the product owner): **no NPC just stands around.** Every creature, person, and monster should
have purpose, a home, relationships, and behavior with the specificity Dwarf Fortress is known for —
expressed through the systems Veloren already has (rtsim + agent AI), not bolted on.

---

## 0. The most important thing to understand first

Veloren already has most of this substrate — and it has a **hard architectural law** that dictates how
every agency spec below must be written. From Veloren's own rtsim docs:

> rtsim runs at **variable, throttled tick rates**, and its code must **assume nothing is stable**. An
> NPC's profession, position — even its *existence* (IDs invalidate) — and any two-way relationship can
> change tick to tick. "The unhappy path does matter." rtsim tends toward **equilibrium over time**, it
> does not guarantee moment-to-moment state.

**Therefore agency in Bastion is authored as *tendencies*, not scripts.** Never "this creature always does
X then Y." Always "this creature *tends* toward X; if the world permits, it does Y; if anything is missing,
it degrades gracefully." A rigid behavior tree that assumes its target still exists next tick will crash or
freeze — which is *exactly* the "standing around" bug we're trying to kill, in a new form. Write every
behavior so that losing its target, home, or faction mid-action is a normal, handled outcome.

This is the #1 rule. Everything below obeys it.

---

## 1. Why NPCs "stand around" today (the real diagnosis)

Veloren has **two NPC tiers** (main doc §2/§4):
- **Simulated** (rtsim): thousands of NPCs with home sites, factions, professions, travel, sentiment —
  alive across the whole world even unloaded. (The harness counted ~2,355 rtsim NPCs / 204 sites / 16
  factions.) rtsim NPCs already: travellers follow roads between towns, airships run routes, birds return
  to home dungeons, wyverns roam, villagers/guards carry & use potions, taverns have behaviors, and a
  sentiment/reputation system makes NPCs remember and react to the player.
- **Loaded** (ECS `agent`): physical entities near the camera.

**The "standing around" problem lives at the promotion boundary.** When a rich rtsim NPC is promoted to a
loaded entity, its high-level intent (`NpcAction`/`NpcActivity` via `RtSimController`) often collapses into
a generic idle `agent` behavior, because the loaded AI doesn't faithfully continue the rtsim plan. So the
fix is, in order:
1. **Fidelity (B-AG1):** make loaded NPCs *express the rtsim life they already have* — the trader keeps
   trading, the guard patrols, the hunter hunts — instead of idling. This fixes most of the problem for the
   *entire population* at once, before authoring a single new behavior.
2. **Depth (B-AG2):** *deepen* each archetype's agency per this Bible — richer purpose, relationships, and
   interactions — now that agents actually act.

Fidelity first. You can't enrich a corpse.

---

## 2. The Agency Schema (how every creature is described)

Every entry in this Bible fills this template. Keep each field a **tendency**, rtsim-safe.

- **Identity:** name; body type (verify against `common/src/comp/body.rs`); loaded vs. simulated cost class.
- **Purpose (drive):** the one-line "what it's for" — the goal it tends toward when nothing overrides it.
- **Home / territory:** home site or roaming range; how it returns; what happens if home is gone (degrade
  gracefully — pick a new home / wander / disband).
- **Faction / alignment:** which faction/alignment (`assets/common/entity/wild` for wild alignment); how
  sentiment shifts (Veloren already: damage→sentiment down, help downed→sentiment up).
- **Interacts with — WHO:** other creatures/NPCs/player it engages (allies, prey, predators, trade partners,
  rivals).
- **Interacts with — WHAT:** world objects/resources it uses (crafting stations, food sources, ore, water,
  nests, campfires, taverns, doors).
- **Interacts — HOW:** the verbs (trade, patrol, hunt, graze, flee, raid, build, worship, socialize, mate,
  nest, hoard, ambush). Each verb is a tendency with a graceful-failure fallback.
- **Needs / rhythm:** day/night cycle, hunger/rest analog (reuse or mirror rtsim needs), seasonal/migratory
  behavior.
- **Threat response:** fight / flee / call-for-help thresholds; courage; pack behavior.
- **Death / removal:** what its death does to the world (sentiment ripples, faction loss, repopulation via
  rtsim's repopulation queue — note: rtsim repopulates on a delayed queue, not instantly).
- **Fidelity notes:** what MUST persist across the loaded↔simulated boundary so it doesn't "reset to idle"
  when it loads. This is the B-AG1 contract for that type.

---

## 3. Flagship archetypes (authored in full — the proven template)

Four archetypes chosen to exercise the *whole* agency system. Prove these end-to-end, then mass-expand §4
using them as patterns. Ground the specifics against the repo (`body.rs`, `assets/common/entity/wild`,
`rtsim/src/rule/npc_ai.rs`) during implementation — treat named stats/creatures below as the design intent,
verify exact values in-tree.

### 3.1 Faction Humanoid — the Townsperson (villager / guard / merchant)
*The social, site-bound agent. Exercises: factions, sites, professions, trade, daily rhythm, sentiment.*
- **Purpose:** sustain and defend the settlement; perform a profession; live a daily routine.
- **Home/territory:** a specific town/site (`SiteId`). Sleeps at home, works at a station, socializes at the
  tavern. If the site is destroyed: flee, become a refugee/traveller, or join another site (degrade, don't
  freeze).
- **Faction/alignment:** the town's faction; friendly to same-faction, wary of hostiles; sentiment tracks
  player history (already in Veloren).
- **WHO:** other villagers (socialize), guards (protection), merchants/traders (commerce), the player
  (dialogue, trade, hire — Veloren supports two-way conversation, hiring, asking directions), hostiles
  (flee or, for guards, engage).
- **WHAT:** crafting stations, market stalls, the tavern (drinking/socializing — tavern behaviors exist),
  homes/beds, farms/barns, town roads.
- **HOW:** *profession loop* by role — merchant runs a stall / travels trade routes; guard patrols a beat
  and responds to threats; farmer tends fields/barns; craftsperson works a station. *Daily rhythm:* work by
  day, tavern in the evening, home at night. *Social:* chat, form relationships. All tendencies.
- **Needs/rhythm:** strong day/night structure; the routine IS the agency here.
- **Threat response:** civilians flee to safety / raise alarm; guards muster and fight (potions in hand).
- **Death:** faction sentiment ripple; town population drops; rtsim repopulation queue backfills later.
- **Fidelity (B-AG1):** on load, a townsperson must resume its *current rtsim task and daily-phase*, walking
  to the right station/tavern/home — NOT spawn idle in the street. This single fix cures most "standing
  around."

### 3.2 Predator — the Wolf (territorial pack hunter)
*The wild aggressor. Exercises: territory, hunting, packs, prey/predator graph, day/night.*
- **Purpose:** hunt to feed; hold territory; survive.
- **Home/territory:** a roaming range/den rather than a built site; returns to range; if displaced, claims
  new range.
- **Faction/alignment:** wild-hostile (verify `entity/wild` alignment); hostile to prey and to lone
  humanoids, wary of groups. ("Wolves are deadlier" per recent changes.)
- **WHO:** prey (deer, rabbits, livestock — §3.3), pack-mates (coordinate), rival predators (avoid/contest),
  humanoids (opportunistic threat).
- **WHAT:** prey animals, carcasses (feed), water, den/territory markers.
- **HOW:** *hunt* — detect prey, stalk, chase, kill, feed; *pack* — hunt in coordination, share territory;
  *patrol* territory; *rest* in den by day if nocturnal. Failure-graceful: no prey → widen range / scavenge.
- **Needs/rhythm:** hunger drives hunting cadence; more active at dawn/dusk/night.
- **Threat response:** fights when advantaged or defending den/pack; flees when outnumbered/hurt; may call
  the pack.
- **Death:** removes a predation pressure (prey populations should respond over time via rtsim); pack
  weakens.
- **Fidelity (B-AG1):** on load, a wolf mid-hunt continues the hunt / returns to territory, not idle-wander.

### 3.3 Herd Herbivore — the Deer (grazer / prey / migrator)
*The prey-base and ecosystem keystone. Exercises: herds, grazing, flight, migration, population dynamics.*
- **Purpose:** feed, breed, survive predation.
- **Home/territory:** a grazing range; seasonal migration tendency; loose herd cohesion.
- **Faction/alignment:** wild-peaceful; flees rather than fights.
- **WHO:** herd-mates (cohere, follow), predators (flee — §3.2), player (skittish).
- **WHAT:** vegetation/grass (graze), water, open terrain (flee routes), shelter.
- **HOW:** *graze* on vegetation; *herd* — stay loosely together, follow movement; *flee* predators as a
  group; *migrate* seasonally/toward resources; *breed* when population/food allow (feeds rtsim wildlife
  population tracking — a roadmap goal). Failure-graceful: lost herd → seek nearest herd / solo-graze.
- **Needs/rhythm:** graze by day; hunger/grazing cadence; seasonal movement.
- **Threat response:** flee (fast, group scatter/regroup); never fights.
- **Death:** feeds predators; population dynamics ripple (over-predation → decline → predator decline).
- **Fidelity (B-AG1):** on load, resume grazing/fleeing/herding with the herd, not stand frozen.

### 3.4 Roaming Apex / Raider — the Wyvern (and the raid pattern)
*The world-scale threat. Exercises: wide roaming, lairs, apex behavior, raids on sites, loot/hoard.*
Two sub-patterns under one archetype:
- **Roaming apex (wyvern / phoenix / roc):** *Purpose:* roam a wide range, dominate, return to lair/nest
  (birds already return to a home dungeon; wyverns already roam the world for scale-loot). *WHO:* everything
  weaker is prey/target; avoids nothing. *HOW:* patrol a large territory, hunt big prey, return to lair,
  guard hoard. *Death:* rare, high-impact; drops signature loot; frees the region.
- **Raider (adlet / gnarling / cultist band):** *Purpose:* lair-based faction that *raids* settlements.
  *Home:* a lair/cave/camp (`SiteId`). *WHO:* hostile to town factions (raid targets), loyal to own band.
  *HOW:* muster at lair → travel toward a target site → raid (fight, loot, burn) → retreat to lair with
  spoils. This is the pattern the main doc's **B8 threats** consumes: an rtsim rule sends the band toward the
  colony; they promote to loaded on arrival, use hostile agent AI, retreat + demote on defeat.
- **Fidelity (B-AG1):** a raid party mid-march continues its march on load; a returning apex continues toward
  its lair — never resets to idle at the chunk edge.

---

## 4. The expansion inventory (author these next, using §3 as templates)

The full type list is in-repo — the source of truth is `common/src/comp/body.rs` (every body) and
`assets/common/entity/wild` (wild alignments/spawns); the wiki lists ~33 NPC pages. Map each to the closest
flagship template and fill the §2 schema. A starting grouping (verify/expand against the repo):

- **Townsfolk & faction humanoids** (→ 3.1): villager, guard, merchant/trader, blacksmith, alchemist,
  farmer, traveller, tavern-goer, cultist (hostile faction variant), captain.
- **Predators** (→ 3.2): wolf, saber cat, bear, fox, hyena, cave spider, tiger, and aquatic hunters.
- **Herd/prey herbivores** (→ 3.3): deer, pig, rabbit, sheep/cattle, antelope, tortoise, and other grazers.
- **Roaming apex** (→ 3.4 apex): wyvern, phoenix, roc, frost gigas, and other world-boss-scale roamers.
- **Raider factions** (→ 3.4 raider): adlets (adlet caves), gnarlings, sahagin, cult bands, dwarven-mine
  denizens.
- **Special / critters:** rats, birds, fish, and ambient critters — even these get a *minimal* tendency
  (forage/flock/scatter) so nothing is truly inert.

For each: one schema fill-in, tagged with its flagship template and any deviations. That's the growth path —
a pass per group.

---

## 5. Governing principles (apply to every entry and every implementing block)

1. **Tendencies, not scripts** (the §0 law). Everything degrades gracefully when data is missing.
2. **Express, then deepen.** B-AG1 makes loaded NPCs honor existing rtsim intent; B-AG2 enriches. Never
   enrich before the loaded tier faithfully expresses.
3. **Author in rtsim terms.** Behaviors live in / mirror `rtsim/src/rule/npc_ai.rs` and drive via
   `NpcAction`/`NpcActivity`/`RtSimController`; the loaded `agent` reads the same intent. One brain, two
   fidelities.
4. **Ecosystem over set-pieces.** Predator/prey/faction relationships should *tend toward equilibrium*
   (populations rise/fall, factions wax/wane) — this is what makes it feel DF-alive rather than scripted.
5. **Cheap when unwatched.** Simulated-tier behavior must stay cheap (throttled ticks); rich detail is for
   the loaded tier. Don't push per-creature high-res sim into rtsim (main doc's #1 gotcha).
6. **Nothing inert.** Every body type gets at least a minimal tendency. "Standing around" is a bug, always.
7. **Player is a god, not a general.** NPC agency is *theirs*; the player influences the world they live in
   (main doc Pillar §1a). Agency depth makes indirect influence meaningful — you change conditions, they
   react.

---

## 5b. The Mind — a full Dwarf Fortress–style inner life (for every creature)

The product owner's target: **transfer DF's unit model** — thoughts, personality, values, relationships,
memory, mood — so that selecting any NPC shows a DF-style inner life, and that inner life actually *drives*
behavior. This is the crown jewel and the most entangled system in DF; author it as one connected model, not
isolated features (a "thought" is meaningless without the personality/values/memory that shape it).

### 5b.1 The causal chain (how DF actually works, mirrored)
A **thought** is not a stored string; it is the *output* of a pipeline:

> **event** (saw a corpse, ate a fine meal, slept on the ground, friend died, was forced by the god)
> **× personality** (facets: e.g. cheerful↔gloomy, brave↔anxious, forgiving↔vengeful …)
> **× values** (what this individual cares about: family, craftsmanship, nature, tradition …)
> **× memory** (recent related events; standing grudges/bonds)
> → **emotion** (joy, disgust, grief, satisfaction, rage …) of some intensity
> → contributes to **mood** (a running aggregate) → at extremes, **breakdown / tantrum / elation**.

Two dwarves with different personalities experience the *same* event as *different* thoughts. That is the
whole point — it's what makes a DF population feel like individuals rather than clones. Bastion mirrors this
chain; every field is a *tendency* (Bible §0 law) and degrades gracefully if inputs are missing.

### 5b.2 The Mind model (data) — *fact-checked against DF (see corrections)*
- **Personality facets (0–100, bell-curved around 50):** determine how a creature **acts** (bravery, anger,
  altruism, greed…). Set at creation per a species-biased distribution (DF: dwarves greedier, goblins low
  altruism — mirror with per-species medians/caps). Start ~a dozen, expand toward DF's full set.
  **Correction (DF-accurate):** facets are *mostly* stable but **significant memories can slowly shift
  facets and beliefs over time** — model slow drift from major life events, not full immutability.
- **Values / beliefs (−50 to 50):** determine what they **believe** (tradition, cooperation, romance,
  nature, sacrifice…); influenced by cultural values of their civilization (entity-level bias) + individual
  variation. Amplify/dampen specific thoughts.
- **Facet–belief CONFLICT (DF signature — do not skip):** one individual can hold a value their facets
  prevent them living ("never falls in love… and is bothered by this, since s/he sees romance as one of the
  highest ideals"). Model conflicts explicitly: a conflicting pair produces its own recurring distress
  thoughts and inspector text. This inner tension is a large part of what makes DF characters feel human.
- **Needs — two kinds (correction):**
  - *Bodily needs* (B7): hunger, rest, recreation.
  - ***Personal needs* (DF Need system):** individually-weighted wants **derived from facets/values** —
    pray/meditate (per religiosity), be with family/friends, make romance, craft, fight/argue, see animals,
    acquire things, learn… Satisfying one refreshes it; prolonged insufficiency causes distraction + bad
    thoughts.
- **FOCUS (missing system, now added):** the ratio of met:unmet personal-need weight produces a **focus**
  stat, *separate from mood*, granting up to **±50% work speed/quality**. A dwarf can be happy but
  distracted, or unhappy but focused. NPCs **self-generate need-satisfaction jobs** (low-priority ones yield
  to colony work; high-priority ones — e.g. "Pray!" — do not). This closes the loop: personality → needs →
  focus → work performance, and it's a big lever the player influences *indirectly* (build a temple, a
  tavern, a zoo — the god-game way).
- **Memory:** a decaying log of significant events + **persistent** items (grudges, bonds, trauma). Thoughts
  fade; grudges linger. Major memories can drift facets/beliefs (above).
- **Relationships (correction — make it mechanical):** per-actor sentiment/bond, but formation follows DF's
  rule: **facet similarity breeds friendship/romance; strong facet divergence (>60 vs <40) breeds grudges.**
  Compatibility is computable from the two minds — relationships emerge from personality, not randomness.
  (Builds on Veloren's sentiment: damage↓, help↑.)
- **Thoughts:** the recent emotional events produced by the pipeline, each with source + intensity + decay.
- **Mood:** running aggregate → states (content / stressed / breaking / elated). Drives behavior (breakdown
  refuses work, wanders, lashes out — the tantrum spiral). Mood and focus are **independent axes**.

### 5b.3 Mind LOD — the critical performance principle (obey this or it breaks the engine)
Running a full mind every tick for thousands of NPCs is precisely the "push high-res per-entity sim into
rtsim" mistake the main doc names as **gotcha #1**. rtsim stays alive *because* each simulated NPC is cheap.
So minds run at **level-of-detail, mirroring the loaded/simulated body split**:
- **Every creature *has* a full mind** — same data model, same inspector fields. This is the honest design
  truth.
- **Simulated tier (unwatched):** the mind runs as a **cheap summary** — dominant mood, a few key
  relationships, standing grudges — updated on rtsim's throttled ticks. History still accrues; the world
  stays coherent; grudges/bonds persist.
- **Loaded tier (near camera / selected / possessed):** the mind runs at **full resolution** — all facets,
  live thought generation, memory formation. Detail is spent where the player is looking.
- **On inspect / promotion:** selecting (or loading) an NPC **promotes its mind to full-res**, so the unit
  sheet is always fully populated when opened, and demotes back to summary when unwatched (persisting the
  durable parts). This is DF's own trick — offscreen historical figures are lower-res than your fortress.
- **Animals included:** a DF cat has thoughts. Animals get the *same* model with a simpler personality/values
  profile and fewer thought sources — not a *lesser* system, a *lighter parameterization* of the same one.

### 5b.4 It must *drive* behavior, not just display
The mind is not a cosmetic sheet. Mood gates work (a breaking colonist refuses jobs); values/personality bias
job choice and social behavior; grudges alter who helps whom in a fight; a forced action (god-power) leaves a
resentful thought. The inspector (B-AG4) *surfaces* the mind; the agency systems (B-AG1/2, B7) *consume* it.
If it only displays, it's DF cosplay; if it drives, it's DF.

## 5c. World Verbs & Generative Systems — what agents actually DO

Minds (§5b) give agents an inner life; **world verbs** give them the ability to *change the world* while
living it. The product owner's goal: NPCs chop trees, build houses, farm, craft, breed, and expand villages
**on their own initiative** — not only when the player designates work. This is what makes DF's world feel
alive: civilizations build and grow without you.

### 5c.1 The core principle — one action library, two drivers
A world verb (chop tree, build wall, plant crop, forge tool, dig burrow) is defined **once**, with **one
authoritative world-effect and code path**, and can be triggered by **either** driver:
- **Player-driven:** designation → colonist job (main doc B4/B5/B6). *You* asked for it.
- **NPC-driven:** the agent's own drive — a woodcutter's profession, a village's growth need, an animal's
  instinct. *They* chose it, per their mind/agency.

Same verb, same effect, different *reason*. This keeps colonist work and autonomous NPC life from becoming
two divergent codebases, and means every verb added serves both. **Build the action library as a shared
`bastion` module both the job system and the NPC AI call.**

### 5c.2 The verb families (build in this dependency order — it's also the product-owner ranking)
1. **Gathering** — chop (tree→logs), mine (voxel→stone/ore), forage (plants→food), hunt (animal→meat/hide),
   fish. *Reuse:* terrain edit + Veloren's block/sprite→loot mapping (§2a). The foundation — everything else
   consumes gathered materials.
2. **Construction** — build houses/walls/roads; **expand villages**. *Reuse:* `MakeBlock`/`MakeVolume` (§2a)
   + blueprint→build flow (B5). Consumes gathered materials.
3. **Production / crafting** — workshops, farming (plant→tend→harvest), cooking, smithing. *Reuse:* Veloren's
   item/recipe system + crafting stations (which exist in towns). Consumes gathered/grown inputs.
4. **Animal / creature building** — nests, dams, burrows, lairs. The instinct-driven analog of construction;
   a lighter parameterization of the same build verbs, driven by animal minds.
5. **Reproduction** — the capstone (§5c.4). Closes the loop so populations *grow*, not just decline.

### 5c.3 Autonomous settlement growth (LOD-critical)
Villages expand **on their own** (product-owner choice), but at **level-of-detail** or it breaks rtsim:
- **Simulated tier (unwatched):** a village "builds a house" is a **low-res rtsim site-growth event** —
  population↑ / resources↑ → the site record gains a structure. Cheap; rtsim already does site generation.
  *No per-block placement for 200 offscreen villages.*
- **Loaded tier (you're there):** growth manifests as **actual placed voxels** via the real build verbs — an
  NPC woodcutter gathers, a builder constructs, the house rises where you can watch.
- The two reconcile at the boundary: an rtsim "site grew a house" becomes real geometry when loaded; real
  construction you watch updates the rtsim site record. Same loaded↔simulated law as bodies and minds.

### 5c.4 Reproduction & genealogy (deep, DF-style — LOD-aware)
Reproduction closes the loop that makes a world self-sustaining and *generative* (families, dynasties, growing
factions). rtsim already has a repopulation queue + wildlife-population goals — build on them.
- **Humanoids:** colonists/NPCs form pairings (driven by mind relationships §5b — lovers/spouses), have
  **children** who inherit a blend of parent **personality facets + values + traits** (§5b model) and enter
  the kin graph. Villages grow from births, not just migration.
- **Animals:** herds/packs breed when population+food allow (feeds ecosystem equilibrium §5.4); offspring
  inherit lighter trait profiles.
- **Genealogy LOD:** **full kin graphs + family trees + inherited traits** run at **full-res for tracked/
  loaded lineages** (your colony, nearby historical figures); **distant bloodlines persist as compact
  ancestry records** (parent links + key inherited traits), promotable on inspect. This gives DF's "grandson
  of the founder who slew the wyvern" **without** holding every living thing's full tree in memory always.
- **Ties to the Mind & inspector:** kinship shows in relationships (B-AG4 tab); a child's inherited
  personality comes from B-AG3; a parent's death creates grief thoughts + a persistent memory (§5b).

### 5c.5 Drives — how NPCs *decide* to use verbs
An NPC-driven verb is chosen the same tendency-first way as all agency (§0): the agent's **profession +
mind + needs + site context** produce a drive ("this woodcutter tends to fell trees near the village";
"this village needs housing → commission a build"; "this doe is ready to breed"). Drives feed the same
job/task machinery colonists use — the NPC essentially *self-designates*. Graceful failure applies: no trees
→ woodcutter idles differently or relocates, never freezes (the standing-around bug, §1).

## 6. Open questions / to author in later passes
- Exact stat/alignment values per creature (fill from `body.rs` / `entity/wild`).
- Full social graph (who-likes/hates-whom) as a data table.
- Seasonal/weather hooks (tie to `WeatherGrid`, main doc §2a) — migration, hibernation.
- Reproduction/population curves per species (rtsim wildlife-population goal).
- Faction diplomacy (rtsim2 goal: organic power struggles) — how bands/kingdoms contend.
- Interaction between agency and the player's god-powers (blessing a herd, cursing a raider band, etc.).

*End of Agency Bible v0.1 — flagships authored; expand §4 group by group, always tendency-first.*
