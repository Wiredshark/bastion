#!/usr/bin/env bash
# RECONCILE THE LEDGER against the log corpus on disk.
#
# WHY: `run-ledger.sh`'s denominator is `*-attest.txt` -- tags that produced an
# attestation. That is CORRECT for what it claims (accounting coverage of gated
# runs) and BLIND to everything that ran before the gate existed. The disk holds
# far more `server-*.log` than there are attest files, and nothing has ever
# compared the two populations.
#
# THE MAPPING IS VERIFIED BEFORE IT IS COUNTED. The gate-coverage row was VOIDED
# because its mapping bar fired -- `userdata-<TAG>` dirs did not exist for 12 of
# 15 attested tags, so the ratio it was about to report compared two populations
# that did not correspond. Same bar here: if an attested tag cannot find its own
# server log, the populations are not comparable and the ratio is VOID.
#
# usage: reconcile-ledger.sh [control|full] [evidence-dir]
set -u
MODE="${1:-control}"
EV="${2:-/e/veloren-master/bastion-test-evidence}"

# Structural derivation, both directions, stated once so they cannot drift:
#   attest file   <TAG>-attest.txt
#   server log    server-<TAG>.log
tag_of_attest() { local b; b=$(basename "$1"); echo "${b%-attest.txt}"; }
tag_of_srvlog() { local b; b=$(basename "$1"); b=${b#server-}; echo "${b%.log}"; }

if [ "$MODE" = control ]; then
  echo "L1 CONTROL PAIR -- run BEFORE any count"
  echo

  # POSITIVE: a known-attested tag must round-trip.
  KNOWN=pit-pitwood
  L="$EV/server-$KNOWN.log"; A="$EV/$KNOWN-attest.txt"
  echo "positive: server-$KNOWN.log"
  if [ ! -f "$L" ]; then echo "  !! the control log itself is absent -- control is VOID"; exit 2; fi
  D=$(tag_of_srvlog "$L")
  echo "  derived tag  : $D"
  echo "  attest exists: $([ -f "$EV/$D-attest.txt" ] && echo yes || echo NO)"
  [ "$D" = "$KNOWN" ] && [ -f "$A" ] || { echo "  !! POSITIVE CONTROL FAILED"; exit 2; }
  echo "  -> PASS (maps to its own attestation)"
  echo

  # NEGATIVE: an unattested log must map to NOTHING, not to a wrong tag.
  # Chosen structurally: the first server log whose derived tag has no attest file.
  for l in "$EV"/server-*.log; do
    d=$(tag_of_srvlog "$l")
    [ -f "$EV/$d-attest.txt" ] || { NEGL=$l; NEGD=$d; break; }
  done
  if [ -z "${NEGL:-}" ]; then
    echo "negative: NO unattested log exists -- the negative control cannot be run"
    echo "  !! without it the mapping is unproven in one direction"; exit 2
  fi
  echo "negative: $(basename "$NEGL")"
  echo "  derived tag  : $NEGD"
  echo "  attest exists: $([ -f "$EV/$NEGD-attest.txt" ] && echo YES--WRONG || echo no)"
  echo "  -> PASS (maps to nothing, not to a wrong tag)"
  exit 0
fi

# ---- L1 full: EVERY attested tag must find its own server log ----
echo "L1 FULL -- every attested tag must find server-<TAG>.log"
miss=0; tot=0
for a in "$EV"/*-attest.txt; do
  [ -e "$a" ] || continue
  t=$(tag_of_attest "$a"); tot=$((tot+1))
  if [ ! -f "$EV/server-$t.log" ]; then
    miss=$((miss+1)); printf "  MISSING  %s  (no server-%s.log)\n" "$t" "$t"
  fi
done
echo "attested tags: $tot   ·   without a server log: $miss"
if [ "$miss" -gt 0 ]; then
  echo
  echo "!! L1 FAILS -- the populations are not comparable. The L2 ratio is VOID."
  exit 3
fi
echo "-> L1 PASS: all $tot attested tags map onto the log corpus"
echo

# ---- L2: the gap, with BOTH denominators and the unit named ----
with=0; without=0; files=0
UNATT="$EV/.reconcile-unattested.txt"; : > "$UNATT"
for l in "$EV"/server-*.log; do
  [ -e "$l" ] || continue
  files=$((files+1)); d=$(tag_of_srvlog "$l")
  if [ -f "$EV/$d-attest.txt" ]; then with=$((with+1))
  else without=$((without+1)); echo "$l" >> "$UNATT"; fi
done
echo "L2  of $files SERVER LOG FILES:  with attestation $with  ·  without $without"
echo "    THE UNIT IS A LOG FILE, NOT A RUN -- a re-run tag overwrites its log,"
echo "    so $files is a LOWER BOUND on runs."
echo

# ---- L3: the pre-attestation era is DATED, not assumed ----
echo "L3  mtime range of the $without unattested logs:"
if [ "$without" -eq 0 ]; then
  echo "    (none)"
else
  # oldest and newest, plus how many fall on/after the gate's own date
  while read -r f; do date -r "$f" +%Y-%m-%dT%H:%M; done < "$UNATT" | sort > "$EV/.reconcile-mtimes.txt"
  echo "    oldest : $(head -1 "$EV/.reconcile-mtimes.txt")"
  echo "    newest : $(tail -1 "$EV/.reconcile-mtimes.txt")"
  ON=$(grep -c '^2026-08-15' "$EV/.reconcile-mtimes.txt" || true)
  echo "    on 2026-08-15 (the day the per-binary gate exists): $ON of $without"
fi
