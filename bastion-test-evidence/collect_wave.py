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


def _shape_compatible(other, mine_keys):
    """Is `other` the SAME SCENARIO as the wave being written?

    Two tests, both cheap, and each catches a case the other misses:

      1. Every seed carries the verdict field AT TOP LEVEL. A paired A/B wave
         nests everything under paired_base/paired_variant/paired_delta, so
         its verdict is one level down and it fails here. This is also what
         makes a wave SCORABLE at all -- an unscorable baseline contributes
         nothing but a refusal.
      2. Top-level key sets overlap substantially (Jaccard >= 0.5). Deliberately
         NOT equality: waves legitimately gain fields (wave26 127 leaves ->
         wave30 132), and demanding an identical schema would reject every
         additive window -- i.e. it would fail in the direction that looks
         strict while making the tool useless.
    """
    import derived as _d
    keys = set()
    for v in other.values():
        if not isinstance(v, dict) or _d.VERDICT_FIELD not in v:
            return False
        keys |= set(v)
    if not keys or not mine_keys:
        return False
    return len(keys & mine_keys) / float(len(keys | mine_keys)) >= 0.5


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
    # DECISIONS #67 (Fable-ruled): a wave used to SCORE a registration
    # (e.g. wave32 against the REG-A..D predictions in
    # WAVE32-PREREGISTRATION.md) must name its baseline explicitly --
    # auto-selection picks the newest structurally-compatible wave, which
    # is a reasonable default for exploration but not a substitute for
    # the SPECIFIC wave a pre-registration was written against. `wave30`
    # and `wave31` are both valid auto-picks for a wave32-shaped run, and
    # picking the wrong one silently answers a different question than
    # the one that was registered. `--baseline` makes the choice a
    # command-line fact instead of a rule buried in cross-wave selection
    # logic -- and REFUSES outright (does not fall back to auto-select)
    # if the named wave doesn't exist, doesn't parse, or isn't actually
    # comparable, so a typo'd --baseline can't silently degrade into an
    # unnamed auto-pick.
    argv = sys.argv[1:]
    baseline_arg = None
    if "--baseline" in argv:
        i = argv.index("--baseline")
        if i + 1 >= len(argv):
            print("REFUSED: --baseline given with no value.")
            return 2
        baseline_arg = argv[i + 1]
        argv = argv[:i] + argv[i + 2:]

    if len(argv) < 2:
        print(__doc__)
        print("\n    --baseline WAVE   name the SPECIFIC earlier wave to score")
        print("                      against (registered mode). Without it,")
        print("                      cross-wave comparison auto-selects and")
        print("                      labels its own output EXPLORATORY.")
        return 2
    out_path, logs = argv[0], argv[1:]

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
        # Cross-wave identity delta against the newest EARLIER wave whose
        # seed set is IDENTICAL. Auto-picking a baseline is the dangerous
        # part -- a delta against a wave that ran different seeds invents
        # both FIXED and NEW -- so the identical-set requirement is the
        # SELECTION rule, not just a check afterwards. The chosen baseline
        # is NAMED in the output; an unnamed automatic comparison is a
        # number whose meaning nobody can reconstruct later.
        #
        # Baseline resolution/validation runs BEFORE the stdout redirect
        # below -- a REFUSED here must print to the REAL terminal and
        # return immediately. Printing it into the redirected buffer and
        # then returning before that buffer is ever flushed would bury
        # the refusal exactly as silently as the thing this whole
        # mechanism exists to stop (found by testing this refusal path
        # directly, not assumed correct after writing it).
        mine = set(parsed)
        mine_keys = set()
        for v in parsed.values():
            if isinstance(v, dict):
                mine_keys |= set(v)
        here = os.path.dirname(os.path.abspath(out_path))
        wrong_shape = []

        if baseline_arg is not None:
            # REGISTERED MODE (#67): the baseline is a command-line fact,
            # not an auto-pick. Resolve permissively (literal path,
            # relative to `here`, or a name-substring glob under `here`)
            # but VALIDATE strictly -- any failure REFUSES the whole run
            # rather than degrading into exploratory auto-select, so a
            # typo'd --baseline can't silently produce an unnamed
            # comparison.
            candidates_on_disk = []
            for guess in (baseline_arg, os.path.join(here, baseline_arg)):
                if os.path.isfile(guess):
                    candidates_on_disk.append(guess)
            if not candidates_on_disk:
                matches = glob.glob(os.path.join(here, "*%s*_FULL.json" % baseline_arg))
                candidates_on_disk = matches
            if len(candidates_on_disk) == 0:
                print("\nREFUSED: --baseline %r matched no file (checked "
                      "as a literal path, relative to %s, and as a "
                      "*%s*_FULL.json glob there)." % (baseline_arg, here, baseline_arg))
                return 2
            if len(candidates_on_disk) > 1:
                print("\nREFUSED: --baseline %r is ambiguous -- matched "
                      "%d files: %s" % (baseline_arg, len(candidates_on_disk),
                                         ", ".join(sorted(candidates_on_disk))))
                return 2
            bpath = candidates_on_disk[0]
            bname = re.split(r"[\\/]", bpath)[-1]
            try:
                with open(bpath, encoding="utf-8") as f2:
                    bother = json.load(f2)
            except Exception as e:
                print("\nREFUSED: --baseline %r (%s) did not parse as JSON: %r"
                      % (baseline_arg, bpath, e))
                return 2
            if not bother:
                print("\nREFUSED: --baseline %r (%s) is an empty wave."
                      % (baseline_arg, bpath))
                return 2
            if set(bother) != mine:
                only_mine = sorted(mine - set(bother))
                only_base = sorted(set(bother) - mine)
                print("\nREFUSED: --baseline %r (%s) does not share this "
                      "wave's exact seed set. In this wave only: %s. In "
                      "the baseline only: %s." % (baseline_arg, bpath,
                                                   only_mine[:6], only_base[:6]))
                return 2
            if not _shape_compatible(bother, mine_keys):
                print("\nREFUSED: --baseline %r (%s) has an incompatible "
                      "scenario shape (verdict field not at top level, "
                      "or key-set overlap below 50%%) despite an "
                      "identical seed set." % (baseline_arg, bpath))
                return 2
            try:
                bn = derived.wave_number(bname)
            except ValueError:
                bn = -1
            cands = [(bn, bname, bother)]
        else:
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
                if not other or set(other) != mine:
                    continue
                # SELECTION RULE, second half (2026-08-08): a baseline is
                # chosen by SCENARIO SHAPE **and** seed set -- never by seed
                # set alone, and never by recency. wave29 shares seeds 49-96
                # with every plain wave and is a PAIRED A/B run whose keys sit
                # under paired_base/paired_variant/paired_delta. Same seeds is
                # not the same scenario. Excluded HERE, by name, rather than
                # discovered downstream when its verdict lookup fails.
                if not _shape_compatible(other, mine_keys):
                    wrong_shape.append(nm)
                    continue
                cands.append((n, nm, other))
            try:
                mine_n = derived.wave_number(this_name)
                cands = [c for c in cands if c[0] < mine_n]
            except ValueError:
                cands = []

        mode_tag = "REGISTERED" if baseline_arg is not None else "EXPLORATORY"

        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            drc = derived.report_wave(parsed, this_name)
            if wrong_shape:
                print("\n(excluded from cross-wave, INCOMPATIBLE SCENARIO "
                      "SHAPE despite an identical seed set: %s)"
                      % ", ".join(sorted(wrong_shape)))
            if cands:
                # Pass EVERY comparable wave, not just the newest. Recency-
                # singular selection lost the whole analysis on wave30: it
                # picked the newest candidate, that one turned out to be
                # unusable, and the run reported "fewer than 2 waves carry a
                # readable verdict" while two perfectly good baselines sat on
                # disk. `derived.cross_wave` already sets aside unusable waves
                # BY NAME and groups by seed set -- handing it one wave threw
                # away the recovery it was built to do.
                cands.sort(key=lambda c: c[0])
                if baseline_arg is not None:
                    print("\n(%s baseline, explicitly given via --baseline: %s)"
                          % (mode_tag, ", ".join(c[1] for c in cands)))
                else:
                    print("\n(%s cross-wave baselines: %s -- every earlier wave "
                          "with an identical seed set AND a compatible shape, "
                          "auto-selected because no --baseline was given)"
                          % (mode_tag, ", ".join(c[1] for c in cands)))
                derived.cross_wave([(c[1], c[2]) for c in cands]
                                   + [(this_name, parsed)])
            else:
                print("\n(%s: no cross-wave baseline: no earlier wave in %s "
                      "shares this seed set exactly AND a compatible shape. "
                      "FIXED/NEW are not computed -- they would be "
                      "meaningless across differing sets.)" % (mode_tag, here))
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
