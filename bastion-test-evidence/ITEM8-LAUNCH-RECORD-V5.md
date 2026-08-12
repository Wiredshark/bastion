# ITEM 8 (ENDURANCE RUN) — v5, on the mine-fix cluster (defect 2 + honest
completion metric + all four watchdog gates)

**Cleared by Opus's review of `e60e34ec5d`/`4d9180252f`** ("REVIEW PASS DONE
— GREEN LIGHT. PROCEED TO v5."). Supersedes v4 (`ITEM8-LAUNCH-RECORD-V4.md`),
whose famine was the indestructible-mine-cell loop
(`ROW-INDESTRUCTIBLE-MINE-CELL.md`), not claim lifetimes — routes 1-3/F5/F6
from v3/v4 are confirmed correct fixes for real defects that were never
what stopped the farm.

## THE PIN

**Fix pin this run launches against:** `4d918025` — read fresh via
`git rev-parse --short=8 HEAD` at launch time, verified against the boot
log's own `Server version:` line TWICE before founding began (double
gate-0, Fable-ruled):

    gate-0 #1 (disposable):  Server version: 4d918025 [2026-08-11]
    gate-0 #2 (disposable, with v5's real launch env vars):
                              Server version: 4d918025 [2026-08-11]

Both exact matches, confirmed by grep, not assumed. Full gate-0 logs
preserved at `bastion-test-evidence/ITEM8-V5-GATE0/gate0-{1,2}-stdout.log`.

## THE REBUILD

**Verified from the build's own `Compiling` list, not the command or exit
code:**

    Compiling veloren-server-cli v0.18.0 (...\server-cli)

0 errors. Binary mtime `Aug 11 19:53` (confirming an actual relink from the
prior session's binary).

## THE ACTUAL LAUNCH

**Boot config, read live from gate-0 #2 (identical env to the scored
launch):**

    hunger_decay_per_sec=0.000889  rest_decay_per_sec=0.000444
    generic_claim_leak_secs=1860.0  colony_terminal_zero_streak_samples=10

Identical to v3/v4 — no drift, expected since this arc's fixes touch
mine-completion/egress lifecycle, not decay/leak config.

**Cosmetic note (Opus, gate-0 review):** the raw log line this is read from
still says `"bastion effective ITEM8-V4 config"` — a stale label from when
these two thresholds were introduced, not a claim about which fix's
config is running. Substantively correct (the thresholds are deliberately
unchanged for v5); flagged so a future reader of the raw log doesn't
misread the label as "v5 is running v4's code." Not changed mid-run.

`Authentication is disabled` confirmed. Assets resolved:
`E:/veloren-master/.engine-integration-wt/assets`.

**Userdata:** fresh dir, `userdata-item8-endurance-v5/`.
`BASTION_ENTITY_EVENT_LOG=1`. **`BASTION_EGRESS_DIAG=1`** — new for v5, per
Opus's explicit precondition: this is the diagnostic that was OFF for the
entire v4 run and is why the wipe-reason trace ("stuck_watch wiped") never
appeared anywhere in 945K lines despite the backstop being disarmed the
whole time. On for v5 so the reconnected failsafe chain (bounded replan →
STICKY release → net accrues → teleport → bar lifts on surface) is
observable, not just code-read-verified.

**Founding** (`script-15-item8-endurance.txt`, unchanged): spawn
`(15216.5, 16016.5, 419.0)`, stockpile/farm/bed designated, registered
(`rev=3`). No `give_item`/`dropall` — founding stock only.

**Verified before ending the launch turn:**

    colonist promoted to loaded:  8
    colonist demoted:             0
    food stock sample heartbeat:  firing (8 hits inside the founding window)
    client connect+disconnect:    1 pair, both before the scored window
                                   connect    2026-08-11T23:55:39.026245Z
                                   disconnect 2026-08-11T23:56:43.678282Z
    panics / debug_assert fires:  0
    stderr:                       0 bytes

## TWO CLOCKS

    PROCESS START:       2026-08-11T23:55:10.897657Z (19:55:10 EDT)
    SCORED-WINDOW START: 2026-08-11T23:56:43.678282Z (19:56:43 EDT, driver disconnect)

**v5 flies REAL TIME** — the impossibility for compressed mode is named and
proven this same session (chunk-gen → colonist-promotion wall-coupling,
measured across three live runs: promotion-complete tick 624/192/2184 at
identical raw-tick comparison, diverging by construction under
`BASTION_UNCAPPED_TPS`). `BASTION_UNCAPPED_TPS` is deliberately UNSET for
this run.

**Expected end (5 cycles, ≈30 sim-min each, at the 30-TPS baseline):**
≈2.5 wall-hours from launch ⇒ **≈22:26 EDT / ≈02:26 UTC (2026-08-12)**,
continuing to 7 cycles (≈3.5h) if healthy — score registers at 5
regardless.

## THE BAR — v5's F1 amended, per Fable's pre-registered ruling

    F1  GENERATION-2 completions > 0     (amended: "completions > 0" alone
                                           is satisfiable by a dead colony,
                                           per v4's own 361 phantom
                                           completions with zero real
                                           production — completions must be
                                           attributable to Farm's till/sow/
                                           harvest cycle continuing PAST the
                                           founding-stock wave, not merely
                                           counted)
    F2  no immortal jobs
    F3  cells recycle
    F4  food produced
    F5  claim-expiry events (unreachable-gated) > 0, zero = VOID not PASS
    F6  generic leak-witness backstop: zero firings = expected PASS,
        any firing = a RECORDED FINDING, never absorbed
    F7  no single position accounts for >10% of completions (ITEM8-V5-
        PACKET.md, d3c54e461b -- calibrated by v4 itself: 143/145 = 98.6%
        at one cell would have failed this at ANY reasonable threshold)
    F8  "job completed" fires ONLY for completions with a world-effect
        (the metric fix landed in e60e34ec5d/4d9180252f -- this bar is
        what SCORES it live, not just unit-tests it)

**Opus's pre-data correction, registered here before any data exists**
(entry 7's "what observation would make this go red" applied to the bar
tier itself): my first draft of this record had F9/F10 as separate,
uncalibrated bars duplicating F7/F8 with vaguer thresholds ("a handful is
normal, hundreds is defect 1" has no number). Corrected:

    emergency_access_completions -- a MEASURE, reported alongside real
        production every heartbeat, NEVER scored as pass/fail. Nonzero is
        expected and uninformative alone (it just means the exhaustion
        bound fired at least once, which the bar's F5 already covers).
    F10 folded into F7 -- same threshold (>10%), no separate name.

## WHAT WOULD VOID THIS RUN

- **F5 == 0 across the whole scored window**: the fix's own precondition
  never exercised — VOID on the crash-adjacent claim, not a pass.
- **F6 > 0 at any point**: not a run failure, but a FINDING.
- **`Server version` in this log ever fails to match `4d918025`**: cannot
  happen absent a mid-run relink, which the "one binary for the whole run"
  precondition rules out by construction.
- **F7 fails (a single position accounts for >10% of completions)**: would
  mean defect 1 recurred through a path the exhaustion bound doesn't
  reach — a genuine new finding, not absorbed into F1. Calibrated by v4
  itself: 143/145 = 98.6% at one cell would have failed this immediately.
- **F8 fails (`job completed` fires for a suppressed-effect completion)**:
  the metric fix (`e60e34ec5d`/`4d9180252f`) regressed live, not just in
  the unit tests that already cover it.

## WHAT THIS RUN ANSWERS AS A BY-PRODUCT (no separate live capture, per Opus)

- **Defect 1's mechanism** — `completed_kind` on every "job completed" line
  decides whether the block-write ever lands (constant `Rock` = write never
  sticks) or something else re-fills it (anything else = names the culprit).
- **The `job.progress`-ticking question** — whether progress can advance on
  an emergency-access claim the colonist can't service; the same open
  question the `"work-progress"` watchdog gate itself raised.
- **The reconnected failsafe chain, observed not just read** —
  `BASTION_EGRESS_DIAG=1`'s wipe-reason trace + any teleport firings prove
  (or refute) the 7-link chain Opus traced by code read: bounded replan →
  STICKY release → net accrues → teleport → `emergency_reengage_exhausted`
  lifted on safe surface arrival.

## WAKE PLAN

Standing heartbeat pattern (early-crash check ~30min in, health looks,
run-end kick at the bar's end). Server identified by
`VELOREN_USERDATA=...userdata-item8-endurance-v5` and boot timestamp
`23:55:10 UTC` in the process's own log.
