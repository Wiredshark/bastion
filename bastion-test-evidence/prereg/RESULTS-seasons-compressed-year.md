# RESULTS — plant in spring, stock for winter (Y0-Y3): the calendar PASSED a whole year; the economy read is confounded by the walker and re-registered on the fixed pair

Read 2026-09-02 11:51 (spring to the first autumn line) and 13:44 (autumn
and winter) against PREREG-seasons-plant-in-spring-stock-for-winter.md.
Arm b2 on 0a1772ddb8 with BASTION_DAYS_IN_YEAR=8 (an 8-day year: spring
days 0-1, summer 2-3, autumn 4-5, winter 6-7), NoRaids, booted 09:38.

## The calendar (Y0-Y2): PASSED across all four seasons

| day | season | sow jobs done | stage-ups | MATURE | harvested | tills | sows refused (per pass) |
|----:|--------|--------------:|----------:|-------:|----------:|------:|------------------------:|
| 0   | Spring | (boot)        | --        | --     | --        | --    | 0                       |
| 1   | Spring | 25            | 126       | 2      | 4         | --    | 0                       |
| 2   | Summer | 11            | 215       | 13     | 1         | --    | 6,015                   |
| 3   | Summer | 4             | 196       | 15     | 1         | --    | 21,845,775              |
| 4   | Autumn | 2             | 70        | 0      | 0         | 426   | 22,012,995              |
| 5   | Autumn | 1             | 60        | 19     | 14        | 135   | --                      |
| 6   | Winter | 1             | 0         | 0      | 1         | 18    | --                      |
| 7   | Winter | 0             | 0         | 0      | 0         | 0     | --                      |

- The lever held: `days_in_year=8.0`, `stage_secs=24685.71` (8 x 0.5 x
  86,400 / 14); the growth ticker and the census read the same TimeOfDay
  frame.
- Sowing followed the season (25 spring, then the summer tail of jobs
  claimed before the turn, none after). Autumn's 1.5x finished the late
  sowings (MATURE 19 on day 5) and autumn broke ground for spring (tills
  426 and 135, the "autumn tills unbroken ground only" verdict). Winter
  stopped growth entirely: 0 stage-ups on days 6 and 7.
- INSTRUMENT SCALE: `sows_refused_by_season` counted scan passes (21.8
  million on one summer day); Y3c (e860824090) now counts cells per day.

## The winter par (Y3) and the economy: the par rose as designed; the town could not follow it

| day | season | food_stock (drawable) | food_par | days_of_food | roster | starving 09:00 / 21:00 |
|----:|--------|----------------------:|---------:|-------------:|-------:|------------------------|
| 3   | Summer | 413                   | 200      | 2.58         | 51     | 7 / 20                 |
| 4   | Autumn | 418                   | 1,176    | 2.56         | 53     | 16 / 24                |
| 5   | Autumn | 190                   | 1,052    | 1.12         | 55     | 19 / --                |
| 6   | Winter | 211                   | 899      | 1.22         | 56     | -- / 43 (03:00)        |
| 7   | Winter | 66                    | 753      | 0.37         | 59     | --                     |

- The par did what it was built to do: from the first autumn line it
  asked for the days until the next harvest window (1,176 units for 53
  people, falling as winter passed). Trade and forage demand follow it;
  nothing in this town could answer it. The stock fell 418 -> 66 through
  autumn and winter while starving rose to 43 of 59 at night.
- The eat leak dominated the summer (313-360 eat jobs a day for 71-79
  meals with ~400 units in stores; see RESULTS-wedge-probe.md); this
  pair predates every walker fix. The pre-registered economy bar
  ("days_of_food >= remaining winter days on every winter day") is NOT
  READ on this arm: the numerator was food the walkers could not draw,
  and the roster kept growing through the famine (49 -> 59; the settler
  gate read beds only -- Ben's ruling at 13:40: a famine closes the gate,
  row F1).

## Disposition

Y0-Y2 PASSED for a whole compressed year (sow in spring, mature through
summer and autumn, break ground in autumn, nothing in winter). Y3's par
PASSED as a signal and is NOT evidenced as an outcome: the town never
stocked because it could not reach or grow enough. The economy read is
re-registered on the compressed-year re-run that boots from a pair
carrying the walker rows (W1, W5, W2c) and, later, F1; if that run still
cannot hold a summer, the judgement goes to Ben with plain numbers
(units per harvest, units eaten per day, fields needed). NOT evidenced:
the winter par's effect on trade.
