# ITEM 8 (ENDURANCE RUN) — RE-LAUNCH after ROW-COLONY-PRESENCE fix

**Ruled by Fable:** relaunch now, per Opus's packet (N=5 scored/designed-to-7,
founding-default colony, fully unattended). Supersedes the killed first
attempt (`bastion-test-evidence/ITEM8-LAUNCH-RECORD.md`, found frozen in
`SimulationMode::Simulated` — that finding is what produced
ROW-COLONY-PRESENCE, `ea2cfa5192`).

**Boot stamp:** binary built from `fb9a740110f60446722f3f3416f45aa6e2a68465`
(no `.rs` changes since — confirmed via `git diff --stat` against the current
tip `ea2cfa5192`, which is docs-only). `veloren-server-cli.exe` mtime 08:07
EDT, `bastion_playtest.exe` mtime 08:06 EDT.

**Boot config, read live:**

    hunger_decay_per_sec=0.000889  hunger_interrupt=0.2  hunger_comfort=0.5
    rest_decay_per_sec=0.000444    rest_interrupt=0.2    rest_comfort=0.5

Matches the pre-registration doc's planning estimate exactly (§1 of
`ITEM8-PREFLIGHT-BAR-PREREGISTRATION.md`) — cycle length ≈1802 sim-sec
(rest full-to-interrupt).

**Userdata:** fresh dir, `userdata-item8-endurance-v2/`.
`BASTION_ENTITY_EVENT_LOG=1`. `VELOREN_ASSETS` pinned to this worktree.
**`BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG` deliberately NOT set** — that
diagnostic logs every colonist every pass unconditionally (27,640 lines /
8.25MB in the 15-minute row-acceptance leg); over a 2.5-hour endurance run it
would dominate the log. Item 8's own instrumentation (uid-tagged ate/slept,
food-stock sampler, `NeedCrossed`, `BREAKDOWN` despondency) is already
event-driven, not per-pass, and doesn't need it.

**Founding (`script-15-item8-endurance.txt`, unchanged from the first
attempt):** spawn `(15216.5, 16016.5, 419.0)`, stockpile/farm/bed designated,
registered (`rev=3`). No `give_item`/`dropall` — founding stock only.

**Verified before ending the launch turn (Fable's standing rule):**

    colonist promoted to loaded:  8
    colonist demoted:             0   <- the exact thing that failed last time
    food stock sample heartbeat:  firing (tick=3000/3300/3600)
    client connect+disconnect:    1 pair, both before the scored window

**Launch time (driver disconnect, the releasing event):** `2026-08-11
13:14:07.963 UTC` (driver-log timestamp `1786454047963`) ≈ **09:14:08 EDT**.

**Expected end (5 cycles, ≈30 sim-min each, ~1:1 wall ratio):** ≈2.5 wall-hours
from launch ⇒ **≈11:44 EDT / ≈15:44 UTC**, designed to continue to 7 cycles
(≈3.5h / ≈12:44 EDT) if healthy — score registers at 5 regardless.

**Wake plan (Fable's standing rule, adopted):** Fable's own heartbeat wakes
this session at run-end for teardown + capture + report — no self-scheduled
wakeup chain held open for the full duration. Server PID: check via `ps -W`
at wake time (not recorded here since it will have already been superseded by
whatever's running then; identify by `VELOREN_USERDATA=...userdata-item8-
endurance-v2` and boot timestamp `13:12:30 UTC` in the process's own log).
