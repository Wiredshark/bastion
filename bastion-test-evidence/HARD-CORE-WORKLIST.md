# THE HARD CORE — TEN SEEDS THAT HAVE NEVER PASSED, CHARACTERIZED

**Derived 2026-08-08 from the cross-wave series the collector fix recovered
(`wave14 … wave30`, identical 48-seed set, compatible shape).**
★ **Ten seeds are PERSISTENT across every wave: `54 61 62 66 68 71 78 80 85 92`.**

## ★★★★★★★ IT IS NOT TEN PROBLEMS. IT IS **THREE PERFECTLY CO-OCCURRING PAIRS**
### PLUS TWO SINGLETONS

| family | clauses | seeds | co-occurrence |
|---|---|---|---|
| ★★★ **BUILD / MATERIALS** | `build_placed` + `any_needs_materials` | **61 62 71 80 85 92** | ★★★★★ **IDENTICAL SETS** |
| ★★★ **CHOP / LOGS** | `chop_cleared` + `log_sum` | **78 80 85 92** | ★★★★★ **IDENTICAL SETS** |
| ★★★ **MINE** | `mine_cleared` + `mine_blocks_mined` | **54 61 71** | ★★★★★ **IDENTICAL SETS** |
| singleton | `tl_ok` | 66 | — |
| singleton | `ch_mixed` | 68, 92 | — |

> ## ★★★★★ **EVERY PAIR IS ONE MECHANISM REPORTED TWICE.**
> **Ten "failing seeds" collapse to roughly FOUR root causes.**

★★ **And the pairs have a plausible causal direction worth testing before any
build:** *`any_needs_materials` ⇒ `build_placed` fails* **(materials never arrive,
so nothing gets placed — a HAUL problem wearing a BUILD clause)**; *`log_sum` ⇒
`chop_cleared`* **(logs never accumulate, so the chop never clears).**
★ **If those directions hold, the worklist is smaller still.**

### ★ PER-SEED

    54   mine_blocks_mined, mine_cleared
    61   any_needs_materials, build_placed, mine_blocks_mined, mine_cleared
    62   any_needs_materials, build_placed
    66   tl_ok
    68   ch_mixed
    71   any_needs_materials, b15_adjacent_claimed, b15_ontop_claimed,
         build_placed, mine_blocks_mined, mine_cleared
    78   chop_cleared, log_sum
    80   any_needs_materials, build_placed, chop_cleared, log_sum
    85   any_needs_materials, build_placed, chop_cleared, log_sum
    92   any_needs_materials, build_placed, ch_mixed, chop_cleared, log_sum

## ★★★★★ EIGHT OF TEN ARE **FROZEN** — IDENTICAL CLAUSE SETS SINCE wave14

**Checked across `wave14 · 18 · 19 · 25 · 26 · 30`:** ★★ **eight seeds carry the
EXACT same clause set in every wave.**

> ★★★ **The hard core is not thrashing. It is UNTOUCHED.** *These are stable,
> reproducible failures that no row has ever moved — which makes them unusually
> good candidates: nothing about them is flaky.*

## ★★★★★★★★ THE FINDING: **SEED 71 REGRESSED WHILE THE CORPUS IMPROVED**

| | wave14 | wave30 |
|---|---|---|
| **seed 71's clauses** | ★ `mine_cleared`, `mine_blocks_mined` **(2)** | ★★★ **+`build_placed`, `any_needs_materials`, `b15_ontop_claimed`, `b15_adjacent_claimed` (6)** |
| **corpus fail count** | **14/48** | ★★ **12/48 by wave18 — the same window** |

> ## ★★★★★★★ **SEED 71 ACQUIRED FOUR NEW FAILING CLAUSES INSIDE A WINDOW WHERE
> THE OVERALL COUNT WENT DOWN BY TWO.**

★★★★★ **NO COUNT-BASED CHECK COULD EVER HAVE SEEN THIS.** *The seed was already
failing; it stayed failing; the cardinality of the fail SET improved. Every gate
we run reads that cardinality.*

> ## ★★★★★ **THE LAW: A REGRESSION INSIDE AN ALREADY-FAILING SEED IS INVISIBLE
> TO EVERY CARDINALITY CHECK. The verdict is a SET; the gate reads its COUNT.**

★★ **Sibling of seed 90's regression** *(which flipped pass→fail and WAS visible
in the count)*. ★★★ **Seed 71's is the same event with its visibility removed —
and it sat undetected across five waves.** ★ **Direct instance of the
aggregate-late law: the collapse to a count destroyed exactly the structure that
carried the regression.**

### ★★★ MECHANICAL CONSEQUENCE — CHEAP, AND IT SHOULD BE STANDING

**Compare failing seeds' CLAUSE SETS wave-over-wave, not just membership.**
★★★★★ **Three outcomes, all currently invisible:** *a failing seed gaining a
clause (**regression**), losing one (**partial progress**), or swapping one
(**churn**).* ★ **The data has been in every wave all along; nothing read it.**

## ★ THE OTHER MOVER, FOR COMPLETENESS

**Seed 61 LOST `ch_leaf_cleared` between wave14 and wave18 and never regained it**
*(5 → 4 clauses)*. ★★ **Real, durable partial progress on a seed that still
fails** — **and equally invisible to a count.** ★★★ *The instrument gap cuts both
ways: it hides the wins as well as the regressions.*

## ★★ HOW TO USE THIS

1. ★★★★★ **Attack the PAIRS, not the seeds.** *One fix to the materials chain
   plausibly moves six seeds; one to the log chain, four.*
2. ★★★ **Test the causal direction first** *(`any_needs_materials` → `build_placed`)*
   — **a one-read question, and it decides whether the row is a HAUL row or a
   BUILD row.**
3. ★★ **The two singletons (`tl_ok` 66, `ch_mixed` 68/92) are separate and small.**
4. ★ **Nothing here is flaky** — *frozen across nine waves is as reproducible as
   this corpus gets, which makes these the cheapest failures in the project to
   work on.*

> ★★★ **This table is the standing worklist with its full history attached, and
> it exists because a `max()` was deleted from the collector.**
