# BANKED WORK QUEUE — work that needs NO VMs

**Rule (Ben, 2026-08-19): a fan launch names its banked item in the same breath.**
Pull the top item at every launch. Refill whenever a row files a successor. An
empty queue is itself a finding — refill from `readme/BUILD-ROADMAP.md`.

## Ready now

1. **Run the `bastion-server` suite in full** once the `--all-targets` build
   lands, to confirm the haul-predicate extraction broke nothing. ★ A filtered
   `cargo test <name>` cannot see a guard in a crate it never compiles — the
   whole suite has to run.
2. **`b5_mine_cell_diag` content mover (#84)** — **BLOCKED on data**: no adjacent
   wave pair exists, and wave24 vs wave34 is not comparable (75 vs 119 fields).
   Needs a new wave pair at adjacent commits.
3. **`emit_drop` has no landing-position log** — the toss witness records the
   launch vector only. Low priority: the toss is 0.5 horizontal, so items land
   within half a block and the landing site is unlikely to matter.
4. **#110 gate 1** — instrument built, subject extinct at the current tip.
   Re-aim at a trait-pinned reckless population per the roadmap's rider.
5. **`ch_cancel_clean` is an unexercised falsifier** — true on all 41 seeds where
   it ran, false only where it could not run. A check that has never gone red
   has never shown it can.

6. **★ Colonists claim work headlessly and NEVER ARRIVE.** In a driverless run:
   **93 jobs claimed, `colonist arrived at job site` = 0**, across 7,112 ticks.
   The same emit fires **985–1,009** times in driven endurance runs. Candidate:
   pathing depends on terrain outside the presence radius, so a VD=1 colony can
   claim work it cannot reach. **The VD=6 re-run tests that for free** — if
   arrivals appear at VD=6, the dependency is confirmed and the cause is named.

## Blocked / needs a decision (Ben)

- **Tick-loading scope call** — roadmap criterion passes, mandate bar 2 fails.
- **Run-gait trigger** — `running = true` appears nowhere; 2 of 4 status
  variants never observed in 33,926 samples.

## Done from this queue today

**Item 1 — three field ties resolved.** All shared-precondition ties, not
duplicate measurements.

**Item 2 — the refusal census conflates blocks with correct skips.** Corrected my
own figure *against* me: 100% of genuinely-blocked jobs are `materials`, not 79%.

**★★ Item 2 led to the session's largest finding: THE HAUL DEADLOCK.** A blocked
job's own presence prevents the haul that would unblock it. Four job kinds have
the shape; it breaks a contract the code documents elsewhere; no test guarded it;
and **it closes #114**, whose origin run *is* a deadlocked run and whose bar was
calibrated on that run's own maturation count.

Plus: wave field-independence audit · status-surface census · bimodality
mechanism · memory-index defect (10 live rules filed as retired) · speed-lever
corrections · `fan-shape-check.sh` · `score-hauldeadlock.sh`

★ None of it needed a VM.
