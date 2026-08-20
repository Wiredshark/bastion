#!/bin/bash
# ITEM 21 pin-legs scorer: bar 1 (pinned archetype renders, matches record)
# + bar 2 (opposite pin FLIPS the display). Run after BOTH arms:
#   run-pit.sh pintrait  (Adventurous)   run-pit.sh pintraitctl  (Closed)
EV="${PIT_EV:-/e/veloren-master/.engine-integration-wt/bastion-test-evidence}"
for ARM in pintrait pintraitctl; do
  S=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/server-pit-$ARM.log" 2>/dev/null)
  D=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/driver-pit-$ARM.log" 2>/dev/null)
  echo "=== $ARM ==="
  echo "pin witness (treatment): $(echo "$S" | grep -c 'personality PINNED')"
  echo "$S" | grep -m1 "personality PINNED" | grep -oE 'trait_=[A-Za-z]+'
  echo "inspects: $(echo "$D" | grep -c 'INSPECT uid=')"
  echo "-- traits per colonist (every line must carry the pinned trait):"
  echo "$D" | grep "INSPECT uid=" | grep -oE 'uid=[0-9]+|traits=\[[^]]*\]' | paste - - | sort -u | head -10
  echo "-- bravery spread (Adventurous pin should shift it if axis-coupled):"
  echo "$D" | grep -oE "bravery=[0-9.]+" | sort | uniq -c
done
echo "=== BAR 2: the two arms' trait sets must be DISJOINT on the pinned axis ==="
