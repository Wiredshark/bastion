# RESULTS — a deposit run spreads its own bag (S8): the runs spread; the bar is now held by the founding stack

Read 2026-09-02 11:28 (replicate 1), 12:12 (2) and 12:35 (3) against
PREREG-deposit-run-split.md. Arm b1, game day 1 census on three boots
that carry S8: b8d2733d2b (S8), 35cd156e00 (W1) and a1441bcf4e (W2b).
Raids on.

| pair / arm (day 1)         | barn units | heaviest cell | cells used | max cell def        |
|----------------------------|-----------:|--------------:|-----------:|---------------------|
| c3b30ac4db b1 (before S7)  | 539        | 225           | 29         | mushroom x127 + ... |
| c3b30ac4db b2 (before S7)  | --         | 67-69         | --         | --                  |
| 21adf2f5df b1 (S7 in)      | 358        | 157           | 22         | mushroom x79 + ...  |
| b8d2733d2b b1 (S7 + S8) #1 | 394        | 58            | 51         | mushroom x54        |
| 35cd156e00 b1 #2           | 451        | 112           | 46         | mushroom x57        |
| a1441bcf4e b1 #3           | 339        | 82            | 40         | mushroom x64        |

- The runs spread (replicate 1: every forage deposit of >= 32 units on
  3-8 cells; 131 runs and 22 hauls all with cells; the identity switch
  off). The heaviest cell fell from 157-225 to 58-112 and the cells in
  use rose from 22-29 to 40-51.
- The bar (<= 64 with >= 20 cells) PASSED once and FAILED twice (112,
  82). The max cell def on all three replicates is a mushroom stack of
  54-64: the FOUNDING delivery drops its 64 mushrooms as 64 single items
  on ONE cell (a 2x2 pattern inside the cell), which merge into one stack;
  the day's chunks then land on it and around it. Replicate 3's 82 is
  64 founding + 18; replicate 1's 58 is the founding stack after a day's
  meals. The bar's producer changed: it is no longer the deposit run.
- Other general stores on the three days: heaviest 23-46 on 13-39 cells.

## Disposition

S8's mechanism PASSED (the runs are the spread they were meant to be).
The row's BAR is not met in two of three replicates because the founding
delivery (row S4, "deferred seed items DELIVERED") drops one 64-stack;
S4b (the founding stock deposited in chunks of DEPOSIT_CELL_CAP through
the same chooser) is the follow-on, registered under this row's bar. NOT
built: a day-long per-cell cap. The S7 read's FAIL branch named the
producer correctly; so does this one.
