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

8. **IN FLIGHT 2026-08-19 — axis 1 (mask) built, first run VOID, re-running
   with a witness.** Probe `BASTION_SELFRESCUE_BUBBLE` swaps
   `designated_regions()` for the emergency site's bubble geometry
   (`6d87fef84d`), runs **locally** via `bastion-harness --b5-scenario` — no
   VM, the row was chartered as a fan and is a laptop job.
   **Run 1 reproduced the banked corpus exactly** (calls 37→11, 3→7, 29→6,
   16→6, 11→6, matching wave34) with **emissions 0 in BOTH arms** — which
   *looks* like NEITHER but is **VOID**: exactly one field differed across the
   whole payload (`b5_soak_avg_tick_ms`, wall-clock noise), so "bubble used,
   changed nothing" and "flag never arrived" were indistinguishable. Witness
   added (`self_rescue_bubble_active` counter + emit); a NEITHER verdict is
   only meaningful once that counter is nonzero.

9. **★ AXIS 1 DONE (mask ELIMINATED). Axes 2–3 remain — and they are CHEAPER
   than I said.** Axis 1 scored **NEITHER** on 2026-08-19 (`1330233787`): 36
   calls, **0 emissions in both arms**, with the witness confirming the bubble
   branch on **36 of 36** calls. `designated_regions()` is exonerated; the
   leading hypothesis is refuted.
   **★ Correction to my own cost estimate:** I wrote that axes 2–3 need a
   claimant uid "not in scope — plumbing, not a flag". **Read the producer:**
   the request is pushed at `bastion_jobs.rs:14483` as
   `carve_requests.push((feet, job.pos, active.job))` from **inside the
   per-entity loop**, where `entity` is already in scope (used at 14262–14286).
   So it is a **3-tuple → 4-tuple change** carrying `uids.get(entity)`, then
   `Some(uid)` at the call site. Small, not deep.
   Registered outcomes **CONTEXT / BOTH-REQUIRED / NEITHER**; witness pattern,
   scorer and falsifier all already exist. Runs locally.

10. **★ Isolate WHICH of the three call-site axes blocks `self_rescue`** — the
   two live sites differ on mask (`designated_regions()` vs a synthesised
   egress bubble), `emergency_owner`, and `emergency_approach` **all at once**,
   so the emergency site proves `plan_access` *can* succeed but isolates
   nothing. Vary one axis at a time at the self_rescue site; score on the
   existing verified counter pair. Registered outcomes **MASK / CONTEXT /
   BOTH-REQUIRED / NEITHER**. **Cost: one seed, no fan** (s37 has 11 calls,
   s3 has 7, s29 has 6). Not chartered.

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
