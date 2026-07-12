# Project Bastion — Current Build Block

> The only actionable builder assignment. Builder reads: this file → the block's [Implementation Playbook](BASTION_IMPLEMENTATION_PLAYBOOK.md) entry → its [Design Index](BASTION_MASTER_DESIGN_INDEX.md) resolver → only the named authoritative design sections → the relevant [Shared-Engine Registry](BASTION_SHARED_ENGINE_REGISTRY.md) entries. Sequence authority = [Master Build List](BASTION_MASTER_BUILD_LIST.md) (architect-owned; builder need not read it during implementation).

---

**BLOCK:** FR15-TIGHTDIG — tight-dig stance/reposition/depth locomotion (one class, two fixes)
**MASTER-LIST NUMBER:** 31
**STATUS:** CURRENT
**TAG:** `bastion-block-TIGHTDIG` (on completion)
**ROLLBACK TAG:** `bastion-block-CASE003`

**ONE-LINE OUTCOME:** every reachable tight-dig target is worked — no A*-bob stalls (horizontal) and no wall-hang→teleport in stairless deep pits (vertical); the teleport reverts to a rare backstop.

**WHY THIS BLOCK IS CURRENT:** row 31; depends CASE-003 (tagged).
**HARD DEPENDENCIES COMPLETE:** CASE-003 (safety net + fail-safe standability), DETRNG (deterministic repro for locomotion scenarios).
**NEXT BLOCK FOR CONTEXT ONLY:** LOD-0 (row 32; not actionable).

**AUTHORITATIVE DESIGN:** [Build Review Log §FR15 + §FR15-REFINEMENT](BUILD_REVIEW_LOG.md) (reviewer FEASIBLE-WITH-CHANGES, option (a): TWO per-axis fixes).
**CURRENT CODE-TRUTH:** bastion drives travel via `NpcActivity::Goto` per tick (`bastion_jobs.rs` travel upkeep); vertical staged-routing filters `board.access_anchors` and `unwrap_or(target)`s when none in range; `climb_free_until` + crest-dismount/ledge-snap exist (B-LIVE3/R5/R6); `ActiveJob` is `Copy` (a waypoint Vec breaks it — fixed-size array + len, or audit Copy uses).

**APPROVED CODING SOLUTION (fix-1 FIRST, then fix-2):**
1. Fix-1 (horizontal A*-bob → committed staged path): compute the full path ONCE at claim (deterministic `find_path`, FxBuildHasher per FR1), store waypoints on `ActiveJob`, `Goto` the CURRENT waypoint and advance on arrival; re-compute only when a segment is no longer clear. Works through the Goto bastion owns — NO vanilla Chaser touch. Instrument failure/progress BEFORE changing movement (playbook row: no new velocity shoves).
2. Fix-2 (stairless deep-pit egress): extend `climb_free_until` + a steer to the nearest rim for a colonist done/wall-hanging in a tight pit — per-colonist climb-out (each climbs its OWN wall; no shared-ladder queue-fight BY CONSTRUCTION; the D16 hard constraint). Teleport stays the rare ultimate backstop.
**EXISTING SUBSYSTEMS TO REUSE:** `find_path`, `Goto` drive + arrival/watchdog semantics, `climb_free_until`, crest-dismount/ledge-snap, `egress_scan` rim targets.
**GENUINELY NEW CODE:** waypoint storage/advance on ActiveJob + segment-clear re-check; the pit-egress climb-free grant trigger.

**DO NOT:** touch the vanilla Chaser/A*; add another velocity shove; reintroduce a shared ladder (queue-fight); re-pick stance per tick (R3 oscillation).
**HEADLESS SCENARIO:** b5 phases (tight/deep digs) + `--slope-mine-scenario`/`--floating-block-scenario` (SET-A/B) + b58; instrument bob-counts/wall-hang before vs after.
**ASSERTED INVARIANTS:** all inherited gates green; tight-dig completion (the FR15 target geometry mined out); teleport-fire count REPORTED (expect a drop).
**REVIEW TIER:** self-verify + tag (routine); escalate to the architect only if the mechanism surprises.
**DONE-WHEN:** fix-1 verified (bob eliminated on the repro geometry) → fix-2 verified (stairless pit egress without teleport) → inherited gates green → tagged `bastion-block-TIGHTDIG`.

**POST-GREEN PROCEDURE:** rerun gates; commit; tag; record rollback (→ `bastion-block-CASE003`); flip Master-Build-List row 31 → DONE; update FLEET_STATUS / RUN_LOG / RESTORE_LEDGER; swap this file to the row-32 packet (LOD-0) resolved via Playbook + Design-Index.

---
_Packet staged by the builder at CASE-003 tag time per the architect's post-green step; architect owns the order — reorder/veto via a new packet if this mis-resolves row 31._
