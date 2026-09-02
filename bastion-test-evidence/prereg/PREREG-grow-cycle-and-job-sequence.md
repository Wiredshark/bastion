# PREREG — grow cycle in game days; the job-sequence census (Ben's ruling, 2026-09-01 20:35)

Written 21:05, before the binary exists. Source: Ben's own 4-hour session
(userdata-play-ben/voxygen/logs/2026-09-01_voxygen.log, singleplayer on the
08-31 client): stage-up every ~49 ticks, sown->mature 0.3 game hours (n=72
cells, 20,603 gaps); census working 0-3 of 9, moving 4-7 of 9; 4,179
per-item hauls admitted.

## Grow cycle

Mechanism: FARM_CYCLE_DAYS (4.0, ASSUMED until Ben names the number; under
3.0 is the rejected setting, pinned) x common::resources::DAY on the
TimeOfDay clock, divided over 14 stages; season factors unchanged; stamps
re-based to the same clock (an old sim-seconds stamp stages up once, then
re-stamps).

PASS: on the flat arm in Spring, the median sown->mature span over >= 20
cells is within +/-10% of FARM_CYCLE_DAYS game days (measured by the same
stage-up-gap analysis as Ben's log). FAIL: outside that, or any stage-up
faster than FARM_STAGE_SECS x 0.75 (Summer) on the day clock.
FALSIFIER of the frame choice: if stage gaps vary with tick rate (compare the
cargo-starved arm to an idle one), the clock is wrong.

## Job-sequence census (instrument only, no mechanism)

JOB SEQUENCE CENSUS, daily, per lane: works, hauls, alternations,
haul_share_pct (by CLAIM COUNT), mean_alternations, mean_max_work_streak.
Baseline to record before batching/haulers: expect, from Ben's watching,
mean_alternations >> 1 per day and mean_max_work_streak ~1-2 for Farm.
PASS for the later batching row: mean_alternations for Farm falls by >= 3x
and mean_max_work_streak rises to a day's work; haul_share stays above 0
(hauls still happen, at shift end / backlog). This row records the baseline
only.
