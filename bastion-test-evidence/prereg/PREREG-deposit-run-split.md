# PREREG — a deposit run spreads its own bag (S8)

Registered 2026-09-02 10:40, before the binary exists.

## What the barn row's failed read found

Row S7 (the deposit re-aim remembers recent drops) was read on arm b1 on
21adf2f5df, day 1: the barn's heaviest cell held 157 units (mushroom x79
and more) with 22 cells in use, against a pre-registered bar of <= 64.
The pre-registration's FAIL branch said "another path drops without the
re-aim, and the census's max_cell_def names it". It did:

- the 40 haul deposits of the day were spread: at most 14 units on any
  cell (`haul deposited` carries its cell);
- the 129 forage deposits (DepositRun jobs, `forage deposited`) put 1,683
  units into three stores, 68 runs into the barn, and their amounts read
  144, 127, 79, 55, 54, 47, 46 ... in ONE drop each. `deposit_all_of`
  drops every slot of a def at one cell. A forager's bag is not a
  hauler's chain load (16); no cell chooser can split a single drop.

The deposit-run's cell IS chosen by the spread chooser at mint, counting
ground items and in-flight loads; the defect is the size of the drop, not
the choice of cell.

## Mechanism

At the DepositRun completion, each def is deposited in chunks of at most
DEPOSIT_CELL_CAP (16, the hauler's chain load: one number for "what one
cell takes from one trip"). Each chunk picks its cell with the spread
chooser over a local occupancy map (ground items in the zone, the recent
drops, and the chunks already placed by this run), so a 144-unit bag lands
on nine cells, not one. Non-stackable items and stacks within the cap are
dropped as they are (identity kept). `BASTION_NO_DEPOSIT_SPLIT` restores
the single drop. The `forage deposited` line gains `cells=` (cell:amount
per chunk) so the drop can show itself failing.

Pin: `a_bag_is_deposited_in_chunks_no_larger_than_the_cap` (144 -> nine
chunks of 16; 5 -> one of 5; 0 -> none; the sum is the amount). Planted
defect: a cap of u32::MAX makes the pin's chunk count 1 and the pin red.

## Pre-registered outcomes (arm b1's first day on the S8 pair)

- PASS: the barn's heaviest cell <= 64 with cells_used >= 20 (the S7 bar,
  unchanged), and every `forage deposited` line with amount >= 32 names
  >= 2 distinct cells.
- FAIL branches: a 100+ cell persists and the `cells=` field shows the
  chunks landing on ONE cell -> the chooser's floor filter (S5b) admits
  too few cells and the row is the chooser, not the drop; a 100+ cell
  whose max_cell_def is a def that was chunked -> another producer
  (the harvest drop, the founding delivery) and the census names it.
- NOT a fix for the eat leak: fewer units per cell does not move the
  store's approach (P1/P1b).
