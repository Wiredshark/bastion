# PREREG — the wedge probe (P1): what a stalled fetch stands against

Registered 2026-09-02 09:45, before the probe pair is built.

## The symptom that ranks this row first

Flat arm b2 on c3b30ac4db, game day 1 (the run's second day), 09:00 to
15:00: starving rose from 6 to 20 of 50 while `food_stock` sat flat at
~297 units. Nobody drew a meal from a store for six game hours. In that
window 143 stalled targets were shunned; 107 of them were the two food
cells of general store zone 72 at (7776, 6356) and (7774, 6355), which held
the town's mushrooms (154 units at the 09:00 census). 111 of the fetch
stalls stood on ONE spot, (7744-7751, 6328-6335, z 181), 40 blocks from
those cells; 52 of the 90 EatFrom budget expiries reported a distance of
30-39 blocks. Arm b1 at the same clock: 55 stalls on the same spot and 46
shuns on the same cell, starving 5 (its founding stock was spread
differently; colony counts vary 2-3x between arms).

Day 0 on both arms fed from the founding stock, which row S4 spread on
other stores. On day 1 the food is where the walkers cannot go.

## Why the fix is not known

The fetch leg's rescue, detour and egress paths are all gated by
`fetch_steer.is_none()` (documented at the fetch watchdog in
`bastion_jobs.rs`), so a wedged fetch is rescued by nothing: MOVE ASSIST
fired 2,012 times on b2's second day and 0 times near this spot. The stall
clock (row E1) expires the trip after 15 s, the shun (row E2) hides the
cell for six hours, the re-target picks the store's other cell, and the
next eater walks into the same spot.

What the body meets there is unknown. RESULTS-store-close named this spot
as "an approach, not a door" and asked for a looking sweep with a client.
Ben is away and a client look by me needs his desktop. The probe stands in
for the look.

## The instrument

At the existing once-per-job first-stall witness (FETCH STALLED), a WEDGE
PROBE line: the 5x5 blocks around the feet on four layers (z-1 floor, z+0
feet, z+1 body, z+2 head+1), north row first, west to east, as glyphs
(`#` solid, `.` air, `~` filled but not solid, `?` unloaded), the integer
vector to the item, and the fetch steer target. Read-only. No behaviour
changes; identity by construction.

Instrument validation first: the probe must fire at the known spot with
`to_item` pointing north-east (+28, +28) and a `steer` that is Some. If
`steer` is None on the stalled fetches, the walker had no target, which
is itself the finding.

## Pre-registered outcomes (one arm day on the raid arm b1 on the probe pair)

- PASS (the probe answers): >= 20 WEDGE PROBE lines with feet in
  (7744-7751, 6328-6335) and one block pattern shared by >= 80% of them.
  The pattern is then read for the obstacle class:
  - a `#` at z+0 or z+1 in the item's direction = a solid the walker
    cannot step (a two-high edge, a fence, a plot wall) -> the pathing row
    prices or routes around it, or the rescue is extended to fetches;
  - a `?` layer = an unloaded column -> the chunk-load row;
  - all `.` around the feet at z+0 and z+1 = not terrain -> a route or
    kinematic refusal (the glide override's ruling, the chaser's node)
    and the next probe reads the chaser.
- FAIL (the probe is blind): fewer than 20 probes at the spot while
  FETCH STALLED keeps counting there (the witness and the probe disagree),
  or no dominant pattern (< 50% agreement), which means the spot is a
  region of several obstacles and the probe box is too small.
- NOT a fix. A fix row follows the reading and gets its own registration.
