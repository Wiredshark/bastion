# PREREG — night hunger is met at home, and a sleeper burns less

Written 2026-09-02 02:00, before the build. Source: flat arm b2 (staged pair
5836a476ca), first game day, read from the EXPERIENCE census, the "ate"
lines and the "Personal entry releases the held work job" lines.

## What the arm showed

- `fed` (hunger > 0.3) fell 49 -> 1 at hour 19 and recovered to ~30 by
  hour 22; fell again 34 -> 13 at hours 2-3 and recovered from hour 4-7.
- The 33 logged meals of the day landed at hunger_before 0.0-0.25: seven
  at ~0.0. Of 49 Personal-drive entries (a hungry colonist dropping work to
  eat), 26 never led to a meal in the window; the never-fed went straight
  back to claiming work.
- The cause is the CURFEW: in the Sleep block the eat scan skips every
  food pile (`NEED-SKIP curfew -- night hunger holds until dawn`, a diag-only
  line, so the null had no witness). The curfew exists for a real reason
  (the night massacre: hungry stragglers walked out to the store and into
  nocturnal hunters). Its cost: a raw meal restores 0.5, a night of ~600
  sim-seconds burns 0.53, so anyone who ate supper at hour 20-22 is at or
  under the interrupt by hour 2 and at zero by dawn.
- Meanwhile 557 food units sat in the general stores. Nothing was scarce;
  the night was.

## Mechanisms (pure; deterministic)

N1 NIGHT MEAL AT HOME. In the Sleep block a hungry colonist may still eat
   from a pile INSIDE ITS OWN HOUSE (the household shelf -- Ben: "it's fine
   they store in their house"), or from its pack. The candidate scan is
   restricted to those piles at night, so nobody walks out; the curfew
   keeps its purpose. Witness `NIGHT MEAL AT HOME`. Prior art: every colony
   sim lets pawns eat at home at night (RimWorld eats from the nearest
   food including the bedroom shelf; Banished houses hold food).
N2 SLEEP METABOLISM. Hunger and rest-independent needs decay at half rate
   while the colonist is in its Sleep block (`SLEEP_METABOLISM = 0.5`):
   a sleeper burns less. Prior art: The Sims (hunger decays slower asleep),
   Dwarf Fortress (sleeping dwarves' hunger counters slow).
Identity: `BASTION_NO_NIGHT_LARDER` (N1 off), `BASTION_NO_SLEEP_METABOLISM`
(N2 off). Neither touches the day; neither moves a colonist outdoors at
night.

Instrument (already queued): the census carries hunger_mean, hunger_min,
below_interrupt (< 0.2) and starving (< 0.05) beside fed.

## Pre-registered pass / fail (flat arm, days 2-3, dawn samples hour 3-5)

- PASS: `starving` = 0 at every census sample from day 2 on; `below_
  interrupt` at hours 2-4 <= 20% of the roster (b2 day 1: ~70%); no meal
  logged at hunger_before < 0.05 after day 1; `downed` stays 0 (the curfew
  still holds -- no colonist eats at the general store during Sleep, which
  the `NIGHT MEAL AT HOME` witness distinguishes from a store meal).
- FAIL: starving > 0 at dawn on day 2 with N1+N2 on -> the household shelf
  holds no food (the hauling never stocks homes) and the row moves to
  "take rations home before curfew", a haul-side mechanism.
- Falsifier of N2 alone: if below_interrupt at dawn is unchanged with N1
  off and N2 on, the night burn is not the binding term; the meal size or
  the supper timing is.
