# PREREG — ROW D: two pin gaps, falsified by planting (review 2026-09-01)

Written 2026-09-01 20:35, before the pins compile.

## The gaps

D1. `off_grade_verdict`'s airborne arm `(!grounded && dist >= OFF_GRADE_BLOCKS)`
    had no fixture near its boundary with `grounded = false`; dropping the
    distance clause left the suite green.
D2. `lift_over_ground`'s comparison `gz + 1 > line_z` was never exercised at
    `gz == line_z` (ground block at the line's own height, feet in rock).

## New pins

- `off_grade_airborne_arm_is_pinned_at_its_own_boundary`
- `lift_over_ground_is_pinned_at_the_ground_equals_line_boundary`

## Pre-registered results

GREEN on HEAD: both pins pass unmodified.

RED under each planted mutation, one at a time, reverted after:
- M1: `(!grounded && dest_xy.distance(pos_xy) >= OFF_GRADE_BLOCKS)` → `(!grounded)`
      must turn `off_grade_airborne_arm_is_pinned_at_its_own_boundary` red.
- M2: `if gz + 1 > line_z {` → `if gz > line_z {`
      must turn `lift_over_ground_is_pinned_at_the_ground_equals_line_boundary` red.

A PIN THAT STAYS GREEN under its mutation is reported as such in the commit,
and is worth more than the pin. No mechanism changes in this row.
