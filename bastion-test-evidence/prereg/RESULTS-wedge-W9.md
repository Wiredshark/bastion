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
