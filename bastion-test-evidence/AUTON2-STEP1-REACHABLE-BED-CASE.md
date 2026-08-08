# AUTON-2 Step 1: the planted reachable-bed case, and a defect it found

Extends `auton2_needs_probe` (already existing, already committed, previously
only proved INITIATION) with a COMPLETION phase — claim → travel → arrive →
sleep → restore → resume work — from a genuine decay-driven interrupt, the
pipeline the corpus had never observed fire outside a forced
`bastion_set_needs` (`preempt_scenario`'s PHASE 1). Config route (item 1,
env-gated `MoodConfig::current()` override) was already done earlier this
session; re-confirmed by reading `common/src/bastion.rs` before building
anything new.

## Design

- **Phase-scoped acceleration** (Fable's naming): the override's job is to
  make ONE phase — the interrupt crossing — observable in a short window
  (`rest.decay_per_sec` accelerated ~130x). It is dropped back to shipped
  (`env::remove_var`, `unsafe` per this edition, matching the codebase's
  existing pattern at `main.rs:1470`) the moment the interrupt fires, before
  the completion-tracking loop starts. `matches_shipped_when_unset` is
  computed BEFORE this removal, so the byte-identical-absent proof is
  untouched by it.
- **Why one constant can't serve both phases**: `decay_needs` runs
  unconditionally every tick regardless of job state — decay never pauses
  for sleep. A decay rate fast enough to cross the interrupt band quickly
  (0.04/sec) is faster than a bedroll's own recovery ceiling
  (`BED_REST_RECOVERY_PER_SEC * BedKind::Bedroll.quality()` = 0.02*0.6 =
  0.012/sec) — first-pass testing at a single accelerated constant showed
  rest occupying the bed at 0.117 and never recovering. Measured, not
  guessed, before the fix landed.
- **`RESTORE_THRESHOLD = 0.6`** matches the real mechanism's own completion
  bar exactly (`bastion_jobs.rs`'s SLEEP arm: `slept = needs.rest >=
  cfg.rest.comfort + SLEEP_MARGIN` = 0.5 + 0.1), not an invented number.
- **Arrival asserted separately from restoration** (Opus review checklist
  item 2): `bed_claimed_and_arrived` (bed slot occupant == this colonist's
  uid) and `rest_restored` are independent booleans, plus a
  `completion_classification` string (`travel_row_failure_never_arrived` /
  `needs_row_failure_arrived_never_restored` /
  `needs_row_failure_restored_never_released` / `completed` /
  `not_under_test`) so a red result never reads as a needs-machinery
  failure when the actual cause is a different row (travel).
- **Work resumes, not just reaches the band** (item 4): `job_completed`
  watches the bed slot's occupant clear back to `None` (the real
  completion arm's own side effect) — a colonist that reaches comfort and
  sleeps forever is not tracked as success.
- **Planted-failure falsifier** (item 6): `planted_case_proven =
  override_env_set && natural_interrupt_reached && completion_ok`. Unlike
  `completion_ok` (vacuously true when the override is absent — it's
  proving a DIFFERENT claim), this field requires the override to have
  actually been exercised. Disabling the override structurally reads
  `false` — confirmed directly, not asserted: the absent-override control
  run shows `planted_case_proven: false`.

## A defect found while sizing the completion budget

First pass used a 2400-tick (80 sim-sec) completion budget. 2 of 4 test
seeds failed with `needs_row_failure_arrived_never_restored`. Traced with
`BASTION_AUTON2_DIAG` rather than assumed: the colonist occupied the bed
immediately, was released again ~10 sim-sec later (`bed_occupant -> None`)
with zero rest gain, then re-occupied ~60 sim-sec after that and completed
successfully. The two intervals match `STUCK_TIMEOUT` (10s) and
`PREEMPT_COOLDOWN_SECS` (60s) exactly, not approximately.

**Root cause: the Traveling-state stuck-watchdog has no arrived-and-
restoring exemption.** Sleeping is standing still; the watchdog tracks
`sdist` toward the target and cannot distinguish "stuck" from "successfully
doing the thing that requires not moving." A colonist sleeping in a
genuinely reachable bed — arrived, occupying, correctly accruing rest per
tick — gets released at `STUCK_TIMEOUT` with zero rest gain and cannot
retry for `PREEMPT_COOLDOWN_SECS`.

**This is a real, previously-undiscovered defect, not specified/accepted
behavior.** Filed as DECISIONS #69 (Opus). The distinction matters: ENDURE
is specified for an *unreachable* bed (`preempt_scenario`'s floating slab,
no route up) — this bed is reachable and was successfully occupied. Opus's
own earlier framing ("watchdog releases... all correct, all specified")
was about the unreachable case and doesn't cover this one; he retracted
that framing on reading this fixture's trace, and separately noted this
answers his own earlier-retracted §4b question ("what interrupts an
in-progress sleep?") with a real referent.

## Interruption-rate measurement (the 7-seed sweep, seeds 49-55)

The `occupancy_interruptions` counter — added explicitly so the retry isn't
buried inside "it eventually worked" — makes the defect's rate directly
measurable:

| seed | occupancy_interruptions | outcome | ticks to restore |
|---|---|---|---|
| 49 | 1 | completed | 1613 |
| 50 | 2 | completed | 3082 |
| 51 | 1 | completed | 1237 |
| 52 | 2 | completed | 3104 |
| 53 | 1 | completed | 1222 |
| 54 | 2 | completed | 3089 |
| 55 | 2 | **budget exceeded** | — |

**`interruption_rate = total_interruptions / (total_interruptions +
zero_interruption_seeds) = 11 / (11 + 0) = 1.0` exactly.** Every single
observed sleep attempt in this sample was interrupted at least once — not
"sometimes," not a distribution with a clean tail. Seed 55's completion
budget (4800 ticks / 160 sim-sec, already widened once from a first-pass
2400) still wasn't enough after 2 interruptions — a near-miss on budget,
not a 3rd interruption (the counter would show 3 if a third release had
actually occurred).

**Correction to an earlier read, recorded rather than silently fixed**: an
initial 4-seed test at the ORIGINAL 2400-tick budget looked like "2 clean,
2 interrupted" — that was an artifact of the counter not existing yet in
that run, plus the smaller budget masking seed 49/51's own interruption by
having enough slack to still complete. Once the counter was added and the
budget widened, all 4 of those seeds showed >=1 interruption. There is no
clean/interrupted matched pair in the corrected data — the real per-seed
variance is interruption COUNT (1 vs 2), not presence/absence.

**An honest instrumentation gap, not a null result**: `ticks_to_bed_occupied
== 0` in every single seed (all 7) — first arrival always happens inside the
pre-existing initiation loop's 60-tick post-crossing headroom, before the
completion loop's own instrumentation starts watching. This means "does bed
distance / travel duration affect interruption count" (a candidate root-
cause Opus proposed) cannot be answered from this data — it isn't evidence
distance doesn't matter, it's a blind spot in where this fixture starts
looking. Answering it for real needs the observation window moved earlier,
into the initiation loop itself — not built here, flagged as the natural
next read (`BASTION_SDIST_TRACE_JOB`, already built for the seed-7 work, is
the cheapest available instrument for it).

## Verified

- **Item 1** (band reached naturally): regression-confirmed, `natural_
  interrupt_reached: true` on every accelerated-override seed, `ticks_to_
  interrupt: 511` (deterministic across all 7 — this fixture's setup is
  seed-invariant up to that point).
- **Item 2** (arrival asserted separately): `bed_claimed_and_arrived` /
  `completion_classification` distinguish travel-row from needs-row
  failure; not exercised as RED in this sweep (no travel failures
  observed), but the machinery is in place and load-bearing.
- **Item 3** (sleep completes at the real bar): `RESTORE_THRESHOLD = 0.6`,
  6 of 7 seeds reach it (seed 55 times out on budget, not on the
  threshold itself).
- **Item 4** (work resumes): `job_completed` / `ticks_to_bed_released`
  confirm the bed slot clears on all 6 completing seeds.
- **Item 5** (`preempted_rested`/`ate` flip green): out of this fixture's
  scope — that's `preempt_scenario`'s own registered-prediction flags, not
  a decay-driven claim this probe makes.
- **Item 6** (planted-failure falsifier): confirmed directly — the absent-
  override control shows `planted_case_proven: false`, `natural_interrupt_
  reached: false`, while `matches_shipped_when_unset: true` (the byte-
  identical proof this fixture ALSO carries, unaffected).
- **Item 7** (ENDURE regression): `preempt_scenario --seed 49` still
  passes clean (`PREEMPT SCENARIO: PASS`, `thrash_bounded: true`,
  `endured: true`, `preempted_rested: true`) — the completion-phase
  additions to `auton2_needs_probe` don't touch `preempt_scenario` and
  don't regress it.

## Status

The fixture proves the machinery CAN complete end-to-end from a genuine
decay trigger — and in doing so, measured a real defect (100% interruption
rate on first sleep attempt) that no prior corpus run had surfaced, because
no prior scenario ever let a colonist reach and occupy a bed under natural
decay for long enough to be interrupted. The defect is filed (DECISIONS
#69) and is NOT this fixture's to fix; `occupancy_interruptions` is left in
place as the acceptance instrument for whatever fix lands, measuring the
rate before and after.
