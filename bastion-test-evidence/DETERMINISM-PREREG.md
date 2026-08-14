# DETERMINISM FOR SCORED LIVE MAGNITUDES — **PRE-REGISTRATION**

Written before the runs. Forced by `HAUL-THROUGHPUT-RESULTS.md`, where the n=8 arm ran
A3's identical script and harvested **14 and 8** (A3 itself: 10), hauls **5 and 0**, peak
stock **4 and 0**.

## 1 · THE PROBLEM, AND THAT IT IS ALREADY DOCUMENTED

`server/src/lib.rs` says it outright:

> *"live otherwise boots PARALLEL with OS-entropy rtsim RNG (`tick_rng` falls back to
> `rand::rng()` when the flag is off) — which is exactly why two live runs of the same
> world diverged (different colonists, different wander)."*

So the variance is **known and named in the code**. What has never been checked is
whether the flag actually delivers reproducibility **for the numbers this program
scores** — a comment is a claim, not a measurement.

`BASTION_DETERMINISTIC` does three things: deterministic rtsim RNG, deterministic
worldgen, and serial execution.

## 2 · THE BARS

### D1 · **TWO DETERMINISTIC RUNS ARE IDENTICAL**
- Two runs, same script, same fresh userdata, `BASTION_DETERMINISTIC=1`.
- **PASS:** `harvested`, `haul deposited`, wheat/seed split and peak `food_stock` all
  match **exactly**.
- **FAIL:** any divergence — which would mean the flag does not cover the paths these
  bars read, and would be a **finding against the code's own comment**.

### D2 · **THE MATCHED CONTROL IS ALREADY MEASURED**
- The same script without the flag: **h8a vs h8b — 14 vs 8 harvested, 5 vs 0 hauls, 4 vs
  0 peak stock.**
- Re-running it would be theatre; it is recorded, from this session, on this binary.
- **D1 without D2 would be vacuous** — "two runs agreed" means nothing unless two runs
  *can* disagree, and they demonstrably do.

## 3 · WHAT I WILL **NOT** DO

1. **I will not use the chop-yield script.** Its numbers (15, split 5/6/4) reproduced
   across **three** non-deterministic runs already — they are fixed by geometry, not RNG,
   so they cannot discriminate. **A bar needs a measure that is capable of varying.**
2. **I will not retro-apply this to closed rows.** A2-B's 47.6 vs 0.0 and the work-pull
   52.5% are separations far wider than the observed spread, and the yield/XP bars are
   exact integers from constants. Those stand. This governs **future** bars resting on a
   live magnitude.
3. **I will not claim determinism I have not measured.** If D1 passes it covers *these
   counters over this window* — not "the engine is deterministic".
