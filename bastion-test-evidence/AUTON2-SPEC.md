# AUTON-2 SPEC — needs unification, day-aligned (DECISIONS #64)

**Blob for every line cite: `90dc4d11f0`** (`bastion/wip-batch-verify`, carries
Row B′). Line numbers move — **re-locate by symbol before editing.**

> ## ★ THE BRIEF WAS WRONG AND THE READS CORRECTED IT
> The row was briefed as *"needs become real drives — extend `Drive` with Rest
> and Eat."* **The needs machinery already exists and runs every tick.**
> Building to the brief would have produced a duplicate of a live system — the
> `stuck_strikes` error at feature scale. **This row is UNIFICATION, which is
> what `Drive`'s own doc comment already says AUTON-2 is for.**

## §1 — WHAT ALREADY EXISTS (READ — do not rebuild)

| piece | site | state |
|---|---|---|
| `Needs { hunger, rest, recreation }` | `common/src/comp/bastion.rs:131` | exists |
| per-tick decay `decay_needs(dt)` | called at `bastion_jobs.rs:5691` | **running** |
| restore on sleep / on food | `11821` / `11915` | wired |
| interrupt thresholds, personality-staggered | `9397-9415` | wired |
| **candidate scoring** — push both needs, sort by severity, take worst | `9397-9420` | **already arbitrating** |
| preempt cooldown + no-re-preempt guard | `9425-9440` | wired |
| self-jobs `RestAt` / `EatFrom` | `insert_rest_job` / `insert_eat_job` | exist |

**The gap is not the needs. It is WHO arbitrates them.** `Drive`
(`comp/bastion.rs:160`) says so itself:

> *"Self-jobs (RestAt/EatFrom/Despond) are deliberately NOT a variant — they are
> an exempt occupancy the arbiter steps around (GUARD 6 …); **the full
> unification is AUTON-2's job**."*

## §2 — THE ARBITER, READ (`bastion_jobs.rs:8884-8955`, consts `1689-91`)

```
URGENCY_FLEE = 1.0     URGENCY_WORK = 0.5     URGENCY_IDLE = 0.1
scores = modulated_urgencies((work_sig ? WORK : 0, flee_sig ? FLEE : 0, IDLE))
pick max  ->  commit for ARB_COMMIT_SECS      (anti-thrash)
Flee preempts the commitment per-tick (802); same-tier does not
```

**So the unification is concrete:** needs stop being a *separate selector* and
become **urgencies in this scale**, entering the same `max` under the same
commitment window.

**Ranking, with its reason:** a severe need must outrank `Work` (0.5) and must
**never** outrank `Flee` (1.0) — *a starving colonist still runs from a storm.*
So need-urgency occupies the band **(0.5, 1.0)**, scaled by severity:

```
need_urgency = URGENCY_WORK + (URGENCY_FLEE - URGENCY_WORK) * severity
severity     = shortfall(value, interrupt) / interrupt        // 0..1, clamped
```

At the interrupt threshold `severity → 0`: ties Work and loses to it — **correct,
a just-crossed need should not abandon work mid-swing.** At `value → 0`,
`severity → 1`: approaches Flee without ever reaching it.

★ **The existing worst-need selection (9397-9420) is PRESERVED, not replaced.**
It becomes the *which need* question, asked after the arbiter has decided *a need
wins*. Personality-staggered thresholds and the anti-thrash guards stay exactly
as they are. **Unification changes who arbitrates, not how needs score.**

★ **GUARD 6 retires.** Self-jobs stop being an exempt occupancy the arbiter steps
around, and become the *execution* of a Drive the arbiter chose. **Read every
GUARD-6 site before removing any** (`805-814`, `1375-1384`, plus the exempt
checks in the work tick) — **a guard removed at one site and left at another is
worse than either state.**

## §3 — HYSTERESIS (ruled; the design's own oscillation guard)

Enter and exit thresholds **differ**. Two mechanisms already exist and must not be
duplicated: **`ARB_COMMIT_SECS`** (the arbiter's commitment window) and
**`comfort + SLEEP_MARGIN`** (`11827`, the sleep exit). **Prefer wiring those two
to inventing a third.** State in the build which governs Rest/Eat exit, and why.

## §4 — THE TIMESCALE FINDING, AND THE RE-TUNE

```
day_cycle_coefficient = 1440 / day_length = 48   ->  1 game day = 1800 sim-sec = 30 min

rest    2667s to interrupt = 1.48 days  ->  0.67x/day   (intent ~1)   ~50% too slow
hunger  2000s to interrupt = 1.11 days  ->  0.90x/day   (intent ~2)   ~2.2x too slow
```

**Targets** (day-aligned, per the ruled principle): rest ≈ 1×/day →
`decay_per_sec ≈ 0.00044`; hunger ≈ 2×/day → `≈ 0.00089`. *Arithmetic only —
verify against the day length in force before applying.*

> **★ "COMPLETE BUT INVISIBLE → COMPLETE AND FELT."** Colonists live on the clock
> the world runs on. That is the row's one-line justification.

**And it explains three things at once:**

- **the playthrough's "needs inert"** — 4.5 min is **14%** of the shortest cycle.
  *The prediction was mis-specified; the game was fine.*
- **both expected-reds** — a b5 window (~167 sim-sec) is **8%** of it.
  `preempted_rested` and b73's `ate` are **TUNING ARTIFACTS, not design
  deferrals**: unreachable at shipped constants, so they could never have gone
  green however correct the code was. **Re-label now, not with the row.**
- **why nobody noticed** — no window we run is long enough to see it.

## §5 — UNLOADED CHUNKS: (b) CATCH-UP-ON-RELOAD (ruled)

Per-colonist `last_decay_tick: u64`; on reload apply **one** deterministic decay
for elapsed **sim ticks**. **No wall clock. No new per-tick work.**
(a) *freeze* rejected — inheriting a player-attention-dependent world by silence.
(c) *background tick* rejected — new steady-state cost against the observability
budget. **Validate in build:** if the tick source or save semantics cannot
support it, **that is a report** and the ruling revisits.

## §6 — BUILD ORDER (ruled; each step its own measured delta)

1. **Planted-case infrastructure** — a test-only `common.bastion_mood` through
   the existing hot-reload. `MoodConfig::current()` already falls back to the
   compiled default on a missing asset, so a fixture supplies its own tuning with
   **zero shipped-behaviour change.** Gives every later step its instrument.
2. **Unification at CURRENT constants** — the behaviour change, isolated.
3. **The re-tune, ALONE** — one asset, arithmetic-verified before any run.

**Two behaviour changes in one window are confounded by construction.**

## §7 — ACCEPTANCE + BUDGET

- **Registered prediction:** `preempted_rested` and `ate` **flip green** when
  step 1's planted cases land. They are the acceptance tests and they already
  exist.
- **Planted-failure test:** disable the need→urgency mapping; the planted case
  must go **RED**. *A test that cannot fail is not one.*
- **Observability budget:** drive transitions are **per-event, not per-tick
  reads** — the observer-effect law, measured. State cells × reads × cadence.
- **FR15, full strength on step 3.** Direction stated up front: **throughput DOWN
  by roughly the fed/rested fraction.**

> ★ **"The economy shifted" is the POINT here, not a failure** — the exact
> opposite of Row A/B's bar, and **that reflex does not transfer.** Read the A/B
> against **intent**: colonists rest when depleted — *not never, not constantly.*
