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

W12-b-b +10 on b2 (read 19:41, hour 14 of day 0): FILTER VOIDS 15
(every call); DROP CELL SPREAD from the centre out (dist2 1..169);
founding cells 24, DepositRun job_pos 51 distinct; general_max_cell
128; EMBED WATCH 0 (W12-a's +10: 12; W12-b's: 4; W13's: 2); GLIDE
HELD 0; snaps 32,768; unreachable 3 of 342 (open 2; two NoDoor
probes, the upstairs window); FIRST LEG GATE 7 of 8; arrivals 333;
pump pending 34, mean wait 105, max 225; stuck 0, panics 0. So the
row's own bars hold at +10 (the store spread, the corner gone, the
pile gone) with the void still fired every call (W12-c's).

THE DECIDER FOR W13's ARRIVALS BAR: 333 with the store spread. The
arrivals at +10 on b2 by pair: W12-a 502, W12-b 437 (the pile),
W13 301 (pile and snap), W12-b-b 333 (spread and snap); b1's W13
471 against a 527-617 baseline. The pile is not the cause; the snap
is. W13's clause ("arrivals under 350: the snap holds bodies at
edges") FIRED on both b2 replicates. The likely mechanism, read
next: the arrive test measures the body against the on-top target
(cell + 1, one above the standable cell), which the line's z used
to float the body up to; the snapped body stays on the floor, one
block under the target, and does not arrive. If so W13 is right
about the walk and wrong about the arrive, and the companion row
(the arrival is measured on the floor) goes in before W13 stays.

W12-b-b day 1 on b2 (read 19:58 at hour 0 of day 1; the reader's
block was cut at its sixth line by the E2-j-b reader's takeover,
the rest recovered from the rolled log): the row's bars -- DepositRun
job_pos 59 distinct (bar at least 8), general_max_cell 380 (under
600), founding cells 24 (at least 15), the corner 0, the void
witness firing every call with surface 752 / standable 0 -- ALL
PASSED; W12-b-b stands. The day beside it: arrivals 497 (W12-a's b2
day 1: 729; W12-b's: 675; W13's b1 day: 772), EMBED WATCH 0 all
day (W12-a: 18), STALL BLAMED 4, unreachable 6, exhausted 677 (W12-a
761), works 147 (the day-0 line; W12-a's b2 day-0 works 252), no
panic. So on b2 the snap's arrivals cost holds at the day scale
(-32%) while the embeds are gone; W13-b's read decides whether the
lift alone keeps the embeds off without the cost.

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

W13 day 1 on b1 (read 19:51, hour 0 of day 1): EMBED WATCH 9 (bar
under 12; W11-b's day 18, W10-g's 11) -- PASSED, all chaser-pure-
glide, most at the store rows' z (181.96, 180.0x: the plank, W12-c);
STALL BLAMED 5, ROUTE FAULT 4; arrivals 772 by day 1 (W12-a's b2
day 1: 729) -- the +10 deficit closed over the day on this arm;
unreachable 0 of 447; FIRST LEG GATE 119 of 128; snaps 65,536;
stuck 0, starving 3 at midnight, panics 0; the day's works 446
(E2-g-c's 444). So on b1 W13 passes its day bars and the arrivals
question is left to W12-b-b's b2 day 1 and W13-b's read.

### W12-c landed (8f1bd71ae6, staged 20:15)

Check clean, the pin green, both halves staged 20:15; the binary
verified by its witness string ("DROP CELLS LIFTED ONTO THE FLOOR"
present once), shipped to lab-bin 20:15. The falsifier (the lift
ignored) went RED at 20:18, the tree restored clean. The b1 reader
restarted the 160-day arm at 20:16; at boot the founding deliveries
landed at z 182 (every previous pair: 181) and the store's biggest
cell read 144. +4, +10 and day 1 follow; W13-b's chain fires five
minutes after this stage.

W12-c +4 on b1 (read 20:20, hour 11 of day 0), against the bars:
the void witness 0 (every call before) -- PASSED; DROP CELLS LIFTED
15 lines, the general store lifted 698 of 1,005 surface cells (the
rise cap admits more cells once lifted; the 307 unlifted are bare
ground at the edges) -- PASSED; DELIVERED z 182 on all 25 founding
cells (181 before) -- PASSED; DepositRun job_pos z 182 on all 69 --
PASSED; embeds at the store rows 0, EMBED WATCH 0 -- PASSED;
general_max_cell 144 -- PASSED. Beside them: the pump's mean wait
41 ticks (W12-b-b's +4: 73; W12-a's: 33) and pending 14 -- the
plank was the pump's cost; arrivals 279 (b1 baseline 209-335 at
+4); unreachable 0 of 165; stuck 0; panics 0. The search targets in
the store now read z 183 (the on-top convention over the lifted
cell; W12-a moves them one down until W12-a-b lands).

W12-c +10 on b1 (read 20:26, hour 15 of day 0): the general store's
void 0 (the eight void lines are one 1x1 private shelf, region min
(7640, 6317, 181), surface 1 standable 0 -- a shelf whose cell is not
standable, E2-k's class); DELIVERED z 182 (25), DepositRun job_pos z
182 (87 distinct); store-row embeds 0; general_max_cell 144;
unreachable 0 of 236; arrivals 490 (b1 baseline 527-617; W13's b1
+10: 471); pump mean wait 70, pending 32 (41 and 14 at +4: the
pump's load grows through the day on every pair); stuck 0; panics
0. EMBED WATCH 5, all chaser-pure-glide and all of one shape --
entry z 180.09 -> embedded 180.003, a body snapped down to an
integer z and read as inside the block -- W13's lowering, the half
W13-b removes; none at the store rows. The five sit on one line:
(7648..7656, 6417, 180.09), bodies bound north for (7621..7648,
6461..6513, 181..182) -- a floor at 180 whose block the snap's
`surface_at` reads as standable (not solid) and the watch reads as
terrain (filled): the snapped body is held at 180.0 step after step
and persists a second; the old line rose through it. W13-b's
max(line, floor) leaves the line's z there. W12-c's bars all hold
at +10; the day-1 read follows.

W12-c day 1 on b1 (read 20:42, hour 0 of day 1): the general store's
void 0 all day (the nine void lines are the one 1x1 shelf); lifted
698 of 1,005; DELIVERED z 182 (25); DepositRun job_pos z 182 (129
distinct); general_max_cell 417 (bar under 600); embeds at the store
rows 0; EMBED WATCH 5 in all -- the five snapped bodies at (7648..
7656, 6417, 180.09) from +10 and none since (W13-b's) -- PASSED
(bar under 8); unreachable 0 of 401; FIRST LEG GATE 54 of 60;
arrivals 710 by day 1 (W13's b1 day 772; W12-a's b2 day 729); STALL
BLAMED 3; ROUTE FAULT 4; stuck 0; starving 1 at midnight; panics 0.
W12-c STANDS: every bar it registered held at +4, +10 and day 1,
and the pump's wait at +4 fell from 73 to 41 with it.

## W13-b, registered 19:48 (keyed on the W12-c stage; ahead of E2-l and the rest)

THE GLIDE RISES TO THE FLOOR AND NEVER SINKS TO IT. W13's snap set z
to the floor whether the line was above or below it; the embeds
came only from lines under the floor, and the lowering cost a third
of the arrivals (b2 +10: 502 -> 301 with the pile, 333 with the
store spread; b1 471 against 527-617). The arrive test (3D, 2.5
blocks against the on-top target) is not the reason: a body one
block under the target still arrives. Now `glide_snap_z` = (x, y,
max(line z, floor z)); a line above the floor keeps its z. Pin
re-stated (a floor above lifts; a floor below leaves the line);
planted: the snap lowering again, red. Prediction (b2 fresh,
`wait-w13b-b2.sh`, after E2-j-b's night-1 read; +10 and day 1):
arrivals at +10 at least 450 (301, 333); pure-glide embeds at most
2 by +10 and 4 by day 1; pump mean wait under 70 (104); GLIDE HELD
0; stuck 0. Falsified if arrivals stay under 400 (W13 comes out
whole), or the pure-glide embeds return above 4 by +10. Rejected:
W13 out whole before this read; a tolerance band around the floor.
NOT evidenced: b1; day 2. The E2-l chain was re-keyed behind this
row and the W12-a-b reader behind its day 1; the dry tree was
rebuilt in the order E2-j-b, W12-c, W13-b, E2-l, E2-m, E2-k, E2-i2,
W12-a-b, W15-i1.

### W13-b landed (b9d05905ca, staged 20:37)

Check clean, the pin green, both halves staged 20:37. The row has no
witness string of its own (one expression changed); the binary
carries W13's and W12-c's strings and the pair's lineage, and the
behaviour is read live; shipped to lab-bin 20:38. The falsifier
(the snap lowering again) went RED at 20:41, the tree restored
clean. The b2 reader restarted b2 at 20:39 (after E2-j-b's night-1
block) and reads +4, +10 and day 1 -- the decider for W13; E2-l's
chain fired five minutes after this stage.

W13-b +4 and +10 on b2 (read 20:43 and 20:49, hours 11 and 15 of
day 0), against the bars: arrivals at +10 at least 450 -- 387 (W13
301, W12-b-b 333, W12-a 502): FAILED, and the clause "arrivals stay
under 400: W13 comes out whole" FIRED by thirteen; pure-glide embeds
at most 2 -- 0 (+4 and +10), PASSED; pump mean wait under 70 -- 96
at +4, 79 at +10, pending 32-34, worst 858: FAILED; GLIDE HELD 0;
stuck 1; snaps 1,024 and 2,048 (the raises alone; W13's 2,048 and
16,384). The raise-only snap gave back a sixth of the loss and kept
the embeds off; the pre-registered clause names the disposition,
and it is honoured: W13-w (the snap comes out) is written, dry-run
and queued behind E2-l, with E2-m re-keyed behind it and the
W12-a-b reader behind its day 1. If W13-w's +10 reads at least 450
the cost was the snap's and the embed class is open again, owned by
the next wedge row at the ROUTE (a node per step-down) rather than
at the walk; if it stays under 400 the cost was never the snap's
(W12-b's centre picker, or the world) and W13-b returns.

W13-b day 1 on b2 (read 21:04, hour 0 of day 1): EMBED WATCH 0 the
whole day (W12-a's day 18; W12-b's 9; W12-b-b's 0), STALL BLAMED 1,
ROUTE FAULT 1; arrivals 586 by day 1 (W12-a 729, W12-b 675, W12-b-b
497); unreachable 0 of 421; pump at midnight pending 4, mean wait
38; stuck 0; starving 2 at midnight; panics 0. The trade is now
plain on two b2 days: the raise-only snap keeps every embed off and
costs a fifth of the arrivals against the no-snap pair. The
pre-registered clause fired at +10 and W13-w runs; its read says
what the arrivals are worth without the snap, and the middle -- the
raise at step-down edges only, where the seven of eight embeds were
-- is the candidate if the embeds return with the arrivals.

## W13-w, registered 20:52 (keyed on the E2-l stage; ahead of E2-m and the rest)

THE SNAP COMES OUT. The CommittedGlide::Step arm steps the line as
before W13; glide_snap_z, GLIDE_SNAPS and the pin
the_glide_follows_the_surface are removed (a helper nothing calls
is a dead lane). No pin is added and no falsifier runs: the removal
is the row and the live read is its falsifier. Prediction (b2 fresh
after W13-b's day-1 block; +10): arrivals at least 450 (301-387 on
the snap pairs); pump mean wait under 60 (79-105); EMBED WATCH by
+10 back to W12-a's order (12), the class OPEN; GLIDE HELD 0; stuck
0. Falsified if arrivals stay under 400 with the snap gone (W13-b
returns). The chain's green pin is `the_drop_cell_stands_on_the_
floor` (the row removes its own).

### W13-w landed (d84a137678, staged 21:23)

Check clean, the neighbouring pin green (the row removes its own),
both halves staged 21:23 and shipped to lab-bin 21:24; the binary
verified by contents: "THE GLIDE FOLLOWS THE SURFACE" absent (0),
W12-c's string present (1). The b2 reader restarts b2 after W13-b's
day-1 block and reads +4, +10 and day 1 -- the arrivals and the
embeds with the snap gone.

### W13-w read at +10 (b2, 21:35): the arrivals clause was two frames

ARRIVED AT JOB SITE 355 at +10 with the snap gone; EMBED WATCH 1
(one pure-glide writer, entry z 181.5 -> 181.7); GLIDE SNAPS 0
(the string is out); pump mean wait 99, max 453, pending 34;
stuck 0; panics 0. The pre-registered decider (arrivals at least
450 -> the embed class stays open; under 400 -> W13-b returns)
compared W12-a's 502 with the snap pairs' 301-387, and W12-a's
+10 was read on the OLD store geometry, before W12-b-b/W12-c lifted
the drop cells onto the plank floor; every pair since (with the
snap 301, 333, 387; without it 355) sits in one band. The snap
never cost the arrivals -- the store lift changed what a delivery
is -- and the clause is VOID, not fired: W13-b does not return on
it. Two frames compared as one, caught by the read the clause was
written for. The embed count is the only decider left: the day-1
block (hour 6 of day 1) reads it; at most 3 pure-glide embeds
means the embed class was the plank store (W12-c) and W13-w
stands with W13-c held; ten or more means W13-c launches.

### W13-w read at day 1 (b2, 21:48): five embeds, four of them risers -- the raise returns

EMBED WATCH 5 by day 1 (W12-a's 18 before the store lift; W13-b's
0), all chaser-pure-glide; the entry -> embedded z pairs 181.54 ->
181.72, 181.56 -> 182.01, 181.41 -> 182.00, 181.68 -> 181.85 and
184.00 -> 183.94: four of the five ROSE into a riser (the line's z
lagging under a step-up) and one dipped six hundredths. Arrivals
535 by day 1 (W13-b 586); pump at midnight pending 1, mean wait 13
(W13-b 38); stuck 0; unreachable 0 of 372; panics 0. The count fell
in the gap the decider left open (4-9), and the signature settles
it: W13-c's own-floor hold answers the dip, which is one embed in
five, and is WITHDRAWN; the raise-only snap (W13-b) lifts exactly
the riser lag and cost nothing on this geometry (387/586 with it,
355/535 without). W13-b was removed on a clause that compared the
pre-lift store with the lifted one; the removal was for nothing.
The pair kept running while W13-b-r queued: by hour 15 of day 1
EMBED WATCH stood at 18 (5 at the day-1 line, 13 more in fifteen
game hours), arrivals 1,215, pump pending 23 and mean wait 134 --
the no-raise rate on the lifted store is W12-a's order (18 a day)
once the town is under way, and the raise's day-1 count of 0 is
the number to beat.

## W13-b-r, registered 21:58 (keyed on the E2-o stage; ahead of E2-k, E2-i2, W12-a-b and W15-i1)

THE RAISE RETURNS. W13-b's three hunks return as W13-w removed
them: `glide_snap_z(try_pos, floor)` = the line lifted to a known
floor above it, the line otherwise; the Step arm steps the snapped
point; THE GLIDE FOLLOWS THE SURFACE (uid, line_z, floor_z, node,
snaps) and GLIDE_SNAPS. The removed comments' false claim ("cost a
third of the arrivals on three replicates") is rewritten to the
reads. Pin `the_glide_follows_the_surface` (a floor above: the body
rises; a floor below: the line; no floor: the line); planted: the
snap lowering again (z = floor), red. Prediction (b2 fresh after
W13-w's day-1 block, `wait-w13br-b2.sh`; +10 and day 1): EMBED
WATCH at most 1 by day 1 (W13-b 0; W13-w 5); arrivals at +10 at
least 300 and by day 1 at least 480 (the lifted-store band:
355/535 without, 387/586 with); pump mean wait at midnight under
60; GLIDE SNAPS at least 512 by day 1; stuck 0; unreachable 0.
Falsified if the embeds reach 4 by day 1 with the raise aboard
(the residual is not the riser lag), or if day-1 arrivals fall
under 430 (the raise costs after all on this geometry). Rejected:
W13-c (one embed in five); a route node per step (the route is
right, the walk lags); leaving the class open with the raise in
hand. NOT evidenced: b1 (its 490-at-+10 "cost" was the same two
frames; E2-l's pair is b1's first post-lift no-raise number); the
one dip; night 2. The queue was reordered (E2-m, E2-o, W13-b-r,
E2-k, E2-i2, W12-a-b, W15-i1); the E2-k chain and the W12-a-b b2
reader were re-keyed by their pid files; the dry tree was rebuilt
in the new order.

### W13-b-r landed (cb11a71b0d, staged 22:49)

Check clean, the pin green, both halves staged 22:49; the binary
verified by contents (THE GLIDE FOLLOWS THE SURFACE back, E2-o's
string present). The b2 reader restarts b2 at the stage (the W13-w
pair's log, with its day-2 count of 21 embeds at hour 8, rolls to
/tmp/arm-b2) and reads +4, +10 and day 1 against the bars above;
Falsified: the snap lowering again (z = floor) planted at
cb11a71b0d, the pin RED at 22:54 on "a floor below the line: the
line keeps its z (W13-b)", the tree restored clean. Shipped to
lab-bin 22:50.

W13-b-r at +10 on b2 (read 23:01, hour 14 of day 0): arrivals 372
(bar at least 300; W13-w 355, W13-b 387): PASSED; EMBED WATCH 1
(one pure-glide writer, entry z 181.88 -> 181.98, a tenth of a
block up; the day-1 bar is at most 1); GLIDE SNAPS 2,048 (the
raises firing, W13-b's order); pump mean wait 18, max 29, pending
37 (bar under 60 at midnight); stuck 0; unreachable 0 of 373;
panics 0. Day 1 decides the embed bar and the arrivals band.

W13-b-r at day 1 on b2 (read 23:18, hour 0 of day 1): arrivals 591
(bar at least 480; W13-w 535, W13-b 586): PASSED; EMBED WATCH 2
(bar at most 1; W13-b 0, W13-w 5): FAILED by one -- the +10 riser
(181.88 -> 181.98) and one DIP at a step-down edge (184.99 ->
183.81, a block and a fifth), the signature W13-c was written for;
GLIDE SNAPS 4,096; pump at midnight pending 3, mean wait 14 (bar
under 60): PASSED; stuck 0: PASSED; unreachable 11 of 645
(unreachable_open, from_in_house, first_leg false; W13-w 0 of 372,
W13-b 0 of 421; bar 0): FAILED, one replicate, a class this row
does not touch (the W12-a stand rule's territory: eight DOOR PROBE
NoDoor verdicts on the same day, the known false verdict). Neither
falsification clause fired (embeds under 4; arrivals over 430).
Disposition: PARTIAL, the raise stands -- the arrivals band holds
with it (591 against 535 without), the residual is one riser and
one dip a day against 5-18 without it; the dip is W13-c's case,
one a day, and stays in the ledger; the 11 unreachable are read
again on the next b2 pair before they are a row.

W13-b-r at day 2 on b2 (read 00:02, hour 0 of day 2; the pair ran
on until W12-a-b's stage): EMBED WATCH 3 over two days (2 by day 1,
one more on day 1; the no-raise pair carried 5 by day 1 and 21 by
hour 8 of day 2), arrivals 1,450 (W13-w 1,215 by hour 15 of day
1), stuck 0. The raise holds at one embed a day. Two counts rose
with the day: unreachable deliveries 48 (11 by day 1; 20
UNREACHABLE APPROACH lines, 10 NoDoor probes -- the W12-a stand
class, read next on the W12-a-b pair, whose rule is that very
target) and the pump's exhausted deliveries 2,729 over the two
days (the flat arm prints no longest-tier line; W15-i1 classifies
them by direction, W14 damps the re-asks). The store's max cell
918 is the wheat-seed stack (seeds merge by def), not the larder.

### W12-a-b landed (b6b78b96d1, staged 00:17)

Check clean, the pin (re-stated) green, both halves staged 00:17;
the row adds no string of its own (one condition widened), the
binary carries W12-a's and E2-i2's strings. Shipped to lab-bin
00:17. The b2 reader restarts b2 at the stage (the W13-b-r pair's
two days roll to /tmp/arm-b2) and reads +4, +10 and day 1: the
on-top target's stand, and the unreachable deliveries that rose to
48 by day 2 on the pair before.

**THE PIN STAYED GREEN (00:20).** The falsifier planted the old
line (`if standable(target) {`, the on-top rule removed) at
b6b78b96d1 and `the_search_aims_at_the_stand_not_the_stone`
PASSED. The W12-a-b assert -- one below standable, expect the
target -- is vacuous: `search_stand`'s ring search skips the
target's own column, so with only the cell below standable the
ring finds nothing and the fallback returns the target, the same
answer the rule gives. A pin that cannot tell the rule from its
absence guards nothing, and the row landed and shipped on it.
Reported here, in the handoff and to Ben. Fix: W12-a-b-p (the
assert offers a competing ring cell, one east; the rule answers
the target, the ring alone answers the east cell), queued at the
queue's end after W14 with the same plant, expected RED. The live
read on b2 (below) stands as W12-a-b's evidence regardless; the
pattern to check on every "one condition widened" row is an
assert that coincides with the fallback.

W12-a-b at +10 on b2 (read 00:29, hour 15 of day 0): SEARCH TARGET
MOVED 512 (W12-a's pairs 4,096-8,192 by +10: the on-top targets no
longer move -- the first four moved targets are (7714,6342,185)
and one at z 182, cells with nothing standable below either);
arrivals 435 (372, 355, 387 on the three pairs before); EMBED
WATCH 2 (chaser-pure-glide; the raise aboard); unreachable 1 of
372 (0 at +10 before); pump mean wait 78, pending 30, max 187
(18 on W13-b-r's +10 -- the pump's load swings between boots);
stuck 0; panics 0. Day 1 reads the unreachable class that reached
48 by day 2 on the pair before.

W12-a-b at day 1 on b2 (read 00:42, hour 0 of day 1): arrivals 641
(591 on W13-b-r, 535 on W13-w, 586 on W13-b -- the best day 1 of
the run); unreachable deliveries 1 of 501 (11 of 645 on W13-b-r's
day 1; UNREACHABLE APPROACH lines 0 against 16): the on-top stand
rule is where that class lived; SEARCH TARGET MOVED 2,048 by day
1 (8,192 on W12-a's pairs); EMBED WATCH 2 (the raise's residual:
W13-b-r 2); pump at midnight pending 13, mean wait 50; stuck 0;
panics 0. Disposition: PASSED on the live read -- the moved
searches down four times, the unreachable deliveries down eleven
times, arrivals up -- on a pin that stayed green; W12-a-b-p
re-states the pin so the plant goes red. The pair keeps running
on b2 until the next b2 row: its day 2 and beyond are the free
replicate of both counts.

W12-a-b at day 2 on b2 (read 01:28, hour 0 of day 2): EMBED WATCH
2 over two days (W13-b-r's 3); arrivals 1,270 (W13-b-r 1,450, the
band); unreachable deliveries 1 over two days (W13-b-r's 48 --
the stand rule is that class); exhausted deliveries 1,144 over two
days (W13-b-r's 2,729: halved, the on-top targets no longer
exhaust the searches that aimed beside them); SEARCH TARGET MOVED
8,192 by day 2 (W12-a's 8,192 by day 1); stuck 0; pump mean wait
55, pending 9. The two-day replicate confirms the day-1 read on
both counts the day before flagged.

### W15-i1 landed (fc0d3a31b3, staged 00:40)

Check clean, the pin green, both halves staged 00:40; the binary
verified by contents (THE SEARCH EXHAUSTED present). The b1 reader
restarts b1 at the stage (after E2-i2's night-1 block, already
landed) and reads the boot shelves and night 1: the exhausted
searches by direction (up, down, flat) through the night hours.
Falsified: the band ignored planted at fc0d3a31b3, the pin RED at
00:43, the tree restored clean. Shipped to lab-bin 00:41.

W15-i1 read (b1 fresh on fc0d3a31b3; boot, day 0 and night 1, read
01:16): exhausted fill searches by day 1 181 -- up 38, down 9,
FLAT 133 (74%); through the night hours 21-3: up 4, down 1, flat
11. Up does not dominate; flat does, four to one. The first
witnesses name the shape: from (7767,6399,181) to (7793,6365,183)
dz 2, from (7647,6339,182) to (7659,6371,183) dz 1, from
(7640,6344,182) to (7657,6367,183) dz 1, from (7690,6371,183) to
(7657,6362,183) -- thirty to forty blocks across town to the
STORE's lifted drop cells at z 183, the exact fill search spending
its budget on the way. Classified after the block: eleven of the
twelve printed witnesses target a STORE's standing cell at z 183
(the main store's 7657-7673, 6362-6373; zones 61, 59 and 87's), from
five to forty blocks away, dz 0-2; the twelfth a cell at z 186.
The escalation runs Small, Medium, Long, Longest (500, 5,000,
25,000, 75,000 expansions) before the pump delivers
BudgetExhausted, so these are not budgets too small for a walk:
seventy-five thousand expansions did not reach a store cell that
deliveries reach six hundred times a day. The search's own
verdict at exhaustion (`PathResult::Exhausted` carries the
closest node; `Astar::closest_node` its distance) is the probe
the next instrument (W15-i2) prints: the target's walkability at,
below and above, its 3x3x3, the start's, and how close the
frontier came -- sealed locally (a door, a step) or cut off. W15 (the top step under the
slab) is WITHDRAWN: its premise was the upstairs bed and the
staircase, and the night's exhausted searches are sixteen, four of
them up. The class that IS there -- the flat store-bound search
that exhausts -- is W14's territory first (the memo stops the
re-asks) and then a budget or trunk question (why the trunk route
did not carry a store trip), read on W14's pair. The night
otherwise: 55 loads, 15 swept, 38 private arrivals; starving
sleepers 2; in-bed starving at 0-3 7, 8, 7, 8; NIGHT SHELF 15
Empty, no Refused (E2-i2's arm unexercised again); meals 75,
no_food_found 158.

### W14 landed (0b5c172d15, staged 01:27)

Check clean, the pin green, both halves staged 01:27; the binary
verified by contents (THE SEARCH IS NOT ASKED TWICE present). The
b1 reader restarts b1 at the stage (after W15-i1's night-1 block)
and reads hour 19 of day 0 and night 1 against the bars above; the
E2-l-i printer reads the same pair's sweep after that block.
Falsified: the cell ignored (a moved body still refused) planted
at 0b5c172d15, the pin RED at 01:32, the tree restored clean.
Shipped to lab-bin 01:28. W12-a-b-p's chain fires five minutes
after this stage, W15-i2's five minutes after that one.

### W14 read at hour 19 (b1 fresh on 0b5c172d15, read 01:52): FAILED by its own clause, and the clause named the cause

Against the bars at hour 19 of day 0: THE SEARCH IS NOT ASKED
TWICE 5 refusals (bar 200 by day 1) -- one colonist, uid 127, five
refusals inside fifteen seconds at hour 16, then none; the
most-repeated longest-tier pair 3,791 steps (bar 1,500) -- from
(7714,6344,182) to (7715,6344,186), the SAME raised-road spot that
held colonist 112 on the E2-l pair, four blocks up; longest-tier
steps 53,693 by hour 19 (the E2-l pair 46,545 by hour 21: the same
order); the day's pump mean wait 77 (72-173 in the E2-l pair's hot
window); arrivals 758; stuck 0; STUCK CENSUS 6 colonists. The
pre-registered falsification named it: "refusals under 20 while
the top pair stays high: the memo never matches". The code says
why: the memo is WRITTEN at the exhausted delivery with
`ps.target` -- which for the trunk's APPROACH search is node 0's
centre, not the job's target -- and READ before the enqueue
against the job's target cell; the two never agree for the lane
that re-asks, so the approach search under the raised road is
asked again every two seconds as before. Generator and consumer
disagree, by my own hand. Disposition: FAILED as built; the rule
and its pin stand, the key is wrong for one lane. W14-b1 (the
pending search carries the job's target cell; the memo stores
that) is written next and queued after W15-i2; its read is the
same reader, the same bars.

W14 at night 1 (read 02:14, hour 6 of day 1): THE SEARCH IS NOT
ASKED TWICE 32,768 by night 1 (bar 200: PASSED) -- five refusals
through the day, then from hour 23 the exact-search lane's re-asks
(the same job target from the same cell, which the key does match)
were refused by the tens of thousands, four walkers named (127
arrived after, 110 arrived twice, 123 and 130 not); the top
longest-tier pair 14,791 steps by day 1 (bar 1,500: FAILED) -- the
raised-road approach search, whose key the memo never matched;
longest-tier steps 77,947 by day 1 (bar 30,000: FAILED); the day's
pump mean wait 59 (bar 60: PASSED, just); pending at midnight 24
(bar 20: FAILED); arrivals 963; stuck 0; STUCK CENSUS 19 colonists
across the day. Disposition: PARTIAL -- the memo works on the
lane whose key matches and is blind to the lane that re-asks
most; W14-b1 carries the job's target into the key, and its read
on the same bars decides the row. NOT evidenced: whether thirty
thousand refusals starve a walker the memo protects (the beeline
stands; the named walkers' fate is mixed and small).

### W15-i2 landed (a6fa8fe2ee, staged 02:04)

Check clean, the pin green, both halves rebuilt (common/ changed)
and staged 02:04; the server verified by contents (THE EXHAUSTED
SEARCH NAMES ITS TARGET present), the client from the same commit.
The b1 reader restarts b1 after W14's night-1 block and reads the
probe by class at hour 19 and night 1. Falsified: every frontier
called sealed planted at a6fa8fe2ee, the pin RED at 02:09 on "the
frontier never came near", the tree restored clean. Shipped to
lab-bin 02:05.

### W15-i2 read at hour 19 (b1 fresh on a6fa8fe2ee, read 02:34): the probe names two thirds of the exhausted searches as aimed one above a standing cell

65 probes by hour 19 of day 0, every one with a closest node
(unknown 0: the stash agrees with the outcome; the reader's class
grep missed the quoted field, the log tallied by hand). By class
and target: 44 target_unwalkable with target_walk (false, true,
false) -- the target cell is not a standing cell, the cell below it
is; 13 sealed and 8 cut_off with a walkable target, (true, false,
false). The 44 aim at store cells at z 183 over standing cells at
z 182 (the main store's 7662-7667, 6362-6367; zone 61's 7791-7793,
6365-6367) from starts five to forty blocks away; the frontier's
closest node stopped 2-15 blocks short in xy at z 182 (histogram
of closest_xy: one under 1.5, four under 3, twenty-one under 8,
eighteen beyond). The router's goal is the exact end cell
(`satisfied = |node| node.pos == end`), and no node can stand in
the end cell, so these searches never close; W12-a-b (the on-top
target stands) kept exactly this target for the search. The
deliveries arrive by the trunk and the glide, which never ask the
fill. The ring strings all read '~' (air over air at z 183): the
probe reads the target's own row, one above the floor, which is
the finding restated. Disposition: the instrument PASSED (every
probe classed, no unknown) and named the row: W12-a-c, the search
aims one below the on-top target. The sealed 13 and cut_off 8
(walkable targets, the frontier near or far) are the interior
gate's or the door's territory, read after W12-a-c lands.

W15-i2 at night 1 (read 02:50, hour 6 of day 1): 68 probes over
the day -- 44 target_unwalkable (the on-top target), 13 sealed, 11
cut_off (tallied by hand from the log; the reader's class grep
misses the quoted field). Unknown 0. The pair's longest-tier
steps by day 1: 1,857 -- against 77,947 on the W14 pair the day
before and 46,545-82,331 on the pairs before that: nobody was
stranded under the raised road today (the top pair there, 7721,
6355 to 7721,6354,185, took 201 steps; on the W14 pair one walker
took 14,791), and the day's search load is what a town without a
stranded walker costs. THE SEARCH IS NOT ASKED TWICE 8,192 by
night 1, two walkers named, both arrived after; pump mean wait 59,
pending at midnight 31; arrivals 717; stuck 0; panics 0. The
stranded walker is a one-in-two event per day on this arm --
W14-b1's read (the third replicate, the key fixed) says whether
the memo holds it when it happens.

### W14-b1 read at hour 19 (b1 on the E2-p pair 105f775a97, read 03:11): the key is fixed and the day's re-asker was still never refused

THE SEARCH IS NOT ASKED TWICE 4 refusals by hour 19 (bar 200 by
day 1), two walkers (16 and 17, at hours 11 and 14, both arrived
after); longest-tier steps 6,969 by hour 19 (53,693 on the W14
pair's hour 19; 947 on W15-i2's); the most-repeated pair 3,737
steps (bar 1,500) -- from (7725,6404,181) to (7742,6404,181),
eighteen blocks east on the flat, a new spot: job 797, an EatFrom
for item 1058, FETCH STALLED six times with no displacement; by
03:20 that walker had started 7,205 searches from the one cell
(7725,6404) and 805 from (7725,6403), a hundred-odd searches
reaching the longest tier, and the memo refused none of them. The
exhausted-search probe printed no line for that target: it
samples the first sixty-four exhaustions of the day and the
powers of two after, and a walker that starts re-asking at hour
12 is past the sample -- a witness that shows the head misses the
late offender. The raised-road pair (7714,6342 -> 7713,6345,186)
took 706 steps today. Pump mean wait 62; arrivals 532; stuck 0.
What the log cannot say: whether the memo was written for that
walker (its search may never have delivered: a pending fill is
replaced by the next enqueue), or which field of the key failed.
W14-i (below) is that instrument.

W14-b1 at night 1 (read 03:26, hour 6 of day 1): THE SEARCH IS NOT
ASKED TWICE 16,384 by night 1 (bar 200: PASSED; four walkers
named, the exact lane from bed); longest-tier steps 12,450 by day
1 (bar 30,000: PASSED; 77,947 on the W14 pair); the raised-road
pair 873 steps (14,791 on the W14 pair: the key fix holds it); the
most-repeated pair 7,205 + 805 steps (bar 1,500: FAILED) -- the
flat re-asker at (7725,6404) never refused; the day's pump mean
wait 60 (bar under 60: FAILED by nothing); pending at midnight 26
(bar 20: FAILED); arrivals 718; stuck 0; STUCK CENSUS 5 colonists.
Disposition: PARTIAL -- the memo holds the two lanes whose key it
matches (the bed's exact lane, the raised road's approach) and is
blind to one flat re-asker whose key or write it never met; W14-i
names which, and W14-b2 follows from that name.

### W12-a-c read at +10 (b2 fresh on c75a908c89, read 03:23): the on-top class is gone; the rest stop on the store's perimeter

64 probes by +10: 43 cut_off, 21 sealed, and NONE target_unwalkable
(44 of 65 the day before): the on-top class is closed on this
read (bar: at most 5%: PASSED). Every remaining target is a
standing cell with a standing cell below it not, (true, false,
false), and 41 of 64 rings read '.........' -- the target's whole
neighbourhood walkable. Where the frontier stopped, for the main
store's targets: (7662-7665, 6360-6363, 182) and (7668-7670, 6378,
182) -- the store's south and north edges, three to eight blocks
from targets deep inside; closest_xy by class: sealed 5 under 1.5
and 16 at 1.5-3, cut_off 14 at 3-8, 16 at 8-20, 13 beyond 20.
SEARCH TARGET MOVED 8,192 by +10 (512 under W12-a-b: the on-top
moves counted again, the fix working); exhausted deliveries 70 by
+10; arrivals 354; embeds 0; pump mean wait 5; stuck 0. The router
refuses a FLAT step into the store: its interior gate applies to
climbs only (path.rs, dir.z >= 2, "roofs are not routes"), the
interior surcharge is a cost of 2 and no max_cost bounds the
search, so it is `is_walkable` on the cell just inside -- a rail,
a post pattern (six rings '#..#..#..'), or a door sprite the
router does not open while the trunk (which recognises doors)
and the glide do. The probe read the target's ring; W15-i3 reads
the frontier's.

W12-a-c at day 1 on b2 (read 03:39, hour 0 of day 1): exhausted
deliveries 135 by day 1 (bar at most 200; 572 a day under
W12-a-b): PASSED; probes 65, none target_unwalkable (bar at most
5%): PASSED -- 44 cut_off, 21 sealed, every target a standing
cell; unreachable deliveries 0 (bar at most 3): PASSED; stuck 0:
PASSED; arrivals 543 by day 1 (bar at least 550; 641 under
W12-a-b, 591 and 535 on the pairs before): FAILED by seven, in the
band; pump mean wait 89 at midnight, pending 13 (bar under 40; 50
and 55 before): FAILED, one replicate (the pump's wait swings
between boots: 5 at +10 on this pair). No falsification clause
fired (the on-top class is 0, exhausted under 400, arrivals over
500). Disposition: PASSED on the claim it made -- the on-top
class is closed and the exhausted searches fell four times -- the
two peripheral bars missed. The 135 that remain are the perimeter
class W15-i3 reads next.

### W14-i landed (c6536a2d4b, staged 03:46)

Check clean, the pin green, both halves staged 03:46; the binary
verified by contents (THE MEMO DID NOT MATCH present). The b1
reader restarts b1 after E2-p's night-1 block and reads W14's
bars with the memo's writes and near misses. Falsifier at 03:51:
start and target swapped in memo_near_miss, the pin went RED
(0 passed, 1 failed), restored to 0 dirty files. W15-i3's chain
fired at 03:51.

## W15-i3, registered 03:34 (keyed on the W14-i stage; the queue's end)

THE FRONTIER NAMES WHAT STOPPED IT. At the exhausted delivery the
probe also prints the 3x3 ring around the closest node at its z
(`frontier_glyph`: '#' solid, 'D' a door sprite, '.' walkable, '~'
neither; solid over door over walkable) and the sprite or block
kind of every '~' and 'D' cell. No behaviour changes. Pin
`the_frontier_names_what_stopped_it`; planted: a door drawn as
neither, red. Prediction (b2 fresh at the stage,
`wait-w15i3-b2.sh`, +10 and day 1): every cut_off and sealed probe
prints a frontier ring; the perimeter rings are not all walkable
and name what stopped the frontier. Falsified if the perimeter
rings read '.........' (the transition rule refused, not the cell:
the next probe prints the transition's verdict). NOT evidenced:
the fix; b1.

### W15-i3 landed (23195dc174, staged 04:10)

Check clean, the pin green (six glyph cases), both halves staged
04:10:12, shipped to lab-bin 04:11; the binary verified by
contents (frontier_ring present). Falsifier at 04:14: a door drawn
as neither, the pin RED, restored to 0 dirty files. The b2 reader
restarts b2 at this stage and reads +10 and day 1 with the
frontier rings and sprites.

### W15-i3 read: b2 +10 (23195dc174, 04:21, hour 15 of day 0)

64 probes: 46 cut_off, 17 sealed, 1 target_unwalkable; 63 with a
walkable target. Frontier rings: 19 `.........` (every neighbour
of the closest node walkable: the router refused for a reason the
cell does not show -- the transition rule, the pre-registered
falsification of the sprite hypothesis for those 19), 10
`###...###` (a wall row north and south, an east-west corridor),
9 `...#.....`, 7 `......#..`, 4 `......###`, 2 `#~~#..###`;
sprites named: 11 `Empty` (the `~` cells are air with nothing to
stand on, not a rail or a post). No door glyph anywhere: the
frontier does not stop at a door sprite. Exhausted searches 87 in
ten minutes (135 for W12-a-c's whole day 1: to be read against
the day-1 block). Reading: the store's perimeter frontiers are
walls and drops, and a third are nothing the ring shows. The
day-1 read decides W15-i4 (print the refused transition's
verdict at the frontier: which neighbour the router declined and
why).

### W15-i3 read: b2 day 1 (23195dc174, 04:35, hour 0 of day 1)

Exhausted searches for the day 366 (W12-a-c's day 1 was 135 on
another run; colony counts vary 2-3x). Probes 66 (the head sample
caps the frontier evidence at the day's first 64: the rings are
the MORNING's), 47 cut_off, 18 sealed, 1 target_unwalkable, 65
with a walkable target. Rings: 20 `.........`, 10 `###...###`, 9
`...#.....`, 7 `......#..`, 4 `......###`, 2 `#~~#..###`; sprites
11 Empty; no door glyph in any ring. Arrivals 603, stuck 0, pump
pending 0 at the read, unreachable 0. PASSED on what it
registered: every cut_off and sealed probe prints a ring and the
`~` cells carry a kind. FALSIFIED on the sprite hypothesis for 20
of 66: the ring is all walkable and the router still did not step
toward the target -- the transition rule, not the cell. The
corridor rings (`###...###`, a wall row north and south) say the
frontier sits in a one-wide passage beside the store's wall with
the target beyond it, and the search spent its 75,000 expansions
without finding the way in. Next: W15-i4 -- one transition
predicate shared by the router and the probe (generator and
consumer agree), and the probe prints, for the frontier's
neighbour toward the target, which sub-check refused it.
Queued behind E2-s and W14-b2.

### W14-i read: b1 hour 19 (the 23195dc174 pair, 04:29)

THE SEARCH IS NOT ASKED TWICE 0 refusals (4 at hour 19 on W14-b1's
read); memo writes 93; near-miss lines 39, the miss counter past
4,096; every named miss why="start" (39 of 39), the top missers
uid 58 (8), 8 (7), 16 (5). Longest-tier steps 8,950 by hour 19;
top pairs (7712,6342,181)->(7712,6341,186) 4,051 (five up, one
block over), the flat re-asker (7725,6404)->(7742,6404) 2,479,
(7712,6342,181)->(7712,6342,198) 678, (7712,6342,182)->(7712,6341,
186) 469. Arrivals 601, stuck census 2 distinct, exhausted lines
11. PASSED as registered: the field is named, and it is the start
cell -- the walker's feet have moved a block or two from the cell
the memo stored (the glide moves the body while the search is
pending), so the exact-cell key never matches and the memo refuses
nothing. W14-b2: the start matches within a radius (3 blocks xy,
2 z), one predicate for the refusal and the near miss.

### W14-i read: b1 night 1 (04:46, hour 6 of day 1)

Refusals 32,768 by the night (0 at hour 19: the memo refuses only
a body whose feet cell has not moved, which at night is a sleeper
or a wedged walker; uid 58 refused 13 times seen, arrived after 0,
stuck census 3 lines, last refused at hour 1); misses 8,192, every
named one "start" (40 of 40); writes 150; longest-tier steps
10,802, the top pair (7712,6342,181)->(7712,6341,186) at 4,380,
the flat re-asker 2,479 (unchanged since hour 19: it re-asks by
day). Arrivals 838, stuck census 8 distinct, exhausted lines 14,
pump mean wait 56. PASSED as registered; the field is the start,
by day and by night, and W14-b2's radius is the answer it names.

## W14-b2, registered 04:40 (keyed on the E2-s stage; the queue's end)

THE MEMO MATCHES A BODY THAT SLID. Defect: above (0 refusals,
4,096 misses, every one "start"). Mechanism:
`memo_start_matches(stored, feet)` -- Chebyshev within
MEMO_START_XY (3) in xy and MEMO_START_Z (2) in z -- used by
`search_memo_refuses` and `memo_near_miss` (generator and consumer
agree); target and window unchanged. No new log line. Pins
re-stated: `the_search_is_not_asked_twice` (same cell: refused;
slid one block: refused; slid to the radius (3,-3,2): refused;
moved four: asks again; moved three up: asks again; another
target, expired, no memo: asks again) and
`the_memo_names_its_near_misses` (slid one block: no miss; off by
four: "start"). Planted: MEMO_START_XY = 0, red on "slid one
block: refused". Prediction (b1 fresh after E2-s's night-1 block,
`wait-w14b2-b1.sh`; hour 19 and night 1): refusals >= 100 by hour
19 (0 and 4 before); "start" under half of the named misses; the
top repeated longest-tier pair under 1,000 steps by hour 19
(4,051 and 2,479); arrivals within 20% of 601; stuck census <= 2.
Falsified if refusals stay under 10 (the memo is written for a
different target than the fill asks) or the top pair stays above
2,000 (the re-ask is another lane's). Rejected: keying on (uid,
target) alone (a walker that walked away and back could be refused
from a place the search would now succeed from); a larger radius.
NOT evidenced: the re-asked targets' own reachability (five up,
one over: the stair the mover does not climb); b2. The dry tree
at de3c397aa6 (E2-s) validated every anchor.

### W14-b2 landed (c49a3e40b0, staged 05:11)

Check clean, both pins green, committed 05:03, both halves staged
05:11:57 (the server build's log shows ten Compiling lines; the
row adds no string literal, so the binary is verified by the
fresh compile and the falsifier, not by a grep). The b1 reader
restarts b1 after E2-s's night-1 block and reads hour 19 and
night 1. Falsifier at 05:16: a zero radius, the pin RED (0 passed,
1 failed), restored to 0 dirty files. Shipped to lab-bin 05:12.
W15-i4's chain fired at 05:17.

### W14-b2 read: b1 hour 19 (c49a3e40b0, 05:52) -- FAILED its bar; the premise was wrong

THE SEARCH IS NOT ASKED TWICE 32 refusals (bar >= 100; 0 and 4
before), one walker (uid 97, ten refusals, arrived after 4). Memo
writes 97, misses past 4,096, every named miss "start" (39 of 39).
Longest-tier steps 55,255 by hour 19 (8,950 on W14-i's run); top
pairs (7630,6273,180)->(7632,6274,182) 1,587, (7670,6443,183)->
(7681,6444,182) 1,503, (7721,6355,181)->(7721,6354,185) 1,296.
Arrivals 600, stuck 1, exhausted lines 11, pump mean wait 16-72.
The named misses, measured from their own lines (stored start
against the feet, Chebyshev xy): min 8, p25 22, median 25, p75 38,
max 172 blocks; 0 of 39 within 3, 1 within 8, 30 over 20. The
"slid body" of W14-b2's premise -- a walker a block or two from
where it asked -- does not exist in the live population: the
re-askers are walkers that keep GLIDING toward the target between
exhausted searches (THE PRE-PATH GLIDE IS THE TOWN'S LOCOMOTION)
and ask again from twenty-five blocks on, which is a new question
the memo must not refuse. W14-b2's mechanism is correct and its
pin holds, and the arm it guards is unreached (every gate arm must
be reachable in the live population). Disposition: LANDED,
harmless (identity for moves beyond three blocks), FAILED on its
bar, the premise falsified by the instrument it asked for. No
W14-b3: the cost these walkers pay is the exhausted search itself,
which W15-i4 has just placed in the target's own component and
W15-c prices (the destination's building is not a detour). The
night-1 read follows for the record.

### W14-b2 read: b1 night 1 (06:11) -- for the record

Refusals 65,536 by the night (the sleepers' still feet, as on
W14-i's night), writes 717 (150 on W14-i's run: this run asked
far more), misses 8,192 all "start", longest-tier steps 66,001,
the top pair (7697,6425,180)->(7698,6448,182) at 2,600. Arrivals
788 (838; within 20%), stuck census 11 distinct (8), exhausted
lines 14, p95 710 us. Same disposition: landed, harmless, its bar
failed, its premise falsified. The E2-s-i reader took b1 at 06:12.

## W15-i4, registered 04:45 (keyed on the W14-b2 stage; the queue's end)

THE EXHAUSTED SEARCH NAMES ITS COMPONENTS. An instrument row from
W15-i3's two reads: 20 of 64-66 frontiers have a fully walkable
ring and the router did not advance; the move set is
four-connected with one-block rises and drops, so a walkable
neighbour that was never popped means the open set flooded -- the
target in another walking component (a door the router does not
admit) or the 75,000-expansion budget spent inside one. The shadow
component index (`board.component_labels`, rebuilt daily from
every colonist as a seed; readers gated, writer not) can tell
them apart. Mechanism: `exhaust_components(idx, prev_cells, start,
target)` -> (no_index | untrusted | start_unlabelled |
target_unlabelled | same | different, start label, target label),
the target tried at its cell, one below and one above; the probe
line gains `components`. No behaviour changes. Pin
`the_exhausted_search_names_its_components` (no index; two cells:
untrusted; two labels: different; one label: same; found one
below: same; target unlabelled; start unlabelled); planted: same
and different swapped, red. Prediction (b2 fresh after E2-s's
night-1 block on b2, `wait-w15i4-b2.sh`; +10 and day 1): at +10
mostly no_index or untrusted (the index is rebuilt on the day's
clock); by day 1 the perimeter class (sealed and cut_off with a
walkable target) says `different` for at least 80% -- the store's
interior is not the walker's component. Falsified if `same`
carries the majority: the budget or the surcharge, and the next
row prices the route, not the door. Rejected: a shared step
predicate refactor in common (the move set is not what stopped a
walkable ring); printing the open set's size. NOT evidenced: the
door the verdict points at; b1. The dry tree at de3c397aa6 with
W14-b2 applied first validated every anchor.

### W15-i4 landed (a1dc121908, staged 05:35)

Check clean, the pin green (seven cases), committed 05:27, both
halves staged 05:35:42; the binary verified by contents
(start_unlabelled present). The b2 reader restarts b2 at this
stage (E2-s's b2 night block already read) and reads +10 and day 1
with the component verdicts. Falsifier at 05:40: same and
different swapped, the pin RED (0 passed, 1 failed), restored to
0 dirty files. Shipped to lab-bin 05:36. E2-s-i's chain fired at
05:40; W16-a (the stair) is registered behind it.

### W15-i4 read: b2 +10 (a1dc121908, 05:49, hour 14 of day 0)

59 probes (45 cut_off, 13 sealed, 1 target_unwalkable; 58 with a
walkable target); the component index rebuilt 13 times by then.
Verdicts: `same` 34, `untrusted` 13, `start_unlabelled` 11,
`target_unlabelled` 1, `different` 0. The perimeter class (sealed
or cut_off with a walkable target): same 34, different 0. The
pre-registered prediction (different for at least 80%) is
FALSIFIED at the first read: the store's interior IS the walker's
component; no door stands between them. The search spends its
75,000 expansions inside one component with the target a few
blocks away -- the cost, not the connectivity: the store's cells
are interior columns (+2 per column) and wall-adjacent (+5 per
step), so the true cost of the last few blocks is tens of units
and an admissible A* floods every cheaper cell in the town before
paying it (path.rs's own note: "12.0 broke DOORWAYS ... the full
cure is a door-cell exemption at ingest -- queued"). Rings: 29
all-walkable, 8 `###...###`, 3 `#.##.##.#` (posts), 2 `~~~~..~.#`.
The day-1 read confirms the tally; the next row prices the route:
THE DESTINATION'S BUILDING IS NOT A DETOUR (no interior surcharge
and no wall band inside the building the target stands in).

### W15-i4 read: b2 day 1 (a1dc121908, 06:08, hour 0 of day 1)

Exhausted 413 for the day; probes 66 (52 cut_off, 13 sealed, 1
target_unwalkable); the index rebuilt 40 times. Verdicts: `same`
37, `untrusted` 13, `start_unlabelled` 13, `target_unlabelled` 3,
`different` 0; the perimeter class same 37, different 0. CONFIRMED:
the door hypothesis is dead; W15-c prices the route. Arrivals 560,
stuck 1, p95 520 us. Rings 33 all-walkable, 9 `###...###`, 3 posts.
A second class in the same block: unreachable deliveries 39 (0 on
W12-a-c's day 1), UNREACHABLE APPROACH 20 lines, all
from_in_house=true and to_in_house=true, four walkers (146 eight,
147 six, 93 four, 68 two), brief episodes (146: 09:48:06-09:48:36;
147: six minutes; 93: 35 s; 68: once) around the Sleep block --
colonists at home asking for targets in other houses. Colonist 146
at (7706,6310,183) in the house whose box starts at (7690,6300,
180), targets at z 186; the door probe: doors=0, NoDoor -- it
scans z-1..z+2 of the feet, and the house's floor is three above.
W17-i (below) makes the probe scan the whole box and print the
walker's column.

## W17-i, registered 06:15 (keyed on the W15-c stage; the queue's end)

THE SEALED WALKER NAMES ITS FLOOR. Mechanism: at the door probe,
every door sprite in the house's whole box, the nearest by
Manhattan distance including height, its dz, `door_dz_verdict`
(no_door | level | door_above | door_below), the house's z range,
and `column` (feet-3..feet+6 in the frontier glyphs). No
behaviour changes. Pin `the_sealed_walker_names_its_floor` (no
door; two and three up: door_above; two down: door_below; within
one: level); planted: above and below swapped, red. Prediction
(b2 fresh after W15-c's day-1 block, `wait-w17i-b2.sh`; +10 and
day 1): every from_in_house unreachable line carries a verdict;
the sealed walkers' verdict is door_above and their column shows
walkable cells above; falsified if no_door dominates (doorless
worldgen houses: a plot row) or level (the door at the feet's
height and the search still fails: a router row). Rejected:
fixing the drop or the climb before the floor is named; widening
the original scan in place. NOT evidenced: how the walker got
below its floor; b1. The dry tree at e6b5bc7e60 validated the
anchors.

### W17-i landed (ce3bec459e, staged 07:17)

Check clean, the pin green (seven cases), committed 07:03, both
halves staged 07:17:19; the binary verified by contents
(door_dz_verdict and doors_any_z present). The b2 reader restarts
b2 after W15-c's day-1 block and reads +4, +10 and day 1 with the
door verdicts. Falsifier at 07:23: above and below swapped, the
pin RED (0 passed, 1 failed), restored clean. Shipped to lab-bin
07:18. W16-i's chain fired at 07:22.

### W17-i read: b2 +10 (on the W16-i pair e7ad98977a, which contains W17-i; 08:04, hour 13)

Unreachable deliveries 0, UNREACHABLE APPROACH 0, no door probe
yet -- the sealed episodes on the earlier run came around the
Sleep block (hours 22-4). Bans 0, stalls 0, sleepers 0, p95 542.
The day-1 read (~08:45) carries the verdicts.

### W17-i read: b2 day 1 (e7ad98977a; 08:25, hour 0 of day 1) -- the class did not occur

Unreachable deliveries 0 for the whole day (39 on the W15-i4 run
that named the class, 17 on W16-a's, 12 on W15-c's), UNREACHABLE
APPROACH 0, no door probe fired: the sealed-walker episodes are
run-to-run (which colonist sleeps where, which night job asks
from indoors), and this run had none. The instrument is landed
and unexercised; its arm was not reachable in this population on
this day (every gate arm must be reachable -- it was on three
earlier runs and reads when the class recurs; nothing to fix, and
no verdict to dispose of). Elsewhere in the block: exhausted 663
(513 on W15-c's day, 707 on W16-a's: the evening indoor-start
class W15-c-b prices), `same` 36, `start_unlabelled` 21, arrivals
840 (781, 742: +8%), CLIMB BANNED (fetch) 1, FETCH STALLED 4,
starving sleepers 0, stuck 0, p95 481 us. The W16-i b2 reader
took b2 at 08:26 on the W16-i2 pair.

### W17-i read: the class recurred on b2 (the W2-b pair e6ce19724f; 08:39, hour 14) -- door_below

Unreachable deliveries 24 by hour 14, UNREACHABLE APPROACH 18,
every one from_in_house; the door probe fired nine times, every
verdict door_below: colonists 52 and 30 at (7706,6310,183) in the
house whose box spans z 180..210, doors_any_z 2, the nearest door
at (7690,6304,181) TWO BELOW the feet, the walker's own column
'#~~~~~~~~~' -- solid three below, then nine cells neither solid
nor walkable. The body stands at z 183 on a bed sprite two blocks
above its floor at 180; a sprite block is not solid and the cell
above it is not colonist-walkable (no solid directly below), the
router descends one block at a time, so the search from the bed
is an island and answers "unreachable" in its first poll. W17-i
registered three arms (door_above, no_door, level) and the read
gave a fourth; the instrument PASSED (every from_in_house line
carries a verdict and a column) and the column named the class.
W17-b (below) starts the search where the body can stand.

### W16-i read: b2 +4 and +10 (the W2-b pair e6ce19724f; 08:33, 08:39)

+4 (hour 11): CLIMB BANNED (fetch) 1 (credit short_of_prev,
rise_next Some(1), push chaser-settle), exhausted 19, arrivals
324, p95 431. +10 (hour 14): bans 2, both short_of_prev on
colonist 16 (prev two blocks off and one below, rise_next
Some(1)), CLIMB BANNED (other) 4, FETCH STALLED 2, exhausted 40
(30 on W15-c's +10), `same` 17, `start_unlabelled` 10, arrivals
432 (485), p95 527, sleepers 0. No on_prev ban -- the jump class
absent on b2 with W2-b aboard (it was 0-2 here before, so b1
decides W2-b); the two short_of_prev bans are W16-b's class.

## W17-b, registered 08:46 (keyed on the W16-b stage; the queue's end)

THE SEARCH STARTS WHERE THE BODY CAN STAND. Defect: above.
Mechanism: `search_start_stand(walkable, feet)` -- the feet when
their cell is colonist-walkable, else the first cell one or two
below that is, else the feet (identity), the xy kept; both
enqueued searches (the approach search to the trunk's node 0 and
the exact search) start from it; the detour search's start is a
route node already; the mover drops the difference on its own. No
new log line. Pin `the_search_starts_where_the_body_can_stand`
(walkable feet: identity; one below; two below; three below not
tried; nothing standable: identity); planted: the search below
never tried, red. Prediction (b2 fresh after W16-b's day-1 block,
b1 fresh after W16-b's night-1 block; `wait-w17b-b2.sh`,
`wait-w17b-b1.sh`): on b2 unreachable deliveries for the day at
most 5 (24 by hour 14 on this run; 12-39 on the days the class
occurred), UNREACHABLE APPROACH from_in_house at most 5, meals >=
70, sleepers <= 2, arrivals within 20%; on b1 no regression.
Falsified if from_in_house unreachables hold with door_below (the
bed's floor itself is sealed: a plot row) or a new verdict class
appears. Rejected: a two-down move in the router (the descent
note's branching cost, and drops planned for every walker); moving
the body (a teleport); the fall-priced drop edge (the long fix for
cliffs, not for beds). NOT evidenced: beds three or more above
their floor (identity: still an island); the target side. The dry
tree at 1377f60249 with W16-b applied first validated the anchors.

## W15-c, registered 06:01 (keyed on the W16-a stage; the queue's end; common/)

THE DESTINATION'S BUILDING IS NOT A DETOUR. Defect: above (34 of
34 trusted perimeter verdicts `same`, 0 `different`; path.rs's own
note that the wall band "broke DOORWAYS ... the full cure is a
door-cell exemption at ingest -- queued"). Mechanism:
`destination_plot(interior, target_xy, cap)` -- the 4-connected
flood over the founding-computed interior columns from the
target's column, capped at DEST_PLOT_CAP (4,096), empty when the
target stands outside every building; `priced_outside_destination
(in_destination, surcharge)` -- 0.0 inside the plot; `find_path`
keeps its signature (the inline chaser and every vanilla caller
unchanged) and `find_path_priced` takes the plot and applies the
rule to both the interior surcharge and the wall band; the
scheduled colonist search (`FullPathSearch`) computes its plot
once per target column and keeps it across polls; the one-shot
search computes it once. Deterministic (a BFS in a fixed direction
order over a static set). No new log line. Pin (veloren-common,
bastion_vertical_tests) `the_destinations_building_is_not_a_detour`
(outside every building: no plot; inside a 3x3 plot: nine columns,
not the plot five blocks away; the cap holds exactly; inside:
free; outside: paid; the band too); planted: the surcharge paid
inside the destination, red. Prediction (b2 fresh after W16-a's
day-1 block, b1 fresh after W16-a's night-1 block;
`wait-w15c-b2.sh`, `wait-w15c-b1.sh`): exhausted searches on b2
by +10 at most half of 61 and by day 1 at most half of the
previous run's; the perimeter probes' `same` class at most a
quarter of 34; unreachable 0; arrivals within 20%; ITEM 39 p95_us
at most double. Falsified if exhaustion holds with `same` still
the majority (the budget itself, or the walker's own start
building's surcharge) or p95_us more than doubles. Rejected: a
door-cell exemption at ingest (the door is not the obstacle);
raising the Longest budget (a bigger flood); an inadmissible
heuristic; a radius exemption around the target (a walker would
cut through the neighbour's house on its last twenty blocks). NOT
evidenced: routes whose start is inside a surcharged building; the
inline chaser searches; b1's stores. The dry tree at e6b5bc7e60
with W16-a applied first validated every anchor.

### W15-c read: b2 +4 and +10 (on the W17-i pair ce3bec459e, which contains W15-c and W16-a; 07:28 and 07:34)

+4 (hour 11): exhausted 22, probes 22 (14 sealed, 7 cut_off), the
perimeter class `same` 3, `start_unlabelled` 11, `untrusted` 7;
CLIMB BANNED (fetch) 0, FETCH STALLED 0, arrivals 363, p95 445 us.
+10 (hour 14): exhausted 30 (bar <= 30, half of 61; 61-87 on the
three previous runs at +10) -- PASSED at the edge; the perimeter
class `same` 4 (bar <= 8, a quarter of 34) -- PASSED;
`start_unlabelled` 18 now the largest class (the remaining
exhaustions start from cells the index has not labelled:
colonists indoors, the W17-i class); CLIMB BANNED (fetch) 0 (2 on
W16-a alone at +10), FETCH STALLED 1 (2), PROMISED CLIMB 1, FETCH
BUDGET EXPIRED 4 (1); arrivals 485 (425 on W16-a alone, 394 on
W15-i3 at +10: +14% to +23%); ITEM 39 p95 420 us (494, 460-463:
the lowest of any run; the flood was the tick's cost); starving 0;
stuck 0; the pump pending 31, delivered_exhausted 0 in the last
census. Reading: W15-c does what it registered -- the searches
that flooded now finish -- and the b1 bans are not seen here: no
ban at all in fourteen game hours on the arm whose store lies on
flat ground. The day-1 read (~08:15) decides the day's bars.

### W15-c read: b2 day 1 (ce3bec459e, 07:50, hour 1 of day 1) -- FAILED the day bar, on the class it named as its falsifier

Exhausted 513 for the day (bar <= 354, half of W16-a's 707; 413
and 366 on the earlier runs) -- FAILED; the perimeter class
`same` 15 (bar <= 8) -- FAILED; `start_unlabelled` 38 of 66
probes, now the largest class by far (18 at +10, 11 at +4). The
+10 read had passed both bars (30 exhausted, `same` 4); the day's
483 further exhaustions came in the afternoon, evening and night
-- the supper hauls to private shelves and the night's own-house
meals, searched FROM inside a house: the start's building pays
the interior surcharge and the wall band on the way out, exactly
the clause W15-c registered as NOT evidenced and named as its
falsification ("a surcharge outside the destination -- the
walker's own start building -- and the next row prices that").
Outcomes: arrivals 781 (742 on W16-a alone: +5%), CLIMB BANNED
(fetch) 1, FETCH STALLED 8, FETCH BUDGET EXPIRED 13, STALL BLAMED
3, starving sleepers 1, stuck 0, p95 527 us, unreachable 12.
Disposition: W15-c PASSED on the class it priced (the store as a
destination: the daytime flood is gone on both arms) and FAILED
its day bar on the start's building; W15-c-b (below) prices both
endpoints' plots. The W17-i b2 reader took b2 at 07:52.

### The first W16-i verdicts: b1 (e7ad98977a, 07:47, hour 10 of day 0)

Six CLIMB BANNED (fetch) by hour 10, five of them on_prev -- the
body standing on the very node the chaser credited, (7647..7657,
6433, 180) -- with the route head one block north and two up,
(x, 6432, 182), prev_dz 0, rise_next Some(2), push_site
chaser-settle: a real two-block ledge into the building at
y=6432, a JUMP edge the router plans and the gliding body cannot
take; the sixth short_of_prev (dxy 2, dz -2: colonist 103's
two-step stair, the body two blocks short and two below the
credited node). The credit verdicts name the class the b1 bans
belong to: not a stair credited from below (W16-a's), not the
settle (one case), but the router's jump edges. W2-b (below)
removes them for gliding bodies.

By hour 21 on b1 (08:06): 24 bans, credit on_prev 19 and
short_of_prev 5, rise_next Some(2) 22 and Some(1) 2, push_site
chaser-settle 23 and chaser-probed 1, head-feet (dz 2, dxy 1) 19,
(2, 2) 3, (2, 3) 2; heads at (7673..7674, 6436..6437) -- colonist
103's stair building -- and (7661..7664, 6431..6432) -- the y=6432
ledge; FETCH STALLED 38, budgets expired 33, PROMISED CLIMB TAKEN
1. The class is one class: a two-up edge the router plans and no
mover takes. (The readers' credit tally had missed the quoted
value; fixed and relaunched at 08:08 before any read.)

## W2-b, registered 07:53 (keyed on the W16-i2 stage; the queue's end; common/)

THE FETCH LEG PLANS NO JUMP. Defect: above. Mechanism:
`jumps_admitted(scramble_reach, on_land, can_climb, can_fly)` --
false at reach 1, otherwise the rule as it was (on land, or a
climber, or a flyer); the neighbour closure asks it; reach 1 now
names a colony body that glides (the stair floor and the ladders
stay, the jump edges go, the scramble edges were never its); the
scheduled colonist search (the one bastion-server config with
reach 2) asks for reach 1; reach 0 (vanilla) and 2-3 (the inline
chaser configs) plan exactly as before. Pin (veloren-common,
bastion_vertical_tests) `the_fetch_leg_plans_no_jump` (six
cases); planted: reach 1 admitting jumps, red. Prediction (b1
fresh after W16-i2's night-1 block, b2 fresh after W16-i2's day-1
block; `wait-w2b-b1.sh`, `wait-w2b-b2.sh`): on b1 CLIMB BANNED
(fetch) <= 3 by hour 19 (15; 6 by hour 10), FETCH STALLED <= 24
(40), FETCH BUDGET EXPIRED <= 15 (34), arrivals >= 550 (601),
sleepers <= 2, unreachable at most 20 more than the previous run;
on b2 no change (bans 0-2). Falsified if the bans hold on b1 (the
two-up edges are not the jump set's) or unreachable rises by more
than the bans fell (the ledge building has no other way in: a
door or plot row). Rejected: the assist for trunk walkers (W6-C);
a new config field; pricing jumps instead of removing them;
removing jumps at reach 2-3 (the assist can lift those walkers,
and Ben's climbing skill lives there). NOT evidenced: targets on
a ledge with no stair (now honestly unreachable); the trained
climber's scrambles; the trunk mover's own step. The dry tree at
e7ad98977a with W16-i2 applied first validated every anchor.

A stale `w2b-staged.txt` from 2026-09-02 (an older row of the
same id) pre-satisfied this row's falsifier (it ran at once
against the old hash and failed harmlessly) and both readers
(they passed their MARK wait); caught at 07:54 within a minute,
the marker moved aside, the readers killed by their pid files and
all three relaunched. Filed: a stale stage marker pre-satisfies a
reused row id.

### W2-b landed (e6ce19724f, staged 08:26)

Check clean, the common pin green (1 of 713), committed 08:16,
both halves staged 08:26:43 with the client compiled fresh
against common; the scheduled search's config carries reach 1.
The b2 reader restarts b2 after W16-i2's day-1 block, the b1
reader after W16-i2's night-1 block. Falsifier at 08:30 (its
own detached worktree): reach 1 admitting jumps, the pin RED (0
passed, 1 failed of 713), restored clean. Shipped to lab-bin
08:27. W15-c-b's chain fired at 08:31. The W16-i b2 reader,
restarting b2 at 08:27, took this pair: b2's next reads carry
W2-b.

## W15-c-b, registered 07:59 (keyed on the W2-b stage; the queue's end; common/)

THE START'S BUILDING IS NOT A DETOUR EITHER. Defect: W15-c's day
on b2 (above): 513 exhausted with `start_unlabelled` the largest
class -- the evening and night searches start indoors and the
start's building pays the surcharge and the band on the way out
(W15-c's own NOT-evidenced clause and falsification). Mechanism:
`endpoint_plots(interior, start_xy, target_xy, cap)` -- the union
of both endpoints' plots; the scheduled search keys its cached
plot on (start, target); the one-shot search computes the union
once; the surcharge rule unchanged. Pin (veloren-common)
`the_starts_building_is_not_a_detour_either` (a start in one
plot and a target in another: eleven columns; a start outside:
nine; both outside: none; the same plot: nine once); planted: the
start's plot dropped, red. Prediction (b2 fresh after W2-b's
day-1 block, b1 fresh after W2-b's night-1 block;
`wait-w15cb-b2.sh`, `wait-w15cb-b1.sh`): on b2 exhausted at most
half of 513 for the day, `start_unlabelled` at most half of 38,
`same` at most 8, arrivals within 20%, p95 at most double; on b1
longest-tier steps under 5,000 by hour 19. Falsified if the
evening exhaustion holds with `start_unlabelled` still the
largest class (the indoor start's cost is not the surcharge: the
component index or a sealed floor -- W17-i's class). Rejected:
exempting every interior for indoor starts; raising the budget;
an inadmissible heuristic. NOT evidenced: start and target in
different houses (both free; the way between still priced through
a third house); b1 (one replicate on b2 first). The dry tree at
83e3666cb6 (W16-i2) with W2-b applied first validated every
anchor.

### W16-b read: b2 +4 / +10 (f408e4b8a9 under W2-b's b2 reader; 09:33 hour 10, 09:40 hour 13)

+4: exhausted 19, unreachable 1, bans 0/1 (other), stalls 0, arrivals
280, p95 438. +10: exhausted 35 (47 at +10 on 1377f60249, 30 and 40
before), components start_unlabelled 21 / same 7, unreachable 1,
CLIMB BANNED (fetch) 0 by hour 13 and still 0 at hour 17, other 1,
stalls 0, arrivals 421 (536 at hour 15 on the run before: the clock
sits two hours earlier at this +10), p95 457, starving 0. W16-b's
own class (on_prev with rise_next Some(1), push_site chaser-settle)
has not occurred yet; the day-1 block reads it. This pair still
carries W2-b, so the day's exhaustion is confounded as before.

### W14-c registered (09:52): THE EXHAUSTED SEARCH STRIKES THE JOB

The re-ask loop's consumer. The pump's Fill lane on BudgetExhausted
writes the start-keyed memo and drops the path_cache; the walker
keeps the job, moves (the glide, the move assist), misses the memo
(why=start; refusals saturate at 32,768, misses at 4,096) and asks
the whole-town search again. The Approach lane's proven Unreachable
already strikes the held job (three strikes: UNREACHABLE PROVEN,
benched; the arbitration latch clears benches every 40 intervals).
Mechanism: `job_strike(strikes) -> (next, bench)` with
`UNREACHABLE_STRIKES` 3, shared by the approach arm (identity) and
the fill arm, which strikes only at `PathLength::Longest` (the
75,000-iteration tier: a proof at the town's scale) and logs "three
exhausted fill searches" on the bench with uid and target. Pin
`the_exhausted_search_strikes_the_job`; falsifier plants the
threshold at 30. Bars, b1: the most-repeated exhaust end <= 6 a day
(10 before W2-b, 37-52 under it), LONGEST-TIER lines <= 1,000
(1,642 before W2-b), fill benches >= 1 whenever an end repeats
three times and <= 15 a day, arrivals >= 700, starving <= 1; b2:
benches <= 15, arrivals >= 760, exhausted <= 520. Falsified if the
top end repeats >= 10 with zero fill benches (the re-asker is
another lane) or benches exceed 15 (reachable work benched).
Chain behind W2-b-r (stage ~10:15 -> W14-c ~10:45).

### W17-b landed (e5c4d9965d, staged 09:47, shipped to lab-bin 09:48)

Check and pin green, both halves fresh; the falsifier planted the
search-below never tried at 09:48 and the pin went RED at 09:51,
the tree restored clean (0 dirty). The b1 restart
at 09:45 (W2-b's reader) took f408e4b8a9, the latest pair at that
moment; e5c4d9965d boards at the next restart on each arm.

### W2-b read: b1 night 1 (1377f60249; 09:42)

CLIMB BANNED (fetch) 9 for the day (15 on W16-a's b1 day, 24 on
W16-i's: W2-b did cut the class by half or more, short of its bar
of 3), other 4, PROMISED CLIMB TAKEN 3, FETCH STALLED 18, budgets
expired 19, starving sleepers 0; arrivals 915 (933 on W15-c's
night: -2%); exhausted 15 delivered; LONGEST-TIER steps 57,826; p95
642; stuck 8 distinct. The revert (W2-b-r) gives the ban reduction
back knowingly: the bans have a witness and a consumer (the shun);
the floods had neither. Recorded as W2-b's benefit to be regained
by the ramp row and the shun-on-exhaustion row.

### W2-b DISPOSED: REVERTED (09:45) -- the flood named; W2-b2 WITHDRAWN

The b1 log under 1377f60249 by hour 19: LONGEST-EXHAUST NEIGHBOURHOOD
617 lines (13 on the W16-i run e7ad98977a, the last b1 run before
W2-b), LONGEST-TIER SEARCH 51,798 (1,642), and every exhaust read
`expanded_states=60696 distinct_cells=60696` with a bbox of 324 x 307
blocks: the whole town, flooded per ask. The top exhaust (37 times):
end (7742,6404,181), closest reached (7725,6404,181) 17 blocks short,
the frontier +x Wood -- colonist 55 (a DepositRun to destination 59,
then a haul) behind a wall whose way round needs a two-up edge the
reach-1 trunk no longer plans; it re-asked 1,675 times (MOVE ASSIST
DID NOT STICK repeats=14 between asks; the memo missed why=start
because the body glided between asks). Before W2-b the trunk planned
the jump, the body stalled, the fetch was banned and the walker shun
(E2-s) sent the pick elsewhere: cheap, local, self-limiting. After
W2-b the search itself was the failure, exhaustive and unwitnessed
by any consumer. Arrivals were flat (733 vs 736) so the outcome hid
it; the mechanism counters did not. Also seen: 101 of the 617
exhausts print closest == end (the end cell expanded and the search
still exhausted; `satisfied` is `node.pos == end`) -- a W15-i5
question, not chased here. Disposal: W2-b FAILED its bar and
REGRESSED the search load 30-47x on the ledge arm -> reverted by
W2-b-r (below). W2-b2 (the chaser's leg at reach 1 for trunk
walkers) was premised on the reach-1 trunk and is WITHDRAWN before
its chain fired (killed by pid file 09:44; its scripts and
registration stay in the scratchpad and above for the record). The
`jumps_admitted` rule and its pin stay (reach 1 still means "no
jump"; nothing asks for reach 1 now). Memory filed:
removing-an-edge-class-turns-bans-into-whole-map-floods.

### W16-b read: b1 hour 19 (f408e4b8a9 under W2-b's b1 reader; 10:05) -- PASS, and the flood's second replicate

CLIMB BANNED (fetch) 1 by hour 19 on the ledge arm (8 on the W2-b
run, 15-24 on the runs before): colonist 65, on_prev, rise_next
Some(2), push_site chaser-probed, frame trunk, trunk_dxy 1. Other
bans 4, PROMISED CLIMB TAKEN 1, FETCH STALLED 4 (12), budgets
expired 8 (15), STALL BLAMED 2, starving 0; arrivals 790 (733 on
the W2-b run, 736 on W15-c's: +7%); stuck 5 distinct; p95 726. The
ledge tally puts the two old ledges in the STALL column now instead
of the ban column: stalls near +2-edge heads 2, clusters
(7664,6432) 2 and (7640,6504) 2 -- the bodies still stop there, but
the fetch stalls and the shun answers it rather than a climb ban.
PASS on the ban bar; the ledge itself stands (a ramp is the plot
row). The flood: LONGEST-EXHAUST 10 (617 on the W2-b run, 13 before
W2-b), LONGEST-TIER lines 1,346 (51,798 / 1,642), whole-town floods
10, top ends 5x(7721,6346,188), 3x(7706,6310,181) (the W17-i sealed
house), 2x(7741,6393,180). This pair STILL CARRIES W2-b, so the
flood is not a fixed cost of reach 1: it needs a claimed job behind
a two-up-only barrier (colonist 55's DepositRun to destination 59
on the W2-b run) and the re-ask loop; today no such job was
claimed. The W2-b-r revert stands as a removal of that latent
60,000-cell-per-ask risk (the closest-frontier evidence was a wall
the reach-1 graph cannot pass), and W14-c makes the loop
self-limiting either way; but the flood has ONE replicate, and the
record says so. The next question is whether W2-b-r brings the bans
back with W16-b aboard (its read, ~11:15).

### W16-b read: b2 day 1 (f408e4b8a9 under W2-b's b2 reader; 10:02)

CLIMB BANNED (fetch) 0 for the whole day (2 on the day before, 2-6
on the days before that), so W16-b's own class (on_prev with
rise_next Some(1), chaser-settle) is 0 of 0; other bans 5
(sleepers), FETCH STALLED 1 (5), budgets expired 3 (8), STALL
BLAMED 0, starving 0; arrivals 808 (846: -4.5%, within the 10%
bar); p95 659 (508). PASS on b2 for the ban class. Two counters
moved the other way and are recorded against it: exhausted (pump
census sum) 1,043 (598 the day before under W2-b + W15-c-b, 513 the
day before that: outside the two-day band), and unreachable
deliveries 24 / UNREACHABLE APPROACH 18, all from_in_house (W17-b's
class, absent the day before, present today; W17-b is not on this
pair). The exhaustion doubling has two candidates: W2-b's flood
growing with the day's job mix (this pair still carries W2-b), or
W16-b's walking bodies searching from new places. The b2 arm
cannot read the flood tally (its LONGEST-TIER diag is not set on
b2: 0 lines), so the separation comes from the next b2 day that
carries W2-b-r and W16-b without W2-b: exhausted back near 500 =
W2-b's flood; still near 1,000 = W16-b's, and W16-b is then
re-examined. The W15-c-b reader restarts b2 now onto e5c4d9965d
(W17-b, still with W2-b); W2-b-r boards at the restart after it.

### W2-b-r landed (d4760fa9fd, staged 10:09:54, shipped to lab-bin 10:10)

Check and pin green on the chain (1 test passed), both halves built
fresh, the binary verified by its contents ("THE TRUNK'S REACH"
present once in stage-bin and in lab-bin). The falsifier planted
reach 1 at 10:11 and the pin went RED at 10:14, the tree restored
clean (0 dirty); W14-c's chain fires at 10:15.

### W16-b read: b1 night 1 (f408e4b8a9; 10:23) -- PASS

CLIMB BANNED (fetch) 1 for the whole day (9 on the W2-b day, 15 and
24 on the two days before): colonist 65 once, on_prev, trunk frame;
other 7 (sleepers), PROMISED CLIMB TAKEN 1, FETCH STALLED 6 (18),
budgets expired 10 (19), STALL BLAMED 2; arrivals 1,013 (915 on the
W2-b night, 933 on W15-c's: +8-10%); starving 1 (colonist 8 at hour
6, hunger 0.04, EatFrom Traveling -- on its way to eat, bar <= 1);
stuck census 18 distinct (8 the day before; a night rise to watch);
p95 732. Flood tally for the day: LONGEST-EXHAUST 21 (617 / 13),
whole-town 21, LONGEST-TIER lines 2,315 (51,798 / 1,642), top ends
6x(7667,6200,182), 5x(7721,6346,188), 4x(7706,6310,181). Ledge
tally: bans under a +2 edge 1 (trunk), stalls near +2-edge heads 2,
clusters (7664,6432) 2 and (7640,6504) 2. W16-b PASSES on both
arms: the ledge class on b1 went 24 -> 15 -> 9 -> 1 across W16-a,
W2-b and W16-b, and the two ledges now cost a stall each a day.

### W14-c landed (5f785e2a5a, staged 10:33:34)

Check and pin green on the chain (1 test passed), both halves built
fresh, the binary verified by its contents ("three exhausted fill
searches" present once in stage-bin, the trunk witness still there).
The falsifier planted the threshold at 30 at 10:35 and the pin went
RED at 10:38, the tree restored clean (0 dirty); shipped to lab-bin
at 10:34. The readers restart b1 after W2-b-r's
night block and b2 after W2-b-r's day-1 block; the shipper moves
the pair to lab-bin when Ben's server is down.

### W2-b-r replicate 2: b1 night 1 (5f785e2a5a; 11:44) -- the W2-b day's flood, at reach 2

LONGEST-EXHAUST 558 for the day (whole-town 556), LONGEST-TIER
lines 49,593, top ends 57x(7693,6457,182), 39x(7734,6404,181),
35x(7736,6398,180): the W2-b day (617 / 51,798) reproduced with the
jump edges planned. CLIMB BANNED (fetch) 3, other 42 (colonist 110:
19, 137: 15), FETCH STALLED 20, budgets expired 18, PROMISED CLIMB
TAKEN 1, arrivals 945 (977 on replicate 1; 1,013 on the reach-1 day
with W16-b; 915 on the W2-b day), stuck 12, starving 0, p95 717,
route-proof benches 42. The reach-2 + W16-b pair now has two full
b1 replicates: floods 33 and 558, loops 154 and 42, arrivals 977
and 945; the reach-1 + W16-b pair has one: flood 21, loops 7,
arrivals 1,013. The flood is the chaser's re-ask loop and W14-d is
its consumer; the loops are the wall-climb release loop and W6-D
is theirs; the decision day (the first full b1 day with both
aboard) is what the registered rule reads.

### The shipped pair's second b2 day (5f785e2a5a under the W17-b reader; 11:40)

Arrivals 835 (641 on replicate 1: that drop was the day's mix, not
the pair; 807-846 on the three days before), exhausted 748 (844;
513-598 before W16-b: W16-b's cost stays at roughly +40% on this
counter, against its benefits on bans, stalls and starving), probes
67 (55 cut_off, 12 sealed), components start_unlabelled 27 / same
20 / untrusted 11 / target_unlabelled 7; CLIMB BANNED (fetch) 0,
other 1, FETCH STALLED 3, budgets expired 6, unreachable 0 all day
(W17-b's class 0 for the third day), starving 0 all day (no
STARVING COLONISTS line), p95 484; benches 0 of every kind (W14-c
dead as found; no route-proof bench either). The b2 arm is quiet on
this pair; the trunk-reach question is the ledge arm's.

### W2-b-r replicate 2: b1 hour 19 (5f785e2a5a under the W16-b reader; 11:28) -- its flood bar FAILED

LONGEST-TIER steps 37,002 (2,674 on replicate 1; 40,932 on the W2-b
day; 1,230 on W16-b's reach-1 day), LONGEST-EXHAUST 493 by hour 21
(33 / 617 / 21), whole-town 491, top ends 34x(7734,6404,181),
28x(7693,6457,182), 23x(7615,6271,182). The registered bar
(LONGEST-EXHAUST <= 30 by hour 19) FAILED; the revert's flood
attribution is falsified on this replicate, as recorded above.
CLIMB BANNED (fetch) 3 (all trunk under a +2 edge, chaser-refused-
rock), other 21 by hour 19 and 39 by hour 21 (colonist 110: 19,
137: 15 -- the loop class again, smaller than replicate 1's 154),
FETCH STALLED 19 (11 / 6), budgets expired 16, arrivals 728 (772 /
790 / 733), stuck 2, starving 0, p95 647; ledge tally: stalls 20,
clusters (7664,6432) 6, (7648,6388) 6. Two reach-2 replicates now
stand against one reach-1 replicate with W16-b: reach 2 reads
floods 33 and 493 and loops 154 and 39; reach 1 read flood 21 and
loops 7 with arrivals 1,013. Both consumers (W6-D for the loops,
W14-d for the floods) are aboard from the W14-d pair on; the
decision rule's day is the first full b1 day on that pair.

### W14-e replicate 2: b1 night 1 (203321df48 under the W14-c reader; 14:04)

LONGEST-EXHAUST 729 for the day (whole-town 729), LONGEST-TIER
lines 63,087, top ends 64x(7734,6411,181), 56x(7752,6393,180),
41x(7632,6281,182), benches by Longest exhausts 0 (the reset
defect; W14-e2 boards b1 now), route-proof benches 5; CLIMB BANNED
(fetch) 3, other 2, PROMISED CLIMB TAKEN 3, FETCH STALLED 27 (the
day's high), budgets expired 29, STALL BLAMED 4, stuck 19; arrivals
943; starving sleepers 2 (colonists 70 and 105: E2-t's case, the
posted haul's reservation on the sleeper's supper); p95 668. b1
restarts onto dfa366b6db (W18-b + W14-e2 + W18-i) under the W6-D
reader: the flood fix's and the pit fix's first ledge-arm day, its
first-hour witness read by hand at hour ~12 (~14:40).

### W18-i read: b2 day 1 (0d603edbae under the W14-d reader; 14:03) -- PASSED, and the clock is blind

THE BODY BOBS: 19 lines; colonist 75 named with bobs=128 at cell
(7632,6317) (eight of the lines at that cell), colonists 118 and
58 at 2 and 1; at bobs 64 and 128 the active job's stuck_time read
0.033 s -- the stuck clock sees a bob as progress (the drop shortens
the distance to a target below or beyond the edge, the climb back
adds under the epsilon) and never fires; the same body bobbed 242
times at another cell two pairs ago. The instrument's bar (a
100-plus-drop colonist named with bobs >= 64 and its stuck_time;
lines < 200) PASSED. The day: arrivals 868, exhausted 1,064, CLIMB
BANNED (fetch) 1, other 1, FETCH STALLED 1, budgets expired 4,
starving sleepers 0, W14-d's benches 5 (jobs 516, 564 and three
more: the stuck-branch consumer fires on b2), p95 524. Colonist
75's cell is a terrace edge with a way up (it climbs back each
time), so W18-b's refusal will not touch it: the bob's own consumer
is the next candidate -- THE BOB IS NOT PROGRESS (the stuck clock
measured on xy, or a bob counted as no displacement, so the stall's
consumers act) -- registered after W18-b's first read.

### W18-b2 on b2: the day PASSED (reader's day-1 block at 16:01, hour 0)

Starving sleepers 0 (bar 0; 3 under W18-b), FETCH STALLED 3 (bar <=
6; 19), arrivals 736 (bar >= 720; 744 the day before), POS-WRITE
drops at (7712,6306) 0 (bar 0), THE DROP INTO THE OPEN IS ALLOWED
opened=256 (bar >= 1), THE DROP HAS NO WAY UP 0 (the rim not reached
on b2 today; refused three times on b1 at a one-cell hole), bobs
peak 2 (W18-c aboard; 256 the day before without it), climb bans
0/0, W14-e2 benches 3, terminal 2, budgets expired 13, panics 0.
Every registered bar held on its first b2 day. Not falsified: no
body dropped into the pit, no sleeper starved with the Open witness
at its ledge. One replicate; b1's day (booted 15:46) is the second
world, its night at ~16:30. W18-b stands superseded.

### W18-b2 on b2, the Sleep block by hand (15:57; 5c8285fb34 since 15:26, hour 21)

STARVING lines 0 (3 farmers at 0.00 at this hour two days ago under
W18-b), FETCH STALLED 3 (19; bar <= 6: PASSING), shuns 13, THE DROP
INTO THE OPEN IS ALLOWED opened=256 (sampled lines at (7754,6343/
6344,181) x9, (7775,6390,182), (7726,6362,185), (7718,6346,181)), THE
DROP HAS NO WAY UP 0 (the pit's rim not reached today), POS-WRITE
drops at (7712,6306) 0 (bar: PASSING), bobs peak 2 (W18-c aboard;
256 on this arm yesterday without it), NIGHT SHELF EMPTY 0, arrivals
678 at hour 21 (bar >= 720 by day's end). Yesterday's three starvers:
17 ate at hour 18, 20 at hour 20, 49 not starving. At hour 23
(16:00): STARVING lines still 0, arrivals 727 (bar >= 720: PASSED),
stalls 3, Closed 0, pit drops 0, bobs 2 -- every W18-b2 bar on b2
PASSED by hand; the reader's day-1 block (~16:15) closes the read.
On b1 at hour 14 the pit's rim was reached and refused three times
(THE DROP HAS NO WAY UP, Closed) -- the guard still guards the pit:
uid 125 at landing (7648,6446,181), cells=1, a one-cell hole. b1's
stalls at hour 14: 12 (23 at hour 12 yesterday), all Designated
walks, clustered at (7636,6504) x4, (7664,6432) x3 and (7648,6388)
x3 -- feet at (7665,6433,180) and (7637,6505,181) sit at the FOOT of
a ledge, not its head: bodies wanting to go UP a two-block step the
trunk plans as a jump (reach 2) and the gliding body cannot make --
W2-b's original class and the open judgement item (climbing as a
skill vs a mover that cannot jump), not W18-b2's. The down-direction
holds are gone. Desktop probe at 16:00: 0x0 (the eighth; locked).

### CORRECTION (16:41): my "pit drops 0" hand reads were blind; the pit bar reads differently

POS-WRITE prints float positions (x: 7712.77, y: 6306.13); my hand
pattern matched integers (x: 7712, y: 6306) and returned a silent
zero on every read today. Float-aware, b2's second W18-b2 day had
TWO mover drops into the pit cell -- uid 125 at 20:34:56 UTC and uid
137 at 20:40:15, z 181 -> 179 at (7712.8, 6306.1) -- and one assist
lift out (uid 129, z 179 -> 181). W18-b2 did not refuse them: by the
rule the landing is not a Closed basin (Open or WayUp), which the
pin's nine-cell pit never was. Outcome: uid 125's next logged write
came 47 s later 45 blocks away at (7755,6343) -- it walked out; no
bob (peak 1), no starving. So W18-b2's REGISTERED pit bar ("drops 0")
FAILED as a mechanism bar on b2's second day, while its outcome bars
(no bob, no starver, no held body) held; and every earlier "pit
drops 0" I wrote for W18-b and W18-b2 was the instrument's zero, not
the town's -- the readers' own DROPS line ("mover drops at the pit
cell 7712/7713,6306") is float-aware and is the number of record
(it read 1 at 16:38, before the second drop). The pit as a trap is
closed by W18-c (a bobbing body stalls) more than by W18-b2 (which
refuses only basins the walk cannot leave); the pit itself is not
such a basin. Lesson filed under the silent-zero memories.

W14-i6's falsifier: RED at 16:41 (the bound off by one in its own
worktree: 0 passed, 1 failed; restored, 0 dirty).

### W18-b2 on b2, the second day (c288d55479 since 16:05; read by hand 16:38 at day 1 hour 0): PASSED again

Arrivals 946 (736 the first day; bar >= 720), STARVING lines 0,
starving sleepers 0, FETCH STALLED 1 (3), bobs peak 1, Open 512,
Closed 4, POS-WRITE drops at (7712,6306) 0, NIGHT SHELF EMPTY 0,
supper yields 0 (E2-t's case did not arise), W14-e2 benches 1 job,
LONGEST-EXHAUST 644 (far 524, touched 120 -- the flood is the day's
only bad number and is W14-g's and the plot geometry's). Two b2
days and one b1 day under W18-b2, every bar held; the arrivals
swing 736 -> 946 on the same rule is the 2-3x variance the law
names, not a mechanism.

### The far class measured (16:37; b2 with the diag, hour 23): 502 far, 120 touched

end_z minus closest_z over the 502 far exhausts: +2 in 184 (37%),
0 in 121 (24%), -1 in 108 (22%), +1 in 81 (16%), -2 in 5. Far ends'
z: 182 in 417, 183 in 42, 181 in 23, 180 in 17. House sprites
(chair, table, bed, door, window, bench) on the closest node's
blocked neighbours in 118 of 502. So "an upper floor two above the
reach" is the plurality, not the class: a quarter are on the SAME
level three cells away and never entered, and a third one level
off. All share one property -- the end is walkable, its lateral
neighbours are walkable, and no admitted edge leads from the
reachable set into it -- which is the graph being cut at the last
one to three cells: a held door (DOORS HOLD LONGER: 60 s a wall to
the router), a fence or window band, or the interior jump's
headroom. The frontier's own blocks and sprites (16:37): Air/Empty
663 (no floor: the reach ends at a drop or a rise -- F1 fall marks
144, F2 6, F3 3), Grass 283, Wood 194, Rock 138, Earth 57;
sprites ChairWoodWoodland2 184, FenceWoodWoodland 123, Lantern 4,
Window1 1; DOORS 0 of 508. So the cuts are (a) a fence line the
town rule refuses to cross (a fenced plot whose target sits inside
and whose gate the router does not admit), (b) furnished rooms
(chairs on the frontier: a table's far side, an upper floor), and
(c) a floor edge one or two blocks up. Not doors. W14-i7 names the
walkers and their jobs; the plot rows (gates in fences, stairs
routable) are Ben's roadmap. Rate: 622 whole-town exhausts in 32
minutes on b2 -- the largest CPU cost in the town.

### W14-i6 landed (e4351e2457, staged 16:35:49)

Chain bla01mpua: check ok, pin the_proof_was_false green, committed
e4351e2457, both halves from one commit, the server exe carries
"THE PROOF WAS FALSE" (grep 1). Falsifier at +90 s (verdict below
when it prints). The live read: the B5 arrival line's strikes= and
benched= fields and the false-proof witness on the arms' next
restarts (b1 boarded W14-w ten seconds before this stage; b2 boards
the latest pair at ~17:00). W14-g's chain fires at +300 s.

### W18-c's third day on b1 (W14-w pair; read by hand 17:04 at day 1 hour 0): the outcome bars hold, the arrivals question needs its own baseline

Starving sleepers 0 (STARVING lines 16, all colonist 53 awake and
walking to eat after a shun), FETCH STALLED 16, STUCK CENSUS 6,
bobs peak 2, fetch bans 5 / other 4 (the ledge-foot jump class),
flood 18 (the quiet day), bench lines 2 = distinct jobs 2 (W14-w's
unit holds all day), NIGHT SHELF EMPTY 0, supper yields 0. Arrivals
839 at the hour-0 frame: the three W18-c days read 828, 863, 839
there against ONE W18-b-only day's 991 -- a consistent ~13% below
one comparison day, which is not a baseline (COUNTS VARY 2-3x;
three replicates or nothing applies to the baseline too). At the
registered hour-6 frame the two prior days read 889 and 916; the
reader's block (~17:10) gives the third. Disposal stands: W18-c's
mechanism works (bobs 128-256 -> 2, the held bodies stall and the
consumers act, no starver on the W18-b2 days); its arrivals cost is
an open WATCH with a baseline to collect, not a verdict.

Float-aware pit count on b1 today: 23 mover writes into the pit's
columns at z 179 -- bodies step in and out (the assist lifts them;
bobs peak 2; no starver). W18-b2's "drops 0" bar fails on b1 as
on b2 for the same reason: the pit is not a Closed basin by the
rule, and W18-c is what keeps a body from bobbing in it.

### W14-w2 registered (17:12): THE BENCH HAS ONE DOOR (instrument honesty)

Mechanism: SilentBench { StuckTimeoutRelease, SelfRescueNoAccess,
NotExposed } + silent_bench_label(reason); each of the three silent
writers (39178, 44776, 48401) now tests bench_is_new(true,
job.unreachable), sets the flag as before, and prints "UNREACHABLE
PROVEN -- job benched off the board (silent writer named)" with the
job and its reason. The five strike sites are unchanged; no state,
gate or route changes. Pin the_bench_has_one_door (three labels,
non-empty and distinct; the door with bench = true); falsifier
empties the stuck-timeout label -> red. Bar (the next days): bench
lines by label sum to the distinct benched jobs per latch period;
on b2 the silent-writer lines number at least the arrivals with
strikes >= 3 and no strike-site bench line (four today); the
stuck-timeout label leads. Falsified as an instrument if arrivals
still carry strikes >= 3 with no bench line of any label. Rejected:
one JobBoard::bench method for all eight writers (five sites hold
&mut Job while the board is borrowed); removing the silent benches
(they are the town's design). Queued behind W14-i7 (bastion-server).

### W14-i6 on b2, the first day to hour 18 (17:08; e4351e2457): FOUR false proofs, and strikes without a bench line

THE PROOF WAS FALSE x4: job 539 (Haul to (7662,6370,182), colonist
63, strikes 4), job 328 (Designated at (7721,6354,184), colonist
156, strikes 3), job 676 (Haul to (7704,6347,182), colonist 144,
strikes 3), job 663 (Haul to (7640,6367,181), colonist 21, strikes
4). Eleven arrivals carried strikes >= 1 (1: 3, 2: 4, 3: 2, 4: 2);
benched=true at arrival 0. Bench lines all day: ONE (a terminal
chaser bench) -- so four jobs reached three or four strikes with no
bench witness: either a writer sets unreachable without a witness
(the bench_is_new gate then hides the transition) or the strikes
came from an arm whose witness did not print; the grep below names
the writers. Three of the four false proofs are Hauls to STORE
cells at z 181-182 -- the same z as the far class's ends -- and the
walkers arrived (arrive_dist 2): the search cannot satisfy a store
cell it may not enter, the walker never has to. If W14-i7 shows the
far ends are store and shelf cells, the far class is not plot
geometry but a target cell the search demands exactly while the
job only needs adjacency -- GENERATOR AND CONSUMER MUST AGREE on
what "reached" means. Flood 509 by hour 18 (far 359, touched 150);
stalls 0; arrivals 663.

The silent benches, found (17:10): three writers set
job.unreachable = true with no witness and no strike -- the
stuck-timeout RELEASE path (bastion_jobs.rs 39178, "retries are the
mechanism": every non-haul stuck timeout benches the job silently),
the self-rescue's None arm (44776: no auto-access could be emitted),
and the exposure check (48401: a designation with no open face). So
W14-w's "one witness per bench" covers the five STRIKE sites only;
a job benched silently first never prints its later third strike
(the gate reads unreachable already true). NOT evidenced by W14-w,
now evidenced: its unit read (bench lines == distinct benched jobs)
undercounts benches; the honest count needs one door -- a
JobBoard::bench(job, reason) that every writer goes through, with
the witness there (candidate W14-w2 THE BENCH HAS ONE DOOR). And
W14-i6's four false proofs are jobs whose strikes came from W14-e2's
Longest exhausts (no approach proofs today) after a silent bench:
the "proof" was a budget exhaustion on a store-cell target that the
walker then reached to within two blocks -- the far class is not
unreachability, it is the search demanding the exact end cell
while the job needs adjacency, most likely at doorways and
interiors the admission rules refuse to the search and the glide
walks through (the frontier's 663 floorless-or-refused air cells).
W14-i7's walker names settle it.

### W14-g landed (a468786d41, staged 17:02:18; shipped to lab-bin 17:02:24)

Chain bw6thgwjl: check ok, pin the_search_restarts_when_its_end_moves
green, committed a468786d41, both halves from one commit (common/
changed), the server exe carries "THE END MOVED UNDER THE SEARCH"
(grep 3) beside the W14-i5, W14-i6 and W18-b2 witnesses. Falsifier
RED at 17:07 (the tag test planted always-false in its own
worktree: the moved goal on the old heap found no Path; 0 passed,
1 failed; restored, 0 dirty). W14-i7's chain fired at 17:07:18.
Readers for the W14-g pair are armed behind the W18-c readers
(wait-w14g-b1/b2) so the arms keep restarting daily after the
current cascade; the first-hour live reads come from two waiters
keyed on each arm's next boot.
Live read: b1 at the W14-e2 reader's restart (~17:25), b2 at the
W18-c reader's restart (~17:35, with the diag) -- the first hour's
bar: the restart witness >= 1 on each arm, the touched class
(closest_dist=0) at 0, LONGEST-EXHAUST falling toward 60 a day on
b2 (644 today). lab-bin is a468786d41 (playable).

### b1 on W14-w (6e60a081ff since 16:34), hour 19 (16:55): the third W18-c day's afternoon

Arrivals 688 at hour 19 (688 on the second W18-c day at hour 19,
686 on the first: three days identical at this frame), FETCH
STALLED 15 (16 / 13 on the W18-b2 days; (7664,6432) x9 -- the ledge
foot), CLIMB BANNED fetch 5 (all under a two-block edge, trunk
frame: the jump the trunk plans and the glider cannot make -- W2-b's
class, five today), other 3, STUCK CENSUS 3 distinct, bobs peak 2,
starving sleepers 0, W14-e2 bench 1, terminal 1, LONGEST-EXHAUST 17
for the day so far (the ledge arm's flooders absent today), bench
lines 2 = distinct jobs 2 (W14-w's unit holding). The night block
(~17:20) closes the third replicate.

### W14-i6 on b2, first look (16:47; e4351e2457 since 16:42, hour 10)

Every B5 arrival line carries strikes= and benched= (219 of 219;
all strikes=0 so far -- no job struck three times by hour 10), THE
PROOF WAS FALSE 0, bench lines 0 (W14-w's unit read waits for the
first bench). Flood 84 by hour 10 (far 64, touched 20), stalls 0,
starving none. The instrument's field bar PASSED; its false-proof
count reads at the day's end.

### W14-g: check ok, pin GREEN, committed a468786d41 (16:46; building both)

The slab pin held every assumption: one Small poll toward a goal 110
blocks east stayed Pending with >= 200 iterations spent; the next
poll toward the cell one block behind the start returned a Path in
under 50 expansions; two polls toward the same far goal continued
one search (> 250 spent); the tag rule and the tag's distinctness
held. Stage ~17:02 (common/ changed: both halves rebuild), then the
falsifier (the tag test always false -> the moved goal on the old
heap -> no Path -> red) and W14-i7's chain at +300 s. Live read: b2
boards the pair at the W18-c reader's restart (~17:35) with the
diag; b1 at the W14-e2 reader's restart (~17:25). b1's flood is
quiet today (3 exhausts by hour 14 against 139-256 on earlier days;
LONGEST-TIER step lines 1,100 against 40-54k: the producers -- a few
colonists with specific targets -- are absent, not a change), so b2
is the read that counts.

### W14-i7 registered (16:25): THE FLOOD NAMES ITS WALKER (instrument)

No log line ties an exhaust to its walker (the search has no uid;
the job loop's consumer sees only active jobs; no job, bed, spot or
item line names the far ends). Mechanism: exhaust_rose(seen, now)
= now != seen && now > 0; the board keeps exhausts_seen per walker;
at the mover's position write (every walker passes it) the chaser
snapshot's longest_exhausts is compared with the last seen and each
rise prints "THE FLOOD NAMES ITS WALKER" (uid, job or none, kind,
job_pos, the search's last target, route target, count, feet) at
the first sixteen and powers of two. Pin the_flood_names_its_walker
(0->1, 2->3, 2->1 named; 1->1 and resets not); falsifier plants a
plain greater-than -> red on 2->1. Bar: named within 10% of b2's
LONGEST-EXHAUST count; the far and touched ends' producers named;
the jobless share a number. Falsified as an instrument if named
runs far below the exhaust count (the flooders never take a mover
write: vanilla NPCs or held bodies -- then the agent system is the
place). Queued behind W14-g (bastion-server only).

### W18-c's replicate on b1 (W18-b2 + W18-c), the full day (16:20, read at day 1 hour 0): W18-c STANDS; the arrivals bar stays open

Starving sleepers 0 (4 the night before under W18-c + W18-b; bar <=
1: PASSED), STARVING lines 1 (colonist 22 at 0.05 once, awake).
FETCH STALLED 14 (29 under W18-c + W18-b; 12-13 under W18-b alone;
bar <= 2x: PASSED) -- the held-at-the-edge class is gone with the
step down allowed; the 14 are the up-direction jump stalls. STUCK
CENSUS 6 (15). Bobs peak 2 (bar <= 16: PASSED; 128 and 256 before
W18-c). Other bans 1, W14-e2 benches 0 jobs, flood 594 (585; W14-g
and the far class own it), Open 1,024, Closed 3, NIGHT SHELF EMPTY
0. Arrivals 863 (828 under W18-c + W18-b; 991 under W18-b alone;
bar >= 900: FAILED by 4%, and -13% against the W18-b-only day while
b2's W18-b2 day read 736 against 744 -- flat). Reading (a) holds:
the stalls and the starvation were W18-b's and the night classes';
W18-c stands. The one open cost is b1's arrivals, two days below
900 with W18-c aboard and one day above without it -- one arm, and
COUNTS VARY 2-3x: the third replicate is b1's next day (the W14-i6
pair); if it reads under 900 again with b2 flat, the x-y window is
costing the ledge arm real walks and W18-c is reconsidered.

The reader's own night block (16:29, read at day 1 HOUR 6 -- the
registered frame): arrivals 916 (889 at the same frame on the
W18-c-only day; bar >= 900 by night: PASSED at the registered
frame; my hour-0 hand read of 863 is the other frame -- TWO FRAMES,
kept apart), starving sleepers 0, FETCH STALLED 16, STUCK CENSUS
13 distinct, other bans 3, terminal bench 1 (job 628, colonist
131), W14-e2 benches 0, flood 604 (58x (7705,6393,180), 35x
(7615,6271,182), 30x (7609,6265,181)), panics 0. So W18-c's
arrivals clause, read at its own frame, passed on the replicate;
the third day remains the tie-break for the hour-0 frame.

### W14-i5 on b2, the second read (16:18; 262 exhaust lines in 13 minutes)

b2's flood, invisible until today's diag, runs at b1's scale: 262
whole-town exhausts by hour 14. Classes: FAR (the end never visited,
end_g None) 192 = 73% -- 107 with the flee term, 85 without; TOUCHED
(end visited, end_g Some 10-13, never popped) 70 = 27%, all without
the flee term, three with a priced direct edge. The far ends sit at
z 182 ((7686,6453,182) x26, (7615,6271,182) x23, (7679,6191,182)
x18, (7673,6203,182) x14) and the search's closest node is 3 cells
away (Manhattan) in 65 of 192: the end is one layer above the
closest reach and no edge enters it -- an island cell, read next by
its own neighbours' walkability. The touched ends sit at z 180-181
((7629,6266,180) x16, (7679,6203,181) x10, (7610,6273,181) x9). So
two mechanisms share the flood: W14-g's moved goal (the touched
quarter) and, for the far three quarters, a goal no edge can enter
-- proven unreachable the expensive way, 61k states per ask; A
GUARD MUST REFUSE BEFORE IT SPENDS is the shape of its cure (an end
with no walkable neighbour is refused before the search).

### W14-g registered (16:17): THE SEARCH RESTARTS WHEN ITS END MOVES

From W14-i5's live read (below): a state at f = 11 cannot sit
unpopped through 61,000 pops in one correctly ordered search (the
heap pops the smallest f by total_cmp; every visited insert is
pushed; a popped end satisfies at once), so the cheap end's heap
entry was consumed EARLIER, when that cell was not the goal: the
retained astar (kept across polls and the tiers' budget
escalations; reset only when the start moves more than two blocks,
path.rs 2166) was built for another resolved end, and when the goal
moved onto an already-popped cell `satisfied` could never fire.
Within-search steps show one end (16:11), so the move happens
between the lower tiers and the Longest tier, or between polls
before the diag prints.

MECHANISM: Astar.goal_tag (0 = untagged; tagged()/goal_tag());
search_end_tag(end) a nonzero hash of the resolved end;
search_end_moved(retained, current) true only for a tagged search
whose tag differs; find_path_priced restarts such a search before
the start-moved reset, counts it (END_MOVED) and prints "THE END
MOVED UNDER THE SEARCH" (start, end, tier, iterations the abandoned
search had spent, count) at the first eight and powers of two; the
new search is created tagged. Untagged searches and unmoved goals:
identity. Pin the_search_restarts_when_its_end_moves (the rule; the
tag nonzero and distinct; on a 120x40 slab a Small poll toward a
goal 110 blocks east stays Pending with >= 200 spent; the next poll
toward the cell one block behind the start -- popped in the first
poll -- returns a Path in under 50 expansions; two polls toward the
same far goal continue one search, > 250 spent). Falsifier plants
the tag test always false -> the moved goal continues the old heap
-> no Path -> red.

PREDICTION (both arms at their next restarts; b2 with the diag):
the restart witness >= 1 in the first hour on each arm;
LONGEST-EXHAUST under 60 a day on b1 (567, 585, 461-by-hour-19) and
on b2, the touched class (closest_dist=0) at 0; arrivals not below
900 / 720; starving unchanged. FALSIFIED if touched exhausts with
end_g of a few units persist on a pair that shows the restart
witness (the goal did not move; another cause), or if restarts run
into the thousands a day with the flood unchanged (goals moving
every tick: the producer first). Rejected: re-pushing a goal found
in visited (the rest of the heap stays stale); a distance-based
reset (a moved goal is a moved goal); fixing the producers first
(unknown which). NOT evidenced: which producer moves the goal; the
far class (end never visited); arrivals under frequent restarts.
Chain behind W14-i6 (common/; both halves rebuild).

### W18-c's replicate on b1 (5c8285fb34 = W18-b2 + W18-c, since 15:46) at hour 19 (16:12)

FETCH STALLED 13 (27 at hour 19 on the W18-c-only day; 9 on the
W18-b-only day), all Designated walks, at ledge FEET (bodies wanting
to go up: the trunk's jump the glider cannot make), none held at a
ledge head; STUCK CENSUS 3 (9); bobs peak 1 (bar <= 16: PASSED
again); arrivals 688 at hour 19 (686 on the W18-c-only day, 752 on
the W14-e day); STARVING lines 1 at hour 19 (read at the night
block); other bans 1; Open 1,024; Closed 3 (the one-cell hole).
Reading (a) is holding: with the step down allowed, the stalls
halved and the held-at-the-edge class is gone; the night block
(~16:32) reads starving and the day's arrivals.

### W14-w landed (6e60a081ff, staged 16:10:43; shipped to lab-bin 16:11:02)

Chain bh8jac50g: check ok, pin the_bench_is_witnessed_once green,
committed 6e60a081ff, both halves from one commit ("compiled
fresh"). No new string (a witness-gate row): verified by the fresh
compile from HEAD and the falsifier: RED at 16:15 (the already-
benched test dropped in its own worktree: 0 passed, 1 failed;
restored, 0 dirty);
the live read is the next b1/b2 day's bench-line count against
distinct benched jobs. lab-bin is 6e60a081ff. W14-i6's chain fires
at +300 s.

Within-search check of the moved-end theory (16:11): for the 35
touched exhausts on b2, every search's own steps carry ONE resolved
end (e.g. 67 steps, one end; 12 steps, one end); the many one-step
"searches" are a retained, already-exhausted astar answering
Exhausted at once. So the end did not move within the flood. The
cheap end still went unpopped inside a single, first, no-flee
search: the defect is in the search's own bookkeeping -- reading the
push condition and the heap entry's ordering next.

### W14-i5 LIVE READ on b2 (c288d55479 with the diag, 16:05-16:09): the end is CHEAP and never popped

61 exhaust lines in four minutes (b2 had never shown its floods:
the diag was off). Classes: 23 touched / end_g Some / direct_edge
None / no flee; 21 far / end_g None / flee; 16 far / end_g None /
no flee; 1 touched / end_g Some / direct_edge Some(5.0). The touched
ones: end (7611,6273,181) end_g=11.0, end_states=1, max_g=866,
expanded 61,565; (7679,6203,181) end_g=10.01, max_g=616, 62,106;
(7611,6266,181) end_g=13.01, max_g=844; (7629,6266,180) end_g=12.01,
direct_edge Some(5.0) -- ALL WITH flee=None (first searches, not the
re-ask), all with the end VISITED AT A COST OF 10-13 (three or four
steps from the start) and NEVER POPPED while 61,000 states with g up
to 866 were. A correct A* pops a state with f = 11 (g 11, h 0) before
any state with f > 11; this one did not. So the end's heap entry did
not carry f = 11: its priority was computed against ANOTHER end --
the retained search (path.rs 2166 resets the astar only when the
START moves more than two blocks, never when the resolved END
changes) kept a heap built for a previous end while `satisfied` and
the diag read the new one. THE END MOVED UNDER A RETAINED SEARCH is
the mechanism candidate; the test below reads the search's own
steps for more than one resolved end. NOT the flee term (flee=None),
NOT a refused or dear edge (end_g 10-13), NOT a true island (the end
is in the visited set). The instrument's own bar held: every line
carries the fields; end_g is Some for the touched class (the
prediction said "above max_g" -- it is far BELOW it, which is the
finding).

### W14-i5 landed (c288d55479, staged 15:46:36; shipped to lab-bin 15:46:55)

Chain bi37xumk0: check ok, pin the_exhaust_names_its_ends_cost green,
the slab experiment printed (above), committed c288d55479, both
halves from one commit, the server exe carries "W14-i5: end_g"
(grep 3). Falsifier RED at 15:51 (the dearest end state planted as
the cheapest in its own worktree: 0 passed, 1 failed; restored, 0
dirty). The live read: b2 boards it at the W18-b reader's restart (~16:15) with
the endpoint diag env now in its restart script; b1 at the W18-i
reader's restart (~16:45) -- b1 booted 5c8285fb34 ten seconds before
this stage. lab-bin is c288d55479 (playable; identical to W18-b2 in
behaviour, plus the diag fields and the astar accessor).

### W18-c on b1, the full day (15:30; 5346279326, read at day 1 hour 0): the bob bar PASSED, the stalls bar FAILED, the arrivals clause TRIPPED -- one replicate, re-read on the W18-b2 day

Bobs: 10 lines, peak bobs=1 (bar <= 16: PASSED; 128 yesterday). FETCH
STALLED 29 (12-13 yesterday, 2.3x; bar <= 2x: FAILED) -- 21
Designated at the terrace edges, 6 EatFrom, 2 Cook; STUCK CENSUS 15.
Arrivals 828 (bar >= 900: FAILED; 991 yesterday at the same clock,
-16%: the registered falsification clause "arrivals fall > 10%"
TRIPPED). Starving sleepers 1 (colonist 141, RestAt Traveling at
hunger 0.04 at tick 34500; bar <= 1: PASSED). Other bans 3 (19),
fetch bans 2, W6-D 0, route proofs 0 jobs, W14-e2 benches 53 (1
yesterday: the consumer now reaches the jobbed flooders every time
the count hits three -- 53 jobs benched a day is a new number to
watch, and W14-i6 says whether those proofs are true),
LONGEST-EXHAUST 585 (567), drops refused 8,192.

The 57 W14-e2 lines are TWO jobs, re-benched once per arbitration
latch period through the day: colonist 148's job at (7769,6401,186)
x31 and colonist 62's at (7626,6321,186) x26 -- both targets at z 186,
beds or home spots three above a floor (W17-c's sealed-sleeper
class), and (7769,6401,186) is the very bed b2's colonist 136 was
walking to when it bobbed 256 times. The jobbed half of b1's flood
is the bed-at-z-186 class; the jobless half (the one-block ends)
still has no name. W14-i5 prints end_g and direct_edge for both.

THE NIGHT (reader's block at day 1 hour 6, 15:43): STARVING
SLEEPERS 4 (bar <= 1: FAILED) -- colonist 141 (44 samples at 0.00
in bed, ~7 min), 148 (15; the bed-at-z-186 flooder: it never reached
its bed and starved on the way), 149 (14), 37 (10); colonist 26
eating at dawn. Arrivals 889 by hour 6 (bar >= 900). Flood 812 by
dawn, top end 285x (7721,6335,188) -- another upper-floor target,
z 188. Yesterday under W18-b alone b1 had 0 starving sleepers; the
day before (W14-e) 2. So the W18-c + W18-b pair starved four on the
ledge arm: the held bodies that used to jitter through now stall,
shun and lose their suppers, and the bed floods take the rest of
the night. W18-b2 boards b1 at 15:46; if its night reads starving
<= 1 with the bob peak still <= 16, the cause was the hold (W18-b)
and W18-c stands; if it starves again, W18-c is reconsidered.

Attribution (15:48), each starver traced: 148 = the bed flood (a
RestAt to the z-186 bed it could never reach, hunger falling as it
walked all night). 141, 149 and 37 = the E2 night-shelf class: all
three took a hunger preempt at 19:41:24 UTC (hour ~22) toward the
general store's food (Store 39 / a cell), and a rest preempt a
minute later sent each to bed unfed (141: NIGHT SHELF EMPTY x15,
0 stalls, 0 refusals; 149: 0 stalls; 37: 0 stalls) -- the curfew's
empty shelf (E2), not the ledge, not the clock. So W18-c's night
bar failed on two classes it does not touch, both older than it:
the sealed bed's flood (W17-c / the router row) and the night
shelf (E2's open half: the supper carried home). Neither is an
argument against W18-c; the W18-b2 day still reads its cost.
Candidate from the three (not rowed): THE REST PREEMPT WAITS FOR
THE SUPPER -- a hunger walk begun before the curfew is not
overridden by the rest need a minute later; the sleeper eats first
or the store is closed. It sits beside the open judgement item (the
night watch's supper) and E2's carried-home half; Ben's call on the
night rule comes first.

Disposal, honestly: W18-c did what it claimed (the bob class is gone
from the ledge arm) and its registered costs came due -- the bodies
the old clock let bounce now stall, and the day's arrivals fell by
more than the clause allowed. Two readings compete: (a) the stalls
and the lost arrivals are W18-b's refused step down made visible
(21 of 29 stalls sit at the terrace edges W18-b refuses; W18-b2
lifts that today), or (b) the x-y window now stalls walks that were
real (the clause's own reading). One replicate cannot separate them;
COLONY COUNTS VARY 2-3x (b1 arrivals ran 736-1,090 across the last
days). The W18-b2 b1 day (b1 boards it at ~15:40) is the replicate:
if stalls return to ~12 and arrivals to >= 900 with the bob peak
still <= 16, (a) holds and W18-c stands; if stalls stay >= 25 or
arrivals stay < 900, (b) holds and W18-c is reconsidered (a wider
minimum, or the window measured on the route's own axis).

### W18-b2 landed (5c8285fb34, staged 15:23:28; shipped to lab-bin 15:23:47)

Chain bdx5gezga: check ok, pin
the_body_does_not_drop_into_a_cell_it_cannot_leave green (the pit
Closed with nine cells, the ramp WayUp, the step, the 40x40 ledge
Open), committed 5c8285fb34, both halves from one commit, the server
exe carries "THE DROP INTO THE OPEN IS ALLOWED" (grep 1) beside the
W18-b and E2-t witnesses. Falsifier RED at 15:28 (the open-basin
cap planted at a million in its own worktree: the 1,600-cell ledge
read Closed, 0 passed, 1 failed; restored, 0 dirty). b2
boards it now under the relaunched W14-e2 reader (held back so b2
would not board the older pair minutes before this stage); its
night is the starving read (~16:15). b1 boards it at the W14-e
reader's restart after tonight's block (~15:45). lab-bin is
5c8285fb34 (playable). W14-i5's chain fires at +300 s.

First witness, b2 at boot+60 s (15:27): "THE DROP INTO THE OPEN IS
ALLOWED" x8, uid 138, landing (7754,6343,181), edge_z 183, cells=64,
site bridge -- the same landing that W18-b refused eight times at
boot+50 s on both arms yesterday (uid 111 / 118). The Closed witness
0 so far (the pit's rim not yet reached). The mechanism is live and
reads as registered.

### W18-c on b1 at hour 19 (15:20; 5346279326): the bob bar PASSES, the stalls bar FAILS on W18-b's cost

Bob lines 5, peak bobs=1 (bar <= 16: PASSED; 128 at this hour the
day before, colonist 117 at (7700,6303) -- 0 bobs and 0 stalls at
that cell today). FETCH STALLED 27 (9 at hour 19 the day before:
3x; bar <= 2x: FAILED) -- 21 of them Designated walks at the terrace
edges ((7640,6436):4, (7664,6432):4, (7684,6388):4), i.e. W18-b's
refused step down held for 15 s and now SEEN by the x-y clock; 4
EatFrom, 2 Cook. STUCK CENSUS 9. Arrivals 686 at hour 19 (752 on
the W14-e day's hour 19; 991 for yesterday's whole day; the bar is
>= 900 by night). Other bans 3, benches 11, LONGEST-EXHAUST 462
(the flood, W14-i5's subject). The reader's own hour-19 block
(15:20) adds: W14-e2 benches 11 on b1 today ("three exhausted
Longest searches", jobs 1017 and others; 1 for the whole day
before) -- the consumer now reaches b1's flooders that hold a job;
route-proof benches 0 today (the far TradeMission target absent);
flood 461, top ends 52x (7697,6459,182), 39x (7679,6203,181), 27x
(7721,6335,188). Disposal: W18-c did what it claimed
(a bobbing body now stalls, and the bob class is gone from the
ledge arm); the stalls it exposes are W18-b's, and W18-b2 (staging)
is their cure -- the honest bar for W18-c's cost is re-read on the
W18-b2 b1 day. The night's arrivals and starving close the read.

### E2-t on b2, night 0 (a86bb23715 since 14:44; read by hand 15:18 at day 1 hour 0)

Starving sleepers 0 (3 the night before), STARVING lines 0 all day,
NIGHT SHELF EMPTY 0 (so no home's food carried an unclaimed haul's
reservation tonight: the yield witness is 0 and UNEXERCISED, not
falsified -- the bar reads "at least 1 on any night the case
occurs"), haul releases 0, arrivals 744 (bar >= 720: PASSED), FETCH
STALLED 2 for the day (19 the day before under the same W18-b rule:
the ledge stalls were the supper walk's, and tonight's suppers went
another way -- COUNTS VARY 2-3x, the W18-b2 day decides), pit drops
0, drops refused 4,096. One number against the pair: bobs peak 256
(this pair carries W18-i and W18-b, not W18-c) -- named below; on
b1, under W18-c since 14:54, the peak is 1. E2-t stands unexercised;
its witness waits for a night with the case.

The 256-bob body: colonist 136 (Mine), cell (7700,6303) z 183 -> 181,
site bridge-probe, bobs 64/128/256 at 19:15:29-19:17:33 UTC (hours
21-22, after its evening meal and a RECREATE "home" walk toward its
bed at (7769,6401,186)), stuck_time 0.033 s at every sample, 0 stalls,
0 census lines: six minutes of bouncing that the clock never saw --
W18-c's exact case, on the pair without it. The same cell held b1's
colonist 117 (bobs=128) the day before and b2's colonist 65 (bobs=2)
this morning: a terrace step on the home road at (7700,6303). Under
W18-c a body there stalls in 15 s and its job's consumers act; the
step itself (a two-block edge with a way up beside it) is the ramp
row's geometry.

### W18-c on b1, first half-day (15:07; 5346279326 since 14:54): the bobs are gone, the ledge reads as stalls

Bob lines 2, peak bobs=1 (128 on the day before, colonist 117).
FETCH STALLED 23 by about hour 12 (12-13 for a whole day before):
21 are Designated(Farm) walks with feet at (7642,6439,182) -- the
farm terrace's ledge, b1's copy of the one that starved b2's farmers
-- "no displacement, expiring early", 15 s each, 24 shuns, no stuck
timeouts, no benches. The chain of causes: W18-b refuses the step
down (refused=2048 by then), the body holds at the edge, and W18-c's
x-y clock now sees the hold (the old 3-D clock saw the probe's
jitter as progress). So the stalls bar W18-c registered (<= 2x the
day before) will read as W18-b's cost until W18-b2 lands; the honest
disposal waits for W18-b2's b1 day, where the same walk steps down
and neither bobs nor stalls. Arrivals 533 by hour ~12 (374-499 at
hour 12 on earlier days).

### W14-i5's slab experiment: PRINTED (15:34) -- the flee term is not the flood; a dear end is

    open   / no-flee       : Path(len=3,  cost=6.01)   consumed=5
    open   / flee-at-start : Path(len=3,  cost=6.01)   consumed=272
    walled / no-flee       : Path(len=81, cost=254.83) consumed=2717
    walled / flee-at-start : Path(len=81, cost=254.83) consumed=3516

Against the prediction below: OPEN/FLEE printed 272 -- neither "under
100" nor "thousands": the flee term inflates a 5-expansion search
54x but does NOT flood the slab (the negative-h region is local, as
the arithmetic said). WALLED/NO-FLEE 2,717 of ~3,540 walkable cells:
the ordinary price of a Euclidean heuristic against a long detour.
WALLED/FLEE 3,516: the whole slab -- the flee term turns "most of
it" into "all of it". So on the live town (61k cells, 75k budget): a
target reached only by a long detour floods most of the component
by itself, and the flee term after the first exhaust saturates it
and (with the duplicates) tips it past the budget into "Exhausted"
instead of a Path -- the fixed point. W14-f (the flee term clamped)
would buy ~25% on the slab and not remove the flood; the fix must
remove the DETOUR or PROVE the unreachability cheaply. W14-i5's
live fields (end_g, direct_edge) name which: a refused one-block
edge, a price, or a genuinely unreachable end (the bed at z 186
class). Pin the_exhaust_names_its_ends_cost green; committed
c288d55479 (building both; stage ~16:00). The live read: b1 boards
it at the W18-i reader's restart (~16:40); b2 sooner: its restart
script now exports BASTION_PATH_ENDPOINT_DIAG=1 (15:35), so b2's
next boot (the W18-b reader's restart, ~16:15) prints the exhaust
diag too -- a new instrument on b2, not a behaviour change; b2's
flood tally reads nonzero from then on and is not comparable with
its earlier zeros.

### W14-i5's slab experiment, predicted before it prints (15:28)

Four searches on a 60x60 slab (3,600 cells), the end two blocks
east of the start, Longest tier (75k budget): OPEN / NO-FLEE: Path
of 2-3 nodes, consumed under 20. OPEN / FLEE at the start: the
decisive cell -- under ~100 if the flee term merely reorders near
the body (my arithmetic: h goes negative only within ~30-40 blocks),
thousands (the whole slab) if it inverts the search; the live flood
needs the latter AND a dear end. WALLED (one gap 39 rows away) /
NO-FLEE: Path of ~80 nodes, consumed in the thousands (admissible
A* with a Euclidean heuristic explores the disc whose path-plus-
distance is under the detour's cost: the ordinary price of a
misleading heuristic). WALLED / FLEE: the whole slab. If OPEN/FLEE
prints thousands, W14-f (the flee term clamped) is the fix and my
arithmetic was wrong; if it prints under 100, the live flood is a
dear or refused edge and the direct_edge field on the exhaust line
names it. Either way the walled case shows that a target reached
only by a long detour floods a town-sized component with or without
the flee term -- the router's missing step down (job 440) is that
detour's most likely maker.

### W14-i6 registered (15:05): THE PROOF WAS FALSE (instrument)

Job 440 was benched by three "found no way" approach searches and
its walker arrived 100 s later (above). The router has no two-block
step down; the mover takes one. Before the router is changed, the
rate: proof_was_false(strikes) = strikes >= UNREACHABLE_STRIKES; the
B5 arrival line carries strikes= and benched= for every arrival, and
an arrival on three or more strikes prints "THE PROOF WAS FALSE --
the body arrived at a job the search had struck three times" (job,
colonist, kind, job_pos, body, strikes, benched). Pin
the_proof_was_false (0, 2: not; 3, 9: yes); falsifier plants the
bound off by one. Bar: every B5 line carries the fields; the rate
(false proofs / distinct benched jobs) on the next b1 day; falsified
as an instrument if a false-proof line names a job with no bench
witness that day (the strikes came from another arm -- the join must
go to the route-proof label). Queued behind W14-w (bastion-server
only). NOT evidenced: which arm struck (the next instrument carries
the source if the join is ambiguous).

### W14-w registered (15:00): THE BENCH IS WITNESSED ONCE (instrument honesty)

The 101 route-proof bench lines were 18 jobs: job_strike saturates
(next >= 3 stays true) and the four older bench sites (route proof,
fill exhaust, banned climb, terminal chaser) printed on every
bench=true, benched or not -- job 440 printed 39 times in 2.5 min.
W14-e2's site already gated on !job.unreachable. Mechanism:
bench_is_new(bench, already_benched) = bench && !already_benched at
all five sites; state unchanged (the assignment was idempotent, the
strikes keep counting, the latch re-arms the bench). Pin
the_bench_is_witnessed_once; falsifier drops the already-benched
test -> red on the re-fire. No new string (verified by pin,
falsifier and the next b1 day's count: lines == distinct benched
jobs). Queued behind W14-i5 (bastion-server only, a short build).
Second instance (15:21): b2's a86bb23715 day read route_proofs=101
in the tally -- 2 distinct jobs, job 713 alone 88 lines.

### Observation (14:55): the router cannot plan the step the mover takes

b1 day 0 (dfa366b6db): 101 "three failed route proofs" bench lines
are 18 jobs; 17 were never reached (a TradeMission to (7280,7824),
1,500 blocks out, leads them); ONE was reached anyway -- job 440, a
DepositRun to zone 87 at (7742,6143,182): colonist 120's exact
approach search "found no way" three times (18:10:15-18:11:52), the
job was benched 39 times over 2.5 minutes while the body kept
walking, and the body ARRIVED at 18:13:38 and deposited. The
search's world and the mover's disagree, and the neighbour generator
says how (path.rs 1795-1850): the move set is DIRS (lateral, lateral
+-1, straight down 1), JUMPS (+2, reach-gated) and SCRAMBLES (+3);
every move's target cell must be walkable; there is NO two-block
step down. The mover (pre-W18-b) dropped two blocks freely by the
pre-path glide and the probes' dz -2 arm; W18-b2 keeps that for open
landings. So a target below a two-block ledge is "unreachable" to
the search and walked to by the body: false proofs, long detours,
and the benches, shuns and strikes that hang off them -- GENERATOR
AND CONSUMER MUST AGREE. Candidate after the queue and W14-i5's
read: THE ROUTER PLANS THE STEP DOWN (a dz -2 lateral edge for
colony walkers, admitted by the same open-basin verdict the mover
uses, priced as a hazard). Also: the bench witness re-fires per
re-bench (39 lines for one job) -- a witness defect, parked.

### W18-c landed (5346279326, staged 14:52:43)

Chain bkkum6y3n: check ok, pin the_bob_is_not_progress green (1
passed), committed 5346279326, both halves built from one commit
("compiled fresh", server exe 14:52). No string marker: the row adds
no log line and its helper's name does not survive in the release
exe (grep 0), so the binary is verified by the chain's fresh compile
from HEAD after the commit and by the falsifier: RED at 14:57 (the
three-dimensional distance planted back in its own worktree: 0
passed, 1 failed; restored, 0 dirty). The live read is the bob
line's stuck_time on b1, which took this pair at 14:54. The pair still
carries the E2-t and W18-b witness strings (grep 1 each). Readers:
b1 fresh after E2-t's night-1 block, b2 fresh after E2-t's day-1
block (each takes the latest staged pair at that moment).

### b1 night 0 on dfa366b6db (14:46): the night watch's supper, again

STARVING 1 at tick 41100: colonist 112, the NIGHT WATCH (took post
(7722,6320) at 18:38:19 UTC, released by "Personal entry releases
the held work job" at 18:41:00, a hunger preempt to an EatFrom at a
store at (7725,6368,186), Traveling at hunger 0.00). Not E2-t's class
(no shelf refusal) and not a new one: the open judgement item "should
the night watch carry its own supper" -- its witness on the ledge
arm. Recorded, not rowed.

### b1 day 0 on dfa366b6db, read at day 1 hour 0 (14:41): W6-D's first bench; W14-e2's first b1 bench; the flood 567

Arrivals 991 (bar >= 950: PASSED), starving 0, fetch bans 0, other
bans 19 (bar <= 20: PASSED by one; colonist 117 nine of them),
stalls 12, benches: route proofs 101, W6-D "three banned climbs" 1
(job 1162, colonist 112 -- W6-D's consumer fired for the first time
since it landed at 11:17), W14-e2 "three exhausted Longest searches"
1 (job 1431 -- its first b1 witness), terminal 0. LONGEST-EXHAUST 567
for the day (bar <= 60: FAILED), all whole-town, top ends 33x
(7679,6203,181), 26x (7702,6453,182), 26x (7609,6263,181) -- the
closest_dist=0 class, W14-i5's subject. Bobs: colonist 117 bobs=128
at (7700,6303) with stuck_time 0.033 s at 64 and 128 -- W18-c's case,
now on the ledge arm too (the same clock blindness; the pair W18-c
builds on carries it). Drops refused 8,192, 13 of 14 sampled lines
"bridge"; no starver on b1 from the rule -- its refused bodies
re-routed, unlike b2's farmers.

### W18-b FAILED its starving bar on b2's night 0 (14:38) -- W18-b2 registered (14:45): THE DROP INTO THE OPEN IS ALLOWED

b2, dfa366b6db, tick 33000 (the Sleep block): STARVING 3 -- colonists
17, 20 and 49, all Farm, all RestAt Traveling at hunger 0.00. Each
evening reads the same: a hunger preempt to an EatFrom at the stores
north of the farm (items at (7675,6394) / (7644,6394), z 181) from
feet on the terrace at (7644-7648, 6440-6447, z 182); THE WALKER
SHUNS ITS STALL on it; a second pick under the shun, stalled and
shunned too; NIGHT SHELF EMPTY x6 / x6 / x2 (their own shelves held
nothing); bed. The day's stalls cluster at the terrace's edge
((7640,6436):8, (7644,6436):3, (7652,6440):2), FETCH STALLED 19 by
hour 17 against 1 on the W18-i day, and THE DROP HAS NO WAY UP fired
4,096 times at that edge (landings (7635,6439..6447,180), edge_z
182). Before W18-b those bodies glided off the ledge to supper; the
landing is the whole lower town and nobody needs a way back up THERE.
W18-b asked for a way up within four blocks of the landing. A GUARD
CAN STARVE ITS PROTECTEE: the guard built for a nine-cell pit refused
the town's own step down. (b1: the same rule fired 8,192 times by
hour 16, no starver -- its refused bodies re-routed.) W18-b's pit bar
PASSED (0 drops at (7712,6306) on both arms); its starving bar
FAILED; the rule is superseded, not reverted.

W18-b2 MECHANISM: drop_verdict(standable, landing, edge_z) walks the
standable cells from the landing (one up or down per step) and
answers WayUp / Open { cells } (OPEN_BASIN_CELLS = 64 reached with
no way up: the landing opens onto the town) / Closed { cells } (the
walk ran out first). A two-block drop is refused only on Closed;
Open is witnessed at the first eight and powers of two ("THE DROP
INTO THE OPEN IS ALLOWED", uid, landing, edge_z, cells, opened,
site); the radius cap is gone, the cell cap bounds the walk. The
pit (nine cells under a rim two above) is Closed, still refused.
Pin: the pit Closed with nine cells; the ramp WayUp; the step safe;
a ledge onto a 40x40 lower floor with no way up anywhere Open and
taken. Falsifier plants the cap at a million -> the 1,600-cell floor
reads Closed -> red.

PREDICTION (the W18-c readers' blocks read it; the arms take the
pair at their next restarts): b2 starving sleepers 0 (3 tonight),
FETCH STALLED <= 6 for the day (19 by hour 17 today), the Open
witness >= 1 at the terrace ledge, the pit's rim still refused (>= 1
whenever a body reaches it), POS-WRITE drops at (7712,6306) 0,
arrivals >= 720; b1 starving <= 1, arrivals >= 900. FALSIFIED if a
body drops into the pit again (a basin over 64 cells with no way up:
the cap is the wrong instrument) or a sleeper starves with the Open
witness at its ledge (the stall was not the drop's).

Rejected: reverting W18-b (colonist 16's pit comes back); a larger
way-up radius (the ledge's way up is a stair across the town);
routing the trunk around the ledge (the ramp plot row is the town's
answer; until then the step down is how the town walks -- MOVEMENT
LENIENCY). Queue: W18-c (building) -> W18-b2 -> W14-i5.

### W14-i5 registered (14:40): THE EXHAUST NAMES ITS END'S COST (instrument)

The closest_dist=0 flood class (above) needs its end's g before a
fix is chosen. Astar::visited_cost(node) exposes the recorded g;
end_cost_summary(states) -> (the cheapest g among the end's states,
their count, the dearest g anywhere) is pinned
(the_exhaust_names_its_ends_cost; falsifier plants the dearest end
state reported as the cheapest); the Longest exhaust diag prints
end_g / end_states / max_g / direct_edge (the neighbour generator
asked once from the start node for the end: None = the edge is
refused, a number = its price) / flee. Plus an experiment test
(flee_experiment_prints: a 60x60 slab, the end two blocks east, open
and behind a wall with one gap 39 rows away, with and without the
flee term) whose four expansion counts the chain prints. Bar (the
instrument's own): every exhaust line carries the fields; for the
closest_dist=0 class end_g is Some and above the popped set's max_g,
and direct_edge is None for the one-block ends (or a number, which
names the price). Falsified as an instrument if those lines carry
end_g=None. No readers: b1 reads it at its next restart (the floods
recur every 15 s).

### E2-t landed (a86bb23715, staged 14:28:43; shipped to lab-bin 14:29)

Chain btaa424v0: check ok, pin the_supper_outranks_the_posted_haul
green (1 passed), committed a86bb23715, both halves built from one
commit, the server exe carries the witness string (grep count 1).
The falsifier went RED at 14:33 (the class test set to a name no job
has: 0 passed, 1 failed; restored, 0 dirty). Readers:
b1 fresh after W18-b's night-1 block, b2 fresh after W18-b's day-1
block. lab-bin is now a86bb23715 (playable).

### The first-hour read on dfa366b6db, by hand (14:27; b1 hour 16, b2 hour 17)

W18-b: the refusal witness fires on both arms (b1 refused=8192 by
hour 16, b2 4096), and NOBODY IS HELD: the refused bodies (uid 53
on b1, 130 on b2) went on to arrive at cook and haul jobs within
minutes, 0 stalls and 0 census lines of their own. Pit drops at
(7712,6306): 0 on both (the W18-b bar's first half PASSES so far).
Starving 0 both. Bob lines 5 / 11, no bobber past 2. The COST shows
on b2: FETCH STALLED 19 by hour 17 (1 on the whole W18-i day; 4-13
on the days before), clustered at the refused ledge (7640,6436):8,
(7644,6436):3 -- the drops became stalls, as W18-b's registration
said they would; the ramp plot row (Ben's roadmap) is that ledge's
real answer. b1 stalls 8 (6-18 on the W16-b days). Arrivals 724 /
545 at hours 16 / 17, on pace for both bars.

W14-e2: on b2 the bench fires (2 by hour 17, colonist 120's jobs 310
and 453, exhausts=2 and 3 -- one bench carried a strike from another
arm, the label names the last striker). On b1 ZERO benches against
374 LONGEST-EXHAUST by hour 16 (bar <= 60/day: FAILED), whole-town
371, ends repeating 26x, 26x, 21x; 149 of the 374 (40%) exhausted
with closest_dist=0: the search VISITED its end cell and still
spent 61k states. The top end (7607,6272,181) is ONE block from its
body (7608.3,6272.5,181.0), which re-asked the whole-town search 21
times at 15 s intervals for five minutes, jittering 0.2 blocks in
y; the goal endf z=180.0 sits inside Rock (end_snap_dz=1), and 94%
of all Longest-tier search steps carry end_snap_dz 1-2 -- goals one
or two blocks inside terrain. No id-bearing line places that body
(no job line at the target, no glide line, no arrival): a jobless
colonist or one of the 180 vanilla rtsim NPCs; either way W14-e2's
consumer (the job loop's active-job branch) never sees it -- A
CONSUMER PIN MUST COVER THE PRODUCER'S PATH, third instance.

The fixed point: after a Longest exhaust the chaser sets
flee_from=pos and keeps path_length=Longest across targets (reset
only on a complete route); the flee heuristic (path.rs 1662-1670)
subtracts 10*sqrt(nd)*(cos+0.1) from d. But the arithmetic does not
flood 61k cells on its own with g(E) of a few units (f goes
negative only within ~30 blocks of the body), so either the S->E
edge is priced in the hundreds, or E is reached only by a long
detour (a fence or wall rule between the body and a target one
block away). The exhaust diag does not print g(E). NEXT: an
instrument row -- the exhaust names its end's g, its state count and
the max g popped -- then the fix (the flee term clamped, or the
price named). W14-e2 stands on b2's witness; its b1 bar FAILED for
the stated reason.

### W18-c registered (14:15): THE BOB IS NOT PROGRESS

Not gated on W18-b's first read after all: a bob climbs back, so its
landing has a way up by construction, and W18-b's refusal never
touches the class. The instrument (W18-i) named the mechanism: the
stuck clock's displacement window (tightdig_measure, every
TIGHTDIG_WINDOW 2.0 s) takes pos.distance(anchor) in three
dimensions against TIGHTDIG_MIN_PROGRESS 1.5; a two-block bob
displaces 2.0 in z alone, so every window reads as progress and no
stall, timeout, shun or strike reaches the body (colonist 75:
stuck_time 0.033 s at bobs 64 and 128). TWO FRAMES COMPARED AS ONE:
a walk and a bob in one distance.

MECHANISM: tightdig_displacement(pos, anchor) = the x-y distance;
the window asks it instead of the 3-D distance. Nothing else in the
measure changes (window, minimum, steer switch, segment check). No
new log line: the bob witness's stuck_time and the stall counters
read the change. Pin (bastion-server) the_bob_is_not_progress: a
two-block vertical oscillation displaces nothing; two over displaces
two; two over and two down displaces two. Falsifier plants the 3-D
distance back -> red on the bob.

PREDICTION (b1 fresh after E2-t's night-1 block, b2 fresh after
E2-t's day-1 block; wait-w18c-b1/b2): b2 bob peak <= 16 for the day
(128, 242, 221 before), stuck_time on any bob line at >= 64 above
0, FETCH STALLED and STUCK CENSUS <= 2x the day before (the bobs
become stalls: the honest cost), starving sleepers 0, arrivals >=
720; b1 bob peak <= 16, arrivals >= 900, starving <= 1. FALSIFIED
if a bobber still reaches 64 with stuck_time 0 (another reset path:
the steer switch at 2.0 blocks, or best_dist improving from the drop
itself), or arrivals fall > 10% (the window's z-credit was counting
real walks).

Rejected: counting the bob inside the witness (the witness stays
blind to its fix); a smaller window or larger minimum (a taste
number; a 2.0 bob clears any minimum a walk must pass). NOT
evidenced: the gate-OFF branch (unchanged); the assist and glide
overrides that lift bodies on purpose (they gain no ground either
and now read as stalls when they never do -- which is what they
are). Chain behind E2-t's stage (+300 s); the dry tree HEAD + E2-t +
W18-c applied clean; types checked (pos and anchor both Vec3<f32>).

### E2-t registered (14:03): THE SUPPER OUTRANKS THE POSTED HAUL

b1's replicate-2 night (203321df48): starving sleepers 2. Colonist
70's night pick at its own shelf read NIGHT SHELF EMPTY fourteen
times: present=1, units=1, reserved=1, refused_cap=1, holders
["988:haul:None"], verdict Refused, night_no_food reaching 1,024;
then RestAt Arrived at hunger 0.00. The pick admits a home item
only when has_capacity (reserved_count < amount), and the shelf's
one unit carried one reservation held by job 988, a Haul with
claimed_by None: posted, unclaimed, holding the sleeper's supper
through the Sleep block -- a guard starving what it protects.
Colonist 105 (SUPPER CARRIED HOME at 17:28, then RestAt Traveling
at 0.01) is a different shape, not evidenced here. Mechanism:
`reservation_yields_to_owner(claimed, class)` (an unclaimed haul
yields; a claimed job or another class stands);
`JobBoard::yield_unclaimed_hauls_on(item)` clears and releases such
reservations; at the sleeper's night pick, before pick_food, every
food item in its home yields; witnessed at the first eight and
powers of two as "THE SUPPER OUTRANKS THE POSTED HAUL" (colonist,
yielded, home_min). Pin `the_supper_outranks_the_posted_haul`;
falsifier plants the class test to a name no job has. Bars: b1
starving sleepers after night 1 <= 1 (2 tonight), NIGHT SHELF EMPTY
with unclaimed-haul holders 0 (14), the yield witness >= 1 on any
night such a reservation exists, haul releases <= 1.5x, arrivals >=
900; b2 starving sleepers 0, arrivals >= 720. Falsified if a sleeper
still starves with the yield witness at its home, or haul releases
rise by more than half. Rejected: never reserving from private
shelves (the shift's-end hauls are the design; only the unclaimed
case starves); yielding claimed hauls; cancelling the haul job.
Chain behind W18-b (stage ~14:30); readers behind the W18-b readers.

### W18-b landed (dfa366b6db, staged 14:03)

Check and pin green on the chain (1 test passed; the bridge
closure's scope held), both halves built fresh, the binary verified
by its contents ("THE DROP HAS NO WAY UP" present once in
stage-bin); shipped to lab-bin at 14:04. The falsifier planted the
radius zeroed at 14:05 and the pin went RED at 14:08, the tree
restored clean (0 dirty); E2-t's chain fired at 14:08. Both arms
booted this pair within the minute (b2 14:06, b1 14:08; witness
reach=2): its first day carries the pit fix, the flood fix and the
bob witness together; the first-hour reads are at ~14:40. Ten rows
landed today with red falsifiers.

### W14-e replicate 2: b1 hour 19 (203321df48 under the W14-c reader; 13:41) -- the flood at full scale, unconsumed

LONGEST-EXHAUST 629 by hour 19 (90 on replicate 1 for the whole
day: the flood's day-to-day range on this arm is 21 to 654),
LONGEST-TIER lines 53,901, top ends 41x(7632,6281,182),
33x(7685,6459,182), 31x(7679,6203,181), benches by Longest
exhausts 0 (the reset defect, W14-e2 boards b1 at the next
restart), route-proof benches 5; CLIMB BANNED (fetch) 3, other 1,
PROMISED CLIMB TAKEN 3, FETCH STALLED 17, budgets expired 19, STALL
BLAMED 3, stuck 9, arrivals 744, starving 0, p95 738. W14-e FAILED
twice as landed; W14-e2's first day is the read that matters.

### W14-e2 landed (537c8031e6, staged 13:38:38; both halves, common/ changed)

Check green across both crates, the common-crate pin green on the
chain (1 test passed, 714 filtered), the client rebuilt with the
server; shipped to lab-bin at 13:39. The falsifier planted the
partial arm zeroed at 13:40 and the pin went RED at 13:43, the tree
restored clean (0 dirty); W18-b's chain fired at 13:43 (stage
~14:10). Nine rows landed today with red falsifiers. b1 boards this pair at the W6-D reader's
restart after the replicate-2 night block (~14:07): the flood fix's
first ledge-arm day, whose first-hour witness (benches "three
exhausted Longest searches" >= 1 once an end repeats three times)
is read by hand at hour ~12 (~14:40), per the memory filed today.

### W18-b registered (13:36): THE BODY DOES NOT DROP INTO A CELL IT CANNOT LEAVE

Mechanism: `drop_has_way_up(standable, landing, edge_z)` -- a
breadth-first walk from the landing over colonist-standable cells,
one block up or down per step, within `DROP_EXIT_RADIUS` (4) in x
and y, true when it reaches a cell at least as high as the edge.
Both surface probes (the walk probe and the pure-glide bridge's
`surface_at`) consult it on the -2 arm only: a -2 landing without a
way up is skipped, the probe finds no surface, the body holds at
the edge and the stall's consumers act; steps of 0, +1 and -1 are
untouched. A refused drop is counted (DROPS_REFUSED) and logged at
powers of two as "THE DROP HAS NO WAY UP" (uid, landing, edge_z,
count, the probe). Pin `the_body_does_not_drop_into_a_cell_it_
cannot_leave` (a two-deep pit whose floor meets only its rim: no
way up; a ramp cell one up beside the floor: a way up; a one-block
edge with a cell one up beside it: a way up); falsifier plants the
radius zeroed. Bars, b2: mover drops of two or more at (7712,6306)
0 for the day (39, 5, 3), the refusal witness >= 1 there, starving
sleepers after night 1 0 (1 tonight), the bob peak <= 8, FETCH
STALLED <= 2x the day before, arrivals >= 720; b1: arrivals >= 900,
starving <= 1, the bob peak <= 8. Falsified if a starving sleeper
appears with the refusal witness at its cell (holding at the edge
starves as the pit did: then the edge needs the router), if
arrivals fall > 10% (the refused edges were routes), or if the
pit's drops continue (another writer takes the step). Rejected:
the probe without -2 at all (the trunk's ground-following holds
two-down steps at every terrace, TRUNK_REJECT_DZ 2); a two-up
assist out of the pit (a teleport; Ben's open judgement); pricing
the drop in the router (the router already plans none). Chain
behind W14-e2 (committed 537c8031e6 at 13:35, building; W18-b's
stage ~14:20); readers behind the W14-e2 readers.

### W14-e read: b2 day 1 (203321df48 under the W6-D reader; 13:24)

Arrivals 649 (740, 907, 835 on the days before), exhausted 896,
probes 67 (44 cut_off, 23 sealed), CLIMB BANNED (fetch) 0, other 4
(colonist 125: 3 across jobs, no bench by design), PROMISED CLIMB
TAKEN 4, FETCH STALLED 10, budgets expired 28, STALL BLAMED 1,
unreachable 0, starving sleepers 1 (colonist 16, the pit), W14-d's
benches 3 (jobs 490, 756 and one more: the stuck-branch consumer
fires more on b2 than on b1), Longest benches 0 (W14-e's reset
defect), p95 569.

### The pit cost a meal (b2, 203321df48, hours 20-0; 13:25) -- the pit is the row

The two-minute watch: hour 21 starving 3 (16, 97, 100, all EatFrom
Traveling at hunger 0.00); hour 22 starving 1 (16); hour 23
colonist 16's job is RestAt (bed (7589,6370,181)), still starving;
hour 0 "Arrived", still starving -- a night starver, the class the
morning's E2 rows closed for the stalled-target case, made here by
the pit: 39 mover drops at (7712/7713,6306) from z 181 to 179
through the evening, its feet in the drop cell at 13:23. The
registered condition ("a meal is lost to this cell") is met. The
row: THE BODY DOES NOT DROP INTO A CELL IT CANNOT LEAVE -- the
mover's surface probe takes a -2 step only when a neighbour of the
landing cell stands within one block above it (an exit the glide
can take); otherwise the body holds at the edge, a stall, which
has consumers (the shun, the stuck timeout, the strikes). Prior
art: every colony sim's mover walks a grid whose down-steps are
the same as its up-steps (RimWorld, Banished: no vertical at all;
DF: ramps both ways); a one-way drop is a trap in all of them.

### The pit's third day (b2, 203321df48, hour 20; 13:20) -- now with hunger

STARVING 3 at hour 20 (colonists 16, 97, 100; all EatFrom
Traveling, hunger 0.00-0.03). Colonist 16 has 39 mover drops
today and 97 five, both last dropped from z 181 to 179 at
(7712/7713,6306): the same two-deep cell that held 33 and 62 on
the W14-c day and 134 on the W6-D day. Thirty-nine drops in a day
by one body at one cell is the bob (W18-i's class) at a pit on a
store route: fall in, climb out one and one, fall in. Whether the
three eat before day 1 is being watched every two minutes; W18-i
boards b2 at the next restart (~13:40) and names the pit's bobs
with the stuck clock's state. If a starver stays or a meal is
lost to this cell, the pit becomes the row: the mover's -2 step
where the router plans none.

### W14-e read: b1 night 1 (203321df48; 13:11) -- the day closed FAILED, as read at hour 19

LONGEST-EXHAUST 90 for the day (65x one end, (7712,6345,186); 18x
colonist 55's wall), LONGEST-TIER lines 7,571, benches by Longest
exhausts 0 (the partial-route reset; W14-e2 chained); route-proof
benches 17, W14-d's bench 1; CLIMB BANNED (fetch) 1, other 8 (three
colonists at two each: no loop), FETCH STALLED 14, budgets expired
18, STALL BLAMED 3, stuck 9 (5 at the census); arrivals 974 (1,090
on the decision day, 945-1,013 before); starving: no STARVING
COLONISTS line all day; p95 685. b1 restarts onto the latest pair
under the W14-c reader (203321df48 again until W18-i stages).

### W18-i landed (0d603edbae, staged 13:16:32)

Check and pin green on the chain (1 test passed), both halves built
fresh, the binary verified by its contents ("THE BODY BOBS" present
once in stage-bin); shipped to lab-bin at 13:17. The falsifier
planted the cell test at 100 blocks at 13:18 and the pin went RED
at 13:21, the tree restored clean (0 dirty); W14-e2's chain fires
five minutes after the stage (stage ~13:48, both halves, common/
changed). Of the pit's three starving at hour 20 on b2, two had
eaten by hour 22 and colonist 16 (39 drops at the pit today) was
still Traveling to eat at hunger 0.00; the watch runs to day 1. The readers restart b1 after W14-e's night block
and b2 after W14-e's day-1 block; with the arms cycling every ~35
minutes, the first bob read comes from whichever reader restarts
onto this pair or a later one (the bob tally is run by hand on any
log carrying 0d603edbae or newer).

### W14-e read: b1 hour 19 (203321df48 under the relaunched W2-b-r reader; 12:56) -- FAILED by its own defect

LONGEST-EXHAUST 78 by hour 19 (bar <= 60 for the day: FAILED),
LONGEST-TIER lines 6,473, top ends 58x(7712,6345,186) (bar <= 6:
FAILED), 18x(7742,6404,181), benches "three exhausted Longest
searches" 0 with an end asked fifty-eight times (bar >= 1: FAILED)
-- the falsifying condition ("an end repeats ten or more times with
zero benches") met, for the reason read at hour 17 and fixed by
W14-e2 (the partial-route reset), which is chained. The rest of the
day: arrivals 752 (882 on the decision day at hour 19; 728-790 on
the reach-2 days before), CLIMB BANNED (fetch) 1, other 2, FETCH
STALLED 11, budgets expired 16, STALL BLAMED 3, stuck 3, starving
0, route-proof benches 17, W14-d's bench 1 (job 666, colonist 808),
p95 624. Disposed: W14-e FAILED as landed; its pin held a rule the
Chaser undid; W14-e2 carries its bars.

### W14-e read early, and its defect (b1, 203321df48, hour 17; 12:53) -> W14-e2 registered (12:56)

LONGEST-EXHAUST 31 by hour 17 (a quieter flood day: 158 by hour 12
on the decision day), LONGEST-TIER lines 3,295, top ends
18x(7712,6345,186) and 12x(7742,6404,181) (colonist 55's wall
again) -- and "three exhausted Longest searches" benches 0 with an
end asked eighteen times. Read in the Chaser: all three route-set
arms of search_step_inner reset longest_exhausts, and two of them
are the exhausted search's own result (PathResult::None and
PathResult::Exhausted return the partial path to the closest node
and the chaser walks it), so every exhausted search zeroed the
count it had just earned; the count never passed one. W14-e's pin
covered fresh_exhausts, not the count's life in the Chaser: a
mechanism that could not show itself failing until read live.
W14-e2: `exhaust_count_after(complete, count)` -- the count on a
partial route, zero on a complete one (PathResult::Path); all three
sites ask it. Pin `the_partial_route_does_not_forgive_the_exhaustion`
(veloren-common); falsifier plants the partial arm zeroed. Bars:
W14-e's own, re-registered (LONGEST-EXHAUST <= 60/day, top end <=
6, benches >= 1 when an end repeats 3x and <= 15/day, arrivals >=
900, starving <= 1; b2 benches <= 15, arrivals >= 720). Falsified
if an end repeats >= 10 with zero benches, or benches > 15. Chain
behind W18-i (stage ~13:50; common/ rebuilds both); readers behind
the W18-i readers. Other numbers at hour 17: other bans 2, fetch 1,
stalls 10, route-proof benches 11, W14-d's bench 1 (job 666,
colonist 808), arrivals 678, starving 0.

### W18-i registered (12:53): THE BODY NAMES ITS BOB (instrument)

Found by the mover-drop tally across three b2 days: colonist 75
dropped two blocks 242 times in fourteen minutes at one cell
(7791,6242,184) on the 5f785e2a5a day; colonist 149 221 times in
twenty-three minutes on the 5fb1fc4aee day; colonists 30 and 139
86 and 47; 25-30 colonists a day drop two or more blocks three or
more times. Every drop is the mover's own write (POS-WRITE
site="mover", dz -2) and no other writer signed a lift: the probe
(dz 0, 1, -1, -2) takes the -2 step at a terrace edge and the
next steps take +1 and +1 below the diag's 1.5 threshold. Colonist
75's lines in the span: the pump's memo refusing, the exhausted
search naming its target, GLIDE INTO A WALL (whose sampled counter
reads 262,144 or more ticks a day on each of the three days) --
and no stuck-clock line in fourteen minutes. A body bouncing two
blocks every few seconds is what a player sees first. Instrument
before hypothesis: `bob_repeats(prev, cell, now)` (a two-block drop
within one block of the body's previous drop and within 900 ticks)
at the mover's write; a bob increments the body's count and logs
"THE BODY BOBS" at powers of two with uid, cell, z_from, z_to, the
push site and the active job's stuck_time; a drop elsewhere resets
the count. No behaviour change. Pin `the_body_names_its_bob`;
falsifier plants the cell test at 100 blocks. Bars: on any b2 day
whose drop tally shows a colonist at 100+ drops, the line names it
with bobs >= 64 and its stuck_time; lines < 200/day. Falsified if
the tally shows such a colonist and the line never names it (the
drops are not at one cell within the window: another shape).
`bob-tally.sh` reads it. Chain behind W14-e (stage ~13:25);
readers behind the W14-e readers.

### W14-d read: b2 day 1 (89bc78af1d under the W14-c reader; 12:46)

Arrivals 740 (907, 835, 641, 807-846 on the days before), exhausted
(pump census) 1,072, probes 68 (42 cut_off, 26 sealed), CLIMB
BANNED (fetch) 0, other 1, FETCH STALLED 4, budgets expired 8,
unreachable 0 all day, starving 0 at day's end (colonist 50 read
starving at 12:47 after seven mover drops and stood at z 184 by the
census: out again), p95 469. W14-d's bench count stays at the one
from +10 (job 536, colonist 59): its arm is rare on b2 as on b1.
b2 restarts onto 203321df48 (W14-e) under the W6-D reader.

### Observation, not a row (12:43): the night's route-proof burst is W17-b's three-above case

The W2-b-r b1 day's 83 "three failed route proofs" benches were a
night burst: 78 between 10:50 and 10:59, 22 distinct jobs, 18 of
them benched more than once, 24 UNREACHABLE RETRY latch clears --
the jobs of colonists 61 and 52, both from_in_house=true with the
door probe reading door_below and the column '#~~~~~~~~~' (solid at
feet-3): sleepers on beds THREE above their floor, the case W17-b
named as not evidenced (its start looks one and two below and
returns the feet, still an island). The approach lane proves
Unreachable (the frontier empties at once: a cheap proof), the job
is benched, the arbitration latch clears it, the proof repeats.
Cost: churn in the bench counter; outcome: none (starving sleepers
0 that night; the decision day read 5 such benches all day).
Candidate W17-c (the start looks three below, or to the house's
floor) with its magnitude recorded; built only if the class costs
a meal.

### THE DECISION: the reach-2 trunk STANDS (b1, 89bc78af1d, hour 5 of day 1; 12:33)

The registered rule's three numbers on the first full ledge-arm day
with W6-D aboard: CLIMB BANNED (other) 1 (rule <= 20), benches by
banned climbs and terminal searches 0 with route-proof benches 5
(rule <= 15), arrivals 1,058 for the day (rule >= 950; the highest
day the ledge arm has had: 945-1,013 before). Starving 1 at hour 5
(colonist 134, hunger 0.02, EatFrom Traveling: walking to eat; the
night's bar <= 1), CLIMB BANNED (fetch) 2 for the day, FETCH
STALLED 8, stuck 10, p95 682. The formal night block at 12:35
closed the day at arrivals 1,090, other bans 5 (the night's), fetch
bans 2, stalls 8, starving sleepers 0, stuck 19, p95 670: the three
numbers hold. Verdict by the rule as registered:
the reach-2 trunk stands, and W2-b's reach 1 is not re-applied.
Honest caveats, recorded with the verdict: the wall-climb loop
class did not occur today (one ban, one colonist) against 154 and
42 on the two reach-2 days before, so the "other bans" number
passes partly on the day's mix, and W6-D's cap is aboard but
untested on both arms (no colonist reached three banned climbs);
the whole-town flood ran unconsumed all day (LONGEST-EXHAUST 654,
LONGEST-TIER lines 58,654, top ends 73x/60x/50x) because W14-d's
arm is bypassed by the glide, and W14-e -- counting at the source
-- boards b1 at the restart that follows this block. The trunk's
reach is next judged only if W14-e fails its own bars.

### W6-D read: b2 day 1 (5fb1fc4aee under the relaunched W2-b-r reader; 12:15)

Arrivals 907 (835, 641, 807-846 on the days before), CLIMB BANNED
(fetch) 2, other 2 (two colonists, one each: nothing for W6-D to
cap, benches 0 -- untested on b2 as on b1), FETCH STALLED 5,
budgets expired 6, unreachable 0 all day, starving 0 at day's end
(colonist 134 read starving at 12:07 in a drop cell at (7698,6304)
after two mover drops, 185.9 -> 183 -> 181, and was fed by the
day's end: the pit class again, left again), p95 555. Exhausted
(pump census) 1,111 -- the series with W16-b aboard runs 1,043,
758, 844, 748, 1,111 against 513-598 before it, components same
30 / start_unlabelled 27 / untrusted 9. It is a response counter
(Medium-tier fill searches spent), not an outcome: arrivals, bans,
stalls and starving all read better than before W16-b, so it is
recorded as W16-b's cost, not a row. The b2 flood tally reads 0 by
construction (no LONGEST-TIER diag env on b2).

### THE DECISION DAY, hour 19 and hour 21 (b1, 89bc78af1d: reach 2 + W16-b + W6-D, W14-d dead; 12:12 / 12:15)

The registered rule's numbers: CLIMB BANNED (other) 1 by hour 21
(rule <= 20; 154 and 42 on the two reach-2 days before: the loop
class did not occur today -- one colonist, one ban -- so W6-D's arm
had nothing to cap and reads untested, benches by banned climbs 0);
benches by the new kinds 0 and by route proofs 5 (rule <= 15);
arrivals 882 at hour 19 and 949 at hour 21 (rule >= 950 by night:
the night block decides, and the trend passes it); starving 1 at
hour 21 (the evening's hunger); CLIMB BANNED (fetch) 0 for the day
so far -- the ledge arm's best number yet, at reach 2 with W16-b
(1 on the reach-1 day, 3-4 on the reach-2 days before); FETCH
STALLED 6, budgets expired 7, stuck 10, p95 682. The flood is at
full scale and unconsumed: LONGEST-EXHAUST 561, LONGEST-TIER lines
49,855, top ends 60x(7631,6280,182), 50x(7632,6281,182),
42x(7688,6459,182) -- W14-d's benches 0 as diagnosed; W14-e is its
consumer and ships ~12:40. By the rule as registered (other bans,
benches, arrivals) the reach-2 trunk STANDS, provisionally on the
night's arrivals; the flood was never in the rule because its
consumer was thought aboard, and it is now judged by W14-e's own
bars on the day after.

### W14-e landed (203321df48, staged 12:26:49; both halves rebuilt, common/ changed)

Check green across both crates and the pin green on the chain (1
test passed), the client rebuilt with the server, the binary
verified by its contents ("three exhausted Longest searches" present
once in stage-bin); shipped to lab-bin at 12:27. The falsifier
planted the difference zeroed at 12:28 and the pin went RED at
12:32, the tree restored clean (0 dirty). b1 boards this pair at
the W2-b-r reader's restart after the decision day's night block
(~12:38): its hour 19 (~13:05) and night (~13:25) are the first
read of the flood with a live consumer. Seven rows landed today
with red falsifiers (W16-b, W17-b, W2-b-r, W14-c, W6-D, W14-d,
W14-e); two of them (W14-c, W14-d) hold rules the live population
rarely or never calls, and say so.

### W14-e registered (12:05): THE THIRD LONGEST EXHAUSTION STRIKES THE JOB

The flood's consumer, placed where the re-ask lives. The Chaser
(common/src/path.rs) keeps `longest_exhausts`: +1 in the Longest arm
of its escalation (one whole-town search spent), reset to 0 at the
three route-set sites, 0 by Default, exposed on
ChaserDiagnosticSnapshot. The job loop reads it every tick before
the progress check (not in the stuck branch), keeps
`exhausts_struck` per walker (cleared with the terminal streak at
the job's end), and strikes the held job once per fresh exhaustion
through `job_strike`: `fresh_exhausts(count, struck)` = count -
struck, saturating; the third strike benches with "three exhausted
Longest searches" (job, colonist, job_pos, exhausts). Pin
`the_third_longest_exhaustion_strikes_the_job`; falsifier plants
the difference zeroed. Bars: b1 LONGEST-EXHAUST <= 60/day (33, 558,
617, 21), top end <= 6 (15-57), benches >= 1 whenever a walker's
count reaches 3 and <= 15/day, arrivals >= 900, starving <= 1; b2
benches <= 15, arrivals >= 720, exhausted <= 900. Falsified if
LONGEST-EXHAUST stays >= 100 with zero benches (the count does not
reach the loop) or benches exceed 15. Rejected: striking in the
Chaser or the scheduler (common/ knows no jobs; the scheduler's
board is read-only); a per-tick strike without the struck mark.
common/ changed: both halves rebuild. Chain behind W14-d (fires at
once, stage ~12:35); readers behind the W14-d readers.

### W14-d, a correction (12:31): its arm is rare, not dead

b2's +10 block on the W14-d pair (89bc78af1d, hour 14): "three
terminal chaser searches" benched job 536 for colonist 59 -- the
stuck-branch consumer does fire where a body stands still at its
terminal state. It is rare (one bench on b2 by hour 14; one
observation a day on b1 against 158-561 whole-town exhausts), so
the disposal stands in substance -- the flood's consumer had to
count at the source (W14-e) -- but "unreachable" was too strong:
the arm is reachable and nearly always bypassed by the glide.

### W14-d DISPOSED: FAILED AS A MECHANISM (12:00) -- its arm is unreachable too

The decision day's log at hour 12 (89bc78af1d): LONGEST-EXHAUST
158, LONGEST-TIER lines 13,824, three ends asked 15-26 times, and
"three terminal chaser searches" benches 0; the CONN-SHADOW census
reads chaser_terminal_releases=1 for the whole morning. The
chaser_terminal branch sits inside the job loop's STUCK branch (no
progress this tick), and a chaser that has exhausted Longest sets
flee_from and re-asks while the body keeps gliding toward the
target -- the body makes progress, the stuck branch is skipped, and
the (Longest, Exhausted) state is observed once a day. So the
terminal streak never reaches six, the lift never fires, and
W14-d's strike has no live caller: EVERY GATE ARM MUST BE
REACHABLE, and this is the second row today (after W14-c) whose
arm was not. Its pin holds a rule with no caller; its bars are
void. The producer is now certain (the chaser's own re-ask, seen
in the scheduler), and the consumer must count the exhaustions
where they happen: in the Chaser itself (common/src/path.rs), as
a per-target count of consecutive Longest exhaustions exposed on
its diagnostic snapshot, read by the job loop every tick and
struck at three -- reachable by construction. Registered next as
W14-e once the Chaser's fields are read.

### W14-d landed (89bc78af1d, staged 11:47:22)

Check and pin green on the relaunched chain (1 test passed), both
halves built fresh, the binary verified by its contents ("three
terminal chaser searches" present once in stage-bin); shipped to
lab-bin at 11:47. The falsifier planted the threshold at 60 and the
pin went RED at 11:52, the tree restored clean (0 dirty). The W17-b
b1 reader restarted b1 onto this pair at 11:48 (witness reach=2):
that day (hour 19 ~12:20, night ~12:40) is the decision day with
both consumers aboard, read with `decision-tally.sh` since that
reader predates the tallies. Six rows landed today with red
falsifiers on their planted defects (W16-b, W17-b, W2-b-r, W14-c,
W6-D, W14-d); W14-c's is red on a rule with no live caller.

### W2-b-r replicate 2, early (b1, 5f785e2a5a, hour 13; 11:16) -- THE FLOOD IS BACK AT REACH 2

LONGEST-EXHAUST 166 by hour 13 (19 by hour 20 on replicate 1;
617 for the W2-b day; 13-33 on the other days), LONGEST-TIER lines
14,927, top ends 23x(7615,6271,182), 18x(7631,6280,182),
14x(7679,6203,181) -- three ends re-asked 14-23 times each by
midday; fetch bans 3, other bans 2 (154 yesterday: the loop class
is the day's mix), stalls 16, arrivals 539 at hour 13, route-proof
benches 2. The trunk witness reads reach=2. So the whole-town
flood is NOT what W2-b's reach 1 did to the town: it recurs with
the jump edges planned, on a second replicate, at the same order
of magnitude, and W2-b-r's own bar (LONGEST-EXHAUST <= 30 by hour
19) will fail today. The attribution in W2-b-r's DEFECT ("reach 1
disconnects the town") is falsified; what the two flood days share
is the chaser's terminal re-ask loop -- a target the chaser cannot
reach at any reach (behind a wall, above a ledge, or simply farther
than 75,000 iterations find), asked again from a flee point without
a consumer. W14-d (registered above, chained behind W6-D) is that
consumer. W2-b-r's revert stands on its other ground (the fetch
bans have a witness and a shun; the trunk's reach is now a named,
witnessed constant), and the trunk's reach is re-judged once both
consumers (W6-D for loops, W14-d for floods) are aboard, per the
registered decision rule. The memory that named reach 1 as the
flood's cause is corrected.

### W14-d registered (11:17): THE TERMINAL CHASER SEARCH STRIKES THE JOB

The flood's real consumer. The chaser climbs Small -> Medium ->
Long -> Longest and, exhausted at Longest from the same start, sets
flee_from and asks again (the 60,696-cell exhausts; colonist 55's
1,675 asks). The jobs system already recognises that state
(chaser_terminal: (Longest, Exhausted) with the last search target
at the job) and answers it with a goal verdict (reader gated OFF), a
claim penalty at the release (reader gated OFF) and a per-colonist
terminal streak whose sixth observation lifts the body two blocks;
nothing strikes the job. Mechanism: `chaser_terminal_strike(streak,
strikes)` = `job_strike` at `TERMINAL_STREAK_STRIKE` (6, the
existing one-shot edge), None below; the third episode benches with
"three terminal chaser searches". Pin
`the_terminal_chaser_search_strikes_the_job`; falsifier plants the
threshold at 60. Bars: b1 top exhaust end <= 4/day, LONGEST-TIER
lines <= 1,500 by hour 19, benches >= 1 when an end repeats 3x and
<= 15/day, arrivals >= 900, starving <= 1; b2 benches <= 15,
arrivals >= 720, exhausted <= 900. Falsified if the top end repeats
>= 10 with zero benches or benches exceed 15. W14-c's dead gate
stays in place, named. The strikes tally now counts all four bench
kinds; the W2-b-r b1 day already had 83 route-proof benches (the
approach arm's existing consumer, recorded). Chain behind W6-D;
readers behind the W6-D readers. The first chain failed its check
(the streak is a `HashMap<Uid, u8>`, the const was typed u32: the
dry tree validates anchors, not types); the tree was restored,
the const and the rule retyped to u8, the running falsifier killed
by its pid file before its plant strings were edited, and both
relaunched at 11:28 (stage ~11:55).

### W14-c and W2-b-r read: b2 day 1 (5f785e2a5a under the W16-b reader; 11:09)

Exhausted 844 for the day (1,043 on the W16-b day and 758 on the
W17-b day, both with W2-b; 598 with W2-b and W15-c-b; 513 with
W15-c alone). The reach-1 trunk is gone from this pair and the
exhaustion stays high: the doubling was NOT W2-b's flood, and by
the registered rule W16-b's walking bodies are re-examined for it.
Components: `same` 44, start_unlabelled 11, untrusted 7,
target_unlabelled 5 -- the exhausted searches now mostly join
endpoints the 1-up labeller calls connected (W15-i5's question,
sharpened). Arrivals 641 (808, 807, 846 on the three days before:
-20%; the registered bar >= 760 FAILED on this replicate); CLIMB
BANNED (fetch) 0, other 5 (colonist 55: 3), FETCH STALLED 6,
budgets expired 15, unreachable 0 all day (W17-b's class 0 again,
open 0), starving 1 at hour 0 (colonist 62, Traveling to eat),
p95 497. W14-c: fill benches 0 with memo refusals at 16,384 (the
saturating counter) -- 844 exhaustions and not one third strike is
suspect: the strike is gated on PathLength::Longest, and if the
fill lane enqueues at a lower tier the row has no consumer.

### W14-c DISPOSED: FAILED AS A MECHANISM (11:16) -- its live arm is unreachable

Checked: the pump's fill lane enqueues its approach search at
PathLength::Small (500 iterations) and its exact search at Medium
(5,000); it never runs Longest. A BudgetExhausted at 5,000
iterations "says NOTHING about reachability" (the enum's own doc);
the proof is FullPathOutcome::Unreachable (the frontier emptied),
which the approach arm already strikes. So W14-c's strike, gated on
Longest, cannot fire in the live population -- EVERY GATE ARM MUST
BE REACHABLE, and this one is not; its pin holds a rule with no
caller. The Longest-tier floods on b1 (colonist 55's 1,675 asks,
the 60,696-cell exhausts) were not the pump's fill lane at all:
they were the agent chaser's own escalated searches (path.rs, the
Chaser climbs Small -> Medium -> Long -> Longest and asks again),
and the "fill" attribution in W14-c's DEFECT was wrong -- a number
without its producer. The pin and the shared `job_strike` rule stay
(the approach arm uses it, identity); the dead gate is left in
place and named here rather than churned; the flood's real consumer
belongs at the chaser's exhausted Longest search, read next. W14-c's
registered bars are void (the mechanism never ran), not failed.

### W6-D landed (5fb1fc4aee, staged 11:17:58)

Check and pin green on the chain (1 test passed), both halves built
fresh, the binary verified by its contents ("three banned climbs"
present once in stage-bin); shipped to lab-bin at 11:18. The
falsifier planted the gate inverted at 11:19 and the pin went RED
at 11:22, the tree restored clean (0 dirty); W14-d's chain fires
five minutes after the stage. b1 boards this pair at the W17-b
reader's restart after W16-b's night block (~11:55): the first day
under the decision rule (other bans <= 20, benches <= 15, arrivals
>= 950) reads at hour 19 ~12:25 and night ~12:45.

### Observation, not a row (b2, 5f785e2a5a, hours 21-22): a two-deep pit, left again

At hour 21 two colonists (33, 62) stood starving in a two-deep pit
at (7712,6306,179) after the mover's surface probe (dz 0, 1, -1,
-2) dropped one of them from z 181; the chaser's next node sat two
up and the glide override walked at it. By hour 22 both had left
the pit (62 at z 181, Traveling to eat; 33 fed) and starving read 1
(the evening's hunger, E2's class). Named and not chased: the
mover's -2/-3 drops run ~160 a day on b2 with W16-b aboard (1,312
on the W2-b day without it: W16-b cut them eightfold), the router
plans no two-down move (DIRS) and prices falls at 3.5 a block, and
`TRUNK_REJECT_DZ` 2 lets the coarse trunk's ground-following hold
a two-down step. `pit-tally.py` (a starver against its last mover
drop) is the instrument if a night starver ever sits in one.

### W2-b-r read: b1 night 1 (d4760fa9fd; 11:01) -- the flood bars PASSED, and the climb loops are the cost

Flood: LONGEST-EXHAUST 33 for the day (617 on the W2-b day, 21 on
W16-b's reach-1 day, 13 before W2-b; the bar <= 30 was at hour 19,
where it read 19: PASSED), LONGEST-TIER lines 4,309 (51,798 /
2,315 / 1,642), top ends 6x(7711,6359,188), 4x(7742,6404,181),
4x(7706,6346,181). Arrivals 977 (1,013 on W16-b's night, 915 on
W2-b's, 933 on W15-c's: -3.5%, within the bar); starving 0; stuck
21 distinct (18, 8); p95 649. CLIMB BANNED (fetch) 4 for the day
(1 / 9 / 15 / 24), all trunk-frame under a +2 edge; FETCH STALLED
11 (6 / 18); PROMISED CLIMB TAKEN 3. The cost, and it is the worse
witness: CLIMB BANNED (other) 154 for the day (7 on W16-b's reach-1
day, 0 on W2-b's, 4-7 on the reach-2 days before W16-b) -- three
loopers: colonist 68 72 (job 745, the haul above the wall, looping
from 10:38 to 10:49 and never arriving), colonist 54 28, colonist
61 27. The reach-2 trunk plans the jump edge; W16-b walks the body
to the wall beneath it instead of standing still; the physics puts
it on_wall; the stuck timeout bans the column and releases the
job; the same colonist re-claims it. Under the reach-1 trunk there
was no jump edge to walk into (7 other bans), and the cost was the
flood, once, now capped by W14-c at three asks per job. Two
replicates of the reach-2 + W16-b day are needed before the trunk's
reach is judged again; the b1 restart onto 5f785e2a5a (W14-c, still
reach 2, no W6-D) under the W16-b reader is the second, and the
W6-D day after it reads the loops capped.

DECISION RULE, registered now: after one full b1 day under W6-D
(reach 2 + W16-b + the strike): if CLIMB BANNED (other) <= 20,
benches by banned climbs <= 15 and arrivals >= 950, the reach-2
trunk with strikes stands; otherwise the trunk goes back to reach
1 with W14-c as the flood's consumer (the reach-1 + W16-b day read
bans 1, other 7, exhausts 21, arrivals 1,013), and that is not a
flip-flop but the three-replicate comparison the loop demands.

### W14-c and W2-b-r read: b2 +10 (5f785e2a5a under the W16-b reader; 10:53, hour 14)

Exhausted 43 (35, 47, 30, 40 at +10 on the four pairs before), probes
42 (29 cut_off, 13 sealed), unreachable 0, CLIMB BANNED (fetch) 0,
other 2 (two colonists, one each), stalls 0, budgets 0, arrivals 388
(421 at hour 13 on the W16-b pair), p95 428, starving 0; fill benches
0 with memo refusals 3 (no end has repeated three times yet); the
trunk witness reach=2. The day-1 block (~11:35) carries W14-c's b2
bars and the un-confounded W15-c-b / W16-b exhaustion read.

### W6-D registered (10:57): A BANNED CLIMB STRIKES THE JOB

The climb loop's consumer. The stuck-timeout release of a non-self
job with the body in Climb or on_wall bans the feet and ahead
columns (300 s) and writes a claim penalty whose reader is behind
`claim_penalty_enabled()` (BASTION_CLAIM_PENALTY, default OFF): a
writer without a reader, and colonist 68 re-claimed job 745 every
ten seconds. Mechanism: `banned_climb_strike(climbing, strikes) ->
Option<(next, bench)>` = `job_strike` when the release was a banned
climb, None otherwise; applied in the release arm's non-self branch
before the release; on the bench: unreachable + "UNREACHABLE PROVEN
-- job benched off the board (three banned climbs)" with job,
colonist, job_pos. Self-jobs (suspended, not released) untouched;
the fetch-lane ban (its consumer the walker shun) untouched. Pin
`a_banned_climb_strikes_the_job`; falsifier plants the gate
inverted. Bars: b1 other-bans per colonist-day <= 6 (58), per day
<= 20 (61), benches >= 1 when a colonist reaches 3 and <= 15/day,
arrivals >= 700, starving <= 1; b2 benches <= 15, other bans <= 40,
arrivals >= 760. Falsified if a colonist's other-bans stay >= 20
with zero benches (another release arm), if benches exceed 15
(reachable hauls benched), or if arrivals fall > 10%. Rejected:
switching the claim penalty's reader on (Ben's gate ruling stays
open; the strike's consumer is already on); the whole-wall ban (a
plot row, and still no consumer); striking every stuck timeout
(crowd stalls and door waits would bench the board). Chain behind
W14-c (fires at once: W14-c staged 10:33; stage ~11:25); readers
behind the W14-c readers. The loop tally (`loop-tally.sh`) reads
other-bans per colonist and the benches.

### W2-b-r read: b1 hour 19 (d4760fa9fd, the reach-2 trunk with W16-b and W17-b aboard; 10:45)

The flood: LONGEST-EXHAUST 19 by hour 20 (617 on the W2-b day, 21
on W16-b's reach-1 day, 13 before W2-b) -- bar <= 30 PASSED;
LONGEST-TIER lines 2,674 at hour 19 (bar <= 3,000 PASSED; 3,044 by
hour 20); the trunk witness reach=2 in the log. The registered bar
"whole-town floods 0" was ill-posed: every Longest-tier exhaustion
expands the whole town (its 75,000-iteration budget exceeds the
town's 60,696 cells), on every run before too; the count is the
number. Top exhaust ends 4x(7742,6404,181) -- colonist 55's wall
target from the W2-b flood, asked FOUR times today instead of
1,675: with the jump edge planned, the body walks, is banned and
shunned, and the pick goes elsewhere (the cheap path, as claimed).
Arrivals 772 (790 / 733 / 736; bar >= 700 PASSED); exhausted 21 (up
10, down 2, flat 9); stuck 4 distinct; p95 597; starving 0. The
recorded cost: CLIMB BANNED (fetch) 4 by hour 19 (1 on W16-b's
reach-1 day, 8 on W2-b's, 15-24 before), all trunk-frame under a
+2 edge one over (trunk_dxy 1, trunk_dz 1-2: the trunk itself holds
the jump again), push_site chaser-refused-rock 3 / chaser-probed 1;
FETCH STALLED 11 (4 / 12); the ledge tally 3+3 stalls and 4 bans at
the same two ledges. And a NEW counter: CLIMB BANNED (other) 37 by
hour 19, 61 by hour 20 (0 on W2-b's day, 7 on W16-b's), 58 of them
colonist 68 at one column (7698,6349,181) within ten minutes: a
live climb loop -- the ban prices the column out and the replan
comes back to it. Named: colonist 68 holds job 745, a Haul of item
782 to destination 79; every ten seconds the cycle is claim (ITEM
27 material-job claim committed) -> CLIMB BANNED (the column priced
out for CLIMB_BAN_SECS 300) -> RELEASE-DIAG class=haul reason=Other
site=38438 -> the same job re-claimed by the same colonist -> the
jump planned at the NEXT column of the same wall (feet x 7698 to
7706 along y 6349, z 181). Fifty-eight cycles in ten minutes. The
ban's only consumer is the column price, which the router answers
with the adjacent column; the job is never struck, so nothing ends
the loop but the day. This is the "infinite climb loop" Ben named,
now with a witness, and it exists at reach 2 regardless of W2-b-r
(0 on the reach-1 days because the trunk planned no jump and the
fill search flooded instead; 4-7 on the earlier reach-2 days: the
loop needs a haul to a destination above a wall to be claimed).
Next row: the ban's release strikes the held job with W14-c's
rule; three banned climbs bench it.

### W17-b read: b2 day 1 (e5c4d9965d; 10:38) -- its class PASSED, the total bar missed

Unreachable deliveries 8 for the day (24 on the W16-b day, 0 on the
W15-c-b day, 12-39 on the days the class occurred): 6 open-start, 1
boxed, 1 start-in-solid, and from_in_house 0 of 8. W17-b's own bar
(from_in_house <= 5) PASSED at 0 against 24 the day before; the
registered total (<= 5) missed by three, and the three-plus-five
that remain are other classes (an open start whose target the
search cannot reach; one boxed; one in solid). No door probe fired
(the probe fires on the in-house class). Exhausted 758 (1,043 /
598 / 513: W2-b still aboard), probes 67 (50 cut_off, 17 sealed),
components start_unlabelled 34 / same 15 / target_unlabelled 10 /
untrusted 7 / different 1. CLIMB BANNED (fetch) 3 (2 on_prev with
rise_next Some(2) at push_site chaser-refused-rock, 1 no_prev; all
trunk), other 1, FETCH STALLED 13 (1 on the W16-b day, 5 before:
above the two-day band -- recorded, watched on the next b2 day),
budgets expired 17, STALL BLAMED 4, arrivals 807 (808), starving 0,
p95 496. The W16-b reader restarts b2 next onto 5f785e2a5a (W14-c
+ W2-b-r): that day separates W2-b's flood from W16-b's bodies on
the exhaustion, and reads W17-b's stalls a second time.

### W17-b read: b2 +10 (e5c4d9965d; 10:16, hour 13)

Unreachable deliveries 7 (6 open, 1 start_solid; the day's in-house
class comes with the night), exhausted 51 (35 at +10 on the W16-b
pair, 47 on W15-c-b's), probes 50 (34 cut_off, 16 sealed), CLIMB
BANNED (fetch) 1, other 1, PROMISED CLIMB TAKEN 3, FETCH STALLED 4,
budgets expired 5, arrivals 520, p95 446, starving 0.

### W17-b read: b2 +4 (e5c4d9965d under the W15-c-b reader; 10:10, hour 10)

Unreachable deliveries 2, both start=Open and from_in_house=false
(not W17-b's class: that class is the night's sealed sleeper, read
at day 1); exhausted 16, probes 16 (11 cut_off, 5 sealed; components
untrusted 7, start_unlabelled 5, same 3); bans 0/0, stalls 1,
budgets expired 2, arrivals 229, p95 521, starving 0.
b1 booted fresh on this pair at 10:26 under the W15-c-b reader, and
its log carries "THE TRUNK'S REACH reach=2" (the binary names
itself): the first ledge-arm read of the reach-2 trunk with W16-b
aboard is that run's hour 19 (~11:00) and night (~11:20).

### W2-b-r registered (09:50): THE TRUNK PLANS THE JUMP AGAIN

Mechanism: `TRUNK_SCRAMBLE_REACH: u8 = 2` at the pump's search
config (identity with the pre-W2-b trunk), and a once-per-boot
witness line "THE TRUNK'S REACH" carrying the value so the binary
names itself in every log. Pin `the_trunk_plans_the_jump_again`
(the const is 2 and `jumps_admitted(2, land, climb, no fly)` is
true); falsifier plants 1 -> red. Bars, b1 (hour 19 / night 1):
LONGEST-EXHAUST <= 30 (617 under W2-b, 13 before it), LONGEST-TIER
lines <= 3,000 (51,798 / 1,642), arrivals >= 700, starving after
night 1 <= 1; CLIMB BANNED (fetch) is expected back at 15-24 a day
and is recorded, not barred (the known cost; the shun answers it).
b2 (+10 / day 1): exhausted for the day <= 520 (598 under W2-b, 513
on W15-c's day), and W15-c-b's own bars re-read un-confounded:
exhausted <= 256 and start_unlabelled <= 19 (a second FAIL there is
W15-c-b's own). Falsified if b1's exhausts stay >= 300 with the
trunk at reach 2 (then the flood is W15-c-b's pricing, not W2-b's
admission: revert W15-c-b next) or if fetch bans exceed 30 a day.
Chain behind W17-b (e5c4d9965d, building 09:50).

### W2-b2 registered (09:35): THE CHASER'S LEG ADMITS WHAT ITS FRAME CAN EXECUTE -- WITHDRAWN 09:44 (see above)

W2-b's early ledge read (below) named the router: the chaser's own
per-tick search (the scheduler in bastion_path.rs) re-plans each
local leg between trunk waypoints with `traversal_config_for`'s
skill reach (2, or 3 trained) and cut the corner over the two-block
ledge the reach-1 trunk had walked around; a committed walker gets
no stall assist (W6-C), so the leg's jump is never made. Mechanism:
`chaser_reach(skill_reach, committed)` = min(reach, 1) for a walker
the path_cache holds (`JobBoard::walker_committed`), the skill reach
otherwise, 0 identity; applied at the scheduler's leg (cands carry
the flag) and the detour search. Pin
`the_chasers_leg_admits_what_its_frame_can_execute`; falsifier plants
the clamp dropped. Bars: b1 fetch bans <= 3 by hour 19 (8 by hour
16 on the W2-b run), trunk-frame bans under a +2 edge 0, FETCH
STALLED <= 1.5x the W2-b run's hour-19 count with no stall cluster
>= 3 at a +2-edge head (the ledge tally), arrivals >= 90% of the
W2-b run's, starving after night 1 <= 1; b2 no regression (bans <=
3, arrivals >= 760). Falsified if trunk bans under +2 edges persist
(a third router), if stalls at the ledge heads rise by the bans
removed (the reach-1 leg exhausts and the glide walks into the wall:
a plot row, the ledge needs a ramp), or if arrivals fall > 10%.
Rejected: reach 1 for every colonist search (kills the skill where
the assist can execute it; Ben's judgement item stays open); the
assist for trunk walkers (W6-C's loop). Chain behind W17-b.

### W2-b read: b1 hour 19 (1377f60249; 09:25) -- FAILED, and a flood came back

CLIMB BANNED (fetch) 8 (bar <= 3): 7 on_prev + 1 short_of_prev, all
frame trunk, all rise_next Some(2), push_site chaser-settle, trunk_dxy
68-74 for five of them (the leg, not the trunk: W2-b2's premise);
other bans 0, PROMISED CLIMB TAKEN 2, FETCH STALLED 12, budgets
expired 15, starving 0; arrivals 733 (736 on W15-c's b1 run at hour
19: flat); exhausted 19 delivered (up 5, flat 14), stuck 2, p95 640.
LONGEST-TIER steps 40,932 against 951 and 2,434 on the W15-c run: a
flood is back, and it is a RE-ASK flood, not a wide search -- one
pair (7725,6403,181 -> 7742,6404,181, flat, 17 blocks) asked 1,675
times by hour 19, three more pairs 788-1,169 times each, two of them
with end_snap_dz=2 (the goal resolved two above the requested cell);
the memo's misses saturate at 4,096, every one why=start. W15-c-b's
b2 day (598 exhausted) ran under the same pair and is confounded by
this. Disposed: W2-b FAILED its ban bar (the leg is the router:
W2-b2), and its cost or a coincident regression is a re-ask flood
(W14 class) to be named from this log before the next row.

### W2-b read: b1 early (1377f60249, 09:20, hour 16 of day 0) -- FAILED

CLIMB BANNED (fetch) 8 by hour 16 against a bar of <= 3 by hour 19;
seven of the eight under a +2 edge one over (head minus prev), credit
on_prev, frame trunk, rise_next Some(2), at the two ledges of every
run before ((7665,6432)->182, (7637,6504)->183); one short_of_prev.
FETCH STALLED 12, other bans 0, unreachable 0. W2-b's pin is green
and its mechanism real (the pump's route holds no jump), and the bar
failed anyway: the leg the body walks is planned by the chaser's own
search at reach 2 (W16-i2's frame finding, read after W2-b was
written). Disposed: FAILED, partial mechanism; W2-b2 above.

### W15-c-b and W2-b read: b2 day 1 (1377f60249; 09:26)

Exhausted 598 for the day (513 on W15-c's day; the bar was <= 256:
FAILED), `start_unlabelled` 27 (bar <= 19: FAILED), `same` 30,
`target_unlabelled` 6, `untrusted` 4; probes 67 (46 cut_off, 19
sealed, 2 target_unwalkable); arrivals 846 (the highest day yet);
unreachable 0 all day (W17-b's class did not occur: unexercised);
CLIMB BANNED (fetch) 2 (one on_prev with rise_next Some(1) and
push_site chaser-settle = W16-b's slid-off class; one no_prev), other
4 (sleepers), FETCH STALLED 5, budgets expired 8, STALL BLAMED 1,
starving sleepers 1, p95 508. W2-b on b2: no regression (pass). The
exhaustion residual with both endpoints' plots free is a new
question (W15-i5 candidate: which cost the `same`-component searches
still pay -- the road factor, the wall band outside both plots, or
the iteration budget itself).

### W16-b landed (f408e4b8a9, staged 09:22, shipped to lab-bin 09:22)

The relaunch's check and pin green; both halves built fresh; the
falsifier planted the old vertical-only push and the pin went RED at
09:27, the tree restored clean (0 dirty). b2 restarted fresh on it at
09:27 under W2-b's b2 reader; b1 follows after W16-i2's night block.

### W15-c-b and W2-b read: b2 +10 (1377f60249 under the W16-i2 reader; 09:10, hour 15)

Exhausted 47 by +10 (30 on W15-c's +10, 40 on W2-b's, 61-87
before them), `start_unlabelled` 24 (18, 10), `same` 13 (4, 17);
unreachable 0; CLIMB BANNED (fetch) 0 and (other) 0, FETCH STALLED
0, budgets expired 3; arrivals 536 (485, 432, 425 at +10 on the
three runs before: the highest); p95 444 us; starving 0; no door
probe, no credit, no frame (nothing to name by hour 15). The
day-1 read (~09:50) carries W15-c-b's bars (the day's exhaustion,
the indoor-start class) and W2-b's on b2.

### W15-c-b landed (1377f60249, staged 08:50)

Check clean, the common pin green (1 of 714), committed 08:37,
both halves staged 08:50:38 with the client compiled fresh
against common; shipped to lab-bin 08:50. The b2 reader restarts
b2 after W2-b's day-1 block, the b1 reader after W2-b's night-1
block. Falsifier at 08:54 (its own detached worktree): the
start's plot dropped, the pin RED (0 passed, 1 failed of 714),
restored clean. W16-b's chain fired at 08:55.

### W16-i read: b1 hour 19 (the W16-i2 pair 83e3666cb6; 08:42)

CLIMB BANNED (fetch) 8 (7 on_prev with rise_next Some(2): the jump
ledge; 1 short_of_prev), CLIMB BANNED (other) 0, PROMISED CLIMB
TAKEN 5, FETCH STALLED 19, FETCH BUDGET EXPIRED 18, STALL BLAMED 3;
arrivals 727 (736, 601), stuck 5, sleepers 0; longest-tier steps
5,777 (951 and 2,434 on the W15-c runs: run-to-run). The block's
p95 sample read 2,237 us; the series over the run is 685-1,015 us
with spikes (114 samples: 9 above 1,000, 5 above 2,000, max
3,846) against the previous run's 93 samples with 6 and 5 and a
max of 2,693 -- the same shape, the sample fell on a spike; no
regression. The verdicts hold their partition (7:1) -- W2-b on
b1 (after W16-i2's night) is the read that matters.

### W16-i read: b1 night 1 (83e3666cb6; 08:59, hour 6 of day 1) -- the second replicate

CLIMB BANNED (fetch) 8 (none new since hour 19): on_prev 7,
short_of_prev 1, every push_site chaser-settle, every rise_next
Some(2); CLIMB BANNED (other) 2, PROMISED CLIMB TAKEN 6, FETCH
STALLED 20, FETCH BUDGET EXPIRED 21; arrivals 926 (933, 838, 788),
stuck 10, starving sleepers 0, steps 7,235, p95 678 us. W16-i on
b1, second replicate: PASSED as registered, the same partition as
the first (19:5 on e7ad98977a, 7:1 here): the jump ledge is the
class (W2-b), the slid-off step the residual (W16-b). The W16-i2
b1 reader took b1 at 09:00 on the W15-c-b pair 1377f60249, which
carries W2-b: b1's hour-19 read (~09:45) is W2-b's first day on
the ledge arm.

### W15-c read: b1 hour 19 (on the W16-i pair e7ad98977a; 08:01)

Longest-tier steps 951 by hour 19 (55,255 and 8,950 on the runs
before W15-c; 1,749 on its first b1 run): the flood is gone on
b1's world too. Arrivals 736 (600-601 at hour 19 on every earlier
b1 run: +22%), stuck census 1 (2-4), starving 0, p95 665 us
(637-711), pump pending 21. The stair line: CLIMB BANNED (fetch)
24, FETCH STALLED 35, FETCH BUDGET EXPIRED 33, PROMISED CLIMB
TAKEN 1 -- the two-up-edge class W16-i named and W2-b removes.
Reading: W15-c on b1 PASSED its route bar (the flood) and raised
the arrivals a player would count; the bans are the next row's.

### W15-c read: b1 night 1 (e7ad98977a; 08:19, hour 6 of day 1) -- and W16-i's b1 verdicts

Arrivals 933 (788 and 838 on the two earlier b1 night reads: +12%
to +18%); longest-tier steps 2,434 (66,001 and 10,802); stuck
census 8 (8, 11); starving sleepers 2 (bar <= 2; the replicates 3,
2, 3, 0, 0, 2, 0, 1, 0, 2); p95 687 us (710-764); pump pending 36,
oldest wait 265. CLIMB BANNED (fetch) 24 (none new since hour 21),
CLIMB BANNED (other) 3, FETCH STALLED 41, FETCH BUDGET EXPIRED 36,
PROMISED CLIMB TAKEN 2. W15-c on b1: PASSED (the flood gone, the
arrivals up, the sleepers at the bar). W16-i's verdicts on this
pair for the whole day: credit on_prev 19, short_of_prev 5, the
rest none; rise_next Some(2) 22, Some(1) 2; push_site chaser-
settle 23. W16-i PASSED as registered: every ban carries a verdict
and they partition -- on_prev with a two-up standable column ahead
(19 of 24) is the router's jump edge into a ledge (W2-b, staged
next); short_of_prev with chaser-settle (5 of 24) is the settle
displacing a body off a credited stair step (a mover row, W16-b,
after W2-b's read). The W16-i b1 reader took b1 at 08:20 for a
second replicate on the W16-i2 pair.

### W15-c landed (be2258bba6, staged 06:52)

Check clean, the common pin green (1 of 712), committed 06:43,
both halves staged 06:52:23 with the client compiled fresh
against common; shipped to lab-bin 06:53. The b2 reader restarts
b2 after W16-a's (second-run) day-1 block; the b1 reader after
W16-a's night-1 block. Falsifier at 06:56 (its own detached
worktree): the surcharge paid inside the destination, the pin RED
(0 passed, 1 failed of 712), restored clean. W17-i's chain fired
at 06:57.

## W16-a, registered 05:40 (keyed on the E2-s-i stage; the queue's end; common/)

A COLONIST DOES NOT COMPLETE A NODE FROM BELOW -- the ★★★ mover
row's first cut. Defect: both of the night's starving cases (28 on
b1, a meal five up a one-up-one-over stair; 149 on b2, its own
shelf four up) end FETCH STALLED with no displacement and CLIMB
BANNED "the route's next node was a climb the body never makes":
the route head two above the feet. The chaser's node-completed
test (common/src/path.rs) accepts `pos.z - node.z` in -1.0..=2.25,
so a body at the FOOT of a one-up stair node completes it without
rising (vanilla bodies jump; the credited step is carried by the
jump), and on a staircase the next node is then two up, which the
gliding colony body never takes. Baselines to the night-1 read:
CLIMB BANNED (fetch) 12 (b1) and 6 (b2), PROMISED CLIMB TAKEN 9
and 1, FETCH STALLED 24 and 12, FETCH BUDGET EXPIRED 15 and 13.
Mechanism: `node_z_completed(dz, in_liquid, scramble_reach)` --
floor -0.5 when `scramble_reach > 0` (colony workers only:
bastion_path.rs gives colonists 2 or 3, every vanilla NPC 0), -1.0
otherwise; ceiling and liquid clause unchanged; the chaser calls
it. No new log line. Pin (veloren-common, bastion_vertical_tests)
`a_colonist_does_not_complete_a_node_from_below` (nine asserts);
planted: the colony floor back at -1.0, red. Prediction (b1 fresh
after E2-s-i's night-1 block, b2 fresh after W15-i4's day-1 block;
`wait-w16a-b1.sh`, `wait-w16a-b2.sh`; hour 19 and night 1 / +10
and day 1): CLIMB BANNED (fetch) <= half the baseline (<= 6, <= 3);
FETCH STALLED <= baseline; starving sleepers <= 2 on b1; arrivals
within 20% of the previous run; stuck census <= 2; FETCH BUDGET
EXPIRED <= half (<= 7, <= 6). Falsified if CLIMB BANNED holds or
FETCH STALLED rises (the glide does not take a one-up either: the
mover's step is the row) or arrivals fall by more than 20% (the
stricter completion stalls walkers on slopes). Rejected: a new
TraversalConfig field (thirty-odd construction sites for one bit
`scramble_reach > 0` already carries); a two-block lift in the
mover (the PROMISED CLIMB assist exists and did not fire for 28);
a wider xy tolerance. NOT evidenced: descents; ladders; the 3-up
scramble edges. The dry tree at a1dc121908 validated the anchors.

### W16-a landed (cb7bea7543, staged 06:26)

Check clean, the common pin green (1 of 711), committed 06:22,
both halves staged 06:26:47 with the client compiled fresh against
the changed common crate (a mismatched pair would be UnexpectedEnd
at the join screen; both halves are from one commit); shipped to
lab-bin 06:26. The b2 reader restarts b2 at this stage (W15-i4's
day-1 block already read) and reads +10 and day 1 with the stair
tallies; the b1 reader restarts b1 after E2-s-i's night-1 block
and reads hour 19 and night 1. Falsifier at 06:37 (in its own
detached worktree, so the W15-c chain patching path.rs in the main
worktree at the same time was untouched): the colony floor back at
-1.0, the pin RED (0 passed, 1 failed of 711), restored clean.
W15-c's chain fired at 06:31.

### W16-a on b2: the first restart was REFUSED; the first blocks are void

The b2 reader stopped the W15-i4 server (pid 10760) at 06:29 and
the restart script refused to boot the new pair ("port still held
after stop -- refusing", a safety check that failed loudly and
correctly); the reader then read the DEAD W15-i4 log and printed
"+4", "+10" and "day 1" blocks stamped `pair a1dc121908`, `hour=12
game_day=1` -- not W16-a, not a run. Those blocks are void (a
reader's restart takes the latest staged pair: name the pair
actually run). Repair at 06:43: every b2 reader (W16-a, W15-c,
W17-i) now checks the restart's output, waits a minute and retries
once on "refusing", aborts on a second refusal, and does not read
until the NEW log's first schedule line shows game_day=0 (the b1
readers had that guard; the b2 readers did not). The W16-a b2
reader was relaunched at 06:44 writing to a second file; the
W15-c b2 reader is keyed on that file's day-1 block. The W16-a b2
read follows from the relaunch.

### W16-a read: b2 +4 and +10 (cb7bea7543, booted 06:46; read 06:51 and 06:57)

+4 (hour 10): CLIMB BANNED (fetch) 2, FETCH STALLED 2, FETCH
BUDGET EXPIRED 1, probes 30, arrivals 181, sleepers 0; ITEM 39
p95 3,370 us with spiky samples (742, 624, 2,106, 3,370, 492; the
three previous runs sat at 412-432 at +4). +10 (hour 13): p95 494
and every sample since 10:56 between 471 and 527 (458-463 on the
previous runs at +10: within noise) -- the +4 spikes were boot
transients (this boot seeded a different store, (7698,6446), and
loaded different chunks first). CLIMB BANNED (fetch) still 2,
FETCH STALLED 2, probes 64 (48 cut_off, 14 sealed), exhausted 66
(61-87 before), arrivals 425, unreachable 4 (from inside houses:
the W17-i class), sleepers 0, pump pending 36 (18-26 before; the
oldest wait 114 ticks). The same eleven minutes of W15-i4's run:
CLIMB BANNED (fetch) 1, FETCH STALLED 1. Both of this run's bans
are colonist 103 at (7671,6437,181) with the route head at
(7674,6437,183): three blocks east and two UP. Its wedge probe
(job 157, a Craft fetch, route_next_idx 55): route_prev
(7673,6437,182), head (7674,6437,183), ahead (7674,6436,183) -- a
two-step stair up onto a wall top (the blocks: solid at (7673,181)
and (7674,181..182)), one up per node, W16-a's own class; the
chaser's index had passed the first step while the body stood two
blocks WEST of it and one below, `last_push_site=chaser-settle`,
`assist_why=head_far` (the PROMISED CLIMB assist needs the head
within one block in xy). The skip rule ("the next-next node is
closer") does not fire here (the head is farther than the step,
and walls make the chase precise), and W16-a's floor refuses a
credit from one below -- so either the body stood on the step
when it was credited and was then settled back down and west
(the mover), or the index advanced by a path not yet read. The
day-1 read (~07:40) decides W16-a's bars; if the class holds, the
next instrument (W16-i) prints at every CLIMB BANNED the previous
node, the body's z against it, and the mover's rise probe at the
next column -- why the body is not where its credit says.

### W16-a read: b2 day 1 (cb7bea7543, 07:19, hour 0 of day 1)

CLIMB BANNED (fetch) 2 for the whole day (bar <= 3; 6 on the
W15-i4 run to its night read) -- PASSED; CLIMB BANNED (other) 0;
PROMISED CLIMB TAKEN 0 (1); FETCH STALLED 9 (bar <= 12) --
PASSED; FETCH BUDGET EXPIRED 8 (bar <= 6, half of 13) -- FAILED
by two; starving sleepers 0 -- PASSED; arrivals 742 (560-603 on
the two previous day-1 reads: +25%, above the band on the good
side); ITEM 39 p95 567 us (520; within 2x) -- PASSED; stuck 0.
Not registered and worse: exhausted searches 707 for the day (413
and 366 on the two previous runs; +70%), the probes 51 cut_off / 14
sealed with `same` 32 of the perimeter class as before; EMBED
WATCH 8 (5 on W15-i4's day 1), writers pure-glide 5, hold 2,
refused-rock 1; unreachable 17 (39). Reading: the stair class fell
as registered (both remaining bans are colonist 103's two-step
stair, above) and nobody starved, while the searches exhausted
more -- a body that no longer takes credit for a step it has not
climbed pushes at the step, stalls when the mover does not rise,
and asks again; W15-c (staged 06:52, on b2 from this restart)
prices the searches that flood, and its read says whether the
exhaustion comes back down. Disposition on b2: PASSED on its
primary bars, one secondary bar missed by two, and a cost
(exhaustion +70%) to be read against W15-c. b1's hour-19 and
night-1 reads follow.

### W16-a read: b1 hour 19 (07:22) -- ON THE W15-c PAIR be2258bba6, both rows aboard

The b1 restart at 07:01 took the latest staged pair, W15-c's,
which contains W16-a; this read is of both rows together. CLIMB
BANNED (fetch) 15 by hour 19 (bar <= 6 to the night; 12 to the
night on the E2-s run) -- FAILED; FETCH STALLED 40 (bar <= 24) --
FAILED; FETCH BUDGET EXPIRED 34 (bar <= 7) -- FAILED; PROMISED
CLIMB TAKEN 14 (9); STALL BLAMED 5; starving sleepers 0; arrivals
601 (600-601 at hour 19 on the two previous runs: unchanged);
stuck census 4; p95 711 us (637-710: unchanged). Longest-tier
steps 1,749 (55,255 and 8,950 at hour 19 on the two previous
runs): W15-c removed the flood on b1 -- the exhausted probes 65 by
hour 19, the last pump census delivered_path 3, exhausted 6. The
fifteen bans, from their own lines: every one head one block away
in xy and TWO up (dz 2, dxy 1: a true two-block climb, not a
stair credited from below), eight colonists (46, 20, 65 three
each; 68 two; 76, 22, 58, 61 one), and all but one at ONE place:
heads at (7657..7665, 6431..6432, 182), the edge of one building
north of the store, its floor two above the ground -- a ledge the
router's JUMP edges admit and the gliding body cannot take, and
where the PROMISED CLIMB assist (one block over, two up: its own
case) did not fire on these fifteen though it fired fourteen
times elsewhere. Reading: on b1 the W16-a bars fail, but not on
W16-a's class -- W15-c's routes now ENTER buildings that the flood
never reached, and one of them is entered over a two-block edge.
The confound is the pair; b2's W15-c read (ce3bec459e against
cb7bea7543, W16-a alone) separates the two rows, and W16-i (the
ban names the step it missed: prev, credit, rise_next, push_site)
lands on b1 after W15-c's night and names why the assist stood
down. Candidate rows: W16-b THE ASSIST TAKES THE PROMISED CLIMB
IT REFUSED (if the standable test or the committed-walker rule
refused it), or W2-b THE FETCH LEG PLANS NO JUMP (scramble_reach
0 for the fetch search, so the router takes the stair or gives
the honest "unreachable").

The wedge probes for those fifteen stalls (07:34): assist_why
"committed_walker" on thirteen, "eligible_climb" on one,
last_push_site "chaser-settle" on fourteen. The assist is refused
to a trunk-committed walker BY DESIGN (W6-C, in `assist_allowed_for`:
"the trunk is this body's mover and the assist's head is the
CHASER's -- two frames; the W6 boot measured the oscillation: step
assists 3 -> 185, embeds 1,034 -> 2,036"), so W16-b as first
written is the rejected design. The ban itself is judged on the
CHASER's route head (`route_head_is_a_climb(feet, sn.route_head)`)
while thirteen of the fifteen bodies were moving in the TRUNK's
frame (path_cache: waypoints, index, target): TWO FRAMES COMPARED
AS ONE. Whether the body stalled under the trunk's next waypoint
(a two-up waypoint the trunk router's profile should reject) or
with the trunk spent and the chaser's head the true next step, the
line cannot say.

### W16-a read: b1 night 1 (the W15-c pair be2258bba6; 07:40, hour 6 of day 1)

CLIMB BANNED (fetch) 15 (none new since hour 19), PROMISED CLIMB
TAKEN 19, FETCH STALLED 45, FETCH BUDGET EXPIRED 37, STALL BLAMED
5; starving sleepers 0; arrivals 791 (788 and 838 on the two
previous night reads: unchanged); stuck census 8 (8, 11); p95 764
us (710, 745: unchanged); longest-tier steps 2,073 (66,001 and
10,802: the flood gone under W15-c); refusals 65,536 (the memo,
the sleepers' still feet). Disposition of W16-a on b1: its three
stair bars FAILED on this pair, on a class it did not claim (the
two-up edge of one building, fourteen of fifteen, judged in the
chaser's frame while the trunk moved the body -- W16-i2 names it);
the outcomes a player sees (arrivals, stuck, sleepers) are
unchanged, and the search cost fell thirtyfold. W16-a stands as
landed on both arms: PASSED its bars where its class was the class
(b2), FAILED them where another class filled the count (b1). The
W15-c b1 reader took b1 at 07:41.

## W16-i2, registered 07:36 (keyed on the W16-i stage; the queue's end)

THE BAN NAMES ITS FRAME. Mechanism: at CLIMB BANNED (fetch) the
line gains `frame` ("trunk" when path_cache holds the walker,
else "chaser"), `trunk_next` (the trunk's waypoint at its index),
`trunk_dxy`, `trunk_dz` (against the feet), `trunk_idx` (index,
length). No behaviour changes; wiring, no new pin, no falsifier
(the chain's gate runs W16-i's pin). Prediction (b1 fresh after
W16-i's night-1 block, b2 fresh after W16-i's day-1 block;
`wait-w16i2-b1.sh`, `wait-w16i2-b2.sh`): the bans partition by
frame; for the trunk frame either trunk_dz >= 2 (the trunk router
emitted a two-up waypoint: a trunk-router row) or trunk_dz <= 1
with the chaser's head two up (the ban judged the wrong frame: a
ban-rule row -- judge a trunk walker on its trunk waypoint, and
the promised climb belongs to the trunk mover). Falsified only if
the line prints without the frame. Rejected: allowing the assist
for trunk walkers (W6-C measured it worse); fixing the ban rule
before the frames are counted. NOT evidenced: the fix the frame
names. The dry tree at e7ad98977a (W16-i) validated the anchors.

### The mover's phase rule, read at 08:35 -- why every ban ends "chaser-settle"

The chaser mover phases its move from `d` (target minus feet):
a pure glide takes `d`; else a target more than 1.2 above ->
(0, 0, dz), a VERTICAL push only; else a drop with horizontal
distance -> horizontal; else horizontal; else settle. The
vertical-only branch is a physics body's jump reflex. The
kinematic mover cannot rise in place: its surface probe (dz 0,
+1, -1, -2 at the body's own column) finds the floor it stands
on, the settle branch writes the same position, the fetch stalls
with no displacement, and the ban rule fires on the head two up.
That is one mechanism under both credit classes: the jump edge
(on_prev, the head two up and one over: W2-b removed the edge)
and the slid-off step (short_of_prev, the body two blocks short
and below a credited stair node whose next column is standable
one up: the body never walked to it because the rule pushed it
straight up). W16-b (below) changes the rule for the second class.

## W16-b, registered 08:38 (keyed on the W15-c-b stage; the queue's end)

THE MOVER WALKS TO THE STEP IT SLID OFF. Defect: above (5 of 24
bans on b1 short_of_prev with push_site chaser-settle and rise_next
Some(1) or Some(2)). Mechanism: `glide_phase(d, pure_glide)` -- a
pure glide is d; a target more than 1.2 above with no horizontal
distance (<= 0.3) is pushed vertically as before (the lift case);
a target more than 1.2 above WITH horizontal distance is walked
toward (the surface probe at the next column lifts the body by
one where a standable cell sits one up); a drop with distance, a
flat target and an xy-arrived target as before; the mover asks it
in place of the inline chain (the unused `horiz` binding goes).
Identity for pure glides and for every target within 1.2 of the
feet. Pin `the_mover_walks_to_the_step_it_slid_off` (six cases);
planted: the old vertical-only push for any higher target, red.
Prediction (b1 fresh after W15-c-b's night-1 block, b2 fresh after
W15-c-b's day-1 block; `wait-w16b-b1.sh`, `wait-w16b-b2.sh`): on
b1 short_of_prev bans 0 or 1 by the night (5) and CLIMB BANNED
(fetch) <= 3 in total with W2-b aboard (24), FETCH STALLED <= 20
(35-45), arrivals >= 700, sleepers <= 2, stuck <= 8; on b2 no
regression (bans <= 2, arrivals within 20%). Falsified if
short_of_prev holds (the surface probe refuses the one-up column:
a probe row) or a new credit class appears (above_prev: a body
pushed past its node). Rejected: lifting the body to the credited
node (a teleport by another name; the W6-C oscillation); widening
the completion window back (W16-a's class returns); treating the
settle as the fault (the symptom of a vertical push with nowhere
to go). NOT evidenced: bodies whose next column is not standable
one up either (walked toward, stalled at the foot, banned as
before); the pure-glide trunk legs. The dry tree at 1377f60249
(W15-c-b) validated the anchors.

The first chain (08:55) FAILED its check: E0425, the patch had
removed the `horiz` binding as unused and a later branch (some
two hundred lines on) still read it -- the dry tree validates
anchors, not types (a law already filed). The chain exited with
the patch in the tree; restored clean at 08:58 by `git checkout`
of the one file; the patch now keeps the binding; the dry tree
re-run; the chain relaunched at 08:59 and fires after its hold.

### The first W16-i2 frames: b1 (83e3666cb6, 08:29, hour 12 of day 0)

Four bans, every one frame="trunk", credit on_prev, rise_next
Some(2), and the trunk's next waypoint 74 to 86 blocks away in xy
(trunk_idx 0 or 1 of 2 to 6 waypoints) and two above the feet
(the road corridor's cells sit at z 182 over ground at 180). The
trunk is the coarse road corridor; the walker executes each leg
by the chaser's local route, and that local route is where the
two-up jump edge sits (the chaser's head, credited on_prev). So
the ban was judged in the frame that was actually moving the
body at that step -- the chaser's leg -- and the "trunk" flag
names only who owns the coarse plan; the assist's refusal to a
trunk walker left the jump unexecuted, and W2-b (no jump edges
for gliding bodies) removes the jump itself. Prediction disposed:
the trunk frame with trunk_dz 2 was the "trunk router emitted a
two-up waypoint" arm, but at 74-86 blocks the waypoint's height
is not the step in question; the ban-rule row is not needed
while W2-b holds. The night-1 tally on b1 completes the read.

### W16-i2 landed (83e3666cb6, staged 08:05)

Check clean, the gate pin (W16-i's) green, committed 07:56, both
halves staged 08:05:08; the binary verified by contents
(trunk_dxy present). Wiring: no falsifier. The b2 reader restarts
b2 after W16-i's day-1 block, the b1 reader after W16-i's night-1
block. W2-b's chain fires five minutes after this stage.

## W16-i, registered 07:08 (keyed on the W17-i stage; the queue's end)

THE BAN NAMES THE STEP IT MISSED. Mechanism:
`stair_credit_verdict(prev_dxy, prev_dz)` -> no_prev | above_prev
| short_of_prev (two or more blocks away in xy, not above) |
below_prev (within one block, lower) | on_prev; at CLIMB BANNED
(fetch) the line gains `prev` (the chaser's previous node),
`prev_dxy`, `prev_dz`, `credit`, `rise_next` (the first standable
dz in feet-1..feet+2 in the next column toward the head) and
`push_site` (the last mover push site). No behaviour changes. Pin
`the_ban_names_the_step_it_missed` (eight cases; colonist 103's
two west and one below is short_of_prev); planted: short and
below swapped, red. Prediction (b2 fresh after W17-i's day-1
block, b1 fresh after W15-c's night-1 block; `wait-w16i-b2.sh`,
`wait-w16i-b1.sh`): every ban carries a verdict and they
partition -- short_of_prev with push_site chaser-settle: the
mover's settle displaces bodies off credited steps (a mover row);
below_prev: a credit path other than the z-window (a chaser row);
on_prev with rise_next None: a real two-block ledge (the router's
jump). Falsified only if the line prints without a verdict.
Rejected: fixing the settle or the index before the verdicts are
counted. NOT evidenced: the fix the verdict names. The dry tree
at ce3bec459e (W17-i) validated the anchors.

### W16-i landed (e7ad98977a, staged 07:41)

Check clean, the pin green (eight cases), committed 07:31, both
halves staged 07:41:38; the binary verified by contents
(stair_credit_verdict and rise_next present). The b2 reader
restarts b2 after W17-i's day-1 block, the b1 reader after W15-c's
night-1 block. Falsifier at 07:46: short and below swapped, the
pin RED (0 passed, 1 failed), restored clean. Shipped to lab-bin
07:42. W16-i2's chain fired at 07:46. b1's restart at 07:45 (for
the W15-c read) took this pair, so W16-i's credit verdicts are in
b1's log from that boot.

## W14-i, registered 03:22 (keyed on the W12-a-c stage; the lane was idle, so it fires at once)

THE MEMO NAMES ITS NEAR MISSES. SEARCH_MEMO_WRITES counts the
memo's writes; `memo_near_miss(memo, feet, target, now)` names,
for a colonist with a memo that does not refuse it, the field
that failed -- start, target, expired -- and the fill's check
prints THE MEMO DID NOT MATCH (uid, why, stored, feet, target,
now, writes, misses) for the first thirty-two misses and the
powers of two after. No behaviour changes. Pin
`the_memo_names_its_near_misses`; planted: start and target
swapped, red. Prediction (b1 fresh after E2-p's night-1 block,
`wait-w14i-b1.sh`; hour 19 and night 1): every re-asked pair over
1,000 steps has either near-miss lines naming its walker or no
write for it (writes below the exhausted count: the search never
delivered); the misses partition by field. Falsified if the top
pair re-asks with neither. Rejected: widening the memo before the
field is named. NOT evidenced: the fix (W14-b2); b2.

## W12-a-c, registered 02:42 (keyed on the E2-p stage; the queue's end)

THE SEARCH AIMS ONE BELOW THE ON-TOP TARGET. `search_stand` returns
the target when standable, the cell BELOW when that is standable
(the on-top case), else the ring search; one branch changed, the
arrival check untouched (it tolerates the offset by the ruling).
SEARCH TARGET MOVED counts the on-top moves again (it fell 4,096
to 512 under W12-a-b; the rise back is the fix working). Pin
`the_search_aims_at_the_stand_not_the_stone` re-stated: one below
and one east -> the cell below; one below alone -> the cell below;
planted: the on-top branch returning the target, red on both.
Prediction (b2 fresh at the stage, `wait-w12ac-b2.sh`, +10 and day
1, with W15-i2's probe aboard): probed searches with target_walk
(false, true, false) at most 5% (44 of 65 on b1); exhausted
deliveries by day 1 at most 200 (572 a day under W12-a-b);
pump mean wait at midnight under 40 (50-55); arrivals by day 1 at
least 550 (641); unreachable at most 3; stuck 0. Falsified if the
on-top class stays above a fifth of the probes, or exhausted
deliveries stay above 400 with the on-top class gone (the sealed
and cut_off classes are the bulk), or arrivals fall under 500.
Rejected: a goal tolerance in the router (vanilla, beyond the
colony); moving the target at the job (the arrival check and the
witness read the on-top cell by the ruling); trusting the trunk.
NOT evidenced: the sealed and cut_off classes; b1's next pair. Dry
tree from HEAD 105f775a97 (E2-p).

### W12-a-c landed (c75a908c89, staged 03:11)

Check clean, the re-stated pin green, both halves staged 03:11.
The b2 reader restarts b2 at the stage and reads +4, +10 and day 1
with the probe's classes. Falsified: the on-top branch returning
the target planted at c75a908c89, the pin RED at 03:16, the tree
restored clean. Shipped to lab-bin 03:11. The queue is empty
behind this row. The looking sweep was booted on this pair (slot
120, ready in 30 s, the client launched on the granted path) and
could not capture the desktop (every screenshot 0x0: the session
locked or the display off, 03:13); the world and client were
stopped by pid, and the sweep stays OWED for when Ben is at the
machine.

## W14-b1, registered 02:00 (keyed on the W15-i2 stage; the queue's end)

THE MEMO KEYS ON THE JOB'S TARGET. From W14's hour-19 read: the
memo was written at the exhausted delivery with `ps.target`, which
for the trunk's approach search is node 0's centre, and read
against the job's target cell; the approach lane -- the one the
stranded walker re-asks -- was never refused (5 refusals, bar
200). Now `PendingSearch` carries `job_target`, set at all three
constructors (the approach: the job's target, not node 0; the
exact search: the stand rule's starting cell; the detour: its own
target), and the memo stores it. The rule, its pin and the check
stand. PIN: none new -- no pure function changed, and a plant of
the old key leaves the rule's pin green by construction, so no
falsifier is run and the live read is the falsifier. Prediction
(b1 fresh after W15-i2's night-1 block, `wait-w14b1-b1.sh`, W14's
reader and bars): refusals at least 200 by day 1 (5); the top
longest-tier pair at most 1,500 steps (3,791 by hour 19);
longest-tier steps by day 1 at most 30,000 (53,693 by hour 19);
the day's pump mean wait under 60 (77); the named walkers' fate
recorded. Falsified if refusals stay under 20 with the top pair
high (the key still does not match), or refusals rise past 200
with the top pair above 3,000 (the re-ask is not the fill's), or
the pump's wait does not fall. Rejected: a memo matching either
key (two keys for one search); a memo on the start cell alone.
NOT evidenced: the rescue (W14-b); b2. Dry tree from HEAD
a6fa8fe2ee (W15-i2).

### W14-b1 landed (dd2cc43cf2, staged 02:27)

Check clean, W14's pin green, both halves staged 02:27; the row
adds no string (the key is wiring), the binary carries W14's. No
falsifier by design (stated above). The b1 reader restarts b1
after W15-i2's night-1 block and reads W14's bars again; E2-p's
chain fires five minutes after this stage.

### W12-a-b-p landed (9a2925e522, staged 01:41)

Check clean, the re-stated pin green (the on-top assert now offers
a competing east cell), both halves staged 01:41. Nothing to read
live (a pin fix). Falsified: the same plant (`if standable(target)
{`, the on-top rule removed) at 9a2925e522, the pin RED at 01:45 on
"one below and one east stand: the on-top target stands, not its
east neighbour", the tree restored clean. The green of 00:20 is
closed: the pin now guards the line. Shipped to lab-bin 01:42.

## W15-i2, registered 01:32 (keyed on the W12-a-b-p stage; the queue's end)

THE EXHAUSTED SEARCH NAMES ITS TARGET. An instrument row from
W15-i1's read: a quarter of the fill searches exhaust the Longest
tier (75,000 expansions) on STORE cells at z 183 that deliveries
reach six hundred times a day, and the outcome carried nothing
about where the frontier stopped (`PathResult::Exhausted(path)`
holds the path to the closest node; the step matched it as `_`).
Now `FullPathSearch` keeps `last_closest`, and the pump's
exhausted arm prints THE EXHAUSTED SEARCH NAMES ITS TARGET for the
first sixty-four exhaustions and the powers of two after: uid,
target, start, closest node and its xy distance, the target's
walkability at, below and above, the start's, the 3x3 ring of the
target's row (# solid, . walkable, ~ neither), and
`exhaust_probe_class`: target_unwalkable (the stand rule should
have moved it), sealed (walkable, the frontier within 3 blocks),
cut_off (walkable, the frontier never near), unknown (no closest
node). No behaviour changes; common/ changes one struct and one
match arm, so both halves rebuild. Pin
`the_exhausted_search_names_its_target` (near: sealed; far:
cut_off; unwalkable; none: unknown); planted: every frontier
called sealed, red. Prediction (b1 fresh after W14's night-1
block, `wait-w15i2-b1.sh`; hour 19 and night 1): unknown 0; the
classes partition the sample and are counted, no bar; a
target_unwalkable count above 0 is a W12-a-b finding. Falsified if
unknown is common or every ring reads '~'. Rejected: a wider budget
(75,000 did not reach it); carrying the node in FullPathOutcome
(seven readers for one field the search keeps itself). NOT
evidenced: the fix the classes name; b2. Dry tree from HEAD
0b5c172d15 (W14) in the order W12-a-b-p, W15-i2.

## W14, registered 22:12 (keyed on the W15-i1 stage; the queue's end)

THE SEARCH IS NOT ASKED TWICE. Found while E2-m built, by mining
the E2-l pair's b1 log for repeated longest-tier searches: colonist
112 stood 18 wall minutes (hours 19-23 of day 0) at (7714,6344,181)
under a raised road whose tile centre lies at (7713,6344,187); its
trunk route's node 0 was that tile six blocks above its head, the
first leg went to the pump, the search escalated to the longest
tier and spent its budget, the fill consumer dropped the path
cache, and two seconds later (`path_fill_at`) the same search was
asked again: 11,849 LONGEST-TIER SEARCH steps for one (start,
target) pair, a quarter of the arm's longest-tier steps at that
point (82,331 by day 1), one of the pump's two slices per tick held
by one body while the town's searches waited 72-173 ticks (pending
20-33). The walker was never rescued: with no route it beelined at
its bed into the wall under the road and slid along it, so the
stuck census saw a moving body (named once in 15 minutes). Two more
spots on the day share the shape (7721,6353: 861 steps; 7714,6344:
735). b2, the flat lab arm, has no longest-tier line at all: the
class is terrain's. Mechanism: on a BudgetExhausted fill delivery
the pump writes `search_memo[uid] = (start cell, target cell, tick +
EXHAUSTED_MEMO_TICKS)`; before the fill enqueues (trunk approach or
exact search) `search_memo_refuses(memo, feet cell, target cell,
tick)` refuses the same cell, the same target, inside the window
(900 ticks, 30 s of sim -- a stated assumption); a moved body,
another target or an expired memo asks again; no memo, no refusal.
THE SEARCH IS NOT ASKED TWICE (uid, feet, target, refusals) names
the first eight and the powers of two -- every refusal names a
stranded walker, the instrument for the rescue row W14-b. Prior
art: RimWorld's cached unreachable verdict per pawn and destination
region with an expiry; Dwarf Fortress's "cannot reach" job flag;
Song of Syx's timed failed-search cache. Pin
`the_search_is_not_asked_twice` (the same cell and target inside
the window: refused; a moved body, another target, an expired memo,
no memo: asked); planted: the cell ignored, red. Prediction (b1
fresh after E2-i2's night-1 block, `wait-w14-b1.sh`; hour 19 of day
0 and night 1): the most-repeated longest-tier pair at most 1,500
steps (11,849); longest-tier steps by day 1 at most 30,000
(82,331); refusals at least 200 by day 1, the named walkers and
their fate recorded without a bar; the day's PUMP CENSUS mean wait
averaged under 60 ticks (72-173 in the hot window); pending at
midnight under 20 (30-34). Falsified if the top pair stays above
3,000 (another lane re-asks), or the pump's mean wait does not fall
below the E2-l pair's, or refusals stay under 20 with the top pair
high (the memo never matches under the slide: widen to the 3x3).
Rejected: rejecting a trunk route whose node 0 is more than three
blocks off the feet (W14-b's candidate with the rescue; it changes
routing for every body under an overhang and does nothing for the
exact search after it); treating an exhausted search as
unreachable (a released bed job is not a rescued body); a longer
window. NOT evidenced: the rescue itself (W14-b); b2; night 2. The
patch anchors on W15-i1's rewritten fill arm and sits last in the
dry tree (E2-m, E2-o, W13-b-r, E2-k, E2-n-i, E2-i2, W12-a-b,
W15-i1, W14).

## W13-c, written 21:07 and HELD (launches only if W13-w's read shows the embeds returning with the arrivals)

THE GLIDE NEVER SINKS BELOW THE FLOOR IT STANDS ON. The narrowest
rule for the seven-of-eight signature (the line dipping under the
body's own floor before a step-down edge): while the try cell's
column is standable at the body's current floor level, the step's z
is at least that floor; past the edge or at a riser, the line.
Nothing beyond the floor the body is on is probed. Pin
`the_glide_never_sinks_below_its_floor`; planted: the hold ignored,
red; witness THE GLIDE HOLDS ITS FLOOR. Prediction (b2 after
W13-w's day-1 block; +10 and day 1): arrivals at least 450;
pure-glide embeds at most 3 by +10 and 6 by day 1 (W12-a 8, 12);
pump mean wait under 60; GLIDE HELD 0. Falsified if arrivals fall
under 400 (the embed class goes to the route) or the embeds stay
at W12-a's order. Dry-run clean after W13-w; scripts derived; not
launched.

## W15, written 20:24 and HELD (launches only if W15-i1's night count says the exhausted searches go up)

THE TOP STEP UNDER THE SLAB. The house staircase
(`Primitive::Ramp`, inset storey, rise storey + 1) paints heights 1,
2, 3, 4, 5 by cell -- five 1-up steps ending under the floor slab of
the storey above -- and the router (common/src/path.rs) admits a
1-up only when the cell two above the ORIGIN feet is free, a jump's
clearance; the top step is refused and no route reaches the
bedroom storey. `stair_step_under_slab(dz, colonist_rules)`: a
colonist's 1-up (the scramble_reach discriminator that already
scopes the town's window and fence rules) is admitted on the
destination's headroom alone; vanilla walkers keep the jump rule;
2-ups keep their clearances; the flood's symmetric admission is
untouched. Pin `the_top_step_under_the_slab` in veloren-common;
planted: the stair rule refusing, red. Prediction (b1 fresh after
W15-i1's night-1 read; night 1): exhausted at hours 21-3 at least
60% below W15-i1's night with exhausted_up under 30%; NIGHT MEAL AT
HOME at least twice; RestAt/Traveling starving at 0-3 at most half;
EMBED WATCH by day 1 at most 6 with no upstairs entries; ROUTE FAULT
at most 4; arrivals not below. Falsified if the night's exhaustions
stay within 30% (the slab is not the refusal) or arrivals fall by a
fifth. Rejected: raising scramble reach; carving the slab; a
colonist jump. The chain, falsifier and b1 reader are derived and
dry-run (path.rs); they launch after W15-i1's read, not before.

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
