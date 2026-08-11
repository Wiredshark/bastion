# ITEM 8 (ENDURANCE RUN) — v4, on the famine fix (routes 1-3 + F5/F6 + S1)

**Cleared by Opus's commit review of `b96830d161`** (registry fix separately
at `48a6579f62`) — "Good series. Nothing held back, one deviation,
flagged." Fable ruled the launch sequence go on Opus's approval.
Supersedes v3 (`bastion-test-evidence/ITEM8-V3-*`), whose famine (28
orphaned SOW claims, 0% food recovery after tick 99300) is what routes
1-3 target, and whose binary was wrong for the crash-fix question
entirely (`ITEM8-V3-WRONG-BINARY-FINDING.md`).

## THE PIN

**Fix pin this run launches against:** `b96830d1` — read fresh via
`git rev-parse --short=8 HEAD` at launch time, **verified against the
boot log's own `Server version:` line before founding began (Gate-0)**:

    Server version: b96830d1 [2026-08-11]

Exact match, confirmed by grep, not assumed.

## THE REBUILD (§4, checklist entry 5)

**Verified from the build's own `Compiling` list, not the command or
exit code:**

    Compiling veloren-server-cli v0.18.0 (...\server-cli)

Binary mtime `Aug 11 14:51` (was `13:46` before this rebuild, confirming
an actual relink). **Gate-0 dry-run performed first**, on a disposable,
never-scored userdata dir, killed via `reap-server.sh` — clean SIGTERM
exit, no escalation needed — before this scored boot.

## THE ACTUAL LAUNCH

**Boot config, read live, DIFFED against v3's own boot log:**

    hunger_decay_per_sec=0.000889  hunger_interrupt=0.2  hunger_comfort=0.5
    rest_decay_per_sec=0.000444    rest_interrupt=0.2    rest_comfort=0.5

**Identical to v3, no delta** — expected, since routes 1-3 touch claim
lifecycle and job sweeping, not mood/decay config. **New this run** (the
ITEM8-V4 effective-config line, emitted unconditionally at boot per the
packet's checklist entry 5):

    generic_claim_leak_secs=1860.0  colony_terminal_zero_streak_samples=10

`1860.0` = `2 × access_stall_secs()` at its default (930.0, no env
override) — confirms F6's threshold resolved as designed, no surprise
value.

`Authentication is disabled` confirmed. Assets resolved:
`E:/veloren-master/.engine-integration-wt/assets`.

**Userdata:** fresh dir, `userdata-item8-endurance-v4/`.
`BASTION_ENTITY_EVENT_LOG=1`. `VELOREN_ASSETS` pinned to this worktree.
`BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG` deliberately NOT set, same
rationale as v2/v3.

**Founding** (`script-15-item8-endurance.txt`, unchanged): spawn
`(15216.5, 16016.5, 419.0)`, stockpile/farm/bed designated, registered
(`rev=3`). No `give_item`/`dropall` — founding stock only.

**Verified before ending the launch turn (Fable's standing rule):**

    colonist promoted to loaded:  8
    colonist demoted:             0
    food stock sample heartbeat:  firing (9 hits inside the founding window)
    client connect+disconnect:    1 pair, both before the scored window
                                   connect    2026-08-11T18:53:34.265468Z
                                   disconnect 2026-08-11T18:54:39.570589Z

## TWO CLOCKS (per Opus's terminology note — every timestamp above is one
of these; never write "the run started at" without saying which)

    PROCESS START:       2026-08-11T18:52:50.414036Z (14:52:50 EDT)
    SCORED-WINDOW START: 2026-08-11T18:54:39.570589Z (14:54:39 EDT, driver disconnect)

**Launch time (driver disconnect, the releasing event):**
`2026-08-11 18:54:39.570589 UTC` ≈ **14:54:39 EDT**.

**Expected end (5 cycles, ≈30 sim-min each, at the 30-TPS baseline —
NOT the retired 31.8 ticks/s figure):** ≈2.5 wall-hours from launch ⇒
**≈17:24 EDT / ≈21:24 UTC**, continuing to 7 cycles (≈3.5h / ≈18:24 EDT)
if healthy — score registers at 5 regardless.

## THE BAR — F1-F5 from the packet, F6 added

    F1  farm completions > 0                          (the spine)
    F2  no immortal jobs
    F3  cells recycle
    F4  food produced
    F5  claim-expiry events (unreachable-gated) > 0, zero = VOID not PASS
    F6  generic leak-witness backstop: zero firings = expected PASS,
        any firing = a RECORDED FINDING, never absorbed

**Read `board.claim_expiry_releases`, `board.designated_sweep_reaps`,
`board.generic_claim_leak_releases`, and `board.b5_split_off_one_fired`
off the `"bastion food stock sample"` heartbeat line** (all four now
ride it, per-300-tick) — no end-of-run-only scalar this time.

## WHAT WOULD VOID THIS RUN

- **F5 == 0 across the whole scored window**: the fix's own precondition
  never exercised — VOID on the crash-adjacent claim, not a pass.
- **F6 > 0 at any point**: not a run failure, but a FINDING — a leak
  route the targeted fix doesn't cover exists, report it, don't patch
  around it mid-run.
- **`Server version` in this log ever fails to match `b96830d1`**:
  cannot happen absent a mid-run relink, which the "one binary for the
  whole run" precondition rules out by construction — named as a
  sanity check, not expected to fire.

## WAKE PLAN (Fable's standing rule, adopted)

Fable's own heartbeat wakes this session (early-crash check ~30min in,
health looks, run-end kick at the bar's end) — no self-scheduled wakeup
chain held open for the full duration. Server identified by
`VELOREN_USERDATA=...userdata-item8-endurance-v4` and boot timestamp
`18:52:50 UTC` in the process's own log.
