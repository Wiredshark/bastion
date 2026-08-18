#!/usr/bin/env bash
# THE SHARED SINGLE-ARM LAUNCH TEMPLATE for scored Bastion runs.
#
# WHY THIS EXISTS: eight launchers (ack blk clp dash drp insp stn swp) were
# proven byte-identical after normalising away TAG, ports and script name --
# 7/7 empty diffs, 1310 bytes of residue each. They shared every hole with
# perfect fidelity: no attestation, no declared config, no declared outputs,
# no `SRV` capture, no `exec` in the server subshell, no teardown. The
# postamble's header predicted this shape ("a copied template propagates its
# holes") and the fix belongs HERE, once, not in eight synchronized edits
# that can drift apart again.
#
# AND THE PORTS ARE DERIVED, NOT LISTED. The eight originals had distinct
# game and web ports but ALL EIGHT wrote query port 18006 -- a collision that
# survived because the engine's query server (a UdpSocket; see
# server/src/lib.rs QueryServer spawn) dies with a single error! line and
# the run continues. Silent degradation never fails loudly enough to be
# noticed. WEB=GAME+1 and QUERY=GAME+2 make distinctness a consequence of
# distinct GAME values instead of a property eight literals must maintain.
#
# Interface, from a wrapper:
#     TAG=ack GAME=25004 SCRIPT=script-ack.txt
#     . "$EV/run-template-live.sh"
# Optional: PORT_WAIT (default 900s), BASTION_EXTRA (appended to BASTION_ENV).
#
# Requires from the wrapper: TAG, GAME, SCRIPT.
set -u
WT=/e/veloren-master/.engine-integration-wt
EV=/e/veloren-master/bastion-test-evidence
B=$WT/target/no_overflow
A=E:/veloren-master/.engine-integration-wt/assets
: "${TAG:?wrapper must set TAG}"; : "${GAME:?wrapper must set GAME}"
: "${SCRIPT:?wrapper must set SCRIPT}"
WEB=$((GAME+1)); QUERY=$((GAME+2)); PORT_WAIT="${PORT_WAIT:-900}"
UD="E:/veloren-master/.engine-integration-wt/userdata-$TAG"

# THE SINGLE DEFINITION: recorded by the attestation and applied to the
# server from the same string. The eight originals set these vars inline on
# the launch line, which is exactly the configuration the attestation could
# never see.
export BASTION_ENV="BASTION_DETERMINISTIC=1 BASTION_AUTOFOUND_COLONY=8 BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1${BASTION_EXTRA:+ $BASTION_EXTRA}"

# The four files every run of this template writes, declared before any of
# them exists. `run-ledger.sh` resolves each promise against the disk.
export BASTION_LOGS="$EV/server-$TAG.log $EV/$TAG.log $EV/driver-$TAG.log $EV/driverout-$TAG.log"

# ATTEST + GATE. Refuses stale binaries, missing binaries, and (since the
# declaration gate) a run that declares no outputs.
. "$EV/launch-preamble.sh"

rm -rf "$WT/userdata-$TAG"
VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A "$B/veloren-server-cli.exe" \
    --no-auth admin add "$TAG" admin > /dev/null 2>&1
S=$WT/userdata-$TAG/server/server_config/settings.ron
sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:$QUERY\"/g" "$S"
sed "s/:14005\"/:$WEB\"/" "$WT/userdata-$TAG/server-cli/settings.template.ron" \
    > "$WT/userdata-$TAG/server-cli/settings.ron"

# `exec` IS LOAD-BEARING: without it `$!` names the subshell and the
# postamble's kill reaches nothing (measured in run-live-check.sh: `$!` said
# 42308, the server was 33664, and the orphan held its port).
( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A \
    exec env $BASTION_ENV \
    "$B/veloren-server-cli.exe" --no-auth > "$EV/server-$TAG.log" 2>&1 ) &
SRV=$!
echo "server pid=$SRV (started by this template for $TAG)" > "$EV/$TAG.log"

t=0
while [ $t -lt "$PORT_WAIT" ]; do
  if (exec 3<>"/dev/tcp/127.0.0.1/$GAME") 2>/dev/null; then exec 3<&- 3>&-; break; fi
  sleep 3; t=$((t+3))
done
echo "port $GAME open after ${t}s" >> "$EV/$TAG.log"

"$B/bastion_playtest.exe" "127.0.0.1:$GAME" "$TAG" \
    "$EV/$SCRIPT" "$EV/driver-$TAG.log" > "$EV/driverout-$TAG.log" 2>&1
echo "driver exited rc=$?" >> "$EV/$TAG.log"

# STOP + VERIFY: kill only the recorded pid, then witness the PORT, which is
# what actually blocks the next run.
. "$EV/launch-postamble.sh"
