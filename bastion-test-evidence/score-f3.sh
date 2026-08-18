#!/usr/bin/env bash
# PRE-SPECIFIED SCORING for the F3-branch live row. Written BEFORE the run.
#
# Derived from the emit's own field list (bastion_jobs.rs:16621), not guessed:
#   info!(tick, branch = ?branch, access_jobs = access_jobs_exist,
#         claimed = access_claimed, material_held, idle_before,
#         idle = board.access_idle_secs, stalled = board.access_stalled_secs,
#         "bastion F3-BRANCH")
#
# L1 IS A PRECONDITION, PRINTED ABOVE THE RESULT. The emit is TRANSITION-gated
# (`if board.access_branch_state != Some(branch)`), so:
#   0 lines  = the branch never changed, OR the diag gate never engaged
#            -> VOID. It is NOT "branch B is 0%".
# A per-tick distribution CANNOT be computed from this log at all; the
# per-tick counters (b5_f3_ticks_branch_a/b/c) are harness-only.
#
# L2's number is a PEAK, not a rate: the chain's own comment (:16490) says a
# B->A or B->C transition line reports the peak branch B actually reached --
# "the number the pruner's 20-second threshold turns on" -- and that is
# `idle_before`. Denominator is the COUNT OF TRANSITIONS, stated as such.
#
# Usage:  score-f3.sh <server-log>
set -u
LOG="${1:?usage: score-f3.sh <server-log>}"
[ -f "$LOG" ] || { echo "!! NO SERVER LOG AT $LOG"; exit 2; }

# ANSI is stripped: the server's tracing output IS coloured (measured), and a
# pattern that stops matching after a formatter change is the failure this
# guards against.
CLEAN=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG")
LINES=$(printf '%s\n' "$CLEAN" | grep "bastion F3-BRANCH" || true)
N=$(printf '%s\n' "$LINES" | grep -c "bastion F3-BRANCH" || true)

echo "=== L1 · PRECONDITION ==="
if [ "${N:-0}" -eq 0 ]; then
  echo "  F3-BRANCH lines: 0  -> VOID"
  echo "  The branch never changed, or BASTION_ACCESS_CLAIM_DIAG never engaged."
  echo "  This is NOT evidence that branch B is rare -- a silent log is evidence"
  echo "  about the LOGGER."
  exit 2
fi
echo "  F3-BRANCH lines: $N   (unit: branch TRANSITIONS, not ticks)"

echo "=== L2 · THE BRANCHES ENTERED ==="
printf '%s\n' "$LINES" | grep -oE "branch=[A-Za-z]+" | sort | uniq -c | sed 's/^/   /'

echo "=== L3 · claimed / material_held AS OBSERVED ==="
for f in claimed material_held access_jobs; do
  printf '   %-14s %s\n' "$f" \
    "$(printf '%s\n' "$LINES" | grep -oE "$f=[a-z]+" | sort | uniq -c | tr '\n' ' ')"
done

echo "=== L2 · PEAK IDLE REACHED (seconds; threshold is 20.0) ==="
PEAK=$(printf '%s\n' "$LINES" | grep -oE "idle_before=[0-9.]+" | cut -d= -f2 \
        | sort -g | tail -1)
echo "   max(idle_before) = ${PEAK:-none}  over $N transitions"
echo "   (idle_before on a B-exit is the peak branch B reached -- :16490)"

echo "=== VERDICT ==="
# CORRECTED after the first real data (disclosed): the zero branch used to
# read "consistent with a CLAIMED plan pinning the clock (row 21)". The live
# run showed `claimed=false` on EVERY transition and the clock still pinned at
# 0 -- so that text asserted a mechanism the same line's own fields refuted.
#
# The clock is reset by branch A (material_held) OR branch C (claimed, or no
# access jobs at all). Naming only one of them was the single-writer error this
# project has a law about. The verdict now reports WHICH resets were observed
# instead of guessing between them.
awk -v p="${PEAK:-0}" 'BEGIN{
  if (p+0 >= 20.0) print "   the pruner threshold WAS reached (peak >= 20.0s)";
  else if (p+0 > 0) print "   idle got off the floor but never reached 20.0s -- peak " p "s";
  else print "   idle NEVER left 0 -- branch B never accumulated. See the branch\n   and material_held/claimed counts ABOVE for which reset held it there;\n   do not attribute it to one of them without reading those.";
}'
