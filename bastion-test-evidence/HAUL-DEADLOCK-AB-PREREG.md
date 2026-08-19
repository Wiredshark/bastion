# HAUL DEADLOCK — WITNESS + FIX A/B, pre-registered

Written before any run. Both arms carry `BASTION_HAUL_SKIP_DIAG=1`; the fix arm
adds `BASTION_FIX_HAUL_STARVED_CELL=1`. One axis.

## BAR 1 — ★ THE FALSIFIER FOR MY OWN MECHANISM (control arm)

`SEED-DEADLOCK.md` claims a blocked farm job makes its own seeds invisible to
hauling. That chain was **read, not observed**.

| outcome | verdict |
|---|---|
| control logs ≥1 skip with **`starving_job_on_cell=true`** | mechanism **OBSERVED** — the deadlock is real, not inferred |
| control logs skips but **`starving_job_on_cell=false` on all** | items sit under work sites, but no job starves on its own input. **`SEED-DEADLOCK.md` is WITHDRAWN** |
| control logs **no skips at all** | the `occupied` exclusion never fires here; the whole chain is wrong and withdrawn |

★ This is the bar that can cost me the finding, so it is first. A mechanism I
derived by reading has to survive its own instrument before any fix is credited.

## BAR 2 — the fix ACTS (fix arm)

| outcome | verdict |
|---|---|
| ≥1 `haul ADMITTED onto a starved job's cell` | the fix engages |
| zero such lines | fix **inert** ⇒ the arm is **VOID**, not "no effect" |

Vacuity guard: a fix that never fires cannot be reported as not working.

## BAR 3 — outcome, directional only

Baseline: **11 of 26** identically-seeded runs collapse (<50 maturations).

| outcome | reading |
|---|---|
| fix-arm collapses ≈ 0 | consistent with the deadlock being the cause |
| fix-arm collapse rate ≈ baseline | the deadlock is real but **not the binding constraint** |

★ Stated as directional **on purpose**: detecting 42% → 10% needs ~10–15 runs
per arm, which is hours of hosts. **The mechanism bars are the cheap ones and
they are the ones that decide whether the fix is right** — an outcome shift with
no witness line would be luck, and a witness line with no outcome shift is still
a real defect found.

## Preconditions above every verdict

1. Both arms must reach the end (`script complete`), or that run is VOID.
2. Colonist name-hash `d6a994cb04e6` on every run, or it leaves the comparison.
3. The witness emit must appear in **both** arms — its absence in the FIX arm
   only would mean the fix suppressed the diagnostic rather than the deadlock.

## Why the fix is narrow, on purpose

The `occupied` exclusion protects against stripping items out of an active work
site. The exemption requires **all three**: same cell, job **unclaimed**, and
`required_item` **equal to this item's def**. An item under a claimed job, or
one the job does not want, is skipped exactly as before — so only the deadlock
case changes and the original protection is intact.
