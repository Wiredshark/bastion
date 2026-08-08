# BUILD INSTRUMENT — `b5_build_job_diag`, COMPOSED WITH CHOP IN ONE WINDOW

**Same JOIN shape, same fields, same end-of-run placement, same free hold-check
verification as `CHOP-INSTRUMENT-SPEC.md`.** ★ **Both diags land in ONE additive
window with ONE re-baseline** — there is no reason to pay two windows for two
instruments of identical shape and budget. **Analysis priority on arrival:
BUILD first**, per the ranking.

## §1 — THE VOID IT FILLS, BOUNDED ON BOTH SIDES

**`build_placed` fails in 6 of 11 — the corpus's most frequent clause — and all
twelve build fields read IDENTICALLY on passing and failing seeds.**

> **Build's failure is bounded: DOWNSTREAM of nothing we measure, and UPSTREAM of
> materials** (Aug-4 Mode-A: the job exists, never progresses, **never even
> requests materials**; `stone_sum = 27` on failing seeds 80/85/92, identical to
> every passing seed).

★ **The instrument's job is to fill exactly that void: what happens between "the
job exists" and "it never asked for materials."**

## §2 — ★★★★★ THE THREE STATES IT MUST SEPARATE

**This is the acceptance criterion, transferred verbatim and made build-specific:**

| state | today | must become |
|---|---|---|
| **never claimed** — no colonist ever took it | ★ indistinguishable | distinct |
| **claimed, never arrived** — took it, never reached the site | ★ indistinguishable | distinct |
| **arrived, never requested materials** — got there, then stalled | ★ indistinguishable | distinct |

> ★★★ **All three currently render as `build_placed: false` with every other field
> identical to a passing seed.** **Three different bugs wearing one output.**

## §3 — THE FIELDS (mirroring `mine_cell_diag`, plus build's own)

**One entry per Build job open at settle:**

```
pos · claimant · progress · times_offered · cycles_since_last_claim
starvation_cycles · starvation_crowded_cycles · timeouts_on_this_cell
unreachable · blocked_by
★ needs_materials · required_item · reservation     <- BUILD-SPECIFIC
```

★ **The last three are the ones that separate state 3 from states 1–2**, and they
are already fields on `Job` (`required_item`, `needs_materials`, `reservation`).
**No new engine state. A JOIN, not a measurement** — fourth instance this week.

★ **`claimant` + `progress` separate state 1 from state 2**; `timeouts_on_this_cell`
and `min_distance` (via the existing travel-timeout arrays 5b landed at
`e5a288d9cc`) say **whether the colonist ever got near**.

## §4 — ★★★★★★ SEED 71 IS A KNOWN-GOOD CONTROL INSIDE THE POPULATION

**71 is the ONE build failure that is already explained:** `stone_sum = 5` (not
27), both `b15_*claimed` **FALSE** — **its mine failed, so the stones never
existed.**

> ★★★ **So the instrument's first read comes with its own calibration case: FIVE
> zero-signal seeds against ONE whose cause is known.** **If the new fields
> cannot distinguish 71 from 80/85/92, the instrument is wrong** — and we find
> that out on the first wave rather than after a build.

★ **That is a control the chop population does not have**, and it was the
decisive count in the re-rank. ★ **Score 71 SEPARATELY in any summary** — it is
downstream of a mine failure, not a build defect.

## §5 — BUDGET

- ★ **`build_stall_jobs = 1` and `build_ok_jobs = 1` on every corpus seed** ⇒
  **1–2 entries.** Same order as chop, **two orders below `mine_cell_diag`.**
- ★★★ **END-OF-RUN, after final verdicts.** Nothing left to perturb.
- ★★★★★ **AND VERIFIED FREE:** the composed window's **hold-check on PRE-EXISTING
  fields IS the noise detector.** Old fields move ⇒ the settle-time reads
  perturbed something; old fields hold ⇒ **the presumption is confirmed as a side
  effect.** ★ **Read the hold-check AS that confirmation and say so in the wave
  notes** — *presumptively safe → measured safe, at zero cost.*
- **Reads at settle, not per tick.** The bisection indicted **per-cell, per-tick**.

## §6 — ACCEPTANCE

- ★ **PRIMARY:** for each of 61, 62, 80, 85, 92 the report **says which of §2's
  three states occurred.** *The row succeeds when the currently-empty question has
  an answer shaped like data — even if it does not yet name a cause.*
- ★ **CALIBRATION (unique to this row):** **seed 71 must classify differently**
  from the five. *An instrument that reports the same thing for a known-cause
  failure and five unknown ones has not measured anything.*
- **Planted-failure test:** a build job **claimed and progressing** must render
  **visibly differently** from one **never claimed**.
- **Regression:** passing seeds keep `b5_build_job_diag` **empty** — and ★
  **"empty means the build completed, never 'not looked at'" goes in the field's
  own doc.** *(That exact ambiguity cost me a wrong claim about `mine_cell_diag`
  this morning.)*
- ★ **GATE FIELDS:** `b5_build_job_diag` — **INSTRUMENT-GAP.** No gate claim about
  build failures is admissible until it lands.
- ★ **EXISTING TESTS/FIXTURES:** `build_ok_jobs`, `build_stall_jobs`,
  `build_stall_untouched`, `b15_*`, `stone_sum`, `tool_stone`. ★ **NONE
  discriminates** — all read identically on passing and failing seeds.
  *(Per Fable: these are **fixture descriptors dressed as diagnostics**, already
  in the report-fix backlog for renaming.)*

## §7 — WHAT THIS DOES NOT CLAIM

- ★ **Not** that the twelve existing fields are wrong. **Producers unread.**
  > **They may measure exactly what they were built to measure; that thing simply
  > isn't what decides the outcome.**
- **Not** that the five share a mechanism. ★ **The instrument may well split them
  across §2's three states** — and **that would be the result**, not a failure of
  the row.
- **Not** a fix. **An instrument**, and the fifth thing this week specced against
  a law its own absence taught us.
