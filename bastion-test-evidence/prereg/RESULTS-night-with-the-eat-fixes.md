# RESULTS — the night (N1, N2, N3 supper) read again with the eat fixes (E1, E2, S6) in place

Read 2026-09-02 08:10 against PREREG-night-hunger-met-at-home.md and
PREREG-supper-before-curfew.md. Arm: flat arm b2 on 06f9a5cb91 (every row
through store close), first night. Roster 50.

| tick   | hour ~ | fed | below_interrupt | starving |
|--------|--------|-----|-----------------|----------|
| 36,000 | 0      | 44  | 4               | 1        |
| 40,500 | 2      | 42  | 3               | 1        |
| 45,000 | 4      | 37  | 5               | 3        |
| 48,000 | 5      | 33  | 6               | 2        |
| 54,000 | 7      | 41  | 2               | 1        |

Against the same hours before the eat fixes (night rows on, food on
crate tops or in the unenterable store): fed 19-33, starving 7-11 at
dawn; and before the night rows: fed 8-14 at dawn.

- below_interrupt at hours 2-4: 3-5 of 50 (6-10%) -- PASS on the <= 20%
  line.
- starving at dawn: 1 (never 0 through the night; 1-3) -- the strict
  line FAILS by one to three colonists. Night meals at home: 0 (nobody
  needed one); supper preempts: 7 of 44 (most colonists were already fed
  when the supper hours came).
- Reading: the night was never the binding term once the meals could be
  served; the residual 1-3 starving is the walker tail (a stalled trip
  now waits its 90 s budget and re-aims, but a wedged walker on a
  structure still misses a meal). That tail is the pathing row and needs
  a looking sweep, not another hunger rule.

## Disposition of the deferred rows

- Supper severity (N3b): NOT built. The eat census showed
  drive_not_personal was secondary (5-7k passes against 15-20k on need
  jobs and the trip failures); with the trips fixed, 7 supper preempts
  sufficed. Stays in reserve.
- Rations home: NOT built; no night meal was needed on this run.

## Not evidenced

- Two replicates of this night (b1 was moved to the instrument pair).
- The night on Ben's terrain.
