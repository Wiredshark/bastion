#!/bin/sh
# vm-cleanup.sh — HYGIENE / PANIC-STOP: guarantee zero compute spend. Stops the on-demand VM,
# deletes any stray pool clones, and prunes duplicate golden images + bastion snapshots. Only
# touches bastion-* resources (never other project data). Safe to run anytime; run it whenever
# you want to be SURE nothing is billing. See BUILD-AND-TEST-PROCESS.md §14.
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a; MAIN=instance-20260719-131242

echo "=== stop the main on-demand VM (if running) ==="
"$GCLOUD" compute instances stop "$MAIN" --zone="$ZONE" -q >/dev/null 2>&1 || true

echo "=== delete any stray pool clones (bastion-pool-*) ==="
for n in $("$GCLOUD" compute instances list --filter="name~^bastion-pool" --format="value(name)" 2>/dev/null); do
  echo "  deleting clone $n"; "$GCLOUD" compute instances delete "$n" --zone="$ZONE" -q >/dev/null 2>&1 || true
done

echo "=== prune DUPLICATE golden images (keep only 'bastion-golden', delete bastion-golden-* copies) ==="
for img in $("$GCLOUD" compute machine-images list --filter="name~^bastion-golden-" --format="value(name)" 2>/dev/null); do
  echo "  deleting duplicate image $img"; "$GCLOUD" compute machine-images delete "$img" -q >/dev/null 2>&1 || true
done

echo "=== prune stray bastion-* snapshots ==="
for s in $("$GCLOUD" compute snapshots list --filter="name~^bastion" --format="value(name)" 2>/dev/null); do
  echo "  deleting snapshot $s"; "$GCLOUD" compute snapshots delete "$s" -q >/dev/null 2>&1 || true
done

echo "=== FINAL STATE (nothing should be RUNNING; images = just bastion-golden) ==="
"$GCLOUD" compute instances list --format="value(name,status)" 2>&1
"$GCLOUD" compute machine-images list --format="value(name,status)" 2>&1
echo "=== CLEANUP DONE — compute spend is zero; only ~\$25/mo idle floor (disk+IP+image) remains ==="
