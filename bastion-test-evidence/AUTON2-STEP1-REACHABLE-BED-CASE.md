# AUTON-2 Step 1: the planted reachable-bed case, and a defect it found

> **★★★★★ CORRECTION (2026-08-08, same day, after `BASTION_SDIST_TRACE_JOB`
> ran): the "watchdog interrupts an in-progress sleep" diagnosis below is
> WRONG.** `bastion_bed_slot`'s occupant field is set at RestAt job
> **creation** (`insert_rest_job`'s own comment: "reserve the bed at
> CREATION, not at arrival") — NOT at physical arrival. `bed_claimed_and_
> arrived`/`ticks_to_bed_occupied` were measuring the job's **reservation**,
> not the colonist actually reaching the bed — precisely the travel-row-vs-
> needs-row conflation the checklist's item 2 named as the trap to avoid,
> and this fixture fell into it by using the wrong signal for "arrival."
>
> A direct per-tick trace of seed 50's real interruption (job 10, occupancy
> interval 0→304 ticks / 10.13 sim-sec) shows `sdist` **never dropping below
> ~17** for the entire window — the colonist never got near the bed at all
> — with jump-attempt signatures (`vel_z=2.5` spikes, `on_ground` flickering)
> and zero net progress. `stuck_time` climbs smoothly and continuously from
> ~0 to ~10.0, so the stuck-watchdog fired correctly on a genuinely stuck
> **travel** attempt, not on a sleeping colonist. This looks like a
> pathing/obstacle issue specific to this bed's placement (one block above
> the flush plateau floor), not a needs-vs-watchdog interaction.
>
> **DECISIONS #69/#70's mechanism, as reported, was wrong. No fix was built
> on it** — caught before landing any code. `occupancy_interruptions` and
> the `interruption_rate = 1.0` measurement likely still hold as raw
> numbers (a real multi-attempt retry cycle exists), but their
> **interpretation** below is retracted: read "arrived and sleeping, then
> evicted" as "repeatedly failed to arrive, then eventually succeeded."
> The rest of this document is preserved as originally written, for the
> record of what was measured and where the reasoning went wrong — do not
> treat the "Root cause" section below as current.

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

**`interruption_rate = interruptions / (interruptions + clean_first_attempt_
completions)`** — both terms EVENT counts, per Opus's exact definition
(the acceptance instrument for the fix, so it must stay dimensionally
consistent: an initial pass computed the denominator's second term as a
SEED count instead of a completion-event count — coincidentally identical
here since this sample has zero clean completions either way, 11/(11+0)
either reading, but the two diverge the moment any run produces a clean
completion, which the fix is expected to make common). **= `11 / (11 + 0)
= 1.0` exactly.** Every single observed sleep attempt in this sample was
interrupted at least once — not "sometimes," not a distribution with a
clean tail. Seed 55's completion budget (4800 ticks / 160 sim-sec, already
widened once from a first-pass 2400) still wasn't enough after 2
interruptions — a near-miss on budget, not a 3rd interruption (the counter
would show 3 if a third release had actually occurred).

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

**The matched-pair investigation had no population.** Opus's original Ask 2
framed this as "what protects some seeds" — but once the counter existed,
zero seeds were clean, so there was no clean/interrupted pair to compare in
the first place. The falsifier's own precondition (a population containing
both outcomes) failed before the investigation could run; caught before
spending a read on it rather than after.

**Scope of the 1.0 rate**: measured on ONE fixture's fixed geometry (one
bed offset, one spawn point), not "everywhere." The scoped pre-fix this
number opens is justified on the MECHANISM being geometry-independent — the
watchdog times out on stillness, and sleeping is always stillness; there is
no bed placement in which a sleeping colonist moves — with 1.0 as
confirming evidence, not the sole basis. That framing survives a future
measurement on different geometry coming back lower.

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
decay trigger. The 100%-interruption-rate defect claim above is
**superseded by the final finding below** — read that section as current;
this "Status" paragraph is preserved for the record of what was originally
concluded, alongside the top-of-file correction.

## FINAL FINDING (2026-08-08, after two instrumentation fixes)

Two bugs in this fixture's own instrumentation were found and fixed before
the underlying phenomenon could be trusted:

**Bug 1 — reservation read as arrival** (the correction at the top of this
file). Fixed by adding a TRUE arrival signal: `bastion_colonist_states_
full()` (an already-existing getter, no new engine state) exposes
`ActiveJobState::Arrived` directly. New fields `ticks_to_true_arrival`/
`arrived_at_bed`; the old reservation-based fields were kept but renamed
honestly (`ticks_to_bed_occupied` → `ticks_to_bed_reserved`, `bed_claimed_
and_arrived` → `bed_claimed`), and `completion_classification` now gates
on the real signal.

**Bug 2 — `occupancy_interruptions` off by one.** The restore-threshold
check ran AFTER the interruption-count check in the loop; since the real
completion arm clears the reservation the SAME tick rest crosses the
threshold, every clean completion's own reservation-clear was miscounted
as its own "interruption" (`ticks_to_rest_restored` was still `None` at
the moment the guard evaluated). Confirmed directly: seeds that arrived on
tick 0–2 with a single continuous reservation for the whole run (verified
via `BASTION_SDIST_TRACE_JOB` showing only one job id, no
`AUTON2-TRACE-SWITCH` event) still reported `occupancy_interruptions: 1`.
Fixed by reordering the two checks.

**The corrected 7-seed table** (seeds 49–55, same accelerated override):

| seed | ticks_to_true_arrival | occupancy_interruptions (corrected) | outcome |
|---|---|---|---|
| 49 | 369 | 0 | completed |
| 50 | 1801 | 1 | completed |
| 51 | 2 | 0 | completed |
| 52 | 1823 | 1 | completed |
| 53 | 0 | 0 | completed |
| 54 | 1808 | 1 | completed |
| 55 | null (never arrived) | 2 | **budget exceeded** |

**The real split: 3 of 7 seeds (49, 51, 53) arrive clean on the very first
reservation — zero genuine watchdog releases.** 4 of 7 (50, 52, 54, 55)
show at least one genuine reservation-drop-then-retry, with seed 55 never
succeeding at all within the 4800-tick budget. **Not the 100% rate
originally reported** — that number was the counting artifact inflating
every seed's count by exactly one.

**What the genuine retries look like, per the `BASTION_SDIST_TRACE_JOB`
trace (seed 50, first attempt, job 10):** `sdist` never drops below ~17
for the entire pre-release window; jump-attempt signatures (`vel_z=2.5`
spikes, `on_ground` flickering) with zero net progress; `stuck_time`
climbs smoothly and continuously to ~10.0 (within noise of `STUCK_
TIMEOUT`). The watchdog fires correctly on a genuinely obstructed travel
attempt. The retry (job 11, created ~60 sim-sec later per `PREEMPT_
COOLDOWN_SECS`) then closes the remaining distance and arrives — the
"retry" is not a symptom of anything broken in the retry path itself.

**Filed as a TRAVEL-ROW specimen, not chased for a fix** (per Opus and
Fable's explicit direction): a bed placed one block above a flush plateau
floor fails first-approach roughly 4 times in 7 (this specific fixture's
sample), with a reproducible jump-attempt/zero-progress signature,
succeeding on a later attempt in 3 of those 4 cases and exceeding a
generous (160 sim-sec) budget in the 4th. This is the same one-block-short
/ small-obstacle class as the mine-26/27 cluster and the chopfell egress
work — a new specimen for an existing row, not a new defect class. The
bounded question for whoever picks this up: what differs between the
failed first attempt and the successful retry at the SAME bed (a matched
pair across attempts, not across seeds) — candidates include final-approach
geometry and whether the retry's spawn position happens to be closer.

**Two upstream corrections, filed for the next report window (not
blocking, per Fable):** `insert_rest_job`'s `slot.occupant` field is named
in a way that reads as "presence" but means "reservation" — a naming trap
this fixture fell into once and any future caller could fall into again.

**DECISIONS #69/#70's original mechanism (watchdog interrupts an
in-progress sleep) is retracted in full** — no fix was built on it. The
pre-fix Opus/Fable had opened is cancelled. `occupancy_interruptions` (now
counting correctly) and `ticks_to_true_arrival` remain in the fixture as
the honest instruments for whoever picks up the travel-row specimen.
