#!/usr/bin/env bash
# THE SHARED MULTI-ARM (FAN) LAUNCH TEMPLATE for scored Bastion runs.
#
# WHY THIS EXISTS: seven launchers fan one binary across parallel arms with
# per-arm logs named `server-<FAM>-<ARM>.log` -- the exact naming the ledger
# reconciliation proved unreachable (the `powered` attestation cannot find
# `server-pw-*.log` by any rule). The family attestation here DECLARES every
# arm's outputs, so the link is recorded instead of living in launcher source.
#
# THE PID CAPTURE IS THE MEASURED ONE. `run-prio-arms.sh` wrote `$!` from a
# subshell with no `exec` into a `.pid-` file; the pid named the subshell and
# the kill reached nothing (measured in run-live-check.sh: `$!` said 42308,
# the server was 33664). `exec` makes `$!` the server. The teardown witness
# is the PORT, never the pid table -- MSYS and Windows pids are different
# namespaces and two rows of analysis died on that.
#
# PER-ARM PORT TRIPLES ARE DERIVED. The producer rewrote only the game port,
# leaving all concurrent arms sharing web 14005 and query 14006 (the query
# socket dies with one `error!` line and the run continues -- silent).
# WEB=PORT+1, QUERY=PORT+2 per arm, both settings files rewritten per arm.
#
# Interface, from a wrapper:
#     FAM=prio2
#     ARMS="prioA:14104:script-prio-A.txt prioB:14204:script-prio-B.txt ..."
#     . "$EV/run-template-fan.sh"
# Optional: PORT_WAIT (default 900s per arm), BASTION_EXTRA.
#
# HOLD-EXTENSION-PREREG.md adds two pieces for the headless shape:
#   - a script of `-` launches NO driver for that arm, and says so explicitly
#     in the family log ("<arm> headless (no driver)") -- a skipped driver
#     and a crashed driver must never render identically;
#   - HOLD_TICK=N (optional): after drivers (if any) exit, each arm's server
#     log is polled (ANSI-stripped, last `tick=`) until it passes N or
#     HOLD_WAIT (default 3600s) expires, emitting "hold reached tick=T" or
#     "!! hold TIMEOUT at tick=T" per arm BEFORE its teardown line. This is
#     the piece whose absence left four f1 servers running per invocation
#     under a comment claiming they were stopped.
#
# File layout per arm: server-$FAM-$ARM.log, driver-$FAM-$ARM.log,
# driverout-$FAM-$ARM.log, admin-$FAM-$ARM.log; plus $FAM.log for the family.
set -u
WT=/e/veloren-master/.engine-integration-wt
EV=/e/veloren-master/bastion-test-evidence
B=$WT/target/no_overflow
A=E:/veloren-master/.engine-integration-wt/assets
: "${FAM:?wrapper must set FAM}"; : "${ARMS:?wrapper must set ARMS}"
PORT_WAIT="${PORT_WAIT:-900}"

export BASTION_ENV="BASTION_DETERMINISTIC=1 BASTION_AUTOFOUND_COLONY=8 BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1${BASTION_EXTRA:+ $BASTION_EXTRA}"

# EVERY output of every arm, declared before any exists. This is the line
# that closes the reconciliation's root cause for fans: the per-arm names
# differ from the family tag, and only a declaration can relate them.
BASTION_LOGS="$EV/$FAM.log"
for spec in $ARMS; do
  ARM=${spec%%:*}; rest=${spec#*:}; rest2=${rest#*:}
  # Same 4-field parse as the driver loop below: the script is the third
  # field, never the third-plus-fourth.
  SCRIPT=${rest2%%:*}
  BASTION_LOGS="$BASTION_LOGS $EV/server-$FAM-$ARM.log $EV/admin-$FAM-$ARM.log"
  # A `-` arm writes no driver files, so it must not PROMISE any -- the
  # ledger checks every declared path against the disk, and a headless arm
  # declaring driver logs would be MISSING by its own declaration.
  if [ "$SCRIPT" != "-" ]; then
    BASTION_LOGS="$BASTION_LOGS $EV/driver-$FAM-$ARM.log $EV/driverout-$FAM-$ARM.log"
  fi
done
export BASTION_LOGS

# VALIDATE EVERY ARM SPEC BEFORE LAUNCHING ANYTHING. The malformed-env
# refusal originally sat in the driver loop -- after six servers were
# already up -- so a refused parse would have orphaned every one of them.
# A gate that fires after the side effects is not a gate.
for spec in $ARMS; do
  rest=${spec#*:}; rest2=${rest#*:}; SCRIPT=${rest2%%:*}
  if [ "$rest2" != "$SCRIPT" ]; then
    case "${rest2#*:}" in
      [A-Z_][A-Z_0-9]*=*) ;;
      *) echo "!! MALFORMED arm env in spec '$spec' -- refusing before launch" >&2; exit 2 ;;
    esac
  fi
done

TAG="$FAM"
. "$EV/launch-preamble.sh"

# ---- launch every arm ----
PIDS=""   # "arm:port:pid" triples, the only record the teardown trusts
for spec in $ARMS; do
  ARM=${spec%%:*}; rest=${spec#*:}; PORT=${rest%%:*}
  WEBP=$((PORT+1)); QUERYP=$((PORT+2))
  UD="E:/veloren-master/.engine-integration-wt/userdata-$FAM-$ARM"
  rm -rf "$WT/userdata-$FAM-$ARM"
  VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A "$B/veloren-server-cli.exe" \
      --no-auth admin add "$ARM" admin > "$EV/admin-$FAM-$ARM.log" 2>&1
  S=$WT/userdata-$FAM-$ARM/server/server_config/settings.ron
  sed -i "s/:14004\"/:$PORT\"/g; s/:14006\"/:$QUERYP\"/g" "$S"
  sed "s/:14005\"/:$WEBP\"/" "$WT/userdata-$FAM-$ARM/server-cli/settings.template.ron" \
      > "$WT/userdata-$FAM-$ARM/server-cli/settings.ron"
  ( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A \
      exec env $BASTION_ENV \
      "$B/veloren-server-cli.exe" --no-auth > "$EV/server-$FAM-$ARM.log" 2>&1 ) &
  PIDS="$PIDS $ARM:$PORT:$!"
done
echo "family $FAM arms:$PIDS (started by this template)" > "$EV/$FAM.log"

# ---- wait for every port (arms boot concurrently; waits overlap) ----
for entry in $PIDS; do
  ARM=${entry%%:*}; rest=${entry#*:}; PORT=${rest%%:*}
  t=0
  while [ $t -lt "$PORT_WAIT" ]; do
    if (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then exec 3<&- 3>&-; break; fi
    sleep 3; t=$((t+3))
  done
  echo "$ARM port $PORT open after ${t}s" >> "$EV/$FAM.log"
done

# ---- drivers in parallel ----
# WAIT ON THE DRIVER PIDS BY NAME, never bare `wait`: a bare `wait` includes
# the server jobs, which do not exit until killed -- so it blocks until
# something external kills a server. The producer used a bare `wait` with
# its servers as live jobs of the same shell; whether its kills were ever
# reached under its own power is not answerable from the artefacts (it
# writes no family log at all), and this template does not inherit the
# question.
DPIDS=""
for spec in $ARMS; do
  ARM=${spec%%:*}; rest=${spec#*:}; PORT=${rest%%:*}; rest2=${rest#*:}
  SCRIPT=${rest2%%:*}
  # Optional 4th field (LAST-FANS-PREREG.md): a per-arm DRIVER env var,
  # `tag:port:script:VAR=value`. Driver only, NEVER the server -- the server
  # env is BASTION_ENV and attested; a per-arm server env would silently
  # fork the attested config. A malformed 4th field REFUSES at parse: an arm
  # silently launched without its env is a wrong-scored arm, worse than a
  # dead one.
  # Validated pre-launch; extracted here.
  ARMENV=""
  [ "$rest2" != "$SCRIPT" ] && ARMENV=${rest2#*:}
  # `-` = headless arm: no driver at all. The EXPLICIT line is the point --
  # a skipped driver and a crashed driver must never render identically.
  if [ "$SCRIPT" = "-" ]; then
    echo "$ARM headless (no driver)" >> "$EV/$FAM.log"
    continue
  fi
  ( env $ARMENV "$B/bastion_playtest.exe" "127.0.0.1:$PORT" "$ARM" \
      "$EV/$SCRIPT" "$EV/driver-$FAM-$ARM.log" \
      > "$EV/driverout-$FAM-$ARM.log" 2>&1 ) &
  DPIDS="$DPIDS $!"
done
if [ -n "$DPIDS" ]; then
  # shellcheck disable=SC2086
  wait $DPIDS
  echo "all drivers exited" >> "$EV/$FAM.log"
else
  echo "no drivers to wait for (all arms headless)" >> "$EV/$FAM.log"
fi

# ---- optional hold: run every arm's server PAST a tick before teardown ----
# The piece whose absence orphaned four f1 servers per invocation under a
# comment claiming they were stopped. Poll code is the same ANSI-strip +
# last-tick extraction the four hold-shaped launchers already carry.
if [ -n "${HOLD_TICK:-}" ]; then
  HOLD_WAIT="${HOLD_WAIT:-3600}"
  for entry in $PIDS; do
    ARM=${entry%%:*}
    t=0; last=""
    while [ $t -lt "$HOLD_WAIT" ]; do
      last=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/server-$FAM-$ARM.log" 2>/dev/null \
             | grep -oE 'tick=[0-9]+' | tail -1 | cut -d= -f2)
      [ -n "${last:-}" ] && [ "$last" -gt "$HOLD_TICK" ] && break
      sleep 10; t=$((t+10))
    done
    if [ -n "${last:-}" ] && [ "$last" -gt "$HOLD_TICK" ]; then
      echo "$ARM hold reached tick=$last (bound $HOLD_TICK)" >> "$EV/$FAM.log"
    else
      echo "!! $ARM hold TIMEOUT at tick=${last:-none} (bound $HOLD_TICK after ${t}s)" >> "$EV/$FAM.log"
    fi
  done
fi

# ---- teardown: one three-outcome line PER ARM, port-witnessed ----
for entry in $PIDS; do
  ARM=${entry%%:*}; rest=${entry#*:}; PORT=${rest%%:*}; SPID=${rest#*:}
  if [ -z "$SPID" ]; then
    echo "!! $ARM: NO SERVER PID RECORDED -- teardown not attempted" >> "$EV/$FAM.log"
    continue
  fi
  kill "$SPID" 2>/dev/null
  wait "$SPID" 2>/dev/null
  if (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then
    exec 3<&- 3>&-
    echo "!! TEARDOWN FAILED: $ARM port $PORT still held after kill pid=$SPID" >> "$EV/$FAM.log"
  else
    echo "teardown verified: $ARM pid=$SPID stopped and port $PORT is free" >> "$EV/$FAM.log"
  fi
done
