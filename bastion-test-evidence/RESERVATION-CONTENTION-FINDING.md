# ★ THE SCALING CONSTRAINT, FOUND: reservations are per-STACK, not per-UNIT

**One merged pile serves exactly one worker at a time.** That is why a colony
of 32 employs the same two-to-three people as a colony of 8.

## The evidence

`scale32diag` (32 colonists, 256 seeded materials, attested fresh). The
refusal instrument fires with the same two numbers every time, across hundreds
of instances:

```
RESERVATION-ONLY colonist=10 req="…wheat_seeds" stocked=1 reserved=1   × 229
RESERVATION-ONLY colonist=11 req="…wheat_seeds" stocked=1 reserved=1   × 279
RESERVATION-ONLY colonist=12 req="…wheat_seeds" stocked=1 reserved=1   × 352
RESERVATION-ONLY colonist=11 req="…wheat"       stocked=1 reserved=1   × 2
…
claim refusal census: materials=372
```

`stocked=1 reserved=1` — the material **is** in the stockpile, and **every unit
of it is reserved**. Not absent. Not unreachable. Spoken for.

## How the diagnosis narrowed

Three candidates were registered before any of them was tested, and the
instruments eliminated two:

| Candidate | Verdict | Evidence |
|---|---|---|
| Haul generation quota-capped on a fixed cadence | **REFUTED** | `pending=1 cap=64` — the quota is nowhere near binding |
| Stockpile-membership disagreement | **REFUTED** | `rej_in_stockpile=5`, `stocked=1` — the material is demonstrably in a stockpile |
| **Reservation contention** | **CONFIRMED** | `reserved=1` on every refusal, hundreds of times |

It was also the only candidate that predicted deliveries would *decrease*
(62 loads at 8 colonists → 37 at 32) rather than merely fail to rise. That
prediction is now met by measurement rather than by argument.

## The mechanism

A stockpile holds **one pickup entity per item def** — piles merge. Reservation
at this gate is `is_reserved(uid)`, a **boolean per entity**. So a colonist
reserving a stack of 224 stone makes all 31 others refuse, no matter how much is
in it.

More colonists means more contention for the *same single stack*, which is
exactly the shape of the measurement: throughput falls as population rises.

**This is the same defect class F13's root cause was**, and I half-fixed it
there: the commit-time fetch scan was made unit-aware
(`has_capacity(iuid, amount)` / `reserve(iuid, amount)`), but this claim gate
still asks the boolean question. **Two predicates about one quantity, one
counting units and one counting entities** — the drift this session has now
paid for in bed scans, ownership fields, demand sources, generator-vs-claim
tests, and rest thresholds.

## The fix, and why it is not made in this commit

The gate should ask the unit-aware question the fetch path already asks: does
this stack have *capacity remaining* for my amount, rather than *is it touched*.

It is not made here because it changes a hot claim path that every leg in the
corpus runs through, and it deserves its own pre-registered row with a matched
before/after on the same three scale arms — the numbers to beat are already
recorded above. Registered, not rushed.

## Note

`wheat_seeds` dominates the refusals (860 of ~880 sampled). Farming contends
hardest because sowing is the highest-frequency material consumer — so this
constraint bites the food economy first, which is also the one that keeps a
colony alive.
