#!/usr/bin/env python3
"""Narrow atomic updater for readme/APEX-DETERMINISM-PROGRAM-REGISTRY-v1.json.

Implements APEX-A.3.09: edits exactly one row's status field or one
finding's live_status, revalidates the complete resulting registry, and
only then publishes via write-temp-then-atomic-rename. Refuses unknown
fields, refuses to publish a registry that fails validation, and refuses a
stale write when --expected-digest does not match the current on-disk
registry (optimistic concurrency).
"""
import argparse
import hashlib
import importlib.util
import json
import os
import sys
from pathlib import Path

VALIDATOR_PATH = Path(__file__).resolve().parent / "validate-apex-program-registry.py"
spec = importlib.util.spec_from_file_location("apex_validator", VALIDATOR_PATH)
apex_validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(apex_validator)

STATUS_FIELDS = {"specification", "microstep_research", "implementation", "verification", "deployment"}
COMPLETENESS_FIELDS = {"source_surfaces_status", "evidence_status", "rollback_plan_status"}


def canonical_dumps(obj):
    return json.dumps(obj, sort_keys=True, ensure_ascii=False, indent=2) + "\n"


def digest_of(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def atomic_write(path, text):
    tmp = str(path) + f".tmp.{os.getpid()}"
    with open(tmp, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)  # atomic on POSIX and NTFS


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("registry", help="path to APEX-DETERMINISM-PROGRAM-REGISTRY-v1.json")
    ap.add_argument("--expected-digest", required=True, help="sha256 the caller believes is currently on disk")
    ap.add_argument("--set-row-status", nargs=3, metavar=("ROW_ID", "FIELD", "VALUE"))
    ap.add_argument("--set-finding-status", nargs=2, metavar=("FINDING_ID", "VALUE"))
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    reg_path = Path(args.registry)
    current_digest = digest_of(reg_path)
    if current_digest != args.expected_digest:
        print(f"REJECT: stale write. expected_digest={args.expected_digest} actual_digest={current_digest}", file=sys.stderr)
        return 1

    with open(reg_path, encoding="utf-8") as f:
        reg = json.load(f)

    if args.set_row_status:
        row_id, field, value = args.set_row_status
        if field not in STATUS_FIELDS and field not in COMPLETENESS_FIELDS:
            print(f"REJECT: unknown row field {field!r}", file=sys.stderr)
            return 1
        row = next((r for r in reg["rows"] if r["row_id"] == row_id), None)
        if row is None:
            print(f"REJECT: unknown row_id {row_id!r}", file=sys.stderr)
            return 1
        if field in STATUS_FIELDS:
            row["status"][field] = value
        else:
            row[field] = value

    if args.set_finding_status:
        finding_id, value = args.set_finding_status
        finding = next((f for f in reg["findings"] if f["finding_id"] == finding_id), None)
        if finding is None:
            print(f"REJECT: unknown finding_id {finding_id!r}", file=sys.stderr)
            return 1
        finding["live_status"] = value

    if not args.set_row_status and not args.set_finding_status:
        print("REJECT: no mutation requested (--set-row-status or --set-finding-status)", file=sys.stderr)
        return 1

    issues = apex_validator.validate(reg)
    new_issues = set(f"{i}" for i in issues)
    old_reg = json.loads(reg_path.read_text(encoding="utf-8"))
    old_issues = set(apex_validator.validate(old_reg))
    introduced = new_issues - old_issues
    if introduced:
        print("REJECT: mutation introduces new validation issues, refusing to publish:", file=sys.stderr)
        for i in sorted(introduced):
            print(f"  NEW: {i}", file=sys.stderr)
        return 1

    out_text = canonical_dumps(reg)
    if args.dry_run:
        print("DRY-RUN OK: mutation is valid, not published.")
        print("new_digest(would-be):", hashlib.sha256(out_text.encode("utf-8")).hexdigest())
        return 0

    atomic_write(reg_path, out_text)
    new_digest = digest_of(reg_path)
    print(f"PUBLISHED: old_digest={current_digest} new_digest={new_digest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
