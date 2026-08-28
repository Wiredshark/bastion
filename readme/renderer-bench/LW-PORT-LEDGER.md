# LW enhancement-ladder port ledger (2026-08-21)

Source: `bastion/renderer-lw-integration-v1` @ 6d96ea24 — the accepted
Codex renderer enhancement program (DECISIONS-FOR-BEN row 81; tag
`renderer-accepted-package-97efd2f6`). Target: this fork.

## What landed (49 commits cherry-picked in ladder order)
- **r0d** (12): the certified renderer substrate — `bastion-renderer-r0d`
  crate (admission, canonical protocol/cbor, identity, extraction, camera
  readiness/selection, capture, tape, atlas, pass_graph, visual_oracle,
  shutdown, replay …) + `voxygen/src/render/bastion_r0d.rs` integration +
  certification fixture + frozen-tick capture. Env-gated `BASTION_R0D_*`,
  fail-closed.
- **r0p** (1): bounded production renderer observatory
  (`voxygen/src/r0p_observer.rs`).
- **r1a–r1d** (7): immutable presentation handoff, modular figure
  packages, figure GPU resource receipts, figure draw batching,
  individual rendering tiers, group representations, population scale
  transitions (`voxygen/src/r1d_scale.rs`).
- **r1e** (6): terrain cutaway foundation + capture binding + stage
  anchor + frozen-tick settling, interior visibility snapshot, render
  islands.
- **r1f–r1g** (11): material schema, authoritative environment
  projection, coherent weather presentation + fixture acknowledgment
  (`server/src/bastion_weather_fixture.rs`), fog projection, lighting
  policy + capture metadata, shadow importance tiers, weather world lens
  + minimap badge.
- **r2** (4): canonical GPU culling parity path, same-frame CPU/GPU cull
  parity, deterministic indirect draw submission, R2-accelerator
  admission.
- **post-r2** (8): far-terrain residency band, durable R0P frame
  measurements, continuous streaming-horizon diagnostics, horizon
  fixture, multicamera horizon + post-apply authority.

## Drift reconciliation (the month between bases, resolved by hand)
- `bastion_jobs.rs` / `world/site/mod.rs`: both sides carried the SAME
  determinism fixes (lw's July DETRNG vs mainline's landed DET-RNG-007/8
  audit versions) — took mainline's canonical versions.
- `server/src/lib.rs`: mainline's bastion crate-split re-exports kept;
  lw's one real payload (`bastion_weather_fixture` module) added beside
  them.
- `server/src/weather/tick.rs`: mainline's T0.87 adoption epoch and lw's
  zone-generation acknowledgment are ORTHOGONAL counters — merged both.
- `bastion_flat_arena.rs`: file moved crates on mainline; lw's
  `within_flat_override_radius` helper + boundary test transplanted to
  the new home.
- `client/src/lib.rs`: union (prediction-buffer consts + streaming-
  horizon distance shape).
- Cargo.lock: ours + regenerate.

## Verification on the ported tree
- `cargo check` clean: voxygen, server, client, bastion-server,
  bastion-renderer-r0d, bastion-harness.
- **bastion-renderer-r0d test suite: 292/292 PASS** (Codex's own
  certification tests, first run on the new base).
- Bench referee suites unperturbed: W1+W3 vectors 15/15, common-net
  138+2.
- **Three-leg bench smoke GREEN on the ladder tree** (binary ab361eba,
  fixture 3bbdff4a): twins run_root identical; client-leg run_root
  identical; acks 20/20 echo-matched, resolved [0,2,2,…,2] — the
  ladder's presence does not perturb the semantic tape (dormant-unless-
  armed, proven live, not claimed).

## Owed next (from row 81's own exclusions + the program's endpoint)
- The unconditional per-frame visible-horizon census on the default path
  (budget-violating) — gate it.
- Invalid streaming-measurement env value kills the embedded server —
  make it fail loud, not fatal.
- The horizon RETEST under the item-19 fixture fix (arena radius ≥
  tested VD): the reason the far band stayed UNPROVEN.
- Live-arming legs for the r0d/r0p/r1 instruments themselves
  (BASTION_R0D_* smoke), then the architecture-selection memo on
  measured evidence.
