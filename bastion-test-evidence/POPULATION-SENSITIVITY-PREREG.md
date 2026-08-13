# POPULATION SENSITIVITY OF THE SCORED BARS — **PRE-REGISTRATION**

Written before the run. Arises from `FOUNDING-COUNT-RESULTS.md`, which established that
**every scored bar in this program ran at 8 colonists** while the shipped widget passed
`6` — so the bars' conclusions have only ever been observed at one population, the one
that happens to saturate the bed plot (capacity 8).

## 1 · THE QUESTION, STATED NARROWLY

Not *"is 8 right?"* — that is a design question the count row deliberately left open.
The question here is **whether A2-B's mechanism is a property of the colonists or an
artefact of the population it was measured at.**

A2-B established §8 B4's claim — *work is what holds colonists* — by putting work 20
blocks from F and measuring presence:

| arm (n=8 colonists) | samples | within 8 of the outcrop |
|---|---|---|
| work designated at the outcrop | 704 | **335 = 47.6%** |
| no designations at all | 1008 | **0 = 0.0%** |

## 2 · THE DERIVED PREDICTION — registered before any data

Work-pull is a **per-colonist** behaviour: each colonist independently claims a job and
walks to it. Nothing in the mechanism is a crowd effect. And the outcrop supplies far
more work than any population here can consume — `RESOURCED_OUTCROP_HALF_WIDTH = 2`,
`HEIGHT = 3` ⇒ **5 × 5 × 3 = 75 cells**, against at most 8 workers.

> **PREDICTION: the presence FRACTION is population-invariant in kind.** Halving the
> population should not move it toward zero. If anything, less job contention should hold
> it equal or higher.

**A fraction is the right statistic precisely because it normalises out population** —
which is why A2-B chose it, and why this test is possible at all without re-deriving the
bar.

## 3 · THE BARS

### P1 · **THE PULL SURVIVES AT HALF THE POPULATION**
- Arm: identical to A2-B's work arm — same F, same outcrop, same designation, same
  window — **only `spawn 8` becomes `spawn 4`.**
- **PASS: within-8-of-outcrop ≥ 20%.**
- **Threshold basis:** the no-work control measured **exactly 0 of 1008**. 20% is an
  order of magnitude above any noise that control admits, and comfortably below the 47.6%
  observed, so the bar tests *the mechanism holding* rather than *reproducing a number*
  — which n=1 per arm could not support anyway.
- **FAIL: ≈0%**, which would mean the pull is population-dependent and A2-B's conclusion
  is narrower than it was stated.

### P2 · **THE POPULATION ACTUALLY CHANGED** — the precondition, printed above the result
- **PASS:** the founded emit reports `colonists=4`, and the presence diag shows **4**
  distinct uids.
- Without this, a script that silently founded 8 would make P1 a re-run of A2-B wearing a
  different label — the same class of void that cost this session a scored run.

## 4 · WHAT I WILL **NOT** DO

1. **I will not treat a difference in the fraction as a finding on its own.** n=1 per arm
   at each population; only a move **to ≈0** is interpretable, and that is what P1 tests.
2. **I will not re-run the no-work control at n=4.** It is not needed: the control's role
   is to show the outcrop is not visited absent work, and it measured **0 of 1008** — a
   floor no smaller population can undercut. Running it again would be theatre.
3. **I will not retro-score A2-B.** It stands as measured at 8. This row either extends
   its scope to a second population or narrows its claim; it does not rewrite it.
4. **I will not skip attestation.** `attest-run.sh` runs before the scored leg.
