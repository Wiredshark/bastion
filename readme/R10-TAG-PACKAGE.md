# R10 TAG PACKAGE (for the architect's Opus gate) — DRAFT, fills as evidence lands

## What R10 is
Ownership-epoch fencing token at the bastion owned-write seam (distributed-lock prior art).
Stale/delayed/duplicate traversal movement writes become impossible BY CONSTRUCTION: every owned
writer presents `TraversalAuthority{link_id, epoch, member}`; the JobBoard's `link_epochs` store
advances ONLY at release-class events; `fenced_movement_write` validates-then-writes at ONE choke
point (stale = logged no-op + recorder event, never a panic, never blocks a valid write).

## Commits (all on bastion/builder, auto-pushed)
- 7caa3b83c3 — docs recovery (plan of record + M2 findings, crash survivors)
- 0877488cae — milestones 1-3: types + truth-table, retirement choke point (5 sites), phase-machine
  fence (13 sites, 2 deliberate raw terminals) + riders (b4 premise gate, corpus echo, LEGC diag)
- db3297713c — milestones 4-5: recorder v2 (ownership_epoch + climb_token_witness + stale-write-
  rejected event; v1↔v1 comparator untouched) + REC-2 exhaustiveness source-scan pin
- ea73cbc4ef — REC-1 CLOSED as a REAL find (despawn sweep; lost_members provably did NOT cover
  entity-less members — task lingered, epoch never advanced)
- <pending> — N-FENCE fixture episode + probes (bastion_traversal_authority /
  bastion_r10_stale_write_probe — the PRODUCTION fence is the tested path)

## The 4+1 Opus pre-review RECs — status
1. Despawn advance-site: CLOSED (audit found it genuinely uncovered; per-pass sweep added). ✔
2. Exhaustiveness assert: CLOSED (source-scan pins: raw remove==1, advance sites==1, fence==13). ✔
3. Bounded post-handoff drive: IN the NFENCE pass bar (move ≥0.5 within 300 ticks of re-engage). ⧗
4. Recorder v2 separate schema: CLOSED (sample/v2 + event/v2 additive; serde-default reads v1). ✔
5. (F1) climb token witness: CLOSED (Some(owned)/Some(false)=vanilla-leak tell/None). ✔

## Locked semantics honored
- Adopt-on-acquire (all 4 construction sites read current_epoch; advance never on acquire).
- Advance on release-class only: frontier-complete / abort-teardown / verified-dismount /
  lost-member / failsafe-delivery / member-despawned (+ M3 re-election later, pin updates).
- Rejection = logged no-op with both tuples (tracing + recorder v2 event). No panic path exists.
- R9-ordering nuance: link ids stay owner-derived until M3's persistent links; epoch semantics
  unchanged by that migration (per-link monotone counter).

## Evidence (fills in)
- [x] authority_valid unit truth table (6 rows incl. future-epoch exactness): 1/1 green
- [x] r10_retirement_is_sole_removal_and_fence_covers_owned_writers: 1/1 green
- [x] NFENCE episode LOCAL (seed 1337): PASS — captured link=1 epoch=0; stale (false,false) =
  byte-clean rejected no-op; fresh accepted; handoff 7 ticks; alive+unentombed (nfence-2.log)
- [x] NFENCE episode ON-VM @ 508c8136: PASS with identical tuple numbers — the fence proof
  reproduces cross-machine (vm-jobs batch A, SHA-attested)
- [~] Regression net batch A @ 508c8136 (VM, canonical seed 1337): P0 PASS · P0G PASS · N1B PASS ·
  N1C PASS · NFENCE PASS · N1 FAIL-AS-BASELINE (the ruled known-open vanilla-leak, fork #15 —
  fixture's own M2-N1-RED-FLAG printed, per the M2 tag ruling [M2-FIXTURE-FINDINGS :117-122:
  "successful exit despite the permanent seal prints M2-N1-RED-FLAG → escalate"]; outcome shape
  identical to tag-time = the no-op property HOLDS for N1) · N2 CREATE_FAIL (VM quota infra,
  retried in batch B)
- [x] Regression net COMPLETE — 14/14 baseline-consistent at canonical seed 1337:
  VM @508c8136: P0, P0G, N1B, N1C, NFENCE, N3, N6, N4, N5G (batches A/B/C; SHA-attested)
  LOCAL @508c8136(+committed inert diag): N7B, N2, N5, N7
  N1 = FAIL-AS-BASELINE (ruled M2-N1-RED-FLAG known-open, fork #15, zero R10 involvement,
  formally reported to the architect per its ruling)
  → THE NO-OP PROPERTY HOLDS: every episode verdict identical to its pre-R10 state.
  (VM create-flake note: ~8 CREATE_FAILs across batches from quota/teardown races — all episodes
  ultimately ran; wrapper retry-backoff + guard fix landed mid-stream, architect-owned.)
- [x] N-FENCE race proof: PASS locally AND on-VM with identical tuple values.
- [~] Corpus determinism spot (x2 same-seed byte-consistency): NOT separately rerun this cycle —
  DETRNG rides the fixture family as always; flag if the Opus gate wants the explicit x2 pair,
  it is one command.

## TAG
bastion-block-R10 at 508c8136cb (the R10-completion commit; the two later commits are leg-C
diagnostic instruments outside R10 scope).

## Known interactions / honest notes
- The fence's current_member derivation excludes Abort-phase tasks → an aborted task's writes are
  fenced the same tick, BEFORE retirement even runs (defense in depth vs the retirement sweep).
- Complete/Abort terminal zeroing writes are DELIBERATELY unfenced (the safe state a dead task
  must leave; fencing them would preserve the last driven input — the inverse of safety).
- The stale-write probe writes a sentinel input on ACCEPT — in the NFENCE episode the fresh-tuple
  accept perturbs one tick of a live member (overwritten by the owned writer next tick); confined
  to the fixture.
- Live-fire expectation: zero `stale-write-rejected` events in all regression tapes (the no-op
  property). Any nonzero count in a NON-fixture tape = a real caught race = exactly what R10 is for.

## x2 VERDICT (the Opus-gate condition � CLOSED)
X2-VERDICT: IDENTICAL. Two P0 runs, seed 1337, one quiet ephemeral VM (fresh from bastion-golden,
trap-deleted after). Every artifact byte-identical post wall_unix_millis normalization:
events.jsonl, summary.json, trajectory.csv, trajectory.jsonl. Both runs rc=0.
ATTEST: RAN_COMMIT=911746f4. Instrument: x2-pair.sh (committed alongside).
ARCHITECT CLEARANCE: M3 GO (held for a fresh builder per the session-cycle ruling).
