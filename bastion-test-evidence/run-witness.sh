#!/usr/bin/env bash
# ITEM 9: exercise the inspector's ENTITY arm live -- the arm the HUD uses and
# that nothing automated had ever touched (the driver could only inspect CELLS).
set -u
WT=/e/veloren-master/.engine-integration-wt
EV=/e/veloren-master/bastion-test-evidence
B=$WT/target/no_overflow
A=E:/veloren-master/.engine-integration-wt/assets
TAG=wit; GAME=26004
UD="E:/veloren-master/.engine-integration-wt/userdata-$TAG"

# THE RUN'S CONFIG, DECLARED ONCE. This variable is the SINGLE definition:
# it is recorded by the attestation below and applied to the server at
# launch, so the config in the evidence file cannot drift from the config
# the server actually ran with. Previously these assignments existed only
# inline on the launch line and NO evidence file recorded them -- and they
# decide what a Bastion run does far more than HEAD does.
#
# It cannot be recovered by observation: set inline, they are never
# exported, so the attestation's own environment does not contain them.
export BASTION_ENV="BASTION_DETERMINISTIC=1 BASTION_AUTOFOUND_COLONY=8 BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1"

# ATTEST BOTH BINARIES AND GATE ON IT -- now via the shared preamble, so
# the next runner inherits it by sourcing one line instead of re-deriving
# four steps. This runner was the reference implementation: the preamble
# was extracted from it and verified to reproduce its attestation byte for
# byte apart from the timestamp.
# THE FOUR FILES THIS RUN WILL WRITE. Hardcoded `wit` rather than $TAG,
# matching the redirects below verbatim -- this runner names its logs with a
# literal, and a declaration derived differently from the redirect it claims to
# describe would be exactly the drift the single-definition rule exists to stop.
export BASTION_LOGS="$EV/server-wit.log $EV/wit.log $EV/driver-wit.log $EV/driverout-wit.log"

. "$EV/launch-preamble.sh"

rm -rf "$WT/userdata-$TAG"
VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A "$B/veloren-server-cli.exe" \
    --no-auth admin add "$TAG" admin > /dev/null 2>&1
S=$WT/userdata-$TAG/server/server_config/settings.ron
sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:18006\"/g" "$S"
sed 's/:14005"/:26005"/' "$WT/userdata-$TAG/server-cli/settings.template.ron" \
    > "$WT/userdata-$TAG/server-cli/settings.ron"

# APPLIED from the SAME variable the attestation recorded -- `env` takes the
# assignments as arguments, so there is exactly one place the config is
# written. Editing the launch line without editing the record is now
# impossible, because there is no second place to edit.
( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A \
    env $BASTION_ENV \
    "$B/veloren-server-cli.exe" --no-auth > "$EV/server-wit.log" 2>&1 ) &

t=0
while [ $t -lt 900 ]; do
  if (exec 3<>"/dev/tcp/127.0.0.1/$GAME") 2>/dev/null; then exec 3<&- 3>&-; break; fi
  sleep 3; t=$((t+3))
done
echo "connecting after ${t}s" > "$EV/wit.log"
"$B/bastion_playtest.exe" "127.0.0.1:$GAME" "$TAG" \
    "$EV/script-witness.txt" "$EV/driver-wit.log" > "$EV/driverout-wit.log" 2>&1
echo "driver exited rc=$?" >> "$EV/wit.log"
