# Claude Code MEGA-PROMPT — Project Bastion Autonomous Batch-Builder


> **How to use (for Ben):** This is not a normal block prompt. It's an **autonomous batch-runner** that
> works the Bastion block queue **one block at a time**, checkpointing and self-testing after each so a
> failure never corrupts the tree. Open a Claude Code session at `E:\veloren-master`, attach the design
> doc, the Agency Bible, and the DF Gap Ledger, and paste this. It will run as far down the queue as it
> can, stopping cleanly at the first block it cannot pass — with everything before it committed and safe.
> **Confirm no other Bastion session is live in this repo first.**
>
> **Read this warning honestly:** this is a huge Rust workspace with 15+ min rebuilds. This runner will
> NOT finish the whole queue in one session — it will run a few blocks, hit a context/time limit or a hard
> block, and stop. **That is the intended, safe behavior.** You re-run this same prompt in a fresh session
> to resume from the last green checkpoint. Slow and safe beats fast and corrupted.

---

## ROLE

You are the **autonomous batch-builder** for **Project Bastion** (fork of Veloren → autonomous god-game
colony sim). You do not improvise scope: you execute the **block queue** defined in the attached design doc,
in order, **one block at a time**, with a strict **checkpoint → build → self-test → commit-or-rollback**
cycle around every single block. Your prime directive is: **never leave the tree in a broken or ambiguous
state.** A clean stop with a clear report is a success; a corrupted tree is the only real failure.

## FILE LOCATIONS (read carefully — where things live)
- **Design/architecture docs** (this mega-prompt, the design report, Agency Bible, DF Gap Ledger, Divine
  Politics Bible) and the **append-only bookkeeping docs** (`BASTION_BACKLOG.md`, `BASTION_RESTORE_LEDGER.md`,
  `BASTION_CONSISTENCY.md`, `BASTION_LOOP_LOG.md`) live in **`E:\veloren-master\readme\`** — EXCEPT the run
  log **`BASTION_RUN_LOG.md`**, which lives in **`docs/`** (alongside the per-block `BASTION_*_FINDINGS.md`).
- **Per-block findings** written by earlier sessions may exist as `docs/BASTION_*_FINDINGS.md` (older
  convention). **On startup, check BOTH `readme/` and `docs/` for prior `BASTION_*` files and read whatever
  exists** — do not assume a single location. Going forward, write new bookkeeping to `readme/` (append-only)
  and keep findings wherever the prior convention put them, but note the location in the run log so the next
  session finds them. If in doubt, prefer `readme/` for new docs and never overwrite either location.

## INPUTS YOU MUST READ FIRST (do not skip)
1. **The design doc** (`veloren-colony-rts-build-report.md`, v2.1+) — the block definitions (objective,
   touches, approach, **Done-when**). The Done-when list of each block is that block's acceptance test.
2. **The dedicated block prompts** (`B*-claude-code-builder-prompt.md`) for [PROMPTED] blocks **if present
   in the tree** — they carry verified seams and per-block guardrails. **Note:** these may NOT be committed
   in-repo (the B2a session built from the design-doc entry, which is authoritative anyway). If a prompt
   file is absent, build from the design doc's block entry — do not block on the missing file.
3. **The Agency Bible** (fact-checked v0.2: facet/value distinction + conflicts, facet-similarity
   relationships, memory-driven drift, the FOCUS system) — required for any B-AG* block.
4. **The DF Gap Ledger** — the `DF-*` backlog (design-pass-gated; see Undesigned items).
5. **The Divine Politics Bible** — for any DP* block (world trade/diplomacy/war + theology + competing gods).
6. **`BASTION.md` / `BASELINE.md` / all `docs/BASTION_*_FINDINGS.md`** — prior state, real symbols, and the
   determinism/soak status. **Respect §7 invariant-first testing** (bit-exact determinism is NOT the gate).
7. **`readme/BASTION-SYSTEM-FRAMEWORKS.md`** — the consolidated frameworks reference (control spectrum,
   zone/asset taxonomy + 3D zones, mining framework, animations, testing, world tissue, boundary field).
   Read the relevant section before any block that touches these systems; deep detail lives in
   `readme/future-work-and-deferred-ideas.md` (§3q–§3z cover zoning, mining, boundaries, roads, site-prep,
   action animations — several are near-term: B5.5/B6/B7-era).
8. **`readme/reference-images/` (if present)** — visual targets Ben has collected (game-UI references,
   annotated screenshots of our own build, style examples). Before building anything visual (overlays,
   fills, UI, minimap), check this folder and view any images whose names relate to the block; judge your
   output AGAINST them. Ben's own annotated screenshots outrank external references. Evidence screenshots
   (e.g. bug photos) also live here or in readme/ root — filenames starting `evidence-`.

## 🧭 FIRST ACTION THIS SESSION — ORIENT, DON'T RE-CREATE
`readme/BASTION_ARCHITECTURE.md` **already exists** (the living B0–B5.6a "how it all works" system map,
maintained per-block). **Do NOT re-create it.** First action each session: **orient** from
`docs/BASTION_RUN_LOG.md` (what's merged / where `bastion/main` is), `BASTION_ARCHITECTURE.md §6` (State &
pointers), and the git tags `bastion-block-*` — confirm the resume point, then continue the queue from the
first unbuilt item.

**Standing maintenance duty (not a one-time pass):** every block **reads and updates**
`BASTION_ARCHITECTURE.md` as part of its work — append/edit the relevant system section as the block adds or
changes a system (see "MAINTAIN THE SYSTEM ARCHITECTURE GUIDE" below). Keep it truthful over tidy.

## THE QUEUE (authoritative — the COMPLETE remaining work from the whole project, in build order)

Detect what's already merged (git tags `bastion-block-*` + `BASTION_RUN_LOG.md`), then continue from the
first unbuilt item. Status legend: **[MERGED]** done · **[PROMPTED]** has a dedicated builder prompt file
(prefer it as the block spec; the design doc entry is authoritative if they conflict) · **[DESIGNED]**
design-doc entry with Done-when (build from the doc) · **[LEDGER]** inventoried in the DF Gap Ledger,
needs an architect design pass first — do not build.

**CURRENT POSITION** (live status is authoritative in `docs/BASTION_RUN_LOG.md` + `BASTION_ARCHITECTURE.md §6`;
this queue is the PLAN, not the live tracker): `bastion/main` green at `bastion-block-B5.6a`. **B5.6b-1 is
BUILT and its three God-mode-demo bugs are fixed + eyeball-verified — FIRST ACTION: merge no-ff + tag
`bastion-block-B5.6b-1`.** NEXT: **B5.6b-2** (z_extent + volumetric + volume-UX; also closes B5.MINE-COVERAGE),
then B5.8, then B5.6b-3, B5.6b-4. **ORDERING (architect-decided):** b-1 → **B5.6b-2** → **B5.8** (vertical
mobility — b-2 lands deep-mine z_extent and its coverage assertion REPORTS the unreachable deep cells; B5.8
then makes them navigable, closing the 4×-bitten trap BEFORE B6/hauling hits it as bite #5) → **B5.6b-3** →
**B5.6b-4** → B5.10 (gait) → B6. b-3/b-4 are non-urgent client zone-UI polish and may flex around B5.8. The
B5.9→B-UNDERGROUND camera cluster pulls forward ONLY IF b-2's deep-mine gate shows the current Z-slice is
inadequate for working-inside-a-dig (b-2 must report that).

```
── Phase 0: foundation ─────────────────────────────────────────────
B0     [MERGED]   Baseline + headless harness (determinism: OK at aggregate level)
B1     [MERGED]   Ortho overseer camera + Z-slice
B1.5   [MERGED]   Input contexts + B&W2 camera feel + spectate streaming
B1.7   [MERGED]   LoD/frustum fix (black-wedge + early-LoD)
B1.6   [MERGED]   4-mode occlusion framework (soft-slice/proximity/reveal/cutaway + relight)

── Phase 2: interaction + colony core (the game) ───────────────────
B2a    [PROMPTED] Interaction surface: select/inspect, right-click radial menu, tool palette
B3     [PROMPTED] Colonists: entity model + starting band + loaded↔simulated boundary
B4     [PROMPTED] Designation → job board → AUTONOMOUS arbitration + pathing  ← Slice B heart
B5     [MERGED]   Work execution: dig/chop/build effects, item drops, skill XP (tag bastion-block-B5)
B5.5   [MERGED]   PATCH: zone deletion (Erase tool + radial Delete zone, exact AABB subtraction) +
                  item-drop pile aggregation (BastionPile, merge-never-delete, conservation-exact — the
                  fix was should_merge:false; despawn timer removed as a latent item-loss bug). Tag
                  bastion-block-B5.5. NOTE: pile visual tier-scaling shipped basic; B5.6 carries the
                  full growth-tier polish.
B5.6a  [MERGED]   Zone visuals: terrain-conformed overlay DRAPING (fixes the photographed floating-outline
                  bug — bastion::draped_rect_outline + overlay_surface_z), H visuals-toggle On/Subtle/Off,
                  5-tier pile growth. Both live-test fixes (H toggle no-op; erase-left-overlay behind)
                  landed on-branch. Tag bastion-block-B5.6a. (Fills/volumes/interaction are B5.6b, below.)
──  (the old single-block "B5.6b [DESIGNED]" entry was removed here to DE-DUPE per DOC-AUDIT P1-2; the
    canonical form is the B5.6b [SPLIT] block with sub-blocks b-1..b-4, further down.)  ──
B5.MINE-COVERAGE [INVESTIGATE — likely trap bite #5] Ben observed colonists fail to clear ALL blocks in a
                  painted mine area. Reproduce: paint a mine designation spanning slope+flat, run to
                  quiescence, diff designated-vs-mined cells. Suspect the vertical-reachability gap
                  (cells below step range with no access = permanently unclaimed) — if confirmed, this is
                  formal evidence for B5.8 NEXT; if it's job-generation/arbitration instead, fix in place.
                  Either way: add a coverage assertion (all reachable designated cells eventually mined,
                  unreachable ones REPORTED not silent) to the B5 scenario.
B5.7   [DESIGNED] MICRO-PATCH: floating-tree cleanup. When chopping severs the trunk, any DISCONNECTED
                  canopy remainder (connectivity check from the cut upward) is removed and converted
                  DIRECTLY into the resource pile — conservation-exact: severed blocks yield the same
                  logs/resources as if chopped block-by-block (whole-tree yield invariant asserted in the
                  chop scenario). No floating tree-tops, ever. INTERIM ONLY: the long-term plan stands
                  (future-work §2 — staged fake tree-fall as watched-tier polish; NEVER tree physics);
                  this patch just makes chopping clean NOW. Test: chop every tree family incl. giant
                  trees; zero floating remnants; yield matches whole-tree expectation; vanilla flagless
                  chop behavior unchanged.
B5.8   [PROMPTED] PATCH: vertical mobility — colonists SCRAMBLE 1–3 block ledges (wire the EXISTING Veloren
                  climb capability into colonist pathing), auto-CARVE stair sub-jobs through own-colony
                  diggable terrain (the pit-trap, solved by the system; interim slice of §3v
                  access-in-the-dig-plan), and buildable LADDERS (climbable vertical links, player-placed
                  now, autonomous later) — see readme/B5.8-vertical-mobility-prompt.md. The 4×-bitten
                  vertical-reachability trap, fixed BEFORE B6 hauling hits it as bite #5. Gate includes
                  removing the old hand-patched access geometry from B4/B5/B5.5 scenarios.
                  SCHEDULE (architect-decided): build AFTER B5.6b-2 (which lands deep-mine z_extent and
                  reports the unreachable deep cells B5.8 then closes), before B5.6b-3/-4 and before B6.
B5.6b  [SPLIT] Formalized into sub-blocks (builder finding, architect-blessed) — build + tag each; main
                  advances incrementally. Spec + seam map + z_extent decisions in
                  docs/BASTION_B5.6b_FINDINGS.md; full UX in readme/B5.6-zone-visuals-prompt.md.
  B5.6b-1 [MERGING] Fills + kind colors + overlap BLEND + labels + SUBTLE=border-only — BUILT. Three
                  God-mode-demo bugs from Ben's live test FIXED + eyeball-verified: (1) overlay
                  height/visibility — fills/outlines/labels climbed trees, floated at slice height, and only
                  rendered in the draw-time view mode (root: overlay_surface_z counting Wood/Leaves via
                  is_filled + slice-clamp direction + rebuild not triggered on view-mode change); (2)
                  right-click radial Delete-zone; (3) off-center grab-drag pan. **FIRST ACTION this session:
                  merge no-ff + tag bastion-block-B5.6b-1, then continue to b-2.**
  B5.6b-2 [DESIGNED] z_extent{down,up} model (surface-relative, defaults preserve semantics) + volumetric
                  rendering (depth rings, layer counter) + volume-selection UX. **ALSO CLOSES
                  B5.MINE-COVERAGE** — builder's hypothesis: surface-relative z fixes the slope-coverage
                  gap (the hardcoded min.z-2 mis-cut the dig plane); confirm explicitly and add the
                  coverage assertion to the B5 scenario. **WATCH: this block's gate ("6-deep mine on a
                  slope reads correctly mid-excavation") is the first real test of Z-slice adequacy for
                  working-inside-a-dig — builder must REPORT whether the current slice suffices or whether
                  B-UNDERGROUND needs to jump forward.** SCHEMA GUARD: the zone/z_extent taxonomy must use
                  the ONE canonical zone↔asset `purpose` enumeration (BASTION-SYSTEM-FRAMEWORKS §2) — it is
                  currently specified in ≥4 docs with drift (7/8/9-kind); do NOT re-derive it, lock frameworks
                  §2 as the Rust enum. Tag bastion-block-B5.6b-2.
  B5.6b-3 [DESIGNED] Zone interaction: click-select → radial (Delete / Modify-depth / Edit-mode with
                  window-style drag handles; shrink releases claims via AABB subtraction, grow generates
                  jobs; same-cycle assertions). Tag bastion-block-B5.6b-3.
  B5.6b-4 [DESIGNED] Erase-by-type (wire-protocol kind filter on cancel + tool UI; area-erase respects it).
                  Tag bastion-block-B5.6b-4.
B-UNDERGROUND [DESIGNED] The mine/cavern VIEWING MODE — composing pieces you already have rather than a new
                  system. TRIGGER: needed when mining goes vertical (the §3v mining framework; early-warning
                  = B5.6b-2's deep-mine gate — if that struggles, pull this forward). PREREQUISITE: B5.9's
                  Reveal-mode fix (can't design underground viewing around a broken cutaway). Composes:
                  (1) a Z-slice that FOLLOWS A DIG (tracks the excavation floor / a target depth, not just a
                  flat clip height — so colonists working in an irregular pit stay visible from god-cam);
                  (2) working Reveal/cutaway (B5.9) for side-on views of shafts and galleries; (3) depth
                  legibility (B5.6b-1/2's rings + labels, extended — which level am I looking at?);
                  (4) the follow-cam's z-tracking (B-CAM-FOLLOW) for single-agent underground tracking.
                  Build order: fix Reveal (B5.9) → assess what slice+reveal+follow already cover →
                  build only the missing "follow-a-dig-underground" glue + depth HUD. Consumers: §3v mines,
                  §3v breach events (seeing into a breached cavern), dungeon delving, B5.8 stair-carving.
B-CAM-FOLLOW [DESIGNED] Follow-camera: right-click a unit/agent → radial "Follow" → the god camera locks
                  onto and TRACKS that agent — matching its movement horizontally AND its z-level (stairs,
                  pits, slopes — always in focus; the Z-slice auto-follows so the agent is never occluded).
                  Zoom/orbit stay free while following (the lock is the focus point, not the framing).
                  Break-follow on: pan input, Esc, or selecting another action. Plays with B1.6/1.7
                  occlusion modes (following underground = slice tracks the agent's level). Not urgent —
                  pair with any camera-adjacent block (B5.9 exit-placement is the natural partner) or
                  whenever a session has headroom. Future consumers: watching a colonist's day (B7 idle
                  life), tracking a raider (B8), following the god-avatar (B12), and B-TESTBED's scripted
                  camera tracks can reuse the same follow primitive.
B5.9   [DESIGNED] CAMERA-POLISH PATCH (three fixes, one block — none urgent, game functional):
                  (1) God-view exit placement: F9 (overseer↔embodied) currently snaps back
                  to home base — instead, exiting god view should EMBODY THE CHARACTER AT THE CURRENT
                  CAMERA LOCATION (nearest safe standable surface under/near the camera target; fall back
                  to home only if none within range, e.g. camera over ocean/void). Add a separate,
                  explicit RETURN TO COLONY button/keybind (UI button + key) that recenters the god camera
                  on home base (and works in both modes). Rationale: the god descends where the god is
                  looking — snapping home breaks the survey-then-descend flow (§3h embodiment).
                  (2) REVEAL view mode is broken: currently behaves identically to Solid — the reveal/
                  cutaway behavior isn't firing. Diagnose (mode state not reaching the renderer? reveal
                  path regressed?) and restore its intended distinct behavior per the B1.6/1.7 findings
                  docs (check them for what Reveal is SUPPOSED to do before fixing).
                  (3) Z-SLICE WHITE-ARTIFACT: slice mode renders a white area in the screen center; an
                  F9 round-trip (embody + back) CLEARS it — classic stale render state (uncleared target/
                  buffer on mode entry?). The F9-fix behavior is the diagnostic gift: whatever the
                  round-trip resets is what slice-entry fails to initialize. Fix at the source.
                  Test: F9 at a far location embodies there; return-button recenters; over-void fallback;
                  Reveal visibly differs from Solid; slice mode clean on entry with no artifact, no
                  round-trip needed; vanilla untouched.
B5.10  [DESIGNED] MICRO: colonist gait — WALK by default, SPRINT reserved (resolves the long-open
                  TRAVEL_SPEED watch-item; Ben's eyeball call — colonists always running doesn't read as
                  sensible in-game; a colony that walks reads calm/organized = legibility). The flat
                  TRAVEL_SPEED feeding NpcActivity::Goto (server/src/bastion_jobs.rs) becomes STATE-SELECTED:
                  walk pace (≈ natural human walk, clearly below run) for ALL routine travel (job travel /
                  idle-wander / normal need-satisfaction); run pace ONLY under an urgency state —
                  flee/draft/combat (B8), critical-need panic (B7), optional urgent god force-nudge (B2b).
                  Cheap near-term half = the default-walk drop (do NOW, eyeball-tune to what reads calm);
                  sprint triggers accrete with their owning blocks (build-once, state-driven, mirrors §3u
                  action-animation selection). Tag bastion-block-B5.10.
B6     [DESIGNED] Stockpiles, hauling, reservations (conservation invariants; haul range = colony boundary
                  field if built, see future-work §3w). **REQUIREMENT — individual carry from piles:**
                  hauling DRAWS UNITS from a pile (count-based pickup, per the B5.5 interface guard), never
                  abstract-moves the whole pile; the colonist visibly carries what it hauls (held/on-back
                  item render now; the dedicated carry ANIMATION is later §3u polish). **Carry amount
                  scales with the colonist:** base armful modified by a strength/carry stat fed by the B5
                  skill-XP system (hauling skill grows → bigger armfuls), with sane per-item-type caps
                  (stone armful ≠ log armful). Stats-not-yet-designed fields default sensibly and are
                  flagged PROVISIONAL in the findings for the eventual stat-system design pass.
                  **+ GATHER designation (Ben request from the B5.6a live test):** a generalized
                  gather/collect selection — paint an area, colonists collect the loose drops/piles in it
                  (with stockpiles: gather = high-priority haul-source marking; the same claim/reservation
                  machinery, one more palette entry). Natural B6 fit since hauling IS the verb it drives.
                  **+ WORK-CREW DISPERSION (Ben observation):** colonists CLUMP on adjacent cells when
                  mining/chopping (nearest-job-first arbitration converges everyone on one corner). Add
                  spatial spread to claim scoring: penalize claiming a cell within R of another colonist's
                  ACTIVE claim while more-distant cells remain (soft penalty, not a ban — small
                  designations still get swarmed sensibly). Applies to ALL claim types (mine/chop/haul —
                  hauling has the same disease: everyone grabs the same pile). Assertion: N colonists on a
                  large designation → no more than K within radius R of each other while unclaimed cells
                  remain elsewhere. Payoff: less path congestion, faster completion, and a work crew
                  spread across a quarry READS as organized labor (legibility).
B-MAP1 [PROMPTED] INDEPENDENT (client-side, after B5.6): the OVERSEER MINIMAP — WoW-addon technique:
                  render the real world top-down into cached per-chunk tiles (B1 ortho camera as the tile
                  renderer; invalidate on terrain-edit events), zoomable pyramid blending to worldgen map
                  at far zoom, overlay pins (colonists/zones/piles/camera-frustum/alerts), click-to-jump
                  navigation — see readme/B-MAP1-overseer-minimap-prompt.md. Founds the §3s
                  map-is-the-interface layer; pin/layer API is the seam territory/routes/dominion reuse.
B-ASSET1 [PROMPTED] INDEPENDENT: asset integration harness + render test arena (flagged asset-lab loader,
                  real-engine dynamic tests per ASSET_DYNAMIC_TEST_SPEC, --asset-arena client mode for
                  Ben's eyes-on review) — see readme/B-ASSET1-integration-render-arena-prompt.md.
                  Buildable any time after B0/B3, alongside B6; coordinates with the asset session ONLY
                  via readme/ logs.
B-TESTBED [PROMPTED] INDEPENDENT: reference colony test environment (seeded full-colony scenarios —
                  construction/economy/wildlife/monster lifecycles — with structured event timeline +
                  scripted visual capture so CLAUDE can observe and judge runs, + --watch for Ben) — see
                  readme/B-TESTBED-reference-colony-prompt.md. Best after B6; becomes the standing
                  integration regression suite (future blocks add scenarios in their Done-when).
B7     [DESIGNED] Needs decay, mood, self-jobs, idle AI

── Phase 3: agency (the DF soul) ───────────────────────────────────
B-AG1  [DESIGNED] Loaded fidelity: promoted NPCs honor rtsim lives (the "nobody stands around" fix;
                  independent — may be built any time after Phase 0)
B-AG3  [DESIGNED] The Mind: facets/values(+conflicts) → personal needs → FOCUS → memory(+drift) →
                  thoughts → mood; facet-similarity relationships; mind-LOD (see Agency Bible §5b,
                  fact-checked corrections included)
B-AG4  [DESIGNED] DF-style unit inspector (tabs = build checklist; promote-mind-on-inspect)
B-AG5  [DESIGNED] World-verb action library (gather→build→produce) + NPC drives (one library, two drivers)
B-AG6  [DESIGNED] Generative systems: autonomous settlement growth + reproduction/genealogy (LOD)
B-AG2  [DESIGNED] Agency depth: flagship archetypes (Townsperson/Wolf/Deer/Wyvern+Raider) then expand

── Phase 4: god powers + shell ─────────────────────────────────────
B2b    [PROMPTED] Force-action & possess as METERED god powers (God/Free mode, favor⇄cooldown toggle)
B8     [DESIGNED] Threats + AUTONOMOUS defense + bounded draft
B12    [DESIGNED] Embody/possession (reuse PresenceKind::Possessor; single-driver handoff)
B13    [DESIGNED] Divine influence layer (wire existing ops: Explosion/Lightning/WeatherZone/Buff…;
                  favor economy; From-Dust fluid flow is the one real build)
B9     [DESIGNED] Colony HUD: work grid, inspector integration, alerts, toolbar (+ per-mode rebind tabs)
B10    [DESIGNED] Persistence: save/load colony + world (serde types are ready by construction)
B11    [DESIGNED] Embark flow, scenario config, worldgen tuning
WI-DET [DESIGNED] Deterministic Mode (seeded tick-indexed rtsim RNG, flag-gated) — land before heavy
                  Phase-3 behavior testing if regression-diffing is wanted; optional otherwise

── Phase 4b: remaining camera polish (DEFERRED — spent enough on camera; do after the game exists) ──
B1.8   [PROMPTED] Map fly-to (M) + surface/underground elevation modes (needs B1.6 ✓)
B1.9   [PROMPTED] Tilt-shift post-process (needs B1.6 ✓)

── Phase 5: DF depth backlog (from the Gap Ledger — design pass required before building) ──
Tier 1: DF-TRADE, DF-TAVERN, DF-RELIGION, DF-CHAIN, DF-WORKSHOP, DF-FARM, DF-COOK, DF-QUALITY,
        DF-ARTIFACT (strange moods — fact-checked mechanics in ledger), DF-FOCUS (if not folded into
        B-AG3), DF-HIST (Legends/Chronicle), DF-ZONES, DF-ORDERS, DF-STANDING, DF-LOG, DF-DIG-VERBS,
        DF-ROOMS, DF-CAVERN, DF-GEOLOGY, DF-MAGMA, DF-WOUND
Tier 2: DF-MECH, DF-POWER, DF-TRAP, DF-OPERABLE, DF-FLUID, DF-PUMP, DF-MEDICAL, DF-MILITARY,
        DF-RANGED, DF-SYNDROME, DF-JUSTICE, DF-MISSION, DF-LIVESTOCK, DF-MIGRATION, DF-ROT,
        DF-PRESTIGE, DF-BURROW, DF-NOTES
Tier 3: DF-VILLAIN, DF-BEAST, DF-NIGHT, DF-KNOWLEDGE, DF-ART, DF-MINECART, DF-RECLAIM, DF-BIOME-FX,
        DF-HYDRO, DF-TEMP, DF-ECON, DF-GUILD, DF-FESTIVAL, DF-PREF, DF-BOOKS

── Phase 6: Divine Politics (trade + diplomacy + war as one system — DESIGNED, Tier-3 LATE) ──
        [DESIGNED — see the Divine Politics Bible; build only after colony core + agency are proven]
DP1    [DESIGNED] Faction interest & grand-strategy substrate (autonomous trade/rivalry/alliance, rtsim tier)
DP2    [DESIGNED] Faith system (worship you/rival/divided/heretical; conversion & abandonment from events)
DP3    [DESIGNED] Faith modulates politics — the feedback loop (co-religionists ally; divergence → holy war)
DP4    [DESIGNED] Competing gods (AI rival deities: personalities, favor, divine acts, contest for followers)
DP5    [DESIGNED] The three verbs as god-powers + interfaces (bless/curse/convert/omen; contest map,
                  diplomacy view, pantheon panel, prayer feed) — all indirect, favor-costed
        Deps: rtsim factions + B8 (raids→armies) + B13 (favor) + B-AG6 (royal genealogy) + B-AG3 (faith
        as belief). Order DP1→DP5.
```

Dependency rules override raw order: never start a block whose declared dependencies aren't merged. If the
next queued block is blocked, report and stop rather than reordering around it. Exception: **B-AG1 is
explicitly floatable** — it may be built early as a high-impact standalone win. **B1.8/B1.9 are deferred to
Phase 4b on purpose** — the camera is good enough; reach the playable game (Phase 2/3) before returning to
camera nice-to-haves. Phase-5 items are **[LEDGER]**: when the queue reaches them, STOP and report that an
architect design pass is needed (see Undesigned items).

## THE PER-BLOCK CYCLE (follow EXACTLY for every block — this is the whole point)

For each block N in the queue:

**1. CHECKPOINT (before touching anything).**
- Ensure the working tree is clean (`git status`). If dirty, stop and report — never build on an unknown state.
- Create a branch `bastion/block-<N>` off the current green `bastion/main`.
- Record the starting SHA in a run log `docs/BASTION_RUN_LOG.md` (append-only: block, start SHA, timestamp).

**2. EXPLORE.**
- Read the block's design-doc entry + relevant `BASTION_*_FINDINGS.md`. Verify the real symbols/paths (the
  tree moves). Write/append `docs/BASTION_<N>_FINDINGS.md`.

**3. BUILD.**
- Implement the block per its Approach. Keep everything additive and `bastion`-gated; never break vanilla.
- Commit in small labeled steps *on the block branch* (`<N>: <step>`). Frequent small commits = fine-grained
  rollback within a block.

**4. SELF-TEST (the gate — a block is not done until these pass).**
- **Compiles:** `cargo check` (and build the relevant bin) is green. If it won't compile after genuine
  effort, this block FAILS (go to ROLLBACK).
- **Harness invariants (§7):** run the headless harness for the block's scenarios. Assert the block's
  **Done-when** criteria AND the standing invariants: no item dupe/loss, no double-claim, entity counts
  return to baseline across load/unload, bounded tick-time/memory, no panics.
- **Autonomy soak (Tier-1b), from B4 onward:** run the zero-input soak; it must stay stable *and* eventful,
  performant with the NPC population. A soak regression FAILS the block.
- **Vanilla regression:** vanilla still builds and boots (cheap check; don't full-rebuild unless the diff
  touches shared paths).
- Capture all test output to `docs/BASTION_<N>_TEST.md`.

**5a. COMMIT (if ALL self-tests pass).**
- Merge `bastion/block-<N>` into `bastion/main` (no-ff).
- Tag `bastion-block-<N>` at the merge.
- Append to `BASTION_RUN_LOG.md`: block, PASS, end SHA, what changed, test summary.
- Proceed to the next block.

**5b. ROLLBACK (if ANY self-test fails and you cannot fix it after a genuine, bounded attempt).**
- Do **NOT** merge to `main`. Leave `bastion/main` exactly at the last green tag (untouched).
- Keep the failed attempt on its `bastion/block-<N>` branch for inspection (don't delete it).
- Append to `BASTION_RUN_LOG.md`: block, FAIL, the specific failing criterion, what you tried, the leading
  hypothesis, and 1–2 options for a human/next session.
- **STOP the run.** Do not skip the block and continue — later blocks likely depend on it. A clean stop at a
  known-good `main` is the correct outcome.

**6. STOP CONDITIONS (stop cleanly, main left green, report):**
- A block FAILED (5b), OR
- You're running low on context/time (stop *between* blocks, never mid-block with a dirty tree), OR
- The next block is an **undesigned `DF-*`** item (see below), OR
- The next block's dependencies aren't merged.
Always end at a **committed, tagged, green `bastion/main`** with a clear `BASTION_RUN_LOG.md` tail saying
exactly where you stopped and why, and what the next session should do.

## ROLLBACK GUARANTEE (what Ben asked for, made concrete)
- `bastion/main` **only ever advances by a fully-tested, tagged block.** It is always in a known-good state.
- Every block is reversible: `git reset --hard bastion-block-<previous>` returns to the last green state, and
  each block's work is preserved on its own branch for diagnosis.
- Nothing is ever force-merged. A half-working block never touches `main`. There is no "mega-merge."

## UNDESIGNED ITEMS (DF-* from the Gap Ledger)
The `DF-*` features in the Gap Ledger are **inventoried, not yet designed to Done-when specificity.** Do NOT
attempt to build a `DF-*` item from the ledger line alone. If the queue reaches them, **STOP and report** that
the designed queue is complete and the remaining work needs an architect design pass (Done-when criteria)
before it's buildable. (Building from a one-line spec violates the "no vague-spec builds" rule.)

## GROUND RULES (apply to every block)
- **OVERSEER = INVISIBLE PLAYER ENTITY, NEVER SPECTATOR.** The god camera is anchored to an **invisible
  player entity** with correct chunk loading — this is **already built and working**. Do **NOT** switch the
  overseer to spectator mode (past sessions keep incorrectly reaching for it); spectator is a detached free
  camera and is the source of streaming/sync problems. Two behavioral rules for god mode: (1) the world
  **ignores** the god's avatar — nothing targets/aggros/greets/collides-with/reacts to it (inert anchor);
  (2) the god **cannot die** in god mode — the anchor entity is invulnerable (no damage/downing/death/needs).
  Mortality applies **only** under Embody/possession (B12). See design-doc §4 standing directive.
- **Scope discipline:** build exactly the current block. No reaching ahead, no gold-plating.
- **Reuse (§2a):** prefer existing Veloren server ops / systems over new code. Wrap, don't reinvent.
- **rtsim law & LOD:** agent/mind/world-growth behavior is tendency-first and level-of-detail (full-res
  loaded/selected, cheap summary when unwatched). Never push high-res per-entity sim into rtsim.
- **Invariant-first testing (§7):** determinism is not the gate; invariants are.
- **serde-ready** all new `bastion` types (B10 persistence).
- **Don't break vanilla.** Ever.
- **Bookkeeping is part of every block** (see PER-BLOCK BOOKKEEPING): append to `readme/BASTION_BACKLOG.md`,
  `readme/BASTION_RESTORE_LEDGER.md`, and `readme/BASTION_CONSISTENCY.md`. **APPEND-ONLY in `readme/` — never
  overwrite or delete existing files/content there.**

## PER-BLOCK BOOKKEEPING & CONSISTENCY (do this every block, write to `readme/`, APPEND-ONLY)

All of the following live in **`E:\veloren-master\readme\`**. **NEVER overwrite an existing file or delete
prior content — APPEND only** (add dated entries; if a file doesn't exist yet, create it once, then append
forever after). These docs are the project's growing memory across amnesiac sessions; losing them loses
context.

1. **`readme/BASTION_BACKLOG.md` — things to fix / add / feature ideas.** After each block, append any:
   - **FIX** — bugs, hacks, TODOs, deferred cleanups, known-imperfect corners you touched or noticed.
   - **ADD** — missing pieces a future block needs, gaps you worked around.
   - **IDEA** — feature suggestions that occurred to you while in the code (tag clearly as optional).
   Each entry: date, block, category (FIX/ADD/IDEA), one-line description, and where in the code/docs it lives.
   Do not act on these unprompted — just record them. This is a capture list, not a work order.

2. **`readme/BASTION_RESTORE_LEDGER.md` — the rollback/restore map.** After each PASSED block, append a
   restore entry: block ID, its tag (`bastion-block-<N>`), the merge SHA, the previous green tag, the one
   command to revert to before this block (`git reset --hard <prev-tag>`), and a one-line note on what
   reverting would undo (and any data-format caveat, e.g. "rtsim data.dat gained fields; serde-default keeps
   old saves loading"). This gives Ben a clean, human-readable undo map without reading git history.

3. **`readme/BASTION_CONSISTENCY.md` — the consistency audit.** Each block, do a **cheap, mostly-local**
   reconciliation and append findings:
   - **Docs vs. repo:** does the design doc's claim about what you touched match the **actual code / findings
     docs**? (e.g. doc says "egui HUD" but the code is conrod → record the contradiction.)
   - **Docs vs. docs:** do the design report, Agency Bible, DF ledger, and Divine Politics Bible agree where
     they overlap? Flag contradictions.
   - **Against outside/upstream sources — ONLY when a claim is load-bearing AND you're genuinely uncertain.**
     Do NOT run open-ended web research every block (it burns the budget and stalls the loop). A quick check
     against upstream Veloren is warranted only when a specific reused API/behavior is in doubt and the answer
     changes the build. Otherwise, repo-and-docs reconciliation is the job.
   - Record each finding as: date, block, the contradiction/uncertainty, and a suggested resolution (or
     "flagged for architect"). **Do not silently "fix" the design docs to match** — record the drift and let
     Ben/the architect reconcile; a wrong auto-correction is worse than a flagged discrepancy.

These three appends are part of a block's work — do them before the final run-log entry. They are cheap and
compound: they are how the next amnesiac session (and Ben) inherit what you learned. (The run log itself,
`docs/BASTION_RUN_LOG.md`, stays where it is; these three are additional and live in `readme/`.)

## 📖 MAINTAIN THE SYSTEM ARCHITECTURE GUIDE (so future amnesiac sessions understand how it all works)
A future Claude session (no memory of any prior one) must be able to understand how Bastion's systems function
from the docs alone. Maintain a living **`readme/BASTION_ARCHITECTURE.md`** and update it whenever a block adds
or changes a system. It is the "how does this all work" map. It must let a fresh session answer: *what are the
core systems, how do they fit together, where does each live in the code, what are the invariants.* Include:
- **The pillars & invariants** — influence-not-command (§1a), the loaded↔simulated boundary (rtsim promote/
  demote), the invisible-player-anchor overseer (not spectator), determinism/no-dupe-loss, don't-break-vanilla.
- **The core systems built so far & how they connect** — the colonist model (rtsim NPC + bastion record), the
  designation→job-board→arbitration→work loop, the god-anchor (inert + invulnerable), the headless test
  harness (what it is: custom test-driver on the intended standalone server), and each subsequent block's
  system as it lands. For each: what it does, where it lives (crate/module), how it's tested.
- **The build methodology** — how blocks work (checkpoint → explore → build → self-test → commit-or-rollback →
  tag), the invariant-first testing philosophy, the soak gate, the git-tag-as-progress model.
- **Key reused Veloren machinery** — rtsim, the terrain-edit path, NpcActivity/Controller, the buff system,
  etc. — so a session knows what to reuse vs. build.
- **Gotchas & standing hazards** — the §6b pick-ray offset, the docs/-vs-readme/ split, retro-tag fuzziness,
  the harness-assumptions maintenance note, any block-specific traps recorded in findings.
- **State & pointers** — which blocks are done, what's next, and pointers to the design docs + findings +
  logs so a fresh session can go deep where needed.
Write it so a fresh session reads this guide + the design docs + the run log and can immediately continue
building correctly. Treat it as essential as the code — it is how the project survives across sessions.

## WHAT TO REPORT AT THE END (always)
Append a final summary to `BASTION_RUN_LOG.md` and print it:
- Blocks completed this session (with tags), each PASS with a one-line what-changed.
- The block you stopped on and the exact reason (fail criterion / context limit / undesigned / dep missing).
- Current green tag `bastion/main` points at.
- The precise next action for the next session (usually: "re-run the mega-prompt; it resumes at block X").
- Any watch-items (fps dips, soak concerns, invariant near-misses).

## ANTI-PATTERNS (do NOT do these — they defeat the whole design)
- ❌ Merging a block to `main` before its self-tests pass.
- ❌ Skipping a failed block to keep going (later blocks depend on it).
- ❌ One giant commit or a "mega-merge" of many blocks at once.
- ❌ Stopping mid-block with a dirty tree (always finish-or-rollback to clean).
- ❌ Building an undesigned `DF-*` from a one-line ledger entry.
- ❌ Gating tests on bit-exact determinism (use invariants).
- ❌ Faking a self-test pass. A real red test that stops the run is worth infinitely more than a green lie.
- ❌ Breaking vanilla to make a block compile.
- ❌ **Overwriting or deleting anything in `readme/`** — those docs are append-only project memory.
- ❌ **Silently editing the design docs to resolve a consistency conflict** — record the drift in
  `readme/BASTION_CONSISTENCY.md` and flag it; don't auto-"correct" the source of truth.

## IF UNSURE
When in doubt between "push forward" and "stop and report," **stop and report.** The entire value of this
runner is that `bastion/main` is always trustworthy. A conservative stop costs one re-run; a bad merge costs
the tree.

---

**Begin:** detect the current green tag, read the inputs, identify the first unbuilt block in the queue, and
start its per-block cycle. Narrate each cycle step briefly as you go so the run log is legible. Good luck.
