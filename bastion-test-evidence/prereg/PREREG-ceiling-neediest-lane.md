# PREREG — the hauler ceiling falls back to the neediest lane

Written 2026-09-02 03:15, before the build. Source: HAUL LANE CEILING day-1
lines on flat arm b1 (f5f18c6734) and flat arm b2 (8e9ca2c2fd).

## What the arms showed (two replicates)

| arm | roster | cap | named Haul | demoted | left over the cap |
|-----|--------|-----|------------|---------|-------------------|
| b1  | 49     | 12  | 22         | 1       | 9                 |
| b2  | 49     | 12  | 17         | 3       | 3                 |

The ceiling demotes the weakest haulers to their incumbent trade or their
best other lane; a colonist whose only lane count is Haul has neither and
"stays a hauler" (the row's own pin named that case). On a first day, when
most colonists have only hauled, that is most of the surplus.

## Mechanism (pure)

`neediest_lane(open_by_lane, named_by_lane)`: the non-Haul lane with the
most open (unclaimed) jobs per named colonist, open / (named + 1); None
when no lane has open work; ties break on the lane's name. `cap_haul_lane`
takes it as the LAST resort after the incumbent trade and the best other
lane. The ceiling line reports `after_haul` and `neediest`. Prior art:
RimWorld's work-tab auto-priority (unassigned pawns take the work with
the largest backlog), Banished (idle laborers fill the neediest job).
Identity: BASTION_NO_HAUL_CAP (unchanged) disables the whole ceiling.

## Pre-registered pass / fail (flat arm, day lines 1-3)

- PASS: `after_haul <= cap` on every HAUL LANE CEILING line; the day-after
  JOB SEQUENCE line for the lane the surplus went to shows works > 0 (they
  found work there).
- FAIL: `after_haul > cap` with `neediest = Some(..)` (the fallback is not
  applied), or the receiving lane shows works = 0 with hauls > 0 (they
  kept hauling under a new name -- then the priorities path, not the name,
  is the row).
- Falsifier of the design: if `neediest` is the same lane on every day
  line while its open count never falls, the ratio is measuring a lane
  whose jobs cannot be done (unreachable or material-blocked), and open
  work is the wrong signal -- the fix would then be to count only
  claimable jobs.
