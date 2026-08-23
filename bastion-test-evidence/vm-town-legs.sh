#!/bin/sh
# vm-town-legs.sh — the town-leg A/B on the VM FLEET (testing framework rule
# 9, Ben 2026-08-23: THE VM FLEET RUNS THE LEGS; the local desktop is the
# play surface and the build box). One VM per leg: an uncapped town server
# wants the whole machine, and 6 x e2-standard-16 = 96 vCPU = exactly the
# CPUS_ALL_REGIONS quota's proven-clean wave size (vm-pool.sh's wave log).
#
# Usage:
#   BRANCH=<branch> bash vm-town-legs.sh <fence_ticks> "<env_A>" "<env_B>" [MAX_USD] [MAX_MIN]
# e.g. the goal-verdict 3v3:
#   BRANCH=bastion/item29-trade bash vm-town-legs.sh 50000 "" "BASTION_GOAL_VERDICT=1" 20 120
# Arms are interleaved across VMs (0,2,4 = A; 1,3,5 = B) so a slow VM hits
# both arms equally. Each VM streams ONE @@@LEG judge line; the caller
# aggregates. Skeleton (create/ssh/guard/teardown, unpushed-tip pre-flight,
# burn guard) is vm-pool.sh's, kept behaviourally identical where shared.
set -u
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
ZONE="${ZONE:-us-central1-a}"; IMAGE=bastion-golden; KEY="$HOME/.ssh/id_ed25519"; SSHKEYS_FILE="C:/Users/q/.ssh/bastion-sshkeys.txt"
if [ -z "${BRANCH:-}" ]; then
  echo "!! BRANCH is unset and there is NO default (vm-pool.sh's own law:" >&2
  echo "   a silent default already invalidated an investigation)." >&2
  exit 2
fi
# Unpushed-tip pre-flight — the VMs build the PUSHED state, not the local tree.
if command -v git >/dev/null 2>&1; then
  _wt="${TOWN_WT:-/e/veloren-master/.item29-wt}"
  if _local=$(git -C "$_wt" rev-parse --short "$BRANCH" 2>/dev/null) \
     && _remote=$(git -C "$_wt" rev-parse --short "bastion-origin/$BRANCH" 2>/dev/null); then
    if [ "$_local" != "$_remote" ]; then
      echo "!! UNPUSHED TIP: local $BRANCH=$_local, pushed=$_remote. Push first," >&2
      echo "   or ALLOW_UNPUSHED=1 to measure the pushed tip deliberately." >&2
      [ -n "${ALLOW_UNPUSHED:-}" ] || exit 2
    fi
  else
    echo "[legs] NOTE: tip comparison SKIPPED (cannot resolve $BRANCH at $_wt) — skipped, NOT passed." >&2
  fi
fi
FENCE="${1:?fence ticks}"; ENV_A="${2-}"; ENV_B="${3:?env for arm B (\"\" for none)}"
MAX_USD="${4:-20}"; MAX_MIN="${5:-120}"
N=6; MACHINE="${MACHINE:-e2-standard-16}"
VCPU_PER=$(echo "$MACHINE" | sed 's/.*-//'); RATE=0.035
POOL="${POOL:-bastion-townlegs}"
OUT=/tmp/$POOL; mkdir -p "$OUT"; rm -f "$OUT"/*.log "$OUT"/TRIPPED 2>/dev/null || true

cleanup() { k=0; while [ "$k" -lt "$N" ]; do "$GCLOUD" compute instances delete "$POOL-$k" --zone="$ZONE" -q >/dev/null 2>&1 & k=$((k+1)); done; wait; }
trap 'cleanup' EXIT INT TERM

run_one() {
  k="$1"; name="$POOL-$k"; cerr=""
  if [ $((k % 2)) -eq 0 ]; then arm="$ENV_A"; else arm="$ENV_B"; fi
  tries=0
  until cerr=$("$GCLOUD" compute instances create "$name" --zone="$ZONE" --source-machine-image="$IMAGE" \
        --machine-type="$MACHINE" --metadata-from-file=ssh-keys="$SSHKEYS_FILE" 2>&1 >/dev/null); do
    tries=$((tries + 1)); [ "$tries" -ge 4 ] && { echo "CREATE_FAIL vm=$k :: ${cerr##*ERROR: }"; return; }
    sleep $((tries * 15))
  done
  ip=$("$GCLOUD" compute instances describe "$name" --zone="$ZONE" --format="value(networkInterfaces[0].accessConfigs[0].natIP)")
  i=0; while [ "$i" -lt 45 ]; do ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "benshumeyko@$ip" true 2>/dev/null && break; i=$((i+1)); sleep 4; done
  ssh -i "$KEY" -o StrictHostKeyChecking=no "benshumeyko@$ip" "
    source \$HOME/.cargo/env; cd ~/bastion
    git fetch -q origin && git reset --hard -q origin/$BRANCH
    H=\$(git rev-parse --short HEAD); R=\$(git rev-parse --short origin/$BRANCH)
    [ \"\$H\" = \"\$R\" ] && echo COMMIT=\$H || { echo STALE=\$H/\$R; exit 3; }
    cargo build --profile no_overflow -p veloren-server-cli -p veloren-client -q \
      || { echo BUILD_FAIL@\$H; exit 4; }
    bash bastion-test-evidence/vm-town-leg-remote.sh $((60 + k)) $FENCE \"$arm\""
  "$GCLOUD" compute instances delete "$name" --zone="$ZONE" -q >/dev/null 2>&1
}

guard() {
  gstart="$1"; acc=0; echo 0 > "$OUT/COST"
  while :; do
    sleep 90
    up=$("$GCLOUD" compute instances list --filter="name~^$POOL" --format="value(name)" 2>/dev/null | wc -l)
    acc=$(awk "BEGIN{printf \"%.3f\", $acc + $up*$VCPU_PER*(90/3600.0)*$RATE}")
    echo "$acc" > "$OUT/COST"
    el_m=$(( ($(date +%s) - gstart) / 60 ))
    echo "[guard] ${el_m}m | ~\$$acc | $up VMs | ceiling \$$MAX_USD / ${MAX_MIN}m"
    if [ "$el_m" -ge "$MAX_MIN" ] || [ "$(awk "BEGIN{print ($acc>=$MAX_USD)?1:0}")" = 1 ]; then
      echo "[guard] *** CEILING HIT — CUTTING OFF ***"; : > "$OUT/TRIPPED"; cleanup; return
    fi
  done
}

start=$(date +%s); guard "$start" & GUARD_PID=$!
k=0; while [ "$k" -lt "$N" ]; do run_one "$k" > "$OUT/vm-$k.log" 2>&1 & k=$((k+1)); done
wait $(jobs -p | grep -v "^$GUARD_PID$") 2>/dev/null || true
kill "$GUARD_PID" 2>/dev/null
echo "── LEG RESULTS (arm A = even VMs: '$ENV_A' | arm B = odd VMs: '$ENV_B') ──"
grep -h "@@@LEG\|CREATE_FAIL\|STALE=\|BUILD_FAIL\|LEG_BOOT_FAIL\|LEG_DEAD\|COMMIT=" "$OUT"/vm-*.log
[ -f "$OUT/TRIPPED" ] && echo "!! RUN WAS CUT OFF BY THE BURN GUARD — results above are PARTIAL"
