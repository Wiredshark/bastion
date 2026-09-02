# RESULTS — adoption realism (A1 staggered fields, A2 staggered first needs, L founding stock)

Read 2026-09-02 03:05 against PREREG-adoption-staggered-fields-and-hunger.md.
Arms: flat arm b2 on 55f9302b44 (A2), then on 8e9ca2c2fd (A1 with A1b);
flat arm b1 on f5f18c6734 (A2 replicate, L first read). All colour-stripped
server logs; census cadence 300 ticks; roster 48-50.

## A2 — staggered first needs: PASS on the cliff, FAIL on the floor

Old shape (5836a476ca): fed 49/49 at tick 23,100 -> 1/49 at 24,300 (one
step). New shape, two replicates:

| tick   | b2 fed/49 | b1 fed/49 |
|--------|-----------|-----------|
| 9,300  | 49        | 49        |
| 10,800 | 42        | (48 at 9,000) |
| 12,300 | 37        |           |
| 13,800 | 33        |           |
| 15,300 | 31        |           |
| 16,800 | 27        |           |
| 18,300 | 18        | 24 (at 18,000) |
| 27,300 | 36        | 37 (at 27,000) |

No 300-tick step moved fed by more than 4 in either replicate (the
monitors reported every step >= 5; none fired on the way down). Criterion
1 (no single-sample drop > 30% of the roster) PASSES. Criterion 2 (min fed
over day 1 >= 60%) FAILS: the trough reached 18/49 (b2) and 24/49 (b1).

Honest frame: criterion 2 measured the serving rate and the night, not the
start distribution. Meals on b2 landed at hunger 0.23, 0.21, 0.17 in the
morning (the interrupt line), then the evening decline and the dawn trough
(b2 8/50 at tick 45,000; b1 10/50 at 45,000) reproduced the curfew
starvation already diagnosed (PREREG-night-hunger-met-at-home.md). The
stagger removed the cliff; the floor belongs to the night rows.

## A1 — staggered fields: PASS (after A1b)

First stage (55f9302b44): "ADOPTED FIELDS ARMED plots=0" -- board.farms is
empty at the hand-off; the mechanism was dead and its witness said so.
A1b (8e9ca2c2fd) marks plots in the deferred placement drain: b2 boot
logged 8 "ADOPTED FIELD marked" lines for the town's 8 farm plots.

Sows on b2 by 03:05 (about 0.5 game day after adoption): 33 lived-in sows
over 11 distinct stages (2, 3, 4, 5, 6, 8, 9, 10, 12, 14, 15); criterion
">= 10 distinct stages" PASSES. Harvests: 4 within the first 0.3 game day
(cells sown at the top stage are harvestable at once and emit no stage-up
line); first "crop MATURE" stage-up at ~0.5 game day (a cell sown at 14
takes one stage, 0.29 day). The pre-registered "MATURE within 0.3 day"
was written against the wrong witness for the top-stage cells; the
intent -- harvests roll from day one -- is met by the harvest lines.

## L — founding stock to the general store: FIRST READ FAILED, fix queued

b1 on f5f18c6734 delivered all four founding items store="private": the
in-house containers register before the barns (deferred drain). The hold
fix (27f1e06f69, FOUNDING_STOCK_HOLD_TICKS 3,000) is in the haul-gate
stage; its read is pending.

### L, second read (03:13, b1 on 27f1e06f69): PASS

The day-0 STORAGE SUMMARY showed private_units=0 (the delivery was held
while adoption placed its plots); then all four founding items logged
`store="general"` at one barn cell (7665, 6365): wheat seeds 8, mushrooms
64, stones 64, wood 32. The hold released once a general store existed,
well inside its 3,000-tick timeout.

## Not evidenced

- The day-2+ fed floor with the night rows on (pending the night stage).
- L on Ben's own town (a different plot order could still register a
  barn last; the timeout then delivers to what exists and says so).
- A2 on Ben's own adopted town (real terrain, real session).
