# PREREG — a barred door holds longer (D2) and the founding harvest (H0)

Registered 2026-09-02 14:05, before the binaries exist. Both are Ben's
rulings from 13:52, live.

## D2 — DOOR_FIGHT_SECS 10 -> 60

What stood: three first-raids, every door opened after held_secs=10.03;
two militia per cry, Guard claims 140-200 blocks of travel. Ben: "should
be longer; later door type/quality and the colonist's barring skill; I'll
let you choose the length." Sixty sim-seconds is my base (a muster from
150 blocks at walk speed arrives inside it); type, quality and skill are
named as future multipliers, both 1 today.

- Instrument: DOOR GAVE WAY lines carry held_secs; the raid recorder
  appends each boot's first raid to the raid table.
- PASS: on the next three first-raids, DOOR GAVE WAY per raid <= 1 (from
  4 of 4), held_secs on any gave-way line >= 60, downed still 0, and a
  militia post (AUTO-GUARD posted and STAFFED) within 8 blocks of a held
  door before it gives way on at least two of the three.
- FAIL branches: doors still give way at 60 with the militia never
  within 8 blocks -> the muster's routing is the row, not the hold; the
  raid ends before any door is tried -> unexercised (kept).

## H0 — the founding harvest

What stood: the adopted fields' MATURE cells stood for harvest jobs and
hauls through the walker leak; the town started its year with 64
mushrooms. Ben: the farmers should instantly harvest the crops that come
with the town.

- Mechanism: at the lived-in sowing a MATURE cell's yield goes into the
  founding delivery queue (the general store, chunked by S4b) and the
  cell restarts freshly sown; witness FOUNDING HARVEST per cell.
- PASS (the next boot): >= 20 FOUNDING HARVEST lines on day 0 with
  units summing to >= 40 (the arms place 300-700 lived-in sowings over
  14 stages; the top stage is ~1/14 of them); DELIVERED lines carry the
  crop items; the day-1 food_stock above the P1c boot's 855; the
  LIVED-IN stage histogram keeps 14 distinct stages.
- FAIL branches: no FOUNDING HARVEST lines while ADOPTED FIELD SOWN AT A
  LIVED-IN STAGE lines with stage=15 exist -> the branch is not reached;
  DELIVERED lines without the crop items -> the queue drops defs the
  delivery cannot resolve (an asset name); day-1 stock not above 855 ->
  the yield per cell (2) is too small to matter and the number goes to
  Ben with the economy read.
