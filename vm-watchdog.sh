#!/bin/sh
# vm-watchdog.sh — SAFETY SWEEP (run every 10 min via Task Scheduler, or in a loop). Deletes any
# bastion-* VM older than MAX_AGE_MIN. Every bastion VM is ephemeral and finishes in MINUTES, so
# anything older is a hang / forgotten run -> killed. This guarantees NOTHING runs unattended, which
# bounds spend to pennies no matter what the credit balance is (GCP has no real-time credit API to
# poll, so we bound RUNTIME instead of trying to read the balance). One-shot + idempotent.
#
# Usage:   bash vm-watchdog.sh [MAX_AGE_MIN]        # default 60
# Loop:    while true; do bash vm-watchdog.sh; sleep 600; done
# Windows Task Scheduler: run every 10 min ->  cmd /c "bash E:\veloren-master\vm-watchdog.sh"
#
# It never touches non-bastion VMs (the ^bastion- filter). For a LONG soak that legitimately runs
# > MAX_AGE_MIN, pass a larger value (e.g. bash vm-watchdog.sh 180).
set -u
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
MAX_AGE_MIN="${1:-60}"
now=$(date +%s)
killed=0

"$GCLOUD" compute instances list --filter="name~^bastion-" \
  --format="value(name,creationTimestamp,status,zone)" 2>/dev/null | while IFS="	" read -r name ts status zone; do
  [ -z "$name" ] && continue
  created=$(date -d "$ts" +%s 2>/dev/null || echo "$now")
  age_min=$(( (now - created) / 60 ))
  if [ "$age_min" -ge "$MAX_AGE_MIN" ]; then
    echo "[watchdog] KILL $name  (age ${age_min}m >= ${MAX_AGE_MIN}m, $status)"
    "$GCLOUD" compute instances delete "$name" --zone="${zone##*/}" -q >/dev/null 2>&1 || true
    killed=$((killed + 1))
  else
    echo "[watchdog] ok   $name  (age ${age_min}m, $status)"
  fi
done

echo "[watchdog] sweep complete $(date '+%Y-%m-%d %H:%M:%S')"
