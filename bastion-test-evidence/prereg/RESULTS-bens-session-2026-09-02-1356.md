# RESULTS — Ben's session of 2026-09-02 13:56 (pair 5d4ce3ad69, real terrain): eleven minutes of empty world, ninety seconds of colony, three of nine bodies vibrating at a fence

Read 2026-09-02 14:25 from `play-server.log` (2,004 lines, 13:56:26 to
14:08:57). One real session; mined before any soak, per the loop.

## Timeline

| wall  | tick   | what                                                                 |
|-------|-------:|----------------------------------------------------------------------|
| 13:56 | 0      | boot; ADOPT-A-TOWN WAITING for a town to be chosen (every 600 ticks) |
| 14:07 | ~18000 | no town chosen in 10 minutes -> AUTOFOUND (fields first), 37 sites   |
| 14:07 | 18300  | roster 9, all fed, game hour 17 (Leisure) -- the work day was over   |
| 14:08 | 19500  | the first fence vault at (6627, 25842, 181): uid 36, then 37, 233    |
| 14:09 | --     | server stopped (no shutdown line; the harness closed it)             |

The colony existed for 90 seconds of wall time. Every census before tick
18300 reads roster=0; the whole "work day" that the sows=0, hauls=0
numbers describe was an empty world. Nothing about farming can be read
from this session.

## What the colony did in its 90 seconds

- 35 houses adopted (8 occupied), 5 buildings labelled (2 taverns, 2
  workshops, 1 dock), 18 fields marked for lived-in first sows.
- Founding seed items DELIVERED into the general store (8 wheat seeds,
  64 mushrooms, 64 stones, 32 wood) -- the chunk-load deferral worked.
- HOUSING GROWTH fired ("a house stands empty", drive Expand) and one
  settler arrived: Sten of the Vale, Chef.
- TRADE LANE DEAD x30 (WARN): food_stock 72 against a seasonal par of
  1,823 for nine people -- a par-vs-consumer question, not a symptom.
- REACHABILITY FLOOD BLIND x2: 95,654 cells surveyed, 4,644 on an
  unloaded frontier, 11 condemnations withheld -- correct behaviour on a
  client-loaded world.
- One slow rtsim tick (1,408 ms) at founding.

## The worst row: THE VAULT THAT NEVER LANDS

In the session's last 45 seconds three of the nine colonists stood at a
hurdle with the route head two blocks beyond it and never crossed:

| uid | feet                 | front              | errand                       | assists | glide refusals |
|----:|----------------------|--------------------|------------------------------|--------:|---------------:|
| 36  | (6627.5, 25841.9, 181) | FenceWoodWoodland | deposit run (seeds, mushrooms) to the store 55 blocks north | 24 | doubling to 64 |
| 37  | (6627.5, 25841.9, 181) | FenceWoodWoodland | the same deposit run, the same cell | 19 | to 256 |
| 233 | (6586.5, 25782.0, 181) | Window            | walking to a lounge seat      | 16 | to 512 |

Each MOVE ASSIST line says "the vault/step completes"; the feet on the
next line are unchanged. The mechanism, read in the producer:

1. The router builds a VAULT EDGE over any solid sprite of height
   0.2..=1.6 (path.rs ~1926) -- a fence (1.09) and a house WINDOW alike.
   The body's glide refuses windows (`blocks_colonist_body`) and the
   fence cell is solid, so the glide override is REFUSED INTO ROCK every
   tick and pushes a `chaser-refused-rock` hold carrying the STALE
   position onto `pending_kinematic`.
2. After 1.5 s the MOVE ASSIST fires: `pending_assists` writes the body
   onto the promised cell (bastion_jobs ~37004).
3. The kinematic drain runs AFTER the assist drain (~37132) and writes
   the stale hold back. The teleport lives for zero ticks.

The same loop on the flat arm b1 (W2c boot, day 0): 814 vault assists,
made of exactly three (colonist, head) pairs repeated 687, 127 and 63
times. No vault assist has ever landed on either world. Nudges share the
drain and the fate.

## Also seen

- TRUNK ROUTE REJECTED x6 (worst_dz 7 over 62 waypoints): the town's
  trunk routes still carry a 7-block step; the search pump takes over.
- ADOPT-A-TOWN held founding for 10 minutes because no town was chosen
  on the character screen or by middle-clicking the map. Ben played an
  empty world for 11 of 13 minutes. The timeout is a taste number; his
  call.

## Disposition

Worst row picked: W4, THE ASSIST IS THE LAST WRITER + A WINDOW IS NOT A
HURDLE (PREREG-fence-vault.md). Farming, hauling and the plaza cannot
be read from a 90-second colony; the next session needs the town chosen
at character creation.
