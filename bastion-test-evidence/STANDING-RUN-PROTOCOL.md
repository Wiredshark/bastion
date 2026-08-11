# STANDING RUN PROTOCOL — **EVERY LANE, EVERY LIVE RUN**

★★ **This file exists because a rule that lives in one head protects one head.**
*Each item below was adopted after it had already cost a run, a night, or an
irrecoverable artifact. None is a preference.*

## ★★★★★★★ 1. THE LAUNCH BLOCK — **SET ALL OF IT, EVERY TIME**

```bash
export VELOREN_ASSETS=<worktree>/assets
export BASTION_REQUIRE_EXPLICIT_ASSETS=1     # DET-AST-007: declared root is the ONLY candidate
```

★★★★★ **`BASTION_REQUIRE_EXPLICIT_ASSETS=1` turns "I forgot `VELOREN_ASSETS`" from
a silent wrong-tree load into a PANIC**, and skips the ambient search entirely
(exe dir, cwd repo root, XDG). *DECISIONS #83, amended.*

> ★★★ **WHY IT IS NOT OPTIONAL:** `find_root()` returns the first ancestor
> containing `.git`. **26 of this repo's 52 worktrees are nested under
> `E:/veloren-master`**, whose own tree is usually behind. **Launching from the
> parent cwd silently serves a different tree's assets** — a clean boot, a valid
> RON, and last week's tuning. *It cost a 25-minute live run and four hours.*

★ **Every live report quotes the banner** — `Certified asset root path=` **or**
`Assets found path=`, whichever fired. **A live result is uninterpretable without
it.**

### ★★★★★★★★ 1a. **A LIVE LOG STAMPS ITS OWN CODE IDENTITY AT BOOT** (Fable-ruled 2026-08-10)

> ## ★★★★★★★ **A DATE THAT LINES UP IS A COINCIDENCE UNTIL SOMETHING IN THE
> ARTIFACT NAMES THE CODE THAT PRODUCED IT.**

**The case that bought this.** A live log was offered as the item-3 sit-trap run:
right directory, right naming convention, right date (2026-08-04). **Three
consistent identifiers.** ★★★★★ **One grep settled it —
`grep -c "GOTO-STAND-RESCUE"` returned `0`, and the fix that CREATES that counter
landed days later.** *The date did not corroborate the identification; it refuted
it, and nobody could see that from the filename.*

★★★★ **So: a live run's log OPENS with its commit**, exactly as the fan banner now
does (`#83`) and as the flight recorder's `SOURCE_HEAD` fields already provide.
**One line at boot turns "is this the right run?" from a dating argument into a
read.**

★★★ **This is the last unstamped artifact class we produce.** *Harness JSON
carries `b5_build_stamp`; fan logs carry `COMMIT=` per VM and the effective config
in the banner; live logs carry neither.*

★★ **AND THE CONSUMING-SIDE RULE, which applies before the stamp exists:** *when
identifying an artifact, find the field that can only be true of the run you mean
— usually the presence of a counter, field, or string that a specific commit
introduced.* **Filename, directory, and timestamp are all consistent with runs
they did not produce.** ★ Sibling of [[a-number-must-carry-its-producer]] read
from the other end: **a RUN must carry its producer too.**

### ★★★★★★★ PROVISIONING VIA `dropall` IS A **5-MINUTE WINDOW**

**`/dropall` hardcodes `persistent: false`, and `state_ext.rs:385-388` gives every
non-persistent drop `Object::DeleteAfter { timeout: Duration::from_secs(300) }`.**

> ## ★★★★★ **DROPPED ITEMS DESPAWN AFTER FIVE MINUTES.** **A live arm whose need
> crossing lands at ~30 minutes will find NOTHING**, no matter how much was
> dropped, unless a drop happens inside the five minutes before the crossing.

★★★ **Measured 2026-08-09 (run-51):** *a 40-minute provisioned arm dropped food
twice and still logged **`no_food_found` × 21,176 with exactly ONE successful
`EatFrom`** — and that one landed almost exactly at the resupply's timing.* **The
timing correlation is the discriminator: it points at DESPAWN, not at supply
volume or a search defect.**

★★ **So "provisioned" is a claim about a MOMENT, not about the run.** **State
WHEN the food existed relative to the crossing**, or use a persistent source
*(stockpile / farm)* rather than `dropall`.

★ *Three plausible mechanisms were refuted before this one — food-def rejection,
scenario-never-provisions, reservation-leak-on-removal — and a fourth (one
stacked entity holding one reservation) was refuted by the same timing evidence.*
**The item PRODUCER (`dropall`) was the last thing anyone read; the search was
read first and was correct all along.**

### ★★★ AND THE ADMIN STEP — `--no-auth` GOES ON THE SUBCOMMAND

```bash
server-cli --no-auth admin add <name>      # BEFORE boot
```

★★ **Without `--no-auth` on the SUBCOMMAND the auth-server UUID lookup fails**
*(found 2026-08-09 setting up the provisioned arm; caught pre-boot, but it is a
silent-until-you-need-it failure and the flag's position is not guessable)*.
★ **Do it before boot, not after.**

### ★★★★★★★★ 1b. **READ THE SCRIPT'S OWN HEADER BEFORE LAUNCHING IT — EVERY TIME**

**Added 2026-08-10 after TWO instances in one hour of "the answer was written down
and nobody read it."**

★★★★ **A run failed because admin was granted AFTER boot; the server reads
`admins.ron` once at startup and never again.** ★★★ **Every prior script's header
comment already said so.** *In the same hour, a six-read investigation re-derived —
and inverted — a finding sitting in `bastion-test-evidence/live-playthrough/FARM-DIAGNOSIS.md`.*

> ## ★★★★★ **OUR ARTIFACTS CARRY THEIR OWN PRECONDITIONS AND WE TREAT THEM AS
> REFERENCE RATHER THAN CHECKLIST.** **The script header is the shortest document in
> the loop and the one most likely to name the thing about to go wrong.**

★★ **Companion rule, same family: before investigating any symptom, grep the
EVIDENCE TREE for the symptom's own log line — not just the code.** *Our diagnoses
quote the strings they explain, which makes the evidence tree a full-text index of
everything already understood.*

### ★★★★★★★★ 1c. **A "NOTHING HAPPENED" BAR NEEDS A POSITIVE CONTROL**

**An acceptance run asserting *"ambient NPCs did not take the pile"* was one
permission error away from passing on an EMPTY WORLD:**

    give_item denied -> no items exist -> no pickups occur -> "zero ambient pickups" -> GREEN

> ## ★★★★★ **A PRECONDITION FAILURE IN A DON'T-DO-X TEST PRODUCES A CONFIDENT PASS,
> NOT A NULL.** *Nothing in the run is false; the bar is satisfied by absence.*

★★★★ **MECHANICAL: every "X did not happen" assertion carries a positive control
proving X WAS POSSIBLE.** *Here: assert the pile EXISTS, and let a legitimate
colonist pickup in the same run prove the path was live while the refusals were
counted.* ★★ **Zero events of ANY kind reads VOID, never PASS.**

## ★★★★★ 2. DIAGS — **ANOMALOUS RUNS ARE BY DEFINITION THE UNINSTRUMENTED ONES**

**Always-on, no flag needed:** the effective-config emit and `preempt_attempts`.

**Set on any run that could become evidence:**

```bash
BASTION_DECAY_JOIN_DIAG=1  BASTION_NEED_SKIP_DIAG=1  BASTION_NEED_LOAD_FILTER_DIAG=1
```

> ★★★★★ **A flag-gated instrument recreates the gap it was built to close, because
> the run you most need it on is the one nobody predicted.** *driver-12 did not
> have the flags. Neither will the next anomaly.*

★★★ **Before commissioning ANY new emit, enumerate the ones that exist:**
`python bastion-test-evidence/env_surface.py <ref>`. **SIX times in one
investigation the instrument already existed and was merely absent from the path
under test** *(one of them authored by the person asking, twenty minutes
earlier)*. *See `BASTION-ENV-SURFACE-CATALOGUE.md`.*

### ★★★★★★★ AND FROM DECISIONS #87 — **76 ASSERTIONS ARE NOW LIVE IN YOUR RUN**

**`[profile.no_overflow]` sets `debug-assertions = true`.** *Live `server-cli`
builds come from that profile, so **every `debug_assert!` in the engine — 76
sites, 9 of them in `bastion_jobs.rs` — is now armed during a live run.***

> ## ★★★★★ **A FIRING `debug_assert!` MID-RUN IS A PANIC — i.e. A LOST 40-MINUTE
> ARM.** **Run the survey pass BEFORE any further live run**, not merely before
> the gate default. *Any assert that fires is a pre-existing violation and its own
> finding, not a blocker.*

★★★ **AND IT CLOSES A MEASUREMENT SERIES:** *assertions cost throughput, so any
overage or tick-coverage figure taken before this change is **not comparable**
with one taken after.* **Treat the next live run's coverage as a NEW BASELINE** —
*a parked question whose reactivation condition is "the same signature again" must
be re-read against the new numbers, or it manufactures a phantom regression.*

## ★★★★★★★ 3. SIZING — **QUOTE THE BUDGET, NEVER RECALL IT**

**At shipped rates, from full meters:**

| need | crosses at | ticks @30 tps |
|---|--:|--:|
| hunger | 900 sim-sec | **27,000** |
| rest | 1800 sim-sec | **54,000** |

★★★★★ **A run shorter than the threshold it means to observe produces an ABSENCE
that looks exactly like a failure.** *This voided one 25-minute leg and nearly
condemned a correct fix on a 40-minute one.*

**Also state:** the **sim : wall ratio** (dt is FIXED at 1/30 per tick, so sim
time tracks TICKS EXECUTED, not wall time), and the **starting meter values** —
`Needs` are **restored from the save** at promotion, not necessarily `1.0`.

### ★★★★★★★ AND REPORT THE **SERVER'S** FINAL TICK AGAINST BUDGET

> ## ★★★★★ **"SCRIPT COMPLETE" IS A STATEMENT ABOUT THE DRIVER, NOT THE SERVER.**

**`bastion_playtest`'s `Wait(n)` calls `client.tick()` n times paced by the
CLIENT's own clock and never polls confirmed server tick state.** ★★★ **Measured
2026-08-09 under contention:** *driver "script complete" and server
`Client disconnected!` agreed within **52 ms** — and the server's authoritative
counter read **61,756 of 72,300 = 85.4% of budget**, ~352 sim-sec short.*

★★★★★ **A run can report itself complete while the server simulated 34 of the
40 minutes.** **Score coverage from the server's own tick field** *(present in
`BASTION_DECAY_JOIN_DIAG` / `BASTION_NEED_LOAD_FILTER_DIAG`)* **against the
intended budget — never from the driver's completion alone.**

★★ **This invalidates a CLAIM CLASS, retroactively:** every prior *"ran the full
budget"* resting on driver-side completion is **UNPROVEN** *(not wrong —
unproven)*. **A figure derived from wall-vs-nominal is driver-derived and must
not be compared against a server-derived one** — *see
**a number must carry its producer**.*

★ **Derived overage is not a substitute:** `wall − (wall − nominal)` is an
identity, not a measurement. **If `C := A − B`, then `A − C = B` confirms
nothing.**

## ★★★★★★★★ 4. EVIDENCE RETENTION — **FIVE CLAUSES, IN THIS ORDER**

> **(a0) write it where it will be cited from · (a1) stop the writer before you
> read · (a) cited means copied · (b) delete by name · (c) name the cost first**
>
> ★★ *Was "THE TRIO" until 2026-08-10, when (a0) and (a1) were added the same
> night. Renamed because a stale label is the defect this file's own §5 is about —
> and a reader who trusts the heading counts three and stops.*

> ### ★★★★★★★★ **(a0) WRITE IT WHERE IT WILL BE CITED FROM. — ADDED 2026-08-09, AFTER THE THIRD LOSS IN ONE SESSION.**
> ## **THE LOG PATH GOES IN THE LAUNCH COMMAND, POINTING AT
> `bastion-test-evidence/<row>/`. NOT A TEMP TREE. NOT `/tmp`. NOT THE CWD.**
>
>     ... > bastion-test-evidence/<row>/run.log 2>&1
>
> ★★★★★ **Rule (a) below says copy it AT REPORT TIME — and report time is AFTER
> the analysis.** **That leaves a window in which the only copy sits somewhere
> sweepable, and an analysis that spans hours or a session boundary can outlive
> its own evidence.** ★★★ **Three raw logs died in that window in a single
> session, including run-51's, whose absence forced a three-hop unit conversion
> to answer a question one `grep` would have closed.**
>
> ★★★★ **The window is not a discipline problem. It is a DEFAULT problem — and
> writing to the destination removes it entirely.** *No copy step to forget, no
> path to pin from memory, no sweep to lose a race with.*
>
> ★★ **`2>&1` is not optional: tracing writes to STDERR.** *A redirect that
> captures only stdout keeps the JSON and discards every diagnostic — the same
> defect that makes the VM fan structurally unable to carry a log line
> ([[vm-corpus-fan-ownership]]).*
>
> ★ **A run whose log is absent is VOID — never "it ran and we lost the detail."**

> ### ★★★★★★★★ **(a1) STOP THE WRITER BEFORE YOU READ THE FILE. — ADDED 2026-08-09, SAME NIGHT AS (a0).**
> ## **AN EVIDENCE FILE WITH A LIVE WRITER IS NOT FINAL, AND A "FINAL TICK" READ
> ## FROM ONE IS A SNAPSHOT, NOT AN ENDPOINT.**
>
> ★★★★ **(a0) above closed the LOSS hazard and left the GROWTH hazard wide open.**
> *A file that survives can still be unfinished — and a still-running server keeps
> appending to the log you are citing.*
>
> **MEASURED INSTANCE, same night:** *a row's server ran ~2 h past evidence capture.
> The log grew **2.28 MB / 9,251 lines** between the analysis and the re-read.*
> ★★★ **The run's own "final tick 76,153 = 105.3% of budget" was itself a live
> snapshot; the true final tick was 204,388 — 283% of budget.** **Every "% of
> budget" figure taken from a running server is meaningless.**
>
> ★★★★★ **MECHANICAL — three steps, in this order:**
>
>     1. stop the server explicitly (the run is not over until the writer is dead)
>     2. THEN record byte size and line count
>     3. cite those numbers in the writeup
>
> ★★★ **AND WHEN A LOG MAY HAVE GROWN, RE-MEASURE PER STREAM — DO NOT ASSUME EITHER
> WAY.** *In the measured instance the conclusions SURVIVED: every stream that
> mattered was byte-identical and the 9,251 new lines were all one unrelated diag.*
> **The check cost two minutes and converted "possibly invalid" into "bounded".**
>
> ★★ **What made the re-check possible was having published EXACT COUNTS.** *A
> summary with round numbers or prose would have been unfalsifiable — the precision
> was what let a re-run of the same measurements settle it in one pass.*

> ### ★★★★★ **(a) CITED MEANS COPIED.** *(now the fallback, not the primary)*
> **A raw log stops being scratch the moment a ledger row cites it.** If it was
> written elsewhere anyway, copy it to `bastion-test-evidence/<row>/` at report
> time. **Too large? Commit its byte count, line count, and the extracted counts
> the conclusion rests on.**

> ### ★★★★★★★ **(b) DELETE BY NAME — ONE PATH AT A TIME.**
> **Never a brace expansion, never a glob, never a loop.** *`{9,9diag,10..14}` is
> "everything that isn't mine" wearing an index. It destroyed six runs' raw
> evidence including the only calibrator `logcount.py` had.*

> ### ★★★ **(c) NAME THE COST BEFORE DELETING.**
> *That deletion killed a queued falsifier and a tool's only known-positive —
> neither noticed until after.*

### ★★★★★★★ AND FOR ANY FREEZE OR QUIET-WINDOW YOU ASK ANOTHER LANE TO HOLD

> ## ★★★★★ **A HOLD INSTRUCTION STATES ITS RELEASING EVENT, NEVER A DURATION.**
> **A duration is a guess the holder cannot audit. An event is a fact they can
> check.**

> ### ★★★★★★★★ **COMPLEMENT — ADDED 2026-08-10 AFTER THIS RULE CAUSED A 13-HOUR STALL**
> ## **THE EVENT BOUNDS THE WAIT; A LIVENESS DEADLINE BOUNDS THE SILENCE.**
>
> ★★★★★ **An event-bounded wait has NO liveness check. If the producer dies,
> silences, or was never started, FOREVER reads identically to THIRTY SECONDS.**
>
> **MEASURED, twice in one night:** *(1) a finished run whose report never came —
> 13 h passed, server running the whole time; (2) a run that had NEVER STARTED —
> a scheduled wakeup silently failed, the server sat idle to tick 1,419,300 with
> zero colony spawn and no `driver.log` on disk.*
>
> > ★★★★ **SO THE LIVENESS PING'S FIRST QUESTION IS NOT "IS IT DONE?" BUT
> > "DID IT START?"** — *existence before progress, the same ordering as every
> > other law here.*
>
> ★★★ **MECHANICAL: at wait time, name (a) the releasing EVENT and (b) the ONE
> ARTIFACT whose existence proves the producer started** *(a `driver.log`, a spawn
> line, a nonzero attempt counter)*. **Then the liveness check is a single
> existence test, not a conversation — it costs nothing and cannot preempt the
> producer's work.**
>
> ★★ **These two rules are COMPLEMENTS, never substitutes.** *The event-discipline
> silently overrode the older ~10-minute ping rule, which would have caught both.*

★★★ **Worked instance, 2026-08-09:** *I told the builder "hold commits ~20
minutes" for a corpus fan.* **20 minutes was the FAN's runtime; the actual
constraint was the split-brain window — each VM runs
`git fetch && git reset --hard origin/$BRANCH` once at its own boot, and is
pinned the moment it prints `COMMIT=`.**

    bastion-pool-{0,1,2,3}.log : COMMIT=156a2ece      <- the releasing event
    observed hold: ~4 minutes, not the 20 quoted

★★ **The builder had no way to tell the difference and would have held ~16
unnecessary minutes on my word** — *a builder queued on a reviewer's unfalsifiable
bound is the stalled shape the never-stop rule exists to prevent.*

★ **Same family as a count carrying its producer, and it generalises to every
freeze, quiet-window, or "don't touch the box" instruction any lane issues:**
**say what has to become TRUE, not how long you think it will take.**

### ★★★★★★★ RECOVERY — **BEFORE DECLARING A TIMESTAMP UNRECOVERABLE, READ THE OTHER LANE'S TRANSCRIPT**

> ## ★★★★★ **CROSS-SESSION MESSAGES ARE TIMESTAMPED ON THE *RECEIVING* SIDE.**
> **A lane that loses its own history can often recover event times from its
> correspondent's log** — *the message that reported the event is itself a dated
> artifact.*

★★★ **Proven 2026-08-09:** a 40-minute run's start and end were declared lost
with its deleted log **and both were recovered from the reviewer's session file
in one grep** — the builder's *"Running now…"* and *"Falsifier triggered…"*
messages carried real timestamps on the receiving side.

★★ **CAUTION, learned the same minute:** the first search matched `45.6` against
raw lines and hit **193 timestamp fractions** (`04:41:45.605Z`) with zero signal.
**Anchor the pattern** — `45\.6\s*min` found it immediately. *A count from a
loose pattern is not a measurement.*

★★★★★ **AND THE RULE THAT ACTUALLY SAVED US:** **write the report so the raw log
is NOT load-bearing.** *Byte counts, derivations, scored bars and histograms go
IN the committed document.* **Total evidence loss then costs three regenerable
reads instead of the night — which is exactly what happened.**

## ★★★★★ 5. READING A ZERO

> ## **A ZERO COUNTS AS EVIDENCE ONLY FROM A CHANNEL PROVEN REACHABLE.**

**Four ways a channel goes mute, all rendering as a confident zero:**

| | mute mode | check |
|---|---|---|
| **1** | **STRUCTURAL** — the emit sits in a branch this scenario excludes | evaluate the emission condition under the scenario's own constraints |
| **2** | **ENCODING** — non-ASCII literal vs a codepage-blind reader | ★★★ **score the pattern on a KNOWN-POSITIVE first** |
| **3** | **GATED OFF** — env gate unset | diff the run's env against §2 |
| **4** | **FILTERED UPSTREAM OF THE EMITTER** — the loop's entry filter is a skip it cannot report | a skip diagnostic must emit at its own entry filter |

★★★★★ **Count with `python bastion-test-evidence/logcount.py`**, never a bare
grep — **it refuses to report a zero whose encoding it cannot prove.**

> ★★★ **THE CALIBRATOR PATH IS WORKTREE-RELATIVE — GIVE IT ABSOLUTE.**
> *`run15-extract.log` lives under `.engine-integration-wt/`, i.e. in ONE of this
> repo's 52 worktrees.* **A bare `calibrators/run15-extract.log` resolves for
> whoever committed it and fails for everyone else** — *the wrong-tree class, in
> the artifact built to prevent artifacts.*

★★ **AND A CALIBRATOR PROVES ENCODING, NOT PRESENCE.** *A pattern legitimately
absent from the known-positive is still REPORTABLE, so long as some canary covers
its non-ASCII characters.* ★ **Do not read "0 on the known-positive" as a
calibration failure** — *`BREAKDOWN —` scoring at all is what licenses every
other U+2014 pattern; pure-ASCII patterns never needed a canary.*

★★ **Strip ANSI first: tracing colour codes sit INSIDE field values**
(`\x1b[3mtick\x1b[0m\x1b[2m=\x1b[0m<N>`, not `tick=`). `logcount.py` does this.

## ★★★ 6. BUILDS

★★★★★ **Never pipe a build.** *`cargo … | tail` reports **tail's** exit status —
and the task notification inherits it, so "completed, exit 0" can mean cargo
failed outright.* **Cost: one full run against a stale binary.**

★ **Verify the binary is fresh** — file timestamp or `--print-git-hash`, not the
notification. ★★ **`RUSTC_WRAPPER=""` on any verification build** *(sccache
serves stale objects user-globally)*. ★★★★★ **Cap at `-j 8` shared / `-j 16`
uncontended** — *the box is **16 logical / 8 physical**; the old `-j 48` was 6×
physical over-subscription* *(measured: `nproc`, `Win32_ComputerSystem.NumberOfLogicalProcessors`, and `Win32_Processor.NumberOfCores` all agree)*.

## ★★★★★★★ 7. SUITE GATES — **A GATE MUST NOT GO RED FOR A REASON THE CODE DIDN'T CAUSE**

### ★★★★★ 7a. A TEST DECLARES WHAT ITS SUBJECT REQUIRES

| subject | must declare |
|---|---|
| ★★★ a **`debug_assert!`** | ★★★★★ **the PROFILE** — `#[cfg(debug_assertions)]`. *`no_overflow` and `release` compile the assertion away, so the test fails for a reason the code did not cause* |
| ★★★ a **wall-clock bound** | ★★★★★ **WHICH RESOURCE it measures, and the quiet-host precondition for THAT resource** |

### ★★★★★★★ 7b. NAME THE RESOURCE, NOT "LOAD"

**Worked example — `t4_1_content_live_real_asset_tree_walk_completes_in_bounded_time`:**

```rust
assert!(elapsed.as_secs() < 30, "asset-tree content walk took {elapsed:?} …");
```

★★ **Its own doc: *"30 seconds is deliberately generous (this row's own measured
run was well under that)."*** ★★★ **It is a DISK TREE WALK — I/O bound, not
CPU-parallel.**

> ## ★★★★★ **SO A CORE-COUNT MIS-CALIBRATION CANNOT EXPLAIN IT, AND CPU
> CONTENTION WOULD NOT FLAKE IT.** **What flakes it is another process walking
> the same disk** — *a concurrent `cargo`, or a full-tree `find` (one of which I
> ran during this session's build churn).*

★★★★★ **AND THAT CHANGES THE DISPOSITION: do NOT quarantine it.** *A 30-second
bound on a walk that normally runs far under it does not flake from marginal
calibration — it flakes because the disk was genuinely saturated.* ★★★ **The test
is working: it reported a real condition, just not a code regression.**
**Quarantining it deletes a true signal about the host.**

> ★★ **"It flaked" is a claim about the RUN, not about the test.** *Establish
> which resource the bound measures before deciding the bound is wrong — the
> answer decides whether you fix the test, fix the host, or leave both alone.*

### ★★★ 7c. PROFILE THE GATE ACTUALLY RUNS UNDER

**`veloren-common`'s suite runs under `dev` at tag cadence.** ★★★★★ **A suite gate
under a profile that deletes the assertions closes nothing** — *76 `debug_assert!`
sites, 9 of them in `bastion_jobs.rs`, are compiled out under `no_overflow`
(DECISIONS #87 sets `debug-assertions = true` there, keeping `overflow-checks =
false` EXPLICIT so the profile keeps its name-contract).*
