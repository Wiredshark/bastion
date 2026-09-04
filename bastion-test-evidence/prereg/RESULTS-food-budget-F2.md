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

F2c' replaces the three: every ripe roster crop in an adopted field is
the founding harvest at adoption (Ben's ruling, literally), the cell
restarts as the town's own with a clock, the yield becomes 4 on the
measured 8,424 cells (33,696 >= 30,720), and the granary is dropped
because the founding harvest is the barn. The day-1 and day-7 reads
of the F2 pair still stand as the seeds' and the eat path's test; the
budget bars move to the F2c' pair.
