# WAVE 30 — PRE-REGISTERED, WRITTEN WHILE THE FAN IS STILL RUNNING

**Fan: `bastion/baseline-pre-site6` → `0fb7ca07b7`, `--b5-scenario`, seeds 49-96.**
★ **Every line below was written BEFORE any wave-30 number existed.**
★★★ *A registration written after the numbers is a story about them.*

## WHAT THIS WAVE IS FOR

| comparison | question |
|---|---|
| ★★ **wave26 → wave30** | **what did the last 49 commits do to the work economy?** *(retrospective — never run)* |
| ★★★★★ **wave30 → post-row fan** | **the CLEAN exact-match test for site 6 + both gates + sites 4/5** |

★ **Baseline is `wave26_ROWA_d5b56d1c79_FULL.json`** *(48 seeds, 93 keys, plain
`--b5-scenario` shape — NOT `wave29`, which is a paired A/B run in a different
key shape).*

## ★★★★★ THE 49-COMMIT DELTA, ENUMERATED BY SOURCE

**Four populations of change. ★ Each gets its own expected effect and falsifier.**

### ~~P1 — MUTATING WINDOW `REG-1..4` — EXACT DELTAS ALREADY REGISTERED~~
### ★★★★★★★ P1 **INVERTED BEFORE THE FAN LANDED — THE CODE WAS NEVER BUILT**

**I wrote P1 as *"the only movers with per-seed exact predictions — the wave's own
self-test."* ★★★ Then I checked the tree instead of the plan.**

**At `0fb7ca07b7`, verified against BOTH the harness source and the `wave26`
baseline:**

| REG | field | tree at tip | wave26 | verdict |
|---|---|---|---|---|
| **1** | `b5_tool_stone` / `_steel` / `_ok` | ★ **old names, old sites** (`5018-5022`) | `1.5` / `2.0` / `true` | ★★★ **NOT BUILT** |
| **2** | `route_next_idx_pinned` | still the **summary** (`3614`) | ★ **nested, not top-level** | **NOT BUILT** |
| **3** | `b5_55_blocked_by` / `_names_blocker` | ★ **old names** *(one appears only in a COMMENT)* | present, old names | **NOT BUILT** |
| **4** | `build_ok_jobs` etc. | old names | `1` | **NOT BUILT** |

> ★★★★★ **`b99e8dcbac "Mutating window: REG-1..4"` IS THE REGISTRATIONS DOCUMENT.
> The window was declared ELIGIBLE with its paperwork complete — and then never
> built.**

### ★★★ SO P1'S FIELDS JOIN THE **HOLD** SET, AND A MOVER AMONG THEM IS A FINDING

**`b5_tool_ok` stays `true`; the sentinel stays; `b5_55_*` keep their old names;
the constant diags keep theirs.** ★★★★★ **If any REG-1..4 field moves in wave30,
something changed it WITHOUT the registered fix — which is a defect, not a
confirmation.**

> ★★★★★★★ **AND NOTE WHAT I NEARLY DID: predicted a mover, seen it absent, and
> filed the ABSENCE as the defect** — *when the truth is the change was never
> made.* ★★ **"Registered" and "built" are different states, and the registration
> document reads identically in both.**

★ **Same costume as reading my own spec as the producer, four hours earlier.**
★★★ *A plan is not a producer. Check the tree.*

### P2 — FARM-PAINT (`a067c17329`) — ★★★ FARM COUNTS **MAY RISE**

**Per-column surface resolution; its commit message says it *"fixes silent
zero-jobs on a mis-painted farm."***

| | |
|---|---|
| **expect** | ★★ **farm job counts RISE on seeds that previously produced zero** |
| **falsifier** | ★★★★★ **a farm count FALLING anywhere.** *The fix admits jobs that were silently dropped; it removes none.* |
| ★ **and** | **a rise on seeds whose farms were never mis-painted** — *that would mean the resolver changed correctly-painted farms too, which is a different change than advertised* |

### P3 — ★★★★★★★ THE UNIFICATION (sites P/1/2/3) — **WORK THROUGHPUT MUST FALL**

**This is the one that matters, and it is the one prediction I most want on the
record before the numbers.**

> ★★★ **The fan is BLIND to needs** *(zero bed/sleep/eat/hunger/mood fields —
> measured)*. ★★★★★ **But it is NOT blind to their CONSEQUENCE: colonists who now
> actually rest and eat are colonists not mining, chopping, or building.**

| | |
|---|---|
| ★★★★★ **expect** | **work-job throughput DOWN by roughly the fed/rested fraction** |
| ★★★ **falsifier A** | **throughput UNCHANGED** ⇒ *the unification is INERT in the corpus scenario — self-jobs never fire here, and every "harmlessness" claim about it is vacuous* |
| ★★★★★★★ **falsifier B** | **throughput UP** ⇒ *something is wrong that I have no story for, and it outranks everything else in this document* |

★★ **This is FR15 read against INTENT, not against zero** *(my own checklist §7:
"the economy shifted" is the POINT — colonists rest when depleted, not never and
not constantly).* ★ **A fall is a PASS. Only its ABSENCE or its inversion is a
finding.**

> ★★★★★ **AND FALSIFIER A IS THE LIVE RISK.** *The whole AUTON-2 row has been
> validated on `preempt_scenario`, a bespoke fixture with a planted bed. Nothing
> establishes that `--b5-scenario` colonists ever get tired enough to rest.*
> ★★★ **If throughput is unchanged, that is not a null result — it is the
> discovery that the harmlessness gate was never exposed to the mechanism.**

### ★★★★★★★ FALSIFIER A — CHECKED WHILE THE FAN RAN, AND LARGELY KILLED

**`b5_scenario`'s own code (`~3634`) says self-jobs occur in it:**

> *"Self-jobs (`RestAt`/`EatFrom`/`Despond` — **e.g. seed 7's bed**) never got
> [the reachability probe]"*

★★★ **So self-jobs are CREATED and TRAVEL and TIME OUT in this scenario.** ★★
**The gate is exposed to the mechanism after all.**

★ **What remains genuinely unknown: whether those self-jobs COMPLETE** *(a
colonist that starts toward a bed and never arrives costs travel time but banks
no rest)*. ★★★★★ **So the refined risk is not "the mechanism never fires" but
"it fires and never finishes" — which would show as throughput DOWN with no
compensating rest, a WORSE outcome than either registered branch.**

> ★★ **REGISTERED AS FALSIFIER A′: throughput falls AND self-job travel timeouts
> are high ⇒ colonists are paying for rest they never receive.** ★ *That is a
> defect, not a pass, and I would have read it as P3 succeeding.*

### P4 — ADDITIVE INSTRUMENTS — ★ NEW KEYS ONLY

**The chop+build instrument window (`6d0077e0d6`) and the settle invariant going
live in every scenario (`1912d8193b`).**

- ★★ **Expect the key count to RISE from 93.** *`--expect-new` covers it.*
- ★★★★★ **`settle_invariant_holds` — its FIRST corpus appearance.** *Both readings
  pre-registered hours ago: `false` somewhere ⇒ the site-6 bar is real; `true`
  everywhere ⇒ the bar is VACUOUS on this population and must be reported as
  such, never narrated as a pass.*
- ★ **An additive key that does NOT appear is a finding** — *the instrument
  landed in the tree but not in this scenario's emit path, which is exactly the
  gap I found today with the invariant* **(instrumented everywhere, observed in
  one probe).**

### ★★★★★★★★ `self_job_reachability_probe` — **THE FIRST SELF-JOB FIELD THE CORPUS HAS EVER CARRIED**

**Verified: neither `wave26` nor `wave29` contains it** *(their only "self" keys
are `b5_access_plan_self_rescue_*`, a different mechanism)*. **Its commit
`a2745d5a7d` post-dates `wave26`, so ★★★ WAVE 30 IS ITS FIRST APPEARANCE.**

> ★★★★★ **This partially closes the blindness measured in
> `NEED-SUBSYSTEM-OBSERVABILITY-SPEC.md` — by a change I directed this morning,
> which I had not connected to tonight's baseline until reading the scenario.**

★★ **AND IT IS THE MOST USEFUL FIELD IN THE WAVE, because it gives P3 an
EXPOSURE POPULATION:**

| | |
|---|---|
| ★★★★★ **seeds with a NON-EMPTY probe** | **self-jobs demonstrably travelled and timed out ⇒ the mechanism is live on that seed** |
| ★ **seeds with an EMPTY probe** | **either no self-jobs, or none that ever timed out** — ★★ **and those two are NOT the same thing; the field cannot distinguish them** |

> ## ★★★★★★★ **READ P3'S THROUGHPUT CHANGE CONDITIONED ON THE NON-EMPTY SET,
> NOT ACROSS ALL 48.**
> ★★★ **`WIP-STATE.md`'s own lesson: the 48-seed aggregate diluted the last
> result ~4:1 and hid it.** *A real effect on 11 seeds is a 1/4-size effect on 48.*

★ **CAVEAT, and it is the field's own doc that supplies it:** *the FIRST version
of this filter mislabeled completed mine cells as self-jobs* **(seed 90 showed 6
entries, all inside the mine designation)** *and was fixed to exclude the whole
mine region.* ★★★ **So on this wave, verify the entries are OUTSIDE the mine
region before treating the population as real** — **the instrument's own known
failure mode, named by its author, and exactly the kind of thing that gets
forgotten one wave later.**

## ★★★★★ REHEARSED ON FREE DATA FIRST — AND IT FOUND SOMETHING

**`wave30_diff.py` was written before wave 30 existed and rehearsed on
`wave25 → wave26` (Row A). ★★★ Three defects in my own tool, caught at zero
cost:**

| # | defect | why it mattered |
|---|---|---|
| 1 | **console encode crash on a non-ASCII marker** | ★ would have died mid-report on paid data |
| 2 | ★★★ **a substring rule fired a PHANTOM `HOLD-VIOLATION`** on `b5_tool_steel_measured` *(prefix of `b5_tool_steel`)* | ★★★★★ **a classifier that over-reaches is as dishonest as one that under-reaches — it just fails in the direction that looks diligent** |
| 3 | ★★★★★ **`b5_build_stamp` and `b5_soak_avg_tick_ms` move on EVERY wave by construction** | **48 identical lines at the top of the findings section, burying the one field that mattered** |

> ★★ **ROW A MOVED EXACTLY ONE MEANINGFUL FIELD OUT OF 121** *(`b5_mine_cell_diag`,
> 6 seeds)*. ★ **That is the precedent for how contained these comparisons
> normally are — and it sets the bar for reading wave 30's volume.**

### ★★★★★★★ AND THE REHEARSAL RECOVERED A FACT ABOUT TONIGHT'S RULING

**Row A's single mover added a nested `blocked_sources` key — and on seeds 71 and
90 its value is `["route_exhausted"]`.**

> ★★★ **So `route_exhausted` ALREADY FIRES in the corpus, on ~2 of 48 seeds, for
> MINE cells — with source attribution intact.**

★★★★★ **That is the work-kind path, which the `12248` gate leaves enabled — so
it is consistent, and it gives the gate a MEASURED base rate rather than an
assumed one.** ★★ **It also independently confirms the D5 withdrawal: the producer
is live and its rate would have risen once self-jobs began feeding it.**

★ **I ruled on this producer tonight from source alone. The corpus had the
runtime evidence the whole time, one diff away.**

## ★★★ WHAT I EXPECT TO **HOLD** — the load-bearing half

**Everything not named in P1-P4.** ★★★★★ **Specifically: mine counts, chop counts,
haul counts, `blocked_regions` counts, access-plan call/emission pairs, and every
reachability probe.**

> ★★ **A mover outside P1-P4 is a FINDING, and I have pre-committed to treating it
> as one rather than reaching for an explanation after the fact.**

★ **`b5_blocked_regions_count_*` is explicitly in the HOLD set** — *D5 was
withdrawn once the `12248` gate was ruled; self-jobs will never feed it.*

## ★★★★★ THE TRAP I AM SETTING FOR MYSELF

**Four populations, ~49 commits, and a rich diff is guaranteed. ★★★ The failure
mode is not missing a mover — it is EXPLAINING one.**

> ## ★★★★★★★ **EVERY MOVER GETS ASSIGNED TO P1, P2, P3, OR P4 *BY NAME*, OR IT IS
> UNEXPLAINED. "Probably the unification" IS NOT AN ASSIGNMENT.**

★★ **Today's law, applied forward: a registration EXPLAINS a mover; it must never
EXCUSE one.** ★★★★★ **And its companion, learned the same day — before accepting
any assignment, ask whether the producing chain is itself CORRECT.** *P2's farm
rise and P3's throughput fall are both "expected" — and both would still be worth
reading as defects if their mechanism turned out to be wrong.*

★ **Collection is `collect_wave.py` + `derived.py` automatically** *(there is no
step 3)*. ★★ **`derived.py`'s rate/concentration/denominator-gap families run on
this wave without my choosing which to look at — which is the point of wiring it
in.**

## ★★★★★★★ A DEFECT IN MY OWN TOOL, FOUND WHILE WAITING — **PROTECTED BY ACCIDENT**

**`derived.py`'s cross-wave analysis groups comparable waves by SEED SET
(`groups.setdefault(frozenset(seeds), …)`, `313`). ★★★ `wave26`, `wave29` and
`wave30` all carry seeds 49-96 — so by that rule the PAIRED wave and the PLAIN
waves are one comparable group.**

> ★★★★★ **SAME SEEDS IS NOT THE SAME SCENARIO. That is precisely the error I
> nearly made tonight by reaching for the newest wave as the baseline.**

### ★★★ WHY IT DOESN'T BITE TODAY — AND WHY THAT ISN'T REASSURING

**`failing_set` (`181`) reads `VERDICT_FIELD = "b5_failed_clauses"` at the TOP
LEVEL of each seed. ★★ A paired wave nests everything under
`paired_base`/`paired_variant`/`paired_delta`, so the lookup fails, the wave
returns `(None, reason)`, and it is set aside BY NAME in the `no_verdict` list.**

> ★★★★★★★ **THE GUARD THAT KEEPS SCENARIOS FROM MIXING IS A SIDE EFFECT OF A
> FIELD LOOKUP, NOT A SCENARIO CHECK.** ★★★ **It holds for exactly one reason: the
> paired shape happens to bury its verdict one level down.**

★★ **A future variant that carries a top-level `b5_failed_clauses` with different
semantics would be SILENTLY grouped with the plain waves** — *same seeds, readable
verdict, different scenario, and `derived.py` would report FIXED/NEW across the
boundary as though it meant something.*

### ★ THE FIX, FILED NOT BUILT

**Group by `(seed set, MODAL KEY SET)`, not seed set alone.** ★★★ *`collect_wave.py`
already computes a modal key set — for excluding short-JSON seeds — so the concept
exists in the tree and needs no new machinery.*

> ★★★★★ **AN ACCIDENTAL GUARD IS ONE REFACTOR AWAY FROM GONE, and it leaves no
> trace when it disappears** — *nothing names the property it was protecting.*
★ **Same shape as today's other finding about the settle counter: a mechanism can
be correct and undocumented as to WHY, and the why is what the next change
deletes.**
