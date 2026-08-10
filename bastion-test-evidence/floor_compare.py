#!/usr/bin/env python3
"""Paired determinism-floor comparator: two waves at ONE commit must be identical.

For the entity-event-log stage-1 floor: arm A (env unset) vs arm B (env set,
zero producers). Same pin, same seeds. The chassis is FREE only if every field
matches; a divergence means the store's mere presence perturbs the sim.

WHY A UNIT TEST DOES NOT COVER THIS: a unit test proves the code PATH is free.
This proves the PROCESS is -- allocation, a process-global slot, and a HashMap
whose presence can move allocation or iteration order are all invisible at call
scope and visible at whole-binary scope. Different claims.

ALLOWED TO DIFFER (and only these):
  b5_build_stamp        -- carries a build TIMESTAMP, differs per compile
  b5_soak_avg_tick_ms   -- wall-clock; measured 3.9-9.5ms across identical runs

Anything else differing is a FLOOR FAILURE.

Usage:  python floor_compare.py <armA.json> <armB.json>
Exit:   0 = floor GREEN;  2 = divergence or refusal
"""
import json
import sys

ALLOWED = {"b5_build_stamp", "b5_soak_avg_tick_ms"}


def load(p):
    with open(p) as f:
        return json.load(f)


def scalars(seed_obj):
    """Only scalars. Nested diagnostic containers are compared via their own
    JSON text so a nested change cannot hide -- see below."""
    return {k: v for k, v in seed_obj.items() if not isinstance(v, (dict, list))}


def main(pa, pb):
    a, b = load(pa), load(pb)

    # REFUSALS FIRST. A comparison over a mismatched population is not a floor.
    if set(a) != set(b):
        only_a, only_b = sorted(set(a) - set(b)), sorted(set(b) - set(a))
        print(f"REFUSED: seed sets differ. only in A: {only_a[:5]}  only in B: {only_b[:5]}")
        return 2
    if not a:
        print("REFUSED: empty wave. A floor over zero seeds is not a floor.")
        return 2

    diverged = {}
    nested_diverged = {}
    for s in sorted(a, key=int):
        sa, sb = scalars(a[s]), scalars(b[s])
        if set(sa) != set(sb):
            print(f"REFUSED seed {s}: field sets differ "
                  f"(+{sorted(set(sb)-set(sa))[:4]} -{sorted(set(sa)-set(sb))[:4]})")
            return 2
        for k in sa:
            if k in ALLOWED:
                continue
            if sa[k] != sb[k]:
                diverged.setdefault(k, []).append((s, sa[k], sb[k]))
        # nested containers compared as canonical JSON -- a diagnostic list that
        # changed shape is still a behavioural change, and skipping it would be
        # the aggregate-late error in reverse.
        for k, v in a[s].items():
            if isinstance(v, (dict, list)):
                if json.dumps(v, sort_keys=True) != json.dumps(b[s].get(k), sort_keys=True):
                    nested_diverged.setdefault(k, []).append(s)

    n = len(a)
    print(f"paired floor: {n} seeds, arm A = {pa}, arm B = {pb}")
    print(f"fields allowed to differ: {sorted(ALLOWED)}")
    print()
    if not diverged and not nested_diverged:
        print(f"FLOOR GREEN — every field identical across all {n} seeds.")
        print("The chassis is free: its presence does not perturb the sim.")
        return 0

    print("!! FLOOR RED — the chassis PERTURBS the simulation.")
    for k, rows in sorted(diverged.items()):
        s, va, vb = rows[0]
        print(f"   {k:<42} {len(rows)}/{n} seeds  e.g. seed {s}: {va} -> {vb}")
    for k, seeds in sorted(nested_diverged.items()):
        print(f"   {k:<42} {len(seeds)}/{n} seeds  (nested container)")
    return 2


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(sys.argv[1], sys.argv[2]))
