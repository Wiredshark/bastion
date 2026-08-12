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
2. TERRAIN VALIDATION: every preset column standable within ±1 z; on
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
