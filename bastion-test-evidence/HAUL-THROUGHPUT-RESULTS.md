# HAUL THROUGHPUT vs POPULATION — **RESULTS & ROW DISPOSITION**

Scored against `HAUL-THROUGHPUT-PREREG.md`. Engine tip `9332a553c8`.

## THE SCORE

| bar | verdict | evidence |
|---|---|---|
| **H1** hauling is witnessed | ✅ **PASS** | `haul deposited` with `item=`, `dropped=`, `dest=` — 12 deposits across four legs |
| **H2** arrivals explain the stock | ⚠ **PARTIAL** | wheat deposits track stock in 3 of 4 legs; **h4a is unexplained** (2 wheat, peak 14) |
| **H3** haul throughput scales with population | ⛔ **REFUTED** | **n=4 outperformed both n=8 legs** |

## THE DATA — two runs per arm, identical script but for the count

| leg | harvested | hauls | wheat | seeds | peak food_stock |
|---|---|---|---|---|---|
| n=8 run 1 | 14 | 5 | 2 | 3 | 4 |
| n=8 run 2 | 8 | **0** | 0 | 0 | **0** |
| **n=4 run 1** | **18** | 5 | 2 | 3 | ✅ **14** |
| n=4 run 2 | 10 | 2 | 1 | 1 | 2 |

## ⛔⛔ **H3's REFUTATION IS ITSELF REFUTED — DETERMINISTIC RERUN, 2026-08-14**

Everything below this section was measured **through a connected client with the flag
off**, before the determinism row existed. Re-run **headless + `BASTION_DETERMINISTIC`**,
two legs per arm:

| | harvested | hauls | wheat | peak stock |
|---|---|---|---|---|
| n=8 leg 1 | 22 | **24** | 10 | 44 |
| n=8 leg 2 | 22 | **24** | 10 | 44 |
| n=4 leg 1 | 8 | **0** | 0 | 0 |
| n=4 leg 2 | 8 | **0** | 0 | 0 |

**Each arm reproduces exactly, and the arms differ starkly: 24 hauls vs 0.**

> **H3 HOLDS. Haul throughput does scale with population** — and the original refutation
> was an artefact of the noise the determinism row later characterised. The old spread
> (hauls 5, 0, 5, 2) straddled the true effect completely, and one noisy n=4 leg
> out-hauling a noisy n=8 leg was enough to make me call it dead.

**What this costs:** a refutation I reported with confidence was wrong. What saved it from
staying wrong is that the row **registered its own weakness in the same breath** — "the
effect, if any, is smaller than this run-to-run variance" — which is precisely the claim
determinism was then able to test.

**And A3-at-n=4's retraction stands anyway.** Its peak-stock 6→2 was still n=1 per arm
through a client; this rerun does not rehabilitate that observation, it replaces it with a
measured one.

## ⛔ H3 IS REFUTED — AND SO IS THE OBSERVATION THAT MOTIVATED IT *(superseded — see above)*

A3-at-n=4 recorded peak stock **6 (n=8) → 2 (n=4)** and I proposed halved haul
throughput as the mechanism, registering it as *untested*. Testing it:

**The best leg in this whole set is n=4** — 18 harvested, 5 hauls, peak stock **14** —
beating *both* n=8 legs, one of which hauled **nothing at all**.

**The between-run spread exceeds the between-population difference.** Within n=8: hauls
5 vs 0, stock 4 vs 0. Within n=4: stock 14 vs 2. Population explains none of it.

> **So A3-at-n=4's 6 → 2 was NOISE, not a population effect.** The observation that opened
> this row does not survive replication, and the mechanism I proposed for it is refuted.
> Both are retracted here rather than left standing in that disposition.

This is exactly why the earlier row recorded the peak difference as *an observation with
an untested mechanism* instead of an explanation. Had it been written as a finding, it
would now be a false fact with a plausible story attached.

## ⚠ THREE INSTRUMENT ERRORS IN ONE ROW — all mine, all caught before scoring

1. **No item id.** First witness logged only a count. `deposit_all_of` moves
   `required_item` — *any* kind — while `food_stock` counts food only, so the first run's
   *95 deliveries vs 54 stock* compared two different populations of thing. Caught by a
   number that could not mean what it needed to.
2. **A hand-written food classifier that missed the food.** My grep listed
   carrot/cabbage/mushroom/… and **not `wheat`** — the arena's actual crop. It reported
   `FOOD hauls=0` in every leg while wheat was being hauled in front of it. *A grep
   pattern is a claim about naming, and this one was false.* Recomputed from the real ids:
   **5 wheat, 7 wheat_seeds.**
3. **n=1 per arm.** The first attempt would have compared two single runs whose variance
   dwarfs the effect. Two per arm is what exposed the refutation.

## ★ VARIANCE IS THE ROW'S REAL RESULT

These runs use **OS-entropy `tick_rng`** unless `BASTION_DETERMINISTIC` is set. The n=8
leg here ran A3's *identical script* and harvested 14 and 8, where A3 itself recorded 10.
**Single-run live numbers in this program are not reproducible to better than ~2×**, and
several earlier rows report exactly one run per arm.

That does **not** overturn them — A2-B's 47.6% vs 0.0% and the work-pull 52.5% are
separations far larger than this spread, and the yield/XP bars are exact integers derived
from constants. **It does mean any future bar resting on a live *magnitude* needs
replication or determinism**, and that is now on the board rather than in my head.

## WHAT I DECLINE TO CLAIM

- **Not** that hauling is population-independent. Four legs cannot show that either; the
  honest statement is **the effect, if any, is smaller than this run-to-run variance.**
- **Not** that h4a's 2 wheat → peak 14 is explained. Stock counts items lying in the
  stockpile, and seed drops and the founding stock also land there; the arithmetic is not
  closed and is registered open.
- **Not** that `food_stock` is a food-only counter *in the sense H2 assumed*. That premise
  needs its own read before any future bar leans on it.

## SESSION QUEUE STATE — fifteen rows closed

…13. The chop yield · 14. The XP witness · 15. **Haul throughput**, this document.

**Next:** the determinism question this row forces — either run scored live magnitudes
under `BASTION_DETERMINISTIC`, or replicate them. That is a standing methodology fix, not
a feature row.
