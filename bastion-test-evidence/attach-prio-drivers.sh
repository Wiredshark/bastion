#!/usr/bin/env bash
# Attach drivers to the ALREADY-RUNNING prio servers, polling until each port
# actually accepts. The fixed `sleep 45` was the defect: three parallel
# worldgens take far longer than one, the driver has NO connect retry, and it
# panicked on ConnectionRefused -- voiding all three arms a second time.
set -u
EV=/e/veloren-master/bastion-test-evidence
B=/e/veloren-master/.engine-integration-wt/target/no_overflow

waitport() { # $1=port  $2=max seconds
  local t=0
  while [ $t -lt "$2" ]; do
    if (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null; then exec 3<&- 3>&-; return 0; fi
    sleep 3; t=$((t+3))
  done
  return 1
}

for pair in "prioA 14104" "prioB 14204" "prioC 14304"; do
  set -- $pair
  ( if waitport "$2" 900; then
      echo "$1 port $2 ACCEPTING after wait" >> "$EV/attach.log"
      "$B/bastion_playtest.exe" "127.0.0.1:$2" "$1" \
        "$EV/script-prio-${1: -1}.txt" "$EV/driver-prio2-$1.log" \
        > "$EV/driverout-prio2-$1.log" 2>&1
      echo "$1 driver exited rc=$?" >> "$EV/attach.log"
    else
      echo "$1 port $2 NEVER OPENED in 900s -- arm VOID" >> "$EV/attach.log"
    fi ) &
done
wait
echo "=== all drivers finished ===" >> "$EV/attach.log"
