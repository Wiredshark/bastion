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
