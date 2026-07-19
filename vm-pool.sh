#!/bin/sh
# vm-pool.sh — EPHEMERAL POOL / BREADTH runner WITH A LIVE BURN-GUARD. Creates N VMs from the golden
# image in parallel, each builds once + runs <seeds_per_vm> harness seeds, then EVERY VM is deleted.
# A background guard meters cost LIVE (total_vCPU x elapsed x rate — no billing API needed) and CUTS
# THE WHOLE RUN OFF if it exceeds $MAX_USD or $MAX_MIN. On cutoff it exits 42 so a caller can retry
# smaller (see vm-pool-safe.sh). Guaranteed teardown via trap; nothing is ever left billing.
#
# Usage: bash vm-pool.sh <N_vms> <machine_type> <seeds_per_vm> <first_seed> "<args>" [MAX_USD] [MAX_MIN]
#   e.g. bash vm-pool.sh 8 e2-standard-4 4 1000 "--mine-fidelity-scenario --mf-minutes 5" 5 30
# NOTE: keep N*vCPU BELOW the CPUS_ALL_REGIONS quota (leave headroom — scheduling to the exact cap
# bounces creates). vCPU per VM = the trailing number of the machine type (e2-standard-4 -> 4).
set -u
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a; IMAGE=bastion-golden; KEY="$HOME/.ssh/id_ed25519"; SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"
BRANCH=bastion/builder   # every VM lands EXACTLY on this branch's remote tip (validated per-run)
N="$1"; MACHINE="$2"; SPV="$3"; FIRST="$4"; ARGS="$5"; MAX_USD="${6:-5}"; MAX_MIN="${7:-30}"
VCPU_PER=$(echo "$MACHINE" | sed 's/.*-//'); TOTAL_VCPU=$((N * VCPU_PER)); RATE=0.035  # $/vCPU-hr, conservative
OUT=/tmp/bastion-pool; mkdir -p "$OUT"; rm -f "$OUT"/*.log "$OUT"/TRIPPED 2>/dev/null || true

cleanup() { k=0; while [ "$k" -lt "$N" ]; do "$GCLOUD" compute instances delete "bastion-pool-$k" --zone="$ZONE" -q >/dev/null 2>&1 & k=$((k+1)); done; wait; }
trap 'cleanup' EXIT INT TERM

run_one() {
  k="$1"; name="bastion-pool-$k"; base=$((FIRST + k*SPV)); cerr=""
  tries=0  # retry w/ backoff — transient quota bounces from racing a prior batch's teardown
  until cerr=$("$GCLOUD" compute instances create "$name" --zone="$ZONE" --source-machine-image="$IMAGE" \
        --machine-type="$MACHINE" --metadata-from-file=ssh-keys="$SSHKEYS_FILE" 2>&1 >/dev/null); do
    tries=$((tries + 1)); [ "$tries" -ge 4 ] && { echo "CREATE_FAIL :: ${cerr##*ERROR: }"; return; }
    sleep $((tries * 15))
  done
  ip=$("$GCLOUD" compute instances describe "$name" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
  i=0; while [ "$i" -lt 45 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$ip" true 2>/dev/null && break; i=$((i+1)); sleep 4; done
  ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$ip" "
    source \$HOME/.cargo/env; cd ~/bastion
    git fetch -q origin && git reset --hard -q origin/$BRANCH
    H=\$(git rev-parse --short HEAD); R=\$(git rev-parse --short origin/$BRANCH)
    [ \"\$H\" = \"\$R\" ] && echo COMMIT=\$H || { echo STALE=\$H/\$R; exit 3; }
    cargo build --profile verify -p bastion-harness -q || { echo BUILD_FAIL@\$H; exit 4; }  # NEVER fall through to a stale binary
    for s in \$(seq $base $((base + SPV - 1))); do
      ./target/verify/bastion-harness $ARGS --seed \$s --data-dir /tmp/mf-\$s >/tmp/mf-\$s.json 2>/dev/null &
    done; wait
    echo DONE=\$(ls /tmp/mf-*.json 2>/dev/null | wc -l)"
  "$GCLOUD" compute instances delete "$name" --zone="$ZONE" -q >/dev/null 2>&1
}

# --- live burn-guard: prints cost every 90s, cuts the whole run off at the ceiling ---
guard() {
  gstart="$1"; acc=0; echo 0 > "$OUT/COST"
  while :; do
    sleep 90
    up=$("$GCLOUD" compute instances list --filter="name~^bastion-pool" --format="value(name)" 2>/dev/null | wc -l)
    acc=$(awk "BEGIN{printf \"%.3f\", $acc + $up*$VCPU_PER*(90/3600.0)*$RATE}")  # true VM-time, not TOTAL_VCPU-whole-run
    echo "$acc" > "$OUT/COST"
    el_m=$(( ($(date +%s) - gstart) / 60 ))
    echo "[guard] ${el_m}m | ~\$$acc (actual VM-time) | $up VMs up | ceiling \$$MAX_USD / ${MAX_MIN}m"
    if [ "$el_m" -ge "$MAX_MIN" ] || [ "$(awk "BEGIN{print ($acc>=$MAX_USD)?1:0}")" = 1 ]; then
      echo "[guard] *** CEILING HIT (${el_m}m / ~\$$acc) — CUTTING OFF THE RUN ***"
      : > "$OUT/TRIPPED"; cleanup; return
    fi
  done
}

echo "[pool] $N x $MACHINE ($TOTAL_VCPU vCPU), $SPV seeds each = $((N*SPV)) total. Ceiling \$$MAX_USD / ${MAX_MIN}m. Launching..."
start=$(date +%s)
guard "$start" & GUARD_PID=$!
k=0; PIDS=""; while [ "$k" -lt "$N" ]; do run_one "$k" > "$OUT/bastion-pool-$k.log" 2>&1 & PIDS="$PIDS $!"; k=$((k + 1)); done
for p in $PIDS; do wait "$p" 2>/dev/null; done   # wait ONLY the run_one workers, never the guard
kill "$GUARD_PID" 2>/dev/null; wait "$GUARD_PID" 2>/dev/null || true
end=$(date +%s)
total=$(grep -h '^DONE=' "$OUT"/*.log 2>/dev/null | sed 's/DONE=//' | awk '{s+=$1} END{print s+0}')
fails=$(grep -h 'CREATE_FAIL' "$OUT"/*.log 2>/dev/null | wc -l)
cost=$(cat "$OUT/COST" 2>/dev/null || echo 0)
if [ -f "$OUT/TRIPPED" ]; then
  echo "=== POOL CUT OFF at ceiling after $((end-start))s (~\$$cost actual). Rerun smaller. ==="
  exit 42
fi
echo "=== POOL DONE in $((end-start))s | ~\$$cost burned (actual VM-time) | $total/$((N*SPV)) seeds across $N VMs ($fails create-fails) ==="
grep -H 'DONE=\|CREATE_FAIL\|COMMIT=' "$OUT"/*.log 2>/dev/null | head -40 || true
