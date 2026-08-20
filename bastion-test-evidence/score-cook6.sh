#!/bin/bash
# ITEM 27 census-leg scorer (+ item 22 first-look from the same leg).
# Reads the WORKTREE evidence dir (PIT_EV redirect — the cookery6 VOID was
# exactly this path defaulting to the main checkout).
EV="${PIT_EV:-/e/veloren-master/.engine-integration-wt/bastion-test-evidence}"
S=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/server-pit-cookery.log" 2>/dev/null)
D=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/driver-pit-cookery.log" 2>/dev/null)

echo "=== PRECONDITIONS (a VOID run and a RED run must not look alike) ==="
echo "seed emit      : $(echo "$S" | grep -c 'BASTION_SEED_FOOD active')"
echo "stations built : $(echo "$S" | grep -c 'cook station registered')"
echo "driver inspects: $(echo "$D" | grep -c 'INSPECT uid=')"

echo "=== ITEM 27: the census (absent vs fell vs def-mismatch) ==="
echo "$S" | grep "ITEM 27 census — loose item table" | tail -3 | cut -c30-200
echo "$S" | grep "ITEM 27 census — food item" | head -8 | cut -c30-230

echo "=== ITEM 27: the pipeline ==="
echo "cooked (raw consumed): $(echo "$S" | grep -c 'raw consumed')"
echo "cook-without-raw     : $(echo "$S" | grep -c 'WITHOUT raw')"
echo "idle-no-raw witnesses: $(echo "$S" | grep -c 'cook station idle')"
echo "$S" | grep "RESERVATION-ONLY" | grep -oE 'req="[^"]*" stocked=[0-9]+ reserved=[0-9]+' | sort | uniq -c | sort -rn | head -4

echo "=== ITEM 22 first-look (same leg, free) ==="
echo "co-work deltas queued: $(echo "$S" | grep -c 'ITEM 22 sentiment delta queued')"
echo "SENT lines (driver)  : $(echo "$D" | grep -c '^.*SENT uid=')"
echo "$D" | grep "SENT uid=" | awk '{print $2, $3}' | sort | uniq -c | head -6
echo "-- per-sample sentiment values for uid=3:"
echo "$D" | grep "SENT uid=3 " | cut -c1-180
