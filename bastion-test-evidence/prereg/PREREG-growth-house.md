# PREREG — the town lays out a house with worldgen and its colonists build it (G1)

Registered 2026-09-02 14:20, before any code exists. Ben's rulings, live
today: new houses, farms and mines must appear as properly sized plots
as the town grows (13:45), laid out with the same worldgen that made the
town and built autonomously by colonists; player zoning, roads and
single-building placement come later (13:58).

## What stands (the two mapping passes, file:line in worldgen-plot-machinery-map)

The town's only growth act today is `housing_build_verdict` -> a ring
scan -> ONE `DesignationKind::Bed` cell -> a bedroll sprite on open
ground; no wall or roof is ever placed, and the verdict never fires on
the adopted flat town (58 houses for 49-57 people: "a house already
stands empty"). Farms are never laid out after founding; mines are
per-cell jobs. Worldgen can place a plot on an EXISTING Site
(`find_roadside_aabr` -> `House::generate` -> `create_plot` ->
`blit_aabr`) and can emit one plot's blocks without a chunk
(`CanvasInfo::with_mock_canvas_info` + `render_collect` + `sample_at`).
The server keeps the Site for its life and never mutates it.

## Mechanism (three parts, each its own commit)

G1a WORLD: `layout_plot_for_colony(site, kind, land, index, sim, rng)`
returns the plot id, its world aabr, its door, its blocks (pos -> Block)
and its bed positions; deterministic from a seed the colony supplies
(site id + a plot counter). Unit-tested on a generated site.

G1b BOARD: a build plan that carries the exact Block per cell (today's
plan places grey Rock in every empty cell of a Region), minted as Build
jobs; on completion the colonist places THAT block; the plan's last
block registers the plot: beds -> `DesignationKind::Bed` region
(households), a field -> `DesignationKind::Farm`. Witness "PLOT LAID
OUT — kind, tiles, door, blocks" at layout and "PLOT BUILT — kind,
blocks placed, beds" at completion; "BUILD BLOCK" per block is too many
lines, so a daily "BUILD PROGRESS — plot, placed/total" census instead.

G1c TRIGGER: the existing housing verdict ("the town needs another
roof") calls G1a+G1b instead of the bedroll ring scan; the verdict's
material condition (20 stone) becomes the plan's material draw.
`BASTION_NO_WORLDGEN_PLOTS` restores the bedroll.

Prior art: Banished (a house is a footprint the builders fill from a
material pile), Manor Lords (plots laid on roads, families move in),
Dwarf Fortress (designations built block by block by whoever is free).

## Pre-registered outcomes

- Instrument validation first: on a world where houses < roster (the
  founding preset without adoption, or the flat arm with a forced
  verdict switch BASTION_FORCE_HOUSE=1), the day's HOUSING BUILD line
  reads "the town needs another roof" and a PLOT LAID OUT line follows
  the same day with tiles >= 4 (a 2x2-tile house) and blocks >= 200.
- PASS: BUILD PROGRESS reaches >= 80% of the plot's blocks within three
  game days with >= 2 distinct builders; PLOT BUILT registers >= 1 bed;
  the next HOUSING GROWTH line counts one more house; a client look (Ben
  or a screenshot) shows a house with walls and a roof where open ground
  was.
- FAIL branches: PLOT LAID OUT fires but no block is ever placed -> the
  build plan's jobs are unreachable or unclaimed (the walker rows and the
  Build lane); blocks placed out of order leave a shell without a door
  -> the plan needs an ordering (floor, walls, roof); the layout lands on
  a road or a field -> `find_roadside_aabr`'s tile test disagrees with
  the colony's designations and the two must be reconciled; blocks
  placed but the house does not register beds -> the bed sprites of the
  plot were not in the block list (the mock canvas's colour defaults are
  not the issue; sprites are) or the registration reads the wrong
  cells.
- NOT in this row: farms and workshops (G2, G3: the same three parts
  with `find_rural_aabr` and `generate_farm`), player zoning, roads.
