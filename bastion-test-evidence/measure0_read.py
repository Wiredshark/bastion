#!/usr/bin/env python3
"""Measure 0: the clustering read, applying MEASURE0-CLUSTERING-PREREG.md.

The decision rule was registered BEFORE this data existed (c5649a155b). This
script implements it and does not choose thresholds.

  C = largest number of RemovedExternally releases sharing ONE tick, per seed
  S = seeds with >= 2 such releases

  CLUSTERED   C >= 2 in MORE THAN HALF of S   -> batch removal; claim-overlap live
  DISPERSED   C == 1 in >= 80% of S           -> independent removals; hypothesis loses its mechanism
  VOID        |S| < 5                          -> not a pass for either side

Refusals run FIRST. An unattested or truncated wave is not scored:
  * every seed's b5_eelog_event_count must be numeric (the log ran)
  * b5_eelog_released_events_truncated must be false everywhere (a capped list
    is right-censored, and a censored tail is exactly what a clustering read
    would misread)

Usage:  python measure0_read.py <wave.json>
Exit:   0 = a branch was reached;  2 = refusal
"""
import collections
import json
import sys

REASON = "RemovedExternally"
EVENTS = "b5_eelog_released_events"
TRUNC = "b5_eelog_released_events_truncated"
COUNT = "b5_eelog_event_count"


def main(path):
    d = json.load(open(path))
    n = len(d)
    if not n:
        print("REFUSED: empty wave.")
        return 2

    # --- refusals, before any statistic ---
    missing = [s for s in d if EVENTS not in d[s]]
    if missing:
        print(f"REFUSED: {len(missing)} seed(s) lack `{EVENTS}` -> {sorted(missing)[:5]}")
        print("        Absent is not empty. Wrong pin, or the accessor did not wire.")
        return 2
    unattested = [s for s in d if not isinstance(d[s].get(COUNT), (int, float))]
    if unattested:
        print(f"REFUSED: {len(unattested)} seed(s) unattested (`{COUNT}` not numeric).")
        print("        The event log did not run there; 'no events' would be unreadable.")
        return 2
    truncated = [s for s in d if d[s].get(TRUNC)]
    if truncated:
        print(f"REFUSED: {len(truncated)} seed(s) TRUNCATED -> {sorted(truncated)[:5]}")
        print("        A capped list is right-censored. Raise the cap and re-run;")
        print("        a clustering read over a censored tail is exactly the error")
        print("        this programme has already paid for twice.")
        return 2

    # --- the registered statistic ---
    per_seed = {}
    for s, v in d.items():
        ticks = [e["tick"] for e in (v.get(EVENTS) or []) if e.get("reason") == REASON]
        if ticks:
            per_seed[s] = collections.Counter(ticks)

    S = {s: c for s, c in per_seed.items() if sum(c.values()) >= 2}
    total_events = sum(sum(c.values()) for c in per_seed.values())
    print(f"wave: {n} seeds | seeds with any {REASON}: {len(per_seed)} "
          f"| total events: {total_events}")
    print(f"S (seeds with >= 2): {len(S)}")

    if len(S) < 5:
        print()
        print(f"** VOID ** — |S| = {len(S)} < 5. Too few multi-event seeds to")
        print("   distinguish clustered from dispersed. NOT a pass for either side.")
        if total_events == 0:
            print("   Zero events at all: a POPULATION finding — the harness scenario")
            print("   may not produce orphaned claims, exactly as item 6's witness")
            print("   found no pickups. That is not a refutation of the hypothesis.")
        return 0

    clustered = {s: max(c.values()) for s, c in S.items()}
    n_clustered = sum(1 for m in clustered.values() if m >= 2)
    frac_dispersed = sum(1 for m in clustered.values() if m == 1) / len(S)
    print(f"C >= 2 in {n_clustered}/{len(S)} seeds; C == 1 in {frac_dispersed:.0%}")
    print()
    if n_clustered > len(S) / 2:
        print("** CLUSTERED ** — batch removal. One removal orphans several claimants")
        print("   in the same tick, so how many depends on how long claims are held.")
        print("   The claim-overlap hypothesis SURVIVES and keeps its mechanism.")
    elif frac_dispersed >= 0.8:
        print("** DISPERSED ** — independent removals, one claimant at a time.")
        print("   There is no batch for claims to overlap WITH: the claim-overlap")
        print("   hypothesis LOSES its mechanism and the movers need another producer.")
        print("   (My registered prior was CLUSTERED. This refutes it.)")
    else:
        print("** INDETERMINATE ** — neither branch's threshold met.")
        print("   Report as such; do NOT pick whichever is closer.")
    print()
    print("SCOPE: this does not attribute the shape to #94, says nothing about")
    print("seed 69 (separate field), and is a HARNESS population only.")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
