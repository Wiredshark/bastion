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
# v2: arena anchored ON the flat-arena slab at world center (z=401) so
# the terrain regime is defined — the server pins those chunks at boot
# whenever the bench is armed, observers or none.
FIX="$WT/readme/renderer-bench/fixtures/walk-and-seek-v2.rbdm"
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

if [ "${ONLY_LEG_A:-0}" = "1" ]; then
  # Re-run just the client leg against an existing green tape-1.
  [ -s "$EV/tape-1.json" ] || { echo "ONLY_LEG_A=1 but no tape-1.json"; exit 1; }
  rm -f "$EV/tape-A.json"
else
  rm -f "$EV/tape-1.json" "$EV/tape-2.json" "$EV/tape-A.json"
  leg 1 || exit 1
  leg 2 || exit 1

  "$B/bastion-harness.exe" --renderer-bench-golden "$EV/tape-1.json" "$EV/tape-2.json"
  RC=$?
  echo "TWIN VERDICT exit=$RC (0 = run_root identical = determinism witness GREEN)"
  [ $RC -eq 0 ] || exit $RC
fi

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
  # Gate the bot on the server's OWN readiness line (boot takes minutes;
  # a fixed sleep raced it and the bot died ConnectionRefused). The
  # WAITING line means the tick loop is live, so the port is bound.
  local bt=0
  while [ $bt -lt 300 ]; do
    if grep -q "WAITING for client readiness" "$EV/server-A.log" 2>/dev/null; then break; fi
    sleep 5; bt=$((bt+5))
  done
  if ! grep -q "WAITING for client readiness" "$EV/server-A.log" 2>/dev/null; then
    echo "leg A: server never reached WAITING after ${bt}s — leg VOID"
    taskkill //F //PID "$SRV" >/dev/null 2>&1
    return 1
  fi
  echo "leg A: server WAITING after ~${bt}s — launching ackbot"
  ( cd "$WT" && BASTION_RENDERER_BENCH_ACK=1 VELOREN_ASSETS="$A" \
      "$B/rbench_ackbot.exe" "localhost:$GAME" "$TAG" 16384.5 16384.5 441 420 \
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

python - "$EV/tape-A.json" "$FIX" <<'PYEOF'
import json, struct, sys
d = json.load(open(sys.argv[1]))
# Expected entity count comes from the FIXTURE, not a hand constant
# (walk-and-seek has 2; a wrong constant already burned one leg).
b = open(sys.argv[2], "rb").read()
o = 8
slen = struct.unpack_from("<I", b, o)[0]; o += 4 + slen  # scenario_id
o += 8 * 3 + 4 + 4 * 3                                   # seeds, tps, origin
slen = struct.unpack_from("<I", b, o)[0]; o += 4 + slen  # camera_script_id
o += 4 + 4                                               # gfx + schema versions
expected = struct.unpack_from("<I", b, o)[0]
acks = d.get("client_acks", [])
ready = d.get("ready_count", 0)
echo_all = bool(acks) and all(a.get("echo_match") is True for a in acks)
resolved = [a.get("entities_resolved") for a in acks]
# The sync ramp is physical (spawn + replication latency after run
# start), so the bar is: the ramp COMPLETES — >=2 acks at the full
# count and the final ack at the full count.
full = [r for r in resolved if r == expected]
ok = (len(acks) >= 2 and echo_all and ready >= 1
      and len(full) >= 2 and resolved and resolved[-1] == expected)
print(f"ACK VERDICT acks={len(acks)} ready_count={ready} expected={expected} "
      f"echo_all={echo_all} resolved={resolved} => {'GREEN' if ok else 'RED'}")
sys.exit(0 if ok else 1)
PYEOF
RCB=$?
[ $RCA -eq 0 ] && [ $RCB -eq 0 ] && { echo "W3 INTEGRATED VERDICT GREEN"; exit 0; }
echo "W3 INTEGRATED VERDICT RED (neutrality=$RCA acks=$RCB)"
exit 1
