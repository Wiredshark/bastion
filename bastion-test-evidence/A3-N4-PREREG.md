# A3 (THE EAT LOOP) AT A SECOND POPULATION — **PRE-REGISTRATION**

Written before the run. The remaining half of the question `FOUNDING-COUNT-RESULTS.md`
opened, and the half that can actually break.

## 1 · WHY A3 IS NOT COVERED BY THE WORK-PULL RESULT

`POPULATION-SENSITIVITY-RESULTS.md` extended A2-B to n=4 (52.5% vs 47.6%) — but that
mechanism is **per-colonist and uncoupled**: 75 outcrop cells against ≤8 workers, so
contention never binds, and a *fraction* normalises population out by construction.

**A3 is the opposite.** The farm's yield is fixed by the plot; **consumption scales with
head-count.** Food per head is therefore `yield / N` — the one bar in this program with a
real population coupling, and the one that could genuinely have broken. It does not get
to inherit A2-B's result.

## 2 · THE BASELINE, AT n = 8

```
tilled 30 · sown 10 · harvested 10 · ate 3
food_stock:  0 -> 6 -> 4 -> 0
min hunger:  0.1509   (below the 0.2 base gate)
```

## 3 · THE BARS

### E1 · **THE LOOP STILL CLOSES** *(A3's own registered criterion, unchanged)*
- **PASS:** `bastion: ate` fires **≥ 1**, with `food_stock` having **risen first**.
- **FINDING (not failure):** `food_stock > 0` **and** hunger below threshold **and** no
  eat — that would mean the eat is blocked by something other than hunger or supply.

### E2 · **THE DIRECTIONAL PREDICTION** — registered, and falsifiable
> **Same yield, half the mouths ⇒ the ending `food_stock` should EXCEED the 0 seen at
> n=8.**
- **PASS:** ending `food_stock` > 0.
- **REFUTED:** ending `food_stock` = 0, which would mean consumption is *not* simply
  head-count-scaled and the coupling is more complex than the arithmetic suggests. **That
  is a finding, and a more interesting one than the pass.**

### E3 · **THE POPULATION ACTUALLY CHANGED** *(precondition, printed above the result)*
- **PASS:** `colonists=4`.

## 4 · WHAT I WILL **NOT** DO

1. **I will not shorten the 28,000-tick window.** It is derived from the eat gate and the
   decay rate; A3 went PARTIAL → PASS *because* the window stopped being guessed.
   Trimming it to save ~15 minutes of wall time would re-create the exact defect that row
   closed.
2. **I will not treat `ate 3` → some smaller number as a failure.** Fewer colonists
   crossing the gate inside one window is expected; **E1 asks whether the loop closes,
   not how many meals it serves.**
3. **I will not quietly drop E2 if it is refuted.** A refuted directional prediction is
   the row's most valuable output and gets reported as such.
