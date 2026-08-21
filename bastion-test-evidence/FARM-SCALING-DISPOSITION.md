# FARM SCALING — DISPOSITION: **PASS 3/3**, with a dose-response the row did not ask for

Scored against `FARM-SCALING-PREREGISTRATION.md`. Arm `scalelong` (32
colonists, 33,900 ticks), attested fresh, `dirty .rs 0`.

| prediction | bar | result |
|---|---|---|
| 1. farm plot materially larger | — | width **5 → 20** cells (16362…16381) |
| 2. mean working share | **>15%** (was 12%) | **17.7%** |
| 3. sow/harvest rise | — | **725 / 627** (was 382/341) |

## ★ The unplanned dose-response

Three attested runs of the *same arm*, differing only in farm area, because my
own broken intermediate fix accidentally supplied the low point:

| farm cells | working share |
|---|---|
| 30 (the broken multiplicative form) | 8.7% |
| 48 (the original additive form) | 12.0% |
| 120 (linear in population) | **17.7%** |

Monotone across three points. This is much stronger than the pass/fail the
pre-registration asked for: it is not "the fix helped" but "**farm area drives
the working share, in proportion**". I did not design that comparison — I got
the middle and low points by being wrong twice, and the honest thing is to say
that the strongest evidence in this row is an accident of my own errors rather
than an experiment I planned.

## What is fixed, and what is not

**Fixed:** renewable work now scales with population. At n=32 the farm is 4×
the area for 4× the people, and the n=8 identity is untouched
(`plan_matches_preset_at_eight` green), so no banked corpus leg moves.

**Not fixed:** 17.7% is better than 12%, and it is not *good*. Roughly 26 of 32
colonists are still idle at steady state. Farm area was **a** binding
constraint, not **the** binding constraint — the colony still runs out of work,
just later and less severely.

That remainder is the banked design question — *what does a large colony do all
day?* — and it is not an arithmetic problem. A colony whose only renewable
activity is farming will always cap out; the answer is standing work that is
not farm-shaped, which is Ben's call to shape.
