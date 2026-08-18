#!/bin/sh
# aa-pair.sh — run the SAME seeds twice at the SAME commit and emit two
# wave-shaped JSONs, for the order-only test (#88).
#
# ★ EACH RUN GETS ITS OWN DATA DIR. rtsim persists (it saves every 60s), so a
#   second run pointed at the first run's dir is a RESTART, not an independent
#   run -- it would load the first run's world and the pair would test nothing.
#   This is the whole reason the dirs are named per ARM and wiped up front.
#
# ★ AND EACH RUN IS ITS OWN PROCESS. hashbrown's iteration seed is per-process,
#   so two runs in one process could agree by construction and the test would
#   pass for the wrong reason.
#
# Usage: bash aa-pair.sh <outdir> <seed> [seed...]
set -u
OUT="$1"; shift
[ $# -ge 1 ] || { echo "REFUSED: no seeds given"; exit 2; }
HARNESS=./target/verify/bastion-harness.exe
[ -x "$HARNESS" ] || { echo "REFUSED: no harness at $HARNESS"; exit 2; }

# Attestation FIRST -- the whole prediction is about which code ran.
GH=$("$HARNESS" --print-git-hash 2>/dev/null)
RH=$(git rev-parse --short=10 HEAD)
echo "harness git-hash : $GH"
echo "repo HEAD        : $RH"
[ "$GH" = "$RH" ] || echo "!! MISMATCH -- binary is not HEAD; results are NOT attributable to HEAD"
echo "dirty .rs files  : $(git status --short -- '*.rs' | wc -l)"
echo "seeds            : $*"

mkdir -p "$OUT"
for ARM in A B; do
  rm -rf "$OUT/dd-$ARM"; mkdir -p "$OUT/dd-$ARM"
  for s in "$@"; do
    # HARNESS_EXTRA: extra flags for BOTH arms, e.g. --deterministic-parallel.
    # Applied identically to A and B on purpose -- this script compares a build
    # against ITSELF, so any flag that differed between arms would silently turn
    # an A/A into an A/B.
    # shellcheck disable=SC2086
    "$HARNESS" --b5-scenario ${HARNESS_EXTRA:-} --seed "$s" --data-dir "$OUT/dd-$ARM/s$s" \
      > "$OUT/$ARM-$s.json" 2> "$OUT/$ARM-$s.err" &
  done
  wait
  echo "arm $ARM done: $(ls "$OUT"/$ARM-*.json | wc -l) files"
done

# Merge each arm into the wave shape {seed: payload}. A seed whose JSON failed
# to parse is DROPPED WITH A NAMED REFUSAL rather than silently skipped -- an
# absent seed and an excluded seed must not render identically.
python - "$OUT" "$@" <<'PY'
import json, sys, os
out = sys.argv[1]; seeds = sys.argv[2:]
for arm in ("A", "B"):
    doc, bad = {}, []
    for s in seeds:
        p = os.path.join(out, "%s-%s.json" % (arm, s))
        # The harness writes the payload on line 1 and a HUMAN verdict line
        # ("B5 SCENARIO: PASS") after it, so json.load on the whole file raises
        # "Extra data". Take the first line that parses as an object -- and
        # never fall back to "whatever parsed", which would let a partial file
        # through as if it were a run.
        try:
            with open(p, encoding="utf-8") as f:
                first = next(l for l in f if l.lstrip().startswith("{"))
            doc[s] = json.loads(first)
        except Exception as e:
            bad.append((s, type(e).__name__))
    with open(os.path.join(out, "wave-%s.json" % arm), "w", encoding="utf-8") as f:
        json.dump(doc, f)
    print("arm %s: %d seeds merged%s" % (arm, len(doc),
          ("  REFUSED: " + ", ".join("%s(%s)" % b for b in bad)) if bad else ""))
PY
