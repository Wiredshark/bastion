# PRE-REGISTRATION — the tile trunk materialises every waypoint at ONE z
Base: bastion/item29-trade @ 9a4b5c9222. Written before the fix exists.

## DEFECT (measured, with a control)
    let z = if g.ground_z != 0 { g.ground_z } else { pos.0.z.floor() as i32 };
    wps = tiles.map(|t| { ... Vec3::new(c.x, c.y, z) })
The tile graph is 2D with ONE global ground height. Every waypoint of a trunk
route is placed at that street-level z regardless of the real ground under
that tile. In a town with two grades -- the measured 181 and 186 -- a route
at 181 crossing ground at 186 puts waypoints FIVE BLOCKS INSIDE ROCK.
`pure_glide` (Ben's simplicity ruling) then walks the body to them with no
terrain check, by design and on the stated premise that the plan is
admissible.

Chain, every link measured:
  63,445 EMBED WATCH fires in the owner's real world in 18h, 50 of 51
  colonists, 50% at ten cells
    <- writer_site = chaser-pure-glide, 69 of 69
    <- kinematic = true, 52 of 52 (the mover owns the body)
    <- entry_step = 0.140 = KINEMATIC_WALK_SPEED*dt (a glide, not a write)
    <- route_prev SOLID in 94-96% of embeds vs a 12% BASE RATE across all
       router-following colonists (n=102,264 body-ticks) -- 7.8x enrichment
    <- waypoints materialised at a constant z
Geometry agrees: entry z=181 against a +5.0 face, lodging 183.9-184.5,
relocated to 186.

## THE CHANGE
Each tile-centre waypoint resolves its OWN z from the column beneath it
(`column_surface_z`, already used by the teleport/egress probes), landing on
the standable cell above that surface. When the column cannot be read the
waypoint keeps today's constant z -- FALLBACK IS IDENTITY. Door tiles keep
their probed door-sprite cell, untouched.

## PASS / FAIL, pre-registered
T1. `ROUTE-PREV CENSUS` pct_solid falls from 12% to <= 4%.
T2. EMBED WATCH fires fall >= 50% vs the matched control AT THE SAME TICK.
T3. Nothing freezes: census `stuck` <= 1 sustained, work-hour `idle` up no
    more than 5 points, claim-census `residual` still 0.
T4. Fail-safe teleports do not rise.
T5. Haul throughput does not collapse: `haul delivered` per 10k ticks within
    30% of control (ROW 28 v4 collapsed haul chains 4x -- that is the exact
    failure this row must not repeat).

## WHAT FALSIFIES THIS
- F1. T2 fails while T1 passes -> waypoint z was not the lever; the embeds
  come from the segment BETWEEN waypoints (corner-cutting), which is a
  different repair.
- F2. T5 fails -> route shape regressed. ROW 28 records FOUR same-day
  route-shape fixes that each made embeds worse (207/282/304/415 by hour 16)
  and one that collapsed haul chains 4x. If this joins them, REVERT; do not
  iterate on it in the same session.
- F3. T3 fails -> a body that used to glide through rock now stands at it.
  That is worse: the embed self-corrects in a second, a frozen colonist is
  visible forever.

## NOT EVIDENCED
- Whether `g.ground_z` is ever RIGHT for a whole town. If a town is genuinely
  flat, this change is a no-op there and T1/T2 will show nothing -- the flat
  lab arm may therefore UNDERSTATE the gain, and the owner's real terrain is
  where it matters. n=1 per arm; counts vary 2-3x.
