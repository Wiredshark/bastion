#!/bin/sh
# vm-town-leg-remote.sh — ONE town leg on a VM (testing framework rule 9: THE
# VM FLEET RUNS THE LEGS). Runs FROM the checkout the pool reset, so this
# script always matches the code it measures.
#
# Usage (invoked over ssh by vm-town-legs.sh):
#   bash bastion-test-evidence/vm-town-leg-remote.sh <slot> <fence_ticks> "<arm_env>"
#
# Boots one deterministic town leg via play-harness.sh (PLAY_* roots pointed
# at this checkout), waits for the tick fence, emits ONE judge line in the
# exact field set the local judges use, then stops the slot. Logs stay on the
# VM (they die with it); the judge line is the deliverable, streamed to
# stdout — the same discipline as the pool's @@@SEED blocks.
set -u
SLOT="${1:?slot}"; FENCE="${2:?fence ticks}"; ARM_ENV="${3:-}"; SCENARIO="${4:-town}"
WT="$(cd "$(dirname "$0")/.." && pwd)"
EV="$WT/bastion-test-evidence"
strip() { sed 's/\x1b\[[0-9;]*m//g'; }

cd "$EV"
export PLAY_WT="$WT" PLAY_B="$WT/target/no_overflow" PLAY_EV="$EV" PLAY_EXE=""
PLAY_EXTRA_ENV="BASTION_UNCAPPED_TPS=1 BASTION_PATH_ENDPOINT_DIAG=1 $ARM_ENV" \
  bash play-harness.sh boot "$SLOT" "$SCENARIO" || { echo "LEG_BOOT_FAIL slot=$SLOT"; exit 3; }

L="$EV/play/server-$SLOT.log"
# Wait for the fence; a leg that stops writing for 5 minutes is DEAD, not slow
# (no terminator + no error = VOID, and this says which).
last_size=0; stall=0
while :; do
  t=$(strip < "$L" | grep "ITEM 39 tick cost" | tail -1 | grep -o "tick=[0-9]*" | cut -d= -f2)
  [ -n "${t:-}" ] && [ "$t" -ge "$FENCE" ] && break
  sz=$(stat -c %s "$L" 2>/dev/null || echo 0)
  if [ "$sz" = "$last_size" ]; then stall=$((stall+1)); else stall=0; last_size=$sz; fi
  [ "$stall" -ge 20 ] && { echo "LEG_DEAD slot=$SLOT tick=${t:-none} (log static 5m)"; exit 4; }
  sleep 15
done

V=/tmp/judge-$SLOT.txt; strip < "$L" > "$V"
FL=$(grep -n "ITEM 39 tick cost" "$V" | awk -F'tick=' -v f="$FENCE" '{split($2,a," "); if (a[1]+0>=f) {split($0,b,":"); print b[1]; exit}}')
[ -z "$FL" ] && FL=$(wc -l < "$V")
head -n "$FL" "$V" > "$V.w"
lt=$(grep -c "LONGEST-TIER SEARCH" "$V.w"); ex=$(grep -c "LONGEST-EXHAUST" "$V.w")
term=$(grep "CONN-SHADOW census" "$V.w" | tail -1 | grep -o "chaser_terminal_releases=[0-9]*" | cut -d= -f2)
stuck=$(grep -c "STUCK CENSUS" "$V.w")
sleepers=$(grep "bastion: slept" "$V.w" | grep -o "uid=[0-9]*" | sort -u | wc -l)
slept=$(grep -c "bastion: slept" "$V.w"); ate=$(grep -c "bastion: ate" "$V.w")
unreach=$(grep -c "job unreachable" "$V.w"); arrived=$(grep -c "colonist arrived at job site" "$V.w")
top3=$(grep "LONGEST-TIER SEARCH" "$V.w" | grep -oE "resolved_end=Vec3 \{ x: [0-9-]+, y: [0-9-]+, z: [0-9-]+" | sort | uniq -c | sort -rn | head -3 | awk '{s+=$1} END {print s+0}')
# ITEM 36 field block (df2c19b5a0's bars, greppable halves): the death roll,
# the outright split, the belongings drop + item count, the final
# population/downed census, and the extinction sentinel. A null-control leg
# must show ZEROES here with a couldn't-happen witness (the census being
# present at all = colonists existed and were sampled).
died=$(grep -c "COLONIST DIED" "$V.w")
outright=$(grep "DEATH v2 roll" "$V.w" | grep -c "outright=true")
bel_drops=$(grep -c "belongings dropped at the death cell" "$V.w")
bel_items=$(grep "belongings dropped at the death cell" "$V.w" | grep -o "dropped=[0-9]*" | cut -d= -f2 | awk '{s+=$1} END {print s+0}')
final_census=$(grep "EXPERIENCE census" "$V.w" | tail -1 | grep -oE "total=[0-9]+ downed=[0-9]+" | tr ' ' '_')
extinct=$(grep -c "COLONY EXTINCT" "$V.w")
# RESCUE chain (slot-93 follow-up): plant -> posted -> rescued, plus the
# census tail so recovery is visible.
downed_plant=$(grep -c "DOWNED PLANT" "$V.w")
rescue_posted=$(grep -c "RESCUE posted" "$V.w")
rescued=$(grep -c "RESCUED — helped" "$V.w")
# ALARM v1 (the cry, the response, the all-clear) — a raid leg should show
# raised>0; a BASTION_NO_ALARM or hostile-free leg must show clean zeroes.
alarm_raised=$(grep -c "ALARM RAISED" "$V.w")
shelters=$(grep -c "civilian takes shelter" "$V.w")
shelter_released=$(grep -c "shelter released" "$V.w")
alarm_over=$(grep -c "ALARM over" "$V.w")
# EVENING LIFE (looking-sweep rows): the lounge trigger, arrivals AT the
# gathering ring (distinct seats = the anti-stack pin, judged as distinct
# arrival cells), completed breaks. gather_seats counts DISTINCT Recreate
# arrival positions — 1 means stacking, ≈colony size means a real circle.
lounges=$(grep -c "leisure lounge" "$V.w")
lounge_arrivals=$(grep "arrived at job site" "$V.w" | grep -c "Recreate")
gather_seats=$(grep "arrived at job site" "$V.w" | grep "Recreate" | grep -oE "pos=Vec3 \{ x: [0-9-]+, y: [0-9-]+" | sort -u | wc -l)
break_over=$(grep -c "break over" "$V.w")
# REVIVAL INSTRUMENT: every HelpDownedEvent names its helper — nonzero
# helped with zero bastion RESCUED lines = vanilla agents revive organically
# (the mystery named); helper=None would be a helperless refresh.
helped=$(grep -c "HelpDownedEvent fired" "$V.w")
helped_by_none=$(grep "HelpDownedEvent fired" "$V.w" | grep -c "helper=None")
# SLEEP TIMES, v2 (stay-down made completions land at DAWN by design, so
# completion-time no longer measures lateness): bracket each RestAt ARRIVAL
# (bed entry) by the last-seen tick and count entries landing in the
# 02:00-06:00 window of any day (tick mod 54000 in [38250,49500]). Bar:
# near zero -- people reach their beds in the evening, not the small hours.
late_bed_entries=$(awk '/ITEM 39 tick cost/ {if (match($0, /tick=[0-9]+/)) t=substr($0, RSTART+5, RLENGTH-5)+0} /arrived at job site/ && /RestAt/ {d=t%54000; if (d>=38250 && d<=49500) n++} END {print n+0}' "$V.w")
echo "@@@LEG slot=$SLOT arm='$ARM_ENV' fence=$FENCE longest_search=$lt top3_share=$top3 exhaust=$ex terminal_releases=${term:-NA} stuck=$stuck sleepers=$sleepers slept=$slept ate=$ate unreachable=$unreach arrived=$arrived died=$died outright=$outright belongings_drops=$bel_drops belongings_items=$bel_items final=${final_census:-NA} extinct=$extinct downed_plant=$downed_plant rescue_posted=$rescue_posted rescued=$rescued alarm_raised=$alarm_raised shelters=$shelters shelter_released=$shelter_released alarm_over=$alarm_over lounges=$lounges lounge_arrivals=$lounge_arrivals gather_seats=$gather_seats break_over=$break_over helped=$helped helped_by_none=$helped_by_none late_bed_entries=$late_bed_entries@@@"
# RECREATE AUTOPSY (evening regression): the first three posted lounge
# seats' full lifecycles — the posted line, then every subsequent line
# naming that job id (claims, steers, releases with site numbers, arrival
# or its absence). Needs BASTION_RELEASE_DIAG=1 in the arm env to carry
# the per-job release lines.
echo "@@@RECREATE-AUTOPSY slot=$SLOT"
for J in $(grep "RECREATE posted (lounge seat)" "$V.w" | grep -o "job=[0-9]*" | cut -d= -f2 | head -3); do
  echo "--- job $J:"
  grep -E "job=$J[^0-9]|job=Some\($J\)" "$V.w" | tail -12 | sed 's/^.*bastion/bastion/' | cut -c1-200
done
echo "@@@END-AUTOPSY"
# The chronicle half (Death records + actors incl. witnesses) via the real
# wire path — one driver turn against the still-live world. Failure to turn
# is reported, never silent: the chronicle bar reads VOID for that leg.
printf 'inspect_chronicle\ninspect_colonists\n' > /tmp/i36-$SLOT.play
if bash play-harness.sh turn "$SLOT" /tmp/i36-$SLOT.play > /tmp/i36-$SLOT.out 2>&1; then
  echo "@@@CHRONICLE slot=$SLOT"
  grep -iE "death|chronicle|actors" /tmp/i36-$SLOT.out | head -40
  echo "@@@END"
else
  echo "@@@CHRONICLE slot=$SLOT TURN_FAILED (chronicle bar VOID for this leg)@@@"
  tail -5 /tmp/i36-$SLOT.out
fi
bash play-harness.sh stop "$SLOT" >/dev/null 2>&1
rm -f "$V" "$V.w"
