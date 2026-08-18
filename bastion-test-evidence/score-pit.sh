#!/usr/bin/env bash
# PRE-SPECIFIED SCORING for the vertical-fixture row. Written BEFORE the
# run, against the driver's format READ from source, not guessed:
#
#   bastion_playtest.rs:711 --
#   "INSPECT uid={} ... energy={:.4} ... activity={:?} status={:?} ..."
#
# `status` is Option<BastionColonistStatus>, so `{:?}` renders exactly
# `status=None` or `status=Some(RestingToClimb)`. Five naming failures in
# this session came from patterns invented at scoring time; this one is
# fixed in advance and cites the line it was derived from.
#
# THE PRECONDITION IS PRINTED ABOVE THE RESULT. `energy` is the same
# payload field the gate reads: `route_energy_ready` is
# `energy.current() >= energy.maximum()`, i.e. FULL. The inspector reports
# energy as a 0..1 fraction, so `energy=1.0000` means the gate is CLOSED and
# `RestingToClimb` is impossible by construction. That distinguishes:
#
#   VOID -- energy never left full: the precondition never held
#   RED  -- energy below full AND grounded, yet status stayed None
#
# Without that split, a run where nobody ever spent energy would score
# identically to a run where the field is broken.
#
# Usage:  score-pit.sh <driver-log>
set -u
LOG="${1:?usage: score-pit.sh <driver-log>}"
[ -f "$LOG" ] || { echo "!! NO DRIVER LOG AT $LOG -- the leg produced no samples"; exit 2; }

# ANSI is stripped defensively: the driver writes plain text today, but the
# server log's tracing output is coloured, and a pattern that silently stops
# matching after a formatter change is the failure this guards against.
# BOUND TO ONE RUN: this reads a DRIVER log, and `bastion_playtest` appends
# (DRIVER-LOG-APPEND-AMENDMENT.md -- driver-pit-shaft.log reached five runs
# and inflated a denominator 5x). `last-run.sh` refuses a headerless file
# rather than assuming one run, and this inherits that refusal.
RUNS=$(bash "$(dirname "$0")/last-run.sh" "$LOG" --count) || exit $?
[ "$RUNS" -gt 1 ] && echo "note: $LOG holds $RUNS runs -- scoring the LAST one only"
CLEAN=$(bash "$(dirname "$0")/last-run.sh" "$LOG" | sed 's/\x1b\[[0-9;]*m//g')

SAMPLES=$(printf '%s\n' "$CLEAN" | grep -c "INSPECT uid=")
echo "=== PRECONDITION ==="
if [ "$SAMPLES" -eq 0 ]; then
  echo "  colonist samples: 0  -> VOID: the driver never observed a colonist"
  exit 2
fi
FULL=$(printf '%s\n' "$CLEAN" | grep "INSPECT uid=" | grep -c "energy=1\.0000")
BELOW=$((SAMPLES - FULL))
echo "  colonist samples      : $SAMPLES   (denominator: one per colonist per inspect)"
echo "  samples at FULL energy: $FULL       -> gate CLOSED, RestingToClimb impossible"
echo "  samples below full    : $BELOW      -> gate OPEN on these"

echo "=== RESULT ==="
# ANCHORED ON `INSPECT uid=` (fixed 2026-08-15, after the pit arm): the
# unanchored form counted a `[note]` line from the SCRIPT -- my own sentence
# "every colonist should read status=None before any pit work" -- as a
# sample, reporting 41 statuses against 40 INSPECT lines.
#
# The verdict was unaffected (it keys on `Some(` counts and on energy, and
# no note mentions `status=Some(`), but a status count that includes the
# operator's own prose is not a measurement. Disclosed rather than quietly
# corrected: the numbers below changed after data existed.
PAYLOAD=$(printf '%s\n' "$CLEAN" | grep "INSPECT uid=")
W=$(printf '%s\n' "$PAYLOAD" | grep -c "status=Some(RestingToClimb)")
ANY=$(printf '%s\n' "$PAYLOAD" | grep -c "status=Some(")
printf '  status=Some(RestingToClimb) : %s  of %s samples\n' "$W" "$SAMPLES"
printf '  status=Some(<any variant>)  : %s\n' "$ANY"
printf '  status=None                 : %s\n' \
  "$(printf '%s\n' "$PAYLOAD" | grep -c 'status=None')"

echo "=== VERDICT ==="
if [ "$W" -gt 0 ]; then
  echo "  V1 PASS -- the witness fired ($W of $SAMPLES samples)"
elif [ "$BELOW" -eq 0 ]; then
  echo "  V1 VOID -- energy never left full in any sample; the gate never opened."
  echo "            NOT a failure of the field: its precondition did not hold."
else
  echo "  V1 NOT OBSERVED -- $BELOW sample(s) had the energy gate open and still read None."
  echo "            Report as PRECONDITION UNMET on the emergency-route arm unless an"
  echo "            emergency access job is independently shown to have existed:"
  echo "            energy is only ONE of the gate's conditions."
fi
