#!/bin/sh
# score-cancel-falsifier.sh — banked item 5: does `ch_cancel_clean` have a RED?
#
# THE PROBLEM. `b5_ch_cancel_clean` is true on ALL 41 seeds where it ran, and
# false only on the 7 where no tree was found — i.e. where it could not run.
# A check that has never gone red has never demonstrated that it CAN. Until it
# does, it carries no information beyond its own precondition.
#
# THE PLANT. BASTION_PLANT_CANCEL_MISS cancels a region 4096 blocks away rather
# than the tree's own AABB. The cancel still RUNS — the precondition is
# untouched — but misses, so the tree's jobs survive and the predicate must go
# FALSE.
#
# ★★ THE TRAP THIS SCRIPT EXISTS TO AVOID. `ch_cancel_clean` is ALSO false when
# `ch_aabb` is None, because the whole thing is `ch_aabb.is_some_and(...)`. So
# "the cancel missed and jobs survived" and "no tree was ever found" render
# IDENTICALLY as `false`. Scoring the plant red without checking the
# precondition would be scoring a coincidence. Every verdict below therefore
# prints `b5_ch_trees` beside `b5_ch_cancel_clean`, and a seed with no tree is
# excluded BY NAME rather than counted as a success.
#
# REGISTERED OUTCOMES (declared before the run):
#   FALSIFIER LIVE — baseline true, planted false, ON SEEDS WITH A TREE.
#                    The check can go red; it is a real assertion.
#   VACUOUS        — planted ALSO true on seeds with a tree. The assertion
#                    cannot detect the failure it names. This is the finding
#                    worth having, and it is the bad one.
#   VOID           — no seed produced a tree. Neither arm could have run;
#                    nothing is learned about the falsifier.
#
# Usage: sh score-cancel-falsifier.sh <outdir> <seed> [seed...]
set -u
OUT="${1:?usage: score-cancel-falsifier.sh <outdir> <seed> [seed...]}"; shift
[ $# -ge 1 ] || { echo "REFUSED: no seeds given" >&2; exit 2; }

HARNESS=./target/verify/bastion-harness.exe
[ -x "$HARNESS" ] || { echo "REFUSED: no harness at $HARNESS" >&2; exit 2; }

GH=$("$HARNESS" --print-git-hash 2>/dev/null)
RH=$(git rev-parse --short=10 HEAD)
echo "harness git-hash : $GH"
echo "repo HEAD        : $RH"

# ★★ REFUSE, do not warn (2026-08-19). The first version printed
# "!! binary is not HEAD — check whether any .rs differs" and left the decision
# to a human who was about to score a PLANT that was not in the binary. That
# run would have shown baseline == planted and read as VACUOUS — the worst
# possible false verdict, since it says "this assertion cannot detect its own
# failure" when in truth the failure was never injected.
#
# A warning that must be read to be safe is not a safeguard. Decide it here.
if [ "$GH" != "$RH" ]; then
  CHANGED=$(git diff --name-only "$GH" HEAD -- '*.rs' 2>/dev/null)
  if [ -n "$CHANGED" ]; then
    echo "!! REFUSING: binary is $GH, HEAD is $RH, and these .rs files differ:" >&2
    echo "$CHANGED" | sed 's/^/     /' >&2
    echo "   The plant may not be in the binary. Rebuild before scoring." >&2
    exit 4
  fi
  echo "  (binary != HEAD, but no .rs differs — safe to score)"
fi
DIRTY=$(git status --short -- '*.rs' | wc -l)
echo "dirty .rs files  : $DIRTY"
if [ "$DIRTY" -ne 0 ]; then
  echo "!! REFUSING: uncommitted .rs changes — the binary cannot contain them." >&2
  exit 5
fi
echo "seeds            : $*"
echo "arms             : A=baseline   B=BASTION_PLANT_CANCEL_MISS=1"
echo

mkdir -p "$OUT"
for ARM in A B; do
  rm -rf "$OUT/dd-$ARM"; mkdir -p "$OUT/dd-$ARM"
  for s in "$@"; do
    if [ "$ARM" = B ]; then
      BASTION_PLANT_CANCEL_MISS=1 "$HARNESS" --b5-scenario --seed "$s" \
        --data-dir "$OUT/dd-$ARM/s$s" > "$OUT/$ARM-$s.json" 2> "$OUT/$ARM-$s.err" &
    else
      "$HARNESS" --b5-scenario --seed "$s" \
        --data-dir "$OUT/dd-$ARM/s$s" > "$OUT/$ARM-$s.json" 2> "$OUT/$ARM-$s.err" &
    fi
  done
  wait
  echo "arm $ARM done: $(ls "$OUT"/$ARM-*.json 2>/dev/null | wc -l) files"
done
echo

python - "$OUT" "$@" <<'PY'
import json, sys, os, re

def winpath(p):
    # Git Bash /c/Users/... is not openable by native Windows python.
    return p[1] + ":" + p[2:] if re.match(r"^/[a-zA-Z]/", p) else p

out = winpath(sys.argv[1]); seeds = sys.argv[2:]
CLEAN, TREES = "b5_ch_cancel_clean", "b5_ch_trees"

def load(arm, s):
    p = os.path.join(out, f"{arm}-{s}.json")
    if not os.path.exists(p) or os.path.getsize(p) == 0:
        return None, "absent or empty"
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
    return (best, None) if best else (None, "no line parsed as an object")

rows, no_tree = [], []
print(f"{'seed':<6}{'A_trees':<9}{'A_clean':<9}{'B_trees':<9}{'B_clean':<9}note")
for s in seeds:
    a, ae = load("A", s); b, be = load("B", s)
    if a is None or b is None:
        print(f"{s:<6}{'-':<9}{'-':<9}{'-':<9}{'-':<9}DROPPED: A={ae} B={be}")
        continue
    at, ac = a.get(TREES), a.get(CLEAN); bt, bc = b.get(TREES), b.get(CLEAN)
    note = ""
    if not at or not bt:
        note = "NO TREE — predicate could not run; excluded by name"
        no_tree.append(s)
    print(f"{s:<6}{str(at):<9}{str(ac):<9}{str(bt):<9}{str(bc):<9}{note}")
    if at and bt:
        rows.append((s, ac, bc))

print()
print(f"PRECONDITION: {len(rows)} seeds had a tree in BOTH arms; "
      f"{len(no_tree)} excluded for having none {no_tree if no_tree else ''}")
if not rows:
    print("VERDICT: VOID — no seed produced a tree in both arms. The predicate")
    print("         never ran, so nothing is learned about whether it can fail.")
    sys.exit(3)

base_true = [s for s, ac, bc in rows if ac]
went_red  = [s for s, ac, bc in rows if ac and not bc]
stayed    = [s for s, ac, bc in rows if ac and bc]
print(f"baseline true : {len(base_true)}/{len(rows)}")
print(f"planted false : {len(went_red)}/{len(base_true)}  (the falsifier firing)")
print()
if not base_true:
    print("VERDICT: UNEXPECTED — baseline was already false on every tree seed.")
    print("         Reconcile against the 41/41-true corpus before scoring the plant.")
elif len(went_red) == len(base_true):
    print("VERDICT: FALSIFIER LIVE — the check goes RED under the plant on every")
    print("         seed where it could run. It is a real assertion, not a tautology.")
elif went_red:
    print(f"VERDICT: MIXED — red on {went_red}, stayed true on {stayed}.")
    print("         Report both; do not collapse. A partly-firing falsifier needs")
    print("         its non-firing cases explained before it is trusted.")
else:
    print("VERDICT: VACUOUS — the plant removed the cancellation and the check")
    print("         STILL passed. It cannot detect the failure it names.")
PY
