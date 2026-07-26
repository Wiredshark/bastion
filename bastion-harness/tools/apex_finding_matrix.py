#!/usr/bin/env python3
"""Validator for readme/apex/APEX-FINDING-STATUS-MATRIX-v1.csv.

Implements APEX-A.2's closure-rule enforcement
(readme/apex/APEX-FINDING-CLOSURE-RULES-v1.toml). Rejects false closure,
untraceable supersession, and unmatched status labels. Deterministic: same
CSV bytes always produce the same verdict, independent of invocation order
or wall-clock time.
"""
import csv
import sys

VALID_STATUSES = {"OPEN", "PARTIAL", "CLOSED", "SUPERSEDED"}


def load_rows(csv_path):
    with open(csv_path, encoding="utf-8", newline="") as f:
        return list(csv.DictReader(f))


def validate(rows):
    errors = []
    seen_ids = set()
    for i, r in enumerate(rows, 1):
        fid = r.get("finding_id", "")
        status = r.get("status", "")
        if not fid:
            errors.append(f"row {i}: missing finding_id")
            continue
        if fid in seen_ids:
            errors.append(f"row {i} ({fid}): duplicate finding_id")
        seen_ids.add(fid)

        if status not in VALID_STATUSES:
            errors.append(f"row {i} ({fid}): UNKNOWN_STATUS {status!r}")
            continue

        replacement_rows = (r.get("replacement_rows") or "").strip()
        scope_note = (r.get("scope_note") or "").strip()

        if status == "PARTIAL" and not replacement_rows:
            errors.append(f"row {i} ({fid}): PARTIAL_WITHOUT_ROOT_PATH (no replacement_rows)")

        if status == "SUPERSEDED" and (not replacement_rows or not scope_note):
            errors.append(f"row {i} ({fid}): SUPERSEDED_WITHOUT_TRACEABILITY (needs replacement_rows and scope_note)")

        if status == "CLOSED":
            # No CLOSED row exists in the current matrix (see APEX-A.2 counts);
            # this branch exists purely so a future false-CLOSED edit is caught.
            tests_present = (r.get("tests_present") or "").strip()
            evidence_gap = (r.get("evidence_gap") or "").strip()
            if not tests_present or evidence_gap:
                errors.append(f"row {i} ({fid}): FALSE_CLOSURE (CLOSED requires tests_present and no evidence_gap)")

    return errors


def main():
    if len(sys.argv) != 2:
        print("usage: apex_finding_matrix.py <matrix.csv>", file=sys.stderr)
        return 64
    rows = load_rows(sys.argv[1])
    errors = validate(rows)
    for e in errors:
        print("FAIL:", e)
    print(f"rows={len(rows)} errors={len(errors)}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
