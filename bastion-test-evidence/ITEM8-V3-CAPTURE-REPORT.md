# ITEM 8 v3 — TEARDOWN + CAPTURE REPORT

**Pin:** `f5267f15bb` (fix + all prereg amendments) / `0f7d9bfacd` (current
tip at capture time — mute-channel addendum, unrelated to the fix).
**Teardown per Fable's run-end kick**: manual v2 procedure, `reap-server.sh`
NOT used against this run (banned per the pre-ruling — its first live
exercise is the post-v3 sacrificial dry run).

## TWO CLOCKS (every timestamp below is one of these — never write "the run
started at" without saying which)

    PROCESS START:      2026-08-11T14:46:51Z  (10:46:51 EDT)
    SCORED-WINDOW START: 2026-08-11T14:48:31.162203Z (10:48:31 EDT, driver disconnect)
    KILL SENT:           2026-08-11T17:20:48.322Z (13:20:48 EDT)
    FINAL TICK LOGGED:   274200 (2026-08-11T17:20:41.233095Z server-side)

## ★★★ INSTRUMENTATION GAP FOUND AT CAPTURE TIME — reported now, not buried

**`board.b5_split_off_one_fired` (the precondition-witness counter the
whole v3 prereg's registered scoring rule depends on) was never wired to
any log emission, anywhere — only declared and incremented
(`bastion_jobs.rs:4436`, `:13757`).** Confirmed via
`grep -c "b5_split_off_one_fired" server-stdout-item8-endurance-v3.log` →
**0**. Unlike `preempt_attempts`, which has a `tick.0 % 300 == 0` periodic
`info!` emission (`bastion_jobs.rs:6341-6347`) making its final value
readable from a killed server's log, `b5_split_off_one_fired` has no such
line. **The same is true for `b5_pile_pickup_by_member`** (0 matches) and
the `ItemEventKind::PickedUp` entity-event-log records (the
`BASTION_ENTITY_EVENT_LOG` ring buffer is in-memory only — confirmed no
output file exists anywhere under `userdata-item8-endurance-v3/`).

**Consequence for the registered prediction
(`ITEM8-V3-PREREGISTRATION.md`'s three-way scoring table):** the
CONSEQUENCE half is answerable and clean — zero `panicked`/`debug_assert`
lines across the entire 274,200-tick, ~2h34m run (`grep -c "panicked"` → 0).
**The PRECONDITION half (`b5_split_off_one_fired > 0`?) is UNRECOVERABLE
from this run, not "read as 0."** This is a different failure mode than
the one the registered rule anticipated — the rule assumed the value would
be *readable and might read 0* (VOID on that reading); it did not
anticipate the value being *unreadable at all* because I never built the
periodic emission. **This is my own process gap, named as such: I built
the counter and the scoring rule around it, and skipped the observation
mechanism** — exactly the "enumerate what the instrument can see" /
"a sampled field is not a trace" lesson this session has applied to other
people's work, missed on my own.

**What can still be said, and how it's qualified:** 40 `"ate — hunger
restored"` completions occurred (15:01:47–15:41:06Z) against a founding
stock that peaked at 19 units (tick 45300) before the famine cascade —
population and mechanism both make it *plausible* most of those eats
crossed a multi-unit pile and therefore took the `Some`/split path, but
**this is inference from adjacent numbers, not a read of the field the
prereg registered.** Per this session's own standing law, a plausible
story is not evidence for a registered precondition. **Scoring this run's
crash-fix claim on the precondition-witness criterion specifically:
VOID BY MISSING INSTRUMENT, not PASS, not FAIL.** The consequence half
(no panic across 3× v2's fuse length) stands on its own regardless.

**Possible future recovery, not attempted here:** `userdata-item8-
endurance-v3/server/save_universe/epochs/*/payload-*.bin` are rtsim's
periodic world-state snapshots and may contain enough persisted
`PickupItem` structure across epochs to reconstruct split activity after
the fact — untried, would need a dedicated offline reader, named as a
lead for whoever next needs this specific number rather than left
unmentioned.

**Fix for the next run, not done here (would touch the live-run code
path, out of scope for a teardown):** add a `tick.0 % 300 == 0` periodic
`info!` line for `b5_split_off_one_fired` (and ideally
`b5_pile_pickup_by_member`) mirroring `preempt_attempts`'s existing
pattern, before any v4 that needs this number to be scoreable from a log
alone.

## THE FAMINE CASCADE (Fable's health-look finding, confirmed from the
full log at capture)

    farm jobs 104/106/107 created:  14:53:47Z / 14:54:29Z / 14:54:31Z (last farm activity of the entire run)
    sow job completions, any:        0 (`grep -c "sow.*complet"` → 0)
    food stock peak:                 19 units, tick 45300 (~15:12:31Z)
    food stock reaches 0:            tick 99300 (~15:42:44Z), never recovers for the remaining ~1h38m
    last "ate" completion:           15:41:06Z (uid 75, job 183) -- 1 eat before the 0-floor, consistent
    total eats this run:             40 (vs. 16 in the ~15min colony-presence-acceptance leg -- rate is far lower per elapsed time, consistent with famine)
    total sleeps this run:           48
    BREAKDOWN (despondent) events:   331
    preempt_attempts (final):        502
    colonist demotions:              0 (ROW-COLONY-PRESENCE fix held for the full run)
    emergency egress "no route" events present near end of log (uid 73 GOTO-STAND-RESCUE at kill time) -- consistent with a colony in distress, not examined further here (post-mortem territory, not teardown territory)

**Why sows never completed is explicitly NOT diagnosed in this report** —
that is the post-mortem's job, reading from the preserved userdata (board
state, job claims) alongside this log, per Fable's instruction.

## LOG STABILITY (capture protocol, v2-identical)

    read 1 (immediately post-kill): 1,123,528 bytes
    read 2 (~5s later):             1,123,528 bytes
    read 3 (~9s later):             1,123,528 bytes -- stable, three consecutive reads agree

Server process (PID 51100 / msys pid 749) confirmed gone via `ps` after
`kill`. stderr log: 0 bytes throughout (no crash, no OS-level error output).

**Log size note:** 1.12 MB total, well under the ~52-75 MB Fable
estimated pre-run. Consistent with v2's own rate (287 KB / ~23.6 min ≈
12 KB/min; 1.12 MB / ~154 min ≈ 7.3 KB/min — same order of magnitude,
somewhat lower per-minute here, plausibly because the famine-stalled
colony produced fewer eat/sleep/haul completions to log than an active
one). Not a truncation — the tail of the log is coherent, ends mid-tick
with the expected shutdown-adjacent lines (egress retry, GOTO-STAND-RESCUE),
no abrupt cutoff mid-line.

## PROVENANCE — the raw-commit decision was pre-ruled

Per Fable/Opus's pre-decision (ruled before teardown, not chosen under
teardown-time pressure): **the full server log is committed RAW — no
gzip, no filtered extract** — because this is the first full-length
endurance artifact and nobody can predict which lines the eventual
post-mortem needs; a filter's exclusions are invisible exactly when they
matter. `userdata-item8-endurance-v3/` is committed UNTOUCHED alongside
it as first-class evidence for the why-no-sows post-mortem (board state,
job claims, saves) per the same pre-ruling.

**★ Size note, disclosed rather than assumed pre-cleared:** the pre-
ruling's size estimate (~52-75 MB) was scoped to the log; the log itself
landed smaller (1.12 MB, see above). **`userdata-item8-endurance-v3/` is
779 MB across 474 files** — almost entirely `server/save_universe/epochs/
*/payload-*.bin` (rtsim's periodic world-state snapshots, ~5.15 MB each,
one set roughly every ~2 minutes across the ~2.5h run). **No single file
exceeds GitHub's 100 MB hard per-file push limit** (max observed: 5.15
MB), so the push itself is not blocked, but the total is an order of
magnitude larger than anything discussed before teardown. Committing it
anyway per the letter of "commit userdata untouched, first-class
evidence" — flagging the actual number here rather than silently
under-representing it, since "under GitHub's limit" was said about a
different artifact.

## INFORMAL LOG-USAGE NOTE (for a future filter spec, per Fable's ask)

Sections of this log actually read for this report: `"bastion food stock
sample"` (trend), `"ate — hunger restored"`/`"slept — rest restored"`
(measures 2/3), `"farm job created"` (sow diagnosis lead), `"BREAKDOWN"`
(despondency), `"bastion preempt attempts"` (telemetry), `"demoted"`
(measure 1), `panicked`/`debug_assert` (crash-fix consequence half),
first/last timestamp (duration). Never read: the bulk of per-tick
job-arbitration `info!` lines that make up most of the file's bytes.
