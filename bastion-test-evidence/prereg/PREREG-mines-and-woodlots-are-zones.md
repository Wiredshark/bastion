# PREREG — mines and woodlots are zones: "the colonist should mine the entire area"

Written 2026-09-02 02:05, before the build. Source: the code (the
assignable-zone list is built from `board.farms` and `board.stockpiles`
only) and flat arm b1 day 3 (JOB SEQUENCE: Farm lane in_zone_pct=100,
scoped_claims=4; every other lane scoped_claims=0, in_zone_pct=0).

## What stands today

Ben's ruling: "if there's a field or a mine etc, the colonist should mine
or farm the entire area." The zone-scope row (M2) delivered that for
farms: an assigned farmer's claims are restricted to their field while it
has work. Mines and woodlots are DESIGNATED regions (`DesignationKind::Mine`,
`::Chop`) that mint per-cell jobs, but they are not zones: they get no
ZoneId, no assignee, no scope, no line in the ASSIGNMENT CENSUS, and no
"Assigned to" row in the inspector. A miner picks the nearest open Mine
cell anywhere, then the nearest anything.

## Mechanism (pure, deterministic; same shape as farms)

WORK ZONES. `place_designation` registers a Mine or Chop region as a work
zone exactly as it registers a farm: `board.work_zones.push((id, region,
kind.work_type()))` from the same `next_zone` counter. The daily
`assign_zones` list gains one entry per work zone (its lane = the
designation's work type, its open-work count = its live Designated jobs).
`zone_scope` and the "Assigned to" row see work zones through the same
`(ZoneId, Region)` slice farms use. Removal follows the farm rule (a
cancelled designation drops its zone). Prior art: Dwarf Fortress (mining
designations are areas; a miner works the designation), RimWorld (work
zones and area restrictions), Song of Syx (job slots per building).
Identity: `BASTION_NO_ZONE_SCOPE` already restores unscoped claiming for
every lane; no new switch.

## Pre-registered pass / fail (flat arm with a Mine and a Chop designation,
two day lines)

- PASS: ASSIGNMENT CENSUS lists the Mine/Chop zones with kind=Mine/Chop;
  on day 2+ each has >= 1 assignee while it has open work; the JOB
  SEQUENCE Mine and Chop lanes show scoped_claims > 0 and in_zone_pct
  >= 80 (farms: 100).
- FAIL: the zones list but nobody is assigned (the lane has no named
  colonist -- then the professions row, not this one), or in_zone_pct
  stays 0 with assignees (the scope is not on the claim path the miners
  take).
- Falsifier of the design: a miner assigned to an exhausted mine idles
  while another mine has work -- `zone_scope` already lets go when the
  zone is worked out; if the census shows `unassigned` rising while Mine
  jobs stay open, the release is not firing for work zones.
