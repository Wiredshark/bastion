# preempt_scenario's `preempted_rested` red: cooldown-on-attempt, not on outcome

AUTON-2 Step 1 prep. Four hypotheses raised and closed in sequence, each
by measurement or direct code read, not argument — the row's own working
method this whole thread.

**Final answer: the need-check pass works correctly. The defect is in
`preempt_cooldown`, which is set at attempt-INITIATION for all three of
its writers, not at successful completion.** A colonist whose rest/eat
attempt fails at travel/arrival is rate-limited exactly as if it had
succeeded — one try per `PREEMPT_COOLDOWN_SECS` (60s), regardless of
outcome. Unification (AUTON-2's main proposal) does not fix this: the
failure is downstream of arbitration, at travel/arrival, and no arbiter
change touches that.

## The four hypotheses, in the order they were raised and closed

1. **"Design deferral"** (original label). Not investigated directly —
   superseded before this thread started.
2. **"Tuning artifact — interrupt band unreachable within any scenario
   window"** (AUTON-2 spec §4's original claim). REFUTED: `preempt_scenario`
   force-sets `rest = 0.15` via `bastion_set_needs`, directly below the
   confirmed-real interrupt threshold (0.2, unstaggered — this colonist's
   Craft=0, Tradition=0, temperament=(false,false), so
   `stagger_interrupt` returns exactly the base value). The band is
   reached by construction, not by decay. No timing issue exists for this
   scenario.
3. **"Mechanism failure — the need-check pass never engages for an
   actively-working colonist"** (this thread's own first reading, from a
   trace showing rest monotonically decreasing across a 360-tick window
   with zero rise). REFUTED by a second run with skip-reason
   instrumentation: "need preempt — rest below interrupt" fired 6 times,
   with real bed targets, and one "slept — rest restored" event
   completed. The pass does engage.
4. **"Bed gate — no free bed exists"**. REFUTED by the same skip-reason
   data already in hand: `no_bed_found` = 0 across all 664 skip events in
   that run. The bed lookup never failed.
5. **"GUARD 6 blocks entry into the self-job downstream of
   `preempt_pending`"**. REFUTED by direct read of the drain
   (`bastion_jobs.rs:12877-12900`): job creation and `ActiveJob` insertion
   are unconditional once a `PendingNeed` is pushed — no exempt-occupancy
   check gates entry there.
6. **★ Cooldown-on-attempt (the actual answer)**. All three
   `preempt_cooldown.insert(*uid, time.0 + PREEMPT_COOLDOWN_SECS)` sites
   (`bastion_jobs.rs:9376-9377` despond/break, `9526-9527` eat,
   `9591-9592` rest) fire the moment the colonist DECIDES to attempt —
   before the job is created, before travel, before any outcome exists.
   Each site also increments a counter literally named
   `preempt_attempts`, confirming this is attempt-scoped by design, not
   outcome-scoped by accident.

## What this explains, without further instrumentation

The observed trace (6 rest-preempt initiations over ~330 sim-seconds, two
distinct bed targets — 3 attempts each — one confirmed sleep) now reads
cleanly: attempt bed 1 → cooldown arms for 60s → travel fails (the same
claim-release churn class documented elsewhere in this campaign) → 60s
later, cooldown expires, rest is still below interrupt, attempt bed 2 →
repeat. The mechanism isn't broken; it's rate-limited identically whether
an attempt succeeds or fails, so a colonist that cannot reach its bed
gets slowed to one try per minute rather than either succeeding quickly
or failing visibly.

## The law, ninth costume

An attempt and a success render identically to the cooldown map — it
cannot distinguish "I slept" from "I never got there," so it penalizes
both the same way. The guard is real and defensible (without it, an
unreachable target means a retry every tick) — but it measures the wrong
event. Anti-thrash wants to limit *failed* attempts; this limits *all*
attempts.

## Consequence for AUTON-2

Unification (making the arbiter directly own needs alongside Work/Flee)
does not fix this — the failure is at travel/arrival, downstream of
arbitration entirely, and no arbiter change touches it. The spec needs a
new, small component: the cooldown must distinguish outcomes (set on
completion or genuine abandonment, not on intent), or stay attempt-scoped
but make the failure visible (an unreachable bed reported rather than
silently rate-limited) — same shape as the corner-cell row's own
blocked-designation messaging precedent.

## The displaced-colonists motif (fourth appearance, per Fable/Opus — flagged, not built on here)

Bed's own walk-test, the 52/54 vantage split, farm colonists found 9
cells below their plot in a natural gap, and now a colonist that cannot
reach its bed. Four independent rows, one recurring shape: colonists
failing to arrive somewhere they were sent. Worth naming as a risk before
AUTON-2 ships a feature whose entire job is sending colonists to new
destinations — not investigated further here; this is a pointer for
whoever picks up that thread next, not a claim about its cause.

## Instrumentation left in place (all env-gated, zero cost when unset)

- `BASTION_NEED_SKIP_DIAG` — one log line per `continue` in the
  need-check pass (6 sites), keyed by reason.
- `BASTION_PREEMPT_DIAG` — rest-value + open-mine-job trace in
  `preempt_scenario`, plus the colonist's real values/temperament at the
  point needs are force-set.

Neither was needed for the final answer (which came from a code read of
the three cooldown writers), but both were the instruments that closed
the three hypotheses ahead of it, and are cheap to keep.
