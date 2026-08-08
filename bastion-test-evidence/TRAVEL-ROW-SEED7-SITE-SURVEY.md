# Seed 7 site survey: the colonist is pinned to one y-coordinate, at the fixture's own boundary seam

Per TRAVEL-ROW-SPEC.md §4.3 (site-survey the terrain — "applies unchanged"
from the seed-1337/92 corner-cell method). Run ahead of §4.1/§4.2's wiring
since the diagnostic was already built and cheap; does not block or
substitute for the corpus-wide classification work, which is still next.

## The measurement: six timeouts, one y-coordinate

Full seed-7 trace, `BASTION_LEGC_DIAG` + a one-shot terrain dump
(`BASTION_STUCK_TERRAIN_DIAG`, added this pass, env-gated) at each timeout:

| job | kind | actual_pos (feet) | column at feet | cardinal neighbors |
|---|---|---|---|---|
| 33 | RestAt (bed) | (21868, **16003**, 250) | Air,**Wood**,Air×4 (z 248-253) | -x=Wood, others Air |
| 17 | Mine | (21875, **16003**, 243) | Earth,Grass,Air×4 (z 241-246) | +y=**Rock**, others open |
| 18 | Mine | (21875, **16003**, 243) | same | +y=**Rock**, others open |
| 34 | RestAt (bed, retry) | (21868, **16003**, 250) | same as job 33 | -x=Wood, others Air |
| 17 | Mine (retry) | (21875, **16003**, 243) | same | +y=**Rock**, others open |
| 18 | Mine (retry) | (21875, **16003**, 243) | same | +y=**Rock**, others open |
| 19 | Mine | (21876, **16003**, 244) | same shape | +y=**Rock**, others open |
| 20 | Mine | (21876, **16003**, 244) | same shape | +y=**Rock**, others open |

★ **`y` is 16003 (±0.55) at every single one of eight timeouts, across FOUR
different job targets (two different beds attempts, four different mine
targets) spanning ~50 sim-seconds.** `x`/`z` vary modestly, tracking
whichever target is current — only `y` is frozen. That is not what terrain
obstruction looks like (different targets would meet different obstacles at
different places); it is what a **fixed anchor point** looks like — the
colonist keeps ending up back at (or never leaving) the same y-line
regardless of what it's walking toward.

## Where that line is: exactly the fixture's own boundary

`preempt_scenario` builds its flattened plateau across `y in (cy-12)..=(cy+12)`.
The bed sits at `y = cy` directly (`bed = Vec3::new(cx-6, cy, gz+1)`), and the
bed's own `need preempt` line puts `cy = 16016`. **`cy - 12 = 16004`.** The
colonist is pinned at `y ≈ 16003` — **one block outside the flattened zone**,
in real, unflattened terrain, at the exact seam where the deliberately-built
safe plateau ends.

## What's actually there — real terrain, not a wall

The two distinct feet positions show genuine, unremarkable natural ground:
Earth-then-Grass-then-open-air for the mine-side position (with a single Rock
block immediately to +y — the direction back toward every target), and a
single Wood column (almost certainly a tree trunk) immediately to -x for the
bed-side position. Neither is an enclosing wall or a pit — a colonist that
could path normally should be able to walk past either. **This rules out
"the colonist is buried/walled in"** as cleanly as the flat-plateau finding
already ruled out "the target itself is unreachable terrain."

## Reading, held provisionally

The regularity (same y to within rounding, across different targets, on a
FLAT-terrain colonist that the LEGC-DIAG trace confirms is receiving a fresh
`Goto` every tick) points away from organic per-target obstruction and
toward something that keeps **placing or holding** the colonist at this one
line — a stale cached waypoint, a chunk/fixture-boundary artifact in the
Chaser's own route planning, or a "safe fallback" the astar-reset logic
(`TGT-DRIFT`, already observed firing repeatedly across these same events)
retreats to. Not confirmed — this note stops at the measurement and the
ruled-out alternatives, per the row's own §4.3 ordering (classify before
diagnosing the mechanism further); the Chaser internals themselves are a
different file and a separate read.

## Consequence for TRAVEL-ROW-SPEC §4.1/4.2

This specimen is exactly the kind of case `min_distance_to_target` should
classify as **UNREACHED**, not UNREACHABLE — every LEGC-DIAG line already
shows `sdist` in the 11-22 range (not INFINITY, not hundreds), meaning the
colonist got meaningfully close by any reasonable threshold, on a target with
open terrain around it. Once §4.1 lands, seed 7 is the natural first
cross-check that the classifier agrees with what's already been read here by
hand.

## Instrumentation added this pass

`BASTION_STUCK_TERRAIN_DIAG` (env-gated, fires only at travel-timeout events,
~6-8/run) — dumps the block-kind column (feet z-2..z+3) and the four
cardinal neighbors at the colonist's actual position, right where
`BASTION_LEGC_DIAG`'s existing "travel timeout firing" line already fires.
Zero cost when unset, matching every other diagnostic in this file.
