# RESULTS — the assist is the last writer; a window is not a hurdle (W4): the fence class is cured a hundredfold; a step-class residual names a third writer

Read 2026-09-02 15:51 against PREREG-fence-vault.md. Arm b1 on
b7536f4a21 (W4 on top of S4b), booted 15:30, day 0 cut at the first
18:00 line, against the S4b boot (9b42974b7e) cut at the same line.
Both cuts by `w4-compare.sh` (the first waiter's cut regex never
matched; its output is not used).

| by 18:00 game day 0            | control S4b | W4   | bar                 |
|--------------------------------|------------:|-----:|---------------------|
| vault assists                  | 1290        | 12   | <= 100   PASS       |
| step assists                   | 708         | 55   | --                  |
| nudges                         | 14          | 4    | --                  |
| worst (colonist, head) repeat  | 674 / 488 / 353 (vault) | 43 / 10 / 2 (step) | <= 3  FAIL |
| DID NOT STICK (exact to 16, then sampled) | -- | >= 32 (last sample) | <= 10  FAIL |
| OUTRANKS THE MOVER (sampled)   | --          | >= 64 assists with a rival | >= 1  PASS (validation) |
| window fronts                  | 0 (164 later that evening) | 0 | 0  PASS (weak: the control frame had 0) |
| glide refused into rock        | 28          | 28   | <= control  PASS    |
| glide overrides                | 1609        | 906  | --                  |
| probes                         | 6           | 18   | within 2-3x  FAIL (3.0x) |
| shuns                          | 17          | 36   | within 2-3x  PASS (2.1x) |
| store / forage deposits        | 75 / 131    | 82 / 131 | --              |
| starving max                   | 1           | 0    | --                  |
| panics                         | 0           | 0    | 0  PASS             |
| tick rate                      | 21.0        | 16.4 | (compile load)      |

## The evening frame (both logs cut at tick 33,300, game hour 23)

| by 23:00 game day 0            | control S4b | W4   | bar                 |
|--------------------------------|------------:|-----:|---------------------|
| vault assists                  | 1918        | 14   | PASS                |
| worst (colonist, head) repeat  | 678 / 488 / 429 | 49 / 10 / 2 (step) | FAIL (step residual) |
| window fronts                  | 164         | 0    | 0  PASS (real control now) |
| glide refused into rock        | 30          | 29   | PASS                |
| glide overrides                | 2372        | 1129 | --                  |
| probes / shuns                 | 17 / 29     | 23 / 39 | within 2-3x  PASS (1.35x / 1.3x) |
| store / forage deposits        | 75 / 154    | 82 / 163 | --              |
| starving max                   | 5           | 1    | --                  |
| tick rate                      | 23.2        | 18.9 | --                  |

In the longer frame the window bar passes against a real control (164
window vaults on the old pair, none on W4), and the probe and shun
rises shrink to within the replicate band. The step residual is the
only failed bar.

## Instrument validation

OUTRANKS THE MOVER fired from the first assist (colonist 89, dropped=1)
and reached 64 assists with a rival by 18:00: the stale kinematic write
existed in the drain exactly as the mechanism claimed. DID NOT STICK is
exact to 16 and sampled after; the line count from the MOVE ASSIST
records (43 + 10 + 2 repeats) agrees with the sampled witness (>= 32).

## What passed

The fence class. Vault assists fell from 1,290 to 12 in the same
frame; the three fence cells that held one colonist each all day (674,
488, 353 repeats on the control) do not appear. Colonist 38 crossed the
same fence line in two assists at two different cells and walked on.
Refusals into rock did not rise; starving fell to 0 in the frame.

## What failed, and what it names

- Two colonists repeated a STEP assist (an adjacent same-height cell,
  front Empty, on_ground, velocity zero) 43 and 10 times: colonist 85
  at (7727 -> 7728, 6400, 181) under a Rock/Grass seam, colonist 90 at
  (7733, 6370 -> 6371, 181). OUTRANKS fired for colonist 85 only twice
  in 43 assists: the drain held no rival write for the other 41, so the
  body was put back by a writer OUTSIDE the kinematic drain — physics or
  a safety net. This is the pre-registered FAIL branch "repeats persist
  with OUTRANKS rare". W4b (instrument, queued in the D1c chain): the
  DID NOT STICK line carries the body's cell against the promised one,
  the mover's last push site, the marker and the physics state, so the
  writer is named without a rerun.
- Probes 6 -> 18: the bodies freed from the fences stall somewhere new.
  Nine of the 18 sit at (7748, 6328, 181) on Designated jobs at tier
  Medium, not top-tier exhausted. The block map of that cluster is the
  next probe read.

## Falsification

All three pins turned RED with their defect planted (no retain; window
as hurdle; no repeat window) in an isolated worktree at the commit, and
green again at the commit. No pin stayed green.

## Disposition

W4 PASSED on the class it was built for (vault, window, refusal bars)
and FAILED its repeat bars through a step-class residual with a
different writer, named for W4b. Not yet evidenced: the live look (no
client run on the arm this round; Ben's session on b7536f4a21 is the
acceptance test), the evening window-front count on this pair, and
whether the freed bodies' new stall at (7748, 6328) is a route or a
mover problem.
