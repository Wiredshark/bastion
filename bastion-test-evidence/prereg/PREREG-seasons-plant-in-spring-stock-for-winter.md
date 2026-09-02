# PREREG — plant in spring, harvest by autumn, stock for winter (Ben, 2026-09-02)

Ben's ruling (live, 09:05): "grow cycle should be realistic: plant in
spring, harvest in summer, fall stock for winter." Supersedes the 4-game-
day cycle (a number of taste from row 45).

## What the engine has (read 2026-09-02 09:20)

- A year: `SeasonConfig.days_in_year` = 160 (an asset, `common.season_config`,
  hot-reloadable), four equal seasons by year phase (`Season::at`).
- Growth: `season_stage_factor` Spring 1.0, Summer 0.75, Autumn 1.5, Winter
  None (no growth); `FARM_STAGE_SECS = FARM_CYCLE_DAYS(4) * DAY / 14` -- the
  stage length ignores the year, so a crop sown any day matures in ~4 days.
- Sowing: `seasonal_till_verdict` re-tills a column once per SEASON (Sow when
  the column's recorded season is the current one) -- a crop can be sown in
  any season, autumn and winter included.
- Stock: `food_par_for(roster) = roster * 4` units, the same every day; the
  par drives trade and forage demand. No winter term.
- Economics: hunger decays 0.000889/s = 1.6 per game day = 3.2 raw meals
  (0.5 each) per colonist per day; a harvested cell yields 2 food + 2 seed.
  A 40-day winter for 50 colonists needs ~6,400 raw units; eight 24x24
  fields fully sown yield ~9,000 per cycle. Feasible on paper; the read
  says whether the town can actually gather and hold it.

## Mechanisms (pure; the year is the unit)

- Y0 (fixture lever): `BASTION_DAYS_IN_YEAR` overrides the asset's year
  length in `SeasonConfig::current()` so the flat arms run a compressed
  year (8 days = 2 per season, ~90 min a season under load). Identity
  when unset; no balance number changes.
- Y1 STAGE LENGTH FROM THE YEAR: `farm_stage_secs(days_in_year) =
  days_in_year * GROW_SEASON_FRACTION(0.5) * DAY / 14` -- a spring sowing
  reaches maturity across spring and summer; autumn's 1.5x finishes late
  sowings; winter stops. Witness: the day line gains season and stage
  length.
- Y2 SOW IN SPRING: the till verdict takes the season; Sow / SowGrace only
  in Spring; in Summer an unsown tilled cell waits; in Autumn and Winter
  nothing is tilled or sown (harvest and hauling only). Witness "SOW
  REFUSED — not spring" (counted daily).
- Y3 STOCK FOR WINTER: `seasonal_food_par(roster, season, day_of_season,
  days_in_year)` -- from midsummer the par rises to roster x 3.2 x (days
  until the next spring harvest window), so trade and forage pull food in
  through autumn; a daily YEAR CENSUS line: season, day_of_season,
  food_stock, food_par, days_of_food (= stock / (roster x 3.2)).
- Adoption realism (later): the founding larder scaled by the season of
  adoption (a town adopted in autumn has a winter store).

Prior art named: Banished (seasonal planting; food stock must last the
winter; the town starves in year 2 when it does not), Stardew / Harvest
Moon (crop seasons), Manor Lords (sow in spring, harvest in autumn, the
granary's winter), Dwarf Fortress (seasonal farming, above-ground crops by
season).

## Pre-registered pass / fail (flat arm, BASTION_DAYS_IN_YEAR=8, one full year)

- Y2 PASS: sows only on spring days (SOW REFUSED > 0 in summer/autumn,
  0 sows in autumn/winter). FAIL: any sow outside spring.
- Y1 PASS: first MATURE of spring-sown cells in summer, the peak of
  MATURE lines in early autumn, none in winter. FAIL: MATURE before
  midsummer (stage too short) or none by autumn's end (too long).
- Y3 PASS: YEAR CENSUS days_of_food >= remaining winter days on every
  winter day; starving at dawn in winter <= the summer level (1-3 of
  50); the par visibly rises from midsummer. FAIL: days_of_food < 1 in
  winter -> the economy (yield 2/cell, field count) is the number, and
  it goes to Ben as a design judgement, not a tweak.
- Falsifier of the frame: if the compressed year (2 days a season) makes
  a stage shorter than a work day, the arm cannot separate "sown in
  spring" from "matured in spring"; then the year on the arm is 16 days
  and the read takes a night.

## Ben's own play

At the default 160-day year a spring sowing matures over ~80 game days
(~40 real hours). The adopted town's lived-in fields (row A1) carry the
first year; the winter store is what he will watch for.
