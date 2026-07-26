#!/usr/bin/env python3
"""Adversarial fixtures for tools/validate-apex-program-registry.py.

Positive (must be issue-free) plus negative (must fail with a specific
issue code) fixtures. Every negative fixture is derived from a minimal
in-memory registry, not from the real 53-row registry, so failures are
attributable to one intentionally broken guard at a time.
"""
import importlib.util
import sys
from pathlib import Path

VALIDATOR_PATH = Path(__file__).resolve().parents[2] / "validate-apex-program-registry.py"
spec = importlib.util.spec_from_file_location("apex_validator", VALIDATOR_PATH)
apex_validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(apex_validator)


def base_row(rid, seq, deps=None, finding_ids=None):
    return {
        "row_id": rid, "sequence_index": seq, "title": rid,
        "hard_dependencies": deps or [], "finding_ids": finding_ids or [],
        "source_surfaces": [], "source_surfaces_status": "PENDING_ROW_PACKET",
        "packet_file": None, "evidence_commands": [], "evidence_artifacts": [],
        "evidence_status": "PENDING_ROW_PACKET", "rollback_plan": None,
        "rollback_plan_status": "PENDING_ROW_PACKET",
        "status": {"specification": "SPECIFICATION_COMPLETE", "microstep_research": "MICROSTEP_RESEARCH_COMPLETE",
                   "implementation": "NOT_STARTED", "verification": "NOT_STARTED", "deployment": "NOT_DEPLOYED"},
    }


def minimal_registry(rows, findings=None, row_order=None, unresolved=None):
    return {
        "schema": "bastion.apex-program-registry/v1-json-interim",
        "canonical_guide": "test", "finding_matrix": "test",
        "audit_basis": "0" * 40, "last_live_commit_checked": "0" * 40,
        "row_order": row_order if row_order is not None else [r["row_id"] for r in rows],
        "rows": rows,
        "findings": findings or [],
        "unresolved_row_references": unresolved or [],
    }


PASS, FAIL = [], []


def check(name, reg, expect_issue_substring):
    issues = apex_validator.validate(reg)
    if expect_issue_substring is None:
        ok = len(issues) == 0
    else:
        ok = any(expect_issue_substring in i for i in issues)
    (PASS if ok else FAIL).append((name, issues))


# 1. positive: diamond dependency, valid order, one finding cleanly closed
r1 = [base_row("R1", 1), base_row("R2", 2, ["R1"]), base_row("R3", 3, ["R1"]),
      base_row("R4", 4, ["R2", "R3"], finding_ids=["F1"])]
f1 = [{"finding_id": "F1", "originating_package": "test", "live_status": "OPEN",
       "closure_rule": {"kind": "Row", "row": "R4"}, "source_anchors": [], "last_live_commit_checked": "0" * 40}]
check("positive: diamond + clean Row closure", minimal_registry(r1, f1), None)

# 2. positive: AllOf across two rows, both reverse-linked
r2 = [base_row("R1", 1, finding_ids=["F1"]), base_row("R2", 2, finding_ids=["F1"])]
f2 = [{"finding_id": "F1", "originating_package": "test", "live_status": "PARTIAL",
       "closure_rule": {"kind": "AllOf", "rows": ["R1", "R2"]}, "source_anchors": [], "last_live_commit_checked": "0" * 40}]
check("positive: AllOf with correct reverse links", minimal_registry(r2, f2), None)

# 3. negative: self-cycle
r3 = [base_row("R1", 1, ["R1"])]
check("negative: self-cycle", minimal_registry(r3), "SELF_DEPENDENCY")

# 4. negative: multi-node cycle (R1->R2->R3->R1)
r4 = [base_row("R1", 1, ["R3"]), base_row("R2", 2, ["R1"]), base_row("R3", 3, ["R2"])]
check("negative: multi-node cycle", minimal_registry(r4), "DEPENDENCY_CYCLE")

# 5. negative: unknown dependency node
r5 = [base_row("R1", 1, ["DOES_NOT_EXIST"])]
check("negative: unknown dependency", minimal_registry(r5), "UNKNOWN_DEPENDENCY")

# 6. negative: dependency after dependent (order violation)
r6 = [base_row("R1", 1, ["R2"]), base_row("R2", 2)]
check("negative: dependency declared after dependent in row_order", minimal_registry(r6), "ORDER_VIOLATION")

# 7. negative: orphan finding (closure rule points at nonexistent, unlisted row)
r7 = [base_row("R1", 1)]
f7 = [{"finding_id": "F1", "originating_package": "test", "live_status": "OPEN",
       "closure_rule": {"kind": "Row", "row": "GHOST"}, "source_anchors": [], "last_live_commit_checked": "0" * 40}]
check("negative: orphan finding row reference", minimal_registry(r7, f7), "ORPHAN_FINDING_ROW_REF")

# 8. negative: reverse-link mismatch (row doesn't list the finding back)
r8 = [base_row("R1", 1, finding_ids=[])]
f8 = [{"finding_id": "F1", "originating_package": "test", "live_status": "OPEN",
       "closure_rule": {"kind": "Row", "row": "R1"}, "source_anchors": [], "last_live_commit_checked": "0" * 40}]
check("negative: reverse-link mismatch", minimal_registry(r8, f8), "REVERSE_LINK_MISMATCH")

# 9. negative: duplicate finding id
r9 = [base_row("R1", 1, finding_ids=["F1"])]
f9 = [{"finding_id": "F1", "originating_package": "test", "live_status": "OPEN",
       "closure_rule": {"kind": "Row", "row": "R1"}, "source_anchors": [], "last_live_commit_checked": "0" * 40}] * 2
check("negative: duplicate finding id", minimal_registry(r9, f9), "DUPLICATE_FINDING")

# 10. negative: unjustified AnyOf (no rationale)
r10 = [base_row("R1", 1, finding_ids=["F1"]), base_row("R2", 2, finding_ids=["F1"])]
f10 = [{"finding_id": "F1", "originating_package": "test", "live_status": "OPEN",
        "closure_rule": {"kind": "AnyOf", "rows": ["R1", "R2"]}, "source_anchors": [], "last_live_commit_checked": "0" * 40}]
check("negative: unjustified AnyOf", minimal_registry(r10, f10), "UNJUSTIFIED_ANY_OF")

# 11. negative: SUPERSEDED without traceability
r11 = [base_row("R1", 1)]
f11 = [{"finding_id": "F1", "originating_package": "test", "live_status": "SUPERSEDED",
        "closure_rule": {"kind": "SupersededBy", "rows": [], "reason": ""}, "source_anchors": [], "last_live_commit_checked": "0" * 40}]
check("negative: superseded without traceability", minimal_registry(r11, f11), "SUPERSEDED_WITHOUT_TRACEABILITY")

# 12. negative: false closure (DEPLOYED without VERIFIED)
r12 = [base_row("R1", 1)]
r12[0]["status"]["deployment"] = "DEPLOYED"
r12[0]["status"]["verification"] = "NOT_STARTED"
check("negative: DEPLOYED without VERIFIED", minimal_registry(r12), "FALSE_CLOSURE")

# 13. negative: duplicate row id
r13 = [base_row("R1", 1), base_row("R1", 2)]
check("negative: duplicate row id", minimal_registry(r13), "DUPLICATE_ROW_ID")

# 14. negative: row_order / rows mismatch (orphan in row_order)
r14 = [base_row("R1", 1)]
check("negative: row_order references nonexistent row", minimal_registry(r14, row_order=["R1", "GHOST"]), "ROW_ORDER_ORPHAN")

# 15. real registry: fully clean after Fable's ruling folded in the T4.3
# split (resolves the ORDER_VIOLATION) and the T5.5 GUIDE_MISSING_ROW
# placeholder (resolves the UNRESOLVED_ROW_REFERENCE). Both original findings
# are proven fixed, not just re-labeled: this asserts zero issues, not "the
# same two issues with different codes".
import json
real_path = Path(__file__).resolve().parents[3] / "readme" / "APEX-DETERMINISM-PROGRAM-REGISTRY-v1.json"
with open(real_path, encoding="utf-8") as f:
    real_reg = json.load(f)
real_issues = apex_validator.validate(real_reg)
ok = real_issues == []
(PASS if ok else FAIL).append(("real registry: zero issues after the T4.3 split and T5.5 placeholder", real_issues))

# 16. negative: GUIDE_MISSING_ROW fingerprint drift is caught (non-vacuity
# for the fingerprint check itself -- mutate the real, on-disk T5.5 row's
# title and confirm the validator flags it).
import copy
drifted_reg = copy.deepcopy(real_reg)
t55 = next(r for r in drifted_reg["rows"] if r["row_id"] == "APEX-T5.5")
t55["title"] = "some content quietly appeared here"
check("negative: GUIDE_MISSING_ROW fingerprint drift is caught", drifted_reg, "GUIDE_MISSING_ROW_FINGERPRINT_DRIFT")

print(f"PASS: {len(PASS)}  FAIL: {len(FAIL)}")
for name, issues in FAIL:
    print(f"  FAILED: {name}")
    for i in issues:
        print(f"    {i}")
sys.exit(1 if FAIL else 0)
