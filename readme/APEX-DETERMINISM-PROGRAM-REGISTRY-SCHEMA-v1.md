# APEX Determinism Program Registry Schema v1

Frozen vocabulary for `readme/APEX-DETERMINISM-PROGRAM-REGISTRY-v1.json`,
implementing `APEX-A.3`
(`PROJECT-BASTION-APEX-MICROSTEP-APEX-A.3-PROGRAM-REGISTRY.md`).

Determinism story: the registry is a pure function of (the canonical guide's
row list, the finding matrix's replacement-row references, each row's own
`hard_dependencies` as explicitly declared here — never inferred from
document position). `sequence_index` is presentation order; `hard_dependencies`
is the only thing the validator treats as an architectural edge. A row
earlier in the document is not automatically a dependency of a later one.

## 1. Authority split (section 5.1 of the packet)

```
canonical guide   -> human-readable sequence and architecture
program registry  -> machine-readable graph, trace links, terminal state
row packet        -> exact implementation micro-steps and acceptance contract
finding matrix    -> live finding disposition and closure-rule input
raw evidence      -> test/build outputs and attestations
```

No artifact silently replaces another. The registry never asserts an
implementation is done; it asserts that the *graph* (dependencies, finding
coverage) is internally consistent.

## 2. `ApexRowStatusV1` — five separate dimensions, never one `DONE` field

```
specification:      SPECIFICATION_COMPLETE | NEEDS_DESIGN
microstep_research:  MICROSTEP_RESEARCH_COMPLETE | NOT_APPLICABLE
implementation:      NOT_STARTED | BLOCKED_ON_PREREQUISITE | BLOCKED_ON_DESIGN
                      | BLOCKED_ON_EVIDENCE_GATE | DEFERRED | IN_PROGRESS
                      | IMPLEMENTATION_READY | IMPLEMENTED
verification:        NOT_STARTED | IN_PROGRESS | VERIFIED
deployment:          NOT_DEPLOYED | DEPLOYED
```

A row may never claim `IMPLEMENTED` while `specification == NEEDS_DESIGN`,
and may never claim `DEPLOYED` without `verification == VERIFIED` and a
non-`PENDING_ROW_PACKET` `rollback_plan_status` (`NOT_APPLICABLE_WITH_RATIONALE`
is an allowed alternative for evidence-only rows).

## 3. `CompletenessStatus` (source/evidence/rollback readiness)

```
PENDING_ROW_PACKET             — no row packet has landed yet; builder may not admit this row
VERIFIED                       — the referenced artifact was directly re-read at last_live_commit_checked
NEEDS_RECHECK                  — live commit advanced past the commit this field was verified at
NOT_APPLICABLE_WITH_RATIONALE  — this row has no source surface / evidence / rollback need, and why
```

## 4. `FindingClosureRuleV1` — exactly one rule per finding

```
Row          { row }                    — one row fully closes this finding
AllOf        { rows }                   — every listed row must close before the finding closes
AnyOf        { rows, rationale }        — forbidden unless a row packet documents that each
                                           alternative independently closes the full finding
SupersededBy { rows, reason }           — original finding not implemented literally; risk
                                           tracked by replacement rows; NOT closure
```

## 5. `ApexRowRecordV1` / `ApexFindingTraceV1` fields

See the packet's section 8 for the exact struct. Notable invariants the
validator enforces:

- `hard_dependencies` graph is acyclic.
- Every `hard_dependencies` entry names a row that exists in `row_order`.
- `row_order` is a valid topological order of the `hard_dependencies` graph.
- Every finding in the matrix has exactly one closure rule.
- For `Row`/`AllOf`/`AnyOf`, every referenced row's own `finding_ids` contains
  that finding (reverse-link equality) — unless the referenced row itself
  does not exist, in which case it is reported as an **unresolved row
  reference**, not silently dropped.
- No row is orphaned from the guide (every `row_order` entry has a record and
  vice versa).

## 6. `APEX-T5.5` — `GUIDE_MISSING_ROW` (2026-07-26) → `CONFIRMED_PHANTOM` terminal (2026-07-26)

`APEX-A.2`'s finding matrix cited `APEX-T5.5` as a replacement row for
`DET-WTH-010`, `DET-PRD-008`, and `DET-PRD-011`, but no `APEX-T5.5` row
existed anywhere in the canonical guide (Tier 5 only defines `T5.1`–`T5.4`).
Fable's first ruling: add `APEX-T5.5` as a frozen `GUIDE_MISSING_ROW`
placeholder — reserved ID, empty `hard_dependencies`, fixed title stating no
packet exists, **no fabricated content**. Content recovery was routed to the
guide's author via Ben; a builder was never to reconstruct it.

That placeholder was itself a tracked-red (M3A-style): its frozen fields
were checked on every validation run so that content silently
appearing/disappearing on a "reserved, pending recovery" row would be
caught, not silently accepted.

**Terminal ruling, same day:** Ben confirmed the ChatGPT-routed artifacts
that were supposed to carry the recovery were hallucinated — there is
nothing upstream to recover. Fable ruled `APEX-T5.5` `CONFIRMED_PHANTOM`
(terminal, `status.specification == "CONFIRMED_PHANTOM"`): the row will
never gain a packet, its `finding_ids` and `hard_dependencies` are now both
frozen empty by construction (no live finding may cite it, nothing may
depend on it), and the three findings that used to include it in an `AllOf`
closure rule had that citation voided and re-derived from their remaining
real replacement rows (`readme/apex/APEX-FINDING-STATUS-MATRIX-v1.csv`).

The old per-row `GUIDE_MISSING_ROW` fingerprint-drift check
(`check_guide_missing_row_fingerprints`) is retired — a row confirmed to
never have existed and never recur has nothing left to drift-watch for.
`tools/validate-apex-program-registry.py` instead carries a general
`check_confirmed_phantom_invariants`: any row with
`status.specification == "CONFIRMED_PHANTOM"` must have empty
`hard_dependencies`, empty `finding_ids`, and no other row may hard-depend
on it. This is the same tracked-red spirit generalized past one hardcoded
row — the thing worth catching was never "T5.5's title changed," it was
"a phantom row silently grew a live reference."

Similarly, `APEX-T4.3`'s `ORDER_VIOLATION` (it depended on `APEX-T6.2`,
which the guide documents after `APEX-T4.3`'s own tier) was resolved by
**splitting T4.3** into `APEX-T4.3a` (kept the original tier position,
depends only on `T0.5`) and `APEX-T4.3b` (re-sequenced to after Tier 6,
depends on `T6.2`) — see `readme/APEX-DETERMINISM-PROGRAM-REGISTRY.md` for
the full rationale, including why `APEX-T4.5` was re-scoped to depend on
`T4.3a` alone rather than both halves. **Final, 2026-07-26:** same as
`APEX-T5.5` above, this split was never going back upstream for
ratification against a canonical guide revision — Fable's ruling is the
final word, `T4.3a`/`T4.3b` are locally-canonical row IDs going forward.

## 6a. `FLEET_AUTHORED` — fleet-written specs replacing unrecoverable packets (2026-07-27)

Ben ordered (2026-07-27, routed via Fable): rows whose standalone packet
was never real (hallucination-class, same root cause as `APEX-T5.5`'s
original citation and the `APEX-T0.2`/`T0.1` pin/vector issues resolved
2026-07-26) are no longer waited on. The fleet authors a replacement
packet directly, grounded only in the master build order's own row text
(never the invalid standalone files), the verified finding matrix, and
live code. `status.specification = "FLEET_AUTHORED"` marks this: distinct
from `SPECIFICATION_COMPLETE` (a real delivered packet) and from
`NEEDS_DESIGN` (no packet content exists at all) — a fleet-authored spec
exists and is intended to be build-grade, but went through a fleet
author→reviewer cross-review gate instead of external delivery. A row
must not move past `FLEET_AUTHORED` to `IMPLEMENTED` without that
cross-review actually happening (author ≠ reviewer at both the spec-review
and build-approval layers) — this is process discipline, not yet a
validator-enforced invariant.

## 6b. `T1.1-INCOMPLETE-NEEDS-NIX-LANE` — partial-implementation sentinel (2026-07-27)

`status.implementation`'s frozen vocabulary (section 2) has no value for
"some of this row's sub-steps have real landed code, but the row as a
whole is not done" — every prior row either used `NOT_STARTED` (even once
built, a known, program-wide bookkeeping gap this program has generally
deferred rather than fixed inline) or, once genuinely complete, would use
`IMPLEMENTED`. `APEX-T1.1` is a real, concrete case of the middle state:
`T1.1.02` (environment-first build-identity stamping) is landed and
verified (`bastion-harness/build.rs`, live on `bastion/apex`, doc-tagged
`APEX-T1.1.02` in the code itself) — real code, not a stub — but the row
as a whole is not complete (the Nix harness package / source-neutral VM
lane it also specifies has not landed). Recording this row as
`NOT_STARTED` would be simply false (T1.1.02 is used as a load-bearing
prerequisite by `APEX-T1.2`'s own build-identity stamping); recording it
as `IMPLEMENTED` would overclaim work that has not happened. Per Fable's
ruling: `status.implementation = "T1.1-INCOMPLETE-NEEDS-NIX-LANE"` names
the honest aggregate — landed-but-partial, nix-gate-pending — rather than
force-fitting either frozen-vocabulary extreme. This is a
row-specific sentinel (unlike `FLEET_AUTHORED`/`CONFIRMED_PHANTOM`, which
are general patterns other rows may reuse); it is expected to be
superseded by `IMPLEMENTED` once `T1.3`/`T1.4`'s reproducibility work
(which depends on the Nix lane) completes, at which point this row
updates again at that boundary, not before.

## 7. Temporary serialization

Before `APEX-T0.2` lands: UTF-8 JSON, object keys sorted, arrays in
registry-defined order, no floats, newline-terminated, SHA-256 over the exact
bytes, schema label `bastion.apex-program-registry/v1-json-interim`. `T0.2`
may migrate the encoding without changing IDs or semantics.

## 8. Guide row-count note

The packet's own header states "52 row records"; the canonical guide as
currently read contains 53 distinct row IDs (`APEX-A.1`–`APEX-A.3`, `APEX-T0.1`–`T0.5`,
`T1.1`–`T1.5`, `T2.1`–`T2.5`, `T3.1`–`T3.6`, `T4.1`–`T4.6`, `T5.1`–`T5.4`,
`T6.1`–`T6.6`, `T7.1`–`T7.5`, `T8.1`–`T8.5`, `T9.1`–`T9.3`). This registry uses
the actual counted set (53) rather than force-fitting to the packet header's
number.
