# RESULTS — roofs are not routes, including the eaves (W5): the roof-stair wedge is gone; the single-store deposits bar failed while the town's totals rose

Read 2026-09-02 13:41-14:10 against PREREG-eaves.md. Arm b1 on
738cce7277 (W5 on top of P1c, W1, S8), game day 0 by 18:00 and the day-1
line; the comparison boot is the P1c pair (4ea213f029) at the same
clock; the original baseline is the P1b arm-day.

## Instrument

The founding line read interior_columns=31,104 -> interior_columns_dilated
=43,412 with eaves_margin=4; the guard census's town bounds grew by four
blocks on every side. The probe's block map at the old spot is unchanged
in shape on the boots that still probed there; on this boot nothing
probed there.

## The bars

| by 18:00 game day 0                 | P1b baseline | P1c boot | W5 boot | bar         |
|-------------------------------------|-------------:|---------:|--------:|-------------|
| probes (stalled fetches)            | 56           | 22       | 10      | --          |
| probes at the roof-stair spot       | 37           | 12       | 0       | <= 5  PASS  |
| probes at the spot with a roof head | 26 (P1b)     | 12       | 0       | 0     PASS  |
| shuns                               | 60           | 26       | 14      | <= 25 PASS  |
| haul deposits into the store at (7776-7780, 6356) | 12 | 14 | 8   | >= 13 FAIL  |
| haul deposits, all stores           | --           | 48       | 72      | --          |
| EatFrom expiries                    | 6            | 7        | 4       | --          |
| evening starving                    | 1-2          | 0-2      | 1       | --          |
| day-1 travel per claim (lane None)  | --           | 32.4     | 30.1    | +25%  PASS  |
| day-1 eat jobs / meals / shunned    | --           | 45/46/34 | 52/48/23 | --         |
| day-1 general units / heaviest cell | --           | 758/108  | 855/66  | --          |

- The roof-stair wedge is gone: zero stalls at the spot all afternoon,
  from 37 on the P1b day and 12 on the P1c day, with no climb ban needed
  there (2 bans on the whole day). The town's haul deposits rose from 48
  to 72 and the day's stalls halved.
- The single-store bar FAILED: 8 deposits into the north-east store
  against >= 13. The hauls went to the barn instead (44 in the barn's
  x-band against 22 on the P1c day; 8 in the store's band against 14).
  The haul chooser picks the nearest store by centre from the load; with
  the roof crossing priced out, the search's routes to the far store are
  longer, and the deposit-time re-aim and the store admission chose the
  barn for loads near the boundary. That is a redistribution, not a
  loss: the town stocked 855 units by day 1 against 758.
- New small wedge clusters on this boot: three fetches at (7781, 6318,
  180) against a fence column (a vault-class move the fetch leg cannot
  make: the W4 candidate), and two clusters on rooftops at z 186
  (walkers already up there by legal one-block steps inside houses).

## Disposition

PASSED on the spot, the roof heads, the shuns and the travel bars;
FAILED on the single-store deposits bar with the redistribution named
and the town totals up. The eaves rule stands. The bar was the wrong
frame (one store's intake in a town that chooses stores by distance);
the honest outcome numbers are the town's deposits, stalls and stock.
Open: the fence-vault fetch (W4), the rooftop walkers (a one-block step
onto a roof is still legal; pricing it is the next lever if the
rooftop clusters grow), and W2c's tier escalation on this same wedge
class, read next.
