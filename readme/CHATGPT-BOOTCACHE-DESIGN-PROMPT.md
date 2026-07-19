# ChatGPT/Codex Design Prompt — World-Boot Cache (the biggest remaining test-speed lever)

Paste the block below into ChatGPT/Codex (has the repo in Drive). It DESIGNS a determinism-safe
world-boot cache; a builder implements it POST-M3 with an x2 determinism gate. Token-discipline is the
prime directive so output stays canonical + cheap.

---

You are designing a **world-boot cache** for Project Bastion's headless test harness (`bastion-harness`,
Rust). GOAL: the ~65-second world boot dominates every behavioral test cycle now that builds are fast;
snapshot the post-boot state so repeated runs on the SAME world skip that ~65s — WITHOUT breaking
determinism (our entire test discipline depends on bit-identical, seed-reproducible runs). You DESIGN;
a builder implements post-M3 behind a determinism gate.

**PRIME DIRECTIVE — output discipline (token conservation; canonical in this chat):**
- Write the design as Markdown **once**. No duplicate docs/ZIP/regenerated `.md` of prior content.
- Read **only** the source needed; never the whole repo.
- Don't reread your own output; it stays canonical here. Update **deltas only** on later passes.
- One failure report per failed tool op — no retry loops.

**CRITICAL CONTEXT — the naive approach ALREADY FAILED, do NOT re-propose it:**
- A FileOpts map-cache (`map_file`/LoadOrGenerate) was tried and REJECTED. Findings: `map_file:None`
  loads the bundled DEFAULT map and never generated a world — the ~65s cost is **civsim + rtsim
  generation + spawn-chunk generation**, which a file map-cache cannot capture. And `LoadOrGenerate`
  generated a DIFFERENT world (silently swapping terrain under every seed-keyed baseline), plus the
  load path was not internally deterministic (generate-then-save ≠ load). So a shallow map file is a
  dead end. See the warning comment in `bastion-harness/src/main.rs`.

**WHAT TO ANCHOR TO (read only as needed):**
- `bastion-harness/src/main.rs` — the boot path (how a world/server is stood up before a scenario; the
  ~65s is civsim + rtsim gen + spawn-chunk gen, NOT a file load).
- The server/world/rtsim state that constitutes "booted" (what a scenario reads after boot).
- `readme/BUILD-AND-TEST-PROCESS.md` §5 (determinism rules) — the invariant you must not break.

**DESIGN THE SNAPSHOT/RESTORE:**
1. **What to snapshot** — the exact post-boot state a scenario depends on: civsim output, rtsim state,
   generated spawn-chunks, RNG state, tick counter, ECS world — enumerate precisely what must be captured
   for a restore to be indistinguishable from a fresh boot.
2. **Snapshot + restore mechanism** — serialize post-boot → restore into a fresh process; keyed by
   (seed + worldgen params + code version) so a stale snapshot can NEVER be silently loaded (a hash/
   version guard that INVALIDATES on any input change — the map-cache's fatal flaw was silent staleness).
3. **Determinism strategy (the hard part)** — how a RESTORED run stays BIT-IDENTICAL to a fresh-boot run:
   RNG-state capture/restore, HashMap iteration-order determinism, any tick/time seams. The design MUST
   pass an x2 byte-consistency proof (fresh-boot run vs restored run → identical outcome + trajectory
   tapes) — describe how it achieves that, and what could break it.
4. **Scope + fallback** — restore only when the key matches exactly, else fall back to a full boot (never
   silently run a stale/mismatched snapshot). Where the snapshot lives (per-key file / in-memory reuse
   across a corpus batch).
5. **Risks + the acceptance gate** — enumerate the determinism risks; specify the acceptance test (the
   x2 byte-consistency proof + a corpus spot-check that restored ≡ fresh across N seeds).

**OUTPUT:** one design doc with those five sections. Concrete (name the state, the key, the seams), not
hand-wavy. Flag anything you can't determine from the source as an open question for the builder.

---

**How to use:** paste once → ChatGPT returns the design. Review it, then a builder implements POST-M3
behind the x2 determinism gate. The whole point: if it can't prove bit-identical restore, it doesn't ship
(same discipline that gated R10).
