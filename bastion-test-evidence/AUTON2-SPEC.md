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

> ## ⚠ §1 AMENDED AFTER MEASUREMENT — "ALREADY ARBITRATING" WAS READ, NOT VERIFIED
> The table above is **read from code**. 5b then **measured** it, and the precise
> statement is narrower than what I first wrote:
>
> **The arbitration WORKS — it selects, it finds a bed, it issues a preempt.**
> *(Measured: 6 initiations, 2 bed targets, one completed sleep.)*
> **The FOLLOW-THROUGH is where it dies, and TWO GUARDS HIDE THAT:** the preempt
> cooldown treats *initiation* as success (§4b), and the churn path releases the
> claim silently.
>
> **Unification touches neither.** *"The code exists"* and *"the code runs"* are
> different claims — and I made the first while asserting the second.

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
- **b73's `ate`** — a b5 window (~167 sim-sec) is **8%** of the shortest cycle,
  so this one plausibly **is** a tuning artifact. *Untraced — do not re-label
  until someone measures it, per the mistake below.*
- ~~**`preempted_rested` is a tuning artifact**~~ — ★ **STRUCK. WRONG.**
  `preempt_scenario` **force-sets** `rest = 0.15` via `bastion_set_needs`,
  **below the 0.2 interrupt, bypassing decay entirely.** The band is reached by
  construction, so the timescale finding does not apply to it at all.
  **My re-label would have buried a real defect under a bookkeeping
  correction.** See §4b — the actual cause is code-grounded and different.
- **why nobody noticed** — no window we run is long enough to see it.

## §4b — ★ THE PREEMPT DEFECT: COOLDOWN RENEWED BY INITIATION, NOT COMPLETION

**Five stories were proposed for `preempted_rested`. Four are dead. This is the
survivor, and it is the only one grounded in code rather than inference:**

| # | story | fate |
|---|---|---|
| 1 | "design deferral" | original label, never checked |
| 2 | "tuning artifact" | **mine — refuted:** needs are force-set below the band |
| 3 | "mechanism never fires" | 5b's first trace — **refuted by their own second run** |
| 4 | "sampling artifact: 12 s window vs 60 s period" | **mine — refuted:** the run was ~330 s. *An infinite window would not help either.* |
| 5 | **cooldown renewed by INITIATION** | ★ **survives — READ, three sites** |

**READ — all three writers, `time.0 + PREEMPT_COOLDOWN_SECS` (=60), each followed
immediately by `preempt_attempts += 1`:**

    9377  breakdown/despond roll
    9527  EAT   — fires when food is FOUND, not when eating completes
    9592  REST  — fires when `bed_pos` is FOUND, before the colonist walks anywhere

> **AN ATTEMPT AND A SUCCESS RENDER IDENTICALLY TO THE COOLDOWN.** It cannot
> distinguish *"I slept"* from *"I never got there,"* so it penalises both — and
> a colonist whose bed is unreachable is rate-limited to **one try per 60 s by
> the guard meant to stop it thrashing.** *(Ninth costume of the campaign's
> central law.)*

Measured: **6 initiations over ~330 s ≈ one per 55–60 s**, bed A abandoned after
3, bed B after 3, **one** completed sleep.

### ★★★★★ §4c — CHASED. **NOTHING INTERRUPTS THE SLEEP. THE SLEEP NEVER STARTS.**

**Read `5f8cdf1392`, five sites, re-located by symbol. The question below had no
referent** — *"what interrupts an in-progress sleep"* presupposed the sleep began.
**It does not.** What fires is a **TRAVEL timeout**, and it is **kind-agnostic**.

| # | site | effect on a `RestAt` |
|---|---|---|
| 1 | `insert_rest_job` **5337** | **PRE-CLAIMED**; bed reserved at **creation** |
| 2 | `auton_travel_ok` **11250** | self-jobs travel **UNGATED** → watchdog **NOT frozen** |
| 3 | travel timeout **~11490** | `claimed_by = None` + release — ★ **no kind check** |
| 4 | to_release drain **12816** | clears bed occupancy **by uid** |
| 5 | orphan sweep **8669** | `RestAt` **is** in the filter → job **removed** |

**The two sites that look like sleep-interrupts are not:** `auton_work_ok`
(**12190**) is *"a SUSPEND — the claim stays held… nothing releases the job"* and
self-jobs bypass it unconditionally; job-moot (**12452**) has `RestAt => false`.

★ **Two of my own theories, killed by my own follow-through:** *"the orphaned
RestAt is unclaimable forever"* (refuted — sweep **8669** covers it) and *"the bed
leaks, because `remove_job`'s `slot.occupant == j.claimed_by` guard cannot match
after step 3 nulls `claimed_by`"* (refuted — the drain clears by **uid**, and runs
first). **The second would have elegantly explained bed A → bed B. It was wrong.**

> ## ★ WHY NOBODY NOTICED: EVERY SINGLE SITE IS CORRECT
> Nothing leaks, nothing errors, nothing logs. The bed frees, the job sweeps, the
> colonist is healthy. **The system cleans up after itself perfectly**, and the
> only visible consequence is *a colonist that never sleeps.*
>
> **A COMPLETE, CORRECT CLEANUP PATH IS INDISTINGUISHABLE FROM A COMPLETED TASK.**
> *Tenth costume of the law — and the first where **no single site is wrong.***

**It predicts the measurement:** 6 initiations / ~330 s ≈ **one per cooldown**,
because the cooldown is the only rate limit on the retry loop.

### ★★★ THEREFORE THE COOLDOWN IS THE WRONG OBJECT TO FIX

> **The failure is a property of the (colonist, bed) PAIR. The cooldown AND the
> release both record it on the COLONIST.** So the retry re-picks the same
> unreachable bed and re-pays the same timeout — **rate-limiting the colonist was
> always treating the symptom.**

Cooldown-on-completion is leak-safe but yields **retry-every-slot against a bed
demonstrably unreachable**: the cause does not dissolve the thrash, it **confirms**
it. ★ **Expected landing: Row B′'s `benched_until_tick` shape, on the BED** — a
proven, measured mechanism rather than a new one. *Held pending 5b's trace.*

**Still open, and empirical:** (1) does the timeout **observably** fire —
`BASTION_LEGC_DIAG` (**11487**) already prints `stuck_time`/`sdist`/`drive`, so
likely **no new instrumentation**; (2) **why** the travel stalls — the fifth
appearance of *displaced colonists failing to arrive*; (3) every one of the ~6
terminations must land in a **named row above** — ★ *one that lands in none
refutes this read.*

### ★★ (SUPERSEDED FRAMING — kept because the ORDER it forced was right)

**Was open and unchased: WHAT INTERRUPTS AN IN-PROGRESS SLEEP?**

> Change cooldown-on-initiation → cooldown-on-completion **without knowing why
> sleeps abort**, and *"locked out 60 s"* becomes **"retries every tick."** The
> cooldown is currently **the only thing masking an unknown interrupt.** Remove
> the mask without removing the cause and you get precisely the thrash it exists
> to prevent, now unbounded.

**So: find the interrupt cause FIRST, then fix the cooldown semantic.** The
reverse order is **strictly worse than doing nothing** — and that makes the
*interrupt* the row, with the cooldown a symptom-mask over it.

### ★ NAMED RISK: DISPLACED COLONISTS FAILING TO ARRIVE (fourth appearance)

bed walk-test · seeds 52/54's vantage split · farm colonists 9 blocks below ·
**a colonist that cannot reach its bed.** Four rows, one shape. **AUTON-2 sends
colonists to MORE destinations, not fewer** — this is a standing risk to a
feature whose whole job is making colonists walk somewhere, and it is named here
*before* the build rather than discovered during it.

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
   **★ UNBLOCKED — proceeds now, independent of §4b.**
2. **★ NEW — THE INTERRUPT CAUSE (§4b), BEFORE UNIFICATION.** *What aborts an
   in-progress sleep?* Then, and only then, the cooldown semantic.
   **Why it gates Step 3:** unification's acceptance is *"the planted case
   rests."* **A case that cannot complete a rest cannot demonstrate
   unification** — the gate would be measuring the wrong failure.
3. **Unification at CURRENT constants** — the behaviour change, isolated.
4. **The re-tune, ALONE** — one asset, arithmetic-verified before any run.

**Two behaviour changes in one window are confounded by construction.**

> ★ **The order is forced, not preferred.** Step 2 before 3 because a corrupted
> gate is worse than a late one; the cause before the cooldown fix because
> removing the mask without the cause is *strictly worse than doing nothing*.

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
