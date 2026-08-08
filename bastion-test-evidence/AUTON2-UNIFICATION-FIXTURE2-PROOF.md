# AUTON-2 unification, Fixture 2 (DESPOND-RESUME DETERMINISM): the before-state, and why it stops here

Implements the required before-state check from `AUTON2-ACCEPTANCE-FIXTURES.md`
(a regression fixture, per Opus's explicit distinction from Fixture 1 — must
be GREEN before the GUARD-6 build, not RED). This document covers what was
built, two real construction bugs found along the way, and a decisive
finding that closes the investigation per Opus's own ruling rather than
continuing to chase it.

## Two real construction bugs, found and fixed

1. **The rest/hunger tie-break trap.** First pass zeroed rest, hunger, and
   recreation identically to force a breakdown. With `rest == hunger`
   exactly, the need-ranking candidate sort (`bastion_jobs.rs` ~9484-9503,
   a stable sort with rest pushed before hunger) resolves the tie toward
   rest — and this fixture has no bed, so the colonist got stuck forever
   on `NEED-SKIP-DIAG reason=no_bed_found`, never falling through to try
   hunger at all (`ate` stayed `false`). Diagnosed directly via
   `BASTION_NEED_SKIP_DIAG`, not guessed. Fixed by keeping rest safely
   above any staggered interrupt ceiling (`base * 1.5 = 0.3` max, per
   `stagger_interrupt`'s own doc) so it's never a candidate, while its
   own comfort-band shortfall still contributes enough to mood to cross
   `break_minor`.
2. **A refuted hypothesis, tested with numbers, not assumed.** After
   `original_until` was measured (`63.43`), `resumed_until` stayed `null`.
   The first hypothesis — the 60-second despond window expired during
   travel-to-food before the eat could complete — was tested directly:
   `sim_time_at_ate = 17.77`, well under `63.43`, more than 45 seconds of
   margin. That hypothesis is dead.

## The instrumentation problem: catching a transient the live state can't hold

`bastion_bed_slot`-style live polling of the Despond job (`bastion_despond_
until`, reading `board.jobs` between ticks) could not catch the ORIGINAL
despond job at all, even at continuous 1-tick granularity from the moment
the breakdown roll fires — `BASTION_DESPOND_DEBUG` traced `despond_jobs()`
staying `0` for 40 straight ticks immediately after `attempts` incremented.
Fixed by not racing the read: `until` is deterministic (`SimSecs::after`,
`common/src/bastion.rs:790` — plain `now.0 + self.0`, no rounding), so
`bastion_sim_time()` read the tick the roll is observed plus the shipped
`despond_secs` (60.0; this fixture applies no override) computes the exact
same value the engine's own roll computed, without needing to catch the
live job at all.

## The decisive read, and what it settles

`resumed_until` still could not be captured after the analytical fix for
`original_until`. Trace (`has_active_job` via `bastion_colonist_states_
full`, `despond_jobs()`, `sim_time`, polled every tick starting right
after `ate`): the colonist shows `has_active_job = false` for only ~5
ticks (0.17 sim-sec) after eating, then flips to `true` and stays there —
consistent with the mine-strip fixture backdrop getting picked up by
normal Work selection almost immediately, a plausible race against the
despond re-issue's own `!active_jobs.contains(entity)` precondition. But
`despond_jobs()` stayed `0` through that window too, including the ~5
ticks the colonist was genuinely free — raising a sharper question than a
timing race: did the carve-out (the actual `despond_resume` insert) even
run at all?

**Per Opus's ruling ("one read, then move on regardless"), added
`Server::bastion_despond_resume_pending(uid)` — a direct read of the
`despond_resume` side table itself (`JobBoard::probe_despond_resume`, a
new read-only getter), distinct from `bastion_despond_until` which reads
the live job. Checked right after `ate`:**

```
ate=true sim_time_at_ate=17.77 original_until=63.43
despond_resume_after_eat=None
```

**`None`. The carve-out never fired.** Per the pre-stated decision table:
an empty table means `no_reroll_on_resume = true` (measured earlier as
`attempts_after_break == attempts_after_resume`) was **vacuous, not a real
confirmation** — the assertion held because its precondition (a resume
actually happening) never occurred, the same shape as b73's own `paused`
carrying no information when `ate` is false. This is a fixture LIMIT, not
a fix — the resume path, as this fixture is currently constructed, is not
exercised.

## Why this stops here (Opus's own correction)

Fixture 2 was specced to protect a guarantee across the unification
change: *"an active condition is not a new breakdown."* But per the
design's own §5b, `despond_resume` and the whole destroy-and-recreate
lifecycle it exists to bridge are scheduled for **deletion** under family
1 — once the Despond job **persists** across a need-preempt interruption
(rather than being destroyed and recreated from a side table), the
deadline persists trivially, and no re-roll can occur because nothing
re-creates anything. **The guarantee becomes structurally true by
construction under the change this fixture was meant to gate before.**

Fixture 2 was specced as "prove the current mechanism's behaviour"; it
should have been "confirm the guarantee's intent carries forward." That
scoping correction is Opus's, made explicitly rather than continuing to
harden an observation of code about to be deleted. **The guarantee moves
to being an acceptance item on the unification build itself**: after
family 1 lands, assert that a Despond interrupted by eat/sleep resumes
with the SAME deadline and NO roll — under persistence this should be
trivially demonstrable, which is itself evidence the design decision
(persist, don't destroy-and-recreate) was correct.

## Status

Not GREEN, not RED — **inconclusive by construction**, and correctly so:
the mechanism this fixture targets is scheduled for deletion, and the
property it protects becomes free under the replacement. Two real
construction bugs found and fixed along the way (tie-break trap,
travel-expiry hypothesis refuted with measured numbers) — those fixes and
the new hooks (`bastion_sim_time`, `bastion_despond_until`,
`bastion_despond_resume_pending`) are kept; the fixture itself is not
pursued further per explicit direction. Proceeding to Fixture 1's settle
invariant running in every scenario, then the GUARD-6 build.
