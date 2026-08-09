# SITE 6 (self-job RE-CLAIM) — DELTA REGISTRATION

**Written before the build, against `0fb7ca07b7`.**
★ **Registered per the #55 entry ticket and the re-baseline law**
*(a re-baseline is evidence only if its expected delta was named field-by-field
and seed-by-seed FIRST).*

## ★★★★★★★ VERDICT FIRST: SITE 6 **FAILS** THE MUTATING-WINDOW ENTRY TICKET

**#55's rule: a mutating item joins a window ONLY with an exact pre-registered
per-seed delta for every field it touches.**

★★★ **Site 6 changes job LIFECYCLE — which job object a colonist holds, how long
jobs live, and how many exist at any tick. ★★★★★ Its effects are not enumerable
field-by-field, and I will not manufacture an enumeration to buy the ticket.**

> ## ★★★★★ **SITE 6 IS A RE-BASELINE, NOT A WINDOW ITEM.**
> ★ **Specifiability is the entry ticket, and site 6 does not have it.**
> **The honest move is to say so, not to under-declare the blast radius.**

★★ **This document therefore registers what it CAN: the DIRECTIONAL predictions
that are falsifiable, and — more importantly — the baseline that does not exist.**

## ★★★★★★★★ THE BLOCKING GAP: THE ACCEPTANCE BAR HAS NO BEFORE-VALUE

**Checked `corpus-waves/wave29_ROWBPRIME_B_7590dfa962_FULL.json` — 48 seeds, 283
leaf fields. ★★★ It carries NO `settle_invariant_*` key. The instrument landed
after the newest wave.**

> ★★★★★★★ **`settle_invariant_holds → 0 after site 6` is the acceptance I called
> "real" — and there is no measured before-number for it anywhere in the corpus.**

★★★ **A 0-after is then indistinguishable from an instrument that never fired on
these seeds** — *the void-pair law: zero-drift means CHANGED NOTHING or MEASURED
NOTHING, and with no before you cannot tell which.*

★ **The before-value is PERISHABLE**: `0fb7ca07b7` is the pre-site-6 tip and 5b is
running `preempt_scenario` 49/50/51/53 on it right now. **Requested from those
existing runs; no rebuild, ~2 minutes.**

### ★★ BOTH READINGS REGISTERED BEFORE EITHER IS SEEN

| reading | consequence |
|---|---|
| ★★★ **`holds:false` on some/all four** | **the bar is REAL** — *the field doc's "expected to fire broadly" is confirmed and 0-after is genuine evidence* |
| ★★★★★ **`holds:true` on all four** | ★★ **THE BAR IS VACUOUS on this population and must be REPORTED AS SUCH** — *acceptance moves to a scenario that trips it, or is dropped; never narrated as a pass* |

★ **Registering both in advance is the point** — *neither outcome can be turned
into a success story afterwards.*

## ★★★ WHAT **IS** SPECIFIABLE — DIRECTIONAL, AND EACH WITH ITS FALSIFIER

**These are DIRECTION predictions, not value predictions. ★ Stated as such.**

| # | field | before | after | falsifier |
|---|---|---|---|---|
| **D1** | `settle_invariant_holds` | ★ **unmeasured** *(see above)* | **true everywhere** | **any `false`** ⇒ the sweep race (blocker 1), first hypothesis |
| **D2** | `stuck_strikes` on self-jobs | ★★★ **0 across all 660 ticks of job 33** *(measured 2026-08-08)* | **ACCUMULATES on a `RestAt` that fails and retries** | **still 0** ⇒ re-claim didn't land; the pass is still inserting fresh jobs |
| **D3** | self-job **ID stability** across a retry | **new id each attempt** | ★★ **SAME id** | **id changes** ⇒ same as D2, and the more direct observable |
| **D4** | **Despond `until` across a suspend cycle** | *(re-rolled by the breakdown arm)* | ★★★★★ **byte-identical, per-colonist** | **see the two-colonist requirement below** |

> ★★ **D2/D3 are family 1's actual deliverable.** *The design said "the entry
> persists"; a run where the counter still reads 0 is a different claim, and only
> the run is evidence.*

## ★★★★★ D4'S TRAP: A ONE-COLONIST TEST CANNOT SEE CROSS-ATTRIBUTION

**`Job` carries NO owner field but `claimed_by`, and release NULLS it**
*(`common/src/bastion.rs:1079-1119`, read at `0fb7ca07b7`)*. ★★★ **So re-claim's
only available match key is `kind` — and by kind alone, one colonist inherits
another's breakdown deadline.**

★★★★★ **With ONE despondent colonist, *"`until` is byte-identical"* PASSES on a
build that cross-attributes** — *there is no other deadline for it to be wrong
about.* **TWO colonists, distinct deadlines, each asserts its OWN.**
*(Carried into `AUTON2-ACCEPTANCE-FIXTURES.md`.)*

## ★★ THE SCHEMA HALF — `Job` IS SERIALIZED

**Site 6 adds an ownership field that survives release**
*(`suspended_for: Option<Uid>`, or an explicit suspend state — shape is 5b's).*

- ★★★ **`#[serde(default)]` is REQUIRED** — *every field added to `Job` since B5.8
  carries it (`is_access`, `stuck_strikes`); a save written before site 6 must
  still load.* **This is a build requirement, not a preference.**
- ★ **ADDITIVE at the schema level** — no existing field changes type or meaning,
  so `--expect-new` covers the harness side.
- ★★ **NOT additive at the behaviour level.** *That is the whole point of the
  verdict at the top: an additive SCHEMA delta is the cheap proxy, and site 6
  passes the proxy while failing the real test.*

## ★★★★★ WHAT I AM **NOT** PREDICTING, AND WHY THAT IS THE HONEST ANSWER

★ **Job counts, throughput, per-seed pass/fail, `endure_dug`, travel totals.**
**Site 6 changes how long jobs live and how many exist per tick; every
count-over-jobs field is downstream of that.**

> ★★★ **I could write plausible directions for all of them. ★★★★★ A plausible
> direction that is not derived is a story, and it would launder the re-baseline
> into a "registered" change.**

## ★★★★★★★★ CORRECTION (same session): **D1–D4 ARE NOT FAN-VISIBLE.**

**I wrote above that the handling is *"run the fan, diff it, explain every mover
against D1–D4."* ★★★ I then enumerated what the fan can actually see. Leaf counts
in `wave29_ROWBPRIME_B_7590dfa962_FULL.json`:**

    bed 0 · sleep 0 · eat 0 · hunger 0 · despond 0 · mood 0
    preempt 0 · orphan 0 · sweep 0 · stuck 0 · settle_invariant 0

★★ **The 27 `job` leaves are all WORK-job counts** *(`b5_build_ok_jobs`,
`b5_chop_jobs`, `b5_mine_jobs_remaining`, `b5_slope_jobs_total`, …)* — **none is a
lifecycle or claim-identity field.** ★ *A tree-wide grep confirms `settle_invariant`
appears in exactly one file in `bastion-test-evidence/`: this fixture doc. Never
in a wave.*

> ## ★★★★★ **THE FAN CANNOT SEE A SINGLE ONE OF D1–D4.**
> ★★★ **Same blindness that made this corpus structurally unable to see the
> AUTON-2 defect** — *and Fable already ruled the consequence for that case.*

### ★★★ SO THE FAN'S ROLE IS **HARMLESSNESS**, AND THE BAR IS **EXACT-MATCH-OR-BUST**

| instrument | proves |
|---|---|
| ★★ **the 48-seed fan** | **site 6 did not disturb the work economy.** *Nothing else. It is a blind instrument, which makes it a PERFECT harmlessness gate.* |
| ★★★★★ **instrumented `preempt_scenario` runs + the two fixtures** | **the whole of D1–D4.** *The mechanism proof lives here entirely.* |

★★★★★★★ **AND THE BLINDNESS CUTS THE RIGHT WAY: because no fan field is
downstream of need-drive behaviour by design, ANY movement in the 27 work-job
fields is unexplained-by-construction — and therefore a finding, not noise.**

★ **That is a stronger gate than a diff-and-explain, not a weaker one.** ★★ **It
also retires my refusal-to-predict above: the honest prediction for every
fan-visible field is EXACT MATCH, and I am registering that as the bar.**

> ★★ **What survives from the section above:** *site 6 still fails the window
> ticket, and D2/D3's before-values were measured AD HOC (job 33, 660 ticks), not
> in any wave — so their after-checks must be run in the SAME instrumented style,
> never inferred from a fan that carries neither field.*

## ★★★★★★★★ SECOND CORRECTION: `stuck_strikes` IS AN **INPUT**, AND SITE 6 WAKES THREE DORMANT CONSUMERS

**I registered `stuck_strikes` as an OBSERVABLE (D2). ★★★ It is also an INPUT to
three mechanisms — and for SELF-JOBS all three are dormant TODAY precisely
because it never accumulates past 0.**

| # | consumer | site | what site 6 wakes |
|---|---|---|---|
| **C1** | ★★★★★ **arrival tolerance** `ARRIVE_DIST + stuck_strikes.min(3)*1.2` | `10180` | ★★★★★★★ **bed/food arrival widens 2.5 → up to 6.1 blocks.** *`12064`: growth is "unconditional on kind."* |
| **C2** | `PERSIST_ESCALATE_STRIKES` escalation arm | `12096` | a path self-jobs have never reached |
| **C3** | ★★★ `route_exhausted` → `blocked_regions` | `12249` | ★★★★★ **FAN-VISIBLE — see D5** |

★ **`PERSIST_ESCALATE_STRIKES = 3` (`1610`); its doc names the shared consumers.**
★★ **`12214`'s comment says the gate is *"kind-agnostic, survives re-claim"* —
written in anticipation of exactly this change.**

### ★★★★★ C1 IS A GAMEPLAY CHANGE NOBODY SPECCED

**A colonist that strikes out three times on a bed then counts as ARRIVED from
6.1 blocks away — and receives rest recovery without being at the bed.**

★★ **It may be entirely correct** *(the designed bounded remote-work reach, Ben's
anti-loop invariant)* — ★★★★★ **but it was tuned for Mine/Chop targets on awkward
blocks and has never once applied to a bed.** ★ **This is the FR15 stuck-economy
re-tuning constraint arriving through a side door.**

> ★★★ **AND IT COLLIDES WITH THE TRAVEL ROW:** *`bastion_bed_slot.occupant` is a
> RESERVATION, so an arrival assertion must use the `ActiveJobState` transition or
> the arrive-tolerance DISTANCE — and C1 changes that distance.* ★★ **Any fixture
> asserting "reached the bed" must state WHICH tolerance it means, before and
> after.**

### ★★★★★★★★ C1 READ END-TO-END — IT IS REAL, AND IT ALSO **LOCKS THE BED**

**I escalated this as a question, then read it instead. Four sites, every link:**

| # | link | site |
|---|---|---|
| 1 | `stuck_strikes` accumulates | **site 6** |
| 2 | `arrive = ARRIVE_DIST + stuck_strikes.min(3)*1.2` → **2.5 … 6.1** | `10180` |
| 3 | `Traveling → Working` fires at that distance | same |
| 4 | ★★★★★ **recovery applies in the Working arm with NO distance check** | **`12410`** |

**`needs.rest += BED_REST_RECOVERY_PER_SEC * kind.quality() * dt`. The gates above
it are `work_stable` (supported, not moving) and the `RestAt` match. ★★★ Nothing
reads distance to `bed_pos`.**

> ★★★★★★★ **A colonist with 3 strikes SLEEPS FROM UP TO 6.1 BLOCKS AWAY, standing
> on the ground.**

★★★★★ **AND FOUR LINES ABOVE THE RECOVERY:** `if let Some(slot) =
board.beds.get_mut(&bed_pos) { slot.occupant = Some(u); }` — ★★★ **it TAKES the
capacity-1 slot, denying the bed to a colonist who could actually reach it.**
★ *And per the travel row, `occupant` is already a reservation rather than a
presence, so nothing downstream can distinguish the two cases.*

★★ **Dormant today** *(strikes never accumulate on self-jobs — the 0-across-660-
ticks measurement is exactly why nobody has seen this)*. **Site 6 switches it on.**

### ★★★ THE HANDLING — **NOT** A FIX

★★★★★ **Nothing changes in site 6.** *A pre-existing mechanism meets a newly-live
input; inventing a bed-specific tolerance mid-build is the same error as the
"third anti-thrash mechanism" I made on the hysteresis.*

| | |
|---|---|
| ★★★★★ **(a) RECOMMENDED — ship it, measure it** | *one assertion in the rest fixture: the colonist's distance to `bed_pos` at the tick recovery first applies. Records the real number instead of the worst case.* |
| ★ **(b) exempt self-jobs from tolerance growth** | ★★ **a mid-build change to Ben's anti-loop invariant, on a mechanism not yet measured. Not recommended.** |

**Escalated to 5b and to Fable in parallel; the gameplay call may be Ben's.**
★ **Neither answer blocks the build — (a) is one assertion, (b) is a later row.**
★★★ **The goal is that it lands CHOSEN rather than inherited.**

## ★★★★★★★ D5 — `b5_blocked_regions_count_*` **MAY RISE, AND A RISE IS EXPECTED**

**This BREAKS the exact-match-or-bust bar I registered one section above.**

> ★★★ **`b5_blocked_regions_count_at_settle` / `_at_end` are fan-visible, and site
> 6 feeds them by a real chain:**
> **re-claim → strikes accumulate → `>= PERSIST_ESCALATE_STRIKES` → a
> `route_exhausted` entry → the count rises.**

| | |
|---|---|
| ★★ **`b5_blocked_regions_count_*`** | **MAY RISE. A rise is EXPECTED, not a regression.** |
| ★ **every other fan-visible field** | ★★★ **exact match still holds** |
| **falsifier** | ★★ **a rise with `source != "route_exhausted"`**, or a rise on seeds where `stuck_strikes` never reached 3 |

★★★★★ **THIRD TIME TODAY the "unexplained mover ⇒ finding" bar has needed a
carve-out I had not derived.** *A blanket exact-match bar is worth exactly as much
as the enumeration of what legitimately moves — and I keep publishing the bar
before finishing the enumeration.* ★ **Registered BEFORE the fan runs, which is
the only thing that makes it a prediction rather than an excuse.**

### ★★★★★★★★ D5 **WITHDRAWN** — I REGISTERED A DEFECT AS EXPECTED BEHAVIOUR

**Fable held C3 on the store's-unit question** *(a bed is not a designation
region; `blocked_regions`' unit IS designation regions; the haul producer was
rejected for this exact mismatch)*. ★ **The read, four facts:**

| # | fact | site |
|---|---|---|
| 1 | ★★ **the producer IS reachable for self-jobs** — standalone `if`, and the increment's comment says **"Kind-agnostic and survives re-claim"** | `12248`, `12074` |
| 2 | **guard is `designated.iter().find(\|r\| r.contains_point(job.pos))`; for `RestAt`, `job.pos == bed_pos`** | `12250`, `5443` |
| 3 | ★★★★★ **`designated` is pruned ONLY on explicit CANCELLATION** *(exact AABB subtraction)* — **a COMPLETED designation stays forever** | `5106` |
| 4 | ★★★ **so a bed inside a once-painted build region still matches** | — |

**TWO BRANCHES, BOTH BAD:** ★★ *bed outside any painted region* → `None` → **nothing
recorded, D5 vacuous**; ★★★★★ *bed inside a painted, possibly long-COMPLETED build
region* → **the entry is attributed to THAT region — a finished designation
reported "blocked" because a colonist couldn't reach a bed inside it.**

> ## ★★★★★ **THE GUARD ASKS "WHICH PAINTED REGION HAPPENS TO CONTAIN THIS POINT."
> A COINCIDENCE TEST, NOT AN OWNERSHIP TEST.**

★★★★★★★ **And the codebase already named this class** — task #55's comment under
`5106`: *"leaving the stale entry would report 'blocked' on a designation the
player already erased."* ★ **Same objection, four hundred lines apart, handled for
cancellation and unhandled for self-jobs.**

**RECOMMENDED: gate `route_exhausted` to WORK KINDS at `12248`** *(matches C1's
shape — per-kind gate at the consumer, named-case comment)*. ★★ **The self-job
visibility gap files as its own row: *"colonists repeatedly fail to reach a bed"*
is a real thing a player should hear, and `blocked_regions` is the wrong store to
say it in.**

### ★★★★★ WHAT THIS DOES TO THE BAR — AND TO ME

**With that gate, self-jobs never feed `blocked_regions`, so
`b5_blocked_regions_count_*` RETURNS TO EXACT MATCH and the carve-out
disappears.**

> ★★★★★★★ **I REGISTERED D5 AS AN EXPECTED MOVER FOR A CHAIN THAT SHOULD NOT
> EXIST.** *The carve-out was correct about the mechanism and wrong about whether
> the mechanism was legitimate.*

★★★ **A pre-registered delta can LAUNDER A DEFECT INTO "EXPECTED BEHAVIOUR."**
★★ *Pre-registration is what makes a mover honest — but only after the prior
question:* ★★★★★ ***should this move at all?*** ★ **I registered the carve-out
instead of asking whether the chain was correct — the same move as publishing the
bar before finishing the enumeration, one level up.**

> ★★★★★ **BEFORE REGISTERING AN EXPECTED DELTA, ASK WHETHER THE PRODUCING CHAIN
> IS ITSELF CORRECT. A registration EXPLAINS a mover; it must never EXCUSE one.**

★ **C1 is ruled the same way** *(Fable, #73: NO for self-jobs — beds and meals keep
strict `2.5`)*, **so C1's consequences are withdrawn too.** ★★★ **C2 applies as
designed.** ★★ **Net: the exact-match bar is restored WHOLE, with no carve-outs —
which is what it should have been once both chains were read rather than
registered.**

## ★ SEQUENCING

1. ★★★★★ **Capture the pre-site-6 `settle_invariant_holds` reading** *(in flight —
   perishable, blocks D1's meaning, not the build).*
2. **Site 6 builds** *(ownership field · sweep owner-liveness · re-claim lookup).*
3. **Site 4's `despond_resume` deletion** — ★★ *now a genuine consequence, which is
   what family 1 was always supposed to be.*
4. **Site 5** — the unit test, rewritten LAST against what sites 1-4 actually do.
5. **Fan + diff against D1-D4.**

> ★★★ **The counter needs NO changes** *(see the retraction in
> `AUTON2-ACCEPTANCE-FIXTURES.md` — it counts REMOVALS, so it is already the free
> detector for the sweep race).*
