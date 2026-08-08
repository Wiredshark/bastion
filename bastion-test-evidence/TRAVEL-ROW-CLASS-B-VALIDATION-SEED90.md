# Class-B live validation, seed 90: the class splits — two adjacent cells, two different outcomes

Per Opus/Fable's assigned validation: order a colonist to seed 90's stuck
target(s) and observe which of Fable's three branches fires, with the
router's own answer (`route_exists`/raw `timeout_route_states`, not the
`route_next_idx_pinned` summary flag) captured alongside per Opus's
correction.

## The three specimens, live

Seed 90's mine designation contains (at least) three adjacent cells the
offline probe flagged with `path_exists_step: false, path_exists_jump:
true` — Class B by the probe's own classification:

| job | target | offline probe | live outcome | jump attempted? |
|---|---|---|---|---|
| 2  | (17989,9263,336) | min_dist 3.78, step:false, jump:true | **ARRIVES** (1 churn, then completes 8.3s later) | yes-adjacent-signal (see caveat below) |
| 23 | (17989,9264,338) | (not separately probed; adjacent cell) | **ARRIVES** (2 churns, then completes) | not traced |
| 20 | (17989,9263,338) | min_dist 16.24, step:false, jump:true, route_next_idx raw=`[8,8,3,4,4]` | **STALLS, never resolves** (4 churns across the whole run, zero completions) | **NO — confirmed absent** |

Job 20 and job 23's targets are **one block apart in y** (9263 vs 9264),
same z (338), same mine designation — as close to a matched pair as this
corpus offers, and they resolve completely differently.

## Job 20: correction — the jump IS occasionally dispatched, but never produces progress

**Earlier draft of this note claimed "zero jump attempts anywhere in the
trace." That claim was too strong and is corrected here.** Extending the
instrumentation to the actual dispatch predicate
(`server/agent/src/action_nodes.rs::traverse`, `jump_if((on_ground &&
bearing.z > 1.5) || can_fly)`) shows `jump_condition = true` fires **5
times** in a ~0.37s window (`BASTION_BEARING_TRACE_UID=3`), each with
`bearing_z = 2.0` and `on_ground = true` — the dispatch predicate is
satisfied and the jump input IS pushed.

**But `SDIST-TRACE`'s own reading of `on_ground`/`vel_z`, sampled from a
different system (`bastion_jobs.rs`) at nearly the same wall-clock instant,
shows `on_ground = false` and a small, REPEATING four-value `vel_z` cycle
(1.75 → 0.0 → -0.75 → 2.5) that persists continuously through and beyond
the dispatch window** — not a jump spike (contrast job 33's isolated 7.48
peaks), and not correlated with the 5 specific dispatch ticks; the cycle
runs before, during, and after them unchanged. Two readings of the same
moment disagree on `on_ground` by microseconds — consistent with the two
systems sampling physics state at different points in the tick pipeline,
not a contradiction in the underlying physics itself.

**What this most likely means, not yet fully confirmed:** the small
periodic `vel_z` cycle reads as collision/wall-jitter rather than
locomotion — consistent with the terrain dump below, which shows the
target's own layer (z=338) is enclosed by solid Rock on **all 8 lateral
neighbors**. The colonist appears to be bumping against a wall rather than
approaching an open jump-off point; the rare dispatch events may be firing
into that same jitter and being absorbed by it, or may be genuinely
cancelled same-tick (Opus's `unstuck_if`-push / `traverse`-cancel race
hypothesis — `push_cancel_input`'s same-tick-vs-next-tick semantics are
unread and would decide this outright, flagged as a further read, not
chased here).

**Revised verdict: still Fable's branch 2 in effect** (the colonist never
makes progress and the observable physics never shows a real jump), but
the mechanism is sharper than "dispatch never fires" — it fires rarely and
produces nothing, in a context (full rock enclosure at the working layer)
that looks like collision jitter rather than a clean miss. `min sdist =
16.24` (matches the probe exactly), `max sdist = 60.87` — genuine large
excursions elsewhere in the run, but the specific window examined here
never leaves a ~0.08-unit band. `route_exists: true` every sample (per the
raw `timeout_route_states` list) still refutes the astar-reset/"search
never produced a route" hypothesis for this specimen.

## Target-cell terrain dump: fully enclosed at its own layer

`TARGET-TERRAIN-DIAG` at job 20's target `(17989, 9263, 338)`: column
`z=336..338` all `Rock` (unmined), `z=339..341` all `Air`. **All 8
horizontal neighbors at z=338 are `Rock`.** The only open direction is
straight up. This is consistent with — but not identical to — Opus's
corpus-derived read (`b5_mine_cell_diag`'s `standable_target` one z above
`job_pos`, `below_open=2`, `top=true`): the live dump confirms the target
itself is a sealed plug reachable only from above, though `below_open=2`
in the corpus record refers to a different reference frame (cells below
the stand-at position across the wider dig, not this exact column, which
reads fully solid at the mining layer itself) — flagged as a discrepancy
worth resolving before leaning further on either reading alone.

**TGT-DRIFT correlation confirms the target itself never destabilizes
either.** Of 12 `TGT-DRIFT` events in the full run, **none** precede any
of job 20's four timeouts for its own colonist (uid 3 for the first
attempt, uid 1 for the remaining three — the job gets reclaimed by a
different colonist between attempts, but neither colonist's own steer
target is ever reset by an astar-reset before its timeout fires). `steer =
target = (17989.5, 9263.5, 339.0)` holds constant across all four
attempts. Two independent negatives (no astar-reset, route always exists)
converge on the same reading: the failure sits downstream of both
retargeting and search, in whatever decides HOW to execute a route that
was already found.

## Job 2: contrast — the same instrumentation shows real jump attempts elsewhere

For comparison (seed 7, not seed 90, but the same `SDIST-TRACE` build):
job 33's trapped-zone phase shows six distinct `vel_z` spikes to 7.48 with
`on_ground=false`, roughly every 0.5-1.5s (`TRAVEL-ROW-SEED7-CLOCK-RESET-
MECHANISM.md`). **The capability and the dispatch both clearly exist
somewhere in this system** — job 20 is a case where they don't co-occur,
not evidence the jump tier is broadly unavailable.

## The raw route_states list (Opus's correction: read the list, not the pinned flag)

Job 20's target `[17989,9263,338]` in the corpus/harness JSON
(`b5_mine_reachability_probe`) records `route_next_idx_pinned: false` with
raw sequence `[8, 8, 3, 4, 4]` — **not simply pinned, and not simply
advancing either.** The index drops from 8 to 3 then rises to 4: read per
Opus's own engine-doc note, this is consistent with the route being
**recomputed** between samples (a different route each time has a
different waypoint list, so "index 3" in one attempt isn't the same
physical point as "index 3" in another) rather than one route stalling at
a single waypoint. `route_exists: true` in every one of the 5 samples —
the search always produces something, it just isn't the same something
twice.

## Verdict against Fable's three branches

- **(1) ARRIVES → probe defect.** True for job 2 and job 23. The probe's
  `path_exists_step: false` claim does not predict failure — both
  adjacent cells succeeded live despite it.
- **(2) STALLS WITHOUT a jump attempt → mode-dispatch gap.** True for job
  20, physics-confirmed (zero jump-velocity events across the entire
  4-churn lifetime).
- **(3) STALLS WITH a failed jump attempt.** Not observed in this seed's
  specimens — job 33 (a different seed) shows this shape instead, for a
  different job kind (RestAt, not Mine).

**All three of Fable's branches are now live-confirmed to occur, just not
on the same specimen.** The class the offline probe calls "Class B" is not
mechanistically uniform: it contains cells that succeed anyway (probe
over-flags), cells that never attempt the escalation (dispatch gap), and
(elsewhere) cells that attempt and fail (capability/execution gap). A
single fix aimed at "Class B" would be aimed at three different things at
once.

## Caveat on job 2's jump-attempt evidence

Job 2's own `SDIST-TRACE` shows a sustained `vel_z=2.5` with
`on_ground=false` across its early approach (many consecutive ticks) —
this reads more like a steady climb/terrain-following signal than a
discrete jump event (contrast job 33's isolated spikes to 7.48 from a
resting baseline). Not confirmed as "the jump succeeded" — only that the
job did complete. Whether it succeeded via the jump tier, a step path the
offline probe wrongly ruled out, or something else isn't established
here; the ARRIVES verdict stands regardless of mechanism.

## Instrumentation

No new instrumentation this pass — `SDIST-TRACE` (with the
`stuck_strikes`/`on_ground`/`vel_z` extension from the prior commit) and
the existing `b5_mine_reachability_probe` were sufficient. Determinism
cross-check: two independent runs of job 2 (once via a separate stderr
capture, once combined with the SDIST-TRACE run) produced identical
churn/complete timestamps down to the tick, and identical `min_distance_
to_target = 3.78`/`16.24` matching Opus's own corpus read exactly.
