# Project Bastion — doc/code consistency audit (append-only)

Contradictions between the design doc / prior findings docs and what's
actually implemented, found while building each block. Not all of these are
bugs — some are intentional simplifications for the block's scope — but
each is a place a future reader could reasonably expect different behavior
than what's there. Append new findings; never edit or remove an old entry
even if a later block resolves it (note the resolution as a *new* entry
instead, referencing the old one).

## B5 — Work execution

- **Build material sourcing**: `readme/veloren-colony-rts-build-report.md`
  §B5 (line 647) says "a blueprint voxel is a ghost until a colonist hauls
  the required material (**ties into B6**) and completes construction" —
  i.e. the design doc assumes B6's hauling system exists before Build jobs
  can ever complete. B5 as actually shipped does **not** wait for B6: it
  uses a single-material stand-in (`common::bastion::BUILD_MATERIAL_ITEM`,
  a colonist must simply be *carrying* one unit in `Inventory`, sourced in
  the harness via a direct `bastion_give_colonist_item` injection, not any
  in-game hauling action) so the Done-when criterion ("a wall blueprint
  with materials present gets constructed... without materials, it stalls
  ...and raises a needs-materials job") could be verified without B6
  existing yet. This is a deliberate, scope-correct simplification — B5's
  design-doc section itself says "Touches: ... item spawning" not "requires
  B6" in its own Touches/Approach breakdown, only the one aside does — but
  it means anyone reading only the design doc's B5 section would expect
  Build to be unreachable/untestable until B6 lands, which is not the case.
  See `readme/BASTION_BACKLOG.md`'s B5 section for the plan to replace the
  stand-in with real hauling once B6 exists.
- **Build "ghost" visual**: the same design-doc passage describes a
  blueprint voxel as "a ghost until... construction, replacing ghost with
  real voxel" — implying a distinct, player-visible pending-construction
  render state. No such visual exists: a Build designation's target
  position renders as whatever it already was (typically empty air) right
  up until the job completes, at which point it snaps directly to the
  final solid block. There is no in-between "ghost/blueprint outline"
  state. Not covered by B5's own Done-when criteria (which are all
  headless/mechanical: "gets constructed," "stalls... and raises a
  needs-materials job" — no visual requirement), so this isn't a B5 gate
  failure, but it's a real gap between the design doc's description and
  what a player would see. Likely belongs to whichever block does
  designation-overlay polish (B9, per B4's findings doc §5 "per-job status
  render is B9's").
- **Chop = "fells a tree"**: design doc §B5 Done-when says "Chop
  designation fells a tree and yields logs," read most naturally as one
  designation → one whole-tree action. The actual implementation (carried
  from B4, unchanged in B5) generates one Chop job **per `BlockKind::Wood`
  voxel** in the painted region — for a real multi-block tree trunk this
  produces many jobs, several of which are unreachable by a ground-walking
  colonist with no climb/ramp model (see `readme/BASTION_BACKLOG.md`'s B5
  "ADD" entry on the base-interaction verb this actually needs). B4's own
  code comments and B5's harness test comments already flag this as a
  known, out-of-scope gap — this entry exists so it's also visible from
  the doc-consistency side, not just buried in test-geometry comments.

## B5.5 — Zone deletion + pile aggregation (2026-07-09)

- **Block prompt vs. Veloren reality (resolved in Bastion's favor, worth
  recording)**: the B5.5 prompt describes building "a single pile entity
  carrying a count" as if new machinery — Veloren's `PickupItem` already
  IS that (a `Vec<Item>` with exact `amount()` accounting and
  conservation-exact `try_merge`); it simply never fired for B5 drops
  because `should_merge` was `false` (an anti-DoS flag documented as
  "currently only used for inventory dropped items"). B5.5 is therefore a
  flag + persistence-class wrapper, not a new pile system. No doc change
  needed; noted so future readers don't go looking for a bespoke pile
  entity type.
- **Design doc §B5 "stone, wood, ore per block type — Veloren already maps
  block/sprite → loot"**: still unimplemented (B5 ships flat
  stones-for-any-block, logs-for-wood; see the B5 consistency entry).
  B5.5's aggregation is loot-type-agnostic, so the eventual per-block loot
  mapping slots in without touching pile code. Drift unchanged, flagged
  again only because B5.5 touched the drop path.
- **Prompt's "reuse an existing crate/heap-like object model if one
  fits"**: surveyed `comp::body::item::Body` — nothing heap-like exists
  (item bodies mirror item kinds). Tier-scaling the item mesh
  (`comp::Scale`) is the shipped stand-in; real heap meshes belong to the
  asset pipeline (flagged in the backlog, and exactly the kind of gap the
  concurrent asset-lab session's tooling is meant to fill — coordination
  point for the architect).
