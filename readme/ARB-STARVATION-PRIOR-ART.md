# ARB-STARVATION — prior-art survey (DECISIONS #53, gate (b))

**Read-only deliverable. No fix designed here.** Gate (b) must exist before any
fix design regardless of what the per-attempt trace shows.

## The problem shape, stated precisely

Task #59's hypothesis, corroborated 6/6 by its own counters: **greedy per-cycle
scoring with no penalty or cooldown after a failed attempt.** A cell that is
expensive to serve loses the score comparison every cycle while cheaper
unclaimed work exists, so it is never attempted — `starvation_crowded_cycles /
starvation_cycles ≈ 1.000` on every measured seed, kill case absent.

**This is a named, solved class.** Nothing below is novel; the value of the
survey is picking the variant that survives *our* constraints.

## ★★ FIRST: the trace must split two different problems

The literature treats these as distinct and their fixes do not substitute:

| shape | symptom in the per-attempt trace | fix family |
|---|---|---|
| **A — lost the comparison** | attempts EXIST and fail/never win | **aging / escalation / cooldown** |
| **B — never evaluated** | **ZERO attempts recorded** for the cell | **round-robin / deficit / raise the cap** |

**Shape B is live for us, not hypothetical.** `path` runs with
`peak_tick_iters == cap == 3000` — the scheduler is pinned at its iteration
ceiling. **If evaluation stops at a cap and always starts from the same place,
jobs late in the order are never scored at all**, which produces "open,
unclaimed, competitors present, 360 cycles" with no scoring defect whatsoever.

> **Applying an aging fix to a shape-B problem does nothing and looks like a
> failed hypothesis.** The trace's zero-attempts-vs-failed-attempts distinction
> is what picks the family. **Do not design before it lands.**

## CS scheduling prior art

### Shape A — starvation under priority/greedy selection

| technique | mechanism | fit here |
|---|---|---|
| **Aging** (classic MLFQ) | effective priority rises with time waited | **Best fit.** We already record `cycles_since_last_claim` per cell — the aging term is a field we have. Deterministic, O(1), no new state. |
| **Priority boost / escalation** | periodic sweep promotes long-waiters to the front | Same family, coarser. Boost interval is another tuned constant. |
| **Cooldown / backoff after failure** | a failed attempt suppresses re-selection for k cycles | Attacks the *other* side — stops the winner hogging rather than helping the loser. Classic from CSMA/CD and TCP. Composes with aging; alone it can idle a server that would have succeeded. |
| **Lottery scheduling** (Waldspurger) | tickets ∝ share, random draw | **REJECT — randomness.** Determinism-by-construction is a permanent law here. Only admissible seeded from the deterministic RNG, which buys nothing aging doesn't. |
| **Stride scheduling** (Waldspurger) | deterministic dual of lottery: least-passed-value wins, pass += stride | **Strong fit, deterministic by construction.** Naturally starvation-free; the "pass" accumulator is one `u64` per job. |
| **CFS / virtual time** (Linux) | always pick least virtual runtime | Same idea as stride; monotone counter makes starvation impossible by construction rather than by tuning. |

### Shape B — never evaluated

| technique | mechanism | fit here |
|---|---|---|
| **Deficit Round Robin** (Shreedhar & Varghese) | O(1), each queue carries a deficit counter across rounds | **Best fit for cap exhaustion.** Bounded work per tick *and* every queue advances. |
| **Rotating start offset** | resume scanning where the last cycle stopped | **The cheapest possible fix** if the cap is the real constraint — one index, fully deterministic, no scoring change. |
| **Raise / remove the cap** | more iterations per tick | Trades CPU; `soak_avg_tick_ms` is already reported so the cost is measurable. Doesn't fix ordering, just widens the window. |

## Games prior art

- **Dwarf Fortress** — the closest analogue, and it has fought exactly this.
  DF's answer is *not* smarter global scoring: it is **explicit priority
  designations**, **burrows** to constrain candidate sets, and per-labour
  enable/disable. **The lesson is scope reduction:** DF makes the candidate set
  smaller rather than the comparison cleverer. Its well-known job-cancel and
  task-thrash spam is the failure mode of the *unconstrained* version — which is
  the shape we are in.
- **RimWorld** — per-pawn, per-work-type **priority tiers (1–4)** scanned in
  order, plus a **reservation system** so two pawns cannot claim one target.
  Notably the tiers are *authored*, not derived: the game refuses to infer a
  global optimum and lets the player break ties. **We already have the
  reservation half** (`claimed_by`); we lack the tiering.
- **Factorio** — construction bots take from an explicit **job queue** with
  assignment, not per-agent greedy re-scoring of a global set. Assignment-side
  rather than selection-side; avoids the problem by construction.

## What our constraints exclude

1. **No randomness** — determinism by construction is a permanent law. Rules
   out lottery scheduling; stride/CFS are the deterministic equivalents.
2. **FR15** — an arbitration change re-rolls the whole colony economy, so the
   fix lands with a paired A/B or not at all. **This is the expensive
   constraint**, and it argues for the *smallest* change that resolves the
   measured shape.
3. **Existing scoring carries deliberate terms** — the top-down depth bonus, the
   access-step exclusion from Mine scoring (`09578b0172`: access steps were
   excluded precisely because the depth bonus chased the unreachable top step).
   **Any aging term must compose with these, not replace them** — and that
   history is a warning that this scoring function has already been tuned
   against a starvation-shaped bug once.

## Recommendation (design NOT started; for the row when its gates clear)

**If the trace shows shape A:** an **aging term on `cycles_since_last_claim`**,
added to the existing score. Smallest possible change, uses a field already
recorded, deterministic, composes with the depth bonus rather than replacing it.
**Stride/CFS is the more principled answer but is a larger rewrite** — reach for
it only if aging proves untunable.

**If the trace shows shape B:** a **rotating scan start offset** before anything
else. One index, no scoring change at all, and it should show up in the
per-caller counters immediately.

**Either way, prefer DF's lesson over a cleverer comparison:** if the candidate
set can be scoped (pocket, region, depth band) the starvation pressure drops
without touching the score function — and the descent caller's Chebyshev-8
pocket scoping is *already that pattern, already proven in this codebase*.

## Sources

Standard scheduling literature (MLFQ/aging; Waldspurger's lottery & stride;
Linux CFS; Shreedhar & Varghese's deficit round robin) and the three games'
published/observable designs. **Written from domain knowledge, not fetched** —
if the row wants citations for the DF/RimWorld specifics before relying on them,
those should be verified rather than taken from this document.
