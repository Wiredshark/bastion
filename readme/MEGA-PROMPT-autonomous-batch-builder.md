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
  `BASTION_CONSISTENCY.md`, `BASTION_RUN_LOG.md`, `BASTION_LOOP_LOG.md`) all live in **`E:\veloren-master\readme\`**.
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
   `readme/future-work-and-deferred-ideas.md` (§3q–§3x cover zoning, mining, boundaries, roads, site-prep,
   action animations — several are near-term: B5.5/B6/B7-era).

## 🥇 FIRST ACTION THIS SESSION — CATCH-UP DOCUMENTATION PASS (do this BEFORE building anything)
`readme/BASTION_ARCHITECTURE.md` does not exist yet, but B0–B4 are already built. **Before you build the next
block, create it and document everything built SO FAR**, by reading the repo, the findings docs, the run log,
and the git history. This is a one-time retroactive pass so the "how it all works" map exists and is accurate;
after this, every block (including this one) just edits and adds to it.

Produce `readme/BASTION_ARCHITECTURE.md` covering everything already built (see the full contents list in
"MAINTAIN THE SYSTEM ARCHITECTURE GUIDE" below):
- The pillars & invariants actually in force.
- Each system B0–B4 built and how they connect — the headless harness (what it is + how it works), the
  colonist model (rtsim NPC + bastion record + promote/demote boundary), the god-anchor (inert + invulnerable),
  the designation→job-board→arbitration→pathing loop — with **where each lives in the code** (real crate/
  module/symbol names, pulled from the actual repo, not guessed) and **how each is tested**.
- The build methodology, reused Veloren machinery, and standing gotchas.
- State (B0–B4 done, B5 next) + pointers to the design docs/findings/logs.
Reconstruct it faithfully from what's actually in the repo — if something is unclear, read the code to confirm
rather than guessing. **Then continue to the queue and build the next block**, updating this doc as part of
that block's work. This catch-up pass is the priority first task; do not skip it.

## THE QUEUE (authoritative — the COMPLETE remaining work from the whole project, in build order)

Detect what's already merged (git tags `bastion-block-*` + `BASTION_RUN_LOG.md`), then continue from the
first unbuilt item. Status legend: **[MERGED]** done · **[PROMPTED]** has a dedicated builder prompt file
(prefer it as the block spec; the design doc entry is authoritative if they conflict) · **[DESIGNED]**
design-doc entry with Done-when (build from the doc) · **[LEDGER]** inventoried in the DF Gap Ledger,
needs an architect design pass first — do not build.

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
B5.5   [PROMPTED] PATCH: zone/designation deletion + item-drop pile aggregation (merge-never-delete,
                  conservation-exact) — see readme/B5.5-zone-delete-drop-aggregation-prompt.md. Found in
                  the first live demo; do BEFORE B6 (B6 hauling consumes piles).
B6     [DESIGNED] Stockpiles, hauling, reservations (conservation invariants; haul range = colony boundary
                  field if built, see future-work §3w)
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
