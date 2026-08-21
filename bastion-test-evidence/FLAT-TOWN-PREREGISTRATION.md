# PRE-REGISTRATION — a REAL Veloren village on FLAT ground

**Ben, 2026-08-21:** *"i think we should place the town on our flat arena"* and
*"remember veloren actual town support pretty much all these systems."*

Both points are right, and together they rule out the obvious implementation.

## Why not "draw a town into the arena"

`bastion_flat_arena::override_chunk` builds a `TerrainChunk` from scratch and
**returns before `world.generate_chunk` is ever called** — and `generate_chunk`
is what renders site structures. A village inside the arena radius is therefore
not drawn at all. Hand-building houses into the slab would also throw away
exactly what Ben says we should keep: Veloren's villages already have doors,
beds, roads, farm fields and workshops.

## Why not "flatten under an existing village"

`World::generate` runs `WorldSim::generate` **first**, then
`civ::Civs::generate(seed, &mut sim, …)`. Site altitudes are derived during
civ placement. Flattening *after* placement moves the ground out from under
buildings whose heights are already baked — floating or buried houses.

## The design: flatten BEFORE the civs are placed

Flatten `SimChunk.alt` (and `basement`, and `water_alt` below it) across a
radius at world centre **between `WorldSim::generate` and `Civs::generate`**.
Villages that land there are then placed and rendered by the **real generator**,
on genuinely level ground, with every system Veloren already supports intact.

Gated behind `BASTION_FLAT_WORLD_RADIUS=<chunks>`, off by default. This is a
**worldgen** change and it moves every seed's terrain when enabled, so it must
never be on for a run that compares against a banked baseline.

## The prediction

**PASS requires all four:**

1. **A village exists inside the flattened radius** and `bastion_adoptable_town_plots`
   finds it — houses ≥ 3, and ideally `farm_fields ≥ 1` (the arena arm has
   never had a real field; item F5's live witness is still owed on that).
2. **The village renders.** Houses have walls, roofs and doors; roads exist.
   Verified by looking, not by a plot count — a count of 4 houses proves
   placement, not rendering.
3. **The ground under it is level.** Sampled altitudes across the village
   footprint vary by ≤ 1 block.
4. **Colonists path on it without magic.** `failsafe_teleports` = 0 across a
   leg of the same length that previously produced them.

**FAIL / VOID branches, named now:**

| Observation | Means |
|---|---|
| No village inside the radius | VOID, not fail. Civ placement is seed-driven and may simply put nothing there. Retry on other seeds before concluding anything; report how many seeds were tried. |
| Village placed but houses are broken/half-rendered | The flatten fought the site's own terrain assumptions (a plot expecting a slope). **This is the outcome that would make the whole approach wrong**, and it is why criterion 2 is "looked at" rather than counted. |
| Village fine, `failsafe_teleports` still > 0 | **Terrain was not the cause of the pathing failures.** Flat ground is then cosmetic for this goal, and the climbing/falling is building-driven (roofs, upper storeys, doorways) — a different row entirely. |
| Water sitting on the flattened plain | `water_alt` was not lowered with `alt`. Fixable, but it will look like a flooded town. |

## What this does NOT test

- Whether colonists **occupy** the houses (that is adoption + bed assignment).
- Whether stations land in sensible rooms.
- Anything about building-driven climbing — see the third failure branch above,
  which is the one that would redirect the whole effort.

## The honest risk

**Flat terrain may not fix the pathing.** The measured failures — 30 fail-safe
teleports, colonists stranded in one cell on *dead-flat arena ground*, 261
unreachable-job releases — happened on ground that was **already flat**. That is
evidence against terrain being the cause, and it is recorded here *before* the
run so a disappointing result cannot be reinterpreted afterwards.

The value of this row is not primarily "fix pathing." It is: **a real Veloren
village, with all its systems, in a controlled environment we can watch** —
which is what makes every later look-and-feel judgement mean something.
