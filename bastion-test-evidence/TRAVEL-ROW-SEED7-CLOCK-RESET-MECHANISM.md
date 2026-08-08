# Seed 7, job 33/34: the stuck-clock resets on backward jumps, not just target switches

Follow-on to TRAVEL-ROW-SEED7-SITE-SURVEY.md, testing Opus's hysteresis
hypothesis ("sub-block oscillation never zeroes stuck_time"). The measured
trace refines it into something sharper and different in shape.

**Reproduced independently on the retry (job 34, same colonist, same
bed, ~55s later after the cooldown): freeze at sdist ≈ 6.15 (vs job 33's
6.13-6.96) for ~6.9s (vs job 33's ~7.03s), then a monotonic climb away
from the target (this time gradual, not a discrete jump), crossing the
`>best_dist+4.0` threshold, ending in the same 22.4-22.7 trapped band at
final timeout (stuck_time 9.97). Same closest approach, same stall
duration, same terminal band, two independent attempts. Not a one-off.**

## Instrumentation

`BASTION_SDIST_TRACE_JOB=<id>` (new, env-gated to a single job id so it
can't leak into corpus runs): per-tick `sdist`/`best_dist`/`reset_dist`/
`stuck_time`, logged right where the existing closest-approach tracker
reads `sdist` (`bastion_jobs.rs`, just before the `min_distance_to_target`
update). ~660 lines for job 33's full attempt.

## What the trace actually shows — not oscillation, STATIONARY + JUMPS

Contrary to "sub-block wobble": for long stretches, `sdist` is **bit-
identical** across dozens of consecutive ticks (e.g. `6.133596420288086`
unchanged for 40+ ticks straight, `stuck_time` climbing 5.03 → 6.37s with
zero measurable movement). The colonist is not oscillating — it is
**completely still**.

That stillness is interrupted twice by **large, discrete jumps**, each
coinciding with `stuck_time` resetting to `0.0`:

| phase | sdist | duration stationary | what happens next |
|---|---|---|---|
| 1 | ~11.6 → 6.13 (real early progress) | — | settles, freezes |
| 2 | frozen at 6.13-6.96 | ~7s (stuck_time 0→7.03) | jumps to 10.52 |
| 3 | jump 6.13→10.52 (+4.4) | — | **stuck_time resets to 0** |
| 4 | jump 10.52→21.5→22.7 (+~12) | — | **stuck_time resets to 0** again |
| 5 | frozen at 22.38-22.69 | ~9.7-10s | STUCK_TIMEOUT fires, churn |

## Precondition check (Opus's ask): the arrive-tolerance-widening hypothesis is dead

Opus noted `ARRIVE_DIST=2.5 + stuck_strikes.min(3)*1.2` caps at exactly
**6.1** at `stuck_strikes>=3`, close to the observed ~6.13-6.15 freeze
point elsewhere in this trace family, and asked for the precondition to be
checked before treating that as evidence. Extended `SDIST-TRACE` with
`stuck_strikes`/`on_ground`/`vel_z` and re-ran: **`stuck_strikes = 0`
across all 660 ticks of job 33.** `arrive` was 2.5 the whole time, nowhere
near 6.1 — coincidence, not mechanism, per his own registered criterion.

**Why it's structurally 0, not just 0 this run:** `RestAt` jobs get a
FRESH job id on every retry (`insert_rest_job`, `stuck_strikes: 0` at
creation, called anew each time `preempt_pending` fires) — unlike Mine
jobs, which reuse the SAME job entry across churns and accumulate strikes
there. The strike-based arrival-tolerance rescue can structurally never
engage for self-jobs: each bed attempt starts its own counter at 0
regardless of how many prior attempts at the same bed already failed.

## Bonus finding from the same instrumentation: the colonist IS jumping

`on_ground`/`vel_z` (added alongside `stuck_strikes`) show the colonist
attempting real jumps during the final trapped zone (sdist ~22.4-22.7):
`vel_z` peaks at 7.48 with `on_ground=false` at `stuck_time` 0.57, 1.4,
6.23, 6.77, 7.17, 8.07 — roughly every 0.5-1.5s. None of them close the
distance. For this specimen, that's Fable's branch 3 (stalls WITH a jump
attempted and failing), not branch 2 (never attempts a jump at all).

## The reset branch fires on regression, not just retargeting

`bastion_jobs.rs` ~11342 (unchanged by this investigation, cited for the
mechanism): `else if sdist > active.best_dist + 4.0 { best_dist = sdist;
reset_dist = sdist; stuck_time = 0.0; }`. Its own comment: *"A large upward
JUMP in the measure means the steer target switched (anchor reached → real
target) — rebase, don't count it as being stuck."*

**That justification doesn't hold for job 33.** A `TGT-DRIFT` correlation
run (`BASTION_SDIST_TRACE_JOB=33` + `BASTION_LEGC_DIAG` together) shows
exactly **one** `TGT-DRIFT (astar-reset trigger)` event during job 33's
entire active window — it fires once, at the very start, establishing
`steer = target = (21872.5, 16025.5, 251.0)`, and **the target never
changes again** for the rest of the attempt (confirmed independently by the
timeout line itself: `steer`/`target` identical at the moment of churn).
**Both large jumps happen with a constant target.** So this isn't the
retargeting case the branch was built for — it's the colonist's own
position moving backward by 4-12 units against an unchanged goal, and the
branch resets the clock anyway because it can't tell the difference: it
only looks at the *size* of the change in `sdist`, not *why* it changed or
which direction relative to genuine progress.

## The mechanism, stated plainly

**A colonist that gets shoved 12 units away from its target is granted the
exact same fresh 10-second grace period as one that just reached a newly-
revealed real target after clearing an anchor.** The branch measures
magnitude, not direction or cause. Combined with the earlier finding
(stationary stretches don't reset via the ≥1.0-forward-progress path
either, since there's no forward progress to record), the net effect on
job 33: two regressions bought roughly 17 extra seconds of "not stuck yet"
time on a target the colonist was moving steadily AWAY from, before the
watchdog was finally allowed to fire.

## What caused the colonist to physically move backward, twice

Not established here — that's a step below this note's scope (the Chaser's
own step-selection logic, not the watchdog). Ruled out: astar/steer
retargeting (no `TGT-DRIFT` at either jump), terrain obstruction at the
final resting spot (already ruled out in the site survey — ordinary
Earth/Grass with one Rock/Wood neighbor, not a wall or pit). Open for
whoever picks up the Chaser-internals read.

## Relationship to Opus's hysteresis hypothesis

Partially confirmed, partially refined. Confirmed: the watchdog's own
design genuinely cannot distinguish "stuck" from "moving the wrong way" in
this case — that half of the prediction lands. Refined: the failure mode
isn't small-amplitude wobble under the 1.0-block threshold (the trace shows
zero wobble at all during the stationary stretches — perfect bit-identical
`sdist`) — it's the *opposite* problem, the ≥4.0-block "big jump, don't
count it" branch firing on genuine backward displacement rather than
legitimate retargeting.
