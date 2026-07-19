#!/bin/sh
# x2-pair.sh — R10 Opus-gate condition: same-seed ×2 byte-consistency pair on ONE quiet
# ephemeral VM (vm-run.sh's exact lifecycle, payload = two recorder runs + in-place compare).
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a; IMAGE=bastion-golden; MACHINE=e2-highmem-8
KEY="$HOME/.ssh/id_ed25519"; SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"
BRANCH=bastion/builder
NAME="bastion-x2-$$"
trap '"$GCLOUD" compute instances delete "$NAME" --zone="$ZONE" -q >/dev/null 2>&1 || true' EXIT INT TERM

echo "[x2] creating ephemeral $MACHINE from $IMAGE ..."
"$GCLOUD" compute instances create "$NAME" --zone="$ZONE" --source-machine-image="$IMAGE" --machine-type="$MACHINE" --metadata-from-file=ssh-keys="$SSHKEYS_FILE" >/dev/null 2>&1
IP=$("$GCLOUD" compute instances describe "$NAME" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
echo "[x2] $NAME @ $IP — waiting for sshd..."
i=0; while [ "$i" -lt 40 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$IP" true 2>/dev/null && break; i=$((i + 1)); sleep 4; done

ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$IP" '
  source $HOME/.cargo/env; cd ~/bastion
  git fetch -q origin && git reset --hard -q origin/bastion/builder
  H=$(git rev-parse --short HEAD)
  cargo build --profile verify -p bastion-harness -q || { echo BUILD_FAIL; exit 4; }
  rm -rf /tmp/x2-1 /tmp/x2-2
  for i in 1 2; do
    BASTION_FLIGHT_RECORDER_DIR=/tmp/x2-$i ./target/verify/bastion-harness --seed 1337 --b58-ladder-integration-fixture --ladder-episode P0 > /tmp/x2-run$i.out 2>/dev/null
    echo "run$i rc=$?"
  done
  verdict=IDENTICAL
  grep -a "^{" /tmp/x2-run1.out > /tmp/x2-j1; grep -a "^{" /tmp/x2-run2.out > /tmp/x2-j2
  cmp -s /tmp/x2-j1 /tmp/x2-j2 || verdict=JSON-DIVERGED
  for f in /tmp/x2-1/*; do
    b=$(basename "$f"); [ "$b" = "metadata.json" ] && continue
    [ -f "/tmp/x2-2/$b" ] || { verdict="MISSING-$b"; continue; }
    sed "s/\"wall_unix_millis\":[0-9]*/\"wall_unix_millis\":0/g" "$f" > /tmp/xa
    sed "s/\"wall_unix_millis\":[0-9]*/\"wall_unix_millis\":0/g" "/tmp/x2-2/$b" > /tmp/xb
    if cmp -s /tmp/xa /tmp/xb; then echo "compared $b: identical"; else verdict="DIVERGED-$b"; echo "compared $b: DIVERGED"; fi
  done
  echo "X2-VERDICT: $verdict"
  echo "=== ATTEST (end): RAN_COMMIT=$H | verdict=$verdict ==="
'
echo "[x2] deleting VM..."
"$GCLOUD" compute instances delete "$NAME" --zone="$ZONE" -q >/dev/null 2>&1 || true
echo "[x2] VM gone."
