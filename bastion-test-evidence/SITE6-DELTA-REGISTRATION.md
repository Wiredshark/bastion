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
