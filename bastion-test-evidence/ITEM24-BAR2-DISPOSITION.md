# Item 24 bar 2 (the annual cycle, unpinned) — DISPOSITION: **PASS**

Leg: `year` (chain9, 2026-08-20) — PIT_DAY_LENGTH=0.5 (delivered-value
witness: `day_length: 0.5` read back from the FILE), 163k ticks ≈ 161 game
days, 484 food-stock samples, unpinned seasons (no BASTION_PIN_SEASON).

**The bar — food stock rises through the growing seasons, falls through
Winter — measured on per-season means (900 ticks/day, 160-day year):**

| season | mean stock | n |
|---|---|---|
| Spring | 224.0 | 124 |
| Summer | 404.7 | 120 |
| Autumn | 569.0 | 120 |
| **Winter** | **0.0** | 120 |

Growth engine's own witness agrees: stage-ups Spring 4,429 / Summer 6,175 /
Autumn 3,430 / **Winter 0** — the pause is total, unpinned, for a full
in-cycle winter. The colony survived the starved winter (0 deaths, health
1.0 throughout, hunger floored at 0.0) and the new Spring released the
paused crops in a burst (stock 1,935 within days of the thaw) — the
accumulate → deplete → dead-stop → boom shape is the annual cycle the
design wanted, visible in one unpinned leg.

**Bar-3 evidence banked from the same leg:** `COLONY TERMINAL (sentinel S1,
log-only) tick=110400 consecutive_zero_samples=10` — winter starvation
reaches the colony-terminal path's own trigger ORGANICALLY (mid-winter).
The registered bar-3 fixture (autumn founding, seed 0, sentinel armed to
act) remains a separate leg; the mechanism it tests is now witnessed live.

**Honest notes:** the "fall" is a crash at winter's onset (stores eaten +
growth stopped), not a gradual decline — the shape is starker than the bar's
wording. Sample at tick 145,200 reads 0 (teardown edge; excluded from
means). Individual starvation does NOT damage health (hunger=0, health=1.0
for 40 days) — mortality is colony-level by design today; noted for item 35
(injuries) and item 8's survival-loop lineage.
