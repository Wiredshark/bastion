# B4 findings — designation → job board → autonomous arbitration + pathing

Spec: design doc §B4 (no prompt file in-tree). Built 2026-07-09 on
`bastion/block-B4` (start `4724318`).

## 1. Movement: the rtsim intent interface, exactly as the doc suggested

`common::rtsim::NpcActivity::Goto(pos, speed)` is executed by loaded agents
with real traversal (`server/agent/src/action_nodes.rs:269` — the same code
rtsim NPCs walk with). The job system writes it into
`Agent::rtsim_controller.activity` each tick for traveling colonists.

**The clobber gate**: `server/src/rtsim/tick.rs`'s sync loop copies
`npc.controller.activity` into the agent every tick for rtsim entities — while
a colonist has `comp::bastion::ActiveJob`, that copy is skipped (the job
system owns the activity; personality/look/actions still sync). Without a job
the vanilla brain resumes untouched.

## 2. The board

`server/src/bastion_jobs.rs`: `JobBoard` resource (jobs: HashMap<JobId, Job>;
`common::bastion::Job` is serde-ready). v1 generation: Mine = every filled
block in the painted region, Chop = every `BlockKind::Wood` block;
Build/Stockpile map to work types now but generate no jobs until B5/B6.
`Sys` runs every tick (travel upkeep + watchdog) and arbitrates every 15
ticks: per idle colonist, highest `WorkPriorities` priority → nearest →
claim; claims marked on the board during selection = atomic within a pass.

## 3. Reachability = travel watchdog, progress-based

First attempt used displacement ("hasn't moved 0.5 blocks in 10s") — an agent
pacing around an unreachable underground target moves constantly and never
trips it. The shipped watchdog tracks **best distance-to-target**: no
improvement ≥0.5 blocks for 10s → claim released, job marked `unreachable`,
skipped by future arbitration, logged. Arrival is **3D** distance < 2.5
(XY-only would count "standing on the surface above a deep job" as arrived).

## 4. Headless colony testing unlocked: force-loaded chunks

Colonists only promote in loaded chunks; the harness has no clients. New
hooks (all bastion-gated, reused by every later block's gate):
- `Server::bastion_force_load_area(center, radius)` — synchronously
  `world.generate_chunk` + terrain insert + `TerrainChanges::new_chunks` +
  `rtsim.hook_load_chunk` (the vanilla recipe minus wildlife supplements).
  **Gotcha:** `generate_chunk`'s `should_continue` closure actually means
  *"cancel?"* — passing `|| true` cancels every chunk (see
  `chunk_generator.rs`'s `cancel.load(..)`).
- `BastionForceLoaded` resource — the server unload sweep
  (`sys/terrain.rs`) skips pinned chunks.
- `Read<T: Default>` does NOT auto-register resources through the server's
  dispatcher setup path — `JobBoard`/`BastionForceLoaded` are inserted
  explicitly at `Server::new`.
- Ground scans must range above z=1000 (world altitudes exceed it).

## 5. Client → board path

`BastionPlaceDesignation` (B2a) now also generates jobs post-parallel-loop
(the board can't be touched inside `in_game.rs`'s par join — same deferral
pattern as B3's spawn). New `BastionCancelDesignation { region }` removes
jobs + releases claims; colonists re-idle within one upkeep tick because
their job id no longer resolves. No cancel UI yet (message + harness only;
B9 owns the tooling).

## 6. Notes for B5

- `ActiveJobState::Arrived` holds the claim forever (nothing completes work
  yet) — B5's work tick replaces this with progress → completion → release.
- Colonists visibly wander (rtsim civilised brain) whenever idle — correct;
  arbitration reclaims them next cycle.
- Job overlays: the B2a echo shows painted regions; per-job status render is
  B9's.
