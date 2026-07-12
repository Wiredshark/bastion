# Project Bastion — Master Design Index

> Resolver for the [Approved Master Build List](BASTION_MASTER_BUILD_LIST.md). Each entry states the owning
> design, purpose, and principal dependency/gate. Implementation direction is in
> [BASTION_IMPLEMENTATION_PLAYBOOK.md](BASTION_IMPLEMENTATION_PLAYBOOK.md).

## Completed and current foundation

- **B0** — [Architecture §2.1](BASTION_ARCHITECTURE.md) — baseline/harness; gate: deterministic headless startup.
- **B1** — [Build report §B1](veloren-colony-rts-build-report.md) — overseer camera; depends B0.
- **B1.5** — [Architecture §2.2](BASTION_ARCHITECTURE.md) — input contexts; depends B1.
- **B1.6** — [Architecture §2.2](BASTION_ARCHITECTURE.md) — occlusion/view framework; depends B1.
- **B1.7** — [Architecture §2.2](BASTION_ARCHITECTURE.md) — LOD/frustum polish; depends B1.6.
- **B2a** — [Architecture §2.3](BASTION_ARCHITECTURE.md) — selection/designation surface; depends camera/input.
- **B3** — [Architecture §2.4](BASTION_ARCHITECTURE.md) — colony actors; depends B2a.
- **B4** — [Architecture §2.5](BASTION_ARCHITECTURE.md) — job board; depends B3.
- **B5** — [Architecture §2.6](BASTION_ARCHITECTURE.md) — work execution; depends B4.
- **B5.5** — [Architecture §2.7](BASTION_ARCHITECTURE.md) — deletion/piles; depends B5.
- **B5.6a** — [Architecture §2.8](BASTION_ARCHITECTURE.md) — spatial legibility; depends zones.
- **B5.6b-1** — [Architecture §2.9](BASTION_ARCHITECTURE.md) — zone UI foundation; depends B5.6a.
- **B-MAP1** — [Architecture §2.10](BASTION_ARCHITECTURE.md) — map/navigation; depends camera.
- **B5.6b-2** — [Architecture §2.9](BASTION_ARCHITECTURE.md) — volume/depth UI; depends b-1.
- **B5.8** — [Architecture §2.10b](BASTION_ARCHITECTURE.md) — vertical access; depends B5.
- **B5.6b-2.1** — [Architecture §6](BASTION_ARCHITECTURE.md) — anti-stuck mining; depends B5.8.
- **TIMECTL** — [Architecture §2.10c](BASTION_ARCHITECTURE.md) — time control; depends simulation tick.
- **TOOL0** — [Architecture §2.10d](BASTION_ARCHITECTURE.md) — initial tool factor; depends B5.
- **B-ASSET1** — [Architecture §2.11](BASTION_ARCHITECTURE.md) — asset acceptance pipeline.
- **SCCACHE** — [Architecture §6](BASTION_ARCHITECTURE.md) — build infrastructure.
- **B6-SOFT** — [Architecture §2.10e](BASTION_ARCHITECTURE.md) — crowd movement; depends B5.8.
- **AR2** — [Architecture §6](BASTION_ARCHITECTURE.md) — soft-collision hardening.
- **LADDEROFF** — [Architecture §2.10f](BASTION_ARCHITECTURE.md) — ladder/mine fixes.
- **SLOPE** — [Architecture §2.10g](BASTION_ARCHITECTURE.md) — slope/standability.
- **CAVEIN** — [Architecture §2.10h](BASTION_ARCHITECTURE.md) — collapse/safe eject.
- **NIGHTHORROR** — [Night Horror Integration](NIGHT-HORROR-INTEGRATION-design.md) — creature pipeline reference.
- **CHOP** — [Chop Redesign](CHOP-REDESIGN-design.md) — tree-volume felling.
- **COORD** — [Coordination](COLONIST-COORDINATION-design.md) — stigmergic crew division.
- **DETRNG** — [Fleet Status §BUILD LANE](FLEET_STATUS.md) — deterministic gate substrate.
- **CASE-003** — [Fleet Status §BUILD LANE](FLEET_STATUS.md) — current terrain-safety correction; blocks forward work.

## Survival and autonomy spine

- **FR15-TIGHTDIG** — [Build Review Log §FR15](BUILD_REVIEW_LOG.md) — remaining work-stance/locomotion class; depends CASE-003.
- **LOD-0** — [LOD §12](LOD-PERSISTENCE-SPEC.md) — state save-back; prevents XP/inventory/needs loss.
- **LOD-1** — [LOD §12](LOD-PERSISTENCE-SPEC.md) — atomic mode transition; prevents double simulation.
- **B6-HAUL + JOB-CORE** — [Build report §B6](veloren-colony-rts-build-report.md) — logistics and typed work; depends piles/jobs.
- **B-AG1** — [Build report §B-AG1](veloren-colony-rts-build-report.md) — loaded fidelity; depends rtsim promotion.
- **B-AG5-CORE** — [Build report §B-AG5](veloren-colony-rts-build-report.md) — shared action vocabulary; Gather is proof caller.
- **ZONE-0** — [DF-ZONES §ZONE-0](DF-ZONES-design.md) — canonical zone schema/magnet; precedes behavior wires.
- **GATHER** — [DF-ZONES §§2, ZONE-1](DF-ZONES-design.md) — immediate food acquisition; depends Haul, actions, Zone core.
- **HIST-0** — [DF-HIST §HIST-0](DF-HIST-design.md) — Chronicle API lock; precedes new emitters.
- **B-AG2** — [Build report §B-AG2](veloren-colony-rts-build-report.md) — archetype behavior over shared brain.
- **B-AG3** — [Build report §B-AG3](veloren-colony-rts-build-report.md) — mind substrate for mood/hazards/morality.
- **SEASON-0..2** — [Season Clock](SEASON-CLOCK-design.md) — shared annual rhythm; precedes farm/festival/climate.
- **FOCUS-0** — [DF-FOCUS §FOCUS-0](DF-FOCUS-design.md) — `NeedKind` lock; precedes B7 consumers.
- **B7** — [Build report §B7](veloren-colony-rts-build-report.md) — food demand/consumption and mood; depends Haul/Gather.
- **PATH-0** — [Pathfinding §PATH-0](PATHFINDING-SCALE-SPEC.md) — bounded deterministic path scheduling; ships with B7 scale.
- **FARM / PROD-2** — [Production §PROD-2](DF-PRODUCTION-design.md) — renewable food; depends B7 demand and Gather baseline.
- **RUN-0..2** — [Emergency Run §4](COLONIST-EMERGENCY-RUN-design.md) — urgency gait; precedes defense crises.
- **AUTON-0** — [Autonomy §AUTON-0](AUTONOMY-ARBITRATION-SPEC.md) — sole activity authority.
- **AUTON-1** — [Autonomy §AUTON-1](AUTONOMY-ARBITRATION-SPEC.md) — self-generated jobs; depends real verbs/logistics.
- **AUTON-2** — [Autonomy §AUTON-2](AUTONOMY-ARBITRATION-SPEC.md) — needs/death-spiral gate; depends B7 + Farm.
- **AUTON-3** — [Autonomy §AUTON-3](AUTONOMY-ARBITRATION-SPEC.md) — traits/policy/legibility; depends mind.
- **B8** — [Build report §B8](veloren-colony-rts-build-report.md) — autonomous defense; closes autonomy G3.
- **HAZ-0..3** — [Hazard Events](HAZARD-EVENTS-design.md) — shared acute effects/mind attribution; depends mind+B8.
- **HIST-1..2** — [DF-HIST §HIST-1/2](DF-HIST-design.md) — capture/feed; depends HIST-0.

## God-game, UI, persistence, and founding

- **GH-A** — [God Hand Integration §showpiece](GOD-HAND-INTEGRATION-design.md) — independent render/nav/select proof.
- **B13-W0 / POWER-0** — [Power Catalog Wave 0](GOD-POWERS-CATALOG.md), [Dispatch §POWER-0](GOD-POWERS-DISPATCH-SPEC.md) — favor/cast substrate.
- **GH-B** — [God Hand §GH-B](GOD-HAND-design.md) — physical touch; depends favor and safe throw/drop.
- **B13-W1 / POWER-1** — [Power Catalog Wave 1](GOD-POWERS-CATALOG.md) — first useful god; depends autonomy/food consumers.
- **GH-C** — [God Hand §GH-C](GOD-HAND-design.md) — cast/paint/sculpt presentation.
- **GH-D** — [God Hand §GH-D](GOD-HAND-design.md) — alignment mirror; depends attributed Chronicle deeds.
- **B9 / UI-PLATFORMS** — [Build report §B9](veloren-colony-rts-build-report.md), [UI Audit](UI-MISSING-ELEMENTS-audit.md) — one UI/legibility platform.
- **B-AG4 / UI-4** — [Build report §B-AG4](veloren-colony-rts-build-report.md), [Dialogue/Selection](UI-DIALOGUE-SELECTION-design.md) — inspector/history.
- **OBJ-0..2** — [Object Inspection](UI-OBJECT-INSPECTION-design.md) — reuses B2a, UI shell, and HIST.
- **B10** — [Build report §B10](veloren-colony-rts-build-report.md) — durable colony persistence.
- **LOD-2** — [LOD §LOD-2](LOD-PERSISTENCE-SPEC.md) — abstract progression; depends autonomy+B7.
- **LOD-3** — [LOD §LOD-3](LOD-PERSISTENCE-SPEC.md) — reconciliation/partial-load soak; depends LOD-0..2.
- **WORLDGEN-TUNE** — [Caves/Ore Investigation](WORLDGEN-CAVES-ORE-density-investigation.md) — site differences before embark.
- **B11** — [Founding §7](FOUNDING-EMBARK-DESIGN.md) — embark; depends persistence/world differences.
- **B12** — [Build report §B12](veloren-colony-rts-build-report.md) — possession reuse; depends B7/autonomy handoff.
- **PLAYER-MODES** — [Player Modes](PLAYER-MODES-design.md) — face-switch UX; depends B12.
- **GOD-DOMAIN** — [God Domain §§6–8](GOD-DOMAIN-design.md) — domain catalog; depends functioning powers.
- **GH-E** — [God Hand §GH-E](GOD-HAND-design.md) — hand-to-Embody transition; depends B12.
- **B13-W2 / POWER-2** — [Power Catalog Wave 2](GOD-POWERS-CATALOG.md) — polished condition powers; depends mind/defense/farms.

## Production, society, and the deep

- **DIG-0..4** — [DF Dig Verbs §6](DF-DIG-VERBS-design.md) — explicit excavation vocabulary atop B5.8.
- **BUILD-0..3** — [Build Framework §BUILD-0..3](BUILD-FRAMEWORK-design.md) — construction core/vertical placement.
- **BUILD-4..6** — [Build Framework §BUILD-4..6](BUILD-FRAMEWORK-design.md) — richer/autonomous structures.
- **PROD-0** — [Production §PROD-0](DF-PRODUCTION-design.md) — station work; depends Haul.
- **PROD-1 + POL-0** — [Production §PROD-1](DF-PRODUCTION-design.md), [Policy §POL-0](DF-POLICY-design.md) — one order engine/census.
- **PROD-3 / COOK** — [Production §PROD-3](DF-PRODUCTION-design.md) — cook/brew/permissions; depends Farm+Produce.
- **QUAL-0..1** — [DF Quality](DF-QUALITY-design.md) — per-instance quality/payoff; depends production.
- **TOOL-1..2** — [Tools Upgrade](TOOLS-UPGRADE-design.md) — material/quality progression; depends production+quality.
- **PROD-4..5** — [Production §PROD-4/5](DF-PRODUCTION-design.md) — legibility/LOD/animation after full loop.
- **QUAL-2..3 / ARTIFACT** — [DF Quality §QUAL-2/3](DF-QUALITY-design.md) — artifact drama; depends mind+Chronicle.
- **ROT-0..3** — [DF Rot](DF-ROT-design.md) — decay/hygiene; depends food/items/mind.
- **ZONE-1-REST** — [DF Zones §ZONE-1](DF-ZONES-design.md) — Refuse wire; depends Haul+Rot.
- **POL-1..4** — [DF Policy](DF-POLICY-design.md) — standing rules/LOD; depends POL-0.
- **BURROW-0..3** — [DF Burrow](DF-BURROW-design.md) — hard policy/shelter; safe only after B7+B8.
- **ROOM-0..3** — [DF Rooms](DF-ROOMS-design.md) — construction→mind; depends build/quality/rot.
- **FOCUS-1..3** — [DF Focus](DF-FOCUS-design.md) — personal pursuit/performance; depends B7+venues.
- **REL-0..3** — [DF Religion](DF-RELIGION-design.md) — temples/worship/devotion; depends needs/zones.
- **DF-TAVERN** — [DF Tavern stub](DF-TAVERN-design.md) — SPEC gate; depends Cook/Focus/Zones.
- **FEST-0..2** — [DF Festival](DF-FESTIVAL-design.md) — requires Season/Cook/Focus/venue loop.
- **STOCK-0..3** — [DF Livestock](DF-LIVESTOCK-design.md) — husbandry/population; depends zones/production.
- **TRADE-0..6** — [DF Trade](DF-TRADE-design.md) — requires production/hauling/founding/quality.
- **MIG-0..3** — [DF Migration](DF-MIGRATION-design.md) — prestige/population/threat pair; depends B8.
- **JUST-0..3** — [DF Justice](DF-JUSTICE-design.md) — governance; depends migration/policy/mind.
- **B-AG6** — [Build report §B-AG6](veloren-colony-rts-build-report.md) — growth/genealogy; depends persistence/needs/economy.
- **PATH-1..3** — [Pathfinding §PATH-1..3](PATHFINDING-SCALE-SPEC.md) — hierarchy/fields/repair; justified by population.
- **CAVERN-0..4 / GEO-1** — [Cavern/Geology](DF-CAVERN-GEOLOGY-design.md) — requires DIG/Hazard/B8.
- **LIGHT-0..3** — [Underground Lighting](UNDERGROUND-LIGHTING-design.md) — gameplay light; precedes DF-NIGHT.
- **UNDERGROUND-UX** — [Underground Experience](UNDERGROUND-EXPERIENCE-design.md) — consumes camera/light/hand.
- **STRUCT-1** — [DF Struct](DF-STRUCT-design.md) — support prevention atop cave-in effect.
- **MECH-0..3** — [DF Mech](DF-MECH-design.md) — trigger/link/effect; depends Hazard+Build.
- **POW-0..1** — [DF Power](DF-POWER-design.md) — linkage/powered production; depends Mech.
- **FLUID-0..3** — [DF Fluid](DF-FLUID-design.md) — contained experimental solver; static fallback mandatory.
- **TEMP-0..2** — [Temperature/Biome](DF-TEMP-BIOME-FX-design.md) — requires Season and colony answers.
- **SYN-0..3** — [DF Syndrome](DF-SYNDROME-design.md) — chronic effects; depends buffs/mind/hygiene.
- **DF-WOUND / MEDICAL** — [DF Wound stub](DF-WOUND-design.md) — SPEC gate after Syndrome+B8.
- **DF-MILITARY / RANGED** — [Gap ledger §G](df-feature-gap-ledger.md) — SPEC gate after B8/Wound.
- **NIGHT-0..2** — [DF Night](DF-NIGHT-design.md) — requires light/syndrome/B8/burrows.
- **VILLAIN-0..2** — [DF Villain](DF-VILLAIN-design.md) — requires B8/Justice/Migration/Night/Chronicle.
- **MISS-0..2** — [DF Mission](DF-MISSION-design.md) — requires persistent actors/threats/destinations.
- **REP-0..2** — [Reputation](REPUTATION-design.md) — Chronicle read-back after deed systems.
- **AGENT-CULTURE** — [Agent Culture](AGENT-CULTURE-CHARACTERIZATION-design.md) — mind/history/relations content.
- **HIST-3..6** — [DF Hist](DF-HIST-design.md) — browser/world depth after enough emitters exist.
- **ART-0..2** — [DF Art](DF-ART-design.md) — requires Chronicle/Quality/Rooms/Culture.
- **KNOW-0..2** — [DF Knowledge](DF-KNOWLEDGE-design.md) — requires production/tools/trade/mission sources.
- **HALLOW-0..2** — [Sacred Sites](SACRED-SITES-design.md) — requires Chronicle/Religion/Art/place persistence.
- **EPITHET-0..2** — [God Epithet](GOD-EPITHET-design.md) — requires alignment/Chronicle/faith.
- **OMEN-0..2** — [DF Omen](DF-OMEN-design.md) — requires Season/Faith/Culture/Epithet.
- **CHAMP-0..2** — [Divine Champion](DIVINE-CHAMPION-design.md) — requires hand/powers/B8/reputation.
- **DEAD-0..2** — [DF Ancestors](DF-ANCESTORS-design.md) — requires death/history/religion/night/omens.
- **CURSE-0..2** — [DF Curse](DF-CURSE-design.md) — requires Syndrome/Justice/Mission/powers.
- **BEAST-0..2** — [DF Beast](DF-BEAST-design.md) — requires persistent figures/Mission/B8/Livestock.
- **RENOWN-0..2** — [Collective Renown](COLLECTIVE-RENOWN-design.md) — requires prestige+Chronicle.
- **RECLAIM-0..2** — [DF Reclaim](DF-RECLAIM-design.md) — requires B11/B12/ruins/ancestors/sites.
- **GH-F** — [God Hand §GH-F](GOD-HAND-design.md) — full remembering-colony depth after downstream systems.
- **B13-W3 / POWER-3 / REL-4** — [Power Catalog Wave 3](GOD-POWERS-CATALOG.md), [Religion §REL-4](DF-RELIGION-design.md) — late adapters.
- **DP1..DP5** — [Divine Politics §6](divine-politics-bible.md) — requires B8/B13/B-AG3/B-AG6.
- **NAVAL-0..3** — [Ships/Naval](SHIPS-NAVAL-design.md) — requires trade/routes/persistence/water substrate.
- **DF-PUMP / MAGMA / HYDRO** — [Fluid §FLUID-2/3](DF-FLUID-design.md), [ledger §§E–F](df-feature-gap-ledger.md) — SPEC gate after viable Fluid.
- **DF-ECON / GUILD** — [ledger §D](df-feature-gap-ledger.md) — SPEC gate after economy stabilizes.
- **DF-MINECART** — [ledger §E](df-feature-gap-ledger.md) — SPEC gate after logistics/path scale.
- **DF-PREF** — [ledger §B](df-feature-gap-ledger.md) — SPEC gate after mind/food/art/rooms.
- **DF-BOOKS / DF-NOTES** — [ledger §K](df-feature-gap-ledger.md) — final low-cost interface/depth.
- **AUTON-FULL-SOAK** — [Autonomous Colony §6 AUTO-3](AUTONOMOUS-COLONY-OPERATION-design.md) — final composed acceptance.
