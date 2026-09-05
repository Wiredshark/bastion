# RESULTS — W8: the route fault at the stall

## The defect (b2 F2c'-b, 2026-09-04 day 1; b1 F2c'-b day 2)

b2 day 1: 59 wedge probes, 48 of them cooks, 45 of those at two spots
between the kitchens (y 6388-6397, z 181) and the store (y ~6356, z
182): (7649,6390,183) and (7685,6390,183). `assist_why=head_far`,
`path_state Exhausted`, the route head three blocks south at the
walker's own z, `last_push_site chaser-refused-rock`; the 5x5 map: the
cell one south solid at z+0, air at z+1, solid at z+2 -- a one-block
slot. The spot pre-exists the dense founding planting (ten cook
stalls at the same feet on the F2c' pair) and no adopted field spans
y=6390. A non-eat fetch expires on its first stall (`FETCH STALLED ...
tolerated=false`) and the expiry shuns the target cell 13,500 ticks.

b1 day 2, the consequence: `cooked_today` 90 -> 40, `targets_shunned`
12 -> 110, 8 starving at the evening line with 3,698 units in store;
`STORE WOULD CLOSE` 7 times ("three stalls on one spot aimed at this
store"). The kitchen could not fetch from its own store.

## Instruments before the fix

W8-i (071312b7c2): `nearby_bodies` and `nearby_bodies_3` on the probe.
First read (b2, F2c'-c pair, 24 probes): 0 at every one. Crowding
refuted; the stalls are geometry.

## The mechanism (W8-f, b4a1eb9aa6)

`route_fault_at_stall(assist_why, search_exhausted)` = `head_far` and
Exhausted. At the first stall, when it holds and no climb was taken,
the chaser's route is dropped (a fresh search from the feet next
tick), the stall warning is cleared so the clock re-arms, `expires` is
cleared (no shun, no early end), and the job's one re-path
(`ROUTE_FAULT_REPATHS_PER_JOB`) is spent; the second stall on the same
job expires as before, so a true trap still ends the fetch, thirty
seconds in instead of fifteen. Witness `ROUTE FAULT AT THE STALL`.
`BASTION_NO_ROUTE_FAULT_REPATH` restores the old path.

Falsified at the commit: `||` for `&&` turned
`only_a_far_head_with_a_spent_search_is_a_route_fault` red at
`bastion_jobs.rs:52450`.

## Registered bars (the W8-f boot of both arms, from 22:22)

targets_shunned per day under 20 (was 59-110); cook stalls at the two
spots a handful, with `ROUTE FAULT` lines in their place (~40 a day);
`cooked_today` at 80 or more; no `STORE WOULD CLOSE`. If the
re-searched route runs into the same slot, the shun count falls only
by half and W8-ii's node verdicts name the next fix.

## Day 1 on the W8-f pair (b2, b4a1eb9aa6, 22:22-22:48)

| bar | registered | read | verdict |
|---|---|---|---|
| targets_shunned per day | < 20 | 38 (28 Cook, 11 EatFrom) | FAIL |
| ROUTE FAULT lines | ~40 | 4 (repaths=4) | the mechanism fires, once per job |
| cooked_today | >= 80 | 43 | FAIL |
| STORE WOULD CLOSE | 0 | 2 (zone 28) | FAIL |
| cook stalls at the two old spots | a handful | 0 | the trap moved |
| starving (dawn) | -- | 1 of 49 | -- |

What the log says the mechanism did and did not do. Cook job 264
(colonist 63, station (7706,6310,181)) stalled at (7691,6299,181)
every ~17 s from 02:27 to 02:33: 21 probes, all `head_far` /
`Exhausted`, the route head three west and one down, `nearby_bodies
0`, `EMBED WATCH` relocated the body twice, no `GOTO-STAND-RESCUE`.
W8-f re-pathed the job once (its allowance) at the second stall; every
later stall expired and shunned another store cell: 20 of the day's
28 cook shuns are that one job, and both `STORE WOULD CLOSE` lines are
its store. So W8-f is right about the route fault and wrong about the
unit: a wedged WALKER stalls the same job again and again, and a
per-job allowance of one re-path spends itself on the first repeat.

Disposition: the row stands as built (a route fault is not the
target's fault; one re-path; no shun on that stall) and its bars are
not met on this boot. The next row, W8-g, blames a repeated stall
spot on the walker (the job is released, the target is not shunned),
which removes the shun chain; the wedge itself -- a body the embed
watch relocated twice into the same trap -- is the mover's row after
that, with W8-ii's node verdicts. (The `GOTO-STAND-RESCUE` lines in
the logs are the vanilla agent's sit-to-stand on a Goto,
`server/agent/src/action_nodes.rs:916`, not a wedge rescue; the embed
watch is the only handler a wedged body has.)

b1's day 1 on the same W8-f pair (the 160-day arm, read late at
00:10): targets_shunned 37, STALLED TARGET SHUNNED 149 in total,
STORE WOULD CLOSE 12, ROUTE FAULT 6 lines (16 re-paths), cooked_today
89, meals 48, one starving at the census. The same picture as b2's:
the mechanism fires and the shun chain still runs.

## W8-g: a repeated stall spot blames the walker (0e865d5b65)

Mechanism: the board keeps `last_stall_spot` per colonist; a stall at
the same block as the walker's previous stall is `StallBlame::Walker`
(`stall_blame(repeated_spot)`), which releases the job and shuns
nothing (witness `STALL BLAMED ON THE WALKER`); a first stall at a new
spot is `StallBlame::Target` and runs the W8-f route-fault path as
before. `BASTION_NO_WALKER_BLAME` restores the old behaviour for a
control boot.

Pin `a_repeated_stall_spot_blames_the_walker` (bastion-server).
Falsified at the commit: always blaming the target turned it red at
`bastion_jobs.rs:52536` (23:15); the tree restored to 0 dirty files.

Registered bars for day 1 on b2 (boot 23:13, the same flat world):

| bar | W8-f read | W8-g bar |
|---|---|---|
| targets_shunned per day | 38 | < 15 |
| STORE WOULD CLOSE | 2 | 0 |
| cooked_today | 43 | >= 80 |
| STALL BLAMED ON THE WALKER | -- | > 0 (the wedged cook's repeats) |
| starving at dawn | 1 | 0 |

Falsified if the shun count stays above 30 or a store closes. NOT
evidenced by this row: the wedge itself (the walker is released, not
freed; the embed watch still relocates it into the same trap).

### Day 1 on the W8-g pair (b2, 0e865d5b65, 23:13-23:48)

| bar | W8-f | W8-g read | verdict |
|---|---|---|---|
| targets_shunned | 38 | 8 (5 Farm, 1 Cook, 1 Craft, 1 Ladder) | PASS |
| STORE WOULD CLOSE | 2 | 1 (zone 25, opt-in witness only) | see below |
| cooked_today | 43 | 87 | PASS |
| STALL BLAMED ON THE WALKER | -- | 2 (Farm jobs 409 and 401, colonists 30 and 12) | fired |
| ROUTE FAULT re-paths | 4 | 2 | -- |
| fed at midday / at dawn (tick 54,000) | -- | 42 of 49 / 43 of 50, starving 0 | PASS |
| food_stock at day 1 | -- | 4,170 (26.6 days) | -- |

The store witness is not the shun chain this time: three DIFFERENT
walkers (colonists 30, 12 and a third, Farm jobs 409, 401, 433) each
stalled once at the same spot on the way into the field at (7636,6438),
and the spot counter reached three. W8-g blames the walker only when
the SAME walker repeats a spot; a spot that stops three walkers is the
world's -- the wedge class, row W9 (`RESULTS-wedge-W9.md`): the
embed watch fired 646 times in these 35 minutes, 604 with a solid
route head, 368 from one colonist at (7687,6447,181) and 210 from
another at (7639,6279,174), the same two cells as the W8-f boots.

Disposition: W8-g's own bars hold (the shun chain is broken: 38 -> 8,
the kitchen recovered 43 -> 87). The remaining stalls are walkers
wedged at trunk nodes inside solid cells, which is W9's row.
