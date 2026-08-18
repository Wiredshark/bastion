#!/usr/bin/env bash
# FAN PRE-FLIGHT — **does the scenario actually EMIT what the scorer will read?**
#
# ★★★ WHY THIS EXISTS, WITH THE BILL ATTACHED. Two fans on 2026-08-17 were VOID
# for the same reason, and both were answerable by ONE local run:
#
#   b55_deep   ~$0.80, 18 min  -> the rescue count is a TRACING emit
#                                 (`info!`), which vm-pool never captures.
#                                 Scored 0 across 96 seeds; had the registered
#                                 refusal not fired it would have published
#                                 "b55_deep performs zero rescues".
#   dig_access ~$1.82, 41 min  -> the scenario emits NO boolean clause fields
#                                 AT ALL, so no per-clause distribution could
#                                 ever have come back, complete run or not.
#   ------------------------------------------------------------------------
#   TOTAL      ~$2.62 and ~59 minutes of 96-vCPU time for two VOIDs.
#
# ★★ THE GAP WAS NOT CARELESSNESS ABOUT PROVENANCE — every fan already attests
# HEAD, dirty count, binary freshness and branch. It is that ALL of that
# describes the SUBJECT and NONE of it describes the MEASUREMENT. A run can be
# perfectly attested and still be incapable of answering the question asked.
#
# Usage: bash fan-preflight.sh --some-scenario field1 [field2 ...]
#   exit 0 = every field present   exit 4 = at least one ABSENT (do not fan)
set -u
WT=/e/veloren-master/.engine-integration-wt
FLAG="${1:?usage: fan-preflight.sh --scenario-flag FIELD [FIELD...]}"; shift
[ $# -gt 0 ] || { echo "!! name at least one field the scorer will read"; exit 2; }

# ★ Overridable because scenario runtimes differ by an order of magnitude:
# farm finishes inside ~4 min, dig_access exceeded 300s and produced 14151 lines
# without reaching its JSON. A fixed timeout would make slow scenarios
# permanently INCONCLUSIVE.
TIMEOUT_S="${PREFLIGHT_TIMEOUT:-300}"
BIN="$WT/target/verify/bastion-harness.exe"; [ -x "$BIN" ] || BIN="${BIN%.exe}"
[ -x "$BIN" ] || { echo "!! no harness binary at $BIN"; exit 3; }

echo "=== FAN PRE-FLIGHT: $FLAG ==="
echo "  binary : $(date -r "$BIN" '+%Y-%m-%d %H:%M:%S')"
echo "  fields : $*"
OUT=$(mktemp)
# ★★★ CD TO THE WORKTREE FIRST — ASSETS RESOLVE BY CWD, NOT BY BINARY LOCATION.
# The first version of this script ran from wherever it was invoked and the
# harness died in two lines, reporting every field ABSENT. ★ That is a FALSE
# NEGATIVE in the exact direction that matters: a pre-flight whose failure mode
# is "everything is missing" would veto every fan, and I would have deleted the
# tool instead of the bug. Caught only because the CONTROL (a scenario whose
# fields I had already measured at 16/96) came back ABSENT.
# ★★ Same lesson the run scripts already encode by setting VELOREN_ASSETS.
cd "$WT" || { echo "!! cannot cd to $WT"; exit 3; }
# ★ Short and single-seed on purpose: this asks "is the field PRESENT", never
# "what is its value", so a full-length run would buy nothing.
timeout "$TIMEOUT_S" "$BIN" "$FLAG" --seed 1 --ticks 600 --tps 30 --data-dir "/tmp/preflight-$$" > "$OUT" 2>&1
rc=$?
# ★ ANSI FIRST — tracing colours field names, and a zero-match grep is
# indistinguishable from a real absence. This exact bug cost a full triage.
TXT=$(sed -e 's/\x1b\[[0-9;]*m//g' "$OUT")

echo "  exit   : $rc  ($(echo "$TXT" | wc -l) lines)"

# ★★★★ A TRUNCATED RUN AND A MISSING FIELD ARE NOT THE SAME ANSWER, AND THIS
# TOOL SHIPPED CONFLATING THEM. `timeout` returns 124. Scenarios emit their JSON
# at the END, so a run killed mid-flight reports EVERY field ABSENT — which reads
# exactly like "this scenario has no such field" and would veto a fan that is
# perfectly fannable.
# ★ Caught on the first red case: dig_access ran 14151 lines and hit the 300s
# limit, and the tool said ABSENT with total confidence. ★★ The FINDING was still
# right (the 64 completed seeds of the real fan carried zero boolean fields) —
# but this tool had not EARNED it. **An instrument that cannot distinguish
# "absent" from "never got there" is the exact defect this file exists to catch,
# and I wrote it in on day one.**
if [ "$rc" -eq 124 ]; then
  echo "  !! TIMED OUT after ${TIMEOUT_S}s -- the scenario emits its JSON at the END,"
  echo "     so field presence is INCONCLUSIVE, not absent. Re-run with"
  echo "     PREFLIGHT_TIMEOUT=<secs> higher than this scenario's runtime."
  echo "=== INCONCLUSIVE (exit 5) -- NOT a refusal, and NOT a clearance. ==="
  rm -f "$OUT"; exit 5
fi
MISSING=0
for f in "$@"; do
  # Present as a JSON field? (what a fan CAN aggregate)
  if echo "$TXT" | grep -qE "\"$f\"[[:space:]]*:"; then
    echo "  ★ PRESENT (json)   $f"
  # Present only as a log line? (what a fan CANNOT see)
  elif echo "$TXT" | grep -q "$f"; then
    echo "  ⚠ LOG-ONLY         $f  -- appears in output but NOT as a JSON field."
    echo "                        vm-pool captures the scenario's JSON; a tracing"
    echo "                        emit will read as ZERO on every seed. DO NOT FAN."
    MISSING=$((MISSING+1))
  else
    echo "  ✗ ABSENT           $f"
    MISSING=$((MISSING+1))
  fi
done

if [ "$MISSING" -gt 0 ]; then
  echo "=== REFUSED: $MISSING of $# field(s) unusable. Fanning this would spend VM time on a VOID. ==="
  rm -f "$OUT"; exit 4
fi
echo "=== CLEAR TO FAN: every named field is a readable JSON field. ==="
rm -f "$OUT"
