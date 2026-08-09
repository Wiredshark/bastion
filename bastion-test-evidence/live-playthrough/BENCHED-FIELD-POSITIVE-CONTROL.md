# `benched_since_tick`/`benched_until_tick` positive control (2026-08-09)

`#56` ported both fields into `ch_job_diag`/`build_job_diag`, but every
observation across 7 entries (seeds 80, 92, and the default 3-seed corpus)
was null on both fields — indistinguishable between "genuinely not
benched" and "the field never fires in this scenario" (the structural
mute mode). This is the planted positive control that clears it, per
Opus's framing: a field with no known-positive can't certify a null, and
every future wave's nulls inherit that ambiguity until one exists.

## Setup

Live server-cli, `BASTION_ROWB_BENCH=1` (so both fields could populate),
1 colonist spawned, a 1x1x1 Mine designation placed 199 blocks below the
colony's working depth (`[15213,16013,220]`, colony operates around
z=410-420), confirmed solid via `survey` before designating (49 columns,
0 with no surface in range, 0 overhang candidates — no cavity, no
adjacent open cell for `job_stance` to find).

## Precondition, confirmed via proxy observables (not assumed)

`benched_since`'s guard requires: unclaimed, affordance != Untargeted,
missing from `standable`. The third isn't directly inspectable, so per
Opus's ask the nearest observable proxies were used and are reported
explicitly: `claimant: None` (confirmed) and `stuck_strikes: 0` +
`unreachable: true` (confirmed) — zero stuck strikes means the job was
never even attempted, consistent with "no stance ever existed to attempt
it from," not "attempted and failed."

## Result

    inspect_cell checkpoint 1 (~7s post-designate):  None (job not yet
      registered by the board sweep at this cadence)
    inspect_cell checkpoint 2 (~20s post-designate):
      claimant: None, unreachable: true, stuck_strikes: 0,
      blocked_by: None,
      benched_since_tick: Some(1845),
      benched_until_tick: None

**`benched_since_tick` fired.** The field demonstrably populates under
the documented precondition. `benched_until_tick` stayed null — not a
contradiction: its own gate (`stuck_strikes >= PERSIST_ESCALATE_STRIKES`)
requires prior claim-and-stuck-attempts, a population `since`'s guard
doesn't require and this fixture never produced (`stuck_strikes: 0`).
The two fields cover genuinely different populations (never-attempted vs.
attempted-and-repeatedly-stuck), not the same event observed two ways —
this run is evidence for `since` specifically, silence (not refutation)
on `until`.

`unreachable: true` appearing alongside a benched job is not a
contradiction either: that flag comes from the planner's own pathfinding
attempt, a separate mechanism from the benching guard's bookkeeping. The
doc's claim ("a benched job is deliberately not flagged unreachable")
describes what the *benching guard itself* does to that field, not a
guarantee that no other mechanism ever sets it independently.

## What this settles

The structural mute mode is cleared for `benched_since_tick`. Combined
with the clean 7-of-7 null result across the corpus and the two
build-family seeds (80, 92) — cells that genuinely aren't in the benched
state — the earlier nulls are now positive evidence, not ambiguous
silence. **`#56`'s original question: 6-of-6 now stands on measurement**
("the second build job is healthy on every seed"), not 4-of-6 measured
plus 2 consistent-but-unproven.

`benched_until_tick` remains unproven by this control (silence, not a
finding) — it would need a fixture that gets a job claimed and stuck
repeatedly rather than never-reachable, a different setup than this one.
