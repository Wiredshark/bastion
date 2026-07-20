# M3 TAG PACKAGE — ladder contention (fair queue, capacity one) — builder-3, 2026-07-19

The Opus-gate evidence document (the R10-TAG-PACKAGE pattern). Architect verifies independently:
the new advance_epoch call-site, the broadened removal pin, and fence integrity under the queue.

## COMMIT CHAIN (all on bastion/builder, all pushed)
- d9c0ba9ae1 — M3 core: TraversalLink fair queue ((enqueue_tick, uid), capacity=1 locked),
  single-leave discipline (leave_route + source pins), queue re-election = the second
  advance_epoch site (the R10-package-predicted addition), queue-wait progress-exempt hold,
  additive ownership/recorder/probe surfaces.
- 09db8077a9 — M3A-D fixture episodes + chamber geometry + the 1-cell mount Goto fix (N2-green).
- e500402b3b — --print-git-hash stale-binary pre-flight (architect-wired across the wrappers).
- cebb45746f — the M3A fix stack (5 tape-rooted fixes incl. the Sonnet-ruled + architect-co-ruled
  UNIFIED promoted-head corridor authority) + BOTH pre-tag R10 safety fixes (REQ-0040 teardown
  retires through retire_traversal_task; the pin bans retain/drain/clear/swap_remove).
- 1dca97248f — corpus-episode composition rider (enables the matrix).

## THE SAFETY SURFACE (what the Opus gate verifies)
1. advance_epoch call-sites == 2 (retirement + leave_route head-re-election), source-pinned with
   rationale; double-advance on same-block retire+leave documented safe by the fence's equality
   algebra.
2. Removal exhaustiveness: `.remove(` == 1 (inside retire_traversal_task) AND retain/drain/clear/
   swap_remove == 0 on the task map, BY CONSTRUCTION (runtime-constructed needles).
3. Fence under the queue: 12 owned movement writers still present authority; terminal zeros still
   deliberately raw; fenced_movement_write site count unchanged (13). The unified corridor drive
   writes ONLY for task-less heads (pre-reservation approach = the ordinary non-owned path, same
   class as corridor/egress drives); from reservation on, the phase machine is sole writer.
4. Queue-wait never wipes an over-budget watch (position-scaled budget, (C)-rule denial); M3D
   proves both arms empirically.

## EPISODE RESULTS (canonical gate seed 1337, local, deterministic)
- M3A (N=3 fair queue): PASS — out [66,143,150] BY CLIMBING, exit order == queue order,
  teleports 0, owned-conflicts 0, SOFT-0 violations 0.
- M3B (N=5 chokepoint): PASS — out [66,88,95,101,175], four re-elections (generation 5),
  teleports 0, conflicts 0, violations 0. NOTE: runs the 2x2-shaft chamber, not the packet's
  "1-wide shaft" (geometry deviation, documented; 1-wide crew-funnel realism lives in the
  standing --chokepoint-scenario).
- M3C (mid-traversal owner abort): PASS — C-0 relocated mid-climb (production ExternalRelocation
  interrupt), queue re-elected cleanly (generation past injection), remaining out [58,87,92],
  conflicts 0, production teleports 0.
- M3D (never-stranded BOTH arms): PASS — permanent rim seal + 30s override budget: all three
  net-delivered (teleports 3 BY DESIGN), waiter deliveries at 206/265s >= the 85s discriminator
  (hold-alive = budget 30 + watch 60; hold-dead would deliver ~60-70s) — the queue-wait exemption
  AND the inviolable net both demonstrated. Within-budget arm = M3A/B/C's zero-teleport bars.
- N2 (M2 single-member contract): PASS 10x consecutive across every fix iteration — untouched.

## THE M3A FIX ARC (5 fixes, each tape-rooted; the full forensic arc lives in the session
## scratchpad m3a-arc-package.md and the cebb45746f commit message)
corridor-commit anchored at wp0 (B57 site 2) → entry fallback belt → promotion driver (the turn IS
the permission) → mount-preflight own-prefix contact (B57 site 3, architect-predicted) → the
UNIFIED corridor authority (queue decides WHO, corridor decides HOW, per-tick arm DRIVES; the
uncoordinated-waypoint-source livelock class dies by construction). Two implementation dimensions
the tapes forced: CADENCE (once-per-second passes cannot drive) and HEAD-ONLY scoping (corridors
for waiters = the fork-#15 vanilla-leak, caught by the fixture's own SOFT-0/fair-order bars).

## MULTI-SEED MATRIX (24 runs: 4 episodes x 6 seeds; local quiet-machine after the VM batch died
## to the LFS/+dirty guard false-alarm — guard since fixed sha-part-only by the architect)
Raw verdicts 9/24 PASS; canonical 1337 = 4/4 PASS. ★ THE TAG-CRITICAL INVARIANTS HELD ON ALL 24:
exit order == queue order on EVERY run where members exited (including every red), owned-conflicts
0 on 24/24, staging premises green. The red classes, each CLASSIFIED (Sonnet-ruled):
- (b-inherited) NET-RELIANCE AT ORGANIC GEOMETRIES (seeds 21/42 class; the tag-gating question):
  fair order held; the reds are the fixture's zero-teleport bar only. DISCRIMINATOR: N2 — the
  M2-era single-member contract — takes 1-2 net deliveries at the SAME seeds and was never held
  to zero-teleport. Inherited M2-era approach-chain behavior that M3A's stricter bar EXPOSES;
  proportional per-member (3-4 nets for 3 members vs N2's 1-2 for one), NOT contention-amplified.
  The packet's own never-stranded bar treats the net as a VALID backstop ("a queued member whose
  turn never comes must still be caught by the net") — the zero-teleport target was a fixture
  aspiration, not the contract. RULING: tag-acceptable TRACKED OPEN (registry row B58); the
  frontier-approach chain is a scoped follow-up (candidate: the same corridor-unification the
  promoted-head path got), a DIFFERENT block's territory, not M3-queue work. En route the thread
  produced two REAL landed fixes: B57 site 4 (the 1770-cycle corridor commit→invalidate livelock —
  dead, corridor advancing 59 samples post-fix) + the grab-band entry-fallback widen.
- (a1) HARD-ROLL BACKSTOP (777-class): slow organic service + 2-3 nets, fair order intact —
  never-stranded working as designed (same classification family as the ruling above).
- (a2) FIXTURE-PREDICATE (M3B@7: 44 violations with 0 teleports and all five climbing fast in
  fair order; M3A@99 similar): the SOFT-0 check counts the fixture's CARVED shaft columns; organic
  rolls can site the planner's actual lane elsewhere — a fixture hardening item (read the PLAN's
  lane), post-tag.
- (a3) M3D TIMING BARS on non-canonical rolls: the both-arms windows were calibrated on 1337's
  build timeline; shifted rolls move delivery windows — bar calibration, not mechanism (the hold+
  net mechanism is visible in the timing fields on every roll).
- CORPUS-RUNNER caveats logged for the follow-up list: children discard stderr (forensics lost —
  tee per-seed), and in-process multi-child runs are load-suspect until proven otherwise (the
  quiet-machine rerun is the arbiter; all classifications above are from quiet runs).

## DETERMINISM (packet: x3)
✅ IDENTICAL — M3A @1337 x3 reps, verdict-JSON byte-equal across all three (same machine, quiet;
recorder-tape determinism was R10-x2-proven on this fixture family; the JSON is the behavioral
surface). Artifacts: session scratchpad m3a-det-{1,2,3}.json.

## HONEST RIDERS / KNOWN-OPENS
- M3B geometry deviation (above).
- Multi-seed net-backstop deliveries (777-class) pending classification — fair order + zero
  conflicts held on every observed seed; the question is only escape-mode distribution.
- The head-gate diag + queue-wait budget env override are env-gated, sim-inert instruments
  (BASTION_EGRESS_DIAG / BASTION_M3_QUEUE_WAIT_BUDGET_TICKS — the latter never set by live
  binaries, M3D-only).
- Codex's HashMap-order persistence fix rides a separate isolated branch (architect-managed,
  merges after M3; informational).
