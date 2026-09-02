# PREREG — a wedged fetch bans the climb it could not make (W2)

Registered 2026-09-02 11:00, before the binary exists. Baseline corrected
11:05 (before the binary exists): the first draft assumed no haul reached
the target store on the P1b day; the log reads 12.

## What the wedge probe found (P1, P1b)

Arm b1 on 96022099c9, game day 0: 42 stalled fetches by 15:00, 26 of them
on one spot, feet (7748, 6328, 181), every one with the same block map
(flat ground; a house's roof edge rising to the south) and the same
route: `route_head` (7748, 6327, 183), `route_ahead` (7749, 6327, 184),
`path_state` Exhausted, `route_complete` false, target store zone 69 at
(7776-7780, 6356, 182), 30-40 blocks north-east. The search ran out of
its budget, kept the partial path, and that path's next node is a
two-block climb onto the roof edge. The kinematic mover takes one-block
steps; the step/vault assist and the climb-ban recorder both sit behind
`fetch_steer.is_none()`, so on a fetch leg the body pushes at the roof
edge for 15 s, the stall clock expires the trip, the cell is shunned for
six hours, and the next walker gets the same route.

The design already has the remedy for job legs: a failed climb column is
priced out of that colonist's searches for CLIMB_BAN_SECS (300 sim-s), the
search profile key includes the bans, so the retained route is discarded
and the next search goes another way (Ben: "infinite climb loops... try a
secondary route"). Fetch legs never record one.

A second class on the same arm: 12 probes with `path_state` None and an
empty route, all aimed at store zone 45, a raised terrace behind a
two-high wall. That is W1 (an unreachable store withdrawn on the search's
word), registered separately.

## Mechanism

At the fetch leg's first-stall witness: if the chaser's `route_head` is
two or more blocks above the feet, the head and ahead columns are added to
the colonist's climb bans (same list, same TTL, same cap of 8 as the job
leg), the fetch's progress clock is reset so the re-plan gets its own 15 s,
and a `CLIMB BANNED (fetch)` line is written with feet, head and ahead.
`BASTION_NO_FETCH_CLIMB_BAN` restores the old behaviour. Pure predicate
`route_head_is_a_climb(feet, head)` pinned: dz 2 -> true, dz 1 (a
doorstep) -> false, no head -> false; planted defect: a threshold of 1
would ban doorsteps and the pin goes red.

## Baseline: the P1b arm-day (b1 on 96022099c9, day 0, same boot clock)

| by game hour | probes | at the spot | haul deposits into the store | EatFrom expiries | shuns | evening starving |
|-------------:|-------:|------------:|-----------------------------:|-----------------:|------:|-----------------:|
| 15:00        | 35     | 19          | 12                           | 0                | 40    | --               |
| 18:00        | 56     | 37          | 12 (none between 15 and 18)  | 6                | 60    | 1-2 (18-22h)     |

So the store IS reachable by some approaches (12 deposits before 15:00);
the spot's walkers are the ones routed over the roof.

## Pre-registered outcomes (arm b1's day 0 on the W2 pair, read at 18:00)

Instrument validation first: at the (7744-7751, 6328-6335) spot, every
probe whose `route_head` z is >= feet z + 2 must be followed by a
`CLIMB BANNED (fetch)` line for that job; if probes with such heads exist
and no ban lines do, the mechanism is not reached and nothing below is
read.

- PASS: probes at that spot by 18:00 <= 8 (from 37), AND `haul deposited`
  into the store at (7776-7780, 6356) by 18:00 >= 12 (not below the
  baseline; >= 18 would show the re-plans arriving), AND shuns by 18:00
  <= 30 (from 60), AND no colonist records more than 2 fetch climb bans
  on the same column in the day (the ban worked: the re-plan went
  elsewhere).
- FAIL branches: the bans are recorded but the probes move to another
  wall with `path_state` Exhausted (the ground route exhausts the 750-iter
  budget) -> the search budget row (W3); the bans are recorded and the
  next search returns None (no ground route at all) -> the store is
  reachable only by climbing from that side, and W1's withdrawal applies
  to it; a colonist banning the same column repeatedly -> the ban does
  not reach the search profile on a fetch and the fix is in the profile
  key; deposits fall below 12 -> the bans priced out a route that used to
  work for someone and the row is reverted.
- NOT a fix for zone 45 (W1) and NOT evidenced live yet.
