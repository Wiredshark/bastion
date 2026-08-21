# FARM SCALING — pre-registration, 2026-08-21

Fixing a formula I wrote earlier this session without deriving it.

## The defect, measured

`founding_plan` grows the farm additively:

```rust
let grow = |base: i32| base + ((n as f32).sqrt() as i32 - 2).max(0);
```

Farm base is 5×6 = 30 cells. At n=8 it stays 30; at n=32 it becomes 8×9 = 72.
So **area grows 2.4× for a 4× population**, and since farming is the colony's
only *renewable* work, the working share must fall. Measured: 25% → 14% → 9%,
and 12% mean over a 33,900-tick run at 32 with `farm plot registered: 1`.

## The fix, and why this shape

For farm area to track population, a square plot's side must scale as √n —
but **multiplicatively from the base**, not by adding √n−2 to it:

```rust
let grow = |base: i32| ((base as f32) * ((n as f32) / 8.0).sqrt()).round() as i32;
```

- n=8 → `base × 1.0` = base — **identical to today**, so
  `plan_matches_preset_at_eight` and every banked corpus leg still hold.
- n=32 → `base × 2.0` → 10×12 = 120 cells = **4× the area for 4× the people.**

The n=8 identity is the load-bearing property: this change must be invisible to
the corpus and visible only above it.

## PREDICTION

Re-run `scalelong` (32 colonists, ~33,900 ticks) with nothing else changed.

1. **`farm plot registered` still reports 1 plot**, but a materially larger one
   — the plot count is not the measure, its area is.
2. **Mean working share rises materially above 12%.** I will not pretend a
   threshold I cannot derive; the honest bar is *"above the 12% this exact arm
   measured, by more than the run-to-run noise the triplet showed (±3 points)"*,
   so **>15%** is the number to beat.
3. **Sow/harvest counts rise**, since the renewable work is what grew.

## FALSIFIERS

- Working share stays ≈12% ⇒ farm area was **not** the binding constraint at
  steady state, and the "runs out of work" diagnosis is incomplete. This is a
  real possibility: more farm cells could simply mean more cells sitting at
  Growth 0 waiting, rather than more claimable work at any instant.
- `plan_matches_preset_at_eight` goes red ⇒ the change leaked below n=8 and
  silently rebased the corpus. That test is the guard, and it caught me moving
  the pantry once already tonight.
- Tick cost at 32 rises sharply ⇒ the fix bought labour with performance, and
  the trade must be stated rather than hidden.
