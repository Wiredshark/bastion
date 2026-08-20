#!/usr/bin/env bash
# renderer-bench W2 smoke + determinism twin (fork build).
#
# Launches the FORK-BUILT server on the flat arena with the bench armed on
# the walk-and-seek fixture, waits for the tape terminal, runs a SECOND
# identical leg, then compares the two tapes with the golden CLI:
# identical run_root on a twin pair IS the stack's determinism witness
# (fixture load → agentless drive → semantic tape, end to end).
#
# PRECONDITION printed above the result (house law): the binary stamp and
# the fixture sha are echoed before any verdict line.
set -u
WT="E:/veloren-master/.renderer-wt"
B="$WT/target/debug"
A="$WT/assets"
FIX="$WT/readme/renderer-bench/fixtures/walk-and-seek-v1.rbdm"
EV="$WT/readme/renderer-bench/smoke"
GAME=27024; WEB=27025; METRICS=27026
TICKS="${TICKS:-600}"; CADENCE="${CADENCE:-30}"
mkdir -p "$EV"

echo "PRECONDITION binary=$(sha256sum "$B/veloren-server-cli.exe" | cut -c1-16)"
echo "PRECONDITION fixture=$(sha256sum "$FIX" | cut -c1-16) ticks=$TICKS cadence=$CADENCE"

leg() {
  local N="$1"
  local TAG="rbench-$N"
  local UD="$WT/userdata-$TAG"
  rm -rf "$UD"
  # First boot writes default settings; then pin the ports.
  VELOREN_USERDATA="$UD" VELOREN_ASSETS="$A" "$B/veloren-server-cli.exe" \
      --no-auth admin add "$TAG" admin > /dev/null 2>&1
  local S="$UD/server/server_config/settings.ron"
  sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:$METRICS\"/g" "$S"
  if [ -f "$UD/server-cli/settings.template.ron" ]; then
    sed "s/:14005\"/:$WEB\"/" "$UD/server-cli/settings.template.ron" \
        > "$UD/server-cli/settings.ron"
  fi
  ( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS="$A" \
      BASTION_FLAT_ARENA=1 BASTION_DETERMINISTIC=1 \
      BASTION_RENDERER_BENCH_MANIFEST="$FIX" \
      BASTION_RENDERER_BENCH_OUT="$EV/tape-$N.json" \
      BASTION_RENDERER_BENCH_TICKS="$TICKS" \
      BASTION_RENDERER_BENCH_CADENCE="$CADENCE" \
      "$B/veloren-server-cli.exe" --no-auth > "$EV/server-$N.log" 2>&1 ) &
  local SRV=$!
  echo "leg $N: server pid=$SRV"
  # Wait for the tape (mtime-fresh file), bounded.
  local t=0
  while [ $t -lt 420 ]; do
    if [ -s "$EV/tape-$N.json" ]; then break; fi
    sleep 2; t=$((t+2))
  done
  kill "$SRV" 2>/dev/null
  wait "$SRV" 2>/dev/null
  if [ -s "$EV/tape-$N.json" ]; then
    echo "leg $N: TAPE ARRIVED after ~${t}s"
  else
    echo "leg $N: TAPE MISSING after ${t}s — leg VOID (see server-$N.log)"
    return 1
  fi
}

rm -f "$EV/tape-1.json" "$EV/tape-2.json"
leg 1 || exit 1
leg 2 || exit 1

"$B/bastion-harness.exe" --renderer-bench-golden "$EV/tape-1.json" "$EV/tape-2.json"
RC=$?
echo "TWIN VERDICT exit=$RC (0 = run_root identical = determinism witness GREEN)"
exit $RC
