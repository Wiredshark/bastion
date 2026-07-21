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

## bastion-block-AR2 (2026-07-10) — access-reliability hardening (first batch)

- Block: AR-2 = grace density-gate (reviewer R1/P4) + reviewer F6
  (universal-teleport designation-mask scope hole). Both server-side
  `bastion_jobs` refinements on the B6 fail-safe cluster.
- Tag: `bastion-block-AR2` (main `abb527dae1`)
- Previous green: `bastion-block-B6` (main `6bd1c91a60` / the docs
  tag-line `2e72df4338`)
- Revert: `git reset --hard 2e72df4338` — pure refinement of B6's
  mechanism; reverting restores B6's grace (fires on terrain stalls
  too) and re-opens the F6 jobless-trapped-inside-designation hole.
- Data-format caveats: NONE (runtime-only logic; no field/wire/save
  changes). Safe cold revert.

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

- Block: LADDEROFF (B6-hotfix, Ben live-test bundle): auto-ladder disable +
  Erase-deletes-ladders + crest-dismount snap + mine-oscillation telemetry +
  descent-gate release (D16) + harness-compiles-at-tag integrity fix
- Tag: `bastion-block-LADDEROFF` (main `fcfee0c602`)
- Previous green: `bastion-block-AR2` (main was at `c2acf8ba01` = AR-2 +
  its restore-ledger commit, the LADDEROFF merge base)
- Revert: `git reset --hard c2acf8ba01` on `bastion/main`
- Undoes: `const AUTO_LADDER_ACCESS` + `plan_access` `None => None`; the Erase
  ladder deletion (`JobBoard::drop_access_anchors_in` + the `in_game.rs`
  `SpriteKind::Ladder` region scan); the crest-dismount snap loop
  (`bastion_jobs.rs`); the `total_claims` counter + `bastion_total_claims`
  hook + b58 claims/`d_deep_unlocked`/`d_blocks_dug` telemetry; the
  descent-gate release; and RE-COMMITS `bastion_rename_colonists_unique`
  (it was uncommitted before this tag — reverting PAST this tag returns the
  harness to the won't-compile-at-tag drift, so re-add the method if you need
  to run the harness at an earlier tag).
- Data-format caveats: **NONE** — no net-protocol, comp, or rtsim/save
  changes. All changes are server job-logic + harness + one voxygen tooltip
  string + one readme registry class (D16). Fully reversible cold. In-code
  half-revert: flip `AUTO_LADDER_ACCESS` back to `true` to restore BOTH
  auto-ladders AND the old gated-descent, no git revert needed.

- Block: SLOPE (BUILD 2, slope-mining pair): flatten-hill (true-crest flat-floor
  surface) + B15 standability (claimability gated on a standable stance)
- Tag: `bastion-block-SLOPE` (main `a92afeae18`)
- Previous green: `bastion-block-LADDEROFF` (main was at `f4f1f0b972` = LADDEROFF
  + its docs commit, the SLOPE merge base)
- Revert: `git reset --hard f4f1f0b972` on `bastion/main`
- Undoes: `column_flat_surface_z` / `resolve_column_surface` / `FLAT_SURFACE_SCAN_MAX`
  + the flat-mode surface swap in `place_designation_surface` /
  `resolve_surface_bounds` + the `max_crest_for` volume gate (`in_game.rs`); the
  `ActiveJob.stance` field, `has_standable_stance`, the `standable` claim gate,
  and the stance-based arrive-target; b5 phases 7.8 (hill) + 7.9 (B15).
- Data-format caveats: `comp::bastion::ActiveJob` gained a `stance: Vec3<i32>`
  field, but it is SERVER-ONLY (`DenseVecStorage`, never NetSync'd or persisted)
  — NO wire/save impact. No net-protocol, comp-sync, or rtsim/save changes.
  Fully reversible cold. The flatten-hill change is server job-logic + one
  in_game.rs volume estimate + harness; no client changes.

- Block: CAVEIN (CAVE-IN v1 FR11 + B16 clock crash-fix + R7 rust-lld):
  mining-remnant collapse (bounded support check at mine-completion, floating
  chunks fall to resource) + eject-and-injure (crush victims shoved to safety,
  hurt, never buried) + the alt-tab dt-panic fix + the linker flip
- Tag: `bastion-block-CAVEIN` (main `437577ed25`)
- Previous green: `bastion-block-SLOPE` (main was at `e85ad68990` = SLOPE +
  its docs commit, the CAVEIN merge base)
- Revert: `git reset --hard e85ad68990` on `bastion/main`
- Undoes: `floating_chunk`/`eject_dest`/`CAVEIN_*` + the mine-completion
  collapse + the post-loop eject-and-injure (Health/Mood joined the bastion
  system's SystemData) + hooks `bastion_force_collapse_check`/
  `bastion_colonist_health`/`bastion_colonist_mood` + `--cavein-scenario`;
  the B16 `clock.rs` lower clamp (REVERTING THIS RESTORES BEN'S ALT-TAB HARD
  CRASH — cherry-pick `61aeec7cf9`+the `341e260f67` refine if you revert the
  block but want the crash fix); the `.cargo/config.toml` rust-lld block
  (reverting changes rustflags → another full-rebuild cache bust).
- Data-format caveats: **NONE** — no net-protocol, comp-sync, or rtsim/save
  changes (Health/Mood are existing comps; the system only gained write access
  to them). `clock.rs` is client+server shared but pure-runtime (no wire).
  Fully reversible cold, with the two riders above called out.

- Block: NIGHTHORROR (NIGHT_HORROR FR14 + the ARCH-001 /aura guard): the
  creature-integration pipeline's reference instance — night_horror registered
  end-to-end (species 35, wendigo-frame model/manifests/offsets, Beast Claws
  hostile, /spawn-testable)
- Tag: `bastion-block-NIGHTHORROR` (main `e1a6d2ba27`)
- Previous green: `bastion-block-CAVEIN` (main was at `d76147aa91` = CAVEIN +
  its docs commit; the NIGHTHORROR branch base is `1d4d48ddd0`, code-identical)
- Revert: `git reset --hard d76147aa91` on `bastion/main`
- Undoes: the Species::NightHorror registration (enum 35 + every touch-point:
  body dims/health/mount, rtsim wild map, npc_names keyword, voice, loadout,
  anim offset arms, both figure manifests, i18n) + the 11 model parts + the
  wild-entity/loot `.ron`s; ALSO the ARCH-001 `/aura` parse guard
  (`a456846f4c` — cherry-pick it if reverting the block; it is vanilla-safe
  and reviewer-approved standalone).
- Data-format caveats: `Species::NightHorror = 35` is a NEW wire/save enum
  variant — appended at the END so existing ids never shift; old
  clients/saves simply never reference 35 (additive, safe). No other
  net/comp/save changes. New asset files are additive. Fully reversible cold
  (a save containing a spawned night_horror would lose that entity on revert
  — acceptable; none exist pre-tag).

- Block: CHOP (FR10 redesign): whole-tree felling — Area2D Chop paint +
  World-oracle tree detection (shared handler/harness fn) + per-tree echo
  boxes + Leaves-clear-no-drop
- Tag: `bastion-block-CHOP` (main `05c016dbfa`)
- Previous green: `bastion-block-NIGHTHORROR` (main was at `3a6fd2919e` =
  NIGHTHORROR + its docs commit; the CHOP branch base is `6f4caddc6f`,
  code-identical)
- Revert: `git reset --hard 3a6fd2919e` on `bastion/main`
- Undoes: `FootprintMode` + `footprint_mode()` (common); `bastion_chop.rs`
  (shared detection) + the handler Area2D arm + the 4-tuple deferred op +
  `place_chop_cells` + `tree_fell_set`/caps + the `bastion_place_chop_area`
  hook; the Wood|Leaves `job_wanted`/`still_valid`/drop-branch semantics
  (Chop reverts to Wood-only slabs); the client stepper-hide + Area2D paint;
  b5 phase 7.10 + the `tree_fell_set` unit test.
- Data-format caveats: **NO message-SCHEMA change** (the per-tree echo reuses
  `BastionDesignation` verbatim — one message per tree). BEHAVIORAL wire note:
  a new client sends `z_extent: None` for Chop paints; an OLD server would
  treat that as a legacy region Chop (harmless — Wood-only slab). Client+
  server ship together as always. No comp/save changes. Fully reversible cold.

- Block: COORD (COORDINATION-stigmergic-v1, FR13-REV): the decaying saturation
  field — emergent crew division (deposit-while-working, decay, claim-time
  gradient read) + the coordination flow-bark
- Tag: `bastion-block-COORD` (main `e3b792fc44`)
- Previous green: `bastion-block-CHOP` (main was at `f96ada1f69` = CHOP + its
  docs commit; the COORD branch base is `5deade74c8`, code-identical)
- Revert: `git reset --hard f96ada1f69` on `bastion/main`
- Undoes: the `saturation`/`last_bark` board fields + COORD_* constants +
  `coord_cell` + the decay/deposit pass + the `sat_penalty` scoring term + the
  flow-bark (ChatEvent emitter in the bastion_jobs SystemData) +
  `saturation_at`/`bastion_saturation_at` + `--coord-scenario`. Allocation
  reverts to distance/top-down/clump-only (the mad-scramble returns).
- Data-format caveats: **NONE** — board-resident runtime state only; the bark
  rides the existing chat pipeline. No net/comp/save changes. Fully
  reversible cold.

- Block: DETRNG (B8 root fix): deterministic harness rng (`rtsim::tick_rng` +
  `DETERMINISTIC_RTSIM` flag; the 3 rtsim rule sites + the bastion_jobs
  drop-toss) + the cave-in conservation belt (`cavein_drop_cells` +
  bounded stone asserts) + b5 window headroom
- Tag: `bastion-block-DETRNG` (main `0ce3517b71`)
- Previous green: `bastion-block-COORD` (main was at `17e0fbf91e`)
- Revert: `git reset --hard 17e0fbf91e` on `bastion/main`
- Undoes: rtsim tick_rng/static + the 4 rng-site conversions (rules revert to
  OS entropy — the same-seed flake class RETURNS), the harness flag-set + the
  rtsim dep, the conservation counter/hook/asserts, the wider b5 window.
- Data-format caveats: **NONE** — runtime-only. The LIVE GAME's rtsim rng is
  UNCHANGED by default (the flag is harness-set only); the one live-game
  behavior delta is the bastion drop-toss becoming tick-seeded (cosmetic
  scatter, identical feel). No net/comp/save changes. Fully reversible cold.

| bastion-block-CASE003 | ecc069fd18 | CASE-003 wedge: fail-safe teleport standability + phys center-safety-net (shared eject_dest -> common) | revert tag bastion-block-DETRNG (0ce3517b71) | no save/wire change (behavior-only; CENTER_NET_FIRES is process-global telemetry) |

| bastion-block-EMBED-WATCH | bf858917ea | belt v2 (persistence embed watch) + Build occupancy guard + staged_at_anchor + locomotion counters; FR15 core reverted | revert tag bastion-block-CASE003 (ecc069fd18) | no save/wire change |

| bastion-block-LOD0 | bce7ecfc68 | LOD-0 save-back: per-tick record mirror + demote flush + wholesale bag restore (Option semantics) | revert tag bastion-block-EMBED-WATCH (bf858917ea) | BastionColonist.inventory serde-default None (old saves fine); rtsim data version unchanged |

| bastion-block-LOD1 | 51150baca3 | LOD-1 Loaded-gate on bastion_jobs (tier dupe guard) + --lod1-scenario gate leg | revert tag bastion-block-LOD0 (bce7ecfc68) | no save/wire change |

| bastion-block-B6HAUL | 03b649c451 | B6-HAUL+JOB-CORE: JobKind (Designated+Haul) + ReservationTable + Stockpile activation + auto-haul + Build-fetch, --b6haul-scenario gate leg | revert tag bastion-block-BELT-EXERCISE-TEST (a3ee084346) | `Job.kind` type change `DesignationKind`→`JobKind` (Designated variant wraps the old value, byte-identical); `Job.reservation: Option<ReservationId>` serde-default (pre-B6 saves have none) |

| bastion-block-BAG5CORE | 5fc29a4101 | B-AG5-CORE: six action verbs extracted into `bastion_actions` (approach/work/complete/drop/pickup/deposit), `bastion_jobs::Sys` calls them, byte-identical behavior spot-checked | revert tag bastion-block-B6HAUL (03b649c451) | no save/wire change (server-internal refactor only) |

| bastion-block-BAG1 | 4522857fd4 | B-AG1: Gather-stub degrade fix (`action_nodes.rs::idle()`) + promoted-townsfolk verification scenario; no dedicated gate leg (covered retroactively by ZONE-0's full-ladder run) | revert tag bastion-block-BAG5CORE (5fc29a4101) | no save/wire change |

| bastion-block-CASE004-MAGNET | bb858c1cf9 | 31.1: ladder-magnet else-nudge + on_pillar snap gated on own-z 2-high headroom (skip-not-relocate, B19 close), --magnet-scenario gate leg | revert tag bastion-block-BAG1 (4522857fd4) | no save/wire change |

| bastion-block-ZONE0 | 7b6d7ee08c | ZONE-0: `ZoneKind` schema + `DesignationKind::Zone` + the activity-zone soft magnet (mechanism `518ac9c46c`, Opus R12 green-light) + gate close-out aligning `--zone-scenario` with the accepted ruling | revert tag bastion-block-CASE004-MAGNET (bb858c1cf9) | `DesignationKind` gains the `Zone(ZoneKind)` variant, appended wire-stable |

| bastion-block-BELT-EXERCISE-TEST | a3ee084346 | 31.3: belt persist→relocate exercised (failing-capable harness leg) | revert tag bastion-block-LOD1 (51150baca3) | harness-only |

| bastion-block-GATHER | 8ce1b77821 | GATHER: `DesignationKind::Gather` (Area2D) + `JobKind::Gather`/`JobKind::DepositRun`, forage via `ControlAction::Collect` + one end-of-forage deposit trip via `deposit_all_of`, --gather-scenario gate leg | revert tag bastion-block-ZONE0 (7b6d7ee08c) | `DesignationKind` gains `Gather`, `JobKind` gains `Gather`/`DepositRun` — all appended, wire-stable |

| bastion-block-HIST0 | 410460f875 | HIST-0: the Chronicle store (`rtsim/src/data/chronicle.rs`) + the locked 54-kind schema + the ONE `record()` capture API, banded caps + Legendary immortality, --chronicle-scenario gate leg | revert tag bastion-block-GATHER (8ce1b77821) | rtsim `Data` gains `#[serde(default)] chronicle: Chronicle` (sibling pattern, no version bump) |

| bastion-block-BAG2 | 0093d4b7e8 | B-AG2: archetype-keyed RON decision data (`rtsim/src/rule/npc_ai/archetype.rs` + `assets/common/rtsim/archetypes.ron`), one shared `archetype_gate()` replaces 3 hardcoded profession/rng gates, --archetype-scenario gate leg | revert tag bastion-block-HIST0 (410460f875) | new RON asset `common.rtsim.archetypes`; no rtsim `Data`/save-format change |

| bastion-block-SEASON0 | 73397de696 | SEASON-0: derived `Season`/`year_phase`/`day_of_year` (pure fn of `TimeOfDay`, `DayPeriod`'s shape one scale up), RON-tunable year length, --season-scenario gate leg | revert tag bastion-block-BAG2 (0093d4b7e8) | no save/wire change (derived, stateless — nothing to persist) |

| bastion-block-SEASON1 | 889f1e20ed | SEASON-1: `SeasonalSchedule` day-of-year event hook (`Calendar::is_event`'s mirror, one axis over), RON-configured, --season1-scenario gate leg | revert tag bastion-block-SEASON0 (73397de696) | no save/wire change (pure lookup, no stored mutable state beyond the loaded RON schedule) |

| bastion-block-FOCUS0-ENUM | c752571be1 | FOCUS-0 (narrowed): the `Need` enum lock + `BastionColonist.personal_needs: HashMap<Need,f32>` serde-defaulted collection — schema only, nothing reads/writes it yet | revert tag bastion-block-SEASON1 (889f1e20ed) | `BastionColonist` gains `#[serde(default)] personal_needs` (old saves default empty, no migration) |

| bastion-block-SEASON2 | 641b74b5c5 | SEASON-2: the one-interface contract (`season`/`year_phase`/`day_of_year`/`season_bias`), no consumers wired — the last of SEASON-0..2 | revert tag bastion-block-FOCUS0-ENUM (c752571be1) | no save/wire change (pure interface functions, nothing stored) |

| bastion-block-FR15-TIGHTDIG | ed29c00781 | FR15-TIGHTDIG: displacement+arc-length progress metric + reinstated committed-path steer, ALL flag-gated (`BASTION_TIGHTDIG=1`, default OFF); Opus R13 green-light | revert tag bastion-block-SEASON2 (641b74b5c5) | no save/wire change (flag-gated behavior only; flag OFF = bit-for-bit prior behavior) |

| bastion-block-B-AG1-FIXTURE-GEO | 73cd8df83d | 35.1: bag1 fixture geography fixed (settle-first + grounded-civ filter), --bag1-scenario now genuinely PASSES 2/2 | revert tag bastion-block-FR15-TIGHTDIG (ed29c00781) | harness-only, no game code |

| bastion-block-SEASONHUD | 93c1970d42 | SEASON-HUD: overseer "Season · Day N" readout via the SEASON-2 interface, no new sim state | revert tag bastion-block-B-AG1-FIXTURE-GEO (73cd8df83d) | voxygen-only, no save/wire change |

| bastion-block-B70 | 0aea5c63e6 | B7-0: needs-decay + mood-recompute formula (design §3), RON tunables (`bastion_mood.ron`), chronicle `thought_sum` layering (server-side `bastion_thoughts.ron` keeps rtsim's `ChronicleKind` out of `common`), LOD-0 persistence mirror; folded one `chronicle.record(CaveIn)` emitter into the crush path to fix the pre-existing direct-mood-write conflict, fear-persists scenario proves the queue→drain→chronicle→formula pipeline end to end | revert tag bastion-block-SEASONHUD (93c1970d42) | `BastionColonist.needs`/`.mood` already serde-defaulted (pre-existing shells) — no NEW save-shape change this tag; new RON assets `common.bastion_mood` + server-side `bastion_thoughts` (asset-only, not on the save wire) |

| bastion-block-B71 | 4e56c3d8ca | B7-1: `DesignationKind::Bed` (Ladder placement pattern, vanilla `SpriteKind::Bedroll`/frame sprites, zero asset requests) + `JobKind::RestAt` + board-side `BedSlot` (reservations-table shape, capacity-1 occupancy) + the closed rest loop (sleep restores `rest` to comfort, quality-scaled); thought queue generalized to `(who, where, kind)`, `ChronicleKind` gains `SleptInBed`/`SleptOnGround`; two real bugs fixed en route (dead-colonist corpse re-occupying its bed — upkeep loop now death-aware; test-hook damage now uses `Health::kill()`) | revert tag bastion-block-B70 (0aea5c63e6) | `DesignationKind` gains `Bed`, `JobKind` gains `RestAt`, `ChronicleKind` gains `SleptInBed`/`SleptOnGround` — all appended, wire-stable; `BastionColonist` gains `#[serde(default)] owned_bed` (old saves default None, no migration); new board-side `BedSlot` side-table (server-internal, not on the colonist save wire) |

| bastion-block-B72 | 656c1efda8 | B7-2: the self-job preemption mechanism (design §5), ★OPUS-GATE CLEARED (Opus CLEAR-TO-TAG, BUILD_REVIEW_LOG §R14 — all 3 safety claims confirmed true by code-read). Pre-claimed self-jobs bypass claim-selection entirely (out-tiers all work by construction, not comparison); new NEED-CHECK arbitration pass drops work via the proven `to_release` seam + creates a pre-claimed need-job after the drain; anti-livelock trio (hysteresis, unreachable→ENDURE, 60s `PREEMPT_COOLDOWN`); zero new steer/drive code, existing watchdog + `stuck_watch` teleport apply automatically | revert tag bastion-block-B71 (4e56c3d8ca) | no save/wire change (server-internal mechanism only; `NeedTuning.interrupt` field was already serde-defaulted in B7-0's `MoodConfig`) |

| bastion-block-B73 | 1287b161b9 | B7-3 (B7 COMPLETE): the eat-job (`JobKind::EatFrom`, hunger joins the urgency ranking) + the despondent breakdown state (`JobKind::Despond`, sustained-low-mood self-job at own feet) — additive-only on B7-2's cleared NEED-CHECK pass, zero new preemption code, self-verify+tag tier (architect-decided, no dedicated Opus gate). Sonnet-verified live: no-entombment via identical `ActiveJob` seeding + the fully job-orthogonal `embed_watch` center-net; no-thrash via active-despond skip + sustained-window + shared cooldown + probabilistic roll. Fixed en route: the B6 fetch-contract silent-release bug (reservation without `required_item` — new registry B26) | revert tag bastion-block-B72 (656c1efda8) | `JobKind` gains `EatFrom`/`Despond` — appended, wire-stable; no save-shape change beyond that (Needs/Mood fields already existed since B7-0) |

| bastion-block-BAG3V | 199a834f57 | B-AG3 narrowed slice 1: `Value` enum locked (Glory/Tradition/Kin/Wealth/Piety/Nature/Craft/Freedom) + `values: HashMap<Value,i8>` on `BastionColonist` (personal_needs shape, empty=neutral=bit-for-bit pre-B-AG3 mood) + `care_factor` multiplier on `thought_sum` (mood_formula signature unchanged) + `ValueAffinityTable` RON. Reuses vanilla `Personality` (boolean-trait API only, zero touches) + `Sentiments` (relationship/grudge substrate, unmodified) — both reachable through the existing rtsim read-guard, zero new coupling | revert tag bastion-block-B73 (1287b161b9) | `Value` enum + `BastionColonist.values` both appended/serde-defaulted (old saves default empty, no migration); new asset `common.bastion_value_affinities` (server-side RON, not on the save wire); zero changes to any vanilla file |

| bastion-block-FOCUS0DERIVE | ffd7ab1aed | FOCUS-0-DERIVE (row 43.1, THE FOCUS-0 ARC CLOSES): the real generation-time value-roll (`BastionColonist::generate` rolls all 8 values ±50, same rng thread as skills/name/backstory) + `derive_need_weight` pure fn (Pray/Family/Craft/SeeAnimals/Acquire/Fight exact from Value weights, Socialize via the boolean-Personality-trait 3-level API, Drink/AdmireArt/Learn baseline) — produced and proven only, not yet consumed by any job-selection path (FOCUS-1's job). Verified against a genuinely rolled 12-colonist roster (per-colonist exact match, directional ordering, independent-probe consistency, save/load roundtrip through the live LOD boundary) | revert tag bastion-block-BAG3V (199a834f57) | no new save-shape (rolled `values` rides the same serde-defaulted `HashMap<Value,i8>` slice 1 already added — this block only changes what POPULATES it, not its shape) |

| bastion-block-PATH0 | 42f4eb832c | PATH-0 (row 45), ★OPUS-GATE-AT-TAG CLEARED (BUILD_REVIEW_LOG §R15, all 4 load-bearing properties confirmed in code). Colonist Goto searches lifted OUT of the agent `.par_join()` into a Uid-ordered cursor'd round-robin under `PATH_TICK_ITER_CAP` (3000 iter units); `find_path`/`astar.poll` reused wholesale via the extracted `search_step`. Starvation-free BY CONSTRUCTION (cursor rotation bounds deferral, denial is impossible not tuned); determinism-by-construction (`BTreeMap`+`sort_unstable_by_key(Uid)`, zero `HashMap`, zero rng); stuck-economy/no-entombment preserved (waiting state byte-identical to mid-search); vanilla/combat pathing inline-unchanged (config-root gate). TIGHTDIG explicitly out of scope (deferred to row 31.4). Re-scoped to synthetic-N (18-colonist proof, peak 3000/3000, peak_wait=1) since nothing in the codebase currently grows colonist-N. Does NOT close the separate ARCH-003 entropy seam — reconcile at merge | revert tag bastion-block-FOCUS0DERIVE (ffd7ab1aed) | no save/wire-shape change — the scheduler is server-internal (a new sequential system + request queue), no new persisted fields |

| bastion-block-FARM | 682211eac9 | FARM/PROD-2 (row 46), the renewable food loop. First tag to run the new permanent VOXCHECK gate leg (B30 fix confirmed green — first client-buildable tag since B7-1). `DesignationKind::Farm` (persistent footprint, Stockpile precedent) drives till→sow→grow→harvest via one bounded `%15==3` scan; `Growth(1)` reserved for farm wheat so worldgen's `Growth(0)` wheat stays visually untouched; seed conservation proven via ground+bag item-total ledger (15=14+1). Four fixes filed: B31 (job_wanted dual-master silent churn), B32 (vacancy reads Some(Empty) not None), B33 (B6 material-fetch generalized off a BUILD_MATERIAL hardcode — future material kinds inherit free), B30 (voxygen gate gap, now permanently guarded) | revert tag bastion-block-PATH0 (42f4eb832c) | `DesignationKind` gains `Farm`, `JobKind` gains Till/Sow/Harvest-shaped kinds (appended, wire-stable); new items `common.items.bastion.wheat`/`wheat_seeds` (data-only); `WorkType` gains `Farm` + a farming skill field (serde-defaulted) |

| bastion-block-RUN0 | 39543568ea | RUN-0 (row 47, narrowed from RUN-0..2 — RUN-1/RUN-2 blocked, deferred to rows 47.1/47.2): the emergency-run gait + energy governor. `running: bool` on `BastionColonist` (only new state) selects `RUN_SPEED`(1.0) vs `TRAVEL_SPEED`(0.8) at the existing Goto write site; governor drains Energy 15/s while flagged (must beat vanilla's accelerating regen CAP of 10/s — B34, a 6/s first attempt lost the race silently), forces `running=false` below `RUN_MIN_ENERGY`(10); recovery rides vanilla's existing regen untouched. Colonist-only by construction, zero Chaser/pathing/anim surface; stuck-watchdog composition confirmed clean (keys on progress not speed, a running colonist is strictly less stuck-prone) | revert tag bastion-block-FARM (682211eac9) | `BastionColonist` gains `#[serde(default)] running: bool` (old saves default false, no migration); no other save/wire change |

| bastion-block-AUTON0 | afc175f89c | AUTON-0 (row 48), ★OPUS-GATE-AT-PACKET-CRAFT + tag-review — "the plays itself keystone." `comp::bastion::{Drive,Arbiter}` (Work/Idle/Flee, no B7); Guard 1's narrowed activity-authority unification (gate ONE writer `:3436` + the claim-loop entry, `:4583` fail-safe exempt, clears left alone); Guard 6 self-job skip (RestAt/EatFrom/Despond colonists bypass the arbiter entirely, preserving B7-2/B7-3's guarantees by construction); Guard 4 stuck-watch independence demonstrated live (two unprompted organic rescues + a zero-false-trip storm-window proof); Flee built for real off two vanilla per-colonist signals (hostile-target, below-flee-health). Pre-tag fixes: dead-colonist skip (root-caused), B35 (windowed vs global counter asserts), B36 (health-based test conditions must be held not set once) | revert tag bastion-block-RUN0 (39543568ea) | `comp::bastion` gains `Drive`/`Arbiter` (new component, colonist-scoped only, serde as needed for persistence); no vanilla struct changes; `rtsim_controller.activity` write-site behavior changes only for `comp::Colonist`-scoped entities |

| bastion-block-AUTON1 | ede5b80b1a | AUTON-1 (row 49), G2 CLOSES — self-designation generators (mine/haul/build; defense/muster deferred to B8, hygiene/expand skipped). Demand-driven mine generator (deficit = unfilled-plan-cells − stone-supply − pending-mine, `BUILD_MATERIAL==MINE_DROP==stone` so the plan's own bill IS the quota) — quiescence is a structural freeze assert, not a tuned cap. `board.plans` frozen cell lists via `queue_build_plan` (intent-only, zero jobs at queue time, the farm-paint precedent); a per-plan unfilled census drives emission/retirement/demand off ONE shared scan. Skip-columns (plans+stockpiles+farms+beds+`built_xy`) prevent the generator eating its own output — `built_xy` coverage is honest-not-complete for player-painted structures (D20). DF-POLICY hook = one `generator_enabled()` check site, const-true v1. Fixed en route: B24's 4th fixture-geometry instance + B37 (unreachable-haul reservation leak, `should_merge`-amplified) | revert tag bastion-block-AUTON0 (afc175f89c) | zero new wire enums; `board.plans` is server-internal state (no save-shape change); generated jobs use the existing `Job` literal contract identically to painted ones |

| bastion-block-HAULPIN | 5d6b8a133d | HAULPIN (row 49.2, the B37 fix), self-verify+tag. A churning unreachable Haul job DROPS at `HAUL_DROP_STRIKES=3` (the arrival-tolerance growth cap) via the existing `remove_job` (frees the reservation); the slot-7 generator re-emits from a FRESH scan — retry-by-rescan replaces retry-by-churn (the AUTON-1 run-2 starvation class). Designated/player-painted kinds keep the unreachable economy as-is (their own 60-tick amnesty). New read-only probes: `probe_next_id`/`probe_reservations`, `bastion_board_probe` | revert tag bastion-block-AUTON1 (ede5b80b1a) | no save/wire change (server-internal generator + release-path fix only) |

| bastion-block-AUTON2 | 01151c61c1 | AUTON2 (row 50), THE E1 GATE — ★OPUS-GATE, FULLY CLEARED (BUILD_REVIEW_LOG §R17 + addendum). `stagger_interrupt` (care_factor's pattern): hardiness `h` from Craft/Tradition values + Conscientious/Neurotic traits; `eff = base×(1−0.4h)` clamped `[0.05 floor, base×1.5 ceiling]` — hardiest possible (h=1.5) = 0.08, safety-floor-pinned exact, UNIT-tested. Swapped in as the two threshold INPUTS only (`:3442`/`:3445`) — B7-2/B7-3 machinery entirely untouched. `FOOD_DEFS` gained wheat (the const's own designed extension, data-only). Acceptance ruling: E1 recovery reads COLONY-level (stock≥start, ≥5/6 fed, all alive, straggler reported-not-gated), not every-individual-deadline — individual mechanism properties (floor, stagger discipline) independently asserted elsewhere. PURE-STAGGER sufficed, no food-urgency fallback built. Post-tag: a quiet-machine PREEMPT rerun (proper-evidence request) found a real deterministic FAIL on a different assert (`preempt_hover_silent`) masked behind the known flake — diagnosed as a stale test fixture (not a mechanism bug), fixed in `bastion-block-PREEMPT-FIX` (below). Four registry classes surfaced across the ten-draw forensics chain (trait-surface ownership, window-as-decay-clock, IDLE-HOME-LEASH banked to Design backlog, B38 filed), plus B22's 5th instance (classify-by-field-not-leg-name) from the post-tag finding | revert tag bastion-block-HAULPIN (5d6b8a133d) | `NeedTuning`/mood-adjacent structs gain the stagger inputs (server-internal, no new save fields beyond what B-AG3/B7-0 already added); `FOOD_DEFS` const gains one entry (data, not wire) |

| bastion-block-PREEMPT-FIX | b0b7016d89 | AUTON-2 closure fix (post-tag, own commit): the PREEMPT scenario's `preempt_hover_silent` fixture hardcoded `0.21` against the pre-AUTON-2 flat interrupt; made threshold-aware by computing `eff_rest` via the mechanism's own public `stagger_interrupt()` (zeroing the fixture colonist's Craft/Tradition values to isolate temperament, hover level = `eff_rest + 0.01`) — restores the fixture's original intent relative to the colonist's real per-colonist band edge. Test-only, Sonnet-tier (architect-ruled, no mechanism code touched). Closure evidence: PREEMPT ×3 PASS/PASS/PASS, UNIT 30/30, SPIRAL clean via two confirmatory ×2 draws | revert tag bastion-block-AUTON2 (01151c61c1) | no save/wire change (harness-only test fixture fix, `bastion-harness/src/main.rs` only) |

| bastion-block-HIST1 | 574f401132 | HIST-1 (row 54, self-verify+tag): `ChronicleEvents` (`rtsim/src/rule/chronicle_events.rs`), `ReportEvents`' sibling, binds `OnDeath`/`OnTheft` to `chronicle.record()` — same event, two sinks (persistent Chronicle alongside the existing ephemeral Reports), not a new capture mechanism; `Reports` byte-untouched. Death: `actors=[victim,killer?]`, no witness gate (history ≠ gossip). Theft: `[thief]`, site+pos. Verified through REAL pipelines (a real kill, a real theft-emission hook), one event → exactly one record, conservation held, ×2 identical. HIST-2 stays blocked (needs B9 HUD + client sync) | revert tag bastion-block-PREEMPT-FIX (b0b7016d89) | no save-shape change (Chronicle entries were already a persisted structure since HIST-0; this just adds two new live writers into it) |

| bastion-block-AUTON3 | 5e9ed6385f | AUTON3 (row 51, THE AUTONOMY ARC CLOSES — rows 48-51), self-verify+tag. `modulated_urgencies` (pure/RNG-free): Wealth→Work `[0.4,0.6]`, Glory/Adventurous/Worried→Flee `[0.85,1.15]`+0.8-floor, Kin/Sociable-or-Extroverted/Introverted→Idle `[0.07,0.13]`; input-swapped at the arbiter's two `last_scores` writes, selection/commitment/hysteresis untouched. Drive-order safety guard UNIT-pinned (bravest Flee 0.85 > greediest Work ceiling 0.6; `.min(base)` zero-preservation; idle ceiling < work floor for every roll — AUTON-0's liveness contract survives). Scenario proved E2 legibility to the exact f32 bit without a UI. Gate-storm (5 legs red at once) root-caused to a scope-hoisted rtsim read-guard acquisition (B39) + a separately-found timing-marginal HAULPIN window (B40, widened to 480 polls) — both fixed, zero real regressions in the new mechanism itself. Deferred: UI-4 display, DF-POLICY bias, Ben-tuning, B8's live Flee | revert tag bastion-block-HIST1 (574f401132) | no new save fields (server-internal scoring only); `last_scores` is probe-readable, not yet client-synced (UI-4's job) |

| bastion-block-UI4 | e28cc2b7d0 | UI4 (row 62, THE ARC BECOMES VISIBLE), self-verify+tag. Click a colonist → live panel (needs/mood/personality/Drive/`last_scores`, ~1Hz). New wire pair `ClientGeneral::BastionInspect{target:Uid}`/`ServerGeneral::BastionInspectInfo{target,payload:Option<..>}`, tail-appended, ship-together (B30). Server: the `bastion_spawn` deferral pattern (par_join gather → post-join drain-and-answer), rtsim guard at REQUEST cadence (B39's lesson applied proactively); payload = existing probes re-packaged by Uid, zero new data-gathering. Client: latest-wins reply cache. Voxygen: reuses `bastion_pick_entity`/`bastion_select_set` exactly, one plain-text block, placeholder-first. Read-only end to end, non-colonist/stale target → `None` payload (no-crash). Ben-eyeball item flagged to Play-Tester for the next BEN-TEST-CHECKLIST entry | revert tag bastion-block-AUTON3 (5e9ed6385f) | new `ClientGeneral`/`ServerGeneral` wire variants (tail-appended, wire-stable, non-breaking); no save-shape change (all data was already persisted, this only adds a client-facing READ transport) |

| bastion-block-CHOPFELL | 50aff8808a | CHOP-FELLING (row 51.6), base-cut→whole-tree timber. Self-verify+tag + targeted-Opus-at-tag pending (3 named points: no-float-by-construction, ordering determinism, side-table B22-scheduling). ONE base-cut job per tree at the ground-rooted base (fell-set frozen in `chop_fell_sets` keyed by job id, B6-HAUL/BedSlot table shape); top-down z-band drain on completion; yield + per-Wood labor conserved exactly (proven via `cut_polls` telemetry). FR10's floating-canopy residual closed for free. B42 fixed (BED's arbiter-vs-assign race, harness-only); B43 filed separately (a rare pre-existing CASE-003 physics-embed, NOT a CHOP-FELLING issue — the safety backstop correctly fired). B6HAUL-WIDEN (`a6de03b44d`) landed in the same bundle, unrelated logic. NOTE: the intervening commit history also carries the FLAT-TEST-ARENA live-path fix (`1d693b6b2b`) and the Farm-palette fix (`23087dbd68`) — both tracked separately as still-provisional/pending-real-verification, not folded into this row's own evidence | revert tag bastion-block-UI4 (e28cc2b7d0) | `DesignationKind`/`JobKind` unchanged; `JobBoard` gains `chop_fell_sets`(`HashMap`)+`felling`(`Vec`) server-internal fields, no save-shape change |

| bastion-block-UI41 | f6ac4c8bc7 | UI4.1 (row 62.1), the selected-colonist world-space highlight ring. Self-verify+tag, pure voxygen render (no server/wire/harness change). Mirrors `bastion_sync_colonist_markers`'s exact per-entity `DebugShape` sync, keyed on `bastion_selected`; a flat wide Cylinder (r0.7/h0.05, gold `[1.0,0.85,0.3,0.85]`) approximates a ring since no dedicated ring primitive exists — tracks the colonist each frame, clears on the same triggers UI-4's panel uses. VISUAL-UNVERIFIED until Ben eyeballs it (B41's lesson applied) — ships in the same combined rebuild as ARENA+Farm-palette | revert tag bastion-block-CHOPFELL (50aff8808a) | no save/wire change (pure client-side render, one new `HashMap<Entity,DebugShapeId>` in voxygen state only) |

| bastion-block-ARCH003 | 9dfff6ec7e | ARCH-003-INTEGRATE (row 50.6), architect-directed integration (not a standard builder packet). Rebased the Bug-Tester's Opus-CLEARED clean tree (8 fixes, `codex/arch003`) onto fleet HEAD, resolving the known PATH-0 conflict (flagged at row 45's own tag entry); also the Grok Phase-1+2 test-infra bundle merge point. Fleet-wide priority halt held through this merge (builder+Play-Tester paused, 7+ Ben findings banked without building). Pre-merge tree cleanup (Sonnet): a bookkeeping commit (`84f269b0c9`, 17 readme/run-log/ledger/master-list files, scanned clean for secret-shaped patterns first) + the builder discarding 2 paused EXHAUSTIVENESS-ASSERTS/Bed-fix source files (pre-verified patch-recoverable via `git apply --reverse --check`, HEAD unchanged at `f6ac4c8bc7` through the discard). Merge verified by the architect on the actual merged tree: clean release build, 3x seed-21 byte-identical (only the known benign wall-clock field differs), seed-22 sanity PASS. FREEZE LIFTED — builder/Play-Tester resume; next is EXHAUSTIVENESS-ASSERTS/TOOLBAR-ICONS/Bed-fix (row 51.52), then the 8 banked Ben findings | revert tag bastion-block-UI41 (f6ac4c8bc7) | merge/integration commit — no new save-shape or wire-shape change introduced by this row itself beyond whatever the 8 rebased ARCH-003 fixes + Grok test-infra bundle already carried (Opus-cleared prior to this integration step) |

| bastion-block-EXHAUST | 92ec5eabf1 | EXHAUSTIVENESS-ASSERTS + BED-TOOL fix (row 51.52), on the merged ARCH-003 tree. Root-cause fix for the hand-mirrored-enum-list class that dropped Farm then Bed from the toolbar (`ToolMode::ALL` was a literal array, bypassing Rust's exhaustiveness checker which only engages on a real `match`). `DesignationKind::is_tool_paintable()` — exhaustive match, no wildcard, so a future variant fails to compile until categorized — plus a bidirectional voxygen parity test (every paintable kind has a button, every button maps to a paintable kind); both would have caught Farm AND Bed. Bed folded in per architect ruling (3rd instance of the class, found by this work itself): `is_tool_paintable(Bed)=true` + real `ToolMode::Designate(Bed)` button (palette 10→11), all supporting infra pre-existed from B7-1. Verification scope disclosed honestly: SIM-INERT (common additions touched by no sim path, only behavioral change is the client Bed button) — verified via common UNIT (31/31) + voxygen parity test (1/1) + harness BUILD + VOXCHECK, full 37-scenario gate deliberately skipped as redundant (scenarios test sim, this can't touch it; Bug-Tester's isolated-worktree catalog run separately covers the full suite), same precedent as the Farm-tool/UI-4.1 client-only tags | revert tag bastion-block-ARCH003 (9dfff6ec7e) | `DesignationKind` gains `is_tool_paintable()` (new method, no schema change) + `EnumIter`/`ZoneKind::Default` derives (compile-time only); `ToolMode::ALL` grows 10→11 (Bed added, client-internal array, no wire/save shape change) |

| bastion-block-ICONS | 4f3fe6aa10 | TOOLBAR-ICONS (row 51.5), voxygen-only UI polish, no sim/server touch. Overseer palette buttons render as 34×34 `Button::image` widgets (active=bright/dimmed otherwise) in place of text `.label()` calls; God/Free stays text. 11 pre-delivered asset-lab icons copied into `assets/voxygen/element/ui/bastion/` + declared in `img_ids.rs` + palette loop rewired. VOXCHECK green, all 11 specifiers confirmed resolving to real committed files. Known readability issues (mine/chop/pan misreads, gather↔farm collision) shipped as-is per placeholder-first, deferred to asset-lane resume. New gap found by this pass: Bed has no icon (added to the palette by EXHAUSTIVENESS-ASSERTS after this 11-icon set was made) — falls back to transparent image + text label; `tool_bed.png` logged as a 12th-icon asset backlog item in `ASSET_REQUESTS.md`. VISUAL-UNVERIFIED until Ben eyeballs it (VOXCHECK proves compile+resolution only) — Ben-checklist item routed via architect | revert tag bastion-block-EXHAUST (92ec5eabf1) | pure client-side asset/UI change — no save-shape, wire-shape, or sim-behavior change; `img_ids.rs` gains 11 new image id declarations only |

| bastion-block-PROGRESS | 7f087da317 | CHOP-PROGRESS-INDICATOR (row 51.61). Sim-inert display field `Arbiter.activity: Option<(WorkType,f32)>` (None default, never read by scoring/selection/hysteresis) written at the job-progress path, cleared at the existing `to_release`/`last_scores` sites — no new borrow. Re-packaged into the existing `BastionInspectPayload` (tail-appended, no new wire message, B30 discipline held); panel shows "Doing: Chop N%" / "(idle)" reusing UI-4's fetch cache. New probe `bastion_colonist_activity`. Verified via the testing tools (harness scenario, not code-read alone) per the architect's standing directive: chopfell ×2 byte-identical, activity populates and climbs to ~99.9% pre-fell on both small/big trees, all pre-existing felled/topdown/no_orphan/drops asserts unchanged confirming sim-inert; UNIT 31/31; VOXCHECK+BUILD green | revert tag bastion-block-ICONS (4f3fe6aa10) | `comp::bastion::Arbiter` gains one `Option` field (server-internal, no save-shape change — display-only, not persisted meaningfully beyond the existing struct); `BastionInspectPayload` gains one tail-appended field (wire-stable, non-breaking, same discipline as UI-4 itself) |

| bastion-block-UI5 | b5e4755336 | UI-5 UNIVERSAL DEBUG INSPECTOR (row 62.2), self-crafted in the UI-4 pattern per Sonnet's green-light, Sonnet-routine tier (no new dynamic mechanism, widens WHAT a target can be not how targeting works). `BastionInspect` generalized: `target: BastionInspectTarget::{Entity(Uid)|Cell(Vec3<i32>)}`, `payload: Option<BastionInspectKind> = Colonist(UI-4's payload verbatim)|Job|Stockpile(contents, closes 51.64's legibility gap)|Farm|FellSet`. Server resolves a clicked cell XY-column-first [job→stockpile→farm→fell-set→None] in UI-4's existing post-join drain; empty-handed click inspects, colonist click still selects [UI-4 behavior fully preserved]. READ-ONLY end to end. New `--inspect-scenario` = ladder leg 38. Gate: full 38-leg ladder at HEAD `42f7c464a0` = 37/38, all PASS incl. INSPECT+CK, BED red field-classified as the registered CASE-003 `bed_occupied_mid` signature (architect-accepted). Earlier dev-time verification: inspect ×2 bit-identical, server/voxygen rc=0. NOTE: two more commits sit above this on the branch not tracked as fleet rows — `871a9157d9` (B5.8 Stage-1, external-effort-originated, architect's own provenance lane) and `42f7c464a0` (an overruled/untagged CK CarvedStair fix shape, superseded by a Phase-1 walkable-stairs commit in verification that will carry the real CK-fix tag separately) | revert tag bastion-block-PROGRESS (7f087da317) | `BastionInspect`'s wire target/payload types both change shape (target gains `Cell` variant, payload gains 4 new kind variants) — non-breaking append/widen on the existing UI-4 wire pair, no save-shape change (all data was already server-side state, this only widens the READ transport) |

| bastion-block-CKSTAIR | 9ad9d97808 | STAIR-LADDER Phase 1 (CK CarvedStair fix) + STUCKJOB (α) watchdog fix, tracked in `STAIR-LADDER-MINE-ACCESS-DESIGN.md` §Phasing rather than a numbered master-list row. Pair of commits: `177c12094f` (Phase 1 — emergency stairs become walkable plain Mine digs, no route ownership/traversal task, permanent infrastructure; plan tuple descriptor became `Option`; only ladder/shaft plans stay route-owned — fixes the root cause of the CK entombment regression, a walkable stair plan wrongly wrapped in a route-owned `EmergencyRouteDescriptor{kind:CarvedStair}` with no executor ever written) + `9ad9d97808` (STUCKJOB — a second, independently-latent Stage-1 watchdog defect: teleport-suppression must be EARNED by verified per-colonist `(job,progress)` baseline, not claim-holding/churn; new `--stuckjob-scenario` = ladder leg 39, proven RED→GREEN [unfixed: never rescued in 200s vs 60s target; fixed: rescued at 59.0s]). CK 5/5 PASS + recorder + full ladder 38/39 (BED = registered CASE-003 `bed_occupied_mid`, third identical draw). Corrects misfiled B22 `ck_failsafe_out` (invariant, not flake). Positive capability note: STUCKJOB's rev-1 falsifier proved organic stair self-rescue end-to-end (26s, no backstop). Untagged intermediates on the branch, no row action: `42f7c464a0` (overruled CK fix shape, superseded) + `871a9157d9` (B5.8 Stage-1, architect's provenance lane) | revert tag bastion-block-UI5 (b5e4755336) | plan tuple descriptor (`server/src/bastion_jobs.rs` access-planning) changes shape to `Option` for the walkable-stair case — server-internal planning state, no save/wire-shape change; the watchdog gains a per-colonist `(job,progress)` baseline field, also server-internal, no persisted-save change |

| ff2874b4b6 | ff2874b4b6 | STAGE-1 SCOPE COMPLETION — external-effort provenance lane (same shape as `871a9157d9` B5.8 Stage-1), no master-list row. Closes the 12-vs-24-file gap in Stage-1's own declared dependency set: the 6 files its original list omitted — `common/systems/src/phys/{mod,collision}.rs`, `common/src/{bastion,rtsim,path}.rs`, `server/src/connection_handler.rs` (+500/−124). **Restores tag reproducibility for every tag since Stage-1** (`871a9157d9` through `bastion-block-CKSTAIR` were all clean-checkout-unbuildable without these 6 files — discovered when the grok-integration worktree hit `cylinder_sweep_first_collision` missing from committed phys). These files were LIVE in the working tree through every gate since Stage-1 landed regardless — a provenance gap, not a correctness gap. Opus review: `capsule_terrain_cylinder` confirmed an exact behavior-preserving extraction of the old inline sweep; `route_squeeze_until` gating confirmed robust across all 15 write sites — the same mechanism [REQ-0052-ROUTE-SQUEEZE-DESIGN.md](REQ-0052-ROUTE-SQUEEZE-DESIGN.md) documents as a contract; that doc's two open items (FrontierWork gating, rescue_pending co-interaction) match Opus's own two non-blocking notes exactly, not duplicated here | revert tag bastion-block-CKSTAIR (9ad9d97808) | no NEW save/wire-shape change beyond what Stage-1 itself already introduced — this commit only makes already-live behavior reproducible from a clean checkout, it does not change runtime behavior (files were already in the working tree) |

| bastion-block-CLIMBCAP | 7483439958 | FREE-CLIMB DEPTH CAP + A2 RESCUE-PROGRESS GATE + BELOW-GRADE BOUNDARY FIX (STAIR-LADDER Phase-2 mechanic pulled forward, Ben-ruled, Opus-cleared design + CLEAR-TO-BUILD re-review), no master-list row (tracked in `STAIR-LADDER-MINE-ACCESS-DESIGN.md`). Server-side-only skill-scaled climb reach (`cap_for_skill(level)=3·(level+1)`, descent/hold never capped, ladder token exempts, single-source gate per Opus R1, `climb_anchor` per Opus R3) fixing the emergency ladder tier's unreachable-in-practice problem (free-climb self-rescue always won pre-cap). Seed-corpus leak found 2 root causes (spawn-variance + XP self-licensing) closed by an inverted XP grant + a frozen cap-skill episode snapshot — B44/B45 filed. A2's `rescue_pending` made progress-earned (mirrors STUCKJOB-α, closes an F5-class stale-target suppression hole). Below-grade boundary fix (grounded-only near-surface wipe). New `watch_wipe()` diag shim on all 11 stuck_watch resets. F5 rev-2 falsifier rewritten (3-colonist A/B/C, C = pure A2 discriminator). Full seed-corpus verification (6 seeds × 3 reps × 2 scenarios, byte-deterministic): geometry-probe 6/6, STUCKJOB+F5-rev-2.1 6/6×3, full ladder 38/39 (BED = registered CASE-003, unchanged signature). One residual reported not self-cleared: a seed-8 through-wall breach, identified as the pre-existing CLASS-6 arrive-through-walls seam (B46) — proven independent of the cap/A2, architect-flagged, retroactively reopens this tag if a rerun implicates the cap/A2 instead | revert tag ff2874b4b6 (ff2874b4b6) | `BastionColonist` gains `climb_anchor`/`climb_cap_skill` fields (server-internal climb-mechanic state); `rescue_pending`'s suppression semantics change from any-target-exists to per-colonist-progress-earned (server-internal watchdog logic) — no wire-shape change, save-shape addition only (new serde-defaulted fields, old saves unaffected) |

| bastion-block-M2LADDER | cd69f61111 | M2 owned constructed-ladder egress (mine-complexity-ladder M2 tier, Ben-greenlit; no master-list row). Planner fixes + deterministic mount-snap + at-entry unlock + the two-layer approach-corridor productionization (corridor drives via owned controller inputs, not the rtsim Goto slot [tolerance inversion killed]; runtime sweeps anchor planned segments, not live position [own-rung clip killed]). Seed-20 stranded class CLOSED organic-owned 55s; GATE-HOLD 18/18→0; organic 52% best-ever; never-stranded 18/18. Architect Opus-depth GREEN with 4 named-open items (s1337/s22 engaged-but-backstop optimization frontier; vanilla-leak fork #15; AgentInbox interruption dead-on-live-path; class-7 item identity) | revert tag bastion-block-CLIMBCAP (7483439958) | EmergencyApproachCorridor gains `origin` field (server-internal planning state, no save/wire shape change); JoinData gains constructed_ladder_traversal (component was already committed; plumbing is read-only threading); no schema/persistence changes |

| bastion-block-BACKSTOPOPT | 2880f341d6 | 2/6-backstop optimization (Ben fork #16 step 1): the release-decision three-outcome state machine + energy-wait hold (120s per-episode, progress-gated) + sticky exhaustion + progress-discrimination; B organic 6/6 (51-187s), C in-bar everywhere (60-133s via the designed net), never-stranded 36/36; safety proofs N7+N7B both arms w/ the watch-accrual tape excerpt; two self-introduced regressions caught+fixed by the block's own corpus mid-flight (registry classes 11/12) | revert tag bastion-block-M2LADDER (cd69f61111) | JobBoard gains emergency_reengage_aborts / emergency_energy_wait_ticks / emergency_reengage_exhausted / emergency_no_progress (all server-internal transient state, no save/wire shape change); no schema changes |

| bastion-block-M3 | 8c4543094a | LADDER CONTENTION (mine-complexity-ladder tier M3, no master-list row). M2's single-owner ladder contract proven under crew contention: reservation/fair queue, exactly one owner per link, release frees the slot. Landed via bastion-block-M3A (cebb45746f) — the crew-contention fix stack (corridor-commit validation, own-position entry fallback, promotion driver, B57 site-3 mount-preflight fix) capped by a Sonnet-ruled unification: 3 independently-disagreeing waypoint sources (a genuine livelock) collapsed to ONE bounded-A* corridor authority committed at promotion, consumed statefully, head-only. B57 site 4 (corridor-stepper runtime-revalidation livelock, same signature) closed post-tag at cf837245da. 24-run seed matrix: queue invariants (fair order, zero double-ownership, zero lane violations) universal on 24/24; 4 red classes decomposed — a1 hard-roll backstop, a2 fixture-predicate false-positive, a3 M3D timing-bar calibration, and b-inherited/B58 (Sonnet-ruled tag-acceptable tracked-open: seeds 21/42's residual net-reliance is inherited M2-era behavior, proven via the N2-at-same-seeds discriminator, not an M3-queue defect — the packet's own acceptance criteria treats the net as a valid backstop). Opus gate PASS, 2 tracked follow-ups none blocking (BUILD_REVIEW_LOG.md §M3) | revert tag bastion-block-BACKSTOPOPT (2880f341d6) | new `bastion_traversal_tasks`-adjacent server-internal corridor/queue state (fair-queue fields, corridor commit/consume state, epoch/generation fencing) — no save-shape change (server-internal traversal-session state, not persisted); no wire-shape change |

| bastion-block-CRATESPLIT | 6357c35d23 | Efficiency slate #2, no master-list row. Branch `bastion/builder`, NOT YET on `bastion/block-B6HAUL` at time of logging. Pure structural move: 11 of 12 `server/src/bastion_*.rs` modules (~18.2k lines) extracted into new leaf crate `bastion-server` (`bastion_arena` stays server-side, the one deliberate exception); `veloren-server` re-exports every moved item at its old path so all existing references compile unchanged; dispatcher system names unchanged, dispatch order byte-stable by construction. 6 coupling knots resolved (Tick, RtSim-as-trait+generic-Sys, `traversal_config_for`, RepositionToFreeSpace, test_world/worldgen feature forward, bastion_arena carve-out — the packet scoped 3, the full 12-module survey found 3 more). Architect-ruled: byte-identity is conclusive for a pure structural move, no separate validation corpus needed. Verified: R10/M3 pins fire from the new home (35/35+11/11); `--dig-access-scenario`@1337 byte-identical pre/post ×2; `--mine-fidelity-scenario` canonicalized-identical (one pre-existing field-order nondeterminism, unrelated, architect-confirmed, Codex's determinism sweep owns it separately). Measured compile win: full rebuild 65s→~50s (−23%), check-loop 9.1s→3.2s (−65%). Opus gate PASS. Sequencing note: no further server/bastion-server-tree work until the architect's determinism-integration base (this tip + a Codex rebase) lands | revert tag bastion-block-M3 (8c4543094a) on `bastion/builder` (not `bastion/block-B6HAUL` — branches have diverged, revert path assumes the branch this tag actually landed on) | pure structural move — zero behavioral/save/wire-shape change by design (the whole point of the block); every moved item re-exported at its original path, so nothing outside `server/src/bastion_*.rs`'s own file boundaries changed at all |

| bastion-block-B58 | 8e0e3bc03d | Frontier-approach corridor-unification (closes the B58 tracked-open follow-up filed at the M3 tag), no master-list row, mine-ladder-adjacent same as M3/CRATESPLIT. Branch `bastion/builder`, built atop the determinism-integration base (a643d8dee6 fast-forward, untagged merge event — no ledger row of its own, chained here as "revert to CRATESPLIT" since that's the last actual tag in the chain). 3 commits: a7213f735d (frontier-reacquire unified onto the live-position corridor authority via a `frontier` param on `m3_promoted_corridor_waypoint`; no-progress replan-from-position trigger [≥30 ticks/<0.1 blocks] replaces stored-corridor replay for displaced members; stuck_time wipes now earned by real movement) + 6dcd679253 (rider: single-owner debug_asserts at authority-entry + reacquire-drive, release-inert) + 8e0e3bc03d (test fix-forward: corridor unit-test initializer needed the new `last_check` field, caught in self-gate before tagging, 35/35 green). Evidence: seed-21's previously-frozen member now forms its transaction and climbs out for real (first_owner None→Colonist-0, teleports 4→3, delivery 234s→183s); self-gate M3A+M3D+N2 all PASS@1337 with rider asserts live. Honest residual: seeds 21/42 still red — every remaining net traced to the SEPARATELY-FILED organic-climb-bounce escalation-starvation class (BUILD_REVIEW_LOG.md §FILED, ties stuck-economy/R11 + FR15 paired-A/B, routed to Ben's morning triage, not folded into this tag). R10/M3 pin counts unchanged (remove==1, advance_epoch==2, insert==3, fenced==13) — confirms no touch to fencing invariants this block doesn't own. Opus gate PASS (BUILD_REVIEW_LOG.md §B58) | revert tag bastion-block-CRATESPLIT (6357c35d23) on `bastion/builder` (the determinism-integration fast-forward sits between them, untagged) | `m3_promoted_corridor_waypoint` gains a `frontier` parameter (server-internal signature widening, no save/wire-shape change); corridor state gains a `last_check` field (server-internal, test-initializer-visible only) — no persisted-save or wire-protocol change |

| bastion-block-ENGOPT1 | 115cd34e54 | Engine-optimization #1 (ledger 177+175), no master-list row. Branch `bastion/builder`. `common/src/astar.rs` only: A* frontier tie-break widened to a total-order tuple `(f,h,g,fxhash64(node),seq)` [architect-ruled option (c), vek lacks Ord]; fallback best-so-far made Detour-faithful [stores neighbor not parent, seeds from start — 2 real pre-existing bugs fixed]. Falsifier-verified (RED on seq-only key, GREEN on the full tuple); 4/4 property tests; workspace rc=0; mf ×2 canonicalized-identical, da ×2 byte-identical on a fresh binary; N2 PASS. M3A@1337 classified red (registry B60, LATER RECLASSIFIED same day — see the B58/B60 chain and the FORK15 closure) — architect-ruled tag-acceptable, exposed not caused. Opus gate PASS (BUILD_REVIEW_LOG.md §ENGINE-OPT-1) | revert tag bastion-block-B58 (8e0e3bc03d) | `astar.rs`-internal frontier ordering + fallback bookkeeping only — no save/wire-shape change (pure algorithm-internal state) |

| bastion-block-ENGOPT2 | 623fc58f01 | Engine-optimization #2 (ledger 176), no master-list row. Branch `bastion/builder`, packet standards inherited from ENGINE-OPT-1. `common/src/astar.rs` only: A* decrease-key/reopen correctness via lazy deletion (Detour `findPath` reopen quoted as prior art) — the pre-176 `!previously_visited` guard silently dropped real improvements to closed nodes, a genuine correctness bug not just a determinism issue. Two falsifiers both confirmed firing on the emulated OLD mechanism (diamond graph 17.0-vs-10.0; Bellman-Ford divergence) → green post-fix; astar suite 6/6; ENGINE-OPT-1's determinism preserved; M3A/M3C/N2/M3D byte-stable, classified M3A red preserved EXACTLY at `lane_violations=3`. Downstream dig-access economy effect on a 7-seed paired A/B is a mixed reshuffle, architect-ruled SHIPPED-CLASSIFIED not silently absorbed (registry B62) — a stuck-economy retune tracked as its own separate follow-up, batched post-pathfinding-arc, logged to `DECISIONS-FOR-BEN.md`. Also registered: 2 vm-jobs.sh test-tooling incidents under a shared SILENT-RESULT-INTEGRITY class (registry B63, infra not game code, both already fixed). Architect SHIP ruling (Ben-directed GO, no morning hold) | revert tag bastion-block-ENGOPT1 (115cd34e54) | `astar.rs`-internal reopen/lazy-deletion bookkeeping only — no save/wire-shape change |

| bastion-block-ENGOPT3 | 695bbb0172 | LOOT-AUTHORIZATION INVERSION fix (ledger #160, registry B64), no master-list row. Branch `bastion/builder`, architect-GO'd after a crossing reconciliation. `server/agent/src/action_nodes.rs` + `common` loot_owner + a tooling source-scan pin. TWO inversions found, not one (the outer `!` on the whole authorization + an independently-inverted hostility polarity in the soft-wish term, partially cancelling — the soft+hostile/soft+peaceful branches were accidentally correct, which is why a naive single-flip would have broken them; the falsifier documents the cancellation executably). Severity bounded all along by a pre-existing `InventoryEvent::Pickup` commit-time revalidation gate — the TOCTOU half of the original ledger item was already moot; observable damage was entitled-loot refusal + attempt-spam, not theft. Pins: intended truth table + verbatim-old falsifier (agent), can_pickup truth table (common, that file's first tests), commit-gate source-scan guard (tooling). Verified: VM-fan all-attested, M3A classified red (B60) byte-preserved, N2 PASS, mf undisturbed | revert tag bastion-block-ENGOPT2 (623fc58f01) | `attempt_pickup`/hostility-polarity logic fix only — pure boolean-logic correction, no save/wire-shape change |

| bastion-block-ENGOPT4 | 7b994ea99c | SlowJobPool/ARCH-003 scheduling diagnosis, self-gated (new batched-review process), no master-list row. Branch `bastion/builder`. Same-platform triple-divergence diagnosis with attested cpuPlatform; 3 stages (sorted chunk-apply, hasher-independent pool selection, harness-mode deterministic apply barrier), all falsifier-pinned. Measured: cross-VM field divergence 20→12, mf_completion byte-equal cross-machine. HONEST: full mf byte-identity NOT achieved — residual = agent-layer par_join() seam, named as next block, not claimed closed. PATH-0 re-verified already-deterministic; ledger #181 premise stale. Safety floor: M3A classified red byte-preserved, N2/M3D PASS, attested with the barrier active | revert tag bastion-block-ENGOPT3 (695bbb0172) | scheduling/chunk-apply-ordering changes only (server-internal ARCH-003 determinism plumbing) — no save/wire-shape change |

| ledger-178 | 4f5de38f08 | Profile-keyed invalidation for the Chaser's retained search-context, opened mid-ENGOPT4 per the never-stop-on-the-ledger rule, self-gated, no master-list row. A sharp falsifier fired on stale admission through a since-unloaded band; a broader falsifier is an honest executable negative pinning that ENGINE-OPT-2's reopen fix already self-heals this case (not a second bug needing its own fix) | revert tag bastion-block-ENGOPT4 (7b994ea99c) | Chaser retained-search invalidation logic only — no save/wire-shape change |

| bastion-block-ENGOPT7 | a4018f948a | ★ SUPERSEDED — #183's portion of this tag was REVERTED at `daaf8aba45` (see the row below); DO NOT restore to `a4018f948a` alone, it reintroduces a proven M3A strand regression. #179 (Small→Medium continuation equivalence, executable negative, no fix) still stands and is unaffected by the revert. Original entry preserved for the trail: ~~Ledger #183 (Chaser no-path negative cache, SHIPPED) + #179, self-gated, no master-list row. Branch `bastion/builder`. Named ENGOPT7 deliberately out of strict landing order to preserve the still-open residual's in-tree ENGOPT6 name. #183: a completed empty-frontier verdict against an unchanged (target, #178-profile) question suppresses re-search; invalidation on target-move/profile-flip/InvalidPath/arrival/90-tick half-open re-probe. Falsifier fired unfixed (200 searches/200 ticks), pinned ≤5 fixed. veloren-common 155/156, bastion-server 36/36.~~ #179: sharp precondition PROVEN engaged via a test-only Astar::visited() accessor (not vacuous like #181/B65), still came back negative — ENGOPT2's strict-improvement re-push heals the mixed frontier; test kept as a permanent equivalence pin, no code shipped | revert tag bastion-block-ENGOPT4 (7b994ea99c) — the ENGOPT6 residual (fix `3b137017e6`) sits between ENGOPT4 and this tag chronologically, still untagged/open (fix kept, root seam not yet closed — see the correction in the run-log) | `Chaser` gained negative-cache state, now removed by the revert below; #179 ships zero production code, test-only |

| bastion-block-ENGOPT7-REVERT-183 | daaf8aba45 | REVERT of ledger #183 (Chaser no-path negative cache) — the actual safe tip superseding `a4018f948a` above, no master-list row. `floor6` safety fan caught an M3A strand regression at `3b137017e6`: `[66,null,null]`/0 lane violations vs the byte-baseline `[66,82,94]`/3 (the tracked fork-15 red — its absence was the tell). Root cause: suppressing PATH-0's search cycle changed a waiting queue-member's movement duty 14%→91% bearing-ticks, relocating its parking spot, which flipped the feet-anchored organic-egress target computation onto an unreachable elevated ladder-column cell — `final_mount`'s `route_mount.xy==target.xy` gate never fired, no `QueuedForLink` task, provenance window lost. A stuck-economy-constraint-class finding (a new steer/drive duty invalidated the tuned egress web). The 200-searches/200-ticks inefficiency #183 targeted is real; the fix is blocked on decoupling egress-target selection from search-cycle side effects, its own future block. Revert removes #183's live semantics + pins/fixtures; #178/#179/#180 untouched. `veloren-common` 153/154 (B59 only). M3A local at this tip: `[66,82,94]`/3, correct byte-baseline. `floor7` fan re-running here | revert tag bastion-block-ENGOPT4 (7b994ea99c) — `a4018f948a` sits between them chronologically but is NOT a safe intermediate restore point (see its own row above) | pure revert — removes exactly what `a4018f948a` added for #183, no new save/wire-shape change beyond returning to the pre-#183 shape |

| bastion-block-ENGOPT6 | 781a553eb71e | Agent-layer determinism residual, ROOT-CAUSED + CLOSED, no master-list row. Branch `bastion/builder`. Chain: LootOwner wall-clock→sim-time (3b137017e6, kept, real bug, not the seam) → HAUL-RETARGET fallen-item fix (502ad6897a, registry B68, made the seam's race observable) → merge-topology UID-sort (781a553eb71e, the ACTUAL seam — entity-ID join order vs stable UIDs during a mass-merge burst, cured via ENGOPT4's own sorted-apply pattern applied to this consumer). END-PROOF: tapes8 pair raw BYTE-IDENTICAL across both VMs (36,059 trajectory + 24,726 event tick-blocks, zero divergence) — the strongest evidence grade available. Floor green (N2 rc=0/tp1, M3D rc=0 [145,44,204]/2/hold[T,F,F]/alive). Also landed on this branch since the last ledger row (T0.1-T0.11, all self-gated, individually run-logged not separately ledgered): T0.6 (tick-rate-invariant probability, DONE.18), T0.7 (tick-rate-safe AI rates, DONE.20, new canonical fixture baselines M3A [66,44,97]/2 + M3D [145,44,204]/2), T0.8 consistency slice (DONE.21, hitch clocks lag together; bounded-substeps half deferred), T0.9 (subsumed by T0.1, DONE.19), T0.10/T0.11 (strategic time, DONE.22, fixes registry B69 — Job::Hired/Quest.timeout restart-relative deadline). Instrumentation (haul-item/pickup-verdict/merge trails) now permanently in-tree, env-gated, zero live cost | revert tag bastion-block-ENGOPT7-REVERT-183 (daaf8aba45) — everything between (T0.1-T0.11 + the ENGOPT6 hunt commits) rides this single checkpoint; see the run-log for the granular commit-by-commit trail if a narrower revert is ever needed | `Job::Hired`/`Quest.timeout` change from sim-`Time` to `TimeOfDay` (a genuine SAVE-SHAPE change — old saves' payloads reinterpret as past world timestamps on load, a disclosed one-time migration, not backward-compatible by design); everything else in this range is server-internal state (merge/pickup ordering, clock consistency, hazard-rate conversions) — no other save/wire-shape change |
| bastion-block-CTRLFRAME | 9b3c6850ac3e | T0-002 group (T0.12-T0.27, phase manifest/scheduling) + T0-003 opening slice (T0.28/32/33/34/38/39/42), all self-gated, individually run-logged. Highlights: T0.12 declarative phase manifest; T0.14/15/18/20/23 order contracts; T0.16 jobs→RTSim outbox edge; T0.21/22 Controller frames + tagged envelope (the block's namesake — private generation-stamped buffers, single sequenced QueuedCommand{phase,seq,payload}); T0.25 handler-registry validation; T0.26 all-builds topology check; T0.38/39 claim/need-target determinism now unconditional in live mode. floorv green at tip `b3314978`: M3A `[66,44,97]`/2 (tracked-red), N2 rc=0 tp1, M3D `[145,44,204]`/2 hold[T,F,F] rc=0 — no baseline shift. Acceptance: tapes9/10/11/12/13, all byte-identical, unbroken chain since ENGOPT6. Registry: B70 (debug-only field made load-bearing, fixed), B71 (mid-plan-mutation tracked finding, deferred-networking), B72 (python whole-file writes churn CRLF, Edit-tool-only rule adopted), B73 (pre-existing plugins-cfg gap, not yet fixed), B74 (choose()-candidate-ordering forward caveat) | revert tag bastion-block-ENGOPT7-REVERT-183 (daaf8aba45) — everything between (T0.12-T0.27, T0.28/32/33/34/38/39/42) rides this checkpoint; see the run-log for the granular commit trail | server-internal scheduling/RNG/ordering state only (dispatcher edges, event delivery classification, RNG stream keys) — no save/wire-shape change |
| bastion-block-T0DET3 | 96315c8fbf85 | T0-003 group complete (T0.29-T0.49, deterministic order/RNG/canonical-state/item-identity), all self-gated, individually run-logged. Highlights: T0.29-31 stamped event bus (EventStamp{epoch,producer,seq}+causation/correlation/idempotency fields, machinery-present-unpopulated); T0.32/33/34/36/37 keyed-RNG family (ChaCha8 replacing OS entropy at 2 previously-undetected latent seams — NPC spawn orientation + 3 Apply-handler economy outcomes, filed as B75); T0.38/39 claim/need-target total order unconditional; T0.40 Neumaier-compensated thought_sum; T0.46 defragment tie-break; T0.47 SQL batch order; T0.48 the standing persisted-collection gate (+ one real fix, NpcLinks.rider_map canonical-serialize); T0.49 ItemInstanceId (per-world nonce + synchronous monotonic counter). floort green at tip `b73db158`: M3A `[66,44,97]`/2 (tracked-red), N2 tp1, M3D `[145,44,204]`/2 hold-live — all canonical. Acceptance: tapes9-15b, all byte-identical. Registry: B75 (byte-identity coverage-gap lesson) | revert tag bastion-block-CTRLFRAME (9b3c6850ac3e) — everything between (T0.29-T0.49) rides this checkpoint; see the run-log for the granular commit trail | server-internal RNG/ordering/hashing state + one new persisted field (`ItemInstanceId` on PickupItem, serde-default Option, additive-only) — no breaking save/wire-shape change |
| bastion-block-T0DET4 | a1130b1c5793 | T0-004 group complete (T0.50-T0.66, async acceptance/agent parallel merge/domain hashing/recorder proof/schedule fuzzing/event budgets — the last Tier-0 determinism group), all self-gated, individually run-logged. Highlights: T0.50/51 async ownership substrate (AsyncOwnerKey/AsyncGeneration/AsyncRequestId, exhaustive AsyncTerminal, bounded queue, owner-phase semantic-key merge); T0.52 Agent parallel plan buffers — proven byte-identical serial-vs-multi-worker-parallel (`--deterministic-parallel`, now permanent standing infra); T0.53/55/61 canonical domain-hash + Merkle tree + FinalStateCertificate; T0.56/58/60 causal recorder + schema versioning + span hierarchy; T0.54/57 provenance + content manifest; T0.59/62/63 causal oracle + run-equivalence (LIVE-PROVEN, not types — see below); T0.64 Bastion-specific schedule fuzzer (`BASTION_SCHEDULE_SEED`, Loom/Shuttle deferred, DECISIONS-FOR-BEN.md #24); T0.65/66 token-bucket+DRR event budgets + hierarchical per-domain work quotas (reusing T0.12's manifest domains, zero new taxonomy). **CAPSTONE PROOF (fuzz1, definitive acceptance):** a real FinalStateCertificate now emits at the harness's final phase; serial + 2 schedule-fuzzed legal schedules, run on 3 DIFFERENT MACHINES, all produced the IDENTICAL durable_composite — determinism proven cross-machine AND cross-schedule simultaneously, the strongest close in the whole arc. Registry: B76 (combat-parallel coverage caveat) | revert tag bastion-block-T0DET3 (96315c8fbf85) — everything between (T0.50-T0.66) rides this checkpoint; see the run-log for the granular commit trail | server-internal async/hashing/recorder/scheduling substrate only — no save/wire-shape change beyond T0DET3's already-disclosed ItemInstanceId field |
| bastion-block-T1CMD | d319508dacb6 | T1-001 group complete (T1.1-T1.11, command/commit/capability protocols — Tier 1's first packet), all self-gated, individually run-logged. Highlights: T1.1 feature-protocol fitness gate; T1.3/T1.10 CommandReceipt admission + 9-state CommandStatus lifecycle; T1.2/T1.4 effect_journal unit-of-work (rejects general 2PC); T1.5 orchestrated conservation_saga; T1.7 DatabaseBatchOutcome; T1.8 BastionCommitQueue; T1.9 3-tier audit_framework; T1.11 server-issued capability grants. **T1.6 = a REAL LIVE BUG FIX**: `execute_character_edit` was committing on FAILED edits — `is_err()` omitted `CharacterEdit(Err(_))`, silently corrupting data on every failed character edit; fixed, `is_err()` helper removed so a future variant can't bypass again. **LIVE WIRE-IN PROOF**: T1.3/T1.10 routed through the haul ItemTransfer completion (Accepted→Executing→Committed) as a PURE REFACTOR — FinalStateCertificate durable_composite byte-identical pre/post, confirmed both locally and at full VM scale (t1cmd: serial + 2 schedule-fuzzed legal schedules, cross-machine, certificate-identical, matching the pre-wire-in T0DET4 baseline exactly). t1cmdfloor canonical (M3A `[66,44,97]`/2 tracked-red, N2 tp1, M3D `[145,44,204]`/2 hold[T,F,F]) — wire-in didn't perturb the ladder fixtures | revert tag bastion-block-T0DET4 (a1130b1c5793) — everything between (T1.1-T1.11 + the wire-in) rides this checkpoint; see the run-log for the granular commit trail | server-internal command/commit/audit/capability substrate + one real bug fix (character-edit rollback correctness) — no save/wire-shape change |
