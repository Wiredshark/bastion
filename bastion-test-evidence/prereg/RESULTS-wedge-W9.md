# RESULTS — W9: a trunk node stands where a body can stand

Registered 2026-09-04 23:40, before the binary exists. Source: the
`EMBED WATCH` lines of the W8-f boots of b1 and b2 (22:22-23:13) and
the first nine minutes of the W8-g boot of b2 (23:13-23:22), all on the
same flat world.

## The defect, counted

| boot | embed fires | `route_head_solid=true` | top (uid, route head) | writer |
|---|---|---|---|---|
| b2 W8-f b4a1eb9aa6 | 206 | 181 | uid 71 x 161 at (7639,6279,174) | 204 `chaser-pure-glide` |
| b1 W8-f b4a1eb9aa6 | 1,631 | 1,587 | five settlers x 1,159 at (7639,6279,174); uid 34 x 397 at (7687,6447,181) | 1,631 `chaser-pure-glide` |
| b2 W8-g 0e865d5b65, first 9 min | 379 | -- | uid 19 x 355 at (7687,6447,181) | -- |
| b2 W8-g 0e865d5b65, first 35 min (day 1) | 646 | 604 | uid 19 x 368 at (7687,6447,181), 23:14-23:23; uid 27 x 210 at (7639,6279,174) | -- |

| b1 W8-f b4a1eb9aa6, whole boot to day 2 (23:41) | 3,389 | -- | six settlers (uids 131-145) x 3,166 at (7639,6279,174); uid 34 x 397 at (7687,6447,181) | -- |
| b3 R1d 43008d9fdb, restored boot, first 6 min | 20 | -- | (7686.3,6447.5,181) again, from the first minute | -- |
| b2 F3 48fd9c1390, boot to day 1 (00:33) | 614 | -- | uid 43 x 365 at (7687,6447,181); uid 131 x 210 at (7639,6279,174) | -- |

b1's day 2.9 (04:32 log time) puts the cost in people: `STARVING
COLONISTS starving=9`, all on RestAt at their beds with hunger 0.00,
four of them (90, 131, 133, 135) the settlers wedged at (7639,6279,174)
for the whole day (687, 513, 478 and 242 fires). The starving tail of
the 160-day arm is, in this boot, the wedge.

The fourth boot names the same two cells, and b1's second day puts
3,166 of its 3,389 fires at one of them, from the six settlers who
arrived that day (their route into town crosses it). This is the
control the W9 pair is read against.

`builds_since_route=0` on every line. uid 71 fired 47 times a minute
for four minutes while walking from the lounge seat at z=180 to its
bed at z=186, nine seconds after the route was planned (the chunk was
loaded). At (7639,6279,174) the previous node is solid too, so Row 49's
back-along-route relocation stands down and the shared ejector puts the
body at (7629,6269,173); at (7687,6447,181) the previous node is clear,
so the body goes back onto the route. Either way the committed glide
walks it straight back into the head. The wedged cook of W8-f (uid 63,
whose repeated stall shunned twenty store cells) reads the same:
both its route nodes solid.

## The mechanism, read from the producer

Generator: a trunk waypoint's z is `waypoint_z(column_surface_z(..))`,
the topmost NATURAL block (`is_surface_terrain`: Rock, Grass, Earth,
Sand, Snow, Ice) plus one. A built block or a solid sprite standing on
natural ground puts the node inside that mass. `lift_over_ground`
inserts intermediates by the same rule.

Consumer: a committed route glides by pure interpolation
(`pure_glide = committed.is_some()`, "the route was admissible when the
router computed it"); the rock gate `glide_would_enter_rock` guards only
the override branch. The embed watch relocates and the glide returns.

## The fix (two halves, `bastion_jobs.rs`)

1. At the trunk, after `lift_over_ground`, every waypoint is judged by
   `common::path::colonist_walkable` (the router's own rule: floor
   below, feet and head clear, no window, no fence top) and moved to
   the first accepted cell in `TRUNK_NODE_LIFT_ORDER = [0, +1, +2, +3,
   -1, -2]` (`trunk_node_z`; `TRUNK_NODES_LIFTED`). Doors keep their
   probed cell; an unloaded column keeps today's node. A column with
   nothing in reach rejects the route to the search pump
   (`TRUNK_ROUTES_REJECTED_SOLID`; witness `TRUNK ROUTE REJECTED — a
   waypoint's column has no cell a body can stand in`).
2. At the glide, the committed branch tests its STEER NODE (not the
   interpolated step, so a step up is never refused) with the override's
   rock predicate; `committed_glide_verdict(node_in_rock) -> DropRoute`
   queues the walker on `routes_to_drop`, drained after the pass
   (`path_cache`, `route_built_at`), and the hold keeps the body still
   for the tick. Witness `COMMITTED GLIDE REFUSED — the route's steer
   node is inside rock` with `COMMITTED_GLIDE_DROPS`.

Pins: `a_trunk_node_stands_where_a_body_can_stand`,
`a_committed_glide_into_a_solid_node_drops_the_route`. Planted defects
(run at the commit by `falsify-w9.sh`): `trunk_node_z -> Some(z0)`;
`committed_glide_verdict -> Step`.

## Registered bars (b2 restarted on the W9 pair, read by `wait-w9-b2.sh`)

| bar | before | W9 bar |
|---|---|---|
| EMBED WATCH fires, day 1 | 206 (b2 W8-f), 355 in 9 min (b2 W8-g) | < 40 |
| largest (uid, route head) pair | 161 / 355 | <= 5 |
| (7639,6279,174) or (7687,6447,181) as a route head | 161 / 355 | 0 |
| generator half fired | -- | `TRUNK_NODES_LIFTED` > 0 or TRUNK ROUTE REJECTED (solid) > 0 |
| consumer half fired | -- | COMMITTED GLIDE REFUSED > 0 (its count = nodes that went solid AFTER planning) |
| W8-g's bars carried | 38 / 2 / 43 | targets_shunned < 15, STORE WOULD CLOSE 0, cooked >= 80 |

Falsified if any (uid, head) pair exceeds 30, or the trunk rejects
routes by the thousand (the pump's collapse class, not a fix).

Rejected: gating the interpolated step (refuses every step up); lifting
to "topmost solid + 1" without the router's rule (a fence top is solid
and becomes a node -- Ben's fence-running); fixing only the rescue (Row
49 already relocates to the previous node; the glide walks back).

Not evidenced by this row: a wedge between two CLEAR nodes over ground
that stands above the line (`lift_over_ground`'s row); a node that goes
hollow under a dug mine.

## Falsified at the commit (9e96efcd58, 01:15-01:21)

Both plants turned their pin red: `trunk_node_z -> Some(z0)` at
`bastion_jobs.rs:52887` (the "one up" case), `committed_glide_verdict
-> Step` at `:52896`; the falsify tree restored to 0 dirty files after
each. Committed 01:00 with a clean first check and both pins green;
staged 01:15; shipped to lab-bin 01:15.

## Live evidence

b2 restarted on the pair at 01:16 (the same flat world, the seventh
boot of it tonight).

| read | boot +4 min (01:20) |
|---|---|
| EMBED WATCH | 0 (W8-g boot: 379 by nine minutes) |
| the two cells as a route head | 0 / 0 |
| TRUNK nodes lifted / routes rejected (solid) / rejected (dz) | 1,037 / 512 / 15 |
| COMMITTED GLIDE REFUSED (drops) | 64, at store-area nodes z=182 |
| CHASER GLIDE REFUSED INTO ROCK (the override's gate) | 12 |
| STALL BLAMED / ROUTE FAULT / STORE WOULD CLOSE | 0 / 0 / 0 |
| ITEM 39 p95 / jobs | 507 us / 204 |
| food_stock / food_locked / food_anywhere (F-i2) | 3,632 / 0 / 3,632 |

Boot +10 min, both arms (b1 restarted on the pair at 01:24 as the
second replicate; its settlers walk into trap 1):

| read | b2 (01:26) | b1 (01:34) |
|---|---|---|
| EMBED WATCH | 1 | 2 (neither at a trap cell) |
| the two cells as a route head | 0 / 0 | 0 / 0 |
| TRUNK lifted / rejected (solid) | 2,026 / 1,024+ | 2,325 / 1,024+ |
| COMMITTED GLIDE REFUSED (drops) | 512+ | 512+ |
| ITEM 39 p95 | 571 us | 742 us |

The wedge is gone on both arms. The cost: trunk routes rejected by the
thousand (2,048 by 17 minutes on b2), each for one or two nodes in
9-62, and b2's first ten minutes showed 391 job arrivals against the
W8-g boot's 811 -- an arm-noisy count (b1 read the other way against
its own control) that the day-1 lane census settles.

### Day 1 on b2 (01:54) against the registered bars

| bar | W8-g control | W9 read | verdict |
|---|---|---|---|
| EMBED WATCH, day 1 | 646 by 35 min (W8-f: 206 a day) | 2 | PASS |
| largest (uid, route head) pair | 368 | 1 | PASS |
| the two cells as a route head | 368 / 210 | 0 / 0 | PASS |
| generator half fired | -- | lifted 8,133; rejected (solid) 4,096+; rejected (dz) 21 | fired |
| consumer half fired | -- | COMMITTED GLIDE REFUSED 2,048+ (store-cell nodes, z=182-183: piles landing after the plan) | fired |
| targets_shunned / STORE WOULD CLOSE / cooked | 8 / 1 / 87 | 2 / 0 / 83 | PASS |
| meals / no_food_found / panics | 45 / 0 / 0 | 48 / 0 / 0 | -- |
| food_stock / food_anywhere (F-i2) | -- | 3,860 / 3,868 | the frames agree at the day line |
| ITEM 39 p95 | -- | 591 us | -- |

The lane census is the cost: works 206 against the control's 550 with
mean travel per claim 40.8 against 24.5 blocks (b2); on b1, works 325
against its own W8-f day 1 of 481 with travel flat (30.8 against
30.2). Part of b2's drop is honest walking around walls that bodies
used to phase through; b1's flat travel says the rest is the pump:
4,096 rejected trunk routes a day queue on two search slices a tick.
Disposition: W9 stands (the wedge is closed on two arms, every bar
met); W9-b is queued next, ahead of E1-f, with its bars above.

### The residual, read on b3's restored boot (R2 pair, 12 fires in 4 min)

Every one of the twelve carries `route_head_solid=false`. Eleven are
not on any route segment: the body is 12-47 blocks from its route's
FIRST node (`idx == 0`, prev == head), gliding in a straight line from
where it stood -- inside the warehouse at y≈6349, z≈181.6 -- to the
nearest road tile, through the building's wall (the committed drops of
the W9 boots sit on the same line, y=6355 z=182). The trunk validates
its nodes; the leg from the feet to node 0 is nobody's, and a walker
who starts off the road network (a hauler in the store, a restored
colonist at the spot it was saved) walks it raw. The twelfth is a
five-block drop between two clear nodes (7717,6345,186 -> 7715,6342,
181), under the trunk's six-block reject. Candidate W10-a: when node 0
is more than a tile from the feet, the approach is searched exactly
(the pump) and stitched onto the trunk tail; sized by b3's day-1 read
before it is built. Sized (02:22): the restored boot's whole first day
read EMBED WATCH 20 -- 12 in the first four minutes, then one every
four minutes or so -- with `back_along_route=true` on 18 of 20 and no
solid node anywhere; the first-leg burst is what a player sees right
after loading a kept world, which is why W10-a is built.

The five-block drop is not a one-off: b2's W9 day 1 had seven stall
probes (W8-ii), every one an EatFrom walker at the upper terrace edge,
feet (7709-7711, 6344-6345, 186), where the trunk steps from the 186
grade to the 181 grade in one segment. `TRUNK_REJECT_DZ = 6` was set
against 22-block cliffs; a body walks down two. Candidate W10-b: a
trunk segment that drops more than two blocks is rejected to the
pump (which prices the Falls arm), or the segment gets a stair of
intermediate nodes along the ground. Both W10 candidates spend the
pump; W9-b's sidestep lands first and its day-1 rejection count says
how much room there is.

b2's whole W9 boot (1.6 days, read from the saved log at 02:36 after
the W9-b restart): EMBED WATCH 23, rejected (solid) 8,192+, committed
drops 4,096+, starving 1, panics 0 -- and 61 WEDGE PROBE stalls (the
W8-g day: 12), every one `exhausted=false`, with `no_head` 14,
`head_far` 18, `committed_walker` 17: walkers standing still while the
pump works through the rejected routes, at scattered spots (six at
(7637,6331,181), five at (7807,6341,183), five at (7755,6342,185)).
That is the work drop in person, and W9-b's first read is the test of
it.

b1's day 2 on W9 (read 02:38) against its own W8-f day 2: works 308
against 257, cooked 49 against 46, EMBED WATCH 21 by day 2 against
3,190; the two cells 0 / 0; meals 111, no_food_found 0, starving
(last) 1, panics 0. The day-1 work dip reversed by day 2 on the
160-day arm. What rose: targets_shunned 38 (day 1: 13), STALL BLAMED
6, STORE WOULD CLOSE 3, p95 1,043 us -- stalls on pump latency
expiring into shuns, W9-b's target; rejected (solid) 8,192+ by then.

W9-b landed as fb14467af7 (02:23; check clean, pin green; staged
02:35). Falsified at the commit: the neighbours never tried turned
`a_trunk_node_steps_aside_before_the_route_is_thrown_away` red (02:38);
the tree restored clean. b2 restarted on it at 02:36; its reads follow
under "W9-b live".

### W9-b live (b2, fb14467af7)

| read | W9 boot +4 min | W9-b boot +4 min (02:40) |
|---|---|---|
| TRUNK rejected (solid) / lifted / sidestepped | 512 / 1,037 / -- | 256 / 743 / 511 |
| COMMITTED GLIDE REFUSED (drops) | 64 | 256 |
| EMBED WATCH | 0 | 5 (four at the terrace edge (7714-7716, 6342-6344, 182-183), `back_along_route=true`, no solid node) |
| the two cells as a route head | 0 / 0 | 0 / 0 |
| rejected (dz) | 15 | 18 |
| ITEM 39 p95 | 507 us | 486 us |

Half the rejections gone to sidesteps at four minutes; the embeds
that appeared are the terrace-edge class (W10-b's cliff), which a
sidestepped node may now reach the lip of. The ten-minute and day-1
reads decide W9-b's bars (rejections under 100 per ten minutes, embeds
under 40 a day).

Placed on their segments (02:48, eight embeds by then): uids 67, 60,
60 and 62 sit at t = 0.35-0.39, within 0.4 of the line from
(7717,6345,186) to heads at (7714-7716, 6342-6344, 182-183) -- a
three- or four-block drop over 3.2-4.2 blocks that the six-block limit
admits and W10-b's two-block limit rejects; one (uid 14) is a flat
seven-block segment at z=186 with the body at 185 (ground rising above
the line, `lift_over_ground`'s class); two (55, 66) are first legs
11-15 blocks off any segment (W10-a). None is a solid node: W9-b did
not put a node into anything; it let a route reach a cliff it used to
reject for a wall.

| read | W9 boot +10 min | W9-b boot +10 min (02:46) |
|---|---|---|
| TRUNK rejected (solid) / lifted / sidestepped | 1,024 / 2,026 / -- | 512 / 1,531 / 1,026 |
| COMMITTED GLIDE REFUSED (drops) | 512 | 1,024 |
| EMBED WATCH | 1 | 9 (the terrace edge five, first legs, one ground rise) |
| rejected (dz) / STALL BLAMED | 18 / 0 | 22 / 2 |
| ITEM 39 p95 | 571 us | 497 us |

Rejections halved at ten minutes; drops doubled (more routes survive
to be dropped at the store wall) and the embeds are the cliff W10-b
rejects. The day-1 census decides the work bar.

### W9-b day 1 on b2 (03:04) against its bars

| bar | W9 day 1 | W9-b day 1 | verdict |
|---|---|---|---|
| TRUNK rejected (solid), the day | 4,096+ | 1,024 (lifted 4,016; sidestepped 1,960) | -75%, above the bar of ~100 per ten minutes |
| EMBED WATCH, the day | 2 | 18 (the terrace cliff, first legs, one ground rise; no solid node) | under 40, PASS |
| the two cells as a route head | 0 / 0 | 0 / 0 | PASS |
| WEDGE PROBE stalls | 61 in 1.6 days | 10 (no_head 5, head_far 2), none exhausted | down |
| COMMITTED GLIDE REFUSED (drops) | 2,048+ | 2,048+ | unchanged |
| works (lane None) / travel per claim | 206 / 40.8 | 227 / 39.8 (control 550 / 24.5) | FAIL |
| cooked / meals / shunned / no_food_found | 83 / 48 / 2 / 0 | 81 / 48 / 7 / 0 | held |
| food_stock / locked / anywhere | 3,860 / 0 / 3,868 | 3,902 / 45 / 3,963 | the frames agree |
| ITEM 39 p95 / panics | 591 us / 0 | 484 us / 0 | -- |

Disposition: W9-b keeps the wedge closed, cuts the rejections by three
quarters and the stalls by most, and does not move the work. With
stalls down to ten a day the pump is no longer the explanation for
227 against 550; the trips themselves are 60% longer per claim on both
W9 and W9-b. Whether that is bodies walking around walls they used to
phase through (honest, and the end of the "phase" class) or routes
that now climb and detour (a regression the counters cannot see) is
the next instrument: W9-i, a trunk route profile (length against the
straight line, nodes lifted up, down and aside) and a body-height
histogram from the authoritative position dump, read on b2 before any
further mover row.

### W9-b day 2 on b2 (03:47)

| read | day 1 | day 2 |
|---|---|---|
| works (all lanes) | 227 (lane None only, the founding day) | 429 (Mine 131, Farm 131, Cook 57, Haul 55, Build 29, Chop 19, Guard 7) |
| EMBED WATCH (cumulative) / stall probes (cumulative) | 18 / 10 | 44 / 50 |
| rejected (solid) / sidestepped / drops (cumulative) | 1,024 / 1,960 / 2,048 | 4,096 / 3,913 / 4,096 |
| meals / shunned / no_food_found / cooked / harvested | 48 / 7 / 0 / 81 / 0 | 113 / 21 / 0 / 46 / 496 |
| food_stock / anywhere / p95 / panics | 3,902 / 3,963 / 484 us / 0 | 4,240 / 4,253 / 539 us / 0 |

The day-1 work dip on this pair is partly the founding day (every
colonist re-plans at once, nobody named); day 2 reads 429 against a
day-1 control of 550 (the control never ran a day 2 on b2). The
question W9-i answers is unchanged: how much of the remaining gap is
routes climbing walls.

## W9-i, the route profile (instrument, registered 03:15; queued on R3's stage, ahead of W10)

`route_profile(wps)` -> (walked xy length, straight line) in hundredths
of a block, summed over every trunk route with its node count;
`TRUNK ROUTE PROFILE` every 1,024 routes: mean_nodes, mean_len,
mean_straight, detour_ratio, lifted_up, lifted_down, sidestepped,
rejected_solid, rejected_dz. The b2 reader boots with
`BASTION_AUTH_POS_LOG` and histograms colonist z against the grades
181 and 186 (within one block; two to three above; four or more
above). Pinned (`a_route_profile_is_its_length_against_its_line`;
planted: the straight line reported as the length). Registered: honest
walking reads detour_ratio under 1.5, lifted_up a small share, bodies
two or more above a grade under 3%; the regression reads detour_ratio
above 2 or lifted_up rivalling sidestepped, bodies two or more above a
grade over 10%. The next mover row is aimed at whichever number.

W9-i landed as 66902acf5b (03:39; check clean, pin green; staged
03:52). Falsified at the commit: the straight line reported as the
length turned `a_route_profile_is_its_length_against_its_line` red at
`bastion_jobs.rs:53163` (03:55); the tree restored clean. b2 restarted
on it at 03:53 with `BASTION_AUTH_POS_LOG`; the profile and histogram
reads follow under "W9-i live".

### W9-i live: the first profile (b2, boot +4 min, 03:57)

`TRUNK ROUTE PROFILE routes=1024 mean_nodes=23.1 mean_len=86.8
mean_straight=39.5 detour_ratio=2.20 lifted_up=982 lifted_down=0
sidestepped=474 rejected_solid=280 rejected_dz=401`. Both registered
regression signatures at once: the detour ratio above 2 (a trunk route
walks 2.2 times its straight line) and lifted_up (982) at twice the
sidesteps (474), with not one node lifted down. Nearly one node per
route goes UP onto something. The trips are not honest; they climb.
W9-c is the answer already in the chain. (The first body-height
histogram read 100% above four blocks and was wrong: the script took
the dump's y field as z; corrected below.)

At ten minutes (04:03, 3,072 routes): `mean_nodes=18.4 mean_len=65.5
mean_straight=25.2 detour_ratio=2.60 lifted_up=2595 lifted_down=0
sidestepped=1159 rejected_solid=536 rejected_dz=1027`. Body heights
from the position dump (19,737 colonist-ticks, every 30th tick):
within one block of a grade 89.5%, two to three blocks above 10.5%,
four or more 0. Both regression lines crossed: a trunk route walks 2.6
times its straight line, 0.84 nodes per route are lifted UP and none
down, a tenth of every body's time is spent two or three blocks above
the street, and a third of routes (1,027 of 3,072) are rejected for a
step over six blocks that the lifts manufacture. These are the
before-numbers for W9-c, whose reader prints the same fields.

## W9-c, a node stands on ground, not on a wall (registered 03:45; queued on W9-i's stage, ahead of W10)

The regression, read on b1's E1-f day 1 (03:36): 67 WEDGE PROBE
stalls, 55 at one cell, feet (7649,6390,183) -- two blocks above the
181 grade at the store's north edge -- `head_far`, not exhausted, the
W8-ii node window `(7649,6388,183):W (7649,6387,183):W
(7649,6386,183):W (7649,6385,183):W`: every next node walkable, along
the wall line, three blocks away, and no displacement across five Farm
jobs (516, 403, 613, 625, 437; FETCH STALLED, ROUTE FAULT AT THE STALL,
re-search, the same spot). A walker on the store's wall top with a
wall-top route ahead of it and a fetch bridge into the store refused
into rock. `colonist_walkable` is "solid below, two clear above", which
a one-block wall top satisfies; `trunk_node_z` tries +1..+3 before
-1..-2 and takes the top; `trunk_node_fix` finds more one column over.

Mechanism (`fix-w9c.py`): `on_ground_not_a_wall(solid, q)` -- the
block under q has at least `GROUND_NEIGHBOURS_MIN = 3` solid orthogonal
neighbours at its own level (floor, road, cliff edge, corridor: 3-4; a
wall or fence top: 2; a post: 0; an unseen neighbour counts as solid,
identity). One predicate with `colonist_walkable` inside the trunk's
validation, so lift and sidestep both refuse wall tops
(`TRUNK_NODES_WALLTOP_REFUSED`, reported as `walltop_refused` in the
route profile). Pinned (`a_node_stands_on_ground_not_on_a_wall`;
planted: the minimum at two). Bars on b2 (restarted on the pair with
the position dump): lifted_up well below sidestepped; body-ticks two or
more above a grade under 3%; wall-top stalls (z = grade + 2) 0; day-1
works above 400 with travel per claim under 32; embeds under 40.
Falsified if works stay near 230 with the histogram already under 3%
(honest walking: the work drop is the price of no phasing), or
rejections climb past 2,000 a day (the ground rule refuses real
floors).

W9-c landed as 0b0acffedc (04:00; check clean, pin green; staged
04:15). Falsified at the commit: the ground minimum at two (a wall top
passes) turned `a_node_stands_on_ground_not_on_a_wall` red at
`bastion_jobs.rs:53220` (04:18); the tree restored clean. b2 restarted
on it at 04:16 with the position dump; its profile and histogram
follow under "W9-c live".

### W9-c live (b2, 0b0acffedc)

| read | W9-i +4 (1,024 routes) | W9-c +4 (1,024) | W9-i +10 (3,072) | W9-c +10 (2,048) |
|---|---|---|---|---|
| detour_ratio | 2.20 | 1.90 | 2.60 | 2.14 |
| lifted_up per route / lifted_down | 0.96 / 0 | 0.66 / 0 | 0.84 / 0 | 0.53 / 0 |
| sidestepped per route | 0.46 | 2.68 | 0.38 | 2.70 |
| rejected_solid / rejected_dz | 280 / 401 | 622 / 169 | 536 / 1,027 | 1,174 / 311 |
| bodies 2-3 above a grade (two-grade frame) | 11.0% | 10.8% | 10.5% | 10.0% |
| EMBED WATCH | 4 | 3 | 13 | 6 |
| ITEM 39 p95 | 462 us | 447 us | 469 us | 442 us |

Day 1 on W9-c (04:48, 7,168 routes): detour 2.18, lifted_up 0.56 a
route, sidestepped 2.5 a route, rejected_solid 3,750, rejected_dz 808;
EMBED WATCH 10 for the day (W9-i: 13 by boot +10 alone); STALL
BLAMED ON THE WALKER 4, ROUTE FAULT 5; food_stock 3,274, 20.9 days;
meals 46; panics 0.

Disposition: W9-c does what it says -- wall-top lifts fall by a third
per route, the manufactured steep steps by 70%, embeds by half -- and
the two numbers it was aimed at barely move. The detour ratio near 2
is now read as mostly honest: a trunk route follows the road graph
between six-block tile centres, and a road route through a town is
1.5-2 times its straight line where the old routes phased through
walls at 1.0; the sidesteps (2.7 a route) add their zigzag. The 10%
"above a grade" is the instrument's frame, not the town's: b2 has
slopes between its 181 and 186 grades, and a body walking one reads
as two above the lower grade. Both need the right frame before another
mover row: W9-i2 (queued at the end of the chain) counts colonists by
height above the natural surface of their OWN column, which is what a
wall top or a roof is and a slope is not.

### W10-b landed (c6d9d9f55e, 04:43)

Check clean, pin `a_trunk_segment_drops_at_most_two` green (1 passed),
staged 04:43, shipped to lab-bin 04:44. Falsified at the commit: the
limit planted back at six turned the pin red (0 passed, 1 failed),
tree restored clean at 04:46. The live read waits for W10-a's pair on
b2 (`wait-w10a-b2.sh`), where the two rows are read together.

### W10-a landed (0192879b24, 05:07)

Check clean, pin `the_first_leg_is_searched_and_stitched` green (1
passed), staged 05:07, shipped to lab-bin 05:07. Falsified at the
commit: the approach planted as dropped turned the pin red (0 passed,
1 failed), tree restored clean at 05:10. The b2 reader (`wait-w10a-b2.sh`: the
wedge witnesses, FIRST LEG STITCHED, the profile at +4/+10 and days
1-3) run from the stage; both rows are read together there.

### W10-a and W10-b live (b2, 0192879b24, boot 05:08)

| read | +4 min (hour 10) | +10 min (hour 13) |
|---|---|---|
| FIRST LEG STITCHED lines (the witness prints at every power of two, so 1 stitch prints) | 0 | 0 |
| idx0 embeds (prev == head) / stall probes | 4 / 4 | 7 / 20 |
| EMBED WATCH (W9-c: 3 / 6) | 5 | 11 |
| TRUNK REJECTED (dz) with step_limit_blocks=2 | rejected=256 | rejected=512 |
| STALL BLAMED ON THE WALKER / ROUTE FAULT | 0 / 1 | 5 / 2 |
| panics | 0 | 0 |

Day 1 (05:38, hour 0 of day 1): FIRST LEG lines still 0; EMBED WATCH
15 for the day (W9-c day 1: 10), idx0 embeds 11, stall probes 32,
rejected_dz 2,048, STALL BLAMED 5, ROUTE FAULT 2; meals 45, shunned
16; panics 0. W10-b's limit did not move the embed class on this
replicate either.

**W10-a FAILED live: the arm never fires.** Zero first-leg lines in
ten minutes while the class it was built for goes on (seven embeds at
the route's first node, twenty stall probes). Either the gate
(`first_leg_needs_search`, farther than TRUNK_FIRST_LEG_MAX=6 from the
first node) is never true in the live population, or the pump's
completion never reaches the stitch. The mechanism carries no
"considered / needed / searched" counters, so the read cannot tell
which -- the instrument gap is the next row (W10-a-i), before any fix.
W10-b's limit is active (rejected_dz climbs at 256 a read with
step_limit_blocks=2); its effect on embeds is not separable here and
is read on the profile at day 1.

Day 1 on the W10 pair (05:38, one replicate): lane None works 516 /
hauls 90 over 40 colonists, mean travel per claim 25.4 blocks, far
claims 25%, travel share of work 62% -- against W9-c's day 1 of works
262 / hauls 117 over 43, travel 35.8, far 36%, travel share 42%. With
W10-a dead, whatever moved here is W10-b's (routes that drop more than
two blocks a step are refused) or the 2-3x day-to-day swing the
three-replicate law exists for. Not a result until b2 shows it twice
more; noted so the W10-a-i reads can carry the same line.

Third replicate (W10-a-i pair, day 1, 06:17): works 298 / hauls 108
over 43, travel per claim 45.2, far 47%, travel share of work 58%. The
three day-1 lines on the same arm now read works 262 / 516 / 298 and
travel 35.8 / 25.4 / 45.2: the 516 was the swing, not W10-b. Withdrawn
as a signal; the works figure needs a different instrument than one
day line.

## W10-a-i, registered 05:30 (keyed on the R3-b stage, before the binary)

An instrument row. `first_leg_gate(needs_search, search_pending)` ->
Near | BlockedPending | Searched, evaluated once per trunk route
built; the stitch match's wildcard arm counts a tail it removes under
another target instead of dropping it silently; FIRST LEG GATE every
256 routes with routes / near / blocked_pending / searched / stitched
/ unreachable / tail_dropped. Behaviour unchanged. Pin
`the_first_leg_gate_names_its_arm`; planted defect: blocked reads as
searched, red.

Prediction (b2 fresh, +10 min): near + blocked_pending + searched =
routes at 256 and 512. The arm holding searched at zero names the
fix: blocked_pending near routes = the pump's one-search-per-colonist
rule starves the approach (a lane); near at ~100% = the first node is
always within a tile on a fresh arm and W10-a's case is the restored
boot only (read on test 9); searched > 0, stitched = 0, tail_dropped =
searched = the completion arrives under another target (the match).
Falsified if the sum misses routes, or if stitched > 0 (W10-a was
alive and +10 was early).

### W10-a-i landed (87ff5dbec4, 05:53)

Check clean, pin `the_first_leg_gate_names_its_arm` green (1 passed),
staged 05:53, shipped to lab-bin 05:54. Falsified at the commit:
blocked reading as searched turned the pin red (0 passed, 1 failed),
tree restored clean at 05:56. The b2 reader (`wait-w10ai-b2.sh`:
FIRST LEG GATE at +4, +10, days 1-2, with the embed and profile lines
beside it) runs from the stage.

### W10-a-i live (b2, 87ff5dbec4, boot 05:55)

+4 min (hour 11): FIRST LEG GATE routes=256 near=256 blocked_pending=0
searched=0 stitched=0 unreachable=0 tail_dropped=0; the profile at the
same read: routes=1,024, detour 1.71, lifted_up 1,039, walltop_refused
3,490, rejected_solid 392, rejected_dz 475; EMBED WATCH 3, all three at
the route's first node (idx0); stall probes 0; panics 0.

+10 min (hour 15): FIRST LEG GATE routes=768 near=768, the rest 0;
profile routes=3,072, detour 1.90, rejected_solid 1,319, rejected_dz
1,114 (79% of routes refused, so the gate's 768 against 3,072 is the
accepted share, as the code's structure says); EMBED WATCH 7, idx0 5;
ROUTE FAULT 4; ITEM 39 p95 422 us; panics 0.

Day 1 (06:18, hour 0): FIRST LEG GATE routes=1,536 near=1,536, the
rest 0; profile routes=6,144, detour 2.12, rejected_solid 2,827,
rejected_dz 1,735; EMBED WATCH 11 for the day, idx0 9; ROUTE FAULT 4;
panics 0. The arm was then restarted on W10-a-b.

Two findings, both against W10-a's premise. **Near is 100%**: on a
fresh arm the first node is never farther than a tile, so
`first_leg_needs_search` is never true and the stitch can never fire;
and the embeds still happen at that near first node, so the body
glides a SHORT first leg through something -- the wall between a
house or store and the nearest road tile -- not a long one. **The
"quarter" is two frames compared as one, withdrawn**: the gate prints
at multiples of 256 and the profile at multiples of 1,024, so "256"
and "1,024" are the last thresholds each crossed, not counts at the
read; the fraction is unknown from this line. What IS known from the
profile: rejected_solid 392 + rejected_dz 475 of 1,024 routes -- the
W9-c ground rule and W10-b's step limit refuse most trunk routes on
this arm, and the gate sits under `if let Some(wps) = trunked`, so a
refused route never has a first leg at all; where those bodies walk
instead is the next read. The fix that follows (W10-a-b) must (1) sit
where every accepted route is built and (2) gate on the LINE, not the distance:
the straight first leg is walked cell by cell against
`colonist_walkable` and searched when it crosses a cell a body cannot
pass -- prior art is Detour's raycast before a straight move.

## W10-a-b, registered 06:08 (keyed on the P-zero-hours-b stage, before the binary; E2 re-keyed behind it)

`first_leg_crosses_solid(solid, feet, node0)`: the straight segment
from the feet to node 0's centre sampled every half block at the
feet's z and the head's z; any solid cell but the body's own means
the approach goes to the pump and the trunk waits as a tail (W10-a's
search-and-stitch, unchanged). The gate is far OR crossing;
`crossed` joins the FIRST LEG GATE line. Unseen terrain is clear
(identity). Pin `the_first_leg_is_walked_before_it_is_assumed`;
planted defect: the line is never walked, red.

Prediction (b2 fresh, +10 min): crossed > 0, searched >= crossed -
blocked_pending, FIRST LEG STITCHED prints for the first time, idx0
embeds in the first ten minutes at most 1 (3 at +4 on W10-a-i, 7 at
+10 on W10-a), EMBED WATCH at +10 at most 3 (11, 6), ITEM 39 p95
under 600 us. Falsified if crossed > 0 and stitched stays 0 (the arm
that eats the search is named by blocked_pending or tail_dropped), or
if stitched > 0 and idx0 embeds do not fall.

### W10-a-b landed (9488400a79, 06:44)

Check clean, pin `the_first_leg_is_walked_before_it_is_assumed` green
(1 passed), staged 06:44, shipped to lab-bin 06:45. Falsified at the
commit: the line never walked turned the pin red (0 passed, 1
failed), tree restored clean at 06:48. The b2 reader
(`wait-w10ab-b2.sh`: FIRST LEG GATE with `crossed`,
STITCHED, idx0 embeds, the profile, at +4, +10, days 1-2) run from
the stage.

### W10-a-b live (b2, 9488400a79, boot 06:46)

| read | +4 min (hour 10) | +10 min (hour 13) | bar |
|---|---|---|---|
| FIRST LEG GATE | (not yet at 256) | routes=256 near=228 crossed=28 blocked_pending=12 searched=16 stitched=13 unreachable=0 tail_dropped=1 | crossed > 0; searched >= crossed - blocked |
| FIRST LEG STITCHED lines / last | 3 / stitched=4 searched=4 | 4 / stitched=8 searched=10 | stitched > 0 |
| idx0 embeds (prev == head) | 0 | 1 | at most 1 (W10-a: 7 at +10) |
| EMBED WATCH | 0 | 2 | at most 3 (W10-a: 11; W9-c: 6) |
| ITEM 39 p95 | 476 us | 503 us | under 600 |
| STALL BLAMED / ROUTE FAULT / stall probes | 0 / 0 / 0 | 4 / 2 / 0 | -- |
| profile detour / rejected_solid / rejected_dz | 2.02 / 375 / 533 | 2.24 / 999 / 843 | -- |
| panics | 0 | 0 | 0 |

Day 1 (07:18, hour 0): FIRST LEG GATE routes=512 near=471 crossed=41
blocked_pending=19 searched=22 stitched=19 unreachable=0
tail_dropped=3 (STITCHED lines 5); EMBED WATCH 6 for the day (W10-a
15, W10-a-i 11, W9-c 10), idx0 5; STALL BLAMED 5, ROUTE FAULT 4;
profile routes 6,144, detour 2.31, rejected_solid 3,268, rejected_dz
2,152; p95 509 us; starving 3 at the day line; panics 0. The blocked
share is 46% of crossings at the day line (43% at +10).

**W10-a-b PASSED** its registered bars. The first leg is searched
exactly where the straight line would cross a wall, the stitch fires
(13 in the first 256 accepted routes), and the first-node embed class
falls from seven in ten minutes to one. The searched count equals
crossed minus blocked_pending to the unit, so the arms are exhaustive.
The residual is the blocked_pending arm: 12 of 28 crossings met a
search already pending for that colonist and glided raw -- the pump's
one-search-per-colonist rule, named by W10-a-i's prediction as "a
lane, not a distance". That is the next mover row (W10-a-c) if the
day-1 and day-2 reads hold the share near 40%; tail_dropped 1 is the
third arm and stays counted.

## W10-a-c, registered 07:02 (keyed on the E2 stage, before the binary; T1 re-keyed behind it)

The pump keeps one search per colonist in two lanes: Fill (a
committed path into path_cache) and Detour (a wall detour with its
own tier and counters). `first_leg_gate_lanes(needs_search, pending)`:
a pending Fill is superseded by the approach (ReplacedFill: the
approach search overwrites it, the tail too), a pending Detour still
blocks (BlockedPending, as before), near still wins;
`replaced_fill` joins the FIRST LEG GATE line. Pin
`a_pending_fill_search_yields_to_the_approach`; planted defect: a
pending Fill blocks, red.

Prediction (b2 fresh, +10 min): blocked_pending at most 2 of the first
256 routes (12 on W10-a-b) with replaced_fill taking the rest of that
share; searched + replaced_fill >= crossed - blocked_pending; stitched
at least 0.8 x (searched + replaced_fill); idx0 embeds 0-1; EMBED
WATCH at most 2; STALL BLAMED and ROUTE FAULT no higher than 4 and 2;
p95 under 600 us. Falsified if blocked_pending stays near 12 (the
pending lane is Detour) or STALL BLAMED climbs (the replaced Fill was
serving a walk this route did not own).

### W10-a-c landed (be49881c8e, 07:54)

Check clean, pin `a_pending_fill_search_yields_to_the_approach` green
(1 passed), staged 07:54, shipped to lab-bin 07:54. Falsified at the
commit: a pending Fill blocking turned the pin red (0 passed, 1
failed), tree restored clean at 07:57. The b2 reader
(`wait-w10ac-b2.sh`: FIRST LEG GATE with
`replaced_fill`, idx0 embeds, STALL BLAMED, at +4, +10, days 1-2)
run from the stage.

### W10-a-c live (b2, be49881c8e, boot 07:56)

| read | +4 min (hour 11) | +10 min (hour 15) | bar |
|---|---|---|---|
| FIRST LEG GATE | (not yet at 256; STITCHED 16 of 22 searched) | routes=512 near=476 crossed=36 blocked_pending=1 replaced_fill=17 searched=35 stitched=28 unreachable=0 tail_dropped=2 | blocked at most 2; searched + replaced >= crossed - blocked; stitched >= 0.8 x |
| idx0 embeds / EMBED WATCH | 1 / 3 | 2 / 6 | 0-1 / at most 2 |
| STALL BLAMED / ROUTE FAULT | 1 / 0 | 1 / 0 | no higher than 4 / 2 |
| ITEM 39 p95 | 463 us | 491 us | under 600 |
| profile detour / rejected_solid / rejected_dz | 1.97 / 404 / 498 | 2.15 / 2,152 / 1,396 | -- |
| panics | 0 | 0 | 0 |

Day 1 (08:21, hour 0): FIRST LEG GATE routes=768 near=726 crossed=42
blocked_pending=1 replaced_fill=18 searched=41 stitched=33
tail_dropped=2; EMBED WATCH 9 for the day (W10-a-b 6, W10-a-i 11,
W10-a 15, W9-c 10), idx0 5 (W10-a-b 5); STALL BLAMED 4, ROUTE FAULT 0;
profile routes 7,168, detour 2.13, rejected_solid 4,097, rejected_dz
2,094; p95 570 us; starving 3 at the day line; panics 0.

Day 2 (09:04, hour 0): FIRST LEG GATE routes=3,072 near=2,872
crossed=200 blocked_pending=1 replaced_fill=32 searched=199
stitched=172 tail_dropped=15; EMBED WATCH 31 over two days (22 on day
2 alone), idx0 21; STALL BLAMED 5, ROUTE FAULT 5; profile routes
15,360, detour 2.40, rejected_solid 7,848, rejected_dz 4,376; p95 582
us; panics 0. The gate's arms stay exhaustive and the stitch rate
holds at 86%; the embeds grow with the second day's traffic, and
they are the mid-route class (W10-d), not the first leg.

Disposition at day 1: the mechanism half PASSED on both reads
(blocked_pending 19 -> 1 over the day, the replaced arm carrying 18,
stitched 33 of 41), and the outcome half is NOT separable from
W10-a-b (9 / 5 against 6 / 5, inside the swing). The first-node
embeds that remain are not first-leg glides -- the first leg is now
searched and stitched on every crossing -- which is what the six
embed lines already said (chaser-pure-glide, entry_vel z +3.8, mid-
route). W10-d carries that class.

Disposition at +10: the mechanism half PASSED -- blocked_pending
12 -> 1 with the Fill arm replaced 17 times, searched equals
crossed minus blocked to the unit, stitched 28 of 35 (the 0.8 bar
exactly), no stall or route-fault rise, the pump under 500 us. The
outcome half did NOT pass at this replicate: idx0 embeds 2 and EMBED
WATCH 6 against W10-a-b's 1 and 2 at the same read (W10-a-i 5 / 7,
W10-a 7 / 11, W9-c 6). One replicate on a count that swings 2-3x; the
day-1 line (W10-a-b: 5 / 6) is the comparison that decides whether
the remaining first-node embeds are first-leg glides at all.

The six embeds themselves (b2, +10): every one
writer_site="chaser-pure-glide", route_head_solid=false,
back_along_route=true, entry_vel z=+3.8, relocated_to two blocks above
embedded_at -- the glide met a wall between two ground nodes mid-route
and the step-up lodged the body in the wall's top; uid 47 three times
at (7567, 6386). Not the first leg: the other twenty-odd segments were
still straight lines assumed clear.

## W10-d, registered 08:12 (keyed on the T1 stage, before the binary; W9-i2 re-keyed behind it)

`segment_crosses_solid(solid, a, b)` walks every trunk segment's line
every half block, feet and head, at a height that follows the segment
(a one-block terrace step is a step, not a wall);
`trunk_crossing_segment` names the first crossing leg; a route with
one is refused to the exact pump like the ground rule and the step
limit refuse theirs -- TRUNK ROUTE REJECTED (crossing) at powers of
two, `rejected_crossing` on the profile. Pin
`every_leg_is_walked_before_it_is_assumed`; planted defect: the
height not following the segment (a step reads as a wall), red.

Prediction (b2 fresh, +10 and day 1): rejected_crossing between 3%
and 20% of routes; EMBED WATCH at +10 at most 2 and at day 1 at most
3 (W10-a-b 6); idx0 at day 1 at most 2 (W10-a-b 5); p95 under 800 us;
the accepted share no lower than 15%. Falsified if rejected_crossing
is under 1% and the embeds hold (the wall is off the trunk's lines),
or if p95 passes 800 us (the pump's budget is the row).

### W10-d landed (56bdcff764, 09:34)

Check clean, pin `every_leg_is_walked_before_it_is_assumed` green (1
passed), staged 09:34, shipped to lab-bin 09:34. Falsified at the
commit: the height not following the segment turned the pin red (0
passed, 1 failed), tree restored clean at 09:37. The b2 reader
(`wait-w10d-b2.sh`: rejected_crossing on the profile, the embeds and
their writers, the pump cost, at +4, +10, days 1-2) run from the
stage.

### W10-d live (b2, 56bdcff764, boot 09:35)

| read | +4 min (hour 10) | +10 min (hour 13) | bar |
|---|---|---|---|
| profile routes / rejected_solid / rejected_dz / rejected_crossing | 1,024 / 243 / 337 / 410 | 3,072 / 1,079 / 1,212 / 709 | crossing 3-20% |
| accepted share (routes minus the three refusals) | ~3% | ~2% | no lower than 15% |
| FIRST LEG GATE line | none (fewer than 256 accepted) | none | -- |
| EMBED WATCH / idx0 / writers | 2 / 2 / chaser-pure-glide 2 | 10 / 7 / chaser-pure-glide 9, chaser-hold 1 | at most 2 / -- |
| STALL BLAMED / ROUTE FAULT | 0 / 0 | 4 / 3 | -- |
| ITEM 39 p95 | 441 us | 462 us | under 800 |
| detour ratio | 1.57 | 1.85 | -- |
| panics | 0 | 0 | 0 |

**W10-d FAILED its outcome.** The crossing test fires on 23% of
routes (above the band's edge) and, stacked on the ground rule and
the step limit, leaves the trunk accepting about one route in fifty;
the pump carries the rest cheaply (p95 462 us, working/moving as
before). And the embeds did not fall -- ten in ten minutes is the
worst read of the series -- all from the chaser's pure glide, seven
of them at the route's first node. With the trunk all but gone, the
premise that the embeds are trunk legs through walls is refuted: the
same writer embeds bodies on the exact pump's paths. The refusal is
principled and useless here; the next row names the writer from the
embed lines' own geometry (entry point, velocity, head and previous
node) before anything else is built. Disposition for the refusal
itself: keep the count, drop the refusal (a row, W10-d-r, so the
trunk is not dead by a rule that bought nothing).

The ten embeds' geometry (b2, +10, from the EMBED WATCH fields):
all kinematic=true, all chaser-pure-glide; eight of ten a body with a
ONE-node route (route_prev == route_head == the goal, 40-45 blocks
off) gliding at (-0.6, -4.2) into the same wall line at y 6348 (uids
136, 141, 139, 84, 135, 87, x 7777-7804, within a minute); one at
(7762, 6142) -> (7833, 6175), 78 blocks off; one chaser-hold with no
route. The chaser's steer is the raw target whenever path_cache holds
no route -- the window while the pump's search is pending -- so a
walker whose trunk was refused sets off in a straight line at its
goal. **That is the class**: every refused route (79% before W10-d,
98% under it) was a straight pre-path glide, and the wedge rows since
W9 trimmed the trunk's own faults around it.

## W10-e, registered 09:55 (keyed on the E2-c stage, the end of the chain, before the binary)

NO PATH, NO GLIDE: with no route in the cache the steer becomes the
body's own position unless the goal is within NO_PATH_GLIDE_MAX = 3
blocks and the straight line to it is clear; GLIDE HELD FOR THE PATH
at powers of two. W10-d's refusal becomes a count (TRUNK ROUTE
CROSSING (counted), `rejected_crossing` kept on the profile line for
continuity). Pin `no_path_no_glide` (forty blocks holds; two clear
glides; two through a wall holds; just past the limit holds); the
W10-d pin stays green in the chain. Planted defect: the distance
limit removed, red. Prior art: RimWorld pawns stand until a path
exists; Detour agents hold until the corridor is built.

Prediction (b2 fresh, +10 and day 1): GLIDE HELD fires (thousands of
held ticks by +10); EMBED WATCH at +10 at most 2 (W10-d 10) and at
day 1 at most 3 (W10-a-c 9, W10-a-b 6); idx0 at day 1 at most 1;
EXPERIENCE stuck no higher than 3 and working no lower than 8 of 48;
the trunk's accepted share back near 20%; p95 under 600 us.
Falsified if the embeds hold with held ticks in the thousands, or if
stuck or idle climb past 6 (holding starves the work: the pump's
latency is the row).

### W9-i2 landed (97612eacd3, 09:56)

Check clean, pin `a_height_is_measured_from_its_own_ground` green (1
passed), staged 09:56, shipped to lab-bin 09:57. Falsified at the
commit: measured from the surface block itself turned the pin red (0
passed, 1 failed), tree restored clean at 09:59. The b2 reader
(`wait-w9i2-b2.sh`: BODY HEIGHT CENSUS at
+4, +10, days 1-3, beside the profile and embed lines; T1's read on
the same boot at +15) run from the stage.

### W9-i2 live (b2, 97612eacd3, boot 09:57)

| read | +4 min (hour 11) | +10 min (hour 14) |
|---|---|---|
| BODY HEIGHT CENSUS, last three lines (on_ground / one_above / two_plus / unseen) | 40/7/2/0, 43/4/2/0, 44/3/2/0 | 44/5/0/0, 48/1/0/0, 46/3/0/0 |
| two_plus over all 47 lines to +10 | -- | 0 on 29 lines, 1 on 2, 2 on 15, 3 on 1 (mean 0.7 of 49, 1.4%) |
| EMBED WATCH / COMMITTED GLIDE REFUSED | 3 / 4 | 13 / 5 |
| FIRST LEG stitched / searched | 32 / 32 | 64 / 64 |
| profile routes / detour | 1,024 / 1.84 | 4,096 / 1.96 |
| panics | 0 | 0 |

Day 1 (10:25, hour 0): the last three lines 47/3/0/0, 47/3/0/0,
50/0/0/0; over all 124 lines to the day line the two_plus count reads
0 on 58, 1 on 7, 2 on 16, 3 on 6, 4 on 5, 5 on 9, 6 on 10, 7 on 3, 8
on 3, 9 on 1, 11 on 6 -- a mean of 2.4 of 50 (5%) with peaks of 11
(22%). EMBED WATCH 16 for the day (the W10-d refusal pair); panics 0.
**The day-1 bar (under 3%) FAILED**, and the instrument cannot say
what the 5% is: a builder standing on the wall it raises and a
household on a raised floor are both "two above their own column",
and neither is a walker on a wall. W9-i3 splits the count by indoors
(inside a Bed-designated house), Build lane, and other, before the
number is read against the mover rows again.

**W9-i2 PASSED at +10** (the frame): bodies two or more above their own column run at
about one in seventy, not one in ten -- the two-grade histogram was
reading the town's slopes. One pair of bodies held two-up for about
five minutes (fifteen consecutive lines) and then none; the day reads
say whether that pair recurs (a roof, a stair, or the wall class the
mover rows chase). The EMBED WATCH of 13 at +10 is the W10-d pair's
refused trunk (W10-e, queued, holds the walker instead).

### W10-e landed (aa05aa373c, 11:05)

Check clean, both pins green (its own and W10-d's), staged 11:05. The
falsifier (the distance limit removed) and the b2 reader
(`wait-w10e-b2.sh`: GLIDE HELD, the trunk's accepted share, the
embeds and their writers, the experience census, at +4, +10, days
1-2) run from the stage. The b2 restart ends W9-i2's reader before its
day-2 line; W9-i3 carries the height census forward.

### W10-e FAILED LIVE (b2 fresh on aa05aa373c, read 11:10-11:17)

The falsifier went red (the distance limit removed, restored clean
11:08). Live, the town stopped. +4 min (hour 11 of day 0): GLIDE HELD
held_ticks=262,144, EXPERIENCE working=0 moving=49 stuck=0, EMBED
WATCH 0, p95 716 us. Hour 13: held_ticks=524,288 against 49 x 14,700
= 720k colonist-ticks (three in four held), EXPERIENCE working=2
moving=47 at three consecutive samples, three "arrived at job site"
lines in six game hours, 214 jobs open. The last witnesses: uid 13
dist_xy 84.3 crosses=false pending=true; uid 143 dist_xy 39.1
crosses=false pending=true. Every held walker had a search pending.
The search pump advances TWO searches a tick, round-robin across the
whole board (`THE SEARCH PUMP: two slices per tick`; its delivery arm
calls itself a PURE FALLBACK): with forty-seven walkers holding, each
search stepped once in twenty-four ticks. The embed bars (0 at +4)
were met by a town that did not walk, which is the null a hold
produces by construction and not evidence. Disposition: W10-e's rule
is withdrawn; the pre-path glide is the town's locomotion, and the
defect it fixed was the glide INTO a wall, not the glide.

## W10-f, registered 11:22 (keyed on the W9-i3 stage; inserted ahead of T1-b and E2-e; before the binary)

`glide_leg_end(feet, target)`: the line to the goal cut at
TRUNK_FIRST_LEG_MAX (six blocks; a goal within six is its own end).
With no route, `first_leg_crosses_solid` is walked on that leg and
`glide_held_for_path(crosses)` holds the body only when the leg
crosses solid; NO_PATH_GLIDE_MAX is gone. Witness GLIDE HELD AT A
WALL (uid, dist_xy, leg, pending, held_ticks). Pin
`no_path_glides_only_a_clear_leg` replaces `no_path_no_glide`:
eighty-four along +x from (10.5, 10.5) gives the leg end (16, 10); two
along gives (12, 10); a clear leg glides, a crossing leg holds.
Planted defect: the leg not cut (checked at the goal), red.
Prediction (b2 fresh, `wait-w10f-b2.sh`): GLIDE HELD under 2,000 by
+10 (W10-e 524,288 by +9); working at least 8 at +10 (2); job-site
arrivals at least 20 by +10 (3); EMBED WATCH at most 2 at +10 and 3
by day 1 (W10-d: 10 a day, eight glides into walls); stuck at most 3;
p95 under 600 us. Falsified if the embeds return to W10-d's count
(the wall sits in the seventh block or on another z), or if the holds
at walls pass 20,000 with working under 8 (a hold at a wall as
permanent as W10-e's). Rejected: reverting W10-e outright (eight of
ten embeds return); a hold with a timeout (the same embed later); more
pump slices (the pump's budget is the tick's). W9-i3's pair
(654958baa5) still carries the W10-e hold: its b2 reads are of a held
town and its height context is read under that confound; the
lab-bin pairs from aa05aa373c until W10-f ships are not to be played.

### W10-f landed (ca5fc654a5, staged 11:52; falsifier red 11:55)

Check clean, the pin green, both halves built and staged 11:52; the
falsifier (the leg not cut) went red and restored clean at 11:55.
lab-bin carries the pair from 11:53. b2 restarted on it at 11:54;
the +4, +10 and day-1 reads follow (`wait-w10f-b2.sh`).

### W10-f FAILED LIVE (b2 fresh on ca5fc654a5, boot 11:54, read 11:57-12:03)

The same freeze, and its true shape. +4 min: GLIDE HELD AT A WALL
held_ticks=262,144, EXPERIENCE working=0 moving=49, ARRIVED 0. Hour
13: the claim refusal census colonists_seen=0 considered=0 at every
sample from tick 300 (every colonist holds its boot deposit run and
never finishes it); "job claimed" 0; "arrived at job site" 0; TRUNK
ROUTE PROFILE 0 lines and FIRST LEG GATE 0 lines (no trunk route was
ever built); all 19 witness lines across 17 bodies carry
pending=true; POS-WRITE 5 lines in the run; UNREACHABLE PROVEN 177
lines, all job 45. The colonists start inside their houses; the line
to the store crosses the house wall within six blocks; the approach
search is pending and never delivers a usable route; the hold keeps
them indoors. On the W10-d pair the same boot glided them out,
through the wall now and then (the embed class, ten a day). The
hold fixed ten embeds a day by stopping the town. The W10-e read was
this same shape (its "far walker" story was the rule's, not the
town's). +10 min (hour 16, read 12:03): held_ticks=524,288, ARRIVED
0, working=0 moving=49, EMBED WATCH 0 (the null a held town produces
by construction), p95 744 us. Disposition: the hold is withdrawn in
both forms.

Replicate on the other world (b1, the 160-day arm, fresh on the T1-b
pair 5599d1cbd6 which carries the W10-f hold; boot 12:14, read at
tick 7,512 / 12:20): "job claimed" 0, "arrived at job site" 2, GLIDE
HELD held_ticks=262,144, TRUNK ROUTE PROFILE 0, EXPERIENCE working=2
moving=47, claim census colonists_seen=0, UNREACHABLE PROVEN 124. The
same arm on the E2-d pair had 941 claims and 1,665 arrivals by day 2.
Two worlds, two arms, one shape: the freeze is the hold's.

## W10-g, registered 12:05 (keyed on the T1-b stage; inserted ahead of E2-e; before the binary)

`no_path_steer(feet, target, leg_crosses) = target`: with no route the
body glides again (W10-d and every pair before). `glide_leg_end` and
`first_leg_crosses_solid` stay as the COUNT: GLIDE INTO A WALL
(counted) (uid, dist_xy, leg, pending, glides_into_wall), the base
rate for W11, which catches the body at the wall by its feet and
drops the route. GLIDES_HELD becomes GLIDES_INTO_WALL. Pin
`no_path_glides_and_the_wall_is_counted` (W10-f's re-stated: the
leg cut at six; the steer is the target whether the leg crosses or
not); planted: the hold restored, red. Prediction (b2 fresh,
`wait-w10g-b2.sh`): "job claimed" at least 40 by +4 (0); arrivals
at least 20 by +10 (0); working at least 8 at +10 (0); TRUNK ROUTE
PROFILE at least one line by +10 (0); GLIDE INTO A WALL fires and its
day-1 count is the base rate; EMBED WATCH at most 12 by day 1 (W10-d:
10); p95 under 700 us. Falsified if the claims stay under 40 with
the glide back (the freeze was never the hold; the W9-i3/E2-d lineage
on this world is the suspect, and the next boot is the E2-d pair on
b2), or if EMBED WATCH passes 30 by day 1. Rejected: a hold with a
timeout; a boot-only exception; more pump slices before the pump has
a witness (W10-i1). NOT evidenced: why the approach search from a
house delivers no route (W10-i1's census answers it); the door's part.

### W10-g landed (1ad1870abc, staged 12:35; falsifier red 12:38; the town moves)

Check clean, the pin green, both halves staged 12:35 and in lab-bin
from 12:36; the falsifier (the hold restored) went red and restored
clean at 12:38. b2 fresh on the pair, read at tick 912 (12:37, thirty
seconds in): "job claimed" 21, "arrived at job site" 32, GLIDE HELD 0,
GLIDE INTO A WALL (counted) glides_into_wall=16,384 (a wall on the
next six blocks of the line for about a third of colonist-ticks --
the base rate that the hold turned into a frozen town), claim census
colonists_seen=1 assigned=1, CHASER GLIDE OVERRIDE 1, EMBED WATCH 0.
The two held pairs had 0 and 0 at hour 13. +4 min (hour 11, read
12:41): "job claimed" 173 (bar 40), "arrived at job site" 239 (bar
20 by +10), EXPERIENCE working=8 moving=41 stuck=0 (bar 8 at +10),
TRUNK ROUTE PROFILE routes=1024 detour_ratio=1.63 rejected_solid=407
rejected_dz=438 rejected_crossing=157 (bar: one line), EMBED WATCH 2
(both writer_site="chaser-pure-glide", the class W11 takes at the
wall; bar 12 by day 1), GLIDE INTO A WALL glides_into_wall=65,536,
p95 400 us (bar 700), panics 0. **W10-g PASSED its +4 bars**; the
+10 and day-1 reads (embeds by day, the wall count as the base rate)
follow. The E2-e pair (with this rule) restarts b1 at ~13:05.

+10 min (hour 14, read 12:47): "job claimed" 356, "arrived at job
site" 450, EXPERIENCE working=6 moving=39 stuck=0 idle=4 (the bar of
8 at +10 is NOT met at this one sample; +4 read 8; hour 14 is the
shift's end and the day line's works decide), TRUNK ROUTE PROFILE
routes=3072 detour_ratio=1.71 rejected_solid=1385 rejected_dz=1197
rejected_crossing=429; FIRST LEG GATE routes=512 crossed=74
stitched=70; EMBED WATCH 6 (5 chaser-pure-glide, 1
bridge-refused-rock; idx0 1), STALL BLAMED 5, ROUTE FAULT 2; GLIDE
INTO A WALL 262,144; p95 441 us; panics 0. Six embeds in seven game
hours is on pace past the day-1 bar of 12: the glide-into-a-wall
class is back at its W10-d rate, which is the rate W11 is built to
take at the wall (W11 stages ~14:05). The day-1 read decides the bar.

Day 1 (the day frame, hour 0 of day 1, read 13:04): EMBED WATCH 11
(9 chaser-pure-glide, 2 bridge-refused-rock; idx0 6) against the bar
of 12 and W10-d's 10; "job claimed" 419, "arrived at job site" 735;
TRUNK ROUTE PROFILE routes=7168 detour_ratio=1.83 rejected_solid=3166
rejected_dz=2698 rejected_crossing=1213; FIRST LEG GATE routes=1280
crossed=122 stitched=114; STALL BLAMED 6, ROUTE FAULT 4; GLIDE INTO A
WALL glides_into_wall=524,288 (the base rate: about a fifth of
colonist-ticks over the day have solid on the next six blocks of the
line); EXPERIENCE working=34 moving=7 stuck=0 idle=9; p95 487 us;
starving 1; panics 0. **W10-g PASSED** (the one miss: working 6 at
the +10 sample against 8, with 8 at +4 and 34 at the day line). The
town is back to its W10-d shape with the wall counted; W11 takes the
nine glide embeds at the wall.

## W10-i1, registered 11:30 (keyed on the E2-e stage, the end of the chain, before the binary)

An instrument row. The pump had no witness: W10-e's failure was read
from the held-glide line alone, and the queue's length (47) and a
search's wait (one step in 24 ticks) were inferred, not read.
`PendingSearch.since` (the enqueue tick, stamped at the three enqueue
sites); `PumpCensus` notes every step and every delivery
(`PumpOutcome::{Path, Unreachable, Exhausted}`, wait = tick - since);
at tick % 300 == 17 the PUMP CENSUS line gives pending, oldest_wait,
the deliveries by kind, mean_wait, max_wait, steps, then resets. Pin
`the_pump_census_keeps_its_waits` (fresh reads zero; waits 10 and 30
count two, one of each kind, sum 40, max 30, mean 20); planted: the
maximum not kept, red. Prediction (b2 fresh, W10-f in force): the
census fires 180 times a game day; pending under 20 at the day-0
hour-11 sample; mean_wait under 300 ticks and max_wait under 2,000;
exhausted under a third of the delivered. If max_wait passes a game
hour (2,250) while pending stays above 30, the pump is a queue the
town outgrows and W10-f's hold at a wall is a stall by construction:
the next row gives the pump slices proportional to pending.

### W10-i1 landed (65db6d4cab, staged 13:28)

The first chain refused at 13:04 (its pin anchor was W10-f's comment,
re-stated by W10-g; the dry tree had been validated in the old order
-- memory written); re-anchored, the queue re-validated from HEAD in
the new order, relaunched. Check clean, the pin green, both halves
staged 13:28. The falsifier (the maximum not kept) and the b2 reader
(`wait-w10i1-b2.sh`: the PUMP CENSUS lines at +4, +10, days 1-2) run
from the stage; the W11 chain keys on it. The falsifier (the maximum
not kept) went red and restored clean at 13:32; lab-bin carries the
pair from 13:29.

b2 +10 min (hour 15 of day 0, read 13:40; 50 census lines): the last
PUMP CENSUS pending=27 oldest_wait=95 delivered_path=4
delivered_unreachable=10 delivered_exhausted=0 mean_wait=24
max_wait=170 steps=600 (two a tick); over the run the worst max_wait
442 and the largest pending 47. Bars: mean_wait under 300 MET;
max_wait under 2,000 MET; exhausted under a third MET (none);
pending under 20 NOT met (27-47). **The pump is not slow**: a
search waits a second and a half on average, five seconds at worst;
what it delivers is UNREACHABLE seven times in ten. The hold of
W10-e/W10-f waited, in the end, for answers that said "no route"
-- and the trunk's first leg falls back to the tail when the
approach is unreachable, so the body glides that leg anyway. The
row after W11: W12-i1, an UNREACHABLE APPROACH witness (feet, node0,
whether the feet stand inside a house region, the house's door
count, the search's admission that refused) to name why the exact
search from indoors finds no way out; then the fix at the admission
(the door, the threshold, or the feet cell), not at the pump. Same
read: EMBED WATCH 8 at +10 (4 chaser-pure-glide, 4
bridge-refused-rock), routes=3072, arrived 474, working=8, p95 476.

Day 1 (hour 0 of day 1, read 13:54; 116 census lines): the last
PUMP CENSUS pending=10 oldest_wait=60 delivered_path=0
delivered_unreachable=0 delivered_exhausted=13 mean_wait=58
max_wait=60 -- at night the pump's deliveries are all
BudgetExhausted (the detour lane's tiers, a different population
from the day's fill searches; the reader prints the last line, and
the day's sum is W12-i1's reader's job); over the day the worst
max_wait 442 and the largest pending 47 stand. EMBED WATCH 10 (6
chaser-pure-glide, 4 bridge-refused-rock; idx0 8) -- W10-g's second
replicate under its bar of 12; routes=6144, arrived 637, working=26
at hour 0, p95 700 us (at the bar), starving 2, panics 0.

### W11 landed (af70369a91, staged 13:55)

Check clean, the pin green, both halves staged 13:55. The falsifier
(the movement test inverted) and the b1 reader (`wait-w11-b1.sh`:
after E2-e's night-2 read, a fresh b1; OVERRIDE FAILED AT A WALL, the
override loops' wall-seconds, the eat timeouts, the starving streaks
at +10 and at day 1 hour 12) run from the stage; the E2-f chain keys
on it. The falsifier went red and restored clean at 13:58; lab-bin
carries the pair from 13:55.

b1 +10 min (hour 13 of day 0, boot 14:17, read 14:27): OVERRIDE
FAILED AT A WALL 2 (uid 57 with stuck=7.06 and had_route=true, uid 54
with stuck=1.76 and had_route=false -- the second after three
seconds of pushing with the stall clock at 1.8 s: the clock the
crumbs reset, read rather than inferred); CHASER GLIDE OVERRIDE 44
lines; override loops 12, wall-seconds p50 3.1, p90 13.3, max 27.0,
sum 56, loops over thirty seconds 0 (the E2-d pair at day 1 hour 12:
113, 4.5 / 68.9 / 246.1, sum 2,627, 20); TimedOut releases haul 2,
cook 1, build 4, EAT 0 (E2-d: eat 20); starving EatFrom/Traveling
streaks 0 (18); EMBED WATCH 5; PUMP CENSUS pending=13 mean_wait=28
max_wait=79; working=11 moving=33 stuck=0; arrived 445; panics 0.
Against the bars at +10 (the window runs to day 1 hour 12): p90 13.3
over the bar of 10 and max 27 under 30; sum 56 far under 600; eat
timeouts 0; streaks 0; the failure count 2 is on pace for the low
end of 20-200. The day-1-hour-12 read decides.

**Day 1 hour 12 (the defect's window; read 15:09): W11 FAILED.**
OVERRIDE FAILED AT A WALL failed=128 (in the 20-200 band), but of
the twelve witnessed lines ELEVEN carry had_route=false (uid 943 six
times, uid 54 five, uid 57 once; stuck at failure 1.06-7.06 -- the
crumb-reset clock throughout); CHASER GLIDE OVERRIDE 586 lines;
override loops 82, wall-seconds p50 2.9, p90 27.0 (bar 10), max
246.4 (bar 30 -- one loop as long as the E2-d pair's worst), sum
1,061 (bar 600), loops over thirty seconds 6 (E2-d: 20); TimedOut
releases eat 40, build 18, cook 10 (E2-d pair: eat 20, build 8);
starving EatFrom/Traveling streaks 23, p50 7, p90 25, max 47 (E2-d:
18 / 5 / 18 / 38); EMBED WATCH 10; working=4 moving=34 idle=12 at
hour 12; PUMP CENSUS pending=18 mean_wait=49 max_wait=122; arrived
1,030; panics 0. The mechanism fires by the feet as designed, and
its ACTION is wrong for eleven of twelve: with no committed path the
chaser's head is the rtsim chaser's own route (fetch legs and eat
legs walk that way, `committed.or_else(snap.route_head)`), and
dropping `path_cache` changes nothing, so the same body fails every
three seconds at the same node (uid 943 x6) for as long as before.
The eat timeouts (40 against 20) are not a clean comparison: this
pair carries E2-e's supper traffic and W10-g's glide and the E2-d
pair carried neither; no control with W11 absent was read on this
world. Disposition: the verdict stands, the escape must also reset
the chaser's own route (W11-b, registered below once the chaser's
API is read); the eat-timeout doubling is NOT evidenced as W11's.

### W12-i1 landed (79ff47e087, staged 15:09)

Check clean, the pin green, both halves staged 15:09 and in lab-bin
from 15:10. The falsifier (Boxed at three of four) and the b2 reader
(`wait-w12i1-b2.sh`: after E2-f's second-day read, a fresh b2; the
unreachable split summed over the census lines and the first eight
witnesses at +4, +10, days 1-2) run from the stage; the E2-h chain
keys on it. The falsifier went red and restored clean at 15:12.

b2 +10 min (hour 14 of day 0, boot 15:35, read 15:45; 42 census
lines): summed over the lines delivered_unreachable=9,
unreachable_open=9, unreachable_boxed=0, unreachable_start_solid=0
(the three sum to the total: the instrument agrees with itself);
UNREACHABLE APPROACH 8 witnesses, every one start=Open,
from_in_house=true, first_leg=false (plain fill searches from inside
a house, not the trunk's approach). The last PUMP CENSUS pending=27
oldest_wait=111 mean_wait=99 max_wait=184 (worst 779, pending up to
45). Same read: EMBED WATCH 1, routes=2048, arrived 363, working=12,
p95 508, panics 0. **The embedded-start hypothesis is falsified**
(0 of 9, against the 60% the row predicted): the feet stand in the
open, inside a house, and the exact search still finds no way. The
next question is the house's exit -- whether the search's admission
passes the door cell (a door is a sprite on a block; the admission
may refuse it) -- or the target. W12-i2 reads the door: for an
unreachable approach from inside a house, the door cells of that
house region and whether the search's walkable() admits each.

### W11-b landed (1531be2c65, staged 15:53)

Check clean (the chaser reset compiles at the escape site), the pin
green, both halves staged 15:53 and in lab-bin from 15:53. The
falsifier (the chaser kept) and the b1 reader (`wait-w11b-b1.sh`:
after E2-g's night-2 read, a fresh b1 with the fresh-boot guard; the
loops, the failures with chaser_dropped, the eat timeouts and the
streaks at +10 and day 1 hour 12) run from the stage; the E2-g-b
chain keys on it. The falsifier went red and restored clean at 15:56.

b1 +10 min (hour 14 of day 0, boot 16:30, read 16:40): OVERRIDE
FAILED AT A WALL 2 (uid 30 had_route=true stuck=4.03, uid 15
had_route=false stuck=2.93), chaser_dropped 1; CHASER GLIDE OVERRIDE
61 lines; override loops 12, wall-seconds p50 0.0, p90 14.9, max
35.1, sum 68, loops over thirty seconds 1 (the last eight override
lines carry stuck 10.9 -> 14.9: a push the feet kept moving through,
re-anchored each three seconds, not a failure); TimedOut build 10,
deposit 1, cook 1, haul 1, EAT 0; starving EatFrom/Traveling streaks
0; EMBED WATCH 10 (W11's +10: 5); PUMP CENSUS pending=20
mean_wait=54 max_wait=128; working=11, arrived 560, panics 0. Early;
the day-1-hour-12 read decides.

Two harness faults, named: the reader's restart at 16:30 took the
CURRENT stage-bin (E2-g-b's 4ded49d334, staged 16:15), not W11-b's
1531be2c65 -- the +10 above was read on a superset pair that carries
W11-b, valid for its bars, wrong in its label; and E2-g-b's reader,
bounding its wait for this row's day read at 30 minutes from its own
stage, restarted b1 at 16:46 (day 0 hour 18) on W12-i2's pair
7d28997261, so the day-1-hour-12 read was lost. Recovery: that pair
carries W11-b (E2-g-b changes the supper window, W12-i2 adds a
probe), and `read-w11b-day1.sh` reads the same bars on this run
without a restart at day 1 hour 12 (~17:45). Memory written:
a reader's restart takes the latest staged pair; a wait on another
reader is bounded by its schedule.

**Day 1 hour 12 (recovered on b1's W12-i2 run, boot 16:46, read
17:32): W11-b PARTIAL.** OVERRIDE FAILED AT A WALL failed=32,
chaser_dropped=31 (97%; bar 80: MET); had_route false 7 / true 3 of
the ten witnessed; uid 25 failed 6 times (bar 3: NOT met), uid 39
twice; CHASER GLIDE OVERRIDE 214 lines (W11: 586); override loops 51
(W11: 82; E2-d: 113), wall-seconds p50 1.1, p90 10.0 (bar 10: at
the bar), max 116.1 (bar 30: NOT met; W11 246.4), sum 294 (bar 400:
MET; W11 1,061; E2-d 2,627), loops over thirty seconds 3 (bar 0: NOT
met; W11 6; E2-d 20); TimedOut releases eat 7 (bar 15: MET; W11
40), build 28 (W11 18), haul 7, deposit 4; starving EatFrom/Traveling
streaks 14, p50 5, p90 15 (bar 12: NOT met; W11 25), max 19 (W11
47); EMBED WATCH 18 (W11's day: 10; up); arrived 1,363; PUMP CENSUS
pending=12 mean_wait=49 max_wait=109; working=9; panics 0. The
escape frees most bodies (the sum down 72%, the worst loop down
53%, eat timeouts down 83%) and not the few whose next search puts
the head back in the same wall (three loops past thirty seconds, one
body six failures at one wall): the falsifier's first arm, in
proportion -- the node placement, W12's question, now W12-a's (the
search aims at a stand beside the target, not the target). The
embeds rose to 18 on this pair: the dropped chaser re-searches and
its first leg glides again; W12-a is expected to cut that too, and
the count is carried as its baseline.

### The build lane's 74 blocks a claim is re-claim churn by stalled fetchers (b1 day 2, read 18:08)

JOB SEQUENCE CENSUS on b1 day 2: Build colonists 13, works 171,
mean_travel_blocks_per_claim 74, far_claim_pct 43. The producer:
`note_travel` adds the feet-to-job xy distance at CLAIM time, once
per claim commit. But successive build ARRIVALS by the same colonist
are 2 blocks apart (266 arrivals, 17 colonists, 12 site cells of
8x8; median hop 2.0, p90 8.1, max 21, none over 30). The gap is the
re-claims: build job 1681 (stones) was claim-committed 13 times in
13 minutes by seven colonists (28; 18 five times; 39 three; 57 two;
25; 53), every one "fetch=true carried=false", and each fetch ended
FETCH STALLED (no displacement) -> FETCH BUDGET EXPIRED (15.5 s) ->
RELEASE-DIAG TimedOut -> the same colonist re-claims the same job
within a second, from the same feet, 74 then 146 blocks from the
next item. Whole log: FETCH STALLED 56, BUDGET EXPIRED 45, TARGET
SHUNNED 33, STALL BLAMED ON THE WALKER 5; the stalled feet cluster
at three spots ((7649, 6390) x10, (7748, 6328) x8, (7587, 6502) x8).
Per build job on day 2: 16 claimed once, 8 twice, 3 three times, 2
four, 2 five, one each of 6, 7, 8 and 13. So the 74 is the wedge
class wearing a census: a body that does not move re-claims every
fifteen seconds and each re-claim is a "far claim". W12-a/W12-b are
the rows for the bodies; the re-claim loop itself (no bench after a
TimedOut fetch for the same claimant) is measured below before any
row is built on it.

The measure (b1 whole log to 18:10, RELEASE-DIAG TimedOut of work
classes: build 28, work 2, craft 2): re-claimed within 60 s by the
SAME colonist 19 (median gap 0.7 s, p90 14.4 s), by another 5, not
re-claimed 8. A real loop, a modest count: the bodies' stalls (56)
are the root and W12-a/b come first; a bench for the same claimant
after a TimedOut fetch is the candidate if the loop survives them.

### W12-a landed (64d42eb0cb, staged 18:20)

Check clean, the pin green, both halves staged 18:20 and shipped to
lab-bin 18:21. The chain's symbol-name marker read 0 (release
binaries carry no symbol names); the binary was verified by its
witness string instead: "SEARCH TARGET MOVED TO A STAND" present
once in the staged and the shipped server. The falsifier (the stand
never sought) went RED at 18:24, the tree restored clean. The b2
reader restarted b2 on the pair at 18:24 after E2-j's night-1 read;
+4, +10, day 1 and day 2 follow.

W12-a +4 on b2 (read 18:28, hour 11 of day 0): PUMP delivered_path
267, delivered_unreachable 0, unreachable_open 0 (the W12-i2 pair at
+10: 17 of 17 open); UNREACHABLE APPROACH 0; DOOR PROBE 0; EMBED
WATCH 4 (writers: chaser-hold 1, chaser-pure-glide 2,
chaser-refused-rock 1); STALL BLAMED 0; pump pending max 45, worst
max_wait 344, mean 33; arrivals 359; stuck 0, panics 0. SEARCH
TARGET MOVED fired 2,048 times by +4 -- not the twenty the row
expected: the moved targets are the general store's own cells
((7648..7655, 6354..6355, 182), the seed rows) and the stands are
their neighbours at the same z or one up ((7647, 6353, 183); (7648,
6355, 182); (7649, 6354, 182)...). The store's item cells sit at z
181 (DELIVERED pos) and the searches aim at z 182, one above the
standable cell, which colonist_walkable refuses (no ground under
it) while the old exact search accepted as a goal (it delivered
paths to them before). So the witness counts a convention gap (the
search's target is the item cell plus one) rather than unwalkable
objects; the moves are one cell and harmless, and the outcome bars
hold at +4. The +1 convention is read before anything is built on
it; the +10 and day-1 reads decide the row.

W12-a +10 on b2 (read 18:34, hour 15 of day 0), against the bars:
unreachable_open per +10 under 5 -- 0 (17), PASSED; SEARCH TARGET
MOVED at least 20 -- 4,096 (the convention gap above; W12-a-b
narrows it), PASSED as written and not as meant; FIRST LEG GATE
stitched at least 80% of searched -- 27 of 32 crossed, 84%,
PASSED; delivered_path at least twice delivered_unreachable -- 449
to 1, PASSED; EMBED WATCH by day 1 under 12 -- 12 lines already at
+10 (writers: chaser-pure-glide 7, bridge-refused-rock 3,
chaser-hold 1, chaser-refused-rock 1; idx0 embeds 7), the day-1
read decides and it will not be under 12. Pump: 49 census lines,
pending max 45, worst max_wait 348 ticks, mean 51; arrivals 502;
stuck 0, idle 2, starving 0, panics 0. So the unreachable class is
closed on this arm by the stand rule (with W12-b's corner fix still
to land), and the embed class is NOT the unreachable class: the
bodies embed in the chaser's pure glide with a delivered route. That
is the next wedge row, read from the EMBED WATCH lines below.

W12-a day 1 on b2 (read 18:50, hour 0 of day 1): unreachable 1 of
642 deliveries (delivered_path 641), unreachable_open 0 -- PASSED;
FIRST LEG GATE 73 of 84 crossed, 87% -- PASSED; EMBED WATCH 18 by
day 1 (bar under 12; W11-b's day 18, W10-g's 11) -- FAILED, writers
chaser-pure-glide 12, bridge-refused-rock 4, chaser-hold 1,
chaser-refused-rock 1; GLIDE HELD 0; arrivals 729; stuck 0; panics
0. Disposition: W12-a stands for what it claimed (the unreachable
class is closed on this arm) and the embed bar it also claimed is
FAILED -- the embeds are the pure glide's, W13's row, building now.
New in the last census: delivered_exhausted 38 at hour 0 (the exact
search's budget spent without a verdict) -- read below before it is
named.

## W13, registered 18:43 (keyed on the W12-b stage; ahead of E2-j-b and the rest)

THE GLIDE FOLLOWS THE SURFACE. The EMBED WATCH lines on b2 (W12-a
pair, +10): 8 of 12 written by chaser-pure-glide, 7 of the 8 on a
leg whose route head sits one block lower than the entry (head dz
-1.0, -1.7, -1.0, -0.6, -1.0, -1.0, -1.3), one on a step up (+1.6),
the body wedged 4.2 blocks along the leg with its feet 0.05-0.64
under the entry's z (uid 18: entry (7643.1, 6352.2, 182.0),
embedded (7640.2, 6349.2, 181.4), head (7638, 6347, 181)). The pure
glide's step is the straight 3D line to the phased node (`try_pos =
pos + dir * step`), so the z falls from the first step while the
upper floor still lies under the body until the edge; the probed
glide snaps to `surface_at`, the committed glide never did. Now
`glide_snap_z(try_pos, surface_at(try_pos))`: the floor's z when
known (dz 0, +1, -1, -2 from the current feet), the line's z when
not (identity, no hold); witness THE GLIDE FOLLOWS THE SURFACE
(uid, line_z, floor_z, node, snaps) when the snap moved the z by
more than 0.05. Pin `the_glide_follows_the_surface`; planted: the
floor ignored, red. Prediction (b2 fresh, `wait-w13-b2.sh`, after
W12-b's day-1 read; +10 and day 1): pure-glide embeds at most 2 by
+10 (8) and 4 by day 1; EMBED WATCH by day 1 under 12; GLIDE HELD
0; arrivals at +10 at least 450 (502); stuck 0; snaps at least 50 by
+10. Falsified if the pure-glide embeds stay at 6 or more by +10,
or arrivals fall under 350 or GLIDE HELD rises (the snap holds
bodies at edges), or the snaps stay under 10 (surface_at does not
answer on the glide's cells). Rejected: the probed glide for every
committed route (W10-e/f); a z-clamp to the entry's z; a route-node
rewrite. NOT evidenced: the bridge-refused-rock embeds (3); the
deep-drop arm. The queue is now W12-b, W13, E2-j-b, E2-l, E2-k,
E2-i2, W12-a-b; the E2-j-b chain and reader were re-keyed behind
W13 and the dry tree rebuilt in that order.

### W12-b landed (02fdf9d782, staged 18:43)

Check clean, the pin green, both halves staged 18:43; the binary
verified by its witness string ("DROP CELL SPREAD FROM THE CENTRE
OUT" present once, W12-a's string too), shipped to lab-bin 18:44.
The falsifier (the distance ignored, the corner first) went RED at
18:47, the tree restored clean. The b2 reader restarts b2 after
W12-a's day-1 read; W13's chain fires five minutes after this stage.

### W12-b FAILED LIVE at boot on b2 (pair 02fdf9d782, read 18:53): every deposit at the centre

The 25 founding deliveries all at (7665, 6365, 181), the general
store's centre; STORAGE SUMMARY day 0 general_units 12,028,
general_max_cell 12,022 (the pair before: 200); the day's DepositRun
job_pos 28 at (7665, 6365), 15 at (7791, 6365), 6 at (7743, 6143),
5 at (7698, 6446) -- one cell per store, each its centre. DROP CELL
SPREAD FROM THE CENTRE OUT never fired (it prints only when the
chosen cell is not the centre): the centre came from the identity
arm, because the standable filter admitted NO surface cell of any
store at runtime. The pin proved the picker with a closure that
answers true; the live predicate answers false for cells that are
air over grass by every reading of `walkable`, and the log cannot
say why. The pair is NOT PLAYABLE (the whole larder in one cell);
lab-bin carries it from 18:44 until W12-b-b ships. The corner,
meanwhile, is gone (its tie rule is gone).

W12-b +10 on b2 (read 19:01, hour 15 of day 0), for the record: the
corner 0 (8 of 8 before W12-a), unreachable 0 of 332, DROP CELL
SPREAD 0 lines (the identity arm, as diagnosed), EMBED WATCH 4 (all
chaser-pure-glide; W12-a's +10 read 12 -- every deposit walking to
one centre cell makes fewer varied legs), FIRST LEG GATE 23 of 27,
arrivals 437, stuck 0, panics 0. The pair's wedge numbers are sound;
its store is one pile. Day 1 (read 19:15, hour 0 of day 1): the
corner 0, unreachable 0 of 482, EMBED WATCH 9 (all chaser-pure-
glide; W12-a's day 18), STALL BLAMED 5, FIRST LEG GATE 44 of 50,
arrivals 675, stuck 0, starving 3 at midnight, panics 0; the store's
biggest cell 12,346 units at midnight (12,022 at boot).

## W12-b-b, registered 18:56 (keyed on the W13 stage; ahead of E2-j-b and the rest)

A FILTER THAT EMPTIES THE STORE IS VOID. The surface cells are
collected first; the standable filter is applied second; when it
empties a non-empty set it is void (the spread runs over the surface
cells) and DROP CELL FILTER EMPTIED THE STORE (region_min, surface,
standable, voids) names it; otherwise the standable cells. The
nearest-to-centre rule stands either way. Pin
`a_filter_that_empties_the_store_is_void` (admitting nothing: the
unfiltered spread; admitting one cell: that cell; no surface: the
centre); the W12-b pin's "nothing standable: the centre" re-stated;
planted: the fallback removed, red. Prediction (b2 fresh,
`wait-w12bb-b2.sh`, after W13's +10 read; +10 and day 1): distinct
DepositRun job_pos at least 8 by day 1 (4); general_max_cell under
600 at day 1 (12,022); founding delivery cells at least 15 (1); the
void witness fires and its counts say whether the predicate refuses
the store's cells; the corner 0. Falsified if the deposits still
pile on one cell, or the corner returns. Rejected: dropping the
filter; a lenient walkable; guessing the predicate's reason. NOT
evidenced: why the predicate refuses the cells (this pair's witness
counts). The E2-j-b chain and reader were re-keyed behind this row.

### W13 landed (21edda27ad, staged 19:06)

Check clean, the pin green, both halves staged 19:06; the binary
verified by its witness string ("THE GLIDE FOLLOWS THE SURFACE"
present once), shipped to lab-bin 19:07. The pair inherits W12-b's
one-cell store (W12-b-b follows five minutes behind). The falsifier
(the floor ignored) went RED at 19:10, the tree restored clean. Two
replicates: the b2 reader restarts b2 after W12-b's day-1 read; a
b1 reader restarts the 160-day arm after E2-g-c's night-2 read and
reads +4, +10 and day 1 until E2-l's stage takes b1 back.

W13 +10 on b2 (read 19:27, hour 13 of day 0), against the bars:
pure-glide embeds at most 2 -- 1 (8), PASSED; EMBED WATCH 2 in all
(chaser-hold 1); GLIDE HELD 0 -- PASSED; snaps at least 50 -- 16,384
(every step snaps), PASSED; stuck 0 -- PASSED; arrivals at least 450
-- 301 (W12-a 502, W12-b 437): the clause "arrivals under 350 (the
snap holds bodies at edges)" FIRED as written. But the pair carries
W12-b's one-cell store, and with every haul's anchor the same cell
the anchor queue (E2-m's finding: any nearer body holds me) queues
the haulers behind each other -- the pump on this read waited
twice as long (mean 104 ticks, max 253; W12-b: 51, 77) with 33
pending, which is a queue at the pile, not a body held at a step.
So the arrivals clause is CONFOUNDED by a defect this row does not
own; it is not excused. The deciding replicates: b1's W13 read
(the E2-g-c world, the same pile) and, above all, W12-b-b's b2 read
(the same snap with the store spread): arrivals at +10 at least
450 there, or W13's snap is the cost and comes out. For b1 the
baseline (the three previous b1 runs, arrivals within 4 and 10
minutes of the first log line): 335 and 562 (E2-g-c), 274 and 617
(E2-g-b), 209 and 527 (the run before); W13 on b1 at +4 read 221 --
inside that range -- with EMBED WATCH 1 and snaps 2,048; its +10
follows. (Both W13 replicates carry the one-cell store; only the
W12-b-b read is clean of it.)

W13 +10 on b1 (read 19:28, hour 12 of day 0): pure-glide embeds 5
(bar at most 2) -- FAILED on this replicate; EMBED WATCH 5 in all,
STALL BLAMED 1; snaps 16,384; GLIDE HELD 0; stuck 1; arrivals 471
(baseline 527-617, the pile aboard); unreachable 0 of 245. The five
embeds have a NEW signature: entry z 181.96 -> embedded 181.996,
181.09 -> 181.01, 180.09 -> 180.003 (three at the store rows, (7687,
6353) and (7649..7654, 6417)): the body snapped onto an integer z
and the watch read it as inside the block below. Together with
W12-b-b's void witness (surface 752, standable 0 for the general
store) this says the store's cells at the picked z are not air:
the drop-cell surface (`column_surface_z`, which deliberately
resolves through Wood) sits one block UNDER a wooden floor, so
every item, target and snapped body at z 181 is inside the plank.
Read below (the probe's block map) before it is named.

## W15-i1, registered 19:07 (keyed on the W12-a-b stage, the end of both queues)

THE PUMP NAMES THE EXHAUSTED SEARCH. On the E2-j pair (b2) the
pump's exhausted deliveries by hour ran 32, 76, -, 133, 101, 155,
169, 155 at hours 16-23 and 119, 158, 154, 225, 27 at hours 0-4,
against 4-19 an hour in the Work block; the W12-a pair the same
shape (761 in a day, 664 at hours 21-23). An exhausted fill search
is dropped silently and the body keeps the trunk's tail. The night
is when the town goes home: every added shelf on b2 is on the
bedroom storey, worldgen puts the bedroom upstairs, and the router
cannot climb a worldgen staircase (a `Primitive::Ramp` of rise
`storey` under the floor above). So the night's exhaustions are
most likely the routes to bed and to the night shelf, five blocks
up -- the glide through the house being how anyone gets to bed --
but the census cannot say so. Now PumpCensus splits exhausted
deliveries into up, down and flat (EXHAUST_BAND_DZ 3) and THE SEARCH
EXHAUSTED (uid, from, to, dz) names the first of them. Pin
`the_pump_names_the_exhausted_search`; planted: every exhaustion
flat, red. Prediction (b1 fresh, `wait-w15i1-b1.sh`, after E2-i2's
night-1 read; night 1): exhausted_up at least 60% of the night's
exhausted at hours 21-3, with the first witnesses' targets bed and
shelf cells five blocks above their starts; falsified as a
hypothesis if flat is the majority (the curfew's routes on the
flat, not the stairs). Rejected: routing the stairs before
counting; a longer budget. NOT evidenced: the router's stair
admission (W15, if the count says so); the day's exhaustions.

### W12-b-b landed (e27cb07886, staged 19:29)

Check clean, the pin green, both halves staged 19:29; the binary
verified by its witness string ("DROP CELL FILTER EMPTIED THE
STORE" present once), shipped to lab-bin 19:30. The falsifier (the
fallback removed) went RED at 19:32, the tree restored clean. The
b2 reader restarted b2 at 19:31 after W13's +10 read and reads +4,
+10, day 1 -- the deciding replicate for W13's arrivals bar and
this row's own.

### W12-b-b at boot on b2 (pair e27cb07886, read 19:33): the store spreads again, and the void is named

The founding deliveries spread from the centre out ((7665, 6364),
(7666, 6364), (7667, 6365)...); STORAGE SUMMARY day 0 general_units
12,200, general_max_cell 128 (W12-b: 12,022; the older pairs 200).
DROP CELL FILTER EMPTIED THE STORE fired on every call for the
general store: region min (7642, 6342, 178), surface 752, standable
0 -- no cell of the store passes colonist_walkable at surface + 1.
The row's own bars are read at +10 and day 1; the void's cause is
W12-c's row.

W12-b-b +4 on b2 (read 19:35, hour 11 of day 0): FILTER VOIDS 15
(every call, surface 752, standable 0 -- the fallback carried every
pick); DROP CELL SPREAD 15 lines, dist2 1, 1, 1, 1, 2, 2, 2, 2, 5,
10, 20, 40, 80, 169 (from the centre out, as pinned); founding
delivery cells 24 distinct (1 on W12-b); DepositRun job_pos 50
distinct (4); general_max_cell 128 (12,022); EMBED WATCH 0 (W12-a
at +4: 4); unreachable 0 of 173; arrivals 256 (W12-a's +4 on b2:
359); pump pending 32, mean wait 73, max 236 (W12-a's +4: 33, 85);
stuck 0, panics 0. The store spreads and the corner stays gone; the
pump has been slower on every pair since W12-b (W12-b-b's +10 and
W12-c's read say whether the plank floor is that too).

## W12-c, registered 19:36 (keyed on the E2-j-b stage; ahead of E2-l and the rest)

THE DROP CELL STANDS ON THE FLOOR, NOT UNDER IT. `column_surface_z`
resolves through Wood by design (a roof over a house), so under a
plank floor it finds the ground beneath the floor and surface + 1
is the plank: items delivered at z 181, targets accepted at 182,
colonist_walkable false at 181 for all 752 cells, W13's b1 embeds
at z 181.96 in the store rows. Now the drop cell is the first
standable cell above the surface (up to DROP_CELL_LIFT_MAX 6),
surface + 1 when none is; the centre lifts the same way; DROP CELLS
LIFTED ONTO THE FLOOR (region_min, centre, lifted, surface, lifts).
Pin `the_drop_cell_stands_on_the_floor` (a plank lifts the cell to
surface + 2; bare ground surface + 1; nothing standable surface +
1); planted: the lift ignored, red. Prediction (b1 fresh,
`wait-w12c-b1.sh`, after W13's b1 day-1 read; +10 and day 1): the
void witness 0; lifted about 752; DELIVERED z 182 in the general
store; DepositRun job_pos z 182; embeds at the store rows 0 and
under 8 in all by day 1; general_max_cell under 600. Falsified if
the void still fires for the general store, or the delivered z
stays 181, or the store-row embeds persist with items at 182.
Rejected: stopping the surface at Wood (a roof again); a per-zone
offset; a walkable-only scan from the top. NOT evidenced: the four
other stores' floors; the haul re-aim's cells. The E2-l chain and
reader were re-keyed behind this row; the dry tree was rebuilt in
the order E2-j-b, W12-c, E2-l, E2-m, E2-k, E2-i2, W12-a-b, W15-i1.

## W12-a-b, registered 18:32 (keyed on the E2-i2 stage, the end of the queue)

THE ON-TOP TARGET STANDS. `approach_target(job_pos, stance)` for an
Untargeted job is the cell plus (0, 0, 1), the classic on-top
target, one above the standable cell; colonist_walkable refuses a
cell with no ground beneath, so W12-a moved every pickup's target
one cell sideways (2,048 by +4). Now search_stand returns the target
when it or the cell below is standable; the rings run only for a
target whose own column cannot be stood on. Pin re-stated (one below
stands: the target); planted: the cell below ignored, red.
Prediction (b2 fresh, `wait-w12ab-b2.sh`, after E2-j-b's night-1
read; +10): SEARCH TARGET MOVED under 40 (2,048 by +4); unreachable
open 0-3; delivered_path at least ten times delivered_unreachable;
EMBED WATCH by day 1 under 12. Falsified if the witness stays in the
thousands, or if unreachable_open returns above 5 (the sideways
move was doing the corner's work; W12-b's read decides). Rejected:
colonist_walkable accepting an on-top cell; moving the target down
one. NOT evidenced: day 1 on the pair; the corner after W12-b.

## W12-b, registered 17:50 (keyed on the W12-a stage, the end of the chain, before the binary)

The shared unreachable target (7672, 6426, 182) is the minimum
corner of stockpile zone 61, region min (7672, 6426, 178) max
(7725, 6467, 184), a 54 x 42 general store. Every deposit run's pos
is `stockpile_drop_cell_spread`: the centre when it is among the
least filled, else the FIRST least-filled cell in (y, x) order --
the corner; a claimed run counts 16 at its cell, so the centre is
full after one claim and every later run of the day aims at the
corner, 33 blocks off the centre and not a cell the A* admits. The
pin `private_shelves_belong_to_households_and_drops_spread_out`
blessed the corner by name ("first row-major"). Now a candidate must
be standable (the callers pass colonist_walkable on the live
terrain); among the least filled the nearest to the centre wins
(squared xy distance, ties by (y, x)); the centre when it is among
them; nothing standable leaves the centre. Witness DROP CELL SPREAD
FROM THE CENTRE OUT (centre, cell, dist2, candidates) at the first
eight and powers of two; five call sites. Pin
`the_drop_cell_spreads_from_the_centre_out` (5x5, centre full: (2,
1); never the corner while a nearer cell is free; the nearest
blocked: (1, 2); nothing standable: the centre) and the old pin
re-stated (3x3, centre full: (1, 0)); planted: the distance
ignored, red. Prediction (b2 fresh, `wait-w12b-b2.sh`, after W12-a's
day-1 read): unreachable targets at the corner 0 (8 of 8; 8 of 11
on b1); every printed dist2 at most 50 (the corner's 1,089);
unreachable_open per +10 at most 3; W12-a's SEARCH TARGET MOVED for
deposit targets 0 on this pair; delivered_path at least four times
delivered_unreachable over the day; STUCK on deposit runs 0.
Falsified if the corner stays a target, or if unreachable_open
stays above 5 with the targets within 7 of the centre (the path
between), or if a haul's delivered count falls (the standable
filter refusing the store's own cells). Rejected: a ring search
from the corner (W12-a's rule covers the search, not the
destination; the corner is wrong even when walkable); an occupancy
cap per cell; a random cell. NOT evidenced: private shelves (1x1,
identity); the spawn-side deposit cells' fetch reach; the NoDoor
probes (W12-i3).

### The two NoDoor probes explained (b2 read 17:43, the W12-i2 pair)

The six probes were two bodies: uid 32 four times from the house at
min (7636, 6456, 180), its door found at (7636, 6460, 181) and
reached; uid 825 three times from the house at min (7582, 6498, 180),
doors 0. Uid 825's feet were at z 186 -- an upstairs bed (SHELF ADDED
names the bed at (7590, 6502, 186)) in a house whose floor is at 180
-- and the probe scans z from one below the feet to two above, so
the ground-storey door at about 181 lies outside its window. NoDoor
is the probe's window, not a doorless house. Both bodies' targets
were the shared cell (7672, 6426, 182). Candidate W12-i3: scan the
house's whole z range (region min z to feet plus two); not built
until W12-a's read says the path between is the defect.

## W12-a, registered 17:36 (keyed on the E2-g-c stage, the end of the chain, before the binary)

`search_stand(target, standable)`: the target cell when
colonist_walkable admits it, else the nearest walkable cell within
SEARCH_STAND_REACH (3) by rings and z 0, +1, -1, +2, -2, else the
target as before; the plain fill enqueue aims at that stand (the
trunk's node-0 approach is a road tile, untouched); SEARCH TARGET
MOVED TO A STAND (uid, target, stand, moved) at the first eight and
powers of two. The prior art is this file's `trade_mission_pos`.
Pin `the_search_aims_at_the_stand_not_the_stone` (a standable target
stands; one east; two south one up in the second ring; nothing
within reach leaves the target); planted: the stand never sought,
red. Prediction (b2 fresh, `wait-w12a-b2.sh`, +10 and day 1):
unreachable_open per +10 under 5 (17); SEARCH TARGET MOVED at least
20 by +10; FIRST LEG GATE stitched at least 80% of searched; PUMP
CENSUS delivered_path at least twice delivered_unreachable over the
day; EMBED WATCH by day 1 under 12 (W11-b's day: 18; W10-g's: 11);
W11-b's loops over thirty seconds 0 on the next b1 read. Falsified
if unreachable_open stays above 10 with the witness firing (the
path between, not the target), or if the witness stays at zero
(the shared target cell (7672, 6426, 182) is walkable and the
doorstep is the defect), or if EMBED WATCH rises. Rejected: widening
the admission to bed and item cells; a stance offset per job kind;
reach 8 like the mission's. NOT evidenced: the two NoDoor houses;
the detour lane's targets.

## W12-i2, registered 15:55 (keyed on the E2-g-b stage, the end of the chain, before the binary)

The door is not solid to the search: `walkable` tests `is_solid()`,
which is `solid_height().is_some()` for a sprite block, and the door
sprites carry none (the index's `conn_passable` pin already says a
door is a way through). So the exit fails at the doorstep, the floor
or the target, and nothing says which. Mechanism: at a witnessed
unreachable approach from inside a Bed-designated region, the
region's door sprites (Door, DoorDark, DoorWide at the feet's
z-1..z+2) are scanned; a second `bastion_full_path_ext` runs from
the delivery's own start and cfg to the nearest door cell;
`door_probe_verdict(door_found, door_reached)` -> NoDoor /
SealedInside / ExitOpenTargetUnreachable; UNREACHABLE APPROACH DOOR
PROBE carries the door cell and `colonist_walkable` at, below and
above it. Pin `the_unreachable_approach_probes_its_door`; planted:
the sealed and the open swapped, red. Prediction (b2 fresh,
`wait-w12i2-b2.sh`, +10 and day 1): the probe fires on at least 6 of
the first 8 witnesses; doors >= 1 on every probed house;
SealedInside on at least 5 of 8 (the doorstep) else
ExitOpenTargetUnreachable on at least 5 (the target, node placement
outside); NoDoor dominating means the region, not the search.
Rejected: changing the admission before the probe has spoken; probing
every unreachable. NOT evidenced: which neighbour rule refuses the
doorstep; the probe's cost.

### W12-i2 landed (7d28997261, staged 16:39)

Check clean (the door probe and the second search compile), the pin
green, both halves staged 16:39. The falsifier (the sealed and the
open swapped) and the b2 reader (`wait-w12i2-b2.sh`: after E2-h's
night-2 read, a fresh b2; the door-probe verdicts and the walkable
triplets at +4, +10, days 1-2) run from the stage; the E2-i1 chain
keys on it. The falsifier went red and restored clean at 16:43;
lab-bin carries the pair from 16:40.

b2 +10 min (hour 14 of day 0, boot 17:04, read 17:14): UNREACHABLE
APPROACH 15 witnesses, all start=Open from_in_house=true
first_leg=false; the census sum delivered_unreachable=17 =
unreachable_open 17 (boxed 0, solid 0) -- W12-i1's finding holds on
a second boot. DOOR PROBE 6 lines: **ExitOpenTargetUnreachable 4,
NoDoor 2**; doors per probed house 2, 2, 2, 2, 0, 0; at every found
door colonist_walkable(at)=true, below=false, above=false (the door
cell walks; the cells under and over it are floor and air over
nothing, as they should be). Prediction: "SealedInside on at least 5
of 8" FALSIFIED (0 of 6); "ExitOpenTargetUnreachable on at least 5
of 8" met in proportion (4 of 6, two-thirds) with the other two
NoDoor (a house region with no door sprite at the feet's storey --
the region or the storey, its own question). The way out of the
house is fine; the search fails at its TARGET. The fill search aims
at the job's own cell (`ps.target` is the raw target, the arrive
owning the last leg), and a mine face, a tree, a build site or a
shelf cell is not a walkable cell: a search to it is Unreachable by
construction. W12-a (next): the search aims at a STAND cell -- the
nearest walkable cell beside the target -- not the target; the
probe's job kinds on b2's current run (which carries the witness)
are read first to confirm the targets are work cells. Same read:
EMBED WATCH 9 (4 chaser-pure-glide, 2 chaser-refused-rock, 2
bridge-refused-rock, 1 chaser-settle), routes=3072, arrived 452,
working=7, p95 435, panics 0.

The probe with job kinds (b2 on the E2-i1 pair, which carries the
witness; hour 21 of day 0, 17:34): DOOR PROBE ExitOpenTargetUnreachable
4, NoDoor 3; the eight unreachable approaches all aim at ONE cell,
(7672, 6426, 182), for uid 32 and uid 825 -- six with no job claimed
before the line (the boot's minted deposit runs), one on a Cook job,
one on a Farm designation. A shared destination inside the town's
bounds: a store cell by its place. W12-a's witness will say whether
that cell is walkable: if the stand rule moves the target, the cell
was the reason; if the witness stays at zero and the searches stay
unreachable, the path between the house and the store (a step, a
threshold) is the defect and the row is falsified as written.

## W11-b, registered 15:15 (keyed on the E2-h stage, the end of the chain, before the binary)

`wall_escape_targets(had_route) -> (had_route, true)`: on Failed the
committed route goes when there is one, and the chaser's own route
goes always (`Chaser::drop_route`, which by the W2c rule escalates
the next search's tier); CHASER_ROUTES_DROPPED_AT_WALL counted and
carried on the witness as chaser_dropped. The prior art is in this
file: the W2 stall arm drops the chaser's route the same way. Pin
`the_wall_escape_drops_the_chasers_own_route`; planted: the chaser
kept, red. Prediction (b1 fresh, `wait-w11b-b1.sh`, day 1 hour 12):
loops p90 under 10 s and max under 30 s (27.0 / 246.4), sum under
400 (1,061), loops over thirty seconds 0 (6), a body failing at most
3 times (6), chaser_dropped at least 80% of failed, TimedOut eat
under 15 (40 on this pair, its own number to beat), streaks p90
under 12 (25). Falsified if the loops persist with chaser_dropped
high (the next search puts the head back in the same wall: the node
placement is the defect, W12's question) or the failures triple.
Rejected: dropping the chaser on every override tick; banning the
wall cell before the tier has been tried (the W2 bans are the row
after this one). NOT evidenced: the eat timeouts' own cause on this
pair; a control read with W11 absent.

## W12-i1, registered 13:45 (keyed on the E2-g stage, the end of the chain, before the binary)

An instrument row from W10-i1's census (seven of ten deliveries
Unreachable, the pump fast). At the Fill+Unreachable delivery,
`start_condition(feet_solid, four neighbours solid at z or z+1)` ->
InSolid / Boxed / Open, counted into the PUMP CENSUS
(unreachable_start_solid, unreachable_boxed, unreachable_open);
UNREACHABLE APPROACH (uid, from, to, first_leg, start, from_in_house,
to_in_house against the Bed-designated regions) at the first eight
and powers of two. Pin `the_unreachable_approach_names_its_start` (in
a wall is InSolid whatever the neighbours; four solid is Boxed; three
is Open); planted: Boxed at three of four, red. Prediction (b2 fresh,
`wait-w12i1-b2.sh`, +10 and day 1): the three counts sum to
delivered_unreachable on every line; InSolid + Boxed at least 60% of
the unreachable (the embedded start, W11's class); from_in_house on
at least half of the first eight witnesses. If Open passes 70%, the
TARGET is the defect and the next row is the route's node placement,
not the feet. Rejected: more pump slices (the census says enough);
logging every unreachable. NOT evidenced: a closed doorway against a
wall (a Boxed start inside a house does not tell them apart).

## W11, registered 11:45 (keyed on the W10-i1 stage, the end of the chain, before the binary)

The E2 "day trip" class named. b1 on the E2-d pair (3765c74e87),
day 0 to day 1 hour 12, read 11:30: 1,637 CHASER GLIDE OVERRIDE
lines in 113 loops (one uid at one node, lines a second apart);
wall-seconds p50 4.5, p90 68.9, max 246.1 (uid 30), sum 2,627;
twenty loops over thirty seconds. uid 77, a miner walking to eat,
146 s at node (7807,6343,183) from feet (7807.5,6341.95,183), a
solid between, then RELEASE-DIAG class="eat" reason=TimedOut.
TimedOut by class: eat 20, build 8, recreate 8, cook 4, haul 3.
Starving EatFrom/Traveling streaks (300-tick samples): 18, p50 5,
p90 18, max 38 (uid 77). The override glides a refused node by
ruling; the mover writes the feet into the wall each tick and physics
ejects them; the velocity reads as walking (EXPERIENCE stuck=0 at
every sample with ten bodies looping); no assist applies (a lateral
cell, not a hop); the clock the ten-second machinery reads is fed the
same crumbs. Mechanism: `board.override_anchor` (uid -> node, feet,
tick); `override_verdict`: no anchor / another node / stale (600) ->
Anchor; younger than 90 ticks -> Pushing; feet within 0.5 -> FAILED;
else re-anchor. On Failed the route is dropped (the rebuild from the
feet sends the crossing approach to the pump, W10-a), nothing is
pushed that tick, OVERRIDE FAILED AT A WALL (uid, node, feet, secs,
stuck, strikes, had_route, failed). The override witness carries
stuck and strikes. Pin `the_override_fails_at_a_wall_by_its_feet`;
planted: the movement test inverted, red. Prediction (b1 fresh, the
same window): failures 20-200; loops p90 under 10 s and max under
30 s; wall-seconds under 600; TimedOut eat under 6; streaks p90 under
8; working at least 8 at day 1 hour 8; EMBED WATCH not above 12.
Falsified if one body fails more than twenty times at one node (the
node is the route's defect) or the failures pass 500 (the threshold
fires on honest slow pushes). Rejected: refusing crossing legs at the
route (W10-d), disabling the override, keying on stuck_time, a
lateral assist (row 49's rooftops). NOT evidenced: whose nodes the
walls are (2 of 1,024 trunk routes cross; the chaser's own corner cut
is the next suspect).

## W9-i3, registered 10:30 (keyed on the W10-e stage, the end of the chain, before the binary)

An instrument row. `height_context(indoors, builder)` -> Indoors
(feet inside a Bed-designated house) | Builder (the Build lane,
outdoors) | Other; the BODY HEIGHT CENSUS carries two_plus_indoors,
two_plus_builders and two_plus_other beside the total. Pin
`the_height_census_names_its_context` (a builder at home is indoors);
planted defect: the lane read before the house, red. Prediction (b2
fresh, day 1): two_plus_other under 1% of the roster at every line;
builders carry the work-day peaks and indoors the evening ones; the
two explain at least 80% of the total. Falsified if two_plus_other
holds above 2% -- bodies really stand on walls and the mover rows have
a number.

## W9-i2, registered 04:40 (queued at the end of the chain, before the binary)

An instrument row. `height_class(feet_z, surface_z)`: 0 on the ground
(feet on the block above the surface, or below it), 1 a step up, 2 two
or more above (a wall top, a fence, a roof, a pile). Every 300 ticks a
BODY HEIGHT CENSUS counts the colonist join by that class against
`column_surface_z` under each body; unseen columns are counted, never
folded in. Pin: `a_height_is_measured_from_its_own_ground`; planted
defect: measured from the surface block itself, red.

Prediction (b2 fresh on the pair): two_plus_above under 3% at every
census after boot +4 min, unseen 0. Falsified if two_plus_above holds
above 5% -- then bodies really do stand on walls and the next mover
row is a route row.

## W10-b and W10-a, registered 02:20 (queued behind R3, before the binaries)

W10-b (`fix-w10b.py`): `TRUNK_REJECT_DZ` 6 -> 2 through
`trunk_step_walkable(worst_dz)`, pinned (`a_trunk_segment_drops_at_most_two`;
planted: the limit back at six). Bars on b2's day 1 against the W9
day: terrace-edge stalls 0 (W9: 7 of 7), EMBED WATCH under 40, TRUNK
REJECTED (dz) up from 21 a day with the tick p95 still sub-millisecond,
works not below the W9-b day. Falsified if EMBED climbs or the (dz)
rejections reach the thousands.

W10-a (`fix-w10a.py`): `first_leg_needs_search(feet, node0)` (over a
tile, 6 blocks xy) holds the trunk as `board.trunk_tail` and sends a
Small exact search feet -> node 0 to the pump; at completion
`stitch_first_leg` joins the approach onto the tail (the shared node
once) and commits it (`FIRST LEG STITCHED`); an unreachable approach
commits the tail alone (identity). A tail is stitched only onto the
search made for it (node 0 matched). Pinned
(`the_first_leg_is_searched_and_stitched`; planted: the approach dropped
at the stitch). Bars: FIRST LEG STITCHED fires with searched ~=
stitched + unreachable; committed drops at the warehouse wall under
100 a day (W9: 2,048+); idx-0 embeds under 5 a day; a restored boot's
first ten minutes under 5 embeds (R2: 12 in four). Falsified if
unreachable dominates or the tick p95 leaves the sub-millisecond band.

b1's day 1 (the 160-day arm, read 01:55): EMBED WATCH 12 against its
W8-f control's 1,631 by day 1, the two cells 0 / 0, no (uid, head)
pair above 1, two settlers arrived and walked in (trap 1 was their
route); trunk lifted 7,023, rejected 4,096+, committed drops 1,024+,
p95 734 us; food_stock 3,944 against food_anywhere 3,953; meals 47,
shunned 13, no_food_found 0, starving 1 at the last census. cooked
60 misses the carried bar (control 89): the same work drop as b2's
lane census, W9-b's row. Registered before
that read: W9-b (`fix-w9b.py`, queued behind R3) moves a wall-bound
node to the first standable neighbouring column before rejecting;
bars: rejections under 100 per ten minutes, sidesteps in the hundreds,
embeds still under 40 a day, day-1 works within 20% of 550.
