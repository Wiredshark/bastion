# Mine Access Archetypes — design capture (Ben, 2026-07-19)

Status: DESIGN-PENDING (roadmap, NOT for M3). Captured from Ben's vision; feeds the mine-complexity
ladder + the no-material access tier. Companion to STAIR-LADDER-MINE-ACCESS-DESIGN.md.

## The ask (Ben, verbatim intent)
Minecraft-style **delving mines** where colonists descend in a STAIRCASE fashion — with **multiple
types** (spiral, straight staircase, switchback…) — PLUS mines that **cut into hillsides and mountains**
(horizontal adits into a slope face, and vertical shafts into high terrain).

## ★★ CONCEPTUAL MODEL / north star (Ben 2026-07-19): real-world mines × ant colonies
The mines shouldn't be random holes — they should read + function like **real-world mines** AND **ant
colonies**, which together give the guiding model: an **emergent-yet-purposeful underground NETWORK that
grows organically**. This is bigger than "access to ore" — it's the colony's underground BODY. Especially
the dwarf fantasy (elaborate carved networks, living/working underground).

- **From real-world mines (engineering / purpose):** a HIERARCHY — main haulage arteries → secondary
  drifts branching along ore veins → working faces; connected LEVELS via shafts/declines; logistics +
  (eventually) ventilation. Purposeful, efficient, follows the resource. [real mine layouts]
- **From ant colonies (emergence / growth):** decentralized, organically BRANCHING tunnel networks that
  EXPAND with need; specialized CHAMBERS off the tunnels (storage, work, living — the colony's rooms);
  adaptive, self-organizing, no master blueprint. [Formica/leafcutter nest structure]
- **Synthesis (the target feel):** the AI grows the underground as a living network — main arteries +
  branches to resources + specialized chambers — carved in the primitive vocabulary below, expanding
  naturally as the colony delves and grows. Not one-shot generated; ACCRETED over time. The archetypes
  (below) are the building blocks; THIS is the shape they compose into. Great for dwarves especially.

## ★ Why this matters beyond aesthetics: it's the NO-WOOD access tier
The live no-wood LIVELOCK (Builder-3 find, 2026-07-19: wood-costed rungs unclaimable → prune/emit
livelock) has no graceful COMPLETION path today — a wood-less deep mine simply can't descend. A
**stone-CARVED staircase is dug from the terrain itself → costs NO wood.** So carved stairs are the
missing "no-material access tier": a colony with no wood can still delve by carving stairs (slower,
more dig-labor, but self-sufficient). Wood ladders/scaffolds stay the FAST/vertical option. This
resolves the design question flagged in decision #20.

## Carved PRIMITIVES (a graduated vocabulary the mine is composed from — all stone-carved, no wood)
Ben's framing (2026-07-19): staggered CARVED approaches the AI assembles. A real mine = a composition
of these primitives — an ENTRY + horizontal EXTENT + vertical CONNECTORS — chosen by terrain + goal.

ENTRIES (how the mine starts):
- **Carved-in (adit)** — cut into the VERTICAL face of a hill/mountain; horizontal drive into the slope.
  [real-world adits; "into the mountain" dwarf-hall entrance]
- **Carved flat-plains** — entry/cut on FLAT ground (surface cut → the mouth of a descending mine).
  [DF surface start; Minecraft flat-ground portal]

HORIZONTAL EXTENT:
- **Carved hallways** — level horizontal tunnels / galleries following ore or connecting rooms. [DF
  galleries; real drifts]

VERTICAL CONNECTORS (all carved, no-wood):
- **Carved stairs — descending** — stair cut down to the next level. Forms: straight / switchback /
  spiral (spiral = smallest footprint for depth). [Minecraft staircase & spiral mining; DF down-stairs]
- **Carved stairs — ASCENDING** — stair cut UP (return to surface, connect an upper level). Mines must
  come back up, not just down — ascending is a first-class primitive, not an afterthought.
- **Vertical shaft** — straight-down bore; the wood-ladder/hoist FAST option, or a carved spiral around
  it for the no-wood path. [DF shafts]

## ★ AI-NATURAL is the core principle (Ben)
For the AI colony this must be NATURAL/emergent — the AI composes the right primitives for the situation
as it delves, NOT special-cased: hillside goal → carve-in adit; flat-ground ore below → carved-flat
entry + descending stairs; ore spread laterally → carved hallways; need to return/connect up → ascending
stairs; no wood → all-carved (slower, self-sufficient); wood on hand → shaft+ladder as the fast option.
This is the natural extension of dig-provisioned-access (DPA): the AI provisions access in the carved
vocabulary, terrain-and-goal-driven, by default. The primitives ARE the AI's mine-building language.

## DEFERRED — structural supports / cave-in prevention (Ben 2026-07-19: "that can wait")
Eventually: DF-style **supports** (pillars / columns / beams) that hold up spans and PREVENT cave-ins —
an unsupported ceiling over a threshold span collapses; a placed/carved support holds it. Interacts with
the EXISTING cave-in mechanic (the CK/entombment system) as its PREVENTION layer: carved mines gain
structural integrity rules, and the AI (or player) places supports to keep spans safe. Adds real
decisions to mine layout (support spacing vs. dig efficiency) and stakes (neglected supports → collapse).
LOW PRIORITY / explicitly deferred — captured so it's not lost; reaches the queue well after M3 + the
archetypes + the no-wood tier. Prior art: Dwarf Fortress support/collapse; real mine roof-bolting/timbering.

## Player build mechanism — OPEN QUESTION (Ben flagged)
How the PLAYER designates/builds these (paint an archetype? place an entry + let the AI carve? a
blueprint palette of the primitives?) is deliberately UNRESOLVED — a separate design decision. The AI
path is the priority + should feel natural; the player-facing tooling comes after (or reuses the AI's
carve primitives as paintable pieces).

## Design surface (to flesh out — candidate for external-LLM offload)
- **Site selection / archetype choice**: how does the colony PICK an archetype? By terrain (flat →
  staircase; hillside → adit; peak → shaft), by material availability (no wood → carved stair; wood on
  hand → faster ladder shaft), by depth target (deep → spiral for footprint), by player designation?
- **Carve geometry + pathing**: the walkable-slope generation for each type; how colonists path a
  carved stair vs a ladder; integration with dig-provisioned access (DPA) + the mine-complexity ladder
  (M-tiers: which archetype unlocks at which tier).
- **Cost model**: carved stair = dig-labor only (no wood, slower); ladder shaft = wood + faster;
  adit = horizontal dig into slope (labor, no vertical-access material). Tune the tradeoff.
- **Aesthetics / legibility**: readable, deliberate-looking mines (not random holes) — the "dwarf hall
  cut into the mountain" fantasy; the player should recognize the archetype at a glance.
- **Assets**: carved-stair sprites/blocks, adit portal, shaft framing — asset request when scoped.

## Sequencing
Roadmap, behind M3 + the current no-wood ORDERING fix (which makes no-wood degrade to a clean
material-hold in the interim). This design is the eventual PROPER answer. Tie into the mine-complexity
ladder tiers. NOT a now-build; reaches the front of the queue as a design pass.
