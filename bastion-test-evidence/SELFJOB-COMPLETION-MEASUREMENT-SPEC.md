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
