# Milestone Live Session — AUTON-2 unification + STEP-3 RE-TUNE (2026-08-09)

Driven by `client/src/bin/bastion_playtest.rs` (extended this session with a
`cmd <name> <args...>` script verb — a raw chat-command send, needed to
provision food; see below) against a real hosted `server-cli` instance.
Scripts: `script-09-milestone.txt` (Run A/B), `script-10-milestone-food.txt`
(Run C). Raw logs (server stdout/stderr, driver logs, per-run userdata) are
intentionally NOT committed — kept out per instruction, large and
ephemeral. (Note for the record: the repo's own `.gitignore` actually
un-ignores `bastion-test-evidence/**/*.log`, and Run 1/2's raw logs from
2026-08-04 ARE tracked in git — so "gitignored as before" isn't quite
literally true of the current tree. Following the intent regardless: this
scorecard + the two scripts + the driver code change are what's committed.)

## Setup

- Engine tip: `2adb6d0651` (AUTON-2 STEP-3 RE-TUNE), on `bastion/wip-batch-verify`,
  in `.engine-integration-wt`. Attested by grepping the compiled
  `veloren-server-cli.exe` for `BASTION_SELFJOB_COMPLETION_DIAG` (1 hit,
  matches all 6 source call sites) — NOT the version banner, which happened
  to read correctly this run (`2adb6d06`) but per Run 1's own lesson isn't
  trusted on that basis.
- `BASTION_ROWB_BENCH`: left unset (shipping default) — explicit choice,
  unrelated to this row.
- `--no-auth`, fresh `VELOREN_USERDATA` per run. Zero stderr across all
  three runs — no crash (the Run-1 P0 disconnect fix still holds).
- World seed / spawn point: same deterministic `(15216.5, 16016.5, 419.0)`
  as Run 1/2.

## Run A — script-09-milestone.txt, undiagnosed (~25 min before stopped)

8 colonists, same footprint as script-02 + extended checkpoints to ≥35min.
**Result: 8/8 colonists cycled into repeated Despond breakdowns** (17+
events, ~60s apart matching `despond_secs`), never once eating or
sleeping. Zero `need preempt` lines fired the entire run. Stopped early
once the pattern was clear, in favor of a targeted diagnostic re-run.

## Run B — same script, diagnostic re-run (`BASTION_ARB_PERSONAL_DIAG=1
BASTION_NEED_SKIP_DIAG=1`)

- **The arbiter mechanism works, and on schedule.** Hunger tracked the
  retune's own arithmetic almost exactly (0.441 at tick 20086 ≈ 11.2min,
  vs. predicted 0.405 — close given per-colonist trait-stagger). Severity
  stayed 0.0 / `no_need_below_interrupt` correctly until hunger crossed its
  staggered interrupt right around the predicted ~15min mark, at which
  point `Drive::Personal` engaged (confirmed via the need-check pass
  advancing past the interrupt gate into food search).
- **Root cause of Run A's non-observation: no food source existed in the
  scenario.** Mine/chop/build/farm(pre-existing-broken)/stockpile/ladder —
  none produce food, and `bastion_playtest` had no item-spawn verb.
  `reason=no_food_found` fired 13,000+ times.
- **A separate, real finding underneath that gap**: the need-check pass
  ranks candidates by raw severity (`sort_by` ascending, most-depleted
  first) and only ever tries `candidates.first()`. When hunger is that
  candidate and its search dead-ends, the pass `continue`s — it never
  falls through to try the second candidate (rest) in the same pass, even
  after rest also crossed interrupt with 8 free beds sitting unoccupied
  the whole time. `reason=preempt_cooldown_active` dominated from ~30min
  on with rest never once attempted in this run. Escalated to Fable by
  Opus as a shared-arbitration design question (source confirmed the gap;
  the design intent behind "most urgent wins" is silent on the dead-end
  case — not a documented decision).

## Run C — script-10-milestone-food.txt, food-provisioned re-run (~45 min)

Adds `cmd give_item common.items.food.mushroom 40` + `cmd dropall` at the
anchor point before spawning the colony (requires the connecting player
hold the `admin` role — `server-cli admin add <user> admin` against the
same `VELOREN_USERDATA` before boot), plus a resupply at the ~15min
checkpoint. Isolates rest without touching the Run B arbitration question.

**Row 6 (bed/sleep) — OBSERVED, full cycle, live, on the world's own
clock:**

- 8/8 colonists' rest preempted correctly (`need preempt — rest below
  interrupt`) once it became their most urgent need.
- **5/8 completed the full cycle end-to-end**: preempt → travel → arrive
  (`colonist arrived at job site, working (B5)`) → sleep → `slept — rest
  restored` (jobs 841/842/843/845/846, all five distinct beds). This is
  exactly what the milestone was scoped to close.
- The remaining 3/8 (colonists 77/78/79) plus colonist 74's own rest
  attempt fired in the final ~2 minutes of the 45-minute window and simply
  ran out of observation time — not confirmed stuck, just not resolved
  before disconnect.

**A third, independent confirmation of the pre-existing TRAVEL ROW
defect**, exactly as anticipated going in: colonist 74's hunger preempt
(the ONE hunger attempt that fired all session, targeting a real,
correctly-provisioned food item) got permanently stuck — 11 `ULTIMATE
FAIL-SAFE — teleporting stuck colonist to ground` events over 12 minutes
on the same `EatFrom` job (`terminal_cause: "below_grade_watch_without_
egress_verdict"`) before giving up into Despond. Two other colonists
(75, 78) hit the same fail-safe on unrelated jobs (a wall-climb egress
timeout, a stalled haul). 13 fail-safe events total, 75 despondency events
total across the ~45min run — most of the colony spent much of the window
in a despond/recover cycle before the food supply and later reclaim
attempts let 5 colonists complete cleanly. This corroborates Opus's own
harness measurement ("3 of 4 rest attempts never complete") from the game
side, independently, via a different instrument.

## Driver extension

`bastion_playtest.rs` gained a `cmd <name> <args...>` script verb — sends
a raw chat command via the same `Client::send_command` wire path a real
player's chat bar uses. Used here for `give_item`/`dropall` to provision
food; admin-gated like any other privileged command, so the connecting
player needs the role pre-seeded via `server-cli admin add`.

## Summary verdict

- STEP-3 RE-TUNE: confirmed working correctly and on-schedule, live.
- Milestone's primary gate (row 6, bed/sleep use-cycle): **CLOSED** — 5/8
  colonists completed it end-to-end in Run C.
- Two real findings surfaced that no existing fixture had exercised:
  1. Candidate-starvation in the need-check pass (Run B) — escalated to
     Fable, not fixed here.
  2. Live, third-instrument confirmation of the TRAVEL ROW's known
     arrival defect, now shown blocking `EatFrom` as well as `RestAt`
     (previously only measured against rest/bed targets).
