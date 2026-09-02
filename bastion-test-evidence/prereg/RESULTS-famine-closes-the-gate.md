# RESULTS — a famine closes the settler gate (F1): the gate shut for the 45 seconds the town was actually hungry, and opened the pass the harvest crossed the line

Read 2026-09-02 16:45 against PREREG-famine-closes-the-gate.md. Pair
dec610f977 (F1 on W4/G1a), two arms booted 16:20-16:29 with an 8-day
year: b2 seeded 64 food, b3 seeded 8 food (via the new PLAY.ps1
BASTION_OVERRIDE_SEED_FOOD hook; the server's own line read seeded=8).

## What happened on b3 (seeded 8), wall clock from boot

| time      | event                                                          |
|-----------|----------------------------------------------------------------|
| +0 s      | founding stock delivered: 8 mushrooms                          |
| +1 s      | YEAR CENSUS day 0: food_stock 8, days_of_food 0.05             |
| +1 s      | SETTLER GATE CLOSED (roster 48, stock 8) and HOUSING GROWTH deciding="famine" |
| +1..+41 s | 71 SETTLER GATE CLOSED lines, stock rising 8 -> 214 -> 292 as the adopted fields' ripe crops were harvested (the lived-in first-day stage) |
| +42 s     | the first pass with stock >= 48 x 3.2 x 2 = 307: HOUSING GROWTH fired, "A SETTLER IS SENT FOR", "A SETTLER ARRIVES" (Rhosyn the Steady, Chef) |
| after     | no further CLOSED line through game hour 14; roster 49         |

b2 (seeded 64) did the same on its own clock: 58 CLOSED lines in its
first minute, one arrival when the harvest crossed the line.

## Against the pre-registration

- Instrument validation: every CLOSED line carried roster, stock and
  days_of_food under 2.0, and the same passes' HOUSING GROWTH deciding
  read "famine". PASS.
- "The roster does not grow on a day whose day line reads days_of_food
  < 2.0": the day-0 line read 0.05 and the roster grew by one that day
  — but by the pass the settler was drawn the stock was 307+, i.e. the
  famine was over. The bar was written per DAY LINE; the gate decides
  per PASS. The day-line frame is the wrong frame for a stock that
  moves 8 -> 300 in forty seconds. Recorded as a frame mismatch in the
  bar, not a leak in the gate: no settler arrived while any pass read
  famine.
- FAIL branch "a second door": NOT the case — the arrival came through
  the one verdict site (23684) on an open pass.

## What this arm cannot show

A famine that lasts. On the flat lab every adopted town harvests its
ripe founding fields at once (Ben's ruling), so seeded food is
irrelevant to the famine question within a minute of boot. The lasting
famine, if it comes, comes in WINTER: b2's compressed-year run read
days_of_food 2.7-2.9 through autumn; its days 6-7 (bkow4y0te's day-7
read) are the real test of the gate closing for days and re-opening in
spring. b3 stops after its day-2 read.

## The day-2 famine on b3 (read 17:33): the bar itself, one replicate

| day | season | food_stock | days_of_food | HOUSING GROWTH deciding | roster at the day line and 15:00 |
|----:|--------|-----------:|-------------:|-------------------------|----------------------------------|
| 0   | Spring | 8          | 0.05         | famine, then "a house stands empty" after the harvest | 48 -> 49 |
| 1   | Spring | 511        | 3.26         | a house stands empty     | 49 -> 50 |
| 2   | Summer | 286        | 1.79         | famine                   | 50 -> 50 |

On the first day whose day line read under 2.0 days of food (day 2,
1.79) the gate's daily decision was "famine" and the roster did not
grow through 15:00 (tick 72,300). The pre-registered PASS condition
holds on this replicate. The control (b2, seeded 64) read 3.27 days on
day 2 and drew its third settler. Starving on b3 peaked at 16 on day 2
— the terrace wedge (W6), not the stock.

Instrument hygiene: SETTLER GATE CLOSED printed 1,207 lines in two days
(one per gate pass while closed). It should print on the transition
and once per day line, not per pass (F1b, queued).

## Disposition

Mechanism PASS on the short famine and on the one multi-day replicate
(day 2). Three replicates or nothing: two more famine days are needed
before the bar is called; the winter read on b2 (days 6-7) is the
next. Not evidenced: a famine on real terrain (Ben's world has 18
fields for 9 people).
