# RESULTS — E1: who is starving with a full store

The symptom, three arms, 2026-09-04 evening: b1 (F2c'-b) evening lines
`starving=6..8 of 50` with 3,700 units in store; b2 (F2c'-b) `starving=7
of 51` with 10,600; b3 (R1 pair) `starving=2` on day 1. The hunger model
read off the code: decay 0.000889/s, a full bar in 15 game hours; fed =
hunger > 0.3, starving = hunger < 0.05; a raw meal restores 0.5, a
cooked one 0.9; 110 meals a day for 51 covers the town in aggregate.
The tail was invisible to every census, so E1-i (f05a802088) names it.

## The instrument's first read (b2, pair 5b111503a0, day 0, 21:56)

27 `STARVING COLONISTS` lines by hour 20 of the founding day. Three
raw lines, ticks 26,100 / 26,400 / 26,700:

```
starving=3 who=30:h0.00:EatFrom { item: Uid(1310) }:Traveling 124:h0.00:EatFrom { item: Uid(1310) }:Traveling 144:h0.00:EatFrom { item: Uid(783) }:Traveling
```

The same three colonists (30, 124, 144), at hunger 0.00, each holding
an `EatFrom` job in state `Traveling`, across at least 600 ticks; two of
them aimed at the same item. The prediction's first branch: the ids
repeat and they are travelling to food they never reach. This is the
mover, not the supper schedule, not the arbiter (their personal drive
won and minted the meal), not the cooldown (they hold the job). An eat
fetch is tolerated at its first stall (`FETCH STALLED ... tolerated=true`)
and serves its full budget, so a starving colonist on a route that
does not arrive stays starving for the budget's length.

## The bodies at the stall (W8-i, 071312b7c2, same boot)

24 wedge probes (19 EatFrom, 4 Designated, 1 Cook): `nearby_bodies=0`
at every one and `nearby_bodies_3=0` at every one. Crowding at the
store cells is REFUTED as the stall mechanism; the walkers stand alone.
The stalls are route geometry, which is W8-f's row (drop the route at a
far head with a spent search and search again from the feet; never
shun the target for it) and W8-ii's instrument (the route's nodes with
their walkable verdicts).

## The starving colonists' own probes (same boot)

Jobs 960 and 1020, both `EatFrom { item: Uid(1310) }`: feet at
(7631,6467,180) and (7635,6461,181), the item at (7634.1, 6470.6,
186.0) -- `to_item` z +6 and +5. The lettuce field 7612-7635 x
6468-6491 spans z 178-185 (a stepped field); the item is a harvest
drop on the upper terrace and the eaters stand on the lower one, with
no route up the step (`path_state: Exhausted`, `assist_why: head_far`,
route heads at z 181). The 19 EatFrom probes on the boot: 8 head_far /
Exhausted, 5 committed_walker / Exhausted, 3 no_head, 4 head_far with
a live search; feet clustered at three spots on that field's lower
edge. `nearby_bodies=0` throughout.

So the tail is field litter on a terrace: the harvest completion drops
its yield on the cell (four units, scattered), an eater targets the
drop, the route cannot climb the step, and the eat fetch is tolerated
for its full `FETCH_BUDGET_SECS` (90 s) before it expires and
retargets. Two rows already in the chain cover it: F3 puts the harvest
in the farmer's bag (no litter, no floating targets) and W8-f drops a
far-head route once and lets the second stall expire and shun the
item's cell (30 s instead of 90). No new row; the E1-i census on the
W8-f and F3 boots is the evidence.

## What decides next

After W8-f: the same census on its boot -- do the starving ids still
repeat, and are they still `EatFrom:Traveling`? If the tail moves to
`none:idle` or `RestAt`, the remaining row is the need scan; if it stays
`Traveling`, W8-ii's node verdicts name the next mover fix.

## What the night answered (W8-f through W9-b, 2026-09-05)

The starving tail on b1's W8-f boot was the wedge: nine starving at
bedtime on day 2, four of them the settlers wedged at (7639,6279,174)
all day (`RESULTS-wedge-W9.md`); on W9 the two trap cells vanished
from both arms (embeds 646 -> 2 on b2, 3,190 -> 21 on b1 by day 2),
and b1's starving read 1 at the day-2 census. What remains on W9 is
pump latency (walkers standing on `no_head`/`head_far` while rejected
trunk routes are searched), which W9-b halves and the W10 rows finish.

## E1-f (b2f3f48d3f): one standability, the item-reach gate on

Landed 02:48 (check clean, both pins green), staged 02:58. Falsified
at the commit: the gate off by default (`.is_none()` -> `.is_some()`)
turned `the_item_reach_gate_is_on_by_default` red at
`bastion_jobs.rs:53129` (03:02); the tree restored clean. The live
read runs on b1 (the 160-day arm, restarted 02:59 on the pair;
`wait-e1f-b1.sh`): the STARVING census at day 1, `no_food_found`, the
item-reach lines, against b1's W9 day 1 (starving 1 at the census,
meals 47, shunned 13).

Day 1 on b1 (03:36; the pair carries W9, W9-b and E1-f):

| read | b1 W9 day 1 | b1 E1-f day 1 |
|---|---|---|
| meals / no_food_found / targets_shunned | 47 / 0 / 13 | 47 / 0 / 11 |
| STARVING at the last census | 1 | 1 (uid 122, `EatFrom … Traveling`) |
| cooked_today / food_stock | 60 / 3,944 | 77 / 4,042 |
| works (lane None) / travel per claim | 325 / 30.8 (control 481 / 30.2) | 411 / 33.2 |
| EMBED WATCH / stall probes / rejected (solid) | 12 / -- / 4,096+ | 25 / 67 / 2,048 |
| house placed / panics | -- / 0 | 6 / 0 |

The reach gate costs nothing to eating, but E1-f's other half cost
the town its builders. The eighth restart test's first boot (R3 pair,
03:30): 98 Build jobs open from PLOT PLAN QUEUED, Build arrivals 0 in
26 minutes (the seventh test's boot, before E1-f: 27 arrivals and 25
cells placed in six); Mine 107, Farm 197, Haul 229 arrivals on the
same boot; `CONNECTIVITY rebuilt cells=57,766` against 69,170 on the
R2 boot. `conn_standable` now delegates to `colonist_walkable`, which
calls a fence or a crop solid, so the reachability flood fill cannot
cross the town's fence line and the house plot beyond it is gated as
unreachable, while the router vaults the same hurdle (W4). b1's E1-f
day 1 placed 6 with builders=0; the P-zero-hours boot before it placed
26. E1-g (`fix-e1g.py`, queued on W9-c's stage): the fill crosses a
hurdle to the standing cell beyond it, as the router does
(`CONN_HURDLES_VAULTED`, `hurdles_vaulted` in the CONNECTIVITY
witness), refactored generic so its pin
(`the_index_vaults_a_fence_as_the_router_does`) runs on the mock world;
planted: the vault removed. Bars on b1 (house lever): cells back above
65,000 with vaults in the hundreds; Build arrivals above 10 and placed
above 15 within ten minutes of the plan; eating unchanged. Falsified
if cells stay near 58,000 or arrivals stay 0 with cells restored.

The reach gate itself shows no benefit of its own on this arm: the
starving tail it was aimed at was the wedge, closed by W9.
The gate stays on (it refuses an item a body cannot stand beside, which
W9's world no longer offers often). E1-f holds. The rise in stall
probes (67) and embeds (25, the terrace class) belongs to W9-b's
sidestep and W10-b's cliff; the work recovered to 85% of the control
on this arm.
