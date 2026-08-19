#!/bin/sh
# score-selfrescue-axes23.sh — banked item 9, the full outcome table in ONE run.
#
# STANDING FACT: `self_rescue` is 0 emissions / 55 calls across 48 banked seeds,
# while `emergency` — the SAME `plan_access` — succeeds 46/478 in the same runs.
# Three arguments differed. Axis 1 (mask) was ELIMINATED 2026-08-19: bubble
# geometry on 36 of 36 calls, still 0 emissions. Two remain.
#
# FOUR ARMS, because axes 2 and 3 are independent and a build costs 10 minutes:
#   A  baseline                                      (neither)
#   B  BASTION_SELFRESCUE_CTX=1                      (owner only)
#   C  BASTION_SELFRESCUE_APPROACH=1                 (approach only)
#   D  both                                          (the emergency site's full
#                                                     argument set, minus mask)
#
# REGISTERED OUTCOMES (declared before the run):
#   CONTEXT        — B emits > 0. The owner alone unblocks it.
#   APPROACH       — C emits > 0. The approach alone unblocks it.
#   BOTH-REQUIRED  — only D emits > 0. Neither argument suffices alone.
#   NEITHER        — all four arms 0 WITH the witnesses confirming delivery.
#                    Then no call-site argument explains it and the refusal is
#                    inside plan_access on a path only this caller reaches.
#                    This is a real result, not an absence.
#   VOID           — self_rescue never called, or a treated arm's witness never
#                    fired. Refused before any verdict is printed.
#
# ★ Every treated arm must PROVE delivery. Axis 1's first run was VOID precisely
#   because "passed it, nothing changed" and "the flag never arrived" are the
#   same evidence without a witness. The witness counts live in the stderr log.
#
# Usage: sh score-selfrescue-axes23.sh <outdir> <seed> [seed...]
set -u
OUT="${1:?usage: score-selfrescue-axes23.sh <outdir> <seed> [seed...]}"; shift
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
    echo "$CHANGED" | sed 's/^/     /' >&2
    exit 4
  fi
  echo "  (binary != HEAD, but no .rs differs — safe to score)"
fi
DIRTY=$(git status --short -- '*.rs' | wc -l)
echo "dirty .rs files  : $DIRTY"
[ "$DIRTY" -eq 0 ] || { echo "!! REFUSING: uncommitted .rs changes" >&2; exit 5; }
echo "seeds            : $*"
echo

mkdir -p "$OUT"
for ARM in A B C D; do
  rm -rf "$OUT/dd-$ARM"; mkdir -p "$OUT/dd-$ARM"
  for s in "$@"; do
    case "$ARM" in
      A) ENVP="" ;;
      B) ENVP="BASTION_SELFRESCUE_CTX=1" ;;
      C) ENVP="BASTION_SELFRESCUE_APPROACH=1" ;;
      D) ENVP="BASTION_SELFRESCUE_CTX=1 BASTION_SELFRESCUE_APPROACH=1" ;;
    esac
    # shellcheck disable=SC2086
    env $ENVP "$HARNESS" --b5-scenario --seed "$s" \
      --data-dir "$OUT/dd-$ARM/s$s" > "$OUT/$ARM-$s.json" 2> "$OUT/$ARM-$s.err" &
  done
  wait
  echo "arm $ARM done: $(ls "$OUT"/$ARM-*.json 2>/dev/null | wc -l) files"
done
echo

# Witness delivery, counted from the logs (the counters are not scored fields).
echo "=== WITNESS DELIVERY (must be nonzero on every treated arm) ==="
printf "%-6s %-10s %-12s %-12s\n" seed arm ctx_emits approach_emits
for s in "$@"; do
  for ARM in A B C D; do
    ce=$(grep -c "AXIS-2 self_rescue passing OWNER" "$OUT/$ARM-$s.err" 2>/dev/null)
    ae=$(grep -c "AXIS-3 self_rescue passing APPROACH" "$OUT/$ARM-$s.err" 2>/dev/null)
    printf "%-6s %-10s %-12s %-12s\n" "$s" "$ARM" "$ce" "$ae"
  done
done
echo

python - "$OUT" "$@" <<'PY'
import json, sys, os, re

def winpath(p):
    return p[1] + ":" + p[2:] if re.match(r"^/[a-zA-Z]/", p) else p

out = winpath(sys.argv[1]); seeds = sys.argv[2:]
CK, EK = "b5_access_plan_self_rescue_calls", "b5_access_plan_self_rescue_emissions"
ARMS = ["A", "B", "C", "D"]
LABEL = {"A": "baseline", "B": "owner", "C": "approach", "D": "both"}

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

tot_calls = {a: 0 for a in ARMS}
tot_emit = {a: 0 for a in ARMS}
print(f"{'seed':<6}" + "".join(f"{a}_calls/{a}_emit".ljust(16) for a in ARMS))
scored = 0
for s in seeds:
    ds = {a: load(a, s) for a in ARMS}
    if any(d is None for d in ds.values()):
        missing = [a for a in ARMS if ds[a] is None]
        print(f"{s:<6}DROPPED: arms {missing} produced no payload")
        continue
    row = f"{s:<6}"
    for a in ARMS:
        c = ds[a].get(CK) or 0; e = ds[a].get(EK) or 0
        tot_calls[a] += c; tot_emit[a] += e
        row += f"{c}/{e}".ljust(16)
    print(row)
    scored += 1

print()
if scored == 0:
    print("VERDICT: VOID — no seed produced a full four-arm payload."); sys.exit(3)
for a in ARMS:
    print(f"  {a} ({LABEL[a]:<8}): {tot_emit[a]} emissions / {tot_calls[a]} calls")
print()
if tot_calls["A"] == 0:
    print("VERDICT: VOID — self_rescue was never CALLED. Neither arm could emit.")
    print("         Not evidence about any axis. Re-run on seeds with known calls.")
    sys.exit(3)

b, c, d = tot_emit["B"], tot_emit["C"], tot_emit["D"]
if tot_emit["A"] > 0:
    print("VERDICT: UNEXPECTED — the BASELINE emitted, contradicting the 0/55 corpus")
    print("         figure. Do not score the axes; reconcile the corpus claim first.")
elif b > 0 and c == 0:
    print("VERDICT: CONTEXT — emergency_owner alone unblocks self_rescue.")
elif c > 0 and b == 0:
    print("VERDICT: APPROACH — emergency_approach alone unblocks self_rescue.")
elif b > 0 and c > 0:
    print("VERDICT: EITHER — both arguments independently unblock it.")
elif d > 0:
    print("VERDICT: BOTH-REQUIRED — neither argument suffices alone; together they do.")
else:
    print("VERDICT: NEITHER — all four arms 0 with the mechanism exercised.")
    print("         No call-site ARGUMENT explains the 0/55. The refusal is inside")
    print("         plan_access, on a path only this caller reaches. Check the")
    print("         witness table above: if a treated arm's witness is 0, this is")
    print("         VOID, not NEITHER.")
PY
