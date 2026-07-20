# ENGINE-OPT-1 — A* pathfinding determinism + fallback correctness

**Phase:** engine-optimization #1 (first block of the ChatGPT engine-deep-dive ledger work).
**Source:** IMPROVEMENTS-LEDGER items 175 + 177 (A* run-08 additions).
**Scope:** `common/src/astar.rs` only (the vanilla A*). Bounded, testable, high-leverage — the colony sim pathfinds constantly.
**Branch/tag:** off bastion/builder tip (a643d8dee6 base + B58 @8e0e3bc03d); tag bastion-block-ENGOPT1.

## Why this first
Pathfinding is the hottest shared substrate in the colony sim. Two concrete, bounded, testable improvements — one determinism, one correctness — that complement Codex's data-layer determinism sweep without colliding (different file/mechanism).

## The two items

### (177) Stable frontier total-order key — DETERMINISM
Today `PathEntry`'s `Ord`/`Eq` don't define a *total* deterministic order, so equal-`f` frontier ties resolve in a non-deterministic order → the same search can expand nodes in different orders run-to-run → non-reproducible paths. That's a determinism hole in pathfinding (the twin of the HashMap-iteration holes Codex is closing in rtsim/common).
- **FIX:** give `PathEntry` a **total key `(f, h, g, node-coordinate, insertion-sequence)`** with coherent `Eq`/`Ord` (Ord consistent with Eq; no `PartialOrd`-only or NaN-fragile float compares — use a stable total float order or fixed-point). Prior art: lockstep deterministic priority queues.
- **Symbols:** `common::astar::PathEntry` (its `Ord`/`Eq`/`PartialOrd`), the binary-heap frontier in `Astar::poll`.
- **TEST:** same seed + same graph ⇒ **identical expansion order and identical resulting path across N≥3 runs** (byte/структure-identical). Add a unit/property test that two runs of a fixed search produce identical node-expansion sequences.

### (175) Fallback best-neighbor correctness
On search exhaustion (no full path), A* returns a partial/fallback path — but it must return the path to the **actual best-so-far** node (min `h`, tie-broken by the total key), not a stale or wrong neighbor.
- **FIX:** track and store the **actual best neighbor** (monotone best-so-far by `h`, then the total key) and return the reconstructed path to it on `Exhausted`/partial.
- **Symbols:** `common::astar::Astar::poll`, `closest_node`.
- **TEST:** **monotonic best-so-far** — over the search, the recorded best-`h` never worsens; on exhaustion the returned partial path ends at that best node. Property test on a graph where the goal is unreachable (assert the fallback endpoint is the provably-closest reachable node).

## Acceptance (this is CORRECTNESS + DETERMINISM, prove both)
1. `cargo check --workspace` + `cargo test -p veloren-common` (astar tests) green.
2. **Determinism:** the new deterministic-frontier property test passes (identical expansion order ×N).
3. **Correctness:** the monotonic best-so-far + closest-fallback property tests pass.
4. **NO REGRESSION to the colony sim:** run the bastion harness fixtures that exercise pathing (`--mine-fidelity`, `--dig-access`, the M3/N2 ladder fixtures) and confirm they still PASS. NOTE: paths may *change* (177 makes tie-breaking deterministic — previously-arbitrary ties now resolve one fixed way), so byte-identical vs the OLD binary is NOT expected; what's required is (a) the fixtures still pass their invariants, and (b) the NEW binary is now itself reproducible run-to-run (byte-identical across its own reps — an improvement).
5. Vanilla smoke: server boots, an NPC paths normally.

## Guardrails
- `astar.rs` is vanilla-core (all entities path through it) — this is the "sensitive vanilla-engine" tier. Self-gate hard (the pathing fixtures above), and I (architect) review at the tag before merge. The never-stranded net + the fixtures bound the risk.
- Keep it to these two items. Item 176 (frontier decrease-key/reopen) is a bigger algorithmic change — deferred to ENGINE-OPT-2 if these land clean.
- Prior-art-first: name what you adapted (Hart–Nilsson–Raphael A*, lockstep deterministic priority queues).
