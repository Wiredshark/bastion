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

## THE ROOT CAUSE, found after this disposition was first written

The answer above ("the founding ships no materials") was the **symptom**. The
cause, found by reading the generator rather than the founding:

**The colony already owns an autonomous mining economy.** It scans for exposed
rock near home and emits Mine jobs, demand-driven and capped per colonist. It
never ran for a single tick, because the whole block is gated on

```
tick % ARBITRATION_INTERVAL == 2 && !board.plans.is_empty()
```

and `demand` was accumulated **only over `board.plans`**. A founding places
DESIGNATIONS, not plans — and so does every region a player paints. So the
colony owned 8 bed jobs each billing stone, owned a miner that could have dug
it, and the miner never woke.

The code said so in advance: *"Gated on a live plan: v1's only demand source
(AUTON-2 adds standing stock floors)."* A known-partial demand signal reads
exactly like a working one until something outside its population needs
supplying. The comment knew; nothing asserted it.

This makes decision 112's fork mostly **moot**, and that is recorded rather
than quietly banked as a win: I posed it as "ship a starter cache vs. build an
autonomous system". The autonomous system already existed and was blind to half
its demand. That is a defect, not a game-design fork.

### THE REGISTERED PREDICTION for the fix (written before its leg ran)

Re-run the **original `injury` arm** — the one with NO seeded materials, the
arm that produced `beds=0` and `rested→0`. If the demand fix is right, that
arm should now reproduce `matbeds` **without being handed anything**:

- `bastion: designation placed` for Mine jobs the colony generated *itself*
- `bed registered (built)` reaching 8
- `rested` dipping at ~tick 9300 and then **recovering**, not pinning at 0
- `idle` falling well below the 6.00 mean it held

If beds still do not appear, the demand fix is not sufficient and something
else also gates the miner — which is the result worth finding, because the
alternative is shipping a fix that reads as working because a *different* arm
was fed by hand.

### Independently confirmed, three times, before the fix existed

Three play sessions run the same night reached this defect separately:

- **Founder** (arena, 56,700 ticks): "founding gives you eight bed jobs and no
  way on earth to build them" — 64 of 64 claim considerations refused at the
  materials gate, `working=0 idle=8` on 29 of 49 samples. Then it painted a
  quarry by hand and the colony came alive within ~1,000 ticks:
  `working=6 moving=1 stuck=0 idle=1`, `blocked_materials` 8→0, all 8 beds
  registered inside four minutes. **That is the transition the fix must now
  produce with no player intervention** — and it also proves the rest of the
  pipeline was never broken, only starved.
- **Survivor** (arena, 87,600 ticks): all 8 bed jobs swept as unclaimable after
  930 seconds and never regenerated; `beds=0` in all 29 drive samples;
  `rested=0` for the final ~40,000 ticks; population fell 8 → 1.
- **Villager** (adopted town): the same starvation from the other direction —
  an adopted village mints no work at all, `working=0` in all 193 samples until
  a region was painted by hand.

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
