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
2. ~~**`b5_mine_cell_diag` content mover (#84)**~~ **ANSWERED 2026-08-19 from
   banked data — `30047a0593`, `ITEM2-CONTENT-MOVER-IS-PLAN-ACCESS.md`.**
   The mover is **`plan_access`**: 120 of 126 blocked-cell entries (**95%**),
   `route_exhausted` 3 (2%), 46% additionally flagged unreachable. Same subject
   as the self_rescue thread — two independent lines converged on one function.
   ★ **This entry carried TWO stale states before the answer** — first "BLOCKED
   on data", then "PRODUCIBLE, ~40 min of local CPU". **Neither was true.** The
   corpus already held it, and the ~40 min was nearly spent before the
   corpus-first check was run. **Zero spend.**

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
4. **#110 gate 1** — **UNBLOCKED 2026-08-19 by Ben's ruling 6: the proxy is
   `Adventurous`.** The pin itself was already built generic over all 16
   variants (`BASTION_PIN_TRAIT`, `e4808a8976`), so the ruling is a flag value,
   not a code change: `BASTION_PIN_TRAIT=Adventurous`.
   ★ **Ben's caveat, carried verbatim into the row rather than paraphrased:**
   *"the desires/aspirations charter may later add an explicit risk-tolerance
   axis that supersedes this proxy — if Adventurous behaves as
   exploration-seeking rather than carelessness in the data, say so rather than
   forcing it."*
   **So the row owes a behavioural check before it owes a result:** confirm the
   pinned population is actually trap-prone, and report it if it is merely
   well-travelled. A proxy that is never checked against the behaviour it
   proxies is an assumption wearing a measurement's clothes.

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

9. ~~**Axes 2 and 3**~~ **CLOSED 2026-08-19 — BOTH-REQUIRED** (`5467e2f833`,
   `SELF-RESCUE-NEVER-SUCCEEDS.md`). Four arms x 5 seeds, local, zero spend:
   baseline **0/36**, owner-only **0/36**, approach-only **0/36**, **both
   5/38**. Against 0/55 corpus-wide, `self_rescue` **succeeded for the first
   time**. Mechanism matches the code exactly: `emergency_owner.is_some()` is
   the only live disjunct opening the ladder tier for this caller, and
   `emergency_approach` is consumed only inside that arm by `ladder_pillar` —
   so the call site was missing two arguments its sibling passes and could
   never have succeeded as written.
   ★ **Registered predictions scored:** arm C ≡ arm A **CONFIRMED** (the
   falsifiable one); "owner alone will move it" **REFUTED** — I read that owner
   opens the tier and wrongly concluded it was sufficient. *Opening a path is
   not traversing it.*
   ★ **Two limits recorded, not buried:** denominators are not fixed (arm D
   changed call counts in exactly the seeds that emitted, 6→7/6→9/6→4), and
   **seeds 37 and 3 still emit zero with both arguments** — necessary, not
   sufficient.

10. **★ NEW: why do seeds 37 and 3 still refuse with BOTH arguments?** The
   residual from item 9. 37 has 11 calls, 3 has 7, both 0 emissions in arm D
   while 29/16/11 emit. Everything needed already exists — the four-arm scorer,
   both probes, their witnesses, and a within-run positive control
   (`emergency` succeeds in the same runs). **Next step is a consumption-site
   witness inside `plan_access`**, not another call-site flag: the call-site
   witness proved delivery and told us nothing about which internal branch
   refuses. Runs locally, no VM. Not chartered.

   **★ IN FLIGHT 2026-08-19** — `BASTION_PLAN_ACCESS_DIAG` built (default OFF)
   and running on seeds **37, 3** (the refusers) plus **29** as a positive
   control that DOES emit, all with both axes on. It separates three outcomes
   that previously all rendered as a bare `None`: `ladder_pillar` returning
   none (logging `owner_present`/`approach_present`), the silent
   `emergency_approach?` early exit in the escape-shaft fallback, and the
   owner-gated `emergency_reengage_exhausted` refusal — the counter-prediction
   registered before the axes-2/3 run.
   **★★ LOCALIZED 2026-08-19 (`e06e66a62a`, `ITEM10-SHAFT-LOCALIZED.md`).**
   Two candidates **DEAD**: `emergency_reengage_exhausted` fired **0** times on
   all 3 seeds (my registered counter-prediction — true of the code, false of
   these seeds), and the shaft was never skipped for a missing approach (**0**).
   Accounting is exact (19=19, 19=17+2, 27=27), so attribution survives the
   diag's missing caller tag. **`ladder_pillar` succeeds 2 times in 65 calls,
   both to the `emergency` caller**; `emergency_escape_shaft` is the workhorse
   and returns `None` every time on 37/3 while succeeding 3× on 29. A
   tall-climb hypothesis was measured and **dropped** (emitting seed's median
   span 59 = refusing seed's 59). **Residual now localised to
   `emergency_escape_shaft`.**

11. **★ NEW: why does `emergency_escape_shaft` refuse on seeds 37 and 3?** The
   successor to item 10, one level deeper. Needs a witness **inside** the shaft
   naming which of its own preconditions fails, **and a caller tag** — item
   10's run escaped the mixed-caller confound only because the arithmetic
   happened to close, which is luck, not design. Runs locally, no VM.
   **★★ CLOSED 2026-08-19 (`8fb2bb94a7`, `ITEM11-SHAFT-CAUSE.md`).**
   `cells.is_empty()` dominates on every seed — **1928 of 2026** rejections on
   seed 37 (~95%), i.e. essentially every one of the 121 candidate columns.
   The shaft finds **no mineable column within radius 5**: the span is already
   open air, so there is no rock to carve. **`rej_no_approach = 0` everywhere**,
   which explains by measurement why the pre-existing `EGRESS_DIAG` was silent.
   My registered `rej_no_wall` candidate fires (882 on seed 3) but is only 56 on
   seed 37 — real, secondary, **demoted**. Refusing to carve where there is
   nothing to carve is CORRECT, so this is a terrain/position fact, not a logic
   defect. **What the colonist should do instead is a DESIGN question.**

## The self_rescue chain, complete and every link measured

`self_rescue` **0/55** corpus-wide → the call site was missing two arguments its
sibling passes (**BOTH-REQUIRED**: 0/36 baseline, 0/36 owner, 0/36 approach,
**5/38 both**) → with both, the ladder tier still almost never succeeds
(**2 of 65**, both to the `emergency` caller) → the escape-shaft fallback is
what actually carries it → on seeds 37/3 the shaft finds **nothing to mine**.

**Open, and Ben's:** (a) should `self_rescue` be given emergency-egress
semantics at all, and (b) what should a colonist do when it is stranded in open
air with no column to carve?

## Refilled from the roadmap 2026-08-19 (the queue's own rule)

12. **★ Roadmap item 39 — sub-threshold tick degradation. OPENED, first result
   banked** (`d92fce6af4`, `PERF39-GUARD-IS-BLIND.md`). Across 48 banked seeds
   `b5_soak_avg_tick_ms` runs **1.90 … 4.44 ms** against a pass clause of
   **`< 100.0`** — the guard sits **22–53×** above the operating range and
   cannot see a 2.34× spread. ★ A workload correlation was computed and
   **withdrawn**: the field is a *zero-input soak at step 8*, so `ch_cells`
   describes work already finished before the clock starts. The number is the
   **idle steady-state** cost, which makes the real question sharper — *which
   residual state variable drives it*. **Next step is free**: correlate against
   end-of-run state (colonists, live jobs, loaded chunks, job-board map sizes),
   all already in the banked payloads.

13. **★ Re-base the `avg_tick_ms` threshold** — 100.0 against a 1.9–4.4 ms
   population should be argued from the observed distribution. **A BAR change,
   so it needs Ben**; recorded here rather than applied.

14. ~~**SPEED ROW: invert the `veloren-server → bastion-server` edge**~~
   **REFUSED AND REVERTED 2026-08-19** (`d5de850603`, revert `d4ed8024c3`).
   ★ **This entry is kept only so nobody rebuilds it.** It previously read
   "feasibility DONE, verdict INVERT, predicted ~48%, the build is the open
   decision" — that was the *pre-build* read, and the build refuted it. The
   extraction was carried to completion (5,317 lines into `bastion-core`, both
   crates green, 135/135 conserved) and then the edge-cut **refused to
   compile**: `veloren-server` uses **31** symbols from `bastion_jobs`, of which
   **26** live in the very logic the row had to leave behind. The 10.2%
   "movable closure" was real; the *arrow* was not cuttable. Reverted forward,
   tree clean.
   **Do not re-open on the feasibility number.** A closure being movable is not
   the same claim as an edge being cuttable, and that distinction is what cost
   the row.

## Closed 2026-08-19 (later block)

- **#84 content mover — ANSWERED from banked data** (`30047a0593`): `plan_access`
  is the source on **120 of 126** blocked-cell entries (95%). Same subject as the
  self_rescue thread; they should never have been two rows.
- **Item 39 opened + advanced** (`d92fce6af4`, `ea1f2a2166`): the tick guard sits
  **22–53× above** the operating range (1.90–4.44 ms vs a 100.0 threshold), and
  the spread it would have to detect is barely above the instrument's own
  **1.21× noise floor**. A workload correlation was computed and **withdrawn**
  (the field is a *zero-input soak at step 8*).
- **Deterministic cost proxy BUILT** (`44bd816182`): `JobBoard::work_units`,
  surfaced as `b5_work_units` **beside** the millisecond field. Counts work, not
  time — so it doubles as a determinism witness, which a clock can never be.
- **Item 4 trait pinning BUILT** (`e4808a8976`): `BASTION_PIN_TRAIT` pins any of
  the 16 variants; an unrecognised name **panics** rather than silently falling
  back to random.
- **SPEED ROW 14 — REFUSED** (`d5de850603`, reverted forward in `d4ed8024c3`):
  built to completion, then the edge-cut showed `veloren-server` uses **31**
  symbols from `bastion_jobs` of which **26 are in the logic the row needed to
  leave behind**. Killed by its own measurement.
- **Two accelerants ENFORCED, not documented**: `CORPUS` (exit 9) and `BANKED`
  (exit 7) now refuse a fan launch; `DRYRUN` lets both be tested without spend.

## Open, and each needs a decision rather than a build

**Nothing below is blocked on work I can do.** Each has its measurement beside
it; what is missing is a choice. Items 12 and 14 above are CLOSED, not pending —
14 was built to completion, refuted by its own edge-cut, and reverted forward.

### ★ Bar 2 is no longer an open search — its cause is NAMED

`BAR2-CAUSE-IS-STRUCTURAL.md`: the client and server are **two independently
wall-paced loops**, so the mapping spin → server-tick is a function of the wall
clock. That explains why **all four** eliminated candidates returned nulls —
every one was a wall-clock *read inside* a loop, and the coupling is the loop's
*period*. It also predicts the corpus shape (100% divergence with a client,
capped and uncapped alike; **0% headless**).

Three options, priced:

| option | cost | who |
|---|---|---|
| 1. make the driver tick-driven | **11 pacing + 9 `dt` sites, all-or-nothing** — a partial build leaves half the coupling and manufactures an encouraging number | design |
| 2. scope bar 2 to the engine | membership identical 30/30, robust even under a deliberate wall-clock plant | redefines a bar |
| 3. retire the timing clause; re-register headless | already passes 6/6 there, and now `work_units` 5/5 | redefines a bar |


1. **bar-2 scope** — engine PASS vs engine+client FAIL. Decides how the row closes.
2. **haul-deadlock default** — `BASTION_FIX_HAUL_STARVED_CELL` on by default?
3. **`self_rescue` egress semantics** — measured necessary; adopting them is design.
4. **stranded-colonist behaviour** — what should a colonist do with no column to carve?
5. **`avg_tick_ms` threshold re-base** — a BAR change.
6. **which `PersonalityTrait` stands in for "reckless"** — one word; `Adventurous` is nearest.

## ★ ITEM 11 — banked at a PRECISE question (2026-08-19)

**Not "does recreation restore work?" — that is unanswerable until this is
settled.** The recreation gate at `bastion_jobs.rs:12460` is **never evaluated**:

| emit | line | fired in `recrgate` |
|---|---|---|
| hunger preempt | 12244 (inside the `'candidates` loop) | **40** |
| gate census (mine) | 12446 | **0** |
| recreation preempt | 12460 | **0** |

Flag delivered (`BASTION_ENV` confirms), binary contains the census (built
23:32, committed 23:29), `'candidates` loop demonstrably runs.

**The census sits AFTER the `if !serviced && struck_out` block at 12397, at the
same nesting level as the recreation gate** — so on the face of it, anything
reaching 12244 should reach 12446. It does not.

**THE QUESTION:** what path leaves the per-colonist iteration between 12244 and
12446? Candidates: a `break 'candidates` that exits further than it appears, an
outer-loop `continue`, or an enclosing binding that filters the population.
**This is a producer read, not a run** — and it is the whole of item 11, because
no fixture change can reach an unreachable branch.

★ **Do not re-run any recreation arm before answering it.** Three runs have now
been spent on this item (`recrab`, `recrabfed`, `recrgate`) and every one was
uninformative for the same reason.

## Arc sweep, 2026-08-20 (grand mandate day 1)

| row | state |
|---|---|
| Arc 3 item 14 (guards) | **3 of 4 bars PASS** (final disposition); bar 1b banked on colonist target-acquisition |
| Arc 2 item 11 (recreation) | **★ PASS, FINAL (823da4bc1b)** — 8/8 crossings→preempts at the exact tick; the restore REFILLS (0.40→~1.0 across breaks); previously: ROOT CAUSE FIXED — the gate's `contains_key` treated expired cooldowns as active (34,720-row census; every clause open except that one). Post-fix leg queued |
| Arc 2 item 12 (chronicle UI) | **BUILT** — universal inspect channel, payload carries enabled+truncated. Live legs queued |
| Arc 4 item 17 (skills) | **BUILT** — payload visible half + felt-curve test green. Live leg shares item 12's arm |
| Arc 4 item 16 (P2+P3) | fixture + in-window scorer ready (`haulrev`); blockers dissolved by today's fixes |
| Adopt-a-town mode A | **BUILT** (search/re-anchor/placement via shipping paths); live leg queued |
| Arc 5 desires + societal axis | **★ PASS (070e9e1e34)** — merit 106>101 throughput AND 0.5944<0.5997 mood, same-seed deterministic arms, causally attributable; margins thin (stated) |
| Arcs 5–8 bars | `LONG-HORIZON-ACCEPTANCE-BARS.md` written assuming compressed mode |

**Instrument catches today, each disclosed in its commit:** the attestation
guard refused FOUR legs over a stale driver; the suite gate read a piped exit
and sailed past a red floor (rebuilt unpiped); t0_27 fired four times on
founding-side bloat until the whole founding block left `tick()`; the
run-pit-while-running edit corrupted one postamble (data unaffected).

## Item 18's remaining half — determination (2026-08-20, read-only)

**PARKED BY DESIGN, not open work.** The "entity-log promoted-set migration" is
the log's stage 3 (persistence across the retention boundary), and the module
doc *deliberately excludes it* pending: (a) the promotion→RETENTION rename
(Opus ruling 2026-08-10, packet `55cdeb003c`) — done in doc, to be applied at
stage-3 open; (b) the `PickupItem` identity gap (uid re-minted per re-drop, so
in-inventory events have no stable subject) — **routed to Fable as its own
row**. Wake condition: Fable's identity ruling. Nothing here is buildable
without crossing that routing.

## Block 3 closures (2026-08-20, later)

| row | state |
|---|---|
| Item 16 P2+P3 | **PASS in-window** (baseline 10 / zeroed 0 / restored 15) — the retraction honestly re-scored |
| Item 17 | **PASS 4/4** — bar 3 from banked sweeps (farm 3→4 mid-run) |
| Item 24 bar 1 | **PASS** — summer 1,151 stage-ups vs winter 0 with the winter colony fully alive (30 tills, 8 sows, 213 claims) |
| Item 12 | **PASS 4/4**, Arc 2 complete |
| Desires/societal axis | **PASS** — tradeoff measures both directions, deterministic same-seed |
| Adopt-a-town | **bar 1 PASS** (5 plots → 1,519 designations, founded IN the town), bar 2 PARTIAL (access-work proven; production+eats pending one seeded leg). Three silent-fallback root causes found by successive witnesses |
| Item 27 (cooking) | vocabulary + all arms landed (CookStation + JobKind::Cook); NEXT: station registry + generator + completion handler |

## Blocked / needs a decision (Ben)

- **Tick-loading scope call** — roadmap criterion passes, mandate bar 2 fails.
- **Run-gait trigger** — `running = true` appears nowhere; 2 of 4 status
  variants never observed in 33,926 samples.
- **★ Should `self_rescue` be given `emergency_owner` + `emergency_approach`?**
  Measured 2026-08-19: without both it can **never** succeed (0/55 corpus-wide,
  0/36 across three arms); with both it succeeds on 3 of 5 seeds. **But passing
  `emergency_owner` is not a neutral hint** — it enrols the colonist in the
  emergency route machinery (`emergency_route_members` / `_targets` /
  `_descriptors`, `leave_route`) and activates an owner-gated `return None` on
  `emergency_reengage_exhausted`. Whether a *self-rescue* should adopt
  **emergency egress semantics** is a judgement about what self-rescue IS, so
  the builder stopped at the measurement. See `SELF-RESCUE-NEVER-SUCCEEDS.md`.

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
