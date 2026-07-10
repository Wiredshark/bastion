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

## 2b. SCOPE ADDITION mid-block (architect relay of Ben's live b-2 test —
"the heart of the block"): DF-STYLE MINING BEHAVIOR

Observed live: on a 6-deep mine, colonists all rushed the DEEPEST cells,
couldn't reach them, and stalled. Required (and built as Step 2.5, in the
arbitration pass of `bastion_jobs.rs`):

1. **Reachability gate:** a Mine job is claimable only when EXPOSED — at
   least one of its 6 neighbors is non-solid (the cheap sound proxy for
   "a digger can stand next to it"; the watchdog + carve stay the safety
   net for exposed-but-unreachable). Interior cells unlock as the shell
   clears. NOTE the elegant side effect: carve-stair steps are emitted
   bottom-up and only the NEXT step is exposed at any time — the gate
   naturally sequences stair digging.
2. **Top-down frontier:** among exposed Mine jobs, scoring prefers HIGHER
   z (a level above outweighs several blocks of travel) — the dig clears
   layer by layer, DF-style, standing on each cleared floor to take the
   next.
3. **Dispersion:** claims taken this pass (and standing claims) repel new
   claims within 2 XY blocks — N colonists spread across the frontier
   instead of stacking on one corner. (The B6 work-crew dispersion item,
   pulled forward for mining.)

Gate additions (`--b58-scenario` part (d)): a 5×5×6 deep dig with 3
colonists — completes fully; layers finish strictly top-down; multi-claim
samples are majority-dispersed; a post-dig job outside the pit still
completes (the diggers carve their own way out — no one is entombed).
New harness hook: `bastion_claimed_job_positions`.

ALSO QUEUED (not this block): ABSOLUTE-FLOOR depth mode (flat-bottom
quarries) — backlog entry; architect put it on FLEET_STATUS NEXT.

## 2b-ii. SECOND scope wave (two more Ben directives, folded in)

**CLIMBING IS A SKILL** — built as: `ColonistSkills.climbing` (a MOVEMENT
skill, deliberately not a `WorkType`; `#[serde(default)]` so pre-B5.8
rtsim saves load; spawn rolls 0..=1 — most settlers start poor climbers).
`TraversalConfig.can_scramble` became **`scramble_reach: u8`** (0 = vanilla
NPC, 2 = novice colonist, 3 = climbing level 1+ unlocks the 3-up edges;
ladder edges need any reach > 0). XP accrues at 1.5/s while actually in
the `Climb` character state (bastion Sys reads `CharacterState`) — reach
literally grows with use. The same movement-skill shape extends to flying
entities later (do NOT fold it into the work-skill enum). Harness hooks:
`bastion_set_colonist_climbing` / `bastion_colonist_climbing`.

**AUTONOMOUS ACCESS IS THE DEFAULT** — the self-rescue branch now chooses
access that fits the geometry:
- **Access mask** (`in_access_mask`): claim box ±1 in XY, ≥ floor−1 in z,
  UNBOUNDED upward ("air rights" — a colony may always rise from its own
  claim to the surface; never tunnels sideways/down beyond the paint).
  This replaced the rise-expanded margin, which let spurious approach
  carves through wilderness-adjacent stone.
- **Stairs** where the claim has room: `carve_ramp` gained the mask
  parameter + SWITCHBACKS (snakes inside the mask, never reuses a column
  — a reused column's floor was already dug) + the FLOOR RULE (every
  step's under-block must be solid — stairs cannot route through open
  space). Multiple stair BASES are tried (the digger's cell + walkable
  neighbors) because a pit-escape's first step must cut into a wall
  column. Still ONE library — DIG-1's player verb passes its designation
  box as the mask.
- **Ladder** where it's tight or hollow: a material-free rung pillar
  (`ladder_pillar`) up an open column adjacent to the digger, starting one
  above the feet (lure-hole lips), topping out one above the target
  (dismount). Material gates re-keyed off `Job::required_item` so
  auto-access rungs flow free while PLAYER-placed ladders still cost
  stone (consistency note: infrastructure-from-spoil vs player builds).
- **Geometry choice falls out of the claim shape**: a 1-block-painted
  shaft is "tight" (stairs can't route in mask ±1) → ladder; a wide claim
  (incl. a Stockpile designation, which is a pure claim marker with zero
  jobs) → switchback stairs. Gate: b58 (b1) tight→ladder-sprites-present,
  (b2) roomy→stairs-and-no-ladder, (d) post-dig rescue with no manual
  input and no one entombed.
- ALSO fixed en route: Build/Ladder material consumption ate the WHOLE
  STACK (`inv.remove`) — a 6-stone stack vanished on the first ladder
  rung and the builder stopped being a carrier (run-2's 1/5 rungs).
  Now decrements one unit (`Item::decrease_amount`).

## 2c. First scenario run — findings (all fixed same-session)

Run 1 (pre part-(d), pre exposure-gate binary) was rich:

- **(a) SCRAMBLE WORKS.** The colonist traversed 1-step + 2-up + 3-up and
  cleared the top job — the jump → wall-contact → auto-Climb chain needs
  no agent-side changes, as predicted. (A spurious carve also fired during
  the LONG approach — see the zombie-job fix below.)
- **(b) PIT SELF-RESCUE WORKS END-TO-END.** Lured in over fall edges, got
  stuck on the 5-up ascent, carve branch fired, dug its own staircase,
  exited. The B5 pit-trap is mechanically solved.
- **(c) LADDER MOUNT-GAP (fixed):** vertical ladder edges originally
  required a ladder beside BOTH ends — but the ground cell below the
  bottom rung has no ladder at its own z, so there was no MOUNT edge (and
  no top-out past the top rung). Only rung 1 (arrival-reachable from the
  ground) ever got built; the carve safety net then rescued the top job —
  a nice accidental proof of the fallback. FIX: `beside_ladder(pos) ||
  beside_ladder(next)`.
- **ZOMBIE JOBS (fixed with a real mechanism):** a carve stair that cuts
  through a claimed job's own block left the job un-completable — moot was
  only checked at COMPLETION (arrival), so the job cycled
  claim→stuck→unreachable→retry forever and the board never drained. FIX:
  the same moot predicate now also runs DURING travel (one terrain read
  per traveling colonist per tick); jobs whose block changed under them
  drop immediately.
- **Long approaches can stall the watchdog on A* budget alone** (Chaser
  iteration caps across ~35 blocks of town) and fire a spurious carve
  (refused by the scope guard → churn). Scenario geometry moved closer to
  spawn; the mechanism note stands: STUCK_TIMEOUT vs approach length is a
  tuning surface, and the scope guard held (no wilderness was carved).

## 2d. Iteration state at the FLEET PAUSE checkpoint (runs 4–10, 2026-07-10)

Ten scenario iterations; every mechanism has passed at least once; the
remaining instability is localized. Scoreboard (STABLE = passed in all
recent runs):

- **(a) scramble gauntlet: STABLE ✓** since run 4 (climb assist) — 1-step,
  2-up, 3-up traversed, no carve assist (`a_max_total == 1`), climbing XP
  accrues. The reach-aware carve trigger (fire only when the ascent
  exceeds 2 + min(climbing,1)) killed the spurious approach carves.
- **(b2) roomy claim → auto-STAIRS: STABLE ✓** since run 5 — switchback
  stair carved through solid inside the Stockpile-claim mask, dug
  exposure-sequenced, colonist out, NO ladder placed. Geometry choice
  works.
- **(d) DF deep dig: STABLE ✓** since run 5 — 150/150 cleared, strict
  top-down layer completion (after making the depth weight MINE-ONLY ×16 —
  a top-weighted rung claim is unreachable until the rungs below exist,
  which froze ladder builds), dispersion 0.87–0.96.
- **(c) built-ladder climb / (b1) pillar exit / (d) rescue: OPEN.** Each
  passed in SOME run ((c) run 6, (b1) run 7) but they flip-flop. Fixes
  landed en route, each verified by traces: material stack-consumption
  (one unit, not the stack), `is_access` marker + ONE-PLAN-AT-A-TIME
  (run 7's trace showed three concurrent rescues digging each other's
  stair floors out into a useless gallery), the climb assist generalized
  to a deterministic "ladder elevator" (beside-ladder + job-target-above →
  vel.z floor; Climb-state entry is ~50% timing-flaky), then REACH-CAPPED
  on the wall/Climb arms (run 9 "passed" a pit exit by free-climbing a
  5-block wall — the cap keeps Ben's skill model honest: lift only while
  standable ground is within reach below; ladders exempt).
- **THE open diagnosis (run-10 (c) trace):** the climber attacks the WALL
  FACE at the reach cap and bobs, instead of walking to the ladder line —
  i.e. A* is apparently NOT ROUTING via the ladder edges and the Chaser
  falls back to straight-line bearing. NEXT ACTION (queued): unit-test
  `find_path` directly on a mock volume (ground + 4-wall + ladder column;
  in `common/src/path.rs` tests — `find_path` is module-private but
  in-file tests reach it) to pin the graph in milliseconds instead of
  8-minute sim runs; fix the edge generation; if execution still bobs at
  tops, add the "top-out" dismount edge (beside-ladder → diagonal-up onto
  a walkable cell, i.e. stepping onto the pillar top) and/or a snap
  dismount. Scenario staging (teleport hook) already removed the
  cross-town-travel confound — a separate pre-existing weakness, logged.

Run logs (b58-run3..10.log) lived in the session temp dir — perishable;
everything load-bearing is in this section.

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
