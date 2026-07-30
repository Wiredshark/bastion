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
