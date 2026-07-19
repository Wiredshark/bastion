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
GUIDING RULE (Ben-directed): test the REALISTIC case at scale before perfecting edge cases of the toy. The
last builder burned 8h hardening adversarial SINGLE-colonist pit scenarios (BACKSTOP-OPT) instead of advancing
to the realistic CROWD case — backwards. Run the bigger, realer test; fix what it actually breaks.
1. ★ MINING-LIVE-FIDELITY — Ben's live bug: a dig completes only ~50/50 + colonists run back-and-forth.
   PRE-DIAGNOSED (DAY-PLAN amendment 2): completion-half = the dormant descent-gate ladder arm (pull
   DIG-PROVISIONED-ACCESS forward — the re-enable IS the fix); movement-half = commit the mining stance to
   block completion (RimWorld stand-and-mine, DESIGNER-SUGGESTIONS §16). MEASURE first, then fix.
2. ★ M3 CONTENTION (pulled forward, Ben-directed) — the DELIBERATE crowd test: many colonists, ONE ladder
   (packet: M3-BUILDER-PACKET-FINAL.md; "the M2 climb test but bigger"). Build the fair-queue + capacity-one
   core (R9-based — does NOT need R10 first), RUN IT under realistic load, see what actually breaks.
   ★ R10 IS NO LONGER A PREREQUISITE: R10 is the fencing-token HARDENING against a stale-write race. Don't
   build it before the crowd test — let M3 reveal whether that race actually bites in practice. If it does →
   R10 next; if M3 runs clean → R10 drops in priority. (The formal model says R10 is needed in theory; the
   live crowd test says whether it's needed in fact — trust the realistic test, per the BACKSTOP-OPT lesson.)
   If reading the packet shows a HARD code dependency on R10's helper, flag the architect — don't silently block.
3. MERGED-PILE-EAT (row 50.7) — mass-hunger death-spiral fix.
4. FOREST-ECONOMY — the first big new feature (DAY-PLAN Phase 3).

## OPERATING RULES (the lean discipline)
- Own worktree + own target dir. BACKGROUND long builds and keep writing — never sit watching cargo.
- Commit at every boundary. CYCLE your session per block — don't run one session 16 hours (context rots →
  the fat-fingered-IDs / transcription-slip failure mode that ended the last builder).
- REVIEW ESCALATION LADDER (KEPT — Ben-directed): Sonnet = first-line on every real change; escalate
  genuinely HARD / safety-critical / gate work up to Opus, and apex / capstone / adversarial to Fable —
  routed VIA the architect, tier by tier (Sonnet→Opus→Fable). What we CUT is the REDUNDANCY (three reviewers
  on the same solid work, review-of-reviews, bookkeeping/350-task churn), NOT the ladder. Routine solid work =
  Sonnet's lean single pass, done. Higher tiers engage ONLY when something is routed up.
- No sub-agents, no fleet SPRAWL. One builder + the Sonnet→Opus→Fable review ladder + Ben testing. Cycle the
  builder per block; fewer concurrent BUILD lanes = no shared-tree collisions.
