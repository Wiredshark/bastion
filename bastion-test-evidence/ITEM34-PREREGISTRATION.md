# Item 34 (Raids scaling with wealth + thievery #97) — PRE-REGISTRATION

**Substrate, read not assumed:** hostiles exist and reach colonists (item
13's flee branches, guard machinery item 14); the #97 ambient-loot gate is
BUILT with witnesses (`ambient-loot-disabled` refusal + timing-race
witness) — "gate lift is line one" per the roadmap; chronicle wiring for
theft events pre-built per #100. Colony WEALTH is measurable today: the
stockpile census (items in zones) + `colony_food_stock` are live producers.

## Build shape (v1)

1. **The wealth signal**: one producer — stockpiled item count (the census
   fold, weighted flat in v1) — sampled at a slow cadence into a
   `colony_wealth` board value. Same-source law: raid scaling reads THIS
   number, no second census.
2. **Raid pressure scales on it**: hostile spawn cadence/count near the
   colony derives from wealth bands (data table, not code branches).
   Deterministic: banded thresholds + tick-keyed spawn draws.
3. **Thievery (#97 line one)**: lift the ambient-loot gate FOR RAIDERS
   ONLY — a raider that reaches an unguarded stockpile picks up (steals)
   one item and leaves (the flee drive re-used with cargo). The existing
   gate witnesses flip to per-actor.
4. **Witnesses**: wealth sample (value, band), raid spawn (count, band
   that priced it), theft (item, thief, from-zone) — chronicle-carried
   (#100's wiring).

## BARS

1. Wealth A/B (poor vs seeded-rich, same seed): the rich arm draws
   measurably more raid pressure; both arms' bands + spawn counts stated.
2. A theft completes live: item leaves the zone in a raider's possession,
   witnessed + chronicled; colony census decrements (conservation across
   the theft — the item MOVED, not vanished).
3. Guard interaction: a Fight-mode guard within range interrupts the
   theft (item 14's machinery consumed, not reimplemented).
4. Twin determinism.

VOID branches: hostiles never reach the stockpile (pathing — the item
14/15 fixture lessons apply); the gate lift leaks to NON-raiders (the
regression witness: ambient-loot-disabled must still fire for ambient
NPCs); wealth bands never differ across arms (seed the rich arm harder,
report the values).
