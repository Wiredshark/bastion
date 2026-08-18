#!/usr/bin/env bash
# THE ENDURANCE TEARDOWN (#73; ENDURANCE-TEARDOWN-PREREG.md).
#
# Scripts the checklist's fragile minutes: kill BOTH processes with a
# three-outcome verdict each, then capture integrity -- timestamp, final
# heartbeat, md5 of the raw log after the writer is dead, last-line
# completeness. "#73 exists because one of them has been left behind
# before" -- and a teardown verified by 'the command returned' is exactly
# the verification standard this project has spent a week refusing.
#
# TOUCHES NOTHING IT SHOULDN'T: there is no userdata argument, so the
# checklist's "do not clean, do not tidy" law cannot be violated here by
# construction. The capture artefacts land beside the log.
#
# PIDS ARE PASSED, NEVER DISCOVERED: the operator provides what the
# launcher recorded. Killing a guessed pid is the never-kill-what-you-
# did-not-start rule, one step removed.
#
# usage: endurance-teardown.sh <server-pid> <driver-pid> <raw-log> [game-port]
set -u
SRV="${1:?usage: endurance-teardown.sh <server-pid> <driver-pid> <raw-log> [game-port]}"
DRV="${2:?driver pid required (pass 0 if the driver already exited and you know it)}"
LOG="${3:?raw log path required}"
PORT="${4:-}"

if [ ! -f "$LOG" ]; then
  echo "!! NO LOG AT $LOG -- refusing: a teardown that cannot capture is a kill, not a teardown" >&2
  exit 2
fi

echo "=== ENDURANCE TEARDOWN $(date '+%F %T') ==="
echo "teardown timestamp (scored window END): $(date '+%F %T')"

# FINAL HEARTBEAT BEFORE THE KILL: the last counter values are the run's
# terminal state and cannot be recovered from a partial log.
echo "final line before kill:"
tail -1 "$LOG" | sed 's/\x1b\[[0-9;]*m//g' | cut -c1-200 | sed 's/^/  /'

# ---- the kills: three outcomes each, never 'the command returned' ----
stop_one() { # name pid
  local NAME=$1 PID=$2
  if [ "$PID" = "0" ]; then
    echo "$NAME: pid 0 passed -- operator asserts it already exited (recorded, not verified)"
    return 0
  fi
  if ! kill -0 "$PID" 2>/dev/null; then
    echo "$NAME: pid $PID already dead before teardown"
    return 0
  fi
  kill "$PID" 2>/dev/null
  # bounded wait for exit; then the verdict comes from the process table
  local t=0
  while [ $t -lt 30 ]; do
    kill -0 "$PID" 2>/dev/null || break
    sleep 1; t=$((t+1))
  done
  if kill -0 "$PID" 2>/dev/null; then
    echo "!! $NAME: pid $PID STILL ALIVE ${t}s after kill -- teardown NOT complete"
    return 1
  fi
  echo "$NAME: pid $PID killed and gone (verified by process table, ${t}s)"
}

RC=0
stop_one server "$SRV" || RC=1
stop_one driver "$DRV" || RC=1

if [ -n "$PORT" ]; then
  if (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then
    exec 3<&- 3>&-
    echo "!! port $PORT STILL HELD -- something is listening after the kills"; RC=1
  else
    echo "port $PORT is free"
  fi
fi

# ---- capture integrity, AFTER the writer is dead ----
sleep 1
MD5=$(md5sum "$LOG" | cut -d' ' -f1)
SIZE=$(stat -c %s "$LOG")
echo "raw log: $LOG"
echo "  md5    : $MD5"
echo "  bytes  : $SIZE"
# LAST-LINE COMPLETENESS: a half-written final line is the signature of a
# kill that raced the writer. A complete log ends in a newline.
if [ "$(tail -c 1 "$LOG" | od -An -c | tr -d ' ')" = "\n" ]; then
  echo "  last line: COMPLETE (ends in newline)"
else
  echo "  !! last line: TRUNCATED (no terminating newline) -- the kill raced the writer"
  RC=1
fi
{
  echo "=== TEARDOWN CAPTURE $(date '+%F %T') ==="
  echo "log=$LOG md5=$MD5 bytes=$SIZE rc=$RC"
} >> "$LOG.teardown"
echo "capture record appended to $LOG.teardown"

echo
echo "ORDER (checklist section 4): teardown -> capture -> md5 verify -> COMMIT -> only then idle legs."
echo "userdata: NOT TOUCHED by this script, by construction. Preserve it untouched."
exit $RC
