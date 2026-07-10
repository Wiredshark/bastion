# B5.8 gate — test results

Branch `bastion/block-B5.8` (off `efc777475a` = `bastion-block-B5.6b-2`).
23 scenario iteration runs total (findings §2c-2e hold the discovery log);
this doc records the FINAL gate.

## 1. Unit tests (`cargo test -p veloren-common --lib`)

**PASS — 117/117** (2.4s), including the new `bastion_vertical_tests` (3):
the ladder route EXISTS in the path graph; a 4-wall without a ladder does
NOT route (no free-climb); scramble reach 3 routes a 3-up, reach 2
(novice) does not. Plus the carve_ramp suite (switchbacks in a narrow
mask, floor rule, reachability order, refusal cases) and the b-2 schema
guards.

## 2. `--b58-scenario` (seed 1337) — run 23, quiet machine (5.9ms avg tick)

**PASS.** Gating asserts all green:
- (a) scramble gauntlet: 1-step + 2-up + 3-up traversed, ZERO carve assist
  (`a_max_total == 1`), climbing XP accrued (skill-on-use live).
- (b1) tight shaft: lured, auto-access fired, LADDER pillar built (the
  geometry choice), zero orphans.
- (b2) roomy Stockpile claim: auto-STAIRS carved through solid
  (switchbacks), colonist out, NO ladder placed — the geometry choice's
  other branch.
- (c) player ladder: material given, 5/5 rungs built bottom-up.
- (d) DF deep dig: 150/150 jobs, ALL cleared, layers finished strictly
  TOP-DOWN, claim dispersion 1.0, post-dig rescue cleared + every digger
  cumulatively surfaced (this run: even the known-open fields were green
  except b_exited/c_top — see below).
- Zero orphaned claims, 5.9ms avg soak tick.

KNOWN-OPEN (reported, not gating — architect-sanctioned descope): the
climb-execution COMPOSITE outcomes (b_exited/b_drained, c_top_cleared/
c_no_carve, d_rescue_cleared/d_all_out). Each passed in ≥3 of 23 runs
(several ≥5, (b1) twice consecutively after the ladder collision waiver
landed); the residue is rotating multi-agent execution jitter owned by
SOFT-COLLISION (COMMITTED at B6, `readme/SOFT-COLLISION-design.md`).

## 3. `--b4-scenario` (seed 1337)

**PASS** — the buried deep job's unreachable invariant now rides the
exposure gate's proactive flagging (same assert, new mechanism; all other
B4 invariants unchanged).

## 4. `--b5-scenario` (seed 1337)

**PASS with the hand-carved quarry EXIT RAMP REMOVED** — scramble covers
the 2-3-block pit exit under its own power: the spec's "the workarounds
become unnecessary" proof, delivered. The quarry also digs exposure-gated
top-down now, and the b-2 slope-coverage phase (72/72 vs legacy 45)
remains green.

## 5. `--b55-scenario` (seed 1337)

**PASS** — exact conservation (200/200) intact; the reach-aware carve
trigger emits no stray spoil.

## 6. Vanilla flagless harness boot + soak

**PASS** — 1000 ticks clean. Player climbing untouched; scramble/ladder
edges gate on `scramble_reach` (colonists only); the phys ladder waiver
requires BOTH parties to be colonists.

## 7. Voxygen compile check

**PASS** (compile-only; the live-build slot for Ben's eyeball is the
test-lane worktree's, per fleet protocol).

## 8. Ben's in-game verify — BATCHED per fleet protocol

TEST LIST goes with the tag ping: 6-deep dig behavior (top-down, spread,
self-laddered/staired), ladder tool + climb, scramble feel/speed
(TRAVEL_SPEED verdict outstanding), climbing-skill growth.
