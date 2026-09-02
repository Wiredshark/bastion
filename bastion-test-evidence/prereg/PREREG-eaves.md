# PREREG — roofs are not routes, including the eaves (W5)

Registered 2026-09-02 12:45, before the binary exists.

## What three probe cuts established

The main wedge spot, feet (7748, 6328, 181): every stalled fetch there
carried a partial route whose next node was (7748, 6327, 183), the first
landing of a stair that climbs the adopted house at (7738-7755,
6306-6323) and crosses its roof toward the general store 30-40 blocks
north-east. The route is built from one-block steps (dz 1), which the
climb ban never covers; the mover advances past the stair's first step
by node tolerance without gaining the height and then faces a two-block
rise. Dropping the route and re-searching found the same stair fourteen
times for one job (W2b, failed).

The engine already has the rule "ROOFS ARE NOT ROUTES" (Ben: climbing is
a last resort, never a shortcut): a jump or scramble may never land on a
building column, and every walk edge into a building column pays
INTERIOR_SURCHARGE (2.0, additive). Both read `interior_cells`, which the
founding builds from the site's TILE footprints. The stair and roof edge
at y 6326-6327 lie four blocks outside the house's tile (max y 6323):
Veloren's houses overhang their tiles. Neither rule saw them.

## Mechanism

At founding, the interior set is dilated by EAVES_MARGIN (4) columns in
every direction, excluding road columns (the surcharge would otherwise
price the street beside every house). Pure `dilate_columns` pinned:
a one-cell set dilated by 1 is 9 cells; a road cell inside the margin is
not added; margin 0 is the identity; the planted defect (not excluding
roads) puts a road column in the set and the pin goes red.
`BASTION_NO_EAVES` sets the margin to 0. The founding line gains
`interior_columns_dilated`.

With the eaves priced, the house crossing (~20 cells x 2.0) costs more
than the road detour (~60 blocks x 0.5) and a complete road route beats
the roof at any tier; the landing rule also refuses jumps onto the eaves.

## Baseline (P1b arm-day, by 18:00 game day 0)

probes 56, at the spot 37, shuns 60, haul deposits into the store 12,
EatFrom expiries 6; W2 day: 14 / 3 / 21 / 13 / 1.

## Pre-registered outcomes (arm b1's day 0 on the W5 pair, read at 18:00)

- Instrument validation first: the founding line must show
  interior_columns_dilated > interior_columns, and the probe's block map
  at the spot is unchanged (the terrain is the same; only the pricing
  moved).
- PASS: probes whose route_head is a building column at the spot (z >=
  183) -> 0; probes at the spot <= 5; haul deposits into the store >= 13
  (the W2 day); shuns <= 25; mean_travel_blocks_per_claim on the day-1
  lane lines within +25% of the W2 day's (the detour is priced, not
  banned: colonists should not be walking round the world).
- FAIL branches: the spot's stalls persist with Exhausted at Longest ->
  no road route exists from that side and W1's withdrawal must cover
  (Longest, Exhausted); stalls move to the barn's west wall or the
  terrace wall with Path/Exhausted at low tiers -> the mover's skipped
  step (W4) is the row; travel per claim rises > 25% -> the margin is too
  wide and prices streets (then exclude wall-margin columns too, or
  shrink the margin to 2).
- NOT evidenced live yet.
