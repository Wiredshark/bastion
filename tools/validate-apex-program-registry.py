#!/usr/bin/env python3
"""Validator for readme/APEX-DETERMINISM-PROGRAM-REGISTRY-v1.json.

Implements APEX-A.3. Standard library only, no network or game build
required. Deterministic: same registry bytes always produce the same issue
list, independent of invocation order or wall-clock time (generated_at-style
fields, if any, are never consulted by the checks below).
"""
import json
import sys


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def check_row_ids(reg, issues):
    row_order = reg["row_order"]
    row_ids = [r["row_id"] for r in reg["rows"]]
    if len(set(row_ids)) != len(row_ids):
        seen = set()
        for rid in row_ids:
            if rid in seen:
                issues.append(f"DUPLICATE_ROW_ID: {rid}")
            seen.add(rid)
    if set(row_order) != set(row_ids):
        missing_from_rows = set(row_order) - set(row_ids)
        missing_from_order = set(row_ids) - set(row_order)
        for m in sorted(missing_from_rows):
            issues.append(f"ROW_ORDER_ORPHAN: {m} in row_order but has no row record")
        for m in sorted(missing_from_order):
            issues.append(f"UNORDERED_ROW: {m} has a row record but is missing from row_order")
    if len(row_order) != len(set(row_order)):
        issues.append("DUPLICATE_ROW_ORDER_ENTRY")
    return {r["row_id"]: r for r in reg["rows"]}


def check_dependency_graph(reg, rows_by_id, issues):
    row_order = reg["row_order"]
    index_of = {rid: i for i, rid in enumerate(row_order)}

    # unknown edges
    for r in reg["rows"]:
        for dep in r["hard_dependencies"]:
            if dep not in rows_by_id:
                issues.append(f"UNKNOWN_DEPENDENCY: {r['row_id']} -> {dep}")
            if dep == r["row_id"]:
                issues.append(f"SELF_DEPENDENCY: {r['row_id']}")

    # cycle detection (DFS, only over known nodes)
    WHITE, GRAY, BLACK = 0, 1, 2
    color = {rid: WHITE for rid in rows_by_id}
    cycle_found = []

    def visit(rid, stack):
        if color[rid] == BLACK:
            return
        if color[rid] == GRAY:
            cyc = stack[stack.index(rid):] + [rid]
            cycle_found.append(cyc)
            return
        color[rid] = GRAY
        stack.append(rid)
        for dep in rows_by_id[rid]["hard_dependencies"]:
            if dep in rows_by_id:
                visit(dep, stack)
        stack.pop()
        color[rid] = BLACK

    for rid in rows_by_id:
        if color[rid] == WHITE:
            visit(rid, [])

    for cyc in cycle_found:
        issues.append(f"DEPENDENCY_CYCLE: {' -> '.join(cyc)}")

    if not cycle_found:
        # row_order must be a valid topological order of hard_dependencies
        for r in reg["rows"]:
            rid = r["row_id"]
            for dep in r["hard_dependencies"]:
                if dep in index_of and index_of[dep] >= index_of[rid]:
                    issues.append(
                        f"ORDER_VIOLATION: {rid} (index {index_of[rid]}) depends on "
                        f"{dep} (index {index_of[dep]}) which does not precede it"
                    )


def check_findings(reg, rows_by_id, issues):
    seen_finding_ids = set()
    unresolved = set(reg.get("unresolved_row_references", []))

    for ft in reg["findings"]:
        fid = ft["finding_id"]
        if fid in seen_finding_ids:
            issues.append(f"DUPLICATE_FINDING: {fid}")
        seen_finding_ids.add(fid)

        rule = ft["closure_rule"]
        kind = rule["kind"]
        if kind not in ("Row", "AllOf", "AnyOf", "SupersededBy"):
            issues.append(f"UNKNOWN_CLOSURE_RULE_KIND: {fid} -> {kind}")
            continue

        if kind == "Row":
            rows = [rule["row"]] if rule.get("row") else []
            if not rows:
                issues.append(f"EMPTY_CLOSURE_ROW: {fid}")
        elif kind == "AnyOf":
            rows = rule["rows"]
            if not rule.get("rationale"):
                issues.append(f"UNJUSTIFIED_ANY_OF: {fid}")
        elif kind == "SupersededBy":
            rows = rule["rows"]
            if not rows or not rule.get("reason"):
                issues.append(f"SUPERSEDED_WITHOUT_TRACEABILITY: {fid}")
        else:  # AllOf
            rows = rule["rows"]
            if not rows:
                issues.append(f"EMPTY_ALL_OF: {fid}")

        for rid in rows:
            if rid in unresolved:
                issues.append(f"UNRESOLVED_ROW_REFERENCE: {fid} -> {rid} (row does not exist in canonical guide)")
                continue
            if rid not in rows_by_id:
                issues.append(f"ORPHAN_FINDING_ROW_REF: {fid} -> {rid} (not in unresolved_row_references either)")
                continue
            if fid not in rows_by_id[rid]["finding_ids"]:
                issues.append(f"REVERSE_LINK_MISMATCH: {rid}.finding_ids missing {fid} (declared by {fid}'s closure_rule)")

    return seen_finding_ids


def check_terminal_invariants(reg, issues):
    for r in reg["rows"]:
        s = r["status"]
        if s["specification"] == "NEEDS_DESIGN" and s["implementation"] == "IMPLEMENTED":
            issues.append(f"FALSE_CLOSURE: {r['row_id']} IMPLEMENTED while specification NEEDS_DESIGN")
        if s["deployment"] == "DEPLOYED":
            if s["verification"] != "VERIFIED":
                issues.append(f"FALSE_CLOSURE: {r['row_id']} DEPLOYED without VERIFIED")
            if r["rollback_plan_status"] not in ("VERIFIED", "NOT_APPLICABLE_WITH_RATIONALE"):
                issues.append(f"FALSE_CLOSURE: {r['row_id']} DEPLOYED without a rollback plan")


def validate(reg):
    issues = []
    rows_by_id = check_row_ids(reg, issues)
    check_dependency_graph(reg, rows_by_id, issues)
    check_findings(reg, rows_by_id, issues)
    check_terminal_invariants(reg, issues)
    return issues


def main():
    if len(sys.argv) != 2:
        print("usage: validate-apex-program-registry.py <registry.json>", file=sys.stderr)
        return 64
    reg = load(sys.argv[1])
    issues = validate(reg)
    for i in issues:
        print("FAIL:", i)
    print(f"rows={len(reg['rows'])} findings={len(reg['findings'])} "
          f"unresolved_row_references={len(reg.get('unresolved_row_references', []))} issues={len(issues)}")
    return 1 if issues else 0


if __name__ == "__main__":
    sys.exit(main())
