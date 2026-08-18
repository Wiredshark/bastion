#!/usr/bin/env bash
# Score the NEXT-TICK READBACK (READBACK-PREREG.md) from a server log.
#
# R2: readback_tick == completed_tick + 1 on EVERY sample.
# R3: >=1 sample with lands=true (the instrument has seen a write land).
#
# VOID before RED: a missing log and a log with zero readback lines are
# different facts and neither is a failure of the bars -- R1's OFF control
# *expects* zero lines. The caller says which mode it is scoring.
#
# usage: score-readback.sh <server-log> <expect: on|off>
set -u
LOG="${1:?usage: score-readback.sh <server-log> <on|off>}"
MODE="${2:?expect on|off}"
if [ ! -f "$LOG" ]; then
  echo "MISSING LOG $LOG -> VOID"; exit 2
fi
CLEAN=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG" | grep 'mine readback' || true)
N=$(printf '%s' "$CLEAN" | grep -c . || true)

if [ "$MODE" = off ]; then
  echo "readback lines : $N (OFF control -- must be 0)"
  [ "$N" -eq 0 ] && { echo "R1-off PASS"; exit 0; } || { echo "R1-off FAIL"; exit 1; }
fi

echo "readback lines : $N"
if [ "$N" -eq 0 ]; then
  echo "ZERO SAMPLES with flag ON -> R1 FAIL (or the scenario completed no Mine)"; exit 1
fi

# R2 -- both ticks are on every line by construction; any line where they do
# not differ by exactly 1 is a violation, and a line MISSING either field is
# counted as a violation too (a sample that cannot prove its timing is not
# a passing sample).
BAD_TICK=$(printf '%s\n' "$CLEAN" | awk '
  {
    ct = ""; rt = "";
    if (match($0, /completed_tick=[0-9]+/)) ct = substr($0, RSTART+15, RLENGTH-15) + 0; else { bad++; next }
    if (match($0, /readback_tick=[0-9]+/))  rt = substr($0, RSTART+14, RLENGTH-14) + 0; else { bad++; next }
    if (rt != ct + 1) bad++;
  }
  END { print bad + 0 }')
echo "R2 violations  : $BAD_TICK of $N (readback_tick != completed_tick+1)"

LANDS_T=$(printf '%s\n' "$CLEAN" | grep -c 'lands=true' || true)
LANDS_F=$(printf '%s\n' "$CLEAN" | grep -c 'lands=false' || true)
echo "R3 lands       : true=$LANDS_T false=$LANDS_F of $N"

RC=0
[ "$BAD_TICK" -eq 0 ] || { echo "R2 FAIL"; RC=1; }
[ "$LANDS_T" -ge 1 ] || { echo "R3 FAIL -- the instrument never saw a write land"; RC=1; }
[ "$RC" -eq 0 ] && echo "R2 PASS · R3 PASS"
exit $RC
