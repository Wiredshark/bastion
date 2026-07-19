#!/bin/sh
# vm-scale.sh — SCALE-UP burst: resize the on-demand VM to a bigger machine type, run a corpus
# command on it (build ONCE, N parallel seed-processes), then resize back to the default + stop.
# This is the EFFICIENT path for a single-provider corpus (no redundant per-clone builds). The
# clone POOL (vm-pool.sh) is only for multi-provider / >one-machine overflow. Decision tree: §13.
#
# Usage:  bash vm-scale.sh <machine-type> "<remote command>"
#   e.g.  bash vm-scale.sh e2-highmem-16 \
#           "source \$HOME/.cargo/env; cd ~/bastion && git pull -q && cargo build --profile verify -p bastion-harness -q && ./target/verify/bastion-harness --corpus mine-fidelity --corpus-jobs 16"
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
INSTANCE=instance-20260719-131242; ZONE=us-central1-a; HOST=benshumeyko@34.9.50.247; KEY="$HOME/.ssh/id_ed25519"
BIG="$1"; CMD="$2"; DEFAULT=e2-highmem-8

echo "[scale] stop -> resize to $BIG -> start"
"$GCLOUD" compute instances stop "$INSTANCE" --zone="$ZONE" >/dev/null 2>&1 || true
"$GCLOUD" compute instances set-machine-type "$INSTANCE" --zone="$ZONE" --machine-type="$BIG"
"$GCLOUD" compute instances start "$INSTANCE" --zone="$ZONE" >/dev/null 2>&1
i=0; while [ "$i" -lt 25 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "$HOST" true 2>/dev/null && break; i=$((i + 1)); sleep 4; done

echo "[scale] running on $BIG ..."
ssh -i "$KEY" -o StrictHostKeyChecking=no "$HOST" "$CMD"

echo "[scale] resize back to $DEFAULT + stop"
"$GCLOUD" compute instances stop "$INSTANCE" --zone="$ZONE" >/dev/null 2>&1
"$GCLOUD" compute instances set-machine-type "$INSTANCE" --zone="$ZONE" --machine-type="$DEFAULT"
echo "=== SCALE DONE — VM back to $DEFAULT, stopped (self-manages) ==="
