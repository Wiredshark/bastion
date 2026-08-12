# EXTERNAL ADVERSARIAL LANE — FINDINGS TRIAGE (verified, not accepted)

**Source:** ChatGPT web/Codex, first outing (roadmap 2026-08-12 (8)):
"PARTIAL — REQUEST CHANGES; F8 remains PARTIAL", 4 findings.
**Triage:** Builder Opus 5c, 2026-08-12, by independent read of the tree.
**Ground rule 1 governs: external findings are CLAIMS until our own read
confirms them.** Every disposition below cites what was read.

**Where the code is:** `bastion/wip-batch-verify @ac8ca746d0`, worktree
`.engine-integration-wt`, `bastion-server/src/bastion_jobs.rs` unless
stated. This branch is NOT `bastion/block-B6HAUL` (where this document
and the roadmap live) — `block-B6HAUL` has no `completion_outcome` at
all. The merge is its own step and its own risk.

---

## FINDING 1 (HIGH) — "the F8 fixture proves the HELPER, not the PATH"

**DISPOSITION: CONFIRMED. Survives `ac8ca746d0` unchanged — the split
WIDENED the untested surface rather than narrowing it.**

All four F8 tests (`:20569-20651`) call `completion_outcome(...)`
directly with a literal first argument. Nothing in the crate constructs
the caller: `bastion-server` has no `tests/` directory, and the live
call site sits inside a specs `System::run` whose SystemData tuple runs
to ~40 storages (`:6590`). So the tests certify the mapping
`bool → CompletionOutcome` and nothing about the arm that computes the
bool or consumes the struct.

**The external reviewer's "five ways to stay green", re-derived
independently against the current tree (not copied from the report) —
every one of these mutations leaves 94/94 green:**

1. `:11507` — `let is_emergency_access = board.emergency_access_jobs
   .contains_key(&active.job)` replaced by `false` (or an inverted/wrong
   lookup). **The polarity computation has no test at any tier.**
2. `:14915` — the call site passes a literal `false` instead of the
   computed flag. Same green.
3. `:14916` — the `if let Some(item_id) = outcome.drop_item` block
   deleted, or the drop emitted unconditionally. No test observes a live
   drop.
4. `:14940-14959` — the two `log_channel` match arms swapped: real
   production logs to the honest line and phantom completions log to
   `"bastion: job completed"`. **This is v4's exact lie, reachable
   through green tests.**
5. `:15069-15074` — `reset_completion_watch(..., outcome
   .reset_stuck_watch, ...)` changed to `true`. **This re-introduces the
   precise defect the row was opened to close** (the disarmed
   ultimate fail-safe, roadmap 2026-08-11 (98)) and every test stays
   green.

A sixth, worse than any of the five: **the whole generic completion arm
could be unreachable and nothing would fail.** That is the reachability
question F8 has always been about.

**Correction the split does earn:** because drop/XP/log/watch now read
ONE struct, a single caller-tier test would cover four consumers at
once. The unification raised the value of the missing test; it did not
substitute for it.

**Disposal:**
- **F8 stays PARTIAL.** The scorer's original refusal #2 was right and
  the external lane agrees with it.
- **The closer is the live run, and it is already scheduled**: the
  founding-preset acceptance on the resourced arena (real trees/stone ⇒
  real chop/mine completions through the generic path) with
  `bastion: job completed` observed carrying drop + XP. The convergence
  in the packet's §5 F8-INCLUSION row **stands, and is now this
  finding's named exit condition.**
- **Consequence for the packet:** this makes the founding-preset run
  load-bearing for two rows, which raises the cost of B7 (binary
  provenance) being missing from it. Filed as blocking there.

---

## FINDING 2 (MED) — "the red demo mutated INSIDE the helper; the wiring
has no falsifier of its own"

**DISPOSITION: CONFIRMED as a gap. MIS-SCOPED as a fix — §6's
"integration-tier fixture" is not buildable as written.**

The gap is real and is just finding 1's mutations 2-5 restated at the
falsifier tier: the red demonstration was a shape-revert inside a pure
function, so it demonstrates the function, not the wiring.

What §6 asks for — "a polarity-flip at the LIVE call site must fail an
integration-tier fixture" — requires standing up an ECS world with the
full ~40-storage SystemData (`:6590`), terrain, a board with an active
job, a colonist entity, event emitters. That is a subsystem harness,
not a fixture, and nothing like it exists in this crate today.

**Affordable form (recommended, and honest about what it buys):**
extract the consumer application one level up —
`apply_completion_outcome(outcome, &mut sinks)` returning/recording the
four effects (drop request, XP grant, log channel, watch reset) — and
have the live arm call it, exactly as the arm now calls
`completion_outcome`. A unit test then kills mutations **3, 4 and 5**,
and the polarity-flip test at that boundary kills **2**.

**What it does NOT buy, stated plainly:** mutation **1** (the flag's own
computation at `:11507`) and reachability. Those close only in a live
run. **No fixture at any tier closes F8's inclusion half — that was
finding 1's true hit and it survives this fix.**

**Disposal: ACCEPTED with the scope correction above.** §6's wording is
amended by the packet's §8; the row is the signal-split row, which is
already open.

---

## FINDING 3 (MED) — "'suppresses every world-effect' asserts drop/XP/log
only; cave-in is gated elsewhere"

**DISPOSITION: CONFIRMED — and it UPGRADES from a naming defect to a
live re-derivation the signal-split row believes it eliminated.**

Read at `:14994`: the cave-in block is gated by its own
`if !is_emergency_access && job.kind.is(DesignationKind::Mine)` — a
fourth independent derivation of the same predicate, sitting eleven
lines below the comment at `:15058-15065` that says this site now reads
`outcome.reset_stuck_watch` "rather than re-deriving
`!is_emergency_access` a third time at this site". The emit string at
`:14957` tells the operator cave-in is "suppressed by design"; the
suppression is true, but it is true by a duplicate condition, not by the
origin decision. **The row's own thesis — one origin, no parallel
derivation — is not yet satisfied at this arm.**

**Disposal: FIX BY FIELD, not by rename.** Add the cave-in gate to
`CompletionOutcome` (e.g. `world_mutation: bool` / `cave_in: bool`,
`had_effect`-derived like the others at `:759-769`), have `:14994` read
it, and the existing test name becomes true instead of being retitled to
a smaller claim. A rename plus a separate cave-in regression test (§6's
alternative) leaves the duplicate predicate alive and is the weaker
half. **The consumer enumeration should be re-walked once for any fifth
site: this one was found by reading the arm top-to-bottom, not by the
enumeration.**

---

## FINDING 4 — "no CI evidence"

**DISPOSITION: ACKNOWLEDGED, NOT A FINDING.** Known standing condition
of this program, not a defect discovered by review; no row moves. Worth
recording that the external lane surfaced it independently — that is
evidence the condition is visible from outside, which is the argument
for the lane, not against the program.

---

## DEFECT-1 — restated CLOSED

Unchanged by this triage and by the restart: **CLOSED, premise-void.**
The save-read the row turned on is impossible — the b5 instrument is
harness-only, so the premise the read depended on never held. Consistent
with the ledgered finding. Nothing in findings 1-4 touches it.

---

## SUMMARY

| # | claim | our read | disposal |
|---|---|---|---|
| 1 | fixture proves helper not path | CONFIRMED, 5+1 green-survivals re-derived | F8 stays PARTIAL; live founding-preset run is the named closer |
| 2 | wiring needs its own falsifier | CONFIRMED; §6's tier is unbuildable | accepted with scope correction (extract one level up; 4 of 5 mutations) |
| 3 | "every world-effect" overclaims | CONFIRMED + UPGRADED (live re-derivation at `:14994`) | fix by field on `CompletionOutcome`, not by rename |
| 4 | no CI evidence | true, known | acknowledged; no row |

**The lane's scoring holds: PARTIAL was the right verdict, and it agreed
with our own scorer's refusal. Its one genuinely new fact — that a
fixture cannot substitute for a real completion — is now a scheduled
run, not an open argument.**
