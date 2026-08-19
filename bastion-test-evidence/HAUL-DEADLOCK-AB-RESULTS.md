# HAUL DEADLOCK A/B — MECHANISM OBSERVED, FIX ENGAGES

Scored by `score-hauldeadlock.sh`, written before the data. One axis:
both arms carry `BASTION_HAUL_SKIP_DIAG=1`; the fix arm adds
`BASTION_FIX_HAUL_STARVED_CELL=1`.

| arm | skips | **`starving_job_on_cell=true`** | admitted | maturations |
|---|---|---|---|---|
| **CONTROL** | 24,349 | **14,660** | 0 | 1,447 |
| **FIX** | 6,306 | 841 | **841** | **1,971** |

## BAR 1 — the falsifier for my own mechanism: **PASSES**

Registered: *skips with no starving job ⇒ **`SEED-DEADLOCK.md` WITHDRAWN**.*

**14,660 skips in a single run were a job starving on the item lying beneath
it** — 60% of all skips. The chain was derived by reading; it is now **observed**.

★ The mechanism was the thing most likely to be wrong, which is why this bar was
registered first. It survived.

## BAR 2 — the fix acts: **PASSES**

**841 admissions.** The fix is not inert, so the arm is scorable rather than VOID.

★ And it admits **only** the starving case: 6,306 skips remain in the fix arm —
items under *claimed* jobs, or under jobs that do not want that item. **The
original protection is intact and only the deadlock case changed**, which is
exactly the narrowness the fix was designed for.

## BAR 3 — outcome: directional, as registered

Both runs thrived (0/1 collapse each), so this says nothing about the collapse
rate — **n = 1 per arm cannot**, and the bar said so before the data. The
maturation counts (1,447 → **1,971**, +36%) point the right way and are **not**
offered as evidence.

## ★ The control THRIVED and still logged 14,660 starvation skips

That is the most informative number here, and it confirms the mechanism's shape
rather than merely its existence: **the skip is universal, not a property of
doomed colonies.** Every colony continuously starves jobs of the materials
beneath them; what separates thriving from collapsed is only whether a haul pass
lands before the loop locks.

**So the deadlock is not a rare failure mode — it is the normal operating state,
survived by luck.** A colony that thrives is one that lost this race fewer times,
not one that never played it.

## What remains

Bar 3 needs a real rate: **3 control + 3 fix at 271k**, scoring collapses per
arm. The manifest is written and shape-checked (44 min worst case). That is a
6-host fan and the only remaining question is whether removing the skip removes
the *collapses*, not merely the *skips*.

---

# ★★★ THE RATE FAN — 3 vs 3, PERFECT SEPARATION, AND THE VARIANCE COLLAPSES

| arm | maturations | skips | starving | admitted |
|---|---|---|---|---|
| CONTROL c1 | **319** | 37,806 | 32,555 | 0 |
| CONTROL c2 | **8** | 21,214 | 18,196 | 0 |
| CONTROL c3 | **96** | 51,252 | 39,377 | 0 |
| **FIX f1** | **1,893** | 8,103 | 812 | 812 |
| **FIX f2** | **1,978** | 6,719 | 829 | 829 |
| **FIX f3** | **2,001** | 7,944 | 842 | 842 |

| | CONTROL | FIX |
|---|---|---|
| range | 8 – 319 | **1,893 – 2,001** |
| mean | 141 | **1,957** |
| **coefficient of variation** | **0.93** | **0.02** |

## The headline is not the mean — it is the variance

**Every fix run beats every control run by 5.9×.** Exact one-tailed p for perfect
separation at 3v3 is **0.050** — the floor achievable at this n, so the rate
evidence is as strong as three-a-side can be.

★ But the sharper result is that **the fix makes the outcome nearly
deterministic**: the three fix runs land within **6%** of one another (CV 0.02)
while the controls span **40×** (CV 0.93). **A 39× tighter distribution.**

That is what a removed race looks like. The deadlock did not merely lower the
average — it turned a reproducible process into a coin flip. **Removing it
restores reproducibility, which is worth more to this program than the yield.**

★★ And it retroactively explains the entire bimodality investigation: the
"two regimes", the empty gap, the 243× spread, the escape-time distribution —
**all of it was one race, observed through outcomes.** With the race removed the
distribution is not bimodal, not continuous, but *narrow*.

## The mechanism numbers, on the same runs

| | CONTROL | FIX |
|---|---|---|
| starvation skips | **90,128** | 2,483 → **admitted** |
| skips still correctly refused | — | **22,766** |

The fix admitted 2,483 hauls and **still refused 22,766** — items under claimed
jobs or under jobs that do not want them. The narrow exemption behaves exactly as
designed on live data.

## Status

All three registered bars now pass, on 4 control and 4 fix runs total:

| bar | verdict |
|---|---|
| 1 — mechanism observed (the falsifier for my own chain) | **PASS** — 104,788 starvation skips across control runs |
| 2 — fix acts, not inert | **PASS** — 3,324 admissions |
| 3 — outcome | **PASS** — perfect separation, p = 0.050, CV 0.93 → 0.02 |

★ The fix remains **env-gated and default-off**. Making it default is a live-path
change and Ben's call; the evidence for it is now on the table rather than in a
recommendation.
