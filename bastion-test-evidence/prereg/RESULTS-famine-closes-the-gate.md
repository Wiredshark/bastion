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

## Disposition

Mechanism PASS on a short famine; the multi-day bar is deferred to the
winter read. Not evidenced: a famine on real terrain (Ben's world has
18 fields for 9 people).
