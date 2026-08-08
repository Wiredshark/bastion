# TEN SEEDS HAVE FAILED IN EVERY WAVE OF THE CAMPAIGN — AND A REGRESSION HID INSIDE AN IMPROVEMENT

**Read from disk, no run.** Eight 48-seed waves carrying a verdict field
(wave14→wave26). ★ **Seed set is IDENTICAL across all of them** — 49–96, verified
set-equal, so **composition is excluded and every comparison below is
like-for-like.**

## §1 — THE DRIFT WAS REAL, AND IT WAS NOT THE STORY

The fail count I flagged as drifting (14 → 16 → 14 → 14 → 12 → 12 → 11 → 11)
decomposes into:

| | seeds |
|---|---|
| **FIXED** (failed w14, passes w26) | 51, 52, 55, 69 — **4** |
| ★ **NEWLY BROKEN** (passed w14, fails w26) | **90** — **1** |
| ★★★ **NEVER FIXED, fails in EVERY wave** | **54, 61, 62, 66, 68, 71, 78, 80, 85, 92 — 10** |

> ## ★★★★★ TEN SEEDS HAVE FAILED IN EVERY SINGLE WAVE, ACROSS ~12 BASELINES AND
> MANY COMMITS — AND NO LIST OF THEM EXISTS ANYWHERE.
>
> The aggregate **"11/48"** conceals that **10 of those 11 are the SAME seeds
> every time.** Only **7 seeds** in the whole corpus have ever churned; the rest
> of the failure population is a **fixed, permanent core.**

★ **`b5_failed_clauses` was on disk the entire time.** This is not a missing
instrument — it is [[aggregate-late-keep-the-structure]] again: **a count answers
"how many failed" and no adjacent question, including the one that matters —
"the same ones?"**

## §2 — ★★★★★★ AND A REGRESSION LANDED INSIDE A FALLING COUNT

**Seed 90 passes in waves 14–17. It fails in waves 18, 19, 25, 26.**

**In that same window the total fell 14 → 12.**

> **A GENUINE REGRESSION ARRIVED WHILE THE HEADLINE NUMBER MOVED IN THE GOOD
> DIRECTION, AND SO NOBODY SAW IT.** Three seeds were fixed and one broke in the
> same window; the aggregate netted to *"improving"* and **absorbed the
> regression whole.**

★ **This is the exact composition-defect shape of every other finding this
week** — no wrong site, nothing lied, the number was accurate. **It simply
answered a different question than the one whose answer we needed.**

★★ **Seed 90 is not an anonymous seed.** It is one of the campaign's **two Row A
specimens**, and it carries a **registered fork-marker** (failing *outside* the
rescue-refused set — evidence for a distinct mechanism). **Its sibling, seed 71,
sits in the permanent core.**

## §3 — WHAT THIS DOES **NOT** ESTABLISH

- **Not** what caused seed 90's regression. The window is **between wave17 and
  wave18**; the commits are the next read. ★ **A window is not a cause.**
- **Not** that the 10 core seeds share a mechanism. **They may be ten unrelated
  bugs.** ★ *"They all fail"* is a property of the report, not evidence of a
  common cause — assuming otherwise would be exactly the error this document
  criticises.
- **Not** that the 4 fixes were deliberate. They may be incidental to other work.

## §4 — WHAT TO ADD, AND IT IS SMALL

**`derived.py` gains a cross-wave mode:** given N waves, report **FIXED / NEW /
PERSISTENT** rather than N independent counts.

- ★ **NEW ≠ ∅ is a regression alarm that fires even when the total FALLS.** That
  single rule would have caught seed 90 four waves ago.
- **PERSISTENT is the standing worklist** the campaign never had.
- **Cost: set arithmetic on data already on disk.** No schema change, no new
  field, no run.

★ **And it needs the identical-seed-set precondition asserted, not assumed** —
the comparison is meaningless across different seed sets, and the tool must
**refuse** rather than silently compare, per its own law.

## §5 — THE LESSON, WHICH IS THE DAY'S LESSON AGAIN

> **A HEADLINE NUMBER MOVING THE RIGHT WAY IS NOT EVIDENCE THAT EVERYTHING UNDER
> IT MOVED THE RIGHT WAY.** Ask what the aggregate would look like if half its
> population improved and half regressed — **and if the answer is "the same," the
> aggregate cannot gate anything.**

This is the fourth instance this week of a correct, honest measurement answering
the wrong question: `calls` without `emissions`, `travel_timeouts` without kind,
`rescue_fired` uncrossed, and now **a pass count without its identity.**
