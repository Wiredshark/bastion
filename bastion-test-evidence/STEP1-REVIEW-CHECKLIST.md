# STEP 1 REVIEW CHECKLIST — written BEFORE the fixtures land

**Reviewer-on-call prep.** ★ **Same pattern as the arbiter's interpretation table:
when it lands, the review is a LOOKUP, not a fresh derivation.**

## §1 — WHAT STEP 1 ALREADY PROVED (do not re-review)

★ **The override mechanism is DONE and verified both ways** (`5f8cdf1392`):

| | |
|---|---|
| **positive** | `BASTION_AUTON2_MOOD_OVERRIDE` set → decay reads back 0.03/0.04 → **hunger crosses 0.2 NATURALLY at tick 511** → `natural_interrupt_reached = true` |
| **negative** | unset → reads the **exact shipped** 0.0003/0.0004 → never crosses → `matches_shipped_when_unset = true`; **real asset md5 unchanged** |

★★ **The negative control is what makes the positive mean anything.** **Don't ask
for it again.**

## §2 — ★★★★★★★ WHAT STEP 1 STILL OWES: **THE MACHINERY COMPLETING**

> **Reaching the band is proven. ARRIVING AND SLEEPING is not.**

★★★ **Today established that a rest can fail entirely downstream of the need:**
the watchdog releases, the sweep removes, the cooldown rate-limits — **all
correct, all specified, and the colonist never sleeps.** *(ENDURE.)*

**So the fixture must show: need crosses → colonist travels → ARRIVES → sleeps →
`rest` restored to `comfort + SLEEP_MARGIN` → resumes work.**

## §3 — ★★★★★★ THE TRAP I WILL CHECK FIRST

> **If the fixture's bed is nominally reachable but the colonist fails to ARRIVE,
> the fixture goes RED for a reason that has NOTHING to do with needs.**

★★★ **The fixture MUST distinguish "the need machinery failed" from "the colonist
couldn't get there."** **Those are different rows** — one is AUTON-2, the other is
the travel row — **and a fixture that conflates them will be read as a needs
failure and send the build in the wrong direction.**

★ **What I'll look for:** an assertion that **separates arrival from restoration.**
*Minimum: did the colonist reach the bed cell (or its stand-at), and separately,
did `rest` rise?* ★★ **`arrived == false` with `rest` unchanged is a TRAVEL result,
not a needs result, and must not be reported as the latter.**

★★ **And the bed must be reachable BY CONSTRUCTION, not by luck** — ★ note that
`preempt_scenario`'s PHASE-2 bed is a **floating slab with no route up**,
*deliberately unreachable*. **Step 1 needs the exact opposite, and it should be as
deliberate.** *Flat ground, short path, no seam.*

★★★ **Seed 7's y-pinning was at a FIXTURE'S OWN FLATTENED-PLATEAU SEAM.** ★ **If
step 1's bed sits near a seam, the fixture inherits a known artifact.** *Check
where it's placed, not just that it's placed.*

## §4 — THE ACCEPTANCE ITEMS, PRE-STATED

| # | check | pass condition |
|---|---|---|
| 1 | **band reached naturally** | already proven — **regression only** |
| 2 | ★★★ **colonist ARRIVES** | asserted **separately** from restoration |
| 3 | ★★ **sleep completes** | `rest` reaches **`comfort + SLEEP_MARGIN`**, the exit that owns the need boundary |
| 4 | **work resumes** | *the loop closes — a colonist that sleeps forever is not a success* |
| 5 | ★ **`preempted_rested` / `ate`** | **the registered prediction: these flip GREEN** |
| 6 | ★★★ **planted-failure** | **disable the override ⇒ the case must go RED** *(a test that cannot fail is not one)* |
| 7 | **ENDURE regression** | ★ **`thrash_bounded (1..=3)` still passes** — *untouched by step 1* |

## §5 — ★ BUDGET AND PLACEMENT — THE TWO I GOT WRONG TODAY

- ★★★ **PLACEMENT: read the run structure, not narrative time.** *"End of run" was
  a LABEL* — a later phase cleared the state and both new diags came back **empty
  on their calibration seeds.** **Whatever step 1 captures, check WHERE against
  what actually runs after it.**
- ★★ **Per-EVENT, not per-tick.** *The observer-effect bisection indicted per-cell,
  per-tick reads.* **Drive transitions are events.**

## §6 — ★★ WHAT I WILL **NOT** ASK FOR

- **Re-proof of the override.** **Done, both directions.**
- ★ **A unification assertion.** **Step 1 is the FIXTURE; unification is step 3**
  — *a step-1 fixture asserting unified behaviour would be testing code that
  doesn't exist.*
- **The travel fix.** ★ **Explicitly out of scope and named in the design** — *a
  unified need-drive sends colonists to MORE destinations, and that risk was
  registered before the build, not discovered during it.*
