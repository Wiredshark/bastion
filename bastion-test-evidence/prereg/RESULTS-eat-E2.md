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

b1 at boot +10 min (07:43, hour 15): SUPPER ROUND 0 (the supper hour
has not come), NIGHT MEAL AT HOME 0, private_units 0, starving 0,
food_stock 3,688 (24 days), panics 0. The read that matters is the
first night, on the day-1 line.

### Night 1 on b1 (07:55, the day-1 line): E2 FAILED as built

SUPPER ROUND day=0 hour=20 houses=49 heads=49 shortfall=64 loads=12
no_shelf=17 general_stacks_left=123 -- and then zero Haul arrivals
after the round, no haul yields, STORAGE SUMMARY day=1 private_units
0, NIGHT MEAL AT HOME 0, and the starving by clock: EatFrom/Traveling
2-8 an hour through 17-19, RestAt/Traveling 6 at hour 20,
RestAt/Arrived 10 at 21 and 6 at 22, three distinct starving sleepers.
The mechanism minted and nobody hauled: hour 20 is Leisure on the
default schedule (Work 8-15, Leisure 6-7 and 16-21, Sleep 22-5), the
haul gate refuses non-haulers, and the haulers were off shift, so the
twelve loads waited for the morning. The supper hour is the right
window for eating supper and the wrong one for hauling it. Beside it:
17 of 49 occupied houses have no private shelf at all (E2-c, its own
row), and 12 loads against 32 shelved houses is a quarter of an
evening.

Second replicate, b2 (W10-a-c pair be49881c8e, E2 inside, boot
07:56, day-1 line 08:20): SUPPER ROUND day=0 hour=20 houses=49
shortfall=53 loads=12 no_shelf=17; Haul arrivals after the round 0,
NIGHT MEAL AT HOME 0. The day-1 STORAGE SUMMARY reads private_units
117 there -- not the round's (no haul arrived) and not the founding
delivery's (0 private deliveries); whatever lands on shelves by other
paths on the eight-day arm fed no sleeper, since the night scan found
none (b2's night 1 had no starving sleepers to feed: EatFrom/Traveling
7 and 4 at hours 21-22, sleepers 0). Same mechanism, same failure.

Night 2 on b1 (08:38, the day-2 line): the second round at day 1
hour 20 (houses 50, shortfall 64, loads 12, no_shelf 18); 32 Haul
arrivals during day 1 after the first round, and still NIGHT MEAL AT
HOME 0 and private_units 0 at the day-2 line; the small hours read
RestAt/Arrived 21 at hour 0, 27 at 1, 29 at 2, 46 at 3, then
EatFrom/Traveling 38 at hour 4 (the sleepers get up and walk to eat).
The night-1 loads were hauled in the morning and the shelves still
hold nothing, so the hour is not the whole defect: the drop is. The
haul's deposit path is read next; E2-b's live read (Haul arrivals
after a shift-end round against private_units at the day line) names
it either way.

## E2-b, registered 08:02 (keyed on the P-zero-hours-d stage, before the binary; T1 re-keyed behind it)

`shift_end_hour(night_watch, uid, hour)`: Work now and Work does not
last two more hours (14-15 on the default schedule) is the round's
window; the cap rises 12 -> 36 (twelve haulers over two Work hours at
about a minute a load). Pin `the_supper_round_runs_at_shift_end` (14
and 15 yes; 13, 16, 20, 3 no); planted defect: the Work test
inverted, red. Prediction (b1 fresh, days 1-2): SUPPER ROUND at 14 or
15 with 20-36 loads; Haul arrivals after it above 15 before hour 16;
private_units above 60 at the day-1 line; NIGHT MEAL AT HOME above 5
on night 1; Sleep-block RestAt starving samples under a third of
tonight's 16; at most one distinct starving sleeper with a shelf.
Falsified if the loads mint and arrivals stay near 0 (the deposit
refuses the private destination), or if the shelves fill and NIGHT
MEAL AT HOME stays 0 (the night scan does not see the shelf's cell).

Where the night-1 loads went (b1's E2 log, read 08:44): after the
round, Haul arrivals by destination zone -- 68 (general) 22, 48
(general) 3, and one each to zones 2, 11, 1, 0, 13 (private, 1-cell
shelves) and 25 (general); `haul deposited` for those jobs 16 units.
The supper hauls reached the shelves and dropped; at the day-2
STORAGE CENSUS those five shelves read units=0. The food landed in
the morning (the round's loads waited for the work day) and the
household drew it during the day as an ordinary meal -- a private
shelf admits its own household at any hour -- so by night the shelf
was bare again. The deposit is not the defect; the hour is, as E2-b
says.

**Amendment to E2-b's prediction, before its read** (the binary
exists, the first night has not): "private_units above 60 at the
day-1 line" is withdrawn as a frame error -- the shelf is stocked at
15, supper is eaten from it at 20-21, and the day line reads it at 0,
so a working mechanism reads near zero there. The bars that stand:
SUPPER ROUND at 14 or 15 with 20-36 loads; Haul arrivals to private
zones after the round at least 15 before hour 16; and the OUTCOME --
Sleep-block RestAt starving samples under a third of the E2 night
(under 6 of 16) and at most one distinct starving sleeper with a
shelf. NIGHT MEAL AT HOME above 5 stays as a secondary read (it fires
only for a sleeper the scan wakes; a household that ate supper at
home before bed does not need it).

E2-b landed bcb0c2b60c at 08:40: check clean, pin green (1 passed),
staged 08:40, shipped to lab-bin 08:41. Falsified at the commit: the
Work test inverted turned the pin red (0 passed, 1 failed), tree
restored clean at 08:43. The b1
reader (`wait-e2b-b1.sh`: the round's hour and loads, Haul arrivals
after it, private_units at the day line, NIGHT MEAL AT HOME, the
starving by clock, at days 1-2) run from the stage.

### E2-b live, b1 (bcb0c2b60c, boot 08:41)

Early read at hour 15 (08:55): SUPPER ROUND day=0 hour=14 houses=49
shortfall=66 loads=33 no_shelf=16 -- the round in the Work block with
the cap no longer binding; one game hour later, Haul arrivals after
the round to private shelves 6 (general 6), units deposited on
private shelves 5, the rest of the 33 loads in flight. The window
bar holds (14, loads 20-36); the arrivals bar (15 to private zones
before hour 16) and the outcome (the first night's starving) read at
the day-1 line.

Night 1 (09:12, the day-1 line): SUPPER ROUND day=0 hour=14
houses=49 shortfall=66 loads=33 no_shelf=16; Haul arrivals after the
round 27 (the early read: 6 private, 6 general by hour 15), haul
yields 1; private_units 5 at the day line (supper eaten, as the
amended frame says); NIGHT MEAL AT HOME 0; FED starving 2 at the last
three censuses (E2: 3); EAT CENSUS meals 48, no_food_found 583 (the
E2-i counter now counts what the census could not see); panics 0.
The starving by clock:

| game hour | E2 night 1 (b1) | E2-b night 1 (b1) |
|---|---|---|
| 17-19 | EatFrom/Traveling 2, 6, 8 | EatFrom/Traveling 4, 7, 8 |
| 20 | RestAt/Traveling 6, EatFrom 2 | RestAt/Traveling 7 |
| 21 | RestAt/Arrived 10, Traveling 4, EatFrom 2 | RestAt/Traveling 8 |
| 22 | RestAt/Arrived 18 | RestAt/Traveling 8, EatFrom 5 |
| 23 | -- | RestAt/Traveling 1, EatFrom 1 |
| distinct starving sleepers | 3 | 2 |

**E2-b PASSED** its standing bars: the window (14), the loads
(33), and the outcome -- starving samples IN BED (RestAt/Arrived)
16 -> 0 across the Sleep block; the sleepers who reach bed have eaten
at home. What remains: sixteen of forty-nine occupied houses have no
private shelf (E2-c, a shelf in every house), and seven or eight
colonists an hour walk to bed hungry at 20-22 -- those are the
shelfless houses' people and the day-trip class, not the round's.
NIGHT MEAL AT HOME 0 is the secondary read and reads as designed: the
night lane fires for a sleeper the scan wakes, and supper eaten
before bed leaves it nothing to do.

## E2-c, registered 09:20 (keyed on the W9-i2 stage, the end of the chain, before the binary)

A house with adopted beds and no container gets a one-cell private
shelf at `shelf_cell_beside(first bed, footprint, standable)` -- the
first standable floor cell beside the bed, orthogonal neighbours
first, inside the footprint -- registered as a container's zone is;
witness SHELF ADDED. The round collects the short houses and serves
them emptiest first, so a house a capped round misses one evening is
first the next. Pin `every_house_gets_a_shelf_beside_its_bed` (a
wall east puts it west; an edge bed takes the inside neighbour;
nothing standable means none); the E2 pin must stay green in the
chain. Planted defect: the footprint ignored, red.

Prediction (b1 fresh, days 1-2): SHELF ADDED 12-20 at adoption;
SUPPER ROUND no_shelf 0 (E2-b: 16), shortfall near 98, loads 36 on
day 0 and the day-1 round serving the houses day 0 missed; in-bed
starving 0-1 across the Sleep block; the hungry walk to bed
(RestAt/Traveling at 20-22) under 4 an hour (E2-b: 7-8); distinct
starving sleepers at most 1. Falsified if SHELF ADDED is 0, or if
no_shelf reads 0 and the hungry walk to bed holds at 7-8 (the day trip
is the walker, not the shelfless house).

Night 2 and day 2 on b1 (09:59, the day-2 line): **E2-b FAILED at
the town's scale.** The rounds ran on time (day 0 hour 14 loads 33,
day 1 hour 14 loads 32, no_shelf 16-17) and 102 hauls followed the
first, yet the second night read RestAt/Arrived 10, 21, 28 at hours
1-3 (ten distinct starving sleepers) and the day read EatFrom/Traveling
7-29 an hour from dawn to dusk, no_food_found 8,339, meals 109. The
food frame: food_anywhere 3,821 at the day-1 line -> 1,251 at day 2,
food_stock 3,799 -> 1,228 (7.7 days), food_locked 6, and the
discriminator's in_bags 111 -> 1,693. Two thousand five hundred units
left the ground in a day and sixteen hundred of them are in bags.
The round's "smallest covering stack, else the largest" served
two-unit needs with stacks of hundreds once the small stacks were
gone; the haul reserves and picks up the whole entity (#89) and the
shelf drop landed 1-3 units, so the hauler kept the rest. A guard
that spends before it refuses: the row that follows (E2-d) never
hauls a stack larger than the need allows and returns stranded cargo
to the general store. E2-c (a shelf in every house) is building on
top of this and is read with that caveat.

## E2-d, registered 10:10 (keyed on the E2-c stage, before the binary; W10-e re-keyed behind it)

The bag series by game hour on E2-b's b1: 90 at 16, 315 at 21, 234 at
4, then 2,327 at 7 of day 1 -- the dawn claim of the loads the
hour-14 round left unclaimed at dusk, each a whole stack. Three
changes: `supper_stack_pick(amounts, need)` takes the smallest stack
that covers the need and is no larger than SUPPER_STACK_MAX = 8, else
the largest under the cap (a partial supper), else nothing (the house
stays short, counted `skipped_no_small`); at the start of the Sleep
block the unclaimed private-destination hauls are removed with their
reservations (SUPPER HAULS SWEPT, `stale_removed`); BAG CENSUS names
the top three holders whenever bags exceed 300 units. Pin
`a_supper_stack_is_never_bigger_than_the_supper` (two among [800, 3,
2, 50] takes the 2; five takes the 3; [800] alone takes nothing);
planted defect: the cap removed, red.

Prediction (b1 fresh, days 1-2): in_bags under 400 at every
discriminator line (E2-b: 2,327); food_anywhere at day 2 within 500 of
day 1 less the town's eating; loads 12-36 with skipped_no_small
counted; stale_removed counted on at least one night; night-2 in-bed
starving under 6 an hour (E2-b: 10-28) and distinct starving sleepers
at most 3 (E2-b: 10); no_food_found at day 2 under 1,000 (8,339).
Falsified if in_bags climbs past 1,000 while stale_removed counts (the
BAG CENSUS names the strander), or if loads fall under 12 for want of
small stacks (the kitchen's output is the row).

E2-c landed 26fbfc3d08 at 10:19: check clean, both pins green (its
own and E2's), staged 10:19, shipped to lab-bin 10:20. Falsified at
the commit: the footprint ignored turned the pin red (0 passed, 1
failed), tree restored clean at 10:23. Early on b1 (boot 10:21): the
day-0 STORAGE SUMMARY reads private_zones 88 against 69 -- nineteen
shelves added at adoption, inside the predicted 12-20. The b1 reader (`wait-e2c-b1.sh`: SHELF ADDED, the round's
no_shelf, the starving by clock, days 1-2) run from the stage; read
with the E2-b caveat (the stack strand E2-d corrects lands one pair
later).

E2-d landed 3765c74e87 at 10:43: check clean, both pins green (its
own and E2's), staged 10:43, shipped to lab-bin 10:43. Falsified at
the commit: the cap removed turned the pin red (0 passed, 1 failed),
tree restored clean at 10:46. The b1 reader (`wait-e2d-b1.sh`: the bag series, BAG CENSUS, the rounds'
skips and sweeps, the food frame, the starving by clock, days 1-2)
run from the stage; the b1 restart ends E2-c's own reader before its
day-1 line, so E2-c's shelves are read on this pair (SHELF ADDED, the
round's no_shelf).

Prediction for the row that follows, registered now: after it, the
Sleep-block STARVING samples fall below 10% of tonight's (281 -> under
28 RestAt/Arrived over a comparable window) and FED at the day
boundary reads at least 46/50 (baseline 36/50). Falsified if the
sleepers are fed and the day-boundary FED does not move -- then E2-b
(the day trip) is the larger half and the next row is a route row.
