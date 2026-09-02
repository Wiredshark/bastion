# PREREG — danger answered like a town (a raid arm, first measurement)

Written 2026-09-02 09:05, before the raid arm boots. Every flat arm so far
ran with -NoRaids; the danger rows (alarm with a radius, militia to posts,
civilians indoors, doors that hold) have NO flat-arm evidence. The engine
already emits the witnesses: ALARM RAISED, "civilian DROPS WORK and runs
home", MUSTER, AUTO-GUARD posted and STAFFED, DOOR FIGHT, DOOR GAVE WAY,
HOSTILE BUDGET expires, ALARM over, ALARM shelter released; the EXPERIENCE
census carries `running=` (0 on every NoRaids run) and `downed=`.

## Instrument validation first (step 3)

Before any hypothesis: one raid on the arm must show ALARM RAISED with a
radius, at least one "runs home" line, at least one MUSTER, and `running`
> 0 during the alarm and 0 outside it. If `running` stays 0 through a
raid, the running witness is blind and is fixed before anything else is
read.

## Pre-registered reads (per alarm, day 1-2 of a raid arm; three raids
or nothing)

- CIVILIANS INDOORS: "runs home" lines >= 80% of the non-militia roster
  within 30 s of ALARM RAISED; `running` returns to 0 within 60 s of
  ALARM over.
- MILITIA TO POSTS: MUSTER >= the number of Guard-lane colonists awake;
  AUTO-GUARD STAFFED >= 1 per alarm.
- DOORS THAT HOLD: DOOR FIGHT lines >= DOOR GAVE WAY lines (a door holds
  more often than it breaks); raiders reaching a bed cell 0.
- OUTCOME, NOT RESPONSE: `downed` per raid <= 1 with the roster >= 40;
  the fed/starving series unaffected the next morning (no post-raid
  hunger cliff).
- FAIL branches: runs-home < 50% -> the alarm radius or the civilian
  hearing test; MUSTER 0 with guards awake -> the militia drive; DOOR
  GAVE WAY > DOOR FIGHT -> door strength / raider budget; downed >= 3 ->
  the intercept is too late (posts vs entrances), a candidate for the
  entrance posts from the patrol row.
- Falsifier of the frame: if the flat arm never raises an alarm in two
  days (no raider reaches the town, or the spawn is off the flat map), the
  arm cannot evidence these rows and the read moves to Ben's session.

Prior art named: RimWorld (raid -> drafted colonists, others to a safe
room), Dwarf Fortress (burrows and military alerts), Banished (no
combat; not a model here), Manor Lords (militia muster from farms).
