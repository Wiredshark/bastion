# RESULTS — a far target starts at a far tier (W3): the tiers moved as designed; the barn cluster shrank but persists, and its route is the terrace

Read 2026-09-02 17:47 against PREREG-far-target-tier.md. Arm b1 on
89be848420 (W3 on top of D2/H0/G1b/F1/W4), booted 17:28, day 0 by
18:00, raids on. Baseline: the W2c boot (5d4ce3ad69).

| by 18:00 day 0                    | W2c boot | W3 boot | bar        |
|-----------------------------------|---------:|--------:|------------|
| probes                            | 22       | 18      | <= 15  FAIL (by 3) |
| barn-wall cluster                 | 9        | 6       | <= 2   FAIL |
| the old roof-stair spot           | 3        | 2       | --         |
| store deposits (NE store)         | 11       | 12      | >= 12  PASS |
| all haul deposits                 | --       | 77      | --         |
| shuns                             | 26       | 19      | <= 26  PASS |
| EatFrom expiries                  | 8        | 3       | --         |
| tick rate over the afternoon      | ~19      | 20.2    | >= 15  PASS |
| probes at tier Small with a target > 30 blocks | 15 of 22 | 1 of 18 | 0  FAIL (by 1) |

Tier by state on the W3 boot: Medium-Exhausted 10, Long-None 3,
Long-Exhausted 3, Small-Path 1, Small-Exhausted 1. The ladder starts
where the distance says (instrument validation holds but for one
probe, probably a target that moved under 30 blocks after the start).

## What the surviving cluster says

The six barn probes sit at (7648, 6384, 183) inside the barn and
exhaust at Medium and Long. The store they want is the NE store on the
plateau at (7774, 6354, 182) — the plateau behind the two-block terrace
step at (7748, 6327) that b2's 209 stalls and b1's 42-of-60 climb
probes also stand under. More search budget does not find a route the
move set cannot take: the searches climb the ladder and exhaust higher.
This is the pre-registered FAIL branch ("the cluster persists at
Medium/Long Exhausted -> there is no route ... and the store's approach
is the row"). The approach is the terrace: W6.

## Disposition

W3 PASSED its deposit, shun and tick-rate bars and its start-tier
intent; FAILED the cluster and probe bars, by mechanism now named
(W6, PREREG-terrace-scramble.md). Kept: a far target starting at a far
tier is right on its own; the tick rate held. Not evidenced: whether
the plateau store is reachable by any walk at all once the scramble is
either taken or withdrawn.
