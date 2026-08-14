#!/usr/bin/env bash
# Relaunch arms B and C with FULL port isolation.
#
# Attempt 2 isolated only the GAME port (14004). The server then logged
# "Server is ready to accept connections" on its own port and panicked a
# moment later on `web_address` 14005 -- AddrInUse -- because all three arms
# shared it. prioA won the race and survived; B and C died AFTER announcing
# readiness, which is why the port poll saw nothing wrong.
#
# A parallel leg needs every listening socket moved, not just the one the
# test talks to.
set -u
WT=/e/veloren-master/.engine-integration-wt
EV=/e/veloren-master/bastion-test-evidence
B=$WT/target/no_overflow
A=E:/veloren-master/.engine-integration-wt/assets

arm() {
  TAG=$1; GAME=$2; WEB=$3; MET=$4
  UD="E:/veloren-master/.engine-integration-wt/userdata-$TAG"
  rm -rf "$WT/userdata-$TAG"
  VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A "$B/veloren-server-cli.exe" \
      --no-auth admin add "$TAG" admin > "$EV/admin-$TAG.log" 2>&1

  S=$WT/userdata-$TAG/server/server_config/settings.ron
  sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:$MET\"/g" "$S"
  # server-cli only reads settings.ron, never the template it writes
  CS=$WT/userdata-$TAG/server-cli/settings.ron
  sed "s/:14005\"/:$WEB\"/" "$WT/userdata-$TAG/server-cli/settings.template.ron" > "$CS"

  echo "$TAG: admins=$(grep -c 'role: Admin' "$WT/userdata-$TAG/server/server_config/admins.ron") \
game=$(grep -c ":$GAME\"" "$S") web=$(grep -c ":$WEB\"" "$CS") met=$(grep -c ":$MET\"" "$S")"

  ( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A \
      BASTION_DETERMINISTIC=1 BASTION_AUTOFOUND_COLONY=8 \
      BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 \
      "$B/veloren-server-cli.exe" --no-auth > "$EV/server-prio2-$TAG.log" 2>&1 ) &
}

echo "=== relaunching B and C with full port isolation ==="
arm prioB 14204 14205 14206
arm prioC 14304 14305 14306
sleep 2
echo "launched; drivers attach via attach-prio-drivers.sh port poll"
