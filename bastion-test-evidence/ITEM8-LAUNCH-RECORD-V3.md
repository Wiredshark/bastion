# ITEM 8 (ENDURANCE RUN) — v3, on the `split_off_one` crash fix

**Cleared by both reviewers:** Opus GREEN on `468fe8f07c` (commit review) and
`517cb50f6d`/`f5267f15bb` (post-review closures: exclusivity placement
verification, rate-condition text, denominator refinement, corrected TPS
baseline) — "Go." Fable's redundant green-confirm arrived in parallel.
Supersedes v2 (`bastion-test-evidence/ITEM8-LAUNCH-RECORD-V2.md`), which
crashed at tick 45000 on `PickupItem::try_merge`'s `debug_assert!` — root
cause and fix in `bastion-test-evidence/ITEM8-CRASH-FINDING.md`.

**Fix pin this run launches against:** `f5267f15bb` — read fresh via
`git rev-parse HEAD` immediately before launch (per standing rule, never
carried from memory).

## THREE LAUNCH-MECHANICS DEFECTS FOUND AND FIXED DURING THIS LAUNCH

None are in the fix under test — all three are pre-existing gaps in how
this run is invoked, surfaced because this was the first `--no-auth`-style
relaunch attempted fresh rather than copy-pasted from a still-running
example. Recorded so the next launch doesn't rediscover them:

1. **`bastion_playtest`'s CLI args are POSITIONAL, not `key=value`.**
   `server`, `username`, `script_path`, `log_path` — in that order, via
   `std::env::args()`, no `=`-splitting anywhere in the binary
   (`client/src/bin/bastion_playtest.rs:214-220`). Passing
   `script=<path>` (matching the *display* format in the driver's own log
   line, which is misleadingly `key=value`-shaped) makes the literal
   argument `"script=<path>"`, which `fs::read_to_string` then fails to
   open — `NotFound` for a relative path, `InvalidFilename` for an
   absolute Windows path (the embedded `script=` prefix breaks the drive
   specifier). Two wasted attempts before reading the source instead of
   inferring the CLI shape from a log line.
2. **`veloren-server-cli` needs an explicit `--no-auth` flag.** Without
   it, `auth_server_address` stays set and the driver's connection is
   rejected with `AuthClientError(... "username + password combination
   was incorrect or the account does not exist")` — the flag is read at
   `server-cli/src/cli.rs:147-149` / applied at
   `server-cli/src/main.rs:129-132`. v2's own launch record didn't note
   this flag explicitly; this run's first boot (killed, userdata wiped,
   rebooted) omitted it and hit exactly this error.
3. **Background-Bash cwd is not reliable for a Windows-native child
   process's own argument resolution**, even when the same session's
   `cargo build` in the same worktree succeeded moments earlier (that
   proof is about `cargo`'s own manifest discovery, not about what cwd a
   spawned child sees). Practical fix used: `cd` immediately before the
   command, verify `pwd` in the same call, use paths relative to that
   confirmed cwd for both binaries and script/log arguments — matches
   `[[memory: background-bash-cwd-reset-gotcha]]`, sharpened to note it
   can affect real argument resolution, not just be a reporting quirk.

None of the above touched server or client state — every failed attempt
below was pre-connection or pre-founding; the userdata dir was wiped and
recreated once (after the `--no-auth` miss) to guarantee a clean fresh
save, matching the "fresh userdata dir" launch requirement exactly.

## THE ACTUAL LAUNCH

**Boot config, read live** (`server-stdout-item8-endurance-v3.log`):

    hunger_decay_per_sec=0.000889  hunger_interrupt=0.2  hunger_comfort=0.5
    rest_decay_per_sec=0.000444    rest_interrupt=0.2    rest_comfort=0.5

Matches the prereg's planning estimate and v2's own boot exactly.
`Authentication is disabled` confirmed in the log. Assets resolved:
`E:/veloren-master/.engine-integration-wt/assets`.

**Userdata:** fresh dir, `userdata-item8-endurance-v3/` (wiped and
recreated once mid-launch per defect 2 above — no prior state survived
into the run that actually launched). `BASTION_ENTITY_EVENT_LOG=1`.
`VELOREN_ASSETS` pinned to this worktree.
`BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG` deliberately NOT set, same
rationale as v2 (unconditional per-pass logging would dominate a
2.5-hour log; item 8's own instrumentation is already event-driven).

**Founding** (`script-15-item8-endurance.txt`, unchanged from v2): spawn
`(15216.5, 16016.5, 419.0)`, stockpile/farm/bed designated, registered
(`rev=3`). No `give_item`/`dropall` — founding stock only.

**Verified before ending the launch turn (Fable's standing rule):**

    colonist promoted to loaded:  8
    colonist demoted:             0   <- the exact thing that failed on the very first attempt (pre-ROW-COLONY-PRESENCE)
    food stock sample heartbeat:  firing (8 hits inside the founding window)
    client connect+disconnect:    1 pair, both before the scored window
                                   connect  2026-08-11T14:47:26.147383Z
                                   disconnect 2026-08-11T14:48:31.162203Z

**Launch time (driver disconnect, the releasing event):**
`2026-08-11 14:48:31.162203 UTC` ≈ **10:48:31 EDT**.

**Expected end (5 cycles, ≈30 sim-min each):** baseline is the server's
**read** `const TPS: u64 = 30` (`server-cli/src/main.rs:49` — Opus's
correction; the ~31.8 ticks/s figure derived from v2's crash timing is
NOT used as the ratio here, since it carries an unexplained ~6% artifact
from an unstated start-point term). At the stated 1:1 wall ratio for this
run mode: ≈2.5 wall-hours from launch ⇒ **≈13:18 EDT / ≈17:18 UTC**,
designed to continue to 7 cycles (≈3.5h / ≈14:18 EDT) if healthy — score
registers at 5 regardless.

**Live health signal for this run** (optional, non-gating, per Opus's
addendum): if ticks-per-wall-second drops materially below 30 sustained,
treat it as server degradation under load — a finding in its own right.

**Precondition-witness scoring commitment (Opus's registered rule,
applies at scoring regardless of doc timing):** `b5_split_off_one_fired`
must be commensurate with measure 2's own eat count for that stretch —
splits ≪ eats means the split path was suppressed, VOID on the fix claim
for that window, not a pass. See `ITEM8-V3-PREREGISTRATION.md` for the
full rule and the `debug_assert`/VOID-on-zero three-way scoring table.

**Wake plan (Fable's standing rule, adopted):** Fable's own heartbeat
wakes this session at run-end for teardown + capture + report — no
self-scheduled wakeup chain held open for the full duration. Server
identified by `VELOREN_USERDATA=...userdata-item8-endurance-v3` and boot
timestamp `14:46:52 UTC` in the process's own log.
