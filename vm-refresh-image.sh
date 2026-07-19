#!/bin/sh
# vm-refresh-image.sh — rebuild the golden image so clones boot with a FRESH baseline
# (small git-pull deltas + warm sccache + up-to-date target/). Run after significant merges,
# or nightly via a scheduled task.
#
# NOTE ON "ALWAYS UP TO DATE": correctness is ALWAYS current without this — every clone and the
# on-demand box git-pull latest on boot before running (vm-run.sh / vm-pool.sh). This refresh is
# purely a SPEED optimization: it keeps the image's baseline near HEAD so the boot-time pull is a
# small delta and the incremental build is fast. Stale image = still-correct, just a bigger first pull.
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
INSTANCE=instance-20260719-131242; ZONE=us-central1-a; HOST=benshumeyko@34.9.50.247; KEY="$HOME/.ssh/id_ed25519"

echo "[refresh] starting the golden VM..."
"$GCLOUD" compute instances start "$INSTANCE" --zone="$ZONE" >/dev/null 2>&1 || true
i=0; while [ "$i" -lt 25 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "$HOST" true 2>/dev/null && break; i=$((i + 1)); sleep 4; done
echo "[refresh] pulling latest + warming the build..."
ssh -i "$KEY" -o StrictHostKeyChecking=no "$HOST" "source \$HOME/.cargo/env; cd ~/bastion && git pull -q && cargo build --profile verify -p bastion-harness -q"
echo "[refresh] stopping for a clean snapshot..."
"$GCLOUD" compute instances stop "$INSTANCE" --zone="$ZONE" >/dev/null 2>&1
echo "[refresh] replacing bastion-golden with a fresh snapshot..."
"$GCLOUD" compute machine-images delete bastion-golden -q >/dev/null 2>&1 || true
"$GCLOUD" compute machine-images create bastion-golden --source-instance="$INSTANCE" --source-instance-zone="$ZONE" 2>&1 | tail -1
echo "=== REFRESH DONE — bastion-golden now at $(git rev-parse --short HEAD 2>/dev/null || echo HEAD) baseline ==="
