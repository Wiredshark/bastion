# THE BIMODALITY IS ONE BINARY EVENT, NOT TWO DISTRIBUTIONS

## The measurement

14 runs, identical founding RNG. Escape = `food_stock` first exceeding 50.

| | escape tick | maturations |
|---|---|---|
| **escaped** (7) | **10,200 … 117,900** | **936 … 2,015** |
| **never escaped** (7) | — | **8 … 46** |

★ **Escape time varies 10×; yield varies 2×.** A colony escaping at tick 117,900
still produced 991. So post-escape production **saturates** — total is roughly
rate × remaining ticks, at ~0.005–0.008 maturations/tick across all seven.

## Why the 46 → 936 gap is empty

Not because there are two populations. Because **escape is binary and production
after it is near-constant**: the latest observed escape (117,900) still leaves
~153,000 ticks, which at the observed rate yields ~900+ — just above the gap.

**A colony escaping at tick 250,000 would land squarely in the gap, and none
did.** So the gap is a joint artifact of the **run length** and the **escape-time
distribution**, not evidence of two mechanisms.

★ That is a real limit on the "bimodal" language I used, and it is a prediction:
**a longer run, or a fixture that delays escape, should FILL the gap.** If it
does not, the binary-escape reading is wrong.

## The state the colony escapes from

| | |
|---|---|
| runs that jammed (`blocked_materials ≥ 20`) | **11 of 13** |
| of those, later recovered to 0 | **5** |
| never jammed at all | **2** — both started at `blocked_materials = 8`, both top the THRIVE range (2,015 and 1,948) |

**The jam is NOT absorbing** — 5 of 11 escaped it. Escape is possible at any
time: the last tick at which any job was assigned spans **300 … 264,300** in
collapsed runs, so a colony can work for 264,000 ticks and still finish with 10
maturations.

★ Recovery timing sets the magnitude: the one COLLAPSE run that recovered
(`b3/t2`, at its final sample) scored **46** — the top of the collapse range.
Recovering too late is arithmetically the same as not recovering.

## Corrections this supersedes

1. **"The split happens at the first harvest (tick ~3,600)"** — true of one twin
   pair, where ten consecutive census samples agreed and then diverged. Across
   the corpus, collapse onset spans 300 … 264,300. **Withdrawn as a general
   claim.**
2. **"`eligible = 0` is the signal"** — THRIVE shows `eligible = 0` too whenever
   it has jobs pending. The signal is whether the board **drains**.
3. **"The materials jam is absorbing"** — 5 of 11 recovered.

Each was checked against banked data within minutes of being written, and each
cost nothing but the check.

---

# ★ TESTING MY OWN PREDICTION — and it comes back INCONCLUSIVE

The prediction above was *"a longer run should FILL the gap."* That is testable
now, from the escape-time distribution, without spending anything.

Seven runs escaped by tick **117,900**. Seven never escaped in **271,000**.
Under a constant hazard fitted to those data (λ = **2.99e-6**/tick, mean
time-to-escape ~335,000 ticks):

| | |
|---|---|
| runs still trapped at tick 117,900 | 7 |
| **expected** late escapes before tick 271,000 | **2.57** |
| **observed** | **0** |
| P(observe 0 \| constant hazard) | **0.077** |

**Not significant at n=14.** It leans toward *early-or-never* — a hazard that
collapses once the colony has been trapped a while — but it does not reject a
constant hazard, and I am not going to report a 0.077 as if it did.

## The two models, and the run length that separates them

| model | consequence for the gap |
|---|---|
| **constant hazard** | late escapes happen; a longer run **fills** the gap with mid-range yields |
| **early-or-never** | trapped colonies stay trapped; the gap **never** fills, at any length |

Sizing, under the fitted λ:

| target late escapes | further ticks | total run | vs current |
|---|---|---|---|
| 3 | 187,000 | 305,000 | **1.1×** |
| 5 | 420,000 | 537,000 | **2.0×** |

★ So the discriminating experiment is **one fixture-length change**, not a new
instrument: at ~2× the current 271k-tick run, a constant hazard predicts ~5
mid-range outcomes and early-or-never predicts **zero**. The models diverge
sharply and cheaply.

★★ Recording this as **owed**, not done. The prediction in this document is now
quantified rather than rhetorical, and the honest current state is that
**n=14 cannot tell the two apart**.
