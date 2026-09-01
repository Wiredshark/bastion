# PRE-REGISTRATION — the tile graph cannot see a cliff
Base: bastion/item29-trade @ 8508758564. Written before the change exists.

## DEFECT
    pub tiles: HashMap<Vec2<i32>, (f32, Option<u32>, bool)>,  // cost, plot, door
    pub ground_z: i32,                                        // ONE, for the town
The trunk router's graph holds NO per-tile height. `tile_route` is purely 2D,
so it cannot know that two adjacent tiles differ by 21 blocks. It does not
CHOOSE a cliff -- it cannot SEE one. Every downstream fix has been treating a
symptom of this.

Measured residual after the column-height fix (85693da38e): 8.6 embeds per
10k on real relief, `writer_site=chaser-pure-glide` 17 of 17,
`route_prev_solid=FALSE` 17 of 17 -- endpoints valid, LINE through rock --
segment geometry mean |dz| 12.9, repeatedly `seg(1,-1,-21)`.
Subdividing those spans was built, pinned, run and REVERTED (8508758564): a
cliff cannot be subdivided.

## PRIOR ART
Theta*/lazy Theta* gate the EDGE on line of sight; voxel movement adds a
step-height constraint on the edge. A* is only as good as its transition
function, and this one encodes cost and doors but not traversability in z.
The graph already fences illegal edges -- crossing between two buildings is
priced at 10000 -- so the MECHANISM for refusing an edge exists and is
proven; it simply has no height to refuse on.

## THE CHANGE
Carry `tile_z` (per-tile ground, from `sim.get_alt_approx` at ingest -- the
same source the site scorer already uses) and, in `tile_route`, PRICE an edge
whose |dz| exceeds a walkable step at the same 10000 the door rule uses.
PRICE, not forbid: a fence keeps the tile reachable when there is no
alternative, which is what the door rule does, and is the difference between
a longer route and a colonist who can never get home.

## PASS / FAIL, pre-registered
R1. Mean |dz| between consecutive route waypoints falls below 4 (from 12.9).
R2. Real-terrain embeds fall below 5 per 10k (from 8.6), matched arm.
R3. NOTHING becomes unreachable: claim-census `residual` stays 0, `stuck`
    <= 1, and `FETCH BUDGET EXPIRED` does not rise above control.
R4. Routes do not explode: hauls delivered within 30% of control.

## WHAT FALSIFIES THIS
- F1. R3 fails -> the fence cut the town in half. A colonist who cannot
  route home is far worse than one who clips a hill, and this REVERTS.
- F2. R1 passes and R2 fails -> segment dz was never the embed cause, and the
  remaining embeds are something the site tag has not separated.
- F3. `get_alt_approx` disagrees with loaded terrain at the tile centre --
  it is sim data, deliberately terrain-independent, and the waypoint z uses
  `column_surface_z` on REAL terrain. Two producers of one quantity is the
  commonest defect in this program, so if R1 passes while embeds cluster at
  tiles whose two heights disagree, THAT is the finding.

## NOT EVIDENCED
- The step threshold. Reusing 2 (the mover's own probe window) rather than
  choosing freely, but the probe window is itself unmeasured.
- Whether a town on real relief is routable at all once cliffs are priced.
  R3 is the guard on that and F1 is the exit.
