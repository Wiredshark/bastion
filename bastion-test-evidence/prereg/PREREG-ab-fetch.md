# PRE-REGISTRATION — matched A/B: does the fetch watchdog bound the stall AND lift the famine?

Written before any leg produced a number. 6 legs, ONE binary (22:49:29), identical
env except `BASTION_FETCH_BUDGET_SECS`.

- **T1/T2/T3** budget 90 (watchdog ON)
- **C1/C2/C3** budget 99999 (unreachable => the pre-fix unbounded code path)

Three replicates per arm because colony event counts vary 2-3x run to run; n=1
has been the weakness of every measurement tonight.

## The two claims, which are NOT the same claim

**CLAIM 1 (mechanical, should be near-certain): the stall is bounded.**
- PASS: `FETCH BUDGET EXPIRED` appears in T and never in C; and
  `traveling-with-reservation` per-leg is materially lower in T than C.
- FAIL: no expiry lines in T => the branch is not reached and the fix is inert.
- Predicted by the mechanism: C should reproduce ~87,791 steers concentrated in
  <20 jobs; T should cut each stuck job at 90 sim-sec = 2,700 ticks.

**CLAIM 2 (economic, genuinely uncertain): the famine lifts.**
- PASS: mean hunger at matched tick materially higher in T, and fewer colonists
  at hunger 0.000.
- FAIL(no effect): stall bounded, hunger unchanged => the withheld-reservation
  story is WRONG or not the binding constraint, and the famine has another
  cause. This is a real possibility and must be reported as a refutation of my
  hypothesis, not as "needs more runs".
- FAIL(worse): T hunger LOWER than C. Plausible mechanism: releasing a claim
  mid-fetch churns more, so cooks restart forever and never finish. That failure
  would look like "the fix did nothing" if I only read the expiry count.

## Failures that render identically to success

- **Bounded but useless.** Expiry lines appear, steers drop, hunger unchanged.
  Reads as a win if I stop at the mechanical claim. Guarded by measuring hunger
  separately and reporting the two claims apart.
- **A dead leg.** A crashed server has zero steers AND zero expiries, which
  looks like "no stall occurred". Guard: check process alive + panic count +
  final tick BEFORE reading any count.
- **Arms not comparable.** Different worldgen per leg would make hunger
  incomparable. Same seed/config across all 6, but I must verify each leg
  actually adopted a town (`adopted=true`, colonists=8) before comparing.

## What this CANNOT test

- Whether the town LOOKS better. Separate looking sweep.
- The vertical strandings (row 0b) and the churn (row 0) — untouched by this fix.
- Whether 90s is the right budget; only that it is better than unbounded.
- Long-horizon effects. Legs run ~15-25 min wall, well under a full colony arc.
