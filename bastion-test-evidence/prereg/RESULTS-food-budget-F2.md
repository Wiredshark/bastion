# RESULTS — F2: the town feeds itself

Read against PREREG-food-budget-F2.md. The defect evidence is the 42-hour
unattended run of 2026-09-02 22:00 to 2026-09-04 16:00; the census
extracts of both logs are kept beside the scratchpad
(`b1-logs/b1-g1cd-fce20cf9b9-69days-census.txt`,
`b2-logs/b2-w7b-0a224cdb3b-53days-census.txt`).

## The defect (b1, the 160-day year, pair fce20cf9b9, house lever on)

| day | food stock | meals | crops matured | starving at the feed line | roster |
|----:|-----------:|------:|--------------:|--------------------------:|-------:|
| 0   | 573 (256 fixture + 4 rations x 48 + 6 founding cells) | 0 | 0 | 0 | 49 |
| 1   | 456 | 49  | 0 | 0  | 50 |
| 2   | 301 | 100 | 0 | 0  | 51 |
| 3   | 171 | 101 | 0 | 0  | 51 |
| 4   | 84  | 100 | 0 | 4  | 52 |
| 5   | 74  | 107 | 0 | 16 | 52 |
| 6   | 52  | 74  | 2 | 25 | 53 |
| 7   | 16  | 74  | 0 | 33 | 54 |
| 8   | 5   | 60  | 0 | 45 | 54 |
| 68  | 2   | 30  | ~100 in all | 38-45 of 46 | 46 (19 dead) |

Year census, day 0: `season=Spring food_stock=64 food_par=9773` (the
par is 48 x 3.2 x the 64 days to the harvest window); day 67:
`season=Summer stage_secs=493714 food_stock=2 food_par=184
sows_refused_by_season=6002`. Founding seeds 8 for 4,320 field cells
(eight adopted fields of 30 x 18); first-day lived-in sows 49; the
founding harvest 6 cells of carrot, 12 units. The settler gate closed
from day 5 (F1 working as ruled). The eat census at day 66-68:
`no_food_found=0` but 130 `EAT RE-TARGET found NO remaining food`
lines in five days -- the town had nothing to eat and the eaters said
so.

## The defect (b2, the eight-day year, pair 0a224cdb3b)

Fed 0 of 70 at every feed line from day 12 to day 53; year census day
53: `season=Autumn day_of_year=5 food_stock=0 food_par=1389`; eat
census day 52-53: `eat_minted=0 meals=0 preempt_cooldown_active=10783`
-- nobody even tried, every hungry colonist in cooldown after finding
nothing. Harvests 350 and matured 450 by day 30 (the four-day cycle
works), then 0 and 0 in days 48-53: two seasons of no growing and no
stock to bridge them.

## The budget

Eaten per year, the founding town: 48 x 3.2 x 160 = 24,576 units.
Grown per year at the old yield with every cell sown once: 4,320 x 2 =
8,640. Seed-locked at 8 seeds with the doubling tied to an 80-day
cycle, the fields reached a few percent of even that. No staffing,
pathing or lane change could close a gap of that size; the numbers
had to.

## The first minute of the F2 pair (1e9b2604ad, both arms, 17:00-17:05)

The pins were falsified at the commit first: yield 2, the seed floor
and a SOWN-only draw each turned their pin red.

| witness | b1 (160-day year) | b2 (8-day year) |
|---|---|---|
| FOUNDING SEEDS, per field | 576, 540, 576, 864, 864, 2,916, 324, 1,764 = 8,424 | the same |
| FOUNDING FIELDS PLANTED | 8 fields, cells 8,424, planted 0, already_cropped 8,411, ripe 0 | the same |
| FOUNDING GRANARY | roster=20 units=4,072 | roster=24 units=217 |
| crops: | Tomato, Lettuce, Carrot, WheatYellow x3, Carrot x2 (worldgen) | Carrot, Tomato, Lettuce, WheatYellow x3, Carrot x2 |
| store at the day-0 census (b2, after delivery) | -- | store_units 8,809, store_seeds 8,432, food_stock 281 |

Three corrections, read off the instrument the row installed:

1. **The fields were never bare.** Worldgen fills an adopted field with
   ripe crops; F2c's planting found 8,411 of 8,424 cells already
   cropped and planted nothing. The 49 first-day sows of the old run
   were the thirteen bare cells and the churn.
2. **Five of eight fields could never be harvested.** The harvest
   trigger mints a job only for wheat or for a CLOCKED crop
   (`crop_is_colony_managed`); tomato, carrot and lettuce placed by
   worldgen carry no clock, so those fields stood ripe for 69 days.
   The three wheat fields were harvested at `VOLUNTEER_YIELD` (2),
   never at the new yield -- F2d changed nothing a founding crop paid.
3. **The granary fired early.** It waited for "a live roster", which
   at the first field's drain was 20 of 48 colonists (24 on b2); the
   par it delivered was for them.

The fields' true area is 8,424 cells, not the 4,320 the pin's literal
carried (eight fields of 540 was a guess from one field's aabr).

### Day 1 on the F2 pair

| day 1 | b1 (160-day year) | b2 (8-day year) | PREREG bar |
|---|---|---|---|
| food_stock / par | 4,662 / 9,879 (29.7 days) | 880 / 345 (5.6 days) | b1 >= 9,000: FAIL (the granary on 20 heads) |
| starving at the feed lines | 0 (fed 36-41 of 49) | 0-1 | <= 3 on day 7: pending, on course |
| meals / cooked | 46 / 81 | 43 / 70 | -- |
| matured / harvested | 0 / 0 | 9 / 0 | -- |
| store units / seeds | 13,214 / 8,315 | 9,480 / 8,344 | -- |
| field_planted (clocked cells) | 144 | 125 | the farmers' own day-0 sows |
| house (b1, lever on) | placed 0, remaining 1,909, builders 0 | -- | see G1c-e-a: 98 plot cells swept unclaimed |
| panics | 0 | 0 | 0 PASS |

F2c' replaces the three: every ripe roster crop in an adopted field is
the founding harvest at adoption (Ben's ruling, literally), the cell
restarts as the town's own with a clock, the yield becomes 4 on the
measured 8,424 cells (33,696 >= 30,720), and the granary is dropped
because the founding harvest is the barn. The day-1 and day-7 reads
of the F2 pair still stand as the seeds' and the eat path's test; the
budget bars move to the F2c' pair.

## The first minute of the F2c' pair (a55321b958, both arms, 17:45-17:48)

Falsified at the commit first: yield 4 -> 2, the seed floor and the
SOWN-only draw each turned their pin red.

| field (b1) | cells | already_cropped | ripe | harvest units | crop |
|---|---:|---:|---:|---:|---|
| 7612,6468 | 576 | 517 | 59 | 236 | Lettuce |
| 7636,6438 | 540 | 488 | 52 | 208 | Tomato |
| 7672,6318 | 576 | 253 | 35 | 140 | Carrot |
| 7702,6264 | 864 | 857 | 0 | 0 | WheatYellow |
| 7732,6252 | 2,916 | 2,916 | 0 | 0 | WheatYellow |
| 7780,6306 | 324 | 293 | 31 | 124 | Carrot |
| (two more) | 1,764 + 864 | -- | 215 + 0 | 860 + 0 | Carrot, WheatYellow |

Planted 0 on every field, both arms. The founding harvest on b1 came to
about 1,570 units, not the 33,696 the budget predicted; the day-0
census at +3 min read `food_stock=1632 days_of_food=10.6
field_planted=392` (the farm pass's own lived-in sows, not the founding
drain).

The producer, read after the fact (`world/src/site/plot/farm_field.rs`):

- crops are placed only on trench stripes (`row_spacing`: wheat 6 wide
  x 0.8 covered, tomato 3 x 1/3, roots 6 x 0.75), never on a bank
  column, and at a per-cell weight of 1/6 (wheat: 4 Empty, 1
  WheatGreen, 1 WheatYellow) to 2/5 (tomato) -- a worldgen field is
  roughly nine-tenths bare;
- every worldgen crop starts at Growth 15 (the attribute's default in
  `to_initial_bytes`), so "ripe" is right where a crop exists;
- `Block::get_sprite()` returns `Some(SpriteKind::Empty)` for plain air,
  so the drain's "already cropped" branch swallowed every bare cell and
  the planting branch below it never ran. The F2 read's "the fields
  were never bare (8,411 of 8,424 already cropped)" was this miscount,
  not a fact about the fields.

Two conclusions. The founding harvest cannot be the year's food on a
worldgen field; the bare cells must be planted at founding, which F2c
intended and this miscount blocked (F2c'-b: the Empty sprite is a bare
cell; pin `an_empty_sprite_is_a_bare_founding_cell`). And the bridge
from founding to the first ripe stage of the lived-in draw is a design
question with numbers, put to Ben: a founding larder sized on the full
roster and the days to the first harvest, or denser founding fields.

### Day 1 on the F2c' pair

| day 1 | b1 (160-day year) | b2 (8-day year) | PREREG bar |
|---|---|---|---|
| food_stock / par | 2,162 / 9,879 (13.8 days) | 2,128 / 345 (13.6 days) | b1 >= 9,000: FAIL (the founding harvest is ~1,600; see the producer read) |
| starving at the midday feed line | 0 (fed 40 of 49) | 1 (fed 38 of 49) | <= 3 on day 7: pending |
| meals / eat_minted | 48 / 41 | 45 / 44 | -- |
| targets_shunned | 18 | 15 | the eat path's stall count, not the budget's |
| settler gate | open (roster 48 -> 49) | open (48 -> 49) | -- |
| house (b1, lever on) | placed 0, builders 0, drafted 0 | -- | the G1c-e read |
| panics | 0 | 0 | 0 PASS |

The stock rose from 1,632 at +3 min to 2,162 at day 1: the lived-in
draw's ripe share and the kitchen, not the founding harvest. The F2c'-b
boot (bare cells planted at founding) and Ben's bridge ruling decide
the day-7 bar.

## The first minute of the F2c'-b pair (a56ecb2822, both arms, 19:29)

Falsified at the commit first: `None | Some(Empty) => Bare` reduced to
`None => Bare` turned `an_empty_sprite_is_a_bare_founding_cell` red at
`bastion_jobs.rs:52256`.

| field (b1) | cells | planted | foreign | ripe | harvest units | crop |
|---|---:|---:|---:|---:|---:|---|
| 7636,6438 | 540 | 356 | 125 | 73 | 236 | Tomato |
| 7612,6468 | 576 | 410 | 98 | 97 | 272 | Lettuce |
| 7672,6318 | 576 | 405 | 109 | 88 | 224 | Carrot |
| 7702,6264 | 864 | 610 | 247 | 43 | 0 | WheatYellow (plan) |
| 7732,6252 | 2,916 | 2,263 | 653 | 154 | 0 | WheatYellow (plan) |
| 7780,6306 | 324 | 215 | 76 | 50 | 132 | Carrot |
| (1,764) | 1,764 | 1,329 | 226 | 297 | 836 | Carrot |
| 7684,6480 | 864 | 432 | 432 | 24 | 0 | WheatYellow (plan) |
| **sum** | 8,424 | **6,020** | 1,966 | 826 | 1,700 | |

Against the F2c'-b bars: planted 6,020 >= 6,000 PASS; day-0 census at
+3 min `food_stock=3368 days_of_food=21.9 field_planted=6445` on b1
(bar >= 3,000 PASS), `food_stock=3236` on b2; `growing=0` everywhere
(every worldgen crop starts at Growth 15, as the producer read said).

Two things the instrument added. `foreign` is 1,966 cells, not "the
hundreds": the three fields the plan calls WheatYellow carry no roster
crop at all (their cropped cells are all foreign and their harvest is
0 on both pairs) -- worldgen grew flax, corn or flowers there, the
colony's plan names them wheat, and the bare cells now carry wheat.
Those 1,966 foreign cells are dead to the colony until a farmer clears
them (a later row). And `ripe` 826 = the worldgen ripe (~425, the 1,700
units) plus the lived-in draw's own ripe share (~400 cells, delivered
per item: the 56 tomato, 116 lettuce, 128 carrot, 172 + 616 + 96
wheat lines).

b2, the same minute, as the replicate: planted 6,080, foreign 1,930,
ripe 793, harvest units 1,604, day-0 food_stock 3,236.

F2c'-c follows from the foreign column: a non-roster sprite whose rtsim
resource is a plant, flower, grass, vegetable or fruit is CLEARABLE at
founding and planted like a bare cell; a scarecrow or a fence stays.
Registered before its binary: planted ~7,900 of 8,424 on both arms,
foreign in the tens, harvest units unchanged, day-0 stock up by the
extra cells' ripe share (~500). Falsified below 7,000 planted.

#### The first minute of the F2c'-c pair (5b111503a0, b2, 21:32)

| field | cells | planted | cleared | foreign | ripe | harvest units | crop |
|---|---:|---:|---:|---:|---:|---:|---|
| 7612,6468 | 576 | 430 | 13 | 85 | 88 | 244 | Lettuce |
| 7636,6438 | 540 | 388 | 29 | 97 | 73 | 220 | Tomato |
| 7672,6318 | 576 | 438 | 23 | 86 | 79 | 184 | Carrot |
| 7702,6264 | 864 | 747 | 291 | 117 | 43 | 0 | Wheat (plan) |
| 7684,6480 | 864 | 748 | 127 | 109 | 59 | 0 | Wheat (plan) |
| 7732,6252 | 2,916 | 2,695 | 391 | 221 | 181 | 0 | Wheat (plan) |
| 7780,6306 | 324 | 212 | 3 | 73 | 52 | 156 | Carrot |
| (1,764) | 1,764 | 1,405 | 57 | 168 | 289 | 764 | Carrot |
| **sum** | 8,424 | **7,063** | 934 | 956 | 864 | 1,568 | |

Planted 7,063 >= 7,000: PASS, at the bar's edge rather than the 7,900
predicted, because `foreign` did not fall to the tens: 956 cells stay
foreign, 85-117 per 24 x 24 field, which is the fence line (a 24-cell
square has 92 perimeter cells) plus the scarecrows -- correctly kept,
wrongly predicted. The wheat that the big fields now carry shows in
the deliveries (724 wheat units from the 2,916-cell field's ripe
share). Day-0 store 12,048 units; day-0 census at +3 min
`food_stock=3520 days_of_food=22.9 field_planted=7038` (the F2c'-b pair
read 3,236 on this arm: +284, against the ~500 predicted -- the
cleared cells' ripe share, less than the bare cells' because the wheat
fields' draw skews young). Falsified at the commit: plant resources
made Foreign turned `a_worldgen_crop_the_colony_cannot_use_is_cleared_at_founding`
red at `bastion_jobs.rs:52382`.

Day 1 on this pair (b2): `food_stock=4105 days_of_food=26.2
matured_today=888 cooked_today=74 field_planted=7083 meals=44
targets_shunned=29` -- the stock rose 585 over the first day, the best
first day of any pair (the F2c'-b pair rose 377 on b1 and 403 on b2).

### Day 1 on the F2c'-b pair

b1: `food_stock=3745 days_of_food=23.9 cooked_today=90 meals=49
targets_shunned=12 field_planted=6445`; the stock rose 377 units over
the day (the planted cells' ripening share outran 49 eaters). The
day-1 budget bar (>= 9,000) stays failed by design pending Ben's bridge
ruling; the day-7 starving bar is read on this pair (b1 keeps it to day
7; b2 moves to the F2c'-c pair when it stages).

A hypothesis raised and refuted the same evening: that the dense
planting (tomato 1.65 and carrot 0.18 are `is_solid` to the
pathfinder) walls the fields against the farmers. b2's day-1 work
hours completed 152 harvests with ~2,500 cells matured (the F2c' pair
had 17 by its day 2 with 41 matured); the night before, with 50 jobs
minted and 2 claimed, was the night. What the read did establish is
the harvest capacity: about 15 cells per farmer-day (a harvest is
travel-priced like a build cell; 6 open jobs per plot, 8 plots). On
the eight-day year maturity outruns the harvest; on the 160-day year
ripening spreads across the season.

### Day 2 on the F2c'-b pair: the harvest never reaches the store

| day 2 | b1 (160-day year) | b2 (8-day year) |
|---|---|---|
| food_stock (day 1 -> 2) | 3,745 -> 3,698 | 3,639 -> 3,565 |
| harvested_today (units) | 0 | 604 |
| matured_today | 0 | 1,189 |
| cooked_today | 40 (day 1: 90) | 95 |
| meals / targets_shunned | 106 / 110 | 109 / 26 |
| starving (evening line) | 8 of 50 | 3 of 51 |
| store_units (day 1 -> 2) | -- | 12,423 -> 12,386 |

b2 harvested 604 units and its store did not move. The harvest
completion emits one ground drop per unit at the cell (four per wheat
cell, scattered); the haul generator read `seen_pickups=403 admitted=0
pending=202 cap=204`; eight haulers completed ~136 hauls a day, mostly
curry and stone; four carrots reached the store all boot. F3 (the
harvest rides in the farmer's basket: the yield into the bag, a
deposit run at a basket of twelve or at shift end) is registered
against this: crop deposits track the harvest within a day, the store
rises by roughly the harvest.

b1's collapse is a different mechanism: the cook trap at the store
approach (W8-f) shunned the store cells 110 times in a day and halved
the kitchen; the eight starving at the evening line are its tail.

b1 day 3: `food_stock=3520 cooked_today=63 meals=113 targets_shunned=65`,
starving 2 at the dawn census -- the kitchen recovers partly day to
day and the trap keeps shunning at 65-110 a day. W8-f's bar on the next
boot: shunned under 20 a day, cooked back at 80 or more.

b1 day 4: `food_stock=3319 days_of_food=19.9 cooked_today=72 meals=121
targets_shunned=47 matured_today=0`, roster 53. The 160-day arm drains
about 200 a day and has matured nothing since the founding draw: a
stage on this clock is 6.6 game days, so the draw's next ripening wave
is due around day 7. That is the bridge Ben's ruling decides; the
day-7 read on this pair is the evidence.

## A caution on every `food_stock` above: the count has two frames

b1 on the W8-f pair (b4a1eb9aa6), tick 54000: the FOOD-WIPE
discriminator read `in_stockpile=3613 on_ground_total=3616` and the
YEAR CENSUS of the same tick read `food_stock=1508` (the trade mint read
1531 the same minute); at day 0 all three read 3624. b2 on the same
pair reads the other way (census 4,202 against 3,621). Both sites call
`colony_food_stock` over the same storages with the same board. The
settler gate and the famine logic key on the census's number, and the
"drains about 200 a day" reads above are that number. F-i2 adds
`food_locked` and `food_anywhere` to the census from the same join so
the next day line names which frame is wrong; until then the daily
`food_stock` values in this file are consistent with themselves and
not yet trustworthy against the discriminator.

### b1 (160-day year) on the W8-f pair, days 1-3 (read 00:20)

| day | food_stock | days_of_food (roster 51) | harvested_today | cooked_today | store_units |
|---|---|---|---|---|---|
| 1 | 1,508 | 9.6 | 0 | 89 | 10,247 |
| 2 | 3,136 | 19.6 | 0 | 46 | 11,946 |
| 3 | 2,726 | 16.7 | 0 | 56 | 11,566 |

At day 3 the discriminator agrees with the census (`in_stockpile=2726
on_ground_total=2732`) and reads `in_bags=1279`: over a thousand food
units are riding in bags at the day line, which neither the census nor
the discriminator's stockpile count sees, and which is the size of the
day-1 gap. The census's frame at a day boundary is the stockpile at
that instant; what the haulers and cooks are carrying is off its books.
Two day-0 reads this night (the F3 boot of b2: census 1,132 against a
discriminator of 3,636 seconds later) were the same thing at the
founding: the census printed while the deliveries were still landing.
Rule for this file: a day-0 line is never a stock; a day line is the
stockpile, not the town's food.

No harvest since the founding on the 160-day year (the fields are
weeks from ripe); at 3.2 raw units a head the town eats ~163 a day,
and the founding larder carries it to about day 20. That is the bridge
judgement still open with Ben (larder on the full roster, denser
fields, or both).

### F3 (48fd9c1390, the harvest rides in the basket) -- b2 day 1, 00:33

Falsified at the commit (a basket of one -> the pin red, 00:05). Day 1
on b2 (8-day year, boot 00:02):

| read | value |
|---|---|
| harvested (cell returns) | 9 cells, `basketed=4` on every line, emit_drop fallbacks 0 |
| harvested_today / matured_today | 36 / 938 (the 938 ripen for day 2) |
| food_stock / discriminator | 4,118 / 4,117 (in_bags 46) |
| store_units day 0 -> day 1 | 12,164 delivered -> 12,926 |
| cooked / meals / targets_shunned | 85 / 49 / 11 |
| starving at the last census | 1 (uid 25, RestAt) |
| SETTLER GATE CLOSED / panics | 0 / 0 |

The basket holds at nine harvests; the scale test is day 2, when the
938 matured cells are cut (the pre-F3 boots dropped ~600 units a day on
the ground there). The day-0 census line of this boot (food 1,132,
store 2,928) was the mid-delivery snapshot described above and is not
a stock.

Day 2 (read from the saved log after the W9 restart, 01:20):

| read | day 1 | day 2 |
|---|---|---|
| harvests (cell returns) / units basketed / ground drops | 9 / 36 / 0 | 161 / 644 / 0 |
| harvested_today / matured_today | 36 / 938 | 608 / 1,373 |
| food_stock / discriminator in_stockpile / on_ground_total | 4,118 / 4,117 / 4,124 | 4,563 / 4,563 / 4,577 |
| store_units | 12,926 | 13,506 |
| cooked / meals / targets_shunned / no_food_found | 85 / 49 / 11 / 0 | 107 / 110 / 20 / 0 |
| starving (last census) / STORE WOULD CLOSE / panics | 1 / -- / 0 | 1 / 0 / 0 |

Disposition: F3 PASSES. Every harvested unit rode in a basket (644 of
644, zero fallbacks to the ground), the store rose 580 on the day, and
the town's food outside its stores is 14 units against ~600 a day of
litter before the row. The eaters' chase onto the terraces has nothing
left to chase.

### T1, the trade mission (registered 04:15; queued at the end of the night's chain)

On the 160-day arms the food par (9,879) sits above the founding
larder, so ITEM 29 mints trade missions -- and the price book's
nearest priced site to the store is the colony's OWN site: b3's R3
boot minted two missions to (7696,6288,182), the spawn; job 568's
walker, carrying 11 wood, stood at 7.99 blocks from that cell (inside
a structure) and printed 3,965 `arrived at job site, working (B5)`
lines in eight minutes until a need preempt released it; job 129
printed 1,717 and completed one exchange. `fix-t1.py`: the book drops
any site inside the colony's settlement bounds
(`price_book_admits`, pinned in the server crate); the mission's
target is the first standable cell in ring order within eight blocks
of the site (`trade_mission_pos`, pinned; the mint witness carries
`mission_stand`); the arrival line prints once per job. Bars on b1:
the book reads one site fewer (11 -> 10), no mission to the spawn,
arrivals per mission under 5, exchanges per mission minted. Falsified
if missions still mint to (7696,6288) or arrivals per mission stay in
the hundreds.
