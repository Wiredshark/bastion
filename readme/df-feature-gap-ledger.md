# Project Bastion — DF Feature Gap Ledger (the "everything we missed" mega-doc)

**Companion to the main build report (v2.1) and the Agency Bible.** This is the *completeness ledger*:
every major Dwarf Fortress system, mapped against what Bastion already covers, what Veloren gives us as
substrate, and what remains a gap — each gap assigned a build ID (`DF-*`). It is intentionally exhaustive
and intentionally *unsequenced by ambition*; the main doc + Mega-Prompt decide order and pacing.

**Read this against three anchors from the main doc:**
- **Pillar §1a** — autonomous god game; the player influences, doesn't micromanage. DF is *inspiration*,
  not a spec to port feature-for-feature. Every DF feature below is reinterpreted through the god-game lens.
- **The loaded↔simulated LOD law** — anything population-scale runs cheap when unwatched, full-res when
  loaded/selected. Applies to every system here.
- **rtsim's "assume nothing, tend toward equilibrium" law** — all agent/world behavior is tendency-first,
  graceful-failure.

**Honest framing:** DF is ~20 years of one designer's obsession, and its systems are deeply entangled. This
ledger does NOT promise all of it. It *inventories* it so nothing is invisible, tags what's cheap (Veloren
substrate exists) vs. expensive (net-new), and lets you choose. Treat "coverage" as: **[DONE]** designed in
main doc · **[SUBSTRATE]** Veloren has most of it, needs wiring · **[GAP]** net-new design/build.

---

## Legend
- **Coverage:** DONE (in main doc) / SUBSTRATE (Veloren has it) / GAP (net-new)
- **Cost:** ¢ cheap (wire existing) · $ moderate · $$ large · $$$ epic (entangled, multi-block)
- **DF-ID:** build identifier for the Mega-Prompt queue.

---

## A. Colony / Fortress core (mostly covered — cross-reference)
| DF system | Coverage | Bastion home | Notes |
|---|---|---|---|
| Designations (dig/chop/build/haul) | DONE | B4/B5/B6 | Player intent → jobs |
| Labors & skills, work priorities | DONE | B3/B4 | RimWorld work grid |
| Stockpiles & hauling | DONE | B6 | Conservation invariants |
| Needs / mood / tantrum spiral | DONE | B7 + B-AG3 | Mood drives behavior |
| Multi-Z digging & cross-section view | DONE | B1/B1.6/B1.8 | Z-slice + underground mode |
| Threats / sieges / raids | DONE | B8 + Agency Bible §3.4 | Autonomous defense |
| Save/load persistence | DONE | B10 | rtsim + ECS slice |
| Embark / site selection | DONE | B11 | Reuse worldgen |

## B. The Mind & the individual (mostly covered — cross-reference)
| DF system | Coverage | Bastion home | Notes |
|---|---|---|---|
| Personality facets (~50), values | DONE | B-AG3 (Agency Bible §5b) | Drives thoughts |
| Thoughts/emotions pipeline | DONE | B-AG3 | event×personality×values×memory→emotion→mood |
| Memory (decaying) + grudges (persistent) | DONE | B-AG3 | Asymmetric decay |
| Relationships / sentiment | SUBSTRATE→DONE | B-AG3 (+ Veloren sentiment) | Deepen to friend/rival/kin/lover/grudge |
| Preferences (likes/dislikes: art, food, weather) | GAP | **DF-PREF** | Feeds thoughts; per-individual likes. ¢–$ (extend B-AG3 values) |
| Unit inspector (full sheet) | DONE | B-AG4 | Tabs = build checklist |

## C. Reproduction, family, history (mostly covered — cross-reference)
| DF system | Coverage | Bastion home | Notes |
|---|---|---|---|
| Reproduction / children / inherited traits | DONE | B-AG6 | Deep genealogy |
| Kin graph / family trees | DONE | B-AG6 | LOD-aware |
| Historical figures & legends | SUBSTRATE | **DF-HIST** | rtsim already accrues history; needs a **Legends/Chronicle** browser. $ (main doc already wants a Legends viewer) |
| Villains / plots (embezzlement, sabotage, kidnapping, assassination) | GAP | **DF-VILLAIN** | rtsim2 faction goal aligns. $$$ entangled with factions/reputation |

## D. Production & industry (the big GAP cluster — DF's economic depth)
| DF system | Coverage | Notes / DF-ID |
|---|---|---|
| World-verb action library (gather/build/produce) | DONE | B-AG5 |
| Workshops (typed: mason, carpenter, smith, craftsdwarf, kitchen, still, loom…) | GAP | **DF-WORKSHOP** — typed production buildings with recipe sets. Veloren has crafting stations → SUBSTRATE-ish. $$ |
| Multi-step production chains (ore→bar→goods; plant→thread→cloth→dye) | GAP | **DF-CHAIN** — the heart of DF industry. Reuse Veloren recipes. $$ |
| Farming (plots, seeds, seasons, crop selection) | GAP | **DF-FARM** — plant→tend→harvest→process. $$ |
| Cooking / brewing (meals, drinks, quality) | GAP | **DF-COOK** — food quality → mood. $ |
| Animal husbandry (pasture, breeding, milking, shearing, butchery) | GAP | **DF-LIVESTOCK** — Veloren has taming/pets → SUBSTRATE. $$ |
| Quality levels (masterwork→ *artifact*) on all goods | GAP | **DF-QUALITY** — quality tier per item, tied to skill; feeds value + mood. $ |
| **Artifacts / strange moods** (fact-checked mechanics) | GAP | **DF-ARTIFACT** — DF-accurate: once the colony passes a citizen threshold (~20), a counter+chance clock strikes a colonist with a **strange mood**; they **claim a workshop**, demand materials **in a specific order**, work to the exclusion of eating/sleeping, and produce a **named legendary artifact** + legendary skill — **or go insane and die if their demands can't be met** (the failure mode IS the drama; keep it). Mood type ties to B-AG3 (fey/possessed/macabre by mind state); item type honors the colonist's *preferences*. $$ |
| **Focus / personal needs** (missing system, fact-checked) | GAP | **DF-FOCUS** — personal needs derived from facets/values (pray, family, romance, crafts, see animals…); met:unmet ratio = **focus**, worth **±50% work speed/quality**, independent of mood; NPCs self-generate need-jobs (low-prio yields to work, high-prio doesn't). Player influences it indirectly (temple/tavern/zoo). Extends B-AG3/B7. $$ |
| Trade / caravans / merchants / trade depot | SUBSTRATE | **DF-TRADE** — Veloren has merchants/trade/economy in rtsim. Wire caravans + a depot + haggling. $$ |
| Economy / wealth / property / rooms & values | GAP | **DF-ECON** — room value, wealth rating (draws migrants/sieges). $$ |
| Guilds / craft guilds | GAP | **DF-GUILD** — profession guilds w/ demands. $ (ties factions) |

## E. Engineering & machinery (DF's signature GAP)
| DF system | Coverage | Notes / DF-ID |
|---|---|---|
| Mechanisms, levers, pressure plates, linkages | GAP | **DF-MECH** — player-wired logic. $$ (fits god-designation model) |
| Gears / axles / power (water/wind) | GAP | **DF-POWER** — power grid. $$ |
| Pumps / fluid engineering | GAP | **DF-PUMP** — ties to fluid sim (B13 From-Dust flow). $$ |
| Traps (weapon/cage/stonefall) + siege engines (ballista/catapult) | GAP | **DF-TRAP** — defensive engineering, fits autonomous-defense B8. $$ |
| Bridges / floodgates / hatches / doors (operable) | GAP | **DF-OPERABLE** — lever-linked terrain. $ |
| Minecarts / hauling routes / rollers | GAP | **DF-MINECART** — advanced logistics. $$ (low priority) |

## F. Water, magma, geology, caverns (world-depth GAP)
| DF system | Coverage | Notes / DF-ID |
|---|---|---|
| Fluid simulation (water/magma flow, pressure) | GAP | **DF-FLUID** — B13 From-Dust flow is the seed; DF-grade adds pressure/level. $$$ |
| Magma (forges/smelters w/o fuel, magma sea) | GAP | **DF-MAGMA** — deep-layer magma; fuel-free industry. $$ |
| Layered caverns (3 danger tiers, underground biomes) | SUBSTRATE | **DF-CAVERN** — Veloren has dungeons/dwarven mine + underground culling. Formalize danger tiers + cavern life. $$ |
| Geology / ore veins / stone layers / gems | SUBSTRATE | **DF-GEOLOGY** — Veloren worldgen has ores/materials. Expose veins/layers to mining. $ |
| Aquifers / drainage / cave-ins / flooding | GAP | **DF-HYDRO** — hazards; "losing is fun." $$ |
| Temperature (heat/cold, melting, freezing, fire spread) | GAP | **DF-TEMP** — Veloren has weather/seasons → partial. $$ |

## G. Combat, health, military (partial — deepen)
| DF system | Coverage | Notes / DF-ID |
|---|---|---|
| Real combat / damage / death | SUBSTRATE | Veloren combat (B8) |
| **Anatomically-detailed wounds** (bodypart, organ, sever, bleed, pain, infection) | GAP | **DF-WOUND** — DF's signature gore-sim; deepen Veloren health. Feeds B-AG4 Health tab. $$$ |
| Healthcare (hospital, doctor, diagnosis, surgery, splints, rest) | GAP | **DF-MEDICAL** — a labor+building chain. $$ |
| Military (squads, uniforms, barracks, training, schedules, patrols) | GAP | **DF-MILITARY** — but reframe god-side: *policy* not micro (B8). $$ |
| Ranged/fortifications/archery ranges | GAP | **DF-RANGED** — wire to B8 defense. $ |
| Syndromes / poison / disease / were-curses / vampirism | GAP | **DF-SYNDROME** — status-effect engine; Veloren buffs → SUBSTRATE. $$ |

## H. Culture, knowledge, society (the "tavern release" GAP cluster)
| DF system | Coverage | Notes / DF-ID |
|---|---|---|
| Taverns (drink, socialize, performers, visitors) | SUBSTRATE | **DF-TAVERN** — Veloren has taverns + tavern rtsim behavior. Wire socializing/needs. $ |
| Temples / religion / worship / prophets / monastic orders | SUBSTRATE | **DF-RELIGION** — ties to the *god* theme beautifully (you're the god). $$ |
| Libraries / scholars / knowledge / research tree | GAP | **DF-KNOWLEDGE** — a knowledge/discovery system. $$ |
| Art forms (music, dance, poetry, writing; procedural forms) | GAP | **DF-ART** — procedural culture; feeds mood + value. $$ |
| Festivals / ceremonies / holidays | GAP | **DF-FESTIVAL** — faction culture events. $ |
| Nobles / positions / mandates / justice (sheriff, hammerer, jail, trials) | GAP | **DF-JUSTICE** — social hierarchy + law. $$ (fits policy layer) |
| Petitions / residency / citizenship / migration waves | SUBSTRATE | **DF-MIGRATION** — rtsim migration exists; add petitions/waves tied to wealth. $ |

## I. World & meta (worldgen depth GAP)
| DF system | Coverage | Notes / DF-ID |
|---|---|---|
| Deep worldgen (geology+history+civ, thousands of years) | SUBSTRATE | Veloren worldgen + rtsim history. Deepen. |
| Legends mode / chronicle browser | SUBSTRATE | **DF-HIST** (see C) |
| Off-map missions / raids / rescue / artifact retrieval | GAP | **DF-MISSION** — send colonists off-map (rtsim resolves). Fits god directing colony. $$ |
| Reclaim / retire fortress / cross-mode play | GAP | **DF-RECLAIM** — retire/reclaim a colony; Embody (B12) is the adventurer-mode seed. $$ |
| Evil/good biome effects, evil weather | GAP | **DF-BIOME-FX** — ties WeatherGrid + god-powers (B13). $$ |
| Procedural monsters (forgotten beasts w/ unique abilities) | GAP | **DF-BEAST** — proc-gen apex threats; Veloren has wyverns/gigas → SUBSTRATE seed. $$$ |
| Night creatures (vampires, werebeasts, necromancers, undead) | GAP | **DF-NIGHT** — curse/syndrome-driven (ties DF-SYNDROME). $$$ |

## J. Interface / experience (partly covered)
| DF system | Coverage | Notes / DF-ID |
|---|---|---|
| Unit view / sheets | DONE | B-AG4 |
| Colony overview / alerts / work tab | DONE | B9 |
| Announcements / combat log / event stream | GAP | **DF-LOG** — scrolling event log → the Chronicle. $ |
| Manager / work orders / conditional orders | GAP | **DF-ORDERS** — standing production orders ("keep 20 meals"). Fits *policy* layer. $ |
| Zones (meeting/pasture/hospital/garbage/water source) | GAP | **DF-ZONES** — typed zones beyond stockpiles. Veloren `AreaAdd` → SUBSTRATE. $ |
| Burrows (restrict movement) | GAP | **DF-BURROW** — restrict colonists to areas (policy). $ |

## K. Systems surfaced by the DF fortress-mode audit (previously missing — now inventoried)
| DF system | Coverage | Notes / DF-ID |
|---|---|---|
| **Vertical digging verbs** (up/down stairs, ramps, channels, up/down passages) | GAP | **DF-DIG-VERBS** — DF's fortress is *built* from these; more than "mine a voxel." A colony sim needs stairs/ramps to move between Z-layers. Ties to B5 + B1.8 underground. $ |
| **Standing orders / auto-behaviors** (gather refuse y/n, render fat, forbid-on-death, auto-loom…) | GAP | **DF-STANDING** — colony-wide default rules (distinct from per-order DF-ORDERS). Pure *policy* — perfect god-game fit. $ |
| **Kitchen / cook & brew permissions** (which crops may be cooked/brewed; don't cook your seeds) | GAP | folds into **DF-COOK** — the permission layer is the gameplay depth, not just the recipe. |
| **Refuse / rot / miasma / vermin** (corpses rot, refuse piles, miasma clouds → bad thoughts, vermin/pests) | GAP | **DF-ROT** — decay & hygiene as a pressure; miasma feeds B-AG3 thoughts. Ties DF-TEMP. $ |
| **Bookkeeper / accounting accuracy** (stock counts are only as accurate as your record-keeping) | GAP | **DF-BOOKS** — a lovely DF touch: information itself is a managed resource. Optional flavor. ¢ |
| **Notes / map annotations / patrol routes** | GAP | **DF-NOTES** — player-placed map notes + (for defense) patrol routes. Fits the overseer HUD. ¢ |
| **Rooms & room-quality from furniture** (a walled space + bed = bedroom; furniture value → room value → mood) | GAP | **DF-ROOMS** — the bridge between construction and the mind (nice room → good thoughts). Ties DF-ECON + B-AG3. $ |
| **Attract-the-monarch / colony prestige goal** (wealth + prestige draws migrants, nobles, and *bigger threats*) | GAP | **DF-PRESTIGE** — the soft objective: prosperity attracts both immigrants and danger. God-game reframe: your favor/miracles raise prestige → the world *notices* your chosen people. $ |
| **Interface-completeness note** | — | The DF fortress menu (Squads/Notes/Burrows/Stockpiles/Zones/Locations/Nobles/Status[Animals/Kitchen/Stone/Stocks/Health/Justice]/Unit-list) maps almost 1:1 onto Bastion's B9 HUD + B-AG4 inspector + the DF-* systems above. No *whole category* is missing — the audit confirms coverage. What remains is depth-per-system, which is the Phase-5 grind. |

---

## Priority reading (my architect's recommendation on what actually matters)

Not all gaps are equal for *this* game. Weighted by (a) fit with the autonomous-god-game pillar, (b) how
much they make the world feel DF-alive, (c) Veloren substrate leverage:

**Tier 1 — high value, strong substrate (do within the main arc):**
DF-TRADE, DF-TAVERN, DF-RELIGION (you're literally the god — huge thematic fit), DF-CHAIN + DF-WORKSHOP +
DF-FARM + DF-COOK (the industry that gives agents things to do), DF-QUALITY + DF-ARTIFACT (mood payoff),
DF-FOCUS (personality→needs→work performance — the loop that makes minds *matter*), DF-HIST (Legends/
Chronicle — the world's memory), DF-ZONES + DF-ORDERS + DF-LOG (policy-layer wins), DF-CAVERN + DF-GEOLOGY +
DF-MAGMA (the vertical world DF is famous for), DF-WOUND (signature depth).

**Tier 2 — great, more build:**
DF-MECH + DF-POWER + DF-TRAP + DF-OPERABLE (engineering — fits the god-designation model), DF-FLUID +
DF-PUMP (extends B13), DF-MEDICAL, DF-MILITARY (policy-framed), DF-SYNDROME, DF-JUSTICE, DF-MISSION,
DF-LIVESTOCK, DF-MIGRATION.

**Tier 3 — epic / late / risky:**
DF-VILLAIN, DF-BEAST, DF-NIGHT, DF-KNOWLEDGE, DF-ART, DF-MINECART, DF-RECLAIM, DF-BIOME-FX, DF-HYDRO,
DF-TEMP, DF-ECON, DF-GUILD, DF-FESTIVAL, DF-PREF.

**The through-line that keeps this coherent:** every gap above is reinterpreted through the god game —
you don't micro a workshop, you *set policy and watch*; you don't command a squad, you shape defense and
*occasionally intervene* as a god-power. DF supplies the *depth of simulation*; Bastion supplies the
*god's-eye relationship to it*. That's the reinterpretation that makes this Bastion and not a DF clone.

*End of gap ledger. This inventories the mountain; the main doc + Mega-Prompt decide the climbing route.*
