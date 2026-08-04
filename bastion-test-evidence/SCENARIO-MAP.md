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

---

# ★ READ THIS FIRST — actionable state as of 2026-08-04 (`ebdc3480f3`)

**The sections below this summary are a chronological research log from one
session of reading fan data already on disk (zero new runs). This block is the
part you act on.** Everything here is measured, not argued; each claim's working
is in the section named.

## ★★★★ BIGGEST FINDING: task #59's starvation hypothesis is SUPPORTED 6/6

`b5_mine_cell_diag[].starvation_cycles` / `.starvation_crowded_cycles` carry a
**documented decision rule** (ratio ≈ 1.0 supports greedy-arbitration
starvation; crowded ≪ starvation is the kill case). **Five of six seeds are
exactly 1.000, the sixth 0.938, and the kill case occurs in ZERO seeds** — every
starved cell had competing unclaimed work ~100% of the cycles it waited, and 360
is the whole run.

**Seed 71 is ARBITRATION-starved, not access-starved** (it emitted 3 access
plans). **Caveat: seeds 52 and 66 hit ratio 1.000 and still mined 27/27, so a
1.0 ratio is not sufficient for failure.** What's supported is the mechanism,
not the outcome — a row owes *why did 71 never recover and 66 did*.

**This was in every fan we have ever run.** Full working in the section of the
same name below.

## ★★★ RED RE-RUN PASS (8 of 9, at `460626a6e2`) — every STATUS was right, a third of the DESCRIPTIONS were wrong

| finding | count |
|---|---|
| descriptions materially wrong | **3** — `run`, `auton3`, `farm` |
| clauses proven UNSATISFIABLE | **1** — `farm_growth_rose` |
| reds proven UNINTERPRETABLE | **1** — `b55` (baseline not emitted) |
| dependent clauses identified | **1** — `farm_sown` |
| rows fully solved | **1** — `farm` = 8-of-9 tilled |
| new cross-scenario patterns | **1** — one-unit-short in ≥3 places |
| **statuses changed** | **0** |

**Headlines:** `run` is a **14.07%-vs-15% near-miss**, not "running isn't
faster". `auton3` is **one score component reading 0.0**, not a broken model.
`farm` was **a count threshold**, not an unexplained mystery. `b55` **cannot be
read at all** until it emits its baseline. `path` is **saturated at its
iteration cap** (`peak_tick_iters == cap == 3000`).

**Not re-run:** `b55-deep` — 21 conditions behind 2 emitted bits; it needs the
verdict/diag split before a run tells anyone anything.

> **The red list should carry NUMBERS, not prose summaries.** Three rows had
> drifted into descriptions that one sub-minute run contradicted.

## Three CLEAN SPECIMENS — one per failure mode, no fixture needed

Deterministic, in the standing 48-seed fan, **attested at the merged tip
`d3235e5329`** (9/9 properties identical to wave19).

| mode | seed | what makes it clean | investigable? |
|---|---|---|---|
| **build** | **62** | mine + chop both cleared, `rescue_fired: false` — only build failed | **no — build has zero diagnostic fields** |
| **mine** | **90** | 3 cells, all claimed by named colonists, `cycles: 0`, nothing blocking, no progress | yes (`mine_cell_diag`) |
| **chop** | **78** | path exists from spawn (complete, 228 cols), nothing blocking, `log_sum: 0` | yes (probe + `blocked_sources`) |

## Instrument work, in priority order — all read-only, no behavior change

1. **PER-ATTEMPT outcome record** (claim granted → completed / timed out /
   preempted / released / material-blocked, with cycle). **Gates DECISIONS
   #53's ARB-STARVATION row**, whose discriminator is provably NOT derivable
   from the current corpus: every per-cell field is coupled to the outcome, and
   `mine_cell_diag` aggregates per CELL while the question is per ATTEMPT.
2. **`build_cell_diag`** on the `mine_cell_diag` pattern. **Build is the largest
   failure mode (6 of 12) and has NO diagnostics** — every build field is a
   constant across all 48 seeds. Seed 62 can be isolated but not investigated.
3. **Add `blocked_sources` to `mine_cell_diag`** (already wired for chop). One
   line; converts seeds 54 and 61 from "blocked by something" to "blocked by a
   named mechanism."
4. **Per-call-site access-emission counters** — the un-park condition for the
   seam row (DECISIONS #52).

**★ When building #1, exclude `starvation_cycles: 0` DELIBERATELY** — the hook's
own doc says all-zero also means *"never open/unclaimed during arbitration"*, so
zero is ambiguous between "never starved" and "never open."

## Do NOT trust these fields

- **`b5_55_*` family** (`blocked_by`, `names_blocker`, `clears_on_cancel`,
  `notified_once`, `diag`) — **constant across all 48 seeds, inert.**
  `55_diag`'s `unreachable: true` looks like a diagnosis and is not.
- **`b5_drift_events`** — documented non-discriminating, and verified so.
- **`tool_stone`/`tool_steel` at `0.0`** — that is the `.unwrap_or(0.0)`
  sentinel, **below the metric's own 1.0 floor**. Seed 66's failure is very
  likely instrument, not product.
- **Constant ≠ broken:** `flat_hint_decoupled` and `slope_cancel_clean` are
  constant-true **regression guards doing their job**. Ask what a field is FOR
  before judging its constancy.

## Corrections issued this session (do not cite the superseded versions)

| claim | status |
|---|---|
| "`plan_access` models a capsule" | **WRONG** — it is a construction planner |
| "zero `probe_incomplete` corpus-wide" | **WRONG** — a type guard dropped every chop probe; seed 92 is incomplete |
| "the corpus has zero access-plan visibility" | **WRONG** — `b5_cascade_probe.access_emissions_max` exists (nested) |
| "a seam-row green would be a FALSE GREEN" | **too broad** — the corpus demonstrably moves |
| "seed 71 should improve if the bar is removed" | **REFUTED** — it emitted 3 plans, never starved |
| "perfect clause pairing = one mechanism twice" | **true for 3 of 4 pairs**; mine hides a 19–96% spectrum |
| "`unreachable` cells discriminate 71 from 66" | **WITHDRAWN** — `unreachable` = "all six faces currently solid", downstream of dig progress |
| "jobs persist on already-mined cells" | **WITHDRAWN** — `open_cells` means *has a job*, not *is open*; my reading was inverted |
| "the point-model STRENGTHENS seed 80" | **REVERSED** — true for body-width, false for column-collapse; soundness direction belongs to the ERROR MODEL |
| "`auton3` is a model/computation gap" | **REVERSED** — **FIXTURE**: the harness hardcodes the work-urgency input; the engine gates it correctly |
| "`run` is a threshold conversation" | **REVERSED** — the bar (1.15) sits *below* design intent (1.25); a real gap |

**★ Three of these six are the same error: I substituted a NAME for its
CONTENT** (`plan_access`, the top-level-only field scan, `unreachable`) — twice
*after* writing the rule against it. **The tell is speed: it happens on the
promising thread, never on the careful one.** The countermeasure that actually
works is reading the definition **before** building on it, not after.

## Claims CHECKED that HELD

Task #61's *"the corpus's only genuinely-unreachable chop case (seed 80)"*;
`vm-pool.sh`'s *"+dirty = LFS noise, code clean via reset --hard"*;
`drift_events`' documented non-discrimination. **The comments in this tree are
mostly right — record survivals so the checking habit stays credible.**

---

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

### Needs-reverify at current tip (0) — ★ RESOLVED 2026-08-04

- **auton — NOW GREEN**, verified at `460626a6e2`. **All 14 conjuncts true**,
  including the sole previously-failing `mine2_count_held`, and
  `verdict_matches_legacy: true` (the report-fix refactor still reproduces the
  legacy verdict exactly). `failed_clauses: []`, `root_failure: null`.

  **★ The prerequisite engaged**: `storm_baseline_captured: true`, so
  `mine2_count_held` is a real reading rather than the unmeasured case its own
  `Some("storm_baseline_captured")` guard exists to catch. **A green on a term
  whose prerequisite was dark would have proved nothing.**

  **Provenance** — built and run locally, not on the fan:
  - binary `--print-git-hash` → `460626a6e2`, checkout `460626a6e2`,
    **matched, and no `+dirty`** (clean worktree, unlike the VMs' LFS noise)
  - built 2026-08-04 05:30 from a cold `no_overflow` profile in
    `.wip64-guard-revert-wt` detached at the tip; `builder5` untouched
  - `RUN_EXIT=0` with a **72,102-byte** log — not an exit-0-with-empty-log

  **★ STABILITY MEASURED — 3/3, not a single-run lottery.** The map's own
  standing hazard says phase-sensitive scenarios need their own stability
  measurement, and auton is a drive-storm scenario. Re-ran it twice more on the
  same binary: **all 14 conjuncts identical in all three runs**, `failed_clauses`
  empty each time, `verdict_matches_legacy: true` each time.

  Log sizes differ (72102 / 72091 / 72435), so the runs are **not byte-identical**
  — but with timestamps stripped only **8 lines** differ, and all of them are
  per-run identity noise: a random **boot UUID** and the **per-run temp datadir
  path** (the harness selects its own datadir; `VELOREN_USERDATA` did not apply).
  **No simulation variance reaches any conjunct.** Stated precisely: *auton's
  verdict is stable; auton's log is not byte-reproducible, for reasons unrelated
  to the sim.*

  **This clears the map's last NEEDS-REVERIFY row; the re-count is complete.**

### Expected-red (2) — tracked fingerprints, red until AUTON-2
- **preempt** — root `preempted_rested` (rest never engages; needs-as-drives
  deferred by design, Drive enum's own doc comment).
- **b73** — root `ate` (same deferral). In-code EXPECTED-RED note present.
  ★★ **STATUS AMENDED 2026-08-04: EXPECTED-RED (FINGERPRINT UNVERIFIABLE).**
  The unsatisfiable-watch sweep found **two of b73's watches emit no baseline** —
  `attempts0 → broke` and `jobs_frozen_at → resumed_after_break`. The M3A
  precedent requires a tracked red to **HOLD its fingerprint**, but a fingerprint
  resting on flags whose baselines nobody can see **cannot be observed to hold or
  shift.** **No stability claims about b73 until its baselines are emitted** —
  those two emissions are **REQUIRED** items in the report-fix window, not
  optional hardening. *The AUTON-2 deferral is unaffected — that is design
  intent, not fingerprint-dependent.*

### Red (9) — NUMBERS FORMAT (ratified 2026-08-04): failing clause · measured · threshold · tip

**Prose demoted to one clause of context.** The re-run pass found three rows
whose *descriptions* were wrong while their statuses were right — **a
description that IS the number cannot drift.** All measurements at
`460626a6e2` unless noted.

| scenario | failing clause | measured | threshold / expected | verdict |
|---|---|---|---|---|
| **run** | `run_ran_faster` | **1.1407** (walk 0.263 → run 0.300), **deterministic ×4** | bar **1.15**; **design intent 1.25** (`TRAVEL_SPEED 0.8` / `RUN_SPEED 1.0`) | **NOT threshold** (bar < design) · **NOT noise** (4-run identity) · **NOT yet attributed** — real gap vs deterministic window overhead, **batch item 7** |
| **farm** | `farm_tilled` | **8** tilled, **9** till jobs created | `tilled_count == 9` | **CLASS (a)** job created / work never performed. Missing cell `(24072,20239)` — a **corner**, not the `plot.min` probe corner. `farm_sown` is **downstream**; `farm_growth_rose` is **UNSATISFIABLE** (`g1=15`, needs `>15`, loop breaks at `>=15`) |
| **auton3** | `auton3_scores_match` | got `(0.0, 0.0, 0.0800)` / `(0.0, 0.0, 0.1200)` | pred `(0.6, 0.0, 0.0800)` / `(0.4, 0.0, 0.1200)` | **FIXTURE DEFECT.** Harness `predict` hardcodes work-urgency `0.5`; engine gates it on `work_sig` (`URGENCY_WORK = 0.5`). Engine's `0.0` is **correct**; the harness omits the gate. Only component 3 discriminates — component 2 is a constant |
| **b55** | `remainder_progressed` | `jobs_in_half` 18 → 0, `board_after_whole` 0, orphans 0, stone 200 → 200 | needs `total < remainder_before` | **UNINTERPRETABLE** — `remainder_before` is **never emitted**, so "stalled" and "nothing left to progress" are indistinguishable |
| **path** | `path_no_starvation` | `grants` 10452 · **`peak_tick_iters` 3000** · `peak_wait` 75 | **`cap` 3000** | **SATURATED AT CAP.** `cap_held: true` reads as reassurance and means *pinned at ceiling*. `peak_wait` is lifetime-cumulative (cannot localise; delta-capture filed) |
| **bed** | `beds_built` | colonist stands at **z=445**, plateau floor **449**, spawn **457**; min-dist to bed never < **7.7** over 6000 ticks | reach a bed | **FIXTURE GEOMETRY DEFECT** (5b's walk-test + block query): natural cavity under the plateau; the fill seals `gz-6..gz` from above without reaching real ground. **Not `plan_access`, not the probe** |
| **selfgen** | `hauled` | `mine=4 build=4 plans_done=0 fires=0` | — | root is the **haul stage**, upstream of placement; unclassified |
| **zone** | `zone_freed` | `colonists 4 · in_control 5 · in_zone 870 · jobs 1` | — | sole false term; unclassified, **now with a tip** |
| **b55-deep** | `active_route_owners_at_deadline` | **2 emitted bits behind 21 conditions** | — | **NOT RE-RUN** — a run cannot say anything new until the verdict/diag split lands (report-fix candidate 7) |

**Two of the nine are INSTRUMENT-SUSPECT, not game bugs** (`auton3`'s ungated
prediction, and `b55` which cannot be read at all) — plus **seed 66's sentinel**
inside the corpus. **2 of 12 corpus failures instrument-suspect tracks the
historical ~10.4% fixture-false-failure prior almost exactly**, which is why
that prior is the FIRST question of every triage: **asked first twice today,
paid both times.**

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

## ★★ STANDING CAVEAT (filed 2026-08-04): probe soundness is PER-ERROR-MODEL
### ⚠️ HEADING SUPERSEDED — this section originally read "SOUND FOR NEGATIVES ONLY", which is the UNSCOPED form. See the error-model section below: negatives are sound under BODY-WIDTH in SINGLE-LAYER terrain only; under COLUMN-COLLAPSE both directions are unsound.

`offline_reachability_probe`'s flood fill (`bastion_jobs.rs`, the
`column_height_near` / `ascent <= ascent_bound` loop) is **purely
column-based**: it walks 2D `(x,y)` columns and references **no width, no
headroom, no collider** at any step. `has_standable_stance` is the only
body-aware check and it runs **only on the destination cell**. Intermediate
columns get zero clearance modeling.

**So the probe models a POINT — it answers "can a dimensionless walker get
there".** That is narrower than "can a colonist get there", and the soundness
of a point model is asymmetric:

| probe result | status **under BODY-WIDTH only** | valid claim |
|---|---|---|
| **NO ROUTE** | **SOUND** *(single-layer terrain only)* | point-unreachable ⇒ capsule-unreachable |
| **ROUTE EXISTS** | **UNSOUND** | says nothing about whether a body fits |

> **⚠️ THIS TABLE IS SCOPED TO ONE ERROR MODEL.** The column-collapse model
> (`column_height_near` returns the HIGHEST solid per column) makes **BOTH**
> directions unsound in multi-layer terrain — see the error-model section at the
> end of this file. **Cite as: "under the body-width model in single-layer
> terrain, negatives are sound."** Never bare. Consequences already applied:

- **seed 80 / #61 — ⚠️ SUPERSEDED, see the error-model section. Originally read "UNAFFECTED, arguably strengthened"; that holds only under body-width. Now CONDITIONAL on seed 80's site being single-layer — batch item 6.**
- *(original text, retained for the record)* **UNAFFECTED, arguably strengthened.** Its claim was a
  NEGATIVE (no route from either vantage). Even the most permissive traversal
  model available found no way in. The first reading of this finding was that
  seed 80 weakened alongside bed; **that is backwards** and the row's n=1
  evidence stands.
- **bed — POSITIVE, therefore void.** Row moved to UNRESOLVED above.

**Filed, not started:** extending clearance/headroom into the per-step column
test would make positives sound too. Real work in a hot path, behavior-
re-rolling; priority sits with the architect.

### ★★ CORRECTION, same day, one hour later — `plan_access` IS NOT A REACHABILITY CHECK

The paragraph above originally read *"the probe models a POINT, `plan_access`
models a CAPSULE."* **The second half was my inference from the NAME and it is
wrong** — written into the very commit whose closing line warns that four rows
read this instrument's name instead of its contents.

**`plan_access` is a CONSTRUCTION PLANNER, not a predicate.** It takes
`&mut JobBoard`, returns `Option<(DesignationKind, usize)>`, and EMITS digging
designations to *create* access — carve-ramp stairs first, then a ladder
pillar, then an emergency escape shaft. `None` means **"I could not build a
way there,"** never **"a colonist cannot fit."** Nothing in it models a body.

**So bed's rejection has candidate causes with no relation to body width:**

1. **The ladder tier is DARK at two of three call sites.**
   `const AUTO_LADDER_ACCESS: bool = false` (B6 hotfix — the auto-pillar caused
   a queue-fight). The tier lights only via `emergency_owner.is_some()` or
   `dig_provisioned && DIG_PROVISIONED_LADDER_ACCESS`. The **self-rescue call
   site passes `false, None, None`** — so if `carve_ramp` finds no stair, that
   site returns `None` with no fallback. Call sites: self-rescue (stairs only),
   emergency (ladder live), proactive descent (ladder live).
2. **Cell contention.** `unavailable_cells` is seeded from *every* live job's
   `pos` plus emergency route cells; any tier rejects a plan intersecting it.
3. **Stair geometry.** `carve_ramp` plus the walkability rule (no two
   consecutive digs rising in the same XY column).

**Which call site bed reaches is not yet established** and is the next
question, not an assumption.

**And the B6 comment carries its own unnamed-case claim** — *"the universal
teleport-to-ground fail-safe backstops any colonist a stair can't reach"* —
true for a **trapped colonist**, false for a **job needing access to a work
face**. The fail-safe rescues bodies; it does not grant a job access. Same
species as the T3.52 `sufficient` comment.

**What survives from the original finding:** everything about the probe. It is
column-based, its negatives are sound, its positives are uninterpretable as
claims about a colonist, seed 80 stands and bed's positive is void. **What does
not survive is the account of what the probe was being compared AGAINST.**

**Why it went unnoticed through four rows:** the instrument's NAME describes a
broader question than its CONTENTS answer, and four rows read the name — the
label rule in a new costume. It is also `aggregate-late` in a new costume: one
boolean collapsing two different questions, invisible until someone read the
loop.

## ★★★ SEAM FINDING (2026-08-04, `460626a6e2`): the global one-plan bar is fixed at ONE of two callers

`plan_access` has three call sites. Two of them gate *whether it is called at
all*, and they disagree.

**Self-rescue (B5.8 autonomous access) — colony-global bar, STILL LIVE:**
```rust
let access_pending = board.jobs.values().any(|j| j.is_access);
for (from, to, parent) in carve_requests.into_iter()
        .take(if access_pending { 0 } else { 1 })
```
While ANY access job exists anywhere in the colony, the loop body never runs —
`plan_access` is **never called**, for any stuck colonist, regardless of
geometry or distance.

**Proactive descent (DPA-1) — pocket-scoped at Chebyshev 8**, with this comment
in the tree today:

> *"selection above already skips pockets with live access jobs — the
> M2-precedent pocket scoping; **the old colony-global `!any(is_access)` bar
> starved every second dig**."*

**The codebase declares the bar a starvation bug at one caller while the other
caller still runs it.** `7c37ddbad5` (DPA-0/1/2) scoped the fix explicitly to
*"descent_plan selection"*; self-rescue was not in the change.

**And the shared callee's comment describes the fixed state as global.**
`plan_access`'s M2 PLANNER-FIX comment ends *"…and disjoint pockets plan in
parallel"* — true for the descent caller, false for the self-rescue caller. A
**sufficiency claim at a callee about its callers**, which the comment grep
cannot catch because the false part is again the unstated scope.

**★ It was observed once and ABSORBED, not fixed.** Run-17 (`2af7ee3ade`)
logged *"12 permanent unreachable leftovers freezing one-plan-at-a-time"*; the
response was the **F3 stale-access pruner** (access job unclaimed 20s → plan
abandoned). That bounds the damage only when access jobs go UNCLAIMED — a
claimed, actively-worked stair dig holds the bar for its whole duration. The
absorption is why the gate survived: **find the absorber before correcting the
value** ([[refusal-needs-refusal-aware-consumers]]).

**NOT established: that this is bed's root.** Falsifiable, instrument already
exists (`access_job_dump`, `access_block_reason`, both read-only, both shipped
in `7c37ddbad5`). Prediction: *if any `is_access` job is live during bed's
window, the self-rescue path emitted zero plans and `plan_access` was never
called.* Check the precondition engaged before reading the verdict.

**Removal is architect-gated** — one line, well-documented replacement pattern
at the sibling caller, but squarely behavior-re-rolling and needs a fan.

## ★★★ CORPUS BLINDNESS + a free repro for the offline/live split (2026-08-04, wave19/21)

Both found by reading `wave19_FULL.json`, no new runs.

### The corpus cannot see access planning at all

**75 fields; none report access-plan state** — no `is_access` count, no plan
emissions, no `access_pending`, no `carve_attempted`, no unreachable-flag
count. The one access-adjacent field,
`b5_ch_scan_incomplete_unreachable_columns`, is **0 across all 48 seeds**.

**And `b5_rescue_fired` is TRUE in 44 of 48 seeds** — the self-rescue path
behind the colony-global `take(0)` bar is exercised in nearly every seed and
reported by nothing.

> **Removing that bar could produce ZERO corpus drift and still be a real
> behavior change. A green fan on that row would be a FALSE GREEN.**

The bar for the seam row must be pre-stated on a **new instrument that exists
before the row does**. This also explains the gate's survival: it lived through
every fan because **no fan could see it**.

### Seeds 52 and 54 are a deterministic repro of the offline/live split

Across all 48 seeds' `b5_mine_reachability_probe` / `b5_chop_reachability_probe`:

| bucket | entries | seeds |
|---|---|---|
| **offline YES, live router NO** | **10** | **52, 54** |
| offline NO, live NO | 4 | 52 |
| live route existed | 26 | 54, 61, 66, 71, 90 |
| **probe incomplete** | **0** | — |

SPLIT = `path_exists_step: true` + `probe_incomplete: false` while the live
`timeout_route_states` report `route_exists: false`. **This is bed's exact
discrepancy in a scenario with no beds and no `plan_access` involvement** — so
the split is general, not a bed fixture artifact.

**Determinism verified, not assumed.** Raw-log signature across two independent
fans at the same commit (`ed532c600e`): **wave19 → 26, wave21 → 26. Identical.**

~~**Zero `probe_incomplete` in the entire corpus**~~ — **RETRACTED, see the chop
section below.** This scan covered only `b5_mine_reachability_probe`; a type
guard silently dropped every `b5_chop_reachability_probe` (the two fields have
different container types). **Seed 92's chop probe IS incomplete on both
vantages**, at the ~100k column cap. The MINE probes are all complete; that much
holds.

**MECHANISM NOT CLAIMED.** Point-model vs body-aware live router, router path
budget, and chunk-load state at timeout are all live candidates. The value here
is a **cheap deterministic handle**, not a diagnosis — study 52/54 rather than
building a bed fixture, then carry the answer back to bed.

### ★ Instrument provenance note
The newest local `bastion-harness.exe` is **2026-08-03 19:01**, predating all
three of today's merges (`#64` 02:29, `d3235e5329` 03:09, `460626a6e2` 04:17).
**Nothing local can measure today's tip until a rebuild.** Caught by attesting
provenance rather than checking that a binary existed.

## ★★★ CORPUS MODE ANALYSIS (2026-08-04, wave19) — "12/48 failures" is really ~5 MODES

Derived from `b5_failed_clauses`, cross-validated against `wave19_VERDICTS.json`:
**the 12 seeds with non-empty clause lists are EXACTLY the 12 FAIL verdicts.**
The field is present on all 48 seeds (36 empty, 12 non-empty) — no missing-key
ambiguity. Analysis is sound to read.

### The clause pairs are PERFECT — two fields, one mechanism

| pair | seeds | co-occurrence |
|---|---|---|
| `any_needs_materials` == `build_placed` | 61, 62, 71, 80, 85, 92 | **6/6, never apart** |
| `chop_cleared` == `log_sum` | 78, 80, 85, 92 | **4/4, never apart** |
| `mine_blocks_mined` == `mine_cleared` | 54, 61, 71, 90 | **4/4, never apart** |
| `b15_adjacent_claimed` == `b15_ontop_claimed` | 71 | 1/1 |

**11 distinct clause names collapse to ~5 independent axes.** Each perfect pair
is one mechanism reported twice — [[aggregate-late-keep-the-structure]]
inverted: not one aggregate hiding structure, but two fields duplicating one
fact, which inflates an apparent failure count.

### Mode membership

| seed | modes | extra |
|---|---|---|
| 52 | — | `ch_leaf_cleared` |
| 54 | **B mine** | |
| 61 | **A materials/build + B mine** | |
| 62 | **A materials/build** | |
| 66 | — | `tl_ok` |
| 68 | — | `ch_mixed` |
| 71 | **A + B** | `b15_*` pair |
| 78 | **C chop** | |
| 80 | **A + C** | |
| 85 | **A + C** | |
| 90 | **B mine** | |
| 92 | **A + C** | `ch_mixed` |

**★ Mode A (materials/build) is in 6 of 12 failures — half the corpus's failing
seeds — and never appears without `build_placed` also failing.** It is the
single largest mode and it is materials-shaped, which puts it adjacent to the
DPA-0 material work and the b58 fixture material mismatch (stone handed to a
wood job). **Highest-leverage target by seed count.**

**NOT CLAIMED: that a mode is one ROOT.** Perfect clause pairing establishes
that the two *fields* move together, not that the six *seeds* share a cause.
Per [[matched-control-must-match-on-system-and-axis]], each seed needs its own
minimal control before any multi-seed fix is built.

**Confirms two earlier readings.** Seed 61 (parked carve-cascade) carries A+B
and seed 80 (#61's evidence) carries A+C — both multi-mode, consistent with the
32-clause sweep that killed the cascade pair. **And both SPLIT seeds (52, 54)
are FAIL seeds**, so the offline/live reachability split co-occurs with real
failures rather than sitting in healthy runs.

### ★★★ MODE A CORRECTED — it is NOT material starvation (2026-08-04)

I first described Mode A as *"materials-shaped, adjacent to the DPA-0 material
work and the b58 stone-to-a-wood-job mismatch."* **Wrong, and the data refutes
it with a zero count rather than an argument.**

Harness definitions at `460626a6e2` (`bastion-harness/src/main.rs`):

```rust
build_placed = server.bastion_block_kind(build_ok_pos).is_some_and(|k| k.is_filled());
let any_needs_materials = server.bastion_any_job_needs_materials();
```

`build_placed` is a **terrain read** — did the block actually get built.
`any_needs_materials` is a **live board query sampled once, after the settle
loop** — is any job currently waiting on materials.

**Joint distribution across all 48 seeds:**

| `build_placed` | `any_needs_materials` | seeds |
|---|---|---|
| false | false | **6** (all of Mode A) |
| true | true | 42 |
| **true** | **false** | **0** |
| **false** | **true** | **0** |

> **If builds were failing for want of materials, Mode A would read
> `build_placed:false` + `any_needs_materials:TRUE`. That combination occurs
> ZERO times in 48 seeds. Mode A is not material starvation.**

**What it IS consistent with:** `build_ok_jobs = 1` and `build_stall_jobs = 1`
in Mode A — **identical to passing seeds** — so the build job *exists*. It
simply never progresses far enough to request materials, and never produces a
block. **The failure is upstream of materials entirely.**

**Direction still not established.** Perfect co-variation plus these
definitions cannot separate "no placement ⇒ nothing ever requests materials"
from a shared upstream cause. **What it does establish is the exclusion above**,
which is the part that changes the row's target.

**Consequence for the pair rule:** `any_needs_materials` carries **no
independent information** in this corpus — it is a dependent of `build_placed`.
Counting it as a second failed clause inflates the apparent failure count
without adding a symptom. Same for the other perfect pairs until each is
checked the same way.

### ★★★ THE PAIR RULE WAS TOO COARSE — "perfect pairing" is not "redundant"

I derived the pair rule from **clause co-occurrence** (pass/fail booleans) and
called each pair "one mechanism reported twice." Checking the underlying
**values** splits the four pairs into two opposite kinds:

**REDUNDANT — the second field restates the first:**

| pair | joint values | verdict |
|---|---|---|
| `chop_cleared` / `log_sum` | `true↔1` (44), `false↔0` (4) | `log_sum` is a direct restatement; **zero independent information** |
| `build_placed` / `any_needs_materials` | `true↔true` (42), `false↔false` (6) | dependent; **zero independent information** |
| `b15_ontop` / `b15_adjacent` | `true↔true` (47), `false↔false` (1) | **n=1 — cannot distinguish either way** |

**NOT REDUNDANT — the count preserves what the boolean destroys:**

| seed | `mine_blocks_mined` | share | other clauses |
|---|---|---|---|
| 71 | **5/27** | 19% | + Mode A + both `b15_*` |
| 54 | **16/27** | 59% | none |
| 90 | **25/27** | 93% | none |
| 61 | **26/27** | 96% | + Mode A |

> **"Mode B mine" is not one mode. It is a SPECTRUM from 19% to 96%, and
> 5/27 versus 26/27 are almost certainly different problems.** The boolean
> `mine_cleared` collapses all four into one symptom; `mine_blocks_mined`
> preserves the structure. Textbook [[aggregate-late-keep-the-structure]] — and
> here the un-collapsed field was sitting in the report the whole time.

**★ Seeds 61 and 90 are one block short (26/27, 25/27)** — that is a different
claim from seed 71's 5/27, and it sits right next to the harness's own note
about the settle window (*"the remaining run-to-run variance is ASYNC
SCHEDULING … occasionally left the last mine block one window short"*). **A
near-miss and a near-total failure must not share a row.**

**★ Seed 71 fails in three systems at once** (mine 5/27, Mode A, both `b15_*`)
and **seed 80 fails chop + Mode A**. Both are multi-system seeds, not clean
single-mode specimens — the same shape that voided the carve-cascade pair.
**Do not use 71 or 80 as a control for anything.**

**The lesson on my own method:** I read the pass/fail *labels* to build the
mode map instead of the field *values* — the day's own error class, applied to
my own analysis. **A pair that always fails together may still carry different
magnitudes, and the magnitudes are where the modes actually separate.**

### ★★★ "ONE BLOCK SHORT" IS NOT THE ASYNC TAIL — REFUTED, so seeds 61/90 are REAL

I proposed that seeds 61 (26/27) and 90 (25/27) might be the settle-window
scheduling tail the harness comment describes, i.e. instrument noise rather
than defects. **Tested immediately against the two independent fans. It fails.**

`b5_mine_blocks_mined` multiset, wave19 vs wave21 — **8 different VMs, two
separate fan runs:**

```
wave19:  44 x 27,  and 5, 16, 25, 26  (once each)
wave21:  44 x 27,  and 5, 16, 25, 26  (once each)      IDENTICAL
```

> **Scheduling noise varies run to run. These do not — they are bit-identical
> across independent fans on separate machines. The partial mines are
> DETERMINISTIC, therefore real.**

**Seeds 61 and 90 are genuine one-block-short defects, not tail.** They do not
get shrugged off, and they still must not share a row with seed 71's 5/27.

**This also re-reads the harness comment.** The settle loop was widened
120 → 180 with the note *"the remaining run-to-run variance is ASYNC SCHEDULING
… occasionally left the last mine block one window short."* Our residual is
**not** run-to-run variance — so either the widening fixed the async case and
this is a different mechanism wearing the same symptom, or the original
diagnosis was wrong. **Either way the comment no longer covers what we observe**
([[sufficiency-claims-must-name-their-case]]: a dated rationale stays true only
against the code it was written for).

**★ Method note worth keeping: determinism is a DIAGNOSTIC, not just a
verification property.** "Does it reproduce exactly across independent runs?"
separates instrument noise from real defect in one grep, with data already on
disk and no new runs. **Reach for it whenever a failure is about to be
dismissed as flake.**

## ★★★ THE FOUR MINE FAILURES HAVE FOUR DIFFERENT SIGNATURES (2026-08-04, wave19)

All four were already fully diagnosed **in fields the corpus has been carrying
all along**. Nobody had read `b5_mine_cell_diag`. Field semantics READ before
use (`blocked_by` = `JobBoard::blocked_by`, first-match over `blocked_regions`,
returning that region's `blocking_cell`).

| seed | mined | cells in diag | blocked region? | claim state | signature |
|---|---|---|---|---|---|
| **90** | 25/27 | 3 | **none** | **all 3 claimed by named colonists, `cycles=0`** | **claimed, unblocked, NOT PROGRESSING** |
| **61** | 26/27 | 3 | yes — blocker `[24515,26191,164]`, an **adjacent column** | claimant `None`, cycles 0–6 | blocked by a neighbouring column |
| **54** | 16/27 | 18 | yes — blocker `[26659,4849,176]`, **itself a cell in the set** | one cell claimed | one region swallowing 18 cells |
| **71** | 5/27 | 27 | **none** | **mostly `cycles=360` (never claimed)**; only top-layer/frontier cells claimed recently | claim/arbitration never engaged the volume |

> **"Mode B mine" is not one mode and not two. It is FOUR seeds with four
> distinct signatures**, and the magnitude ordering (5 → 16 → 25 → 26) tracks
> them. Merging them into one row would have been a four-way confound.

**★ Seed 90 is the cleanest specimen in the corpus.** Three cells, all
*actively claimed by named colonists* at `cycles_since_last_claim = 0`,
`blocked_by = None`, and two blocks never mined. **Colonists hold the job,
nothing blocks it, no progress happens.** No confound to strip. If a
"claimed but not progressing" row is ever built, this is its subject.

**★ Seed 71 is the opposite** — 27 of 27 cells in the diag, no blocked region,
`cycles_since_last_claim = 360` (the whole run) on the interior cells.
**Arbitration never engaged the volume at all.** Note the top-layer /
`is_column_frontier` cells DO show recent claims (88–148) — so the frontier was
worked and the interior never became claimable.

### ★★ INSTRUMENT GAP — `blocked_sources` exists and the mine diag doesn't use it

`JobBoard::blocked_by` is a **first-match scalar**. Task #61 built
`blocked_sources` precisely because of that, and its doc says so outright:

> *"A scalar first-match here would silently hide whichever mechanism pushed
> second … this proved that a task #61 candidate lazy chop probe never
> independently fired on the corpus's only genuinely-unreachable chop case."*

**It is wired for chop (`b5_ch_base_blocked_sources`) and NOT for mine.** So for
seeds 54 and 61 we know a block exists and which cell — but **not which
mechanism recorded it**, which is the exact ambiguity that infrastructure was
built to remove. **Adding `blocked_sources` to `mine_cell_diag` is a one-line
read-only probe change, no behavior impact**, and it converts two of the four
signatures from "blocked by something" into "blocked by a named mechanism."
Highest instrument-value-per-line available right now.

### ★ `b5_55_diag` IS A CONSTANT — do not read it as a finding

`{"claimant": null, "progress": 0.0, "unreachable": true}` in **47 of 48
seeds**, `null` in one — **including all 36 PASSING seeds**. `unreachable:true`
looks like a diagnosis and carries **zero information**. **A diag that reports
the same value in passing and failing runs is not a diagnostic**, and this one
sits under a scenario (b55) currently tracked RED. Check any `_diag` field's
distribution across passing seeds before quoting it.

## ★★★ THE FOUR CHOP FAILURES — four signatures, and a CORRECTION to my own claim

### ★ CORRECTION FIRST: "zero `probe_incomplete` corpus-wide" was WRONG

I stated twice — in the corpus-blindness entry and to the architect — that
**every probe in the corpus ran to completion**. **False.** My scan guarded on
`isinstance(entry, list)`, and **`b5_chop_reachability_probe` is a DICT while
`b5_mine_reachability_probe` is a LIST.** The guard silently dropped every chop
probe and I reported the remainder as the total.

> **A filter that skips a whole class and reports the rest as the total is the
> same failure as an empty log read as a pass.** "No hits" and "not examined"
> arrived at the same value again — in my own analysis code, on the day's own
> theme. **Schema inconsistency between two sibling fields is the trap; assert
> the type, never guard it away.**

**Seed 92's chop probe is `probe_incomplete: true` on BOTH vantages** at
`columns_visited_step: 99890` — it hit the ~100k cap. **That is UNKNOWN, not
unreachable** (the three-way reading).

### The four signatures

| seed | `blocked_sources` | probe from spawn | probe from last timeout | reading |
|---|---|---|---|---|
| **80** | `['plan_access']` | **False, complete, 81371 cols** | **False, complete** | **GENUINELY UNREACHABLE — sound negative both vantages** |
| **85** | `['plan_access']` | False, complete, 62265 cols | **TRUE (23 cols)** | unreachable from spawn, **reachable from where the colonist stood** |
| **92** | `['plan_access']` | **INCOMPLETE, 99890 cols (cap)** | **INCOMPLETE** | **UNKNOWN — the probe never finished** |
| **78** | `[]` — **no block recorded** | **TRUE, complete, 228 cols** | False (scramble True) | **UNEXPLAINED** — reachable, nothing blocking, `log_sum: 0` |

**Three seeds share `blocked_sources: ['plan_access']` and are three different
situations.** The shared source was the reason to think they were one mode; the
probe evidence separates them.

### ★★ Task #61's "only genuinely-unreachable chop case" — CHECKED, and it HOLDS

The `blocked_sources` doc says the parked probe *"never independently fired on
the corpus's only genuinely-unreachable chop case (b5 seed 80, covered earlier
by `plan_access` alone)."* **"Only" is a sufficiency-shaped claim and I expected
it to have an unnamed scope — it does not.** Of the three `plan_access`-sourced
chop blocks, **85 is reachable from the colonist's own position, 92 is unknown,
78 isn't blocked at all. Seed 80 really is the only one.** The parking rationale
stands on independent evidence, and per today's probe caveat seed 80's negative
is the SOUND half of the instrument. **A claim that survives checking should be
recorded as having survived.**

### ★ Seed 78 is the corpus's cleanest unexplained chop failure

Path exists from spawn (complete, 228 columns), **nothing recorded a block**,
and still zero logs. No reachability story available. **It is to chop what seed
90 is to mine** — the specimen with no confound to strip.

## ★★★ INSTRUMENT DEPTH BY MODE — build is the LARGEST mode and has ZERO diagnostics

The three big modes are not equally investigable. This is the actionable output
of the whole corpus read.

| mode | seeds | diagnostic fields available | signatures separable? |
|---|---|---|---|
| **build (Mode A)** | **6 — largest** | **NONE** | **no** |
| mine | 4 | `mine_cell_diag` (per-cell claimant, `cycles_since_last_claim`, `blocked_by`, frontier/top flags) | **yes — 4 distinct** |
| chop | 4 | `ch_base_blocked_sources` + full reachability probe (both vantages, `probe_incomplete`, columns visited) | **yes — 4 distinct** |

**Every build-related field is a CONSTANT across all 48 seeds:**

```
build_ok_jobs         = 1     in all 48
build_stall_jobs      = 1     in all 48
build_stall_untouched = true  in all 48
b15_floater_skipped   = true  in all 48
b15_ontop/adjacent    = true  in all 48 except seed 71
```

**`build_stall_jobs: 1` and `build_stall_untouched: true` hold in PASSING seeds
too** — they describe an intentional stall fixture, not a failure. Another
alarming-looking constant, same trap as `b5_55_diag`.

> **Half the corpus's failures live in the one subsystem the corpus cannot say
> anything about.** Mine and chop each yielded four separable signatures from
> fields already recorded; build yields the bare fact that it didn't happen.

**Instrument priority is therefore unambiguous: a `build_cell_diag` on the
pattern of `mine_cell_diag`** (claimant, `cycles_since_last_claim`, `blocked_by`
+ `blocked_sources`, material state at the target cell). **Highest
coverage-per-line available, and it is read-only probe work** — no behavior
change, no fan required to land it.

### ★ METHOD CORRECTION — my disjointness test produces false positives

The scan that produced this compared **value sets** for Mode-A vs passing seeds
and flagged any field with no overlap. That is **wrong for fields whose values
are unique per seed**: `ch_ground_truth_witness` (a position), `locomotion`
(counters) and `soak_avg_tick_ms` (a float) were flagged as "discriminating"
purely because **any** six seeds differ from **any** other seeds on a
continuous or per-seed-unique field.

**Set-disjointness only tests discrimination for fields with a small shared
value domain.** For continuous fields it tests nothing. Reported here because
the technique is otherwise worth reusing — with that guard stated. The
substantive finding (build's fields are constants) is unaffected: constants are
exactly the case where the domain is shared and the test is valid.

## ★★★ ALL 12 FAILURES NOW CLASSIFIED — three clean specimens, one likely instrument defect

### ★★ Seed 66 (`tl_ok`) is almost certainly an INSTRUMENT DEFECT, not a product one

```rust
let tool_name = names.first().cloned().unwrap_or_default();
let tl_stone = server.bastion_colonist_tool_factor(&tool_name, WorkType::Mine).unwrap_or(0.0);
let tl_steel = server.bastion_colonist_tool_factor(&tool_name, WorkType::Mine).unwrap_or(0.0);
```

Seed 66 reports **`tool_stone: 0.0` and `tool_steel: 0.0`** (passing seeds: 1.5
and 2.0). **The documented bare-hands floor is 1.0** — the harness's own comment
says so. **0.0 is not a low tool factor; it is BELOW THE FLOOR, i.e. an
impossible reading.** It is the `.unwrap_or(0.0)` sentinel: the probe returned
`None`, twice.

**And `names.first()...unwrap_or_default()` yields an EMPTY STRING on an empty
list**, after which every lookup by that name fails. Seed 66 still shows
`any_mining_xp: true` and `any_woodcutting_xp: true` — **work happened; only the
measurement failed.**

> **"Couldn't measure" was collapsed into a value that reads as "measured, and
> it's terrible" — and the value is outside the metric's own documented range,
> which is what should have caught it.** The standing law with a new costume:
> a sentinel inside the valid-looking numeric range is worse than a missing
> field, because it survives every presence check.

**1 of 12 corpus failures (8.3%) is therefore probably instrument** — consistent
with the historical ~10.4% fixture-false-failure rate.

### ★ Seeds 68 and 92 (`ch_mixed`) — my first hypothesis was WRONG

I guessed `ch_mixed` needed ≥2 trees (both seeds have `ch_trees: 1` vs passing
7). **Read the definition: it scans the FIRST tree's AABB for both `Wood` and
`Leaves`** — trunk plus canopy in one box. Tree *count* is irrelevant.

**AABB DERIVATION NOW READ** (`bastion_place_chop_area` over a 64×64 window,
first ring with ≥1 tree; `ch_mixed` then scans that AABB for Wood AND Leaves).
Normalised **cells-per-tree across all 48 seeds**:

| seed | trees | cells | **cells/tree** | `ch_mixed` |
|---|---|---|---|---|
| **68** | 1 | **30** | **30.0** | **false** |
| 50 | 1 | 1895 | 1895.0 | true |
| 78 | 2 | 4049 | 2024.5 | true |
| 94 / 95 / 96 | 1 / 4 / 1 | 2048 / 8192 / 2048 | **2048.0** (the cap) | true |
| **92** | 1 | **2048** | **2048.0** | **false** |

**Every other seed in the corpus sits in a tight 1891–2048 band. Seed 68 is
30.0 — a 63× outlier and the only seed below 1891.**

> **★ This SPLITS the two `ch_mixed` failures instead of uniting them.** Seed 68
> is a degenerate 30-cell placement (30 cells plausibly cannot contain both a
> trunk and a canopy, which is exactly what the clause scans for). **Seed 92 is
> at the cap, 2048, completely normal — so its `ch_mixed: false` has a DIFFERENT
> cause.**

**My earlier "the box is the common factor" hypothesis is REFUTED for seed 92**
and survives only for 68. Seed 92 is separately anomalous — it is also the seed
whose chop probe ran `probe_incomplete: true` at the ~100k cap on both vantages.
**Two `ch_mixed` reds, two unrelated situations.**

### The three CLEAN SPECIMENS — one per mode, all deterministic, all already captured

| mode | seed | why it is clean |
|---|---|---|
| **build** | **62** | mine cleared, chop cleared, `stone_sum: 27`, `gave_item: true`, **`rescue_fired: false`** — **only build failed**, and it is the sole Mode-A seed with no rescue to confound it |
| **mine** | **90** | 3 cells, all claimed by named colonists at `cycles: 0`, `blocked_by: None`, no progress |
| **chop** | **78** | path exists from spawn (complete, 228 cols), **nothing recorded a block**, `log_sum: 0` |

**No fixture needs building for any of the three.** Each is a single
deterministic seed in the standing fan, verified reproducible across two
independent runs.

## ★★★ INSTRUMENT SENSITIVITY — the corpus HAS moved; my "false green" warning was too broad

I warned that the seam row would produce a false green because nothing reports
access-plan state. **The blindness is real but I over-generalised from it.**
Measured across every wave on disk:

| wave | failures | note |
|---|---|---|
| wave7 | 7/36 | |
| wave8 | 11/36 | |
| wave14 / 16 / 17 | **14/48** | seeds 51, 55, 69 failing |
| wave15 | **16/48** | 74, 76 added |
| wave18 / 19 | **12/48** | 51, 55, 69 gone; **90 appeared** |
| **wave20** | **11/48** | **`ch_leaf_cleared` ABSENT — seed 52 flipped to PASS** (the #64 guard debut, the recorded +1/0) |
| wave22 / 23 / 24 | 12/48 | byte-identical to wave19 |

**The corpus moves, seeds enter and leave, and the exact-match bar has fired on
a one-line change.** It is a demonstrably sensitive instrument for the outcomes
it reports. **Waves 19/22/23/24 being byte-identical across four commits
(`ed532c600e` → `34db70bac2` → `b89cbc799d` → `d3235e5329`) is therefore real
evidence of neutrality, not merely absence of resolution** — and that is a
stronger verification of today's merges than was recorded, since the whole
clause *composition* per seed is identical, not just pass/fail.

### ★★ So the seam row is PREDICTABLE, not unverifiable — name the seed

The corpus cannot see access-plan internals, but it **can** see whether mining
outcomes change. If the colony-global `take(0)` bar starves self-rescue, the
signature is: **rescue fires, access plans never get emitted, interior cells
never become claimable.** Exactly one seed has that shape strongly:

| seed | mined | `rescue_fired` | interior cells never claimed |
|---|---|---|---|
| **71** | **5/27** | **true** | **15 of 27** |
| 54 | 16/27 | true | 7 of 18 |
| 61 | 26/27 | true | 0 of 3 |
| 90 | 25/27 | true | 0 of 3 |

> **PRE-STATED PREDICTION for the seam row: removing the self-rescue bar should
> raise seed 71's `b5_mine_blocks_mined` above 5/27, and to a lesser extent
> seed 54's above 16/27. Seeds 61 and 90 should NOT move — their cells are
> claimed, so plan starvation is not their mechanism.**

**That is a directional bar with a named subject and a named null**, registered
before the change exists. If 71 doesn't move, plan starvation is not its cause
and the bar's removal is a correctness fix with no measured benefit — which is
still worth knowing, and is exactly the kind of result the pre-registration
protects.

**Superseded:** my earlier "a green fan on that row would be a FALSE GREEN."
Correct version — **a green would be uninformative only about the mechanism's
internals; seed 71's magnitude is a genuine outcome-level test.**

### ★★ SPECIMEN ATTESTATION at the MERGED tip `d3235e5329` (wave24)

The specimens were found in wave19 (`ed532c600e`). **Rows would be built against
the merged tip**, so every defining property was re-checked in wave24's raw logs
— per-seed records recovered via the `@@@SEED n` markers.

| seed | property | wave19 | wave24 | |
|---|---|---|---|---|
| **62** | `build_placed` | false | false | OK |
| | `rescue_fired` | false | false | OK |
| | `mine_cleared` | true | true | OK |
| | `chop_cleared` | true | true | OK |
| **90** | `mine_blocks_mined` | 25 | 25 | OK |
| | `mine_cleared` | false | false | OK |
| **78** | `log_sum` | 0 | 0 | OK |
| | `chop_cleared` | false | false | OK |
| | `ch_base_blocked_by` | None | None | OK |

**9 of 9 identical.** All three specimens are valid at `d3235e5329`, not only at
the wave they were discovered in. **A specimen inherits the provenance rule like
any other status: it is attested at a tip, or it is a claim.**

## ★★★ SEAM HYPOTHESIS — QUANTIFIED AND NOT SUPPORTED. My prediction is refuted.

### First, a correction: the corpus DOES have access-plan visibility

I wrote that the corpus has **zero** access-plan visibility. **Wrong.** I searched
**top-level field names** for "access" and missed
`b5_cascade_probe.access_emissions_max` and `.members_seen` — **nested inside a
field whose name is about cascades.** *Read the schema at every level; a
top-level name scan is not a schema scan.* Same error class as the rest of the
day, one level down.

`access_emissions_max` varies **0–3** across the 48 seeds. It is exactly the
access-plan emission counter I said did not exist.

### The starvation hypothesis, measured

If the colony-global `take(0)` bar starves self-rescue, failing seeds should
cluster on **rescue fired + zero access plans emitted**. They do not:

| group | seeds | fail | pass | failure rate |
|---|---|---|---|---|
| zero-emission | 14 | 4 | 10 | **29%** |
| whole corpus | 48 | 12 | 36 | **25%** |
| rescue fired **and** zero emissions | 11 | 3 | 8 | **27%** |

**Zero-emission seeds fail at the same rate as everything else.** Eight seeds
fire a rescue, emit no access plan, and **pass** — so emitting no plan is
normal, not pathological.

### ★★ And my named subject is excluded outright

> **PRE-STATED (registered an hour earlier): removing the bar should raise seed
> 71's `mine_blocks_mined` above 5/27.**
>
> **REFUTED. Seed 71 has `access_emissions_max: 3` — it emitted THREE access
> plans. It was never starved.** Whatever stalls seed 71 (15 of 27 cells never
> claimed), plan starvation is not it.

Per-failure emissions: 52→2, **54→0**, **61→0**, **62→0**, 66→2, 68→1,
**71→3**, **78→0**, 80→3, 85→1, 90→2, 92→1.

**The bar remains a real code defect** — fixed at the descent caller, live at
self-rescue, with the codebase calling it a starvation bug at the sibling site.
**But the corpus provides no evidence it is currently causing harm**, so the
row's justification is *"remove an inconsistency"*, not *"fix a starvation"* —
a materially weaker case for spending a fan.

**Value of having pre-registered:** the prediction was falsified **by data
already on disk, before any change was built or any VM time spent.** That is
the protocol working exactly as intended.

### ★ Two more instrument facts from the same read

**The b55 family in the b5 corpus is INERT** — `55_blocked_by` (null),
`55_names_blocker` (false), `55_clears_on_cancel` (true), `55_notified_once`
(false) are **constant across all 48 seeds**, and `55_diag` is constant across
47. **Do not read them as evidence about b55.**

**Distinguish GUARDS from DIAGNOSTICS, though.** `flat_hint_decoupled` and
`slope_cancel_clean` are also constant-true across 48 — but those are
*assertions that always hold*, i.e. regression guards doing their job. **A
constant guard is healthy; a constant diagnostic is inert.** My earlier line
("a field that reports the same value in passing and failing runs is not a
diagnostic") needs that carve-out.

### ★★ The parked cascade row (#34) — now confirmed with a number

`abort_ceiling_max` and `abort_resets_max` are non-zero on **exactly one seed of
48 (seed 59), and seed 59 PASSES.** So the cascade abort mechanism has **one**
corpus instance and it is the wrong polarity to be a failing pair member.

> **A matched pair is impossible in principle here — n=1, and that 1 passes.**

That is a far stronger basis for the parking than *"no sound pair exists in this
corpus"*. **The row stays parked, and now the record says why in one line.**

### ★★ DETERMINISM, STRENGTHENED: a 12-valued counter reproduces exactly

Our determinism evidence has been pass/fail identity and clause composition —
both **low-entropy**, so identity across runs is weak evidence. The strongest
test available is the corpus's **highest-variance field**, `b5_drift_events`
(range 4–17, **12 distinct values**, zero seeds at 0):

```
wave19  (ed532c600e)          4:2  5:4  6:7  7:7  8:9  9:2  10:7  11:5  12:1  13:2  15:1  17:1
wave21  (1bf3ab2e, docs-only) 4:2  5:4  6:7  7:7  8:9  9:2  10:7  11:5  12:1  13:2  15:1  17:1
wave24  (d3235e5329, MERGED)  4:2  5:4  6:7  7:7  8:9  9:2  10:7  11:5  12:1  13:2  15:1  17:1
```

**Identical, to the count, in all three.** Two independent fans on the same code
**and** a fan on the merged tip. **A 12-valued distribution matching exactly is
far stronger than 48 booleans matching**, and it extends merge-neutrality to a
counter nobody was watching.

**★ And `drift_events` documents its own limitation, accurately.** The harness
comment says *"see the field doc comments below for what each one means and
(for drift) its known non-discriminating limitation."* **Checked: failing seeds
span 5–17 against a corpus range of 4–17 — no discriminating power, exactly as
documented.**

**That is the third claim checked today that HELD** (with task #61's *"only
genuinely-unreachable chop case"* and `vm-pool.sh`'s *"+dirty = LFS noise, code
clean via reset --hard"*). **Recording survivals matters: a checking habit that
only ever reports bad news stops being believed, and the comments in this tree
are mostly right.**

## ★★★★ TASK #59's STARVATION HYPOTHESIS IS SUPPORTED — 6/6, by its own purpose-built instrument, unread until now

**Found by applying my own new rule properly.** A recursive schema dump of
`wave19_FULL.json` gives **161 leaf paths, only 68 of them top-level — 93 were
invisible to a top-level name scan, and 69 of those VARY across seeds.** Among
them: `b5_mine_cell_diag[].starvation_cycles` and `.starvation_crowded_cycles`.

### The counters carry their own decision rule, written by whoever built them

> `starvation_cycles` = cycles this cell was open+unclaimed;
> `starvation_crowded_cycles` = of those, how many had at least one OTHER
> unclaimed job competing. **A ratio near 1.0 supports the hypothesis; a cell
> unattempted for many cycles with an EMPTY field (crowded far below
> starvation) is Fable's kill case.** *Report-only, never gates `pass`.*

Task #59's hypothesis: **greedy arbitration with no cooldown/penalty after a
failed attempt — a hard cell just loses the score comparison every cycle while
easier unclaimed work exists.**

### The result, worst-starved cell per seed

| seed | mined | starv | crowded | **ratio** | verdict |
|---|---|---|---|---|---|
| 52 | 27/27 | 294 | 294 | **1.000** | FAIL (chop) |
| 54 | 16/27 | 360 | 360 | **1.000** | FAIL |
| 61 | 26/27 | 306 | 287 | **0.938** | FAIL |
| 66 | 27/27 | 222 | 222 | **1.000** | FAIL (tool) |
| 71 | **5/27** | **360** | **360** | **1.000** | FAIL |
| 90 | 25/27 | 332 | 332 | **1.000** | FAIL |

> **Five of six ratios are EXACTLY 1.000; the sixth is 0.938. The documented
> KILL CASE occurs in ZERO seeds.** Every starved cell had competing unclaimed
> work essentially 100% of the cycles it sat unclaimed. **360 is the whole run.**

### ★★ What this explains

**Seed 71 is ARBITRATION-starved, not ACCESS-starved.** It emitted three access
plans (so the seam bar never touched it — my refuted prediction was right for
this reason), yet 15 of 27 cells sat open and unclaimed for the entire run with
competitors present every cycle. **The mine failures have a mechanism, and it is
not the one we spent the day on.**

**And it is a different question from `cycles_since_last_claim`**, which I used
for the four-signature split. That field says *how long since a claim*; these
say *whether anything else was competing while it waited*. **The signatures
stand; this adds the WHY for the never-claimed ones.**

### ★ Caveat that must travel with it

**Seeds 52 and 66 show ratio 1.000 with 294 and 222 starved cycles and still
mined 27/27.** So a 1.0 ratio is **not sufficient for failure** — contention is
normal and cells usually get claimed eventually. **The claim supported is the
MECHANISM (starvation is by contention, not by an empty field), not that
starvation alone determines the outcome.** A row here needs to explain why 71
never recovered and 66 did.

**NOT read: the arbitration scoring code itself.** This is the instrument's own
verdict on its own hypothesis, applied per its own documented rule.

### ★★★ WHY 71 NEVER RECOVERED AND 66 DID — and a refinement of the #59 claim

Same `mine_cell_diag`, per-cell aggregates:

| seed | mined | shortfall | cells in diag | **`unreachable` cells** | offered | timeouts | claimed now |
|---|---|---|---|---|---|---|---|
| 66 | **27/27** | 0 | 27 | **0** | 27 | 9 | 2 |
| 52 | **27/27** | 0 | 21 | **0** | 25 | 16 | 0 |
| **90** | 25/27 | 2 | 3 | **0** | 8 | 6 | **3** |
| 61 | 26/27 | 1 | 3 | 3 | 5 | 3 | 0 |
| 54 | 16/27 | 11 | 18 | 7 | 19 | 14 | 1 |
| **71** | **5/27** | **22** | 27 | **10** | 22 | 17 | 0 |

> **Every seed that fully mined has ZERO unreachable cells. The shortfall tracks
> the unreachable count: 10 → 22 unmined, 7 → 11, 3 → 1.** Seed 66 sat at
> starvation ratio 1.000 with 222 starved cycles and still finished — **because
> nothing was flagged unreachable.**

### ★★ This REFINES the #59 result — read both together

**Starvation-by-contention (ratio ≈ 1.0) is present in successes too** — seeds 52
and 66 have it and mine 27/27. **So contention is the normal state, and it is
`unreachable` that separates the failures.** My #59 write-up said the mechanism
was supported but that the caveat needed explaining; **this is the explanation,
and it demotes starvation from cause to background condition** for these seeds.

**#59's finding still stands as stated** — when a cell was starved, it was
starved *by competition*, never by an empty field (the kill case is absent
6/6). **What changes is its weight**: starvation alone predicts nothing, and any
row built on it must carry `unreachable` as the primary term.

**NOT ESTABLISHED — direction.** `unreachable` may cause the shortfall, or may
be a flag set *because* the cell was never successfully worked. Given today's
probe caveat (the reachability instrument is sound only for negatives), **this
flag's own provenance is the next thing to read, not to assume.**

### ★ And it promotes seed 90 to the unambiguous specimen

**Seed 90 is the ONLY shortfall seed with zero unreachable cells** — 3 cells,
all actively claimed by named colonists at `cycles: 0`, `blocked_by: None`,
`progress: 0.878` on the best cell, and still 2 blocks unmined. **No
unreachability confound, no blocked region, no starvation-by-absence.** Every
other failing seed carries an `unreachable` term that has to be stripped first.

**If one mine row gets built, seed 90 is its subject.**

### ★★★ WITHDRAWN: `unreachable` is NOT the discriminator — it is downstream of dig progress

I read the flag's provenance, as the previous section said must happen before
trusting it. **It does not mean what its name says, and my discriminator claim
is withdrawn.**

```rust
// per pass, for each UNCLAIMED Mine job:
let is_exposed = [ +x, -x, +y, -y, +z, -z ]
    .into_iter()
    .any(|d| terrain.get(job.pos + d).map(|b| !b.is_filled()).unwrap_or(true));
if !is_exposed {
    // "Fully enclosed: flag unreachable-for-now ... the periodic retry sweep
    //  re-tests as the dig opens the shell"
    job.unreachable = true;
    continue;
}
```

> **`unreachable` = "all six face-neighbours are currently solid."** It is a
> **geometric enclosure test on present terrain**, recomputed each pass (plus a
> blanket amnesty clear). It is NOT a pathfinding verdict, and the comment says
> so outright: *unreachable-for-now*, re-tested *as the dig opens the shell*.

**In a 27-cell mine volume the interior cells are enclosed BY CONSTRUCTION until
a face opens.** So:

- seed 71 mined 5/27 → most of the volume still solid → **10 cells still
  enclosed** → flagged.
- seed 66 mined 27/27 → nothing left solid → **0 flagged**.

**The "discriminator" is close to a restatement of the shortfall.** That is the
`build_placed` / `any_needs_materials` trap — *two fields, one fact* — which I
named this morning and then walked into this afternoon, on a field whose name
had already been flagged as needing its content read.

**Not perfectly tautological, and the residue is interesting:** seed 61 shows
**3 flagged cells against 1 unmined**, which enclosure alone does not explain
(amnesty timing and the per-pass recompute are the candidates). **Not chased.**

### What survives

**The #59 starvation result is untouched.** It rests on
`starvation_cycles`/`starvation_crowded_cycles`, measured over cycles a cell sat
*open and unclaimed* — a different quantity, independently recorded, with its
own documented decision rule. **Ratio 1.000 on five of six and zero kill cases
still stands.**

**What is withdrawn is only my claim to have answered "why 71 and not 66."
That question is OPEN**, and it remains the ARB-STARVATION row's gating
deliverable (DECISIONS #53).

**And seed 90 is unaffected as the specimen** — its 3 cells are *claimed*, so
the enclosure branch (`if job.claimed_by.is_some() { continue; }`) never even
runs for them. **Zero `unreachable` there is a real zero, not an artifact.**

### ★★★ #53's GATING DELIVERABLE CANNOT BE ANSWERED FROM DISK — and here is what would answer it

Second attempt at *"why did 71 never recover while 66 did"*, this time reading
every field's definition before using it. **`times_offered` is not offers** —
the harness emits `"times_offered": claims_here` and the hook returns
`board.claims_by_pos`, i.e. **claims GRANTED**. (Caught before building on it,
unlike `unreachable`.)

**Timeouts per claim** — normalised, because raw counts are coupled to the
shortfall the way `unreachable` was:

| seed | mined | claims | timeouts | **per claim** |
|---|---|---|---|---|
| **66** | **27/27** | 27 | 9 | **0.33** |
| 52 | **27/27** | 25 | 16 | 0.64 |
| 61 | 26/27 | 5 | 3 | 0.60 |
| 90 | 25/27 | 8 | 6 | 0.75 |
| 54 | 16/27 | 19 | 14 | 0.74 |
| 71 | **5/27** | 22 | 17 | **0.77** |

**Suggestive and insufficient.** Seed 66 sits at half everyone else's rate — but
**seed 52 fully mined at 0.64, overlapping seed 61's 0.60 at one block short.**
The statistic does not separate success from failure, and n=6.

### ★★ The structural reason — and it names the instrument

**Every per-cell field available is COUPLED TO THE OUTCOME.** A cell that never
completes gets re-claimed and re-timed-out repeatedly, inflating claims and
timeouts together; `unreachable` tracks enclosure, which tracks dig progress;
`starvation_cycles` counts cycles spent unclaimed, which grows when work isn't
finishing. **There is no field that records what happened to an individual
ATTEMPT.**

> **`mine_cell_diag` aggregates per CELL. The question is per ATTEMPT.** Cell
> totals collapse exactly the structure that would answer it —
> [[aggregate-late-keep-the-structure]], now naming a specific missing
> instrument rather than a general principle.

**What would answer it:** a per-attempt record — *claim granted → outcome*
(completed / timed out / preempted / released / material-blocked), with the
cycle it happened on. Then *"71's attempts fail for reason X while 66's succeed"*
becomes a direct read instead of an inference from coupled aggregates.

**That is a third instrument row**, alongside `build_cell_diag` and
`blocked_sources`-for-mine — and it is the one #53's gating deliverable actually
depends on. **The honest status is: the discriminator is not derivable from the
current corpus, and more reading will not produce it.**

**Also worth noting for the row:** the hook's own doc warns *"All zero if the
position was never open/unclaimed during arbitration (e.g. always claimed
instantly)"* — so **`starvation_cycles: 0` is ambiguous between "never starved"
and "never open"**, the same two-readings-one-value shape as everything else
today. Any starvation statistic must exclude zeros deliberately, not silently.

## ★★★★ CROSS-SCENARIO PATTERN: colony work stalls ONE UNIT SHORT — Ben's ">=3 places" directive fires

Independent scenarios, independent subsystems, same shape:

| scenario / seed | completed | target | short by |
|---|---|---|---|
| **farm** | **8** tilled | 9 | **1** |
| **b5 seed 61** | **26** mined | 27 | **1** |
| **b5 seed 90** | **25** mined | 27 | **2** |

**Three places. Ben's standing directive applies: *symptom in ≥3 places ⇒ STOP
BISECTING, read the whole pipeline.*** Filed here so the next lane meets it
before opening three separate rows.

**NOT CLAIMED: one mechanism.** Farm is a 3×3 till plot; the mine seeds are a
27-cell volume with different claim states (61 blocked-region, 90
claimed-and-stalled). **The SHAPE matches; the cause is unestablished** — and
today's whole lesson is that a matching shape recruits any nearby mechanism
([[matched-control-must-match-on-system-and-axis]]: *the more elegant the
mechanism, the more readily it recruits any nearby red*).

**What makes it worth flagging anyway:** each of these was separately described
as its own mystery — farm's *"unexplained under both stances"*, the mine seeds'
*"one block short"* which I nearly wrote off as scheduling tail and then proved
deterministic. **They only look like a family once the descriptions are replaced
by the numbers**, which is what this session's re-run pass did.

**Cheapest discriminator before anyone opens a row:** is the missing unit
always in a **structurally distinguished position** (a plot corner, a volume
edge, a column frontier)? Farm's `growth_rose` probe reads `plot.min` — **if the
untilled cell IS that corner, position is implicated immediately** and that is a
one-line check on an existing log.

## ★★ MINE-SIDE POSITION CHECK — CANNOT BE ANSWERED AS ASKED, and my first attempt was artifactual

The architect's broad read asked whether the mine seeds' missing cells are
structurally distinguished (edge / corner / frontier), as farm's turned out to
be. **Two problems, both mine, caught before the answer was reported.**

### Problem 1 — my edge/corner computation was an ARTIFACT

I derived the volume extent from **the diag's own cells**. For seeds 61 and 90
the diag holds **3 cells in a single (x,y) column**, so `xs` and `ys` are
single-valued and `p.x in (xs[0], xs[-1])` is **trivially true for every cell**.
**Every cell scored CORNER=True by construction.** Meaningless. (Seed 71's diag
does span the full 3×3×3, so only its geometry was real.)

### Problem 2 — `mine_cell_diag` is NOT a list of unmined cells

**Criterion READ** (harness, the `mine_cell_diag` build loop): a cell is
included **iff a live JOB still exists at that position** —

```rust
for x .. for y .. for z {
    if let Some(BastionInspectKind::Job(j)) = /* inspect at pos */ {
        ... mine_cell_diag.push(...)
```

**So the diag lists cells that still carry an outstanding job — not cells that
are still solid.** Every position inference I attempted rests on a set I had not
read the membership rule for. **Fourth name/criterion assumption of the session;
first one caught before it left the session.**

### ★ The residue this exposes — FLAGGED, NOT CLAIMED

Live jobs exceed unmined cells in every failing seed:

| seed | mined | unmined | cells with live jobs | surplus |
|---|---|---|---|---|
| 61 | 26/27 | 1 | 3 | **+2** |
| 90 | 25/27 | 2 | 3 | **+1** |
| 54 | 16/27 | 11 | 18 | **+7** |
| 71 | 5/27 | 22 | 27 | **+5** |

And `cells_above_open` (counted over `open_cells`) implies some job-bearing cells
are **already open** — e.g. seed 61's z=162 reports 2 open above it, and both of
those cells appear in the diag with live jobs of their own.

> **Reading: jobs may be persisting on already-mined cells.** That would be a
> real defect and it would also explain seed 61's earlier "3 flagged vs 1
> unmined" residue.

### ★★★ READ — and it KILLS the hypothesis. `open_cells` does not mean "open".

```rust
// First pass: which (x,y,z) cells are still open JOBS at all ...
if let Some(BastionInspectKind::Job(_)) = server.bastion_inspect_cell(pos) {
    open_cells.insert((x, y, z));
}
```

**`open_cells` is populated by the SAME predicate as `mine_cell_diag` — "a job
exists here."** The comment says it outright: *"which cells are still open JOBS
at all."*

> **So `cells_above_open` counts cells above that STILL HAVE A JOB — not cells
> that are mined. My reading was exactly inverted:** seed 61's z=162 reporting
> "2 open above" means 163 and 164 **still have jobs**, i.e. are still
> outstanding — **not** that they were mined.

**The "jobs persisting on already-mined cells" hypothesis is NOT SUPPORTED and
is withdrawn.** The surplus between live-job count and `mine_blocks_mined`
remains unexplained, but it needs `mine_blocks_mined`'s own definition — a
block-state count vs a job-existence count are simply different measures, and I
have not read the first.

**Fifth name-vs-content instance of the session, and it was in a field I had
built a reading on twenty minutes earlier.** `cells_above_open` says *open* and
means *has an outstanding job*. **Flagged-not-claimed is what made this cheap:
the hypothesis died in a five-minute read instead of in someone's fix design.**

**Consequence for the four-signature table: it stands.** Those signatures rest
on `blocked_by`, claim state and `cycles_since_last_claim` — per-cell facts that
do not depend on the diag being an unmined-cell list. **Only the position
question is blocked.**

## ★★★ UNSATISFIABLE-WATCH SWEEP (2026-08-04) — 5 baseline-relative watches; only ONE emits its baseline

Architect-assigned after `farm_growth_rose` proved unfireable. **Scan coverage
stated explicitly, because a clean negative from a narrow scan is not absence:**

```
loops scanned (<=150 lines)             667
  ... that set a bool AND break          55
  ... comparing against a pre-loop base   5   <-- the watch class
```

### The five, and the finding

| line | scenario | baseline | sets | **baseline EMITTED?** | unsatisfiable when |
|---|---|---|---|---|---|
| 4689 | `b55_scenario` | `remainder_before` | `remainder_progressed` | **NO** | baseline already 0 (`total < base`, count floors at 0) |
| 10043 | `b73_scenario` | `attempts0` | `broke` | **NO** | no further attempt occurs |
| 10059 | `b73_scenario` | `jobs_frozen_at` | `resumed_after_break` | **NO** | baseline already 0 (`jobs < base`, floors at 0) |
| **11027** | `farm_scenario` | `g1` | `rose` | **YES** (telemetry `println`) | **baseline at/past target — CONFIRMED unfireable this run** |
| 23037 | `chokepoint_scenario` | `done_before` | `ml_done` | **NO** | everything already done at capture |

> **★★★ FOUR OF FIVE BASELINES ARE NEVER EMITTED — and the ONE that is emitted
> is the only watch we were able to diagnose.** The correlation is causal: farm
> was solved *because* `g1=15` was printed; b55 is uninterpretable *because*
> `remainder_before` is not. **Observability of the baseline is the difference
> between a diagnosed red and an uninterpretable one.**

**Two share b55's exact count-floors-at-zero shape** (4689, 10059): `X < baseline`
is impossible when the baseline is already 0, and neither emits the baseline that
would reveal it.

### Filed to the report-fix backlog — one line each

**Emit the baseline beside every watch flag**, and apply the three-way treatment:
assert the watched quantity is BELOW target at window-open; when it isn't, emit
**`already-complete-at-open`** as its own state rather than `false`. Same
discipline as `probe_incomplete` and the `starvation_cycles` zeros; the
precedent is `auton`'s `storm_baseline_captured`, which cost one line.

**NOT CLAIMED: that the other four are firing unsatisfiably today.** Only farm is
confirmed (its baseline is visible). **The other four are UNVERIFIABLE either
way, which is the finding** — and `b73` is a tracked EXPECTED-RED whose
fingerprint rests on two of them.

### Scan limits — what this sweep would MISS
Comparisons against expressions rather than bare identifiers; baselines captured
more than 30 lines above the loop; loops over 150 lines; watches that set state
other than a plain `flag = true`; and equality-held watches (`x == baseline`),
which have a different unsatisfiability mode. **A second pass wanting those
should widen deliberately rather than trust this count.**

## ★★★★ THE CAPSULE ASYMMETRY DOES NOT TRANSFER — multi-layer collapse breaks BOTH directions

Architect flagged this as UNREAD and it needed the read, because **it reverses
what I asserted this morning about seed 80.**

**`column_height_near` READ** (`bastion_jobs.rs:1901`):

```rust
fn column_height_near(terrain, x, y, z_hint) -> Option<i32> {
    (z_hint - 60..=z_hint + 60)
        .rev()                                     // scans DOWNWARD from z_hint+60
        .find(|&z| terrain.get(..).is_filled())    // returns the FIRST filled block
}
```

> **It returns the HIGHEST solid block within ±60 — the top of an overhang, not
> the floor beneath it.** For 5b's seed-52 column (rock 151–155, air 146–150,
> real ground 145) it returns **155**, and the traversable band at 146–150 is
> invisible.

### The two error models push in OPPOSITE directions

| model | error | negatives | positives |
|---|---|---|---|
| **body width** (point vs capsule) | too PERMISSIVE | **SOUND** | unsound |
| **column collapse** (multi-layer) | **BOTH** — reports connectivity across a surface that isn't continuous **AND** misses passages underneath | **UNSOUND** | unsound |

**So in multi-layer terrain NEITHER direction of the probe is sound.** The
capsule asymmetry I derived this morning — *"negatives sound, positives not"* —
**holds only in single-layer terrain**, which is exactly the scope I failed to
name when I stated it.

### ★★ Consequence for seed 80 — I had this backwards for this model

This morning I told two lanes that the point-model **strengthens** seed 80's
no-route finding. **True for body width. FALSE for column collapse:** a
subsurface route (tunnel, gallery, cave passage) is invisible to the probe, so
*"no route from either vantage"* means **"no route over the top surfaces"** —
and **mine sites are precisely where dug galleries exist.**

**Seed 80's negative is therefore CONDITIONAL on its site being single-layer.**

### The test — cheap, decisive, needs 5b's block-query

**Column-scan seed 80's site** (`[24484, 26192, 153]`, standable target
`[24485, 26192, 153]`) the same way 5b scanned 52 and 54. **Single-surface
terrain ⇒ the negative stands** (only the capsule caveat applies, and negatives
are sound there). **Cave/overhang terrain ⇒ the negative is unsound** and #61's
parking rationale, which cites that soundness, needs the caveat even if the
parking survives on other grounds.

**Goes on the single-seed diagnostics batch** — it is the same instrument the
batch already needs, one more site.

> **★ The generalisable error: I derived an asymmetry under one error model and
> stated it without naming the model.** When a second error model appeared, the
> asymmetry was assumed to carry. **An instrument's soundness direction is a
> property of the ERROR MODEL, not of the instrument** — and a caveat that
> doesn't name its model is the sufficiency-claim family again, one level up.

## ★ SINGLE-SEED DIAGNOSTICS BATCH — 6 items, no fan, behind 5b's binary

| # | seed | run | question |
|---|---|---|---|
| 1 | **71** | per-attempt claim trace | **shape A vs shape B** — attempts exist and fail, or zero attempts recorded |
| 2 | **66** | per-attempt claim trace | the contrast case: why does contention resolve here |
| 3 | **61** | extended settle window | deterministically SLOW or deterministically STALLED |
| 4 | **90** | extended settle window | same question, claimed-and-stuck variant |
| 5 | **92** | raised-cap probe | UNKNOWN → known; also carries `ch_mixed:false` at a normal 2048-cell box |
| 6 | **80** | **column scan at `[24484, 26192, 153]`** | **single-layer ⇒ its no-route negative stands; multi-layer ⇒ the "genuinely unreachable" citation needs a caveat everywhere** |

**Outcome branches are pre-stated for every item.** #61's parking survives item 6
either way — it rests on `n=1, wrong polarity`, which is structural impossibility
rather than an instrument reading. **That is the argument for preferring
structural grounds when both are available.**

## ★★★ TWO SPECIES-DECIDING READS (architect-assigned) — `run` and `auton3`

### `run` — the bar is NOT structurally unclearable. This is a REAL movement gap.

**Constants READ** (`bastion_jobs.rs:1604`, `:1609`):

```rust
const TRAVEL_SPEED: f32 = 0.8;   // walk
pub const RUN_SPEED: f32 = 1.0;  // "the full vanilla speed_factor"
```

| quantity | value |
|---|---|
| **design intent** | 1.0 / 0.8 = **1.25 → 25% faster** |
| **clause bar** | `> walk * 1.15` → **15%** |
| **measured** | 0.300 / 0.263 = **1.1407 → 14.07%** |

> **The bar sits 10 points BELOW design intent, so it is not testing instrument
> overhead — the architect's structural-unclearability hypothesis is REFUTED.**
> **We are delivering ~14% where the design says 25% — roughly half the intended
> advantage.**

**This REVERSES my own afternoon reclassification.** I called it "a near-miss
against a possibly-uncalibrated threshold." **The threshold is generous; the
measured delta is genuinely short.** Per the architect's own decision rule
(*"design 1.5 measured 1.14 ⇒ a real movement question"*), design 1.25 vs
measured 1.14 lands on the same side.

**★★ N-RUN STABILITY CHECK DONE — 4 runs, PERFECTLY IDENTICAL:**

```
run1  walk=0.263 run=0.300 e_full=107.0 e_mid=81.8 e_floor=11.3 e_after=107.0
run2  walk=0.263 run=0.300 e_full=107.0 e_mid=81.8 e_floor=11.3 e_after=107.0
run3  walk=0.263 run=0.300 e_full=107.0 e_mid=81.8 e_floor=11.3 e_after=107.0
run4  walk=0.263 run=0.300 e_full=107.0 e_mid=81.8 e_floor=11.3 e_after=107.0
```

**14.07% is DETERMINISTIC, not measurement noise.** So the shortfall is a
reproducible property, and "it's just a noisy sample" is excluded.

**But determinism does NOT separate the two remaining candidates**, because both
are deterministic:
- **(a)** a real speed-factor shortfall in the movement layer, or
- **(b)** a deterministic measurement overhead — acceleration ramp and path
  curvature inside a 45-tick displacement window, identical every run.

> **★ The decisive test is cheap and already needed elsewhere: LENGTHEN THE
> WINDOW.** If it is (b), a longer window dilutes the ramp and the ratio climbs
> toward 1.25. If it is (a), the ratio stays at ~1.14 regardless. **Same
> extended-window instrument the batch already needs for seeds 61/90 — add
> `run` as a seventh item, parameterised, never by editing the default.**

### `auton3` — SAME SITE, therefore a model/computation gap, not plumbing

**Both write sites READ** (`bastion_jobs.rs:8641`, `:8695`):

```rust
// 8641 — flee-preempt path
arb.last_scores = modulated_urgencies((0.0, URGENCY_FLEE, URGENCY_IDLE), ..);
//                                     ^^^ FIRST COMPONENT HARDCODED

// 8695 — ordinary arbitration
... modulated_urgencies((<work>, if flee_sig { URGENCY_FLEE } else { 0.0 }, URGENCY_IDLE), ..);
arb.last_scores = (w, f, i);
```

**The observed second component is `0.0`, which matches the no-flee branch of
8695** — so the colonist took the **ordinary** path, where **all three components
are written at the same site.** Per the architect's rule: **model/computation
gap, not recording plumbing.** Row shrinks accordingly.

**Correcting my own earlier phrasing:** I said components 2 and 3 matching
"proves the modulation works." **Component 2 is `0.0` in both — a constant here,
carrying no information.** Only **component 3 discriminates** (0.08 vs 0.12,
matching prediction), and that alone is what shows modulation working.

**NOT ESTABLISHED:** whether the engine's `0.0` work-urgency is *correct for the
situation* (no work available ⇒ legitimately zero, and the harness's `predict`
assumes work exists) or a genuine computation gap. **Fixture-vs-game triage
applies before this ranks.**

### ★ A latent misread found en route — file it

**Site 8641 hardcodes the first component to `0.0`** with a comment explaining
why (*"Flee fired on the signal, not on the score"* — deliberate). **So whenever
a colonist takes the flee-preempt path, `last_scores.0` is 0.0 regardless of
actual work urgency.** Any consumer reading `last_scores` as "the work urgency"
is wrong for those ticks. **The comment names its case correctly at the write
site; nothing names it at the READ site** — and UI-4 is a named future consumer.

## ★★★★ `auton3` TRIAGE CLOSED — FIXTURE DEFECT, and it reverses my own "model gap" call

The last unclassified red's last question, answered by two reads.

### The harness hardcodes the work-urgency input

```rust
// harness `predict`
modulated_urgencies((0.5, 0.0, 0.1), &vals, adv, wor, soc, intr)
//                    ^^^ work urgency HARDCODED — never read from the engine
```

### The engine GATES it on a signal

```rust
// bastion_jobs.rs ~8672, ordinary arbitration
let work_sig = active_jobs.contains(entity) || work_available;
let (w, f, i) = modulated_urgencies(
    ( if work_sig { URGENCY_WORK } else { 0.0 },   // <-- the gate
      if flee_sig { URGENCY_FLEE } else { 0.0 },
      URGENCY_IDLE ), ..);
```

`URGENCY_WORK = 0.5` — **so the harness's constant is correct.** What the harness
omits is **the gate**. The engine's own comment states the intent: *"Zero-
preservation in the pure fn keeps signal-gated zeros at zero (no invented
flee/work)."*

> **Observed first component `0.0` ⇒ `work_sig` was FALSE at the tick the scores
> were last written** — the colonist had no active job and no work available.
> **The engine recorded 0.0 CORRECTLY. The harness asserts 0.5 unconditionally.**

**VERDICT: FIXTURE DEFECT** (the 10.4% family), **not a model gap.** Fix shape:
`predict` must read `work_sig` — or the scenario must guarantee it true at the
sample tick — rather than assuming it.

### ★★ This REVERSES my own conclusion, and the reason is worth keeping

An hour ago I applied the architect's rule — *same write site ⇒ model/computation
gap; different site ⇒ recording plumbing* — concluded **model gap**, and reported
it. **The rule's dichotomy is incomplete: there is a third option neither of us
listed.**

| | |
|---|---|
| different site | recording **plumbing** |
| same site, computation wrong | **model gap** |
| **same site, computation RIGHT, PREDICTION wrong** | **FIXTURE — the missing case** |

> **The site-location test distinguishes plumbing from computation. It cannot
> distinguish "the computation is wrong" from "the expectation is wrong" —
> for that you must read the INPUT GATE on both sides.**

**Third reversal of a conclusion I had already reported, and the same cause each
time: a rule applied without checking whether its cases are exhaustive.** The
countermeasure that worked here is the one that keeps working — **read both
sides of the comparison, not just the surprising one.**

## ★ RESTORED WORKING — the farm entry's detail, recovered from `d4c020bd8e`

**I destroyed this converting the red list to table form** — the till count, the
corner analysis and the ANSI trap note all lived inside the farm row I
compressed. **Recovered from git and re-filed here as working, below the
table.** The compression lesson applies to my own record-keeping:
**a table is an aggregate, and aggregating late means keeping the structure
SOMEWHERE, not nowhere.**

★★★ **NO LONGER UNEXPLAINED. Three red clauses collapse to ONE
root: 8 of 9 cells tilled.** Re-run at `460626a6e2`:
`FARM TELEMETRY: tilled=8 wheat=2 seeds=15 g1=15`, and
```rust
if tilled_count(&server) == 9 { tilled = true; }   // requires ALL NINE
if grown_cells(&server, 1) >= 9 { sown = true; }   // requires NINE grown
```
**The farm WORKS end-to-end** — `matured`, `harvested`, `cycled`,
`seed_positive` all true, 2 wheat, 15 seeds. **One cell of nine never gets
tilled**, and:
- `farm_sown:false` is **DOWNSTREAM** — ≥9 grown is impossible from 8 tilled.
  A dependent clause, exactly like `any_needs_materials` under
  `build_placed`.
- ★★★ **`farm_growth_rose:false` is an INSTRUMENT DEFECT — the clause is
  UNSATISFIABLE in this run.** Chased it rather than handing it off:
  ```rust
  let g1 = server.bastion_sprite_growth(probe_cell).unwrap_or(0);  // = 15
  for _ in 0..900 {
      let g = ...;
      if g > g1 { rose = true; }                     // needs g > 15
      if g >= 15 || grown_cells(&server,15) > 0 {    // breaks AT 15
          matured = true; break;
      }
  }
  ```
  **`g1 = 15` at window open** (it is right there in the telemetry). `rose`
  requires `g > 15`; the same loop **breaks the moment `g >= 15`**. **The crop
  was already mature before the watch began, so "did it rise" can never become
  true and the loop exits on iteration one.**

  **`farm_growth_rose: false` reports "growth didn't rise" when the truth is
  "growth had already finished before we started watching."** Same family as
  the `tool 0.0` sentinel and the `starvation_cycles: 0` ambiguity — **a
  clause that CANNOT FIRE, reporting as a failure.** It is also measured at a
  single `plot.min` cell standing in for a plot-level property.

> **★ So farm's three red clauses are: ONE instrument defect
> (`growth_rose`, unsatisfiable), ONE dependent (`sown`, downstream of
> tilled), and ONE possibly-real shortfall (`tilled`, 8 of 9). Only one of
> three carries a product signal.**

**The "mystery under both stances" was a count threshold all along.**

★★★ **TILL COUNT = NINE. Class (a): job created, work never performed.**
From the same log (ANSI stripped — see the trap note below):
```
TILL jobs at z=455 : 9 distinct XY   (full 3x3 grid = 9)   missing: NONE
SOW  jobs at z=456 : 8 distinct XY                          MISSING: (24072, 20239)
```
**All nine till jobs EXIST; `tilled_count` reached 8.** One till job was
created and never completed — so the silent-skip classes (foreign sprite,
`occupied` suppression, terrain-read bail) are all EXCLUDED. **Farm is a true
member of the last-unit family, which stands at three.**

★ **POSITION: the missing cell `(24072, 20239)` is a CORNER of the 3×3**
(min-x, max-y) — structurally distinguished, YES. **But it is NOT `plot.min`
`(24072, 20237)`, the `growth_rose` probe corner**, which tilled, sowed and
grew to 15 normally. **So the one-worldgen-accident-explains-everything story
is dead: three red clauses, two independent causes** — an unsatisfiable watch,
and one uncompleted corner till. The missing SOW at the same XY is downstream

## ★★★★ SEAM ROW: un-park condition NOT MET on three seeds — parked on EVIDENCE now

5b's per-caller counters, three seeds, the instrument built for exactly this:

| seed | `self_rescue_calls` | emissions | **starved** | `emergency_calls` / emissions | `access_pending_true_ticks` |
|---|---|---|---|---|---|
| 52 | 6 | 0 | **0** | 4 / 3 | **488 — armed, never fired** |
| 54 | 11 | 0 | **0** | 1 / 0 | **0 — never armed** |
| 71 | **0 — never engaged** | 0 | **0** | 13 / 3 | **379 — armed, never fired** |

**DECISIONS #52's un-park condition (*"if the counters ever show starved cycles
anywhere, the row returns"*) has NOT fired.** Seeds 52 and 54 made **17
self-rescue calls with zero emissions and zero starvation** — those rejections
are **genuine `plan_access` refusals**, not non-calls. **The row stays parked on
evidence rather than on absence of evidence.**

**★ "Armed but never fired" is a state nothing could previously express.** Seeds
52 and 71 had the bar up for 488 and 379 ticks with zero starvation — no carve
request happened to be pending while it was armed. **That is a latent hazard
measured as latent**, which is a different and much more useful record than
either "the bar is harmless" or "the bar is biting."

### ★★ Instrument CROSS-VALIDATED, not merely present

**Seed 71's new `emergency_emissions: 3` reconciles exactly with the corpus's
pre-existing `b5_cascade_probe.access_emissions_max: 3`** (`members_seen: 1`, so
max == total). **A new counter that appears and reports plausible numbers can
still be wired to the wrong thing; one that agrees with an independent existing
measurement is validated.** This is the field-presence guard upgraded from *"do
the fields appear"* to *"do they agree with what we already had."*

### ★★★ Seed 71: `self_rescue_calls = 0` — access is NOT its problem

**15 of 27 cells unclaimed for the entire 360-cycle run, and the self-rescue path
never engaged once.** Its 3 emissions were entirely emergency-side. **So the
access seam is excluded for seed 71 by direct measurement** — which
**independently corroborates #59's arbitration-starvation reading from a
completely different instrument.** Two instruments, one conclusion, no shared
assumption.

## ★★ `run`'s window test — the harness ALREADY controls for hypothesis (b)

Both speed samples are structurally identical, and each is preceded by a
**warm-up**:

```rust
for _ in 0..60 { tick(&mut server, 5); if moved > 2.0 { break; } }
let p1 = pos_of(&server)..;  tick(&mut server, 45);  let p2 = ..;
let walk_rate = p1.xy().distance(p2.xy()) / 45.0;      // run side identical
```

- **Acceleration ramp: already excluded** — sampling starts only after >2 blocks
  of movement.
- **Path curvature: cancels in the ratio** — same method and geometry both sides.

> **Both mechanisms behind "deterministic measurement overhead" are already
> controlled for. The prior moves strongly toward a REAL speed shortfall.**

**Residual, named precisely:** the warm-up exits on a distance threshold checked
every 5 ticks, so the sample start can differ by up to 5 ticks of phase between
the two runs — **which matters only if the colonist is still accelerating at 2
blocks.** The window test still settles it and is cheap; **it is no longer
expected to rescue the threshold.**

**★ Implementation catch (routed to 5b before they built it):** `run_scenario`
contains **two loops that both use 45** — the CarvedStair settle loop and the
displacement sample. **The batch item needs the displacement sample**, and the
flag must reach **both the `tick()` AND the `/ 45.0` divisor, on both the walk
and run sides** — otherwise a longer window silently deflates the rate and
produces a wrong number that looks fine.

## ★★★ BATCH RECONCILIATION TABLE — pre-registered BEFORE the runs

**Built prospectively so results cannot be fitted to an interpretation after the
fact**, and so the FIELD-AGREEMENT guard has a baseline ready rather than one
constructed from the answer. Every value below is corpus-known at `wave19`
(deterministic, reproduced across independent fans).

| seed | mined | `access_emis_max` | `members_seen` | starv / crowd | claims | timeouts |
|---|---|---|---|---|---|---|
| **71** | 5/27 | **3** | 1 | 360 / 360 | 22 | 17 |
| **66** | 27/27 | **2** | 2 | 222 / 222 | 27 | 9 |
| **61** | 26/27 | **0** | 0 | 306 / 287 | 5 | 3 |
| **90** | 25/27 | **2** | 2 | 332 / 332 | 8 | 6 |
| **92** | 27/27 | **1** | 1 | — | — | — |
| **80** | 27/27 | **3** | 3 | — | — | — |

*(92 and 80 have no `mine_cell_diag` entries — they mined 27/27, so no cell
carried an outstanding job at sample time. Their reds are chop-side.)*

### Agreement checks the batch MUST pass before its results are read

1. **`emergency_emissions + self_rescue_emissions + proactive_emissions` must
   reconcile with `access_emis_max`** where `members_seen == 1` (max == total).
   **Confirmed already for 71: 3 == 3.** **66, 92 and 80 have
   `members_seen` 2, 3 and 3, so max ≠ total there — the check is an INEQUALITY
   (`total ≥ max`), not an equality.** *Stating that now, before someone reads a
   legitimate `total > max` as a defect.*
2. **`starvation_cycles` re-read must match the corpus exactly** (360/222/306/332)
   — these are deterministic; **any drift means the binary or the seed changed,
   not the colony.**
3. **Seed 61's `access_emis_max: 0` predicts `self_rescue_calls` may be 0 too** —
   if the counters show calls with zero emissions instead, that is new
   information, not a contradiction.

### Pre-stated branches, per item

| item | outcome A | outcome B |
|---|---|---|
| **71 / 66 traces** | attempts EXIST and fail ⇒ **shape A** (aging/cooldown family) | **ZERO attempts** recorded ⇒ **shape B** (cap/round-robin family) |
| **61 / 90 window** | completes late ⇒ **window-sizing artifact**, corpus-defect ledger | never completes ⇒ **real stall**, frozen-block identity is the lead |
| **92 raised cap** | probe completes ⇒ **UNKNOWN → known**, and its `ch_mixed` anomaly gets a second look | still incomplete ⇒ the cap is not the binding constraint |
| **80 column scan** | single-surface ⇒ **its no-route negative STANDS** under the body-width caveat | multi-layer ⇒ **negative unsound**, caveat everywhere it is cited |
| **`run` window** | ratio climbs toward **1.25** ⇒ window overhead after all | stays **~1.14** ⇒ **real speed shortfall**, constants are the spec |

> **★ Every branch is informative.** No item can come back "inconclusive" — the
> null outcomes are named and each one redirects a row rather than failing it.

## ★ ENGINE TIP ADVANCED: `460626a6e2` → `cf5757e31b` (5b's branch, fast-forward)

**Verified a TRUE fast-forward before pushing** — `merge-base(branch, remote engine)
== remote engine`, so no divergence and no merge commit. Remote confirmed at
`cf5757e31b` after the push (checked, not assumed).

**Contents:** bed reachability probe + the re-scoped two-error-model caveat · bed
walk-test (root-caused bed's fixture defect) · guard-row limbs 1+2 ·
terrain-ground-dump · ARB-ATTEMPT-01 step 1 (`ReleaseReason`) · the observability
row's access-plan counters (cross-validated on seeds 52/54/71) · solid-cell query
· settle-window flags.

### Lean pass — four checks

1. **`in_game.rs` was the only file outside the stated scope, and it is
   legitimate:** `benched_since_tick` needs the job id, so `.values()` →
   `.iter()` with `(_, j)` in the closures. **Filter and `min_by_key` semantics
   unchanged.**
2. **Field completeness structurally guaranteed** — no `Default::default()`
   spread at any `BastionJobInspect` literal, so the compiler rejects any missed
   site. *The usual struct-field-addition risk is absent by construction, not by
   diligence.*
3. **`ReleaseReason`: 26 push sites, zero passing a bare entity.** The enum's own
   comment is the positive-exemplar pattern — *"A nonzero count of these after
   step 2 would mean a site was missed, not that it's genuinely
   unclassifiable"* — **so a residual `Other` cannot be misread as benign.**
4. **Settle flags reproduce their literals** (`unwrap_or(180)` / `unwrap_or(45)`)
   with intent commented at both sites. Byte-identical when absent.

### ★★ GAP: batch item 7 has no instrument yet

`--b5-settle-iters` (61/90) and `--ck-settle-iters` (chokepoint) landed.
**Neither is the `run` displacement sample**, so the `run` window test cannot
run. **One flag must drive FOUR places** — both `tick()` calls **and both
`/ 45.0` divisors**, walk side and run side. **Miss a divisor and a longer
window silently deflates the rate: a wrong number that looks fine.** Two flags
instead of one would measure the ratio across mismatched windows and mean
nothing.

### ★ Correction to my own catch

I claimed both 45-loops lived in `run_scenario`. **The CarvedStair loop is in
`chokepoint_scenario`** — 5b's fresh enclosing-function read found it before
editing. **The substance of the catch held (`ck_settle_iters` is not the run
window); the location claim did not.** Eleventh correction, caught by the
builder, pre-build.

## ★ ENGINE TIP: `cf5757e31b` → `a85dec2912` — batch item 7's instrument lands

Single-commit fast-forward, verified (`FETCH_HEAD^ == cf5757e31b`, and
`merge-base(branch, remote engine) == remote engine` before the push).

**`--run-sample-ticks` — all four sites verified by reading, not by description:**

```
let sample_ticks = args.run_sample_ticks.unwrap_or(45);   // ONE source of truth
tick(&mut server, sample_ticks);                          // walk tick
walk_rate = .. / sample_ticks as f32;                     // walk divisor
tick(&mut server, sample_ticks);                          // run tick
run_rate  = .. / sample_ticks as f32;                     // run divisor
```

**Zero literal `/ 45.0` divisors remain in `run_scenario`.**

> **★ The spec asked for "one flag, not two"; the implementation is ONE SHARED
> LOCAL — so the walk and run windows cannot drift apart BY CONSTRUCTION.** Same
> shape as the absent `Default` spread in the previous branch: **a guard the
> compiler enforces beats a guard someone has to remember.**

**The flag's doc comment names its failure mode** — *"a flag that only changed
the tick calls would silently deflate the rate … producing a
wrong-but-plausible-looking number"* — so the coupling's *reason* survives, not
just the coupling. **Named-case rule in the first draft, second branch running.**

**A third `tick(&mut server, 45)` at `auton3_scenario` was excluded by reading
its enclosing function** — the same method that caught my `run_scenario`
location error.

**Filed, not blocking:** `--run-sample-ticks 0` divides by zero → `inf`, and
`ran_faster` evaluates `inf > inf * 1.15` = false. **Not a crash — a garbage
result shaped like a clean fail**, which is the taxonomy's worst object. One-line
guard whenever someone is next in the file.

### Batch is at SEVEN items with instruments; sequencing set

**Post-merge field-presence AND agreement re-verify first** (`RUSTC_WRAPPER=""`,
at `a85dec2912`), **then** all seven items against an attested tip. **No
measurement is spent on an unverified build** — and the reconciliation table is
already frozen, `total ≥ max` inequality included.

## ★★★ SECOND-PASS CORPUS SCREEN — 114 scalar paths, and the honest result is three-part

I read ~25 of the corpus's varying paths today; **the "under-read" claim applies
to my own coverage**, so I screened all 114 scalar paths for pass/fail
discrimination.

### 1. The screen's top signal was an ARTIFACT — and I nearly reported it

`ch_rings_tried` showed **FAIL mean 3.33 vs PASS mean 15.72** — a large,
*reversed* correlation (failing seeds search fewer rings). **The distributions
refute the means:**

```
FAIL: [1,1,1,2,3,3,3,3,4,4,5,10]                                  median ~3
PASS: [1×14, 2,2,2, 3,3, 4,4,4,4, 5,10,12,13,15,18,19,21,22,  70,77,121,121]
                                                                  median ~2
```

**Medians are near-identical. The PASS mean is driven entirely by four
outliers.** *Screening by mean ratio produced a false signal on a skewed
distribution* — the same aggregate-late lesson, now about my own screening
method: **a mean is an aggregate; check the distribution before believing a
delta.**

### 2. But the outliers ARE the finding — and it is a CLAIM THAT HELD

**Seeds 55 and 63 tried all 121 rings and found ZERO trees, and both PASS.**
The instrument's own doc names exactly this risk: *"if this ever creeps up
corpus-wide, the gate is quietly decaying toward vacuous-green even while
individual seeds still pass."*

**And the harness CATCHES it:**

| seed | rings | trees | `ch_oracle_class` | `ch_engaged` | `gt_tree_present` |
|---|---|---|---|---|---|
| 55 | 121 | 0 | **`precondition_unmet`** | **false** | false |
| 63 | 121 | 0 | **`precondition_unmet`** | **false** | false |

**`ch_engaged: False` on exactly 2 of 48.** The falsifier asserts its own
precondition, an **independent** ground-truth scan confirms no tree exists, and
the vacuity is correctly classified. **Another claim checked that HELD — this
machinery works as designed.**

### 3. The refinement: vacuity is DETECTED but not PROPAGATED

**Those seeds still count as full passes in the 36/48.** Nothing in the verdict
says the chop portion was vacuous.

> **The corpus's effective CHOP denominator is 46, not 48.** A reader computing
> "chop passes on 44 of 48" over-counts by two.

**This refines a claim I made today:** I told the architect wave20's guard-row
evidence was *"Chop-only in practice"* (b5 places zero `Gather` designations).
**Sharper: chop on 46 of 48 seeds, with 2 structurally unable to contribute.**
Same over-count class, one level down.

**Not a defect — a coverage-accounting gap.** Fix shape for the report-fix
backlog: **propagate `ch_engaged` into any per-clause coverage denominator**, so
"passes" and "exercised" stop being the same number.

### Also surfaced, less interesting
`timeouts_on_never_completed_jobs` (6.25 vs 0.00) and `mine_jobs_remaining`
(4.42 vs 0.00) separate perfectly but are **near-tautological** — a failing seed
has jobs left by definition. `travel_timeouts` (10.67 vs 2.78) and
`max_same_target_timeouts` (2.50 vs 0.97) are real friction differences and
expected. **None of these is a lead; recorded so the next screener doesn't
re-derive them.**

## ★★★★ EXERCISED DENOMINATORS — the architect's standing form, computed (n=48)

**Ruling applied:** *every per-kind coverage claim carries its
exercised-denominator alongside its pass count.* **"Passed" and "exercised" are
not the same number**, and five of twelve measured families prove it.

| family | exercised-criterion | **denominator** |
|---|---|---|
| mine | `mine_jobs > 0` | 48 / 48 |
| build | `build_ok_jobs > 0` | 48 / 48 |
| slope | `slope_jobs_total > 0` | 48 / 48 |
| flat | `flat_total > 0` | 48 / 48 |
| cascade | probe present | 48 / 48 |
| **tool** | factors readable (`> 0`) | **47 / 48** |
| **chop** | `ch_engaged == true` | **46 / 48** |
| **access** | self-rescue emissions `> 0` | **34 / 48** |
| **cavein** | `cavein_drop_cells > 0` | **8 / 48** |
| **cascade abort** | abort path fired | **1 / 48** |
| **Gather** | designations placed | **0 / 48** |

### What each low denominator changes

**`cavein` — 8 / 48.** Within the b5 corpus the cave-in path is exercised on
**eight** seeds. *(Distinct from the standalone `cavein` scenario in the green
list — different instrument; this is the corpus's sub-check.)* Any b5-derived
cave-in claim rests on 8, not 48.

**`access` — 34 / 48.** **Fourteen seeds never emit an access plan at all.** So
every access-related corpus conclusion — including the seam row's — rests on 34
seeds. **That is the denominator the seam row's evidence should have carried
from the start**, and it makes the "zero starvation on 3 seeds" result a sample
from 34, not 48.

**`cascade abort` — 1 / 48.** The same number that permanently parked task #34,
now expressed as what it is: **a denominator of one.** *A matched pair is
impossible because the exercised set has one member.*

**`tool` — 47 / 48.** ★ **The denominator computation independently
re-discovers seed 66's sentinel.** The 48th seed is not a tool failure; its
factor was **unreadable** (`.unwrap_or(0.0)`, below the metric's own 1.0 floor).
**A denominator computed from "is this readable" surfaces instrument defects for
free** — the method validating itself.

**`chop` — 46 / 48** and **`Gather` — 0 / 48**: the two already-known cases, now
in the same table at their true magnitudes. **Both were being quoted as if the
denominator were 48.**

> **★ Every one of these was previously reported as a fraction of 48.** The
> ruling's value is not per-family bookkeeping — it is that **five separate
> claims were quietly inflated by the same unstated assumption**, and one line
> per family retires all of them.
