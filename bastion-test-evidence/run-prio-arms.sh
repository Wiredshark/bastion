#!/usr/bin/env bash
# ITEM 16 -- haul priority live path. Three arms, PARALLEL on distinct ports.
#
# FIXES over attempt 1 (both arms were VOID by their own registered preconditions):
#   1. `--no-auth` on the admin grant. Without it the CLI resolves the username
#      through the auth server, fails, and writes an EMPTY admins.ron -- the
#      driver then got `command-no-permission` and cmd_witness=0.
#   2. Window 5400 -> 12000 ticks. Arm A (the CONTROL) hauled 0 at 5400, which
#      the prereg says voids B and C outright.
#   3. Parallel ports, NOT uncapped TPS. `BASTION_UNCAPPED_TPS` skips clock.tick()
#      from server boot with no gate on a client being connected, so a
#      client-driven leg would free-run past its whole window pre-connect.
#      Uncapped is a HEADLESS-ONLY lever.
set -u
WT=/e/veloren-master/.engine-integration-wt
EV=/e/veloren-master/bastion-test-evidence
B=$WT/target/no_overflow
A=E:/veloren-master/.engine-integration-wt/assets

arm() {
  TAG=$1; PORT=$2
  UD="E:/veloren-master/.engine-integration-wt/userdata-$TAG"
  rm -rf "$WT/userdata-$TAG"
  # creates userdata + settings.ron AND grants admin (uuid derived locally)
  VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A "$B/veloren-server-cli.exe" \
      --no-auth admin add "$TAG" admin > "$EV/admin-$TAG.log" 2>&1
  sed -i "s/:14004\"/:$PORT\"/g" "$WT/userdata-$TAG/server/server_config/settings.ron"
  grep -c ":$PORT\"" "$WT/userdata-$TAG/server/server_config/settings.ron" \
      | xargs -I{} echo "  $TAG port lines rewritten: {}"
  grep -c '"role": *Admin\|role: Admin' "$WT/userdata-$TAG/server/server_config/admins.ron" \
      | xargs -I{} echo "  $TAG admin entries: {}"

  ( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A \
      BASTION_DETERMINISTIC=1 BASTION_AUTOFOUND_COLONY=8 \
      BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 \
      "$B/veloren-server-cli.exe" --no-auth > "$EV/server-prio2-$TAG.log" 2>&1 ) &
  echo $! > "$EV/.pid-$TAG"
}

echo "=== launching three arms in parallel ==="
arm prioA 14104
arm prioB 14204
arm prioC 14304

sleep 45   # worldgen + founding
for pair in "prioA 14104" "prioB 14204" "prioC 14304"; do
  set -- $pair
  ( "$B/bastion_playtest.exe" "127.0.0.1:$2" "$1" \
      "$EV/script-prio-${1: -1}.txt" "$EV/driver-prio2-$1.log" \
      > "$EV/driverout-prio2-$1.log" 2>&1 ) &
done
wait
echo "=== drivers done; stopping servers ==="
for T in prioA prioB prioC; do
  kill "$(cat "$EV/.pid-$T")" 2>/dev/null
done
sleep 5
