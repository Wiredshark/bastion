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
  NOT that `plan_access` is correct.
  **Next question is NOT capsule modeling** (see the correction below —
  `plan_access` is a construction planner, not a reachability check), and it is
  **not which call site bed reaches either**. It is: **was `plan_access` called
  at all?** The self-rescue site is gated `take(0)` while ANY `is_access` job
  lives colony-wide (see the seam finding below), so *"fails at `plan_access`"*
  may be a **NON-CALL**. Falsify with the existing read-only probes
  `access_job_dump` / `access_block_reason` BEFORE tracing any arm. 5b assigned.
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

**So the probe models a POINT — it answers "can a dimensionless walker get
there".** That is narrower than "can a colonist get there", and the soundness
of a point model is asymmetric:

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

**Zero `probe_incomplete` in the entire corpus** — every probe that ran, ran to
completion. No "couldn't measure" hiding in this data
([[log-time-namespace-and-vm-attestation]]: that value must never be read as a
negative).

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
