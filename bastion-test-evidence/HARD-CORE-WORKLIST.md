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

## ★★★★★★★★ THE DIRECTION CLAIM IS **DEAD** — AND IT WAS DEAD BEFORE I WROTE IT

**I proposed: *"`any_needs_materials` ⇒ `build_placed` fails — materials never
arrive, so nothing gets placed. A HAUL problem wearing a BUILD clause."***

★★★ **Fable pre-killed it as the THIRD resurrection of the materials story, citing
my own prior commits. ★ I then confirmed it independently on wave 30:**

| seed class | `any_needs_materials` | `build_placed` | `stone_sum` |
|---|---|---|---|
| **all 40 passing** | `True` | `True` | **27** |
| ★★★★★ **all 6 core** *(61 62 71 80 85 92)* | ★★★ **`False`** | ★★★ **`False`** | **27** *(62 80 85 92)*, 26, 5 |

> ## ★★★★★★★ **THE CELL MY STORY REQUIRES — `build_placed=false` WITH
> `needs_materials=TRUE` — IS EMPTY. ZERO OF 48.**

★★★★★ **The failing seeds are `false/false`: BUILD NEVER PROGRESSED FAR ENOUGH TO
REQUEST MATERIALS.** ★★ **`any_needs_materials=False` does not mean "materials
missing" — it means NOTHING EVER ASKED.**

★★★ **And supply is provably fine: `stone_sum=27` on four of the six core seeds —
IDENTICAL to every passing seed.** ★ *My own banked line, from August: "build's
failures are upstream of materials entirely."*

> ★★★★★ **SO IT IS A BUILD PROBLEM, AND `any_needs_materials` IS THE CLAUSE
> WEARING BORROWED CLOTHES. I had the direction exactly backwards.**

### ★★★ WHAT THE IDENTICAL SETS ACTUALLY MEAN

**The co-occurrence is real — but it is a DEFINITIONAL CONSEQUENT, not a causal
chain.** ★★★★★ **`any_needs_materials` is downstream of `build_placed` BY
CONSTRUCTION** *(no build progress ⇒ no material request)*, **and chop/logs is the
same intra-subsystem coupling — logs come from chopping.**

★★ **This is the dependent-pairs table rediscovered, banked 2026-08-04.** ★ **The
family count STANDS — the core really is ~4 roots — but the collapse is
definitional, which is a weaker and more honest claim than a causal one.**

> ## ★★★★★ **MECHANICAL ANTIDOTE: "HAS THIS DIRECTION BEEN KILLED BEFORE?" IS A
> GREP OF `DECISIONS`, AND IT BELONGS IN THE CHARACTERIZATION STEP.**
> ★★★ **Perfect co-occurrence invites a causal story and is equally consistent
> with one field being DEFINED in terms of the other. Check which, before
> proposing a row.**

★ **THE ONE LEAD THAT SURVIVES:** ★★★ **seed 71 has `stone_sum = 5` against
everyone else's 27** — *a genuine outlier, on the same seed that carries the
invisible regression.* ★★ **Recorded as a lead, NOT a direction.**

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

1. ★★★★★ **Attack the PAIRS, not the seeds.** *One fix to the BUILD family
   plausibly moves six seeds; one to the chop family, four.*
2. ★★★★★★★ **The build family's row is: WHY DOES BUILD NEVER START?** *Not "why
   don't materials arrive" — that question is answered and the answer is that
   nothing ever asked.* ★★ **`build_placed=false` with full stone in the
   stockpile is the whole finding, and it is upstream of every material
   mechanism.**
3. ★★ **The two singletons (`tl_ok` 66, `ch_mixed` 68/92) are separate and small.**
4. ★ **Nothing here is flaky** — *frozen across nine waves is as reproducible as
   this corpus gets, which makes these the cheapest failures in the project to
   work on.*

> ★★★ **This table is the standing worklist with its full history attached, and
> it exists because a `max()` was deleted from the collector.**

## ★★★★★★★★ THE TWO SINGLETONS — RESOLVED, AND THE HARD CORE COMPRESSES TO 9+1

### ★★★★★ SEED 66 (`tl_ok`) — **THE CLAUSE FAILS ON AN UNKNOWN, NOT ON A DEFECT**

| field | seed 66 | passing seeds |
|---|---|---|
| `b5_tool_ok` | ★★★ **`null`** | `true` |
| `b5_tool_stone` / `_steel` | ★★★ **`null`** | `1.5` / `2.0` |
| ★★★★★ **`b5_tool_stone_measured` / `_steel_measured`** | ★★★★★ **`null`** | `1.5` / `2.0` |

★★★ **REG-1 did exactly what it registered — replaced the impossible `0.0`
sentinel with an honest `null`.** ★★★★★ **And the CLAUSE `tl_ok` still fails,
now on "unknown" rather than on a poisoned value.**

> ## ★★★★★★★ **SEED 66'S ONLY FAILING CLAUSE IS AN UNMEASURED QUANTITY. The
> verdict asserts a tool problem the guard never established — which is the exact
> thing REG-1's registration warned about, arriving from the other side.**

★★ **The RAW fields are `null` too, so nothing was measured at all** — *not a
derivation failure, an absence of input.* ★★★ **And seed 66 carries NINE probed
travel timeouts** *(the re-score's largest single-seed set)*.

★ **INFERENCE, stated as one:** *tools are measured from work performed; a colony
whose colonists never arrive never uses tools; so `tl_ok` fails on absence.*
★★★★★ **If that holds, seed 66 is a TRAVEL seed wearing a tools clause — and it
is not a "failing" seed at all but an UNPROVEN one.**

★★ **ACTIONABLE EITHER WAY:** *a clause that fails on `null` cannot distinguish
"the tools were bad" from "we never looked."* ★★★ **Same law as `wave13`'s empty
dict and the exit-0-empty-log: an exclusion and an absence must never render
identically.** ★ **The fix is a third verdict state, not a different threshold.**

### ★★ SEEDS 68 / 92 (`ch_mixed`) — **A CLEAN BOOLEAN, AND 68 IS THE OUTLIER**

**`ch_mixed`: `false` on 68 and 92, `true` on passing seeds AND on 66.**
★ *Consistent — 66 doesn't fail this clause.*

> ★★★★★★★ **SEED 68 IS THE ONLY HARD-CORE SEED WITH NO PROBED TRAVEL TIMEOUT AT
> ALL** *(absent from the re-score's 44-case table entirely)*, **and its ONLY
> clause is `ch_mixed`.**

★★★ **So seed 68 is the one genuinely separate failure in the hard core.**
★★ *92 also fails `ch_mixed`, but 92 is already deep in the travel families —
its `ch_mixed` is a second, possibly independent problem.*

## ★★★★★★★★ THE HARD CORE, FINAL SHAPE

| | seeds | |
|---|---|---|
| ★★★★★ **TRAVEL / ACCESS** | **54 61 62 71 78 80 85 92** + ★★ **66** *(via unmeasured tools)* | ★★★ **NINE OF TEN** |
| ★★★ **genuinely separate** | ★★★★★ **68** *(`ch_mixed`, no travel timeouts)* | **ONE** |

> ## ★★★★★ **NINE OF THE TEN SEEDS THAT HAVE NEVER PASSED ARE ONE SUBSYSTEM.**
> ★★★ **The tenth is a single boolean nobody has looked at.**

★ **`ch_mixed` on seed 68 is now the smallest, cleanest, most isolated open
question in the corpus** — *one seed, one boolean, no travel confound.* ★★ **That
makes it the cheapest thing on this list, and the only one that does not wait on
the travel row.**
