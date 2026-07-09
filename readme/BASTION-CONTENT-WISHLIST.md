# BASTION MASTER CONTENT WISHLIST — assets, animations, systems (exhaustive)

**How to read this:** every entry tags what it needs — **[A]** asset (generatable NOW by the pipeline),
**[AN-N]** animation-native (state+tool reuse), **[AN-C]** animation-custom (new Animation impl, code),
**[OP]** operable part (Phase-1 operable framework), **[SYS:x]** gated on system x (inert until built).
The asset sessions draw generation batches from here (READY items freely; NEEDS items when their system
lands). Zone-compatible `purpose` tags per the shared taxonomy. Lore for every generated asset, always.

---

## 1. COLONY ECONOMY & INDUSTRY
**Workshops (production purpose), per race style-set where sensible:**
- [A] Carpenter, mason, smithy (exists — reuse), smelter/furnace, kiln, tannery, loom/weaver, tailor,
  brewery, bakery/mill, butcher, fishery shack, glassworks, jeweler, fletcher, alchemist/apothecary
  — each with interior WORK POINT (function-harness reachable) [SYS: production-chains for function]
- [AN-C] craft-at-station (hammer-at-anvil, saw, knead, stir) — ONE parameterized station-work anim covers
  most; [AN-N] none native beyond tool swings
- [A] Crafting-station sprites (anvil, workbench, loom, cauldron — census: furniture sprites exist, extend)
**Storage:** [A] granary, warehouse, cellar, silo; stockpile marker posts; crates/barrels/sacks (piles →
containers as stockpile visuals) [SYS: B6 stockpiles — landing now]
**Mining (§3v):** [A] mine-entrance timber frames, shaft headframe, support beams, mine-cart props, ore-vein
sprites per mineral, lanterns; [OP] mine gate; [SYS: mine-template + ore survey]
**Items at breadth:** [A] tools (pick/axe/hoe/hammer/saw/fishing rod), raw materials (ore/ingot/plank/
cloth/leather), foods (bread/meat/fish/ale), furniture (bed/table/chair/shelf — census has ~80, extend per
race), trade goods (bolts of cloth, gem boxes, spice sacks)

## 2. TRADE & TRANSPORT (the cart example, done fully)
- [A] **Cart family:** handcart (exists), horse-drawn cart, heavy wagon, covered merchant wagon — multi-part
  (census: common/voxel vehicles have collider manifests — the substrate EXISTS: airship/cart/galleon)
- [SYS: hitching] horse + cart + **NPC driver** = a composite entity (mount machinery exists in Veloren —
  verify riding→pulling adaptation); [AN-N] horse walk (native quadruped), [AN-C] driver seated-driving pose
- [SYS: caravan-routes] caravans travel roads between settlements (rtsim travel exists for merchants —
  extend with physical cart entity when watched, abstract when not — the X4/Elite two-tier per §3t)
- [A] pack animals: mule with saddlebags, llama; [A] roadside infra: waystone, milestone, signpost, roadside
  shrine, coaching inn (social), toll gate [OP]
- [A] market square set: stalls (produce/cloth/tools), money-changer table, auction post [SYS: trade/market]
- **Naval:** [A] rowboat, fishing skiff, river barge, cog/trader, warship (sail_boat/galleon exist — extend);
  [A] dock/pier set, boathouse, lighthouse, harbor crane [OP]; [SYS: naval-movement — the big gate];
  [AN-C] rowing, sail-handling
## 3. AGRICULTURE & HUSBANDRY
- [A] field/furrow tiles, crop sprites per growth stage ×8 crops (census: crops ×12 exist — extend), scarecrow,
  irrigation channel pieces [SYS: farming jobs]; [AN-C] hoe/sow/harvest gestures (one farm-gesture anim set)
- [A] barn (exists), stable, chicken coop, pigsty, apiary, dovecote, pasture fencing + gate [OP], trough,
  hay bales; [SYS: husbandry — breeding/taming/penning on EXISTING animals (sheep/pig/cattle/horse — §3r:
  startable without custom creatures)]; [AN-N] animal locomotion native; [AN-C] milking/shearing gestures
- [A] husbandry species VARIANTS (wool colors, cattle breeds) — cheap variation-pack density

## 4. MILITARY & DEFENSE (B8-era)
- [A] wall sets per race (palisade exists → stone curtain, brick, dwarven), towers (corner/gate/watch),
  [OP] gates (portcullis proven), drawbridge [OP], murder-holes, hoardings; siege-ready crenel components
- [A] barracks, armory, training yard (dummy + archery butts), guardhouse, beacon tower (signal fire — ties
  alerts); [A] weapon/armor breadth per race+tier (781 weapons exist — fill gaps, boss/artifact uniques)
- [SYS: trap engine = trigger→link→effect] [A]+[OP] traps: pit (cover panel), spike panel (proven in ceiling
  test), swinging log, tripwire+alarm bell, caltrops; [A] lever/pressure-plate/winch marker sprites
- [AN-N] all combat native; [AN-C] patrol-stand-guard idle, alarm-ringing
- [SYS: siege] siege engines: ram, catapult, ballista, siege tower [A, multi-part, OP arms] — far-tier

## 5. FAITH & THE DIVINE
- [A] shrine set (wayside → chapel → temple → grand temple) PER GOD ASPECT (war/harvest/sea/death…), altar
  sprites, idol statues, offering bowls, prayer flags, censers (Vaultback's censer reusable), reliquaries
- [A] monastery components (Skyreach set reusable!), bell tower [OP bell], graveyard set (graves exist —
  extend: mausoleum, bone-yard, barrow) [SYS: death/burial]
- [AN-C] **worship/prayer set** (kneel, arms-raised, procession walk) — high priority, the faith layer's
  visibility; [AN-C] funeral procession + burial
- [SYS: divine-acts] god-power VFX props: blessing glow, wrath scorch decals, miracle markers
- [A] the GOD-COMPANION body (§3r — when the convergence lands): one iconic custom creature, full anim set

## 6. SOCIAL & CIVIC
- [A] tavern (interior: bar, kegs, tables, hearth), inn rooms, meeting hall, elder's house / chief's hall /
  throne room (ruler tier), notice board, well (exists), bathhouse, festival set (maypole, banners, feast
  tables, bonfire) [SYS: festivals/events]
- [AN-C] social set: sit-and-drink, converse (gesture pair), dance, play-instrument [A instruments];
  sleep-in-bed pose; [SYS: B7 idle AI drives these — the anims make idle life VISIBLE]
- [A] clothing/appearance breadth per race+class (armor overlays exist — civilian clothes gap)

## 7. WILDERNESS & BESTIARY (the content axis §3r)
- [A] regional wildlife packs by biome (variation-first): tundra (elk done, white wolf, snow hare, bear),
  desert (camel, jackal, scarab swarm), jungle, coast (gulls, crabs — crustacean family exists), rivers
  (fish exist — extend); [AN-N] ALL inherit family animation — pure density win
- [A] monsters: per-biome apex (novel-rig budget: 1–2 per biome, wurm-lesson rigs), dungeon-themed sets
  matching existing factions, MEGABEAST tier (DF-style world-scars: 3–5 uniques, full custom rigs + lore)
- [SYS: stat-linked generation] procedural forgotten-beasts: parts-library recombination driven by generated
  stats — the pipeline maturity roadmap's capstone
- [A] flora breadth: per-biome tree/shrub variants (53 trees exist — variation packs), giant/ancient trees,
  cave flora (glow sets exist — extend), crop wildforms

## 8. DUNGEONS & UNDERGROUND (§3v spelunking)
- [A] per-theme prefab piece SETS (dwarven quarry exists — extend; ruins, crypt, cult sanctum): rooms,
  corridors, junctions as registry components; boss-arena centerpieces; treasure props (chest variants,
  hoard piles); [A] breach-reveal props (cracked walls, collapse rubble)
- [OP] dungeon doors, secret doors, boss gates; [SYS: trap engine] dungeon trap variants
- [A] cave decor breadth: crystal sets per mineral, mushroom forests, underground water features
  [SYS: DF-FLUID for flowing versions — static pools fine now]

## 9. WORLD INFRASTRUCTURE (§3s/§3x)
- [A] road surface sets (dirt→cobble→paved per tech tier §3f), bridge sets (log, plank, stone arch, rope
  [OP drawspan]), ford stones, tunnel portals, retaining walls / terraces (site-prep visuals §3x)
- [A] boundary markers (§3w): stones, banners, totems per race; border-fort kit
- [A] settlement-tier kits: hamlet→village→town→city component sets per race (the catalog tiers = §3f tech
  expression); daughter-settlement starter kit [SYS: B-AG6 + territory]

## 10. EMBODIMENT / RPG TIER (far)
- [A] quest-object props (relics, standing stones, sealed vaults), landmark uniques (the world's "places"),
  hero gear uniques; [AN-C] emote set for dialogue (point, bow, beckon); [SYS: quests, dialogue]
- [A] player-god avatar forms (per aspect); [SYS: B12 possession + embodiment modes]

## 11. AMBIENT & POLISH (the alive-ness layer)
- [A] weather/season dressing: snow caps (variant packs), autumn leaf variants, puddle decals [SYS: seasons
  as ambience]; birdsong emitters (birds exist); [A] smoke/fire FX anchors (chimneys smoke when hearth lit
  [SYS: building-state])
- [AN-C] carry set (§3u note: pick-up / walk-carrying / set-down) — B6 polish, interface already guarded
- [A] lighting set: lanterns (16 exist), sconces, street lamps [SYS: lighting jobs — lamplighter!]
- Debug/dev: [A] fixture library actors (§3j), test-arena kit — infrastructure assets

---

## 12. NATURE & ENVIRONMENT (§3y — the living-world layer)
- [A] juvenile creature variants (fawn, calf, cub, chick — variation packs, inherit animation), aged/old
  variants, carcass + decay-stage props, nests/burrows/dens, monster lair dressing (bone piles, hoards)
- [A] flora growth stages per family (sapling → young → mature → ancient → snag/dead), succession stages
  (shrub/pioneer sets), burned-ground + regrowth variants
- [A] snow-layer blocks/overlays (depth stages), ice blocks (frozen water surface), icicles, frost sprites,
  snow-covered variant packs (trees/roofs — cheap recolors), puddle/mud decals
- [A] drainage infrastructure [SYS: DF-FLUID for function; generatable now]: ditches, gutters + cistern,
  culverts, canal segments, levee/dike pieces, drains, well-pump
- [A] weather props: windmill [OP sails], weathervane, storm shutters [OP], lightning-scorch decals
- [AN-C] shovel-snow (winter job), [AN-N] everything else inherits
- [SYS] tags: wildlife-populations, flora-regrowth (extend rtsim seams — earliest), seasons-temp,
  snow/freeze block-swap, weather-coupling, monster-drives (B-AG5), water-cycle (DF-FLUID)

## 13. MATERIALS & 2D RASTER (§3z — the texture layer)
- **Material library** (voxel surface functions — the engine's "textures"): stone set (clean/mossy/
  weathered/cut), wood set (plank/log/aged/painted), thatch, plaster+timber, brick, metals (polished/
  rusted), fabric, bone, ice/snow-dusted, lava-rock, per-race decorative motifs (dwarven banding, elven
  swirls). Seeded + parameterized (age/wear/wetness) → weathered variants free; ties tech-tier aging.
- **2D raster:** item icons for every wishlist item (verify if auto-rendered from .vox first), UI panels/
  buttons in-style, map/overlay icon language (§3s borders/routes/alerts), buff/status icons (faith, mood,
  weather), loading/menu art. Raster preview harness + style checks (readability-at-16px).
- [SYS: shader-work, far] water/lava/sky variants; **dominion-tint** (visible god-territory — the one with
  design pull).

## GENERATION STRATEGY NOTES
- **Density first via variation packs** (biome wildlife, house variants, clothing) — cheap, animation-free,
  makes the world feel full. **Novel rigs are a budget** (1–2/biome + megabeasts) — spend deliberately.
- **The [AN-C] list is finite and small:** station-work, farm set, worship set, social set, carry set,
  rowing, driver pose, emotes — ~8 custom animation families cover the entire wishlist. Prioritize:
  station-work → carry → worship → social → farm.
- **Every [SYS:] tag is a build-queue demand signal** — when that block lands, its asset batch unlocks
  (the §3i delegation loop, now with a full demand map).
- **Immediately generatable with zero system gates:** items #1, wildlife/flora #7, dungeon sets #8,
  infrastructure #9, faith props #5, boundary markers — hundreds of assets of pure READY work.

---

## Appendix — DF-PRODUCTION flip map (from `DF-PRODUCTION-design.md`, 2026-07-09)

The industry (§1) + agriculture (§3) assets above get precise READY/NEEDS triggers from the DF-PRODUCTION
design pass. Generate a NEEDS batch when its sub-block lands (the §3i delegation loop):

- **READY now** (systems already consume them): crafting-station sprites (anvil/loom/cauldron/…); crafted-good
  models (recipes exist — 326 in `recipe_book_manifest.ron`); prepared-meal + drink icons. Generate
  demand-ordered, not all at once.
- **NEEDS:DF-WORKSHOP → READY on PROD-0:** workshop *building shells* (smithy/kitchen/loom-house the stations
  sit inside), each with a function-harness-reachable WORK POINT.
- **NEEDS:DF-FARM → READY on PROD-2:** crop growth-stage sprites (keyed to `Growth(0..max)`, wheat-style), seed
  item icons, tilled-soil ground texture + fence/trellis dressing.
- **[AN-C] craft-at-station / farm set** — named in the design §5: `anim::craft_hammer/stir/weave`,
  `anim::farm_hoe/sow/harvest`. v1 uses NATIVE stand-ins (mining-swing / gather / crouch); PROD-5 pays the debt.

## Appendix — DF-QUALITY/DF-ARTIFACT asset note (from `DF-QUALITY-design.md`, 2026-07-09)
- **[A/READY]** Quality tiers need NO new assets — the `Quality` enum ships colors (Grey→Orange) + UI.
- **[SYS: DF-ARTIFACT]** artifact-tier **ornateness/glow treatment** for masterwork + artifact items (a
  shader/tint pass so a legendary work *reads* as special; the named artifact reuses the produced-good model
  with an ornate variant) → READY on QUAL-2. Mostly shader, not new geometry.

## Appendix — DF-HIST (Chronicle/Legends) asset note (from `DF-HIST-design.md`, 2026-07-09)
DF-HIST is **UI-only** — no 3D, no in-world sprites, no animation. Its assets are 2D UI, nearly all
authored-in-code. Only one real batch:
- **[SYS: DF-HIST-UI · near-term → ASSET_REQUESTS]** ~10–15 **event-type glyphs** (death/theft/birth/founding/
  war/harvest/masterwork/famine/siege/divine-act), monochrome, ~16px, matching HUD icon style — one per
  `ChronicleEvent` kind; the live feed's at-a-glance legibility. Core ~6 requested now as spec-pressure.
- **[A/READY]** Feed panel frame + importance-band styling (Routine/Notable/Legendary color+weight) = existing
  chat/HUD style reskin, no art. Legends browser layout reuses inspector/map screen furniture.
- **[A/READY→reuse]** Figure/site/faction rows use **existing NPC/site role icons** at v1 — no new art.
- **[SYS: S6 · gated/far-future]** an "attribution / divine-hand" glyph (distinguishes attributed god-acts) +
  bespoke figure portraits — wishlist only, gated on God-Powers/faith; not requested.

## Appendix — DF-RELIGION flip map (from `DF-RELIGION-design.md`, 2026-07-09)

The faith-asset batch (§5 "faith props" above) gets precise READY/NEEDS triggers from the DF-RELIGION pass.
Colony-tier religion sits just past **B7** (worship is a B7 need); REL-0 (the buildable temple) precedes B7.

- **NEEDS:DF-RELIGION → READY on REL-0:** **shrine** (small, one altar — the starter faith building),
  **temple** (`faith`-purpose zone structure, altar + congregation hall), **altar/idol/effigy** prop (the
  worship focal point — the tavern `Bar`/`Stage` analog). Each temple ships a colonist-reachable WORSHIP POINT
  (function-harness gate). Per-race set where sensible; author with a **lore field** (temple lore biases the
  faith it generates — future-work §lore). The worldgen `DesertCityTemple` proves the pattern but is not the
  colony structure.
- **NEEDS:DF-RELIGION → READY on REL-1:** **pew / prayer-mat / kneeler** (congregation-spot dressing — the
  tavern chairs analog; makes worship read), **offering bowl / brazier / incense** (ambience + future offering
  hook, low priority).
- **NEEDS:DF-RELIGION → READY on REL-2:** **priest/prophet vestment** (figure dressing so the priest reads in a
  congregation; ties the per-race cultural-look system).
- **NEEDS:DF-RELIGION → READY on REL-3:** **faith/devotion overlay + HUD icons** (shares the mood/needs overlay
  layer — one overlay engine).
- **NEEDS:B13 → READY on REL-4:** sanctified-ground VFX (holy shimmer on a `faith` zone — the visible mark of a
  god-act; God-Powers-owned).
- **[AN-C] worship set** — named in the design §5: `anim::pray` (kneel+bow), `anim::kneel`, `anim::bless`
  (priest raised-arms). **v1 is NATIVE** (Sit/Cheer/Talk facing the altar — the tavern arena-crowd reused
  wholesale, the cheapest custom-animation topic in the ledger); enrichment pays the debt in the §3u batch.
