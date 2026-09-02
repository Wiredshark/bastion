# RESULTS — a deposit run spreads its own bag (S8): PASSED

Read 2026-09-02 11:28 against PREREG-deposit-run-split.md. Arm b1 on
b8d2733d2b (S8 in), game day 1 census; raids on.

| pair / arm (day 1)         | barn units | heaviest cell | cells used | max cell def        |
|----------------------------|-----------:|--------------:|-----------:|---------------------|
| c3b30ac4db b1 (before S7)  | 539        | 225           | 29         | mushroom x127 + ... |
| c3b30ac4db b2 (before S7)  | --         | 67-69         | --         | --                  |
| 21adf2f5df b1 (S7 in)      | 358        | 157           | 22         | mushroom x79 + ...  |
| b8d2733d2b b1 (S7 + S8)    | 394        | 58            | 51         | mushroom x54        |

The other general stores on the S8 day: 132 units / heaviest 34 / 18
cells; 60 / 12 / 13; 140 / 46 / 26. Town-wide heaviest cell 58 (the
STORAGE SUMMARY's general_max_cell: 225 -> 141 -> 157 -> 58 across the
four reads).

- PASS on the pre-registered bar: the barn's heaviest cell <= 64 (58)
  with cells_used >= 20 (51).
- PASS on the chunking: every forage deposit of >= 32 units named >= 2
  cells (33 -> 3 cells; 55 -> 5; 54 -> 4; 52 -> 8; 45 -> 3; 37 -> 3;
  34 -> 8). 131 forage deposits and 22 haul deposits on the day, all
  with cells; the identity switch was off.
- The day's eat census: 51 eat jobs, 49 meals (96%), 48 targets shunned
  -- unchanged in kind from the P1b day (56 / 47 / 40); the split does
  not touch the walker.
- The residual 54-unit stack on one cell is several runs' chunks landing
  on the same emptiest cell across the day (the recent-drops memory is
  twenty seconds), not one drop; it sits under the bar.

## Disposition

PASSED. S7 (the re-aim remembers recent drops) was necessary for the
hauls and insufficient for the runs; S8 finished the row. NOT built: a
per-cell cap across the whole day (the 54-unit stack would need it; the
bar does not). The S7 read's FAIL branch named the producer correctly:
the forage deposit, not the haul.
