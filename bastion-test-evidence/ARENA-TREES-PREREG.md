# ARENA TREES (F8-C1) — **PRE-REGISTRATION**

**Written before any code is changed and before any data exists.**

## 1 · THE FINDING THIS ROW DISCHARGES

F8-C1, from the FOUNDING PRESET v1 row: **the arena's trees cannot be chopped, so §2's
chop claim is false for the arena.** It was attributed with a matched control on real
worldgen — chop works there — so the defect is the arena's, not chop's. It currently
blocks any arena-hosted chop acceptance.

## 2 · WHY, AT FILE::SYMBOL — read, not assumed

`bastion_chop::detect_trees` resolves a fell-set in four stages:

1. `sim.get_area_trees(min, max)` — candidates from `structure_gen.iter()` + a climate
   `make_forest_lottery`. **Purely generative: there is no registry to insert into.**
2. `sampler.get(...)` then `world::layer::tree::tree_valid_at(...)` — the engine's env
   filter.
3. seed search for a `Wood`/`Leaves` block near `col.alt`.
4. `tree_fell_set` — bounded flood-fill.

`bastion_flat_arena::resourced_feature_cells` paints trunks as **blocks at chunk
generation**. Stage 1 never proposes those columns, and even if it did, stage 2 would
filter a synthetic flat column. So the arena's trees are invisible **before any block is
ever looked at** — which is exactly why F8-C1's refusal was `no_trees_rooted` rather
than an empty flood-fill.

## 3 · THE DESIGN — augment the CANDIDATE SOURCE, share everything downstream

When `bastion_flat_arena::resourced()` is on, `detect_trees` gains the arena's trunk
columns as **additional candidates**, and stages 1–2 only (the oracle lottery and the
env filter) are bypassed for them — those two are meaningless in a synthetic world.
**Stages 3 and 4 stay shared and unmodified**: the seed search and the flood-fill run on
the arena's real blocks exactly as they do on real worldgen.

That distinction is the whole point. If I synthesised the fell-set directly I would be
testing my own arithmetic, not the chop path — the F8 defect again. The bar below is
only meaningful because the cells it counts come from `tree_fell_set` reading real
blocks.

## 4 · THE NUMBERS, DERIVED FROM `RESOURCED_TREES`

```rust
pub const RESOURCED_TREES: &[(Vec2<i32>, i32)] = &[
    (Vec2::new(18, 2), 5), (Vec2::new(22, -3), 6), (Vec2::new(25, 4), 4),
];
```

- **3 trees.** Trunks start at `FLAT_ARENA_Z = 400` (on the ground, not in it).
- **Trunk heights 5, 6, 4 ⇒ 15 Wood blocks total.** Only Wood yields (FR10 — leaves
  clear free), so **the trunk height IS the yield**.
- Offsets span `x ∈ [18, 25]`, `y ∈ [−3, 4]`. A designation over
  `x ∈ [16, 27], y ∈ [−5, 6]` encloses all three with margin: **12 × 12 = 144 tiles**,
  inside the path's own `64 × 64 = 4096` cap.
- `RESOURCED_CLEAR_RADIUS = 12` and the nearest trunk is at 18, so the cluster cannot
  contend with the founding footprint. No interaction to control for.

## 5 · THE BARS

### T1 · **THE ARENA'S TREES RESOLVE** — the direct refutation of F8-C1
- **PASS:** `bastion: chop designation resolved region=… trees=3` on the arena, live.
- **FAIL:** `bastion: chop designation refused reason="no_trees_rooted"` — the exact
  line F8-C1 recorded. *This bar's failure mode is the finding's current behaviour*,
  which is the cleanest possible before/after.

### T2 · **EACH TREE IS ITS OWN DESIGNATION, WITH A REAL FELL-SET**
- **PASS:** **3** per-tree echo designations, and each tree's cell count matches its
  trunk height — **5, 6, 4** — because the flood-fill read the arena's actual blocks.
- Counting only `trees=3` would pass even if every fell-set were empty. **The cell
  counts are what make T1 non-vacuous.**

### T3 · **THE PLANT** — oracle-registration disabled
- Disable the arena candidate injection (one condition), leaving everything else.
- **REQUIRED RED:** `trees=0` and `reason="no_trees_rooted"` returns — i.e. the code
  falls back to exactly F8-C1's recorded behaviour.
- **CONTROL:** the restored build on the same arena, same script, same profile.

### T4 · **REAL WORLDGEN IS UNCHANGED** — the non-regression, and it is not optional
- The injection is gated on `resourced()`. A change that made arena trees work by
  loosening the *shared* path would break the real-worldgen guarantee F8-C1's control
  established.
- **PASS:** the F8-C1 real-worldgen chop control still resolves trees with **no arena
  env set**, on the same binary that passes T1.
- Without T4, T1 could be bought by breaking chop everywhere and nobody would see it.

## 6 · WHAT I WILL **NOT** DO AT SCORING TIME

1. **I will not count `trees=3` as sufficient.** Without T2's per-tree cell counts the
   number 3 could come from three empty resolutions.
2. **I will not skip T4.** A green T1 with a broken real path is a worse outcome than
   the finding I started with.
3. **I will not bypass stages 3–4** to make the numbers come out. If the flood-fill
   cannot read the arena's blocks, that is a **second finding** and I report it rather
   than routing around it.
4. **I will not adjust `RESOURCED_TREES`, the heights, or the designation rectangle
   after seeing a result.** The expected yield is **15** because the table says 5+6+4,
   fixed here, before the build.
5. **I will not claim the row closes F8-C1 unless T1, T2, T3 and T4 all hold.** A
   partial result is reported as partial, and F8-C1 stays open in that case.
