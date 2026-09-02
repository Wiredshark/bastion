# RESULTS — night hunger met at home (N1) and sleep metabolism (N2)

Read 2026-09-02 04:45 against PREREG-night-hunger-met-at-home.md.
Arms: flat arm b1 on ef8a172174 (two nights), flat arm b2 on 4a6baabe3c
(one night; same night rows). Census with the hunger distribution
(below_interrupt < 0.2, starving < 0.05); roster 49-51.

## The dawn, before and after

| run                          | hour ~2 fed | hour ~4-5 fed | dawn starving | night meals at home |
|------------------------------|-------------|---------------|---------------|---------------------|
| b1 f5f18c6734 (no night rows)| 13/50       | 10/50         | (no field)    | (no mechanism)      |
| b2 8e9ca2c2fd (no night rows)| 14/50       | 8/50          | (no field)    | (no mechanism)      |
| b1 ef8a172174 night 1        | 29/49       | 24/49         | 7             | 4                   |
| b1 ef8a172174 night 2        | 23/51       | --            | 10            | --                  |
| b2 4a6baabe3c night 1        | 19/50       | 33/50 (dawn)  | 11            | 10                  |

## Disposition

- DIRECTION: PASS. The dawn moved from 8-14 fed to 19-33 fed on two arms
  (two to three times), the night meal fires (4 and 10 meals in one
  night), and nobody walked out under the curfew (no store meals in the
  Sleep block; the witness names the household pile).
- PRE-REGISTERED LINE: FAIL. `starving` at dawn was 7, 10 and 11, not 0;
  `below_interrupt` at hours 2-4 was 10-19 of 49-51 (20-38%), not <= 20%.
- CAUSE, from the same census: the private shelves carry 27-118 units
  over 68 houses (b1 day 2: 27 units, max 5 on any shelf), so most
  colonists have nothing at home and the curfew rightly holds them. That
  is the pre-registration's named failure branch verbatim: "the household
  shelves hold no food ... the meal has to happen BEFORE curfew".
- NEXT: Row N3, supper before curfew (PREREG-supper-before-curfew.md),
  staged as 563f67b2f7; b1 restarted on it at 04:45 for the read. The
  "take rations home" haul-side mechanism stays in reserve behind it.

## Not evidenced

- N2 alone (sleep metabolism without the night meal): not isolated; the
  arms ran both. The dawn improvement cannot be attributed between them.
- Night meals on Ben's town (real houses, real shelves).
