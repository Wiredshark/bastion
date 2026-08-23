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
SLOT="${1:?slot}"; FENCE="${2:?fence ticks}"; ARM_ENV="${3:-}"
WT="$(cd "$(dirname "$0")/.." && pwd)"
EV="$WT/bastion-test-evidence"
strip() { sed 's/\x1b\[[0-9;]*m//g'; }

cd "$EV"
export PLAY_WT="$WT" PLAY_B="$WT/target/no_overflow" PLAY_EV="$EV" PLAY_EXE=""
PLAY_EXTRA_ENV="BASTION_UNCAPPED_TPS=1 BASTION_PATH_ENDPOINT_DIAG=1 $ARM_ENV" \
  bash play-harness.sh boot "$SLOT" town || { echo "LEG_BOOT_FAIL slot=$SLOT"; exit 3; }

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
echo "@@@LEG slot=$SLOT arm='$ARM_ENV' fence=$FENCE longest_search=$lt top3_share=$top3 exhaust=$ex terminal_releases=${term:-NA} stuck=$stuck sleepers=$sleepers slept=$slept ate=$ate unreachable=$unreach arrived=$arrived@@@"
bash play-harness.sh stop "$SLOT" >/dev/null 2>&1
rm -f "$V" "$V.w"
