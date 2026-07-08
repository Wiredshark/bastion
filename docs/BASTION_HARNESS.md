# bastion-harness — the headless simulation harness (B0)

`bastion-harness` boots the **real** Veloren simulation stack — `world` + `rtsim` + `server` —
with no voxygen, no GPU, and no network clients, ticks a seeded world a fixed number of times
faster than real-time, and dumps aggregate state as JSON. It is where ~90% of all colony-sim
testing happens from B3 onwards, including the Tier-1b zero-input autonomy soak (design doc §7).

It reuses the exact construction path of `server-cli`/singleplayer (`server::Server::new` →
`server.tick(...)` loop); the verified APIs are recorded in `BASTION_B0_FINDINGS.md`.

## Running

```powershell
cargo run -p bastion-harness                                # seed 1337, 1000 ticks
cargo run -p bastion-harness -- --seed 42 --ticks 3000      # explicit
cargo run -p bastion-harness -- --verify                    # determinism self-check
RUST_LOG=warn cargo run -p bastion-harness -- --verify      # quieter (env var syntax per shell)
```

Logs go to **stderr**; **stdout** carries exactly one JSON line (the `Summary`), so you can pipe
it: `cargo run -p bastion-harness | jq .rtsim_npc_count`.

## Flags

| Flag | Default | Meaning |
|---|---|---|
| `--seed <u32>` | `1337` | `server::Settings::world_seed`. Seeds rtsim data generation (`Data::generate` uses `index.seed`) and civ/site generation. Terrain itself comes from the shipped default map asset (see below). |
| `--ticks <u64>` | `1000` | Server ticks to run. **One server tick == one rtsim tick** (the `rtsim::tick` ECS system runs every dispatch), so this is also the rtsim tick count. |
| `--tps <f64>` | `30.0` | Sim ticks-per-second used to derive the fixed `dt` passed to `Server::tick`. The harness never sleeps — 1000 ticks of 33 sim-seconds complete in a few wall-seconds. |
| `--verify` | off | Run the same configuration **twice in isolated child processes**, diff the dumps field-by-field, print `DETERMINISM: OK` (exit 0) or `DETERMINISM: DIVERGED` + diff (exit 1). Child failure → exit 2. |
| `--data-dir <path>` | fresh temp dir | Server data dir. Leave defaulted for reproducibility: rtsim **loads** `<data_dir>/rtsim/data.dat` if present instead of generating from seed. A persistent dir is only useful for testing persistence itself (B10). |

## Output (`Summary`)

```json
{"seed":1337,"tick_count":1000,"rtsim_tick":1000,"rtsim_npc_count":…,"rtsim_site_count":…,
 "rtsim_faction_count":…,"rtsim_report_count":…,"loaded_entity_count":…,"sim_time":…,"time_of_day":…}
```

- `rtsim_*_count` — sizes of rtsim's flat tables (`data.npcs/sites/factions/reports`). These are
  the world-scale "is the world alive and stable" numbers.
- `loaded_entity_count` — entities in the server ECS. With no clients no chunks load, so this
  stays near zero in B0; from B3 on it's the promote/demote-leak canary (design doc §8.4).
- `sim_time` / `time_of_day` — pure functions of `ticks × dt`; they double-check that time
  advanced exactly as configured.
- Deliberately **aggregates only**: exact NPC positions/decisions are not reproducible today
  (rtsim rules seed RNG from OS entropy — findings §4), and aggregates are what invariants care
  about anyway.

Every field must stay a pure function of the simulation — no wall-clock, no absolute paths —
so `--verify` can compare dumps with plain equality. Wall-clock timings are logged to stderr only.

## What it does under the hood

1. Fresh temp data dir (rtsim persistence isolation).
2. `server::Settings` built in code: **no** TCP/QUIC listeners, **no** auth, **no** UDP query
   server, `CalendarMode::None` (no wall-clock calendar), default map asset, `world_seed` from
   `--seed`.
3. `Server::new(...)` — full worldgen (map asset load + civ/site gen), ECS + dispatcher setup,
   rtsim init/generation. Takes the bulk of the runtime (O(minutes) cold, cached assets help).
4. `--ticks` × (`server.tick(Input::default(), dt)` + `server.cleanup()`), no pacing.
5. Read aggregates from the `server::rtsim::RtSim` ECS resource
   (`rtsim.state().data()`) and the ECS, print JSON, tear down.

## How future blocks add assertions

Keep the harness a **dump tool**; put assertions in the caller (a test, a script, or a later
`--assert` mode), so one binary serves every block:

- **B3+ Tier-1 tests**: run the harness (or link `run_once` as a lib fn — split
  `main.rs` into `lib.rs` + `main.rs` when first needed) with a fixed seed/ticks, deserialize
  `Summary`, assert invariants (`colonist_count == N`, `loaded_entity_count` returns to baseline
  after promote/demote cycles, item-conservation counters once they exist as aggregates).
- **New aggregates**: add a field to `Summary` (it is `serde` + `PartialEq`; `--verify` and the
  diff printer pick it up automatically). Extend, don't repurpose, existing fields — soak-test
  baselines depend on their meaning staying fixed.
- **Tier-1b soak (Slice C+)**: `--ticks` scaled to in-game days (`day_length` default 30 min ⇒
  1 day = 54 000 ticks at 30 TPS), plus periodic mid-run `Summary` snapshots (add a
  `--snapshot-every N` flag when needed) to assert "stable *and* eventful" over time.
- **Loaded-world testing**: `server.create_centered_persister(view_distance)` force-loads chunks
  around a point with no client — the hook for testing loaded-ECS colonists headlessly.

## Determinism status (measured)

See `BASTION.md` for the current verdict of the standard check (seed 1337, 1000 ticks).
Known structural limitation: rtsim per-tick rules (`npc_ai`, `migrate`, `cleanup`) seed their
RNGs from OS entropy rather than the world seed, so exact trajectories differ run-to-run by
design; initial world/rtsim *generation* is seed-derived. Plumbing a seeded RNG through the rule
state is a small patch, deferred to the first block that needs strict determinism (see findings
§4 and design doc §8.10).
