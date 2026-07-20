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
# VM_ZONE override (ENGOPT4 arc: us-central1-a threw ZONE_RESOURCE_POOL_EXHAUSTED
# across families — n2 then e2 — within one session; stock-outs are transient and
# zonal, so fans can just move zones).
ZONE="${VM_ZONE:-us-central1-a}"; IMAGE=bastion-golden; KEY="$HOME/.ssh/id_ed25519"; SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"
BRANCH="${BRANCH:-bastion/builder}"   # override e.g. BRANCH=codex/boot-cache for a parallel lane
# CONCURRENT-FAN SCOPING (Builder-4 incident, 2026-07-20): three fans launched
# in parallel collided on EVERY shared resource — index-only VM names
# (bastion-job-$k: CREATE_FAILs + fan A's cleanup deleting fan B's VMs), the
# shared /tmp/bastion-jobs (each fan's startup rm CLOBBERED the others'
# results — jobs reported DONE with zero output), and the guard counting/
# ceiling-cutting by the shared ^bastion-job prefix. Every resource is now
# scoped by a unique FAN id (overridable for debugging).
FAN="${FAN_ID:-$(date +%s)-$$}"
JOBS_FILE="$1"; MACHINE="$2"; MAX_USD="${3:-5}"; MAX_MIN="${4:-30}"
N=$(grep -cvE '^[[:space:]]*(#|$)' "$JOBS_FILE")
VCPU_PER=$(echo "$MACHINE" | sed 's/.*-//'); TOTAL_VCPU=$((N * VCPU_PER)); RATE=0.035
OUT=/tmp/bastion-jobs/$FAN; mkdir -p "$OUT"; rm -f "$OUT"/*.out "$OUT"/*.log "$OUT"/TRIPPED 2>/dev/null || true

cleanup() { k=0; while [ "$k" -lt "$N" ]; do "$GCLOUD" compute instances delete "bastion-job-$FAN-$k" --zone="$ZONE" -q >/dev/null 2>&1 & k=$((k+1)); done; wait; }
trap cleanup EXIT INT TERM

run_job() {
  k="$1"; cmd="$2"; name="bastion-job-$FAN-$k"; cerr=""
  # RECORDER=1 -> the harness runs with the flight recorder writing to ~/tape
  # (env exported inline in the ssh command), and the tape is scp'd back.
  RECORDER_ENV=""
  [ -n "${RECORDER:-}" ] && RECORDER_ENV="mkdir -p ~/tape && BASTION_FLIGHT_RECORDER_DIR=\$HOME/tape "
  tries=0  # retry w/ backoff — transient quota bounces from racing a prior batch's teardown
  # MIN_CPU_PLATFORM (ENGOPT4 diagnosis): pin VMs to one microarchitecture so
  # cross-machine comparisons separate SCHEDULING nondeterminism from
  # hardware-float variance. Only N1/N2/C2 families honor it (e2 ignores the
  # need entirely — it must not be passed there).
  set -- --zone="$ZONE" --source-machine-image="$IMAGE" --machine-type="$MACHINE" \
      --metadata-from-file=ssh-keys="$SSHKEYS_FILE"
  # NOTE: passing --min-cpu-platform (value contains spaces) is NOT POSSIBLE
  # from this msys sh → gcloud.cmd path: bash's command-line re-assembly for
  # .cmd targets mangles ANY spaced/quoted arg (two attempts: bare and
  # embedded-quotes both died with the 'C:\Program' mangle — the GCLOUD
  # path itself loses quoting; CREATE_FAIL rows caught both at zero cost).
  # Platform control is therefore POST-HOC: create unpinned, and the job
  # header below ATTESTS the actual cpuPlatform so same-platform claims are
  # verified, not assumed.
  [ -n "${MIN_CPU_PLATFORM:-}" ] && { echo "MIN_CPU_PLATFORM unsupported on Windows sh->cmd (see comment); create unpinned + verify attested cpuPlatform" >&2; }
  until cerr=$("$GCLOUD" compute instances create "$name" "$@" 2>&1 >/dev/null); do
    tries=$((tries + 1)); [ "$tries" -ge 4 ] && { echo "CREATE_FAIL: $cmd :: ${cerr##*ERROR: }" > "$OUT/job-$k.out"; return; }
    sleep $((tries * 15))
  done
  ip=$("$GCLOUD" compute instances describe "$name" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
  # Attested hardware platform (ENGOPT4 diagnosis: same-platform claims must
  # be verified from the VM's own record, not assumed from the machine type).
  # Captured here, appended AFTER the ssh block (whose `>` would clobber it).
  cpu_platform=$("$GCLOUD" compute instances describe "$name" --zone="$ZONE" --format="value(cpuPlatform)")
  i=0; while [ "$i" -lt 45 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$ip" true 2>/dev/null && break; i=$((i+1)); sleep 4; done
  ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$ip" "
    source \$HOME/.cargo/env; cd ~/bastion
    git fetch -q origin && git reset --hard -q origin/$BRANCH
    H=\$(git rev-parse --short HEAD); R=\$(git rev-parse --short origin/$BRANCH)
    [ \"\$H\" = \"\$R\" ] || { echo STALE=\$H/\$R; exit 3; }
    cargo build --profile verify -p bastion-harness -q || { echo BUILD_FAIL@\$H; exit 4; }  # NEVER fall through to a stale binary
    GH=\$(./target/verify/bastion-harness --print-git-hash 2>/dev/null); RH=\$(git rev-parse --short=10 HEAD)
    [ -z \"\$GH\" ] || [ \"\${GH%%+*}\" = \"\$RH\" ] || { echo \"BINARY_STALE: built \$GH != checkout \$RH\"; exit 5; }  # sha-part only (+dirty = LFS noise, code clean via reset --hard)
    echo \"### JOB $k @ \$H : $cmd\"
    ${RECORDER_ENV}./target/verify/bastion-harness $cmd; rc=\$?
    echo \"=== ATTEST (end): RAN_COMMIT=\$H | job=$k | rc=\$rc ===\"" > "$OUT/job-$k.out" 2>"$OUT/job-$k.log"
  echo "### VM $name cpuPlatform=$cpu_platform" >> "$OUT/job-$k.out"
  # TAPE mode (first-divergence methodology): pull the recorder tapes back
  # before teardown — they die with the VM otherwise.
  if [ -n "${RECORDER:-}" ]; then
    mkdir -p "$OUT/job-$k-tape"
    scp -i "$KEY" -o StrictHostKeyChecking=no -r "benshumeyko@$ip:~/tape/*" "$OUT/job-$k-tape/" >/dev/null 2>&1 || echo "TAPE_PULL_FAIL: job=$k" >> "$OUT/job-$k.out"
  fi
  "$GCLOUD" compute instances delete "$name" --zone="$ZONE" -q >/dev/null 2>&1
}

guard() {
  gstart="$1"; acc=0; echo 0 > "$OUT/COST"
  while :; do
    sleep 90
    up=$("$GCLOUD" compute instances list --filter="name~^bastion-job-$FAN-" --format="value(name)" 2>/dev/null | wc -l)
    acc=$(awk "BEGIN{printf \"%.3f\", $acc + $up*$VCPU_PER*(90/3600.0)*$RATE}")  # true VM-time
    echo "$acc" > "$OUT/COST"
    el_m=$(( ($(date +%s) - gstart) / 60 ))
    echo "[guard] ${el_m}m | ~\$$acc (actual VM-time) | $up VMs up | ceiling \$$MAX_USD/${MAX_MIN}m"
    if [ "$el_m" -ge "$MAX_MIN" ] || [ "$(awk "BEGIN{print ($acc>=$MAX_USD)?1:0}")" = "1" ]; then
      echo "[guard] *** CEILING HIT — CUTTING OFF ***"; : > "$OUT/TRIPPED"; cleanup; return
    fi
  done
}

echo "[jobs] $N different jobs x $MACHINE ($TOTAL_VCPU vCPU). FAN=$FAN. Ceiling \$$MAX_USD/${MAX_MIN}m. Launching in parallel..."
start=$(date +%s); guard "$start" & GUARD_PID=$!
k=0; PIDS=""
while IFS= read -r line; do
  case "$(echo "$line" | tr -d '[:space:]')" in ""|\#*) continue ;; esac
  echo "LAUNCH: job=$k :: $line" >> "$OUT/launch.log"   # slot-loss forensics (seed-2024 incident)
  run_job "$k" "$line" & PIDS="$PIDS $!"; k=$((k + 1))
  sleep "${STAGGER:-10}"  # GCP rate-limits parallel creates from one machine-image — space them out
done < "$JOBS_FILE"
for p in $PIDS; do wait "$p" 2>/dev/null; done   # wait ONLY the run_job workers, never the guard
# SLOT GUARANTEE (Builder-4 seed-2024 incident: a tail-slot job left ZERO trace
# — no .out, not even a CREATE_FAIL — on two independent fans; the same
# silent-result-integrity class as the concurrent-fan clobber): a slot may
# fail, but it may NEVER vanish. Any slot without an .out gets a loud
# SLOT_LOST marker so downstream completeness gates fail closed, and
# launch.log records how far the launcher actually got.
j=0; while [ "$j" -lt "$N" ]; do
  [ -f "$OUT/job-$j.out" ] || echo "SLOT_LOST: job=$j left no trace (see launch.log; run_job never wrote)" > "$OUT/job-$j.out"
  j=$((j + 1))
done
kill "$GUARD_PID" 2>/dev/null; wait "$GUARD_PID" 2>/dev/null || true
end=$(date +%s)
cost=$(cat "$OUT/COST" 2>/dev/null || echo 0)
[ -f "$OUT/TRIPPED" ] && { echo "=== JOBS CUT OFF at ceiling after $((end-start))s (~\$$cost actual) ==="; exit 42; }
echo "=== ALL $N JOBS DONE in $((end-start))s | ~\$$cost burned (actual VM-time) ==="
for f in "$OUT"/job-*.out; do echo "--- $f ---"; head -1 "$f"; tail -3 "$f"; done
