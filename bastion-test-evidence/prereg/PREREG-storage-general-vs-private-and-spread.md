# PREREG — general stores vs private property; spread storage
(Ben, live 2026-09-01 22:15; written 22:20, before any of it is built)

## What Ben saw, and the producers (read, not guessed)

"Colonists stockpile in their houses on one little shelf." Producers:
- At adoption every chest/shelf inside a house is registered as a ONE-CELL
  stockpile zone (bastion_jobs.rs, ADOPT-IN-PLACE container: `Region { min:
  pos, max: pos }` pushed to `board.stockpiles`, `chests += 1`).
- A DepositRun picks its destination as the NEAREST stockpile by region
  centre (`min_by_key` on centre distance) -- from a house, that is the
  house's own shelf.
- `stockpile_drop_cell` always returns the region's CENTRE cell, so every
  drop in a zone lands on the same cell ("stacks on top of each other").

## Mechanisms to build (order)

S0 INSTRUMENT: STORAGE CENSUS, daily: per stockpile zone: kind, cells,
   units on the ground inside, max units on one cell, `private` (the zone
   lies inside a Bed region) or `general`; totals per kind. Baseline first.
S1 KINDS: `board.store_kind: ZoneId -> StoreKind { General, Private(house
   region) }`; adopted containers inside a Bed region are Private; painted
   Stockpile zones and adopted barns are General. Witness: the census.
S2 DESTINATIONS: a DepositRun / haul goes to the nearest GENERAL store; a
   Private store receives only from its own household (the colonist whose
   bed lies in that house) and is drawn on only by that household. Jobs of
   the town never fetch from a Private store. Fallback = identity when no
   General store exists (a town with only shelves keeps today's behaviour,
   and says so in the log).
S3 SPREAD: `stockpile_drop_cell` takes an occupancy closure and returns the
   least-filled standable cell of the region (row-major on ties, the centre
   first when all are empty); a one-cell store has nothing to spread (S2
   keeps town hauls out of it anyway). Pin: a 3x3 zone with the centre
   holding 5 units drops elsewhere; all-empty drops at the centre; identity
   for a 1x1 region.
S4 SEEING IT: the zone radial title says "General store" / "Private:
   <house>"; the inspector's colony section lists general stock.

## Prior art

Manor Lords (families keep supplies at home; granary/storehouse workers
serve the town), Song of Syx (warehouses vs homes), RimWorld (stockpile
zones with per-cell stacks, priorities), Dwarf Fortress (stockpiles per
category; one item per tile), Banished (storage barns vs houses).

## Pre-registered pass / fail (flat arm, two day boundaries)

- S3 PASS: in general stores, max units on one cell <= one stack while any
  cell in the zone is empty; FAIL: the centre keeps the max while cells sit
  empty.
- S2 PASS: units in Private stores stop growing from town hauls (only the
  household's own deposits appear there); General stores receive >= 90% of
  hauled units; FAIL: shelves keep filling from DepositRuns.
- Falsifier of the whole row: if the town has NO general store (no painted
  stockpile, no barn) then S2 must report identity every day and Ben's
  complaint stands until a store is painted -- that is a design finding
  (adoption should found a general store), not a bug in S2.
