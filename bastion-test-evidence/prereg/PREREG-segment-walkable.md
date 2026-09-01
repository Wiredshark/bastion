# PRE-REGISTRATION — a route SEGMENT must be walkable, not just its endpoints
Base: bastion/item29-trade @ 85693da38e. Written before the change exists.

## DEFECT (measured, and produced by my own previous fix)
856132601b gave each tile-centre waypoint its own column height, which took
real-terrain embeds from 53.0 to 8.6 per 10k (-84%). The RESIDUAL is now
fully attributed on real terrain:
    writer_site = "chaser-pure-glide"   17 of 17
    entry_step  = 0.140                 (the glide, at walk speed)
    route_prev_solid = FALSE            17 of 17   <- waypoints are CLEAR now
    segment geometry: mean length 13.3, mean |dz| 12.9,
                      repeated seg(1,-1,-21) -- a 21-BLOCK DROP over ONE
                      block of horizontal distance
Both endpoints are valid standable cells. The straight line between them is
not. Tile centres are coarse, so on a slope adjacent tiles differ by many
blocks, and `pure_glide` interpolates straight through the hillside between
them. The previous fix traded "waypoints inside rock" for "segments through
rock", and that is the honest description of it.

## PRIOR ART
Theta* and lazy Theta* only shortcut between graph nodes when the segment
passes a LINE-OF-SIGHT test; without it, any-angle smoothing walks through
walls. Voxel movement adds a second constraint: a segment is walkable only
if its per-step rise is within the body's STEP HEIGHT. The mechanism
borrowed is the line-of-sight/step test on the SEGMENT, not on its ends --
which is exactly the property this router never checked.

## THE CHANGE
After the tile route is materialised, walk consecutive waypoint pairs and,
where the column height changes by more than one step, INSERT intermediate
waypoints sampled along the segment at their own column heights. The
endpoints are untouched; only the space between them gains detail. Where a
column cannot be read, no intermediate is inserted -- FALLBACK IS IDENTITY.

## PASS / FAIL, pre-registered
S1. Mean |dz| between CONSECUTIVE waypoints falls below 3 (from 12.9).
S2. Real-terrain embeds fall below 4 per 10k, from 8.6, on a matched arm.
S3. `route_prev_solid` stays FALSE -- the endpoints must not regress into
    rock while the segments are being fixed.
S4. No freeze: stuck <= 1, residual 0, hauls within 30% of control.
S5. Route length does not explode: waypoint count per route <= 4x control.

## WHAT FALSIFIES THIS
- F1. S2 fails while S1 passes -> segment dz was not the cause and the
  residual is something else the site tag has not separated.
- F2. S5 fails -> the sampler is emitting a waypoint per block and has turned
  a trunk route into a block path, which is the thing the tile graph exists
  to avoid.
- F3. S4 fails -> more waypoints means more places to stall; a body that
  used to glide through a hill now stops at it, which is WORSE.

## NOT EVIDENCED
- That one step is the right threshold. It is chosen to match the mover's own
  probe (dz in [0,+1,-1,-2]) rather than picked freely, but the mover's probe
  is itself a guess I did not measure.
- Whether the tile graph should simply not route across a 21-block drop at
  all. That is a router-topology question, larger than this row, and this
  change does not answer it.
