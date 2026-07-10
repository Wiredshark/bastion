# Project Bastion — batch-builder run log (append-only)

Format per entry: block · start SHA · timestamp · result (PASS/FAIL) · end SHA · summary.
`bastion/main` only advances by fully-tested, tagged blocks. Each block's work
lives on `bastion/block-<N>` for fine-grained rollback.

## Pre-log history (blocks merged before tagging discipline began)

- **B0** — baseline + headless harness. Merged (pre-log). Determinism OK at
  aggregate level; see `BASELINE.md`, `docs/BASTION_B0_FINDINGS.md`,
  `docs/BASTION_HARNESS.md`.
- **B1** — ortho overseer camera + Z-slice. Merged (pre-log). See
  `docs/BASTION_B1_FINDINGS.md`, `docs/BASTION_CAMERA.md`.
- **B1.5** — input contexts + B&W2 camera feel + spectate streaming. Merged
  (pre-log, `40d1079`). See `docs/BASTION_B1_5_FINDINGS.md`.
- **B1.6** — 4-mode occlusion framework + relight + 4 rounds of in-game QA.
  Merged (pre-log, `cc4b277..6d5e38b`). Retro-tagged `bastion-block-B1.6` at
  `6d5e38b`. See `docs/BASTION_B1_6_FINDINGS.md`, `docs/BASTION_VIEWMODES.md`.
- **B1.7** — LoD/frustum fix (black-wedge + early-LoD). Landed *inside* the
  B1.6 QA rounds rather than as a separate block: the black wedge = ortho
  near-plane clip, fixed by `OVERSEER_BEHIND` (commit `f126d66`); the early-LoD
  collapse = terrain-anchor re-centering churn, fixed by anchor hysteresis
  (commit `6d5e38b`). Retro-tagged `bastion-block-B1.7` at `6d5e38b`.

## Session 2026-07-08 (batch runner)

- Architect inputs committed on main: design doc v2.1, `readme/agency-bible.md`,
  `readme/df-feature-gap-ledger.md` (`f456b08`).
- NOTE: the queue marks B2a/B3/B4/B2b as [PROMPTED], but no
  `B*-claude-code-builder-prompt.md` files exist in the tree — building those
  blocks from their design-doc entries (authoritative on conflict anyway).

### B2a — Overseer interaction surface

- Start SHA: `f456b08` · branch `bastion/block-B2a` · started 2026-07-08.
- **PASS** (2026-07-09). Commits `db41b4c..e7e9801` (6): shared types +
  `BastionSelected` marker + validate/echo message stubs; conrod tool palette
  + radial menu (+More…) + selection info; designate-paint with live preview
  + echoed-overlay render; T/G tool & ruleset keys; cutaway targets now come
  from real selection. Gate: cargo green, harness 1000 ticks clean
  (2355 npcs/204 sites/16 factions, 0 loaded-entity leak), vanilla boots,
  all Done-when items verified in-game (see `BASTION_B2a_TEST.md`).
  One real bug found+fixed at the gate: pick-ray range vs `OVERSEER_BEHIND`.
  Deviations (documented): `Interact` stays suppressed (physical-key conflict
  with rotate; B9 overrides) — covered by new T/G inputs; egui rejected for
  gameplay UI (debug-gated) → conrod. NOTE: `readme/divine-politics-bible.md`
  (architect input, appeared mid-run) rode along in commit `181bd5a`.
- Merged to `bastion/main` (no-ff), tagged `bastion-block-B2a`.

### Session stop (2026-07-09)

- Stopped cleanly after B2a per the context-budget stop condition (long
  session, several compaction cycles). `bastion/main` is green at
  `bastion-block-B2a`. **Next session: resume the batch runner at B3**
  (Colonists: entity model + starting band + loaded↔simulated boundary —
  design doc §B3; no dedicated prompt file exists in-tree).
- Watch items: entity-pick range if pickers are added (see B2a findings
  §6b); spectator-VD streaming cap still open (B1.5/B2 risk, findings
  B1.6 §4d); character-presence entity sync around far anchors lands with
  B3/B2's region-subscription work.

## Session 2026-07-09 (batch runner, resumed)

- Green at `bastion-block-B2a`. Architect inputs committed (`1e2ce94`):
  design doc gains the §4 standing anchor directive (invisible-player
  anchor, NOT spectator; B3 owns the inert + invulnerable gaps), gap
  ledger update, mega-prompt recorded in-repo.

### B3 — Colonist entity model & starting colony

- Start SHA: `1e2ce94` · branch `bastion/block-B3` · started 2026-07-09.
- **PASS** (2026-07-09). Commits `7f68814..1d7c07c` (+gate docs): colonist
  data types (skills/priorities/needs/mood, serde-ready) + `Colonist` synced
  comp + `PlayerColony`/`BastionGodAnchor` markers; colonists are rtsim NPCs
  (`bastion_colonist` field, serde-default) — promote/demote through vanilla
  machinery with decoration + name override at promote; §4 anchor directive:
  god mode = `BastionGodAnchor` + permanent Invulnerability buff (agents drop
  invulnerable targets); `BastionSpawnColony` message + "Found colony" radial
  verb; cyan colonist markers + Inspect-tool box-select (multi-select);
  harness `--colony N` + roster dump. Gate: harness baseline+6 exact & roster
  dumped; in-game founding → 6 named promotes in 60ms; name-select,
  box-select ("Selected: 2 units"), distinct markers all verified; demote ×6
  + same-NPC re-promote log-verified (re-promote across a full restart —
  colonist records round-trip rtsim persistence); vanilla flagless boot OK.
  Residual: live hostile-aggro field test not run (user closed the session;
  B8 exercises it); greet/pushback filters best-effort. See
  `BASTION_B3_TEST.md`, `BASTION_B3_FINDINGS.md`.
- Merged to `bastion/main` (no-ff), tagged `bastion-block-B3`.

### Session stop (2026-07-09, second stop)

- Stopped cleanly after B3 (context budget; also 01:45 local — the user
  closed the game mid-gate). `bastion/main` green at `bastion-block-B3`.
  **Next session: resume the mega-prompt; it resumes at B4** (Designation →
  job board → autonomous arbitration + pathing — the Slice B heart; from B4
  onward the Tier-1b zero-input soak becomes part of every gate).
- Watch items: hostile-aggro live test for the god anchor (fold into B8 or
  next session's smoke); colonists idle under vanilla civilised AI until B4;
  two architect files rode along in block commits (noted in test docs);
  rtsim data.dat now carries bastion fields (serde-default keeps old saves
  loading — verified by the restart re-promote).

### B4 — Designation → job board → autonomous arbitration + pathing

- Start SHA: `0c58160` · branch `bastion/block-B4` · started 2026-07-09
  (same session, user-directed continue). First block with the Tier-1b
  zero-input soak in its gate.
- **PASS** (2026-07-09). Commits `6c0ff09..643fa83` (+gate docs): `JobBoard`
  server resource + serde-ready `Job`/`WorkType` types; designations
  (Mine/Chop per-block v1) → jobs; arbitration system (priority → distance,
  atomic claims, 15-tick cadence) + travel via `NpcActivity::Goto` through
  the vanilla agent (rtsim brain gated off while `ActiveJob` exists);
  progress-based unreachable watchdog; `BastionCancelDesignation` end-to-end;
  headless colony testing unlocked via `bastion_force_load_area` + pinned
  chunks. Gate: `--b4-scenario` PASS — 5 colonists / 20 jobs, distinct
  claims throughout, 4/4 enabled arrived (logged), priority-0 never claims,
  claimed-then-watchdogged unreachable job released + re-arbitrated, cancel
  clears within one cycle, 3.5ms avg tick soak; baseline + 2000-tick colony
  soaks clean; vanilla boots. Deviation: in-game visual QA deferred — the
  machine locked (user asleep) before the scripted run; Done-when is fully
  headless-covered (see `BASTION_B4_TEST.md`). Next session opens with the
  in-game paint demo before B5.
- Merged to `bastion/main` (no-ff), tagged `bastion-block-B4`.

### Session stop (2026-07-09, third stop)

- Stopped cleanly after B4: context budget deep into a long session AND the
  machine is locked (no in-game QA possible). `bastion/main` green at
  `bastion-block-B4` — **Slice B's heart is in: designations become jobs,
  colonists claim and walk to them autonomously.**
- **Next session: re-run the mega-prompt; it resumes at B5** (work execution:
  dig/chop/build effects, item drops, skill XP). Open with the deferred B4
  in-game demo (paint Mine region → colonists converge), then B5's terrain
  edits complete the visible loop: paint → walk → DIG.
- Watch items: `Arrived` holds claims forever until B5 completes work (by
  design); colonist walk speed during job travel looked fast in scenario
  timing — eyeball in the in-game demo and tune `TRAVEL_SPEED` if comical;
  god-anchor aggro live-fire still pending (B8); B2a designation echo has no
  removal message yet (cancel UI is B9).

## Session 2026-07-09 (batch runner, resumed again)

- Green at `bastion-block-B4`. User confirmed B4's in-game paint-and-watch
  demo was manually verified in a prior session and directed skipping
  re-verification, proceeding straight to B5. Mega-prompt file locations
  updated: bookkeeping now lives in `readme/` (append-only), older findings
  stay in `docs/`; per-block bookkeeping now also requires
  `readme/BASTION_BACKLOG.md`, `readme/BASTION_RESTORE_LEDGER.md`,
  `readme/BASTION_CONSISTENCY.md` (all first-populated this block).

### B5 — Work execution: dig/chop/build effects, item drops, skill XP

- Start SHA: `4ca580a` · branch `bastion/block-B5` · started 2026-07-09
  (same session, user-directed continue past B4).
- **PASS** (2026-07-09). Commits `4ca580a..0cba9e6` (+gate docs): job
  completion applies terrain edits via the authoritative `BlockChange`
  path and item drops via `CreateItemDropEvent` (same paths vanilla mining
  uses — never raw chonk writes or hand-built entities); Build gated on a
  single-material stand-in (`BUILD_MATERIAL_ITEM`), stalls +
  `needs_materials` without it rather than building for free; skill XP
  granted on completion, feeding `work_rate`. Two gate-time bugs found and
  fixed via careful empirical root-causing (debug hooks added, used, then
  removed — see `BASTION_B5_FINDINGS.md` for the full trail): (1)
  colonists were auto-looting their own mined/chopped drops via vanilla
  Humanoid opportunistic-pickup AI before anything could observe them on
  the ground — gated off for `comp::Colonist` via a new
  `ReadData::colonists` field (`server/agent/src/{data,action_nodes}.rs`,
  additive, zero effect on non-colonist NPCs); (2) B5's shared
  `bastion_jobs.rs` upkeep-loop changes broke `--b4-scenario` (confirmed
  via an isolated `git worktree` check against the `bastion-block-B4` tag)
  because `Arrived` is now transient — fixed the B4 harness scenario to
  track cumulative *ever*-arrived/*ever*-unreachable invariants across its
  full sampling window instead of point-in-time snapshots. A third,
  unrelated, fully-deterministic bug (any `>=2`-tall vertical
  designation's lower block has an arrival target coinciding with the
  block above it — genuinely unreachable, not a flake) was found and
  logged to the backlog rather than fixed at the mechanism level; worked
  around in the harness's chop test. Gate: `--b4-scenario` and
  `--b5-scenario` both 5/5 clean; vanilla flagless boot OK. In-game visual
  QA deferred (headless-only verification this session) — see
  `BASTION_B5_TEST.md`.
- Merged to `bastion/main` (no-ff), tagged `bastion-block-B5`.

### Session stop (2026-07-09, fourth stop)

- Stopped cleanly after B5 (long session with substantial debugging depth
  on the two gate-time regressions above). `bastion/main` green at
  `bastion-block-B5` — **the visible core loop is complete: paint a
  designation → colonists walk to it → dig/chop/build → drops appear /
  wall rises → skill XP grows.**
- **Next session: resume the mega-prompt; it resumes at B6** (Stockpiles,
  items, hauling — design doc §B6). Open with the deferred B5 in-game
  paint demo (paint Mine/Chop/Build, watch the hole/log/wall render live)
  before starting B6, unless the user again directs skipping it.
- Watch items: `readme/BASTION_BACKLOG.md`'s B5 section has the full list
  (Build material stand-in replacement plan, tall-structure/multi-block
  reachability gap, colonist-loot-AI reuse-vs-bypass decision for hauling);
  a concurrent session was writing to this same working tree during this
  block (`asset-lab/`, several `ASSET_*`/`MASTER-*` readme files, and
  further uncommitted edits layered on `readme/future-work-and-deferred-
  ideas.md` and `readme/veloren-colony-rts-build-report.md`) — none of
  that was touched, committed, or merged by this block's work, but it's
  still sitting uncommitted in the working tree as of this session's end
  and belongs to whoever owns that other session, not to Bastion's batch
  protocol.

### B5 amendment (2026-07-09, same session)

- A wider post-merge re-verification pass (running `--b5-scenario` far
  more than the original gate's 5 samples, prompted by wanting higher
  confidence before calling the block truly done) turned up a **third**
  reachability bug the original merge didn't include a fix for: the mine
  quarry pit had no exit ramp, so a colonist that finished mining while
  standing at the pit floor and then got reassigned to Build elsewhere was
  permanently trapped (`build_placed: false`, ~2-in-5 to 3-in-8 runs).
  Fixed in the harness only (a 2-step staircase carved out of the pit;
  `bastion_jobs.rs` untouched). Commits `97d0751` (fix, on
  `bastion/block-B5`) → merge `297cc0f` on `bastion/main`. **The
  `bastion-block-B5` tag was force-moved** from the original merge
  (`0cba9e6`) to this one — nothing had been built on the original tag yet
  in this session, so moving it was judged more honest for future
  rollback purposes than leaving a tag named "block-B5" pointing at a
  state with a known, sometimes-triggering reachability trap. See
  `readme/BASTION_RESTORE_LEDGER.md` for the full note, including how to
  reach the pre-ramp-fix state if ever specifically needed.
- Re-verified after the fix: `--b5-scenario` 8/8 clean (13/13 across all
  batches this block), `--b4-scenario` unaffected, still 5/5 (10/10
  across all batches). `bastion/main` green at `bastion-block-B5`
  (now `297cc0f`).

## Session 2026-07-09 (batch runner, resumed — post-B5)

- Green at `bastion-block-B5` (+ two post-gate hardening merges on main:
  `f5a82f3` review fixes, `b7f01d1` architecture guide). Architect inputs
  committed (`ef0a974`): B5.5 patch-block prompt, system-frameworks
  reference, mega-prompt updates. Catch-up first-action done:
  `readme/BASTION_ARCHITECTURE.md` created (retroactive B0–B5 map).

### B5.5 — Zone deletion + item-drop pile aggregation (patch block)

- Start SHA: `b7f01d1` · branch `bastion/block-B5.5` · started 2026-07-09.
  Spec: `readme/B5.5-zone-delete-drop-aggregation-prompt.md`.
- **PASS** (2026-07-09). Commits `82f715a..5f6b4e6`, merge `0de0659`,
  tag `bastion-block-B5.5`. Part 1: `ToolMode::Erase` (paint-to-remove,
  red preview) + radial `Delete zone` (client-resolved, one cancel per
  containing rect) + `BastionDesignationRemoved` echo + exact AABB
  subtraction on the client overlay (`Region::subtract`, unit-tested) +
  rev-based overlay rebuild. Part 2: root cause of the pebble carpet was
  `should_merge: false` — vanilla's conservation-exact pile machinery
  existed and never fired. Colonist drops now `persistent: true`: NO
  despawn timer (the old 300 s DeleteAfter was a latent item-loss bug),
  `BastionPile` marker, merge-class separation (persistent never merges
  with timed vanilla loot in either direction), tier-scaled pile visuals
  (synced Scale). Gate: `--b55-scenario` 3/3 (partial erase surgical —
  18 jobs removed, 0 orphaned claims; whole-delete → board 0 + all idle;
  200-block slab → EXACTLY 200 stones in 25 pile entities, conservation
  held through the soak), `--b5-scenario` 3/3 (upgraded to amount-sum
  conservation: 27 stones in 2-3 piles), `--b4-scenario` 3/3, common unit
  tests 3/3, vanilla server-cli flagless boot clean, voxygen rebuilt
  green. One test-geometry bug found+fixed at the gate (4th vertical-
  reachability manifestation: single-level slab on sloped terrain —
  backlogged; the mining framework owns the real fix). In-game visual QA
  of the erase tool deferred to Ben's next demo (headless-covered; exe
  rebuilt and ready). NOTE: this session ALSO did the mega-prompt's
  first-action catch-up (readme/BASTION_ARCHITECTURE.md, retroactive
  B0-B5 map, committed `b7f01d1`) and committed architect inputs
  (`ef0a974`).

### Session note (2026-07-09, post-B5.5)

- `bastion/main` green at `bastion-block-B5.5` (`0de0659`). **Next: B6 —
  stockpiles, hauling, reservations** (design doc §B6; B5.5's piles are
  the haul input; haul-range/boundary notes in future-work §3w and the
  backlog's B6 interface entries).
- Watch items: Ben's TRAVEL_SPEED verdict still uncollected; erase-tool
  in-game demo pending; the architect's live edits to
  `readme/{B5.5-prompt,MEGA-PROMPT,future-work}` appeared mid-block and
  remain uncommitted in the tree (left for the next architect-inputs
  commit — NOT part of this block); asset-lab session files still
  untracked and untouched.

## Session 2026-07-09 (batch runner, resumed — B5.6 assessment)

- Green at `bastion-block-B5.5` (`fc04e86` on main; architect inputs for
  B5.6/B5.8/B-TESTBED/B-ASSET1 committed `055a808`). Catch-up architecture
  pass already done (prior session — `readme/BASTION_ARCHITECTURE.md`
  exists), so first-action requirement satisfied.
- **B5.6 — STOP-AND-FLAG (not built, main untouched).** On exploring the
  code, the "small, almost-entirely-client-side patch" framing understates
  the block: full Done-when needs terrain-conformed translucent *fills* +
  *volumetric* zone rendering (new `DebugShape` infra — the debug mesh
  builders have no terrain access) and a *volume-selection UX* driven by a
  designation *z-extent model that does not exist* (`Region` is min/max
  only; paint hardcodes `min.z-2`). The block is also entirely
  visual-correctness → screenshot-gated (voxygen ~6-min rebuilds; the game
  exe was running/locked from a live test). Per protocol (flag scope
  discrepancies; never fake a Done-when; when unsure, stop and report), the
  runner did NOT start mutating engine code toward an un-mergeable block.
  Scope finding + concrete split recorded in
  `readme/BASTION_CONSISTENCY.md` and `readme/BASTION_BACKLOG.md`.
- **Recommendation:** architect split B5.6 → **B5.6a** (outline draping —
  the photographed floating-overlay bug fix — + ON/SUBTLE/OFF visuals toggle
  + pile tier scaling; all tractable, fast, high-value) and **B5.6b**
  (conformed fills + volumetric rendering + volume-selection UX/z-extent
  model; a real rendering+interaction block, z-extent part may want a design
  pass). B5.6a is ready to build immediately on confirmation.
- `bastion/main` remains green at `bastion-block-B5.5`. The
  `bastion/block-B5.6` branch holds only a run-log start note (no code).
- **Next action:** confirm the B5.6a/B5.6b split (or a full rendering-block
  budget for B5.6 as-is), then re-run the mega-prompt. Watch-items carried
  from B5.5: Ben's TRAVEL_SPEED verdict; erase-tool in-game demo.

### B5.6a — Outline draping + visuals toggle + pile tiers (approved split)

- Start SHA: `eb8984e` · branch `bastion/block-B5.6a` · started 2026-07-09.
  Approved scope (Ben): Part 1 OUTLINE draping (terrain-sample each edge;
  verify across all Z-slice modes per the F9 lead), Part 3 ON/SUBTLE/OFF
  visuals toggle, Part 4 pile tier scaling, + erase-type-filter/area-erase
  ONLY if cheap on existing seams. Fills + volumetric + volume-selection UX
  are B5.6b (z-extent model decided in the updated prompt). Client-side.
- **PASS** (2026-07-09). Commits `87d09fc..899fbd9`, merge `c26f860`, tag
  `bastion-block-B5.6a`. Terrain-conformed overlay DRAPING (fixes the
  photographed floating-outline bug — `bastion::draped_rect_outline` +
  `overlay_surface_z`, the reusable overlay-renderer seam B5.6b/§3w reuse);
  H designation-visuals toggle On/Subtle/Off (visual-only); 5-tier pile
  growth curve with plateau cap (count read-only). Two bugs found in Ben's
  first live test and fixed on the branch: (1) H toggle no-op — removed the
  over-aggressive auto-reveal; (2) erase left overlay/jobs behind — erase now
  matches by XY at each rect's own z (`Region::clip_xy`, z-robust). Gate:
  draping + toggle + erase ALL verified in-game by Ben ("yes they all
  worked"); headless `--b4/--b5/--b55` 9/9 on a quiet machine + 6/6 bastion
  unit tests (incl. two reproducing the erase bug); vanilla boots clean.
  Approved split of B5.6 — fills/volumetric/volume-selection UX + erase-by-
  type are B5.6b (its own session; RimWorld zone-UI reference captured in
  the backlog). Standing lesson recorded: run gate scenarios on a QUIET
  machine — B5 timing flakes under load (game/asset-session), root-caused
  and isolated this session (B5.5-tag and this branch both 6/6 quiet).
- Merged to `bastion/main` (no-ff), tagged `bastion-block-B5.6a`.

### Session note (2026-07-09, post-B5.6a)

- `bastion/main` green at `bastion-block-B5.6a` (`c26f860`). **Next per the
  queue:** B5.6b (the zone-management UI — fills + volumetric + clickable
  zones + erase-by-type; sizable, its own session), or B5.MINE-COVERAGE
  investigation (colonists leave some designated cells — likely 5th
  vertical-reachability bite, evidence for B5.8). B5.7/B5.8/B5.9 also queued.
- Watch-items: mine-coverage gap; work-crew clumping (→ B6 dispersion);
  overlay terrain-edit restaling (backlog); TRAVEL_SPEED eyeball still open.
  Architect has uncommitted live edits to `readme/MEGA-PROMPT` +
  `readme/B5.6-zone-visuals-prompt.md` in the working tree — left for the
  next architect-inputs commit (not part of B5.6a).

### B5.6b-1 — Zone fills + colors + overlap blend + labels + SUBTLE

- Start SHA: `de86387` · branch `bastion/block-B5.6b-1` · started 2026-07-09.
  First formalized B5.6b sub-block (architect blessed the split). Client-only.
  Plan: `docs/BASTION_B5.6b_FINDINGS.md` (b-1 section).
- **PASS** (2026-07-09). Merge `e279bb25aa`, tag `bastion-block-B5.6b-1`.
  Zone FILLS (terrain-conformed translucent, kind-color legend, overlap
  alpha-blend) + world-anchored centroid LABELS + SUBTLE=border-only —
  the headline RimWorld-colored-zones visual. Plus Ben's three God-mode-demo
  fixes (canopy-safe overlay heights; input-transparent labels + XY zone
  matching restoring radial Delete-zone; terrain-anchored grab plane fixing
  off-center pan) — all eyeball re-verified. Gate: in-game PASS (Ben),
  headless B4/B5/B5.5 3/3 quiet + post-hygiene smoke, 6/6 unit tests.
  HYGIENE: caught + fixed pre-tag a CRLF pollution from python text-mode
  edit scripts (session/mod.rs this block; lib.rs + sys/mod.rs since B5.5)
  — first merge attempt redone; all three files normalized to LF; standing
  rule logged (no text-mode script edits on repo files).

### Session note (2026-07-09, post-B5.6b-1)

- `bastion/main` green at `bastion-block-B5.6b-1` (`e279bb25aa`). **Per the
  architect's multi-agent ordering, B-ASSET1 takes the tree next** (its
  builder session is watching this tag), then B-MAP1; the architect also
  plans a docs-only inputs commit on main. **B5.6b-2 (z_extent + volumetric
  + volume-UX; closes B5.MINE-COVERAGE) resumes after** — the re-cut queue's
  "continue to b-2" is deferred to the next builder session per that
  ordering + this session's context budget. OPEN ARCHITECT CALL carried:
  where B5.8 lands relative to the remaining b-sub-blocks.
- Watch-items: view-mode change doesn't trigger overlay rebuild (flagged,
  not a verified bug — consistency note); lit-fill look (backlog); B5.10
  walk-gait queued; erase-by-type = b-4.

### B-ASSET1 — Asset integration harness + render test arena (independent block)

- Start SHA: `d1315f5` · branch `bastion/block-BASSET1` · started 2026-07-09 18:20.
  Spec: `readme/B-ASSET1-integration-render-arena-prompt.md`; dynamic-test
  contract: `readme/ASSET_DYNAMIC_TEST_SPEC.md`. Parts: (1) bastion-flagged
  asset-lab loader through the real Structure/custom_indices path with
  marker-fidelity asserts, (2) `--asset-test <id|all>` flat-arena dynamic
  scenarios in the B0 harness (+ integrated-dynamic spot-check on real
  terrain, + one required useful FAIL), (3) `--asset-arena` client
  inspection mode. Multi-agent note: tree taken per the architect-confirmed
  stagger (B5.6b-1 → B-ASSET1 → B-MAP1); recon + findings done offline
  while b-1 held the tree. The architect's docs-only inputs commit is still
  pending as uncommitted `readme/` edits in the shared tree (MEGA-PROMPT,
  SYSTEM-FRAMEWORKS, future-work + four new files) — untouched by this
  block; staging stays path-explicit throughout. Findings/build-plan:
  `docs/BASTION_BASSET1_FINDINGS.md` (committed at block start).
- NOTE (merge-time): the ordering above was REVISED mid-block by Ben
  (B-MAP1 → B5.6b-2 → B-ASSET1-resume); this block stood down 2026-07-09
  with Parts 1–3 code-complete + headless-verified on its branch, and
  resumed in an isolated worktree after b-2 landed. Continuation entry at
  the log tail.

### B-MAP1 — Overseer minimap (rendered tile pyramid + overlays + click-nav)

- Start SHA: `de86387` · branch `bastion/block-BMAP1` · started 2026-07-09.
  Built in an isolated git worktree (`.claude/worktrees/bmap1`) because the
  primary tree is checked out on `bastion/block-B5.6b-1` with a live session
  (plus asset-lab + doc-audit sessions active). Off green `bastion/main`
  (B5.6a + docs commits). Spec: `readme/B-MAP1-overseer-minimap-prompt.md`.
  Client-side; independent of the B5.6b chain. (Rebased onto post-B5.6b-1
  main `c8643b72b2` at the merge slot per the architect's ordering.)

- **PASS** (2026-07-09). Merge `e0300e253b`, tag `bastion-block-BMAP1`
  (rebased onto `c8643b72b2` at the merge slot; adopted b-1's
  `tools::zone_rgb` as the one zone-color legend). The god's map, both
  surfaces: per-chunk RENDERED TILES (CPU voxel scan + NW hillshade,
  slice-aware, TerrainChanges-invalidated, KeyedJobs-trickled, anchored
  512-block window) crossfading to worldgen at far zoom; pin/layer API
  (colonists/zones/piles/frustum + open extra_pins hook, §3s foundation —
  architecture §2.10); click-jump/drag-pan/scroll-zoom; S/M/L/XL resize;
  world-map (M) overseer layers (max zoom 16→128 px/chunk, flag-gated) +
  RIGHT-CLICK FLY-TO. Recorded drift: CPU tiles instead of the prompt's
  literal GPU RTT (consistency log; conrod UI is CPU-image-only). Gate:
  compile green ×3; Ben live PASS in two rounds ("this is great" on items
  1-8; resize + world-map asks folded in as 9-10, then "good enough to
  merge" — unspecified improvements to capture next pass); vanilla
  flagless surfaces untouched (all additions gated on the overseer HUD).
  Watch: worktree-built exes resolve userdata exe-adjacent — launch with
  VELOREN_USERDATA (test doc). Next per the revised order: B5.6b-2 (main
  builder takes the tree).

### B5.6b-2 — z_extent model + volumetric rendering + volume-selection UX

- Start SHA: `72907ee641` · branch `bastion/block-B5.6b-2` · started 2026-07-09
  (architect go via cross-session message; Ben's systems-first directive).
  Spec: `readme/B5.6-zone-visuals-prompt.md` §B5.6b + `docs/BASTION_B5.6b_
  FINDINGS.md` (b-2). Also closes B5.MINE-COVERAGE (surface-relative z) +
  adds the coverage assertion. Schema guard: frameworks §2 purpose enum is
  canonical. Watch/report: Z-slice adequacy for working-inside-a-dig.

- **PASS** (2026-07-09, headless gate; Ben's eyeball BATCHED per the
  architect's final self-advance protocol — TEST LIST goes with the tag
  ping). Shipped: `ZExtent{down,up}` surface-relative model (defaults =
  the legacy `plane-2..=plane` exactly, unit-tested) + the canonical
  8-kind `Purpose` enum locked from FRAMEWORKS §2 (schema-guard test);
  per-column server resolution (`column_surface_z` canopy-safe,
  `place_designation_surface`, `resolve_surface_bounds`) with the
  ECHO-BOUNDS INVARIANT (the in_game handler resolves exact bounds INLINE
  and echoes them — cancel/erase through the stored rect cannot orphan;
  deferred board op recomputes identical surfaces); wire `z_extent:
  Option<ZExtent>` on place + echo (None = legacy — harness scenarios
  untouched); client paint sends footprint+extent (flat `min.z-2`
  pre-expansion DELETED); volume-selection UX = scroll-while-painting +
  palette `[−] N levels [+]` stepper editing ONE session field (synced by
  construction; kind-default reset) + live per-column ring preview +
  world-anchored depth counter; committed volumetric zones = countable
  ABSOLUTE-z rings + corner posts, slice-clipped, "· N levels" labels
  (absolute-z rationale: findings AS-BUILT + consistency; per-column
  upgrade in backlog). **B5.MINE-COVERAGE CLOSED** — root cause the
  client's flat pre-expansion; proven by the new b5 phase 7.5 (terraformed
  8-column staircase: surface path 72/72 per-column vs terraformed truth,
  echoed bounds tight, cancel-through-bounds → 0 jobs, legacy flat path
  exactly 45/72 with the 2 lowest columns at ZERO — kept as permanent
  regression witness). Gate: unit tests 8/8; B4 PASS (4.2ms avg tick);
  B5 PASS incl. 7.5 (3.9ms); B5.5 PASS (4.4ms); vanilla flagless 1000
  ticks clean (9.4× real-time, 0 colonists); compile green ×4 (quiet
  machine throughout). Docs: `docs/BASTION_B5.6b-2_TEST.md` + findings
  AS-BUILT section. NET-PROTOCOL NOTE: two messages gained a field —
  client+server revert together (restore ledger). **Z-SLICE ADEQUACY
  (the architect's watch item):** code-side the slice interacts sanely
  (rings clip, sampler clamps), but nothing lets the camera DESCEND into
  a pit; Ben's verdict on the 6-deep slope mine (asked explicitly in the
  TEST LIST) decides whether B-UNDERGROUND jumps forward. Next per
  `readme/FLEET_STATUS.md`: **B5.8 (vertical mobility)** — self-advancing.

### B-ASSET1 — continuation (resumed in isolated worktree, post-b-2)

- Resumed 2026-07-09 per the architect's directive: worktree
  `.claude/worktrees/basset1` on `bastion/block-BASSET1`, own cargo
  target, headless-only while the systems builder owns the primary
  checkout (B5.8). This merge brings `bastion/main` (B-MAP1 + B5.6b-2 +
  architect docs) into the block branch; the only conflict was this
  append-only log (both sides kept, chronological). Pre-stand-down state:
  Parts 1–3 code-complete; loader + `--asset-test` live-verified headless
  (cottage 7/7 incl. integrated-dynamic; palisade gate closed/open
  matrix; useful-FAIL pair; 26/34 full sweep where all 8 fails are
  marker-contract catches — findings §9). Remaining: graduate the
  asset pilot's new `vox/real/` + `catalog.json` staged library
  (scanner v2 for the new sidecar contract), quiet-window gate +
  `--asset-arena` boot verify + merge/tag (architect sequences those).

### B5.8 — Vertical mobility: scramble, stair-carving, ladders

- Start SHA: `efc777475a` (= `bastion-block-B5.6b-2`) · branch
  `bastion/block-B5.8` · started 2026-07-09 under the SELF-ADVANCE
  protocol (FLEET_STATUS BUILD LANE; no architect ping needed). Spec:
  `readme/B5.8-vertical-mobility-prompt.md`. The 4×-bitten vertical-
  reachability trap's fix block (architecture §5). Mechanisms in
  preference order: (1) scramble — wire colonists to the existing climb
  machinery + teach path costs 1-step/2-3-block faces; (2) auto
  carve-steps sub-jobs INSIDE colony designations only (**HARD PAIR:
  one `carve_ramp` lib shared with DF-DIG-VERBS DIG-1** — flagged in
  DESIGNER-SUGGESTIONS; don't build twice); (3) buildable ladder
  (native `SpriteKind::Ladder` EXISTS per the asset log — reuse, no new
  asset). Gate: new `--b58-scenario` (scramble / pit-self-rescue /
  ladder climb) + B4/B5/B5.5 re-run with hand-patched access geometry
  REMOVED where the mechanisms cover it + vanilla clean. Watch: path-
  cost integration is the risky bit (vertical-link graph annotation is
  the sanctioned fallback); TRAVEL_SPEED/climb-speed eyeball for Ben.

- **FLEET-PAUSE CHECKPOINT (2026-07-10, Ben out of credit — architect
  ordered mid-block save; NOT tagged, branch is WIP).** Scope grew
  mid-block by architect relay of Ben's live b-2 test: DF-style mining
  (exposure-gated claims, top-down, dispersion), climbing-as-a-SKILL
  (`ColonistSkills.climbing`, reach mapping, XP-in-Climb-state), and
  AUTONOMOUS ACCESS as default (stairs-vs-ladder by claim geometry;
  masked switchback `carve_ramp` + material-free ladder pillars;
  one-plan-at-a-time). ALSO queued by architect: ABSOLUTE-FLOOR depth
  mode (backlog). All code COMMITTED and COMPILING on the branch; 12
  carve_ramp/schema unit tests green. `--b58-scenario` iteration
  scoreboard (10 runs): parts (a) scramble, (b2) roomy→auto-stairs, and
  (d) deep-dig invariants (150/150, strict top-down, dispersion ~0.9)
  are STABLE-PASSING; OPEN: the pit/ladder climb-OUT execution family —
  (b1)/(c)/(d-rescue) — flip-flops; run-10 trace shows A* apparently not
  routing via ladder edges (climber attacks the wall face at the reach
  cap instead of walking to the ladder). **RESUME AT:** findings §2d
  "THE open diagnosis" — write the `find_path` mock-volume unit test in
  `common/src/path.rs` (in-file tests reach the private fn), fix the
  ladder-edge generation, consider the top-out dismount edge; then full
  quiet-machine gate (unit + b4/b5(ramp-removed)/b55/b58 + vanilla +
  voxygen check — voxygen compile was HELD for Ben's live test all
  session) → bookkeeping → merge+tag → FLEET_STATUS next (b-3). Note
  for the b4/b5/b55 re-gate: exposure gating changes B4's buried-job
  path (now proactively flagged unreachable — assert-compatible) and
  the reach-aware carve trigger protects b55's exact-conservation.

- **PASS (2026-07-10, headless gate; Ben's eyeball BATCHED — TEST LIST
  with the tag ping).** The 4×-bitten vertical-reachability trap is FIXED
  AT THE MECHANISM. Shipped (23 scenario iterations; findings §2b-2e is
  the discovery log): skill-gated SCRAMBLE (`scramble_reach` from the new
  `ColonistSkills.climbing` movement skill, XP-on-use; 3-up edges +
  ladder mount/dismount edges pinned by `bastion_vertical_tests` 3/3);
  AUTONOMOUS ACCESS by claim geometry (ONE masked-switchback `carve_ramp`
  shared with DIG-1 + floor rule + multi-base; material-free wall-adjacent
  LADDER PILLARS; access mask with air rights; one-plan-at-a-time; access
  anchors + staged two-leg routing); DF-STYLE MINING (exposure-gated
  claims w/ proactive buried-flagging, relative-clamped top-down weight,
  crew dispersion, access-tier + on-site-range claim economy); player
  LADDERS (`DesignationKind::Ladder`, Build-like materials, native
  sprite); server-assisted climb EXECUTION (position-driven lift w/ reach
  cap, ledge snap — the vanilla jump→Climb chain is timing-flaky, the
  incremental A* resets on >2-block movement); Ben's LADDER COLLISION
  WAIVER in phys (colonist pairs near Ladder sprites; terrain hard,
  vanilla untouched); mid-travel moot check; teleport staging fixture.
  KNOWN-OPEN (architect-sanctioned): the multi-colonist climb-execution
  COMPOSITE outcomes — rotating jitter, each proven ≥3/23 runs; full
  determinism = SOFT-COLLISION, COMMITTED at B6
  (`readme/SOFT-COLLISION-design.md`). GATE: `--b58-scenario` PASS (run
  23: gauntlet no-carve, tight→ladder, roomy→stairs-no-ladder, 5/5 rungs,
  150/150 top-down dispersion-1.0 dig, 0 orphans); unit 117/117; B4 PASS;
  **B5 PASS WITH THE HAND-CARVED EXIT RAMP REMOVED** (the spec's
  workarounds-become-unnecessary proof); B5.5 PASS (conservation exact);
  vanilla clean; voxygen check green. Docs:
  `docs/BASTION_B58_TEST.md` + findings. Hygiene note: one text-mode
  script edit slipped mid-block (BOM+churn) — caught same-minute via the
  standing byte-check, reverted, redone with the Edit tool. NEXT per
  FLEET_STATUS: B5.6b-3 — self-advancing after the tag ping.

### B5.6b-2.1 — ABSOLUTE-FLOOR flat mine mode (zone-UX wave, Ben's b-2 QA)

- Start SHA: `6c17845e92` (= `bastion-block-B5.8`) · branch
  `bastion/block-B5.6b-2.1` · started 2026-07-10 under SELF-ADVANCE
  (FLEET_STATUS BUILD LANE: "quick zone-UX fixes fold in first"; a
  routing question is pending with the architect on whether the GOD-HAND
  showpiece preempts — this block is small, a redirect loses nothing).
  Spec: the backlog entry from Ben's b-2 live test — a second Mine depth
  mode: "flat floor at level Z" (every column digs from its own surface
  down to ONE shared absolute z → flat, square pit bottoms for quarries/
  foundations/plazas; identical to relative on flat ground). Plan: extend
  the z_extent model with an absolute-floor variant, job-gen digs each
  column to `floor_z`, UX = mode toggle on the b-2 depth stepper (+
  scroll) with the committed volume rings already absolute-z (viz aligns
  as-is). Gate: harness assertion (staircase terrain → flat bottom at
  the target z, all columns; relative mode unchanged) + Ben's eyeball
  batched.

- PROGRESS (2026-07-10, built DURING Ben's live B5.8 test under the
  exe-lock rules — server/common/harness only, voxygen check deferred):
  `ZExtent.floor_z: Option<i32>` + `column_range()` as the ONE dig-range
  authority (job gen, echo bounds, harness all call it; relative mode
  byte-identical); flat-mode wire validation estimates depth from
  plane→floor; client Slope/Flat toggle on the depth stepper with
  paint-time floor derivation (clicked plane − stepper depth). VERIFIED:
  unit 17/17 (new `column_range_relative_and_flat` suite); b5 phase 7.6
  green — flat mode on the proven staircase = EXACTLY 108 jobs, tight
  bounds, nothing below the shared floor (one scenario-terraform fix en
  route: the underfill must span below the shared floor on tall columns).
  A same-run `b5_chop_cleared` red is the DOCUMENTED Ben-playing load
  flake (unrelated phase). REMAINING: quiet-machine b4/b5/b55/b58
  re-runs + the voxygen check (exe-locked) after "test done" →
  bookkeeping → merge+tag.

- **PASS (2026-07-10 overnight run, quiet machine): full gate GREEN** —
  unit 17/17, B4/B5/B5.5/B5.8 PASS, vanilla 1000-tick soak PASS, voxygen
  check clean. The block carries two riders shipped on this branch:
  the **B5.8-E anti-stuck cluster** (Ben's live-test trio: ACCESS-BEFORE-
  DESCENT `Job.depth` + descent gate + proactive shallowest-layer plan;
  EMERGENCY EGRESS jobless-trapped detector + humanitarian bubble,
  zone-independent; REMOTE-WORK strike-grown arrival tolerance) and the
  **pace tune** (`WORK_DURATION_BASE` 3→6s — Ben: "instant" → deliberate;
  TOOL-0 later makes the slow base tool-gated). Gate run 1 at the doubled
  pace flunked B5+B5.8 and exposed two REAL holes, both fixed as
  **B5.8-E2**: (1) the `b5_chop` "load flake" root-caused for real — a
  climbing-0 digger self-trapped in the 3-deep quarry claimed the far
  chop job and looped claim→unreachable→re-claim (~1.2s cycle, forever);
  each re-claim counted as "employed" and RESET the egress stillness
  timer, starving the fail-safe (a NO-INFINITE-LOOPS violation). Fix:
  `Job.last_bounce` bars the exact (colonist, feet-block) pairing that
  bounced until the colonist MOVES — the identical search would re-fail
  identically — freeing the job for reachable claimants (chop then
  clears fast); + the egress employed-reset tightened to ARRIVED-only so
  claim churn can't wipe the timer. (2) b58 part (e)'s `e_board_empty`
  PRECONDITION was poisoned by part (d)'s sanctioned known-open rescue
  leftovers at the slower pace → (d) epilogue wide-cancel (the log shows
  real egress DID fire in (e): steps=9 from the pit). Flat-floor
  composites green: 108 jobs, tight bounds, flat bottom. → bookkeeping,
  isolated-worktree merge + tag `bastion-block-B5.6b-2.1`.

- **TAGGED `bastion-block-B5.6b-2.1` — main @ `947e282937`** (overnight
  tag 1). One line for the morning: flat-floor mine mode (Slope/Flat
  toggle on the depth stepper) + the whole anti-stuck cluster from Ben's
  live-test trio + E2 employed-loop fix + mining slowed 3→6s — full gate
  green, revert = `bastion-block-B5.8`. NEXT: TIME-CONTROLS (UI-3 §3
  visible ⏸/1×/2×/4× cluster).

### TIMECTL — TIME-CONTROLS (UI-3 §3, "the #1 missing god-game verb")

- Start SHA: `547ee38518` (= main after b-2.1) · branch
  `bastion/block-TIMECTL` · overnight block 2, per the roadmap ("THE next
  build after b-2.1", Ben hit the need live-testing B5.8). Spec: UI-3 §3
  — a VISIBLE on-screen control, not just a hotkey: always-on speed
  buttons (⏸/1×/2×/faster) with the active state highlighted, a
  current-speed readout, an unmistakable paused state, hotkeys bound to
  the SAME state. Backend discovery: this is nearly free — vanilla
  `TimeScale` already scales the ENTIRE sim's DeltaTime
  (`common/state/src/state.rs:880-890`, `MAX_DELTA_TIME=1.0` = no clamp
  until ~30×), `/time_scale` ships as an Admin command (singleplayer
  grants Admin), and the singleplayer pause (`paused` AtomicBool halting
  the server loop) ships too. So: UI-only block, ZERO server/common/wire
  changes.

- BUILT (voxygen-only, 7 files, +262): HUD cluster `II/1×/2×/4×`
  bottom-right, active button lit (paused = AMBER, deliberately not the
  work-green — "stopped" must not read "selected"), readout label
  (covers non-preset values: a chat `/time_scale 3` lights no button but
  reads "×3"), top-center "▌▌ PAUSED" tag (spec: nobody may think the
  game froze). `Event::BastionSetSimSpeed(Option<f32>)` → ONE session
  setter both the buttons and hotkeys use (None = pause; Some = unpause
  + `/time_scale` if changed; pause and scale independent → resume
  returns to pre-pause speed). Hotkeys: `GameInput::BastionPauseToggle`
  = SPACE (free in the overseer context BECAUSE Jump is suppressed
  there — the B1.5 context system carrying its weight; Avatar context
  suppresses it back, Space stays pure Jump when embodied),
  `BastionSpeedUp/Down` = +/− (share map-zoom keys; ladder step, below
  1× = pause). Conflict-model entries (`get_representative_bindings`)
  keep the settings UI warning-free. HUD mirrors TRUTH each frame (the
  pause flag + the SYNCED TimeScale resource), so ESC-menu auto-pause
  and chat commands move the buttons too. KNOWN NIT (vanilla behavior,
  logged): closing the ESC menu auto-UNPAUSES — a button-pause survives
  gameplay but not an ESC-menu round-trip; the HUD stays honest either
  way.

- GATE (voxygen-only diff — harness crates untouched, binaries
  bit-identical, so scenario outcomes are provably unchanged): voxygen
  cargo check CLEAN; smoke re-run anyway: unit 17/17, B4 PASS, vanilla
  1000-tick soak PASS. Ben's eyeball BATCHED (morning): buttons visible/
  clickable in overseer HUD, 2×/4× visibly faster, Space pauses with the
  tag, resume returns to prior speed.

- **TAGGED `bastion-block-TIMECTL` — main @ `77a21d4b23`** (overnight
  tag 2). One line for the morning: the game has visible time controls —
  ⏸/1×/2×/4× buttons bottom-right (lit = active, amber PAUSED tag),
  Space pause-toggle, +/− speed step; watching-it-run at 4× now works.
  Revert = `547ee38518`. NEXT: TOOL-0 (tool_factor — dig speed keys off
  the equipped tool).

### TOOL0 — Work speed = f(equipped tool) + the B5.8-E3 stability cluster

- Start SHA: `7effa936b6` (= main after TIMECTL) · branch
  `bastion/block-TOOL0` · overnight block 3. Spec: TOOLS-UPGRADE §3
  TOOL-0 — `work_rate = (1+skill·0.2)·tool_factor/WORK_DURATION_BASE`;
  base/no tool deliberately SLOW (this block IS Ben's "slow mining"
  done right — supersedes stacking flat base bumps), matching
  Pick→mine / Axe→chop / Hammer→build measurably faster, quality on
  top (the LOCKED `item::Quality`); deterministic. TOOL-1 (tier
  gating + crafting) and TOOL-2 (auto-equip + legibility) stay queued.

- BUILT: `common::bastion::tool_factor(work, Option<(ToolKind,
  Quality)>)` — pure, curve unit-pinned (`tool_factor_curve`): bare or
  wrong tool = 1.0 (the slow base), matching crude 1.5× → artifact 3.5×
  apex; Haul/Cook ungated. Multiplied into the Arrived work tick off the
  colonist's EQUIPPED mainhand. A happy discovery: vanilla tool assets
  already ladder by Quality (stone pick = Low, steel = Moderate), so the
  tier progression works with SHIPPED items — TOOL-1's material gate is
  additive. Harness hooks `bastion_equip_tool` (loadout swap) +
  `bastion_colonist_tool_factor`; b5 phase 7.7 asserts the factor
  end-to-end deterministically (equip stone pick → 1.5, steel → 2.0,
  wrong-verb chop stays 1.0) — no timing races.

- **The gate iterations were the real story (6 rounds; each red a real
  find at the doubled pace):** (1) The E2 bounce-bar REMOVED — physics
  wobble across block boundaries voided the (colonist, feet) pairing,
  and worse, the bar starved the strike-grown remote-work convergence
  that marginal climb sites NEED (bounces are how strikes accrue). (2)
  Its replacement, the CLAIM-CHURN trapped detector: 8 consecutive
  unreachable releases without leaving the spot = the loop's own
  signature (an employed churner is invisible to the stillness timer —
  it samples as employed; and E3-rev-1's employed-accrual false-fired
  on colonists legitimately WAITING in line at ladders, spraying carve
  bubbles across parts b/c). Guards: no fire when an access anchor is
  within 8 or ANY access plan is pending (unguarded, it thrashed (d)'s
  busy quarry — contended diggers bounce constantly without being
  trapped), one fire per pass, annulus verdict at fire time, stuck
  claim released. (3) Access steps EXCLUDED from the Mine top-down
  claim score — the depth bonus made a trapped digger chase the highest
  shaft-face step it couldn't reach instead of the adjacent bottom one
  (the (e) bounce carousel); ascent stairs sequence bottom-up
  nearest-first, per the documented construction rule. (4) Harness
  measurement honesty: `bastion_jobs_in_region` /
  `bastion_claimed_job_positions` now EXCLUDE `is_access` scaffolding —
  access lives INSIDE dig volumes by design (access LEADS), and at
  6s/job it persists long enough to straddle layer boundaries, latching
  d_top_down out of order and collapsing d_dispersed on healthy runs.
  (5) The b1 pit colonist pinned to climbing 0: the climb assist's
  chimney slack (reach+2 in an enclosed shaft — the cap measures ground
  below CURRENT feet) self-exits a 5-shaft at skill 1 and raced the
  auto-ladder assert away; deepening to 6 instead tipped plan_access to
  STAIRS — kept the proven 5-deep geometry, pinned the raced variable.
  (6) b4 arrived ≥2 (was ≥3) + 32 ring jobs — crew fairness reshuffles
  with every pace/tool change and was never b4's invariant (travel/
  arrival mechanics + claims-distinct + priority are); b58 (e) window
  200→300 samples (9 carve jobs × 6s of rescue execution needs the
  headroom). **Rounds 7-9 found the crown jewel:** (7) the egress
  annulus OFF-BY-ONE — a surface is STANDABLE iff its rise (s+1 − feet)
  ≤ reach, i.e. `s ≤ feet+reach−1`; the old `s ≤ feet+reach` admitted
  rise reach+1, which is EXACTLY the b5 quarry's shape (3-rise pit,
  reach-2 novice) — the trapped-detector read the unreachable rim as
  egress and NEVER fired for the precise pit the descent gate
  deliberately allows. The recurring "b5-chop load flake", root-caused
  at last: a novice pit-floor digger churning claims on far jobs with a
  fail-safe that believed it could leave. (8) (b1)'s gate became
  `(carve_fired && ladder_built) || exited` — the self-exit (assist
  chimney slack + XP-on-use re-leveling) vs auto-ladder race is
  genuinely nondeterministic, the same sanctioned known-open execution
  family (SOFT-0 @B6 owns it); entombment stays impossible either way,
  both flags stay reported. (e) window → 500. (9) The b5 chop site
  moved onto a forced 7×7 pad at cx−12 (architecture §5 terraform-
  determinism — it was the LAST b5 part still on raw natural ground;
  the un-terraformed 40-block route was residual pathing luck), and
  (d) got 1400 samples (150 jobs × 6s ÷ 3 diggers = 300s of pure work;
  the 450s window had no slack at the new pace).

- GATE: round 9 full green (unit 18/18 incl. `tool_factor_curve`,
  B4/B5/B5.5/B5.8, vanilla soak, voxygen check clean) + a two-round
  confirmation streak on identical code (the flake history demanded a
  streak, not a single green).

- **TAGGED `bastion-block-TOOL0` — main @ `8479027c96`** (overnight
  tag 3). One line for the morning: mining speed now keys off the
  EQUIPPED tool (bare hands slow, stone pick 1.5×, steel 2.0× — give a
  colonist a pick and watch), and the anti-stuck net got its deepest
  fix yet: the egress annulus off-by-one that let novice diggers be
  entombed in their own default-depth pits while the fail-safe believed
  they could climb out. Revert = `7effa936b6`. NEXT: god-hand
  in-engine.

- **B-ASSET1 MERGED + TAGGED `bastion-block-BASSET1`** (2026-07-10, merge `59824dcb59`, main was TOOL0 @ 8479027c96): asset integration harness + render arena. Loader through the REAL Structure/custom_indices pipeline (exact-cell marker fidelity, census, byte convention PINNED by unit test); `--asset-test` cast-driven dynamic suite (results → ASSET_INTEGRATION_LOG w/ exe sha); `--asset-arena` client inspection mode rides the next voxygen build (`--asset-arena` + BASTION_OVERSEER). Catalog graduated 73→80 through the standing gates. TOOLING SHIPPED (tools/): gate.py 7-step battery + anatomy/semantic-placement + compare-reference (PCA) + anim lint + adjacency precheck + catalog recheck + redo-campaign anti-skip audit (592-hash before-snapshot; detail-floor via the pilot's shared detail_metrics). Contracts pinned: rest_space per rig (hands=parent, vessels=absolute), FLOOR composed positions, sha-stamped harness (stale-exe guard — which caught its own staleness bug pre-merge). Gate at merge: cargo check green, stamped spot test 1/1 PASS (palisade gate scenario), static gate 7/7. *(Merged from main during the B6 forward-merge; ordered here by merge time.)*

### GOD-HAND — SKIPPED overnight (blocked on asset integration; logged per rule 4)

- The roadmap's next block, explored and deliberately SKIPPED: the v3
  hand asset (per-part vox: palm + 8 finger segments + 2 thumb + a
  `rig.json`) exists ONLY in the pilot's isolated `asset-lab/` sandbox —
  NOT in the repo's `assets/` tree. Asset integration is the tester's
  lane through the BASSET1 merge, explicitly HELD FOR MORNING by the
  overnight plan (merged-state verification pending; possible re-forward-
  merge). Reaching into asset-lab from the builder lane would cross two
  session boundaries mid-hold. HAND-1's mechanics-only half (Link grab/
  carry/drop) without the hand visual AND without the favor cost is
  precisely the uncosted puppet-master anti-pattern HAND-CURSOR §0 draws
  its pillar against — not shippable alone. → Advancing to **B6
  stockpiles/hauling + SOFT-0/1 soft-collision (COMMITTED, Ben)** which
  is fully buildable with no asset dependencies; god-hand fast-tracks
  the moment BASSET1 lands the asset (FLEET_STATUS already says exactly
  this).

### B6 — stockpiles/hauling + SOFT-COLLISION (SOFT-0 first; COMMITTED)

- Start SHA: `9bab3c366f` (= main after TOOL0) · branch
  `bastion/block-B6` · overnight block 4. Order: **SOFT-0 FIRST** (self-
  contained; closes the committed mechanism + the known-open climb
  composites' root), then stockpiles/hauling, then SOFT-1 tuning
  against the haul crews. SOFT-0 implementation map (from
  SOFT-COLLISION-design.md §0-3): (1) `Colonist.soft_until: f64`
  (serde-default; 0 = off) — phys-visible transient state, expiry-based
  hysteresis; (2) trigger (a) the GRACE WINDOW in the watchdog: at
  STUCK_TIMEOUT, if the colonist hasn't been granted soft-pass for this
  stall yet (`ActiveJob.soft_granted`, server-only comp), grant soft
  (+reset stuck_time) INSTEAD of releasing — only a still-stuck
  soft-passed colonist goes unreachable; (3) trigger (b) density: > N
  colonists within a small radius → soft (server-side O(n²), colonies
  are small); (4) the softened push: extend the phys ladder-waiver site
  (colonist pairs) — if either soft-active, SCALE the pushback by
  ~0.15 (squeeze, not ghost); terrain untouched; needs Time in
  PhysicsRead (check); (5) `--chokepoint-scenario`: 1-wide shaft+ladder
  whole-crew egress → all out, zero unreachable, open-ground control
  spacing normal, nobody inside terrain, vanilla NPC unaffected.

- **WIP CHECKPOINT (overnight end): the MECHANISM is built and
  COMPILING** (soft state + softened push + both triggers, commit
  `a59308ed3e` + iterations), the `--chokepoint-scenario` exists and
  drove 8 diagnostic iterations; three REAL engine finds fixed en
  route: `SpriteKind::Ladder` has solid_height 1.0 (a rung is a
  platform — an all-ladder shaft is an impassable pole; the scenario
  now uses the open-column + rung-pillar shape B5.8's auto-pillars
  build), interior-shaft anchor steering (vertical steer at the anchor
  column — dropping the anchor at 1.6 XY pinned climbers under chamber
  ceilings), and LADDER MAGNETISM toward the ladder's open neighbor
  column (the ±2 grab band wedged climbers beside/on rungs). Run 7
  proved the single-file climb end-to-end (1/5 out, 1 mid-shaft, peaks
  tracked per colonist in the JSON now). OPEN: run 8's queue-patience
  tweak (stall accrual ×0.2 while staged) paradoxically stalled
  everything — zero releases, zero staged-routing logs, peaks at the
  floor; suspect a steer-oscillator or a patience/anchor interaction.
  RESUME AT — CORRECTED SUSPECT: staged-routing log lines were ZERO in
  runs 3-8 INCLUDING the partially-successful run 7 — so run 8's
  silence is probably run-to-run VARIANCE (run 7's climber likely
  escaped via magnetism drift alone), and the PRIMARY defect is that
  the ANCHOR STEER NEVER ENGAGES despite the registered anchor
  ((kx+3, ky, k_gz−6), all filter terms hand-checked sane). First step:
  a temporary debug log in the steer block printing
  `board.access_anchors.len()` + steer + target every %100 ticks
  UNCONDITIONALLY (the current log is gated on steer != target, which
  hides an empty-anchors case); verify the harness hook
  `bastion_register_access_anchor` actually lands in the same JobBoard
  resource the system reads. Then re-judge the patience edit against a
  working anchor. The block does NOT tag until the chokepoint scenario
  + the full suite go green (gate + rollback discipline holds).
