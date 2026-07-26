# APEX Source-Admission Schema v1

Frozen vocabulary for `tools/apex-source-admission.sh` and its evidence record.
Implements `APEX-A.1` (`PROJECT-BASTION-APEX-MICROSTEP-APEX-A.1-SOURCE-CURRENT-ADMISSION.md`).

Determinism story: this schema fixes the meaning of every enum value and terminal
code ahead of implementation so the admission verdict is a pure function of
(audit commit, target commit, impact policy digest, checkout state) — never of
wall-clock time, tool invocation order, or which host ran it.

## 1. `SourceRelationV1`

```
ExactAuditBasis    — target_commit == audit_commit
Descendant         — audit_commit is a strict ancestor of target_commit
DivergedHistory     — audit_commit is not an ancestor of target_commit
Unresolved         — relation could not be computed (e.g. shallow history)
```

## 2. `SourceImpactClassV1` (per changed path)

```
DocumentationOnly     — matched an exact policy rule with this class
EvidenceOrTooling      — matched an exact policy rule with this class
ProductionOrBuild       — matched an exact policy rule with this class, OR any
                          type-change/submodule-pointer change not covered by
                          an exact rule saying otherwise
UnknownImpact          — no exact policy rule matched
```

Aggregation rule: `ProductionOrBuild` > `UnknownImpact` > `EvidenceOrTooling` >
`DocumentationOnly` > `NoChanges`. A rename evaluates `max(impact(old_path),
impact(new_path))`.

## 3. `SourceImpactVerdictV1` (aggregate)

```
NoChanges
DocumentationOnly
EvidenceOrToolingChanged
ProductionOrBuildChanged
UnknownImpact
```

## 4. `CheckoutVerdictV1`

```
NotChecked        — --check-worktree not requested
ExactAndClean      — HEAD == target_commit, no tracked/untracked/unmerged changes
WrongHead          — HEAD != target_commit
TrackedChanges       — index or working-tree tracked modifications exist
UntrackedChanges      — untracked paths exist, not covered by an explicit local-ignore rule
Unmerged           — unmerged index entries exist
Unresolved         — checkout state could not be determined
```

## 5. `ChangedPathV1`

Fields: `status`, `similarity_score` (renames/copies only), `old_mode`,
`new_mode`, `old_object_id`, `new_object_id`, `old_path` (renames only),
`new_path`, `impact`, `matched_policy_rule`.

Sourced from `git diff-tree --no-commit-id -r --raw -z --no-abbrev` (mode/blob
identity) cross-checked against `git diff --name-status -z --find-renames=50%`
(rename/copy detection). NUL-delimited throughout; no shell word-splitting.

## 6. `SourceAdmissionRecordV1`

```
schema                        "bastion.source-admission/v1"
admission_tool_version         string, e.g. "1.0.0"
generated_at_utc                string, diagnostic only — never load-bearing
repository_expected              string
repository_observed_remote        string
target_named_ref                 string or null
audit_commit / audit_tree          full lowercase hex
target_commit / target_tree        full lowercase hex
merge_base                       full lowercase hex or null
source_relation                  SourceRelationV1
impact_policy_path                 string
impact_policy_digest              sha256 hex
changed_paths                    array of ChangedPathV1, raw Git path order
impact_verdict                   SourceImpactVerdictV1
checkout_verdict                 CheckoutVerdictV1
terminal_code                    string, see section 7
```

Serialization until `APEX-T0.2` lands: UTF-8 RFC 8259 JSON, newline-terminated,
object keys in lexicographic order, changed paths in raw Git path order, full
lowercase object IDs, no floating-point fields, SHA-256 over the exact bytes,
labeled `NON_AUTHORITATIVE_EVIDENCE_JSON_V1`.

## 7. Terminal codes

| Terminal code | Meaning | May implementation continue? |
|---|---|---:|
| `ADMIT-EXACT` | target equals audit basis; clean checkout if requested | Yes |
| `ADMIT-DOC-ONLY` | target is a descendant and all changes are exact-policy documentation-only | Yes |
| `RECHECK-EVIDENCE` | evidence/tooling changed without production changes | Only after affected tests/tools are revalidated |
| `READMIT-PRODUCTION` | production/build/content/schema path changed | No; run APEX-A.2 and targeted re-audit |
| `BLOCK-UNKNOWN-IMPACT` | at least one path has no reviewed impact rule | No |
| `BLOCK-DIVERGED-HISTORY` | audit basis is not an ancestor of target | No |
| `BLOCK-INVALID-REVISION` | audit or target does not resolve to one commit | No |
| `BLOCK-REPOSITORY-MISMATCH` | repository identity is not expected or authorized | No |
| `BLOCK-WRONG-HEAD` | checkout HEAD is not admitted target | No |
| `BLOCK-DIRTY-TRACKED` | tracked/index changes exist | No |
| `BLOCK-DIRTY-UNTRACKED` | untracked paths exist outside an explicit local-ignore rule | No |
| `BLOCK-UNMERGED` | index contains unmerged entries | No |
| `BLOCK-SHALLOW-MISSING-HISTORY` | required ancestor/history object is unavailable | No |

Terminal derivation order (first match wins):
1. Repository mismatch (no `--allow-mirror`) → `BLOCK-REPOSITORY-MISMATCH`.
2. Audit or target does not resolve to a commit object → `BLOCK-INVALID-REVISION`.
3. Ancestry could not be computed (shallow) → `BLOCK-SHALLOW-MISSING-HISTORY`.
4. `source_relation == DivergedHistory` → `BLOCK-DIVERGED-HISTORY`.
5. Any changed path has `impact == UnknownImpact` → `BLOCK-UNKNOWN-IMPACT`.
6. `impact_verdict == ProductionOrBuildChanged` → `READMIT-PRODUCTION`.
7. `impact_verdict == EvidenceOrToolingChanged` → `RECHECK-EVIDENCE`.
8. `--check-worktree` requested and `checkout_verdict` is not `ExactAndClean`/`NotChecked` → corresponding `BLOCK-WRONG-HEAD` / `BLOCK-DIRTY-TRACKED` / `BLOCK-DIRTY-UNTRACKED` / `BLOCK-UNMERGED`.
9. `source_relation == ExactAuditBasis` → `ADMIT-EXACT`.
10. `source_relation == Descendant` and `impact_verdict` in `{NoChanges, DocumentationOnly}` → `ADMIT-DOC-ONLY`.

No other combination is reachable; the script must reject any state that does
not match one of the above.
