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
  # Git Bash `kill` does not reliably terminate Windows processes; use
  # taskkill on the pid Git Bash reports (it maps to the Windows pid for
  # direct children). Fall back to kill for portability.
  taskkill //F //PID "$SRV" >/dev/null 2>&1 || kill -9 "$SRV" 2>/dev/null
  wait "$SRV" 2>/dev/null
  # Belt-and-braces: if the port is still held by OUR binary, name+path kill.
  powershell -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='veloren-server-cli.exe'\" | Where-Object { \$_.CommandLine -match 'renderer-wt' } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force }" >/dev/null 2>&1
  if [ -s "$EV/tape-$N.json" ]; then
    echo "leg $N: TAPE ARRIVED after ~${t}s"
  else
    echo "leg $N: TAPE MISSING after ${t}s — leg VOID (see server-$N.log)"
    return 1
  fi
}

rm -f "$EV/tape-1.json" "$EV/tape-2.json" "$EV/tape-A.json"
leg 1 || exit 1
leg 2 || exit 1

"$B/bastion-harness.exe" --renderer-bench-golden "$EV/tape-1.json" "$EV/tape-2.json"
RC=$?
echo "TWIN VERDICT exit=$RC (0 = run_root identical = determinism witness GREEN)"
[ $RC -eq 0 ] || exit $RC

# ── W3 leg A: server WAITS for a client, ackbot spectates + acks. ──
# The wave's integrated proof (W3-LAUNCH-PACKET.md): tape A must carry
# >=2 acks, all echo_match, entities_resolved=3 — and run_root(A) must
# equal run_root(clientless leg 1): observing through the net does not
# perturb the tape.
echo "PRECONDITION ackbot=$(sha256sum "$B/rbench_ackbot.exe" | cut -c1-16)"
leg_a() {
  local TAG="rbench-A"
  local UD="$WT/userdata-$TAG"
  rm -rf "$UD"
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
      BASTION_RENDERER_BENCH_OUT="$EV/tape-A.json" \
      BASTION_RENDERER_BENCH_TICKS="$TICKS" \
      BASTION_RENDERER_BENCH_CADENCE="$CADENCE" \
      BASTION_RENDERER_BENCH_WAIT_CLIENT=1 \
      "$B/veloren-server-cli.exe" --no-auth > "$EV/server-A.log" 2>&1 ) &
  local SRV=$!
  echo "leg A: server pid=$SRV (WAIT_CLIENT=1)"
  sleep 20  # let the server bind before the bot dials
  ( cd "$WT" && BASTION_RENDERER_BENCH_ACK=1 VELOREN_ASSETS="$A" \
      "$B/rbench_ackbot.exe" "localhost:$GAME" "$TAG" 0 0 40 420 \
      > "$EV/ackbot-A.log" 2>&1 ) &
  local BOT=$!
  echo "leg A: ackbot pid=$BOT"
  local t=0
  while [ $t -lt 420 ]; do
    if [ -s "$EV/tape-A.json" ]; then break; fi
    sleep 2; t=$((t+2))
  done
  taskkill //F //PID "$BOT" >/dev/null 2>&1 || kill -9 "$BOT" 2>/dev/null
  taskkill //F //PID "$SRV" >/dev/null 2>&1 || kill -9 "$SRV" 2>/dev/null
  wait "$SRV" 2>/dev/null; wait "$BOT" 2>/dev/null
  powershell -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='veloren-server-cli.exe'\" | Where-Object { \$_.CommandLine -match 'renderer-wt' } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force }" >/dev/null 2>&1
  powershell -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='rbench_ackbot.exe'\" | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force }" >/dev/null 2>&1
  if [ -s "$EV/tape-A.json" ]; then
    echo "leg A: TAPE ARRIVED after ~${t}s"
  else
    echo "leg A: TAPE MISSING after ${t}s — leg VOID (see server-A.log + ackbot-A.log)"
    return 1
  fi
}
leg_a || exit 1

"$B/bastion-harness.exe" --renderer-bench-golden "$EV/tape-A.json" "$EV/tape-1.json"
RCA=$?
echo "NEUTRALITY VERDICT exit=$RCA (0 = client presence did not perturb run_root)"

python - "$EV/tape-A.json" <<'PYEOF'
import json, sys
d = json.load(open(sys.argv[1]))
acks = d.get("client_acks", [])
ready = d.get("ready_count", 0)
ok = (len(acks) >= 2
      and all(a.get("echo_match") is True for a in acks)
      and all(a.get("entities_resolved") == 3 for a in acks)
      and ready >= 1)
print(f"ACK VERDICT acks={len(acks)} ready_count={ready} "
      f"echo_all={all(a.get('echo_match') is True for a in acks) if acks else False} "
      f"resolved3_all={all(a.get('entities_resolved') == 3 for a in acks) if acks else False} "
      f"=> {'GREEN' if ok else 'RED'}")
sys.exit(0 if ok else 1)
PYEOF
RCB=$?
[ $RCA -eq 0 ] && [ $RCB -eq 0 ] && { echo "W3 INTEGRATED VERDICT GREEN"; exit 0; }
echo "W3 INTEGRATED VERDICT RED (neutrality=$RCA acks=$RCB)"
exit 1
