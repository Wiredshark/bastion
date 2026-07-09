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
