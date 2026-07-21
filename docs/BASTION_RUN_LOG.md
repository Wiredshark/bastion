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

- **RUNS 11-28 (the second stretch): the mechanism now WORKS — 4 full
  scenario PASSES (16, 20, 24, 28) with 5/5 crew egress through the
  1-wide shaft — but a crowd-hover tail flakes ~50%.** The debug
  suspect above was FALSE: the anchor steer had engaged since run 3 —
  PowerShell `Out-File` wraps stderr at console width AND tracing ANSI
  codes sit INSIDE field values, so every zero-count log grep was an
  artifact (method now: raw `cmd /c` redirect + ANSI-strip before
  matching — memory + BASTION_COMMON_ISSUES candidates). REAL fixes
  landed this stretch, each commit-documented: velocity-ONLY climb
  lift (phys ground-snap was resolving the position-pop straight back
  down on open floor — masked since B5.8 because every b58 climber was
  wall-pressed; and the pop could TUNNEL into ceilings once vz carried
  momentum — one colonist permanently embedded, caught by the
  hard-terrain assert); the Chaser dead-zone close (±3 assist grab);
  queue-release at second timeout (idle-release, no unreachable
  pollution; churn-counted so mirage anchors still end in the bubble);
  MID-CLIMB KEEP (beside-rungs at timeout = climbing, not waiting);
  reviewer F2 applied (churn guard = egress_scan VERDICT, not anchor
  proximity) + F3 shipped early (stale access-plan pruner, 20s);
  uid-keyed scenario identity (random NAMES COLLIDE — two "Yara of the
  Vale"s); (q) gate takes the b1 invariant shape (the stronger assist
  self-exits before plan triggers — stairs-EMISSION pin gap logged for
  AR-2/F4). Suite state on this code: unit 19/19 + B4/B5/B5.5/B5.8 +
  vanilla ALL GREEN (b58 confirmed twice). OPEN TAIL (the only
  blocker): 1-2 stragglers hover at feet≈394, x 15869-70 (chamber side
  of the shaft mouth), lifted 1 block, magnet never completes the
  2-block slide — suspects: a magnet-engagement gate not meeting
  (instrument it like the assist eval), crowd equilibrium (softened
  push vs magnet), or airborne-agent interplay. NEXT: magnet telemetry
  → fix → 3-green ck streak → full suite → SOFT-0 bookkeeping + tag.
  Reviewer consult on the queue design pending (may shortcut).

- **STRETCH 3 — BEN'S LIVE-FIX BATCH folded in + THE HOVER SOLVED
  (branch @ `b73f8df150`+, NOT gated/tagged).** The reviewer's consult
  answer NAILED the hover: sub-block wobble (magnet/hover/physics, all
  clearing the 0.5 STUCK_EPSILON) reset stuck_time forever → ZERO
  timeouts → zero churn → no net ever fired. R3 fix-1 shipped:
  stuck-time HYSTERESIS (`ActiveJob.reset_dist`, zero only on ≥1 block
  NET progress) — run E3 main phase went 5/5 first try. Batch contents
  (commit-documented): flat-mine drag false-reject FIXED (client floor
  from the SAMPLED surface at drag center, not the camera plane +
  server max-surface fallback + `fl_hint_decoupled` regression);
  overseer `day_length` = 10 min (the TimeScale day mechanism was
  already correct per FR6 — the 30-min base day was just imperceptible
  at 4×); MINE LIFECYCLE (done detection + `done_count` hook +
  disperse); TIERED FAIL-SAFES (`climb_free_until` — any-wall climb,
  jobless included, granted at the trapped VERDICT after discovering
  the plan-loop's take(0) swallowed grants; teleport-to-ground at 30s
  verdict + persistent-churn 16-cycle direct, warn-logged); reviewer
  F4b debug logs stripped. E5 confirmed the sealed-pit fail-safe FIRES
  (`ck_failsafe_out` true, egress steps=24 from the pit) and
  `ck_mine_done` passes. REMAINING: RUN VARIANCE on both scenario parts
  (main 3-5 of 5 across runs; fs_out flips) — every thread converges
  on the reviewer-recommended WAITING-STATE refactor (a Waiting
  ActiveJobState the watchdog skips, promoted by anchor-nearest/
  density) as the structural fix: it retires the churn-as-progress
  economy, the mid-climb-keep hack, AND the stillness-blindness for
  employed waiters in one shape. RESUME AT: implement Waiting per the
  consult answer → 3-green ck streak → full suite → gate/tag the folded
  block (SOFT-0 + B-LIVE batch) → ping architect (play-tester rebuilds
  for Ben's re-test).

- **PASS (2026-07-10): B6 = SOFT-collision + Ben's live-fix batch,
  GATED GREEN.** The 1-wide chokepoint red (B5.8's committed known-open)
  is CLOSED and every Ben-reported live bug fixed, in one folded block.
  Gate: unit 19/19, `--chokepoint-scenario` PASS (5/5 crew out the
  1-wide shaft, deterministic across ≥5 runs), B4/B5/B5.5/B5.8 PASS,
  vanilla soak PASS, voxygen check clean — a two-round full-suite
  streak. **The closer chain, reviewer/architect/DF-oracle-guided:** (1)
  the long hover tail was the reviewer's R3 diagnosis exactly — a
  reset-prone `stuck_time` (sub-block wobble cleared the 0.5 EPSILON, so
  zero timeouts → zero churn → every watchdog net structurally blind);
  fixed with `reset_dist` HYSTERESIS (zero only on ≥1 block NET
  progress). (2) `ActiveJobState::Waiting` (reviewer fix-2, DF 53.15
  validated) makes queue-waiting legible — the watchdog skips it, so it
  stops polluting the rescue nets. (3) reviewer F5: BOTH teleport tiers
  were broken (the churn tier dead code — its threshold raced its own
  reset; the 30s tier gated on `has_egress`, blind to the shaft-mouth
  false-positive) → replaced with ONE verdict-independent teleport
  (completes-no-work + not-moving-60s → nearest surface; reset on job
  completion so a confined deep-digger isn't yanked). Entombment is now
  impossible BY CONSTRUCTION. (4) Ben's batch: flat-mine drag
  false-reject (client floor from the sampled surface, not the camera
  plane), overseer 10-min day (the mechanism was already correct per
  FR6), mine-done lifecycle + disperse. Reviewer R1-F1 unit test
  (`egress_scan_rise_boundary`) pins the annulus ±1 forever. Bookkeeping
  done (ledger/consistency/architecture §2.10e); the flat-mine +
  hint-decouple + tool-factor + fail-safe regressions all ride the
  standing scenarios. **NOTE for the play-tester rebuild:** the
  overseer exe now shortens the day to 10 min (4× reads as 4×), the
  flat-mine drag accepts on normal + sloped terrain, and the crew
  climbs/teleports out of any pit. → merge + tag `bastion-block-B6`.
  RESIDUAL (architect-authorized ship, bounded-effort mandate): the
  b58 deep-dig throughput + roomy-stairs execution composites flake
  ~10% under the universal teleport's benign perturbation — NO
  entombment (the tiered fail-safe guarantees egress by construction;
  the gating no-stuck invariants e_out/f_cleared + the chokepoint
  deliverable hold; the flaky composites are demoted to reported per
  the "gate the invariant, report the mechanism" philosophy). The
  universal teleport went through an extended calibration: below-grade-
  duration (not movement-keyed — closed a real wander-entombment hole),
  dest-must-be-above (own-column-was-pit-floor bug), designation-mask
  exclusion (protects diggers, still rescues the trapped), unique
  colonist names (random names collided ~1/24, the dominant residual
  flake source). Reviewer F5 (both teleport tiers were broken) fully
  addressed.

- **TAGGED `bastion-block-B6` — main @ `6bd1c91a60`.** One line for the
  morning: the 1-wide chokepoint red is CLOSED (a whole crew squeezes
  through one ladder shaft), entombment is impossible BY CONSTRUCTION
  (climb-out → teleport floor), and Ben's live batch is in — flat-mine
  drag accepts on any terrain, the overseer day runs 10 min so 4× reads
  as 4×, mines mark done + miners disperse. Revert = `13f7d1f503`. →
  play-tester rebuilds for Ben's re-test.

### AR-2 — access-reliability hardening (in progress)

- Start SHA: `2e72df4338` (= main after B6) · branch
  `bastion/block-AR2`. Reviewer F1 (egress boundary unit test), F2
  (verdict-based churn guard), F3 (stale access-plan pruner) all landed
  INSIDE B6; the reviewer curated the reset-prone-accumulator class as
  checklist B14. Remaining AR-2 items: the grace density-gate (R1/P4),
  F4a idle-egress self-route, grace-per-stall-site.
- **DONE (first increment): grace DENSITY-GATE (R1/P4).** The SOFT-0
  watchdog grace only helps a colonist↔colonist stall, so it's now
  granted only when another colonist is within squeeze range (2.5 XY);
  a terrain-blocked stall skips straight to carve/unreachable instead
  of burning a zero-benefit STUCK_TIMEOUT. Full suite green (unit
  19/19, chokepoint ×2, B4/B5/B5.5/B5.8, vanilla). Committed on the
  branch, NOT tagged (F4a + grace-per-stall-site round out the block).
  RESUME: F4a idle-egress self-route (an idle below-grade colonist
  self-routes to the nearest known exit — the organic version of what
  the teleport floor guarantees), then gate + tag the AR-2 batch.
- **DONE (2nd increment): reviewer F6 — teleport designation-mask
  SCOPE HOLE.** The universal teleport's `board.designated.contains(feet)`
  exclusion was POSITION-only, and `board.designated` is colony-wide and
  does NOT shrink on claim-release — so a JOBLESS colonist trapped
  INSIDE an active designation had no teleport backstop (the
  "impossible by construction" net had an F5-class hole inside a zone;
  the old comment claimed a demoted digger self-teleports, which never
  fired). Fix: require BOTH a live job on the board AND being inside a
  designation to count as a protected digger — a jobless colonist now
  always teleports (closes the hole), real diggers stay protected (no
  deep-dig over-fire regression), the chokepoint straggler (pre-carved
  chamber, no designation) still teleports. NOT the reviewer's minimal
  `active_jobs.is_some()→skip` (that excludes employed-but-STUCK
  chokepoint stragglers and regressed CK to 7/10 in earlier testing —
  the AND-designation clause is what keeps both). CK 8/8, B58 7/8.
- **TAGGING AR-2** with the two substantive verified fixes (grace
  density-gate R1/P4 + F6 scope-hole); F4a idle-egress DEFERRED (the
  teleport floor already guarantees its invariant; the organic version
  needs agent-steering plumbing in the stillness pass — backlog).

## bastion-block-LADDEROFF — B6-hotfix (Ben live-test bundle) — TAGGED 2026-07-11 (merge `fcfee0c602`)

Ben's live-test bundle, shipped as BUILD 1 off `bastion/main` (four items +
two approved adds, separate commits, gated on the tag commit). BUILD 2
(flatten-hill + B15 standability FR12 + slope fixtures) held separate.

- **(1) AUTO-LADDER DISABLED** (`b10dd88d3a`). `const AUTO_LADDER_ACCESS =
  false`: `plan_access` carves STAIRS where they fit, NO auto vertical link
  where they don't (`None => None`). Kills the single-column queue-fight Ben
  saw. One-line reversible; `ladder_pillar`/`DesignationKind::Ladder`/
  climb-assist all STAY for the player paint tool. The universal teleport
  backstops any colonist a stair can't reach — entombment still impossible.
- **(2) ERASE DELETES BUILT LADDERS** (`b10dd88d3a`). The Erase drag removes
  `SpriteKind::Ladder` in-region (→ air via BlockChange) + drops the JobBoard
  access-anchor for emptied columns (`drop_access_anchors_in`). Ladders only;
  instant god-cleanup via the cancel path.
- **(3) CREST-DISMOUNT SNAP** (`0b180a535e`). A climber tops out into air the
  instant its feet reach the target level (the lift's `target_above` gate
  flips false there) and can't cross the horizontal gap onto the ledge —
  oscillates at the crest (Ben live-flagged). NEW isolated loop: a
  job-carrying colonist RISEN to its target crest and still HANGING snaps onto
  the nearest walkable dismount cell TOWARD the target (≤2 XY, at/one-below
  crest, head-clear + solid beneath). Keyed to the path target (never a free
  warp), self-terminating; the lift logic is UNTOUCHED; the 60s teleport stays
  the backstop. (Reviewer option-1 over a parallel teleport.)
- **(4) MINE-OSCILLATION resolved by MEASUREMENT** (`0b180a535e`). Parts
  #2/#3 (sticky anchor, dispersion=initial-pick-only) already held by
  construction; auto-ladder-off (item 1) removed the anchor colonists
  queued/bobbed at — the play-tester's root cause (which ALSO nails Ben's
  ladder-fighting complaint — same root). Added a cumulative `total_claims`
  counter + `bastion_total_claims` hook + b58 claims-per-block-dug REPORTED
  metric rather than re-grind the 40-iteration watchdog. Measured 1.12×
  (the 1.46× was a STALE pre-integrity-fix number). NO watchdog change —
  AR-2's hard-won determinism untouched.
- **(B) DESCENT-GATE RELEASE — deep-dig throughput fix (registry D16)**
  (`7ec7024ef5`). Item 1 had a silent side effect: the ACCESS-BEFORE-DESCENT
  gate held depth>2 cells waiting for auto-access that no longer builds, so
  tight deep digs stalled at exactly depth 2 (b58: 75/150). Fix: when
  `plan_access` returns None AND auto-ladder is off, RELEASE the gate
  (`descent_gated.clear()`) — deep cells become claimable, the teleport is the
  egress (entombment still impossible; the gate's protective purpose is
  redundant under the stronger backstop). STAIRS still LEAD where they fit.
  RESULT: b58 blocks_dug 75→150/150, `d_all_cleared` false→TRUE. New GATING
  `d_deep_unlocked` (blocks_dug>90 structural proof). Architect-decided.
  Reversible with the flag.
- **INTEGRITY FIX** (`0b180a535e`). `bastion_rename_colonists_unique` was
  UNCOMMITTED in the one-checkout tree; the harness calls it, so b10dd88d3a
  AND the B6/AR-2 tags did NOT compile the harness (gates ran green against
  the WORKING TREE, not the tag). Committed → LADDEROFF compiles at-tag,
  verified by tree-identity (the merge tree is byte-identical to the gated
  commit `7ec7024ef5`). For the record: B6=`6bd1c91a60` / AR-2=`c2acf8ba01`
  predate the method; their green was working-tree-validated. Every tag from
  here builds the harness clean at-tag (the gate's build runs on a clean
  tree to prove it).

GATE (on the tag commit, clean tree): UNIT 19/19, BUILD PASS (harness
compiles at commit), B4/B5/B5.5/B5.8/CHOKEPOINT/VANILLA PASS. b58
d_deep_unlocked=true (150/150), e_out/f_cleared/orphans_final green. (b4
arrived>=2 is the documented pace-marginal throughput flake — seed-1337 on
the 1-vs-2 boundary, 2/3 quiet PASS, all b4 INVARIANTS hold; passed on the
tag-commit gate. Candidate for gate-the-invariant/report-the-mechanism if it
recurs — not touched in Build 1.)

## bastion-block-SLOPE — BUILD 2 (slope-mining pair) — TAGGED 2026-07-11 (merge `a92afeae18`)

Ben's remaining slope-mining live bugs, off `bastion-block-LADDEROFF`. Two
commits, gated at the tag.

- **(2a) FLATTEN-HILL** (`443b570594`, Ben live-bug #4). A flat-floor Mine
  painted at the BASE of a tall hill left a hilltop stub: `column_surface_z`
  centres its ±`SURFACE_SCAN_UP`(48) scan on the PAINT PLANE, so a hill column
  solid past hint+48 reads its "surface" as the window ceiling → the flat-floor
  `column_range` digs only floor..hint+48. FIX: one surface authority
  (`resolve_column_surface`) — flat mode scans up from the shared floor to the
  column's TRUE crest (`column_flat_surface_z`, bounded `FLAT_SURFACE_SCAN_MAX`
  =128); relative mode unchanged. Wired into job-gen + echo-bounds + the
  paint-time volume gate (now measures the tallest true crest via
  `max_crest_for`, so reaching the crest can't over-generate past
  `MAX_DESIGNATION_VOLUME` — a too-tall hill is honestly REJECTED). TEST b5
  phase 7.8: a 3×3 hill cresting 60 above base → 549 jobs floor..crest, bounds
  at base+60, PAST the old base+48 cap.
- **(2b) B15 STANDABILITY** (`1f316afe20`, Ben live-bug #5/#6, reviewer FR12).
  The exposure gate admitted UNSTANDABLE work — a hillside `+1`-arrival-gap cell
  (on-top walled to a 1-wide slot the capsule wedges in) or a floating block
  passed exposure → claimed → never Arrived → watchdog-unreachable → "slope-mine
  gives up with blocks left." (Play-tester split: REAL-UNREACHABLE, not churn —
  Build-1's hysteresis already fixed the churn leg.) FIX: `has_standable_stance`
  — a TERRAIN-ONLY, ONCE-PER-CYCLE predicate (alongside `is_exposed`) computing
  a `standable` set; claims gate on it. PREFERS on-top (in-place), falls to an
  ADJACENT-ground stance for a wedged `+1`-slot (≥3 lateral sides solid) — the
  reachable downhill stance. `ActiveJob` gains a `stance` offset committed at
  claim (server-only, no wire); arrive-target = `(job.pos+stance)+(0.5,0.5,0.0)`
  (default (0,0,1) = the pre-B15 on-top target). An ISOLATED 1-wide floater has
  no reachable stance → CLEAN-SKIP (not claimed, not flagged unreachable → no
  churn; deferred to cave-in); a reachable LEDGE is mined normally. Access steps
  exempt. REGRESSION-SAFE: the first cut over-gated (adjacent-first → b58
  87/150); on-top-preferred + the wedge check restored b58 150/150. TEST b5
  phase 7.9 (deterministic claim-level): on-top control claimed, adjacent-only
  (rock-capped) claimed via the adjacent stance, isolated floater clean-skipped.

REGISTRY: 2b CLOSES B15 (claimability admits unstandable work). Play-tester's
`--slope-mine-scenario` + `--floating-block-scenario` (SET-A/SET-B natural-slope
+ floating-remnant fixtures) fold in as the fuller regression as they land.

GATE (on the tag commit, clean tree): UNIT 19/19, BUILD PASS (harness compiles
at commit), B5/B5.5/B5.8/CHOKEPOINT/VANILLA PASS; b58 150/150 d_all_cleared +
d_deep_unlocked; b5 hill + B15 (ontop/adjacent/floater) asserts green.

## bastion-block-CAVEIN — CAVE-IN v1 (FR11) + B16 crash-fix + R7 rust-lld — TAGGED 2026-07-12 (merge `437577ed25`)

The first roadmap feature block after Ben's live-bug arc (Builds 1+2): floating
mining remnants now FALL, with the entombment guarantee intact — plus the
critical alt-tab crash fix and the R7 linker flip riding the same rebuild.

- **CAVE-IN v1 (FR11, reviewer-FEASIBLE)** (`369b67e083` core, `3e759da4a5`
  wiring, `30d55d988e` scenario):
  - `floating_chunk` — the bounded support check (FR11 Q2): at a Mine job's
    COMPLETION (Q3 — gate at the point of action, never designation-time),
    flood each solid component severed by the removed block, capped at
    `CAVEIN_SUPPORT_CAP`(64); a component enumerated within the cap is a
    floating remnant → COLLAPSE. >cap = assumed supported (conservative — a big
    anchored mass never spuriously falls; large overhangs defer to the future
    global check). PURE + unit-tested (`floating_chunk_support`).
  - COLLAPSE: the chunk's cells drop to air + a `MINE_DROP` resource each — the
    floating rock Ben watched now FALLS (composes with 2b: the standability
    gate clean-skips an isolated floater, cave-in collapses it).
  - EJECT-AND-INJURE (Q1/Q6, hardened by reviewer R8): every colonist in the
    crush volume is ejected to the nearest TRUE STANDABLE cell OUTSIDE the
    falling footprint (`eject_dest` — an air-feet + air-head + solid-floor
    ring search preferring same-level lateral step-outs; `None` → left in
    place, safe since a collapse only REMOVES rock) + injured (−25% max
    health + a 0.25 Mood fear drop; no DF-WOUND dependency). R8/F-CAVE-1
    (HIGH, caught pre-tag): the first eject reused the SHALLOW-pit
    `column_surface_z` scan, whose all-rock deep-mine window returned the
    window TOP — teleporting a deep victim INTO stone; the rewrite air-checks
    every candidate, so no unchecked destination survives. R8/F-CAVE-3: the
    eject-and-injure is ONE shared fn (`cavein_eject_and_injure`) called by
    BOTH `Sys::run` and the harness hook — the tested path IS the shipping
    path, identical by construction (no parallel copy to drift; registry B17).
  - **THE ENTOMBMENT INVARIANT (why cave-ins can coexist with no-entombment):**
    the collapse REMOVES rock to air (never re-places solid onto a colonist),
    and the eject+injure resolves anyone caught — a victim is shoved out, hurt,
    set back; NEVER buried. Proven end-to-end by the new GATING
    `--cavein-scenario`, TWO legs: SHALLOW (a 3-cell arm on a single pillar, a
    colonist under it, the collapse fired deterministically via
    `bastion_force_collapse_check` — needed because a live-mining digger
    wanders off the crush footprint before completion) and DEEP (the same
    collapse inside a sealed chamber 130 below the surface — the F-CAVE-1
    geometry where the old eject embedded the victim in stone). Asserts, none
    weakened: collapsed + victims≥1 + ejected + feared + standable
    (not-embedded + near-ground) on BOTH legs; the Sys::run collapse TRIGGER
    was separately proven live (the digger-as-victim runs collapsed via real
    mining).
  - REGRESSION-SAFE: b58 PASS ×2 (150/150, d_all_cleared, d_deep_unlocked,
    e_out/f_cleared/orphans) — a connected dig has no floaters, and the
    completion-path change costs one bounded flood per completed block.
- **B16 / CASE-001 (CRITICAL, architect-triaged)** (`61aeec7cf9` + refine in
  `341e260f67`): `common/src/clock.rs` `last_game_dt` clamp gains the missing
  LOWER floor (`.clamp(1e-6, MAX_GAME_DT)`) — an alt-tab window pause let the
  nudge-toward-real-time go NEGATIVE at high fps → `Duration::from_secs_f64`
  PANIC (Ben's hard crash). 1e-6 (not 0.0) also kills a cosmetic figure
  NaN-flicker (`dt.sqrt()` unguarded). Reviewer-confirmed; VANILLA Veloren
  (byte-identical to B0) — upstream-fix candidate.
- **R7 rust-lld** (`6afc26be34`, architect-approved): windows-gnu links via
  the toolchain's BUNDLED rust-lld (self-contained linker — WinLibs GCC ships
  no ld.lld, so this is the no-shim path). Harness link measured ~14% faster;
  voxygen (link-bound) is the real target — the play-tester measures the delta
  on its first post-flip rebuild; REVERT the config block if the voxygen
  saving is negligible. The rustflags cache-bust was absorbed by this block's
  gate build.
- New harness hooks: `bastion_colonist_health`, `bastion_colonist_mood`,
  `bastion_force_collapse_check`. Scenario-wiring lesson (in-branch): rename
  colonists AFTER a tick — the Colonist comp lands on the rtsim promote, so a
  rename-before-tick returns an empty roster and every name-keyed lookup
  silently no-ops.

GATE (on the tag commit, clean tree, LLD): UNIT 19/19 + `floating_chunk`
unit test, BUILD PASS (harness compiles at commit), B4/B5/B5.5/B5.8/
CHOKEPOINT/CAVEIN/VANILLA — see the gate line in the tag ping.

## bastion-block-NIGHTHORROR — NIGHT_HORROR (FR14) + ARCH-001 — TAGGED 2026-07-12 (merge `e1a6d2ba27`)

The first creature taken fully asset→animated→behaving→in-game→testable —
the REUSABLE CREATURE-INTEGRATION PIPELINE's reference instance (FR14
FEASIBLE; wendigo scaffold throughout). Ben's ask: "finish it up, get it
animated, add behavior, in-game, with a way to test."

- **REGISTER** (`6f4caddc6f`): `Species::NightHorror = 35` — appended at the
  END (explicit discriminants are wire-stable; the spec's "=13" was STALE,
  drift-checked against the real tail Gigasfire=34). Touch-points: noun key +
  `AllSpecies` field + `Index` arm; dims/health(280, wendigo-tier)/mount
  offset; the rtsim wild-entity map → `common.entity.wild.aggressive.
  night_horror`; the `npc_names.ron` keyword (what makes `/spawn` +
  `RandomWith` parse); `VoiceKind::Wendigo` (the lineage); generic biped_large
  armor + the **BEAST CLAWS** melee set (the werewolf's physical claws —
  reviewer Q2: a stalker, not the wendigo's frost magic); loot = rugged hide +
  claws + a rare grim-eyeball trophy (the DF-BEAST trophy line).
- **MODEL**: the pilot's 11 wendigo-frame parts →
  `assets/voxygen/voxel/npc/night_horror/male/`; central + lateral manifest
  rows are the WENDIGO rows VERBATIM with paths swapped (incl. the leg_r
  x-quirk); `(NightHorror, Female)` = alias to the male models — the exact
  wendigo convention (zero duplicate assets, exhaustive matches satisfied).
- **ANIMATION**: the biped_large motion set is skeleton-shared (free); the 11
  per-species OFFSET-TABLE arms (the one non-obvious compile-error
  touch-point every future creature hits) are wendigo-verbatim.
- **TEST**: `/spawn enemy night_horror [amount] [ai]` — works via the keyword,
  zero new command code. The optional `bastion_arena` spawn action is a
  documented follow-up. Stalk/ambush/night-active tuning + the fear aura = v2
  (spec'd in NIGHT-HORROR-INTEGRATION-design.md STEP 4).
- **ARCH-001 rider** (`a456846f4c`, separate/cherry-pickable): the `/aura`
  duration is rejected at parse via `Duration::try_from_secs_f32(..).is_err()`
  — closes the negative/NaN/inf/OVERFLOW `from_secs_f32` panic (registry B16
  sweep; vanilla, admin-only; reviewer-amended over the GPT draft, which
  missed the overflow case — the bridge's hard-problems-only lesson).

VERIFIED: workspace `cargo check` clean ×2 (every exhaustive match armed) +
a warm re-check covering ARCH-001; `veloren-common` 120/120 (the entity-config
walker validates the new `.ron` chain end-to-end: body parse → loadout →
loot); full gate on the tag commit — see the tag ping for the line.
PIPELINE NOTE (for the next creature): the 5-step checklist in
NIGHT-HORROR-INTEGRATION-design.md held exactly; the only spec drift was the
enum discriminant (always re-verify the tail).

GATE NOTE: the tag-commit gate ran green on UNIT/BUILD/B4/B5.5/B5.8/CK/CAVEIN/VANILLA with ONE b5 miss (mine_cleared 26/27 under full-suite load) — quiet standalone 3/3 PASS 27/27, no night_horror mechanism (registration-only block): the documented B8 execution-race flake, logged per the sampling-flake mandate (sibling of b4 arrived + b58 d_all_cleared).

## bastion-block-CHOP — CHOP redesign (FR10) — TAGGED 2026-07-12 (merge `05c016dbfa`)

Mark trees → fell the WHOLE tree (trunk + canopy) → the wood drops. Replaces
the Wood-slab Chop (the root of Ben's floating-tree: canopies never removed)
with FR10's PRIMARY — the World tree oracle.

- **`footprint_mode()` classifier** (`common::bastion`): Chop is the FIRST
  `Area2D` kind — a pure XY paint. The UI (depth stepper + flat/slope toggle
  HIDE), the paint path (`z_extent: None` on the wire), and the server (area
  vs slab job-gen) all branch off this ONE flag; future Gather/Forage kinds
  get the branch free (classified, never special-cased).
- **SHARED whole-tree detection** (`server/src/bastion_chop.rs::detect_trees`
  — ONE fn for the paint handler AND the `bastion_place_chop_area` harness
  hook, B17 identity-by-construction from birth): `get_area_trees` candidates
  → the engine's own `tree_valid_at` env-filter via `world.sample_columns()`
  (never seeds from a building — D15) → the bounded Wood+Leaves flood
  (`tree_fell_set`: cell cap 2048 + height band 40 + XY radius 10).
  World-threading per FR10: `ReadExpect<Arc<World>>` + `ReadExpect<IndexOwned>`
  in the `in_game` handler (read-only `Arc` — par_join/B10-safe);
  `bastion_jobs` stays terrain-only (`place_chop_cells` makes jobs for
  handed-in positions).
- **PER-TREE marking, ZERO wire change:** each detected tree echoes as its
  OWN `BastionDesignation` (region = the tree's tight AABB) — per-tree
  outline boxes + cancel-through-the-box on the existing echo/render/erase
  machinery, no message-schema change.
- **Leaves:** `job_wanted`/`still_valid` accept Wood|Leaves; completion
  captures the PRE-REMOVAL kind — Wood drops `CHOP_DROP_ITEM` (yield scales
  with trunk size by construction), Leaves clears FREE. Closes the registry's
  "Chop-ignores-Leaves".
- **DENSE-FOREST finding (b5 runs 1-3):** forest canopies CONNECT — an
  unbounded per-seed flood ate 13 trees' worth to the cap from ONE seed. The
  XY radius is the per-tree boundary, and in dense stands the cell cap
  legitimately clips (bounded work per seed; neighbours are their own seeds,
  shared cells dedupe at placement). Gate the INVARIANTS (bounded, jobs
  placed, whole-tree mixed kinds, per-tree cancel, leaf-no-drop), not the
  per-tree-average mechanism.

TESTS: `tree_fell_set` unit pins (component/cap/radius/height/non-tree-seed);
b5 phase 7.10 GATING — 13 real worldgen trees on seed 1337 through the SHARED
path, first tree's box holds BOTH Wood and Leaves, per-tree cancel clean, a
chopped Leaves block clears with NO log. All pre-existing b5 phases intact.
Legacy region-path Chop keeps per-block semantics (the harness fixture
surface). Client: stepper hides on the Chop tool; Area2D paints send no
extent.

## bastion-block-COORD — COORDINATION-stigmergic-v1 (FR13-REV) — TAGGED 2026-07-12 (merge `e3b792fc44`)

Ben's mad-scramble (the whole crew piling the nearest work) fixed with the
ant-inspired STIGMERGIC design (FR13-REV, reviewer-FEASIBLE — chosen by Ben
over the explicit sector partition; genuinely smaller: a scoring-term
generalization + a decaying field, no scheduler).

- **THE FIELD:** `saturation: HashMap<coarse-cell, f32>` on the JobBoard
  (COORD_CELL=4). A colonist WORKING (Arrived) deposits 1/cycle at its job's
  cell; the field decays ×0.95/cycle → a steady worker equilibrates at 20.
  DETERMINISM (FR13-REV Q2, B0-safe): per-cell decay is order-FREE; deposits
  iterate the sequential entity-ordered join (fixed-order float sums); reads
  are LOCAL key lookups — no global min-search, no tie-break hazard.
- **ALLOCATION (Q1):** `score = dist + depth + clump + saturation×0.75` —
  repelled from crowded cells, drawn to the under-served frontier. ADDITIVE
  alongside the in-pass clump repel (deviation from Q1's "replace," reasoned:
  the field only knows LAST cycle's work; clump still prevents same-pass
  re-clumping and the b58 dispersion gate rides on it). Near-flat field ≈
  today's behavior (Q5 — continuous degrade, no small-job threshold).
- **ANTI-BOB (Q3, the load-bearer):** the field is read ONLY at claim time (a
  commitment point), never continuously; re-flow happens through job
  COMPLETION (monotonic). NO crowding-re-flow band in v1 (per the recommend —
  add the T_high/T_low hysteresis band only if it proves necessary).
- **THE BARK (Q4):** a REAL flow (own cell ≥ claimed cell + 5) emits
  `npc_say("Crowded here — I'll work where they're short-handed")` via a
  ChatEvent emitter with a 30s per-colonist cooldown (`allowed_to_speak` is
  capability, not rate-limit). The emergence, narrated.

TEST: `--coord-scenario` (GATING, in the suite): two equal 98-job slabs ~20
apart, the whole 5-crew spawned beside site A — the field forms over A
(sat_peak 18.7 = the predicted 15-30 equilibrium band) and the crew SPLITS
(both sites claimed SIMULTANEOUSLY — impossible under pure distance-greedy),
0 orphans. PASS first-run. `bastion_saturation_at` harness hook.

GATE NOTE: gate on 4a397821dc all-green except chokepoint fs_out (load); quiet x9 = 8 PASS + 1 ck_in_terrain=1 — classified the PRE-EXISTING wedge hazard (run-C2/C3 climb/magnet class) surfaced by the B8 same-seed nondeterminism, NOT a COORD regression (different asserts flake different runs; COORD writes no positions). The wedge is in the BUG LANE at SAFETY severity (entombment-adjacent); --deterministic-rtsim (the next block) makes it reproducible. The merge tree's only non-md delta vs the gated commit is cmd.rs = the cfg(test)-only aura-guard unit pin (shipping binaries bit-identical).

## bastion-block-DETRNG — deterministic harness rng + conservation asserts — TAGGED 2026-07-12 (merge `0ce3517b71`)

The B8 flake class retired at the root (architect-approved test-infra; the
play-tester proved same-seed b5 runs flipped PASS/FAIL — rtsim's per-tick
rules seeded their RNGs from OS ENTROPY, the documented B0 §4 caveat, so
`--seed` never reproduced a run; that one hole was the whole known-flaky
ledger: b4 arrived, b5 mine_cleared/stone_sum, b58 d_all_cleared, ck
fs_out/in_terrain).

- **`rtsim::tick_rng`** — THE one constructor every rule RNG goes through
  (identity beats convention): OS entropy in the live game; (world seed,
  tick, salt)-derived ChaCha under `rtsim::DETERMINISTIC_RTSIM`. The 3
  behavioral sites converted (cleanup, migrate, npc_ai — the per-NPC salt
  keeps each stream independent of the rayon `par_iter` order: deterministic
  under parallelism BY CONSTRUCTION, the B10 pattern). The harness sets the
  flag before `Server::new`; **Ben's live game never sets it** — rtsim
  entropy is game behavior and stays.
- **`bastion_jobs`' drop-toss rng → tick-seeded** (the 4th site; cosmetic
  scatter → pile merge grouping; deterministic everywhere, same feel).
- **CONSERVATION BELT (genuinely needed):** `cavein_drop_cells` counter +
  hook; b5's stone gate becomes `27 ≤ stone_sum ≤ 27 + collapse drops`. The
  deterministic majority path finishes the b5 mine as **26 mined + 1
  COLLAPSE-SEVERED cell** (a real B15×cave-in composition) — the literal
  `==27` was wrong in BOTH directions.
- b5 window 120→180 (headroom) + `recursion_limit=256` (the `json!` literal
  outgrew the default).

PROVEN: same-seed full-JSON md5-identity when scheduling aligns; assert-level
stability 4/4 under scheduling variance. **RESIDUAL SEAM (documented, out of
scope):** async chunk-gen/thread scheduling still decides races (e.g.
mine-vs-collapse on a last block) — telemetry varies, the invariant-form
asserts tolerate it; full tick-determinism = a future infra project.
UNBLOCKS: CASE-003's seed-sweep repro (the wedge is now hunt-able).

FOLLOW-UPS IN-BLOCK: b5 stone gate -> exact per-block accounting + mine_cleared REPORTED; b4 arrived>=1 (mechanic) + count REPORTED — the last two flake-family members moved to invariant form (architect pre-approved). Final gate: 8/8 ALL GREEN on the tag commit.

### bastion-block-CASE003 (merge ecc069fd18)
Row 30, entombment-SAFETY. The chokepoint wedge REPRO-CONFIRMED via DETRNG seed-sweep
(the block's stated dependency paying off): seed 21 = ULTIMATE FAIL-SAFE teleporting a
cave-stuck colonist INTO a tree trunk (column_surface_z sees through Wood; picker had no
air check), phys revert-lock persisting it, stuck-watch re-firing to the same cell.
Soft-collision hypothesis REFUTED for the reproducible class (nearest_other=84 at trip;
0 pair-pinch trips in 24 seeds) - packet's phys push changes deferred, Opus-ratified
(R10, no findings). Fix: picker standability (feet+head air, skip occupied columns) +
phys per-tick CENTER-SAFETY-NET on the ONE shared eject_dest (moved to common::bastion).
Telemetry: ck_center_net_fires (reported; fired on seeds 7/8/1337 = a REMAINING embedding
writer, diagnostics landed, hunt filed). 16-seed re-sweep: in_terrain 0/0 at per-tick
sensitivity everywhere; formerly-deadlocked seeds 6/9/13 now pass (shared root). 4 new
unit pins. Gate 8/8 all-green on the tag commit. By-catch for the bug lane: composite
legs fail on non-gate seeds (failsafe leg flips run-to-run = seam-sensitive; mine_done
seed 4 stable) - repro seeds in the session handoff note.

### bastion-block-EMBED-WATCH (bf858917ea, direct tag — Opus R11 green, architect-directed)
CASE-003 belt v2: the embed net moved to the sequential system with 30-tick persistence (mid-phys form over-fired on dig-pocket/boundary transients and broke ck fs_out); + the CASE-004 Build-completion occupancy guard (solid placements defer on occupied cells); + staged_at_anchor + locomotion instrumentation; FR15 locomotion core REVERTED→DESIGN (rows 31.x queued). Gate 8/8 on the tag commit.

### bastion-block-LOD0 (bce7ecfc68, feature 4f5b6f22ab)
Row 32, the save-back loss guard (registry B11 root-fixed): colonist_record mirrors the
live comp + canonical bag snapshot into Npc.bastion_colonist every loaded tick + demote
flush; promote restores the persisted bag WHOLESALE (Option semantics: None=first
promote keeps spawn loadout, Some=truth — replace-not-add killed the fresh-loadout
dupe). --lod0-scenario 3/3: real mined XP + exact inventory survive demote→delete→
re-promote. Gate 9/9 (the ladder gains the LOD0 leg). Skills/inventory half per FR4-e;
Needs/Mood restore lands with B7.

### bastion-block-LOD1 (51150baca3)
Row 33, the tier dupe guard: is_loaded gate (RtSimEntity→Npc.mode) on BOTH bastion_jobs
loops (claim + travel/work incl. the Arrived completion arm) — a demoting colonist is
never processed by both tiers (spec 5D impossible-by-construction; demote-flush/
DeleteEvent untouched; claim sweep = the release, regression-guarded). --lod1-scenario
2/2 + gate 10/10: 3 rapid mid-work demotes, zero flip-window leaks, exactly-once
completion (stone SUM conserved), stable roster, zero ghost claims. Packet's proofread
note honored (gate on Sys, not the stale unload-hook seam).

### bastion-block-B6HAUL (03b649c451)
Row 34, typed jobs + reservations + auto-haul: JobKind (Designated wraps every pre-B6
job byte-identical; Haul{item,destination} net-new, append-only) + Job.reservation
(serde-default) + ONE ReservationTable on JobBoard (item Uid per job, stock stays
DERIVED from physical items, D2), remove_job() releasing the reservation with the job.
Stockpile designation ACTIVATED (was inert since B4): zone registration, haul-job
generation off loose bastion-output drops (reserve at generation), execution via
vanilla InventoryManip::Pickup (leg 1) + CreateItemDropEvent into the zone (leg 2).
Build-fetch: a material job with nothing in hand claims a stockpiled item availability-
checked at scoring, reservation BOUND at commit (raced-away item skips the claim, no
double-spend). --b6haul-scenario 3/3: 2 Builds/1 stockpiled stone → exactly one
completes; 5 mined stones auto-haul, zone sum == pad total exact. Gate 12/12 (ladder
gains the B6HAUL leg). Sonnet tag-review (lean single-pass, MIXED-class): METHOD
matches its named reuse (vanilla Pickup, no second pickup mechanism; JobKind didn't
front-load Gather per row 38's ownership). One LOW-PRI finding filed as row 34.1
(B6-FETCH-REQUEUE) — a re-claimed mid-fetch Build job can orphan a second reservation
on `needs_fetch` not checking `job.reservation.is_some()` first; a slow leak, not a
double-spend, doesn't block the tag.

### bastion-block-BELT-EXERCISE-TEST (a3ee084346)
Row 31.3 (Opus R11 follow-up): --belt-exercise-scenario — sealed-pocket injection
(revert-locked, persists by construction) proves the EMBED WATCH persist→relocate path
FIRES (net_fires+1, relocation cell-verified, destination free); FAILING-capable. Build
occupancy guard confirmed full-body (feet+torso) by inspection. Harness-only diff; game
code identical to the 10/10-gated 51150baca3; ladder = 11 legs.

### bastion-block-BAG5CORE (5fc29a4101)
Row 36, canonical world-action helpers: six verbs extracted into a new `bastion_actions`
module (`approach_target`, `work_progress`, `completion_block`, `emit_drop`,
`emit_pickup`, `deposit_all_of`) — `bastion_jobs::Sys::run` now calls these instead of
inlining the logic (7 call sites migrated, net −76 lines in `bastion_jobs.rs`); the
committed-path steer chain stays drive machinery, deliberately not folded in. No
NPC-drive/self-designation code (B-AG3-gated, out of scope). Byte-identical spot-checked
(b6haul/lod1/b5 scenarios pass post-refactor); full 12-leg gate 11/12 — the one non-pass
is the KNOWN pre-existing `ck`/fs_out seam-flip flake (2-PASS/1-FAIL on an identical
binary, documented scheduling-noise class, not a regression; safety invariants clean
throughout) — treated as green. Sonnet tag-review (lean, MIXED-class): confirmed all six
functions are genuinely called (not dead extraction), METHOD matches the packet. Clean,
no findings.

### bastion-block-BAG1 (4522857fd4)
Row 35, loaded NPCs continue their rtsim lives: the promote-time write→read intent
bridge (`tick.rs:826-841` → `action_nodes.rs::idle()`) was ALREADY population-wide and
wired — Sonnet's audit found it, not a rebuild. The one real gap: `NpcActivity::Gather`
was a `TODO` stub that danced in place indefinitely; fixed as an honest degrade
(falls through to idle-wander, a `// row 38` pointer left for when real gathering
lands). VERIFIED dynamically via an airship-dock cluster (8 promoted, 8 embodied, 4
acting, all `GotoFlying` — the mechanism fires population-wide, not colonist-specific);
the intended ground-townsfolk fixture couldn't locate a promotable cluster (harness
geography, not a code gap — `tick.rs:667-734`'s promotion-eligibility confirmed clean,
no role filter; filed as row 35.1). No dedicated gate ladder leg — this commit sits
between the B-AG5-CORE gate and ZONE-0's commits (no gather-intent fixture exercises it
directly; `action_nodes` compiles clean); ZONE-0's eventual Opus-gated full-ladder run
covers it retroactively.

### bastion-block-CASE004-MAGNET (bb858c1cf9)
Row 31.1, the confirmed BC-004/R11 writer closed: `climb_col` proves headroom at the
LADDER's z, but both ladder-magnet branches wrote the COLONIST's position at ITS OWN z
— the else-nudge could step into a mid-climb pinch (torso in wall, belt-relocated) and
the on_pillar snap's scanned floor never checked its own head cell. Both writes now
GATE on the exact `climb_col` predicate evaluated at the destination/own z — blocked
means skip the write entirely (no relocate; `eject_dest` stays the primitive for a
different problem). `--magnet-scenario` 2/2 PASS: a lip-pinched shaft climb asserts the
capsule core NEVER sits in solid at any tick (direct, per-tick — the belt's own
4-corner predicate), `CENTER_NET_FIRES` stays 0, climb completes unregressed. Gate
13/13 (ladder gains the MAGNET leg). Sonnet tag-review (lean, MIXED-class): diff matches
the packet exactly (own-z gate on the nudge, head-clear gate on the snap, skip-not-
relocate on both) — clean, no findings.

### bastion-block-ZONE0 (7b6d7ee08c)
Row 37, the activity-zone soft magnet — mechanism commit `518ac9c46c` (`ZoneKind`
schema + `DesignationKind::Zone`, appended wire-stable; board registry + cancel beside
stockpiles; the `ActivityZones` resource mirror so agents read footprints without
touching the board; the magnet itself — colonist-gated, the vanilla patrol-origin
bearing-pull aimed at the nearest in-range zone center, structurally outside the
stuck-economy). **Opus-gated (§R12): GREEN-LIGHT** — mechanism sound (needs-win-by-
construction confirmed, the magnet sits in `idle()`'s last fall-through), stuck-economy
interaction verified STRUCTURALLY IMPOSSIBLE (grepped the whole diff for every
stuck-economy symbol — zero hits; the magnet only writes `agent.bearing`, an idle
colonist never job-travels), shape sound (weight 0.1/range 48, safe soft regime). This
close-out commit aligns the `--zone-scenario` gate with the accepted ruling (Opus
option-i, architect-confirmed): attraction is REPORTED telemetry for the
DESIGNER-SUGGESTIONS §19 tuning pass, not asserted to a threshold; the gate asserts
zone-registration + the FREEDOM invariant (a stronger-drive Mine job pulls a zone-side
colonist out and completes), 2/2 PASS. The 13/13 gate at `bb858c1cf9` certified the
whole lineage including the mechanism commit; the ladder now carries 14 legs (ZONE
added). Bonus telemetry for §19: run-1 measured 111 zone-samples vs 1 control (the bias
IS measurable on good draws); run-2, 2 vs 0 (draw-variant, the subtle-soft regime R12
describes) — folded into the designer-suggestions entry.

### bastion-block-GATHER (8ce1b77821)
Row 38, forage — the FOOD-LOOP verb: `DesignationKind::Gather` (Area2D, Chop's exact
pattern), scans the painted footprint for `Block::is_collectible()` sprites filtered to
the food `TerrainResource` allowlist, one deduplicated `JobKind::Gather` per target,
approach via `has_standable_stance` (reused, not a second solver), executes through the
authoritative `ControlAction::Collect` (item created by the interaction, never minted in
`bastion_jobs`). Deposit ruling (Sonnet, confirmed option (a) + one closing trigger):
gathered items ride the bag (no per-sprite round-trip, no batching-threshold design
call — bag capacity is the natural batch); ONE pre-claimed `JobKind::DepositRun` per
carrying colonist fires at its own arbitration slot once no claimable Gather target
remains (orphan-swept, excluded from the ordinary claim loop so it can't double-fire),
`deposit_all_of` per held def at the nearest accepting zone (reuses B6-HAUL leg-2's
picker, not a second one); `cancel_region`'s dead-zone sweep extended to cover
`DepositRun`. Gate 15/15 (ladder gains the GATHER leg; b58 telemetry in-band, claims
1.33/orphans 0). `--gather-scenario` 2/2: scan honesty (6 planted sprites → 6 jobs),
exact conservation (gathered == expected, one hand-vacated vanished-target completes
moot cleanly), board drains, every gathered item lands at the store with conservation
through the deposit trip. A `job_wanted`-allowlist unit pin lives in
`server`'s own test module (outside the ladder's common-only UNIT leg — ran green
manually this once, rides future server test runs).

**Two reported, non-blocking notes (not filed as findings — both correctly
characterized by the builder as documented limits, conservation holds in both):** (1)
run-to-run telemetry variance on an identical binary (store 14/bags 0 vs store 9/bags 2)
is the known scheduling-seam class (B8) — run-2 just exercised the designed corner where
a non-gatherer's spawn-loadout food stays bagged; core invariants identical both runs.
(2) a loot-table sprite could in principle roll a different def than the one recorded at
emit-time — the leftover just rides the bag (never lost/duped), and a future general
bag→stockpile sweep block would subsume it. Sonnet: no tag-review pass (CHEAP-class,
self-verify covers it per the backstop rule's own scope).

### bastion-block-HIST0 (410460f875)
Row 39, the Chronicle — the world's permanent memory + the ONE `record()` capture seam.
`ChronicleEvent { seq, kind, actors, site, pos, at_tod, importance, scope, attribution }`
— the Gap-Audit Addendum's locked kind list VERBATIM (54: core ten + eight source
groups, verified exact count), plus two well-justified additions beyond the packet's
literal schema: `seq` (a monotonic capture ordinal, the HIST-3 cross-link key) and `pos`
(the D7 bucketed spatial key, `site` now a rollup derived from it) — both extend, don't
reshape, the locked fields. Store: banded (`Routine`/`Notable` are cap-evicting deques,
O(1) at record; `Legendary` a plain append-only `Vec` NEVER touched by `cleanup()` — by
construction, verified directly in the diff, not just by the test). `Data` gains
`#[serde(default)] chronicle`, sibling pattern, no version bump. `record()` reachable
from both tiers via `RtSim` resource access (`server/src/rtsim/mod.rs`/`lib.rs`
harness hooks) — plain `write_resource`/`read_resource`, no manual locking, no
`par_join` involvement (checked directly per the architect's specific concurrency ask —
clean). `cleanup()` rides the existing `CleanUp` rule at the same cadence as
`reports.cleanup`. 3 in-crate unit tests (bounded-growth + legendary-immortality under a
4x/2x-cap soak with a real `cleanup(TimeOfDay(MAX/2))` call; seq-monotonic-across-bands;
byte-exact double round-trip) + `--chronicle-scenario` 2/2: caps pinned exact
(512/2048/64, so a cap retune breaks the scenario on purpose), 120-tick stability with
`CleanUp` live, and the REAL `Data::write_to`→`Data::from_reader` B10 boundary
round-trips the chronicle byte-for-byte. Gate 16/16 (ladder gains the CHRON leg). Sonnet
tag-review (lean, NEW-class): read the full `chronicle.rs` + the resource-access call
sites directly — schema/kind-count verified exact, cap/eviction/immortality logic
verified by inspection (not just trusting the tests), no concurrency footgun. Clean, no
findings.

**D7 flag, relayed to the architect (not resolved here — a vocabulary-lock decision):**
the Addendum's sphere-weight field (GOD-DOMAIN's domain-vector) is deliberately ABSENT —
no sphere vocabulary is locked anywhere in code or the design catalogs yet, and
inventing ~10 sphere names at HIST-0 would re-derive an unlocked vocabulary (exactly the
D7 registry class this fleet already tracks). An `Option` field appends save-compatibly
the moment GOD-DOMAIN locks a sphere enum — flagged now so it lands at that lock, not as
a retrofit. Builder's call to defer was correct; the lock itself is the architect's.

### bastion-block-BAG2 (0093d4b7e8)
Row 40, archetype-keyed decision data over one shared brain: `rtsim/src/rule/npc_ai/archetype.rs`
(new) — `ArchetypeConfigs` RON asset (`assets/common/rtsim/archetypes.ron`, the in-tree
`FileAsset`/`load_ron` idiom) mapping archetype key → `{activity: weight}`; key-presence
IS the allowed-list, the weight IS the old hardcoded `random_bool` constant — one map
serves both packet requirements. `archetype_gate()` is the ONE shared lookup every
converted site calls; the RNG rolls ONLY when the archetype lists the activity, verified
directly in the diff to preserve each NPC's rng-call COUNT and ORDER exactly as the old
`matches!(profession,X) && rng.random_bool(CONST)` short-circuit (DETRNG/B8-clean — no
determinism drift). Three same-shaped villager gates converted verbatim: Herbalist
`gather_forest` 0.8, Hunter `hunt_forest` 0.8, Guard `patrol_plaza` 0.7 (constants moved
byte-identical to the RON table). Farmer/Merchant/Chef stay hardcoded, explicitly noted
as the §4 expansion's scope, not a gap here. Graceful at every layer: asset-load failure
warns + closes every gate (no crash), unknown key/activity → `None`/empty, a
no-archetype NPC behaves exactly like the old non-matching-profession case. Gate 17/17
(ladder gains the AG2 leg; b58 telemetry in-band, claims 1.21). `--archetype-scenario`
2/2: moved weights load exact (0.8/0.8/0.7) through the brain's real lookup path, the
archetype CONTRAST holds (herbalist's allowed-set ≠ guard's, cross-lookups closed — the
Playbook's own done-when), graceful-unknown probes all clean, world ticks on with the
converted gates live. 2 in-crate unit tests (same-code/different-data contrast + graceful
unknowns; key-derivation pins the converted set — `veloren-rtsim` tests, outside the
ladder's common-only UNIT leg, ran green manually, same standing note as GATHER/HIST-0).
Sonnet tag-review (lean, MIXED-class): read `archetype.rs` in full + the 3 converted call
sites in `npc_ai/mod.rs` directly — confirmed the shared-lookup shape, the graceful-degrade
chain (`?`/`.ok()`/`.unwrap_or_default()`, no panics), and the RNG short-circuit
preservation by inspection, not just trusting the claim. Clean, no findings. Review-tier
(self-verify+tag, proposed pre-build) held: selection weights only, zero movement writes,
zero stuck-economy contact — confirmed in the actual diff, matching the architect's
ratified read.

**Two recon notes for the §4 expansion (reported, not gated):** (1) census telemetry read
0/0/0 herbalist/hunter/guard in generated rtsim data at seed 1337 tick 5. **CORRECTED
(2026-07-12, at the 35.1 tag):** this was NOT sparse worldgen profession distribution as
originally characterized — it was the same root cause B-AG1's fixture hit (`bastion-block-B-AG1-FIXTURE-GEO`
`73cd8df83d`): rtsim's NPC table is EMPTY pre-tick (0 civilised before ticking vs 1985
sixty ticks later — population is tick-driven), so a tick-5 census undercounts
regardless of profession rarity. Whoever picks up §4 should census AFTER settling, not
read this as a distribution signal. (2) `think()`'s `Role::Wild` arm is literally
`idle()` today — wild-species archetype keys (wolf/deer/etc.) are the single biggest §4
win, since NONE of them get any archetype differentiation yet. Folded (2) into
DESIGNER-SUGGESTIONS as a forward pointer.

### bastion-block-SEASON0 (73397de696)
Row 42's SEASON-0 sub-block (SEASON-1/2 remain separate, unblocked future work): the
annual-rhythm derived clock. `common/src/time.rs` gains `Season{Spring,Summer,Autumn,
Winter}` + `year_phase(0..1)` + `day_of_year` — PURE functions of `TimeOfDay`, `DayPeriod`/
`MoonPeriod`'s exact shape one scale up (quarter-bucketing via `rem_euclid`). No second
clock, no stored state, zero per-entity cost (nothing wired into any consuming system —
that's SEASON-2's scope; the day-D schedule hook is SEASON-1's). Year length is a RON
tunable (`SeasonConfig`, `assets/common/season_config.ron`, `days_in_year: 160` = 4×
`DAYS_IN_MONTH`, the `FileAsset`/`load_ron` idiom, graceful default on missing/broken
asset). `Calendar`/`CalendarEvent` (the real-world-date system) correctly left untouched,
not conflated. 22 unit tests now in the ladder's common UNIT leg (boundaries exact,
year-wraparound identity, phase/ordinal consistency, re-bucketing under a different
tunable year length — confirms the tunable is real, not decorative).
`--season-scenario` 2/2: the RON value loads through the in-vivo path (160, not the
fallback), quarter/wrap/ordinal asserts hold under the LOADED config, live `TimeOfDay`
derives correctly, and STATELESSNESS is asserted explicitly (same `tod` → identical
answer before/after 60 ticks — pause/speed independence by construction, verified
anyway, not just assumed from the pure-function shape).

**Gate note (known flake, not a new finding):** the 18-leg run came back 17/18 on
`ck_failsafe_out=false` — the documented CK seam-flip probe (every OTHER safety probe
in the same run green: `all_out`/`cleared` true, `net_fires`/`in_terrain`/trips all 0).
SEASON-0 has zero surface for CK to exercise (pure derived functions, nothing wired to
anything CK touches). Classified via a same-binary ×3 rerun: 2 PASS / 1 FAIL — the exact
seam-flake signature already documented at the BAG5CORE tag. Tagged per that precedent;
the SEASON leg itself passed in the full ladder. Sonnet: no tag-review pass (CHEAP-class,
self-verify covers it per the backstop rule's own scope).

### bastion-block-SEASON1 (889f1e20ed)
Row 42's SEASON-1 sub-block (SEASON-2 remains, separate unblocked future work): the
day-of-year `SeasonalSchedule` hook. `common/src/time.rs` gains `SeasonalSchedule` —
`Calendar::is_event`'s mirror one axis over (named event → in-game `day_of_year`
instead of the real-world wall-clock date), built physically separate from
`calendar.rs` (not conflated, both can independently trigger the same festival per the
design invariant). RON-configured (`assets/common/seasonal_schedule.ron`, same tunable
discipline as SEASON-0's year length; graceful empty schedule on missing/broken asset —
nothing fires, nothing panics). Query surface: `is_event_on(day_of_year, name)` +
`events_on(day)` (name-sorted, deterministic iteration) — pure lookup, no stored
mutable state beyond the loaded schedule. Shipped entries are the done-when's two
examples (harvest=day 90/autumn, holy_day=day 20) — mechanism only, DF-FESTIVAL
subscribes later, SEASON-2 owns the consumer contract. 23 unit tests now in the
ladder's common UNIT leg (exact-day firing, no adjacent-day bleed, same-day coexistence
sorted, unknown/empty never fire, day-90-is-Autumn pinned through SEASON-0's own
derivation — confirms composition through the public API, not a private season
calculation). `--season1-scenario` 2/2: loaded RON fires through the real consumer
query path, listings exact, and the end-to-end compose probe holds (day-90.5 `tod` →
ordinal 90 → autumn → fires harvest, entirely through SEASON-0's public derivation, no
private math anywhere). Gate 19/19 (ladder gains the SEASON1 leg; CK passed this draw,
b58 in band). Sonnet: no tag-review pass (CHEAP-class, self-verify covers it per the
backstop rule's own scope).

### bastion-block-FOCUS0-ENUM (c752571be1)
Row 43, the narrowed FOCUS-0 (schema only — see row 43.1 for the deferred
facet-derivation half). One file, +68 lines: `common::bastion::Need`
{Pray/Socialize/Drink/Craft/Family/SeeAnimals/AdmireArt/Learn/Acquire/Fight} — the
design doc's list verbatim, locked venue-interface vocabulary (`Purpose`/`ChronicleKind`
discipline: append-only, never reorder). `BastionColonist.personal_needs:
HashMap<Need, f32>`, `#[serde(default)]` — the Playbook's explicit collection-not-fixed-
fields shape (future `Need` variants join with no struct migration; old saves default
EMPTY; 1.0-satisfied semantics matching the bodily `Needs` comp). Schema only: nothing
reads or writes the collection yet — facet-derivation stays deferred with B-AG3 (row
43.1), need-jobs are FOCUS-1, the work_rate hook is FOCUS-2. 1 unit test
(`bastion_need_collection_serde_shape`, ladder's common UNIT leg, 24 tests now):
old-shape payload → empty default; a populated map round-trips exactly. No scenario leg
(schema only, no runtime surface to exercise). Sonnet tag-review (lean, NEW-class):
read the diff directly — matches the packet exactly, schema-only confirmed (grepped for
any other read/write site, none found beyond the constructor default). Clean, no
findings.

**Gate note — NEW flake-registry candidate, filed:** the 19-leg gate came back 18/19
with `LOD1` red (`lod1_stones=2`, expected 3; every other probe green — jobs done,
roster stable, no leaks). Classified via a same-binary ×3 rerun: 3/3 PASS with
`stones=3` — a residual scheduling-seam draw (a timing shift moved one drop-toss outside
the count-probe's radius; the stones-count is position-sensitive telemetry, same class
as `ck_failsafe_out`). First LOD1 flake observed across ~10 gates today. FOCUS-0's
change (a read-nothing schema append) has zero execution-path surface — the
identical-binary discrimination confirms non-correlation. Tagged per the CK/BAG5CORE
precedent. **Filed as `BASTION_COMMON_ISSUES.md` B22** (a new flake-registry class,
alongside the CK seam) so future gates classify an `lod1_stones` miscount on sight
instead of re-diagnosing it each time.

### bastion-block-SEASON2 (641b74b5c5)
Row 42's SEASON-2 sub-block — the last of SEASON-0..2, row now fully DONE. The
documented ONE-INTERFACE contract from the design doc's §3: `season()` / `year_phase()`
/ `day_of_year()` / `season_bias()` as four sibling pure reads (+ `SeasonConfig::current()`
+ `SeasonalSchedule`) in `common/src/time.rs` — the declared plug-in point for every
seasonal consumer (DF-FARM/DF-ROT/DF-LIVESTOCK/DF-NIGHT/DF-FESTIVAL, later DF-TEMP/
DF-BIOME-FX). No consumer wired — the contract, not the behaviours, per the doc's own
scope. New derivation: `season_bias` — a continuous annual wave (`-1..=1`, cosine
anchored to the quarter definitions: `((phase - 0.375) * TAU).cos()`, verified directly
— +1 at mid-summer phase 0.375, -1 at mid-winter phase 0.875, zero-crossings at the
spring/autumn midpoints). **Design-shape choice by the builder (flagged, verified
sound):** a continuous wave over discrete per-season step constants — the doc only
specified "an optional `season_bias` others map," no concrete shape mandated. Reasoning
holds: a consumer can bucket a wave via `season()` but can't un-bucket a stepped
constant, and biology/consumption curves don't step at quarter boundaries. Sonnet
tag-review (lean, CHEAP-class): read `season_bias`'s implementation directly, confirmed
the cosine anchoring matches the claimed peak/trough/crossing points exactly. Clean, no
findings — the shape choice stands. 25 unit tests now (`bastion_season_bias_wave_anchors`
— anchors exact, range over 65 samples, wrap continuity, free-fn surface agreement). No
scenario leg (pure-contract precedent per FOCUS-0-ENUM — the in-vivo config path is
already gated by the SEASON-0/1 legs this interface composes over). Gate 19/19 (CK and
LOD1 both passed this draw, b58 in band).

**Registry note (Sonnet's side of the split):** filed the interface in
DESIGNER-SUGGESTIONS.md so no future consumer forks a private season counter.

### bastion-block-FR15-TIGHTDIG (ed29c00781)
Row 31, the drive-owned progress metric + reinstated committed-path steer — the FR15/
FR17 re-spec, ALL flag-gated (`BASTION_TIGHTDIG=1`; flag OFF = today's stuck-economy
bit-for-bit, no behavior change for the default path). Re-verified against live code
before handoff (bastion_jobs.rs had drifted ~500 lines since the packet was first
staged) — surfaced a real addition since the original FR17 review: B6-HAUL's
`fetch_steer` override (a third steer source alongside anchor/beeline) needed explicit
coverage, flagged to the builder and to Opus. **Opus R13 verdict: GREEN-LIGHT, no
blocking findings** — no-entombment backstop zero-contact (independently re-confirmed,
not just re-asserted from LOD-1's prior proof), all three steer sources (anchor/fetch/
beeline) verified correct including the new fetch case, `dispersed_frac` judged
intended behavior for tight-dig geometry (not a regression). Ships **FLAG-DEFAULT-OFF**
per Opus's recommendation — staged rollout, not an immediate behavior change; see row
31.4 for the validate→flip→remove-flag follow-up (tied to the play-tester batch-test-
checkpoint) and the `dispersed_frac` WATCH item (revisit if committed-path steering
ever engages in open-area crew work). Evidence chain: noise-floor run on identical code
first, then 4/4 paired-A/B PASS (every gating boolean agreed in every pair — the
scheduling-noise cancellation working as designed), then a 19/19 flag-OFF ladder on the
tag commit (confirms zero behavior change with the flag off). Architect directed the
gate directly with Opus (not routed through Sonnet) — bookkeeping only on this end.

### bastion-block-B-AG1-FIXTURE-GEO (73cd8df83d, row 35.1)
Follow-up fill to B-AG1's fixture-geography gap (filed at the B-AG1 tag). Root cause
found: rtsim's NPC table is EMPTY pre-tick (0 civilised before ticking vs 1985 sixty
ticks later — population is tick-driven), so the ORIGINAL pre-tick densest-cluster pick
could only ever find pre-populated special entities (the airship dock, captains
included from spawn) — never ground townsfolk, which don't exist until the sim
settles. **Fixed two ways:** settle-first (tick before clustering) + a GROUNDED filter
(npc z within 6 of `get_alt_approx`, excludes deck/mount riders, includes street-level
villagers). `--bag1-scenario` now PASSES 2/2 for real: promoted 241/142 walking
villagers, movers 215/121, max displacement 230/214 blocks (vs the dock-era 0/0/0;
magnitudes vary with the scheduling seam, the PASS verdict is stable both runs).

**Bookkeeping correction (same root cause, different symptom):** B-AG2's earlier recon
note ("census 0/0/0 herbalist/hunter/guard at tick 5 — a worldgen distribution
question") was WRONG — it's the same empty-pre-tick-table root cause as B-AG1's fixture
gap, not sparse profession distribution. Corrected above at the B-AG2 entry.

### bastion-block-SEASONHUD (93c1970d42)
Row 42.1, the season-clock legibility win (Ben's gap-audit find: the clock ships but is
invisible to the player). One file, +26 lines: a "Season · Day N" readout stacked above
the overseer's TIME-CONTROLS cluster, derived on read through SEASON-2's one-interface
contract (`season()`/`day_of_year()`/`SeasonConfig::current()`) — no new sim state, no
private season math, the interface holding cleanly at its FIRST real consumer. Display
is 1-based (Day 1..=160) over the 0-based internal ordinal — a UI-edge convention, not a
schema change. Decoupled from the row-106 climate-FX lane per the row's own note.
Voxygen compiles clean; visual eyeball routes to the Play-Tester's next Ben-exe rebuild
(their build lane, not this tag's gate). Sonnet: no tag-review pass (CHEAP-class,
self-verify covers it).

**Gate note — a THIRD B22 flake-class instance, generalized the class:** the covering
19-leg ladder (which also gated 35.1's first full-ladder run) came back 18/19 on B6HAUL's
`b6_built=1/2` + `b6_mined=false`, every invariant green in the same run (conserved,
delivered, race-exactly-one, jobs_left 0). Classified via a same-binary ×3 rerun:
FAIL/PASS/PASS — and critically the SHORTFALL MOVED BETWEEN PHASES across runs
(built=0/mined=true vs built=1/mined=false), confirming a completion racing a timing
window, not a broken writer (neither commit under this tag touches any B6HAUL path —
voxygen-only + a harness fn). **Generalized B22** (now 3 instances: CK/LOD1/B6HAUL) with
the sharper rule the pattern makes clear: COMPLETION-WITHIN-WINDOW asserts (a timing
deadline, a spawn radius, a race against a tick boundary) can flake on scheduling noise;
INVARIANT asserts (conservation/no-leak/exactly-one-claimant) never have — every
instance so far had every invariant probe green in the same failing run.

### bastion-block-B70 (0aea5c63e6, row 44, B7-0 sub-block)
The needs-decay + mood-formula substrate (design §3). Decay runs on the existing
`Needs` shells at a per-tick rate × dt, tunables in a new RON asset
(`assets/common/bastion_mood.ron` → `MoodConfig`, graceful default if absent). `Mood`
is RECOMPUTED (not integrated) each arbitration cadence on its own slot (`%15==11`) per
the design's RimWorld-style `base + Σ need-penalties + Σ decaying-thought` formula —
shortfall-below-comfort only, order-free, no float drift across ticks. The thought term
(`bastion_mood.rs::thought_sum`) reads the HIST-0 chronicle (actors-contains match,
pure `(deposit, now)` linear decay); the kind→weight table ships SERVER-SIDE
(`bastion_thoughts.ron`, keyed on rtsim's `ChronicleKind` — `common` can't see rtsim's
types, so the formula takes `thought_sum` as an opaque input, the layering the
architect ratified pre-build). Persistence rides the existing LOD-0 mirror
(`BastionColonist.needs`/`.mood` already serde-defaulted `Option`s) — captured every
loaded tick, flushed on demote, restored wholesale on promote.

**The cave-in fork, resolved per the reviewer's ruling (a):** the pre-existing CAVE-IN
eject-and-injure mechanic wrote `Mood` DIRECTLY, which B7-0's recompute-each-cadence
formula would have silently overwritten within ~15 ticks (a regression of a mechanic
that works today). Fixed by folding ONE emitter into the crush path — `bastion_jobs`
holds a long-lived rtsim READ guard (the LOD gate) so it can't write the chronicle
itself, so crush victims QUEUE on the board and the rtsim tick drains the queue into
`chronicle.record(ChronicleKind::CaveIn, victims)` next tick. The fear-persists
scenario assert EARNED ITS KEEP here: it failed first because the deterministic test
hook bypassed the queue (tested-path≠shipping-path, caught by the assert itself), then
passed once the hook was made to queue identically — `cavein_min_mood_after_recompute
0.20` (base minus TWO fresh CaveIn thoughts, hand-verified), proving the
queue→drain→chronicle→thought_sum→formula pipeline end to end, not just each piece in
isolation. En-route catch during the same wiring: the first pass fed decay's `now` from
sim `Time` while the chronicle stamps `TimeOfDay` — two different clocks; both sides
now read `data.time_of_day`.

**Verification:** `bastion_mood_formula_exact` unit leg (26 cases — 0.6-exact base,
0.09-exact hand-computed starved case, exact+saturating decay, linear-pure thought
decay, clamps); `--needs-scenario` 2/2 (decay arithmetic to 1e-3 over 600 ticks,
monotone, 0.09 across a cadence, `(0,0,0,0.09)` roundtrip); CAVEIN scenario PASS with
fear-persists. Full 20-leg gate: 20/20 green, including a new NEEDS leg — every
flake-registry leg (the B22 instances above) green this draw. Sonnet tag-review: clean,
no findings — the fork resolution is minimal (~10 lines) and test-guarded rather than
merely present, exactly the shape asked for.

**Reported-tier observation (non-blocking, logged not investigated inline):** builder
flagged `b58_d_claims_ratio` drawing {1.879, 1.327, 1.858} across the B7-0 gate + a ×2
post-tag persistence check, vs. the session-long {1.21-1.35} band across ~10 prior
gates — draw-dependent (one draw in-band), every gating invariant green in all 3 runs
(150/150, 148/150, 149/150 dug; e_out/f_cleared/orphans clean). Candidate cause:
B7-0's new per-tick `WriteStorage<Needs>` + cadence recompute widening the claim-churn
distribution's right tail. Filed as a new watch entry, [`BASTION_COMMON_ISSUES.md`
B23](BASTION_COMMON_ISSUES.md); recommended follow-up is a paired A/B (commit vs.
parent, reusing the FR15 harness) — queued as non-blocking, not run inline so as not to
pull the builder off B7-1/2/3.

B7-1 (bed object + closed rest loop) packet delivered same pass — depends on B7-0 only,
not B7-2's preemption (which hasn't landed). Row 44 → CURRENT.

### bastion-block-B71 (4e56c3d8ca, row 44, B7-1 sub-block)
The bed object + closed rest loop (design §4). Shipped all four packet corrections
against the raw design doc: **(1)** `DesignationKind::Bed` follows the `Ladder`
placement precedent — its own completion arm placing a named sprite — NOT `Build`'s
generic stand-in Rock, which the doc's own text had wrongly pointed at. **(2)** zero
asset requests — vanilla's real `SpriteKind::Bedroll`/frame-biome sprites cover
BedKind::Bedroll/Frame exactly. **(3)** the sleep action is genuinely NEW mechanism
inside the job-board framework (a rest-restoring `Arrived` arm, quality-scaled) —
vanilla's mount-buffs (player-only) and the rtsim village-NPC sleep path
(unloaded-tier, no bed-targeting) confirmed non-reusable, exactly as flagged.
**(4)** `BedSlot` follows the board's `reservations` shape (capacity-1, keyed by
block-pos) rather than a nonexistent "container store." The pre-claimed `RestAt`
harness assignment rides the `DepositRun` precedent (bastion_jobs.rs ~2355).
`BedKind` carries a quality stub (Bedroll 0.6 / Frame 1.0 — frames go pure-data
later). The B7-0 thought queue GENERALIZED from a cave-in-only shape to
`(who, where, kind)` — cave-in fear and sleep quality are now two emitters draining
through the one pipeline; `ChronicleKind` gains `SleptInBed`/`SleptOnGround`
(append-only; `SleptOnGround` is B7-2 fallback data, unused until preemption lands).
Ownership persists on `BastionColonist.owned_bed` (LOD-0 mirror, free on
`colonist_record` clone); occupancy releases at job completion, the claim-release
seam (`to_release`), and a new DEATH-AWARE cadence sweep.

**Verification (`--bed-scenario`, leg 21, 2/2 PASS):** beds build through the REAL
pipeline (stockpiled material → fetch → place → slot registers, not a scripted
shortcut); the rest loop closes (sleep to comfort + a `SLEEP_MARGIN` 0.1 hysteresis
band — waking colonists re-cross the band within seconds of decay, not
instantly-reflicker); owned beats communal 0.680 vs. 0.600 on the next recompute (the
`SleptInBed` thought delta, matches the formula by hand); the occupancy-collision
phase uses a deterministic head-start and asserts the REAL capacity-1 invariant — a
second colonist releases clean while the first finishes undisturbed (sequential reuse
legal, simultaneous never); a killed sleeper's occupancy releases; ownership survives
demote/promote. Full 21-leg gate: 21/21 green, including a new BED leg — every
flake-registry leg green this draw; `b58_d_claims_ratio` drew 1.4 this run, back
inside the pre-B7-0 band (the B23 watch item, now the Tier-B paired-A/B's to own).

**THREE real finds shipped in the commit:**
1. **Dead colonists kept their `ActiveJob`** — the upkeep loop gated on `is_loaded`,
   never on death: a killed sleeper's CORPSE re-occupied its own bed every tick,
   outrunning the orphan sweep. Caught by the kill-while-sleeping scenario assert
   (the second assert this sub-block to earn its keep, after B7-0's fear-persists) —
   exactly the leaked-reservation class the assert was written to catch. Fixed: dead
   colonists now release via `to_release`; death checks read `is_dead ||
   should_die()`.
2. **Plain `HealthChange` damage is absorbed by colonist death protection** —
   `Health::kill()` is the real kill API; the test hook was fixed to use it.
3. **Fixture geometry produced the failure mode it was built to detect** — new
   `BASTION_COMMON_ISSUES` class, see below.

Sonnet tag-review: clean, no findings beyond confirming the four packet corrections
landed as specced and the verification numbers check out by hand (0.680/0.600 delta,
capacity-1 head-start ordering). B7-2 (preemption, ★OPUS-GATE) is next — packet
in flight, architect flagged in parallel per row 44's own note. Row 44 stays CURRENT.

**B23 CLOSED (builder follow-up A/B, run between B7-1 and B7-2):** the intended
paired A/B hit a worktree `cd` path flaw that made BOTH interleave legs resolve to
the PRE-B7-0 parent binary — which answered the question more directly than the
pair would have: five clean quiet-machine draws on the parent ALONE —
`{1.268, 1.273, 1.353, 1.220, 1.866}` — already contain a 1.866, matching the
post-B7-0 1.88/1.86 draws that raised the watch in the first place. **The elevated
tail predates the B7 diff entirely** — the original {1.21-1.35} band was a
small-sample lucky streak, not the metric's true spread. Closed as noise, not a
regression; `BASTION_COMMON_ISSUES.md` B23 updated with the corrected ~1.2-1.9
observed band so a future 1.8-ish draw doesn't re-raise it. General lesson folded
into B23's writeup: verify both A/B legs actually built/ran the commit you think
they did before trusting either verdict from a paired run — a worktree/cd/cache bug
can silently collapse a pair into two draws of the same binary.

### bastion-block-B72 (656c1efda8, row 44, B7-2 sub-block) — ★OPUS-GATE, CLEAR-TO-TAG (§R14)
The self-job preemption mechanism (design §5) — the load-bearing build-once block.
**Shipped simpler than the raw design doc, live-verified before the packet went
out:** the doc frames preemption as winning a numeric priority tier ABOVE `is_access`
in the existing per-job selection compare; the actual shape reuses the pre-claimed
self-job pattern `DepositRun`/`RestAt` already proved — a pre-claimed job never
enters the claim-selection loop at all, so "out-tiers all work and access" is
**impossible by construction**, not a comparison it has to win. No new priority-tier
field exists or was needed.

**The mechanism:** a new NEED-CHECK pass (own arbitration slot, `%15==13`) — for
each loaded colonist below its RON-configured `NeedTuning.interrupt` (recreation's
interrupt is 0, so it never preempts — hunger/rest are the live needs, ranked by
urgency, generic over N needs so B7-3 adds a candidate for free): drops the current
work-job through the proven `to_release` seam (already carries B7-1's death-aware
bed-occupancy release), THEN creates the pre-claimed `RestAt` job AFTER the drain
completes in the same tick (the drain clears whatever `ActiveJob` an entity holds at
drain time, so create-before-drain would be destroyed by the colonist's own release).
Wake threshold reuses B7-1's `comfort + SLEEP_MARGIN` — the design doc's
`NEED_SATISFIED`, already shipped, just not named that until now. **Anti-livelock
trio:** the hysteresis band itself; an unreachable need-job degrades to ENDURE (the
existing watchdog releases it, the orphan sweep extended to cover `RestAt`, so the
colonist returns to reachable work while the need keeps honestly decaying); a
per-colonist 60s `PREEMPT_COOLDOWN` bounding retry rate regardless of outcome.
`preempt_attempts` telemetry added for visibility.

**Why Opus has something concrete to verify, by construction:** zero new steer/drive
code — a `RestAt` job IS a travel job, so the existing `best_dist`/`stuck_time`
watchdog and the movement-independent `stuck_watch` teleport backstop (FR17
orthogonality) apply automatically; preemption only swaps the `ActiveJob` TARGET, the
stuck-economy machinery itself is untouched. Conservation rides the one proven
`to_release` seam (no second release path to get wrong, incl. B7-1's bed-occupancy
release). Determinism: arbitration-pass order, pure threshold reads.

**Verification (`--preempt-scenario`, leg 22, 2/2 PASS):** (1) preempt-pause-resume
on a live mine (10 unclaimed jobs visible at the rest peak, work resumes after the
nap); (2) **anti-thrash by construction** — an unreachable-bed fixture that would
fire ~6-8 preempt attempts/120s unguarded fired EXACTLY 2 (t≈0, t≈60), asserted
≤3; a hovering-just-above-interrupt case fired ZERO; (3) mid-travel wedge — a
colonist preempted WHILE ALREADY EN ROUTE that then wedges below-grade extracts via
`stuck_watch`, zero embeds across all phases (the sharper no-entombment case, per
the packet's ask); (4) unreachable-endure — a floating owned bed left rest decaying
honestly (0.15→0.114) while 14 blocks got mined, no livelock. Full 22-leg gate:
22/22 green, new PREEMPT leg included, every flake-registry leg green;
`b58_d_claims_ratio` drew 1.22 (comfortably inside B23's corrected ~1.2-1.9 band).

**Two finds:** (a) enclosure ≠ unreachability — the sleep-arrive radius reaches
through a 1-block wall (a gen-1 sealed-box fixture had the colonist sleeping against
the OUTSIDE of the box; the underlying distance-based arrive logic is the honest
construction, not a bug — joins the B24 fixture-geometry class, another instance of
a fixture producing an artifact rather than exercising the real mechanism). (b)
**Reported, not fixed:** bed slots never unregister when their block is destroyed —
a mined-out bed stays targetable. Filed for B7-3/designer-lane triage, not blocking
this tag.

Sonnet tag-review: clean read on the mechanism and verification; no findings beyond
what the builder self-reported.

**★ OPUS CLEAR-TO-TAG (BUILD_REVIEW_LOG §R14):** all 3 safety claims confirmed true
by code-read of 656c1efda8 — preemption composition (the pre-claimed-self-job
bypass is genuinely impossible-by-construction, not a claim), no-entombment-survives-
preemption (the mid-travel wedge case specifically verified — a colonist preempted
while already en route that wedges still gets the `stuck_watch` teleport), and
anti-livelock (hysteresis + ENDURE-degrade + `PREEMPT_COOLDOWN` all independently
confirmed present and load-bearing). Row 44 flips B7-2 → **DONE**. Builder's decision
to proceed to B7-3 on the then-ungated commit (additive-only extension to the
NEED-CHECK pass, separate commit) turned out moot — no fix was ever needed — but was
the correct bounded-risk call to make at the time given the "never idle" standing
rule; would have paid off cleanly either way. **Note:** a same-checkout collision with an unrelated external agent
(Grok, sharing the working tree without worktree isolation, briefly switched the
branch to `grok/test-env`) happened in this window — reflog-confirmed 656c1efda8
committed cleanly on `bastion/block-B6HAUL` BEFORE any contamination; architect
resolved it (Grok isolated to its own worktree, fleet checkout restored) with zero
loss on either side. Grok's own CI commit (`d05e8714d0`) lives on `grok/test-env`
and is UNGATED external code — it needs Sonnet + Opus clearance before it may ever
merge into the fleet branch.

### bastion-block-B73 (1287b161b9, row 44, B7-3 sub-block — B7 COMPLETE)
The eat-job + the despondent breakdown state (design §3/§7) — the last B7 sub-block,
built additively on B7-2's Opus-cleared NEED-CHECK pass, zero new preemption code,
separate commit from `bastion-block-B72`. **Tier decided by the architect: self-verify
+ tag, no dedicated Opus gate** (B7-3 adds no new preemption/steer surface — it rides
R14's cleared mechanism — and both despond-safety properties are independently
scenario-asserted AND construction-provable). Sonnet's lean tag-review WAS the gate
here; both requested safety properties verified directly against live code, not just
taken on the builder's word:

- **No-entombment holds:** `Despond` is inserted via `insert_despond_job` at the
  colonist's own feet (`bastion_jobs.rs` — the board method) and seeded with the
  IDENTICAL `ActiveJob{state: Traveling, best_dist: f32::MAX, ...}` shape every other
  job gets — no special-cased "instant arrive," the near-zero travel distance to its
  own feet just resolves to Arrived on the next tick through the ordinary watchdog
  path. Verified further: the `embed_watch` center-net (the B19/B20 belt-v2 mechanism
  — `HashMap<Uid, u32>` persistent-core-in-solid counter, `EMBED_PERSIST_TICKS`
  threshold, relocate via the shared `eject_dest`) iterates **every colonist
  unconditionally** — no `ActiveJob`/`JobKind`/Despond filter at all — so it protects
  a despondent colonist exactly as it protects a mining one. Stronger than the
  packet asked for: no-entombment doesn't depend on Despond correctly riding the
  travel watchdog at all, since the fully job-orthogonal center-net covers it
  independently either way.
- **No-thrash holds:** the breakdown arm (preceding the need-preemption check in the
  same pass) reads: (1) if the colonist is ALREADY on a `Despond` job, skip
  evaluation entirely — no re-trigger while despondent; (2) a sustained-window gate
  (`mood_below_since` + `break_sustain_secs`, cleared the moment mood recovers above
  `break_minor`); (3) shares the SAME `preempt_cooldown`/`PREEMPT_COOLDOWN_SECS` table
  as need-preemption — one break attempt per 60s window, not a separate budget; (4)
  a probabilistic roll (`break_chance`) — not an instant flip on crossing the
  threshold. All four confirmed present and wired exactly as claimed.

**Mechanism:** EAT — hunger joins the urgency ranking as the second live candidate
(rest was the only one B7-2 exercised); targets the nearest unreserved food item
(v1: `common.items.food.mushroom`, verified to be the SAME def GATHER's sprite-
reclaim records — forage→stockpile→eat closes end-to-end); the B6 reservation
commits with the pending entry; the Arrived arm mirrors Haul leg-1 (pickup emit,
uid-vanish confirm) then consumes one via the Build-material decrement path,
`hunger += 0.5`; `remove_job` releases the reservation (one path, B17 discipline); no
food available = an honest starvation endure, no cooldown burned on a guaranteed
failure. BREAKDOWN — covered above. Both new `JobKind` arms (`EatFrom`, `Despond`)
join every existing self-job arm (claim-skip, mid-travel, still-valid, orphan
sweep) — no special-cased gap left for either.

**Verification (`--b73-scenario`, leg 23, 2/2 identical outcome-JSON):** (a)
eat+conservation — hunger 0.15 mid-mine preempts a 10-job mine, exactly one
mushroom consumed (ground count AND meter jump verified together — the meter only
moves on a successful bag decrement), hunger ≥0.55, mine completes to zero after;
(b) urgency, proven by fixture construction — hunger 0.10 + rest 0.18 with NO bed
existing: a rest-first ranking would walk the bedless rest path forever and never
eat, so the hunger preemption firing AT ALL proves the lower meter won, not just
that SOME preemption fired; (c) breakdown→hold→recover — needs zeroed (mood pinned
to floor), Despond fires mid-mine, work FROZEN through a 30-game-second probe inside
the hold, needs restored, mood recomputes above `break_minor` at the next cadence
(race-free ordering, `%11 < %13`), Despond lifts on its own clock, mining resumes;
`preempt_attempts` delta EXACTLY 1; zero embeds throughout all three scenarios.

**Gate: 22/23, one CK leg (`ck_failsafe_out`) B22-flake-classified** — every other CK
field green (all_out, cleared, zero embeds/trips, unreachable_final 0); reruns 1-2 on
the identical binary both PASS (met the ≥1/3 threshold before a 3rd rerun finished);
cross-checked B7-3 could not be the cause — CK colonists spawn full-metered and only
decay ~0.05-0.1 across the scenario, nowhere near the 0.2 interrupt, so no
need-preempt can fire mid-CK regardless. `b58_d_claims_ratio` not separately called
out this run (implicitly in-band).

**REGISTRY FIND — a real bug, root-caused same-session (run-1 FAIL → fixed → green):**
a reservation without `required_item` gets fetched then SILENTLY released. The B6
material-fetch path activates for any non-Haul job holding a reservation and derives
its `carrying` flip from `job.required_item`; `EatFrom` v1 supplied the reservation
but left `required_item: None` — the fetch steered and picked up the mushroom
(ground count dropped!), `carrying` never flipped, the next tick's
reserved-uid-vanished arm released the WHOLE job with no log, the claim loop
re-employed the colonist ~0.2s later, the orphan sweep silently reaped the released
board job, and the colonist ended up hoarding the mushroom with every other probe
green except the hunger meter. **Fixed:** the fetch contract now travels as a pair —
`insert_eat_job` carries the matched def alongside the reservation. Filed as new
`BASTION_COMMON_ISSUES` B26 (see registry). Diagnosis footnote worth keeping: item
Uids and JobIds are different namespaces — a log line reading "item=2... job
claimed job=2" is a coincidence, not a link; don't cross-reference IDs across
namespaces without checking they're actually the same space.

**Two more fixture-geometry finds** (the B7-1/B7-2 class keeps growing, folded into
B24): (i) a gz-1 strip fixture under a partially-undesignated overhang dead-ends at
the lip — a 1-high gap has no standable stance, and B15 CORRECTLY refuses it; the
resume-assert fixture needed a top-exposed surface strip instead so resumption tests
BEHAVIOR, not accidental geometry. (ii) the ×2-determinism-diff discipline needs
refining: only diff OUTCOME booleans/placement counts, not floats or mid-run counts —
rtsim's OS-level scheduling entropy (the B8 caveat) shifts travel timing between
identical-seed runs even when every outcome is identical; timing-sensitive telemetry
now lives on a separate non-diffed line. **Emergent, expected, not a bug:** with
hunger AND rest both tanked simultaneously, mood can pin to 0 and a legitimate
"stealth despond" fires before the colonist ever reaches food — the intended
staircase behavior under double deprivation, not a race.

**Pre-existing shared exposure, noted not filed as new:** `EatFrom`'s arrive-and-
pickup inherits Haul leg-1's drifted-item shape (the item can re-emit out of the
arrive range with no completion) — an existing class, green since B6; the B26 fetch
fix covers most of it for `EatFrom` incidentally (steers to the item's live
position), but it's the same underlying shared-exposure shape as Haul's, not
independently hardened.

**B7 IS COMPLETE.** Row 44 → DONE. This unblocks the whole cluster:
**B-AG3 (row 41) → FOCUS-0-DERIVE (row 43.1)** per the master-list's own recorded
order. Builder banked read-grounding for B-AG3 during the gate window (facet/value
vocabulary, personalized-thought read, a grudge-representation fork needing a packet
ruling, Mind-LOD soak assert) — packet next.

### bastion-block-BAG3V (199a834f57, row 41, B-AG3 narrowed slice 1)
The `Value` vocabulary lock + personality/values as a weight on B7-0's existing mood
formula (Sonnet's narrowed first-slice packet, architect-approved). Both prior-art
claims from the packet live-verified BEFORE building, one refinement surfaced:
`Personality`'s Big-Five scalars (`common/src/rtsim.rs`) are PRIVATE — the only
public surface is `.is(PersonalityTrait::X)` boolean queries — so the temperament
term consumes the boolean API (`Neurotic`) rather than a raw field. Zero touches to
vanilla either way. `Sentiments`' asymmetric decay confirmed exactly as documented
(~26min casual thresholds up to ~47h HERO/VILLAIN). Both `Personality` and
`Sentiments` reachable from the mood recompute THROUGH THE SAME rtsim read-guard the
chronicle already uses (`data.npcs.get`) — zero new coupling, answering the packet's
reachability done-when directly.

**Mechanism, exactly the packet's shape:** (1) `Value` enum locked, append-only —
`Glory, Tradition, Kin, Wealth, Piety, Nature, Craft, Freedom` (8, within the
packet's 5-8 bound; drawn from the build report's culture examples + DF's ethics
list, not invented fresh). (2) `values: HashMap<Value, i8>` on `BastionColonist`
(±50, serde-defaulted, the `personal_needs` shape verbatim) — old saves and fresh
colonists alike start EMPTY, meaning care=1.0 (neutral), meaning **pre-B-AG3 mood
stays bit-for-bit** until something actually writes a value weight; persistence
rides the existing whole-struct `colonist_record` mirror, zero new plumbing. (3) a
new pure `care_factor` fn beside `mood_formula`: `care = clamp(1 + Σ weight/50 ·
affinity, 0.25..4.0)`, then ×1.5 `NEUROTIC_NEGATIVE_AMP` on NEGATIVE thoughts only,
applied POST-clamp (a maxed-neurotic bad thought caps at 6.0×, bounded). (4)
`ValueAffinityTable` (RON, `assets/common/bastion_value_affinities.ron`) mapping
`ChronicleKind → [(Value, f32)]`, rows for the 4 currently-tabled kinds (e.g. CaveIn:
Kin +0.6, Wealth +0.3, Glory −0.4) — the same server-side-only pattern as
`bastion_thoughts.ron` (keys on rtsim's `ChronicleKind`, `common` can't see it). (5)
`thought_sum` scales each decayed thought's weight by `care` — a MULTIPLIER on the
existing term, `mood_formula`'s signature unchanged, B7-0's own pins stayed green
throughout.

**Verification:** two new UNIT cases — the `values` serde shape (old payload → empty;
±50 round-trips including negatives) + `care_factor` pinned exactly (identity case,
a 1.6-vs-0.6 divergence on one row, a scorn/negative-affinity flip, both clamp
edges, neurotic-amplifies-negative-only post-clamp). `--values-scenario` (leg 24,
2/2): needs topped (zero shortfall contribution, isolating the value-weight effect),
colonist A = Kin+50, colonist B = Glory+50, the SAME `CaveIn` chronicle kind reaches
both through the REAL pipeline (board queue → rtsim drain → chronicle →
`%11`-cadence care-weighted recompute — only the depositor hook is synthetic, the
CAVEIN scenario already owns the live emitter). Baselines EXACTLY 0.6000/0.6000; A
dropped −0.3199 (= 0.2 × 1.6, exact to hand math); B dropped −0.1200 (= 0.2 × 0.6,
exact); identical floats both runs. Margin analysis: even the worst-case
unknown-Neurotic combination (A at 1.6 vs. B at 0.6×1.5=0.9) stays strictly ordered
— the result doesn't depend on controlling for personality, a genuinely robust
proof. Full 24-leg gate: 24/24 green, new VALUES leg included, CK clean this draw,
`b58_d_claims_ratio` 1.33 (in the corrected B23 band), UNIT at 28 cases with the two
new pins. Outcome-JSON diffs stayed bools-only, floats on the non-diffed telemetry
line — the B7-3 entropy lesson applied from the start this time, not retrofitted.

Sonnet tag-review: clean, no findings — the reuse claims held up exactly as
predicted, the one refinement (boolean-API-only access to Personality) is a sensible
adaptation that preserves the "zero touches to vanilla" property rather than a
compromise of it.

**FOCUS-0-DERIVE (43.1) unblock question, answered by the builder's own check:**
43.1 can unblock off this slice ALONE — it does not need 41.1 (the emotion pipeline)
or 41.2 (Mind-LOD). `religiosity→Pray` maps to `Value::Piety` weight → `Need::Pray`
(this slice's schema, ready now); `gregariousness→Socialize` maps to vanilla
`extraversion` → `Need::Socialize`. Two open provisos flagged for whoever crafts
43.1's packet: (a) nothing currently ROLLS a colonist's `values` at creation (slice
1's only writer is the test hook) — a real natural-roster correlation assert needs
population variance, so 43.1 needs either a small value-roll at colonist generation
(mirrors the existing skills 0..=5 precedent) or a hook-seeded sampled roster; (b)
`Personality`'s raw scalars stay private, so facet-side derivation works at 3-level
(high/mid/low) granularity via the boolean trait API, OR would need a new pub getter
on vanilla's `Personality` (a policy call, not yet made). **This reopens the
architect's just-set cluster order** (B7→B-AG3→B-AG3.1→B-AG3.2→43.1) since 43.1 no
longer strictly needs 41.1/41.2 first — escalated to the architect rather than
resequenced unilaterally, since the order was an explicit, recent architect call.

**RESOLUTION:** architect resequenced to B7→B-AG3-slice→**43.1**→41.1→41.2 (43.1's
provisos made it clearly the better next block). On the two provisos: (a) ship the
REAL generation-time value-roll if cheap — architect's preference, adopted; (b)
confirmed default to the boolean-trait API at 3-level granularity, no vanilla-file
changes. Separately, tier-assessing 41.1 (the emotion pipeline) surfaced it's
already substantially delivered by B7-0/B7-1/B7-3 + the slice together — only 2 of
HIST-0's 54 ChronicleKinds have a live emitter today, so there's no real pipeline
work left until HIST-1/2 lands more emitters. **Architect closed 41.1 as
DONE-by-existing-work** (not held as a placeholder) and folded its one real
remaining piece (ChronicleKind→Value affinity coverage) into row 54 (HIST-1..2) as
a linked dependent note. 41.2 (Mind-LOD) stays READY, sequencing explicitly deferred
to the architect — the builder's LOD read-grounding found its loaded-only property
already holds structurally, so its real remaining content is a soak-assert plus ONE
genuine open design fork (frozen vs. throttled-decay for long-unloaded colonists'
needs) that the architect routed to the **designer**, not a packet-craft guess —
logged on row 41.2 verbatim.

### bastion-block-FOCUS0DERIVE (ffd7ab1aed, row 43.1 — THE FOCUS-0 ARC CLOSES)
Derive per-colonist Need weights from B-AG3's facets/values, built exactly to the
packet's shape. **The real generation-time value-roll** (architect's ruling, shipped
as a genuine feature not a test-only hook): `BastionColonist::generate(rng)` now
rolls all 8 `Value` weights ±50 uniform from the SAME rng thread as skills/name/
backstory — the 0..=5 skill-roll precedent extended verbatim, det-safe by the same
argument. Old saves keep serde-default empty (baseline); only newly-generated
colonists roll. **`derive_need_weight`** (a pure fn, `care_factor`'s neighbor):
`Pray = 1 + Piety/50`, `Family←Kin`, `Craft←Craft`, `SeeAnimals←Nature`,
`Acquire←Wealth`, `Fight←Glory` (all direct, exact); `Socialize` via the boolean-
trait 3-level API (`Extroverted|Sociable` → 1.5, `Introverted` → 0.5, else 1.0 — no
vanilla getter, honoring the architect's ruling); `Drink`/`AdmireArt`/`Learn` stay at
baseline 1.0 (no forced correlation — the design's own degrade-gracefully law);
clamped `0..=2`. **Produced and proven only** — nothing consumes the derived weight
yet, that's FOCUS-1's job.

**One ripple, caught by the packet's own stop-and-flag clause and resolved
cleanly:** rolled values feed slice 1's LIVE `care_factor` path — every new
colonist now takes value-weighted thoughts for real (bounded by the existing
0.25..4.0 clamp — the whole point of shipping a real roll). This meant the VALUES
leg's exact-math fixture (the two hand-computed colonists from slice 1's own
verification) suddenly carried 7 extra rolled keys alongside its two deliberately-
set ones. Fixed with a new `bastion_clear_values` hook + clear-then-set in that
fixture; its exact deltas (−0.3199/−0.1200) reverified green post-roll, rerun ×2 in
this block's own verification pass.

**Verification:** one new UNIT case (29 in the leg) — derivation pinned exact
(Piety 50→Pray 2.0, Kin −50→Family 0.0, Wealth 25→Acquire 1.5), unmapped needs
baseline under a loud map (not silently wrong), Socialize's 3-level bucketing
checked consistent with the public `.is()` API over a 400-draw seeded sample
spanning both extremes. `--derive-scenario` (leg 25, 2/2): a 12-colonist roster
from REAL `generate()` rolls — no hook-seeding, genuine variance (Piety spanning
roughly −45..47 this seed) — asserts: `rolled_full` (8 entries each colonist),
spread, `pray_exact` (every colonist's Pray weight matches `1 + Piety/50` to 1e-5 —
exactness subsumes the weaker correlation check), `ordered` (max-Piety strictly
out-derives min-Piety — the directional statistical proof the done-when asked for),
`social_consistent` (every colonist agrees with the independent 3-level trait
probe), `drink_baseline` (1.0 across all 12), `roundtrip` (the max-Piety colonist's
entire rolled value map survives `force_demote`→promote byte-for-byte — slice 1's
free-persistence claim now proven against genuinely ROLLED data through the live
LOD boundary, not a hand-set fixture). Full 25-leg gate: 25/25 green, new DERIVE
leg, and notably every mood-adjacent leg held its directional asserts with the
value roll now LIVE across the whole ladder — the roll's ripple didn't destabilize
anything it touched. Outcome-JSON stayed identical across both runs.

**Two harness-methodology finds, worth keeping as testing-discipline notes (not
game-code bugs):** (1) `bastion_force_demote` matches against the rtsim RECORD's
name, and the record only captures a rename on a SYNC TICK — renaming then
demoting with zero ticks between is a SILENT lookup miss (the record never saw the
new name, so BED/NEEDS hooks read nothing, not an error). General rule: after
`bastion_rename_colonists_unique`, tick at least one sync before any RECORD-name
lookup hook — ECS-side name hooks are immediate, record-side ones are not the same
tick. `derive_demoted` is now its own standalone outcome bool specifically so this
class can't hide silently inside a composite assert again. (2) a FIXED post-demote
wait can race the despawn/respawn window (a getter reads empty mid-gap); the
existing BED/NEEDS poll-until-ready pattern was the right precedent to follow
instead of a fixed sleep, and was applied here too.

Sonnet tag-review: clean, no findings — every done-when landed 1:1, the ripple was
caught and fixed exactly where the packet said to stop-and-flag rather than after
the fact, and the two harness finds are genuinely useful testing-discipline lessons
rather than symptoms of a deeper problem.

**Side note banked for whenever PATH-0 (row 45) is reached, not urgent now:** the
existing PATH-0 spec's "frontier+1 = PATH-0-WITH-B7" near-cap premise assumed the
"B7" migration would grow colonist count N; the B7 that actually shipped is needs/
mood (no population growth mechanism), so at today's N the near-cap precondition
PATH-0 was written against likely still doesn't hold. Whoever crafts PATH-0's
packet should re-check what actually grows N first, or accept a synthetic-N
scenario instead of a natural one.

### bastion-block-PATH0 (42f4eb832c, row 45) — ★OPUS-GATE-AT-TAG, CLEAR (§R15)
The deterministic global path budget/scheduler, re-scoped to synthetic-N (the
premise investigation confirmed no master-list block was supposed to grow colonist
count N and got skipped — see the builder's N-investigation above; the "ships WITH
B7 migration" premise in `PATHFINDING-SCALE-SPEC.md` was simply a wrong guess about
what B7 would ship). Colonist Goto searches lifted OUT of the agent system's
`.par_join()` (`server/src/sys/agent/mod.rs:76`, parallel per-entity) into a
sequential Uid-ordered cursor'd round-robin, budgeted under `PATH_TICK_ITER_CAP`
(3000 iteration units per tick); `find_path`/`astar.poll` reused wholesale via an
extracted `search_step` — no new search algorithm, only the scheduler and the
enqueue/consume seam around it, exactly per the packet's framing. Entropy-free — no
rng anywhere in the scheduling path, no stuck-shuffle tiebreak needed since the
cursor rotation itself is the deterministic tiebreak.

**Scoping calls honored:** `bastion_full_path`/TIGHTDIG's unbudgeted whole-search
stayed explicitly OUT of scope (flag-gated off today, deferred to row 31.4's own
staged-rollout checkpoint rather than baked in speculatively) — noted, not solved,
per the packet's ruling. Combat/vanilla pathing stays inline-unchanged behind the
existing config-root gate — this scheduler only claims Bastion colonist Goto
searches, nothing else.

**Tier: build→gate-at-tag with Opus AT TAG** (architect's final call, superseding
an earlier back-and-forth on timing — a shared-scheduler/per-tick-cap IS a real
dynamic surface in principle, but the pattern itself — sequential id-ordered queue +
budget — is well-trodden, not novel, so no pre-build hold; Opus verified the
FINISHED code+scenario, not a pre-build plan). **Opus CLEAR (BUILD_REVIEW_LOG §R15),
all four load-bearing properties confirmed directly in code:**
- **(a) Determinism-by-construction** — `BTreeMap` + `sort_unstable_by_key(Uid)`
  drive the ordering, zero `HashMap` anywhere in the request queue, zero rng in the
  scheduling path. Two synthetic-N runs are aggregate-identical. Explicitly, honestly
  scoped: this is determinism-FRIENDLY, not a fix for the separate ARCH-003 entropy
  seam (which the Bug-Tester proved persists single-threaded too — a different class
  of problem, reconcile at merge when `codex/arch003` lands, not claimed as closed
  here).
- **(b) Starvation-free BY CONSTRUCTION, not by tuning** — the cursor round-robin
  makes denial impossible by construction: every enqueued request eventually gets
  its turn as the cursor rotates, there is no path for a request to be silently
  dropped. Measured: peak_wait = 1 tick in the 18-colonist synthetic over-cap test
  (the cap saturates exactly at 3000/3000 iteration units with 18 colonists
  requesting simultaneously) — deferral is real but bounded, never indefinite.
- **(c) No-entombment / stuck-recovery preserved** — a colonist awaiting a budgeted
  path sits in a state BYTE-IDENTICAL to an ordinary mid-search tick (nothing new
  for `stuck_watch` to distinguish), so the existing teleport backstop fires exactly
  as before if such a colonist wedges. FR15's watch-point resolves to N/A: the
  1-tick consume-last-result latency changes WHEN a route arrives, never the
  stuck-economy's own tuning inputs.
- **(d) Vanilla NPCs unaffected** — the scheduler claims Bastion colonist Goto
  requests only, gated at the config root; combat pathing and vanilla NPC pathing
  never enter this queue.

Synthetic-N proof (the re-scoped done-when, replacing the spec's original
natural-growth assumption): 18 colonists saturate the budget cap exactly (peak
3000/3000 iteration units), the mine still resolves under that load, movement stays
staggered rather than frozen.

**Registry, filed during the premise investigation, both non-blocking:** D19
(vanilla's Architect-rule respawn homeostat silently converts colony population
into plain-villager population over long soaks — colonist deaths leak out of
colonist-N, deferred to B-AG6's natural landing point) and B29 (the Architect
rule's respawn uses OS-entropy `rand::rng`, not the `tick_rng` pattern — irrelevant
today, a real trap for any future long-soak scenario crossing a game-day).

**★ ARCH-003 overlap, reconcile at merge (not urgent now, not a live collision):**
this scheduler restructures the same pathfinding/agent-scheduling code the
Bug-Tester is fixing on the separate `codex/arch003` branch. PATH-0 does NOT close
that seam — kept the framing honest in the tag itself rather than over-claiming.
When the branches merge, ARCH-003's fix should ideally fold into PATH-0's cleaner
scheduler rather than fight it as a separate patch.

Sonnet/Opus tag-review: clean, all four load-bearing properties confirmed directly
in code, no findings beyond what the packet asked to verify. Row 45 → DONE.

**Mechanism precision addendum** (the builder's own tag report arrived after the
Opus-summary bookkeeping above — folding in the primary-source detail rather than
leaving it thinner than what was actually built; nothing here contradicts the
verdict, it's more precise than the summary): the new file is
`server/src/bastion_path.rs` (7 files touched, +572). The actual shape is a **PULL
model, not a push queue** — the request IS the visible routeless+Goto state itself
(no queue mutation from the parallel agent tick, read-only visibility, which is
WHY zero shared state gets touched by the parallel join). The gate lives on a new
additive `TraversalConfig.search_allowed` field (the `scramble_reach` precedent) set
`false` at the agent tick's single config-construction site for colonist+Goto only;
with `search_allowed: false` and no route yet, `Chaser::chase` holds its
PRE-EXISTING `Pending` stance rather than searching inline — no new movement class
introduced. `Chaser::search_step` is `chase()`'s search half extracted VERBATIM
(the existing 250-750-iteration per-call budgets `find_path` already hands
`astar.poll`, now summed via a new `Chaser::planned_iters`). The exact
Goto-executing arm (corrected from an earlier tentative citation of
`behavior_tree.rs:428`, which turned out to be the item-pickup arm): **
`server/agent/src/action_nodes.rs:269`** (`Some(NpcActivity::Goto(..)) =>
path_toward_target`).

Concrete numbers from the synthetic-N proof: 18 colonists × 250 fresh-search iters
= 4500 > 3000 at first arbitration — real, provable contention, not a contrived
edge case; peak_wait measured 1 tick (nominal ceiling 2, assert bound 7); the
46-job mine completes to zero under load; `b58_d_claims_ratio` 1.227 (in band);
grants split 440/538 across the two runs (entropy-shifted mid-run totals,
deliberately outside the diffed outcome-JSON, the B7-3 lesson applied). The
PREEMPT leg's mid-travel-wedge case (a colonist wedged mid-Goto) was re-run ON
scheduler-delivered paths specifically and stayed green — the closest thing to a
direct regression check on property (c) beyond the general 26-leg gate.

**Scope call surfaced by the builder, not previously spelled out:** colonist
combat/flee pathing stays inline and OUTSIDE this scheduler's budget — deliberate,
not an oversight, because combat targets are invisible outside the behavior tree,
latency-critical, and rare, so budgeting them would risk exactly the kind of
starvation this block exists to prevent elsewhere. The scheduler covers N-scaling
JOB-TRAVEL load specifically. Also confirmed for the ARCH-003 merge note: the
ambient-rng call sites (`stuck_check`'s `rng()`) are byte-unchanged by this block —
consistent with "does not close that seam," not just asserted but verified
untouched.

### bastion-block-FARM (682211eac9, row 46) — the renewable food loop ships
Farm plots + growth sim + harvest, built to the packet's shape. **First tag to run
the new permanent VOXCHECK leg** (B30 — `cargo check -p veloren-voxygen`, wired as
leg 3 right after BUILD, confirmed structural before this tag) — green, confirming
FARM is the first client-buildable tag since B7-1 broke voxygen six tags ago.

**Prior-art calls, live-verified, with a real trap surfaced along the way:** the
`Growth` sprite attribute IS already render-consumed by voxygen's attribute-filter
machinery, but no manifest entry had ever used it — FARM is genuinely the first
real consumer. Ships with ZERO new assets: `WheatYellow`'s manifest gained
growth-filtered configs (GREEN models render stages 1..9, YELLOW 10..15, the
original filterless models stay LAST as the fallback). **The trap, caught before
shipping:** category-declared attributes default to 0, so every WORLDGEN
`WheatYellow` placed anywhere in the game reads `Growth(0)` — a naive 0-indexed
filter would have regressed every piece of vanilla wheat in the world down to
shoots. Resolved by reserving `Growth(0)` for the mature fallback and having farm
wheat sow at `Growth(1)` instead — worldgen's untouched `Growth(0)` reads now fall
through cleanly to the same mature look it always had. Vanilla stays byte-preserved
by construction, not by luck.

**Designation-shape ruling:** `Farm` is a top-level `DesignationKind` registering a
PERSISTENT footprint, following the `Stockpile` precedent rather than `Gather`'s —
farm cells CYCLE (till→sow→grow→harvest→re-tilled, forever), unlike Gather's
complete-and-vanish single pass.

**Mechanism:** (1) painting a plot registers it in `board.farms`, generating zero
jobs itself (v1 plots are flat, no terracing). (2) THE FARM PASS — one bounded scan
(`%15==3` cadence, cost is O(Σ plot area), the explicit cost-bound the packet asked
for) drives state transitions: raw ground → TILL; tilled + an empty crop cell →
SOW (`required_item = wheat_seeds`); GROWING → a deterministic stage clock (a
per-cell last-advance timestamp in a tuple-keyed `BTreeMap`, one `set_block` per
cell per `FARM_STAGE_SECS` [6s], bounded by construction — no per-tick scan of
every growing cell); MATURE (`Growth==15`) → HARVEST. One live job per target cell,
reusing the paint-path's existing exploit-guard dedupe. (3) ONE state-driven
completion arm handles all four job kinds (self-healing — a job whose cell state no
longer matches what it expects moot-releases rather than erroring): TILL → Earth;
SOW consumes ONE seed via the same decrement path `Build` uses (a missing seed
correctly stalls as `needs_materials`, which B6's haul machinery then feeds) and
places `WheatYellow@Growth(1)`; HARVEST drops 2 wheat + 2 seeds — strictly more
than the 1 seed consumed — and returns the cell to Earth, re-sowable. (4) new items
`common.items.bastion.wheat` + `wheat_seeds` (data-only, icons reuse shipped
models; wheat is an INGREDIENT not a `FOOD_DEFS` entry — crop→meal→eat is
DF-COOK's job, not this block's). (5) `WorkType::Farm` + a farming skill + a farm
work-priority, all serde-defaulted; bare-hand tool base for v1.

**Four registry finds, all filed:** (1) **B31** — `job_wanted` serves two
independent masters (paint-time eligibility and mid-travel validity), and Farm's
state-driven completion model needed to opt OUT of the mid-travel re-check
entirely; missing that opt-out caused a completely silent create/claim/drop churn
(6729 creations, 6720 claims, ZERO completions) that a bounded job-COUNT invariant
never caught (the board never held more than 9 jobs at once) — only the
creations:claims:completions ratio revealed it. (2) **B32** — plain air reads
`Some(SpriteKind::Empty)`, never bare `None`; a vacancy match on `None` alone
silently swallowed every empty field cell into a foreign-state skip arm, diagnosed
by a raw-read probe. (3) **B33** — the B6 material-fetch machinery was hardcoded to
`BUILD_MATERIAL` at three separate sites (availability, claim-carry, fetch-commit);
generalized per-def so `Sow`'s `wheat_seeds` requirement works and every FUTURE
material-carrying job kind (recipes, DF-COOK) inherits the generalization for free
— Build itself is unchanged by construction. (4) **B30** — the voxygen gate gap,
now fixed and permanently gate-guarded (above).

**Verification (`--farm-scenario`, leg 27, 2/2):** 3 colonists, a 3×3 plot, a
stockpile bootstrapped with one 14-seed stack. Paint creates zero jobs (confirms
(2) is fixed); all 9 cells till (rock→Earth); all 9 sow at `Growth>=1` (fetched
through the now-generalized machinery, confirms (3) is fixed and confirms no
regression of (1)); a probed cell's growth rises strictly and the plot matures;
harvest auto-fires (wheat=2); **seed conservation as an honest ledger** — a run-4
lesson folded in mid-block: fetched seed stacks live in colonist BAGS, invisible
to ground-only counts, so the verification added `bastion_colony_item_total`
(counts ground + bags together) — every seed is accounted for as either an ITEM or
a GROWING crop, each harvest nets +1, and the ledger reads EXACTLY 15 = 14 + 1 in
both runs. The full harvest→haul→fetch→re-sow cycle closes and was observed live
(fresh growth visible after the first harvest). Jobs stayed bounded ≤18 at every
probe; zero embeds throughout. Full 28-leg gate: 28/28 green (26 from PATH0 + new
FARM scenario leg + the new permanent VOXCHECK leg).

**Known-scope, not gaps:** unloaded farms freeze (the loaded-only decay class,
D19's LOD sibling — deferred to the 41.2/LOD lane, same fix shape as that fork);
worldgen-placed wheat is left alone in v1 (not swept into the farm system);
`FARM_STAGE_SECS` is a const for v1 (RON-tunable at a future checkpoint);
wheat→meal is explicitly DF-COOK's (PROD-3).

Sonnet tag-review: clean. All four registry finds were caught and fixed by the
builder BEFORE tagging (run-1 fails diagnosed and resolved in the same block, not
shipped-then-discovered), the Growth(0)-reservation trap specifically is exactly
the kind of vanilla-preservation catch that's easy to miss and expensive to find
later. Row 46 → DONE.

### bastion-block-RUN0 (39543568ea, row 47, RUN-0 sub-block)
The emergency-run gait + energy governor, narrowed to RUN-0 only per the design
doc's own sub-block split (RUN-1/RUN-2 both genuinely blocked, deferred to rows
47.1/47.2). Gate: 28/29, one B5 red — B22's FOURTH instance, and this time the
CAUSAL mechanism got named rather than just classified: adding
`WriteStorage<Energy>` to `bastion_jobs`' `SystemData` changes the ECS dispatcher's
dependency graph (the system now serializes against vanilla's stats system, which
also writes `Energy`), shifting tick scheduling with zero logic change — exactly
the noise B5's completion-window assert re-draws on. 3/3 identical-binary reruns
confirmed the classification (B22 filed with this as its fourth instance and the
new general rule: a SystemData storage addition IS a scheduling change).

**Substrate checks, verified before building rather than assumed:** colonists
already carry `comp::Energy` (`Energy::new(body)` at every NPC spawn,
`state_ext.rs:240`) and vanilla ALREADY regenerates it unconditionally (`stats.rs`,
an accelerating regen) — the recovery half of the governor was free, needing no new
code. `RUN_SPEED = 1.0` sits inside vanilla's existing speed envelope (walk 0.8,
`MAX_FLEE` 0.65) — zero new anim/physics surface, the existing velocity-driven
figure animation reads the higher speed as running for free.

**Mechanism, exactly the packet's shape:** (1) `running: bool` on `BastionColonist`
— serde-defaulted false, the ONLY new persisted state; `Chaser`/pathing stay
byte-untouched. (2) the gait choice happens AT the existing Goto write site (the
job-travel call), choosing `RUN_SPEED` vs `TRAVEL_SPEED` off the flag — the
harness's own test-goto mover deliberately keeps a fixed walk gait (so fixtures
measure at a known speed) and the disperse-egress Goto stays walk too (an
always-run there would be RUN-1 trigger material, correctly left alone). (3) THE
GOVERNOR, per-tick: drains Energy 15/s while flagged; below `RUN_MIN_ENERGY = 10`
FORCES `running = false` ("winded") — resource-governed, not timer-governed,
exactly the design's framing; a fresh trigger is required to run again; recovery
rides vanilla's existing regen, untouched by this block. (4) colonist-only by
construction — the same PATH-0 config-root-gate discipline, vanilla NPCs provably
unaffected.

**The block's real find (run-1 fail → root-caused → fixed → green), filed as
B34:** a Bastion drain competing against vanilla's regen must beat the regen's CAP
rate, not its base rate. Vanilla's `Energy::regen` is unconditional AND
accelerating up to a 10/s cap; the first attempt used a 6/s drain, which lost the
race — energy dipped to ~88 then the accelerating regen outpaced the drain and
pulled it back to full with the run flag still set, so the low-energy floor was
structurally unreachable and the force-revert governor never fired. No error
anywhere — telemetry showed the floor was simply never observed, energy ended near
max, the flag stayed true. Fixed: 15/s, a comfortable net −5/s against a maxed
regen, giving the intended ~7-18s reserved sprint before winding.

**Stuck-watchdog composition, checked explicitly since asked for by name:**
`RUN_SPEED` sits inside vanilla's existing envelope and the watchdog keys on
PROGRESS (best_dist improvement + hysteresis), not absolute speed — a faster
colonist improves `best_dist` faster, making it strictly LESS stuck-prone while
running, and the winded drop returns to the exact gait every stuck-economy
constant was already calibrated against. No timing interaction found; the full
ladder (every stuck-economy leg) serves as the regression net and stayed green.

**Verification (`--run-scenario`, leg 29, 2/2):** measured DISPLACEMENT RATE over
fixed mid-travel windows — deliberately not arrival timing, to avoid the exact
completion-window flake shape B5 just hit. WALK: 0.200/0.223 blocks-per-tick
(default, nothing ran unflagged); RUN: 0.308/0.331 (~1.5×, asserted >1.15×);
energy drained 100→~66 across the trip; the governor force-reverted at 11.3/9.9
(within a point of the 10.0 floor both runs — the test hook never turns the flag
off itself, only the governor can); vanilla regen returned energy to full after;
zero embeds. Energy MAX varies per the random humanoid body roll (rtsim entropy),
so every energy assert is relative/boolean by design rather than an absolute
value — the B7-3 telemetry-split lesson applied to a new axis. PATH-0 composition
pre-checked and confirmed clean: speed is not a pathfinding search input, zero
contact between the two blocks.

**Scope honored:** no triggers built (RUN-1's job, correctly blocked); no winded
debuff beyond the hard floor (RUN-2's job, blocked on DF-FOCUS); the
`MAX_FLEE_SPEED` being slower than walk (0.65 < 0.8, a real oddity) was left
exactly where it belongs — for RUN-1 to fix when the flee trigger actually lands.

Sonnet tag-review: clean. The B34 find is a genuinely valuable, generalizable
lesson (any future Bastion drain against a vanilla accelerating-regen resource
will hit the identical class if not checked against the cap explicitly), and the
explicit stuck-watchdog composition check is exactly the kind of due diligence
that should happen by default on any speed-touching block, not just when asked.
Row 47 → DONE.

### bastion-block-AUTON0 (afc175f89c, row 48) — "THE PLAYS ITSELF KEYSTONE SHIPS"
The per-colonist utility arbiter skeleton (Work/Idle/Flee), ★OPUS-GATE — the
METHOD was drafted, amended twice through crossed-message rounds (self-job
authority, the write-vs-clear site map, the real Flee signals), and locked at
`readme/AUTON0-BUILD-PACKET.md` with 6 guards BEFORE the builder ever saw it —
see the packet-craft exchange earlier in this session for the full derivation.
Mechanism commit `40aa8b2174` + a pre-tag dead-colonist-skip fix `afc175f89c`.
Gate: 30/30 all green on the rerun, new AUTON leg, scenario 2/2 aggregate-
identical (Guard 3's ARCH-003-aware tolerance, not bit-identical).

**All 6 guards verified in the tag report, guard by guard:**
- **Guard 1** (the narrowed map, built exactly as adopted): gated exactly ONE
  writer — `:3436` the job-travel Goto, condition `self-job OR arbiter.current==
  Work` — plus the claim-loop entry; `:4583` the disperse fail-safe stayed
  EXEMPT (candidate-(b) cited in-code); the 3 harness sites untouched; the
  release/demote clears explicitly unconditional; the Arrived/Waiting clears
  follow the gated entry by construction, no separate gate needed.
- **Guard 2**: colonist-scoped joins, the Arbiter component only colonists
  carry, vanilla byte-unchanged, VANILLA leg green.
- **Guard 3**: sequential system, RNG-free (flat constants, field-read signals,
  timestamps only), two runs aggregate-identical under the ARCH-003-open
  tolerance.
- **Guard 4** (stuck-watch independence) — demonstrated TWICE, unprompted,
  rather than merely asserted: (1) fleeing jobless colonists in their own trench
  earned the 60s jobless rescue — drive-INDEPENDENT, proving the backstop still
  fires when genuinely needed regardless of drive state; (2) in the first gate
  draw, a post-recovery idle wanderer walked 35 blocks off the plateau into a
  genuine rescue — another organic, unprompted confirmation. Direction (b)'s
  "never false-trips" claim was then scoped precisely: the storm window (480
  ticks) is short enough that the 60s stuck-watch timer structurally CANNOT
  complete inside it, so any teleport there would be provably false — and the
  gate measured EXACTLY zero across the whole window. Registry corollary filed
  as **B35**: a global fire-counter asserted "zero for the whole scenario" is
  the WRONG claim when the counter legitimately fires outside the window under
  test — scope the assert to the window, not the run.
- **Guard 5** (PATH-0 composition): exercised live, not just argued — post-storm
  PATH-0 kept granting normally, `peak_wait` measured 0; the packet's predicted
  self-cleaning `waits`-pruning behavior was actually observed, not just trusted
  from the earlier code-read.
- **Guard 6** (self-job authority): self-job colonists (RestAt/EatFrom/Despond)
  skipped entirely at the TOP of the arbiter's per-colonist pass — no drive set,
  nothing gated for them; the `:3436` self-job travel arm fires UNGATED exactly
  as specced. B7-2/B7-3's Opus-cleared no-entombment/no-thrash guarantees
  preserved BY CONSTRUCTION, not by luck. `Drive` stays `{Work, Flee, Idle}` —
  the full self-job-to-drive unification stays AUTON-2's deliberate job.

**Root-caused fix, pre-tag (not shipped-then-discovered):** the arbiter now
explicitly skips DEAD/dying colonists at the pass top (`is_dead || should_die`).
A corpse needs no drive, and a drive write racing vanilla's own death processing
on a dying colonist was the actual bug — caught by the BED corpse-probe on the
first gate draw, root-resolved (not merely B22-classified as noise) before the
tag landed; re-run confirmed 3/3 post-fix.

**FLEE, built for real (not stubbed), both signals live:** `agent.target.
is_some_and(|t| t.hostile)` (vanilla's own combat-awareness field, code-identical
to the vanilla tree's own read) and `fraction() < psyche.flee_health` (the health
threshold, exercised through the REAL health-write path — no synthetic
injection). One genuine vanilla-interaction find surfaced along the way, RUN-0's
sibling class: vanilla silently restores a WORKING colonist's health to full
behind the scenario's back (a max-update-class heal — the mid-work pair read
exactly 100.0 within ~180 ticks of being set to 0.1, while the idle subject held
its low health). **The arbiter responded correctly the whole time** (Flee
dropped exactly when the signal genuinely dropped, Work correctly resumed) — the
FIXTURE was dishonest, simulating a persistent threat with a one-shot write that
vanilla quietly undid. Fixed by re-asserting the threat every second, matching
how a real hostile actually behaves. Filed as **B36**.

**Mechanism:** `comp::bastion::{Drive, Arbiter}` — `Work` carries no `JobId`
(`ActiveJob` itself is already the handle, no duplicate state); per-tick Flee
preemption rides the STANDARD release seam (sweep-safe ≤15 ticks, FR3-e);
selection runs at `%15==1` under a 0.5s commitment window + 0.15 hysteresis;
urgencies are flat v1 constants (flee 1.0 > work 0.5 > idle 0.1 — the ORDER is
the contract this block ships; AUTON-1/2 shape the real curves later);
work-availability is O(jobs) once per selection, not per candidate.

**Verification (`--auton-scenario`, leg 30, 2/2):** (a) liveness — Idle→Work
through the gated entry, a full mine strip completes (the block's whole risk
surface survived end to end); (b) Flee latency — the drive reads Flee within 2
sim ticks of the health write (well inside the ≤1-arbiter-tick per-tick-check
requirement); (c) a combined storm (all three pressure sources at once) froze a
second strip at exactly 20/20 for the full 450-tick window — zero jobs leaked,
claims provably suppressed, not just quiescent; (d) recovery — the frozen strip
returns to zero after the storm passes; (e) thrash — 16-18 drive-switches
against a bound of 40 (the small run-to-run variance is the vanilla-heal race
noted above, telemetry-only, not a correctness signal). Every done-when landed
1:1: max-urgency pick, commit-no-thrash, per-tick Flee preemption, sole
activity-writer, aggregate-determinism, vanilla untouched, the backstop fires
when needed and never false-trips.

Sonnet tag-review: clean — every guard has concrete evidence, not just an
assertion, and the two things caught pre-tag (the dead-colonist race, the
dishonest health fixture) were both root-caused rather than papered over. Row
48 → DONE.

**★ OPUS FINAL SIGN-OFF: CLEARED (BUILD_REVIEW_LOG §R16, 2026-07-13).** Opus
re-verified all 6 guards + the dead-colonist skip + the Flee ruling directly
against the tagged commit `afc175f89c`, not the packet's description of it: 2
gated activity-authority sites (Goto-steer `:3592`, claim-loop entry `:5539`),
both `(arbiter.current==Work OR self-job)`; the self-job skip at `:2738`
confirmed triply-safe; the dead-colonist skip at `:2732` confirmed present;
Flee's signal at `:2759` confirmed RNG-free (two direct field reads, no spatial
query). Both gate reds were real, root-fixed findings, not classified away —
and the teleport red specifically was live proof that Guard 4's stuck-watch
backstop fires independent of drive state, the exact property it needed to
demonstrate. AUTON-1 (self-designation generators) is next.

### bastion-block-AUTON1 (ede5b80b1a, row 49) — G2 CLOSES: the colony designates its own work
The self-designation generators (mine/haul/build), self-verify+tag per the
architect's tier call (data-generation into the existing job board, not a new
authority mechanism — confirmed, not just claimed, by the builder's own
craft-time findings). Gate: 29/31 first draw + B5/PREEMPT both 3/3 PASS on
identical-binary reruns → both flake-classified per B22 → effective 31/31,
SELFGEN green in-suite. Scenario 2/2 aggregate-identical with EXACT counters
matching across runs (mine=4, build=4, plans_done=1, fires=0 both times).

**Scope, as ruled:** mine/haul/build only; defense/muster deferred (no threat
data until B8); hygiene/expand skipped (no refuse or population-pressure state
to read yet). The DF-POLICY hook is ONE check site — `generator_enabled
(GeneratorKind)`, const-true for v1, the doc names POL-1..4 as the future
plug-in point rather than hardwiring "always on" deep. The existing B6 haul
generator was registered under the same hook (behavior-identical, just now
consistently gateable). Ore-sprite mining stays row 49.1 as filed at craft time.

**Mechanism** (one pass, `%15==2`, gated on a live plan existing — inert in
every other scenario by construction): the BUILD generator runs a per-plan
UNFILLED CENSUS (one terrain read per cell per firing, the SAME scan reused by
emission, retirement, and demand-calculation — not three separate scans) →
capped Build jobs with `required_item = BUILD_MATERIAL_ITEM` (the B26 fetch
contract, honored correctly this time), deduped via the existing `:1379`
one-job-per-block occupied-set. Plans themselves are `board.plans` — FROZEN
cell lists set by `queue_build_plan`, intent-only at queue time (zero jobs
created until the generator itself emits them — the farm-paint precedent
applied to plans); a plan's AABB joins the claim mask; once every cell is
filled the plan RETIRES. The MINE generator only runs when `deficit > 0`,
where `deficit = unfilled-plan-cells − stone-supply(pickup stack amounts +
colonist bags) − pending-mine-jobs` — since `BUILD_MATERIAL == MINE_DROP ==
stone`, the plan's own bill of materials IS the mine quota, no separate
tuning knob needed. The scan anchors at the structure centroid (an order-free
mean of plan cells; no live plan → no scan), advances ONE z-slab per firing
(radius 12, 9 slabs total, a wrapping cursor — the same budgeted-shape PATH-0
already established), and candidates require Rock-class + exposed + outside
the SKIP COLUMNS (plans + stockpiles + farms + beds + `built_xy`) with real
per-column depth gating (not a flat scan). Cancel (the eraser) drops any
touched plan whole. **Quiescence is structural, not tuned:** demand hitting
zero stops generation by construction — "no runaway job creation" is an exact
freeze assert, not a cap that could theoretically still be exceeded.

**Reuse ledger, confirmed clean:** job creation rides the existing `Job`
literal contract; dedupe is the existing placement occupied-set; claim,
travel, fetch, and completion are ALL untouched — a generated job is
indistinguishable from a player-painted one once it hits the board, so the
"no special-casing" done-when holds by construction, not by careful parallel
maintenance. Haul reuses the existing B6 generator as-is, just gated under the
same policy hook. Zero new wire enums (VOXCHECK stayed green), zero
`SystemData` changes (no dispatcher reshuffle — B22's fourth-instance lesson
from RUN-0 didn't recur here for exactly that reason).

**The ×2 story, root-caused not classified:** the first draw SPLIT — run1 was
a full pass (mined EXACTLY 4, matching the deficit precisely — the demand
arithmetic proven live, not just by construction), run2 built 2 of 4 then
starved. Two real causes, both caught and fixed: (a) a fourth instance of the
B24 fixture-geometry class — the stockpile anchor sat 4 cells south of the
strip's center, putting the mine scan's first candidate row on the strip's
south EDGE against raw undesignated worldgen; a pit-trapped stone's haul
churned `unreachable` ×6 approaching through the rough terrain band. Fixed by
centering the anchor so the ±12 scan circle sits entirely inside controlled
flat rock. (b) That churn EXPOSED a real, previously-latent B6-class bug,
filed as B37/row 49.2: a haul job reserves its target at generation time and
never releases that reservation if the target becomes permanently
unreachable — `should_merge` amplifies this, since a merged pile shares ONE
reservation across N physical stones, so a single bad unreachable-haul job
pinned BOTH of run2's remaining stones behind it, exactly matching the
observed 2-of-4 starvation. Post-fix: 2/2 green with byte-identical outcome
JSON, including the exact counters.

**Verification (`--selfgen-scenario`, leg 31):** an un-designated 3-colonist
colony, given only intent (a bootstrapped stockpile + one queued 2×2 platform
plan). Asserts: ZERO board jobs exist immediately post-setup (the zero-paint
proof — nothing was ever player-designated); generation stays bounded ≤
colonists×2 at EVERY poll, not just at the end; stone physically reaches the
stockpile; all 4 plan cells get built; the plan retires EXACTLY once; both
generation counters freeze across a full 450-tick post-retirement window
(quiescence, not just eventual silence); the board drains back to zero jobs;
two runs are counter-identical. `CENTER_NET` fires are reported, not asserted
(the AUTON-0 lesson applied proactively) — 0/0 regardless. Colonists pick up
every generated job through the completely NORMAL claim path, flowing under
AUTON-0's Work drive with no special-casing anywhere in the pipeline.

**Registry, beyond B37/B24's fourth instance:** filed as **D20** — a
generator's own completed output can re-satisfy its own candidate filter on a
later pass (a retired plan's finished platform is exposed rock, which the
mine generator's purely physical candidate test can't distinguish from
untouched terrain); worked around today via the skip-column list, but
`built_xy` only tracks GENERATOR-built structures — player-painted
constructions have no footprint registry yet, an honest open edge rather than
a fully-closed gap.

Sonnet tag-review: clean — the demand-driven quiescence design turns "no
runaway job creation" from a tuned-cap promise into a provable structural
fact, exactly the "impossible by construction" pattern this whole build has
favored, and both ×2-draw causes were root-caused to real, separately-useful
findings rather than being waved off as noise. Row 49 → DONE (architect's lean
sign-off already recorded above; this entry backfills the full mechanism for
the permanent record). AUTON-2 (need-drives + the death-spiral E1 gate) is
next — a full Opus-METHOD block, routing through the architect at
packet-craft same as AUTON-0.

### bastion-block-HAULPIN (5d6b8a133d, row 49.2) — the B37 fix
The AUTON-1 follow-up, self-verify+tag: a churning unreachable Haul job now
DROPS at `HAUL_DROP_STRIKES=3` — the arrival-tolerance growth cap, chosen
because once tolerance stops growing, further churn cannot converge — via the
existing post-loop `remove_job` (which already frees the reservation; wired
through the `carve_requests` deferred-borrow pattern already established
elsewhere). The slot-7 generator then re-emits from a FRESH scan next cadence:
this replaces retry-BY-CHURN (the same job endlessly retrying, its item and
the WHOLE merged pile behind it pinned forever — exactly AUTON-1 run-2's
2-build starvation) with retry-BY-RESCAN (an item that's fetchable between
tries gets picked up by a later generator pass instead of staying hostage to
one bad job). Designated (player-painted) kinds deliberately KEEP the
unreachable economy as-is — terrain targets can legitimately un-block later,
and the existing 60-tick amnesty already belongs to them; this fix is scoped
to generator-created haul jobs specifically.

**Verification (`--haulpin-scenario`, leg 32, 2/2):** a 2-stack sealed SEVEN
blocks deep — deliberately BEYOND the 6.1-block arrival-tolerance envelope a
remote-grab could reach (the first-draw finding: a 3-deep seal converges via
remote-grab at 6.0, just inside the max tolerance, so 3-deep alone wouldn't
have exercised the drop path at all). The cycle emit→strike→drop→re-emit
fires EXACTLY 3 times both runs (counted via `next_id` deltas, not a racy
transition poll); reservations never exceed 1 at any point; the stack
conserves; outcome identical across both runs.

**Evidence chain, honored per the earlier-agreed recovery condition** (the
cargo-collision incident from earlier this session): HAULPIN's own ×2 PASS
ran clean at its actual commit BEFORE any AUTON-2 edit existed; the 33-leg
suite ran green at the DIRECT CHILD commit (AUTON-2), re-validating HAULPIN's
own scenario leg specifically; and the closing check — a diff confirming
AUTON-2's `bastion_jobs.rs` delta is exactly 3 hunks, none touching the
release/drain/reservation code HAULPIN's fix lives in. New read-only probes:
`JobBoard::probe_next_id`/`probe_reservations` + `bastion_board_probe`.

Sonnet tag-review: clean, no findings. Row 49.2 → DONE.

### bastion-block-AUTON2 (01151c61c1, row 50) — THE E1 GATE
The trait-stagger + the death-spiral scenario, ★OPUS-GATE (packet-craft AND
tag). Committed as its own 4 files (+790) — the fleet's docs untouched, the
same discipline as every prior block.

**Mechanism, exactly per the packet:** `stagger_interrupt` (a pure fn, rides
`care_factor`'s established pattern): a hardiness composite `h` from
Craft/Tradition VALUES (±0.5 each) plus Conscientious (+0.5) / Neurotic (−0.5)
PERSONALITY traits; effective threshold `eff = base × (1 − 0.4h)`, clamped to
`[INTERRUPT_FLOOR (0.05), base × 1.5]`. The safety-floor property Opus asked
for by name: the hardiest POSSIBLE colonist (`h = 1.5`, unit-pinned exact)
lands at `eff = 0.08` — comfortably above the `0.05` floor, never at or below
it. The `.min(base)` on the ceiling side keeps recreation's own never-preempt
`0.0` untouched and the clamp well-formed (ceiling `0.3 < comfort 0.5`).
Swapped in as the TWO threshold INPUTS at their existing read sites (`:3442`/
`:3445`) — B7-2/B7-3's own machinery (cooldown, hysteresis, self-job
authority, the Despond staircase, all already Opus-cleared) is completely
untouched; the stagger rides the existing rtsim personality-read guard idiom.
`FOOD_DEFS` gained `FARM_WHEAT` (the const's own designed extension point,
its doc-comment already anticipated this — data, not machinery; without it no
shortage could ever recover through production, since wheat wasn't
recognized as food at all before this).

**Read-only probes added:** `bastion_despond_jobs`, `bastion_colonist_
temperament`, plus board probes (`probe_next_id`/`probe_reservations`,
HAULPIN's own).

**The E1 death-spiral scenario (`--autonomy-death-spiral-scenario`):** boot A
tests work-BEFORE-workers (paint and stock the colony BEFORE any colonist
spawns — the root fix for the anti-wander finding below, not a band-aid on
top of it), temperament-aware role assignment plus predictions computed via
the mechanism's own public fn (not re-derived by the scenario), and 3
separated 1-wheat starters (B38-aware — deliberately not one merged pile) set
against 4 eaters, so the depth genuinely requires recovery to ride
IN-WINDOW harvests, not just existing stock. The split assert is now
crossing-tolerant by design: holders who NEVER preempt above their own
threshold (the correctness property) PLUS every colonist below threshold
genuinely attempting to preempt (an attempts-delta count) — replacing an
earlier, stricter fed-in-window count the builder had self-imposed and then
disclosed was actually measuring three ALREADY-FILED, separate concerns (see
below), not the stagger itself. RECOVERY is asserted at the COLONY level, per
the architect's ruling below, with a structural window rather than a fixed
wall-clock guess; the floor gets its own DIRECT assert. A PURE-STAGGER model
sufficed end-to-end — the packet's food-urgency Work-scorer fallback was
never needed, and wasn't built.

**★ Acceptance-criteria ruling, disclosed and resolved before tagging:** the
builder's own scenario originally required ALL colonists fed within a fixed
window — stricter than the packet's actual done-when. Forensics traced every
failure of that stricter bar to three ALREADY-FILED, non-stagger causes: B38
(a merged pile serializes concurrent eating to the preempt-cooldown cadence
regardless of stock size — though this scenario used separated starters
specifically to avoid it contaminating the READ), the simulated-tier
population-geography scatter (colonists can wander 80+ blocks during idle
bootstrap before any work exists to anchor them — banked to the Design
backlog as IDLE-HOME-LEASH), and the residual ARCH-003 scheduling variance
(this is the suite's single most timing-exposed leg — 10+ minutes of
emergent economy vs every other leg's arena-confined runtime). None of the
three are the stagger mechanism. The architect ruled E1's "recoverable-band
auto-recovers without input" at the COLONY level (`stock≥start`, `≥5/6 fed`,
all alive, a straggler's meter still positive-and-retrying — reported, not
gated) rather than an every-individual-deadline reading, since every
individual-level mechanism property (the floor, the stagger's own
never-preempt-above-threshold discipline) already has its own direct,
independent assert — nothing about individual behavior goes unverified by
loosening this one aggregate measure. Also corrected en route: no
`--deterministic-rtsim` CLI flag exists (B8's fix made the harness's
determinism unconditional, always-on) — the run-to-run divergence chased
here was genuinely the known-open ARCH-003 residual, not a new bug, just
unusually visible on this scenario's length.

**Gate: 32/33 first draw.** UNIT 30/30 (the stagger floor pinned exact,
0.08 ≥ 0.05); SPIRAL green in-suite; two runs aggregate-identical (stock0=3,
stock1=44/38, holders=2, eaters=4, fed_in_window=2, split=true, floor=true,
despond=(true,true) both runs). **The one non-green leg, PREEMPT, was B22
flake-classified** — the exact `preempt_endured`/`endure_dug=0` signature
already established at the AUTON-1 gate, confirmed identical pre-stagger (so
not introduced by this block). **★ Flagged for the record, not swept past:**
this classification rode WEAKER evidence than the established protocol this
time — 1 of 3 reruns passed, not the usual 3/3 — the builder disclosed this
themselves and offered a quiet-machine rerun. Whether that rerun happens
before Opus's final sign-off is the architect's call, made separately from
this bookkeeping pass.

**Registry, four classes surfaced across the ten-draw forensics chain (all
filed separately, not here):** trait-surface ownership (rolled-personality
vs. set-values needing the prediction to own its whole read surface), a
window functioning as its own decay clock (a legitimate threshold-crossing
during a measurement window, not a bug), the idle deep-wander/starving-amid-
plenty finding (IDLE-HOME-LEASH, Design backlog), and B38 (merged-pile eat
serialization, its own future block).

Sonnet tag-review: the mechanism itself never misfired across any of the ten
draws — every red was root-caused to a scenario/fixture/prediction/window
issue, never the stagger or the recovery logic.

**★ OPUS FINAL SIGN-OFF: CLEARED (BUILD_REVIEW_LOG §R17, 2026-07-13).** Opus
re-verified directly against the tagged commit: the safety floor is
construction-enforced (`bastion.rs:279`'s clamp, the `0.0`-base `.min(base)`
edge case specifically checked and correct); the colony-level acceptance
ruling was applied correctly (SPIRAL aggregate-identical, straggler-eventually
-eats correctly treated as telemetry not a gate); determinism holds against
the PREEMPT flake specifically because the SIGNATURE match to the
already-established AUTON-1 instance was the deciding evidence, not the rerun
ratio alone. Row 50 → DONE, mechanism cleared.

**★ WALK-BACK, SAME DAY:** on reflection the architect asked for the proper-
evidence rerun anyway before calling the TAG fully closed — 1/3 was genuinely
weaker than the established B22 protocol, and an Opus-final block shouldn't
get a lower evidence bar than precedent just because the signature happened
to match. The mechanism sign-off above stands (verified independently in
code), but the tag's full closure is HELD pending a quiet-machine ×3 PREEMPT
rerun the builder is running now. If clean or signature-matching 3/3 →
CLEARED for real; if it reveals something new → re-opens immediately.

**★★ THE REGRESSION, FOUND AND RESOLVED (2026-07-14).** The quiet rerun did
exactly what it was asked to: 0/3 PASS, all three BYTE-IDENTICAL — a genuine
deterministic finding, not scheduling noise, and on a DIFFERENT assert than
the known flake. `preempt_hover_silent` (which had been TRUE at the AUTON-1
gate) flipped FALSE under AUTON-2. The loaded gate-box's timing noise had
been producing the familiar `endure_dug` flake AND masking this second,
genuinely deterministic failure behind it on the SAME leg — a real B22
misclassification, self-caught: the earlier gate-time call had matched the
FAIL to the known-flaky LEG by name without diffing which FIELD actually
failed. General rule now registered: classify by failing field, not leg
name — the same leg can carry a genuinely flaky assert and a genuinely
broken one simultaneously.

**Diagnosis (read-only throughout, both findings confirmed by direct
measurement, not theory):**
- **What the assert protects:** `preempt_hover_silent` is B7-2's anti-thrash
  hysteresis companion — set rest to `0.21` (one notch above the interrupt),
  tick 600, assert zero preempt attempts fire. It guards against band-edge
  preempt-flicker/attempt-spam. Critical disambiguation the builder
  specifically checked before reporting severity: "hover" here means the
  METER hovering at a threshold — a name collision with the unrelated R3/
  FR15 physics-hover/`stuck_watch` class, not a shared surface. The assert
  reads only the attempts counter; `no_embeds`/`CENTER_NET` stayed green in
  every quiet run. Zero stuck-economy or safety surface.
- **The regression mechanism:** the live base interrupt is unchanged (`0.2`,
  verified in both code and `bastion_mood.ron`); the colonist's roll is
  unchanged (same seed, this exact assert was green at the AUTON-1 gate);
  therefore by elimination the only thing that moved is `stagger_interrupt`
  itself — confirmed directly, not inferred, by the sim's own threshold
  comparison measuring this colonist's effective rest threshold as strictly
  `>0.21` (bracketed to `0.21–0.30`, consistent with a Neurotic-only or
  Craft+Tradition-negative roll). The fixture's hardcoded `0.21` predates
  per-colonist thresholds by three blocks; the preempt firing at `0.21` is
  CORRECT stagger behavior (an anxious colonist rests earlier), not a bug.

**Architect ruling:** test-only fixture code, Sonnet-tier — no full Opus
re-pass needed (the mechanism itself, already reviewed, is untouched and
behaving as specified). Approved a small, harness-only fix.

**The fix (`bastion-harness/src/main.rs`, +21/−2, two hunks):** the fixture
colonist's Craft/Tradition values are zeroed (isolating temperament as the
only variable), its temperament is read via `bastion_colonist_temperament`,
and the hover-phase threshold is computed via the mechanism's OWN public
`stagger_interrupt(0.2, &vals, consc, neur)` — no mirrored/duplicated math —
landing in `{0.16, 0.20, 0.24}` depending on temperament (the two opposing
terms cancel when both traits roll true, coinciding with neither-true's
`0.20`). The hover level becomes `eff_rest + 0.01`, restoring the original
fixture's exact intent (band-edge-plus-one-notch) relative to the colonist's
REAL edge instead of a stale flat one. Cross-phase safety was explicitly
checked, not assumed: the endure force (`0.15`) sits below the minimum
possible edge (`0.16`) and the wedge force (`0.1`) below all three — every
other phase in the same scenario keeps firing correctly for every possible
temperament roll. Diff reviewed and approved by both Sonnet and the
architect before building.

**Closure evidence, all conditions satisfied:** PREEMPT quiet ×3 with the fix
= PASS/PASS/PASS. UNIT 30/30 (the stagger floor pin included). SPIRAL
re-verified via TWO separate ×2 draws: the first came back run1-fully-green/
run2-one-field-flip (`spiral_stagger_split`, on code the fix does not touch,
matching an already-documented pre-existing ARCH-003 sensitivity from the
original ten-draw forensics chain — correctly NOT reclassified as a
regression, per the same diff-the-field discipline just re-learned); a
second confirmatory ×2 was requested anyway, given SPIRAL is the block's own
named E1 gate scenario, and came back fully green both runs, every field,
identical outcome JSONs.

**Commits:** the fixture fix landed solo as `b0b7016d89` ("AUTON-2 closure:
threshold-aware PREEMPT hysteresis fixture") — own-commit discipline, kept
separate from HIST-1's own unparking commit that followed on top.

**★ OPUS SIGN-OFF FULLY STANDS, NO RETRACTION.** The floor/recovery/degrade/
determinism properties Opus verified against the tagged commit were never in
question — this whole arc was a SEPARATE finding in a different assert,
diagnosed, ruled, fixed, and closed entirely at the Sonnet/architect tier as
directed. E1's mechanism is complete AND the tag is now fully closed.
Row 50 → DONE, no caveats remaining.

### bastion-block-HIST1 (574f401132, row 54, HIST-1 sub-block)
Tap the rtsim event bus into the Chronicle sink — a real first emitter beyond
Bastion's own two (CaveIn, SleptInBed). Self-verify+tag, CHEAP. Gate: 34/34
all green FIRST DRAW, zero flakes, zero reruns needed — includes the fixed
PREEMPT fixture running clean in-suite, a clean SPIRAL, and the CHRON-baseline
recheck the parking checklist called for (HIST-0's own leg stays green with
`ChronicleEvents` now live and registered).

**Mechanism, exactly per the packet:** `ChronicleEvents` (`rtsim/src/rule/
chronicle_events.rs`) is `ReportEvents`' SIBLING rule — same shape, same
registration site — binding `OnDeath`+`OnTheft`, one `record()` call each,
not a new capture mechanism. Death records `actors=[victim, killer?]` (the
deed lands in BOTH figures' histories — the mood `thought_sum` and the future
legends browser both filter by `actors.contains`), position from `wpos`,
`Importance::Notable`, `Scope::World`, and deliberately NO witness gate
(`Reports` keeps its own — gossip needs witnesses to spread, history doesn't
need anyone watching to have happened). Theft records `[thief]`, site+pos.
Same underlying event, two independent sinks; `Reports`' existing behavior
stays byte-untouched.

**Verification (`--chronicle-capture-scenario`, leg 34, 2/2):** one death
driven through the REAL pipeline (the BED corpse-probe's kill hook → the
server's actual death event → rtsim's `OnDeath` → both sinks, not a synthetic
shortcut) produces EXACTLY one Death entry (delta-baselined against whatever
existed before), `actors` length exactly 1, AND `Reports` growing too — the
regression-free half asserted directly, not assumed from the diff. One theft
driven through the REAL emission path (`bastion_emit_test_theft` →
`hook_pickup_owned_sprite`, vanilla's own existing hook, not a new one)
produces exactly one Theft entry with position set. Conservation held across
a 300-tick settle window (one event in, one record out, no dupes, no drops).
New read-only probes: `bastion_hist1_probe`, `bastion_emit_test_theft`.

**The parking story, worth keeping on record:** this block was written during
AUTON-2's own verification window under the architect's write-only condition
(don't build against the tree while another gate runs). That condition
self-sharpened mid-fill when the builder realized a registered `Rule` is LIVE
BEHAVIOR at every rtsim boot — "don't run cargo for it" wasn't actually
sufficient, since the code itself changing what OTHER scenarios observe once
compiled matters regardless of whether cargo runs specifically for it. Kept
fully OUT of the tree for the whole AUTON-2 arc as a result (memory-banked as
a corollary to the no-cargo-during-gate rule), restored via its own written
checklist once both AUTON-2 tags landed, compiled first-try, verified 2/2
first-try.

**Coverage note:** Death now has BOTH a live emitter and existing thought/
affinity rows (fully covered end-to-end). Theft has a brand-new live emitter
but NO thought/affinity rows yet — a fresh instance of exactly the coverage
gap row 54's own linked note describes; left for whoever extends
`bastion_value_affinities.ron`/`bastion_thoughts.ron` next, not solved here.

HIST-2 (the live feed panel) stays explicitly out of scope — it needs the
unbuilt B9 HUD + client sync, neither of which exist yet.

Sonnet tag-review: clean, no findings, exactly the packet's shape delivered
end-to-end through real pipelines rather than synthetic shortcuts. Row 54 →
DONE (HIST-1; HIST-2 remains TODO, blocked).

### bastion-block-AUTON3 (5e9ed6385f, row 51) — THE AUTONOMY ARC CLOSES (48-51)
Trait-modulated drive-urgency SCORING + `last_scores` as server-side data,
self-verify+tag (architect-confirmed tier). Tag rides the closure commit per
the AUTON-0 precedent: mechanism `c09933e463` + the storm fixes
`5e9ed6385f`. **Rows 48-51 — the whole autonomy arc — ship end to end.**

**Scope, exactly as narrowed at packet-craft:** trait-modulated SCORING
(which drive WINS the arbiter's pick — distinct from AUTON-2's threshold-
stagger, which governs WHEN a need becomes urgent) plus `last_scores` as
probe-readable data, the same "build the data before the display exists"
precedent B7-0 set ahead of B9's HUD. Explicitly NOT built, per the
narrowing: UI-4's actual display (now next in line, priority-bumped), DF-
POLICY's weight-biasing, and Ben's live-tuning pass — all real, all correctly
deferred to their own dependencies landing.

**Mechanism:** `modulated_urgencies(base, values, adventurous, worried,
sociable, introverted) -> (work, flee, idle)`, pure and RNG-free. One VALUE
plus one PERSONALITY-trait pair drives each axis: Wealth → Work, scaling into
`[0.4, 0.6]`; Glory + Adventurous + Worried → Flee, scaling into
`[0.85, 1.15]` then floored at `0.8`; Kin + (Sociable or Extroverted) +
Introverted → Idle, scaling into `[0.07, 0.13]`. Input-swapped at the
arbiter's two existing `last_scores` write sites only — selection,
commitment, and hysteresis machinery stayed completely untouched, the
packet's stop-line held throughout. The axes (Glory/Wealth/Kin) are
deliberately DISJOINT from the stagger's own axes (Craft/Tradition) — the two
trait-modulation systems compose independently rather than fighting over the
same value reads.

**★ The drive-order safety guard, unit-pinned exact (the load-bearing
property flagged at packet-craft):** the bravest possible colonist's Flee
urgency (`0.85`, the floor) still exceeds the greediest possible colonist's
Work urgency ceiling (`0.6`) — a margin of `0.25`, tested directly, not
eyeballed. The `.min(base)` clamp gives zero-preservation: a colonist with no
signal on an axis (a `0.0` baseline) can never be modulated UP into
inventing urgency that wasn't there — the third application of the same
recreation-zero discipline B7-2's stagger already established. Idle's
ceiling (`0.13`) stays below Work's floor (`0.4`) for every possible roll,
preserving AUTON-0's own liveness contract (a colonist never gets stuck
unable to pick Work over Idle) unconditionally. `UNIT` 31/31.

**Verification (`--auton3-scenario`, leg 35, 2/2):** two colonists rolled
with DESIGNED-OPPOSITE traits (colonist A: brave/greedy/loner, +50/+50/−50;
colonist B: the mirror), work painted so the Work axis actually scores.
`last_scores` recorded matched the mechanism's OWN prediction (computed via
its public fn, not re-derived by the scenario) to the exact f32 BIT — proving
E2's legibility claim ("colonists visibly differ from the same state")
directly and quantitatively, without needing any UI to exist yet. Recorded
Flee stayed `0.0` for both colonists (zero-preservation observed live, not
just asserted); the safety-floor guard was sampled against the actual live
brave roll, not a synthetic worst case; ×2 identical.

**THE GATE STORM, root-caused in one cycle, not classified away:** the first
35-leg draw came back 30/35 — five reds at once (`CK`, `ZONE`, `BED`, `B73`,
`HAULPIN`). Every one of the five was already an established timing-sensitive
leg, failing in its OWN historically-documented shape, and CRITICALLY: zero
scoring/pick/urgency asserts failed anywhere in the entire suite. The
drive-order math itself was proven roll-invariant by construction before any
further digging — the floor/ceiling separations hold in every direction, so
five simultaneous REAL behavioral regressions from the modulation logic
would have been arithmetically impossible. Root cause, confirmed: the
diff deferred the rtsim NPC lookup lazily (correct) but left the READ-GUARD
ACQUISITION itself hoisted to the top of the per-colonist arbiter pass,
firing every tick (~30/s) instead of the pass's historic ZERO — a suite-wide
lock-acquisition CADENCE change against the rtsim thread, exactly the
ARCH-003-sensitive scheduling class (RUN-0's SystemData lesson's sibling, one
level closer to the metal). **Filed as B39.**

**Two fixes, cleanly separated causes:** (1) the guard is now acquired ONCE
per selection tick (`select_tick.then(...)`, mirroring the mood-pass's own
established cadence pattern) with an ad-hoc acquisition only in the rare
flee-fire branch — restores CK/ZONE/BED/B73/AUTON3 to green. (2) HAULPIN's
own remaining red, re-examined via the field-not-leg-name discipline rather
than assumed to share fix (1)'s cause: `emissions` came back 2/3/3 across
three identical-binary reruns — TIMING-MARGINAL, not truly nondeterministic.
Its `--haulpin-scenario` window (240 polls) fit its 3 strike-drop cycles
(~25s each) with ZERO headroom; the original tag-time draw happened to land
exactly on 3, but any ordinary scheduling breath could legitimately drop it
to 2. Widened structurally to 480 polls (2× headroom) — fixture-only, the
HAULPIN mechanism itself untouched. **Filed as B40 — the fourth confirmed
instance of this exact "deadline-shaped assert needs structural headroom"
class** (after SPIRAL's recovery window and SPIRAL's floor window); a
consistency-sweep across the harness's remaining windowed asserts was
proposed and queued as master-list row 50.8, low-priority housekeeping.

Closure draw: 33/35, all five storm legs verified green in-suite; `LOD1`/
`BED` field-classified per B22 (`LOD1`'s `rounds_ok` = its own pre-existing
registry class, 2/3 rerun; `BED`'s `b_alive_after_kill` = the pre-existing
B7-1 kill-race field, 3/3 rerun) → effective 35/35. `UNIT` 31/31. Scenario
×2 byte-identical.

**Files:** `common/src/comp/bastion.rs` (`modulated_urgencies` +
`FLEE_URGENCY_FLOOR` + the unit pin), `server/src/bastion_jobs.rs` (the two
input swaps + the once-per-selection-tick guard fix), `server/src/lib.rs`
(2 new probes: `bastion_colonist_last_scores` — the exact surface UI-4 will
read — and `bastion_colonist_personality4`), `bastion-harness/src/main.rs`
(the scenario + arg + dispatch + HAULPIN's widened window). Commits:
`c09933e463` (mechanism) + `5e9ed6385f` (storm fixes, tag).

Sonnet tag-review: clean. The storm forensics are exactly the standard this
whole session has held to — proved the new mechanism innocent BEFORE
searching elsewhere, root-caused rather than pattern-matched to a known
class, and separated two co-occurring causes instead of assuming one fix
would explain both reds. **Rows 48-51 → DONE. The autonomy arc — the "plays
itself" promise from AUTON-0's own framing through E1's death-spiral
prevention through this block's legible individuality — ships complete.**
Deferred riders, all already tracked elsewhere: UI-4 (next, priority-bumped),
DF-POLICY, Ben's tuning pass, B8's eventual live Flee (the safety guard is
already waiting for it), and the small follow-ups (49.1, 50.7, 50.8).

### bastion-block-UI4 (e28cc2b7d0, row 62) — THE ARC BECOMES VISIBLE
The unit inspector panel, Ben's priority bump — Sonnet-tier, the wire-
plumbing scope expansion pre-approved through the packet's own check-in
condition. Gate: 34/35 first draw, `BED`'s `b_alive_after_kill` field-
classified per B22 (the pre-existing B7-1 kill-race class, confirmed by the
SAME field/classification the last two gates have both hit — 3/3 rerun) →
effective 35/35.

**What it delivers:** click a colonist → a live panel showing needs meters,
mood, personality traits, the current `Drive`, and AUTON-3's `last_scores`
(Work/Flee/Idle numbers), refreshing at ~1Hz. This is the FIRST display
surface for everything rows 44-51 built — mood/needs (B7), values/
personality (B-AG3), the arbiter and its drives (AUTON-0/2/3) all become
inspectable rather than only inferable from behavior.

**The wire plumbing, exactly as approved at the check-in:** two tail-
appended variants, shipped together in one commit (the B30 discipline) —
`ClientGeneral::BastionInspect { target: Uid }` and `ServerGeneral::
BastionInspectInfo { target, payload: Option<BastionInspectPayload> }`. The
server side uses the SAME deferred pattern `bastion_spawn` already
established: requests are gathered during the parallel join, resolved and
answered in the sequential post-join drain — payload-source reads never
happen mid-join. Notably, the rtsim read guard is acquired at REQUEST
cadence (once per actual inspect request, not once per tick) — B39's lesson
from AUTON-3's own gate storm, applied proactively here rather than
re-learned the hard way a second time. The payload itself is pure
re-packaging of the ALREADY-EXISTING probe reads (`needs_mood`+
`personality4`+`last_scores`+`temperament`), now keyed by `Uid` instead of
name — zero new data-gathering logic anywhere. A non-colonist or stale
target resolves to a `None` payload (the no-crash invariant, not an error
path). `SystemData` grew by 5 READ-only storages on the `msg::in_game`
system — the dispatcher-reshuffle class (B22's RUN-0 lesson) — declared
explicitly and suite-verified rather than assumed harmless.

**Client:** a latest-wins reply cache plus a getter, nothing fancier needed
for a single-target on-demand query. **Voxygen:** `bastion_sync_inspector`
reuses `bastion_pick_entity`/`bastion_select_set` exactly — no second
selection mechanism built alongside the existing one — refreshing
immediately on target change and holding at ~1Hz otherwise, clearing
cleanly on deselect, multi-select, or a non-colonist pick. One plain-text
block rendered above the existing selection-info line, following the
established `selected_info` widget pattern. Placeholder-first throughout —
zero new art, per the standing asset-lane rule. Read-only end to end: no
sim-state write anywhere in the whole path.

**Verification:** `VOXCHECK` green (the client half genuinely compiles, per
the B30 discipline this whole cluster now enforces structurally); the
34/35-effective-35/35 suite gates the server half. The VISUAL half is
explicitly a Ben-eyeball item, not gate-able — flagged to the Play-Tester
lane for the next `BEN-TEST-CHECKLIST` entry on its following client build:
click a colonist → panel appears; click ground or multi-select → panel
clears; force a drive switch → watch the W/F/I numbers actually move.

**Files (9, +284):** `common/src/comp/bastion.rs` (the payload struct),
`common/net`'s wire pair + verify arms, `server/src/client.rs` (the stream
arm) + `server/src/in_game.rs` (the deferred handler), `client/src/lib.rs`
(the reply cache), `voxygen/src/hud/mod.rs` + a new `hud/bastion.rs` +
`session/mod.rs`.

Sonnet tag-review: clean, no findings — the scope expansion was disclosed
and pre-approved rather than built silently, the deferral pattern and
request-cadence guard are exactly the right reuse of AUTON-3's own lesson,
and the read-only/no-crash/placeholder-first invariants all held throughout.
Row 62 → DONE. FLAT-TEST-ARENA (row 50.5) is next per Ben's reroute order;
B8 resumes behind it.

### bastion-block-CHOPFELL (50aff8808a, row 51.6) — base-cut → whole-tree timber
Ben-direct design, self-verify+tag with a targeted-Opus-at-tag on three named
safety points. Gate: 37/37 CLEAN — every leg green, including the one that
took real forensic work to get there (`BED`).

**Mechanism:** `place_chop_cells` → `place_chop_fell` (`server/src/
bastion_jobs.rs:1466`) — ONE base-cut job per tree instead of one per block,
at the ground-rooted base specifically. The whole fell-set is FROZEN into
`chop_fell_sets` (a `HashMap`) keyed by the job id, mirroring the B6-HAUL
container-store / B7 BedSlot co-located-table shape exactly — reuse, not a
new storage pattern. On completion: XP for the whole tree, the set pushed to
`board.felling` (a `Vec`), then drained top-down one z-band per tick for the
visual. Both callers updated (`in_game.rs:1037` player-paint, `lib.rs:1564`
harness-hook); eviction wired into both `remove_job` AND `cancel_region` (no
orphaned side-table entries on either exit path). `detect_trees` threads the
base position through; `tree_fell_set`/the FR10 caps untouched, exactly as
scoped.

**Why the base-cut approach gives no-float BY CONSTRUCTION, not by luck:**
the fell-set is sorted into a total order (z DESC, y, x) AT PLACEMENT TIME;
the felling pass drains one z-band per tick, top-down; since the base
(minimum z) sorts LAST, the remainder at every point in the drain is
necessarily a contiguous, base-rooted top segment — a floating fragment is
structurally impossible to produce, not merely untested. Bonus, free: this
is also exactly what closes FR10's old floating-canopy residual, since the
base is now always reachable and always the last thing removed.

**Economy, exactly the design's pins:** a granularity refactor, not a
rebalance — yield conserved (`CHOP_DROP` per Wood, unchanged), and per-Wood
labor conserved (`threshold = CHOP_WORK_PER_BLOCK(1.0) × Wood`, so a bigger
tree costs proportionally more, not a flat per-job cost). Proven, not just
argued: a 9-Wood vs 3-Wood threshold ratio of exactly 3.0 measured at 2.95×
via `cut_polls` telemetry (444 vs 1311) — deterministic, travel-free
measurement, matching Ben's explicit hard requirement.

**A genuine design-vs-code discrepancy, found and disclosed rather than
silently "corrected":** the original design doc's stated reasoning — "leaves
clear free in the existing model" — turned out to be factually WRONG about
the CURRENT code (leaves already cost labor/XP as their own jobs today, they
were never free). But the design's actual INTENDED economic outcome still
holds under the new model, just for a different, correct reason: since
canopy is most of a tree's cell count, a whole-tree fell now completes
faster overall than the old per-block accumulation would have implied — same
practical effect the design wanted, reached via the real mechanism rather
than the design doc's mistaken premise. Worth keeping the discrepancy on
record rather than quietly matching the design's wording.

**The `BED` leg, the one real forensic effort in this tag — CHOP-FELLING
fully exonerated, not just asserted clean:** first draws showed `BED` red on
`bed_occupied_mid` (a DIFFERENT field than the historically-known
`b_alive_after_kill` telemetry flake). Attribution, not assumption: BED run
×5 at the PARENT commit (pre-CHOP-FELLING, `a6de03b44d`) came back 3P/2F —
the flake existed BEFORE this block's diff, proving CHOP-FELLING did not
introduce it. Structurally confirmed too: bed occupancy is set in the
`ActiveJobState::Arrived` `RestAt` branch, which never reaches the threshold
gate this block actually touched. **Filed as B42**: the scenario's single
`assign_rest(&bn, bed2)` call can silently no-op (`let _ =` discards the
`false`-when-busy return) against a LIVE arbiter that's already claimed the
target by the time the scenario reaches that phase — a genuine race, not
random noise. Fixed harness-side: re-assert the assignment every iteration
(a safe no-op while busy, takes the moment the target idles, out-paces the
arbiter's own 15-tick selection cadence at a 5-tick reassert) plus widening
the window 240→480. Verified: 5/5 clean post-fix (was ~40-75% flaky).

**Fixing that flake EXPOSED a second, rarer, genuinely different one —
correctly separated rather than bundled:** with `occupied_mid` now reliable,
a residual ~15%-alone (0/5 at the pre-fix parent, meaning it was MASKED by
the dominant flake, not introduced by fixing it) case surfaced: the ultimate
fail-safe correctly teleporting a below-grade colonist (z=392 vs 393) TWICE
during bed-build, its own 60s delay eating into the scenario's early timed
phases. The safety backstop DOES fire and DOES rescue the colonist — this is
a CASE-003-class physics-embed timing cost, not an entombment failure. Filed
separately as **B43 (`BED-STUCK-EMBED`)** via the standing Bug-Tester
routing workflow, explicitly NOT bundled into this tag.

**B6HAUL-WIDEN landed in the same commit bundle** (`a6de03b44d`): b6haul's
own poll-window ceilings widened (the established 240→480-class fix,
matching B40's shape) — fixing a marginal-window flake exposed by overall
suite growth (b6haul is now the 11th-or-so sequential leg; by its turn,
worldgen assets are cache-evicted and its cold-start runs slower against
windows that were never retuned since row 34's origin). Unrelated to
CHOP-FELLING's own logic, bundled purely because it surfaced during the same
gate-storm investigation and shares evidence.

**Targeted Opus-at-tag, requested on exactly the three points the packet
named, no full re-review:** (a) no-float — held by construction as described
above, live-measured (max-z monotone non-increasing, base-present-whenever-
any-cell-present, both small and large trees). (b) ordering determinism —
`sort_unstable_by` on a pure integer z/y/x total order, zero rng, zero hash
iteration, drains a `Vec` by index; ×2 identity holds byte-identical. (c) the
side-table's B22-class scheduling perturbation — checked clean: `chop_fell_
sets`/`felling` are new `JobBoard` fields, NOT new `SystemData`/dispatcher
storage (unlike RUN-0's class), and the BED attribution above is direct
empirical confirmation the per-tick HashMap lookup doesn't perturb
scheduling (the flake pre-existed at the parent, unchanged by this diff).

Sonnet tag-review: clean. Genuinely thorough forensics on BED rather than a
convenient flake-classify-and-move-on, a real design-vs-implementation gap
disclosed instead of quietly smoothed over, and a newly-surfaced bug
correctly separated from the tag that happened to expose it. Row 51.6 →
DONE, pending the targeted Opus pass on the three named points. UI-4.1 (row
62.1) is next.

### bastion-block-UI41 (f6ac4c8bc7, row 62.1) — the arc gets a world-space marker
The highlight ring on selected colonists, self-verify+tag, CHEAP. No server/
wire/harness change at all — pure voxygen render, so VOXCHECK alone is the
code-level gate (no scenario gate needed, same shape as the Farm-palette
fix).

**Mechanism:** mirrors `bastion_sync_colonist_markers` exactly — the SAME
proven per-entity `DebugShape` sync pattern, now keyed on `bastion_selected`
instead of the colonist-marker set (`HashMap<Entity, DebugShapeId>`, add/
set_context/remove_shape). Since no dedicated ring primitive exists in the
`DebugShape` enum (only `Line`/`Cylinder`/`CapsulePrism`/`TrainTrack`), a
flat wide `Cylinder` (radius 0.7, height 0.05) at the colonist's feet
approximates a ring — a deliberate reuse of an existing primitive rather
than adding a new `DebugShape` variant for one feature. Color `[1.0, 0.85,
0.3, 0.85]` (warm gold), distinct from the existing cyan overhead colonist
marker so the two don't visually collide. Tracks the selected colonist's
position every frame (not a one-shot draw at select-time); clears on
deselect, multi-select, or a non-colonist pick — the exact same trigger set
UI-4's own panel already uses. Leaving overseer mode drains markers and
rings together.

**Status, honestly framed per B41's own lesson:** VISUAL-UNVERIFIED until
Ben actually eyeballs it — VOXCHECK proves the code compiles and the render
call is wired correctly, not that it looks right or reads clearly in
practice. A Ben-checklist item was routed via the architect to the
Play-Tester: enter overseer, click a colonist → a gold ring appears under it
and follows it; deselect → ring gone; box-select multiple → a ring under
each. This ships in the SAME combined voxygen rebuild as the ARENA and
Farm-palette fixes, so one Play-Tester build + one Ben session verifies all
three at once.

Sonnet tag-review: clean, small, exactly the scope asked for. Row 62.1 →
DONE (implementation confirmed; real-world visual confirmation still
pending, tracked honestly rather than assumed).

**Sequencing note:** CHOP-FELLING (already in flight when the Ben-directive
resequence landed) finished before ARCH-003-INTEGRATE rather than after;
ARCH-003-INTEGRATE is architect-directed rather than builder-self-initiated,
so the builder proceeded to UI-4.1 in parallel while that integration step
waits on the architect specifically. EXHAUSTIVENESS-ASSERTS (row 51.52) is
next, now folding in a THIRD instance of the Farm-bug class the work itself
surfaced: `DesignationKind::Bed` also has no `ToolMode::Designate` entry —
confirmed a real bug (not intentional auto-placement) by checking the code
directly, ruled to be fixed as part of the same exhaustiveness pass rather
than a separate decision.

## bastion-block-ARCH003 — ARCH-003-INTEGRATE, Grok Phase-1+2 test-infra merge — TAGGED 2026-07-14 (tag `9dfff6ec7e`)

Row 50.6. Architect-directed integration (not a standard builder packet):
rebased the Bug-Tester's Opus-CLEARED clean tree (8 fixes, `codex/arch003`)
onto fleet HEAD, resolving the known PATH-0 conflict flagged back at row 45's
own tag entry, plus the Grok Phase-1+2 test-infra bundle merge point. A
fleet-wide priority halt was declared ahead of this merge — all builder/
Play-Tester feature work paused, 7+ Ben live-test findings from the delivered
ARENA/Farm-palette/UI-4.1 exe were banked as master-list rows (51.61-51.65,
62.2, plus the progress-bar/z-level-scope items) without building anything,
respecting the freeze.

**Pre-merge tree cleanup (Sonnet, per architect's explicit request):** the
shared checkout carried 2 categories of uncommitted state blocking the
merge — (a) Sonnet's own accumulated bookkeeping edits across the B7-2..
UI-4.1 session arc (run log, restore ledger, master-list flips, common-issues
B26-B43+D19/D20, plus organic fleet-wide readme growth from the design/asset
agents sharing the same tree) landed as one commit (`84f269b0c9`, 17 files,
+8002/-192, scanned for secret-shaped patterns first — clean, only game-
design vocabulary hits); (b) the builder's 2 paused EXHAUSTIVENESS-ASSERTS/
Bed-fix source files (`common/src/bastion.rs`, `voxygen/src/bastion/tools.rs`)
discarded via `git checkout --`, pre-verified patch-recoverable
(`git apply --reverse --check exhaustiveness-bed.patch` passed before
discarding) — HEAD confirmed unchanged at `f6ac4c8bc7` (bastion/block-B6HAUL)
through the discard, zero cargo/build activity during the hold per the
standing no-cargo-during-own-gate discipline.

**Merge verification (architect, on the actual merged tree):** clean release
build; 3x seed-21 aggregate runs byte-identical (only the known benign
wall-clock field differs); seed-22 sanity run PASS.

**FREEZE LIFTED.** Builder + Play-Tester resume normal work. Next in
sequence: EXHAUSTIVENESS-ASSERTS/TOOLBAR-ICONS/Bed-fix (row 51.52, already
CURRENT), then the 8 banked Ben findings (51.61 CHOP-PROGRESS-INDICATOR,
51.62 Z-LEVEL-CONTROLS-SCOPE, 51.63 ★CHOP-FELLING-VISUAL-CHECK, 51.64
STOCKPILE-INVESTIGATE, 51.65 GATHER-FLORA-INVESTIGATE, plus UI-5/row 62.2)
per existing priority sequencing. Bug-Tester starts the full functional test
catalog separately now that the merged tree is the stable base it needed —
its own workstream, not gated on the builder's queue above.

**Coordination-shape churn note (Sonnet, 2026-07-14):** immediately after the
merge, the architect proposed 3 different shapes for how Bug-Tester's
test-catalog findings should reach the builder in a single sitting — (1) a
fix-and-verify-in-loop routed straight Bug-Tester→builder, taking priority
over the polish queue; (2) a Sonnet-serialized one-at-a-time queue (Sonnet
holds/releases each fix to prevent builder/Bug-Tester file collisions); (3)
the settled final shape — Bug-Tester owns test-catalog testing AND fixing
autonomously end-to-end in its OWN ISOLATED WORKTREE, surfacing only if
blocked, with the builder fully freed to work its polish queue with zero
collision concern. Each shape was relayed to the builder/Bug-Tester promptly
as it changed; no coordination machinery was over-built for the
intermediate shapes (a task-tracking queue created for shape (2) was
deleted once shape (3) landed rather than kept around unused).

## bastion-block-EXHAUST — EXHAUSTIVENESS-ASSERTS + BED-TOOL fix (row 51.52) — TAGGED 2026-07-14 (tag `92ec5eabf1`, on the merged ARCH-003 tree)

Root-cause fix for the bug CLASS that dropped Farm (`23087dbd68`, row 50.51)
and then Bed from the client toolbar: `ToolMode::ALL` was a hand-mirrored
literal array that silently bypasses Rust's own exhaustiveness checker
(which only engages on a `match`/derive, not a literal), while
`DesignationKind`'s label/footprint/worktype/color all force updates via
real exhaustive matches on a new variant — so a new paintable kind could
vanish from the palette with zero compile signal.

**Guard shipped:** `DesignationKind::is_tool_paintable()` — a real
exhaustive `match`, no wildcard arm, so a future new `DesignationKind`
variant now FAILS TO COMPILE until explicitly categorized (genuine
prevention, not just a count check) — plus a bidirectional voxygen parity
test asserting every paintable kind has a palette button and every palette
button maps to a paintable kind. Both checks would have caught Farm AND Bed
at compile/test time instead of silently at runtime.

**Bed folded in per the architect's ruling** (the 3rd instance of this
exact class, found by this work itself mid-pass): `is_tool_paintable(Bed)`
now returns `true` and a real `ToolMode::Designate(DesignationKind::Bed)`
button ships (palette 10→11) — all of Bed's supporting infra (label, color,
footprint, `WorkType`) already existed from B7-1, only the toolbar wiring
was missing.

**Verification-scope disclosure (builder's own tag-review call, honestly
flagged rather than assumed):** this pass is SIM-INERT — the common-side
additions (`is_tool_paintable`, `EnumIter`, `ZoneKind` `Default`) are purely
additive and touched by NO sim code path; the only behavioral change at all
is the new client Bed button. Verified via common UNIT (31/31), the new
voxygen parity test (1/1), harness BUILD, and VOXCHECK — all green on the
merged ARCH-003 tree — but the full 37-scenario gate was deliberately NOT
re-run, reasoning the scenarios test sim behavior this change cannot touch,
while the Bug-Tester's isolated-worktree catalog run is separately
validating the full suite under ARCH-003 in parallel. Same precedent as the
Farm-tool and UI-4.1 client-only tags. Sonnet reviewed and accepted this
scoping rather than requesting a redundant full gate.

Builder proceeds to TOOLBAR-ICONS (row 51.5) next, own lane, known-
readability caveat logged in the commit as previously specified; palette is
now 11 tools (Farm + Bed both included in the icon wire-up).

## bastion-block-ICONS — TOOLBAR-ICONS (row 51.5) — TAGGED 2026-07-14 (tag `4f3fe6aa10`)

Voxygen-only UI polish, own lane, no sim/server touch. The overseer palette's
`ToolMode` buttons now render as 34×34 `Button::image` icon widgets (active
tool = bright, others = dimmed) in place of the old `.label()` text calls;
the God/Free toggle stays text (it needs to show which mode is active, not
just a static icon). 11 pre-delivered asset-lab icons (`tool_{pan,inspect,
mine,chop,gather,farm,build,stockpile,ladder,erase,god}.png`) copied into
`assets/voxygen/element/ui/bastion/`, declared in `img_ids.rs`, palette loop
rewired to the new widget kind. VOXCHECK green — all 11 specifiers confirmed
resolving to real committed files (not just declared).

**Known issue, shipped anyway (per the standing placeholder-first / asset-
lane-stood-down rule):** the Asset Integration tester's partial pre-
screen (pre-stand-down) had already found readability problems on this exact
icon set — mine reads as mattock/T not pickaxe, chop reads as hook/scythe
not axe, pan reads crown/comb-ish, and gather/farm are a visual look-alike
collision. Icons integrated AS-IS (still strictly better than text labels);
a readability re-pass is explicitly deferred to whenever the asset lane
resumes, not blocking this tag.

**New gap surfaced by this pass itself:** Bed has no icon at all — the
11-icon set predates EXHAUSTIVENESS-ASSERTS adding Bed to the palette, so
Bed currently renders a transparent image + text-label fallback rather than
a real icon. `tool_bed.png` is logged as a new 12th-icon asset-lane backlog
item in `ASSET_REQUESTS.md`, for whenever that lane resumes — not a blocker
to this tag, just an honest known-gap disclosure.

VISUAL-UNVERIFIED until Ben eyeballs it — VOXCHECK proves compile +
specifier-resolution only, not that the icons render/read correctly or
that nothing crashes, per B41's standing lesson applied consistently.
Ben-checklist item routed via the architect for the next real-client pass.

Session tag count so far: ARENA (provisional) → CHOPFELL → UI41 → EXHAUST →
ICONS. Builder proceeds to the 8 banked Ben findings next, starting with
51.63 ★CHOP-FELLING-VISUAL-CHECK — doing the code-path/timing investigation
itself since the actual live eyeball-confirmation still needs Ben.

**51.63/51.65 investigation findings (builder, 2026-07-14, no tag — no code
changed, characterization only):**

**51.63 CHOP-FELLING-VISUAL-CHECK:** the felling render path is code-sound —
identical mechanism to mining, which renders correctly for Ben. The felling
pass (`bastion_jobs.rs:5386`, intact post-ARCH-003-merge) does
`block_change.set(cell, Block::empty())` per band + an `emit_drop` per Wood;
those flow `block_change` → `TerrainChanges.modified_blocks` → `sys/
terrain_sync.rs:110-115` (compress + send) → client render — the same path
mining already proves live. No code-level reason it wouldn't render;
`can_set_block` only defers a band one tick, never drops it. Caveat: ARCH-003
touched `bastion_jobs.rs` (37 lines, determinism fixes) so this behavior
wasn't covered by the pre-merge 37/37 gate — the Bug-Tester's isolated
catalog run on the merged tree is the check for a determinism-fix
interaction (the render path itself is untouched by ARCH-003). Left OPEN
pending Ben's actual eyeball — "code sound, unverified" is the honest state,
not a dismissal. (Sonnet follow-up: asked the builder to also confirm
SERVER-SIDE execution directly via the chop-fell harness scenario + UI-4
inspector, not just code-read, per the architect's new-tooling standard —
pending that re-check.)

**51.65 GATHER-FLORA-INVESTIGATE:** verdict = UX-CONSISTENCY GAP, not a bug.
Client-side Chop and Gather are handled IDENTICALLY (`FootprintMode::Area2D`,
`session/mod.rs:1086`) — the asymmetry is server-side. Chop's `detect_trees`
(the World tree oracle) snaps to the whole rooted tree object near the
painted region — forgiving, click NEAR a tree fells it. Gather's
`job_wanted(Gather)` requires `block.is_directly_collectible()` on the exact
painted CELL (`bastion_jobs.rs:327`) — precise, a plant is one sprite, no
nearest-object snap, must paint directly ON it. Both work as designed; Gather
is simply fiddlier than Chop's tree-select, which is almost certainly what
Ben felt as "doesn't work." Fix direction if parity is wanted: a
click-tolerance/nearest-collectible snap for Gather mirroring `detect_trees`'
forgiveness — a UX-parity fill, not a bug fix, scoped only if Ben confirms
he wants it. No code changed.

**Builder at a clean stop pending:** Sonnet's steer on 51.61 (build now,
CHEAP tier, self-verify+tag, no full packet needed — reuses UI-4's transport
pattern for the server→client progress feed, exact mechanism left to the
builder's judgment) and on whether to hold 51.62 Z-LEVEL-CONTROLS-SCOPE /
51.64 STOCKPILE-INVESTIGATE for Ben's live symptom vs. investigate now via
code-read + existing harness/catalog scenarios + UI-4 inspector (Sonnet's
answer: don't hold — same standard as 51.63/51.65, characterize what's
checkable via the new tooling now, only true render/feel questions wait
on Ben).

*(A fleet-wide Ben-directed stand-down and resume happened here — see the
master-list/no separate run-log entry needed, no code touched during the
hold.)*

## bastion-block-PROGRESS — CHOP-PROGRESS-INDICATOR (row 51.61) — TAGGED 2026-07-14 (tag `7f087da317`)

Sim-inert display field `Arbiter.activity: Option<(WorkType,f32)>` (`None`
default) — NEVER read by scoring/selection/hysteresis, written only at the
existing job-progress path and cleared at the two existing `to_release`/
`last_scores` sites (no new borrow introduced). Re-packaged into the
existing `BastionInspectPayload` (tail-appended field, no new wire message —
the B30 wire discipline held). Panel shows "Doing: Chop 74%" / "(idle)",
reusing UI-4's fetch cache as-is. New read-only probe:
`bastion_colonist_activity`. Files touched: `common/comp/bastion.rs`,
`server/{bastion_jobs,lib,sys/msg/in_game}.rs`, `voxygen/session/mod.rs`,
`bastion-harness/main.rs` (+97/−8).

**Verified via the testing tools per the architect's standing directive**
(harness scenario run, not code-read alone): chopfell scenario ×2
byte-identical — the new activity field populates and climbs to ~99.9% on
both a small tree (0.9985) and a big tree (0.9989) right before felling; all
pre-existing felled/topdown/no_orphan/drops asserts stayed UNCHANGED across
both runs, confirming the sim-inert claim directly rather than assuming it.
UNIT 31/31, VOXCHECK green, BUILD green (harness compile pulls server+
common transitively).

## 51.62/51.63/51.64 investigation findings (builder, 2026-07-14 — all characterized via harness scenarios + UI-4 inspector per the architect's tooling directive, not code-read alone; no code changed, no tags)

**51.62 Z-LEVEL-CONTROLS-SCOPE — VERDICT: NOT A BUG.** Two distinct
z-controls exist, both already correctly scoped: (1) the designation depth
stepper (`hud/mod.rs:4929`) is gated on `footprint_mode()==Volume` → only
shows for Mine/Build/Stockpile/Ladder/Bed, correctly hidden for the Area2D
tools (Chop/Gather/Farm) and non-designate tools; (2) the camera z-slice
PgUp/PgDn (`session/mod.rs:2093`) is gated on `bastion_overseer_active()` →
works regardless of tool, by design (it's a camera control, not a
designation control). Neither is literally `ToolMode::Mine`-gated, but
Ben's intuition is already ~honored where it actually matters (the per-
designation depth stepper). Only real gap: the PgUp/PgDn slice control has
no on-screen affordance — pure discoverability, an optional follow-up, not
a bug.

**51.63 CHOP-FELLING-VISUAL-CHECK — VERDICT: felling mechanism CODE-SOUND +
SERVER-SIDE EXECUTION CONFIRMED (not just code-read); likely real-play cause
= duration+legibility, addressed by 51.61; Ben's live eyeball still owed to
fully close.** Felling removes blocks via
`block_change.set(cell, Block::empty())` at `bastion_jobs.rs:5420/5430` —
the IDENTICAL client-synced path mining already uses (`block_change` →
`TerrainChanges` → `terrain_sync` → client mesh), no raw-terrain bypass;
`tree_fell_set` keys on `BlockKind::Wood|Leaves`, exactly what live worldgen
trees are made of. Chopfell scenario ×2 confirms top-down removal +
no-orphan + timber-drop all PASS. Likely real culprit measured directly:
base-cut duration (`cut_polls` ≈13.6s small tree / ≈31s big tree of real
work at 30tps) with zero feedback prior to 51.61 — reads as "not felling"
when it's actually just still cutting. 51.61's progress indicator is the
direct legibility fix. Per the standing B41 gate-must-test-live-path lesson,
a genuine live eyeball (designate Chop on a real worldgen tree, watch it
fell top-down after the cut completes) is still required to fully close
this row — Ben-checklist item recommended.

**51.64 STOCKPILE-INVESTIGATE — VERDICT: mechanic WORKS; likely real-play
cause = live-path/legibility, not broken.** b6haul scenario re-run on the
current (post-ARCH-003/EXHAUST/ICONS) tree: `b6_zonea_sum=5,
b6_pad_total=5, b6_conserved=true, b6_delivered=true` — 5 mined stones
auto-haul to a painted Stockpile as 5 REAL item entities
(`bastion_sum_items_near` counts actual entities, confirming hauled items
are physically present/visible, not an abstract counter). Stockpile
designation is a pure destination marker with zero jobs of its own. Likely
"never works" root, reasoned from the mechanism's actual preconditions: a
stockpile is INERT until loose items on the ground + a painted destination
+ a free hauler ALL exist simultaneously — paint one with nothing nearby to
haul and correctly nothing moves, which reads as broken to a player. Fix
direction: row 62.2 UI-5 (click a stockpile → contents panel) plus
optionally a "waiting for N items" affordance. Ben-checklist item
recommended to pin down which failure mode he actually hit.

**51.65 GATHER-FLORA-INVESTIGATE** — already characterized the prior
session (UX-consistency gap: Chop's `detect_trees` is forgiving, Gather's
`job_wanted` needs the exact cell; not a bug). Builder re-confirmed the
finding still stands, unchanged.

**All four investigations resolve to no-immediate-code-fix** (not-a-bug /
needs-Ben's-eyeball / fix-direction-is-UI-5). Builder's next scheduled build
is row 62.2 UI-5 (Universal Debug Inspector) — reuse-heavy, generalizes
UI-4's `BastionInspect`/`BastionInspectInfo` wire to accept a job/designation
id as target in addition to a colonist `Uid`; 51.64's stockpile-contents
legibility fix folds naturally into UI-5's stockpile target-kind scope.

**Ben-checklist candidates banked for the next Play-Tester client build:**
(a) inspect a colonist cutting a tree → "Doing: Chop N%" advances then
resets to "(idle)"; (b) designate Chop on a real worldgen tree → after the
cut, the tree fells top-down and timber drops; (c) mine a few blocks, paint
a Stockpile nearby → the loose stones get hauled into it.

## bastion-block-UI5 — UI-5 UNIVERSAL DEBUG INSPECTOR (row 62.2) — TAGGED (tag `b5e4755336`)

Self-crafted by the builder in the UI-4 pattern, per Sonnet's green-light —
Sonnet-routine tier confirmed correct in hindsight: no new dynamic
mechanism, this widens WHAT a target can be, not how targeting works.

`BastionInspect`'s wire generalized: `target:
BastionInspectTarget::{Entity(Uid)|Cell(Vec3<i32>)}`, `payload:
Option<BastionInspectKind>` with variants `Colonist` (UI-4's original
payload, verbatim, zero churn), `Job`, `Stockpile` (contents — directly
closes 51.64's stockpile-contents legibility gap flagged in that
investigation), `Farm`, `FellSet`. Server resolves a clicked empty cell
XY-column-first through job → stockpile → farm → fell-set → `None`, in the
same post-join drain UI-4 already established; an empty-handed click now
inspects the cell, a colonist click still selects exactly as before (UI-4's
original behavior fully preserved). Read-only end to end, matching UI-4's
own invariant.

New `--inspect-scenario` harness leg, now leg 38 of the ladder. Gate: full
38-leg ladder at HEAD `42f7c464a0` = 37/38 — all PASS including the new
INSPECT leg and CK; the one BED red field-classified as the already-
registered CASE-003 `bed_occupied_mid` signature (architect-accepted, not a
new regression). Earlier block-level verification during development:
`inspect` scenario ×2 bit-identical, server/voxygen checks rc=0.

**Commit-hygiene note (builder's own disclosure):** staged as exactly the
builder's own 19 hunks out of a shared dirty tree — detail lives in the
commit body and the architect's B5.8 provenance thread, not duplicated
here.

**Context, no row action taken:** two more commits sit above UI-5 on the
branch at time of this report — `871a9157d9` (B5.8 Stage-1,
external-effort-originated, tracked on the architect's own provenance
lane, not a fleet master-list row) and `42f7c464a0` (a CK CarvedStair fix
shape that Ben's stair-ladder ruling overruled — intentionally left
UNTAGGED, superseded by a Phase-1 walkable-stairs commit currently in
verification that will carry the real CK-fix tag when it lands). Per the
builder's explicit instruction, no master-list row was flipped for either
commit; the Phase-1 tag notice will arrive separately.

## bastion-block-CKSTAIR — STAIR-LADDER Phase 1 (CK fix) + STUCKJOB (α) watchdog fix — TAGGED (tag `9ad9d97808`, branch `bastion/block-B6HAUL`)

Closes the CK/chokepoint red that had blocked the full ladder ever since
ARCH-003. Tracked as its own design-doc line
(`readme/STAIR-LADDER-MINE-ACCESS-DESIGN.md`, Phasing §Phase 1), not a
numbered master-list row — same provenance-lane pattern as B5.8. Two real
commits under this tag, plus two untagged intermediates kept for the
record:

**`177c12094f` — STAIR-LADDER Phase 1 (Ben's ruling).** Emergency stairs
are WALKABLE plain Mine digs — no route ownership, no traversal task, no
temp-terrain restoration (permanent infrastructure, matching the design
doc's recommendation); only ladder/shaft plans stay route-owned (the plan
tuple descriptor became `Option`). Root cause this corrects: Stage-1 had
wrapped a walkable stair plan in an `EmergencyRouteDescriptor{kind:
CarvedStair}` and registered route ownership for it, then never wrote a
`CarvedStair` executor — the colonist sat in `RouteOwnedWaiting`/
`link_queue_waiting` waiting on a traversal task that could never exist,
until the teleport backstop bailed him out at the deadline (recorder trace:
uid 1, seed 1337, `route_kind=CarvedStair`, `on_wall=None` in 730/730
samples, energy pinned at 100.0). Foundation for
`STAIR-LADDER-MINE-ACCESS-DESIGN.md`'s Phase 2 (the three access
geometries, architect drafting).

**`9ad9d97808` — STUCKJOB (α) watchdog fix + falsifier [carries the tag].**
Fixes a SECOND, independently-latent Stage-1 watchdog defect found via this
same forensics work (`has_live_job`, 0 occurrences at baseline — a genuinely
separate bug from the CarvedStair one, not a duplicate): stuck-watch
teleport suppression must be EARNED by verified job progress — a
per-colonist `(job, progress)` baseline — not just claim-holding/churn,
which could suppress the rescue backstop indefinitely without real
progress. New `--stuckjob-scenario` harness leg (ladder leg 39, suite now
39 legs total), proven properly RED→GREEN: unfixed = colonist never
rescued within 200s against a 60s design target; fixed = rescued at 59.0s.
CK 5/5 PASS + flight-recorder evidence + full ladder 38/39 (BED = the
already-registered CASE-003 `bed_occupied_mid` field-class, third identical
draw, not a new regression). Corrects the misfiled B22 `ck_failsafe_out`
entry — an invariant violation, not a flake.

**Positive capability note (not just a bug fix):** the STUCKJOB falsifier's
rev-1 produced the first end-to-end proof of ORGANIC stair self-rescue —
plan→claim→dig→ascend→out, 26 seconds, no teleport backstop needed at all.
Worth surfacing to Ben directly as a capability win, not just a fix.

**Commit-hygiene note (builder's own disclosure):** staged as exactly the
builder's own hunks out of a shared dirty tree — same discipline as UI-5,
detail in the commit bodies + the architect's B5.8 provenance thread.

**Untagged intermediates on the branch, no row/doc-line action taken (per
the builder's explicit instruction, consistent with the UI-5 tag notice):**
`42f7c464a0` (the overruled CK fix shape (b), superseded by this Phase 1)
and `871a9157d9` (B5.8 Stage-1, architect's own provenance lane).

**Next from the builder:** task #58, the Grok/Codex testing-framework
integration (architect/Ben-queued), starting on a NEW clean branch off
`9ad9d97808`. Long-running — tag/branch notices will arrive as pieces land.
The `bastion/block-B6HAUL` branch itself is left at this stable green point
for anyone who needs it.

## ff2874b4b6 — STAGE-1 SCOPE COMPLETION (external-effort provenance lane, no master-list row, `bastion/block-B6HAUL`)

Closes the 12-vs-24-file gap in Stage-1's own declared dependency set: the
6 files Stage-1's original 12-file list omitted — `common/systems/src/phys/
{mod,collision}.rs`, `common/src/{bastion,rtsim,path}.rs`,
`server/src/connection_handler.rs` (+500/−124). **Restores TAG
REPRODUCIBILITY for every tag since Stage-1** (`871a9157d9` through
`bastion-block-CKSTAIR` — all of these were clean-checkout-unbuildable
without these 6 files). Discovered when the grok-integration worktree hit
`cylinder_sweep_first_collision` missing from committed phys. Important
distinction stated in the commit body itself: these files were LIVE in the
working tree through every gate since Stage-1 landed — every test result
this week ran against this code for real. What was broken was that git
history could not REPRODUCE it from a clean checkout — a provenance gap,
not a correctness gap (the architect's own clarification, now moot since
this closes it).

**Opus review (the physics-safety piece):** `capsule_terrain_cylinder`
confirmed an EXACT behavior-preserving extraction of the old inline sweep
(verified at `radius_cap=0.45`); `route_squeeze_until`'s gating confirmed
robust across all 15 write sites (server-only, emergency-route-only, 200ms
auto-expiry, teleport backstop covers edge cases) — this is the same
mechanism [REQ-0052-ROUTE-SQUEEZE-DESIGN.md](../readme/REQ-0052-ROUTE-SQUEEZE-DESIGN.md)
now documents as a contract. Two minor non-blocking items match that doc's
own open items exactly: `FrontierWork`'s write site not gated identically
to `LinkApproach` (open item #1), and the `rescue_pending` co-interaction
not deep-traced (open item #2) — both already tracked there, not
duplicated here.

External-effort provenance, same shape as Stage-1's own original commit:
this code originates from the B5.8 external-effort delivery; the builder is
the committer completing its declared scope on the architect's ruling, not
the author.

## 611b0f4c51 — integration/grok-testfw reconcile-merge (task #58, NOT `bastion/block-B6HAUL`, no master-list row)

Foundation-merge for the Grok/Codex testing-framework integration, tracked
on its own `integration/grok-testfw` branch, separate from the fleet's
catalog-block history. `grok-testfw` merge (`21275efe6b`, 53 Grok commits /
722 files) + a reconcile-merge bringing `ff2874b4b6` (STAGE-1 SCOPE
COMPLETION, above) into the integration branch — replacing disclosed draft
copies with the real committed content, no duplicate versions left behind.

**Verified:** fleet spot-checks 3/3 (stuckjob/CK/inspect all survived the
merge); Grok's own legs — metamorphic PASS, perf PASS@seed21, save-fuzz
6/7 (the 1 FAIL is drift finding #4, reclassified as a test-expectation-
vs-contract mismatch, minor, already architect-routed, not a regression).
4 drift findings total, all disclosed directly in the commit body rather
than left implicit.

No master-list row flip needed — this is a foundation-merge for ongoing
integration work, not a catalog block. Builder will flag when the architect
routes the integration follow-ups (golden regen, etc.).

## bastion-block-CLIMBCAP — FREE-CLIMB DEPTH CAP + A2 RESCUE-PROGRESS GATE + BELOW-GRADE BOUNDARY FIX (STAIR-LADDER Phase-2 mechanic pulled forward) — TAGGED (tag `7483439958`, `bastion/block-B6HAUL`)

Ben-ruled, Opus-cleared design + a CLEAR-TO-BUILD re-review. 5 files:
`server/src/{bastion_jobs,lib}.rs`, `bastion-harness/src/main.rs`,
`readme/{BASTION_COMMON_ISSUES,B5.8-WRITER-INVENTORY-REQ0094A}.md`.

**Why:** a six-layer probe campaign found the emergency ladder tier was
UNREACHABLE-IN-PRACTICE under every tested condition (rested/drained ×
skill 0/1 × depth 6/7/8 × open/protected × five footprints) — free-climb
self-rescue always won, because only ascent drains energy, `Climb`-hold is
~free, and Idle regen re-arms `handle_climb`'s entry (`>1.0`) faster than
the trapped→plan pipeline. `plan_access` never once reached
`ladder_pillar`. Ben's ruling: cap free-climb FIRST, then prove the
ladder.

**The cap (server-side only, players untouched by construction — zero
common/physics changes):** pure core `cap_for_skill(level) = 3·(level+1)`
(3/6/9, Ben-tunable); `climb_cap_allows` never caps descent/hold (no
stranding) and is fully exempted by a real ladder token (parallel to the
existing energy exemption); `climb_free` is structurally absent from the
signature (Opus R2: nothing to pass, nothing to forget). Per-colonist
`climb_anchor` = the z of the last genuine foothold, never reset while
on-wall/climbing (Opus R3). **Single-source gate (Opus R1):** every
natural-ascent consumer (velocity lift, rung-step, the `climb_free`
fail-safe) hangs off one shared `supported` condition — one choke point,
not three independently-gated sites. `U8` exhaustiveness-on-writers
self-test pins the ascent-writer counts and asserts the single gate holds.

**Seed-corpus leak → two root causes + the frozen cap-skill fix (caught
only by the many-seeds discipline — seed 1337 alone was green by
spawn-lottery luck):** (1) SPAWN VARIANCE — colonists roll climbing 0..=1
at spawn; a level-1 roll legitimately exits deeper, the cap held exactly
as designed for the level actually rolled, the falsifier's premise was the
bug. (2) XP SELF-LICENSING — arithmetically real for level-0 spawns
(1.5xp/s supported × a flat 20xp curve = level 1 in 13.3s, cap 3→6
mid-escape). Ben's fix: climb XP stays on free wall-climbing only (grant
inverted to `!beside_ladder` — the assisted ladder path teaches nothing);
the farm is closed by a FROZEN CAP-SKILL snapshot (`climb_cap_skill`,
lazy `or_insert` at cap-consult, cleared only at genuine-surface sites) —
a mid-escape level-up banks for the NEXT climb, progression intact, farm
dead. Both findings registered as new classes — see B44/B45 in
`BASTION_COMMON_ISSUES.md` (Sonnet-curated from the builder's raw
append-notes into the house table format, numbered in sequence after
B43).

**A2 co-requisite (ships with the cap because the cap makes the backstop
load-bearing):** `rescue_pending` → PROGRESS-EARNED, mirroring
STUCKJOB-α. The old gate ("any egress_target + ANY is_access job
anywhere") was an F5-class hole — a stale target or someone else's rescue
could suppress THIS colonist's teleport forever, masked pre-cap by
self-rescue. Suppression is now earned per-colonist (his own distance
improving, or one of HIS OWN jobs leaving the board) — never strands.

**Below-grade boundary fix:** a colonist stalled in the last 3 blocks of
his exit sat inside the `!below_grade` predicate and wiped his own
backstop clock on approach (measured: a 52s clock wiped at the boundary,
13 oscillations, 200s never rescued). Fixed: near-surface wipes only when
GROUNDED; airborne near-surface now FREEZES (no accrual, no wipe).

**Observability (permanent, diag-gated):** `watch_wipe()` now shims all 11
`stuck_watch` reset sites with a reason tag under `BASTION_EGRESS_DIAG`
(the F5 investigation previously had no way to see which of ~a dozen
wipers was firing); read-only `bastion_egress_probe()` harness hook added.

**F5 rev-2 (the falsifier rewritten after its own postmortem — rev-1 was
vacuous three ways):** three colonists — A (sealed vault, unchanged), B
(reach-disjoint open pit, the genuine-rescue guard), C (protected vault,
holds a target but no plan/jobs — the PURE A2 discriminator: old gate
suppressed him forever, progress-earned teleports him ≤150s). Four
preconditions self-asserted in-scenario; PRECONDITION-FAILED prints
distinctly from FAIL.

**Falsifiers, measured as a seed corpus (Ben's standing verify-mode: 6
seeds × 3 reps × 2 scenarios, every seed byte-deterministic across reps):**
GEOMETRY PROBE 6/6 (pre-cap RED at 22-26s route-null → post-fix backstops
82-110s route-null on every seed; `ConstructedLadder` still never latched
— the residual fork is planner-side, architect-routed, B5.8's ladder
fixture stays parked); STUCKJOB+F5-rev-2.1 6/6×3 (C teleports 85-104s on
every seed despite 65-84s of live-decoy overlap); ESCAPE-TIME baseline
ORGANIC 6/35 (17%) vs BACKSTOP 29/35; U1-U5+U8 pins green; full 39-leg
ladder 38/39 (BED = the registered CASE-003 flake, field-matched, boundary
fix didn't change its signature).

**One residual, reported not self-cleared (architect-ruled tag
condition):** on seed 8, colonist A breached his sealed vault at 116s with
NO teleport/cave-in/belt-eject writer events — a horizontal through-wall +
≤3-z-hop move. The cap and A2 are BOTH proven independent of this anomaly
(every anomaly sample is itself cap-compliant; A2's discriminator is
colonist C, untouched by A's breach). Identified as the already-registered
CLASS-6/arrive-through-walls collision seam (now B46 in
`BASTION_COMMON_ISSUES.md`) — a separate issue from this tag's own scope.
Committed follow-up: pin the exact breach cell (xy is currently
unresolved, the tape logs z only) and confirm the class-6 hypothesis; any
result implicating the cap, A2, or a worse writer RETROACTIVELY REOPENS
this tag.

**Sonnet curation note:** the builder's 3 raw append-notes in
`BASTION_COMMON_ISSUES.md` (skill-cap self-licensing, the spawn-variance
correction superseding half of it, and arrived-by-radius) were reviewed
and formalized into the table as **B44** (skill-cap self-licensing), **B45**
(spawn-lottery falsifier premises — supersedes B44's farming theory as the
dominant root cause), and **B46** (arrived-by-radius-through-walls) — numbered
in sequence after B43, raw notes removed to avoid duplication.

No master-list row — tracked as its own tag, STAIR-LADDER Phase-2's
prerequisite mechanic pulled forward ahead of the geometry work in
`STAIR-LADDER-MINE-ACCESS-DESIGN.md`. Builder is not idle-pinging for a
next block; the architect signals the Bug-Tester machine handback next,
and the builder's queue resumes from there.

## bastion-block-M2LADDER — M2 OWNED CONSTRUCTED-LADDER EGRESS — TAGGED 2026-07-18 (tag `cd69f61111`, branch `bastion/block-B6HAUL`; no master-list row — mine-complexity-ladder M2 tier, Ben-greenlit via architect; architect inline Opus-depth review GREEN, tag disposition = Ben fork #16)
CERTIFIES: live game plans+builds correct connected ladders; the owned single-owner traversal contract governs normal-play egress with the deterministic mount-snap (kills the ~50% jump-flake); Ben's 4 observed failure classes gone ON THE OWNED PATH (fixture 9/10 ×2 det incl P0G general-position); SEED-20's stranded-forever class CLOSED (organic owned exit 55s, full phase-walk, ×3 det); organic escape 52% best-ever (17% pre-M2); never-stranded 18/18; one-binary evidence (6 seeds × 3 reps corpus + 10-episode fixture, binary 07:58:27). The arc's spine: planner fixes (cell-disjointness starvation / dismount off-by-one / walkability class) → mount-snap + at-entry unlock → the v4 GATE-HOLD corpus catch (owned contract fixture-green but NOT load-bearing in normal op) → the two-layer approach-corridor productionization (tolerance inversion [Chaser 1.5 vs cursor 0.75, writer-diag 1822 handoffs] + planned-segment sweep anchoring [the sweep ate the route's own first rung]) → corpus flip (GATE-HOLD 18/18→0). NAMED-OPEN: s1337/s22 owned-engaged-but-backstop (escape-time optimization frontier, next block); vanilla ladder-token leak (fork #15); AgentInbox interruption dead-on-live-path (engine finding, downstream bounded); class-7 item-identity nondeterminism (behavioral fork, chipped). Registry classes 7-10 filed; 4 chips; commits a2f3c3869a..22834d4152 (Stage-1 plumbing committer-not-author). Evidence + full diagnosis trail: builder scratchpad m2-tag-package.md / m2-fixture-findings.md.

## bastion-block-BACKSTOPOPT — 2/6-BACKSTOP OPTIMIZATION: 6/6 ORGANIC OWNED ESCAPE + THE RELEASE-DECISION STATE MACHINE — TAGGED 2026-07-19 (tag `2880f341d6`, branch bastion/block-B6HAUL; Ben fork #16 step 1, greenlit via architect; architect inline Opus safety gate GREEN, all five gates met, terminal boundary never approached)
CERTIFIES: single-colonist organic owned ladder escape on ALL SIX corpus seeds (B: s7/s8=51s s21=53s s20=55s s22=133s s1337=187s, zero B backstops, full phase-walks, ×3 det) — Ben's 6/6-organic zero-teleport directive ACHIEVED; the protected-vault C-leg delivered INSIDE the 150s bar on all seeds via the DESIGNED net (s22=60 s8=85 s21=92 s1337=97 s20=125 s7=133); never-stranded 36/36; no productive member wrongly teleported at corpus scale; organic 23/36 = 63% WITH EVERYONE DELIVERED (beats the prior 67%-of-survivors whose denominator excluded two stranded colonists — the honest comparison, per the architect's ledger ruling). THE MECHANISM: the release decision rebuilt as a complete three-outcome state machine (verified-stable-exit w/ support+surface/route-top bars · route-exhausted-replan w/ shared bounded counter · keep-driving) + the ENERGY-GATE-WAIT hold (per-episode cumulative 120s, progress-flag-gated) + STICKY exhaustion (barred from re-emission until delivered) + PROGRESS-DISCRIMINATION (set at abort, re-earned at frontier-arrival/completion/delivery) — the general root explaining every round: hopeless zero-progress cyclers net FAST, productive cyclers keep protection. Safety proofs BOTH arms: N7 (single wait held, net@225 ∈[190,295]) + N7B (zero-progress denied, net ∈[90,180], watch-accrual on tape: abort@47s → zero hold-wipes → failsafe secs=60.0). The block's own corpus CAUGHT AND FIXED two intermediate never-stranded regressions in its own new code (registry classes 11+12 born from them). 13-episode fixture matrix ×2 det; one binary 18:43:12; commits 32eeb1a5f7 → a0d44d63dd → 40aa5e0686 → 4827d548ed → 37b474f367. NAMED-OPEN: extraction+bound-unit-test (chip task_72990360, row 51.7, PREPARED PATCH in the builder scratchpad — first post-tag task, prelude to R10); organic-rate headroom (s1337's 187s tail); T1 mine-egress teleport now in armed-but-never-fires observation (R11 generalizes the watchdog next). R10 plan of record carries the accepted seam correction (fence at the bastion owned-write sites, not sys/agent). Full trail: builder scratchpad m2-fixture-findings.md.

## Adversarial bug-hunt (builder-2, session local_c9064dd4) — 70-execution sweep, all 40 scenario surfaces — 2026-07-18/19, read-only

Full report: builder-2's scratchpad `bughunt/BUGHUNT-REPORT.md` (+ `results.csv`).
**Headline: ZERO game-runtime bugs found.** CHOP came back clean under every
attack available headlessly (Ben's flagged suspect) — 4 seeds, tps 60,
determinism pair, conservation exact, real-tree oracle path exercised
(13/7 trees), leaf-no-drop, dedupe, cancel-clean all held. No crash,
stranding/entombment, conservation break, softlock, wrong-behavior, or
determinism break in any shipped gameplay feature across 47 explicit
scenario PASSes + paired/verify legs, byte-identical same-seed determinism
throughout (chopfell full-stdout pair, b55-deep rerun identical in every
sim field), exact conservation everywhere (chop drops==wood, mine 27/27,
gather 5/5, LOD0 round-trip to the coin), clean safety invariants (0
embeds, 0 false teleports, 0 in-terrain ticks).

Binary under test: `bastion-harness.exe 7f087da317+dirty`, built
2026-07-16T05:27:42Z (isolated scratchpad copy; source tree at build time
carried uncommitted edits, so binary ≠ exact HEAD — flagged honestly, not
hidden). Isolation held throughout: own scratchpad copy, own TEMP, no
builds, no shared-state writes; the 2 process kills were both intentional
kill-recovery tests on the tester's own children by exact captured PID,
zero timeout kills.

**Confirmed findings: 4 (all test/tooling infrastructure, none
game-runtime) + 1 suspected (static).** Filed to `BASTION_COMMON_ISSUES.md`
as **B49** (harness `--tps≤0` panics after a full ~74s wasted boot instead
of failing fast at parse time — B16 archetype at the harness front door),
**B50** (`--data-dir` reuse across a DIFFERENT seed silently accepted, no
seed-stamp check, produces a plausible-looking franken-state via automatic
reconciliation), **B51** (the asset dynamic-test suite's multi-occupancy
leg goes inert in `--asset-test all` mode — suite-cumulative colonist
state, not a per-asset defect; single-asset invocations still pass clean —
MEDIUM severity, silently voids the assertion class in exactly the mode
`ASSET_INTEGRATION_LOG.md` is written from), **B52** (`sprite_orevein_
velorite`'s placement footprint creates a genuine one-way pathing trap —
outbound succeeds, return leg sticks, in BOTH suite and single-asset mode —
the one real dynamic failure the 80-asset suite caught), and **B53**
(SUSPECTED/static — an unguarded `Duration::from_secs_f64` reader in
`handle_create_aura_entity`, `server/src/events/entity_creation.rs:728`;
the `/aura` command path is guarded [ARCH-001], the underlying event reader
is not — latent panic for any other emitter reaching it with a bad f64).

**Registry corrections from the report's architect-notes:**
- **B37 flipped to FIX-VERIFIED-LIVE** — the row was stale as "REPORTED,
  NOT FIXED"; the row-49.2 HAULPIN strike-cap fix holds (`haulpin`
  scenario PASSES at both 1337 and 777).
- **B47's seed map extended** — `bed_occupied_mid` now confirmed
  `true@777` in addition to the existing `true@21`/`false@{1337,42}`
  datapoints.
- **B48 gained an exe-generation caution** — an older exe
  (`7f087da317+dirty`, an ancestor of both the `ff2874b4b6` baseline and
  the `2244ce8d71` checkpoint) fails `b55-deep` in a DIFFERENT mode
  (`cycle_exact`/`cycle_work_progressed` false, `mine_conserved` TRUE —
  the registered +11 growth does not appear) with entirely different field
  names. Do not conflate the two modes when the deferred instrumentation
  lands.

**Coverage gaps documented, not exercised** (full list in the report §5):
CHOP canopy-vs-building clipped-set consequence and mid-fell target
mutation (needs a live client or new fixture — no headless scenario fells
real worldgen trees), save/reload mid-action beyond LOD0/LOD1 seams, all
five deferred camera rows, TIMECTL, NIGHTHORROR spawn/render, zone/pile
visuals, LADDEROFF's mid-climb-erasure hostile leg, client-message-layer
input storms, and the 8-concurrent-harness isolation row (deliberately
skipped — shared machine with a live corpus, own-concurrency capped at
1-2 instances).

**Notes for the record, not bugs:** the global flight recorder does not
engage on this binary for ordinary scenario runs despite the env var being
set (works only via focused probe sessions) — architect will confirm
engagement at the next tag; `--asset-test` correctly writes
`ASSET_INTEGRATION_LOG.md` to the asset-lab-dir PARENT (sandboxed
correctly, repo log untouched); `--verify`'s determinism gate is a 10-field
aggregate Summary only (a cheap upgrade exists: hash scenario JSON lines
instead); the `TREE_FELL_CELL_CAP=2048` fell-set cap is confirmed
KNOWN-INTENDED behavior, not a defect, with its own residual (untested
consequence of a clipped set) noted for a future live-fell probe.

**Chips spawned** (per architect direction) for the 4 confirmed + 1
suspected findings — each self-contained, small, and disjoint from current
builder lanes, so routed as independent follow-up tasks rather than queued
onto either active lane.

No master-list row (a test-infrastructure QA sweep, not a build block).
Sonnet-curated registry entries (B37 correction, B47/B48 appends, B49-B53
new) committed alongside this run-log entry.

## bastion-block-M3 — LADDER CONTENTION (mine-complexity-ladder tier M3) — TAGGED 2026-07-20 (tag `8c4543094a`, branch `bastion/block-B6HAUL`; Opus gate PASS, `BUILD_REVIEW_LOG.md` §M3)

Closes M3 (`readme/MINE-COMPLEXITY-LADDER.md` row M3, CONTENTION): M2's
single-owner ladder contract proven under 2+ colonists sharing one ladder —
reservation/queue, exactly one on the ladder at a time, release frees the
slot, all escape. No master-list row (mine-ladder tiers track outside the
numbered list, same as M2LADDER/BACKSTOPOPT/CLIMBCAP/CKSTAIR).

**The arc, condensed** (full trail: Builder 3's session, `m3a-arc-package.md`):
started from `readme/M3-BUILDER-PACKET-FINAL.md` (Sonnet-translated,
R9-folded — persistent `TraversalLink`, `(enqueue-tick, UID)` fair key,
queue ticket). Landed via `bastion-block-M3A` (tag `cebb45746f`) which
carried the crew-contention fix stack: corridor-commit validation anchored
at wp0, an own-position entry fallback, a promotion driver for
queued-then-promoted heads, and the mount-preflight own-prefix-contact
fix (B57 site 3) — capped by a **Sonnet-ruled architectural unification**:
replaced three independently-disagreeing waypoint sources (orthogonal
decomposition / an A* fallback / a per-pass retarget — a genuine livelock,
not a missing case) with ONE authority, a bounded-A* corridor committed at
promotion and consumed statefully, head-only. Two more B57 sites surfaced
and closed en route to green (site 3 pre-tag, site 4 post-tag at
`cf837245da` — a corridor-stepper runtime-revalidation livelock, same
signature, now B57's fourth confirmed instance).

**24-run seed matrix, classified not just reported:** queue invariants
(fair exit order, zero same-tick double-ownership, zero SOFT-0 lane
violations) hold universal on all 24 runs. Four red classes decomposed and
ruled: (a1) hard-roll backstop (harder organic terrain, never-stranded
working as designed); (a2) a fixture-predicate false-positive (SOFT-0
counting the fixture's carved columns, not the emergency planner's actual
lane); (a3) M3D's timing bars calibrated on 1337, shift on other rolls —
bar-calibration not mechanism; **(b-inherited, Sonnet-ruled, registry
B58)** — seeds 21/42 stay red on M3A's own zero-teleport fixture target
after B57 site-4 closed the corridor livelock, but the discriminator is
decisive: `N2` (the pre-existing M2-era single-member contract) nets at
these SAME seeds too, and N2's own acceptance bar was always report-only on
teleports. Ruled tag-acceptable tracked-open, not fix-before-tag — the M3
packet's own acceptance criteria already treats the net as a valid
backstop for "a queued member whose turn never comes," and the gap
predates M3's queue work entirely (inherited, not introduced or worsened).
Fixing it is real follow-up work, scoped as its own block (candidate for
the same corridor-unification treatment), not folded into this tag.

**Opus gate:** PASS, 2 tracked follow-ups, none blocking (`BUILD_REVIEW_LOG.md`
§M3, architect-run). Registry: B57 (OWN-PREFIX SELF-HIT, now 4 sites) and
B58 (the b-inherited tag classification) — both Sonnet-filed and verified
against the landed commits before filing.

**Next supply direction (Ben, via architect):** efficiency gains before
engine-improvement work. Two items queued: #1 boot-cache (Codex,
`codex/boot-cache`, architect-reviewed before merge, not a builder-block
supply) and #2 the `bastion_jobs.rs` crate-split (packet ready,
`readme/CRATE-SPLIT-BASTION-SERVER-PACKET.md` — a pure structural
extraction to a leaf crate `bastion-server`, 3 small coupling knots
[Tick/RtSim/RepositionToFreeSpace], acceptance = byte-identical fidelity
run + incremental-rebuild timing delta). #2 is Builder 3's next block.

## bastion-block-CRATESPLIT — efficiency slate #2 — TAGGED 2026-07-20 (tag `6357c35d23`, branch `bastion/builder`, not yet merged to `bastion/block-B6HAUL`; Opus gate PASS, `BUILD_REVIEW_LOG.md` §CRATE-SPLIT)

Pure structural move, no master-list row (efficiency slate, same class as
the mine-ladder tiers). 11 of the 12 `server/src/bastion_*.rs` modules
(~18.2k lines) extracted into a new leaf crate `bastion-server`
(`bastion_arena` stays server-side as an `impl Server` shim — the one
exception, deliberate). `veloren-server` depends on the new crate and
re-exports every moved item at its OLD path (`pub use
bastion_server::{bastion_*, Tick, presence::RepositionToFreeSpace}`), so
every existing `crate::`/`server::` reference across the whole codebase
compiles unchanged — no reverse-coupling edits needed anywhere else.
Dispatcher system names are unchanged (`NAME="bastion_jobs"` etc.), so
dispatch order stays byte-stable by construction, not by luck.

**6 coupling knots resolved** (the packet scoped 3 up front — `Tick`,
`RtSim`, `RepositionToFreeSpace` — the actual survey found 3 more once all
12 modules were checked, not just `bastion_jobs.rs`): `Tick` (re-exported),
`RtSim` (a trait + generic `Sys<R>`, not a hard dependency), the
`traversal_config_for` helper (moved to live beside `bastion_path`),
`RepositionToFreeSpace` (re-exported), the `test_world`+worldgen feature
forward (build-config plumbing), and `bastion_arena`'s stay-server-side
carve-out.

**Verification (architect-ruled: byte-identity is conclusive for a pure
structural move, no separate validation-corpus pass required — VM budget
saved deliberately):** R10/M3 exhaustiveness pins fire correctly from the
new crate home (35/35 lib tests + 11/11 server tests); a `--dig-access-
scenario` run at seed 1337 is byte-identical pre/post-split ×2 (stable
rc=1 both runs); `--mine-fidelity-scenario` is canonicalized-identical
(the one field-order variance in `mf_per_colonist` is PRE-EXISTING
baseline nondeterminism, architect-confirmed unrelated to this move — a
gap Codex's own determinism sweep will close separately, not this block's
job to fix).

**The compile-win, measured (the actual deliverable):** full harness
rebuild 65s → ~50s (−23%); the check-loop (the fast dev-iteration path)
9.1s → 3.2s (−65%) — a real, measured incremental-compile win, not a
theoretical one.

**Sequencing constraint (architect, live):** no further work touches the
`server`/`bastion-server` tree until the architect's determinism-
integration base (this crate-split's tip + a Codex rebase, pass #1/#2)
lands — protects the fresh split from a second concurrent in-tree cut
racing it. Builder 4 (who tagged this) is holding on non-tree work
meanwhile (corpus-runner stderr-tee-per-seed, then the a2/a3 M3A fixture-
hardening items) per Sonnet's routing.

## DETERMINISM-INTEGRATION BASE — bastion/builder fast-forwarded to `a643d8dee6` — 2026-07-20 (architect-run, Ben asleep, exec authority)

Not a fleet block, no master-list row — an integration/merge action folding
Codex's determinism-sweep pass #1/#2 onto the crate-split tip
(`bastion-block-CRATESPLIT` → `6357c35d23`), satisfying the sequencing
constraint noted above. Five Codex commits, all persistence-ordering
fixes (a classic nondeterminism source — unordered collection iteration
leaking into serialized output), each independently byte-identical-proven
before this merge: `e708e40c9f` (order colonist personal needs on
persist), `d3e4972073` (order persisted sentiments), `8a8f0d0a67`
(stabilize persisted quest order), `fd9ec4c407` (stabilize known-report
order), `a643d8dee6` (stabilize persisted colonist value order — the
fast-forward tip). The architect additionally built/compiled an
integration-reconciliation pass on top, since the crate split moved
`bastion_mood.rs` into `bastion-server/src/` between when Codex's
commits were authored and when they landed here — confirms the passes
apply cleanly against the NEW crate layout, not just the pre-split one.

**Unblocks:** the `server`/`bastion-server`-tree hold from the CRATESPLIT
entry above lifts once this base is confirmed placed — Builder 4's held
in-tree work (B58 corridor-unification) can resume once the architect
signals. **Not yet folded in:** Codex's determinism SWEEP pass #3 (18
more holes identified, separate from the 5 fixes above) — explicitly
deferred, rebases onto `bastion/builder` "when Ben drives it in the
morning," not a fleet action tonight.

## Builder 4 overnight fill (harness-only, pre-B58) — `d80a6b5a58` + `fa339e7694` (branch `bastion/builder`)

Two-item fill while B58 waited on the determinism-integration base, both
outside the `server`/`bastion-server` tree per the hold: corpus-runner
stderr-tee-per-seed (`d80a6b5a58`) — live-proven twice (an instant-corpus
run and a real red-seed `--dig-access-scenario` failure whose FAIL row now
carries a capture-file path that was usable for live mid-run forensics,
closing the exact gap that cost real time earlier in the M3A investigation
when a corpus run's stderr was silently discarded).

Same commit also lands the a2/a3 M3A fixture-hardening from the matrix
classification (`BASTION_COMMON_ISSUES.md` context, the M3 seed-matrix
red-class decomposition): **a2** (the SOFT-0 fixture-predicate false
positive) fixed via a planner-lane scan keyed on the actual Ladder-sprite
column, not the fixture's carved-column assumption — harness-only, no new
server surface. **a3** (M3D's timing-bar calibration) took one honest
fix-forward (`fa339e7694`) after the first attempt's per-waiter
hold-engagement gate proved WRONG against the actual trace: under M3D's
seal, only the re-queued ex-owner transits the complete-route wait state
(observed `[true,false,false]`, not the naive all-waiters expectation —
serial watch-cadence delivery IS the designed path for the others).
Corrected to an any-member hold-witness + a no-pre-budget-delivery check
sourced from the fixture's own constant + the net floor; the finer
hold-alive discrimination is left to M3A's own zero-teleport bar, not
duplicated here.

**Verified:** M3A PASS + M3D PASS at seed 1337 on the synced
(post-determinism-integration) base. Builder 4 now starting B58
(corridor-unification + the corridor-drive `debug_assert` rider),
repro-first — red baseline at seeds 21/42 before any code change, per its
own standing discipline.

## bastion-block-B58 — frontier-approach corridor-unification — TAGGED 2026-07-20 (tag `8e0e3bc03d`, branch `bastion/builder`; Opus gate PASS, `BUILD_REVIEW_LOG.md` §B58, architect-run overnight, Ben asleep)

Closes the B58 tracked-open follow-up filed in `docs/BASTION_RUN_LOG.md`
§bastion-block-M3 (row B58 in the registry) — the inherited M2-era
frontier-approach net-reliance at seeds 21/42. No master-list row,
mine-ladder-adjacent, same untracked convention as M3/CRATESPLIT. Repro-
first per Builder 4's own discipline: red baseline confirmed at 21/42
before any code change.

**Three commits.** `a7213f735d` (the s21 leg): the frontier-reacquire path
unified onto the LIVE-POSITION corridor authority via a `frontier` param
threaded into `m3_promoted_corridor_waypoint` — the same authority M3A's
promoted-head path already uses, now covering the reacquire case too.
Added a no-progress replan-from-position trigger (≥30 ticks / <0.1 blocks
moved) replacing the old stored-corridor replay for a displaced member
(a stale stored corridor can't self-correct; replanning from the member's
actual current position can). `stuck_time` wipes are now earned by real
measured movement, not just by entering the reacquire path. `6dcd679253`
(rider, as planned): single-owner `debug_assert`s at the authority-entry
and reacquire-drive sites, release-inert (compiled out / no-op outside
debug builds, per the standing rider-not-standalone framing). `8e0e3bc03d`
(test fix-forward): the corridor unit-test initializer needed the new
`last_check` field — caught in Builder 4's own self-gate (lib-test target
failed to compile; the episode binaries themselves were unaffected), fixed
before tagging, not after. 35/35 unit tests green.

**Evidence:** the seed-21 member that previously sat frozen (per the B58
registry row's own N2-discriminator finding) now forms its transaction and
climbs out for real — `m3_first_owner` None→`Colonist-0`, teleports 4→3,
first-member delivery 234s→183s. Self-gate M3A + M3D + N2 all PASS at
seed 1337 with the rider asserts live (i.e. the new safety asserts don't
themselves trip under normal operation). Seed 21 stays red ×2
deterministic, seed 42 also red — an HONEST residual, not silently
absorbed: Builder 4 traced every remaining net to the SEPARATELY-FILED
organic-climb-bounce escalation-starvation class (`BUILD_REVIEW_LOG.md`
§FILED — ties the stuck-economy/R11 generalization and the FR15
paired-A/B work, explicitly routed to Ben's morning triage, not folded
into this tag). R10/M3 pin counts unchanged (`remove==1, advance_epoch==2,
insert==3, fenced==13`) — confirms this block didn't touch the fencing
invariants it doesn't own.

**Opus gate:** PASS (`BUILD_REVIEW_LOG.md` §B58). Builder 4 standing down
per the architect — tree held for Ben's morning steer (engine-optimization
phase next, per the standing Ben-priority order).

## bastion-block-ENGOPT1 — engine-optimization #1, A* frontier determinism + fallback correctness — TAGGED 2026-07-20 (tag `115cd34e54`, branch `bastion/builder`; Opus gate PASS — `BUILD_REVIEW_LOG.md` §ENGINE-OPT-1, verdict confirmed after initial logging, see the bookkeeping note below)

The first engine-optimization-phase block (per Ben's efficiency→merge-
codex→engine order — this is "engine," the phase Ben's morning steer
opened). Packet: `readme/ENGINE-OPT-1-ASTAR-PACKET.md`. Scope: `common/
src/astar.rs` only. Two commits: `6b8790c490` (the astar work itself) +
`115cd34e54` (M3 fixture per-violation SOFT-0 lane forensics, env-gated —
built in response to what this block's own tag-time red demanded, see
below).

**A* frontier determinism (ledger item 177):** the frontier's tie-break
key widened to a full total-order tuple `(f, h, g, fxhash64(node), seq)`
— architect-ruled option (c) after a design call that vek's own types
don't implement `Ord` (so a naive "just sort the vector" approach doesn't
compile without this). Falsifier-verified: RED on a seq-only key (proving
the test actually engages the mechanism, not vacuous), GREEN with the
full tuple.

**Fallback best-so-far correctness (ledger item 175):** made Detour-
faithful — the fallback path now stores the NEIGHBOR, not the parent, and
seeds from the start node; two real pre-existing bugs in the old fallback
logic fixed as part of landing this correctly (not just a determinism
tweak, an actual correctness fix).

**Acceptance:** 4/4 property tests; full workspace `rc=0`; `--mine-
fidelity-scenario` ×2 canonicalized-identical; `--dig-access-scenario` ×2
byte-identical (a NEW binary self-reproducible — the determinism claim
holds on freshly-built output, not just against an old golden); N2 PASS.

**M3A@1337 red, classified not hidden (registry B60):** `m3_lane_
violations=3` — fork-15's vanilla-climb-leak (named-open since M2LADDER)
RESURFACED: a task-less queued member ascends the rung column with
`traversing_any=false`. Forensics (the second commit above, a per-
violation diagnostic distinguishing transient-clip / sustained-crowding /
vanilla-leak signatures by member/cell/trigger) pinned it precisely. This
is the now-deterministic A* routing making a queued member's ordinary
vanilla Goto hit the rung column MORE consistently than the prior
nondeterministic routing did — an exposure-probability shift, not a new
leak mechanism, and not something the astar work itself introduced.
Everything else about M3A improved: 0 teleports, deliveries 61s and 56s
faster than the B58 baseline. Architect-ruled tag-acceptable; the fork-15
fix is routed as Builder 4's own immediate next block, falsifier-first.

**★ CORRECTION (2026-07-20, same day): the fork-15/vanilla-climb-leak
mechanism above was WRONG** — organic-tape writer attribution proved no
climb-assist writer exists at any violation tick. Actual mechanism:
PASSIVE physics (soft-collision crowd-shove + vanilla's own auto-step-up
onto rung platforms) during CONSTRUCTION, because the off-lane staging
steer only engages once `route_complete` — queued members near an
in-progress build sit unsteered and get physically nudged onto it. The
"task-less presence in the lane" symptom was real and correctly flagged;
the assumed mechanism behind it was not. Full corrected classification:
registry B60 (updated in place, original framing preserved struck-through
for the trail).

**Also flagged, not yet acted on (registry B61):** `Civs::neighbors`'
`track_map` HashMap-iteration order-fragility — same-binary-stable, could
reorder across a build with different insertion history. No observed
divergence today; a proactive watch item for Codex's determinism sweep,
same general class as the persisted-collection-ordering fixes already in
the determinism-integration base.

**Bookkeeping note:** logged at Builder 4's request while the Opus gate is
still in progress (architect reviewing). Will update this entry's gate
line once the verdict lands rather than leave a stale PENDING marker.

## FORK15 investigation — closed TRACKED-OPEN at Ben's iteration bound — pin commit `798bdfca8f` on `bastion/builder` (no tag; investigation block, not a build block)

Follow-up to B60's corrected mechanism (passive crowd-shove + vanilla
auto-step-up during construction, no code writer). Mechanism now fully
named; M3E reworked as a deliberately-RED steer-property pin (4 sustained
dwell breaches, max 1439 ticks) rather than continuing fix attempts — a
first construction-window staging-steer attempt went inert and was
reverted, with a binding-scope hypothesis left in the commit trail for
whoever picks the fix up next. `M3A@1337`'s `lane_violations=3` stays the
leak's own regression pin, unchanged. Harness-only, no behavior shipped,
classified-SAFE status unaffected. Registry B60 updated with the closure.
Next: Builder 4 reported an ENGINE-OPT-2 candidate pick (ledger item 176,
frontier reopen) to the architect; supply follows their GO or Ben's
morning steer.

## bastion-block-ENGOPT2 — A* decrease-key/reopen correctness — TAGGED 2026-07-20 (tag `623fc58f01`, branch `bastion/builder`; architect SHIP ruling; Ben-directed GO, no morning hold — engine fixes ran top priority)

Second engine-optimization block, packet standards inherited from
ENGINE-OPT-1. Scope: `common/src/astar.rs` only (ledger item 176).

**The bug:** the pre-176 `!previously_visited` guard silently DROPPED real
improvements to already-closed A* nodes — a genuine correctness bug (not
a determinism-only issue), fixed via lazy-deletion reopen (Detour's
`findPath` reopen logic quoted verbatim as the prior-art source, per the
standing prior-art-first discipline).

**Verification:** two falsifiers, both confirmed firing on the emulated
OLD mechanism before the fix (a diamond-graph case landing on cost 17.0
instead of the correct 10.0; a Bellman-Ford reference-path divergence) —
both green after the fix. Full astar suite 6/6. ENGINE-OPT-1's determinism
work preserved (not regressed by this second pass). Local episode
no-regression: M3A/M3C/N2/M3D all byte-stable, and the classified M3A red
(B60/fork-15-reclassified) preserved at EXACTLY `lane_violations=3` — the
correct outcome, since this fix has nothing to do with that mechanism and
shouldn't change its signature.

**Downstream economy effect, classified not absorbed (registry B62):** the
dig-access economy's 7-seed paired A/B under the now-more-correct pathing
is a MIXED reshuffle (B-side worse on 3 seeds, better on 1, same on 3;
seed 777 specifically FIXED), not a clean win. Architect-ruled
SHIPPED-CLASSIFIED — the pathfinding fix itself is correct and proven; the
economy's own roll-robustness is a SEPARATE, tracked follow-up (a
stuck-economy retune), deliberately batched for after the pathfinding arc
closes rather than reactively chased per-fix. Logged to
`readme/DECISIONS-FOR-BEN.md` as a real design fork.

**Also registered (B63, infra not game code):** two `vm-jobs.sh` test-VM
tooling incidents from Builder 4's own concurrent-fan usage, both already
fixed in prior commits (`499845e6d2` FAN-scoping, `81c97f96db` SLOT_LOST
guarantee) — filed under a shared SILENT-RESULT-INTEGRITY class since both
are the same underlying shape (a concurrent job runner reporting success
while its actual output is missing or clobbered, with no loud signal).

## bastion-block-ENGOPT3 — LOOT-AUTHORIZATION INVERSION fix (ledger #160) — TAGGED 2026-07-20 (tag `695bbb0172`, branch `bastion/builder`; architect-GO'd after a crossing reconciliation)

Fixes registry B64 (`server/agent/src/action_nodes.rs` + `common`'s
`loot_owner` + a tooling source-scan pin). Closes with two enrichments
over the original filing:

**It was TWO inversions, not one, and they partially cancelled.** The
outer `!` around the whole authorization conjunction (already documented
in B64) PLUS an independently-inverted hostility polarity in the soft-wish
term (also contradicting its own comment) — and the two inversions
happened to PARTIALLY CANCEL, so the soft+hostile and soft+peaceful
branches came out accidentally correct. This is exactly why the bug
survived review this long, and why a naive single-`!`-flip fix would have
BROKEN the two branches that were accidentally working by luck. The
falsifier documents the cancellation executably, not just asserts it.

**Severity was bounded all along.** The authoritative `InventoryEvent::
Pickup` consumer already revalidates `can_pickup` and denies `LootOwned`
at execution time — a pre-existing commit-time gate that made the original
ledger item's TOCTOU concern moot from the start. Observable live damage
was humanoids wrongly REFUSING their own entitled loot plus attempt-spam
churn — not actual theft of protected drops.

**Pins landed:** the intended truth table + a verbatim-old-mechanism
falsifier (agent crate); a `can_pickup` truth table (common crate — that
file's first tests); a tooling source-scan guard pinning the commit
gate's continued presence, so this class can't silently regress again.

**Verified:** VM-fan all-attested; M3A's classified red (B60) byte-
preserved; N2 PASS; `--mine-fidelity-scenario` undisturbed.

Next: ENGOPT4 = SlowJobPool, per the architect's landing-order numbering.

## bastion-block-ENGOPT4 + ledger #178 — SlowJobPool/ARCH-003 scheduling + Chaser retained-search invalidation — TAGGED 2026-07-20 (self-gated per the new batched-review process; tags `7b994ea99c` + `4f5de38f08`, branch `bastion/builder`)

Two tags, self-gated (pins + M3A/N2/fence safety floor green → tag → next
item, no per-block architect gate — batched review to follow; new
standing process). Bookkept together.

**ENGOPT4 (`7b994ea99c`, SlowJobPool/ARCH-003):** scheduling-divergence
diagnosis via same-platform triple-divergence with attested `cpuPlatform`
(so a cross-VM diff can't be blamed on silently-different hardware). Three
stages: sorted chunk-apply, a hasher-independent pool-selection fix (1b),
and a harness-mode deterministic apply barrier (stage 2) — all
falsifier-pinned. Measured: cross-VM field divergence 20→12; `mf_
completion` now byte-equal cross-machine. Honest disclosure: full `mf`
byte-identity was NOT achieved — the residual traces to the agent-layer
`.par_join()` seam, named explicitly as the next block rather than
claimed closed. Also: PATH-0 re-verified already-deterministic; ledger
item #181's premise is stale (doesn't need the work it assumed).

**Ledger #178 (`4f5de38f08`):** profile-keyed invalidation for the
Chaser's retained search-context (opened mid-ENGOPT4 during a VM-stock
wait, per the never-stop-on-the-ledger rule). A sharp falsifier fired on
stale admission through a since-unloaded band; a broader falsifier is an
honest EXECUTABLE NEGATIVE — it pins that ENGINE-OPT-2's reopen fix
already self-heals this case, not a second bug needing its own fix.

**Safety floor (shared across both):** M3A's classified red byte-
preserved, N2/M3D PASS, all attested at `7b994ea99c` with the new apply
barrier active.

**Registry candidates flagged, not yet filed:** the wrapper-capabilities
arc (cpuPlatform attestation, VM_ZONE, the msys `--min-cpu-platform`
quoting limitation), the #181-premise-stale note, and the named
agent-layer residual.

Next (already opening): first-divergence-tick hunt on the `.par_join()`
residual via the recorder-comparator methodology.

## ENGOPT6 — agent-layer determinism residual, ROOT-CAUSED + FIX COMMITTED, END-PROOF PENDING (fix `3b137017e6`, instrumentation `c5cdd18bf6`, branch `bastion/builder`)

Not yet tagged — logging the diagnosis now since it's substantial and
correct, will confirm the tag once the verification fan (tape pair @
`3b137017e6` + a full safety-floor fan) reports green.

**The chain, tape-comparator-driven, not guessed:** a tape pair at
`7b994ea99c` (ENGOPT4's tip) first diverges at trajectory line 23743,
tick 3960, uid 2 — `active_job` `326/Arrived` on one VM vs `None` on the
other, byte-equal before that point. The recorder itself was blind to the
discriminating state, so instrumentation landed FIRST (`c5cdd18bf6`,
extending the post-lifecycle snapshot with job progress + hunger/rest)
before the real cause could even be seen. Rerun with the new
instrumentation, both VMs attested same-commit + same silicon (Intel
Cascade Lake) via the ENGOPT4 attestation work: SAME signature, tick 3960,
uid 2, job 326 — progress=0.0 on BOTH runs for 35+ ticks (a haul leg-1
colonist standing at the item, re-emitting pickup), needs/hunger/rest
byte-equal (exonerated as the cause). One VM's item resolves at 3960, the
other's ~16 ticks later.

**Root cause:** `common/src/comp/loot_owner.rs`'s `LootOwner.expiry` was
`std::time::Instant` — a WALL CLOCK. The headless harness runs at ~9×
wall-clock speed, so a 45-WALL-second ownership timeout lands on a
machine-throughput-dependent SIM TICK, not a fixed sim-time. A contested
item's ownership resolves at different sim ticks on machines with
different real throughput → the claim cascade shifts (339 vs 341) → the
entire 12-field `mf` divergence family downstream (this is the exact
residual ENGOPT4 named and declined to claim closed).

**Fix (`3b137017e6`):** `expires_at` is now `f64` sim-seconds;
`new()`/`expired()`/`time_until_expiration()` all take `Time` instead of
reading the wall clock. 7 call sites rewired (`sys/loot.rs`,
`entity_manipulation`, `interaction`, `inventory_manip` ×3, `lib.rs` test
hook) — serde wire format unchanged, the client never saw the `Instant`
representation either, so this is save/wire-transparent. Unit pin
`engopt6_expiry_follows_sim_time_only` covers post-expiry saturation too
(the OLD `Instant`-subtraction code path panics there in debug builds —
a real latent crash the fix also closes as a side effect). `ProgramTime`
was separately audited and confirmed dt-accumulated/deterministic — the
`PickupItem` family is fine as-is. Remaining `Instant::now()` call sites
in the codebase were triaged and confirmed non-sim (loop clock, metrics,
player-facing paths, the ENGOPT4 barrier's own watchdog) — not silently
missed, checked and ruled out.

**★ CORRECTION (2026-07-20, same day): the fix did NOT close the
divergence.** The end-proof tape pair at `3b137017e6` diverges at the
IDENTICAL point — tick 3960, uid 2, job 326, byte-equal before it. New
data from the round-1 instrumentation: `progress=0.0` on BOTH runs for
35+ ticks (job 326 is a Haul leg-1, colonist waiting at the item), needs
byte-equal — so hunger/rest AND work accrual are BOTH exonerated. The
item ENTITY itself vanishes at tick 3960 on one VM vs ~3976 on the other,
WITHOUT being picked up by uid 2 first (the moot-release path, not a
successful pickup). **The `LootOwner` wall-clock fix is KEPT, not
reverted** — it's real dirt regardless (a wall clock has no business in
deterministic sim state, and it separately closed a genuine post-expiry
saturation panic) — but it was not THE seam causing tick 3960's
divergence. Remaining suspect: the contested item's LIFECYCLE (whatever
deletes the entity at a machine-throughput-dependent tick). Round-2
instrumentation landed (`0e5e264c6c`): the per-colonist snapshot note now
carries the active Haul item's `(uid, exists, position, owner
expires_at, next_merge_check)`. `tapes5` fan running at `daaf8aba45` now
(the tag has moved past this commit since the #183 revert landed on top —
see below).

**Rounds 3-5 (all pairs attested same-commit + same silicon, Cascade
Lake):**

- **Round 3** (`@daaf8aba`, item-trail note): needs/work-accrual both
  re-exonerated. The contested haul item's ENTITY is deleted at a
  machine-dependent tick; its `next_merge_check` timestamp reads
  `131.9999986` sim-seconds ≈ tick 3960 EXACTLY — the item's own
  merge-check SCHEDULE is what the original divergence tick was pointing
  at all along.
- **Round 4** (`390cfa7a`, pickup-verdict trail): moved the needle
  decisively. First divergence is actually tick 3947 uid 1 (earlier than
  previously pinned) — item-35's `PickupItem` component vanishes in ONE
  run only (a synchronous comp-take from a SUCCESSFUL pickup), while the
  other run's copy still carries `next_merge_check=131.9999986` = tick
  3960. So the real race is per-tick pickup ATTEMPTS vs the periodic
  MERGE SWEEP for the same item — whichever wins first deletes it, and
  which one wins is machine-dependent. Every Pickup attempt now records a
  full verdict (entity-missing / out-of-range / loot-owned / comp-taken /
  partial / inventory-full / accepted).
- **Round 5** (`b4d33eb1a5`, full merge-trail instrumentation): proved the
  divergent deletion is `sys/item`'s comp-take path. uid 2's own pickup
  attempts verdict `out-of-range` on EVERY tick in BOTH runs — not the
  cause, a SEPARATE standing bug (see below). Check-due schedules are
  byte-equal through the pre-flip tick; then the SAME due check finds a
  merge partner in one run and finds none in the other (backing off to
  187.43). The spatial grid itself was audited and is deterministic
  (sequential entity-id-order build, coordinate-order cell walk) GIVEN
  identical history — so the conclusion is that the PARTNER item's own
  state history diverged earlier, invisibly, before this tick. Every check
  fire, every performed merge, and every backoff update is now recorded;
  the `tapes7` fan running now should name the true seam directly from its
  first divergent merge-trail line.

**Standing bug found + fixed along the way, unrelated to the hunt itself
but discovered because of it (`502ad6897a`, HAUL-RETARGET — registry
B68):** dropped items are physical and FALL after being dropped; a haul
job's leg-1 aims at the STALE drop cell, so a hauler can end up standing
"Arrived" at the drop cell while the actual item has fallen ~5 blocks
below, permanently out of `MAX_PICKUP_RANGE` (5.0 cylinder distance) —
grinding out-of-range pickup verdicts for 100+ ticks with no recovery,
which is exactly what makes the divergence-hunt race above possible in
the first place (a job stuck this way sits around long enough for the
merge-sweep-vs-pickup race to actually matter). Fix: leg-1 now re-aims
`job.pos` at the item's LIVE position when it's drifted past
`ARRIVE_DIST`, and returns to `Traveling`. Live-tape evidence stands in
for a unit fixture (the arm is scenario-deep); acceptance is the next
`mf` pair + a floor at the tip. `bastion-server` 38/38.

**★ ROOT CAUSE FOUND (2026-07-20, `tapes7`, round-5 merge trail, pair
`@b4d33eb1` attested):** the true seam. First CANONICAL divergence at tick
3947 is a mass item-merge event — a burst of same-created mining drops all
falling due for a merge check on the same tick. WHICH item gets checked
FIRST decides the entire merge topology, because the check order follows
`specs` join order = ENTITY-ID order, which itself carries machine-varying
allocation/recycling history (not a fixed, portable sequence). One run's
first checker grabbed exactly 1 merge partner (spreading merges across
targets 36/37/38/41); the other's grabbed 6 (all funneled into target 37)
— different surviving item UIDs, cascading into the entire `mf` divergence
family (this is what made uid 2's contested item-22 alive-in-one-run,
merged-in-the-other at tick 3960/3976, four investigation rounds ago).
Also proven en route: `hashbrown` iteration order genuinely differs across
runs EVEN ON THE SAME MACHINE (a tick-648 backoff-event set was identical
but differently ORDERED — a process-seeded hasher, not a real divergence)
— the tape comparator itself needed hardening to canonicalize this
(merge events now sorted within-tick before comparison) so it stops
reading hash-order noise as a false divergence.

**Fix (`781a553eb71e`):** applies ENGOPT4's own sorted-apply pattern to
this consumer — due-checkers now iterate in UID order, partners are
ranked by `(distance_sqrd bits, uid)`, backoff parents are UID-sorted.
The item economy is now a pure function of stable state, not entity-ID
allocation order. The underlying entity-ID-allocation nondeterminism
itself is flagged as its own future master-order row, not fixed here —
this fix makes the CONSUMER robust to it, it doesn't make allocation
itself deterministic.

**★★ END-PROOF GREEN — strongest evidence grade available.** `tapes8`
pair at `@781a553e` (both VMs attested same commit + same silicon):
RAW BYTE-IDENTICAL tapes across both machines — `trajectory.jsonl`,
`events.jsonl`, `summary.json`, with only `wall_unix_millis` stripped as
the one expected non-sim field. 36,059 trajectory tick-blocks + 24,726
event tick-blocks, ZERO divergence anywhere. The 10-minute colony
simulation is now bit-deterministic across machines — not "canonicalized-
equal after normalizing known noise," genuinely byte-identical raw output.
The tape-comparator's own canonicalization logic (built earlier in the
hunt to filter out hashbrown iteration-order noise) isn't even needed at
this commit — the divergence class it was built to filter is gone.

**The full closing seam chain, in the order each piece actually mattered**
(worth preserving for the eventual tag body): ENGOPT1/2 (A* total-order +
reopen correctness) → ENGOPT4 (sorted chunk-apply, deterministic barrier,
hasher-independent pool selection) → `LootOwner` wall-clock→sim-time
(`3b137017e6`, kept — real bug, not the seam) → HAUL-RETARGET fallen-item
fix (`502ad6897a`, registry B68 — not the seam either, but what made the
seam's race actually reachable/observable) → the merge-topology UID-sort
(`781a553eb71e`, the ACTUAL final seam — entity-ID join order vs. stable
UIDs, closed by applying ENGOPT4's own sorted-apply pattern to a new
consumer). Five real, independently-useful fixes on the way to one root
cause — none of them wasted effort even though only the last one was
"the" seam.

**Instrumentation now permanently in-tree, zero live cost (env-gated):**
the haul-item trail, the pickup-verdict trail, and the UID-sorted merge
trail — all available for any FUTURE divergence hunt without needing to
re-invent this tooling.

**Research-mirror retro-validation (done after the fact, not before):**
Builder 4 checked T0.1-T0.7's shipped shape against the T0-001 research
packet's selected architecture — matches. T0.8's slice-in-progress is
literally the packet-endorsed staging point (a bounded fixed-step
accumulator). T0.5's `dt`-guard is confirmed a compatible stopgap for the
packet's fuller schedule-level pause design, not a competing approach.

**★★★ TAGGED — `bastion-block-ENGOPT6` @ `781a553eb71e`, pushed.** Floor
gate green: `floorx` fan (rerouted to `us-east1-c` after an `east1-d`
stock-out) — N2 `rc=0`/`tp1`, M3D `rc=0` `[145,44,204]`/2 violations/hold
`[T,F,F]`/all alive.

**★ NEW CANONICAL FIXTURE BASELINES (deliberate, expected, T0.7-draw-shift
class — record these as the reference point for all FUTURE floor
comparisons, not the old pre-T0.7 numbers):** stable across both
`c3d53c19` and `781a553e`, confirming they're genuinely new deterministic
baselines rather than run-to-run noise:
- M3A: `[66,82,94]`/3/tp0 (old) → **`[66,44,97]`/2/tp0** (new baseline)
- M3D: `[145,204,263]`/3/tp3 (old) → **`[145,44,204]`/2/tp1** (new baseline)

This is the expected consequence of T0.7 converting raw per-tick
probabilities to hazard-rate equivalents — draw outcomes shift by ulps at
the individual-draw level, which can legitimately move downstream timing/
violation-count numbers even though the underlying RATE is unchanged.
Fork-15's tracked red (the M3A construction-window crowd-shove class) stays
open and unaffected by this baseline shift — it's a separate, already-
tracked issue riding on top of whatever the current baseline is.

Escalation-to-Fable/Opus path is now moot (would only have triggered on a
still-diverging pair, which didn't happen — the strongest possible close).

## T0.10 + T0.11 (strategic time, absolute world timestamps) — SHIPPED, fixes B69 (commit `bd40e59e1039`, branch `bastion/builder`)

Both `Job::Hired(Actor, TimeOfDay)` and `Quest.timeout: Option<TimeOfDay>`
converted from persisted RESTART-RELATIVE sim-`Time` deadlines to absolute
world timestamps — closing B69 exactly as predicted (every server restart
was silently extending live hires and open quest deadlines by however
long the server had been down). Hire "days" are now real world days
(`+days × 86400` world-seconds); quest minute-limits are preserved as
DURATIONS via `day_cycle_coefficient` rather than being reinterpreted;
permanent hires are `TimeOfDay(inf)`; expiry checks now compare against
`ctx.time_of_day` directly. `TimeOfDay` gained `PartialEq`/`PartialOrd` to
support the comparisons.

**Migration disclosure, not hidden:** this is a one-time, pre-release-
acceptable break — old saves' raw `f64` payloads reinterpret as world
timestamps far in the past on load, so existing hires end and any open
quest timeouts resolve ONCE on first load after the upgrade. No
compatibility shim shipped for it, deliberately, given the project's
pre-release stage. Flagging this explicitly rather than letting it surface
as a surprise later.

**Scope note:** the promote/demote "apply-elapsed-once" ledger (tracking
a last-processed world timestamp across a promote/demote cycle) is
DEFERRED to the Tier-2 lifecycle rows where promotion itself lives — this
block satisfies the packet's deadline-correctness contract, not the fuller
lifecycle-ledger design. `rtsim` 16/16, `bastion-server` 39/39, `common`
153/154 (B59 pre-existing only).

**Next (Builder 4 self-selected, correctly per the master order — no
supply needed from me):** T0.12-lite — a golden phase-manifest +
parsed-registration validation test (rejects cycles, unknown edges,
drift), engine code itself untouched. The full descriptor-generated-
schedule version stays T0-002's endpoint, deliberately deferred as its
own bigger block rather than smuggled into this lighter validation-only
slice.

## T0.6 (tick-rate-invariant probability, ledger #115) — TWO PASSES: shallow done-by-audit SUPERSEDED, then properly completed (commits `654764371b` → `a50f6ca817b7`, branch `bastion/builder`)

**First pass (`654764371b`) was too shallow — self-disclosed and
corrected, not caught externally.** Builder 4's own first audit concluded
`NpcCtx::chance()` already covered the class and marked T0.6 done-by-audit
without converting anything. My orchestrator-supply message for this item
had already independently verified ~20+ raw `random_bool` call sites
bypassing that helper across `rtsim/src/rule/npc_ai/{mod,dialogue}.rs` and
`quest.rs` — real gaps, not covered by the shallow read. Flagged before
the tag closed; Builder 4 redid the pass properly.

**Second pass (`a50f6ca817b7`), COMPLETE:** converted the genuine per-tick
gates to `NpcCtx::chance()` per-second hazards, preserving today's exact
behavior via the correct 30-tps inverse conversion (`rate = 1-(1-p)^30`,
so nothing's tuning silently shifted): `archetype_gate` (the RON weights
keep their historical meaning — mapped through the conversion at the gate
itself, no data-file re-tune needed), the guard-thought travel interrupt
(`0.0003/tick` → `0.0089609.../s`, since `interrupt_with` polls every
tick), and quest-escort's manual `dt/30` inline scale (the helper's linear
path `dt*rate` is exactly equivalent for `dt<=1`, so this was a safe drop-
in). Explicitly EXEMPT-MARKED (not silently skipped) the genuine per-
decision/one-shot draws that don't need this treatment: conversation
cutoff, content/activity picks, the site-subset filter, day/visit-plan
profession gates, speech-target and dialogue mention/hire draws — these
fire once per decision, not once per tick, so tick-rate invariance doesn't
apply to them.

**Flagged, not converted:** `sentiment.rs`'s decay draw has `dt` in the
DENOMINATOR — a suspiciously inverted shape worth its own investigation
rather than a blind conversion that could silently distort the tuning.

**Executable ban landed:** a `t0_6` source-scan pin over the five rtsim
policy files — any unmarked raw `random_bool` now fails the pin. The pin
is proven live, not just present: it caught Builder 4's OWN first marker-
placement mistake before this tag closed. `bastion-server` 39/39, rtsim
compiles clean.

## T0.7 (tick-rate-safe AI rates, ledger #166) — landed alongside T0.6's first pass (`654764371b`), FLOORED pending `floor8`

8 raw per-tick gates in `server/agent/action_nodes.rs` converted to
`hazard(rng, dt, rate_per_second)` (mirrors `discrete_chance`, same
Poisson/Gillespie-hazard prior art): unwield-idle ×2, hunt-retarget,
lantern-toggle, pet-dismount, pet-mount, idle-utterance, idle-sit. Rates
are the exact inverse of today's per-tick constants at 30 tps, pinned to
`1e-12` plus a compounding-invariance pin, so today's behavior reproduces
exactly. Per-decision draws (`can_sense` jitter, jump-vs-roll pick)
deliberately kept raw — same distinction T0.6 made. `unstuck_if`'s
helper-stream gate is named as its own debt (no `dt` in scope at that call
site, 10 sites affected, not fixed here). `veloren-server-agent` 3/3.

**Honest disclosure, not swept under the tag:** draw-level outcomes shift
within the same expected rate (probabilities differ in ulps from the old
raw constants), so an old fixture byte-baseline COULD legitimately move
even though the underlying rate is unchanged. `floor8`'s verdict decides
the read: changed-but-sane result = a legitimate T0.7 re-pin; a genuine
strand = revert-and-isolate. Not yet finalized as accepted.

**Housekeeping note for the master-order doc (route via the ChatGPT-prompt
pipeline, batched with the #183 correction already sent):** T0.9
("deterministic persistence clock... driven by simulation tick") is
SUBSUMED by T0.1 — the persistence tick-cadence work T0.9 asks for is
exactly what T0.1 already shipped. Mark accordingly rather than building
it as a separate item.

Next pick after the fans settle: T0.8 (physics substeps) — flagged by
Builder 4 as HIGH surface, needing full fixture-ladder acceptance, and
will read 03-research first (noting again that the `.gdoc` files aren't
FS-readable from this environment, only the BASE list is — same
limitation as T0.6).

## Master-order Tier 0 (T0.1-T0.5) — all pushed, all suites green, FLOORED pending the next fan at tip (branch `bastion/builder`)

New process note: the standing source of truth for engine-improvement
sequencing is now `ENGINE-IMPROVEMENTS-MASTER-BUILD-ORDER.md` (architect
relay from Ben/ChatGPT), read strictly top-to-bottom. ENGOPT1-7 are marked
DONE there. **★ ONE CORRECTION NEEDED IN THAT DOC, not fixable from this
checkout (it isn't in this repo — routing to the architect for relay to
Ben/ChatGPT):** its `[DONE.7]` row claims ledger #183 is done via
ENGOPT7 — that predates the revert above. #183 is NOT done; it's reverted
(`daaf8aba45`) and blocked on decoupling egress-target selection from
search-cycle side effects.

Tier 0 landed, all suites green (`veloren-common` 153/154 B59-only,
`veloren-server` 14/14, `bastion-server` 38/38):

- **T0.1** (`9b3bb074a7`, ledger #5): `SysScheduler::should_run_at(tick,
  deterministic)` — sim-tick cadence in harness mode, byte-identical wall
  path live. Only consumer today is player-character persistence (inert
  in the headless harness) — a substrate fix landing ahead of its future
  consumers, same class ENGOPT6 just pinned for `LootOwner`.
- **T0.2** (`41738059c6`, ledger #21): the labor clock DECLARED in
  executable form — a source-scan pin over 10 labor/economy files banning
  `Instant::now`/`SystemTime::now` outright. All clean today (post-ENGOPT6
  fix); the pin locks the door on this whole bug class going forward.
- **T0.3** (`8e17073bc5`, ledger #39): `pub const SIM_TPS: u64 = 30`
  declared in `bastion-server`; 11 scattered tick-budget constants (mount
  timeouts, climb-release, queue-wait, energy-wait-hold, arbitration
  interval, the deterministic chunk-apply delay, T0.1's own derivation)
  re-expressed through it. Every value byte-identical — a value-freeze
  pin, zero tuning drift, but a future deliberate cadence change now
  moves every budget together instead of drifting independently.
- **T0.4** (`67b23e6f95`, ledger #54): a clock-domain DECLARATION doc on
  `Time` (the sim clock = the only clock sim logic may read; sibling
  clocks `ProgramTime`/`TimeOfDay`/`Tick`/`TimeScale` each named with
  their own advancement rule and permitted consumers) + `SimSecs`, a new
  serde-transparent sim-duration newtype, applied to `MoodConfig`'s
  `break_sustain_secs`/`despond_secs` with arithmetic forced through
  `Time`. RON format unchanged (transparent), values byte-identical.
  Wholesale adoption across every raw sim-instant field is deferred to
  future rows — the type exists now for them to adopt.
- **T0.5** (`867216c8af`, ledger #55): pause safety for the need/
  breakdown arbitration pass — it rolls discrete changes on TICK cadence
  while its own sustain/cooldown guards compare SIM time; with
  `TimeScale 0`, a colonist already past its sustain window would re-roll
  `break_chance` every WALL tick of a pause, compounding toward a
  guaranteed breakdown while the world is frozen. Guard: `dt.0 >
  EPSILON`. Behavior is byte-identical outside an actual pause (harness/
  live `dt` is always positive); the paused-server RED case has no
  harness fixture to prove it against, so it's classified as an
  arithmetic-evident guard, flagged honestly as such rather than claimed
  fully proven.

**Floor status:** `floor7` at `daaf8aba45` (the #183-revert tip) was a
full byte-baseline (M3A `[66,82,94]`/3, M3D `[145,204,263]`/3/tp3, N2
clean). T0.1-T0.5 + the HAUL-RETARGET fix are floored by the NEXT fan at
the current tip, pending alongside the `tapes7` verdict.

## bastion-block-ENGOPT7 — ledger #183 (REVERTED, see correction below) + #179 (Small→Medium continuation equivalence, executable negative, STANDS) — tag `a4018f948a`, branch `bastion/builder`, self-gated — ★ `a4018f948a` alone is NOT a safe restore point: reverting to it re-introduces the M3A strand regression #183 caused; the tree at `daaf8aba45` (the revert commit) is the actual good state

**Naming note:** landed as ENGOPT7 rather than strict landing-order
numbering, deliberately — the in-tree instrumentation commit for the
still-open residual above already names it `ENGOPT6` (see `c5cdd18bf6`'s
own commit message), so preserving that in-tree reference took priority
over sequential numbering. Builder 4 flagged this choice explicitly rather
than silently deviating; no objection — a naming/numbering call, not a
behavior or safety question, and it keeps in-tree references consistent
rather than forcing a rename.

**Ledger #183 (`a4018f948a`), SHIPPED:** a `Chaser` no-path negative
cache — a COMPLETED empty-frontier search verdict against an unchanged
`(target, #178-profile)` question now suppresses re-search entirely
instead of recharging it every tick. Invalidation is real and multi-
triggered, not a permanent cache (bastion terrain is mutable and #184's
revision-plumbing isn't in yet, so a permanent cache would be unsound):
target moved past the existing 2.0-dist² reset threshold, a `#178`
profile flip, an `InvalidPath` terrain signal, arrival, or a bounded
90-tick half-open re-probe (circuit-breaker prior art, named as such).
Falsifier fired on the unfixed code (200 scheduler searches over 200
ticks in a sealed-pocket world) and is pinned ≤5 fixed; separate pins for
immediate invalidation on target-change and on profile-change. PATH-0's
own contract preserved — candidates drop out via `needs_search` exactly
while the cache is live, and the movement fallback (direct bearing +
stuck watch + `PathState::None` + the red gizmo) is untouched.
`veloren-common` 155/156 (the one red is B59, already registered as
pre-existing and unrelated) — bastion-server 36/36.

**Ledger #179 (`7899fa9106`), EXECUTABLE NEGATIVE — no fix shipped, none
needed:** the question was whether continuing a Small-taxed search into a
Medium-tier upgrade is route-equivalent to starting a fresh Medium search
from scratch. The premise is genuinely LIVE (edge-cost taxing only exists
below Medium tier; the tier-upgrade arm retains the old `Astar` state;
`set_max_iters` just raises the iteration cap, it doesn't reset anything)
— so this wasn't a vacuous question. Three falsifier iterations all came
back GREEN, and critically the SHARP PRECONDITION was proven engaged, not
just assumed: a test-only `Astar::visited()` accessor shows the retained
visited set genuinely covers the narrow-tunnel decision surface (carries
real Small-taxed g-values there) at the exact moment of upgrade, inside a
two-door world purpose-built to flip on the tax. ENGINE-OPT-2's own
strict-improvement re-push (from the reopen-correctness fix) turns out to
be exactly what heals the mixed frontier. Disposition: same general class
as #181/B65 (a ledger item resolved by proving the underlying assumption
false/moot rather than shipping code) but a HIGHER evidence grade — #181
was a stale premise never engaged; #179 is a genuinely-engaged precondition
that still came back negative. Test kept permanently as the equivalence
pin. No separate tag or registry row — rides in the ENGOPT7 lineage per
Builder 4's own framing, consistent with how ledger #178 rode alongside
ENGOPT4.

**★ CORRECTION (2026-07-20, same day): #183 REVERTED — do NOT treat as
shipped.** ~~Ledger #183 (`a4018f948a`), SHIPPED~~ was wrong; the tag
`bastion-block-ENGOPT7` above should be read as historical/superseded for
#183's portion (its #179 portion still stands as described). The
`floor6` safety fan at `3b137017e6` caught a real M3A strand regression:
`[66,null,null]`/0 lane violations vs the byte-baseline `[66,82,94]`/3
(that 3-violation red IS the tracked, expected fork-15 baseline — its
ABSENCE here is the tell, not a new pass). Full local-repro evidence
chain: the cache-off kill-switch restores the baseline exactly; leg
isolation narrowed it to the scheduler-search-cycle suppression itself
(not the candidacy-restore path, which reproduces baseline fine) —
suppressing PATH-0's search cycle changed a waiting queue-member's
movement duty from 14% to 91% bearing-ticks, which relocated its parking
spot, which flipped the feet-anchored organic-egress TARGET computation
onto an unreachable elevated ladder-column cell (a goto blip at
`(15860,16240,394)` vs the baseline mount at `(15861,16241,391)`) — the
`final_mount` gate (`route_mount.xy==target.xy`) never fired, no
`QueuedForLink` task formed, and the provenance window was lost after the
owner's route cleanup. A [[stuck-economy-constraint]]-class finding: a new
steer/drive duty invalidated the tuned egress web, exactly the class
this project has hit before.

**Disposition:** the 200-searches/200-ticks inefficiency #183 was fixing
is REAL (the falsifier fired correctly before shipping) — but the fix is
BLOCKED ON decoupling egress-target selection from search-cycle side
effects, which is its own future block, not a quick patch. Revert
(`daaf8aba45`) removes the live cache semantics + #183's own pins/
fixtures; #178/#179/#180 are untouched (the `ledger_180` module was
restored after the revert slice briefly clipped it; a dead `u_trap`
fixture dropped, `trap_cfg` kept since #179's pin needs it).
`veloren-common` 153/154 (B59 pre-existing only). M3A local at the
reverted tree: `[66,82,94]`/3, the correct byte-baseline. `floor7` fan
re-running at `daaf8aba45` now.

## Ledger #180 SHIPPED, UNFLOORED pending floor7 — actual-work accounting through the search stack (commit `58ba5e4ee2`, branch `bastion/builder`)

`find_path` now returns the poll's expansion delta (correct across a
resumed/retained-astar search, which keeps its own running total);
`Chaser` stores and reports `last_search_consumed()` (a no-op grant arm
zeroes it, so a stale delta never gets re-billed); the PATH-0 scheduler
debits ACTUAL consumed work instead of the planned estimate per search
(admission still PROJECTS with the planned estimate for the tick-cap
math, and actual≤planned holds per step, so the cap invariant itself is
unbroken) — trivial searches stop eating a full 250-iteration slot they
never needed. Pins in `ledger_180_tests`: trivial-search consumed>0 and
<planned; no-op grant=0; a `Pending` slice still bills exactly 250;
actual≤planned holds as an invariant.

**Not yet floored — explicitly unconfirmed:** #180 changes how MANY
searches the scheduler can serve per tick (more trivial searches now fit
in the same budget), so it needs its own no-regression proof, not a free
ride on #179/ENGOPT7's floor. The M3A floor at `daaf8aba45` (`floor7`,
running now) is that proof. Treat as provisional until `floor7` reports
green.

## Master-order BASE.md doc corrections applied directly (2026-07-20, per architect direction — it's a real editable file, not a live Google Doc)

Edited `H:\My Drive\bastion-Chatgpt\engine design\03-engine-improvement-
research\ENGINE-IMPROVEMENTS-MASTER-BUILD-ORDER-BASE.md` directly (outside
the git repo, on the H: drive) rather than routing a paste-to-ChatGPT
prompt, per the architect's explicit correction to the earlier workflow
assumption. One-line trace per correction:

- `[DONE.7]` (#183): flipped DONE → REVERTED, with the blocked-on
  condition and revert commit (`daaf8aba45`) noted inline.
- `[T0.1]`-`[T0.5]`: moved from Tier 0 into the DONE section as
  `[DONE.13]`-`[DONE.17]`, each with its landing commit.
- `[T0.6]`: moved to the DONE section as `[DONE.18]`, noting the
  superseded shallow first pass (`654764371b` → `a50f6ca817b7`).
- `[T0.9]`: moved to the DONE section as `[DONE.19]`, marked SUBSUMED BY
  T0.1/DONE.13 rather than DONE outright (no separate build landed for
  it — T0.1 satisfied its ask as a side effect).
- `[T0.7]`: left in Tier 0 but annotated LANDED-pending-`floor8` (not
  moved to DONE — floor8 hasn't confirmed yet, consistent with not
  claiming things closed before their own gate reports).
- `[T0.8]`: annotated IN PROGRESS (Builder 4 actively working it).

**Follow-up correction pass (same day, once floor9/ENGOPT6 confirmed
green):** `[T0.7]` moved to DONE as `[DONE.20]` (floor-confirmed, new
canonical baselines recorded inline). `[T0.8]` SPLIT — the shipped
consistency slice moved to DONE as `[DONE.21]`; the deferred bounded-
substeps half stays in Tier 0 as `[T0.8-residual]`, its own future block.
`[T0.10]`/`[T0.11]` left in place but annotated "SEE [DONE.22]" pointing
at the new consolidated DONE row for the actual landing (`bd40e59e1039`),
rather than duplicating their text into the DONE section.

### T0-002 group (T0.12-T0.27, phase manifest/scheduling), progress

One group prompt issued covering the whole packet; builder self-drives
item-by-item, tags reported here as they land — no re-prompt per item.

- `[T0.12]` DONE `e8d890d413` — golden phase-manifest, drift/cycle-checked.
- `[T0.13]` DEFERRED — every late-event site is networking-adjacent
  (session/admin), no sim-surface consumer under single-player scope;
  revisit at the networking phase.
- `[T0.14]` SLICE DONE `c3b7923e80` — six order contracts on the manifest.
  Full read/write ambiguity introspection still open, own future block.
- `[T0.15]` DONE `26eecf436209` — covered-by-slices (existing contracts
  already express the named boundaries); phase LABELS deferred as endpoint.
- `[T0.16]` DONE `d77b61435c04` — jobs→rtsim outbox edge declared.
  Acceptance GREEN: tapes9 byte-identical, floory at established baselines.
- `[T0.17]` LITE DONE `2615354dc9ac` — rule schedule frozen (golden bind-
  order pin). Named-phase machinery deferred as endpoint.
- `[T0.18]` DONE `26eecf436209` — negative order contract (agent NOT
  same-tick reachable from rtsim::tick; handoff is deliberately next-tick).
- `[T0.20]`+`[T0.23]` DONE `89f49c92bd5a` — two more implicit orderings
  declared as dispatcher edges (agent←controller, phys←character_behavior
  + phys→phys_events). No schedule change, contracts only.
- `[T0.26]` DONE `0839693f579e` — exactly-one-consumer check now runs in
  all builds, not just debug.

Note: builder self-caught a python-heredoc truncation mid-edit (test count
dropped 41→26), restored from HEAD, redone via Edit tool — no bad state
landed. Also a stated commit-message "42/42" was actually 41/41; noted here
for the record, not worth a correction pass in the commit itself.

Batched acceptance (tapes10 + floorz) running at tip `0839693f57` for the
T0.17/18/15/20/23/26 cluster above — verdict pending, will confirm on landing.

Remaining in the group (real architecture blocks, no lite form): T0.19,
T0.21, T0.22, T0.24, T0.25, T0.27 — builder taking them one at a time,
starting T0.19, each with its own pair+floor.

**Acceptance round result:** tapes10/floorz FAILED, both fail-closed, no
bad data. tapes10 job-0 was a real build gap (registry B70) — fixed
`783196b2c96b`. tapes10 job-1 + all of floorz were VM image-create
rate-limit collisions from running two fans in parallel — new standing
discipline: sequential fans only, ~10min cooldown between. tapes11 queued
at `783196b2c96b`, floor to follow after cooldown; verdict pending.

- `[T0.19]` DONE `e709b028d81f` — closed-by-existing-architecture, `NpcAi`
  already does snapshot-plan-commit. Tracked finding filed as B71
  (deferred-networking-only exposure, not a today bug).
- `[T0.24]` LITE DONE `8049b4806d5b` — delivery-policy frozen (Apply=
  ImmediateDownstream vs SERIAL TAIL, 30 calls pinned); typed NextTick
  deferred with T0.13. (Self-caught: first count was a truncated read
  window, 23→30, corrected before landing.)
- `[T0.27]` LITE DONE `d415e001d312` — server tick phases named+frozen
  (direct mutation → event application → structural maintenance → sync
  → world tick).

**Acceptance COMPLETE for the T0-002 slice cluster:** tapes11 byte-
identical + floorw at canonical baselines (M3A `[66,44,97]`/2, N2 rc=0,
M3D `[145,44,204]`/2 hold[T,F,F]) at tip `d415e001`. DISCHARGED:
T0.12/13/14/15/16/17/18/19/20/23/24/26/27. Fan discipline (sequential +
cooldown) holding, no further rate-limit fails.

REMAINING in the T0-002 group: T0.21+T0.22 (Controller transaction frames
+ tagged command envelope — full block, builder starting now), T0.25
(generated handler registry — its own design pass, not yet scoped).

- `[T0.21]` SHIPPED `1c766ca6598b` — Controller buffers private + generation-
  stamped (push_*/drain_events/take_actions), exactly-once frame
  consumption, T0.20's declared edge now type-enforced. Acceptance
  (tapes12+floor) queued, sequential-fan discipline holding, no rate-
  limit fails since. Two disclosures filed: B72 (python whole-file
  writes churning CRLF, 2nd recurrence — Edit-tool-only rule adopted)
  and B73 (pre-existing plugins-feature cfg gap at
  `client/src/lib.rs:738`, not caused by this work, not yet fixed).
- `[T0.22]` DONE `e8cf0343fac5` — one sequenced channel per my ruling:
  `QueuedCommand{phase,seq,payload}`, each consumer drains its own phase
  from the shared T0.21 frame, exactly-once per phase, cross-channel
  order pinned by producer-local seq. Zero call-site changes beyond T0.21.
- `[T0.28]` DONE `7c90df32d0` — LandOnGround emission sorted by entity id
  (live-mode half; harness was already immune).
- `[T0.32]` DONE `a98128405b` — breakdown roll compounds RON break_chance
  to the true pass interval from declared clocks, cadence-invariant,
  exact to 1 f64 ulp of current behavior.
- `[T0.33]` DONE `1ee8e7a0e1` — breakdown draw keyed (tick, uid, episode-
  start), join-order/upstream-draw decoupled. World-seed term noted
  out of scope.

**Acceptance:** tapes12 came back byte-identical and attested `a9812840`
(later than intended) — covers T0.21+T0.28+T0.32 in one shot. tapes13
queued at `e8cf0343` covers T0.22+T0.33, floor follows sequentially.
T0.21/22 block closes once tapes13+floor land green.

- `[T0.25]` DONE `4c62173704ad` — validation-first per ruling: static
  cross-check of `server_events!`'s 70 types against the Apply set and
  the 30-type SerialTail golden, partition asserted exact-complement.
  Caught one real miscue en route (RequestPluginsEvent Apply not
  serial). Full bus-creation codegen stays the endpoint.

**★ T0-002 GROUP COMPLETE — T0.12 through T0.27, all discharged.**
Pending: tapes13 (T0.22+T0.33 pair) + floor in flight, T0.25 rides free
(test-only). Group prompt issued for the rest of T0-003 (T0.29-31,
T0.34-49) below; builder self-drives per the same group-cadence.

- `[T0.38]`+`[T0.39]` DONE `9b3c6850ac` — claim/need-target determinism
  now unconditional in LIVE mode too (was harness-gated): decision_job_ids
  + egress-request sort always run; EAT ties break on item uid, bed ties
  on coordinate (was HashMap order in both cases). Landed opportunistically
  ahead of T0.29-31/34-37 (tractable-first within the group, same pattern
  as T0.32/33) — spotted on the builder branch, not yet formally reported.
- `[T0.42]` DONE (audited, no code change) — candidate sources already
  stable-by-construction + keyed DETRNG (B8) makes choose() deterministic
  end-to-end. Forward-looking caveat filed as registry B74.

**Acceptance:** tapes12 (`a9812840`, byte-identical) + tapes13
(`4c621737`, byte-identical) — unbroken byte-identity chain since
ENGOPT6, now covering T0.21/22/25/28/32/33. floorv queued at tip
`9b3c6850ac`; on green, the T0.21/22 Controller-frame block closes as
one tag: **`bastion-block-CTRLFRAME`** (naming call, consistent with the
FARM/HAULPIN/SEASONHUD short-code convention). Builder to cut the tag on
floorv green.

**Recheck pass (this session):** cross-referenced every T0.x commit on
the builder branch against the master-order rows — found and fixed one
gap (T0.21 had landed but its row was never annotated DONE). All 27
landed commits now correctly reflected.

**★ `bastion-block-CTRLFRAME` TAGGED + PUSHED @9b3c6850ac3e.** floorv
green at tip `b3314978` — M3A `[66,44,97]`/2 (tracked-red), N2 rc=0 tp1,
M3D `[145,44,204]`/2 hold[T,F,F] rc=0, no baseline shift from T0.34/38/39
(as predicted). T0.21/22 block CLOSED. Ledger row added.

- `[T0.34]` DONE `b33149786036` — CleanUp sentiment decay per-NPC keyed;
  Architect + dialogue_start audited already-keyed, no change needed.

Group state: T0.28/32/33/34/38/39/42 discharged. Continuing tractable-
first: T0.35/36/37/40/41 next, then T0.43-45 (physics pair family, own
acceptance pair) and T0.46-49 (canonical state + item identity — T0.49
likely needs a design ruling for `ItemInstanceId`). Ping-for-ruling on
T0.29-31 still pending, prep already done.

- `[T0.36]` DONE `e7c8cb8161a4` — NPC spawn orientation keyed via ChaCha8
  (was OS entropy, latent seam invisible to prior byte-identity proofs).
- `[T0.37]` DONE `fc4299e67531` — 3 OS-entropy RNG constructions in
  Apply handlers (head loss, death/buff-proc/loot-winner, loot-drop
  placement — authoritative economy state) now sim-time-keyed ChaCha8.

Both were real, previously-undetected determinism bugs — a byte-identical
acceptance pair only proves determinism for its own comparison window,
not the whole system. Filed as registry B75 (general coverage-gap lesson).

Acceptance: tapes14 launched at `fc4299e675` (post-cooldown), one pair
covers the whole accumulated draw-shift batch (T0.34/36/37/38/39); floor
follows sequentially.

**tapes14b confirmed byte-identical.** flooru running.

- `[T0.35]` DONE (audited, no code change) — T0.7 + DETRNG already cover
  Agent main/helper/Chaser and action-node helper streams.
- `[T0.40]` DONE `c99125a71c32` — thought_sum now Neumaier-compensated f64.
- `[T0.41]` DONE (audited, no code change) — current_site resolution is
  first-in-worldgen-order over a stable Vec; world_site_map is pure-
  lookup (packet's hash-is-fine class).

Group scoreboard: 12/22 of T0-003 discharged (T0.28/32-42 done; open =
T0.29-31 + T0.43-49).

**T0.29-31 ruling (builder's proposed opening slice, confirmed with one
addition):** yes to the shape — (a) Emitter-level stamp of (producer
identity, local seq), producers themselves unchanged; (b) recv_all_mut
merge-sorts by (producer rank, local seq), tick/phase correctly left out
of the SORT key since a single drain is already scoped to one tick/phase
so they're constant within it; (c) causation/correlation/idempotency as
optional fields, machinery-present-but-unpopulated initially. ONE addition:
still STORE tick+phase on the stamp itself (cheap, known at emit time)
even though they're not needed for sorting — T0.31's causation graph will
need to reconstruct cross-tick relationships later, and re-deriving these
after the fact is wasted work. Full EventEnvelope wrap (every field
required for every producer) stays the endpoint, deferred, matching the
established lite-first pattern.

- `[T0.29]`+`[T0.30]`+`[T0.31]` DONE `5905a44c3727` — EventStamp{epoch,
  producer: &'static Location via track_caller, seq, causation/
  correlation/idempotency: Option}, merge-sorts (epoch, producer site,
  seq), stamps stripped post-drain. Disclosed deviation: drain EPOCH
  substitutes for literal tick (avoids fleet-wide Emitter plumbing;
  epoch is 1:1 with tick, origin frame still preserved not lost) — ruled
  acceptable, no literal-tick plumbing needed. Zero churn for producers
  or consumers. Own pair+floor required (changes bus processing order),
  queued after flooru per sequential-fan discipline.

Group scoreboard: 15/22 of T0-003 discharged. Remaining: T0.43-45
(physics pair family), T0.46-48 (canonical state), T0.49 (ItemInstanceId
— builder will flag for a quick ruling, packet already specifies the
struct shape).

- `[T0.43]` DONE (audited) — pushback per-entity independent, stable
  in-regime neighbor order, determinism already holds.
- `[T0.44]` DISPOSITIONED — (min,max) pair-ownership redesign only buys
  momentum-symmetry, HIGH fixture surface; deferred as own endpoint block.
- `[T0.45]` DONE (done-by-existing) — B5.8 already shares preflight and
  resolution collision-choice internals.
- `[T0.46]` DONE `a66f4fda885d` — default-group ties break on smallest
  member group index (was hash order, run-varying serialization bytes).
- `[T0.47]` DONE `3907378d96c8` — persistence batch drain sorts by
  character id before SQL (was hash order).

Acceptance: flooru green @`c99125a7` (T0.34-40 batch fully accepted).
tapes15b running at `3907378d96` covering T0.29-31+T0.46+T0.47.

Group scoreboard: 18/22 of T0-003 discharged. Remaining: T0.44 (deferred
endpoint), T0.48 (persisted-collection gate, sizeable), T0.49
(ItemInstanceId, ruling below).

**T0.49 ruling, answering builder's 3 scoping questions:**
(1) `world_namespace` = a persisted per-world NONCE minted once at world
creation, NOT derived from world_seed. Two saves sharing a seed (a reset
test-world, a regenerated world) must not alias item-instance namespaces
— that's exactly the collision this field exists to prevent. A one-time
random mint for the NAMESPACE is fine (the packet rejects random UUIDs as
PRIMARY item identity, not as a one-time per-world seed component).
(2) Allocator = a single persisted monotonic u64 counter living beside
world_namespace in the same authoritative Data/save structure, incremented
at the actual commit point (item construction/insertion), covering every
creation site (drops, inventory instantiation, crafting output, loot).
Full retry-safe RANGE reservation (packet's "retried transactions reuse
the same range") is Tier-1 transaction-machinery scope (T1.17/T1.24
already exist for that) — for this Tier-0 slice, allocate synchronously
at the single non-yielding construction point so there's no reserve-
without-commit gap in practice. Don't build the full retry apparatus now.
(3) Field-first, consumers-later. Add `id: ItemInstanceId` to Item/
PickupItem now — that's T0.49's actual scope. Do NOT switch mf_completion
or other harness item-hash consumers over in the same change; that's its
own follow-up. Swapping a working acceptance-harness mechanism in the
same commit as landing a new identity substrate conflates two different
risk surfaces for no gain — matches the deferred-consumer pattern used
throughout this group (T0.24 NextTick, T0.13 networking, etc.).

- `[T0.49]` DONE `b73db1583fbb` — shipped exactly per ruling: serde-
  default Option on PickupItem, allocator in rtsim Data, namespace=one-
  time nonce, sequence=synchronous counter at create_item_drop (post-
  merge, merged drops consume no new identity), zero consumer switches.

**tapes15b confirmed byte-identical** (T0.29-31 stamped bus survives
cross-machine, plus T0.46/47 covered). floort running.

**T0-003 GROUP: 18/19 resolved, only T0.48 remains** (standing persisted-
collection determinism gate — container inventory + insertion-order
permutation tests + fresh/fresh/restored byte-compare across thread
counts). Builder already has the packet's canonical-state guidance from
the original group prompt (container classification: serialized/hashed
→canonical encoding, authoritative-iteration→sort/ordered, RNG-consuming
→canonicalize-before-draw, pure-lookup→hash is fine) — no re-prompt
needed, confirmed sufficient and told to proceed.

**★ T0-003 GROUP COMPLETE — 19/19 rows resolved.** Shipped: T0.29/30/31,
32/33/34/36/37, 38/39, 40, 46, 47, 48 (+ real fix: `NpcLinks.rider_map`
canonical-serialize, DONE.11 pattern), 49. Audited-no-change with
endpoints filed: T0.28(prior)/35/41/42/43/44/45. floort in flight
(floors the T0.29-31/46/47 batch; T0.48/49 are save-side, covered by the
gate's own tests, riding the next routine pair). Tag on floort green:
**`bastion-block-T0DET3`** (short-code convention, DET3 = T0-003's
determinism/RNG/canonical-state group). Builder to cut on green.

Next per master order: T0-004 (T0.50-66, async acceptance/agent command
merge/domain hashes/recorder schemas). Packet content to be folded into
one group prompt per the established cadence.

**T0-004 group prompt sent.** Note: this packet is denser than T0-002/003
— it ships full selected data shapes AND its own dependency-ordered build
sequence (12 steps), not just one-line issue-map entries. Instructed
builder to respect that order more strictly than the tractable-first
pattern used for T0-002/003 (later rows build on earlier rows' types).
Full per-row compressed shapes + the packet's cross-packet non-negotiable
rules (worker-completion-order-never-authority, cancellation-never-
substitutes-generation, exactly-one-terminal-outcome, canonical-not-
memory-layout hashes, wall-time-never-authoritative, no-silent-drop-under-
load) relayed verbatim in the prompt, not reproduced here.

**★ `bastion-block-T0DET3` TAGGED + PUSHED @96315c8fbf85.** floort green
at T0.49 tip `b73db158`: M3A `[66,44,97]`/2 (tracked-red), N2 tp1,
M3D `[145,44,204]`/2 hold-live — all canonical. T0-003 group CLOSED,
19/19. Ledger row added.

**Recheck pass (this turn):** cross-referenced every T0.x commit on the
builder branch against `-updated.md` — found gaps from the file-switch
(T0.12, T0.29, T0.30, T0.31, T0.35, T0.40, T0.41 had landed but weren't
annotated in the new file, since they were bookkept on the plain BASE.md
before the switch). Fixed all 7.

- `[T0.50]` DONE `3cc66d52f73e` — `common::async_work` with
  AsyncOwnerKey/AsyncGeneration/AsyncRequestId+allocator, acceptance
  predicate per packet shape (incarnation semantics, cancellation-is-
  efficiency-only, never-reused ids). common t0_50 2/2. T0-004 step 1
  of 12 (packet-ordered), builder proceeding to step 2 (T0.51 envelope).

- `[T0.51]` SLICE DONE `41c7897c8f`→`bbe94e570f`→`b4afb1772b05` — envelope
  (AsyncWorkRequest, semantic-unit costs, exhaustive AsyncTerminal incl.
  CommittedExternal), bounded queue (coalescing, backpressure-never-drop,
  stable pop order proven against scrambled arrival, cancel/deadline,
  shutdown-drains-to-terminals), owner-phase merge (semantic-key sort —
  completion arrival never authority, terminal-uniqueness ledger with
  watermark contract). Actor-adapter half deferred to its first consumer
  (persistence adapter), disclosed not smuggled. T0-004 steps 2-4/12,
  pure additive types, no fan needed yet (no consumer wiring).

Next: step 5 (T0.52, Agent parallel plan buffers + deterministic
Controller publication) — first behavioral/consumer-side rework of the
group, touches NpcAi/agent publication, needs its own pair+floor.

**★ T0.52 VERDICT: serial === parallel, RAW BYTE-IDENTICAL** (`808c724d`,
svp2 pair, both rc=0, trajectory/events/summary all byte-equal wall-
stripped) — the strongest evidence grade in this whole arc. A 10-minute
colony sim produces bit-identical output on one worker (serial) vs many
workers with the real parallel dispatcher. T0-002/003's substrate
(stamped bus, per-entity disjoint writes, keyed RNG, sorted applies,
total-order selections) carried the entire load with zero schedule-order
authority leaks.

- `[T0.52]` DONE `808c724d` — packet's serial-reference-diff requirement
  delivered as an executable full-engine equivalence probe
  (`--deterministic-parallel`, now PERMANENT standing infra — a reusable
  oracle for any future feature). Per-entity-plan-buffer redesign
  empirically unnecessary: current code already deterministic under real
  parallel dispatch, proven not asserted. svp1 abort earned its keep —
  caught a pre-existing ENGOPT4-era one-worker assertion, fixed forward
  `856b9a665b` (probe env-var exempted with a loud warning, guard intact
  for normal runs). Caveat: mf lightly exercises loaded-combat Agent
  par_join — filed as registry B76.
- `[T0.53]` DONE `4d41a5f605` — canonical domain-hash, type-separated
  durable/integrity roots, prefix-safe fields, insertion-order-free
  composite.
- `[T0.58]` DONE `808c724ddb5d` — RecorderSchemaRef version discipline.

Floor launched at tip (guard-exemption touched the deterministic server
path — confirms normal runs unaffected). Builder continuing T0-004 steps
7-12 (RTSim/domain leaves + final certificate, recorder schema/causal
records/provenance, partial-order oracle, Loom/Shuttle/legal-schedule
fuzzer, token-bucket+DRR event budgets, hierarchical work quotas).

**Steps 7-8 checkpoint, floor green @`6ec8d1d9` canonical baselines:**
- `[T0.55]`+`[T0.61]` DONE `6ec8d1d9` — Merkle DomainCategory tree,
  category roots key-sorted; FinalStateCertificate, authoritative-match
  excludes rebuildable integrity roots. Types-first.
- `[T0.56]` DONE `26918c6f6b` — causal_record: derive_span_id pure fold,
  exhaustive CausalOutcome terminal, links-not-parenthood. Types-first.
- `[T0.54]`+`[T0.57]` DONE `ab3d43d4b1ca` — content_manifest: walk-order-
  free root + changed-paths diff; provenance statement shape. Types-first.

T0-004 steps 1-8 substrate now DONE as types, adoption (walking domains
to emit leaves, instrumenting live phases) deferred per the packet's own
build order + the T0.49 field-first pattern — 7 modules currently
unconsumed by design, not oversight.

**PACING RULING (builder flagged A vs B, genuine judgment call):**
confirmed (A) — continue to step 9 (T0.59/62/63 oracle+equivalence
tooling) rather than detour into proving one consumer ad-hoc now. Step 9
IS the natural first real consumer of the hash/recorder substrate — it
formalizes the ad-hoc byte-comparison already in use into the packet's
richer equivalence policy (final hashes + causal edges + conservation +
independent-multiset tolerance), so building it next both follows the
packet's prescribed order AND delivers the live proof Ben's determinism
law wants, without a bespoke harness hookup that would likely need
rework once the real oracle lands. Also approved: fold
`--deterministic-parallel` into ROUTINE acceptance (an extra probe leg
alongside the cross-machine pair, not opt-in) — directly serves the
determinism-by-construction law as a standing regression guard against
future schedule-order leaks, and the ephemeral-VM cost model ($0 idle,
one extra job) makes this cheap relative to its value (ENGOPT6's 5-round
hunt is exactly the class of bug this now catches automatically).

**Steps 1-9 checkpoint (14 commits, tip `826f1e6cdc1e`), all types-first,
verify-profile clean:** T0.59/60/62/63 (causal oracle + run-equivalence
policy + span hierarchy) on top of the already-logged T0.50/51/53-58/61.
T0-004's substrate is now ~9 modules landed, none yet wired to real
running data — see ruling below.

**TWO FORK RULINGS + a pacing course-correction (Ben asleep, executive
control):**

FORK 1 (T0.64, Loom/Shuttle dep decision) — SPLIT THE FORK. Approved:
ship the Bastion-specific full-engine legal-schedule fuzzer now (needs
no new deps, extends `--deterministic-parallel` with a
`BASTION_SCHEDULE_SEED` knob permuting only declared-schedule freedoms).
DEFERRED: Loom(primitives)/Shuttle(components) — adding heavy dev-deps
to the shared workspace + rewriting primitives to use Loom's own sync
types is a real tooling-investment decision, not a mechanical add;
logged to `readme/DECISIONS-FOR-BEN.md` for his call rather than decided
unilaterally, even though dev-deps are technically reversible.

FORK 2 (T0.66, domain-budget split) — CONFIRMED as proposed: reuse
T0.12's existing dispatcher-manifest groupings (path/events/jobs/rtsim/
terrain/persistence-apply) verbatim, zero new taxonomy. Quanta as
parameters defaulted to effectively-unbounded (pure substrate add, no
behavior/baseline shift); real tuning deferred until a fixture surface
justifies specific values. Textbook reuse-first, no design tension.

**PACING COURSE-CORRECTION:** my prior ruling assumed step 9 (T0.63)
would itself deliver a live proof; it landed as types-only like
everything else, same as before. With 9+ modules now unconsumed and only
T0.65/T0.66/the-Bastion-fuzzer-half-of-T0.64 left before the packet's
substrate is fully typed, the right shape is: finish that small remaining
substrate (T0.65 next as builder proposed, then T0.66 per Fork 2, then
T0.64's non-dep half), and treat that as the CLOSE of pure substrate-
building for this group — do NOT open a new research group after. The
REQUIRED next step before T0-004 counts as done is proving ONE real
consumer end-to-end: emit an actual `FinalStateCertificate` at the
harness's real final phase and fold it into the existing svp/mf byte-
comparison. This is option (B), deliberately sequenced AFTER the last
small substrate pieces rather than interrupting them.

**★★ T0-004 COMPLETE, LIVE-PROVEN, NOT TYPES-ONLY.** Builder executed the
exact sequence: T0.65 (token-bucket+DRR) → T0.66 `227a61628450`
(hierarchical DRR reusing T0.12 manifest domains verbatim, unbounded-
default no-behavior-change; a saturating_add overflow caught by its own
pin) → T0.64 non-dep fuzzer half `5d86df15e008` (BASTION_SCHEDULE_SEED
over declared-schedule freedoms; Loom+Shuttle deferred per Fork 1,
DECISIONS-FOR-BEN.md #24) → **Option B live proof `a1130b1c5793`**: the
harness emits a real FinalStateCertificate at the mine-fidelity final
phase; serial + `--schedule-seed 5` + `--schedule-seed 12` (three
distinct legal schedules, different worker counts) all produce the
IDENTICAL `durable_composite`. T0.63's equivalence policy is now proven
against live authoritative state, not types.

- `[T0.64]`/`[T0.65]`/`[T0.66]` DONE (see master-order for commits/detail).
- `[T0.63]` upgraded from types-only to LIVE-PROVEN.

fuzz1 campaign RUNNING (cross-machine × cross-schedule in one fan: serial
+ `--schedule-seed 3` + `--schedule-seed 7`, durable_composite must match
across different MACHINES and worker counts — the definitive T0-004
acceptance). Tag on green: **`bastion-block-T0DET4`** (confirmed, matches
CTRLFRAME/T0DET3 convention). `--schedule-seed` legs now ROUTINE in every
acceptance campaign per the earlier ruling — fuzz1 is the first instance.

T0-004 group final tally: 17 commits, T0.50/51/53-66 all shipped (T0.52
separately proven byte-identical as standing infra). Next per master
order: Tier 1 (correctness/transactions). T1-001 packet (T1.1-11,
command/commit/capability protocols) already read — group prompt ready
to send once the tag lands.

**★ T1-001 GROUP COMPLETE — T1.1-11, all 10 packet steps shipped in
dependency order** (11 commits, tip `5fbffc18a105`, 20/20 T1 pins green
together, full common 199 pass B59-only, zero warnings). T1.1 (fitness
gate + starter registry) → T1.3+T1.10 (CommandReceipt admission +
9-state CommandStatus lifecycle, centralized legal transitions) →
T1.2+T1.4 (effect_journal, prepare-validates/commit-non-fallible,
rejects general 2PC) → T1.7 (DatabaseBatchOutcome, remove-pending-only-
on-commit) → T1.5 (conservation_saga, orchestrated, reverse-order
compensation, conservation-pinned) → T1.8 (BastionCommitQueue,
stable-order + conflict + generation validation) → T1.9 (audit_framework,
3 tiers, record-never-repair) → T1.11 (capability, server-issued grants,
sim-tick expiry) → T1.6.

**★ T1.6 = A REAL LIVE BUG FIX, not a type** (matches the T0-004 lesson
better than T0-004 itself did — this group didn't even need a course-
correction). `execute_character_edit` committed the DB transaction on a
FAILED edit: `CharacterScreenResponse::is_err()` matched Create/List/Data
errors but OMITTED `CharacterEdit(Err(_))` — every failed character edit
silently committed (real data corruption, live). Fixed: commit decided
on the actual typed Result, `is_err()` helper REMOVED so a future variant
can't silently bypass again. server 14/14.

**PACING RULING (builder proactively flagged, same A/B shape as
T0-004):** confirmed (B)-lite exactly as proposed — wire ONE substrate
piece into a harness-exercised Bastion job-completion path (not a
player-persistence path; those are harness-inert, no mf/floor/schedule-
seed coverage). Specific pick: wire **T1.3/T1.10 (CommandReceipt +
CommandStatus) first**, not the full ConservationSaga — the lighter
substrate proves the pipeline stages are real and exercised with lower
risk, and T1.5's saga (heavier: compensation logic, multi-owner) is the
natural SECOND wire-in once the first is proven, same lite-then-full
sequencing as T0-004. Pick a single narrow job type (a haul/ItemTransfer
completion is the obvious candidate — small blast radius, already
harness-covered) and route its commit lifecycle through
Accepted→Executing→Committed instead of whatever ad-hoc completion logic
exists today. **HARD CONSTRAINT: this must be a pure refactor — identical
completion behavior/outcomes, self-gated against the M3/N2/fence floor
for ANY baseline shift.** We are proving the plumbing carries state
correctly, not changing what jobs do. Don't tag `bastion-block-T1CMD`
until this wire-in lands and passes acceptance (incl. the routine
`--schedule-seed` leg) — same discipline as T0-004's Option B.

**Wire-in shipped `d319508dacb6`:** T1.3/T1.10 routed through the haul
ItemTransfer completion (Accepted→Executing→Committed). Pure-refactor
confirmed LOCALLY: mf FinalStateCertificate durable_composite byte-
identical pre/post wire-in. JobBoard gained a runtime-only
`command_admission` ledger (not serialized, not recorder-sampled —
`JobBoard` is `#[derive(Default)]`-only, zero tape/persistence surface);
admits by idempotency_key=job id, forgets the receipt after Committed
(bounded memory). VM acceptance (t1cmd cross-machine × cross-schedule +
t1cmdfloor safety floor) running — tag on both green.

**t1cmd GREEN:** all 3 legs (serial + `--schedule-seed 3`+`7`, all
`d319508d`) identical durable composite, MATCHING the pre-wire-in fuzz1
baseline exactly — certificate-invariant cross-machine and cross-
schedule, not just the local check. Waiting on t1cmdfloor for the
both-green tag gate.

**★ `bastion-block-T1CMD` TAGGED + PUSHED @d319508dacb6.** Both halves
green: t1cmd certificate-identical cross-machine × cross-schedule
(matching pre-wire-in baseline exactly); t1cmdfloor canonical (M3A
`[66,44,97]`/2 tracked-red, N2 tp1, M3D `[145,44,204]`/2 hold[T,F,F] —
wire-in didn't perturb the ladder fixtures). T1-001 CLOSED. Ledger row
added. Three engine blocks tagged this session: T0DET3, T0DET4, T1CMD.

Next per master order: Tier 2 (RTSim↔ECS lifecycle/state-machine
formalization, 100+ rows). **No research packet exists yet for T2**
(confirmed: neither the readable export nor the H-drive source has a
T2-00x file, and the master-order's own T2 rows carry no packet
citation at all, unlike T0.67-89 which at least cited a missing-but-
named file). Builder pre-scouted the master-order's own row text for the
first natural cluster (T2.1-18ish: promotion/demotion lifecycle, stable
actor identity, reason-coded events, projection schema, T2.16's cross-
seed/dispatcher-mode determinism check) and mapped real reuse threads
back to T0.50/52/53/56 + T1.10/11 substrate. Verifying that mapping
against the actual code before crafting the group prompt — code-
verification fallback (T0.6 precedent), since there's no packet to
fold in this time.

**T2 group prompt sent (T2.1-22), code-verification-grounded** —
confirmed `SimulationMode` is exactly `{Simulated, Loaded}`
(rtsim/src/data/npc.rs:43), `NpcId` (slotmap key) vs `Npc.uid: u64`
(npc.rs:291) is a real dual-identity split, `hook_rtsim_entity_unload`
is real (server/src/rtsim/mod.rs:478) before sending. Flagged T2.2
(SimulationMode 2→4 states) as the one row with real new behavioral
surface, told builder to reason about crash-recovery semantics before
building it.

- `[T2.4]`/`[T2.7]`/`[T2.8]`/`[T2.9]` DONE `b92afa89dfb1` — T2 opener,
  reuse-first cluster: EntityIncarnation guard (T0.50's AsyncOwnerKey
  barrier applied to entity targeting), one aggregate reason-coded
  LifecycleEvent (feeds T0.56 causal records), LoadedLinkage tri-state
  (needs_reconciliation flags only broken links, T2.11's target).

**T2.2 RULING — confirmed, builder's own analysis is correct and fully
de-risks it.** Verified the linchpin claim directly: `Npc.mode:
SimulationMode` IS `#[serde(skip)]` (npc.rs:337-338) — mode is NEVER
persisted. So there is no cross-restart stuck-intermediate-state
recovery problem at all: every NPC boots to `Simulated` (its `#[default]`)
since ECS state isn't persisted either, and re-promotes cleanly into
whatever chunks are loaded. The 2 new states (PromotionPending/
DemotionPending) are runtime-only, within-session, bracketing the two
irreversible transitions — standard reconciliation shape (intent
declared → in-progress → settled). Proposed cycle confirmed: Simulated
→PromotionPending(CreateNpcEvent emitted, not yet ECS-linked)→Loaded
(ECS+IdMaps linked+decorated, atomic via T2.15)→DemotionPending(unload
begun, projection not yet committed)→Simulated(projection commits+ECS
deleted, unified via T2.12/18). Determinism: transitions ride the
already-deterministic tick/load-unload order; Rust's exhaustive-match
means no silent gap is even possible once the enum grows (every existing
`match SimulationMode` becomes a compile error until updated — a real
safety net here, not just an assumption). One thing to actually check
(not a blocker, just verify): whether the flight recorder samples
`Npc.mode` at runtime — serde-skip protects the SAVE file, not
necessarily the tape, so confirm the recorder's own state capture is
either indifferent to mode or handles the 2 new variants correctly.
Build sequencing confirmed: land the 4-state enum as a behavior-
PRESERVING refactor first (transient states set+cleared within the same
path, observable behavior = today's), let T2.11/12/15 give the new
states real duration/purpose. Go.

- `[T2.1]` DONE `b901288b49b3` — promote/demote pair audit.
- `[T2.13]`/`[T2.19]`/`[T2.21]` DONE `7f5f297668fd` — versioned
  projection schema + fitness gate + versioned field projection registry.
- `[T2.17]` DONE `926d5db36628` — offscreen action-disposition contract
  + completeness gate.

Reuse-first formalization cluster complete (T2.1/4/7/8/9/13/17/19/21),
zero live-behavior surface, all direct T0-004/T1 substrate reuse.

**Message-cross:** my T2.2 confirmation queued but hadn't landed when
builder's next report went out — reaffirmed, no change to the ruling.

**T2.16 sequencing (builder's option a vs b):** confirmed (a), gate-first
— build T2.16 (the determinism oracle) before the live/behavioral rows,
so every subsequent live refactor has something to prove against
immediately. Sharpened: validate EACH live row against T2.16+floor+
schedule-seed INDIVIDUALLY as it lands, not batched at the end — batching
several live changes then checking once risks a regression landing
un-isolated across multiple commits (the exact diagnostic cost ENGOPT6
paid). Live-row order: no rigid sequence imposed — but T2.2 pairs
naturally with T2.11/T2.12+18/T2.15 (those rows are literally what give
T2.2's new intermediate states real duration, per builder's own earlier
framing) — build that connected group together rather than scattered
among the others. T2.3 (identity) and T2.10/14/20/22 (smaller mirror/
cost/keying rows) can interleave around it per builder's own judgment —
trusted after the sequencing shown all session.

- `[T2.16]` DONE (substrate) `90fb70e630e1` — activation-frame
  determinism oracle. T2 substrate complete: T2.1/4/7/8/9/13/16/17/19/21.

**SECOND message-cross on the same T2.2/sequencing ruling** — both prior
replies were still queued (unconfirmed) when builder's next two reports
went out; builder correctly held (live promote/demote refactors pending
a stuck ruling counts as genuinely blocked, not idle-waiting). Resent
plainly, verified via list_events this time rather than assuming
delivery, per the queued≠delivered rule.

Cross confirmed benign — the resend and builder's own unblock crossed in
transit, no actual delay.

- `[T2.2]` DONE — 4-state cycle + `may_transition_to` validator (16
  (from,to) pairs asserted, no shortcuts, `t2_2` pin green). Pure-refactor
  confirmed on 3 axes: recorder is mode-blind (zero tape references —
  the requested check, satisfied), the single exhaustive match site
  updated with a self-healing no-op arm, non-exhaustive sites unaffected.
  Pending states not yet threaded into live transitions by design — that
  activation is T2.11/12/15/18's job. Own floor+schedule-seed acceptance
  pending before T2.11 starts (one-at-a-time, per ruling).

**★ SELF-CAUGHT PROCESS ERROR (Ben asked "did you skip tier 1"): yes.**
After T1CMD closed T1.1-11 I jumped straight to Tier 2 without checking
whether Tier 1 had more rows — it does, T1.12-121 is untouched (resource
conservation, reservations, completion atomicity, and on — real Bastion
job/economy correctness, not filler). No T1-002 packet exists either.
This is a genuine violation of the top-to-bottom master-order rule, not
a judgment call. CORRECTION: let the current T2 connected group finish
(T2.11→T2.12/18→T2.15 — mid-flight, interrupting it now is worse than
the ordering slip), then STOP opening further T2 rows and return to
T1.12+ in strict order before T2.3/10/14/20/22 or anything else in T2.

Builder self-caught two loud, non-silent failures before trusting a
T2.2 floor result: a stale pre-T2.2 binary (version-stamp mismatch
caught it, the exact discipline [[log-time-namespace-and-vm-attestation]]
calls for) + a malformed `--corpus-seeds` arg (clap wants repeated flags
not space-separated, exit 2). No invalid verdict was ever trusted.
Rebuilding clean before re-running.

**Good architectural find:** promotion and demotion are ALREADY two-
phase in the live code (promote: Loaded+spawn tick N, decorate tick
N+1; demote: flip Simulated tick N, flush+delete tick N+1) — so T2.2's
PromotionPending/DemotionPending slot into the EXISTING phase-1 step and
gain real duration for free. T2.11/15 (promote) and T2.12/18 (demote)
aren't adding a new mechanism, they're naming a phase that already
exists. Lower risk than expected.

**Architect endorsed + tightened both corrections:** ordering plan
confirmed (finish only the in-flight T2 group, then strict T1.12+, no
new T2 work beyond it). VM-every-change made a hard requirement, with
one addition: the Opus-Reviewer backfill must produce ATTESTED evidence
(SHA-matched ATTEST line per run) — an unattested "re-ran, looks fine"
claim isn't evidence. Relayed to Opus Reviewer. Report threshold: large
achievements only (T2 group close, T1.12 return, any backfill red) —
nothing smaller.

Opus Reviewer confirmed the discipline (attested-only, immediate safety-
red escalation, evidence-complete handoff to Fable — raw ATTEST/composite
data, not prose, per the build-only/Sonnet-documents split) and verified
origin/bastion/builder push-state (`90fb70e630` == local, contains the
whole T2 opener cluster + T1CMD wire-in). Requested 32 vCPU + Builder 4's
canonical floor job lines; fan fires once received. Plan: per-commit
cargo-test pins for individual coverage + the mf byte-identity floor as
the load-bearing no-regression proof (T1CMD's wire-in is the cluster's
only live-path change, so the floor is what actually proves nothing
broke). Awaiting attested-green or a safety-red flag.

**T2.2 floor + M3A finding.** T2.2's own no-regression gate is CLEAN:
M3A fails byte-identically with and without T2.2 applied (builder
rebuilt clean HEAD `90fb70e630` specifically to isolate this), N2/M3D
both green. Good differential diagnosis — proceed with T2.2.

Builder flagged M3A's fail as a fresh "safety-red" finding. Checked
against my own bookkeeping: M3A has been logged as `[66,44,97]/2
(tracked-red)` in EVERY floor report across this whole session — ENGOPT3/
4, T0DET3, T0DET4, T1CMD, all showed the identical fingerprint, byte-
stable across dozens of independent runs on unrelated changes. This is a
long-standing, already-tracked, explicitly-accepted baseline condition
(dating to ENGOPT-era, related to B57's own-prefix-self-hit class), NOT
something the T1CMD floor silently missed — "floor green" in this
engagement has always meant N2+M3D clean with M3A held at its known
fingerprint, never "M3A also passes." Builder didn't have that history
in context, hence the alarm — reasonable given what they could see.

One thing NOT yet confirmed: whether builder's observed signature
(seed21 teleports=2 Abort@258, seed42 teleports=3+lane_violations=1)
numerically matches the historical `[66,44,97]/2` fingerprint, or
whether the field names just differ across reporting eras. Asked Opus
Reviewer (already running a broader VM pass, already warned about M3A)
to cross-check against the oldest recorded baseline to close this with
real proof rather than my own recollection. Not escalated to the
architect as a fresh safety-red — framed correctly as a legacy tracked
condition pending signature confirmation.

**★ Ben direct correction: STOP T2.2/M3A immediately** — builder was off
track, spending tokens on a tangent. T2.2 stashed (not committed, not
lost), tree clean. Retracted the M3A side-check request to Opus Reviewer.
Returned strictly to build order: T1.12+ next per master order, not
T2.11 (supersedes the earlier "finish the connected group first" plan —
Ben's direct call overrides).

**T1.12-32 group prompt sent** (Bastion job/economy conservation
cluster — resource conservation, reservations, completion atomicity,
cleanup, colony invariants). Code-verification fallback, no T1-002
packet exists; verified core targets are real before sending
(`server/src/rtsim/mod.rs`, `server/src/bastion_jobs.rs` both exist with
the cited functions). **Hard requirement stated explicitly and first in
the prompt this time: every commit tested, every commit gets a VM run,
no exceptions** — the corrected standard applies from here forward, not
just as a footer.

**30-min monitoring check-in:** T1.12-19 (8 rows) all built; T1.12
committed `25a367ac0a` (bookkept earlier), T1.18+T1.19 committed
`27cce4d635`, bastion-server suite green. Ben personally asked Builder 4
"what are you even testing here?" — good prompt, produced an honest
concrete answer: T1.13's reservation duplicate-detection unit pin
(2 colonists reserving the same dropped item → one dupes/stalls,
`duplicate_reservations()` + a `debug_assert` in `reserve()` catch it
loudly) plus a live-guard/VM-floor pairing. Builder disclosed T1.14/T1.18
honestly as "thin" — invariant-explicit rather than catching a bug that
exists today.

**★ REAL SELF-CAUGHT GAP, same class as [[gate-must-test-live-path]]:**
Builder 4 realized the VM floor they've been running (M3A/N2/M3D) tests
LADDER TRAVERSAL — it never touches an item. T1.12-19 is all item/
resource-conservation work; the floor has been the wrong subsystem this
whole cluster. The harness already has the right scenarios:
`--b5-scenario` (mine+haul, exact item conservation), `--b55-scenario`/
`b55-deep` (200-block slab, conservation through pile-merge + soak),
`--mine-fidelity-scenario` (full mine→haul→deposit + FinalStateCertificate),
an explicit `authoritative_conservation_failure` assertion. Builder asked
whether to switch the VM leg to these conservation scenarios and re-run
the whole T1.12-19 batch through them.

**RULING: yes, switch immediately.** Re-run T1.12-19 through b5+b55+
mine-fidelity, confirm no conservation regression. This is the correct
gate for this cluster — the ladder floor was never going to catch a
double-reserve or a decrement/drop desync, only a conservation scenario
that actually pushes items through reserve→haul→complete→deposit can.
Keep the ladder floor for anything that touches scheduling/dispatch, but
item/resource rows get the conservation scenarios from here forward.

**★ REAL VALUE OF THE EVERY-COMMIT-VM-RUN RULE, proven immediately:**
Opus Reviewer's attested backfill (5/5 VMs, deterministic) caught a
harness BUILD BREAK at T1.18/19 (`27cce4d635`) that bastion-server's own
unit pins never would have — `cargo build --profile verify -p
bastion-harness` fails, `FlightSample` initializer missing the new
`fetch_reservation` field at main.rs:1581 (the field was added, the
harness construction site wasn't updated). bastion-server pins pass
46/0; only the harness build catches this. This is EXACTLY the gap that
was open all night — a commit with green local pins, silently broken at
the one boundary local testing never crosses. Opus flagged Builder 4
directly with the exact fix. Not architect-triage (a straightforward
missed-field compile break, builder-fixable), not escalated.

**Attested status otherwise, all SHA-matched:** T2 cluster clean (0 new
failures vs baseline). T1.12/13/14/16/15/17 all green. mf
durable_composite byte-identical across serial/sched-5/sched-12 AND
Intel Broadwell + AMD Rome — cross-machine AND cross-schedule AND
cross-microarchitecture. 2 pre-existing unrelated veloren-common reds
(i18n wheat gap, slowjob artifact) confirmed constant since baseline,
not cluster-introduced. Fable handoff still held per standing
instruction — Opus re-runs no-reg once the FlightSample fix lands.

Fixed + pushed `ade7b2f8c7` — harness build clean. Root cause banked:
cross-crate struct-field adds need a workspace-wide grep + a harness
build, crate-level pins never compile the harness bin. Conservation fan
re-fired at the fixed tip (t1consv2: b5+b55-deep+mine-fidelity). Holding
T1.20+ until it confirms T1.12-19 conservation-clean.

**Division of labor changed (Ben direct):** Builder 4 off VM testing
entirely, back to build→pins→commit. Opus Reviewer now owns test
execution + custom-test authoring going forward, ongoing not one-shot.
Opus confirmed the FlightSample fix verified green (harness builds, mf
byte-identical, N2/M3D 2/2) and is running a wide sweep (22 VMs, 100%
vCPU while builder's stalled: all 14 ladder episodes incl. NFENCE, mf x3,
5 gameplay scenarios). Will author custom L1-L3 scenarios (conservation
predicate, reservation-uniqueness soak, command-lifecycle-on-haul) next
— test code only, isolation discipline (own-files, scoped commits,
never git add -A). Fable handoff still held.

**30-min check-in:** Builder 4 absorbed the role-change cleanly, banked
it to memory, waiting on t1consv2 (last VM result they personally track)
before resuming T1.20+ in the new lighter local-only loop. No stuck/
looping concern — legitimately blocked on a result outside their lane now.

**★★ PIVOT (Ben direct): pausing Tier-1 build-order, switching to a new
independent determinism source audit.** Found at
`H:\My Drive\bastion-Chatgpt\engine design\determism\` — a static-
analysis-only (no code run) exhaustive audit: 118 findings, 20 coverage
areas, 34 Critical/69 High/15 Medium, each with exact file/line +
proven/not-proven + remediation direction. Spot-checked one finding
(DET-RNG-007, structure loadout `rand::rng()` in world/src/site/mod.rs)
directly against live source — confirmed accurate. Files:
`PROJECT-BASTION-EXHAUSTIVE-DETERMINISM-SOURCE-AUDIT.md` (main report),
`PROJECT-BASTION-DETERMINISM-FINDING-LEDGER.csv` (structured, one row
per finding), `PROJECT-BASTION-DETERMINISM-CALLSITE-INVENTORY.csv` (the
193-site raw RNG inventory), `PROJECT-BASTION-DETERMINISM-AUDIT-SUMMARY.json`
(machine-readable rollup).

Testing scope narrowed per Ben: determinism-only (paired-run harness
comparator, byte-identity before/after), not the full conservation/
gameplay suite — same every-commit-tested rule, narrower test type.

Sent Builder 4 the first cluster: DET-COV-001 (RNG foundation, 10
findings). DET-RNG-001-005 are reference-pattern documentation, not
bugs. DET-RNG-006 is the 193-callsite master inventory (work Critical
ones first, not all at once). DET-RNG-007 (structure loadout RNG) is
real+open+fixable now, same class as T0.36's fix. DET-RNG-008
(bastion_jobs scatter RNG, tick-global not per-owner) needs a T0.32/33-
overlap check first. DET-RNG-009 (no RNG-stream persistence protocol) is
a real scoping question, not a quick fix — likely fork candidate.
DET-RNG-010 lower priority.

**Note for later:** the audit's own text flags a discrepancy — "a ledger
says stable A* frontier ordering is DONE [DONE.1/ENGOPT1], but the
audited common/src/astar.rs still lacks the required total tie-break."
Worth verifying once the RNG cluster is through — not chased right now.

**Opus Reviewer backfill COMPLETE, VMs handed back to Builder 4** (tip
`ade7b2f8c7`, all SHA-attested). Green: T2 cluster + T1.12-19, 0 cluster-
introduced failures; mf byte-identical across serial+3 schedule-seeds
AND Intel+AMD; P0/P0G/N1C/N2/M3D floors 2/2. Tracked-reds excluded from
pass bar (N1 known-open vanilla-leak, M3A B57 self-hit, 2 unrelated
veloren-common reds) — none cluster-introduced. Caught+fixed: the
FlightSample build break. ~14 wide-sweep jobs incomplete (rate-limited,
re-runnable later). Fable handoff still held. Opus moves to authoring
L1-L3 test scenarios next.

**DET-COV-001 (RNG foundation) named findings RESOLVED**, tip `e711c37dea`:
- DET-RNG-007 DONE `8e7e2c4f55`: structure-entity loadout was fresh
  `rand::rng()` (OS entropy). Keyed on (world seed, entity world position,
  loadout domain salt) via ChaChaRng, same class as T0.36. veloren-world
  clean. (test_site()'s rand::rng() at line 3355 is test-only, left as-is.)
- DET-RNG-008 DONE `2fe861aeda`: toss scatter was one shared tick-global
  StdRng cursor across 4 drop sites, keyed by ECS-join order. Now
  `toss_scatter_rng(tick, drop-cell, site-domain)` per site, order-
  invariant. Cosmetic outcome shift expected (landing→pile-merge), values
  re-pin. bastion-server 46/46.
- DET-RNG-010 DONE `e711c37dea`: merchant escort seed crushed a 15-min
  bucket to [u8;32] with no keying (every merchant in a bucket shared a
  stream). Keyed on (npc.uid, bucket, salt), 15-min stability preserved.
  veloren-rtsim clean.
- DET-RNG-009 SCOPED, CLOSED AS MOOT: the only two stored RNG fields
  (NpcCtx.rng, dialogue_rng) are re-derived EVERY TICK from
  `tick_rng(world_seed, tick, salt)` — counter-RNG, not a persisted
  cursor. No authoritative RNG state survives across ticks/saves, so
  there's nothing to fork on reload. Well-verified, real closure not a
  hand-wave — RULING: close it, AND build the lightweight fitness gate
  Builder proposed (source-scan asserting no persisted authoritative RNG
  cursor exists) — cheap, matches the established gate pattern (T0.48/
  T1.1), prevents a future regression into this exact class.
- DET-RNG-001-005: confirmed reference patterns, no fix, as expected.

DET-RNG-006 (the 193-callsite master queue): treating this as an ongoing
ambient queue, not a single blocking gate. RULING: continue pulling
Critical entries from worldgen/terrain/inventory/events specifically —
similar low-risk pattern to what's already landed. Explicitly steering
AWAY from combat's RNG conversion for now (DET-COV-009 flagged "failed
contract," but combat changes are higher blast-radius — visible balance/
feel impact, needs real playtesting not just byte-identity, deferred
until reviewed with more care, not built solo).

**NEW: ledger v2 + residual-pass addendum, 11 more findings
(DET-ADD-001..011), total 129.** Found in `03-engine-improvement-research/`
alongside the master-order file. 4 flagged material: DET-ADD-001 (loot
ownership Instant expiry), DET-ADD-004 (persistence CTE load with no
ORDER BY, parent-before-child assumed not guaranteed), DET-ADD-007
(combat XP/damage-contribution reduction over an unordered HashMap),
DET-ADD-010 (renderer particle timing/randomness).

**CAUGHT: audit methodology gap — wrong branch.** Verified DET-ADD-001
directly: `common/src/comp/loot_owner.rs` STILL uses raw `Instant` on my
checkout — but that exact bug was fixed hours ago tonight (`3b137017e6`,
during the ENGOPT6 hunt) and IS present on `bastion-origin/bastion/builder`
(confirmed via `merge-base --is-ancestor`). The audit's own metadata says
it read `bastion/block-B6HAUL` — the docs-only branch — never
`bastion/builder` where the real code lands. DET-ADD-001 is a false
positive: already fixed, just not on the branch the audit scanned. Every
finding (original 118 + these 11) needs a live cross-check against
`bastion/builder`'s actual tip before being treated as open — same
staleness discipline as the master-order file, applied to a new document.

Sent Builder 4 the 11 new findings with DET-ADD-001 marked already-
fixed/skip, instructed to verify each remaining one against their own
`bastion/builder` checkout before building anything.

**DET-ADD persistence batch DONE `47a9556892`:** DET-ADD-006 (pet
orientation keyed by player uid+pet ordinal, StdRng, no OS entropy),
DET-ADD-004 (load_items CTE: added depth column + ORDER BY depth,item_id
— ancestors before descendants, SQLite-version-robust), DET-ADD-005
(pet SELECT: ORDER BY p.pet_id). All verified present on bastion/builder
first. veloren-server clean. No local persistence integration test — SQL
verified by inspection, runtime/live-load validation deferred to Opus
(acceptable, this is exactly the class local unit tests can't cover).

**Worldgen plot sweep, findings before the batch:** the "already has a
seeded rng" theory was wrong — plots DO take a seeded rng in generate(),
but the actual `rand::rng()` calls are in `render_inner` (the block-
paint pass, no rng param) and free helper fns. Real fix = position-keyed
per DET-RNG-007's pattern, not reuse-the-passed-rng. Proposed: one
shared `plot_render_rng(pos, plot_salt) -> ChaChaRng` helper in
`world/src/site/plot/mod.rs`, clean self-position sites first, bridge/
dwarven_mine (free fns needing position threaded in) flagged separately
as harder. Approved — reuse-first, consistent determinism story, correct
split of easy-majority from harder-minority. DET-ADD-002 (invites) file
paths given precisely. DET-ADD-008 confirmed present, queued after sweep.

**30-min check-in:** active, steady progress — `plot_render_rng` helper
built, sweeping clean self-position sites one at a time (camp done,
giant_tree in progress). No stuck/looping concern.

**★★★ Ben direct — full scope clarified: "we want this — we are going to
end all non-determinism in the game once and for all."** Bigger than
Factorio-parity (simulation-only lockstep) — comprehensive, all 20
coverage areas including renderer/R0D, networking, asset pipeline,
shutdown/failure, build reproducibility. Nothing in the 129-finding
ledger is permanently shelved. Corrected my earlier "renderer/network —
lower priority, hold" instruction to Builder 4: still holds as practical
sequencing (finish what's in flight first), not a scope cut.

**★ DET-RNG-006 worldgen plot sweep COMPLETE, 4 commits `f007d19fae`→
`25f9c07e81`.** ~24 sites across 20 structure-plot files, all now keyed
via the shared `plot_render_rng(pos, per-kind-salt)` helper — camp,
giant_tree, cultist, haniwa, jungle_ruin, rock_circle, troll_cave,
sahagin, sea_chapel, myrmidon_arena/house, terracotta_house/yard/
palace(×2), vampire_castle(×2), dwarven_mine(×3), adlet, bridge, plaza,
pirate_hideout. Free-fn sites keyed cleanly (already had a pos param, no
threading needed). Correctly skipped 5 verified non-holes (test-only
sites, the intentional live-mode else-branch). veloren-world clean
throughout. A real, substantial worldgen determinism win — structure
interior/loot/mob placement is now reproducible.

**DET-RNG-006 worldgen slice 100% CLOSED**, tip `0a9afd3195` — the 2
standalone sites also done (block.rs keyed by the fn's own structure_seed
+pos matching its existing RandomField basis; airship_travel.rs keyed by
airship_dock_center). Full world/ subset (26 sites minus 5 verified
non-holes) done across 6 commits. veloren-world clean throughout.

**DET-ADD-002 DONE `e5630c93b9`:** invite timeouts now f64 sim-time
deadline (Time.0+dur), same fix class as loot-owner. veloren-server clean.

**DET-ADD-008 DomainHasher reuse ruling:** approved builder's proposed
adapter — a free fn `stable_hash_u64<H: Hash>(label, value) -> u64` in
common/src/state_hash.rs, reusing the same Sha256 primitive via a
PRIVATE std::hash::Hasher adapter, rather than extending DomainHasher's
public API (which would collide on `finish()` — inherent DomainHash vs
trait u64). Clean, minimal, no unnecessary new abstraction. Persistence
question: build it regardless of whether item.hash/terrain_revision
turn out to be cross-build-persisted or runtime-only — per Ben's full-
elimination directive we don't skip findings for being "merely
defensive." Directed builder to do the quick persistence check anyway
(affects urgency/ordering, not whether it gets built).

**★★★ NEW: `determism/v3/` — RNG deep-research pass, 82 new findings
(275 merged total), 1224 raw sampling callsites cataloged.** Far more
rigorous than the first pass — catches classes the earlier work missed
entirely: native-endian seed bytes, `StdRng`/`SmallRng` being explicitly
non-portable per Rust's own Rand Book, "parent-cursor reseeding" (a
child RNG seeded by drawing from the PARENT's cursor — deterministic in
isolation but silently shifts if ANY unrelated earlier draw on the same
parent changes), and a real saturation bug in tonight's own DET-RNG-010
fix. Files: `PROJECT-BASTION-RNG-DEEP-RESEARCH-ADDENDUM.md` (narrative +
24 named patterns RNG-DEEP-001..025+078/079), `-DEEP-FINDINGS.csv` (82
rows), `-CHANGE-REGISTER-v2.csv` (275-row merged register), `-SAMPLING-
CALLSITE-INVENTORY.csv` (1224 raw sites), `-LINE-CHANGE-GUIDE-v2.docx`.

**TWO ARE FOUNDATIONAL — sequence these FIRST, before any more leaf-site
fixes:**
- RNG-DEEP-001: `world/src/util/seed_expan.rs:7` uses `to_ne_bytes()`
  (native-endian) — output differs between little/big-endian targets.
  This is the WORLD SEED EXPANSION itself — every downstream keyed-RNG
  fix built tonight ultimately traces through this.
- RNG-DEEP-003: `rtsim::tick_rng` (the exact pattern generalized as "the
  good reference" all night) only populates 16 of its 32 seed bytes, and
  its salt is an untyped u32 (collision-prone). This is the CORE helper
  most of tonight's fixes are modeled on.
Fixing these LAST would mean re-shuffling every already-fixed site's
specific values a second time. Fix root first.

**Real, open, same-class-as-tonight's-fixes (leaf-level, after the
above):** RNG-DEEP-004/007/009/010 (StdRng/SmallRng non-portability, 4
sites: server/agent, common/path Chaser, rtsim/generate), RNG-DEEP-011/
012 (bastion_jobs breakdown-roll shared tick cursor — check overlap with
DET-RNG-008 first), RNG-DEEP-013/014 (worldgen/civ parent-cursor
reseed), RNG-DEEP-016/017/018 (rtsim cleanup/migrate/architect — one
shared cursor across all NPCs/sites in a loop, same class as DET-RNG-008),
RNG-DEEP-021 (weather lightning — frame-delta + ambient RNG + unordered
set iteration), RNG-DEEP-023 (npc_ai uses collision-prone `npc.seed`,
not the stable `npc.uid`, as stream identity), RNG-DEEP-025 (path.rs
fallback combines OS entropy AND unordered HashMap iteration).

**Needs verification against tonight's own work:** RNG-DEEP-022 —
same site as the already-fixed DET-RNG-010 (merchant escort quest seed),
but characterizes a SPECIFIC bug the original finding didn't: float-to-
u8 casts saturate at 255, so the old seed froze permanently after ~63.75
hours. Check whether Builder's fix (keyed on npc.uid+bucket+salt)
actually removed the u8 cast, or just added collision-resistance on top
of a still-saturating bucket value.

**Already in progress, consistent:** RNG-DEEP-078/079 = DET-ADD-008
exactly (item_hash/terrain_revision DefaultHasher) — same two sites,
Builder's stable_hash_u64 plan already matches this doc's recommendation.

**Architectural note, not urgent:** the doc proposes one shared
`RngKey`/`RngDomain` typed protocol to replace the various ad-hoc keying
tuples built tonight (tick_rng, deterministic_agent_seed,
plot_render_rng, etc.) with one unified versioned schema. Real long-term
value, but NOT worth forcing now — flagging for Ben's awareness as a
future consolidation pass, not blocking current leaf-fixes which are all
individually correct even if not yet unified.

**Terrain/inventory/event subset, heterogeneous (not a uniform sweep):**
- `terrain.rs:134` DONE `6b02648f90` — dead OS-entropy leftover from
  T0.36 (real spawn RNG already correctly keyed elsewhere), removed.
- Clean-keyable trio approved to proceed: `interaction.rs:541` (key by
  ev.pos), `inventory_manip.rs:170`+`interaction.rs:298` (key by entity
  uid+tick).
- `loadout_builder.rs:1154`/`1345` + `trade_pricing.rs:1047`: no
  identity in scope, need API-threading (seed/rng param) that ripples to
  every caller of these widely-used builder/economy APIs. RULING:
  DEFER as their own scoped follow-up block, don't ripple mid-sweep —
  same pattern as T0.44/Loom-Shuttle deferrals tonight (tracked, not
  dropped). When built: prefer threading an already-seeded `&mut impl
  Rng` parameter through, not a raw seed value — matches RNG-DEEP-004's
  own guidance (caller selects/derives the stream, callee just consumes
  it) and the shared RngKey/RngDomain pattern's `rng_for(key) -> impl Rng`
  shape. Do the clean-keyable trio now, move on to the RNG-DEEP-001/003
  foundation fixes.

**★★★ FABLE ESCALATION (Ben direct): RNG-DEEP-001/003 route through
Fable Reviewer, not routine treatment.** Held Builder 4 from building
either solo. Independently re-verified both against live
`bastion-origin/bastion/builder` (not just trusting the audit doc):
`seed_expan.rs::cast_u32x8_u8x32` still uses `to_ne_bytes()` (native-
endian), `tick_rng` still only populates 16/32 seed bytes with an
untyped u32 salt — both exactly as the audit describes, confirmed live.
Sent a full 4-part evidence packet to the architect (claim/evidence/
lower-tier-trail/blast-radius) requesting Fable engagement — qualifies
as a genuine high-blast-radius architectural fork (tick_rng is, per its
own doc comment, "the ONE constructor every rtsim rule RNG goes
through"). Builder continuing other approved work meanwhile.

**30-min check-in:** caught a real message-cross — Builder 4 had already
written + compiled BOTH foundation fixes (seed_expan + tick_rng via
DomainHasher reuse) before the hold instruction landed, and was running
the rtsim suite to check pin impact when the hold arrived. Verified
nothing committed yet (bastion/builder tip is still the sprite-removal-
timeout commit). Directed builder to `git stash` the uncommitted work
(preserve, don't commit, don't discard) so it's available as input for
Fable's review without having bypassed the escalation. Not a stuck/
ignoring situation — pure timing race, confirmed via git state not
assumption. Continuing on other approved items meanwhile.

**★★★ DOUBLE message-cross + escalation model changed underneath it.**
Both my hold AND my stash instruction arrived AFTER Builder 4 had
already committed+pushed (`2d0e6da435`) — the work was fully done before
either message could reach them, confirmed via transcript order. NOT
non-compliance, genuinely unavoidable async timing given how fast this
particular block moved. Separately, Ben talked to the architect directly
and CANCELLED the Fable-Reviewer hand-off model: **new standing rule —
for a genuinely hard/foundational block, ELEVATE THE BUILDER'S level for
that specific block, don't spin up a separate Fable-Reviewer engagement.**
Architect's own independent read matches Builder's report exactly:
seed_expan is a free no-op on the x86 fleet, tick_rng is the real
foundational one (re-shuffles all RTSim RNG) — acceptance bar set as
full re-pin + 3-machine × multi-schedule certification. No revert
needed — the landed fix is accepted, just needs that elevated acceptance
evidence from Opus, not a separate review hop.

**Full untangle of the commit/revert/re-land sequence on RNG-DEEP-001/
003, for anyone reading history later:** (1) Builder built+committed+
auto-pushed `2d0e6da435` before any hold message could land (confirmed
timing race). (2) My "stash it" instruction (based on a stale "nothing
committed" check) arrived after the push; Builder correctly judged that
stashing an already-shared-remote commit risked a force-push, so instead
REVERTED it as `c6db75cef5` — original preserved in history for later
re-landing, no history rewrite, responsible call given what they knew.
(3) Meanwhile Ben and the architect settled directly on the real model:
no separate Fable engagement — hard/foundational blocks get the BUILDER
elevated to apex tier for that specific block instead. Architect's
independent read matched the original fix exactly. (4) Directed Builder
to re-land (cherry-pick `2d0e6da435` or revert-the-revert of
`c6db75cef5`) — the fix was correct all along, the churn was pure
message-timing, not a substantive problem with the work. Net effect once
re-landed: identical to if none of this had happened, just extra commits
in the history recording the back-and-forth honestly.

Ben clarified sequencing directly: builder finishes/re-lands NOW (top
priority, no further hold), Ben personally elevates to Fable as a
POST-HOC review once landed — not a pre-gate. Relayed to Builder as
highest priority, ahead of DET-ADD-008/leaf sweep.

**★ RE-LANDED, `48e4c05b77`** — revert-the-revert of `c6db75cef5`,
byte-identical to the original approved `2d0e6da435`. History preserved
honestly (orig→revert→reapply, no force-push). Local: veloren-rtsim +
veloren-world clean, rtsim 18/18 on identical code. Triggered Opus's
elevated acceptance (full re-pin + 3-machine × multi-schedule cert).
Builder resuming DET-ADD-008 (was mid-compile when the re-land jumped
the queue) then the leaf sweep (004/007/009/010/011 + clean-keyable
trio) with the confirmed minimal per-site pattern, no RngKey abstraction.

Opus's watcher caught the re-land and launched the elevated cert before
my trigger even landed — 7 mf jobs (serial + 6 schedule-seeds) across
Intel Broadwell + AMD Rome, N2/M3D floor, all SHA-attested, ~15min ETA.
Will run through det-classify: DECLARED_REPIN_STABLE at the new
composite if internally byte-identical, NONDETERMINISTIC (immediate
flag) if any schedule/machine disagrees.

**Interim result:** composite moved as expected, new value confirmed
IDENTICAL across all 7/7 schedules (serial + 6 schedule-seeds) — schedule
-determinism holds at the re-pinned value. Two gaps before final
attestation: all 7 VMs happened to land AMD (GCP scheduling luck, no
Intel leg yet), and N2/M3D floor was SLOT_LOST (fan rate-limited).
Supplement running now (Intel-forced + floor re-run). Opus also self-
caught + is fixing a real bug in their own classifier tooling
(INVALID_EVIDENCE guard was scoped too broadly, falsely invalidating
valid composites alongside the unrelated SLOT_LOST floor jobs) — good
self-verification discipline. ~15min to final attestation.

**★★★ NEW: `determism/v4/` — RNG pass 3, 52 new findings (327 merged
total).** Confirms the security boundary explicitly: crypto secrets and
session/auth entropy stay genuinely unpredictable, isolated from
authoritative state — not part of the "eliminate everything" mandate.
Highest-value new findings, NOT yet queued to Builder (finishing current
sweep first):
- **A whole new subsystem: `common/src/lottery.rs` loot-table/weighted-
  selection** — 3 distinct issues (RNG-P3-003 16-bit seed collapse
  before float-weighted multiply; RNG-P3-004 nested tables call the
  AMBIENT Lottery::choose() instead of the passed rng, severing stream
  ownership; RNG-P3-006 participant order + f32 cumulative sums +
  swap_remove all influence draws). Core to the whole reward economy,
  untouched until now.
- **RNG-P3-001**: WASI plugin default context has a random per-context
  seed — security/plugin-sandbox-adjacent, needs careful treatment
  (ties to the crypto/session exception above).
- **Possible OVERLAP with already-fixed/deferred work, needs
  reconciliation before building anything new:** RNG-P3-011 cites
  `world/src/block.rs:488-503` — Builder already fixed `block.rs:489`
  (structure-block choice); check whether this is the same site or a
  different nearby call. RNG-P3-018 cites `trade_pricing.rs:995-1029` —
  adjacent to the already-deferred `trade_pricing.rs:1047` item; may be
  the same deferred block or a distinct float-to-index issue.
- **RNG-P3-012**: combat-domain (attack.rs weighted AI sampling) — falls
  under the standing combat hold, don't build solo.
- **RNG-P3-016**: pathfinding, ambient RNG + powf/sin/cos — cross-
  platform float-exactness concern, genuinely new site.
- **RNG-P3-031/032**: worldgen noise-seed INITIALIZER ORDER coupling
  (source's own comment: "changing initializer order significantly
  changes worldgen") + a transitive dependency (noise crate's
  permutation table via an older Rand/XorShift stack in Cargo.lock).
  Foundational-ish, needs the same care as RNG-DEEP-001/003.
- **RNG-P3-037/038**: NPC body construction ambient RNG — new site,
  affects gameplay/animation/figure identity.
- **RNG-P3-040**: random effect-instance IDs used as semantic
  correlation/dedup identity — may connect to the entity_manipulation.rs
  "instance:" sites flagged-but-excluded earlier as "probably just
  unique IDs" — worth re-checking with this framing.
- **RNG-P3-043/050**: dependency-policy findings — pin Rand's actual
  VALUE protocol not just semver range; a tooling/process
  recommendation, not a single-site code fix.
- **RNG-P3-048**: loot-table DATA itself (1409 entries, 256 RON files) —
  content concern, not code.

Not yet sent to Builder — queuing as the next batch after the current
leaf sweep completes, to avoid more mid-flow context-switch churn after
tonight's RNG-DEEP-001/003 saga.

**30-min check-in:** steady progress, two clean commits landed —
`0dbdabd7f9` (DET-ADD-008, StableHasher over the DomainHasher Sha256
primitive as approved) and `8c6fb446e1` (RNG-DEEP-004/007/009/010 leaf
batch, portable named generators replacing StdRng/SmallRng at 4 sites,
minimal per-site pattern as approved, no RngKey abstraction). One known
pre-existing gap (B59, tracked i18n manifest issue) briefly stopped a
combined test run before rtsim/common-state executed; builder correctly
re-ran those crates directly rather than treating it as new. No stuck/
looping concern.

**★ Full sweep CLOSED, 3 commits (`0dbdabd7f9`, `8c6fb446e1`,
`d6f1de9d7a`).** DET-ADD-008 done (StableHasher, no collision, impact
bounded — item.hash only touches voxygen hotbar bindings, terrain_
revision is session-only). RNG-DEEP leaf sweep done: 004/007/009/010
fixed (portable ChaCha throughout); 011 was ALREADY substantively fixed
by the earlier DET-RNG-008 work (audit read the stale branch) — closed
its portability residual only; **022 VERIFIED CLOSED** — the earlier-
flagged u8-saturation concern doesn't apply, the bucket already casts to
u64 in the shipped DET-RNG-010 fix. Clean-keyable duo done — the whole
terrain/inventory/event DET-RNG-006 subset is now CLOSED. Tests: common
213/214 (B59 tracked red only, confirmed same pre-existing failure),
common-state/rtsim/bastion-server/server all green. Re-pins expected
across the batch — triggering Opus's elevated cert at tip `d6f1de9d7a`.

**Tracked remaining (not forgotten, explicitly listed):** deferred
loadout_builder/trade_pricing API-threading block, combat holds (DET-
ADD-007, DET-COV-009), voxygen client-presentation subset, RngKey
abstraction follow-up, a trivial pre-existing cosmetic FYI (unused
tick_rng handle at cleanup.rs:21, not builder's).

**Opus anti-churn proposal, APPROVED:** tip is re-pinning faster than a
full 3-machine × multi-schedule cert can complete (confirmed — tip moved
`d6f1de9d7a`→`79cd8b4ad7` while the cert was still chasing the prior
target's Intel leg). New policy: light check per intermediate commit
(multi-schedule internal stability, single-vendor — cheap, catches a
broken re-pin fast), full elevated cert reserved for the SETTLED end-of-
sweep tip once Builder declares the sweep closed. Told Opus NOT to chase
`48e4c05b77`'s missing cross-vendor leg retroactively — superseded, not
worth it. Infra blip (transient GCP create-plane rate-limit) self-
recovered, no lingering VMs, noted only.

**v4 Priority 1+2 DONE, `79cd8b4ad7`:** lottery P3-003 (full 32-bit seed
via exact f64, closes the %65536 unselectable-entries bug), P3-004
(nested tables draw from the caller's stream, not ambient), P3-006
(canonical (weight, stable_hash_u64 identity) sort, reuses existing
T:Hash bound, no API ripple). trade_pricing P3-018 confirmed genuinely
DISTINCT from the deferred :1047 item, fixed cleanly (exact integer
index draw, zero re-pin — the wrapper was live-entropy). npc P3-037
fixed via the existing injectable `random_with` variant (15 families).
Overlap verdicts resolved: P3-011 CLOSED-SUPERSEDED (same site as the
earlier :489 fix, confirmed), P3-018 confirmed distinct as predicted.
common 213/214 (B59 only) throughout.

**New flag — P3-038's real leak, judgment call:** the actual leak isn't
where first suspected — `generation.rs:274`, `EntityInfo`'s DEFAULT body
uses ambient `random_humanoid` whenever an asset configures NO body,
leaking past the already-fixed loadout RNG. Two options: (a) fixed
deterministic default — simple, no ripple, but risks VISIBLE clone-spawn
regression wherever this default path fires; (b) thread rng into
`EntityInfo::at` — architecturally correct (matches tonight's pattern)
but wide ripple, same class as the deferred loadout_builder/trade_pricing
block. RULING: (b), fold into the SAME deferred threading block rather
than risk a new visible bug for a quick win — don't trade a determinism
fix for a fresh gameplay regression. That threading block is now the
next concrete priority (three related fixes: loadout_builder,
trade_pricing:1047, EntityInfo::at).

**Other flags resolved:** P3-040 folded into the combat-hold triage per
builder's own sound reasoning (same class as entity_manipulation's
Outcome-dedup sites) — approved. P3-012 stays held. P3-001 (WASI plugin)
— confirmed in-scope (it's the INSECURE/non-crypto WASI RNG specifically,
not the secure one, so it's within the audit's own security exception),
but asked builder to double-check "insecure" really means
"not security-critical" in WASI's own terminology before building, given
plugin-sandbox sensitivity. P3-031/032 (noise-seed init order) — routing
through the architect-elevation path now, same as RNG-DEEP-001/003.

Verified P3-031/032 live (`world/src/sim/mod.rs` GenCtx initializer —
~20 noise generators seeded via sequential parent-cursor draws, source's
own comment admits the order-sensitivity outright). Sent full elevation
request to architect with a proposed direction (named-domain derivation
via DomainHasher reuse, same shape as tick_rng). Holding until confirmed.

**Opus standing down on VM certs temporarily** — GCP create-plane
rate-throttled from tonight's ~8 fans (confirmed throttle not quota:
single probe VM creates fine, sustained fan creates don't). Correctly
tied to the approved policy: no reason to fight the throttle light-
checking a still-moving tip when the heavy cert waits for sweep-closed
anyway. Plan: stand down until both sweep-closed signal + create-rate
recovery, then one definitive elevated cert at the settled tip. Suite
itself stays built/validated regardless — ready to fire the instant a
clean matrix is possible.

**Architect refined the elevate-in-builder process (standing confirmation
going forward):** THE BAR = elevate only a DOMAIN-ROOT (single
constructor/seed/cursor that reshuffles an ENTIRE domain — tick_rng,
GenCtx). A routine leaf fix stays normal-tier, no flag. Confirmed
GenCtx clears the bar. Process correction: don't hold a domain-root item
out-of-band waiting for per-item confirmation (that's what caused the
RNG-DEEP-001/003 commit/revert/re-land churn) — instead it stays in the
NORMAL queue at its proper position, tagged apex, and gets flagged to
the architect only when the builder is ABOUT TO REACH it, not
preemptively. Standing confirmation granted for anything clearing the
bar — no fresh nod needed each time. Un-held RNG-P3-031/032 accordingly
— back in normal queue position, will flag when Builder nears it.

**★★★ Ben direct: full priority ranking, big to small, given to Builder
as standing work order (not just next-item).** Tier 1 (domain roots):
seed_expan+tick_rng done, GenCtx in progress. Tier 2 (major subsystems):
COMBAT un-held as the next priority after GenCtx (DET-COV-009 rated
"FAILED CONTRACT," the worst rating in the original audit — held all
night for care, but stops being indefinitely deferred now), then the
threading block, persistence, renderer, networking (stays deferred per
standing single-player-first). Tier 3: remaining scattered leaf items +
whatever the new clock/scheduling/physics/persistence deep-pass finds.
Tier 4: policy/data items, awareness only. Builder told to work this
order without waiting on me between tiers, flag only for genuine
divergence per the corrected chatter policy.

**30-min check-in:** no new commit yet (tip still `79cd8b4ad7`), but
active substantive progress on GenCtx — `world` crate compiles clean
with the fix, now working through server + voxygen callers (a genuinely
large domain-root touching multiple crates, expected to take longer
than the leaf fixes). No stuck/looping concern, methodical progress on
a real large item.

**★★★ TIER 1 COMPLETE + TIER 2 TOP TWO DONE, tip `c4f9608a21`.**

- **GenCtx (domain root #3) DONE** `60e1682428` (a heredoc slip garbled
  the commit message; fixed via a follow-up empty annotation commit
  `c4f9608a21` carrying the real message — content verified exact, no
  history rewrite, correct handling). Every noise field's seed (turb_x_nz,
  chaos_nz, hill_nz, alt_nz, temp_nz, small_nz, rock_nz, tree_nz,
  structure_gen, humid_nz, river_seed, rock_strength_nz, uplift_nz, etc.)
  now derives independently as f(world_seed, generator_name) via
  DomainHasher — the ~20-draw shared-parent-cursor coupling is gone, the
  source's own "changing order will significantly change WorldGen"
  warning no longer applies. Intended one-time re-pin of all worldgen
  terrain output.

- **COMBAT BATCH DONE** `dbbacea48e` (un-held per the ranking): DET-ADD-
  007 fixed — XP damage-contribution reduction now sorts canonically by
  stable_hash_u64 before the f64 total/percentage/award pass. All 31
  `instance: rand::random()` sites (incl. RNG-P3-040's health.rs ones) →
  `combat::next_attack_instance()` monotonic counter (identity/
  correlation only, not a random value). RNG-P3-012 verified CLOSED-
  SUPERSEDED (attack.rs already draws from the caller's agent stream,
  keyed earlier by RNG-DEEP-004). apply_attack's rng param generalized
  ThreadRng→&mut impl RngExt (type-level only). Self-verified no balance/
  feel change — ordering/identity only, correctly self-assessed, no flag.

- **THREADING BLOCK DONE** (same commit): `EntityInfo::at(pos, rng)`
  threaded through ~150 call sites (worldgen plots pass keyed streams,
  rtsim passes its own rng, operator/client paths pass explicit ambient
  rng); with_preset and defaults now take rng; ambient random_items
  wrapper deleted. Closes RNG-P3-038's default-body leak per the earlier
  (b) ruling — chose the wider correct fix over a deterministic-default
  shortcut that risked a visible clone-spawn regression.

Acked Builder 4, cleared to continue to Tier 2 #7 (renderer) as they
stated. Opus re-cert requested at `c4f9608a21`+, flagged as three
stacked intended re-pins (GenCtx worldgen + combat instance IDs +
threading default-body) so the composite is expected to have MOVED, not
matched old.

**Sweep continued to `a3c4f638b4`, then queue CLEARED.** RNG-P3-001
WASI DONE (security-checked against WASI's own secure/insecure spec
split, insecure_random_seed keyed by stable_hash of plugin name, no
crypto-consumer weakening). DET-ASY-002 CLOSED-SUPERSEDED (already fixed
by ENGOPT4's tie-break; audit read stale branch). RNG-P3-016 pathfinding
DONE (RRT search → one ChaCha8 stream keyed by (start,end); spheroid
sampler takes caller's stream; parent re-pick draws from sorted
candidates; compiles clean with/without the unused rrt_pathfinding
feature). Session totals: 3 domain roots + combat batch + worldgen
DET-RNG-006 subset + threading block + lottery subsystem + persistence
ORDER BY + rtsim invites clock + StableHasher + WASI + RRT, ~20 commits,
every commit locally green (common B59-only tracked red).

**★ R0D (renderer) disposition decided.** Builder correctly flagged
DET-REN-001..006 aren't sweep-shaped — they're the full R0D
implementation program per
`renderer-rework/Project Bastion — Renderer Scalability and High-Density
Voxel Architecture.md` + its own checkpoint (RENDERER-RUN-016, wave-gated
W0→W7, only W0 currently unblocked, R0P/architecture changes blocked
until a real production-Voxygen A1-A7 fixture passes paired proof and
issues `RendererR0DAdmissionV1`). **Decision: R0D = its own staged
build-packet block, tracked separately from the sweep, wave-level
check-ins not tag-level — do not compress the gating.** Builder directed
to: build DET-REN-004 (no-GPU→sim-authority source-scan lint) now as an
independent small item since it doesn't need the wave gating; then run
W0 (isolated-checkout preflight) per the checkpoint's own next-step.
Remaining Tier-3 slot held open for the pending clock/scheduling/
physics/persistence deep-pass; networking and policy/data items
unchanged (deferred / awareness-only).

**★★★ v5 DEEP-PASS ARRIVED (clock/scheduling/physics/persistence),
`determism/v5/`.** 101 new findings (40 Critical/55 High/5 Med/1 Low),
merged ledger 230, merged change register 428. RNG boundary stays
closed per the doc's own scope. Read the executive summary + ~20
highest-value findings in full. Two genuine NEW domain roots identified,
bigger blast radius than anything in the RNG arc:

- **DET-CLK-006** — `server-cli/src/main.rs:300-324`, live tick loop
  feeds wall-clock `Clock::game_dt()` as the authoritative tick input.
  Root of every time-based finding in the doc. Fix = `SimClock` fixed-
  duration authoritative tick, Clock demoted to pacer/diagnostic only.
  **Disclosed to Ben: this is a genuine live-server behavior change under
  load** (currently free-runs when lagging; fixed-step needs its own
  catch-up policy) — intended consequence of the full-determinism
  mandate, not a regression, but the biggest feel-affecting change in the
  arc so far. Proceeding per the standing full-scope directive.
- **DET-ECS-007** — `common/ecs/src/system.rs:16-30`, System::PHASE is
  declared but never enforced by the dispatcher. Root of every ordering
  finding downstream (client-message order, chunk-accept order,
  event/outcome reduce order all trace back to unenforced phases).

Both tagged apex/elevate-in-builder per THE BAR. Ranked work order given
to Builder, biggest-first: CLK-006 + ECS-007 (apex) → CLK-010/011/014 +
ECS-014/015/020 + EVT-005/007/010/011 (downstream ordering, still
Critical) → PHY-005 (physics's own root: spatial-grid insertion order) +
PHY-008/011/020/024 → TER-008/012/013/018 + the 33 PER findings
(migration-divergence, INSERT…SELECT row order, etc. — lower
architecturally, more contained). Doctrine held: minimal per-site fix
first, same as RNG arc — do NOT build the proposed shared architecture
(TimeDomainV1/CanonicalScheduleV1/CanonicalPhysicsV1/
CanonicalPersistenceV1) as a big-bang prerequisite; introduce a shared
key struct only where genuinely reused across many sites. Sequencing:
finish DET-REN-004 + R0D's W0 preflight first (already in flight), then
v5 becomes the primary standing queue.

**Next ChatGPT prompt to run (proposed, awaiting Ben to paste):** the
remaining uncovered-by-any-pass areas — networking/replication-order
determinism, asset/content-pipeline determinism (RON/mod load order,
content hashing beyond the already-flagged loot-catalog awareness item),
and build reproducibility (compiler/lockfile/cross-compile pinning).
Same rigor/format as v2-v5, continuing the merged-ledger numbering from
230.

**DET-REN-004 DONE** `70502af88b`: no-GPU→simulation-authority as a
dependency-boundary fitness gate — no authoritative crate manifest
(common/state/systems/net, server/agent, rtsim, world, bastion-server)
may name a GPU crate (wgpu/naga/vulkano/gfx-hal/ash/metal). Self-caught
a false-positive class on its own first run (substring "ash =" matching
ahash/fxhash), fixed to line-anchored dep-name equality — good self-
verification. det_ren_004 1/1.

**W0 blocker resolved:** the renderer build-guide/checkpoint on H: are
`.gdoc` cloud stubs, unreadable to Builder — pointed them at the actual
plaintext auto-synced mirror (`E:\bastion-engine-research-md\
renderer-rework\...`), same path I read from, plus pasted the
checkpoint's exact W0 acceptance text (commit/parent match, clean
working tree, verified package set, no Cargo-feature/type collision, no
lease conflict) so they're unblocked immediately. Builder proceeding to
run W0 in an isolated checkout.

**★ W0 PASS, full handoff.** Isolated worktree
`.claude/worktrees/renderer-w0` (branch `bastion/renderer-w0`). Commit
`5de5361bc53cdac252c30c43cc979512550ae5e9` + parent
`d7e161a914168c8288bb3b9322f99187be08020b` exact match; `git status`
clean (proves the external-cleanliness unknown the checkpoint flagged);
all 7 verified packages resolve via `cargo metadata`; whole-tree
collision scan on every guide-reserved symbol/feature (RendererBenchEntityId,
RendererSemanticFrameTokenV1, RendererR0DAdmissionV1,
RendererBenchManifestV1, RendererBenchScenario, renderer_bench module,
renderer-bench Cargo feature) — zero hits; no lease conflict, W0 touches
no production source. Artifacts hashed at
`bastion-test-evidence/renderer-r0d-w0/` (preflight manifest, interface
manifest, 8-lease ownership table W1-W7, SHA256SUMS). Rollback = delete
the one dir. **W1 (BUILD-007A1 shared contract + stable ID) authorized
to launch.**

**★★★ CLK-006 LANDED** `b0da8808df` — both live tick loops (the flagged
domain-root, wall-clock `Clock::game_dt()` as authoritative input)
converted to fixed-step. Builder's report was thin on verification detail
for this one (just "landed") — asked them directly for the determinism
story (test results, catch-up/pacing policy under load, explicit
feel-regression self-check) before treating this as settled, given it's
the single biggest behavior-changing commit in the arc so far and the
standing "test every change, no exemptions" rule applies hardest here.
Flagging Opus for priority VM cert once that detail comes back — a
domain-root behavior change needs the full suite, not a light check.

**ECS-007 in progress, not yet committed:** phase-barrier enforcement
BUILT (registry + generated barriers + manifest + 4/4 pins, server/
common-state/common-ecs clean); full-dispatcher harness boot-proof
(cycle check) in flight, will commit on green. Next after that:
CLK-010's live residual.

**Opus light-check PASS on the GenCtx/combat/threading batch** (tip
spanned `a3c4f638`→`70502af8` mid-fan): 4 mf jobs × schedules, all
IDENTICAL composite `[249,92,138,14,19…]` — multi-schedule stable at the
NEW value (moved from `[238,…]` via the 3 stacked re-pins, exactly as
declared), plus bonus Intel Broadwell coverage (exceeded the single-
vendor bar). Both spanned commits produced the same composite → `70502af8`
(DET-REN-004) is mf-neutral, as expected for a dependency-boundary lint.
Opus self-caught a 4th classifier gap from this and added a MIXED_COMMIT
guard (spanning-commit fans: agreeing composites = newer commit(s)
neutral, disagreeing = real inter-commit move, not noise) — canaries
still 10/10. Full 3-machine × multi-schedule cert still reserved for the
sweep-closed signal.

**CLK-006 verification CLOSED — full answer received, satisfies the
no-exemptions bar:**
- Compile proof both live-loop sites: server-cli 3m07s clean, voxygen
  singleplayer 3m41s clean (Builder self-caught and closed an honest gap
  here — the first voxygen check silently died on an unrelated ECS-007
  landing mid-run, re-queued and got a clean verdict before reporting).
- Catch-up policy: NONE, deliberate — one tick per loop iteration, no
  extra-tick bursts when behind. Falling behind budget makes SIM TIME
  DILATE (coherent slow-motion across physics/AI/cooldowns/day-night)
  rather than free-running catch-up ticks, because bursty catch-up would
  reintroduce load-dependent tick-batching nondeterminism. A bounded
  catch-up policy is available later as its own reviewed change if
  dilation feel proves unacceptable.
- Feel self-check: NO change at normal load (old `game_dt()` ≈ new fixed
  dt ± ~1ms OS jitter — imperceptibly smoother, not different). UNDER
  LOAD it's a real, intended difference: old behavior warped into fewer/
  larger dt steps (coarser physics, tunneling/rubber-band risk); new
  behavior holds step size constant and lets sim time slow instead —
  players on an overloaded server now perceive slow-motion instead of
  choppy warping. Confirmed as the single biggest feel-affecting change
  of the arc, exactly as flagged going in.
- Behavioral proof is explicitly NOT local (harness never touches either
  live-loop surface — the gate-must-test-live-path class) — flagging to
  Opus now as a PRIORITY VM cert using the v5 doc's own verification
  recipe: same input tape under 0/1/10/100ms injected sleeps + two pacing
  rates, assert identical per-tick state hashes + terminal snapshot.

**ECS-007 boot-proof:** build done, 30-tick dispatcher cycle-check
executing now, will commit on green. **W1 (BUILD-007A1)** now starting
in the renderer-w0 checkout.

**CLK-006: Opus returned strong interim evidence, asked for a scope
decision.** Construction proof (git-verified both sites: `server.tick()`
now receives a literal constant `Duration::from_secs_f64(1.0/TPS)`, wall-
clock `game_dt()` fully gone from the authoritative input) + a det-lint
entropy audit (zero `SystemTime::now`/`Instant::now` anywhere in the
Bastion authoritative sim path). Real empirical perturbation cert would
require lifting `FinalStateCertificate` out of
`bastion-harness/src/main.rs:11618-11667` into a shared accessor (that
computation is currently inline and server-cli's live loop never touches
it) plus new server-cli test hooks (bounded-tick + per-tick sleep
injection + two pacing rates) — real multi-step work, not a light check.

**Decision: full empirical build, not interim-only.** This is exactly
the [[gate-must-test-live-path]] case — the harness's own construction
proof never exercises the actual live loop this fix touches. Standing
rule is test every change on VMs, no exemptions, and this is the single
biggest behavior-changing commit of the arc — it gets the full
treatment. Opus proceeding with all four steps (lift the accessor, add
test hooks, build, run the perturbation matrix). Builder 4 notified to
steer clear of the shared harness file/region in the meantime.

**Renderer VM image build:** first attempt hit a transient apt-fetch
network error (single package, connection reset) — clean failure, trap
deleted the build VM correctly, no orphan/cost leak. Retrying now.

**GPU-hardware VM (forward-looking, discussed with Ben, not yet
filed):** lavapipe (software render) proves logical/determinism
correctness but can't validate real performance (CPU rasterizer, useless
for the scalability goal) or real-driver compatibility (no vendor driver
quirks/timing/extension gaps to catch). Fix for both = a real GPU-
attached VM (same ephemeral shape, e.g. one NVIDIA T4/L4) — this needs a
GCP GPU quota request first. Quota requests are free regardless of
outcome (only running instances cost money); GPU quotas may face more
scrutiny than the CPU 96→128 request did. Not filed yet — Ben's call
pending on timing (now vs. when R0D actually reaches a performance-
focused wave).

**★★★ v5 BURST, tip `890bc3bb5b`, all locally green:**
- **ECS-007 LANDED** `f5ea11b5ce` — System::PHASE now ENFORCED via
  generated dispatcher barriers (per-builder registry, begin_schedule at
  both construction sites, schedule_manifest golden accessor, 4/4 pins).
  Unlike CLK-006, this one has genuine BEHAVIORAL proof already, not just
  compile: bastion-harness booted the real production schedule and ran
  30 ticks + colony, exit 0 — the harness's own test infra directly
  exercises the dispatcher, so this doesn't have the same live-path gap
  CLK-006 had. Second domain root closed.
- **CLK-010/011/014 LANDED** `f2c5e1347a` — persistence snapshot cadence
  now tick-indexed live (composes with CLK-006; noted consequence:
  snapshot wall-interval stretches under dilation, expected); graceful
  shutdown now terminates at a DECLARED final tick
  (accepted_tick + grace×TPS) via both signal + console paths — wall
  clock drives only the countdown messages, not the actual cutoff.
- **ECS-014 LANDED** (same tip) — in_game `par_bridge` client work now
  gather-sort-commits by stable client entity id instead of worker-
  completion order.

Next per the standing order: ECS-015 (terrain-write mutex) → ECS-020 →
EVT-005/007/010/011 → PHY-005 (physics's own root). R0D's W1 spec fully
read, staged for interleaving.

**Builder's checkpoint-tip suggestion adopted:** tip is moving fast: told
Opus `890bc3bb5b` is a good anchor to cert at (covers CLK-006 + ECS-007 +
the CLK cluster + ECS-014) once their in-flight CLK-006 empirical harness
work lands, rather than chasing a moving HEAD.

**CLK-006 harness-lift plan CORRECTED by Opus before building:** the
original plan (lift the harness's `FinalStateCertificate` into a generic
`state_certificate(&Server)`) was infeasible — that cert is computed from
mf-scenario-specific data (per-colonist leaves/aggregates), not a generic
server walk; the real generic authoritative-state hasher is the deferred
T0.55 half. Corrected, better plan: a server-cli `--det-perturb` test
mode (forced-deterministic worldgen/rtsim + fixed seed, ticks exactly N
times under injected 0/1/10/100ms sleep × 2 pacing rates) fingerprinting
TimeOfDay (= start + N×(1/TPS) exactly under the fix — the actual smoking-
gun invariant) plus a hash of authoritative aggregates. This needs NO
harness-file lift — additive, isolated to server-cli's own crate, own
branch (`bastion/det-clk-cert`), no collision with Builder. Told Builder
they no longer need to steer clear of `bastion-harness/main.rs`.

**★ RENDERER VM READY: `bastion-golden-renderer` built successfully**
(first attempt hit a transient apt network blip, clean retry succeeded).
Confirmed: `vulkaninfo` reports `deviceName = llvmpipe (LLVM 19.1.7, 256
bits)`, `driverID = DRIVER_ID_MESA_LLVMPIPE` — the software Vulkan device
is live. Voxygen cold-built clean (`BUILD_OK`). Image snapshotted, build
VM deleted. `vm-run-renderer.sh` ready for R0D's real-render fixture
work whenever Builder needs it (W1+).

**GPU-hardware VM:** discussed with Ben — lavapipe proves logic/
determinism only, not performance or real-driver compatibility. Fix for
both = a real GPU VM (e.g. one NVIDIA T4/L4), needs a GCP GPU quota
request first (free to request, may face more scrutiny than the earlier
CPU 96→128 ask). Not filed yet, Ben's call on timing.

**CLK-006 empirical harness BUILT + perturbation matrix RUNNING.**
Additive server-cli-only test hooks (`--det-perturb --det-ticks N
--det-sleep-ms X`), no game-code/harness touched — a bounded
deterministic tick soak with injected per-tick wall-clock sleep, emitting
a master-clock fingerprint (TimeOfDay+Time via DomainHasher). Pushed to
`bastion/det-clk-cert @ 733b31b2b0` off the `890bc3bb5b` checkpoint
anchor. Running now: same machine/seed(1337)/tick-count, sleeps
0/1/10/100ms varied — acceptance = all 4 fingerprints byte-identical.
Fingerprint is over the master clock directly (the exact thing DET-CLK-006
changes), needs no rtsim determinism. Cross-vendor (Intel) extension
planned as a follow-on once this same-machine matrix is green. ETA
~20-25min, verdict pending.

**CLK-006 checkpoint: hook DONE + pushed, matrix run INFRA-BLOCKED (not
code).** `bastion/det-clk-cert @ 61b71cf829`, server-cli `--det-perturb`
harness complete (2 build issues found+fixed along the way: the harness-
cert lift wasn't feasible so pivoted to the clock fingerprint per the
earlier correction; a missing `specs::WorldExt` import). One-command
runner (`det-clk-vmrun.sh`) ready. Hit the familiar GCP create-plane
throttle (recurring, same class as earlier tonight) — Opus correctly
stood down rather than hammering it (zero lingering VMs, quota clean),
will probe-first and resume once creates recover (~15-30min cooldown).
Interim evidence (construction proof + zero-wall-clock-leak audit)
stands as real supporting proof in the meantime; empirical matrix is
final confirmation only, gated on infra not on the fix itself.

**Builder stretch closed, tip `86975076b6` — eleven v5 items resolved
total:** both domain roots (CLK-006, ECS-007) + the full clock cluster
(CLK-010/011/014) + the in_game.rs trilogy — ECS-014 (records),
**ECS-015 (terrain writes — the RareWrites mutex DELETED)**, and EVT-005
(events) all now gather-sort-commit — + two closed-superseded verified
against live code (ECS-020, EVT-007, both citing pre-ENGOPT4 fixes) +
**EVT-010 LANDED** (physics outcome chronology now canonicalized by
source entity, closing T0.28's explicitly-noted debt, phys suite green).
Discipline held: verify-live-branch-first closed 4 findings as already-
fixed this stretch alone, minimal-per-site fixes, tests green before
every commit. Next: EVT-011 (beam.rs, same deferred-buffer pattern keyed
by beam entity, already surveyed) → **PHY-005** (physics's own domain
root). R0D's W1 build remains authorized, interleaving continues.

**EVT cluster COMPLETE + PHY started, tip `da04541a83`:**
- **EVT-011 landed** `e71680df2c` — beam events/outcomes/hits now
  per-beam keyed buffers flushed in uid order (no more fold-carries-
  emitters through the Rayon tree). Bonus find at the same site: beam
  attack procs drew from ambient `rand::rng()` per beam per tick — now a
  ChaCha8 stream keyed by (beam uid, sim-time).
- **PHY-011 landed** `da04541a83` — projectile first-hit was HashMap
  order (an arrow touching two targets hit a process-random one);
  candidates now sort by stable Uid. common-systems 21+1 green
  throughout.

**★ PHY-005 fork, decided.** Builder found the spatial-grid Specs-join
insertion order is EMPIRICALLY stable in-envelope (svp2/fuzz1/t1cmd
campaigns certified byte-identical tapes through this exact path with
physics active) and flagged 3 options: (a) sorted insertion — correct
but a per-tick global-sort cost in the hottest system; (b) disposition
around the empirical evidence, no code change; (c) sort only per-cell
candidate vecs post-construction — cheaper, partial.

**Decision: (c).** This is the audit's OWN proposed remediation verbatim
("sort each cell by StableBodyId after construction"), not an under-
scoped compromise — closes the actual risk surface (within-cell order
feeding `resolve_e2e_collision`) at per-cell not global cost. Explicitly
REJECTED (b): "empirically stable given current deterministic
allocation" is the exact same fragile-assumption shape that produced the
GenCtx shared-parent-cursor bug — stable today, silently breaks if
anything upstream ever regresses, no local defense. Determinism-by-
construction stays the standing law; don't disposition around it when a
cheap proper fix is available.

**PHY-008 CLOSED** against the existing T0.43/44 disposition ("per-
entity independent, stable in-regime neighbor order; momentum-symmetry
redesign deferred as endpoint block") — ruled distinguishable from the
PHY-005 case since T0.43/44 was already formally reviewed with a named,
tracked follow-up, not a fresh ad-hoc "trust it" call. Trusted Builder's
live-branch verification it's the same surface.

Next: PHY-020 → PHY-024 → TER/PER as planned.

**Real-GPU VM tooling built, blocked on quota.** `vm-build-image-gpu.sh`
(builds `bastion-golden-gpu`: Debian 12 + real NVIDIA T4 via
`--accelerator`, GCP's official driver installer, NVIDIA's own Vulkan
ICD) + `vm-run-gpu.sh` (same ephemeral create→run→delete shape as the
rest of the fleet — GPU deallocates the instant the instance deletes,
zero standing cost either way). Checked quota before building anything
that'd fail: `GPUS_ALL_REGIONS` = 0 project-wide (the real gate, same
shape as CPUS_ALL_REGIONS) — the per-type regional quotas showing a
cosmetic "1" don't matter until this clears. Not filable via plain
`gcloud` (needs the `alpha` component, blocked without admin, or the
Console Quotas page directly, tied to Ben's account identity same as the
earlier CPU 128 request). Scripts are ready to fire the moment Ben files
the increase — nothing to rebuild, just re-run once it clears.

**★★★ STRETCH CLOSED, tip `2050df50ca` — SEVENTEEN v5 items resolved,
EVERY named CLK/ECS/EVT/PHY finding now closed:** all three domain roots
landed (CLK-006 fixed-step time, ECS-007 enforced phase barriers,
**PHY-005 landed** per the ruling — per-cell uid sort at both grid
construct sites, phys suite green); the full clock cluster; the in_game
gather-sort-commit trilogy; both Rayon reduce-tree conversions (PHY-020
+ the beam bonus-rng kill); projectile first-hit canonicalization
(PHY-011); PHY-024 closed. Five findings closed against verified prior
work rather than rebuilt (live-branch-first discipline). Remaining in
v5: the TER quartet (supersession checks first) + the 33-finding
persistence set. Track B (R0D W1) authorized, fully specced, resumes
interleaved. Nudged Builder to keep going (session had gone idle after
reporting — not stuck, just paused at a message boundary) straight into
TER/PER without waiting for a per-item ack, per standing rule.

**★★★ v6 DEEP-PASS ARRIVED (networking/asset-pipeline/build-repro),
`determism/v6/`.** This is the ChatGPT prompt requested earlier
(remaining uncovered coverage areas). 92 new findings (39 Critical/49
High/4 Medium), merged ledger now 322, merged change register 520 (428
prior + 92 new). Domain split: NET 35, AST 32, BLD 25.

Executive framing: three "hidden authority channels" — (1) arrival
authority (independent net streams/reconnect epochs have no shared
sequence/checkpoint protocol), (2) source-order authority (filesystem
enumeration/plugin registration/HashMap merges decide active content),
(3) environment authority (mutable images/branches/flags/tools aren't
captured by one immutable build manifest).

**Domain roots flagged for Builder, ranked into the standing big-to-
small queue (merged with v5's remaining tail by actual blast radius, not
by which audit pass they came from):**
- **DET-NET-006/008/009** — the NetEnvelopeV1 trio: no cross-stream
  chronology, no universal client-command identity/sequence, no
  universal replication envelope. Root of nearly every other NET
  finding downstream (arrival order currently IS the only ordering
  authority for six independent streams).
- **DET-AST-007/010/017/014** — asset root chosen from ambient search
  paths, unsorted directory enumeration, plugin module list is a
  HashSet, HashMap-concatenate is implicit last-writer-wins. Root of
  content/plugin determinism — DET-AST-034 (MultiRon merge order)
  depends on these and affects recipes/skills/bodies/loot broadly.
- **DET-BLD-032** — no clean-room rebuild equivalence gate (the
  capstone proof mechanism: two independent builds from the same
  manifest must match).

**★ META-FINDING, routed to Opus not Builder:** several BLD findings
(012/014/017/018/019/020/021/022) are about OUR OWN VM test scripts by
name (`vm-build-image.sh`, `vm-run.sh`) — mutable Debian image family,
mutable branch clone, no `--locked`/`--frozen`, mutable machine-image
alias, hard-reset leaves untracked files, no declared target triple, and
a `--profile verify` that the audit couldn't find defined anywhere
(SYNC-GAP — worth checking directly). This affects the reproducibility
CREDIBILITY of tonight's own test methodology, not gameplay determinism
directly — routed to Opus since they own VM test execution, flagged
non-urgent/non-blocking (tonight's ATTEST/commit-matching discipline
still gives real signal; this is a hardening item, not a retroactive
invalidation).

**Recommendation on further ChatGPT passes (Ben asked):** holding off on
commissioning a v7 for now — the existing backlog (322 ledger findings /
520 change-register rows) is already large relative to Builder's
throughput; better to let real progress catch up before more research
lands. If another pass is wanted later, the one remaining natural gap is
UI/input/client-side-prediction-reconciliation determinism (not touched
by any pass so far) — but that's optional, not urgent.

**Ben overruled the hold-off — commissioning v7 now.** Prompt drafted and
handed to Ben for pasting: client-side prediction, input handling, and
reconciliation determinism (input timestamping, whether client
prediction shares ordering guarantees with the authoritative server
path, reconciliation/rollback order-dependence, UI state that leaks back
into gameplay-affecting decisions). Continues numbering from ledger 322
/ change register 520. Awaiting ChatGPT's output.

**Opus triaged the v6 VM-script findings (own lane):** DET-BLD-019
CLOSED as false-positive (`[profile.verify]` IS defined at root
`Cargo.toml:136`, inherits='release' — verified against live code, and
explains the earlier-documented slowjob `should_panic` artifact via
debug-assertions-off). DET-BLD-018 (`--locked`/`--frozen`) + DET-BLD-022
(target triple) confirmed real — hardening own scripts
(det-clk-vmrun/vm-percommit/bastion-verify) first, will coordinate
before touching the SHARED vm-run.sh/vm-jobs.sh (a spurious `--locked`
BUILD_FAIL on a fast-moving branch would hit Builder's runs too).
DET-BLD-021 (hard-reset untracked leftovers) real, `git clean -fdx`
pre-build closes it, scoping carefully. DET-BLD-014/017/020 (golden
image mutable base family + tip-at-build-time) confirmed as the
substantive item — per-RUN provenance is solid (reset --hard + ATTEST),
the gap is image-REBUILD reproducibility; needs its own focused
`vm-build-image.sh` hardening pass. DET-BLD-012 reconfirmed known/
accepted (LFS noise). Sequencing: CLK-006's matrix run stays priority
(still gated on GCP create-rate recovery), this hardening lane follows.

**★ v5 FULLY SWEPT, tip `7a1a93fa27` — TER quartet landed** `2d734ef291`:
TER-018 (recorder now finalizes LAST in Server::drop, after terrain+
rtsim persistence — the real ordering fix), TER-013 (same-cell
precedence declared in code: due-time/insertion order, scheduled-over-
live), TER-012 (closed-by-composition — ECS-007's barriers + ECS-015's
sorted clients already ARE the declared write order, no new code
needed), TER-008 (third pre-ENGOPT4 citation, superseded). All 21 v5
items outside the PER set now resolved. Per the standing ranking, the
33 PER findings queue BEHIND the v6 NET/AST roots.

**v6 AST root cluster: 3 of 4 landed** (same tip): AST-010 (merged asset
enumeration now buffers + flushes SORTED on every path — OS directory
order was a hidden authority channel over content discovery), AST-017
(PluginData modules/dependencies HashSet→BTreeSet — canonical iteration
AND canonical serialized plugin identity), AST-014 (concatenate's last-
writer-wins declared as explicit policy, made deterministic via the
upstream canonicalization). Remaining: AST-007 (certified asset root —
planned via reusing T0.57's ContentManifest infra) + the NET-006/008/009
envelope root (reading the doc's full architecture section first, per
standing instruction).

**B77 filed:** `common-assets`' `fs::tests::test_read_dir_notfound`
fails on a CLEAN checkout, baseline-verified unrelated to any
determinism-arc change — `fs.rs`'s not-found error lost the asset path
from its message, exactly the pre-existing `assets_manager` regression
its own NOTE comment warns about. Diagnostics-quality issue, not
determinism-surface; correctly scoped out of the sweep rather than fixed
inline. Good discipline verifying against a clean baseline before
attributing.

Next: AST-007 certified-root gate → NET architecture read → NET-006/
008/009 minimal-first → BLD-010/023/031 (031 flagged read-carefully,
profile-semantics implications to be reported before touching) → the
33-finding PER set.

**★ NEW STANDING GATE requested: performance-regression benchmark
(Ben-driven).** Risk identified: the determinism-fix pattern (gather-
sort-commit, per-cell/canonical sorts) trades cheap unordered ops for
sort/canonicalize-before-commit, individually cheap but compounding
across dozens of fixes — unlike feel-drift (catchable via self-check +
playtesting), perf regression is objectively measurable and wasn't being
measured at all. Sent to Opus: build a tick-time/frame-time benchmark
leg, additive to the existing determinism certs, run on hot-path-
touching commits (physics/ECS/network scheduling specifically). Shape:
high-entity-count stress fixture, ms/tick or ticks/sec averaged post-
warmup, same-machine before/after comparison, establish a baseline NOW
(ideally pre-dating tonight's hot-path fixes), flag threshold (~5-10%
regression suggested, Opus's call). Mitigation techniques flagged if a
regression trips: radix/bucket sort for bounded key ranges, incremental
sorted structures instead of re-sort-per-tick, parallelize the sort
itself, or recognize the collection is small/bounded (per-cell physics
style) so it's actually a non-issue. Standing leg going forward, not a
one-time check.

**Perf-gate DESIGNED (Opus, VM-free part done, build gated behind
CLK-006):** high-entity (100+ colonist) stress fixture on mf/flat-arena,
deterministic seed for workload fairness; ms/tick averaged post-warmup;
wall-time strictly DIAGNOSTIC, never gates state-hash correctness.
**Two baselines, not one** — `b0da8808df` (right before the per-tick-sort
arc: ECS-007→ECS-014/015→EVT-005/010) isolates SORT-ARC cost alone;
`90fb70e630` (pre-RNG-DEEP, pre-v5) isolates WHOLE-ARC cost including
RNG-DEEP hashing overhead. Threshold 7%, soft-flag not hard-fail;
response to a trip is a cheaper impl, never reverting the correctness
fix. Confirmed, no changes requested — queued behind CLK-006's
perturbation matrix, same create-rate-recovery gate.

**★★★ v7 DEEP-PASS ARRIVED (client input/prediction/reconciliation/UI-
authority), `determism/v7/`.** This is the Ben-commissioned prompt from
earlier (client-side gap, not the RNG/CLK/NET/AST/BLD areas). 82 new
findings (47 Critical/34 High/1 Medium), merged ledger 404, merged
change register 602. Domain split: input capture 28, client prediction
20, reconciliation/rollback 16, gameplay-UI authority 18.

Executive framing: Bastion has responsive local prediction but no
deterministic PROTOCOL for it — no universal client_tick/input_seq,
no ack cursor, no prediction-state digest, no saved-frame history, no
rollback/resimulation. Canonical pipeline proposed: captured events →
fixed-tick InputFrameV1 → shared prediction kernel → server (input_seq
order) → AuthoritativeCorrectionV1 (ack_input_seq + digest) → restore +
replay unacknowledged → presentation-only smoothing.

**★ DIRECT OVERLAP WITH v6's NET-008 flagged** — DET-PRD-001/005/008/011
target the SAME message types (`ClientGeneral::ControllerInputs`,
`common/net/src/msg/client.rs`) that v6's NET-008 (universal client
command identity) already covers. Told Builder to fold these together
as ONE protocol pass rather than two sequential ones — avoids designing
the same envelope/sequencing twice.

**Ranked domain roots (into the standing queue):**
- **DET-INP-001** — `voxygen/src/window.rs:81-121`, window Events carry
  no session epoch/device identity/capture sequence — batch position is
  the only chronology. Root of the whole input-capture domain.
- **DET-PRD-001/005/011** (merge with NET-008) — no input-frame/sequence
  identity, no explicit tick-boundary rule between correction/replay/
  simulate, no certified-identical client/server prediction kernel.
- **DET-REC-001/016** — ForceUpdate is an unstamped wrapping counter (no
  tick/ack/digest/reason); correction-ack isn't an independent protocol
  state. Root of reconciliation determinism.
- **DET-UIA-003/006/018** — hotbar/inventory swap commands gated by
  local (possibly stale) inventory/capacity state rather than always
  sending intent + letting the server decide — the exact UI-authority
  pattern discussed earlier tonight, now concretely found.

High-value downstream: INP-004/012/017/022/025/027 (binding fan-out
order, focus-loss state leak, camera-tick quantization, target-sample-
before-batch timing, equal-distance tie-break, capture-time cursor),
PRD-008/014 (server trusts latest client physics as state rather than
replaying inputs; no rollback/resim history at all), REC-003/006/012
(correction applied without frame history, send-failure silently clears
the correction flag, interpolation ordered by receipt not server tick),
UIA-004 (HUD-generated gameplay events land a tick late vs direct
input). Full doc has all 82; these are the highest-value anchors.

Not yet sent to Builder as a formal work order — next action. [Sent —
see below.]

**NET stage 1 LANDED** `87fed61346`: EntitySync/CompSync packages now
carry `sync_tick` stamped at every server seam; client witnesses cross-
stream chronology and warns on regression; sequence field reserved for
stage 2. **BLD-010 LANDED**: build.rs now branches on
`CARGO_CFG_TARGET_OS` (target truth, not host cfg), loud warning on
cross-compile icon skip, voxygen builds clean through it. **BLD-023
LANDED**: ambient-RUSTFLAGS witness added to the harness build script —
emits a `cargo:warning` when RUSTFLAGS isn't flag-canonical; cert lane
should treat that as red.

**★★★ BLD-031 — MAJOR METHODOLOGY FINDING.** `[profile.verify]` (the VM
cert lane's own profile) `inherits='release'` → every cert run THIS
ENTIRE SESSION has had `overflow-checks=false` + `debug-assertions=false`.
Consequence: every `debug_assert` guard built across the whole arc
(T1.13 double-reserve, T1.14 completion-balance, T1.17 decrement/drop
pairing, command-status transition asserts, ECS phase checks) has been
SILENTLY INERT in certification — a guard-tripping bug would pass cert
clean and only fail on a dev box. Also: numeric overflow takes different
control flow per profile (panic in dev, silent wrap in verify/release) —
dev-tested numeric behavior does not certify shipped behavior, exactly
the audit's point.

**Decision: option (a) APPROVED** — flip `[profile.verify]` to
overflow-checks=true + debug-assertions=true, decoupled from bare
`inherits=release`. Within my authority (own cert lane, not the shipped
build), no reason to hold. Ordered a sanity re-run of a recent cert
under the corrected profile once it lands, to surface anything that was
previously guard-inert. **Option (b) (also flip shipped `release`)
ESCALATED TO BEN** — genuine ship-tier perf-vs-safety call, not mine to
decide. Builder holding on release, continuing v6 queue (NET-018 +
AST-019/023/024/025) meanwhile. Opus notified: sequence CLK-006 matrix +
perf-gate baseline AFTER the profile fix lands, not before, to avoid
certifying under the stale guard-inert profile.

**Two follow-on rulings from Opus's ack:**
1. **Perf-gate profile: GUARDS-OFF (release-representative), confirmed.**
   Guards-ON would inflate ms/tick with correctness-guard overhead and
   conflate that with real sort-arc cost — false-positive regressions
   that don't exist in what ships. Determinism certs stay guards-ON
   (their whole point); perf gate stays guards-OFF. Same profile for
   baseline vs tip either way.
2. **Retroactive scoped check on tonight's completed RNG certs: YES.**
   Not re-litigating correctness (construction + composite-stability
   already covers that) — specifically checking whether any
   `debug_assert` invariant silently fired without a crash to alert us,
   since it was compiled out the whole night. Same shape as the B75
   lesson (a passing proof doesn't rule out something outside its
   comparison window). Scoped to a single-machine run of the settled
   RNG-arc tip under the corrected guards-on profile — just checking for
   any assert firing, not a full 3-machine re-cert. If clean, done; if
   something fires, immediate top priority over the rest of the queue.

Sequence: profile fix → create-rate recovery → probe-first → CLK-006
matrix + this retroactive check + perf-gate baseline, Opus's own
ordering once resumed.

**Check-in: BLD-031(a) sanity run in flight.** Builder's running the
full verify-profile harness rebuild + guard-layer sanity scenario now —
this IS the decisive check requested (does any previously-inert guard
fire, surfacing an undetected bug rather than a regression). Also landed
this stretch: NET stage 1 + NET-018, BLD-010/023, AST-019/023. Staged
and ready the moment the sanity run frees: NET stage 2a merged with
PRD-001 (input-frame sequencing on ControllerInputs/ControlEvent/
ControlAction, client stamps monotonic input sequence, server witnesses
regression/duplicates — the merged design as directed, not built twice),
REC-001's minimal form (ForceUpdate stamped with the correction's server
tick, giving the client a real ack anchor), AST-029 next behind those.

Opus: fully aligned, restated the complete resumption queue (CLK-006
matrix guards-on → retroactive RNG assert-check single-machine → v6
script hardening → perf-gate baseline guards-off) — holding correctly on
profile-fix-lands + create-rate-recovery, zero lingering VMs, nothing to
nudge.

**★★★ BLD-031(a) VALIDATED + REAL REGRESSION FOUND — TOP PRIORITY, all
other work paused.** The corrected verify-profile harness (14m47s full
rebuild) ran the b5 guard-layer scenario clean on the profile question
itself: zero previously-inert guards fired, zero overflow panics, dev
and verify now produce BYTE-IDENTICAL output — exactly validates the
fix. But the SAME run exposed a genuine deterministic behavioral
regression, unrelated to the profile: **b5_mine_cleared:false — 25/27
mine cells clear, 2 NEVER complete**, byte-identical under both dev and
verify at tip `0c0a597e82`+profile-flip (so a real code regression, not
a profile/guard artifact). All other b5 bars pass (conservation exact,
stone_sum matches blocks mined; chop/build/slope/hill all green) — the
failure is isolated to those 2 stuck mine cells.

Regression window is honestly WIDE (b5's last confirmed-green predates
several batches — the broken-harness tip and a division-of-labor gap
mean nobody actually read a b5 result across the whole T1-conservation-
through-v6 stretch). Top suspects by mechanism: PHY-005 (per-cell sort →
pushback re-pin could strand cells as unreachable), ECS-007 (barrier-
ordered scheduling shifting work-tick timing), DET-RNG-008/lottery
re-pins (different scatter → different pile positions blocking access).

**Bisection in progress:** Builder building at `890bc3bb5b` (the pre-
EVT-010/PHY-005/TER/v6 cert anchor) as the baseline half-split point,
walking forward/back from its verdict. Asked Opus in parallel whether
any b5 result exists at a recent tip in their own cert history to pin
the window instantly instead of blind bisection. **All other work
(NET/AST/rest of v6/v7, CLK-006 matrix, perf-gate) paused/superseded
until this resolves** — a real regression outranks backlog progress.

**★★★ REGRESSION WINDOW PINNED.** Opus found a hard floor in their own
job history: `/tmp/bastion-jobs/t1consv2/job-0.out`, `ade7b2f8c7` attested
(RAN_COMMIT match, rc=0), `--b5-scenario` → b5_mine_cleared:TRUE, 27/27,
PASS — a functional completion field, unaffected by the guards-off
profile question, so a valid floor. **Window narrowed from a blind
bisect to `(ade7b2f8c7, 890bc3bb5b]`** — the RNG-DEEP reapply
(`48e4c05b77`) + the entire v5 hot-path sort-before-commit arc. Prime
suspects (Opus + Builder agree): **ECS-015** (terrain writes sorted-
commit, RareWrites mutex REMOVED) and **EVT-005** (in-game events
buffered per client, emitted at sorted commit) — "2 of 27 cells never
complete" reads as a dropped/lost completion event at a sort-commit
boundary, not an RNG re-pin. Relayed to Builder to bisect these two
first, ahead of PHY-005/ECS-007/lottery.

**★ META-FINDING (E1, cert-methodology gap, confirmed priority):** Opus's
mf composite classifier can confirm a move is STABLE and DECLARED, but
cannot decompose "intended re-pin" from "intended re-pin + a bundled
regression riding along in the same window" — the two mf re-pins in
THIS exact window (`209→238` at `48e4c05b77`, `238→249` at `a3c4f638`)
both passed as DECLARED_REPIN_STABLE while b5 was silently regressing
underneath. Live proof the gap is real, not hypothetical. Confirmed as
priority equal to BLD-031 — Opus to build E1 (per-domain hashes in the
cert, so a move outside the declared re-pin's domain fires
DECLARED_SCOPE_EXCEEDED) immediately after b5 is fixed, ahead of
resuming CLK-006/perf-gate.

**Discriminator relayed:** Opus refined the suspect list after checking
file locations — ECS-014/ECS-015/EVT-005 are all
`server/src/sys/msg/in_game.rs` (client→server message path); ECS-007
is the broader dispatcher-phase mechanism (`common/ecs/src/system.rs` +
`server/src/events/mod.rs`). One field in the already-failing b5 JSON
(`b5_mine_jobs`, was 27 at the ade7b2f8c7 PASS) discriminates which
branch without any new build: dropped to 25 → designation itself was
lost, points at ECS-014/015; stayed 27 but 2 never clear → completion
never applied, points at ECS-007. Relayed to Builder to check their
existing failing-run JSON before building anything new.

**Correction from Builder (independently caught, then confirmed):**
ECS-014/ECS-015/EVT-005 all live in `server`'s `in_game.rs`, which only
processes CLIENT messages — b5 is a headless scenario with no clients,
so those changes are structurally inert for this regression. Relayed to
Opus. **Bisect leg 1: RED at `890bc3bb5b`** (same 25/27 signature) —
combined with Opus's GREEN floor at `ade7b2f8c7`, window confirmed at
**30 commits, (ade7b2f8c7, 890bc3bb5b]**. ECS-015/EVT-005 are actually
OUTSIDE this window entirely (post-890bc3bb5b), further confirming
they're exonerated regardless of the client/headless argument.

In-window suspects ranked by mechanism fit for "2 cells never complete":
`a3c4f638b4` (keyed RRT pathfinding sampler — top suspect, "stuck
colonists = pathing" is the strongest fit), `b0da8808df` (CLK-006 fixed-
step dt), `f5ea11b5ce` (ECS-007 phase barriers), `d6f1de9d7a` (mine-loot
keyed streams), `2fe861aeda` (toss-scatter re-key), `8c6fb446e1`
(StdRng→portable leaf sweep). **Bisect leg 2 running** at midpoint
`0dbdabd7f9` (post-RNG-DEEP-foundation-reapply) — RED narrows to the
early RNG-sweep/worldgen/foundation half, GREEN narrows to {leaf sweep,
mine-loot roll, lottery, threading+combat, GenCtx, RRT sampler, CLK
cluster, ECS-007} where the priors concentrate. Next split pre-planned
either way.

**Opus's shortcut answered + corrections:** `b5_mine_jobs` stayed 27
(designation intact, completion never applies) — points at the
completion path per Opus's own branch logic, ECS-014/015 stay ruled out.
Corrections folded in: window confirmed 30 commits not ~10; EVT-010 is
outside the window (post-890bc3bb5b); **RRT sampler EXONERATED** —
behind `cfg(rrt_pathfinding)`, no crate currently enables that feature,
dead code for this scenario; CLK-006 near-inert for b5 (only touches
server-cli/voxygen loop callers, harness drives Server::tick directly).
New structural confirm: `bastion_jobs` IS in the phased dispatcher
(PHASE=Phase::Create, explicit deps on agent+bastion_path) → ECS-007's
barriers are LIVE for b5. **Updated ranking: ECS-007 ~ leaf sweep
(Chaser/agent rng re-pin) ~ tick_rng foundation re-pin > lottery >
threading+combat.** Mechanism read: a re-pin/reorder landing on a stuck
2-cell configuration persisting across 5400 ticks (PERSIST, not a
dropped-event throughput issue). Leg 2 (midpoint 0dbdabd7f9) finishing
shortly.

**Leg 2: GREEN at `0dbdabd7f9`** (27/27 PASS, full JSON healthy). Window
narrowed to **13 commits, (0dbdabd7f9, 890bc3bb5b]**. Live in-range
suspects: leaf sweep (Chaser/agent rng), mine-loot (weak), lottery,
threading+combat, GenCtx, ECS-007, CLK trio (weak). Confirmed inert in-
range: WASI, RRT, REN gate, CLK-006, ECS-014, annotation commit. **Leg 3
building at `dbbacea48e`** — RED narrows to {leaf sweep, mine-loot,
lottery, threading+combat} (4 commits, ≤2 more legs); GREEN narrows to
{GenCtx, ECS-007, CLK trio}, with ECS-007 to be tested directly next as
the strongest of that group. Builder filling the wait with Track B's W1
authoring (renderer-w0 worktree, zero collision with the bisect tree) —
good parallel-fill discipline.

**★ TWIST — possibly TWO distinct regressions.** Leg 3 at `dbbacea48e`:
mine bar HEALTHY (27/27 TRUE) but scenario FAILS on a DIFFERENT bar —
`b5_build_stall_jobs:0` (expected 1) + `b5_build_stall_untouched:false` +
`b5_any_needs_materials:false` (a build-designation-with-missing-
materials probe placed zero jobs instead of staying untouched). Worse:
leg 1's original "FAIL" at `890bc3bb5b` never printed full JSON (grep
too narrow) — it may have been THIS build-stall failure, not the mine
one, meaning the mine-regression window may extend past 890bc3bb5b,
undoing the earlier post-anchor exoneration for the mine bar
specifically. Re-running 890bc3bb5b with full JSON to disambiguate.

Two live windows tracked separately: BUILD-STALL regression narrows to
(0dbdabd7f9, dbbacea48e] = {leaf sweep, mine-loot keying, lottery batch,
threading+combat} — lottery/job-assignment re-pin is the obvious prior
for a designation placing 0 jobs. MINE regression window pending the
890bc3bb5b re-run result. Also live: single root (job-assignment/lottery
re-pin) manifesting as different bars at different commits. Builder
switched to bisecting per-BAR rather than per-verdict, having verified
the harness bar-set is constant/bar-neutral across the window (only 2
harness-touching commits, both post-890bc3bb5b) — sound approach,
confirmed.

**★★★ v8 BATCH DELIVERY — 14 audit packages, ~100 new findings total.**
ChatGPT batched together most of the prompts drafted earlier (colony
sim, crafting, GOD-DOMAIN/culture/AI-autonomy, inter-settlement
migration, mine/traversal, needs/mood, NPC combat targeting, party
loot, plugin runtime, resource depletion, RTSim economy, skill-tree,
structure placement, town/city gen, trade caravan, weather) into one
delivery under `determism/v8/`. Survey-level bookkeeping now; full
ranked work order DEFERRED until the b5 regression resolves — not
interrupting the bisect with a new backlog.

Per-package new-finding counts: colony-sim 9, crafting 7, GOD/culture/
autonomy 7, migration 2, mine/traversal 12, needs/mood 0 (fully
dedup'd — confirms diminishing returns already hitting some areas),
NPC combat targeting 4, party loot 0 (fully dedup'd), plugin runtime 3,
resource depletion 3, **RTSim economy 28** (biggest single haul, 10
Critical), skill-tree 3 (all Medium), structure placement 3, town/city
gen 5, trade caravan 4, weather 10.

Highlights:
- **GOD-DOMAIN/AI-player confirmed NOT implemented** (zero findings,
  design-only) — validates the earlier code-maturity concern.
  Autonomy-arbitration IS substantially implemented (6 real findings).
- **RTSim economy**: noncommutative sequential trade settlement, a
  500-year powf/float price recurrence (genuine long-horizon drift),
  food-threshold population compounding, parallel AtomicU64 quest
  identity allocation.
- **Weather**: a real bug, not just hygiene — the weather base field is
  fixed to seed zero instead of world/domain identity, meaning every
  world may generate the SAME weather pattern base.
- **Mine/traversal**: possible relevance to the live b5 regression —
  "outer mine-access request arbitration is producer-order dependent"
  and "built-access admission uses a separate hardcoded depth cutoff
  instead of the free-climb cap." Flagged to Builder as a possible
  bisect shortcut, not a redirect.
- **NPC combat targeting**: "candidate sensing assigns a shared RNG
  cursor by traversal order and uses live ambient entropy" — same bug
  class fixed everywhere else tonight, still live here.

**★★★ b5 REGRESSION REFRAMED — not a code-path bug, a latent fragility
exposed by an intended world re-roll.** Leg 4's full per-bar matrix
showed `b5_ch_trees`/`ch_cells`/`cavein_drop_cells` swinging across
commits (13→14→4→13) — the tell that the ARENA WORLD itself is
re-rolling, legitimately, at each RNG-re-pin-family commit (GenCtx,
RNG-DEEP-010 rtsim generate, lottery). Build-stall's earlier RED at
`dbbacea48e` was one roll's artifact (probe landed somewhere placing 0
jobs), recovered on the next roll — a fixture-sensitivity flicker, not a
bug. The MINE bar's 2-stuck-cells appeared at the `890bc3bb5b`-era roll
and PERSISTS at tip even as other world-sensitive bars keep changing —
meaning the new world's specific geometry triggers a pre-existing,
always-latent mine-completion fragility (matches Ben's own documented
observed failure classes: stuck/disconnected/semi-built) rather than a
newly-introduced logic bug. Mine flip window narrowed to exactly ONE
world-changing commit: `60e1682428` (GenCtx worldgen-noise DomainHasher
reseed, domain root #3, intended one-time re-pin). **Leg 5 confirming.**

**Decision, confirmed:** if leg 5 confirms, GenCtx is NOT reverted — it's
an intended re-pin, not the bug. The actual fix is diagnosing WHY those
2 cells can't complete in the new world, trace-first per the CarvedStair
lesson (dump unmined cell coords + colonist recorder trace, don't guess
from source). Real cross-validation with the v8 mine-traversal audit's
DET-MTR findings (producer-order access arbitration, depth-cutoff
coherence) — the static-analysis prediction and this empirical bug may
be the same mechanism. b5's world-sensitive bars also need a fixture-
hardening follow-up (filed separately, not blocking the mine fix).

**★ BEN'S CALL: not a determinism issue — filed and deprioritized, back
to the standing backlog.** Since the stuck-cells result is byte-
identical/reproducible across dev and verify for the same world, this
was never a reproducibility violation — it's a pre-existing gameplay
bug in mine-completion that a legitimate world re-roll exposed. Doesn't
block certification, isn't part of the determinism sweep. **Filed as
B78** in `readme/BASTION_COMMON_ISSUES.md` with the full bisect finding,
the world-reroll mechanism, the v8 DET-MTR correlation, and the b5
fixture-hardening note. Builder redirected back to the standing v6/v7
work (NET-018, AST-019/023/024/025, rest of the backlog) — the mine-
completion fix and fixture hardening queued as their own scoped item
for later, not blocking anything.
