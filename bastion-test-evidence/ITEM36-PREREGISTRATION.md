# Item 36 (Death that matters) — PRE-REGISTRATION

**Substrate, read not assumed:** death EXISTS and already half-matters —
`Death: (-0.15, 172800.0)` is live in the thought table (bastion_thoughts.ron)
weighing on colonists named as actors; the chronicle records it; Health/is_dead
runs colonist mortality; the bed-slot sweep releases a dead sleeper's slot.
**Missing: the colony-side CONSEQUENCES beyond mood** — the body, the
belongings, the roster hole, the memory surface.

## Build shape (v1)

1. **The death is RECORDED as a colony event with actors** (kill source +
   witnesses within R — the item-22 pair scan's shape reused for "who saw
   it"): witnesses carry the Death thought (today only named actors do —
   verify who the recorder names; the gap between "died" and "was seen
   dying" is the design).
2. **The body and the belongings**: on colonist death, inventory drops at
   the death cell (emit_drop reuse, conservation instrument applies) —
   wealth is not deleted, it is now a hauling problem (and later a raid
   target: item 34's wealth signal).
3. **The roster hole is visible**: the colony census logs the death
   (population, cause) — the dashboard's population number must move.
4. **Chronicle surfacing**: the inspect payload's chronicle already carries
   events; assert a Death entry appears for the colony scope.

## BARS

1. A planted death (cave-in lethal or flee-health plant escalated) produces:
   the chronicle Death record, mood drops on witnesses (MOODX shows the
   Death thought), belongings on the ground at the cell, population
   decrement in the census — ALL four from ONE event, each witnessed.
2. The null: a no-death control leg shows zero Death records/thoughts
   (couldn't-happen witness: the health census at 1.0 all leg).
3. Conservation: dropped belongings equal the dead colonist's inventory
   (count-in vs count-out, T1.13's discipline).
4. Twin determinism.

VOID branches: the death plant doesn't kill (health floor/guard absorbs —
report the absorber); items despawn before counting (name the sweep);
the thought table entry missing at runtime (asset load check).
