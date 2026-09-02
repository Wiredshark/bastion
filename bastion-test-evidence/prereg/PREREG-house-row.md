# PREREG — the house row (review ROW A) and the build-count instrument (ROW B)

Written 2026-09-01 20:25, before the binary that carries the instruments exists.
Source: bastion-test-evidence/review-2026-09-01/synthesis.md, rows A and B.

## What is being measured

A1. `bed_jobs_in_flight` and `unbuilt_bed_regions` on every HOUSING BUILD
    witness line (once per game day, at the daily gate). They count what the
    "ONE plan at a time" guard is meant to bound; the guard itself reads
    `board.plans`, which the housing drain never writes.

A2. Whether a placed bed's XY falls inside a Farm/Zone/Plaza footprint
    (read from the HOUSE SITE / bed registered positions against the painted
    designations, by true surface z).

B1. `world_builds` and `ground_lifts` on the ROUTE-PREV CENSUS line (every
    300 ticks), after WORLD_BUILDS is bumped by EVERY completed block edit
    instead of Bed completions only.

## Arm

Flat map (`PLAY.ps1 flattown -NoRaids -NoWait -UserData ... -Port 14104`),
64 seeded materials (the preset), carried across at least THREE day
boundaries (tick 54,000 x 3). Precondition printed above every result:
`total=` equals `roster=` (headless presence VD 7 loads the whole town).

## Pre-registered pass / fail

A1 PASS: at every witness tick across three day boundaries with stone on
    hand, `bed_jobs_in_flight <= 1` and `unbuilt_bed_regions <= 1`.
A1 FAIL: either reads 2+ at any witness tick — the guard is dead and a real
    one keyed on the Bed job itself is the next build.
A1 UNREACHABLE (demote, keep the correction): three replicate arms never
    produce two simultaneous un-built Bed designations because `vacant_free`
    serialises them first.

A2 PASS: 0 registered beds inside a Farm/Zone/Plaza by true surface z.
A2 FAIL: any.

B1 PASS: after the change, a Mine or Build completion visibly increments
    `world_builds` at the next census line (it could not before).
B1 FALSIFIES the stale-route hypothesis: if, once honest, `builds_since_route`
    shows no correlation with embed events at any value, the 0.4–10 per 10k
    residual has a producer other than "terrain changed under a cached route".

## Not decided here (owner judgement, asked 2026-09-01 20:20)

Whether "the town builds a house" must place a real multi-block structure or
a bed marker is enough. The instruments above are unconditional either way.
