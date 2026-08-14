#!/usr/bin/env bash
# ITEM 16 -- POWERED re-run. Two replicates per arm, 3x window, six servers in
# parallel with every listening socket isolated.
#
# WHY: the short run gave control=3 hauls vs treatment=0. That looks like a
# separation, but MY OWN earlier row (HAUL-THROUGHPUT-RESULTS.md) recorded two
# n=8 client legs on one script hauling 5 and 0 -- so ZERO IS INSIDE THE
# CONTROL'S OWN DISTRIBUTION, and a 3-vs-0 result at n=1 cannot be scored.
# Replicates give the control a distribution; the longer window gives it a
# magnitude big enough for zero to mean something.
set -u
WT=/e/veloren-master/.engine-integration-wt
EV=/e/veloren-master/bastion-test-evidence
B=$WT/target/no_overflow
A=E:/veloren-master/.engine-integration-wt/assets

arm() { # tag script game web metrics
  TAG=$1; SCR=$2; GAME=$3; WEB=$4; MET=$5
  UD="E:/veloren-master/.engine-integration-wt/userdata-$TAG"
  rm -rf "$WT/userdata-$TAG"
  VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A "$B/veloren-server-cli.exe" \
      --no-auth admin add "$TAG" admin > "$EV/admin-$TAG.log" 2>&1
  S=$WT/userdata-$TAG/server/server_config/settings.ron
  sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:$MET\"/g" "$S"
  sed "s/:14005\"/:$WEB\"/" "$WT/userdata-$TAG/server-cli/settings.template.ron" \
      > "$WT/userdata-$TAG/server-cli/settings.ron"
  echo "$TAG admins=$(grep -c 'role: Admin' "$WT/userdata-$TAG/server/server_config/admins.ron") game=$GAME web=$WEB met=$MET" >> "$EV/powered.log"

  ( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A \
      BASTION_DETERMINISTIC=1 BASTION_AUTOFOUND_COLONY=8 \
      BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 \
      "$B/veloren-server-cli.exe" --no-auth > "$EV/server-pw-$TAG.log" 2>&1 ) &

  ( t=0
    while [ $t -lt 1200 ]; do
      if (exec 3<>"/dev/tcp/127.0.0.1/$GAME") 2>/dev/null; then exec 3<&- 3>&-; break; fi
      sleep 3; t=$((t+3))
    done
    if [ $t -ge 1200 ]; then echo "$TAG PORT NEVER OPENED -- VOID" >> "$EV/powered.log"; exit 0; fi
    echo "$TAG connecting after ${t}s" >> "$EV/powered.log"
    "$B/bastion_playtest.exe" "127.0.0.1:$GAME" "$TAG" \
        "$EV/$SCR" "$EV/driver-pw-$TAG.log" > "$EV/driverout-pw-$TAG.log" 2>&1
    echo "$TAG driver exited rc=$?" >> "$EV/powered.log" ) &
}

: > "$EV/powered.log"
arm pwA1 script-prioL-A.txt 14404 14405 14406
arm pwA2 script-prioL-A.txt 14504 14505 14506
arm pwB1 script-prioL-B.txt 14604 14605 14606
arm pwB2 script-prioL-B.txt 14704 14705 14706
arm pwC1 script-prioL-C.txt 14804 14805 14806
arm pwC2 script-prioL-C.txt 14904 14905 14906
wait
echo "=== ALL SIX ARMS DONE ===" >> "$EV/powered.log"
