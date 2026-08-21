# F13 — "a founded colony never sleeps and finishes nothing" — DISPOSITION: **CONFIRMED**

The prediction was registered in `FUNCTIONALITY-FIRST.md` and committed
(`387326c9b0`) **before** this leg was scored. Every branch of it landed. The
repair is still Ben's call (decision 112) — this row establishes the defect,
not the fix.

## The two arms

Matched control: identical seed, colony size, decay multiplier, script and
arena. The env lines differ in **one declared variable**, quoted from the two
attestations, both of which record `dirty .rs: 0` and a fresh binary:

```
injury   … BASTION_SEED_FOOD=64 BASTION_NEEDS_DECAY_MULT=6
matbeds  … BASTION_SEED_FOOD=64 BASTION_NEEDS_DECAY_MULT=6 BASTION_SEED_MATERIALS=64
```

## The result

| | injury (no materials) | matbeds (+materials) |
|---|---|---|
| `bed registered (built)` | **0** | **8** |
| `beds=` in the drive line | **0** all run | **8** |
| mean idle (n=40 / n=37) | 6.00 | 4.92 |
| mean working | 1.30 | 2.24 |
| mean rested | 5.20 | 6.86 |

The means understate it badly. The **trend** is the finding:

```
tick      300  1800  3300  4800  6300  7800  9300  10800
injury      8     8     8     8     8     8     0      0     rested
matbeds     8     8     8     8     8     8     5      7     rested
injury      1     8     7     8     8     6     8      3     idle
matbeds     1     8     7     7     7     6     2      1     idle
```

Both colonies start fully rested and decay **identically** — same seed, same
multiplier — through tick 7800. At ~9300 the need crosses its threshold and
the arms separate: the material-less colony drops to **0/8 rested and never
recovers**, while its twin dips to 5 and **climbs back to 7**. That recovery
is the sleep mechanism working. Its absence is not a colony choosing to stay
awake; it is a colony that cannot sleep, because recovery requires a bed, a
bed requires a completed Bed job, and that job requires a material the
founding never supplies and has no way to obtain.

`idle` separates at the same tick and in the same direction: 8→3 against 2→1.

## SELF-CORRECTION, and it matters

The commit that pre-registered this row states `rested=0/8 for the ENTIRE
run`. **That is wrong, and it is my error.** I read the last few census lines
instead of the aggregate. The colonists spawn rested and decay for ~9,300
ticks first; `rested=0` is where the run *ends*, not what it *is*.

The corrected reading is stronger evidence, not weaker: a claim of "always 0"
has no baseline and no control, and would have been satisfied by a colony that
simply spawned broken. What actually happened — two arms tracking each other
exactly for 7,800 ticks and then diverging at the precise tick the need
becomes live — is an attributable divergence with a shared baseline. I nearly
shipped the weaker claim because it was more dramatic.

This is the second time in this session the same reflex cost me: earlier I read
a two-hour-stale `adoptfed` log as current and briefly treated `threats=22` as
a live bug (it is arithmetically impossible under the current loop — that log
predates the fix, and its two rerun attempts both failed the staleness gate).
Both errors are the same shape: **reading the most recent lines and calling it
the run.** Aggregate first, then look at the trend, then quote a line.

## What is proven, and what is NOT

**Proven.** The defect is real and materials are *sufficient* to clear it. The
bed→sleep machinery works end to end the moment a bed exists: 8 beds built, 8
registered, rest recovering.

**Not proven, and not claimed.** That shipping materials is the right *repair*.
These green numbers come from a colony that was **handed** its materials by a
test env var. A pre-stocked colony proves the machinery, not the autonomy —
and autonomy is the thing the project is actually for. Decision 112 records
the fork (starter cache vs. the colony generating its own Mine/Chop work) and
recommends the autonomous branch with a small founding grant, but the ruling
is Ben's.

**Also unresolved, seen in passing and not chased here:** `food_per_cap`
oscillates to 0.0 and back within seconds in both arms, and mean `fed` is
*lower* in the arm that works more (4.54 vs 5.72) — consistent with busier
colonists eating less, but unmeasured. Logged, not diagnosed, and not counted
toward this row.
