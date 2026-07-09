# B-ASSET1 — Asset Integration Harness + Render Test Arena (game-side block)

> **For Ben:** paste into a game-build session at `E:\veloren-master` (or slot into the batch queue — it's
> INDEPENDENT of the colony-loop chain; buildable any time after B0/B3, alongside B6). Standard block
> protocol: branch → build → self-test → merge → tag `bastion-block-BASSET1`; bookkeeping per the
> mega-prompt; the concurrent asset session's `asset-lab/` files are inputs to READ, never touch/commit.

## WHY
The asset pipeline has produced verified content (static geometry + style + simulated reachability) but **no
generated asset has ever been loaded by the real engine.** `readme/ASSET_DYNAMIC_TEST_SPEC.md` defines the
dynamic tests; nothing runs them. This block builds the game-side bridge — the thing that lets an asset
graduate STATIC → ISOLATED-DYNAMIC (real physics) → and lets Ben *see* any asset in-engine on demand.

## PART 1 — Flagged asset-lab loader (experimental path; vanilla untouched)
- Load assets from `asset-lab/vox/real/` (+ composition manifests) through the game's real ingestion:
  structures via the spot/plot prefab pattern (the `barn.ron` route — verify it in-repo), items/props via
  their native paths. **Behind a bastion flag** (`--asset-lab-load` or config): default off, vanilla asset
  tree untouched, flagless boot byte-identical. Composed assets (manifests of components) either
  pre-flatten to a single .vox at load or place per-component — pick the simpler that preserves markers,
  document the choice.
- **Marker fidelity:** custom_indices in generated assets must resolve exactly as authored (the
  welded-gate-class bug guard, now engine-side): assert every marker byte maps to the intended
  StructureBlock on load.
- Failure mode: a malformed asset logs + skips, never panics the server.

## PART 2 — Dynamic test scenarios (the flat-plane arena, for real)
Implement `readme/ASSET_DYNAMIC_TEST_SPEC.md` in the B0 harness:
- **Arena scenario:** flat superflat test world (or force-flattened region), load the asset under test,
  spawn fixture colonist(s) (B3 machinery), run the assertions with REAL pathfinding + collision:
  reachability to named interior points, traversal (collision box clears doors), arrival within tick
  budget, **egress**, multi-occupancy where relevant, interior function points. Operable assets: run the
  matrix in each state (gate open/closed) once operable parts have engine-side state (until then, test the
  poses as separate static loads).
- **Per-category scenario derivation:** the asset's catalog metadata picks its test cast (house → colonist
  + interior target; gate → colonist + blocked/unblocked check), per the spec.
- **CLI:** `--asset-test <asset-id|all>` → per-asset PASS/FAIL with reasons, machine-readable summary
  appended to `readme/ASSET_INTEGRATION_LOG.md` (append-only). The content-side catalog's READY tag can
  then be upgraded to READY-INTEGRATED by the asset session reading this log — the two agents coordinate
  through the log file, never through each other's code.
- Include one **integrated-dynamic** spot-check: place one asset on real (non-flat) worldgen terrain and
  re-run reachability — the flat plane's blind spot, sampled.

## PART 3 — The RENDER TEST ARENA (Ben's eyes-on environment)
A bootable client mode for human inspection:
- `--asset-arena [asset-id]` (bastion flag): starts the full client into a small flat test world with the
  asset placed at origin. Free god-camera (B1 machinery). Simple controls: cycle assets (from the loaded
  asset-lab set), **spawn/despawn a fixture colonist** at the edge (watch it path in — the dynamic test,
  visually), toggle the debug overlays that exist (identity labels; navmesh if/when available).
- Ben's loop: boot arena → orbit the asset in real engine lighting/meshing (this is also the first check of
  LOD/mesh integrity in-engine) → spawn a colonist → watch it enter → verdict. Five minutes per asset, no
  colony setup needed.
- Keep it minimal — a test chamber, not a feature. No persistence needed; regenerate each boot.

## DONE-WHEN
- Flagless vanilla boot clean (byte-identical asset behavior with flag off).
- At least 3 existing REAL assets (a structure with interior, the wall+gate, a prop) load in-engine, pass
  their arena dynamic tests via `--asset-test`, results logged.
- One asset demonstrably FAILS usefully (pick or make a known-bad one — e.g. the original too-small
  cottage): the harness reports the failure with reasons. A gate that can't fail proves nothing.
- `--asset-arena` boots; Ben can orbit an asset and watch a spawned colonist path into it.
- Integrated-dynamic spot-check on real terrain executed and logged.
- Tag `bastion-block-BASSET1`; findings + the loader's format assumptions documented (the asset session
  reads them back).

## WATCH-ITEMS
- Figure-layer assets (creatures) are a LATER integration rung (need Body/skeleton Rust work — see
  ANIMATION_RESEARCH addendum); this block is world-layer (structures/props/sprites/items). Note it, don't
  attempt it.
- Operable-part engine-side state machine = a future block (pairs with DF-MECH trigger→link→effect); this
  block tests operable assets as static poses only.
- If the spot/plot ingestion path resists runtime (vs worldgen-time) loading, document the seam and the
  workaround chosen — that finding gates how autonomous building will place catalog assets later (B-AG6
  cares deeply).
