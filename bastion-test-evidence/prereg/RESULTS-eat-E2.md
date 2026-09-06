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

### E2-c and E2-d live, b1 (E2-d pair 3765c74e87, boot 10:45)

Early read at the first round (10:54, hour 14): SHELF ADDED 19;
SUPPER ROUND day=0 hour=14 houses=49 shortfall=98 loads=36 no_shelf=0
skipped_no_small=0 stale_removed=0 -- **E2-c's shelf bar PASSED**
(no_shelf 16 -> 0; the shortfall is exactly 49 heads x 2); the cap
binds at 36 of a 98-unit shortfall, so the emptiest-first order does
the rest tomorrow. The bag series by game hour: 451 at boot (the
founders' own inventories, which the BAG CENSUS named on two lines),
then 86, 78, 80, 77, 71, 73, 69, 63 through hours 7-13 (E2-b: 90 at
16 and climbing by 21); 9 Haul arrivals in the round's first three
minutes. The dawn line (E2-b's 2,327) and the night's starving are
the day-1 read.

Day 1 on b1 (11:08, the day-1 line): SUPPER ROUND day=0 hour=14
houses=49 shortfall=98 loads=36 no_shelf=0 skipped_no_small=0;
SUPPER HAULS SWEPT day=0 swept=35; in_bags by game hour 451 (boot),
80, 73, 56, 40, 61, 48 through hours 9-21 (E2-b: 315 by 21 and 2,327
at dawn); BAG CENSUS 3 lines, the last in_bags=393 with one holder at
366 (a stack in transit); YEAR day=1 food_stock 4,067, food_anywhere
4,080, 25.9 days (E2-b: 3,799 / 3,821 / 24.2, then 1,228 at day 2);
NIGHT MEAL AT HOME 1 (the first ever logged); starving by clock
EatFrom/Traveling 2-8 an hour through 17-22 and RestAt/Arrived 3 at
hour 22 only; distinct starving sleepers 2; no_food_found 777 (E2-b
day 1: 583, day 2: 8,339); panics 0. **E2-d PASSED its day-1 bars**
(bags under 400 at every line, the sweep counted, in-bed starving
under 6 an hour, sleepers at most 3, no_food_found under 1,000); the
food frame holds where E2-b's slid.

CORRECTION (11:30, the same log at day 1 hour 8): that read was taken
at clock hour=0 game_day=1, where the YEAR CENSUS day line fires,
which is BEFORE night 1 (the Sleep block runs 22-5). The night bars
were judged on the evening alone. With the night in: starving by
clock RestAt/Arrived 14, 16, 14, 24 samples at hours 0-3 (the census
samples every 300 ticks, 7.5 an hour: two to three sleepers starving
at any sample), EatFrom/Traveling 42 at hour 4 (the hungry rise
before the block ends) and 18, 8, 14, 13 through 5-8; distinct
starving sleepers 5. Against the night bars E2-d FAILS (under 6 an
hour, sleepers at most 3); against E2-c's night 1 (10-28 an hour, ten
distinct) the sleepers halve and the samples do not. The bags, the
sweep and the food frame stand as read. E2-e's baseline for the night
is this, not the evening's 3 and 2. Every reader that keys a night on
the YEAR CENSUS day line reads the evening; `wait-e2e-b1.sh` now
reads day N at hour 6 of day N, and `read-e2d-night2.sh` reads E2-d's
night 2 the same way. **And the sweep names the next
defect**: 35 of 36 supper loads were never claimed before dusk --
arrivals after the round 9, of which 2 to private shelves -- so
supper reached two houses. The haul lane's day: 8 haulers, 23 hauls,
20 works. The claim scoring is read next (E2-e).

### E2-d day 2 (the evening frame: day 1 whole and night 1), read 11:48

SUPPER ROUND day=1 hour=14 houses=50 shortfall=100 loads=36
stale_removed=35; SWEPT day=2 swept=35; NIGHT MEAL AT HOME 2 in two
days; no_food_found 6,317 on the day-2 census (day 1: 777; the wake
rush at hour 4 is 42 starving samples walking to eat); distinct
starving sleepers 6; meals 111. **The bag bar FAILS on day 1**:
in_bags 686 at d1h07, then 400, 636, 548, 428, 376, 471 through the
day against 27-80 on day 0, and in_stockpile fell 4,100 -> 3,400-3,600
with it. BAG CENSUS 122 lines. The holders are not the supper round:
uid 68 (Cook, 28 Cook fetches) held 520 units from tick 51,000 to
54,300 and beyond (twelve censuses); uid 52 (Guard, no fetches) 1,444
once; uid 34 (Build, 9 material fetches) 945 twice; uid 69 (a Haul
with 16 Cook fetches) 364; uid 46 (Guard) 366; uid 37 (Chop) 360; uid
80 (Haul, 14 Haul fetches) 318. Every pickup takes the whole item
entity (#89: the reservation is u32::MAX; the site `inv.push(item)`)
-- a cook fetching four mushrooms carries five hundred, a guard
eating one unit carries a thousand, a builder fetching stones carries
the pile -- and while it walks, the larder is in a bag: the E2-b
collapse class through every lane, not the supper's. E2-d's stack
cap covered the supper loads only. Disposition: E2-d stands for the
supper round (sweep, cap, shelves) and its day-1 bag bar was met by
day 0 alone; the whole-stack pickup is its own row (E2-f, "A PICKUP
TAKES THE NEED, NOT THE STACK": split the item at the pickup to the
job's need -- the recipe amount, a meal, a load -- and leave the rest
where it lay), registered below once the pickup site is read.

### E2-d after night 2 (read at day 2 hour 6, 11:56; `read-e2d-night2.sh`)

Cumulative through two nights: RestAt/Arrived starving samples at
hours 0-3 = 18, 25, 33, 52 (night 2 alone: 4, 9, 19, 28, worse than
night 1 at hours 2-3); RestAt/Traveling 30 at hour 3 (walking to bed
hungry); EatFrom/Traveling 51 at hour 4 (the rise before the block
ends); distinct starving sleepers 13 (night 2 added seven). NIGHT MEAL
AT HOME 2 in two days. YEAR day=2 food_stock 3,525 / food_anywhere
3,546 / 22.5 days (day 1: 4,067 / 4,080 / 25.9): down 540 in a day
against ~111-160 eaten, with 465-470 in bags at every line from d1h21
to d1h02. **E2-d FAILS both nights' bars** (under 6 an hour, sleepers
at most 3); the supper round as built does not feed the night, and
the larder rides in bags. Two rows carry it: E2-e (the loads claimed
first, the window wider) and E2-f (a pickup takes one load).

## E2-f, registered 12:00 (keyed on the W11 stage, the end of the chain, before the binary)

Every pickup takes the whole ground entity (`emit_pickup` at the haul
site and the material-fetch site; the eat site already splits one).
`PickupItem::split_off_n(n)` (common): from the last entry when it
holds more than n (`take_amount`), else the last entry whole when
there is another (the invariant that every entry but the last is at
max holds), else None. Server: `PICKUP_LOAD_UNITS = HAUL_CHAIN_MAX_LOAD`
(16, row 20's load, the same unit the chain and the deposit cell use);
`pickup_take(total, aboard, load)` -> Nothing when no room, Whole when
the ground holds no more than the room, Split(room) otherwise. The
haul's pickup block is entered only while less than a load is aboard;
a Split pushes the items into the bag (a push that fails drops them
where the colonist stands) and the deposit leg begins next tick on
what is aboard; the fetch's Split releases its reservation and the
claim re-checks its materials carried. Witness A LOAD, NOT THE STACK
(uid, job, def, took, left) at the first eight and powers of two.
Pin `a_pickup_takes_a_load_not_the_stack`; planted: the Whole test
inverted (a stack of 520 taken whole), red. Prediction (b2 fresh,
`wait-e2f-b2.sh`, day 1 in the evening frame): in_bags under 120 at
every line after the boot line (E2-d day 1: 400-686), BAG CENSUS
lines under 10 (122), no holder above 64 in any census; in_stockpile
at day 1 within 250 of the boot's (E2-d: 4,100 -> 3,400); the Haul
lane's day-1 hauls at least 15 (23) and cook meals at least 80 (111);
A LOAD, NOT THE STACK fires (a null with no fires means the sites are
not the producers). Falsified if hauls halve (the load is too small
for the lane's day) or the bags stay above 300 with the witness
firing (a third producer: the harvest basket or the deposit run).
Rejected: a bigger load (the chain, the backlog and the deposit cell
already agree on 16); splitting at the store's drop instead (the walk
is where the larder is invisible).

### E2-f landed (5175194ed1, staged 14:20)

Check clean (common and bastion-server), the pin green, both halves
built fresh (the common change rebuilds both) and staged 14:20. The
falsifier (the Whole test inverted) and the b2 reader
(`wait-e2f-b2.sh`: after W10-i1's day-1 read, a fresh b2; the bags,
the holders, the food frame, the hauls and the meals at +10 and day
1) run from the stage; the E2-g chain keys on it. The falsifier (the
Whole test inverted) went red and restored clean at 14:25; lab-bin
carries the pair from 14:21.

b2 +10 min (hour 13 of day 0, boot 14:22, read 14:32): A LOAD, NOT
THE STACK 9 witness lines, splits=16 (took=16 left=16, 16/11, 16/8,
16/14, 16/16, 1/3); in_bags 451 at the boot line (the founding
loadout, as every pair), then 103, 113, 130, 106 through hours 8-11
with in_stockpile 3,917-3,941; BAG CENSUS 2 lines; the largest
holder 32 units (uid 120: two loads); "job claimed" 370, arrivals
499, working=6 moving=40; panics 0. The morning matches E2-d's
(27-80 at hours 9-16 on b1); the bar is day 1, where E2-d's b1 held
400-686 all day, read at the day line (~15:05).

Day 1 in the evening frame (hour 0 of day 1, read 14:52): A LOAD,
NOT THE STACK splits=32; in_bags 103, 113, 130, 106, 97, 140, 89,
81, 36, 45, 27, 24 through hours 8-21 (one line, hour 14's 140, over
the bar of 120; E2-d's b1 day 0: 27-80); in_stockpile 3,841-3,984;
BAG CENSUS 2 lines; the largest holder 32; YEAR day=1 food_stock
3,969 / 25.3 days; meals 44, no_food_found 598; the town's day line
works 336, hauls 106 (its W10-g day: 345 / 73; its E2-e day: 248 /
131) -- the works are back to the W10-g level with the loads split;
"job claimed" 443, arrivals 790, working=33; panics 0. This frame
is the FIRST day (hours 7-23 of day 0); E2-d's 400-686 came on the
SECOND day, when the store's small stacks are gone and every fetch
is from a stack of hundreds. The comparable read is b2's day-1
series at the day-2 line (`read-e2f-day2.sh`, ~15:35); the W12-i1
reader is re-keyed behind it.

The second day (day 1, hours 7-23; read at the day-2 line, 15:33):
A LOAD, NOT THE STACK splits=64; in_bags 222, 221, 259, 236, 192,
201, 198, 153, 126, 107, 97, 69, 67 through hours 7-21 (max 274) with
in_stockpile 3,828 -> 4,074 (the harvest lands and stays); BAG CENSUS
lines on day 1: 0 (E2-d's day 1: 122); the largest holder over the
whole run 32 units (E2-d: 520, 945, 1,444); YEAR day=2 food_stock
4,074 / 25.5 days (E2-d day 2: 3,525; E2-e: 3,610); the day-1 lane
line works 336 hauls 106; meals 117; no_food_found 8,221 on the day-2
census (this pair has E2-e's round but not E2-h's stack fallback nor
E2-g's carrying: the night rush stands); panics 0. Against the bars:
no holder above 64 MET; census lines under 10 MET; the food frame
within 250 MET (up, not down); hauls at least 15 and meals at least
80 MET; in_bags under 120 at every line NOT met as written (200-260
through the morning) -- the producer is fifteen carriers with loads
of sixteen in transit, the bounded carry the row was built to make,
and the bar assumed fewer carriers than a town with 106 hauls a day
has. **E2-f PASSED on its mechanism** (the larder no longer rides in
bags: 0 census lines, 32 the largest holder, the stockpile up); the
aggregate bar is re-stated for the next read as "no holder above 64
and no census line", which is what it measured.

## E2-e, registered 11:15 (keyed on the T1-b stage, the end of the chain, before the binary)

The claim scoring gave a supper load the Haul base priority, then a
clump penalty of 12 (every supper pickup is at the same store cells)
and the saturation penalty, so it scored below a field haul across
town; and two Work hours were not enough for what did get claimed.
`errand_priority(base, is_errand)` lifts a supper load to the
player-order priority (5) and the clump penalty is skipped for it;
the round records its loads in `board.supper_jobs` (cleaned on
remove_job); `shift_end_hour` becomes the last four Work hours
(12-15), and the E2-b pin says so. Pin
`a_supper_load_is_the_haulers_first_claim`; planted defect: the lift
removed, red. Prediction (b1 fresh, day 1): the round at hour 12; Haul
arrivals to private shelves before hour 16 at least 25 of 36 (E2-d:
2); SUPPER HAULS SWEPT under 10 (35); NIGHT MEAL AT HOME at least 5
(1); in-bed starving at most 3 an hour and distinct starving sleepers
at most 1 (E2-d night 1, read after the night: 14-24 an hour, 5
distinct; the "2" first written here was the evening's count, see the
correction above); bags under 400 at every line and the food frame
within 200 of E2-d's. Falsified if the loads are claimed and the shelf
arrivals stay under 10 (the errand is dropped on the way), or if field
hauls fall to zero while the errands run (the lift starves the lane's
own work).

### E2-e landed (b10b561b59, staged 12:58)

Check clean, both pins green (its own and E2-b's re-stated for the
four-hour window), both halves staged 12:58. The pair carries W10-g
(the glide back) and T1-b. The falsifier (the lift removed) and the
b1 reader (`wait-e2e-b1.sh`: the round's hour, the errands' arrivals
to shelves before 16, the sweep, night meals and the starving, read
at hour 6 of days 1 and 2) run from the stage; b1 leaves the held
T1-b pair for this one. The falsifier (the lift removed) went red and
restored clean at 13:01; lab-bin carries the pair from 12:59.

Early read (b1, day 0 hour 17, 13:15): SUPPER ROUND day=0 hour=12
houses=49 shortfall=98 loads=36 no_shelf=0 skipped_no_small=0 (the
window bar: hour 12); Haul arrivals since the round by destination
kind: private 21, general 4 (E2-d by the sweep: private 2); "job
claimed" 579, arrivals 637, Haul-kind arrivals 65; in_bags 66 (bar
400); private_units 0 on the day-0 STORAGE SUMMARY (that line fires
at boot, before the round -- the boot-order artefact noted before,
not a null). 21 of 36 by hour 17 against the bar of 25 by hour 16:
under the bar as written, ten times E2-d. The sweep at dusk, the
night meals and the sleepers are read at hour 6 of day 1.

### E2-e after night 1 (b1, read at day 1 hour 6, 13:35; `read-e2e-nights.sh`)

The restart reader had misfired at boot on the old log's clock
(memory addendum); this read is the live log at hour 6. SUPPER ROUND
day=0 hour=12 loads=36; SWEPT day=0 swept=18 (E2-d: 35; bar under
10: NOT met); arrivals to private shelves before the sweep 21 (E2-d:
2; bar 25: NOT met); NIGHT MEAL AT HOME 8 (E2-d: 1; bar 5: MET);
no_food_found 204 on the day-1 census (E2-d: 777; E2-b: 583); meals
48; YEAR day=1 food_stock 4,052 / 4,090 / 25.8 days (E2-d: 4,067,
within 200: MET). Starving by clock: RestAt/Arrived 14, 17, 22 at
hours 1-3 with RestAt/Traveling 6-8 (E2-d night 1: 14, 16, 14, 24 --
UNCHANGED; bar 3 an hour: NOT met); distinct starving sleepers 5
(E2-d: 5; bar 1: NOT met); the morning EatFrom/Traveling rush of
hours 5-9 is GONE (E2-d: 18, 8, 14, 13; here none after hour 4's
11), and the evening EatFrom/Traveling 6-9 an hour through 15-23
stands (W11's class). in_bags 33-132 by day, then 591 from tick
42,900 to 46,500 (hours 2-3): BAG CENSUS top=[(124, 454), (125, 86),
(134, 15)]; uid 124 is a Guard (the night watch's shift end admits
its haul, and the pickup took the stack -- E2-f's class, queued),
back to 131 at tick 46,800. The Haul lane's day line: 7 colonists,
35 hauls, 12 works (E2-d: 23 hauls, 20 works) -- the errands did not
starve the lane, they fed it.

Disposition: E2-e delivers the loads that get claimed and the
houses that get one eat at night (8 meals, 204 misses); but the
lane carried 21 of 36 in four hours and 28 of 49 short houses got
nothing, and the five sleepers who starve are in those. The
throughput is the limit (35 hauls a day for seven haulers), and
more loads will not deliver. The next lever is not the lane: the
eater carries its own supper home (E2-g, registered below once the
round's destination and the home lookup are read).

### E2-e after night 2 (b1, read at day 2 hour 6, 14:16)

SUPPER ROUND day=1 hour=12 houses=? shortfall=98 loads=31 no_shelf=0
**skipped_no_small=23** stale_removed=18; arrivals to private shelves
before the sweep 16 (day 0: 21); SWEPT day=2 swept=17; NIGHT MEAL AT
HOME 9 cumulative (night 2 added ONE); no_food_found 6,823 on the
day-2 census (day 1: 204); meals 119; YEAR day=2 food_stock 3,610 /
3,655 / 22.6 days (day 1: 4,052; down 442 against ~150 eaten).
in_bags through day 1 and night 2: 510, 549, 490, 518, 472, 472,
469, 467 -- about five hundred units in bags all day (E2-f's class;
E2-f is building). Starving by clock, night 2: RestAt/Arrived 18, 24,
15 at hours 1-3 (night 1: 14, 17, 22); distinct starving sleepers 7
cumulative (night 2 added two); the EVENING rush EatFrom/Traveling
14, 20, 24, 24, 16 at hours 15-19 (night 1's evening: 6-9) -- the
town hunts food in the evening because the larder is in bags and the
shelves are bare. **E2-e FAILS its night bars on both nights**; its
mechanism holds (the loads that are claimed arrive: 16 of 31), and
the round found nothing to mint for 23 houses: `supper_stack_pick`
returns None when no store stack is within SUPPER_STACK_MAX (8) --
by day 1 the store's small stacks are gone and the big ones (harvest,
consolidation) are all that is left. Two rows carry it: E2-f (the
pickup splits a load, so the carry is bounded whatever the stack) and
E2-h below (the round may then pick any stack).

## E2-h, registered 14:20 (keyed on the W12-i1 stage, the end of the chain, before the binary)

`supper_stack_pick`: the smallest stack covering the need within
the cap; else the largest under the cap; else -- new -- the SMALLEST
stack of all (E2-f's pickup splits a load of sixteen off it; the
carry stays bounded and the shelf's deposit cell caps at sixteen).
skipped_no_small becomes a count of "no food in the store at all".
Pin: E2-d's `the_supper_load_is_a_small_stack` gains the case
[800, 900] -> index 0 (800, the smallest of all) and [] -> None;
planted: the fallback removed (None again), red. Prediction (b2
fresh on the pair, day 1's round at hour 12): skipped_no_small 0
(23) with loads = the short houses (31 with 23 skipped -> 49-54);
private-shelf arrivals before the sweep at least 30; with E2-f in
the pair, no supper carry above 16 units. Falsified if the picks
land on the largest stacks and the store's food frame drops by more
than the loads' units (the split did not bound the carry).
Rejected: raising SUPPER_STACK_MAX (the cap was the E2-d fix for the
whole-stack strand; E2-f bounds the carry at the pickup instead).

### E2-g landed (3abbbdcf3d, staged 14:45)

Check clean, the pin green, both halves staged 14:45. The falsifier
(the lift removed) and the b1 reader (`wait-e2g-b1.sh`: after W11's
day-1 read, a fresh b1 with the fresh-boot guard; SUPPER CARRIED
HOME, the round, the shelf arrivals, the sweep, night meals and the
starving at hour 6 of days 1 and 2) run from the stage; the W12-i1
chain keys on it. The falsifier went red and restored clean at 14:48;
lab-bin carries the pair from 14:46.

b1 day-1 line (hour 0 of day 1, 15:40): STORAGE SUMMARY
private_units=39 (E2-e's day 1: 21; E2-d: 0 and 12); the lanes'
works Build 86, Cook 67, Mine 56, Farm 50, Craft 40, Guard 6 (about
305) with hauls in every lane (Mine 9, Farm 9, Build 15, Cook 5 --
the eaters carrying), the Haul lane 37 hauls; the E2-e day on this
world: Build 183, Mine 115, Cook 72, Craft 76, Farm 66 (about 530).
**The works fell by about two fifths**, past the third the row named
as its own falsifier: the eater's supper claims at 6 from hour 12,
and four of the eight work hours go to the errand. The lane counts
differ between the two days (Build 8 against 12) and colony counts
vary, so the size is not exact; the direction is. The night-1 read
(hour 6) gives the benefit side; the re-cut (E2-g-b) is the window:
the eater carries its supper on the way home -- the last work hour
and the Leisure hours -- not from noon.

After night 1 (b1, hour 6 of day 1, 15:48; the pair 79ff47e087 =
W12-i1's, which carries E2-g and E2-f, not E2-h): SUPPER ROUND day=0
hour=12 shortfall=98 loads=54 skipped_no_small=0; arrivals to
private shelves before the sweep 48, general 9 (E2-e: 21 / 4; bar
40: MET); SWEPT 14 (E2-e 18; bar 10: not met); SUPPER CARRIED HOME
own_supper_claims=32 (bar 25: MET); NIGHT MEAL AT HOME 9 (E2-e: 8;
bar 15: NOT met); no_food_found 305 (E2-e 204); YEAR day=1 food_stock
4,103 / 26.2 days (bar MET); the Haul lane 37 hauls (bar 25: MET);
in_bags 19-171 by day (E2-f in the pair). Starving by clock:
RestAt/Arrived 7, 16, 14, 18 at hours 0-3 (E2-e: 14, 17, 22 -- bar
6: NOT met, and unchanged); distinct starving sleepers 3 (E2-e 5;
bar 2: not met); the evening EatFrom/Traveling 10, 19, 10 at 19-21
and RestAt/Arrived 6-8 at 21-23. **The shelves are stocked and the
sleepers do not eat from them**: 48 houses got their supper, the
night meals stayed at nine, and two bodies starve in bed through the
night as before. The carrying is done; the eating is the row -- the
night-meal rule's own conditions (read next).

After night 2 (b1, hour 6 of day 2, 16:28; cumulative): the day-1
round shortfall=91 loads=30 **skipped_no_small=20** stale_removed=14
(E2-e's day 1: 23 skipped, 31 loads -- the same shortage on the
same world a pair later; this pair lacks E2-h); arrivals to private
shelves before the sweep 21 (day 0: 48); SWEPT day 2: 13; SUPPER
CARRIED HOME 32 (no new claims on day 1: the loads were few);
NIGHT MEAL AT HOME 10 cumulative (night 2 added one); no_food_found
6,161 on the day-2 census (day 1: 305); meals 113; YEAR day=2
food_stock 4,158 / 26.0 days (up: E2-f holds the larder). Starving
by clock, night 2 alone: RestAt/Arrived 23, 32, 28, 38 at hours 0-3
(night 1: 7, 16, 14, 18), EatFrom/Traveling 35 at hour 4 (the rise),
19-21 through 11-13 (the noon errand's hungry walkers), 28 at hour
20; distinct starving sleepers 10 cumulative (night 2 added seven).
Night 2 without a stocked shelf is E2-e's night 2: the stack
shortage empties the round, and the eaters carry nothing because
nothing is minted. E2-h's fallback (b2, the day-1 round at ~16:35)
is the test; E2-i1 reads what the shelf held on the nights it was
stocked.

E2-g's cost, second replicate (b2 on the E2-h pair 9cbeb09719, which
carries the noon lift; the day-1 line at 16:13): works 154, hauls
123, haul_share 44%, private_units 39 -- against b2's own E2-f day
(336 works, 106 hauls) and W10-g day (345 / 73). Two worlds, the
same direction: the noon lift takes half the day's works. E2-g-b
(the walk-home window) is the re-cut.

### E2-h landed (9cbeb09719, staged 15:31)

Check clean, the pin green (E2-d's re-stated), both halves staged
15:31. The falsifier (the last fallback refusing) and the b2 reader
(`wait-e2h-b2.sh`: after W12-i1's +10 read, a fresh b2 with the
fresh-boot guard; the day-0 round at hour 13, then nights 1 and 2 at
hour 6) run from the stage; the W11-b chain keys on it. The falsifier
went red and restored clean at 15:34; lab-bin carries the pair from
15:32.

b2 day-0 round (hour 12, read 15:56): houses=49 shortfall=98
loads=58 no_shelf=0 skipped_no_small=0 (E2-g's day 0: 54 loads,
0 skipped -- the day-0 store still holds small stacks, so day 0 does
not test the fallback). The row's test is the day-1 round (E2-e's
day 1: 23 skipped, 31 loads), read at hour 13 of day 1 (~16:35).

After night 1 (b2, hour 6 of day 1, 16:21): the day-0 round's loads
58, arrivals to private shelves before the sweep 38 (general 3),
SUPPER CARRIED HOME 16, SWEPT 26, NIGHT MEAL AT HOME 6, no_food_found
282, meals 48, YEAR day=1 food_stock 3,976 / 25.4 days; in_bags
62-128 by day (E2-f in the pair). Starving by clock: RestAt/Arrived
5, 8, 11, 16 at hours 0-3 with RestAt/Waiting 7, 7 at 0-1 and
RestAt/Traveling 6-7 at 2-3; distinct starving sleepers 3; the
evening EatFrom/Waiting 11, 11, 7, 6 at 17-20 (a queue at the food,
a state E2-e's nights did not show). The second world's night says
what b1's E2-g night said: the shelves are stocked and two or three
bodies starve in bed regardless. E2-i1 reads the night shelf.

The day-1 round (b2, hour 12 of day 1, read 16:32): houses=50
shortfall=89 **loads=51 skipped_no_small=0** stale_removed=26 (E2-e's
day 1: 23 skipped, 31 loads; E2-g's on b1: 20 skipped, 30 loads).
**E2-h PASSED its round bars** (skipped 0; loads at least 45): the
fallback mints a load for every short house from the big stacks. The
night-2 read (hour 6 of day 2, ~16:50) gives the deliveries, the
night meals and the sleepers on a night whose shelves were minted
for.

After night 2 (b2, hour 6 of day 2, 17:03; cumulative): the day-1
round's 51 loads -> arrivals to private shelves before the sweep 24,
SWEPT day 2: 30 (day 0: 58 loads, 38 arrivals, 26 swept); SUPPER
CARRIED HOME 32 cumulative (16 on each day); NIGHT MEAL AT HOME 8
cumulative (night 2 added two); no_food_found 4,856 on the day-2
census (day 1: 282); meals 117; YEAR day=2 food_stock 3,923 / 24.5
days (day 1: 3,976 -- steady, E2-f holds it); in_bags through day 1
146-236 (the bounded loads in flight; the pair carries E2-f, and
BAG CENSUS holders were under 64 on its own read). Starving by
clock, night 2 alone: RestAt/Arrived 7, 10, 14, 14 at hours 0-3
(night 1: 5, 8, 11, 16), EatFrom/Traveling 5-14 through the night
(hungry bodies walking at 1-3: the curfew is not holding them --
they have no home to be held to? E2-i1 reads it), the rise 22 at
hour 4; distinct starving sleepers 6 cumulative (night 2 added
three); the evening EatFrom/Waiting 11, 11, 7 at 17-19 (the queue at
one pile) and RestAt/Traveling 6-8 through 13-17 (walking to rest
hungry in the afternoon). **E2-h PASSED its round bars and its
outcome did not move the night**: the loads exist, 24 of 51 arrive,
30 are swept, and the sleepers starve as before. The carry is the
limit on this world too (the pair has E2-g's noon lift and E2-e's
errand); E2-g-b re-cuts the eaters' window, E2-i1 reads the shelf
the sleepers wake beside.

## E2-i1, registered 15:55 (keyed on the W12-i2 stage, the end of the chain, before the binary)

An instrument row from E2-g's night: the curfew never fired
(curfew=0), every sleeper owns a bed (beds_owned 50 of 50), the
night pick is already scoped to the home region (`night_ok`), and it
returned nothing 305 times. At `eat_skip_count("no_food_found")`
during Sleep the home region's food is counted -- present,
refused_reach (the component index), refused_cap (the reservation),
refused_closed (a closed store) -- and `night_shelf_verdict(home_known,
present, admissible)` -> NoHome / Empty / Refused is counted into the
EAT CENSUS (night_no_home, night_shelf_empty, night_shelf_refused)
and logged as NIGHT SHELF EMPTY at the first eight and powers of two.
Pin `the_night_shelf_names_its_emptiness`; planted: Empty and
Refused swapped, red. Prediction (b2 fresh, `wait-e2i1-b2.sh`, night
1 at hour 6): the three night keys sum to the night's no_food_found;
night_no_home 0; Empty at least 70% (coverage: the row after is the
round's ordering against the sweep) else Refused at least 70% with
refused_reach dominant (the shelf cell's component: the shelf's
placement). Rejected: scoping the pick further; lifting a curfew that
never fires; guessing between bare and refused. NOT evidenced: the
day-side no_food_found (the evening rush is a day pick).

### E2-i1 landed (61fbaebf71, staged 17:11)

The first chain refused at cargo check (a double deref of the item
uid in the new count loop; fixed to a single deref, the tree
restored to HEAD before the relaunch). Check clean, the pin green,
both halves staged 17:11. The falsifier (Empty and Refused swapped)
and the b2 reader (`wait-e2i1-b2.sh`: after W12-i2's +10, a fresh
b2; the round at hour 13, then the night-shelf verdicts and the EAT
CENSUS night keys at hour 6 of days 1 and 2) run from the stage. The
falsifier went red and restored clean at 17:14; lab-bin carries the
pair from 17:11.

### E2-i1 night 1 on b2 (pair 61fbaebf71, read 17:48 at hour 6 of day 1): REFUSED BY REACH, then EMPTY

NIGHT SHELF EMPTY 16 lines: Refused 9, Empty 7. Every printed
Refused carries present 1, refused_reach 0, refused_cap 1,
refused_closed 0: the house held ONE food stack and `has_capacity`
refused it -- as many reservations against the stack as it has
units (the eat path's per-unit reservation, #89), so a one-unit
stack reserved by the housemate, or by the sleeper's own earlier
pick, is a refusal. Not the reach: the component index admitted
every one. The sixteen lines are FOUR sleepers: colonist 64 nine
times Refused from the house at min (7744, 6402), the same single
stack every time; colonists 20 (five times), 34 and 44 once each,
Empty. So the night's shape on this arm is one sleeper locked out
of its own house's one stack by a reservation that never clears,
and three bare shelves. Colonist 64's house (min (7744, 6402), two
adopted beds at (7756, 6412, 186) and (7757, 6412, 186), one pot):
SHELF ADDED put the one-cell shelf at (7757, 6412, 186) -- the
SECOND BED's own cell ("beside its first bed" walked one east into
the other bed). The nine refusals came six seconds apart at
21:32:28-21:33:20 (the night pick's retry cadence); at 21:33:31 the
hunger preempt sent 64 to the general store at (7774, 6355) and it
ate there (EatFrom item 846, arrived). The refuser's identity (the
housemate's reservation, or 64's own stale one) is not in the
witness; E2-i2 prints it before anything is built on it. The round: shortfall 98, loads 49, no_shelf 0, skipped 0,
swept 8; arrivals before the sweep general 29, private 45 (E2-h's
count on this arm); SUPPER CARRIED HOME 2 (the one-hour window, this
pair); NIGHT MEAL AT HOME 14; in-bed starving at hours 0-3: 7, 8, 7,
16; hour 4: EatFrom/Waiting 9, EatFrom/Traveling 3; distinct
starving sleepers 2 (b1 on E2-g-b: 8 -- another arm, another world);
food_stock 4,117, days_of_food 26; panics 0. The disposition: E2-i2,
the night pick reaches its own house's shelf from any bed of the
house (the shelf is one cell beside the FIRST bed; a sleeper in the
second bed, or upstairs, fails a radius) -- registered below once
the reach's producer is read.

### E2-j night 1 on b2 (pair 98d548575d, read 18:22 at hour 6 of day 1): FAILED ITS OWN CLAUSE

The bars as registered: EatFrom/Waiting at 16-20 at most 3 an hour
-- PASSED (0, 0, 0, 2, 2; was 11); EatFrom/Traveling at 16-20 not
above 12 an hour -- FAILED (6, 12, 20, 22, 28; the E2-i1 pair read
5, 7, 7 at 18-20): the clause "falsified if EatFrom/Traveling
doubles (the spread walks too far)" fired. Meals day 1: 46 (bar 45,
passed). The night, which the row said it would leave unchanged:
NIGHT MEAL AT HOME 33 (E2-i1 pair: 14) and arrivals to private
shelves before the sweep 50 (45), general 27 -- better; but
EatFrom/Waiting 12, 14, 16 at hours 21-23 and 17, 19, 14, 16 at
0-3 (E2-i1 pair: none before hour 4), in-bed starving 5, 17, 21,
24 (7, 8, 7, 16), distinct starving sleepers 5 (2), NIGHT SHELF
EMPTY 17 (Refused 14, Empty 3), no_food_found 414 (184). The queue
the row cleared from the evening re-formed at the night shelves.
Disposition: E2-j does not stand as written. E2-j-b (registered
below) keeps the spread but bounds it to the store: an unreserved
stack beats a reserved one only within SPREAD_REACH of the nearest
admissible stack; queued ahead of E2-l so the night rows read on
the bounded pick. The reader's own EatFrom/Waiting-by-hour python
block died on an unterminated string (a heredoc-eaten newline); the
hour table above is from the census block, which printed.

### E2-g-b night 2 on b1 (pair 7d28997261, read 18:04 at hour 6 of day 2): WORSE THAN NIGHT 1

In-bed starving at hours 0-3: 16, 34, 56, 73 samples (night 1: 7,
18, 35, 43); hour 4: EatFrom/Traveling 50, Recreate/Traveling 12;
hour 5: EatFrom/Traveling 16; distinct starving sleepers 13 (night
1: 8). The round on day 1: shortfall 96, loads 55, stale_removed 30
(the day-0 loads never delivered); arrivals before the sweep private
27, general 1; SWEPT day 2: 32; SUPPER CARRIED HOME 16 claims over
two days; NIGHT MEAL AT HOME 7 over two nights. EAT CENSUS day 2:
meals 113, no_food_found 9,046 (the pick retried every tick through
the night). food_stock 3,867, days_of_food 24.7. In-bags rose
through day 1 (89 -> 192 at hour 14) and fell to 96-123 by night:
the bags are not the larder (E2-f held). The one-hour window
(E2-g-b) FAILS night 2 by every bar; E2-g-c (two hours) boots on b1
from this read. The bed count that matters: 13 of 50 asleep hungry,
and 50 samples of eaters travelling at hour 4 -- the curfew's end
sends the whole starving house out at once.

### E2-g-c landed (1c3ebf3a2e, staged 17:58)

Check clean, the pin green, both halves staged 17:58, shipped to
lab-bin 17:59. The falsifier (the walk home from noon) went RED at
18:01, the tree restored clean. The b1 reader (`wait-e2gc-b1.sh`)
restarts b1 after E2-g-b's night-2 read and reads nights 1-2.

### E2-g-c night 1 on b1 (pair 1c3ebf3a2e, read 18:38 at hour 6 of day 1): FALSIFIED BY ITS OWN CLAUSE

Against the bars: arrivals to private shelves before the sweep at
least 40 -- 23 (E2-g-b 25, E2-g 48), FAILED; SUPPER CARRIED HOME at
least 24 -- 16 (16 / 32), FAILED; the lanes' works at least 420 --
444 (481 / 305), PASSED; NIGHT MEAL AT HOME at least 8 -- 4 (4 / 9),
FAILED; distinct starving sleepers at most 4 -- 5 (8 / 3), FAILED;
in-bed starving at most 12 an hour at 0-3 -- 0, 7, 16, 24 (7, 18,
35, 43), FAILED at hours 2-3; the Haul lane's hauls at least 25 --
41, PASSED. The clause "falsified if shelves stay under 35 (the
window is not the lever)" fired: 23. The round minted 59 loads,
SWEPT 43 at dusk; the eaters' nine claims fell at hours 13-14 (the
DAY SCHEDULE hour, which lags), none after the whistle; hour 4 sent
20 samples of eaters out. So the two-hour window keeps the works
and does not stock the shelves, as E2-g-b did not: the lever is the
claim door (E2-l, queued), not the window's width. E2-g-c stays in
the pair (its cost is nil: works 444) until E2-l's read; if E2-l
stocks the shelves the window can narrow back to one hour in a
later row, measured. The night's other shape, new here: RestAt/
Traveling starving 3, 8, 7, 13 at hours 0-3 -- two bodies, and 26
of the 31 samples are colonist 9: rest-preempted at tick 18,583
(hour 15) toward the bed at (7583, 6383, 181), still Traveling at
hour 3, released by the hunger preempt at tick 47,263 -- a body
that walked toward its bed for 28,000 ticks and never arrived. Not
a night-shelf case: a stranded walker, the wedge class (W12-b, W13
in the queue); E2-l's read watches whether it recurs.

### E2-g-c night 2 on b1 (read 19:17 at hour 6 of day 2): AS BAD AS E2-g-b's

In-bed starving at hours 0-3: 9, 29, 50, 64 (E2-g-b night 2: 16,
34, 56, 73); distinct starving sleepers 13 (13); hour 4: EatFrom/
Traveling 34 (50); day-1 round loads 55, stale_removed 43 (the
day-0 loads never delivered), arrivals to private shelves 25 (day
0: 23), SWEPT 36; SUPPER CARRIED HOME 32 claims over two days; NIGHT
MEAL AT HOME 15 over two nights; no_food_found 7,725 by day 2;
food_stock 3,907. A second replicate of the same verdict: the
window's width does not stock the shelves. New in this read:
RestAt/Waiting 5-8 samples an hour at hours 22-3 -- sleepers
WAITING at their bed's anchor because another body stands nearer
it (the Waiting rule is the eat queue's, applied to beds): either
two sleepers on one bed, or a housemate standing beside it. Read
below before it is named (B7-2 assigns beds to sleepers; a shared
bed would be a defect of that row).

### E2-j-b landed (230257dddd, staged 19:53)

Check clean, the pin green, both halves staged 19:53 and shipped to
lab-bin 19:53. The row has no witness string of its own (a pure
ordering change); the binary carries W13's string and the pair's
lineage, and the behaviour is read live. The falsifier (the reach
unbounded) went RED at 19:56, the tree restored clean. The b2
reader restarts b2 after W12-b-b's day-1 read and reads the evening
walk, the queue and night 1.

### The W13 pair's night 1 on b1 (21edda27ad, read 20:00 at hour 6 of day 1; one-off, no E2 row new since E2-g-c)

The one-cell store (W12-b, in this pair) reaches the supper: SUPPER
ROUND loads 30, skipped_no_small 20 (E2-h's skip: a single 12,000-
unit pile has no small stack), arrivals to private shelves 27,
SUPPER CARRIED HOME 8, NIGHT MEAL AT HOME 3, no_food_found 1,387,
distinct starving sleepers 6. The night's samples: at hours 20-3
every census sees three starving bodies -- one RestAt/Waiting (the
bed queue, E2-m's), one RestAt/Traveling, one in bed -- 7-8 samples
an hour each; hour 4 sends 15 eaters out. The same shape as E2-g-c's
night 2, on the pair that also carries W12-a, W12-b and W13; the
wedge rows did not change the night. E2-l and E2-m read next on b1.

### E2-j-b night 1 on b2 (pair 230257dddd, read 20:34 at hour 6 of day 1): THE EVENING PASSED, THE NIGHT IS CONFOUNDED

Against the bars: EatFrom/Traveling at 18-20 at most 12 an hour --
0, 10, 10 (E2-j: 20, 22, 28), PASSED; EatFrom/Waiting at 16-20 at
most 4 -- 1, 2, 7, 8, 2 at 16-20 (E2-j 0-2; before E2-j 11): FAILED
at 18-19 and at the clause's edge ("returns above 8": 8); Waiting at
21-3 at most 6 -- 0, 0, 0, 3, 8, 2, 0 (E2-j 12-19), PASSED but hour
1; in-bed starving at 0-3 at most 16 -- 7, 17, 35, 46 (E2-j 5, 17,
21, 24), FAILED; distinct starving sleepers at most 3 -- 7 (5),
FAILED; NIGHT MEAL AT HOME at least 25 -- 12 (33), FAILED and the
clause "under 20: the night's gain was the unbounded spread's"
fired as written; meals day 1 at least 45 -- 44. NIGHT SHELF EMPTY
17: Empty 16, Refused 1 (E2-j: 3 and 14) -- the shelves are BARE,
not locked; arrivals to private shelves before the sweep 32 (E2-j
50, E2-i1 45), SWEPT 32; no_food_found 821 (414).

The confound: this pair carries W12-b-b (the store spread over
cells whose standable test fails) and not W12-c (the plank floor);
the supper loads' pickups sit inside the plank (W12-c's read: every
store cell one block under the floor until 20:15), and the round's
arrivals fell from 50 to 32 -- the bare shelves are the plank's,
not the spread's. So E2-j-b PASSED the evening bars it was built
for and its night bars are not readable on this pair. Disposition:
E2-j-b stands for the evening; the night is re-read on b1's E2-l
night (the W12-c pair carries E2-j-b), and if NIGHT MEAL AT HOME
stays under 20 there with the shelves stocked, the bound gives the
night back and comes out.

### The W12-c pair's night 1 on b1 (8f1bd71ae6, read 20:50 at hour 6 of day 1; one-off, the first night with the larder out of the floor)

SUPPER ROUND loads 56, skipped_no_small 0 (the W13 pair's one-cell
store: 30 and 20), arrivals to private shelves 31 (E2-g-c 23, the
W13 pair 27), SUPPER CARRIED HOME 16, SWEPT 35, NIGHT MEAL AT HOME 5,
no_food_found 113 (the W13 pair 1,387; E2-g-c 241) -- the pick's
failures fell by an order of magnitude with the larder reachable.
The night itself is unchanged in kind: in-bed starving 7, 10, 21,
38 at hours 0-3, RestAt/Waiting 4, 7, 8 at hours 1-3 (E2-m's bed
queue), hour 4 sends 31 eaters out, distinct starving sleepers 7.
The store is right; the shelves are not stocked; E2-l reads next on
this arm.

### E2-l landed (bbcbc5944e, staged 21:00)

Check clean, the pin green, both halves staged 21:00; the binary
verified by its witness string ("THE WALK HOME OPENS" present once),
shipped to lab-bin 21:00. The falsifier (Leisure always open) went
RED at 21:03, the tree restored clean. The b1 reader restarted the
160-day arm at 21:01 (after W12-c's day-1 block; the store's
biggest cell 96 at boot) and reads the walk home's claims by hour,
the door, the shelves and night 1 (about 21:35).

### The W13-b pair's night 1 on b2 (b9d05905ca: the store fixed, E2-j-b aboard; read 21:12 at hour 6 of day 1; one-off)

The E2-j-b night re-read on a pair with W12-c: SUPPER ROUND
shortfall 66 (98 on every earlier pair: the shelves already held
32 units at noon), loads 36, arrivals to private shelves 32 and to
the general store 22, SWEPT 9; NIGHT MEAL AT HOME 13 (E2-j 33, E2-i1
pair 14, the W13 pair on b1 3, E2-g-c 4); in-bed starving at hours
0-3: 0, 5, 15, 24 (E2-j-b's plank pair 7, 17, 35, 46; E2-i1 pair 7,
8, 7, 16); distinct starving sleepers 5 (7); hour 4 sends 10 out;
no_food_found 389 (821); RestAt/Traveling 7-8 an hour all night
(one body walking to bed hungry). So with the larder reachable the
night improves but E2-j-b's NIGHT MEAL AT HOME bar (25) still fails
at 13, and across every pair the number sits at 3-14 except E2-j's
33. The reading that fits: the evening's eaters eat the supper off
their own shelf before bed (the hunger preempt at 18-21 picks the
nearest stack, which at home is the shelf), and the night meal is
what is left; E2-j's unbounded spread sent the evening's eaters to
unreserved stacks across town and by accident left the shelves for
the night. Measured below (the evening's eats inside a house) before
it is named: the shelf is for the night (E2-n).

The measure (the same b2 log, 58 houses): EatFrom arrivals in the
evening (16-21) 25, of which 13 inside a house (52%); at night
(22-5) 17, 12 inside a house. And the meal's span, from an eat
arrival to the same colonist's next hunger interrupt: n 21, median
12.4 game hours, p25 10.3, p75 15.4, min 6.3, max 17.8 -- a meal is
half a day, two meals a day, human. So a colonist who ate supper at
19 does not starve at 2; the starving sleepers are the ones who did
NOT eat in the evening (25 evening eats for a town of 50: half the
town ate its last meal at midday and hits the interrupt asleep,
when the pick is home-only and the shelf is what the evening's
eaters left). The row this names is not the shelf's ownership but
the supper's TIME -- and the supper hour already exists
(`supper_hour`: the two hours before the colonist's own Sleep block,
`SUPPER_LINE` 0.6 raising the interrupt there, `SUPPER_SEVERITY`
0.5 giving it weight). Traced on the same log: colonist 24 ate at
hour 15 (a raw meal: FOOD_RESTORE 0.5, so 0.23 -> 0.73), was posted
to the evening visit at 16 and again at 19 (RECREATE, "schedule, not
need"), had no hunger preempt at 20-21 though its hunger stood near
0.37 (under the 0.6 line), took the rest preempt at 21, was in bed
at 22 and starving by hour 1. The EAT CENSUS's skips on day 1:
drive_not_personal 6,648, preempt_cooldown_active 7,392 -- the need
scan does not run while the arbitration's drive is Leisure, and in
supper hours the supper's severity (0.5) does not outrank the
scheduled evening visit. The supper line's own construction (0.6 -
0.27 for an eight-hour night at half burn = 0.33 > 0.2) assumed a
sleeper entering bed at 0.6; a colonist fed raw at midday enters
bed at about 0.3 and crosses 0.05 by hour 1. E2-o is therefore
SUPPER OUTRANKS THE VISIT: in supper hours a hunger under the line
wins the arbitration over the scheduled leisure, after E2-l's and
E2-m's reads. The cohort (the same log, day 0): 46 colonists ate;
17 of them ate last at or before hour 17 and not at 20-21; 15 of
the 17 were posted to the evening visit (RECREATE at 16-21); 4 of
the night's 5 starving sleepers are among them (24, 30 and 36 ate
at 15 and were posted to the visit at 16 and 19 with no preempt
after 15; 64 ate at 16, visit at 17); the fifth, 41, was preempted
at 20 and 21 and found no meal. A third of the town skips supper
for the visit and a quarter of those starve in bed.

## E2-n-i, registered 22:12 (keyed on the E2-k stage; ahead of E2-i2, W12-a-b and W15-i1)

A MEAL NAMES ITS SOURCE. An instrument row for E2-o's open
question and E2-n's premise: the meal line ("ate — hunger
restored", uid and job) says nothing about where the food was, and
NIGHT MEAL AT HOME names a shelf meal only when the pile lies
inside the eater's own house. Now the line carries the pile, the
store kind under it (`meal_source`: private = a household shelf,
general = a store, none = no store under the pile) and the hour;
`stockpile_region_at` sits beside `stockpile_at` with the same
containment rule and returns the region too. No behaviour changes.
Pin `a_meal_names_its_source` (private under a shelf, general
under a store, none under nothing); planted: the shelf named as the
store, red. Prediction (b1, read on E2-i2's pair after its night-1
block by `wait-e2ni-b1.sh`): every meal line carries a source, and
"none" is under 10% of meals; the private night meals (22-5) agree
with NIGHT MEAL AT HOME within 2; the evening (20-21) split is
recorded without a bar. Falsified if a meal line lacks a source or
the two shelf counts disagree by more than 2. Rejected: deriving
the source in the reader from the pile's coordinates (the region
rule lives in the engine); a counter per kind (the distribution by
hour is the question). NOT evidenced: E2-n itself; b2. The E2-i2
chain was re-keyed behind this row by its pid file; the dry tree
was rebuilt in the order E2-m, E2-o, W13-b-r, E2-k, E2-n-i, E2-i2,
W12-a-b, W15-i1.

## E2-o, registered 21:25 (keyed on the E2-m stage; ahead of E2-k and E2-i2)

SUPPER IS EATEN AT SUPPER TIME. Two defects in one: (1) the supper
hour's raised line (`supper_interrupt`, 0.6 in the two hours before
the colonist's own Sleep block) is applied to the ARBITRATION's
interrupt and never to the NEED LOOP's `hunger_th`, which stays the
stagger interrupt (0.2) -- the arbiter says Personal at 0.37 and
the loop finds no candidate (no_need_below_interrupt 79,129 on day
1); (2) the leisure lounge branch posts the visit and continues
before the candidates are consulted whenever a colonist is in
Leisure, idle and off cooldown, under the false comment "needs
still outrank". Now `hunger_th` carries the supper line in supper
hours, and `lounge_may_post(candidates.is_empty(), own_supper_pending)`
gates the visit, with THE LOUNGE YIELDS TO A NEED (colonist, hunger,
rest, own_supper, yields). AMENDED 21:45, before the chain fired, on
E2-l's early read (b1, E2-l pair, day 0, hour 21): the door works
when the eater is free (all eight colonists named by THE WALK HOME
OPENS at 16-18 claimed their own load next) but the visit is an
active job and the claim scan admits only the job-free: 152 visits
posted at 16-21 (16: 32, 17: 12, 18: 12, 19: 34, 20: 48, 21: 14)
against 13 own-supper claims (16: 3, 17: 2, 18: 4, 19: 3, 20: 1),
and 19 of the round's 54 loads SWEPT unclaimed at the Sleep block;
the two colonists named at hour 20 took the visit instead of the
load. The lounge now yields to an own unclaimed supper load as well
(E2-l's own predicate on `supper_eaters`). Pin
`the_lounge_yields_to_a_need` (posted only with no need pending and
no own load unclaimed; the supper line in supper hours, the base
otherwise); planted: the lounge posting over an own unclaimed load
(`no_need_pending || own_supper_pending`), red. Added bars: own-supper
claims at the Sleep block at least 40 of the round's loads (32 of 54
by hour 20 on the E2-l day 0); SWEPT at most 10 (19); falsified if
the swept stay above 15 with the yields up. Prediction
(b1 fresh, `wait-e2o-b1.sh`, after E2-m's night-1 block; day 0 and
night 1): hunger preempts at 20-21 at least 20 (12 on the W13-b day
0); meals at 20-21 at least 18 (11); the cohort (last meal by 17, no
supper) at most 5 (17); distinct starving sleepers at most 2 (5);
in-bed starving at 0-3 at most 8 an hour (0, 5, 15, 24); NIGHT MEAL
AT HOME not above the pair before; lounge yields at least 20.
Falsified if the supper preempts stay under 15, or the cohort
stays above 10 with the preempts up, or the visits fall to zero.
Rejected: a higher supper severity; a supper self-job for all; a
lower supper line. NOT evidenced: night 2; the night watch's supper
hour; whether the meals come off the shelf or the store (E2-n's
question, read on this pair). The E2-k chain and reader were
re-keyed behind this row; the dry tree was rebuilt in the order
E2-m, E2-o, E2-k, E2-i2, W12-a-b, W15-i1 on HEAD = W13-w.

### E2-l read (b1 fresh on bbcbc5944e; day 0 and night 1, read 21:41): the door works, the visit holds the eater

The bars: THE WALK HOME OPENS 32 by hour 20 (bar 20, PASSED);
private haul arrivals before the sweep 43 of 46 (bar 40, PASSED);
distinct starving sleepers 3 (bar 4, PASSED); leisure-hour
own-supper claims 13 (16: 3, 17: 2, 18: 4, 19: 3, 20: 1; bar 15,
FAILED by two); works on the day-1 lane lines about 353 (Craft 108,
Build 111, Farm 46, Haul 31, Cook 29, Mine 22, Guard 3; bar 420,
FAILED as written -- but the bar was set above the arm's own
previous replicates: the two b1 logs before this one summed 446
(the 20:16 pair) and 394 (the W12-c pair), so 353 sits within the
colony's replicate band, the lanes were reshuffled by the haul
ceiling's demotions (Craft 108 against 19-22, Mine 22 against
91-92), and the door cannot have cost it: 13 claims of any kind in
the leisure hours; no works cost is evidenced). The round minted 54 loads at hour 12 and SWEPT 19
unclaimed at the Sleep block. What held the loads: the door opens
onto the job-free only, and the evening visit is an active job --
all eight colonists named by the door at hours 16-18 claimed their
own load next, the two named at hour 20 (26, 19) took the visit
instead, and the visit was posted 152 times at 16-21 against the 13
claims. The night then reads as before: NIGHT MEAL AT HOME 7
(12-13 on the pair before); NIGHT SHELF EMPTY 7 in the night
(present 0, nothing refused -- the shelf never got its load;
colonist 19 among them); in-bed starving samples 7, 15, 21, 24 at
hours 0-3 from 3 sleepers, who leave for the store at hour 3-4
(EatFrom/Traveling 30 at hour 4); EAT CENSUS day 1 meals 48,
no_food_found 452. Disposition: PARTIAL. E2-l's mechanism holds
(the door is not what fails); the walk home fails at the lounge,
and E2-o was amended (21:45) to yield the visit to an own unclaimed
load before its chain fired. E2-n (the shelf is for the night) is
not answered by this read: the shelves that were stocked (35 of
54) fed 7 night meals, and which shelves those were is E2-i2's
read.

### E2-m read (b1 fresh on bc45399a42; day 0 and night 1, read 22:43): its own bars pass, the night's co-measures worsen

Against its bars: RestAt/Waiting starving samples at hours 22-3 --
none at all (5-8 an hour on the E2-g-c night; bar at most 1):
PASSED; THE QUEUE IGNORES A STRANGER 32,768 by night 1 (bar 20):
PASSED, and the twenty printed lines anchor at z 186 (17), 183 (2)
and 189 (1) -- upper-floor beds, the housemate case the row was
written for, none in the store box at z 182; RestAt/Traveling
starving at 0-3: 0, 0, 0, 6 (bar at most 4 an hour; hour 3 over by
two, the sleepers leaving for the store). The co-measures, one
replicate, all worse than the E2-l pair the day before: distinct
starving sleepers 9 (3); in-bed starving samples at 0-3 14, 25, 28,
43 (7, 15, 21, 24); NIGHT MEAL AT HOME 13 (7); EAT CENSUS day 1
no_food_found 1,040 (452), eat_stalls_tolerated 20 (9),
targets_shunned 25 (15), meals 46 (48); EatFrom arrivals 54 (52);
evening hungry walkers (EatFrom/Traveling) at 18-20 19, 31, 15 (3,
11, 0); in_bags at hour 21 59 (118). The round: 55 loads, 19 swept
(19), 41 private arrivals (43). Works 416 on the day-1 lines (353,
394, 446 before). Disposition: PASSED on its own bars; the night's
worsening is one replicate against a colony that varies 2-3x, and
the E2-o pair (restarted 22:44, carrying E2-m) is its second
replicate -- if the starving sleepers and no_food_found stay at
this order there, E2-m's rule is re-read for a cost at the store
(eaters bound for neighbouring items no longer queue). NOT
evidenced: why the packs ran down faster on this day (in_bags 84
at hour 16 against 119).

### E2-o landed (ead39f481d, staged 22:25)

Check clean on the first try, the pin green (four asserts on the
two-argument rule), both halves staged 22:25; the binary verified
by contents (THE LOUNGE YIELDS TO A NEED present, E2-m's string
present). The E2-m chain before it needed three launches: the
first retyped `queue_snapshot` and broke three other readers
(the dry tree validates anchors, not types); the second borrowed
`board.jobs` while the active job's own mutable borrow was live;
the third took the anchor from the live `job` binding. The b1
reader restarts b1 after E2-m's night-1 block and reads day 0 and
night 1 against the bars above. Falsified: the original pid-less
falsifier failed loudly on its stale one-argument string ("nothing
planted, verdict INVALID", as a precondition must); the
two-argument one planted `no_need_pending || own_supper_pending`
and the pin went RED at 22:30 (the "own supper load unclaimed"
assert), the tree restored clean. Shipped to lab-bin 22:26.

### E2-o read (b1 fresh on ead39f481d; day 0 and night 1, read 23:17): the supper line works, the walk home still stalls after the door

Against its bars. The supper hour (a): hunger preempts at 20-21 18
and 8, 26 (bar 20; 12 on the W13-b day): PASSED; meals at 20-21 18
and 9, 27 (bar 18; 11): PASSED; the cohort (last meal by 17, no
supper) 12 of 48, 6 of them on the visit (bar 5; 17 and 15):
FAILED as written, halved; distinct starving sleepers 3 (bar 2; 5
on the W13-b night, 9 on the E2-m pair, 3 on E2-l): FAILED by one;
in-bed starving samples at 0-3 0, 0, 7, 8 (bar 8 an hour; 0, 5,
15, 24 before; 14, 25, 28, 43 on the E2-m pair): PASSED; NIGHT MEAL
AT HOME 7 (13 on the pair before): PASSED; THE LOUNGE YIELDS 1,024
by night 1 (bar 20): PASSED. EAT CENSUS day 1: meals 69 (46-48),
no_food_found 125 (452 on E2-l, 1,040 on E2-m). Evening hungry
walkers at 18-20 7, 11, 7 (19, 31, 15 on E2-m). The E2-m
co-measures on this second replicate: RestAt/Waiting starving 0
again; RestAt/Traveling starving at 0-3 7, 8, 7, 14 (E2-m's bar 4;
its own pair read 0, 0, 0, 6) -- the late walkers to bed, mixed
across replicates; starving sleepers back to 3. The walk home (b):
own-supper claims 32 by the Sleep block (bar 40; 32 the day
before), SWEPT 18 of 56 (bar 10; 19): FAILED, both -- with the
door now opened 1,024 times (32 the day before; the lounge no
longer holds the eater) and leisure-hour claims of any kind 32
(13). The door opens and the claim still does not follow: the
first nine colonists the door named at 16-18 all claimed within
seconds, the ones named at 20-21 (30, 71) did not; the claim
refusal census through 19-21 reads considered 1,676-2,250 a
window, eligible 0-1, refused all but one, priority_zero
1,504-2,030 (E2-l's own rule on the other jobs), already_claimed
168-215, affordance 4-5. The own load's refusal is not named by
the census -- the next instrument (E2-l-i: the walk home's verdict
per colonist) reads it; the haul gate (non-hauler haul claims,
open at shift end) is the suspect, since the door-named colonists
at hours 16-18 claimed and those at 20-21 did not. Disposition:
PARTIAL -- the supper line and the lounge yield hold (the night's
starvation halved against the pair before and a fifth of the E2-m
pair's), the walk home is still shut one gate further in.

### E2-k landed (eda115908a, staged 23:11)

Check clean, the pin green, both halves staged 23:11 (the row's
string present in the binary). The b1 reader restarts b1 after
E2-o's night-1 block and reads the shelves. Falsified: the bed
admitted as a shelf cell planted at eda115908a, the pin RED at
23:14, the tree restored clean. Shipped to lab-bin 23:11. E2-n-i's
chain fires five minutes after this stage.

## E2-m, registered 19:27 (keyed on the E2-l stage; ahead of E2-k and E2-i2)

THE QUEUE IS FOR THE SAME ANCHOR. The anchor queue (the eat queue's
rule, `staged_at_anchor`) holds a body when ANY colonist stands
nearer its steer by half a block within four blocks of height, and
`queue_snapshot` is every colonist's position; so the housemate
lying in the adjacent bed (worldgen lays beds side by side) is
"ahead of me" as long as it sleeps, and the E2-g-c night-2 read's
RestAt/Waiting 5-8 an hour at hours 22-3 (no shared bed: 0 of 40)
is that lock. Now the snapshot carries each colonist's job anchor
and `queue_ahead` counts a body only when its anchor is mine; THE
QUEUE IGNORES A STRANGER (uid, anchor, my_d, strangers) counts the
holds the old rule would have made. Pin
`the_queue_is_for_the_same_anchor` (the next bed's sleeper does not
queue me; a nearer body bound for my anchor does; no anchor or
nobody: none; a farther one is behind me); planted: the stranger
counted, red. Prediction (b1 fresh, `wait-e2m-b1.sh`, after E2-l's
night-1 read; night 1): RestAt/Waiting starving at hours 22-3 at
most 1 an hour (5-8); strangers ignored at least 20; RestAt/
Traveling starving at 0-3 at most 4 an hour (3-13); EatFrom/Waiting
at 16-20 unchanged in kind; in-bed starving no worse than E2-l's
read (the row puts the waiting sleepers in bed, it does not feed
them). Falsified if RestAt/Waiting persists above 4 an hour, or if
the evening eat queue loses its members (Waiting 0 while Traveling
rises). Rejected: exempting RestAt only; a per-bed lock; a shorter
wait. NOT evidenced: night 2; the climb queue; eaters at the W12-b
pairs' one-cell store. The E2-k chain and reader were re-keyed
behind this row and the dry tree rebuilt in the order W12-b-b,
E2-j-b, E2-l, E2-m, E2-k, E2-i2, W12-a-b, W15-i1.

## E2-j-b, registered 18:33 (keyed on the W12-b stage; ahead of E2-l, E2-k, E2-i2)

THE SPREAD STAYS WITHIN THE STORE. E2-j's order (reservations, then
distance) with no bound sent eaters across town to any unreserved
pile: Traveling at 18-20 tripled while the evening queue cleared.
Now `eat_pick_key(reserved, dist2, nearest_dist2, uid)` = (band,
reserved.min(8), dist2, uid), band 0 when the stack's distance is
within SPREAD_REACH (12 blocks) of the nearest admissible stack's;
`pick_within_store` collects the admissible stacks, bounds by the
nearest, picks by the key. Pin `the_spread_stays_within_the_store`
(reserved and two off beats unreserved and thirty off; beyond reach
the less reserved first; the picker keeps the eater at the near
store and spreads within it; nothing: none) and the E2-j pin
re-stated; planted: the reach unbounded (x1000), red. Prediction
(b2 fresh, `wait-e2jb-b2.sh`, after W12-b's day-1 read; night 1 at
hour 6): Traveling at 18-20 at most 12 an hour (20, 22, 28 / 5, 7,
7); Waiting at 16-20 at most 4 (E2-j 0-2; before 11); Waiting at
21-3 at most 6 (12-19); in-bed starving at 0-3 at most 16 (24);
distinct starving sleepers at most 3 (5 / 2); NIGHT MEAL AT HOME at
least 25 (33 / 14); meals day 1 at least 45. Falsified if Traveling
at 18-20 stays above 16, or the evening Waiting returns above 8, or
NIGHT MEAL AT HOME falls under 20. Rejected: withdrawing E2-j
outright (its evening bar passed and the night meals doubled); a
reach of 30 (across town on this world); reservations ignored
beyond reach. NOT evidenced: night 2; the night Waiting's cause
(E2-i2); b1's evening. The E2-l chain was re-keyed behind this row
and the dry tree rebuilt in the order W12-a, W12-b, E2-j-b, E2-l,
E2-k, E2-i2.

## E2-l, registered 18:16 (keyed on the W12-b stage, the end of the chain, before the binary; ahead of E2-k and E2-i2)

THE WALK HOME IS NOT WORK. On the E2-g-b run (b1, pair 7d28997261)
the eaters' SUPPER CARRIED HOME claims fell at DAY SCHEDULE hours 13
(1), 14 (5) and 15 (3), none at 16-21: ROW 27's door
(`work_claims_open`) refuses every colonist outside the Work block
before the scorer runs, so the Leisure half of every supper window
(E2-g 12-21, E2-g-b 15-21, E2-g-c 14-21) never produced a claim,
and the windows' differences were their Work hours only. Now
`claims_admit(block, own_supper_pending)`: Work always; Leisure when
a supper load bound for the claimant's own shelf is still unclaimed;
Sleep never; through the leisure door every other job is zero.
Witness THE WALK HOME OPENS (colonist, hour, opened). Pin
`the_walk_home_is_not_work`; planted: Leisure always open, red.
Prediction (b1 fresh, `wait-e2l-b1.sh`, after E2-g-c's night-1 read;
night 1 at hour 6): Leisure-hour SUPPER CARRIED HOME claims at least
15 (0 on every pair); THE WALK HOME OPENS at least 20; arrivals to
private shelves at least 40; SUPPER CARRIED HOME at least 30; NIGHT
MEAL AT HOME at least 10; distinct starving sleepers at most 4;
in-bed starving at most 12 an hour at hours 0-3; works at least 420.
Falsified if the Leisure claims stay 0, or works fall under 400, or
colonists claim anything but their own load in Leisure (the reader
counts every leisure-hour claim). Rejected: opening Leisure for all
hauls; a self-job at the whistle; the haulers alone. NOT evidenced:
night 2; the night watch's own Leisure; E2-g-c's 14-15 (its own
read, the pair before). The queue was reordered W12-a, W12-b, E2-l,
E2-k, E2-i2 and the dry tree rebuilt from HEAD in that order.

## E2-i2, registered 18:02 (keyed on the E2-k stage, the end of the chain, before the binary)

THE NIGHT SHELF NAMES ITS HOLDERS. An instrument row: E2-i1's Refused
could not say whose reservation held the stack. `has_capacity(item,
amount)` is reserved_count(item) < amount; the candidates are the
housemate's Waiting EatFrom (E2-j's queue holds its reservation),
a supper haul's whole-stack reservation (u32::MAX) outliving its
arrival, or the sleeper's own pick released without its
reservation. `reservation_holders(jobs, ids)` names the jobs whose
reservation is one of the stack's ids as job:class:claimant; the
witness adds units, reserved and holders (named when the capacity
refused). Pin `the_night_shelf_names_its_holders`; planted: nobody
named, red. Prediction (b1 fresh, `wait-e2i2-b1.sh`, night 1 after
E2-k's read): every Refused line names a holder; holders=[] with
reserved > 0 is a reservation with no job (the leak; the row after
is the release); the class decides the next row (eat by another:
the housemate's Waiting pick on a stack too small for two; haul:
the round's reservation; eat by the sleeper: a stale self-hold).
Falsified as an instrument if a Refused line shows reserved 0.

## E2-k, registered 17:58 (keyed on the W12-b stage, the end of the chain, before the binary)

THE SHELF IS NEVER A BED. `shelf_cell_beside` took the first
standable cell around the first bed, east first; worldgen lays beds
side by side and a bed sprite on an air block over a floor is
standable, so five of b2's nineteen added shelves sit on the second
bed (beds 114, shelves 19, on a bed cell 5, all nineteen on the
bedroom storey), colonist 64's among them. Now the function takes
the house's beds and never picks one; the caller passes
`beds_here`; SHELF ADDED gains beds_in_house. Pin
`the_shelf_is_never_a_bed` (a bed east: west; beds east and west:
north; no beds: east as before); planted: the beds ignored, red.
Prediction (b1 fresh, `wait-e2k-b1.sh`, after E2-g-c's night-1
read; boot +3 and night 1): shelves on a bed cell 0 (b2: 5 of 19;
b1's own baseline is in its previous log); NIGHT SHELF EMPTY Refused
by cap from a house whose shelf was a bed 0; the rest of the night
is E2-g-c's read. Falsified if a shelf still lands on a bed, or if
shelves added falls (a bed on every neighbour cell yields None).
Rejected: the pot's cell as the shelf; a two-cell shelf; the last
bed instead of the first. NOT evidenced: the reservation that
locked colonist 64 out (the housemate's or its own stale one:
E2-i2's witness names it); haulers reaching upstairs shelves.

### E2-j landed (98d548575d, staged 17:36)

Check clean, the pin green, both halves staged 17:36. The falsifier
(the reservations ignored) went RED at 17:39 (`falsify-e2j.out`),
the tree restored clean. The b2 reader (`wait-e2j-b2.sh`:
after E2-i1's night 1, a fresh b2; the evening EatFrom/Waiting per
hour, the eat travel share, then the nights) run from the stage; the
E2-g-c chain keys on it.

## E2-g-c, registered 17:25 (keyed on the E2-j stage, the end of the chain, before the binary)

`SUPPER_WALK_HOME_WORK_HOURS = 2`: the walk home is any of the day's
last two Work hours and the Leisure after them (14-15 and 16-21 on
the default schedule). A stated middle between E2-g's four hours
(works down two fifths) and E2-g-b's one (half the shelves bare,
eight sleepers): measured, not a taste. Pin
`the_eater_carries_supper_on_the_way_home` re-stated (14, 15, 16,
17, 20, 21 yes; 12, 13, 6, 7, 22, 3 no); planted: four hours, red.
Prediction (b1 fresh, `wait-e2gc-b1.sh`, the day-1 line and night 1
at hour 6): shelf arrivals before the sweep at least 40 (25 / 48);
SUPPER CARRIED HOME at least 24 (16 / 32); the lanes' works at least
420 (481 / 305); NIGHT MEAL AT HOME at least 8 (4 / 9); distinct
starving sleepers at most 4 (8 / 3); in-bed starving at most 12 an
hour at 0-3 (43 / 18); the Haul lane's hauls at least 25. Falsified
if the shelves stay under 35 (the window is not the lever) or the
works fall under 400 (the cost returns at two hours); then the carry
needs another shape (a load per eater minted at the store as the
eater passes, without a round). Rejected: three hours (the next step
if two fails the shelves); a per-eater cap; a second round in the
evening. NOT evidenced: night 2; the night watch; whether the
starving sleepers' shelves were bare or refused (E2-i1).

## E2-j, registered 17:13 (keyed on the E2-i1 stage, the end of the chain, before the binary)

The evening queue at one pile: EatFrom/Waiting 11, 11, 7 an hour at
17-19 on b2's two E2-h evenings. `ActiveJobState::Waiting` is the
anchor queue (a farther body waits while a nearer one takes the
steer); the eat pick takes the nearest admissible food by squared
distance with the uid as the tie-break, so the plaza's hungry pick
the same stack, reserve it (a stack of hundreds admits dozens), and
queue at its cell. Mechanism: `eat_pick_key(reserved, dist2, uid)` =
(reserved.min(8), dist2, uid) with reserved = the stack's
reservation count; the pick's min_by_key uses it -- an unreserved
stack ten blocks off beats a reserved one at the feet. Prior art:
RimWorld skips a meal another pawn has reserved; Dwarf Fortress
claims a food item per job. Pin `the_pick_spreads_over_the_stacks`;
planted: the reservations ignored, red. Prediction (b2 fresh,
`wait-e2j-b2.sh`, night 1 at hour 6): EatFrom/Waiting at 16-20 at
most 3 an hour (11); EatFrom/Traveling there not above 12; meals at
least 45; the night unchanged (this row is the evening's). Falsified
if the Waiting stays above 8 (the anchor cell, not the pick) or
Traveling doubles (the spread walks too far). Rejected: a second
anchor cell at the pile; widening the capacity refusal; a random
tie-break. NOT evidenced: the cook-station queue; the night.

## E2-g-b, registered 15:45 (keyed on the W11-b stage, the end of the chain, before the binary)

`supper_walk_home_hour(night_watch, uid, hour)`: the last Work hour
(Work now, not Work next) and the Leisure hours after it (walking
back from a Leisure hour, the nearest non-Leisure hour is Work) --
15 and 16-21 on the default schedule; not the work day's middle, the
morning Leisure or Sleep. The own-supper lift (6) applies only then;
the haulers' errand (5) fills the shelves from noon as E2-e did. Pin
`the_eater_carries_supper_on_the_way_home` (15, 16, 17, 20, 21 yes;
12, 13, 14, 6, 7, 22, 3 no); planted: the walk home from noon, red.
Prediction (b1 fresh, `wait-e2gb-b1.sh`, the day-1 line and night 1
at hour 6): the lanes' works at least 450 (305; E2-e 530); SUPPER
CARRIED HOME at least 15; private_units at midnight at least 25 (39
/ 21); NIGHT MEAL AT HOME at least 10; in-bed starving at most 8 an
hour at 1-3 and sleepers at most 3; the Haul lane's hauls at least
25. Falsified if the works stay under 400 (the loads themselves pull
the lanes: E2-e's errand is the cost, its own row) or SUPPER CARRIED
HOME falls under 5 with shelves still short. Rejected: a per-hour
cap on the eaters (a queue is the work day); a lift only in Leisure
(the last work hour is when the store is on the way). NOT evidenced:
night 2; the night watch's walk home.

### E2-g-b landed (4ded49d334, staged 16:15)

Check clean, the pin green, both halves staged 16:15. The falsifier
(the walk home from noon) and the b1 reader (`wait-e2gb-b1.sh`: after
W11-b's day-1 read, a fresh b1 with the fresh-boot guard; the lanes'
works, SUPPER CARRIED HOME, the round, the shelf arrivals, the sweep,
night meals and the starving at hour 6 of days 1 and 2) run from the
stage; the W12-i2 chain keys on it. The falsifier went red and
restored clean at 16:17; lab-bin carries the pair from 16:15.

After night 1 (b1, hour 6 of day 1, 17:20; the pair 7d28997261 =
W12-i2's, which carries E2-g-b): SUPPER ROUND day=0 hour=12 loads=49;
arrivals to private shelves before the sweep 25, general 13 (E2-g:
48 / 9; E2-e: 21 / 4); SUPPER CARRIED HOME 16 (E2-g: 32); SWEPT 30
(E2-g 14); NIGHT MEAL AT HOME 4 (E2-g 9; bar 10: NOT met);
no_food_found 423 (305); meals 45; YEAR day=1 food_stock 3,961 /
25.8 days; the lanes' works Build 180, Farm 74, Mine 71, Craft 55,
Guard 4 and Cook (cut in the print) -- about 481 with Cook 97 from
the day line (bar 450: MET; E2-g 305); the Haul lane 36 hauls (bar
25: MET); in_bags 71-142 by day. Starving by clock: RestAt/Arrived
7, 18, 35, 43 at hours 0-3 (E2-g: 7, 16, 14, 18 -- bar 8: NOT met,
and worse); distinct starving sleepers 8 (E2-g 3; bar 3: NOT met);
the evening EatFrom/Traveling 2-8 at 15-20. **E2-g-b traded the
shelves for the works**: with the eaters' window cut to one work
hour and the evening, the eaters carried 16 (32) and the haulers 9
(E2-e's lane carried 21 in its four-hour window on this world; 36
hauls a day is the lane's whole capacity, and 49 loads exceed it),
25 shelves were stocked (48), and the night had eight starving
sleepers (3). Neither window is right: four hours took two fifths
of the works, one hour left half the shelves bare. E2-g-c takes the
middle -- the last TWO work hours and the evening -- as a stated
assumption to be measured, not a taste: the bars are shelves at
least 40, works at least 420, sleepers at most 4, and the read will
say whether the middle holds both or the carry needs a different
shape (a load per eater minted at the store on the walk home,
without the round).

### E2-e's second replicate: b2 on the W10-i1 pair (65db6d4cab, which carries E2-e), day-1 line at 13:52

STORAGE SUMMARY day=1 private_units=52 (b1's E2-e run: 21; E2-d: 0
and 12) -- the shelves are stocked at midnight on both worlds. The
town's day line: works 248, hauls 131, haul_share 34%, against b2's
own W10-g day (345 works, 73 hauls, 17%). The errands at priority 5
doubled the day's hauls and took a quarter of its works: a cost the
E2-e bars did not name (its falsifier watched for hauls falling to
zero, not works). E2-g moves the carrying to the eaters at their
shift's end; its bar "the day's works not down by a third" now has
this number to beat, and the works on the W10-i1 pair's day 2 are
the control.

## E2-g, registered 13:45 (keyed on the E2-f stage, the end of the chain, before the binary)

Why only haulers took E2-e's loads: the claim scoring's base is the
claimant's lane weight for the job's work, a miner's or a cook's Haul
weight is zero, and the zero skip (priority_zero) runs BEFORE E2-e's
errand lift. Mechanism: the round records each load's eaters (the
bed owners of its house) in `board.supper_eaters`;
`own_supper_priority(base, own)` = 6 for a load whose eaters include
the claimant, placed before the haul gate (skipped for it), the guard
door (skipped) and the zero skip; SUPPER_ROUND_LOADS_MAX 36 -> 64 so
every short house gets a load; the claim commit counts SUPPER
CARRIED HOME. Prior art: RimWorld's meal for the road, the Sims'
own fridge, Banished families stocking their own house. Pin
`supper_is_carried_home_by_its_eater`; planted: the lift removed,
red. Prediction (b1 fresh, night 1 at hour 6 of day 1,
`wait-e2g-b1.sh`): SUPPER CARRIED HOME at least 25 (0); shelf
arrivals before the sweep at least 40 of the loads (21 of 36); SWEPT
under 10 (18); NIGHT MEAL AT HOME at least 15 (8); in-bed starving
at most 6 an hour at hours 1-3 (14-22) and sleepers at most 2 (5);
the Haul lane's hauls at least 20 (35); food_stock within 200 of
4,052; bags under 400 but for the Guard's night stack (E2-f's).
Falsified if the eaters claim and the shelf arrivals stay under 30
(they claim and do not carry), or if the day's works fall by a third
(the errand pulls the eaters off their trade: the window, not the
rule). Rejected: more haulers (row 48's cap), a bigger load (E2-b's
collapse), a morning round, a dedicated supper hauler. NOT
evidenced: night 2; houses whose eaters are all off-shift at the
window (the night watch), whose loads fall to the haulers' errand.

Prediction for the row that follows, registered now: after it, the
Sleep-block STARVING samples fall below 10% of tonight's (281 -> under
28 RestAt/Arrived over a comparable window) and FED at the day
boundary reads at least 46/50 (baseline 36/50). Falsified if the
sleepers are fed and the day-boundary FED does not move -- then E2-b
(the day trip) is the larger half and the next row is a route row.
