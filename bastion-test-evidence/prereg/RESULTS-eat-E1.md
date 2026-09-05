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
