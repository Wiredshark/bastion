#!/usr/bin/env bash
# D1, FINISHED: are `blocked_materials` samples INSIDE a blind-spot window,
# and do they move there?  Requires the COLONY-TICK field (row 29) -- without
# it the driver's colony line has no tick and the two series cannot be aligned,
# which is exactly why row 28's D1 was PARTIAL.
#
# T1 (same-clock) is checked FIRST and gates T2: a colony tick outside the
# run's F3-BRANCH tick range is a DIFFERENT clock, not an alignment.
#
# Usage:  score-align.sh <arm-tag>
set -u
EV=/e/veloren-master/bastion-test-evidence
ARM="${1:?usage: score-align.sh <arm-tag>}"
S="$EV/server-$ARM.log"; D="$EV/driver-$ARM.log"
[ -f "$S" ] && [ -f "$D" ] || { echo "  $ARM: MISSING LOG -> VOID"; exit 2; }
SL=$(sed 's/\x1b\[[0-9;]*m//g' "$S" | grep "bastion F3-BRANCH")
# BOUND TO ONE RUN (DRIVER-LOG-APPEND-AMENDMENT.md): `bastion_playtest`
# appends, so a re-run tag's driver log holds every previous leg -- and this
# scorer's T1 checks MONOTONICITY of `COLONY tick=`, which a pooled file
# breaks outright: run 2's tick sequence restarts below run 1's last value,
# so a sound pair of runs would read as a monotonicity FAILURE.
# ★ That makes this the scorer where pooling produces a false RED, not just
# a false denominator.
RUNS=$(bash "$(dirname "$0")/last-run.sh" "$D" --count) || exit $?
[ "$RUNS" -gt 1 ] && echo "  note: driver-$ARM.log holds $RUNS runs -- scoring the LAST one only"
CL=$(bash "$(dirname "$0")/last-run.sh" "$D" | sed 's/\x1b\[[0-9;]*m//g' | grep "COLONY tick=")

echo "  === $ARM ==="
if [ -z "$CL" ]; then
  echo "    T1 VOID: no 'COLONY tick=' lines -- the payload carries no tick"
  echo "             (a driver built before the COLONY-TICK field, or none sent)"
  exit 2
fi
if [ -z "$SL" ]; then echo "    T1 VOID: no F3-BRANCH lines to align against"; exit 2; fi

ct=$(printf '%s\n' "$CL" | grep -oE "^.*COLONY tick=[0-9]+" | grep -oE "tick=[0-9]+$" | cut -d= -f2)
ft=$(printf '%s\n' "$SL" | grep -oE "tick=[0-9]+" | cut -d= -f2)
fmin=$(printf '%s\n' "$ft" | sort -n | head -1); fmax=$(printf '%s\n' "$ft" | sort -n | tail -1)
echo "    colony ticks : $(printf '%s' "$ct" | tr '\n' ' ')"
echo "    F3 tick range: $fmin .. $fmax"

# T1a monotonic
mono=yes; prev=-1
for t in $ct; do [ "$t" -le "$prev" ] && mono=no; prev=$t; done
echo "    T1 monotonic increasing: $mono"
# T1b same-clock: colony ticks must be >= first F3 tick (run start) -- they may
# exceed fmax, since the last window runs to end of run.
oob=0; for t in $ct; do [ "$t" -lt "$fmin" ] && oob=$((oob+1)); done
echo "    T1 colony ticks below the F3 range: $oob  (0 = consistent with one clock)"
[ "$mono" = yes ] && [ "$oob" -eq 0 ] || { echo "    -> T1 FAILS; T2 not scored"; exit 1; }

# T2: windows are [open, close) from C&&claimed=true transitions.
opens=$(printf '%s\n' "$SL" | grep "branch=C" | grep "claimed=true" | grep -oE "tick=[0-9]+" | cut -d= -f2)
inwin=0; vals=""
for t in $ct; do
  bm=$(printf '%s\n' "$CL" | grep "tick=$t " | grep -oE "blocked_materials=[0-9]+" | cut -d= -f2 | head -1)
  for o in $opens; do
    close=$(printf '%s\n' "$ft" | sort -n | awk -v o="$o" '$1>o {print; exit}')
    close=${close:-99999999}
    if [ "$t" -ge "$o" ] && [ "$t" -lt "$close" ]; then
      inwin=$((inwin+1)); vals="$vals $bm"; break
    fi
  done
done
nwin=$(printf '%s\n' "$opens" | grep -c . || true)
echo "    blind-spot windows: ${nwin:-0}"
echo "    colony samples INSIDE a window: $inwin   values:$vals"
if [ "$inwin" -eq 0 ]; then
  echo "    -> T2 VOID: no sample landed inside a window; the proxy does not observe the gap"
elif [ "$(printf '%s' "$vals" | tr ' ' '\n' | sort -u | grep -c .)" -gt 1 ]; then
  echo "    -> T2: the proxy MOVED inside the gap"
else
  echo "    -> T2: in-window samples all equal -- no movement OBSERVED (not proof of stability)"
fi
