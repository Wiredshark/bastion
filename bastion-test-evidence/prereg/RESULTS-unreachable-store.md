# RESULTS — an unreachable store is withdrawn on the search's word (W1): PASSED on its first exercised day

Read 2026-09-02 12:03 (W1 boot), 12:35 (W2b boot) and 13:12 (P1c boot)
against PREREG-unreachable-store.md. Arm b1 (raids on), game day 0 of
each boot. The terrace store is the raised, walled yard whose region
contains (7672, 6426); its zone id changes per boot (45, 44, 38, 44).

| boot / pair              | FETCH UNREACHABLE | STORE UNREACHABLE | store named            | probes at its cells after | deposits into it after |
|--------------------------|------------------:|------------------:|------------------------|--------------------------:|-----------------------:|
| 35cd156e00 (W1)          | 0                 | 0                 | (not targeted that day) | --                       | --                     |
| a1441bcf4e (W2b)         | 1                 | 0                 | zone 38, the terrace   | --                        | --                     |
| 4ea213f029 (P1c)         | 3                 | 1 at 17:06        | zone 44, the terrace   | 0                         | 0 (7 forage before)    |

- Instrument validation: every FETCH UNREACHABLE line named the terrace
  store's corner cell (7672, 6426, 182) with kind EatFrom; on the W1 boot
  none fired and no probe held path_state None, so the day was
  unexercised, as the pre-registration allowed.
- PASS on the P1c boot: the third no-path fetch in the window withdrew
  zone 44 at 17:06 (game day 0, mid-afternoon); after it no probe aimed
  at the store's cells and no deposit went into it (seven forage runs had
  before). The barn and the store at (7776-7780, 6356) were not named on
  any boot; the withdrawal is STORE_CLOSE_TICKS (one game day) and
  reopens.
- The eat-census bar: the P1c boot's day-1 EAT CENSUS read 45 eat jobs,
  46 meals, 18 stalls tolerated, 34 targets shunned (the bar: meals over
  jobs not below 47 / 56); evening starving 0-2. PASS.
- The fetches that reached the "no path" verdict did so at tier Small
  (500 iterations): a raised terrace with no ground way in is decided
  cheaply; the withdrawal spares the walkers the 15 s stall each and the
  choosers the six-hour cell shun that only moved them to the next cell.

## Disposition

PASSED (one exercised day, one partial, one unexercised). NOT built:
the (Longest, Exhausted) extension (no top-tier exhausted probe has been
seen yet; W2c's escalation will produce them if the ground route does
not exist). Open: whether the terrace store should be reachable (a
ramp or door is a town-layout row, not a chooser row) and whether the
seven forage runs that reached it before the withdrawal came in over
the wall (a climb the design allows for job legs).
