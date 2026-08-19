# BANKED WORK QUEUE — work that needs NO VMs

**Rule (Ben, 2026-08-19): a fan launch names its banked item in the same breath.**
Pull the top item at every launch. Refill whenever a row files a successor. An
empty queue is itself a finding — refill from `readme/BUILD-ROADMAP.md`.

## Ready now

1. ~~**Run the `bastion-server` suite in full**~~ **DONE 2026-08-19 — GREEN.**
   `cargo test --profile no_overflow -p bastion-server --all-targets` on
   `76d714daa1`: **135 passed, 0 failed, exit 0**, 69 s. Includes
   `bastion_jobs::tests::haul_skip_starves_a_job_on_its_own_required_item`, so
   the haul-predicate extraction is covered and green.
   **★ Denominator verified, not assumed:** only one "running N tests" line
   appeared, which is the failure mode this item existed to catch — so the
   inventory was checked. `#[test]` fns in source = **135**, exactly the number
   run; the crate has **no** `tests/`, `benches/` or `examples/` dir and
   declares no extra `[[target]]`, so the lib target *is* the whole suite.
2. **`b5_mine_cell_diag` content mover (#84)** — **BLOCKED on data**: no adjacent
   wave pair exists, and wave24 vs wave34 is not comparable (75 vs 119 fields).
   Needs a new wave pair at adjacent commits.
3. ~~**`emit_drop` has no landing-position log**~~ **MIS-SPECIFIED — rewritten
   2026-08-19 after a producer read, not implemented.** The landing position
   **does not exist at `emit_drop`**: the site emits `CreateItemDropEvent` with
   a *velocity* (0.5 horizontal, `2.0..4.0` vertical) and physics decides where
   the entity settles afterwards. A log line at the producer can only ever
   restate the launch vector it already prints. Satisfying the intent requires
   an **observer on the item entity at rest** — a different system and a
   different row. Combined with the queue's own reasoning (0.5 horizontal ⇒
   lands within half a block), the value does not justify that build.
   **Correct successor, if ever wanted:** log position at settle, keyed by the
   drop's item uid, so "scattered out of reach" and "reachable but never
   claimed" become distinguishable — which was the real question behind it.
4. **#110 gate 1** — instrument built, subject extinct at the current tip.
   Re-aim at a trait-pinned reckless population per the roadmap's rider.
   **★ SCOPED 2026-08-19 (producer read, no run): THE PINNING DOES NOT EXIST
   YET — this item is a BUILD, not a re-run.** Personality traits exist
   (`common/src/rtsim.rs::PersonalityTrait`, 16 variants; `Personality::is`
   consumed at `common/src/comp/bastion.rs:550`), but **nothing pins them** —
   no `BASTION_PIN_TRAIT`-style hook anywhere in `bastion-server/` or
   `common/`. ★ And **"reckless" is not among the 16 variants**: the only
   `Reckless` in the tree is `BuffKind::Reckless` (`common/src/cmd.rs:161`), a
   different system. So the rider's wording cannot be taken literally — the
   re-aim needs (a) an env-gated trait-pin hook and (b) a decision about
   **which actual variant** stands in for "reckless" (`Adventurous` is the
   nearest by name, and that is a gameplay-design judgement, i.e. **Ben's**).
   Do not start the build until (b) is answered, or the instrument aims at a
   trait nobody chose.
5. **`ch_cancel_clean` is an unexercised falsifier** — true on all 41 seeds where
   it ran, false only where it could not run. A check that has never gone red
   has never shown it can.
   **★ CONTROL BUILT 2026-08-19 (`bc309249ce`), awaiting a run slot.**
   `BASTION_PLANT_CANCEL_MISS` cancels a region 4096 blocks away instead of the
   tree's AABB — the cancel still RUNS (precondition untouched) but misses, so
   the jobs survive and the predicate must go FALSE. Scorer:
   `score-cancel-falsifier.sh`, outcomes **FALSIFIER LIVE / VACUOUS / MIXED /
   VOID**. ★ It prints `b5_ch_trees` beside the predicate because
   `ch_cancel_clean` is `ch_aabb.is_some_and(..)` — so "cancel missed" and "no
   tree found" both render `false`, and a red scored without the precondition
   would be scoring a coincidence. Runs locally; no VM.
   **★★ CLOSED 2026-08-19 — FALSIFIER LIVE (`1273ec3a33`,
   `CANCEL-FALSIFIER-LIVE.md`).** Baseline **5/5 True** (reproduces the corpus),
   planted **5/5 False**, precondition met on 5 of 5 with 0 excluded, and
   `ch_trees` **identical between arms** (1,1,10,3,1) — so the plant moved the
   cancellation and nothing else. `ch_cancel_clean` is a real assertion; its
   41/41 green is a passing test, not an untested constant. **Transfers to no
   other field** — each unexercised check needs its own red.

6. ~~**★ Colonists claim work headlessly and NEVER ARRIVE.**~~ **CLOSED
   2026-08-19 — REFUTED. See `HEADLESS-ARRIVAL-REFUTED.md` (`21a0f3321a`).**
   A genuinely driverless run (**zero** client connections) arrives fine:
   `server-headless-nd1` 58 claims → **61 arrivals**. Census of all 260 logs
   with claims>0 found **exactly one** zero-arrival run (`wave34/s37`), whose
   own wave-mates arrive normally — an outlier in a matched population, not a
   headless mode. **The cause was instrumented all along:** `auto-access
   refused (no in-claim route) — job unreachable`, emitted **13 times** in the
   run whose note said "the cause is still unnamed". No spend.

7. ~~**★ Why does `auto-access` find no in-claim route on some seeds?**~~
   **ANSWERED 2026-08-19 — bigger than "some seeds". See
   `SELF-RESCUE-NEVER-SUCCEEDS.md` (`6ceb8ece43`).** The `self_rescue` entry
   point to `plan_access` is **0 emissions in 55 calls across 48 seeds — 0%**,
   while the `emergency` entry point to the *same function* succeeds **46 of
   478** in the same runs. Counter pair verified against the emit on 48/48
   seeds. Not a per-seed effect: corpus-wide and total. No spend.

8. ~~**Axis 1 (mask)**~~ **CLOSED 2026-08-19 — NEITHER, mask EXONERATED**
   (`1330233787`, `SELF-RESCUE-NEVER-SUCCEEDS.md`). `BASTION_SELFRESCUE_BUBBLE`
   gave `self_rescue` the emergency site's exact bubble geometry: **36 calls, 0
   emissions**, witness confirming the bubble branch on **36 of 36**. Call
   counts reproduced banked wave34 exactly (37→11, 3→7, 29→6, 16→6, 11→6).
   **Within-run positive control:** on seeds 3/29/16 the *same run* got `Some`
   from `plan_access` via `emergency` (27 calls, 4 emissions) and `None` via
   `self_rescue` every time — so the failure is specific to the CALL SITE.
   ★ **Run 1 of this was VOID, not NEITHER** — one field differed across the
   whole payload (`b5_soak_avg_tick_ms`, wall-clock noise), so "bubble used,
   changed nothing" and "flag never arrived" were the same evidence. The probe
   had shipped without a witness. **A probe is born with its witness.**

9. **IN FLIGHT — axes 2 and 3, both built, ONE build, FOUR arms.** Two
   differences remain: `emergency_owner` and `emergency_approach`.
   `BASTION_SELFRESCUE_CTX` passes the requesting colonist's uid;
   `BASTION_SELFRESCUE_APPROACH` passes the approach context. `carve_requests`
   carries `Option<Uid>` and `Option<(Vec3<f32>,(f32,f32,f32))>`, both built at
   the **push** site where `pos`/`entity`/`scales`/`colliders` are already in
   hand — the 3→5-tuple change the producer read predicted, not the deep
   plumbing I first claimed. Both **born with witnesses**, both handling the
   third state (*flag on, nothing to pass*) as its own named line rather than a
   silent untreated arm. Scorer `score-selfrescue-axes23.sh`, arms
   A/B/C/D = baseline/owner/approach/both, registered outcomes **CONTEXT /
   APPROACH / EITHER / BOTH-REQUIRED / NEITHER / VOID**. Runs locally, no VM.
   ★ Batched deliberately: three rebuilds today cost ~30 min and two were
   avoidable, so axes 2 and 3 ride a single build and a single run.

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
