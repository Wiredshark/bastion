# PREREG — a famine closes the gate (F1)

Registered 2026-09-02 13:50, before the binary exists. Ben's ruling, live
at 13:40: asked whether a famine should close the settler gate, "yes".

## What stood

The settler gate (immigration_verdict) read: enabled, not already today,
drive Expand, roster under target_pop, a vacant house. On the
compressed-year arm (b2, 0a1772ddb8) it fired every day from day 2 to
day 7 ("a house stands empty", 10-12 vacant of 58) while the night
census read 24 -> 43 starving of 55-59 and the eat census 313-360 eat
jobs for 71-79 meals. The roster grew 49 -> 59 through the famine.

## Mechanism

famine = drawable food stock < FAMINE_DAYS_OF_FOOD (2.0) x roster x 3.2
raw units a day (the same producer and frame as the food par); an empty
town is never in famine. The verdict refuses with deciding "famine"
before drive, target and vacancy; a SETTLER GATE CLOSED line carries
roster, stock and days_of_food. BASTION_NO_FAMINE_GATE restores the
beds-only gate. Pinned (the verdict order and the threshold's edges).
The two-day line is my number until Ben names one.

## Pre-registered outcomes

- Instrument validation: on any arm-day where the YEAR CENSUS's
  days_of_food reads under 2.0 at the day line, the same day's HOUSING
  GROWTH line must read deciding="famine" and a SETTLER GATE CLOSED line
  must exist; where days_of_food >= 2.0 the deciding must not be
  "famine".
- PASS (the next compressed-year run, days 0-8): the roster does not
  grow on any day whose day line reads days_of_food < 2.0, and resumes
  (a "A SETTLER IS SENT FOR" line) on the first day at or above it with
  a vacant house.
- FAIL branches: the roster grows on a famine day -> the gate has
  another entry (pending_immigrants queued before the famine deliver
  after it: the queue must be checked, not the verdict); days_of_food
  swings across 2.0 daily and the gate flaps -> the threshold needs a
  hysteresis (open at 3, close at 2); the town never leaves famine on the
  fixed pair -> the economy row, not this gate.
- NOT evidenced live yet.
