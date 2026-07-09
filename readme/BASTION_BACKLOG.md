# Project Bastion — backlog (append-only)

FIX = a known bug/gap in something already built. ADD = a missing capability
a future block should own. IDEA = a design thought not yet triaged into a
block. Newest entries go at the bottom of each block's section; never edit or
remove an earlier block's entries.

## B5 — Work execution (dig/chop/build effects, item drops, skill XP)

- **FIX** (confirmed, worked around in-test, not fixed at the mechanism
  level): any vertically-stacked designation of height >= 2 has its *lower*
  block's arrival target (`block_pos + (0,0,1)`) land exactly on the block
  above it. On flat ground with no adjacent same-height terrain, that
  target is only reachable once the block above is cleared *and* even then
  requires standing on top of a lone 1-block pillar — which the vanilla
  ground-walking traversal doesn't reliably solve (confirmed empirically:
  claim → 10s stuck → release → retry-sweep reclaim, looping indefinitely,
  never resolving even after the upper block was cleared). This is the same
  underlying gap already flagged for tall trees (see below), just
  triggering one layer sooner than assumed. The B5 harness's chop test was
  changed from a 2-tall stump to a single block to sidestep it — the
  underlying job-placement/arrival-target model is unchanged. A real fix
  needs either a smarter per-job arrival target (e.g. "any exposed face,
  not always +1 z") or a base-interaction verb (see next item) that
  sidesteps per-voxel jobs for multi-block structures entirely.
- **ADD** (carried over from B4's own comment, now empirically reinforced
  by the above): chopping a real multi-block-tall tree needs a
  base-interaction verb ("fell the tree" from ground level, one job) rather
  than per-voxel Chop jobs — the per-voxel model cannot express "the top of
  a tall trunk is reachable" without solving general freestanding-structure
  climbing, which is out of scope. Whichever block adds tree-chopping to
  the real game (not just the harness's short test stump) should own this.
- **ADD**: B6 hauling should supersede B5's `BUILD_MATERIAL_ITEM`
  single-material stand-in (`common::bastion::BUILD_MATERIAL_ITEM`,
  `server/src/bastion_jobs.rs`'s `carries_material`/`needs_materials`
  logic) with real per-blueprint recipes and colonists that actually haul
  materials to a site rather than a harness-injected starting inventory
  item. The `needs_materials` flag and stall-not-build-for-free semantics
  should carry forward as-is; only the "how does a colonist get the item"
  path changes.
- **IDEA**: colonists' vanilla opportunistic-loot AI (any Humanoid NPC
  auto-targets and picks up nearby item drops — `server/agent/src/
  action_nodes.rs`'s `is_valid_target`) is now gated off entirely for
  `Colonist` entities (see the B5 fix below) so B5's mined/chopped drops
  stay on the ground. When B6 hauling exists, decide whether it should
  reuse this vanilla pickup path (re-enabling it selectively, e.g. only
  when a colonist has a `Haul` job targeting that specific item) or
  implement its own deliberate pickup, bypassing vanilla AI entirely like
  B5's Build-material consumption already does. Leaning toward the latter
  for determinism/control, but worth a real design pass rather than
  defaulting.
- **FIX** (found and fixed in this block, noted here so the mechanism is
  documented): the vanilla NPC "wander over and pick up any nearby loose
  item" behavior (`is_valid_target`'s `Body::Item` branch) was silently
  consuming every stone/log drop within the same tick or two it appeared —
  colonists standing right next to their own work would auto-loot it into
  their inventory before anything else could observe it as a ground drop.
  Root-caused via a debug hook logging every `handle_create_item_drop` call
  (confirmed `created=true` for all 27 stone drops) cross-referenced
  against a per-tick dropped-item census (confirmed the count repeatedly
  fell back to exactly 0 well before all mining completed, then rose again
  as new drops appeared — the signature of continuous auto-pickup, not a
  merge or despawn-timeout artifact, both of which were investigated and
  ruled out first). Fixed by gating item-drop targeting off entirely for
  `comp::Colonist` entities (new `ReadData::colonists` field in
  `server/agent/src/data.rs`, checked in `action_nodes.rs`). This is a
  vanilla-adjacent file change but strictly additive/gated — no behavior
  changes for any non-colonist NPC.
- **FIX** (found and fixed in this block): B4's own `--b4-scenario` broke
  as a side effect of B5's already-committed work-execution changes to the
  *shared* `bastion_jobs.rs` upkeep loop — not from anything in this
  session's colonist-loot fix. Root cause: B4's test sampled "how many
  enabled colonists are simultaneously `Arrived` right now" and broke early
  once that hit 4, which was reliable when `Arrived` was a terminal state
  (B4 never completed jobs). B5 makes `Arrived` transient (a job completes
  after a few seconds of work and releases the colonist), so the
  instantaneous count could undercount even though every colonist
  successfully arrived at some point; separately, B5's fast job completion
  meant the deliberately-unreachable "deep" test job wasn't picked up
  until well after the sampling loop's old early-exit condition had
  already fired, so its 10s watchdog never got a fair chance to run before
  the test snapshotted the audit. Fixed by tracking *ever* arrived (a
  `HashSet` of colonist names, accumulated across the whole fixed 60s
  window) and *ever* unreachable (an OR-accumulated flag) instead of
  point-in-time snapshots, and loosening the arrived threshold from
  `>= 4` to `>= 3` of 4 enabled colonists (which of the 4 gets stuck
  babysitting the one shared unreachable job, if any, is a timing
  coin-flip — requiring literally all 4 to *also* land a real arrival in
  the same fixed window is more brittle than the invariant needs).
  Verified 5/5 clean on both `--b4-scenario` and `--b5-scenario` after the
  fix (was previously failing 3/3 and flaking ~30-65% respectively before
  the two separate root causes above were both addressed).
