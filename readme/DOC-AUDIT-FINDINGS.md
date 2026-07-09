# Project Bastion — Doc Reconciliation Audit (findings)

**Author:** Agent 4 (doc-reconciliation audit, read-only). **Date:** 2026-07-09.
**Scope:** `readme/` + `docs/` design corpus. **Discipline:** flag, don't fix — same as the builders.
This doc is the ONLY file this pass wrote; no existing doc or code was touched.

## Method & ground truth

Cross-read the status spine (MASTER-COLLATION-index, MEGA-PROMPT queue, `docs/BASTION_RUN_LOG.md`,
`BASTION_ARCHITECTURE.md`, `BASTION_CONSISTENCY.md`, BASTION-SYSTEM-FRAMEWORKS), the design doc, the
future-work catch-all, the B5.6 prompt + B5.6b findings, and spot-checked the bibles + gap ledger.

**Established build reality (from the run log + git branch, the truth sources):**
- `bastion/main` is green at **`bastion-block-B5.6a`**.
- Merged + tagged so far: B0, B1, B1.5, B1.6(+B1.7), B2a, B3, B4, B5, B5.5, **B5.6a**.
- **B5.6b-1** (zone fills + colors + labels) is IN PROGRESS on the current branch `bastion/block-B5.6b-1`
  (run-log start entry only; no merge yet).
- The FRESHEST status docs are `docs/BASTION_RUN_LOG.md`, `readme/BASTION_ARCHITECTURE.md` §6, and
  `readme/BASTION_CONSISTENCY.md`. Several higher-altitude docs lag them — that lag is most of what follows.

Findings are grouped by priority (how likely each is to actively mislead a fresh amnesiac session), and
tagged **CONTRADICTION / STALE / ORPHAN / DUP**. Each gives both sides' location and a recommended
reconciliation for Ben to action.

---

## PRIORITY 1 — will actively mislead the next build session

### P1-1 · STALE · MEGA-PROMPT "FIRST ACTION" tells every session to create a doc that already exists
- **A:** `readme/MEGA-PROMPT-autonomous-batch-builder.md` lines 60–77 — "🥇 FIRST ACTION THIS SESSION —
  CATCH-UP DOCUMENTATION PASS … **`readme/BASTION_ARCHITECTURE.md` does not exist yet** … Before you build
  the next block, create it …" and "This catch-up pass is the priority first task; do not skip it."
- **B:** `readme/BASTION_ARCHITECTURE.md` **exists** (453 lines, retroactive B0–B5.6a map; run log confirms
  it was created at `b7f01d1` and is maintained per-block).
- **Impact:** the runner's own instructions order the first task be a one-time pass that is already done.
  A literal-minded session could waste a cycle, or (worse) overwrite the existing guide.
- **Recommend:** replace the "🥇 FIRST ACTION" block with a standing "read + update `BASTION_ARCHITECTURE.md`
  as part of each block" instruction (the maintenance directive at lines 427–447 already says this — the
  catch-up preamble is the stale part). Delete the "does not exist yet / create it" framing.

### P1-2 · STALE · MEGA-PROMPT queue shows B5.6a un-merged and lists B5.6b twice
- **A (status):** MEGA-PROMPT line 105 marks **B5.6a `[TESTED-HOLD]`** with "MERGE BLOCKED on two fixes."
  Reality (run log, "B5.6a … PASS … merge `c26f860`, tag `bastion-block-B5.6a`"): both fixes were made and
  **B5.6a merged**. It should read `[MERGED]`.
- **B (duplicate entry):** MEGA-PROMPT lists **B5.6b twice** — line 113 `[DESIGNED]` (single block) AND
  line 148 `[SPLIT]` (into B5.6b-1…-4). The two entries co-exist; a reader can't tell if B5.6b is one block
  or four.
- **C (next-block):** the queue never marks B5.6b-1 as started; the current branch is building it.
- **Impact:** a fresh session auto-detecting "first unbuilt block" from this queue gets an ambiguous answer
  (is B5.6a still on hold? is B5.6b one block or the split?).
- **Recommend:** set B5.6a → `[MERGED]`; delete the standalone B5.6b `[DESIGNED]` entry at line 113 (keep
  only the `[SPLIT]` block at 148 as the canonical form); mark B5.6b-1 `[IN-PROGRESS]`. Single-source the
  sub-block list to the `[SPLIT]` entry.

### P1-3 · CONTRADICTION · MEGA-PROMPT disagrees with itself on where the run log lives
- **A:** MEGA-PROMPT line 29 lists **`BASTION_RUN_LOG.md`** among the bookkeeping docs that "all live in
  **`E:\veloren-master\readme\`**."
- **B:** MEGA-PROMPT line 313 ("Record the starting SHA in a run log **`docs/BASTION_RUN_LOG.md`**") and
  line 425 ("The run log itself, **`docs/BASTION_RUN_LOG.md`**, stays where it is") say `docs/`.
- **Reality:** the file is at **`docs/BASTION_RUN_LOG.md`** (there is no `readme/BASTION_RUN_LOG.md`).
- **Impact:** a session trusting line 29 could create a second run log in `readme/`, splitting the
  append-only history across two files.
- **Recommend:** fix line 29 to say `BASTION_RUN_LOG.md` lives in `docs/` (with the other `BASTION_*`
  bookkeeping in `readme/`). This is the general `docs/`-vs-`readme/` split that future-work §3o (line
  1319) and the collation §8 already flag as a live hazard.

### P1-4 · STALE · MASTER-COLLATION-index build-status is two blocks behind
- **A:** `readme/MASTER-COLLATION-index.md` §1 lines 15–22 — "…**B5.5 merged + tagged**… **Queue next:
  B5.6** (zone visuals…) → B5.7 → B5.8 → B6." Written before the B5.6 split even existed.
- **B:** Reality — B5.6 was split into B5.6a (merged) + B5.6b (in progress). `BASTION_ARCHITECTURE.md` §6
  (lines 432–439) has the correct current state.
- **Impact:** the collation index bills itself as the "read-first reorientation" doc (line 5) — the one a
  new session opens first — yet its headline status is the most stale in the corpus.
- **Recommend:** update §1 to "main green at B5.6a; B5.6b-1 in progress," and point the status line at
  `docs/BASTION_RUN_LOG.md` + `BASTION_ARCHITECTURE.md` §6 as the live source rather than restating a
  snapshot that goes stale each block. (Consider making the collation status a pointer, not a copy.)

### P1-5 · STALE · FRAMEWORKS doc header says "B5 merged, B6 next"
- **A:** `readme/BASTION-SYSTEM-FRAMEWORKS.md` line 6 — "Build status context: **B5 merged**
  (paint→claim→walk→mine→stone works); **B6 = stockpiles/hauling next**."
- **B:** Reality — B5.5 and B5.6a merged since; B6 is several blocks out (B5.6b, B5.7, B5.8 precede it per
  the queue).
- **Recommend:** drop the hard-coded status line (it's incidental to a *frameworks* reference) or replace
  with "see `docs/BASTION_RUN_LOG.md` for live status." Same single-source fix as P1-4.

---

## PRIORITY 2 — real drift, lower blast radius

### P2-1 · STALE · future-work section ordering is scrambled; §-refs are hard to locate
- **Where:** `readme/future-work-and-deferred-ideas.md`. The `§3x` sub-sections are in near-random order and
  some sit under the wrong top-level heading:
  - Body order runs 3a,3b,3c,3d,3e, **3i,3j,3k,3l,3m, 3f,3g,3h**, then the `## 4. Open watch-items` heading
    (line 712), then **3z,3y,3x,3w,3v,3u,3s,3t,3r,3q,3n,3o,3p**.
  - i.e. §3f/§3g/§3h appear *after* §3i–§3m; and §3n–§3z (researched *systems* — mining, boundaries, world
    tissue, materials, nature-sim) are physically nested under "## 4. Open watch-items from build sessions,"
    which is the wrong bucket (they're §3 researched-systems, not watch-items).
- **Impact:** dozens of cross-refs across the corpus point at these sections ("future-work §3v," "§3w,"
  "§3q," "§3e-schema," "§3x(a)"). They still *resolve* (the anchors exist), but a human scanning the doc
  top-to-bottom can't predict where a section is, and the "## 4" mis-nesting hides ~15 major system designs
  under a heading nobody reads for system designs.
- **Recommend:** re-order §3a–§3z alphabetically (or by theme) under a single `## 3. Researched systems`
  heading, and move the genuine watch-items (the bullets at lines 1313–1329: god-anchor aggro, TRAVEL_SPEED,
  docs-vs-readme, retro-tag) into their own `## 4`. Purely editorial, but it's the single biggest
  legibility win in the corpus and it de-risks every §-ref.

### P2-2 · DUP + internal CONTRADICTION · the zone↔asset "purpose" enumeration is specified 4× with drift
- **Canonical intent:** one shared purpose enumeration for zones and assets (future-work §3e-schema /
  §3m / §3q; frameworks §2).
- **Drift across the copies:**
  - future-work §3m lines 545–549 lists the **`purpose` bullet TWICE, back-to-back** — the first
    (housing/production/defense/social/faith/storage/infrastructure, 7 kinds) and the second
    (…/**commerce**/…/**agricultural**/…, 9 kinds). Two adjacent definitions of the same field, disagreeing.
  - future-work §3q line 1216 lists zone types as residential/industrial/religious/**commercial**/civic/
    defensive/storage/**agricultural** (8).
  - frameworks §2 lines 22–24 lists the 8-kind version (…commercial→commerce… agricultural→farming).
  - future-work §3e-schema (line 560+) and §3z-taxonomy restate it again.
- **Impact:** whichever session first *implements* the enumeration as a Rust enum will pick one of these
  and the others silently become wrong. This is exactly the "lock the vocabulary NOW" decision the docs
  themselves flag as high-value (§3m line 560, "Soft preference… lock the shared vocabulary").
- **Recommend:** delete the duplicated bullet in §3m; declare ONE canonical list (the 8/9-kind version in
  frameworks §2 is the fullest) in a single place; have every other mention say "the zone↔asset taxonomy
  (frameworks §2)" rather than re-listing. Single-source before it becomes a code enum.

### P2-3 · CONTRADICTION · Agency Bible is "v0.2" everywhere except its own title (v0.1)
- **A:** MEGA-PROMPT input #3 (line 44) — "The Agency Bible (**fact-checked v0.2**: facet/value distinction
  + conflicts, facet-similarity relationships, memory-driven drift, the FOCUS system)." The run log echoes
  "Agency Bible" as a fact-checked input.
- **B:** `readme/agency-bible.md` line 1 — "# Project Bastion — Agency Bible **v0.1**."
- **Impact:** low functional risk (the content is the fact-checked one — §5b exists with the corrections),
  but the version mismatch means "is the on-disk bible the fact-checked one?" can't be answered from the
  title. A reader could think a v0.2 is missing.
- **Recommend:** bump the file's title to v0.2 (the fact-check corrections it references are already in
  §5b.2), OR correct the mega-prompt/run-log to "v0.1." Pick one number.

### P2-4 · STALE · "ground-follow / draping overlay" is attributed to B1.8, but it shipped in B5.6a
- **A:** future-work §2 line 54 — "**Ground-follow designation overlay** — already folded into **B1.8**
  (drape overlays over terrain/slice…)." And §3x(a) lines 878–886 present outline-draping as future work.
- **B:** Reality — outline draping was **built and merged in B5.6a** (run log: "terrain-conformed overlay
  DRAPING … `bastion::draped_rect_outline` + `overlay_surface_z`"). B1.8 is deferred to Phase 4b and hasn't
  been built.
- **Impact:** anyone reading future-work would look for draping in B1.8 (unbuilt) and miss that it's already
  in `voxygen/src/bastion/mod.rs`, and might re-plan it.
- **Recommend:** mark §2's ground-follow bullet and §3x(a) as **DELIVERED in B5.6a** (fill draping continues
  in B5.6b-1); correct the "folded into B1.8" attribution. Note the reusable `overlay_surface_z` seam is the
  §3w boundary-overlay's future customer (already noted in the backlog).

### P2-5 · STALE · asset-pipeline progress differs sharply between collation and frameworks
- **A:** MASTER-COLLATION §5 (lines 134–152) describes the asset pilot as producing essentially **one asset**
  (the timber cottage) plus a viewer + two prototype harnesses — "what actually happened."
- **B:** `BASTION-SYSTEM-FRAMEWORKS.md` §3 line 30 — "live: asset-lab, **12 REAL assets, ladder rungs 0–9
  PASS**."
- **Impact:** the two "current state of the asset workstream" claims are far apart (1 asset vs 12, pilot vs
  all 9 rungs passed). A session picking up the asset work can't tell which is current.
- **Note:** the asset workstream is isolated and self-tracks via `readme/COMPONENT_SYSTEM_LOG.md` (the
  designated resume point). Neither the collation nor the frameworks header is the source of truth for it.
- **Recommend:** have both docs point to `COMPONENT_SYSTEM_LOG.md` for asset-pipeline status instead of each
  stating a number; reconcile the "12 assets / rungs 0–9" claim against that log and correct whichever is
  stale.

---

## PRIORITY 3 — orphans, hedged refs, and cosmetic drift

### P3-1 · ORPHAN · superseded prompt files referenced as archival, but absent from the tree
- **Where:** MASTER-COLLATION §6 (lines 162–163) and §9 archive (line 239) name
  **`asset-generator-prompt.md`** and **`component-system-prompt.md`** as "archival stepping-stones … keep
  in `readme/archive/` or delete."
- **Reality:** neither file exists in the tree (only `asset-lab-claude-code-prompt.md` and the superseding
  `MASTER-asset-tooling-prompt.md` are present). `readme/archive/` also does not exist.
- **Recommend:** if these were never committed, strike them from the checklist (or note "never in-repo");
  they read as files to go find and archive, but there's nothing there.

### P3-2 · ORPHAN · "taxonomy/census docs (`readme/*TAXONOMY*`)" don't exist in `readme/`
- **Where:** MASTER-COLLATION §5 line 137 ("Census done (`readme/*TAXONOMY*`)") and §9 line 245
  ("taxonomy/census docs").
- **Reality:** no `*TAXONOMY*`/`*census*` file exists under `readme/` (likely lives in the isolated
  `asset-lab/`, which is untracked).
- **Recommend:** correct the path (point at wherever the census actually landed in `asset-lab/`) or drop the
  `readme/` glob so the reference doesn't dangle.

### P3-3 · ORPHAN (hedged) · `readme/reference-images/` promised but absent
- **Where:** MEGA-PROMPT input #8 (lines 55–58) — "`readme/reference-images/` (if present)"; and
  `BASTION_BACKLOG.md` lines 244–254 say Ben provided a RimWorld screenshot as the B5.6b visual target "to
  put there."
- **Reality:** `readme/reference-images/` does not exist. (One evidence PNG,
  `evidence-b56-floating-selection-bleabrolm.png`, sits in `readme/` root, matching the mega-prompt's
  "filenames starting `evidence-`" convention.)
- **Impact:** low — the mega-prompt hedges with "if present." But the backlog implies the RimWorld
  reference was captured, and it isn't in-repo, so B5.6b-1 can't judge against it.
- **Recommend:** either create `readme/reference-images/` and drop the RimWorld shot in (so B5.6b-1 can use
  it, per its own gate), or note in the backlog that the reference wasn't committed.

### P3-4 · STALE · MASTER-COLLATION §9 says future-work runs "§3a–§3x"; it runs to §3z
- **Where:** MASTER-COLLATION §9 line 220 — "`future-work-and-deferred-ideas.md` — THE catch-all (**§3a–§3x**:
  …)." The doc actually contains §3a–§3z (materials §3z, nature-sim §3y are the latest additions).
- **Recommend:** update the range to §3a–§3z; folds naturally into the P2-1 re-order.

### P3-5 · DUP (low risk) · B-AG block numbering is non-sequential and the "mind" block name varies
- **Observations:**
  - MEGA-PROMPT Phase 3 (lines 250–258) orders B-AG1, **B-AG3, B-AG4, B-AG5, B-AG6, B-AG2** — B-AG2 is
    deliberately last (fine, but the out-of-order numbering invites confusion).
  - The "Mind" system is **B-AG3** in the design doc (§ line 253-region), mega-prompt, and gap ledger
    (`df-feature-gap-ledger.md` line 45, "B-AG3 (Agency Bible §5b)"), but `agency-bible.md` line 4 frames
    only "B-AG1, B-AG2" as "the main doc's agency blocks" and describes the Mind under §5b without the
    B-AG3 label.
- **Impact:** minor; the §5b cross-ref resolves. But a reader mapping blocks→bible sections has to infer
  that B-AG3 == Agency Bible §5b.
- **Recommend:** add a one-line "B-AG3 = the Mind (§5b)" note to the Agency Bible's intro so the block
  numbering is self-documenting; consider renumbering the queue so B-AG order matches build order.

---

## Cross-cutting recommendation (the root cause of most STALE findings)

Five of the P1/P2 findings (P1-1, P1-2, P1-4, P1-5, P2-5) are the same failure mode: **build status is
copied into multiple high-altitude docs and then not updated in lockstep.** The run log + `BASTION_ARCHITECTURE.md`
§6 + `BASTION_CONSISTENCY.md` stay current (they're per-block bookkeeping); the collation index, the
frameworks header, and the mega-prompt queue drift because nothing forces them forward each block.

**Single-source fix:** make `docs/BASTION_RUN_LOG.md` + `BASTION_ARCHITECTURE.md` §6 the ONE status source,
and have the collation index, frameworks doc, and mega-prompt say "see the run log / architecture §6 for
live status" instead of restating a snapshot. Snapshots in an append-only, block-per-session project are
stale by construction.

---

*End. All findings are for Ben/the architect to action; nothing here was auto-fixed. Line numbers are as of
2026-07-09 and will shift if the docs are edited.*
