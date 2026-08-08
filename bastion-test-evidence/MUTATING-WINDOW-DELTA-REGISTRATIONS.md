# MUTATING WINDOW — PRE-REGISTERED PER-SEED DELTAS (#55 ENTRY TICKET)

**Baseline: `wave26_ROWA_d5b56d1c79_FULL.json`, 48 seeds, 93 keys.** Every delta
below is **derived from the baseline on disk**, never transcribed — per #57.

> **#55's rule: a MUTATING item may join a window ONLY with an exact
> pre-registered per-seed delta for every field it touches.** Four items follow.
> ★ **Each states what MOVES, what HOLDS, and what would FALSIFY it.**

## REG-1 — THE TOOL-FACTOR SENTINEL (seed 66)

**Fix:** `unwrap_or(0.0)` → `Option` + range-assert. *(`0.0` is below the
metric's own documented floor of `1.0` — an impossible reading surviving every
presence check.)*

**MOVES — exactly 3 fields, exactly 1 seed:**

| field | seed 66 now | after |
|---|---|---|
| `b5_tool_stone` | `0.0` | ★ `null` |
| `b5_tool_steel` | `0.0` | ★ `null` |
| `b5_tool_ok` | `false` | ★ **`null`, NOT `true`** |

★★★ **`tool_ok` is the trap.** It is `false` **because** the sentinel poisoned it.
**Flipping it to `true` would assert the tools were fine — which the guard never
established.** ★ **The honest post-fix value is "unknown."** *(Behavioral-claims
standard: a field may assert only what its guard establishes.)*

**HOLDS:** all 47 other seeds on all three fields (`1.5` / `2.0` / `true`), and
**every other field on seed 66.**

★ **FALSIFIER:** any movement on a seed other than 66, or `tool_ok → true`.
**Verify with `--expect-move` on a DERIVED seed set** (`{s : tool_stone < 1.0}`),
never a transcribed `66`.

## REG-2 — `route_next_idx_pinned` → THREE-WAY

**Fix:** the summary's `null` currently carries **two different facts**. Split
into `too_few_samples` / `no_route_present` / `compared`.

**MOVES — 89 probe results, and the split is EXACT:**

| current | count | becomes | seeds |
|---|--:|---|---|
| `null` | **79** | `too_few_samples` | 52, 54, 61, 66, 71, 90 |
| `null` | ★ **10** | ★ `no_route_present` | ★ **52, 54, 92** |
| `true` | 8 | `compared: pinned` | 54, 71, 78, 80 |
| `false` | 6 | `compared: advancing` | 61, 71, 85, 90 |

> ★★★★★ **TEN PROBE RESULTS ACROSS THREE SEEDS CURRENTLY REPORT `null` WHEN THE
> REAL FACT IS "AT SOME POINT THERE WAS NO ROUTE AT ALL."** ★ **That is the
> astar-reset population** — the substantive case the summary has been hiding.
> **Seed 92's chop probe is all four samples no-route.**

**HOLDS:** `timeout_route_states` (the raw list) is **untouched** — it already
distinguishes these; only the **summary** changes. **No other field moves.**

★ **FALSIFIER:** counts not summing to 89, or any seed outside the four sets
above changing value.

## REG-3 — `b5_55_*` RENAMES

**Pure renames.** ★ **The delta is mechanically complete without knowing the
target names**, because a rename's invariant is exact:

    for every seed:  value(new_key) == value(old_key)
                     old_key ABSENT, new_key PRESENT
                     no third field moves

**Current values, all 48 seeds (identical in wave25 and wave26):**

| field | value |
|---|---|
| `b5_55_blocked_by` | `null` × 48 |
| `b5_55_clears_on_cancel` | `true` × 48 |
| `b5_55_names_blocker` | ★ `false` × 48 |
| `b5_55_notified_once` | ★ `false` × 48 |
| `b5_55_diag` | one object × 47, `null` × 1 |

★★ **`names_blocker` and `notified_once` are FALSE on all 48 seeds even AFTER Row
A landed** — consistent with the known finding that these probe **fixed cells**,
so the store is *"reported at the wrong coordinates."* ★ **The rename must not be
read as fixing that**; it is a naming change over an already-known defect.

★ **FALSIFIER:** any value change at all. **A rename that moves a value is not a
rename.**

## REG-4 — CONSTANT-DIAG RENAMES (`build_ok_jobs`, `build_stall_jobs`, `build_stall_untouched`)

**Same pure-rename invariant as REG-3.** Current: `1` / `1` / `true` on **all 48
seeds**, passing and failing alike.

★ **These are fixture descriptors dressed as diagnostics** — the rename makes the
name match the content. ★★ **Register explicitly that the rename does NOT make
them discriminate**, so no later reader mistakes the new name for new
information.

## ★★★★★★ BONUS FINDING — 38 OF 93 FIELDS (41%) NEVER VARY

**Derived while building REG-4.** Constant across all 48 seeds:

- ★ **Fixture constants** — `mine_jobs 27`, `flat_total 108`, `slope_jobs_total 72`
- **Always-true invariants** — `flat_bounds_ok`, `slope_bounds_ok`, `hill_total_ok`
- ★★★ **DEAD INSTRUMENTS** — `access_plan_self_rescue_emissions ALWAYS 0`
  *(this morning's 100%-refusal finding, visible as a constant)*,
  `proactive_descent_calls ALWAYS 0`, `ch_scan_incomplete* ALWAYS 0`

> ★★★ **41% of the schema carries ZERO information about any seed.** Not
> necessarily wrong — an invariant that always holds is a real assertion — **but
> a constant field cannot participate in ANY discrimination, and four of them are
> instruments that have never once fired.**

★ **Suggested standing addition to `derived.py`:** report the constant-field
count per wave, and **flag any field that becomes constant** *(a diagnostic that
stops varying has usually stopped working)*. **Not proposed for this window** —
it is additive and belongs with the other tooling.

## ENTRY-TICKET STATUS

| item | delta registered | falsifier | eligible |
|---|---|---|---|
| REG-1 sentinel | ★ exact (3 fields, 1 seed) | ✅ | ★ **YES** |
| REG-2 three-way | ★ exact (79/10/8/6) | ✅ | ★ **YES** |
| REG-3 `b5_55` renames | ★ invariant-complete | ✅ | ★ **YES** |
| REG-4 constant renames | ★ invariant-complete | ✅ | ★ **YES** |

> **The mutating window moves from PARKED to ELIGIBLE.** ★ It can follow the
> additive diag window immediately, **with its paperwork already done rather than
> started on arrival.**
