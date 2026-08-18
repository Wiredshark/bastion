#!/usr/bin/env bash
# THE BLIND SPOT: do `blocked_materials` samples land inside a window where
# the F3 emit is blind, and do they move there?
#
# BLIND-SPOT WINDOW = from a transition into branch C with claimed=true, to
# the next transition (or end of run). During it the branch is C regardless of
# `material_held`, so the F3 emit reports nothing about material state.
#
# THE PROXY IS NOT THE VARIABLE. `material_held` is a BOOLEAN over ACCESS
# PLANS; `blocked_materials` is a COUNT over every job needing materials minus
# Haul. Registered in BLINDSPOT-PREREG.md before scoring: agreement is not
# equivalence, and this can only show the material picture is observable in the
# gap BY SOMETHING ELSE.
#
# Sample ticks come from the DRIVER log's inspect_colony lines; window bounds
# from the SERVER log's F3-BRANCH lines. Both are stripped of ANSI first.
#
# Usage:  score-blindspot.sh <arm-tag>
set -u
EV=/e/veloren-master/bastion-test-evidence
ARM="${1:?usage: score-blindspot.sh <arm-tag>}"
S="$EV/server-$ARM.log"; D="$EV/driver-$ARM.log"
[ -f "$S" ] && [ -f "$D" ] || { echo "  $ARM: MISSING LOG -> VOID"; exit 2; }

SL=$(sed 's/\x1b\[[0-9;]*m//g' "$S" | grep "bastion F3-BRANCH")
[ -n "$SL" ] || { echo "  $ARM: no F3-BRANCH lines -> VOID (no windows definable)"; exit 2; }

# Window opens on: branch=C AND claimed=true. Closes at the next transition.
opens=$(printf '%s\n' "$SL" | grep "branch=C" | grep "claimed=true" \
        | grep -oE "tick=[0-9]+" | cut -d= -f2)
alltick=$(printf '%s\n' "$SL" | grep -oE "tick=[0-9]+" | cut -d= -f2)
nwin=$(printf '%s\n' "$opens" | grep -c . || true)

echo "  === $ARM ==="
echo "    blind-spot windows (C && claimed=true): ${nwin:-0}"
if [ "${nwin:-0}" -eq 0 ]; then
  echo "    -> VOID for this arm: the emit never entered a claimed-C state"
  exit 0
fi

# blocked_materials samples: the driver prints them in inspect_colony output.
# There is no tick on that line, so samples are ORDINAL, not timestamped --
# stated because it bounds what this row can conclude.
# BOUND TO ONE RUN (DRIVER-LOG-APPEND-AMENDMENT.md): the driver appends, and
# this scorer reads a SERIES (`blocked_materials` across samples) whose shape
# is the measurement -- pooling concatenates two runs' series into one that
# no single run produced.
RUNS=$(bash "$(dirname "$0")/last-run.sh" "$D" --count) || exit $?
[ "$RUNS" -gt 1 ] && echo "  note: driver-$ARM.log holds $RUNS runs -- scoring the LAST one only"
BM=$(bash "$(dirname "$0")/last-run.sh" "$D" | sed 's/\x1b\[[0-9;]*m//g' | grep -oE "blocked_materials=[0-9]+" | cut -d= -f2 | tr '\n' ' ')
nbm=$(printf '%s' "$BM" | wc -w)
echo "    blocked_materials samples: $nbm   values: $BM"

for o in $opens; do
  close=$(printf '%s\n' "$alltick" | awk -v o="$o" '$1>o {print; exit}')
  printf "    window opens tick=%-6s closes tick=%s\n" "$o" "${close:-<end of run>}"
done

uniq_n=$(printf '%s' "$BM" | tr ' ' '\n' | grep -c . )
distinct=$(printf '%s' "$BM" | tr ' ' '\n' | sort -u | grep -c .)
if [ "$distinct" -gt 1 ]; then
  echo "    -> blocked_materials MOVED across the run ($distinct distinct of $uniq_n samples)"
else
  echo "    -> blocked_materials CONSTANT across the run ($distinct distinct value)"
fi
