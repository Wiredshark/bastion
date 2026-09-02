# PREREG — a guard's work day is the roads

Written 2026-09-02 04:25, before the build. Source: GUARD SUMMARY day 2 on
flat arm b2 (8e9ca2c2fd) and day 3-4 on flat arm b1 (26e0852dae).

## What the arms showed

| arm/day | guards | patrols posted | plaza | street | entrance | elsewhere |
|---------|--------|----------------|-------|--------|----------|-----------|
| b2 d2   | 7      | 5              | 10%   | 27%    | 0%       | 61%       |
| b1 d3   | 7      | 0 (no entrances on that stage) | 5% | 34% | 0% | 60% |
| b1 d4   | 8      | 0              | 9%    | 18%    | 0%       | 72%       |

The census samples Guard-lane colonists during the Work block (hours
8-15). Six in ten work-hour samples find a guard neither on a road, at
an entrance, nor on the plaza. The patrol generator (row G1) posts a leg
only for a guard with NO active job, and a guard's work priorities
(`in_lane(Guard)`) admit every other lane as fallback; the open board
hands the guard a haul or a farm cell within the tick after a leg ends,
before the next 300-tick generator pass can post the next leg. Five
patrols a day for seven guards is the arithmetic of that race.

## Mechanism (pure, deterministic)

THE GUARD DOOR. In the Work block, while the town has at least two
entrances to walk between, a Guard-lane colonist's claim scan gives
non-Guard jobs priority 0 (the same door shape as the haul gate): the
open board cannot pull a guard off the roads. The generator then finds
every guard idle at each pass and posts the next leg. Witness: a daily
`guard_door_shut` count on the GUARD SUMMARY (claims refused by the
door). Identity: fewer than two entrances (nothing to patrol), outside
the Work block, or `BASTION_NO_GUARD_DOOR`. Prior art: RimWorld (a
drafted pawn does no work; guard duty as an assigned zone), Banished
(a profession works only its building), Dwarf Fortress (squads on
patrol routes do not take labours).

## Pre-registered pass / fail (flat arm, day lines 2-3)

- PASS: `street_pct + entrance_pct >= 50` on every GUARD SUMMARY from day
  2 (b2 day 2: 27), `patrols_posted >= 2 * guards`, `elsewhere_pct <= 30`.
- FAIL: `elsewhere_pct` stays >= 50 with `patrols_posted >= 2 * guards`
  -> the guards are walking between posts OFF the roads (the leg chooser
  or the router), not taking other work; `patrols_posted < guards` with
  the door shut -> the generator is not posting (its idle test or the
  entrance list), and the door has merely idled the guards -- that is the
  starve-the-protectee case and would REVERT the door.
- Falsifier of the design: if the Farm and Haul lanes' works fall by
  more than the guards' former share of them (the guards were doing real
  work the town needed), the town is under-staffed and the professions
  row (how many guards a town of 50 names) is the row, not the door.
