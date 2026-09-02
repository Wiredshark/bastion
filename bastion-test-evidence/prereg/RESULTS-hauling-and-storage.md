# RESULTS — hauler ceiling fallback (H2), haul gate with no haulers (M3b), founding stock spread (S4), storage heaviest cell

Read 2026-09-02 05:20. Arms: flat arm b2 on 4a6baabe3c (zones stage: H2
and the max-cell instrument), flat arm b1 on 563f67b2f7 (supper stage,
which carries M3b and S4).

## H2 — the ceiling falls back to the neediest lane: PASS (criterion 1)

| run          | roster | cap | named Haul | after demotion | neediest |
|--------------|--------|-----|------------|----------------|----------|
| b2 zones d2  | 50     | 12  | 15         | 12             | Mine     |
| b1 supper d1 | 49     | 12  | 20         | 12             | Build    |

Before the row (PREREG-ceiling-neediest-lane.md): 22 -> 21 and 17 -> 14.
Criterion 2 (the receiving lane works the next day) is not yet read: b2
was restarted before its day 3 and b1 before its day 2. Open.

## M3b — the gate opens when nobody is a hauler: PASS

b1 supper stage day 1: haul_claims_gated = 0, hauls = 81 (in the day-1
range of 50-120 seen before the gate). Before the fix: 44,466 refusals
and 13 hauls (b1 on ef8a172174), 266,249 and 21 (b2 on 4a6baabe3c).

## S4 — the founding stock spreads: PASS

b1 supper stage: the four founding deliveries landed on four cells
((7698,6446), (7672,6426), (7673,6426), (7674,6426)); b2 guard stage
likewise (four cells). Day-1 STORAGE SUMMARY general_max_cell = 64 (the
largest single delivery, the pre-registered bound), against 102 / 113 /
116 / 125 / 143 on the five earlier runs.

## Storage heaviest cell (instrument): the second stack is not the delivery

b2 zones stage day 2: zone 74 held 166 units, 143 of them on ONE cell at
(7769, 6343), max_at_centre = false. That is not the founding pile (which
sat at the centre on that stage). A second producer stacks; the follow-on
instrument (max_cell_def, staged next) names the item on that cell.

## Not evidenced

- H2 criterion 2 (the demoted lane works next day).
- All three on Ben's own town.
