# RESULTS — plant in spring, stock for winter (Y0-Y3), first read: the calendar works, the economy read is confounded by the walker

Read 2026-09-02 11:51 against PREREG-seasons-plant-in-spring-stock-for-
winter.md. Arm b2 on 0a1772ddb8 with BASTION_DAYS_IN_YEAR=8 (an 8-day
year: spring days 0-1, summer 2-3, autumn 4-5, winter 6-7), NoRaids,
booted 09:38; this read covers spring through the first autumn line.

## The calendar (Y0-Y2): confirmed

| day | season | sow jobs done | lived-in sows | stage-ups | MATURE | harvested | sows refused (per pass) |
|----:|--------|--------------:|--------------:|----------:|-------:|----------:|------------------------:|
| 0   | Spring | (boot)        | 300 (adopted) | --        | --     | --        | 0                       |
| 1   | Spring | 25            | 3             | 126       | 2      | 4         | 0                       |
| 2   | Summer | 11            | 0             | 215       | 13     | 1         | 6,015                   |
| 3   | Summer | 4             | 0             | 196       | 15     | 1         | 21,845,775              |
| 4   | Autumn | (line only)   | --            | --        | --     | --        | 22,012,995              |

- The lever held: `days_in_year=8.0`, `stage_secs=24685.71` (8 x 0.5 x
  86,400 / 14) on the boot line; the growth ticker and the census read the
  same TimeOfDay frame.
- Sowing followed the season: 25 sow jobs on the spring day, 11 and 4 on
  the summer days (claimed before the season turned; the verdict refuses
  new ones), none after. The adopted fields (300 lived-in sowings across 14
  stages) matured through summer: MATURE 2 -> 13 -> 15.
- INSTRUMENT SCALE: `sows_refused_by_season` counts every refused scan
  pass over every cell, so it reads 21.8 million on a summer day. It shows
  the refusal exists and nothing else; it should count distinct cells per
  day. Queued as a census fix (Y3c).

## The economy (Y3): confounded, not read

| day | roster | food_stock (drawable) | eat jobs | meals | targets shunned | starving at 21:00 / 03:00 |
|----:|-------:|----------------------:|---------:|------:|----------------:|---------------------------|
| 1   | 49     | 515                   | 40       | 47    | 60              | 2 / 3                     |
| 2   | 50     | 382                   | 167      | 97    | 180             | 7 / 13                    |
| 3   | 51     | 413                   | 313      | 79    | 324             | 20 / 23                   |
| 4   | 53     | 418                   | 360      | 71    | 363             | 24 (03:00)                |

The stock sat flat at ~400 units (2.5 days for 53) through summer while
starving rose from 2 to 24 of 53, and the eat jobs rose from 40 to 360 a
day with meals FALLING from 97 to 71: the town spent its summer walking
to food it could not reach. That is the walker class read on b1 the same
morning (RESULTS-wedge-probe.md: the roof-edge route and the raised
terrace store), not the calendar. This pair predates W2/W1/W2b. The
pre-registered economy bar ("days_of_food >= remaining winter days on
every winter day") cannot be read on this arm: the numerator is food the
walkers cannot draw.

Also seen: the roster grew 49 -> 53 during the famine. The population
loop gates arrivals on beds, not on food. Whether a starving town should
still take settlers is a design judgement for Ben (Banished: no; Dwarf
Fortress: migrants come regardless and starve).

## Disposition

Y0-Y2 PASSED on this read (the calendar, the lever, the stage length).
Y3 NOT READ (confounded). The arm runs on to winter for the calendar's
last claim (no stage-ups in winter); the economy read is re-registered
on a compressed-year arm booted from a pair carrying W2b and W1. NOT
evidenced: the winter par's effect on trade and forage demand.
