# NEED-SUBSYSTEM OBSERVABILITY — WHY THE FAN HAS NEVER GATED AN AUTON ROW

**Written 2026-08-08 while site 6 builds. ★ Measured, not asserted:
every count below comes from `corpus-waves/wave29_ROWBPRIME_B_7590dfa962_FULL.json`
(48 seeds, 283 leaf fields) and from the emission sites in
`bastion-harness/src/main.rs` at `0fb7ca07b7`.**

## ★★★★★★★ THE MEASUREMENT

**Leaf-field counts in the newest wave, by substring:**

    bed 0 · sleep 0 · eat 0 · hunger 0 · despond 0 · mood 0
    preempt 0 · orphan 0 · sweep 0 · stuck 0 · settle_invariant 0

★★ **The 27 `job` leaves are all WORK-job counts** — `b5_build_ok_jobs`,
`b5_build_stall_jobs`, `b5_chop_jobs`, `b5_mine_jobs`, `b5_mine_jobs_remaining`,
`b5_slope_jobs_total`, `b5_slope_legacy_jobs`, … ★ **Not one is a lifecycle,
claim-identity, or need field.**

> ## ★★★★★ **THE COLONY-SIM CORPUS CANNOT SEE THE NEED SUBSYSTEM AT ALL.**
> **Not "sees it poorly." Zero fields.**

### ★★★ AMENDED SAME EVENING — ONE FIELD ARRIVES IN WAVE 30, AND IT IS NOT A NEED FIELD

**`self_job_reachability_probe` lands in the pre-site-6 baseline** *(commit
`a2745d5a7d`, post-dating `wave26`; verified absent from both `wave26` and
`wave29`, whose only "self" keys are `b5_access_plan_self_rescue_*`)*.

★★★★★ **So wave 30 is the first wave in this corpus carrying ANY self-job-specific
field — by a change directed this morning, which I had not connected to the
baseline until reading `b5_scenario` tonight.**

> ★★ **BUT IT DOES NOT WEAKEN THE MEASUREMENT ABOVE.** *It reports self-job TRAVEL
> FAILURE — which positions timed out — and says nothing about rest, hunger, mood,
> drive selection, or job identity.* ★★★★★ **N1-N6 are all still needed, unchanged.**

★★★ **What it DOES give is an EXPOSURE POPULATION** *(which seeds demonstrably
run self-jobs)*, **which is why it matters out of proportion to its size:**
★ *it converts every future need-drive claim from a 48-seed dilution into a
conditioned comparison.* ★★ *`WIP-STATE.md`'s own lesson — the 48-seed aggregate
diluted the last result ~4:1 and hid it.*

★ **And its author named its failure mode: the first version mislabeled completed
mine cells as self-jobs** *(seed 90, 6 entries, all inside the mine designation)*.
★★★ **Verify entries fall OUTSIDE the mine region before trusting the population.**

★★★ **This has now cost twice: the fan was structurally unable to see the AUTON-2
defect, and it is unable to see all four of site 6's predictions
(`SITE6-DELTA-REGISTRATION.md` D1-D4).**

## ★★★★★★ THE SHARPER FINDING: INSTRUMENTED EVERYWHERE, OBSERVED IN ONE PLACE

**`board.settle_invariant_violations` increments in the sweep
(`bastion_jobs.rs:8794`), and the sweep runs in EVERY scenario.**
★★★ **The harness surfaces it ONLY inside `fn auton2_needs_probe`**
*(`main.rs:10037`; emits at `10557-10559`).*

> ★★★★★★★ **My own directive was: *"the invariant runs in EVERY scenario — one
> that runs only in the fixture that tests it protects nothing else."*
> ★★★ IT IS INSTRUMENTED EVERYWHERE AND OBSERVED IN ONE PROBE. I verified the
> half that was already done.**

★★ **EXISTENCE · CALLERS · SEMANTICS · EMISSION SITE are four separate reads.**
★ **A field that exists in the harness is not a field any given run reports** —
*and I made exactly this mistake today when I asked 5b to read the invariant off
`preempt_scenario` runs that never emit it.*

## ★★★ THE MINIMAL FIELD SET — SETTLE-TIME AGGREGATES ONLY

**Budget discipline is not optional here.**
★★★★★ **`the-instrument-changes-what-it-sees`: two per-cell-per-tick diagnostic
reads made a bit-reproducible run start varying.** ★ **Everything below is a
counter read ONCE at settle, or a counter incremented at an existing seam that
already iterates the population.**

| # | field | source | cost | what it would have CAUGHT |
|---|---|---|---|---|
| **N1** | `settle_invariant_violations` *(cumulative u64)* | ★ **already exists** — just needs emitting in the common summary | **zero** | ★★★ the sweep race, in every scenario instead of one |
| **N2** | `self_jobs_created` **by kind** *(3 counters)* | the need-check pass's own insert sites | **zero** — increment at an existing branch | ★★ *"the need-check pass keeps inserting fresh jobs"* — **site 6's D2/D3 without a bespoke run** |
| **N3** | `self_jobs_reclaimed` **by kind** | site 6's new re-claim arm | **zero** | ★★★★★ **N2 vs N3 IS D2/D3.** *Re-claim landed iff N3 > 0 and N2 falls.* |
| **N4** | `self_job_stuck_strikes_max` | the sweep's existing pass over `board.jobs` | **one field per pass, ~2 Hz** | ★★★ **`stuck_strikes` stuck at 0 across 660 ticks** — *the family-1 defect, measured ad hoc today because no field carried it* |
| **N5** | `drive_selected_counts` *(Work/Flee/Idle/Personal)* | the arbiter's existing selection point | **zero** | ★★ *"Personal never wins"* / *"Personal never releases"* — **the ENDURE wedge, which took a bespoke seed hunt** |
| **N6** | `despond_started` / `despond_resumed` | the breakdown roll + site 6's re-claim | **zero** | ★★★★★ **a re-roll wearing a resume's name** — *Fixture 2's roll-count assertion, made universal* |

★★ **N1-N3, N5, N6 are pure counter increments at branches the code already
takes. N4 is one `max` inside a loop that already runs.** ★ **No new iteration,
no per-cell read, no per-tick scan added anywhere.**

## ★★★★★ WHAT THIS BUYS, STATED AS A GATE

**Today:** ★ *the fan proves site 6 didn't disturb the WORK economy, and nothing
else. Every mechanism claim needs a bespoke instrumented run.*

**With N1-N6:** ★★★★★ **the fan becomes a REAL gate on need-drive behaviour** —
*D1 through D4 become fan-visible, and the next AUTON row inherits the
instrument instead of rebuilding it.*

> ★★★ **AND THE BLINDNESS STOPS BEING FREE.** *The current blindness has one
> genuine virtue — it makes the fan a perfect harmlessness gate, exact-match-or-
> bust, because no field is downstream of need behaviour by construction.*
> ★★★★★ **ADDING N1-N6 FORFEITS THAT. Once these fields exist, an exact-match
> bar over the WHOLE summary is no longer meaningful for a need-drive change.**

★★ **THE HANDLING, AND IT MUST BE DECIDED BEFORE THE FIELDS LAND:** *the
harmlessness bar becomes exact-match over the **work-economy subset** (the 27
`job` leaves and their downstream counts), with N1-N6 read as the **mechanism**
half.* ★ **Two bars over two field sets, not one bar over a mixed summary.**

> ★★★★★ **This is the `enumerate-the-delta` law arriving one level up: the
> instrument's OWN expansion changes what a baseline means, and the split has to
> be named BEFORE the first wave carries the new fields — otherwise the next
> re-baseline silently absorbs it.**

## ★ SEQUENCING — NOT THIS ROW

★★★ **Site 6 is mid-build with four sites and a schema addition. None of this
goes in it.**

1. **Site 6 / 4 / 5 land and pass on the bespoke instruments.**
2. ★ **N1 alone** *(zero cost, already-computed value, one emit site)* — **the
   cheapest possible test of whether the field-set split is workable.**
3. **N2-N6 as one additive window**, with the two-bar split registered first.
4. ★★ **A fresh wave becomes the baseline for the need subsystem** — *the first
   one that can see it.*

## ★★ WHAT THIS SPEC DOES **NOT** CLAIM

- ★ **Not** that the current fan is broken. **It is correctly scoped to what it
  was built for** *(the work economy)* — **the defect is that AUTON rows were
  gated against it anyway.**
- ★★ **Not** that N1-N6 are sufficient. **They are the set that would have caught
  the SIX defects this lane actually hit** — *named in the table, each to its
  incident.* **A field justified by a hypothetical is a field nobody reads.**
- ★ **Not** a schedule. **Sequencing above is a proposal; the fan is mine to
  schedule and I am not scheduling it over an in-flight build.**
