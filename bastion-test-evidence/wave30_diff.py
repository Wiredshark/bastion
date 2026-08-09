#!/usr/bin/env python3
"""wave26 -> wave30 diff, WRITTEN BEFORE WAVE 30 EXISTED.

The point of writing it first: its classification rules cannot be shaped by
what it finds. Every moving field is assigned to a NAMED source or reported
as UNEXPLAINED -- there is no bucket for "probably the unification".

WHAT IS PRE-REGISTERED vs WHAT IS DISCOVERED (stated so the distinction
survives into the report):
  * PRE-REGISTERED: the direction of each expected effect, the hold set, and
    the rule that an unnamed mover is a finding.
  * DISCOVERED: which specific field carries P3's throughput effect. Naming
    the field after seeing it is fine; naming the DIRECTION after seeing it
    would not be, and the direction is fixed here.

See WAVE30-PREREGISTRATION.md for the full statement, including P1's
inversion (REG-1..4 were registered but never built, so their fields HOLD).
"""
import json
import sys
import collections

# ---- named sources -------------------------------------------------------
# Substring rules. Deliberately SHORT: anything not matched is UNEXPLAINED,
# which is the whole discipline. Do not grow this list to silence a mover.

P2_FARM = ("farm",)                     # FARM-PAINT (a067c17329): counts MAY RISE
P4_NEW_INSTRUMENTS = (                  # additive; expected as NEW keys, not movers
    "settle_invariant",
    "self_job_reachability_probe",
)
# P1 (REG-1..4) was INVERTED: the code never landed, so these must HOLD.
# A mover here is a defect, not a confirmation.
#
# EXACT leaf names, not substrings. The first rehearsal (wave25->wave26)
# flagged `b5_tool_steel_measured` as a HOLD-VIOLATION because "b5_tool_steel"
# is a prefix of it -- a PHANTOM FINDING manufactured by a lazy match rule.
# A classifier that over-reaches is as dishonest as one that under-reaches:
# it just fails in the direction that looks diligent.
P1_MUST_HOLD_EXACT = {
    "b5_tool_stone", "b5_tool_steel", "b5_tool_ok",
    "b5_build_ok_jobs", "b5_build_stall_jobs", "b5_build_stall_untouched",
}
P1_MUST_HOLD_PREFIX = ("b5_55_",)       # this family really is a name prefix

# Fields that change on EVERY wave by construction and carry no information
# about behaviour. Found by rehearsal: `b5_build_stamp` moved 48/48 between
# two waves that were otherwise nearly identical, and printed 48 lines at the
# top of the findings section -- burying the two real movers under the one
# field guaranteed to move. An always-mover in the findings bucket is noise
# that looks exactly like signal.
IGNORE_ALWAYS_MOVES = {
    "b5_build_stamp",
    # Wall-clock timing. Moves 48/48 on every pair of waves because it is
    # non-deterministic BY NATURE -- it measures the VM, not the colony.
    # Rehearsal: wave25->wave26 had exactly TWO real movers, and this was
    # one of them, printing 48 distinct transitions above the one that
    # mattered.
    "b5_soak_avg_tick_ms",
}


def leaves(o, p=""):
    if isinstance(o, dict):
        for k, v in o.items():
            yield from leaves(v, p + "/" + k)
    else:
        yield p, o


def flat(seed_obj):
    return dict(leaves(seed_obj))


def classify(field):
    """Classify a MOVING field. Never called on newly-added keys -- an added
    key is additive by definition and cannot violate a hold."""
    f = field.lower()
    base = f.rsplit("/", 1)[-1]
    if base in P1_MUST_HOLD_EXACT or base.startswith(P1_MUST_HOLD_PREFIX):
        return "HOLD-VIOLATION (P1 was never built -- a mover here is a DEFECT)"
    if any(s in f for s in P2_FARM):
        return "P2 FARM-PAINT (rise expected; a FALL is the falsifier)"
    if any(s in f for s in P4_NEW_INSTRUMENTS):
        return "P4 additive instrument"
    return "UNEXPLAINED"


def note_added(field):
    """Additive keys get a NOTE, not a verdict."""
    f = field.lower()
    if any(s in f for s in P4_NEW_INSTRUMENTS):
        return "P4 additive instrument (expected)"
    if any(s in f for s in P2_FARM):
        return "farm-related, new"
    return "new (unremarked in the pre-registration)"


def main(base_path, new_path):
    base = json.load(open(base_path))
    new = json.load(open(new_path))

    bseeds, nseeds = set(base), set(new)
    if bseeds != nseeds:
        print("[!] SEED SETS DIFFER -- refusing to compare.")
        print("    only in base:", sorted(bseeds - nseeds))
        print("    only in new :", sorted(nseeds - bseeds))
        return 2
    seeds = sorted(bseeds, key=int)
    # Flatten ONCE. Re-flattening inside the mover loop walked every seed
    # object per field -- correct, but it made the cost quadratic in the
    # schema for no reason.
    bflat = {s: flat(base[s]) for s in seeds}
    nflat = {s: flat(new[s]) for s in seeds}

    bkeys = set().union(*(set(bflat[s]) for s in seeds))
    nkeys = set().union(*(set(nflat[s]) for s in seeds))

    print("=" * 72)
    print("SEEDS: %d, identical sets (asserted)" % len(seeds))
    print("KEYS : base %d -> new %d" % (len(bkeys), len(nkeys)))
    print("=" * 72)

    added, removed = sorted(nkeys - bkeys), sorted(bkeys - nkeys)
    print("\n## NEW KEYS (%d) -- additive, expected" % len(added))
    for k in added:
        print("   + %-55s %s" % (k, note_added(k)))
    print("\n## REMOVED KEYS (%d) -- * each one is a FINDING unless renamed" % len(removed))
    for k in removed:
        print("   - %s" % k)

    # ---- movers over the SHARED key set ---------------------------------
    shared = sorted(bkeys & nkeys)
    movers = collections.OrderedDict()
    ignored = []
    for k in shared:
        if k.rsplit("/", 1)[-1] in IGNORE_ALWAYS_MOVES:
            ignored.append(k)
            continue
        moved = []
        for s in seeds:
            bv, nv = bflat[s].get(k), nflat[s].get(k)
            if json.dumps(bv, sort_keys=True) != json.dumps(nv, sort_keys=True):
                moved.append((s, bv, nv))
        if moved:
            movers[k] = moved

    print("\n## MOVERS OVER SHARED KEYS: %d of %d fields"
          % (len(movers), len(shared) - len(ignored)))
    if ignored:
        print("   (ignored, always-moves-by-construction: %s)" % ", ".join(ignored))
    buckets = collections.defaultdict(list)
    for k, moved in movers.items():
        buckets[classify(k)].append((k, moved))

    # UNEXPLAINED and HOLD-VIOLATION print FIRST and in full. That ordering is
    # the point: the findings must not scroll off the top under a pile of
    # expected movement.
    order = sorted(buckets, key=lambda b: (not b.startswith(("UNEXPLAINED", "HOLD")), b))
    for b in order:
        entries = buckets[b]
        print("\n### %s -- %d field(s)" % (b, len(entries)))
        full = b.startswith(("UNEXPLAINED", "HOLD"))
        for k, moved in entries:
            print("   %-55s %d/%d seeds" % (k, len(moved), len(seeds)))
            # Group by DISTINCT transition, not by seed. A 48/48 mover with
            # one transition is one fact, and printing it 48 times buries
            # everything under it. Seeds are named so the population stays
            # recoverable -- aggregate late, but present compactly.
            trans = collections.OrderedDict()
            for s, bv, nv in moved:
                key = (json.dumps(bv, sort_keys=True), json.dumps(nv, sort_keys=True))
                trans.setdefault(key, []).append(s)
            shown = list(trans.items()) if full else list(trans.items())[:4]
            # Findings get room to show WHERE they differ; expected movers
            # get a glance. Truncating a finding at 32 chars hid whether
            # two long JSON blobs differed at all during rehearsal.
            w = 110 if full else 32
            for (bj, nj), ss in shown:
                who = ",".join(ss) if len(ss) <= 8 else "%s ... (%d seeds)" % (
                    ",".join(ss[:8]), len(ss))
                if full and (len(bj) > w or len(nj) > w):
                    i = next((j for j in range(min(len(bj), len(nj)))
                              if bj[j] != nj[j]), min(len(bj), len(nj)))
                    lo = max(0, i - 40)
                    print("        [%s] first differs at char %d:" % (who, i))
                    print("          - ...%s" % bj[lo:i + 70])
                    print("          + ...%s" % nj[lo:i + 70])
                else:
                    print("        %s -> %s   [%s]" % (bj[:w], nj[:w], who))
            if len(trans) > len(shown):
                print("        ... %d more distinct transitions" % (len(trans) - len(shown)))

    # ---- P3 exposure population -----------------------------------------
    # Read the throughput effect CONDITIONED on seeds that demonstrably ran
    # self-jobs. WIP-STATE's lesson: the 48-seed aggregate diluted the last
    # result ~4:1 and hid it.
    print("\n" + "=" * 72)
    print("## P3 EXPOSURE POPULATION (self_job_reachability_probe non-empty)")
    exposed = []
    for s in seeds:
        hits = [v for k, v in nflat[s].items()
                if "self_job_reachability_probe" in k]
        if any(v not in (None, [], {}, 0) for v in hits):
            exposed.append(s)
    if not exposed:
        print("   * EMPTY. Either the field is absent, or no self-job ever timed")
        print("     out. THOSE ARE DIFFERENT FACTS and this field cannot tell")
        print("     them apart -- report as a limit, never as 'no exposure'.")
    else:
        print("   %d/%d seeds exposed: %s" % (len(exposed), len(seeds), exposed))
        print("   * Read P3's throughput change on THIS set, not all 48.")
        print("   * VERIFY entries fall OUTSIDE the mine region first -- the")
        print("     filter's first version mislabeled completed mine cells as")
        print("     self-jobs (seed 90, 6 entries). Named by its own author.")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(__doc__)
        print("usage: wave30_diff.py <wave26_FULL.json> <wave30_FULL.json>")
        sys.exit(1)
    sys.exit(main(sys.argv[1], sys.argv[2]))
