# POPULATION SENSITIVITY — **RESULTS & ROW DISPOSITION**

Scored against `POPULATION-SENSITIVITY-PREREG.md` (`ec5fae5f6b`). Engine tip
`f6e1707988` — **no code change**; this row is a measurement.

## THE SCORE — **2 PASS, 0 FAIL**

| bar | verdict | evidence |
|---|---|---|
| **P2** the population actually changed *(precondition)* | ✅ PASS | `colonists=4`, **4** distinct uids, 276 samples |
| **P1** the pull survives at half the population | ✅ PASS | **145/276 = 52.5%** within 8 of the outcrop (bar: ≥20%) |

Attested before the leg: HEAD `f6e1707988`, `dirty .rs : 0`, binary fresh.

## THE COMPARISON

| | n = 8 (A2-B) | **n = 4 (this row)** |
|---|---|---|
| samples | 704 | 276 |
| **within 8 of the OUTCROP** | 47.6% | ✅ **52.5%** |
| within 8 of F | 44.6% | 40.9% |
| *no-work control (n=8)* | *0.0% of 1008* | *not re-run — see below* |

**The mechanism is population-invariant in kind**, and the small move is in the direction
the prediction named: less job contention at n=4, so marginally *more* time at the work,
not less. 52.5% vs 47.6% is **not** claimed as a real difference — n=1 per arm at each
population, exactly as registered.

## WHY THE PREDICTION WAS DERIVABLE IN ADVANCE

Work-pull is a **per-colonist** behaviour — each colonist independently claims a job and
walks to it — and the outcrop supplies `5 × 5 × 3 = 75` cells against at most 8 workers,
so contention cannot bind at either population. A **fraction** normalises population out
by construction, which is why A2-B's choice of statistic made this test possible without
re-deriving the bar.

That the observed number landed near the predicted place is worth exactly what it is: one
confirmed prediction, not a law.

## WHAT THIS DOES AND DOES NOT SETTLE

**Extends:** §8 B4's mechanism — *work is what holds colonists* — now holds at **two**
populations rather than one. The count row's worry, that every bar had only ever been
seen at the bed-saturating 8, is answered **for A2-B**.

**Does not extend:** A3 (the eat loop) and A1/A4/A5 were not re-run here. A3 is the one
with a genuine population coupling — food consumption scales with head-count while farm
yield does not — so it is the row that could actually break at a different population,
and it is **registered open**, not assumed to follow.

## WHAT I DECLINE TO CLAIM

- **Not** that 52.5% > 47.6% means anything. One run per arm; only a collapse toward the
  control's 0.0% would have been interpretable, and that is what the bar tested.
- **Not** that the no-work control needed re-running at n=4. Its role is to show the
  outcrop is unvisited absent work, and it measured **0 of 1008** — a floor no smaller
  population can undercut. Re-running it would have been theatre, and the prereg said so
  before the result was known.
- **Not** that this validates the shipped `6`. It tests 4 and 8; 6 lies between them and
  is *interpolated, not measured*.

## SESSION QUEUE STATE — eleven rows closed

1–7 as recorded · 8. Cancel across restart (`71d06226a4`) · 9. Run attestation
(`003d583f96`) · 10. Founding colonist count (`b78ba830bf`) · 11. **Population
sensitivity**, this document.

**Next:** **A3 at a second population** — the eat loop is the one bar with a real
population coupling (consumption scales with head-count, farm yield does not), and it is
the remaining half of the question row 10 opened.
