# Scale, root cause: **the colony runs out of work** — and my own farm formula is part of why

The chain is now closed. Three earlier readings were each true of their phase
and none was the root.

## The three phases, from one 33,900-tick leg at 32 colonists

| phase | working | what binds |
|---|---|---|
| tick ~300 | **21 / 32** | nothing — spending 256 seeded materials |
| tick ~2k–8k | falling | **material contention**: 3,125 refusals, all `req="…bastion.wheat"` |
| tick 8k–34k | **4 / 32** | **job starvation**: `materials=0`, `already_claimed=192`, `considered=192` |

That last census is the one that settles it. At steady state the colonists are
**not** materials-blocked — `materials=0`. Every one of 192 considerations
refuses as `already_claimed`: across 32 colonists that is roughly **six jobs on
the whole board**, all taken. Twenty-six people have nothing to claim.

## Why the board empties

Once the founding is complete the colony's standing demand collapses:

- **Beds**: built (32) — that work is finished, permanently.
- **Mining**: demand is `max(job_demand, PAR_STOCK_STONE)`. With beds done and
  the floor met, demand → 0 and the generator correctly goes quiet.
- **Farming**: the only *renewable* work — and it is capped by farm AREA.

`farm plot registered: 1`. One plot, at 32 colonists.

## ★ The formula is mine, and it is the wrong shape

From `founding_plan`, written earlier this session:

```rust
let grow = |base: i32| base + ((n as f32).sqrt() as i32 - 2).max(0);
```

- n=8 → `sqrt=2.83→2`, grow = base + 0
- n=32 → `sqrt=5.66→5`, grow = base + 3

So a colony of 32 gets a farm three blocks wider in each dimension than a
colony of 8 — roughly **2× the area for 4× the people**. Renewable work grows
as √n while population grows as n, so the working *share* must fall. It is
arithmetic, not a defect in the job board.

I chose sqrt scaling for the pantry and farm without deriving it, and wrote at
the time that v1 was "NOT cost-driven". This is the cost.

## What this retracts and what it leaves

- **RETRACTED (again):** "sustained throughput is the binding constraint."
  Throughput binds in the *middle* phase only. At steady state the colony is
  not starved of material — it is starved of **work**.
- **STANDS:** every measurement. 21→4 working, the 3,125 wheat refusals, the
  2–5 unit stockpile, 62→37 haul deposits, 12% mean share.
- **OPEN:** whether the fix is a farm that scales linearly with population, or
  standing work that is not farm-shaped (the par-stock floor already gestures
  at this — it creates work with nothing asking). That is a design question
  about what a large colony *does all day*, and it is Ben's.

## Bearing on the NPC charter, sharpened

Converting a village to ~30 colonists gives you thirty people and **one
village's worth of standing work**. On this evidence they would be idle within
~8,000 ticks. The charter needs the village's *fields* — plural, scaled — not
just its houses and its people.
