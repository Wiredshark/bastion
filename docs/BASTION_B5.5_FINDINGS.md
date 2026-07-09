# B5.5 findings — zone deletion + item-drop pile aggregation

Spec: `readme/B5.5-zone-delete-drop-aggregation-prompt.md`. Explored
2026-07-09 on `bastion/block-B5.5` (start `b7f01d1`).

## 1. The pebble-carpet root cause: `should_merge: false`

Veloren's pile machinery ALREADY EXISTS and is conservation-exact by
construction: `PickupItem` (`common/src/comp/inventory/item/mod.rs:512`)
holds a `Vec<Item>` (`amount()` sums across them), `try_merge` moves items
between vecs (never destroys), `Server::create_item_drop`
(`server/src/state_ext.rs`) merges new drops into nearby entities at spawn,
and `sys::item` (`server/src/sys/item.rs`) periodically merges neighbors
within `MAX_ITEM_MERGE_DIST = 2.0` with exponential back-off. **None of it
fires for B5 drops** because `PickupItem::can_merge` requires
`should_merge == true` on BOTH sides, and B5 (mirroring vanilla mining's
emit at `interaction.rs:426`) passes `false` — the flag exists to prevent
entity-DoS from inventory dumping and is only `true` for inventory-dropped
items. So the fix's heart is one flag + a persistence/class wrapper.

## 2. Persistence: colonist drops must never despawn (and never catch a timer by merging)

- `create_item_drop` unconditionally attaches `Object::DeleteAfter
  { timeout: 300 s }` — a latent item-LOSS bug for colonist-produced
  resources (any soak/pause > 5 min destroys player stone). B5.5 adds
  `persistent: bool` to `CreateItemDropEvent` (all 4 vanilla emit sites pass
  `false`; `bastion_jobs` passes `true`) and threads it through
  `StateExt::create_item_drop` (2 call sites: the event handler +
  `cmd.rs:599`). Persistent drops skip `DeleteAfter` and get a
  `comp::bastion::BastionPile` marker.
- **Merge-class separation** (the subtle loss path): merging is
  directional — the source entity is deleted and its items join the target.
  A persistent pile merging INTO a timed vanilla drop (either at spawn or
  via `sys::item`'s periodic pass, e.g. a player drops stones from
  inventory next to a colonist pile) would inherit the 300 s timer → loss.
  Fix: `get_nearby_mergeable_items` gains the `BastionPile` storage and
  only offers pairs whose marker-presence MATCHES. Persistent↔persistent
  and vanilla↔vanilla merge freely; the classes never mix. (Vanilla mining
  drops are `should_merge: false` anyway; the class gate covers the
  inventory-drop case.)

## 3. Pile visuals: tier-scaled, one entity

A merged pile is already ONE entity/physics-body/mesh — the carpet problem
is solved by merging alone. For "reads as a heap": new tiny system
`server/src/bastion_piles.rs` (registered in `server/src/sys/mod.rs` next
to `bastion_jobs`) sweeps `BastionPile` entities every 30 ticks and writes
`comp::Scale` by amount tier (1.0 / 1.35 / 1.7 for <5 / <20 / ≥20). `Scale`
is a synced comp (`synced_components.rs:36`) — the client re-renders the
item mesh larger, zero client changes. No custom heap mesh this block
(catalogued as a future asset-pipeline item).

## 4. Zone deletion: the server side already exists; the work is client/UI

- `JobBoard::cancel_region` (B4) + `ClientGeneral::BastionCancelDesignation`
  (validated at `sys/msg/in_game.rs:318`, applied post-parallel-loop at
  `:801`) already release claims within one upkeep tick — proven by
  `--b4-scenario`'s cancel step. Partial cancel is inherently supported
  (arbitrary region).
- What's missing (B4 findings called it out: "designation echo has no
  removal message"): the client overlay list only grows. B5.5 adds
  `ServerGeneral::BastionDesignationRemoved { region }` echoed from the
  cancel handler (mirrors the place echo at `:290`).
- Client (`client/src/lib.rs`): on removal, each stored rect is replaced by
  `Region::subtract(erased)` — exact 3D AABB subtraction (≤ 6 remainder
  boxes; new method + unit tests in `common/src/bastion.rs`). A revision
  counter (`bastion_designations_rev`) replaces voxygen's fragile
  index-based incremental sync (`bastion_designation_synced` +
  `bastion_designation_shapes` in `session/mod.rs:774`): on rev change the
  session drops ALL designation shapes and rebuilds from the list (dozens
  of rects — trivially cheap, and removal-correct by construction).
- **Erase tool**: `ToolMode::Erase` (`voxygen/src/bastion/tools.rs`,
  T-cycle order: Pan → Inspect → Mine/Chop/Build/Stockpile → Erase). Reuses
  the designate-paint drag verbatim (red preview instead of yellow); on
  release sends `BastionCancelDesignation` with the same region math
  (incl. the `min.z - 2` under-plane reach, kept symmetric with place so
  erasing at the paint slice hits what was painted there).
- **Delete-entire-zone**: `RadialAction::DeleteZone` — a voxygen-local
  radial action (NOT a new `ContextVerb`; it never crosses the wire as a
  verb). Offered on block targets lying inside ≥1 client-known designation
  rect; the handler sends one cancel per containing rect. "The zone" =
  the painted rect, which is exactly what the client stores.

## 5. Harness additions

- `Server::bastion_sum_items_near(pos, radius, asset) -> u64` — sums
  `PickupItem::amount()` (the entity-count hook stays for pile-count
  bounds). `bastion_pickup_entity_count()` — system-wide entity bound.
  `bastion_set_colonist_skill(name, work, level)` — lets the 200-block
  scenario run at work_rate ≈ 1 block/s instead of 3 s.
- `--b55-scenario`: Part 1 — 6×6 surface mine strip, claims up, cancel
  half → cancelled half's jobs gone + claims consistent within one
  arbitration cycle + remainder still worked; cancel rest → board empty,
  all idle. Part 2 — 20×10 surface strip (200 jobs), skill-boosted
  colonists mine it out; assert system-wide stone amount-sum == 200 EXACT
  (conservation through merges), pickup-entity count ≤ 48, soak tail
  bounded.
- `--b5-scenario` updated: stone/log assertions switch from entity counts
  to amount sums (with merging on, 27 drops may be < 27 entities; the
  conservation invariant is the amounts).

## 6. Scope notes / deferred

- Render-distance culling for piles: skipped per the prompt ("optional if
  trivial" — it isn't; aggregation is the real fix).
- Custom heap mesh + count label: future asset/B9 work (backlogged).
- Erase depth asymmetry: erasing at a different Z-slice than the original
  paint can miss the paint's `-2` under-reach; the radial Delete-zone
  covers exact cleanup. Backlogged as a UX polish item.
