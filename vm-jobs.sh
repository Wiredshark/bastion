#!/bin/sh
# vm-jobs.sh — BREADTH across DIFFERENT tests. One ephemeral VM per job line, all in parallel; each
# lands on the latest bastion/builder tip (fetch+reset+assert), runs its OWN harness command, streams
# the result back, then deletes itself. Same live burn-guard + guaranteed teardown as vm-pool.sh.
# Use when you want MANY DIFFERENT tests at once (a whole suite / general data) rather than one test
# across many seeds — you get all results in the time of the SLOWEST single job, not their sum.
#
# Usage: bash vm-jobs.sh <jobs_file> <machine_type> [MAX_USD] [MAX_MIN]
#   jobs_file = one harness arg-line per job (blank lines / #comments ignored), e.g.:
#     --mine-fidelity-scenario --mf-minutes 10
#     --dig-access-scenario
#     --b4-scenario
#   -> 3 VMs, 3 different tests in parallel. Results: /tmp/bastion-jobs/job-<n>.out
# Keep (#jobs x vCPU) under the CPUS_ALL_REGIONS quota, with headroom.
set -u
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE=us-central1-a; IMAGE=bastion-golden; KEY="$HOME/.ssh/id_ed25519"; SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"
BRANCH=bastion/builder
JOBS_FILE="$1"; MACHINE="$2"; MAX_USD="${3:-5}"; MAX_MIN="${4:-30}"
N=$(grep -cvE '^[[:space:]]*(#|$)' "$JOBS_FILE")
VCPU_PER=$(echo "$MACHINE" | sed 's/.*-//'); TOTAL_VCPU=$((N * VCPU_PER)); RATE=0.035
OUT=/tmp/bastion-jobs; mkdir -p "$OUT"; rm -f "$OUT"/*.out "$OUT"/*.log "$OUT"/TRIPPED 2>/dev/null || true

cleanup() { k=0; while [ "$k" -lt "$N" ]; do "$GCLOUD" compute instances delete "bastion-job-$k" --zone="$ZONE" -q >/dev/null 2>&1 & k=$((k+1)); done; wait; }
trap cleanup EXIT INT TERM

run_job() {
  k="$1"; cmd="$2"; name="bastion-job-$k"
  "$GCLOUD" compute instances create "$name" --zone="$ZONE" --source-machine-image="$IMAGE" \
     --machine-type="$MACHINE" --metadata-from-file=ssh-keys="$SSHKEYS_FILE" >/dev/null 2>&1 || { echo "CREATE_FAIL: $cmd" > "$OUT/job-$k.out"; return; }
  ip=$("$GCLOUD" compute instances describe "$name" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
  i=0; while [ "$i" -lt 45 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$ip" true 2>/dev/null && break; i=$((i+1)); sleep 4; done
  ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$ip" "
    source \$HOME/.cargo/env; cd ~/bastion
    git fetch -q origin && git reset --hard -q origin/$BRANCH
    H=\$(git rev-parse --short HEAD); R=\$(git rev-parse --short origin/$BRANCH)
    [ \"\$H\" = \"\$R\" ] || { echo STALE=\$H/\$R; exit 3; }
    cargo build --profile verify -p bastion-harness -q
    echo \"### JOB $k @ \$H : $cmd\"
    ./target/verify/bastion-harness $cmd" > "$OUT/job-$k.out" 2>"$OUT/job-$k.log"
  "$GCLOUD" compute instances delete "$name" --zone="$ZONE" -q >/dev/null 2>&1
}

guard() {
  gstart="$1"
  while :; do
    sleep 90
    el_s=$(( $(date +%s) - gstart )); el_m=$(( el_s / 60 ))
    est=$(awk "BEGIN{printf \"%.2f\", $TOTAL_VCPU*($el_s/3600.0)*$RATE}")
    up=$("$GCLOUD" compute instances list --filter="name~^bastion-job" --format="value(name)" 2>/dev/null | wc -l)
    echo "[guard] ${el_m}m | ~\$$est | $up VMs up | ceiling \$$MAX_USD/${MAX_MIN}m"
    if [ "$el_m" -ge "$MAX_MIN" ] || [ "$(awk "BEGIN{print ($est>=$MAX_USD)?1:0}")" = "1" ]; then
      echo "[guard] *** CEILING HIT — CUTTING OFF ***"; : > "$OUT/TRIPPED"; cleanup; return
    fi
  done
}

echo "[jobs] $N different jobs x $MACHINE ($TOTAL_VCPU vCPU). Ceiling \$$MAX_USD/${MAX_MIN}m. Launching in parallel..."
start=$(date +%s); guard "$start" & GUARD_PID=$!
k=0
while IFS= read -r line; do
  case "$(echo "$line" | tr -d '[:space:]')" in ""|\#*) continue ;; esac
  run_job "$k" "$line" & k=$((k + 1))
done < "$JOBS_FILE"
wait $(jobs -p | grep -v "$GUARD_PID" 2>/dev/null) 2>/dev/null || wait
kill "$GUARD_PID" 2>/dev/null || true
end=$(date +%s)
final_est=$(awk "BEGIN{printf \"%.2f\", $TOTAL_VCPU*(($end-$start)/3600.0)*$RATE}")
[ -f "$OUT/TRIPPED" ] && { echo "=== JOBS CUT OFF at ceiling after $((end-start))s (~\$$final_est) ==="; exit 42; }
echo "=== ALL $N JOBS DONE in $((end-start))s | ~\$$final_est burned ==="
for f in "$OUT"/job-*.out; do echo "--- $f ---"; head -1 "$f"; tail -3 "$f"; done
