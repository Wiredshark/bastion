#!/usr/bin/env bash
# THE MATERIAL ANCHOR: `material_held` at the FIRST F3-BRANCH transition with
# `access_jobs=true`, per arm. Registered in MATERIAL-ANCHOR-PREREG.md BEFORE
# being applied to any arm.
#
# DENOMINATOR: ONE TRANSITION PER ARM. This is two data points, not a rate.
#
# BLIND SPOT, registered not discovered: the branch is C whenever a job is
# CLAIMED, regardless of `material_held` -- and the emit fires only on branch
# CHANGE. So material state during claimed periods produces no line at all.
# This anchor sees material_held at branch-change moments only, and the
# disposition must restate that.
#
# Usage:  score-anchor.sh <server-log> [<server-log> ...]
set -u
for LOG in "$@"; do
  arm=$(basename "$LOG" .log); arm=${arm#server-}
  if [ ! -f "$LOG" ]; then printf "  %-12s NO LOG -> VOID\n" "$arm"; continue; fi
  # ANSI stripped: the server's tracing output is coloured (measured).
  LINE=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG" | grep "bastion F3-BRANCH" \
         | grep "access_jobs=true" | head -1)
  if [ -z "$LINE" ]; then
    printf "  %-12s VOID -- no transition with access_jobs=true (the anchor never occurs)\n" "$arm"
    continue
  fi
  tick=$(printf '%s' "$LINE" | grep -oE "tick=[0-9]+" | cut -d= -f2)
  br=$(printf '%s'  "$LINE" | grep -oE "branch=[A-Za-z]+" | cut -d= -f2)
  mh=$(printf '%s'  "$LINE" | grep -oE "material_held=[a-z]+" | cut -d= -f2)
  cl=$(printf '%s'  "$LINE" | grep -oE "claimed=[a-z]+" | cut -d= -f2)
  printf "  %-12s tick=%-6s branch=%-2s claimed=%-5s material_held=%s\n" \
         "$arm" "$tick" "$br" "$cl" "$mh"
done
