# Project Bastion — restore ledger (append-only)

Rollback map for `bastion/main`. Every tag below is a fully-tested,
gate-passed block boundary — `git checkout <tag>` (or `git reset --hard
<tag>` on `bastion/main` if a later block needs to be discarded entirely)
returns the tree to that block's known-good state. Never delete or rewrite
an entry; if a block is later reverted, add a new entry noting it rather
than editing the old one.

| Tag | Represents | Rolls back past |
|---|---|---|
| `bastion-baseline` | Pre-Bastion vanilla Veloren, before any block work. | Everything. |
| `bastion-block-B1.6` | Ortho overseer camera + Z-slice + 4-mode occlusion/relight (B1 + B1.5 + B1.6 + B1.7, retro-tagged). | All camera/viewmode/input work. |
| `bastion-block-B1.7` | Same commit as B1.6 (B1.7's fixes landed inside B1.6's QA rounds, not a separate merge). | Same as B1.6. |
| `bastion-block-B2a` | Overseer interaction surface: tool palette, radial menu, designate-paint + echo overlay, selection. | All designation-UI work; colonists/jobs do not exist yet at this tag. |
| `bastion-block-B3` | Colonist entity model: `Colonist`/`PlayerColony`/`BastionGodAnchor` comps, promote/demote, §4 god-anchor invulnerability, founding + selection UI. | All job-board/work-execution work; colonists exist but are idle (vanilla civilised AI only). |
| `bastion-block-B4` | Designation → job board → autonomous arbitration + pathing. Colonists claim/travel to jobs; nothing completes work yet (`Arrived` was terminal). | All work-execution/item-drop/skill-XP work. |
| `bastion-block-B5` | Work execution: dig/chop/build terrain effects, item drops, skill XP, Build material gating. `Arrived` is now transient (jobs complete and release). Colonist opportunistic item-pickup AI gated off. | Hauling (B6) and everything after. |

## Notes for future rollback decisions

- Tags mark **merge boundaries on `bastion/main`**, not every commit on a
  block's working branch (`bastion/block-<N>`) — those branches carry the
  fine-grained history (checkpoint → build → self-test → commit-or-rollback
  per sub-step) if a *partial* revert within a block is ever needed instead
  of rolling back the whole block.
- `server/agent/src/action_nodes.rs` and `server/agent/src/data.rs` (the
  `ReadData::colonists` field + colonist item-pickup gate) are vanilla
  agent-AI files touched for the first time in B5 — rolling back past
  `bastion-block-B5` also reverts that gate, meaning any *other* future
  code that came to depend on `ReadData::colonists` existing would need to
  be rolled back too. None does yet as of B5.
- `common::bastion::BUILD_MATERIAL_ITEM` and the single-material Build
  stand-in are B5-only; B6 is expected to replace (not extend) that
  mechanism, so rolling back to B4 cleanly removes it with no dangling
  references.
- **`bastion-block-B5` was moved once**, same session it was first cut: a
  wider post-merge re-verification pass (running the gate far more than
  the original 5 times) turned up a third reachability bug (the mine
  quarry pit had no exit ramp — see `BASTION_B5_FINDINGS.md` §4b) that the
  original tag's state didn't include the fix for. Since nothing had yet
  been built on top of the original tag, the tag was force-moved to the
  commit with the fix rather than leaving a known-flaky boundary as the
  permanent rollback target — judged more honest for future rollback
  purposes than a tag whose name promises "fully-tested" but whose
  content sometimes wasn't. If a rollback to "B5 before the ramp fix" is
  ever specifically needed, it's `ec29fda` on `bastion/block-B5` (the gate
  fixes commit, pre-ramp) — not tagged, but preserved in that branch's
  history.

## B5.5 (2026-07-09)

| Field | Value |
|---|---|
| Block | B5.5 — zone deletion + item-drop pile aggregation (patch block) |
| Tag | `bastion-block-B5.5` |
| Previous green tag | `bastion-block-B5` (at `297cc0f`, post tag-move) |
| Revert command | `git reset --hard bastion-block-B5` (on `bastion/main`) |
| Reverting undoes | Erase tool + radial Delete-zone + designation-removal echo/overlay subtraction; persistent pile aggregation (colonist drops would resume carpeting one entity per block AND regain the 300 s despawn timer — i.e. reverting reintroduces a known item-LOSS hazard); the `--b55-scenario` gate; the B5 scenario's amount-sum assertions (reverts to entity counts). |
| Data-format caveats | `CreateItemDropEvent` gained `persistent: bool` (in-memory only, not serialized). `ServerGeneral::BastionDesignationRemoved` is a new net message — old client + new server (or vice versa) across this boundary would break protocol; irrelevant for the single-tree singleplayer setup. No rtsim `data.dat` changes. `comp::bastion::BastionPile` is a new server-side comp (not persisted, not synced). |

## B5.6a (2026-07-09)

| Field | Value |
|---|---|
| Block | B5.6a — zone visuals: terrain-draped outlines + visuals toggle + pile tiers (approved split of B5.6) |
| Tag | `bastion-block-B5.6a` |
| Previous green tag | `bastion-block-B5.5` (at `0de0659`) |
| Revert command | `git reset --hard bastion-block-B5.5` (on `bastion/main`) |
| Reverting undoes | Terrain-conformed overlay draping (outlines float flat again — the photographed bug returns); the H visuals-toggle (On/Subtle/Off); the erase XY-clip robustness fix (erase becomes z-fragile again — silently misses after camera moves); the 5-step pile tier curve (reverts to the B5.5 basic 3-tier scale). |
| Data-format caveats | None. Client-side (voxygen) + one additive `common::bastion::Region::clip_xy` method + a `server/src/bastion_piles.rs` scale-curve tweak. No new comps, no net-protocol change, no rtsim `data.dat` change. Fully backward-compatible. |

## B5.6b-1 (2026-07-09)

| Field | Value |
|---|---|
| Block | B5.6b-1 — zone fills + kind colors + overlap blend + labels + SUBTLE (first B5.6b sub-block) |
| Tag | `bastion-block-B5.6b-1` |
| Previous green tag | `bastion-block-B5.6a` |
| Revert command | `git reset --hard bastion-block-B5.6a` (on `bastion/main`) |
| Reverting undoes | Terrain-conformed zone fills (`DebugShape::ConformedTris` + `bastion::draped_fill_tris`), the kind-color legend + overlap blending, world-anchored zone labels, SUBTLE=border-only; ALSO the three demo-bug fixes (canopy-safe overlay heights, input-transparent labels + XY zone matching, terrain-anchored grab plane) — reverting reintroduces tree-climbing overlays, dead Delete-zone near centroids, and off-center pan. |
| Data-format caveats | None. Client-side (voxygen) + one additive `common::bastion::Region::contains_point_xy`. No net-protocol, comp, or rtsim changes. |

## bastion-block-BMAP1 (2026-07-09)

- Block: B-MAP1 (overseer minimap + world-map overseer layers)
- Tag: `bastion-block-BMAP1` · merge `e0300e253b`
- Previous green: `bastion-block-B5.6b-1` (main then advanced by docs
  commits to `c8643b72b2`, the merge base)
- Revert: `git reset --hard c8643b72b2` (or the b-1 tag to also drop the
  docs commits)
- Undoes: the bastion minimap + big-map overseer layers/fly-to + the
  minimap size button (client-only; no data-format or protocol changes —
  nothing serialized, safe to revert cold).

## bastion-block-SCCACHE (2026-07-10)

- Block: INFRA P5 — sccache shared compile cache (`rustc-wrapper` in
  `.cargo/config.toml`). Not a game block; a build-speed infra commit.
- Tag: `bastion-block-SCCACHE` (main `13f7d1f503`)
- Revert: `git revert 13f7d1f503` (or delete the `[build] rustc-wrapper`
  lines) — nothing else depends on it; sccache itself stays installed
  user-side harmlessly.
- Data-format caveats: NONE. Config-only. Reverting just stops routing
  compiles through the wrapper.

## bastion-block-B6 (2026-07-10) — SOFT-0 + Ben's live-fix batch (folded)

- Block: B6 SOFT-0 soft-collision + the whole Ben live-fix batch
  (B-LIVE1 flat-mine drag, B-LIVE2 day-speed, B-LIVE3 mine lifecycle +
  tiered fail-safes) folded together — the chokepoint red closed +
  every Ben-reported live bug in one merge.
- Tag: `bastion-block-B6` (see run-log for merge SHA)
- Previous green: `bastion-block-SCCACHE` (main `13f7d1f503`); the branch
  also forward-merged BASSET1 + sccache.
- Revert: `git reset --hard 13f7d1f503` (on `bastion/main`) — but note
  this drops Ben's live-fix batch AND re-opens the chokepoint red AND
  the flat-mine drag reject; prefer a targeted revert of a single sub-fix
  if only one regresses.
- Undoes (server): `Colonist.{soft_until, climb_free_until}` +
  `ActiveJob.{reset_dist, soft_granted}` + `ActiveJobState::Waiting`;
  the phys softened-push gate + `Time` in PhysicsRead; the watchdog
  grace + density trigger; the stuck-time hysteresis; the Waiting queue
  state; the universal verdict-independent stuck-teleport +
  `surface_teleport_dest`; `climb_free` any-wall lift; mine-done
  detection (`done_count`) + disperse; the churn/egress nets' B6
  refinements; the flat-mine server fallback; harness hooks
  (`bastion_done_designations`, `bastion_colonist_states_full`,
  `bastion_register_access_anchor`, `bastion_equip_tool` [TOOL0]).
- Undoes (voxygen): the flat-mine client floor-from-surface derivation;
  the overseer `day_length`=10 min via the `run()` param.
- Data-format caveats: **rtsim SAVE, serde-default** — `BastionColonist`
  gained `soft_until` + `climb_free_until` (both `#[serde(default)]`,
  transient runtime state, old saves load forward as 0.0). `ActiveJob`'s
  new fields are server-runtime only (never serialized to disk).
  `ActiveJobState::Waiting` is a new enum variant — a REVERTED build
  reading a live save mid-Waiting would fail to deserialize the variant,
  but ActiveJob isn't persisted (server-runtime), so no disk risk. No
  wire-protocol change. Safe cold revert.

## bastion-block-TOOL0 (2026-07-10, overnight run)

- Block: TOOL-0 (tool_factor work speed) + B5.8-E3 stability cluster
  (churn trapped-detector, egress annulus off-by-one fix, access
  nearest-first, measurement honesty)
- Tag: `bastion-block-TOOL0`
- Previous green: `bastion-block-TIMECTL` (main tip `7effa936b6` = its
  docs rider; the merge base)
- Revert: `git reset --hard 7effa936b6` (or the TIMECTL tag to also
  drop the docs rider)
- Undoes: `common::bastion::tool_factor` + its work-tick multiply (dig
  speed decouples from tools again — flat 6s base returns);
  `JobBoard.{churn_watch, egress_pending}` + the churn trapped-detector;
  the egress annulus rise fix (REVERTING RESURRECTS the b5-chop pit
  entrapment bug — a reach-2 novice trapped in a 3-rise pit gets no
  egress); access nearest-first claim scoring; the E2 `Job.last_bounce`
  bar (added AND removed within this block-pair — net zero);
  is_access-filtered harness hooks; hooks bastion_equip_tool /
  bastion_colonist_tool_factor; b5 phase 7.7 + chop pad, b4/b58
  scenario reshapes.
- Data-format caveats: NONE — `Job` lost no shipped field (last_bounce
  never reached main), the board is runtime-only, no wire or save
  changes. Safe cold revert; note the annulus regression above before
  choosing to.

## bastion-block-TIMECTL (2026-07-10, overnight run)

- Block: TIME-CONTROLS (UI-3 §3 visible sim-speed cluster + hotkeys)
- Tag: `bastion-block-TIMECTL`
- Previous green: `bastion-block-B5.6b-2.1` (main tip `547ee38518` = its
  docs rider; the merge base)
- Revert: `git reset --hard 547ee38518` (or the b-2.1 tag to also drop
  the docs rider)
- Undoes: the HUD speed cluster/readout/PAUSED tag, the three
  `GameInput::Bastion{PauseToggle,SpeedUp,SpeedDown}` bindings
  (Space/+/−) + context-scheme entries, `Event::BastionSetSimSpeed`, the
  session sim-speed setter/stepper. Voxygen-only.
- Data-format caveats: NONE on the wire or saves. One SETTINGS surface:
  the three new GameInputs get default key bindings — a `settings.ron`
  saved after this block lists them; reverting past it leaves unknown-
  input entries that vanilla settings loading tolerates/drops. Safe cold
  revert.

## bastion-block-B5.6b-2.1 (2026-07-10, overnight run)

- Block: B5.6b-2.1 (ABSOLUTE-FLOOR flat mine mode + B5.8-E anti-stuck
  cluster + B5.8-E2 employed-loop fix + pace tune 3→6s)
- Tag: `bastion-block-B5.6b-2.1`
- Previous green: `bastion-block-B5.8` (merge `6c17845e92` — the direct
  merge base; no intervening checkpoints)
- Revert: `git reset --hard bastion-block-B5.8` (on `bastion/main`)
- Undoes: `ZExtent.floor_z` flat-floor mode + `column_range()` as the one
  dig-range authority; the client Slope/Flat stepper toggle + paint-time
  floor derivation; `Job.{depth, stuck_strikes, last_bounce}`; the
  ACCESS-BEFORE-DESCENT gate + proactive descent plan; EMERGENCY EGRESS
  (`egress_watch` + humanitarian bubble + ARRIVED-only reset); the
  unreachable-bounce claim bar; strike-grown arrival tolerance; b5 phase
  7.6 flat asserts + b58 parts (e)/(f) + the (d) epilogue cleanup;
  `WORK_DURATION_BASE` 6.0 (reverts to 3.0 — Ben wanted the slowdown;
  re-apply on any rollback or mining goes "instant" again).
- Data-format caveats: **WIRE-TOUCHING, serde-default** —
  `ZExtent.floor_z: Option<i32>` is `#[serde(default)]` on the
  already-shipped wire struct: a mismatched client/server pair
  deserializes the field as `None` (silent relative-mode fallback), not a
  hard error — still ship client+server together (a stale pair mis-reads
  flat-mode paint intent). `Job`'s three new serde-default fields are
  runtime-only (the board never crosses the wire). No comp/rtsim
  `data.dat` changes. Safe cold revert as a pair.

## bastion-block-B5.8 (2026-07-10)

- Block: B5.8 (vertical mobility: scramble + climbing SKILL + autonomous
  access stairs/ladders + DF-style mining + ladder collision waiver)
- Tag: `bastion-block-B5.8`
- Previous green: `bastion-block-B5.6b-2` (= merge base `efc777475a`)
- Revert: `git reset --hard bastion-block-B5.6b-2` (on `bastion/main`)
- Undoes: `scramble_reach` + 3-up SCRAMBLES + ladder edges in the path
  graph (+ the 3 graph unit tests); `ColonistSkills.climbing` (movement
  skill, XP-on-use); `DesignationKind::Ladder` (wire enum APPEND) +
  ladder tool/color; the shared masked-switchback `carve_ramp`; the
  autonomous-access machinery (claim mask, access anchors, stairs-vs-
  ladder geometry choice, one-plan-at-a-time, material-free access jobs);
  exposure-gated top-down dispersed mining arbitration; the position-
  driven climb assist + ledge snap; the phys LADDER COLLISION WAIVER;
  mid-travel moot check; staged routing; the B5 quarry scenario's ramp
  removal + `--b58-scenario` + hooks (teleport/climbing/sprite/claims).
- Data-format caveats: **NET PROTOCOL CHANGE** — `DesignationKind` gained
  the `Ladder` variant (appended last; client+server revert together).
  **RTSIM SAVE CHANGE** — `ColonistSkills.climbing` is `#[serde(default)]`
  (old saves load forward; a REVERTED build reading a post-B5.8 save
  drops the field silently — acceptable). `Job.carve_attempted/is_access`
  are runtime-only. The phys waiver reads a synced comp — no format
  impact.

## bastion-block-B5.6b-2 (2026-07-09)

- Block: B5.6b-2 (z_extent surface-relative model + volumetric zones +
  volume-selection UX; closes B5.MINE-COVERAGE; canonical Purpose enum)
- Tag: `bastion-block-B5.6b-2`
- Previous green: `bastion-block-BMAP1` (main then advanced by the
  architect's docs-only checkpoint `72907ee641` = `fleet-ckpt-01`, the
  merge base)
- Revert: `git reset --hard 72907ee641` (or `bastion-block-BMAP1` to also
  drop the architect checkpoint)
- Undoes: `ZExtent` + `Purpose` in `common::bastion`; the surface-relative
  placement path (`column_surface_z`/`place_designation_surface`/
  `resolve_surface_bounds` + harness hooks); the `z_extent` field on
  `BastionPlaceDesignation`/`BastionDesignation`; the client paint's
  footprint+extent send (flat `min.z-2` pre-expansion returns, and with it
  the MINE-COVERAGE slope gap); scroll/stepper depth UX + ring previews +
  volumetric committed rendering; the b5 scenario's slope-coverage phase.
- Data-format caveats: **NET PROTOCOL CHANGE** — two messages gained an
  `Option<ZExtent>` field (client+server must match; there is no version
  negotiation). No comp/rtsim `data.dat` changes; the job board is
  runtime-only. Safe to revert cold as long as client+server revert
  together.
