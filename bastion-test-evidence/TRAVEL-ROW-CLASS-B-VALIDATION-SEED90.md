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

## Job 20: the genuine failure, physics-confirmed

`BASTION_SDIST_TRACE_JOB=20` across job 20's full lifetime (4 churn
cycles, 2031 ticks, `stuck_strikes` climbing 0→4): **`on_ground=false`
fires 1781/2031 ticks, but every `vel_z` sampled while airborne is in the
0.005-0.35 range** — ordinary walking-gait air time, not a jump. **Zero
ticks anywhere in the trace show a jump-velocity spike** (contrast job 33
below, which peaks at 7.48). `min sdist = 16.24` (matches the probe's
`min_distance_to_target` exactly), `max sdist = 60.87` — genuine large
excursions, consistent with the seed-7 freeze/jump-elsewhere pattern, but
never once with an actual jump attempt near the target.

**This is Fable's branch 2: stalls WITHOUT attempting a jump.** Not a
capability gap — a dispatch gap. The colonist has a route (`route_exists:
true` every sample, per the raw `timeout_route_states` list, never `false`
— refuting the astar-reset/"search never produced a route" hypothesis for
this specimen), the route never completes, and whatever would trigger a
jump-tier move along that route never fires.

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
