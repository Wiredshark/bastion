# ARCH-003 next-divergence report

## Scope and base

This work is based on reviewed commit `552cacaa3b` in the isolated
`E:\bastion-codex` worktree. It does not modify the live fleet checkout,
colonist traversal, emergency routes, stuck-watch, exit release, or ladder
fixtures. It makes no B5.5/B5.8 gameplay or disk save/load claim.

The harness process enables the existing `DETERMINISTIC_RTSIM` and deterministic
world-generation modes before parsing a scenario. This change does not alter
either gate. The dialogue fix deliberately consumes a new `tick_rng` stream,
which is deterministic only under that existing mode and remains OS-seeded in
live mode.

## Task 1 review follow-ons

- Required recorder and authoritative-observation tapes with `records == 0`
  are invalid. The unit test constructs matching empty evidence for both
  children and proves all four required recorder streams are rejected.
- Promotion may compact live ECS inventory slots (for example `589830 -> 4`).
  This is accepted only when paired reconstruction is deterministic and item
  definition/content, `item_hash`, amount, and gameplay selection remain
  stable. `InvSlotId` is explicitly documented as an ECS-lifetime address,
  not a persistent identity.

## Production search before the first divergence

All paired commands used seed 21 and only the allow-listed
`wall-unix-millis` normalization.

| Mapping | Evidence | Result |
|---|---|---|
| `world-summary` | `pre-fix-world-summary` | deterministic, gate pass |
| `lod0-promotion` | `pre-fix-lod0-promotion` | deterministic, gate pass |
| `needs-agent-state` | `pre-fix-needs-agent-state` | deterministic, gate pass |
| `archetype-entity-gen` | `pre-fix-archetype-entity-gen` | deterministic, gate pass |
| `bag1-agent-decision` | `pre-fix-bag1-agent-decision` | deterministic, gate pass |
| `b55-deep` | `pre-fix-b55-deep` | deterministic, functional fail in both children; exit 3, never a green |

The B5.8 recorder mapping and its preserved prior observations were not run or
changed, per owner scope correction.

## Measured first divergence

The next focused production action seam was
`rtsim::data::npc::Controller::dialogue_start`. Both isolated children passed
their functional fixture, but the authoritative observation diverged at tick
0:

```text
field: $.result.first_dialogue_id
writer: rtsim::data::npc::Controller::dialogue_start
run-a: 13213271902926357224
run-b: 3336267181154778293
```

Evidence:
`E:\bastion-codex-evidence\determinism-next-20260718\pre-fix-rtsim-dialogue-action`.
The source used `rand::rng().random()` directly for the session ID. That is the
measured root; no behavioral field was normalized.

## Fix

- `Controller::dialogue_start` now consumes an explicit RNG supplied by its
  caller; it no longer reaches thread/global RNG.
- `NpcCtx` carries a dedicated dialogue-identity `ChaChaRng`, derived through
  existing `tick_rng(world_seed, tick, npc_seed ^ DIALOGUE_ID_RNG_SALT)`.
- The ordinary NPC decision RNG remains separate, so a dialogue identity draw
  cannot shift action/body/config draws.
- The produced `u64` remains uniformly distributed. In live mode the existing
  `tick_rng` path remains entropy-seeded; under `DETERMINISTIC_RTSIM` the stream
  is reproducible per NPC and tick.

## Closure

Focused post-fix evidence:
`E:\bastion-codex-evidence\determinism-next-20260718\post-fix-rtsim-dialogue-action`.

```text
artifact_sha256: 1f2a8781f92209096ce484c64c18c2a74efe19e35a9005ebdd9dbc3193acfa56
deterministic: true
valid: true
gate_pass: true
first_divergence: null
first_dialogue_id: 3447711456709716563 (both children)
second_dialogue_id: 14380734479260864755 (both children)
```

The two IDs remain distinct within each run. The production-stream unit test
also proves identical supplied streams produce identical sequences.

Bounded real Agent/RTSim/physics evidence:
`E:\bastion-codex-evidence\determinism-next-20260718\post-fix-bag1-agent-decision`
is a valid
paired deterministic pass. It exercises production RTSim intent, Agent/Chaser,
and physics movement and shows the new identity stream did not perturb normal
Agent decisions. It is not a general gameplay acceptance claim.

## Commands

```powershell
cargo test -p veloren-rtsim `
  dialogue_identity_uses_the_supplied_independent_stream --jobs 1

cargo test -p bastion-harness determinism_regression::tests --jobs 1

target\debug\bastion-harness.exe `
  --determinism-regression rtsim-dialogue-action `
  --seed 21 `
  --determinism-normalize wall-unix-millis `
  --determinism-output <fresh-evidence-dir>

target\debug\bastion-harness.exe `
  --determinism-regression bag1-agent-decision `
  --seed 21 `
  --determinism-normalize wall-unix-millis `
  --determinism-output <fresh-evidence-dir>
```

## Proof boundary

This closes one measured RTSim dialogue-session identity divergence and stops.
It does not claim every NPC action, every seed, disk persistence, B5.5/B5.8
functionality, or full ARCH-003 convergence.
