# Project Bastion — ASSET_REQUESTS (the request board)

The **interface from the design passes to the asset pilot.** A design pass writes an entry here for any asset
a **near-term** block will consume (per `GENERAL-DESIGNER-prompt.md` step 4 + the §3i delegation model). The
pilot (`MASTER-asset-tooling-prompt.md`) draws generation batches from here. Each entry: what · why (which
sub-block consumes it) · READY-now vs gated · **creative brief** · **lore seed** · style/function notes.

**Rule:** only near-term, real-demand items go here. Speculative breadth stays on `BASTION-CONTENT-WISHLIST.md`
until its `[SYS:]` gate lands. A `NEEDS:<system>` entry may be requested as a **deliberate placeholder to
pressure-test/spec** its system (generate one, not forty).

**REQUEST-WRITING RULE (Ben, 2026-07-09 — so assets don't come out boring/samey):** every request MUST carry
(1) a **Creative brief** — a vivid, specific direction the creator builds *toward*: silhouette, materials, which
named tonal ramps from `ASSET_STYLE_GUIDE.md §5` to hook, the CULTURE theming (human/dwarven/gnarling/adlet/
terracotta, §7), and the **variation axis** that keeps a set from reading same-y (by depth/tier/season/role);
and (2) a **Lore seed** — 1–2 sentences in the world's voice (folk-craft detail + consequence; see the
`ASSET_GENERATION_LOG.md` lore examples) the asset creator authors from. Be creative WITHIN the lore + art
style; never file a bare noun. If a lore seed won't write sensibly, the asset probably doesn't fit (pipeline
rule §2).

---

# ★★★ THE REQUIRED-ASSET QUEUE — pull TOP-DOWN (designer-owned, kept current) ★★★

**Pilot: THIS is your single prioritized work-list. Pull from the TOP.** Priority = **build proximity (frontier+1)**:
🔴 P0 = building now/next · 🟠 P1 = near-frontier · 🟡 P2 = designed-downstream (systems built after B6/B7/B8) ·
🔵 P3 = far-downstream (generate speculatively only if idle). Each row points to its **full brief+lore below**
(Ctrl-F the "Brief in" batch name). This queue **REPLACES both urgency-guessing AND running-wild.** ✅ DONE = already
built, skip. **The designer keeps this current** (updated as builds advance / requests land). — Established 2026-07-10.

| # | Pri | Asset | Gate (NEEDS:) | Consumer / why this priority | Brief in |
|---|---|---|---|---|---|
| — | ✅ | **God-hand v3 rig + ~16 named anims** | — | the showpiece — BUILT | GOD-HAND (definitive) |
| — | ✅ | **God-hand GOOD/EVIL two-faces** (alignment morph over the neutral rig) | — | BUILT 2026-07-10 (architect-confirmed) | UI-5 / GOD-HAND live entry |
| — | ✅ | **God-hand DIVINE VFX presets** (smite/blessing/heal/conjure/shelter + aura/trail, both tints) | — | BUILT 2026-07-10 (`asset-lab/vfx/divine_vfx_presets.md` — on the Outcome/ParticleMode/LightEmitter bus) | UI-5 / GOD-HAND live entry |
| — | ✅ | **Time-control HUD buttons + speed indicator** (⏸ / ▶ 1× / ⏩ 2× / ⏩⏩ faster, active highlighted) | TIME-CONTROLS/UI-3 | **DONE 2026-07-10** — `asset-lab/ui/hud/tc_pause/play/ffwd2/ffwd3/active/clock.png` (16px monochrome, chronicle-glyph language, legibility+distinctness gated, play+active demo) | UI-2 HUD-icon note + UI-3 §3 spec |
| — | ✅ | **Tool-tier variants** (pick/shovel/axe: crude→iron→steel + masterwork stamp) | TOOLS-UPGRADE | **DONE 2026-07-10**: pick crude/iron/steel/dwarven (asset39); shovel crude/iron/steel/masterwork + axe crude/iron/steel/masterwork (asset43) — masterwork = the reused DF-QUALITY brass+cyan stamp | TOOL TIERS |
| — | ✅ | **Divine god-power icons + favor bar** | UI-1 | **DONE 2026-07-12** — asset72 → ui/divine/ (6 power + 4 category glyphs + favor bar), cyan divine-signature aura, gilt→cyan favor bar. LOCKED 11/11 via two-tier blind VLM panel (self + tester + Play-Tester cold-read). Reserve: seal→padlock if a live playtest squints. | UI-1 DIVINE UI ICONS |
| — | ✅ | **HUD-icon set** (panel tabs / alert types / trend arrows) | UI-2 | **DONE 2026-07-12** — asset73 → ui/hud/ (6 panel-tabs + 6 alert-types + 2 trend arrows), ONE HUD language (imports asset72). LOCKED 14/14 via blind panel. Caught 2 real hazards: siege-X=close-button (→battering-ram, mistake-class #34), migration=grave-crosses (→people). *Overlay-legend keys deferred — gated on which map-overlays exist.* | UI-2 HUD-icon note |
| — | ✅ | **Festival décor** (bonfire · feast-table · banners · maypole) | DF-FESTIVAL (ds B7) | **DONE 2026-07-10** (asset38, human set — dwarven on FEST-2) | DF-FESTIVAL FEAST-DAY |
| 6 | 🟡 P2 | **Night-creature reskins + signature horror** | DF-NIGHT (ds B8) | **signature horror DONE** (asset40, creature_night_horror); reskins HELD to coordinate w/ cavern-life (don't fork) | DF-NIGHT + deeper-cavern-life |
| 7 | 🟡 P2 | **Open earlier-batch props** (farm dressing/seeds→DF-FARM · pew/mat→REL-1 · mechanism&trap→DF-MECH · herd→DF-LIVESTOCK · rot/hygiene→DF-ROT · hazard-aftermath→HAZARD-EVENTS · build piece-pools→BUILD-FRAMEWORK) | their systems | designed-downstream (B6/B7/B8-era) | the per-system batches below |
| 8 | 🔵 P3 | **Tier-3 epic assets** (VILLAIN marks · BEAST legendary+trophy · ANCESTORS ghost/shrine · OMEN portents/omen-birth · TEMP furs/frost/brazier · KNOWLEDGE book/library · CURSE mark · CHAMPION aura · ART monument · SACRED-SITES shrine/overlay · RECLAIM ruin · RENOWN heraldry) | their systems (far from build) | far-downstream — **speculative only when idle** | GAP-AUDIT ASSET FILL |

*(P0/P1 are the real "required-now." P2 = ready-to-generate-ahead but not urgent. P3 = the overnight-epic batches —
their systems are many blocks from build, so generate only as idle creative work, not as required demand.)*

---

## From DF-PRODUCTION (`DF-PRODUCTION-design.md`, session 2026-07-09)

The production cluster sits just past **B6** on the frontier; PROD-2 (farm) can partly precede B6. These are
its near-term consumers:

- **[FULFILLED(asset-lab/vox/workshop_kitchen.vox + workshop_loomhouse.vox NEW; workshop_carpenter/mason/smelter/tannery.vox + structure_production_smithy.vox existing — all staged in asset-lab/vox/real/ with RON custom_indices, work points bytes 201-216 REACHED, dynamic-sim PASS 2026-07-09)] Workshop building shells** — smithy / kitchen / loom-house / carpenter — the zone-scale
  structures the existing station sprites sit inside. *Consumed by:* PROD-0 (DF-WORKSHOP) as production-zone
  assets. *Function:* each needs a colonist-reachable WORK POINT at the station (function-harness gate,
  clearance ≥3 blocks, door ≥2.2). *Style:* per-race set where sensible; barn/witch-hut prefab pattern. *Gate:*
  NEEDS:DF-WORKSHOP — request **1–2 as pressure-test/spec placeholders** now (motivate the zone-structure
  spec), full batch on PROD-0 landing.
- **[FULFILLED(asset-lab/vox/sprite_crop_barley_0..5.vox — 1 spec-placeholder crop, 6 distinct stages: height 2->26, green->gold, heads at 4, ripe droop at 5; style vs new sprite-farm reference category (wheat/carrot/flax) PASS; staged 2026-07-09)] Crop growth-stage sprites** — a wheat-style multi-stage sprite set keyed to the existing
  `Growth(0..max)` block attribute, for 2–3 starter crops (grain, a vegetable, a fibre crop). *Consumed by:*
  PROD-2 (DF-FARM). *Function:* stage count must match the crop's max `Growth`; each stage visually distinct
  (the sprite IS the farm's legibility). *Gate:* NEEDS:DF-FARM — **1 crop as spec placeholder** now, batch on
  PROD-2.
- **[REQUEST] Seed item icons** — one per farmable crop (small icon set). *Consumed by:* PROD-2 (new seed item
  defs — none exist today). *Gate:* NEEDS:DF-FARM.
- **[REQUEST] Farm dressing** — tilled-soil ground texture, low fence / trellis props. *Consumed by:* PROD-2
  (makes a farm zone *read* as a farm). *Gate:* NEEDS:DF-FARM.
- **[FULFILLED-FIRST-3(asset-lab/vox/sprite_goods_planks.vox + sprite_goods_bread.vox + sprite_goods_cloth.vox — demand order = carpenter/kitchen/loom outputs; all PASS + staged 2026-07-09; more on demand)] Crafted-good + prepared-meal breadth** — the recipes already exist (326); models/icons for the
  goods colonists will actually make (tools, planks, cloth, bread, ale). *Consumed by:* PROD-0/PROD-3.
  *Generate demand-ordered* (what the starter economy produces first), NOT all at once (§3i warning).

## From DF-HIST (`DF-HIST-design.md`, session 2026-07-09)

DF-HIST (the Chronicle/Legends) is a **UI-only** system — no 3D, no sprites-in-world, no animation. Its single
near-term, real-demand asset is a small 2D icon batch for the live event feed (HIST-2, the DF-LOG slice).
Everything else is UI-authored-in-code or far-future polish (stays on the wishlist).

- **[FULFILLED-CORE-6(asset-lab/ui/chronicle/glyph_{death,theft,birth,founding,harvest,divine_act}.png — 16x16 monochrome white-on-alpha; quantified gates PASS: filled 0.27-0.39 legibility + pairwise Hamming >=48/256 distinctness; contact sheet asset-lab/renders/chronicle_glyphs.png; remainder incl. the DF-TRADE + DF-POLICY chronicle kinds join this set as emitters land)] Chronicle event-type glyphs** — ~10–15 small **monochrome UI glyphs**, one per `ChronicleEvent`
  kind: death, theft, birth, founding, war-declared, harvest, masterwork, famine, siege, divine-act (+ a few as
  the kind enum grows). *Consumed by:* HIST-2 (the live event feed) — each feed row shows its kind glyph; this
  is the feed's at-a-glance legibility. *Function:* readable at ~16px, distinct silhouettes, match the existing
  HUD icon style (not full illustrations). *Gate:* NEEDS:DF-HIST-UI — request **the core ~6 (death/theft/birth/
  founding/harvest/divine-act) as a spec-pressure batch** now; the rest as their emitter sub-blocks land.
- **[NOTE — no other assets]** Figure/site/faction avatars ship v1 as **reused existing NPC/site role icons**
  (no new art); bespoke portraits + the attribution "divine hand" glyph are far-future / gated on S6 — wishlist
  only, not requested.

## From DF-RELIGION (`DF-RELIGION-design.md`, session 2026-07-09)

Colony-tier religion sits just past **B7** (worship is a B7 need); **REL-0 (buildable temple) precedes B7** and
is the near-term consumer. The worldgen `world/src/site/plot/desert_city_temple.rs` proves a temple *renders*
but is desert-/worldgen-specific — the colony needs its own buildable structure.

- **[FULFILLED(asset-lab/vox/structure_faith_shrine.vox + structure_faith_temple_human.vox — worship point byte 218 (declared in ASSET_MARKER_REGISTRY) REACHED in both; temple ships byte-208 pew rows + 204 braziers; lore fields authored; style+function+dynamic-sim PASS; staged w/ RONs 2026-07-09)] Shrine + temple structures** — a **shrine** (small, one altar) and a **temple** (`faith`-purpose
  zone: altar + congregation hall). *Consumed by:* REL-0 (DF-RELIGION). *Function:* each must ship a colonist-
  reachable WORSHIP POINT (the altar-facing congregation spot; clearance ≥3, door ≥2.2 — same function gate as
  workshop shells). *Style:* per-race where sensible; barn/witch-hut prefab pattern; **author with a lore
  field** (temple lore biases faith — future-work §lore). *Gate:* NEEDS:DF-RELIGION — request **1–2 as
  pressure-test/spec placeholders** now (motivate the `faith`-zone structure spec), full batch on REL-0.
- **[FULFILLED(asset-lab/vox/prop_altar_stone.vox — marble+gilt 4x3x3, staged; NOTE for REL-0: no altar SpriteKind exists in vanilla, so worship byte 218 maps to a Filled placeholder until the faith zone adds its focal sprite — seam flagged in ASSET_MARKER_REGISTRY)] Altar / idol / effigy prop** — the worship focal point the congregation faces (the tavern
  `Detail::Bar`/`Stage` analog). *Consumed by:* REL-0. *Function:* placeable focal marker at the worship point.
  *Gate:* NEEDS:DF-RELIGION — **1 as spec placeholder** now, batch on REL-0.
- **[REQUEST] Pew / prayer-mat / kneeler** — congregation-spot dressing (the tavern chairs analog; makes a
  temple *read* as a place of worship). *Consumed by:* REL-1. *Gate:* NEEDS:DF-RELIGION.
  *(pilot 2026-07-09: PARTIAL — temple ships byte-208 bench-marker pew rows (engine renders BenchWoodEnd);
  bespoke pew/mat/kneeler props stay gated on REL-1 per the board rule. Seed icons + farm dressing likewise
  left REQUESTED — gated on DF-FARM, no now-ask.)*

## From DF-TRADE (`DF-TRADE-design.md`, session 2026-07-09)

DF-TRADE sits **downstream of the production/founding frontier** (needs surplus to trade), so most breadth is
gated. The caravan vehicle is **already DONE** (`Body::Cart`/`Body::Carriage` ship). Two near-term real-demand
items:

- **[FULFILLED(asset-lab/vox/structure_trade_depot.vox — covered-store + open goods bay; drop/work point byte 219 (declared in ASSET_MARKER_REGISTRY before use) REACHED; style+function(storage)+dynamic-sim full battery PASS incl. slope; staged w/ RON 2026-07-09)] Trade depot / market-stall structure** — the `ZoneKind::TradeDepot` building the goods pool +
  caravan arrivals sit in (the workshop-shell / temple analog). *Consumed by:* TRADE-1 (the depot zone).
  *Function:* a colonist-reachable **drop/work point** (function-harness gate, clearance ≥3, door ≥2.2) where
  hauled surplus pools and caravans unload. *Style:* market-stall + covered-store pattern; barn/witch-hut
  prefab lineage. *Gate:* NEEDS:DF-TRADE — request **1–2 as spec placeholders** now (motivate the depot-zone
  structure spec), full batch on TRADE-1.
- **[FULFILLED-CORE-3(asset-lab/ui/trade/{caravan_dot,route_line,route_cursed}.png — 16x16 monochrome, quantified legibility + distinctness gates PASS, contact sheet asset-lab/renders/trade_glyphs.png; blessed-halo variant joins when TRADE-5's overlay lands)] Trade-route + caravan map glyphs** — a small UI set: a **caravan map dot**, a **route line**
  style, **blessed / cursed route** markers. *Consumed by:* TRADE-5 (the trade-route overlay — the §3s
  legibility layer). *Function:* readable at map scale, distinct blessed(halo)/cursed(marked) states. *Gate:*
  NEEDS:DF-TRADE-UI — request the **core 3 (caravan dot / route line / cursed marker)** as a spec batch now.
- **[NOTE — feeds DF-HIST]** trade chronicle glyphs (*caravan-arrived*, *caravan-lost-to-bandits*,
  *trade-deal-struck*) join the **DF-HIST event-glyph batch** — do not fork a separate set.
- **[READY-ish] Loaded-pack animal** — optional cart alternative; quadruped bodies ship, only a loaded-pack
  attachment sprite is missing. Low priority (cart suffices for v1).

## From DF-POLICY (`DF-POLICY-design.md`, session 2026-07-09)

DF-POLICY (DF-ORDERS + DF-STANDING — the colony policy layer / Manage tier) is a **UI-in-code + policy-data**
system: **no 3D, no sprites-in-world, no animation, and no asset-pipeline demand.** The whole surface is a HUD
manager/policy panel and a standing-rules toggle list, authored in code like the DF-HIST viewers.

- **[NOTE — no new assets requested]** The only content is a **small set of HUD status glyphs** for order/rule
  rows (quota-met, condition-active, order-stalled, forbidden), and even those are **UI-authored-in-code icons
  in the existing HUD style**, not asset-pipeline work — no request is filed. If any glyph is ever pipeline-
  generated it joins the existing HUD-icon style, not a new batch. Zone-scoped orders reuse the DF-ZONES
  overlay; policy-triggered Chronicle events reuse the **DF-HIST event-glyph batch** (e.g. *famine-policy-
  engaged*, *great-work-completed*) — do not fork a separate set.

---

## From DF-DIG-VERBS + DF-CAVERN-GEOLOGY — "THE MINE & THE DEEP DARK" batch (2026-07-09)
*(GAP FIX: both designs surfaced asset needs but had NO board entries. This is the shared underground/mining-
framework §6 content batch. **Anti-samey framing:** this batch is the colony's DESCENT — orderly timber-and-
lamp near the surface, stranger and more menacing with depth; the cold cyan Velorite glow is the one light in
the dark. Two cultural veins run through it — HUMAN (rough oak cribbing, iron, tallow warmth) and DWARVEN
(machined dark stone, brass/gold, cyan glow — the `metal.brass_gold` + `glow.cyan` ramps, the quarry look).
Vary every piece by DEPTH TIER and CULTURE; never a generic "rock with dots.")*

- **[FULFILLED-HUMAN(asset-lab/vox/mine_pithead_human.vox — squared beam collar, heavy SW brace, off-center tallow hook; dwarven variant on DIG-2; style+function+dynamic-sim PASS, staged 2026-07-09)] Mine-shaft pit-head frame (cribbing + collar)** — the beam/stone collar framing a dug shaft or
  stairwell mouth. *Consumed by:* DIG-2 (shaft dressing) + mining-framework §6 constructed-mine template.
  *Creative brief:* a squared collar of face-connected chunky beams with 1-vox notched joints (`wood.oak_beam`
  for human; `stone.dungeon_dark` + `metal.brass_gold` fittings for dwarven). Organic asymmetry — one corner
  brace heavier, an off-center lantern hook (`glow.cyan` bead dwarven / warm literal human tallow). Must read
  worked and load-bearing, NOT a clean square. Two variants (human / dwarven). *Lore:* "A pit-head is a promise
  to the dark: we are coming down, and we are coming back up. The first beam is set with a coin under it — for
  the earth, the old miners say, so it doesn't ask for a miner instead." *Gate:* NEEDS:DF-DIG-VERBS — 1 (human)
  spec placeholder now; dwarven + batch on DIG-2.

- **[FULFILLED-SPEC(asset-lab/vox/mine_headframe_human.vox — tapering 4-post tower, spoked wheel, rope + hanging kibble; NOTE: a LOW kibble bucket makes a roofed trap pocket the no-trap gate rejects — hang buckets mid-rope, recorded for the DIG-2 batch; staged 2026-07-09)] Headframe / windlass (deep-shaft hoist)** — the landmark tower over a deep vertical shaft that
  reads "this mine goes DEEP" from across the colony. *Consumed by:* DIG-2 (shaft landmark) + §6.
  *Creative brief:* a tapering A-frame / four-post timber tower 3–5 blocks tall, a spoked winding wheel + rope
  drum, a bucket/kibble in the throat. Human = weathered rope-and-oak; dwarven = brass gears + a cyan-lit cage.
  Silhouette must read at distance (chunky wheel, clear throat); asymmetric lean, worn treads. *Lore:* "You can
  measure a shaft's depth by the wheel that lifts from it. The deepest have wheels a man can't reach the top of,
  greased with tallow that never quite covers the smell of what comes up in the water." *Gate:* NEEDS:DF-DIG-
  VERBS — batch on DIG-2 (1 spec placeholder motivates the shaft-landmark spec).

- **[FULFILLED(human rope = existing asset-lab/vox/sprite_ladder_rope.vox (Batch A); dwarven iron = asset-lab/vox/sprite_ladder_iron_dwarven.vox NEW — set-in forged rungs, solid stone footing (low rungs = trap pockets, same catch as the headframe), cyan head bead; staged 2026-07-09)] Ladder variants (climbable shaft rungs)** — culture/material variants of vanilla `SpriteKind::
  Ladder` for the ShaftLadder verb. *Consumed by:* DIG-2 + B5.8 climb. *Creative brief:* (a) human rope-and-
  rung — two knotted ropes, lashed oak rungs, uneven spacing (organic); (b) dwarven iron ladder — forged rungs
  set into stone (`metal.iron`), a `glow.cyan` bead at the head. 1-block-face footprint. *Lore:* "Rope ladders
  are for a season's work; iron rungs mean the dwarves mean to keep the shaft a century. A colony's age is
  written in what it climbs down on." *Gate:* READY (Ladder ships) — variants generate freely.

- **[FULFILLED-FIRST-2(asset-lab/vox/sprite_orevein_velorite.vox — faceted cyan glow shards erupting from a dark collar, glow band 14/15; sprite_orevein_bloodstone.vox — branching wet blood-red veining + ember beads; Coal/Iron/Lodestone on GEO-1; staged 2026-07-09)] Ore-vein sprites — per mineral, depth-graded (the GEOLOGY legibility + the anti-samey heart)** —
  each mineral is a CHARACTER, not a recolor; embedded-in-rock silhouettes (not floating gems), 11 vox/block.
  *Consumed by:* GEO-1 (depth-graded distribution + prospecting reveal; mining drops the existing ore item).
  *Per-mineral brief:*
    · **Coal** — dull matte-black crumbling seams in grey shale, no shine (`stone.dungeon_dark` + true black);
      reads dirty/common/shallow.
    · **Iron** — rust-red/ochre knobbly nodules in grey rock (oxides over `stone.neutral`); honest, workaday.
    · **Velorite** — the cold cyan glow-crystal (Veloren's magic stone): faceted shards erupting from a dark
      collar (`glow.cyan` core / `stone.dungeon_dark`); the one light in the deep — should feel a little wrong,
      a little alive.
    · **Lodestone** — near-black magnetic sheen with a blue-steel `metal.iron` glint, iron filings clinging in
      whorls; subtle, uncanny.
    · **Bloodstone** — the deep-tier menace: dark rock veined with `accent.blood_red` (+ember), wet-looking —
      the "you have dug too deep" mineral.
  Depth-grade: Coal/Iron shallow (dull, safe) → Velorite/Lodestone mid → Bloodstone deep (menace).
  *Lore:* "Miners read the rock like a page. Grey means keep going; red means stop and pray. Velorite is the
  only stone that looks back at you — the dwarves carve their lamps from it, so the light and the danger come
  from the same hand." *Gate:* NEEDS:DF-GEOLOGY — request **Velorite + Bloodstone FIRST** (they carry the whole
  depth-danger read + the glow legibility); Coal/Iron/Lodestone on GEO-1.

- **[FULFILLED(asset-lab/vox/prop_claim_cairn.vox — leaning hand-stacked cairn, chalked slab, blue assay-rag (the glow-stone read); staged 2026-07-09)] Prospecting marker / claim-cairn** — the "we surveyed here" marker (colony-knowledge model,
  discovered≠omniscient). *Consumed by:* GEO-1 + §6 "assess." *Creative brief:* a hand-stacked survey-cairn or
  claim-stake, NOT a generic flag — stacked stone + a chalk-marked slab (human) or a brass claim-plate on a cut
  post (dwarven, `metal.brass_gold`), topped with a colored assay-rag or a cyan tier-bead; organic lean, sprite
  scale. *Lore:* "A cairn means someone stood here and read the stone. The rag on top is the assay: white for
  iron, black for coal, blue for the glow-stone that pays for a whole winter — or costs one." *Gate:* NEEDS:DF-
  GEOLOGY — 1 spec placeholder now.

- **[FULFILLED-BRACKET(asset-lab/vox/sprite_caveflora_shallow.vox — calm teal soft caps + damp lichen; sprite_caveflora_deep.vox — over-large leaning stalks, EMISSIVE bruise-purple caps (glow band) + hanging strands; middle tiers on CAVERN-3; staged 2026-07-09)] Cave-flora dressing — depth-tiered (the deep-dark biome legibility)** — the glow-flora that makes
  each DF-CAVERN danger tier READ. *Consumed by:* CAVERN-0/3 (biome/tier dressing). *Creative brief:* the
  `glow.cave_teal` ramp is the key — glow-mushroom clusters, hanging luminous fungus, cave-pearls. **Grade by
  tier for the depth read:** shallow = ordinary damp lichen + soft glow-caps (calm); mid = denser teal, taller
  stalks; deep = strange, over-large, wrong-colored (push toward `accent.cultist_purple` + sickly teal),
  pulsing menace — "not a place for people." Vary silhouettes (caps / reeds / shelf-fungus / hanging strands;
  see existing `SporeReed`/gloomcap). *Lore:* "The first cavern glows friendly, blue-green, and a child could
  gather the caps. Go down. The light gets brighter and the caps get bigger and somewhere below the blue turns
  the color of a bruise — and the miners who go that deep don't gather anything, they just leave." *Gate:*
  NEEDS:DF-CAVERN — shallow + deep exemplars first (they bracket the range); fill the middle on CAVERN-3.

- **[FULFILLED-EXEMPLARS(asset-lab/vox/creature_gloamwing/ — bat -> bloodmoon pale-maroon range, mid tier; creature_blindmaw_crawler/ — crawler_moss -> blind sand-pale + molten blood-red eye spots, deep tier; per-part style/function PASS (L15 targets from family ref pools); figure-layer, species reg NEEDS:code as filed)] Deeper-tier cavern-life variants (menace-by-depth)** — recolor/rescale variants of shipped Veloren
  underground fauna, graded darker/stranger by tier (the fell-wolf / barrow-troll recolor pattern applied to the
  deep). *Consumed by:* CAVERN-3. *(models = variation-of; species reg NEEDS:code.)* *Creative brief:* take
  cave-fauna skeletons (bats, cave spiders, deep crawlers, troll/golem families) and grade — mid = darker,
  pale-eyed, `hide.theropod_pale` gone grey; deep = glow-eyed apex (`accent.blood_red`/`accent.cultist_purple`),
  larger scaler, unsettling silhouette tweak (extra spines, a blind-white cave variant). Inherit animation free.
  *Lore:* "Everything down there is the pale color of things that never needed to be seen. The deep ones have
  given up eyes entirely — they hunt the shake of a pick on stone, and they are never in a hurry, because down
  there, neither are you." *Gate:* NEEDS:DF-CAVERN + species-reg code — 1 mid + 1 deep exemplar.

- **[FULFILLED-SPEC(asset-lab/vox/mine_breach_maw.vox — neat dungeon-dark cut broken to a jagged maw, raw rock beyond, snapped timber props, teal breath (byte 217); staged 2026-07-09)] The Breach maw (cave-mouth dressing)** — the jagged raw opening where a constructed mine breaks
  into the natural dark; the visible drama of the Breach Event. *Consumed by:* CAVERN-2 (the Breach). *Creative
  brief:* NOT a clean doorway — a broken irregular maw of shattered rock + snapped timber where the colony's
  neat `stone.dungeon_dark` cut gives way to raw asymmetric stone, a breath of `glow.cave_teal` (or, deep, a
  wrongness-purple) leaking from beyond. Reads as violence done to the earth. *Lore:* "There's a sound a pick
  makes when the next swing meets nothing — a hollow note, and then a smell of air that has never been breathed.
  The old hands drop their tools at that note. The new hands lean in to look. That is how you tell them apart,
  after." *Gate:* NEEDS:DF-CAVERN + hazard engine — 1 spec placeholder.

## From DF-ZONES + DF-BURROW — zone & shelter markers (2026-07-09)
*(Small props that make a painted policy-zone READ as itself; culture-themed, characterful — not generic signs.)*

- **[FULFILLED-FIRST-2(asset-lab/vox/prop_zonemarker_meeting_totem.vox — carved leaning communal post, painted face-knot + ochre hand-mark; prop_zonemarker_refuse_stake.vox — leaning midden-stake, tied rag, a thrown bone; gather/water markers on ZONE-1; staged 2026-07-09)] Zone-marker posts (per purpose kind)** — a small characterful marker per `ZoneKind` so a zone
  reads without the overlay. *Consumed by:* DF-ZONES ZONE-0/1. *Creative brief:* one silhouette per purpose,
  NOT a recolored signpost — a **refuse** marker (a leaning midden-stake with a rag), a **gather** marker (a
  forager's bent-branch bundle), a **meeting** totem (a carved communal post, gnarling-totem lineage but
  human/dwarven-styled), a **water** marker (a well-hood or draw-post). Muted, hand-made, asymmetric.
  *Lore:* "A colony marks its ground the way a dog does — a stake here means 'throw your bones here,' a post
  there means 'this is where we gather when the work is done.' Strangers learn a village by reading its stakes."
  *Gate:* READY-ish (DF-ZONES ZONE-0) — the meeting totem + refuse stake first.

- **[FULFILLED-SPEC(asset-lab/vox/prop_muster_bell.vox — iron bell in a stout oak frame, flared mouth + clapper; dwarven gong on BURROW-3; staged 2026-07-09)] Shelter muster-post / alarm bell (the "Call to Shelter" focal)** — the marker at a DF-BURROW
  shelter that reads "run HERE when the horn sounds." *Consumed by:* DF-BURROW BURROW-1/3. *Creative brief:* a
  mounted alarm-bell or a beacon-brazier on a stout post — human = an iron bell in an oak frame; dwarven = a
  brass gong + cyan warning-glow. Should read as urgent/protective, a village's held breath. *Lore:* "Every
  colony keeps one bell it hopes never to ring. When it rings, the children already know the way — they were
  made to walk it, laughing, on the calm days, so their feet would know it in the dark ones." *Gate:* NEEDS:
  DF-BURROW (siege alert = B8-gated) — 1 spec placeholder now.

## From DF-QUALITY — the masterwork read (2026-07-09)
- **[FULFILLED-SPEC(the treatment demonstrated as prop_chair_{plain,fine,masterwork}.vox — see the DF-ROOMS quality-tier entry; masterwork adds brass inlay crown + cyan rune-line + the carved cat; one treatment, not forked, per the board's own rule)] Masterwork / artifact ornate-item treatment** — an ornate overlay/variant so a `Legendary`/
  `Artifact`-quality good reads as legendary at a glance (the strange-mood payoff, DF-QUALITY QUAL-2).
  *Consumed by:* QUAL-2 (the named artifact + masterwork legibility). *Creative brief:* take a plain produced-
  good model (a sword, a chair, a mug) and give it the masterwork read — fine `metal.brass_gold` inlay,
  a `glow.cyan` rune-line, an extra carved flourish, a subtle emissive; the DIFFERENCE from the plain version
  must read instantly. One ornate exemplar over one plain good (spec the treatment, not 40 items). *Lore:*
  "A masterwork announces itself across a room. You cannot say why the plain chair is a chair and this one is a
  throne, only that the hand that made it was, for one strange season, more than a hand." *Gate:* NEEDS:DF-
  QUALITY — 1 spec placeholder (motivates the quality-tier visual treatment).

## Enrichment — existing terse requests now carry creative briefs + lore (2026-07-09)
*(These were filed thin; enriched here so when their DF-FARM gate lands they don't generate generic.)*
- **Seed item icons** (DF-PRODUCTION PROD-2): give each crop's seed its own character — barley in a burlap
  twist, a fat gourd-seed, flax in a waxed-paper fold; NOT one recolored pouch. *Lore:* "A colony's next year is
  in a handful of seed, kept drier than the ale and guarded better. To cook your seed-grain is the one theft the
  old law hangs you for."
- **Farm dressing** (PROD-2): tilled-soil rows, low split-rail fences, bean trellises, a water-butt — **plus a
  scarecrow** (the characterful centerpiece: a cross-stake in a cast-off tunic, a gourd or a wolf-skull head,
  wind-rags). *Lore:* "Every field has its watchman that never eats or sleeps. Children dress the scarecrow in
  the coat of whoever died last winter — so the dead keep working the ground that took them, the farmers say,
  and mean it kindly."

## From DF-ROOMS — "MAKE A ROOM IMPRESSIVE" décor batch (2026-07-09)
*(A room's ceiling is its décor — furniture ships, but the Beauty stat needs objects with soul. **Anti-samey
framing:** décor is where a colony shows its CULTURE and its FORTUNE — a human hearth-hall vs a dwarven vaulted
gallery vs a gnarling totem-den read completely differently. Vary by CULTURE (§7) and by WEALTH TIER (a founding
hovel's clay lamp → a prospering hall's brass chandelier). Every piece hooks a named ramp and carries a lore
hook the room's story can use. Consumed by DF-ROOMS ROOM-1 Beauty stat + role classification.)*

- **[FULFILLED-SPEC-PAIR(asset-lab/vox/prop_wallart_tapestry.vox — wool field, fur border, the woven wolf stitched larger than life; prop_wallart_trophy_skull.vox — mounted horned skull, asymmetric stubs; staged 2026-07-09)] Wall art & hangings** — the cheapest big Beauty lever: woven wall-tapestries, painted hide banners,
  a mounted trophy skull, a carved relief. *Creative brief:* NOT one flat rug recolored — vary the medium: a
  human woven tapestry (`cream.plaster_wool` field with a `fur.earth_brown` border scene), a gnarling war-banner
  (crooked frame, `accent.blood_red` paint), a dwarven cast-brass relief (`metal.brass_gold` + `stone.dungeon_
  dark`). Wall-mounted, thin depth, off-center hang (organic). *Lore:* "A family's whole history hangs on one
  wall — the winters survived woven in rows, the wolf that took the eldest son stitched larger than life so the
  younger ones would fear the right things." *Gate:* NEEDS:DF-ROOMS — request the human tapestry + a trophy-skull
  as the spec pair (they bracket refined vs rough).

- **[FULFILLED-SPEC(asset-lab/vox/prop_statue_ancestor.vox — mossy weathered figure, BOWED head, one raised open hand, clean plinth; dwarven founder on ROOM-1; staged 2026-07-09)] Statue / effigy (freestanding Beauty landmark)** — the room's centerpiece flourish; the "someone
  spent a season on this" object. *Creative brief:* a carved figure on a plinth — human = a weathered stone
  ancestor or a saint (`stone.mossy`/`stone.neutral`); dwarven = a machined brass founder-figure with a
  `glow.cyan` inlaid eye; scale to sprite/small-structure. Give each a POSE that tells a story (a raised hand, a
  bowed head, a hand on an axe), not a generic pillar. *Lore:* "Every hall keeps its stone ancestor by the
  hearth. The dwarves swear theirs remembers the vein that started the hold; the humans admit theirs is just
  someone's grandfather, but he watches the door all the same." *Gate:* NEEDS:DF-ROOMS — 1 (human ancestor) spec
  placeholder.

- **[FULFILLED-SPEC(asset-lab/vox/prop_hearth_human.vox — weathered stone breast, live glow-band fire + ember, timber mantel w/ pot + carved bird, asymmetric soot; staged 2026-07-09)] Hearth / fireplace (warmth + Beauty + the heart of a room)** — the human hall's soul; ties
  Consecrate/Hearth-bless (ROOM-3 god aura). *Creative brief:* a stone-and-timber hearth with a live-fire glow
  (warm literal + ember), a mantel with small clutter (a pot, a carved bird); dwarven = a forge-lit brass fire-
  box, cyan-edged. Asymmetric soot-staining, worn hearthstone. *Lore:* "The hearth is lit the day the roof
  closes and kept alive until the last of the family leaves or dies. A cold hearth in a lived-in house is the
  worst omen a visitor can read — it means the people inside have given up before the fire did." *Gate:* NEEDS:
  DF-ROOMS — 1 (human) spec placeholder.

- **[FULFILLED-FIRST-2(asset-lab/vox/prop_throne.vox — high carved back, brass finial crown, draped pelt; prop_bed_fourpost.vox — canopy posts + rails, one drape drawn; grand table + lectern on ROOM-1; staged 2026-07-09)] Role-defining centerpieces (the furniture that NAMES a room's role)** — a **throne** (great-hall),
  a **grand dining table** (dining hall), a **four-post bed** (fine bedroom), a **lectern/bookshelf cluster**
  (study). *Creative brief:* these must out-read their plain cousins at a glance — a throne is not a big chair
  (high carved back, `metal.brass_gold` or antler crown, a pelt draped organic); a grand table seats a hall
  (long, heavy, scarred with use); a four-post bed has canopy posts + drapes. Culture-vary. *Lore:* "You can
  tell who rules a hall by which seat has a back taller than a man. In dwarven holds the throne is cut from the
  first stone of the mountain; in human towns it's whatever chair the founder died in, and no one dares replace
  it." *Gate:* NEEDS:DF-ROOMS — the throne + four-post bed first (they define the two highest-value roles).

- **[FULFILLED-SPEC-PAIR(asset-lab/vox/prop_potted_herb.vox + prop_hanging_lantern.vox — clay pot w/ straggly herbs; iron cage lantern, tallow flame; staged 2026-07-09)] Small-comfort clutter (the "lived-in" Beauty layer)** — potted greenery, a woven rug, a hanging
  lantern/chandelier, a shelf of crockery, a rush-strewn floor accent. *Creative brief:* the cheap, plentiful
  layer that lifts a plain room from bare to homely — vary silhouettes and scatter them (organic asymmetry).
  Human = rushes, clay pots, a tallow lantern; dwarven = a brass chandelier with `glow.cyan` beads, cut-crystal
  clutter. *Lore:* "It's the small things that make a cell a home — a pot of kitchen-herbs on the sill, a rug so
  the floor doesn't bite your feet in winter. The colonists who add them are the ones who mean to stay." *Gate:*
  NEEDS:DF-ROOMS — a potted-herb + a hanging lantern as the cheap-Beauty spec pair.

- **[FULFILLED-SPEC-SET(asset-lab/vox/prop_chair_{plain,fine,masterwork}.vox — plain: rough planks, low back, one short leg; fine: oak, carved crest + detail; masterwork: brass inlay + cyan rune-line + the carved cat on the arm; reads instantly across tiers, reuses the QUAL-2 treatment; staged 2026-07-09)] Quality-tiered furniture variants (make DF-QUALITY's Wealth stat VISIBLE)** — plain → fine →
  masterwork versions of the core pieces (bed, chair, table). *Creative brief:* the same object at three craft
  tiers so a room's Wealth READS: plain (rough oak, `wood.plank_warm`), fine (planed, joined, a carved detail),
  masterwork (inlay + `metal.brass_gold` + a `glow.cyan` rune-line — ties the DF-QUALITY masterwork-treatment
  request; DO NOT fork it, reuse that treatment). The DIFFERENCE across tiers must read instantly. *Lore:* "A
  guest reads your fortune from your chairs before you've said a word. The rough stool says lean year; the
  chair with the cat carved into the arm says a master carpenter owed someone a favor, and got to show off." *Gate:*
  NEEDS:DF-ROOMS + DF-QUALITY — one piece (a chair) at all three tiers as the spec set.

## From BUILD-FRAMEWORK — "COMPOSED STRUCTURE PIECE-POOLS" batch (2026-07-09)
*(The tier-3 composition batch — per-race pieces that snap into multi-story structures (BUILD-5/§6). **Anti-samey
framing:** this is where a settlement stops looking copy-pasted — a story assembled from pieces looks emergent,
and the CULTURE is carried entirely in the piece pool (a dwarven vaulted stone story vs a human timber-framed
one read completely differently from the SAME composition logic). Vary by CULTURE (§7) and by STORY (ground floor
= heavy/public, upper = lighter/private, roof = the crown). Each piece must tile/stack cleanly at 1-block seams
(the composer snaps them by offset). Consumed by BUILD-5 composed templates + BUILD-2 build-UP.)*

- **[REQUEST] Wall segments (per race, per story-role)** — the modular wall unit that repeats along a footprint
  and stacks between floors. *Creative brief:* NOT one wall recolored — human = timber-frame-and-plaster (oak
  posts `wood.oak_beam`, `cream.plaster_wool` infill, a `stone.neutral` ground-story base course); dwarven =
  dressed dark stone with machined `metal.brass_gold` string-courses + a `glow.cyan` sconce niche; gnarling =
  lashed crooked poles + war-paint. Ground-story variant heavier (a base course, fewer windows); upper-story
  lighter (more window, a jetty overhang). Corner + straight + windowed variants so a run isn't monotone. *Lore:*
  "You can read a house's age in its walls — the ground course is always the oldest and the heaviest, laid by
  hands now dead; each story above is a lighter promise made by whoever could afford to build up instead of out."
  *Gate:* NEEDS:BUILD-FRAMEWORK — the human timber-frame wall (straight + corner + windowed) as the spec set.

- **[REQUEST] Floor / story-deck pieces** — the horizontal slab that caps a story and becomes the next floor
  (the multi-story primitive; must carry a stair-opening variant for DF-DIG-VERBS access). *Creative brief:*
  human = planked joists over beams (`wood.plank_warm`), a hatch/stair-well cut variant; dwarven = flagstone
  over vaulting, a cyan-lit stair void. The stair-opening variant is functionally load-bearing (BUILD-2 access).
  *Lore:* "The second floor is a act of faith in the first — you sleep above people trusting the beams your
  grandfather sank. The carpenter who cuts the stair-well leaves his mark on the top step, so the house always
  knows who let the light down into it." *Gate:* NEEDS:BUILD-FRAMEWORK — plain + stair-opening variants (human).

- **[REQUEST] Roof caps (per race — the crown of a composed structure)** — the top piece that reads a building's
  silhouette from across the colony. *Creative brief:* reuse the barn roof-grammar (flared eave → diagonal
  treads → flat ridge cap; §7 style rules) but VARY by culture: human brown-plank gable (`wood.roof_barn`);
  dwarven low stone vault + brass ridge; gnarling shaggy thatch + a totem finial; adlet snow-dome. A chimney /
  finial / weathervane accent (off-center, organic). *Lore:* "A roof is the one part of a house a stranger sees
  first and a family sees last. The finial is chosen the day the roof closes — a wolf for a hunter, a sheaf for
  a farmer, and for the unlucky, nothing, because the coin ran out before the carver did." *Gate:* NEEDS:BUILD-
  FRAMEWORK — human gable + dwarven vault as the culture-contrast spec pair.

- **[REQUEST] Foundation / understory + terrace pieces (site-prep §5 made visible)** — the platform/base a
  structure sits on when the ground isn't flat (the barn-on-a-slope answer). *Creative brief:* a dry-stone
  retaining course + a filled platform top, terraced for steep sites; human = rough fieldstone (`stone.mossy`);
  dwarven = cut-block engineered terrace with a drainage channel. Reads as "someone made this ground behave."
  *Lore:* "On a hill you build the ground before you build the house. The old masons say a good foundation is
  invisible — you only ever notice the bad one, the winter it slides, taking the family's chimney down the slope
  with it." *Gate:* NEEDS:BUILD-FRAMEWORK (site-prep) — the human fieldstone terrace course.

- **[REQUEST] Doors & windows (the composition's openings + operable seam)** — the door/window pieces that punch
  a wall segment (and the door is the DF-OPERABLE seam). *Creative brief:* culture-vary the opening — human
  plank door + iron hinges + a shuttered window; dwarven brass-bound stone door + a cyan-lit arrow-slit; each
  with an open/closed state hook for the operable framework (§1b, later). Off-center, worn thresholds. *Lore:*
  "A door is a house's opinion of the world. The dwarves hang theirs to swing inward, so a mob can't push it;
  the humans hang theirs outward, so the smell of bread gets out to the road and brings the travelers in."
  *Gate:* NEEDS:BUILD-FRAMEWORK + DF-OPERABLE (operable state later) — a human plank door + shuttered window now.

- **[REQUEST] Road-surface + path pieces (§3x roads)** — the surfaced path variants a road-build lays down (the
  desire-line-hardens-into-road payoff). *Creative brief:* reuse Veloren's road/path block material but give
  wear variety — a packed-dirt track, a cobbled main street (`stone.neutral` set in `fur.earth_brown` mud), a
  worn rut down the center; a milestone/waystone marker (ties FOUNDING/trade). Organic meander, not a ruler line.
  *Lore:* "A road is just where enough feet agreed to go. The cobbles come later, laid by a village tired of mud,
  and the rut down the middle is every cart that ever carried something heavier than hope to market." *Gate:*
  NEEDS:BUILD-FRAMEWORK (roads) — a cobbled-street + packed-track pair.

## From HAZARD-EVENTS — "hazard aftermath" prop batch (2026-07-09)
*(Small props so a hazard LEAVES A MARK the colony has to live with / clean up — the visible scar that makes the
event linger. Culture-neutral (the earth has no culture) but VARY by hazard kind. Consumed by HAZ-2 callers.)*

- **[REQUEST] Rubble / cave-in debris pile** — the scar a rockfall or cave-in leaves (blocks a passage until
  cleared — ties DF-DIG-VERBS re-dig). *Creative brief:* a heaped tumble of broken rock + snapped support-timber
  splinters (`stone.dungeon_dark` + `wood.oak_beam` shards), dust-pale on top, NOT a tidy pile — asymmetric,
  half-burying whatever was under it; a variant with a boot or a dropped pick just visible. *Lore:* "A cave-in
  leaves a wall where a tunnel was, and the colony argues for a season over whether to dig the miner out or dig
  around him. The pick is always still there; the man, sometimes." *Gate:* NEEDS:HAZARD-EVENTS — 1 spec now.

- **[REQUEST] Scorch / ash mark** — the burn a fire or lava/wrath hazard leaves on the ground. *Creative brief:*
  a flat blackened scorch decal + charred stubs + a thin ash-grey drift (true black → `stone.dungeon_dark` → a
  pale ash), edges licking outward organically; a lava variant with a cooling-crust `accent.blood_red` ember
  glow fading to black. Reads "something burned here and the grass remembers." *Lore:* "Grass grows back green
  over a house fire but never over a god's wrath — the black stays, and the village walks around it, and in a
  generation no one remembers why that field is cursed, only that it is." *Gate:* NEEDS:HAZARD-EVENTS — 1 spec.

- **[REQUEST] Splintered stump + fallen trunk** — the timber-hazard aftermath (a felled tree that fell wrong).
  *Creative brief:* a jagged burst stump (pale inner wood `wood.plank_warm` splintering out of dark bark) + a
  separate fallen-trunk piece lying across the ground (the §2 faked-fall come to rest); moss/organic asymmetry.
  *Lore:* "A tree falls the way it wants, not the way the axe intends. The one that killed the woodcutter's
  apprentice was left where it lay for a year, and they carved his name into the trunk, and the moss took both."
  *Gate:* NEEDS:HAZARD-EVENTS (timber caller) — 1 spec now.

## From DF-MECH/TRAP/OPERABLE — "MECHANISM & TRAP" batch (2026-07-09)
*(The trigger→link→effect content. **Anti-samey framing:** a mechanism reveals a culture's mind — DWARVEN =
machined brass gears + `glow.cyan` runes + oiled precision (they LOVE a mechanism); HUMAN = timber + iron +
rope, honest and improvised; GNARLING = a cruel improvised deadfall of crooked wood + bone. Vary by CULTURE (§7)
and by ROLE (benign operable vs lethal trap). The operable ones (door/gate/bridge/floodgate) COORDINATE with the
asset-lab operable framework — SLIDE/HINGE component states, don't fork §7. Consumed by MECH-0/1/2.)*

- **[REQUEST] Trigger sprites — lever / pressure-plate / tripwire** — the input side. *Creative brief:* a
  **lever** (a pivoting handle — human oak-and-iron on a post; dwarven a brass throw-switch with a cyan detent);
  a **pressure plate** (a subtle floor tile, slightly proud, that a raider won't notice — the DANGER is that
  it's almost invisible; a sprung/depressed variant); a **tripwire** (a taut line low across a passage, a bell
  or a snag at one end). *Lore:* "The dwarves make a lever you can feel through your boots, a satisfying throw
  and click; the humans make one that sticks in the wet and has to be kicked. Both open the same gate — but only
  one gets bragged about in the tavern." *Gate:* NEEDS:DF-MECH — the lever + pressure-plate (armed/sprung) first.

- **[REQUEST] Operable terrain — door / portcullis-gate / drawbridge / floodgate** — the benign effect side
  (SLIDE/HINGE, coordinate w/ asset-lab operable §7). *Creative brief:* a **portcullis** (an iron/timber grille
  that drops — the classic; dwarven brass-bound); a **drawbridge** (a timber span that raises on chains over a
  gap/moat); a **floodgate** (a sluice board in a channel that lifts to release water — ties DF-FLUID). Each
  needs a clean open + closed state (the operable framework). *Lore:* "A portcullis is a town's held breath —
  dropped, it says 'not tonight'; and the winch that lifts it is greased with the same fat as the feast it lets
  in. The drawbridge is trust made of timber: raise it and you have decided who your neighbors are." *Gate:*
  NEEDS:DF-OPERABLE — portcullis + drawbridge as the spec pair (both prove SLIDE + HINGE).

- **[REQUEST] Trap effects — spike-field / cage / boulder / stonefall** — the lethal effect side (fire a
  HazardEvent). *Creative brief:* a **spike-field** (retracted flush → thrust up, blood-darkened iron over
  stone); a **cage** (a dropped grille that traps — the take-alive option); a **boulder / stonefall** (a poised
  rock that a trigger releases down a slope — TerrainDestruction). Dwarven precision-machined vs gnarling crude
  deadfall. *Lore:* "The best trap is the one a raider springs for his fellows to see — the spike-field at the
  gate is set shallow on purpose, so the second rank watches the first rank learn, and thinks better of the
  third step." *Gate:* NEEDS:DF-TRAP + HAZARD-EVENTS — the spike-field (retracted/sprung states) first.

- **[REQUEST] Wiring / gear / mechanism props** — the LINK made visible (the mechanism overlay's physical
  counterpart). *Creative brief:* dwarven brass **gears + axle + a winding drum** (`metal.brass_gold` +
  `glow.cyan` runes, oiled and precise); human **rope-and-pulley + a wooden cam**; a small **mechanism box** the
  link runs through. Reads as "someone engineered this." *Lore:* "The dwarves sign their mechanisms the way a
  painter signs a wall — a maker's rune on the main gear, so that in a hundred years, when it still turns, they
  get the credit and the blame. The humans just tie a good knot and hope." *Gate:* NEEDS:DF-MECH — a dwarven
  gear-train + a human rope-pulley as the culture-contrast pair.

## From DF-LIVESTOCK — "HERD & HUSBANDRY" batch (2026-07-09)
*(The domesticated-animal layer. **Anti-samey framing:** livestock carry a REGION and a PURPOSE — a shaggy
highland cow bred for hardiness reads nothing like a sleek lowland dairy cow; a war-goat vs a milk-goat. Vary by
BREED/REGION and by AGE (adult vs the cheap juvenile variation-packs — §3y). Reuse existing quadruped bodies +
recolor/rescale (the fell-wolf/elk pattern — generate-to-skeleton = free animation). Consumed by STOCK-0..2.)*

- **[REQUEST] Livestock species + breed variants** — sheep, cattle, pig, goat, chicken/fowl, each with a
  region/breed twist. *Creative brief:* variation-of existing quadruped bodies — a **highland cow** (long shaggy
  coat `fur.earth_brown`, wide horns, peat-dark) vs a **dairy cow** (patched cream/brown, heavy udder); a
  **hill sheep** (dense `cream.plaster_wool` fleece, curled horns) vs a shorn one; a **boar-ish pig** vs a fat
  sty pig. Breed says what the colony bred FOR. *Lore:* "You can read a valley's weather in its cattle — the
  highland cows wear a winter no lowland beast would survive, and the lowland dairy cows would drown in the
  highland rain. Each is the shape of a hundred years of someone choosing which calf to keep." *Gate:* NEEDS:
  DF-LIVESTOCK — a highland cow + a hill sheep as the region-contrast spec pair.

- **[REQUEST] Juvenile variation-packs** — lamb / calf / piglet / kid / chick (the §3y aging visual). *Creative
  brief:* scale-down + softer-palette + shorter-proportion variants of each adult (bigger head, stubbier legs —
  the universal "young" read); they must clearly be the SAME animal, younger. *Lore:* "The spring lambs are the
  colony's whole hope wearing legs it hasn't grown into yet. The children name them, which the farmers allow and
  regret, because a named lamb is a hard autumn." *Gate:* NEEDS:DF-LIVESTOCK — a lamb + calf first (prove the
  aging variant).

- **[REQUEST] Husbandry products + butchery drops** — milk pail, wool bundle, eggs, and the **butchered
  carcass / hanging hide** (feeds the TanningRack + CookingPot chains). *Creative brief:* a wooden milk pail
  frothed white; a tied wool fleece (`cream.plaster_wool`); a hung skinned carcass on a hook (honest, not gory-
  gratuitous — DF-matter-of-fact); a stretched drying hide. *Lore:* "Nothing of a slaughtered beast is wasted or
  the old woman who taught you will know — hide to the tanner, fat to the candles, bones to the broth, and the
  bladder to the children for a ball. Waste is the one sin a lean winter can't forgive." *Gate:* NEEDS:DF-
  LIVESTOCK — the milk pail + the hanging hide first.

- **[REQUEST] Pasture dressing** — hay-rack, milking-stool, shearing-post, a salt-lick, a low field shelter.
  *Creative brief:* the small marks of a tended pasture (ties the DF-ZONES pasture marker) — a timber hay-rack
  half-full, a three-leg stool worn smooth, a rubbing-post the beasts have polished. Organic, well-used.
  *Lore:* "A good pasture is furnished like a poor man's parlor — a rack for the hay, a post worn shiny by a
  hundred itchy flanks, and a stool that knows one cow's temper from another's by now." *Gate:* NEEDS:DF-
  LIVESTOCK — the hay-rack + shearing-post.

## From DF-ROT — "ROT & HYGIENE" batch (2026-07-09)
*(The decay-stage + hygiene content. **Anti-samey framing:** decay is a PROGRESSION — the same object at fresh /
spoiled / rotten / inert must read its stage at a glance (it's the legibility of the whole pressure). Mostly
culture-neutral (rot has no culture) EXCEPT burial, which is deeply cultural. Vary by STAGE and by MATTER-TYPE.
Consumed by ROT-0..2.)*

- **[REQUEST] Decay-stage food + carcass progression** — the same item across freshness stages. *Creative brief:*
  a loaf/meal: golden-fresh → grey-green spotted (spoiled) → a slumped fuzzed ruin (rotten); a carcass: red-
  fresh → bloated-dull → a `stone.dungeon_dark` skeletal remains (bones/inert). The STAGE must read instantly —
  this is the pressure's legibility. Matter-of-fact, not gore-porn (DF's honest tone). *Lore:* "A cook's whole
  art is racing the rot — she can tell the hour of a joint of meat by the color, and the day she loses that eye
  is the day the colony starts burying more than it feasts." *Gate:* NEEDS:DF-ROT — the food 3-stage set first
  (it's the most-seen decay).

- **[REQUEST] Miasma haze** — the visible pressure (VFX-leaning; a low drifting sprite/particle if not pure
  shader). *Creative brief:* a sickly low-lying haze, `accent.cultist_purple` bruised into a dirty grey-green,
  thicker over the source, thinning at the edges — reads WRONG, reads "hold your breath." Not a pretty fog.
  *Lore:* "Miasma pools in the low corners like bad water. The old folk knew to sleep upstairs and build the
  midden downwind; the ones who forgot woke sour, and the ones who really forgot woke sick." *Gate:* NEEDS:
  DF-ROT — 1 haze exemplar.

- **[REQUEST] Refuse / compost pile + midden** — the Refuse-zone content (DF-ZONES). *Creative brief:* a heaped
  midden of broken crockery, bones, ash, peelings, a cracked pot — the colony's honest garbage, NOT tidy; a
  composting variant (darker, steaming, half-turned to soil — ties DF-FARM fertilizer). *Lore:* "You can read a
  village's whole diet out of its midden, the tinkers say, and a hundred years after the last hearth goes cold
  the only thing left to find is the heap of what they threw away." *Gate:* NEEDS:DF-ROT + DF-ZONES Refuse — the
  midden first.

- **[REQUEST] Grave / burial-mound + pyre** — the corpse destination (ties DF-RELIGION — cultural). *Creative
  brief:* a simple earth mound + a marker (human = a carved oak stake or a fieldstone; dwarven = a cut-stone
  slab in a wall-niche; a fresh mound vs a grassed-over old one); a pyre variant (charred timber + ash). VARY by
  culture — burial is where a people show what they believe about death. *Lore:* "The dwarves seal their dead
  in the stone they mined, so the mountain keeps its own; the humans give theirs back to the field, a mound and
  a stake and a season of the widow walking out to it. Both hope the same thing, and neither will say what."
  *Gate:* NEEDS:DF-ROT (burial) + DF-RELIGION — a human grave-mound + a dwarven wall-niche as the culture pair.

- **[REQUEST] Vermin** — rats + flies/insects (the filth pest). *Creative brief:* small, quick, unlovely —
  a lean midden-rat (reuse a small quadruped/rodent skeleton, `fur.earth_brown` gone greasy-grey), a fly-swarm
  (a dark particle cluster). Reads "this place is not clean." *Lore:* "Rats are the tax the careless pay. Every
  colony has them; the well-run ones have a cat and a clean midden, and the rest have a plague waiting for a
  bad winter to introduce itself." *Gate:* NEEDS:DF-ROT — the midden-rat first (variation-of a rodent body).

## From DF-SYNDROME — "AFFLICTION" batch (2026-07-09)
*(Small — symptoms are buffs and were-forms reuse existing bodies. **Anti-samey framing:** an affliction READS
on the sufferer + on the place. Vary by AFFLICTION-TYPE. Consumed by SYN-0..3.)*

- **[REQUEST] Sickly overlay set** — the visual an afflicted colonist wears (recolor/particle, not a new body).
  *Creative brief:* a pallor (skin grey-drained, dark eye-hollows) + a fever-sweat sheen + a pox variant
  (spotted); a distinct "the thirst" look for vampirism (bloodless pale, too-bright eyes). Must read across a
  room — the whole point is spotting the sick before they spread. *Lore:* "You learn to read a face for the
  fever before the sufferer admits it — the grey comes first, then the shine, and by the time they cough you
  should already have moved the children to the far hall." *Gate:* NEEDS:DF-SYNDROME — the fever pallor first.

- **[REQUEST] Plague-doctor mask + herb-kit** — the DF-MEDICAL crossover, the icon of an outbreak fought.
  *Creative brief:* a beaked leather mask stuffed with herbs, waxed robe; a satchel of dried herbs + a bleeding-
  bowl. Ominous but competent — the person you're relieved to see and afraid to need. *Lore:* "The beak is full
  of rue and rosemary against the miasma — whether it works no one knows, but the doctor who wears it lives more
  often than the one who doesn't, and in a plague that's all the proof a colony needs." *Gate:* NEEDS:DF-
  SYNDROME + DF-MEDICAL — the mask.

- **[REQUEST] Quarantine marker** — the warded/sealed sign on an isolation burrow (ties DF-BURROW quarantine).
  *Creative brief:* a chalk cross on a plank door, a hung warning-rag, a barred hasp — reads "do not enter, the
  sick are here." Grim, hasty, hand-made. *Lore:* "A chalk cross on the door is the hardest mark a colony makes
  — it means someone inside is being left to live or die on their own, so the rest can. The families argue it
  every time and draw it anyway." *Gate:* NEEDS:DF-SYNDROME + DF-BURROW — the crossed door.

## From ARCHITECT directive (Ben) — NAVAL SET (2026-07-09)
*(Not designer-filed: direct order. ALL pieces tagged **NEEDS:naval-movement** — the sailing sim is being
designed now; these are pressure-test/spec pieces riding the ship-manifest substrate (bone0 hull vox at block
scale + per-vehicle RON custom_indices low bytes -> Air(Sprite): airship convention 1=seat 2=helm 6=lantern;
namespace note added to ASSET_MARKER_REGISTRY). Native skiff/sail_boat/galleon calibrate the new vehicle-ship
style category. Variation-first honored: fishing skiff + warship are VARIATIONS-OF native hulls.)*

- **[FULFILLED(asset-lab/vox/vehicles/ — vehicle_rowboat 7x13x5, vehicle_skiff_fisher (variation-of skiff: tan patched sail + creels), vehicle_river_barge 13x35x10 (open hold, crate cargo, stern sweep helm), vehicle_cog_trader 15x37x28 (round belly, stern castle + companionway, square sail, bowsprit), vehicle_galleon_warship (variation-of galleon: shield rows, iron ram, sea-grey sails) — each style PASS + DECK-CHECK PASS (stern->bow path, 0 NEW trap pockets; ship interiors are a family norm, judged on delta) + manifest-entry snippet per vessel + catalog_vehicles.json)] Naval vessels: rowboat / fishing skiff / river barge / cog trader / warship** — NEEDS:naval-movement.
- **[FULFILLED(components c_pier_seg + c_pier_end (LINE, mooring point byte 224) + composed pier_line_demo; prop_mooring_bollard; structure_boathouse (water-side open end + land door); structure_lighthouse (tapering tower, beacon byte 223, gallery — top reached by ladder verb, engine-side); prop_harbor_crane (timber jib, counterweight, hook, work point 224) — all style+function PASS, staged world-layer 2026-07-09)] Harbor set: dock/pier / boathouse / lighthouse / harbor crane** — structures usable pre-naval (piers + lighthouse read on any shore); markers 223/224 declared in the registry BEFORE use.

## From DF-JUSTICE — "AUTHORITY & JUSTICE" batch (2026-07-09)
*(Small — positions are colonists with role markers; jail reuses DF-BURROW. **Anti-samey framing:** authority is
where a culture shows what it VALUES — a dwarven chain-of-office is machined and heavy, a human one is a worn
oak staff and a good cloak. Vary by CULTURE. Consumed by JUST-0..3.)*

- **[REQUEST] Marks of office (regalia)** — the role-marker worn by a leader/steward/sheriff (an armor/accessory
  overlay, not a new body). *Creative brief:* a **leader's** circlet or chain-of-office (dwarven brass +
  `glow.cyan` bead; human a worn iron circlet + a fur-trimmed cloak); a **sheriff's** badge/staff-of-office + a
  ring of keys. Reads "this one has authority" across a crowd. *Lore:* "The chain of office is heavier than it
  looks, the old stewards say, and they don't mean the metal. A colony can tell a leader who wears it easy from
  one it's strangling — and so, eventually, can the god." *Gate:* NEEDS:DF-JUSTICE — the leader's circlet +
  sheriff's staff.

- **[REQUEST] Jail / stocks / pillory** — the punishment structure (ties DF-BURROW confinement). *Creative
  brief:* a barred timber cell (human) / a cut-stone lockup (dwarven) for jail; **stocks** (a hinged board that
  clamps hands/head — public shame) + a **pillory** post for lesser crimes. Grim but not torture-porn — the
  matter-of-fact justice of a hard place. *Lore:* "A day in the stocks is worse than a week in the cell — the
  cell forgets you, but the whole village walks past the stocks, and a colonist's neighbors have a longer memory
  than any lock." *Gate:* NEEDS:DF-JUSTICE + DF-BURROW — the stocks first (the everyday punishment).

- **[REQUEST] Moot-seat / judgment throne** — where rule + judgment sit (ties DF-ROOMS great-hall / DF-QUALITY
  throne). *Creative brief:* a raised seat of authority — human a heavy carved oak chair on a dais with a pelt;
  dwarven a cut-stone throne with brass + a cyan-lit back-niche; worn, used, a little intimidating. Distinct
  from an ordinary chair (the DF-ROOMS throne request — reuse/coordinate, don't fork). *Lore:* "The moot-seat is
  set a single step above the floor — high enough that the judged must look up, low enough that a bad judge can
  be pulled back down to it. The step is the whole of the law, really." *Gate:* NEEDS:DF-JUSTICE (coordinate w/
  the DF-ROOMS throne) — 1 human moot-seat.

## From DF-POWER — "POWER" batch (2026-07-09)
*(Generators + machines. **Coordinate with the DF-MECH gear/axle batch — the transmission (gears/axles/belts) is
SHARED, don't fork.** Anti-samey: a windmill is a colony's proudest silhouette — vary by CULTURE (human canvas-
sail post-mill vs dwarven brass-vaned tower). Consumed by POW-0/1.)*

- **[REQUEST] Windmill (the ridge-top landmark)** — the wind generator; must read from across the colony.
  *Creative brief:* a tower + big turning sails — human = a timber post-mill, four canvas-and-lath sails, a
  weathered cap that turns to the wind; dwarven = a squat brass-vaned tower, cyan-lit hub, machined precision.
  Sails are an operable ROTATION state (turn with wind). Silhouette is everything (a colony landmark). *Lore:*
  "A village raises its mill on the windiest ground it owns and argues about it for a generation — too far to
  carry the grain, they say, until the first winter the mill saves them, and then the walk is never mentioned
  again." *Gate:* NEEDS:DF-POWER — the human post-mill first (POW-0's generator).

- **[REQUEST] Water-wheel + race** — the river generator (undershot/overshot on a channel/race). *Creative
  brief:* a big spoked wheel in a timber race, water-darkened lower paddles, moss on the still side; an overshot
  variant (water fed over the top). Turning = operable rotation. Human timber; dwarven iron-and-brass. *Lore:*
  "The water-wheel never sleeps and never asks for hay — the millers say it's the only honest worker in the
  valley, and only half joking. It rots from the waterline up, a hand's-width a decade, and a good miller knows
  to the year when it will need re-timbering." *Gate:* NEEDS:DF-POWER — the undershot wheel + race.

- **[REQUEST] Powered machines — millstone + sawmill** — the consumers (DF-PRODUCTION powered stations).
  *Creative brief:* a **millstone** (a heavy turning grindstone in a hopper-frame, flour-dusted, a feed-chute);
  a **sawmill** (a powered reciprocating/circular blade in a frame, sawdust drift). Both read "this is driven by
  the wheel, not a hand." *Lore:* "A hand-quern grinds a family's bread; a powered millstone grinds a village's,
  and turns the miller from a poor man into the one everyone owes. Power is the difference between surviving and
  being needed." *Gate:* NEEDS:DF-POWER + DF-PRODUCTION — the millstone (POW-0's machine).

- **[REQUEST] Animal-treadmill / horse-gin** — the muscle generator (ties DF-LIVESTOCK / DF-JUSTICE prisoner).
  *Creative brief:* a treadmill wheel or a horizontal horse-gin sweep an animal (or a jailed colonist) walks to
  turn a central axle; worn track, a blinkered beast plodding. *Lore:* "When the wind fails and the river
  freezes, the mill turns on legs — a blind old horse, or in a cruel colony, a man who wronged it. The horse
  forgets the circle it walks; they say the man never does." *Gate:* NEEDS:DF-POWER + DF-LIVESTOCK — the horse-
  gin.

## From UNDERGROUND-LIGHTING — "UNDERGROUND LIGHT" batch (2026-07-09)
*(Race-keyed light sources §3c — the mine's lamps + the carried torch. **Anti-samey:** light is where a culture's
whole relationship to the dark shows — humans FIGHT the dark with fire (tallow, smoke, warmth), dwarves OWN it
with cold cyan Velorite-glow (they carve their lamps from the danger, per the ore-vein lore). Vary hard by
CULTURE. Consumed by LIGHT-0/1.)*

- **[REQUEST] Human fire-light set** — torch, tallow-lantern, iron wall-sconce, floor brazier. *Creative brief:*
  warm literal + ember glow (NOT cyan — humans burn things); a pitch torch guttering smoke, a horn-paned tallow-
  lantern, a bracket sconce, a low brazier for a work-face. Soot-blackened, honest, a little smoky. *Lore:* "A
  human lights the dark the only way he trusts — by burning something. The tallow stinks and the smoke stings and
  the light is never enough, but it flickers like a living thing, and a man alone in a deep place will talk to it."
  *Gate:* NEEDS:UNDERGROUND-LIGHTING — the tallow-lantern + wall-sconce first (LIGHT-1's auto-lamp).

- **[REQUEST] Dwarven Velorite-lamp (the signature — the cold-light loop)** — a machined lamp housing a shard of
  the cyan glow-crystal (`glow.cyan`, ties DF-GEOLOGY Velorite). *Creative brief:* brass/dark-stone housing
  (`metal.brass_gold` + `stone.dungeon_dark`) around a caged Velorite shard casting steady cold cyan — NO flame,
  NO smoke, eternal. The lamp on the pit-head, the lamp in the miner's hand, the lamp in the wall-niche. The
  glow-ore you MINE is the light you CARRY — that loop is the point. *Lore:* "The dwarves don't burn the dark,
  they answer it — a Velorite lamp gives the same cold blue the deep gives, so the light and the danger come from
  one stone. It never gutters, never gutters out, and a dwarf trusts it exactly because it isn't alive." *Gate:*
  NEEDS:UNDERGROUND-LIGHTING + DF-GEOLOGY — the Velorite wall-lamp + the carried hand-lamp (the batch centerpiece).

- **[REQUEST] Mine lamp-post / sconce-standard** — the free-standing/wall light the lamplighter drive auto-places
  along tunnels (LIGHT-1). *Creative brief:* a simple repeatable standard — human a timber post with a lantern
  hook + a spare-tallow shelf; dwarven a stone-set bracket with a Velorite bead. Must tile down a tunnel readably
  (it's placed every N tiles). *Lore:* "The lamplighter's round is the loneliest job in the hold — down the dark
  galleries at shift-change, topping the tallow, and everyone else walking the other way toward the light and the
  air. A good lamplighter is never thanked and always missed the one time he's sick." *Gate:* NEEDS:UNDERGROUND-
  LIGHTING — the human timber lamp-post (the auto-placed unit).

## From DF-STRUCT — mine-support addendum (2026-07-09)
*(Supports mostly REUSE the "Mine & Deep Dark" batch — pit-head cribbing + support beams. Just these gaps:)*

- **[REQUEST] Roof-props + shoring frames + standing pillar** — the in-gallery supports the dig plan places to
  hold the roof (STRUCT-1 mitigation). *Creative brief:* a single **roof-prop** (a vertical timber/stone post
  wedged floor-to-ceiling, `wood.oak_beam` / dwarven cut-stone), a **shoring frame** (a squared timber set that
  lines a span — post-cap-post), and a **left pillar** dressing (an un-mined rock column the miners chose to
  leave, tool-marked on its faces). Human rough-hewn + wedged; dwarven fitted + a Velorite bead. These read
  "someone decided the roof needed holding here." *Lore:* "A miner learns to listen to the roof — a good prop is
  set the moment the stone starts talking, not after. The pillars they leave un-mined are an argument the whole
  colony has: every one is ore they'll never dig, and every one is a shift that came home." *Gate:* NEEDS:DF-
  STRUCT — the roof-prop + shoring frame (coordinate w/ the Mine & Deep Dark cribbing — same timber family).

## From SHIPS-NAVAL — "VESSEL CATALOG & HARBOR" batch (2026-07-09)
*(Airship-referenced multi-part vehicles — reference `Body::Ship` SailBoat/Galleon + their structure manifests +
the animated-part rig. **Design every hull with SEPARABLE, RIGGABLE parts** for the animation line-items below.
**Anti-samey:** a boat is a culture's whole relationship to the water — a human fishing skiff is patched, lucky-
charmed, and named after a daughter; a dwarven river-barge is iron-strapped and grim; a coastal-raider warship is
all teeth. Vary by CULTURE + ROLE. NOTE: assets are viable NOW but INERT until NAVAL-MOVEMENT is built — generate
to pressure-test the vessel spec, NOT 40 of them. Consumed by NAVAL-1..3.)*

- **[REQUEST] Fishing skiff (the humble workhorse — do first)** — a small open boat, net/line, a bailing-bucket,
  a single mast + oars. *Creative brief:* patched planks (`wood.plank_warm`), a furled canvas sail on a stubby
  mast, oar-pins, a coil of net; weathered, lucky-charmed (a carved eye on the prow against drowning). Separable
  parts: sail, 2 oars, a flag/pennant, rudder. *Lore:* "A fishing family names its skiff for a daughter and
  paints an eye on the prow so the boat can see the squall the men can't. When a skiff comes home empty they
  blame the eye; when it doesn't come home they don't speak of it at all." *Gate:* NEEDS:naval-movement — 1 human
  skiff (the spec-pressure vessel; separable parts for the anim rig).

- **[REQUEST] Trader cog + river barge** — the DF-TRADE naval haulers. *Creative brief:* a **cog** — a rounded
  single-masted sea-trader with a deep hold + a castle fore/aft (human canvas sail; a race-keyed hull); a **river
  barge** — a flat broad hull, poled/towed, stacked with covered cargo (dwarven iron-strapped + grim). Separable:
  sail, rudder, flags, cargo. *Lore:* "A cog rides low with a full hold and every wave is an argument with the
  captain's greed — load her right and she's a floating warehouse, load her wrong and she's a lesson the sea
  teaches once." *Gate:* NEEDS:naval-movement + DF-TRADE — the cog (sea trade) + barge (river trade).

- **[REQUEST] Warship** — coastal defense (ties B8). *Creative brief:* oars + sail for speed, a ram or a deck
  ballista, shields on the gunwale, a fierce prow (a carved beast-head — culture-keyed: human dragon-prow,
  dwarven iron-tusked). All teeth. Separable: sail, oar-banks, flags, ballista, rudder. *Lore:* "You hear the
  war-galley before you see it — the drum that keeps the oars, and then the oars, and by the time you see the
  prow the only choice left is the beach or the bottom." *Gate:* NEEDS:naval-movement + B8 — the warship.

- **[REQUEST] Harbor infrastructure** — dock/pier, boathouse, lighthouse, harbor crane. *Creative brief:* a
  **dock/pier** (timber piles + planked walk — the naval "depot", boats moor + load, ties B6); a **boathouse**
  (a covered slip for build/repair); a **lighthouse** (a coastal tower + a fire/Velorite beacon — ties the
  UNDERGROUND-LIGHTING light family, but ABOVE ground, a night sea-guide); a **harbor crane** (a timber jib for
  load/unload). Race-keyed, weathered by salt. *Lore:* "The lighthouse-keeper is the loneliest office a colony
  grants and the one it trusts most — a night he sleeps is a ship on the rocks, so they give him the job for
  life and a good chair, and everyone pretends not to know why he drinks." *Gate:* NEEDS:naval-movement (dock/
  crane tie B6) — the dock/pier + lighthouse first.

- **[ANIMATION LINE-ITEMS — NEEDS:animation-code (airship-rig referenced, spec'd per SHIPS-NAVAL §3)]:**
  `anim::sail_billow` (canvas billow/luff, wind-driven — the signature) · `anim::oar_cycle` (crewed rowing
  stroke) · `anim::flag_flutter` (pennants/telltales, wind-driven, high-charm) · `anim::rudder` (steering
  deflection). Design all hulls above with these parts SEPARABLE/riggable. Hull + crew locomotion = NATIVE.

## From HAND-CURSOR — "THE GOD-HAND" (2026-07-09)
*(The one real asset of the pass — the divine hand cursor + its animation set. This is the game's SIGNATURE
image (Black & White): the god IS this hand. It must be beautiful + expressive + characterful — the opposite of
a generic pointer. Design as ONE rig with SEPARABLE riggable fingers for the animation line-items.)*

- **[FULFILLED(asset-lab/vox/godhand/ — 11-part rig (palm + 2-segment fingers + opposed thumb) + rig.json w/ knuckle-pivot skeleton + PARAMETRIC 5-anim spec; weathered marble-flesh, gilt wrist band + palm-crease hairline, cyan tip beads; 11/11 part style PASS, single-component, 360-sheet eyeballed (v1 REJECTED at that eyeball and redesigned — rejection log); animates in studio.html (hand pose system); READY-pending-animation-code 2026-07-10)] The god-hand model** — a detailed divine hand rendered at the cursor / over the world. *Creative
  brief:* NOT a human hand and NOT a cartoon glove — a hand that reads as DIVINE: larger-than-life, weathered
  like carved stone or old bronze but ALIVE, fingers that could cradle a person or crush a hut. Consider a subtle
  otherworldly signature (a faint `glow.cyan` at the fingertips / a hairline of gilt in the creases / a texture
  between marble and skin) so it's unmistakably a GOD's hand, not a player's. It should look capable of both
  blessing and wrath — the same hand does both. Riggable fingers (5 named anims below). Read at cursor scale +
  when it reaches down into the world. *Lore:* "The people do not agree on what the hand looks like — the miller
  swears it is calloused and kind like his father's, the widow that it is pale and terrible, the child that it is
  huge and warm. They are all looking at the same hand. A god is whatever you most need it to be, right up until
  the moment it isn't." *Gate:* NEEDS:HAND-CURSOR — the hand model (HAND-0's presence) — HIGH PRIORITY, this is
  the game's signature cursor.

- **[ANIMATION LINE-ITEMS — NEEDS:animation-code (B&W 2-anim-set + idles style; spec'd per HAND-CURSOR §2)]:**
  `anim::hand_idle` (a SET of 2+ — the hand hovers, breathes, flexes; never frozen — the "alive" read) ·
  `anim::hand_grab_ground` (fingers splay then clench onto terrain on pan-start, release on pan-end — the
  world-drag) · `anim::hand_grab_npc` (reach down + close gently-or-firmly around a colonist — the seizing) ·
  `anim::hand_carry` (hold a colonist while the hand moves — the carried pose, the colonist dangling/cradled) ·
  `anim::hand_release` (open + set down softly, OR let fall — two flavors: the blessing and the drop). The whole
  set must convey CHARACTER (a gentle god vs a rough one reads in HOW the hand moves) — this is the game's most-
  seen animation, spend the care.

## From UI-1 GOD-POWERS-ACCESS — "DIVINE UI ICONS" batch (2026-07-10)
*(The access layer's icons — the god's reach made legible. **Anti-samey:** each power icon must read its ACT at
a glance (a smite ≠ a blessing ≠ a harvest) AND read as DIVINE (a signature the mortal ability-icons lack —
`glow.cyan`/gilt edge, per the god-hand). Match the existing HUD icon language + scale (~16-32px, distinct
silhouettes). Consumed by UI1-0/1.)*

- **[REQUEST] God-power icons (one per catalog power)** — the action-bar + catalog icons for the GOD-POWERS-
  CATALOG powers, grouped by tier. *Creative brief:* per power, a bold readable symbol of the ACT — ① Miracles
  (a smite/lightning-strike, a blessed-harvest sheaf-with-glow, a call-to-shelter hand-over-flock, a still-the-
  breach sealed-fissure, a conjure-water droplet); ② Blessings (a standing aura-ring, a fecundity seed-with-halo,
  a ward-shield, a consecrate-flame); ③ Passives (a fortune-star, a faith-radiance — softer, "ambient" reads).
  ALL carry a faint divine signature (`glow.cyan` rim / gilt) so they're unmistakably a GOD's, not a mortal's.
  *Lore:* "The priests carve the god's signs above the altar so the faithful learn to read heaven's hand — the
  jagged one is wrath, the ringed one is blessing, and the one no one can quite describe is the fortune that
  comes whether you pray or not." *Gate:* NEEDS:UI-1 — the core ~6 (smite / blessed-harvest / call-to-shelter /
  ward / seal-breach / fortune) as the spec batch; the rest as catalog powers firm up.

- **[REQUEST] Category glyphs + favor-bar art** — small category tabs (terrain/weather/life/faith/…) for the
  catalog + the FAVOR/faith resource bar (a divine-resource readout, distinct from the mortal energy bar).
  *Creative brief:* category glyphs = simple domain symbols (a mountain / a cloud / a heart / an altar); the
  favor bar = a radiant fill (gilt/cyan) that reads "divine power to spend", visibly NOT the green stamina bar.
  *Lore:* "Favor fills like water into a font — the more the people love you the higher it rises, and a god who
  spends past the bottom of the font finds, briefly and instructively, what it is to be only watched." *Gate:*
  NEEDS:UI-1 — the favor-bar + 4 category glyphs.

## From UI-2 MISSING-UI-ELEMENTS — HUD-icon note (2026-07-10)
- **[REQUEST — coordinate, don't fork] HUD-icon set for the colony-legibility layer** — panel-tab icons (colony-
  status / stocks / unit / room / governance / divine), alert-type icons (breach / siege / famine / strange-mood
  / cave-in / migration-wave), overlay-legend keys (per map-overlay layer), trend arrows (↑↓ for stock trends).
  *Creative brief:* small 2D UI icons in the EXISTING HUD language — readable at ~16px, distinct silhouettes,
  monochrome-or-2-tone. **ONE coherent HUD-icon language: coordinate with the DF-HIST chronicle-glyph batch
  (already routed) + the UI-1 divine-icon batch — reuse/extend, do NOT fork a third style.** *Lore:* not needed
  per-icon (functional UI); the SET should feel like one hand drew the whole HUD. *Gate:* NEEDS:UI-2 platforms —
  the alert-type + panel-tab icons first (they surface the corpus's most urgent reads).

## From UI-4 DIALOGUE-AND-UNIT-SELECTION — panel-frame note (2026-07-10)
- **[REQUEST] Dialogue/event-box + selection-panel FRAMES** — the one real new asset (portraits + icons REUSE
  existing). *Creative brief:* two HUD panels in the existing conrod style — (a) the **dialogue/event box** (a
  scrolling event-feed region + a dialogue region + a portrait slot; reads as "the colony's/world's voice"), and
  (b) the **unit-selection panel** (a tabbed sheet frame: Overview/Mind/Needs/History/Skills/Relationships — a
  portrait slot + tab strip). Match the HUD language; frame art only (contents are UI-in-code). *Lore:* n/a
  (functional UI). *Gate:* NEEDS:UI-4 — the two panel frames. **REUSE, don't fork:** portraits = existing NPC/
  role icons (v1, no new art); event/dialogue-type icons = the DF-HIST chronicle-glyph batch + UI HUD-icon set
  (already routed — ONE HUD-icon language).

## From UI-5 GOOD/EVIL HAND + DIVINE EFFECTS — "THE HAND'S TWO FACES + DIVINE VFX" batch (2026-07-10)
*(The Black & White signature. The NEUTRAL hand = the HAND-CURSOR base (already routed) — this batch is the two
ENDS of the spectrum + the effects. **Design as ONE rig with an alignment-driven material+geometry BLEND** so the
5 hand animations inherit the morph. The hand is the game's most-seen object AND the truth about the player —
spend the care. Consumed by ALIGN-0/1.)*

- **[REQUEST] The GOOD hand (the giving hand)** — the benevolent end of the alignment spectrum. *Creative brief:*
  the neutral weathered hand made LUMINOUS + KIND — smoother, warmer stone/flesh, a soft inner radiance, hairline
  gilt in the creases, `glow.cyan`/gold at the fingertips, maybe a faint corona; the SHAPE reads open, giving,
  gentle (fingers that cradle, not grip). It should make a player FEEL they've been merciful. *Lore:* "A god
  who has been kind grows a kind hand, though no one can say when it happened — the widow who has watched the sky
  her whole life will tell you the hand was terrible once, in her grandmother's time, and is gentle now, and that
  this is the only proof of grace anyone has ever needed." *Gate:* NEEDS:UI-5 — the GOOD hand (the spectrum's
  bright pole; morph-blends from the HAND-CURSOR neutral base).

- **[REQUEST] The EVIL hand (the taking hand)** — the cruel end. *Creative brief:* the neutral hand CLAWED +
  DARKENED + WRONG — the stone cracked and blackened, veins of `accent.blood_red` / `accent.cultist_purple`
  under the skin, nails grown to claws or talons, knuckles spiked, a low ember/shadow smoke clinging; the SHAPE
  reads grasping, taking, a fist waiting to close. It should make a player FEEL the weight of their cruelties —
  not cartoon-devil, but genuinely unsettling, a hand you'd fear over your rooftop. *Lore:* "The claws come in
  slowly, one cruelty at a time, and the god is always the last to notice — it is the children who see it first,
  the way they see everything, and stop waving at the sky, and no one has to tell them why." *Gate:* NEEDS:UI-5 —
  the EVIL hand (the spectrum's dark pole).

- **[REQUEST] Divine-effect VFX presets (per-power × alignment, on the Outcome/ParticleMode bus)** — the cast
  effects. *Creative brief:* per power, a cast VFX + a GOOD-tint and an EVIL-tint variant (reuse the reagent/
  ParticleMode preset mechanism, NOT a new particle system): a **smite** (good = a clean white/gold bolt; evil =
  a jagged red-black strike + ash), a **blessing** (good = a warm gold/petal shimmer; evil-god's blessing =
  a sickly bound-by-fear glow), a **heal** (green/cyan motes), a **conjure-water** (a bright splash), a **call-
  to-shelter** (a radiant beacon vs an ominous summons). The SAME power wears the god's face. *Lore:* "A blessing
  from a good god falls like warm rain; the same word from a cruel one falls like a debt — the crops grow either
  way, but the farmers learn to flinch at the light." *Gate:* NEEDS:UI-5 + GOD-POWERS-CATALOG — the smite +
  blessing presets (both alignment tints) first.

- **[REQUEST] The hand's aura + trail (per alignment)** — the ambient divine glow that follows the hand.
  *Creative brief:* GOOD = a soft light halo + a trail of drifting motes/light as the hand moves (glow +
  ParticleMode); EVIL = a dark haze + an ember/smoke trail, the light around it dimmed. Reads the alignment even
  when no power is cast — the hand always announces the god. *Lore:* "You can read a god's temper by what trails
  its hand across the sky — light that lingers like a blessing, or a smoke that the birds won't fly through."
  *Gate:* NEEDS:UI-5 — the two aura/trail presets (good motes / evil embers).

## From GOD-HAND (definitive) — "THE GOD-HAND: COMPLETE RIG + ANIM SET" (2026-07-10)
*(AUTHORITATIVE + CONSOLIDATED — supersedes the earlier HAND-CURSOR hand-model request + the UI-5 good/evil-hand
request. This is the FULL spec so nothing is discovered late (Ben's ask): ONE rig, the complete ~15-animation set,
the alignment blend, the divine VFX. The game's signature object — spend the care.)*

- **[REQUEST — AUTHORITATIVE] The god-hand RIG (one rig, alignment-blend)** — a detailed DIVINE hand (see the
  HAND-CURSOR + UI-5 briefs, now consolidated). *Creative brief:* a hand that reads as a god's — larger-than-
  life, between marble/bronze and living flesh, capable of blessing AND wrath. Built as ONE rig with an
  ALIGNMENT-DRIVEN material + geometry blend spanning GOOD (luminous, gilt, smooth, radiant, giving) ↔ NEUTRAL
  (weathered — the base) ↔ EVIL (clawed, dark-cracked, `accent.blood_red`/`accent.cultist_purple`-veined,
  spiked, ember-smoking, taking). Riggable fingers for the full anim set below. *Lore:* "The people do not agree
  what the hand looks like — kind as a father's, or pale and terrible — and they are all looking at the same
  hand, on the same day; it is only the god who cannot see which." *Gate:* NEEDS:GOD-HAND — the rig at the
  neutral base first, then the two alignment poles.

- **[REQUEST — the COMPLETE animation set, NEEDS:animation-code, per GOD-HAND §2]:** all on the one rig, each
  inheriting the alignment morph (a gentle vs cruel version is the SAME anim through a different hand):
  `anim::hand_idle` (a SET, 2+) · `anim::hand_point` · `anim::hand_grab_ground` (splay→clench on pan) ·
  `anim::hand_select` (tap/touch) · `anim::hand_grab_npc` · `anim::hand_carry` · `anim::hand_release` (gentle
  set-down / let-fall — 2 flavors) · `anim::hand_throw` (wind+hurl, `states/throw.rs` timing) · `anim::hand_
  stroke` (gentle pat — the KIND verb) · `anim::hand_slap` (slap/flick — the CRUEL verb) · `anim::hand_cast`
  (casting flourish) · `anim::hand_gesture` (draw-a-shape, optional B&W flavor) · `anim::hand_paint` (brush/
  sweep — designation + blessing) · `anim::hand_sculpt` (scoop/press — terrain, From-Dust-style) · `anim::hand_
  descend` (plunge + dissolve into a body — the embody transition). ~15 anims = the corpus's largest single
  animation debt, but COMPLETE + named up front. *Priority:* idle + grab_ground + grab_npc/carry/release +
  stroke + slap + cast first (the most-seen + the good/evil-defining pair).

- **[REQUEST] Divine-effect VFX presets (per-power × alignment) + hand aura/trail** — per GOD-HAND §4 / the
  UI-5 brief (consolidated here): on Veloren's Outcome/ParticleMode/glow bus (NO new particle system) — smite
  (good gold-bolt / evil red-black strike), blessing (warm shimmer / fearful glow), heal-motes, conjure-splash;
  + the hand aura/trail (good = light+motes / evil = haze+embers). *Gate:* NEEDS:GOD-HAND + GOD-POWERS-CATALOG —
  the smite + blessing presets (both tints) + the two aura/trail presets first.

- **[FULFILLED(poles: asset-lab/vox/godhand_good/ + godhand_evil/ — 11 parts each, SAME skeleton/offsets as the
  shipped neutral so all 21 anims play at any band; band breakpoints + geometry gates (claws/spikes ≤ −0.5, gilt
  ≥ +0.5) in each rig.json alignment_blend; good = warmed skin + healed cracks + knuckle-worked gilt + gold tips,
  evil = blackened cracked stone + blood/cultist-purple subdermal veins + talons + 2-tall knuckle spikes +
  tarnished wrist gilt + ember tips; per-part style/connection PASS ×22, assembled single-component, 360 sheets
  eyeballed ×2 rounds (renders/godhand_good_assembled_360.png + evil); the 5 NEW definitive-set anims
  hand_select/stroke/slap/gesture/descend added to vox/godhand/rig.json (now 21) + studio dropdown w/ pose
  curves; VFX: asset-lab/vfx/divine_vfx_presets.md — smite+blessing both tints first, heal/conjure/shelter,
  aura/trail pair, ALL rows on the existing Outcome/ParticleMode/LightEmitter bus (real variant names cited,
  clone-recolor pattern where color is shader-baked); studio 139 entries incl. both poles; 2026-07-10 overnight)]
  **[was REQUEST — LIVE / ACTIONABLE, issued to UNBLOCK the pilot (2026-07-10)] THE HAND'S TWO FACES + DIVINE VFX
  — the good/evil alignment brief, spelled out.** *(The two-poles + VFX detail existed only inside the UI-5
  batch above, which THIS authoritative section marked "superseded" — so it read as dead and never landed
  actionably. This entry pulls it forward as the LIVE spec: treat THIS as the good/evil + VFX brief; the UI-5
  section's four rows are consolidated here, not cancelled. The NEUTRAL v3 hand + its anim set are BUILT
  (`asset-lab/vox/godhand/`, READY-pending-animation-code) — this is NOT two new models; it is ONE alignment-
  driven material+geometry MORPH over that shipped rig, so every existing anim inherits the face automatically.)*

  **① THE TWO FACES as ONE alignment-blend (a morph over the shipped neutral rig, per UI-5 ALIGN-0):**
  - *Neutral = the base* (the built weathered marble/bronze hand). The face is a **single blend parameter**
    (alignment −1..+1) driving material + geometry from one pole to the other through neutral — NOT three
    separate sculpts. Deliver the two POLE targets + the neutral base; the engine samples the continuum.
  - *GOOD pole (giving hand):* **material** — the stone/flesh grows smoother and warmer, a soft inner
    **radiance** (emissive), hairline **`metal.brass_gold` gilt** worked into the palm-creases + knuckles,
    fingertip beads warming from `glow.cyan` toward warm gold, a faint corona. **geometry** — knuckles soften,
    fingers relax OPEN, the whole posture reads *cradle, not grip*. A player should FEEL they've been merciful.
  - *EVIL pole (taking hand):* **material** — the stone **cracks and blackens** (`stone.dungeon_dark`),
    subdermal veins of **`accent.blood_red` / `accent.cultist_purple`** under the skin, a low ember/shadow
    smoke, the ambient light around it DIMMED. **geometry** — nails grow to **claws/talons**, knuckles spike,
    the posture reads *grasping, a fist waiting to close*. Genuinely unsettling — a hand you'd fear over your
    roof — NOT a cartoon devil-hand.
  - *THE DRIFT (the legibility — the point):* the blend is a **spectrum with legible BANDS** (author ~5 read-
    at-a-glance steps per side: neutral → touched → marked → deep → pole) so a glance tells you roughly who the
    god has been; **GRADUAL** (drifts one deed at a time) and **REVERSIBLE** (kindness walks the claws back).
    Deliver the pole morph-targets + the band breakpoints; each of the ~15 anims plays identically at any band.

  **② PER-POWER × ALIGNMENT VFX PRESETS** (good-tint + evil-tint per power, on Veloren's **`outcome.rs` Outcome
  / `reagent` / ParticleMode / glow bus — REUSE the preset mechanism, NO new particle system**). The same power
  wears the god's face:
  - **Smite** — good = a clean white/gold bolt, crisp report; evil = a jagged **red-black** strike trailing ash.
  - **Blessing** — good = a warm gold/petal shimmer that falls like rain; evil = a sickly **fear-bound** glow
    (the crop grows, but the light is a debt).
  - **Heal / mend** — good = green/`glow.cyan` motes rising; evil = a bruised, grudging green (life at a price).
  - **Conjure-water** — good = a bright clean splash + light spray; evil = a dark brackish welling.
  - **Call-to-shelter** — good = a radiant protective beacon; evil = an ominous summons (they come out of fear).
  - *Priority:* the **smite + blessing** presets (BOTH tints) first — they are the good/evil-defining pair.

  **③ THE HAND AURA + TRAIL (per alignment, idle-persistent):** GOOD = a soft light halo + a trail of drifting
  motes/light as the hand moves; EVIL = a dark haze + an ember/smoke trail with the light around it dimmed.
  Reads the alignment **even when no power is cast** — the hand always announces the god. (glow + ParticleMode;
  two presets.) *Priority:* the two aura/trail presets after the smite+blessing VFX.

  *Lore (the signature object — the god's soul made visible):* "The hand is the one thing a god cannot lie
  about. A god may call itself just and merciful, may hear its own name sung kindly in the temples — but the
  hand keeps the honest account: a little gilt for every mercy, a little of the claw for every cruelty, drawn
  in so slowly that the god is always the last to know its own shape. The people read it long before the priests
  admit it. That is the whole of theology, the old woman says: not what the god says it is, but what its hand
  has quietly become." *Gate:* NEEDS:GOD-HAND (rig BUILT) + GOD-POWERS-CATALOG (the `alignment_weight` +
  `cast_vfx` columns — ALIGN-0/1). **Order:** the two pole morph-targets first (unblocks ALIGN-0), then smite +
  blessing VFX (both tints), then the aura/trail pair.

## From AGENT-CULTURE-CHARACTERIZATION — culture-priority + name-pool note (2026-07-10)
- **[NOTE → pilot] CULTURE-PRIORITY LIST for race/culture variety** (feeds the pilot's existing race-keyed asset
  work — no NEW .vox batch, this is the ORDER to key against): **Tier 1 (now) = HUMAN + DWARVEN** (the founding-
  colony + deep-mining core — the vertical world is dwarven-flavored) · Tier 2 = ELF + ORC + GNARLING (migration/
  rivalry/antagonist) · Tier 3 = Danari, Draugr + faction site-cultures (Sahagin=naval, Myrmidon, Cultist, Adlet,
  Terracotta…) as their frontier reaches them. RULE: key a culture across ALL 5 axes together (stats+behavior+
  history+relations+language) or it reads as a reskin.
- **[REQUEST — content/writing, not .vox] Per-culture NAME POOLS + dialect flavor-lexicons** — extend Veloren's
  name-gen (i18n `name.ftl` pools) with per-culture personal + place-name pools + a small dialect term-lexicon,
  so a dwarf is NAMED dwarven (stone/metal roots, guttural) and a hold is named in-culture, an elf elven (flowing),
  etc. *Creative brief:* each culture's names should SOUND like its people — dwarven hard consonants + stone/forge
  roots; elven long vowels + nature/light; orcish blunt + blood/war; human plain/varied. Plus ~10-20 dialect
  terms/culture the chronicle can flavor with ("the Underneath" = dwarven for the deep-dark). *Lore:* the naming
  IS the lore — a name carries the culture. *Gate:* NEEDS:AGENT-CULTURE — Tier-1 Human + Dwarven pools first
  (per the priority list). A full generated conlang is DEFERRED (Tier-3); v1 = pools + dialect flavor.

## From DF-FESTIVAL — "FEAST-DAY / THE COLONY CELEBRATES" batch (2026-07-10)
*(The joy beat. **Anti-samey framing:** a festival is where a colony shows its CULTURE and its FORTUNE at once —
a human harvest-home (fire, long table, ribbons) reads nothing like a dwarven stone-day (a carved pillar, cyan
lamps, deep drums) or a gnarling revel (totems, bone, war-paint). Vary by CULTURE (§7) and by SEASON (a harvest
table groans gold; a midwinter feast is fire-and-fur). Reuses DF-COOK food models + DF-ROOMS décor. Consumed by
FEST-0..2.)*

- **[FULFILLED(asset-lab/vox/prop_festival_bonfire_human.vox — a BUILT crib pyre: 4 corner log-posts carry
  crossed oak/plank courses (one connected stack, no floating logs), packed kindling floor, a continuous live
  ember/flame core on the figure glow band 14/15 (declared to both harnesses), a crooked straw-man crown lashed
  atop with arms out to burn; head-taller-than-a-work-fire proportions; style(prop)+function+connection single-
  component PASS, 360 eyeballed; dwarven brass-bowl version deferred to FEST-2 per brief; 2026-07-10)]**
  **[was REQUEST] Festival bonfire / celebration-fire** — the heart of a festival ground; the fire the colony gathers
  around. *Creative brief:* a big built pyre — stacked logs (`wood.oak_beam`) with a live-fire glow-band + ember
  spray, taller and prouder than a work-fire; a human version rough-stacked with a straw-man crown to burn, a
  dwarven version a brass fire-bowl on a carved plinth with `glow.cyan` under-light. Asymmetric, generous, ALIVE
  (it's the party's pulse). *Lore:* "The feast-fire is built a head taller than the tallest man present, so that
  even the dead, the old ones say, can see it from wherever they are and know the colony made it through another
  year — and come, if they like, and stand at the edge of the light where the living pretend not to notice them."
  *Gate:* NEEDS:DF-FESTIVAL — 1 (human) spec placeholder now; dwarven on FEST-2.

- **[FULFILLED(asset-lab/vox/prop_feast_table_human.vox — a heavy scarred-oak trestle GROANING with the harvest:
  a roast platter heaped center, bread loaves clustered + stacked, ale tankards, a wildflower bunch, and the
  STORY beat — a knocked-over cup with the spill running to the table edge; two benches shoved in (one askew,
  pushed out as if someone rose in a hurry); trestle A-frame legs + stretcher, not 4 posts; classed as 3 ground-
  seated objects (table+benches, flora-clump connection rule); reuses DF-COOK bread/roast/ale grammar; harvest-
  laden load per brief (lean-winter reload = sparser is a SEASON param the zone can swap); style+function PASS,
  360 eyeballed; 2026-07-10)]**
  **[was REQUEST] Laden feast-table / trestle** — the long board the feast is set on (the surplus made visible).
  *Creative brief:* a heavy trestle table GROANING with the year's food (reuse DF-COOK bread/roast/ale models
  piled generous), benches shoved in, a spilled cup — reads abundance, not a tidy dinner. Human = scarred oak +
  wildflowers; dwarven = a dark-stone slab + brass plate + a cyan lamp. Vary the load by SEASON (harvest = piled
  gold; lean-winter = sparser, more meat/fire). *Lore:* "You can measure a year by the feast-table: the good
  years it bows in the middle under the weight, and the bad years the elders eat least of all and loudest, so the
  children never learn to count the empty places." *Gate:* NEEDS:DF-FESTIVAL — 1 (harvest-laden human) spec.

- **[FULFILLED(asset-lab/vox/prop_bunting_line_human.vox — a strung festoon: two hand-set lean poles, a FACE-
  CONNECTED swagged catenary cord (the sag steps are bridged vertically so the cord is one line, not diagonal
  beads — the #13 connection catch in miniature), triangular cloth pennants hanging straight down in the colony's
  colours (cream.plaster_wool + fur.earth_brown, alternating); the wind-stir read comes from VARYING pennant
  LENGTH per position (a lateral wobble would detach the columns); single connected component; ties
  anim::flag_flutter (naval-set reuse — the pennants are single-plane cloth, flutter-ready); style+function+
  connection PASS, 360 eyeballed; human pennant-line first per brief; 2026-07-10)]**
  **[was REQUEST] Banners, bunting & garlands** — the cheap big charm-lever that turns a plain ground into a festival.
  *Creative brief:* strung lines of cloth pennants (a colony's colours — hook the culture's tapestry ramps:
  human `cream.plaster_wool` + `fur.earth_brown`; dwarven `metal.brass_gold` + `stone.dungeon_dark`), leaf/
  flower garlands (harvest), a banner on a pole. Off-center, wind-stirred (ties `anim::flag_flutter` from the
  naval set — reuse). *Lore:* "The bunting comes down from the same chest every year, a little more faded, and
  the year they hang it new everyone knows someone came into money — or someone died and left the old ones cursed
  with grief they'd rather bury under fresh colour." *Gate:* NEEDS:DF-FESTIVAL — the human pennant-line first.

- **[FULFILLED(HUMAN pole: asset-lab/vox/prop_maypole_human.vox — a tall oak pole wound with FOUR bright ribbons
  (red/gold/green/harebell) spiralling down as a real helix (each ribbon occupies one of the 4 pole-adjacent
  cells per height, rotating with descent — reads as wound ribbon + guaranteed single-component), a brass carved
  cap + garland-ring knot at the top, and a worn trodden dance-RING of bare earth at the foot with a couple of
  dropped flowers (the ring that never quite grows back); ground-seated (pole + ring, flora-clump rule);
  FESTIVAL-GROUND zone marker EXTENDS the existing meeting-totem per the board ruling — NO new marker byte forked;
  style+function PASS, 360 eyeballed. Dwarven stone-day pillar + gnarling revel-totem DEFERRED to FEST-2 per the
  culture-priority list; 2026-07-10)]**
  **[was REQUEST] Dance-totem / maypole (culture-keyed centerpiece)** — the "this is where the celebration happens"
  focal, per culture. *Creative brief:* NOT one recolored pole — a **human** ribbon-maypole (a tall pole, bright
  ribbons spiralling, worn dance-ring at its foot); a **dwarven** carved stone-day pillar (a squat rune-cut
  monolith, `glow.cyan` inlay, struck like a bell); a **gnarling** revel-totem (crooked stacked skulls + war-
  paint, gnarling-totem lineage). Each says how its people rejoice. *Lore:* "Every people keeps one pole they
  only raise for joy. The humans wind theirs with ribbon and the dwarves ring theirs like a bell and the
  gnarlings — well. You always know which festival you've wandered into by what stands in the middle of it."
  *Gate:* NEEDS:DF-FESTIVAL — the human maypole + dwarven stone-day pillar as the culture-contrast pair.

- **[NOTE — reuse] Festival-ground marker** = a `ZoneKind` marker (the DF-ZONES zone-marker-post family — a
  "gathering/festival" post); do NOT fork — extend the meeting-totem lineage already requested. Food = DF-COOK
  models (reuse). Fire glow = existing glow-band tech.

## From DF-NIGHT — "THE THINGS IN THE DARK" batch (2026-07-10)
*(The nightly menace. **Anti-samey framing:** night-creatures are the dark given a shape — they must read WRONG
and read DANGEROUS at a glance, graded by menace-tier (a shallow scavenger vs the deep horror the colony names).
Mostly REUSE — the fell-wolf/barrow-troll recolor pattern applied to night/undead/deep fauna — so this
**COORDINATES with the already-requested "deeper-tier cavern-life" batch; do NOT fork a parallel set.** The
werebeast transform uses a NATIVE creature body (no new rig). Culture-neutral (the dark has no culture); vary by
MENACE-TIER. Consumed by NIGHT-0..2.)*

- **[REQUEST — coordinate w/ deeper-cavern-life, don't fork] Night-creature reskins (menace-by-dark)** —
  recolor/rescale variants of shipped night/undead/deep fauna that emerge in the unlit dark. *Creative brief:*
  take night-active + undead + deep-crawler skeletons and push them toward the WRONG — pale eyeless things that
  hunt by sound, glow-eyed apex predators (`accent.blood_red`/`accent.cultist_purple` eye-spots), a grey-rotten
  revenant, gaunt over-limbed silhouettes; light should make them flinch/recoil (the ward reads visually).
  Inherit animation free (generate-to-skeleton). *Lore:* "They are not new, whatever the young ones say. They are
  only the same old dark, wearing whatever it has eaten lately. The lamp doesn't kill them — nothing the colony
  has kills them — the lamp just reminds them that they, too, were once afraid of something, and buys you until
  it burns down." *Gate:* NEEDS:DF-NIGHT + species-reg code — 1 shallow + 1 deep exemplar (coordinate w/ the
  cavern-life batch's exemplars — SAME set, extended).

- **[FULFILLED(asset-lab/vox/creature_night_horror.vox + _rig.json — an over-tall (17×13×31) GAUNT wrong-jointed
  biped: stone.dungeon_dark flesh, back-bent digitigrade knees, arms hanging PAST the knees with splayed talon-
  fingers, a low-slung forward head carrying a CLUSTER of too-many pale cold eyes (glow band 14) + 2 blood eye-
  spots (#cb0000), cultist-purple subdermal seams down the ribs/spine (band 15); the hunched reaching silhouette
  reads "run" at a glance (360 eyeballed, all 4 views). Conformed to the native biped_large envelope + bone set
  (troll/ogre/WENDIGO lineage) so it inherits stand/run/attack FREE (generate-to-skeleton); ward-light read =
  a FLINCH modifier on the shipped anim keyed off nearby LightEmitter (NOT a new anim), spec'd in the rig.json.
  Culture-neutral APEX tier. This is the ONE original distinct silhouette — the night-reskin set stays COORDINATED
  with (not forked from) the cavern-life batch. style(creature-biped-large)+connection single-component PASS;
  2026-07-10)]**
  **[was REQUEST] The signature night-horror (the one the colony tells stories about)** — the apex menace of a bad
  night, a distinct silhouette. *Creative brief:* NOT just a bigger wolf — something the deep dark makes that has
  no daytime equal: over-tall and wrong-jointed, or a swallowing dark with too many pale eyes, or a thing that
  wears the shape of the last miner it took. `stone.dungeon_dark` body, a wrongness-purple/blood glow, a
  silhouette that reads across the colony and empties the streets. Reads "run" before it reads anything else.
  *Lore:* "Every colony has one it has named — not to summon it, the priests are careful to say, but because a
  thing with a name can at least be prayed against. Ours took the bell-ringer first, the year the shaft went too
  deep, and rang the bell himself the second night, in his voice, to see who would come. We do not open for the
  bell anymore. That is the price of a name." *Gate:* NEEDS:DF-NIGHT — 1 signature-horror spec placeholder.

- **[NOTE — reuse, no new asset] The werebeast transform** = a body-swap to a shipped creature body (NATIVE anim,
  DF-SYNDROME SYN-2) — no new rig. **The ward-light** (the relief) = already the UNDERGROUND-LIGHTING batch
  (lamps/sconces/Velorite-lamp) — reuse, do not fork.

## From REPUTATION (S1) + GOD-EPITHET (S2) — near-asset-free NOTES (2026-07-10)
- **[NOTE — no new .vox/anim] REPUTATION** is data surfaced in the **UI-4 inspector** (reuse). Its only pipeline
  touch = **2 chronicle glyphs** (*rose-in-standing* / *fell-in-disgrace*) that **join the DF-HIST event-glyph
  batch** (already routed — one glyph language) — do NOT fork a set.
- **[NOTE — content/writing, not .vox] GOD-EPITHET** is data + text surfaced in the **faith/god readout + the
  UI-4 dialogue box** (reuse). Its content need = **per-culture DIVINE EPITHET name-pools** (good/neutral/evil
  bands — "the Merciful"/"the Distant"/"the Wrathful", named in-culture) that **join AGENT-CULTURE's per-culture
  name-pool authoring** (Human+Dwarven first per the priority list) — do NOT fork; it's the same naming system,
  aimed at the god. *Lore is the deliverable* (the epithets ARE lore).

## From UNDERGROUND-EXPERIENCE — VFX/tuning notes (mostly wiring shipped systems, 2026-07-10)
*(This pass is reuse-heavy — the camera occlusion framework + CaveDust/Drip particles + cave ambience already
ship. These are TUNING/config + small VFX presets, NOT model batches. Consumed by UX-ATMO-0 / UX-HAND-LIGHT-0.)*
- **[NOTE — config, no new art] The god-hand's LIGHT** = a `comp::LightEmitter` preset attached to the cursor's
  world-position (`bastion::unproject_to_world_plane` at the work-layer depth), **alignment-TINTED by reusing the
  UI-5 hand aura/trail presets** (good = warm gold / neutral = base cyan-white / evil = cold red-purple). No new
  asset — a light config + the existing tint. *Guard:* a VIEW aid (reveals the dark as the hand moves), NOT a
  gameplay light (doesn't ward DF-NIGHT / speed work — those key off placed/carried Option-B lights).
- **[REQUEST — VFX tuning] Depth/pressure fog + dug-tunnel particulate** — tint + thicken the shipped shader fog
  with DEPTH (keyed off `CullingMode::Underground` + DF-CAVERN danger tier — shallow damp/calm → deep heavy/wrong,
  `accent.cultist_purple` tint); densify `ParticleMode::CaveDust`/`Drip` in the colony's DUG tunnels (not just
  wild caves), heaviest near active mining. *Lore:* "The deep has a weather of its own — a dust that never
  settles and an air that leans on you, thicker the farther down you dig, until the lamplight itself seems to
  wade." *Gate:* NEEDS:UNDERGROUND-LIGHTING Option-B + DF-CAVERN — a depth-fog tuning + the dug-tunnel dust wire.
- **[REQUEST — VFX+audio] The BREACH sting** — the dread cue when a pick meets the natural dark (DF-CAVERN
  Breach). A **dust-burst** (reuse `ParticleMode::CaveDust`/`Dust`) + a **low echo/hollow-note audio sting**,
  fired off the HAZARD-EVENTS `HazardKind::Breach`/`CaveIn` bus (extend the cave ambience/echo, already shipping).
  *Lore:* "There is a sound a pick makes when the next swing meets nothing, and a smell of air never breathed —
  the old hands drop their tools at that note." *Gate:* NEEDS:HAZARD-EVENTS + DF-CAVERN — the breach dust+sting.

## From TOOLS-UPGRADE — "TOOL TIERS" batch (2026-07-10)
*(The upgrade progression made visible. **Anti-samey framing:** a tool's TIER is its story — a lashed stone pick
reads nothing like a clean-forged steel one; the colony's fortune is in its miners' hands. Mostly RECOLOR/detail
variants of the shipped pick/shovel/axe (the item-recolor pattern), NOT new models. Vary by TIER (crude→refined)
and by CULTURE where sensible (§7 — dwarven tools run to brass/cyan precision). Consumed by TOOL-0..2.)*
- **[FULFILLED(the PICK across the ladder — the priority tier-contrast set: asset-lab/vox/item_pickaxe_crude.vox
  (chipped stone head LASHED with cord to a rough knotty oak haft — asymmetric, lumpy, primitive; stone.neutral +
  wood.oak_beam) → item_pickaxe_iron.vox (asset36, the forged twin-spike mid rung, unchanged) →
  item_pickaxe_steel.vox (clean bright forged steel, planed haft, steel ferrule) → item_pickaxe_dwarven.vox
  (steel head + brass_gold socket collar + a glow.cyan rune-bead at the crown, band 14). Same native shaft+head
  silhouette across all four so only the TIER changes — crude→refined reads at a glance (the brief's core ask);
  the DF-QUALITY masterwork stamp REUSES the dwarven treatment, not a forked set. style(item-held)+connection
  single-component PASS ×3 new, 360 eyeballed. Shovel/axe tiers follow the same pattern on TOOL-1/2; 2026-07-10)]**
  **[was REQUEST] Tool-tier variants (pick / shovel / axe, per material tier)** — the same tool across the upgrade
  ladder so a colonist's tier READS. *Creative brief:* a **crude** tier (a stone/flint head lashed to a rough
  haft with cord — asymmetric, primitive, `stone.neutral` + `wood.oak_beam`), a **mid** tier (forged iron head,
  planed haft, `metal.iron`), a **high** tier (clean steel / dwarven brass-and-cyan precision, `metal.brass_gold`
  + a `glow.cyan` bead for dwarven). **★ TIER = MATERIAL + COLOR, NOT SILHOUETTE (pilot-correction confirmed):**
  keep the **SAME recognizable tool shape across every tier** (a pick is always the same pick) — the tier reads by
  **material + colour** (stone-grey → iron → bright steel → dwarven brass+cyan) + at most a *subtle* finish cue
  (a lashing vs a forged socket), never a different SHAPE/silhouette. Held-item scale; **reuse the one native
  pick/shovel/axe silhouette per tool**, recolour by tier. *Lore:* "You can date
  a colony by the tools in its hands — the lashed stone of a first hard winter, the honest iron of a colony that
  found its vein, the bright steel of one that lived long enough to get good at living. A miner's pick is his
  whole biography, worn smooth at the grip." *Gate:* NEEDS:TOOLS-UPGRADE — the pick at crude/iron/steel as the
  tier-contrast spec set (it's the most-used tool + carries the whole progression read).
- **[NOTE — reuse, don't fork] Masterwork tools** = the **DF-QUALITY masterwork treatment** (brass inlay + cyan
  rune-line + a carved flourish) applied to a top-tier tool — reuse that stamp, do NOT fork a separate ornate set.

---
# ═══ GAP-AUDIT ASSET FILL (2026-07-10) — the ~12 designed systems that had NO request filed ═══
*(Consolidation, not invention: these systems were designed with an in-doc "small note" but never got a proper
ASSET_REQUESTS entry. Filed here as the pilot's queue. Most are REUSE-heavy — VFX presets, recolors, one item —
which is WHY they slipped; still, per the "assets are a mandatory pass step" rule, they belong on the board.
Priority order: the near-frontier/showpiece-adjacent first (VILLAIN/BEAST/TEMP/OMEN), the downstream ones after.)*

## From DF-VILLAIN — "THE NAMED ENEMY" batch (gap-fill)
*(A nemesis must read as THE one, not a mook. REUSE shipped hostile bodies + the recolor/gear/standard pattern.)*
- **[REQUEST] Nemesis distinguishing treatment** — a **war-standard/banner** (a warlord's device on a pole) + a
  **scarred/gear recolor** of a shipped hostile body (a captured-armor, a trophy-cloak, a distinguishing scar or
  size-up) so a named villain reads as a saga-figure. Culture-keyed banners for a warlord's host. *Creative brief:*
  take a bandit/orc/gnarling body → mark it (a broken-crown standard, `accent.blood_red` war-paint, a stolen
  piece of the colony's own gear worn as a trophy); the mark says "this one has a name." *Lore:* "You know the one
  who matters by what he wears that used to be yours — the captain's cloak off the man he killed at the first
  gate, worn every raid since, so you cannot forget, so you cannot pretend the wall did not once fail." *Gate:*
  NEEDS:DF-VILLAIN (rides B8) — 1 standard + 1 marked-body as the spec pair.

## From DF-BEAST — "LEGENDARY BEAST & TROPHY" batch (gap-fill)
*(The megafauna SHIP; a legend = a decorated variant + its trophy. Coordinate w/ the DF-NIGHT signature-horror.)*
- **[REQUEST] Legendary-beast variant treatment** — a scar/scale-up/aura pass on a shipped `biped_large` (Gigas/
  Wendigo/Cyclops/Harvester) so a region-apex beast reads as ancient + named. *Creative brief:* an old scar, a
  frost-rime or ember-glow aura (reuse the glow ramps), a size/silhouette tweak (extra tusk, a broken horn), a
  hide gone pale/grey with age — it should read "older than the colony." *Lore:* "The young ones are grey; the
  old one on the high snow is white, and missing an eye a hunter took a generation ago and did not live to boast
  of." *Gate:* NEEDS:DF-BEAST (titans ship) — 1 legendary variant.
- **[REQUEST] Beast trophy (the hunt's prize)** — a mounted skull/hide/horn from a slain legendary beast (DF-ART/
  DF-ROOMS trophy — extends the already-requested trophy-skull to a MASSIVE beast scale). *Creative brief:* a
  colossal skull on the great-hall wall, a hide rug bigger than a bed, a single horn used as a horn — the object
  that says "we killed the mountain's monster." *Lore:* "It took nine to carry the skull down and the whole fort
  came out to watch them do it; it hangs in the hall now, and the children dare each other to touch the teeth."
  *Gate:* NEEDS:DF-BEAST — 1 trophy (reuse the DF-ROOMS trophy pattern at beast scale).

## From DF-TEMP + BIOME-FX — "CLIMATE" batch (gap-fill)
*(PLACE & SEASON made visible. REUSE Veloren clothing/armor + particle/shader; recolors + VFX, not new bodies.)*
- **[REQUEST] Cold-weather clothing / furs** — heavy fur cloaks, hoods, wraps (recolor/material variants of the
  shipped clothing) the colony wears against cold. *Creative brief:* thick `fur.earth_brown` cloaks + hoods,
  layered wraps, breath-fog; a tundra colonist bundled vs a desert one in light linen — the clothing reads the
  climate. *Lore:* "In the high forts a coat is not vanity but arithmetic — so many furs between you and the
  winter, and the ones who count wrong do not count a second one." *Gate:* NEEDS:DF-TEMP — a fur-cloak set.
- **[REQUEST] Frost / heat-haze VFX + a warming brazier** — a **frost/ice-crust** ground/edge VFX (cold) + a
  **heat-shimmer haze** (desert noon) + a standing **brazier/fire** for warmth (reuse the DF-ROOMS hearth glow).
  *Creative brief:* frost = a pale rime creeping in at the screen/terrain edges in deep cold; heat = a wavering
  shimmer over hot ground; the brazier = a warm glow-pool (ties Option-B lit_at). *Lore:* "You can read the
  temperature off the walls — white with rime in the cruel months, and the braziers lit in a chain down the
  halls, each a small argument against the dark and the cold at once." *Gate:* NEEDS:DF-TEMP + Option-B — the
  frost VFX + brazier.

## From DF-OMEN — "SIGNS & PORTENTS" batch (gap-fill)
*(The god's signs + the colony's superstition. REUSE the outcome.rs/weather bus + a DF-LIVESTOCK birth-variant.)*
- **[REQUEST] Portent VFX** — a **blood-moon** tint, a **comet/falling-star**, a **divine sign** (a shaft of
  wrong-colored light) — celestial omens on Veloren's weather/`outcome.rs` bus (no new engine). *Creative brief:*
  the blood moon = the sky/moon gone `accent.blood_red`; the comet = a slow bright streak; the divine sign =
  the GOD-HAND cast-VFX aimed at the sky (alignment-tinted). Reads as "the heavens are speaking." *Lore:* "The
  night the moon came up red the priests did not sleep, and the miller barred his door, and no one could say
  what it meant, only that it meant, and that meaning something is the most frightening thing a sky can do."
  *Gate:* NEEDS:DF-OMEN — the blood-moon + a divine-sign preset.
- **[REQUEST] Omen-birth creature** — a **two-headed calf / a white raven / a black lamb** (a DF-LIVESTOCK
  birth-defect variant — reuse the herd bodies, mark them wrong). *Creative brief:* a livestock body with a
  wrongness (two heads, an albino-white or pitch-black coat, too many limbs) — small, unsettling, a walking
  omen. *Lore:* "A white calf is born maybe once in a grandmother's life, and the whole valley comes to look,
  and half of them call it a blessing and half a curse, and both are right, which is exactly the problem with
  a sign." *Gate:* NEEDS:DF-OMEN + DF-LIVESTOCK — 1 omen-birth (reuse the herd variant pattern).

## From DF-ANCESTORS — "THE HONORED DEAD" batch (gap-fill)
*(Ancestors + ghosts. REUSE the DF-ROT burial batch + shipped undead + a translucent VFX.)*
- **[REQUEST] Ghost / apparition VFX** — a **restless-dead apparition** (a translucent, glow-edged figure at a
  cursed/unburied site, at night). *Creative brief:* reuse a shipped undead/humanoid body rendered **translucent
  + glow-edged** (a cold `glow.cave_teal` or a bruise-purple), faint, wavering — reads as "someone who was not
  put down right." Not gory — mournful, wrong. *Lore:* "It has her walk, the widower swears, exactly her walk,
  and it stands at the edge of the unlit field where they buried her too shallow the hard winter, and it is
  waiting, everyone agrees, though no one will say for what." *Gate:* NEEDS:DF-ANCESTORS + DF-NIGHT — 1 apparition.
- **[REQUEST] Ancestor shrine / effigy** — a household/hall **ancestor-shrine** (a niche, an effigy, offering
  bowls) where the honored dead are venerated (reuse the DF-ROT grave + DF-ROOMS statue + DF-RELIGION shrine
  pattern, culture-keyed). *Creative brief:* dwarven = a carved stone ancestor-niche in the wall; human = a
  hearth-side effigy with a bowl for offerings; worn smooth by touch. *Lore:* "Every hall keeps its dead close —
  a niche by the hearth with the grandfather's face cut into the stone, a bowl kept filled, so the living never
  quite eat alone." *Gate:* NEEDS:DF-ANCESTORS — 1 (culture-keyed) shrine.

## From DF-KNOWLEDGE — "LORE & LEARNING" batch (gap-fill)
*(The tech/knowledge arc made visible. A book item + library dressing; partly reuse DF-ROOMS lectern/bookshelf.)*
- **[REQUEST] Book / tome / scroll item + library dressing** — a **book/tome** item (the record that preserves
  knowledge against loss) + **library/study** dressing (bookshelves, a reading-stand, scroll-racks — extends the
  DF-ROOMS lectern/bookshelf). *Creative brief:* a heavy chained tome (dwarven, brass-cornered) vs a scroll-case
  (human); shelves of them = a library that reads "this colony remembers how." *Lore:* "The chained book in the
  deep library is worth more than the gold it's clasped with — it is the only thing in the fort that knows how
  the great forge was lit, now that the smith who lit it is a name on a grave." *Gate:* NEEDS:DF-KNOWLEDGE — the
  tome + a bookshelf cluster.

## From DF-CURSE — "THE MARK" note (gap-fill, reuse-heavy)
- **[NOTE — reuse] Curse mark + blighted ground** = the **DF-SYNDROME affliction** VFX + the **GOD-HAND evil
  aura/cast-VFX** applied to the cursed (a dark subdermal mark, a shadow) + a **blighted-ground** dressing (reuse
  SACRED-SITES cursed + DF-ROT decay — soured soil, dead grass). No new batch — a preset over shipped VFX.
  *Lore:* "You can see it on him if you know to look — not a wound, a wrongness, a shadow that falls the wrong
  way." *Gate:* NEEDS:DF-CURSE — a curse-mark preset (reuse syndrome + hand-evil-aura).

## From DIVINE-CHAMPION — "THE CHOSEN" note (gap-fill, reuse-heavy)
- **[NOTE — reuse] Champion regalia + divine mark** = the **GOD-HAND alignment aura** (good=radiant gold /
  evil=dread-red) applied to a colonist + a distinguishing **regalia/standard** (reuse the DF-VILLAIN named-enemy
  pattern, but RADIANT — a blessed banner, a mark of favor). No new rig — a decorated colonist. *Lore:* "You can
  tell the chosen one across a battlefield: the light lands on them a little too well, and their enemies notice
  it before their friends do." *Gate:* NEEDS:DIVINE-CHAMPION — a champion-aura + mark preset (reuse hand-aura).

## From DF-ART / SACRED-SITES / DF-RECLAIM / COLLECTIVE-RENOWN — reuse notes (gap-fill)
- **[NOTE — reuse] DF-ART monument** = a **great-statue/monument** (reuse the DF-ROOMS statue at structure scale +
  the DF-QUALITY masterwork treatment); the DEPICTION is procedural TEXT, not art. 1 monument exemplar. *Gate:*
  NEEDS:DF-ART.
- **[NOTE — reuse] SACRED-SITES** = a **shrine/waymark** (reuse DF-RELIGION shrine + DF-ROT grave) + a **sacred/
  cursed-site OVERLAY layer** (a plug-in on the UI-2 overlay framework — holy=warm / cursed=dark). No new .vox.
  *Gate:* NEEDS:SACRED-SITES.
- **[NOTE — reuse] DF-RECLAIM ruin dressing** = **weathered/overgrown versions of the colony's OWN structures**
  (reuse the build catalog + DF-ROT weathering + HAZARD-EVENTS rubble/scorch + flora overgrowth). No new batch —
  the ruin is the colony's own assets, aged. *Gate:* NEEDS:DF-RECLAIM (downstream B11/B12).
- **[NOTE — reuse] COLLECTIVE-RENOWN heraldry** = a **colony device/banner/heraldry** (reuse the DF-FESTIVAL
  banner + AGENT-CULTURE culture-heraldry — a colony's colours/sigil the world knows it by). The byname is text.
  *Gate:* NEEDS:COLLECTIVE-RENOWN — a colony-sigil/banner.

## ═══ DF-DERIVED ASSET DIRECTION (2026-07-10, from DF-BASTION-TRANSLATION.md) ═══
*(Ben directed the designer to direct DF-derived asset creation. HONEST FINDING: **most DF-derived objects are
ALREADY filed** — my ~20 asset batches ARE DF re-skins (workshops/tools/mechanisms/creatures/ore-veins/décor/
grave/mine all map to DF concepts, see DF-BASTION-TRANSLATION §1-9, mostly ✅). So this is NOT a big new flood on
top of your ~520-redo backlog — it's the 2 genuine GAPS below + a priority reconciliation. Rule: DF function kept,
Bastion name + lore per the lore-bible tone.)*

**PRIORITY RECONCILIATION (pilot):** the DF-derived assets slot into the existing top REQUIRED-ASSET QUEUE by
build-proximity — the redo-backlog + the P0/P1 (time-controls, tools[fixed], divine icons, HUD) come FIRST; these
DF gaps are P2 (production-era); the Tier-3 DF objects (beasts/artifacts) stay P3-speculative. **Do NOT interrupt
the 520-redo for these.**

- **[REQUEST — DF-gap] Brewhouse / still (the DrinkAlcohol venue)** — DF's Still → the Bastion **brewhouse**, the
  `Need::Drink` venue (ties DF-TAVERN/FOCUS). *Creative brief:* a workshop shell around a **fermenting-vat + a
  drip-still** — human = oak vats, copper coil, a cellar-cool damp; dwarven = a brass still + `glow.cyan` gauge.
  A colonist-reachable WORK POINT (clearance ≥3, door ≥2.2 — the workshop function-gate). Barrels stacked, a
  tasting-cup left out. *Lore:* "The brewhouse is the one building a colony keeps warm for reasons that aren't
  survival — the ale is kept drier than the seed and guarded better, because a winter with no drink is a winter
  the arguments don't stop." *Gate:* NEEDS:DF-TAVERN/PRODUCTION — 1 (human) spec placeholder; dwarven on demand.
- **[REQUEST — DF-gap] Maker's-bench (the general crafts workshop)** — DF's Craftsdwarf's-workshop → the Bastion
  **maker's-bench**, where the small crafts (the potted-herb, the carved bird, the toys, the trinkets) are made.
  *Creative brief:* a cluttered work-bench + a tool-rack + a stool worn smooth — half-finished small goods on it
  (a carving, a bowl); human rough-and-homely, dwarven precise-and-brass. The WORK POINT gate. *Lore:* "The
  maker's bench is where a colony makes the things it doesn't need — the carved bird, the child's toy, the second
  bowl — and a fort that has time for the bench is a fort that means to stay." *Gate:* NEEDS:DF-PRODUCTION — 1
  spec placeholder.
- **[NOTE — next wave] The FULL DF taxonomy** (all furniture/finished-goods/plant/building breadth) is not yet
  mapped — awaiting the raw df-structures / live-DF-oracle extraction (coordinating w/ the "Agent translator"
  session). As it lands, the designer re-skins it (DF-BASTION-TRANSLATION §10) → more asset rows here, priority-
  ordered. Until then, the corpus batches + these 2 gaps cover the DF-derived near-frontier.

## From TIME-OF-DAY INDICATOR — the sun/moon HUD clock (2026-07-10, Ben pull)
*(A WC3-style day-clock — pairs with the speed cluster as the time HUD. Small 2D HUD art in the UI-2 icon language.
Consumed by the time-control HUD build. P1 — pairs with the time-controls, the next build.)*
- **[REQUEST] Sun / moon / arc time-of-day indicator** — the always-visible day-clock. *Creative brief:* a small
  framed **arc (a horizon line)** with a **sun DISC** (warm gold, a simple radiant disc — rises east, peaks, sets
  west) and a **moon** (pale, a soft crescent-or-full — arcs through night); + **dawn/dusk TINT states** (a golden
  sunrise wash, a blue-violet dusk) at the transitions. Keep it iconographic + readable at HUD scale (not a
  detailed scene) — hook the warm `metal.brass_gold`/gold for the sun, a cool pale for the moon, and the HUD's
  existing frame language. Match the UI-2 HUD-icon style + the speed-cluster it sits beside. *(Optional variant: a
  BLOOD-MOON red tint for the DF-OMEN blood-moon — a recolor of the moon state.)* *Lore:* "The fort tells time the
  way the fields do — by where the light is. The overseer's little sun climbs its arc and the whole colony's day
  hangs under it: the miners down before it peaks, the gates barred before the moon takes its place. A god that
  watches the sun cross knows, without a word from anyone, exactly how much daylight its people have left to
  live in." *Gate:* NEEDS:TIME-CONTROLS HUD — the sun + moon + arc-frame + dawn/dusk tints as the core set;
  the blood-moon variant with DF-OMEN.

## From DF-ORACLE MILESTONE 1 — herd + crop breadth (2026-07-10, P2, variation-of existing)
*(The live DF oracle confirmed the colony-relevant fauna/flora. These are the GENUINE gaps beyond my herd + crop
batches — variation-of the shipped bodies/sprites, region/culture-keyed. P2 — production-era, do NOT interrupt the
520-redo + P0/P1. Bastion names+lore per the lore-bible tone.)*
- **[REQUEST — P2] Herd breadth: goat · poultry · draft** — beyond the cow/sheep/pig/fowl core. *Creative brief:*
  a **goat** (a hardy hill-browser, curled horns, ragged coat — the poor colony's cow); **poultry variety** (a
  **duck**, a **goose**, a **turkey** — barnyard fowl beyond the chicken, each a distinct silhouette); **draft
  animals** (a **mule / donkey** + an ox/**yak** — the haul-and-cart beasts, ties DF-TRADE's pack-animal + B6
  hauling — a loaded-pack variant). Variation-of the shipped quadruped/bird bodies (the fell-wolf recolor pattern).
  *Lore:* "The goat is the animal of a colony that can't afford a cow and won't admit it — eats the thorns the
  sheep won't, gives milk out of spite, and outlives every optimist who ever tried to fence it." *Gate:* NEEDS:
  DF-LIVESTOCK — the goat + a draft-mule first (the two highest-value: the poor-colony beast + the hauler).
- **[REQUEST — P2] Crop breadth: a grain · a root · a pulse** — beyond barley/carrot/flax. *Creative brief:*
  a **wheat** (the bread-grain — golden heads, distinct from barley's), a **root** (a **potato** or **turnip** —
  the winter-store staple, a leafy top + a hint of the buried tuber), a **bean/pulse** (climbing pods on a
  low frame — the lean-winter protein). Multi-stage sprites like the barley set (`Growth 0..max`), each stage
  distinct. *Lore:* "The bean is the crop that saves the arrogant — no one plants it in a fat year, and everyone
  who lived through a lean one plants it forever after." *Gate:* NEEDS:DF-FARM — the potato (the winter-store
  staple) + a wheat first.

## From DF-ORACLE MILESTONE 2 — item coverage (2026-07-10, P2) — 2 genuine gaps
*(The item_type breadth is otherwise COVERED by DF-ROOMS/PRODUCTION/BUILD/MECH/TOOLS/KNOWLEDGE batches + Veloren-
shipped weapons/armor/clothing. Only these 2 gaps. P2 — do NOT interrupt the redo + P0/P1.)*
- **[REQUEST — P2] Storage props (barrel · bin · bucket · crate · cage)** — the stockpile-fill (DF's BARREL/BIN/
  BUCKET/CAGE; ties B6-haul, where hauled goods pool). *Creative brief:* a **stave barrel** (banded oak, a little
  bulge), a **woven bin/basket**, a **bucket** (a handle, a stave body), a **crate**, a **slat cage** (for a caught
  beast/fowl) — humble, well-used, stackable; human = oak-and-iron-band, dwarven = brass-hooped + a `glow.cyan`
  tag. Vary the wear (a cracked stave, a patched bin). *Lore:* "A colony's whole surplus lives in its barrels, and
  a cooper who can't keep a band tight is a cooper who costs the fort a winter's ale — so the good ones are kept
  fed and flattered like the smith, and the bad ones learn to fish." *Gate:* NEEDS:B6/stockpile — the barrel +
  bin first (the two most-seen).
- **[REQUEST — P2] Engraved memorial slab / gravestone** — a carved stone slab that DEPICTS/records (DF's SLAB;
  ties DF-ART depiction + SACRED-SITES/DF-ANCESTORS — the marker of the honored dead + a chronicle-made-physical).
  *Creative brief:* an upright or set **stone slab** with a carved face — a name, a simple relief (a pick, a sheaf,
  a wolf), worn by weather/touch; human = a rough fieldstone stele, dwarven = a dressed dark-stone plaque with
  `metal.brass_gold` lettering + a `glow.cyan` line. The carving reads "someone is remembered here." *Lore:* "The
  slab is the cheapest immortality a colony offers — a name and a picture of the one thing you did that the carver
  thought worth the chisel. Most get a tool. The lucky get a deed. The great get a face, and the children learn
  the fort's whole history walking the wall of them." *Gate:* NEEDS:DF-ART/SACRED-SITES — 1 (human) spec.

## BUILD PALETTE + GOD-PLACE VFX (from BUILD-MODE-design.md, 2026-07-10) — near-frontier
**Context:** the three build modes (base-game/avatar/god) + the shipped designate path all share ONE block palette.
Two asset needs — a palette-icon set + a god-place placement flourish.

### 1. BUILD-PALETTE ICONS (2D HUD, UI-2 icon language)
- **Silhouette/read:** small square swatch icons for the block/component palette — one per placeable, grouped in
  three categories: **TERRAIN** (rock · earth · grass · wood · sand) · **STRUCTURE** (wall-segment · floor · pillar ·
  roof — reference the REDONE Bastion structure-assets; no new geometry, the palette just points at them) · **SPRITE**
  (ladder already ships; furniture later). Recognizable-at-16px, category-tinted frames.
- **Materials/ramps:** the ASSET_STYLE_GUIDE §5 named ramps per material (stone-grey / earth-umber / leaf-green /
  timber-tan). Frame tint = category (terrain/structure/sprite) so the three groups read at a glance.
- **Variation axis:** by material within a category (NOT silhouette — the palette shows the material, per the tool-
  tier lesson: material/colour tiers, same recognizable shape).
- **Lore-seed:** *"The god's own hand knows every stone the world is made of, and sets them down as a mason sets a
  course — grey granite for the deep walls, umber earth for the berm, greenwood for the rafters."*

### 2. GOD-PLACE VFX (later; ties the god-hand)
- A brief "block set by the hand" placement flourish — a downward divine-light tap + a settling shimmer as the block
  appears, distinct from a COLONIST's build (which throws dust + a tool-swing). The player must read *divine* placement
  vs *labor* placement at a glance. Ties the GOD-HAND cursor + the raise-land god-power VFX family.
- **Lore-seed:** *"Where a mason leaves sweat and chippings, the god leaves only a moment's brightness and a stone
  that was not there before."*

## THE THREE FACES — face indicator + face-shift VFX + watcher-sight (from PLAYER-MODES-design.md, 2026-07-10)
**Context:** the 3 player modes = three FACES OF THE GOD (SOVEREIGN/god · WATCHER/spectator · INCARNATE/avatar). The
player shifts which face they wear; each needs to READ at a glance.

### 1. FACE INDICATOR + POWER-BAR GLYPHS (2D HUD, UI-2 icon language)
- **Silhouette/read:** three distinct face glyphs — **SOVEREIGN = a crown** (rule-from-above) · **WATCHER = an open
  eye** (all-seeing) · **INCARNATE = a walking figure** (god-made-flesh). Recognizable at 16px, one worn-face
  highlighted + its per-face power-bar icons.
- **Ramps/tint:** ASSET_STYLE_GUIDE §5 — a divine-gold accent shared across all three (one god); each face a distinct
  secondary tint (sovereign = throne-purple/gold · watcher = pale seeing-blue · incarnate = warm flesh/earth).
- **Variation axis:** the three faces (fixed set, not procedural).
- **Lore-seed:** *"The god has three faces, and a wise colony learns which is turned toward it — the crown that rules,
  the eye that watches, the hand that walks among them."*

### 2. FACE-SHIFT VFX (the transition between faces)
- Three transitions: perspective **descending into the eye** (→ watcher) · **into a body** (→ incarnate) · **rising
  to the throne** (→ sovereign). A brief divine-light morph, tied to the god-hand + god-place VFX family (one visual
  language for "the god's perspective moving").
- **Lore-seed:** *"When the god changes its face the light changes with it — sinking to a watchful glimmer, warming
  to a walking flame, or rising cold and bright back to the high seat."*

### 3. WATCHER-SIGHT OVERLAYS (later — the omniscience powers' visuals)
- **★ Scry-the-Memory** (the signature): ghostly **chronicle-echoes** of past deeds layered on a place (faint spectral
  vignettes of who died / what was forged / the oath sworn there) — the remembering-world made visible. Ties DF-HIST +
  SACRED-SITES + the chronicle-reader.
- **Deep-inspect readout** panel (the full soul: facets/values/needs/mood/skills/relations/life-story) — extends the
  OBJECT-INSPECTION UI.
- **Farsight** fog-lift shimmer (the unseen revealed).
- **Lore-seed:** *"To the watching eye the ground is never silent — every stone keeps the faint bright ghost of what
  was done upon it, and the god has only to look to remember."*

## MINE STRUCTURES ×3 (SILHOUETTE-FIRST — no Veloren native reference) — 2026-07-10 [#1 player-visible, Ben in the mining loop]
**Why silhouette-first:** none of these exist in Veloren (no headframe/pithead/breach in canon), so the pilot has no
native reference — they must read from silhouette + identity alone or they go generic. Each brief gives FORM
(silhouette) · MATERIAL · WHAT-MAKES-IT-OURS · tone/lore-seed. Human culture. Veloren-canon-safe = hand-hewn timber +
iron, animal/hand-powered (NO steam/anachronism). Grim-tender folk-mythic. Miners are superstitious — that's the
identity hook (votive charms against the deep-dark). See ASSET_STYLE_GUIDE §5 ramps (timber-tan, iron-rust, deep-stone).

### mine_headframe_human — the winding-tower over a deep shaft
- **FORM (silhouette):** four splayed squared-timber legs converging to a head platform; a big **sheave/winding wheel**
  at the apex catching the hoist-rope; an **ore-skip** (bucket) hung in the shaft; a lean-to **winding house** (the
  windlass/whim — hand- or beast-cranked) at the base. TALL, purposeful, asymmetric — reads "things are hauled up from
  below here" at a glance. The wheel is the identifying beat.
- **MATERIAL:** heavy pegged timber (weathered grey-brown, timber-tan ramp darkened), iron strap-banding at the joints,
  tarred-hemp rope, an iron-rimmed sheave wheel (rust-streaked). Hand-hewn, slightly crooked (a colony built it, not a
  mill).
- **WHAT MAKES IT OURS:** a WORKING VOTIVE object — a **watching guardian-face carved into the tallest leg**, charms +
  a lantern hung on the legs, a **tally-ledger scratched into the timber** (who went down / who came up). The grim-
  tender read: the machine that lowers people into the dark and *mostly* brings them back.
- **LORE-SEED:** *"The headframe stands over every deep shaft like a gallows that mostly gives its hanged men back —
  the miners carve a watching face into the tallest leg, and none pass beneath the wheel without touching it for luck."*

### mine_pithead_human — the shaft mouth at grade (the threshold)
- **FORM (silhouette):** a squared-timber **collar/frame** around a black shaft opening; a short windlass or ladder-
  head; a **sliding plank cover**; a sorted **spoil-heap of tailings** beside it. LOW, horizontal — the deliberate
  contrast to the tall headframe. Reads "a framed, worked hole into the ground."
- **MATERIAL:** squared timber collar (same timber language, worn smooth where ropes/hands pass), plank cover, grey
  rubble tailings heaped neat, an iron grate or rope-rail.
- **WHAT MAKES IT OURS:** a **votive niche at the lip** (a little carved recess holding an offering — a coin, a crust)
  + a lantern-hook; the collar-timbers polished by a thousand passings. The read: the threshold — the last daylight
  before the deep.
- **LORE-SEED:** *"At the pithead the colony leaves its small bribes to the dark — a coin in the niche, a crust on the
  collar — for the deep keeps what it is not paid for, and every miner knows it."*

### mine_breach_maw — the raw torn breach into the deep (ominous/organic; ties DF-DEEP-DARK)
- **FORM (silhouette):** NOT built — **broken into.** A jagged, irregular rent in rock/hillside; **hasty, half-
  collapsed timber shoring** (failed, not neat); darkness pouring out. The silhouette must read WRONG / unplanned — a
  wound, not a doorway. The anti-pithead.
- **MATERIAL:** torn raw dark stone (deep-stone ramp), splintered/failing shoring timbers, a faint breath of dark
  mist/miasma at the lip, creeping cave-fungus at the edges (the deep reaching up).
- **WHAT MAKES IT OURS:** it is the exact OPPOSITE of the tended, votive pithead — it's the thing the offerings were
  *against*. Uncanny, organic, breached. The read: something was opened that shouldn't have been; the dark has a mouth
  now.
- **LORE-SEED:** *"A breach is not dug, it is suffered — the moment the picks go through into a blackness that was
  already there, older than the colony, and the miners stop singing. What comes up through a maw is never ore."*

## GOD-DOMAIN sphere-glyphs + domain readout (from GOD-DOMAIN-design.md, 2026-07-10) [flagship]
**Context:** the god's DOMAIN (its sphere) colours all three faces. Six sphere-emblems + a domain readout chart.

### DOMAIN SPHERE-GLYPHS ×6 (2D HUD emblem icons, UI-2 language)
- **Silhouette/read (each recognizable at 16px, in its domain tint):** **DEEP** — a downward mountain / an ore-vein
  cleft · **HARVEST** — a wheat-sheaf / a sprout · **DEAD** — a grave-mound or a skull *in repose* (tender, votive —
  NOT gory; the Mourner's mark) · **STORM** — a bolt from a cloud · **FORGE** — an anvil crowned with a flame ·
  **HEARTH** — a hearth-fire / two clasped hands.
- **Ramps/tint (ASSET_STYLE_GUIDE §5):** each in a distinct domain colour (deep=stone-grey/lantern-amber · harvest=
  green-gold · dead=pale bone/violet-grey · storm=slate-blue/white · forge=ember-orange · hearth=warm hearth-red).
  All share the divine-gold accent (one god, six spheres).
- **Variation axis:** the six spheres (fixed set).
- **Lore-seed:** *"Every god is the god of something, and the something shows — in the cast of its light, the shape of
  its blessings, the name the people give it. Six marks for six inclinations, and most gods bear more than one."*

### DOMAIN READOUT (→ pilot, HUD, later)
- A small **six-petal / radial chart** of the affinity vector (the dominant sphere lit) + a **drift-from-invocation**
  indicator (how far the god has become something other than what it swore at founding). Composes with the face-
  indicator (PLAYER-MODES) into one "who is my god right now" panel.
- **Lore-seed:** *"The petals show what the god has been doing, whatever it once claimed to be — and a wise colony
  watches them turn."*

## TOOLBAR-ICONS 12th icon gap — `tool_bed.png` (builder finding, EXHAUSTIVENESS-ASSERTS+ICONS tags, 2026-07-14)
**Context:** the overseer palette's 11 icons (`tool_{pan,inspect,mine,chop,gather,farm,build,stockpile,ladder,erase,
god}.png`) were made before EXHAUSTIVENESS-ASSERTS added `DesignationKind::Bed` to the paintable set (row 51.52).
TOOLBAR-ICONS (row 51.5) wired all 11 existing icons in but Bed currently renders a transparent image + text-label
fallback, not a real icon.
- **Ask:** a 12th palette icon, `tool_bed.png`, matching the existing set's style/size (34×34, same silhouette
  language as the other 11 designation tools) — a bed/bedroll glyph, distinct enough not to collide with the
  existing gather↔farm look-alike issue already logged for this set.
- **Priority:** LOW — queue behind the asset lane's eventual resume and the already-logged readability re-pass on
  the other 11 icons (mine/chop/pan misreads, gather↔farm collision); not blocking anything, Bed's text-label
  fallback works fine in the meantime.
