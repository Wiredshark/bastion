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

**Starter building catalog — what agents can build (the answer to "what do they have access to").**
The catalog IS the list of buildable things; it's culture-keyed (each race draws from its own set). Walls,
gates, and fortifications belong here — Veloren's worldgen already generates fortified towns/castles, so the
defensive templates exist (reuse, don't invent). First-pass catalog, each entry tagged with a **placement
type** (how it gets sited — this is as important as the template itself):

| Category | Entries | Placement type |
|---|---|---|
| **Housing** | home, longhouse | POINT — flat spot near settlement (the growth driver) |
| **Production** | workshop, farm plot, mine entrance | POINT/AREA — farm wants fertile flat; mine wants ore/mountainside |
| **Social / faith** | tavern, temple, well | POINT — central, near housing (feeds FOCUS/religion) |
| **Storage** | stockpile, granary | AREA — near production it serves |
| **Defensive** | **wall, gate, watchtower, palisade** | LINE/ENCLOSE — the hard one; see below |
| **Infrastructure** | road, bridge | PATH — connects points (roads between structures/settlements) |

**Placement types (the "how" — walls are the interesting case):**
- **POINT** — single flat footprint, scored by suitability (flatness/proximity/terrain/culture). Houses,
  workshops, temples. The easy case; reuses the B1.8 terrain sampler.
- **AREA** — a region rather than a point (farm plots, stockpiles). Score a flat contiguous region.
- **PATH** — connect two points along walkable terrain (roads, bridges). Reuses pathfinding to route.
- **LINE / ENCLOSE — walls, palisades (genuinely harder than a house).** A wall isn't a point — it's a
  *perimeter*. The logic: compute an enclosure around the settlement's built area (a rough hull / ring),
  route the wall along defensible terrain (follow ridgelines, span gaps), leave **gate gaps** where roads
  cross the perimeter, and **don't wall off the settlement's own farms/water** (they must stay inside or have
  a gate). Segments are placed along that computed line. This is a real layout problem, not a drop-in-place.
  **Staging:** (1) *player-designated* walls first — you paint a wall line in god/DF mode, colonists build
  along it (cheap, reuses B2a designate-paint + B-AG5 build verb); (2) *autonomous smart fortification* later
  — the colony computes its own defensive perimeter and builds it (harder AI, ties B8 defense).
- **GATE = operable, later.** A static gate (wall-with-a-hole) is easy and early. A *functional* gate that
  opens for friendlies / closes against threats is the **operable-terrain / trigger→link→effect system**
  (DF-OPERABLE, §1b) — a later build. Static gap early, operable gate later.

**Defensive building ties B8 (autonomous defense) + the soak.** A walled colony survives raids an open one
wouldn't — so walls/gates directly serve the "colony survives untouched" Tier-1b test. Static defensive walls
are reachable relatively early (templates exist, placement is the work); smart autonomous fortification +
operable gates are later, harder layers.

**Per-entry build spec (each catalog entry needs, at design-pass):** the template/asset, its placement type,
material cost (what colonists must haul — ties B6), skill/work-type + work-time, culture variants, and any
prerequisite (a gate needs a wall; a granary wants to be near farms). This is the per-entry work that turns
"the catalog" into buildable blocks.

**Bounding growth — carrying capacity, not magic caps.** Settlements must not sprawl infinitely. Bound growth
with *consequences*, not an arbitrary `MAX_SIZE`:
- **Resource limit** — a settlement supports only the population its surrounding food/water/materials allow
  (ties DF-FARM). Past that, growth stalls or population declines.
- **Territory pressure** — neighboring settlements compete for space; a town hemmed by mountains or another
  faction's border *can't* expand (ties Divine Politics rivalry). Growth into contested space becomes conflict,
  not sprawl.
- **Diminishing returns** — bigger settlements grow slower (harder to feed, more internal friction) → growth
  asymptotes on an S-curve, never a straight line up.
- **The soak is the guardrail** — Tier-1b already requires "populations shift plausibly, no runaway, no
  extinction-to-zero." Runaway expansion is a soak *failure*; carrying-capacity is the tuning knob that fixes
  it. Bound with consequences the player can *see*, not hidden numbers.

**Site suitability — where a structure goes.** When a settlement adds a structure, score candidate spots near
it and pick the best (this is runtime procedural placement — worldgen already does a gen-time version):
- **Flatness** — sample terrain height across the footprint; low variance = buildable, steep = reject.
  **Reuses B1.8's terrain-height sampler** (same one doing camera surface-follow + ground-follow overlays —
  one sampler, many uses).
- **Proximity** — near the existing settlement + roads, not overlapping other buildings.
- **Terrain type** — not water/cliff; farms want fertile flat ground, mines want ore/mountainside.
- **Culture fit** — dwarves build into mountainsides, humans into flat valleys (ties the race-keyed catalog).
- **No good spot → growth stalls** — which *also* helps bound expansion (a mountain-hemmed town literally
  can't sprawl). Suitability scoring and carrying-capacity reinforce each other.

### 3d. Time controls / fast-forward (soak tool + player verb)
Veloren has `TimeScale` (reuse-verified, §2a). Two distinct uses:
- **Soak testing:** the Tier-1b soak should run **headless-accelerated** (harness ticks 30 game-days as fast
  as CPU allows, cheap abstract rtsim tier), NOT real-time watching. It's a fast check, not a 30-day wait.
- **Player verb (nice-to-have):** in-game speed control — fast-forward boring stretches, slow down for
  drama (raid, birth, rival-god move). WorldBox "watch at max speed" is core to god-game appeal. Surface in
  the HUD (B9) eventually. Note in cross-genre nice-to-haves too.

### 3e. Asset-generation pipeline — Claude authors game content (novel workstream)
Claude has demonstrated strong voxel-art capability. Veloren is data-driven: structures/items live in **RON**
definitions and **`.vox` (MagicaVoxel) voxel models** — both Claude-authorable (RON is structured text;
`.vox` is generatable programmatically: build the voxel array in code, export). So the building catalog and
creature roster can be partly **generated**, closing a loop: *Claude authors the assets → the catalog serves
them → agents build/spawn them.*

**The workflow (the architect/builder loop, applied to art):**
1. **Reference-grounding** — point a session at Veloren's actual `.vox` assets + RON structure defs to learn
   real conventions (palette, scale, grid, style). Author *against* examples, never from scratch.
2. **Generate** — write the RON def + generate the `.vox` programmatically.
3. **Verify (automated + visual)** — score against the reference envelope with a **style-check harness**
   (below), AND render side-by-side vs. a real asset. Harness is the floor; the eye is final.
4. **Iterate** — refine ("blockier / darker palette / match this reference / add tonal ramps").

**Two generation strategies — VARIATION beats from-scratch (census-confirmed).** The game's own art is
componentized (creatures = part folders + per-family RON manifests; structures = prefabs + reserved-index
decor), so:
- **(A) From-scratch** — for net-new categories; must solve the whole style problem blind.
- **(B) Variation / modification packs (PREFER for variety)** — load a REAL asset and extend/recombine/
  recolor: add a room/wing/tower to a prefab, make a creature variant from existing parts + a new manifest,
  revary palette-index decor. **The base is already in-style — you only match the addition, a far
  lower-drift problem.** This IS the building-catalog "parameterized templates" tier (§3c) and the reliable
  path to *density* (30 house variants, 15 creature variants). Variation is how you cheaply fill the world;
  from-scratch is for genuinely new things.

**The style-verification harness (`style_check.py`) — quality is measured, not eyeballed.** Scores any asset
against the reference envelope and returns per-axis scores + PASS/REVIEW/FAIL, so a failing asset says WHAT
is off. Axes (all vs. REAL extracted reference values): **scale** (family vox/block + height envelope),
**palette** (mean nearest-color distance to reference), **mutedness** (flag oversaturated — Veloren is
earthy), **ramp/dither** (real assets shade via tonal ramps, not flat fills — the #1 generation miss; too
few shades for the voxel count = FAIL), **reserved-index discipline** (world structures must not use reserved
palette bytes 1,2,4–16 = leaves/water/hollow as literal colors), **density** (fill band), **category**
(schema completeness/consistency). Prototyped and validated in this design session — it correctly PASSED a
test cottage overall while flagging its flat shading (ramp axis), exactly the kind of catch a human waves
past.

**Caveats:** (a) VERIFY the exact consumed formats in-repo before building the pipeline (CONFIRMED by the
Phase-1 census: MagicaVoxel `.vox` v150/v200, per-file RGBA palettes, RON manifests, reserved palette
indices, componentized creatures, spot/plot prefab pattern for hand-authored structures); (b)
**style-consistency is the real challenge**, not generation — the harness + variation-first strategy are the
two mitigations; (c) this is a **separate content workstream**, not a queue block.

**PRE-GENERATION DESIGN INTENT (decide this BEFORE building each asset — don't just start voxeling).**
Before generating, the session should predetermine the asset's *design intent* so it builds with purpose, not
blindly. Decide and record: **sizing** (how big — and NOT rigidly locked to a 2×2 player footprint; see the
functional-constraint note below), **ornateness** (plain hovel vs. ornate temple), **complexity** (simple vs.
detailed), **importance** (a common house vs. a landmark), and **lore-fit** (does this make sense in the
world — would this culture/biome/era actually have this?). This intent drives the generation and gets logged,
so the asset is a *considered* thing, not a random shape that happened to pass the harnesses.

**Functional constraints are a FLOOR, not a straightjacket — don't over-constrain Claude.** The colonist must
*almost always* be able to fit (interior clearance/door ≥ the collision box — that floor is non-negotiable,
it's the cottage lesson). BUT bigger is fine and often *better*: a grand hall, a temple, a giant's dwelling
*should* have oversized doorways and soaring ceilings **if the style, taxonomy, and lore permit**. So the
function harness enforces a *minimum* (fits a colonist), never a *maximum* — Claude has full latitude to go
bigger/grander where the design intent calls for it. Don't get stuck making everything player-sized; make it
*at least* player-passable, then size it to what the asset should be.

**LORE GENERATION (each asset gets a story — content that can feed the game).** Generate lore for each asset:
a short in-world description of what it is, who made it, its history/purpose, cultural meaning. This is not
decoration — it's *usable content*:
- **Player-facing flavor** — something to READ (inspect a building → its story; examine a weapon → its
  legend). Depth players love (DF's artifact/item descriptions, item "flavor text" everywhere).
- **Can AFFECT the game** — a temple with lore of a war-god could bias the faith it generates; an artifact
  weapon's legend could tie to prestige/history (DF-ARTIFACT, DF-HIST); a cursed item's lore could carry a
  real effect. Lore as a *hook* systems can read, not just text.
- **Coherence check** — lore also *validates* design intent: if you can't write sensible lore for an asset
  ("a dwarven coral temple in the desert"), that's a signal the asset doesn't make sense in the world — lore
  generation doubles as a lore-fit sanity check.
- **Logged + attached** — each asset's lore stored with it (in the catalog/registry entry + a lore field),
  so it travels with the asset and future systems (inspector, chronicle, faith) can surface it.
Staging: start with simple descriptive lore (what/who/why); later, tie lore to systems (faith bias, prestige,
effects) as those systems exist — same READY-vs-NEEDS discipline (descriptive lore = READY now; lore-that-
affects-gameplay = NEEDS the relevant system).

**Pipeline maturity roadmap (the workstream gets better over time — ranked by value):**
1. **Palette-ramp extraction** — pull the real tonal ramps out of reference `.vox` files (wood ramp, stone
   ramp, foliage ramp — clusters of near-hue varying lightness). Generate *using real ramps* → fixes flat
   shading (the #1 miss) at the source. Cheap, high impact.
2. **Parts library** — extract + catalog the real componentized creature parts (heads/torsos/legs/…) into a
   browsable set. Creature generation becomes *assembly from proven pieces*, not imagination — can't drift
   off-style. (Verify recombination actually works — manifests carry per-part offsets, so arbitrary
   head↔body may need offset reconciliation.)
3. **Parameterized generators** — write a *generator with parameters* (house generator: size/roof/material/
   windows), stamp out 50 coherent-but-varied assets. This IS the building-catalog "parameterized templates"
   tier; the mechanism for DF-scale *density*.
4. **Stat-linked creature generation (novel, distinctly yours)** — generate the *model to match generated
   stats* (DF forgotten-beast style): "large venomous six-legged shelled beast" → stats + a matching model
   assembled from parts. Agency archetypes + parts library + generator = procedurally generated creatures
   that *look* like their description.

Honest caveats on the roadmap: these make *making assets* better; none build the *systems* that use them
(don't let shiny asset-gen pull effort from the core build queue — run it in bursts when content is needed).
And verification has a ceiling: the harness checks measurable style; it cannot check *charm/character* — the
eye stays final, and cheap quantity is not the same as soul.

### 3i. Asset ↔ system delegation (keeps content from becoming a swamp of inert assets)
The line between *pure asset-gen* and *asset that needs new code*. Test before generating anything:
**"does the game already have the system that makes this asset DO something?"**
- **`READY`** — an existing system consumes it (new tree → worldgen places it; new sprite → sprite manifest;
  new house prefab → barn/witch-hut pattern; new weapon → weapon system). Zero new code. Generate freely.
- **`NEEDS: <system>`** — inert until code exists (ship → naval movement; poison-attack creature → poison
  system; operable gate → operable-terrain system; workshop → production system). The model is the *easy
  half*; the system is the real work. Don't kid yourself that generating it is feature progress.
**Every asset carries a `READY` / `NEEDS:<system>` tag.** The tagged catalog is the **shared interface
between the content pipeline and the build queue**: finishing a system (e.g. DF-WORKSHOP) *unlocks* a batch
of `NEEDS:production` assets → they flip to `READY` and become worth generating. Operating model: generate
all `READY` freely (parallel, no coordination); don't generate `NEEDS:` ahead of its system — EXCEPT
deliberate placeholders to *pressure-test/spec* a system (generate one gorgeous ship to motivate the naval
design; don't generate 40 and wonder why they don't sail). The build queue completing a system is the
trigger to generate its unlocked batch.

### 3j. Functional validation harness (assets/behavior that provably WORK, not just look right)
The style-harness checks *aesthetics*; this checks *function* — orthogonal. **Proven necessary by the pilot:**
the first generated cottage passed style 9/9 (great palette, chimney, windows) but was built **too small for
a colonist to fit inside** — a colonist is ~2.2 blocks tall and the interior clearance / door height were
below that. Style and function are genuinely independent: it *looked* right and didn't *work*. The scale-ghost
in the viewer *showed* the mismatch but nothing *enforced* it — that gap is what this harness + the baseline
below close.

**TWO parts, both required:**

**(a) The dimensional framework BASELINE — a spec generators build AGAINST, up front (not a test taken
after).** The root cause of the too-small cottage: Claude generated to *aesthetic* scale with no spec for
*functional* scale. Give the generator hard dimensional constraints derived from gameplay, keyed to the
colonist collision box (~2.2 blocks tall, census: 25 vox ≈ 2.22 blocks):
- **Interior clearance ≥ 3 blocks** (2.2 colonist + headroom/margin) for any habitable structure.
- **Door ≥ 2.2 blocks tall × ≥ 1 block wide** — passable by a colonist.
- **Rooms ≥ N×N** to hold colonist + furniture footprint + pathing space.
- Per-structure-type minimums (housing/workshop/tavern each get a functional-dimension spec).
Note the distinction the census DIDN'T give us: **rendering scale (vox per block) ≠ functional scale (how
big a habitable space must be).** The cottage was at correct *voxel* scale (1 vox=1 block) yet *functionally*
too small (too few blocks of headroom). The baseline is derived from gameplay requirements (collision box,
furniture, pathing clearance), NOT art conventions. **Generators consume this baseline BEFORE generating**;
the harness then confirms rather than rejects.

**(b) The validation harness** — **extends the B0 headless harness** (spawn scenario → run headless N ticks →
assert → pass/fail with reason). **Invariant-first (§7): assert properties that must hold, not exact traces.**
- **Habitability / spatial validity (do FIRST — highest value, and the check the cottage failed):**
  **collision-box fit** — can a 2.2-block colonist *enter and stand inside* (door height, interior
  clearance, room size)? **navigability** — can an agent path into and through every space that should be
  reachable? Plus placement validity (no floating/half-buried), collision coherence (door=air, wall=solid),
  reserved-index resolution. An unpathable/uninhabitable building is the most likely + most damaging asset
  failure — this catches ~80% of "generated asset breaks the game," and it's exactly what the pilot cottage
  needed. Concrete failure output: "FAIL habitability: interior clearance 1.8 < 3.0 required; door 1.6 < 2.2."
- **Behavioral validity (harder, fuzzier, stage second):** actors do what they should — colonist reaches +
  works a workshop, wolf hunts/flees, job claimed→reached→executed→completed, plus standing invariants (no
  dupe/loss, counts return to baseline, no orphaned claims). **Test the FLOOR of "not broken" (stuck forever,
  job never completes, threat ignored), NOT the ceiling of "does exactly this"** — over-tight asserts kill the
  emergence that makes the sim interesting. Doubles as the Tier-1b soak's assertions.
- **Integration validity:** generated asset + real world + real colonists, full-stack.
**The `READY` tag requires passing the FUNCTIONAL harness, not just the style harness** — an asset isn't
ready until proven *usable*, not just *pretty*. The pilot cottage is the proof: style-PASS, function-FAIL.

**(c) STATIC vs DYNAMIC — measurement is a proxy; live pathing is the truth.** The checks above are *static*
(measure geometry against thresholds — fast, asset-alone, no sim). But a door can measure 3 blocks tall and
STILL fail if the navmesh won't route through it, the collision box snags on a lip, or there's no valid path
inside. So static is the cheap first gate; **dynamic testing (a real NPC with real collision pathing in the
running sim) is the real gate.** This is the B0/B4 harness pattern (spawn NPC → give target → assert arrival)
pointed at an asset instead of a job.

**Dynamic test requirements (a building/asset must pass these in the game harness):**
1. **Reachability** — an NPC spawned outside can compute a path to a designated interior point (no path =
   the interior isn't navigable).
2. **Traversal with collision** — the NPC actually moves the path and its collision box clears the doorway +
   interior without snagging (path exists but body won't fit = fail).
3. **Arrival** — reaches the target within a tick budget, not stuck/oscillating (ties B4's progress watchdog).
4. **Egress** — can path back OUT (enter-but-not-leave = a trap).
5. **Multi-occupancy** (bigger structures) — multiple NPCs path in/around without permanent deadlock.
6. **Interior function** — reaches the *functional* point (workshop's work-position, housing's sleep-spot),
   not just the doorway.
Generalizes: **creatures** — collision fits the world, paths in intended terrain (fish in water, quadruped
on land), doesn't clip; **props/furniture** — NPCs path *around* it (a table must not block a door) and can
*use* it (reach the chair/bench). All invariant-first ("reaches within N ticks", not "takes this path").

**THE CONTENT-SIDE vs GAME-SIDE SPLIT (important for the isolated tooling):**
- **STATIC checks = content-side.** The asset-tooling session runs these fully in its sandbox (pure geometry,
  no sim needed). Its `function_check.py` does static habitability.
- **DYNAMIC checks = game-side.** They need the real headless harness (B0/B4), which lives in game code the
  asset tooling is isolated from. The asset session *specifies + documents* the dynamic-test requirements
  (writes a `readme/ASSET_DYNAMIC_TEST_SPEC.md`) but the game-side harness *runs* them when the asset is
  integrated.
- **An asset is truly `READY` only after BOTH: static (in the lab) + dynamic (in the game harness on
  integration).** Three tiers of rising cost/confidence: measure it (static) → prove one NPC uses it
  (dynamic) → prove a colony uses it untended (soak integration).

**(d) The FLAT-PLANE TEST ARENA — test in a controlled void, not the messy world.** Dynamic tests should run
first on an empty flat plane, NOT in the real generated world. Why: in the real world a pathing failure is
ambiguous (the building? the slope it's on? the town around it? other NPCs?). A flat plane strips all
confounds — spawn *exactly* what's under test on known-flat ground with nothing else, and a failure means the
*thing itself* is broken, not its environment. Software-testing 101: control your inputs (a unit test runs
against a fixture, not production).
- **Derive the minimal cast from the asset (type/purpose → test scenario).** The harness figures out what to
  spawn: a **house** → house + 1 colonist + an interior target; a **workshop** → + a work-job at the
  workstation; a **predator** → + a prey creature (does it hunt?) + a colonist (does it threaten?); a
  **gate** → + a colonist (passes?) + a hostile (blocked?). Each asset category has a **standard test
  scenario** (required cast + assertions); the asset's metadata determines it.
- **Test-fixture library** — a set of *verified* standard actors (known-good colonist, prey, target block)
  the arena spawns. They must be trusted so a failure implicates the asset-under-test, not the fixture. Real
  infrastructure worth building.
- **The tier order (each catches what the previous can't):** STATIC (geometry, content-side) → ISOLATED-
  DYNAMIC (flat plane — is the thing itself sound?) → INTEGRATED-DYNAMIC (real world — does it survive
  terrain/towns/NPCs?) → SOAK (a colony uses it untended for 30 days).
- **The flat plane's cleanliness is also its blind spot** — a building that works on flat ground can still
  fail on a 15° slope, near water, at a chunk boundary, or amid a real town's NPC traffic. So the flat plane
  proves *internally sound*, NOT *works everywhere* — the **integrated in-world test stays MANDATORY as the
  final gate.** Some things (site-suitability on varied terrain, biome placement, visual context) are
  *meaningless* on a plane and can only be tested in a real world. Flat plane = functional/behavioral truth;
  real world = placement/context/integration truth. Different tiers test different things, not the same thing
  at different fidelity.
- **This is also why "move the player out of the town" (§3n) is right twice over:** the pre-existing town is
  an uncontrolled confound — for *tests* (use the flat plane) AND for *founding* (found fresh at a chosen
  site). Same instinct from two angles.

**Other tests to include (things easy to forget):**
- **Determinism** — a seeded generator reproduces the same asset (already required; verify it).
- **Save/load round-trip** — an asset placed in the world persists correctly across save/load (B10).
- **LOD/render integrity** — the asset meshes without errors at load, renders at distance without gaps
  (ties the §6b/mesh concerns).
- **Placement-on-terrain** — sits correctly on varied terrain (slope, edge) without floating/burying.
- **Reserved-index resolution** — every custom_index byte resolves to a real StructureBlock (no undefined
  markers → no crash on placement).
- **Collision-coherence** — solid where solid, passable where passable (door=air, wall=solid) — the static
  precondition for the dynamic pathing test.
- **Scale-consistency** — creature parts assemble at the right relative scale (no giant head on a small body
  from an offset error).
- **Performance budget** — a big asset's voxel/entity count stays within the tick/memory budget when placed
  (a 600k-voxel room shouldn't tank the tick).

### 3k. Developer debug mode (know what every asset is, and where it came from)
Once Claude generates hundreds of assets you lose track of what's what. Debug mode = **toggleable in-world
overlays surfacing metadata you already store** (visualization layer, not new sim). Ties the legibility
pillar (dev-facing legibility). **Build EARLY — it's dev infrastructure that accelerates everything else**
(debugging B5, the asset pilot, B-AG* is far faster with it). Toggleable *per category* so you see exactly
what you're debugging:
- **Provenance** — vanilla vs. Claude-generated (+ which session/date). Requires a **`source` schema field:
  `vanilla | generated | variation-of:<id>` — ADD THIS TO THE ASSET SCHEMA NOW (free now, expensive to
  backfill).** Look at a building → "GENERATED, dwarven_house_v3, session 2026-07-09."
- **Identity/category labels** — floating id + category over assets/agents; audit "everything is tagged as
  what it actually is" (catch a wolf tagged as prey).
- **Functional debug** — the functional harness made visible: navmesh overlay (can agents path here?),
  collision bounds, reachability. Colonist stuck → flip on navmesh → *see* the building has no valid path.
- **Agent-state debug** — job/goal/path/state (idle/working/fleeing) + mood/thoughts once the mind exists.
  This is the B-AG4 inspector as a global overlay. Debug "why is that colonist idle" by seeing its state.
- **Generation metadata** — strategy (from-scratch/variation), derived-from, style + functional harness
  scores. Asset's whole birth certificate, inspectable in-game.
Shares the overlay-rendering layer with player-facing legibility overlays (faith/mood/needs) — build the
foundation once, feed it dev-data or player-data. Distinct audiences. Keep toggleable-per-category (an
all-at-once debug mode is as useless as none).

### 3l. Animation — how Veloren does it, and what it means for generation
**Veloren uses SKELETAL animation, code-defined, per body family** (confirmed by census: each creature
family has "its own skeleton + animation set"). The componentized parts ARE the bones — animation moves/
rotates rigid part-models relative to each other over time (legs swing, head turns); parts never deform
internally. Animations live in **code** (the anim crate), per family, as named clips (idle/walk/run/attack…).

**What this means (mostly good news):**
- **Generate-to-skeleton = inherit animation FREE (the huge pipeline win).** A new creature built from
  correctly-structured parts matching an existing family's skeleton (right parts, right offsets) *already
  walks/runs/attacks* — the family's animation set drives any conforming parts. You don't animate it; you
  conform it. **Most of a rich bestiary fits the ~15 existing families** (quadrupeds, bipeds, birds, fish,
  arthropods, dragons…) — huge range before needing anything new.
- **New body plan = new skeleton + animation = CODE, not content** (a floating jellyfish, a rolling sphere —
  nothing existing fits). This is the delegation model on animation: new creature *in* a family = `READY`
  (inherits animation); new *body plan* = `NEEDS: skeleton+animation-system`. Prefer generating within
  existing families; treat new-body-plan creatures as rare, deliberate, system-level additions.
- **Structures mostly static** — buildings don't animate. EXCEPT operable parts (gate opens, drawbridge,
  windmill sails, door) — animated via the operable-terrain/trigger→link→effect system (DF-OPERABLE). Static
  building = no animation work; operable gate = animation + the operable system.
- **Sprites get procedural wind-sway** (census: `wind_sway` in sprite manifest) — cheap ambient motion,
  inherited via manifest flag by generated flora/sprites. A separate (cheaper) category from skeletal.
- **Tool-use / combat animation exists as substrate** (Veloren's an action-RPG with combat anims), but
  *wiring* it to colony work — colonist plays a mining swing while working a mine job, a chop while chopping
  (ties B5's staged tree removal) — is specific integration work, not free.
- **The one genuinely hard, doesn't-parallelize piece:** lots of visually-distinct creatures with distinct
  *movement* that don't fit existing skeletons → new-skeleton-per-body-plan, which is code and scales like
  code, not like asset-gen. Mitigation: stay within existing families for breadth; new skeletons are rare
  deliberate investments.

### 3m. The component system + human editor (makes big assets manipulable; unifies 3 goals)
**The mechanism that turns the pipeline from "generates finished blobs" into "generates a manipulable,
reusable, composable component library."** Mirrors the game's own architecture (census: creatures = part-
`.vox` folders + per-family RON manifests with per-part offsets) — adopting the proven parts+manifest pattern.

**Three parts:**
- **Components** — each chunk of a large asset is an *individually persisted, addressable* asset: own `.vox`
  + metadata + id + registry entry (`dwarven_room/floor`, `/pillar`, `/entrance`). Loadable, modifiable,
  verifiable in isolation.
- **Composition manifest** — RON/JSON referencing components + positioning each by offset (assembly
  instructions; mirrors the game's part-manifest). Manipulate a big asset by editing a component OR the
  manifest (move/swap/add) without regenerating the rest.
- **Registry** (`readme/COMPONENT_REGISTRY.md`, append-only) — every component logged (id/parent/type/dims/
  reusable-vs-specific/TEST-REAL/harness-status/path). **This IS the parts library** — later sessions query
  it and compose from existing pieces.

**Unifying insight (build once, three payoffs):** persisted-addressable-components + manifests + registry is
ONE mechanism delivering big-asset chunking (components = chunks), the parts library (persisted reusable
components), AND variation/parameterized templates (swap a component = a variant). Not three systems, one.

**Chunking is size-gated + experimental:** decompose only big/reusable assets (a sword stays one piece);
over-decomposition (50 tiny pieces) is as messy as none — find the right granularity. Seam coherence is the
real risk (components must align at boundaries); the FUNCTION harness must check the *composed whole*, not
just parts, to catch bad seams. Spatial chunking (tile → process → stitch) is a fallback for assets too big
to process in one pass — distinct from logical component decomposition.

**Human GUI editor + change log (human-in-the-loop):**
- `asset-lab/editor.html` — a browser tool: load a component/composition, edit voxels by hand (fix Claude's
  mistakes), save (export + diff). The human fixes what generation gets wrong.
- **Change log** (`readme/HUMAN_EDIT_LOG.md`, append-only) — every human edit recorded. Claude **reads it
  back** to (a) *verify* Ben's edits are valid (re-run harnesses; flag if an edit broke function/style) and
  (b) *learn* — if Ben keeps fixing the same mistake, Claude notices and stops making it. Closes the
  human↔Claude loop.

**Built via a progressive test ladder** (see the component-system implementation prompt): persistence →
isolated modification → composition → recomposition → seam/whole verification → the GUI editor → change-log
read-back → registry reuse → **animation (research-first, the capstone)**. Each rung proven + approved before
the next. Animation research-first (like assets): prove a component-built creature *inherits* a family's
animation by conforming parts to the skeleton (parts transform as bones, never deform); document the READY
(conforming creature inherits motion) vs. NEEDS:animation (novel body plan = new skeleton = code) boundary.

**Asset categorization schema (every asset carries this metadata — the catalog becomes queryable):**
- **id / name** — unique identifier
- **type** — STRUCTURE / CREATURE / ITEM / FLORA / PROP
- **purpose** — what it's FOR: structures → housing / production / defense / social / faith / storage /
  infrastructure; creatures → predator / prey / livestock / mount / monster / townsfolk; items → tool /
  weapon / material / food
- **purpose** — what it's FOR: structures → housing / production / defense / social / faith / commerce /
  storage / agricultural / infrastructure; creatures → predator / prey / livestock / mount / monster /
  townsfolk; items → tool / weapon / material / food. **This structure-purpose enumeration is the SHARED
  ZONE↔ASSET TAXONOMY** — see below.
- **race/culture** — human / dwarf / elf / orc / wild / universal (keys the culture catalogs; wild = none)
- **placement type** (structures) — POINT / AREA / PATH / LINE-ENCLOSE (see catalog spec, §3c)
- **cost** — materials + work-type/time to build (structures); spawn weight (creatures)
- **biome affinity** — temperate / desert / tundra / underground / any
- **archetype link** (creatures) — which Agency Bible flagship drives its behavior (Townsperson / Wolf-
  predator / Deer-herd / Wyvern-apex / Raider)
- **status** — CONCEPT / GENERATED / STYLE-CHECKED / IN-GAME
The schema makes generated content *systematic* instead of a pile of files: B-AG6 queries "dwarven defensive
structures," the spawner queries "tundra predators," and every asset self-describes its place.

**ZONE ↔ ASSET SHARED TAXONOMY (lock the vocabulary NOW; build zoning later).** A zone's *type* and an asset's
*purpose* are drawn from the SAME enumeration — that shared vocabulary is what lets zoning and asset placement
talk to each other (§3q construction modes). Zone types = the structure-purpose list: residential→housing,
industrial→production, commercial→commerce, religious→faith, civic→social, defensive→defense,
storage→storage, agricultural→farming. Because assets carry `purpose` and zones carry a compatible `type`, the
placement system can query "what assets are valid in THIS zone?" and get exactly the right subset — the
classification IS the matching key between spatial zones and buildable assets.
- **Distinguish activity-zones vs building-zones:** *activity zones* (storage/stockpile, farming — where a
  thing happens; already DF-ZONES) vs *building zones* (residential/industrial/religious — what kind of
  structure belongs). Related but not identical.
- **Soft preference, NOT iron law:** autonomous growth *prefers* to build housing in residential zones but
  shouldn't *forbid* out-of-zone building without a gameplay reason. Zoning organizes; it isn't bureaucratic
  friction (DF/RimWorld mostly don't hard-zone). Lighter touch (zones as hints for autonomous placement +
  a player districting tool) over heavy rigid zoning law.
- **What to lock NOW (cheap) vs build LATER:** the *shared vocabulary* (assets + zones reference one purpose
  enumeration) is a design decision to lock in immediately — it costs nothing today and saves a painful
  translation-layer reconciliation later. The zoning *system* itself is a later build (B-AG6 autonomous
  placement + DF-ZONES player districting). **Next asset session: tag asset `purpose` from this
  zone-compatible vocabulary** so the two systems are born matching.

### 3f. Autonomous civilizational advancement (tech as world-history, NOT a management tree)
**Pie-in-the-sky, Tier-3 late — but genuinely on-theme and possible.** The distinction that makes this a
FIT (vs. the "avoid tech trees" rule): this is **NOT player-micromanaged research** (clicking nodes, queuing
tech — that's 4X busywork that pulls you out of god-mode, still AVOID). This is **civilizations advancing on
their own over time**, the way Stellaris empires or DF's world-history do — *the world having technological
history.* You influence it as a god; you never manage a research queue.

**Why it fits the soul of the project:** the whole point is a world that lives and changes autonomously while
you influence, not command. A world whose civilizations *never advance* is statically wrong — real history
has progress. A god watching (and shepherding) stone-age tribes discover metalworking → masonry → advanced
architecture, over generations, without micromanaging it, is the DF world-history dream and the god-game
dream at once.

**It's an EXTENSION of systems already designed, not a foreign bolt-on:**
- **Settlement growth (B-AG6)** already has autonomous world-tier progression — tech is the same idea on a
  different axis: settlements grow not just *bigger* but *more advanced*. Same LOD (abstract when unwatched,
  concrete when loaded).
- **The race-keyed building catalog's ascending tiers** (fixed→parameterized→composed, §3c) become the
  *visible expression* of tech level — **tech level selects which catalog entries a culture can build.** A
  tech-primitive culture builds tier-1 huts; an advanced one builds tier-3 composed structures. The tiers
  you already designed ARE the tech ladder's output.
- **Genealogy + world history + rtsim world-tier** already track things that advance over time; tech is
  another such thing.
- **The asset pipeline (§3e)** is what makes advancement *visible* — new buildings, better tools, new
  equipment to show the rise. Advancement needs content breadth; the pipeline now provides it.

**What makes it YOURS (not just Stellaris):** because you're a god, tech becomes an **axis of divine
influence** — bless a people with insight (accelerate them), gift knowledge as a divine act, let a rival
god's followers languish, watch a dark-age collapse when faith fails. Ties Divine Politics (competing gods
contest not just faith but the *advancement* of their followers) + B13 (favor-costed divine acts). "A god
shepherding the technological rise of civilizations who advance on their own" is the distinctive thing no
other game has.

**Why it's genuinely LATE / hard (the honest caveats):**
- **Depends on a deep world-sim first** — tech only means something atop factions/economies/production/needs
  (B-AG6, Divine Politics, DF-WORKSHOP chains). It's a capstone on the world-tier, not a foundation.
- **Legibility is the real wall** — autonomous advancement must be *readable* ("the Ashfell clan discovered
  masonry" as a legible chronicle event, DF-HIST/DF-LOG), or it's just confusing silent stat-drift. "Why did
  that tribe suddenly build stone walls?" needs an answer the player can see.
- **Balance over long soaks** — must not runaway (everyone space-age in a decade) or stagnate (nobody ever
  advances); bounded by resources, knowledge-spread, stability — same carrying-capacity discipline as
  settlement growth.
- **Tech must DO something** — better buildings (catalog tiers), weapons (combat/raids), production
  (workshops). Tech with no consequences is just a number; it depends on those systems to plug into.

**Verdict:** capture as a real "someday" pillar. Reframes the building tiers as expressions of something
bigger. Build only after the world-sim (growth + agency + politics + production) is deep and proven. Autonomous
advancement = good fit; player-micromanaged research = still AVOID.

### 3g. What else the content-generation unlock makes possible (the constraint that lifted)
The asset pipeline (§3e) lifts a *specific* constraint — **content volume without an artist** — which was a
real ceiling (Veloren stayed modest-scope partly because art is scarce volunteer labor). Naming what this
does and does NOT unlock, so the excitement aims at the right things:

**Now viable (were CONTENT-bound, not system-bound):**
- **Building catalog at real breadth** — 50 building types, not 5; every workshop, every race's full
  architectural set, defensive variants. B-AG6 can finally have the wide catalog it needs.
- **The bestiary (DF-BEAST)** — every predator/prey/livestock/monster as a content pipeline, not a
  hand-modeling marathon. Procedural creature generation (DF forgotten-beast style) becomes plausible: generate
  the *model* to match generated stats.
- **Item/crafted-good breadth (DF-WORKSHOP chains)** — the hundreds of tools/weapons/furniture/trade-goods
  that production chains imply, each needing a model.
- **DF-scale material/object density** — "every rock type, plant, prepared food" granularity.
- **Themed dungeon sets** — full coherent per-culture building sets → visually rich world.
- **Tech-advancement content (§3f)** — the new buildings/tools an advancing civilization needs to *show* its
  rise.

**NOT unlocked by asset generation (different walls — don't be fooled):**
- **Simulation-wall features** — fluid, the mind, structural collapse, autonomous defense. A generated lava
  texture doesn't build the flow solver. Untouched.
- **Design-fit-wall features** — player-micromanaged tech trees, free-building, unit micro. Free assets don't
  make an off-genre feature fit. Untouched.
- **Legibility** — arguably *harder* now: more content = more for the player to parse. Adding breadth faster
  than ways to understand it is a net loss. Watch this.

**The discipline:** the content unlock should make the *world's variety and density* bigger (the right
thing — it's what makes a DF-like feel alive), NOT tempt bolting on genres/features that were never blocked
by art. The four walls (content / simulation / design-fit / legibility) are separate; only the first just
fell.

### 3h. The embodiment spectrum — RPG modes (god-embodied + mortal-RPG capstone)
**Long-term, pie-in-the-sky.** The three control modes (Autonomous / DF / RTS, design-doc §3d) extend into
two RPG lenses, making one axis from *maximally god* to *maximally mortal* — the same living sim experienced
at five distances of embodiment. Two distinct modes here, sharing the word "avatar" but built on different
assumptions:

**Mode A — God-embodied avatar (RPG, still god-mode).** You're still the god; embodiment is a *state* you
enter and leave (descend into a body, walk the world, ascend back). Reuses Veloren's **native action-RPG
loop** (movement/combat/interaction already exist — this is the game's actual core) but with ALL colony/
agent/god systems live around you. **Extends B12 (Embody/possession)** + God/Free machinery — you already
designed possession; this is "and now play as them, RPG-style, with divine powers still available." *An
extension of existing design, incremental.*
- **New verb: divine quest-giving.** A god descends and charges a hero with a task that ISN'T a colonist's
  autonomous job — "slay the beast in the northern cave," "found a shrine at the falls." Needs a real quest
  system: objective, tracking, reward, and — the on-theme part — a colonist who *accepts* the divine charge
  and pursues it autonomously via the world-verb library + their mind (a devout colonist takes it eagerly; a
  faithless one might refuse). Genuinely cool, genuinely a design pass.

**Mode B — Mortal RPG, no player-god (the capstone).** There is NO god that is you. Rival/agent gods stay
fully live (Divine Politics running), colonies live/grow/war autonomously — and you are a *normal mortal* in
it. A Veloren adventurer in a world that's actually alive (towns that really grew, people with real minds,
gods above contesting faith), experienced from the ground. *A different game sharing the engine + world.*
- **Why it's the ultimate payoff of the autonomy investment:** a ground-level RPG in a *dead* world is just
  Veloren; in *this* world it's unprecedented. Everything built for autonomy is what makes it work.
- **Why it's correctly LAST (the honest reason):** first-person, sustained, close-up scrutiny is *merciless*
  — standing in a town talking to an NPC, that NPC's mind, dialogue, daily life, and reactions must hold up
  at point-blank range. It's the **final exam for B-AG3 (minds) + the dialogue system.** Not hard to *wire*
  (Veloron's RPG loop exists); only *good* once the world underneath survives being seen from the inside.
- **Divine systems for a mortal:** your god-powers/favor just turn off (you're mortal). But the *rival gods
  stay on* — so you experience their divine acts as a mortal on the receiving end (a storm you didn't call, a
  blessing on someone else's village). Atmospheric gold, free once the gods are autonomous.

**Shared caveats:**
- **Clean handoff (again).** Entering/leaving an avatar (Mode A) is the same single-driver, clean-handoff
  problem as possession — the embodied entity was an autonomous agent; it must yield to you and resume
  cleanly. B12 scopes this.
- **Marketability note:** Mode B ("an RPG in a truly living simulated world") may be the single most
  *marketable* thing the project can produce — worth remembering when prioritizing far-future work.

**Mount-&-Blade-style layer (natural long-term elaboration).** Once embodied modes exist, a M&B-style layer
is a natural extension: be embodied *and* lead a warband / rise through a faction / command troops while
personally in the field — bridging RTS-command and mortal-RPG. Different features can hang off the embodied
modes over time (recruitment, party management, faction reputation, personal holdings). Pure pie-in-the-sky,
but the embodiment spectrum is the substrate that makes it possible.

**Verdict:** capture as long-term game modes. Mode A extends B12 (nearer-term of the two). Mode B is the
capstone after minds + dialogue are deep. Both reframe the project as *a living world with many lenses*, not
a single game. Build order: everything else first; these are what you build when the world is proven and you
want to inhabit it more intimately.

---

## 4. Open watch-items from build sessions (track, don't lose)

### 3x. Selection-on-terrain, construction site-prep, and road building (the ground-truth cluster)
Three related items, all about construction meeting real topography (surfaced by the first live demo):

**(a) Selection/zone outline must FOLLOW TOPOGRAPHY.** The painted-zone outline currently reads as a flat
rectangle floating over 3D terrain (visible in the live demo screenshot). Fix: the boundary line drapes over
the terrain surface — sample terrain height along the outline and render the line conformed to it (same for
zone fill overlays). Small client-side rendering fix, big legibility win: you can't judge what a zone covers
on a hillside if the outline ignores the hill. Also applies to the §3w colony-boundary overlay (same draping
renderer — build once).

**(b) Construction SITE PREPARATION — buildings need flat ground, and colonists make it.** Real terrain is
rarely flat; the building system needs an explicit site-prep phase:
- **Plan:** given a placement footprint on uneven ground, compute the prep: which blocks to CUT (dig high
  spots — reuses B5 mining verbatim) and which to FILL (place platform/foundation blocks — reuses B5 build).
  Cut-vs-fill balance can even feed itself (spoil from the cut fills the low side).
- **Execute:** site-prep jobs run before construction jobs — colonists visibly level the pad, THEN build.
  Terraced/platform foundations for steep sites (the barn-on-a-slope answer: build a foundation platform up
  to grade, build on top — foundations are part of the structure template's understory).
- **Ties:** site-suitability scoring (§3c/§3n) should *prefer* low-prep sites (flatness = less work), making
  prep cost a real siting economics; the function harness's placement-on-terrain check (§3j) validates the
  prepped result; autonomous building (B-AG6) uses the same plan→prep→build pipeline unprompted.
- This is the missing middle of the construction pipeline: **place → PREP → build.** Slot the design with
  B7-era construction work; the primitive verbs (dig/place) already exist from B5.

**(c) ROADS — colonists build them, autonomously or by player direction (the §3s infrastructure layer's
first rung, and the §3q spectrum again):**
- **Direct:** player paints a road PATH designation (the PATH placement class from the catalog spec §3c) —
  colonists clear, level (mini site-prep along the line), and surface it (path blocks; Veloren worldgen
  already has road/path block types — reuse the material).
- **Autonomous:** the colony builds roads where traffic actually flows — track colonist movement density
  (desire lines!), and when a route between two well-used points crosses a threshold, generate a road-build
  job along it. Roads *emerge from use* — deeply on-theme (the world organizes itself; you can watch paths
  harden into roads). Also: connect new buildings to the existing road net as part of B-AG6 placement.
- **Function (per §3s):** roads speed movement along them (movement-cost modifier), channel pathing
  (colonists prefer roads), and later carry trade/armies. A road with no speed effect is decoration — the
  movement bonus ships WITH the first road, not later.
- **Inter-settlement roads** (connecting colonies/towns) remain §3s tier — this section is the *intra-colony*
  road loop, which is B6/B7-era buildable (designate → prep → surface → movement bonus).

### 3w. Colony boundary — where the colony ends (organic AND player-defined, one mechanism)
**The question several systems are already asking:** job/hauling range (B6), defense perimeter (B8),
autonomous-building bounds (B-AG6), zone-painting bounds, and — later — what land is *yours* against rival
factions and gods. The colony boundary is the **first customer of the territory layer (§3s #1)**: build the
boundary now as an influence field, and territory tracking later is the same field with more factions.

**One mechanism, three modes (the control spectrum §3q again):**
- **The substrate — an influence/claim FIELD, not a drawn line.** A scalar field radiating from sources
  (buildings, zones, activity, population) with distance falloff; the boundary is a threshold contour. Grows
  organically as the colony builds outward; shrinks if structures are lost. Cheap to compute, naturally
  irregular (follows what the colony actually IS, not a circle), and — critically — the same representation
  Dominions-style **dominion spread** uses (§3t): temples/faith can modulate the field later, and rival
  settlements' fields can overlap = contested ground (DF's overlapping claims, emergent).
- **Autonomous mode:** the field IS the boundary — colonist-defined by what they build and use. Zero player
  input; the default.
- **Manage mode:** player adjusts the organic boundary — extend a claim toward the river, exclude the
  haunted hill. Implemented as player-painted sources/masks added to the same field.
- **Direct mode:** player draws the border outright (paint/drag), the field conforms. Very god-game: "this
  land is my people's."

**Diegetic markers (asset tie-in):** as the boundary settles, colonists erect **boundary stones / banners /
waystones** at the contour — physical, in-world border markers (PROP/civic purpose in the asset taxonomy,
generatable now, race-styled). The border isn't just an overlay; it's *visible in the world*, and rival
agents can read (and topple) it.

**What the boundary DOES (function discipline — each is a consumer):**
- **Work/haul range:** jobs inside (or within reach of) the boundary are valid; beyond needs an expedition
  framing. Gives B6 hauling a natural range answer.
- **Defense perimeter:** B8 patrols the contour; threat-response triggers on incursion; walls (LINE-ENCLOSE)
  naturally want to trace it — suggest wall paths along the boundary.
- **Growth bounds:** B-AG6 autonomous building places within the claim (expanding it as it builds — the
  field grows with the buildings, a natural feedback).
- **Claim vs the world:** rtsim factions/wildlife/rival gods later read it — trespass, contest, tribute,
  dominion. The god's "my people's land" becomes a real spatial object divine acts can reference.
**Legibility:** a border overlay (toggle) + the markers. **LOD:** the field is cheap (recompute lazily on
build/destroy events, not per-tick). **Timing:** the field + overlay + work-range consumer are B6-era
useful; markers and defense-consumer land with B8; faction contest is §3s-era.

### 3v. 3D zones + the mining framework (volumetric designations, traditional mines, cave prospecting)
**The problem:** zones so far are implicitly 2D (paint an area). A mine is inherently VOLUMETRIC — footprint ×
z-levels. And "mining" isn't just "remove this box": real colony mining is a *structured excavation* plus
*exploration of what's already down there*. B5's mine-pit trap bug (colonist stuck in a hollowed pit —
fixed with a test staircase) is the proof this needs a real framework: **access is part of the dig, not an
afterthought.**

**(a) 3D zone definition — the zone schema gains a z-extent.** Every zone type declares its vertical shape:
- **Thin zones (surface + few z):** farming, storage/stockpile, social/market — footprint + 1–3 levels.
- **Tall zones (ground + height):** building zones (residential/industrial) — footprint + build-height
  allowance; defensive zones include wall height.
- **Deep zones (footprint × z-range DOWN):** the mine zone — painted area + "dig N levels" (or to a target
  depth/stratum). DF does designations per-z-level; Bastion has true 3D voxel terrain, so a mine zone is a
  volume: paint the footprint, set the depth, the system stages it level-by-level.
UI: paint area → set z-extent (slider/drag). The zone↔asset taxonomy (§3e) is unchanged — zones just gain
`z_extent` in the schema. **Lock the schema field now** (cheap), like the taxonomy itself.

**(b) The mining framework — two modes, one system:**

**Mode 1 — Constructed mines (dig a traditional mine).** A mine is a *building dug in negative space* — and
that means it can ride the building-catalog machinery (§3c): a **parameterized mine TEMPLATE** (entry
adit/shaft → main gallery → branch tunnels → per-level staircase/ramp access → support pattern → stockpile
near the mouth), parameterized by depth, branch count, and target stratum. Colonists dig it *progressively*
(B5's work-tick, staged like tree removal), hauling spoil/ore out (B6). **Access modeling is mandatory:**
every dig plan includes its own ramps/stairs so no colonist is ever trapped below (the B5 bug, solved at the
framework level, not per-test). Ties the vertical-reachability backlog item directly — this framework is its
biggest customer.
Player involvement per the control spectrum (§3q): **paint-your-own** (DF mode — designate exact cells per
level), **zone it** ("mine zone here, 8 levels down" — colony plans the structured mine inside it), or
**fully autonomous** (colony needs ore → surveys for a suitable site → plans and digs a mine unprompted).
Autonomous mining needs an **ore survey** capability (sample the terrain for mineral density — reuses the
site-suitability sampler pattern, pointed down).

**Mode 2 — Prospecting & spelunking (explore what's already down there).** The world already HAS caves and
dungeons (procedural worldgen). The framework's second half is colonists *finding and exploiting* them:
- **Prospect/scout** — a scout job: discover cave entrances and surface ore signs; ties the agent
  knowledge/memory model (the colony *learns* its map — discovered ≠ omniscient).
- **Assess** — a discovered cave gets evaluated: ore-rich? inhabited? dangerous? (Cave decor sprites =
  visible ore veins; census confirmed cave-decor is the asset half of caves.)
- **Exploit** — mining exposed veins in natural caves is *cheaper* than digging (no excavation), but caves
  have residents — risk/reward.
- **Clear/delve** — dungeons and inhabited caverns become military/adventure objectives (ties B8 defense,
  raids-in-reverse, and the mortal-RPG mode — the dungeons exist to be delved either by colonists or by YOU).
- **THE BREACH EVENT (the DF-sacred moment):** digging a constructed mine can *break into* a natural
  cavern/dungeon — sudden connection, threats can flow OUT into your mine. "Dig too deep" is core to the
  genre's soul and it's nearly free here: caves already exist in the terrain; the breach is just detection
  (dig reveals adjacent void) + a hazard-event (the trigger→link→effect engine, §1) + threat pathing through
  the new opening. Capture as a flagship emergent moment.
**Safety/deferred ties:** digging into water/lava — currently inert (no fluid flow, §3a), note the
interaction lands when DF-FLUID does; cave-ins/structural collapse remain deferred (DF-STRUCT); both slot
into the hazard-events engine when built.

**Dependency order:** zone z-extent schema (now, free) → constructed-mine template + access rule (B6-era —
hauling makes mines real) → ore survey + autonomous mining (with B-AG6) → prospect/assess (needs agent
knowledge model) → breach events (needs hazard engine) → delve/clear (B8+).

### 3u. Action animations — the framework deep-dive (mining, chopping, and the native-vs-custom line)
§3l covered *creature* animation (skeletons, inherit-by-conforming). This covers **action animations** — what
a character visibly DOES while performing a verb (mining, chopping, building, hauling) — which is the part
that bites as early as B5 (colonists mine, but what do they *look* like doing it?).

**How Veloren's action-animation framework actually works (source-confirmed):**
- Animations are **procedural Rust code, not keyframe files.** Each animation is a struct implementing the
  `Animation` trait with an <cite index="45-1">`update_skeleton_inner(skeleton, dependencies, anim_time, rate, skeleton_attr)` function that computes the next skeleton pose, taking dependencies like hands, tool kind, stage section, and ability info</cite> — bone positions/orientations computed
  per-tick by math (sine waves, eased curves), one Rust file per animation (idle.rs, run.rs, swim.rs,
  chargeswing.rs, …).
- **Selection is state-driven:** the entity's `CharacterState` (idle / running / wielding / attacking /
  swimming…) determines which Animation impl runs. Change the state, the animation follows automatically.
- **Animations are parameterized, not per-item:** the same swing animation adapts by <cite index="45-1">`ToolKind` and hands</cite> and per-species `SkeletonAttr` — one melee-swing animation serves sword, axe, and pickaxe, adjusted
  by the tool held and the body's attributes. Attack anims are staged (`StageSection`: buildup/swing/recover).

**What this means — the NATIVE path for colonist work is mostly free:** Veloren *already* mines ore sprites
with pickaxes — a swing animation with a pickaxe equipped IS a mining animation. So for a working colonist:
**equip the right tool + drive the right `CharacterState` while the job executes**, and the visuals come from
existing animations:
- **Mine** → pickaxe equipped + wield/swing state → NATIVE (exists today).
- **Chop** → axe equipped + swing state → NATIVE (combat axe swing reads as chopping; ties B5 staged tree
  removal — one swing per work-tick).
- **Fight/hunt** → the entire combat animation set → NATIVE.
- **Walk/run/haul(simple)** → locomotion set → NATIVE (carrying visible goods on the back = custom polish).
**The actual integration task (small but real, flag for B5-polish/B6):** the colonist job executor must SET
the CharacterState + equipped tool while working (job says "mining" → state says "wielding pickaxe, swinging
at target block"). Today the harness proves work completes; the state-wiring is what makes it *look* like
work. This is wiring, not animation authoring — cheap, high-payoff, the "colony looks alive" moment.

**The CUSTOM path — verbs with no existing state need new animation CODE (one Rust file each, sometimes a
new CharacterState too):** hammering at an anvil/forge (crafting), farming (hoeing/sowing/harvest gestures),
building placement (hammering at a blueprint), fishing, social gestures (conversation, prayer/worship — the
faith layer wants this), sleeping-in-bed poses, operating mechanisms. Each is a per-verb line-item:
procedural bone math in the anim crate + a state to trigger it. Not hard individually (the pattern is
well-established, ~one file each), but it's CODE and it accumulates.

**THE RULE (extends the delegation model §3i):** every new work VERB added to the game carries an explicit
animation line-item in its design: **NATIVE (state+tool reuse — prefer this, bend the verb toward an existing
animation if possible) or NEEDS:animation-code (a named new Animation impl).** A verb with no animation answer
ships as a T-posing colonist — the legibility pillar fails at the most visceral level. Add the animation
column to the work-verb library and the asset catalog's READY/NEEDS tagging.

**Priority order for custom work animations (when they're built):** craft-at-station (workshops are core) →
farm gestures (B-AG/food loop) → build-hammering (construction is watched constantly) → worship/prayer (the
faith layer's visibility) → the rest. Sprites' wind_sway and operable-part motion (gates) remain separate,
cheaper categories (§3l).

### 3s. World connective tissue — roads, bridges, sea lanes, territory, nations (the inter-settlement layer)
What turns a *colony* sim into a *world* sim: the systems BETWEEN settlements. Tier-2/3 — rides on B-AG6
growth + autonomous building + Divine Politics; don't let its coherence-as-vision make it feel buildable now.
**Dependency order (build bottom-up):**
1. **Territory / region tracking (the substrate — who controls what).** A spatial claim system: map regions
   tagged with controlling settlement/faction/nation, borders, and change over time (expansion, conquest,
   collapse). Diplomacy/war/trade ALL reference it — no border disputes without borders. Likely an
   *extension of rtsim's existing sites/factions* (verify), mostly data+sim, could come earliest.
2. **Roads & bridges (infrastructure + chokepoints).** Veloren already generates paths between towns; the
   additions are (a) **autonomous road-building** — settlements grow new roads as they expand/connect (§3c
   applied *between* settlements), and (b) **roads must DO something**: faster travel/trade along them,
   they *channel* movement (armies march on roads, traders follow them), control-the-road = control the
   route. **Bridges = the chokepoints** — cross obstacles, strategically vital (hold the bridge), buildable
   only where geography allows, destroyable/defendable. Ties vertical-reachability work + war.
3. **Trade / diplomacy / war made SPATIAL.** This is what gives the Divine Politics Bible its geography:
   trade flows along physical, interdictable, improvable routes (build a road = enable trade; cut it =
   strangle it); diplomacy is over borders; war is territorial (armies march roads, fight chokepoints,
   besiege, conquest transfers territory in the claim map).
4. **Sea lanes** — the naval equivalent; gated behind naval movement (ships are asset-viable; *sailing* is
   an unbuilt sim). Later.
5. **Daughter settlements** — a thriving settlement autonomously founds a new one (ties §3n embark, but
   agent-driven), expanding territory. The world *grows*.
**Constraints (first-class, not afterthoughts):** LEGIBILITY — shifting territory/trade/war is
information-dense; the *map with overlays IS the interface* (borders, trade flows, control) or it's churning
noise. LOD — all of this is world-tier abstract sim most of the time (territory shifts, wars resolve
unwatched, concrete when you zoom in) and must be cheap or a world of nations melts the tick. FUNCTION
discipline — every element must *do* something (roads speed+channel, bridges gate+defend, territory enables
claims/taxes/conscription); decorative elements get cut. GOD LAYER — bless a trade route, sunder a bridge
with wrath, guide expansion; competing gods contest territory *through their followers* — that's what makes
it yours and not generic grand-strategy.

### 3t. Deep-research: world-sim frameworks to steal from (mega-search findings + catalog)
Researched across DF, CK3, Songs of Syx, Total War, Caves of Qud, RimWorld + known frameworks. The ones that
map onto Bastion, with what each contributes:

**Territory & politics:**
- **CK3 — de jure vs de facto territory** (nominal/legal claims vs actual control): a two-layer claim map
  makes irredentism, "rightful lands," and legitimacy-driven wars emergent. **Casus belli — wars need a
  JUSTIFICATION** (claim, grievance, holy war): <cite index="13-1">wars require legitimate justification — claims, holy wars targeting other faiths — to declare legitimately</cite>. For Bastion: wars become LEGIBLE ("why did that war
  start?" always has an answer — a claim, an insult, a faith conflict) — solves the legibility constraint of
  §3s for free, and faith-based casus belli ties Divine Politics directly.
- **CK3 — universal opinion system:** <cite index="20-1">every character has an opinion of every other character within diplomatic range, a simple sum of modifiers</cite> — cheap, legible, drives alliances/betrayals. The Agency Bible's
  relationship model at the *leader/diplomacy* tier.
- **DF — overlapping claims + political map over time:** <cite index="11-1">multiple civilizations can lay claim to the same area, territory markers overlap, and you can watch territories change by stepping 10 or 100 years through time</cite> — contested (not exclusive) claims + a time-scrubbable
  political map = the chronicle made spatial.
- **Dominions (pretender gods) — dominion spread:** faith as a *territorial field* radiating from
  temples/prophets, smothering rival gods' power where it spreads. This IS the competing-gods territorial
  mechanic — divine territory as spreading influence, not drawn borders. Extremely on-theme; steal it.

**Economy & trade:**
- **Distant Worlds — the private economy:** the civilian sector (mining, freight, trade) runs FULLY
  autonomously; the player controls only the state layer. **The purest influence-not-command economy model
  in games** — the colony/world economy should run itself like this, with god/ruler nudges, never
  spreadsheet management.
- **X4 / Elite's background sim — physical + abstract economies:** X4: every good is really manufactured and
  really shipped on real ships along real lanes (interdictable, watchable — matches "trade flows on physical
  roads"); Elite's BGS: per-region faction *states* (boom/bust/famine/war) driven by events — a cheap
  abstract world-tier layer. Use BOTH per LOD: abstract states unwatched, physical caravans when watched.
- **Victoria — pops:** population as aggregated groups (culture/faith/profession/needs) driving politics and
  economy at the world tier — the population LOD model for unloaded settlements.
- **Songs of Syx — local prices + requirement-based annexation:** per-faction local prices (trade has
  geography); <cite index="29-1">neutral "havens" join a faction when it controls the area AND meets that haven's specific needs</cite> — soft annexation by *meeting needs* rather than conquest: very god-game, adopt it for
  daughter/neighbor settlements.
- **Songs of Syx — knowledge regression:** tech/knowledge must be *maintained* or it slips away — the
  anti-runaway mechanism §3f's autonomous advancement needs (civs can regress; dark ages are real).

**War & logistics:**
- **Total War — supply & attrition:** <cite index="31-1">roads, ports, and supply lines affect movement and income; logistics enable sustained warfare</cite>; armies off-road/unsupplied take attrition — this is what makes ROADS
  matter militarily (§3s's "roads channel armies" mechanized). Also **administration scaling costs**
  (anti-blob: empires get inefficient as they grow) — a carrying-capacity analogue for nations.
- **Sieges as escalation** (Total War): siege = a *process* (encircle → attrit → assault), not an instant
  battle — fits colony defense (B8) and the operable-gate/wall systems.

**History & narrative:**
- **DF — history CONTINUES during play:** <cite index="3-1">civilizations rise, wage war, and fall during world generation, and these activities continue even after world generation as you play</cite> — the world-tier sim doesn't stop at embark. Bastion's rtsim
  already points this way; commit to it.
- **DF — ages:** <cite index="11-1">calendrical ages are named for the greatest powers extant in the world, advancing as megabeasts die and sometimes regressing when new ones are born</cite> — world-state named epochs ("Age of the Wolf-God") = free
  mythic framing for the chronicle, driven by real sim state.
- **Caves of Qud — procedural history with UNRELIABLE accounts:** <cite index="40-1">conflicting accounts can exist across sources — lending an aura of historical mystery and authenticity</cite> — the chronicle doesn't have to be
  omniscient: different cultures record the same war differently; lore items carry *versions* of history.
  Gorgeous, cheap, ties the lore-generation layer (§3e).
- **Talk of the Town (research prototype) — beliefs about people:** NPCs hold (possibly wrong, decaying)
  beliefs about other NPCs — already aligned with the Agency Bible's memory-drift; extend it to *history*
  (what a settlement believes happened).
- **Nemesis system (Shadow of Mordor) — procedural personal rivals** that remember encounters and rise/fall
  in their hierarchy: apply at the world tier (a raider captain who remembers being driven off by your
  colony and returns scarred, promoted, vengeful). Personalizes the RimWorld-director/rival-god pressure.

**Meta-lesson from the research:** every successful world-sim splits **abstract world-tier state** (claims,
faction states, pops, opinion sums — cheap, always-on) from **concrete local simulation** (real caravans,
real armies, real NPCs — expensive, only where watched), with events flowing both ways. That's exactly
Bastion's loaded↔simulated architecture — the frameworks above are all, at bottom, *content for the two
tiers you already have*. Nothing found contradicts the architecture; everything found slots into it.

### 3r. What the custom-creature capability unlocks (husbandry / large creatures / the god-companion)
**Conditional on the novel-creature test actually passing.** Everything here assumes Claude can author a
genuinely NEW body plan (new skeleton + new animation code, not a recolor of an existing family — the
recolor/variation path is proven; the novel path is not). Hold this loosely until the test passes; it's "if
that works, here's what opens," not a plan. But mapping it tells you what the test is really *for*: it's the
gate to a whole content AXIS, not just one weird animal.

**Which-wall breakdown of the three things this seems to unlock:**

- **Animal husbandry — closer than it looks, and mostly NOT a creature-generation problem.** Veloren already
  HAS livestock (census: sheep, pig, cattle, horse, rabbit — real animated quadrupeds). So husbandry doesn't
  need custom creatures; it needs the *systems* around existing animals: breeding (two → offspring), taming/
  domestication, penning (ties activity-zones §3e/§3q), feeding, herding as a colonist job, and products
  (wool/milk/meat/mounts). **Simulation-wall, not content-wall — startable NOW on existing animals**, doesn't
  depend on the hard test. Custom creatures make it *richer* (new domesticable species) but aren't the
  blocker. A DF/RimWorld staple; fits the colony sim; ties zoning; gives colonists autonomous work. *Flag as
  more viable than first assumed.*
- **Large / unique creatures — the DIRECT payoff of the test.** biped_large (troll/ogre/gigas/cyclops) +
  dragons/wyverns already exist as families; what the custom capability adds is *novel* large creatures with
  new body plans — the DF forgotten-beast dream, unique bosses, apex threats, world-bosses, the titan a god
  sends against a rival's people. **Content-wall, directly unlocked if the test passes.** The cleanest, most
  optimistic application.
- **The B&W god-companion creature — animation is ~10% of it; be honest.** The most exciting example is the
  one custom-creatures LEAST delivers. What made the Black & White creature magical was *learning* (watched
  you, imitated, learned reward/punishment — a learning-AI system tied to the Agency Bible mind), a *bonded
  relationship* to the god specifically (god-anchor + mind + persistent individual-relationship model), *scale
  + presence* (a persistent growing individual with personality — mind + persistence + individual LOD), and
  *expression* (emoted its state — the legibility layer). The model animating is the puppet; the soul is the
  whole rest of the project. **So the custom capability gives you the god-companion's BODY — a unique iconic
  animated form — but the creature ITSELF is an agency capstone**, a long-term *convergence* of mind (B-AG3) +
  god-relationship (anchor) + expression/legibility (pillar) + custom body (this test). When all four exist,
  the B&W creature becomes possible — genuinely distinctive, and a beautiful convergence point the whole
  project already builds toward. But it's a convergence, NOT an asset unlock.

**The through-line:** the custom-creature test is the gate to a content *axis* (unique creatures, bosses,
husbandry species, the god-companion's body) — which is why it's the right test and why its success matters
beyond one asset. But keep the excitement honestly aimed: husbandry needs husbandry *systems* (startable now
on existing animals), large/unique creatures are the pure content payoff, and the god-companion's magic is a
*convergence* of systems the animation alone doesn't provide.

### 3q. The control spectrum generalizes across ALL colony domains (construction, governance, military, economy)
The Autonomous / DF-manage / RTS-command spectrum (design-doc §3d) isn't just for *work assignment* — it's a
general pattern that applies to **every domain of colony life**. Each domain is playable at three involvement
levels on the SAME autonomous system: hands-off (autonomous) / policy-level (manage) / hands-on (direct). The
player picks involvement **per domain** — e.g. let construction run automatic, take direct control of military
during a raid, set economic policy while social life self-organizes.

**Construction — the worked example (three modes = the spectrum applied to building):**
1. **Automatic** — the colony develops itself: decides what it needs (housing/production/defense), finds/forms
   appropriate areas, builds from the catalog autonomously. Pure influence. (The god/autonomous mode.)
2. **Zoning** — the player sets *policy*: designate "residential / industrial / religious / commercial /
   civic / defensive / storage / agricultural" areas; the colony autonomously builds *appropriate* structures
   within them. You shape *where categories go* without picking individual buildings. (The DF-manage mode.)
   This is the layer that most needs the zone↔asset shared taxonomy (§3e-schema).
3. **Direct asset selection** — the player picks the specific building and places it ("put THIS workshop
   HERE"). Granular. (The RTS-command mode — reuses B2b/B8 direct-control machinery.)
All three ride the SAME machinery — one catalog, one placement/suitability system, one construction execution
(colonists actually build it). Not three construction systems; one, with three levels of player input. The
zone↔asset taxonomy (§3e) is the shared layer: automatic queries the catalog by purpose + suitability; zoning
constrains that query by zone type; direct places manually from the same catalog with the same validity rules.

**Governance / "ruler stuff" — the spectrum applied to leadership.** A distinct, broader domain: not just
building placement but *policy* (labor allocation, military orders, trade, justice, social structure). Key
design distinction — **the god ≠ the ruler:**
- **The god** (you, from above) — influence via faith/divine acts/blessing-wrath. You don't *rule*, you
  *influence*.
- **The ruler** (a mortal leader *within* the colony) — the chief/king/elder who makes governance decisions.
The spectrum resolves "does the player rule, or influence a ruler?" as **both, by mode:**
- **Autonomous** — the colony has its own AI ruler (an rtsim leader NPC) making governance decisions; you
  influence them as a god (a devout ruler heeds your will; a faithless one strays). Very B&W — shape the
  leader, the leader rules.
- **Manage** — you make *some* ruler decisions directly (policy, priorities); the AI handles the rest.
- **Command / embodied** — you *are* the ruler (via the god-embodied avatar §3h, or by taking the reins),
  deciding directly. Ties the embodiment spectrum: the embodied avatar could literally *be* the ruler.

**THE STANDING GUARDRAIL (critical — this is a lot of surface area):** autonomous is the SOUL and the
DEFAULT. The colony rules itself, builds itself, defends itself; the manage/direct layers are **optional depth
the player leans into by choice, never mandatory operation.** RimWorld/DF let you micromanage but their soul is
the autonomous story. If "ruler stuff" becomes a governance UI you MUST operate, the game drifts into Crusader
Kings / 4X management — exactly what's on the AVOID list. The line: **autonomous by default, ruler-control
available by choice.** Every domain-control feature is tested against this — is it enrichment over a
self-running world, or mandatory management? Only the former ships.

### 3n. Colony founding / embark flow (the real "start a new colony" — currently missing)
**A genuine gap.** B3 founds a colony by promoting existing rtsim townspeople in an *existing* town (the
"Found colony" radial verb). That was the RIGHT first step — it proved colonists-as-rtsim-NPCs + the promote/
demote boundary using the easiest spawn (reuse existing NPCs). **But it's a PLACEHOLDER for the real flow:**
establishing a *fresh* settlement at a site the player *chooses*, DF-embark style — the actual colony-sim/
god-game fantasy (pick a spot, start from nothing, watch it grow from a few founders).

**Why it matters (not just thematic):** founding a fresh colony is what makes carrying-capacity, site-
suitability, and settlement-growth meaningful *from the player's side* — the vulnerable early days, the site
choice mattering, the growth-from-nothing arc (DF's first year, RimWorld's landing) that's core to colony-sim
appeal. Always spawning into an established town skips the founding entirely.

**The real flow (belongs to B11 embark/scenario — reuses pieces already being built):**
1. **World gen / selection** — generate or pick a world, then survey it (B11).
2. **Site selection** — *the same site-suitability scoring* the autonomous-settlement system uses (flatness
   via B1.8 sampler, resources, terrain, biome, culture-fit) — but **surfaced to the PLAYER**: show
   suitability while choosing ("flat, near water, near ore — good site"). Autonomous-placement and player-
   founding are the SAME scoring, one used by AI, one shown to you. Build once.
3. **The founding band** — spawn B3's starting colonists at the *player-chosen empty site*, not by promoting
   town NPCs. A modification of B3's spawn logic: spawn fresh founders at a selected location.
4. **Starting conditions** — founders' starting resources/tools (scenario config, B11).
5. **The selection screen** — the embark UI tying it together: new game → world → survey/pick site (with
   suitability shown) → confirm founders → drop in. Reuses B1.8 map/fly-to for surveying.

**Ties the embodiment spectrum:** god mode = *choose where your people settle* (very Populous — raise land,
guide them here); mortal-RPG mode = you might *arrive* at a young colony rather than found it. **B3's
found-in-existing-town stays as the proven placeholder** until B11 builds the real embark. Flag for B11's
design pass: it's an integration of site-suitability (surfaced) + B3 spawning (at chosen site) + B1.8 survey
camera + the embark UI — mostly wiring existing pieces, plus the screen.

### 3o. The headless harness — what it actually is (custom-on-intended, load-bearing infra)
Worth recording so it's understood + maintained. **Veloren's server can run standalone/headless** — that's
*intended* (it's how multiplayer servers host without graphics). What Bastion **built custom (B0)** is the
**test harness that drives it**: the scenario-runner that spawns N colonists, force-loads chunks, injects
designations, ticks deterministically, dumps rosters, and **asserts invariants** (no dupe/loss, counts return
to baseline, bounded tick/memory, no panics). That orchestration-and-assertion layer is a Bastion invention
*on top of* an intended seam (the standalone server) — the project's whole pattern: reuse substrate, add the
layer that makes it do what you need.
- **It's arguably the most load-bearing thing built** — the entire checkpoint + soak + invariant discipline
  rests on it. Without it, every test needs the full graphical game watched by hand (slow, unrepeatable). It's
  what lets B4 prove "20 colonists, distinct claims, all arrive" in seconds, and it's where the **dynamic
  asset tests** (§3j) run.
- **Maintenance caveat:** because it drives the server in ways normal play doesn't, it can depend on server
  internals that shift as systems change (esp. with a concurrent agent modifying them). **Keep its
  assumptions documented** so it doesn't silently rot.

### 3p. Claude visually checking results (exploratory — not now, but worth mapping)
The gap: Claude generates/verifies assets numerically + geometrically, but "does it actually LOOK right" still
needs Ben's eye. Closing that loop — letting Claude *see* its output — is worth exploring. A spectrum,
cheapest→richest:
- **ASCII/text slices (cheapest, works today).** Print top-down + side Z-slices of a voxel model as text/ANSI
  grids. Claude "sees" structure — is the door where it should be, roof shaped right, silhouette a wolf. Crude
  but real, zero dependencies, already partly prototyped (console-preview). Best for *structural* checks
  (placement, symmetry, holes); weak for aesthetic nuance.
- **Rendered image → Claude views it (richer, proven).** Render to PNG (matplotlib/offscreen Three.js), have a
  Claude session *view the image* (vision). **This is exactly what THIS design session did** — rendered the
  cottage/deer/wolf, judged by eye, caught the pyramid-roof failure, iterated. So it WORKS; the task is wiring
  render→view→judge→iterate into the automated pipeline vs. doing it ad hoc. Best for real aesthetic judgment.
- **Multi-angle render sheets** — several angles + scale ghost in one sheet (a single angle hides problems).
- **Diff rendering** — before/after of a human edit so Claude *sees* what Ben changed (pairs with §3m change-
  log read-back).
**Caveats:** vision-judging is slower/costlier than numeric harnesses → it's the *final* judge after the cheap
harnesses pass (harness = floor, Claude's eye = a second ceiling-check before Ben's). Can't fully replace Ben
— taste/charm want a human. **Staging:** ASCII slices first (cheap structural sanity), then render→view as the
aesthetic gate, then multi-angle + diff. Not now — mapped.

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
