#!/bin/sh
# last-attest.sh — print the LAST attestation block in an *-attest.txt file.
#
# ★ WHY THIS EXISTS. `attest-run.sh` APPENDS, so a file accumulates one block
# per run and `head` shows the OLDEST. On 2026-08-17 I read the head of
# pit-wallctl-attest.txt and saw "!! STALE ... ATTESTATION FAILED" for a run
# that had in fact just passed its gate — the previous attempt's verdict,
# rendered as if it were this one's.
#
# That is the same append defect that once turned five pooled runs into a single
# "1 of 200" figure, and it is worse here: an ATTESTATION is the one artefact
# whose whole job is to say which code produced the run beside it. A stale one
# does not merely mislead, it certifies the wrong thing.
#
# Usage: bash last-attest.sh <arm-attest.txt>
set -u
F="${1:-}"
[ -n "$F" ] && [ -f "$F" ] || { echo "REFUSED: no such attestation file: ${F:-<none>}"; exit 2; }
N=$(grep -c 'RUN ATTESTATION' "$F")
[ "$N" -gt 0 ] || { echo "REFUSED: $F contains no attestation block"; exit 2; }
# Say how many were skipped, so "one run" and "the newest of nine" never render
# identically -- the same reason last-run.sh prints its run count.
echo "# $F: $N attestation block(s); showing the LAST"
awk -v want="$N" '/RUN ATTESTATION/{n++} n==want' "$F"
