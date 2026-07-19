# Codex/ChatGPT Design Prompt — Mine Access Archetypes + Emergent Underground Network

Paste into Codex/ChatGPT (it reads the build files from Google Drive). Pure DESIGN — no code, so it
parallelizes with M3 with zero collision. A builder implements later. Extends the existing capture.

---

You are designing the **mine access archetypes + the emergent underground network** for Project Bastion
(a Dwarf-Fortress-style colony sim forked from Veloren). This is DESIGN ONLY — you produce a design
document; a builder implements it later.

**SOURCE — the build files are in Google Drive** at `H:\My Drive\bastion-Chatgpt` — read the repo from
there. START from the existing capture `readme/MINE-ARCHETYPES-DESIGN.md` (the vision, the north star, the
carved primitives) and EXTEND it into an implementable design — do NOT reinvent it. Also relevant:
`readme/STAIR-LADDER-MINE-ACCESS-DESIGN.md`, the mine-complexity ladder (M-tiers), and dig-provisioned
access (DPA) in the server code. Read only what you need for the area you're designing.

**PROCEDURE — output discipline (follow exactly; keeps the run canonical in the chat window + cheap):**
- Write the finished Markdown only ONCE in chat. No duplicate Word doc, Google Doc, ZIP, checksum, or
  regenerated `.md`.
- Read only the source needed for the current area — do NOT load the whole repository or all prior docs.
- Do NOT reread the completed output — once written it stays canonical in chat unless archival is requested.
- Update only DELTAS — output only changed coverage-index rows and new ledger entries, not the complete
  historical index and ledger every run.
- Archive by byte-level COPY when possible — a future `.md` upload copies the already-produced content
  directly, without rewriting it.
- Avoid repeated failed tool operations — one failure report, no retry loops that flood context.

**THE NORTH STAR (from the capture):** mines should read + function like **real-world mines × ant
colonies** — an emergent, purposeful, ACCRETING underground network (the colony's body), grown by the AI.
For the AI this must be NATURAL/emergent, not special-cased; the player build mechanism is a separate open
question (design the AI path first).

**DESIGN THESE (implementable — name geometry, thresholds, the selection logic, integration points):**
1. **Carve geometry per primitive** — for each: carved-in (adit into a slope face), carved-flat (flat-
   ground mouth), carved hallways (level galleries), carved stairs DESCENDING and ASCENDING (straight /
   switchback / spiral), vertical shaft — the walkable-slope shape, dimensions, dig pattern, and how a
   colonist paths it. All stone-carved = NO WOOD (this is the no-material access tier that fixes the
   wood-less-mine descent gap).
2. **Terrain → archetype selection** — how the AI PICKS a primitive: by terrain (hillside → adit; flat +
   ore-below → flat mouth + descending stairs; peak → shaft), by material (no wood → all-carved, slower;
   wood on hand → shaft+ladder fast option), by depth (deep → spiral for footprint), by goal location.
3. **Emergent network GROWTH (the ant-colony × real-mine core)** — the heuristics/algorithm by which the
   AI ACCRETES the network over time: main haulage arteries → secondary drifts branching along ore veins
   → working faces; specialized chambers off the tunnels (storage / work / living); how it decides to
   extend, branch, or add a chamber as the colony grows and needs. NOT one-shot generated — grown.
4. **Cost model** — carved (dig-labor only, no wood, slower) vs wood ladder/shaft (wood + faster); the
   tradeoff the AI weighs. Integrate with DPA (dig-provisioned access) as the natural extension: DPA
   provisions access in this carved vocabulary by default.
5. **Integration + tiers** — which archetype unlocks at which mine-complexity M-tier; how it composes with
   the existing DPA + the deferred supports/cave-in layer.
6. **Assets** — list the sprites/blocks needed (carved-stair variants, adit portal, shaft framing, chamber
   markers) as an explicit asset request.
7. **Open questions** — flag anything undetermined from the source for the builder/architect.

**OUTPUT:** one design doc with those sections, concrete enough to implement. Deltas only on later passes.
