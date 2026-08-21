# THE UNREACHABLE LATCH — pre-registration, 2026-08-21

Found by two independent play sessions the same hour, from opposite directions.

## The defect

`job.unreachable` is set to `true` when **one** colonist's access request fails
to find a route. It is initialised `false` at job creation and **nothing ever
sets it back**. Verified by grep: every `unreachable: false` in the file is a
struct literal at construction; there is no assignment to `false` anywhere.

So a single colonist, stranded in open ground, marks jobs unreachable **for the
entire colony, permanently**.

## What the sessions measured

**Founder arm** (arena, 48 game-days):
- `tick 20620  jobs_total=6  jobs_unreachable=6`   — 100%
- `tick 42137  jobs_total=15 jobs_unreachable=15`  — 100%
- `census tick 20700  total=8 working=0 moving=0 stuck=0 idle=8`
- **and both "why" fields read zero**: `blocked_materials=0 blocked_stance=0`
- One colonist at `(16413,16371)` released **13 different jobs** as unreachable
  from that single spot; another released 11 from `(16452,16382)`.
- 261 × `job unreachable — claim released`, 176 × `GOTO-STAND-RESCUE` that
  "fires constantly and fixes nothing".

**Adversary arm** (town): 131 jobs, 118 blocked, `working=0` for 13 consecutive
census samples.

The founder also noted the colony *"recovers on its own after a while, then
relapses"* — which fits exactly: recovery comes from **new** jobs being created
(`unreachable: false`), never from old ones clearing.

## Why this is the right diagnosis and not the obvious one

The obvious reading is "pathfinding is broken". It is not sufficient: the same
session found that **painting fresh work un-froze all four stranded
colonists**. They could move. The jobs they were refusing were poisoned, not
unwalkable.

## THE FIX

`unreachable` becomes a **retryable** flag rather than a permanent verdict:
cleared on a slow cadence so a job poisoned by one colonist's bad position gets
re-offered to a colony that has since moved.

This matches the file's own stated philosophy at the churn-release site —
*"Retries are the mechanism"* — which the latch quietly contradicted.

## PREDICTION

Re-run the `injury` arm (founded colony, self-mining, ~12,000 ticks).

1. `jobs_unreachable` must not sit pinned at 100% of `jobs_total`.
2. Mean `working` must rise above the arm's current value, because jobs
   currently latched out of reach return to the pool.
3. The colony must still reach 8 beds — the fix must not break the founding.

## FALSIFIERS

- Working share unchanged ⇒ the latch was not the binding constraint here, and
  the stranding has a separate cause. A real possibility: these colonists may
  be unable to reach the work for a genuine terrain reason, in which case
  clearing the flag just re-runs a failing route.
- Beds regress below 8 ⇒ the retry churn is starving real work; revert.
- Route-planning cost climbs sharply ⇒ clearing too often, and the cadence is
  wrong rather than the idea.
