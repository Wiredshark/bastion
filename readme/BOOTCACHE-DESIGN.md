# Bastion headless world boot cache

Status: **prototype rejected by the mandatory byte gate** on 2026-07-19;
review-only handoff on `codex/boot-cache`, not shippable or merge-ready. See
`BOOTCACHE-EVIDENCE.md`. This is not the rejected
`FileOpts::LoadOrGenerate` map cache.

## Problem and boundary

The headless harness already loads Veloren's bundled default height map when
`Settings::map_file` is `None`. The remaining cold-boot cost is the generated
world/civilization graph, LoD/map views, RTSim generation/setup, and the
scenario's synchronous force-loaded chunks. Persisting only a map file changes
the input world and does not cache those products.

`Server` itself is not a safe serialization unit. It owns a Tokio runtime,
network endpoints, metrics, dispatchers, channels, asset reload handles, a
pointer-rich Specs ECS, and wall-clock `Instant`s. Byte-copying or partially
serializing it would restore invalid process resources and would make stale
state difficult to detect.

The first safe cache is therefore **process-resident and opt-in**. It speeds
several fresh server/scenario runs made by one harness process (the intended
paired/corpus workflow). It is deliberately not a cross-process or on-disk
cache. A process exit destroys it, so no stale native object graph can survive
a rebuild.

## 1. Snapshot contents

The cache stores the expensive deterministic inputs from which an equivalent
post-boot server is rebuilt:

- `Arc<world::World>` after world simulation, civilization generation,
  economy simulation, and spot generation;
- `world::IndexOwned` for that exact world;
- the derived `WorldMapMsg` and server `Lod`;
- a pristine clone of generated/loaded `rtsim::Data` **before** `RtState`
  preparation, rule startup, and `OnSetup` mutate the run-local copy;
- pristine force-loaded terrain chunks, indexed by chunk position, together
  with the RTSim maximum-resource supplement consumed by
  `RtSim::hook_load_chunk`.

The following state is intentionally reconstructed for every restore:

- the Specs ECS, component registrations, event buses, terrain grid, trackers,
  metrics, schedulers and channels;
- `Tick(0)`, simulation `Time(0)`, configured `TimeOfDay`, and run-local
  `TickStart(Instant::now())`;
- Tokio/network/chat/database handles and dispatchers;
- `RtState`, its rule/resource maps, `ChunkStates`, and `OnSetup`, all rebuilt
  from a private clone of the pristine cached `rtsim::Data`;
- scenario entities, jobs, claims, terrain edits, and controller state. None of
  these are cached.

This is a semantic snapshot, not a raw-memory snapshot. The acceptance gate
compares a canonical post-boot/ECS observation and authoritative trajectories;
pointer addresses, wall-clock instants, metrics handles, and channel identities
are neither gameplay state nor compared.

The `World` graph is shared only because server code exposes it immutably after
generation (`Arc<World>` is already the production ECS resource). A restored
server receives its own RTSim data, ECS, terrain grid, and mutable scenario
state. Cached terrain chunks are inserted as immutable `Arc`s; normal terrain
copy-on-write leaves the cached pristine chunk untouched.

## 2. Exact key and restore protocol

Caching is disabled unless the harness explicitly enables it. The cache key is
the canonical serialization of:

- cache schema version;
- harness executable SHA-256 (the code-version authority);
- target architecture and operating system;
- world seed;
- world settings, map option and calendar mode;
- deterministic execution/worldgen mode;
- all boot-affecting `BASTION_*`, `RTSIM_*`, and `VELOREN_RTSIM` environment
  inputs, excluding only output-path/provenance variables that cannot affect
  simulation.

The implementation supports only the deterministic headless recipe:
`CalendarMode::None`, `map_file: None`, deterministic serial RTSim/worldgen,
no terrain persistence, and no pre-existing `rtsim/data.dat`. Anything else is
a classified cache refusal and follows the full fresh path.

Lookup compares the complete canonical key, never a user-supplied short name.
The process holds at most one world template: a different exact key is a miss
and replaces it only after successful generation. Chunk entries live under the
same key. There is no permissive version fallback and no partial hit.

On a miss:

1. run the existing production generation path;
2. clone pristine pre-setup RTSim data;
3. finish normal server construction;
4. publish the template only after construction succeeds;
5. cache each pristine force-loaded chunk before inserting it into mutable
   scenario terrain.

On a hit:

1. clone the exact-key immutable world views and pristine RTSim data;
2. run normal ECS/runtime construction and normal RTSim preparation/rules/
   `OnSetup` from the clone;
3. satisfy force-load requests from exact-key pristine chunks when available;
4. otherwise generate normally and extend the in-process template.

Any missing field, unsupported configuration, poisoned lock, or key mismatch
falls back to a full boot. A hit is reported explicitly; absence of a hit can
never masquerade as a restored run.

## 3. Determinism strategy

- **RNG:** cached `World` includes the post-generation world RNG state. It is
  immutable during server simulation. RTSim's mutable state is cloned from the
  pre-setup snapshot; deterministic rules derive streams from world seed and
  tick as before. Chunk cache entries preserve the exact generated blocks and
  resource supplement.
- **Ordering:** no state is reconstructed by iterating a newly randomized
  `HashMap`. The generated graph and its stores are reused intact. Each RTSim
  run clones the same ordered serialized data structures and executes the same
  setup path. Canonical evidence sorts entity/colonist observations by stable
  identity before encoding.
- **Tick/time:** every server is reconstructed at tick zero and simulation time
  zero with the configured world start time. Wall `Instant`s are run-local and
  excluded. Evidence may normalize only `wall_unix_millis` under the existing
  determinism rule.
- **Mutation isolation:** RTSim `Data`, ECS resources, entities, controllers,
  terrain grids, jobs and claims are per-run. Cached chunks are captured before
  the scenario can edit them.
- **No hidden success:** the proof requires leg A to report `fresh` and leg B
  to report `restored` for the same full key. Two fresh boots are not a passing
  cache test.

## 4. Scope and fallback

The opt-in surface is the harness `--boot-cache` flag and the
`--boot-cache-proof <new-directory>` runner. Ordinary harness,
server-cli, singleplayer and live multiplayer construction remain fresh by
default. `--boot-cache` is useful only for a command path that constructs more
than one `Server` inside the same long-lived harness process; it does not make a
single-server scenario faster. The proof runner deliberately performs those
repeated constructions. This implementation does not accelerate separate
command invocations and is not advertised as durable storage.

Unsupported configurations, existing RTSim persistence, a changed binary,
changed seed/settings/environment, or a different process always run the
production full-boot path. A future durable cache would require stable,
versioned serialization for the generated `World/Civs/Index` graph and a
separate review; this change does not invent one.

## 5. Risks and acceptance gate

Risks:

- an overlooked boot-affecting environment input could make an overly broad
  key;
- interior mutation of the supposedly immutable world/index graph would leak
  between runs;
- RTSim setup could depend on process-global state not represented by its data;
- terrain mutation could modify a cached chunk instead of copy-on-write;
- process-global asset reloads could change data during one cache session;
- a cache hit may save world/RTSim generation but not every scenario-specific
  setup cost.

Mandatory acceptance, on the same architecture and frozen executable:

1. default-disabled test proves no cache lookup/publication;
2. exact-key hit, mismatched seed/key/settings, existing RTSim input and
   unsupported-calendar tests prove fail-closed fallback;
3. fresh leg versus restored leg for one seed reports different cache origins
   but byte-identical canonical outcome and per-tick colonist trajectory tapes;
   the outcome includes spawn, the full persistence-format RTSim-data digest,
   and a canonical loaded-terrain block digest;
4. the pair is repeated (`x2`) from a cleared process cache;
5. a bounded multi-seed corpus repeats fresh/restored equivalence;
6. pristine cached block/resource digest before and after a deliberate
   post-capture scenario terrain mutation is equal (copy-on-write proof);
7. restored wall boot time is recorded but never substituted for the byte gate.

Only `wall_unix_millis` may be normalized. Any behavioral or trajectory
divergence, missing record, accidental fresh second leg, cache refusal, panic,
nonzero child/result, or key mismatch fails the gate. If this evidence cannot
be produced, the cache remains unshipped and the exact failing seam is handed
to the architect.

Rollback is deletion of the harness opt-in/repeat/proof surface, the isolated
boot-template module and the narrow `RtSim` construction split. No saved game,
normal server setting, worldgen algorithm, scenario behavior, or gameplay gate
is changed.

## Harness usage

The mandatory equivalence gate is one process so its second leg can consume
the first leg's native template:

```text
bastion-harness.exe --boot-cache-proof <new-evidence-directory> \
  --boot-cache-proof-seeds 21,22 --boot-cache-proof-ticks 90
```

The first seed runs fresh/restored twice; each additional seed runs one corpus
pair. The command refuses to overwrite evidence and exits nonzero unless every
pair reports `fresh` then `restored`, nonempty tapes, identical outcomes and
trajectories, and a clean chunk copy-on-write probe.

`--boot-cache` enables the same process-local template for other long-lived
harness paths that construct multiple servers. It is intentionally inert as a
speed optimization for a one-server command and cannot share state with child
or later processes.
