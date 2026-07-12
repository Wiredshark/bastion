# Project Bastion — Approved Master Build List

> **Sequence authority.** This is the single order the systems builder executes. Exactly one row is `CURRENT`.
> The builder acts from [BASTION_CURRENT_BLOCK.md](BASTION_CURRENT_BLOCK.md); the architect owns advancement here.
> Coding direction lives in the [Implementation Playbook](BASTION_IMPLEMENTATION_PLAYBOOK.md), references in the
> [Design Index](BASTION_MASTER_DESIGN_INDEX.md), and shared ownership in the
> [Shared-Engine Registry](BASTION_SHARED_ENGINE_REGISTRY.md).

## Operating rules

- One row is one revertible tag-level block. Staged ranges execute internally in their documented order.
- Never begin a later row before the current row is green, reviewed, recorded, and tagged.
- `CHEAP` = predominantly wiring shipped Veloren behavior; `MIXED` = reuse plus a bounded mechanism;
  `NEW` = a new simulation/data engine; `SPEC` = obtain a builder-complete JIT design when reached.
- Folded IDs are not duplicated: FARM/COOK → PRODUCTION; ORDERS/STANDING → POLICY; LOG → HIST;
  TRAP/OPERABLE → MECH.

## Strict build order

| # | Block | Outcome | Status | Class | Design reference |
|---:|---|---|---|---|---|
| 1 | B0 | Fork baseline and deterministic headless harness. | DONE | CHEAP | [Architecture §2.1](BASTION_ARCHITECTURE.md) |
| 2 | B1 | Orthographic overseer camera and Z-slice. | DONE | CHEAP | [Build report §B1](veloren-colony-rts-build-report.md) |
| 3 | B1.5 | Overseer input context and god-camera controls. | DONE | CHEAP | [Architecture §2.2](BASTION_ARCHITECTURE.md) |
| 4 | B1.6 | Occlusion/reveal/slice/underground framework. | DONE | MIXED | [Architecture §2.2](BASTION_ARCHITECTURE.md) |
| 5 | B1.7 | Overseer LOD and frustum tuning. | DONE | CHEAP | [Architecture §2.2](BASTION_ARCHITECTURE.md) |
| 6 | B2a | Selection, designation paint, and authoritative interaction channel. | DONE | CHEAP | [Architecture §2.3](BASTION_ARCHITECTURE.md) |
| 7 | B3 | Colonist entity model, skills, and god-anchor colony. | DONE | MIXED | [Architecture §2.4](BASTION_ARCHITECTURE.md) |
| 8 | B4 | Designation → job board → claim/arbitration/path request. | DONE | NEW | [Architecture §2.5](BASTION_ARCHITECTURE.md) |
| 9 | B5 | Mine/Chop/Build execution, drops, and XP. | DONE | MIXED | [Architecture §2.6](BASTION_ARCHITECTURE.md) |
| 10 | B5.5 | Zone deletion and conserved item-pile aggregation. | DONE | MIXED | [Architecture §2.7](BASTION_ARCHITECTURE.md) |
| 11 | B5.6a | Draped zone visuals, visibility, and pile tiers. | DONE | CHEAP | [Architecture §2.8](BASTION_ARCHITECTURE.md) |
| 12 | B5.6b-1 | Colored zone fills, labels, and blending. | DONE | CHEAP | [Architecture §2.9](BASTION_ARCHITECTURE.md) |
| 13 | B-MAP1 | Map/navigation surface. | DONE | CHEAP | [Architecture §2.10](BASTION_ARCHITECTURE.md) |
| 14 | B5.6b-2 | Volumetric zone extent and depth UX. | DONE | MIXED | [Architecture §2.9](BASTION_ARCHITECTURE.md) |
| 15 | B5.8 | Top-down mining, climbing, and autonomous stair/ladder access. | DONE | MIXED | [Architecture §2.10b](BASTION_ARCHITECTURE.md) |
| 16 | B5.6b-2.1 | Flat-floor mining and anti-entombment/egress cluster. | DONE | MIXED | [Architecture §6](BASTION_ARCHITECTURE.md) |
| 17 | TIMECTL | Pause and variable-rate simulation controls. | DONE | CHEAP | [Architecture §2.10c](BASTION_ARCHITECTURE.md) |
| 18 | TOOL0 | Equipped-tool factor in work speed. | DONE | CHEAP | [Architecture §2.10d](BASTION_ARCHITECTURE.md) |
| 19 | B-ASSET1 | Asset-integration harness and render arena. | DONE | MIXED | [Architecture §2.11](BASTION_ARCHITECTURE.md) |
| 20 | SCCACHE | Shared compiler-cache infrastructure. | DONE | CHEAP | [Architecture §6](BASTION_ARCHITECTURE.md) |
| 21 | B6-SOFT | General colonist soft-collision and chokepoint recovery. | DONE | NEW | [Architecture §2.10e](BASTION_ARCHITECTURE.md) |
| 22 | AR2 | Soft-collision density/teleport hardening. | DONE | MIXED | [Architecture §6](BASTION_ARCHITECTURE.md) |
| 23 | LADDEROFF | Ladder and mine-oscillation live-fix bundle. | DONE | MIXED | [Architecture §2.10f](BASTION_ARCHITECTURE.md) |
| 24 | SLOPE | Slope mining plus standability-gated claiming. | DONE | MIXED | [Architecture §2.10g](BASTION_ARCHITECTURE.md) |
| 25 | CAVEIN | Bounded cave-in with entombment-safe ejection. | DONE | NEW | [Architecture §2.10h](BASTION_ARCHITECTURE.md) |
| 26 | NIGHTHORROR | Flagship creature and reusable integration pipeline. | DONE | MIXED | [Night Horror](NIGHT-HORROR-INTEGRATION-design.md) |
| 27 | CHOP | Whole-tree felling without floating canopy. | DONE | MIXED | [Chop redesign](CHOP-REDESIGN-design.md) |
| 28 | COORD | Stigmergic crew saturation/dispersion field. | DONE | NEW | [Coordination](COLONIST-COORDINATION-design.md) |
| 29 | DETRNG | Deterministic harness RNG and invariant gates. | DONE | MIXED | [Fleet status §BUILD LANE](FLEET_STATUS.md) |
| 30 | CASE-003 | Terrain-aware soft push and center-safety net. | CURRENT | MIXED | [Fleet status §BUILD LANE](FLEET_STATUS.md) |
| 31 | FR15-TIGHTDIG | Close tight-dig stance/reposition/depth locomotion as one class. | TODO | MIXED | [Build Review Log §FR15](BUILD_REVIEW_LOG.md) |
| 32 | LOD-0 | Save back skills, inventory, and later needs. | TODO | CHEAP | [LOD §12](LOD-PERSISTENCE-SPEC.md) |
| 33 | LOD-1 | Atomic loaded↔simulated transition and dupe guard. | TODO | NEW | [LOD §12](LOD-PERSISTENCE-SPEC.md) |
| 34 | B6-HAUL + JOB-CORE | Typed jobs, stockpiles, auto-haul, and reservations. | TODO | MIXED | [Build report §B6](veloren-colony-rts-build-report.md) |
| 35 | B-AG1 | Loaded NPCs continue their rtsim lives. | TODO | CHEAP | [Build report §B-AG1](veloren-colony-rts-build-report.md) |
| 36 | B-AG5-CORE | Canonical world-action helpers and drive vocabulary. | TODO | MIXED | [Build report §B-AG5](veloren-colony-rts-build-report.md) |
| 37 | ZONE-0 | Lock `ZoneKind` and the one soft-magnet mechanism. | TODO | NEW | [DF-ZONES §ZONE-0](DF-ZONES-design.md) |
| 38 | GATHER | Paint Gather, collect wild food, and haul it to stock. | TODO | CHEAP | [DF-ZONES §§2, ZONE-1](DF-ZONES-design.md) |
| 39 | HIST-0 | Lock the Chronicle store and universal `record()` API. | TODO | NEW | [DF-HIST §HIST-0](DF-HIST-design.md) |
| 40 | B-AG2 | Data-driven archetype/race behavior over one brain. | TODO | MIXED | [Build report §B-AG2](veloren-colony-rts-build-report.md) |
| 41 | B-AG3 | Personality, memory, thought, emotion, and mood. | TODO | NEW | [Build report §B-AG3](veloren-colony-rts-build-report.md) |
| 42 | SEASON-0..2 | Deterministic annual clock and consumer interface. | TODO | CHEAP | [Season Clock](SEASON-CLOCK-design.md) |
| 43 | FOCUS-0 | Lock the shared personal `Need` vocabulary. | TODO | NEW | [DF-FOCUS §FOCUS-0](DF-FOCUS-design.md) |
| 44 | B7 | Needs decay, eating/sleeping, mood, and sensible idle. | TODO | NEW | [Build report §B7](veloren-colony-rts-build-report.md) |
| 45 | PATH-0 | Deterministic global path budget/scheduler. | TODO | NEW | [Pathfinding §PATH-0](PATHFINDING-SCALE-SPEC.md) |
| 46 | FARM / PROD-2 | Renewable plots, seeds, growth, tend, and harvest. | TODO | MIXED | [Production §PROD-2](DF-PRODUCTION-design.md) |
| 47 | RUN-0..2 | Energy-governed emergency running and flee burst. | TODO | CHEAP | [Emergency Run](COLONIST-EMERGENCY-RUN-design.md) |
| 48 | AUTON-0 | Single activity-authority arbiter. | TODO | NEW | [Autonomy §AUTON-0](AUTONOMY-ARBITRATION-SPEC.md) |
| 49 | AUTON-1 | Zero-input Mine/Gather/Farm/Haul/Build generation. | TODO | NEW | [Autonomy §AUTON-1](AUTONOMY-ARBITRATION-SPEC.md) |
| 50 | AUTON-2 | Survive/SelfNeed drives and shortage recovery. | TODO | NEW | [Autonomy §AUTON-2](AUTONOMY-ARBITRATION-SPEC.md) |
| 51 | AUTON-3 | Trait urgency, policy tilt, and decision legibility. | TODO | MIXED | [Autonomy §AUTON-3](AUTONOMY-ARBITRATION-SPEC.md) |
| 52 | B8 | Autonomous muster/fight/flee and bounded draft. | TODO | MIXED | [Build report §B8](veloren-colony-rts-build-report.md) |
| 53 | HAZ-0..3 | Shared acute hazard, attribution, and mind reaction. | TODO | MIXED | [Hazard Events](HAZARD-EVENTS-design.md) |
| 54 | HIST-1..2 | Event capture and live Chronicle/announcement feed. | TODO | CHEAP | [DF-HIST §HIST-1/2](DF-HIST-design.md) |
| 55 | GH-A | Renderable hand, navigation, and selection. | TODO | MIXED | [God Hand Integration §showpiece](GOD-HAND-INTEGRATION-design.md) |
| 56 | B13-W0 / POWER-0 | Favor economy, targeting, and authoritative cast pipeline. | TODO | NEW | [Powers Wave 0](GOD-POWERS-CATALOG.md) |
| 57 | GH-B | Costed grab/carry/drop/throw/stroke/slap/tap. | TODO | MIXED | [God Hand §GH-B](GOD-HAND-design.md) |
| 58 | B13-W1 / POWER-1 | Smite, Bless, Weather, and resource-seed powers. | TODO | CHEAP | [Powers Wave 1](GOD-POWERS-CATALOG.md) |
| 59 | GH-C | Cast, paint, sculpt, and divine VFX. | TODO | MIXED | [God Hand §GH-C](GOD-HAND-design.md) |
| 60 | GH-D | Deed-earned alignment morph and aura. | TODO | MIXED | [God Hand §GH-D](GOD-HAND-design.md) |
| 61 | B9 / UI-PLATFORMS | Overlay, dashboard, Chronicle reader, alerts, and policy/meta surfaces. | TODO | MIXED | [Build report §B9](veloren-colony-rts-build-report.md) |
| 62 | B-AG4 / UI-4 | Unit inspector, event box, and per-agent history. | TODO | MIXED | [UI Dialogue/Selection](UI-DIALOGUE-SELECTION-design.md) |
| 63 | OBJ-0..2 | Object live data, lore/history, and context actions. | TODO | CHEAP | [Object Inspection](UI-OBJECT-INSPECTION-design.md) |
| 64 | B10 | Save/load colony state, work, stock, and blueprints. | TODO | MIXED | [Build report §B10](veloren-colony-rts-build-report.md) |
| 65 | LOD-2 | Conservative abstract colony resolve while unwatched. | TODO | NEW | [LOD §LOD-2](LOD-PERSISTENCE-SPEC.md) |
| 66 | LOD-3 | Partial-load reconciliation and no-dupe/no-loss soak. | TODO | NEW | [LOD §LOD-3](LOD-PERSISTENCE-SPEC.md) |
| 67 | WORLDGEN-TUNE | Starter veins and cave/ore density tuning before embark. | TODO | CHEAP | [Caves/Ore Investigation](WORLDGEN-CAVES-ORE-density-investigation.md) |
| 68 | B11 | World survey, embark selection, and starting colony. | TODO | MIXED | [Founding §7](FOUNDING-EMBARK-DESIGN.md) |
| 69 | B12 | Embody/release through Veloren possession with one controller. | TODO | CHEAP | [Build report §B12](veloren-colony-rts-build-report.md) |
| 70 | PLAYER-MODES | Sovereign/Watcher/Incarnate faces over one control sink. | TODO | CHEAP | [Player Modes](PLAYER-MODES-design.md) |
| 71 | GOD-DOMAIN | Invoked domain with deed-derived moral face. | TODO | MIXED | [God Domain §§6–8](GOD-DOMAIN-design.md) |
| 72 | GH-E | Hand/control-spectrum transition into Embody. | TODO | CHEAP | [God Hand §GH-E](GOD-HAND-design.md) |
| 73 | B13-W2 / POWER-2 | Panic, curses, wards, and field blessing/blight. | TODO | MIXED | [Powers Wave 2](GOD-POWERS-CATALOG.md) |
| 74 | DIG-0..4 | Channel/ramp/stairwell/up-stair verbs over one safe decomposer. | TODO | MIXED | [DF Dig Verbs](DF-DIG-VERBS-design.md) |
| 75 | BUILD-0..3 | Construction spine, placement types, vertical build, and volume UI. | TODO | NEW | [Build Framework §BUILD-0..3](BUILD-FRAMEWORK-design.md) |
| 76 | BUILD-4..6 | Parameterized/composed structures and autonomous plans. | TODO | MIXED | [Build Framework §BUILD-4..6](BUILD-FRAMEWORK-design.md) |
| 77 | PROD-0 | Station-based Produce job over existing recipes. | TODO | MIXED | [Production §PROD-0](DF-PRODUCTION-design.md) |
| 78 | PROD-1 + POL-0 | One quota/order engine and stock census. | TODO | NEW | [Production §PROD-1](DF-PRODUCTION-design.md) |
| 79 | PROD-3 / COOK | Cooking, brewing, and seed-protection policy. | TODO | MIXED | [Production §PROD-3](DF-PRODUCTION-design.md) |
| 80 | QUAL-0..1 | Craft-quality stamp, value, and thought payoff. | TODO | MIXED | [DF Quality](DF-QUALITY-design.md) |
| 81 | TOOL-1..2 | Tool material tiers, hard-material gates, quality, auto-equip. | TODO | MIXED | [Tools Upgrade](TOOLS-UPGRADE-design.md) |
| 82 | PROD-4..5 | Economy legibility, aggregate LOD, and animations. | TODO | MIXED | [Production §PROD-4/5](DF-PRODUCTION-design.md) |
| 83 | QUAL-2..3 / ARTIFACT | Strange mood, artifact, and faithful failure tragedy. | TODO | NEW | [DF Quality §QUAL-2/3](DF-QUALITY-design.md) |
| 84 | ROT-0..3 | Conserved freshness/rot, miasma, hygiene, and vermin. | TODO | NEW | [DF Rot](DF-ROT-design.md) |
| 85 | ZONE-1-REST | Finish Refuse behavior; Gather already proves its wire. | TODO | CHEAP | [DF Zones §ZONE-1](DF-ZONES-design.md) |
| 86 | POL-1..4 | Standing rules, policy panel, LOD, and rule adapters. | TODO | MIXED | [DF Policy](DF-POLICY-design.md) |
| 87 | BURROW-0..3 | Hard restriction, survival escape, and shelter alert. | TODO | MIXED | [DF Burrow](DF-BURROW-design.md) |
| 88 | ROOM-0..3 | Enclosure, room role/value, ownership, and mind payoff. | TODO | NEW | [DF Rooms](DF-ROOMS-design.md) |
| 89 | FOCUS-1..3 | Personal-need jobs and focus-driven performance. | TODO | NEW | [DF Focus](DF-FOCUS-design.md) |
| 90 | REL-0..3 | Temples, worship need, priests, and devotion. | TODO | MIXED | [DF Religion](DF-RELIGION-design.md) |
| 91 | DF-TAVERN | Shared recreation/congregation venue loop. | TODO | SPEC | [DF Tavern stub](DF-TAVERN-design.md) |
| 92 | FEST-0..2 | Scheduled/event-driven festivals and feasts. | TODO | CHEAP | [DF Festival](DF-FESTIVAL-design.md) |
| 93 | STOCK-0..3 | Pasture, products, breeding, and bounded herd population. | TODO | MIXED | [DF Livestock](DF-LIVESTOCK-design.md) |
| 94 | TRADE-0..6 | Colony economy node, depot, caravan, and autonomous trade. | TODO | MIXED | [DF Trade](DF-TRADE-design.md) |
| 95 | MIG-0..3 | Prestige migration/emigration plus paired threat pressure. | TODO | MIXED | [DF Migration](DF-MIGRATION-design.md) |
| 96 | JUST-0..3 | Hierarchy, mandates, crime/justice, and divine reach-ins. | TODO | MIXED | [DF Justice](DF-JUSTICE-design.md) |
| 97 | B-AG6 | Settlement growth, reproduction, and genealogy. | TODO | NEW | [Build report §B-AG6](veloren-colony-rts-build-report.md) |
| 98 | PATH-1..3 | Hierarchical paths, shared flow fields, and repair. | TODO | NEW | [Pathfinding §PATH-1..3](PATHFINDING-SCALE-SPEC.md) |
| 99 | CAVERN-0..4 / GEO-1 | Danger tiers, prospecting, ore veins, breach, and deep life. | TODO | MIXED | [Cavern/Geology](DF-CAVERN-GEOLOGY-design.md) |
| 100 | LIGHT-0..3 | Carried/placed light and bounded gameplay darkness. | TODO | MIXED | [Underground Lighting](UNDERGROUND-LIGHTING-design.md) |
| 101 | UNDERGROUND-UX | Camera targets, atmosphere, and cursor-hand light. | TODO | CHEAP | [Underground Experience](UNDERGROUND-EXPERIENCE-design.md) |
| 102 | STRUCT-1 | Mine supports as cave-in prevention. | TODO | CHEAP | [DF Struct](DF-STRUCT-design.md) |
| 103 | MECH-0..3 | Trigger→link→effect for traps and operable terrain. | TODO | NEW | [DF Mech](DF-MECH-design.md) |
| 104 | POW-0..1 | Mechanical power graph and powered machines. | TODO | NEW | [DF Power](DF-POWER-design.md) |
| 105 | FLUID-0..3 | Bounded experimental flow with static fallback. | TODO | NEW | [DF Fluid](DF-FLUID-design.md) |
| 106 | TEMP-0..2 | Felt climate, winter answers, and biome effects. | TODO | MIXED | [Temperature/Biome](DF-TEMP-BIOME-FX-design.md) |
| 107 | SYN-0..3 | Data-driven syndromes and bounded epidemiology. | TODO | MIXED | [DF Syndrome](DF-SYNDROME-design.md) |
| 108 | DF-WOUND / MEDICAL | Persistent injury, infection, healer jobs, and recovery. | TODO | SPEC | [DF Wound stub](DF-WOUND-design.md) |
| 109 | DF-MILITARY / RANGED | Policy squads, equipment, training, patrols, and ranged defense. | TODO | SPEC | [Gap ledger §G](df-feature-gap-ledger.md) |
| 110 | NIGHT-0..2 | Systemic night pressure, light wards, and shelter/defense response. | TODO | MIXED | [DF Night](DF-NIGHT-design.md) |
| 111 | VILLAIN-0..2 | Persistent named nemeses with bounded escalation and closure. | TODO | MIXED | [DF Villain](DF-VILLAIN-design.md) |
| 112 | MISS-0..2 | Off-map missions resolved through rtsim quests. | TODO | MIXED | [DF Mission](DF-MISSION-design.md) |
| 113 | REP-0..2 | Chronicle deeds become live decaying social reputation. | TODO | CHEAP | [Reputation](REPUTATION-design.md) |
| 114 | AGENT-CULTURE | Race/culture-keyed mind, language, history, and relations. | TODO | MIXED | [Agent Culture](AGENT-CULTURE-CHARACTERIZATION-design.md) |
| 115 | HIST-3..6 | Legends browser, world retention, attribution, and epochs. | TODO | MIXED | [DF Hist](DF-HIST-design.md) |
| 116 | ART-0..2 | Art depicts real history and affects minds/rooms/culture. | TODO | MIXED | [DF Art](DF-ART-design.md) |
| 117 | KNOW-0..2 | Discover, teach, record, and potentially lose techniques. | TODO | NEW | [DF Knowledge](DF-KNOWLEDGE-design.md) |
| 118 | HALLOW-0..2 | Significant places become sacred or haunted. | TODO | MIXED | [Sacred Sites](SACRED-SITES-design.md) |
| 119 | EPITHET-0..2 | Colony names its god from recorded deeds and alignment. | TODO | CHEAP | [God Epithet](GOD-EPITHET-design.md) |
| 120 | OMEN-0..2 | Signs receive faith/culture/epithet interpretation. | TODO | MIXED | [DF Omen](DF-OMEN-design.md) |
| 121 | CHAMP-0..2 | Anointed champion with sainthood or fall. | TODO | MIXED | [Divine Champion](DIVINE-CHAMPION-design.md) |
| 122 | DEAD-0..2 | Ancestors and honored/restless dead. | TODO | MIXED | [DF Ancestors](DF-ANCESTORS-design.md) |
| 123 | CURSE-0..2 | Liftable divine syndromes and geasa. | TODO | MIXED | [DF Curse](DF-CURSE-design.md) |
| 124 | BEAST-0..2 | Persistent legendary megafauna and great hunts. | TODO | MIXED | [DF Beast](DF-BEAST-design.md) |
| 125 | RENOWN-0..2 | Derived colony legend/byname and bounded world response. | TODO | CHEAP | [Collective Renown](COLLECTIVE-RENOWN-design.md) |
| 126 | RECLAIM-0..2 | Fallen colonies persist as storied reclaimable ruins. | TODO | MIXED | [DF Reclaim](DF-RECLAIM-design.md) |
| 127 | GH-F | Remembering-colony hand enrichment. | TODO | MIXED | [God Hand §GH-F](GOD-HAND-design.md) |
| 128 | B13-W3 / POWER-3 / REL-4 | Late engines plug into the full god-power catalog. | TODO | MIXED | [Powers Wave 3](GOD-POWERS-CATALOG.md) |
| 129 | DP1..DP5 | Faction interests, faith politics, and competing gods. | TODO | NEW | [Divine Politics §6](divine-politics-bible.md) |
| 130 | NAVAL-0..3 | Sailing AI, vessels, harbors, and abstract trade routes. | TODO | MIXED | [Ships/Naval](SHIPS-NAVAL-design.md) |
| 131 | DF-PUMP / MAGMA / HYDRO | Fluid machinery, lava, aquifer, and flood engineering. | TODO | SPEC | [Fluid §FLUID-2/3](DF-FLUID-design.md) |
| 132 | DF-ECON / GUILD | Deep wealth/property and profession guilds. | TODO | SPEC | [Gap ledger §D](df-feature-gap-ledger.md) |
| 133 | DF-MINECART | Advanced hauling routes, carts, and rollers. | TODO | SPEC | [Gap ledger §E](df-feature-gap-ledger.md) |
| 134 | DF-PREF | Individual likes/dislikes feeding mind and culture. | TODO | SPEC | [Gap ledger §B](df-feature-gap-ledger.md) |
| 135 | DF-BOOKS / DF-NOTES | Accounting accuracy, annotations, and patrol routes. | TODO | SPEC | [Gap ledger §K](df-feature-gap-ledger.md) |
| 136 | AUTON-FULL-SOAK | Full founded colony survives, grows, acts, and remembers with zero input. | TODO | NEW | [Autonomous Colony §6 AUTO-3](AUTONOMOUS-COLONY-OPERATION-design.md) |

## Approved ordering notes

- B6’s tagged soft-collision work and its unbuilt hauling objective are explicitly split.
- NIGHTHORROR proves creature integration; systemic DF-NIGHT remains later.
- `LOD-0/1 → B6-HAUL → B-AG5 → ZONE-0 → GATHER` closes integrity and physical food acquisition before autonomy.
- `B7 → FARM → AUTON-0..3` supplies demand and renewable food before shortage-recovery claims.
- HIST-0 and FOCUS-0 lock shared schemas before downstream emitters/consumers fork them.
- PROD-1 and POL-0 co-own one order evaluator and stock census.
- GH-A is independent; GH-B onward follows the B13 cast/favor substrate; GH-E waits for B12; GH-F waits for depth.
- SPEC rows retain their approved position but receive a builder-complete JIT design only when reached.

**Approved Gather-vs-AUTON call:** acquire food first, make it renewable second, then claim autonomy.
