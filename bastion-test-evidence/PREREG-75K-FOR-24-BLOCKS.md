# PRE-REGISTRATION — why does a 75,000-iteration search fail to close a 24-block gap?

Written before the instrument runs. Successor to the fall-pricing 3v3 (T = C,
disposed honestly in the verdict commit): the burn is demand-side — 65% of
~90k Longest searches per leg aim at EIGHT goal cells, the hottest a reserved
fetch item at (27501, 18256, 409), `end_walkable=true`, closest approach 24
blocks out against a Rock frontier, identical in both arms.

## Established, not being re-tested

- The goal CELL is standable (`end_walkable=true` from the endpoint witness).
- The searches genuinely exhaust (LONGEST-EXHAUST fires; budget 75,000).
- The same cells are re-aimed at for the whole leg (reservation re-forms).
- Repricing falls does not change any of this (3v3, T = C).

## Instrument

Extend the existing LONGEST-EXHAUST NEIGHBOURHOOD witness (same env,
`BASTION_PATH_ENDPOINT_DIAG`) with the explored-set shape at exhaust:
`expanded_states` (visited_nodes.len()), `distinct_cells` (unique `pos` among
visited), `bbox` (min/max of visited pos). One emit per exhaust, diag-gated —
the instrument that separates every branch below in a single leg.

## The branches, which look different in the numbers

- **A — UNREACHABLE IN A\*'S OWN TRUTH.** `distinct_cells` is thousands
  (≈ the colonist's whole reachable region), bbox ≈ town-scale, and every
  exhaust to that goal shows the same saturated shape. The connectivity
  index said "one component" on DIFFERENT predicates than `walkable()`
  (the known index/A\* predicate split) — the goal is connected in index
  arithmetic and unreachable in walk arithmetic. Then the fix is feedback:
  an A\*-verdict must poison the ITEM PICK (the reservation re-forms on a
  goal A\* just proved unreachable — that re-form is the 12,000×).
- **B — STATE-SPACE MULTIPLICATION.** `distinct_cells` ≪ `expanded_states`
  (Node identity is pos × last_dir × last_dir_count, so one cell can be
  visited many times). Then the fix is node identity / dominance pruning in
  the search itself, and the budget was never really 75k cells.
- **C — RE-EXPLORATION ACROSS POLLS.** `expanded_states` small per exhaust
  while the SEARCH COUNT to the goal is huge — each search restarts and
  re-walks the same near-region (retarget reset misfiring on a static item,
  or astar dropped between polls). Then the fix is search lifetime, not
  search internals.
- **VOID** — the witness never fires with the new fields (diag off, wrong
  binary, no Longest exhaust in the window): say so, no inference.

Branches A and B can BOTH be true (a saturated region × state multiplication);
the two numbers separate their contributions arithmetically.

## Falsifier for the instrument itself

A planted 10×10 walled box with the goal outside must report
`distinct_cells` ≈ the box interior (~100), bbox ≈ the box — if it reports
the full budget or a town-scale bbox on a sealed box, the instrument lies.
(Unit test on the existing MockVol harness, not a live leg.)

## Run shape

ONE leg, town, uncapped, deterministic — same recipe as the 3v3 slots. The
question is attributive, not comparative: no arms, no replicates needed for
branch selection (the variance law binds COUNT comparisons, not shape
classification). Judged on the exhausts aimed at the leg's top-3 goal cells.
