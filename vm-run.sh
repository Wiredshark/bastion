#!/bin/sh
# vm-run.sh — run a bastion-harness scenario ON the GCP remote box, ON-DEMAND.
# Auto-starts the VM if it's stopped, waits for sshd, pulls latest, builds (incremental),
# runs the scenario, streams the result back. The VM self-stops ~15 min after this finishes
# (idle watcher: /etc/cron.d/vm-idle-stop). See readme/BUILD-AND-TEST-PROCESS.md §3.
#
# Usage (from the builder's session):
#   bash /e/veloren-master/vm-run.sh --mine-fidelity-scenario --mf-minutes 10
#   bash /e/veloren-master/vm-run.sh --dig-access-scenario
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
INSTANCE=instance-20260719-131242
ZONE=us-central1-a
HOST=benshumeyko@34.9.50.247
KEY="$HOME/.ssh/id_ed25519"

status=$("$GCLOUD" compute instances describe "$INSTANCE" --zone="$ZONE" --format="value(status)" 2>/dev/null || echo UNKNOWN)
if [ "$status" != "RUNNING" ]; then
  echo "[vm-run] VM is $status — starting it..."
  "$GCLOUD" compute instances start "$INSTANCE" --zone="$ZONE" >/dev/null 2>&1
fi

echo "[vm-run] waiting for sshd..."
i=0
while [ "$i" -lt 25 ]; do
  if ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "$HOST" true 2>/dev/null; then break; fi
  i=$((i + 1)); sleep 4
done

echo "[vm-run] running on VM: $*"
ssh -i "$KEY" -o StrictHostKeyChecking=no "$HOST" \
  "source \$HOME/.cargo/env; cd ~/bastion && git pull -q && flock /tmp/bastion-build.lock cargo build --profile verify -p bastion-harness -q && ./target/verify/bastion-harness $*"
echo "[vm-run] scenario done — STOPPING VM now (no idle burn)..."
"$GCLOUD" compute instances stop "$INSTANCE" --zone="$ZONE" >/dev/null 2>&1 || true
echo "[vm-run] VM stopped. (the idle-cron is the backstop if a run is ever interrupted before this line.)"
