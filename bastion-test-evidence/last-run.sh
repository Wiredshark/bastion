#!/usr/bin/env bash
# Emit ONLY the most recent run from an APPENDING driver log.
#
# WHY THIS EXISTS: `bastion_playtest` APPENDS to the log path it is given, so
# a tag re-run leaves every previous leg in the same file. Measured the hard
# way -- `driver-pit-shaft.log` held FIVE runs, and a disposition reported
# "1 of 200" for what was five legs of 40 pooled across four different
# binaries (DRIVER-LOG-APPEND-AMENDMENT.md). The sighting was real; the
# DENOMINATOR was fiction.
#
# This is the same law as `count-at-tick.sh`'s tick bound and the
# `[note]`-line census: A COUNT IS ONLY A MEASUREMENT IF ITS POPULATION IS
# BOUNDED. There the bound was a tick, here it is a run.
#
# THE RUN BOUNDARY IS THE DRIVER'S OWN HEADER, not a timestamp or a file
# size: `=== bastion_playtest starting:` is written once per invocation
# before anything else, so it is the only marker that cannot drift from the
# thing it delimits.
#
# REFUSES rather than guessing: a log with no header at all is not "one
# run", it is a file whose structure this tool does not recognise -- and
# silently treating it as a single run is exactly the false-denominator
# failure above, one level down.
#
# usage: last-run.sh <driver-log>          # prints the last run's lines
#        last-run.sh <driver-log> --count  # prints the number of runs
set -u
LOG="${1:?usage: last-run.sh <driver-log> [--count]}"
MODE="${2:-lines}"

[ -f "$LOG" ] || { echo "!! NO LOG AT $LOG -- refusing" >&2; exit 2; }

N=$(grep -c 'bastion_playtest starting' "$LOG" || true)
if [ "$N" -eq 0 ]; then
  echo "!! NO RUN HEADER in $LOG -- refusing to call an unrecognised file one run" >&2
  exit 3
fi

if [ "$MODE" = "--count" ]; then
  echo "$N"
  exit 0
fi

# The last header's line number; everything from there to EOF is the last run.
START=$(grep -n 'bastion_playtest starting' "$LOG" | tail -1 | cut -d: -f1)
tail -n "+$START" "$LOG"
