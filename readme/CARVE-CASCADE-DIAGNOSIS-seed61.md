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

---

## Control-matching preconditions (learned the expensive way — read before running the pairs)

The paired comparison is `61(16 timeouts, FAIL)` vs `148(16, pass)` and
`146(21, FAIL)` vs `52(21, pass)` — matched on friction so the
discriminator can be something categorical rather than quantitative.
**Two ways that match can be void, both found before running rather than
after, and both invisible in the numbers themselves.**

**1. MATCHED ON THE WRONG AXIS.** Timeout count is a scalar over the
WHOLE run, and `b5` places mine, chop, build, slope, hill and b15
designations. If one member's timeouts landed in the mine volume and the
other's on chop, they are two unrelated runs with a coincidentally equal
number. Check the positions (`b5_mine_timeout_position_diag`, which
classifies in-volume vs elsewhere) before treating any pair as a control.

**2. MATCHED ACROSS TWO BUILDS.** Confirmed real: wave 8 covered seeds
49-84 on `54c22680` (probe-FREE) and wave 9 covered 121-156 on
`b55c0911` (probe-ACTIVE). So `61` and `52` came from one commit while
`148` and `146` came from another — **every pair was stitched across two
different systems.** Nothing about the numbers looks wrong; they are the
same kind of number, which is exactly what makes this worse than the
axis problem. Resolution: one fan covering 49-156 on a SINGLE tip, and
take all four counts from it.

**The rule both cases point at:** a control is only a control if the two
members were measured *the same way, on the same system, over comparable
work*. A pair matched on a number alone looks more rigorous than an
unmatched one and is not — which is the whole hazard. **Delay a wave
rather than run on stitched counts.**

**Do not run the pairs until all four members' counts come from one
wave on one commit.**

---

## AMENDMENT (before any data exists): the reading table above is SUSPENDED

**Found by reading my own instrument after the control design tightened.
No numbers have been produced, so nothing here is fitted to a result.**

`Server::bastion_cascade_probe` folds four per-`Uid` maps to **maxima** and
emits only `members.len()` — the keys are discarded. Two consequences, the
second of which invalidates the pre-registered table as written:

**1. Cascade activity cannot be attributed to a colonist, therefore not to a
VERB.** The whole egress path (`egress_requests` → `plan_access` at ~12810)
is keyed on `uid` and is **designation-agnostic** — nothing gates it to
Mine. A trapped *chopper* enters the identical path and increments the
identical counters. So on a seed that fails both mine and chop (146), the
probe physically cannot say which verb the cascade belongs to.

**2. The four numbers need not describe the SAME cascade.** They are four
independent maxima over the member set. `frontier_completes_max` may come
from one colonist and `abort_max` from another. The pre-registered rows read
the tuple as one member's profile — e.g. "low ceiling + high resets =
confirmed" — and that reading is only valid if both extrema belong to the
same member. **As built, a high-completes/low-aborts tuple could be two
unrelated colonists and would read as confirmation of bound 1.**

**Required before the fan run:** emit one row per member —
`(uid, frontier_completes, abort_resets, abort_max, access_emissions)` —
totals computed over the whole set, only the inspection list bounded. Then
the table's rows are evaluated **per member**, and the seed-level verdict is
"does ANY member show the confirmed profile," which is the question actually
being asked.

**This is the same `.values().max()` error I flagged in another session's
position map, committed by me, in my own probe, the same day** — see
`aggregate-late-keep-the-structure`. It survived because the collapse was
made where the measurement felt done: the interesting thing about a cascade
*is* its worst member. It stopped being sufficient the moment the study
needed to know WHICH member, and that moment arrived from an unrelated
direction (chop contamination) rather than from anything about the cascade.

**Timing note:** this is cheap now and expensive later — the fix destroys no
baseline because nothing has been run. Discovering it from a confusing
result after the fan would have cost the fan.

## Third arm: seeds 78 and 80 as a scope test (not an explanation)

Chop fails on **8/72 seeds (11.1%)**, and **78 and 80 fail chop at 2 and 4
timeouts** — a band with zero mine failures across 55 seeds. Friction does
not explain those, so a friction-based story has a standing counterexample.

Because the egress path is designation-agnostic, the probe *can* be read on
78/80, and the reading is meaningful in exactly one narrow way:

- **zero cascade activity on 78/80** → those failures are outside this
  mechanism, and the cascade's scope is bounded to friction-heavy seeds;
- **nonzero** → cascade activity exists at 2–4 timeouts, and the friction
  story breaks from the low end.

**Admissibility is the same gate 0**: a zero is only evidence if the probe is
demonstrated nonzero SOMEWHERE in the same run. An uninstrumented path and a
quiet path emit identical silence.

**What this arm does NOT do:** it says nothing about why a tree went unfelled.
The probe measures emergency egress, not chop. It bounds my diagnosis' scope;
it does not diagnose chop.

## Control and treatment must BOTH be clean

The control requirement is now "passes every clause at matched high friction,"
not "passes mining." The same bar applies to the **treatment**: a seed failing
two verbs at once cannot attribute the cascade to either, and per-member
emission does not fully fix that (one colonist may chop and mine). Prefer a
treatment that fails **mine only**.

Void-control tally so far, all found before running: matched on the wrong
AXIS; stitched across two BUILDS; and now **contaminated on a clause neither
side was looking at**. Each was invisible in the numbers.

---

## PARKED (Fable ruling, 2026-07-30) — and the precondition for resuming

**Parked on PRIORITY, not on a verdict.** It concerns 1 seed of 72, its
instrument needs a rebuild, and — decisively — **no sound pair exists in this
corpus.**

### The blocking fact: there is no valid pair, even in principle

A 32-clause check (rather than the two clauses we had been tracking) found
**every member of both pairs contaminated, 4 of 4** — and the treatment too:

| seed | role | failing clauses |
|---|---|---|
| 148 | "control" | 7 — flat_total, log_sum, chop_cleared, build_placed, flat_bounds_ok, b15_ontop, b15_adjacent |
| 52 | "control" | 1 — ch_leaf_cleared |
| 146 | treatment | 8 |
| **61** | **treatment (this doc's subject)** | **5 — mine_cleared, build_placed, any_needs_materials, ch_leaf_cleared, mine_blocks_mined** |

**Seed 61 is not a mine-only failure.** It fails five clauses across four
subsystems, so the cascade may not be what makes its mine fail, and nothing in
the current data separates those.

Across all 72 seeds: **2 mine-only failures** (54 at 22 timeouts, 71 at 20)
and **5 fully-clean high-friction controls** (76 at 29, 74 at 12, 66 at 11,
129 at 11, 142 at 10). **No exact timeout match exists between any clean
treatment and any clean control** — nearest gaps are 7 and 8, far too wide on
a variable whose entire premise is that magnitude does not discriminate.

**PRECONDITION FOR RESUMING — this is the line a fresh session cannot derive
and would otherwise spend a fan rediscovering: no sound pair exists in seeds
49-156. The row needs a WIDER CORPUS SEARCH for a mine-only failure matched
to a fully-clean high-friction seed. Do not rebuild pairs from the seeds named
above; all four are contaminated.**

Void-control tally, all caught pre-run: wrong AXIS · stitched across BUILDS ·
contaminated on an UNWATCHED CLAUSE · **no sound pair available at all.**
Exact-count matching on a noisy scalar was fragile from the start.

### What was NOT wrong: the probe is behaviour-neutral, empirically

Waves 10+11 covered **seeds 49-84 — 36 seeds, zero drift on six deterministic
fields** (`mine_blocks_mined`, `mine_cleared`, `travel_timeouts`,
`max_same_target_timeouts`, `chop_cleared`, `log_sum`) against probe-free
`54c22680`. Seeds 61 and 71 are included. Two consequences: measurements may
be combined across the probe boundary (so the 72-seed distribution and the
0/55 low-friction result stand), and **seed 61's five failing clauses predate
the probe** — they are not an artifact of this instrumentation.

### Carried debt: the per-member fix was written, then REVERTED unbuilt

The fix for the suspension above was implemented and **deliberately discarded
rather than committed**, because the box was on another lane's critical path
and committing unverified code is not allowed. Re-implementing is minutes:

- add `Server::bastion_cascade_probe_members() -> Vec<(u64,u32,u32,u32,u32)>`
  beside the maxima accessor — build a `BTreeMap<u64, (u32,u32,u32,u32)>`
  keyed on the **raw** uid (`uid.0.get()`), which sorts for free and avoids
  reconstructing a `Uid` from a `u64` (the inner value is a `NonZeroU64`);
- fill it from all four maps, then emit as `b5_cascade_probe.members` with
  every row — no cap: the set is colonist-bounded and truncation would hide
  diffuse cascades while keeping concentrated ones;
- keep the maxima accessor for the gate-0 aggregate. Bound the detail, never
  the aggregate.

**Live defect found while writing it, still present in committed code:**
`bastion_cascade_probe`'s `members_seen` chains only `cascade_frontier_completes`
and `cascade_access_emissions`. A member that reached an ABORT without ever
completing a frontier or emitting a plan is **invisible to it** — i.e. it
undercounts exactly the members whose bound-1 behaviour this row studies.
Anyone reading `members_seen` today should know it is a lower bound.
