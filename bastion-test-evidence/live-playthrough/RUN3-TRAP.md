# Live Playthrough — Trap Run (rows 14-17 attempt)

Goal per Opus's directive: force real `stuck_strikes` accumulation via a
genuine, player-paintable overhang/floating-shelf trap, to exercise Row B′'s
bench/graduate lifecycle (rows 14-17) live. Banner fix landed first
(commit d5dfd30e54) — this run's boot line reads `2f59ccde`, matching the
tip it actually ran. Flags: `BASTION_ROWB_BENCH=1 BASTION_ROWB_DIAG=1`.

## Tooling built for this run

Added a `survey` command to `bastion_playtest.rs` (committed) that reads the
client's own cached `TerrainGrid` — the same data a real renderer would use,
not a harness-only view — and scans a 2D box for columns whose topmost
filled cell sits over N+ consecutive unfilled cells before hitting solid
ground again ("overhang candidates"). `script-03-survey.txt` ran this over a
140x140 box around spawn: 19,881 columns, 4,043 candidates at `gap>=4`
(confirming trees/terrain irregularities are common — most candidates are
almost certainly tree canopies, not cliffs, since the server's own
minability check, `block.is_filled()` at `bastion_jobs.rs:1294`, doesn't
distinguish wood from stone either).

## Two trap attempts, both failed to reach 3 strikes — and now I know exactly why

**Attempt 1** (`script-04-trap.txt`): picked the most extreme candidate
found, `(15183, 16048)`, gap=42 (a 42-cell drop under a filled cell at
z=460). Painted a tight 3x3x3 box around it (11 mineable cells). Result:
caught almost immediately by `plan_access`'s definitive "no route exists"
verdict (`bastion_jobs.rs:13096-13116`, the **self-rescue carve-planner**) —
`job.unreachable = true` set directly, "designation marked BLOCKED (task
#55)" fired, **zero contribution to `stuck_strikes`**. One claim, one
release, done — this path bypasses the churn counter entirely.

**Attempt 2** (`script-05-trap2.txt`, same live server/colony): picked a far
more moderate candidate, `(15177, 16008)`, gap=8, at the player's own
elevation (z=424 vs spawn z=419) — a small ledge, not a 40-block cliff.
Result: job 444 got claimed **three separate times** by the same colonist
(21:30:05, 21:32:30, 21:33:50) and released once via the routine churn path
("job unreachable — claim released", `bastion_jobs.rs:11574-11581`) — the
path that DOES feed `stuck_strikes` — but the same `plan_access` self-rescue
check also fired twice in between (job 444 at 21:32:28, a sibling job 442 at
21:33:39) and won the race: at 21:34:05 job 444 was **phantom-retired**
(task #57 — its target cell no longer matched the designation; the tree/
terrain there changed state on its own) before strikes could accumulate to
3. `ROWB-DIAG` count for the whole run: **0**, both attempts.

## The actual finding: two independent unreachability paths, and only one feeds Row B′

- **The churn path** (`job.unreachable = true` + `churn_events.push(...)`,
  "job unreachable — claim released") is explicitly documented in its own
  comment as "transient congestion that RETRIES and often resolves itself,
  not a permanent block" — deliberately NOT hooked to task #55's blocked-
  designation message, on purpose, to avoid false-alarm spam. **This is the
  only path that feeds `stuck_strikes`, and by extension the only path Row
  B′ can ever act on.**
- **The self-rescue carve-planner** (`plan_access(...) -> None`) is a
  separate, stronger, more confident check — when it returns `None`, the
  code treats that as definitive: `unreachable = true` immediately, task
  #55's blocked message fires immediately, and (as far as this run's
  evidence shows) the job doesn't get the chance to keep accumulating
  churn-path strikes afterward, because the "no route exists" verdict already
  answered the reachability question.

Real overhangs/floating shelves — the terrain class both Opus and I expected
to be Row B′'s natural trigger — get caught by the SECOND path far more
readily than by the first, because they tend to look like genuinely
carve-unreachable terrain to `plan_access`, not merely congested terrain.
That's *good* design on its own terms (a confident "no route" signal
short-circuiting wasted colonist effort is exactly right) — but it also
means the window where a cell is ambiguous enough to survive `plan_access`
while still failing enough real walks to rack up 3 churn-path strikes is
narrower than "any bad terrain," and two honest attempts at real,
player-paintable overhang geometry both missed it.

## Scoring this against Opus's own framing

"If [strikes] don't reach 3, the trap failed and say so; a trap that
doesn't trap is a null about the trap, not about rows 14-17." Both traps
failed. Rows 14-17 remain **not exercised** — now for a *specific,
mechanism-level reason* rather than an unexplained miss: on real terrain,
the self-rescue carve-planner tends to resolve the reachability question
before the churn counter gets far enough. This sharpens, rather than
weakens, the corpus's own finding that the churn-path stuck-job condition
is rare outside adversarial multi-layer terrain (11/48 seeds) — it now
looks like it may require terrain that's specifically ambiguous to
`plan_access`, not just "an overhang," which is a narrower target than
either of us assumed going in.

## Attempt 3 — congestion, not terrain (`script-06-congestion.txt`)

Opus's call: try it. Max colony (16, the server's own cap) all spawned at
once, aimed at a small 3x3x3 slice (18 mineable cells) of the same open
rock the earlier legit footprints used — betting that raw density on a
small target would cause colonists to lose races for the same approach
stance and fail the *walk* (the churn path), without `plan_access` ever
returning `None` (there genuinely is a route).

Result: **no congestion happened at all.** All 18 jobs completed in ~14
seconds flat, zero `job unreachable` events, zero strikes, `ROWB-DIAG`
still 0. The footprint's perimeter had enough distinct approach faces
(3x3x3 exposed on multiple sides, near-surface, fully open — the same
property that made every prior run's larger footprint resolve cleanly)
that 16 colonists simply fanned out around it instead of queuing for a
shared bottleneck. Packing more colonists onto an *open* target doesn't
create contention; it just parallelizes faster. A genuine congestion trap
would need a target with a real single-file physical bottleneck (e.g. a
1-wide corridor), which this driver has no way to detect or construct from
survey data alone.

## Verdict: three honest attempts, three nulls — stopping here per Opus's own instruction

Terrain-extreme (attempt 1), terrain-moderate (attempt 2), and density
(attempt 3) all failed to reach 3 churn-path strikes, each for a distinct
and now-understood reason: attempt 1 was caught by `plan_access`'s
definitive rejection before the churn counter could move; attempt 2 got
partway there but lost the race to `plan_access` and a phantom-retire;
attempt 3 never even created contention because the target had too many
open faces for 16 colonists to collide over.

Per Opus's own stop condition ("three honest attempts at the precondition
is enough to report 'the band is real but very narrow'... rather than
escalating trap ingenuity until something fires"): **stopping here.**
Rows 14-17 remain not exercised live tonight. This is not the same claim
as "the mechanism doesn't work" — the harness already proved bench → 
graduate → re-bench → graduate fires correctly on seed 90's holdout,
strikes reaching 3/2/1/0 there. It's the narrower, now well-evidenced claim
that the band of terrain/situations able to produce that precondition is
real but genuinely rare — rare enough that three different honest,
player-natural attempts at engineering it tonight all missed.
