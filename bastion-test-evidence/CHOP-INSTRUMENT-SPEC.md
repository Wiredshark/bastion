# CHOP INSTRUMENT — PER-JOB STATE, END-OF-RUN, ~2 ENTRIES

**Designed against the observer-effect budget FIRST, per the ruled order.**
Producers read at `e5a288d9cc`. **Target: why chop fails on 78 / 80 / 85 / 92.**

## §1 — ★ THE GAP, PRECISELY (I got its absence wrong twice; this is from producers)

| | mine | chop |
|---|---|---|
| per-cell/per-job state | ★ **`mine_cell_diag`** — progress, claimant, `times_offered`, `starvation_cycles`, `cycles_since_last_claim`, `timeouts_on_this_cell`, `is_column_frontier`, `unreachable`, `needs_materials` | ★ **NOTHING** |
| reachability probe | per still-open cell (**N probes**) | **ONE** — `probe_target(chop_base)`, only when `!chop_cleared` |
| timeout attribution | `mine_timeout_position_diag` (per position) | one scalar at `chop_base` |

**Chop is not uninstrumented — it is instrumented for REACHABILITY and for nothing
else.** ★ **We can say where the tree is and whether a path exists. We cannot say
whether anyone ever tried.**

## §2 — ★★★★★★ SEED 85 IS THE WHOLE CASE

`b5_chop_reachability_probe` for seed 85: **`path_exists_step / jump / scramble =
TRUE, TRUE, TRUE`.** `min_distance = 8.5`. **And `chop_cleared = False`,
`log_sum = 0`.**

> **The target was reachable by every locomotion mode, and the chop still failed
> — and the corpus carries NO FIELD that could say why.**

★ **That is not a reachability failure**, so the one instrument chop has is
**structurally incapable** of explaining it. **Seeds 80 and 92 show all-modes
FALSE** (a reachability story is at least available); ★ **85 has no story
available at all**, and **78 differs again** (`(False, False, True)`,
`min_distance = 33.5`, `ch_base_blocked_by` **null** where 80/85/92 are
populated).

> ★★★ **Four seeds, at least three different shapes, one instrument that
> addresses one of them.**

## §3 — THE INSTRUMENT: mirror `mine_cell_diag`, per TREE

**`b5_ch_job_diag`** — one entry per chop job still open at settle:

```
pos · claimant · progress · times_offered · cycles_since_last_claim
starvation_cycles · starvation_crowded_cycles · timeouts_on_this_cell
unreachable · needs_materials · blocked_by
```

★ **Every one of these already has a server getter** — the same ones
`mine_cell_diag` calls (`bastion_starvation_stats`, `bastion_timeout_count_for_pos`,
`bastion_blocked_by`, `bastion_inspect_cell`). **No new engine state. A JOIN, not
a measurement** — the row's third instance of that shape.

**It answers the questions seed 85 currently cannot:** *was it ever claimed? did
anyone make progress? was it offered and passed over? was the claimant starved?*

## §4 — ★★★★★ THE BUDGET, STATED UP FRONT AND SMALL

- ★ **Cost: `b5_ch_trees` is 1–2 in every corpus seed** (1 for 80/85/92, 2 for 78).
  **So this is 1–2 entries — versus `mine_cell_diag`'s per-cell scan over a whole
  designation volume.** **Two orders of magnitude cheaper than the instrument it
  mirrors.**
- ★★★ **PLACEMENT: END-OF-RUN, after final verdicts** — the presumptively safe
  shape per the ruling. **Nothing left to perturb.** *No mid-run capture, so no
  noise-floor run is required before trusting a widened wave.*
- ★★★★★ **AND THE PRESUMPTION IS VERIFIED FREE, NOT TRUSTED** (Fable's
  refinement): **the additive window's own hold-check on PRE-EXISTING fields IS
  the noise detector.** If settle-time reads perturbed anything, **old fields
  move and the hold-check fires**; if they hold, **the presumption is confirmed
  as a side effect.** ★ **Read the hold-check result AS that confirmation and say
  so in the wave notes** — *"presumptively safe" becomes "measured safe" at zero
  cost.* **A presumption that can be checked for free must never be left a
  presumption.**
- **Reads are at settle, not per tick.** ★ **The observer-effect bisection
  indicted PER-CELL, PER-TICK reads; this is per-job, once.**
- **Additive schema window**, comparable baselines, `--expect-new` covers it, and
  the identical-seed-set precondition is asserted by the tool already.

## §5 — ACCEPTANCE

- ★ **PRIMARY:** for seed 85 — reachable, failed — the report **says something**.
  *Whether it names the cause or only narrows it, the row succeeds when the
  currently-empty question has an answer shaped like data.*
- **Planted-failure test:** a chop job that is **claimed and progressing** must
  produce **visibly different** entries from one **never claimed**. ★ *An
  instrument that reports the same thing for "nobody tried" and "someone tried
  and failed" is the defect restated.*
- **Regression:** passing seeds keep `b5_ch_job_diag` **empty** — like
  `mine_cell_diag`, **non-empty means work remained.** ★ **Empty must mean
  finished, never "not looked at"** — state which in the field's own doc.
- ★ **GATE FIELDS:** `b5_ch_job_diag` — **currently INSTRUMENT-GAP.** No gate
  claim about chop failures is admissible until it lands.
- ★ **EXISTING TESTS/FIXTURES COVERING THIS MECHANISM:** `b5_chop_reachability_probe`
  (reachability only), `b5_ch_*` scan fields, `ch_oracle_class`. **NONE covers
  per-job state.** *(Third template slot, filled honestly.)*

## §6 — WHAT THIS DOES NOT DO

- **Not** a fix. **An instrument.** ★ The chop failures stay undiagnosed until it
  reports, **and that is the point** — I have twice this week built a story on an
  instrument's shape instead of its output.
- **Not** dependent on the arbiter run. ★ **Chop reachability and the
  router/probe contradiction are separate questions**; this can land in parallel
  without touching Class B, `route_next_idx`, or either contradiction population.
- **Not** a cascade claim. **Constants verified at tip: build eats STONES
  (`MINE_DROP_ITEM`), chop drops WOOD.** ★ **Chop cannot feed build. Family 2 is
  two independent failures**, and this instrument addresses only the chop half.
