# SCENARIO MAP — canonical status per harness scenario

**This file is the artifact the "map re-count" updates.** It did not exist
before 2026-08-04 — the map previously lived scattered across run-log sweep
tables, which is why nobody could find it. One row per scenario: status,
provenance (the commit/wave that measured it), and what would change it.
**A status without provenance is a claim; do not add one.**

Re-count procedure: run every non-green scenario (and any green one whose
subsystem a merged row touched) against a PINNED tip; update status +
provenance together. Statuses: GREEN · RED · EXPECTED-RED (tracked-red with
fingerprint; red until a named future stage) · RETIRED-GREEN (was red, fixed,
now a regression guard) · NEEDS-REVERIFY (a merged row likely changed it).

## Last full accounting (sweeps at `2b1b3ef0` era + subsequent verified changes)

### Green (23) — sweep-verified at 2b1b3ef0
gather · haulpin · stuckjob · cavein · needs · b4 · b6haul · magnet · coord ·
leash · arena · derive · values · season · belt-exercise · bag1 · season1 ·
archetype · chronicle · chronicle-capture · lod0 · lod1 · inspect

### Retired-green (2)
- **chopfell** — was RED (stance bug + fixture void); fixed by `a057ed66cf` +
  `15850c61cc`; both trees verified twice.
- **b58** — was RED (`c_rungs_placed_expected`); root was a FIXTURE material
  mismatch (stone to a wood job); fixed in #64's material fix, verified 5/5 +
  `c_top_cleared` + `c_no_carve` against the revert control; merged
  `d3235e5329`. *Scenario-level verification; not in the b5 fan.*

### Needs-reverify at current tip (1)
- **auton** — sole failing conjunct was `mine2_count_held`; #63 (the flee/work
  gate, merged `d3235e5329`) demonstrably flips that conjunct true. Full
  conjunction not yet re-run at the merged tip. One run decides GREEN.

### Expected-red (2) — tracked fingerprints, red until AUTON-2
- **preempt** — root `preempted_rested` (rest never engages; needs-as-drives
  deferred by design, Drive enum's own doc comment).
- **b73** — root `ate` (same deferral). In-code EXPECTED-RED note present.

### Red (9) — causes known or queued
- **bed** — root `beds_built`, fails at `plan_access` BEFORE stance/materials.
  **UNRESOLVED, not "no bug"** (status revised 2026-08-04 at `460626a6e2`): the
  discriminator ran (four probes, `probe_incomplete:false`, route found) and
  its POSITIVE result turns out to be uninterpretable — see the instrument
  caveat below. We established that the probe cannot adjudicate this question,
  NOT that `plan_access` is correct. Needs a capsule-aware check to decide.
  The emergency-egress agreement is suggestive but not independent if it
  shares `plan_access`'s capsule logic.
- **selfgen** — root `hauled` (haul stage; upstream of placement).
- **farm** — till/sow late + `farm_tilled:false` unexplained under BOTH stances
  (counter-control); Farm's own control blocked on that mystery.
- **zone** — `zone_freed:false`; genuine conjunct; unclassified.
- **path** — `path_no_starvation` red BUT the metric is a lifetime-cumulative
  `peak_wait` that cannot localise in time — instrument fix (delta-capture)
  filed; seam-3 UNPROVEN pending it.
- **run** — `run_ran_faster:false` (Running not faster than walking);
  movement-layer; unclassified.
- **auton3** — `scores_match:false` (modulation recording); small, scoped.
- **b55** — 15 conjuncts; `remainder_progressed:false` (post-partial-erase
  stall); unclassified.
- **b55-deep** — 21 conditions behind 2 emitted bits (report-fix candidate 7);
  `active_route_owners_at_deadline:0`; unclassified.

### New instruments (2) — green at birth, red-by-design verified
- **blocked_retract** — exercises #61's remove_job prune (prune-disabled build
  FAILS it). ★ Shows RUN-TO-RUN VARIANCE — see hazard below.
- **cavein_conservation** — 7 stones / 7 cells through a real 6-cell collapse;
  #58's permanent guard.

## ★ STANDING HAZARD (filed 2026-08-04): determinism claims are PER-ARTIFACT
Waves 19/21/22/24 prove **b5's 48 seeds** deterministic at full-clause level.
**That is a property of b5, not of the harness**: `blocked_retract` shows
run-to-run variance — two watchdogs on independent cadences
(`ARBITRATION_INTERVAL` vs `tick.0 % 30 == 7`) race, and
`bastion_force_load_area`'s synchronous IO-dependent wait plausibly shifts the
sim's STARTING PHASE (a real-world-timing input, against
determinism-by-construction). Cheap confirmation filed: log `tick.0` at
force_load return across runs. Until resolved, scenario-level comparisons on
phase-sensitive scenarios need their own stability measurement first.

## ★★ STANDING CAVEAT (filed 2026-08-04): the reachability probe is SOUND FOR NEGATIVES ONLY

`offline_reachability_probe`'s flood fill (`bastion_jobs.rs`, the
`column_height_near` / `ascent <= ascent_bound` loop) is **purely
column-based**: it walks 2D `(x,y)` columns and references **no width, no
headroom, no collider** at any step. `has_standable_stance` is the only
body-aware check and it runs **only on the destination cell**. Intermediate
columns get zero clearance modeling.

**So the probe models a POINT; `plan_access` models a CAPSULE. They answer
different questions.** A capsule can only go where a point can go, so the
soundness is asymmetric:

| probe result | status | valid claim |
|---|---|---|
| **NO ROUTE** | **SOUND** | point-unreachable ⇒ capsule-unreachable |
| **ROUTE EXISTS** | **UNSOUND** | says nothing about whether a body fits |

**Cite this instrument for negatives only.** Consequences already applied:

- **seed 80 / #61 — UNAFFECTED, arguably strengthened.** Its claim was a
  NEGATIVE (no route from either vantage). Even the most permissive traversal
  model available found no way in. The first reading of this finding was that
  seed 80 weakened alongside bed; **that is backwards** and the row's n=1
  evidence stands.
- **bed — POSITIVE, therefore void.** Row moved to UNRESOLVED above.

**Filed, not started:** extending clearance/headroom into the per-step column
test would make positives sound too. Real work in a hot path, behavior-
re-rolling; priority sits with the architect.

**Why it went unnoticed through four rows:** the instrument's NAME describes a
broader question than its CONTENTS answer, and four rows read the name — the
label rule in a new costume. It is also `aggregate-late` in a new costume: one
boolean collapsing two different questions, invisible until someone read the
loop.
