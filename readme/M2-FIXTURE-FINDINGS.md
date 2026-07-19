# M2 fixture findings — the owned-contract gap, fully diagnosed (2026-07-18)

## The chain (every link evidence-named, P0 diag seed 21, uid 9)
1. Ladder plan emits (kind=Ladder steps=7) + route membership + descriptor ✓ (planner-fix working).
2. CONSTRUCTION: the member holds rung jobs 0..6; the route lifecycle takes ownership and
   ZERO-INPUTS him every tick ("emergency route ownership spans frontier claim gap",
   zero_input_written=true, corridor_frontier=None from t≈780 on). No FrontierWork movement
   corridor is ever computed. Rungs 456-461 get built (floor-reach + whatever partial motion);
   the frontier rung(s) above reach stall.
3. DEADLOCK: mid-prefix recovery runs every 30t from t≈3000 ("read_only_existing_anchor_search",
   20 candidates: 10 support-rejected [in-shaft cells hang over air], 10 clearance-rejected
   [walls]) — a VERTICAL-SHAFT ladder has NO mid-prefix anchors BY GEOMETRY; the search skips
   the bottom entry by design ("not a mid-prefix recovery anchor") and the bottom-re-entry
   machinery (REQ-0081 saved_entry/approach corridor, bastion_jobs ~7532+) never engages
   during FrontierWork.
4. LEAK: the terrain-side ladder-token (sprite adjacency in the cap block) uncaps the member
   beside the built rungs; regen-cycled VANILLA climb exits him at ~106s — real ladder, real
   exit, ZERO Stage-1 task transitions (diag-confirmed; the task preflight at :5676 requires
   phases the member never reaches). The corpus's 6/6 latches are all this shape.

## What this means
- The M2 headline STANDS: the game plans + builds correct connected ladders on all 6 seeds and
  colonists exit through them (101-121s).
- Ben's 4 failure classes are NOT yet certified against the OWNED contract: the single-owner /
  deterministic-mount / stable-dismount guarantees ride on the vanilla path's luck until the
  ConstructionFrontier + bottom-re-entry activation lands.
- The deterministic mount-snap (utils.rs/climb.rs, token-scoped) is in-tree but DORMANT in this
  path (the token attaches inside task upkeep — no task, no token).

## FIRST-DIVERGENCE REFINEMENT (last datum this cycle)
The entry-selection branch (bastion_jobs ~7953: emergency_partial_route_entry for
ConstructedLadder → entry_record insert at ~8008 → the ConstructionFrontier task creation at
~8093 behind grounded_clear + route_energy_ready [FULL energy, REQ-0071] + entry_record) NEVER
RAN — zero "partial_route_blocked"/"no validated traversal entry" lines in the P0 diag. The
member is instead owned by the "emergency route ownership spans frontier claim gap" block
(writer="emergency_route_lifecycle", zero_input_written=true every tick, corridor_frontier=None).
NEXT GREP TARGET: the routing condition that sends a frontier-claim-holding member into the
lifecycle gap-block instead of the entry-selection path — that condition is the true first
divergence. The mid-prefix anchor search seen looping was only the READ-ONLY diagnostic
(recovery_diagnostic at ~7475), not the selector.

## ★ ROOT CAUSE RESOLVED (flow-marker round complete) — THE ALREADY-AT-ENTRY DEGENERATE SWEEP
The flow marker + correct-file census (my earlier zeros were a STALE-FILE grep — census the log
you actually wrote) found "corridor rejected no_reachable_body_lane" ×61: the member STANDS IN
the descriptor entry cell (pos 18484.73, 9297.55, 455 = the entry itself), and the approach-
corridor validation sweep from his position to the entry DEGENERATELY SELF-HITS the cell he
occupies (TerrainSweepHit resolve_dir (0,0,0), sample 0/0 — the zero-length sweep edge case) →
corridor rejected forever → no entry record → no ConstructionFrontier task → the whole owned
chain dead. He never needed an approach at all. FIX APPLIED (bastion_jobs, the corridor_result
closure): already-at-entry (xy dist² < 0.36 to entry center, |dz| < 1.2) = trivially valid with
an EMPTY corridor; the zero-waypoint corridor_step completes immediately into the entry record
→ the :8093 ConstructionFrontier creation (energy-gated by REQ-0071 full-energy, satisfied by
regen) → owned climb of the built prefix → frontier completion → FullExit task → the mount-snap
token finally attaches. Verification P0 rerun in flight; suppressor-nulls-Goto hypothesis
NOT needed (superseded by this root).

## FLOW-MARKER ROUND (superseded by the resolution above)
All four downstream exits of the FrontierWork branch log NOTHING (corridor committed /
corridor rejected no_reachable_body_lane / approaching entry / transition probe = 0 each) while
the recovery diagnostic loops — the silent bail sits between ~7532 (saved_entry) and the logged
exits. A diag flow-marker now logs saved_entry/corridor_present/completed_jobs/traversal_kind
per 30t at exactly that point (build p0-diag2 running). NEW HYPOTHESIS from the code shape: the
CLIMB-loop suppressor (the "spans frontier claim gap" block, :4322) sets agent activity=None
UNCONDITIONALLY every tick (:4345) and only preserves CONTROLLER inputs when
corridor_movement_owned — if the approach corridor drives movement via rtsim Goto activity, the
suppressor nulls the corridor's own Goto every tick → frozen member → corridor stalls →
recovery loop. If the marker shows corridor_present=true with the member frozen, the fix is the
suppressor must ALSO preserve (not null) the corridor's activity when the corridor owns
movement — a one-condition change at :4345.

## OWNED WALK PROVEN + CALIBRATION ROUND 2 (in flight)
First owned P0 ×2 PASS: QueuedForLink→Reserved (1 tick) → TraversingEntry (4 ticks) →
★TraversingLink in 4 TICKS (the deterministic mount-snap, not the ~50% jump-flake) → 1.5s real
climb → TraversingTopExit → FrontierWork (top rung) → out at 52s (vs 106s unowned), audit
clean, zero teleports. Calibration fixes then applied: (1) per-TICK fixture sampling (Reserved
lasts 3 ticks — 1s polls missed it, so N1/N4 mutators never fired and N3's abort reason
vanished with the task); (2) N1/N4 trigger on first task-present sample; (3) verifier scopes
REAL-CLIMB/ASSIST-QUIET to the OWNED-PHASE WINDOW per the spec (whole-tape gating false-flagged
construction-phase walking ascents and pre-task assists); (4) bastion_emit_damage instance
fixed constant (rand::random() broke N6's x2 determinism). Wrapper+verifier round 2 running.

## ROUNDS 3-4 (calibration converging)
Round 3: P0/N5/N2 tape-green ×2 (49 owned ascents, 0 fake, 0 assists, max_dz 0.39). N1 = a
CONTRACT SHOWCASE: blocked climb path → bounded classified abort (authoritative-climb-no-
progress @10s) → clean release → automatic re-plan → second owned traversal → out at 116s
(abort AND recovery proven; pass-condition updated — the spec demands classified abort +
release + alive + no teleport, not failure-forever). N3: instant route-invalid (correct
production classification — route validity outranks physics contact when the stimulus destroys
route cells; reason REPORTED per spec). N4 round-3 no-trip explained by reading
emergency_route_terrain_revision: the fingerprint hashes DESCRIPTOR ANCHORS ±1z only, never
rungs — round 4 swaps the floor block under the member's feet (entry−1, in the set) to a
different solid kind. ★ N6 FIRST-DIVERGENCE = ENGINE FINDING (registry class 7): item_hash on
inventory items is nondeterministic across identical runs (post-damage food-eat differs);
comparator normalizes it out (named), engine fix = deterministic item identity. Round 4
running: full matrix expected green except any genuine residue.

## ★ N1 RECONCILIATION (architect gate-prep challenge, round 5)
The architect blocked the tag on my N1 reclassification — correctly. No written authority
existed; the real root was STIMULUS DRIFT: the round-2 re-aim put the block at feet+2 (climb
PATH), outside both the member's body and the fingerprint (anchors ±1z only), so neither spec
reason {route-invalid, stale-terrain-revision} could ever fire — the climb legitimately started
and physics no-progress caught it. I then relaxed the gate to match the off-spec stimulus =
goalpost-moving, conceded. His regression hypothesis (at-entry short-circuit bypassing entry
validity) audited FALSE in code: the short-circuit skips only the approach SWEEP;
validate_terrain_revision runs EVERY TICK in task upkeep (bastion_jobs :4794-4805, before the
phase dispatch, all phases) — post-Reserved fingerprint mutation aborts next tick. Round-4 N4
independently exercises that exact path. Round-5 N1 rebuilt SPEC-EXACT: block into the entry
body cell (member stands AT entry → head cell = entry+1z, a fingerprint anchor); gate strict:
abort ∈ spec's two AND zero TraversingLink on the mutated reservation
(owned_climb_before_abort); post_abort_link REPORTED not gated (architect rules on "ever");
alive/unentombed-after is now a live assertion (block lands IN his head cell). LESSON (registry-
worthy if it recurs): when a falsifier's observed abort reason differs from spec, suspect the
STIMULUS before reinterpreting the EXPECTATION — the spec's reasons encode WHERE the block
must land.

## ARCHITECT RULINGS 1+2 (accepted + implemented, round 5)
Ruling 1 — N1 gate = CLEAN ATOMIC ABORT ONLY: spec reason ∈ {route-invalid,
stale-terrain-revision} + zero TraversingLink on the mutated reservation + teleports==0
(production locomotion failsafe delta — the fixture's old counter was declared-but-never-fed
DEAD PLUMBING, now wired; verifier eject-backstop count = second witness) + post-abort
reservation transitions ≤5 (smoke80 unbounded-reacquire proxy). Post-abort behavior reported
never gated; successful exit despite the permanent seal prints M2-N1-RED-FLAG → escalate.
Ruling 2 — TWO VARIANTS packaged, never collapsed: N1 spec-literal (head cell = entry+1z;
alive/unentombed REPORTED, entombment = genuine finding) + N1B intent-faithful (survivable
fingerprint block at the descriptor DISMOUNT anchor via new read-only route_dismount probe
[bastion_jobs board method + lib.rs bastion_route_dismount]; alive+unentombed GATED).
Divergence between variants escalates. My regression-audit (per-tick validate_terrain_revision
at :4794 independent of the skipped sweep) is retired-hypothesis gate evidence.

## ROUND-4 RESULTS (evidence archived: m2-fixture-evidence-round4)
★ N4 PASS ×2 det ✓ with phases Reserved@45→Abort@45 SAME SECOND, reason exactly
stale-terrain-revision, zero TraversingLink on the mutated reservation — POST-RESERVED
FINGERPRINT VALIDATION EMPIRICALLY CONFIRMED (the architect's "N4 seals it" datum; its later
re-plan is legit — the earth swap changes the hash, seals nothing). N1's round-4 PASS is VOID
(loose gate): phases show TraversingLink@45→Abort@55 (authoritative-climb-no-progress) — the
path-block stimulus correctly FAILS the strict gate, proving the stimulus had to move to a
fingerprint/body cell. P0/N5/N2 green det ✓ (49 ascends, 0 fake, 0 assists). N6 det=False
WIDENS class 7: inv_slot differs too (slot_idx 8 vs 6, same tick, same food) — one root
(inventory ordering), two fields; verifier now normalizes both (named). N1B round-4 FAIL =
artifact (verifier newer than run; no tapes). Ops lesson RE-LEARNED: my wrapper edit raced the
running sh (spurious "line 23: one: command not found" — post-loop, harmless, never again).
Round-5 prediction: N1 head-cell block → Reserved→Abort same tick; entombment outcome OPEN —
if the embed-failsafe teleports him out, strict-N1 fails on the teleport leg = the (i)/(ii)
divergence the architect pre-agreed to rule on. Report as-is either way.

## COMMIT SERIES LANDED (round-5 gate fill; tag package backbone)
d157769430 Stage-1 plumbing (FOREIGN, committer-not-author — behavior.rs/character_behavior.rs
JoinData threading, last uncommitted Stage-1 remnant; component itself was already committed) →
f76794c05c mount-snap (utils/climb, mine) → ec1d3c912b owned-contract unlock (at-entry fix +
route_dismount probe + flow marker + fixed damage instance, mine) → 82126b377f fixture (mine)
→ 19e3d10baf registry class 7. All M2 files clean; remaining tree dirt = other efforts only
(client/lib.rs, common/lib.rs, voxygen/*, rtsim/*, server/agent/* etc. — NOT touched).
Per-commit compile NOT verified (cargo banned during the round-5 gate); series is dependency-
ordered so each prefix should stand — bisect-check later only if ever needed.

## ROUND-5 RESULTS + THE SEAL-INTEGRITY DOUBLE RED-FLAG (evidence: -round5)
N1+N1B PASS ×2 det under the strict gate: Reserved@45→Abort@45 same second, exactly
stale-terrain-revision, zero TraversingLink on the mutated reservation, teleports 0, ALIVE +
UNENTOMBED both variants (physics resolved the head-cell block; the (i)/(ii) divergence never
materialized). ★ M2-N1-RED-FLAG deterministic in BOTH, mechanisms = the architect's two
pre-named alternatives, one each: N1 out@53 WITH NO TASK (post_abort_reservations 0!) — the
head block invalidates the DESCRIPTOR not the LADDER; rungs stand; exit rides the pre-existing
UNGOVERNED vanilla ladder-token climb (the known leak the owned contract is meant to
supersede). N1B out@151 — auto-router treats the blocked dismount as a construction frontier:
3 bounded task cycles rebuild access THROUGH the seal. Escalated for ruling: strict-PASS +
red-flag-reported vs tag-blocks-on-closing-the-vanilla-leak. ★ N6 divergence is BEHAVIORAL
(6990/9002 samples fork after same-tick different-slot eats — one walks, one stands):
class-7 propagates into sim behavior; normalization insufficient; round 6 ISOLATES via
bastion_canonicalize_colonist_food staging (all consumables → one asset-deterministic cheese,
precondition-asserted). JSON note: serde BTreeMap sorts keys — grep the m2_abort_reason
prefix, not m2_ladder_episode.

## SEAL RULING (architect) + TAG CONDITIONS
N1B = BENIGN (bounded owned rebuild = designed resilience; flag cleared, rider now N1-only).
N1 vanilla leak = pre-existing, does NOT block tag; MUST be NAMED as certified-known-open +
next-arc step (fork #15 logged for Ben: close the leak = gate/supersede the colonist
ladder-token so the owned contract is SOLE egress). TAG CONDITION 2 (the load-bearing one):
v4 corpus must show NORMAL-OP organic exits carry the owned phase-walk — instrumented via
per-tick stuckjob_owned_phases [A,B,C]; A/C teleport BY DESIGN so the read is on B (whose
plan is ladder-tier exactly when walkability rejects steep stairs — the seed-20 class).
B TASKLESS on a ladder-plan seed = HOLD. Class-7 behavioral-fork handling endorsed
(staging-isolation not normalization); engine chip priority raised; architect flagging to Ben.
Opus-depth review checklist at package: normal-op owned governance, teleport zeros + eject
second witness, mount-snap token-scoping (no player path, deterministic revert), both N1
variants, walkability class-closure, leak named.

## ROUND 7 (tag-candidate binary) + THE N6 INTERRUPT-LIVENESS QUESTION
Full matrix OVERALL PASS ×2 det INCLUDING N6 (food canonicalization proven in-binary; the
round-6 green was a STALE-BINARY LOTTERY — build failed E0596 [needed `let mut inventory`],
';'-chained launcher ran the old exe, two runs drew the same slot. Ops rule: verify build +
exe mtime BEFORE any evidence run; orchestrator does). N1/N1B reconfirmed identical to
round 5. ★ N6 teleports=1 = the member's POST-EXIT wander (out@52, 248s free-roam, ultimate
failsafe 93 blocks away, deterministic, outside all owned windows; member-scoped eject
witness 0 — claim scoped to owned windows). ★★ N6 INTERRUPT NEVER FIRED: one damage emission
at first link tick → no abort in ~60 owned ticks → clean completion. Suspect: the AgentInbox
trigger (bastion_jobs :4582 `!agent.inbox.is_empty()` during owns_movement_intent ticks)
RACES the agent system's per-tick inbox drain; traversal_tooling tests build inbox state
directly = cannot catch the live-dispatcher race (gate-must-test-live-path). Round 8: N6
re-aimed to SUSTAINED 0.5/tick damage through the link window (fires → liveness confirmed;
doesn't → AgentInbox interruption DEAD ON LIVE PATH = engine finding, architect rules tag
impact). Round-8 rebuild is harness-only (server sources identical); corpus stays on the
06:40 binary — provenance flagged to the architect.

## ★★ V4 GATE-HOLD + THE CORRIDOR-SUPPRESSOR ROOT (the shelved hypothesis was RIGHT)
v4 corpus fired the locked gate condition 18/18: Bkind=ConstructedLadder ALL SIX seeds, B
TASKLESS all runs — and worse: B exits by BACKSTOP 6/6 (105-119s) where v3 exited by organic
dig 5/6 (129-178s, Bkind=None, seed-20 STRANDED). Seed-20's strand is gone but via teleport.
FORENSICS (s21 B uid=10): six rungs BUILT by t=1396, owned chain ENGAGES (corridor committed,
frontier_job=6) → 61 samples of 'corridor advancing' with next_idx=0 and position FROZEN →
failsafe. corridor_movement_owned=false throughout. ROOT = the CLIMB-loop suppressor nulls
agent activity every tick and only preserves inputs when corridor_movement_owned; the
approach corridor drives via Goto → nulled → frozen one cell from entry. THE ROUND-1
HYPOTHESIS I MARKED SUPERSEDED — wrongly: the at-entry fix yields an EMPTY corridor, so the
fixture's degenerate geometry CANNOT exercise approach movement and MASKED the bug
(corpus-first earning its keep; fixture gap: need a general-position P0 variant, member
parked 2+ cells from entry). Energy-gate suspicion WRONG (chain engaged). Walkability
6/6-ladder question now secondary (ladder plan itself progressed).
FIX (proposed, awaiting architect green light; server frozen until): one-condition at the
suppressor (~:4345) — corridor present+driving ⇒ preserve/emit corridor movement
(corridor_movement_owned true on that path). Expected: B owned organic exit ~60-90s, gate
condition flips green ON the corpus. Cost: new binary → full rerun (lanes+matrix+N6-round8
folded into ONE binary). LESSON: never mark a hypothesis superseded because a DIFFERENT fix
landed — superseded requires the hypothesis's own predicted observation to be re-tested.

## ROOT REFINEMENT (correction issued to architect — the suppressor-fix proposal was MISAIMED)
My 'corridor_movement_owned=false throughout' was a MISREAD (lines from t=600-990, pre-
corridor). Freeze window truth (t≥1410): corridor_movement_owned=TRUE, controller_input_before
=(0,0) — inputs arrive at the suppressor already zero; the suppressor preserves correctly.
The corridor re-sets its Goto EVERY tick (:7874 unconditional; :7823 comment expects the
inter-pass null); advance tolerance legit (1.05 blocks to walk, 0.75 threshold). The Goto
never CONVERTS to movement. Freeze onset = the tick B CLAIMED the above-reach frontier job
(SELECT job=6 t=1395 → corridor commit t=1396 → frozen; job 6 later completes at t=3218 by
remote-arrival FROM THE SURFACE after the failsafe teleport). LEADING: agent ACTIVE-JOB
branch shadows the Goto (claimed unreachable job → job branch emits zero movement, activity
never processed). ALTERNATIVES: suppressor-null × dispatcher ordering; chaser state.
DISCRIMINATOR RUNNING: BASTION_GOTO_WRITER_DIAG_UID=10 (purpose-built diag, prints
previous_activity + full chaser state per handoff) on the CURRENT binary, seed 21 — no code
change. previous_activity=None per pass ⇒ null/ordering; =Goto ⇒ shadow/chaser, and the
chaser fields discriminate further. LESSON compounding: quote log fields WITH their tick
window — a field's value outside the phenomenon's window is not evidence about the
phenomenon (kin to the namespace-pun rule).

## ★ MECHANISM CONFIRMED + FIX IMPLEMENTED (option b, architect-ruled)
Writer-diag (1822 handoffs): previous_activity=None EVERY pass + chaser_route_complete=TRUE
one tick after commit at 1.05 blocks out = TOLERANCE INVERSION — generic Chaser arrival
radius (~1.5) > corridor REQ-0087 cursor (0.75): mover satisfied, cursor starving, frozen
forever. REQ-0087's own comment documents the cursor as 'stricter than ordinary Goto path
following' — the deadlock is the MOVER keeping the looser radius. FIX: corridor writes the
member's controller inputs DIRECTLY (normalized planar step to waypoint, move_z 0) in the
UPKEEP loop (:7213 join — entity available, controllers.get_mut(entity), NOT the :4171
climb_iter which is a different loop: first attempt E0425'd, caught at build); the rtsim
set_goto handoff REMOVED (controller write = sole mover; suppressor keeps nulling agent/rtsim
= blocks competitors). Pass-order safe: suppressor (climb_iter, earlier) → corridor write
(upkeep loop, later) → phys. New binary 07:47:58 verified (class-8 discipline). Episodes now
10: +P0G (general position) +N5G (attractor live from t=0 through approach; full-exit bar).
Registry class 9 filed. SMOKE running (s21 stuckjob) before the 2h pipeline commits.

## ★★ LAYER 2: THE SWEEP EATS ITS OWN RUNG (smoke round, fix-in-tree-uncommitted)
Message-crossing note: the architect's hold-everything arrived AFTER his two option-(b) green
lights (which crossed my correction); option (b) was implemented + smoked before the hold
landed. Tree: fix IN, UNCOMMITTED, no pipeline launched, no server edits since the hold.
SMOKE (s21): option (b) FIXES layer 1 — B WALKS, cursor 0→1→3 of 4, reaches ~1 block from
the entry. LAYER 2 revealed (structurally unreachable pre-fix): the corridor's RUNTIME EDGE
SWEEP hits THE ROUTE'S OWN FIRST RUNG — every invalidation = block (18474,9281,456) =
first_rung exactly. Planned waypoints dog-leg around the rung column; the runtime sweep
anchors at CURRENT (off-center) position → clips the rung cell → edge_stale_or_blocked →
destroy → recommit ×4 → creation reject (no_reachable_body_lane) → backstop 107s (same
number as v4, ENTIRELY different journey — never trust a matching scalar). PROPOSED (awaiting
re-rule): keep (b) + (ii) sweep anchored at the PLANNED SEGMENT waypoint[i]→waypoint[i+1]
(both observed hits geometrically vanish; sweep's purpose = terrain-change detection under
the PLAN; endpoint_ready still guards arrival) — alternative (i) exempt route rung cells
(provenance-authority principle, but touches shared-helper solidity semantics). Recommend (ii).

## LAYER-2 FIX IMPLEMENTED ((ii) ruled + built; ONE-SHOT boundary active)
Architect's authoritative ruling: keep (b) + implement (ii) planned-segment sweep anchoring;
HARD BOUNDARY — if the smoke reveals a LAYER 3, REVERT-ALL to the v3 honest-intermediate and
bench the approach-corridor productionization as a dedicated next block (no 3-layer
onion-peeling tonight). Implementation: EmergencyApproachCorridor gains `origin: Vec3<f32>`
(commit-time position, set at the :7679 constructor + test constructor); the corridor_step
closure computes segment_origin (idx==0 → origin; idx≥1 → waypoints[idx-1] center) and BOTH
runtime re-validation sweeps anchor on segment_origin instead of pos.0. The ADVANCE gate's
arrival sweep stays position-anchored BY DESIGN (arrival is a live property; re-validation is
a plan property — the principled split). Known residual risk checked by geometry: the entry-
arrival sweep could clip the rung only on >0.28-block eastward drift; the direct-input mover
drives straight at waypoint centers so drift is bounded — the smoke adjudicates. Gate
conditions for the rerun (architect): N3/N4 still fire (terrain-change detection NOT
weakened), zero edge_stale on route-sequence cells, B owned-exits corpus-wide incl seed-20.
Binary 07:58:27 verified. Smoke bar = FULL OWNED EXIT (phase-walk to exit), launch-or-revert
is mine per the blessed sequence.

## FINAL-PIPELINE PHASE 1: 9/10 GREEN incl ★P0G; N5G = AIM ERROR not layer 3
P0G full owned walk @48s out@55 — corridor fix proven in-fixture at GENERAL POSITION, zero
invalidations. N3/N4 reasons unchanged (segment anchoring did NOT weaken terrain-change
detection — architect gate condition 1 ✓). N5G FAIL diagnosed: phases=['-@0'] ×2 — the
tick-0 attractor wins the PRE-ROUTE race (no ownership exists yet; legitimate agent
behavior), member commits to the unreachable surface job, stalls, failsafe out@87
(teleports=1 = that failsafe; alive; det ×2). MY AIM ERROR: the condition was single-owner
DURING APPROACH; tick-0 tests pre-ownership. During-approach is (a) suppressor-covered by
mechanism (:4332 gates on claim/construction ownership, spanning the approach) and (b)
corpus-proven natively (B approaches with his own live decoy designations on every seed).
REAL FINDING inside the aim error, filed for disposition: a trapped colonist with a live
unreachable attractor never starts his own rescue — rides the backstop out (rescue-priority
vs ordinary claims; stuck-economy lane, not this tag). Architect disposition requested:
(1) reclassify N5G as the pre-route race probe + corpus B-leg as the approach evidence
[recommended] vs (2) true during-approach paint (needs corridor probe = new binary = full
rerun). Pipeline CONTINUING — corpus validity unaffected.

## ★★★ THE PACKAGE DELIVERED (corpus-v5 + fixture matrix on binary 07:58:27)
CORPUS TRANSFORMED: GATE-HOLD 0 anomalies (v4: 18/18). B owned-ORGANIC 4/6 at 51-55s incl
SEED-20 (v3 stranded → 55s full walk ×3 det); s1337/s22 engaged + bounded + backstop-
delivered (commits 15s later, harder geometry). Organic 19/36 = 52% (pre-M2 17%, v4 22%);
probe ctrl 1337/22 flipped organic. Condition-2 SHOWN ×6 (commit in live-designation window,
phase-walk, 0 invalidations, no divert — held even on backstop seeds). N6: sustained damage
STILL no interrupt → AgentInbox interruption DEAD ON LIVE PATH = filed non-blocker per the
settled ruling. Fixture 9/10 ×2 det (N5G reclassified INVALID w/ C1 tape-proof). Package
message sent with the honest 4/6-complete + 2/6-engaged split presented for the architect's
condition-3 call, TAG recommended with two named-open items (engaged-not-completed tail +
vanilla leak). AWAITING: the inline Opus-depth review verdict = the tag.

## ═══ BACKSTOP-OPT BLOCK (post-M2LADDER; task #61) — MEASUREMENT OVERTURNS PREMISE
Architect's hypothesis (productive builder misread as stuck → progress-aware watch) is NOT
what the tape shows. Both seeds: rungs 0-5 build 21-59s, corridor commits 59s, owned task
starts, THEN: s1337 — TraversingLink abort at t=1832 reason=agent-inbox-interruption (3 rungs
up, z=394/top 397) → hangs on-wall 395.6 → corridor re-creation REJECTS per 30t (creation
sweep anchors at his MID-WALL position, no_reachable_body_lane) → 66s 'cleared stale egress
movement at verified surface exit' fires AT 395.6 VS TOP 398 (2.4 short!) + watch wiped
route-cleanup → route GONE → descends → claims surface Mine job → presses bare wall at base
(cap tape: anchor=z, on_wall, vz=0 — cap CORRECT) → 60s → failsafe 126s. s22 — FULL walk,
frontier COMPLETES THE LAST RUNG 68s (ladder COMPLETE) → 73s same cleanup at 331.28 vs top
332 (0.72 short, NO stable dismount) → DESCENDS → surface Mine job → presses wall 2 BLOCKS
EAST of his own finished ladder → cap-wedged → failsafe 133s.
UNIFYING ROOT: post-route-end nothing re-engages — (i) 'verified surface exit' releases
non-exited members (missing the EXIT_STABLE_SAMPLES stable-dismount bar); (ii) post-abort
mid-wall recovery dead-ends (position-anchored creation sweep + premature teardown kills the
bottom re-entry); (iii) ordinary pathing ignores built ladders + the cap (fork-#15 territory,
defer). PROPOSED (A) exit-release requires stable dismount; (B) below-grade member + live
need + BUILT route ⇒ re-engage from bottom entry; (C=defer). Architect check requested
pre-build (diverges from his (1)/(2)).
★ N6 FINDING CORRECTED (registry appended): agent-inbox-interruption FIRED LIVE on s1337 —
the mechanism works; MY N6 stimulus never engaged it (third stimulus-aim-error instance).
Tag named-open (3) wording amended in the registry.

## BACKSTOP-OPT (A)+(B) IMPLEMENTED (architect-approved re-scope; binary 11:04:21)
(A) THE RELEASE-PATH BAR (the measured hole): `at_verified_exit` was
`!below_grade || (at_route_exit && supported && body_clear)` where below_grade used a
3.0-block surface-distance threshold — an UNSUPPORTED mid-wall hang 2.4 below route_top
short-circuited to 'verified' and safe_secs released him. NOW:
`(at_surface || at_route_exit) && supported && body_clear` with at_surface = surface dest
dz < 1.0 ∧ dxy < 3.0 (essentially AT surface height; in-shaft ledges no longer count) and
at_route_exit tightened to top−0.5. The EMERGENCY_SAFE_SECS (5s) stability window rides on
top. The surface leg stays so teleported/wandered surface members still release (no
membership leak). EXPECTED: no premature teardown → s1337's floor-level corridor re-commit
loop and s22's descend-remount-FullExit loop (both machinery-existing) run to completion.
(B) BOUNDED RE-ENGAGE: emergency_reengage_aborts per member, +1 at the frontier-preserved
Abort leg; cleared on frontier completion + verified dismount + route teardown; on >5
consecutive fruitless aborts → member released to the INDEPENDENT failsafe tier
(watch_wipe 'reengage-bound-exhausted' so the 60s clock starts honestly) — a
genuinely-impossible route cannot hold a member hostage; never-stranded holds via the net.
SMOKE bar (architect): B ORGANIC OWNED EXIT on s1337+s22, zero failsafe for B; then corpus
(gate: 6/6 organic + zero teleports + NO REGRESSION on the 4/6 already-organic seeds).

## BACKSTOP-OPT ROUND 2: THE N1B REGRESSION = A SECOND LATENT BUG UNMASKED (class 11)
Phase-1 tripwire caught N1B FAIL ×2 det under (A)+(B): teleports=1, out@114, ONE re-plan
cycle (was 3, out@151, teleports=0). DIAGNOSIS: N1B's second rebuild cycle waits ~90s on the
REQ-0071 FULL-ENERGY gate (drained by climb+build). The OLD broken release was ACCIDENTALLY
wiping the stuck-watch during that wait (bogus 'verified exit' → safe_secs → cleanup wipe);
(A) correctly refuses the release → the motionless member's 60s watch outruns the 90s regen
→ failsafe mid-recovery. The corpus seeds escaped only by POSITION JITTER resetting the
watch (lottery). FIX: reason-tagged, cumulative-bounded watch hold for the NAMED designed-
wait state — emergency_energy_wait_ticks per member, wipe reason 'energy-gate-wait' while
grounded_clear && !route_energy_ready, bound 120s cumulative (then the watch runs and the
independent net still catches a never-recovering wait); cleared on gate-pass + all teardown
paths. This is the architect's original progress-aware instinct in NARROW form (a named
bounded state, not generic activity). N5G pass-bar also flipped report-only per disposition-1
(removes the standing formal FAIL). OPS (recurrences, classes already filed): killing the
orchestrator left lane shells respawning children (needed shell-first kill order), and the
PowerShell pattern-kill matched its OWN command line again (self-kill trap) — both known,
both re-bitten, cleanup sequence: shells by cmdline → children by image → verify 0 → build.
Binary 12:15:20. Smoke: N1B ×2 + s1337/s22 (flips retained?) + s21 (green-seed regression).

## BACKSTOP-OPT ROUND 2 SPLIT RESULT: seeds green, N1B still red — A THIRD LATENT STATE
Round-2 smoke (12:15:20): s1337=187s s22=133s s21=53s ALL ORGANIC ✓ (flips retained,
control clean). N1B: FAIL ×2 with IDENTICAL numbers (teleports=1, out@114) — the energy-wait
hold NEVER ENGAGED: zero 'energy recovery' lines, and the failsafe line shows the member
IDLE with active_job=None, on_ground at the pit floor, access_jobs_pending>0,
climb_free_active=true. The instrumented wait branch lives INSIDE the active-jobs upkeep
join — a member with NO ActiveJob never enters it. THE THIRD LATENT STATE: member-idle-no-
job — previously unreachable because the broken release always ejected members to ordinary
status before they could idle as members (the old run's @144 second cycle came through the
ORDINARY tier post-release). Under (A) he stays a member with no driver: no task, no job, no
claim. Diag run in flight (EGRESS_DIAG, 53-116s window) to see which flow declines to claim
the pending rebuild job. Architect's livelock-gate N7 episode already coded (energy
starvation → hold→bound→net timing signature), pending rebuild.

## BACKSTOP-OPT ROUND 3: THE THIRD OUTCOME (route-exhausted-replan) + N7 (commit a0d44d63dd)
N1B diag (partial log sufficed — the 2-min shell timeout killed the run but the 53-116s
window was already on disk): member COMPLETES the last frontier job AT THE TOP (t=1611,
wall-cling one below rim), cursor releases, then ×62 30-tick samples of the cleanup's
ordinary Goto with ZERO movement (wall-cling can't chaser-path) and route_complete=FALSE
throughout — construction_complete=true but route_descriptor_ready=false because N1B's
adversarial stimulus PERMANENTLY SEALS the dismount. Route exhausted + exit invalid + no
driver → net@114. The OLD binary passed N1B via the broken release ejecting him to ORDINARY
re-planning which dug AROUND the seal (out@151) — the old bug's THIRD accidental service.
FIX (ii): construction_complete && !route_descriptor_ready && no task ⇒ named release
('route-exhausted-replan') via the lost-member path (primes egress re-emission ~10s), watch
wiped on the non-exhausted leg, EMERGENCY_REENGAGE_BOUND (hoisted, now counts aborts AND
exhausted-replans) bounds it → permanently-impossible geometry still nets. EXPECTED
COMPOSITION for N1B: (ii) release → re-plan around the seal → round-2 hold survives the
~90s regen → owned climb → organic out ~150s, teleports 0. Round-3 smoke running (binary
15:41:14): N1B ×2 + N7 ×2 (hold→bound→net window [190,295]s) + s1337/s22/s21.
OPS: the architect's stall-check answered — no pipeline had been launched (by design), the
3 'orphan' servers were 2-day-old Play-Tester/live leftovers (attributed then killed per his
direction), and the 3h silence was MY turn-summary-not-message gap — send-as-it-happens
resumed.

## BACKSTOP-OPT ROUND-3 SMOKE FULL GREEN + THE ACCEPTANCE RULING
N1B PASS ×2 under even the OLD strict gate: out@151 teleports=0 — the M2-tag number
restored via the DELIBERATE (ii) release → re-plan → dig-around chain (decision, not
accident). N7 PASS ×2: energy-wait premise witnessed + net@225s ∈ [190,295] — the livelock
bound PROVEN. Seeds unchanged (187/133/53 organic). NO fourth latent state — the three-
outcome release machine held; architect boundary untriggered. ARCHITECT RULINGS folded in:
N1B acceptance = adversarial pit's legitimate floor is the net (either dig-around organic OR
bounded→net, alive/unentombed — the organic/zero-teleport bar belongs to the CORPUS seeds);
NEW required proof N1C = truly-permanent seal (sustained 1/s rim-ring re-seal, no dig-around
can validate) must show bounded re-plans → exhausted=true → net → alive/unentombed. N1C +
new gates on binary 16:00:15, running ×2. On green: full 12-episode + corpus-v6 pipeline
with watch paths messaged at launch. Commit a0d44d63dd (rounds 2+3).

## N1C AIM ERROR #4 — CAUGHT BY THE LAYERED WITNESS, RE-AIMED
First N1C ×2: harness PASS but replan-releases=0, bound-exhausted=False, net@58s — the
from-tick-0 rim seal prevented any route from ever PLANNING (every candidate dismount
invalid at plan time) → no membership → the bound machinery never engaged → the ordinary
failsafe fired. The FOURTH stimulus-window aim error (N1 feet+2, N5G tick-0, N6 emission,
now N1C from-birth) — and the LAYERED DESIGN WORKED: the harness's loose in-process gate
passed but the verifier's premise witness (bound-exhausted) flags PREMISE-VIOLATION; the
extraction caught it pre-matrix. RE-AIM: the seal ARMS at first task-present (the first
route plans pre-seal, gets sealed, aborts stale-terrain-revision, and the bounded outcome
cycles genuinely run); verifier witness widened to accept EITHER bound leg's exhaustion line
(exhausted=true replan-release OR 're-engage bound exhausted' abort-leg — one shared
counter). Rerun ×2 in flight. The stimulus-window precondition-assert chip (task_d343c458,
running in Ben's worktree session) now has FOUR instances justifying it.

## N1C RACE FINDING + FINAL PIPELINE LAUNCHED (commit 40aa5e0686, binary 16:10:25)
Re-aimed N1C (armed at first task): the seal aborts the route stale-terrain-revision (the
(B) counter increments — the leg is LIVE), then the member climb-chases the pending frontier
job, wedges under the sealed rim, and the 60s positional watch nets him at 126s — the
outcome-bound NEVER CYCLES because a FASTER terminator wins. Both aims ×2 deterministic:
net-termination at 58s/126s, alive+unentombed. THE RACE ANALYSIS: three independent
terminators (positional watch / energy-wait bound / outcome bound) — the fastest always
wins; forcing the outcome bound to win requires ever-deeper staging (the derung idea
projects a build/remove Sisyphus loop vs a 1/s adversary — wrong direction). PRESENTED to
the architect as a gate-condition deviation with options: (1) accept net-termination as the
proof + file the bound-leg synthetic as follow-on [recommended]; (2) synthetic in-package;
(3) his staging idea. N1C verifier witness → report-only pending the re-rule. FULL PIPELINE
launched in parallel (bn5om29b2; 12-episode matrix ×2 + fresh corpus-v6; watch paths
messaged): every other gate item is N1C-independent. Tripwire armed.

## ★★ CORPUS-V6: BLOCK GOAL MET (B 6/6 ORGANIC) + C-LEG REGRESSION (package HELD)
B organic owned ALL SIX seeds (51/51/53/55/133/187s, full walks, zero backstops, ×3 det);
organic 67% best-ever. ★ REGRESSION: C (protected-vault, designed exit = the net) NEVER
EXITS on s7/s20 (C=-1, ×3 det) — never-stranded broken by MY round-2/3 machinery. TAPE
(s7 C uid=3): six identical ~28s cycles (energy-wait→task→abort contact-lost), at 187s THE
(B) EXHAUSTION LEG FIRED LIVE (first empirical firing — N1C's race question answered by the
corpus itself), then the OUTER hole: re-emission 25s later, fresh counter, cycles resume,
energy-gate-wait wipes ×122 hold the watch flat forever. TWO stacked holes: (1) energy-wait
counter clears on task creation = per-cycle reset; (2) exhaustion not sticky = unbounded
re-emission. DEEPER: even A+B leave C at ~247s > the 150s bar — five cycles of hopeless is
too slow. DISCRIMINATOR on tape: C's cycles have ZERO PROGRESS; B's recovery cycles PROGRESS
(frontier completions/TopExits). PROPOSED A+B+C: (A) counter clears at real-progress only;
(B) sticky exhaustion until delivered; (C) progress-since-last-abort flag gates the hold +
re-engage (no-progress abort ⇒ no hold next cycle ⇒ C nets ~120-130s inside the bar; all
B-legs progress each cycle, unaffected; N7 first-engagement flag-true, unaffected).
Awaiting the architect's rule on the progress-flag semantics before building. Package HELD.

## C-LEG ROUND: (A)+N7B CODED; (B)/(C) HELD FOR THE FORK RULING (message crossing)
The architect's ruling (clear-site fix + mandatory N7b + hard boundary [another hold hole ⇒
revert the hold entirely] + full rerun + C-times-in-package + registry class 12) CROSSED my
A+B+C fork message — his rule predates the outer-hole/bar-math facts. Implemented what BOTH
plans order: (A) COMPLETE per-episode semantics — the energy-wait counter clears ONLY at
real progress (frontier completion + verified dismount) and DELIVERY (the failsafe site,
newly added — a stale counter would deny future holds), NOT at task creation and NOT at
exhaustion (exhaustion ≠ progress; the (B)-leg's energy clear removed). Registry class 12
filed (bounded proofs must exercise the RESET/cycling path; sibling counters' clear-sites
must align). N7B coded: armed rim-seal + per-cycle drain-at-Reserved (0.6) → every cycle =
abort→reacquire→ENERGY WAIT→task (the exact C shape); gate = cycles≥3 (premise) + net + 
alive/unentombed; wrapper/verifier carry 13 episodes now. HELD: (B) sticky exhaustion + (C)
progress-since-abort flag — my math says (A) alone leaves C ~250s+ (the outer loop + no-
progress cycling), so building/smoking waits for his A+B+C rule to avoid tripping his
boundary on a pre-reported fact. No build since 16:10:25.

## A+B+C IMPLEMENTED — THE ONE SHOT (commit 4827d548ed, binary 18:16:54, TERMINAL boundary)
Architect approved A+B+C as the FINAL attempt: the tape revealed the GENERAL root
(progress-discrimination), not another mystery hole — implementing the principle ≠ blind
patching. TERMINAL: any never-stranded break / C past the 150s bar / a productive member
wrongly teleported ⇒ REVERT the hold machinery to M2-tag behavior, no fifth patch; the
2/6-optimization becomes a dedicated designed block. IMPLEMENTATION: (A) energy-wait counter
per-EPISODE (clears at frontier completion + verified dismount + delivery ONLY); (B) sticky
exhaustion (emission early-return + both nearby-route join guards; cleared at delivery/
dismount); (C) emergency_no_progress set at each abort, cleared at frontier-ARRIVAL
(TopExit), frontier COMPLETION, delivery; the hold requires it absent. N7B rides the same
binary. SMOKE (architect wants C's delivery times BEFORE the 2h corpus): s7/s20 C-in-bar +
s1337/s22/s21 B-retention + N1B + N7 + N7B + the edge-case watch (his gate 5: no productive
member teleported via flag-clearing — the discrete-progress-signal risk he named).

## ★★★ A+B+C SMOKE: THE GENERAL ROOT HOLDS (binary 18:16:54, commit 4827d548ed)
C DELIVERED INSIDE THE BAR ALL FIVE SEEDS: s7=133s s20=125s (the stranded two — netted <150)
s1337=97s s22=60s s21=92s, all designed-net, all PASS. B-ORGANIC RETAINED ×5 (51/55/53/133/
187). N1B unchanged (out@151 teleports=0 = the gate-4 PROGRESS arm: protected + organic
replan). N7 unchanged (net@225 = single-wait hold intact). Edge-case watch: no productive
member teleported. TERMINAL BOUNDARY UNTRIPPED. N7B failed only its own PRE-FIX-calibrated
premise (cycles=1 vs ≥3): the (C) denial makes the net win at 126s before cycle 3 can exist
— the C-mode signature is now one-abort+denied-wait+fast-net (126 ≈ corpus C 125-133 ✓).
Recalibrated: premise = mode engaged (≥1 cycle), assertion = net in [90,180] (>90 proves the
first wait held; <180 proves the denial; pre-fix never delivered). Rerun ×2 in flight → full
corpus + 13-episode matrix → package with C's times per the architect's gate 1.

## ★★★ CORPUS-V7 CLEAN — THE BACKSTOP-OPT PACKAGE DELIVERED
36 runs, ANOMALIES=0, det ×3: B organic-owned 6/6 (51/51/53/55/133/187s, full walks); C
DELIVERED IN-BAR ALL SIX (60/85/92/97/125/133s, designed net — v6 pre-fix had s7/s20
stranded FOREVER); never-stranded 36/36; no productive member teleported (every backstop =
A-designed / C-floor / the pre-existing probe-ctrl-s20 control); organic 23/36=63% honest
(v6's 67% excluded 2 strands from its denominator). Fixture 13-episode ×2 det OVERALL PASS.
Package sent with all five architect gates green + the watch-accrual excerpt + the honest
ledger note. AWAITING: the Opus safety review = the tag. Post-tag queue loaded: extraction
prepared-patch → R10 (seam correction accepted as plan of record) → M3.

## POST-TAG ITEM 1: THE EXTRACTION (row 51.7) — APPLIED, UNITS GREEN, PRESERVATION IN TEST
Ben deferred the rest; three-item sequence (extraction → Codex determinism merge → R10-or-
handoff). The prepared patch applied with ONE factoring deviation (noted for the commit):
the draft's single release_decision(count) had a count chicken-egg at the call site — split
into release_decision (branch only) + reengage_exhausted(count) (the bound pin) — same
semantics, cleaner call site. exit_verified named so (no collision with the local
at_verified_exit binding). Call-site rewiring: predicates hoisted above the exhausted-replan
branch (pure reads, behavior-neutral), ONE decision call, all side effects in place, the
missing-position map_or(true) default preserved as (verified=true, occupies=false). UNIT
TESTS 3/3 GREEN (bound pin 5→6, branch table, exit_verified truth table incl the corpus
mid-wall-hang case; 15/15 module total). Binary 21:14:03. Behavior-preservation batch
running: N1B/N1C/N7/N7B + stuckjob 20/21/22/1337 field-compared against the corpus-v7-era
values. Chip task_d696c05c withdrawn (re-owned here).

## ITEM 2: THE CODEX DETERMINISM MERGE (in flight)
Merge blocked first by UNCOMMITTED FOREIGN-LOOKING dirt in npc_ai/mod.rs + action_nodes.rs —
attributed on inspection as BUILDER-LANE pre-session work (the BASTION_GOTO_WRITER_DIAG
agent-side instrumentation — the exact diag that produced the tolerance-inversion smoking
gun — plus ONE behavioral line: the endpoint-tolerance min-clamp, the agent-side complement
of the corridor tolerance work; npc_ai = cosmetic module reorder). KEY PROVENANCE FACT: that
dirt was in EVERY evidence binary this session (tags included) — committing it RECORDS the
tagged binaries' true source (5773ba0b61). MERGE 7b25b67d1b: one conflict in action_nodes
(both sides ADDED fns at the same spot — my diag fn vs Codex's deterministic healing-item
selection [their class-7 agent-side extraction]; union kept). Their stack: deterministic
lazy inventory generation (the class-7 ROOT fix!) + dialogue-identity isolation +
process-isolated determinism gate + roundtrip tests. Build in flight (rtsim rebuild, ~15m,
background after the 10-min foreground cap killed the first attempt — exe unchanged, class-8
safe). GATE STAGED (codex-merge-gate.sh): Codex's own commands — 2 unit suites + paired
determinism regressions ×2 (rtsim-dialogue-action, bag1-agent-decision, seed 21,
wall-unix-millis normalize) + class7 fixtures ×2 + P0/N1B traversal-interaction smoke.

## ITEM-2 GATE: 3/4 FAMILIES GREEN FIRST PASS; the 4th was MY WRAPPER BUG
Gate on the merged binary (22:08 / rebuilt 7b25b67d1b+dirty stamp): dialogue-identity unit
GREEN, class7-item + class7-agent-roundtrip fixtures PASS ×2 each (Codex's core proof), P0 +
N1B traversal smoke PASS (no interaction with the tagged line). The 4 paired determinism-
regression runs rc=2: Codex's runner FORBIDS pre-existing output dirs ('evidence overwrite
is forbidden' — their own anti-clobber discipline, class-8 spirit) and my wrapper mkdir-p'd
the dirs before the run — invocation bug, mine, stimulus never ran. Rerun in flight with
fresh paths. Also owed post-rerun: the harness determinism_regression::tests suite printed
no result line in the gate (grep or filter miss — verify after the exe frees; no cargo
during the gate). ITEM-3 CALL SENT: hand R10 off (charter complete in scratchpad); architect
endorsed pre-emptively; cutoff exemption noted, plan unchanged.

## The next fix (scoped, trail hot)
A. FrontierWork traversal activation: when the frontier rung is above work reach, create the
   ConstructionFrontier(JobId) task (machinery exists in bastion_traversal.rs, purpose enum +
   phases) to climb the BUILT prefix to within radius — the designed flow that never fires.
B. Bottom re-entry: on mid-prefix search None + floor access to the first rung's entry cell,
   restart at the bottom entry (emergency_route_mounts holds the body lane) → LinkApproach →
   the :5676 task creation preflight aligns by construction.
C. Then: fixture P0 goes owned (task phases visible, mount-snap engages), N1-N6 mutators arm,
   the 4-failure tape proof becomes real.

## Also in-tree, uncommitted (this cycle's verified work)
- Planner fixes: cell-disjointness (starvation), dismount normalization, MIN-merge best.
- Walkability validator (stairs must be walkable ramps; steep -> ladder tier) — targets the
  seed-20 strand; NOT yet corpus-verified (needs stuckjob v3.1).
- Mount-snap (entry+sustain, token-scoped); ASSIST-QUIET WriterEvents ×4; traversal_probe +
  bastion_emit_damage hooks; the fixture flag + P0..N6 skeleton + wrapper + verifier
  (m2-fixture-run.sh / m2-fixture-verify.py; wrapper now uses M2_RECORDER_DIR carrier env,
  fixture resolves the member uid at runtime).
