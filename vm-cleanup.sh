#!/bin/sh
# vm-cleanup.sh — HYGIENE / PANIC-STOP: guarantee zero compute spend. In the fully-ephemeral model there
# is no persistent VM — this deletes ANY stray bastion-* VM (run/scale/pool/job/imgbuild) and prunes
# duplicate golden images + bastion snapshots. Only touches bastion-* resources. Safe anytime; run it
# whenever you want to be SURE nothing is billing. See BUILD-AND-TEST-PROCESS.md §14.
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a

echo "=== delete ANY stray bastion-* VM (nothing should persist in the ephemeral model) ==="
for n in $("$GCLOUD" compute instances list --filter="name~^bastion-" --format="value(name)" 2>/dev/null); do
  echo "  deleting $n"; "$GCLOUD" compute instances delete "$n" --zone="$ZONE" -q >/dev/null 2>&1 || true
done

echo "=== prune DUPLICATE golden images (keep only 'bastion-golden') ==="
for img in $("$GCLOUD" compute machine-images list --filter="name~^bastion-golden-" --format="value(name)" 2>/dev/null); do
  [ "$img" = "bastion-golden" ] && continue  # NEVER delete THE golden — only bastion-golden-* duplicates
  echo "  deleting duplicate image $img"; "$GCLOUD" compute machine-images delete "$img" -q >/dev/null 2>&1 || true
done

echo "=== prune stray bastion-* snapshots ==="
for s in $("$GCLOUD" compute snapshots list --filter="name~^bastion" --format="value(name)" 2>/dev/null); do
  echo "  deleting snapshot $s"; "$GCLOUD" compute snapshots delete "$s" -q >/dev/null 2>&1 || true
done

echo "=== FINAL STATE (instances should be EMPTY; images = just bastion-golden) ==="
"$GCLOUD" compute instances list --format="value(name,status)" 2>&1
"$GCLOUD" compute machine-images list --format="value(name,status)" 2>&1
echo "=== CLEANUP DONE — zero compute; only the ~\$0.02/mo bastion-golden image remains ==="
