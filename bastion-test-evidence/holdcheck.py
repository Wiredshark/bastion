#!/usr/bin/env python3
"""hold-check: does every pre-existing corpus field hold its value?

    python holdcheck.py BASELINE_FULL.json NEW_FULL.json [--expect-new f1,f2,...]

Implements the mandatory post-re-baseline check for a schema window
(DECISIONS #55/#56):

    every field present before the window holds its previous value,
    and the only new fields are the ones the manifest enumerated.

Exit 0 = HOLD. Exit 1 = violation. Exit 2 = the check could not be run.

WHY EXIT 2 EXISTS: a checker that cannot distinguish "nothing moved" from
"I could not look" is the defect this whole campaign is about. Empty input,
missing seeds, or a baseline with no leaf paths are REFUSALS, not passes.
"""
import json
import sys


def leaves(obj, prefix=""):
    """Every leaf path in a nested dict. Lists are leaves (compared whole)."""
    if isinstance(obj, dict):
        for k, v in obj.items():
            yield from leaves(v, f"{prefix}.{k}" if prefix else k)
    else:
        yield prefix, obj


def index(doc):
    """{field_path: {seed: value}} across all seeds."""
    out = {}
    for seed, payload in doc.items():
        for path, value in leaves(payload):
            out.setdefault(path, {})[seed] = value
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    expect_new = set()
    for a in sys.argv[1:]:
        if a.startswith("--expect-new="):
            expect_new = {s.strip() for s in a.split("=", 1)[1].split(",") if s.strip()}

    if len(args) != 2:
        print(__doc__)
        return 2

    base_doc, new_doc = (json.load(open(p, encoding="utf-8")) for p in args)

    # ---- REFUSALS: never let "couldn't look" render as "nothing moved" ----
    if not base_doc:
        print(f"REFUSED: baseline {args[0]} has ZERO seeds - "
              "an empty run shaped like a real baseline (wave13's lesson).")
        return 2
    if not new_doc:
        print(f"REFUSED: new run {args[1]} has ZERO seeds.")
        return 2

    base, new = index(base_doc), index(new_doc)
    if not base:
        print("REFUSED: baseline has no leaf paths — nothing to hold.")
        return 2

    base_seeds, new_seeds = set(base_doc), set(new_doc)
    print(f"baseline: {len(base_seeds)} seeds, {len(base)} field paths  [{args[0]}]")
    print(f"new     : {len(new_seeds)} seeds, {len(new)} field paths  [{args[1]}]")

    if base_seeds != new_seeds:
        print(f"REFUSED: seed sets differ - "
              f"missing {sorted(base_seeds - new_seeds)[:5]}, "
              f"extra {sorted(new_seeds - base_seeds)[:5]}. "
              "A hold-check across different seed sets is meaningless.")
        return 2

    dropped = sorted(set(base) - set(new))
    added = sorted(set(new) - set(base))
    violations = []
    for path in sorted(set(base) & set(new)):
        moved = [(s, base[path][s], new[path][s])
                 for s in sorted(base[path])
                 if base[path][s] != new[path].get(s)]
        if moved:
            violations.append((path, moved))

    print()
    if dropped:
        print(f"[!!] DROPPED ({len(dropped)}) - fields present in baseline, gone now:")
        for p in dropped[:20]:
            print(f"    {p}")
    else:
        print("[OK] no dropped fields")

    unexpected = [a for a in added if a not in expect_new]
    missing_expected = sorted(expect_new - set(added))
    print(f"{'[OK]' if not unexpected else '[!!]'} added fields: {len(added)}"
          f"{f' ({len(unexpected)} NOT in the manifest)' if unexpected else ' (all enumerated)'}")
    for p in (unexpected or added)[:25]:
        print(f"    {'UNEXPECTED ' if p in unexpected else ''}{p}")
    if missing_expected:
        print(f"[!!] manifest promised {len(missing_expected)} field(s) that DID NOT APPEAR:")
        for p in missing_expected:
            print(f"    {p}")

    print()
    if violations:
        print(f"[!!] HOLD VIOLATIONS: {len(violations)} pre-existing field(s) MOVED")
        for path, moved in violations[:15]:
            s, b, n = moved[0]
            print(f"    {path}: {len(moved)} seed(s), e.g. seed {s}: {b!r} -> {n!r}")
    else:
        print(f"[OK] HOLD: all {len(set(base) & set(new))} pre-existing fields "
              f"identical across all {len(base_seeds)} seeds")

    ok = not (violations or dropped or unexpected or missing_expected)
    print()
    print("RESULT: HOLD" if ok else "RESULT: VIOLATION")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
