# BASTION SYSTEM FRAMEWORKS — consolidated reference

**Purpose:** one doc collecting the system frameworks designed across the recent architecture sessions, so
builders and future sessions get the *frameworks* (the reusable shapes) without excavating the full
future-work catch-all. Each section points to its detailed source (`readme/future-work-and-deferred-ideas.md`
§ refs). Build status: see `docs/BASTION_RUN_LOG.md` + `BASTION_ARCHITECTURE.md §6` for the live green tag and
next block — this is a *frameworks* reference, not a status tracker (a hard-coded status line here goes stale).

---

## 1. The Control Spectrum — one sim, dialed involvement (design doc §3d, future-work §3q)
**The master pattern.** Every domain of colony life runs on ONE autonomous system with three levels of player
override: **Autonomous** (watch it happen) / **Manage** (set policy) / **Direct** (do it yourself). Extended
into the embodiment spectrum (god → DF-manage → RTS-command → god-embodied avatar → mortal RPG).
- **Construction:** automatic / zoning / direct placement — one catalog, one placement system.
- **Governance ("ruler stuff"):** AI ruler you influence / some decisions yours / you ARE the ruler. God ≠
  ruler: the god influences, a mortal leader decides.
- **Military, economy, mining** — same pattern (see §6 below for mining's three modes).
**GUARDRAIL:** autonomous is the default and the soul; manage/direct are optional depth, never mandatory —
or it drifts into 4X management (AVOID).

## 2. Zone ↔ Asset shared taxonomy + 3D zones (future-work §3e-schema, §3q, §3v)
- **One purpose enumeration** shared by zones and assets: residential→housing, industrial→production,
  commercial→commerce, religious→faith, civic→social, defensive→defense, storage→storage,
  agricultural→farming. The classification IS the matching key ("what can be built in this zone?").
  **These 8 purpose kinds are the ONE CANONICAL zone↔asset enumeration — the authoritative source when it
  becomes a Rust enum (B5.6b-2).** Other docs that restate it (future-work §3e-schema / §3m / §3q / §3z, the
  asset schema) DEFER to this list; where they drift (7- vs 8- vs 9-kind copies exist), THIS wins.
- **Activity zones** (storage, farm — where things happen) vs **building zones** (what structures belong).
- **Soft preference, not iron law** — zones organize autonomous growth; they don't forbid.
- **Zones are 3D:** schema gains `z_extent`. Thin (farm/storage: surface+1–3), tall (building zones:
  footprint+height), deep (mine: footprint × N levels down). Lock the schema field now.

## 3. The Asset Pipeline (future-work §3e–§3m; live: asset-lab, 12 REAL assets, ladder rungs 0–9 PASS)
Research-ground → design-intent (sizing/ornateness/importance/lore-fit; colonist-fit is a FLOOR not a cap) →
generate (prefer VARIATION; component system for big assets) → verify (style harness + function harness;
static content-side, dynamic game-side per the flat-plane spec) → lore (readable + system hooks + coherence
check) → log (TEST/REAL, READY/NEEDS:<system>, append-only).
- **Component system:** persisted addressable chunks + composition manifests + registry = big-asset chunking
  + parts library + variation, one mechanism (mirrors the game's own parts+manifest architecture).
- **Human editor + change-log read-back:** Ben fixes by hand; Claude re-verifies and *learns* the patterns.
- **Delegation:** every asset tagged READY (a system consumes it) or NEEDS:<system> (inert until code exists).
  The tagged catalog is the interface between the content pipeline and the build queue.

## 4. Animation — creatures and actions (future-work §3l, §3u)
- **Creatures:** skeletal, code-defined per body family; parts are bones (never deform). **Generate-to-
  skeleton = inherit animation free.** New body plan = new skeleton = code (the custom-creature capability
  test gates the unique-creature content axis: bosses, husbandry species, the god-companion's BODY — the
  companion's soul is a mind/relationship convergence, not an asset).
- **Actions:** procedural Rust per animation, **state-driven selection, tool-parameterized** — so **mining
  and chopping are NATIVE** (equip pickaxe/axe + drive the wield/swing CharacterState while working; the
  integration is wiring the job executor to set state+tool). Custom verbs (craft/farm/build-hammer/worship)
  = one new Animation impl each, tagged NEEDS:animation-code.
- **THE RULE:** every new work verb carries an animation line-item — NATIVE (prefer; bend toward existing
  states) or NEEDS:animation-code (named). No verb ships as a T-posing colonist.

## 5. Testing & validation (future-work §3j, §3o)
- **Style ≠ function; static ≠ dynamic.** Style harness (aesthetics) + function harness (usability) both
  gate; static geometry checks are content-side, dynamic NPC-pathing tests are game-side (B0/B4 harness).
- **Flat-plane arena:** isolated-dynamic tests in a controlled void with a minimal derived cast from a
  verified fixture library → then integrated-dynamic in the real world (mandatory — the plane's cleanliness
  is its blind spot) → then soak.
- **The headless harness is custom-on-intended** (Bastion's scenario-runner atop Veloren's standalone
  server) — the most load-bearing infrastructure; keep its assumptions documented.
- **Invariant-first everywhere:** assert properties that must hold, not exact traces; test the floor of
  "not broken," never the ceiling of "does exactly this."

## 6. The Mining Framework (future-work §3v)
A mine is a **building dug in negative space** — it rides the building-catalog machinery.
- **Constructed mines:** parameterized mine template (adit/shaft → gallery → branches → per-level
  stairs/ramps → stockpile), dug progressively via the B5 work-tick, hauled via B6. **Access is part of the
  dig plan** (the B5 pit-trap bug, solved at framework level). Three control modes: paint-your-own / mine
  zone ("8 levels down") / fully autonomous (ore survey → plan → dig).
- **Prospecting & spelunking:** scouts discover cave entrances (colony knowledge model — discovered ≠
  omniscient); assess (ore? residents?); exploit exposed veins (cheaper, riskier); clear/delve dungeons
  (military objective + the mortal-RPG's adventure content).
- **THE BREACH EVENT:** a constructed mine breaking into a natural cavern — threats flow out through the new
  opening. "Dig too deep" is the genre's sacred moment and nearly free (caves exist; breach = void detection
  + hazard-event + pathing). Flagship emergent moment.
- Water/lava dig-ins inert until DF-FLUID; cave-ins deferred to DF-STRUCT; both slot into the hazard engine.

## 7. World Connective Tissue (future-work §3s) — the inter-settlement layer, Tier-2/3
Dependency order: **territory/region tracking** (claim map; extends rtsim factions; could come earliest) →
**roads & bridges** (autonomous road-building; roads channel trade+armies; bridges = chokepoints) →
**spatialized Divine Politics** (trade on physical interdictable routes; territorial war; conquest updates
the claim map) → **sea lanes** (gated on naval movement) → **daughter settlements** (thriving colonies found
new ones). Constraints: the map+overlays IS the interface (legibility); world-tier abstract LOD (cheap
unwatched, concrete watched); everything must DO something; the god influences it all (bless routes, sunder
bridges, guide expansion).

## 8. Framework steals from the deep research (future-work §3t)
- **CK3:** casus belli — wars need justification → every war is *legible* ("why?" always has an answer);
  de-jure vs de-facto two-layer territory; universal opinion sums.
- **Dominions:** dominion-spread — faith as a territorial FIELD radiating from temples/prophets, smothering
  rival gods. THE competing-gods territorial mechanic; steal wholesale.
- **Distant Worlds:** fully autonomous private economy, player governs only the state layer — the purest
  influence-not-command economy model.
- **X4 / Elite BGS:** physical economy when watched (real caravans on real roads) + abstract faction states
  (boom/war/famine) when not — both, per LOD.
- **Songs of Syx:** requirement-based soft annexation (settlements JOIN you when you control the area and
  meet their needs — annexation by care); knowledge regression (tech must be maintained — the anti-runaway
  for autonomous advancement §3f).
- **Total War:** supply/attrition (roads matter militarily; armies off-road wither); administration scaling
  (anti-blob for nations); sieges as processes.
- **DF:** history continues during play; overlapping claims; time-scrubbable political map; named AGES
  ("Age of the Wolf-God") from real sim state.
- **Caves of Qud:** conflicting historical accounts — the chronicle needn't be omniscient; cultures record
  the same war differently; lore items carry versions of history.
- **Nemesis (Shadow of Mordor):** recurring personal rivals with memory, at the world tier (the raider
  captain who returns scarred and vengeful).
- **Meta-lesson:** every framework splits abstract-world-tier from concrete-local with events flowing both
  ways — exactly Bastion's loaded↔simulated architecture. Everything slots in; nothing contradicts.

## 9. Founding & embark (future-work §3n)
B3's found-in-existing-town is a placeholder. Real flow (B11): world select → survey (B1.8 camera) → site
selection with **suitability surfaced to the player** (same scoring the autonomous system uses) → founding
band spawned at the chosen empty site → starting conditions → drop in. God mode = choose where your people
settle; the growth-from-nothing arc is core colony-sim appeal.

## 10. Standing principles (apply to everything above)
- **Which wall?** Content (asset pipeline lifts it) / simulation / design-fit / legibility. Only content fell.
- **Build once, many uses:** trigger→link→effect; the component system; the world-verb library; shared
  zone/asset taxonomy; site-suitability (autonomous + player-facing).
- **Autonomous by default; involvement by choice.** Influence, not command — at every scale.
- **Legibility is a pillar.** Every system needs its overlay/chronicle answer at design time.
- **Everything must DO something.** Decorative systems get cut.
- **Experimental flags for foundational risk** (worldgen, fluid): separate paths, base game never at risk.

*Sources: `veloren-colony-rts-build-report.md` (design doc), `future-work-and-deferred-ideas.md` (§ refs
above), `MASTER-COLLATION-index.md` (session state), the agency/DF/divine-politics bibles.*
