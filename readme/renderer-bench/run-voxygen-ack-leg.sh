#!/usr/bin/env bash
# W4 voxygen ack leg: ONE self-contained process — the embedded
# singleplayer server runs the bench (WAIT_CLIENT), the voxygen client
# auto-spectates (silent_spectator), sends readiness, and acks every
# announce carrying the PassDraw/VisualStructure domains from its own
# renderer. GPU session (Ben-authorized standing).
set -u
WT="E:/veloren-master/.renderer-wt"
B="$WT/target/debug"
A="$WT/assets"
FIX="$WT/readme/renderer-bench/fixtures/walk-and-seek-v2.rbdm"
EV="$WT/readme/renderer-bench/smoke/w4"
mkdir -p "$EV"
rm -rf "$EV/userdata" "$EV/tape-vox.json"
mkdir -p "$EV/userdata/voxygen"

echo "PRECONDITION voxygen=$(sha256sum "$B/veloren-voxygen.exe" | cut -c1-16) fixture=$(sha256sum "$FIX" | cut -c1-16) tip=$(cd "$WT" && git rev-parse --short=10 HEAD)"

( cd "$WT" && VELOREN_USERDATA="$EV/userdata" VOXYGEN_CONFIG="$EV/userdata/voxygen" \
    VELOREN_ASSETS="$A" VELOREN_CLIENT_TYPE=silent_spectator \
    BASTION_DETERMINISTIC=1 \
    BASTION_RENDERER_BENCH_MANIFEST="$FIX" \
    BASTION_RENDERER_BENCH_OUT="$EV/tape-vox.json" \
    BASTION_RENDERER_BENCH_TICKS=600 \
    BASTION_RENDERER_BENCH_CADENCE=30 \
    BASTION_RENDERER_BENCH_WAIT_CLIENT=1 \
    BASTION_RENDERER_BENCH_ACK=1 \
    BASTION_POST_R2_STREAMING_MEASUREMENT=continuous-server-v1 \
    "$B/veloren-voxygen.exe" --bastion-flat-arena > "$EV/voxygen.log" 2>&1 ) &
PID=$!
t=0
while [ $t -lt 420 ]; do
  if [ -s "$EV/tape-vox.json" ]; then break; fi
  sleep 5; t=$((t+5))
done
powershell -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='veloren-voxygen.exe'\" | Where-Object { \$_.CommandLine -match 'renderer-wt' } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force }" >/dev/null 2>&1
wait "$PID" 2>/dev/null
if [ ! -s "$EV/tape-vox.json" ]; then
  echo "TAPE MISSING after ${t}s — leg VOID (see voxygen.log)"
  exit 1
fi
echo "TAPE ARRIVED after ~${t}s"

python - "$EV/tape-vox.json" "$FIX" <<'PYEOF'
import json, struct, sys
d = json.load(open(sys.argv[1]))
b = open(sys.argv[2], "rb").read()
o = 8
slen = struct.unpack_from("<I", b, o)[0]; o += 4 + slen
o += 8 * 3 + 4 + 4 * 3
slen = struct.unpack_from("<I", b, o)[0]; o += 4 + slen
o += 4 + 4
expected = struct.unpack_from("<I", b, o)[0]
acks = d.get("client_acks", [])
echo_all = bool(acks) and all(a.get("echo_match") is True for a in acks)
resolved = [a.get("entities_resolved") for a in acks]
vis = [a for a in acks if "pass_draw_root" in a and "visual_structure_root" in a]
# Settled-tail stability: on the static spectator scene the visual roots
# must repeat (the whole point — a stable scene has a stable fingerprint).
tail = vis[len(vis)//2:]
pd_stable = len({a["pass_draw_root"] for a in tail}) == 1 if tail else False
full = [r for r in resolved if r == expected]
ok = (len(acks) >= 2 and echo_all and len(full) >= 2 and resolved and resolved[-1] == expected
      and len(vis) >= 2 and pd_stable)
print(f"W4 ACK VERDICT acks={len(acks)} echo_all={echo_all} resolved={resolved} "
      f"visual_present={len(vis)}/{len(acks)} pd_tail_stable={pd_stable} => {'GREEN' if ok else 'RED'}")
if vis:
    print(f"sample pass_draw_root={vis[-1]['pass_draw_root'][:16]} visual_structure_root={vis[-1]['visual_structure_root'][:16]}")
sys.exit(0 if ok else 1)
PYEOF
exit $?
