# RESTART CHECKLIST — lean single-builder restart (post-crash, 2026-07-19)

Hand this to ONE fresh builder session. Project Bastion = a Dwarf-Fortress-style colony sim forked from
Veloren (E:\veloren-master, branch bastion/block-B6HAUL). The prior ~13-session fleet is retired for a lean
setup: **one builder + one reviewer + Ben testing.** Work directly — do NOT spawn sub-agents or a fleet.

## State (post-crash)
- COMMITTED + SAFE: STATUS-SURFACE (`0bf8d6fb56`), tags M2LADDER / BACKSTOPOPT / CLIMBCAP, checked_tick,
  the determinism fixes, the bastion-model checker, all design + plan docs. Nothing important was lost.
- DIRTY TREE: ~39 uncommitted .rs files — several lanes' WIP tangled in one checkout. Includes builder-2's
  LEASH (rtsim npc_ai + rtsim/tick — Opus clean-billed, coherent) AND some half-done WIP (a `MUSHROOM` edit
  in bastion-harness/src/main.rs that breaks `cargo test -p bastion-harness`).
- BOX: clean (the crash killed everything, incl. a flat-arena build that had hung ~2h in the LTO link).

## STEP 0 — get to a clean committed tree (FIRST)
Triage the 39 dirty files: COMMIT the coherent completed work in its own scoped commits (the leash first),
STASH or fix the half-done WIP (the MUSHROOM harness edit — make `cargo test -p bastion-harness` compile).
Goal: a clean, compilable, committed tree. Then move into your OWN git worktree + target dir so nothing
tangles again.

## STEP 1 — SHARPEN THE AXE (the first real build; do before any feature)
Make the test loop fast — this is why a build hung 2 hours. (Detail: MORNING-TRIAGE-2026-07-19.md build-speed task.)
1. Add a test profile with `lto = false` (root Cargo.toml) so verification builds skip the LTO link tax.
   (Scheduled change — it busts warm caches = one full rebuild; do it deliberately.)
2. Confirm the FLAT WORLD works LIVE: fresh voxygen build → check the server boot log for
   `flat_arena_enabled=true` (server/src/lib.rs:541) → confirm spawn on a flat grass slab. The fix is committed
   (`1d693b6b2b`) but was NEVER live-confirmed. This gives a seconds-boot testbed.
   ★ HEADLESS decisive test (from the retired Play-Tester, PLAYTEST_REPORTS.md): boot
   `--asset-arena --bastion-flat-arena` — asset-arena AUTO-ENTERS a world so the server thread spawns and the
   lib.rs:541 diag fires with NO menu clicking. That's the automatable check; the bool settles it either way.
Result: every later build is fast, and there's a real fast testbed to verify in.

## STEP 2 — FEATURES (on the now-fast loop; order + detail in DAY-PLAN-2026-07-19.md)
1. ★ MINING-LIVE-FIDELITY — Ben's live bug: a dig completes only ~50/50 + colonists run back-and-forth.
   PRE-DIAGNOSED (DAY-PLAN amendment 2): completion-half = the dormant descent-gate ladder arm (pull
   DIG-PROVISIONED-ACCESS forward — the re-enable IS the fix); movement-half = commit the mining stance to
   block completion (RimWorld stand-and-mine, DESIGNER-SUGGESTIONS §16). MEASURE first, then fix.
2. MERGED-PILE-EAT (row 50.7) — mass-hunger death-spiral fix.
3. FOREST-ECONOMY — the first big new feature (DAY-PLAN Phase 3).

## OPERATING RULES (the lean discipline)
- Own worktree + own target dir. BACKGROUND long builds and keep writing — never sit watching cargo.
- Commit at every boundary. CYCLE your session per block — don't run one session 16 hours (context rots →
  the fat-fingered-IDs / transcription-slip failure mode that ended the last builder).
- Reviewer-check the genuinely risky changes; skip the review-of-reviews ceremony and the 350-task churn.
- No sub-agents. No fleet. One builder, one reviewer, Ben tests. Fewer lanes = no shared-tree collisions.
