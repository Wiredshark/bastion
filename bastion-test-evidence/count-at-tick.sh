#!/usr/bin/env bash
# Count an event in a server log UP TO A GIVEN TICK.
#
# WHY THIS EXISTS: `grep -c` over a whole server log is not a windowed count.
# The server keeps simulating after the driver disconnects, so the same log
# yields different numbers depending on WHEN you grep it -- pwA2 read 24 hauls
# at scoring time and 50 once its server had run on unattended. A count with no
# tick bound is not a measurement of the scored window.
#
# `haul deposited` lines carry no tick of their own, so the tick is carried
# forward from the most recent line that does have one (`tick=N`).
#
# usage: count-at-tick.sh <logfile> <cut_tick> [pattern]
set -u
LOG=$1; CUT=$2; PAT=${3:-haul deposited}

# AND A MISSING LOG IS NOT A ZERO. Until 2026-08-15 this script printed `0`
# and exited `0` for a log that did not exist -- byte-identical to the output
# for a log that existed and contained no matches. The only difference was a
# `sed: can't read` on stderr, which any caller redirecting stderr or piping
# stdout loses entirely.
#
# THE IRONY IS THE POINT: this file exists because an unwindowed `grep -c`
# produced a false COUNT (see the header above). It then shipped a false ZERO
# for two months. Fixing one failure mode in an instrument does not inoculate
# it against the neighbouring one -- and the neighbour here is the same law,
# one step over: an absent input and a measured zero must never render
# identically.
#
# Found by auditing all nine evidence-directory scorers for exactly this
# (ABSENT-VS-EMPTY-RESULTS.md); 8 of 9 already distinguished, this was the one.
if [ ! -f "$LOG" ]; then
  echo "!! NO LOG AT $LOG -- refusing to report a count for a file that does not exist" >&2
  exit 2
fi
sed 's/\x1b\[[0-9;]*m//g' "$LOG" | awk -v cut="$CUT" -v pat="$PAT" '
  match($0, /tick=[0-9]+/) {
    t = substr($0, RSTART+5, RLENGTH-5) + 0
  }
  index($0, pat) > 0 {
    # a hit before any tick line at all is pre-tick-0 and counts as in-window
    if (t <= cut) n++
  }
  END { print n+0 }
'
