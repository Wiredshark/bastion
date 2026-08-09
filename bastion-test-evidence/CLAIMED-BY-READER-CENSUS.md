# `claimed_by` READER CENSUS — THE ARTIFACT DECISIONS #77 CONDITIONS ON

**Every READ of `Job::claimed_by` in `bastion_jobs.rs` at `dcbe6f17c7`, typed.**
★ **35 matches; 11 are comments; 24 are substantive readers.**

## THE TYPING RULE

| type | question it asks | correct under a RETAINED claim during suspend? |
|---|---|---|
| **OWNERSHIP** | *whose job is this?* | ★★★ **YES** |
| **AVAILABILITY** | *can this job be claimed?* | ★★★ **YES** — *a suspended job is genuinely not available* |
| ★★★★★ **EXECUTION** | *is someone actively WORKING this?* | ★★★★★ **NO — a suspended colonist is not working** |

## ★★★★★★★ RESULT: **21 OF 24 ARE CORRECT. THREE ARE EXECUTION-TYPED.**

### THE 21 — no action, listed by class

**OWNERSHIP (5):** `5116` *(cancel-region release — a suspended job's owner should
still be released)* · `5294` *(bed-slot clear, `slot.occupant == j.claimed_by` —
★★ and RIGHT under suspend: the bed stays reserved for a colonist who intends to
return)* · `13569`, `14121` *(release guards — "is it still mine?")* · `14914`
*(claim eligibility)*.

**AVAILABILITY (14):** `8762`, `8793`, `8888`, `14399`, `16647`, `16710`, `16767`,
`16773`, `16835`, `17039`, `17084`, `16584`, `16588`, `14383`.
★ *Every one asks `.is_none()` or skips claimed jobs — a suspended job counts as
taken, which is the answer they want.*

**REPORT-ONLY (2):** `9194` *(a diag field inside `info!`)*, `17015`
*(emergency-route bookkeeping; emergency jobs are never self-jobs)*.

## ★★★★★ THE THREE EXECUTION-TYPED READERS

### 1. `audit()` — `5521` — ★★ **KNOWN**

**`claims_distinct` collides when one uid holds two claims.** ★★★ **Already the
subject of branch 2: the invariant re-derives to "distinct ACTIVE claims,"
because it was written when `claimed ⇒ active` held by construction.**

### 2. CLAIM SWEEP — `16133` — ★★★★★ **KNOWN, AND OWED IN EVERY BRANCH**

**`alive = active_jobs.get(e).is_some()`** — ★★★ **strips the claim from a
suspended job on a fixed cadence.** ★ **Ruled a DEFECT in its own right (#77): its
proxy was exact only while `claimed ⇒ active` held.**

### 3. ★★★★★★★ DISPERSION `claimed_pos` — `16982` — **NEW. NOBODY HAS NAMED THIS.**

```rust
// 3. DISPERSION -- claims (standing + taken this pass) repel new claims within 2
//    XY blocks, spreading a work crew across the frontier instead of stacking.
let mut claimed_pos = board.jobs.values().filter(|j| j.claimed_by.is_some())
                          .map(|j| j.pos).collect();
```

> ★★★★★ **A SUSPENDED SELF-JOB'S POSITION WOULD REPEL WORK CLAIMS WITHIN 2 XY
> BLOCKS. A sleeping colonist's BED pushes the work crew away from it.**

★★★ **This is a genuine EXECUTION question answered wrong** — *the dispersion
penalty exists to stop colonists stacking on one cell, and a bed with nobody
standing at it is not a stacking hazard.*

★★ **IMPACT — stated honestly:** *not a correctness break; a SCORING penalty.*
★★★★★ **But it is BEHAVIOURAL: it changes which colonist claims which job, and
the surrounding code is explicitly determinism-hardened (`DET-COL-JOB-001`,
claim order pinned by Uid, `ARCH-003` score ties pinned by JobId).**
★ **A field feeding that scoring is not cosmetic — and the exact-match fan would
see it.**

## ★★★★★ VERDICT — **CLEAN, WITH ONE NAMED ADDITION**

> ★★★ **Of 24 readers, 21 are correct by construction. Of the 3 that aren't, TWO
> were already on the table** *(the audit re-derives; the sweep fix is owed in
> every branch)*. ★★★★★ **The census adds exactly ONE new item, and it is small
> and of the same shape as the sweep.**

**So BRANCH 4 HOLDS** *(5b's design + the sweep fix)*, **with `16982` as a third
line:** ★★ *filter the dispersion set to ACTIVELY-WORKED jobs, or declare the
penalty's inclusion of suspended jobs as intended.* ★ **Either is one line; what
matters is that it is CHOSEN.**

### ★ WHAT WOULD HAVE MADE IT DIRTY

**A reader that gates BEHAVIOUR on `claimed_by` meaning "working" and whose wrong
answer compounds** — *e.g. a starvation or fairness accumulator, or anything
feeding the arbiter's own selection.* ★★★ **None found.** ★★ **The dispersion
penalty is the closest, and it is a soft score rather than a gate.**

## ★★ METHOD NOTE

★★★★★ **The census is only as good as its enumeration, so it was built from a
grep of ALL `claimed_by` occurrences and then NARROWED by hand** — *not from the
sites anyone remembered.* ★★★ **That is the same move that found the claim sweep:
the dangerous reader is never the one you can recall.** ★ **11 comment-matches
were excluded by reading them, not by pattern.**
