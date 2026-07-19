# SESSION HANDOFF — builder-2, 2026-07-19 (lean-restart session, wound down at the cycle point)

The fresh M3 builder starts HERE with zero archaeology. Branch: `bastion/builder` (worktree
`.claude/worktrees/builder`, own target dir). Everything below is committed + pushed to
`bastion-origin/bastion/builder`; VMs fetch+reset+attest that tip.

## ★ YOUR JOB: M3 (ladder contention)
- START: `readme/M3-BUILDER-PACKET-FINAL.md` — the R9-folded packet. Its R10 fire-condition is
  SATISFIED: `bastion-block-R10` is tagged, Opus-gated PASS, and x2-determinism-proven
  (`readme/R10-TAG-PACKAGE.md` = the full evidence document; the fence code lives in
  `bastion_traversal.rs` + the epoch store/retirement in `bastion_jobs.rs`).
- M3's queue re-election is the EXPECTED next `advance_epoch` call-site — the source-scan pin
  (`r10_retirement_is_sole_removal_and_fence_covers_owned_writers`) will trip and must be
  updated WITH that addition (it's the design, not an obstacle).
- The architect runs the process (BUILD-AND-TEST-PROCESS.md is canonical: §2 local-vs-VM rule,
  §16 never-foreground-wait, grind-limit §9 with the 6-iteration/90-min ceiling, Opus gate at
  safety tags). Sonnet = first-line review; escalation ladder per §7.
- AFTER M3: Ben's explicit gate — the FULL VALIDATION PASS (fan the catalog via vm-jobs.sh with
  per-scenario CANONICAL GATE SEEDS — see test-suite.jobs template; b4=1337 etc.) BEFORE any
  new feature.

## State of everything this session touched
- R10: COMPLETE. Tag `bastion-block-R10` @ 508c8136cb. Gate PASS + x2 IDENTICAL (in-package).
- Leg-C churn: CLOSED @ e25046e6c5 (Sonnet signed off). Root + fix + evidence in registry row
  **B56** (`readme/BASTION_COMMON_ISSUES.md` tail; B55 id deliberately skipped — b55 namespace
  pun). Timeouts 356→94; the unreachable set now goes dormant with evidence-gated re-arm.
- DPA (dig-provisioned access): core landed @ 7c37ddbad5 (rungs plan AND build, material
  governance, always-accessed invariant; F3 reservation-leak fixed — the B17-class find).
  KNOWN-OPEN on the dig-access gate at seed 1337: `b-deep-progress` + `c-widedeep-completes`
  stay red — the leg-C cells are HONESTLY unreachable terrain (gate_held=0, DPA exonerated by
  the CLASSIFY discriminator); completing wide-deep digs needs the mine-ladder's deeper access
  tiers (M4/M5 territory), NOT more DPA fixes. The scenario documents the honest state.
- DPA BEFORE/AFTER VM pair: "before" = task b78s34je8 output (82.4% @ 0710088e); "after"
  DISPATCHED at wind-down → results land in scratchpad `dpa-after-fidelity.log` +
  `dpa-after-digaccess.log` (session a9bf8315 scratchpad). ★ UNINTERPRETED BY DESIGN — your
  first cheap win is reading them against the baselines.
- Mining movement-half: MEASURED not fixed (claims/dig ~2.0, walked/dig 20-25, 283 travel
  timeouts in the fidelity runs) — the stand-and-mine block (DESIGNER-SUGGESTIONS §16 +
  FR15-A/B discipline) is queued AFTER M3 + validation.
- Corpus runner (`--corpus`): parallel seed children + full-stdout echo + wedged-child guard
  (--corpus-child-timeout-mins). KNOWN-OPEN: the 0-CPU spawn-wedge cause (one child froze at
  spawn, killed manually — task #6 notes); serial-vs-parallel byte-proof never run.
- Leg-C diag instruments (all env-gated BASTION_LEGC_DIAG, sim-inert): timeout-firing line,
  TGT-DRIFT detector (scheduler), PATHSTATS + CLASSIFY prints (dig-access leg C).
- b4 scenario: spawn-premise gate added (zero-claims ⇒ INVALID verdict; canonical seed 1337;
  seed 1 = the known-pathological repro).
- Toolbar text-mode ("Aa" toggle) shipped @ 0710088eba; settings-persistence = flagged follow-up.

## Cautions that will bite you if unread
- dig-access leg C is RUN-TO-RUN NONDETERMINISTIC in magnitude (two stable roll modes seen:
  ~324 vs ~425 remaining) AND not cross-machine comparable (doc §5 caveat) — judge it by
  MECHANISM metrics (timeouts/claims/classify), never by completion deltas across machines.
- Worktree-built voxygen exes need the toolchain's libstdc++-6/libgcc_s_seh-1/libwinpthread-1
  DLLs beside the exe (STATUS_ENTRYPOINT_NOT_FOUND otherwise) — any exe handed to Ben bundles
  those three.
- The crashed pre-restart session's ENTIRE scratchpad (all evidence tapes) survives at
  `Temp\claude\E--veloren-master\d3469ddb-*\scratchpad` — check there before declaring
  anything lost.
- sccache stats: idle-restarts wipe the window; Rust hit-rate never cleanly measured.
- `-Z threads=8`: measured ~12% on an 11s incremental check — NOT baked in (busts caches).
  bastion_jobs.rs→own-crate split = the architect-logged top post-M3 compile fix.

## Open loose ends (owner in brackets)
- STATUS-SURFACE live-fire 2/4 → "one run from tag-grade" (pre-restart note) [build lane, cheap]
- Chop-felling live bug = probably Ben's STALE EXE (wire verified correct end-to-end; check his
  exe's compiled_git_hash first) [Play-Tester/Ben]
- Seed-777 leg-A teleport residual (2) — starved-hold gap, Sonnet deemed unrelated to leg-C
  [next dig-access pass]
- DPA-3 lateral galleries (deferred by ruling to Part-B/M5) [design]
- Ben eyeball items: BEN-TEST-CHECKLIST.md backlog incl. M2LADDER climb-out + felling [Ben]
