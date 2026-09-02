# PREREG — an adopted town has been living here: fields at every stage, people not all hungry at once

Written 2026-09-02 01:30, before the build. Source: flat arms b1 and b2
(26e0852dae / 5836a476ca) under the 4-game-day grow cycle (a3785102a6).

## What the arms showed (two replicates, same shape)

- `fed` (hunger > 0.3) is 48-49 of 49 until tick ~24,000 (0.45 day), then
  drops to 1 of 49 in one census step -- a CLIFF, not a decline: everyone
  was adopted at the same hunger and decays in lockstep (1.0 -> 0.3 in
  ~787 s = ~23,600 ticks at 30 TPS, exactly where the cliff sits).
- After the cliff, fed oscillates 9-38 of 49-51 for the next two days
  (food_stock 25 in the general stores; sown=51, MATURE=0 at 2.1 days).
  With a real grow cycle the first harvest is ~day 4; the seeded 64 food
  and forage do not bridge 49 mouths for four days.

## Mechanisms (adoption realism; pure, deterministic by cell/uid hash)

A1 STAGGERED FIELDS. At adoption, every colony-managed crop cell gets a
   growth stage drawn deterministically from a hash of its cell (uniform
   over SOWN..=MAX), not "just sown": a village that has been living here
   has fields at every stage, so harvests roll from the first day. Prior
   art: Banished (starting fields partly grown), Manor Lords (fields at
   mixed growth at start), RimWorld (map-gen crops at random growth).
A2 STAGGERED HUNGER. At adoption, each colonist's hunger (and rest) starts
   at a deterministic value in 0.55..=1.0 from a hash of their uid, so
   meals spread across the day instead of one town-wide cliff. Prior art:
   every colony sim staggers initial needs (RimWorld pawn gen).
Identity: the founding fixture (arena / env spawn) is unchanged unless the
same code path adopts; `BASTION_NO_ADOPT_STAGGER` restores both.

## Pre-registered pass / fail (flat arm, two day boundaries)

- A1 PASS: first `crop MATURE` within 0.3 game day of adoption and MATURE
  events on every subsequent day line; stage histogram at adoption spans
  >= 10 distinct stages. FAIL: MATURE still 0 before day 3.
- A2 PASS: no census step where `fed` drops by more than 30% of the roster
  in one 300-tick sample during the first day; the minimum fed over day 1
  >= 60% of the roster. FAIL: the cliff persists (then the cause is not
  the identical start but the decay rate against the larder).
- Falsifier of the whole row: if fed still averages < 60% on days 2-4
  with staggered fields, the food ECONOMY (par, harvest yield, forage) is
  the row, not adoption's starting state -- and the honest next step is
  the starting larder as a design number for Ben.
