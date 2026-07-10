# Project Bastion — ASSET_REQUESTS (the request board)

The **interface from the design passes to the asset pilot.** A design pass writes an entry here for any asset
a **near-term** block will consume (per `GENERAL-DESIGNER-prompt.md` step 4 + the §3i delegation model). The
pilot (`MASTER-asset-tooling-prompt.md`) draws generation batches from here. Each entry: what · why (which
sub-block consumes it) · READY-now vs gated · style/function notes.

**Rule:** only near-term, real-demand items go here. Speculative breadth stays on `BASTION-CONTENT-WISHLIST.md`
until its `[SYS:]` gate lands. A `NEEDS:<system>` entry may be requested as a **deliberate placeholder to
pressure-test/spec** its system (generate one, not forty).

---

## From DF-PRODUCTION (`DF-PRODUCTION-design.md`, session 2026-07-09)

The production cluster sits just past **B6** on the frontier; PROD-2 (farm) can partly precede B6. These are
its near-term consumers:

- **[REQUEST] Workshop building shells** — smithy / kitchen / loom-house / carpenter — the zone-scale
  structures the existing station sprites sit inside. *Consumed by:* PROD-0 (DF-WORKSHOP) as production-zone
  assets. *Function:* each needs a colonist-reachable WORK POINT at the station (function-harness gate,
  clearance ≥3 blocks, door ≥2.2). *Style:* per-race set where sensible; barn/witch-hut prefab pattern. *Gate:*
  NEEDS:DF-WORKSHOP — request **1–2 as pressure-test/spec placeholders** now (motivate the zone-structure
  spec), full batch on PROD-0 landing.
- **[REQUEST] Crop growth-stage sprites** — a wheat-style multi-stage sprite set keyed to the existing
  `Growth(0..max)` block attribute, for 2–3 starter crops (grain, a vegetable, a fibre crop). *Consumed by:*
  PROD-2 (DF-FARM). *Function:* stage count must match the crop's max `Growth`; each stage visually distinct
  (the sprite IS the farm's legibility). *Gate:* NEEDS:DF-FARM — **1 crop as spec placeholder** now, batch on
  PROD-2.
- **[REQUEST] Seed item icons** — one per farmable crop (small icon set). *Consumed by:* PROD-2 (new seed item
  defs — none exist today). *Gate:* NEEDS:DF-FARM.
- **[REQUEST] Farm dressing** — tilled-soil ground texture, low fence / trellis props. *Consumed by:* PROD-2
  (makes a farm zone *read* as a farm). *Gate:* NEEDS:DF-FARM.
- **[READY] Crafted-good + prepared-meal breadth** — the recipes already exist (326); models/icons for the
  goods colonists will actually make (tools, planks, cloth, bread, ale). *Consumed by:* PROD-0/PROD-3.
  *Generate demand-ordered* (what the starter economy produces first), NOT all at once (§3i warning).

## From DF-HIST (`DF-HIST-design.md`, session 2026-07-09)

DF-HIST (the Chronicle/Legends) is a **UI-only** system — no 3D, no sprites-in-world, no animation. Its single
near-term, real-demand asset is a small 2D icon batch for the live event feed (HIST-2, the DF-LOG slice).
Everything else is UI-authored-in-code or far-future polish (stays on the wishlist).

- **[REQUEST] Chronicle event-type glyphs** — ~10–15 small **monochrome UI glyphs**, one per `ChronicleEvent`
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

- **[REQUEST] Shrine + temple structures** — a **shrine** (small, one altar) and a **temple** (`faith`-purpose
  zone: altar + congregation hall). *Consumed by:* REL-0 (DF-RELIGION). *Function:* each must ship a colonist-
  reachable WORSHIP POINT (the altar-facing congregation spot; clearance ≥3, door ≥2.2 — same function gate as
  workshop shells). *Style:* per-race where sensible; barn/witch-hut prefab pattern; **author with a lore
  field** (temple lore biases faith — future-work §lore). *Gate:* NEEDS:DF-RELIGION — request **1–2 as
  pressure-test/spec placeholders** now (motivate the `faith`-zone structure spec), full batch on REL-0.
- **[REQUEST] Altar / idol / effigy prop** — the worship focal point the congregation faces (the tavern
  `Detail::Bar`/`Stage` analog). *Consumed by:* REL-0. *Function:* placeable focal marker at the worship point.
  *Gate:* NEEDS:DF-RELIGION — **1 as spec placeholder** now, batch on REL-0.
- **[REQUEST] Pew / prayer-mat / kneeler** — congregation-spot dressing (the tavern chairs analog; makes a
  temple *read* as a place of worship). *Consumed by:* REL-1. *Gate:* NEEDS:DF-RELIGION.
