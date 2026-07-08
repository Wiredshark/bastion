# Project Bastion

Transforming this fork of [Veloren](https://gitlab.com/veloren/veloren) into an **autonomous
god-game colony sim** (Dwarf Fortress / RimWorld autonomy + Black & White / From Dust indirect
divinity — explicitly *not* a StarCraft-style RTS).

- **Design doc (architect artifact):** `readme/veloren-colony-rts-build-report.md`
- **Design pillar:** the world plays itself; the player influences, never commands. If a feature
  request is "tell a specific unit to do a specific thing right now," it must become policy,
  designation, or god-power instead (doc §8.13 — guard this rule).

## State: B0 complete

| | |
|---|---|
| Baseline | upstream `master` @ `bfef92fcb33e7e610ba24fecd5920d0c0e227221` (2026-07-07), local tag `bastion-baseline` — see `BASELINE.md` |
| Branch | `bastion/main` |
| Build | `cargo build --bin veloren-voxygen --bin veloren-server-cli` (toolchain notes in `BASELINE.md`) |
| Run vanilla | `target\debug\veloren-voxygen.exe` (singleplayer) / `target\debug\veloren-server-cli.exe --non-interactive` |
| Headless harness | `cargo run -p bastion-harness -- --seed 1337 --ticks 1000 [--verify]` — see `docs/BASTION_HARNESS.md` |
| Fast inner loop | `scripts\bastion-check.ps1` or `cargo bastion-check` |
| Findings / real APIs | `docs/BASTION_B0_FINDINGS.md` |

## Determinism status

<!-- BASTION-DETERMINISM-STATUS: updated by hand after each harness-relevant change -->
**Aggregate determinism: OK** (measured 2026-07-08, `bastion-harness --seed 1337 --ticks 1000
--verify`, two isolated child processes):

```
run 1: {"seed":1337,"tick_count":1000,"rtsim_tick":1000,"rtsim_npc_count":2355,"rtsim_site_count":204,"rtsim_faction_count":16,"rtsim_report_count":0,"loaded_entity_count":0,"sim_time":33.333333000000785,"time_of_day":33999.999983998714}
run 2: identical
DETERMINISM: OK          (1000 ticks in ~3.8 s, ≈8.7× real-time, no GPU)
```

Scope of the claim: **aggregates only** (counts + sim clocks). Exact NPC trajectories are *not*
reproducible — rtsim's per-tick rules seed RNG from OS entropy (`npc_ai`/`migrate`/`cleanup`; see
`docs/BASTION_B0_FINDINGS.md` §4 and design doc §7 "Determinism reality" + work item WI-DET).
Re-measure after any change that touches rtsim rules or adds sim systems.

## Conventions

- Everything Bastion adds is **namespaced** (`bastion-*` crates/scripts, `bastion::`/`bastion_`
  modules) and **additive** — vanilla voxygen and server-cli must keep building and running
  unchanged. This keeps future upstream merges tractable (see the history-graft note in
  `BASELINE.md`).
- One block per session; a block is done only when its Done-when tests are green (design doc §6).
- Simulation logic is tested headlessly through `bastion-harness`, not by launching voxygen.
