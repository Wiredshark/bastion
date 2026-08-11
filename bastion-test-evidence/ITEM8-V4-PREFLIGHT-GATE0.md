# ITEM 8 v4 PREFLIGHT — Gate-0 rebuild verification + anomaly close-out

Fable's three-task ruling, all closed here, reported as one.

## TASK 1 — REBUILD, VERIFIED FROM BUILD OUTPUT NOT COMMAND LINE

**Command (unambiguous this time — no `--bin` filter, single `-p`):**

    cargo build --profile no_overflow -p veloren-server-cli

**Verified from the build's own `Compiling` lines**, not from exit code or
the command I typed:

    Compiling veloren-server-cli v0.18.0 (E:\veloren-master\.engine-integration-wt\server-cli)

`bastion-server` does **not** appear in this build's `Compiling` list —
checked, not assumed innocent. Read as: cargo determined `bastion-server`
was already fingerprint-fresh (this session's many `cargo check -p
bastion-server` runs share the same profile's fingerprint cache with
`cargo build`), so only the final binary crate needed compiling+linking.
**This reasoning is not what closes the verification — the stamp
check below is.** Binary mtime confirms an actual relink occurred:

    target/no_overflow/veloren-server-cli.exe -- Aug 11 13:46 EDT (was 08:07 before)

## TASK 1b — GATE-0 STAMP VERIFICATION (the mandatory preflight, now itself
tested)

Booted the freshly-built binary in a disposable, never-scored userdata
dir (`userdata-gate0-sacrificial/`, deleted after), `--no-auth`,
`--` no other flags:

    Server version: 5845b680 [2026-08-11]

**Matches `git rev-parse --short=8 HEAD` (`5845b680`) exactly.** Gate-0
passes: this binary is built from the intended tip. This is the same
check the mandatory preflight gate will run on every future launch (one
`grep "Server version"` against the intended pin, before the launch turn
ends) — exercised here for the first time and it worked as designed.

**This boot also served as `reap-server.sh`'s first live exercise**
(approved for exactly this — "a sacrificial server you launch for the
purpose, never a scored one"): PID captured via the documented
`echo $! > pidfile` convention, killed via
`reap-server.sh sacrificial-gate0.pid` — clean SIGTERM exit, no
escalation needed. Full result and the honest remaining gap (SIGKILL
path, mid-save kill) written up in `REAP-SERVER-README.md`'s update
section.

## TASK 2 — THE "50× LOG-RATE ANOMALY" DOES NOT EXIST

**Root cause of the false alarm, found precisely:** `ITEM8-LAUNCH-RECORD-
V2.md`'s own "Userdata" paragraph mentions *"27,640 lines / 8.25MB in the
15-minute row-acceptance leg"* — **as the stated reason
`BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG` was left unset for v2**, citing
a *different* run's log size to justify a config choice. Read out of
context, that sentence looks like it's describing v2's own log. It is
not — it is describing the row-acceptance leg (`script-19`,
`server-stdout-colony-presence-acceptance-v3.log`, confirmed
independently at exactly 8,252,395 bytes with 27,640
`COLONY-PRESENCE-ACCEPTANCE-DIAG` lines, that diagnostic firing
unconditionally every pass). **Both Opus and Fable read this the same
way independently — a genuinely easy misread, not a fabrication, and the
day's "a right number from the wrong run" pattern one more time.**

**The two runs actually being compared (v2 vs. v3) have identical
env/config, confirmed by direct diff of both launch records:**

| | v2 | v3 |
|---|---|---|
| `hunger_decay_per_sec` | 0.000889 | 0.000889 |
| `rest_decay_per_sec` | 0.000444 | 0.000444 |
| `hunger_interrupt`/`comfort` | 0.2 / 0.5 | 0.2 / 0.5 |
| `rest_interrupt`/`comfort` | 0.2 / 0.5 | 0.2 / 0.5 |
| `BASTION_ENTITY_EVENT_LOG` | 1 | 1 |
| `VELOREN_ASSETS` | pinned to worktree | pinned to worktree |
| `BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG` | unset | unset |

**No difference exists to diff further.** Actual rates, re-verified:

    v2: 286,961 bytes / ~26 min  -> ~11.0 KB/min
    v3: 1,123,528 bytes / ~154 min -> ~7.3 KB/min

~1.5× apart — plausibly explained by v3's famine cascade producing fewer
eat/sleep/haul/farm-completion lines per minute than v2's more active
pre-crash economy, with no config difference required to explain it.
**Anomaly closed: it never existed. Not a v4 blocker.**

## TASK 3 — STATUS

Per Fable's ruling: **no v4 launch until Opus's scoring lands and the v4
packet forms** (immortal-job fix + sentinel S1 + b5 heartbeat port + this
rebuilt/verified binary + this anomaly answer). This binary
(`5845b680`, gate-0 verified) is the one v4 launches on once cleared —
not launched here.
