# PREREG — a far target starts at a far tier (W3)

Registered 2026-09-02 14:20, before the binary exists.

## What the W2c read showed

Arm b1 on 5d4ce3ad69, game day 0 by 18:00: the wedge population moved
into the barn: nine hauls at (7649, 6388, 183) with `route_head` (7649,
6390, 183) two blocks north through the barn's wall, `path_state`
Exhausted at tier Small, items 38-49 blocks north-east. W2c's escalation
fired on every ban (tier_after_drop Long x4, Medium x2) but these trips
never banned a climb: each re-aimed at a new item or cell, and the
pathfinder starts every new target at Small ("a new target is a new
problem", path.rs ~937 — the rule that stops one doomed errand poisoning
an agent's later searches). Small's 500 iterations do not find the barn's
door and the road from inside.

## Mechanism

`initial_path_length_for(distance)`: <= 30 blocks Small, 31-60 Medium
(5,000 iterations), > 60 Long (25,000); Longest never at the start. The
ladder escalates from the start tier as before. Pure and pinned. common/
changed: both binaries rebuilt.

## Baseline (W2c boot by 18:00)

probes 22; barn-wall cluster 9; Small-Exhausted 15 of 22; store deposits
11; shuns 26; EatFrom expiries 8; tick rate under compile load ~19.

## Pre-registered outcomes (arm b1's day 0 on the W3 pair, by 18:00)

- Instrument validation: no probe with tier Small whose target lies more
  than 30 blocks from the feet (the probe carries to_item and tier).
- PASS: the barn-wall cluster <= 2; probes total <= 15; store deposits
  >= 12; shuns <= 26; the server's tick rate over the afternoon >= 15
  (from the census tick deltas against wall time).
- FAIL branches: the cluster persists at Medium/Long Exhausted -> there is
  no route from inside the barn to that store and the barn's door (or
  the store's approach) is the row; TPS < 15 -> the tiers are too dear
  and the thresholds move up; probes rise elsewhere with Path routes ->
  the bigger searches find routes the mover cannot take (W4's vaults).
- NOT evidenced live yet.
