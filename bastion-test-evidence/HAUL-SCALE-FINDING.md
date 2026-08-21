# ★ HAULING GETS WORSE AS THE COLONY GROWS — measured, 2026-08-21

Found while closing item 40. Registered as the next row; **not diagnosed to a
mechanism yet**, and deliberately so.

## The measurement

Three arms, sequential on a quiet host, all attested fresh, differing only in
colonist count.

| colonists | haul deposits | mean working | working share | claim refusals: `materials` |
|---|---|---|---|---|
| 8 | **62** | 2.03 | 25% | — |
| 32 | **37** | 2.77 | 9% | **300 of 390** |

**Four times the population delivered forty percent fewer loads.**

## Why this is the binding constraint, not a symptom

At 32 colonists the colony is not short of anything except delivery:

- **It has the material.** `mine generator STATE … supply=224` — two hundred
  and twenty-four stone on hand.
- **It has the labour.** 30 of 32 colonists idle, and they are *unemployed*,
  not stuck (`stuck`≈0.6 at every size — this is not the pathfinding defect).
- **It has the work.** 32 bed jobs were placed and all 32 eventually completed.
- **And 300 of 390 claim considerations refuse for `materials`** — jobs waiting
  on stone that is sitting in a pile a short walk away.

So the colony's usable labour is capped near three people at any size, because
material cannot reach the work fast enough, and the delivery rate *falls* as
the colony grows.

## What I have NOT established

Which link fails. Candidates, none yet tested:

1. **Haul job generation is quota-capped per firing** on a fixed cadence, like
   the mine generator was before F13 — more colonists would not produce more
   haul jobs.
2. **Reservation contention**: `reserve(item, amount)` is exclusive, so 30
   colonists contending for the same piles may serialise or thrash, which would
   explain deliveries going *down* rather than merely failing to rise.
3. **Stockpile membership**: material counted as `supply` may not sit inside a
   stockpile region, and the claim gate reads `stockpile_has_material`. The two
   predicates could disagree at scale.

Candidate 2 is the only one that naturally predicts a *decrease*, which makes
it the first to test — but "naturally predicts" is a story, and this project has
spent a whole session learning that a fitting story is the weakest evidence.
The next step is an instrument that distinguishes them, not a fix.

## Why it matters beyond item 40

**Ben's NPC-conversion charter** takes a colony from 8 to ~30 people in one
step. On tonight's numbers that would buy almost no additional labour and would
make hauling worse. Worth knowing before that arc starts, and it is flagged in
`DECISIONS-FOR-BEN.md` for exactly that reason.
