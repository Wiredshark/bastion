# PACKET-CRAFT CHECKLIST

**Standing lines. Check every packet.** *Each entry was adopted because skipping it
cost a rework, and each is one question at spec time.*

**Promoted from a private memory to this file 2026-08-11 (Fable-ruled)** — *it had
been referenced by two lanes as a shared artifact while existing in one person's
notes, which is exactly the silently-costly class entry 3 gates on.*

---

## 1 · LIVE-EMIT DECLARATION — *DECISIONS #88*

**Every new harness accessor declares one of:**

    /// LIVE-EMIT: ported (FLAG)
    /// LIVE-EMIT: harness-only -- <reason>

**Tag on touch, never a retro-sweep.** *Converts "harness instrumented, live path
bare" from a recurring surprise into a greppable ledger.*

★ **Also name the TEMPORAL SHAPE: SNAPSHOT or ACCUMULATOR.** *`ever` is not `now`,
and stating it at write time is free.* **A snapshot and an accumulator over the same
predicate are not interchangeable as existence checks** — one such swap produced a
2.43× undercount across 48 seeds.

---

## 2 · PLOT/PLAN DATA MODEL, NOT RAW POSITIONS — *DECISIONS #102 (Ben)*

> **Anything that places structures or zones is designed against a plot/plan model —
> never free positions.**

**Reason, stated as a cost: un-baking spatial assumptions later is the most
expensive rework class there is.** *Settlement form is a first-class requirement;
free placement now means paying to remove it later.*

**Applies to:** stockpile zones, building designations, farm plots, any future
placement. ★ **Prior-art rider when that arc starts: a real READ of vanilla
`site2`'s plot grammar — not a description.**

---

## 3 · WHERE IS ITS ROW? — *2026-08-11, adopted program-wide*

**Every "scoped out / deferred / acceptable for now" — in a diff OR IN A DOC — gets
the question. No row, no pass.**

**Origin:** *item 8's endurance run died on a failure mode predicted **verbatim** in
the causing method's own doc comment and scoped out of its row.* ★★ **A doc comment
cannot page anyone when its stated precondition becomes true. A ledger row can.**

★ **Applies to prereg tables and delta tables too**, not only code comments — *the
second instance the same day was a scoped-out defect named in a delta table.*

### CALIBRATION — when it GATES vs when it merely NOTES

> **Gate IFF the scoped-out condition could SILENTLY COST A RUN OR MISREAD A
> RESULT.**

*A residual whose worst case is COSMETIC gets **"row optional, noting for the
record"** and must not gate.* ★ **An elevated hit rate is not over-application when
you are excavating a corner of the tree where an era shipped reasoning-defended
residuals — judge the flag by the test, never by the count.**

### TWO CLAUSES ON THE FLAG ITSELF

1. ★★ **Before issuing the flag, NAME THE OBTAINABLE OBSERVATION THAT WOULD SATISFY
   IT.** *If you cannot name one, the flag is malformed — refine or drop it, never
   hold the gate on it.* **Founding case: a gate item required comparison to a
   counter that did not exist in the run being compared against.**
2. **A builder who CANNOT satisfy a gate item must SAY SO, not silently omit it.**
   *A gate item nobody can satisfy is a finding about the gate, and a reviewer who
   never hears it keeps issuing it.*

---

## 4 · "WHAT I WILL NOT DO AT SCORING TIME" — *2026-08-11, program-standard*

> **Every scored gate's procedure carries one, written by ITS SCORER, BEFORE the
> data exists — naming the generous moves that will feel reasonable at that gate's
> particular sunk cost.**

**The founding five (item 8 v3):**

1. **No re-baselining a fixture to make it green** — *expected values enumerated
   field-by-field BEFORE the run, or the change is not made.*
2. **No zero-as-pass** on any channel not proven reachable — *zero cases are VOID.*
3. **No partial count as a pass** — *4 of 5 cycles is a shorter run, not a passing
   one.*
4. **No failure parked as "flagged for follow-up"** — *a row, or a reported failure.*
5. ★★★ **No cross-carrying between co-resident results** — *one run can yield a
   PASSING fix claim and a FAILING endurance bar. "The fix holds while the colony
   starves" is LEGIBLE, not mixed.* **Expected to earn its keep most often.**

★★ **Why it works: naming the temptation in advance is cheaper than resisting it
later.** *A scorer who has written "I will not accept 4 cycles" cannot discover at
hour 2.5 that 4 feels close enough.*

---

## 5 · BINARY PROVENANCE — *2026-08-11; the first entry admitted by the cost rule*

> ## **THE RUNNING BUILD'S STAMP MUST BE VERIFIED AGAINST THE INTENDED PIN BEFORE A
> RUN IS SCORED — AND BUILD VERIFICATION READS THE OUTPUT'S COMPILED-CRATES LIST,
> NOT THE EXIT CODE.**

★★★★ **SOURCE-TREE VERIFICATION IS NOT BINARY VERIFICATION.**

**COST — item 8 v3: a full 2h34m run, scored to VOID at preflight.** *The reviewer
certified "the code under test is the code I greened" by diffing COMMITS
(`git diff --name-only` → docs only, zero code files) and reported that as the
running binary. The binary was six commits stale.*

**MECHANISM:**

    cargo build --profile no_overflow -p veloren-server-cli -p veloren-client --bin bastion_playtest

*`--bin bastion_playtest` restricted target selection to a binary that lives in
`veloren-client` — so despite `-p veloren-server-cli`, that package's own binary was
never in scope. The build exited 0. Only common/client crates ever compiled.*

★★★ **The command was the LABEL; the compiled-crates list was the CONTENT.**

**MECHANICAL — two checks, both cheap:**

1. **At launch:** the run's log must stamp its code identity, and that stamp is
   **compared to the intended pin before the run proceeds past founding.** *One grep,
   at minute one.*
2. **At build:** read the OUTPUT's `Compiling` lines and confirm the package you
   care about is among them. *A scoped `-p`/`--bin` filter exits 0 while silently
   excluding the binary that matters.*

## 6 · RUN MODE — **FAST UNLESS AN IMPOSSIBILITY IS NAMED** *(Ben, standing law 2026-08-11)*

> ## **EVERY TEST RUNS COMPRESSED. A REAL-TIME RUN MUST NAME ITS IMPOSSIBILITY IN
> THE PACKET.**

**The only three that count:**

1. **HUMAN-IN-THE-LOOP** — *someone is watching; perceived pacing is not in the
   fingerprint, so real time matters exactly when a person does.*
2. ★★ **A PROVEN WALL-COUPLED SUBSYSTEM** — **and "proven" means proven.** *The
   equivalence spec's hunt list is what proves it; **a suspicion of wall-coupling is
   not an impossibility, it is an unread***.
3. **THE EQUIVALENCE REFERENCE ARM ITSELF** — *the real-time half of the A/B cannot
   be compressed without circularity.*

★★★★ **TRADITION IS NOT AN IMPOSSIBILITY.** *"This has always run at real time" names
a habit, not a constraint — and it is the specific phrase this entry exists to
refuse.*

**COST — the admission rule, paid twice before the law existed:**

- ★★ **An entire evidence class was ruled out on wall-clock grounds alone.** *N=10
  repetition against the intermittent crash was declared unaffordable at a day per
  run; at ~20 minutes it is an hour's work.* **We did not decide the evidence was
  unnecessary — we decided it was too slow, and then reasoned as though it were
  unnecessary.**
- **Two endurance runs spent ~5 hours establishing what compressed runs would have
  established in ~40 minutes** — *and one of them was void on a stale binary, which
  cost the full wall clock for zero evidence.*

### ★★★ WHAT THE LAW COSTS, STATED HONESTLY

**It makes the equivalence proof load-bearing for the entire test programme.**
*Every scored result afterwards inherits it.* ★★★★ **Which is why that proof's
PLANTED FAILURE is not optional** — *a comparison that has never been shown able to
fail certifies nothing, and here it would certify everything.*

★★ **And the revalidation trigger is a standing GATE, not a reminder:** *a change to
the shell (tick loop, IO, scheduling, anything wall-adjacent) **pauses fast runs
until the proof re-passes**.* **Never silent continuation** — *an equivalence proof
is a claim with an expiry date, and the trigger is what stops it decaying into an
assumption.*

## MAINTENANCE

**This file is the single source.** *Any lane's private notes on packet craft should
POINT here, never fork it.* **Add an entry only when skipping it has cost a rework,
and record which one.**
