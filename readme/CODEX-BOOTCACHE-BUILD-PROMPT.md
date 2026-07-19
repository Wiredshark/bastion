# Codex BUILD Prompt — World-Boot Cache (parallel lane, isolated, gated merge)

Paste into Codex. It DESIGNS + IMPLEMENTS the world-boot cache on its OWN branch, in parallel with the
M3 builder, and PROVES determinism-neutrality — then hands off; the architect gates the merge (post-M3).

---

You are building a **world-boot cache** for Project Bastion's headless test harness (`bastion-harness`,
Rust). The ~65-second world boot dominates every behavioral test cycle; snapshot the post-boot state so
repeated runs on the SAME world skip that ~65s — WITHOUT breaking determinism (the whole test discipline
depends on bit-identical, seed-reproducible runs).

★ ISOLATION (mandatory — another builder is actively on M3 on `bastion/builder`):
- Work on YOUR OWN branch `codex/boot-cache` in your OWN worktree + own target dir. Do NOT touch, commit
  to, or merge into `bastion/builder`. Your commits stay on your branch (ungated there).
- When done, STOP and hand off to the architect — do NOT merge yourself. Merge is GATED (after M3 lands +
  your x2 gate passes + architect's determinism review). This keeps the unproven cache away from M3's
  live test evidence.

OUTPUT/TOKEN DISCIPLINE: write designs/reports once; read only the source you need; don't reread your own
output; deltas only; one failure report per failed op, no retry loops.

★ DO NOT RE-PROPOSE THE NAIVE APPROACH — it already failed: a FileOpts map-cache was rejected because
map_file:None loads the bundled default map (never generates), the ~65s is civsim + rtsim gen +
spawn-chunk gen (NOT file-cacheable), and LoadOrGenerate swapped terrain + wasn't internally
deterministic. See the warning comment in bastion-harness/src/main.rs.

ANCHOR (read only as needed): bastion-harness/src/main.rs (the boot path); the server/world/rtsim state a
scenario reads after boot; readme/BUILD-AND-TEST-PROCESS.md §5 (determinism rules).

STEP 1 — DESIGN (write it to readme/BOOTCACHE-DESIGN.md on your branch), five sections:
1. WHAT to snapshot — enumerate the exact post-boot state (civsim, rtsim, spawn-chunks, RNG state, tick,
   ECS world) so a restore is indistinguishable from a fresh boot.
2. SNAPSHOT + RESTORE mechanism, keyed by (seed + worldgen params + code version) so a stale snapshot can
   NEVER be silently loaded (hash/version guard invalidating on ANY input change — silent staleness was
   the map-cache's fatal flaw).
3. DETERMINISM STRATEGY (the hard part): how a restored run stays BIT-IDENTICAL to a fresh boot — RNG
   capture/restore, HashMap iteration order, tick/time seams.
4. SCOPE + FALLBACK: restore only on exact key match, else FULL boot (never silently run stale/mismatched).
5. RISKS + the acceptance gate.

STEP 2 — IMPLEMENT it (behind a flag/opt-in so a fresh boot stays the default path).

STEP 3 — PROVE determinism (MANDATORY — no ship without it): an **x2 byte-consistency** proof — a
FRESH-boot run vs a RESTORED run of the same seed produce BYTE-IDENTICAL outcome + trajectory tapes (after
the standard wall_unix_millis normalization). PLUS a corpus spot-check: restored ≡ fresh across N seeds.
If you CANNOT prove bit-identical restore, the cache does NOT ship — report the divergence, don't force it.

STEP 4 — HAND OFF: commit design + impl + the x2 evidence to `codex/boot-cache`, then STOP and flag the
architect with: the design, the x2 proof result, and any open determinism risks. The architect reviews
(determinism-critical) + gates the merge into the main line post-M3.
