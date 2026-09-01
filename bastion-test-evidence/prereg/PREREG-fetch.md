# PRE-REGISTRATION — why does a colonist holding a fetch reservation stall at dist=0.0?

Written BEFORE the run. Arm: tip + `BASTION_FETCH_DIAG=1`.

## Established already (not being re-tested)

uid 104 claimed farm-sow job 3754 (`required_item=wheat_seeds`,
`carried_amount=0`, `fetch=true`), travelled to the plot, and sat at
`dist=0.0 dz=0 speed=0.0 best_dist=f32::MAX` for 9,300+ ticks while oscillating
1.2 blocks. It never reached `Arrived`: both "job stalled on materials" and
"reverting to fetch leg" are ZERO in the log. `Arrived` is suppressed by design
while a fetch is outstanding.

## The branches, which look different in the log

- **A — PICKUP EMITTED BUT NEVER LANDS.** `FETCH_DIAG pickup emitted` fires
  repeatedly for the same job while `traveling-with-reservation` keeps showing
  `carrying=false`. Mechanism: the pickup event is emitted and does not put the
  item in the inventory. This is the branch the existing comment predicts
  ("stalled empty-handed while every static read of this block says that is
  impossible").
- **B — STEER NEVER ARRIVES AT THE ITEM.** `traveling-with-reservation` fires
  but `pickup emitted` NEVER does. The colonist is steering at a reserved item
  it never gets within 2.8 blocks of. Mechanism is then pathing/steering, not
  pickup — and it would mean `fetch_steer` is set but not being consumed as the
  travel target.
- **C — BLOCK NOT REACHED.** Neither line fires for the stalled colonist. Then
  `job.reservation` is None by that point, or the kind is Haul, and my reading
  of which code path it is on is wrong.
- **D — VOID.** No colonist stalls this run (the stall is not guaranteed
  per-run). Then nothing is learned and I must say so rather than reason from
  the absence.

## The distinguishing question

A and B are separated by ONE line: does `pickup emitted` appear at all for a
job that stays `carrying=false`? A means the pickup is broken; B means the
steering is. They demand opposite fixes, which is exactly why this is being
measured rather than argued.

## What this run CANNOT test

- Whether fixing it makes the town LOOK better. That needs a fresh looking
  sweep afterwards.
- The 99 vertical strandings (rows 0b) — different mechanism, same symptom
  family. Not addressed here.
- n=1. Colony event counts vary 2-3x run to run; the stall either reproduces or
  it does not, and a non-reproduction is branch D, not evidence of a fix.
