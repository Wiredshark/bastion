# PREREG — the route drop escalates the search tier (W2c)

Registered 2026-09-02 13:05, before the binary exists.

## What P1c showed

Arm b1 on 4ea213f029 (P1c: the probe logs the tier; the drop keeps it),
game day 0 by 12:00: seven stalled fetches, six at the roof-stair spot,
ALL path_state Exhausted at tier Medium (5,000 iterations in total),
top_tier_exhausted false; job 448 three times at the same tier. The
chaser escalates a tier only when its RETAINED search's start equals
the current position; drop_route clears that search, so after a drop no
search ever escalated and each returned the same Medium partial path up
the stair. W2b's reset to Small was worse; P1c's keep was not enough.

## Mechanism

drop_route moves the tier one step up (Small -> Medium -> Long ->
Longest; Longest stays). The ban line reports tier_after_drop. A
dropped route is a route that failed; the next search runs with 25,000
and then 75,000 iterations, which finds the road detour if one exists;
if it exhausts Longest, the probe's top_tier_exhausted names the store as
unreachable by the move set. NOT pinned (a three-line match on a private
field); the read is the falsifier.

## Pre-registered outcomes (arm b1's day 0 on the W2c pair, by 18:00)

- Instrument validation: every CLIMB BANNED (fetch) line carries
  tier_after_drop one step above the probe's tier for that job, and no
  job is probed twice at the same tier after a drop.
- PASS: the W2 bars (spot <= 8 from 37, shuns <= 30 from 60, store
  deposits >= 12) AND the probes at the spot with tier Medium -> 0 AND
  top_tier_exhausted=true probes <= 2 (a road route exists and is found).
- FAIL branches: probes at the spot reach Longest and top_tier_exhausted
  -> no ground route from that side; W1's withdrawal must cover
  (Longest, Exhausted) and the road network there is the row; spot
  probes still >= 9 with Long/Longest and path_state Path -> the search
  finds a complete route that still climbs (the eaves row W5 decides);
  deposits fall below 12 -> the escalation's cost starves the walkers
  (TPS) or the ban prices out a working route.
- Read with W5's founding line if W5 stages first (it will): the two
  rows share a boot only if the pipeline collapses; otherwise W2c's boot
  carries W5 too and the read is of both. NOT evidenced live yet.
