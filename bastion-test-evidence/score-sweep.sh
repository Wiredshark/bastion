#!/usr/bin/env bash
# Score THE UNEXERCISED-FIELD SWEEP (FIELD-SWEEP-PREREG.md) from a driver log.
#
# S1 food_stock moves off zero, AND the matched control (wood, not in
#    FOOD_DEFS) leaves it at zero -- both directions inside ONE run, so the
#    discriminator is the item's FOOD-ness and not the run.
# S2 jobs_unreachable moves off zero -- or, if it does not while the server
#    emitted `job unreachable`, the flag's LIFETIME is the finding. Both
#    branches were registered in advance; this script reports which one.
#
# VOID BEFORE RED: a missing log, or a log with no COLONY samples at all,
# is not a failed bar -- it is an absent measurement, and the two must never
# render identically.
#
# usage: score-sweep.sh <driver-log> [server-log]
set -u
LOG="${1:?usage: score-sweep.sh <driver-log> [server-log]}"
SRV="${2:-}"
[ -f "$LOG" ] || { echo "MISSING DRIVER LOG $LOG -> VOID"; exit 2; }

# BOUND THE POPULATION TO ONE RUN. `bastion_playtest` APPENDS, so a re-run
# tag's log holds every previous leg -- driver-pit-shaft.log reached FIVE,
# and a disposition reported "1 of 200" for five legs of 40 across four
# binaries (DRIVER-LOG-APPEND-AMENDMENT.md). A count is only a measurement
# if its population is bounded; here the bound is a run.
#
# `last-run.sh` REFUSES a headerless file rather than assuming one run, so
# this inherits that refusal instead of silently scoring an unrecognised
# shape.
RUNS=$(bash "$(dirname "$0")/last-run.sh" "$LOG" --count) || exit $?
if [ "$RUNS" -gt 1 ]; then
  echo "note: $LOG holds $RUNS runs -- scoring the LAST one only"
fi
CLEAN=$(bash "$(dirname "$0")/last-run.sh" "$LOG" | sed 's/\x1b\[[0-9;]*m//g')
COLONY=$(printf '%s\n' "$CLEAN" | grep 'COLONY ' || true)
N=$(printf '%s' "$COLONY" | grep -c . || true)
echo "COLONY samples: $N"
[ "$N" -gt 0 ] || { echo "NO COLONY SAMPLES -> VOID (the driver never inspected the colony)"; exit 2; }

# The samples in order, with their food_stock / jobs_unreachable.
echo "sample  food_stock  jobs_unreachable"
i=0
printf '%s\n' "$COLONY" | while read -r line; do
  f=$(printf '%s' "$line" | grep -o 'food_stock=[0-9]*' | cut -d= -f2)
  u=$(printf '%s' "$line" | grep -o 'jobs_unreachable=[0-9]*' | cut -d= -f2)
  printf "  %-5s %-11s %s\n" "$i" "${f:-ABSENT}" "${u:-ABSENT}"
  i=$((i+1))
done

FMAX=$(printf '%s\n' "$COLONY" | grep -o 'food_stock=[0-9]*' | cut -d= -f2 | sort -n | tail -1)
UMAX=$(printf '%s\n' "$COLONY" | grep -o 'jobs_unreachable=[0-9]*' | cut -d= -f2 | sort -n | tail -1)
# Sample 1 is the WOOD-ONLY control (see script-sweep.txt's ordering: the
# control drop is measured BEFORE any food exists, because once food is in
# the pile no later sample can show what the wood alone did).
FCTRL=$(printf '%s\n' "$COLONY" | sed -n '2p' | grep -o 'food_stock=[0-9]*' | cut -d= -f2)
echo
echo "S1 control (sample 1, wood only): food_stock=${FCTRL:-ABSENT}  (must be 0)"
echo "S1 max food_stock across run    : ${FMAX:-ABSENT}  (must be > 0)"
echo "S2 max jobs_unreachable         : ${UMAX:-ABSENT}"

RC=0
if [ "${FCTRL:-x}" = "0" ]; then echo "S1 control PASS"; else echo "S1 control FAIL (wood moved food_stock, or sample absent)"; RC=1; fi
if [ "${FMAX:-0}" -gt 0 ] 2>/dev/null; then echo "S1 treatment PASS"; else echo "S1 treatment FAIL (food never registered)"; RC=1; fi

if [ "${UMAX:-0}" -gt 0 ] 2>/dev/null; then
  echo "S2 PASS -- the field moves; the debt was non-exercise, not a defect"
else
  # THE SECOND REGISTERED BRANCH. A zero here is only informative if the
  # server actually stranded a job in the same run -- otherwise the scenario
  # simply never produced one, which is a scenario limit, not a field defect.
  if [ -n "$SRV" ] && [ -f "$SRV" ]; then
    EM=$(sed 's/\x1b\[[0-9;]*m//g' "$SRV" | grep -c 'job unreachable' || true)
    echo "S2 zero, and the server emitted 'job unreachable' $EM times in this run"
    if [ "$EM" -gt 0 ]; then
      echo "S2 FINDING (registered branch 2): the flag FIRES but does not SURVIVE to sampling"
    else
      echo "S2 SCENARIO LIMIT: no job was ever stranded -- the field is untested, not defective"
    fi
  else
    echo "S2 zero, and no server log was given -> cannot distinguish defect from scenario limit"
    RC=2
  fi
fi
exit $RC
