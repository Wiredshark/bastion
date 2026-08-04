# AUTON-2 — DESIGN SKELETON (Fable basics recorded; flesh-out REQUIRES reads not yet done)

**Assigned DECISIONS #62.** Fable set the basics; the flesh-out is mine. **This
file records the basics and NAMES THE READS THE FLESH-OUT NEEDS — it is not the
finished design, and it should not be built from as-is.**

> ⚠ **HONEST STATE: the constraints below are RULED. The design decisions that
> depend on reading the arbiter and the need-accumulation sites are NOT MADE.**
> Anyone continuing this starts at §3.

## §1 — THE SHAPE (Fable, ruled)

**Needs become REAL DRIVES in the existing arbitration.** Extend
`Drive { Work, Flee, Idle }` with **`Rest`** and **`Eat`**, entering the **same
arbiter** that already picks Work/Flee.

> **★ ONE ARBITRATION, NEW COMPETITORS — NOT A SECOND SCHEDULER.** A parallel
> needs-scheduler would have to be reconciled with the existing one on every
> tick, and every reconciliation is a place for the two to disagree. Competing
> inside the existing arbiter means the tie-break already exists and is already
> tested.

## §2 — CONSTRAINTS (ruled, not negotiable)

| # | constraint | why |
|---|---|---|
| a | **DETERMINISM STORY FIRST** — need accumulation is per-tick arithmetic from sim state. **No wall-clock.** Thresholds are constants. | determinism-by-construction; the live game already lost this once to OS entropy in `tick_rng` |
| b | **HYSTERESIS BY DESIGN** — enter-Rest and exit-Rest thresholds **DIFFER** | every colony sim that skipped it got oscillation. A single threshold makes a colonist flip drives on the tick it crosses back |
| c | **PREEMPTION USES THE EXISTING FLEE-GATE** — #63's work-tick gate generalises: a colonist in Rest/Eat doesn't work | the gate exists and is tested; it gains two drives rather than a sibling |
| d | **THE TWO EXPECTED-REDS ARE THE ACCEPTANCE TESTS** — preempt's `preempted_rested`, b73's `ate`. Their fingerprints flipping green is **the row's own registered prediction** | the tests were written before the feature; they are waiting in place |
| e | **OBSERVABILITY BUDGET STATED UP FRONT** — drive-transition counts are **per-event, not per-tick reads** | ★ the observer-effect law, measured this session: per-cell per-tick diag reads perturbed a bit-reproducible run. **Reading is not passive** |
| f | **FR15 PAIRED A/B** — rest/eat time competes with work time; the colony economy WILL shift | the A/B measures the shift **against design intent**: colonists rest when depleted — **not never, not constantly** |

**★ Constraint (f) is the one with a trap in it.** "The economy shifted" is not a
failure — it is the *point*. The A/B must be read against **intent**, not against
"nothing moved". Row A/B taught the opposite reflex this session and it does not
transfer here: a report-only row must not move the economy; **AUTON-2 must.**

## §3 — ★ THE READS THE FLESH-OUT REQUIRES (NONE DONE — start here)

1. **The arbiter's scoring function.** Where do Rest/Eat rank against Work/Flee,
   and **why**? Read the existing Work-vs-Flee tie-break before proposing where
   two new drives sit in it. *A ranking proposed without reading the comparison
   is a guess dressed as a design.*
2. **Need-accumulation sites.** Do need values already exist (B7-NEEDS-MOOD)? If
   so **read what they already do** — the same trap as `stuck_strikes`, which
   turned out to be computed every cycle and discarded. **Check for an existing
   producer before adding one.**
3. **The flee-gate's actual predicate** (#63's work-tick gate) — generalising it
   requires reading what it currently gates on, not what its name suggests.
4. **`Drive`'s definition and every match site.** Adding two variants touches
   every exhaustive match; enumerate them before proposing the extension.
5. **The dossier's prior art**, applied concretely rather than cited — DF/RimWorld
   hysteresis and need-decay shapes, mapped onto the constants proposed in (6).
6. **Threshold constants with their TUNING STORY** — and per this session's
   standing practice, **any unvalidated default is marked unvalidated at build
   time**, so the A/B judges it rather than the author reasoning it into being
   correct.
7. **The planted-failure test**, per the acceptance framework: what deliberate
   break must this row detect?

## §4 — WHAT IS ALREADY TRUE AND SHOULD NOT BE RE-DERIVED

- **Needs are inert today** — pre-registered as a playthrough prediction and
  confirmed: this is AUTON-2's job, and its failure was *scheduled* rather than a
  surprise.
- **The two expected-red scenarios exist and are waiting** — they are acceptance
  tests, not new work.
- **Live sessions are now a recurring gate** (#62) — AUTON-2 gets a scored live
  session, and needs-behaviour is a *player-observable* claim: *"my colonists
  slept when tired and ate when hungry, and didn't stop working to do it
  constantly."*
