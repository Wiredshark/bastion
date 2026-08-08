# CHOP + BUILD instrument window: per-job state for the corpus's two least-instrumented failure modes

Implements `CHOP-INSTRUMENT-SPEC.md` and `BUILD-INSTRUMENT-SPEC.md`
(committed `fa6363feb1`/`a17bd80d9b`/`04900b2a06`), composed in one window
per the ranking (build analyzed first). New fields `b5_ch_job_diag` and
`b5_build_job_diag`, mirroring `mine_cell_diag`'s own shape — one entry
per job still open at the capture point, using getters
`mine_cell_diag` already calls (`bastion_starvation_stats`,
`bastion_timeout_count_for_pos`, `bastion_inspect_cell`), plus one new
minimal getter (`bastion_job_material_info`, `required_item`/
`reservation` — the two `Job` fields not exposed by the shared
`BastionInspectKind::Job` inspect struct).

## A placement bug found and fixed before this landed

Both specs specified **END-OF-RUN** placement ("nothing left to
perturb," verified free by the window's own hold-check). First version
placed both diags right before the final `json!` macro, per that
reading. **Both came back empty on known-failing seeds** — seed 71
(`build_placed: false`) and seed 85 (`chop_cleared: false`) — which
would have been silently indistinguishable from "completed," exactly
the ambiguity both specs' own regression clause warns against.

Traced seed 71's full log directly: job 28 (build_ok) churns twice
("job unreachable — claim released") then is never mentioned again;
job 32 (build_stall) is claimed once then never mentioned again.
Neither shows an explicit completion or removal event. Between their
last mention and the scenario's end, a **later, unrelated
designation-churn/skill-soak phase** runs — repeated "surface
designation placed"/"designation cancelled" pairs with escalating job
counts (72, 45, 108, 108, 549, 1...), clearly a different test's own
activity sharing the same coordinate space, silently clearing whatever
it touches as a side effect.

**Fix:** moved both diags to capture right after each designation's own
resolution window instead — `build_job_diag` right after
`build_stall_kind`'s "couple of arbitration cycles… for the
needs_materials sweep to run" tick, `ch_job_diag` right after
`chop_reachability_probe` (which already treats `chop_base` as the
gated fixture at that point). Both specs' placement rationale doesn't
hold for this scenario; the actual safe window is narrower than
"anything before the final json!"

## A second discovery along the way: `ch_aabb`'s ring-search self-destructs

Almost used `ch_aabb` (the scenario's separate ring-search for
auto-detected trees, `ch_trees`/`ch_ground_truth_witness`) as
`ch_job_diag`'s source instead of `chop_base`. Reading its own code
first: `ch_cancel_clean` **deliberately** calls
`bastion_cancel_designation` on whatever it finds, immediately followed
by a second, sweeping cancel across `(cx±160, cy±160, z 0..2048)` —
this ring-search block is a self-contained test of cancel-designation
semantics that erases everything it touches by design. Using its aabb
would have read either nothing yet (captured too early, before line
4238 populates it) or a target this same function is about to destroy.
`chop_base` — the scenario's own fixture tree, already the position
`chop_cleared`/`chop_reachability_probe` treat as gated — is the
correct, stable source.

## Verified against the corpus

**Seed 71** (calibration control per `BUILD-INSTRUMENT-SPEC.md` §4 —
already known-caused, mine failed first, `stone_sum: 5` not 27):

```
ch_job_diag: []   (chop_cleared: true — chop succeeded, correctly empty)
build_job_diag: [
  { pos: [25168,21092,603], claimant: null, unreachable: true,
    timeouts_on_this_cell: 2, times_offered: 2, starvation_cycles: 323,
    cycles_since_last_claim: 104, needs_materials: false,
    required_item: "...crafting_ing.stones" },   // build_ok
  { pos: [25168,21052,592], claimant: "Rhosyn Ironhand",
    unreachable: false, progress: 0.0, needs_materials: false,
    times_offered: 1, starvation_cycles: 1 }       // build_stall
]
```

build_ok's entry is unambiguous: claimed, churned twice
(`timeouts_on_this_cell: 2` matches the traced log exactly),
`unreachable: true`, then starved for 323 cycles with no further claim
attempt — **state 2, "claimed, never arrived."** build_stall's entry is
a **fourth shape not in the spec's original three-state taxonomy**:
claimed, `unreachable: false` (not stuck), `progress: 0.0`,
`needs_materials: false` (not blocked on materials) — someone has it,
isn't stuck reaching it, isn't blocked on materials, and still hasn't
progressed. Flagged as a discovery, not force-fit into states 1–3.

**Seed 71 classifies visibly differently from a passing seed and shows
a distinct shape from seed 85 below — the calibration requirement
holds**, though not by literally reproducing the "mine failed, stones
never existed" story (the diag doesn't re-derive causation, per §7 of
the spec — it reports state, and the state it reports is genuinely
different).

**Seed 85** (`CHOP-INSTRUMENT-SPEC.md`'s named "whole case": reachable
by every locomotion mode, `chop_cleared: false`, previously no field
could explain why):

```
ch_job_diag: [
  { pos: [23236,17648,317], claimant: null, unreachable: true,
    timeouts_on_this_cell: 2, times_offered: 2, starvation_cycles: 283,
    starvation_crowded_cycles: 266, cycles_since_last_claim: 66,
    blocked_by: [23236,17648,317] }   // self-referencing its own pos
]
```

Directly answers the spec's own question: was it ever claimed? **Yes,
twice.** Did anyone make progress? **No** (`progress` absent from the
entry entirely — never got far enough to accrue any). Was the claimant
starved? **283 cycles since the last claim, 266 of them crowded** — a
target genuinely available but not being retried, sitting starved
rather than actively failing. `blocked_by` pointing at its own position
(not a different obstruction cell) is itself informative: whatever
recorded the block did so against this cell directly, not against some
upstream gating cell elsewhere in a chain.

**Seed 42** (passing) regression check: `ch_job_diag: []` — clean.
`build_job_diag` shows **one entry**, at `build_stall_pos` —
`needs_materials: true, times_offered: 0, claimant: null`. This is
**not a regression**: `build_stall_untouched: true` for this seed
confirms `build_stall_pos` is a **deliberate, permanent negative-control
fixture** (by design, never meant to resolve — testing that a stalled
build doesn't magically complete) — it appears in `build_job_diag` on
every seed, pass or fail, and that's correct. The corpus's own
`build_placed` gate only tracks `build_ok_pos`; it was already `true`
within this same early window for seed 42, confirming `build_ok`'s
resolution happens fast and fits inside the earlier capture point fine.

**Consequence for the field's own regression convention:** "empty
means completed" holds for `ch_job_diag` and for `build_job_diag`'s
`build_ok_pos` entry specifically, but **not** for `build_stall_pos`'s
entry, which is expected non-empty on every seed by construction. A
reader treating any non-empty `build_job_diag` as a failure signal
would misread every seed. Documented in-code at the field's own
construction site; restating here since it's the sharpest correction
this row produced.

## Status

Additive, no existing field changed. `b5_ch_job_diag`/
`b5_build_job_diag` were `INSTRUMENT-GAP` before this; no gate claim
about chop/build failure state was admissible before it landed. Both
now report real, distinguishing per-job state on every tested seed.
