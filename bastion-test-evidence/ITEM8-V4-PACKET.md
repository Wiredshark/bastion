# ITEM 8 v4 — **PACKET**

**Crafted against all five entries of `readme/PACKET-CRAFT-CHECKLIST.md`.**
Evidence base: `ITEM8-V3-RESULTS.md` (capture `807297f2db`).
**v4 passing closes Arc 1.**

---

## 0 · ★★★★★★ WHAT v4 IS FOR — **stated first, because v3 proved this can drift**

| this run VALIDATES | this run does NOT validate |
|---|---|
| ★ the **famine fix** (farm jobs complete; the colony feeds itself) | ⛔ **the crash fix** |
| the **endurance bar** (6 measures over N cycles) | ⛔ #85's gates *(unless a fail-safe fires and carries the six fields)* |

> ## ★★★★ **THE CRASH FIX'S PRIMARY EVIDENCE IS ITS PLANTED TEST — direct
> construction, red-then-green, deterministic.** *(Fable-ruled.)*

★★★ **v4 must NOT inherit the fiction that a silent live run validates it.** *The
crash is known INTERMITTENT — one detonation in two runs of the same binary, a
one-sample fuse each way. A silent run was always weak evidence for it and is now
known to be.* **The live run's job is the famine and the endurance bar.**

---

## 1 · THE FIX — **THREE ROUTES IN MEASURED CAUSAL ORDER**

    route 2  CLAIM EXPIRY        SHIPS FIRST — repairs the famine under EITHER first cause
    route 1  COMPLETION SEAM     the measured ORIGIN — 57/57 arrived+worked, 0 completed
    route 3  SWEEP EXTENSION     the closer — the 27 never-claimed

### ★★★★★ 1a · ROUTE 2 SHIPS FIRST — **the survival-of-being-wrong guard** *(Fable-ruled)*

> ## **A CLAIM HELD BY A COLONIST THAT NEVER COMPLETES MUST EXPIRE.**

★★★★ **Ships first because it repairs the colony whether or not the diagnosis is
right.** *Route 1 is the thing to FIX; route 2 is what makes the colony survive us
being wrong about route 1.* **A fix robust to its own diagnosis is the correct thing
to build while the diagnosis is still open.**

**Requirements:**

- **Expiry is computed, never asserted.** *The `occupied` gate's comment says "one
  LIVE job per cell" while the code computes EXISTS — **"live" must become a property
  the code evaluates.***
- ★ **The expiry releases the CLAIM, and the released job must then be reachable by
  route 3's sweep** — *otherwise the job simply moves from population A (claim-held)
  to population B (unclaimed-unreaped) and the cell stays blocked.* **State the
  post-expiry lifecycle explicitly; a fix that relocates a defect is #60's shape.**

#### ★★★★★★ 1a-0 · **THE F5 TENSION — RESOLVED: OPTION (b), INVERTED LEAK WITNESS**

**Route 2's promotion from guard to primary fix broke F5 as originally written.**
*With a PRECISE fix — unconditional `claimed_by` release + amnesty clearing claims —
a generic expiry should never fire on a healthy run, so "expiry fires > 0" would
demand the symptom the fix exists to prevent.*

> ## **DECISION: KEEP THE GENERIC EXPIRY, AS A LAST-RESORT LEAK WITNESS, WITH ITS
> BAR TERM INVERTED. IT IS NOT PART OF THE FIX — IT IS THE INSTRUMENT THAT TELLS US
> THE FIX WAS INCOMPLETE.**

**Reasoning, and it is the day's own lesson:** *the targeted fix addresses ONE
enumerated route to a held claim (`job.unreachable`). **Today punished incomplete
enumerations four separate times.*** ★★★ **Dropping the backstop bets that
`!job.unreachable` is the only leak; keeping it costs almost nothing and buys a
permanent detector for the next route.**

**THREE CONDITIONS, all mandatory:**

1. ★★★ **INVERTED BAR (F6): zero firings is the EXPECTED PASS. Any firing is a
   RECORDED FINDING** — *not a failure of the run, a discovery that a leak route
   exists which the targeted fix does not cover.* **Report it, do not absorb it.**
2. ★★★★ **REACHABILITY PROVEN BY PLANTED TEST.** *A backstop that should never fire
   is a mute channel unless something demonstrates it CAN.* **Force a leaked claim,
   watch the backstop fire BY NAME, in test.** *(Without this, F6's zero is
   indistinguishable from a backstop that was never wired — this morning's law.)*
3. ★★ **ITS THRESHOLD MUST EXCEED THE TARGETED PATH'S OWN TIMING**, so it cannot
   race the fix it backstops. *Same requirement as §1a-i's watchdog constraint, and
   the same #103 failure if skipped.*

★ **F5 therefore becomes the TARGETED-RELEASE witness** *(releases on `unreachable`
jobs > 0 — the fix's own precondition, zero = VOID)*, **and F6 is the backstop's
inverted term.** *Two witnesses, opposite polarity, neither able to hide the other.*

#### ★★★★★★ 1a-i · THE EXPIRY CONSTANT MUST CARRY ITS PRODUCER *(Fable-ruled)*

> ## **A GUARD WHOSE CONSTANT IS A GUESS IS FINE IF THE GUESS IS NAMED AS ONE. A
> GUARD WHOSE CONSTANT *LOOKS* DERIVED BUT ISN'T IS HOW THRESHOLDS GO WRONG HERE.**

**Three requirements, all mandatory:**

1. **The constant NAMES its derivation in code** — *"derived from X" or "conservative
   guess, not derived" — either is acceptable; silence is not.*
2. **It is emitted in the startup effective-config line.**
3. ★★★ **It is NOT calibrated from any field the expiry itself resets.** *That is the
   #103 censoring shape: a threshold that truncates the distribution it would be
   tuned from cannot be tuned from it, and more data never fixes it.*

##### ★★★★★ AND THE MECHANISM THAT BOUNDS IT — **derive, don't measure** *(#103's real lesson)*

**#103's failure was `ACCESS_STALL_SECS = 120`, which happened to equal the
position-0 queue budget exactly — so the pruner fired precisely where the queue
mechanism was already releasing.** *The correct threshold was derived from the
budget, not calibrated from a distribution that budget generates.*

> ## **THE SAME TRAP IS OPEN HERE: A CLAIM EXPIRY SHORTER THAN THE TRAVEL
> WATCHDOG'S OWN BUDGET WILL FIRE ON COLONISTS THE WATCHDOG IS ALREADY HANDLING** —
> *two mechanisms racing on one condition, and the newer one stealing cases from
> the older.*

★★★ **REQUIRED READ before choosing the number: the travel watchdog's budget, and
any other timeout that can already release or redirect a claimant.** *The expiry
must EXCEED the longest of them, with the margin stated.* **A legitimate claim's
duration is bounded by travel budget + work duration — derive from that, do not
observe it.**

★ *If no such bound can be established, ship a named conservative guess and file a
row for the derivation — **but do not present an observed median as a derived
bound.***

### 1b · ~~ROUTE 1 — THE COMPLETION SEAM~~ — ⛔ **REFUTED 2026-08-11 (5b's read 1)**

> ## ★★★★★ **THE COMPLETION PATH WORKS. My "0 of 87 completed" was a
> SINGLE-CHANNEL MEASUREMENT ERROR.**

**I counted completions by one line kind — `job completed job=N` — found no farm ids
among its 41 entries (39 Mine, 2 Bed), and concluded zero.** *Farm jobs exit by a
path that does not emit that line.*

**5b's count is the real one: 19 `tilled` + 20 `sown` + 20 `harvested` = 59
completions — 19/19 TILL (100%), 20/20 HARVEST (100%), 20/48 SOW (42%).**

★★★★ **VERIFIED INDEPENDENTLY BY CELL RECYCLING** — *the structural proof, since a
cell cannot receive a replacement until its job leaves the board:*

    cells with 1 farm job : 32      <- never recycled
    cells with 3 farm jobs: 10      |  TILL -> SOW -> HARVEST
    cells with 5 farm jobs:  5      |  full second cycles
                                       (32x1 + 10x3 + 5x5 = 87 ✓)

**15 cells cycled 3–5 times. Jobs complete and leave.**

★★★ **THIRD SINGLE-CHANNEL ABSENCE ERROR OF THE DAY** *(the `b5_` counter, the
`NeedCrossed` pattern, this)* — **and it was caught by 5b's arithmetic being too
clean to ignore: 87−59 = 28 = 48−20.** *A number that fits three ways outranks a
number that fits one.*

### ★★★★★★ 1b′ · THE ACTUAL DEFECT — **28 SOW JOBS NEVER ENGAGED**

    completion path ......... WORKS
    the defect .............. 28 SOW jobs never engaged; 32 cells never recycled
    all farm creation ended . 14:54:31

**5b's mechanism, read from code:** *`to_release`'s drain clears `claimed_by` **only
when `!job.unreachable`** — a job marked unreachable never releases its claim, and
the periodic amnesty that resets `unreachable` never touches `claimed_by`.*

> ## **A STUCK SOW CLAIM OUTLIVES ITS CLAIMANT PERMANENTLY.**

★ **Caveat kept as 5b stated it:** *consistent with the TILL/HARVEST-100%
vs SOW-42% split (SOW churns through the high-preemption famine window; TILL and
HARVEST front-load early) — **named as consistent, NOT traced to a specific job's
history.***

### ⛔★★★★★★ 1c′ · ROUTE 3 AS BUILT **CHURNS** — CORRECTED FOR v5 (2026-08-11, mid-v4)

**v4's early check: `designated_sweep_reaps = 186` in 15 minutes, against v3's 87 farm
jobs created in 2.5 hours. Farm silent from minute 3. Confirmed from CODE, no log.**

    :18197   *cycles_since_last_claim.entry(pos).or_insert(0) += 1;   // ticks EVERY cycle
    :19071   cycles_since_last_claim.insert(pos, 0);                  // resets ONLY on CLAIM
    :18261   designated_sweep_should_reap(..., get(&j.pos), reap_threshold_cycles)

> ## **THE GATE MEASURES THE *POSITION'S* TIME SINCE ITS LAST CLAIM — NOT THE
> *JOB'S* AGE. A newly created job at a position unclaimed for one gate-period is
> BORN PAST THE GATE and reaped on sight; the cell frees, a replacement is
> generated, the counter is still high, and it is reaped again.**

★★★ **THE CONSTANT IS INNOCENT.** *`access_stall_secs() / (ARBITRATION_INTERVAL /
SIM_TPS)` is seconds ÷ seconds-per-cycle — units correct, derived from the live
function, ~930 s worth.* **Nothing could age 930 s inside a 15-minute window, and
nothing did: the jobs did not age, they INHERITED an age.** *Not a #103 threshold
problem — **a correct threshold applied to the wrong quantity**.*

#### THE v5 FIX — **intent, not a new constant**

1. ★★★ **The job carries its OWN created-at cycle**; the sweep reads the job's age,
   never the position's claim recency.
2. ★★★★ **PLANTED TEST, and it is the one that would have caught this:**
   **A FRESH JOB AT A STALE POSITION MUST NOT BE REAPABLE.** *Red before, green
   after.*
3. **Keep `cycles_since_last_claim` for what it actually names** — *starvation
   telemetry keyed by position (TASK #59's purpose). It is not wrong; it was
   misused.*

#### ★★★★★ THE REVIEW LESSON — **why this passed my gate**

**I verified: the test exists · the predicate is extracted pure · the constant
reads the live function · the flagged deviation's reasoning holds. All four true.**

> ## **I NEVER ASKED WHAT THE AGE ARGUMENT MEASURES. A PURE PREDICATE'S TEST PROVES
> THE PREDICATE, NOT ITS INPUTS.**

★★★ **`designated_sweep_should_reap(claimed, is_designated, age, threshold)` is
correct and its unit test passes.** *The defect is entirely in what is passed as
`age` — and the field's own name carried the answer: **"cycles_since_last_claim" is
not job age**, and I read it as job age anyway.*

★★ **STANDING ADDITION TO REVIEW PRACTICE:** *when a predicate takes a quantity as
an argument, **the quantity's provenance needs the same check the threshold's got.***
**Ask of every argument: what produces this, and does its name describe what I think
it describes?**

★ **And the failure-mode table's gap:** *route 3's table never contained "reaps young
jobs", because the enumeration assumed age meant age.* **A planted test is only
written for a failure mode someone named.**

### 1c · ~~ROUTE 3 — THE SWEEP EXTENSION~~ *(original, superseded by 1c′)*

**The orphan sweep covers only `DepositRun | RestAt | EatFrom | Despond` **and**
requires `claimed_by.is_none()`.** *`Designated` is outside its kind list.*

★★★ **DO NOT SHIP THIS ALONE.** *Adding `Designated` to the kind list fixes the 27
never-claimed and leaves the 57 untouched — **27 of 84, famine intact.*** *(Caught
by measurement before it was built; #60's absorbed-fix shape.)*

### 1d · EVERY SCOPED-OUT RESIDUAL GETS A ROW *(checklist entry 3)*

★ **Including:** *the `remove_job` audit I declared but did not run (READ for the
orphan sweep, INFERRED for the other ~19 sites), and the six-site `occupied`
uniformity sweep — all six use the identical `values().map(pos)` construction, so a
fix at the shared construction lands once rather than six times.*

---

## 2 · SENTINEL — **S1 LOG-ONLY, RIDING THE HEARTBEAT** *(mandatory staging)*

    stage 1   compute the predicate · LOG "COLONY TERMINAL" · DO NOT SHUT DOWN
    stage 2   BASTION_END_ON_TERMINAL only after N runs check the verdict against outcome

> ## ★★★★★ **v3 IS THE FOUNDING CALIBRATION CASE: the predicate would have tripped
> at 16:02:49 and terminated the run 79 MINUTES EARLY**, destroying the only live
> recording of a colony's failure system under total famine.

★★ **The predicate is not too strict — it fires correctly. It fires EARLY relative
to the data's value.** *A sentinel that ends runs destroys the evidence that it is
wrong; stage 1 is the only way that evidence can ever exist.*

**It is a GATE INPUT, not a diagnostic** — *so it needs acceptance-bar standards: a
planted test forcing a terminal state that trips it BY NAME, **and the mirror — a
registered NON-terminal case (breakdown-with-food-incoming) that must NOT trip it.***

★ **Field on the heartbeat line** *(entry 1: live by construction, and it inherits
the line's cadence).*

---

## 3 · THE `b5` HEARTBEAT PORT — **the cadence dividend**

    info!(tick = tick.0, food_stock,
          splits = board.b5_split_off_one_fired,      // the whole change
          "bastion food stock sample");

★★★ **Verified feasible: the emit site already has `board` in scope** *(it calls
`board.stockpile_at(cell)` eleven lines above).* **And riding a `tick % 300`
unconditional line turns a run-end scalar into a PER-WINDOW SERIES** — *the rate the
bar wanted and could only ask for as a count.*

★ **Row filed:** `ROW-SPLIT-COUNTER-HAS-NO-CONSUMER.md`. *The `b5_*` class is
harness-only by construction and its naming gives no hint of reachability.*

---

## 4 · LAUNCH PRECONDITIONS — **GATE 0 IS MANDATORY** *(checklist entry 5)*

1. ★★★ **REBUILD `veloren-server-cli` from the intended pin — and READ THE OUTPUT'S
   `Compiling` LIST to confirm the package is in it.** *A scoped `-p`/`--bin` filter
   exits 0 while silently excluding the binary that matters.*
2. ★★★★ **GATE 0: the log's `Server version` MUST match the intended pin, checked
   BEFORE the run proceeds past founding.** *One grep, at minute one.*
3. **Effective config emitted and DIFFED against the previous arm.** *(The v2/v3 50×
   log-rate scare dissolved to ~1.5× once the number's producing RUN was checked —
   but the diff is now a named precondition, not a reaction.)*
4. **One binary for the whole run** — *5b's rebuild rides AFTER the famine fix lands,
   so v4 flies one build, not two.*
5. **Teardown by the proven manual sequence**; `reap-server.sh` exercised separately
   on a sacrificial target.

---

## 5 · RUN DESIGN — **justified against the crash's KNOWN INTERMITTENCY**

**What run length buys:**

| length | buys | does NOT buy |
|---|---|---|
| ≥ 5 cycles | the endurance bar's 6 measures; famine-fixed observable across cycles | **crash-fix validation** |
| any length | — | *a silent run cannot validate an intermittent crash at n=1* |

★★★ **State plainly in the results: the run length is chosen for the ENDURANCE BAR's
cycle count, not to "outlast" the crash.** *v3 already ran 6.5× v2's fuse and proved
nothing about the fix.*

★ **If crash-fix live evidence is ever wanted, it needs a REPETITION COUNT derived
from the observed detonation rate — not a longer single run.** *Out of scope here;
noted so it is not silently assumed.*

---

## 6 · ACCEPTANCE — **the famine measures**

| # | measure | PASS | FAIL |
|---|---|---|---|
| F1 | ★★★ **farm jobs COMPLETE** | `job completed` with `kind=Designated(Farm)` **> 0** | **0 completions = the v3 failure reproduced** |
| F2 | **no immortal jobs** | no farm job outlives N cycles unclaimed-or-unprogressing | any job resident for the whole run |
| F3 | **cells recycle** | the same cell receives a NEW job after its predecessor ends | a cell jobbed once and never again |
| F4 | **food is produced** | stock rises after the founding stock is consumed | monotonic decline to zero |
| F5 | ★★★ **targeted release FIRES** | `claimed_by` released on an `unreachable` job **> 0** | **0 = VOID** *(the fix was not exercised)* |
| F6 | ★★ **backstop stays SILENT** | generic expiry firings **== 0** | **any firing = a recorded FINDING** |

★★★ **F5 is the sit-trap discipline applied to the guard**: *a run where nothing
expired cannot distinguish "expiry works" from "expiry never triggered."*

★★ **F1 is the row's spine — everything else is downstream of a farm job completing.**

---

## 7 · ★★★★★★ WHAT I WILL NOT DO AT SCORING TIME *(checklist entry 4 — written by the scorer, before the run)*

**v4's particular temptations, named in advance:**

1. **I will not accept "the colony survived" as the famine fix passing.** *F1 is a
   COUNT of farm completions; survival is not a substitute for the mechanism.*
2. **I will not let the crash fix's silence become evidence.** *Not scored from this
   run. If asked whether the fix held, the answer is "its test says so; this run
   doesn't speak to it."*
3. ★★ **I will not treat a PARTIAL famine recovery as a pass.** *Food rising once,
   then collapsing, is a different failure — reported as what it is.*
4. **I will not score F5 as PASS on zero.** *Zero expiries is VOID on route 2.*
5. ★★★ **I will not let route 2's success hide route 1's failure.** *If claims expire
   and cells recycle but farm jobs still never COMPLETE, the guard worked and the
   origin is unfixed — **that is two results, and the bar says so separately.***
6. **I will verify the binary before reading a single measure.** *Gate 0, then
   preflight, then measures — in that order, no exceptions.*

---

## 7b · ★★★★ CHURN POST-MORTEM — **REGISTERED MID-RUN, BEFORE SCORING**

*Written from the T+70 heartbeat, before teardown and before any log read. The
mechanism is already confirmed from code (§1c′); this registers its **SHAPE** so the
numbers meet a prediction.*

    T+15 min ....... 186 reaps          =    12.4 / min
    T+70 min ....... 184,527 reaps      = 3,352.0 / min over the last 55
    acceleration ... ~270x   (linear from the early rate predicts ~868)

> ## **IT DID NOT GROW. IT IGNITED.**

### THE PREDICTED MECHANISM — **the churn is FAMINE-COUPLED**

**The gate reads a POSITION's time-since-last-claim.** *Early, colonists are actively
claiming, so most positions sit under the threshold and only a few stale cells churn.*

> **AS FAMINE STOPS COLONISTS CLAIMING → MORE POSITIONS CROSS THE THRESHOLD → MORE
> JOBS BORN PAST THE GATE → MORE DESTROYED BEFORE ANYONE CAN CLAIM THEM → FEWER
> CLAIMS. The loop feeds itself.**

★★★ **This resurrects the SPIRAL RIDER that died with preempt-abandonment.** *The
shape was right; the engine was wrong.* **A self-amplifying famine exists — it
belongs to the sweep, not to preemption.**

### DISCRIMINATING READ *(registered now, so 184,527 meets a prediction)*

    reaps per window  PLOTTED AGAINST  claims per window
    -> FAMINE-COUPLED : reap rate rises as claim rate falls, inversely, across the run
    -> PURE RUNAWAY   : reap rate rises independently of claims — coupling story WRONG

★★ **If the inverse correlation holds, v5's fix acquires its falsifier: v5's reap
count must stay FLAT as the colony's claim rate varies.** *A fix that merely reduces
the number is not the same as a fix that decouples it.*

### THE TWO CLEAN RESULTS ALONGSIDE

- ★★★ **SENTINEL: 3 firings, log-only — the SECOND calibration case, in data.**
  *Read when they landed against all-breakdown timing.* **Two independent famines is
  where a threshold stops being a guess.**
- ★★ **F6 ZERO across 70 minutes of pathological churn.** *The leak witness stayed
  silent while a different subsystem raged —* **a witness that stays quiet during
  someone else's catastrophe has demonstrated SPECIFICITY, not just reachability.**

★ *`claim_expiry_releases = 2` and `splits = 0` are both consistent with a colony
that has almost no functioning economy left to leak or split.*

## 8 · READ-LIST

**Kept during scoring, per the raw-commit condition** — *patterns run, fields whose
values entered a verdict, **and line-kinds opened and found irrelevant.*** ★ *The
third column is the one a usage-only filter destroys, and v3's five failed patterns
(`NeedCrossed`, `despond.*uid=`, `colonist=[A-Za-z]+`, `b5_*`, the entity-event-log)
are why it is kept.*
