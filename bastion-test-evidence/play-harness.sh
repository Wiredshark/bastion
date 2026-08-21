#!/bin/bash
# THE PLAY HARNESS (Ben direct, 2026-08-21: "use them to actually play the real
# game like they would play dwarf fortress ... and collect what's going wrong").
#
# The pit runner (run-pit.sh) is a BATCH instrument: one fixed script, one fresh
# world, one verdict. A player is not batch — a player looks, decides, acts,
# looks again, and changes their mind. This harness gives an agent that loop:
# ONE persistent world it can return to, and a turn command that runs whatever
# script the agent just wrote against it.
#
#   play-harness.sh boot  <slot> <arm>     start a persistent world (once)
#   play-harness.sh turn  <slot> <script>  play one turn against it
#   play-harness.sh watch <slot> [n]       tail the SERVER's own view
#   play-harness.sh stop  <slot>           end the session
#
# The world at slot N owns port 26024+N*10 and userdata-play-N; two agents on
# different slots never see each other. Userdata is NOT wiped between turns —
# that is the whole point: the colony the agent left behind is the colony it
# comes back to.
set -u
WT=/e/veloren-master/.item29-wt
B=$WT/target/no_overflow
EV=$WT/bastion-test-evidence
PLAY=$EV/play
mkdir -p "$PLAY"

CMD=${1:?usage: boot|turn|watch|stop}
SLOT=${2:?slot number}
GAME=$((26024 + SLOT * 10))
WEB=$((26025 + SLOT * 10))
METRICS=$((18026 + SLOT * 10))
UD=$WT/userdata-play-$SLOT
SRVLOG=$PLAY/server-$SLOT.log
PIDF=$PLAY/.pid-$SLOT

case "$CMD" in
boot)
  ARM=${3:-arena}
  if [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    echo "slot $SLOT already booted (pid $(cat "$PIDF"), port $GAME)"; exit 0
  fi
  rm -rf "$UD"
  VELOREN_USERDATA=$UD VELOREN_ASSETS=$WT/assets \
    "$B/veloren-server-cli.exe" --no-auth admin add player admin >/dev/null 2>&1
  S=$UD/server/server_config/settings.ron
  sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:$METRICS\"/g" "$S"
  sed "s/:14005\"/:$WEB\"/" "$UD/server-cli/settings.template.ron" > "$UD/server-cli/settings.ron"
  # The two worlds a player can be dropped into. The arena is legible and fast;
  # the town is the real test of "use what is already there".
  case "$ARM" in
    town) ENVV="BASTION_ADOPT_TOWN=1 BASTION_AUTOFOUND_REAL_TERRAIN=1 BASTION_COLONY_PRESENCE_VD=3 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=32 BASTION_SEED_MATERIALS=64" ;;
    *)    ENVV="BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=32" ;;
  esac
  # NOT uncapped: a player watches at human speed, and an uncapped server makes
  # every observation a blur of thousands of ticks.
  ( cd "$WT" && VELOREN_USERDATA=$UD VELOREN_ASSETS=$WT/assets \
      env BASTION_DETERMINISTIC=1 $ENVV \
      "$B/veloren-server-cli.exe" --no-auth > "$SRVLOG" 2>&1 ) &
  echo $! > "$PIDF"
  echo "booting slot $SLOT ($ARM) on port $GAME, pid $(cat "$PIDF")"
  for i in $(seq 1 90); do
    grep -q "ready to accept connections" "$SRVLOG" 2>/dev/null && { echo "READY after ${i}0s"; exit 0; }
    sleep 10
  done
  echo "NOT READY after 900s — read $SRVLOG"; exit 1
  ;;
turn)
  SCRIPT=${3:?script path}
  [ -f "$SCRIPT" ] || { echo "no such script: $SCRIPT"; exit 2; }
  TURN=$(( $(ls "$PLAY"/turn-$SLOT-*.log 2>/dev/null | wc -l) + 1 ))
  OUT=$PLAY/turn-$SLOT-$TURN.log
  ( cd "$WT" && VELOREN_ASSETS=$WT/assets \
      "$B/bastion_playtest.exe" "localhost:$GAME" player "$SCRIPT" "$OUT" \
      > "$PLAY/turnout-$SLOT-$TURN.log" 2>&1 )
  RC=$?
  echo "turn $TURN rc=$RC -> $OUT"
  # The driver's own log is the player's eyes; print it whole. If the driver
  # died, say so with its stderr rather than printing an empty turn.
  if [ $RC -ne 0 ]; then
    echo "--- DRIVER FAILED (rc=$RC), last output ---"
    tail -20 "$PLAY/turnout-$SLOT-$TURN.log"
  fi
  sed 's/\x1b\[[0-9;]*m//g' "$OUT"
  ;;
watch)
  N=${3:-60}
  sed 's/\x1b\[[0-9;]*m//g' "$SRVLOG" | grep -E "EXPERIENCE census|colony drive|dish produced|ate — hunger|slept|unreachable|stalled on materials|RE-TARGET|beds registered|COLONY TERMINAL" | tail -"$N"
  ;;
stop)
  [ -f "$PIDF" ] && kill "$(cat "$PIDF")" 2>/dev/null
  rm -f "$PIDF"
  echo "slot $SLOT stopped"
  ;;
*)
  echo "usage: play-harness.sh boot|turn|watch|stop <slot> [arg]"; exit 2 ;;
esac
