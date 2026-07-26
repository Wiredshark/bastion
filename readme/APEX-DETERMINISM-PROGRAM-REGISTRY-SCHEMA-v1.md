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

## 6. Known unresolved reference (real finding from this A.3 pass)

`APEX-A.2`'s finding matrix cites `APEX-T5.5` as a replacement row for
`DET-WTH-010`, `DET-PRD-008`, and `DET-PRD-011`, but no `APEX-T5.5` row
exists anywhere in
`PROJECT-BASTION-APEX-DETERMINISM-STEP-BY-STEP-MASTER-BUILD-ORDER.md` (Tier 5
only defines `T5.1`–`T5.4`). This registry records it verbatim in
`unresolved_row_references` rather than silently dropping the reference or
inventing a row — that decision belongs to the architect/spec author, not to
this build pass.

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
