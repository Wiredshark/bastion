# B3 findings — colonist entity model & starting colony

Spec: design doc §B3 (no prompt file in-tree) + §4 standing anchor directive.
Explored 2026-07-09 on `bastion/block-B3` (start `1fc5eba`).

## 1. The loaded↔simulated boundary is free if colonists are rtsim NPCs

- `rtsim::data::Npc` (`rtsim/src/data/npc.rs:282`): persisted fields incl.
  `seed/wpos/body/role/home/faction/personality`; `#[serde(skip)] mode:
  SimulationMode`. Creation: `Npc::new(seed, wpos, body, role)` +
  `data.npcs.create_npc(npc)` (assigns rtsim uid).
- **Promote**: `server/src/rtsim/tick.rs:660-673` — Simulated npc in a loaded
  chunk → `mode = Loaded` + `CreateNpcEvent` via
  `get_npc_entity_info(...)` → `to_npc_builder().with_rtsim(id)`.
- **Demote**: `server/src/rtsim/mod.rs:250` (chunk unload → `mode =
  Simulated`), then tick.rs:704 deletes the ECS entity.
- **Sync loop** tick.rs:676 joins loaded entities with `RtsimEntity` — the
  natural place to *decorate* promoted colonists (insert bastion comps + set
  `Stats::name` once) without touching vanilla spawn paths.
- Plan: colonists = rtsim NPCs with `#[serde(default)] bastion_colonist:
  Option<common::bastion::BastionColonist>` (name/skills/priorities stored
  rtsim-side so the roster works headlessly; serde(default) keeps old
  data.dat loading). Promote/demote logging added at the two mode flips,
  gated on the field.

## 2. Anchor inert + invulnerable (§4 directive gaps)

- Server already knows god mode: `presence.bastion_terrain_anchor.is_some()`
  (B1.6). The `ClientGeneral::BastionCameraAnchor` handler in
  `sys/msg/in_game.rs` is the enter/exit hook.
- **Invulnerable**: vanilla `BuffKind::Invulnerability` = `DamageReduction
  (1.0)` (`common/src/comp/buff.rs:459`). Apply as a permanent buff on
  anchor-set, remove on clear — reuses the entire damage path untouched.
- **No aggro**: `server/agent/src/util.rs:31 is_invulnerable` = has that
  buff; the behavior tree already *drops* invulnerable attackers/targets
  (`behavior_tree/mod.rs:285,958`). Verify/extend the initial acquisition
  path; add a `BastionGodAnchor` marker comp for filters that aren't
  buff-aware (greet/interaction, phys pushback).
- Marker: `comp::bastion::BastionGodAnchor` (unsynced), inserted/removed in
  the same handler.

## 3. Component sync

`common/net/src/synced_components.rs` x-macro is the single list; adding
`colonist: Colonist,` + inner re-export + `impl NetSync { SyncFrom::AnyEntity }`
propagates through `EcsCompPacket` and the server sync systems. B3 syncs
**only `Colonist`** (name/skills for markers + box-select); `PlayerColony`
(ownership), `Needs`, `Mood` stay server-side (B7/B-AG4 consume later).

## 4. Spawn + triggers

- Server fn spawns N rtsim colonists near a point (random humanoid bodies,
  `Role::Civilised(Some(Profession::...))`, name from a bastion name list,
  randomized skills; home = nearest site if cheap). Exposed as a pub method
  for the harness + driven in-game by new `ClientGeneral::BastionSpawnColony
  { pos, count }` (validated: god anchor set, count 1..=16).
- Voxygen trigger: radial Ground context gains `ContextVerb::FoundColony`.
- Harness: `--colony N` → boots, spawns, ticks, dumps a roster JSON line
  (names/skills) in addition to the Summary line.

## 5. Voxygen: markers + box-select

- **Markers**: per loaded `Colonist` entity, a small colored debug-shape
  marker above the head (same pipeline as designation overlays; B9 re-skins).
  Vanilla nametag shows the colonist name via the `Stats::name` override.
- **Box-select**: Inspect tool + left-drag = ground-plane rect (same
  plane-pick as designate-paint, yellow preview); release selects all
  `Colonist` entities whose world XY is inside. `bastion_selected` becomes a
  list; `BastionSelected` markers feed cutaway as before; info line shows
  count (or single-entity detail).

## 6. Risks / non-goals

- Colonists have no jobs/AI (B4+); they idle under vanilla civilised agent
  AI (fine — proves promote path drives real agents).
- rtsim data.dat compat: new field is `serde(default)`; old saves load.
- Greet/pushback filters are best-effort this block (cosmetic); the hard
  guarantees are no-damage + no-aggro-targeting.
