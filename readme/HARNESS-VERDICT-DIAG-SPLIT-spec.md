# Harness report fix: make the verdict/diagnostic split STRUCTURAL

**Status:** spec, ready to build. **Scope:** 6 scenarios, `b58` first.
**Author:** Opus (reviewer lane), 2026-08-03. Read at tip `15850c61cc`.

## The finding this fixes

Conjunct variable names and emitted JSON key names are **decoupled, with
no enforced correspondence**:

| conjunct variable | emitted key | relationship |
|---|---|---|
| `beds_built` | `bed_built` | renamed |
| `gm_final` / `gb_final` | `selfgen_mine_total` / `selfgen_build_total` | renamed |
| `dug_before_preempt` | `preempt_dug_before` | renamed **and not a conjunct at all** |

Because the mapping is hand-made, **there is no mechanical way to tell a
verdict term from a diagnostic**, and readers guess. Three real
misreadings resulted: a diagnostic (`preempt_dug_before`) became the
brief for an investigation; a polarity trap (`auton_frozen: false`
reads green while `pass` requires `frozen == true`) was misread by two
people; and a disjunct (`b58_b_carve_fired: false`) was chased as a lead
when its term had already passed via `b_exited`.

## ★ The key point: the distinction ALREADY EXISTS, in prose

`b58` documents it repeatedly in comments:

- `// (q) is REPORTED, not gating (B6).`
- `// d_all_cleared + d_top_down: REPORTED not gating (B6-hotfix…)`
- `// REPORTED, never gates (a throughput/quality mechanism…)`

and gives principled reasons — load-sensitive throughput, execution
races that are genuinely nondeterministic on a loaded machine. **The
authors already invented verdict-vs-diagnostic and implemented it in
comments.** This spec does not introduce a distinction; it makes an
existing, deliberate one machine-readable.

## The design

**1. Two objects, not one flat bag.**
```
"b58": {
  "verdict": { …one field per CONJUNCT, named after it… },
  "diag":    { …everything else, incl. every REPORTED-not-gating field… }
}
```

**2. Keys are named after their conjunct.** `beds_built` emits as
`verdict.beds_built`, not `bed_built`. A rename is what defeats every
mechanical audit, including the scanner I wrote for this review — which
produced a false positive until I matched on **value expressions**
instead of names.

**3. ★ Composite terms: AND-composites are SPLIT, OR-composites EMIT THE
TERM.** These get **opposite** treatments, and the reason is exact.

**OR — emit the term.** `b58` has:
```rust
&& ((b_carve_fired && b_ladder_built) || b_exited)
```
Emit **`verdict.b1_free`** = the whole term's value, with all three
operands under `diag`. A reader then sees `verdict.b1_free: true` and
cannot mistake `diag.b_carve_fired: false` for a failure — the
"red that doesn't matter" class. **Splitting an OR into separate gating
terms would CHANGE THE VERDICT** (each operand would have to hold
independently), so it must stay whole.

**AND — split into separate gating terms.** `auton` had:
```rust
let frozen     = count == frozen_at && frozen_at > 0;
let path_alive = grants > 0 && peak_wait <= 7;
```
Split to `storm_baseline_captured` + `mine2_count_held`, and
`path_grants_nonzero` + `path_wait_bounded`. **Splitting an AND leaves
the conjunction identical, so the verdict provably cannot move** — and
you gain which-half localisation for free. A collapsed AND can only say
"something in here failed."

*(Corrects this spec's first version, which said to emit every composite
whole. That would have kept `path_alive`'s two halves fused for no
benefit — the OR case needs it, the AND case is strictly worse for it.)*

**4. Derive `pass` FROM the emitted verdict set — one source of truth.**
`b5`'s `failed_clauses` already does this. `pass = verdict.values().all()`,
and emit `failed_clauses` as the list of false keys. **A verdict term
that is not emitted then cannot exist**, which is the invariant that
makes the whole report auditable.

**5. Unset ≠ measured.** A diagnostic only assigned inside a branch that
may not run (`preempt`'s `jobs_at_rest_peak = 0usize`, set only inside
`if rest >= 0.58`) must emit `null`, not its initializer. **An emitted
`0` that means "never sampled" is indistinguishable from a measured
zero.**

## Scope, in order

**`b58` first — the most misleading surface**: 21 conjuncts, 19
diagnostics, plus the only known disjunctive term. Then `bed`
(one root failure cascading to 13 red), `preempt` (the unset-zero),
`b73`, `auton` (the polarity trap + a two-condition term), `selfgen`.

**`farm`, `run`, `b55` need NOTHING** — verified: every emitted key is
already a conjunct. Do not touch them.

## Acceptance

- **No scenario's pass/fail verdict changes on any seed.** This is a
  reporting refactor; a behaviour change means a mistake.
- `pass` is derived, never hand-maintained; a conjunct absent from
  `verdict` is a compile-or-test failure, not a silent omission.
- Every field a comment marks "REPORTED, not gating" lands in `diag`.
- Corpus fan after: aggregate holds, no novel modes.

## Flavour 7: NON-INDEPENDENT AGGREGATES (`b55-deep`)

```rust
let pass = functional_pass && runtime_hygiene_clean;   // 18 terms && 3 terms
```
`failsafe_hygiene_clean` appears in **both**. Harmless in an AND — but
the report presents the two as **orthogonal categories**, so one fault
flips both and reads as *"failed functionally AND on runtime hygiene."*
Sneakier than a cascade, because the categories claim independence:
**"how many categories are red" becomes actively anti-informative.**

**Fix:** promote the 21 conditions to real verdict terms, keep
`functional_pass` / `runtime_hygiene_clean` as **derived summaries under
`diag`**, and **de-duplicate `failsafe_hygiene_clean` to one term**, so
one fault reads as one fault. `emergency_access_after_soak == (0,0,0)`
is a tuple equality and gets the composite-term treatment — a red must
say which component moved.

## Cascade: distinguishing a root failure from its wake

`bed` shows **13 red conjuncts from ONE root failure** (`beds_built`).
The count of red fields carries no information about the number of
defects.

**Resolution, proportionate rather than universal:**

1. **Emit `failed_clauses` in declaration order, plus `root_failure` =
   the first entry.** For `bed` that names `beds_built` and the other 12
   read as its wake.
2. **This is a CONVENTION, not an invariant** — it holds only while
   declaration order follows dependency order. **A later reorder breaks
   `root_failure` silently.** Say so in the code, next to the field.
3. **Where a real dependency chain exists, annotate it explicitly**: the
   term declares its prerequisites, and `root_failure` = the failing
   terms whose prerequisites all passed. Do this for `bed` (12 terms
   require `beds_built`); **do not annotate all 39 scenarios** — most
   have no chain and the annotation would be noise.

The weakness is named on purpose: (1) is cheap and right most of the
time, (3) is precise and costs annotation. Use (3) only where the
cascade is real.

## Acceptance — verify EMPIRICALLY, not by inspection

- **Run all 6 scenarios on the SAME commit before and after the
  refactor, and byte-compare the `PASS`/`FAIL` lines and the verdict
  sets.** Diag-side diffs (renames, added `null`s, new fields) are
  expected and do not count.
- A verdict change means a mistake, not a finding. This is a reporting
  refactor.
- `pass` is derived, never hand-maintained; a conjunct absent from
  `verdict` must fail the build or a test, not vanish quietly.
- Every field a comment marks "REPORTED, not gating" lands in `diag`.

## Future work (recorded so it isn't re-derived)

**Split scenarios out of `bastion-harness/src/main.rs` into modules.**
At ~22k lines with every scenario in one file, two lanes cannot touch
different scenarios without a merge queue — which is the only reason
this row needed a file-ownership negotiation at all. The natural
follow-on once the report fix lands.
