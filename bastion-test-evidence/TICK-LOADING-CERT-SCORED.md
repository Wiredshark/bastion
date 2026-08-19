# TICK-LOADING CERTIFICATION — scored on the banked corpus

n = 62 runs / 31 twin pairs, all with driver logs, no new VM spend.

## The three registered bars

| bar | verdict | evidence |
|---|---|---|
| **1 — capped/uncapped promotion distribution OVERLAP** | **PASS, strongest form** | not merely overlapping: capped, uncapped **and** the cap=8 plant land on the **IDENTICAL** 242-key set. n=30 matched pairs |
| **2 — fingerprint holds with loading inside, incl. chunk timing** | **FAIL on the timing clause** | tick-sequence DIFFERS **31/31 pairs**, in **both** anchor classes (20 O/O, 10 R/R) |
| **3 — planted control goes red by name** | **PASS** | cap 4→8 changes the shape decisively: 92% of promoting ticks sit exactly at 4 under the cap; the plant spreads 1..8 near-uniformly |

## The clean separation this corpus establishes

| clause | result |
|---|---|
| promotion **MEMBERSHIP** | **IDENTICAL, 30/30 matched-input pairs** |
| promotion **SCHEDULE** | **DIFFERS, 31/31 pairs** |

The one pair whose membership differed (`0200/i1`) differed in its **input** — one
twin anchored at the colony, the other at the world origin
(`ANCHOR-ORIGIN-DEFECT.md`). With inputs matched, membership is deterministic and
schedule is not, and the two clauses come apart cleanly.

★ Bar 2's failure is **not** explained by that harness defect. Controlling for
anchor class leaves it unchanged: 20 of 20 O/O pairs and 10 of 10 R/R pairs both
diverge in tick sequence. The matched control excludes the defect as the cause.

## ★ THE QUESTION THAT DECIDES THE ROW IS UNANSWERED, AND I NEARLY REPORTED IT GREEN

"Certify membership with timing out of scope" is only legitimate if the schedule
difference **does not propagate into game state**. I measured it: gameplay
samples IDENTICAL in **31/31** pairs — including the split pair.

**That result is VACUOUS.** Checking the fields before quoting them: across
**2,502** pooled sample lines, all **seven** gameplay fields are constant zero.

```
food_stock 1 distinct [0] · splits 1 [0] · preempt_attempts 1 [0]
claim_expiry_releases 1 [0] · designated_sweep_reaps 1 [0]
generic_claim_leak_releases 1 [0] · emergency_access_completions 1 [0]
```

Two constant series agree for free. The `provtrav*` arms carry no colony, so
their gameplay observables cannot move, and "identical" there is worth nothing.
[[null-needs-a-couldnt-happen-witness]]

## ★★ WITHDRAWN — the section below is WRONG. Read this first.

**`provbase` has no deterministic barrier.** `run-pit.sh` clears `PITDET` for it
by name, with the comment *"the ONE arm that must take the LIVE free-running
drain"*. Its logs say `deterministic_drain=false` on every tick. It is the
deliberate free-running **control**, so divergence there is the expected,
designed behaviour and says nothing about the deterministic path.

I checked that precondition **after** committing the conclusion. That is the
process failure, not the arithmetic.

**And the follow-up was wrong too.** I then built a matrix and concluded *no arm
in the corpus has both the barrier and live gameplay*. It reported the barrier as
absent for ~96 arms because I keyed on `deterministic_drain=`, which is a field
of the **terrain census** — arms without `BASTION_TERRAIN_PROVISION_DIAG` cannot
emit it whatever their barrier state. An absent field read as an absent barrier.

The truth, from the config surface and a boot emit rather than from a token:

- `run-pit.sh`: `PITDET="${PITDET-BASTION_DETERMINISTIC=1 }"` — **the barrier
  defaults to ON**, so every arm that does not clear it has it.
- `endurseed` logs `autofound colony founded (deterministic path)` and carries
  live gameplay (`food_stock`, 188 distinct values).

**So `endurseed` can answer the question, and two twin pairs of it are running
now.** The gap I originally filed was real; the arm that closes it already
existed, and neither of my two readings found it correctly.

## The superseded reading, kept because the errors are the lesson

## ★ ANSWERED — and my "instrument gap" was wrong

I filed the gap below and then followed the banked-corpus-first rule anyway.
**The corpus already contained the arm**: `provbase` carries ~3,900 census emits
**and** live colony state. My claim that no arm could answer it was wrong, and
the search that corrected it cost nothing.

Three `provbase` twin pairs, every founding input verified matched first:

| control | result |
|---|---|
| full colonist name-list hash | **`d6a994cb04e6` — IDENTICAL in all 6 runs**, so `seed_tick` and the founding RNG stream match |
| founding block | all 60 untargeted spawns floor to **(16384, 16384)** |
| founding preset placement | farm at `x=16377` in every run |

With inputs matched:

| pair | terrain schedule | job arrivals | food_stock |
|---|---|---|---|
| `0818-0707` | differs | **44 vs 50** | differs at tick 3600 (0 vs 2) |
| `0818-1237` | differs | **44 vs 53** | differs at ticks 3600, 3900 |
| `0818-1510` | differs | 45 vs 45 | **identical** |

**Gameplay state diverges between twins whose inputs are identical.** Job
arrivals and food move together in all three pairs — including the pair where
both hold — which is the internal control.

★ Stated precisely, because the temptation is to overclaim: terrain schedule
differs in **3/3** but gameplay diverges in only **2/3**, so a schedule
difference is **not sufficient** and the causal link is **not established**. What
*is* established needs no mechanism: **twin runs with matched inputs produce
different game states.**

## What that does to the row

"Certify membership, timing out of scope" is **not defensible**. It would
certify something true (membership is deterministic, 30/30) that is **not the
thing that matters** — colony behaviour is not reproducible run-to-run, and that
is visible in the observable a player would care about. The row **FAILS**, now
for a measured reason with a named observable rather than a tripped clause.

## The superseded gap, kept for the record

## The instrument gap, named

Neither existing arm can answer it:

| arm | terrain census | gameplay fields |
|---|---|---|
| `provtrav*` | **11,400+ emits** | **DEAD** (7/7 constant) |
| `endurseed` | **0 emits** | **LIVE** (food_stock 188 distinct, sweep_reaps 19) |

**Successor, runnable today:** `endurseed` **with**
`BASTION_TERRAIN_PROVISION_DIAG=1` — one arm carrying both, so twin pairs expose
terrain schedule *and* live colony state together.

★ The anchor fix is a **precondition** for that arm, not a bonus: enabling the
census is precisely what triggered the origin race (55% vs 0/586). Run before
the fix, a combined arm would have confounded itself more than half the time.

## Standing decision, unchanged

Bar 2 is FAIL, so **compressed mode does NOT become the default** for unattended
runs. The law says on-green; this is not green. What is now ready for Ben is a
sharper question than before — membership determinism is measured at n=30 pairs
with set identity, and only the propagation question stands between that and a
defensible "timing out of scope". **That scoping call is his, not mine.**

---

# ★★ ANSWERED — the row FAILS, on evidence (`DETERMINISM-WITH-LIVE-GAMEPLAY-RESULT.md`)

The measurement this document said would decide the scoping question has run.
`endurseed` twin pair — barrier ON **and** live colony observables, the
combination no arm had ever carried. All four preconditions held, including
identical colonist name-hash (`d6a994cb04e6`), so the founding RNG matched.

**Game state diverges, and it does not reconverge:** `preempt_attempts` differs
on **816 of 905** tick-aligned samples, to the **final tick**;
`designated_sweep_reaps` on 291; `crop MATURE` **8 vs 26**.

And across four identically-seeded runs the outcome is **bimodal**: maturations
**1,948 / 1,325 / 26 / 8** — a **243×** spread, two regimes, not a distribution.

⇒ **Option A ("accept the roadmap criterion, certify membership, timing out of
scope") is not available.** The schedule difference reaches the observable a
player cares about. **The row FAILS**, for a measured reason.

⇒ Compressed mode stays **not** default, and now that is a finding rather than a
withheld default.

★ `food_stock` alone said the opposite — 0 differences across 818 samples after
tick 26,100 — because both runs saturate at zero. Agreement at a bound is not
convergence, and one observable would have certified this row.

---

# ★ TWO DIFFERENT BARS ARE IN PLAY, AND THEY DISAGREE

This row is being judged against two criteria written at different times. They
are not the same test, and reporting one verdict hides that.

### 1. The roadmap's own revalidation criterion — **PASSED**

`readme/BUILD-ROADMAP.md`, TICK-DRIVEN WORLD-LOADING:

> *"Revalidation: the N=8 distribution test re-run post-change must show
> capped/uncapped overlap."*

Measured at **n=30 twin pairs / 62 runs**: capped, uncapped **and** the cap=8
plant all land on the **identical** 242-key promoted set. That is not overlap,
it is set identity — the strongest form the criterion can take, at nearly 4× the
required N.

### 2. Ben's later 3-bar SPEED MANDATE — **BAR 2 FAILS**

> *"determinism fingerprint holds with loading inside (twin runs state-identical
> **including chunk timing**)"*

The timing clause is the addition. Tick-sequence **differs in 31/31 pairs**, in
both anchor classes. Membership is deterministic; schedule is not.

### What that means

The row **does** what the roadmap asked of it: promotion no longer depends on the
capped/uncapped axis, which was the wall-clock coupling the row existed to kill.
It does **not** meet the stricter later bar, which asks for tick-identical
scheduling as well.

**This is a scoping call, not a measurement gap, and it is Ben's:**

| option | consequence |
|---|---|
| **A — accept the roadmap criterion** | the row lands; compressed mode is justified by *what is promoted* being identical |
| **B — hold the mandate's bar 2** | the row stays open until scheduling is tick-identical, which #89 concluded is platform-level |

★ What is still genuinely unmeasured, and what would decide it on evidence rather
than on preference: **does the schedule difference reach game state?** Two
`endurseed` twin pairs — the barrier ON *and* live colony observables — are
running now, with `score-endurdet.sh` written in advance. If game state holds
identical, option A is safe on measurement rather than on definition. If it
diverges, option B is the honest call and neither bar was strict enough.

**Compressed mode is NOT declared default in the meantime.** The law says
on-green, and no reading of these results is unambiguously green.
