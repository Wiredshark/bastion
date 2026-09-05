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
