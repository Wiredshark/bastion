#!/usr/bin/env bash
# W5 ops residual: the UNATTENDED capture flow, as a durable runner.
# Launches the self-starting flat-arena session with the r0d capture
# armed, automates the pause keystroke (Space -> BastionPauseToggle)
# once the session is live, and watches the conjunct witness inside
# observe_visible_scene (change-logged) plus the capture gate telemetry.
# Bounded; identity-scoped kill; evidence under smoke/w5b/.
set -u
WT="E:/veloren-master/.renderer-wt"
B="$WT/target/debug"
A="$WT/assets"
EV="$WT/readme/renderer-bench/smoke/w5b"
SECS="${SECS:-300}"
rm -rf "$EV"; mkdir -p "$EV/captures" "$EV/userdata/voxygen"

echo "PRECONDITION voxygen=$(sha256sum "$B/veloren-voxygen.exe" | cut -c1-16) tip=$(cd "$WT" && git rev-parse --short=10 HEAD)"

( cd "$WT" && VELOREN_USERDATA="$EV/userdata" VOXYGEN_CONFIG="$EV/userdata/voxygen" \
    VELOREN_ASSETS="$A" \
    BASTION_R0D_CAPTURE_OUT="$EV/captures" \
    BASTION_R0D_CAPTURE_WARMUP=120 BASTION_R0D_CAPTURE_COUNT=3 \
    BASTION_R1BC_FIGURE_COUNT=1 \
    BASTION_R0D_AUTOPAUSE=1 \
    BASTION_R0D_FREEZE_AFTER_LOGIN=1 \
    "$B/veloren-voxygen.exe" --bastion-flat-arena > "$EV/voxygen.log" 2>&1 ) &
PID=$!
echo "voxygen pid=$PID (${SECS}s budget)"

# Wait for the session to go live (the capture-gate telemetry line),
# then automate the pause keystroke the flow requires.
# Liveness via WINDOWS, not bash pids (Git Bash kill -0 on a subshell
# pid lies about child processes — it voided a healthy boot once; the
# CIM CommandLine probe ALSO returned a false 0 once, so this probe
# uses Get-Process + Path and LOGS its raw value every check).
alive() {
  powershell -NoProfile -Command "@(Get-Process veloren-voxygen -ErrorAction SilentlyContinue | Where-Object { \$_.Path -like '*renderer-wt*' }).Count" | tr -d '\r\n '
}
t=0
while [ $t -lt 240 ]; do
  if grep -aq "capture gate state" "$EV/voxygen.log" 2>/dev/null; then break; fi
  a=$(alive)
  echo "boot-probe t=${t}s alive='${a}'"
  if [ "$a" = "0" ] && [ $t -gt 30 ]; then echo "voxygen EXITED during boot — leg VOID"; exit 2; fi
  sleep 10; t=$((t+10))
done
if ! grep -aq "capture gate state" "$EV/voxygen.log" 2>/dev/null; then
  echo "session never reached capture gate after ${t}s — leg VOID"
  taskkill //F //PID "$PID" >/dev/null 2>&1; exit 2
fi
# Pause is handled IN-ENGINE (BASTION_R0D_AUTOPAUSE) — no keystroke,
# no focus dependency, canary window-steals can't break the flow.

# Watch for either captures arriving or the run budget lapsing.
t=0
while [ $t -lt "$SECS" ]; do
  if ls "$EV/captures/" 2>/dev/null | grep -q . ; then echo "CAPTURES ARRIVED at +${t}s"; break; fi
  if [ "$(alive)" = "0" ]; then echo "voxygen EXITED at +${t}s"; break; fi
  sleep 10; t=$((t+10))
done
taskkill //F //PID "$PID" >/dev/null 2>&1; wait "$PID" 2>/dev/null
powershell -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='veloren-voxygen.exe'\" | Where-Object { \$_.CommandLine -match 'renderer-wt' } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force }" >/dev/null 2>&1

echo "=== conjunct witness (change log) ==="
grep -a "observe_visible_scene conjuncts" "$EV/voxygen.log" | tail -8
echo "=== final capture gate ==="
grep -a "capture gate state" "$EV/voxygen.log" | tail -2
CAPS=$(ls "$EV/captures/" 2>/dev/null | wc -l)
echo "captures=$CAPS"
[ "$CAPS" -gt 0 ] && { echo "W5 UNATTENDED FLOW GREEN"; exit 0; }
echo "W5 UNATTENDED FLOW RED — read the conjunct witness above"
exit 1
