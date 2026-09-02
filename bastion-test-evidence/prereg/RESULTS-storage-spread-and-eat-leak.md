# RESULTS — the spread at floor level (S5, S5b) and the eat leak (eat census, E1, E2)

Read 2026-09-02 06:45. Arms: flat arm b1 on 6ee29a1817 (food on crate
tops), b1 on 07c3622768 (S5: spread switched off by mistake, food at floor
level on the centre cell -- the control), b2 on a900163959 (S5b: spread at
floor level). Roster 48-50. All from the EAT CENSUS, FETCH lines, STORAGE
CENSUS and the "need preempt" lines.

## The eat leak, three runs

| run                          | eat minted | meals | meals/minted | EatFrom stall expiries | condemned cells |
|------------------------------|-----------:|------:|-------------:|-----------------------:|----------------:|
| b1 6ee29a1817 (crate tops)   | 99         | 46    | 46%          | 70                     | 2 (the mushroom cell) |
| b1 07c3622768 (floor, unspread) | 62      | 45    | 73%          | 33                     | 2 (incl. the centre pile) |
| b2 a900163959 (floor, spread)| 81         | 40    | 49%          | 54                     | 1                |

Skip reasons were the same shape on all three (no_food_found 0, curfew 0;
cooldown 7-10k, already_on_need_job 15-17k, drive_not_personal 5.5-6.7k
passes): the scan always found food; the trip failed.

## S5 / S5b — the spread at floor level

- S5 (07c3622768): the filter measured against the zone's min.z, which
  lies below the barn's built floor, so every floor cell was rejected and
  all four founding deliveries landed on the centre cell (general_max_cell
  198 on day 1). FAIL of the mechanism, caught on its first witness line;
  fixed as S5b (reference = the centre cell's height).
- S5b (a900163959): deliveries on four cells at z 181-182 (was z 183 on
  crate tops); day-1 STORAGE CENSUS: mushrooms at most 14 on a cell, meat
  15, lettuce 12; the heaviest cell (81) is stones, not food. PASS for the
  spread's own claim. Not the eat leak's cure (49% meals).

## Where the trips die

- Crate-top run: targets at item z 184 over a floor at 181; ARRIVE_DIST
  is 2.5 in three dimensions; eaters under the pile never arrived.
- Control run: 15 stalls at z 183 and 7 at z 186 -- walkers on structures
  (the climbing hazard from Ben's log), 30-70 blocks from the barn.
- S5b run: 41 of 54 stalls at ONE spot (7748, 6328, 181), each 25 blocks
  short of one pile at (7768, 6340, 183) inside the fourth general store,
  the store whose cell carried the unexplained 143-unit stack on earlier
  runs. The town cannot enter that store; deposits into it never drain;
  the eat chooser (nearest pile by straight line) re-picks it every time.

## Disposition

- The 15-second fetch stall expires a meal trip before the wedge/strike/
  self-rescue machinery can recover the walker: row E1 (an EatFrom fetch
  expires only at the 90 s budget), staged next.
- Nothing marks an unreachable target for the chooser until N exhausts
  condemn its exact cell: row E2 (a stalled target's cell gets a goal
  verdict for six game hours), staged behind E1.
- A store the town cannot enter is its own row (S6), recorded, not built.
- The supper row's verdict waits on E1+E2: its preempts fire (22 of 49 on
  two evenings) and the meals were lost downstream.

## Not evidenced

- E1 and E2 live (both arms restart on the E2 pair).
- Whether the fourth store is enterable on Ben's terrain (a flat-arm
  adoption artefact or a general one).
