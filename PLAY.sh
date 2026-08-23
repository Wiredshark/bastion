#!/bin/bash
# PLAY — launch a bastion world and the client, for Ben.
#
#   bash PLAY.sh town    -> adopt a real worldgen village (the new framework:
#                           the VILLAGERS become your colony)
#   bash PLAY.sh flattown -> a REAL worldgen village (houses, doors, roads,
#                            fields) standing on FLAT ground, raiders OFF
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
    #
    # ★ THE SELECT STARTING AREA SCREEN IS THE CHOOSER. Pick a town there when
    # you create your character and that is the town you adopt. A middle-click
    # map marker in-game still overrides it.
    #
    # ★ ZERO CEREMONY (Ben, 2026-08-22: "just boot me into a town that i own
    # and have colonists that function"). The scorer picks the best settlement,
    # founds on it immediately, and the player spawns IN it.
    #
    # Choosing was briefly the default earlier today and that was wrong: it put
    # every failure mode of the chooser (stall, wrong town, silent fallback) on
    # the one path everybody uses. Export BASTION_ADOPT_WAIT_FOR_MARKER=1 to
    # get it back (PLAY.ps1 -Pick).
    #
    # Keep this in step with PLAY.ps1. The two launchers named `town` already
    # disagreed once on exactly this flag, so bash and PowerShell silently did
    # different things under the same mode name.
    ENVV="BASTION_ADOPT_TOWN=1 BASTION_SPAWN_AT_COLONY=1 BASTION_AUTOFOUND_REAL_TERRAIN=1 BASTION_COLONY_PRESENCE_VD=3 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=64 BASTION_SEED_MATERIALS=64"
    ;;
  flattown)
    # ★ THE FLAT-MAP TOWN (Ben: "i wanna test this flat map town").
    #
    # A REAL worldgen village -- houses, doors, roads, farm fields, workshops --
    # standing on FLAT ground. `BASTION_FLAT_WORLD_RADIUS` flattens a disc of
    # sim chunks at world centre BEFORE civ generation runs, so villages are
    # placed onto the flat disc rather than having the ground pulled out from
    # under buildings whose heights are already baked.
    #
    # ★ MEASURED, THREE TIMES, AND THE HONEST RESULT IS A TRADE:
    #
    #   flat r=10,  search 16384 -> village 46 houses / 23 fields, but
    #                               chosen_dist=11229 -- ELEVEN KM away, on
    #                               normal bumpy ground. Flattening irrelevant.
    #   flat r=64,  search  1900 -> village INSIDE the disc (dist 1224), but
    #                               considered=1, 3 houses, ZERO fields
    #   flat r=160, search  4800 -> considered=1, ONE house, one field, 2 beds
    #
    # The two radii MUST agree or the town lands off the flat. But binding them
    # is not free: this world simply has no large village near world centre, so
    # a flat town here is a HAMLET. `bash PLAY.sh town` is where the real
    # 46-house / 23-field settlement lives -- on ordinary terrain.
    #
    # Use flattown for LEGIBILITY (flat ground reads clearly, good for watching
    # pathing); use town to see a real settlement.
    #
    # RAIDERS OFF, per Ben's call: "get the town working like real life then
    # introduce raiders and see what breaks." Drop BASTION_NO_RAIDS=1 to let
    # them back in.
    ENVV="BASTION_FLAT_WORLD_RADIUS=64 BASTION_ADOPT_RADIUS=2000 BASTION_ADOPT_TOWN=1 BASTION_AUTOFOUND_REAL_TERRAIN=1 BASTION_COLONY_PRESENCE_VD=3 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=64 BASTION_SEED_MATERIALS=64 BASTION_NO_RAIDS=1"
    ;;
  arena)
    ENVV="BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=32"
    ;;
  *) echo "usage: bash PLAY.sh <flattown|town|arena> [client]"; exit 2 ;;
esac

# ★ CHECK THE BINARY BEFORE DESTROYING THE SAVE (2026-08-21). A goal-driven
# play session lost its best world to exactly this ordering: boot wiped
# userdata FIRST, then failed because a concurrent build had removed the server
# exe. The world was gone and the reboot failed anyway — a destructive step
# gated on nothing, with the check sitting three lines too late.
if [ ! -x "$B/veloren-server-cli.exe" ]; then
  echo "REFUSING: no server binary at $B/veloren-server-cli.exe"
  echo "(a build may be in progress — your existing world is untouched)"
  exit 1
fi
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
