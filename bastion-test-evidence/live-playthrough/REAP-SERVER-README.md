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

## WHAT IS NOT TESTED, NAMED HONESTLY

**Never run against an actual `veloren-server-cli` process** — per the
explicit constraint on this fill item. The SIGTERM→poll→SIGKILL escalation
path (line ~55 of the script, the 10-second-then-SIGKILL branch) is
therefore unexercised against a process that has real teardown work to do
(flushing the DB, closing sockets) rather than a bare `sleep`. Whether 10
seconds is enough for a real server's graceful shutdown, and whether
SIGKILL mid-save-flush is safe, are both open — untested by design, not
assumed safe. Worth a v4-scoped dry run against a killed-and-verified
server once this run lands, not before.
