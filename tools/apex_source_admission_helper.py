#!/usr/bin/env python3
"""Deterministic parsing/policy/JSON helper for tools/apex-source-admission.sh.

Owned by APEX-A.1 admission tooling. Pure function of its inputs: never reads
wall-clock time for anything except the diagnostic generated_at_utc field,
never reorders output, never guesses at ambiguous input (fails closed).

Invoked only by tools/apex-source-admission.sh; not a standalone entry point
for admission decisions.
"""
import hashlib
import json
import re
import sys
import datetime


def die(msg):
    sys.stderr.write("apex_source_admission_helper: " + msg + "\n")
    sys.exit(2)


def parse_policy_toml(path):
    """Minimal parser for the exact fixed shape of APEX-SOURCE-IMPACT-POLICY-v1.toml:
    top-level scalar keys plus repeated [[exact_path]] tables with string fields
    path/impact/rationale/verified_at_commit. Rejects anything else."""
    with open(path, "r", encoding="utf-8", newline="") as f:
        raw = f.read()
    digest = hashlib.sha256(raw.encode("utf-8")).hexdigest()

    default_impact = None
    rules = {}
    current = None
    order_index = 0
    for lineno, line in enumerate(raw.splitlines(), 1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped == "[[exact_path]]":
            if current is not None:
                _finish_rule(rules, current, lineno, path)
            order_index += 1
            current = {"_order": order_index, "_lineno": lineno}
            continue
        m = re.match(r'^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"((?:[^"\\]|\\.)*)"\s*$', stripped)
        if not m:
            die(f"{path}:{lineno}: unparseable policy line: {stripped!r}")
        key, val = m.group(1), m.group(2).replace('\\"', '"').replace("\\\\", "\\")
        if current is None:
            if key == "default_impact":
                default_impact = val
            elif key == "policy_schema":
                pass
            else:
                die(f"{path}:{lineno}: unknown top-level key {key!r}")
        else:
            if key not in ("path", "impact", "rationale", "verified_at_commit"):
                die(f"{path}:{lineno}: unknown exact_path field {key!r}")
            current[key] = val
    if current is not None:
        _finish_rule(rules, current, current["_lineno"], path)

    if default_impact != "unknown_impact":
        die(f"{path}: default_impact must be 'unknown_impact' (fail-closed), got {default_impact!r}")

    return rules


def _finish_rule(rules, rule, lineno, path):
    for req in ("path", "impact", "rationale", "verified_at_commit"):
        if req not in rule:
            die(f"{path}:{lineno}: exact_path rule missing required field {req!r}")
    if rule["path"] in rules:
        die(f"{path}: duplicate exact_path rule for {rule['path']!r}")
    if rule["impact"] not in ("documentation_only", "evidence_or_tooling", "production_or_build"):
        die(f"{path}:{lineno}: unknown impact value {rule['impact']!r}")
    rules[rule["path"]] = rule


IMPACT_RANK = {
    "documentation_only": 1,
    "evidence_or_tooling": 2,
    "unknown_impact": 3,
    "production_or_build": 4,
}
IMPACT_ENUM = {
    "documentation_only": "DocumentationOnly",
    "evidence_or_tooling": "EvidenceOrTooling",
    "unknown_impact": "UnknownImpact",
    "production_or_build": "ProductionOrBuild",
}


def classify_path(policy, path):
    rule = policy.get(path)
    if rule is None:
        return "unknown_impact", None
    return rule["impact"], path


def read_nul_fields(raw_bytes):
    parts = raw_bytes.split(b"\x00")
    if parts and parts[-1] == b"":
        parts = parts[:-1]
    return [p.decode("utf-8", errors="surrogateescape") for p in parts]


def parse_diff_tree_raw(raw_bytes):
    """Parse `git diff-tree --no-commit-id -r --raw -z --no-abbrev` output.
    Each record (no renames from diff-tree without -M): ':oldmode newmode oldsha newsha status' NUL 'path' NUL
    """
    fields = read_nul_fields(raw_bytes)
    entries = []
    i = 0
    line_re = re.compile(
        r"^:(?P<old_mode>\d{6}) (?P<new_mode>\d{6}) (?P<old_oid>[0-9a-f]{40,64}) "
        r"(?P<new_oid>[0-9a-f]{40,64}) (?P<status>[A-Z])\d*$"
    )
    while i < len(fields):
        meta = fields[i]
        m = line_re.match(meta)
        if not m:
            die(f"malformed diff-tree raw record: {meta!r}")
        i += 1
        if i >= len(fields):
            die("truncated diff-tree raw record: missing path")
        path = fields[i]
        i += 1
        entries.append({
            "old_mode": m.group("old_mode"),
            "new_mode": m.group("new_mode"),
            "old_oid": m.group("old_oid"),
            "new_oid": m.group("new_oid"),
            "status": m.group("status"),
            "path": path,
        })
    return entries


def parse_name_status(raw_bytes):
    """Parse `git diff --name-status -z --find-renames=50%` output.
    Non-rename: 'STATUS' NUL 'path' NUL
    Rename/copy: 'R100' NUL 'oldpath' NUL 'newpath' NUL
    """
    fields = read_nul_fields(raw_bytes)
    entries = []
    i = 0
    while i < len(fields):
        status = fields[i]
        i += 1
        if status and status[0] in ("R", "C"):
            score = int(status[1:]) if len(status) > 1 else None
            if i + 1 >= len(fields):
                die("truncated name-status rename record")
            old_path = fields[i]
            new_path = fields[i + 1]
            i += 2
            entries.append({"status": status[0], "score": score, "old_path": old_path, "new_path": new_path})
        else:
            if i >= len(fields):
                die("truncated name-status record")
            path = fields[i]
            i += 1
            entries.append({"status": status, "score": None, "old_path": None, "new_path": path})
    return entries


def build_changed_paths(diff_tree_entries, name_status_entries, policy):
    by_path = {}
    for e in diff_tree_entries:
        by_path[e["path"]] = e

    renamed_old = {}
    renamed_new = {}
    for ns in name_status_entries:
        if ns["status"] in ("R", "C"):
            renamed_old[ns["old_path"]] = ns
            renamed_new[ns["new_path"]] = ns

    results = []
    seen_paths_in_order = []
    consumed_old = set()

    for e in diff_tree_entries:
        p = e["path"]
        if p in renamed_new:
            ns = renamed_new[p]
            old_e = by_path.get(ns["old_path"])
            if old_e is None:
                die(f"rename target {p!r} has no matching delete record for old path {ns['old_path']!r}")
            old_impact, old_rule = classify_path(policy, ns["old_path"])
            new_impact, new_rule = classify_path(policy, p)
            impact = old_impact if IMPACT_RANK[old_impact] >= IMPACT_RANK[new_impact] else new_impact
            matched_rule = old_rule if impact == old_impact else new_rule
            results.append({
                "status": "R",
                "similarity_score": ns["score"],
                "old_mode": old_e["old_mode"],
                "new_mode": e["new_mode"],
                "old_object_id": old_e["old_oid"],
                "new_object_id": e["new_oid"],
                "old_path": ns["old_path"],
                "new_path": p,
                "impact": IMPACT_ENUM[impact],
                "matched_policy_rule": matched_rule,
            })
            consumed_old.add(ns["old_path"])
            seen_paths_in_order.append(p)
            continue
        if p in renamed_old:
            consumed_old.add(p)
            continue
        impact, rule = classify_path(policy, p)
        is_gitlink = e["new_mode"] == "160000" or e["old_mode"] == "160000"
        if (e["status"] == "T" or is_gitlink) and impact != "production_or_build":
            # APEX-A.1 section 5.3: a file-type or submodule-pointer change is
            # at least ProductionOrBuild unless an exact rule says otherwise.
            # The policy schema has no per-rule "covers type changes" field
            # yet, so no matched content-only rule can downgrade this — fail
            # closed rather than let a content rule silently launder a type
            # change (adversarial case 12.5).
            impact = "production_or_build"
            rule = None
        results.append({
            "status": e["status"],
            "similarity_score": None,
            "old_mode": e["old_mode"],
            "new_mode": e["new_mode"],
            "old_object_id": e["old_oid"],
            "new_object_id": e["new_oid"],
            "old_path": None,
            "new_path": p,
            "impact": IMPACT_ENUM[impact],
            "matched_policy_rule": rule,
        })
        seen_paths_in_order.append(p)

    order_index = {p: i for i, p in enumerate(seen_paths_in_order)}
    results.sort(key=lambda r: order_index[r["new_path"]])
    return results


def aggregate_verdict(changed_paths):
    if not changed_paths:
        return "NoChanges"
    ranks = {"DocumentationOnly": 1, "EvidenceOrTooling": 2, "UnknownImpact": 3, "ProductionOrBuild": 4}
    worst = max(changed_paths, key=lambda r: ranks[r["impact"]])["impact"]
    return {
        "DocumentationOnly": "DocumentationOnly",
        "EvidenceOrTooling": "EvidenceOrToolingChanged",
        "UnknownImpact": "UnknownImpact",
        "ProductionOrBuild": "ProductionOrBuildChanged",
    }[worst]


def canonical_json_dumps(obj):
    return json.dumps(obj, sort_keys=True, ensure_ascii=False, separators=(",", ":"))


def main():
    mode = sys.argv[1]
    if mode == "policy-digest":
        policy_path = sys.argv[2]
        parse_policy_toml(policy_path)
        with open(policy_path, "rb") as f:
            print(hashlib.sha256(f.read()).hexdigest())
        return

    if mode == "classify":
        # args: policy_path diff_tree_raw_path name_status_path
        policy_path, diff_tree_path, name_status_path = sys.argv[2], sys.argv[3], sys.argv[4]
        policy = parse_policy_toml(policy_path)
        with open(diff_tree_path, "rb") as f:
            dt_raw = f.read()
        with open(name_status_path, "rb") as f:
            ns_raw = f.read()
        dt_entries = parse_diff_tree_raw(dt_raw) if dt_raw else []
        ns_entries = parse_name_status(ns_raw) if ns_raw else []
        changed = build_changed_paths(dt_entries, ns_entries, policy)
        verdict = aggregate_verdict(changed)
        print(canonical_json_dumps({"changed_paths": changed, "impact_verdict": verdict}))
        return

    if mode == "checkout-status":
        # args: <porcelain-v2--z file path>
        status_path = sys.argv[2]
        with open(status_path, "rb") as f:
            raw = f.read()
        fields = read_nul_fields(raw)
        tracked = False
        untracked = False
        i = 0
        while i < len(fields):
            entry = fields[i]
            if entry.startswith("1 ") or entry.startswith("2 "):
                tracked = True
                if entry.startswith("2 "):
                    # renamed/copied entries carry an extra NUL-terminated origin path field
                    i += 1
            elif entry.startswith("u "):
                tracked = True
            elif entry.startswith("? "):
                untracked = True
            i += 1
        print(canonical_json_dumps({"tracked": tracked, "untracked": untracked}))
        return

    if mode == "emit-record":
        # args: <json fields file (canonical partial record, missing generated_at_utc)>
        partial_path = sys.argv[2]
        with open(partial_path, "r", encoding="utf-8") as f:
            record = json.load(f)
        record["generated_at_utc"] = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        out = canonical_json_dumps(record) + "\n"
        sys.stdout.write(out)
        return

    die(f"unknown mode {mode!r}")


if __name__ == "__main__":
    main()
