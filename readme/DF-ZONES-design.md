# Project Bastion — DF-ZONES Design v0.1 (typed activity & building zones)

**The umbrella zone schema: typed places-with-a-purpose (meeting / pasture / pond / refuse / hospital / water /
gather) that softly bias autonomous behavior — beyond the stockpile.** Companion to `BASTION-SYSTEM-
FRAMEWORKS.md` §2 (the canonical `purpose` enum + activity-vs-building distinction — this pass makes it real),
the B5.6b zone-management work (the paint/overlay/z_extent/color substrate it rides), the DF gap ledger (§J
DF-ZONES), and FOUNDING-EMBARK §5 (zones organizing autonomous growth).

**Which wall:** **LOAD-BEARING SCHEMA** first (locking the zone-type vocabulary while B5.6b-2 hardens it into
code), then **SIMULATION** (each zone biases a behavior). The paint/legibility wall **already fell** to B5.6b
(zone fills, colors, labels, z_extent).

**Fit-check verdict: PASS (the cleanest Manage-layer fit in the ledger).** A zone is **soft policy** — you
designate a *purpose for a place* ("this is the meeting ground", "graze animals here") and autonomous behavior
*flows toward it*; you never command a colonist there. Frameworks §2 already states the guardrail: **"soft
preference, not iron law — zones organize autonomous growth; they don't forbid."** Held to that, DF-ZONES is
the Manage tier of the control spectrum made concrete. Break it (a zone that *forces* colonists) and it drifts
toward RTS command (AVOID).

**Ledger/corpus entries this consolidates:** `df-feature-gap-ledger.md` §J **DF-ZONES** ("typed zones beyond
stockpiles; Veloren `AreaAdd` → SUBSTRATE"). Adjacent (not designed here — separately claimable): **DF-BURROW**
(movement *restriction*, the hard-policy cousin). Appends to the corpus; rewrites nothing.

---

## 0. The one thing to get right first — a zone is a soft magnet, not a fence

Every temptation here is to make zones *authoritative*: "colonists MUST meet here", "animals CANNOT leave the
pasture". That is the RTS/DF-command reflex, and it violates the pillar. In Bastion a zone is a **soft magnet**:
it *raises the utility* of an activity at a place, so autonomous minds (agency bible §5b) *choose* to go there
more often — but a colonist with a stronger drive (hungry, threatened, a personal need) overrides it freely.

This also fixes the honest truth about DF-ZONES (§8): **a zone is only as valuable as the behavior it biases,
and most of those behaviors are separate systems.** A "hospital zone" does nothing until DF-MEDICAL exists; a
"pasture" is inert without DF-LIVESTOCK. So this pass does **two cheap, high-value things now** — (1) **lock the
zone-type vocabulary** (load-bearing, rides B5.6b-2), and (2) **build the one binding *mechanism*** (a zone
raises an activity's utility) — and then each zone type *plugs into that mechanism* as its behavior system
lands. **North star: lock the vocabulary, build the magnet once, let behaviors plug in.**

---

## 1. The reuse split — the de-risk table

DF-ZONES is **almost entirely wiring on B5.6b + the §2 taxonomy** — the paint, overlay, storage, and typing all
exist or are landing; the net-new is a thin behavior-bias hook.

### SUBSTRATE — exists / landing

| Piece | Real symbol / location | What it gives us |
|---|---|---|
| **Zone paint + overlay + z_extent + colors + labels** | B5.6b (`DebugShape::ConformedTris`, `bastion::draped_fill_tris`, `zone_rgb`/`zone_fill_color`, centroid labels; `z_extent` in b-2) | The entire *designate-and-see-a-typed-zone* UX. DF-ZONES adds zone *kinds*, not new rendering. |
| **The canonical `purpose` enum** | `BASTION-SYSTEM-FRAMEWORKS.md` §2 — 8 kinds (residential/industrial/commercial/religious/civic/defensive/storage/agricultural), hardening in **B5.6b-2** | The classification key. DF-ZONES' activity subtypes map onto these; **this is where the vocabulary lock lands.** |
| **`DesignationKind` (the paint verb enum)** | `common/src/bastion.rs:141` — `{ Mine, Chop, Build, Stockpile }`; `Stockpile → WorkType::Haul` | `Stockpile` is the **first activity zone** — the exact pattern the others follow. Zones are new variants here (or a `Zone(ZoneKind)` variant). |
| **Veloren named typed areas** | `common/state/src/special_areas.rs` — `Areas` (named `Aabb` regions), `AreasContainer<Kind>`, `AreaKind` (`BuildArea`, `NoDurabilityArea`) | The engine's typed-region storage substrate (the ledger's `AreaAdd`). A model for typed zone storage; Bastion may use its own `Region` list (B2a) instead. |
| **Idle/meeting behavior** | rtsim/agent idle + gather behaviors (`server/agent/`, `rtsim/src/rule/npc_ai/`) | Meeting/gather zones bias an *existing* idle behavior — the magnet plugs into what agents already do when unoccupied. |
| **Haul (refuse)** | B6 hauling (`WorkType::Haul`) | Refuse/garbage-dump = haul marked items to a spot — rides B6, near-READY. |
| **Mood/thoughts** | B-AG3 (DONE) | Being in a pleasant zone (a nice meeting hall) feeds thoughts (ties DF-ROOMS). |

### BUILD — genuinely net-new

| Piece | Why it's new | Folds into |
|---|---|---|
| **The zone-type vocabulary** | No typed-zone enum beyond `Stockpile`; the §2 purpose kinds + activity subtypes must become a locked enum. | frameworks §2 (the schema lock) |
| **The behavior-bias mechanism (the magnet)** | Nothing yet lets a painted zone *raise the utility* of an activity at a place. This is the one real, reusable hook. | The agent/rtsim **utility/behavior** layer (build-once) |
| **Per-zone-kind behavior wires** | Each kind (meeting→socialize, refuse→dump, water→drink…) needs its small binding — most **gated on the behavior's own system**. | Each target system (mostly NEEDS-tagged) |

**The collapse:** DF-ZONES = **a locked vocabulary + one magnet mechanism + thin per-kind wires**. The scary
breadth ("all of DF's zones!") is honest-limited by the fact that each zone's *value* is its behavior system —
so this pass is deliberately a **thin, high-leverage schema+mechanism layer**, not a promise to build every
zone's behavior.

---

## 2. The zone catalog (DF activity zones → Bastion, READY/NEEDS-tagged)

DF activity zones ([DF Activity zone](https://dwarffortresswiki.org/index.php/DF2014:Activity_zone)) reframed
as soft magnets, each mapped to a §2 `purpose` and tagged by whether its biasing behavior exists:

| DF zone | Bastion zone | §2 purpose | Behavior it biases | Tag |
|---|---|---|---|---|
| Stockpile | **Stockpile** (built, B6) | storage | store/haul items here | **READY** (B6) |
| Garbage dump | **Refuse** | storage | haul marked/rotting items here (ties DF-ROT) | **near-READY** (rides B6 haul) |
| Gather/pick | **Gather** | agricultural | forage wild plants/fruit within the zone | **near-READY** (rides B5 chop/gather verb) |
| Meeting area | **Meeting** | civic | idle colonists congregate/socialize here | **NEEDS:socializing** (B-AG / DF-TAVERN) |
| Pen/Pasture | **Pasture** | agricultural | grazer animals kept + graze here | **NEEDS:DF-LIVESTOCK** |
| Pond | **Pond** | agricultural | colonists fill with water (bucket from a source) | **NEEDS:DF-LIVESTOCK + water-fetch** |
| Water source | **Water** | civic | draw water to satisfy thirst / give water | **NEEDS:B7 thirst + water-fetch** |
| Hospital | **Hospital** | civic | wounded rest + are treated here | **NEEDS:DF-MEDICAL + DF-WOUND** |
| (location: tavern/temple/…) | *deferred* → their own systems | commercial/religious | — | **NEEDS:DF-TAVERN / DF-RELIGION** |
| Farm plot | **Farm** (= DF-FARM PROD-2) | agricultural | till/sow/harvest | designed in DF-PRODUCTION |
| Workshop district | **Production** (= DF-PRODUCTION S2) | industrial | station work | designed in DF-PRODUCTION |
| (movement restriction) | **Burrow** → **DF-BURROW** (adjacent, not here) | — | restrict movement (hard policy) | separate item |

**v1 proves the mechanism with the zones whose behavior already exists** (Refuse, Gather) + the lock; the rest
flip READY as their systems land — the §3i delegation loop applied to *behavior*, not assets.

---

## 3. Systems needed

### S1 — The zone-type vocabulary (the schema lock)
A locked `ZoneKind` enum (the activity subtypes above), each carrying its §2 `purpose` + `z_extent` profile
(thin for activity zones per frameworks §2). Represented on the paint pipeline as `DesignationKind::Zone(ZoneKind)`
(or sibling variants). **Where:** `common/src/bastion.rs` + frameworks §2 (the canonical note). **Deps:**
B5.6b-2 (`purpose` enum). **Folds into:** the §2 taxonomy — **this pass canonicalizes the activity-zone half.**

### S2 — The behavior-bias mechanism (the magnet — build once)
One reusable hook: a zone of kind K **raises the utility** of activity K's target-location within its footprint,
so an autonomous agent choosing among activities is *drawn* there — but a stronger drive overrides (soft, per
§0). This is the single reusable mechanism every zone kind plugs into. **Where:** the agent/rtsim utility layer
(a "zone attraction" input to behavior selection). **Deps:** agent behavior tree. **Folds into:** the
control-spectrum Manage layer (build-once — like the world-verb library, one mechanism many callers).

### S3 — Per-kind behavior wires (mostly NEEDS-gated)
Each zone kind's thin binding to its behavior: Refuse→haul-to-zone, Gather→forage-in-zone (both near-READY);
Meeting/Pasture/Hospital/Water/Pond → **inert stubs that light up when their system lands** (each is a few lines
once the behavior exists). **Where:** each target system's module. **Deps:** per-kind (see §2 tags). **Folds
into:** each behavior system (the zone is the cheap interface; the behavior is the work).

---

## 4. Assets & animations

**Assets:** minimal. Zones are painted regions (B5.6b rendering) — no models. **Zone-marker props** (a
signpost/banner per kind so a zone reads without the overlay — meeting-post, refuse-marker, pasture-fence) are
**[A] READY-ish** polish (asset-lab can generate; the overlay already carries the legibility). Pasture fences
tie DF-LIVESTOCK. Nothing gates the schema.

**Animations:** **none new.** Zones bias *existing* behaviors (idle/gather/haul) whose animations are NATIVE or
already owed by their own system (socialize→DF-TAVERN, treat→DF-MEDICAL). DF-ZONES adds **zero animation debt.**

---

## 5. Legibility · Control-spectrum · LOD

**Legibility:** B5.6b already renders typed zones (kind color + fill + centroid label); DF-ZONES just adds more
kinds to that palette. Per-zone inspector: what the zone is for + who's using it + whether it's *inert* (a
Hospital painted before DF-MEDICAL exists shows "no effect yet — needs medical system", so the player isn't
misled — an honest-legibility touch). Zone activity (colonists drawn here) is visible directly.

**Control-spectrum:** DF-ZONES **is** the Manage tier — soft policy, autonomous execution. No Direct mode (you
don't hand-place colonists in a zone). **God layer:** light — a god could *consecrate* a meeting ground
(passive mood aura, ties DF-RELIGION/God-Powers) or *bless* a pasture's fertility; these are B13 blessings on a
zone, not core here.

**LOD:** zones are cheap data (a typed AABB + a utility bias). **Loaded:** the magnet biases real behavior
selection. **Unloaded (rtsim):** zones inform the aggregate ("this colony has meeting/pasture/hospital
coverage" → small mood/health tendency) — tendency-first, no per-agent zone pathing in rtsim. Zones persist
trivially (serde AABB + kind). No accumulation/decay (a zone is permanent policy until erased).

---

## 6. Sequenced sub-blocks, each with a concrete Done-when

Dependency-ordered. **v1 = ZONE-0..ZONE-1; the rest is per-system flip.**

### ZONE-0 — Lock the vocabulary + the magnet mechanism · [the load-bearing core]
**Depends:** B5.6b-2 (`purpose` enum). Builds S1 + S2.
**Scope:** the `ZoneKind` enum (activity subtypes → §2 purpose), painted + rendered (reuse B5.6b), + the
utility-bias magnet (S2), proven with **one already-behaviored zone**.
**Done-when (`--zone-scenario`):** paint a **Meeting** (or Gather) zone; idle colonists demonstrably **spend
more idle time inside the zone footprint** than a matched control area (a measurable attraction bias over N
ticks) — **and a colonist with a stronger drive (hunger/threat) still leaves freely** (the soft-magnet
invariant: the zone biases, never traps — asserted, not just eyeballed). Zone persists through save/load.

### ZONE-1 — Refuse + Gather (the READY behavior wires) · [v1 value]
**Depends:** ZONE-0, B6 (haul), B5 (gather verb). Builds the two near-READY per-kind wires (S3).
**Scope:** Refuse = haul items marked-for-dump to the zone (ties DF-ROT); Gather = forage wild plants within
the zone.
**Done-when (`--zone-behavior-scenario`):** items marked for dumping are hauled **to the Refuse zone** (and
nowhere else) with conservation (no item loss/dupe); a Gather zone over wild plants gets its plants foraged into
a pile, bounded, zero-input soak stable.

### ZONE-2+ — Per-system zone flips · [enrichment, gated]
**Depends:** each target system. Each is a **thin wire** lighting up a stubbed zone kind when its behavior lands:
Meeting→DF-TAVERN, Pasture/Pond→DF-LIVESTOCK, Water→B7 thirst, Hospital→DF-MEDICAL.
**Done-when (per flip):** the previously-inert zone now biases its behavior (e.g. once DF-MEDICAL exists, a
Hospital zone measurably draws wounded colonists to rest/treat within it), asserted in that system's scenario.
These are **not this pass's build** — they're the delegation contract, tracked so the schema pays off over time.

---

## 7. Dependencies · open questions · tuning-data · corpus notes

### Dependencies
- **B5.6b-2 (`purpose` enum + z_extent) — HARD for the lock.** DF-ZONES canonicalizes the activity-zone half of
  the §2 taxonomy; it must land *with or just after* B5.6b-2 so the vocabulary is locked once. **This is the
  load-bearing-now timing.**
- **B6 (haul) / B5 (gather) — for ZONE-1** (the only behaviored v1 wires).
- **Per-kind behavior systems** — DF-TAVERN, DF-LIVESTOCK, DF-MEDICAL, B7, DF-ROT — each gates its zone's value
  (ZONE-2+). **Designed to degrade:** the schema+magnet ship now; behaviors plug in later.

### Open questions (flagged for the architect)
1. **Enum shape:** one flat `ZoneKind` enum, or `DesignationKind::Zone(ZoneKind)` wrapping a purpose + subtype?
   *Rec:* `DesignationKind::Zone(ZoneKind)` where each `ZoneKind` carries its `purpose` — keeps paint verbs and
   zone kinds cleanly separated and lets the overlay color by purpose. Lock with B5.6b-2.
2. **Storage:** reuse the engine's `special_areas::Areas` typed-region store, or Bastion's own B2a `Region`
   list? *Rec:* Bastion's own `Region` list (already 3D, already synced/overlaid via B5.6b) — `Areas` is
   name-keyed and build-permission-oriented; don't overload it.
3. **Soft-magnet strength** — how strongly should a zone bias vs personal drives? *Rec:* a **tunable utility
   weight** (RON) small enough that needs always win — start conservative; the §0 guardrail is the invariant.
4. **DF-BURROW (restriction)** — fold the hard-restriction cousin into this pass, or keep separate? *Rec:* keep
   **separate** — a burrow *forbids* movement (hard policy), a zone *attracts* (soft); mixing the two blurs the
   pillar-critical soft/hard line. Flag DF-BURROW as its own claimable topic.

### Tuning-data (RON)
Per-kind zone-attraction utility weight; z_extent profile per kind (thin/tall); refuse-haul priority; gather
radius. All data.

### Corpus notes
- **Canonicalizes the §2 activity-zone half** — recommend the locked `ZoneKind` list land in frameworks §2
  alongside the `purpose` enum (like the §2b Quality lock this session added), so future zone systems defer to
  it. No contradiction — this *is* §2 made real.
- **Consistent with FOUNDING-EMBARK §5** (zones organize autonomous growth, soft preference) and DF-PRODUCTION
  (Farm/Production zones are `ZoneKind`s designed there) — this pass is the umbrella; those are two of its kinds.

## 8. Honest limits
- **DF-ZONES is a thin schema+mechanism layer, and this design says so plainly.** Its breadth is real but its
  *value per zone* is the behavior system behind it — most of which are unbuilt. Over-selling it as "all of
  DF's zones, done" would be the exact stale-design-ahead-of-substrate failure the architecture guards against.
  The honest deliverable: **lock the vocabulary now** (cheap, load-bearing, correct to do while B5.6b-2 hardens
  it) + **build the magnet once** + **prove it with the 1–2 zones that already have behavior** (Refuse/Gather).
- **The magnet's tuning is the subtle risk** — too strong and it reads as command (pillar break); too weak and
  zones feel decorative (function-wall break). ZONE-0's Done-when tests *both* failure modes (attraction exists
  AND stronger drives override).
- **Everything past ZONE-1 is a delegation contract, not a build** — correctly deferred to each behavior
  system, tracked so the schema pays off incrementally rather than rotting.

*End of DF-ZONES design. The typed-zone breadth DF is known for is, here, a locked vocabulary on the B5.6b paint
substrate plus one soft-magnet mechanism — deliberately thin, because a zone's worth is the autonomous behavior
it gently bends toward it, and those behaviors arrive with their own systems.*
