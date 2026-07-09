# Project Bastion — DESIGN STATUS (the living design-frontier map)

The "what's designed / designing / still `[LEDGER]`" map for the **GENERAL DESIGNER** workflow
(`readme/GENERAL-DESIGNER-prompt.md`). Kept current so an amnesiac or parallel design session resumes
correctly. Companion to `readme/DESIGN_PASS_LOG.md` (the append-only claim/done ledger) and
`readme/df-feature-gap-ledger.md` (the full DF-* inventory). Build frontier: `BASTION_ARCHITECTURE.md §6`.

**Resume point (2026-07-09):** design frontier advancing fast (multiple sessions). **DESIGNED so far:**
DF-PRODUCTION, DF-DIG-VERBS, DF-QUALITY(+DF-ARTIFACT), DF-ZONES, DF-BURROW (this session, clean pause on context
budget — backlog NOT exhausted) + DF-HIST(+DF-LOG), DF-RELIGION (parallel). Two schema locks landed (Quality →
frameworks §2b; ZoneKind → §2 recommended). The spatial-policy pair is complete (ZONES soft-attract + BURROW
hard-restrict). **Next unclaimed near-frontier Tier-1/2:** DF-ORDERS (PROD-1 seeds it), DF-TAVERN (unblocks the
DF-ZONES Meeting wire + DF-RELIGION gather loop), then DF-WOUND / DF-MECH-TRAP-OPERABLE cluster / DF-TRADE /
DF-CAVERN+DF-GEOLOGY. Always check `DESIGN_PASS_LOG.md` for live CLAIMs before starting (parallel sessions run
concurrently).

---

## DESIGNED (has a buildable design doc with Done-when sub-blocks)
| Topic | Doc | Sub-blocks | Notes |
|---|---|---|---|
| Founding / Embark (B11) | `FOUNDING-EMBARK-DESIGN.md` | B11.0–B11.8 | pre-existing pass |
| God-Powers (B13) | `GOD-POWERS-CATALOG.md` | verb menu | pre-existing pass |
| **DF-PRODUCTION** (DF-WORKSHOP + DF-CHAIN + DF-FARM + DF-COOK) | **`DF-PRODUCTION-design.md`** | **PROD-0..PROD-5** | this session. Behind B6; farm partly precedes. Refines ledger costs. |
| **DF-HIST** (+ DF-LOG consolidated) | **`DF-HIST-design.md`** | **HIST-0..HIST-6** | Chronicle/Legends — the legibility organ. rtsim data+event-bus = substrate; net-new = persistent player sink + the `record()` capture API (lock first) + feed (DF-LOG) + browser. v1 = HIST-0..2. No 3D/anim. |
| **DF-RELIGION** (temples/worship/prophets/faith seam) | **`DF-RELIGION-design.md`** | **REL-0..REL-5** | Best-fit topic (you ARE the god). Splits: colony tier = mostly WIRE (tavern gather-loop retargeted at a temple + a `worship` field on `Needs` + a `Priest` `Profession` arm + a devotion accumulator) behind **B7**; world faith-politics (convert/rival gods/holy war/festivals) = **NOT DF-RELIGION**, it's Divine-Politics DP2–DP4 (LATE). Direct tier deliberately empty. REL-0 (buildable `faith` zone) precedes B7. Cheapest animation in the ledger (v1 NATIVE). |
| **DF-DIG-VERBS** (stairwell/ramp/channel/shaft-ladder) | **`DF-DIG-VERBS-design.md`** | **DIG-0..DIG-4** | this session. Vertical excavation vocab. Mostly wiring on B5 (worldgen ships ramp/staircase/spiral primitives + Ladder sprite + Climb state). Net-new = designation verbs + reachability-safe top-down decomposer (solves pit-trap) + top-down painting UX. KEY: ramp verb == B5.8 auto-carve-steps (one `carve_ramp` lib). HARD PAIR w/ B5.8. Zero new animation debt. |
| **DF-QUALITY** (+ DF-ARTIFACT apex) | **`DF-QUALITY-design.md`** | **QUAL-0..QUAL-3** | this session. LOCKED canonical quality enum = engine `item::Quality` (frameworks §2b). Net-new = skill→quality craft stamp (fills DF-PRODUCTION S6) + per-instance `craft_quality` field + B-AG3 quality→thought hook + strange-mood event (DF-ARTIFACT faithful: artifact-or-death). HARD dep DF-PRODUCTION S1; reuses B-AG3 (DONE). |
| **DF-ZONES** (typed activity/building zones) | **`DF-ZONES-design.md`** | **ZONE-0..ZONE-2+** | this session. Umbrella schema for §2 activity-zone half (canonicalizes it, load-bearing w/ B5.6b-2). Net-new = locked `ZoneKind` vocab + ONE soft-magnet mechanism (bias-not-command) + thin per-kind wires (most NEEDS-gated on their behavior system). v1 proves w/ Refuse+Gather. DF-BURROW kept separate (hard vs soft policy). |
| **DF-BURROW** (movement-restriction zones) | **`DF-BURROW-design.md`** | **BURROW-0..BURROW-3** | this session. Hard-policy cousin of DF-ZONES (completes the zone-policy pair). Two filters on B4 (job-claim + idle-clamp); the real work is the pillar reframe: a burrow = the god's "Call to Shelter" directive WITH a survival escape valve (critical hunger breaks confinement — the DF safe-room-death-trap designed OUT). Standing mode v1; siege shelter-alert gated B8; survival valve gated B7 (never ship confinement without ≥ a stub override). Zero animation debt. |

## DESIGNING / CLAIMED (in progress — check DESIGN_PASS_LOG before claiming)
_(none currently claimed)_

## HIGH-PRIORITY `[LEDGER]` — the recommended next passes (aim here)
Ordered by the selection criteria (near build frontier · unlocks content batch · Tier-1). The frontier is at
B5.6b → B6; anything riding B6 (hauling) or B5.8 (vertical mobility) is near-term real.

1. ~~**DF-DIG-VERBS**~~ **DESIGNED** this session (`DF-DIG-VERBS-design.md`; DIG-0..DIG-4). Mostly wiring on B5;
   the ramp verb unifies with B5.8's auto-carve-steps (one `carve_ramp` lib — flagged so it isn't built twice).
   **HARD PAIR with B5.8** — sequence B5.8 first or co-build. Zero new animation debt.
2. ~~**DF-RELIGION**~~ **DESIGNED** this session (`DF-RELIGION-design.md`; REL-0..REL-5). Colony tier behind
   B7 (mostly wire — the tavern loop retargeted); world faith-politics handed off to Divine-Politics DP2–DP4
   (LATE). Faith-asset batch → ASSET_REQUESTS. Recommend the architect **split the ledger's DF-RELIGION line**
   (colony `$` vs faith-politics `$$$`/Divine-Politics).
3. **DF-ORDERS** (policy-layer) — partly seeded by DF-PRODUCTION PROD-1 (standing targets); full conditional
   orders remain. DF-LOG **DESIGNED** (folded into DF-HIST as the HIST-2 feed). ~~**DF-ZONES**~~ **DESIGNED**
   this session (`DF-ZONES-design.md`; ZONE-0..ZONE-2+) — umbrella schema, rides B5.6b + canonicalizes the §2
   activity-zone half. **DF-BURROW** (hard-restriction cousin) still `[LEDGER]`, separately claimable.
4. ~~**DF-HIST** — the Legends/Chronicle browser.~~ **DESIGNED** this session (`DF-HIST-design.md`; DF-LOG folded
   in as the HIST-2 feed slice). The `record()` capture API is load-bearing — lock before emitters harden.
5. ~~**DF-QUALITY + DF-ARTIFACT**~~ **DESIGNED** this session (`DF-QUALITY-design.md`; QUAL-0..QUAL-3). Quality
   enum **LOCKED** = engine `item::Quality` (frameworks §2b). Fills DF-PRODUCTION S6. Strange-mood kept faithful.
6. **Next unclaimed near-frontier Tier-1/2 (aim here):** **DF-ORDERS** (policy; PROD-1 seeds it), **DF-TAVERN**
   (rides B7 + social — also unblocks the DF-ZONES Meeting wire + DF-RELIGION's gather loop), **DF-BURROW**
   (cheap hard-policy zone), then DF-WOUND, DF-MECH/TRAP/OPERABLE (trigger→link→effect cluster), DF-TRADE,
   DF-CAVERN + DF-GEOLOGY (the vertical world, ties DF-DIG-VERBS + mining framework).

## DEFER — premature (Tier-3 / sits on unbuilt world) — do NOT design ahead of substrate
DF-VILLAIN, DF-BEAST, DF-NIGHT, DF-KNOWLEDGE, deep DF-ECON, DF-BIOME-FX, DF-HYDRO, DF-TEMP, DF-FESTIVAL,
DF-GUILD, DF-ART, DF-MINECART, DF-RECLAIM. (Flag "premature — defer" if reached; they go stale before build.)

## Cross-pass seams to keep coherent (don't fork these)
- **`Quality` enum** — **LOCKED** (DF-QUALITY §0): canonical = engine `common::comp::item::Quality`; Bastion
  defers, never forks (frameworks §2b). Companion schema to lock with it: per-instance `craft_quality:
  Option<Quality>` on the item instance (the skill stamp). DF-PRODUCTION S6 ↔ DF-QUALITY ↔ DF-ARTIFACT all use it.
- **`ZoneKind` vocabulary** — DF-ZONES §3 canonicalizes the §2 activity-zone half (recommend landing in
  frameworks §2 alongside the `purpose` enum). Farm/Production zones (DF-PRODUCTION) + faith zone (DF-RELIGION
  REL-0) are `ZoneKind`s; every zoned system defers to the one list. Soft-magnet (bias) vs DF-BURROW (forbid).
- **Standing orders** — DF-PRODUCTION PROD-1 (minimal target-pull) ↔ full DF-ORDERS (conditional orders).
- **The Chronicle** — DF-HIST **DESIGNED**: the `record()` capture API (`ChronicleEvent` + importance enums,
  lock like Quality) is the one sink DF-PRODUCTION PROD-4 (economy events), God-Powers §1.2 (divine attribution),
  DF-QUALITY (masterwork/artifact = `Legendary` kind), and nature/weather all emit into. DF-LOG = the HIST-2
  feed slice. **API must land before those emitters harden their event points.**
- **Consumption/mood** — DF-PRODUCTION (produces food) ↔ B7 (Needs decay + eat) ↔ DF-COOK payoff.
- **Zone `purpose` enum** — the canonical 8-kind taxonomy (frameworks §2); every zoned system defers to it.
  (DF-RELIGION's temple = the reserved `religious→faith` kind.)
- **`Needs` schema** — DF-RELIGION REL-1 adds a `worship` field to `comp::bastion::Needs` (B7-owned; lock once
  with the B7 designer, Quality-enum style). Worship is also DF-FOCUS's first "pray" personal-need (co-design).
- **Colony devotion → DP2 faith** — DF-RELIGION S4 (the devotion accumulator) is the interface Divine-Politics
  DP2 consumes (bounded/decaying/per-faction-mappable/deity-attributable). Co-lock so it doesn't fork.
- **Faith Chronicle events** — DF-RELIGION (first temple / prophet arises / temple stood empty / prayer
  answered) emit into DF-HIST's `record()` capture API. Divine-attribution ties God-Powers §1.2 / HIST-5.
