#!/bin/bash
# PLAY — launch a bastion world and the client, for Ben.
#
#   bash PLAY.sh town    -> adopt a real worldgen village (the new framework:
#                           the VILLAGERS become your colony)
#   bash PLAY.sh arena   -> found a colony on flat test ground (mines its own
#                           stone, builds its own beds, colonists carry tools)
#
# Then launch the client separately (see the line this prints), or pass
# `client` as a second argument to start it too.
set -u
WT=/e/veloren-master/.item29-wt
B=$WT/target/no_overflow
MODE=${1:-town}
UD=$WT/userdata-play-ben

case "$MODE" in
  town)
    # ADOPT-A-TOWN. The colony IS the village's residents — nobody is spawned.
    # Drop a map marker in-game to choose WHICH town; with no marker it takes
    # the nearest.
    ENVV="BASTION_ADOPT_TOWN=1 BASTION_AUTOFOUND_REAL_TERRAIN=1 BASTION_COLONY_PRESENCE_VD=3 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=64 BASTION_SEED_MATERIALS=64"
    ;;
  arena)
    ENVV="BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=32"
    ;;
  *) echo "usage: bash PLAY.sh <town|arena> [client]"; exit 2 ;;
esac

rm -rf "$UD"
VELOREN_USERDATA=$UD VELOREN_ASSETS=$WT/assets \
  "$B/veloren-server-cli.exe" --no-auth admin add player admin >/dev/null 2>&1

echo "booting a '$MODE' world…  (worldgen takes a few minutes the first time)"
( cd "$WT" && exec env VELOREN_USERDATA=$UD VELOREN_ASSETS=$WT/assets $ENVV \
    "$B/veloren-server-cli.exe" --no-auth > "$WT/play-server.log" 2>&1 ) &
echo $! > "$WT/.play-pid"

for i in $(seq 1 120); do
  grep -q "ready to accept connections" "$WT/play-server.log" 2>/dev/null && {
    echo "READY after ${i}0s — server is up on the default port."
    break
  }
  sleep 10
done

echo
echo "  Launch the client with:"
echo "    cd $WT && VELOREN_ASSETS=\$PWD/assets ./target/no_overflow/veloren-voxygen.exe"
echo
echo "  Server log:  $WT/play-server.log"
echo "  Stop it:     kill \$(cat $WT/.play-pid)"

if [ "${2:-}" = "client" ]; then
  ( cd "$WT" && VELOREN_ASSETS=$WT/assets "$B/veloren-voxygen.exe" )
fi
