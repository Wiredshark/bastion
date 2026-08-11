# ITEM 8 v3 — **RESULTS (of the UNFIXED BUILD)**

> ## ⚠ **EVERY FINDING BELOW IS A FINDING ABOUT `fb9a7401` — THE PRE-FIX BINARY.
> THE CRASH FIX WAS NOT IN THE RUNNING PROGRAM.** *(Fable-ruled labelling; §0.)*

### ★★★★ THE FIX CLAIM'S EVIDENCE, REWEIGHTED PERMANENTLY (Fable-ruled)

**With the crash now known INTERMITTENT — 1 detonation in 2 runs of the same
binary, one-sample fuse each way — a silent live run was ALWAYS going to be weak
evidence.**

> ## **THE CRASH FIX'S PRIMARY EVIDENCE IS ITS PLANTED TEST** — *direct
> construction of the violating state, red-then-green, deterministic, already
> banked.* **The live run's role shrinks to what it can actually prove: the famine
> fix and the endurance bar.**

★★ **v4 must NOT inherit the fiction that 2.5 silent hours validates the crash fix.**
*v4 validates the FAMINE fix and the endurance bar; the crash fix is validated by
its test.*

### THE LAUNCH MECHANISM (5b, `5845b68068`)

    cargo build --profile no_overflow -p veloren-server-cli -p veloren-client --bin bastion_playtest

★★★ **`--bin bastion_playtest` restricted target selection to that one binary —
which lives in `veloren-client`, NOT `veloren-server-cli`.** *Despite
`-p veloren-server-cli` being passed, that package's binary was never in scope.
Confirmed from the captured build output: only common/client crates ever compiled.*

> **The command was the LABEL; the compiled-crates list was the CONTENT.** ★★ *A
> scoped `-p`/`--bin` filter exits 0 while silently excluding the binary that
> matters — so build verification must read the OUTPUT's crate list, not the exit
> code.*



**Scored against `ITEM8-V3-SCORING-PROCEDURE.md`, pre-registered before the data.**
Capture: `807297f2db` · log `server-stdout-item8-endurance-v3.log`, 1,123,528 bytes,
stable across three reads.

---

## 0 · ★★★★★★ PREFLIGHT **FAILS** — THE RUN DID NOT TEST THE FIX

    Server version: fb9a7401 [2026-08-11]        <- the log's own boot line

**`fb9a7401` = `fb9a740110`, the colony-presence acceptance build. It is the binary
v2 ran. The crash-fix chain (`e14795700e → … → b6ae81470b`) is entirely AFTER it.**

**Three independent confirmations:**

1. `git merge-base --is-ancestor fb9a740110 e14795700e` → **YES** (ancestor of the fix)
2. That commit's `split_off_one` still returns `bool` and still **pushes**
3. ★ **The log's 13 `ULTIMATE FAIL-SAFE` lines carry `head_clear` and NONE of
   `in_loaded_chunk` / `is_rider` / the membership bits** — which landed at
   `0891889d64`, *before* the fix

> ## **"ZERO PANICS ACROSS 274,200 TICKS, 3× v2's FUSE" IS NOT A FIX RESULT.**

### WHAT PREFLIGHT VOIDS

| item | status |
|---|---|
| the fix claim | ★ **VOID — never under test** |
| the v2/v3 before-after | ★ **VOID — SAME BINARY both times.** No treatment, no control |
| the substitute-precondition salvage | **VOID** — both arms are one arm |
| #85's six-field read | **VOID** — fields absent from the binary |
| `b5_split_off_one_fired` == 0 | *overdetermined — the counter does not exist in this build* |

### ★★★ THE NEW FINDING THAT REPLACES THE HEADLINE

> **THE CRASH IS INTERMITTENT ON THE UNFIXED BINARY: same build, detonated at
> 23.6 min once, survived 154 min once.**

★★ *v4 must justify its run length or repetition count against **observed variance**,
not against a single fuse. The "v2's detonation is a strong precondition" argument
is now known to rest on n=1.*

### PREFLIGHT ITEMS THAT PASSED

**P1 log stable** (3 reads) · **P3 effective config emitted**
(`hunger_decay_per_sec=0.000889`, `hunger_interrupt=0.2`, `rest_*` likewise) ·
**P4 zero clients in the scored window** — *connect 14:47:26, disconnect 14:48:31;
the window opens AT the disconnect, both events at or before it.*

---

## 1 · ✅ COLONY PRESENCE — **CERTIFIED A SECOND TIME, UNDER HARDER CONDITIONS**

    demotions to SimulationMode::Simulated ......... 0
    across .......................................... 274,200 ticks / ~2h34m
    under ........................................... total famine + 331 breakdowns

★★★★ **The binary DOES contain presence — it is the presence-acceptance build — so
this result is valid and earned.** *Presence's own acceptance leg ran ~15 minutes in
a healthy colony; this is 154 minutes through a total food collapse with every
colonist in breakdown, and it did not drop a single colonist.*

**This is the run's one unambiguous PASS.**

---

## 2 · ❌ THE ENDURANCE BAR — **FAILS. TOTAL FAMINE.**

    food stock ....... first 0 · peak 22 · final 0
    samples at zero .. 607 of 914  (66% of the run)
    eats ............. 40      sleeps ........... 48
    breakdowns ....... 331     distinct colonists 8 of 8   (70,71,72,73,74,75,76,77)
    first breakdown .. 15:53:09      last .. 17:20:15
    fail-safes ....... 13

★★ **Every colonist in the colony broke down.** *The colony did not degrade — it
collapsed and stayed collapsed for the last ~66% of the run.*

---

## 3 · ★★★★★★ FAMINE ROOT CAUSE — **MEASURED, AND IT IS TWO POPULATIONS**

### ⚠★★★★★ THE LIFECYCLE OF ALL 87 FARM JOBS — **CORRECTED 2026-08-11 (5b), MY ORIGINAL TABLE WAS WRONG**

    87  created
    59  COMPLETED   -- 19/19 TILL (100%) · 20/20 HARVEST (100%) · 20/48 SOW (42%)
    28  STUCK, ALL SOW, NEVER ENGAGED

★★★★ **VERIFIED STRUCTURALLY, not accepted:** *a cell cannot receive a replacement
until its job leaves the board —* **32 cells got 1 job, 10 cells got 3, 5 cells got
5 (32+30+25 = 87). Fifteen cells cycled TILL→SOW→HARVEST, some twice. Jobs
complete and leave.**

#### THE PRODUCER COLLISION THAT PRODUCED MY WRONG TABLE

    14131  if job.kind.is(DesignationKind::Farm) {
    14143      "bastion: tilled" · 14185 "bastion: sown" · 14221 "bastion: harvested"
    14234      continue;                    <- Farm EXITS here (Gather likewise, 14236)
    14533      "bastion: job completed"     <- generic; Farm can NEVER reach it

| question | producer | answer | evidence? |
|---|---|---|---|
| did Farm emit the GENERIC completion line? | `job completed` | **0/87** *(mine)* | ⛔ **structurally fixed at zero** |
| did Farm's OWN completion arm fire? | `tilled`+`sown`+`harvested` | **59/87** *(5b's)* | ✅ **the real count** |

> ## ★★★★★ **MY 0/87 WAS CORRECT AND USELESS. A measurement whose value is fixed by
> CODE STRUCTURE rather than by run behaviour cannot be evidence about the run.**

**AND IT PROPAGATED.** *My "57 stuck-claimed" was `claimed − (completed ∪ released)`
computed with the generic line — so **all 59 genuinely-completed jobs were counted
as stuck.*** ★★★ **My "57/57 arrived-and-worked" described a population that was
mostly COMPLETED jobs. The correct characterisation is 5b's: 28 stuck, all SOW,
NEVER ENGAGED.**

★★ **RETRACTED: 0/87 · 57-stuck · 57/57-arrived · "completion seam" as first cause.**
*The completion path works.*

### WHY THAT ENDS THE COLONY

**Farm generation is gated on an `occupied` set built from EVERY JOB'S POSITION:**

    // Dedupe: one LIVE job per target cell
    let occupied: HashSet<Vec3<i32>> = board.jobs.values().map(|j| j.pos).collect();

★★★★ **A farm job that never completes and never dies holds its cell forever, so no
replacement is ever generated for it.** *Farm activity: 20 `sown`, 20 `harvested`,
all before **14:54:30** — six minutes into the window — then nothing for 148 minutes.*

> ## **THE COMMENT SAYS "LIVE". THE CODE COMPUTES "EXISTS". A stuck job exists and
> is not live, and the word carrying that distinction appears only in the prose.**

### ★★★★★★ THE FIRST CAUSE — **A 100% → 0% REGRESSION AGAINST WRITTEN PRIOR ART**

**Diffed against `FARM-SOW-PHANTOM-RETIRE-FINDING.md` (row89-green), per Fable's
grep-the-tree-first pointer:**

| | prior art | v3 |
|---|---|---|
| TILL (`sow=false`) created | 48 | 39 |
| ★★★ **TILL COMPLETED** | ★ **48/48 = 100%** | ★★★ **0/39 = ZERO** |
| SOW (`sow=true`) created | 48 | 48 |
| SOW completed | 0 | 0 |
| phantom retires | **48/48 SOW** | ★ **0 — FIXED** |
| `sown` events | 0 | **20** |

★★★★ **TWO REGRESSIONS IN OPPOSITE DIRECTIONS.** *The phantom-retire that once
killed sowing is GONE.* **And TILL completion fell from 100% to 0%.**

> ⚠ ★★★ **THIS CROSS-RUN DIFF IS SUGGESTIVE AND UNMATCHED — the prior-art run was
> SEED-DEADLOCKED (0 sown ever, no competing work); v3 was famine-stressed with 169
> preempts. Different regimes.** *It is retained as context, NOT as the evidence.*

### ★★★★★★ THE FINDING RESTS ON v3's OWN INTERNAL DISCRIMINATOR INSTEAD

    STUCK-CLAIMED farm jobs        : 57
      ... ever ARRIVED-AT + working : 57       <- "colonist arrived at job site, working (B5)"
      ... never arrived             :  0
    baseline: 59 of 87 farm jobs arrived-at · 0 of 87 ever completed

> ## **COLONISTS REACHED AND BEGAN WORKING ALL 57. NOT ONE COMPLETED.**

★★★★ **A WITHIN-RUN contrast — same run, same colonists, same conditions — so the
population is its own control and no cross-run match is required.** *Arrived-and-
worked vs completed is measured, not inferred.*

★★★ **This also REFUTES the preempt-abandonment candidate**: *if colonists were
being pulled away en route, the signature is claimed-but-never-arrived. The
signature is 57/57 ARRIVED.*

> ## **ZERO FARM JOBS OF EITHER PHASE COMPLETED, against a documented baseline where
> TILL completed 100%. The COMPLETION PATH is the first cause.**

★★★ **20 `sown` + 20 `harvested` occurred anyway — the WORK happened, the COMPLETION
was never registered.** *Completion-undetectable, with direct evidence and a
baseline proving it used to work.*

**THE FIX ORDER, BY MEASURED CAUSAL DEPTH:**

    1. COMPLETION SEAM   regressed 100% -> 0%      <- THE ORIGIN
    2. claim expiry      the 57's container        <- the guard
    3. sweep extension   the 27                    <- the closer

★★ *Routes 2 and 3 are consequences of route 1 plus a pre-existing gap it exposed.
Without route 1 nothing accumulates; with it, both other populations are inevitable.*

### ★★★★★ BOTH FRAMINGS WERE TRUE — FOR DIFFERENT POPULATIONS

| population | mechanism |
|---|---|
| **57 jobs** | claimed by a colonist that never completes and never releases — **a claim leak** |
| **27 jobs** | never claimed, and nothing reaps an unclaimed `Designated` — **immortal, unreaped** |

**The famine needs neither alone. It has both.**

---

### ★★★★ THE CANDIDATE TABLE — **the post-mortem's summary artifact**

| candidate | registered by | verdict |
|---|---|---|
| **completion-undetectable** | both | ★★★ **CONFIRMED FIRST CAUSE** — 57/57 arrived+worked, 0 completed |
| **claim leak** | Fable | ★ **RECLASSIFIED: the 57's CONTAINER, not its cause** — claim-held *because* they cannot complete |
| **immortal / unreaped** | me | **TRUE for the 27 never-claimed** |
| **preempt-abandonment** | me | ⛔ **REFUTED** — abandonment's signature is claimed-but-never-arrived; observed 57/57 arrived |
| **phantom-retire-was-completion** | Fable | ⛔ **REFUTED** — item 7's commit is DOC-ONLY; the addendum names phantom-retires as end-of-run disconnect cleanup, never a completion route |

★★ **Two confirmed for their populations · two refuted with their refuters named ·
one reclassified from cause to container.** *No candidate survived unexamined.*

### ⛔ THE SPIRAL RIDER — **died with its engine**

*A self-amplifying famine was proposed (hunger-preempts abandon farm claims → less
food → more preempts). **Preempt-abandonment was its engine; 57/57-arrived removes
it.*** **The cliff-shaped curve needs no feedback loop: a fixed pool of jobs that can
never complete, with food drawn down against zero production, produces it directly.**

## 4 · ★★★★★ PREDICTIONS SCORED — **BOTH OF MINE WRONG**

### 4a · REGISTERED PREDICTION: FALSIFIED

**I predicted the surviving farm jobs would show `claimed_by = None`** *(confirming
immortal-unreaped, refuting Fable's claim-leak framing)*.

> **57 of 60 claimed farm jobs are STILL CLAIMED. The claim-leak framing is right and
> mine is wrong.** *Registered before the data; recorded wrong, as agreed.*

### 4b · ★★★★ AND MY PROPOSED FIX WOULD HAVE BEEN ABSORBED

**I proposed adding `Designated` to the orphan sweep's kind list. The sweep's own
condition is:**

    matches!(j.kind, …) && j.claimed_by.is_none()

> ## **IT ONLY REMOVES UNCLAIMED JOBS. THE 57 ARE CLAIMED. That change fixes 27 of
> 84 and leaves the famine intact.**

★★★ **#60's fully-absorbed fix, arriving again — correct, clean, and outcome-neutral.**
*Caught by measurement before it was built.*

**THE FIX MUST ADDRESS CLAIM LIVENESS: a claim held by a colonist that never
completes has to expire.** *The orphan-sweep extension remains worth doing — it
closes the 27 — but it is the second half, not the fix.*

### 4c · A COINCIDENCE I ALMOST BUILT ON

**`farm job created` = 87 and `claim released` = 87 — exactly equal.** *I called it
"near-conclusive" that the released jobs were the farm jobs.* ★★ **Position-set
comparison: only 5 of 43 released positions are farm cells; 38 are not.**
*A count adjacent to the thing is not the count of the thing.*

---

## 5 · ★★★★★ SENTINEL CALIBRATION — **FREE, FROM EVIDENCE WE ALREADY OWN**

**Predicate v1: "all colonists in breakdown + zero food, sustained N windows."**

    all 8 of 8 colonists in breakdown by ....... 16:02:49
    food at zero ............................... yes, sustained (607/914 samples)
    run actually continued until ............... ~17:22

> ## **THE SENTINEL WOULD HAVE TRIPPED AT ~16:02:49 AND TERMINATED THE RUN **79
> MINUTES EARLY** — destroying the entire sustained-famine record.**

★★★★ **This is the S2-first ruling vindicated with numbers.** *Those 79 minutes are
the only live recording of a colony's failure system under total famine, and they
were preserved by an ad-hoc judgement call. A sentinel shipped action-enabled would
have taken that decision away before anyone knew it was valuable.*

★ **The predicate is not too strict — it fires correctly. The danger is that it
fires EARLY relative to the data's value.** *Stage-1 log-only is mandatory, and this
run is its founding evidence.*

---

## 6 · READ-LIST — **what was actually read, per the raw-commit ruling's condition**

**Patterns that produced findings:**

    "Server version"                      · "ULTIMATE FAIL-SAFE"     · "head_clear"
    "Client connected|disconnected"       · "bastion effective mood config"
    "farm job created job=[0-9]+"         · "job claimed job=[0-9]+"
    "job completed job=[0-9]+"            · "claim released job=[0-9]+"
    "kind=Designated\([A-Za-z]+\)"        · "food_stock=[0-9]+"
    "BREAKDOWN .. colonist=[0-9]+"        · "bastion: (sown|harvested)"
    "demoted to SimulationMode::Simulated"· "colonist promoted to loaded entity"
    "food stock sample"                   · position sets via "x: N, y: N, z: N"

★★★ **PATTERNS THAT FAILED — the most valuable column, and it would be lost by any
filter built from usage alone:**

| pattern | why it failed |
|---|---|
| `NeedCrossed` | **0 hits.** *Crossings are witnessed by `need preempt — {hunger,rest} below interrupt`* |
| `despond.*uid=` | **0 hits.** *Breakdown lines key on `colonist=`, not `uid=`* |
| `colonist=[A-Za-z]+` | **0 hits.** *The value is NUMERIC* |
| `b5_*` (any) | **0 hits.** *Harness-only — never emitted live, in any build* |
| entity-event-log / `PickedUp` | **0 hits.** *In-memory ring buffer, no output file* |

★★ **Checked-and-cleared is information.** *A filter built only from what was USED
would drop every one of these, and the next scorer would not know the check was
ever possible.*

---

## 7 · ★★★ WHAT THIS RUN DOES **NOT** ESTABLISH

1. **Anything about the crash fix.** *It was not in the binary.*
2. **Anything about #85's four gates.** *The discriminating fields were not in the binary.*
3. **That the famine is caused by the fix or its absence.** *The root cause is a code
   read present in BOTH builds.*
4. ⚠ **~~A clean flakiness sample — the 50× log-rate anomaly~~ — RETRACTED, MY ERROR.**

   > ★★★ **I attributed 8.25 MB to v2. That number belongs to the COLONY-PRESENCE
   > ACCEPTANCE leg (script-19), which ran its per-pass diagnostic ON (27,640 diag
   > lines). v2's actual log is 287 KB / ~26 min.**

   **Corrected: v2 ≈ 11 KB/min, v3 ≈ 7 KB/min — same order of magnitude, ~1.5×.**
   *Both runs had the per-pass diag OFF (0 lines each, matching both launch records).*
   **There was no env/diag mismatch to explain, and the v4 blocker built on it lifts.**

   ★★★★ **ONE MISATTRIBUTED NUMBER CAUSED TWO DOWNSTREAM ERRORS:** *the ~52 MB log
   projection that drove the raw-commit sizing ruling (actual: 1.1 MB), and the "50×
   anomaly" that blocked v4.* **A right number from the wrong run — the
   silent-producer pattern, in my own framing, on the same day I wrote "a derived
   figure is a baseline only if you can name every term in its derivation."**
   *(5b caught it.)*

★ **And the epoch-snapshot recovery path is structurally impossible:**
*`b5_split_off_one_fired` does not exist in `fb9a7401`'s `JobBoard`, so no snapshot
of that build can contain it. 779 MB of save data cannot hold a field the build
lacks.*

---

## VERDICT

| result | outcome |
|---|---|
| **colony presence** | ✅ **PASS** — 0 demotions, 154 min, under total famine |
| **endurance bar** | ❌ **FAIL** — total food collapse, 8 of 8 colonists broken down |
| **crash fix** | ⛔ **NOT SCOREABLE** — wrong binary |
| **#85** | ⛔ **NOT SCOREABLE** — fields absent |
| **famine root cause** | ★ **FOUND AND MEASURED** — 57 stuck-claimed + 27 unreaped, zero completions |

★★★★ **v4 requires a verified-rebuilt binary and a mandatory preflight gate: the
log's `Server version` must match the intended pin, checked at minute one.**
