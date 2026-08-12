# ITEM 8 v5 — **PACKET**

**Supersedes v4's bar. Every judgement below is registered BEFORE the run exists.**

*Predecessors: `ITEM8-V4-PACKET.md`, `ITEM8-V4-RESULTS.md`,
`ROW-INDESTRUCTIBLE-MINE-CELL.md`, `ROW-TIME-COMPRESSION-EQUIVALENCE-SPEC.md`.*

---

## 0 · ★★★★★ WHY v5 IS DIFFERENT FROM v3 AND v4

**v3 and v4 both scored EXACTLY 59 farm completions — 19 tilled · 20 sown · 20
harvested — and both starved, by DIFFERENT fatal mechanisms.**

★★★★★★ **The reason neither fix moved the number is now read, not guessed: the
labour force's completion channel was a single colonist's egress loop mining one
indestructible block 281 times.** *Routes 2 and 3 were correct fixes for real defects
that were never what stopped the farm.*

> ## **v5 IS THE FIRST RUN WHERE WAVE TWO GETS TO EVEN ATTEMPT.**

*Either generation-2 sows fire and Arc 1 closes, or a third stopper shows itself —
and for the first time the bar can tell those apart.*

---

## 1 · ⛔ LAUNCH PRECONDITIONS — **ALL FOUR, OR THE RUN DOES NOT START**

| # | precondition | why |
|---|---|---|
| **1** | ★★★ **GATE 0**: the running binary's stamp is read FROM THE LOG and matched to the intended pin | *v3 was voided by a six-commit-stale binary certified by commit-diff* |
| **2** | **THE MINE FIX** — defect 2 (egress requests terminate) landed, with its planted test green **and its control arm green** | *the actual thing under test* |
| **3** | ★★★★ **THE THREE LOG FIELDS** — `kind` on arrival · an emit on the material-stall path (`kind` + `pos` + `required_item`) · `completed_kind` on completion | *without these v5 reproduces v4's blind spot at full wall-clock cost* |
| **4** | **RUN MODE DECLARED** — real time, impossibility named (§2) | *checklist entry 6* |
| **5** | ★★★★★ **`BASTION_EGRESS_DIAG=1`** — the `watch_wipe` reason trace | *§1b: the discriminator was OFF for all of v4* |
| **6** | ★★★★★★ **THE `watch_wipe` GATE** — the completion wipe carries the same world-effect predicate as the `job completed` emit | *§1b: otherwise the log is honest and the backstop stays disarmed* |

### ★★★★★★ 1b · THE PHANTOM COMPLETIONS DISARMED THE RESCUE BACKSTOP

**Cited by symbol in `bastion_jobs.rs`:**

    watch_wipe(&mut board.stuck_watch, u, "job-completed")
    const STUCK_TELEPORT_SECS: f32 = 60.0;

**owner=80 completed a job every ~15 seconds for 2.5 hours.**

> ## **EVERY PHANTOM COMPLETION WIPED THE STUCK CLOCK. IT NEVER REACHED 20 OF THE 60
> SECONDS IT NEEDED. THE TELEPORT COULD NOT FIRE — NOT ONCE.**

★★★★★ **So the named consumer for an unrescued colonist EXISTS and is structurally
defeated by this exact defect class.** *Arrive → complete → wipe → repeat. To the
watchdog, the trapped colonist looked like the most productive member of the colony.*

**THE SAME FALSE SIGNAL HAD THREE VICTIMS:** *the health metric, the safety net, and —
because `watch_wipe` only emits under `BASTION_EGRESS_DIAG` — the investigation
itself.* ★★★ **"stuck_watch wiped" appears nowhere in v4's 945K lines. That absence
was an EXCLUSION, not a finding.**

★★★★★★ **And the shim's own doc comment describes this case in advance:** *"when a
colonist who should backstop never does, the wipe-reason trace is the discriminator
(the F5 pit-B investigation: 200s below grade, no teleport, no way to see why)."*
**It happened again, for 2.5 hours, and the discriminator was switched off.**

★★ **Precondition 3 is not a nicety.** *v4's deciding read did not exist in its own
capture: the arrival line carried no `kind`, the stall path emitted nothing at all, and
the completion line never logged the block it removed.*

---

## 2 · ★★★★★ RUN MODE — **REAL TIME, IMPOSSIBILITY NAMED AND PROVEN**

**Checklist entry 6 admits three impossibilities. This run invokes number 2, A PROVEN
WALL-COUPLED SUBSYSTEM — "and proven means proven; a suspicion of wall-coupling is
not an impossibility, it is an unread."**

**IT IS NOW READ:**

    capped        colonist promotion complete @ tick  624
    capped-ctrl   promotion complete @ tick  192
    uncapped      promotion complete @ tick 2184

*Promotion `Simulated`→`Loaded` gates on `chunk_states` — real background chunk
generation, bounded by wall seconds — at **two** sites in `server/src/rtsim/tick.rs`.*

> ## **COMPRESSION CHANGES THE LOADED/SIMULATED RATIO, AND ITEM 8 IS THE ROW THAT
> STUDIES LOADED-COLONIST BEHAVIOUR. A COMPRESSED v5 WOULD CERTIFY A DIFFERENT GAME
> THAN THE ONE PLAYERS RUN.**

★★★ **This SATISFIES Ben's fast-mode law through its own named clause — it does not
except it.** *Every other run in the programme still goes compressed once the N=8
promotion-distribution test certifies it.*

⚠ **The promotion numbers remain a CANDIDATE at n=1 uncapped.** *They are sufficient to
name the impossibility; they are not yet the certification. That is the N=8 test's job.*

---

## 3 · ★★★★★★ THE BAR

### F1 — **GENERATION-2 COMPLETIONS > 0** *(Fable, registered pre-run)*

> ## **SOWS CONTINUING PAST THE FIRST SEED WAVE. NOT "COMPLETIONS > 0".**

★★★★ **"Completions > 0" is satisfiable by a dead colony — v4 scored 59 and
starved.** *That is the count-vs-mechanism error at the BAR tier: a number a broken
system reliably produces.* **Generation-2 fails for the impostor and passes only for
the mechanism.**

### THE REST

| | bar |
|---|---|
| **F2** | no immortal jobs |
| **F3** | cells recycle *(and the reap count is not itself the defect)* |
| **F5** | targeted release fires |
| **F6** | leak backstop silent |
| ★★★ **F7 (NEW)** | **no single position accounts for >10% of completions** |
| ★★★ **F8 (NEW)** | **`job completed` fires ONLY for completions with a world-effect** |
| **S1** | sentinel: log-only |

### ★★★★★ LAUNCH-TIME AMENDMENTS — **registered PRE-DATA, 2026-08-11**

*The builder added F9 and F10 in the launch record. Both are reviewed here and one is
demoted, before any data exists.*

**F9 — `emergency_access_completions`, "expected nonzero, benign" → ⛔ DEMOTED TO A
MEASURE.**

> ## **NO OBSERVATION WAS NAMED THAT WOULD MAKE IT RED. A BAR THAT CANNOT FAIL IS NOT
> A BAR.**

★★★★★★ *Tonight's lesson arriving at the bar tier: four instrument vacuities were
caught, and the fifth walked in as an acceptance criterion.* **Entry 7's head question
applies to bars exactly as to instruments.** *Reported, never scored — unless someone
registers a bound, at which point it becomes a bar again.*

**F10 — mine-cell repeat-completion → ✅ KEPT, BOUND TO F7's THRESHOLD.**

*"A handful is normal, hundreds is defect 1 recurring" carries no registered number,
and at hour 2.5 "a handful" is negotiable.* ★★★ **F10 inherits F7's bar: no single
position >10% of completions.** **v4 would have failed it at 98.6% (143/145) — the
threshold is calibrated by a real specimen, not a guess.**

**F7 and F8 exist because v4 passed nothing and still looked busy.** *143 of 145
late-run completions at one cell; 361 completions with zero drops, zero XP, zero
cave-ins.*

---

## 3b · ★★★★★★ THE REGISTERED v4 BASELINE CURVE — **extracted PRE-TEARDOWN**

**Every `food_stock` value in v4's capture (`1bedd79602`), all three parts:**

| part | values seen | count |
|---|---|---|
| **000** | 0 · 10 · 12 · 14 · **18** | 308 · 1 · 5 · 9 · **62** |
| **001** | ⛔ **0 only** | **265** |
| **002** | ⛔ **0 only** | **252** |

> ## **v4's FOOD STOCK PEAKED AT 18, THEN SAT AT ZERO FOR 517 CONSECUTIVE SAMPLES —
> THE ENTIRE LAST TWO THIRDS OF THE RUN.**

★★★ *This confirms and refines the "19-ish" figure in circulation: the peak was **18**,
and — more important than the peak — **the curve is terminal, not declining**.*

### ★★★★★ THE SCORING CONSEQUENCE — **registered before v5's teardown**

**v5 showed `food_stock=67` at 40 min: 3.7× v4's ALL-TIME peak.** ⚠ **That is not the
test.**

> ## **THE DECIDING SHAPE IS WHETHER v5's FOOD RETURNS TO ZERO AND STAYS THERE. A HIGH
> PEAK FOLLOWED BY A TERMINAL ZERO STREAK IS v4's CURVE WITH A BIGGER NUMBER ON IT.**

**REGISTERED MEASURE:** *v5's longest terminal zero-streak in `food_stock`, and whether
the run ends inside one.* ★★ *Compared against v4's 517.*

### ★★★★★★ AND S1 WAS RIGHT ALL ALONG — **a third consumer with the signal and no authority**

**`colony_terminal_zero_streak_samples=10` is the configured sentinel. v4 ran 517
consecutive zeros and S1 fired 3 times — LOG-ONLY.**

> ## **THE SENTINEL DETECTED THE FAMINE IN REAL TIME, CORRECTLY, AND NOTHING ACTED ON
> IT BECAUSE IT WAS NOT A SCORED BAR.**

★★★★ *Same family as the metric that lied up and the watchdog that was disarmed — but
the opposite failure: **this instrument told the truth and had no consumer.*** **Its
calibration now has a full curve behind it, not just a firing count.**

---

## 3c · ★★★★★★ THE FAIL-SAFE RATE — **TWO READINGS, REGISTERED PRE-TEARDOWN**

**14 teleport rescues at T+41, against v3's 4 in 75 minutes.** *A higher rescue rate
than any prior run.*

> ## **A FAILSAFE FIRING MORE OFTEN IS NOT UNAMBIGUOUSLY GOOD NEWS.**

| reading | what it means | signature |
|---|---|---|
| **A — BENIGN TRAFFIC** | *the colony does ~6× more work, so more incidental sticking* | ★★ **rescues DISTRIBUTE across many uids** |
| **B — A RESCUE LOOP** | *one colonist teleported, walks back to the same trap, sticks again* | ★★★★ **rescues CLUSTER on few uids** |

★★★★★★ **READING B IS THE SERIOUS ONE: a rescue that doesn't STICK is the
phantom-completion loop one level up — the net papering over a live trap instead of
ending it.** *And the first two rescues observed were **both `uid=131`**, which is a
clustering signal, not a distributing one.*

### THE DISCRIMINATOR

**Group the teleport lines by `uid`.** *Also carry `active_job_is_access` and the job
kind — both are already on the line.*

### ★★★★ A THIRD DISCRIMINATOR — **THE RATE PROFILE**, registered mid-run

    T+30:  2      T+41: 14      T+64: 23      T+96: 39
    rate:  0.07/min    1.09/min      0.39/min      0.50/min

> ## ⚠ **THE n=3 READING IS WITHDRAWN. AT n=4 THIS IS "BURST, THEN STEADY AT
> ~0.45/min" — NOT A DECLINE.**

★★★★★ **I registered "burst-then-decline" at n=3 and the fourth sample refutes it.**
*Withdrawing my own registered channel rather than re-describing it: 0.39 → 0.50 is not
a decline continuing, it is a plateau.*

★★★ **AND THE CHANNEL IS NOW WEAKER THAN I CLAIMED, NOT STRONGER.** *A steady rate is
consistent with BOTH readings — ongoing traffic from a working colony, **and** a
persistent condition being repeatedly papered over.* **It no longer discriminates.**
*The uid split is the discriminator; the rate was a hopeful third and it did not hold.*

⚠ **AND THE COST, STATED:** *23 rescues × 60 s of prior stuck-time ≈ **23 colonist-
minutes lost** out of ~512 (8 colonists × 64 min) — **~4.5% of total labour**.* **Not
catastrophic; not nothing. It belongs in the results whatever the uid split says.**

★★ **Note also: `emergency_access_completions` stayed FLAT at 6 while rescues went
2 → 23.** *So the rescues are NOT generating egress work — consistent with BOTH
readings (stuck-on-pathing vs a rescue that doesn't stick), which is why it is not
itself a discriminator.*

    many uids, few each      -> reading A, benign
    few uids, many each      -> reading B, the trap survived the fix

### ★★★★★ THREE REGISTERED READS THAT TRIANGULATE ONE QUESTION

**Filed together because they converge, and any two agreeing constrains the third:**

| read | decides |
|---|---|
| **the food SHAPE** *(sawtooth vs terminal streak, vs v4's 517)* | *is the colony alive or dying?* |
| **rescue uid DISTRIBUTION** | *is the net handling traffic or hiding a trap?* |
| **`completed_kind` at repeat positions** | *did defect 1 recur?* |

★★★ **If rescues cluster on one uid holding an emergency-access Mine job, and
`completed_kind` is constant at that position, those are the SAME finding arriving down
two channels** — *and defect 1's staged read is answered by v5 exactly as the packet
promised.*

⚠ **REGISTERED AS MEASURES, NOT BARS.** *Changing a bar mid-run is refused; naming a
zero's — or a spike's — candidate readings before the data is not.*

---

## 4 · FALSIFIERS — **each names the observation that makes it RED**

| claim | falsifier |
|---|---|
| **the sweep reads the JOB's own unclaimed duration** | **reap count FLAT as claim rate varies** |
| **egress requests terminate** | **an invalid-exit request re-issues an identical job** |
| ★★ **and its CONTROL** | **a NORMAL egress request still completes** |
| **completions are real** | ★★★ **`completed_kind` constant across N completions at one pos** |

★★★★ **Every planted failure runs TWO arms: RED on the claimed axis AND GREEN on a
matched control.** *A plant that reddens everything is exactly as vacuous as a test
that reddens nothing.*

---

## 5 · WHAT I WILL NOT DO AT SCORING TIME — *written by the scorer, before the data*

**The founding five carry forward unchanged** *(no re-baselining · no zero-as-pass on
an unproven channel · no partial count as a pass · no failure parked as
"flagged" · no cross-carrying between co-resident results)*, **plus two this arc
bought:**

6. ★★★★★ **NO ZERO READ AS A RESULT UNTIL ITS PATH AND ITS PATTERN ARE BOTH
   VERIFIED.** *Twice in one evening a zero meant "the instrument was not pointed at
   the data": a directory that did not exist, and `kind=` against ANSI escapes that
   sit between key and `=`.* **Strip `\x1b\[[0-9;]*m`; confirm the path resolves;
   enumerate the log's real vocabulary before trusting an absence.**
7. ★★★★ **NO PROMOTING THE ELEGANT STORY.** *The material-deadlock closure — emergency
   mine suppresses the drop → the ladder that would build the egress stalls silently →
   the exit stays invalid → the planner re-issues the mine — is the most attractive
   account available and is **PARKED, INFERRED, UNREAD**.* **`kind`-on-arrival decides
   it from v5's log. Until then it is not a finding.**

---

## 6 · KNOWN-OPEN AT LAUNCH — **declared, not discovered later**

- ⚠ **DEFECT 1 IS DELIBERATELY UNFIXED — *STAGED, NOT PARKED*.** *The block-write does
  not stick; `is_filled()` passed 281 consecutive times and `job moot` fired zero times
  in 945K lines.* ★★ **v5 is its instrument, not its fix.**

  > ★★★★★ **SUPERSEDES the both-sufficient-blockers ruling, accepted on its merits:
  > fixing a mechanism whose one remaining read has not run is this arc's own
  > anti-pattern.** *Three wrong-interval fixes taught us what building on an unread
  > costs.* **With defect 2 wired shut the loop loses its engine, and F7/F8 plus the
  > three log fields make v5 the instrument that reveals defect 1's true shape.**

  | | |
  |---|---|
  | ★★★ **THE READ** | **`completed_kind` on the completion line, across N completions at one `pos`.** *Constant `Rock` ⇒ the write never landed (look at `block_change`'s apply path and chunk writability). Anything else ⇒ it names what re-fills the cell.* |
  | ★★★ **THE FIX WINDOW** | **The row immediately after v5, informed by v5's data — not deferred indefinitely.** *Where-is-its-row applies DOUBLY to a deliberate deferral.* |
  | **RIDER** | *joint with 5b's own flag: does `job.progress` keep advancing on a claim the colonist can no longer service after `leave_route`? Same capture, same watch.* |
- ⚠ **DEFECT 2's FIX NEEDS A NAMED CONSUMER.** *A terminated egress request stops the
  loop but leaves the colonist trapped with nothing rescuing them.* **What observes an
  unrescued member? A terminated request with no observer is the silent-stall shape
  one row over.**
- ⚠ **THE FIVE-SITE ARRIVAL CONCENTRATION IS UNEXPLAINED** *(285 of 479 arrivals; four
  sites with zero completions).* **`kind`-on-arrival is the read.**

---

## 7 · ★★★★★★ EXPECTATIONS — **STATED HONESTLY, BEFORE THE RUN**

> ## **v5's PASS PROBABILITY IS GENUINELY UNCERTAIN, AND THAT IS THE CORRECT POSTURE.**

**What is still broken going in, on purpose:**

- **the silent-stall sites are UNFIXED** *(now instrumented)*
- **defect 1 is UNFIXED** *(now instrumented)*
- ★★★ **defect 2's terminated requests leave colonists UN-RESCUED** — *the ultimate
  fail-safe is the absorber of last resort, and it is the very thing the phantom
  completions were disarming (§1b).* **v5 shows whether it suffices.**

### ★★★★★ WHY THAT IS ACCEPTABLE — **THE FAIL CASE IS DESIGNED TO BE DECISIVE**

**Every known way v5 can fail now EMITS ITS NAME:**

| failure | the field that names it |
|---|---|
| the trap recurs | **F7** + `completed_kind` |
| a completion lies again | **F8** |
| a colonist stalls on materials | **the stall emit** (`kind`+`pos`+`required_item`) |
| the wrong job kind is being attempted | **`kind` on arrival** |
| the backstop is disarmed again | **`BASTION_EGRESS_DIAG` wipe-reason trace** |
| the farm stops at wave one again | **F1 generation-2** |

> ## **A RUN THAT EITHER CLOSES ARC 1 OR NAMES ITS KILLER IN THE LOG IS A GOOD BET AT
> 2.5 HOURS.**

★★★★★★ **v4's sin was not failing. It was failing ILLEGIBLY** — *59 completions
identical to v3, a health metric pointing up, a disarmed backstop, and not one field
in the capture that could say why.* **v5 is built so that cannot happen twice.**
