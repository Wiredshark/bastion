# BASTION B-ASSET1 — gate evidence (self-test results)

Block: asset integration harness + render arena. Branch `bastion/block-BASSET1`
(built in the isolated worktree `.claude/worktrees/basset1`, own cargo target,
per FLEET_STATUS isolation map). Raw per-asset JSON: `readme/ASSET_INTEGRATION_LOG.md`
(append-only, in the primary tree — the pilot's read-back channel).

## Done-when → evidence

1. **Flagless vanilla boot clean.**
   - Headless server: B4/B5/B5.5 scenarios + baseline harness boot exercise the
     flagless server end-to-end (below). `veloren-server-cli --no-tui` smoke boot
     from the worktree build: see §4.
   - Voxygen: `cargo check -p veloren-voxygen` green with the arena changes;
     vanilla path untouched (arena is env-var-gated: no `BASTION_ASSET_ARENA` =
     zero behavior change; no vanilla asset-tree files added or edited).
2. **≥3 REAL assets load + pass arena dynamic tests, results logged.**
   Far exceeded: **effective 69/69 world-layer catalog graduated** across the
   full sweep + reruns (61/64 initial v2 sweep; 3 gate fails were ONE pilot
   sidecar typo — `DoorBars(())` → `Sprite(DoorBars())` — fixed same-day and
   rerun clean; 5 harbor batch-4 pieces passed on arrival).
   - The spec's named trio: cottage (structure w/ interior) **7/7** — marker
     fidelity, reach-interior 4.2–4.5s, egress, 3-colonist multi-occupancy
     in/out, integrated-dynamic on natural slope-3 terrain (reach 2.8–3.4s +
     egress); palisade wall+gate — closed BLOCKS (watchdog, best dist ~11.4)
     / open ADMITS (~2.7s) / open egress; prop assertion at world scale via
     flora (rowan + 3 more: path-around/back).
   - Work-marker casts (7 workshops + depot): colonists reach the AUTHORED
     crafting cell (exact-cell fidelity: authored coordinates == placed voxels).
   - Gate matrices ×3 (palisade/brick/dwarven): closed-blocks / open-admits /
     open-egress all PASS.
3. **One asset demonstrably FAILS usefully.**
   `test_room_door_closed` → reach-interior FAIL "STUCK (watchdog) after 13.5s,
   best dist 4.5" with exit code 1; control twin `test_room_door_open` → 5/5
   PASS. Additionally the marker-fidelity gate produced 8 real catches in the
   first sweep (undeclared bytes + the byte-8→Fruit carve drift) — all led to
   pilot-side fixes + the ASSET_MARKER_REGISTRY.md authority being created.
4. **Vanilla regression (standing invariants).**
   On the merged-forward branch (post B-MAP1 + B5.6b-2), in the worktree:
   - Unit tests: `cargo test -p veloren-common --lib bastion` → **9 passed, 0
     failed** (incl. `bastion_dot_vox_index_convention`, the byte-mapping pin).
   - `--b4-scenario` **PASS** · `--b5-scenario` **PASS** · `--b55-scenario`
     **PASS** (single-sample preliminary run under background-compile load —
     i.e. the harder condition; the quiet-window formal rerun is available on
     request at the merge slot).
   - `veloren-server-cli --non-interactive` flagless boot smoke (worktree
     build, `VELOREN_USERDATA` → temp dir): **PASS** — "Server is ready to
     accept connections" (web 14005 / game 14004), ZERO panics, ran until the
     150 s timeout kill (rc 124 = the timeout, by design). No BASTION_* env
     set → the arena code path provably inert on a vanilla boot.
5. **--asset-arena boots; Ben orbits + fixture paths in.**
   Code complete + compile-green; the EXE BOOT + eyeball is deliberately NOT
   run from this worktree (headless-only directive). Scheduled for the
   architect's `.claude/worktrees/test` lane at the merge window: build voxygen
   there, Ben runs `veloren-voxygen --asset-arena [id]`, orbits, uses
   `/bastion_arena next|prev|fixture|dismiss`.
6. **Integrated-dynamic spot check on real terrain.** Cottage placed on
   natural (unflattened) worldgen terrain, slope 3 across the footprint:
   reach + egress PASS (logged in the integration log, both sweep runs).

## Notable defects found + fixed during self-test (the gate earning its keep)

- Fixed-height pad clearing left natural cliff walls INSIDE the arena pad on
  sloped anchors → sim-probed `pick_flat_anchor` + real-terrain `survey_pad`
  adaptive clearing.
- Cross-country fixture staging (STUCK 337 blocks out) → teleport staging.
- Idle fixtures WANDER between test poses (rtsim brain) — one got walled into
  the defense yard during the open-pose rebuild → teleport-stage each pose.
- Harness self-bug: misleading placeholder outcomes on failed restaging →
  honest per-stage assertions.

## Deferred (documented in backlog + findings §10)

Figure-scale prop casts (11 vox/block — sprite-manifest rung); ladder "climb"
cast (post-B5.8-merge); pose matrices (DF-MECH); runtime SpriteCfg writes;
placement rotation; 50-instance perf + save/load soak (B-TESTBED's lane).
