# ROW A — WINDOW COMPOSITION CALL

> ## ⛔ BLOCKING PREREQUISITE — READ BEFORE RUNNING ANY FAN
>
> **The mandatory hold-check has NO VALID REFERENCE BASELINE today.** The window's
> entire composition argument rests on a check that cannot currently be executed.
> **A pre-Row-A baseline fan must be captured first.** Detail in §0.

## §0 — ⛔ THE HOLD-CHECK HAS NO BASELINE (found 2026-08-04, before the build landed)

**The check I made mandatory:** *every field present before this window holds its
previous value.* **Executing it requires a structured per-seed capture at the tip
Row A builds from.** There isn't one.

| candidate baseline | commit | problem |
|---|---|---|
| **wave24** (most recent fan) | `d3235e5329` | ★ **LOGS ONLY.** The raw dir holds four `bastion-pool-*.log` files and **no `*_FULL.json`.** No per-seed structured data exists, so it cannot support a field-level comparison at all. |
| **wave19** (newest `_FULL.json`) | pre-`d3235e5329` | **Predates the tip by more than ten commits** *and* by nine corpus fields (below). |
| a fan at `a85dec2912` | — | **Does not exist.** No fan has been run at the tip Row A builds from. |

**Ten commits sit between the last fan and the build tip** (`d3235e5329..a85dec2912`),
and they are not inert — diffing `bastion-harness/src/main.rs` across them shows
**nine added corpus fields**:

```
b5_access_plan_self_rescue_calls        b5_access_plan_self_rescue_emissions
b5_access_plan_emergency_calls          b5_access_plan_emergency_emissions
b5_access_plan_proactive_descent_calls  b5_access_plan_proactive_descent_emissions
b5_access_plan_self_rescue_starved      b5_access_pending_true_ticks
b5_live_is_access_count
```

> **★ Comparing Row A's output against `wave19` would show those nine
> already-landed fields as "new", alongside Row A's own additions — and the
> hold-check would be measuring two changes it cannot separate.** That is exactly
> the unfalsifiable re-baseline the enumerable-delta criterion exists to prevent.
> **The criterion would have been satisfied on paper and void in practice.**

### The fix — cheap, and it pays for itself twice

**Run a baseline fan at the pre-Row-A tip, capturing `_FULL.json`, before the
Row-A fan.** Then the comparison is a true pair: same commit for everything
except the window's own delta.

**It is not extra cost — it is cost already owed.** Those ten commits landed
**local-pins-verified but never fanned** (correct under *local=pins, VMs=fixtures*
plus self-gate-on-green — not a process breach). The baseline fan therefore
**also retroactively validates all ten**, including the nine new fields, which
nothing has yet checked at corpus scale.

### ★★★ AND THE GENERAL RULE THIS EXPOSES

**wave24 ran, produced a verdict, and preserved only its logs.** It can never
serve as a baseline for anything, because the body was discarded and only the
conclusion kept.

> **A FAN THAT PERSISTS ONLY ITS VERDICT IS NOT A BASELINE. Every fan must
> persist per-seed `_FULL.json`, not just the fanlog** — otherwise it is a
> conclusion with no evidence behind it, and no future field-level comparison can
> reference it.

Same family as `wave13_FULL.json` being `{}` — *an empty run shaped like a real
baseline* — and the same law throughout: **an exclusion and an absence must not
render identically.** Here, *"the fields held"* and *"the fields were never
recorded"* both present as a clean fanlog.

**Standing capture requirement, effective now:** `_FULL.json` per seed, plus the
`COMMIT=` attestation line, or the wave is a verdict and not evidence.

**Decision owner:** me (composition), on 5b's sizing (build-readiness).
**Rule as issued (Fable):** *all backlog items build-ready ⇒ Row A rides with
them, one re-baseline pays for everything; any item needs design work ⇒ Row A
goes alone and small, so Row B's gate is not held hostage to stragglers.*

**Recommendation: SPLIT ON A DIFFERENT AXIS THAN THE RULE ASSUMES.** Not
build-readiness — **additive vs mutating.** Reasoning below; the rule's intent is
preserved and its failure mode is removed.

---

## §1 — THE BACKLOG (7 items, from `DAY-CLOSE-2026-08-04.md` §7)

| # | item | changes the schema how? | class |
|---|---|---|---|
| 1 | `growth_rose` watch-precondition | **adds** a baseline field | **ADDITIVE** |
| 2 | unsatisfiable-watch sweep (5 watches) | **adds** a baseline beside each watch flag | **ADDITIVE** |
| 3 | `b55`'s unemitted baseline | **adds** a baseline field | **ADDITIVE** |
| 4 | seed 66's sentinel (`.unwrap_or(0.0)`) | **changes an existing field's type/null-ness** | **MUTATING** |
| 5 | `b5_55_diag` (inert constant diag) | renames / retires an **existing** field | **MUTATING** |
| 6 | constant build/`b15` fields | renames / retires **existing** fields | **MUTATING** |
| 7 | probe container normalization (chop dict vs mine list) | **changes an existing field's container** | **MUTATING** |

**Row A's own additions** — `bastion_blocked_regions_count()` emitted in the b5
output, `blocked_sources` at the specimen cells, `stuck_strikes` at the specimen
cells — are all **ADDITIVE**.

> **Marked as MY classification, from the fix shapes recorded in
> `SCENARIO-MAP.md` — not from 5b's sizing.** Items 1–3 share one written fix
> shape (*"emit the baseline beside every watch flag, three-way treatment"*) with
> a one-line precedent (`auton`'s `storm_baseline_captured`). **Item 7 is the one
> I suspect carries real design work** — `SCENARIO-MAP.md:228` says b55-deep
> *"cannot say anything new until the verdict/diag split lands"*, and a
> verdict/diag split is schema restructuring, not a one-liner. **5b's sizing
> decides that; my classification does not.**

---

## §2 — WHY BUILD-READINESS IS THE WRONG AXIS

**Every schema change forces a re-baseline.** Additive changes do **not** escape
that — a new field makes the output differ from the frozen baseline exactly as a
renamed one does. So "one re-baseline pays for everything" is true of *any*
grouping, and it is not what distinguishes the two sets.

**What distinguishes them is whether the re-baseline can be VERIFIED.**

| window contents | after re-baselining, can you check it? |
|---|---|
| **additive only** | **YES — mechanically.** Every pre-existing field must hold its **old value**. Any drift on an old field is a **bug**, full stop. |
| **contains mutating items** | **NO.** The mutated fields are *supposed* to change, so on those fields "changed as intended" and "changed by accident" are **indistinguishable**. |

> **★ An additive-only window re-baselines with a free, total correctness check.
> Mixing in one mutating item does not weaken that check — it DELETES it for the
> fields that item touches.**

This is the campaign's own recurring law in a new place: **an exclusion and an
absence must not render identically.** Here it is *intended* change and
*accidental* change rendering identically, on precisely the fields nobody can
re-derive.

---

## §3 — AND IT IS ROW A's OWN FALSIFIER THAT PAYS THE PRICE

Row A's central structural claim is that it is **report-only**. The registered
free falsifier is:

> **Row A moves no release path, so `Other` must stay at its current value. If
> `Other` moves at all, Row A is not report-only and the split is wrong.**

**That falsifier is exactly an "old field holds its old value" check.** So is
each of the three G3 pre-registrations:

- `b5_55_blocked_by` stays `None × 48` — **if it moves, something is wrong**
- `b5_ch_base_blocked_sources` holds `['plan_access'] × 3` **exactly**
- the new count is `0` on clean seeds

**Four pre-registered checks, all of the form "an existing field must not
move."** A window carrying mutating items is a window in which that form of
check is no longer sound in general — and these four are the entire evidentiary
basis for the row split that Row B's gate depends on.

> **Putting Row A in a mutating window trades away the verification that makes
> Row A worth landing first.**

---

## §3b — ★★★ CRITERION REVISED — 5b's sizing showed ADDITIVE/MUTATING is a PROXY

**5b classified seed 66's sentinel fix as build-ready. Under §2's rule it is
MUTATING and therefore out.** Both readings are defensible, which means the rule
is not cutting at the joint. It isn't.

**The sentinel fix changes `tool_stone` from `0.0` to a distinguishable value —
but only on the seeds whose probe actually failed, and we KNOW WHICH THOSE ARE.**
So its delta is:

> *`tool_stone` changes `0.0 → <sentinel>` on **exactly seed 66**, and on no
> other seed. Everything else identical.*

**That is checkable in advance, field by field and seed by seed.** The
re-baseline is just as verifiable as an additive one — the check is simply
*"matches the enumerated delta"* instead of *"unchanged."*

> **★ THE REAL CRITERION: CAN THE EXPECTED DELTA BE ENUMERATED — FIELD BY FIELD,
> SEED BY SEED — BEFORE THE RUN?**
>
> If yes, the re-baseline is verifiable and the item may ride, mutating or not.
> If no, the re-baseline is unfalsifiable and the item must not.

**Additive-only was a good proxy** — its delta (*"these N new fields appear,
nothing else moves"*) is trivially enumerable — **but it excluded verifiable
mutating changes and would have admitted an unspecified additive one.** The
proxy is retired in favour of the thing it was standing in for.

**And it disposes of item 7 by a better argument.** *Probe container
normalization* is out **not because it is mutating** but because **5b could find
no spec for it** — in either doc, or in the harness source. **An unspecified
change has no enumerable delta by definition**, so its re-baseline could not be
checked no matter what shape it turned out to take. *Same verdict as §2 reached,
on a reason that survives scrutiny.*

**This is the second time tonight a criterion of mine was a proxy for something
sharper, and both times the correction came from someone reading the actual
artifact rather than reasoning about it.** 5b went and read the backlog instead
of accepting *"the ~10 items"* — there are seven, named.

### Revised classification (5b's sizing + the delta criterion)

| # | item | delta enumerable? | in? |
|---|---|---|---|
| 1 | `growth_rose` watch-precondition | new field, nothing moves | **IN** |
| 2 | unsatisfiable-watch sweep (5 watches) | 5 new fields, nothing moves | **IN** |
| 3 | `b55`'s unemitted baseline (`remainder_before`) | new field, nothing moves | **IN** |
| 4 | seed 66's sentinel | **yes — named seeds, named fields** | **IN**, with the delta enumerated in the manifest |
| 5 | `b5_55_diag` | ★ **no delta of its own** — 5b: its doc says these fields *"need the LATERAL ENTRY, not a repair."* **Satisfied BY Row A's required field**, adds no scope | **FREE** |
| 6 | constant build/`b15` fields | 5b: documentation/context, **not new logic** — no schema delta | **FREE** |
| 7 | probe container normalization | **NO SPEC FOUND** ⇒ not enumerable | **OUT** |

## §3c — ★★ FINAL: **ROW A + ITEMS 1–3.** §3b over-admitted two items.

**Corrected by 5b's SECOND pass, which checked source instead of re-reading the
docs — and which revised their own first sizing.**

### The criterion's final form (Fable, DECISIONS #55)

> **The verifiability axis is the DEFAULT, with one named exception:** a mutating
> item may join a window **if and only if it carries an exact pre-registered
> per-seed delta for every field it touches.** That is the merge bar (composition
> exact-match with intended deltas) generalized to schema windows.
>
> **The exception exists so the axis is never read as "mutating changes can't be
> verified."** They can — it just costs per-field registration instead of coming
> free.

### Item-by-item, final

| # | verdict | why |
|---|---|---|
| 1 `growth_rose` | **IN** | ★ **Source-verified.** `g1` is already a local (`main.rs:11151`), captured as the baseline and compared every tick (`g > g1`, 11157) — **never emitted**; only the derived `rose` flag (11206) is. **One line.** |
| 2 watch sweep (5) | **IN** | Same shape, applied mechanically. *Two independent spot-checks landed on the identical code pattern, so the doc's "same fix ×5" is corroborated rather than trusted.* |
| 3 `b55` baseline | **IN** | ★ **Source-verified.** `remainder_before` is already a local (`main.rs:4795`), computed and compared every run, **never reaching the JSON literal** — only `b55_remainder_progressed` (4894) does. **One line.** |
| 4 seed 66 sentinel | **OUT of window one** | Eligible under the named exception, but **nobody has written the per-seed registration** and there is no urgency to. Rides a later window once it carries its delta. |
| 5 `b5_55_diag` | **not a window member** | **Satisfied BY Row A's own build** — its doc says these fields need *the lateral entry, not a repair*, and Row A's finding-72 field is that entry. Not a parallel change; a consequence. |
| 6 constant build/`b15` | ★ **OUT — 5b revised their own sizing** | First pass called it *"documentation/context, not new logic."* Second pass: **"I still don't have a fix spec for this one."** No spec ⇒ no enumerable delta ⇒ out. |
| 7 probe container norm. | **OUT** | ★ 5b's identification: this is likely `SCENARIO-MAP:228`'s *"verdict/diag split"* — **the same restructuring already applied to six other scenarios** (the completed report-fix row 1–6 of 6). If so it is **schema surgery by construction**, and the suspicion of design work is confirmed rather than assumed. |

> **★ ITEM 6 IS THE ONE TO NOTE.** It was admitted in §3b on 5b's first sizing and
> is removed on their second. **The difference is that the second pass read the
> source.** *A classification derived from a doc's summary is a claim about the
> doc, not about the code* — the same distinction that has cost this campaign
> seven label-vs-content findings, arriving one more time in the bookkeeping.

**FINAL WINDOW: Row A + items 1, 2, 3.** Independently reached three ways —
Fable's ruling, 5b's source check, and this criterion.

## §4 — THE CALL

**★ SUPERSEDED — see §3c for the FINAL call: ROW A + items 1–3.** §3b widened
this to items 1–6 on 5b's first sizing; §3c narrows it back after their
source-level second pass. **The reasoning below stands as the argument for *why*
the window must be verifiable** — that part was never in question and Fable
upheld it as DECISIONS #55. Only *which items clear the bar* moved. Left in place
rather than rewritten, per house style.

~~**ROW A + items 1–3 (the additive baseline-emission family). Items 4–7 take
their own window.**~~

**Why this beats both branches of the binary rule:**

- **vs "Row A alone":** items 1–3 are the *same* fix shape applied five times,
  additive, with a one-line precedent. They cost almost nothing to carry and
  they inherit the same verifiable re-baseline. Sending them to a later window
  buys nothing.
- **vs "Row A with everything":** one mutating item deletes the additive
  window's free check on the fields it touches — including the fields Row A's
  own falsifiers are written against.

**Cost:** one extra re-baseline for items 4–7 later. **Bounded, scheduled, and
paid in VM minutes.** The alternative risks an unbounded stall of the window
Row B's gate sits behind, and — worse — a re-baseline nobody can check.

**Fable's intent is preserved exactly:** Row B's gate is not held hostage to
stragglers, and stragglers are now identified by a *property of the change*
rather than by an estimate of someone's remaining work.

### Conditions on the call

1. **5b's sizing can override the classification, not the criterion.** If item 7
   turns out to be a one-liner it still does not join — it is **mutating**. If
   item 2 turns out to need design, it drops out despite being additive.
2. **Frozen manifest** for whatever the window contains, before the run.
3. **Field-presence AND agreement guard** on every new field — presence catches a
   stale binary, agreement catches a miswired counter.
4. **`RUSTC_WRAPPER=""`** on the verification build. **`dev` profile** is the
   substitute of record while `target/no_overflow/build` stays denied.
5. **The post-re-baseline check is stated in advance and is mandatory** —
   **restated for the revised criterion:** *every field present before this
   window holds its previous value **except** the deltas enumerated in the
   manifest, which must match exactly.* **Item 4's entry must name the seeds and
   the fields** (`tool_stone` / `tool_steel`, seed 66 and any other failing-probe
   seed) **before the run, not after.** That check is the reason for the
   composition; skipping it forfeits the argument.
6. **Item 7 stays out until it has a spec.** If someone produces one, it is
   eligible on the same terms as item 4 — an enumerated delta — not on the
   grounds that it is small. **5b declining to certify it build-ready on a name
   alone is the correct call and is why it is excluded**; a one-line mention in
   two docs and no elaboration in the source is exactly the "confident label, no
   content" object this campaign keeps finding.

## §7 — WINDOW RESULT (closed; DECISIONS #57 derivation + #58 ruling)

**Fans:** `wave25_BASELINE_e86fe79893` / `wave26_ROWA_d5b56d1c79`, 48/48 seeds
each, `COMMIT=` attested on all 4 VMs per side, ~$0.29 each.

### The #57 derivation — expected movers DERIVED, never transcribed

| input (all fields OTHER than the one under test) | seeds |
|---|---|
| `b5_blocked_regions_count_at_settle > 0` | 52 54 61 62 66 71 80 85 90 92 |
| `blocked_sources` contains `route_exhausted` on a mine cell | **71 90** |
| already covered at baseline (baseline's own `blocked_by`) | 52 54 61 66 |
| **DERIVED** | **71 90** |
| **OBSERVED** | **71 90** |

**Set-equal both directions, zero extras** — and the derivation **explains the
eight non-movers**, which a transcription never could: the counter fired on 10
seeds, but on 8 the region is a chop region (`['plan_access']`) or an
already-covered mine region. *The mechanism accounts for the whole set, not just
the positives.*

### Final state

- **Report-only falsifier:** all 5 release counters moved **0/48**.
- **Outcome-neutrality:** **zero** top-level fields differ on any of the 48 seeds.
- **Cycle-neutrality:** **3 in-flight snapshot values** differ (2 `claimant`, 1
  `progress`) on seeds 54/90 — mechanism predicted by §G4: the `retain`'s
  `is_empty()` early-out is a *performance* guard, so a populated store pays the
  scan and shifts a snapshot by a fraction of a tick.
- **Ruling (#58):** outcome-neutral is the bar; **Row A ships**, with the
  packet's "report-only" claim amended to the measured scope.

> **★ THE CLAIM THAT SURVIVED IS NARROWER THAN THE ONE I WROTE.** "Report-only"
> asserted more than the evidence supports. **Outcome-neutral but not
> cycle-neutral** is what was measured, and it is what the packet now says.

### What the row actually bought

`blocked_by` resolves on `52 54 61 66` at baseline and `52 54 61 66 71 90` after.
**Seeds 71 (the frontier) and 90 (the holdout) — the two cells this whole
campaign was built around — are reported for the first time.** Seed 90's blocker
is `[17989, 9263, 338]`: the exact dead-end column the G1 scan investigated.
