# F13 — "a founded colony never sleeps and finishes nothing" — **CLOSED, PASS**

## FINAL RESULT (2026-08-21, arm `injury`, attested fresh, `dirty .rs 0`)

**A colony founded on open ground now mines its own stone, builds its own
beds, and sleeps in them — with nothing handed to it.** All four blockers
cleared; the last one was the generator asking an easier question than the
claim path.

The comparison that matters is three arms, where `matbeds` is the *hand-fed*
control that was given materials by a test env var:

| | injury (original) | matbeds (hand-fed) | **injury (now, autonomous)** |
|---|---|---|---|
| `bed registered (built)` | 0 | 8 | **8** |
| `beds=` in the drive line | 0 | 8 | **8** |
| mean working | 1.30 | 2.24 | **2.88** |
| mean idle | 6.00 | 4.92 | **3.70** |
| mean rested | 5.20 | 6.86 | **6.58** |
| rested trend | 8→0, **never recovers** | 8→5→7 | **8→2→6, recovers** |
| completed sleeps | — | — | **6** |

The autonomous colony now **beats the hand-fed one** on working (2.88 vs 2.24)
and idle (3.70 vs 4.92) and matches it on rest recovery — which is the outcome
that distinguishes a real fix from a pre-stocked demo. Decision 112's warning
("green numbers from a pre-stocked colony do not prove autonomy") is answered:
these numbers come from a colony that was handed nothing.

The generator's own witness shows the last fix working as designed:
`rock_seen=25 rock_unstandable=16` — it found 25 rock cells, rejected the 16
no colonist could stand at, and mined the remaining 9. Bed ownership (B7-2)
also fired live: `beds assigned to their sleepers assigned=1 … beds=8`.

---

# Original disposition — **CONFIRMED**

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

## THE FULL CHAIN, traced by instrument across five legs

F13 was not one bug. It was four stacked blockers, each of which perfectly
hid the next — every one of them producing the identical player-visible
symptom (`beds=0`, nobody sleeps, colonists idle) and the identical silence in
the log. Each fix moved the failure one layer deeper and **looked like it had
achieved nothing**, which is the honest reason this took five legs and not one.

| # | Blocker | How it was found | State |
|---|---|---|---|
| 1 | Demand measured over build PLANS only, so designated bed jobs were invisible and the whole generator block never ran | Read the producer | **FIXED** |
| 2 | `MINE_GEN_RADIUS = 12`, and the arena's only stone sits ~20 blocks out — the generator ran and saw nothing | The `WANTED stone … rock_seen=0` witness, 778 firings | **FIXED** (→24) |
| 3 | The demand predicate excluded CLAIMED jobs, so fixing #2 let colonists claim the bed jobs, which **zeroed the demand that pays for the stone** | `already_claimed=13` beside `materials=8` in one refusal census | **FIXED** |
| 4 | The generator's "exposed rock" test and the claim path's "affordance" test disagree about what is minable, so it mints work nobody can accept | `demand=8 pending_mine=8 quota=0` with `affordance=56` refusals | **OPEN** |

Blocker 4 is now the live one, and it is the same shape as F13's original root
cause: **two predicates that must agree about availability, and don't.** The
generator accepts a rock cell if any of its six neighbours is open; the claim
path requires a standable stance for a colonist. Those are different
questions, and the generator is answering the easier one. Eight mine jobs sit
on the board permanently, refused 56 times per census — 8 jobs × 7 colonists,
every single consideration.

An adversarial play session hit the identical wall from the other side (row
F19): four jobs, eight colonists, `affordance=32`, and a dashboard reading
`jobs_unreachable=0 blocked_materials=0` — no player-facing bucket exists for
"minted work that cannot be stood next to".

**The generator must not mint work the claim path will refuse.** Whichever
predicate is right, one of them has to ask the other. That is the next row,
and it is stated as OPEN rather than folded into a fix I have not made.

### What each leg actually showed, including the ones that looked like failures

- Leg 2 (demand fix): 0 mine jobs. Read as total failure; was blocker 2.
- Leg 3 (radius fix): 0 mine jobs **and the witness went silent** — worse-looking
  than leg 2, and actually progress: colonists could now reach and claim the
  bed jobs, which zeroed demand. A silent instrument is not evidence of health.
- Leg 4 (predicate fix): still 0 completions, witness still silent — because
  the witness lived *inside* the branch it was meant to explain.
- Leg 5 (unconditional state emit): `demand=8 supply=0 pending_mine=8 quota=0`.
  The generator had been working for two legs. Nothing downstream could use it.

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
