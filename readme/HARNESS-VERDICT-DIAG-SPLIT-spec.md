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

**3. ★ Composite terms emit the TERM, with operands under `diag`.**
This is the fix for the "red that doesn't matter" class. `b58` has:
```rust
&& ((b_carve_fired && b_ladder_built) || b_exited)
```
Emit **`verdict.b_free`** = the whole term's value; put
`b_carve_fired`, `b_ladder_built`, `b_exited` under `diag`. A reader
then sees `verdict.b_free: true` and cannot mistake
`diag.b_carve_fired: false` for a failure. Same treatment for any
conjunct ANDing two conditions (e.g. `auton`'s
`path_alive = grants > 0 && peak_wait <= 7`) — emit the term, and both
operands as diag, so a red says *which half*.

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

## Cascade caveat (does not block this work)

`bed` shows **13 red conjuncts from ONE root failure**. The count of red
fields carries no information about the number of defects. A `verdict`
object makes this *more* visible, not less, so a later pass should
consider ordering or dependency-annotating conjuncts so the FIRST
failure is distinguishable from its cascade.
