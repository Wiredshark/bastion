#!/bin/sh
# score-work-units.sh — test the claim I made when I built `work_units`:
# "twin runs must agree EXACTLY on it".
#
# ★ THIS IS A REGISTERED PREDICTION, NOT A DEMONSTRATION. I asserted the counter
# is deterministic because it counts simulation work rather than time. That is a
# claim about the code, and an untested claim about determinism is exactly the
# kind this program has been burned by: `b5_soak_avg_tick_ms` also *looked*
# reproducible until its 1.21x floor was measured.
#
# THE SHAPE: same seed, TWO separate processes (hashbrown's iteration seed is
# per-process, so a single process could agree with itself for the wrong
# reason), each with its OWN data dir (rtsim persists; a shared dir makes the
# second run a restart, not an independent run). Both protections are inherited
# from aa-pair.sh, for the reasons its comments give.
#
# REGISTERED OUTCOMES, declared before running:
#   DETERMINISTIC   — work_units identical on every seed. The claim holds and
#                     the counter can serve as a determinism witness.
#   NON-DETERMINISTIC — any seed differs. The claim is REFUTED; the counter is a
#                     performance proxy only, and the "twin runs must agree"
#                     line in its doc comment must be struck.
#   VOID            — work_units is 0 or absent everywhere. It never
#                     incremented, so agreement would be vacuous. This is the
#                     failure mode a bare "they matched!" would hide.
#
# Usage: sh score-work-units.sh <outdir> <seed> [seed...]
set -u
# ★ corpus-first, enforced for LOCAL runs too (see corpus-first.sh)
. "$(dirname "$0")/corpus-first.sh"
OUT="${1:?usage: score-work-units.sh <outdir> <seed> [seed...]}"; shift
[ $# -ge 1 ] || { echo "REFUSED: no seeds given" >&2; exit 2; }
HARNESS=./target/verify/bastion-harness.exe
[ -x "$HARNESS" ] || { echo "REFUSED: no harness at $HARNESS" >&2; exit 2; }

GH=$("$HARNESS" --print-git-hash 2>/dev/null)
RH=$(git rev-parse --short=10 HEAD)
echo "harness git-hash : $GH"
echo "repo HEAD        : $RH"
if [ "$GH" != "$RH" ]; then
  CHANGED=$(git diff --name-only "$GH" HEAD -- '*.rs' 2>/dev/null)
  if [ -n "$CHANGED" ]; then
    echo "!! REFUSING: binary is $GH, HEAD is $RH, and these .rs differ:" >&2
    echo "$CHANGED" | sed 's/^/     /' >&2; exit 4
  fi
  echo "  (binary != HEAD, but no .rs differs — safe to score)"
fi
[ "$(git status --short -- '*.rs' | wc -l)" -eq 0 ] || {
  echo "!! REFUSING: uncommitted .rs changes" >&2; exit 5; }
echo "seeds            : $*"
echo

mkdir -p "$OUT"
for ARM in A B; do
  rm -rf "$OUT/dd-$ARM"; mkdir -p "$OUT/dd-$ARM"
  for s in "$@"; do
    "$HARNESS" --b5-scenario --seed "$s" --data-dir "$OUT/dd-$ARM/s$s" \
      > "$OUT/$ARM-$s.json" 2> "$OUT/$ARM-$s.err" &
  done
  wait
  echo "arm $ARM done"
done
echo

python - "$OUT" "$@" <<'PY'
import json, sys, os, re
def winpath(p):
    return p[1] + ":" + p[2:] if re.match(r"^/[a-zA-Z]/", p) else p
out = winpath(sys.argv[1]); seeds = sys.argv[2:]

def load(arm, s):
    p = os.path.join(out, f"{arm}-{s}.json")
    if not os.path.exists(p) or os.path.getsize(p) == 0:
        return None
    best = None
    for line in open(p, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            o = json.loads(line)
        except Exception:
            continue
        if isinstance(o, dict):
            best = o
    return best

print(f"{'seed':<6}{'A_work':<12}{'B_work':<12}{'match':<8}{'A_ms':<10}{'B_ms':<10}")
rows, exercised = [], 0
for s in seeds:
    a, b = load("A", s), load("B", s)
    if a is None or b is None:
        print(f"{s:<6}DROPPED: missing payload"); continue
    aw, bw = a.get("b5_work_units"), b.get("b5_work_units")
    am, bm = a.get("b5_soak_avg_tick_ms"), b.get("b5_soak_avg_tick_ms")
    if aw is None or bw is None:
        print(f"{s:<6}DROPPED: b5_work_units absent from the payload"); continue
    if aw > 0 or bw > 0:
        exercised += 1
    print(f"{s:<6}{aw:<12}{bw:<12}{str(aw==bw):<8}{am:<10.2f}{bm:<10.2f}")
    rows.append((s, aw, bw, am, bm))

print()
if not rows:
    print("VERDICT: VOID — no scoreable payloads."); sys.exit(3)
if exercised == 0:
    print("VERDICT: VOID — work_units is 0 on every seed. It never incremented,")
    print("         so agreement is vacuous and proves nothing about determinism.")
    sys.exit(3)
print(f"PRECONDITION: {exercised} of {len(rows)} seeds actually incremented work_units.")
bad = [s for s, aw, bw, _, _ in rows if aw != bw]
# the contrast that gives the result its meaning
ms_bad = [s for s, _, _, am, bm in rows if abs(am - bm) > 1e-9]
print(f"work_units differing : {len(bad)} of {len(rows)}  {bad if bad else ''}")
print(f"wall-clock differing : {len(ms_bad)} of {len(rows)}  (the same runs)")
print()
if not bad:
    print("VERDICT: DETERMINISTIC — work_units is identical on every exercised seed")
    print("         while the wall-clock number on those SAME runs is not. The claim")
    print("         holds, and the contrast is the evidence: a counter that agrees")
    print("         where a clock disagrees is measuring the simulation, not the host.")
else:
    print("VERDICT: NON-DETERMINISTIC — the claim is REFUTED. Strike the")
    print("         'twin runs must agree exactly' line from work_units' doc comment;")
    print("         it is a performance proxy only, not a determinism witness.")
PY
