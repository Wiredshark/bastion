#!/bin/sh
# vm-pool-safe.sh — breadth pool with AUTO-FALLBACK. Runs vm-pool.sh; if its live burn-guard cuts the
# run off (exit 42 = hit $MAX_USD or $MAX_MIN), it retries at HALF the VM count, down to a floor. So a
# big "wide/tall" test can never run away — it meters itself and steps down automatically. This is the
# "if burn is too much, cut it off and go smaller" policy, automated.
#
# Usage: bash vm-pool-safe.sh <N_vms> <machine_type> <seeds_per_vm> <first_seed> "<args>" [MAX_USD] [MAX_MIN]
#   e.g. bash vm-pool-safe.sh 32 e2-standard-4 4 1000 "--mine-fidelity-scenario --mf-minutes 5" 15 25
set -u
N="$1"; MACHINE="$2"; SPV="$3"; FIRST="$4"; ARGS="$5"; MAX_USD="${6:-5}"; MAX_MIN="${7:-30}"
FLOOR=2
while [ "$N" -ge "$FLOOR" ]; do
  echo "############ attempt: $N VMs (ceiling \$$MAX_USD / ${MAX_MIN}m) ############"
  bash /e/veloren-master/vm-pool.sh "$N" "$MACHINE" "$SPV" "$FIRST" "$ARGS" "$MAX_USD" "$MAX_MIN"
  rc=$?
  [ "$rc" != "42" ] && exit "$rc"
  N=$((N / 2))
  echo "############ cut off at ceiling — halving to $N VMs, retrying ############"
done
echo "############ reached floor ($FLOOR VMs) and STILL tripping — stopping; investigate the run ############"
exit 42
