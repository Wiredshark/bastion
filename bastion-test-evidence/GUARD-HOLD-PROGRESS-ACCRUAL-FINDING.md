# A HELD GUARD KEEPS ACCRUING WORK PROGRESS — and repels the colony from its own post

**Status: SOURCE-READ, NOT MEASURED.** Filed as the second-order consequence of
the ITEM 14 guard-post fix
([ITEM14-GUARD-MOOT-FINDING.md](ITEM14-GUARD-MOOT-FINDING.md)). Before that
fix the effect could not exist, because no guard job survived past ~6 seconds.

## The shape

A guard job has no arm of its own in `ActiveJobState::Arrived`. It therefore
falls through the **generic** work path every tick it stands its post:

- `job.progress += dur_mult * work_progress(dt, skill_level, job.work, tool)`
  runs unconditionally, with no upper bound. The threshold check
  (`if job.progress < threshold { continue }`) only gates what comes *after*;
  it never clamps the value.
- Past threshold the job traverses the entire completion tail every tick —
  `can_set_block`, the Build occupancy check, the chop-felling lookup, the
  `required_item` consume, the Cook/Trade/Tend arm — before
  `completion_block(Guard) == None` finally yields `continue`. That is the
  mechanism by which "it holds" currently works: by falling off the end of
  machinery that has nothing to do with guarding.

Three consequences follow, in increasing order of importance.

## 1. The inspector shows a full work bar for a colonist standing watch

```rust
arb.activity = Some((job.work, (job.progress / threshold).clamp(0.0, 1.0)));
```

Clamped, so a guard reads as a permanently 100%-complete work job in the UI-4
inspector. Cosmetic, but it makes a standing guard indistinguishable from a
wedged worker at a glance — the opposite of what an inspector is for.

## 2. `job.progress` grows without bound

f32, ~0.0056 per tick at level 0, so ~600 after an hour of held post. No
overflow risk at any realistic session length; noted because it makes
`progress` meaningless as a diagnostic field for this kind, and because it is
a concrete instance of the class already registered in-file as unverified
("`job.progress` can tick on a claim the colonist can't service — on v5's watch
list").

## 3. THE ONE THAT MATTERS — a stationary job poisons the stigmergic field

The coordination field deposits per Arrived colonist per cycle:

```rust
for (_colonist, active) in (&colonists, &active_jobs).join() {
    if matches!(active.state, ActiveJobState::Arrived)
        && let Some(job) = board.jobs.get(&active.job)
    {
        *board.saturation.entry(coord_cell(job.pos)).or_insert(0.0) += COORD_DEPOSIT;
    }
}
```

A held guard is `Arrived` **forever**, so its `COORD_CELL`-sized (4-block) cell
accumulates deposit every cycle with no completion to ever release it.
Equilibrium is `COORD_DEPOSIT / (1 - COORD_DECAY)` = 1.0 / 0.05 = **20**, and
`× COORD_SAT_WEIGHT` (0.75) = **~15 score units of repel** — which the tuning
comment itself describes as "enough to out-pull a modest distance difference".

The field exists to spread workers over *work*. A guard consumes no work; it
occupies a place. So the post's own cell becomes maximally unattractive to
every other job claim, permanently, and the colony quietly stops working near
the thing it posted a guard to protect. That is a colony-wide allocation effect
produced by a stationary job, and it would be very hard to attribute from the
outside — the symptom is "nobody works near the guard post", with nothing in
any log naming the guard as the cause.

It also inverts the intent twice over: a guard post is placed at somewhere
valuable, and the effect is to push labour away from it.

## Falsifier

Registered prediction, both branches, measured per cell rather than in
aggregate:

- **If the effect is live:** with one held guard and no other work in its cell,
  `board.saturation` for `coord_cell(post)` climbs to ~20 and stays; claim
  allocation within that cell drops measurably against a matched control
  colony with no guard posted.
- **If it is not:** saturation for that cell decays to the 0.05 retain floor
  like any other, and allocation is unchanged.

The control must match on colony size and on work available in that cell —
otherwise "nobody worked there" is explained by there being nothing to do.

## Proposed fix

Give `Guard` its own arm in `ActiveJobState::Arrived`, **before** the progress
accumulation — the pattern Farm and Gather already use — that holds the post
and `continue`s. That removes all three consequences at once: no accrual, no
traversal of the completion tail, and the saturation deposit can then be
skipped for a job kind that consumes no work.

Whether a guard should deposit *something* into the field (a "this place is
covered" signal is arguably useful) is a design question, not a defect. The
defect is depositing the **work-saturation** signal, which means the opposite.
