# b5 mine-completion corpus — WAVE 1

- **Commit under test:** `5413915f71` (MINE-COMPLETION-INVARIANT), branch `bastion/apex-engine-integration`
- **Attestation:** all 6 delivering VMs reported `COMMIT=5413915f` (per-VM stale-binary guard passed)
- **Seeds:** 49–120 requested, **72 delivered** (5 VMs lost to the `IN_USE_ADDRESSES`=8 quota, not to test failure)
- **Cost / wall:** $0.74, 760s total including per-VM build
- **Classifier:** JSON fields, NOT exit code — b5's `pass` is a ~40-clause conjunction

> **Why the raw logs are not attached:** `vm-pool.sh` sets `OUT=/tmp/bastion-pool` and
> `rm -f "$OUT"/*.log` at startup, so the next wave destroys the previous wave's logs.
> Extracted results are persisted here per wave from now on.

## Headline

| Class | Count | Rate |
|---|--:|--:|
| **TRUE mine-completion violations (A+B)** | **24/72** | **33.3%** |
| Fully clean | 37/72 | 51.4% |
| Rescue fired (any) | 20/72 | 27.8% |
| Other clause red, mine clean | 5/72 | 6.9% |

**Conflation quantified (Opus's review finding, empirically confirmed):** counting the
failsafe-rescue clause as a mine failure gives 30/72 (41.7%); the true mine-completion
rate is 24/72 (33.3%). The 6-seed gap is bucket C below — holes mined 100% clean where a
rescue happened to fire. The local 19/48 (39.6%) figure carries the same over-count.

## Failure-mode decomposition

| Mode | mined | Seeds | n |
|---|--:|---|--:|
| **1. ZERO MINED** | 0/27 | 51, 76, 92, 110 | 4 |
| **2. ONE BLOCK SHORT** | 26/27 | 49, 78, 83, 91, 95, 98, 106, 107, 109, 114 | 10 |
| **3. PARTIAL STALL** | 15–25/27 | 52, 54, 55, 61, 69, 74, 75, 97, 102, 108 | 10 |

Full shortfall distribution: `0:4  15:1  19:2  22:2  23:1  24:2  25:2  26:10`

### Mode 1 detail — work with no effect
All four: `mine_jobs=27` (designation succeeded), `chop_cleared=true` (scenario otherwise
healthy), **`any_mining_xp=true`** (a colonist was assigned, mined, and gained XP),
`stone_sum=0`, `mine_blocks_mined=0`. A colonist performed mining work and removed zero
blocks.

Sub-signature: seeds 51/76/92 carry large `no_progress_ticks` (4736/8019/1977), but seed
**110 shows [104, 0, 0]** — low no-progress, no timeouts, no teleports, still zero mined.
Seed 110 is probably a distinct mode; tracked as its own lead.

## Bucket C — proof the conflation was real

Mine 100% clear (`27/27`, `mine_cleared=true`) yet a rescue fired:

| Seed | teleports |
|--:|--:|
| 56 | 1 |
| 62 | 1 |
| 66 | 4 |
| 85 | 3 |
| 96 | 2 |
| 104 | 4 |

## Discriminators

| Signal | Failing | Clean |
|---|--:|--:|
| `no_progress_ticks` (median) | **3485** | **600** |
| teleports fired | 14/24 (58%) | 6/48 (12.5%) |

The clean median of 600 is exactly the zero-input soak length — i.e. clean runs have
essentially **no unexplained** no-progress; all of it is the deliberate idle. Failing runs
run ~6x that. Strong, already-instrumented separator.

## Rescue base rate — answers the rare-vs-never question

**20/72 = 27.8%.** The failsafe backstop's own success criterion (`bastion_jobs.rs:13992`)
is reverting to "a RARE backstop." It fires on more than a quarter of seeds. Gating
`locomotion.2 == 0` would have failed ~28% of runs, including 6 with perfectly mined holes.
Confirms the ruling to split it out and hold it report-only (DECISIONS #37). **Filed as its
own bug: backstop firing rate is ~28% against a design target of rare.**

## Build-stall cluster (corroborates 5b's "other 18")

5 seeds, every one with `any_needs_materials=false`: **59, 80, 103, 111, 119**.

Seeds 80/111/119 additionally show `log_sum=0` + `chop_cleared=false`. Suggested causal
direction: chop failed → no material → build could not place → `needs_materials` never set.
If that holds, the build-stall symptom is a DOWNSTREAM effect of a chop failure rather than
a build-arbitration bug, and several of the 18 collapse into one cause.

## Fully clean seeds (37)

50, 53, 57, 58, 60, 63, 64, 65, 67, 68, 70, 71, 72, 73, 77, 79, 81, 82, 84, 86, 87, 88, 89,
90, 93, 94, 99, 100, 101, 105, 112, 113, 115, 116, 117, 118, 120
