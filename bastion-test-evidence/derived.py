#!/usr/bin/env python3
"""Report DERIVED quantities a wave's raw fields cannot show on their own.

    python derived.py WAVE_FULL.json [WAVE_FULL.json ...]

WHY THIS EXISTS (DECISIONS #66, ratified 2026-08-08). The corpus recorded a
100% failure rate faithfully for 25 waves and nobody saw it, because the
finding was a SUBTRACTION of two adjacent columns that no report performed:

    access_plan_self_rescue_calls      71
    access_plan_self_rescue_emissions   0     <- refusal rate 100%

Both fields were built FOR that question (DECISIONS #49) and both were
present in every baseline. Neither was ever differenced.

  A MEASUREMENT NOBODY COMPUTES IS INDISTINGUISHABLE FROM ONE NOBODY TOOK.

The GATE-FIELDS packet slot protects the NEXT row. This protects the ~89
fields already collected every wave.

THREE DERIVED FAMILIES, and each one exists because it was needed and absent:

  1. RATES      -- every *_calls/*_emissions pair, auto-discovered by name.
                   A count answers "did it happen"; only a rate answers
                   "how often did it WORK".
  2. CONCENTRATION -- any flagged population crossed against the verdict,
                   with Fisher's exact test. Answers "do these seeds fail
                   more than base rate".
  3. DENOMINATORS -- populations whose comparison group is EMPTY. A 100%
                   rate cannot distinguish cause from marker when no seed
                   ever succeeded; that is an INSTRUMENT GAP, not a result,
                   and it is reported as one rather than left to be
                   mistaken for a finding.

Refuses rather than guesses: no verdict field, no seeds, or a schema whose
pairs don't line up are reported as such and exit 2.
"""
import collections
import glob
import json
import re
import sys
from math import comb

VERDICT_FIELD = "b5_failed_clauses"


def failed(v):
    """-> True/False/None. None means the verdict could not be read AT ALL,
    which must never silently become 'passed'."""
    fc = v.get(VERDICT_FIELD)
    if fc is None:
        return None
    if isinstance(fc, list):
        return len(fc) > 0
    if isinstance(fc, bool):
        return fc
    if isinstance(fc, (int, float)):
        return fc > 0
    if isinstance(fc, str):
        return bool(fc.strip())
    return None


def fisher_two_sided(a, b, c, d):
    """2x2 exact test. Returns None when a margin is zero (undefined, not 1.0
    -- an undefined test reported as non-significant is the same defect this
    whole file is about)."""
    n = a + b + c + d
    r1, c1 = a + b, a + c
    if n == 0 or r1 in (0, n) or c1 in (0, n):
        return None

    def p(x):
        return comb(r1, x) * comb(n - r1, c1 - x) / comb(n, c1)

    lo = max(0, c1 - (n - r1))
    hi = min(r1, c1)
    obs = p(a)
    return sum(p(x) for x in range(lo, hi + 1) if p(x) <= obs + 1e-12)


def discover_pairs(keys):
    """-> [(label, calls_key, emissions_key)] by NAME, not a hardcoded list --
    a new caller added upstream must appear here without editing this file."""
    pairs = []
    for k in keys:
        if not k.endswith("_calls"):
            continue
        stem = k[: -len("_calls")]
        em = stem + "_emissions"
        if em in keys:
            pairs.append((stem, k, em))
    return sorted(pairs)


def report_rates(seeds, keys):
    pairs = discover_pairs(keys)
    print("\n== 1. REFUSAL RATES (calls - emissions, auto-discovered pairs)")
    if not pairs:
        print("   no *_calls/*_emissions pairs in this schema")
        return {}
    flagged = {}
    for stem, ck, ek in pairs:
        calls = sum(v.get(ck) or 0 for v in seeds.values())
        em = sum(v.get(ek) or 0 for v in seeds.values())
        called = {s for s, v in seeds.items() if (v.get(ck) or 0) > 0}
        if calls == 0:
            print("   %-46s NEVER CALLED (denominator 0)" % stem)
            continue
        rate = 100.0 * (calls - em) / calls
        mark = "  <<< TOTAL" if em == 0 else ""
        print("   %-46s calls=%-5d emitted=%-5d refused=%-5d (%5.1f%%) in %d seeds%s"
              % (stem, calls, em, calls - em, rate, len(called), mark))
        flagged[stem] = (called, em)
    return flagged


def report_concentration(seeds, flagged):
    print("\n== 2. CONCENTRATION vs VERDICT (%s)" % VERDICT_FIELD)
    verd = {s: failed(v) for s, v in seeds.items()}
    unreadable = [s for s, x in verd.items() if x is None]
    if unreadable:
        print("   REFUSED: %d seed(s) have no readable verdict -- a missing "
              "verdict must not be counted as a pass." % len(unreadable))
        return 2
    n = len(seeds)
    nf = sum(1 for x in verd.values() if x)
    print("   base fail rate = %d/%d = %.1f%%" % (nf, n, 100.0 * nf / n))
    for stem, (called, em) in sorted(flagged.items()):
        if not called:
            continue
        a = sum(1 for s in called if verd[s])          # flagged & fail
        b = len(called) - a
        c = nf - a                                     # unflagged & fail
        d = n - len(called) - c
        p = fisher_two_sided(a, b, c, d)
        ps = "undefined (a margin is zero)" if p is None else "%.4f" % p
        print("   %-46s fail %2d/%2d = %5.1f%%   Fisher two-sided p = %s"
              % (stem, a, len(called), 100.0 * a / len(called), ps))
    return 0


def report_denominators(seeds, flagged):
    """The clause that would have caught today's false falsifier."""
    print("\n== 3. EMPTY COMPARISON GROUPS (exercised-denominator = 0)")
    gaps = [stem for stem, (called, em) in flagged.items() if called and em == 0]
    if not gaps:
        print("   none -- every flagged population has both successes and failures")
        return
    for stem in sorted(gaps):
        print("   %-46s INSTRUMENT GAP" % stem)
    print("\n   These have ZERO successful seeds, so 'refused' cannot be compared")
    print("   against 'succeeded' WITHIN this caller. A concentration result")
    print("   above is therefore UNATTRIBUTABLE -- it cannot separate cause from")
    print("   marker. Use a sibling caller that partially succeeds as the")
    print("   control, or declare the gap. Do NOT declare a finding.")


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 2
    paths = []
    for a in args:
        paths.extend(sorted(glob.glob(a)) or [a])

    rc = 0
    for path in paths:
        try:
            with open(path, encoding="utf-8") as fh:
                seeds = json.load(fh)
        except Exception as e:
            print("REFUSED %s: %s" % (path, e))
            rc = 2
            continue
        name = re.split(r"[\\/]", path)[-1]
        print("\n" + "=" * 72)
        print("WAVE %s -- %d seeds" % (name, len(seeds)))
        print("=" * 72)
        if not seeds:
            print("REFUSED: zero seeds. An empty dict shaped like a baseline is "
                  "the wave13 failure -- not a result.")
            rc = 2
            continue
        counts = collections.Counter(frozenset(v) for v in seeds.values())
        keys, modal_n = counts.most_common(1)[0]
        if modal_n != len(seeds):
            print("[!!] schema not uniform: modal key set held by %d/%d seeds"
                  % (modal_n, len(seeds)))
        if VERDICT_FIELD not in keys:
            print("REFUSED: no %s field -- concentration is not computable and "
                  "an absent verdict must not default to pass." % VERDICT_FIELD)
            rc = 2
            continue
        flagged = report_rates(seeds, keys)
        rc = report_concentration(seeds, flagged) or rc
        report_denominators(seeds, flagged)
    return rc


if __name__ == "__main__":
    sys.exit(main())
