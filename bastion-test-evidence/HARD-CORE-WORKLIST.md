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

## ★★★★★★★★ SEED 68 — **CLOSED. THE PRIOR CLASSIFICATION RE-VERIFIES ON WAVE 30.**

**Fable's redirect: run the trio ON MYSELF before a fresh read.** ★★★ *`c362de6b05`
(wave25 era) had already classified 68 as a 63× cells-per-tree outlier.*

### ★★★★★ THE RE-CHECK — IT HOLDS EXACTLY

| | wave25 claim | wave30 |
|---|---|---|
| `ch_cells` / `ch_trees` | **30 / 1 → ratio 30.0** | ★★★★★ **30 / 1 → ratio 30.0** |
| every other seed | **1891-2048 band** | ★★★ **cells/tree < 1000 on SEED 68 ALONE, across all 48** |

> ★★★★★ **SEED 68 CLOSES AS AN INSTRUMENT-SCOPING ARTIFACT.** *A 30-cell
> degenerate AABB cannot hold the trunk-and-canopy pair `ch_mixed` scans for.*
> ★★ **Fix already in the backlog: min-box floor / precondition assert.**

★ **The corroborating detail wave25 couldn't see: `ch_ground_truth_tree_present =
True` with a witness carrying BOTH `wood_pos` and `leaves_pos` one z apart.**
★★★ **A mixed tree demonstrably EXISTS on seed 68 — the 30-cell box simply
doesn't contain it.**

### ★★★★★★★ AND A UNIFICATION I TESTED AND KILLED

**Wave 30 shows 68's witness at `ring_index 5` and 92's at `10`, both against
`rings_tried = 4` — and the passing seeds' witnesses at ring 0.** ★★ **I
hypothesised: *both `ch_mixed` failures have their witness OUTSIDE the rings
scanned* — which would have UNITED the two failures the prior read SPLIT.**

**Tested across all 48 before claiming it:**

    (witness_ring >= rings_tried, ch_mixed):
        (False, True): 28      (True, True): 16   <<<
        (True, False):  2      (missing, False): 2

> ## ★★★★★ **DEAD. SIXTEEN SEEDS HAVE THEIR WITNESS OUTSIDE THE SCANNED RINGS AND
> `ch_mixed` IS STILL TRUE.** ★★★ **The prior read's *"two unrelated situations"*
> STANDS, and my tidier story was wrong.**

★ **Cost: one query. The hypothesis was attractive because it unified — which is
exactly the property that should have made me test it first, and did.**

## ★★★★★★★★ AND THE THIRD-VERDICT-STATE ROW IS CHEAPER THAN I FILED IT

**Seeds 55 and 63 also carry `ch_mixed = False` — and they do NOT fail the
clause. Their `b5_ch_oracle_class` is `precondition_unmet`** *(0 cells, 0 trees:
no tree exists to be mixed)*.

> ## ★★★★★ **THE THREE-WAY VERDICT SEED 66's ROW NEEDS ALREADY EXISTS — FOR CHOP.**
> **`ch_oracle_class ∈ {pass, precondition_unmet}` is exactly pass / UNPROVEN.**

★★★ **So the row is not *"invent a third state"* — it is *"generalize the one chop
already has to every clause that can fail on a null input."*** ★★ **Distribution
across wave 30: `pass` 46, `precondition_unmet` 2.** ★ **The pattern is built,
proven, and in production on one family.**

## ★★★★★★★ THE HARD CORE — **FINAL**

| | seeds | |
|---|---|---|
| ★★★★★ **TRAVEL / ACCESS** | 54 61 62 71 78 80 85 92 **+ 66** | **9** |
| ★★★ **INSTRUMENT ARTIFACT** *(closed)* | ★★★★★ **68** | **1** |
| **open mysteries** | — | ★★★★★★★ **ZERO** |

## ★★★★★★★★ SNAPSHOT-CLASS ENROLMENT (2026-08-09) — AND A CAVEAT ON THIS DOCUMENT

**DECISIONS #80 ordered the wave30-born fields enrolled in holdcheck's IN-FLIGHT
SNAPSHOT class.** ★ **Measured members, each floored on a seed where it moved
(same binary `5b9a1a9724`, two runs):**

| member | evidence |
|---|---|
| `b5_travel_timeout_last_positions` | ★★★ **MEASURED** *(seeds 49 and 90)* |
| `b5_travel_timeout_min_distances` | ★★★ **MEASURED** *(seeds 49 and 90)* |
| ★★★★★ **`b5_mine_cell_diag[*].stuck_strikes`** | ★★★ **MEASURED — `1 → 4` on seed 90, run-to-run.** *A NEW member; `claimant` and `progress` were already enrolled from #58.* |
| `b5_self_job_reachability_probe` | ★★ **DERIVED** — *computed from `travel_timeout_last_positions` by its own producer. Labelled as derivation, not measurement.* |

★★★★★ **DELIBERATELY NOT ENROLLED: `b5_mine_cell_diag[*].unreachable`.** *It moved
in the fan on seed 54 but did NOT vary in either floor run.* ★★★ **It is also the
MECHANISM FIELD behind the entire build/chop family — enrolling a mechanism field
without measuring it is the exact error this class exists to prevent.**

## ★★★★★★★ THE CAVEAT THIS RAISES ABOUT THE WORK ABOVE

> ## ★★★★★ **`stuck_strikes` IS RUN-TO-RUN VARIABLE — AND THIS DOCUMENT'S BUILD
> AND CHOP FAMILY FINDINGS REST ON THE SAME KIND OF QUANTITY.**

**`BUILD-FAMILY-ANSWERED.md` and `CHOP-FAMILY-ANSWERED.md` characterise the hard
core using `times_offered`, `timeouts_on_this_cell` and `starvation_cycles` —
★★★ per-job counters accumulated over a run, exactly the shape `stuck_strikes`
turned out to have.** ★ **I never floored them.**

★★ **WHAT SURVIVES REGARDLESS:** *the STRUCTURAL facts — `unreachable` latching,
arbitration skipping latched jobs, the second build job being healthy on every
seed, and the planner-refused / strike-released split by `blocked_by` — are
categorical, not counted.*

★★★★★ **WHAT NEEDS A FLOOR BEFORE IT IS QUOTED AS A NUMBER:**
*"`times_offered == timeouts_on_this_cell` on five of six"* **and the
`starvation_cycles` range.** ★★★ **Two runs on one affected seed settles it, and
it belongs to the travel row's opening rather than to a later surprise.**

> ★ **Filed against my own strongest result of the night, before anyone builds on
> it.** ★★★ **A delta is only evidence once the instrument is known stable — and
> that applies to the findings I liked as much as to the bar I got wrong.**

## ★★★★★★★★ CAVEAT CLOSED — **THE COUNTERS ARE STABLE, AND THE REASON UNIFIES THE CLASS**

**Seed 85, same binary `5b9a1a9724` *(stamps verified equal)*, two runs, comparing
`times_offered · timeouts_on_this_cell · starvation_cycles · unreachable ·
cycles_since_last_claim` across `b5_build_job_diag` (2 entries) and
`b5_ch_job_diag` (1 entry):**

> ## ★★★★★ **ZERO DIFFERENCES. THE BUILD AND CHOP FAMILY NUMBERS STAND AS
> QUOTED.**

★★★ **So *"`times_offered == timeouts_on_this_cell` on five of six"* and the
`starvation_cycles` ranges are stable measurements, not per-run accumulators.**

## ★★★★★★★ AND THE INTERESTING PART — WHY THESE ARE STABLE WHILE `stuck_strikes` IS NOT

**Both are per-job counters accumulated over a run. One varies, one doesn't.**
★★★ **The discriminator is not the FIELD — it is whether the job was STILL IN
FLIGHT when the snapshot was taken.**

| job | state at snapshot | counters |
|---|---|---|
| **seed 90's mine cell** | ★★★ **LIVE** — *claimant present, `progress 0.87` mid-swing* | ★★★★★ **VARY** *(claimant, progress, `stuck_strikes 1→4`)* |
| **seed 85's build cell** | ★★★ **LATCHED** — *`unreachable: true`, `starvation_cycles 285`, amnesty dormant* | ★★★★★ **STABLE** |

> ## ★★★★★★★ **A LATCHED OR DORMANT JOB STOPPED CHANGING HUNDREDS OF CYCLES
> BEFORE THE SNAPSHOT. A LIVE JOB IS BEING PHOTOGRAPHED MID-STRIDE.**

★★★★★ **THE IN-FLIGHT SNAPSHOT CLASS IS ABOUT THE JOB'S STATE, NOT THE FIELD'S
NAME** — *and that is why the hard core's numbers are trustworthy: every one of
those jobs is latched and starved, frozen long before measurement.*

★★ **It also predicts the class correctly going forward:** *the same field is
snapshot-natured on a live job and stable on a dead one.* ★ **A membership list
keyed on field names will therefore always be an approximation — a useful one,
but the real predicate is liveness.**

★★★ **Stated at its strength: ONE seed, ONE comparison, plus a mechanism that
explains both observations and predicts the split. Not a law yet — a strong
candidate with its own next test named** *(floor a LIVE build job — e.g. seed
80's, the one specimen whose latch has not closed — and it should VARY).*
