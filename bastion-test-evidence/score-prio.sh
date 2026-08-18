#!/usr/bin/env bash
# Score item 16 P1/P2/P3 under AMENDMENT 1: the window opens at the command's
# own witness line, NOT at server boot. Boundary is read from the log, never
# chosen -- for B/C it is the line number of the `work priority set (live
# command)` emit; for A (no command) it is the client's connect, which the
# driver reports as the sim-time the wait started.
set -u
EV=/e/veloren-master/bastion-test-evidence
strip() { sed 's/\x1b\[[0-9;]*m//g' "$1"; }

printf "%-6s %-8s %-9s %-9s %-10s %-8s\n" ARM WITNESS PRE_WIN MID_WIN POST_WIN HARVEST
for T in pwA1 pwA2 pwB1 pwB2 pwC1 pwC2; do
  L=$EV/server-pw-$T.log
  [ -f "$L" ] || { echo "$T MISSING LOG"; continue; }
  S=$(strip "$L")

  # the witness line number (0 = command never ran => arm is VOID, not red)
  WLN=$(printf '%s\n' "$S" | grep -n "work priority set (live command)" | head -1 | cut -d: -f1)
  WIT=$(printf '%s\n' "$S" | grep -c "work priority set (live command)")

  # AMENDMENT 1 + arm C: a run with TWO commands has THREE segments, and
  # collapsing them answers P2 while destroying P3. P3's window opens at the
  # RE-ENABLE witness, not at the first command -- scoring C from witness 1
  # would report 0 and read as a P3 failure on a colony that was still
  # correctly refusing to haul.
  LAST=$(printf '%s\n' "$S" | grep -n "work priority set (live command)" | tail -1 | cut -d: -f1)
  if [ -n "$WLN" ]; then
    PRE=$(printf '%s\n' "$S"  | awk -v n="$WLN" 'NR< n && /haul deposited/' | wc -l)
    # segment between the two commands (for B this is simply the tail)
    MID=$(printf '%s\n' "$S"  | awk -v a="$WLN" -v b="$LAST" 'NR>=a && NR<b && /haul deposited/' | wc -l)
    POST=$(printf '%s\n' "$S" | awk -v n="$LAST" 'NR>=n && /haul deposited/' | wc -l)
    [ "$WLN" = "$LAST" ] && MID=0
  else
    # control arm: no command exists, so the whole run is the window
    PRE=0; MID=0
    POST=$(printf '%s\n' "$S" | grep -c "haul deposited")
  fi
  HARV=$(printf '%s\n' "$S" | grep -c "harvested")
  printf "%-6s %-8s %-9s %-9s %-10s %-8s\n" "$T" "$WIT" "$PRE" "$MID" "$POST" "$HARV"
done

echo
echo "--- arm C: the re-enable must be its OWN second witness ---"
strip "$EV/server-pw-pwC1.log" | grep "work priority set (live command)" \
  | sed 's/.*bastion: /  /' | cut -c1-90
echo
echo "--- driver exit codes (a crashed driver is VOID, not a zero) ---"
grep "driver exited" "$EV/attach.log" 2>/dev/null
