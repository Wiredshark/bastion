# PREREG — the Haul lane has a ceiling (a town of haulers is not a town)

Written 2026-09-02 01:05, before the build. Source: flat arm b1 (26e0852dae),
day 2: JOB SEQUENCE by lane named 16 of 29 colonists `Haul` (works=69,
hauls=41), Farm 5, Cook 5, Chop 2, Guard 1. The daily profession tally is
an argmax over time-held lane counts with hysteresis; hauling is the most
time-held activity for over half the town, so over half the town is
"a hauler". PROFESSIONS IN LANE says a watcher should name someone's job
from an hour of watching; "hauler" for 16 of 29 is not that.

## Mechanism

`cap_haul_lane`: after the argmax names the day's professions, the Haul
lane may hold at most `haul_lane_cap(roster)` = max(2, roster / 4)
(ASSUMED; a number of taste, Ben may name another). Surplus haulers --
the ones with the SMALLEST Haul counts among today's Haul names -- keep
their incumbent trade if they had one, otherwise their best non-Haul
lane, otherwise they stay named by hauling (nothing to fall back on).
Dedicated haulers (M4) still tops the lane UP to its floor afterwards;
floor <= ceiling by construction. `BASTION_NO_HAUL_CAP` = identity.

## Pre-registered pass / fail (arm, two day lines)

PASS: the Haul lane holds <= max(2, roster/4) on every day line from day 2
on, Farm/Cook/Chop/Mine lanes hold the rest, and the STORAGE SUMMARY's
general stores keep receiving (haul work still happens -- the floor
guarantees haulers). FAIL: general stores stop receiving (haul_share
collapses) -- then the cap starves the thing it protects and must rise,
or non-haulers must haul at shift end (M3).
FALSIFIER of the reading: if capping the lane does not change what a
watcher sees (people still spend their hours carrying), the tally was
only mislabelling and the real row is the haul VOLUME (per-item hauls,
row 48), not the label.
