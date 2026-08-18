#!/usr/bin/env bash
# THE DEFERRED LIVE LEG: does the DECLARED config reach the SERVER?
#
# Three rows argued "recorded == applied" from the shell and never launched a
# server through the gate. This runner does, and it is the SECOND ADOPTER of
# launch-preamble.sh -- the previous row declined to claim the preamble
# generalises beyond run-witness.sh, so this tests that rather than repeating it.
#
# Ports are 26014/26015/18016, NOT run-witness.sh's 26004/26005/18006: a
# veloren-server-cli this session did not start is listening on those, and a
# process I did not start is not mine to kill.
#
# Usage:  run-live-check.sh <colonist-count>
set -u
WT=/e/veloren-master/.engine-integration-wt
EV=/e/veloren-master/bastion-test-evidence
B=$WT/target/no_overflow
A=E:/veloren-master/.engine-integration-wt/assets
N="${1:?usage: run-live-check.sh <colonist-count>}"
TAG="live$N"; GAME=26014; WEB=26015; METRICS=18016
UD="E:/veloren-master/.engine-integration-wt/userdata-$TAG"

# THE SINGLE DEFINITION. The count is interpolated here ONCE; the attestation
# records this string and the server is launched from this string, so the
# dose-response plant (8 -> 3) changes exactly one place.
export BASTION_ENV="BASTION_DETERMINISTIC=1 BASTION_AUTOFOUND_COLONY=$N BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1"

# THE FILES THIS RUN WILL WRITE. TWO, not four: this runner reads the server
# log for a spawn line and never launches the driver, so declaring driver logs
# here would promise files that are never written -- and `run-ledger.sh` would
# correctly report them absent. The declaration is per-runner because the file
# set is.
export BASTION_LOGS="$EV/server-$TAG.log $EV/$TAG.log"

# ATTEST + GATE, via the shared preamble. Needs EV, B, TAG, BASTION_ENV.
. "$EV/launch-preamble.sh"

rm -rf "$WT/userdata-$TAG"
VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A "$B/veloren-server-cli.exe" \
    --no-auth admin add "$TAG" admin > /dev/null 2>&1
S=$WT/userdata-$TAG/server/server_config/settings.ron
sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:$METRICS\"/g" "$S"
sed "s/:14005\"/:$WEB\"/" "$WT/userdata-$TAG/server-cli/settings.template.ron" \
    > "$WT/userdata-$TAG/server-cli/settings.ron"

# `exec` IS LOAD-BEARING, NOT STYLE. Without it the backgrounded subshell
# forks the server as a CHILD and `$!` names the subshell, not the server --
# measured on leg 1: `$!` reported 42308, which was not in the Windows
# process table at all, while the real server ran as pid 33664 with the
# subshell 45440 as its parent. `kill "$SRV"` then killed nothing, the
# server survived holding port 26014, and the next leg would have collided.
#
# That is precisely the condition that forced this row's deferral in the
# first place: an orphaned server holding a port that nobody may kill,
# because nobody can prove they started it. The runner was manufacturing
# the problem it exists to work around.
#
# `exec` REPLACES the subshell with the server process, so `$!` names the
# server itself and the kill below reaches it.
( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A \
    exec env $BASTION_ENV \
    "$B/veloren-server-cli.exe" --no-auth > "$EV/server-$TAG.log" 2>&1 ) &
SRV=$!
echo "server pid=$SRV (started by this script)" > "$EV/$TAG.log"

# DECLARED WINDOW, fixed before launch: 300s for the port, then 240s for the
# spawn line. Not extended on a good result.
t=0
while [ $t -lt 300 ]; do
  if (exec 3<>"/dev/tcp/127.0.0.1/$GAME") 2>/dev/null; then exec 3<&- 3>&-; break; fi
  sleep 3; t=$((t+3))
done
echo "port $GAME open after ${t}s" >> "$EV/$TAG.log"

w=0
while [ $w -lt 240 ]; do
  if grep -q "spawned starting colony" "$EV/server-$TAG.log" 2>/dev/null; then break; fi
  sleep 3; w=$((w+3))
done
echo "spawn line after ${w}s in-window" >> "$EV/$TAG.log"

# STOP + VERIFY, via the shared postamble. This runner was the reference
# implementation -- the postamble was extracted from it, and adopting it
# here is a refactor against known-correct behaviour rather than a new
# claim. It also adds the third outcome this script did not distinguish:
# an unrecorded pid now reads differently from a successful teardown.
. "$EV/launch-postamble.sh"
