#!/bin/bash
# ITEM 23 (morale events) thoughts-leg scorer.
# BAR: MOODX prints thoughts>0 with id+magnitude, and total moves off base.
# The treatment witness (NEEDS_DECAY_MULT active) must sit ABOVE the result.
EV="${PIT_EV:-/e/veloren-master/.engine-integration-wt/bastion-test-evidence}"
S=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/server-pit-thoughts.log" 2>/dev/null)
D=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/driver-pit-thoughts.log" 2>/dev/null)

echo "=== PRECONDITIONS ==="
echo "treatment (decay-mult witness): $(echo "$S" | grep -c 'NEEDS_DECAY_MULT active')"
echo "$S" | grep -m1 "NEEDS_DECAY_MULT active" | grep -oE "mult=[0-9.]+"
echo "seed emit    : $(echo "$S" | grep -c 'BASTION_SEED_FOOD active')"
echo "beds built   : $(echo "$S" | grep -ciE 'bed (registered|built)')"
echo "sleep (RestAt completions): $(echo "$S" | grep -c 'kind="RestAt"')"
echo "driver inspects: $(echo "$D" | grep -c 'INSPECT uid=')"

echo "=== ITEM 23: the breakdown ==="
echo "MOODX lines          : $(echo "$D" | grep -c 'MOODX uid=')"
echo "MOODX with thoughts>0: $(echo "$D" | grep 'MOODX uid=' | awk '$5!="thoughts=0"' | wc -l)"
echo "$D" | grep "MOODX uid=" | awk '$5!="thoughts=0"' | head -8 | cut -c1-200
echo "-- distinct totals (base=0.6000; movement = the thought reached the mood):"
echo "$D" | grep "MOODX uid=" | grep -oE "total=[0-9.]+" | sort | uniq -c

echo "=== needs under the multiplier (treatment reached the subjects?) ==="
echo "$D" | grep "INSPECT uid=3 " | grep -oE "rest=[0-9.]+" | head -5
