#!/usr/bin/env bash
# THE RUN LEDGER: which scored runs accounted for what they started?
#
# WHY: a runner can source `launch-preamble.sh` and forget
# `launch-postamble.sh` -- the same "depends on the operator remembering"
# defect both halves exist to remove, one level up. The pair does not check
# itself.
#
# It needs no new emit. Every gated run already writes `<TAG>-attest.txt`
# (proof it STARTED something) and `<TAG>.log` (where a teardown outcome
# WOULD appear). A tag with the first and not the second is a run that
# started something and never accounted for it.
#
# THIS REPORTS ACCOUNTING, NOT ORPHANS. An UNACCOUNTED tag means the run
# did not RECORD an outcome; the server may well have exited on its own.
# The two tags known to have orphaned (`wit`, holding a port for 52
# minutes, and `live8`) are known because they were OBSERVED, not because
# they were unaccounted -- and conflating the two would be claiming a
# measurement the artefacts do not contain.
#
# Usage:  run-ledger.sh [evidence-dir]
set -u
EV="${1:-${PIT_EV:-/e/veloren-master/bastion-test-evidence}}"

acc=0; un=0; fail=0; n=0
printf "%-12s %-14s %s\n" TAG STATE EVIDENCE
printf "%-12s %-14s %s\n" ------------ -------------- --------
for a in "$EV"/*-attest.txt; do
  [ -e "$a" ] || continue
  base=$(basename "$a"); TAG=${base%-attest.txt}; n=$((n+1))
  LOG="$EV/$TAG.log"
  if [ ! -f "$LOG" ]; then
    # ABSENT IS NOT CLEAN. No log at all cannot be reported as accounted;
    # it is the strongest form of "no outcome was recorded".
    un=$((un+1)); printf "%-12s %-14s %s\n" "$TAG" "UNACCOUNTED" "no $TAG.log at all"
  elif grep -q "TEARDOWN FAILED\|CLEANUP FAILED" "$LOG"; then
    fail=$((fail+1)); printf "%-12s %-14s %s\n" "$TAG" "FAILED" "$(grep -m1 'FAILED' "$LOG" | cut -c1-58)"
  elif grep -q "teardown verified\|cleanup verified" "$LOG"; then
    acc=$((acc+1)); printf "%-12s %-14s %s\n" "$TAG" "ACCOUNTED" "$(grep -m1 'verified' "$LOG" | cut -c1-58)"
  else
    un=$((un+1)); printf "%-12s %-14s %s\n" "$TAG" "UNACCOUNTED" "no teardown line in $TAG.log"
  fi
done

echo
echo "ACCOUNTED $acc  ·  UNACCOUNTED $un  ·  FAILED $fail   of $n attested tags"
[ $((acc+un+fail)) -eq $n ] && echo "states sum to the denominator" \
  || echo "!! STATES DO NOT SUM -- a tag was dropped"

# ---------------------------------------------------------------------------
# DECLARED OUTPUTS -- a SECOND, INDEPENDENT question about the same tags.
#
# The block above asks "did the run record a teardown outcome?". This asks
# "can its evidence files be found at all?" -- and until 2026-08-15 the answer
# was no, for every tag. `$TAG.log` was GUESSED here; the four other files a
# run writes were nowhere, and reconciling 24 attested tags against 143 server
# logs on disk came back VOID because the attestation named none of them.
#
# A DECLARATION IS A CLAIM, NOT EVIDENCE. The preamble writes these paths
# before the run starts, so each one is a promise about a file that does not
# exist yet. Every path is therefore resolved against the disk here, and a
# declared-but-absent path is reported BY NAME. A field that recorded promises
# and never checked them would be worse than no field -- it would look like
# provenance, which is the objection `attest-run.sh`'s own header raises
# against the build-script flag it declined to implement.
#
# AND AN UNDECLARED TAG IS NOT A ZERO-OUTPUT TAG. Attestations written before
# the field existed carry no `output :` lines; they render UNDECLARED, never
# `0 outputs`. The 143 pre-field logs stay honestly unreachable -- a
# forward-looking field cannot retro-link evidence it never saw.
#
# `sort -u` because `attest-run.sh` APPENDS: a re-run tag's file holds several
# attestation blocks, so the same path appears once per run.
echo
printf "%-16s %s\n" TAG OUTPUTS
printf "%-16s %s\n" ---------------- --------
dec=0; undec=0; pres=0; absent=0; empty_ok=0; empty_bad=0
for a in "$EV"/*-attest.txt; do
  [ -e "$a" ] || continue
  base=$(basename "$a"); TAG=${base%-attest.txt}
  # SET HERE, NOT INHERITED. The first version of this block read `$LOG` from
  # the accounting loop above, where it holds whatever the LAST tag processed
  # there set it to -- so every EMPTY declaration was tested against one
  # unrelated file and the matched control was flagged as a liar. Caught by the
  # control, which is the only reason a bar has one.
  LOG="$EV/$TAG.log"
  mapfile -t PATHS < <(grep '^output  *: ' "$a" | sed 's/^output  *: //' | sort -u)
  if [ "${#PATHS[@]}" -eq 0 ]; then
    # THREE CASES, NOT TWO. `declared EMPTY` and a pre-field attestation both
    # have zero `output :` lines, and the first version of this block reported
    # BOTH as "attestation predates the field" -- false for the first, and the
    # same absence-vs-exclusion collapse this ledger exists to prevent, one
    # level up. The summary line is what separates them.
    if grep -q '^outputs  *: declared EMPTY' "$a"; then
      # AND THE ONE CHECKABLE CONTRADICTION. `attest-run.sh` cannot detect a
      # false EMPTY at launch time: the run has not happened, so there is
      # nothing to compare the claim against. Here it has. A tag that said it
      # would write nothing, whose own `$TAG.log` exists, wrote a file it
      # declared it would not.
      #
      # THIS REPORTS, IT DOES NOT REFUSE. Voiding a completed run over a
      # bookkeeping mismatch would destroy evidence to punish a label, and the
      # run itself may be perfectly good.
      #
      # ITS REACH IS ONE FILE. `$TAG.log` is the only output this ledger can
      # name without a declaration -- the very guess whose insufficiency
      # voided the reconciliation row. A false EMPTY that writes only
      # `server-<TAG>.log` is NOT caught, and that is a limit, not an oversight.
      if [ -f "$LOG" ]; then
        empty_bad=$((empty_bad+1))
        printf "%-16s %s\n" "$TAG" "!! DECLARED EMPTY but $TAG.log EXISTS"
      else
        empty_ok=$((empty_ok+1)); printf "%-16s %s\n" "$TAG" "declared EMPTY (consistent)"
      fi
      continue
    fi
    undec=$((undec+1)); printf "%-16s %s\n" "$TAG" "UNDECLARED (attestation predates the field)"
    continue
  fi
  dec=$((dec+1)); miss=()
  for p in "${PATHS[@]}"; do
    if [ -f "$p" ]; then pres=$((pres+1)); else absent=$((absent+1)); miss+=("$p"); fi
  done
  if [ "${#miss[@]}" -eq 0 ]; then
    printf "%-16s %s\n" "$TAG" "PRESENT ${#PATHS[@]}/${#PATHS[@]}"
  else
    printf "%-16s %s\n" "$TAG" "MISSING ${#miss[@]}/${#PATHS[@]}"
    for m in "${miss[@]}"; do printf "%-16s   !! declared but absent: %s\n" "" "$m"; done
  fi
done
echo
echo "tags declaring paths $dec  ·  declared EMPTY $((empty_ok+empty_bad)) (contradicted $empty_bad)  ·  UNDECLARED $undec   of $n"
echo "declared paths: PRESENT $pres  ·  ABSENT $absent"
[ $((dec+undec+empty_ok+empty_bad)) -eq $n ] && echo "output states sum to the denominator" \
  || echo "!! OUTPUT STATES DO NOT SUM -- a tag was dropped"
