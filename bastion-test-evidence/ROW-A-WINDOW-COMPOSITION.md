# ROW A — WINDOW COMPOSITION CALL

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

**6 of 7 ride, one is excluded, and item 5 turns out to be paid for by Row A
itself** — the finding-72 field Row A must add anyway is the lateral visibility
`b5_55_diag` was waiting on.

## §4 — THE CALL

**★ SUPERSEDED BY §3b — the call is now ROW A + items 1–6; only item 7 is
excluded.** The reasoning below stands as the argument for *why the window must
be verifiable*; §3b corrects *which items meet that bar.* Left in place rather
than rewritten, per house style.

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
