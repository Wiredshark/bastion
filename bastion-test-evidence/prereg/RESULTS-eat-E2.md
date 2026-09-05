# E2 -- starving beside a full larder (2026-09-05)

## The finding (b1, E1-f pair b2f3f48d3f, days 0-2)

The day-2 line on the 160-day arm: food_stock 3,877, 24.7 days of
food, SETTLER GATE OPEN, roster 49 -> 50 -- and FED 36/50 with
starving 5 at the day boundary. E1-i's STARVING COLONISTS witness
(id:hunger:job:state, every 300 ticks) over 214 census lines, the
starving colonist-samples by game hour (tick 0 is hour 7):

| game hours | who is starving (samples) |
|---|---|
| 7-20 (the work day) | EatFrom/Traveling 10-42 an hour: on the way to a meal at hunger 0.00 |
| 21-3 (the Sleep block) | RestAt/Arrived 22-84, RestAt/Traveling 14-24, RestAt/Waiting 5-16: in bed, walking to bed, waiting for a bed, all at hunger 0.00 |
| 4-6 (dawn) | EatFrom/Traveling 43-84: the sleepers get up and go to eat |

Totals: EatFrom/Traveling 484, RestAt/Arrived 281, RestAt/Traveling
161, RestAt/Waiting 52, Recreate/Traveling 14; 18 distinct sleeper
uids. Two defects, not one:

- **E2-a, the night.** A colonist goes to bed at hunger 0.00 and stays
  there until dawn. The scan's own comment says a starving
  sleeper-to-be eats first (the RestAt skip applies only when eat is
  not the most urgent candidate), and the night branch admits only
  piles inside the sleeper's own house. Which skip actually hits the
  sleepers the census cannot say (it aggregates per pass, any need;
  no_food_found 0, curfew 0, already_on_need_job 47,717,
  drive_not_personal 15,418, preempt_cooldown 9,156).
- **E2-b, the day.** Colonists at hunger 0.00 spend hours travelling
  to food. E1-i cut the trip's stall (90 s -> 30 s at the item's
  cell); the W-rows cut the route detour; what is left is the route
  itself (detour ratio ~2.1 on b2) and the store's distance.

## The instrument, registered 04:50 (before the read)

b3 (test 8's idle server stopped through the shutdown file) reboots a
fresh flat town on the newest staged pair with `BASTION_NEED_SKIP_DIAG`
on, runs through the first night, and `diag-e2-agg.py` joins the
NEED-SKIP-DIAG reasons to the STARVING sleepers in the same 300-tick
window. The read decides the mechanism:

- `already_on_need_job` dominates for RestAt holders -> the urgency
  sort puts rest first for a sleeper (band or candidacy), and the row
  is the sort: a Dire hunger outranks any rest.
- `drive_not_personal` dominates -> the arbiter's hysteresis holds a
  sleeper out of Personal; the row is the arbiter's night rule.
- `preempt_cooldown_active` dominates -> the Dire cooldown (5 s) is
  re-armed by something that never completes; the row is the seam.
- no reason at all hits them (the scan never reaches the sleepers) ->
  a gate before the skip census (asleep, curfew, missing component);
  the row is that gate.
- `no_food_found` -> the house shelf is empty at night; the row is the
  night larder (a supper carried home, or the store admitted at night).

## The diag read (b3, W9-c pair, 05:17: boot to the first midnight)

122,661 NEED-SKIP-DIAG lines joined to 36 STARVING census lines (the
Sleep block ran hours 21-23 only; 3 distinct starving sleepers, 12
starving eaters at hour 19).

| who was starving | skip reasons that hit them in the same window |
|---|---|
| sleepers (RestAt: Waiting, Arrived, Traveling) | no_food_found 808, preempt_cooldown_active 30, already_on_need_job 21 |
| eaters en route (EatFrom/Traveling) | already_on_need_job 744, preempt_cooldown_active 187 |
| loungers (Recreate) | already_on_need_job 13, preempt_cooldown_active 6 |

The shelf branch it is: the scan reaches the sleepers, hunger wins the
sort (already_on_need_job is 2% of their hits), and the night pick
finds nothing inside the house. The EAT CENSUS on the same log printed
`no_food_found=0` -- its counter for that reason is dead (the diag
line fires 808 times where the census counts none); a second
instrument defect, noted for its own small row. The E2 chain stays
armed as written.

## The row, written ahead of the diag (E2, 05:05; keyed after P-zero-hours-b)

The shelf branch is the one the evidence already carries: b2's day-1
STORAGE SUMMARY reads private shelves 69 zones / 112 units against 4
general stores / 11,987, and b1 logged NIGHT MEAL AT HOME 0 in two
days -- the haul generator refuses a private shelf as a destination
whenever a general store exists (`store_admits`), so the shelf the
night scan is confined to is empty by construction. THE SUPPER COMES
HOME: once a day in the supper hour the haul lane mints, for every
house with an owned bed and a private shelf, the shortfall against
heads x 2 units as Haul jobs from unreserved general-store food
stacks to that shelf (smallest covering stack first, 12 loads a round
at most). Pin `the_supper_round_fills_the_shortfall_once_a_day`;
planted defect: the round runs every pass, red. If the diag names a
different branch (the sort, the arbiter, the cooldown, a gate before
the census) the chain is killed by its pid file before it fires and
the row is rewritten; the shelf half stays true either way.

## E2 landed (3ce9276f42, 07:32)

Check clean, pin `the_supper_round_fills_the_shortfall_once_a_day`
green (1 passed), staged 07:32, shipped to lab-bin 07:32, with the
dead census counter (E2-i) in the same commit. Falsified at the
commit: the round running every pass turned the pin red (0 passed, 1
failed), tree restored clean at 07:35. The live read is b1 fresh on
the pair
(`wait-e2-b1.sh`: SUPPER ROUND, NIGHT MEAL AT HOME, STORAGE SUMMARY
private_units, FED at the day boundaries, the starving by clock, at
+10 min and days 1-3), with the P-zero-hours-c draft read beside it
(`wait-pzc-b1.sh`).

Prediction for the row that follows, registered now: after it, the
Sleep-block STARVING samples fall below 10% of tonight's (281 -> under
28 RestAt/Arrived over a comparable window) and FED at the day
boundary reads at least 46/50 (baseline 36/50). Falsified if the
sleepers are fed and the day-boundary FED does not move -- then E2-b
(the day trip) is the larger half and the next row is a route row.
