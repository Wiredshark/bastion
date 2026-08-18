#!/usr/bin/env bash
# THE SHARED TEARDOWN for scored Bastion runs -- counterpart to
# `launch-preamble.sh`. The runner set had a shared HEAD and no TAIL.
#
# WHY THIS EXISTS: measured across the runner set, 15 of 17 server-launchers
# never attempt to stop what they start, and THIRTEEN of them end on the
# identical two lines (driver call, then `echo "driver exited rc=$?"`). They
# share that tail because they were copied from one another -- which is also
# why the missing teardown, the missing attestation and the missing config
# record all live in the same thirteen places. A copied template propagates
# its holes with perfect fidelity, so the fix belongs in a template, not in
# fifteen edits.
#
# The one pre-existing attempt (`run-prio-arms.sh`) does `echo $! > .pid-TAG`
# then `kill "$(cat .pid-TAG)"` and never checks. Writing `$!` to a file and
# reading it back changes nothing about what the pid NAMES -- and `$!` is the
# capture that did not reach the server when this was first measured.
#
# THE PORT IS THE WITNESS, NOT THE PID. The port is what actually blocks the
# next run, and it is observable without any pid-namespace reasoning --
# which is exactly where two rows of analysis went wrong (an MSYS pid was
# checked against the Windows process table, twice, and read as a failure).
#
# THREE OUTCOMES, DELIBERATELY DISTINCT. "Tore down nothing because there was
# nothing to tear down" and "tore down nothing because the pid was never
# captured" are different facts and must not render identically. This is the
# same unset-vs-empty distinction `attest-run.sh` makes for the config -- a
# law this project registered one row before violating it in the teardown
# classification, which is why it is written into the code here.
#
# Usage, from a runner:
#     SRV=$!                       # whatever pid the runner captured
#     . "$EV/launch-postamble.sh"  # stop + verify + record
#
# Requires, from the sourcing runner: EV, TAG, GAME. SRV may be unset.

if [ -z "${SRV+x}" ] || [ -z "$SRV" ]; then
  # NOT the same as "nothing was running" -- the runner never recorded a pid,
  # so this script cannot even attempt a stop, and saying "verified" here
  # would be a clean-looking report of an unperformed action.
  echo "!! NO SERVER PID RECORDED -- teardown not attempted for $TAG" >> "$EV/$TAG.log"
  echo "!! $TAG: no pid recorded; anything it started is still running" >&2
else
  # ONLY the pid the runner recorded. Never by port and never by name:
  # killing whatever holds a port would violate the rule that a process you
  # did not start is not yours to stop.
  kill "$SRV" 2>/dev/null
  wait "$SRV" 2>/dev/null

  if (exec 3<>"/dev/tcp/127.0.0.1/$GAME") 2>/dev/null; then
    exec 3<&- 3>&-
    echo "!! TEARDOWN FAILED: port $GAME still held after kill pid=$SRV" >> "$EV/$TAG.log"
    echo "!! $TAG: an orphan is holding $GAME -- stop it before the next leg" >&2
  else
    echo "teardown verified: pid=$SRV stopped and port $GAME is free" >> "$EV/$TAG.log"
  fi
fi
