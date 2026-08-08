#!/usr/bin/env python3
"""Turn vm-pool.sh's streamed per-VM logs into a wave `_FULL.json`.

    python collect_wave.py OUT_FULL.json POOL_LOG [POOL_LOG ...]

vm-pool.sh streams each seed's harness JSON back inside `@@@SEED n@@@` markers
because the VMs are deleted immediately afterwards. Nothing turned that stream
into the per-seed structured file a future hold-check needs — which is exactly
how wave24 ended up a verdict with no body (DECISIONS #56).

THREE THINGS THIS REFUSES TO DO SILENTLY, each an instance of the same law:

  * a seed whose block is EMPTY or unparseable   -> named, EXCLUDED, counted
  * a seed missing keys the others have          -> named, EXCLUDED as UNPROVEN
  * a run with no COMMIT= attestation            -> refuses to write at all

The schema assert is not fussiness: 2026-08-03 a single b5 run emitted valid
JSON missing ~15 unconditional fields, with no repro. A classifier reading
absent fields as false/zero produces a confident wrong verdict, and at 48 seeds
nobody eyeballs every object.
"""
import collections
import contextlib
import glob
import io
import json
import os
import re
import sys

# Import `derived` from this script's own directory regardless of cwd -- the
# collector is documented as being run from bastion-test-evidence/ but has
# been run from the repo root more than once.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

SEED_RE = re.compile(r"^@@@SEED (\d+)@@@\s*$")


def parse_log(path):
    """-> (commit, {seed: raw_text}). Blocks run to the next marker."""
    commit, blocks, cur, buf = None, {}, None, []
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if line.startswith("COMMIT="):
                commit = line.strip().split("=", 1)[1]
            m = SEED_RE.match(line)
            if m:
                if cur is not None:
                    blocks[cur] = "".join(buf)
                cur, buf = m.group(1), []
            elif cur is not None:
                buf.append(line)
    if cur is not None:
        blocks[cur] = "".join(buf)
    return commit, blocks


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    out_path, logs = sys.argv[1], sys.argv[2:]

    commits, raw = {}, {}
    for lg in logs:
        c, blocks = parse_log(lg)
        commits[lg] = c
        for seed, text in blocks.items():
            if seed in raw:
                print(f"REFUSED: seed {seed} appears in two logs - overlapping "
                      "seed ranges make the wave ambiguous.")
                return 2
            raw[seed] = text

    print("ATTESTATION")
    missing_att = [lg for lg, c in commits.items() if not c]
    for lg, c in commits.items():
        print(f"    {lg}: COMMIT={c or '*** ABSENT ***'}")
    if missing_att:
        print(f"REFUSED: {len(missing_att)} log(s) carry no COMMIT= line. "
              "A wave whose commit isn't verified is not evidence.")
        return 2
    distinct = set(commits.values())
    if len(distinct) != 1:
        print(f"REFUSED: logs attest DIFFERENT commits {sorted(distinct)} - "
              "part of the pool ran a different tip (the mid-fan push hazard).")
        return 2
    commit = distinct.pop()

    # The harness prints its JSON object and THEN a human verdict line
    # ("B5 SCENARIO: PASS"). Take the leading object with raw_decode, then
    # INSPECT the remainder rather than discarding it - trailing bytes we
    # cannot account for are a malformed seed, not something to skip past.
    # Verdict lines are not uniform: "B5 SCENARIO: PASS" but also
    # "B5ROWBPAIRED: PASS". Accept NAME: PASS/FAIL with or without SCENARIO,
    # but keep the NAME shape tight -- a loose pattern here would swallow
    # arbitrary trailing bytes, which is the thing this check exists to catch.
    VERDICT_RE = re.compile(r"^[A-Z0-9][A-Z0-9 _-]*: (PASS|FAIL)(.*)$",
                            re.DOTALL)
    dec = json.JSONDecoder()
    parsed, bad, verdicts = {}, {}, {}
    for seed, text in raw.items():
        t = text.strip()
        if not t:
            bad[seed] = "EMPTY block (harness produced no JSON)"
            continue
        try:
            obj, end = dec.raw_decode(t)
        except json.JSONDecodeError as e:
            bad[seed] = f"UNPARSEABLE ({e.msg} at char {e.pos}; {len(t)} chars)"
            continue
        rest = t[end:].strip()
        if rest:
            m = VERDICT_RE.match(rest)
            if not m:
                bad[seed] = (f"TRAILING CONTENT after the JSON, unrecognised: "
                             f"{rest[:60]!r} ({len(rest)} chars)")
                continue
            verdicts[seed] = m.group(1)
        parsed[seed] = obj

    # ---- schema assert against the MODAL key set, not a hardcoded list ----
    unproven = {}
    if parsed:
        counts = collections.Counter(frozenset(v) for v in parsed.values())
        modal, modal_n = counts.most_common(1)[0]
        for seed, v in list(parsed.items()):
            gap = modal - set(v)
            if gap:
                unproven[seed] = sorted(gap)
                del parsed[seed]
        print(f"\nSCHEMA: modal key set = {len(modal)} keys, held by "
              f"{modal_n}/{len(raw)} seeds")

    print(f"\nseeds streamed : {len(raw)}")
    print(f"seeds usable   : {len(parsed)}")
    if bad:
        print(f"[!!] MALFORMED ({len(bad)}) - excluded, NOT counted as data:")
        for s, why in sorted(bad.items(), key=lambda kv: int(kv[0])):
            print(f"      seed {s}: {why}")
    if unproven:
        print(f"[!!] UNPROVEN ({len(unproven)}) - short schema, excluded:")
        for s, gap in sorted(unproven.items(), key=lambda kv: int(kv[0])):
            print(f"      seed {s}: missing {len(gap)} key(s): {gap[:6]}")

    if not parsed:
        print("\nREFUSED: zero usable seeds. Writing this would produce an "
              "empty dict shaped like a real baseline (wave13).")
        return 2

    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(parsed, fh, indent=1, sort_keys=True)
    print(f"\nwrote {out_path}: {len(parsed)} seeds @ COMMIT={commit}")

    # DECISIONS #66: the derived-quantity check runs HERE, not when someone
    # remembers. The 100% self-rescue refusal rate sat unread because the
    # finding was a subtraction of two adjacent columns that no report
    # performed -- and a standing check that must be invoked by hand is the
    # same failure mode one level up. A wave cannot be produced without its
    # rates, concentrations, and instrument-gaps being produced with it.
    #
    # Deliberately NON-FATAL to the collection: the seed data is valid
    # regardless of what the analysis finds, and conflating "I could not
    # analyse this" with "the collection failed" would be this repo's own
    # exclusion-vs-absence defect. The derived rc is REPORTED, never merged
    # into the collector's exit code.
    derived_txt = re.sub(r"\.json$", "", out_path) + ".DERIVED.txt"
    try:
        import derived

        this_name = re.split(r"[\\/]", out_path)[-1]
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            drc = derived.report_wave(parsed, this_name)
            # Cross-wave identity delta against the newest EARLIER wave whose
            # seed set is IDENTICAL. Auto-picking a baseline is the dangerous
            # part -- a delta against a wave that ran different seeds invents
            # both FIXED and NEW -- so the identical-set requirement is the
            # SELECTION rule, not just a check afterwards. The chosen baseline
            # is NAMED in the output; an unnamed automatic comparison is a
            # number whose meaning nobody can reconstruct later.
            mine = set(parsed)
            here = os.path.dirname(os.path.abspath(out_path))
            cands = []
            for p in glob.glob(os.path.join(here, "*_FULL.json")):
                nm = re.split(r"[\\/]", p)[-1]
                if nm == this_name:
                    continue
                try:
                    n = derived.wave_number(nm)
                    with open(p, encoding="utf-8") as f2:
                        other = json.load(f2)
                except Exception:
                    continue
                if other and set(other) == mine:
                    cands.append((n, nm, other))
            try:
                mine_n = derived.wave_number(this_name)
                cands = [c for c in cands if c[0] < mine_n]
            except ValueError:
                cands = []
            if cands:
                base = max(cands, key=lambda c: c[0])
                print("\n(cross-wave baseline auto-selected: %s -- newest "
                      "earlier wave with an IDENTICAL seed set)" % base[1])
                derived.cross_wave([(base[1], base[2]), (this_name, parsed)])
            else:
                print("\n(no cross-wave baseline: no earlier wave in %s shares "
                      "this seed set exactly. FIXED/NEW are not computed -- "
                      "they would be meaningless across differing sets.)" % here)
        body = buf.getvalue()
        with open(derived_txt, "w", encoding="utf-8") as fh:
            fh.write(body)
        # Echo to stdout too: evidence has to land in the transcript, not
        # only on a disk someone else may clean.
        print(body)
        print(f"wrote {derived_txt}  (derived rc={drc}, "
              f"NOT merged into this script's exit code)")
    except Exception as e:
        print(f"[!!] derived check could not run: {e!r}")
        print("     The wave file above is still valid. Run "
              "`python derived.py <the wave>` by hand and find out why.")

    return 0 if not (bad or unproven) else 1


if __name__ == "__main__":
    sys.exit(main())
