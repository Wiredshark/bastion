# BASTION PIPELINE — the self-running content/build machine (operating + rollback map)

The map of how work flows from idea → live, autonomously, gated by automated harnesses and made safe by
versioned rollback (NOT by a human approving every step). This is the "how the machine runs itself" doc; the
architect coordinates and reviews in batches, the humans (Ben + architect) are touched only where judgment is
irreducible.

## The stations (each an isolated agent; coordinate ONLY via `readme/` logs, never across code)
1. **DESIGNER** (`GENERAL-DESIGNER-prompt.md`) — turns `[LEDGER]` topics into `[DESIGNED]` blocks w/ Done-whens;
   determines systems + assets + animations + legibility. Isolated (docs only, no code/build/git). Parallel-safe
   (claims topics in `DESIGN_PASS_LOG.md`). → emits design docs + asset requests.
2. **ASSET BUILDER / pilot** (`MASTER-asset-tooling-prompt.md`) — generates + STATIC-verifies voxel assets.
   Isolated (`asset-lab/` + `readme/`, no git). Watches `ASSET_REQUESTS.md`; declares markers in
   `ASSET_MARKER_REGISTRY.md`. → emits assets at `READY-pending-dynamic`. **Holds engine-testing** (see rule).
3. **TESTER** (B-ASSET1, game-side) — engine-loads + real-pathing-tests assets; graduates `READY-pending-dynamic`
   → `READY-INTEGRATED`. Runs in a build slot (serializes with the engine chain). → logs `ASSET_INTEGRATION_LOG.md`.
4. **SYSTEM BUILDER** (`MEGA-PROMPT`) — builds `[DESIGNED]` blocks from Done-whens; checkpoint→build→gate→
   merge-or-rollback→tag. Uses a STAND-IN for any asset not yet `READY-INTEGRATED` and hot-swaps later.
5. **INTEGRATE (LIVE)** — the system builder edits the real game `assets/` manifests to make an
   `READY-INTEGRATED` asset permanent. **Flag-gated + additive** so it's a one-line revert.

## The flow (two parallel tracks converging at integration — pipelined, not serial)
```
 designer ─► ARCHITECT REVIEW ─► [LEDGER]→[DESIGNED] queue
     │              │
     │ (assets)     ▼
     ▼        SYSTEM BUILDER builds ──(needs asset? stand-in, never blocks)──► INTEGRATE (LIVE)
 ASSET_REQUESTS ─► PILOT generates+static ─► holds at READY-pending-dynamic ─►(JIT)─► TESTER graduates ─► READY-INTEGRATED
                                                                                            │
 build findings ──refine──► DESIGNER          system-completion ──unlocks NEEDS:<system> batch──► PILOT
```

## Status vocabulary (ONE state machine every station reads the same way)
`REQUESTED` → `GENERATED`(static PASS) → `READY-pending-dynamic` → `READY-INTEGRATED`(engine-tested) → `LIVE`(in real manifests).
Design items: `[LEDGER]` → `[DESIGNED]` → building → `bastion-block-<N>` (merged+tagged).

## The two rules that make "just run it" safe
- **GENERATE AHEAD, TEST JUST-IN-TIME.** The pilot generates + static-checks on request and **HOLDS at
  `READY-pending-dynamic`** — it does NOT engine-test. Engine-testing (the tester, a build slot) runs only when
  the tester is live AND the consuming system is near the frontier, then batched. Generation is cheap+parallel;
  engine-testing is expensive+serial. Front-load the cheap stage, defer the expensive one.
- **NOTHING GOES IRREVERSIBLY LIVE WITHOUT A CHECKPOINT.** Every station emits a *versioned, revertible*
  artifact; the risky final step (real-manifest integration) is flag-gated + additive = one-line revert.

## Rollback ledger (gate-with-rollback, not gate-with-humans — extends `BASTION_RESTORE_LEDGER.md`)
| Stage | Checkpoint / version | Rollback |
|---|---|---|
| Design | architect batch-commits design docs (each commit = a version) | revert the commit + re-open the topic in `DESIGN_PASS_LOG` |
| Assets | append-only generation log + TEST/REAL + versioned `.vox` files | revert the file / drop the catalog entry; harnesses gate before READY |
| Build | block branch + `bastion-block-<N>` tag + restore ledger | `git reset --hard bastion-block-<prev>` (existing discipline) |
| Integration (LIVE) | flag-gated + additive manifest entry | remove the entry / flip the flag |
The automated gates (asset harnesses; invariant + soak harness for builds; B-ASSET1 marker/pathing) catch most
defects *before* they land; rollback catches the rest *after*, when a batch review or a human notices.

## The irreducible human touches (design FOR finite bandwidth — defer + batch + make revertible, don't eliminate)
- **Architect review** — batched, not per-step; rules the routine calls (principle-applications), escalates only
  genuine forks. Lightweight-gates low-risk designs.
- **Ben's taste** — periodic batch (render sheets / B-TESTBED captures) with revert-if-disliked, NOT a per-item gate.
- **Ben's direction** — rare true forks only, surfaced in the architect's high-level overview.
- **The machine bottleneck is unchanged** — engine builds serialize ~1–2 on one machine; automation widens the
  isolated lanes (design, content), not the engine lane.

## The feedback arrows (so it learns, not just runs)
- **build → design:** block findings/backlog refine the design doc; the designer reads them back (like the pilot
  reads `HUMAN_EDIT_LOG`).
- **system-completion → content:** finishing a system flips its `NEEDS:<system>` assets to READY → the pilot
  generates that batch. (Needs a live trigger — the architect flags it when a block tags.)

## Coordinator (architect) routing rules
Designer output → I vet + flip `[LEDGER]→[DESIGNED]`. Ad-hoc builder asset need → I triage into a
pipeline-ready spec on `ASSET_REQUESTS.md`. Build slots → I sequence (one game-builder at a time; quiet-machine
gates). Batch-commit design docs = the rollback checkpoints. Bring Ben high-level overviews, not per-pass detail.

*Companion to `BASTION_ARCHITECTURE.md` (how the game systems work) — this is how the PRODUCTION MACHINE works.
Conveyor logs: `DESIGN_PASS_LOG.md`, `ASSET_REQUESTS.md`, `ASSET_MARKER_REGISTRY.md`, catalog.json,
`ASSET_INTEGRATION_LOG.md`, `BASTION_RUN_LOG.md`, the backlog.*
