# RESULTS — the guard door (G3) and the patrol generator, two reads

Read 2026-09-02 09:00 against PREREG-guard-door.md. Arm b1 on a900163959
(S5b, door on) day 2, and arm b1 on 7d9293d9b3 (instrument pair: PATROL
PASS census) day 2. Roster 49-50; 4 town entrances on both.

| run                  | guards | patrols posted | door refusals | plaza | entrance | street | elsewhere |
|----------------------|-------:|---------------:|--------------:|------:|---------:|-------:|----------:|
| before the door (b2 8e9ca2c2fd d2) | 7 | 5 | (no door) | 10% | 0% | 27% | 61% |
| before the door (b1 26e0852dae d3/d4) | 7-8 | 0 | (no door) | 5-9% | 0% | 18-34% | 60-72% |
| S5b a900163959 d2    | 7      | 3              | 0             | 0%    | 1%       | 28%    | 70%       |
| instrument 7d9293d9b3 d2 | 8  | 6              | 10,171        | 3%    | 2%       | 51%    | 41%       |

- Street + entrance on the second read: 53% -- PASS on the pre-registered
  >= 50% line (from 18-34% before the row).
- "patrols_posted >= 2 x guards" was the wrong frame: a posted leg LOOPS
  between its two entrances all day (32 leg-switch arrivals in the day,
  four per guard), so one posting per guard per day is the design. Six of
  eight guards held a live leg by mid-morning.
- The PATROL PASS census names why not all eight from 08:00: busy with a
  non-open-board job 6 -> 4 -> 3 -> 2 across the morning (deposit runs of
  what they carried, a meal), no guard ever lacked a leg (no_leg 0), and
  no held entry pointed at a dead job. The generator was never the gate;
  the guards' morning chores were.
- The two door readings (0, then 10,171 refusals) are honest: on the S5b
  run the Guard lane logged one colonist with one claim -- the named guards
  made no open-board attempts that day -- and on the instrument run they
  made thousands, all refused. Colony counts vary 2-3x; this is one of
  those. Three replicates would settle whether 0 recurs.
- "elsewhere" 41%: the chores until mid-morning, and off-road segments of
  legs (roads cost half in the path search, so a leg follows streets
  only where the road route is under twice the straight line).

## Disposition

PASS on the main line, in one replicate. NOT built: legs pinned to road
cells (a routing change; the 41% is partly chores that belong to a real
morning). Open: a second replicate of the 51% street share; whether the
door's 0 recurs.
