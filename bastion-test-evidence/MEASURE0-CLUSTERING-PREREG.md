# MEASURE 0 — THE CLUSTERING READ, PRE-REGISTERED

**Written BEFORE the accessor lands and before any event data exists**, so the
decision rule cannot be chosen after seeing the distribution. *That failure mode
has a name in this programme now: a field cannot calibrate its own bound, and a
threshold picked from the data it judges is the same error wearing a smaller hat.*

## THE QUESTION, AND WHAT ACTUALLY DISCRIMINATES

**Hypothesis (claim-overlap):** colonists who no longer sit hold access claims
over *different intervals* — same removals, different overlap — so more claims
are live at the moment a removal happens.

★ **The naive framing — "do orphanings cluster near removal events" — is not
directly testable, because the corpus has no separate removal-event stream.**
`Released{RemovedExternally}` **is** the orphan detection; there is no independent
timestamp for the removal that caused it.

> ## **WHAT IS TESTABLE: DO ORPHANINGS ARRIVE IN TIGHT CLUSTERS?**
> **One removal that sweeps many access jobs orphans every claimant IN THE SAME
> TICK.** *Independent per-job removals orphan one claimant at a time.* **So
> co-occurrence at a tick is the signature of batch removal, and it is readable
> from the `Released` ticks alone.**

★★ **This is the same move as the F3 branch attribution:** the mechanism is not
observable directly, but it leaves an arithmetic fingerprint that nothing else
produces.

## THE DECISION RULE — BOTH BRANCHES AS EXPRESSIONS, BEFORE THE DATA

Let **E** = `RemovedExternally` release ticks in a seed; **C** = the largest
number of such events sharing a single tick; **S** = seeds with `|E| >= 2`.

| reading | expression | consequence |
|---|---|---|
| **CLUSTERED — batch removal** | `C >= 2` in **more than half of S** | Orphanings are swept in batches. Claim-overlap is live: a removal catches several claimants at once, and how many depends on how long claims are held. **Supports the hypothesis.** |
| **DISPERSED — independent removals** | `C == 1` in **almost all of S** (≥80%) | Each orphaning is its own removal. **No batch to overlap with; the claim-overlap story loses its mechanism** and the movers need a different producer. |
| **VOID** | `|S| < 5` | Too few multi-event seeds to distinguish. **Not a pass for either side** — report the population and stop. |

★ **NON-VACUITY, checked at registration:** both branches are reachable — `C` can
be 1 or >1 given the same field, and nothing in the emit path fixes it. *That is
the check I failed twice today (a counter inside a branch whose predicate already
fixed it; a kill-condition against a provably-inert wave pair), so it is asked
here explicitly rather than felt.*

★★ **AND THE PRIOR, REGISTERED SO IT CAN BE WRONG:** I expect **CLUSTERED**,
because `b5_release_removed_externally` concentrated on movers at 5/6 vs 10/42
(p=0.021) — a per-seed concentration is easier to produce by a few batch events
than by many independent ones. **If it reads DISPERSED, the surviving mover lead
loses its mechanism and I will say so plainly.**

## WHAT THIS READ DOES NOT ESTABLISH

- **Clustering does not prove #94 caused it.** It establishes batch-removal
  overlap as the *shape*; attributing the shape to the sit-fix needs the
  commit-level work that is still parked.
- **It says nothing about seed 69**, which is a single event's queue position and
  needs its own field — deliberately not folded in here.
- **Harness population only.** *Per the item-4 lesson ruled tonight: a live-session
  signature may simply not occur in fixture geometry.* **If the fan yields no
  `RemovedExternally` events at all, that is a POPULATION finding, not a refutation.**

## PROCEDURE

1. Fan at a pin containing stages 2+3, `BASTION_ENTITY_EVENT_LOG=1`.
2. **Attest first** — every seed's `b5_eelog_event_count` numeric, per the floor's
   own gate. An unattested run is not scored.
3. **Check the truncation flag before reading any distribution.** *A capped list
   is right-censored, and this programme has now paid for that twice in one day.*
4. Compute `S` and `C`; read the table; report the branch and its population.
