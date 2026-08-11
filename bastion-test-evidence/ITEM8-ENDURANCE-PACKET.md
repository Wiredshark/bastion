# ITEM 8 — THE ENDURANCE RUN (ARC 1's LAST ITEM)

**Written before the reseed leg lands**, because the spec does not depend on it
and the run does. **Status: DRAFT FOR RULING.**

**What this item asks, in one sentence:** *given founding stock and no
provisioning, does a colony SUSTAIN ITSELF across multiple day cycles — or does
it decay in a way only long duration exposes?*

★ **Every previous acceptance in this arc was a MECHANISM test on a short run.
This is the first DURATION test, and duration is the only instrument that can
find a slow leak.** *A colony that survives 12 minutes and starves at 90 is a
pass on every bar we have written so far.*

---

## 1 · SCENARIO — **NO PROVISIONING, THAT IS THE POINT**

- **Founding stock only** (the row 5b just built). **No `dropall`, no injected
  food, no seeded stockpile.** *Provisioning is precisely what this run must not
  have — item 6 measured a protected pile; this measures whether a pile ever
  exists.*
- **Full needs active** — hunger and rest both live, no forcing.
- **Farm loop live** — sow → grow → harvest → eat, with the seed bootstrap fixed.
- **Instrumentation ON**: entity event log (`BASTION_ENTITY_EVENT_LOG=1`), all F3
  counters, all item-6 witness counters, `stalled_final`, release records.
- **Ben's observer seat**: the session runs with him able to watch and interrupt.
  *This is the first run of the arc whose primary consumer is a person, not a
  parser, and that changes what must be legible live.*

## 2 · DURATION AND THE UNIT PROBLEM

**Multi-sim-day. State the target in SIM time and record the wall/sim ratio** —
they diverge ~9× headless, and every previous run's `%-of-budget` confusion came
from conflating them.

> ★★ **The run states its target as N COMPLETE DAY CYCLES, not minutes.** *A cycle
> is the unit the needs are keyed to; minutes are the unit the operator feels. The
> bar is written in cycles and the ETA is quoted in both.*

## 3 · THE BAR — **A SUSTAINED SYSTEM, NOT A SURVIVED MOMENT**

★★★ **The failure this run exists to catch is a SLOW LEAK, so every measure is a
TREND across cycles, never an end-state snapshot.** *An end-state check passes a
colony that is one cycle from collapse.*

| # | measure | PASS expression | FAIL expression |
|---|---|---|---|
| 1 | **No deaths** | colonists alive at end == at start | any death |
| 2 | **Every colonist eats every cycle** | for each cycle, `distinct_eaters == colony size` | any colonist misses a cycle |
| 3 | **Every colonist sleeps every cycle** | same shape on rest | any colonist misses a cycle |
| 4 | ★ **Food stock does not trend down** | stock at cycle N ≥ stock at cycle 2, across all N | monotone decline over ≥3 cycles |
| 5 | **No permanent stall** | no colonist idle-with-unmet-need for > one cycle | any does |
| 6 | **Fail-safe rate does not climb** | teleports per cycle flat or falling | rising across cycles |

★★ **Measure 4 is the one that only duration can test**, and it is the row's real
question: *consumption is continuous, harvest is periodic, and a colony can look
healthy for two cycles while the buffer drains.* **Cycle 2 is the baseline, not
cycle 1 — the first cycle is bootstrap and its stock is not steady-state.**

### THE ZERO-WINDOW, per the acceptance-framework law

**Needs cross at known times. A satisfying event BEFORE its need crosses is not a
better result — it INVALIDATES the leg**, because the outcome arrived by a path
that is not the mechanism.

    cycle window          hunger        rest       REQUIRED
    0 → first crossing    above         above      ZERO eats, ZERO sleeps
    after crossing        below         —          the event, EVERY cycle, EVERY colonist

★ **Name the actual crossing times from the live config at run start and put them
in the results header** — *do not carry them from memory; they are tunable and
this programme has already been bitten by a threshold that meant something else.*

## 4 · NAMED FAILURE MODES, EACH WITH ITS OWN WITNESS

| mode | witness that must exist | if it fires |
|---|---|---|
| **Seed bootstrap deadlock returns** | sow jobs created but never claimed | the fixed row regressed |
| **Starvation by drain** | measure 4's trend | the farm loop is slower than consumption |
| **The suspension state** (successor row) | `on_ground=false && on_wall=false` while idle | the successor row's specimen, at scale |
| **Stall accumulation** | `stalled_final > 0` rising across cycles | claims held longer each cycle |
| **Silent instrument death** | every counter's own presence | *a zero from a dead counter is the trap this arc has hit three times* |

## 5 · THE PROCESS RULES THIS RUN INHERITS — **NON-NEGOTIABLE, ALL EARNED TODAY**

1. ★ **The log stamps its code identity at boot** (§1a). *A run that cannot prove
   which binary produced it is unscoreable, and we spent an hour on exactly that
   yesterday.*
2. **Effective config is EMITTED, not inferred** — thresholds, needs rates, colony
   size. *Inference-from-effect only covers cases where the effect was possible.*
3. **Every bounded/capped field carries a truncation flag**, and the flag is read
   BEFORE any distribution. *Censoring has cost this programme twice in a day.*
4. **Attestation before verdict**: the run's own completion count, not its exit
   code. *A runner's exit status reports whether it finished, not whether it did
   any work.*
5. ★★ **Every measure above must be able to FAIL.** *Before the run: for each row
   in the bar, state what a failing observation looks like. A check that cannot go
   red is not a check — three of mine yesterday couldn't.*
6. **Zero cases = VOID, never PASS.** *If a cycle never completes, measures 2–6
   have no population and the run reports that rather than a green.*

## 6 · WHAT A PASS DOES AND DOES NOT BUY

**A pass says: this colony sustains itself for N cycles under these needs rates,
with this colony size, on this map.** It does **not** establish behaviour at
larger colony sizes, different terrain, or longer horizons — **and the results
must say so**, or a green gets read as "the colony sim works."

★ **And a FAIL here is the most valuable result the arc can produce**, because it
is the first failure mode that short runs structurally cannot see.

## OPEN FOR RULING

1. **N** — how many complete cycles? *My lean: enough that measure 4 has ≥3
   steady-state points, so N ≥ 5.*
2. **Colony size** — founding stock's default, or larger to stress the farm loop?
3. **Interrupt policy** — does Ben's observation pause the clock, and does an
   interrupted run still score?
