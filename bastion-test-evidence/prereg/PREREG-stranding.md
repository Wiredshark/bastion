# PRE-REGISTRATION — the fail-safe teleport's horizontal arm
Written before the fix exists. Base: bastion/item29-trade @ f338d9ca42.

## DEFECT (measured, n=55 over 14 game days, soak-accept)
`off_grade` arms the 60s fail-safe teleport when EITHER
  (a) |dest.z - pos.z| >= 3   [vertical: pit or roof — correct], or
  (b) lateral distance to `dest` >= 3   [horizontal].
`dest` is the nearest STANDABLE OPEN cell (r=0..8). Indoors, the nearest such
cell is outside the wall, ~7 blocks away. So (b) reads "indoors" as "in a hole".

Measured split of the 55 fires:
  6  vertical (genuine)                       -> MUST be retained
  27 horizontal, on_ground=true, Idle, at grade (mean |dz| 0.76) -> FALSE
  21 horizontal, on_ground=false (14 Wallrun) -> a DIFFERENT defect
  1  horizontal, Roll
12 colonists produced 39 of the 55 fires (max 7x one uid): the response repeats,
the outcome does not change.

## WHY NOT the obvious predicate
"grounded && head_clear" would disarm it — but all 6 GENUINE cases are also
grounded && head_clear. That fix would have starved the thing the guard protects.
Rejected on measurement, before building.

## THE CHANGE
`off_grade` keeps the vertical arm unchanged, and admits the horizontal arm ONLY
when the body is NOT on the ground. A colonist standing on solid ground at grade
is never "off grade", however far away the scan found its next open cell.

## PASS / FAIL, pre-registered
P1. Fires classified `horizontal + on_ground=true` drop to ZERO.
P2. All 6 vertical fires still arm (retention 6/6). ANY loss = the fix is wrong.
P3. Total fail-safe teleports per game day fall by >= 40% vs 3.9/day baseline.
P4. No colonist becomes permanently frozen: census `stuck` stays <= 1 sustained,
    and no uid holds a single position for > 300s.
P5. The airborne/Wallrun class (21) is UNCHANGED — it is not this row's target
    and must not be silently absorbed.

## WHAT FALSIFIES THIS
- F1. P2 fails (a vertical rescue lost) -> revert; the arms are not separable.
- F2. `stuck` climbs, or a body sits still > 300s -> the horizontal arm WAS
      load-bearing for a real trap I could not see in the teleport log.
- F3. Teleports fall but idle rises -> I moved the symptom, not fixed it.
- F4. Indoor colonists still teleport -> `on_ground` was the wrong discriminator
      (e.g. they are on a floor the physics does not call ground).

## NOT EVIDENCED, stated up front
- Whether any of the 27 was genuinely trapped indoors. Evidence says no (grounded,
  head clear, at grade, idle, jobless) but the teleport log cannot prove a
  negative. P4 is the falsifier that would catch it.
- The 21 airborne fires are left armed deliberately. Removing their rescue
  without a replacement could hang a body on a wall. Next row.
