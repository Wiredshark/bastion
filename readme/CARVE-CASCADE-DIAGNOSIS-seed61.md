# CARVE CASCADE (mechanism 1) — diagnosis, seed 61

**Read-only diagnosis. No world writes, no behaviour change.** Written as
partner work alongside 5b's mechanism-2 (travel-arrival friction) thread.
Every claim below is a code read at `af6ff047ee`; the two marked PREDICTION
are falsifiable by 5b's in-flight offline probe on 51/54/55/61/71 and
should be checked before anyone builds a fix.

## The one-sentence diagnosis

**Every bound on the access-planning loop is measured RELATIVE TO THE
COLONIST'S CURRENT POSITION OR MOST RECENT OUTCOME; the quantity that
actually diverges is displacement from where the episode STARTED. So the
cascade satisfies all three of its bounds at every step while walking
arbitrarily far.**

That is why it does not self-terminate, and why only an external B24
failsafe rescue ends it.

## The three bounds, and how each is defeated

**1. `EMERGENCY_REENGAGE_BOUND = 5`** (`bastion_jobs.rs:663`) — consecutive
fruitless route outcomes per member, the intended episode cap.

Defeated because it is **progress-RESET**, not cumulative. At
`frontier-complete` (~13007–13013) the code's own comment reads *"Real
progress: both per-episode bounds reset"* and drops
`emergency_reengage_aborts` for the member. Every carve frontier that
COMPLETES refills the budget — even though completing it did not free the
colonist, and a new plan follows immediately. The counter cannot reach 5
while the cascade keeps completing frontiers.

*The cascade's local success is what pays for its next iteration.*

**2. Cell-level disjointness** (the M2 PLANNER-FIX, ~870–890) — a candidate
plan is rejected if its cells intersect `unavailable_cells` (other jobs'
cells + live emergency route cells).

Defeated because **disjointness is not a bound on COUNT when the target
recedes.** The observed march (20 → 50 units out, 17–20 z-levels deeper)
means each new plan is trivially disjoint from every prior plan, so this
gate never fires. Note this gate is not wrong: it replaced a colony-GLOBAL
one-plan-at-a-time gate that caused real request-starvation (seed-22: one
emission then 71 consecutive swallows). The fix was correct for that bug and
simply does not constrain this one — the global bound it removed was the
only thing incidentally bounding COUNT.

**3. `EGRESS_BUBBLE_R = 8`** (`bastion_jobs.rs:11989`) — the humanitarian
permission bubble each emergency plan may carve within.

Defeated because the bubble is built from **`from`, the colonist's CURRENT
position** (~12575–12586), with z from −2 to +64. As he follows each carve
deeper, the next plan's permission window MOVES WITH HIM. The bubble bounds
one plan's extent; it does not bound the walk. **This is the actual
generator of the outward march** — not a target-selection bug, but a
permission window with no memory of its origin.

## Why this is B56's family but not textually B56

`B56` was a **clock**-driven re-test of a **static** set, with no progress
at all: unconditional amnesty cleared unreachable flags forever, so the same
cells churned. This is a **progress**-driven re-arm over an **expanding**
set. Both are unbounded; the generators are opposites.

**The trap rhyme matters more than the family resemblance.** B56's first fix
was per-CELL strike caps, and it was *byte-identical* pre/post because the
burn was colonist-bounded — a cap applied at a finer granularity than the
divergence. Bound 1 here is the same error one level up: a per-EPISODE cap
that cannot bind because the episode keeps being declared complete. **Any
proposed fix must be checked against the quantity that actually grows, or it
will be silently numerically identical again.**

## What a fix must bound (mechanism-level, never the failure rate)

The three existing bounds are all *local*. What is missing is an
*origin-relative* invariant. Candidates, in the shape of Fable's stated
acceptance bar:

- cumulative carve **distance and depth from the episode's origin** (not
  from current position);
- **plans emitted per trapped episode**, where "episode" survives
  frontier-complete rather than being reset by it;
- **monotonic escalation refused**: a new target may not be strictly further
  and deeper than the last without an independent reason;
- **demonstrated termination**: the loop must end on its own, with the B24
  rescue as a backstop that should be observed NOT firing.

## Two predictions 5b's probe can falsify for free

- **PREDICTION A**: on seed 61 the escalating plans each end in
  `frontier-complete`.
- **PREDICTION B**: `emergency_reengage_aborts` for the cascading member
  never exceeds 1–2 before being cleared.

If both hold, the diagnosis is confirmed and the two bounds to re-site are
known. **If B fails — the counter reaches 5 and the cascade continues anyway
— this diagnosis is wrong** and the generator is elsewhere.

## Where mechanism 1 and mechanism 2 may meet

If travel-arrival friction is why a completed frontier does not actually
deliver the colonist to a usable exit, then friction is what makes each
frontier "complete but useless" — which is precisely what refills bound 1.
In that case the cascade is not an independent defect but the *amplifier* of
mechanism 2, and fixing friction alone could collapse both. Worth checking
jointly rather than either of us assuming our own mechanism is primary.

---

## Probe status, and a discipline the probe does not by itself earn

**`2e62ec811f` (the A/B probe) is COMPILE-VERIFIED, NOT RUN-VERIFIED as
of writing.** Hooks are in the right places by code reading and
behaviour-neutrality is grep-proven (every probe field appears only in
write positions in sim code; the sole reads are in the accessor). Runtime
emission is unverified. **Nobody should read a zero out of it until it
has been shown capable of a nonzero.**

That distinction is not pedantry. 5b hit a build-integrity failure the
same day — a "successful" release build with a matching build stamp and a
fresh exe timestamp silently reused a stale `bastion-server` compile, and
it was caught only because 5b verifies field-presence on every new field
before trusting output.

**The generalisable form, which cost two people time today on two
different pieces of work:**

> A stale binary and an uninstrumented binary produce the SAME output —
> silence — and both read as "no problem found."

A counter that compiles and never increments is indistinguishable from a
counter reporting genuine health. `present: true` in the harness JSON
separates "the counters say zero" from "this binary has no counters", but
it only helps if someone has demonstrated the counters can be nonzero at
least once. Field-presence plus a demonstrated nonzero is the pair;
either alone is a false all-clear, which is the same shape as this
diagnosis's own ceiling-versus-resets trap one level up.

**So the order of operations for reading A/B results is:** confirm
`b5_cascade_probe.present`, confirm at least one counter nonzero
somewhere, and only then treat a zero elsewhere as evidence.

---

## Pre-registered reading of the A/B run (written BEFORE the data)

Recorded before the probe has produced a single number, so the
interpretation cannot be fitted to the result afterwards. Seed 61,
release build with `RUSTC_WRAPPER=""` and `cargo clean -p` eviction.

**Gate 0 — is the measurement admissible at all?**
`b5_cascade_probe.present` must be `true` AND at least one counter
nonzero *somewhere*. If the object is absent: stale/uninstrumented
binary, suspect the build before the hypothesis. If present but every
counter is zero including `frontier_completes`: **the probe did not
emit**, which is UNPROVEN, not evidence of health — the same false
all-clear in a third costume. Neither case licenses any claim about the
cascade.

**Then, and only then:**

| Observation | Verdict |
|---|---|
| `abort_ceiling_max` low (≤2) AND `abort_resets_max` HIGH | **Diagnosis CONFIRMED.** The bound is being refilled, not respected; `frontier-complete` is the refill site. Resets answer "how fast". |
| `abort_ceiling_max` low AND `abort_resets_max` 0 | **Diagnosis WRONG on bound 1.** The counter never had a nonzero value to clear, so the progress-reset story is not the generator. Look elsewhere — probably at what admits plan N+1. |
| `abort_ceiling_max` ≥5 (bound actually exceeded) and the cascade continued | **Diagnosis WRONG.** The bound bound and something else overrode it; my prediction B fails on its own terms. |
| `access_emissions_max` large while `frontier_completes` ≈ 0 | Plans are being minted without frontiers completing — a DIFFERENT generator from the one described here, and bound 1 is irrelevant. |

**On magnitude, per the orchestrator's question:** a reset count of 2-3
is a bound working roughly as intended under a hard case. A reset count
in the tens is a loop. The distinction between "exceeded by one" and
"exceeded by hundreds" is the difference between a threshold that needs
tuning and a bound that does not bind at all — only the second justifies
touching shared carve machinery, and only on more than one seed.

**Standing caveat that survives any result:** n=1. Seed 61 is one seed in
72. Nothing here justifies surgery on shared access/carve/dormancy code
until the entry condition (fix mechanism 2, re-measure 61) has been run.
