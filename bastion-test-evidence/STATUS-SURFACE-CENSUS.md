# THE COLONIST STATUS SURFACE IS ~DEAD — measured, with its control

Verifying a load-bearing roadmap claim at the current tip before anyone acts on
it, per the rule that a named symbol must be re-checked rather than recalled.

## The roadmap's claim, and what survived checking

> *"`running = true` appears NOWHERE … ⇒ energy never leaves maximum, which
> makes `route_energy_ready` permanently true and `RestingToClimb` **unreachable
> by construction**."*

| check | result |
|---|---|
| `running = true` / `running: true` in the shipped tree | **0 hits** — claim holds |
| `route_energy_ready` as a **function** | **does not exist** — it is a local `let` at `bastion_jobs.rs:13351`, not a `fn`. The roadmap names it as if it were one |
| the predicate itself | `energy.current() >= energy.maximum()` — so the status needs energy **below max**, and `RestingToClimb` is written only under `!route_energy_ready` |

## The empirical check — and it is wider than the claim

`BastionColonistStatus` has **4** variants. Across **33,926** observations of the
status field in the banked corpus:

| variant | observations |
|---|---|
| *(None)* | **33,833 — 99.7%** |
| `Replanning` | 92 |
| `WaitingForLadder` | **1** |
| `RescueImminent` | **0** |
| `RestingToClimb` | **0** |

**Two of four variants have never been observed; a third exactly once.**

★ The 82 logs that appear to mention `RestingToClimb` are **script note text** —
the driver echoing a comment about the fixture. Zero are real emits. Grepping the
name alone would have reported the status as alive.

## The control, without which the zeros are worthless

A zero means nothing unless the instrument could have fired:

| witness | value |
|---|---|
| `BASTION_STATUS_STAMP_DIAG=1` present in the corpus | **133** occurrences — the diagnostic **was** enabled |
| `bastion: status stamp` emits, corpus-wide | **0** |

**The edge-triggered status-stamp diagnostic has never produced a single
observation in any run, despite being switched on 133 times.** So the zeros are
real zeros, and the instrument built to watch this surface has never once
reported.

## What this changes

The roadmap frames this as one gait's missing trigger. The measurement says the
scope is larger: **the whole status-display surface is inert**, not just the run
gait's corner of it. `RescueImminent` is equally unobserved, and it does not
depend on the run gait at all — so "spec the trigger or retire the gait" would
fix at most one of two dead variants.

**No decision taken here** — the run-gait call is Ben's and it stays his. What is
added is that the call is about a surface with two dead variants and a
never-firing diagnostic, not about one gait.
