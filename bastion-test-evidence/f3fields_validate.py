#!/usr/bin/env python3
"""Validate the F3 corpus fields on a harness JSON BEFORE any paid fan.

Written before the fields have ever emitted on a harness run. Rehearse before
paid data: a 48-seed fan is not an instrument's first test.

Presence is not validation. Every check below is DISCRIMINATING -- it can fail
on a field that exists and is wrong. A missing field REFUSES rather than
scoring zero, because absent and zero are the same defect this row exists to
stop conflating.

Usage:  python f3fields_validate.py <seed.json> [more.json ...]
Exit:   0 = every file passed every check
        2 = at least one REFUSAL or FAILURE
"""
import json
import sys

FIELDS = [
    ("b5_f3_ticks_branch_a", int),
    ("b5_f3_ticks_branch_b", int),
    ("b5_f3_ticks_branch_c", int),
    ("b5_f3_transitions", int),
    ("b5_f3_idle_peak", float),
    ("b5_f3_prunes_fired", int),
    # ITEM 2's stall clock. Added after the six because the fan cannot set
    # ACCESS_STALL_SECS without it -- the emit that carried `stalled` went to
    # stderr, which the fan discards.
    ("b5_f3_stalled_peak", float),
]

# ACCESS_STALE_SECS at 9adcd56d36. The pruner fires at this value, so a peak
# below it with zero prunes is coherent and a peak above it is not.
ACCESS_STALE_SECS = 20.0

# ITEM 2's stall threshold, landed PROVISIONAL at 07ba0cc17b and explicitly
# not to be trusted -- the whole point of the fan is to replace it from the
# 48-seed distribution. Kept here only so the peak/prune consistency check
# has a number; update it when the fan sets the real one.
ACCESS_STALL_SECS = 120.0


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


def check(path):
    # The harness prints the JSON object on its own line, then a human
    # summary line ("B5 SCENARIO: PASS"). Whole-file json.load therefore
    # fails on real output. Try the whole file first (single-object files
    # from other producers), then the FIRST LINE. Never scan for the first
    # thing that parses -- that would silently accept a truncated object.
    try:
        raw = open(path).read()
    except OSError as e:
        print(f"REFUSED {path}: unreadable ({e})")
        return False
    doc, how = None, None
    for label, text in (("whole file", raw), ("first line", raw.split("\n", 1)[0])):
        try:
            doc = json.loads(text)
            how = label
            break
        except json.JSONDecodeError:
            continue
    if doc is None:
        print(f"REFUSED {path}: neither the whole file nor its first line is JSON")
        return False
    trailing = [l for l in raw.split("\n")[1:] if l.strip()]
    if how == "first line" and trailing:
        print(f"  (parsed the FIRST LINE as JSON; {len(trailing)} trailing "
              f"non-JSON line(s), e.g. {trailing[0][:60]!r})")
    d = flat(doc)

    missing = [k for k, _ in FIELDS if k not in d]
    if missing:
        print(f"REFUSED {path}: {len(missing)} of {len(FIELDS)} fields ABSENT -> {missing}")
        print("        Absent is not zero. The accessor did not wire, or the")
        print("        binary predates 002c642cb8. No value below is licensed.")
        return False

    v = {k: d[k] for k, _ in FIELDS}
    ok = True

    def fail(msg):
        nonlocal ok
        print(f"  FAIL {msg}")
        ok = False

    # types
    for k, t in FIELDS:
        if not isinstance(v[k], (int, float)):
            fail(f"{k} is {type(v[k]).__name__}, expected numeric")

    a, b, c = v["b5_f3_ticks_branch_a"], v["b5_f3_ticks_branch_b"], v["b5_f3_ticks_branch_c"]
    tr, peak, pr = v["b5_f3_transitions"], v["b5_f3_idle_peak"], v["b5_f3_prunes_fired"]
    total = a + b + c

    # 1. the pass ran at all -- a run with zero dwell measured nothing
    if total == 0:
        fail("dwell a+b+c == 0: the F3 pass never executed, or counters never increment")

    # 2. transitions must be consistent with dwell: you cannot transition
    #    more often than you have passes, and any run with dwell in >1 branch
    #    must have transitioned at least once.
    if tr > total:
        fail(f"transitions {tr} > total passes {total} -- impossible")
    branches_used = sum(1 for x in (a, b, c) if x > 0)
    if branches_used > 1 and tr < branches_used - 1:
        fail(f"{branches_used} branches used but only {tr} transitions -- unreachable")

    # 3. peak vs prunes: the pruner fires AT the threshold, so a peak that
    #    reached it with zero prunes is contradictory, and vice versa.
    if peak >= ACCESS_STALE_SECS and pr == 0:
        fail(f"idle_peak {peak} >= {ACCESS_STALE_SECS} but prunes_fired 0 -- "
             "the threshold was reached and nothing pruned")
    # NOTE: the converse ("prunes_fired > 0 implies idle_peak reached
    # ACCESS_STALE_SECS") was asserted here and is WRONG. It was written
    # when branch B's sweep was the only writer. ITEM 2 added a SECOND
    # producer to the same counter (bastion_jobs.rs:15199, the branch-C
    # claimed-no-progress sweep), so `prunes_fired` is now a UNION and no
    # single threshold explains it. wave33 failed 8/48 seeds on the stale
    # clause alone -- every one of them a correct branch-C stall prune.
    # The attribution check that replaces it lives in clause 6, where both
    # thresholds are in scope.

    # 4. peak requires branch B: the counter only accrues there.
    if peak > 0 and b == 0:
        fail(f"idle_peak {peak} > 0 but ticks_branch_b == 0 -- "
             "the counter accrued in a branch that never ran")

    # 5. sanity floors
    if peak < 0:
        fail(f"idle_peak {peak} negative")

    # 6. ITEM 2's stall clock -- same discriminating shape as idle_peak.
    #    It accrues ONLY in branch C (a claimed access job making no progress),
    #    so a nonzero peak with no C dwell is impossible.
    speak = v["b5_f3_stalled_peak"]
    if speak < 0:
        fail(f"stalled_peak {speak} negative")
    if speak > 0 and c == 0:
        fail(f"stalled_peak {speak} > 0 but ticks_branch_c == 0 -- "
             "the stall clock accrued in a branch that never ran")
    if speak >= ACCESS_STALL_SECS and pr == 0:
        fail(f"stalled_peak {speak} >= {ACCESS_STALL_SECS} but prunes_fired 0 -- "
             "the stall threshold was reached and nothing pruned")

    # PRUNE ATTRIBUTION (replaces the single-producer clause above).
    # `prunes_fired` is the union of TWO sweeps, so the honest invariant is
    # that at least one of their thresholds was reached. This still fails on
    # a prune nobody can explain -- which is the case worth catching -- while
    # accepting the legitimate branch-C stall prune the old clause rejected.
    if pr > 0 and peak < ACCESS_STALE_SECS and speak < ACCESS_STALL_SECS:
        fail(f"prunes_fired {pr} but NEITHER threshold was reached "
             f"(idle_peak {peak} < {ACCESS_STALE_SECS}, "
             f"stalled_peak {speak} < {ACCESS_STALL_SECS}) -- "
             "an unattributable prune: a third producer, or a reset that "
             "cleared a peak before it was captured")

    # CENSORING WARNING -- not a failure, a limit on what may be concluded.
    # Both peaks reset to 0 the instant their threshold fires, so a peak that
    # EQUALS its threshold is right-censored: the true dwell is >= that value
    # and is not observable at this build's constants. A distribution whose
    # top is a spike exactly at the threshold cannot be used to CHOOSE the
    # threshold -- the current value is manufacturing the tail.
    if speak == ACCESS_STALL_SECS:
        print(f"     CENSORED: stalled_peak == ACCESS_STALL_SECS ({speak}) exactly. "
              "True stall dwell is >= this and UNMEASURED at this build.")
    if peak == ACCESS_STALE_SECS:
        print(f"     CENSORED: idle_peak == ACCESS_STALE_SECS ({peak}) exactly. "
              "True idle dwell is >= this and UNMEASURED at this build.")
    # A latched-but-progressing claim is the case item 2 must NOT prune: long C
    # dwell with a low stalled_peak is the HEALTHY shape, not a defect. Report
    # it so the fan's distribution is readable, never fail on it.
    if c > 0:
        print(f"     stall clock: peak={speak} over {c} C-passes "
              f"({'threshold ' + str(ACCESS_STALL_SECS) + ' not approached' if speak < ACCESS_STALL_SECS * 0.5 else 'approaching threshold'})")

    print(f"{'PASS' if ok else 'FAIL'} {path}")
    print(f"     A={a} B={b} C={c} (total {total}) transitions={tr} "
          f"peak={peak} prunes={pr}")
    return ok


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    results = [check(p) for p in sys.argv[1:]]
    print()
    print(f"{sum(results)}/{len(results)} file(s) passed")
    sys.exit(0 if all(results) else 2)
