# RESULTS — danger answered like a town: three first-raids, read against PREREG-danger-answered-like-a-town.md

Read 2026-09-02 10:22-12:58. Arm b1 as the raid arm (raids on, flat
town, roster 49) across three boots; each boot's FIRST raid is one
replicate, because the pipeline restarts b1 every 45-60 min and no boot
runs long enough for three raids. The wealth-scaled raid fires at the
same clock on every boot (game day 0, 18:00, wealth ~643, two raiders,
origin 48 blocks out), so the replicates are matched.

## Instrument validation (the pre-registration's first gate)

ALARM RAISED, "civilian DROPS WORK and runs home", "AUTO-GUARD posted and
STAFFED (militia muster)", `running` > 0 in the census, DOOR FIGHT and
DOOR GAVE WAY all fired on every boot. The alarm line's counters were
misread once (D1: `skipped_working` counts workers preempted for shelter,
not left at work) and corrected; the already-home count is missing from
the line (D1b, queued), so "civilians indoors" has only half its
numerator on these reads.

## The three first-raids

| boot / pair              | sheltered | of them workers preempted | out of earshot | musters per cry | running peak -> 0 | downed | doors: fights / gave way (held s) |
|--------------------------|----------:|--------------------------:|---------------:|----------------:|-------------------|-------:|-----------------------------------|
| 96022099c9 (P1b), 3 cries | 24 / 14 / 14 | 23 / 14 / 10          | 9 / 15 / --    | 2 / 2 / 2       | 22 -> 0 in 900 ticks (30 s) | 0 | 123 / 4 (10.03 each)          |
| 35cd156e00 (W1)          | 23        | 21                        | 5              | 2               | 20 -> 14 -> 0      | 0      | 0 / 0                             |
| a1441bcf4e (W2b)         | 23        | 22                        | 5              | 4               | 22 -> 18 -> 9 -> 22 -> 5 -> 0 | 0 | (door lines not tabulated)   |

One raid produces several alarm cycles: "ALARM over" fires when the
drive leaves Defend, and a fresh cry re-raises it while the raiders
still stand (three cycles on the P1b boot, two on the W2b boot).

## Against the bars

- ALARM WITH A RADIUS: 23-24 sheltered plus 5-9 out of earshot on a
  roster of 49 with 2 militia; the radius is real (some are out of it)
  and it reaches the working half of the town. PASS as an instrument;
  the "indoors >= 80% within 30 s" number cannot be scored until the
  already-home count is on the line (D1b): 23 sheltered of 47
  non-militia is 49% by the line alone, and the remainder were skipped
  as already home in the code path, uncounted.
- RUNNING RETURNS TO 0 WITHIN 60 S: PASS on all three (900 ticks, ~30 s,
  on the P1b boot; two to five census steps on the others).
- MILITIA TO POSTS: two guards mustered per cry on every boot (the town's
  Guard lane on day 0 is two to five); "muster >= guards awake" is
  partial and NOT scored (the guard count at 18:00 is not on the line).
- DOORS THAT HOLD: FAIL as designed. DOOR FIGHT is a per-tick line while
  a raider is held at a sheltered door; DOOR GAVE WAY fires after
  held_secs = 10.03 every time (4 of 4 on the P1b boot). A door holds for
  exactly ten seconds and then opens: the mechanism is a timer, not a
  strength. "Fights >= gave-ways" is true by construction and says
  nothing.
- OUTCOME, NOT RESPONSE: downed 0 on every boot; nobody was hurt. No
  raider-fate witness exists (downed, dead, fled), so whether the militia
  won or the raiders left is not on record.

## Disposition

Alarm, shelter and muster PASS on three replicates; running-to-zero
PASS; downed PASS. Doors FAIL by design (a fixed ten-second hold). NOT
scored: civilians indoors (until D1b), muster against guards awake. NOT
built: a raider-fate witness; a door strength. JUDGEMENT FOR BEN: how
long should a barred door hold against two raiders, and should it depend
on the door (a house door versus a gate) rather than a timer? That is a
number of taste, not measurement.
