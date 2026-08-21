# ★ THE GUARD POSTS WERE NOT INVALIDATED — they were unconditionally destroyed

**A one-arm inversion where the comment is right and the code returns its
opposite.** Player-painted guard posts were dropped ~3–6 seconds after a
colonist reached them, on every build. Defence was impossible for the whole
adopt-town play session of 2026-08-21.

## The symptom

Two posts painted, two colonists walked to them, both jobs gone within seconds
of arrival:

```
18:39:28  colonist arrived at job site, working (B5)  job=167 colonist=156 kind=Guard{…}
18:39:34  job moot — target block changed under it; dropped  job=167          (6 s)
18:40:51  colonist arrived at job site, working (B5)  job=166 colonist=155 kind=Guard{…}
18:40:54  job moot — target block changed under it; dropped  job=166          (3 s)
```

Whole-session counts: `ITEM 14 guard assignment created` = **2**;
`xp granted work=Guard` = **0** (vs Farm 112, Cook 33, Build 14). Meanwhile
9 × `colony drive TRANSITION … to=Defend deciding="threats"` and 22 colonist
preempts into `drive=Flee`. Real threats, no guards.

## The hypothesis that was wrong

The reported hypothesis was that the moot check compares the post cell against
a block snapshot taken when the job was minted, so a colonist standing on the
cell invalidates the job precisely when it succeeds.

**There is no snapshot.** `still_valid` is a per-kind predicate re-evaluated at
completion; nothing in it remembers a prior block, and nothing consults
occupancy. Recording this because the hypothesis is plausible, fits every
observation, and is false — a fitting story is the weakest evidence.

## The actual cause

`bastion-server/src/bastion_jobs.rs`, the moot check inside
`ActiveJobState::Arrived`:

```rust
// Self-jobs complete in their own arms above — defensive.
// ITEM 14: a guard assignment has no terrain
// precondition to re-check, exactly like the self-jobs below it.
common::bastion::JobKind::Guard { .. }
| common::bastion::JobKind::Haul { .. }
| …
| common::bastion::JobKind::Recreate { .. } => false,
```

The comment states the correct rule. The code returns its opposite: `false`
means **always moot**. The posts were not invalidated by anything in the world.

`false` is safe for `Haul` / `DepositRun` / `RestAt` / `EatFrom` / `Despond` /
`Recreate` **only because each completes in its own arm above and never reaches
that line** — their `false` is unreachable defensiveness. `Guard` has no such
arm (there is no `JobKind::Guard` anywhere between the `Arrived` head and the
moot check), so it accrues the generic `job.progress` at its post and lands
there at threshold.

This is the most expensive shape a one-line defect can take: a reader who
checks the intent finds a comment that is correct, and moves on.

## Why the timing confirms it rather than merely fits it

`work_tool_kind(Guard) → None`, so `tool_factor` = 1.0 and `dur_mult` = 1.0
(no tool). With `work_rate(level) = (1 + 0.2·level) / 6.0`, time from arrival
to destruction is `6.0 / (1 + 0.2·level)` seconds:

| level | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| seconds | **6.00** | 5.00 | 4.29 | 3.75 | 3.33 | **3.00** |

The session logged **6 s** and **3 s** — the two ends of the range. A derived
prediction meeting measurement, not a story chosen to fit the numbers.

## Why it was permanent, and why defence failed

Guard assignments are minted **only** by the paint action: `place_designation`
returns `vec![id]` before the per-cell loop, and no regeneration pass exists.
So `board.remove_job` destroys the assignment forever — 2 created, 0 recreated.
The painted region stays in `board.designated` (the moot path never pushes
`done_regions`), leaving a zombie outline with no job behind it.

Both ITEM 14 consumers gate on `guarding`, which requires
`board.jobs.get(&aj.job)` to still resolve to a `Guard` job. Once the job is
gone: no flee suppression (axis 2), no mode response (axis 1). That is the 22
Flee preempts across 9 Defend transitions.

## The fix

Move `JobKind::Guard { .. }` into the `true` group. No new machinery is
required: `completion_block(Guard) == None` already yields `continue` each
tick, which **is** the documented hold — the creation site states the contract
outright, *"A guard consumes nothing and completes nothing: it holds."*

Five sibling ITEM 14 sites already encode that exemption correctly —
`job_wanted`, `job_still_wanted`, `designation_affordance`, `completion_block`,
and the `Designated(GuardPost | PatrolPoint) => true` arm **inside this very
match**. The `JobKind::Guard` arm was the single dissenter.

Answering the question as posed: guard jobs should be **exempt**, not compared
against a stance-appropriate predicate. A guard post has no predicate to
compare against — that is what "no terrain precondition" means.

## Scope

The defective block is byte-identical (md5 `83b7cc3da40023652a2fc6c610fd375c`)
at `2b447bcde5` (the build that produced the session above), at `d52a637a7c`,
and at `7244035db6`. Not a regression in a recent commit; live since ITEM 14
landed.

## No test asserted it

Nothing in the tree asserts that a Guard job survives to hold; ITEM 14 was
accepted on play/harness scoring alone. A planted test that arrives a colonist
at a post and asserts the job is still on the board N seconds later would have
caught this on the day it landed, and is the real remedy — the guard arm is one
line, the missing assertion is the defect class.

## Adjacent, filed separately

- [MOOT-TERRAIN-READ-GATE-FINDING.md](MOOT-TERRAIN-READ-GATE-FINDING.md) — the
  same check drops **every** kind when the terrain read fails, including the
  five documented as having no terrain precondition.
- [GUARD-HOLD-PROGRESS-ACCRUAL-FINDING.md](GUARD-HOLD-PROGRESS-ACCRUAL-FINDING.md)
  — now that guards survive, a held post accrues progress forever and saturates
  the stigmergic field in its own cell.
