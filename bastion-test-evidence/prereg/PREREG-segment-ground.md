# PRE-REGISTRATION — the ground BETWEEN two valid waypoints
Base: bastion/item29-trade @ 8692098e5b. Written before the change exists.

## DEFECT (measured, one cell, 263 identical events)
    route pair    prev(15165,15929,417) -> head(15165,15935,418)   dz=1 over 6
    entry from    (15165.5,15930.2,417.12)  step=0.140
    embeds at     (15165.5,15934.3,417.80)
    relocated to  (15165,15929,417)          (straight back to prev)
Both endpoints are walkable: SIX blocks of travel for ONE block of rise.
The body lodges in the MIDDLE. Linear interpolation 5.3/6 along the segment
gives z = 417.88, matching where it stops — so the GROUND at the midpoint is
ABOVE the straight line. `pure_glide` follows the line; the hill does not.

WHY THE TWO REVERTED FIXES COULD NOT SEE THIS:
  subdivider (85fe2824ae) triggered on endpoint |dz| > 2. Here dz = 1.
  rejection  (3bbd49b41a) triggers on endpoint |dz| > 6. Here dz = 1.
Both judged the ENDPOINTS. Neither ever sampled BETWEEN them. This is the
same blind spot twice, and it is why the residual survived both.

## PRIOR ART
Theta*/lazy Theta* shortcut two nodes only when the SEGMENT passes a
line-of-sight test — sampled along its length, not inferred from its ends.
That is precisely the test this router has never had.

## THE CHANGE
Walk the segment between consecutive waypoints, sample the real ground
(`column_surface_z`, the waypoints' own frame), and where the ground rises
ABOVE the interpolated line, insert a waypoint at the true surface. Endpoints
untouched; unreadable column inserts nothing (FALLBACK IS IDENTITY); bounded
insertions so a trunk stays a trunk.

## PASS / FAIL, pre-registered
G1. Embeds at the named cell (15165,15934) fall to ZERO. It is 263 of ~265
    events, so this is the whole residual, not a rate.
G2. Total embeds on the arm fall below 2 per 10k (from 11.3).
G3. The insertion witness FIRES — a drop with a silent mechanism is a
    coincidence, and I shipped an unwitnessed fix once today already.
G4. Nothing starves: residual 0, stuck <= 1, hauls and engaged within 30%.
G5. Waypoint counts do not explode (<= 4x), or the trunk has become a block
    path.

## WHAT FALSIFIES THIS
- F1. G1 fails -> the ground is NOT above the line and my inference from the
  interpolated z (417.88 vs the 417.80 where it lodges) is wrong; the cell
  holds something else (a sprite, a door, an entity) and the row moves.
- F2. G4 fails -> more waypoints means more places to stall, and a body that
  used to clip a hill now stops at it. That is WORSE and this reverts.
- F3. G3 silent while G2 passes -> attribution wrong again, for the third
  time on this residual; revert and instrument instead of iterating.

## NOT EVIDENCED
- That one cell generalises. It is 263 events at ONE location on ONE arm.
  The fix is aimed at a mechanism (segment vs endpoints) that should be
  general, but the evidence is emphatically local.
