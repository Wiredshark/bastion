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
echo "adoption witness: $(echo "$W" | grep -oE 'town_origin=[^ ]+ plots=[0-9]+ farm_jobs=[0-9]+ bed_jobs=[0-9]+ stockpile_jobs=[0-9]+')"
farms=$(echo "$W" | grep -oE 'farm_jobs=[0-9]+' | cut -d= -f2)
beds=$(echo "$W"  | grep -oE 'bed_jobs=[0-9]+' | cut -d= -f2)
stocks=$(echo "$W" | grep -oE 'stockpile_jobs=[0-9]+' | cut -d= -f2)
echo
echo "BAR 1 (each mapped kind lands):"
bar1=pass
for pair in "farms=$farms" "beds=$beds" "stockpiles=$stocks"; do
  n=${pair#*=}
  if [ "${n:-0}" -eq 0 ]; then echo "  MISSING KIND: ${pair%%=*} = 0"; bar1=void; fi
done
[ "$bar1" = pass ] && echo "  all three kinds > 0 — PASS" || echo "  VOID naming the missing kind (per prereg: a town without that structure is a fixture fact)"
echo
echo "BAR 2 (survival on adopted infrastructure, existing emits only):"
printf "  eats     : %s\n" "$(grep -c 'bastion: ate\|EatFrom\|bastion: eat' "$C")"
printf "  arrivals : %s\n" "$(grep -c 'arrived at job site' "$C")"
printf "  completions: %s\n" "$(grep -c 'bastion: job completed' "$C")"
echo "  (bar 2 scores PASS when all three are >0 inside the survival window)"
rm -f "$C"
