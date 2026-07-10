# B5.8 findings + build plan — vertical mobility (the 4×-bitten trap)

Explored 2026-07-09 on `bastion/block-B5.8` (start `efc777475a` =
`bastion-block-B5.6b-2`). Spec: `readme/B5.8-vertical-mobility-prompt.md`.
HARD PAIR: `carve_ramp` is ONE library shared with DF-DIG-VERBS DIG-1
(`readme/DF-DIG-VERBS-design.md` §2 — "two callers, one routine" is design
law; DIG-0's top-down-safe decomposition rule applies to the shared lib).

## 1. Seam inventory (all verified in code this session)

### Path graph (`common/src/path.rs`, neighbor gen ~line 870)
- `DIRS` already includes **1-step up** moves (`(±1|0, 0|±1, 1)`) and 1-down;
  **falls** are edges down to 11 blocks (cost `FALL_COST=1.5`/block). So
  1-step and descent were never the trap.
- `JUMPS` = **2-up** edges, gated `standing-on-non-liquid || can_climb ||
  can_fly`, with headroom filters (`pos+2z`/`pos+3z` non-solid checks at
  lines ~944-958).
- **There are NO 3-up edges.** A ≥3-block face has no graph edge at all →
  A* fails → agent stands at the base → our 10s progress watchdog fires →
  `unreachable`. THAT is the 4×-bitten trap's mechanism.
- `TraversalConfig.can_climb` (path.rs:99) is set from `Body::can_climb()`
  (`server/src/sys/agent/mod.rs:195`), which is **humanoid-only**
  (`common/src/states/utils.rs:386`). Colonists are humanoids → they
  already carry `can_climb: true`; the graph just never offers >2-up.

### Execution (behavior tree + character states)
- Goto execution (`server/src/sys/agent/behavior_tree/mod.rs:439-444`,
  508-510): `move_dir = bearing.xy()`, `jump_if(bearing.z > 1.5)`,
  `move_z = bearing.z`. Bearing comes from `Chaser` over the path above.
- `handle_climb` (`common/src/states/utils.rs:1018`): **auto-enters the
  Climb state** whenever airborne + moving toward a wall + `Body::can_climb`
  + energy > 1. No input beyond move_dir/jump — so once the graph offers a
  2-3-up edge, the existing jump→wall-contact→auto-climb chain executes it.
  Climb drains energy; colonists have energy comps; a 2-3-block scramble is
  a small drain (relaxation hook exists if the gate shows starvation —
  `climb::Data::create_adjusted_by_skills`).

### Ladders
- `SpriteKind::Ladder = 0xC2` EXISTS (`common/src/terrain/sprite/mod.rs:255`)
  with `solid_height 1.0` (line ~773) — a solid, standable/climbable-faced
  block sprite. Asset log confirms the .vox exists; asset-lab has a rope
  variant READY. **No new asset needed.**
- There is NO climbable-block flag anywhere in common (grep `climbable` =
  zero hits) and no pathfinding awareness of sprites. Ladder pathing is
  net-new — the prompt's sanctioned fallback ("vertical link graph
  annotation at known points") is the right v1: ladder-adjacent vertical
  edges injected in neighbor gen.
- Physics: the ladder block is solid → a colonist beside the column gets
  `on_wall` against it → auto-climb works. So vertical edges belong at
  positions ADJACENT to a ladder block, not inside it.

### Job system hook (`server/src/bastion_jobs.rs`)
- Travel watchdog: progress-based, `STUCK_TIMEOUT=10s` → release claim +
  `unreachable=true`; retry sweep at `tick%60`. The carve-steps decision
  point is exactly the watchdog branch: BEFORE marking unreachable, ask
  "is self-rescue possible?" (job inside colony designation + diggable).
- SCOPE GUARD data gap: the board keeps only jobs, not the designation
  regions they came from. Auto-carve must be confined to colony-designated
  terrain → the board gains `designated: Vec<Region>` (append on place;
  `Region::subtract` on cancel — exact machinery already unit-tested in
  common). Union-of-regions is the carve permission mask.

## 2. Build plan (mechanism order per spec — cheapest first)

### Step 1 — SCRAMBLE (path graph + verify execution)
- Add `SCRAMBLES: [Vec3; 4]` = `(±1|0, 0|±1, 3)` edges in path.rs neighbor
  gen, gated on `traversal_cfg.can_climb` ONLY (not plain standing — a
  3-up is a climb, not a jump), with the same-pattern headroom filter
  extended one block (`pos+4z` non-solid for dir.z≥3) and a transition
  cost premium (scrambles are slower than jumps — tune so stairs are
  preferred when they exist).
- No agent-side change expected (jump→auto-climb chain, above). Verify in
  the scenario, not by assumption — if 3-up execution stalls, the fallback
  is a small "boost move_z while on_wall" nudge in the goto arm (document).
- Harness: `--b58-scenario` part (a): terraformed 1-step + 2-up + 3-up
  ledges between colonist and job; assert arrival, no stall, job done.

### Step 2 — CARVE-STEPS (the shared `carve_ramp` + the watchdog hook)
- `common/src/bastion.rs`: `pub fn carve_ramp(from: Vec3<i32>, to:
  Vec3<i32>, is_solid: impl Fn(Vec3<i32>) -> bool) -> Vec<Vec3<i32>>` —
  PURE GEOMETRY, ordered TOP-DOWN (DIG-0 law: the digger always stands on
  solid ground above/beside the block it removes; never seals itself
  below). v1 shape: 1-wide stepped diagonal from `to`(upper) descending
  toward `from`(lower), each step = the step block + headroom clears
  (2 above), emitted in dig-safe order. Unit-test the ordering invariant
  (no emitted block is below an earlier-emitted block in the same column;
  each step reachable from the previous). THIS is the routine DIG-1's
  player Ramp verb calls later — signature stays caller-agnostic.
- `server/src/bastion_jobs.rs`: board gains `designated: Vec<Region>`
  (place appends resolved bounds; cancel subtracts). Watchdog branch: on
  stuck, if the job pos is inside `designated` AND a carve line from the
  colonist's position to the job (or pit rim) lies entirely inside the
  mask AND is diggable → emit carve Mine-jobs via `carve_ramp` (dedupe on
  occupied set as always) and DO NOT mark unreachable this cycle; else
  unchanged behavior. Guard rails: one carve attempt per job (a
  `carve_attempted` flag on the Job) so a failed carve degrades to
  unreachable, not a loop; never carve outside the mask (the
  never-chase-a-deer rule).
- Harness part (b): the B5 pit-trap re-created VERBATIM — 5-deep pit job,
  no exit — assert the carve sub-jobs appear, the colonist digs its own
  way out, finishes, exits. Spoil follows normal drop rules (conservation
  assert on total stone).

### Step 3 — LADDERS (placement + graph awareness)
- `DesignationKind::Ladder` (wire enum addition — both sides recompile
  together; net-protocol note in ledger like b-2's). `work_type()` = Build;
  `required_item = BUILD_MATERIAL_ITEM` (reuses B5 Build's material gate +
  needs_materials machinery wholesale). Job gen: paint path already sends
  footprint + z_extent (b-2!) — a ladder drag is a 1×1 footprint with
  `up: N` — jobs = place `Block::air.with_sprite(SpriteKind::Ladder)` at
  each level, generated bottom-up… ordering: Build completion re-validates
  `can_set_block`; bottom-up emerges from reachability (upper rungs become
  reachable as lower ones land + climb). Completion writes the sprite via
  the same `BlockChange::set` path as Build (sprite-on-air block).
- path.rs: in neighbor gen, if any horizontal neighbor of `pos` (or of
  `pos+dir`) holds `SpriteKind::Ladder` → offer `(0,0,±1)` edges at low
  cost. Implementation note: neighbor gen has `vol.get` access already —
  sample `block.get_sprite()`. Keep the check cheap (only when the
  vertical move would otherwise be absent).
- Client: Ladder joins the tool cycle (`ToolMode::ALL` — 8 entries),
  `zone_rgb` gets a ladder color (extend THE one legend, never a second
  mapping), purpose() → None (structure, like Build).
- Harness part (c): 4-block wall, ladder column placed (server hook), job
  on top → colonist climbs, completes.

### Step 4 — remove hand-patched access geometry where covered
- B5 quarry scenario: the hand-carved exit ramp on the 3-deep pit
  (`mine pit exit ramp` from the B5 fix) — remove it; scramble (2-3-up)
  must cover pit exit. Keep the forced-flat rim (that guards a different
  gotcha: natural-terrain nondeterminism).
- B4/B5.5: their geometry guards determinism (slab under-fill, perimeter
  ring footing), NOT vertical access workarounds — leave, with a findings
  note (the architecture-guide §5 rule "test terraforms must fully
  determine geometry" still stands).

### Step 5 — gate + bookkeeping
- `--b58-scenario` (a)+(b)+(c) + re-run B4/B5(+7.5)/B5.5 + vanilla boot
  (player climbing untouched — scramble edges are agent-pathing only,
  gated can_climb which players don't path with; ladder sprite exists in
  vanilla worldgen already, unchanged) + unit tests (carve_ramp ordering).
- Architecture §5: close the trap's bite-list entry (mechanism fixed);
  §2.11 B5.8 section. Backlog: close the standing vertical-reachability
  item; add "mine framework supersedes carving with planned access (§3v)".
- TRAVEL_SPEED/climb-speed eyeball → next batched TEST LIST for Ben.

## 3. Risks / watch items
- **Path-cost integration is the flagged risky bit.** The 3-up edge add is
  small and pattern-following, but `find_path` is shared with ALL agents
  (vanilla NPCs!) — a bad cost makes villagers climb walls. Mitigation:
  scramble edges gated on `can_climb` (humanoids only — same set that can
  already 2-up via climb gate) + a real cost premium; vanilla boot +
  eyeball watch. If regression risk reads too hot at build time, the
  sanctioned fallback: gate 3-up edges on a NEW `TraversalConfig` flag set
  only for colonists (Body stays untouched) — cleaner isolation, one more
  plumb-through. DECIDE AT BUILD: start with the flag (zero vanilla risk),
  note the seam.
- Climb energy: colonists starting low-energy may fail mid-scramble;
  watchdog catches it (progress-based) → carve fallback. Note for tuning.
- Carve spoil inside the pit can bury the job block (stone drops are
  items, not blocks — no burial; fine).
- rtsim (unloaded) colonists don't path this way at all (loaded-only work,
  per DF-DIG-VERBS LOD note) — no rtsim changes.
