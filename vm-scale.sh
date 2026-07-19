#!/bin/sh
# vm-scale.sh — EPHEMERAL SCALE-UP: ONE big VM runs N seeds of ONE scenario IN PARALLEL. Fastest for a
# fixed core budget (one build, N cores — no per-VM build overhead like the pool). Validates the commit,
# burn-guarded, self-deletes. This is the DEFAULT for a same-scenario corpus; use vm-jobs.sh for DIFFERENT
# tests (breadth) and vm-pool.sh only when you need more cores than one VM holds.
#
# Usage: bash vm-scale.sh <machine_type> <N_seeds> <first_seed> "<scenario args>" [MAX_USD] [MAX_MIN]
#   e.g. bash vm-scale.sh e2-standard-32 24 1000 "--mine-fidelity-scenario --mf-minutes 5" 5 20
set -u
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a; IMAGE=bastion-golden; KEY="$HOME/.ssh/id_ed25519"; SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"
BRANCH=bastion/builder
MACHINE="$1"; NSEEDS="$2"; FIRST="$3"; ARGS="$4"; MAX_USD="${5:-5}"; MAX_MIN="${6:-30}"
VCPU=$(echo "$MACHINE" | sed 's/.*-//'); RATE=0.035; NAME="bastion-scale-$$"
trap '"$GCLOUD" compute instances delete "$NAME" --zone="$ZONE" -q >/dev/null 2>&1 || true' EXIT INT TERM

echo "[scale] creating $MACHINE ($VCPU vCPU) ephemeral VM, ceiling \$$MAX_USD/${MAX_MIN}m..."
"$GCLOUD" compute instances create "$NAME" --zone="$ZONE" --source-machine-image="$IMAGE" \
  --machine-type="$MACHINE" --metadata-from-file=ssh-keys="$SSHKEYS_FILE" >/dev/null 2>&1
IP=$("$GCLOUD" compute instances describe "$NAME" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
i=0; while [ "$i" -lt 45 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$IP" true 2>/dev/null && break; i=$((i+1)); sleep 4; done

start=$(date +%s)
# burn-guard: one big VM, so cost = VCPU x time x rate; kill it if it runs past the ceiling
( while :; do sleep 90
    el_s=$(( $(date +%s) - start )); el_m=$(( el_s/60 ))
    est=$(awk "BEGIN{printf \"%.2f\", $VCPU*($el_s/3600.0)*$RATE}")
    echo "[guard] ${el_m}m | ~\$$est | ceiling \$$MAX_USD/${MAX_MIN}m"
    if [ "$el_m" -ge "$MAX_MIN" ] || [ "$(awk "BEGIN{print ($est>=$MAX_USD)?1:0}")" = 1 ]; then
      echo "[guard] *** CEILING HIT — CUTTING OFF ***"; "$GCLOUD" compute instances delete "$NAME" --zone="$ZONE" -q >/dev/null 2>&1; return
    fi
  done ) & GUARD=$!

ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$IP" "
  source \$HOME/.cargo/env; cd ~/bastion
  git fetch -q origin && git reset --hard -q origin/$BRANCH
  H=\$(git rev-parse --short HEAD); R=\$(git rev-parse --short origin/$BRANCH)
  [ \"\$H\" = \"\$R\" ] && echo COMMIT=\$H || { echo STALE=\$H/\$R; exit 3; }
  cargo build --profile verify -p bastion-harness -q
  for s in \$(seq $FIRST $((FIRST + NSEEDS - 1))); do
    ./target/verify/bastion-harness $ARGS --seed \$s --data-dir /tmp/mf-\$s >/tmp/mf-\$s.json 2>/dev/null &
  done; wait
  echo \"=== ATTEST (end): RAN_COMMIT=\$H | DONE=\$(ls /tmp/mf-*.json 2>/dev/null | wc -l)/$NSEEDS ===\""
rc=$?
kill "$GUARD" 2>/dev/null || true
end=$(date +%s)
est=$(awk "BEGIN{printf \"%.2f\", $VCPU*(($end-$start)/3600.0)*$RATE}")
echo "=== SCALE-UP DONE in $((end-start))s | ~\$$est | $NSEEDS seeds on $VCPU cores (rc=$rc) ==="
