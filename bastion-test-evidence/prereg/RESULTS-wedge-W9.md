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
