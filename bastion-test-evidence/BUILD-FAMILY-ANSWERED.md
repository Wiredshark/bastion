# WHY BUILD NEVER STARTS — **ANSWERED**, ON THE INSTRUMENT'S FIRST READ

**`b5_build_job_diag` landed in wave 30 (its first appearance in any wave) and
was specced for exactly this question.** ★★★ **`BUILD-IS-UNINSTRUMENTED.md`
opened with:** *"BUILD IS THE MOST COMMON FAILURE AND NOT ONE OF ITS TWELVE
FIELDS DISCRIMINATES … Fable's record has it **unclassified**."*

> ★★★★★ **It is classified now.**

## ★★★★★★★ THE MEASUREMENT — THE FIRST BUILD JOB ON EACH CORE SEED

| seed | `unreachable` | `times_offered` | `timeouts_on_this_cell` | `starvation_cycles` | `blocked_by` |
|---|---|--:|--:|--:|---|
| **61** | ★ **True** | 2 | **2** | **323** | null |
| **62** | ★ **True** | 3 | **3** | **249** | ★ **== pos** |
| **71** | ★ **True** | 2 | **2** | **323** | null |
| **80** | False | 2 | 1 | 99 | ★ **== pos** *(claimed now)* |
| **85** | ★ **True** | 3 | **3** | **285** | ★ **== pos** |
| **92** | ★ **True** | 3 | **3** | **274** | ★ **== pos** |

> ## ★★★★★★★ **`times_offered == timeouts_on_this_cell` ON FIVE OF SIX.**
> **EVERY SINGLE OFFER ENDED IN A TIMEOUT. A 100% failure rate on a denominator
> of 2-3, after which `unreachable` latches.**

★★★ **And `blocked_by == pos` on four of six — the blocking cell IS the target
cell.**

## ★★★★★ THE CHAIN, END TO END, EVERY LINK MEASURED

> **build cell offered → claimant times out (×2-3) → `unreachable` LATCHES →
> arbitration SKIPS the job → it starves 99-323 cycles → `build_placed` is never
> true.**

★★ **The latch is read from the PRODUCER, not the field name** — `Job::unreachable`'s
own doc: *"Set when a claimant repeatedly failed to reach the site; **unreachable
jobs are skipped by arbitration** and logged."* ★ **So the skip is by design; the
starvation follows from it.**

### ★★★★★★★ AND THE COLONY IS NOT BROKEN — **THE SECOND JOB IS HEALTHY ON EVERY SEED**

**Each core seed has TWO build jobs. The second one, on all six:**
`unreachable: False` · `starvation_cycles: 1-3` · `timeouts: 0` · **claimed by a
named colonist on four of six.**

> ★★★★★ **BUILD WORKS. ONE SPECIFIC CELL IS UNREACHABLE, AND IT STARVES THE
> CLAUSE.** ★★★ **This is not a build-logic failure at all.**

## ★★★★★★★★ SO THE BUILD FAMILY IS A **REACHABILITY** FAMILY

★★★ **Six of the ten hard-core seeds — the largest family — fail on the same
mechanism the travel row has been chasing all along.** ★★ **Same signature as the
self-job travel timeouts measured in wave 30 (14/48 seeds) and the
`route_exhausted` producer that fires on mine cells.**

> ★★ **Named, not narrated: the clause is `build_placed`; the mechanism is
> `unreachable`. Those are different subsystems, and the clause has been pointing
> at the wrong one for the whole life of the corpus.**

## ★★★ THE ADJACENT OBSERVATION — **CO-OCCURRENCE ONLY, NOT A FINDING**

**Tonight's `derived.py` run on the same wave reported:**

    b5_access_plan_self_rescue   calls=71  emitted=0  refused=71 (100.0%) in 15 seeds
    concentration vs verdict:    fail 7/15 = 46.7%   Fisher p = 0.0219

★★★★★ **Self-rescue is the mechanism that exists to carve access to jobs exactly
like these — and it refuses 100% of the time.**

> ★★★★★★★ **BUT `derived.py` ITSELF REFUSES TO CALL THIS A FINDING, AND IT IS
> RIGHT:** *"ZERO successful seeds, so 'refused' cannot be compared against
> 'succeeded' WITHIN this caller. A concentration result is therefore
> UNATTRIBUTABLE — it cannot separate cause from marker. **Do NOT declare a
> finding.**"*

★★★ **Recorded as a CO-OCCURRENCE with its instrument gap named.** ★★ **The two
facts sit next to each other and the corpus cannot join them** — *a tool refusing
to over-claim on my behalf, which is what it was built to do.*

## ★★ WHAT THIS CHANGES

1. ★★★★★ **The build family's row is NOT a build row.** *It is reachability, and
   it should be scheduled with the travel work rather than beside it.*
2. ★★★ **Six seeds plausibly move on one mechanism** — *the largest single lever
   in the hard core.*
3. ★★ **`blocked_by == pos` is the concrete next read**: *why is the target cell
   its own blocker?* ★ **One question, four seeds, and the coordinates are in
   hand.**
4. ★ **Seed 80 is the live specimen** — *still claimed, not yet latched
   unreachable, `starvation_cycles: 99`.* ★★★ **The only one where the latch has
   not closed, which makes it the one to watch a run on.**

## ★★★★★ THE PROCESS NOTE

**I applied the antidote adopted an hour ago — grep the ledger BEFORE
investigating — and it paid immediately: it surfaced `BUILD-IS-UNINSTRUMENTED.md`
and `BUILD-INSTRUMENT-SPEC.md`, which told me the question was already framed and
the instrument already specced FOR IT.**

> ★★★★★ **I did not discover this question. I read the answer to a question this
> lane wrote down, on the first wave that carried its instrument.** ★★★ **The
> corpus was under-read by exactly one field.**
