#!/bin/sh
# vm-build-image.sh — rebuild bastion-golden on a SMALL 30GB disk from a clean Debian 13 base.
# Faithful reproduction of the golden env (introspected 2026-07-19): apt build deps + rustup +
# sccache + mold, clone Wiredshark/bastion @ bastion/builder, cold-build the harness to warm
# target+sccache, then re-image. Deletes the OLD 200GB golden ONLY after the new build verifies,
# so there is always a working fallback. 30GB disk => ~16 concurrent VMs under the 500GB SSD quota.
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a; KEY="$HOME/.ssh/id_ed25519"; SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"
BUILD_VM=bastion-imgbuild; DISK=30; MACHINE=e2-standard-32
trap 'st=$?; if [ $st -ne 0 ]; then echo "[img] FAILED (rc=$st) — leaving old golden intact; deleting build VM"; "$GCLOUD" compute instances delete "$BUILD_VM" --zone="$ZONE" -q >/dev/null 2>&1 || true; fi' EXIT

echo "[img] creating $BUILD_VM (Debian 13, ${DISK}GB pd-ssd, $MACHINE)..."
"$GCLOUD" compute instances create "$BUILD_VM" --zone="$ZONE" \
  --image-family=debian-13 --image-project=debian-cloud \
  --boot-disk-size="${DISK}GB" --boot-disk-type=pd-ssd \
  --machine-type="$MACHINE" --metadata-from-file=ssh-keys="$SSHKEYS_FILE" >/dev/null
IP=$("$GCLOUD" compute instances describe "$BUILD_VM" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
echo "[img] $BUILD_VM @ $IP — waiting for sshd..."
i=0; while [ "$i" -lt 45 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$IP" true 2>/dev/null && break; i=$((i + 1)); sleep 4; done

echo "[img] running setup (apt + rustup + sccache + clone + COLD build)..."
ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$IP" 'bash -s' <<'SETUP'
set -e
export DEBIAN_FRONTEND=noninteractive
sudo apt-get update -q
sudo apt-get install -y -q build-essential clang cmake git git-lfs libssl-dev pkg-config lld mold curl screen cloud-guest-utils
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
. "$HOME/.cargo/env"
cargo install sccache --locked
git clone --branch bastion/builder https://github.com/Wiredshark/bastion.git "$HOME/bastion"
cd "$HOME/bastion"
cargo build --profile verify -p bastion-harness
test -x target/verify/bastion-harness && echo "BUILD_OK"
echo "--- footprint ---"; du -sh "$HOME"; df -h / | tail -1
SETUP

echo "[img] build verified — stopping VM for a consistent image..."
"$GCLOUD" compute instances stop "$BUILD_VM" --zone="$ZONE" >/dev/null 2>&1
echo "[img] swapping golden: delete old 200GB image, create new ${DISK}GB image (same name)..."
"$GCLOUD" compute machine-images delete bastion-golden -q >/dev/null 2>&1 || true
"$GCLOUD" compute machine-images create bastion-golden --source-instance="$BUILD_VM" --source-instance-zone="$ZONE" >/dev/null
echo "[img] deleting build VM..."
"$GCLOUD" compute instances delete "$BUILD_VM" --zone="$ZONE" -q >/dev/null 2>&1 || true
echo "=== IMAGE REBUILD DONE — bastion-golden is now ${DISK}GB (~16 VMs fit the 500GB quota) ==="
