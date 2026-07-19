# Stair-style and ladder-style mine access — design

Status: **DESIGN, Phase 2 build-ready — Phase 1 CK fix has LANDED** (tag
`bastion-block-CKSTAIR` → `9ad9d97808`, see Phasing below). Owner direction (Ben,
2026-07-17). This doc is the builder spec for Phase 2.

## Intent

A dug-out mine, and any emergency escape from a pit/shaft, is reached by one of
**three** access styles, chosen by depth and by whether the top is open:

- **Open stairs** (shallow, exposed excavation): the excavation itself is a walkable
  open-top staircase. Colonists walk in and out. No climbing, no ladder, no traversal
  task. Two geometries — single-edge wide staircase (A) and perimeter spiral (B).
- **Enclosed delving staircase** (deep, NO exposed top — the "traditional deep mine",
  Minecraft-style): an entrance at the surface, then a covered switchback staircase
  bored *down through solid rock* to a deep target. Walkable, minimal excavation
  (only the stair tunnel is cut — the ground above stays intact). Geometry C.
- **Ladder shaft** (deep, compact): a vertical shaft with a constructed ladder.
  Colonists climb. This is route-owned traversal (the Stage-1 `BastionTraversalTask`
  path) — the ONLY style that climbs.

Selection: shallow open work → open stairs (A/B). Deep target with minimal excavation
→ enclosed staircase (C, walk) or ladder shaft (climb) by footprint/traffic. All three
STAIR styles (A/B/C) are walkable and take no route ownership; only the ladder climbs.

## Why this is also the correct architecture (and the CK fix)

A real staircase is **walkable by construction**. It therefore needs no climb, no
`CharacterState::Climb`, no `BastionTraversalOwnership`, no traversal executor — the
colonist just walks the geometry, exactly like baseline `7f087da317` did with its
plain `(digs, DesignationKind::Mine)` stair plan.

The CK entombment regression happened because Stage-1 wrapped a walkable stair plan in
an `EmergencyRouteDescriptor { kind: CarvedStair }` and registered route ownership for
it, then never wrote a `CarvedStair` executor. The colonist was parked in
`RouteOwnedWaiting`/`link_queue_waiting`, waiting for a traversal task that could not
exist, until the teleport backstop bailed him out at the deadline. See
[the CK forensics in the run log] and the recorder trace (uid 1, seed 1337:
`route_kind=CarvedStair`, `on_wall=None` in 730/730 samples, energy pinned at 100.0).

So this design and the correct fix are the same shape:

| Style | Movement | Route ownership | Executor |
|---|---|---|---|
| Open stairs A/B (shallow) | **walk** | none | none — walks the dug geometry |
| Enclosed staircase C (deep) | **walk** | none | none — walks the bored stair tunnel |
| Ladder shaft (deep) | **climb** | `BastionTraversalTask` | Stage-1 ConstructedLadder owner |

Phase 1 restores stairs to the walk/no-ownership shape (unblocks CK). Phase 2 makes
both styles first-class, adds the two stair geometries, and applies the depth rule to
player-designated mines as well as emergency egress.

## Prior art

- **Dwarf Fortress**: ramps (walkable, cheap, wide) vs. up/down stairs; players pick by
  depth and traffic. Walkable ramps need no special traversal — the pathfinder just
  walks them. This is the model.
- **Minecraft**: staircase mine (walk, more excavation) vs. ladder shaft (compact,
  climb). Depth/traffic tradeoff is the folk knowledge this formalizes.
- **In-repo reuse**: `common::bastion::carve_ramp` already generates a single-track
  switchbacking staircase with a floor rule (step feet must sit on solid ground); this
  is the seed primitive for the geometries below. `ladder_pillar` and
  `emergency_escape_shaft` in `server/src/bastion_jobs.rs` are the existing ladder path.

## Free-climb reach — the access-need threshold (Ben, 2026-07-17)

Before any built access is chosen, there is a **free-climb-out reach**: a colonist can
scramble/climb out of a pit *without* stairs or a ladder, up to a **logical, skill-scaled
depth cap**. Ben's ruling: even climbing-skill **0** gets a SHORT reach (not realistic,
but understandable in-game — a person clambers out of a shallow hole); higher climbing
skill reaches deeper; **beyond the cap, built access (ladder or stairs) is required.**

This is the floor the whole selection rule sits on: a pit/shaft **shallower than the
free-climb reach needs no built access at all**; only depths **beyond** it trigger the
stair/ladder selection below.

- **Mechanic gap — CLOSED.** Was: the engine did NOT produce this cleanly —
  `handle_climb` entry was `constructed_ladder || energy > 1.0`, energy-gated never
  skill-gated, and (FABLE-003 correction) energy alone could NOT cap depth at all since
  the colonist regen-cycles (climb→idle/fall-catch regen→re-climb), making effective
  reach unbounded for a rested colonist despite a ~5–6-block single-pass limit. **Fixed
  by `bastion-block-CLIMBCAP` (tag `7483439958`):** a skill/provenance-keyed
  climb-ENTRY gate, not an energy-pool trick — `cap_for_skill(level) = 3·(level+1)`
  (3/6/9, Ben-tunable), descent/hold never capped, a real ladder token fully exempts,
  single natural-ascent gate point (Opus R1), per-colonist `climb_anchor` (last genuine
  foothold, Opus R3). Design-against-the-cycle, exactly as this section originally
  called for. Full mechanism + seed-corpus proof + the A2/below-grade co-requisites
  that shipped alongside it: `docs/BASTION_RUN_LOG.md` §bastion-block-CLIMBCAP.
- **Why it matters for the B5.8 ladder fixture:** the fixture needs a pit *deeper than the
  free-climb reach* so the ladder is genuinely required (else a skilled colonist bypasses
  it by wall-climbing, which fails the ladder-provenance REAL-CLIMB predicate while still
  exiting = a confusing not-quite-pass). Skill-0 + deep + stair-blocked is the cleanest
  ladder-required condition under the current mechanic.

## Depth selection rule

Two axes: **depth** (how far down) and **exposure** (is the top open or must it stay
covered). This applies **only beyond the free-climb reach above** — shallower pits
self-rescue with no built access.

- **Shallow + open** (depth ≤ ~`STAIR_MAX_OPEN_DEPTH`, default ~5): open stairs A/B —
  the excavation itself is the staircase. A single-edge staircase of depth D needs
  ~D–2D blocks of horizontal run; at depth ~5 that is a 5–10 block run, fine.
- **Deep** (beyond the open band): choose enclosed staircase C or ladder shaft.
  - Enclosed staircase C — walkable, minimal excavation, but needs horizontal run for
    the slope; prefer when there is lateral room and walk-access is wanted (no climb
    skill/energy cost, no queue at a single rung column).
  - Ladder shaft — compact vertical, but requires climb (skill/energy) and is the
    route-owned path; prefer when lateral room is tight or the target is directly below.
- Both are tunable named consts, not load-bearing — document them.
- **Tie/failure fallback ordering**: open stairs → enclosed staircase → ladder shaft →
  (emergency only) the teleport backstop, which stays armed as the ultimate net. Never
  plan a style whose geometry does not fit the mask — that unfittable-plan case was the
  entire CarvedStair failure class (a planned style with no viable geometry/executor).

## Geometry A — single-edge wide staircase

The earth is mined in a staggered stepback along **one full edge** of the mine, so the
entire width of that edge becomes one wide walkable staircase rising vertically.

- For a mine footprint W wide × L long × D deep, pick one edge (the W-wide side). Mine
  a stepback: the row nearest that edge is floored at the top level, the next row one
  level down, and so on, so each depth level is a full-width step.
- Every step is the full mine width W, so colonists never queue single-file (this is
  the fix for the "they all fight to use it" single-column queue-fight that the
  B6-hotfix cited when it disabled the auto-pillar).
- Walkability: each step must satisfy the same floor rule `carve_ramp` uses — feet on
  solid, head clearance above. Reuse that predicate; do not invent a second one.
- Selection: prefer this for wider mines (W ≥ 2) and for the shallow emergency case.

## Geometry B — perimeter spiral staircase

The staircase wraps the mine perimeter, winding down around the walls — compact
footprint, works for narrower or somewhat deeper mines within the stair band.

- Steps hug the inner wall face and turn at each corner, dropping one level per side
  (or per N blocks), spiraling to the floor. This generalizes `carve_ramp`'s existing
  switchback logic to a perimeter loop.
- Center of the mine stays open (the actual dig volume); only the perimeter ring is
  the staircase.
- Selection: prefer this when the footprint is squarer/narrower, or when a single-edge
  staircase would not fit the mask but the perimeter has room.

Both geometries are **walkable — no route ownership, no traversal task.**

## Geometry C — enclosed delving staircase (traditional deep mine, no exposed top)

The classic Minecraft-style staircase mine: an entrance at the surface, then a covered
switchback staircase bored **down through solid rock** to a deep target. The top is NOT
opened — only the stair tunnel itself is excavated, so the terrain above stays intact.
This is the deep-but-walkable option; it competes with the ladder shaft, not with the
shallow open stairs.

- **Reuse — this already exists.** `common::bastion::carve_ramp` is fundamentally a
  geometry-C generator: it cuts a switchbacking staircase through solid ground, enforces
  the floor rule (each step's feet sit on solid rock), and **explicitly refuses to route
  through already-open space** (its own comment: "A stair cannot route through already-open
  space — that is the ladder's job"). That refusal is exactly the "enclosed / no exposed
  top" property. Geometry C is largely *productionizing carve_ramp as a first-class
  walkable (non-route-owned) access*, plus head-clearance so the tunnel is walkable, plus
  a surface entrance.
- **Irony worth stating**: carve_ramp's output is the very thing Stage-1 mislabeled as a
  `CarvedStair` route and broke by making it route-owned. Once it is walkable-not-route-
  owned (Phase 1), carve_ramp already *is* geometry C — the feature is mostly exposing it
  intentionally, not new pathfinding.
- **Walkability**: enforce head clearance above each step (2-high walk corridor) so the
  bored tunnel is traversable; reuse carve_ramp's floor rule for the feet.
- **Selection**: deep target, minimal excavation wanted, lateral room available, walk
  access preferred over a climb.
- **Walkable — no route ownership, no traversal task**, like A and B.

## Scope: unified access planner

Apply the depth rule in the single existing selector, `plan_access`
(`server/src/bastion_jobs.rs:546`), so **both** paths share it:

- **Player-designated mines**: when a mine zone is dug below walkable reach, the colony
  plans a stair (shallow) or ladder (deep) access into it, the same way.
- **Emergency egress** (trapped colonist / deleted-zone entombment): same selector,
  same depth rule. This is the path CK exercises.

One selector, one depth rule, two geometries + the ladder — no parallel planners.

## Outline / preview (owner requirement)

Stairs must show a **footprint outline** before/while they are dug, so the player can
see the planned staircase shape.

- Reuse the existing designation-footprint rendering (the tool that already ghosts a
  Mine/Farm/etc. zone) plus the UI-5 universal inspector wire (`BastionInspect` →
  `BastionInspectKind`, just landed) to surface the planned stair cells as a distinct
  overlay.
- Minimum: render the planned stair/step cells as a ghost overlay in a stair-distinct
  color when a stair-style access is planned, so it reads as stairs, not a raw dig.
- This is a voxygen/client surface (the `gate-must-test-live-path` lesson applies —
  prove it renders in a live client, not just a green server gate).

## Phasing

1. **Phase 1 (routed, prerequisite): CK CarvedStair fix. DONE.** Landed as a pair,
   tag `bastion-block-CKSTAIR` → `9ad9d97808` (branch `bastion/block-B6HAUL`):
   `177c12094f` (Phase 1 itself, per Ben's ruling above — emergency stair plans become
   walkable/no-ownership, plan tuple descriptor became `Option`; only ladder/shaft
   plans stay route-owned) + `9ad9d97808` (STUCKJOB (α) watchdog fix + falsifier — a
   SECOND latent Stage-1 watchdog defect found via this work: teleport-suppression
   must be EARNED by verified per-colonist `(job,progress)` baseline, not just
   claim-holding/churn; new `--stuckjob-scenario`, ladder leg 39, proven RED→GREEN
   [unfixed: never rescued in 200s vs the 60s design target; fixed: rescued at
   59.0s]). CK 5/5 PASS + recorder + full ladder 38/39 (BED = the registered
   CASE-003 `bed_occupied_mid` field-class, third identical draw). Corrects the
   misfiled B22 `ck_failsafe_out` entry (invariant, not flake). **Positive capability
   note, not just a fix:** the STUCKJOB falsifier's rev-1 produced the first
   end-to-end proof of ORGANIC stair self-rescue (plan→claim→dig→ascend→out, 26s, no
   backstop needed). Two untagged intermediate commits sit in the branch history with
   no master-list/design-doc row of their own — `42f7c464a0` (an overruled CK fix
   shape, superseded by this Phase 1) and `871a9157d9` (B5.8 Stage-1, tracked on the
   architect's own external-effort provenance lane). **Phase 2 (below) is now
   unblocked.**
2. **Phase 2a: depth/exposure selector + Geometry A** (single-edge wide staircase) in
   `plan_access`, applied to both scopes. Bounded harness scenario per geometry.
   **Before adding any new traversal phase/kind here, read
   [REQ-0052-ROUTE-SQUEEZE-DESIGN.md](REQ-0052-ROUTE-SQUEEZE-DESIGN.md)** — this
   machinery hosts the `route_squeeze_until` collision-radius mechanism, and that
   doc's open item #1 (kind-gated vs. unconditional squeeze) needs an explicit
   decision for any new write site, not a copy-nearest default.
3. **Phase 2b: Geometry C** (enclosed delving staircase). Lowest-risk of the new
   geometries — productionizes the existing `carve_ramp` primitive as a walkable,
   non-route-owned access + head-clearance + surface entrance. Good candidate to do
   before/with A since it is mostly exposing existing code.
4. **Phase 2c: Geometry B** (perimeter spiral) — the most new-code geometry.
5. **Phase 2d: outline/preview** client surface, live-client verified.

## Test plan (fleet testing format)

- Deterministic bounded harness scenarios, one per geometry, real Server ticks + real
  physics + real pathing (not a contract model): trap/designate at a known depth,
  assert the colonist **walks** out (position advances, Mine jobs completed, NEVER
  parked in `RouteOwnedWaiting`, teleport backstop NEVER fires on a healthy shallow
  escape), deterministic x2, flight-recorder evidence.
- Depth-boundary scenarios: just-shallow (stairs chosen, walk) and just-deep (ladder
  chosen, climb) around `STAIR_MAX_DEPTH`.
- Regression: CK stays 5/5 PASS; conservation `1000->1000`, board/orphans `0/0`,
  zero-teleport-on-healthy-escape all intact; full ladder green before tag.
- The Stage-1 ladder path (deep) is proven separately by Fable's bounded constructed-
  ladder integration fixture — unchanged by this work, and its pit geometry (5–8 deep,
  ≤3×3) now correctly resolves to a ladder, not a dead CarvedStair.

## Open / tuning (not load-bearing, builder or owner may set)

- Exact `STAIR_MAX_DEPTH` (default ~5) and step slope (1 level per 1 vs 2 horizontal).
- Whether emergency-carved stairs are permanent infrastructure (recommended: yes — a
  real staircase is infrastructure) or restored on cleanup, provided the conservation
  oracle stays exact either way.
- Geometry selection heuristic when both A and B fit (default: A for wide, B for
  square/narrow).
- Whether the player can force a style per zone, or the game always picks by depth
  (default: game picks; per-zone override is a later nicety).
