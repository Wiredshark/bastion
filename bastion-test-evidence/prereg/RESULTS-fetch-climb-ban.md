# RESULTS — a wedged fetch bans the climb it could not make (W2): PASSED 3 of 4 bars; the fourth named W2b

Read 2026-09-02 11:46-11:50 against PREREG-fetch-climb-ban.md. Arm b1 on
0ad54585cc (D1 + W2), game day 0, raids on; the baseline is the P1b
arm-day (96022099c9) at the same boot clock.

| by 18:00 game day 0     | P1b baseline | W2 pair | bar          |
|-------------------------|-------------:|--------:|--------------|
| probes (stalled fetches) | 56          | 14      | --           |
| probes at the spot      | 37           | 3       | <= 8  PASS   |
| shuns                   | 60           | 21      | <= 30 PASS   |
| haul deposits into the store (7776-7780, 6356) | 12 | 13 | >= 12 PASS |
| EatFrom expiries        | 6            | 1       | --           |
| evening starving (18-22h) | 1-2        | 1       | --           |
| CLIMB BANNED (fetch)    | --           | 11      | --           |

- Instrument validation: 16 of the day's 26 probes carried a route head
  two blocks up, and 16 ban lines followed them; none without.
- Per colonist: colonist 48 banned the same column five times (job 419
  probed five times at (7650, 6389, 184) with the same head), colonist 55
  three times. The bar "no colonist bans one column more than twice"
  FAILS. The early read at 12:00 had already shown why: the ban changes
  the search profile, which invalidates a retained SEARCH, but the
  chaser's retained ROUTE is kept until it is None or the target moves,
  so the walker followed the same partial path back to the same roof
  edge. W2b (Chaser::drop_route after the ban) is registered and queued.
- Why the numbers still fell without the route being dropped: a ban
  resets the fetch clock and the walker stays on the roof edge until the
  90 s budget expires; before W2 a haul's fetch expired at its first 15 s
  stall, so the same spot produced a new fetch, a new probe and a new
  shun every 15 s. Fewer probes and shuns partly measure slower churn,
  not better routes. The store deposits (13 vs 12) and the expiries (1
  vs 6) are the outcome numbers, and they are flat-to-better, not a
  breakthrough. W2b's read decides whether the re-search reaches the
  store.
- The path states on the W2 day: 25 Exhausted, 1 Path (a complete route
  with a two-block head: a legal scramble the fetch body still cannot
  make; the ban covers it).

## Disposition

PASSED on the three outcome-side bars, FAILED on the per-colonist bar,
with the cause named and W2b queued. Held as "improved, not finished".
Rejected on the read: extending the vault assist to the fetch leg (the
walkers would climb the roof; the store is reachable by ground for the
13 hauls that got there). Open: W3 (the search budget) if W2b's
re-searches exhaust on the ground; the endpoint diagnostic
(BASTION_PATH_ENDPOINT_DIAG) is armed on b1's next boots to read where
the Exhausted searches resolve their goal.
