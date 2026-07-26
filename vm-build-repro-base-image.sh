#!/usr/bin/env bash
# APEX-T1.1.08 — build the SOURCE-NEUTRAL reproducibility base image
# `bastion-repro-base-v1` (BUILD_LANE=APEX-NIX-V1 infrastructure).
#
# Deliberately contains: Nix (pinned installer, digest-verified), git, git-lfs,
# CA certs, ssh tooling. Deliberately contains NO Bastion checkout, NO warm
# target/, NO sccache, NO rustup, NO branch stamp — environment only. The warm
# `bastion-golden` image is a SEPARATE lane (BUILD_LANE=FAST-NONCERTIFIED) and
# is not touched by this script.
#
# Every mutable input is pinned + recorded: exact source IMAGE (not a family),
# exact Nix installer version + sha256. Terminals:
#   T1.1-REPRO-BASE-READY | T1.1-BLOCK-IMAGE-CONTAMINATION | T1.1-BLOCK-INSTALLER-DIGEST
set -euo pipefail
echo "BUILD_LANE=APEX-NIX-V1"

PROJECT="${GCP_PROJECT:?set GCP_PROJECT}"
ZONE="${GCP_ZONE:-us-central1-a}"
VM="apex-repro-base-build"
IMAGE_NAME="bastion-repro-base-v1"
# EXACT image, not a family (a family is a moving ref — same class of bug as
# building from a moving branch).
SOURCE_IMAGE="${REPRO_SOURCE_IMAGE:?set REPRO_SOURCE_IMAGE to an exact debian-12 image name, e.g. debian-12-bookworm-v20260701}"
SOURCE_IMAGE_PROJECT="${REPRO_SOURCE_IMAGE_PROJECT:-debian-cloud}"
# Pinned Nix installer (record + verify — never curl|sh an unpinned installer).
NIX_VERSION="${NIX_VERSION:?set NIX_VERSION, e.g. 2.24.9}"
NIX_INSTALLER_SHA256="${NIX_INSTALLER_SHA256:?set NIX_INSTALLER_SHA256 for install-nix-${NIX_VERSION}}"

echo "source_image=${SOURCE_IMAGE_PROJECT}/${SOURCE_IMAGE}"
echo "nix_version=${NIX_VERSION} installer_sha256=${NIX_INSTALLER_SHA256}"

gcloud compute instances create "$VM" \
  --project="$PROJECT" --zone="$ZONE" \
  --machine-type=e2-standard-4 \
  --image="$SOURCE_IMAGE" --image-project="$SOURCE_IMAGE_PROJECT" \
  --boot-disk-size=64GB --boot-disk-type=pd-ssd

trap 'gcloud compute instances delete "$VM" --project="$PROJECT" --zone="$ZONE" --quiet || true' EXIT

gcloud compute ssh "$VM" --project="$PROJECT" --zone="$ZONE" --command="$(cat <<REMOTE
set -euo pipefail
sudo apt-get update -q
sudo DEBIAN_FRONTEND=noninteractive apt-get install -qy git git-lfs ca-certificates curl xz-utils
# Pinned, digest-verified Nix installer (multi-user daemon install).
curl -fsSL "https://releases.nixos.org/nix/nix-${NIX_VERSION}/install" -o /tmp/install-nix
echo "${NIX_INSTALLER_SHA256}  /tmp/install-nix" | sha256sum -c - || { echo "TERMINAL: T1.1-BLOCK-INSTALLER-DIGEST"; exit 9; }
sh /tmp/install-nix --daemon --yes
# Flakes on; no channels (channels are a moving ref).
echo 'experimental-features = nix-command flakes' | sudo tee -a /etc/nix/nix.conf
sudo rm -rf /root/.nix-channels /home/*/.nix-channels || true
. /etc/profile.d/nix.sh 2>/dev/null || true
nix --version
# ── CONTAMINATION SCAN (T1.1-BLOCK-IMAGE-CONTAMINATION) ──────────────────────
FORBIDDEN=0
for p in /root/bastion /home/*/bastion /root/veloren* /home/*/veloren*; do
  [ -e "\$p" ] && { echo "FORBIDDEN: \$p"; FORBIDDEN=1; }
done
command -v rustup >/dev/null 2>&1 && { echo "FORBIDDEN: rustup on base image"; FORBIDDEN=1; }
command -v sccache >/dev/null 2>&1 && { echo "FORBIDDEN: sccache on base image"; FORBIDDEN=1; }
find / -maxdepth 4 -name 'target' -path '*bastion*' 2>/dev/null | grep -q . && { echo "FORBIDDEN: warm target dir"; FORBIDDEN=1; }
[ -e /etc/bastion-branch-stamp ] && { echo "FORBIDDEN: branch stamp"; FORBIDDEN=1; }
[ "\$FORBIDDEN" -eq 0 ] || { echo "TERMINAL: T1.1-BLOCK-IMAGE-CONTAMINATION"; exit 8; }
echo "CONTAMINATION-SCAN: clean"
REMOTE
)"

gcloud compute instances stop "$VM" --project="$PROJECT" --zone="$ZONE" --quiet
gcloud compute machine-images create "$IMAGE_NAME" \
  --project="$PROJECT" --source-instance="$VM" --source-instance-zone="$ZONE" \
  || { echo "machine image may already exist: $IMAGE_NAME"; }

echo "image=$IMAGE_NAME"
echo "TERMINAL: T1.1-REPRO-BASE-READY"
