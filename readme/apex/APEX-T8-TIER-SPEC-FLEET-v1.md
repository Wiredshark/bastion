# APEX-T8 — World-generation economy recurrence evidence and remedy selection (fleet-authored spec v1)

Authored by Builder Opus 5 on `bastion/apex-t34` @ `6c631e973d`, from the
master-order rows `APEX-T8.1`..`T8.5`, grounded in live code reads at that
tip. Symbols cited were read, not recalled.

**The tier's thesis.** T8 is a *diagnosis* tier that ends in one
decision, and its structure is the whole point: three independent lanes
that each isolate one cause (numeric portability, order sensitivity,
model sensitivity), then a decision ladder that picks the **smallest**
remedy the measured evidence justifies. The rows explicitly forbid
choosing a remedy by preference — T8.4's acceptance criterion is
"remedy selection is based on measured model behavior, not crate
preference".

The reason for three lanes rather than one investigation: the worldgen
economy runs 500 simulated years, and by the end a divergence's *cause*
and its *magnitude* have no relationship. One ULP at year 3 and a
transposed reduction at year 400 both present as "the economies differ".
Only lanes that vary exactly one thing can tell them apart.

---

## Shared failure surface (verified)

**The simulation is 2000 sequential phases over parallel sites.**
`simulate_return` (`world/src/site/economy/context.rs:151-170`) loops
`HISTORY_DAYS / TICK_PERIOD` iterations with `TICK_PERIOD = 3.0 *
DAYS_PER_MONTH` (three months, `:37`) and `HISTORY_DAYS = 500.0 *
DAYS_PER_YEAR` (500 years, `:38`) — 2000 phases. Each phase calls `tick`
(`:197`), which runs
`index.sites.par_iter_mut().for_each(|(site_id, site)| ...)`
(`:209-215`) — **per-site economy ticks execute in parallel**.

That is the row's "three-month phase" boundary, verified: the phase
structure T8.1 wants to hash already exists in the loop, so the hashing
row does not have to invent a segmentation.

**Two order-sensitive seams inside each phase.** With `INTER_SITE_TRADE`
on, the same `tick` does:

- `for (id, deliv) in index.trade.deliveries.drain()` (`:200`) — delivery
  distribution
- `for (i, mut v) in site.economy_mut().orders.drain()` (`:219`) — order
  collection, then trade at sites (`:224+`)

`drain()` over map-shaped collections yields implementation order, and
`hashbrown::HashMap` is in use in this module (`:246`). Combined with the
parallel per-site tick, a single phase has at least three candidate
order-dependencies: Rayon partitioning of site ticks, delivery drain
order, and order drain order.

**Why this tier exists rather than "just fix it".** Each of those seams
could be canonicalised in an afternoon. Doing so *before* the evidence
would be exactly the mistake T8.5 forbids: it presumes the cause is order
rather than arithmetic or model instability, and it spends the cheapest
remedy without knowing whether it is sufficient — or whether it silently
changes generated worlds for every existing save.

---

## T8.1 — Per-phase economy baseline evidence

**Objective.** Localise any worldgen economy divergence to one phase and
one state transition.

**Selected architecture.** Define a canonical representation of site /
good / labor / order / delivery state — canonical meaning a fixed field
order and a fixed traversal, independent of the collections' own
iteration. Hash **every** three-month phase, not just the endpoint, plus
a final 500-year baseline root. Per-phase hashing is what turns "the
worlds differ" into "they diverged at phase 412".

Record the first differing site, phase, field, and branch. Feed the final
economy root into `WorldBaselineManifestV1` (T4.3), which is where it
becomes something a save can be checked against.

Separate raw numeric from semantic-quantised evidence — the same probe
pair as T5.3 and T6.2, and the same rule: the semantic one never
certifies the raw one. This is the third tier to need that pair, which is
the argument for the shared types the T6 spec already asks for.

Add fixture worlds and a **stable traversal baseline** — a fixed
traversal is the control that makes the other two lanes interpretable.

**Required tests.** A deliberately perturbed phase is localised to that
phase, not to the endpoint; the canonical representation is invariant
under collection iteration order (the test that proves "canonical" is not
just "whatever the map yielded"); the same fixture world hashes
identically across runs on one binary.

---

## T8.2 — Lane A: numeric portability

**Objective.** Isolate cross-platform arithmetic from non-commutative
order.

**Selected architecture.** Freeze canonical traversal and input state —
then vary *only* the compiler, profile, and target cell. Run the same
algorithm across those cells and compare per-phase raw and semantic
probes. Identify the first arithmetic function or operation that differs.

**The lane's discipline, restated because it is the whole design: do not
permute orders in this lane.** A lane that varies two things measures
neither. Frozen traversal is not a convenience here; it is the control.

This lane consumes T6.4's `NumericProfileV1` — the cells *are* numeric
profiles, and the lane is meaningless without a precise tuple naming what
differs between them.

**Required tests.** Two cells with identical profiles produce identical
per-phase probes (the null result that validates the harness); a cell
differing in one recorded profile field is attributed to that field; the
harness fails loudly if traversal is not actually frozen — otherwise a
silent permutation contaminates every result in the lane.

---

## T8.3 — Lane B: order sensitivity

**Objective.** Every order-dependent transition has a reproducible
minimal fixture.

**Selected architecture.** One binary, one profile, one platform — the
mirror discipline of Lane A. Permute **separately**: site order, order
order, provider/customer pairing, delivery order, and reduction order.
Separately is load-bearing: permuting two at once cannot attribute.

The live seams named above are the starting inventory — the Rayon
partitioning at `context.rs:209`, the delivery drain at `:200`, and the
order drain at `:219`.

Classify each finding as **transactional non-commutativity** (A-then-B
genuinely differs from B-then-A: a stock is consumed, a customer is
served first) versus **reduction rounding** (float summation order). The
distinction decides the remedy: the first needs a canonical order, the
second may need a canonical *reduction* (fixed accumulation order, or
fixed-point). Conflating them leads to canonicalising an order that was
never the problem.

Test saturation and overflow caps and last-writer fields explicitly —
those are where a small ordering difference becomes a large state
difference rather than a rounding one.

**Required tests.** Each permutation axis independently; a minimal
fixture per order-dependent transition found (the acceptance criterion is
"reproducible minimal fixture", so a finding without one is incomplete);
a saturation cap reached in different orders.

---

## T8.4 — Lane C: model sensitivity

**Objective.** Remedy selection based on measured model behaviour.

**Selected architecture.** Perturb **exactly one field at one named
phase** by one ULP or one quantisation unit, holding executable and
traversal fixed. Record the first branch crossing and the final economic
magnitude — the gap between those two is the tier's central measurement.

Sweep price, stock/demand, surplus, population, and smoothing state.
Produce sensitivity curves and an **unstable threshold inventory**: the
list of thresholds where a one-ULP input difference flips a branch.

This lane is what makes T8.5 a decision rather than a preference. If a
one-ULP perturbation at phase 10 produces a bounded difference at phase
2000, the cheapest remedies suffice. If it produces an unbounded one, the
model is chaotic and no amount of canonical ordering saves it — the
remedy has to be quantisation or fixed-point, and that is a gameplay
decision requiring review.

**Required tests.** Perturbation harness reproducibility (the same
perturbation twice gives the same curve); a null perturbation produces a
zero curve; at least one known-unstable threshold is found or the sweep's
coverage is proven insufficient rather than assumed adequate.

---

## T8.5 — `EconomicNumericProtocolV1` decision

**Objective.** Choose the smallest remedy that satisfies declared
portability and save goals given measured sensitivity.

**This is a decision row, `NEEDS-DESIGN until evidence`.** The ladder, in
the row's order, cheapest first:

1. same-profile certification
2. canonical order
3. phase-boundary authoritative quantisation
4. deterministic price-response kernel
5. fixed/decimal stored state
6. persisted generated baseline

**Choose only the smallest option that satisfies the declared goals.**
Each rung costs more and changes more: (2) changes generated worlds, (3)
changes economic values, (4) changes model behaviour, (5) changes the
save format, (6) abandons regeneration entirely. A tier that jumps to (5)
without evidence has made a permanent decision on a hunch.

**The row's constraint that most needs preserving:** *re-derive only
declared caches, never path-dependent stocks, population, or history*.
Option (6) is attractive precisely because it makes regeneration
unnecessary — but anything path-dependent cannot be re-derived at all, so
the boundary between "cache" and "history" must be declared before any
option that re-derives is chosen. Getting that boundary wrong silently
rewrites player worlds.

**Interaction with T4.** Option (6) is a save-manifest change and lands
on T4.6's staged epochs; option (3) or (5) changes the world baseline
root and therefore T4.3. Neither should be chosen without the T4 sequence
being ready to carry it.

---

## Cross-tier notes

**Ordering.** T8.1 gates the tier; T8.2, T8.3 and T8.4 are `READY after
T8.1` and are genuinely parallel — three independent lanes, each with its
own control. T8.5 consumes all three.

**Do not pre-empt the lanes.** The order-canonicalisation fixes at
`context.rs:200/209/219` are cheap and tempting. Landing them before Lane
B measures them would destroy the evidence Lane B exists to produce, and
would change every generated world without knowing whether the change was
necessary or sufficient. If an interim fix is wanted for other reasons,
it should be recorded as a deliberate evidence cost, not slipped in as
cleanup.

**Shared probes, third instance.** T5.3, T6.2 and T8.1 each need a
raw/semantic probe pair with the same non-certification rule. Three
instances is enough to say it belongs in one place; the T6 spec's
shared-type request should be honoured here rather than re-derived a
third time.
