#!/bin/sh
# vm-pool.sh — ELASTIC corpus runner. Spins up N ephemeral clones from the golden image,
# runs a scenario on each with a distinct --seed IN PARALLEL, collects the JSON results,
# and DELETES every clone. Cost = only the burst minutes.
#
# CORRECTNESS: each clone git-pulls latest on boot, so it always runs CURRENT code regardless
# of how old the golden image is (the image only provides the toolchain + warm build baseline).
#
# Usage:  bash vm-pool.sh <N> <first-seed> "<harness args, NO --seed>"
#   e.g.  bash vm-pool.sh 5 1000 "--mine-fidelity-scenario --mf-minutes 10"
#         -> 5 clones, seeds 1000..1004, results in /tmp/pool-results/<name>.json
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a
IMAGE=bastion-golden
MACHINE=e2-highmem-8
KEY="$HOME/.ssh/id_ed25519"
SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"   # each clone gets the key in its own metadata
N="$1"; FIRST_SEED="$2"; ARGS="$3"
OUT=/tmp/pool-results; mkdir -p "$OUT"; rm -f "$OUT"/*.json "$OUT"/*.log 2>/dev/null || true

run_one() {
  idx="$1"; seed="$2"; name="bastion-pool-$idx"; log="$OUT/$name.log"
  echo "[pool] create $name (seed $seed)" | tee "$log"
  "$GCLOUD" compute instances create "$name" --zone="$ZONE" --source-machine-image="$IMAGE" --machine-type="$MACHINE" --metadata-from-file=ssh-keys="$SSHKEYS_FILE" >>"$log" 2>&1
  ip=$("$GCLOUD" compute instances describe "$name" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
  i=0; while [ "$i" -lt 30 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$ip" true 2>/dev/null && break; i=$((i+1)); sleep 4; done
  ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$ip" \
    "source \$HOME/.cargo/env; cd ~/bastion && git pull -q && cargo build --profile verify -p bastion-harness -q && ./target/verify/bastion-harness $ARGS --seed $seed" \
    > "$OUT/$name.json" 2>>"$log" || echo "[pool] $name FAILED (see $log)"
  "$GCLOUD" compute instances delete "$name" --zone="$ZONE" -q >>"$log" 2>&1
  echo "[pool] $name done + deleted"
}

echo "[pool] launching $N clones, seeds $FIRST_SEED..$((FIRST_SEED + N - 1))"
i=0; while [ "$i" -lt "$N" ]; do run_one "$i" "$((FIRST_SEED + i))" & i=$((i + 1)); done
wait 2>/dev/null || true
echo "=== POOL DONE — per-seed results: ==="
ls -la "$OUT"/*.json 2>/dev/null || echo "(no results — check $OUT/*.log)"
