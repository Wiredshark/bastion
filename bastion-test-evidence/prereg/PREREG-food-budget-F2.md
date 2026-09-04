# PREREG — F2: the town feeds itself (the food budget)

Written 2026-09-04 16:40, from the 42-hour unattended run of both flat
arms (b1 on the G1c-d pair fce20cf9b9 to game day 68; b2, the eight-day
year, on the W7b pair 0a224cdb3b to day 53).

## The defect, as measured

| b1 (160-day year), day | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 68 |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| food stock (units)     | 573 | 456 | 301 | 171 | 84 | 74 | 52 | 16 | 5 | 2 |
| meals                  | 0 | 49 | 100 | 101 | 100 | 107 | 74 | 74 | 60 | 30 |
| crops matured          | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | ~100 total by day 57 |
| starving at the feed line | 0 | 0 | 0 | 0 | 4 | 16 | 25 | 33 | 45 | 38-45 of 46 |

The year census at day 67: `season=Summer days_in_year=160
stage_secs=493,714 food_stock=2 food_par=184 days_of_food=0.014
sows_refused_by_season=6002`. At day 0 the same census asked for
`food_par=9,773` -- the design's own statement that 64 days of food
(48 heads x 3.2 units x the days to the harvest window) must be in
store at founding. The store held 573, of which 256 were the
`BASTION_SEED_FOOD` fixture and 192 the four rations each colonist is
issued. Founding seeds: 8 (`FOUNDING_SEED_STOCK`) for 4,320 field
cells; the first-day lived-in sows: 49; founding harvest: 6 cells, 12
units. b2 on the eight-day year fed 0 of 70 from day 12 to day 53 with
`food_par=1,389 food_stock=0`.

The budget, per year, for the founding town: consumption 48 x 3.2 x 160
= 24,576 units; production if every field cell were sown once in spring
and harvested once: 4,320 x 2 = 8,640. The fields as they are feed a
third of the town in the best case and, seed-locked at 8 seeds with an
80-day doubling, a few percent in practice. Both arms and every play
mode (PLAY.ps1 sets the seed-food lever in all of them) starve on
schedule.

## Rulings this row stands on

- Grow cycle seasonal: plant in spring, harvest in summer, stock for
  winter (Ben, 2026-09-02).
- Food for winter: production must meet the winter par; foraging,
  hunting and trade later; farmers harvest the founding crops at once
  (Ben, 2026-09-02).
- A famine closes the settler gate (Ben; landed as F1).

## The change (four parts, each with its identity switch)

1. **The founding granary** (F2a): at adoption the general store
   receives the winter par in the town's own grain
   (`seasonal_food_par(roster, days_in_year, tod)` units of
   `FARM_WHEAT_ITEM`), through the deferred delivery queue, once.
   Witness `bastion: FOUNDING GRANARY`. The `BASTION_SEED_FOOD` lever
   stays a fixture on top. `BASTION_NO_FOUNDING_GRANARY` restores
   today's four rations.
2. **The founding seed store** (F2b): `FOUNDING_SEED_STOCK` becomes one
   seed per adopted field cell (the fields' area), delivered the same
   way. Witness `bastion: FOUNDING SEEDS` (cells, seeds).
3. **Planted fields** (F2c): every bare cell of an adopted field is
   planted at adoption with the colony crop at a stage drawn by the
   existing lived-in hash (`adopted_sow_stage`), so the fields ripen
   continuously from day 1 and the founding harvest (H0) takes the
   ripe fifteenth at once. Witness `bastion: FOUNDING FIELDS PLANTED`
   (fields, cells planted, cells already cropped, ripe).
4. **The yield** (F2d): `FARM_WHEAT_YIELD` 2 -> 8, pinned by
   `a_years_harvest_feeds_the_founding_town`: 4,320 cells x yield >=
   48 x 3.2 x 160 x 1.25 (the flat town's measured numbers, written
   as literals in the pin with their provenance).

And the instrument (F2i): the daily YEAR CENSUS line gains
`harvested_today`, `cooked_today`, `store_food`, `store_other`,
`seeds_in_store`, `field_cells`, `field_planted`, `field_ripe` so the
chain can be read at one line a day.

## Predictions (b1, fresh boot on the F2 pair, lever on as before)

| measure | today (G1c-d pair) | prediction | bar |
|---|---|---|---|
| food stock at day 1 / day 7 | 456 / 16 | >= 9,000 / >= 8,000 | >= the par minus seven days of eating |
| starving at the feed line, day 7 | 33 | <= 3 | <= 3 |
| crops matured by day 7 | 2 | >= 200 | >= 100 (4,320 / 14 stages per 5.7 days, x 1 interval) |
| harvested units by day 7 | 4 | >= 800 | >= 400 |
| settler gate | closed from day 5 | open through day 7 | open |
| meals per day at day 7 | 74 | ~100 | >= 90 |
| panics | 0 | 0 | 0 |

## What would refute it

- Stock at day 1 under 9,000: the granary did not land (delivery cap,
  store cells, or the queue drained elsewhere) -- read FOUNDING GRANARY
  and the DELIVERED lines.
- Matured crops by day 7 under 100 with fields planted: the stagger did
  not apply, or the stage clock is not the day clock -- read FOUNDING
  FIELDS PLANTED and the first MATURE lines' days.
- Starving at day 7 above 3 with stock above 8,000: the eaters cannot
  reach or see the food -- an EatFrom problem, not a budget one (read
  the EAT CENSUS skip reasons).

## Not evidenced by this row

The full year (winter at days 120-160) needs a 60-hour arm run; the
eight-day-year arm (b2) is the proxy: the same predictions at its
day 7 are read at its day 1 (one cycle) and the winter at its days 6-8.
