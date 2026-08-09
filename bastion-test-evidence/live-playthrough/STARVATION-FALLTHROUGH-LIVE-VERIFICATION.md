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
