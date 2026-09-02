# RESULTS — a wedged fetch bans the climb it could not make (W2): PASSED 3 of 4 bars; the fourth named W2b, and W2b FAILED

Read 2026-09-02 11:46-11:50 (W2) and 12:25 (W2b) against
PREREG-fetch-climb-ban.md. Arm b1 on 0ad54585cc (D1 + W2) and on
a1441bcf4e (W2b: the ban drops the route), game day 0, raids on; the
baseline is the P1b arm-day (96022099c9) at the same boot clock.

| by 18:00 game day 0     | P1b baseline | W2 pair | W2b pair | bar          |
|-------------------------|-------------:|--------:|---------:|--------------|
| probes (stalled fetches) | 56          | 14      | 38       | --           |
| probes at the spot      | 37           | 3       | 15       | <= 8         |
| shuns                   | 60           | 21      | 33       | <= 30        |
| haul deposits into the store (7776-7780, 6356) | 12 | 13 | 4 | >= 12   |
| EatFrom expiries        | 6            | 1       | 9        | --           |
| evening starving (18-22h) | 1-2        | 1       | --       | --           |
| CLIMB BANNED (fetch)    | --           | 11      | 22 (all route_dropped) | -- |
| one job's re-stalls with the same head | -- | 5 | 14        | <= 2         |

## W2 (0ad54585cc)

- Instrument validation: 16 of the day's 26 probes carried a route head
  two blocks up, and 16 ban lines followed them; none without.
- PASS on the three outcome-side bars (spot 3, shuns 21, deposits 13).
- FAIL on the per-colonist bar: colonist 48 banned the same column five
  times (job 419 probed five times with the same head). Why the numbers
  still fell: a ban resets the fetch clock and the walker stays until
  the 90 s budget expires; before W2 a haul's fetch expired at its first
  15 s stall, so the same spot produced a new fetch, probe and shun
  every 15 s. Part of the drop is slower churn.
- Path states: 25 Exhausted, 1 Path.

## W2b (a1441bcf4e): FAILED, a regression against W2

- The route was dropped on every ban (22 of 22), and the same job
  re-stalled FOURTEEN times at (7748, 6328, 181) with the same head
  (7748, 6327, 183). Deposits into the store fell to 4 (bar >= 12: the
  "reverted" branch), stalls at the spot rose to 15, shuns to 33.
- Why: the route climbs the house by one-block STEPS (dz 1 is never a
  climb, so never banned; the predecessor node is the stair's first step
  at (7747, 6327, 182)), and the mover advances past that node by
  tolerance without gaining the height, so the body faces a two-block
  rise. drop_route reset the search tier to Small (500 iterations in
  total), so every re-search returned the shortest, most
  heuristic-pulled partial path: the stair again, fourteen times.
- Two engine facts settled on this read. The search has three tables:
  per-call slices 250/400/500/750 and TOTAL budgets 500 / 5,000 / 25,000 /
  75,000 by tier; a search exhausted at Longest ran 75,000 iterations and
  the target is unreachable by the move set, not by the budget. And the
  endpoint diagnostic on the W1 boot (533 Longest searches) showed the
  goal resolving within one block every time: the remaining Exhausted
  routes are cut routes, not snapped goals.

## Disposition

W2 held as "improved, not finished". W2b FAILED and is superseded by P1c
(committed behind Y3c): drop_route keeps the tier, and the probe logs the
tier and whether the top tier was exhausted. Rejected on this read: a
larger colonist search tier (the top tier already runs 75,000 iterations).
The next rows the P1c read chooses between: extend W1's withdrawal to
(Longest, Exhausted); a fetch-leg step assist for the stair step the
mover skips; roofs priced out via one-block steps beyond the tile
footprint (the eaves). The W2b pair was Ben's play build from 12:10
until P1c lands; on it the wedge is at W2 level or worse.
