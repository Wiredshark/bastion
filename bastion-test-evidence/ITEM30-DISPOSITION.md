# Item 30 (Stockpile zones by type) — DISPOSITION: bars 2+4 **PASS**, bar 1 **split**, bar 3 banked

Leg: `zones` (chain10, 2026-08-20). Shape: preset's untyped stockpile
CANCELLED, one FoodStore painted (16392–16396²), loose food + stones given
and dropped outside every zone — each item's route attributable.

**Bar 2 (matching items route preferentially): PASS.** All FIVE haul jobs
minted in the window carried `destination: 3` — the registered FoodStore
(`typed stockpile zone registered zone=3 kind=FoodStore`, the paint→typed
registration witnessed live). Wheat came to rest INSIDE the region
(16396.9, 16396.1); a mushroom was still en-route at leg end (16389 — jobs
outlive short windows; the destination field is the routing evidence).

**Bar 1 (typed refusal) — SPLIT.** Behavioral half PASS: the stones
(16389.5, 16389.5) never entered the FoodStore and no haul was minted for
them — the exclusion held. Witness half VOID-BY-INSTRUMENT: the refusal
line never fired because its throttle (`% (N*40) == 3`) is unsatisfiable
under the enclosing generator's `% N == 7` gate — **a witness that could
not fire**, caught precisely because the behavior it should have witnessed
happened silently. Offset fixed to ==7 (254918dec5); the witness re-runs
on the next zones leg.

**Bar 4 (untyped path unchanged): PASS by construction** — untyped zones
take the `None => true` filter arm (the pre-item-30 predicate verbatim),
and no typed zone exists in any banked corpus leg.

**Bar 3 (twin determinism): banked** — rides the standing twin queue with
item 22's.

Design note: cheese/potion (non-FOOD_DEFS loadout junk) sat where dropped —
no zone accepts them and no haul was minted; when more classes get typed
stores (item 26's chains will want a materials store), the class map is the
only edit (the selector is already class-generic).

## Bar 1 completed (chain14 zones leg, 2026-08-20): **PASS both halves**

The fixed-offset witness fired throttled all leg: `typed-zone refusal — no
store accepts this item class` for `wheat_seeds` (×27 windows) and `stones`
(×13) — and NEVER for wheat proper (the one FOOD_DEFS item in play), so the
filter discriminates exactly on class. Behavior matched: wheat came to rest
INSIDE the FoodStore (16393.9, 16393.4); stones ended OUTSIDE its boundary;
all five minted hauls carried `destination: 3` (the typed store). In-store
cheese/potion are driver-drop artifacts (the player stood in the store for
dropall), not routing — named so the item table reads honestly.

**Item 30 standing: bars 1, 2, 4 PASS; bar 3 (twin determinism) rides the
standing twin queue.**
