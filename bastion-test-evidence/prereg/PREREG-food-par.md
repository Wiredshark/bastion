# PRE-REGISTRATION — the food par does not know how many mouths it feeds
Base: bastion/item29-trade @ 99b40ae2fa. Written before the change exists.

## DEFECT
    pub const TRADE_FOOD_PAR: u32 = 16;          // flat, every town size
    pub(crate) const STONE_PAR_PER_COLONIST: u32 = 4;   // per colonist
`TRADE_FOOD_PAR` is used RAW at all four sites and never multiplied by
population. The trade lane -- the colony's food backstop -- stops trying to
buy once stock reaches 16 units, whether the town holds 2 people or 46. Stone,
the other consumable, already scales per colonist; food is the outlier.

Grounding: hunger decays at 0.000889/sec, so a colonist empties in ~1,125
sim-seconds ~ 15 game hours: about one meal per colonist per game day. A par
of 16 is therefore ~16 colonist-days of buffer for a town of ONE, and about a
QUARTER of a day for the owner's town of 46.

Measured, owner's real session: `TRADE LANE DEAD ... food_stock=0 par=16`
with roster 46, and `fed=0` for sixteen consecutive game days.

## PRIOR ART
Banished, RimWorld and Dwarf Fortress all size food stores against
population, never against an absolute. The mechanism borrowed is per-capita
demand -- and this file already implements it once, for stone, which is the
strongest evidence that food is simply the one that was missed.

## THE CHANGE
`FOOD_PAR_PER_COLONIST`, mirroring `STONE_PAR_PER_COLONIST`, multiplied by
the live colonist count at each of the four use sites. Value 4 -- the same as
stone -- giving roughly four days of buffer per head.

## PASS / FAIL, pre-registered
P1. The witness prints a par that MOVES with roster: a 2-colonist arm and an
    8-colonist arm must not print the same par.
P2. On an arm with no seeded food, `fed` at the LAST samples is >= 50% of
    roster, against a control on the flat par.
P3. Trade missions mint when stock is below the scaled par and not above it.
P4. No regression: `stuck` <= 1, `residual` 0, EMBED WATCH not above the
    post-856132601b baseline.

## WHAT FALSIFIES THIS
- F1. P2 fails and food_stock sits ABOVE the new par -- then the par was
  never the binding constraint and the starvation is on the PRODUCTION side
  (2 farm fields for 46 people), not the trade side. That is a different row
  and this change should be reverted rather than tuned.
- F2. Haul share explodes carrying food nobody eats -- the par is now too
  high and is manufacturing work. Lever is the per-head number.
- F3. The trade lane mints missions it cannot complete (the
  `not_stockpiled` trap) -- a higher par must not spend where a lower one
  refused.

## NOT EVIDENCED / ASSUMPTIONS
- 4 per colonist is chosen for SYMMETRY with stone, not from a measured
  consumption curve. It is a number that is taste, and Ben should overrule it
  if the town hoards. The SHAPE (per-capita, not absolute) is the measured
  part; the coefficient is not.
- Whether trade can actually buy food on a real map at all. The owner's town
  had nothing sellable because its tools were EQUIPPED, not stockpiled, so
  raising the par may change nothing there -- F1 is exactly that outcome.
