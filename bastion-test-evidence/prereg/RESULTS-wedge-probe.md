# RESULTS — the wedge probe (P1, P1b): PASSED, and it named two mechanisms

Read 2026-09-02 10:17-10:56 against PREREG-wedge-probe.md. Arm b1 on
21adf2f5df (P1: the block map) and on 96022099c9 (P1b: the block map plus
the walker's route), game day 0 of each boot, raids on.

## The instrument's bar

- PASS required >= 20 probes at the spot (7744-7751, 6328-6335) with one
  block pattern shared by >= 80%. P1 read 11 of 11 by 15:00 and P1b 26 of
  26 by 15:00, both 100% one pattern.
- The first cut's `steer` field printed None on every probe: it read
  `fetch_steer` before the fetch assigned it. A field that cannot vary is
  not evidence; P1b replaced it with the chaser's diagnostic snapshot.

## What the probe showed

The spot. Feet (7748, 6328, 181) on flat ground, every cell north of the
feet air from floor to head+1, and directly south a stair rising eastward
(surface 182, 183, 184+ across three columns): the roof edge of the
adopted house at (7738-7755, 6306-6323), four blocks beyond that house's
tile footprint. Targets: the general store at (7776-7780, 6356, 182),
30-40 blocks north-east. All Designated hauls (P1) or hauls and two
eaters (P1b).

The route (P1b, 26 of 26): `route_target` the item; `route_complete`
false; `path_state` Exhausted; `route_head` (7748, 6327, 183);
`route_ahead` (7749, 6327, 184). The search ran out of its budget, kept
the partial path, and that path's next node is a two-block climb onto the
roof edge. The kinematic mover takes one-block steps; the step/vault
assist and the climb-ban recorder are both gated by
`fetch_steer.is_none()`, so on a fetch leg nothing rescues the body and
nothing prices the climb out of the next search. The design's own remedy
for job legs (Ben: "infinite climb loops... try a secondary route")
never ran for fetches. That is row W2 (PREREG-fetch-climb-ban.md).

The other spots (P1, 63 probes by day 1 08:00; P1b, 42 by 15:00):

| feet                | probes | block map                                   | route (P1b)                              |
|---------------------|-------:|---------------------------------------------|------------------------------------------|
| (7748, 6328, 181)   | 11 / 26 | roof-stair to the south                    | Exhausted, head two up onto the roof     |
| (7660-7666, 6433, 180) | -- / 12 | two-high wall to the south              | None, EMPTY route (no node at all)       |
| (7637, 6353, 181)   | 13 / -- | two-high wall to the east (the barn's west wall) | not yet read with the route        |
| (7627, 6323, 186)   | 9 / --  | on a roof, fence posts                     | not yet read                             |
| (7689, 6337, 181)   | -- / 3  | wall to the north, roof edge               | Exhausted, target 90 blocks              |

The second class: 12 fetches aimed at store zone 45 (x 7672-7725, y
6426-6467), a raised terrace behind a two-high wall; the item lay six
blocks beyond the wall and two up. The search's verdict was "no path";
the choosers (the reach gate is off by default, and its labelled box of
radius 128 does not reach the store) kept sending walkers. That is row W1
(PREREG-unreachable-store.md).

## Disposition

PASS on the probe's own bar in two cuts. The probe stays in the binary
(once per job, a few dozen lines a day). Named for the fix rows:
- W2: a wedged fetch bans the climb it could not make (Exhausted, head
  two up);
- W1: a store the search cannot reach is withdrawn on the search's word
  (None, empty route);
- W3 (not registered): the search budget for far fetches, if W2's
  re-plans still exhaust on the ground.
NOT read yet with the route: the barn's west-wall spot and the rooftop
eaters (P1 only); the W2 arm-day reads them.
