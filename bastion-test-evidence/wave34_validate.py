#!/usr/bin/env python3
"""Validate wave34's NEW fields (item-6 witness + item-2 stalled_final).

Written BEFORE the fan's data lands and rehearsed on planted cases, per the
standing rule: a 48-seed fan is not an instrument's first test.

Every check is DISCRIMINATING -- it can fail on a field that exists and is
wrong. Absent REFUSES rather than scoring zero.

The prediction read is reported, never failed: `later_colonist > 0` CONFIRMS
Fable's registered prediction (a colonist-timing race) and is a finding, not a
validation error.

Usage:  python wave34_validate.py <seed.json> [more.json ...]
Exit:   0 = every file passed;  2 = at least one REFUSAL or FAILURE
"""
import json
import sys

# item-6 witness (7f20a18438). Flat on the two reasons whose branch predicate
# already fixes colonist-ness (inventory_manip.rs:315-316, :342-343); split
# kept only on loot-owned, whose predicate never reads bastion_colonists.
NEW_FIELDS = [
    ("b5_f3_stalled_final", float),
    ("b5_pickup_refused_pile_protected", int),
    ("b5_pickup_refused_ambient_disabled", int),
    ("b5_pickup_refused_ambient_uids_distinct", int),
    ("b5_pickup_refused_ambient_later_colonist", int),
    ("b5_pickup_refused_loot_owned_colonist", int),
    ("b5_pickup_refused_loot_owned_ambient", int),
]
# carried from wave33 -- the stall pair is only readable together
CARRIED = [("b5_f3_stalled_peak", float), ("b5_f3_ticks_branch_c", int)]


def flat(o, out=None):
    if out is None:
        out = {}
    if isinstance(o, dict):
        for k, v in o.items():
            if isinstance(v, (dict, list)):
                flat(v, out)
            else:
                out[k] = v
    elif isinstance(o, list):
        for v in o:
            flat(v, out)
    return out


def load(path):
    """Whole file, then FIRST LINE. Never scan for the first thing that parses
    -- that silently accepts a truncated object."""
    raw = open(path).read()
    for text in (raw, raw.split("\n", 1)[0]):
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            continue
    return None


def check(path, findings):
    try:
        doc = load(path)
    except OSError as e:
        print(f"REFUSED {path}: unreadable ({e})")
        return False
    if doc is None:
        print(f"REFUSED {path}: neither whole file nor first line is JSON")
        return False
    d = flat(doc)

    missing = [k for k, _ in NEW_FIELDS + CARRIED if k not in d]
    if missing:
        print(f"REFUSED {path}: {len(missing)} field(s) ABSENT -> {missing}")
        print("        Absent is not zero. The accessor did not wire, or the")
        print("        binary predates 7f20a18438. No value below is licensed.")
        return False

    v = {k: d[k] for k, _ in NEW_FIELDS + CARRIED}
    ok = True

    def fail(msg):
        nonlocal ok
        print(f"  FAIL {msg}")
        ok = False

    peak, final = v["b5_f3_stalled_peak"], v["b5_f3_stalled_final"]
    pile = v["b5_pickup_refused_pile_protected"]
    amb = v["b5_pickup_refused_ambient_disabled"]
    uids = v["b5_pickup_refused_ambient_uids_distinct"]
    later = v["b5_pickup_refused_ambient_later_colonist"]
    lo_col = v["b5_pickup_refused_loot_owned_colonist"]
    lo_amb = v["b5_pickup_refused_loot_owned_ambient"]

    for k, _ in NEW_FIELDS + CARRIED:
        if not isinstance(v[k], (int, float)) or isinstance(v[k], bool):
            fail(f"{k} is {type(v[k]).__name__}, expected numeric")
    for k in ("b5_f3_stalled_final", "b5_pickup_refused_pile_protected",
              "b5_pickup_refused_ambient_disabled",
              "b5_pickup_refused_ambient_uids_distinct",
              "b5_pickup_refused_ambient_later_colonist",
              "b5_pickup_refused_loot_owned_colonist",
              "b5_pickup_refused_loot_owned_ambient"):
        if v[k] < 0:
            fail(f"{k} is negative ({v[k]})")

    # 1. FINAL vs PEAK -- peak is a high-water mark, so final can never exceed
    #    it. This is the check that makes the pair readable at all.
    if final > peak:
        fail(f"stalled_final {final} > stalled_peak {peak} -- a high-water mark "
             "cannot be below its own final value; one of them is not what its "
             "name says")

    # 2. DISTINCT UIDS <= TOTAL REFUSALS. More distinct pickers than refusals
    #    is impossible; each refusal contributes at most one uid.
    if uids > amb:
        fail(f"ambient_uids_distinct {uids} > ambient_disabled {amb} -- more "
             "distinct pickers than refusal events")

    # 3. LATER-COLONIST <= DISTINCT UIDS. The recheck is over the recorded uid
    #    set, so it cannot exceed that set's size.
    if later > uids:
        fail(f"ambient_later_colonist {later} > uids_distinct {uids} -- the "
             "recheck returned more colonists than uids it was given")

    # 4. Refusals imply at least one uid recorded, and vice versa. A count
    #    without its uid set (or the reverse) means the two writers diverged --
    #    exactly the divergence the packet required be impossible.
    if amb > 0 and uids == 0:
        fail(f"ambient_disabled {amb} but uids_distinct 0 -- the counter fired "
             "and the uid recorder did not; the two writers have diverged")
    if uids > 0 and amb == 0:
        fail(f"uids_distinct {uids} but ambient_disabled 0 -- uids recorded "
             "with no refusal counted; same divergence, other direction")

    # 5. THE REGISTERED PREDICTION -- reported, never failed.
    if later > 0:
        findings.append((path, later, uids, amb))
        print(f"  ** PREDICTION CONFIRMED (candidate): later_colonist={later} "
              f"of {uids} distinct refused uids **")
        print("     An entity refused as AMBIENT was a COLONIST at run end.")
        print("     bastion_spawn_colony CREATES npcs already carrying the")
        print("     marker and never adopts existing entities (verified at")
        print("     7f20a18438), so recruitment cannot explain this.")

    print(f"{'PASS' if ok else 'FAIL'} {path}")
    print(f"     stall peak={peak} final={final} | refused pile={pile} "
          f"ambient={amb} (uids {uids}, later-colonist {later}) "
          f"loot_owned col={lo_col} amb={lo_amb}")
    return ok


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    findings = []
    results = [check(p, findings) for p in sys.argv[1:]]
    print()
    print(f"{sum(results)}/{len(results)} file(s) passed")
    if findings:
        print(f"\n{'='*66}")
        print(f"PREDICTION CONFIRMED IN {len(findings)} SEED(S) -- "
              "the colonist-timing race is REAL")
        for p, later, uids, amb in findings:
            print(f"  {p}: {later}/{uids} refused uids were colonists at run end")
        print("This is a FINDING, not a validation failure.")
    else:
        print("\nPrediction NOT confirmed in any seed (later_colonist == 0 "
              "everywhere).\nIt dies clean ONLY IF ambient refusals actually "
              "fired -- check that\nb5_pickup_refused_ambient_disabled > 0 "
              "somewhere, or the test never ran.")
    sys.exit(0 if all(results) else 2)
