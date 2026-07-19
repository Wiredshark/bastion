#!/bin/sh
# vm-golden-autorefresh.sh — keep bastion-golden fresh from origin/bastion/builder, INCREMENTALLY + GUARDED.
# Starts from the CURRENT golden (warm build) → git fetch/reset to the tip → incremental build → re-image.
# Fast (~5-8 min, incremental) vs a full cold rebuild (~15-20 min). Keeps each ephemeral run's catch-up
# build small. SKIPS if: (a) the tip hasn't moved since last refresh, or (b) any bastion-* VM is running
# (avoid quota collision with a live run). Schedule nightly:
#   Windows Task Scheduler → cmd /c "bash E:\veloren-master\vm-golden-autorefresh.sh"
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a; IMAGE=bastion-golden; KEY="$HOME/.ssh/id_ed25519"
SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"; BRANCH=bastion/builder
STAMP="/e/veloren-master/.golden-commit"; NAME=bastion-imgrefresh

tip=$(git -C /e/veloren-master ls-remote bastion-origin "refs/heads/$BRANCH" 2>/dev/null | cut -f1)
[ -z "$tip" ] && { echo "[autorefresh] can't read remote tip — skip"; exit 0; }
[ -f "$STAMP" ] && [ "$(cat "$STAMP")" = "$tip" ] && { echo "[autorefresh] golden already at ${tip%${tip#????????}} — skip"; exit 0; }
running=$("$GCLOUD" compute instances list --filter="name~^bastion-" --format="value(name)" 2>/dev/null | wc -l)
[ "$running" -gt 0 ] && { echo "[autorefresh] $running bastion VM(s) active — skip (avoid quota collision)"; exit 0; }

trap '"$GCLOUD" compute instances delete "$NAME" --zone="$ZONE" -q >/dev/null 2>&1 || true' EXIT INT TERM
echo "[autorefresh] refreshing golden to ${tip%${tip#????????}} ..."
"$GCLOUD" compute instances create "$NAME" --zone="$ZONE" --source-machine-image="$IMAGE" \
  --machine-type=e2-standard-16 --metadata-from-file=ssh-keys="$SSHKEYS_FILE" >/dev/null
ip=$("$GCLOUD" compute instances describe "$NAME" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
i=0; while [ "$i" -lt 45 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$ip" true 2>/dev/null && break; i=$((i + 1)); sleep 4; done
ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$ip" \
  "source \$HOME/.cargo/env; cd ~/bastion && git fetch -q origin && git reset --hard -q origin/$BRANCH && cargo build --profile verify -p bastion-harness -q && test -x target/verify/bastion-harness && echo BUILT_OK"

echo "[autorefresh] build verified — stopping + re-imaging..."
"$GCLOUD" compute instances stop "$NAME" --zone="$ZONE" >/dev/null 2>&1
"$GCLOUD" compute machine-images delete "$IMAGE" -q >/dev/null 2>&1 || true
"$GCLOUD" compute machine-images create "$IMAGE" --source-instance="$NAME" --source-instance-zone="$ZONE" >/dev/null
echo "$tip" > "$STAMP"
echo "[autorefresh] golden now at ${tip%${tip#????????}}. done."
