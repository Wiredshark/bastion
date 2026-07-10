# BASTION ASSET PRODUCTION ROADMAP — the batch plan (consumption-ordered, core-first)

**Status:** the pipeline is proven end-to-end (ladder L1–L10, castle/monastery/Godspire ceiling tests
passed with honest limits). The bottleneck is no longer capability — it's **asset density**. This doc is the
standing production plan the asset session works from. It supersedes ad-hoc "generate down the taxonomy"
briefs.

## THE ORDERING PRINCIPLE — core-to-gameplay first, then consumption order
Do NOT generate down the taxonomy in taxonomy order (that produces density, but random density). Generate in
the order the game will actually USE things:
1. **CORE GAMEPLAY assets first** — anything a currently-queued or near-term block will place, spawn, or
   require to function. If a block in the build queue needs it, it's core. These make the pipeline a supply
   chain feeding an active construction site, not a portfolio.
2. **Then density** — the breadth that makes the world feel full (wildlife/flora variation), zero-gated and
   animation-inherited.
3. **Then pre-positioning** — assets for blocks that are designed but further out (B7/B8-era).
Within every batch: standing rules as gates (circulation sweep, no-flat-walls, computed layouts, both
harnesses, lore, TEST/REAL + READY/NEEDS tagging), **variation-strategy preferred**, **material library
throughout**, **no per-asset review stops** (batch autonomously; Ben reviews the batch), backfill studio
metadata opportunistically.

## PRECONDITION WORK (before batching — do first)
1. **Close the Godspire's two logged FAILs** (hall-entry carve wiring + reliquary-open matrix
   inconsistency). Keeps the "nothing ships with a known FAIL" discipline intact.
2. **Build carve/void declarations into the composer** as first-class layout objects checked at compose
   time (from the temple report — would have prevented the hall bug and halved castle/monastery iteration
   counts). Every structure batch after this is cheaper, so do it BEFORE the big batches. This is the ONE
   sanctioned new-tooling investment; otherwise no new frameworks — density.

## BATCH A — CORE: what the queued blocks consume next (HIGHEST PRIORITY)
Aimed directly at the build queue. Each item names its consuming block:
- **Ladder** (wood + rope variants) — **B5.8 explicitly needs one** ("check for existing ladder sprite,
  else generate"). Climbable vertical link. TOP of the batch — a queued block is waiting on it.
- **Stockpile containers** — crates, barrels, sacks, bins, stockpile marker posts — **B6 (stockpiles/
  hauling) is next in the main chain**; these become visible in-game immediately. Piles→containers is the
  stockpile visual.
- **Gather-props** — loose gatherable resource props (the drops/piles B6's Gather designation collects) —
  ensure the common resources (stone, wood, ore) have clean pile/drop visuals at each tier.
- **Boundary markers** — stones, banners, totems, waystones per race — **§3w colony boundary** (field +
  markers) is B6-era; the diegetic border needs these.
- **Basic tools** if gaps — pick/axe/hammer/hoe held-item renders (B6 individual-carry + §3u work anims
  show held tools).

## BATCH B — DENSITY: the world feels full (zero-gate, animation-inherited)
- **Biome wildlife variation packs** — per biome (tundra/desert/jungle/coast/river), variation-first off
  existing families; juvenile variants (fawn/calf/cub/chick) for the §3y lifecycle. All inherit animation.
- **Flora variation packs** — per-biome tree/shrub variants (off the 53 existing trees), growth-stage
  variants (sapling→ancient→snag) for §3y, cave flora.
- **Clothing/civilian appearance** breadth per race (the civilian-clothes gap vs armor overlays).
This is pure breadth — cheapest assets, biggest "the world is alive" payoff.

## BATCH C — PRE-POSITION: designed-but-further-out blocks (B7/B8-era)
- **Workshop interiors with WORK POINTS** — carpenter/mason/smelter/tannery/loom/etc., each with a
  function-harness-reachable work station (B7 idle/work life + the production chains).
- **Defense kit** — wall sets per race (→stone/brick/dwarven), towers, [OP] gates/drawbridges, traps
  (pit/spike/tripwire — B8 + the trap engine).
- **One themed dungeon set** — terracotta or gnarling theme, registry-component rooms/corridors/junctions +
  a boss-arena centerpiece (§3v delving content).

## ONGOING (opportunistic, between batches)
- Studio metadata backfill (`missing — backfill` fields), light material-library retro-passes on older
  assets, creature BODIES continuing (cheap; the Rust figure-layer port stays parked with the game agent).

## THE GRADUATION PATH (why B-ASSET1 matters to this plan)
Everything generated is **READY-pending-dynamic** — static-verified but not engine-tested. When B-ASSET1
(the integration harness + arena, game-agent queue) lands, batched assets graduate STATIC → arena-tested →
testbed-validated, and the studio's TEST→REAL promotions get teeth. Batch freely now; the graduation catches
up. Coordinate ONLY via `readme/` logs (ASSET_INTEGRATION_LOG.md), never across agent code.

*Source of truth for what-needs-what: `readme/BASTION-CONTENT-WISHLIST.md` (the full tagged catalog) +
the build queue in the mega-prompt (what's consuming next). This roadmap = the ordering over that catalog.*
