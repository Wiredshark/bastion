# Starvation Fall-Through Fix — Live Verification (2026-08-09)

Constraint 5 (FR15's live leg, Fable-ruled DECISIONS #82) closed after an
extended diagnostic arc. This document is the record of that arc, not just
the result — the intermediate wrong turns are as load-bearing as the
conclusion for anyone re-deriving it later.

## The fix itself

`bastion-server/src/bastion_jobs.rs`, need-check pass: restructured the
eat/rest dispatch from a hardcoded `if want_eat {...} else {...}` into a
`for (_, kind) in candidates.iter().copied()` loop over the already
most-depleted-first sorted candidate list. Each candidate tries reclaim
then fresh-search; success `break`s; a genuine dead-end `continue`s to the
next candidate instead of `continue`ing the whole colonist.
`preempt_cooldown` (confirmed `HashMap<Uid, f64>`, per-colonist) only arms
once every candidate has been tried and at least one struck out — plain
"nothing found" arms nothing, matching pre-fix single-candidate behavior
exactly. Full detail and the harness-level verification (must-fail-first
both directions via git-stash A/B, corpus zero-drift on `b73_scenario` and
`preempt_scenario`) is in the commit message at `2dbc96127e`.

## Run A / Run B — the live foodless baseline (pre-fix)

Two live runs, `script-09-milestone.txt`, no food anywhere in the world,
against the tip that had the AUTON-2 unification + STEP-3 retune but NOT
yet this fix.

- **Run A** (~25min, no diagnostics): 8/8 colonists cycled into repeated
  Despond, zero successful preempts of either kind.
- **Run B** (diagnostics on, ran ~32min): confirmed hunger crossed its
  interrupt on schedule (~15min), `Drive::Personal` engaged, `no_food_
  found` dominated (13,075 lines), and once rest also crossed (~30min)
  `preempt_cooldown_active` dominated with rest **never** attempted despite
  8 free beds. This is constraint 3/4's "before" arm and the planted-failure
  in the raw: hunger crosses, dead-ends, and permanently starves a
  resolvable, unrelated need.

## The anomaly — driver-12

Post-fix live re-run of `script-09-milestone.txt` (40min, matched against
Run B on script/seed/asset-root/duration) produced **zero** events of any
kind — no preempts on either need, no despondency, across all 8 colonists,
the entire run. This did not match the harness fixture's clean pass, and
triggered an extended live investigation:

1. **Wrong-tree asset resolution** (hypothesized, then measured false):
   `Assets found path=` in driver-12's own log names the correct tree; the
   tree's RON, confirmed via `git show`, carries the retuned rates; a fresh
   `needs_scenario` harness run confirms `MoodConfig::current()` returns
   those rates in-process.
2. **`BASTION_AUTON2_MOOD_OVERRIDE` leftover** (hypothesized, checked): no
   `BASTION_*` env var was set in the launching shell; ruled out as best as
   retroactively checkable.
3. **RNG-lockstep drift shifting the despondency roll** (hypothesized,
   withdrawn): the despond roll's code is upstream of and unaffected by the
   fix's control-flow changes in the same per-colonist iteration; can't
   explain a categorical zero across 8 colonists over 40 minutes regardless.
4. **Grep/encoding artifact** (hypothesized, then measured false at the
   byte level): every relevant log line (`slept`, `need preempt`,
   `RestAt`, `BREAKDOWN`, etc.) uses an em-dash (U+2014). A calibrated
   byte-level check (Python `bytes.count()`, no locale/grep involved) was
   run against both Run B (canary `BREAKDOWN` scored 22, matching known
   truth exactly — method proven sound) and driver-12 (every pattern,
   including pure-ASCII fragments with no dash at all, scored genuinely
   zero). The zeros were real.
5. **Decay not running / rates not loaded / component wiring broken**
   (hypothesized, measured false): a new diagnostic
   (`BASTION_DECAY_JOIN_DIAG`, added to `bastion_jobs.rs` at the
   `decay_needs` call site) confirmed `decay_join_count=8` at every reading
   in a 3-minute live check, correct rates in the same emit, and the
   observed hunger/rest values at the end of that run matched the decay
   arithmetic to within 31.5 seconds — independently derived from both
   meters, traced to the measured pre-promotion delay exactly.
6. **Server tick rate falling behind wall clock under load**
   (hypothesized, contested, ultimately inconclusive for driver-12
   specifically): `dt` is fixed per tick, so sim-time is tick-count-driven,
   not wall-clock-driven — meaning load can only matter by changing how
   many ticks execute, not by diluting each tick's effect. driver-12 has
   **no tick-count diagnostic in its log** (added after that run), so this
   remains an arithmetic inference from wall-clock overage, not a direct
   measurement, for that specific run.

## The accelerated repro — constraint 5's decisive test

`BASTION_AUTON2_MOOD_OVERRIDE` scaled all three decay rates 10x (order
preserved: hunger still crosses first, still at half rest's time),
replaying script-09's dynamic in ~6 minutes instead of 40, with
`BASTION_DECAY_JOIN_DIAG`/`BASTION_NEED_SKIP_DIAG`/`BASTION_ARB_PERSONAL_
DIAG` on and `BASTION_REQUIRE_EXPLICIT_ASSETS=1` enforcing the asset root
(no ambient resolution possible).

**Result, scored against the outcome pre-registered before the run:**

| registered | measured |
|---|---|
| first sleep after the rest crossing, not before | first preempt ~178s vs predicted ~180s |
| zero sleeps before that | structurally guaranteed — no `RestAt` could exist earlier |
| repetition across colonists and over time, not one-then-silence | 11 completions, all 8 distinct colonists (52-59) |
| zero despondency (mood 0.6-0.25=0.35 > break_minor 0.25 once rest restores) | zero `BREAKDOWN`, confirmed |
| zero hunger successes (structurally impossible, no food) | zero, confirmed |
| a light/short run should NOT reproduce driver-12's null | it didn't — clean pass |

Skip-diag histogram for the full run: `no_food_found: 2703,
preempt_cooldown_active: 1744, no_need_below_interrupt: 1411,
already_on_need_job: 39, drive_not_personal: 6` — all ordinary reasons, no
late/unusual skip (`no_bed_found`, a struck-out cap, a missing component),
and a healthy total line count that rules out the need-check pass's
`is_loaded` collect filter silently dropping every colonist (that branch
emits nothing at all, even with the skip diag on — its absence as an
explanation here is itself informative).

**Six for six against the pre-registered bar.** Hunger crosses first,
dead-ends on no food, and instead of blocking the colonist for the rest of
the run, rest gets its own turn the moment it independently crosses — no
wedge, no cooldown block, repeatedly, across every colonist, live.

## The shipped-rate arm — run-15, the strongest evidence this row has (2026-08-09)

The 10x-accelerated repro above established the *mechanism* — order-
dependent fall-through — but explicitly claimed nothing about shipped-rate
timing. Run-15 (full detail: `RUN15-ISLOADED-FOLLOWUP.md`) closes that gap:
`script-09-milestone.txt` verbatim, 8 colonists, no food anywhere, full
72,300-tick / 2,410-sim-sec budget matching driver-12 exactly, current tip,
**natural (1x, non-accelerated) decay**.

- First `RestAt` job ~tick 52,036 (28.9 min) against script-09's own
  ~30 min prediction.
- **8/8 colonists slept** — every colonist, one full sleep cycle each,
  completions landing 29-34 min, inside the 40.2 min budget with buffer
  to spare.
- 21,511 `no_food_found` (hunger correctly, permanently dead-ending — no
  food exists in this scenario), 2,640 `preempt_cooldown_active`, zero
  food successes (structurally guaranteed).

This retires the scope caveat on constraint 5 entirely: the fix now has a
live demonstration at shipped rates, full duration, natural decay — not
just 10x-accelerated reachability. Two independent post-fix live runs
(this one and the 10x repro above) now show none of driver-12's failure
mode.

Run-15 was also built to test driver-12's own follow-on hypothesis (the
`is_loaded` filter dropping colonists under load) via the new `BASTION_
NEED_LOAD_FILTER_DIAG` A/B/C counters. Result: `dropped_by_is_loaded`
zero across all 4,897 samples, the full matched budget. But the arm's own
non-vacuity check (per Opus's charter: the loaded arm must independently
show the overrun driver-12 showed) came back negative — **zero `slow
system execution` warnings** (driver-12: 22, up to 625ms) and **~9.8s /
0.4% wall-vs-nominal overage** (driver-12: ~324s / ~11.8%) across the
active 72,510-tick window. The load precondition was never met, so this
arm is **void for the `is_loaded` hypothesis specifically** — a clean
non-result, not a refutation — while remaining a full, unqualified pass
for the fix itself. See `RUN15-ISLOADED-FOLLOWUP.md` for the complete
read and driver-12's resulting disposition.

## The deliberate-contention arm — `#63` resolved, plus a new finding (run-16)

Run-15's non-vacuity void motivated a follow-up: same script/budget/
footprint/tip, but with 8 sustained CPU-bound threads deliberately
saturating all 8 physical cores for the run's duration. Full detail:
`RUN16-CONTENTION-ARM.md`.

**Non-vacuity satisfied, two independent ways this time**: 5 `slow
system execution` warnings (up to 875ms) and a measured ~29.3%
wall-vs-nominal overage — both exceeding driver-12's own numbers (22
warnings/625ms, ~11.8%). `dropped_by_is_loaded` stayed zero across all
4,234 samples throughout. **Per the pre-registered table: the `is_loaded`
hypothesis is REFUTED with its precondition met** — a real negative
result, not a void one. `#63` closes on this specific mechanism.

The run also surfaced something unplanned: the driver's own "script
complete" claim and the server's authoritative tick counter diverged
substantially under contention (the client's `Wait(n)` loop is paced by
its own clock, not by polling confirmed server tick state) — the driver
believed it ran the full budget while the server had only reached 85.4%
of it. This reopens driver-12's disposition: rather than an unexplained
one-off, driver-12's null is now also explainable by this same mechanism
(the server simply never reaching the tick where hunger/rest cross their
interrupts), independent of the `is_loaded` filter entirely. See
`RUN16-CONTENTION-ARM.md` for the full account.

## Ruling (relayed from Fable/Opus)

Constraint 5 is satisfied — its purpose was reachability (does the fixed
path get entered live under natural decay), not shipped-rate timing. The
mechanism is order-dependent, not rate-dependent, which is why the 10x
scope limit (registered before running) doesn't weaken the finding.

driver-12's own null result is **structurally cleared of implicating this
fix**: the region of `bastion_jobs.rs` upstream of the despondency roll
(the `is_loaded` collect filter, the `missing_component` re-fetch) is
byte-identical between the pre-fix tip (`3918823eb6`) and the post-fix tip
(`2dbc96127e`) at the relevant line ranges — a change inside the candidate
loop cannot alter a gate that runs before it in the same tick. Whatever
silenced driver-12 is upstream of everything this fix touches.

**Leading structural hypothesis for driver-12, filed as its own row, not
this one**: sustained load degrading `rtsim::tick` (22 "slow system
execution" warnings observed in driver-12's own log, zero in the light
3-minute and 10x runs) causing the need-check pass's `is_loaded` filter to
drop colonists from `need_order` on a live, per-tick read of rtsim state —
a branch that emits **no diagnostic output at all**, even with
`BASTION_NEED_SKIP_DIAG` on, making it invisible to every instrument built
during this investigation. Proposed falsifier: paired counters bracketing
the filter (population before vs. after `is_loaded`, not conflated with
the separate missing-component question) showing a gap under load and none
unloaded.

## Standing changes adopted this session

- `BASTION_DECAY_JOIN_DIAG`: new, permanent, gated diagnostic at the
  `decay_needs` call site — reports the decay join's entity count and the
  effective rates together, every 300 ticks when enabled.
- Every live run should set `VELOREN_ASSETS` explicitly and, where
  certification matters, `BASTION_REQUIRE_EXPLICIT_ASSETS=1` — this
  worktree sits inside a root checkout carrying pre-retune rates, and
  ambient asset resolution can silently pick the wrong tree.
- Non-ASCII log content (em-dashes throughout this codebase's `info!`
  literals) is real and byte-present; a reading method should be
  calibrated against a known-positive control before a zero is trusted,
  not because grep failed here (it didn't, on inspection) but because nine
  runs into a single-night arc is exactly when that discipline pays for
  itself.
