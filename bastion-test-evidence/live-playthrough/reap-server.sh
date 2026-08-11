#!/usr/bin/env bash
# reap-server.sh -- idempotent teardown for a bastion-server run.
#
# Fable's ask (parallel-fill during item8 v3, in-tree, DO NOT EXECUTE
# against the live run): the driver already self-terminates by design
# (its script ends, the process exits) but nothing kills the SERVER --
# v2's server died on its own (the split_off_one crash) and nobody
# reaped it; the complementary case is a server that SURVIVES past its
# expected end and is never told to stop. This script covers both: kill
# it if it's alive, no-op if it's already dead, safe to call any number
# of times.
#
# Convention this pairs with (launch-side, not built here): the launch
# command backgrounds the server and writes its PID to
# "<userdata_dir>/server.pid" via `echo $! > "$pidfile"` immediately
# after backgrounding. This script only consumes that file -- it does
# not launch anything.
#
# Usage: reap-server.sh <pidfile>
# Exit 0: nothing to do, or the process was killed cleanly.
# Exit 1: the process was still alive after SIGTERM+SIGKILL (a real
#         failure worth reporting, not silently swallowed).

set -u

pidfile="${1:?usage: reap-server.sh <pidfile>}"

if [[ ! -f "$pidfile" ]]; then
    echo "reap-server: no pidfile at $pidfile -- nothing to reap"
    exit 0
fi

pid="$(cat "$pidfile")"

if [[ -z "$pid" ]]; then
    echo "reap-server: pidfile $pidfile is empty -- treating as nothing to reap"
    rm -f "$pidfile"
    exit 0
fi

if ! kill -0 "$pid" 2>/dev/null; then
    echo "reap-server: pid $pid (from $pidfile) is already dead -- no-op"
    rm -f "$pidfile"
    exit 0
fi

echo "reap-server: pid $pid is alive, sending SIGTERM"
kill "$pid" 2>/dev/null

# Poll for graceful exit before escalating -- SIGKILL first is what
# leaves a corrupted save mid-write; give it a real window.
for _ in $(seq 1 20); do
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "reap-server: pid $pid exited cleanly after SIGTERM"
        rm -f "$pidfile"
        exit 0
    fi
    sleep 0.5
done

echo "reap-server: pid $pid still alive after 10s, escalating to SIGKILL"
kill -9 "$pid" 2>/dev/null
sleep 1

if kill -0 "$pid" 2>/dev/null; then
    echo "reap-server: pid $pid STILL ALIVE after SIGKILL -- reap failed, reporting not swallowing"
    exit 1
fi

echo "reap-server: pid $pid killed via SIGKILL"
rm -f "$pidfile"
exit 0
