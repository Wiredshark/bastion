# PREREG — batched hauls, dedicated haulers, zone assignment, zone-scoped work
(Ben's ruling, live 2026-09-01 20:35-20:50; written 21:15, before any of it is built)

## What Ben saw, and what the log measured

"They do a job, haul, do job, haul" — census over six game days: working 0-3
of 9, moving 4-7 of 9; HAUL-GEN admitted 4,179 per-item loads in 16,978
generations, pending 9-27 at all times. The JOB SEQUENCE CENSUS (2a022283d5's
successor commit) gives it a number first: mean_alternations and
mean_max_work_streak per lane per day. BASELINE FIRST — nothing below is
built until one arm has printed that census across two day boundaries.

## Prior art (mechanisms, not stories)

- RimWorld: Hauling is its own work type; every pawn has a priority per work
  type (the work tab); items lie where produced until a hauler takes them;
  a worker with "haul" at low priority hauls only when nothing higher exists.
- Dwarf Fortress: hauling labours per dwarf; workshop output accumulates at
  the workshop; stockpile links pull it; wheelbarrows batch loads.
- Banished: a field's workers farm the whole field; produce goes to the barn
  in trips of a carrying capacity; labourers do the generic hauling.
- Manor Lords: a family is ASSIGNED to a plot/building and works all of it;
  carts and ox-haulers move goods; assignment is visible per building.
- Song of Syx / Sims: rooms/zones with assignees; the assignment is the UI.

## Mechanisms to build, in order

M1 ZONE ASSIGNMENT (server model): `board.assignments: HashMap<ZoneId,
   Vec<(Uid, AssignSource)>>` with AssignSource {Auto, Manual}; a colonist
   has at most one assignment. Auto-assigner runs daily and on zone changes:
   fill each work zone (farm region, mine designation region, workshop,
   stockpile for haulers) from its lane, by zone size; Manual entries are
   never touched by Auto. Witness: ASSIGNMENT CENSUS daily (zone, kind,
   assignees, source). Inspector Identity section gains `assigned_zone`
   (+ source); a ZONE panel lists assignees (voxygen, second half).
M2 ZONE-SCOPED WORK: a colonist with an assignment claims only jobs whose
   pos lies inside the zone's region while any exist there; falls back to
   lane jobs elsewhere only when the zone has none. Reuses the existing
   lane-commitment preference (JOB COMMITMENT) as the scoring hook.
M3 BATCHED HAULS: the haul generator keeps admitting loads, but the CLAIM
   side gates them: a non-Haul colonist may claim a haul only when the day
   block is past the work block (shift end) OR the backlog at its zone
   (unreserved units on the ground inside the region) >= HAUL_BACKLOG_UNITS
   (= HAUL_CHAIN_MAX_LOAD, 16, so a trip is a full chain). Haul-lane
   colonists claim hauls any time.
M4 DEDICATED HAULERS: the daily profession tally reserves a Haul lane share
   (1 hauler per 6 colonists, min 1 at roster >= 4; a number, taste — ask
   Ben if he wants another), assigned to stockpiles; the tally already
   votes by time held, so haulers stay haulers.
M5 MANUAL ASSIGN TOOL (voxygen): ToolMode::Assign — click a colonist, then a
   zone; Erase clears. The zone panel shows AUTO/MANUAL per entry and the
   auto-assigner's last reason per colonist.

## Pre-registered pass / fail (read on a flat arm across two day boundaries)

- Baseline (before M3): record Farm mean_alternations and
  mean_max_work_streak from the JOB SEQUENCE CENSUS.
- M2 PASS: for assigned farmers, >= 90% of claimed work jobs lie inside the
  assigned zone's region while the zone has open jobs (new field
  `in_zone_pct` on the census). FAIL: below 75%.
- M3 PASS: Farm mean_alternations falls by >= 3x from baseline and
  mean_max_work_streak rises >= 3x; hauls still happen (haul_share > 0) and
  the stockpile still receives produce (FOOD PAR / stock does not fall
  below the pre-change run's floor). FAIL: alternations unchanged, OR the
  stockpile starves (backlog rots on the ground) — the second is the
  falsifier of "haul at shift end": if produce never reaches the stockpile
  in time, the backlog threshold, not the schedule, must drive hauling.
- M4 PASS: with >= 1 dedicated hauler, farmers' haul_share_pct falls below
  20% and the stockpile inflow per day is >= the baseline's. FAIL: farmers
  still haul >= 40% of their claims, or inflow drops.
- M1/M5: assignment visible in the inspector and the zone panel; a manual
  assignment survives the daily auto pass (pin) and a manual erase returns
  the colonist to Auto (pin).

## What would falsify the design

If, with batching on, the census shows the SAME travel share (moving 4-7 of
9) because travel is between zone cells, not hauls, then hauling was not the
cause Ben saw and the row is zone geometry (field far from bed/plaza), not
batching. The ROUTE / travel share on the EXPERIENCE census is the check.
