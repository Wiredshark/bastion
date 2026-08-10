# WAVE 33 — F3 stall corpus (item 2 threshold-setting wave)

**Binary:** `07ba0cc17b` · **Fan:** 4 VMs × 12 seeds = 48 (seeds 49–96)
**Wave JSON:** `corpus-waves/wave33_F3STALL_07ba0cc17b_FULL.json`
**Fan log:** `corpus-waves/wave33-fanlog-07ba0cc17b.txt`

## ATTESTATION

All four pool logs carry `COMMIT=07ba0cc1` and `DONE=12`; 48 `@@@SEED` blocks
collected; no `STALE`, no `BUILD_FAIL`. Collector exit 0.

## FIELD VALIDATION — 48/48 after a CHECKER FIX

First pass: **40/48**. The 8 failures were **my checker's defect, not the
engine's** — proven at source before any interpretation.

`f3fields_validate.py` clause 3 asserted `prunes_fired > 0` implies
`idle_peak >= ACCESS_STALE_SECS`. That is a **single-producer** assumption,
written when branch B's sweep was the only writer. At `07ba0cc17b`
`b5_f3_prunes_fired` has **two** producers:

| site (`bastion-server/src/bastion_jobs.rs` @ `07ba0cc17b`) | branch | fires when |
|---|---|---|
| `:15129` | B `Idle` | `access_idle_secs >= ACCESS_STALE_SECS` (20.0) |
| `:15199` | C `ClaimedOrAbsent` | `access_stalled_secs >= ACCESS_STALL_SECS` (120.0) |

Item 2 added the second producer to the existing counter. The counter is now a
**union**, and no single threshold explains it — the exact shape of
`new-producer-must-fit-the-stores-unit`, committed by me and caught by my own
data.

**The data partitions perfectly against the corrected reading — zero unexplained
prunes:**

- **3 seeds** (68, 76, 79) — `idle_peak == 20.0` → branch B stale prune
- **8 seeds** (53, 58, 60, 63, 65, 69, 72, 89) — `stalled_peak == 120.0` → branch C stall prune
- **0 overlap · 0 prunes without a named threshold reached**

**Fix:** clause 3's converse deleted; replaced by a **prune-attribution** clause
that fails only when *neither* threshold was reached. It is strictly stronger —
it catches an unexplainable prune (a third producer, or a reset that cleared a
peak before capture) rather than merely one inconsistent with a single
threshold. **Non-vacuity proven:** a planted seed (seed 53 with `stalled_peak`
forced to 1.0) goes **red by name**, exit 2. Re-run: **48/48, exit 0.**

## THE THREE REGISTERED QUESTIONS

### 1. `b5_f3_prunes_fired` — does the prune EVER fire? **YES — #38 CLOSES.**

**14 prune events across 11 of 48 seeds** (11 stall-path + 3 stale-path by
seed; per-seed counts 1–3). The pruner is live, reachable, and both of its
paths are exercised by the corpus scenario. #38's witness question is answered.

### 2. `b5_f3_ticks_branch_a` — was 46% a one-scenario artefact? **YES.**

**Branch A is 0 in all 48 seeds. `min=0, max=0, mean=0.000, nonzero=0/48`.**

The earlier 43.9%/46.4% `material_held` dominance does **not** generalise: in
this corpus scenario the `MaterialHeld` branch never executes once across 48
seeds. Branch C dominates totally (`nonzero=48/48`, mean 438 passes); branch B
is sparse (`nonzero=22/48`, mean 4.1).

> **Consequence: option 3's build — scoped on branch A — earns nothing this
> corpus can measure.** It should not be built on the strength of the row-60
> number. Whether branch A is reachable at all in *some* scenario is a separate,
> unanswered question; what is settled is that the 46% figure was
> scenario-specific and is not a general property.

### 3. `b5_f3_stalled_peak` — the distribution that sets `ACCESS_STALL_SECS`

> ## **THE WAVE CANNOT SET THIS THRESHOLD. THE FIELD IS RIGHT-CENSORED BY THE THRESHOLD IT WAS BUILT TO CALIBRATE.**

`access_stalled_secs` **resets to 0.0 the instant it reaches
`ACCESS_STALL_SECS`** (`:15199`, immediately after the sweep). So the peak can
never exceed 120.0. Eight seeds report **exactly 120.0**; their true stall
dwell is `>= 120` and **unmeasured at this build**.

Observed distribution (48 seeds): 17 × `0.0`; then 13, 24, 38, 40, 40, 49, 51,
52, 58, 60, 61 × 6, 62, 72, 74, 76, 83, 91, **119**; then **8 × 120.0 (censored)**.

The const's own comment instructs: *"set from #70's corpus fields across 48
seeds, not from this constant's current value."* **That instruction cannot be
followed as written** — the constant's current value is manufacturing the tail
it would be calibrated from.

**What the wave DOES establish:**

- A **non-pruned** case reached **119.0** (seed 59, `prunes_fired = 0`) — one
  second under the wire. **The current threshold sits essentially at the top of
  the observed non-pruned range**, which argues for **raising** it, never
  lowering it, pending an uncensored measurement.
- The 8 censored seeds cannot be classified. Whether they are true deadlocks
  (120 is correct) or slow-but-progressing legs being killed (120 is too low)
  is **not decidable from this field**.

**A second, independent gap blocks even the uncensored read:** `stalled_peak`
alone cannot separate *"stalled 119 s then recovered"* from *"still stalling
when the run ended."* The peak is identical in both.

**To actually set the threshold, two small changes — both required:**

1. **Uncensor the top.** `ACCESS_STALL_SECS` and `ACCESS_STALE_SECS` are
   hard-coded `const`s (`:15032`, `:15040`) with **no env override**. Make them
   env-tunable (no rebuild per arm, and it makes the A/B a fan parameter), then
   run the corpus with the stall threshold far above any plausible work leg.
2. **Emit the FINAL `access_stalled_secs` beside the peak.** `peak` vs `final`
   discriminates *recovered* from *truncated-by-run-end*, which no high-water
   mark can do alone.

Until both land, **120.0 remains PROVISIONAL and is not licensed by this wave.**

## CROSS-WAVE: 5 MOVERS, ALL **UNATTRIBUTED**

Registered comparison (`--baseline wave32_RETUNE_156a2eceb4_FULL.json`, never
auto-select): **11 → 12 failing; 5 newly failing: 50, 51, 52, 69, 76.**

*(The auto-select run reported only 2 movers against a union of 11 exploratory
baselines. The registered read reports 5. This is exactly the divergence #67
exists to prevent.)*

Two clean signatures:

| seeds | gained clauses | F3 activity |
|---|---|---|
| 50, 69 | `any_needs_materials`, `build_placed` | 50: no prune · 69: stall prune |
| 51, 52, 76 | `mine_blocks_mined`, `mine_cleared` (+`b15_adjacent_claimed` on 52) | 51, 52: **zero branch-B dwell** · 76: stale prune |

> **ATTRIBUTION REFUSED.** The wave32→wave33 range is **16 commits**, +1353/−174
> across 12 files, **988 lines in `bastion_jobs.rs` alone** — item 2, item 6
> (`b9438f19a7`, `cbfb8ae977`), #94 (`6797b5c409`, `d0ce0a58e1`), #89
> (`48886adfa0`), #70, #68. **This is a bundle diff, not an A/B.** Controlling
> the commit is not controlling the mechanism.

Seeds **51 and 52 had zero branch-B dwell and fired no prune** — the F3 pruner
did nothing on them, so item 2 cannot explain 3 of the 5 movers.

**AND THE BUILD SIGNATURE IS WIDER THAN THE MOVER LIST SHOWS.** Seed 66 — a
PERSISTENT failer, so invisible in the newly-failing count — **swapped** its
clause set: it lost `tl_ok` (the tool refusal, now resolved) and **gained
`build_placed`, `stone_sum_lower`.** That is the *same* build/material family as
seeds 50 and 69. So the build signature spans **at least 3 seeds (50, 66, 69)**,
and one of them was hidden inside a seed that "already failed."

> This is precisely the case `collect_wave.py` warns about: **"a GAINED clause is
> a REGRESSION the fail COUNT cannot show — the seed was already failing."** The
> count moved 11→12; the actual clause-level movement is larger, and it
> **strengthens the case that something in this range affects build material
> flow** — which is exactly what item 6's protection gate governs and what the
> corpus cannot see (gap 1).

**The strongest untested candidate is item 6's protection/ambient-loot gate**,
which directly governs whether colonists may take materials and would gate
`any_needs_materials`, `build_placed`, and mining alike. **It could not be
tested:** see the instrument gap below.

## INSTRUMENT GAPS FOUND

1. **Item 6 is invisible to the corpus.** No field matching
   `pile|protect|ambient|loot|provision` exists in any of the 48 seeds. The
   protection mechanism and its ambient-loot gate — a shipped behavioural
   change — emit **nothing** the fan can read, so a wave cannot implicate or
   clear them. (Compounds task #78: the AI-side gate already refuses silently.)
2. **`b5_stack_reserved_units_max` is 0 across all 48 seeds.** #89's reservation
   capacity field never moves in this scenario — a dead field in this corpus.
3. **`stalled_peak` has no `final` sibling** (see above).
4. **Constancy transition — READ AND CLOSED: reading (b), a real improvement.**
   5 fields stopped varying (`b5_tool_ok` now always `true`,
   `b5_tool_steel`/`_measured` 2.0, `b5_tool_stone`/`_measured` 1.5).

   Producer read at `bastion-harness/src/main.rs:4550-4586` (`07ba0cc17b`).
   **My first hypothesis was wrong and the data refuted it:** I expected the
   wave32 variance to be REG-1's old `.unwrap_or(0.0)` sentinel. There were
   **zero `0.0` values in wave32** — REG-1 had already landed by `156a2eceb4`.

   The actual variance was **one seed, 66**, whose tool lookup honestly
   **refused** (`None` on all three fields → failed clause `tl_ok`). In wave33
   it **succeeds** (1.5 / 2.0 / `true`). All 48 seeds now measure; **zero nulls,
   zero absents.** Nothing was lost — a refusal resolved.

   *Seed 66 is the very specimen REG-1's comment names ("survived every presence
   check silently (seed 66)"), so the `Option` conversion did exactly its job:
   the failure was visible as a refusal rather than as a plausible number.*

## STANDING

- #38 **CLOSES** — the prune fires, both paths, 14 events.
- Branch A's 46% is **retired as scenario-specific**; option 3 unjustified.
- `ACCESS_STALL_SECS = 120.0` stays **PROVISIONAL**; the calibration this wave
  was launched to perform is **blocked on two small instrument changes**, not on
  more seeds. More seeds cannot fix censoring.
- The 5 movers are **open and unattributed**; they need a real A/B, not a bundle.
