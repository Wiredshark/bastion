# FOUNDING PRESET v1 — the player path gets the script path's certification

**Chartered by Ben live-playtesting (2026-08-12): "can we have a preset colony
with the basics and see if it works." Packet authored by Fable (orchestrator)
directly — both builder lanes were down at craft time; review on revival.
Checklist entries 1–7 apply (readme/PACKET-CRAFT-CHECKLIST.md).**

## §0 WHY (the observed failure, in the owner's own words)

Ben founded a colony via the in-game UI at a random town. The spawned
colonists **leashed back to the original colony's coordinates** (his
diagnosis — an observation, adopted over the initial no-anchor story):
one global anchor/JobBoard binds any spawned colonist to THE colony, so a
far-away founding produces a cross-country march. Separately, even at a
sane location the UI founding carries **none of the survival preset** that
every certified run received from `script-15`: no designations (empty job
board), no founding stock, so nothing to do and nothing to eat.

**The harness path works; the player path was never exercised —
gate-must-test-live-path at the feature tier.**

## §1 THE PRESET (script-15's proven kit, made relative)

Source of truth: `script-15-item8-endurance.txt` (absolute coords proven
across v3/v4/v5). Expressed as offsets from founding point F = the world
position the god targets with the founding action (script anchor was
`(15216.5, 16016.5, 419)`):

| element | offset region (min..max, relative to F floor) |
|---|---|
| stockpile | (-2, -4, -1) .. (+2, +1, 0) |
| farm | (-7, -4, -1) .. (-3, +1, 0) |
| bed | (-3, -3, 0) .. (-2, -2, +1) |
| founding stock | #105 `FOUNDING_SEED_STOCK` (8 seeds) + founding food, dropped adjacent to stockpile — the FIXED LIVE PATH from #105, reused not reimplemented |
| colonists | 8 spawned AT F, anchored + promoted to the colony (the presence row's promotion path) |

**⚠ REVIEW: this table's z datum, its farm z-extent, "founding food", and
"anchored" are all amended by §8 (B1/B2/B3/B4) — read §8 before coding §1.**

Placement rule: z offsets are relative to the terrain floor at each
column (the preset must survive ±1 z variation); a column failing the
standability check shifts the whole preset per §3 validation, it does not
silently truncate a region.

## §2 TEST ENVIRONMENT — the RESOURCED FLAT ARENA (Ben's call)

The flat arena gains a deterministic RESOURCED VARIANT (env-gated beside
`BASTION_FLAT_ARENA`): placed at fixed offsets from arena center —
a tree cluster (chop), a stone outcrop (mine), open flat ground (found).
Same layout every run: **a red means the FEATURE broke, never the
terrain** — the matched-control property built into the world. This
variant becomes the standing resourced proving ground for all future
feature tests AND Ben's own founding sandbox (owner's play world = test
world).

## §3 THE UI FOUNDING ACTION (the new work)

1. God targets F via the overseer founding action (existing UI entry).
2. TERRAIN VALIDATION (⚠ §8 N6: this check already exists — reuse
   `column_surface_z` / `resolve_column_surface`, do not write a second
   standability rule): every preset column standable within ±1 z; on
   failure, refuse with a player-visible message naming the reason
   ("uneven ground — need a flatter site"). Refusal is a first-class
   outcome with its own emit (`bastion: founding refused reason=...`) —
   refusal-needs-refusal-aware-consumers: the UI shows it, the log
   carries it.
3. On pass: place designations (registered rev increments), drop founding
   stock via #105's live path, spawn 8 colonists AT F, anchor + promote.
4. Emit `bastion: colony founded preset=v1 pos=... rev=...` —
   the live witness line (name-the-line law: every claim below reads
   from a named emit).

## §4 THE ONE-COLONY BOUNDARY (named, not fallen-through)

v1 is ONE COLONY PER WORLD. Founding while a colony exists **REFUSES
with a message** ("your colony already lives at X — relocation and
multiple colonies are future features"). Deliberately chosen over
relocate/recruit (each deferred WITH a row: relocation → settlement arc;
recruit-at-distance → multi-colony/needs-in-rtsim horizon, where Ben's
leash-back observation is filed as live evidence). The refusal emits by
name. **The leash-march fallthrough becomes impossible: spawn-bind only
happens through the founding action or explicit future recruit flows.**

## §5 ACCEPTANCE (on the resourced arena, via the ACTUAL UI)

| # | measure | witness (named emit) | planted failure |
|---|---|---|---|
| A1 | founding places full preset | `colony founded preset=v1` + designation rev | founding action with preset-placement disabled must NOT emit the founded line (red by name) |
| A2 | colonists STAY (no leash-march) | positions within R of F across the window | spawn WITHOUT anchor → drift beyond R (red) — the Ben-observed failure, planted |
| A3 | the loop runs: till → sow → eat on founding stock | farm/eat emits (v5's instruments) | — (covered by A1/A2 controls) |
| A4 | second founding REFUSES by name | `founding refused reason=colony_exists` | boundary check disabled → silent second colony (red) |
| A5 | terrain refusal fires and names its reason | `founding refused reason=terrain` on a deliberately bad site | — |
| F8-INCLUSION (CONVERGENCE) | the arena's tree/stone give REAL chop/mine completions through the generic path — **this acceptance run doubles as F8's missing inclusion evidence** (external finding 1): observe `bastion: job completed` with drop+XP on a real completion, live | the run itself is the witness |

Scoring refusals: the founding-preset scorer writes their own §"will not
do" before the run (program standard). Run mode: fast once certified;
these acceptance runs are minutes-scale regardless.

## §6 FIXTURE UPGRADES RIDING THIS ROW (external findings 2+3, verified scope)

- **Wiring mutation test** (finding 2): a polarity-flip at the LIVE
  `completion_outcome(...)` call site must fail an integration-tier
  fixture — the RED demonstration moves to the caller tier
  (falsifier-needs-its-own-control at the wiring).
- **Cave-in boundary honesty** (finding 3): either cave-in joins
  `CompletionOutcome` or the suppression fixture renames to what it
  actually asserts (`..._suppresses_drop_xp_and_production_log`) plus a
  separate explicit cave-in regression test.

## §7 EXPECTATIONS

First UI-founded colony ever certified. Pass probability genuinely
unknown — the player path has never run once; every failure emits its
name (the §5 witnesses). A failure here is a find, not a setback: it is
exactly what the owner's playtest lane exists to surface.

---

## §8 REVIEW GATE — Builder Opus 5c, 2026-08-12 (independent, post-restart)

**VERDICT: APPROVED WITH AMENDMENTS.** The row is right, the convergence
with F8 is real, and §4's boundary is the correct call. Eight items
BLOCK the build (each would silently cost the run or make the builder
code the wrong thing); seven are noted without gating. Every item below
was read in the tree, not inferred — citations are
`bastion/wip-batch-verify @ac8ca746d0` (worktree
`.engine-integration-wt`), which is where the crate-split code lives;
this packet and the roadmap live on `bastion/block-B6HAUL`, which does
NOT contain `completion_outcome` at all. **The branches have diverged
662/850 commits — "which branch does the founding preset get built on"
is a real question this packet must answer before line one.**

### VERIFIED GOOD (stated so the builder can stop re-deriving it)

- **§1's twelve offsets are ARITHMETICALLY EXACT.** F verified from six
  driver logs (`driver-item8-endurance*.log`: `sent BastionSpawnColony
  pos=Vec3 { x: 15216.5, y: 16016.5, z: 419.0 } count=8`). Against
  reference `(15216, 16016, 419)` = `(floor F.x, floor F.y, F.z)`,
  every one of script-15's designation numbers reproduces:
  stockpile `15214 16012 418 / 15218 16017 419` → `(-2,-4,-1)..(+2,+1,0)`;
  farm `15209 16012 418 / 15213 16017 419` → `(-7,-4,-1)..(-3,+1,0)`;
  bed `15213 16013 419 / 15214 16014 420` → `(-3,-3,0)..(-2,-2,+1)`.
  **No offset is wrong. The DATUM is (B1).**
- **#105 is genuinely a fixed live path, reused not reimplemented** —
  `server/src/sys/msg/in_game.rs:1293-1339`: the live
  `ClientGeneral::BastionSpawnColony` handler already drops
  `FOUNDING_SEED_STOCK` and emits `CreateColonyPresenceEvent`. §1's
  "reused not reimplemented" claim is true *for the seeds*.
- **§4's boundary is the right choice** and its stated mechanism holds
  *given B6*: with one colony per world there is no second spawn to
  march. The deferral rows are named as the program requires.

### BLOCKING (8)

- **B1 · PIN THE Z DATUM — the one-block error that sinks the preset.**
  §1 says "relative to F floor" and "relative to the terrain floor at
  each column"; the arithmetic that reproduces script-15 is relative to
  **F.z = 419.0, the founding position's own z (the first air cell the
  god's feet occupy)** — not to the topmost solid block (418). One
  block either way and the stockpile floor is inside solid ground or
  hanging. This is not pedantry on the arena: `bastion_flat_arena.rs:41,
  72-78` puts the slab's first air cell at `FLAT_ARENA_Z = 400` and
  **spawns the player at 401** ("+1 clears any landing jitter"), so the
  z the god reports at founding time is legitimately 400 *or* 401
  depending on where they stand. **Required: F_z is DERIVED from terrain
  at F's column (the resolver in N6), never taken from the reported
  position; state that all §1 z offsets are relative to that resolved
  first-air cell; carry the script-15 reproduction above as the
  packet's own arithmetic self-check.**
- **B2 · THE FARM ROW MUST NOT CARRY A Z-EXTENT.** Farm is `Area2D`
  (`bastion_jobs.rs:5490-5507`): voxygen never sends a z_extent for it,
  and `region.min.z` is a HINT into `column_surface_z`'s ±window search,
  resolved per column at registration (`:5508-5532`). §1's farm
  `(-7,-4,-1)..(-3,+1,0)` invites the builder to send a z range the live
  path does not carry. **Required: farm expressed as x/y footprint + a
  z HINT, explicitly named as a hint.**
- **B3 · "FOUNDING FOOD" DOES NOT EXIST.** The only founding constant is
  `FOUNDING_SEED_STOCK: u32 = 8` (`bastion_jobs.rs:1951`, seeds only —
  `FARM_SEED_ITEM` at `:1921`); script-15's own header says it verbatim
  ("**NO give_item** ... FOUNDING_SEED_STOCK=8 is the colony's entire
  food-producing capital"). §1 smuggles "+ founding food" in under the
  "reused not reimplemented" banner. **Required: either DROP it (and fix
  A3, which currently reads "eat on founding stock" — with seeds only,
  the first eat waits on a harvest, which is NOT minutes-scale and
  breaks §5's own run-length claim), or charter it as NEW work with its
  quantity's derivation named (a magic number with a certification
  hanging on it is #103's ghost).**
- **B4 · "ANCHORED + PROMOTED" NAMES A PRIMITIVE THAT ISN'T THERE — and
  A2's planted failure is unbuildable as written.** The spawn path
  (`server/src/rtsim/mod.rs:443-514`) sets exactly one binding:
  `npc.home = nearest site`. There is no colonist anchor to omit.
  Activity zones — the soft magnet — register ONLY for
  `DesignationKind::Zone` (`bastion_jobs.rs:5484-5489`); stockpile/farm/
  bed do not create one. **What actually holds colonists at F is the
  WORK being at F.** That is exactly Ben's observation restated
  correctly: his colonists marched because the jobs were at the old
  colony. **Required: §1 names the real retention mechanism (jobs +
  designations at F; `home` = nearest site is the simulated-mode half),
  and A2's planted failure becomes "found WITHOUT the designations
  (stock + colonists only) ⇒ colonists leave R" — a toggle that exists
  and reproduces the owner-observed failure exactly.**
- **B5 · A1's PLANTED FAILURE IS VACUITY COSTUME #3 (mechanism deleted).**
  "Founding action with preset-placement disabled must NOT emit the
  founded line" removes the subject and the witness together — it proves
  the emit is coupled to *something*, never that it means "FULL preset".
  Checklist entry 7's one question: *what would make this go the other
  way, and is that the axis I claim?* **Required: the planted failure is
  a PARTIAL preset (place everything except the farm) and the A1 witness
  must go RED on it — i.e. the founded line carries/implies enough
  (region count or per-element rev) to discriminate full from partial.**
- **B6 · THE ONE-COLONY PREDICATE MUST READ THE THING THAT SURVIVES A
  RESTART.** Colonists persist (rtsim records); JobBoard/designations do
  NOT (roadmap 2026-08-12 (10), found live restarting the celebration
  world), and the presence entity is not persistence-backed either. A
  predicate reading the board or presence reads "no colony" after any
  restart **while the first colony's colonists are still standing in the
  world — and then blesses the second founding whose leash-march §4
  exists to make impossible.** **Required: the predicate's source of
  truth is named (rtsim colonist records), its temporal shape declared
  SNAPSHOT vs ACCUMULATOR (checklist 1), and the extinct-colony case
  ruled (all colonists dead ⇒ re-founding must be permitted; ties to the
  COLONY TERMINAL sentinel row).**
- **B7 · BINARY PROVENANCE IS ABSENT (checklist 5) — and this run needs
  TWO binaries.** No pin, no stamp check, no compiled-crates-list rule
  in §5. This is the first acceptance to run through a real client:
  the celebration session proved that path breaks on its own
  (`64ad49dc1e`, two uncovered `PresenceKind::Colony` matches, first
  voxygen build since the variant landed). **Required: server-cli AND
  voxygen both verified by the OUTPUT's compiled-crates list, both
  stamps recorded, before A1-A5 are scored.**
- **B8 · THE NEW ENV GATE MUST BE REGISTERED.** `BASTION_FLAT_ARENA` is
  entry `common/src/host_input_manifest.rs:211-216`, class
  `GameplayVariant`, in a scanner-backed registry whose whole purpose is
  that a run's attestation can say which host inputs were live. A
  resourced-variant gate that isn't registered leaves the manifest stale
  and the attestation lying by omission. **Required: register it, class
  `GameplayVariant`, at spec time not after.**

### NOTED, NOT GATING (7)

- **N1 · Checklist entry 2 (plot/plan model, DECISIONS #102) is claimed
  and not carried.** §1 is a raw offset table; the roadmap's "#102 plot
  rider (the preset = the first plot template)" never made it into the
  packet body. The offsets are correct — the FORM is the exposure, and
  entry 2's own cost line is "un-baking spatial assumptions later is the
  most expensive rework class there is." Cheap now: name the preset a
  plot template with role-tagged regions, offsets as its data.
- **N2 · "Via the ACTUAL UI" is two different tiers and the packet
  conflates them.** `bastion_playtest`'s `spawn`
  (`client/src/bin/bastion_playtest.rs:174,449-456` →
  `client/src/lib.rs:2844`) already sends the LIVE
  `ClientGeneral::BastionSpawnColony` — the same message the in-game
  action sends, handled at `in_game.rs:1293`. Recommend: A1-A5 run at
  the driver tier (deterministic, repeatable, scriptable), plus ONE
  human mouse-click founding as the widget-wiring witness. Ben's
  original failure was at the widget; the regression value is at the
  message.
- **N3 · Run mode: "fast once certified" needs the human carve-out.**
  Standing law's own second exception is human-in-the-loop sessions.
  A mouse-click leg is real-time by definition; the driver legs are not.
- **N4 · R is undefined and the floor isn't zero.** Spawn scatter is
  `rng.random_range(-5.0..5.0)` in x/y (`rtsim/mod.rs:491-495`) — R must
  clear ~7 blocks before it measures anything. State R and derive it.
- **N5 · A5 needs its not-always-refusing control named.** It already
  has one implicitly (A1 founds successfully on the same arena); say so,
  or the refusal bar is satisfiable by a founding action that refuses
  everything.
- **N6 · §3's terrain validation already exists — reuse it.**
  `column_surface_z(terrain, x, y, hint)` and `resolve_column_surface`
  do exactly the ±window standability search, and are what Farm and
  relative-mode z_extent designations already use. Writing a second
  standability rule is how two answers to one question get shipped.
- **N7 · The arena's resourced variant must place its trees/stone AT
  GENERATION, in `override_chunk` (`bastion_flat_arena.rs:84-106`, which
  currently returns `ChunkSupplement::default()` on purpose), not by
  post-hoc `set_block`** — otherwise the "same layout every run"
  property dies at the first chunk unload/reload, and §2's whole
  matched-control claim ("a red means the FEATURE broke, never the
  terrain") goes with it.

### ON §6 (the external-findings riders) — scope corrected

§6's two riders survive review but one is mis-scoped; both are disposed
in full in `EXTERNAL-FINDINGS-TRIAGE-2026-08-12.md` (same commit).
Short form: **finding 2's "integration-tier fixture" is not buildable as
written** — `bastion-server` has no integration tier (no `tests/` dir;
the call site is inside a specs `System::run` with a ~40-storage
SystemData tuple, `bastion_jobs.rs:6590`). The affordable form is one
more extraction level (`apply_completion_outcome`), which covers four of
the five green-survival mutations; the fifth (the polarity computation
at `:11507`) and reachability itself are closable ONLY by the live run
this packet already schedules. **Finding 3 UPGRADES: cave-in is not just
mis-named in the fixture, it still re-derives `!is_emergency_access`
independently at `:14994` — the exact "third re-derivation" the
signal-split row claims at `:15065` to have eliminated.**
