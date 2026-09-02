# RESULTS — the route drop escalates the search tier (W2c): the escalation fires; the spot stays low; same-tier re-stalls remain by the pathfinder's own retarget rule

Read 2026-09-02 14:07 against PREREG-w2c-route-drop-escalates.md. Arm b1
on 5d4ce3ad69 (W2c on top of W5, P1c, W1, S8), game day 0 by 18:00,
raids on.

| by 18:00 game day 0     | P1b baseline | W2 | P1c | W5 | W2c | bar          |
|-------------------------|-------------:|---:|----:|---:|----:|--------------|
| probes                  | 56           | 14 | 22  | 10 | 22  | --           |
| probes at the spot      | 37           | 3  | 12  | 0  | 3   | <= 8   PASS  |
| shuns                   | 60           | 21 | 26  | 14 | 26  | <= 30  PASS  |
| store deposits (NE store) | 12         | 13 | 14  | 8  | 11  | >= 12  FAIL (by one) |
| EatFrom expiries        | 6            | 1  | 7   | 4  | 8   | --           |
| top-tier exhausted probes | --         | -- | 0   | 0  | 0   | <= 2   PASS  |
| evening starving        | 1-2          | 1  | 0-2 | 1  | 1   | --           |

- Instrument validation: every CLIMB BANNED (fetch) line carried
  tier_after_drop one step above the probe's tier (Long x4, Medium x2).
  PASS.
- Re-stalls at the same tier after a drop persist (job 494 four times at
  Small, job 562 twice, job 488 twice at Medium). The pathfinder resets
  the tier whenever the target moves by more than two blocks ("a new
  target is a new problem", path.rs ~937), and a re-aimed or re-claimed
  fetch is a new target; the escalation survives only while the same
  cell is chased. The bar "no job probed twice at the same tier after a
  drop" FAILS on that rule, which is the engine's own and right for the
  job leg.
- The wedge population moved: nine probes at (7648-7655, 6384-6391, z 183)
  on this boot (one on the P1 boot, one on W5), three at the old
  roof-stair spot, three at (7712, 6152, 183). The block map of the new
  cluster is read in the next probe pass.

## Disposition

PASSED on the spot, shuns and top-tier bars; FAILED the single-store
deposits bar by one (11 vs 12) and the same-tier re-stall bar by the
retarget rule. Kept: the escalation is correct for a retained target.
The next lever is not the tier: it is the new cluster at z 183 (a
rooftop or a raised platform) and the fence-vault fetch (W4).
