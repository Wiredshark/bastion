# DOSSIER — **ARC 1: COLONY SURVIVAL**

**Closed 2026-08-12. Item 8's endurance arc, v1 → v5.** *Per the per-arc-close ritual.*

> ## **THE ARC DID NOT ADVANCE BY BEING RIGHT. IT ADVANCED BY CATCHING ITSELF — AND
> THE CATCHES ARE THE TRANSFERABLE PART.**

★★★★★ *This dossier is organised around that, not around the wins. A record that
lists only what worked teaches nothing, because nobody's next arc will fail in the
places this one succeeded.*

---

## 1 · THE ARC IN ONE LINE

**v3 and v4 both scored EXACTLY 59 farm completions — 19 tilled · 20 sown · 20
harvested — and both starved, by DIFFERENT fatal mechanisms.**

    v5:  56 tilled · 1,749 sown · 1,721 harvested    ~31 cycles/plot
         last farm event 0.6 SECONDS before teardown

> ## **THE COLONY THAT STARVED TWICE AT NINETEEN TILLS NOW FARMS THIRTY-ONE CYCLES A
> PLOT AND IS STILL SOWING WHEN THE CLOCK STOPS.**

---

## 2 · ★★★★★★ THE ROOT CAUSE — **and why two correct fixes didn't move the number**

**Every `bastion: job completed` in v4's 945K lines was `Designated(Mine)`, and 328 of
361 were the SAME TWO ADJACENT CELLS — 281 on one.**

**A 3-beat loop, every ~15 s for 2.5 hours:**

    job completed (Mine, same cell)
    emergency route exhausted with invalid exit; member released
    emergency access restored (REQ-0040) owner=80 cells=1

★★★★ **Routes 2 and 3 were correct fixes for real defects that were never what stopped
the farm.** *Predicted from F1's identity with v3 before the read that confirmed it.*

**TWO SEPARABLE DEFECTS:** *(1) the block never leaves; (2) the egress request never
terminates.* ★★★ **Defect 2's fix alone ended the loop — 281 repeats became zero — so
defect 1 is LATENT, not disproven.** *Its subject is now known (v4's trap cell is
FOLIAGE, `completed_kind=Some(Leaves)`); its mechanism is still UNREAD.*

### ★★★★★ THE FIX WAS A RECONNECTION, NOT AN ADDITION

**A complete 7-link rescue chain already existed and had NEVER been able to run:**

    replans bounded -> exhaustion releases + STICKY bar -> egress can't help (by design)
    -> the stuck-teleport net is the only rescue -> IT CAN NOW ACCRUE -> teleport fires
    -> surface reached -> bar lifts

★★★★★★ **LINK 5 WAS BROKEN AND THAT MADE LINKS 2–7 UNREACHABLE.** *Every phantom
completion wiped the 60-second stuck clock; it never passed 20 seconds. In v5 the chain
fired live, at exactly `secs=60.0`, on the same job class that trapped owner=80.*

---

## 3 · ★★★★★★ THE FALSE-SIGNAL TAXONOMY — **the arc's deepest finding**

**One unverified completion event fed three consumers, and a second signal had none:**

| signal | truth | consumers |
|---|---|---|
| **the completion event** | ⛔ **LIED** | *believed by three: health metric, stuck-watchdog, diag channel* |
| **the S1 sentinel** | ✅ **TRUE** | ⛔ *ignored by three: no bar, no gate, no page* |

> ## **ONE ARC PRODUCED A SIGNAL THAT LIED TO THREE CONSUMERS AND A SIGNAL THAT THREE
> CONSUMERS IGNORED. AN INSTRUMENT NEEDS BOTH TRUTH AND A CONSUMER; EITHER HALF MISSING
> KILLS IT.**

★★★★★ **The mechanism, exactly:** *the completion arm suppressed the drop, the XP and
the cave-in for effect-less work — **and emitted the announcement unconditionally**.*
**The code already knew the work had no consequences and announced it anyway.**

★★★ **S1 was correct for ninety minutes** *(517 consecutive zero samples against a
threshold of 10)* **and log-only.** *Full consumer enumeration — 11 sites — in
`ROW-COMPLETION-SIGNAL-SPLIT.md`.*

---

## 4 · ★★★★★★ THE FOUR COSTUMES — **vacuity, one evening, four disguises**

| # | disguise | what killed it |
|---|---|---|
| **1** | an instrument with **no noise floor** | *its own matched control: two SAME-PACING runs diverged 100%* |
| **2** | a variable **the path never reads** | *grep for consumers: one call site, different binary* |
| **3** | a path that reads it but **deleted the mechanism** | *its own smoke test: 1.4 s vs 14–22 s live* |
| **4** | a falsifier that **fires on everything** | *"would it redden two CAPPED runs?" — yes* |

★★★★★★ **EVERY ONE PASSED THE PREVIOUS INSTANCE'S CHECK.** *Which is why the standing
check is a QUESTION, not a list:*

> ## **WHAT WOULD MAKE THIS GO THE OTHER WAY, AND IS THAT THE AXIS I CLAIM?**

*Full case law: `readme/PACKET-CRAFT-CHECKLIST.md` entry 7.*

---

## 5 · ★★★★★ THE BAR LINEAGE — **an acceptance criterion is an instrument**

| wording | vs v4, the specimen it exists to reject |
|---|---|
| **completions > 0** | v4 scores 59 → **PASSES** |
| **generation-2 by first-harvest timestamp** | v4 has TWELVE → **PASSES** |
| ✅ **farm activity in the FINAL THIRD** | v4 has ZERO → **FAILS** |

★★★★★★ **Wording 2 was written to fix wording 1's defect and reproduced it exactly.**
*Caught minutes before scoring, on the last check still runnable against v4.*

> ## **A BAR MUST FAIL THE FAILING SPECIMEN — tested against the known failure BEFORE
> the candidate's data exists, exactly as a planted test must go RED before its GREEN
> means anything.**

---

## 6 · THE COMPRESSION CERTIFICATION — **a NEGATIVE result, and the most useful kind**

    capped   (n=8):  185 … 233        uncapped (n=8): 1134 … 1458
    ZERO OVERLAP · 901-tick gap · ~6x

★★★★ **Compression is NOT equivalent for promotion-coupled runs.** *Chunk generation is
wall-second-bounded while the tick counter is not, so compression changes the
loaded/simulated ratio — **which is item 8's own subject**.*

★★★ **It drew the fast/real boundary in measured ink, and priced the bounty on moving
it:** *tick-driven world loading would make the dependency class extinct, and **the same
instrument that proved the impossibility certifies its removal** — acceptance is
**OVERLAP *AND* a planted wall-dependency still separating**, never overlap alone.*

★★ *The equivalence of the INPUT was always a code proof, not an A/B: dt is a constant,
so the bypass removes only the sleep. `ROW-TIME-COMPRESSION-EQUIVALENCE-SPEC.md`.*

---

## 7 · ⚠ THE ERROR RECORD — **the spine of this dossier**

**Mine, this arc, each caught and corrected in the record:**

| error | how it died |
|---|---|
| **five mechanisms proposed for #85** | *each killed by its own producer → "stop proposing and instrument"* |
| **a wrong directory read as a result** | *an absence rendering identically to an exclusion* |
| **`kind=` against ANSI escapes** | *a grep pattern is a claim about naming* |
| **"the entire labour force was captured"** | *narrowed: the completion CHANNEL was, owned by ONE colonist* |
| **cited `:10860` by line, not symbol** | *my own standing rule; the site had drifted 3,600 lines* |
| **"corrected on the facts" on one message** | *right by luck ≠ right by method — the later report lost the tie* |
| **the rate-profile channel** | *registered at n=3, WITHDRAWN at n=4 by its own fourth sample* |
| **a 114× ratio raised as alarm** | *the base (14 KB/s) refuted its severity* |
| ★★★ **F1's own operationalization** | *tested against v4 and found passing — minutes before it governed* |

★★★★★ **Nine corrections, and the arc's verdict is trustworthy BECAUSE of them, not
despite them.** *The two that would have cost the most — the F1 bar and the F7
denominator — were both caught by checking my own instrument against the failing
specimen rather than against my expectation.*

---

## 8 · THE VERDICT — **three scopes, reported separately**

**BAR: 7 pass · 1 partial · 0 fail.** *F8 held PARTIAL against the scorer's own wish —
its exclusion half proven (33 cases), its inclusion half unexercised (zero real
Mine/Chop/Build completions). A zero on an unreachable channel is not a pass.*

**OPEN READS: all favourable.** *Food sawtooth ending at 341 (terminal streak **0** vs
v4's **517**) · rescues across **20 uids**, max 6 — the loop hypothesis REFUTED · trap
cell quiet.*

**RECOMMENDATION: ARC 1 CLOSED** — *accepted.*

---

## 9 · CARRIED FORWARD — **staged, with reads named**

1. **DEFECT 1's mechanism** — *the untouched 790 MB `userdata` in `8dd2f9463b` is the
   instrument; the trap cell completed twice in v5.*
2. **F8's inclusion half** — *a one-Mine-job fixture closes it cheaply.*
3. **The completion-signal split** — *`JobEnded` vs `WorkCompleted{effect}`; 11
   consumers enumerated.*
4. **S1's promotion** — *scored bar now; live-abort for ITERATION runs only, earned by
   track record; **never** for certification runs.*
5. **Tick-driven world loading** — *the spec that erases the last named impossibility.*
6. **Script the teardown** *(#73)* — *a 2.5-hour irreplaceable capture should not depend
   on a manual procedure run late at night.*
