# Bastion B0 — Findings (real paths & APIs)

Recorded against the baseline snapshot (see `BASELINE.md`). These are the *verified* construction,
tick, and state-accessor paths that `bastion-harness` and all later blocks build on. File paths are
relative to the repo root; line numbers are approximate and will drift.

## 0. Repo/environment reality (differs from the plan's assumptions)

1. **The checkout was not a git repository.** `E:\veloren-master` is a GitLab "download source"
   snapshot of `master` (no `.git`). Consequences:
   - The upstream SHA was recovered after the fact and content-verified as
     `bfef92fcb33e7e610ba24fecd5920d0c0e227221` (master, 2026-07-07T08:20:06Z) — see `BASELINE.md`
     for the verification method. The *local* baseline commit (`git init` + commit, tag
     `bastion-baseline`) shares no history with upstream; re-grafting onto the real upstream commit
     is recommended before the first upstream merge.
   - `common/build.rs` **requires** `git log` to succeed (it bakes `VELOREN_GIT_VERSION` =
     `<tag?>/<hash>/<timestamp>` into `veloren-common` at compile time) — so `git init` + an initial
     commit is a *build prerequisite*, not just hygiene. Alternative escape hatch: set env
     `VELOREN_GIT_VERSION=/0/0`.
   - `common/assets/src/lib.rs::find_root()` locates the `assets/` dir by walking up from CWD
     looking for **`.git`** — another reason git init had to happen first. `VELOREN_ASSETS` env var
     overrides (checked first), which the harness docs recommend for robustness.
2. **Assets are real binaries, not LFS pointer stubs** (verified `.vox` magic bytes). No
   `git lfs pull` needed for this snapshot. `.gitattributes` declares LFS patterns, so a future push
   to a fork with LFS enabled will store them as LFS objects.
3. Toolchain: repo pins `nightly-2026-06-13` via `rust-toolchain`; it is installed as
   `x86_64-pc-windows-gnu`. `rustup`/`cargo` live in `%USERPROFILE%\.cargo\bin` (not on PATH by
   default in fresh shells).

## 1. Server construction (the recipe the harness reuses)

Reference implementations, in order of usefulness:
- `server-cli/src/main.rs` — the canonical headless boot (`main()` → `Server::new` → `server_loop`).
- `voxygen/src/singleplayer/mod.rs:82-170` — the minimal in-process recipe: build `Settings` in
  code, `Server::new` on a plain thread, tick loop. Proof that no network client is required.

**Entry point:** `server::Server::new(settings, editable_settings, database_settings, data_dir,
report_stage, runtime) -> Result<Server, Error>` at `server/src/lib.rs:259`.

Arguments and what the harness passes:

| Arg | Type | Harness value |
|---|---|---|
| `settings` | `server::Settings` (`server/src/settings/mod.rs:179`) | Constructed in code, **not** loaded from disk (see below) |
| `editable_settings` | `server::EditableSettings` | `EditableSettings::singleplayer(&data_dir)` (no admins/whitelist files) |
| `database_settings` | `server::persistence::DatabaseSettings` | `{ db_dir: <fresh temp>/saves, sql_log_mode: SqlLogMode::Disabled }` |
| `data_dir` | `&Path` | **fresh temp dir per run** (critical: rtsim persistence, see §3) |
| `report_stage` | `&dyn Fn(ServerInitStage)` | logging closure |
| `runtime` | `Arc<tokio::runtime::Runtime>` | small multi-thread runtime (server-cli uses `max(cpus/4, MIN_RECOMMENDED_TOKIO_THREADS)`) |

**Headless-relevant `Settings` fields** (all of `Settings` is `pub`, `Default` exists):
- `gameserver_protocols: vec![]` → **no TCP/QUIC sockets bound at all** (loop at
  `server/src/lib.rs:561`). The only unconditional listener is in-process MPSC:
  `network.listen(ListenAddr::Mpsc(14004))` (`server/src/lib.rs:665`) — no real networking.
- `auth_server_address: None` → auth disabled.
- `query_address: None` → no UDP query server task.
- `world_seed: u32` → **this is the world seed plumbing.** Used at `server/src/lib.rs:307`
  (`World::generate(settings.world_seed, WorldOpts { … })`).
- `map_file: Option<world::sim::FileOpts>` → `None` loads the shipped default map asset
  (`FileOpts::LoadAsset(DEFAULT_WORLD_MAP)`, seed then largely irrelevant to terrain). For a
  *seeded* world the harness passes `FileOpts::Generate(GenOpts { x_lg, y_lg, scale, map_kind,
  erosion_quality })` (`world/src/sim/mod.rs:147-201`); small maps (e.g. `x_lg=y_lg=8` = 256×256
  chunks vs default 1024×1024) keep generation time tolerable.
- `calendar_mode: CalendarMode::None` → avoids wall-clock-dependent `Calendar::from_tz(now)`
  (`server/src/settings/mod.rs:157-175`) — one less nondeterminism source.
- `world: common::rtsim::WorldSettings { start_time }` (`common/src/rtsim.rs:514`) → initial
  `TimeOfDay` for both server (`server/src/lib.rs:547`) and rtsim data generation.
- `day_length` → sim-seconds↔game-time ratio via `ServerConstants::day_cycle_coefficient`.

**Inside `Server::new`** (order matters, `server/src/lib.rs:259-710`):
1. DB migrations + vacuum (rusqlite, in `db_dir`).
2. `World::generate(seed, WorldOpts, &pools, report)` (worldgen feature) — terrain sim + civs/sites.
3. `State::server(...)` builds the ECS with dispatchers: `add_local_systems`,
   `sys::add_server_systems`, and (worldgen) **`rtsim::add_server_systems`** + weather.
4. ~90 ECS resources inserted (settings, metrics, `SpawnPoint`, `Arc<World>`, `IndexOwned`, …).
5. Network listeners (none for us) + MPSC.
6. **rtsim init** (`server/src/lib.rs:669-688`): `rtsim::RtSim::new(&settings.world,
   index.as_index_ref(), &world, data_dir.to_owned())` then inserted as ECS resource, plus
   `weather::init`.

## 2. Ticking

`Server::tick(&mut self, Input::default(), dt: Duration)` at `server/src/lib.rs:783`, followed by
`Server::cleanup()` each iteration (see `server-cli/src/main.rs:304-336` loop).

- `server::Input` is an empty struct (`server/src/input.rs`).
- `dt` is wall-clock frame time in vanilla (`Clock::game_dt`, 1/30 s target, `server-cli` `TPS=30`).
  **The harness passes a fixed `dt = 1/30 s` and never sleeps** — faster than real-time and
  reproducible; nothing in the tick path requires wall-clock pacing. (Some metrics use
  `Instant::now()` but don't feed gameplay.)
- Sim time advances: `Time += dt` and `TimeOfDay += dt * day_cycle_coefficient` inside
  `State::tick` (`common/state/src/state.rs`).
- **rtsim ticks once per server tick**: ECS system `rtsim::tick::Sys` (`server/src/rtsim/tick.rs:447`,
  registered at `server/src/rtsim/mod.rs:381-383`) calls `RtState::tick(...)` →
  `rtsim/src/lib.rs:314`, which increments `data.tick` and emits `OnTick` to all rules.
  So **N server ticks = N rtsim ticks**; "1000 rtsim ticks" = 1000 `server.tick()` calls ≈ 33 s of
  sim time at dt=1/30 s.
- With zero clients, no chunks are ever loaded → all rtsim NPCs stay
  `SimulationMode::Simulated`, no wildlife/chunk systems run, the loaded ECS stays near-empty.
  (`server.create_centered_persister(vd)` exists to force-load chunks around a point if a later
  block wants loaded-entity testing.)
- rtsim autosaves every 60 s wall time inside its tick system (`server/src/rtsim/tick.rs:553-561`)
  to `<data_dir>/rtsim/data.dat` — harmless with a temp dir.

## 3. rtsim state access (the aggregate dump)

- Resource: `server.state().ecs().read_resource::<server::rtsim::RtSim>()`
  (`server/src/rtsim/mod.rs:33`; module is `pub` in `server/src/lib.rs:28`).
- `RtSim::state() -> &RtState` → `RtState::data() -> impl Deref<Target = Data>`
  (`rtsim/src/lib.rs`).
- `rtsim::data::Data` (`rtsim/src/data/mod.rs:40`):
  - `data.npcs.npcs: DenseSlotMap<NpcId, Npc>` → `.len()` = **rtsim_npc_count**; per-NPC
    `npc.mode: SimulationMode` distinguishes loaded/simulated.
  - `data.sites.sites: DenseSlotMap<SiteId, Site>` → `.len()` = **rtsim_site_count**.
  - `data.factions`, `data.reports`, `data.quests`, `data.airship_sim` — more aggregates.
  - `data.tick: u64` — rtsim's own tick counter.
  - `data.time_of_day: TimeOfDay`.
- Loaded ECS entity count: `state.ecs().entities().join().count()`.
- Sim time: `state.ecs().read_resource::<common::resources::Time>().0`.
- **Persistence gotcha:** `RtSim::new` *loads* `<data_dir>/rtsim/data.dat` if present and only
  generates fresh data otherwise (`server/src/rtsim/mod.rs:50-121`). For reproducible runs the
  harness must use a fresh `data_dir` per run (env escape hatches exist: `RTSIM_NOLOAD=1`,
  `VELOREN_RTSIM=<path>`).
- rtsim data generation is seeded from the world seed: `Data::generate` builds
  `SmallRng::from_seed(index.seed)` (`rtsim/src/generate/mod.rs:85-90`) — same seed ⇒ same initial
  NPC/site/faction population.

## 4. Determinism — early risk register (source-level, pre-measurement)

Verified in source; the harness `--verify` mode measures the practical impact:

- **rtsim per-tick rules seed RNG from OS entropy, not the world seed**:
  - `rtsim/src/rule/npc_ai/mod.rs:179` — NPC brain RNG: `ChaChaRng::from_seed(rand::rng().random())`
  - `rtsim/src/rule/migrate.rs:26` — same pattern
  - `rtsim/src/rule/cleanup.rs:21` — same pattern
  So **exact NPC trajectories/decisions are NOT reproducible across runs** out of the box.
  Aggregate counts may still be stable over short horizons (births/deaths are rare in 1000 ticks).
  Fixing this properly (plumbing a seeded RNG into rule state) is a small, upstream-mergeable
  patch — flagged as a candidate for the block that first needs strict determinism (B3/§7 Tier 1).
- Worldgen/erosion runs on rayon thread pools; Veloren ships pre-generated maps as assets, and map
  generation from a fixed seed is designed to be reproducible, but we treat "same seed ⇒ same map"
  as *measured-true* only after `--verify` passes with a generated map.
- Wall-clock leaks into: `CalendarMode::Auto` (disable via `None`), rtsim autosave cadence
  (I/O only), metrics (`Instant::now`, no gameplay effect).
- `specs` ECS iteration order and float accumulation: dormant risk while the loaded world is empty;
  becomes real in B3+ when colonists live in the ECS (design doc §8.10).

## 5. Misc facts later blocks will want

- `server-cli` features: default = `["worldgen", "persistent_world", "plugins", "simd"]`
  (`veloren-server` `default` likewise). The harness depends on `veloren-server` with default
  features (worldgen is required for rtsim systems to be registered at all).
- `DEFAULT_WORLD_SEED` and `DEFAULT_WORLD_MAP` come from `world::sim`
  (re-exported/used at `server/src/lib.rs:147`); default map asset is `world.map.*` under
  `assets/world/map/` (LFS-declared `.bin`).
- The vanilla server userdata root is `common_base::userdata_dir()` (env
  `VELOREN_USERDATA`/`VELOREN_USERDATA_STRATEGY` sensitive) — the harness bypasses all of it by
  passing explicit paths.
- `Server::new` + `tick` never require a frontend, a client connection, or a GPU. The
  `client` crate is *not* needed for B0 (it would be for driving inputs in later blocks — or use
  `server`'s event busses directly).
- Windows note: `server-cli`'s `#[global_allocator] mimalloc` is gated on
  `target_os = "windows"` + not hot-reload features; the harness does not replicate this (default
  allocator is fine for a test tool).
