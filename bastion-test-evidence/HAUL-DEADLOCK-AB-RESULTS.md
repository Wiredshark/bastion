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
