# RESULTS — store closing (S6, S6b): FAILED twice, made opt-in (S6c)

Read 2026-09-02 08:50. Arms: flat arm b1 and b2 on 06f9a5cb91 (S6,
zone-keyed strikes), flat arm b2 on ecef86ff8f (S6b, spot-keyed strikes).

## What closed

| pair / arm        | stores closed on day 1                          | eat jobs / meals | EatFrom expiries | evening starving | general units |
|-------------------|-------------------------------------------------|-----------------:|-----------------:|-----------------:|--------------:|
| S6 b1             | zone 75 (the unenterable store), 51, 26 (BARN)  | 46 / 42 (91%)    | 18               | 2-6              | 593           |
| S6 b2             | zone 75, 51, 25 (BARN)                          | 34 / 50          | 7                | 1-2              | 326           |
| S6b b2            | zone 57 (unenterable), 51, 24 (BARN, two cells) | 53 / 39 (74%)    | 36               | 3-10             | 636           |
| E1+E2 only, b2    | none (no closing)                               | 50 / 43 (86%)    | 21               | 1-5              | 746           |

- S6 closed the barn because any busy store collects three stalled trips
  a day from walkers wedged elsewhere (the pre-registration's falsifier).
- S6b keyed the strike by the stall spot (the unenterable store's
  signature: 41 stalls on one 4-block spot) and STILL closed the barn:
  the barn has a jam spot of its own, (7636, 6352), where 8 eaters stalled
  15 blocks west of its food. The rule cannot tell a blocked door from a
  bad approach, and the store that holds the food is the one it hurts.
- With the barn closed, deposits went elsewhere and the evening starving
  count rose to 10 on the S6b arm -- the closure did more harm than the
  unenterable store it was built for.

## Disposition

Closing is OPT-IN (BASTION_STORE_CLOSE=1) from S6c; the strike rule stays
as a witness ("STORE WOULD CLOSE" with zone, cell and spot) so the case it
is right for can be recognised later. The per-cell shun (E2, six game
hours) is the working mechanism against the unenterable store: it made
the best measured day (86% meals, 21 expiries, starving 1-5). Two
rejected variants: a finer spot grain (the barn's own jam spot still
counts), and exempting the store with the most food (hides the class).

## What it points at

Both jam spots are approaches, not doors: (7748, 6328) sits 25 blocks
short of the fourth store, (7636, 6352) 15 blocks west of the barn's
food; earlier stalls sat on structures at z 183-186. That is the walker
row (pathing: doors, paths, climbing priced), and it needs a looking
sweep with a client -- counters cannot say what the walker meets there.

## S6c read (b2 on c3b30ac4db, day 1, 09:09): closing opt-in, shun alone

eat jobs 47, meals 46 (98%); EatFrom expiries 16; STORE CLOSED 0; STORE
WOULD CLOSE 8, all on zone 39 (the unenterable store) but spread over
four different 4-block spots -- the spot-keyed rule would not even have
fired here, which is one more reason it is a witness and not a switch.
Stores 576 units, heaviest cell 69; evening starving 0-3. The best day
measured so far: this is the pair shipped to lab-bin at 09:08.
