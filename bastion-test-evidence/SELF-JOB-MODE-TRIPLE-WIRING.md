# Self-job mode-triple wiring

Per Opus's direction to close the seed-7/seed-90 Chaser investigation and
move to the row's next queued item: extend the step/jump/scramble
reachability probe (`path_exists_step/jump/scramble`, already proven for
mine cells via `b5_mine_reachability_probe`) to self-jobs
(RestAt/EatFrom/Despond), which never got it — the actual gap §4.1's own
read pointed at (seed 7's bed case has no equivalent instrument).

## Design

`probe_target` (the closure that runs the reachability probe for one
position, `bastion-harness/src/main.rs` ~3509) was already job-kind-
agnostic — it takes a bare `Vec3<i32>`. The only mine-specific part of
`b5_mine_reachability_probe` was which positions fed it
(`mine_cell_diag`'s own list). New field `b5_self_job_reachability_probe`
feeds the same closure with §4.1's already-generic timeout-position list
(`bastion_travel_timeout_last_positions`), filtered to positions **outside
the mine designation's own region bounds** (`mine_min`/`mine_max`, already
in scope in `b5_scenario`). Zero new `Server` methods — pure harness-side
reuse of two already-built, already-proven pieces.

## A caught-before-commit bug: mine_cell_diag-position exclusion is wrong

**First version** excluded only positions present in `mine_cell_diag`
(the still-open-cells snapshot). Wrong: `mine_cell_diag` only lists cells
**still holding an open job** — a mine cell that timed out earlier and
later completed (exactly seed 90's job 23, per Opus's own correction
earlier in this row) is absent from `mine_cell_diag` by the time it's
read, but still carries historical data in the kind-agnostic §4.1 map.
Verified the bug directly: seed 90's first build showed 6
"self_job_reachability_probe" entries, all at positions still inside the
mine designation (17987-17989, 9263-9265, 337-338) — completed mine
cells mislabeled as self-jobs, not a self-job discovery.

**Fix:** exclude by region bounds instead of by `mine_cell_diag`
membership. Correct regardless of a cell's completion state, since region
bounds don't change.

## Verification

- Seed 90 (mixed mine outcome, job 20 unresolved): `self_job: []` after
  the fix (was `[6 entries]`, all mislabeled mine cells, before it).
  `mine_reachability_probe` unaffected, still 3 entries.
- Seed 76 (mine fully resolved, `mine_cleared: true`,
  `mine_jobs_remaining: 0`): `mine_cell_diag` and `mine_reachability_probe`
  both legitimately empty (no open cells left to report) —
  `self_job_reachability_probe` also correctly `[]`, proving the
  region-bounds filter works whether or not the mine cells it's excluding
  are still open, unlike the buggy first version.

## Positive case closed (Opus-directed): `preempt_scenario`'s floating bed

The gap above — "expected to work by construction, not empirically
confirmed" — is now closed. `preempt_scenario`'s PHASE 2 fixture
(`sky_bed`, a floating bed with no route up, unreachable **by
construction**) is a guaranteed-precondition planted-positive test — the
mirror of a planted-failure test, and it didn't need a new fixture.

New env-gated check (`BASTION_SELFJOB_PROBE_DIAG`) reads the exact same
two mechanisms `b5_self_job_reachability_probe` reuses
(`bastion_travel_timeout_last_positions` +
`bastion_offline_reachability_probe`) directly against `sky_bed`, rather
than duplicating `b5_scenario`'s full JSON schema onto a different
scenario — the wiring under test is the two `Server` methods, not the
field name.

**Result (seed 42):** `sky_bed` present in the timeout-position map
(`present_in_timeout_map=true`), and the probe returns a fully-determined
negative — `path_exists_step=false, path_exists_jump=false,
path_exists_scramble=false, probe_incomplete=false, standable_target=
None` — exactly matching the fixture's own design intent ("the floating
slab has no route up"). Not a budget cap firing (`probe_incomplete`
would be `true` for that); a genuine, complete "no path exists" verdict
on all three tiers. The mechanism is now proven end-to-end, not just by
inference from the mine path.

## Status

Additive, no existing field changed. Committed alongside this note.
