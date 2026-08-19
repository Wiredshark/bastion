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
