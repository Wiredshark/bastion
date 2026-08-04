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
import re
import sys


def leaves(obj, prefix="", descend=()):
    """Every leaf path in a nested dict.

    Lists are LEAVES by default — compared whole, because a reordered list is a
    change and index-keying would hide that behind a wall of noise.

    A field named in `descend` has its list elements indexed instead
    (`field[3].subkey`). Use it when a list's ELEMENTS gained a sub-key: the
    whole-list comparison would report one uninformative MOVE per seed, and the
    only alternatives are ignoring the field — which is how you end up not
    checking the very place a change lives.
    """
    if isinstance(obj, dict):
        for k, v in obj.items():
            p = f"{prefix}.{k}" if prefix else k
            yield from leaves(v, p, descend)
    elif isinstance(obj, list) and prefix in descend:
        for i, v in enumerate(obj):
            yield from leaves(v, f"{prefix}[{i}]", descend)
    else:
        yield prefix, obj


def brief(v, n=110):
    """Values go in a REPORT a human reads. An untruncated 8KB list dump is
    technically complete and practically unreadable - and an unreadable report
    is one people stop reading."""
    t = repr(v)
    return t if len(t) <= n else t[:n] + f"... <{len(t)} chars, truncated>"


def index(doc, descend=()):
    """{field_path: {seed: value}} across all seeds."""
    out = {}
    for seed, payload in doc.items():
        for path, value in leaves(payload, "", descend):
            out.setdefault(path, {})[seed] = value
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    expect_new, ignore, descend = set(), set(), set()
    for a in sys.argv[1:]:
        if a.startswith("--expect-new="):
            expect_new = {s.strip() for s in a.split("=", 1)[1].split(",") if s.strip()}
        if a.startswith("--ignore="):
            ignore = {s.strip() for s in a.split("=", 1)[1].split(",") if s.strip()}
        if a.startswith("--descend="):
            descend = {s.strip() for s in a.split("=", 1)[1].split(",") if s.strip()}

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

    base, new = index(base_doc, descend), index(new_doc, descend)
    if not base:
        print("REFUSED: baseline has no leaf paths — nothing to hold.")
        return 2

    if descend:
        print(f"descending into list elements of: {', '.join(sorted(descend))}")
        # A --descend name that matches no list is a SILENT no-op: the caller
        # believes they bought per-element granularity and did not. Same class
        # as the --ignore that names an absent field.
        seen = set()
        for doc in (base_doc, new_doc):
            for payload in doc.values():
                for k, v in payload.items():
                    if isinstance(v, list):
                        seen.add(k)
        for d in sorted(descend - seen):
            print(f"    !! '{d}' is not a list field in either run - "
                  "the --descend had NO EFFECT")
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
    shared = sorted(set(base) & set(new))

    # ---- An EXCLUSION must never render like an ABSENCE: say what was skipped.
    skipped = sorted(p for p in shared if p in ignore)
    if ignore:
        print()
        print(f"!! IGNORED BY REQUEST ({len(skipped)} of {len(ignore)} named) - "
              "NOT checked for holding:")
        for p in skipped:
            n = sum(1 for s in base[p] if base[p][s] != new[p].get(s))
            print(f"    {p}  (would have moved on {n}/{len(base[p])} seeds)")
        for p in sorted(ignore - set(skipped)):
            print(f"    {p}  -- NOT PRESENT in both runs; the --ignore had no effect")

    violations = []
    for path in shared:
        if path in ignore:
            continue
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

    # expect-new entries may use `*`, so a descended list's per-index paths
    # ("f[*].newkey") are declarable in one line instead of 54.
    # NOT fnmatch: `[` and `]` are fnmatch metacharacters, so "f[*].k" silently
    # matches NOTHING - a pattern that fails closed is the zero-match trap.
    # Escape everything except `*`, which becomes `.*`.
    def as_re(pat):
        return re.compile(".*".join(re.escape(p) for p in pat.split("*")) + r"\Z")
    pats = [(e, as_re(e)) for e in sorted(expect_new)]

    def declared(path):
        return any(rx.match(path) for _, rx in pats)
    unexpected = [a for a in added if not declared(a)]
    missing_expected = sorted(
        e for e, rx in pats if not any(rx.match(a) for a in added))
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
        # Breadth is diagnostic: every-seed drift is systematic (a stamp, a
        # timing, a global change); few-seed drift is a localized behavior
        # change. Same verdict, different first suspect - so split them.
        total = len(base_seeds)
        systemic = [(p, m) for p, m in violations if len(m) == total]
        partial = [(p, m) for p, m in violations if len(m) != total]
        print(f"[!!] HOLD VIOLATIONS: {len(violations)} pre-existing field(s) MOVED")
        if systemic:
            print(f"  -- SYSTEMIC ({len(systemic)}): moved on ALL {total} seeds. "
                  "First suspects: a build stamp, a wall-clock timing, or a "
                  "genuinely global change. NOT necessarily a behavior bug.")
            for path, moved in systemic[:10]:
                s, b, n = moved[0]
                print(f"       {path}: e.g. seed {s}: {brief(b)} -> {brief(n)}")
        if partial:
            print(f"  -- LOCALIZED ({len(partial)}): moved on SOME seeds. "
                  "This is the shape of a real behavior change.")
            for path, moved in partial[:10]:
                s, b, n = moved[0]
                print(f"       {path}: {len(moved)}/{total} seeds, "
                      f"e.g. seed {s}: {brief(b)} -> {brief(n)}")
    else:
        checked = len([p for p in shared if p not in ignore])
        print(f"[OK] HOLD: all {checked} checked pre-existing fields identical "
              f"across all {len(base_seeds)} seeds"
              f"{f' ({len(skipped)} ignored by request)' if skipped else ''}")

    ok = not (violations or dropped or unexpected or missing_expected)
    print()
    print("RESULT: HOLD" if ok else "RESULT: VIOLATION")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
