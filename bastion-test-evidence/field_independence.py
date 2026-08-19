"""How many INDEPENDENT signals does a wave actually carry?

A wave reports movers as a per-field tally: fields that moved, over fields
available. Both halves of that fraction can be wrong, and measured on wave34
(48 seeds, 119 `b5_*` fields) both were:

  DEAD fields      50 of 119 (42%) are constant across every seed. They can
                   never be reported as movers, so they dilute the denominator
                   and make any concentration look stronger than it is.

  TIED columns     7 pairs are element-wise identical while genuinely varying.
                   One real change is then counted TWICE (three times for a
                   triple), inflating the numerator.

Both biases push the SAME way -- toward overstating concentration -- so a wave
that does not correct for them cannot be conservative by accident.

★ WHY A TIE NEEDS ITS OWN SIGNIFICANCE TEST. Two lopsided booleans match across
  48 seeds cheaply, and 119 fields make 7,021 pairs, so ties happen by chance.
  This reports p(match | the field's own multiset) per tie and flags the ones
  that survive Bonferroni. Measured: the `_measured` twins tie at p=0.021 --
  statistically unremarkable among 7,021 pairs -- and are nonetheless genuinely
  redundant, because the harness assigns them from the same variable. So the
  script reports BOTH and asserts neither alone: a tie that fails correction is
  a prompt to read the producer, not a verdict.

Deliberately does NOT delete or rewrite anything. A field constant across one
configuration may be the one that moves when the configuration changes, and
several dead fields are deliberate invariants.

Usage:  python field_independence.py <dir-of-seed-logs> [field-prefix]
"""
import collections, glob, itertools, json, math, os, re, sys

PREFIX = sys.argv[2] if len(sys.argv) > 2 else "b5_"


def load(path):
    """Extract the trailing JSON object containing PREFIX-named fields.

    Brace-matched, NOT regex: a non-greedy `\[.*?\]` truncates any nested
    array at its first inner `]` and yields invalid JSON. That mistake reported
    a populated field as empty on 8 of 48 seeds before it was caught.
    """
    txt = open(path, encoding="utf-8", errors="replace").read()
    m = re.search(r'\{[^{]*"' + re.escape(PREFIX) + r'[a-z_0-9]+"', txt)
    if not m:
        return None
    i, depth = m.start(), 0
    for e in range(i, len(txt)):
        if txt[e] == "{":
            depth += 1
        elif txt[e] == "}":
            depth -= 1
            if depth == 0:
                try:
                    return json.loads(txt[i : e + 1])
                except ValueError:
                    return None
    return None


def main(d):
    rows = {}
    for f in sorted(glob.glob(os.path.join(d, "*.log"))):
        r = load(f)
        if r:
            rows[os.path.basename(f)] = r
    if not rows:
        print("!! no seed logs parsed under %s -- NOT a clean result" % d)
        return 4
    keys = sorted({k for r in rows.values() for k in r if k.startswith(PREFIX)})
    col = {k: [json.dumps(r.get(k), sort_keys=True) for r in rows.values()] for k in keys}
    n = len(rows)

    dead = [k for k in keys if len(set(col[k])) == 1]
    live = [k for k in keys if len(set(col[k])) > 1]
    groups = collections.defaultdict(list)
    for k in live:
        groups[tuple(col[k])].append(k)

    npairs = len(keys) * (len(keys) - 1) // 2
    bonf = 0.05 / npairs if npairs else 1.0
    print("seeds parsed ................. %d" % n)
    print("%s* fields declared ........... %d" % (PREFIX, len(keys)))
    print("DEAD (constant every seed) ... %d  (%.0f%%)" % (len(dead), 100 * len(dead) / len(keys)))
    print("live ......................... %d" % len(live))
    print("INDEPENDENT signals .......... %d" % len(groups))
    print("cannot move alone ............ %d of %d  (%.0f%%)"
          % (len(dead) + len(live) - len(groups), len(keys),
             100 * (len(dead) + len(live) - len(groups)) / len(keys)))
    print()
    print("TIED COLUMN GROUPS  (pairs tested: %d, Bonferroni p < %.2e)" % (npairs, bonf))
    any_tie = False
    for cols, members in sorted(groups.items(), key=lambda x: -len(x[1])):
        if len(members) < 2:
            continue
        any_tie = True
        c = collections.Counter(cols)
        den = 1
        for v in c.values():
            den *= math.factorial(v)
        p = den / math.factorial(n)
        print("   %-58s p=%.2e  %s"
              % (" == ".join(members)[:58], p,
                 "SURVIVES correction" if p < bonf else "fails correction -> READ THE PRODUCER"))
    if not any_tie:
        print("   none -- every live field carries its own signal")
    print()
    print("Report the INDEPENDENT count beside the field count in any wave tally.")
    print("Neither number is wrong; they answer different questions, and only one")
    print("of them is the denominator a mover rate should be divided by.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "."))
