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

**F7 and F8 exist because v4 passed nothing and still looked busy.** *143 of 145
late-run completions at one cell; 361 completions with zero drops, zero XP, zero
cave-ins.*

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

- ⚠ **DEFECT 1 IS DELIBERATELY UNFIXED.** *The block-write does not stick; `is_filled()`
  passed 281 consecutive times and `job moot` fired zero times in 945K lines. **Held
  for `completed_kind` to decide the mechanism rather than guessed at.*** ★★ **v5 is
  its instrument, not its fix.**
- ⚠ **DEFECT 2's FIX NEEDS A NAMED CONSUMER.** *A terminated egress request stops the
  loop but leaves the colonist trapped with nothing rescuing them.* **What observes an
  unrescued member? A terminated request with no observer is the silent-stall shape
  one row over.**
- ⚠ **THE FIVE-SITE ARRIVAL CONCENTRATION IS UNEXPLAINED** *(285 of 479 arrivals; four
  sites with zero completions).* **`kind`-on-arrival is the read.**
