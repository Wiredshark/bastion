# AUTON-2 STEP 3 — THE UNIFICATION DESIGN PASS

**Design only.** 5b builds after step 1's fixtures prove the band is reachable.
**All cites `5f8cdf1392` unless noted.** ★ **READ claims carry a line; everything
else is marked UNVERIFIED.**

## §1 — ★★★★★ GUARD-6, FULLY ENUMERATED (the "read every site before removing any" obligation)

**There is ONE predicate and FIVE consumers. Removing the predicate without all
five is worse than either state.**

| # | site | what it does | on unification |
|---|---|---|---|
| **P** | `is_labor_hold_self_job` **810-816** | **THE definition** — `RestAt \| EatFrom \| Despond` | **stays**; its MEANING changes from *"exempt from selection"* to *"which Drive is executing"* |
| 1 | arbiter selection **8838-8843** | `continue` — colonist on a self-job is **skipped entirely** | ★★★ **THE retirement site.** Becomes: the arbiter SELECTS the need-drive rather than stepping around it |
| 2 | `auton_travel_ok` **11242-11250** | self-job travel fires **UNGATED** | ★ **must become "gated on the drive the arbiter chose"** — otherwise a Flee cannot interrupt travel to a bed |
| 3 | `auton_work_ok` **12178-12192** | self-jobs **bypass** the Work-drive gate | ★ same: bypass becomes *"this IS the current drive"* |
| 4 | Despond carve-out **9237-9244** | despondent colonist still preempts for eat/sleep | ★★ **preserved by construction** once needs are drives — *it stops being a carve-out and becomes ordinary ranking* |
| 5 | ★ **unit test `16689-16700`** | **asserts all three ARE labor-hold, and `Designated` is NOT** | ★★★★ **THE SPEC OF CURRENT BEHAVIOUR.** *Must be rewritten, not deleted* — **a test that no longer compiles is not a passing test** |

> ★★★★★ **Site 5 is why this enumeration was worth doing.** *A test asserting the
> mechanism as intended existed for GUARD 6* — **the same check that killed §4c.**

## §2 — ★★★★★★★ THE IDENTITY FAMILY: **1**, AND UNIFICATION MAKES IT NEARLY FREE

§4d named three. ★ **The choice is decided by a property of unification itself:**

> **`RestAt` gets a fresh job id per retry BECAUSE self-jobs bypass the arbiter** —
> `preempt_pending` must *insert* a pre-claimed job precisely because nothing
> would ever *select* one. **Unification removes that reason.**

★★★ **Once a need is a Drive, the arbiter RE-SELECTS it instead of the preempt
pass RE-CREATING it — so the entry can persist, and `stuck_strikes` accumulates
the way Mine's does.** **Family 1 stops being a change and becomes a consequence.**

**Obligations, met up front as required:**

- ★★ **Family 1's orphan-sweep proof.** The sweep (**8669-8681**) exists because
  a pre-claimed self-job that loses its claimant **can never be re-claimed** —
  *"pre-claimed self-jobs never enter the claim selection"* (**9166**).
  ★★★ **Unification dissolves the premise: a self-job the arbiter can select is
  re-claimable by definition.** **The sweep narrows to a genuine-orphan case (no
  live colonist wants it) rather than being the only reaper.**
  ★ **PROOF OBLIGATION, stated: demonstrate no self-job can become unclaimable
  AND unswept.** *That is a real proof and it belongs in the build.*
- ★ **Family 2's store's-unit fit — now moot, and that is the point.** A
  `(colonist,target)` accumulator would be **a new producer**, and per today's
  corollary **a new producer changes the DENOMINATOR of every count over the
  field it feeds.** ★★ **Family 1 adds no producer at all.** *Choosing it avoids
  an obligation rather than discharging one.*
- **Family 3 (purpose-built self-job rescue)** stays the **fallback** if the
  sweep proof cannot be closed.

## §3 — ★★★★★★ THE ARBITER RANKING, AND A DEFECT IN MY OWN FORMULA

**Read (`8884-8955`, consts `1689-91`):** `URGENCY_FLEE 1.0 · WORK 0.5 · IDLE 0.1`;
pick max; commit for `ARB_COMMIT_SECS`; **Flee preempts the commitment per-tick,
same-tier does not.**

**§2 of the spec proposed:**

```
need_urgency = URGENCY_WORK + (URGENCY_FLEE - URGENCY_WORK) * severity
severity     = shortfall(value, interrupt) / interrupt      // 0..1
```

> ## ★★★★★★★★ THE ENTRY AND EXIT BOUNDARIES ARE THE SAME POINT
>
> **At `severity → 0` the need ties `Work` exactly (0.5).** ★ **So the condition
> that STARTS a rest is the same condition that ENDS it** — *a colonist restored
> to precisely the interrupt threshold sits on a tie.* **That is the oscillation
> §3 of the spec was written to prevent, and my own formula reintroduces it.**

★★★ **THE FIX IS ALREADY BUILT: exit on `comfort + SLEEP_MARGIN` (11827), not on
the urgency crossing.** **The sleep completes when restored to comfort, which is
ABOVE the interrupt threshold** — ★ **so the hysteresis is the GAP between
`interrupt` (entry) and `comfort + SLEEP_MARGIN` (exit), and it already exists.**

★★ **Ruling: the need-drive's exit is OWNED BY THE JOB'S OWN COMPLETION, not by
re-ranking.** `ARB_COMMIT_SECS` covers anti-thrash *between* selections; **the
sleep's own margin covers the need boundary.** **Do not add a third mechanism.**

## §4 — ★ THE RANK ITSELF, WITH ITS REASON

**Band (0.5, 1.0), strictly between Work and Flee:**

- ★ **A severe need must outrank Work** — *a starving colonist stops mining.*
- ★★ **and must NEVER outrank Flee** — ***a starving colonist still runs from a
  storm.*** **Approaches 1.0 asymptotically, never reaches it.**
- ★ **At the threshold it ties Work and loses** *(max picks the incumbent under
  commitment)* — **correct: a just-crossed need should not abandon work
  mid-swing.**

★★ **`Flee` keeps its per-tick preemption over the commitment (802).** **A need
must not gain that** — *needs are urgent, not instantaneous.*

## §5 — ACCEPTANCE, AND WHAT WOULD FALSIFY THE DESIGN

- ★★★ **Planted-failure: disable the need→urgency mapping; the planted case must
  go RED.** *A test that cannot fail is not one.*
- ★ **GUARD-6 completeness: all five sites, or none.** **A guard removed at one
  site and left at another is worse than either state.**
- ★★ **Identity: `stuck_strikes` on a re-selected `RestAt` must ACCUMULATE across
  retries** — ★ **that is the whole point of family 1, and it is directly
  measurable** *(5b measured 0 across 660 ticks today)*.
- ★★★ **FR15 at full strength.** **Direction stated up front: throughput DOWN by
  roughly the fed/rested fraction.** ★ *"The economy shifted" is the POINT here,
  the exact opposite of Row A/B's bar — read the A/B against INTENT: colonists
  rest when depleted, not never and not constantly.*
- ★ **GATE FIELDS: needs/rest observability — currently INSTRUMENT-GAP** until
  the AUTON-2 window lands.
- ★★ **EXISTING TESTS/FIXTURES:** `preempt_scenario` *(force-set needs; ENDURE's
  `thrash_bounded (1..=3)`)*, `bed_scenario`, `needs_scenario`, and ★ **the
  `is_labor_hold_self_job` unit test at 16689** — **all four must be re-read
  before the build, and the last one REWRITTEN rather than deleted.**

## §6 — WHAT THIS DESIGN DOES **NOT** CLAIM

- ★ **Not** that ENDURE changes. **It is correct, tested, and specified** —
  §4c's error was proposing otherwise. **Unification must leave
  `thrash_bounded (1..=3)` intact for the genuinely-unreachable case.**
- **Not** that the travel defect is fixed by this. ★ **A unified need-drive sends
  colonists to MORE destinations, not fewer** — *the standing risk named before
  the build, not discovered during it.*
- ★ **Not** verified that site 4's carve-out survives untouched — **UNVERIFIED,
  and it is the subtlest of the five.** *Read it against the new ranking before
  trusting the "preserved by construction" claim above.*
