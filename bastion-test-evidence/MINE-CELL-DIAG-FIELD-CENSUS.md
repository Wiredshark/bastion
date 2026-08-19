# `b5_mine_cell_diag` — FIELD CENSUS, and a mover-count inflator

#84's remaining open item: *"`b5_mine_cell_diag`'s mover is **content**,
untouched."* Before chasing what moves, census what the field can *contain*.
Banked wave34 corpus, 48 seeds, no spend.

## Population

| | |
|---|---|
| seeds with the field populated | **8 / 48** |
| empty (`[]`) | 40 / 48 |
| pooled elements | **126** |
| fields per element | **18** |

★ My first parse said **0 of 48** — a non-greedy `\[.*?\]` truncated every
populated array at the first inner `]`, producing invalid JSON that fell into the
"empty" bucket. The raw text says `[{"benched_until_tick":null…` on 8 seeds. A
regex that cannot represent nesting is not a reader for nested data.

## Structure — three sizes, and they are not arbitrary

Entries per seed take exactly **3, 6, or 27**, and **every seed spans exactly 3
z-levels**:

| entries | z-levels | ⇒ columns | seeds |
|---|---|---|---|
| 3 | 3 | **1** | s32, s39 |
| 6 | 3 | **2** | s16, s23 |
| 27 | 3 | **9** (a 3×3 footprint) | s3, s11, s29, s37 |

So the diag is *columns × 3 layers*, and the three sizes are three mining
footprints — quantized because the footprint is, not because the data is noisy.
[[quantized-outcomes-mean-a-hidden-categorical-input]]

## ★ Four of eighteen fields carry no information

| field | finding |
|---|---|
| `benched_until_tick` | **`null` in 126/126** |
| `needs_materials` | **`false` in 126/126** |
| `progress` | **`0.0` in 126/126** |
| `is_column_frontier` | **element-wise IDENTICAL to `is_top_layer`, 126/126** |

**The duplicate pair is a mover-count inflator.** A wave diff compares fields
independently, so any real change in `is_top_layer` is reported *twice* — one
content change, two movers. #84's concentration counts are drawn from exactly
this kind of per-field tally, so at least one field-pair in this diag cannot move
alone.

★ Counts matching is not identity — 9/27 and 9/27 could be different elements.
This was checked **element-wise**, 126/126, which is what licenses the word.

## Five fields are per-SEED constants, not per-cell

`benched_until_tick`, `blocked_by`, `blocked_sources`, `needs_materials`,
`progress` never vary *within* a seed. `blocked_by` takes 8 distinct values
across 8 seeds — it describes the **run**, not the **cell**, and is repeated 126
times where 8 would do.

So the genuine per-cell payload is **12 of 18 fields**, and the movable surface
is smaller than the field list suggests.

## What moves

`unreachable` is the field that actually varies within a seed (0–18 of n across
the eight), alongside `pos`, `claimant`, the starvation counters, and the
cell-open/timeout counters.

## Not concluded here

This is a census of one wave. It does **not** identify #84's content mover —
that needs a second wave to diff against. What it does is size the surface the
mover must live in, and remove one way the count could have been wrong.
