# PREREG — a private shelf holds a household's kit, not the town's harvest

Written 2026-09-02 02:40, before the build. Source: flat arm b2 (staged pair
5836a476ca), STORAGE CENSUS day=1.

## What the arm showed

- Every `kind="private"` stockpile zone is ONE cell (`cells=1`): a house's
  shelf is a single block. The spread rule (`stockpile_drop_cell_spread`,
  row S3) therefore has nowhere to spread inside a private store, and the
  summary reads `private_max_cell=101` against `general_max_cell=54`.
- Zone 0 (private) holds 101 units on that one cell; the next private zone
  holds 5. The general stores hold 627 units over 4 zones.

So Ben's two rulings meet on the same cell: "there needs to be some
difference between general stores and private property" and "spread out
item storage so it doesn't just stack on top of each other". A one-cell
household shelf holding a hundred units is both the stack and the missing
difference.

## Mechanism (pure, deterministic)

PRIVATE SHELF CAP. A private store admits a deposit only while the target
cell holds fewer than `PRIVATE_SHELF_CELL_CAP` units (a number of taste,
16 to start, until Ben names another). `store_admits` gains the cap check
so the CHOOSER refuses before the hauler spends the trip (a guard must
refuse before it spends): a full shelf sends the load to the general store
the same way a private store already sends another household's goods
there. The general stores keep the spread rule; their per-cell figure is
the spread's business, not the cap's. Identity: `BASTION_NO_SHELF_CAP`.
Prior art: Banished (a house stores a small personal stock; the barn takes
the rest), RimWorld (per-cell stack limits force stockpiles to spread),
Dwarf Fortress (bins and per-tile stockpile slots).

## Pre-registered pass / fail (flat arm, day lines 1 and 2)

- PASS: `private_max_cell <= 16` on every STORAGE SUMMARY day line after
  the change; `general_units` on day 1 >= the pre-change day-1 figure (627)
  plus the units the shelves no longer take; no rise in haul REFUSALs (the
  chooser redirects, it does not refuse).
- FAIL: `private_max_cell` still above 16 (the cap is not on the path the
  deposits take -- find the other producer), or a REFUSAL CENSUS rise
  (the redirect is refusing instead of redirecting).
- Falsifier of the design: if households stop depositing at home at all
  (private_units falls to ~0), the cap is set below a kit and Ben's "it's
  fine they store in their house" is lost; then the cap is the design
  number to raise, not the mechanism to drop.
