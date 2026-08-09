# SITES 4+6 FINAL ROW — REVIEW CHECKLIST, WRITTEN BEFORE THE LANDING

**Sent to 5b while they build, not at review.** ★ **So the review is a lookup and
every item can be objected to while objecting is cheap.**
★★★ **Six items + one binding falsifier + one number.**

## §0 — WHAT I WILL NOT RE-REVIEW

★ **The design itself** *(#78 branch 4: pointer + retained claim — settled on a
census, not taste)*. ★ **The three ENDURE fixes** *(each verified against
`preempt_scenario` before landing)*. ★ **Sites 1/2/3.** **Done.**

## §1 — THE SIX ITEMS, EACH WITH ITS OWN CHECK

| # | item | the check |
|---|---|---|
| **1** | **suspend as built** | ★ **NO CHANGE EXPECTED.** *If it moved, why?* |
| **2** | ★★★★★ **claim sweep `16133`** | **does `alive` now mean ENTITY EXISTS, not `has ActiveJob`?** ★★★ **PLANTED CASE REQUIRED: a genuinely despawned claimant MUST still release** — *the fix must not lose the behaviour the sweep exists for* |
| **3** | **`claims_distinct` → distinct ACTIVE** | ★★ **is the re-derivation in the AUDIT, or did the call site change instead?** *The invariant should read the execution side; the audit is where it belongs* |
| **4** | ★★★ **dispersion filter `16982`** | **filtered to ACTIVELY-WORKED jobs, named-case comment.** ★★★★★ **THE FAN EXPECTS NO MOVEMENT FROM THIS SITE — the filter is what keeps the bar blanket** |
| **5** | **per-kind gate `10180`** | arrival tolerance: **work kinds only.** ★ *Plus the rest fixture's assertion:* `distance-to-bed at first recovery <= ARRIVE_DIST` |
| **6** | **per-kind gate `12248`** | `route_exhausted`: **work kinds only**, comment citing the STORE'S UNIT |

★★ **Then site 5 — the unit test, rewritten LAST against what 1-6 actually do.**
★ *A test that merely compiles is not a passing test.*

## §2 — ★★★★★★★★ THE BINDING FALSIFIER

> ## **FIXTURE 1's TICK-263 VIOLATION ON SEEDS 52/54 MUST DISAPPEAR.**

★★★ **Its mechanism is now known: the claim sweep strips at `%ARB==3`, the orphan
sweep deletes at `%ARB==9` — six ticks apart.** ★★★★★ **Item 2 kills that chain.**

★★ **IF IT SURVIVES: the explanation is WRONG and we are not done.** ★ *Registered
before the build so it cannot be explained away after.*

★★★ **AND `settle_invariant_violations` STAYS AS BUILT** — *still the free
detector, still 0-means-clean.* ★ **The orphan sweep needs NO change** *(a
suspended job keeps its claim and never enters `orphans`)*.

## §3 — ★★★ WHAT MUST **NOT** HAVE MOVED

- ★★★★★ **`preempt_scenario`'s ENDURE numbers** *(`endure_dug` 13-14,
  `thrash_bounded 1..=3`)* — **the three fixes are load-bearing and a seventh
  costume is exactly the risk.**
- ★★ **`claims_distinct` on `b4_scenario`** — *after item 3, TRUE again.*
- ★ **Any `claimed_by` reader outside the three typed as EXECUTION** — *the census
  says 21 of 24 are correct by construction; a change touching them is a scope
  leak.*

## §4 — ★★ THE NUMBER

**`b4_scenario` seed 1337 → `claims_always_distinct`.** ★★★ **Confirmation, not a
decider** *(the census settled the design)* — **but I want it on the record, and a
TRUE before item 3 would itself be interesting.**

## §5 — ★★★★★ MY OWN MISTAKES ON THIS ROW, AS REVIEW POINTS

★ **These are the items most likely to recur, because I made them here.**

1. ★★★ **I predicted a sweep would delete suspended jobs in 0.5s. WRONG for their
   design** *(they never let the claim go)* — **but right about the outcome via a
   DIFFERENT sweep.** ★★ **A correct conclusion from a wrong mechanism is still a
   wrong mechanism. Check WHICH sweep any claim refers to.**
2. ★★★★★ **I said "the separate field needs NEITHER sweep taught" — Fable
   corrected it: their own ruling had already taught the orphan sweep.** ★ **I
   overstated a comparison in my own favour while arguing from mechanism.**
3. ★★★ **I told 5b to STOP a rework that a later ruling might obsolete — correct —
   but only because a message crossed. ★ Check ruling ORDER before relaying.**
4. ★★ **I imposed Fixture 2's two-colonist requirement for a risk their design
   made structurally impossible.** ★★★★★ **Withdraw requirements when the design
   removes their premise — do not make someone build a guard against a dead risk.**

> ★★★★★ **The seventh costume is in here somewhere too. It has been every other
> time.**
