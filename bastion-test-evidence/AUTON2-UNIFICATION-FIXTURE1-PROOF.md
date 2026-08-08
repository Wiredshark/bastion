# AUTON-2 unification, Fixture 1 (TRY-TO-ORPHAN): the must-fail-first proof

Implements the required precondition from `AUTON2-ACCEPTANCE-FIXTURES.md`
before any GUARD-6 build starts: *"Run [the settle invariant] against the
current tip FIRST and require RED. If the fixture passes against a
pre-unification binary, it is not testing the change."* This document
covers ONLY that proof, for the ONE release path already producing a live
specimen (path 1, travel-timeout release, via `auton2_needs_probe`'s bed
fixture). The remaining three named paths (colonist death mid-job,
Despond carve-out removal, explicit `remove_job`) and Fixture 2
(despond-resume determinism) are not built yet — see "Not built" below.

## The invariant

Per the spec: *for every self-job (`is_labor_hold_self_job`:
`RestAt`/`EatFrom`/`Despond`) in `board.jobs`, either it is claimed OR it
is reachable by the arbiter's selection.* Under the current (pre-
unification) lifecycle, GUARD-6 site 1 (`~8838`) unconditionally skips
self-jobs from arbiter selection — "reachable by selection" is `false`
for every self-job today, so the invariant reduces exactly to *no
self-job may be unclaimed*.

## Implementation

- `bastion_jobs::is_labor_hold_self_job` widened from `pub(crate)` to
  `pub` — the settle invariant needs the SAME predicate GUARD-6 site 1
  uses, not a re-derived copy that could drift.
- `Server::bastion_settle_invariant_violations()` (new, `server/src/
  lib.rs`) — one pass over `board.jobs`, returns the positions of any
  self-job with `claimed_by.is_none()`. Settle-time only, per the
  fixture's stated budget (no per-tick engine cost added; the harness
  polls it once per completion-loop tick, which is harness-side reading,
  not new engine work).
- Wired into `auton2_needs_probe`'s existing completion loop (checked
  every tick of that window, since it wasn't clear from this fixture
  alone exactly which tick `board.beds`'s occupant clears relative to
  `job.claimed_by` going `None` — `STUCK_TIMEOUT` sets `claimed_by = None`
  without calling `remove_job`, so a per-tick invariant read is the safe
  choice over guessing the exact edge). New JSON fields:
  `settle_invariant_holds`, `settle_invariant_violation_tick`,
  `settle_invariant_violation_pos`.

## Result: confirmed RED, exactly as required

| seed | occupancy_interruptions | settle_invariant_holds | violation_tick |
|---|---|---|---|
| 49 | 0 (clean arrival) | **true** (no violation) | — |
| 50 | 1 (genuine retry) | **false** | 304 |
| 51 | 0 (clean arrival) | **true** (no violation) | — |
| 52 | 1 (genuine retry) | **false** | 248 |
| 54 | 1 (genuine retry) | **false** | 248 |

**Clean correlation, not coincidence:** every seed with a genuine
travel-timeout release (`occupancy_interruptions >= 1`, per
`AUTON2-STEP1-REACHABLE-BED-CASE.md`'s corrected instrument) shows a
settle-invariant violation; every clean-arrival seed shows none. Seed
50's violation fires at tick 304 (10.13 sim-sec) — the exact tick already
measured via `BASTION_SDIST_TRACE_JOB` as the `STUCK_TIMEOUT` release.

**This is the required falsifier, satisfied**: the fixture fails on the
current (pre-unification) tip, for the specific reason the spec predicted
(*"pre-claimed self-jobs never enter claim selection"*), not for some
unrelated reason. It is testing the change it's meant to test.

## Not built yet

- **Fixture 1's remaining 3 named paths** (colonist death mid-job, Despond
  carve-out removal, explicit `remove_job`/cancel) — path 1 is the only
  one with a live specimen already instrumented; the other three would
  need purpose-built scenarios.
- **Fixture 2** (despond-resume determinism: same deadline, no cooldown
  consumed, no `break_chance` re-roll — asserted via a roll COUNT, not a
  roll result, per the spec's own falsifier).
- The actual GUARD-6 unification build (5 sites, `despond_resume`
  deletion, the arbiter ranking) — this document proves only the
  precondition for starting it.

## Addendum: step 2 — the invariant live in EVERY scenario (2026-08-08)

Per `AUTON2-ACCEPTANCE-FIXTURES.md`'s own note ("an invariant that only
runs in the fixture that tests it protects nothing else"), the invariant
now runs automatically across every scenario, not just this one.

**Wiring, corrected once before landing.** First attempt hooked
`Server::cleanup()` — the one shared choke point every scenario's `tick()`
closure calls, and production too — with a fresh full scan of `board.jobs`
every call. Opus flagged this before it built: the acceptance spec's own
budget explicitly rules out "no per-tick" scans, and `cleanup()` runs at
full tick rate (30 Hz), not arbitration cadence. **Corrected to hook the
EXISTING orphan sweep instead** (`bastion_jobs.rs`, gated at `tick %
ARBITRATION_INTERVAL == 9`, ~2 Hz) — that sweep already computes this
invariant's exact population (self-jobs with no claimant) for its own
purpose, right before removing them. Counting there is free: zero new
scans, correct cadence, and the semantics are arguably better — the sweep
catches the TRANSIENT state (a job about to be reaped because nothing else
can reach it), which is exactly Fixture 1's own violation (fired at tick
304, the release tick, not at any later settle point).

New: `JobBoard::settle_invariant_violations: u64` (cumulative, silent —
NOT a `debug_assert`, since the invariant is currently expected to be
violated broadly; a hard assert would crash every scenario exercising a
self-job release path) and `Server::bastion_settle_invariant_violation_
count()` (the harness-side getter).

**Pre-stated expectation (Opus), recorded before running rather than
after:** turning this on will make it fire broadly — every scenario that
exercises a self-job release path, including `preempt_scenario`, where
ENDURE releases the claim BY DESIGN. **That breadth is the credential, not
an alarm.** A firing there is not a new defect; it is the exact
pre-unification state this row exists to change. The actual finding would
be a scenario that fires it WITHOUT a release path.

**Regression-checked**: `auton2_needs_probe --seed 50` still shows
`settle_invariant_holds: false` at `tick 304` (the live-snapshot variant,
unchanged); `preempt_scenario --seed 49` still passes clean
(`thrash_bounded`/`endured`/`preempted_rested` all true) — the new
counter's wiring doesn't touch behavior, only measures it.

**The after-state bar, stated now so it isn't re-derived later:**
post-GUARD-6, this same counter (and the live-snapshot position list) must
read ZERO across these same scenarios — because `arbiter-selectable`
becomes `true` for self-jobs once family 1 lands, which is the entire
point of the unification. Same invariant, same scenarios, opposite result:
the before/after in one instrument.
