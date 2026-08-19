# DETERMINISM FAILS WITH THE BARRIER ON — measured on the first arm that could see it

`endurseed` twin pair, deterministic barrier ON, live colony observables. This is
the combination **no arm in the corpus had ever carried**: `provtrav*` has the
barrier but 7/7 constant gameplay fields; `provbase` has live gameplay but the
runner **clears** its barrier by name.

Scorer (`score-endurdet.sh`) was written **before** the data landed.

## Preconditions, printed above the verdict — all four hold

| | twin1 | twin2 |
|---|---|---|
| barrier witnessed (`autofound … deterministic path`) | ✅ | ✅ |
| booted | ✅ | ✅ |
| `food_stock` distinct values (**not vacuous**) | 2 | 3 |
| colonist name-list hash | `d6a994cb04e6` | `d6a994cb04e6` |

Identical name hash ⇒ identical `world_seed` **and** `seed_tick`, so the founding
RNG stream matched. Inputs are matched; the barrier is on.

## Verdict: GAME STATE DIVERGES

Aligned **by tick** before comparing — 905 common ticks (twin1 ran one sample
longer):

| observable | samples differing | last difference |
|---|---|---|
| `food_stock` | 75 / 905 (8.3%) | tick 26,100 |
| `designated_sweep_reaps` | **291 / 905** | tick 263,400 |
| `preempt_attempts` | **816 / 905 (90%)** | **tick 271,500 — the final tick** |
| `crop MATURE` | **8 vs 26** | — |
| `colonist arrived at job site` | 985 vs 1,009 | — |

### ★ I nearly reported the opposite, from one observable

`food_stock` differs on 75/75 samples inside ticks 3,900–26,100 and **0 of 818
after** — which reads as a self-correcting system whose divergence is bounded to
8% of the run. I was one step from telling Ben exactly that.

**It is an artifact of the floor.** Both runs return to `food_stock = 0` and stay
there; agreement *at zero* is agreement at a bound, not convergence.
[[a-field-cannot-calibrate-its-own-bound]] The two independent observables show
divergence running to the **last tick of the run**.

One observable said "converges", two said "never reconverges". The difference was
which one saturates.

## ★★ The outcome is BIMODAL, not noisy — 243×

Four `endurseed` runs, **identical founding RNG** (`d6a994cb04e6` on all four),
same arm, same script, ~271k ticks each; the last three are the same commit:

| run | maturations | peak food |
|---|---|---|
| `0438/e1` | **1,325** | 2,450 |
| `0549/e1` | **1,948** | 3,631 |
| `0549/e2` twin1 | **8** | 2 |
| `0549/e2` twin2 | **26** | 14 |

**The colony either establishes its farm loop or it does not.** Not a spread
around a mean — two regimes, 243× apart, from identical inputs.

## Consequences

**1. #114's score is WITHDRAWN.** I scored it *"starvation is the CAUSE — 1,325
maturations against a registered bar of 32, 41× the bar"* from **n=1**, and
stated the replication was owed. It does not replicate: **2 of 4 runs fall
below the bar** (8 and 26). The bar gives opposite answers on the same
configuration, so it does not discriminate what it was written to discriminate.

**2. The certification decision is settled on evidence.** *"Certify membership,
timing out of scope"* is **not defensible**: the schedule difference reaches core
gameplay and persists to the final tick. **The row FAILS**, and now for a
measured reason rather than a definitional one.

**3. Single-run colony measurements in this scenario are near-worthless.** A
result drawn from one endurance run could have been 8 or 1,948. Any row that
concluded from a single long run needs re-reading against this.

## Stated limits

One twin pair for the tick-aligned comparison (e1 timed out at twin1 again, its
second `rc=124`). The bimodality rests on 4 runs. What is **not** limited is the
direction: three independent observables agree that divergence persists, and the
243× spread is far outside anything a sampling artifact produces.
