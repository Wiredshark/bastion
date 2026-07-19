# OVERNIGHT QUEUE — 2026-07-19 (main builder self-advances through this; box-safe SINGLE lane)

Context: fleet went idle ~03:53. Box quiet (~4 procs). checked_tick is WRITTEN but NOT yet
verified/committed. R10 (critical-path safety block) is chartered for a FRESH session. This queue keeps the
ONE reliable overnight build lane (the main builder) productive without box contention. Commit at every
boundary so an interruption loses nothing. Report each step to the architect + append to
MORNING-TRIAGE-2026-07-19.md.

## STEP 1 — finish checked_tick (the task you were mid-flow on)
The box is free now. Verify the reader-guard fix (safe_tick_dt + once-latched warn + the 2 unit tests) and
the residual harness `determinism_regression::tests` result line, then COMMIT citing B16 Tier-2. This closes
the highest-blast-radius crash guard (the ~15 combat states that panicked on a negative buff factor).
→ On commit: message the architect the verdict + evidence; B16 Tier-2 → DONE.

## STEP 2 — the honest R10 call (your decision, ultra-enabled)
R10 = the ownership-epoch fencing token (charter: readme/R10-FRESH-SESSION-CHARTER.md). It is SAFETY-CRITICAL
and the formal model proved every mechanism it fences load-bearing. Make the honest context call:
- **If your context is GENUINELY sound for a fresh safety-mechanism build** → CLAIM R10, build it per the
  charter (you hold box priority), tag. TELL THE ARCHITECT so the fresh-spawn is cancelled.
- **If NOT** (the accumulating context-depth tells — fat-fingered session-ids, transcription slips — are the
  honest signal) → do NOT take R10. Say so, and drop to STEP 3. R10 goes to the fresh morning session.
The discipline rewards the honest handoff; there is no fault in it.

## STEP 3 — (only if you hand off R10) STATUS-SURFACE — clean, context-safe, VISIBLE, no R10-file collision
Build readme/STATUS-SURFACE-BLOCK.md: the four indistinguishable colonist states become inspector status
lines + an energy meter. It REUSES the UI-4/UI-5 BastionInspect transport verbatim (no new wire), touches NO
traversal/protected file (zero collision with R10's turf), and it's the #1 client-surface-audit finding —
it also makes tomorrow's MINING-LIVE-FIDELITY investigation legible. Build → verify via the inspector probe
per the standard → tag. This is the ideal "one more clean block" for a deep context.

## BOX DISCIPLINE (hard)
ONE heavy build at a time. If Ben spawns the fresh R10 session, IT holds box priority — you then do
WRITE-only prep (no cargo) until it tags. Before any cargo build confirm no cargo/rustc/veloren-server-cli/
bastion-harness from OTHER lanes. Never pattern-kill. The 5 chip verifies + Codex DET-0004 stay PARKED for the
morning merge sweep (do not fire them — that would contend the box).

## NOT YOURS overnight (morning / Ben-actions)
- builder-2's leash (npc_ai + rtsim/tick, uncommitted, Opus clean-billed) → morning merge sweep, or builder-2
  self-commits if Ben wakes it.
- Fresh R10 session spawn = Ben's action (paste-ready charter).
- The endpoint float-equality chip (FABLE-005 #1) + the 3 audits = morning (tolerance-adjacent, needs fixtures).
