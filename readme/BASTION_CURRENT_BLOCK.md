# Project Bastion — Current Build Block

## ⛔ BUILDER HARD RULES — apply to EVERY block, no exceptions
- **NO SUB-AGENTS, EVER.** Never spawn `Task` / `Agent` / `Workflow` or anything that starts a second agent context — permanently BANNED. Do 100% of the work in your own single context.
- **You BUILD, you don't interpret or evaluate.** Build from the crafted packet below; don't re-derive it from the design docs. Not confident on the APPROACH → one-line flag to the architect (that's the router = the token budget), never grind variants.
- **Prompt supply = Sonnet DIRECT.** From the NEXT block onward, pull your next-block prompt from the Sonnet reviewer (`local_5f3f9b01`) directly. Through the ARCHITECT only: anything needing OPUS (safety gates), commissioning heavy review, or an ORDER question.
- **Never idle-wait — PIPELINE PROTOCOL (built-in reflex):** the moment you kick off ANY long-running background task (a full gate, a release build, a long test), BEFORE you wait, REQUEST a PARALLEL-FILL prompt from Sonnet (`local_5f3f9b01`): "gating block X touching files Y — give me an independent fill task in non-colliding files." Sonnet picks a queued item + crafts it. You ALWAYS have fill work; you NEVER sit idle on a running task. (Short compiles of a few seconds — just wait.) **Low token:** terse bookkeeping (append, don't re-narrate).
- **TRACEABILITY when juggling (main block + fill) — never lose a thread:** keep a one-line WIP-STATE for EACH in-flight task — `<block> | <files> | <resume-point>` (e.g. `LOD-1 | bastion_jobs.rs | committed 51150, gating→on-green tag / on-red fix`). Fill work stays in its OWN files + its OWN commit/branch — NEVER mix two blocks' changes in one commit. When a background gate returns: finish the MAIN block first (tag or fix), THEN resume the fill from its WIP note. If you can't cleanly separate the two, that's a flag-to-architect, not a guess.

---

**BLOCK:** LOD-1 — Atomic Loaded↔Simulated transition + dupe guard
**MASTER-LIST NUMBER:** 33
**STATUS:** CURRENT
**TAG (on completion):** `bastion-block-LOD1`
**ROLLBACK TAG:** `bastion-block-LOD0` (bce7ecfc68 — last green)
**REVIEW-TIER:** self-verify + tag (Sonnet milestone-rollup pass at the LOD-cluster close; NOT Opus — dupe-guard is correctness, not entombment/panic/safety-net).

### GOAL
No colonist is ever processed by BOTH the Loaded ECS tier (the job board) AND the Simulated rtsim tier in the same tick. Closes the transition dupe window: once `npc.mode` flips to `Simulated`, the ECS entity persists until its deferred `DeleteEvent` is consumed — and today `bastion_jobs::Sys` can still claim/progress/complete a job for it (incl. mid-`Arrived`, which can emit an item drop for an entity rtsim now considers Simulated). That's the real bug to close — not hypothetical.

### METHOD
- Gate `bastion_jobs::Sys` on `Loaded` (spec §5D "gate on Loaded" — the impossible-by-construction fix, NOT a dispatch-order fix). Read each candidate entity's `RtSimEntity` component → look up its `Npc.mode` → EXCLUDE non-`Loaded` entities from BOTH the claim/arbitration loop AND the `ActiveJobState::Arrived` progress/completion loop.
- No existing helper does the mode-lookup yet (verified: `RtSimEntity` is only read in `sys/agent/behavior_tree/mod.rs` + `sys/msg/gizmos.rs`, neither for mode-gating) — add the lookup.
- DO NOT touch the demote-flush / `DeleteEvent` mechanism (vanilla-shared, D12) — gate Bastion's own consumer instead.
- REUSE the claim sweep (the `// ── Claim sweep` comment-anchor block, `tick.0 % ARBITRATION_INTERVAL == 3`) for `ActiveJob` release on demote — it already works; regression-guard only, no new despawn logic.
- ⚠ PROOFREAD NOTE (Sonnet, vs live code): the spec §7 seam citation (extend `hook_rtsim_entity_unload`, `mod.rs:371`) is STALE — the builder's own LOD-0 diff put the demote flush in `tick.rs` `Sys::run`'s `SimulationMode` match block. Attach LOD-1 to the `bastion_jobs::Sys` gate + reference `tick.rs`'s mode-match, NOT `mod.rs:350/371`.

### WHERE TO LOOK (cite by symbol/anchor — these files churn, lines rot)
- `server/src/bastion_jobs.rs` — `Sys::run`'s claim/arbitration loop AND the `ActiveJobState::Arrived` progress/completion arm (both need the Loaded-gate); the `// ── Claim sweep` anchor.
- `server/src/rtsim/tick.rs` — `Sys::run`'s `SimulationMode::{Loaded => …, Simulated => { flush + DeleteEvent }}` match block (REFERENCE ONLY — confirm ordering; don't duplicate its logic).
- `rtsim/src/data/npc.rs::SimulationMode` (L43, stable) — the mode enum; `RtSimEntity` comp — the entity→npc link to read.

### INVARIANTS (assert in the gate)
- No colonist processed by both tiers in any single tick — including mid-`Arrived` progress/completion after a demote.
- No stuck `ActiveJob` claim survives a demote (claim sweep — regression-guard only).
- The Loaded-gate must NOT touch the demote-flush / `DeleteEvent` mechanism (vanilla-shared).

### DONE-WHEN
A scenario that demotes a colonist WHILE it is `Arrived` / mid-progress on a job (not just idle) asserts ZERO progress / completion / item-drop happens after the mode flip, across a rapid load/unload cycle. (Narrower than the full `--lod-soak-scenario` — that's LOD-3's combined gate; LOD-1 needs its own scoped assertion now.) All 8 inherited gates + this new leg green → tag `bastion-block-LOD1`.

### POST-GREEN
Tag → one-line RUN_LOG + RESTORE_LEDGER (rollback → `bastion-block-LOD0`) → then pull the NEXT block's prompt from Sonnet (`local_5f3f9b01`) DIRECT per the new workflow. The architect flips the master-list rows; you don't touch that doc.
