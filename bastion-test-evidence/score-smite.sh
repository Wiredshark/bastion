#!/bin/bash
# ITEM 31 (POWER-0) scorer — bars 1 (real cast), 2 (loud refusal), 4
# (no-command invariant); bar 3 (twin) rides the standing twin queue.
EV="${PIT_EV:-/e/veloren-master/.engine-integration-wt/bastion-test-evidence}"
S=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/server-pit-smite.log" 2>/dev/null)
D=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/driver-pit-smite.log" 2>/dev/null)

echo "=== BAR 2: the refusal (favor starts EMPTY by construction) ==="
echo "$S" | grep "smite REFUSED" | head -2 | cut -c64-200

echo "=== BAR 1: the cast (real damage + VFX + favor paid) ==="
echo "$S" | grep "SMITE cast" | head -2 | cut -c64-240

echo "=== Osric's health across the bracket (1.0 -> ~0.4 after the cast) ==="
echo "$D" | grep "INSPECT uid=" | grep "Osric" | grep -oE "^\[[0-9]+\]|health=[^ ]+" | paste - - | tail -4

echo "=== BAR 4: no-command — Osric's activity/drive around the cast ==="
echo "$D" | grep "INSPECT uid=" | grep "Osric" | grep -oE "drive=[A-Za-z]+|activity=[A-Za-z(]+" | paste - - | tail -4
echo "(the cast must not have rewritten drive/activity; changes there must be the arbiter's own)"
