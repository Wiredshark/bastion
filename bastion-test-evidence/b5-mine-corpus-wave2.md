# b5 mine-completion corpus — WAVE 2 + COMBINED (waves 1–2)

- **Commit under test:** `5413915f71`, branch `bastion/apex-engine-integration`
- **Attestation:** all 3 delivering VMs reported `COMMIT=5413915f`
- **Seeds:** 121–264 requested, **72 delivered** (121–192). 3 VMs lost to the machine-image
  **create-rate limit** — self-inflicted: the standing rule is a ~10 min cooldown between
  fans and wave 2 launched immediately after wave 1. Not a test failure.
- **Cost / wall:** $0.53, 493s
- **Geometry:** 6 × e2-standard-32, 24 seeds/VM (128 GB ÷ 24 ≈ 5.3 GB/seed — the same
  per-seed memory ratio proven in wave 1; no OOM, 0 unparseable results)

## Wave 2 alone

| Class | Count | Rate |
|---|--:|--:|
| **TRUE mine-completion violations** | **21/72** | **29.2%** |
| Rescue fired (any) | 25/72 | 34.7% |
| Bucket C (mine 27/27 clean, rescue fired) | 12/72 | 16.7% |
| Build-fixture artifact (`any_needs_materials=false`) | 10/72 | 13.9% |

## COMBINED — 144 seeds (49–192)

| Class | Count | Rate |
|---|--:|--:|
| **TRUE mine-completion violations** | **45/144** | **31.3%** |
| Rescue-clause firings | 45/144 | 31.3% |
| Bucket C (proves the conflation) | 18/144 | 12.5% |
| Build-fixture artifact — **FALSE FAILURES, root-caused** | 15/144 | 10.4% |
| Fully clean | 66/144 | 45.8% |

Per-wave true-violation rate: 33.3% → 29.2%, **combined 31.3%**. Stable across independent
seed ranges; the headline is not a small-sample artifact.

### Failure modes, combined (n=45)

| Mode | mined | n | share of failures |
|---|--:|--:|--:|
| **ZERO MINED** | 0/27 | **11** | 24% |
| **ONE BLOCK SHORT** | 26/27 | **18** | 40% |
| **PARTIAL STALL** | 1–25/27 | 16 | 36% |

- ZERO MINED seeds: 51, 76, 92, 110, 123, 134, 146, 148, 153, 155, 182
- ONE BLOCK SHORT seeds: 49, 78, 83, 91, 95, 98, 106, 107, 109, 114, 126, 129, 133, 143,
  157, 167, 180, 190
- PARTIAL STALL seeds: 52, 54, 55, 61, 69, 74, 75, 97, 102, 108, 132, 160, 166, 174, 178, 183

**One-block-short holds at ~40% of all failures across both waves** (42% / 40%) — the single
most common failure mode in the game is the exact case the retired `>=26/27` tolerance was
written to permit.

**Zero-mined is 7.6% of ALL seeds** (11/144), not a rare edge: roughly one run in thirteen
assigns a colonist who mines, earns XP, and removes nothing.

### Rescue base rate

**45/144 = 31.3%** across both waves (27.8% → 34.7%). Against a design target of "a RARE
backstop". Gating `locomotion.2 == 0` would fail nearly a third of all runs, **18 of them
with perfectly mined 27/27 holes**. Confirms DECISIONS #37 (split out, report-only).
Filed as its own bug.

### Build-fixture artifact — resolved, and it was never a game bug

15/144 seeds carried `any_needs_materials=false`. **Root cause (5b): `build_ok_pos` /
`build_stall_pos` were never terraformed, unlike the mine and chop sites.** On seeds where
worldgen roots a tree at that exact column, `gz+1` is already filled, `job_wanted` rejects
the Build designation, and the whole build phase reports red. Seed-dependent, hence the
"mysterious cluster" appearance.

Notes:
- This **violates a rule b5's own source already states** ("test terraforms must fully
  determine geometry", cited in the slope-coverage phase and not applied here). Every phase
  is now being audited for the same under-determination.
- The orchestrator's proposed causal chain (chop failure → no material → build stall) was
  **wrong and was killed by reading the constants**: `BUILD_MATERIAL_ITEM`=stones is
  independent of `CHOP_DROP_ITEM`=wood. Correlation among JSON fields is not a causal chain.
- **Open lead, NOT closed by the terraform fix:** seeds 80/111/119 additionally show
  `log_sum=0` + `chop_cleared=false`. The build fix explains their `needs_materials`, not
  their chop failure. Tracked separately.

### Why the mine number is NOT vulnerable to that artifact class

Designation only creates a job for a **filled** cell, and the gate asserts `mine_jobs == 27`.
So on every counted seed, all 27 cells were verified solid before mining began; pre-existing
air cannot inflate `mine_blocks_mined` the way an untended tree deflated the build result.
The mine measurement is self-protecting; the build measurement was not.
