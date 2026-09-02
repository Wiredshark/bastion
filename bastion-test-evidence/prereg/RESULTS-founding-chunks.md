# RESULTS — the founding stock lands in chunks (S4b): the founding stack is gone; the barn's heaviest cell is now a haul-deposit heap

Read 2026-09-02 15:12 from arm b1's S4b boot (pair 9b42974b7e, booted
14:46:43, day 1 line at 15:11) against the bars in commit 9b42974b7e.

## Instrument (the DELIVERED line's `cells=`)

| founding def | count | cells delivered                                   | bar        |
|--------------|------:|---------------------------------------------------|------------|
| mushroom     | 64    | 7660:6354:16 7669:6354:16 7683:6354:16 7649:6355:16 | >= 4 PASS |
| stones       | 64    | 7651:6355:16 7652:6355:16 7653:6355:16 7654:6355:16 | --        |
| wood         | 32    | 7656:6355:16 7657:6355:16                         | --         |
| wheat seeds  | 8     | (none listed)                                     | --         |

The seeds show no cells because a delivery of 16 or fewer takes the
unchunked branch (eight single items around `pos`); the line prints
`pos=` for that branch, so nothing was lost — but `cells=` should read
"unchunked at pos" rather than blank. Instrument note, not a defect.

## The S8 bar on day 1 (barn heaviest cell <= 64, cells_used >= 20)

| zone | units | heaviest cell | what it holds | cells used |
|-----:|------:|--------------:|---------------|-----------:|
| 23 (barn) | 462 | 88 | stones x76 | 43 |
| 36   | 106   | 19            | raw bird meat x9 | 14 |
| 39   | 41    | 8             | mushroom x6   | 15 |
| 56   | 181   | 28            | carrot x14    | 35 |

cells_used 43 >= 20 PASS; heaviest cell 88 > 64 FAIL — but the producer
moved again. On day 0 the barn's stones cell read 32 (a founding chunk
of 16 plus early hauls); by day 1 the same cell held 76 stones because
every `haul deposited` of mined stone lands on the cell that already
holds stones (`dest=(7650, 6355, 181)`, `dropped=3` per haul, all day).
The founding stack is no longer the heaviest cell's producer; the
HAUL-JOB deposit path is, and it has no cell cap (DEPOSIT_CELL_CAP
applies to deposit runs and the founding delivery only).

## Also on this boot

- eat census day 1: eat_minted 52, meals 44, targets shunned 29.
- D1b alarm line (first raid, 18:00): sheltered 24, workers preempted 22,
  already home 2, out of earshot 7 — read in
  RESULTS-danger-answered-like-a-town.md.

## Disposition

S4b's mechanism PASSED its own instrument (four cells of 16 for the
64-stack). The S8 heaviest-cell bar FAILED through a third producer:
haul deposits merging by def onto one cell. Candidate row S9 (the haul
deposit picks a cell under the cap, spreading like the runs do); it is
one item-entity either way on screen, so it ranks below the movement
rows. Not evidenced: whether a 76-stack is visible to a player at all.
