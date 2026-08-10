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

# THE ATTESTATION WITNESS (added at dda63e3766). `b5_eelog_event_count` is
# `null` when the event log is disabled and a number when enabled, so it is the
# ONE field that MUST differ between arms -- it is how each seed proves the env
# actually reached it.
#
# Merely adding it to ALLOWED would be the whole point missed: that permits it to
# differ AND permits it to be identical, which is exactly the ambiguity the field
# exists to remove. So it is asserted instead: arm A null in every seed, arm B a
# number in every seed. A seed failing that is UNATTESTED and the floor refuses,
# because "nothing changed" is only evidence when the subject demonstrably ran.
WITNESS = "b5_eelog_event_count"


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

    # ATTESTATION GATE — runs BEFORE the comparison, because an unattested pair
    # cannot be scored either way. Skipped only when the witness does not exist
    # in the build at all (the pre-dda63e3766 pins), which is reported as such
    # rather than silently passing.
    have_witness = any(WITNESS in a[s] for s in a) or any(WITNESS in b[s] for s in b)
    if not have_witness:
        print(f"NOTE: no `{WITNESS}` in this build -- pre-attestation pin.")
        print("      A green below means 'no perturbation detected, GIVEN the env")
        print("      arrived'. Transport is not witnessed per-seed. CAVEATED.")
    else:
        bad_a = [s for s in a if a[s].get(WITNESS, "MISSING") is not None]
        bad_b = [s for s in b if not isinstance(b[s].get(WITNESS), (int, float))]
        if bad_a or bad_b:
            print(f"REFUSED: attestation failed on `{WITNESS}`.")
            if bad_a:
                print(f"   arm A must be null (log DISABLED) — {len(bad_a)} seed(s) "
                      f"are not, e.g. {bad_a[0]} = {a[bad_a[0]].get(WITNESS)!r}")
            if bad_b:
                print(f"   arm B must be a number (log ENABLED) — {len(bad_b)} seed(s) "
                      f"are not, e.g. {bad_b[0]} = {b[bad_b[0]].get(WITNESS)!r}")
            print("   The env did not reach every seed. An unattested floor is not a floor.")
            return 2
        print(f"ATTESTED: `{WITNESS}` null in all {len(a)} arm-A seeds, "
              f"numeric in all {len(b)} arm-B seeds — the env reached every seed.")

    diverged = {}
    nested_diverged = {}
    for s in sorted(a, key=int):
        sa, sb = scalars(a[s]), scalars(b[s])
        if set(sa) != set(sb):
            print(f"REFUSED seed {s}: field sets differ "
                  f"(+{sorted(set(sb)-set(sa))[:4]} -{sorted(set(sa)-set(sb))[:4]})")
            return 2
        for k in sa:
            if k in ALLOWED or k == WITNESS:  # WITNESS is asserted above, not compared
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
