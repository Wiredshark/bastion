#!/bin/sh
# build-both.sh — build the server AND the driver, because a `common/` change
# invalidates both and I have now forgotten that THREE TIMES IN ONE SESSION.
#
# ★ WHY A SCRIPT AND NOT A NOTE. I already have the note; I wrote it after the
# second occurrence, and the third happened anyway. A memory is a comment, and a
# comment cannot enforce — the same lesson this repo learned when a 2026-07-30
# sort fix was documented in one function and the identical defect reappeared
# nine days later in the function beside it.
#
# Each time, `attest-run.sh` REFUSED the run and no wrong number was published
# — the gate is doing its job. What it cost was a wasted launch, and what it
# could not do is stop me making the mistake. This can.
#
#   veloren-server-cli : the simulation
#   bastion_playtest   : the DRIVER, which links the SAME common/ crate and is
#                        the half that reads the fields a common/ change touches
#
# The change always FEELS server-side — a need, a job kind, a manifest entry —
# which is exactly why the driver is the one that gets forgotten.
#
# Usage: bash build-both.sh [--profile no_overflow]
set -eu
PROFILE="${1:-no_overflow}"
cd "$(dirname "$0")/../.engine-integration-wt" 2>/dev/null || cd "$(dirname "$0")/.."

echo "=== tree state (a binary built dirty bakes a hash naming code it lacks) ==="
DIRTY=$(git status --short -- '*.rs' | wc -l)
echo "dirty .rs files: $DIRTY"
[ "$DIRTY" -eq 0 ] || echo "!! COMMIT FIRST — otherwise the binary's baked hash names HEAD, not this tree"

echo "=== 1/2 server ==="
cargo build --profile "$PROFILE" --bin veloren-server-cli
echo "=== 2/2 driver ==="
cargo build --profile "$PROFILE" --bin bastion_playtest -p veloren-client

echo "=== both built. Binary times: ==="
ls -la "target/$PROFILE/veloren-server-cli.exe" "target/$PROFILE/bastion_playtest.exe" 2>/dev/null \
  | awk '{print "  " $NF "  " $6 " " $7 " " $8}'

# ★ THE SUCCESS SENTINEL, added 2026-08-17 after this script REPORTED A GREEN
# BUILD THAT HAD FAILED.
#
# `set -eu` above is correct and did its job: cargo failed, the script exited
# non-zero. The lie was in the CALLER — `bash build-both.sh 2>&1 | tail -8`
# reports the exit status of `tail`, which is always 0. That is a banked
# lesson ("a piped background build masks the exit code") and it still cost a
# cycle: I read a tail of compiler errors, saw "exit code 0", and had to be
# told by the binary timestamps that nothing had been rebuilt.
#
# A script cannot control how it is piped. What it CAN do is make success
# unmistakable in the ONE view a pipe preserves: the tail. So the last line is
# a sentinel that only ever prints on the success path.
#
# ★ READ IT AS A PRECONDITION, NOT A DECORATION: no `BUILD-BOTH: OK` line at
# the end of the output means THE BUILD FAILED, regardless of what any exit
# code says. Same discipline as printing a run's precondition above its result
# so a VOID can never be mistaken for a RED.
echo "BUILD-BOTH: OK  profile=$PROFILE  head=$(git rev-parse --short HEAD)  dirty_rs=$DIRTY"
