#!/bin/sh
# score-haulrev.sh — ITEM 16 P2+P3: does the priority command BITE, and REVERSE?
#
# Windows are delimited by the command's own server-side witness:
#     "bastion: work priority set (live command)"  (priority=0 -> marker A,
#      priority=3 -> marker B)
# Deliveries counted per window from "bastion: haul delivered". In-window
# counting is the point: P2's earlier PASS was RETRACTED for counting over a
# log that kept growing after the driver left.
#
# BARS: baseline>0 (precondition; else VOID) · zero-window==0 (P2 bite) ·
# restored-window>0 (P3 reversibility).
set -u
LOG="${1:?usage: score-haulrev.sh <server-log>}"
CLEAN=$(mktemp); sed 's/\x1b\[[0-9;]*m//g' "$LOG" > "$CLEAN"

A=$(grep -n "work priority set (live command)" "$CLEAN" | sed -n 1p | cut -d: -f1)
B=$(grep -n "work priority set (live command)" "$CLEAN" | sed -n 2p | cut -d: -f1)
TOT=$(wc -l < "$CLEAN")
if [ -z "$A" ] || [ -z "$B" ]; then
  echo "VOID: expected 2 priority-witness lines, found $(grep -c 'work priority set' "$CLEAN")." >&2
  echo "      The command never reached the server; nothing here scores P2/P3." >&2
  rm -f "$CLEAN"; exit 3
fi
base=$(sed -n "1,${A}p" "$CLEAN"      | grep -c "bastion: haul delivered")
zero=$(sed -n "${A},${B}p" "$CLEAN"   | grep -c "bastion: haul delivered")
rest=$(sed -n "${B},${TOT}p" "$CLEAN" | grep -c "bastion: haul delivered")
echo "windows (lines): baseline 1..$A · zeroed $A..$B · restored $B..$TOT"
echo "haul deliveries : baseline=$base zeroed=$zero restored=$rest"
echo
if [ "$base" -eq 0 ]; then
  echo "VOID: the BASELINE hauled nothing — the precondition failed, so the"
  echo "      zero window proves nothing. (This is the 'control that hauls"
  echo "      reliably' blocker; if it recurs, that blocker is NOT dissolved.)"
  rm -f "$CLEAN"; exit 3
fi
P2=fail; P3=fail
[ "$zero" -eq 0 ] && P2=pass
[ "$rest" -gt 0 ] && P3=pass
echo "P2 (bite)         : $P2  (zeroed window must be 0)"
echo "P3 (reversibility): $P3  (restored window must be >0)"
if [ "$P2" = pass ] && [ "$P3" = pass ]; then
  echo "VERDICT: P2+P3 PASS — the priority bites and reverses, in-window."
elif [ "$P2" = pass ]; then
  echo "VERDICT: P2 PASS, P3 FAIL — it bites but does not come back. A one-way"
  echo "         lever is a trap, not a control."
else
  echo "VERDICT: P2 FAIL — hauling continued through priority 0 ($zero in-window)."
fi
rm -f "$CLEAN"
