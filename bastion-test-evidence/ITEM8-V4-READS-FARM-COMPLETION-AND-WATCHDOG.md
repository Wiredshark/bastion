# ITEM 8 v4 packet — the two mandatory reads (completion predicate + watchdog
budget), done before any design or build

**Note first: `bastion-test-evidence/ITEM8-V4-PACKET.md` is not present in
this worktree or on the shared remote (`bastion-origin/bastion/wip-batch-
verify` fetched and checked) — proceeding from Opus's message content
directly, which specified both reads in enough detail to act on. Flagging
in case the packet lives somewhere I don't have visibility into.**

## READ 1 — THE FARM COMPLETION PREDICATE, AND A CORRECTION TO "0 OF 87"

**The predicate itself, read at `bastion_jobs.rs:14108-14235`:** a job
enters Farm's completion arm only once `job.progress >= threshold` (1.0
for Farm — `chop_fell_sets` doesn't apply). At that point the code reads
the CURRENT terrain state at `job.pos` and matches it against three
shapes (TILL: filled non-Earth ground; SOW: empty crop cell over tilled
Earth; HARVEST: mature `WheatYellow`). A match applies the world edit,
logs `"tilled"`/`"sown"`/`"harvested"`, and removes the job. **No match
(the `_ => {}` arm) still removes the job** — silently, no log line, a
"clean moot release." **The predicate itself is not the defect: every
job that ever reaches it either completes correctly or moots cleanly.
Nothing gets stuck inside this arm.**

**Re-measured directly against v3's log (ANSI-stripped for accurate
grep), correcting the "0 of 87 completed" framing:**

    farm jobs created:        87 total (48 sow=true, 39 sow=false)
    "bastion: tilled":        19
    "bastion: sown":          20
    "bastion: harvested":     20
    total completions:        59  (19 + 20 + 20)

**39 sow=false creations = 19 TILL + 20 HARVEST exactly** (the log's
`sow` field can't distinguish TILL from HARVEST — both have `req: None`
— but the completion counts resolve it precisely: 19+20=39, an exact
match, no slack). **TILL: 19/19 completed (100%). HARVEST: 20/20
completed (100%). SOW: 20/48 completed (42%) — 28 SOW jobs never
completed.** 59 total completions also matches Opus's own "59 arrived-at
and worked" figure exactly — meaning **the 28 uncompleted jobs are not
"worked but never registered," they are the 28 jobs that were NEVER
arrived-at or worked at all** (87 − 59 = 28 = 48 − 20, the same number
both ways).

**Corrected framing: the completion predicate works correctly 100% of
the time it is reached. The defect is specifically that 28 of 48 SOW
jobs (58%) never got engaged by any colonist in the first place — TILL
and HARVEST show no such gap.** This changes where a fix needs to land:
not the predicate, but whatever determines which claimed jobs a colonist
actually works toward completion.

## READ 2 — THE TRAVEL WATCHDOG'S BUDGET, AND THE CLAIM-RELEASE GAP THAT
EXPLAINS THE 28

**`STUCK_TIMEOUT: f32 = 10.0`** (`bastion_jobs.rs:1794`) — per-colonist
travel/movement watchdog. Accrues while a colonist makes no net progress
toward a target; resets on ≥1 block of net progress. On trip
(`stuck_time > STUCK_TIMEOUT`), several release paths exist depending on
context (clean queue-release back to idle, soft-collision grace, or
degradation toward the carve/unreachable pipeline) — this is a
MOVEMENT watchdog, distinct from job-claim lifecycle.

**`ACCESS_STALL_SECS`** (`bastion_jobs.rs:1543-1577`) — currently
DERIVED, not a bare literal, per row 103's own fix: `QUEUE_WAIT_BASE_SECS
+ QUEUE_WAIT_PER_TURN_SECS * QUEUE_WAIT_MAX_POSITION +
QUEUE_WAIT_PER_TURN_SECS` = 930s today, explicitly set above the maximum
lawful queue-wait budget so it can no longer collide with a colonist
correctly waiting its turn (the exact #103 bug Opus's message cited —
already fixed, now "a beyond-lawful-maximum backstop expected to rarely
fire," confirmed by reading the constant's own doc, not assumed from the
name).

## ★★★ THE MECHANISM THAT EXPLAINS THE 28 — found by tracing the
claim-release path, not inferred

**`to_release`'s drain (`bastion_jobs.rs:14688-14694`), the ONE place a
preempted colonist's job claim gets freed:**

    if let Some(job) = board.jobs.get_mut(&job_id)
        && job.claimed_by == uids.get(*entity).copied()
        && !job.unreachable
    {
        job.claimed_by = None;
    }

**The release is gated on `!job.unreachable`.** A job flagged
`unreachable` (set at three sites: `13306`, `15128`, `18125`, all
degradation outcomes of the STUCK_TIMEOUT pipeline above) **never gets
its `claimed_by` cleared when its holder is preempted away** — need-
preemption, breakdown, any `to_release` push. The colonist moves on to
Eat/Rest/Despond; the job stays permanently attributed to a uid that
will never return to it.

**And the one mechanism that DOES reset `unreachable` — the periodic
"amnesty" grant (`bastion_jobs.rs:17901-17928`) — resets `unreachable`
back to `false` but never touches `claimed_by`.** So even after amnesty,
the job is `unreachable: false, claimed_by: Some(stale uid)` — eligible
by the unreachable flag, still permanently excluded from candidate
search by whatever filters on `claimed_by.is_none()` (not traced further
here — that filter is design-phase territory, not this read's scope).

**This is the immortal-job mechanism, read from the code, not inferred
from the symptom:** a SOW job whose claimant stalls long enough to be
marked `unreachable`, then gets preempted (need or breakdown — v3 had
331 breakdowns and 502 preempt_attempts competing for colonist time)
before returning to retry, is claimed forever. TILL and HARVEST jobs
completing at 100% while SOW alone shows a 58% gap is consistent with
this — SOW targets are created continuously through the run as tilled
ground becomes available (unlike TILL/HARVEST which front-load early and
finish before famine pressure peaks), putting more SOW claims in flight
during the high-preemption famine window. **Named as consistent, not
proven** — this read did not trace per-job claim histories to confirm
any specific one of the 28 actually went through this exact path; it
identifies the mechanism the code permits, which is the requested read.

## WHAT THIS MEANS FOR THE PLANNED FIX ORDER (not designed here, just
noted against the read)

Opus's ruled order — claim expiry first, completion seam, sweep
extension last and never alone — is consistent with this read: **route 2
(claim expiry) is exactly what's missing for the `unreachable`-gated
orphan claims found above**, and the requirement that "the post-expiry
lifecycle must end" (not just migrate the job to a different stuck
population) means an expiry fix has to release `claimed_by` unconditionally
(not gated on `!job.unreachable` the way the existing preempt-release
path is) and hand the job back to the pool in a state candidate search
will actually pick up — the SAME `claimed_by.is_none()` filter this read
flagged but did not trace.

**Reads complete. No design or build performed — holding for the
packet/ruling per Opus's instruction.**
