# Project Bastion — DF-DIG-VERBS Design v0.1 (vertical excavation verbs)

**The player-facing designation vocabulary for vertical construction: stairwells, ramps, channels, and shaft
ladders** — the verbs that turn a flat "mine a voxel" (B5) into a colony *built downward and upward* through
Z-levels. Companion to the main build report (Slice B / B5–B6), the DF gap ledger (§K DF-DIG-VERBS), the
**B5.8 vertical-mobility** prompt (its paired traversal half), and the **mining framework** (`BASTION-SYSTEM-
FRAMEWORKS.md` §6 — DF-DIG-VERBS is the verb-set §6's planned access composes).

**Which wall:** split — **SIMULATION** (job decomposition of a shaped excavation into ordered per-block work)
+ **LEGIBILITY** (the genuinely hard part: painting and *reading* vertical excavations in a top-down,
Z-sliced god camera). The **CONTENT wall is a non-issue** — stairs/ramps are terrain, not assets; the one
sprite involved (`Ladder`) already ships.

**Fit-check verdict: PASS (clean).** This is pure **designation** — the exact B4/B5 model (you paint intent;
colonists dig autonomously), extended into the third dimension. You never command a digger; you designate a
stairwell and the colony builds it. No unit micro, no 4X. Colony sims are *made* of these verbs; a fort with
no stairs is not a fort. Strongly in-genre and squarely on the pillar.

**Ledger/corpus entries this consolidates:** `df-feature-gap-ledger.md` §K **DF-DIG-VERBS** ("up/down stairs,
ramps, channels, up/down passages — DF's fortress is *built* from these"). It is the **construction** companion
to **B5.8** (the **traversal** — scramble/carve/ladders) and the **child** of the mining framework §6 (which
composes these verbs into planned mine access). Appends to the corpus; rewrites nothing.

---

## 0. The one thing to get right first — DF-DIG-VERBS *builds* the vertical link; B5.8 *walks* it

There are two separable problems the corpus already names, and conflating them is the trap:

- **B5.8 (vertical mobility) = traversal.** *Can a colonist get up/down?* — scramble a ledge, climb a ladder,
  or descend a carved stair. This is locomotion + pathfinding cost. **Prerequisite, not this doc.**
- **DF-DIG-VERBS = construction vocabulary.** *What shaped hole/structure does the player designate, and how
  does the colony dig it?* — a stairwell spanning 5 levels, a ramp between two, a channel that opens the floor.

They are **paired**: a stairwell the colony can't climb is useless (needs B5.8), and B5.8's *auto*-carve-steps
sub-job is nothing but the *automatic* invocation of this doc's **ramp verb** (see §2 — build once, two
callers: player-designated and colony-auto). **North star: DF-DIG-VERBS produces walkable/climbable geometry;
B5.8 is what makes it walkable/climbable; the two ship together or DF-DIG-VERBS is inert.**

---

## 1. The reuse split — the de-risk table

DF-DIG-VERBS is **mostly wiring on top of B5**: the geometry generators, the traversal sprite, and the Z-slice
camera all exist; the net-new is a designation vocabulary + a safe job-decomposition.

### SUBSTRATE — exists, needs wiring

| Piece | Real symbol / location | What it gives us |
|---|---|---|
| **Dig work-tick** | `server/src/bastion_jobs.rs` — B5 `WorkType::Mine`, `BlockChange::set`, drop/XP | Every vertical verb decomposes into ordinary per-block Mine jobs. **The work executor already exists.** |
| **Ramp geometry** | `world/src/site/generation.rs:1184` — `Painter::ramp(aabb, dir) → Primitive::Ramp { aabb, inset, dir }` | The slope shape (reference algorithm for a stepped ramp). |
| **Staircase geometry** | `generation.rs:1363` `staircase_in_aabb(aabb, thickness, start_dir)`, `:1870` `spiral_staircase(...)`, `:1890` `wall_staircase(...)` | Full straight/spiral/wall staircase generators — used across worldgen (houses, bridges, towers). **The shape of a "dug staircase" is already an algorithm in-tree.** |
| **Ladder (climbable vertical link)** | `common/src/terrain/sprite/mod.rs:255` — `SpriteKind::Ladder = 0xC2` | The 1-tile vertical passage (B5.8 §3's "up/down passage"). Placeable via the B5 Build path. |
| **Climb state** | `common/src/states/climb.rs` — `CharacterState::Climb` (skilled: `ClimbSkill::{Cost, Speed}`) | Colonists climb ladders/faces via a shipped state (B5.8 wires it; the animation is NATIVE). |
| **Z-slice god camera + overlays** | B1.6 (`CameraMode::Overseer`, PageUp/Down slice, cutaway) + B5.6 draped overlays (`overlay_surface_z`, `draped_fill_tris`) | The substrate for the hard part — painting/reading vertical designations across levels. |
| **Designation loop** | B2a/B4 — `Region` (3D AABB), `DesignationKind::Mine`, paint→server→echo→overlay | Vertical verbs are new `DesignationKind`s on the existing paint pipeline; `Region` is already 3D (`z_extent`). |
| **Downward reachability rule** | `BASTION_ARCHITECTURE.md §5` gotcha (the pit-trap: an enclosed digger traps itself; B5's mine test hand-carved an exit ramp) | The exact failure DIG-0's top-down ordering must design *out* at the verb level. |

### BUILD — genuinely net-new

| Piece | Why it's new | Folds into |
|---|---|---|
| **Vertical designation verbs** (`DownStairwell`, `Ramp`, `Channel`, `ShaftLadder`, `UpStair/UpRamp`) | B5 has only flat `Mine`/`Build`; no verb targets a shaped vertical excavation. | The B2a/B4 **designation model** (new `DesignationKind`s) |
| **Safe job-decomposition** | A shaped excavation must become an **ordered** set of B5 dig/build jobs that never traps the digger (top-down for descents, support-below for ramps — the DF "supported ramp" + the pit-trap lesson). | The B4/B5 **job board** + the mining-framework "access is part of the dig plan" §6 |
| **Result-traversability marking** | The dug geometry must be flagged walkable/climbable so B5.8 pathing uses it. | **B5.8** (shared seam — the traversability annotation) |
| **Vertical designation UX** | Painting a multi-level shaft in a top-down Z-sliced camera + previewing its full extent before commit. | B1.6 Z-slice + B5.6 overlay renderer |

**The collapse:** the executor (B5), the shapes (worldgen primitives), the vertical link (Ladder+Climb), and
the camera (B1.6/B5.6) all exist. DF-DIG-VERBS is **a designation vocabulary + a decomposition that respects
reachability + a UX** — no new sim engine, and (see §5) **no new animations**.

---

## 2. Build-once: the ramp verb IS B5.8's auto-carve-steps (don't build it twice)

The single most important unification in this pass: **B5.8's "auto-carve-steps sub-job" and this doc's `Ramp`
designation verb are the same mechanism with two callers.**

- **Player-designated (DIG-1):** the player paints a ramp between level A and B; it decomposes into ordered dig
  jobs; result is walkable.
- **Colony-auto (B5.8 §2):** when a job's path needs a vertical transition through owned terrain, the pathing
  system *auto-emits the same ramp decomposition* to carve the colonist's way out (the pit-trap solved by the
  system).

Both must call **one** `carve_ramp(from_z, to_z, footprint) → ordered Vec<DigJob>` routine. Building them
separately duplicates the reachability-critical ordering logic and guarantees they drift. **Design law for this
cluster: the ramp/stair decomposition is one library; player-paint and colony-auto are two entry points.** This
is the same discipline as the world-verb action library and the trigger→link→effect engine.

---

## 3. The verb set (DF → Bastion reframe)

DF's tile-grid dig designations, reinterpreted for a voxel world with scramble+ladder+climb traversal (B5.8).
DF's abstractions that are pure tile-grid artifacts (e.g. "stairs must be built in pairs, up-stair meeting
down-stair on adjacent levels" — [DF Stairs](https://dwarffortresswiki.org/index.php/40d:Stairs)) are dropped;
the voxel model expresses the intent more directly.

| DF verb | DF semantics (cited) | Bastion verb | Voxel realization |
|---|---|---|---|
| Up/Down Stair (i/u/j) | Vertical-only movement; stairs work in pairs across adjacent z-levels | **DownStairwell / ShaftLadder** | A shaft column: either a **stepped staircase** (reuse `spiral_staircase`/`staircase_in_aabb` fill pattern, walkable) or a **Ladder column** (1-tile, climbable via `Climb`). Spans N levels in one designation. |
| Ramp (r) | Moves diagonally across Z in one step at flat-ground cost; **must be supported by a solid tile underneath** ([DF Ramp](https://dwarffortresswiki.org/index.php/DF2014:Ramp)) | **Ramp** | A 1-wide **stepped diagonal** of blocks (each tile one block higher), walkable by scramble/step-up. = B5.8's carve-steps (§2). |
| Channel (h) | Digs the floor away; **creates a ramp on the level below** ([DF Channel](https://new.dwarffortresswiki.org/index.php/Channel)) | **Channel** | A **downward Mine**: remove the floor block(s) to open the tile to the level below; v1 leaves the exposed lower block as a scramble-able edge, ramp-below as polish. |
| (constructed) Up Stair / Up Ramp | Build vertical access on open space (not carved from stone) | **UpStair / UpRamp** | The **Build** path (B5 Build + material): place stepped blocks / a Ladder going *up* a face or in open air. |

**One control knob, DF-faithful:** a `DownStairwell` / `Ramp` designation carries a **depth** (`z_extent`,
already in the `Region` schema) — "dig down N levels" — which is the mining-framework §6 "mine zone (8 levels
down)" affordance at the single-verb level.

---

## 4. Systems needed (with deps + build-once engine)

### S1 — Vertical designation verbs (`DesignationKind` extension)
New `DesignationKind::{DownStairwell, Ramp, Channel, ShaftLadder, UpStair, UpRamp}` on the B2a/B4 paint
pipeline; each carries depth/direction. **Where:** `common/src/bastion.rs` (enum), voxygen tool palette
(B2a). **Deps:** B2a/B4. **Folds into:** the designation model.

### S2 — The shaped-excavation decomposer (the reachability-safe core)
One library: `decompose_vertical(verb, region) → ordered Vec<Job>` producing B5 Mine/Build jobs in an order
that **never traps the digger** — descents top-down (dig from the level you stand on), ramps with support
below, stairwells level-by-level. This is the verb-level answer to the pit-trap gotcha (`ARCHITECTURE §5`), and
the shared routine §2 requires. **Where:** new `server/src/bastion_dig.rs`. **Deps:** B5 job board. **Folds
into:** the job board + mining-framework §6 (its planned-access solver calls this).

### S3 — Traversability marking (shared seam with B5.8)
The dug result must be annotated walkable (stepped ramp/stair) or climbable (Ladder column) so B5.8 pathing
routes over it. **Where:** the seam B5.8 owns (path-cost model / vertical-link graph annotation — B5.8
watch-item). **Deps:** **B5.8 (hard)**. **Folds into:** B5.8.

### S4 — Vertical designation UX (the hard, legibility-first part)
Painting a multi-level shaft in the top-down Z-sliced camera: a depth picker ("dig down N"), a ghost preview
of the shaft's full vertical extent *across slices* before commit, and a per-slice overlay of the pending
excavation. **Where:** voxygen (B1.6 Z-slice + B5.6 draped-overlay renderer, `overlay_surface_z` authority).
**Deps:** B1.6, B5.6. **Folds into:** the overlay renderer.

---

## 5. Assets & animations (both nearly free — the rare cluster with no new debt)

**Assets — mostly terrain, no generation gate:**
| Asset | Tag | Notes |
|---|---|---|
| Stairs / ramps | **n/a (terrain)** | Dug from ordinary blocks; no model. |
| Ladder | **READY** | `SpriteKind::Ladder` ships; asset-lab may add wood/rope variants (READY — system consumes them). |
| Handrail / support-beam / mine-shaft-frame dressing | **NEEDS:DF-DIG-VERBS** → READY on DIG-2 | Pure polish that makes a shaft *read* as built; ties the mining-framework asset batch (wishlist §1). |

**Animations — NATIVE across the board (no new debt):**
| Verb | Tag | Basis |
|---|---|---|
| Dig stair/ramp/channel | **NATIVE** | Pickaxe swing (B5 mining is NATIVE, §3u). |
| Climb ladder / scramble | **NATIVE** | `CharacterState::Climb` ships (B5.8 wires it). |
| Build up-stair/up-ramp | **NATIVE** (inherits B5 Build) | Same build-hammer debt as B5 Build — not new here. |

This is the rare cluster that ships **no new animation code** — every gesture reuses mining, climbing, or the
existing build verb. Worth noting explicitly against the §3u rule: DF-DIG-VERBS has **zero T-pose risk**.

---

## 6. Legibility · Control-spectrum · LOD

**Legibility (the crux):**
- **Dug geometry is self-legible** once excavated — a staircase/ramp renders as terrain; a channel opening
  shows through the B1.6 cutaway/roof-reveal. The *result* reads for free.
- **The hard part is the designation, not the result:** the player must *paint a shaft they can't see through*
  in a top-down view. S4's cross-slice ghost preview + depth picker is the answer — this is a genuine
  legibility design problem, first-class here, not an afterthought.
- **Chronicle (DF-LOG):** notable digs (breached a cavern — the §6 Breach Event; reached bedrock) log to the
  world's memory.

**Control-spectrum (§3d / frameworks §1):**
- **Autonomous:** the mining framework §6 (autonomous ore survey → plan → dig) *composes these verbs* with no
  player input — a colony stairs its own mine.
- **Manage:** "mine zone, N levels down" (the depth knob, §3) — set the shape, colony fills it.
- **Direct (paint-your-own):** hand-paint each stairwell/ramp — the DF-fort-builder's mode, present but not
  required.
- **God layer:** minimal here (terrain-raise/lower is B13's terraform, out of scope) — DF-DIG-VERBS is a
  *colonist* verb set, not a divine one.

**LOD:** dig verbs are **loaded-only work** (like B5) — no per-block vertical digging in rtsim; an unloaded
colony's excavation is a **summary** (mine depth / levels dug), materialized on load. No accumulation to decay
here (terrain is permanent); the "carrying capacity" is geology (you can't dig past bedrock / into the caverns
without the Breach Event, §6).

---

## 7. Sequenced sub-blocks, each with a concrete Done-when

Dependency-ordered. **v1 = DIG-0..DIG-2 (paired with B5.8); enrichment = DIG-3..DIG-4.** All Done-whens
invariant-first where sim, eyeball where visual. **Hard pairing:** DIG-1/DIG-2 need B5.8 traversal to be
provable end-to-end — sequence B5.8 first or co-build.

### DIG-0 — Downward Mine / Channel + the safe decomposer · [the pit-trap, solved at the verb level]
**Depends:** B5. Builds S1 (Channel) + S2 (decomposer).
**Scope:** a `Channel`/downward-`Mine` designation with a depth; decomposed **top-down** so the digger always
stands on solid ground above the block it removes (never seals itself below).
**Done-when (`--dig-channel-scenario`):** paint a "dig down 5" channel in a solid mass; colonists excavate it
**top-down**, each block removed from a reachable standing position, the shaft opens level-by-level to the
bottom, and the digger is **never trapped** (no stall, no hand-carved exit — the B5 pit-trap workaround is
*unnecessary*); drop conservation holds (Σ blocks removed == Σ stone dropped); zero-input soak stable.

### DIG-1 — The ramp verb (= B5.8 carve-steps, unified) · [walkable Z-transition]
**Depends:** DIG-0, **B5.8 (traversal)**. Builds S3 + the shared `carve_ramp` routine (§2).
**Scope:** a `Ramp` designation between two levels → a stepped diagonal, marked walkable; **the identical
routine B5.8's auto-carve-steps calls.**
**Done-when (`--dig-ramp-scenario`):** paint a ramp from level A to level B; colonists carve it; a colonist on
level A then **paths up the ramp to a job on level B** (and back) with no stall — proving traversability
integration. Assert the routine is shared: B5.8's auto-carve path and the player-paint path invoke the same
`carve_ramp` (one code path, verified by both scenarios exercising it).

### DIG-2 — The stairwell verb (multi-Z shaft: stepped + ladder variants) · [the workhorse]
**Depends:** DIG-1, B5.8 (Climb for the ladder variant). Builds the `DownStairwell` + `ShaftLadder` verbs.
**Scope:** designate a shaft spanning N levels → either a stepped staircase (reuse `spiral_staircase`/
`staircase_in_aabb` fill) or a Ladder column; decomposed safely (S2).
**Done-when (`--dig-stairwell-scenario`):** designate a 4-level stairwell (once as stepped-stair, once as
ladder); colonists build each; a colonist at the top **descends all 4 levels to a job at the bottom** and
returns — for the ladder variant via `Climb`. No trap, conservation holds, bounded ticks.

### DIG-3 — Constructed up-stairs / up-ramps · [enrichment: above-ground vertical build]
**Depends:** DIG-1, B5 Build. Builds `UpStair`/`UpRamp` on the Build path.
**Scope:** build stepped stairs / a ramp / a ladder *upward* on open space (material-gated, like B5 Build).
**Done-when (`--dig-up-scenario`):** build a ramp (and a ladder) up a 3-block wall; a colonist **ascends** to
a job at the top; material consumed per B5 Build gate; conservation holds.

### DIG-4 — Vertical designation UX / legibility · [enrichment: the hard UX]
**Depends:** DIG-0..DIG-2, B1.6, B5.6. Builds S4.
**Scope:** depth picker, cross-slice ghost preview of the pending shaft, per-slice pending-excavation overlay.
**Done-when (eyeball/screenshot):** in the overseer view you can paint a stairwell/ramp/channel with a depth,
**see its full vertical extent previewed across Z-slices before committing**, and read the pending excavation
on each slice — a first-time player can designate a 5-level stairwell without guessing where it goes.

---

## 8. Dependencies · open questions · tuning-data · corpus notes

### Dependencies (build-order truth)
- **B5 (dig) — DONE.** The executor exists.
- **B5.8 (vertical mobility) — HARD PAIR.** DF-DIG-VERBS produces geometry B5.8 traverses; DIG-1/DIG-2 aren't
  provable without it. **Sequence B5.8 first, or co-build the two** (they share the `carve_ramp` routine + the
  traversability seam). This is the single most important scheduling note.
- **B1.6 (Z-slice) — DONE; B5.6 (overlays) — in progress.** For DIG-4's UX.
- **Mining framework §6 — parent.** Its planned-access + Breach Event *compose* these verbs; DF-DIG-VERBS is a
  dependency of the autonomous-mine mode, not vice-versa.

### Open questions (flagged for the architect)
1. **Stairwell representation** — stepped-voxel staircase (walkable, takes footprint) *and* ladder-column
   (1-tile, climbable) as two variants, or pick one? *Rec:* both — ladder for cheap vertical links, stepped
   stair/ramp for walkable multi-use descents. (Baked into DIG-2.)
2. **Drop DF's paired-stairs abstraction?** *Rec:* yes — it's a DF tile-grid artifact; the voxel stepped-ramp +
   ladder-column express the intent directly. (Baked into §3.)
3. **Channel-creates-ramp-below** (DF rule) — replicate, or just open the hole? *Rec:* v1 opens the hole +
   leaves a scramble-able edge; ramp-below is polish (keeps DIG-0 minimal).
4. **Does DIG-0's top-down decomposer *become* the mining-framework §6 access solver, or stay minimal?** *Rec:*
   minimal here (safe ordering for a single designated shape); §6 owns the general planned-access solver and
   *calls* DIG-0/DIG-1's routines. (Prevents over-building ahead of the mine framework.)

### Tuning-data (RON/config)
Ramp slope (blocks per horizontal step, default 1); ladder climb speed (shared with B5.8 `TRAVEL_SPEED`);
stairwell footprint/thickness; dig-job ordering safety margins; max designation depth per paint.

### Corpus notes (consistency, not contradiction)
- **Unifies with B5.8, doesn't duplicate it:** the `carve_ramp` routine is explicitly *shared* between B5.8's
  auto-carve and DIG-1's player-paint (§2) — flagged so a builder doesn't implement it twice. This is a
  *strengthening* of the corpus (two docs that referenced the same carve-steps now name it as one library).
- **Child of the mining framework §6:** consistent — §6 says "access is part of the dig plan"; DF-DIG-VERBS is
  the verb-set that plan is expressed in. No conflict.
- **Zero new animation debt** — worth recording against the §3u rule (every verb NATIVE via mining/climb/build).

## 9. Honest limits
- **Inert without B5.8.** The verbs *build* stairs; if colonists can't climb/walk them (B5.8), DIG-1/DIG-2 are
  undemonstrable. This design is honest that it's the *construction half* of a two-part capability — do not
  ship it as "vertical mobility done."
- **DIG-4 (the UX) is the real risk, and it's a design problem, not a wiring one.** Painting invisible vertical
  shafts in a top-down camera is genuinely hard; the substrate (Z-slice, overlays) exists but the *interaction*
  is unproven. Correctly placed last (enrichment) and flagged, not hand-waved.
- **Everything else is low-risk wiring** — the executor, shapes, and traversal sprite are all in-tree; the
  pit-trap ordering is a known, bounded problem with a known shape (top-down).

*End of DF-DIG-VERBS design. The vertical world DF is famous for is, in this repo, a designation vocabulary + a
reachability-safe decomposer on top of B5 — no new sim, no new animation — paired with B5.8's traversal, with
the real design work sitting in the top-down painting UX.*
