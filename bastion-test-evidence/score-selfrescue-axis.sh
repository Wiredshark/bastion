#!/bin/sh
# score-selfrescue-axis.sh — banked item 8: isolate WHICH call-site axis makes
# `self_rescue` a 0%-success entry point to `plan_access`.
#
# CORPUS FACT BEING TESTED (SELF-RESCUE-NEVER-SUCCEEDS.md, 48 seeds):
#     self_rescue        calls=55   emissions=0    0%
#     emergency          calls=478  emissions=46   9.6%
# Same function, three differing arguments. This script varies exactly ONE —
# the mask — via BASTION_SELFRESCUE_BUBBLE, and scores the existing counter
# pair, which was verified equal to the log emit on 48/48 seeds.
#
# REGISTERED OUTCOMES (declared BEFORE the run):
#   MASK      — B emissions > 0 while A stays 0. The claim mask is the blocker.
#   NEITHER   — both arms 0. The mask is EXONERATED; remaining candidates are
#               emergency_owner / emergency_approach, or a defect inside
#               plan_access itself. This outcome is a real result, not a null:
#               it kills the leading hypothesis.
#   VOID      — A's self_rescue_calls == 0 on every seed, i.e. the mechanism was
#               never exercised and neither arm could have emitted. Checked and
#               REFUSED before any verdict is printed.
#
# ★ PRECONDITION IS PRINTED ABOVE THE RESULT. A run where self_rescue is never
#   CALLED and a run where it is called and refuses render identically in the
#   emissions column (both 0). Seeds are therefore chosen for KNOWN-NONZERO
#   calls, and the calls column is printed beside every verdict.
#
# Protections inherited from aa-pair.sh, for the same hard-won reasons:
#   - each run gets its OWN data dir (rtsim saves every 60s; a shared dir makes
#     the second run a RESTART, not an independent run)
#   - each run is its OWN process (hashbrown's iteration seed is per-process)
#
# Usage: sh score-selfrescue-axis.sh <outdir> <seed> [seed...]
#   Default seeds should be ones with known-nonzero self_rescue calls in
#   wave34: 37 (11 calls), 3 (7), 29 (6), 16 (6), 11 (6).
set -u
# ★ corpus-first, enforced for LOCAL runs too (see corpus-first.sh)
. "$(dirname "$0")/corpus-first.sh"
OUT="${1:?usage: score-selfrescue-axis.sh <outdir> <seed> [seed...]}"; shift
[ $# -ge 1 ] || { echo "REFUSED: no seeds given" >&2; exit 2; }

HARNESS=./target/verify/bastion-harness.exe
[ -x "$HARNESS" ] || { echo "REFUSED: no harness at $HARNESS" >&2; exit 2; }

# Attestation FIRST — the whole claim is about which code ran.
GH=$("$HARNESS" --print-git-hash 2>/dev/null)
RH=$(git rev-parse --short=10 HEAD)
echo "harness git-hash : $GH"
echo "repo HEAD        : $RH"
# ★★ REFUSE, do not warn. A warning that must be READ to be safe is not a
# safeguard — this one printed while a run was launched against a binary that
# predated the probe. Decide it here instead.
if [ "$GH" != "$RH" ]; then
  CHANGED=$(git diff --name-only "$GH" HEAD -- '*.rs' 2>/dev/null)
  if [ -n "$CHANGED" ]; then
    echo "!! REFUSING: binary is $GH, HEAD is $RH, and these .rs files differ:" >&2
    echo "$CHANGED" | sed 's/^/     /' >&2
    echo "   The probe may not be in the binary. Rebuild before scoring." >&2
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
echo "arms             : A=baseline (claim mask)   B=BASTION_SELFRESCUE_BUBBLE=1"
echo

mkdir -p "$OUT"
for ARM in A B; do
  rm -rf "$OUT/dd-$ARM"; mkdir -p "$OUT/dd-$ARM"
  for s in "$@"; do
    if [ "$ARM" = B ]; then
      BASTION_SELFRESCUE_BUBBLE=1 "$HARNESS" --b5-scenario --seed "$s" \
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
    """Git Bash hands out /c/Users/...; native Windows python cannot open that.

    Caught by this script's own dry-check: every seed came back
    'DROPPED: unreadable [Errno 2]' — a TOTAL failure that reads as 'no data'
    rather than 'the path form is wrong'. An earlier manual test had passed
    only because it used `cd` plus a relative filename, so the defect was
    invisible until the script was invoked the way it is actually used.
    """
    return p[1] + ":" + p[2:] if re.match(r"^/[a-zA-Z]/", p) else p

out = winpath(sys.argv[1]); seeds = sys.argv[2:]
CK, EK = "b5_access_plan_self_rescue_calls", "b5_access_plan_self_rescue_emissions"

def load(arm, s):
    p = os.path.join(out, f"{arm}-{s}.json")
    try:
        txt = open(p, encoding="utf-8", errors="replace").read()
    except OSError as e:
        return None, f"unreadable: {e}"
    # ★ PARSE PER LINE, not with a spanning regex. The payload is ONE line
    # (~36 KB) and other lines around it contain brackets — "B5 FAILED CLAUSES:
    # [...]" among them. A greedy \{.*\} runs from the first brace to the last
    # bracket and yields "Extra data"; a non-greedy \{.*?\} truncates the first
    # nested object instead. Both were tried; both were wrong, in opposite
    # directions. Take the LAST line that independently parses as an object.
    best = None
    for line in txt.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except Exception:
            continue
        if isinstance(obj, dict):
            best = obj
    if best is None:
        return None, "no line parsed as a JSON object"
    return best, None

rows, void_seeds = [], []
print(f"{'seed':<6}{'A_calls':<9}{'A_emit':<8}{'B_calls':<9}{'B_emit':<8}note")
for s in seeds:
    a, ae = load("A", s); b, be = load("B", s)
    if a is None or b is None:
        # An excluded seed and an absent seed must never render identically.
        print(f"{s:<6}{'-':<9}{'-':<8}{'-':<9}{'-':<8}DROPPED: A={ae} B={be}")
        continue
    ac, aem = a.get(CK), a.get(EK); bc, bem = b.get(CK), b.get(EK)
    if ac is None or bc is None:
        print(f"{s:<6}{'-':<9}{'-':<8}{'-':<9}{'-':<8}DROPPED: counter field absent")
        continue
    note = ""
    if ac == 0 and bc == 0:
        note = "VOID-seed: self_rescue never CALLED"; void_seeds.append(s)
    print(f"{s:<6}{ac:<9}{aem:<8}{bc:<9}{bem:<8}{note}")
    rows.append((s, ac, aem, bc, bem))

print()
if not rows:
    print("VERDICT: VOID — no seed produced scoreable output."); sys.exit(3)

exercised = [r for r in rows if r[1] > 0 or r[3] > 0]
print(f"PRECONDITION: {len(exercised)} of {len(rows)} seeds actually CALLED self_rescue.")
if not exercised:
    print("VERDICT: VOID — the mechanism was never exercised. Neither arm COULD emit.")
    print("         This is not evidence about the mask. Re-run on seeds with known calls.")
    sys.exit(3)

Aem = sum(r[2] for r in exercised); Bem = sum(r[4] for r in exercised)
Ac  = sum(r[1] for r in exercised); Bc  = sum(r[3] for r in exercised)
print(f"A (claim mask) : {Aem} emissions / {Ac} calls")
print(f"B (bubble mask): {Bem} emissions / {Bc} calls")
print()
if Bem > 0 and Aem == 0:
    print("VERDICT: MASK — the claim mask is the blocker. Bubble mask emits, claim mask does not.")
elif Bem == 0 and Aem == 0:
    print("VERDICT: NEITHER — the mask is EXONERATED. Both arms 0 with the mechanism exercised.")
    print("         Kills the leading hypothesis. Remaining: emergency_owner /")
    print("         emergency_approach, or a defect inside plan_access itself.")
elif Aem > 0:
    print("VERDICT: UNEXPECTED — baseline emitted, contradicting the 0/55 corpus figure.")
    print("         Do NOT score the axis. Reconcile the corpus claim first.")
else:
    print(f"VERDICT: MIXED — A={Aem} B={Bem}. Report both; do not collapse.")
PY
