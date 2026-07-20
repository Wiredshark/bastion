---
status: READY — execute post-M3-tag, quiet tree (efficiency slate item #2)
gate_dependency: M3 tag must land first (no builds on the shared checkout while Builder 3 runs the M3 matrix)
authored: 2026-07-20 (architect, unattended-mode prep during M3 wait)
kind: pure structural refactor — behavior MUST be byte-identical (a fidelity run is the proof)
---

# Crate-split: extract `server/src/bastion_*` → new leaf crate `bastion-server`

## GOAL + WHY (the compile lever, quantified)
`server/src/bastion_jobs.rs` = **13,520 lines** — the largest file in the whole
`veloren-server` crate by 2× (next is `cmd.rs` @ 6,690) **and** the file the
fleet edits most (R10, M3, DPA, no-wood all live here). Today every edit to job
logic recompiles all of `veloren-server` (one of the workspace's biggest crates).

Total bastion-side code inside `veloren-server`: **~18,400 lines** across 12
modules (`bastion_jobs` 13520, `bastion_traversal_tooling` 1868,
`bastion_flight_recorder` 924, `bastion_assets` 791, `bastion_traversal` 231,
`bastion_arena` 222, `bastion_actions` 183, `bastion_path` 173, `bastion_mood`
108, `bastion_flat_arena` 106, `bastion_chop` 101, `bastion_piles` 66).

Extract them into a **leaf crate `bastion-server`** that `veloren-server`
depends on and registers. Then a job-logic edit recompiles only `bastion-server`
+ a server relink — the incremental-build win we want **before** starting the
engine-improvement work (Part-1 ledger). This is efficiency-slate item #2
(item #1 = boot-cache, Codex, merges after this or in parallel — different files).

## PRIOR ART (name it, per house rule)
Standard Rust "hot code → leaf crate" incremental-compile pattern:
- **rust-analyzer**'s crate graph (deliberately many small crates so a change
  recompiles a leaf, not the world).
- **Bevy**'s split into `bevy_ecs` / `bevy_render` / … leaf crates for the same
  incremental reason.
- Veloren already does this: `common` / `common-ecs` / `common-net` /
  `common-state` are leaf crates precisely so a `server` edit doesn't recompile
  `common`. We are applying the *same* pattern to bastion server code — nothing
  novel. `bastion-model` (data) and `bastion-harness` (tests) already exist;
  this adds the missing `bastion-server` (systems/logic) leaf.

## FEASIBILITY (surveyed 2026-07-20 — the coupling is tiny)
`bastion_jobs.rs` has only 2 `use` statements touching intra-crate paths and 37
inline `crate::`/`super::` refs. Enumerated, **almost all point at OTHER
bastion_* modules that move together** (`bastion_actions::*`,
`bastion_flight_recorder::*`, `bastion_traversal::*`, `bastion_mood::*`). All
other imports are already-external crates (`common`, `common_ecs`, `common_net`,
`common_state`, `specs`, `hashbrown`, `vek`, `tracing`) — available to any crate.

**The ENTIRE non-bastion server-internal coupling to resolve is 3 items:**
1. `crate::Tick` — server resource (top of `lib.rs`).
2. `crate::rtsim::RtSim` — server rtsim resource (1 ref; the job Sys reads it).
3. `crate::presence::RepositionToFreeSpace` — server type (1 ref).
Re-run the survey across ALL 12 modules before moving (below) — `bastion_jobs`
is the deepest; the smaller modules may add a few more, but the shape holds.

## THE 3 COUPLING KNOTS — resolution options (builder picks per what the code shows)
- **`Tick`**: small resource. Cheapest = make it `pub` and depend on it, OR move
  the `Tick` type into `common` / `common-state` (it's a plain tick counter).
  Prefer moving to `common-state` if other crates already want it; else re-export.
- **`RtSim`** (the real one): the job `Sys` reads rtsim state. Options, in order
  of preference: (a) the `Sys` takes only the *rtsim data it needs* via a
  `ReadExpect` of a type that already lives in a lower crate (check what fields
  it touches — likely rtsim `Data`/`Npc` already in `rtsim` common crate, not the
  server wrapper); (b) define a small trait in `bastion-server` that
  `veloren-server` impls for its `RtSim`; (c) last resort, make `RtSim` pub.
  **Survey the actual field access first** — the ref count is 1, so this is
  probably a single narrow read, not a deep entanglement.
- **`RepositionToFreeSpace`**: 1 ref, likely a marker/util — move it into
  `bastion-server` or a shared crate, or re-export.

## THE PLAN (ordered)
1. `cargo new --lib bastion-server` (or add manually); add to root `Cargo.toml`
   `[workspace].members`. Deps: `common`, `common-ecs`, `common-net`,
   `common-state`, `bastion-model`, `specs`, `hashbrown`, `vek`, `tracing`,
   plus whatever the smaller modules pull (survey).
2. `git mv server/src/bastion_*.rs bastion-server/src/` and turn the module
   tree into that crate's `lib.rs` (`pub mod bastion_jobs;` …). Keep filenames.
3. Fix the 3 knots (above). Re-point the intra-bastion `crate::bastion_*` refs to
   the new crate root (`crate::bastion_jobs` → `crate::…` stays valid *inside*
   the new crate since they're now siblings — most refs need NO change).
4. In `veloren-server`: delete the `mod bastion_*;` lines from `lib.rs`, add
   `use bastion_server::…` where server code referenced these (grep server for
   `bastion_jobs::`, `bastion_traversal::`, etc. — the reverse coupling), and
   register `bastion_server::Sys` in the dispatcher exactly where it is now.
5. Point `bastion-harness` at `bastion-server` directly if it currently reaches
   through `veloren-server` for the job Sys (BONUS compile win for the test loop —
   verify, don't assume).

## SEQUENCING (hard constraints)
- **AFTER M3 tags.** Do NOT start while Builder 3 runs the M3 matrix on the
  shared checkout — a workspace refactor races the target-dir lock and would
  corrupt the matrix. See [[no-cargo-during-own-gate]], [[concurrent-builder-sessions]].
- **Quiet tree, own commit(s), own gate.** This touches the hottest file — do it
  when nothing else is mid-block, and isolate the commit so a bisect is clean.

## ACCEPTANCE (this is a PURE structural move — prove zero behavior change)
1. `cargo build` (server + full workspace) rc=0.
2. `cargo test -p bastion-server` (moved lib tests, incl. the R10 exhaustiveness
   pin) all pass — the pin must still fire from its new home.
3. A `--mine-fidelity-scenario` run + a `--dig-access-scenario` run on a fixed
   seed produce **byte-identical** output vs. pre-split (structural refactor ⇒
   no behavioral delta; a diff = a mistake in the move).
4. **Compile-win proof**: time an incremental rebuild after a 1-line edit to
   `bastion_jobs.rs` — before vs. after. That delta is the deliverable; record it.

## WHERE TO LOOK
- **START HERE**: `server/src/bastion_jobs.rs` (imports at top: `use crate::{Tick,
  bastion_traversal::…}`; the `Sys` impl + its dispatcher registration in
  `server/src/lib.rs`). First edit = create the crate + `git mv` the 12 files.
- **THEN**: the 3 knots — `Tick`/`RtSim`/`RepositionToFreeSpace` field access;
  the reverse coupling (grep `veloren-server` for `bastion_jobs::` etc.).
- **REFERENCE-ONLY**: `common-state`/`common-ecs` `Cargo.toml` for the leaf-crate
  dep pattern to mirror; `bastion-model`/`bastion-harness` manifests.
