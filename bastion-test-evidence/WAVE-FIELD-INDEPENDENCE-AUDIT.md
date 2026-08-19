# THE WAVE INSTRUMENT CARRIES 63 INDEPENDENT SIGNALS, NOT 119 FIELDS

Found by generalising a single duplicate pair in `b5_mine_cell_diag`. Banked
wave34 corpus, 48 seeds, no spend.

| | |
|---|---|
| `b5_*` fields declared | **119** |
| **DEAD** — constant across all 48 seeds | **50 (42%)** |
| live | 69 |
| **INDEPENDENT signals** (identical columns collapsed) | **63** |
| **columns that cannot move on their own** | **56 of 119 (47%)** |

## The seven exact duplicate pairs — and they genuinely vary

| A | B | distinct values |
|---|---|---|
| `b5_any_needs_materials` | `b5_build_placed` | 2 |
| `b5_ch_cancel_clean` | `b5_ch_engaged` | 2 |
| `b5_ch_cancel_clean` | `b5_ch_ground_truth_tree_present` | 2 |
| `b5_ch_engaged` | `b5_ch_ground_truth_tree_present` | 2 |
| `b5_ch_jobs` | `b5_ch_trees` | 10 |
| **`b5_tool_steel`** | **`b5_tool_steel_measured`** | 2 |
| **`b5_tool_stone`** | **`b5_tool_stone_measured`** | 2 |

A pair of *constants* would be dead, not redundant — these all move, and move
**together, on every seed**.

### ★ Two of these are checks that cannot fail

**`b5_tool_steel` vs `b5_tool_steel_measured`.** The entire point of a
`_measured` twin is to verify the declared value independently. It has **never
once disagreed** across 48 seeds. A verification that cannot disagree verifies
nothing — it is a comment wearing a field's clothes.
[[a-field-cannot-calibrate-its-own-bound]]

**`b5_ch_ground_truth_tree_present` ≡ `b5_ch_engaged`.** A field named *ground
truth* exists to contradict the engine's own claim when the world disagrees. It
is element-identical to the claim on all 48 seeds. Either the world never
disagrees, or both read the same source — and the field as it stands cannot tell
those apart.

## Why this matters to #84 — with the direction of bias named

#84's concentration result is a **per-field tally**: movers observed against
fields available. Both defects push the same way:

- **50 immovable fields dilute the denominator** ⇒ the per-field mover rate
  looks *lower* than the instrument's real resolution, so any concentration
  against it looks *stronger*.
- **6 redundant columns inflate the numerator** ⇒ one real change is counted
  twice (three times for the `ch_` triple).

**Both biases overstate concentration.**

★ **I have NOT recomputed #84's statistic and I am not claiming its result is
wrong.** What is established is that the field list it was computed over
contains 47% columns that cannot move independently, and that correcting for
that can only move the answer in one direction. Recomputing needs the wave pair,
which is separately owed.

## Recommended, not applied

Do **not** delete the dead fields — a field that is constant across 48 seeds of
one configuration may be the one that moves when the configuration changes, and
several are deliberate invariants. The fix is at the **scorer**: collapse
identical columns before tallying, and report the independent-signal count beside
the field count so a wave's denominator states what it actually resolves.

The two "cannot fail" pairs are different: those are defects to fix at the
**producer**, because a verification field that never disagrees with its subject
is not measuring the subject.
