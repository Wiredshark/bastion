# PRE-REGISTRATION — why does a colonist holding a route show speed = 0.0?

Written before the run. This is the row that gates every acceptance criterion:
the churn, the empty houses, the unworked farms and the "mad scramble" are all
plausibly one bug wearing four costumes.

## Established, not being re-tested

- Colonist 98, tick 44,713: `hunger=0.0`, rest 0.038 falling, holding a Haul
  job, `dist=5.0`, `dz=0`, **`speed=0.0`**, `best_dist=3.32` (it approached,
  then drifted back out).
- Colonist 97: `dist=4.12`, `dz=0`, `speed=0.0`.
- Across legs: 78-82% of eat-job budget expiries occur within 15 blocks; one
  colonist logged 0.00006 blocks of progress in 90 seconds, twice.
- `auton_travel_ok` DOES permit a self-job on `Drive::Personal`, so "no Goto is
  issued for eat jobs" is already ruled out by reading.

## Instrument

`BASTION_FLIGHT_RECORDER_DIR` + `SAMPLE_EVERY=15`. `FlightSample` carries
`goto_target`, `controller_move_dir`, `controller_move_z`, `movement_writer`,
`chaser_path_state`, `velocity`, `character_state`, `on_ground`, `on_wall`,
`active_job_state`. That set separates every branch below.

## The branches, which look different in the tape

- **A — NO GOTO.** `goto_target = None` while `active_job_state = Traveling`.
  The travel gate refused. Then the fix is in `auton_travel_ok` / the drive.
- **B — GOTO SET, NO STEER.** `goto_target = Some`, `controller_move_dir ≈
  [0,0]`. The agent received an order and produced no movement. Then
  `chaser_path_state` names it, and the fix is in pathing, not bastion.
- **C — STEER SET, NO MOTION.** `move_dir` non-zero and `velocity ≈ 0`.
  Movement is commanded and physics refuses it — a wall, a collision, or
  colonists standing in each other. Then the fix is geometric.
- **D — BASTION FIGHTS THE AGENT.** `movement_writer` names a bastion override
  on the same ticks `move_dir` is zero. There are 12 `move_dir = Vec2::zero()`
  sites in bastion_jobs; two are arrival-only, ten are not audited.
- **VOID** — no colonist freezes for >=30 consecutive samples this run. Then
  nothing is learned and I say so rather than reasoning from the absence.

## The discriminator

One field ordering answers it: `goto_target.is_some()` splits A from B/C/D;
`controller_move_dir` splits B from C/D; `movement_writer` splits C from D.
Read them in that order and the branch is forced, not chosen.

## What this run CANNOT test

- Whether fixing it makes the town LOOK better. Separate looking sweep.
- The 99 pure-vertical strandings (row 0b) — those are dz +9..+13 and these
  freezes are dz=0. Different population; do not merge them.
- Whether the freeze is the same bug as the churn. Related-looking is not the
  same as related.
- n=1 unless the freeze reproduces across >=2 colonists in the tape.
