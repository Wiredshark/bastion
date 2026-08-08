# AUTON-2 UNIFICATION — THE TWO ACCEPTANCE FIXTURES

**Written before the build so the build starts with its gates in place.**
Both are Fable-ruled acceptance items from `AUTON2-UNIFICATION-DESIGN.md`.
★ **Each states its falsifier IN the document.**

## FIXTURE 1 — TRY-TO-ORPHAN

**Obligation:** *demonstrate no self-job can become **unclaimable AND unswept**
under the new lifecycle.*

### ★★★★★★★ THE SPEC AS BRIEFED CANNOT BE BUILT — AND THE REASON MATTERS

The brief says *"force the claimant away by **every path the old sweep
covered**."* ★ **Measured: there are 26 `to_release.push` sites, and their
reasons are:**

    Other              22
    Completed           1
    RemovedExternally   1
    TargetChanged       1
    TimedOut            1

> ★★★ **22 of 26 share one unclassified reason.** **The release taxonomy cannot
> support a per-path argument** — *you cannot enumerate by reason what the reason
> field does not distinguish*, and enumerating 26 **sites** in a fixture is both
> impractical and stale the moment a 27th lands.

### ★★★★★ SO THE OBLIGATION CONVERTS: PER-SITE ENUMERATION → ONE SETTLE INVARIANT

> **INVARIANT (asserted at settle, over ALL self-jobs):**
> **`∀ j ∈ board.jobs where is_labor_hold_self_job(j.kind):`**
> **`j.claimed_by.is_some()` ∨ `j` is reachable by the arbiter's selection.**
>
> ★★★ **No job may be both unclaimable and present.**

★★ **This covers all 26 paths WITHOUT naming them, and survives the 27th.**
★ **A proof obligation over N call sites is an INVARIANT question, not a fixture
question** — *the fixture's job is to create conditions where the invariant would
break if the design were wrong.*

### THE FIXTURE'S ACTUAL WORK — adversarial, not exhaustive

**Drive a colonist onto a self-job, then remove its claimant by the paths with
DISTINCT downstream behaviour** *(representative, and named — not all 26)*:

| # | path | why this one |
|---|---|---|
| 1 | **travel timeout release** | the only path that nulls `claimed_by` *before* the drain |
| 2 | **colonist death mid-job** | the dead-occupant sweep's own case (**8661**) |
| 3 | **Despond carve-out removal** | ★ site 4's path — *the one this design deletes* |
| 4 | **explicit `remove_job`** (cancel/moot) | the reservation + bed-release path (**5205-5222**) |

★★★ **ASSERT AFTER EACH: the arbiter RE-SELECTS the job, and the settle
invariant holds.** ★ **Under the OLD lifecycle path 1 produces an unclaimable
job** *(pre-claimed self-jobs never enter claim selection, `9166`)* **— so this
fixture must FAIL on a pre-unification build.** *That is its planted-failure
proof.*

### ★ FALSIFIER

**If the fixture passes against a pre-unification binary, it is not testing the
change.** ★★ **Run it against the current tip FIRST and require RED.**

## FIXTURE 2 — DESPOND-RESUME DETERMINISM

**Obligation:** *the re-issue guarantee — **same deadline, no re-roll, no
cooldown** — survives the move to suspend-and-reselect.*

### THE PROPERTY, FROM THE CODE'S OWN WORDS (`9305-9320`)

> *"the CONDITION survives … the original `until`, **untouched** … re-creates
> Despond with this SAME deadline … **no roll, no cooldown — an active condition
> is not a new breakdown**. Eating genuinely PAUSES the breakdown, never ends it;
> RNG only ever STARTS one."*

### THE FIXTURE

1. **Force a breakdown** (mood below `break_minor`, sustained past
   `break_sustain_secs`) → record the assigned `until`.
2. **Drive a need past its interrupt** so eat/sleep preempts the Despond.
3. **Let the need resolve**, colonist becomes free.
4. ★★★ **ASSERT ON RESUME:**

| assertion | why |
|---|---|
| ★ **`until` is BYTE-IDENTICAL to step 1's** | *"the original `until`, untouched"* |
| ★★ **`preempt_attempts` did NOT increment for the resume** | **no cooldown consumed** — resume is not an attempt |
| ★★★ **no `break_chance` roll occurred** | **RNG only ever STARTS a breakdown** — *a re-roll on resume is a new bug wearing the old one's name* |
| **mood/`until` unchanged by the eat/sleep itself** | eating **pauses**, never **ends** |

### ★★★★★ THE RNG ASSERTION IS THE HARD ONE — AND IT NEEDS A COUNTER, NOT AN OUTCOME

★★★ **"No roll occurred" cannot be asserted from the OUTCOME** — a re-roll could
coincidentally produce the same deadline, and *"same value"* would then pass a
broken build. ★★ **Assert a ROLL COUNT, not a roll result.** *(Determinism by
construction: the roll site is keyed by `(tick, uid, episode)` per T0.33, so a
counter is cheap and exact.)*

★ **This is the day's law applied forward: an outcome that could arise two ways
does not discriminate between them.**

### ★ FALSIFIER

**Plant a re-roll on resume** *(temporarily re-issue with a fresh `break_chance`
draw)* — ★ **the fixture must go RED on the roll-count assertion, and it must go
RED even if the drawn deadline happens to match.** **If a planted re-roll passes,
the assertion is on the wrong quantity.**

## BUDGET (both fixtures)

- ★ **Settle-time reads only** — the invariant is one pass over `board.jobs` at
  settle; the determinism assertions are counters read once. **No per-tick, no
  per-cell.** *The observer-effect bisection indicted per-cell-per-tick.*
- ★★ **Fixture 1's invariant is cheap enough to run in EVERY scenario, not just
  its own** — *and it should.* **An invariant that only runs in the fixture that
  tests it protects nothing else.**
- **Additive schema window**; `--expect-new` covers the new fields.

## ★ WHAT THESE FIXTURES DO **NOT** COVER

- ★ **Not** the travel defect. **A unified need-drive sends colonists to MORE
  destinations** — named in the design, unaddressed here, and **not a gap in
  these fixtures.**
- ★ **Not** ENDURE. **`thrash_bounded (1..=3)` must still pass unchanged** for the
  genuinely-unreachable bed — ★ **add it as a REGRESSION assertion, not a new
  one.**
- **Not** the 22 unclassified `ReleaseReason::Other` sites. ★★ **Classifying them
  is its own row** — *the invariant makes the fixture independent of that work,
  which is why the conversion was worth doing.*
