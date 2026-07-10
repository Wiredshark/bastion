# Project Bastion — Comprehensive Feature Gap Analysis (god games · RimWorld · Dwarf Fortress + blind spots)

**Status: RESEARCH / CAPTURE DOC. Nothing here is queued.** The definitive "what are we missing" sweep
across the three source genres plus the unnamed blind spots (physics, feature categories, legibility). Every
item is filtered through one test: **does it amplify "autonomous god-sim over a living voxel world," or pull
toward another genre's identity?** Coverage tags: **[HAVE]** designed/built · **[PARTIAL]** substrate or
partial design · **[GAP]** not addressed. Companion to the DF Gap Ledger (DF depth), the Divine Politics
Bible (world politics), the cross-genre nice-to-haves (borrowable features), and future-work (deferred ideas).
This doc is the *union* — the master checklist.

---

## PART 1 — GOD-GAME mechanics (the genre Bastion actually IS; thinnest prior coverage)

Research finding worth internalizing: the deep-sim 3D god game is an **empty, wanted niche** — Populous/B&W
are dead/unavailable, Godus failed, survivors (WorldBox, Reus, Universim) are 2D or shallow. The genre's
*essence*, per the sources: **you design/influence, you never command; the world is autonomous; you contest
faith.** Bastion is dead-center in this. So these mechanics aren't borrowed flourishes — they're the genre's
core vocabulary, and several are missing.

| God-game mechanic | Coverage | Bastion home / note |
|---|---|---|
| Indirect influence over autonomous followers (the genre's soul) | [HAVE] | Pillar §1a — the whole design |
| Terrain raise/lower (Populous's *founding* verb) | [GAP] | **The single most iconic missing god power.** You have block edit/`MakeVolume`; "raise a hill / lower a valley / flatten for my people" belongs in B13. HIGH priority, cheap. |
| Miracles / divine powers (rain, storm, quake, fire, plague, bless) | [PARTIAL] | B13 wires existing ops (Explosion/Lightning/Weather/Buff); needs the full menu + favor cost. |
| Worship / faith as the power economy | [HAVE] | Divine Politics + B13 favor |
| Competing rival gods contesting followers | [HAVE] | Divine Politics DP4 |
| Convert-or-destroy other tribes (Populous) | [HAVE] | Divine Politics (conversion = master diplomatic verb) |
| **"The game watches and remembers you" (B&W morality)** | [GAP] | **Big missing idea.** B&W's genius: the world *records* your pattern of acts and reflects it — a cruel god's land/temples/music turn dark, a kind god's turn bright. Bastion could track your divine behavior and have the **world's aesthetic + your followers' character reflect the god you've been.** Deeply on-theme; ties faith + Divine Politics. Design-worthy. |
| **The trained beast/avatar (B&W creature)** | [PARTIAL] | B&W's beloved feature: a mortal avatar with its own mind you *teach* by example. Bastion's **Embody (B12)** is the possession version; a persistent *trainable companion creature* that acts on its own between possessions is a richer, on-theme variant. Consider. |
| Awe/humility balance — abundance breeds greed (Reus) | [PARTIAL] | Noted in cross-genre doc; fold into faith (too-comfortable → decadent/faithless). The mechanic that makes *restraint* a strategy. |
| Terraforming to shape where people settle | [PARTIAL] | Ties terrain raise/lower; B-AG6 settlement growth reacts to terrain. |
| Disasters as the destruction half of the loop | [PARTIAL] | Have some (Explosion/Lightning); ledger has rest; make wrath *cost* faith. |
| "Run it at max speed and watch" (WorldBox timelapse) | [GAP] | Time controls (future-work §3d) + the soak-as-entertainment insight. An **observe/timelapse mode** is genuinely god-game-core. |
| Moral/allegiance framework (kind vs cruel god) | [GAP] | Sources stress god games aren't pure sandbox — they have a moral frame that shapes follower loyalty. Bastion's God/Free modes are a start; a **kind↔cruel axis affecting faith/loyalty** would deepen it. Ties B&W "watches you." |
| Prayer as the summon/interaction channel | [HAVE] | Divine Politics prayer feed |
| Player-god progression (grow simple→overwhelming powers) | [GAP] | **See blind-spots §5.** Populous/Universim grow your power over time; Bastion's god is currently a fixed capability set. Real unexplored space. |

---

## PART 2 — RIMWORLD mechanics (autonomy + narrative + colonist depth)

| RimWorld mechanic | Coverage | Bastion home / note |
|---|---|---|
| AI Storyteller / director (paces events for drama) | [GAP→flagged] | **Highest-value borrow** (cross-genre doc). Reframe: rival gods ARE the storytellers. Kills the "boring soak" failure. |
| Mood = running sum of +/− thoughts, **negativity bias** (bad ~2× good) | [HAVE] | B-AG3 — and confirm the negativity-bias asymmetry is modeled (it's the realism that makes it land). |
| Traits ≈ Big Five personality | [HAVE] | B-AG3 facets |
| Mental breaks scaling with severity (wander→berserk→catatonic) | [HAVE] | B-AG3 mood/breakdown |
| Needs (food/rest/recreation/beauty/comfort/social) | [PARTIAL] | B7 core needs; add beauty/comfort/social (some in DF-ROOMS/FOCUS) |
| **Temperature as health** (heatstroke/hypothermia/frostbite + comfortable range) | [GAP] | **DF-TEMP** covers the sim; RimWorld shows the *colonist* side: comfortable range, apparel insulation, work-speed penalty out of range, mood hit. Ties clothing (§ below) + seasons. |
| **Bodypart-level health** (organs, prosthetics/bionics, transplants, pain→consciousness) | [PARTIAL] | **DF-WOUND** is the DF version; RimWorld's is cleaner: per-part health, pain as a consciousness/work debuff, replaceable parts. Feeds B-AG4 Health tab. |
| Disease / infection / immunity race | [GAP] | **DF-SYNDROME** adjacent; a disease system (catch → immunity vs. severity race → treat or die). |
| Medicine / doctoring / surgery / hospital beds | [GAP] | **DF-MEDICAL**. A labor + building chain; ties bodypart health. |
| **Ideology / belief precepts** (Ideology DLC — rituals, taboos, roles, style) | [PARTIAL] | Divine Politics faith is the world-politics version; RimWorld's *per-colony* belief precepts (this colony reveres X, taboos Y, holds these rituals) is a **colony-culture layer** worth noting — ties DF-RELIGION + DF-FESTIVAL. |
| Clothing/apparel (worn, wears out, insulates, mood from quality/tattered) | [GAP] | Ties temperature + colonist inventory (blind-spots §4). Veloren has equipment substrate. |
| Recreation / joy sources (variety matters) | [PARTIAL] | B7 recreation need; RimWorld shows *variety* matters (same joy source → boredom). |
| Prisoners / recruitment / conversion | [GAP] | Capture → recruit/convert. Ties raids (B8) + faith. |
| Animals: taming, training, bonding, hauling, hunting | [PARTIAL] | Veloron has taming/pets → SUBSTRATE; DF-LIVESTOCK. RimWorld adds *bonding* (pet↔colonist mood ties) — feeds B-AG3. |
| Caravans / trade / world map travel | [PARTIAL] | DF-TRADE + Divine Politics trade |
| Drugs / addiction / tolerance | [GAP] | On-theme for DF-ish depth (booze already implied by taverns); addiction as a mood/health system. Low priority. |
| Research tree | [AVOID] | Same caution as Universim tech tree — pulls toward 4X/management. Bastion's "progression" is faith + history, not research. Skip unless deliberate. |
| Wealth → raid scaling (richer colony = bigger threats) | [PARTIAL] | **DF-PRESTIGE** captures this; it's also the RimWorld pacing lever — tie to the storyteller/director. |

---

## PART 3 — DWARF FORTRESS mechanics (depth; mostly in the DF Gap Ledger already)

The DF Gap Ledger is the authoritative DF inventory. Summary of coverage here; see ledger for the full tagged
list with DF-IDs and costs.

- **Well covered / designed:** designations, labors/skills, stockpiles, needs/mood/tantrum, multi-Z view,
  threats/sieges, save/load, embark, the full **Mind** (personality facets + values + conflicts + memory +
  FOCUS + thoughts), reproduction/genealogy, world verbs.
- **Ledger GAPs (design-pass-gated), the notable ones:** production chains + workshops + farming + cooking,
  quality tiers + **artifacts/strange moods**, engineering (mechanisms/levers/power/traps/pumps — one
  trigger→link→effect engine), fluid + magma + caverns + geology, anatomically-detailed wounds + medicine,
  taverns + temples/religion + libraries/knowledge + art forms + justice/nobles + guilds, legends/chronicle,
  off-map missions, villains/plots, forgotten beasts, night creatures (vampires/werebeasts/necromancers),
  **vertical dig-verbs (stairs/ramps/channels — gameplay-critical)**, standing orders, rooms/room-value,
  refuse/rot/miasma, notes, prestige.
- **Confirmed NOT missing any whole category** — the DF fortress-mode menu maps ~1:1 onto Bastion's HUD +
  inspector + the DF-\* backlog. Remaining work is depth-per-system.

---

## PART 4 — UNNAMED FEATURE CATEGORIES (blind spots — not in any prior doc)

These came up as "we've literally never discussed this." Each gets named so it's not a surprise mid-build.

- **Death / corpse / burial** [GAP] — **emotionally load-bearing, do not skip.** Colonists die; the body
  needs handling: haul corpse → bury/tomb (a room with value, DF-ROOMS) or it rots (miasma → Hazard Events →
  bad thoughts). Unburied dead → grief/haunting. Ties B-AG3 (grief memory, grudges), Hazard Events (miasma),
  DF-ROOMS (tombs as valued rooms). DF proves this is core to player *attachment* — you mourn dwarves partly
  through burial. Deserves a real design pass.
- **Colonist inventory / equipment** [GAP] — do colonists wear armor, wield tools that improve work, carry
  hauled goods? Veloron has a rich hero equipment system as substrate. Decision needed: are colonists
  *equipped agents* (armor→combat, better tool→faster work, clothing→temperature) or abstract? Affects
  combat, work rates, hauling, temperature. Unnamed until now.
- **Sound / audio identity** [GAP] — zero prior discussion. Veloron has an audio system (inherit it), but the
  god-game's audio *identity* is unconsidered: ambient world hum, divine-power sounds, combat/alert cues, and
  audio as a **legibility tool** (the tavern's noise tells you it's lively; a scream tells you something's
  wrong before you see it). Cheap to start, big for feel.
- **Day/night as GAMEPLAY** [GAP] — you have the cycle visually. Make it *do* something: colonists sleep at
  night, predators hunt, raids favor darkness, temperature drops, lit vs. unlit work. High value, low cost
  (the cycle exists), and it's core colony-sim texture. Ties lighting (§5) + temperature.
- **Seasons as GAMEPLAY** [GAP] — visual only today. Crops grow seasonally, **winter = food pressure** (the
  classic colony-sim threat that makes "survive a month" tense), migration timing, frozen water. This is
  where the soak test gets *interesting*. High value, cycle exists. Ties DF-FARM + DF-TEMP.
- **Colony-culture / per-colony identity** [GAP] — RimWorld Ideology's insight: a colony has a *culture*
  (reveres X, taboos Y, holds rituals, has a style). Bastion's Divine Politics does world-faith; a
  *per-colony* belief/ritual/taboo layer would give your own colony character. Ties DF-RELIGION/FESTIVAL.

---

## PART 5 — UNNAMED PHYSICS / SIMULATION GAPS (name them so they're not surprises)

- **Structural integrity / cave-ins** [GAP] — dig out a support and nothing collapses; you can dig a floating
  mountain and it hangs there. **Decision needed:** DF-grade support rules (a whole subsystem) vs. accept
  floating terrain (probably the answer for a long time). Naming the decision so it's conscious, not an
  accident. If ever built, cave-in = Hazard Event.
- **Fire spread** [GAP] — have fire as damage + Explosion powers, but no *propagation* (burning building →
  next building, forest fire). It's a cellular-spread system, **cousin to fluid (DF-FLUID) and belongs in the
  Hazard Events engine.** Group it there.
- **Lighting / darkness as mechanic** [GAP] — underground is dark; does it *matter*? Light sources needed,
  slower/refused work in dark, mood from gloom. Have the underground *view* (B1.6 relight) but not
  darkness-as-gameplay. Ties day/night. DF has it.
- **Weight / 3D pathfinding load** [RISK, not feature] — hundreds of colonists pathing in 3D is a real
  **performance cliff** (B4 already chose navmesh-vs-full-3D as a live question). The thing most likely to
  force ugly compromises later. Standing risk to watch, not a feature to add.
- **(Already deferred, cross-ref):** fluid/water flow (DF-FLUID, future-work §3a), temperature sim (DF-TEMP),
  real rigid-body physics (explicitly NOT wanted — trees, etc.).

---

## PART 6 — LEGIBILITY & ONBOARDING (the quiet killer — treat as a PILLAR, not a feature)

**This is the most underweighted risk in the whole project, and unlike fluid it is NOT optional enrichment.**
A beautiful, deep sim that a player cannot *understand* is a failed game — and god games *specifically* fail
here (research: Godus, and the genre's chronic "I have no idea what I'm doing / no control" problem). The
indirect-influence model (you shape conditions, they decide) is *inherently harder to read* than direct
command. So legibility affordances are what make the depth **playable**, and they thread through the HUD
blocks (B9) you'll reach sooner than the deep-sim ones.

- **Teaching the influence model** [GAP] — no tutorial, no "what do I do," no feedback loop that teaches
  "you don't command, you influence." Needs deliberate onboarding design, not a bolted-on tutorial.
- **Failure legibility** [GAP] — when the colony dies or stagnates, does the player see *why*? DF's "losing
  is fun" works *because* death is legible (you watch the tantrum spiral, the flood). Opaque failure is just
  frustrating. The event log / Chronicle (DF-LOG/DF-HIST) is part of this.
- **State legibility (overlays)** [PARTIAL] — faith/mood/needs-coverage overlays on the god map (cross-genre
  doc); the social event feed (RimWorld); alerts + jump-to-event (RTS). These make the invisible sim visible.
- **Consequence feedback** [GAP] — when you use a god power, is its effect (and cost, and ripple) *legible*?
  A god who can't see what their miracle did can't learn to play.

**Recommendation:** treat legibility as a **design pillar tracked alongside the sim**, not a late feature.
Every system that adds depth should answer "how does the player *see* this?" as part of its design pass.

---

## PRIORITY SYNTHESIS — what actually matters, ranked

Filtered by (a) fit with the autonomous-god-sim identity, (b) value-to-effort, (c) whether it's load-bearing
vs. enrichment. Not "do all of this" — this is the triage.

**Tier A — high value, name/design soon (some cheap, some pillar-level):**
1. **Legibility & onboarding** (Part 6) — pillar, not feature; thread through every design pass. The thing
   most likely to sink a great sim.
2. **Director / storyteller** (rival gods as pacers) — kills the boring-soak failure.
3. **Day/night + seasons as gameplay** — cheap (cycles exist), makes survival *tense* (winter), core texture.
4. **Death / corpse / burial** — emotionally load-bearing, ties 3 systems you're already building.
5. **Terrain raise/lower god power** — the iconic missing god verb, cheap given voxel edit.
6. **Over-godding penalty + kind/cruel axis + "world remembers you"** (god-game moral layer) — the on-theme
   depth that makes divinity *mean* something; ties faith.

**Tier B — real, more build, natural homes exist:**
Temperature-as-health, bodypart health + medicine, colonist equipment/clothing, disease, prisoners/recruit,
animal bonding, colony-culture/ideology layer, fire spread (→ Hazard Events), lighting/darkness, the trained
companion beast, observe/timelapse mode.

**Tier C — deep/late/optional (mostly DF ledger + the epics):**
The DF depth backlog (production/engineering/caverns/wounds/culture), fluid, structural collapse, villains,
forgotten beasts, night creatures, drugs, player-god progression as a full arc.

**AVOID (dilutes identity):** research/tech trees (→ 4X), freeform block-building (→ Minecraft), unit micro
(→ RTS), infrastructure micromanagement (→ city-builder). Bastion's moat is *deep autonomous sim under a
god's influence* — deepen into it, don't wander out.

---

## The through-line
Bastion sits in a genuinely empty niche: **a beautiful, deep, 3D, DF-souled living world you preside over as
one of several contesting gods.** Everything above is judged by whether it *deepens that specific thing*. The
biggest under-covered areas aren't exotic — they're **legibility** (so people can play the depth),
**god-game soul** (moral weight, the world remembering you, restraint-as-strategy), and **cheap survival
tension** (day/night + seasons + death). Those, plus the DF depth already inventoried, are the real map.

*End. This is the master union of gaps. Promote items into the design doc / a block / a design pass as they
earn one.*
