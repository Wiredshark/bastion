# PRE-REGISTRATION — the glide override walks bodies into solid rock
Written before the fix exists. Base: bastion/item29-trade @ 1cc1f8035b.

## DEFECT (measured)
Kinematic mover, search-gap bridge. The surface probe scans dz in [0,+1,-1,-2]
for a standable cell. At the vertical FACE between the town's two grades
(z=181 <-> z=186, a 5-block step) nothing qualifies, so `landed == None` and
the probe REFUSES -- correctly. The override then waits `stuck_time > 1.0`
and pushes `try_pos` regardless: a cell it never validated, often solid.
Physics is opted out for these bodies (`kinematic_travels` = sole owner), so
nothing stops the write.

Evidence, live, instrument commit 1cc1f8035b:
- `entry_step` = 0.140 for EVERY embed sampled = KINEMATIC_WALK_SPEED*dt
  (4.2 * 0.0333). `entry_vel.z` = 0. Not one jump.
- `BASTION_POS_WRITE_DIAG` signed only 6 teleport-class writes; uncorrelated.
- Geometry: entry_z=181 face +5.0 -> lodges 183.9-184.5 (up); entry_z=185
  face -4.0 -> lodges 182.3 (down). 7 of 10 at a 1-5 block face.
- Scale: 63,445 EMBED WATCH fires in Ben's real world in 18h (50 of 51
  colonists; top 10 cells = 50%). 7,684 on the flat arm in 14 game days.

## THE CHANGE
The override may still glide when the probe fails for a NON-SOLID reason (a
gap or ledge with no floor below -- the case the bridge exists for). It must
NEVER glide into a cell whose feet or head block is solid. When it would, it
HOLDS and says so in a witness with a counter.

## PASS / FAIL, pre-registered
G1. EMBED WATCH fires fall >= 70% vs the matched baseline AT THE SAME TICK
    (control = the soak-embed run now in flight, same arm, same seed).
G2. The new refusal witness FIRES -- proving the branch is reached and the
    fix is not merely absent. A silent fix is an unproven one.
G3. No freeze traded for the fix: census `stuck` stays <= 1 sustained, and
    work-hour `idle` does not rise by more than 5 points vs control.
G4. `residual` in the claim refusal census stays 0 -- nobody loses a job
    because their body stopped being able to cross.
G5. Fail-safe teleports do not RISE. If bodies now stand where they used to
    phase, the stuck watch must not simply convert embeds into teleports.

## WHAT FALSIFIES THIS
- F1. G3 or G4 fails -> I traded phasing for freezing. That is WORSE: a
  frozen colonist is visible forever, an embed is corrected in a second.
  Revert and widen the probe's dz range instead.
- F2. Embeds fall but G5 rises by more than the embed fall -> the defect was
  moved, not fixed.
- F3. G2 never fires while embeds fall anyway -> something else changed;
  the attribution is wrong and the number is a coincidence.

## NOT EVIDENCED
- Whether widening the probe to dz +5 would be better than refusing. It
  would let bodies CLIMB a 5-block face in one step, which is teleporting by
  another name and violates BODIES GLIDE, NOT PHASE. Refusing is the
  conservative half; the real answer is a route that does not aim a walker
  at a wall, which is a later row.
- The 3 of 10 samples with face_height 0.0 are a separate sub-case this
  change may not touch.
