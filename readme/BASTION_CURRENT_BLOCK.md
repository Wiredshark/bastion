# Project Bastion — Current Build Block

> The only actionable builder assignment. Builder reads: this file → the block's [Implementation Playbook](BASTION_IMPLEMENTATION_PLAYBOOK.md) entry → its [Design Index](BASTION_MASTER_DESIGN_INDEX.md) resolver → only the named authoritative design sections → the relevant [Shared-Engine Registry](BASTION_SHARED_ENGINE_REGISTRY.md) entries. Sequence authority = [Master Build List](BASTION_MASTER_BUILD_LIST.md) (architect-owned; builder need not read it during implementation).

---

**BLOCK:** CASE-003 — chokepoint-wedge / soft-collision terrain-embed fix
**MASTER-LIST NUMBER:** 30
**STATUS:** CURRENT (in flight)
**TAG:** `bastion-block-CASE003` (on completion)
**ROLLBACK TAG:** `bastion-block-DETRNG` (merge 0ce3517b71 — last green)

**ONE-LINE OUTCOME:** soft-collision can no longer leave a colonist's torso-center embedded in terrain at a 1-wide egress chokepoint — the entombment-adjacent safety invariant is restored by construction.

**WHY THIS BLOCK IS CURRENT:** active SAFETY fix (entombment-adjacent, ours), triaged CURRENT; must land green before the sequence advances.
**HARD DEPENDENCIES COMPLETE:** DETRNG (deterministic rtsim RNG → `--deterministic-rtsim` makes the wedge reproducible for confirm + fix verification).
**NEXT BLOCK FOR CONTEXT ONLY:** FR15-TIGHTDIG (staged; not actionable until CASE-003 is green + reviewed + tagged).

**AUTHORITATIVE DESIGN:** [Bug Investigation Log](BUG-INVESTIGATION-LOG.md) §CASE-003 + [Build Review Log](BUILD_REVIEW_LOG.md) §BC-003 (reviewer mechanism-confirm).
**SUPPORTING DOCUMENTS:** [SOFT-Collision design](SOFT-COLLISION-design.md) (SOFT-0/1).
**CURRENT CODE-TRUTH:** SOFT-0 soft-collision in `common/src/states`/physics `phys/mod.rs` (~:535–636), `SOFT_PUSH_FACTOR=0.15` (:626). `box_voxel_collision` terrain resolution caps at 16 attempts and can EXIT still penetrating (vanilla limitation we expose); phys order = pushback-velocity → terrain-resolution. The `surface_teleport_dest` backstop is a slow job-watchdog, not a per-tick net.

**APPROVED CODING SOLUTION:**
1. Seed-sweep `--chokepoint-scenario` under `--deterministic-rtsim` to a failing seed + instrument the trip tick — CONFIRM the soft-collision-vs-wall mechanism before touching physics (no tuning on a hypothesis).
2. Fix: terrain-aware soft push (do not push a capsule laterally into a wall when there is no open lateral direction) + a per-tick **CENTER-SAFETY-NET** (center-in-terrain → nudge to nearest standable = "entombment impossible by construction" belt) + no hard snap-back on `soft_until` expiry mid-overlap.
**EXISTING VELOREN SUBSYSTEMS TO REUSE:** `box_voxel_collision` terrain resolution + the phys pushback pass.
**FILES AND SYMBOLS TO START FROM:** `phys/mod.rs` (SOFT-0 ~:535–636, `SOFT_PUSH_FACTOR` :626); the `--chokepoint-scenario` harness.
**GENUINELY NEW CODE:** the per-tick center-safety-net guard + the terrain-aware push-direction check.
**SHARED ENGINES TO EXTEND:** physics soft-collision (SOFT-0).

**DATA MODEL:** none new. **SYSTEM ORDER:** pushback-velocity → terrain-resolution → (new) center-safety-net. **CLIENT/UI WORK:** none. **PERSISTENCE:** none. **LOADED LOD:** loaded colonists in the sim. **ABSTRACT LOD:** n/a.

**DO NOT:** weaken the entombment invariant to pass a test; blind-tune constants without the deterministic repro; fork a second collision resolver.
**KNOWN FAILURE MODES:** ~5-colonist overlap in a 1×1 egress + `soft_until` expiry mid-overlap → full-force push into a wall with zero lateral room.
**GRACEFUL-DEGRADATION BEHAVIOR:** a center-in-terrain colonist is nudged to the nearest standable cell — never left embedded.

**HEADLESS SCENARIO:** `--chokepoint-scenario` under `--deterministic-rtsim`, seed-swept to the failing geometry (deep/1-wide egress with a funneling crew).
**ASSERTED INVARIANTS:** `ck_in_terrain == 0` across the deterministic seed-sweep; fail-safe egress holds; item/entity conservation unaffected.
**INHERITED REGRESSION GATES:** b4 / b5 / b5.5 / b5.8 / CK / CAVEIN / COORD / VANILLA.
**MANUAL EYEBALL:** checkpoint-batched (spawn a crew through a 1-wide egress).

**REVIEW TIER:** **OPUS** (safety / physics-collision / entombment class).
**DONE-WHEN:** deterministic repro confirmed → fix implemented → `ck_in_terrain=0` across the seed-sweep → all inherited gates green → Opus-reviewed → tagged `bastion-block-CASE003`.

**POST-GREEN PROCEDURE:**
- rerun all gates;
- complete required review (Opus);
- commit;
- tag `bastion-block-CASE003`;
- record rollback (→ `bastion-block-DETRNG`);
- update [Master Build List](BASTION_MASTER_BUILD_LIST.md);
- update FLEET_STATUS.md;
- update BASTION_RUN_LOG.md;
- update BASTION_RESTORE_LEDGER.md;
- update BASTION_CONSISTENCY.md when drift occurred;
- replace this file with the next approved block packet (FR15-TIGHTDIG).

---
_CASE-003 is already in flight (builder working it from the pre-regime queue) — this is the formalized record, not a new instruction; its active work is undisturbed._
