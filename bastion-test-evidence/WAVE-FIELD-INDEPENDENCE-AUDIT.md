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

### ★ I called two of these "checks that cannot fail". Reading the producer, they are not

**`b5_tool_steel` / `_measured` — ALREADY DOCUMENTED, not a discovery.** The
harness says so at the assignment, in full:

> *"`b5_tool_stone_measured`/`_steel_measured` (added earlier, additive-only, as
> a workaround before this mutating window was eligible) now carry the **SAME
> information** as these — left in place since removing them isn't part of this
> registered delta, **not because they're still doing independent work**."*

`let tl_stone: Option<f32> = tl_stone_raw;` — identical **by assignment**. My
measurement reproduced a residual its authors had already registered in a
comment, which is the good case: the corpus agreed with the code.
[[sufficiency-claims-must-name-their-case]]

**`b5_ch_engaged` ≡ `b5_ch_ground_truth_tree_present` — derived, not
independent.** `ch_engaged` is *computed from* that field via `ch_oracle_class`
(`match (ch_trees >= 1, ch_ground_truth_tree_present, ch_scan_incomplete)`), and
`b5_ch_scan_incomplete` is constant `false` corpus-wide. Their agreement is
construction, not evidence of a broken oracle. **Withdrawn.**

### What is NOT explained by reading the producer

| tie | status |
|---|---|
| `b5_ch_cancel_clean` ≡ the other two | `ch_cancel_clean` is a **separate** AABB predicate (`main.rs:4433`), not derived from the oracle class — the identity is unexplained |
| `b5_ch_jobs` ≡ `b5_ch_trees` | 10 distinct values, always equal — unexplained |
| `b5_any_needs_materials` ≡ `b5_build_placed` | semantically unrelated names — unexplained |

Those three are worth a producer read. The other four are accounted for.

## ★ Is a tie surprising? Testing my own claim against multiple comparisons

Two booleans matching across 48 seeds is cheap evidence if both are lopsided,
and there are **7,021** field pairs, so ties are expected by chance alone.
Bonferroni threshold: **0.05 / 7021 = 7.1e-6**.

| tie | balance | p(match by chance) | survives correction |
|---|---|---|---|
| `b5_ch_jobs` ≡ `b5_ch_trees` | 10 values | **2.2e-38** | ✅ |
| `b5_any_needs_materials` ≡ `b5_build_placed` | 39/9 | **6.0e-10** | ✅ |
| `b5_ch_cancel_clean` ≡ `ch_engaged` ≡ `ground_truth` | 41/7 | **1.4e-8** | ✅ |
| `b5_tool_steel` ≡ `_measured` | **47/1** | **0.021** | ❌ |
| `b5_tool_stone` ≡ `_measured` | **47/1** | **0.021** | ❌ |

★ At 7,021 pairs, ~147 ties at p=0.021 are expected **by chance**. So the tool
pairs are statistically unremarkable — and they are nonetheless genuinely
redundant, because the code says `let tl_stone: Option<f32> = tl_stone_raw;`.

**The statistics were weak exactly where the code was decisive, and decisive
exactly where the code explains nothing.** Neither instrument alone would have
got all seven right, and I would have trusted the wrong one: I reached for the
p-values only after the code had already overturned two of my three headline
claims.

Every tie is therefore either statistically decisive **or** code-confirmed, and
the independent-signal count of **63** stands.

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
