# PREREG — supper before curfew: nobody goes to bed on a hunger that will not last the night

Written 2026-09-02 04:10, before the build. Source: flat arm b1 on the
night stage ef8a172174 (N1 night meal at home, N2 sleep metabolism),
EXPERIENCE census with the hunger distribution.

## What the night stage showed (first night)

| tick   | hour | fed/50 | below_interrupt | starving |
|--------|------|--------|-----------------|----------|
| 42,000 | ~2   | 29     | 10              | 3        |
| 45,000 | ~4   | 24     | 14              | 6        |
| 48,000 | ~5   | 19     | 19              | 7        |
| 54,000 | ~7   | 30     | 13              | 7        |

Night meals at home: 4 all night. The previous stage's dawn was 10-13 of
50 fed, so N1+N2 moved the dawn from 10 to 19-30 fed -- and still 7
starving. N1 fires only when the colonist's own house holds food; the
household shelves carry 85 units over 68 houses (a little over one unit
each, mostly not food), so most colonists have nothing at home and the
curfew holds them. That is the pre-registration's named failure branch:
the shelves hold no food, and the meal has to happen BEFORE curfew.

## Mechanism (pure, deterministic)

SUPPER LINE. In the two hours before a colonist's own Sleep block (hours
20-21 on the default schedule; the night watch's own evening), the hunger
interrupt is raised to `SUPPER_LINE = 0.6`: a colonist at or under 0.6
goes to eat now, at the store, while the town is still awake, instead of
carrying 0.5 into an 8-hour night that burns 0.27 (with N2) and crossing
the interrupt at hour 2 with nowhere to go. A colonist above 0.6 does not
eat (identity); a raw meal restores 0.5, so the supper line and the night
burn leave everyone above the interrupt at dawn by construction:
0.6 - 0.27 = 0.33 > 0.2. Witness `SUPPER` at the preempt with the hour and
hunger. Identity: `BASTION_NO_SUPPER`. Prior art: RimWorld (pawns eat
before sleeping when hungry), The Sims (dinner as a scheduled meal),
Dwarf Fortress (meals by schedule).

## Pre-registered pass / fail (flat arm, night 2 onward, dawn samples hours 2-7)

- PASS: `starving` = 0 at every dawn sample from the second night;
  `below_interrupt` at dawn <= 20% of the roster (b1 now: 26-38%);
  SUPPER witnesses on the evening of day 1 >= 40% of the roster.
- FAIL: starving > 0 at dawn with SUPPER witnesses >= 40% -> the supper
  was ordered but not served in two hours (the serving throughput: eat
  scan cadence, one pile, travel) and THAT is the row; or SUPPER
  witnesses < 20% -> the supper hours are not the hours the colonists are
  awake and idle (schedule frame), and the hour window is the number to
  move.
- Falsifier of the design: if the evening plaza census collapses (supper
  empties the leisure window) the supper is competing with the social
  evening; then the supper should be the FIRST leisure hour, not the last.
