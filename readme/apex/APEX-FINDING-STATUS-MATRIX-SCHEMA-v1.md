# APEX Finding-Status Matrix Schema v1

Frozen vocabulary for `readme/apex/APEX-FINDING-STATUS-MATRIX-v1.csv` /
`.md`, implementing `APEX-A.2`
(`PROJECT-BASTION-APEX-MICROSTEP-APEX-A.2-LIVE-IMPLEMENTATION-STATUS-MATRIX.md`).

Determinism story: a finding's status is a pure function of (current source
tree at an admitted commit, the finding's original contract). It is never
inferred from commit messages, run-log prose, wall-clock recency, or
proximity of unrelated campaign work — every row below was independently
re-derived by reading the cited code at the admitted commit, not by diffing
against the stale audit basis (see `APEX-A.1`; audit basis `5de5361bc` is on
a **diverged history** from this program's actual target, `bastion/apex`).

## 1. `ApexFindingStatusV1`

```
Open        — failure condition remains reachable; no separately testable
              subcontract is closed.
Partial     — root unresolved, but a real implemented primitive (not docs,
              not a proposal, not a run-log claim) closes a necessary
              sub-contract or migration seam.
Closed      — failure condition absent AND code + tests + evidence artifact
              + negative canary all exist. Static absence alone is not
              sufficient.
Superseded  — the original finding must not be implemented literally;
              adversarial review redirected its remedy into other accepted
              build rows. Supersession is not closure; the underlying risk
              is still tracked by the replacement rows.
```

Precedence: `CLOSED` requires positive implementation + verification
evidence; `PARTIAL` never auto-upgrades to `CLOSED`; `SUPERSEDED` never
means the risk disappeared; missing or contradictory evidence defaults to
`OPEN`.

## 2. Row fields (CSV columns)

| column | meaning |
|---|---|
| `finding_id` | canonical `DET-<DOMAIN>-<NNN>` ID, sorted by canonical guide order |
| `problem_group` | which of the seven apex problems this maps to |
| `status` | `OPEN\|PARTIAL\|CLOSED\|SUPERSEDED` |
| `live_path` | current code anchor(s) at `live_commit`, `path:line_start-line_end` |
| `live_observation` | what was actually read at that anchor, re-derived independently — not copied from the seed row without reverification |
| `replacement_rows` | apex build rows (`APEX-T*`) whose closure would resolve or supersede this finding |
| `scope_note` | adversarial/scope correction, if any |
| `live_commit` | full commit this row was verified against |
| `evidence_confidence` | `SYMBOL-VERIFIED` (grepped/read the exact cited symbol at the current commit) or `EXISTENCE-VERIFIED` (cited file confirmed still present and same approximate shape; carried forward from the seed's characterization without a fresh line-by-line re-read this pass) |
| `evidence_gap` | known gap in this row's own evidence (e.g. a cited file that no longer exists on this lineage) |

## 3. Serialization

Until `APEX-T0.2` (`BastionManifestEncodingV1`) lands, this row uses the same
temporary rule as `APEX-A.1`: UTF-8 RFC 4180 CSV, newline-terminated,
findings sorted by canonical guide order (not lexical ID), full commit IDs,
SHA-256 recorded alongside. **APEX-A.2 must not invent a competing
permanent canonical codec** — no `.cbor` is emitted by this row; a canonical
CBOR mirror is deferred to once `APEX-T0.2` exists, per the packet's own
"Temporary serialization rule" (mirrors `APEX-A.1` section 8.6).

## 4. Audit-basis note

The original seed matrix (`PROJECT-BASTION-APEX-FINDING-STATUS-MATRIX-v1.csv`,
24 findings) was audited against `audit_basis=5de5361bc`,
`live_commit=f7b30de6d9` (branch `bastion/block-B6HAUL`). `APEX-A.1` proved
`5de5361bc` is **not an ancestor** of this program's actual target,
`bastion/apex` (= `bastion/det-fixtures` tip). Per `A.1`'s own terminal
table, a diverged audit basis cannot support a diff-based re-audit
(`BLOCK-DIVERGED-HISTORY`), so this revision was **independently re-derived**
by reading the cited code paths directly at the current admitted commit,
not by comparing the two branches.
