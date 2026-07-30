# b5 — WAVE 3: before/after on IDENTICAL seeds

- **Before:** wave 1, commit `5413915f71`, seeds 49–120
- **After:** wave 3, commit **`b59ac664`** (attested on all 3 delivering VMs), seeds 49–120
- Same seed range, same geometry, 690s, $0.81. 3 VMs lost to create-rate limit.

This is a true before/after — identical seeds, not a fresh sample.

## Headline

| Class | Wave 1 (before) | Wave 3 (after) |
|---|--:|--:|
| TRUE mine-completion violations | 24/72 (33.3%) | **24/72 (33.3%)** |
| Rescue-clause firings | 20/72 | 20/72 |
| Other clause red | 5/72 | **4/72** |
| Fully clean | 37/72 | 37/72 |

**Mine bug unchanged, exactly as expected — nothing has been fixed for it yet.**
Wave 3 is the clean BASELINE against which the eventual mine fix gets measured.

## The build-fixture fix is CONFIRMED at scale

`build_stall_jobs == 0` — the symptom the terraform fix targeted — **has vanished
entirely** from the corpus (wave 1: seeds 103, 119; wave 3: none).

| Seed | Wave 1 failing clauses | Wave 3 failing clauses | Verdict |
|--:|---|---|---|
| 59 | build_placed, any_needs_materials | *(none)* | **fully clean** |
| 103 | build_ok_jobs=0, build_stall_jobs=0, build_stall_untouched, any_needs_materials | build_placed, any_needs_materials | designation defects gone |
| 111 | log_sum=0, chop_cleared, build_placed, any_needs_materials | **log_sum=0, chop_cleared** | build symptoms gone |
| 119 | build_stall_jobs=0, log_sum=0, chop_cleared, build_placed, build_stall_untouched, any_needs_materials | **log_sum=0, chop_cleared** | build symptoms gone |
| 80 | log_sum=0, chop_cleared, build_placed, any_needs_materials | log_sum=0, chop_cleared, build_placed, any_needs_materials | unchanged |

**Seeds 111 and 119 independently confirm the chop lead is real and separate.** Every
build-related clause cleared; `log_sum=0` + `chop_cleared=false` remained, untouched. That is
the prediction 5b made, tested on its own terms and held.

Residual `build_placed` + `any_needs_materials` on 80 and 103 matches the throughput residual
5b flagged locally (seeds 5/43/44): the designation exists, it just never gets built inside
the 180-window budget. Separate class, tracked.

## OPEN QUESTION — per-seed results moved a lot; determinism control running

The aggregate held at exactly 24/72, but **individual seeds changed substantially**:

| Seed | Before | After |
|--:|---|---|
| 107 | 26/27, 3 teleports (fail) | **27/27 clean**, 4 teleports |
| 113 | clean | **26/27** (fail) |
| 108 | 19/27 | **2/27** |
| 52 | 15/27 | **8/27** |
| 54 | 19/27 | 24/27 |
| 97 | 23/27 | 20/27 |
| 61 | 24/27, 1 tp | 26/27, 2 tp |
| 74 | 25/27 | 26/27 |
| 102 | 22/27 | 23/27 |
| 98 | 26/27, 0 tp | 26/27, **1 tp** |

23 of 24 failing seeds are shared between runs, so this is not noise in the headline — but
the per-seed movement has **two candidate explanations that this data cannot separate**:

- **(a) legitimate perturbation by the commit** — the terraform fix writes blocks into the
  world. If that staging happens during setup (before the mine phase), it can shift colonist
  pathing and scheduling and therefore change mine outcomes. Note the *measurement* order
  cannot explain it: `mine_blocks_mined` is computed BEFORE the build-stall phase, so a
  terraform performed at build time could not affect it. Only a setup-time terraform can.
- **(b) run-to-run nondeterminism in b5** — which, for a determinism program, would be a far
  bigger finding than the mine bug.

**Control launched (wave 4): same commit `b59ac664`, same seeds 49–120, re-run.**
Identical results ⇒ deterministic, and (a) is the explanation. Any divergence ⇒ (b), and it
becomes the top priority. Do not draw conclusions from per-seed deltas until this lands.
