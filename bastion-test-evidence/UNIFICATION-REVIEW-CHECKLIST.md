# UNIFICATION REVIEW CHECKLIST — written BEFORE the build

**Sent to 5b at build start, not at review.** ★ **So the review is a lookup, and
every item can be objected to while objecting is cheap.**
★★ **Six of my own mistakes appear below as named review points — they are the
items most likely to recur, because I made them on this row.**

## §0 — WHAT I WILL NOT RE-REVIEW

★ **The step-1 fixture** *(7 items, accepted, item 2's limit recorded)*.
★ **The discriminator** *(Class A vs Class C, precondition-first, live specimens)*.
★ **The override mechanism** *(both directions, negative control included)*.
**Done. Not asking again.**

## §1 — ★★★★★ THE FIVE GUARD-6 SITES, EACH DISPOSITIONED

**One predicate, five consumers. ALL OR NONE** — *a guard removed at one site and
left at another is worse than either state.*

| # | site | the check |
|---|---|---|
| **P** | `is_labor_hold_self_job` **810** | **kept, MEANING changed** — *"exempt from selection" → "which Drive is executing."* ★ If it was deleted, where did its three kinds go? |
| 1 | arbiter skip **8838** | ★★★ **the retirement site.** **Does the arbiter now SELECT the need-drive?** *A `continue` that merely moved is not a retirement.* |
| 2 | `auton_travel_ok` **11242** | ★★★★★ **MUST become drive-gated.** **CHECK BY TEST: can a Flee interrupt travel-to-bed?** *If not, this site was copied, not converted.* |
| 3 | `auton_work_ok` **12178** | bypass → *"this IS the current drive."* ★ **The `continue` there is a SUSPEND** *(claim held, progress paused)* — **it must stay a suspend.** |
| 4 | Despond carve-out **9237** | ★★★ **see §2 — this is the one I got wrong.** |
| 5 | ★★ **unit test 16689** | **REWRITTEN, not deleted.** ★ *A test that no longer compiles is not a passing test.* **Does the new one assert the NEW contract, or was the file just made to build?** |

## §2 — ★★★★★★ SITE 4: I SAID "PRESERVED BY CONSTRUCTION." IT ISN'T.

**The carve-out does a JOB/CONDITION SPLIT:** *insert deadline into
`despond_resume` → **destroy** the Despond job → re-create it later from the
table.* ★★ **Family 1 removes the destroy-and-recreate that this depends on.**

**CHECKS:**
- ★★★ **`despond_resume` should be DELETED, not adapted.** *Its 4 sites: decl
  `4337`, read `8857`, remove `8862`, insert `9317`.* ★ **If it survives, why?**
  *A side table exists only to carry state across a destruction that no longer
  happens.*
- ★★★★★ **THE DETERMINISM GUARANTEE IS AN ACCEPTANCE ITEM, AND IT NEEDS A
  COUNTER:** *same deadline · no cooldown consumed · **no `break_chance` roll**.*
  > ★★ **Assert a ROLL COUNT, never a roll RESULT.** **A re-roll could
  > coincidentally produce the same deadline, and "same value" would pass a
  > broken build.**
- ★ **Falsifier: plant a re-roll on resume — RED on the count EVEN IF the drawn
  deadline matches.** *If a planted re-roll passes, the assertion is on the wrong
  quantity.*

## §3 — ★★★★★ THE INVARIANT'S MUST-FAIL-FIRST CREDENTIAL — VERIFY BEFORE BUILDING

**The try-to-orphan fixture asserts:** *no self-job is both unclaimable and
present.*

> ★★★★★ **RUN IT AGAINST THE CURRENT TIP FIRST AND REQUIRE RED.**

★ **Today's travel-timeout path produces an unclaimable job** *(pre-claimed
self-jobs never enter claim selection, `9166`)* — **so it MUST fail pre-change.**
★★★ **If it passes before the build, it is not testing the build**, and no amount
of green afterwards means anything.

★ **And the invariant runs in EVERY scenario, not only its own** — *an invariant
that runs only in the fixture that tests it protects nothing else.*

## §4 — ★★★ FAMILY 1: PERSISTENCE **DEMONSTRATED**, NOT ASSERTED

**The claim:** the arbiter RE-SELECTS rather than the preempt pass RE-CREATING, so
the entry persists and `stuck_strikes` accumulates.

> ★★★★★ **THE MEASUREMENT EXISTS AND HAS A BEFORE: `stuck_strikes = 0` across
> all 660 ticks of job 33, measured today.**

★★ **CHECK: on a `RestAt` that fails and retries, does `stuck_strikes` now
ACCUMULATE?** ★ **A design that says "the entry persists" and a run where the
counter still reads 0 are different claims** — *and only the second is evidence.*

★ **Also: same job ID across retries?** *That's the direct observable.*

## §5 — ★★ THE HYSTERESIS BOUNDARY — MY FORMULA'S OWN DEFECT

**My `need_urgency = WORK + (FLEE−WORK)×severity` puts ENTRY and EXIT at the same
point** *(at `severity → 0` the need ties Work exactly)*.

★★★ **CHECK AT THE BOUNDARY:** **the exit must be owned by the JOB'S OWN
COMPLETION (`comfort + SLEEP_MARGIN`, `11827`), NOT by the urgency crossing.**
★ **If a third anti-thrash mechanism appeared, that's a flag** — `ARB_COMMIT_SECS`
covers between-selections, the sleep margin covers the need boundary. **Two, not
three.**

★★ **And the re-tune invariant holds:** **rest decay < `BED_REST_RECOVERY_PER_SEC
× WORST bed quality`** *(Bedroll 0.6 ⇒ ceiling 0.012/sec)*. ★ **Doesn't bind at
current targets (~26× headroom) — but it fails SILENTLY, so state the worst tier.**

## §6 — ★ EVERY NEW READ PRICED

★★★ **State the read budget for anything added:** *per-event or per-tick? per-cell?*
**The observer-effect bisection indicted PER-CELL, PER-TICK.**
★ **And PLACEMENT: read the actual run structure, not narrative time** — *"end of
run" was a LABEL, and a later phase cleared the state, which cost the instrument
window a silent empty on its own calibration seeds.*

## §7 — ACCEPTANCE

- ★★★ **b73's EAT chain goes GREEN** — `ate → eat_conserved / paused / resumed`.
  **THIS ROW'S bar.** *(I omitted b73 from the design's fixture slot; it's in now.)*
- ★ **`preempt_scenario` still passes** — **ENDURE's `thrash_bounded (1..=3)`
  intact for the genuinely-unreachable case.** ★★ *One chain, two cases, and only
  one of them was ever a bug.*
- ★★ **Planted-failure: disable the need→urgency mapping ⇒ the planted case goes
  RED.**
- **FR15: throughput DOWN by roughly the fed/rested fraction.** ★ *"The economy
  shifted" is the POINT here — read the A/B against INTENT: colonists rest when
  depleted, not never and not constantly.*

## §8 — ★★ SIX OF MY OWN MISTAKES, AS REVIEW POINTS

1. ★ *"Preserved by construction"* **for site 4** — **it wasn't.**
2. ★ **My urgency formula** — **entry and exit at the same point.**
3. ★ **b73 omitted** from the fixtures slot **on the row it tests.**
4. ★ **Both halves of the §7 prediction wrong** — *one already green, one
   unsatisfiable by the step I attached it to.*
5. ★ **"End of run" was a label**, not a location.
6. ★ **I described ANALYSES in the vocabulary of IMPLEMENTATIONS** — *if this doc
   says "the classifier," it means a Python script over corpus JSON, not code.*

> ★★★ **All six were caught by another lane. Assume the seventh is in here too.**
