# Project Bastion — Future Work & Deferred Ideas

**Status: CAPTURE DOC. Nothing here is queued or load-bearing.** This is the single home for ideas,
architectural insights, and watch-items that came up in design/build discussion but aren't yet in the main
design doc, the ledger, or a block. It exists so they survive across sessions. Promote any item to a real
design pass + block when the core game is proven and you want it. Companion to `BASTION_BACKLOG.md` (which
the build sessions append to per-block) — this doc is the architect-level "big deferred pieces," the backlog
is the running per-block capture.

---

## 1. Architectural "build-it-once" groupings (the high-value insights)

These are cases where several wished-for features are secretly **one system**. Building the shared engine
once and letting features ride on it is the same discipline as the world-verb action library (one library,
two drivers). Recording them here prevents building the same thing three times.

### 1a. Hazard Events — one radius-effect + NPC-reaction engine
**The insight:** falling-tree "timber!" damage, fluid/flood drowning, lava burns, rockfalls/cave-ins, and
explosion effects are **not separate features** — they're all instances of: *"something happens in a zone →
damage/effect applied there → nearby minds react (fear / injury / grief / grudge)."*
- **Build once:** a Hazard Event system that takes a location, a radius, an effect (damage/type), and emits
  an event the mind system (B-AG3) and threat system (B8) consume. Timber, flood, lava, rockfall, disaster
  all become *callers* of it.
- **Depends on:** B8 (damage/threat plumbing) + B-AG3 (minds that react — fear, grief memory, grudge). The
  fluid work (DF-FLUID) plugs in here as flood-hazard.
- **Why grouped:** "timber damages a colonist" and "a flood drowns the colony" are the same engine wearing
  two costumes. Reproductive of the trap/mechanism insight (1b).
- **DF flavor payoff:** a colonist hurt by a falling tree → injury + a fear/trauma thought + possibly a
  grudge against the god who felled it. That emergent story is free once the engine feeds B-AG3.

### 1b. Trigger → Link → Effect — traps + mechanisms + operable terrain
(Already in the DF Gap Ledger §E — restated here for completeness.) Traps (DF-TRAP), mechanisms/levers/
pressure-plates (DF-MECH), and operable terrain — doors/floodgates/drawbridges (DF-OPERABLE) — all reduce to
**trigger → link → effect.** Build the shared engine once (DF-MECH); traps and operable terrain fall out
cheaply. Veloren has **no** player-buildable trap system, so this cluster is build-not-wrap. God-game
reframe: designate in peacetime → colonists build+wire → triggers autonomously (fits B8 defense).

### 1c. Staged voxel removal — chop/mine/deconstruct share a "progressive removal" pattern
Tree-felling should remove the tree **progressively over the work-tick** (leaves then trunk, top-down), not
blink out at completion. The same progressive-removal driven by work-progress applies to mining a large
volume and deconstruction. **Belongs in B5** (the work-tick already exists there). Doing it work-tick-driven
from the start makes the later fall-animation a one-line hook instead of a refactor.

---

## 2. Deferred polish (cheap, visual, no sim impact — do anytime)

- **Faked tree-fall animation** — on chop-completion, tip the tree mesh over on an axis and fade it out.
  Pure rendering flourish, touches no sim, no dependencies. ~95% of the "timber" *feeling* for ~5% of the
  cost of real physics. (Real rigid-body tree-fall physics is explicitly NOT wanted — same category as
  fluid: a whole physics subproject, fights LOD/multiplayer, DF itself doesn't do it.)
- **Ground-follow designation overlay** — already folded into B1.8 (drape overlays over terrain/slice
  instead of floating flat quads). Listed here only so the cross-reference exists.
- **Slice quality-of-life** — re-anchor to local ground on focus move, "Slice here" (click a block → cut at
  its Z), "Reset slice" — already folded into B1.8.

---

## 3. Researched systems not yet written into a design doc

### 3a. Fluid / water physics (the big one — see also DF-FLUID `$$$`)
Discussed in depth; capturing the build approach so it's not lost. **Not real fluid dynamics — game fluid:**
- **Approach:** 3D **cellular automata** (falling-sand / DF / Minecraft style), NOT SPH particles. Each fluid
  cell follows simple neighbor rules (fall, spread, equalize; add a level 0–7). Optional **pressure** pass
  for DF-style plumbing (water finds its level, flows up a U-bend) — that's what makes moats/pumps/drowning
  traps work.
- **The hard 80% is NOT the flow rules** (those are a weekend). It's making it cheap and correct across a
  **chunked, streamed, server-authoritative** world: fluid only simulates in loaded/active chunks; cells
  **sleep at rest and wake when disturbed** (a dig near water wakes neighbors → they flow in); sane behavior
  at **chunk boundaries**; persists in the save (B10). This is the same LOD discipline as minds/settlements.
- **Build path:** prototype a standalone 3D falling-sand toy first (prove flow feels right, divorced from
  Veloren; Rust has lots of falling-sand prior art + wgpu GPU-compute which Veloren already uses) → solve
  sleep/wake + chunk boundary in the toy → integrate as a server system in loaded chunks, tied to terrain
  edits → then cascades (temperature freeze/melt = DF-TEMP; second fluid = DF-MAGMA; interactions = obsidian;
  pressure structures = DF-PUMP).
- **Why deferred:** non-load-bearing enrichment; the game is a great DF-like without it. It's the isolated,
  optional capstone — build the whole game, then drop fluid in as a self-contained upgrade. Feeds the Hazard
  Events engine (1a) as flood-hazard.

### 3b. Dialogue & Voice (the thread never stubbed until now)
The value of any dialogue system is proportional to how much real *mind* sits behind the words — and Bastion
is building exactly that (B-AG3). Staged plan, cheapest → hardest:
1. **Surface-the-mind via templates (cheap, deterministic, do first).** Pipe B-AG3 state into the existing
   dialogue templates: an NPC greets by mood, mentions a grudge ("haven't forgotten my brother"), refuses
   warmth at low sentiment, voices an unmet need ("no time to pray, it weighs on me"). No LLM — just
   surfacing real state. Already ~10× more alive than vanilla RPG dialogue.
2. **Prayer & gossip (uniquely yours).** The Divine Politics **prayer feed** *is* a dialogue channel — the
   faithful petition the god in words; the response (answer/ignore/omen) is a conversational act with
   mechanical weight. **Gossip:** information spreads through NPC conversation (A witnesses a raid → tells B
   at the tavern → B mentions it to you days later). Uses B-AG3 memory as the transmission layer; makes the
   world legible through hearsay (very DF).
3. **Grounded-LLM dialogue (research-grade, finish-line).** Feed a local LLM the NPC's *actual* mind state
   as context and let it generate dialogue **constrained by real simulation facts** — the model provides
   fluency, the sim provides truth/memory/consequences. The combination nobody has (chatbots have language
   with no world; DF has world with no language; you'd have both). Words feed back: a promise → a memory, an
   insult → a grudge, a confession → a rumor. **Hard part:** keeping the LLM on-rails so it can't contradict
   the sim; latency/cost/local-model constraints. Dessert, not a course to cook early.

### 3c. Autonomous building — the construction model (answers "how do agents build things")
The design question B-AG6 (settlement growth) and B-AG5 (build verbs) both need answered. Resolved here so
the block has it waiting.

**Core model: agents build from a TEMPLATE CATALOG, not block-by-block.** An NPC/village doesn't invent a
structure voxel-by-voxel (that's a research problem, produces incoherent results, no colony sim does it).
It picks a **structure blueprint** from a catalog (house / wall / farm / workshop / well / road / palisade),
places it as a ghost, and colonists haul materials + work the build job (B-AG5) until the templated
structure materializes. Reuses Veloren's existing **site/structure templates** (worldgen already stamps
towns/houses/dungeons from templates) as the building vocabulary — this is substrate that largely exists,
not a system to invent.

**Race/culture is a dimension through the whole thing (Veloren already supports it).** Veloren worldgen
already produces **culturally distinct sites** — dwarven settlements build into mountains/underground,
humans/elves/orcs/cultists/gnarlings each build differently. So the catalog is **keyed by race/culture/
faction**: a dwarven village grows dwarven structures, a human town grows human ones, drawing from *its own
culture's* template pool. Race is not a separate feature to build — it's an axis that runs through every
tier below, and it ties the *look* of a settlement to the cultural identity the faith (Divine Politics),
faction, and genealogy systems already track. A dwarf hold that looks, worships, and builds dwarven is one
coherent cultural entity across rendering + faith + lineage.

**Build in ASCENDING tiers — each a testable working layer (Ben's call):**
1. **Fixed catalog (do first, cheapest).** Each race/faction has an authored building set; agents pick from
   their own culture's templates. Already gives cross-world variety for free (a dwarf hold ≠ a human hamlet
   because different pools). Villages look somewhat samey *within* a culture, but it *works*. Prove the
   growth loop here.
2. **Parameterized templates (moderate work, big payoff).** Variation *within* a race's style — size,
   material, small random features — so houses aren't identical, while staying recognizably in the cultural
   idiom. This is where settlements stop looking copy-pasted.
3. **Template composition (most work, richest, more risk).** Agents assemble structures from *culturally-
   appropriate pieces* (foundation + walls + roof + door), per-race piece pools so results stay coherent (no
   elf roof on a dwarf foundation). Most emergent-looking settlements; the race-keying is also what keeps
   composition from producing incoherent mash.

**LOD (same law as everything):** unwatched villages "build" abstractly at the rtsim tier (a low-res "site
gained a house" event); loaded villages build the actual templated voxels via the B-AG5 build verb. Reconcile
at the boundary.

**Verify at design-pass (the one real unknown):** how cleanly Veloren's site templates are **separated by
race** and whether they're **runtime-addressable as a catalog** the build system can invoke on demand (i.e.
"build a dwarven house here" at runtime), versus baked into the worldgen pipeline. Worldgen *placing* them at
gen-time is confirmed; runtime *invocation* is the thing a builder must grep for. Clean/addressable → mostly
wiring. Baked-in → moderate extraction to make them runtime-callable. Tractable either way; just changes the
cost estimate.

### 3d. Time controls / fast-forward (soak tool + player verb)
Veloren has `TimeScale` (reuse-verified, §2a). Two distinct uses:
- **Soak testing:** the Tier-1b soak should run **headless-accelerated** (harness ticks 30 game-days as fast
  as CPU allows, cheap abstract rtsim tier), NOT real-time watching. It's a fast check, not a 30-day wait.
- **Player verb (nice-to-have):** in-game speed control — fast-forward boring stretches, slow down for
  drama (raid, birth, rival-god move). WorldBox "watch at max speed" is core to god-game appeal. Surface in
  the HUD (B9) eventually. Note in cross-genre nice-to-haves too.

---

## 4. Open watch-items from build sessions (track, don't lose)

- **God-anchor aggro live-fire** — B3 verified the invulnerability buff + inert-anchor *code paths* but never
  field-tested a hostile ignoring the god avatar (machine locked mid-gate). **Fold into B8** (raids exercise
  it hard), or do a 30-second manual test: found a colony, get a hostile near the god avatar, confirm it's
  ignored. **Do not let this fall off** — it's the one B3 residual.
- **`TRAVEL_SPEED` eyeball** — B4's job-travel speed "looked brisk" in scenario timing but was never watched
  rendered. Eyeball in the B4/B5 demo; tune the constant if colonists glide comically.
- **`docs/` vs `readme/` split** — earlier sessions wrote findings to `docs/BASTION_*`; design + new
  bookkeeping live in `readme/`. The mega-prompt tells sessions to check both. Optional one-time cleanup:
  `git mv docs/BASTION_* readme/` to unify, if the split annoys.
- **Bookkeeping-vs-clean-tree tension** — sessions write bookkeeping docs into `readme/`, and the loop's
  clean-tree gate requires them committed before the next iteration. Sessions MUST commit their doc appends
  as part of commit-and-tag, or the loop halts "dirty after iteration." (Runtime loop logs are gitignored;
  the per-block bookkeeping is not and must be committed by the session.)
- **Retro-tag fuzziness** — B1.6/B1.7 were retro-tagged at one SHA because B1.7 content landed inside B1.6's
  commits. A `git reset --hard bastion-block-B1.6` may also revert B1.7 content. Recorded, harmless, just
  know the boundary there is approximate.

---

## 5. Design-pass debts (turn ledger lines into real blocks when reached)

- **Phase 5 DF-\*** (Tier 1 first: DF-TRADE, DF-TAVERN, DF-RELIGION, the production cluster, DF-ARTIFACT,
  DF-FOCUS, DF-HIST, DF-DIG-VERBS, DF-ROOMS, etc.) — each needs a Done-when design pass before it's
  buildable. The runner STOPS at these rather than building from one-liners. **DF-DIG-VERBS** (stairs/ramps/
  channels) is the most gameplay-critical of the backlog — a 3D fortress needs vertical traversal; consider
  promoting it near B5/B6 rather than leaving it in the pile.
- **Phase 6 Divine Politics (DP1–DP5)** — fully designed in the Divine Politics Bible; Tier-3 late; build
  only after colony core + agency are proven.

---

*End. This is the catch-all so nothing discussed-but-not-placed gets lost. Move items OUT of here into the
main doc / a block as they get a real design pass.*
