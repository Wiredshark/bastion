#!/usr/bin/env bash
# Item-19 HORIZON RETEST (Ben-authorized GPU session) + r0d/r0p live arming.
#
# Two isolated arms, each a self-starting --bastion-flat-arena voxygen
# session (fresh per-arm userdata/config so a saved settings.ron cannot
# leak the far-band profile into the control):
#   arm FAR : BASTION_POST_R2_DISTANCE_PROFILE=far-band-v1 (16->24 band)
#   arm REF : no profile (stock defaults) — the control
# Both arms arm the r0p observer (durable frame+horizon census) and the
# horizon fixture camera. The item-19 fixture fix (arena radius 26 >=
# tested horizon) is in this tree, so unlike the July run the slab
# actually covers what the camera sees.
#
# ENUMERATED DELTA between arms (honesty): REF runs stock view distance,
# not a pinned 16 — the far-band plan carries its own 16 as near radius,
# so the FAR arm's census buckets answer the decisive question (is
# terrain VISIBLE in the 16-24 band at all) on their own; REF controls
# "did the profile move anything".
set -u
WT="E:/veloren-master/.renderer-wt"
B="$WT/target/debug"
A="$WT/assets"
EV="$WT/readme/renderer-bench/smoke/gpu"
SECS="${SECS:-300}"
mkdir -p "$EV"

echo "PRECONDITION voxygen=$(sha256sum "$B/veloren-voxygen.exe" | cut -c1-16) tip=$(cd "$WT" && git rev-parse --short=10 HEAD)"

arm() {
  local NAME="$1"; shift
  local UD="$EV/userdata-$NAME"
  rm -rf "$UD" "$EV/r0p-$NAME.json" "$EV/capture-$NAME"; mkdir -p "$UD/voxygen"
  echo "arm $NAME: launching (${SECS}s budget)"
  ( cd "$WT" && VELOREN_USERDATA="$UD" VOXYGEN_CONFIG="$UD/voxygen" VELOREN_ASSETS="$A" \
      VELOREN_CLIENT_TYPE=silent_spectator \
      BASTION_POST_R2_VISIBLE_HORIZON=flat-arena-oblique-horizon-v1 \
      BASTION_POST_R2_STREAMING_MEASUREMENT=continuous-server-v1 \
      BASTION_R0P_OUTPUT="$EV/r0p-$NAME.json" \
      BASTION_R0P_SCENARIO="horizon-retest-$NAME" \
      BASTION_R0D_CAPTURE_OUT="$EV/capture-$NAME" \
      BASTION_R0D_CAPTURE_WARMUP=300 BASTION_R0D_CAPTURE_COUNT=5 \
      env "$@" \
      "$B/veloren-voxygen.exe" --bastion-flat-arena > "$EV/voxygen-$NAME.log" 2>&1 ) &
  local PID=$!
  local t=0
  while [ $t -lt "$SECS" ]; do
    sleep 10; t=$((t+10))
    kill -0 "$PID" 2>/dev/null || { echo "arm $NAME: EXITED EARLY at ${t}s (see voxygen-$NAME.log)"; break; }
  done
  # Identity-scoped kill FIRST: $PID is the SUBSHELL's bash pid, which
  # taskkill cannot map (the documented Git-Bash trap) — killing the
  # actual voxygen by name+cmdline is what releases the wait below.
  powershell -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='veloren-voxygen.exe'\" | Where-Object { \$_.CommandLine -match 'renderer-wt' } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force }" >/dev/null 2>&1
  wait "$PID" 2>/dev/null
  # The observer writes a DIRECTORY of durable chunk files.
  if [ -s "$EV/r0p-$NAME.json/frames.jsonl" ]; then
    echo "arm $NAME: r0p frames PRESENT ($(wc -c < "$EV/r0p-$NAME.json/frames.jsonl") bytes)"
  else
    echo "arm $NAME: r0p frames MISSING — arm VOID"
    return 1
  fi
  ls "$EV/capture-$NAME" 2>/dev/null | head -3 | sed "s/^/arm $NAME capture: /"
}

arm FAR BASTION_POST_R2_DISTANCE_PROFILE=far-band-v1 || exit 1
arm REF || exit 1
echo "ARMS COMPLETE — parse r0p-FAR.json vs r0p-REF.json horizon census next"
