# SELF-JOB COMPLETION — THE ONE MEASUREMENT THE AFTER-FAN'S BAR DEPENDS ON

**Written 2026-08-08 while 5b builds site 6. ★ Ready before it is needed, not
started when it is.**
★★★ **Ruled by Fable: this question FOLLOWS the build but PRECEDES the after-fan's
registration.**

## ★★★★★ WHY IT IS A PREREQUISITE, NOT A FOLLOW-UP

**Wave 30 measured a suspicious null:**

| | |
|---|---|
| ★★ **self-jobs ARE exercised** | `b5_self_job_reachability_probe` non-empty on **14/48 seeds** |
| ★★★ **the work economy did not move** | 6/114 fields, **all registered**; fail set identical BY MEMBERSHIP |

**Two accounts fit and the corpus cannot separate them:**

- **(a)** self-jobs are rare/short enough not to displace work.
- ★★★★★ **(b) they NEVER COMPLETE** — *time out, colonist returns to work, no rest
  banked and no work lost.* **"A defect wearing a pass's clothes."**

> ## ★★★★★★★ **THE MECHANISM THAT MAKES THIS BLOCKING (Fable's, and it is the
> part I missed):**
> **If (b) holds, SITE 6 IS THE FIX** — *persistent identity + re-claim is exactly
> what converts never-completes into completes.* ★★★ **Then rest time begins
> displacing work time, WORK-ECONOMY FIELDS LEGITIMATELY MOVE, and a blanket
> exact-match bar would INDICT THE ROW FOR SUCCEEDING.**

★★ **Fourth carve-out my exact-match bar has needed today — and the first that
could fail a CORRECT build rather than excuse a defect.**

## ★★★★★★★ THE DESIGN GAP I FLAGGED, AND THE FIX

**"Run it when the row is code-complete" yields the POST-fix rate. ★★★ The delta
I must register needs the PRE-fix rate — and by then the pre-row code is no
longer what is built.**

> ★★★★★ **FIX: BUILD THE INSTRUMENT ONCE, RUN IT AGAINST BOTH PINS.**

| arm | ref |
|---|---|
| **before** | ★★ **`bastion/baseline-pre-site6-instrumented`** = the existing `0fb7ca07b7` pin **+ the instrument commit ONLY** |
| **after** | 5b's green commit **+ the same instrument commit** |

★★★★★ **The arms then differ by EXACTLY the row, carrying an identical
instrument** — *matched on system AND axis, not "the same kind of number from two
different builds," which is the void-pair failure this lane keeps hitting.*
★ **Second dividend from the ref-pinning: the before-arm is still buildable
because nobody's commits can move that ref.**

> ## ★★★★★★★ **BINDING CONSTRAINT (Fable): THE INSTRUMENT IS THE *SAME COMMIT*
> ON BOTH ARMS — CHERRY-PICKED, NEVER RE-IMPLEMENTED.**

★★★ **This is the entire point of the shape, not a detail.** *Re-implementing the
counters on the before-arm — "it's only two counters, I'll just add them" — would
reintroduce the exact void pair the design exists to prevent:* ★★★★★ **two numbers
produced by two different pieces of code, differing in ways nobody enumerated,
compared as though they were the same measurement.**

★★ **Cherry-picking makes the instrument's identity BYTE-GUARANTEED.** ★ *If the
cherry-pick conflicts, that conflict is INFORMATION — it means the row touched
the instrument's own sites, and the arms are not as separable as assumed. Resolve
it explicitly and record it; never paper over it by hand-editing one arm.*

★★★ **This formalizes the sibling-binary-as-free-baseline pattern into the
acceptance flow: a sibling build is only a free baseline while the instrument in
it is provably identical.**

## ★★★ THE INSTRUMENT — MINIMAL, AND IT ANSWERS (a) vs (b) DIRECTLY

**Two counters per self-job kind, at branches the code already takes:**

| counter | site |
|---|---|
| ★★ **`self_jobs_created`** *(RestAt / EatFrom / Despond)* | the need-check pass's own insert sites |
| ★★★★★ **`self_jobs_completed`** | ★ **`RestAt` completes on `rest >= comfort + SLEEP_MARGIN`** *(the `slept` arm, `~12415`)*; release carries `ReleaseReason::Completed` |

> ★★★★★ **COMPLETION RATE = `completed / created`.** ★★★ **(a) predicts it is
> already healthy. (b) predicts NEAR ZERO. The accounts are not close together,
> so one number separates them.**

★★ **These are N2/N3 from `NEED-SUBSYSTEM-OBSERVABILITY-SPEC.md`, scoped down to
the two that settle this.** ★ **Increments at existing branches — no per-tick, no
per-cell.** ★★★ **Log-only and env-gated, so it never touches a fan's schema and
needs no additive window.**

## ★★★★★ BOTH OUTCOMES REGISTERED **BEFORE EITHER IS SEEN**

| measured | consequence for the after-fan's bar |
|---|---|
| ★★ **completion already high** ⇒ **(a)** | self-jobs are cheap; **work fields must HOLD.** ★★★ **Blanket exact-match restored, and correct.** |
| ★★★★★ **completion near zero** ⇒ **(b)** | ★★★ **the pre-existing defect is confirmed and site 6 is its fix.** **Bar becomes exact-match EXCEPT the work fields, magnitude derived from the measured completion delta.** |
| ★★★★★★★ **completion RISES but work fields DON'T** | ★★ **the most informative outcome, and the one to watch for.** *Colonists now rest and the economy absorbs it somewhere unenumerated.* **A finding in its own right — never a pass.** |

★ **Registering all three in advance is the point:** *no outcome can be narrated
into a success afterwards, and the third one is precisely the shape I would
otherwise have called "harmless."*

## ★★ SCOPE — WHAT THIS IS **NOT**

- ★★★ **Not a gate on 5b.** *They build uninterrupted; nothing here has been asked
  of them mid-row.*
- ★ **Not a fan.** *Two local, log-only runs over the 14-seed exposure set
  (`49 54 56 61 62 64 66 67 69 71 78 80 85 92`).*
- ★★ **Not the full N1-N6 observability row.** *That stays filed; this borrows two
  of its counters for one question.*
- ★★★★★ **Not a verdict on site 6.** *It measures a PRE-EXISTING condition the row
  did not create — and then tells the fan what to expect.*

## ★ SEQUENCING

1. **5b builds uninterrupted.**
2. **Row code-complete → instrument commit → pin `…-instrumented` off `0fb7ca07b7`.**
3. ★★★ **Two local runs, 14 seeds each, before and after.**
4. ★★★★★ **THEN write the after-fan's registration — a bar derived from a
   measurement rather than from my optimism.**
5. **Fan the pair.**

## ★★★★★★★★ METRIC CORRECTED **BEFORE THE RUN** — 5b FOUND A CONTAMINANT I HAD REGISTERED

**5b flagged it unprompted, on landing the instrument:** *`PendingNeed::Reclaim`
is NOT instrumented as a CREATE — it doesn't exist at the before-pin, and a
reclaim isn't a fresh creation anyway.* ★★★ **So on the AFTER arm a reclaimed
self-job never re-increments `created`, while its eventual `completed` still
fires.**

> ## ★★★★★ **THE RATIO `completed / created` IS THEREFORE NOT COMPARABLE ACROSS
> THE ARMS.** *It rises when the DENOMINATOR shrinks — which is a real effect of
> persistence, and has nothing to do with whether completion improved.*

★★★★★★★ **THIS IS THE NEW-PRODUCER / DENOMINATOR LAW ARRIVING INSIDE THE
MEASUREMENT ITSELF** — *the same law that superseded REG-2's counts, one layer
further in.* ★ **I registered the contaminated metric; 5b caught it before a
number existed.**

### ★★★ THE CORRECTED METRIC — **ABSOLUTE COUNTS, NEVER THE RATIO**

| quantity | what it answers |
|---|---|
| ★★★★★ **BEFORE-arm `completed` (absolute)** | ★★★ **THE DIAGNOSIS, ALONE.** *Near-zero ⇒ account (b): self-jobs never complete.* ★★ **IMMUNE to the contaminant — reclaim does not exist at the before-pin.** |
| ★★★ **AFTER-arm `completed` (absolute)** | **if (b) held and the row fixes it, this RISES** |
| ★★ **AFTER-arm `created` (absolute)** | ★★★★★ **should FALL** — *persistence replacing repeated fresh creates is a SEPARATE, testable prediction of the row working* |
| ★ ~~`completed / created`~~ | ★★★★★ **DO NOT USE ACROSS ARMS.** *Within a single arm only.* |

> ★★★★★★★ **THE DIAGNOSIS DOES NOT NEED THE AFTER ARM AT ALL.** *The before-arm's
> absolute `completed` count answers (a) vs (b) by itself.* ★★★ **The after arm
> exists to DERIVE THE BAR, not to make the diagnosis.**

★★ **And 5b's own framing was half right:** *the shrinking denominator IS "the
thing we're trying to measure"* — ★★★★★ **but it is a DIFFERENT thing from
completion quality, and a ratio welds the two into one number that answers
neither cleanly.** ★ **Two absolutes separate what one ratio confuses.**

## ★ CHERRY-PICK: DONE, AND THE MATCH VERIFIED

    bastion/baseline-pre-site6                -> 0fb7ca07b7   (before, clean)
    bastion/baseline-pre-site6-instrumented   -> ec972cf413   (before + instrument)
    bastion/after-sites46                     -> 0b2a9987c7   (after, clean)
    5b9a1a9724                                              (after + instrument)

★★★ **The cherry-pick applied with NO CONFLICT — which is itself information: the
row did not touch the instrument's own sites, so the arms are cleanly
separable.** ★★★★★ **And the applied diffs were compared line-by-line: the
instrument's added lines are IDENTICAL on both arms.** ★ *"Auto-merging" means git
had to merge — so the match was verified, not assumed.*

## ★★★★★★★★ RESULT 1 — **THE FAN SCENARIO IS INERT. MEASURED ON BOTH ARMS.**

**Same instrument commit, same 14 seeds, both pins, `--b5-scenario`:**

| arm | binary | logs | CREATED | COMPLETED |
|---|---|--:|--:|--:|
| **before** | `ec972cf413` *(attested, no `+dirty`)* | 14 | ★★★★★ **0** | ★★★★★ **0** |
| **after** | `5b9a1a9724` *(attested, no `+dirty`)* | 14 | ★★★★★ **0** | ★★★★★ **0** |

> ## ★★★★★★★ **THE ROW'S MECHANISM FIRES ZERO TIMES IN THE FAN SCENARIO, BEFORE
> AND AFTER.**

### ★★★ THE CONTROL THAT MAKES THE ZERO READABLE

**A zero is only evidence once you know the instrument ran.** ★★★★★ **Matched
control, same process, seed 71:**

| diag | hits |
|---|--:|
| `BASTION_RELEASE_DIAG` *(known-working)* | **34** |
| `BASTION_SELFJOB_COMPLETION_DIAG` | **0** |

★★ **Env, logging and the job pipeline proven live — so the zero is the CODE
PATH, not the plumbing.** ★ **And the instrument is independently proven to work:
`--preempt-scenario` seed 49, same binary, CREATED 4 / COMPLETED 1.**

## ★★★★★ THE BAR, FINAL — AND WHY THIS VERSION IS STRONGER

> **BLANKET EXACT-MATCH OVER EVERY FAN-VISIBLE FIELD. NO CARVE-OUTS. Any movement
> at all is a finding.**

★★★ **Two forms of the same claim, and this is the strong one:**

| form | argues about | cost |
|---|---|---|
| ★ **enumerate the absent fields** *(what I did first)* | **the INSTRUMENT** — *"the fan carries no field downstream of this mechanism"* | free, and weaker |
| ★★★★★ **measure the mechanism's firing on BOTH arms** | **the BEHAVIOUR** — *"the mechanism does not run here, either side"* | **one env-gated run per arm** |

★★ **The absent-fields argument was RIGHT. It was also the weaker version of the
same statement, and the strong one costs almost nothing.**

> ★★★★★ **STANDING FORM: before writing an exercised-denominator bar, MEASURE the
> denominator — on both arms — rather than arguing it from the schema.**

## ★ DISPOSITION OF THE CARVE-OUT MACHINERY

**D5, the dispersion expected-no-movement note, and the completion-derived
magnitude all STAND DOWN UNUSED.** ★★★ **Kept, not deleted** — *un-run machinery
with its rationale attached is a template; deleted machinery is a future
re-derivation.* ★ **They describe the world where the mechanism had been live in
the fan, which remains the world every later AUTON row will be in once the
scenario carries self-jobs.**

## ★★★ THE EPISTEMIC NOTE

> ★★★★★ **The measurement built to DERIVE a carve-out instead proved NONE WAS
> NEEDED.** ★★ **Both outcomes were pre-registered; I registered the wrong one as
> likely.** ★★★ *Being wrong about which branch fires, inside a frame where both
> branches were named, is what the pre-registration is FOR.*
