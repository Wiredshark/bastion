# TRAVEL-ROW-SPEC.md §5: the planted-failure test, run

Implements the required part of §5 ("Planted-failure test (required):
construct BOTH cases... and assert they classify DIFFERENTLY") using two
already-existing fixtures instead of building new ones — a live specimen
beats a constructed one, per Opus's direction, and both cases were already
instrumented before this test ran.

## The two cases

- **Reachable-but-failed**: AUTON2-STEP1's bed (`bastion-test-evidence/
  AUTON2-STEP1-REACHABLE-BED-CASE.md`) — seed 50, a genuine `STUCK_TIMEOUT`
  release measured via `BASTION_SDIST_TRACE_JOB` (job 10, `sdist` pinned
  ~17 blocks, jump-attempt signature, zero net progress for the full
  pre-release window), followed by a successful retry.
- **Genuinely unreachable**: `preempt_scenario`'s `sky_bed` — a floating
  slab with no route up, unreachable by construction per its own doc
  comment.

Both classified via `bastion_offline_reachability_probe` (from spawn,
100,000-node cap), the exact same mechanism `preempt_scenario`'s own
`sky_bed_probe_result` already uses for its positive case
(`SELF-JOB-MODE-TRIPLE-WIRING.md`) — reusing the mechanism directly rather
than duplicating a JSON schema onto a new scenario.

## Precondition, checked before reading the result (Opus's explicit ask)

For the comparison to mean anything, the reachable-but-failed case must
classify `path_exists_step = TRUE` — otherwise both targets would land in
an unreachable-ish class and the comparison would "pass" for the wrong
reason. Checked directly (`bed_precondition_step_reachable` field), not
assumed:

```
seed 50: bed_precondition_step_reachable = true
```

Confirmed.

## Result

| target | path_exists_step | path_exists_jump | path_exists_scramble | standable_target |
|---|---|---|---|---|
| AUTON2-STEP1 bed (seed 50) | **true** | true | true | `[24491,26192,172]` |
| `sky_bed` (floating slab) | **false** | false | false | `null` |

**The classifier discriminates.** The reachable-but-failed bed lands in
Class A (a step path exists — the colonist got there eventually, and the
first attempt's failure is a genuine travel specimen, not a reachability
question). `sky_bed` lands in Class C (no mode works at all — giving up
is correct behavior, exactly ENDURE's designed case). Different classes,
as §5 requires: *"a classifier that cannot separate the two fixtures is
the defect restated, not fixed."* This one separates them.

## What this closes, and what it doesn't

- **Closes**: TRAVEL-ROW-SPEC.md §5's required planted-failure test. The
  row's PRIMARY acceptance criterion ("the harness reports, per travel
  timeout, which class it was — the row succeeds when the distinction is
  VISIBLE") is now demonstrated end-to-end on a live specimen, not just
  argued from corpus data.
- **Does not close**: a fix for the AUTON2-STEP1 bed's Class A behavior
  (the small-step-traversal-fails-on-first-attempt signature spanning
  three job kinds per `TRAVEL-ROW-SPEC.md` §2c). Per Opus and Fable's
  explicit direction, this row is filed, not chased.
- **Regression untouched**: this test reads existing probe/timeout state;
  it does not modify `preempt_scenario`'s ENDURE path or any watchdog
  logic. `thrash_bounded (1..=3)` was independently confirmed still
  passing when the AUTON2-STEP1 fixture itself was regression-checked.

## Implementation note

`auton2_needs_probe` gained three new JSON fields (`bed_probed`,
`bed_probe_result`, `bed_precondition_step_reachable`), computed after the
completion loop using `bastion_travel_timeout_last_positions()` (finds the
bed's entry, populated by the genuine `STUCK_TIMEOUT` this fixture already
produces on most seeds) + `bastion_offline_reachability_probe`. No new
engine state — both are existing, already-proven mechanisms.
