# RESULTS — eat patience (E1) + stalled-target shun (E2), and the guard door (G3) first read

Read 2026-09-02 07:45. Arms: flat arm b2 on f4f7c3f90b (E1+E2, no store
close -- the ablation), flat arm b1 on a900163959 (S5b, guard door on).
Roster 49-50.

## E1 + E2 — the eat leak, before and after

| run                              | eat minted | meals | meals/minted | EatFrom expiries | evening starving (hours 20-23) |
|----------------------------------|-----------:|------:|-------------:|-----------------:|-------------------------------:|
| b1 6ee29a1817 (crate tops)       | 99         | 46    | 46%          | 70               | 3-7                            |
| b1 07c3622768 (floor, unspread)  | 62         | 45    | 73%          | 33               | 0-3                            |
| b2 a900163959 (floor, spread)    | 81         | 40    | 49%          | 54               | 3-5                            |
| b1 a900163959 (floor, spread)    | 74         | 44    | 59%          | 44               | 1-3                            |
| **b2 f4f7c3f90b (E1+E2)**        | **50**     | **43**| **86%**      | **21**           | **1-5**                        |

- PASS on the pre-registered meals line (>= 80% of eat jobs end in a
  meal); the eat-job count itself halved (fewer re-preempts on dead
  targets).
- Expiries fell 54-70 -> 21, not to the pre-registered <= 10. The top
  shunned cell (32 of 68 shuns) is (7673, 6426, 183) at another store, a
  pile one block above the floor-plus-one the spread allows -- the
  arrival-radius edge; the fourth store's cells took 22 shuns.
- The fourth store kept filling on the ablation (87 mushrooms on one
  cell, 131 units): that is what S6 (store close) exists for; both arms
  are on the S6 pair now.
- Instrument defect found and fixed next: the tolerated-stall warning
  fired every tick (37,524 lines in one day); it now fires once per job.

## G3 — the guard door, first read: FAIL (the generator, not the door)

b1 on a900163959, day 2 GUARD SUMMARY: guards 7, patrols_posted 3,
guard_door_shut 0, plaza 0%, entrance 1%, street 28%, elsewhere 70%.
The door never shut: guards never tried non-Guard claims, so the door had
nothing to refuse. Three legs were posted all day for seven guards, and
each posted leg ran to shift end (the leg-switch loop works). The Guard
lane's JOB SEQUENCE line shows one colonist with one claim -- six guards
made no claims at all, took no leg, and stood "elsewhere". The
pre-registration's own FAIL branch names this: "patrols_posted < guards
with the door shut -> the generator is not posting". The generator's
gates (an active job, an existing auto_patrols entry, no leg) are not
distinguishable from the log; a PATROL PASS census is staged next to say
which one holds each guard. The door stays (harmless, pinned) until that
read.

## Not evidenced

- S6 live (b1's day-1 line on 06f9a5cb91 pending).
- Why six guards took no leg.
