#!/bin/sh
# vm-run.sh — EPHEMERAL single-run. Creates a FRESH VM from the golden image, pulls latest,
# builds (incremental — the image ships a warm target+sccache), runs the scenario, streams the
# result back, then DELETES the VM. Nothing persists; nothing idles; zero standing compute cost.
# The trap guarantees deletion even on error/Ctrl-C. See readme/BUILD-AND-TEST-PROCESS.md §3/§11.
#
# Usage (from the builder's session):
#   bash /e/veloren-master/vm-run.sh --mine-fidelity-scenario --mf-minutes 10
#   bash /e/veloren-master/vm-run.sh --dig-access-scenario
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a; IMAGE=bastion-golden; MACHINE=e2-highmem-8
KEY="$HOME/.ssh/id_ed25519"; SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"
BRANCH=bastion/builder   # the builder's push branch — the VM lands EXACTLY on its remote tip
NAME="bastion-run-$$"
trap '"$GCLOUD" compute instances delete "$NAME" --zone="$ZONE" -q >/dev/null 2>&1 || true' EXIT INT TERM

echo "[vm-run] creating ephemeral $MACHINE from $IMAGE ..."
"$GCLOUD" compute instances create "$NAME" --zone="$ZONE" --source-machine-image="$IMAGE" --machine-type="$MACHINE" --metadata-from-file=ssh-keys="$SSHKEYS_FILE" >/dev/null 2>&1
IP=$("$GCLOUD" compute instances describe "$NAME" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
echo "[vm-run] $NAME @ $IP — waiting for sshd..."
i=0; while [ "$i" -lt 40 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$IP" true 2>/dev/null && break; i=$((i + 1)); sleep 4; done

echo "[vm-run] sync to latest origin/$BRANCH + build + run: $*"
ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$IP" \
  "source \$HOME/.cargo/env; cd ~/bastion \
   && git fetch -q origin && git reset --hard -q origin/$BRANCH \
   && H=\$(git rev-parse --short HEAD); R=\$(git rev-parse --short origin/$BRANCH) \
   && [ \"\$H\" = \"\$R\" ] && echo \"RAN_COMMIT=\$H  (== latest origin/$BRANCH — validated)\" || { echo \"STALE: HEAD \$H != origin \$R\"; exit 3; } \
   && cargo build --profile verify -p bastion-harness -q \
   && ./target/verify/bastion-harness $*"
echo "[vm-run] done — deleting VM (trap also guarantees this)..."
"$GCLOUD" compute instances delete "$NAME" --zone="$ZONE" -q >/dev/null 2>&1 || true
echo "[vm-run] VM gone. Zero standing cost."
