# `reap-server.sh` — idempotent server teardown (in-tree, unexercised against
# a real server by design)

**Fable's ask** (parallel-fill during item 8 v3, zero contact with the live
server): the driver already self-terminates by design when its script ends;
nothing kills the SERVER. v2 proved the failure mode both directions —
its server died on its own (the `split_off_one` crash) and nobody reaped it,
and a healthy run that *survives* past its expected end has the same gap in
the other direction: nobody tells it to stop either. `reap-server.sh` covers
both with one idempotent operation: kill it if it's alive, no-op if it's
already dead.

## THE CONVENTION IT PAIRS WITH (launch-side, not built here)

The script only *consumes* a PID file — it does not launch anything. The
launch command is expected to background the server and write its PID
immediately:

    ./target/no_overflow/veloren-server-cli.exe --no-auth ... &
    echo $! > "$USERDATA_DIR/server.pid"

This session's own launches used the harness's own `run_in_background`
tracking instead of a literal `&`/`$!` pair, so no launch in this repo
currently writes this file — the next launch that wants automated teardown
should adopt the convention above (or an equivalent PID capture) rather
than relying on `ps`-matching by boot timestamp, which is what
`ITEM8-LAUNCH-RECORD-V3.md`'s own wake plan currently falls back to.

## TESTED — against a synthetic dummy process only, never the live v3 server

Four cases run and confirmed, all against `sleep 300 &` (a disposable dummy
process) or a fabricated PID, immediately followed by a live check that the
real `veloren-server-cli` process (PID unchanged, boot timestamp unchanged)
was never touched:

1. **No pidfile** — exit 0, "nothing to reap".
2. **Empty pidfile** — exit 0, "nothing to reap".
3. **Alive dummy process** — SIGTERM sent, process confirmed exited within
   the poll window, pidfile removed, exit 0.
4. **Stale pidfile (PID that doesn't exist)** — exit 0, "already dead"
   no-op, pidfile removed.

## UPDATE — first live exercise against a real server, 2026-08-11 (post-v3)

**Approved and run: the gate-0 rebuild verification boot doubled as the
sacrificial dry run.** Freshly rebuilt `veloren-server-cli.exe` (pin
`5845b680`) booted in a disposable userdata dir
(`userdata-gate0-sacrificial/`, never scored, deleted after), PID
captured via `echo $! > sacrificial-gate0.pid` per the documented
convention, killed via `reap-server.sh sacrificial-gate0.pid`:

    reap-server: pid 1953 is alive, sending SIGTERM
    reap-server: pid 1953 exited cleanly after SIGTERM

**Graceful SIGTERM was sufficient — the SIGKILL escalation path still
was not exercised** (nothing to escalate to; the process exited within
the poll window on its own). The DB-flush/socket-close question this
section originally posed is now answered for the plain-SIGTERM case: a
real server with real shutdown work still exits cleanly on SIGTERM
within the existing ~10s poll window. **What remains genuinely untested:
a server killed mid-save (e.g. SIGTERM sent during an active DB write)
and the SIGKILL escalation branch itself** — neither was exercised here
since this sacrificial boot was brief and idle. Left open, not assumed
safe, same discipline as before.

## WHAT WAS NOT TESTED (original list, partially superseded above)

The SIGTERM→poll→SIGKILL escalation path (line ~55 of the script, the
10-second-then-SIGKILL branch) remains unexercised — no process in either
test round needed escalation. Whether SIGKILL mid-save-flush is safe is
still open.
