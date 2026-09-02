# PREREG — guards patrol the streets and the entrances (Ben, live 2026-09-01 22:30)

Written 22:40, before any of it is built.

## What Ben saw, and the producer

"Guards need to patrol the streets and the entrances to the town, not just
sit in the town centers." Producer (read): the Guard lane has NO off-alarm
job. `JobKind::Guard { mode: Alarm | Fight, post }` is posted only by the
muster (nearest hostile spot / cry clusters); between alarms a guard is an
idle colonist and takes the lounge seat like everyone else (RECREATE),
which on an adopted town is the plaza. The board already knows the streets
(`road_cells`, exported at adoption) and the settlement bounds
(`settlement_bounds` = interiors bbox +- 24), and the plaza
(`gathering_anchor`).

## Mechanisms

G0 INSTRUMENT: `town_entrances(road_cells, bounds)`: road cells within
   3 blocks of the bounds rectangle's edge, clustered >= 12 blocks apart in
   a deterministic order, at most 8. GUARD CENSUS: every 300 ticks during
   the Work block, each Guard-lane colonist is bucketed: at the plaza
   (<= 12 blocks of gathering_anchor, XY), at an entrance (<= 8 blocks of
   one), on a street (a road cell), elsewhere; printed daily per guard and
   as a town summary with the entrance count. Pin: a cross of roads through
   a square bounds yields four entrances >= 12 apart; no roads -> none;
   no bounds -> none (identity).
G1 PATROL: `GuardMode::Patrol` (appended LAST). During the Work block a
   generator posts, for each Guard-lane colonist without a job, a Patrol
   job at entrance `(guard_index + hour) % n` so entrances stay covered
   and rotate; the job completes after a dwell of one game hour standing
   within 2 blocks of the post, then the next post is given. An alarm's
   muster still replaces the held job (existing path), so patrol never
   delays the answer. Off the Work block, nothing changes (evenings social,
   nights in bed). `BASTION_NO_PATROL` = identity.
G2 SEEING IT: the inspector's Right-now row names the post ("patrol:
   east gate"); the ASSIGNMENT CENSUS style line lists posts and who holds
   them.

## Prior art

Dwarf Fortress (patrol routes and guard stations), Stronghold (patrol
paths between towers), Manor Lords (retinue stationed at gates), Mount &
Blade (town patrols on a loop).

## Pre-registered pass / fail (flat arm with >= 2 guards, two day boundaries)

- G0 baseline: record plaza share per guard (expected high, Ben's
  observation) before G1.
- G1 PASS: with n entrances and g >= 2 guards, each entrance has a guard
  within 8 blocks in >= 50% of Work-block samples (>= 50% x min(1, g/n)
  when g < n), and the plaza share falls below 30%. FAIL: plaza share
  stays >= 60% or any entrance is never visited in a game day.
- ALARM UNCHANGED: on a planted raid, time from the alarm to the first
  guard at a cry cluster is not worse than the pre-patrol run (the existing
  DANGER witnesses).
- Falsifier of the design: if `town_entrances` finds 0 or 1 entrance on a
  real adopted town (roads do not cross the bounds edge as assumed), the
  post source is wrong -- fall back to the road cells farthest from the
  plaza and say so in the census; do not ship a patrol that circles the
  plaza.
