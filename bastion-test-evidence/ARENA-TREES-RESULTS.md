# ARENA TREES (F8-C1) — **RESULTS & ROW DISPOSITION**

Scored against `ARENA-TREES-PREREG.md` (`e2e650692e`), written **before any code was
changed**. Engine tip `54e7ce353c`.

---

## THE SCORE

| bar | verdict | evidence |
|---|---|---|
| **T1** · arena trees resolve | ✅ **PASS** | `chop designation resolved … trees=3`, live |
| **T2** · per-tree fell-sets | ⚠ **REGISTERED VALUE REFUTED** — purpose met | `cells=[14, 15, 13]`, not the predicted `[5, 6, 4]` |
| **T3** · the plant | ✅ **PASS** | injection off ⇒ `trees=0 cells=[]`, `reason="no_trees_rooted"` |
| **T4** · real worldgen unchanged | ✅ **PASS** | same binary, no arena env ⇒ `trees=4` |

**F8-C1 is CLOSED.** The finding was that the arena's trees cannot be chopped; they now
resolve, the plant restores the old behaviour exactly, and the real path is untouched.

## THE CAUSE, AT SOURCE

`get_area_trees` is **generative** — `structure_gen.iter()` plus a climate lottery — so
it can never propose a hand-placed trunk, and `tree_valid_at` would filter a synthetic
flat column even if it did. The arena's trees were invisible **before any block was
examined**. That is why the refusal read `no_trees_rooted` rather than an empty
flood-fill, and it is why the fix belongs at the candidate source: loosening the
*shared* path to admit them would have broken the very real-worldgen guarantee that
attributed the defect to the arena in the first place. T4 is what proves that did not
happen.

## ⚠ T2 — **MY REGISTERED NUMBER WAS WRONG**

I predicted `cells=[5, 6, 4]`, the trunk heights. The run gives **`[14, 15, 13]`** —
each exactly **height + 9**.

Read at the producer rather than fitted to the result: `resourced_feature_cells` paints
a **3×3 Leaves crown** at each trunk's top, and `is_tree` matches `Wood | Leaves`. So
the fell-set is trunk + 9 canopy cells. **I conflated *yield* with *fell-set size*** —
and the code comment I quoted in the pre-registration ("only Wood yields … the trunk
height IS the yield") is a statement about **yield**, which I misapplied to cells.

The bar's *purpose* — proving the counts are non-vacuous — is met, and met **more
strongly than a bare match would have been**: the three counts differ from one another
and track their trunk heights exactly, with a constant offset explained by the painter.
A prediction that had matched by luck would have told me less.

**The yield claim itself (15 Wood = 5+6+4) remains untested here.** It needs a chop
*completion* with a drop, which is F8-inclusion's work, not this row's. Registered as
open rather than quietly folded into T2.

## ★ AN OBSERVATION T4 HANDED OVER FREE

Real worldgen's four trees each report **exactly 2048** cells — `TREE_FELL_CELL_CAP`.
**Real trees saturate the cap**; their fell-sets are truncated, and the cap is doing
real work rather than sitting as a safety margin. Not a defect (the cap exists to bound
the flood-fill), but it means a real tree's fell-set is **not** its whole tree, and any
future yield accounting on real worldgen must not assume it is.

## WHAT I DECLINE TO CLAIM

- **Not** that chop *completes* on the arena. This row resolved a designation and its
  fell-set; jobs, drops and XP are F8-inclusion's axis.
- **Not** that T2's mechanism is proven beyond the read. It is verified at the producer
  and consistent across all three trees, which is why it is reported as an explained
  refutation rather than a pass.

## SUCCESSOR ROWS

1. **Arena chop *completion*** — with the fell-set now resolving, the 15-Wood yield
   prediction becomes testable end to end.
2. **The `TREE_FELL_CELL_CAP` saturation** — decide whether a truncated fell-set on real
   worldgen is intended, and if so, say so where the yield is computed.
