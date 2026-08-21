# A FAILED TERRAIN READ MAKES EVERY JOB MOOT — including the five kinds documented as having none

**Status: SOURCE-READ, NOT MEASURED.** No live trigger was established. Filed
because the failure *direction* is "destroy the work", which is the same shape
as the ITEM 14 guard-post defect
([ITEM14-GUARD-MOOT-FINDING.md](ITEM14-GUARD-MOOT-FINDING.md)) that was just
fixed one line away from it.

## The shape

`bastion-server/src/bastion_jobs.rs`, the completion moot check inside
`ActiveJobState::Arrived`:

```rust
let completed_kind = terrain.get(job.pos).ok().map(|b| b.kind());
let still_valid = completed_kind.is_some_and(|k| match job.kind {
    JobKind::Cook { .. } => true,
    JobKind::TradeMission { .. } => true,
    JobKind::Tend { .. } => true,
    JobKind::Guard { .. } => true,
    JobKind::Designated(GuardPost | PatrolPoint) => true,
    …
});
if !still_valid { /* remove_job + release */ }
```

The per-kind match is the intended rule. But the whole match is **gated on a
successful terrain read**: `is_some_and` yields `false` when `completed_kind`
is `None`, so if `terrain.get(job.pos)` errors, `still_valid` is `false` for
**every** kind — the `=> true` arms never run.

Five kinds carry an explicit in-file comment saying they have no terrain
precondition: `Cook`, `TradeMission`, `Tend`, `Guard`, and
`Designated(GuardPost | PatrolPoint)`. For those five the gate is not a
conservative guard, it is a contradiction: the code says "this kind does not
depend on the block" and then destroys the job because the block could not be
read.

## Why the direction matters

The other kinds fail safe here. A `Mine` job whose block cannot be read
*should* not complete — dropping it forbids conjuring a drop out of air. For
the no-precondition kinds the same `false` means "delete an assignment that had
nothing to check", and nothing re-mints a guard assignment or a trade mission,
so the loss is permanent rather than retried.

## What is NOT established

- **No live trigger was found.** A colonist must be `Arrived` to reach this
  line, and its own chunk is loaded, so the post cell is normally readable.
  The arrival tolerance can stretch to ~6 blocks via `stuck_strikes`, and
  `TradeMission` targets remote site positions, but neither was shown to
  produce a failing read.
- No log line distinguishes "read failed" from "predicate said no" — the moot
  emit prints the same `target block changed under it` text for both. **A read
  failure is currently indistinguishable from a legitimate moot**, which is why
  the absence of evidence here is not evidence of absence.

## Prior suspicion

An in-file note near the stuck-watch reset already registers `completed_kind`
as unverified and "on v5's watch list", alongside the separate open question of
`job.progress` ticking on a claim the colonist cannot service.

## Falsifier

Force `terrain.get` to fail at the position of an `Arrived` job of each
no-precondition kind and observe whether the job survives. Registered
prediction, both branches:

- **If the gate is live:** the job is removed and the moot line prints with a
  no-precondition `kind=`.
- **If it is unreachable:** no such emit occurs in any run, and the finding
  downgrades to a shape cleanup.

Before running that, add the discriminating field the instrument currently
lacks — print `terrain_read_ok` beside `kind` on the moot emit — or the two
branches render identically and the result is VOID.

## Proposed fix

Hoist the no-precondition kinds out of the terrain-read gate, so the read
governs only the predicates that actually consult a block:

```rust
let no_terrain_precondition = matches!(job.kind, /* the five kinds */);
let still_valid = no_terrain_precondition
    || completed_kind.is_some_and(|k| match job.kind { … });
```
