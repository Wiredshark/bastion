# B5.6b-2 gate — test results

Branch `bastion/block-B5.6b-2` (off `72907ee641`). Machine state: quiet
(no game client, no concurrent builds during scenario runs; the asset-lab
session edits docs only).

## 1. Unit tests (`cargo test -p veloren-common --lib bastion`)

**PASS — 8/8** (6 Region tests + the two new schema guards:
`z_extent_default_preserves_legacy_paint_depth`,
`purpose_enum_is_the_canonical_eight`).

## 2. `--b4-scenario` (seed 1337)

**PASS.** `{"b4_all_idle_after_cancel":true,"b4_arrived_enabled":4,
"b4_cancel_cleared_jobs":true,"b4_claims_always_distinct":true,
"b4_colonists_loaded":5,"b4_jobs_placed":20,"b4_priority_honored":true,
"b4_soak_avg_tick_ms":4.21,"b4_unreachable_marked":true}`

## 3. `--b5-scenario` (seed 1337) — includes NEW phase 7.5 slope coverage

**PASS.** All B5/B5.5-era invariants held (27 mine jobs, conservation
`stone_sum==27` in ≤2 entities, stall untouched, XP granted, 3.9ms avg
tick) AND the new slope-coverage asserts all landed:

- `b5_slope_jobs_total: 72` — surface path covered all 24 staircase
  columns × 3 levels.
- `b5_slope_columns_ok: true` — every column's resolved surface matched
  the terraformed truth AND carried exactly its top-3 jobs.
- `b5_slope_bounds_ok: true` — echoed bounds = the tight AABB
  (footprint × [gz−2, gz+7]).
- `b5_slope_cancel_clean: true` — cancelling exactly the echoed bounds
  left 0 jobs (echo-bounds invariant end-to-end).
- `b5_slope_legacy_jobs: 45` — the legacy flat path on the same staircase
  under-covers exactly as computed by hand (2 columns get ZERO). This is
  the B5.MINE-COVERAGE bug, reproduced and kept as a regression witness.

**B5.MINE-COVERAGE: CLOSED** (root cause: client-side flat pre-expansion;
fix: per-column surface-relative resolution).

## 4. `--b55-scenario` (seed 1337)

**PASS.** Erase semantics + conservation unchanged by the wire/placement
refactor: partial erase released 18/18 half-jobs with 0 orphans, whole-zone
delete → board 0 + all idle, 200-block slab conserved exactly
(`stone_sum 200` in 27 entities), 4.4ms avg tick.

## 5. Vanilla flagless harness boot + soak

**PASS.** 1000 ticks clean, 9.4× real-time, 2355 rtsim NPCs / 204 sites,
`colonist_count: 0` (no bastion machinery active flagless). Compile checks
green ×3 across the block (harness+server+client, voxygen ×2).

## 6. Ben's in-game verify (6-deep mine on a slope; scroll + stepper UX)

**BATCHED** per the architect's final routing protocol (tag on green
headless gate; eyeball items consolidated). The TEST LIST sent to the
architect with the tag ping covers: the 6-deep slope mine (rings countable,
label depth), scroll-vs-stepper sync + kind reset, scroll-only-during-drag,
mid-excavation slice reading (+ the Z-SLICE ADEQUACY verdict the architect
asked Ben for), flat-ground regression, erase/radial regression, vanilla
untouched.
