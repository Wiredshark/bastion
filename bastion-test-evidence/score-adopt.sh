#!/bin/sh
# score-adopt.sh — adopt-a-town mode A, bars 1+2 (ADOPT-A-TOWN-PREREGISTRATION).
# Usage: score-adopt.sh <server-log>
set -u
LOG="${1:?usage: score-adopt.sh <server-log>}"
C=$(mktemp); sed 's/\x1b\[[0-9;]*m//g' "$LOG" > "$C"

if grep -q "ADOPT-A-TOWN VOID" "$C"; then
  echo "VOID (registered branch): no adoptable site in radius — a worldgen fact."
  grep -m1 "ADOPT-A-TOWN VOID" "$C" | cut -c1-160
  rm -f "$C"; exit 3
fi
W=$(grep -m1 "ADOPT-A-TOWN founded into an existing settlement" "$C")
if [ -z "$W" ]; then
  echo "VOID: neither the adoption witness nor the VOID witness appeared —"
  echo "      the flag likely never reached the server (check BASTION_ENV)."
  rm -f "$C"; exit 3
fi
# 2026-08-20 rework: placement is DEFERRED onto pending_restore (the first
# leg placed 0 jobs of every kind racing unloaded terrain). The witness now
# reports QUEUED counts; the DRAIN's own emit ("colony orders replayed") says
# what actually landed once terrain streamed in.
echo "adoption witness: $(echo "$W" | grep -oE 'town_origin=[^ ]+ plots=[0-9]+ queued=[0-9]+')"
queued=$(echo "$W" | grep -oE 'queued=[0-9]+' | cut -d= -f2)
replayed=$(grep "surface designations placed" "$C" | grep -oE 'placed=[0-9]+' | cut -d= -f2 | awk '{s+=$1} END {print s+0}')
# waiting diagnosis, if the drain never fired
grep -m2 "surface queue WAITING" "$C" | cut -c1-160
# fallback field-name form
[ "$replayed" -eq 0 ] && replayed=$(grep "colony orders replayed" "$C" | grep -oE '[0-9]+' | head -1)
echo "queued=$queued  drained(replayed)=$replayed"
echo
echo "BAR 1 (adoption lands as real designations once terrain loads):"
if [ "${queued:-0}" -eq 0 ]; then
  echo "  VOID: nothing queued — the mapper found no usable plots."
elif [ "${replayed:-0}" -eq 0 ]; then
  echo "  FAIL/VOID: $queued queued, NOTHING drained — terrain never loaded the"
  echo "  regions inside the run window, or the drain rejected them. Check"
  echo "  'still_waiting' in the log to tell which."
  grep -m2 "colony orders replayed\|still_waiting" "$C" | cut -c1-140
else
  echo "  PASS: $queued queued, $replayed placed by the drain."
fi
echo
echo "BAR 2 (survival on adopted infrastructure, existing emits only):"
printf "  eats     : %s\n" "$(grep -c 'bastion: ate\|EatFrom\|bastion: eat' "$C")"
printf "  arrivals : %s\n" "$(grep -c 'arrived at job site' "$C")"
printf "  completions: %s\n" "$(grep -c 'bastion: job completed' "$C")"
echo "  (bar 2 scores PASS when all three are >0 inside the survival window)"
rm -f "$C"
