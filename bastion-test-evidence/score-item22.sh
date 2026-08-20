#!/bin/bash
# ITEM 22 scorer. Usage: score-item22.sh <arm-tag>  (default: pintrait)
# Bar 1: a co-working pair's sentiment rises monotonically across samples;
#        pairs NEVER credited by the witness hold no sentiment (the null —
#        vacuous in fixtures where every pair co-works; then say so).
# Bar 4: display equals the record — same-source fill, asserted by build.
EV="${PIT_EV:-/e/veloren-master/.engine-integration-wt/bastion-test-evidence}"
ARM="${1:-pintrait}"
S=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/server-pit-$ARM.log" 2>/dev/null)
D=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/driver-pit-$ARM.log" 2>/dev/null)

echo "=== PRECONDITIONS ($ARM) ==="
echo "co-work deltas queued: $(echo "$S" | grep -c 'sentiment delta queued')"
echo "driver samples       : $(echo "$D" | grep -c 'SENT uid=')"

echo "=== BAR 1a: per-pair values across samples (rise = monotone) ==="
for U in 3 4 5; do
  echo "-- uid=$U:"
  echo "$D" | grep "SENT uid=$U " | cut -c1-190
done

echo "=== BAR 1b: credited pairs (witness) vs displayed pairs ==="
echo "-- credited (subject,object) counts:"
echo "$S" | grep "sentiment delta queued" | grep -oE "subject=[0-9]+ object=[0-9]+" | sort | uniq -c | sort -rn | head -12
echo "-- displayed pairs at last sample:"
echo "$D" | grep "SENT uid=" | tail -8 | grep -oE 'uid:[0-9]+' | sort | uniq -c
echo "NOTE: if every pair is credited, the null is VACUOUS in this fixture — name it."
