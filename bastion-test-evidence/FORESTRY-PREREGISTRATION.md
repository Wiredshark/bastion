# FORESTRY SELF-GENERATOR — pre-registration (2026-08-21)

**Registered BEFORE any code is written.** This is the second half of the
roadmap's own PAR-STOCK PULL ECONOMY charter (Ben: *"why would colonists ever
mine or chop wood — we never really considered that in full"*), and it exists
because closing F13 built the first half by accident and then named what was
missing.

## The claim

Today the colony can feed its own STONE economy: a designated job that bills
`BUILD_MATERIAL_ITEM` counts as demand, and the self-generating mine wakes and
digs until that demand is met. **Wood has no such loop.** A ladder bills
`CHOP_DROP_ITEM` against a forestry economy that nothing wakes, so a colony
that needs timber waits forever exactly as it used to wait for stone.

This row builds the wood analogue.

## What the mine's four blockers already taught, applied up front

F13 took five legs because four sufficient blockers each hid the next behind
the identical symptom. Every one of them has a wood twin, and each is
addressed in the design rather than discovered:

| Mine blocker | Wood twin | Pre-emptive answer |
|---|---|---|
| demand measured over PLANS only | demand measured over nothing | `job_bills_wood_unsupplied`, mirroring the stone predicate |
| radius too small to see the resource | trees outside the search box | reuse the mine's 24, and emit the search bounds in the witness |
| CLAIMED jobs excluded from demand, so progress zeroed demand | identical | predicate is unclaimed **OR** `needs_materials` from the start |
| generator's test easier than the claim path's | chop cells with no standable stance | `place_chop_fell` already re-validates cells; the witness must show kept-vs-rejected |

## Where it must live, and why that is not a detail

`detect_trees` needs `world` and `index`. The bastion jobs system is
deliberately terrain-only (`World stays out of the terrain-only bastion_jobs
system`), so the generator goes in `server/src/lib.rs` beside the raid and
trade ticks, and reads demand off the board. Putting it in the jobs system
would mean either widening that system's data or hand-rolling a tree probe —
and a hand-rolled probe is exactly the drift that produced F13's fourth
blocker.

## PREDICTION (the leg is scored against this, not against a story)

Arm: a colony whose board holds at least one job billing `CHOP_DROP_ITEM`.

1. `FORESTRY` chop designations appear that **no player painted**.
2. Wood (`CHOP_DROP_ITEM`) enters the colony's supply — a count that was 0.
3. The generator goes QUIET once demand is met: it must not keep felling.
   Quiescence is a real bar, not a nicety — the mine's `deficit` arithmetic is
   what stops it, and a forestry loop without one clear-cuts the map.

## FALSIFIERS, with their own witness

- If **no chop designation appears**, the witness must say which of the two
  causes applies: `trees_seen=0` (no forest in range — a worldgen fact) versus
  `trees_seen>0` with none placed (a refusal — a bug). F13 cost three legs to
  a null that could not distinguish these; the witness ships WITH the feature
  this time rather than after it.
- If **wood never reaches supply**, felling works and hauling does not — a
  different row, and the witness must not let me blame the generator for it.
- If the generator **never goes quiet**, the deficit arithmetic is wrong and
  the row FAILS even with wood flowing.

## Explicitly NOT in scope

The **standing par-stock floor** (keep ~N wood with no job asking) is the
charter's remaining third. This row is demand-PULL only: something must want
wood. Saying so now stops a later reading of a green leg as proof of a floor
that was never built.
