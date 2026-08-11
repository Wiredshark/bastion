# ITEM 8 v3 — **SCORING PROCEDURE, PRE-REGISTERED**

**Written BEFORE the run's data exists, by the scorer.** *Pin under test:
`517cb50f6d` (or the exact launch tip, read fresh from the run's own boot stamp —
never carried from memory).*

> ## ★★★★★ **THE POINT: SCORING SHOULD BE MECHANICAL. Every judgement call made
> here, in advance, is one that cannot be made after seeing the numbers.**

*This run has an unusual property — it is the first attempt at item 8 that could
reach a scored cycle, so there will be pressure to read it generously. That is
exactly the pressure pre-registration exists to remove.*

---

## 0a · ★★ THE CONCRETE MARKERS — **greppable, not interpretable**

*Learned from v2's own driver log (`9530d34748`) rather than assumed. Recorded here
so scoring is a grep, not a judgement about where things start.*

| what | marker | note |
|---|---|---|
| **scored window START** | driver's `=== script complete, disconnecting ===` | *the driver exits ~60 s in BY DESIGN; the unattended window begins at its disconnect* |
| **liveness heartbeat** | `"bastion food stock sample"` | ★ **unconditional, `tick % 300`** — the only signal during the unattended window |
| **measure 2 (eats)** | `"bastion: ate — hunger restored"` | per-cycle counts; also the **denominator** for the split ratio |
| **fix precondition** | `b5_split_off_one_fired` | the numerator |
| **the crash** | `try_merge`'s `debug_assert` panic | *absence is only evidence with the numerator > 0* |

> ★★★ **The heartbeat is unconditional and tick-keyed, which makes it the ONE
> channel whose silence is unambiguous** — *it cannot be silenced by a branch not
> being taken, only by the server not ticking.* **That is why it is the liveness
> signal and nothing else is.**

## 0 · PREFLIGHT — **run these BEFORE reading any measure**

**If any fails, the run is VOID and no measure is scored.**

| # | check | why |
|---|---|---|
| 1 | ★★ **log stable across two size reads** | *an evidence file with a live writer is not final; a "final tick" off a running server is a snapshot* |
| 2 | **boot stamp present** — commit + binary mtime after last source edit | *§1a: a live log stamps its code identity, or the leg proves nothing about the code under test* |
| 3 | **effective config emitted** — decay/interrupt/comfort read from the run's OWN boot log | *a wrong-config leg is VOID, and a copied-default fallback is indistinguishable from success* |
| 4 | **zero client connections inside the scored window** | *the arc's founding contamination; the promise is unattended* |
| 5 | ★★★ **crossing times taken from THIS run's `NeedCrossed` records**, not the planning table | *the trait stagger moves them per-colonist; the planning numbers are trait-free* |

---

## 1-AMENDMENT · ★★★★★★★ **INSTRUMENT REPAIR, DATED 2026-08-11, WRITTEN MID-RUN**

> **Written at ~T+30 min of the scored window, BEFORE the run ended and BEFORE any
> landing data was read. `ITEM8-V3-PREREGISTRATION.md` IS NOT EDITED — Fable-ruled:
> a prereg corrected in place is indistinguishable from one tuned to its result; a
> DATED AMENDMENT BESIDE IT is an honest instrument repair.**

### THE DEFECT

**`b5_split_off_one_fired` HAS NO CONSUMER.** *Two sites in the entire tree — the
declaration (`bastion_jobs.rs:4436`) and the increment (`:13757`). No reader, no
emitter, no log line.*

**Verified empirically, not inferred:**

    v2's live server log:   "b5_"              ->    0 occurrences
    same log:               "preempt_attempts" ->  150 occurrences

★★★ **The whole `b5_*` class is HARNESS-ONLY** — board fields read by accessors
(`bastion_item6_witness_stats()` and kin) that **bastion-harness** calls. *A live
`server-cli` run never calls them.* **No live alternative exists either:** *the
entity event log shows 0 occurrences despite `BASTION_ENTITY_EVENT_LOG=1`, and the
only pickup-adjacent live line (`"bastion: ate — hunger restored"`) fires on BOTH
paths.*

> ## **SO A ZERO SPLITS COUNT WAS GUARANTEED BEFORE LAUNCH, AT ANY SPLIT RATE. IT
> IS AN UNWIRED CHANNEL, NOT A MEASUREMENT.**

### WHAT THIS VOIDS — and what it does NOT

| item | status |
|---|---|
| the splits:eats **denominator** | ★ **DEAD for this run — DO NOT COMPUTE IT.** *Its absence is not a finding.* |
| §1's `b5_split_off_one_fired == 0 -> VOID` rule | **superseded for this run** by the argument below |
| item 8's six measures · health signal · #85's six fields | **UNAFFECTED** *(all live-emitting; the fail-safe `tracing::warn!` verified at `:17413`, and v2's log carries it 5×)* |

### ★★★★ THE SUBSTITUTE PRECONDITION — v2 SUPPLIES IT

**v2 CRASHED on this exact path at 23.6 min: same scenario, same config, same food
loop.** *That is an observed DETONATION, which is a stronger precondition witness
than a count.*

★★★ **And the fix cannot have suppressed the population:** *it changes what
`split_off_one` DOES (returns instead of pushes), not WHETHER eats against
multi-unit piles occur — the population is driven by the farm/food loop, untouched.*

> **REGISTERED READING: v2 supplies the precondition; v3 supplies the outcome. A
> silent v3 across the full window, on the scenario that detonated in 23.6 minutes,
> is a valid before/after result — recorded WITH its instrument gap named, never
> dressed up as a measured exercise count.**

★ *Fable's assessment, accepted: this is a matched control in the wild — same
scenario, same config, detonation then silence, with the fix's blast radius
provably clear of the population driver.*

### THE LAW THIS COST

> ## ★★★★★ **A WITNESS NEEDS A CONSUMER ON THE SURFACE THAT WILL BE READ. VERIFY
> THE PRODUCER *AND* THE READER — A COUNTER WITH NO EMITTER IS A DOC COMMENT THAT
> COMPILES.**

★★ **And the preventable-at-spec-time form:** *"add a field to THIS LOG LINE" is
live by construction; "add a counter" is harness-only by default.* **#85's six
fields were specified as fields on a named emit site and came out live; this one was
specified as a quantity and came out mute. Same builder, same review, two framings.**

## 1 · ★★★★★★ THE FIX CLAIM — **scored FIRST, and it can VOID the rest**

**Three-way rule, as pre-registered in `ITEM8-V3-PREREGISTRATION.md`:**

    b5_split_off_one_fired > 0  AND  debug_assert silent   ->  PASS (exercised and held)
    b5_split_off_one_fired == 0                            ->  VOID on this claim
    b5_split_off_one_fired > 0  AND  debug_assert fired    ->  THE FINDING (report, do not patch around)

### ★★★★ THE RATE DENOMINATOR — **my scoring commitment, registered here**

**The prereg's rate condition anchors to "v3's own sustained rate," which is
self-referential: a run splitting 5 times in 2.5 h is consistent with its own rate.**
*A rate benchmarked against itself detects a CHANGE and never a uniformly wrong
value.*

> ## **THE DENOMINATOR IS `EatFrom` COMPLETIONS — a DIFFERENT producer, already
> counted by measure 2.**

    every eat against a pile holding >1 unit takes the `Some` path
    => b5_split_off_one_fired should track (eats - last-unit eats) closely
    => splits << eats  ==  the split path was suppressed  ==  VOID on the fix claim

★ **A ratio can fail; a self-comparison cannot.** *Applied whether or not this
sentence reaches the prereg file — registered before data, which is what binds it.*

**Also VOID for a window: splits high early then collapsing** *(the prereg's own
clause; it catches degradation, and the denominator catches uniform suppression —
both are needed).*

---

## 2 · THE SIX MEASURES

**Scored only if §0 and §1 pass.** *Producers per
`ITEM8-PREFLIGHT-BAR-PREREGISTRATION.md`, which names each with its commit.*

| # | measure | source | VOID / FAIL |
|---|---|---|---|
| 1 | *(REPLACED — "no deaths" is vacuous against an engine with no death path)* | — | — |
| 2 | eats per cycle | `ate` completion logs, uid-tagged | zero eats with hunger demonstrably crossed = FAIL, not VOID |
| 3 | sleeps per cycle | `slept` completion logs, uid-tagged | as above for rest |
| 4 | food stock non-decreasing from cycle 2 | `"bastion food stock sample"`, every 300 ticks | a *trend*, not a max — read the series |
| 5 | no permanent stall | `NeedCrossed{need, dir}` | a need crossing with no subsequent satisfaction |
| 6 | fail-safe rate not climbing | `ULTIMATE FAIL-SAFE` teleport log | a rising per-cycle rate |

### ★★★ THE ZERO-WINDOW — **checked BEFORE any pass/fail scoring**

    0 -> first crossing    hunger above, rest above    REQUIRED: ZERO eats, ZERO sleeps
    after crossing         below                        the event, every cycle

★★ **An event BEFORE its need crosses does not read as a better result — it VOIDS
the leg.** *The outcome arrived by a path that is not the mechanism.*

---

## 2b · ★★★★ #85 RIDES THIS RUN FOR FREE — **all six discriminating fields are in the binary**

**Verified, not assumed:** *`0891889d64` (four membership bits) is an ancestor of the
launch tip `b6ae81470b`, and the tip's `bastion_jobs.rs` carries 16 references to
the bit names.* **The running binary emits all six #85 fields at the fail-safe
site:**

    in_loaded_chunk · is_rider · has_collider · has_mass · has_density · has_body

> ## **SO v3 IS ITEM 8's GATE **AND** #85's DISCRIMINATOR, AT NO EXTRA COST.**

**Read as a SECONDARY result, scored separately** *(the no-cross-carrying rule: this
must not lift or sink item 8's own bar)*:

| observation | reading |
|---|---|
| **any fail-safe fires** | ★★★ read all six bits — **one firing decides all four gates** |
| `in_loaded_chunk == false` | gate 1 — #85 collapses into the presence row (terrain-chunk axis) |
| `in_loaded_chunk == true`, `is_rider == true` | gate 2 — mount/dismount defect |
| a membership bit false | gate 4 — **and the bit names which component**, which the AND could not |
| all six "normal" | **all four enumerated gates dead — genuinely new, and worth a row** |
| ★ **zero fail-safes across the run** | the registered EXTINCTION prediction — *see the caveat below* |

### ★★ THE ZERO CASE NEEDS ITS PRECONDITION — same trap as the fix claim

**Fable's registered prediction: the uid=166 fingerprint class goes EXTINCT
post-presence.** *Zero fail-safes is consistent with that — and equally consistent
with "no colonist entered a below-grade state at all this run."*

> ★★★ **Before reading zero fail-safes as extinction, confirm the population
> existed**: *below-grade/egress activity occurring at all (`egress_verdicts`,
> `below_grade` watch), i.e. colonists DID get into the states that used to end in a
> fail-safe.* **Zero firings with zero exposure is VOID on the prediction, not
> confirmation** — the sit-trap law, third application today.

## 3 · THE HEALTH SIGNAL — **baseline is 30, NOT 31.8**

    const TPS: u64 = 30;        // server-cli/src/main.rs:49 — READ, not assumed

> ★★★★ **DO NOT use v2's derived ~31.8 ticks/s as the baseline.** *45000 ticks /
> 1416 s = 31.8 is **6% above the server's own target**, which a tick loop does not
> do — the ~23.6 min figure almost certainly starts later than tick 0.* **A healthy
> server at 30 TPS would read as 6% degraded against it: a false positive designed
> in.**

**Degradation = sustained materially below 30 ticks/s.** ★ *Non-gating, but an
endurance finding in its own right and uncaught by anything else in the bar.*

---

## 4 · ★★★★★ WHAT I WILL NOT DO AT SCORING TIME

**Written down because these are the moves that feel reasonable at 2.5 hours of
sunk cost:**

1. **Not re-baseline a fixture to make it green.** *Expected new values are
   enumerated BEFORE a run, field by field, or the change is not made.*
2. **Not read a zero as a pass** on any measure whose channel is not proven
   reachable. *Zero cases are VOID.*
3. **Not accept a partial cycle count.** *N=5 scored cycles; 4 cycles is a shorter
   run, not a passing one — report it as what it is.*
4. **Not silently drop a failed measure into "flagged for follow-up."** *A failure
   gets a row or it gets reported as a failure.*
5. ★★ **Not let the fix claim's PASS carry the endurance bar, or vice versa.**
   *They are separate results from one run; the fix can hold while the colony still
   starves, and that is a legible outcome — not a mixed one.*

---

## 4b · ★★★ RECORD WHAT WAS ACTUALLY READ — **a deliverable of scoring, not a by-product**

**Fable's condition on committing v3's log raw (~52–75 MB): before v4's capture,
someone lists what was actually READ from v3's log. That list becomes the filter
spec if we ever build one — derived from USE, not from prediction.**

> ## ★★★★ **THAT LIST CAN ONLY BE BUILT WHILE SCORING. IT CANNOT BE RECONSTRUCTED
> AFTERWARDS** — *"which greps did I run three weeks ago" has no producer.*

**So, during scoring, keep a running list:**

    every grep/pattern run against the log, verbatim
    every field or line-kind whose VALUE entered a verdict
    every line-kind opened and found IRRELEVANT   <- ★ the most valuable column

★★★ **The irrelevant ones matter most for a filter spec.** *A filter built only from
what was USED will drop everything that was CHECKED AND CLEARED — and the next
run's scorer will not know those checks were ever possible.* **"I looked and it was
not the problem" is information the filter must preserve the ability to re-ask.**

★ **Write it into the results doc as its own section**, per write-it-where-you-will-
cite-it-from: *a list kept in scratch and transcribed later is the same lost-evidence
shape as a log copied at report time.*

## 4c · ★★★★ FAMINE POST-MORTEM — **CANDIDATES REGISTERED BEFORE THE CAPTURE**

*Written from the T+75 health look, before teardown and before any board read.
**Registered so a story cannot be fitted to the specimen at 13:20** — this row's own
lesson, five mechanisms deep.*

**THE ONE QUESTION:** *why did sow jobs never complete?* ★★ **"Creation stopped" is
NOT a second puzzle — farm generation is `occupied`-set gated, so an uncompleted job
holds its cell forever. Creation stopping is DOWNSTREAM.** *(READ: the gate. UNREAD:
`occupied`'s construction — live-jobs-derived confirms this, sprite/terrain-derived
reopens it. **That read runs first.**)*

| candidate | discriminating read (from the capture, not from reasoning) |
|---|---|
| ★★★ **claim leak** — job claimed by a colonist that never returns | `claimed_by = Some(uid)` on the stuck sow jobs, **cross-referenced against despondent / rescued / fail-safed uids** |
| **fetch contract unfillable** — no seeds | founding seed stock at the time sows were created; whether any seed-fetch job exists and its state |
| **affordance / standable gate** | the sow cells' standability; the `OnTopAlways` note |
| **board starvation by mine designations** | claimant counts by kind over time — were colonists ever free to take a sow? |

### ★★★★★ FABLE'S REFRAME, REGISTERED AS THE LEADING SHAPE

> ## **A JOB THAT CAN NEVER COMPLETE BUT NEVER DIES IS A CLAIM LEAK WEARING A FARM
> COSTUME — and the SHARED EXCLUSIVE STATE IS THE CELL.**

★★★ *If it holds, the fix conversation is about **job liveness / timeout on farm
claims**, not about farming* — **which would make this an instance of
[[failures-must-not-compose-through-shared-exclusive-state]], not a new class.**

★★ **PRE-REGISTERED CONSEQUENCE, so it cannot be claimed after the fact:** *if the
leak is confirmed, the travel-watchdog family is implicated — **item 3's fix
established that a kind-agnostic watchdog releases colonists that never arrived**,
and a released colonist leaves its claim's cell occupied.* **That connection is
predicted NOW; if the data shows it, it is a confirmation, and if it shows a
different owner-loss path, the prediction was wrong and gets recorded as wrong.**

## 5 · OUTPUT

**A results doc stating, in order:** preflight attestations · the fix claim with its
three-way reading and the splits:eats ratio · the six measures with their own
numbers · the health signal · **and explicitly what this run does NOT establish.**

★ *Per the presence-row scoring's own lesson: the bar's coverage gaps are named in
the scoring, not left for a later reader to discover the hard way.*
