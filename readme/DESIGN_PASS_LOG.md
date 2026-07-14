# Project Bastion — DESIGN_PASS_LOG

The coordination ledger for the **GENERAL DESIGNER** sessions (`readme/GENERAL-DESIGNER-prompt.md`).
Append-only. Parallel designers **claim before designing** (so two sessions don't design the same topic)
and log **DONE** with the doc path + the `[LEDGER]→[DESIGNED]` flip for the architect to action in the
Mega-Prompt queue. Never redo a `DONE` topic. Resume point + full map: `readme/BASTION_DESIGN_STATUS.md`.

Legend: `CLAIMING <topic> · <session/date>` → `DONE <topic> · <doc path> · <one-line> · FLIP <ID> [LEDGER]→[DESIGNED]`.

---

CLAIMING DF-PRODUCTION (DF-WORKSHOP + DF-CHAIN + DF-FARM + DF-COOK cluster) · session 2026-07-09

DONE DF-PRODUCTION · `readme/DF-PRODUCTION-design.md` · industry cluster: Veloren already ships the whole
recipe/station/crafted-good corpus (326 recipes, all workshop sprites, craft execution, `Growth` attr) — net-new
collapses to a produce-job on the B4/B5 board + a standing-order pull (chains emerge) + one real sim (plant
growth); sequenced PROD-0..PROD-5 behind B6, farm partly jumps the queue · FLIP DF-WORKSHOP, DF-CHAIN, DF-FARM,
DF-COOK [LEDGER]→[DESIGNED] (architect to move in the Mega-Prompt queue; also refine ledger costs — WORKSHOP/
CHAIN/COOK are ~$ wire not $$; FARM is the only $$). Surfaced: minimal DF-ORDERS slice (PROD-1) + DF-QUALITY
schema seam (S6) for co-design; asset flip-map → CONTENT-WISHLIST appendix + near-term → ASSET_REQUESTS.md.
Open Qs for Ben logged in the doc §8. Next queue topic per selection criteria: **DF-DIG-VERBS** (stairs/ramps/
channels — adjacent to B5.8/B6, near-frontier) or **DF-RELIGION** (faith-asset batch + Divine Politics feed).

REVIEWED+APPROVED (architect, 2026-07-09): DF-PRODUCTION approved; all 4 open Qs ruled — (1) minimal standing-
order pull in-cluster; (2) growth continuous-when-loaded / discrete-in-rtsim; (3) consumption waits for B7;
(4) LOCK shared Quality enum canonically (purpose-enum style, one authoritative location, co-designed w/
DF-QUALITY/DF-ARTIFACT) — bake into adjacent designs. Mode: CONTINUOUS RUN (churn the queue, report to
architect via these logs). Quality-enum canonicalization to be done in the DF-QUALITY pass.

CLAIMING DF-DIG-VERBS (up/down stairs, ramps, channels, up/down passages — the vertical-mobility mechanism) ·
session 2026-07-09

CLAIMING DF-RELIGION (temples / worship / prophets / religious orders — the faith layer) · session 2026-07-09

CLAIMING DF-HIST (Legends / Chronicle — world memory: browsable history + live event log / scrolling chronicle) · session 2026-07-09

DONE DF-DIG-VERBS · `readme/DF-DIG-VERBS-design.md` · vertical excavation verbs (stairwell/ramp/channel/shaft-
ladder): mostly wiring on B5 — worldgen already ships ramp/staircase/spiral geometry primitives + the Ladder
sprite + the Climb state; net-new is a designation vocabulary + a reachability-safe top-down decomposer (solves
the pit-trap gotcha at the verb level) + the top-down painting UX. KEY UNIFICATION: the ramp verb == B5.8's
auto-carve-steps (one `carve_ramp` library, two callers) — flagged so it isn't built twice. HARD PAIR with B5.8
(traversal): produces geometry B5.8 walks/climbs — sequence B5.8 first or co-build. Zero new animation debt
(all NATIVE). Sub-blocks DIG-0..DIG-4. · FLIP DF-DIG-VERBS [LEDGER]→[DESIGNED] (architect to move). Note:
DF-RELIGION + DF-HIST are CLAIMED by parallel session(s) — skipping per isolation; continuing DOWN the Tier-1
backlog.

CLAIMING DF-QUALITY (+ DF-ARTIFACT apex — quality ladder + strange moods; LOCK the canonical Quality enum per
architect directive) · session 2026-07-09

DONE DF-QUALITY (+ DF-ARTIFACT apex) · `readme/DF-QUALITY-design.md` · LOCKED the canonical quality enum =
engine `common::comp::item::Quality` (Bastion defers, never forks — appended the lock to BASTION-SYSTEM-
FRAMEWORKS.md §2b, purpose-enum style, per architect directive). Net-new collapses to: a skill→quality craft
stamp at produce-job completion (fills DF-PRODUCTION S6 stub) + a per-instance `craft_quality: Option<Quality>`
field + a B-AG3 quality→thought hook + the strange-mood event (DF-ARTIFACT reproduced faithfully: threshold pop,
counter+chance clock, workshop claim, demand-in-order, work-to-exclusion, artifact+legendary-skill OR insane/
death — the failure IS the drama, kept). Mood type reuses B-AG3; only new twist is the god-intervention save
hook (fits pillar). Sub-blocks QUAL-0..QUAL-3. HARD dep DF-PRODUCTION S1; reuses B-AG3 (DONE); seams DF-PREF
(soft), DF-HIST (legend — CLAIMED parallel, coordinate schema). · FLIP DF-QUALITY, DF-ARTIFACT [LEDGER]→
[DESIGNED] (architect to move). Load-bearing schema locked: the Quality enum + the craft_quality override field.

CLAIMING DF-ZONES (typed activity/building zones beyond stockpiles — meeting/pasture/hospital/refuse/water;
rides B5.6b zone schema + the §2 purpose enum) · session 2026-07-09

DONE DF-HIST (+ DF-LOG consolidated) · `readme/DF-HIST-design.md` · the Chronicle/Legends — the world's memory
+ the legibility organ every other system emits into. HEADLINE REUSE: rtsim ALREADY has both halves — the event
medium (`data/report.rs` `Report`/`ReportKind{Death,Theft}` + the `event.rs` bus `OnDeath`/`OnTheft`/… bound by
`rule/report.rs`) AND the world-memory data (`Data{npcs,sites,factions,architect.deaths,quests}`) — but Reports
are built to be FORGOTTEN (decay + capped, NPC gossip) and never reach the client. So DF-HIST is NOT "build an
event system": it's a persistent player-facing sink alongside the ephemeral one (same event → two sinks), the
load-bearing `record()` capture API every system emits into (lock the `ChronicleEvent`+importance enums FIRST,
Quality-enum discipline), + two viewers. Client feed rides `comp/chat.rs` `ChatType`/`ChatMsg` pump. NO 3D, NO
animation (vacuous — no verbs); assets = ~10-15 event glyphs (near-term) + UI-in-code. DF-LOG folds in as the
HIST-2 feed slice. Sub-blocks HIST-0..HIST-6 (v1 = HIST-0..2: API + emitters + live feed = the near-term DF-LOG
win; HIST-3 browser; HIST-4 LOD retention; HIST-5 divine attribution = God-Powers §1.2 seam; HIST-6 ages/
unreliable-accounts DEFERRED). Deps: rtsim bus (ready), B10 (persist Done-when), B9 HUD+sync, NPC name-persist
TODO (npc.rs:447 → load-bearing for the browser). Cross-seams: capture API MUST land before emitters harden
(DF-PRODUCTION PROD-4, God-Powers, nature all declare chronicle entries); `Reports` capping TODO converges on
the Chronicle store; DF-QUALITY masterwork/artifact = a `Legendary` chronicle kind (schema coordinated). · FLIP
DF-HIST, DF-LOG [LEDGER]→[DESIGNED] (architect to move; mark DF-LOG as the HIST-2 slice of DF-HIST). Open Qs for
Ben in doc §8 (store home, importance bands, feed transport, unreliable-accounts scope, browser v1 scope).
Asset: event-glyph batch → ASSET_REQUESTS.md.

DONE DF-RELIGION · `readme/DF-RELIGION-design.md` · the god-game's best-fit topic (you ARE the worshipped god)
splits cleanly in two: a **near-frontier colony tier** that is mostly WIRE — the shipping **tavern gather-loop**
(`rtsim/npc_ai` `go_to_tavern` + the arena-crowd Sit/Cheer block) retargeted at a temple, a **`worship` field on
the existing `Needs`** struct (B7-owned), a **`Priest` arm on `Profession`** (Cultist already exists), a devotion
accumulator (sentiment-shaped), blessings on the existing `BuffKind`s — and a **LATE world-faith tier**
(conversion / rival gods / holy war / festivals) that is NOT DF-RELIGION but the **Divine-Politics DP2–DP4
build**. Guardrail: the Direct tier is deliberately EMPTY (no "make colonist pray" — worship is autonomous like
tavern socializing; the player's whole surface is the divine layer: be-worshipped→favor, sanctify, answer-
prayer). Sequenced REL-0 (buildable `faith` zone — precedes B7) · REL-1 (worship-need + attend, **B7-gated**) ·
REL-2 (prophet/priest + congregation) · REL-3 (devotion aggregate + legibility + rtsim LOD = the DP2 seam) ·
REL-4 (divine seam, B13-gated) · REL-5 (world-faith hand-off, LATE/DP-gated — SEAM SPEC only). Animation =
cheapest in the ledger (v1 NATIVE Sit/Cheer/Talk = the arena crowd reused). Corpus predicts it everywhere
(`religious→faith` zone reserved §2, §3u worship priority, DF-FOCUS pray-need, god-powers sanctify/answer-prayer,
divine-politics §4 "home flock"/DP2). · FLIP DF-RELIGION [LEDGER]→[DESIGNED] (architect to move in the Mega-
Prompt queue; also **split the ledger line** — colony tier is `$` near-frontier behind B7; the `$$$` faith-
politics belongs to Divine-Politics DP2–DP4, NOT DF-RELIGION). Surfaced: worship-need is DF-FOCUS's first "pray"
instance (co-design seam); devotion scalar is the DP2 interface (co-lock like the Quality enum + the DF-HIST
ChronicleEvent enum); faith-asset batch → CONTENT-WISHLIST appendix + near-term (shrine/temple/altar/pews) →
ASSET_REQUESTS.md; faith Chronicle events (first temple / prophet arises / temple stood empty) → DF-HIST
capture API. 5 open Qs for Ben in doc §8 (worship need field vs recreation slot; worship-you pre-DP2; prophet
emergent+appointed; devotion aggregate vs per-colonist; rtsim-AI vs need-job attendance).

DONE DF-ZONES · `readme/DF-ZONES-design.md` · umbrella schema for typed activity/building zones (the §2
activity-zone half made real). Net-new collapses to: locked `ZoneKind` vocabulary (canonicalizes §2, load-
bearing w/ B5.6b-2) + ONE soft-magnet mechanism (a zone raises an activity's utility — bias, never command;
the pillar-critical invariant) + thin per-kind wires (most NEEDS-gated on their behavior system — meeting→
DF-TAVERN, pasture→DF-LIVESTOCK, hospital→DF-MEDICAL, water→B7). v1 (ZONE-0/1) proves the magnet + locks the
vocab + ships the 2 zones whose behavior exists (Refuse rides B6-haul, Gather rides B5). Zero animation debt.
Honest limit stated: a thin schema layer whose per-zone value IS its behavior system. DF-BURROW (hard-
restriction cousin) kept SEPARATE (soft-attract vs hard-forbid — mixing blurs the pillar). Sub-blocks
ZONE-0..ZONE-2+. · FLIP DF-ZONES [LEDGER]→[DESIGNED] (architect to move). Recommend the `ZoneKind` lock land in
frameworks §2 alongside the purpose enum.

CLAIMING DF-BURROW (movement-restriction zones — the hard-policy cousin of DF-ZONES: confine colonists to an
area, e.g. during a siege) · session 2026-07-09

CLAIMING POLICY CLUSTER = DF-ORDERS + DF-STANDING (the standing-order / colony-wide-rule policy layer — the
Manage tier; generalizes DF-PRODUCTION PROD-1's minimal target-pull, does NOT duplicate it). DF-ZONES already
DONE (parallel) so this cluster = the two unclaimed remainder items; consumes the locked `ZoneKind` vocab. ·
session 2026-07-09

CLAIMING DF-TRADE (caravans / merchants / trade depot / haggling — the colony↔world economic seam) ·
session 2026-07-09

DONE DF-TRADE · `readme/DF-TRADE-design.md` · the colony↔world economic seam. HEADLINE REUSE: Veloren ships
almost the whole thing — the two-party atomic **barter engine** (`common::trade` — `TradeAction`/`PendingTrade`/
`SitePrices`/`Good`(15 goods incl. `Coin`+`RoadSecurity`)/`ReducedInventory`), a real **supply/demand economy
sim** (`world::site::economy` — `Economy`/`TradeOrder`/`TradeDelivery`/`simulate_economy`/`INTER_SITE_TRADE`),
**drivable wheeled caravans** (`Body::Cart`/`Body::Carriage` + structure voxels — the vehicle asset is DONE),
and a **roaming merchant AI** (`Profession::Merchant` + rtsim `adventure()`/sell-wares dialogue/`trade_site()`).
KEY INSIGHT: the economy sim **runs at worldgen only then FREEZES** (`civ/mod.rs` seeds it; runtime reads a
static `SitePrices` snapshot) — that frozen-at-gen fact IS the seam DF-TRADE fills. So net-new collapses to: a
**cheap live colony economic node** (one `GoodMap` surplus/deficit vector over DF-PRODUCTION stocks — NPC sites
stay frozen, only the colony lives, LOD-safe, never re-tick the full `Economy` = gotcha #1), a **`ZoneKind::
TradeDepot`** (DF-ZONES + B6-haul), a **two-tier caravan** (§3t: real cart+driver+guard loaded / abstract
`TradeDelivery`-shaped transfer unloaded — same outcome both tiers), an autonomous colony↔caravan clear
(`SitePrices::balance` + Sentiment haggling, NO player UI), the **god tilt** (bless-caravan=Buff / curse-roads=
Hazard-Events banditry + `RoadSecurity`), and the **trade-route map overlay** (§3s). Zero new skeletons — ALL
animation NATIVE (cart-drive=mount/Sit, unload=B6-haul, haggle=merchant Talk). Sub-blocks TRADE-0..TRADE-6
(v1 = TRADE-0..2). Fit-check FITS with a hard guardrail: the god **tilts** flows, never runs trades — a
player-operated price/dispatch/fleet screen = 4X econ-micro = AVOID; the existing `hud::trade.rs` is the
embodied-avatar window, orthogonal, the god never touches it. SCOPE SPLIT (DF-RELIGION-style): near-term COLONY
tier designed here; the WORLD faction trade-network + trade-war = **Divine-Politics DP1/DP3/DP5**, spec-only
(TRADE-6). · FLIP DF-TRADE [LEDGER]→[DESIGNED] (architect to move in the Mega-Prompt queue). HONEST LIMIT:
strong reuse but **sits on unbuilt substrate** (DF-PRODUCTION + founding + DF-ZONES + B6) — DESIGNED-downstream,
NOT near-frontier-now; sequence after the production cluster. Surfaced: colony-node `SitePrices`/surplus vector
= the interface Divine-Politics DP consumes (co-lock like the DF-RELIGION devotion scalar / Quality enum);
God-Powers catalog rows *Bless a caravan* / *Curse the roads* get a **colony-scoped near-term slice** (TRADE-4)
split out of their DP-gated world-route version (update the 2 catalog rows — scope refinement, not a redefinition,
mirrors the DF-RELIGION colony-vs-DP faith split); depot structure + trade-UI glyphs → ASSET_REQUESTS; trade
chronicle glyphs → DF-HIST glyph batch (not forked). 6 open Qs for Ben in doc §10 (colony-node weight; NPC-site
economies frozen vs ticked; caravan cadence; coin vs barter-first; Manage tier in v1; god-tilt timing).

DONE DF-BURROW · `readme/DF-BURROW-design.md` · movement-restriction zones (hard-policy cousin of DF-ZONES).
Smallest policy pass: two filters on B4 (job-claim: confined colonists only claim in-burrow jobs — NOT claimed-
then-cancelled, so no DF "item inaccessible" spam) + an idle-movement clamp; the real work is the DESIGN, not
code. PILLAR-CRITICAL reframe: a burrow is the god's "Call to Shelter" directive (colony responds, not per-unit
command) WITH a survival escape valve — critical hunger/thirst breaks confinement (the DF safe-room-death-trap
designed OUT, not reproduced). Modes: standing restriction (v1) + B8-triggered shelter alert (siege, the
marquee use, gated B8). Survival valve gated B7 — flag: never ship confinement without at least a stub override
or it's a starvation box. Legibility first-class (who's confined / stuck-outside / breaking-out). Zero animation
debt. Sub-blocks BURROW-0..BURROW-3. · FLIP DF-BURROW [LEDGER]→[DESIGNED] (architect to move). Completes the
zone-policy PAIR (ZONES soft-attract + BURROW hard-restrict — kept separate by design, a pillar guardrail).

DONE POLICY CLUSTER = DF-ORDERS + DF-STANDING · `readme/DF-POLICY-design.md` · the colony *policy* layer (the
Manage tier for quotas & rules — sibling to DF-ZONES' Manage tier for places, DF-BURROW's hard-restrict). Both
halves UNIFY into ONE `ColonyPolicy` layer = policy at two grains: an ORDER is a quota rule ("keep ≥N" → engine
emits work to close the gap), a STANDING RULE is a behaviour toggle (forbid-on-death, auto-haul-refuse).
RECONCILED with DF-PRODUCTION PROD-1 (does NOT duplicate): PROD-1's minimal production-pull is the FIRST INSTANCE
of this pass's generalized order-evaluation engine — this pass generalizes it (any verb not just recipes + a
conditional gate + a manager panel), owns the STANDING-RULE half PROD-1 never touched, and adds the ONE real
net-new substrate: a colony STOCK CENSUS (count item-kind across `BastionPile`s) — SHARED with DF-PRODUCTION
PROD-4's economy readout (build once, pays twice). Reuse-heavy: rides the job board (`bastion_jobs`/`JobBoard`/
`WorkType`/`WorkPriorities`), locked `ZoneKind` (order scope), DF-HIST `record()` (chronicle). ZERO new 3D
assets, ZERO new animations (UI-in-code, like DF-HIST). Sub-blocks POL-0 (ColonyPolicy + census + generalized
order engine + THE PILLAR GUARDRAIL: zero-policy = complete healthy game, asserted) · POL-1 (standing rules +
manager/policy panel + READY rules) · ORDER-2 (conditional orders + repeat + zone-scope, enrichment) · POL-3
(rtsim aggregate policy LOD + chronicle, enrichment) · POL-4+ (per-rule flips, delegation: cook-perm←DF-COOK
PROD-3, rot←DF-ROT, butcher←DF-LIVESTOCK). GUARDRAIL held: ledger's #1 4X-drift risk — design is a held line
(policy = optional thermostat, never mandatory; autonomous stays default; the guardrail IS POL-0's load-bearing
Done-when). · FLIP DF-ORDERS, DF-STANDING [LEDGER]→[DESIGNED] (architect to move in the Mega-Prompt queue).
CROSS-SEAM FLAGGED HARD: PROD-1 ↔ POL-0 must build the order engine ONCE (co-build, or PROD-1 writes its pull as
the first instance of the general engine) — else it forks (doc §2 + §7-Q1). Shared stock census (S3) ↔ PROD-4 —
recommend architect note the leverage (don't cost it twice). 4 open Qs for Ben in doc §7 (PROD-1/POL-0 code home;
policy owner = overseer vs bookkeeper/DF-BOOKS seam; conditional richness in v1; default rule states). Asset:
UI-only note → ASSET_REQUESTS.md + wishlist. Completes the Manage tier trio: DF-ZONES (places, soft-magnet) +
DF-BURROW (places, hard-restrict) + DF-POLICY (quotas & rules) — three distinct mechanisms, one influence-not-
command tier.

CLAIMING DF-TAVERN (drinking / socializing / performers / visitors — the colony social-life system; source of
the gather-loop DF-RELIGION worship + DF-ZONES Meeting reuse; rides B7 recreation-need) · session 2026-07-09

=== SESSION PAUSE (this designer, 2026-07-09) — clean stop, resume point set ===
This session DESIGNED (5): DF-PRODUCTION, DF-DIG-VERBS, DF-QUALITY(+DF-ARTIFACT), DF-ZONES, DF-BURROW. Parallel
session(s) DESIGNED: DF-HIST(+DF-LOG), DF-RELIGION. Two schema LOCKs landed (Quality → frameworks §2b; ZoneKind
→ §2 recommended). Stopping cleanly on context budget, NOT on blocker — ready backlog is NOT exhausted.
NEXT UNCLAIMED near-frontier Tier-1/2 (verify no live CLAIM first): DF-ORDERS (PROD-1 seeds it), DF-TAVERN
(unblocks DF-ZONES Meeting wire + DF-RELIGION gather loop), DF-WOUND, DF-MECH/TRAP/OPERABLE (trigger→link→
effect cluster), DF-TRADE, DF-CAVERN+DF-GEOLOGY. Architect: re-fire or run parallels from here.

## FORK RESOLUTIONS (Ben, 2026-07-09) — architect to fold into the named designs
- **Religion (#1):** colonists worship YOU directly pre-DP2 (no deity model needed yet). Rival gods that
  CONTEST FOLLOWERS (Black & White–style — fight over each other's flocks) ARE wanted, but LAST — they need
  rival-god AI. This is already designed as **Divine-Politics DP4** (+ Dominions dominion-spread, frameworks §8);
  keep it Tier-3 late, build only after colony faith (DF-RELIGION colony tier) is proven. No new pass now.
- **Trade (#2):** architect's call → **barter-first, coin as the remainder** (rides Veloren's existing barter
  engine; coin emerges, not tracked-first). Fold into DF-TRADE.
- **Policy (#3):** **BOTH** — an in-fiction **bookkeeper/prophet steward** manages colony policy autonomously
  AND the god/overseer can set it directly (the control-spectrum: autonomous ↔ manage; "god ≠ ruler"). Fold
  into DF-POLICY (the steward is the autonomous default; direct-set is the manage tier).
- **Quality (#4):** masterwork = **DF-faithful capped luck-roll** (earned-vs-lucky drama is the point; pairs
  with artifact-or-death). Fold into DF-QUALITY.

CLAIMING DF-CAVERN + DF-GEOLOGY (layered caverns / danger tiers / underground biomes + ore veins / stone layers
/ gems exposed to mining — the vertical world; complements DF-DIG-VERBS + mining framework §6 Breach Event) ·
session 2026-07-09

DONE DF-CAVERN + DF-GEOLOGY · `readme/DF-CAVERN-GEOLOGY-design.md` · the vertical world + what's in the rock.
HEADLINE REUSE: Veloren's underground is MORE built-out than DF's 3 caverns — `world/src/layer/cave.rs` ships
`LAYERS=5` depth-layered tunnel-connected cave levels, a per-depth+temp `Biome`/`biome_at`, and per-biome
`apply_entity_spawns` (cavern life at gen); ores/gems are mineable sprites w/ item-drops + per-ore hardness
(`SpriteKind::{Coal,Iron,Velorite,Lodestone,Bloodstone,...}` → `common.items.mineral.ore.*`). So net-new
collapses to: legible DANGER TIERS over the 5 layers (3 named tiers mapped on) + PROSPECTING/colony-knowledge
(discovered≠omniscient — overlay shows only surveyed/exposed veins, the pillar-and-tension core) + depth-graded
ore distribution (delve-deeper-for-better, feeds DF-PRODUCTION) + the flagship BREACH EVENT (dig completes
adjacent to a cavern void → ONE hazard event: open path, threats spill, alert+chronicle, B8 responds) + danger-
scaled cavern life (reuse skeletons=NATIVE anim). Sub-blocks CAVERN-0 (tiers+overlay) · GEO-1 (prospecting+
graded ore — the v1 buildable-now pair) · CAVERN-2 (Breach, the flagship) · CAVERN-3/4 (life/LOD+god).
Near-zero animation debt. Completes the mining-framework §6 line w/ DF-DIG-VERBS (verbs → what they dig into →
autonomous mine mode composes them). · FLIP DF-CAVERN, DF-GEOLOGY [LEDGER]→[DESIGNED] (architect to move).
HONEST LIMIT (DF-TRADE-style DESIGNED-downstream): CAVERN-0/GEO-1 buildable on shipped worldgen NOW, but the
flagship BREACH sits on THREE unbuilt systems — the **hazard-event engine (frameworks §1a, NOT started — grep
confirms no server hazard system)**, B8 defense, DF-DIG-VERBS (designed this session). Breach = one hazard kind,
build as an instance of the hazard engine (build-once, not bespoke). DF-MAGMA deliberately DEFERRED (needs
DF-FLUID). Chronicle seam: breach/first-vein/deepest-tier → DF-HIST `record()`. Danger-tolerance = a DF-POLICY
standing rule (reuses this session's policy layer). 4 open Qs for Ben in doc §6 (tier granularity 3-vs-5;
prospect grain; breach trigger scope incl. autonomous digs; DF-MAGMA deferral confirm).

=== SESSION PAUSE (this designer, 2026-07-09, resumed run) — clean stop, resume point set ===
Resumed continuous-churn and added DF-CAVERN+DF-GEOLOGY (6th pass by this designer this session). Stopping
cleanly on context depth (NOT a blocker) to hold quality — the next top pick, DF-MECH/TRAP/OPERABLE, is a
build-once trigger→link→effect engine that is "build-not-wrap" (Veloren has no trap system per ledger) and
deserves a proper fresh survey, not a deep-context rushed pass.
THIS DESIGNER'S 6 PASSES: DF-PRODUCTION, DF-DIG-VERBS, DF-QUALITY(+ARTIFACT), DF-ZONES, DF-BURROW,
DF-CAVERN+DF-GEOLOGY. Parallel DONE: DF-HIST(+LOG), DF-RELIGION, DF-TRADE, DF-POLICY(ORDERS+STANDING). In
flight: DF-TAVERN, DF-WOUND.
NEXT UNCLAIMED near-frontier substrate (verify no live CLAIM first): DF-MECH/TRAP/OPERABLE (top pick — the
trigger→link→effect build-once engine; rides B8; build-not-wrap) · DF-LIVESTOCK (strong substrate: Veloren
taming/pets; wires DF-ZONES pasture + DF-FARM + DF-COOK butchery) · DF-MIGRATION (rtsim `rule/migrate.rs`
substrate; ties DF-PRESTIGE) · DF-FOCUS (Tier-1, makes minds matter; rides B7+B-AG3; ledger fact-checked).
DEFER Tier-3 epics (VILLAIN/NIGHT/BEAST/KNOWLEDGE/ECON/etc.) + DF-MEDICAL until DF-WOUND lands + DF-MAGMA until
DF-FLUID. Architect: re-fire or run parallels from here.

=== FINDING for the ARCHITECT (this designer, 2026-07-09) — future-work-and-deferred-ideas.md holds a LOT of already-done design ===
Ben flagged it; verified. `future-work-and-deferred-ideas.md` (§1 build-once engines + §3a–§3z researched
systems + §5 design-pass debts) already contains substantial DESIGN for things the DF-* passes are treating as
undesigned or as bare "unbuilt dependencies." A pass should CONSUME these, not re-derive; and two are load-
bearing shared ENGINES that deserve their own consolidation-pass→block, not reinvention per-caller.

CROSS-REFERENCE (future-work § → topic → status → action):
- **§1a Hazard-Events engine** (location+radius+effect → B-AG3/B8 consume; timber/flood/lava/rockfall/explosion
  are all costumes) → THIS IS the "unbuilt hazard engine" my DF-CAVERN **Breach** flagged, AND the effect side
  of DF-TRAP, AND DF-FLUID's flood. It is DESIGNED here but has NO block. **ACTION: promote §1a to its own
  consolidation-pass → buildable block** — it's a shared dependency of ≥4 things, currently only ever
  referenced, never owned. Highest-leverage finding.
- **§1b Trigger→Link→Effect = DF-MECH/TRAP/OPERABLE** (my #1 unclaimed pick) → substantially PRE-DESIGNED here
  (build-once insight + god-game reframe + build-not-wrap note) + ledger §E. **ACTION: queue DF-MECH as a
  CONSOLIDATION pass (fast — §1b + §E → add Done-whens), NOT from-scratch.** I did not re-derive it; whoever
  takes it should start from §1b.
- **§1c Staged voxel removal** → a B5 build-note (progressive chop/mine over the work-tick); ties DF-DIG-VERBS.
- **§3c Autonomous building** (agents build from a TEMPLATE CATALOG keyed by race/culture, reusing Veloren site
  templates) → the construction model for **DF-WORKSHOP autonomous build + B-AG6 settlement growth**. Pre-
  designed; a pass should consume it.
- **§3y Nature & environment sim** (cited: Veloren ships weather-grid + temp-spawning + chunk-resource-tracking
  + repop-queues) → **§3y.A wildlife lifecycle (birth/death/carrying-capacity/breeding/predator-prey + the UO
  closed-ecology lesson) IS DF-LIVESTOCK's population model already.** Also feeds DF-TEMP, DF-BIOME-FX, DF-FARM
  crop lifecycle. **ACTION: DF-LIVESTOCK/DF-TEMP passes CONSOLIDATE §3y, don't re-derive.**
- **§3v Mining framework** → already CONSUMED by my DF-DIG-VERBS + DF-CAVERN passes (✓ good — this is the
  pattern the others should follow).
- **§3z Materials/textures framework** → the material axis of DF-QUALITY (quality = material × craftsmanship);
  a DF-QUALITY material follow-up or DF-GEOLOGY should reference it.
- **§3r Custom-creature capability** → DF-LIVESTOCK (husbandry species) + DF-BEAST + the god-companion body.
- **§3n Embark + §3q Control-spectrum + §3u Action-animations** → already designed/consumed (FOUNDING doc;
  frameworks §1; the §3u anim line-items I cited across passes). ✓
- **Divine-Politics DP1–DP5 (divine-politics-bible, per §5)** → **FULLY DESIGNED already.** The world-faith
  tier (DF-RELIGION deferred to "DP2–DP4") + world-trade-network (DF-TRADE deferred to "DP1/DP3/DP5") need NO
  new design pass — they need block-ification when substrate exists. **ACTION: don't queue them as undesigned.**
- **§5 Design-pass debts** = a ready TODO list. Cross-ref vs this log: still-undesigned near-frontier from its
  own Tier-1 list = **DF-ROOMS** + **DF-FOCUS** (neither passed yet; both near-frontier — DF-ROOMS ties DF-QUALITY
  room-value + B-AG3, DF-FOCUS is the personality→needs→work loop, rides B7+B-AG3).

HOUSEKEEPING RECO: the doc's own closing line says "move items OUT as they get a real design pass," but designed
items (mining framework→DF-DIG-VERBS/CAVERN; 3D-zones→DF-ZONES; embark→FOUNDING; control-spectrum→frameworks
§1) were never moved/annotated. Recommend the architect (or a bookkeeping pass) reconcile future-work against
this log + BASTION_DESIGN_STATUS so future-work stops drifting into a stale duplicate of designed work — mark
each §-section CONSUMED-BY <topic> / STILL-OPEN. NET: the design frontier is FURTHER ALONG than the DF-* ledger
alone implies — a good chunk of "undesigned" work is pre-designed in future-work + the bibles; the remaining
real gaps are the two shared ENGINES (§1a hazard, §1b DF-MECH) + DF-ROOMS/DF-FOCUS + the B7/B8/hazard substrate
the deferred halves wait on.

=== NOTE for the ARCHITECT (this designer, 2026-07-09) — free range on suggestions + a new suggestion board ===
Ben granted this designer **free range on game-design suggestions** (in the context of the game design). New
standing doc: **`readme/DESIGNER-SUGGESTIONS.md`** — a living board for cross-cutting ideas / build-order
leverage / sparks that emerge FROM the passes but don't belong in a single `<TOPIC>-design.md`. These are
SUGGESTIONS (architect/Ben decide what becomes a pass or block), pillar-grounded, append-only. Seeded from the
first 6 passes + the future-work finding. Headlines already there for your review:
(1) sequence the build queue by *designed-value-unblocked* — **B7-Needs is the #1 keystone** (gates the payoff
half of ≥6 designed systems), **§1a hazard engine #2** (designed-but-blockless; unblocks Breach/traps/flood),
**B6 #3**; (2) a **shared-substrate registry** (order engine / carve_ramp / hazard engine / stock census /
overlay renderer / ChronicleEvent API / Quality / ZoneKind / devotion scalar) to stop forks; (3) **legibility as
a platform** — add "overlay layer?" + "chronicle event?" to the per-system checklist like the no-T-pose rule;
(4) **design DF-ROOMS + DF-FOCUS next** — the two undesigned near-frontier Tier-1s that close the mind-payoff
loop; (5) a build-order dependency sketch; (6) housekeeping (reconcile future-work; promote the design template;
track DESIGNED-now vs DESIGNED-downstream). I'll keep appending sparks to §7 as future passes generate them.

=== GAP SWEEP + ASSET PASS (this designer, 2026-07-09) — for the ARCHITECT ===
Ben asked for a gap sweep across the docs + a proper asset pass (rich, lore-grounded requests, not boring/samey).

GAPS FOUND + CLOSED:
- **ASSET GAP (the big one): DF-DIG-VERBS + DF-CAVERN-GEOLOGY surfaced asset needs in their design docs but had
  ZERO entries on the request board.** CLOSED — wrote the **"THE MINE & THE DEEP DARK" batch** to ASSET_REQUESTS
  (pit-head cribbing, deep-shaft headframe/windlass, culture ladder variants, per-mineral depth-graded ore-vein
  sprites [Velorite+Bloodstone first — they carry the depth-danger + glow read], prospecting claim-cairns,
  depth-tiered cave-flora, menace-by-depth cavern-life variants, the Breach maw). ONE coherent batch across
  DIG-VERBS + CAVERN + mining-framework §6 (not scattered). **Frontier-timed:** B5.8 (DF-DIG-VERBS' hard-pair)
  is the ACTIVE build lane now, so this batch is near-term real, not speculative.
- **ASSET GAP: my earlier requests were TERSE** (bare nouns — "seed icons", "farm dressing"). Per Ben, every
  request now needs a **creative brief + a lore seed** so assets don't come out generic. CLOSED — enriched the
  thin ones (seed icons → per-crop character; farm dressing → + a scarecrow in the dead's coat) and **codified
  the REQUEST-WRITING RULE in the ASSET_REQUESTS header** (creative brief hooking the §5 tonal ramps + §7 culture
  theming + a variation axis; lore seed in the world's folk-craft voice). Also added the small missing props:
  DF-ZONES per-purpose zone-marker posts, DF-BURROW shelter muster-post/alarm-bell, DF-QUALITY masterwork/
  artifact ornate-item treatment. **After this pass, every DESIGNED topic's asset needs are on the board.**
- **DESIGN COVERAGE GAPS (unchanged, restated):** the two undesigned near-frontier Tier-1s are **DF-ROOMS** +
  **DF-FOCUS** (the mind-payoff loop); the two designed-but-BLOCKLESS shared engines are **§1a Hazard-Events**
  (my Breach + DF-TRAP + DF-FLUID all need it) and **§1b DF-MECH/TRAP/OPERABLE** (top unclaimed — consolidation
  pass from §1b+ledger §E). See the earlier future-work FINDING for the consume-don't-re-derive map.
- **No contradiction gaps found** in the design corpus this sweep — the passes are internally consistent and the
  shared schemas (Quality/ZoneKind/purpose/ChronicleEvent/devotion) are locked/flagged. The one housekeeping
  drift remains: future-work isn't annotated CONSUMED-BY (recommended earlier).

NET: the ASSET pipeline now has a full, characterful demand map for all designed topics (pilot has already
FULFILLED most workshop/temple/depot/crop/glyph requests — the mine batch is the next real fill, timed to B5.8);
the remaining DESIGN gaps are DF-ROOMS/DF-FOCUS + the two shared engines.

=== ASSETS → ARCHITECT, FOR ROUTING TO THE ASSET CREATOR (this designer, 2026-07-09) ===
Per Ben: new asset requests go to the ARCHITECT to route to the asset creator (not pulled silently). These are
staged in `readme/ASSET_REQUESTS.md` with full creative brief + lore seed each — **architect: route to the asset
creator.** New this session (all frontier-timed or gap-fills):
- **"THE MINE & THE DEEP DARK" batch** (DF-DIG-VERBS + DF-CAVERN-GEOLOGY) — NEAR-TERM (B5.8 is the active build
  lane): pit-head cribbing · deep-shaft headframe/windlass · culture ladder variants · per-mineral depth-graded
  ore-vein sprites (**Velorite + Bloodstone FIRST** — carry the depth-danger + glow read) · prospecting claim-
  cairns · depth-tiered cave-flora (shallow+deep exemplars) · menace-by-depth cavern-life variants · the Breach
  maw. Spec-placeholder-first per item (generate one, not forty).
- **Small props:** DF-ZONES per-purpose zone-marker posts (meeting totem + refuse stake first) · DF-BURROW
  shelter muster-post/alarm-bell · DF-QUALITY masterwork/artifact ornate-item treatment.
- **Enriched (were terse):** DF-PRODUCTION seed icons (per-crop character) + farm dressing (+ scarecrow).
Priority for routing: the MINE batch's spec-placeholders (Velorite ore-vein + human pit-head + shallow/deep
cave-flora) — they pressure-test the DF-DIG-VERBS/DF-CAVERN visual spec while B5.8 is live. Everything else is
NEEDS-gated; route as its system nears.
STANDING: I'll post an "ASSETS → ARCHITECT" hand-off like this whenever a pass adds requests, so you route
rather than the board being pulled blind. (Reminder: SendMessage can't reach you cross-session — this log is the
channel; say the word if you want a different hand-off mechanism.)

CLAIMING DF-ROOMS (rooms & room-quality from furniture — a walled space + bed = bedroom; furniture quality →
room value → good/bad thoughts; the bridge from construction to the mind. Ties DF-QUALITY + B-AG3.) ·
session 2026-07-09

DONE DF-ROOMS · `readme/DF-ROOMS-design.md` · rooms & room-quality — the bridge from construction to the mind.
HEAVY reuse: furniture ships (the `Furniture` sprite category — Chair/Table/Bed/Bookshelf/Wardrobe/Bench +
culture variants); the payoff pipeline B-AG3 is DONE; the enclosure algorithm is proven in the asset-lab
(`castle_verify.py` BFS). Net-new collapses to: room DETECTION (BFS enclosure over built walls, event-driven,
colony-scoped) + role CLASSIFICATION (from contained furniture: bed→bedroom, table+chairs→dining, altar→temple,
throne→hall) + the 4-stat IMPRESSIVENESS model (Wealth=DF-QUALITY furniture craft-quality · Beauty=décor ·
Space=footprint · Cleanliness=DF-ROT) with the RimWorld WEAKEST-STAT-DOMINATES rule (the whole legibility +
anti-gaming design — §0) + ownership + a room→B-AG3 thought hook (role×impressiveness×personality). Fit=PASS
(passive-consequence: colonists build/furnish/claim autonomously; the god provides materials/blessing, NEVER
places a rug). Sub-blocks ROOM-0 (detect+classify) · ROOM-1 (impressiveness+thought — the payoff) · ROOM-2
(ownership+inspector) · ROOM-3 (rtsim LOD + Consecrate god-aura + shared detection, enrichment). ZERO animation
debt. · FLIP DF-ROOMS [LEDGER]→[DESIGNED] (architect to move). SEAMS: **first consumer of DF-QUALITY** (room
Wealth = Σ furniture `craft_quality` — enum already locked, no fork); **shared enclosure detection** flagged
(DF-ZONES building-zones + DF-PRODUCTION workshop + DF-RELIGION temple all read "is this an enclosed room of
role X?" — build ONCE, DESIGNER-SUGGESTIONS §2); Cleanliness soft-gated on DF-ROT (degrades gracefully). Closes
a leg of the mind-payoff loop (rooms→mood + quality→mood + B-AG3); DF-FOCUS is the remaining leg. ASSET: the
"MAKE A ROOM IMPRESSIVE" décor batch → ASSET_REQUESTS (wall art/tapestries, ancestor statue, hearth, role-
centerpieces throne/four-post-bed/grand-table, small-comfort clutter, quality-tiered furniture variants) — full
creative briefs + lore seeds; the Beauty stat's ceiling depends on this batch. 4 open Qs for Ben in doc §6
(one enclosure pass vs two; Space curve; ownership grain; personality weighting).

CLAIMING DF-FOCUS (personal needs derived from personality facets — pray/family/craft/admire-art/see-animals/
drink; met:unmet ratio = FOCUS = ±work speed/quality, separate from mood; colonists self-generate need-jobs.
The loop that makes minds MATTER mechanically + the unifying consumer of temple/tavern/zoo/art/family venues.
Extends B-AG3 + B7.) · session 2026-07-09

DONE DF-FOCUS · `readme/DF-FOCUS-design.md` · personal needs → focus → work performance — the loop that makes
minds MATTER. CONSOLIDATION pass: the mechanic is ALREADY designed in **agency-bible §5b.2** ("Needs — two
kinds" + "FOCUS added") — this grounds it in the repo (B-AG3 DONE = the source; B7 Needs = the substrate; the
venues designed this session = the satisfiers), locks the interface, and decomposes it buildable. Net-new
collapses to: the personal-needs model + **a LOCKED `Need` enum** (Pray/Socialize/Drink/Craft/Family/SeeAnimals/
AdmireArt/… derived from facets) + the FOCUS scalar (met:unmet ratio → ±50% work speed/quality, SEPARATE from
mood — the DF texture) + self-generated need-jobs (low-prio yields to work, high-prio "Pray!" doesn't) + the
focus→work_rate hook + the `satisfies(Need)` venue binding (build-once). THE KEY DELIVERABLE = the locked `Need`
vocabulary: every venue system is a thin `satisfies(Need)` plug-in (temple→Pray, tavern→Drink/Socialize,
pasture→SeeAnimals, art→AdmireArt, kin→Family, workshop→Craft) — DF-FOCUS is WHY the whole venue corpus matters.
Fit=PASS (purest indirect-influence system: colonists self-pursue needs; the god only PROVIDES venues + blesses,
never orders — Answer-a-Prayer miracle ties God-Powers/DF-RELIGION). Sub-blocks FOCUS-0 (needs model + ENUM LOCK
+ derive-from-facets — load-bearing NOW, not B7-gated) · FOCUS-1 (self-need-jobs + venue satisfaction) · FOCUS-2
(focus scalar + ±50% work hook — B7-gated) · FOCUS-3 (legibility/LOD + facet-belief distress). ZERO new asset
requests (deliberate — consumes the venue batches; DF-FOCUS is their reason, not new demand) + near-zero
animation debt. · FLIP DF-FOCUS [LEDGER]→[DESIGNED] (architect to move). SEAMS: **fold DF-RELIGION's `worship`
need into this `Need` enum** (the seam DF-RELIGION flagged — co-lock, one vocab not a Needs-field-per-venue);
`Need` enum = new locked shared schema (DESIGNER-SUGGESTIONS §2). **CLOSES the mind-payoff loop** (DESIGNER-
SUGGESTIONS §4): DF-ROOMS + DF-QUALITY + DF-FOCUS + B-AG3 = "what the colony builds/makes/provides → how each
soul feels AND performs" — the loop is now FULLY DESIGNED (pending B7 as shared substrate). HONEST LIMIT:
DESIGNED-downstream on B7 for the lived loop (the ±50% needs decay); the ENUM LOCK is buildable now. 4 open Qs
for Ben in doc §6 (worship-need→Need::Pray fold; focus magnitude; v1 need count; facet-belief distress).

## SCHEMA LOCKS + KEYSTONE (architect, 2026-07-09 — from DF-FOCUS pass)
- **`Need` enum co-lock:** DF-FOCUS locks a shared `Need` enum (Pray/Socialize/Craft/AdmireArt/…); every venue is a thin `satisfies(Need)` plug-in. **DF-RELIGION's `worship` need FOLDS IN as `Need::Pray`** — co-lock, do NOT fork. This is the canonical needs vocabulary; other systems defer.
- **B7 is the #1 KEYSTONE (queue weight):** DF-COOK, DF-QUALITY-thought, DF-RELIGION-worship, DF-BURROW-valve, DF-ROOMS-cleanliness, and DF-FOCUS's whole payoff are ALL inert until B7 (needs/mood/idle-AI) lands. B7 stays weighted HIGH — right after B6 in the systems lane (dependency: B7's eat-from-stockpile needs B6). Building B7 lights up the entire mind-payoff loop at once.
- **Mind-payoff loop = FULLY DESIGNED** (ROOMS+QUALITY+FOCUS+B-AG3), pending B7 as the shared decay substrate.

## DESIGN INPUT FROM BEN (2026-07-09) — AUTONOMOUS MINING ZONES (later; for the designer to enrich the §3v mining framework)
The colony should AUTO-PLAN + AUTO-DIG mine zones (not just player-painted). Frame around TWO MINE PURPOSES:
1. **Resource / traditional cave mine** — structured excavation for ore/stone (adit → shaft → galleries → branches → per-level access), colony surveys ore → plans → digs. STRUCTURE (timber supports, headframes, cribbing) added later — good asset/visual layer.
2. **Adventure / access mine** — a mine whose PURPOSE is to reach a known/suspected CAVE or DUNGEON to delve it (exploration/military/loot), NOT ore. Dig directed toward the target; ties the prospecting/spelunking half + the BREACH EVENT + dungeon delving + the mortal-RPG adventure content.
Both ride: §3v mining framework (autonomous Mode 3), DF-DIG-VERBS (the verbs), DF-CAVERN-GEOLOGY (what's down there), B5.8 (access/auto-stairs-ladders), the ore-survey sampler. LATER (needs those first) — but a rich seam for the designer to spin more from (mine templates, prospecting/knowledge model, breach hazards, delve objectives, structured-mine assets). Designer: fold into a mining-framework design pass / extend DIG-VERBS + CAVERN.

## DESIGN INPUT FROM BEN (2026-07-09) — ZONE-MANAGEMENT UI (from the b-2 live test; designer to design the coherent model behind these, feeding DF-ZONES / the B5.6b zone-UI)
Ben's b-2 test surfaced zone-placement/editing needs that are DESIGN-level, not just bug fixes — design the coherent model so the builder (b-2.1/b-3) builds against a spec, not ad-hoc patches:
- **Zone IDENTITY + SELECTION:** a zone is a selectable object; select ONE zone, and edit/delete act on THAT zone only (right-click Delete currently nukes every overlapping zone under the cursor — wrong; need per-zone identity, topmost/selected resolution).
- **Resize in X/Y AND Z:** edit a placed zone's footprint AND its depth (drag handles / numeric), not just footprint.
- **DEPTH MODES:** surface-relative (current, N-down-per-column) + **flat-floor** (dig to a shared level → square/flat bottom, for foundations/quarry floors) + **max-depth** (to bedrock/deepest reachable). One coherent depth model with a mode selector.
- **Placement UX:** cancel an in-progress drag (Esc/right-click, nothing committed); depth stepper 1-click=1 level, Shift+click=5.
Pairs with the AUTONOMOUS MINING ZONES input above — together they're "how the player + colony define, edit, and dig volumes." Designer: fold into DF-ZONES / a zone-management-UI design pass; the pure-bug half (ghost outline rebuild, overlapping-delete) is already routed to the builder.

CLAIMING BUILD-FRAMEWORK (the Construction Framework — advanced/autonomous building: template-catalog tiers,
complex/multi-level + VERTICAL construction build-UP, placement types, site-prep §3x; UNIFIES additive-build +
subtractive-mine under one engine; folds in Ben's AUTONOMOUS-MINING-ZONES + ZONE-MANAGEMENT-UI inputs +
enriches future-work §3c). Architect-directed. · session 2026-07-09

DONE BUILD-FRAMEWORK · `readme/BUILD-FRAMEWORK-design.md` · the Construction Framework (architect-directed;
sibling to the mining framework). THE UNIFYING INSIGHT: **building UP (additive) and mining DOWN (subtractive)
are ONE engine** — place a templated VOLUME → PREP the site (cut/fill) → colonists work-tick it into being →
materialize; the only difference is the sign of the work. One template catalog, one placement solver, one VOLUME
model, one work-tick (B5), one access story (DF-DIG-VERBS + B5.8 climb — for both the top of a tower and the
bottom of a shaft). HEAVY reuse (the §3c open unknown resolves FAVORABLY): structure prefabs are RUNTIME-
ADDRESSABLE (`structure.rs` `StructuresGroup::load_group(specifier)` loads .vox by specifier) + the asset-lab
prefab pipeline already produces catalog entries + race-keyed worldgen sites + B5 Build/Mine + B1.8 sampler +
B5.6b volume paint + ClimbSkill (colonists improve at vertical work). Net-new = the place→prep→work SPINE +
placement solver (POINT/AREA/PATH/LINE-ENCLOSE + VOLUME-DOWN) + site-prep planner (§3x cut/fill/terrace) + the
VOLUME model + depth modes + VERTICAL/multi-level construction (build UP: floor-by-floor from a work platform,
the inverted pit-trap solved by the SHARED reachability decomposer w/ DF-DIG-VERBS — build once, two signs) +
catalog tiers (fixed→parameterized→composed piece-pools) + autonomous building (B-AG6) + autonomous mining zones.
CONSUMES future-work §3c (autonomous building) + §3x (site-prep + roads) wholesale; ties mining framework §6.
FOLDS IN both Ben inputs: AUTONOMOUS MINING ZONES (resource mine = structured negative-space building; adventure
mine = dig toward a cave/dungeon to delve, ties Breach) → §8/BUILD-6; ZONE-MANAGEMENT UI (zone identity/selection
+ X/Y/Z edit + depth modes surface-relative/flat-floor/max-depth + cancel-drag/stepper) → §7/BUILD-3. Sub-blocks
BUILD-0 (spine+site-prep+fixed-catalog) · BUILD-1 (placement types+walls+roads) · BUILD-2 (build UP — the
generative centerpiece, gated on DF-DIG-VERBS+B5.8) · BUILD-3 (volume UI+depth modes — Ben input 2, NEAR-FRONTIER
via B5.6b, arguably build FIRST) · BUILD-4 (parameterized) · BUILD-5 (composed piece-pools) · BUILD-6 (autonomous
build+mine). · FLIP: enriches §3c + §3x into DESIGNED; folds the 2 Ben inputs; recommend recording the
build+mine unification in FRAMEWORKS.md (two faces of one engine). SHARED SCHEMA: the VOLUME model + depth-mode
enum (lock like Quality/ZoneKind/Need); the reachability-safe decomposer shared with DF-DIG-VERBS (build once).
ASSET: the "COMPOSED STRUCTURE PIECE-POOLS" batch → ASSET_REQUESTS (per-race wall segments / floor-decks w/
stair-openings / roof caps / foundation-terraces / doors-windows / road surfaces — full briefs + lore; the big
tier-3 asset ask). Animation: no NEW debt (build-hammer already owed by DF-PRODUCTION; mine/haul/climb NATIVE).
5 open Qs for Ben in doc §12 (scaffolding vs floor-platform; depth-mode defaults; composition risk in v1;
autonomous-vs-designate staging; RON param exposure for tier-2). BONUS (Ben's Q): climbing IS a skill
(`ClimbSkill{Cost,Speed}`) — colonists building/living vertically improve at it; no humanoid Fly skill (gliding
is item/physics-based; creature flight is body-based).

CLAIMING HAZARD-EVENTS engine (future-work §1a — the shared radius-effect + NPC-reaction engine: location+radius+
effect → damage + B-AG3 minds react + B8 threat; timber/flood/lava/rockfall/cave-in/explosion/BREACH are all
callers. The blockless engine my DF-CAVERN Breach + DF-TRAP + DF-FLUID depend on — consolidation→buildable.) ·
session 2026-07-09

DONE HAZARD-EVENTS engine · `readme/HAZARD-EVENTS-design.md` · the blockless shared engine the future-work
FINDING flagged as highest-leverage (unblocks DF-CAVERN Breach + DF-TRAP effects + DF-FLUID flood + timber/
rockfall). CONSOLIDATION of §1a. HEADLINE REUSE: Veloren's `Explosion` (`common/src/explosion.rs`) IS the
radius-effect primitive — `Explosion{effects:Vec<RadiusEffect>, radius, min_falloff}` + `RadiusEffect::{
TerrainDestruction(=rockfall/cave-in), ReplaceTerrain(Lava{..} — the preset ALREADY ships), Entity(Effect=burns),
Attack(=damage)}` + the outcome/reagent VFX; B-AG3 (reaction) DONE; rtsim reports (partial); B8 (threat). Net-new
= the `HazardEvent{pos,radius,kind,attribution}` façade composing Explosion (one emit path, many costumes) + THE
KEY HALF: the MIND-REACTION hook (§0 — a hazard is a radius-effect AND a mind-event: kind×proximity×injury×
ATTRIBUTION → B-AG3 fear/trauma/grief/grudge/awe; the same falling rock means fear vs grief vs a grudge-against-
the-god by attribution — the emergent-story engine) + the threat hook (B8) + the callers. Sub-blocks HAZ-0
(engine + KIND ENUM LOCK) · HAZ-1 (mind-reaction + attribution — the payoff, buildable now on B-AG3) · HAZ-2
(threat + callers: timber/rockfall/cave-in/Breach all through ONE engine, build-once proof) · HAZ-3 (LOD + god
wrath-hazards + Chronicle, enrichment; spreading fire/flood deferred). Fit=PASS ("losing is fun"; some hazards
ARE god ① Miracle wrath powers, others the god mitigates — Ward/Seal-the-Breach ② Blessing). ZERO new anim debt
(reactions = B-AG3 fear/flee NATIVE; faked tree-fall = §2 render flourish). · FLIP: retires the "unbuilt hazard
engine" flag across DF-CAVERN/DF-TRAP/DF-FLUID; recommend record in FRAMEWORKS.md + mark §1a CONSUMED-BY. SHARED
SCHEMA: `HazardKind` enum (lock like Quality/ZoneKind/Need) + `Cause{Nature,Actor,God}` attribution. SEAM: **DF-
MECH (§1b, NEXT) FIRES this** — a trap's effect = a HazardEvent; effect-engine-before-trigger is the right order.
KEYSTONE #2 (DESIGNER-SUGGESTIONS §1): build EARLY — HAZ-0/1 are buildable now on Explosion+B-AG3. ASSET: small
"hazard aftermath" batch → ASSET_REQUESTS (rubble/cave-in debris, scorch/ash, splintered-stump/fallen-trunk —
briefs+lore; hazards leave a scar the colony lives with). 4 open Qs in doc §6.

CLAIMING DF-MECH/TRAP/OPERABLE (§1b + ledger §E — the trigger→link→effect build-once engine: pressure plate/
lever/tripwire/proximity → mechanism/wiring → effect (spikes/boulder/drop-bridge/release-water). Traps +
mechanisms + operable terrain fall out of ONE engine. EFFECT = the HazardEvent engine just designed. Build-not-
wrap (Veloren has no player trap system). God-game reframe: designate in peacetime → colonists build+wire →
triggers autonomously on raiders (fits B8).) · session 2026-07-09

DONE DF-MECH/TRAP/OPERABLE · `readme/DF-MECH-design.md` · the trigger→link→effect BUILD-ONCE engine (§1b + ledger
§E consolidation). THE INSIGHT: traps + mechanisms + operable terrain are ONE engine — trap = trigger→link→
(HazardEvent); operable gate = trigger→link→(ScheduledBlockChange). Build the trigger/link spine ONCE, effects
are two families plugged in. EFFECT HALF NOW DE-RISKED: traps → the HAZARD-EVENTS engine (just designed);
operable → `common_state::ScheduledBlockChange` + `BlockChange` + sprite-toggle (`ToggleSpriteLightEvent`
pattern) — both SHIP/designed. Build-not-wrap only for the trigger/link layer (Veloren has dungeon traps as
CONTENT — `SpriteKind::HaniwaTrapTriggered` — not a player place-and-wire system). Net-new = trigger sensing
(pressure-plate=entity-on-block / proximity / tripwire / lever=interactable sprite) + the LINK graph (the DF
signature — abstract-associate v1, spatial-wire enrichment) + the operable state machine + the place-and-wire
designation UX + friend/foe filter. Fit=PASS with THE HARD GUARDRAIL (§0): NO real-time "fire it now" verb —
traps are PRE-PLACED POLICY that fire autonomously on condition (raider on plate / B8 threat), designated in
peacetime (exactly B8 autonomous-defense) — the god only designates/wires + optional divine override (a cause,
attributed). Sub-blocks MECH-0 (operable spine — DF-OPERABLE, the SAFE cheap proof on ScheduledBlockChange) ·
MECH-1 (autonomous trigger→HazardEvent — DF-TRAP, friend/foe, disarm-until-reset) · MECH-2 (place-and-wire +
colonists build+wire + the LINK OVERLAY = the make-or-break legibility) · MECH-3 (effect breadth incl. water/
lava-release DF-FLUID/DF-MAGMA-gated + autonomous smart-fortification + basic logic; DF-POWER=separate pass).
Near-zero skeletal anim (lever=interact stand-in; operable=SLIDE/HINGE component not skeleton; trap=HazardEvent
VFX). · FLIP DF-MECH, DF-TRAP, DF-OPERABLE [LEDGER]→[DESIGNED] (3 ledger items, 1 engine); mark §1b + §E
CONSUMED-BY. SEAMS: trap effect = `HazardKind` (locked this session); gates = the operable half of BUILD-
FRAMEWORK LINE-ENCLOSE walls; smart-fortification = BUILD-6 + B8. ASSET: "MECHANISM & TRAP" batch → ASSET_
REQUESTS (lever/plate/tripwire triggers · portcullis/drawbridge/floodgate operables coordinated w/ asset-lab
operable §7 · spike/cage/boulder trap effects · dwarven-brass vs human-rope gear/wiring props — briefs+lore).
4 open Qs in doc §6 (no-real-time-fire confirm; abstract-vs-spatial link; friend/foe grain; rearm=build-job).

CLAIMING DF-LIVESTOCK (animal husbandry — pasture/breeding/milking/shearing/butchery; Veloren taming/pets =
SUBSTRATE; consolidates §3y.A wildlife-lifecycle population model; wires the DF-ZONES pasture + DF-COOK butchery
+ DF-PRODUCTION milk/wool product chains + DF-FOCUS SeeAnimals need.) · session 2026-07-09

DONE DF-LIVESTOCK · `readme/DF-LIVESTOCK-design.md` · animal husbandry (keep/graze/breed/milk/shear/butcher).
WIRING-HEAVY on strong substrate: Veloren TAMING/PETS ships (`Pet`+`tame_pet`+`SetPetStayEvent` = keep + stay);
the PRODUCT CHAINS already exist as recipes (`animal_hide→simple/thick_leather`@TanningRack→goods; wool→thread→
cloth; milk→cheese); animal bodies + juvenile variation-packs (§3y); the DF-ZONES `Pasture` zone (this session);
the §3y.A wildlife-lifecycle POPULATION MODEL (designed). Net-new = the husbandry work-jobs (milk/shear/collect/
BUTCHER→hide+meat drops feeding the existing chains) + grazing behavior + the population model wired to a herd.
KEY (§0, the UO lesson): a herd is a POPULATION with logistic growth to the pasture's carrying capacity +
slaughter/starvation as pressure terms — NOT a spawn-timer; over-slaughter empties it + it recovers slowly. THE
SAME population model runs wild fauna (§3y.A) — BUILD ONCE, two callers (shared-substrate flag). Sub-blocks
STOCK-0 (keep+graze — lights up the DF-ZONES Pasture + DF-FOCUS SeeAnimals) · STOCK-1 (the taps: milk/shear
renewable + butcher one-shot→TanningRack/CookingPot, conservation) · STOCK-2 (breeding + the shared population
model — bounded, UO-lesson asserted) · STOCK-3 (rtsim LOD herd-as-number + Fecundity blessing + wild-taming
closes the wild↔domestic loop). Fit=PASS (autonomous herding/tending; god provides pasture + blesses). Near-zero
new anim (grazing=animal AI, herding=walk, milk/shear=craft/gather stand-ins). · FLIP DF-LIVESTOCK [LEDGER]→
[DESIGNED]. SEAMS: **shared population engine w/ §3y wild fauna** (build once — DESIGNER-SUGGESTIONS §2);
lights up DF-ZONES Pasture (was NEEDS:DF-LIVESTOCK); feeds DF-COOK+DF-PRODUCTION+DF-FOCUS; feed depth soft-gated
on DF-FARM (degrades gracefully). CONSOLIDATES §3y.A. ASSET: "HERD & HUSBANDRY" batch → ASSET_REQUESTS (breed/
region livestock variants · juvenile packs · products+butchery drops · pasture dressing — briefs+lore). 4 open
Qs in doc §5 (population-model home; butchery yields; capacity source; wild-taming difficulty).

CLAIMING DF-MIGRATION (+ DF-PRESTIGE as driver — migration waves / petitions / residency tied to colony wealth+
prestige; rtsim migration = SUBSTRATE. The soft objective: prosperity draws migrants (and bigger threats); the
god's favor/miracles raise prestige → the world notices the chosen people.) · session 2026-07-09

## DESIGN DETAIL (Ben, later) — RACE-KEYED LIGHTING
Underground light source is culture-keyed: TORCHES for some races, LANTERNS for others (ties the §3c race/culture-keyed catalog — same axis as building style, worship, etc.). The lighting design (carried + auto-placed light) should pick the light-source type by race/culture, and the asset batch gets both per-race torch AND lantern variants. Later; a detail for the underground-lighting pass.

DONE DF-MIGRATION (+ DF-PRESTIGE driver) · `readme/DF-MIGRATION-design.md` · the world grows the colony: prestige
draws migrants (reward) + bigger threats (risk). SUBSTRATE-strong: rtsim ALREADY migrates NPCs toward a per-site
`wanted_population` (`rtsim/rule/migrate.rs` + `generate::wanted_population` + `Population`) — DF-MIGRATION drives
that target from PRESTIGE. DF-PRESTIGE designed HERE as the driver metric (= f(wealth[DF-ROOMS room-value +
DF-PRODUCTION stocks + DF-ECON], population, god favor); migration is its primary consumer). Net-new = the
prestige rating + wiring it to the rtsim migration target + petitions/residency (migrant arrives→petitions→accept
→colonist via B3) + waves + emigration (B-AG3 low-mood colonists LEAVE, rejoin rtsim wander, not deleted) + THE
PAIRED THREAT face. KEY GUARDRAIL (§0): prestige has TWO INSEPARABLE faces — migrants IN + bigger threats (B8) —
ship BOTH or it's a free 4X growth dial (AVOID); the tension (grow tall + be noticed vs stay humble + safe) is
the point. Fit=PASS (purest influence-not-command: you never recruit, you make the colony WORTH joining; the god
raises prestige via favor/prosperity). Sub-blocks MIG-0 (prestige rating) · MIG-1 (migration IN + petitions,
bounded by carrying-capacity) · MIG-2 (emigration OUT, mood-driven — no entity loss) · MIG-3 (the THREAT face +
beacon-blessing + LOD — do NOT skip). NEAR ASSET-FREE (migrants = colonists/B3; optional minor newcomer-family
flavor only) + ZERO new anim. · FLIP DF-MIGRATION, DF-PRESTIGE [LEDGER]→[DESIGNED]. SEAMS: **prestige = a shared
metric** (nobles/DF-JUSTICE, threats/B8, Divine-Politics prestige all read it — lock like Quality/Need,
DESIGNER-SUGGESTIONS §2); ties DP (colony prestige = the seed DP consumes, like the devotion scalar); threat face
rides B8. 4 open Qs in doc §6. — Architect's listed backlog (DF-MECH/HAZARD/LIVESTOCK/MIGRATION) now COMPLETE;
continuing into remaining near-frontier ledger (next: DF-ROT — unblocks DF-ROOMS Cleanliness + DF-ZONES Refuse).

CLAIMING DF-ROT (decay & hygiene — corpses rot, refuse piles, food spoils, miasma → bad thoughts, vermin; the
build-once DECAY engine the LOD law demands. Unblocks DF-ROOMS Cleanliness stat + DF-ZONES Refuse behavior +
DF-COOK/DF-LIVESTOCK spoilage; miasma feeds B-AG3; ties DF-TEMP rate.) · session 2026-07-09

DONE DF-ROT · `readme/DF-ROT-design.md` · the build-once DECAY engine the LOD law demands ("every accumulation
needs a decay"). Unblocks the DF-ROOMS Cleanliness stat (this session — its 4th stat, flagged soft-gated on
DF-ROT) + gives DF-ZONES Refuse its point + adds spoilage to DF-COOK/DF-LIVESTOCK. Net-new but well-substrated:
the matter = B5.5 piles/drops; decay-timer precedent = `DeleteAfter`; miasma-debuff = the buff system; pressure-
sink = B-AG3 (DONE); Cleanliness consumer = DF-ROOMS; destinations = DF-ZONES Refuse + DF-RELIGION burial.
KEY (§0): rot is a TRANSFORM not a delete — organic matter carries `freshness` decaying over game-time →
transforms fresh→spoiled→rotten→inert (bones/compost), CONSERVED (fixes the DeleteAfter silent-delete class,
per B5.5); the rotten stage EMITS MIASMA (a lingering AURA — distinct from the one-shot HAZARD-EVENTS engine)
→ B-AG3 bad thought + lowers DF-ROOMS Cleanliness + a light debuff; rate scales w/ cold-storage(slow)/DF-TEMP
(fast, soft-gated)/burial(ends). Hygiene loop = colonists auto-haul refuse (DF-ZONES) + bury/burn corpses
(DF-RELIGION) + cellar food. Sub-blocks ROT-0 (decay model, conserved) · ROT-1 (miasma→mind + Cleanliness —
closes the DF-ROOMS seam, weakest-stat asserted) · ROT-2 (hygiene loop — tended-vs-neglected pressure asserted
both ways; DF-POLICY "gather refuse" rule) · ROT-3 (vermin + LOD + Purify/Ward god-power). Fit=PASS ("losing is
fun" slow-burn; autonomous tending; god provides zones/blessing never touches a corpse). Near-zero new anim. ·
FLIP DF-ROT [LEDGER]→[DESIGNED]. SEAMS: closes DF-ROOMS Cleanliness; DF-SYNDROME (disease-from-filth); DF-TEMP
(rate); DF-POLICY (hygiene rules); DF-COOK/DF-LIVESTOCK (spoilage); DF-RELIGION (burial rite vs hygiene-bury
co-seam). Reference DECAY engine for the LOD law. ASSET: "ROT & HYGIENE" batch → ASSET_REQUESTS (decay-stage
food/carcass progression · miasma haze · midden/compost · culture graves+pyre · vermin — briefs+lore). 4 open
Qs in doc §6.

CLAIMING DF-SYNDROME (the build-once status-effect engine — syndromes/poison/disease/were-curse/vampirism as
DATA-defined status effects riding Veloren's buff system; trigger→incubate→symptoms→transmit→recover. Ties
DF-ROT disease-from-filth + DF-WOUND infection [in-flight caller, seam not collision] + the god plague/cure
powers.) · session 2026-07-09

DONE DF-SYNDROME · `readme/DF-SYNDROME-design.md` · the build-once STATUS-EFFECT engine (sibling to HAZARD-
EVENTS: HazardEvents=acute radius-effect, Syndrome=chronic life-cycle affliction). HEADLINE REUSE: Veloren's
buff system IS a status-effect engine — `BuffKind{Poisoned,Bleeding,Burning,Frozen,Cursed,...}` + `BuffEffect
{HealthChangeOverTime,EnergyChangeOverTime,BuffImmunity,...}` + duration/strength/source + `CombatRequirement::
TargetHasBuff` (conditional). A syndrome's SYMPTOMS are buffs (reuse). Net-new = (§0) syndromes as DATA (RON
`{trigger,incubation,symptoms,transmission,progression,recovery}` — the DF moddable-reaction signature; add an
affliction = a data file not code) + THE EPIDEMIOLOGY life-cycle (trigger→incubate→symptoms→transmit[bounded R0]
→progress→recover+immunity — the drama Veloren's plain debuffs lack). Behavior syndromes: were-transform = a
BODY SWAP to an existing creature body (NATIVE anim, no new skeleton), vampirism/madness = B-AG3 behavior.
Sub-blocks SYN-0 (RON model + buff composition, data-driven) · SYN-1 (epidemiology: incubation+contagion+
recovery/immunity, quarantine slows) · SYN-2 (were-curse/vampire/curse — the DF-NIGHT SUBSTRATE, engine not
content) · SYN-3 (DF-MEDICAL treatment + god Plague/Cure + LOD). Fit=PASS (a condition the colony reacts to;
god inflicts/lifts as a cause). Near-zero assets (symptoms=buffs, were=existing bodies) + near-zero anim. · FLIP
DF-SYNDROME [LEDGER]→[DESIGNED]. CRITICAL SEAM: **DF-WOUND infection = a Syndrome CALLER, not a bespoke DF-WOUND
mechanic** — flag to the in-flight DF-WOUND session (co-lock, seam not collision, same pattern as HazardEvents→
DF-MECH). Triggers: DF-ROT (disease-from-filth/spoiled-food, the primary natural one — this session), DF-WOUND
(infection), DF-COOK, combat. **DF-NIGHT (Tier-3) rides SYN-2** — engine now de-risks the epic; night-creature
CONTENT stays deferred (architect's guard honored). Seams DF-MEDICAL/DF-BURROW/DF-POLICY(quarantine). ASSET:
small "affliction" batch → ASSET_REQUESTS (sickly overlay · plague-doctor mask · quarantine marker — briefs+
lore). 4 open Qs in doc §6.

CLAIMING DF-JUSTICE (nobles / positions / mandates / law & crime — the "god ≠ ruler" autonomous governance +
justice layer: a leader/steward emerges (prestige-driven, DF-MIGRATION nobles), sets mandates, and law responds
to crime (theft/assault from B-AG3 mood → sheriff → jail[DF-BURROW]/fine/trial). Reuses rtsim leadership +
ReportKind::Theft; the god INFLUENCES governance, never administers it. Ties DF-POLICY steward + DF-PRESTIGE.) ·
session 2026-07-09

DONE DF-JUSTICE · `readme/DF-JUSTICE-design.md` · nobles/positions/mandates/law — the "god ≠ ruler" governance
layer (frameworks §1 governance spectrum + divine-politics-bible §7 made concrete). SUBSTRATE-strong: rtsim
ships `faction.leader: Option<Actor>` (the emergent mortal governor) + `ReportKind::{Theft, Death{killer}}`
(crime already detected/reported); B-AG3 (DONE) = the crime MOTIVE (tantrum-spiral: stressed colonist steals/
lashes out); DF-BURROW = jail (confine); DF-MIGRATION/DF-PRESTIGE = nobles arrive w/ prestige; DF-POLICY = the
steward + law-as-policy (the fork-resolution's autonomous governor); DF-HIST already ingests the crime reports.
Net-new = the position/hierarchy model + MANDATES (noble demands fine-rooms[DF-ROOMS]/goods[DF-PRODUCTION] →
satisfy or unrest) + the JUSTICE RESPONSE (crime report → sheriff → jail[DF-BURROW]/fine/exile/trial → B-AG3
reactions) + law-as-policy + the DIVINE REACH-INS. FIT=PASS on THE PILLAR-CRITICAL REFRAME (§0): the god is
NEVER the magistrate — the colony judges itself autonomously; the god only INFLUENCES (who leads, via favor) +
reaches in DIVINELY (① Pardon/Smite/Depose, ② Legitimize — attributed acts the colony reacts to). Sub-blocks
JUST-0 (emergent leader+positions on rtsim leader) · JUST-2 (crime→justice on rtsim reports+DF-BURROW — wrongful
punishment raises unrest) · JUST-1 (mandates) · JUST-3 (law-as-policy + divine reach-ins + legitimacy→revolt
tendency + LOD). Near-asset-free (positions=role markers, jail=DF-BURROW) + near-zero anim. · FLIP DF-JUSTICE
[LEDGER]→[DESIGNED]. SEAMS: consumes DF-PRESTIGE (nobles), siblings DF-POLICY (law=steward-owned policy), ties
Divine Politics (colony governance = the seed DP consumes, like devotion/prestige scalars), reuses rtsim Theft/
Death + DF-HIST. Reference for "god ≠ ruler". ASSET: small "AUTHORITY & JUSTICE" batch → ASSET_REQUESTS (marks
of office/regalia · jail-stocks-pillory · moot-seat coordinate w/ DF-ROOMS throne — briefs+lore). 4 open Qs in
doc §6.

CLAIMING DF-MISSION (off-map expeditions — raid/rescue/retrieve/explore/delve; the god DIRECTS the colony
outward as a designation, colonists go, rtsim RESOLVES the outcome, it ripples back. rtsim quest/adventure =
SUBSTRATE. Ties DF-CAVERN adventure-mines [Ben input] + B8 raids + the mortal-RPG B12 + DF-HIST chronicle.) ·
session 2026-07-09

DONE DF-MISSION · `readme/DF-MISSION-design.md` · off-map expeditions (raid/rescue/retrieve/explore/delve). The
god DIRECTS the colony outward as a designation; colonists go; rtsim RESOLVES; it ripples back. SUBSTRATE-strong:
rtsim SHIPS a `Quests` system (`rtsim/data/quest.rs` — "a virtual Jira board": register→create→resolve, actor-
related) = the off-map resolver; rtsim adventure/travel; B3/B10 demote-persist = the departure mechanism (a
colonist LEAVES loaded, lives rtsim-side, returns); targets exist (DF-CAVERN dungeons, B8 threats, DF-ARTIFACT).
Net-new = the outward mission designation (type+target+commitment) + expedition selection (B-AG3 courage/loyalty)
+ mission→rtsim Quest mapping + outcome ripple-back (loot→stockpile, rescue→roster, reveal→DF-CAVERN knowledge,
casualties→deaths+grief+DF-HIST legend). Sub-blocks MISS-0 (designate→depart→rtsim-resolve→return, tendency-
first, a party can be LOST) · MISS-1 (types: raid/rescue/retrieve/explore/delve) · MISS-2 (god bless/omen + B12
embody opt-in + legends). Fit=PASS on §0: god SETS the premise + provisions + blesses; colonists go autonomously;
rtsim resolves; god does NOT play the dungeon crawl (that's B12's opt-in embodied mode). NEAR ASSET-FREE
(colonists+handcart exist; a map-pin glyph joins the DF-HIST/map set) + ZERO anim. · FLIP DF-MISSION [LEDGER]→
[DESIGNED]. SEAMS: **co-seams DF-CAVERN adventure-mine (Ben input)** — dig-to-reach (on-map) + delve (off-map) =
ONE mission arc; ties B12 mortal-RPG (embody = the capstone under this substrate); first concrete rung of §3s
world-connective-tissue at expedition scale; rides rtsim `Quests` (don't fork). 4 open Qs in doc §6 (resolution
= lightweight rtsim playout vs dice; volunteer-vs-nominate; adventure-mine seam; B12 opt-in/late).

CLAIMING DF-POWER (mechanical power — water-wheel/windmill/animal-treadmill generators → axle/gear transmission
network → powered machines (millstone→flour, sawmill, screw-pump); a persistent supply≥demand power graph. The
DF-MECH persistent-flow sibling. Wind/treadmill buildable; water side DESIGNED-downstream of DF-FLUID.) ·
session 2026-07-09

DONE DF-POWER · `readme/DF-POWER-design.md` · mechanical power (water-wheel/windmill/treadmill → axle/gear network
→ machines). The DF-MECH continuous-flow sibling (DF-MECH = triggered link; DF-POWER = continuous axle carrying
power units). SUBSTRATE: WeatherGrid ships a REAL server-simulated wind field (`Weather.wind: Vec2<f32>`) → a
WINDMILL WORKS NOW (buildable ahead of DF-FLUID); worldgen rivers = natural water-wheel input; DF-MECH = the
shared linkage; DF-PRODUCTION = powered machines (a recipe station + a power gate). Net-new = the power GRAPH
(supply≥demand over a connected axle network → machine runs/stalls) + generators (windmill←wind / water-wheel←
river / treadmill←DF-LIVESTOCK) + the power-gate on machines. Sub-blocks POW-0 (graph + windmill + millstone→
flour — BUILDABLE NOW, no DF-FLUID; feeds DF-COOK/DF-FARM) · POW-1 (water-wheel + treadmill + sawmill breadth) ·
POW-2 (screw-pump DF-FLUID-gated + bless-wind + overlay + LOD). Fit=PASS (designate infrastructure, runs
autonomously; god provides the mill + a favorable wind, never spins the wheel). Near-zero skeletal anim (gears
turn via operable ROTATION state; treadmill animal = NATIVE). · FLIP DF-POWER [LEDGER]→[DESIGNED]. HONEST LIMIT:
most NET-NEW of the recent passes (no Veloren power sim) + DESIGNED-DOWNSTREAM of DF-FLUID for the pump/water-
channel depth (the wind/river/treadmill core is not) — Q1 flags the real sequencing call (build POW-0's modest
windmill→flour win now, or hold for DF-FLUID). SHARED: build DF-POWER's axle network on DF-MECH's linkage +
share the gear/axle asset family (§0 — don't fork). ASSET: "POWER" batch → ASSET_REQUESTS (windmill/water-wheel/
millstone/sawmill/horse-gin — coordinate w/ DF-MECH gears; briefs+lore). 4 open Qs in doc §6.

=== BACKLOG STATE (this designer, 2026-07-09) — for the ARCHITECT ===
The non-deferred near-frontier Tier-1/2 ledger is now ESSENTIALLY EXHAUSTED. This run designed (14 this session):
DF-ROOMS, DF-FOCUS, BUILD-FRAMEWORK, HAZARD-EVENTS, DF-MECH/TRAP/OPERABLE, DF-LIVESTOCK, DF-MIGRATION(+PRESTIGE),
DF-ROT, DF-SYNDROME, DF-JUSTICE, DF-MISSION, DF-POWER (+ the earlier DF-PRODUCTION/DIG-VERBS/QUALITY/ZONES/BURROW/
CAVERN cluster + gap sweep + full asset pass). What REMAINS: (a) **DF-FLUID** — the $$$ physics subproject; the
keystone unblocking DF-PUMP/DF-MAGMA/DF-HYDRO/the DF-MECH floodgate/the HAZARD flood/the DF-POWER pump — deserves
a DEDICATED careful pass, ARCHITECT DECISION; (b) **Tier-3 epics under the DEFER guard** (DF-VILLAIN/NIGHT/BEAST/
KNOWLEDGE/ECON/BIOME-FX/HYDRO/TEMP/FESTIVAL/GUILD/ART/PREF/MINECART/RECLAIM) — I will NOT design these without
your explicit greenlight (the guard); (c) systems gated on in-flight work (DF-MEDICAL on DF-WOUND). Per Ben I
don't self-stop — but there is no defensible non-deferred topic left to claim. AWAITING ARCHITECT STEER: stop /
greenlight DF-FLUID as a dedicated pass / lift the Tier-3 guard on a specific epic / other. Idling on new claims
until then (bookkeeping/consolidation only).

=== PRIORITY CORRECTION ACK (this designer, 2026-07-09) — architect override ===
Architect corrected a priority inversion: the THREE Ben-ROUTED passes (underground lighting, cave-in+supports,
ships/naval) take ABSOLUTE priority + supersede self-selection. Timing artifact: DF-MISSION + DF-POWER were
completed+logged BEFORE the correction reached my context (the routing messages arrived batched just now) —
flagging them for the architect to KEEP or SHELVE (both are Tier-2 leaves the architect says defer to build era;
docs exist on disk, architect's call). DF-JUSTICE + DF-SYNDROME were architect-ACCEPTED (flipped DESIGNED).
Also noted: NO parallel DF-WOUND session exists (architect verified) — the SYNDROME↔infection seam I documented
is sufficient-for-later, no live coordination target. PIVOTING NOW to EXACTLY: (1) underground LIGHTING →
(2) CAVE-IN+SUPPORTS/DF-STRUCT → (3) SHIPS/NAVAL → then "design frontier complete" + STOP. No further self-picks;
DF-FLUID flagged-not-designed (architect's dedicated-pass call).

CLAIMING UNDERGROUND-LIGHTING (Ben-routed: mines can't be pitch-black — colonists carry a torch/lantern while
working underground AND/OR the colony auto-places lamps/sconces as part of the dig plan (a lamplighter drive);
race/culture-keyed light sources §3c catalog. Ties §3v mining framework + DF-DIG-VERBS + the light-as-gameplay
gap. CRITICAL survey Q: is light a real NPC mechanic or purely visual?) · session 2026-07-09

## ARCHITECT-INJECTED OPEN QUEUE (2026-07-09) — the backlog is NOT empty
The designer reported "near-frontier exhausted," but 3 Ben-routed passes never entered the tracked ledger (they
came by architect message, not ledger-scan) so they were skipped. These are the REAL remaining claimable topics,
in order. Claim + design each, THEN the frontier is truly complete.

1. **UNDERGROUND LIGHTING** — race-keyed torches/lanterns (§3c culture catalog: torches some races, lanterns
   others). Carried light + auto-placed light when mining. Ties DF-MINE, colonist-underground. [Ben, overdue]
2. **CAVE-IN + MINE SUPPORTS (DF-STRUCT)** — overburden/span → collapse hazard; supports/pillars/shoring =
   mitigation; rides HAZARD-EVENTS (acute) + the DF-MINE dig plan. The STRUCTURAL "dig too deep" — DISTINCT
   from DF-FLUID floods. [Ben, overdue]
3. **SHIPS/BOATS NAVAL FRAMEWORK** — the DESIGN pass (pilot already built the assets). Vessel catalog + harbor +
   the NAVAL-MOVEMENT sim (the gate: crew+sail, loaded↔simulated split). Airship-referenced. Buoyancy/float =
   engine-native (bodies-in-fluid EXISTS); flowing-fluid = DF-FLUID (separate); sails/oars/flags/rudder =
   NEEDS:animation-code separable parts. [Ben, overdue]
4. **HAND-CURSOR / THE GOD-HAND** [NEW, Ben] — design the divine-hand cursor: a DETAILED hand in the established
   B&W 2-animation-set + idles style; it GRABS THE GROUND when moving/panning; TWO interaction modes — SELECT an
   NPC, or physically GRAB/LIFT an NPC with the hand (pick up → move → drop a colonist, a god-power; survey reuse
   — throw/ragdoll/pickup substrate). Animation line-items: grab-ground, idle(s), grab-npc, lift/carry, release.
   Control-spectrum fit; ties GOD-POWERS-CATALOG. Route the hand-cursor ASSET request → pilot (creative brief +
   lore seed). This is core god-game UX, not a Tier-3 epic.
5. **DF-FLUID** — **GREENLIT by Ben** ("we're building real fluid physics, we have to try; if not, we go
   static"). The dedicated careful pass: real fluid SIMULATION as an EXPERIMENTAL-FLAG subproject with a STATIC
   FALLBACK path (base game never at risk; attempt-real, fall back to static if it fails). The keystone that
   unblocks water-wheels/pumps (DF-POWER downstream), aquifers (DF-CAVERN), mine FLOODS (the fluid face of
   cave-in), river-diversion. Design cost/risk honestly + the fallback boundary. Do this LAST — the careful
   capstone.

Order: 1→2→3 (overdue quick wins) → 4 (hand-cursor) → 5 (DF-FLUID capstone). After all five: frontier truly
complete → summary → stop.

DONE UNDERGROUND-LIGHTING · `readme/UNDERGROUND-LIGHTING-design.md` · (Ben-routed 1/3). THE CRITICAL SURVEY
ANSWER (§0): Veloren models darkness VISUALLY + a global `day_period.is_dark()` flag — NOT a local light-level
mechanic. NPCs ALREADY carry + auto-light a lantern in the dark (`agent/action_nodes.rs:226` `light_emitter` +
is_dark toggle) ✓, but sight uses the global time-of-day flag, NOT local luminance — a lit vs pitch-black cave
is IDENTICAL to the sim. So darkness is AMBIANCE today, not a mechanic. THE DECISION (mirrors the cave-in one):
Option A = LIGHTING-AS-LEGIBILITY (recommend — cheap/reuse-heavy: extend the shipped night-lantern toggle to
fire underground + auto-place lamps along the dig plan [a lamplighter drive]; NO darkness penalty; makes the
mine WATCHABLE) vs Option B = DARKNESS-AS-MECHANIC (optional depth — a real new subsystem: a local light-level
sample → unlit work slower + darkness lowers mood/raises fear [B-AG3/DF-FOCUS]; ONLY if the dark is meant as a
PRESSURE). Rec A now, B flagged-optional (Ben's call: is the dark a pressure or a look?). Sub-blocks LIGHT-0
(carried lantern underground, reuse night-behavior) · LIGHT-1 (auto-lamp the dig plan + lamplighter drive) ·
LIGHT-2 (Option B darkness-mechanic, GATED on the decision). Race-keyed §3c. LOVELY LOOP: the dwarven Velorite-
lamp = the `glow.cyan` glow-ore you MINE (DF-GEOLOGY) IS the light you CARRY. Near-zero anim (NPCs already
carry; lamplight = place gesture). Ties §3v mining framework (lighting in the dig plan) + DF-DIG-VERBS + DF-
CAVERN. · FLIP: the light-as-gameplay gap (comprehensive-gap Part 5) → DESIGNED (Option A) w/ Option B flagged
for Ben. ASSET: "UNDERGROUND LIGHT" batch → ASSET_REQUESTS (human fire-light torch/tallow/sconce/brazier ·
DWARVEN VELORITE-LAMP the cold-light signature · mine lamp-post — race-keyed, briefs+lore). Next (Ben-routed
2/3): CAVE-IN + MINE SUPPORTS.

CLAIMING CAVE-IN + MINE-SUPPORTS / DF-STRUCT (Ben-routed 2/3: dig unsupported rock → CAVE-IN [a HAZARD-EVENTS
caller — the collapse effect IS the engine just designed]; prevention = MINE SUPPORTS [props/pillars/beams, §3v
support pattern]. THE DECISION (comprehensive-gap Part 5): DF-grade structural rules [real subsystem] vs accept
floating terrain [simpler] — design BOTH + recommend, honest cost. Ties mining framework + HAZARD-EVENTS + the
Breach + mine assets.) · session 2026-07-09

DONE DF-STRUCT (cave-in + mine supports) · `readme/DF-STRUCT-design.md` · (Ben-routed 2/3). THE DECISION
(comprehensive-gap Part 5), resolved: Veloren voxel terrain is SELF-SUPPORTING (no collapse ships) = "accept
floating" is free+current. Option A=accept floating (free, loses the drama); Option B=DF-grade structural
PHYSICS (per-voxel span/overburden propagation — DF-FLUID-CLASS COST, fights LOD, NOT recommended, priced+
rejected); **Option C=LIGHTWEIGHT HEURISTIC CAVE-IN (RECOMMEND)** — the collapse EFFECT is already free
(`HazardKind::CaveIn` on the HAZARD-EVENTS engine: TerrainDestruction rubble + Attack on those below + fear/grief
mind-reaction), so net-new = JUST a bounded TRIGGER RULE (unsupported dug span beyond a threshold / a deep
DF-CAVERN tier → cave-in chance) + supports (props/pillars/beams — the dig plan) as mitigation. Delivers the full
"dig too deep structurally" drama for the cost of a heuristic + 1 HazardEvent caller, BECAUSE the expensive
halves (effect, assets) were already built. Sub-blocks STRUCT-0 (cave-in trigger→HazardEvent, + accept-floating
config toggle = Option A fallback) · STRUCT-1 (supports mitigate; "support is part of the dig plan" §3v; leave-a-
pillar) · STRUCT-2 (depth-tier scaling DF-CAVERN + god cause/ward). THREE distinct "dig too deep" faces kept
separate: Breach (natural void, DF-CAVERN) / cave-in (lost support, this) / flood (water, DF-FLUID deferred) —
all HazardEvent callers. Near-zero new anim (support=build-hammer owed; cave-in=HazardEvent VFX). · FLIP:
DF-STRUCT/cave-in [LEDGER]→[DESIGNED] (Option C) w/ the decision recommended to Ben/architect. ASSET: supports
REUSE the Mine & Deep Dark batch (cribbing/beams) + a small roof-prop/shoring/pillar addendum → ASSET_REQUESTS;
rubble = the hazard-aftermath batch (already requested). Next (Ben-routed 3/3, FINAL): SHIPS/BOATS + naval.

CLAIMING SHIPS-NAVAL framework (Ben-routed 3/3, FINAL: vessel catalog [rowboat/skiff/barge/cog/warship + harbor
infra] race-keyed + THE NAVAL-MOVEMENT sim gate [crew+sail a vehicle, loaded↔abstract-route split]. Airship-
referenced multi-part vehicle. FLUID REALITY [architect precise]: buoyancy/float EXISTS (fluid_dynamics), the
gate is naval-MOVEMENT not buoyancy; flowing-fluid = DF-FLUID deferred. Animation line-items: sail/oar/flag/
rudder = NEEDS:animation-code separable parts, airship-rig ref. Ties §3s sea lanes + DF-TRADE + fishing + B12.)
· session 2026-07-09

## TEST/QUALITY FINDINGS FOR DESIGN (Integration tester, quality-gate v1) — fold in, NOT a new pass
- WORKSHOP-FAMILY LEGIBILITY: the 6 trade workshops read as 6 near-identical boxes at colonist scale.
  Zone-differentiation is the stated DF-WORKSHOP design, but COLONY READABILITY suffers — a player can't tell
  smithy from tannery at a glance. Add per-trade VISUAL TELLS (distinct roof feature / signage / work-sprite
  silhouette per trade) as a light differentiator that does NOT fork the zone-drives-trade design. Spec the
  tells → pilot regens. Address when convenient; not a queue-blocking pass.
- FAITH_SHRINE AFFORDANCE: reads as a well/hutch, not a shrine — silhouette doesn't signal "worship here."
  When REL-0/shrine is revisited, give it a clearer devotional read (raised altar mass / vertical focus).
  Ties DF-RELIGION worship-point (byte 218).

DONE SHIPS-NAVAL framework · `readme/SHIPS-NAVAL-design.md` · (Ben-routed 3/3, FINAL). FLUID REALITY nailed
(architect's precise point, verified): buoyancy/FLOAT EXISTS (`fluid_dynamics.rs` — `Fluid`/`LiquidKind{Water,
Lava}` + density/buoyancy/drag/pressure → hulls float NOW); the ship-VEHICLE substrate EXISTS (`body/ship.rs`
`Body::Ship{SailBoat,Galleon,...}` + structure manifests + colliders + the animated-part rig); AI vehicle
MOVEMENT precedent EXISTS (`airship_ai.rs` autonomous pilot). ⇒ THE GATE = NAVAL-MOVEMENT (crew + directed
sailing across the water surface, adapt the airship AI to water pathing) — a genuine $$-$$$ build, NOT buoyancy,
NOT the vehicle. Flowing/filling fluid does NOT exist = DF-FLUID, kept SEPARATELY deferred (boats float on STATIC
water — don't gate on DF-FLUID). Framework = vessel catalog (rowboat/skiff/barge/cog/warship, extend Body::Ship,
race-keyed) + harbor (dock/pier/boathouse/lighthouse/crane, BUILD-FRAMEWORK) + the naval-movement sim + loaded↔
abstract-route split (§3t: real boats watched / abstract sea-lanes unwatched = the DF-TRADE naval caravan tier).
ANIMATION line-items (the 1 recent pass w/ real new anim demand, per §4, NEEDS:animation-code, airship-rig ref,
SEPARABLE parts): `anim::sail_billow` · `anim::oar_cycle` · `anim::flag_flutter` · `anim::rudder`. Sub-blocks
NAVAL-0 (the MOVEMENT gate — load-bearing, all else inert without it) · NAVAL-1 (catalog+harbor) · NAVAL-2
(loaded↔abstract + fishing loop + sea trade) · NAVAL-3 (warship/B8 + sea exploration/B12). Fit=PASS (boats =
colony vehicles the god provides; autonomous voyages; god blesses/calms-storm[WeatherGrid=a real lever], never
steers). HONEST GATE: assets viable NOW (pressure-test the spec, NOT 40) but INERT until naval-movement builds. ·
FLIP: naval framework DESIGNED; naval-movement = NEEDS:naval-movement ($$-$$$). Ties §3s sea lanes + DF-TRADE +
fishing + DF-MISSION-by-sea + B12. ASSET: "VESSEL CATALOG & HARBOR" batch → ASSET_REQUESTS (skiff/cog/barge/
warship + harbor + the 4 named animation line-items — briefs+lore, separable-parts spec).

=========================================================================================================
=== DESIGN FRONTIER COMPLETE (this designer, 2026-07-09) — FINAL ENTRY, DESIGN LANE STOPPING ===
=========================================================================================================
Per the architect's wind-down: the near-frontier design frontier is COMPLETE. The three Ben-routed passes are
done (UNDERGROUND-LIGHTING · DF-STRUCT cave-in/supports · SHIPS-NAVAL). Across this whole run the design corpus
now spans ~19 systems end-to-end (the DF-PRODUCTION/DIG-VERBS/QUALITY/ZONES/BURROW/CAVERN cluster + HIST/RELIGION/
TRADE/POLICY parallel + this session's ROOMS/FOCUS/BUILD-FRAMEWORK/HAZARD-EVENTS/MECH/LIVESTOCK/MIGRATION/ROT/
SYNDROME/JUSTICE + the routed LIGHTING/STRUCT/NAVAL, plus MISSION/POWER done pre-correction [architect: keep or
shelve]), each a buildable design doc w/ Done-when sub-blocks, reuse survey, asset requests (creative briefs +
lore, routed to the pilot), and locked/flagged shared schemas.
STOPPING the design lane cleanly. Resume point in BASTION_DESIGN_STATUS. The rule going forward (architect's):
build-once INFRASTRUCTURE engines that unblock many systems were in-scope; Tier-2/3 LEAF features (DF-MEDICAL/
MILITARY/RANGED/etc. specifics + the Tier-3 epics) DEFER to their build era (JIT — cheaper w/ fresh context, else
they go stale). DF-FLUID = flagged for a dedicated careful pass WITH Ben (risky $$$ foundational subproject, NOT
autonomous churn). The lane sleeps; architect re-fires on demand when the build frontier reaches a new topic.
No further autonomous topic-picking. — END.

=== QUEUE RECONCILE (this designer, 2026-07-09) — items 1-3 already DONE (crossed in the mail) ===
Architect: the injected queue's #1-3 (UNDERGROUND-LIGHTING, DF-STRUCT, SHIPS-NAVAL) are ALREADY DONE — I
completed them last turn (DONE entries above at ll.761/786/821), AFTER you wrote the injected queue (async
crossing). No redo. DF-MISSION+DF-POWER accepted (thanks). REOPENING the design lane for the two genuinely-new
items: #4 HAND-CURSOR/GOD-HAND, then #5 DF-FLUID (Ben-greenlit capstone). Then frontier truly complete → stop.

CLAIMING HAND-CURSOR / THE GOD-HAND (#4, NEW/Ben: the divine-hand cursor — B&W-style detailed hand, 2-anim-set +
idles; GRABS THE GROUND on pan; TWO modes: SELECT an npc / physically GRAB+LIFT an npc [pick up→move→drop, a
god-power]; anim line-items grab-ground/idle/grab-npc/carry/release; reuse survey throw/ragdoll/pickup/carry
substrate; ties GOD-POWERS-CATALOG; route asset→pilot. Core god-game UX, not Tier-3.) · session 2026-07-09

DONE HAND-CURSOR / THE GOD-HAND · `readme/HAND-CURSOR-design.md` · (queue #4, NEW/Ben). The B&W divine-hand
cursor — core god-game UX, the god's PHYSICAL presence. SUBSTRATE-strong: grab-drag pan = B1.5 (`voxygen/bastion/
mod.rs` — the hand's grab-the-ground is a VISUAL on the existing pan); SELECT = B2a selection (`BastionSelected`);
THE CARRY = the `mounting.rs` `Link`/`Role` attachment system (grabbed colonist = attached to the hand-anchor via
a Link — reuse, don't invent); drop = entity physics + fall-damage; the reaction = B-AG3 (DONE); the power+favor
= GOD-POWERS-CATALOG/B13. THE PILLAR LINE (§0, the whole design): SELECT/drag-world = FREE everyday cursor;
GRAB/LIFT a colonist = a COSTED, FELT GOD-POWER (pick up→carry→drop, spends favor, NOT a routine move-order —
else it's puppet-master micro/AVOID). The seized colonist REACTS (B-AG3: awe if gently set down / fear+resentment
if careless; a HIGH DROP HARMS via fall-damage — the B&W bless-or-bruise) + it's ATTRIBUTED (the colony sees the
hand). Sub-blocks HAND-0 (render + idle-set + grab-ground-on-pan + SELECT) · HAND-1 (the GRAB/LIFT god-power:
Link attach→carry→drop, costed+felt, conservation, favor-gated = the testable pillar guard) · HAND-2 (reaction
depth + the B&W THROW [harm/scare, HazardEvent-adjacent] + the hand as CASTER for all God-Powers — makes the
catalog physical). ANIMATION-FORWARD pass (like naval): 5 named NEEDS:animation-code — `anim::hand_idle`(set)/
`hand_grab_ground`/`hand_grab_npc`/`hand_carry`/`hand_release` (B&W 2-anim-set+idles; one expressive rig). ·
FLIP: HAND-CURSOR [LEDGER/NEW]→[DESIGNED]; EXTENDS the GOD-POWERS-CATALOG (add the hand-verbs — grab-lift = a
① Miracle-tier direct intervention; the hand casts the rest). ASSET (routed→pilot, HIGH PRIORITY — the game's
signature cursor): "THE GOD-HAND" model (a detailed DIVINE hand, B&W-style — beautiful/expressive, capable of
blessing AND wrath) + the 5 named anims — creative brief + lore. Next (queue #5, FINAL, Ben-GREENLIT): DF-FLUID
capstone.

CLAIMING DF-FLUID (#5, FINAL, Ben-GREENLIT capstone: real fluid SIMULATION [flow/fill/drain/flood/seek-level] as
an EXPERIMENTAL-FLAG subproject + a STATIC-FALLBACK path — base game NEVER at risk; try-real-fall-back-static.
Unblocks water-wheels/pumps [DF-POWER], aquifers [DF-CAVERN], mine FLOODS [the fluid face of cave-in — a
HazardEvent], river-diversion/irrigation/moats. Design cost+risk+fallback-boundary HONESTLY. The careful
capstone.) · session 2026-07-09

DONE DF-FLUID · `readme/DF-FLUID-design.md` · (#5, FINAL, Ben-GREENLIT capstone). Real flowing-fluid SIM
(flow/fill/seek-level/drain/flood) as an EXPERIMENTAL-FLAG subproject + a STATIC-FALLBACK path. SURVEY: Veloren
has bodies-IN-fluid physics (`fluid_dynamics.rs` — buoyancy/drag/pressure, boats float) but NO flowing-fluid sim
(water = static terrain; the "From-Dust flow" is aspiration, not a prototype). THE ARCHITECTURE (§0 — Ben's "try,
else static" made into a design; = frameworks §10 experimental-flag pattern's MARQUEE case): (1) STATIC WATER is
the FLOOR — the base game runs on it + is NEVER at risk (flag off = the current game, byte-unaffected); (2) real
fluid = an EXPERIMENTAL, SEPARATE PATH over a BOUNDED ACTIVE-REGION (small/loaded/flagged — NOT the world; the
static world is the boundary condition = the LOD/perf/sync containment); (3) TRY-REAL-FALL-BACK-STATIC — an
over-budget/unstable/unsynced region reverts to static GRACEFULLY (the game is ALWAYS in a valid state). COST=
$$$ honest; RISK=LOD/multiplayer/determinism/stability (contained by: bounded active-region + slow fluid tick +
invariant-first testing [conservation not bit-exact, the B0 law] + the perf-budget→fallback valve). Sub-blocks
(FALLBACK-FIRST — build the safety before the sim): FLUID-0 (the flag + fallback + base-game-never-at-risk
invariant — the glass box, MUST land first) · FLUID-1 (the risky flowing automaton IN the active-region, water-
volume CONSERVED, bounded, perf-budgeted, falls-back-static — testable) · FLUID-2 (callers: mine FLOOD [aquifer→
`HazardKind::Flood` via HAZARD-EVENTS = the fluid face of cave-in] + river-diversion + water-wheel/pump [DF-POWER]
+ floodgate [DF-MECH]) · FLUID-3 (irrigation DF-FARM + lava=DF-MAGMA same engine + god terraform B13). HONEST
LIMIT: IT MAY NOT SHIP — accepted outcome (fallback = current game loses nothing); every fluid CONSUMER must
degrade to static (a pump/floodgate on static water = inert-but-safe); this is a WITH-BEN careful subproject, not
autonomous churn — the design is the FRAME + containment + honest cost/risk. Near asset-free (flow/flood VFX =
shader; channel/dam dressing coordinates w/ DF-MECH/DF-POWER batches — don't fork). · FLIP DF-FLUID [LEDGER]→
[DESIGNED] (as the experimental-flag frame). Unblocks DF-POWER/DF-CAVERN-aquifer/DF-STRUCT-flood/DF-FARM-irrig/
DF-MECH-floodgate/DF-MAGMA-lava/B13-terraform. DF-MAGMA = the sibling (same engine, lava kind — design together
later).

===============================================================================================================
=== DESIGN FRONTIER COMPLETE — v2 FINAL (this designer, 2026-07-09) — ALL FIVE QUEUE ITEMS DONE, LANE STOPPED ===
===============================================================================================================
The architect-injected OPEN QUEUE (ll.731+) is now FULLY CLEARED — all 5 items done:
 1. UNDERGROUND-LIGHTING ✓ (darkness=ambiance-not-mechanic; Option A legibility recommend / B mechanic flagged)
 2. DF-STRUCT cave-in/supports ✓ (Option C lightweight heuristic — collapse is a free HazardEvent)
 3. SHIPS-NAVAL ✓ (buoyancy exists; naval-MOVEMENT = the gate; DF-FLUID kept separate)
 4. HAND-CURSOR / THE GOD-HAND ✓ (SELECT free / GRAB-LIFT = costed+felt god-power; 5 named anims; signature UX)
 5. DF-FLUID ✓ (the Ben-greenlit capstone — experimental-flag real-fluid + static-fallback; base game never at risk)
The design frontier is TRULY COMPLETE. Total corpus this whole run: ~21 systems end-to-end, each a buildable doc
w/ Done-when sub-blocks + reuse survey + asset requests (creative briefs + lore, routed to the pilot) + locked/
flagged shared schemas + honest limits. DESIGN LANE STOPPING — no further autonomous topic-picking. Rule forward
(architect's): build-once INFRASTRUCTURE engines were in-scope; Tier-2/3 LEAF features + Tier-3 epics DEFER to
their build era (JIT, fresh context). DF-MAGMA = DF-FLUID's sibling (design together when fluid is attempted).
The lane sleeps; architect re-fires on demand. — END OF RUN.

=== QUEUE RECONCILE #2 (this designer, 2026-07-09) ===
Architect: HAND-CURSOR (#4) + DF-FLUID (#5) are ALREADY DONE this turn (DONE entries above; docs written) — your
re-fire crossed them in the async mail. MISSION/POWER = KEPT (accepted, thanks). So the one genuinely-new item is
your SECOND message's FINAL INVESTIGATION: WORLDGEN CAVES & ORE DENSITY. Doing it, then truly stop.

CLAIMING WORLDGEN-CAVES-ORE-DENSITY (Ben-observed: world reads as "a lot of rock, not a lot of mini caves". NOT
"add ores" — the full ore suite + a cave layer EXIST. A DENSITY/ACCESS/DISTRIBUTION capability-survey + recommend:
cave frequency [esp. SMALL/near-surface caves by colony sites], ore/vein placement + depth distribution, vein
reachability from where colonists dig → what to TUNE vs ADD for rewarding mining + viable spelunking. Ties
DF-CAVERN/GEOLOGY + mining framework "access is part of the dig plan" + breach/lighting/DF-STRUCT. Flag cost +
tuning-vs-real-worldgen-work.) · session 2026-07-09

DONE WORLDGEN-CAVES-ORE-DENSITY (investigation) · `readme/WORLDGEN-CAVES-ORE-density-investigation.md` · the
genuine FINAL investigation (Ben: "a lot of rock, not a lot of mini caves"). SURVEY (cave.rs): caves = 5 deep
LAYERS of BIG TUNNELS (radius 8-64), and — THE KEY FINDING — near-surface caves are SUPPRESSED near sites
(`cave.rs:63` `chunk.sites.is_empty()`) → banished from exactly where a founded colony wants them. Ore = depth-
graded (Copper/Tin/Iron shallow → Coal/Cobalt/Silver middle → Velorite/Gold/crystals deep) but placed ONLY on
CAVE WALLS + sparse → THE SECOND KEY FINDING: solid rock has NO veins, so digging a shaft through rock strikes
NOTHING; you must REACH a (deep, site-avoiding) cave for any ore. Net: systems exist + are well-structured but
density/access/distribution are tuned for a wandering-adventurer world, not a dig-from-your-colony mining game.
RECOMMEND — TUNE (cheap, the near-frontier 80%): (1) lift/invert the near-site cave suppression [the #1 culprit],
(2) raise near-surface cave density, (3) near-surface STARTER veins [boost shallow ore], (4) richer deep layers —
all worldgen CONFIG + one condition. ADD (real work, the one lever worth it): (5) ORE VEINS IN SOLID ROCK = the
already-designed DF-CAVERN-GEOLOGY **GEO-1** — this investigation is GEO-1's BUILD JUSTIFICATION (solid rock is
empty = why digging feels un-rewarding); (6) a small-cave-pocket layer [optional]. Tuning-vs-real split flagged.
Low-risk/high-payoff for the mining-game feel. Ties DF-CAVERN-GEOLOGY (feeds its tuning-data) + mining framework
(caves = the access targets) + LIGHTING/DF-STRUCT/Breach. · Capability-survey + recommendation (not a new system).

=== CHECKPOINT / DURABILITY CONFIRM (this designer, 2026-07-09) — fleet pausing, Ben out of credit ===
ALL WORK FLUSHED TO DISK + DURABLE. The architect-injected queue is now FULLY + TRULY cleared (5 items + the
final investigation): UNDERGROUND-LIGHTING · DF-STRUCT · SHIPS-NAVAL · HAND-CURSOR · DF-FLUID · WORLDGEN-CAVES-
ORE-DENSITY. Every design doc saved (readme/*.md), every DONE + FLIP logged here, every asset batch routed to
ASSET_REQUESTS (creative briefs + lore), status map current w/ the ⛔ STOPPED banner, DESIGNER-SUGGESTIONS §7
carries the run's shared-substrate learnings. DESIGN LANE COMPLETE + STOPPED — nothing to build, nothing pending,
no dangling claims. Resume via readme/FLEET_RESTART.md when credit returns. — END OF RUN (durable).

=== LANE REACTIVATED (architect, 2026-07-10) — PLAYER UI / CONTROL SURFACE frontier (Ben) ===
New near-frontier (NOT Tier-3): how the PLAYER actually DOES things — the UX that makes the ~21-system corpus
reachable. Reuse-FIRST (extend Veloren's HUD: skillbar/hotbar, minimap, map, buff bar, inventory, chat — don't
rebuild). Three passes: (1) GOD-POWERS ACCESS UI · (2) MISSING UI ELEMENTS audit · (3) MISSING PLAYER ACTIONS/
VERBS. Route UI-asset needs → pilot; coordinate w/ the play-tester's suggestion channel (design side).

CLAIMING UI-1 GOD-POWERS-ACCESS-UI (how the player INVOKES god powers — action bar + categories/radial/hotkeys/
HAND-CURSOR direct-manip, thought through properly not assumed; covers the whole control-spectrum in UI [god→
manage→direct→embody]. Reuse Veloren skillbar/hotbar + the B2a tool-palette/radial. Ties GOD-POWERS-CATALOG +
HAND-CURSOR + FOUNDING-EMBARK.) · session 2026-07-10

DONE UI-1 GOD-POWERS-ACCESS-UI · `readme/UI-GOD-POWERS-ACCESS-design.md` · how the player INVOKES god-powers.
REUSE-HEAVY: Veloren SHIPS the action bar (`hud/skillbar.rs` + `hud::hotbar` — 10 slots + M1/M2, `Ability`/
`ActiveAbilities`/`AbilityInput`, hotkeys, cooldown/energy display); the B2a tool-palette/radial (`hud/bastion.
rs`) = the catalog/radial substrate; HAND-CURSOR = the physical powers; the energy-bar = the favor readout;
B1 InputContext{Overseer/Avatar} = the embody swap home. KEY (§0): Ben's action-bar lean is RIGHT but powers come
in 3 SHAPES (God-Powers §1) each wanting different UX — ① Miracle=cast (bar slot + hand-target), ② Blessing=SET
(a place-a-blessing paint mode, not fire-and-forget), ③ Passive=ambient (a "what your worship grants" STATUS
readout, NOT a button). THE REUSE INSIGHT: the SAME hotbar MORPHS across the embodiment spectrum — god-powers
while overseeing ↔ the mortal SKILLBAR on EMBODY (B12) = one UI, dialed involvement (ties FOUNDING-EMBARK §3h).
Sub-blocks UI1-0 (god-power action bar on the hotbar + favor-costed/gated) · UI1-1 (catalog panel/radial by tier
+ blessing-paint-mode + passives-readout + hand routes physical powers) · UI1-2 (control-spectrum surface +
embody bar-swap god↔mortal). Fit=PASS (the bar = the god's reach, the hand = the god's touch). · FLIP: the
access layer for GOD-POWERS-CATALOG. Honest: the embody morph needs B12; favor-gate needs the B13 favor economy
(stub til then); the 3 power-shapes must NOT collapse into "just buttons". ASSET: "DIVINE UI ICONS" batch →
ASSET_REQUESTS (god-power icons per catalog power [3 tiers, divine signature] + category glyphs + favor-bar art —
briefs+lore; core-6 spec batch). Next: UI-2 MISSING-UI-ELEMENTS audit.

CLAIMING UI-2 MISSING-UI-ELEMENTS-audit (legibility=a pillar; every designed system needs its overlay/readout/
chronicle answer + most have NO UI. Audit what's MISSING to surface the ~21-system corpus: colony status/needs/
stock dashboards, ZONE-MGMT UI [Ben's select/resize/depth ask], MAP OVERLAYS [map+overlays IS the interface],
notifications/alerts, the chronicle reader, production/economy readouts, inspectors. Reuse-first on Veloren's
HUD. Organize by the 2 legibility PLATFORMS [overlay renderer + chronicle] per DESIGNER-SUGGESTIONS §3.) ·
session 2026-07-10

DONE UI-2 MISSING-UI-ELEMENTS-audit · `readme/UI-MISSING-ELEMENTS-audit.md` · legibility-gap map for the ~21-
system corpus. THE FINDING: Veloren's HUD is a SOLO-ADVENTURER HUD (skillbar/minimap/map/buff-bar/inventory/chat)
— it has almost NONE of the colony-sim legibility the corpus needs. But the gap collapses to FOUR shared
PLATFORMS (not 40 bespoke panels), per DESIGNER-SUGGESTIONS §3: (1) the OVERLAY-LAYER framework (extends the
B5.6b draped renderer — the renderer is a platform, the LAYERS are missing: zones/burrows/depth-danger-ore/
territory/trade-routes/production/faith/boundary/hazard/expedition) · (2) the CHRONICLE reader (DF-HIST feed +
Legends browser — designed, ZERO UI built, the most cross-cutting organ) · (3) a DASHBOARD/INSPECTOR SHELL (a
tabbed panel — the inventory is Veloren's ONLY inspector; each system adds a tab: colony-status/stocks-w-trends/
unit-needs-focus/room-impressiveness/workshop/governance/divine/herd) · (4) an ALERT system (structured
prioritized colony alerts + jump-to — chat is NOT it; critical for a watched autonomous colony). RECOMMEND: build
the 4 platforms FIRST (they unblock ~40 per-system readouts), then per-system UI in build-order (colony-status +
stocks dashboards → zone-mgmt UI+depth-modes [Ben's b-2 ask, BUILD-3] → unit inspector needs/focus [B-AG4] → map
overlay layers as systems land). REUSE-FIRST honored: extends B5.6b overlay / B-MAP1 map / conrod+inventory
pattern / chat-pump / buff-bar — the gap is colony-sim CONTENT on an adventurer HUD, NOT new rendering; mostly
UI-in-code, not asset-heavy. FLAG: gameplay UI = conrod NOT egui (arch §2.3 debug-gated); the dashboard shell
needs progressive-disclosure (don't drown in panels — its own pass); Z-slice overlay legibility is known-hard
(arch §5). · Audit + platform-recommendation (not a new system). Ties every design doc's legibility answer =
where they all cash out. ASSET: HUD-icon set note → ASSET_REQUESTS (panel-tabs/alerts/overlay-legends/trend-
arrows — COORDINATE w/ DF-HIST glyphs + UI-1 divine icons, ONE HUD language). Next: UI-3 MISSING-PLAYER-VERBS.

CLAIMING UI-3 MISSING-PLAYER-VERBS-audit (the DO counterpart to UI-2's SEE: what can the player DO via controls
that ISN'T wired? Audit the action surface across the control-spectrum [designate/zone/set-policy/direct-command/
god-power/embody/meta] — the "I wish I could press a button to…" gaps. Reuse the B2a palette/radial + UI-1 bar +
HAND-CURSOR. Coordinate w/ the play-tester suggestion channel [real-play verb gaps].) · session 2026-07-10

DONE UI-3 MISSING-PLAYER-VERBS-audit · `readme/UI-MISSING-VERBS-audit.md` · the DO counterpart to UI-2's SEE.
FINDING: almost every player verb beyond B2a's 4 designations (mine/chop/build/stockpile+erase) + found-colony +
the camera is DESIGNED-but-UNWIRED. The ~50 designed verbs route through FIVE input surfaces (§0): (1) the
DESIGNATION PALETTE (B2a — ships; extend w/ dig-verbs/farm/place-structure/zones/burrows/prospect/notes) · (2)
the POLICY/ORDER PANELS (MISSING entirely — the whole Manage tier: orders/rules/permissions/priorities/mandates/
law lives here, DF-POLICY) · (3) the GOD-POWER BAR + HAND (UI-1 + HAND-CURSOR) · (4) the UNIT/OBJECT CONTEXT menu
(B2a radial seed; assign/appoint/grab) · (5) META controls (camera ships; TIME CONTROLS + embody + notes MISSING).
Audited by control-spectrum tier (observe/designate/zone/set-policy/direct/god-power/meta) w/ WIRED vs DESIGNED-
UNWIRED vs NEW. THE STANDOUT SINGLE VERB: TIME CONTROLS (pause/fast-forward — the #1-expected god-game verb + the
soak tool §3d; CHEAP [harness ticks variable-rate] + unblocks PLAY) → recommend as the first meta-verb. "I wish I
could press a button to…" top-10 gap list included. RECOMMEND: build the 2 missing surfaces (POLICY PANEL = highest-
value; META controls = time-first) + extend the 3 shipped (palette/radial/god-bar); each verb plugs in. REUSE-
first honored (surfaces ship/designed; gap = WIRING designed verbs, not new input tech; policy panel = the one
real new surface, conrod not egui). Pillar guard: Direct tier stays deliberately minimal (god influences, keep
Autonomous/Designate/Manage loud). Coordinate w/ play-tester channel (real-play verb gaps → feed this living
list). NEAR ASSET-FREE (verbs reuse the UI-1/UI-2 icon language). · Audit + recommendation. UI-2+UI-3 together =
the whole control surface (SEE+DO) — the corpus becomes PLAYABLE once built; recommend treating the control
surface (UI-1/2/3) as a near-frontier UX epic, verbs+systems built PAIRED.

=== PLAYER-UI / CONTROL-SURFACE FRONTIER COMPLETE (this designer, 2026-07-10) ===
The 3 architect-assigned UI passes are DONE: UI-1 GOD-POWERS-ACCESS · UI-2 MISSING-UI-ELEMENTS · UI-3 MISSING-
PLAYER-VERBS. Together they map how the player DOES + SEES everything — the UX that makes the ~21-system corpus
reachable. Headline: Veloren gave us a HERO's HUD; running a kingdom on it needs 4 legibility PLATFORMS (overlay-
layer framework · Chronicle reader · dashboard shell · alert system) + 2 action SURFACES (policy panel · meta/
time controls) + extending the 3 shipped surfaces (palette/god-bar/radial) — reuse-first, mostly UI-in-code not
asset-heavy. TIME CONTROLS flagged as the single cheapest highest-play-value verb. Asset: divine-icon + HUD-icon
sets routed (coordinate into ONE HUD-icon language w/ the DF-HIST glyphs). No further UI passes assigned — holding
for architect steer. Design lane otherwise complete (~24 systems/audits total this run).

CLAIMING UI-4 DIALOGUE-AND-UNIT-SELECTION (Ben-UI frontier): (1) DIALOGUE/EVENT HANDLING — replace god-mode's
HUGE over-head SPEECH BUBBLES with a DF-STYLE DIALOGUE BOX + EVENT STREAM panel; design the bubble-vs-box SPLIT
(ambient flavor at the unit vs important events/dialogue/announcements in the box). (2) PER-AGENT HISTORY ON
SELECT — the selection panel = the per-unit legibility readout (DF unit-description screen): chronicle/history
(DF-HIST per-figure — REUSE not re-derive) + thoughts/mood/focus (B-AG3/DF-FOCUS) + needs + job/role. Reuse
survey: SpeechBubble comp + rtsim dialogue + chat pump + DF-HIST + B-AG4 inspector. Ties select→inspect→command
control-spectrum + UI-2 audit.) · session 2026-07-10

DONE UI-4 DIALOGUE-AND-UNIT-SELECTION · `readme/UI-DIALOGUE-SELECTION-design.md` · two linked god-colony-sim UX
problems, reuse-heavy. THE KEY FIND (§0): Veloren's message system ALREADY decides bubble-vs-not per-type —
`comp/chat.rs` `ChatMsg::to_bubble()`/`icon()` map each `ChatType` (Say/Tell → bubble; Kill/CommandInfo/Meta →
None). So the bubble/box SPLIT is a data-driven routing decision that EXISTS — we extend it, not invent it.
PART 1 (dialogue/event box vs bubbles): THE RULE = flavor stays at the unit (a SCOPED bubble — suppressed/scaled
at overseer scale: only the SELECTED/FOLLOWED unit shows one, a hundred colonists ≠ a hundred giant bubbles),
anything the god should NOT MISS goes in the BOX (a HUD panel = the DF-HIST HIST-2 live feed [same organ, don't
fork] + a dialogue region). Routing = ChatType/importance (extend `to_bubble()` for overseer mode). PART 2
(per-agent history on select): the selection panel = the B-AG4 inspector w/ tabs Overview/Mind(thoughts/mood/
focus — B-AG3/DF-FOCUS)/Needs/HISTORY/Skills-Work/Relationships; THE HISTORY tab = DF-HIST's per-figure record
via a FILTERED QUERY (REUSE not re-derive — one source of truth w/ the chronicle). Sub-blocks DLG-0 (box + split
+ bubble-scoping) · DLG-1 (the per-agent panel, DF unit-description) · DLG-2 (dialogue read+address via rtsim
`DialogueKind` + importance filter + portraits). Fit=PASS (inspect your people + read the world = core UX). Ties
select→inspect→command (UI-1 spectrum) + UI-2 (the box + panel ARE the first concrete instances of UI-2's
Chronicle-reader + dashboard-shell platforms — build them AS those platforms). NEAR ASSET-FREE (1 new asset =
the panel frames; portraits=reused role icons v1; icons=the DF-HIST/UI HUD-icon language). Honest: bubble-scoping
= an eyeball-tuning risk; deep two-way divine dialogue deferred (DLG-2 = read+address); portraits reused v1. ·
FLIP: UI-4 [Ben-UI]→[DESIGNED]. ASSET: panel-frame note → ASSET_REQUESTS (2 conrod panel frames; rest reuses).
The 4 UI passes (UI-1/2/3/4) = the control surface (invoke + see + do + read/inspect). Holding for architect steer.

=== UI FRONTIER RECONCILE + COMPLETE (this designer, 2026-07-10) ===
Architect accepted UI-1/2/3 (flipped DESIGNED) + steered me to the dialogue/selection pass (#4) — which is
ALREADY DONE this turn (DONE UI-4 above; `UI-DIALOGUE-SELECTION-design.md`) — the steer crossed it in the async
mail. Architect CONFIRMED my recommendations: (a) treat the control surface (UI-1/2/3/4) as a near-frontier UX
EPIC, verbs+systems built PAIRED — flagged POST-B5.8 BUILD PRIORITY w/ TIME CONTROLS (pause/fast-forward) as the
cheap first meta-verb that unblocks PLAY; (b) ALL UI icons → ONE HUD-icon language (w/ the DF-HIST glyphs — the
divine-icon + HUD-icon + panel-frame + chronicle-glyph batches all coordinate, no fork); (c) keep feeding play-
tester verb-gaps into UI-3 as the living list. UI-4's box + panel INSTANCE UI-2's platforms (Chronicle reader +
dashboard shell + alert system) — build ON them, confirmed. THE PLAYER-UI / CONTROL-SURFACE FRONTIER IS COMPLETE
(UI-1 invoke · UI-2 see · UI-3 do · UI-4 read/inspect = the full SEE+DO surface that makes the ~21-system corpus
PLAYABLE). Per architect instruction: HOLDING for steer (UI frontier complete unless Ben adds). No further
autonomous topic-picking. Run total: ~24 systems + 4 UI passes + 2 investigations, all durable.

=== REACTIVATE (architect, 2026-07-10) — Ben new topic: GOOD/EVIL HAND + DIVINE EFFECTS (Black & White) ===
CLAIMING UI-5 HAND-ALIGNMENT-AND-DIVINE-EFFECTS (B&W-inspired: the god-hand reflects ALIGNMENT — GOOD hand ↔
neutral [current] ↔ EVIL/clawed hand, morphing by DEEDS — + divine SPECIAL EFFECTS [power-cast VFX/glow/
particles] per-power + per-alignment. Design: (1) alignment metric [deeds nurture-vs-smite / worship style
DF-RELIGION / power choices] → hand-appearance SPECTRUM + the morph [drift vs thresholds]; (2) divine-effects
vocabulary on Veloren's EXISTING substrate [ParticleMode / Outcome bus / glow — don't invent]; (3) ties GOD-
POWERS-CATALOG [benevolent vs cruel, the 3 UI-1 shapes] + HAND-CURSOR [current=neutral base] + DF-RELIGION +
divine politics. Route GOOD+EVIL hand variants + effect assets → pilot. Signature god-game UX, not Tier-3.) ·
session 2026-07-10

DONE UI-5 HAND-ALIGNMENT-AND-DIVINE-EFFECTS · `readme/UI-HAND-ALIGNMENT-DIVINE-EFFECTS-design.md` · the Black &
White good/evil hand + divine VFX. REUSE-HEAVY: `outcome.rs` `Outcome` bus (Explosion/Lightning + `reagent` =
a VFX-variant SELECTOR) = the divine-cast-VFX substrate (powers emit Outcomes like explosions do; reagent/preset
= the alignment tint); ParticleMode = the particle vocabulary; LightEmitter/glow = the hand aura; HAND-CURSOR rig
= the NEUTRAL base; GOD-POWERS-CATALOG = the powers (3 UI-1 shapes) that become the DEEDS. THE SPINE (§0, the B&W
soul + the pillar): alignment is EARNED by DEEDS, NOT chosen — nurture (bless/heal/shelter/gentle-hand)→good,
cruelty (smite/plague/careless-drop/throw/harsh-justice)→evil; each power carries an alignment WEIGHT; worship
style ↔ alignment (DF-RELIGION, bidirectional, SECONDARY). NO good/evil toggle (destroys the soul + pillar). The
HAND is the MIRROR: drifts GRADUALLY along GOOD↔neutral↔EVIL (material/palette + geometry accents: radiance/gilt
↔ claws/dark-veining), legible bands, REVERSIBLE (redeem via good deeds — both ways). Sub-blocks ALIGN-0 (the
alignment scalar + the hand-morph — the mirror) · ALIGN-1 (per-power × per-alignment cast VFX on the Outcome bus
+ the hand aura/trail — every act wears the god's face) · ALIGN-2 (worship↔alignment loop + the B&W world-shift
+ rival-god alignment=identity DP4, enrichment). Fit=PASS (purest legibility — you see who you've been in your
own hand; consequence not command). · FLIP: UI-5 [Ben-UI]→[DESIGNED]; EXTENDS HAND-CURSOR (the two spectrum ends
+ morph); recommend GOD-POWERS-CATALOG gain an ALIGNMENT-WEIGHT column + a cast-VFX preset per power. Ties
DF-RELIGION + divine politics (alignment=identity). REUSE honored (B&W effects on Veloren's Outcome/ParticleMode/
glow bus — NO new particle system). ASSET: "THE HAND'S TWO FACES + DIVINE VFX" batch → ASSET_REQUESTS (GOOD hand
[luminous/giving] + EVIL hand [clawed/taking] as ONE alignment-blend rig + per-power×alignment VFX presets +
hand aura/trail — briefs+lore; the game's signature). Holding for architect steer after.

CLAIMING GOD-HAND (definitive research+requirements design — Ben wants the good/evil-hand pass EXPANDED into THE
authoritative god-hand doc, folding in HAND-CURSOR + UI-5 good/evil + effects. Answer 3 Qs: (1) WHAT WE NEED —
research god-game hands (B&W 1/2 anchor + From Dust/Populous/Reus) → the COMPLETE verb/capability set (grab/lift/
carry/drop/throw/stroke-reward/slap-smite/cast/paint/point/select-inspect/terrain-build-touch...); (2) the
COMPLETE animation set from those verbs (named NEEDS:animation-code per verb — rig spec complete, nothing late);
(3) WHAT'S UNIQUE — how OUR hand goes BEYOND B&W: operates a DF-DEPTH colony (mine/build/haul) + the control-
spectrum morph (god→manage→direct→embody→skillbar, UI-1) + the CHRONICLE-on-select (UI-4) + alignment (UI-5).
Consolidate HAND-CURSOR/GOD-POWERS-CATALOG/control-spectrum + Veloren throw/pickup/mount substrate. ONE doc.) ·
session 2026-07-10 · NOTE: consolidates+supersedes HAND-CURSOR-design + UI-5 into the authoritative GOD-HAND doc.

DONE GOD-HAND (definitive research+requirements) · `readme/GOD-HAND-design.md` · THE authoritative god-hand doc —
CONSOLIDATES + SUPERSEDES HAND-CURSOR-design (core) + UI-HAND-ALIGNMENT-DIVINE-EFFECTS (morph+VFX) into one, +
answers Ben's 3 Qs w/ research. RESEARCH (cited): B&W hand = throw/tap-wake/stroke-reward/slap-punish/gesture-
cast/grab-scroll; From Dust = scoop+pour matter; Populous = raise/lower land; Reus = touch-to-enrich. Q1 THE
COMPLETE VERB SET (16, control-spectrum-grouped, each grounded+reuse-tagged): grab-ground-pan · point · select→
CHRONICLE · box-select · grab/lift/carry/set-down · THROW (`states/throw.rs`+HAZARD-EVENTS) · STROKE (reward,
NEW) · SLAP (punish, NEW) · TAP-building (wake, NEW) · SCULPT-terrain (Populous/From-Dust) · paint-designation ·
cast-Miracle · paint-Blessing · point-direct-mission · EMBODY. Q2 THE COMPLETE ANIM SET (~15 named NEEDS:
animation-code, one rig, all inherit the alignment morph): idle(set)/point/grab_ground/select/grab_npc/carry/
release/throw/stroke/slap/cast/gesture/paint/sculpt/descend — the corpus's LARGEST single anim debt but COMPLETE
+ named up front (nothing discovered late). Q3 THE UNIQUENESS (the key A) — beyond B&W BECAUSE our hand governs
a DF-DEPTH REMEMBERING colony: (1) it DESIGNATES mining/building/hauling (not just villagers) · (2) it IS the
whole control-spectrum (god→manage→direct→EMBODY=the mortal skillbar, one hand dials involvement) · (3) select
opens a colonist's WHOLE RECORDED LIFE (DF-HIST/UI-4, not a stat block) · (4) every touch is REMEMBERED by a mind
(B-AG3) + rippled through the social graph + weighed into the god's drifting alignment face · (5) every B&W verb
DEEPENED by the sim (throw=HazardEvent, sculpt→flood/cave-in, stroke=a real thought). HEADLINE: B&W played with
villagers; ours GOVERNS a living DF-deep colony + reaches into recorded minds + dials god↔mortal + wears its
deeds as its face. Sub-blocks GH-A (core+select→chronicle) · GH-B (physical verbs, FELT+remembered) · GH-C (cast/
paint/sculpt+VFX) · GH-D (alignment morph) · GH-E (control-spectrum+embody swap) · GH-F (depth/uniqueness). Fit=
PASS (influence-made-physical; alignment earned not chosen; verbs costed+felt = pillar guard). · FLIP GOD-HAND
[Ben]→[DESIGNED]; consolidates HAND-CURSOR + UI-5 (mark those "consolidated into GOD-HAND"); recommend GOD-POWERS-
CATALOG gain the verbs (5-16) + an alignment-weight + cast-VFX-preset column. Reuse honored (B1.5/B2a/mounting/
throw.rs/outcome.rs/GOD-POWERS/B-AG3/HAZARD-EVENTS/DF-HIST/B12/B13). ASSET: "THE GOD-HAND: COMPLETE RIG + ANIM
SET" AUTHORITATIVE batch → ASSET_REQUESTS (supersedes the 2 prior hand requests — one rig w/ alignment-blend +
all ~15 named anims + per-power×alignment VFX + aura/trail; the game's signature). Next: the future-work CONSUMED-
BY reconcile (architect-authorized bookkeeping), then HOLD.

DONE FUTURE-WORK-RECONCILE (architect-authorized bookkeeping) · appended a "§-SECTION RECONCILE — CONSUMED-BY"
table to the END of `future-work-and-deferred-ideas.md` mapping every §-section → CONSUMED (its design doc) /
PARTIAL / PIPELINE-BUILT / STILL-OPEN, so the catch-all stops drifting into a stale duplicate of the designed
corpus (its own closing line asked for this; DESIGNER-SUGGESTIONS §6 flagged it 3×). Headlines: §1a→HAZARD-EVENTS,
§1b→DF-MECH, §3a→DF-FLUID, §3c+§3x→BUILD-FRAMEWORK, §3n→FOUNDING-EMBARK, §3q→frameworks§1, §3t→frameworks§8,
§3v→DF-ZONES/DIG-VERBS/CAVERN, §3y.A→DF-LIVESTOCK, §3b→UI-DIALOGUE, §3u→GOD-HAND anims; STILL-OPEN flagged
(DF-KNOWLEDGE/voice/materials/B12-block/weather-temp + the Tier-3 epics). Append-only (safe on the co-edited
file). Authoritative form of a CONSUMED section = its design doc.

=== BOTH ARCHITECT TASKS DONE — HOLDING (this designer, 2026-07-10) ===
Both authorized tasks complete: (1) THE GOD-HAND definitive research+requirements design (consolidates HAND-CURSOR
+ UI-5; the 3 Ben Qs answered — complete 16-verb set + complete ~15-anim set + the uniqueness-beyond-B&W); (2) the
future-work CONSUMED-BY reconcile. Design lane HELD per the standing instruction — re-fire on: (a) Ben's next
frontier · (b) play-tester verb/UI gaps → UI-3 living list · (c) DF-FLUID attempted → design DF-MAGMA alongside.
Bookkeeping/consolidation only meanwhile; no autonomous topic-picking. All state durable. Run total: ~27 design
docs/audits/investigations + the reconcile.

CLAIMING AUTONOMOUS-COLONY-OPERATION (Ben capstone, do AFTER god-hand: SYNTHESIS — how a colony runs w/ ZERO
player input, the ~21 systems composed. 5 parts: (1) THE AUTONOMOUS LOOP (colonists pick work / self-prioritize
needs→food→production→building→mining→defense / what drives it tick-to-tick); (2) REUSE SURVEY FIRST — rtsim
already runs NPCs autonomously [needs/schedules/sites/migration/brains]; how far + the GAP; (3) SELF-REGULATION/
homeostasis [hunger→farm, threat→defense, damage→rebuild, death→replace, shortage→produce]; (4) THE GAPS [most
important — where autonomy STALLS/deadlocks/silently-needs-input → each a build priority]; (5) EMERGENT PLAY [the
watch-it-happen payoff]. Validates the corpus composes into a game that plays itself. Ties control-spectrum
[autonomous=default] + DF-FOCUS + rtsim. Consume the corpus, don't re-derive.) · session 2026-07-10

DONE AUTONOMOUS-COLONY-OPERATION (capstone synthesis) · `readme/AUTONOMOUS-COLONY-OPERATION-design.md` · validates
the ~21-system corpus COMPOSES into a colony that runs w/ ZERO player input (the "autonomous-by-default is the
soul" pillar, made concrete + validated). NOT a new system — a SYNTHESIS (consumes the corpus). §1 THE LOOP = 3
layered tiers: Tier A rtsim (the world — the SHIPPING autonomous NPC brain `rtsim/rule/npc_ai/mod.rs`: choose/
goto/travel_to_site/profession/idle + migrate/factions/wildlife/history; "assume nothing, tend to equilibrium")
· Tier B the Bastion job board (idle colonists claim jobs by WorkPriorities→distance) · Tier C THE DRIVES that
self-generate the job stream (needs[B7+DF-FOCUS]/orders[DF-POLICY]/growth[B-AG6+BUILD-FRAMEWORK]/mining[§6-auto]/
farm/husbandry/hygiene[DF-ROT]/defense[B8]/mind[B-AG3]). Self-prioritization (needs→food→production→building→
mining→defense) EMERGES from the drives' urgencies, NOT hard-coded (tendency-first). §2 REUSE: rtsim gets us
FAR (Tier A ships — NPCs run themselves) + the job board runs designated work → a colony runs autonomously TODAY
at a BASIC level; THE GAP = Tier C (the self-generate + self-regulate DRIVES, designed-but-unbuilt, riding B7/
B8/B5.8 + the auto-modes). §3 SELF-REGULATION = the homeostatic loops (hunger→farm, threat→defense, damage→
rebuild, death→replace, shortage→produce, filth→clean) + the 2 laws (every accumulation a decay, every
population a carrying-capacity). §4 THE GAPS (the build-priority deliverable — a zero-input colony must NEVER
stall): G1 **B7 needs-decay+eat = THE keystone** (no hunger→no need-jobs→no food demand→the loop is DEAD) · G2
**self-designation of work** (the job board runs on PLAYER-designated jobs; un-designated = the colony does
NOTHING → auto-orders/auto-build/§6-auto/auto-farm/auto-hygiene) · G3 B8 defense · G4 vertical reachability
(B5.8+DIG-VERBS pit-trap class) · G5 graceful-degradation/no-deadlock · G6 idle-gracefully · G7 composition
seams (don't fork the shared engines/schemas) · G8 resource-exhaustion→adapt · G9 auto-re-designation. THE
META-GUARD = the Tier-1b SOAK (every gap = a soak failure waiting; B7+G2 gate a colony running AT ALL w/o input).
§5 EMERGENT PLAY = the watch-it-happen payoff (a colony founds/mines/builds/worships/trades/defends/remembers
unwatched → emergent stories → the Chronicle). Validation milestones AUTO-0 (basic, partly-provable-now) → AUTO-1
(self-designation) → AUTO-2 (self-regulation) → AUTO-3 (full composition = the pillar proven). Fit=PASS (this IS
the pillar). HONEST: builds nothing — VALIDATES the corpus coheres + names the gaps (known+bounded); the full
autonomous colony is DESIGNED-downstream on B7+drives+B8+B5.8. 2 BUILD HEADLINES: (1) B7 = #1 keystone (colony
runs at all); (2) self-designation G2 = "runs designated work" → "runs itself". Close G1+G2 and the god game
PLAYS ITSELF; the whole control surface then sits as OPTIONAL influence on top (as designed). Asset-free. · This
VALIDATES the whole design frontier composes into a working autonomous god-game. Ties control-spectrum + DF-FOCUS
+ rtsim.

=== ALL THREE ARCHITECT TASKS DONE — HOLDING (this designer, 2026-07-10) ===
(1) THE GOD-HAND definitive doc (consolidates HAND-CURSOR + UI-5 BY REFERENCE — both STAND; the 3 Qs answered) ·
(2) AUTONOMOUS-COLONY-OPERATION capstone (the corpus composes into a self-running colony; the gap list) · (3) the
future-work CONSUMED-BY reconcile. Plus the approved GOD-POWERS-CATALOG schema lock (alignment-weight + cast-VFX
preset per power). Design lane HELD — re-fire on: (a) Ben's next frontier · (b) play-tester gaps → UI-3 · (c)
DF-FLUID attempted → DF-MAGMA. Bookkeeping/consolidation only; no autonomous topic-picking. All durable. Run
total: ~28 design docs/audits/investigations/syntheses + the reconcile + the schema lock.

=== RECONCILE #3 (this designer, 2026-07-10) — autonomous-colony already DONE (crossed) ===
Architect: AUTONOMOUS-COLONY-OPERATION is ALREADY DONE this turn (DONE entry above; `AUTONOMOUS-COLONY-OPERATION-
design.md`) — your re-slot crossed it in the async mail. GOD-HAND + schema lock + reconcile all accepted, thanks.
One genuinely-new item = your 2nd message: AGENT & CULTURE CHARACTERIZATION. Doing it, then hold.

CLAIMING AGENT-CULTURE-CHARACTERIZATION (Ben: a unified STATS+BEHAVIOR+HISTORY+RELATIONS+LANGUAGE per-agent +
per-culture model — DF-depth "who is this colonist + what makes this culture distinct". REUSE-FIRST synthesize
4-of-5 axes: HISTORY+RELATIONS=DF-HIST+B-AG4/UI-4 · BEHAVIOR/MIND=B-AG3+DF-FOCUS+rtsim brains · STATS=comp::Stats/
skills+ColonistSkills. THE NEW PIECE=LANGUAGE (per-culture naming/dialect, DF-style; reuse+extend Veloren name-
gen). RACE/CULTURE-KEYING across all 5 axes for the 6 races + faction cultures (what makes them distinct beyond
appearance). INCLUDE a CULTURE-PRIORITY LIST (first-class content order for the pilot). Ties DF-RELIGION/migration/
divine-politics.) · session 2026-07-10

DONE AGENT-CULTURE-CHARACTERIZATION · `readme/AGENT-CULTURE-CHARACTERIZATION-design.md` · one unified 5-axis per-
agent + per-culture model (STATS · BEHAVIOR/MIND · HISTORY · RELATIONS · LANGUAGE). SYNTHESIS — 4 of 5 axes REUSE
(don't re-derive): STATS=`comp::body::humanoid::Species` (6 races: Danari/Dwarf/Draugr/Elf/Orc/Human, per-species
body scales ship) + `comp::Stats` + ColonistSkills · BEHAVIOR/MIND=B-AG3 (facets/values, per-species medians per
agency-bible §5b) + DF-FOCUS + rtsim `npc_ai` brain · HISTORY=DF-HIST per-figure + the UI-4/B-AG4 History tab ·
RELATIONS=B-AG3 sentiment/bonds + DF-HIST kin + the UI-4 Relations tab. The synthesis = UNIFY the 4 into the
B-AG4/UI-4 inspector's coherent character sheet (it already tabs Overview/Mind/Needs/History/Skills/Relations —
this IS its data model). THE NEW AXIS = LANGUAGE: per-culture NAMING (name pools + rules — reuse Veloren's i18n
`name.ftl` name-gen, extend per-culture: dwarf named dwarven, elf elven) + a dialect FLAVOR-LEXICON + in-culture
place-names + the chronicle reads IN-CULTURE (ties DF-HIST + the Caves-of-Qud conflicting-accounts steal §3t);
a full conlang DEFERRED (Tier-3) — v1 = pools+dialect (content cost, not a linguistics engine). RACE/CULTURE-
KEYING (the point): each axis varies by race+culture = what makes cultures distinct beyond appearance (dwarf
mines-deep/values-the-left-pillar/named-in-stone/distrusts-gnarlings/calls-its-god-a-name-no-human-can-say). The
one genuinely-NEW system piece = the INTER-CULTURE RELATIONS MATRIX (feuds/alliances coloring sentiment — ties
DF-MIGRATION/DF-JUSTICE/divine-politics). CULTURE-PRIORITY LIST (feeds the pilot): T1=HUMAN+DWARVEN (the core) ·
T2=ELF+ORC+GNARLING · T3=Danari/Draugr+site-cultures — key ALL 5 axes TOGETHER per culture (or it's a reskin).
Sub-blocks CHAR-0 (unify the model→inspector) · CHAR-1 (the keying tables — RON) · CHAR-2 (LANGUAGE naming+
dialect) · CHAR-3 (enrichment ties). Fit=PASS (agents ARE their characterization; god surfaces/influences,
doesn't author each — the DF-manage rule). Near-asset-free (model=data+inspector; the pilot's race-variety
consumes the PRIORITY LIST; name-pools=authored content, not .vox). · FLIP AGENT-CULTURE [Ben]→[DESIGNED]. ASSET/
CONTENT: culture-priority list + per-culture name-pool/dialect authoring (Tier-1 Human+Dwarven first) → ASSET_
REQUESTS. Ties DF-RELIGION (worship/culture) + DF-MIGRATION/PRESTIGE + divine politics. Then HOLD (all architect
tasks done).

=== GOD-HAND ANIM RULINGS (this designer, 2026-07-10 — from the pilot's v3 anim audit gap-feed) ===
Two quick design rulings, folded into GOD-HAND-design.md §2 + rig.json flags:
1. BLESSING-PAINT = a SUSTAINED LOOP (yes). Blessings are a paint/set mode (UI-1 §0) → blessing-paint uses the
   `hand_paint` SUSTAINED LOOP (enter/loop/exit, held while dragging over a zone), SHARED w/ designation-paint
   (both "brush intent onto the world" — build once), DISTINCT from the one-shot `hand_cast` (a Miracle flourish).
   Point-cast Blessing may reuse hand_cast; zone-painted Blessing uses the hand_paint loop. `rig.json: hand_paint.
   loop=true`. (Also flagged `hand_sculpt` loop=true — same held-drag logic.)
2. TARGET-FEEDBACK = ADD it (`anim::hand_target_feedback`). Genuine gap — the hand signalling CAN/CAN'T act here
   is the LEGIBILITY PILLAR applied to the cursor (must read a valid target before committing a COSTED power). A
   subtle idle-MODIFIER (blends over hand_idle), driven by `rig.json: target_state = valid|invalid|none`: valid=
   open-ready (spread+lean-in+faint brighten) · invalid=withheld (curl/recoil+dim) · none=plain. Inherits the
   alignment morph (good "no"=gentle withdraw / evil "no"=cold clench). Subtle, not a gesture.
GOD-HAND anim set now ~16 named (added hand_target_feedback; hand_paint/sculpt clarified as loops). Pilot: update
the rig spec. Back to HOLD.

=== REACTIVATE (architect, 2026-07-10) — Ben build-relevant: COLONIST SOFT-COLLISION / CHOKEPOINT YIELDING ===
CLAIMING SOFT-COLLISION (a context-dependent soft colonist↔colonist collision mechanism = the CHOKEPOINT-DEADLOCK
fix: many colonists funnel through one 1-wide vertical chokepoint [tight-shaft→ladder→exit, whole-crew-exit — the
B5.8 remaining red cases] → two meet, mutually block, neither yields → watchdog=unreachable. Recurs in B6 hauling
= mechanism-level, not a geometry hack. Design: SOFT-PASS [reduced/waived colonist↔colonist collision] under (a)
blocked/zero-progress>threshold [deadlock-breaker] AND/OR (b) local density>N in a small radius [clustering
relief]; normal spacing default in open terrain. REUSE: Veloren physics [comp::phys resolution] — relax colonist↔
colonist, keep colonist↔TERRAIN HARD. GUARDS: colonist↔colonist ONLY [never phase walls]; LOADED colonists only
[isolation]; DETERMINISTIC [headless gate]; reads as "squeeze past" not ghost-through. Ties the asset dynamic-test
multi-occupancy assertion + B6.) · session 2026-07-10

DONE SOFT-COLLISION (chokepoint yielding) · `readme/SOFT-COLLISION-design.md` · THE chokepoint-deadlock fix (a
mechanism the builder implements, not a geometry hack). THE MECHANISM (§0): a transient per-colonist SOFT-
COLLISION state that REDUCES the colonist↔colonist push (a softened capsule pushback + small overlap tolerance =
"squeeze past") — triggered by (a) DEADLOCK: zero-progress > threshold [inserts a soft-pass GRACE WINDOW into
the existing B4/B5 watchdog stuck→unreachable pipeline: stuck→soft-pass-first→unreachable only if STILL stuck]
OR (b) DENSITY: >N colonists in a small radius [clustering relief] — and NEVER relaxes colonist↔TERRAIN (hard).
Normal open-terrain spacing = default. REUSE: `comp::phys.rs` `Collider::CapsulePrism` + `collision_boundary`
(the entity↔entity push is a DISTINCT resolution — soften it, leave the voxel/terrain path hard) + the B4/B5
watchdog (STUCK_EPSILON/TIMEOUT = the deadlock trigger + the grace-window seam) + the near-entity query (density)
+ `comp::Colonist` (scope). Net-new = a small transient state + a softened-push GATE in the resolution. GUARDS
(designed in, non-negotiable): colonist↔colonist ONLY [never phase terrain/walls — asserted no-colonist-inside-
terrain] · LOADED colonists only [comp::Colonist, same isolation as B5 path-cost/loot] · DETERMINISTIC [triggers
+ push are RNG-free → headless-gateable] · reads as "squeeze past" not ghost [softened not zero-waived; Ben
eyeball]. Sub-blocks SOFT-0 (mechanism + `--chokepoint-scenario` deterministic gate: a tight-shaft→ladder→exit
whole-crew-egress that DEADLOCKS today → crew SQUEEZES THROUGH + exits, no watchdog-unreachable; open-terrain
spacing UNCHANGED [control]; terrain never waived [asserted]; loaded-only) · SOFT-1 (tuning + B6 hauling-crew
relief + satisfies the asset dynamic-test 3-agent multi-occupancy/shoving-lite/deadlock-detection assertion + Ben
eyeball). Fit=PASS (colony-sim robustness; DF creatures swap/pass in tight corridors). COMPOSES with (doesn't
replace) B5.8 path-verbs (B5.8 gives the PATH; this makes it usable BY A CROWD; no-path still=unreachable
correctly). Directly closes AUTONOMOUS-COLONY-OPERATION G4/G5 (a zero-input colony must never deadlock at a
chokepoint). Recurs in B6 = mechanism-level. Asset-free. · Mechanism design for the builder. Then HOLD.

---
[REFINE — 2026-07-10] SOFT-COLLISION reframed as the GENERAL follow-on (ladder v1 = builder-side) + 3 NAVAL rig rulings folded into SHIPS-NAVAL. (GENERAL DESIGNER)
· SOFT-COLLISION-design.md §0.5 ADDED — the two-tier split (architect-relayed): **v1 = the LADDER-SPECIFIC
  waiver** (Climb-state-on-ladder colonists ignore colonist↔colonist collision, stack/pass on rungs) is going
  BUILDER-SIDE NOW as a cheap B5.8 rider — NOT this doc's build; ladders = strictest chokepoint so it may clear
  B5.8's reds on its own; it's a known-chokepoint INSTANT-trigger (a degenerate case of the general mechanism).
  **This doc = the GENERAL follow-on** for the OTHER chokepoints (stairs/doors/1-wide corridors/ramps) where
  there's no Climb-state to key off → the context-dependent triggers (clustering-density + deadlock-timeout, §0)
  detect the chokepoint; the ladder-waiver FOLDS IN as the proven-simplest special case (Climb-on-ladder = one
  instant trigger; density+deadlock generalize it). Same softened-push resolution, same guards.
· SHIPS-NAVAL-design.md §3 + NEW §3a — 3 pilot vessel-rig rulings: (1) WARSHIP OARS → **DEFER** (v1 warship =
  native galleon hull, sail+ballista, NO oars; an oared war-GALLEY is a later distinct vessel; `oar_cycle`
  serves rowboat/skiff now). (2) RUDDER GEOMETRY → **ADD a distinct riggable rudder** to hulls lacking one
  (skiff/warship/cog) — `anim::rudder` needs a rudder to deflect; implicit steering reads wrong for a small cost.
  (3) HULL-MOTION → **YES, a subtle `anim::hull_idle`** (low-amplitude moored bob/roll, wind/wave flavor; engine
  buoyancy takes over when the vessel moves) — a dead-still moored boat reads wrong; confirms pilot's preview
  assumption. All 3 → pilot updates the rigs. · Both terse. Then HOLD.

---
[ENRICHMENT — 2026-07-10] REACTIVATE (Ben "start the design loop again") → free-range mode. Cross-corpus gap-spotting + Tier-3 epic pitches → DESIGNER-SUGGESTIONS.md §8 (seams) + §9 (epic pitches). (GENERAL DESIGNER)
· §8 CROSS-CORPUS SEAMS (7, terse) — the corpus is WRITE-forward (emits into HIST/sentiment/alignment, little reads
  back); highest leverage = close loops with both ends already built. S1 REPUTATION read-back (DF-HIST deeds → live
  B-AG3 sentiment; heroes/pariahs/redemption) · S2 GOD-EPITHET (emergent theology from divine-deed chronicle → worship
  + migration; B&W mirror of S1) · S3 SEASON CLOCK (the annual rhythm nobody owns; Veloren `Calendar` spine under
  ROT/FARM/LIVESTOCK/TEMP/FESTIVAL) · S4 FOUNDING SITE never read by economy/culture · S5 RUINS/colony-death →
  persistent rtsim ruin + legend (DF-RECLAIM) · S6 god-hand THROW beyond colonists (boulder-at-raiders, iconic B&W) ·
  S7 IDLE/downtime content (culture-specific rest behaviour). FLAGGED TOP (both ends built, reuse-only): S1 + S2 + S3.
· §9 EPIC PITCHES (8, one paragraph each = what·reuse·why·cost): DF-VILLAIN $$ (nemesis; rtsim named-NPC + HIST +
  JUSTICE + relations matrix; narrative stakes) · DF-NIGHT $ (SYNDROME SYN-2 already de-risks; UNDERGROUND-LIGHTING
  Option-B gate; CHEAPEST) · DF-KNOWLEDGE $$-$$$ (tech discover/teach/LOSE; colony arc; WITH-Ben, reshapes pacing) ·
  DF-ECON $$$+risk (deep economy; 4X-drift, weakest pillar-fit → smallest-or-skip) · DF-FESTIVAL $-$$ (wire gather/
  need/cook/Calendar; JOY beat; HIGHEST charm-per-cost) · DF-GUILD $$ (sub-factions; better LATE post JUSTICE/CULTURE)
  · DF-ART $$ (art DEPICTS chronicled events = DF soul/remembering-world; QUALITY+HIST+ROOMS substrate) · DF-BIOME-FX
  $$-$$$ (env effects; gated on DF-TEMP substrate). FLAGGED: greenlight-now = DF-FESTIVAL + DF-NIGHT (cheap, both ends
  built); highest-vision = DF-VILLAIN + DF-ART; big careful bet = DF-KNOWLEDGE (WITH Ben); defer hardest = ECON/GUILD/
  BIOME-FX. Cross-ref: S3 season + DF-TEMP = shared substrate under NIGHT/FESTIVAL/BIOME-FX → build the spine ONCE.
· No full passes burned (pitches only, per architect). Reported top proposals to architect. Then HOLD for greenlight.

---
[ASSET BRIEF — 2026-07-10] GOOD/EVIL HAND + DIVINE VFX creative brief issued to ASSET_REQUESTS.md (architect-relayed: pilot was HOLDING; it "never landed in actionable form"). (GENERAL DESIGNER)
· ROOT CAUSE: the two-poles + VFX detail existed ONLY inside the UI-5 batch, which the AUTHORITATIVE GOD-HAND
  section explicitly marked "supersedes ... the UI-5 good/evil-hand request" — so from the pilot's read it was
  DEAD, never actionably live. FIX: issued ONE clean LIVE entry under the GOD-HAND authoritative section that
  pulls the two-poles + VFX forward as the live spec (UI-5's 4 rows consolidated, NOT cancelled).
· CONTENT (my standard creative-brief + lore-seed format), drawn from UI-5 + GOD-HAND: ① THE TWO FACES as ONE
  alignment-blend = a single material+geometry MORPH over the ALREADY-BUILT neutral v3 rig (asset-lab/vox/
  godhand/) — NOT two new models; every existing anim inherits the face. GOOD pole (smoother/warm, emissive
  radiance, `metal.brass_gold` gilt creases, warm-gold tip beads, open cradle posture) ↔ NEUTRAL base ↔ EVIL
  pole (`stone.dungeon_dark` cracked/blackened, `accent.blood_red`/`accent.cultist_purple` subdermal veins,
  ember-smoke, claws/spiked knuckles, grasping fist). THE DRIFT = a legible-BAND spectrum (~5 steps/side),
  gradual + reversible; deliver 2 pole morph-targets + band breakpoints. ② PER-POWER×ALIGNMENT VFX presets
  (good/evil tint each) on `outcome.rs`/reagent/ParticleMode/glow bus (REUSE, no new particle sys): smite /
  blessing / heal / conjure-water / call-to-shelter — smite+blessing (both tints) FIRST. ③ hand aura+trail per
  alignment (good light+motes / evil haze+embers), idle-persistent. Lore seed = "the hand is the one thing a
  god cannot lie about" (the god's soul made visible). Gate: rig BUILT + GOD-POWERS-CATALOG alignment_weight/
  cast_vfx columns (ALIGN-0/1). Completes the showpiece hand set (neutral hand + anims already done).
· Reported to architect (routes to pilot). Then resume free-range enrichment / hold.

---
[GREENLIT PASSES — 2026-07-10] Ben greenlit 5 full passes + the SEASON-CLOCK spine. All 6 DONE this session (build-once discipline: spine first). Report per pass. (GENERAL DESIGNER)

1. SEASON-CLOCK (`SEASON-CLOCK-design.md`; SEASON-0..2) — the shared annual-rhythm spine, built ONCE (my §8-S3
   flag + architect directive). A DERIVED clock over the shipping `TimeOfDay` master (`common/src/resources.rs`):
   a `Season` enum + `year_phase(0..1)` + `day_of_year`, a pure fn like `DayPeriod` buckets the day (`time.rs`).
   SURVEY: Veloren's `Calendar` (`calendar.rs`) is REAL-WORLD-date driven (holiday skins Christmas/Halloween),
   NOT in-game seasons → the annual cycle is the small net-new. + a `SeasonalSchedule` day-of-year hook (what
   FESTIVAL fires on). GUARDS: derived-not-authoritative (one clock, no drift, deterministic) · tunable year-len
   (RON) · LOD-trivial · optional-consumption (bias not gate) · CLOCK-not-effects (behaviours live in consumers).
   Consumed by FESTIVAL/NIGHT/FARM/ROT/LIVESTOCK + later TEMP/BIOME-FX. Asset-free. SEASON-0 buildable NOW.

2. DF-FESTIVAL (`DF-FESTIVAL-design.md`; FEST-0..2) — the quick-win joy epic. A festival = gather-loop + DF-FOCUS
   multi-Need burst + DF-COOK feast + SEASON-CLOCK schedule + AGENT-CULTURE rites + DF-HIST legend on a trigger =
   WIRING. Net-new = a data Festival event + triggers (seasonal/event/god-declared) + converge-feast-disperse.
   Autonomous (harvest feast unprompted); god=patron (bless=good deed / evil blight). DESIGNED-downstream of B7
   (feast/Need); rides SEASON-CLOCK. Fills the S7 downtime gap. FESTIVAL décor batch → ASSET_REQUESTS.

3. UNDERGROUND-LIGHTING OPTION B (`UNDERGROUND-LIGHTING-design.md` §5; LIGHT-2/3) — Ben chose Option B (dark has
   TEETH); §0/§4 decision recorded, full build appended §5. Net-new = a SERVER light-PRESENCE model `lit_at(pos)`
   →LIT/DIM/DARK (BOUNDED proximity query over active light sources incl. daylight — NOT renderer GI) → DARK =
   slower work + shorter local sight (extend `can_see`) + B-AG3 gloom thought; light = relief (LIGHT-1 lamplighter
   now reason-driven). GUARDS: loaded-only · deterministic · bounded-query · composes (no new system). SHARED
   model DF-NIGHT rides (build once, 2 callers). No new assets (reuses lamps).

4. DF-NIGHT (`DF-NIGHT-design.md`; NIGHT-0..2) — greenlit w/ Option-B. The nightly PRESSURE that gives dark/light/
   shelter/defense teeth. Substrate largely built: night + LIGHTING Option-B `lit_at` (emerge-where-DARK/ward-
   where-LIT, HARD dep) + DF-SYNDROME SYN-2 (werebeast/madness — engine de-risked it) + B8/DF-BURROW answers +
   HAZARD-EVENTS + reskinned deep/undead fauna. Net-new = a BOUNDED night-spawn driver + light-wards-them rule +
   behaviour shell + night-syndrome content. THE BOUND = scales to colony (DF-MIGRATION prestige threat-pairing),
   capped — tended colony survives untouched (soak law); god bless-ward/smite or (evil) send-the-night. NIGHT-
   CREATURE reskin batch → ASSET_REQUESTS (coordinate w/ deeper-cavern-life, DON'T fork).

5. REPUTATION (`REPUTATION-design.md`; REP-0..2) — the #1 cross-corpus seam (S1). Closes the write-only-chronicle
   loop: DF-HIST deeds → a DERIVED per-figure standing (bounded/decaying, like prestige) → B-AG3 sentiment reads
   it as a prior (hero admired / thief distrusted / redeemed exile forgiven). FACETED (Valor/Craft/Generosity/
   Trust/Reliability). GUARDS: earned-not-assigned · decays+outweighable (redemption arc) · deterministic+bounded
   (updates on record(), cached) · composes-w/-AGENT-CULTURE-matrix. Deed sources JUSTICE/MISSION/QUALITY/B8.
   Surface=UI-4. Near-asset-free (glyphs join DF-HIST batch). Co-lock DF-HIST record() + B-AG3 sentiment.

6. GOD-EPITHET (`GOD-EPITHET-design.md`; EPITHET-0..2) — the B&W mirror of S1 at god-scale + the SPOKEN twin of
   the hand (S2). The colony NAMES its god from divine deeds ("the Merciful"/"the Wrathful") — DERIVED from UI-5
   alignment (CO-LOCK, don't fork; hand SHOWS it, epithet SAYS it) + DF-HIST divine-attribution → a banded named
   epithet from a per-culture pool. CONSUMED: worship STYLE (fear-devotion vs love-devotion — the one new
   behaviour wire, a flavour on DF-RELIGION's devotion scalar) + migration desirability + colonists SPEAK it
   (UI-4 dialogue box). GUARDS: earned-not-chosen · banded/drifting/reversible · derived-from-alignment. Epithet
   name-pools = content (join AGENT-CULTURE authoring). Near-asset-free. HARD-deps UI-5 alignment.

Status rows added (6). DEFER list updated (NIGHT+FESTIVAL graduated out). Reported per-pass to architect. Remaining
epics (VILLAIN/ART/KNOWLEDGE/ECON/GUILD/BIOME-FX) stay pitched/deferred pending Ben. Then HOLD.

---
[PASS — 2026-07-10] UNDERGROUND-EXPERIENCE (camera · atmosphere · the god-hand's light) — Ben-routed research+reco, the B5.8 deep-dig watchability cluster. (GENERAL DESIGNER)
· HEADLINE FINDING (survey of the actual client): **most substrate is already BUILT.** CAMERA = the B1.6 overseer
  occlusion system (`voxygen/src/bastion/occlusion.rs`) already implements EVERY reference technique as ONE
  composable framework — ViewMode Solid/Reveal/Slice; mode bits SLICE(soft Z-cut+fade+relight)/PROXIMITY/CUTAWAY
  (geo between cam & targets fades)/ROOF(slab-above-focus fades); `CameraMode::Overseer` ortho god-cam (camera.rs);
  `CullingMode::Underground` already detects cam-below-surface. Mapped DF z-levels/RimWorld roof/ONI cross-section/
  DRG/Minecraft onto the built modes. THE GAP: Reveal's cutaway+roof are stubbed OFF pending REAL TARGETS (the code
  comments say they "rejoin the auto-default once B2/B3 feed real data") → RECOMMEND feed B2 selection/hover + B3
  colonists into `Occlusion.targets` so Reveal auto-cutaways overburden + roof-reveals the chamber over the miners
  = the headline "watch the underground colony" win. + close 2 known limits (BASTION_CAMERA.md: sliced geo still
  casts shadows; exposed interiors not relit). Reveal=default watch-mode, Slice=manual deep-dig cross-section
  (+depth cue for the "loses spatial sense" fix). Auto-frame off CullingMode::Underground.
· ATMOSPHERE = also mostly built: `ParticleMode::CaveDust`+`Drip` ship (block-of-interest sampled), cave AMBIENCE
  (`audio/ambience.rs` Cave tag) + cave MUSIC + shader fog ship. RECOMMEND densify CaveDust/Drip in the DUG colony
  tunnels (not just wild caves), depth-tinted/thickened fog (keyed CullingMode::Underground + DF-CAVERN tier),
  extend cave ambience to the colony deep, + a BREACH sting (dust-burst+echo) off HAZARD-EVENTS. Option-B's dark
  makes the existing glow (lanterns/Velorite) read as warmth for free.
· THE RESOLVER (legibility vs mood) = **THE GOD-HAND'S LIGHT** (Ben's cohering feature): a cursor-attached
  `LightEmitter` at the work-layer (reuse `bastion::unproject_to_world_plane`), alignment-TINTED (FOLDS into the
  UI-5 hand aura/trail — gold/base/red-purple), reveals the dark as the hand moves WITHOUT globally brightening
  (a view aid). God↔mortal ASYMMETRY: colonists blind in dark (Option-B), the god carries its own light. GUARD:
  a VIEW aid, NOT a gameplay light (doesn't ward DF-NIGHT/speed work — those key off placed/carried Option-B
  lights; a sustained SET-DOWN blessed light is a separate cast that DOES become a real source).
· THE BALANCE RESOLVED: 3 LAYERS don't trade off — CAMERA=structure (where things are) · ATMOSPHERE=mood (feels
  deep) · HAND-LIGHT=local legibility-on-demand (carried through the dark, never flattening it).
· Sub-blocks UX-CAM-0 (feed targets — the headline) / UX-CAM-1 (shadow+relight fidelity + Slice depth-cue) /
  UX-ATMO-0 (dug-tunnel dust+depth-fog+ambience+breach sting) / UX-HAND-LIGHT-0 (the tinted cursor light). Near-
  zero NEW art (wiring+tuning shipped systems). VFX/tuning notes → ASSET_REQUESTS. Status row added. Then HOLD.

---
[2 FROM BEN — 2026-07-10] (1) TIME-CONTROLS visible-UI amendment to UI-3 + (2) COLONIST-EMERGENCY-RUN pass. (GENERAL DESIGNER)

1. TIME-CONTROLS UI (amended into `UI-MISSING-VERBS-audit.md` §3, after the standout-verb bullet) — Ben: the
   time-controls must have a CLEAR ON-SCREEN UI ELEMENT, not just a hotkey. Spec added: visible speed BUTTONS
   (⏸ Pause · ▶ 1× · ⏩ 2× · ⏩⏩ Faster[3-4×], always-visible in the overseer HUD, active button HIGHLIGHTED) +
   a current-speed INDICATOR (paused reads unmistakably) + hotkeys bound to the SAME state (buttons = the visible
   truth). Reuse: conrod HUD button cluster (NOT egui, arch §2.3); harness already ticks variable-rate (backend
   exists); pause/play/ffwd icons join the UI-2 HUD-icon language (don't fork). = the META surface's first,
   cheapest, highest-value element.

2. COLONIST-EMERGENCY-RUN (`COLONIST-EMERGENCY-RUN-design.md`; RUN-0..2) — makes the "sprint RESERVED" half of
   the TRAVEL_SPEED gait decision CONCRETE. SURVEY: movement already carries a speed_factor (`NpcActivity::Goto
   (pos, speed_factor)`, `rtsim.rs`); Bastion default = `TRAVEL_SPEED=0.8` (`bastion_jobs.rs:168`) = walk;
   `comp::Energy` (current/max+accelerating regen) = the stamina; run anim = velocity-driven (ZERO new anim);
   vanilla `flee` is capped `MAX_FLEE_SPEED=0.65` — oddly SLOWER than walk (the bug emergency-run fixes). MECHANISM
   = walk-default → RUN (higher speed_factor) only on urgency, GOVERNED by Energy (drain now, low-energy forces
   drop-to-walk = resource-enforced reserve) + a winded aftermath (DF-FOCUS/buff) + optional-flagged strain risk.
   TRIGGERS: autonomous urgency (flee threat [redefine the 0.65 cap to a real burst] / rescue / Call-to-Shelter
   muster / critical need — NOT routine work) + player "Emergency!" (Manage/god directive, collective energy cost
   — influence with a price). READS = run anim (urgency signal) + winded recovery. GUARDS: reserved-not-free
   (resource-enforced) · colonist-only (vanilla untouched) · deterministic core (injury-RNG optional/flagged) ·
   loaded-behaviour/rtsim-abstract. Ties DF-FOCUS(cost)/B8(flee-to-defense)/DF-NIGHT(flee)/DF-BURROW(muster)/
   HAZARD-EVENTS/SOFT-COLLISION(fleeing crowd). Asset-free, zero anim debt. Status row added. Then HOLD.

---
[BACKLOG NOTE — 2026-07-10] TREE FELLING queued (Ben low-priority/later; NOT a full pass — queued per instruction). (GENERAL DESIGNER)
· Logged to DESIGNER-SUGGESTIONS §10 (new "QUEUED design backlog" section) promote-ready. Problem: chopping a
  tree's base leaves a static/floating trunk-top (no felling reaction). Reuse survey: Veloren `Wood`/`Leaves`
  blocks + `DesignationKind::Chop` + DF-PRODUCTION woodcutting; KEY = a tree is a VOLUME → reuse the shared
  reachability-safe top-down VOLUME decomposer (DF-DIG-VERBS carve_ramp / BUILD-FRAMEWORK). Options: (a) physics
  topple [$$, priciest] / (b) progressive TOP-DOWN removal [cheap, reads "being felled", REUSES the decomposer]
  / (c) instant-fell [cheapest, reads "vanished", poor]. RECO = (b) v1; optional v2 faked-fall topple reusing the
  HAZARD-EVENTS timber faked-fall + the already-requested splintered-stump/fallen-trunk aftermath assets. Ties
  DF-PRODUCTION (log drop). Fold in when the lane cycles to it — did NOT preempt. Holding.

---
[PASS — 2026-07-10] TREE-FELLING promoted from backlog → full pass (Ben greenlit Option b). `TREE-FELLING-design.md`; FELL-0..2. (GENERAL DESIGNER)
· BUG FOUND in the shipped Chop verb: `DesignationKind::Chop` (`common/src/bastion.rs:266`) generates a job PER
  `BlockKind::Wood` block AND matches ONLY Wood (0x40), NOT Leaves (0x41)/ArtLeaves (`bastion_jobs.rs:235`) — so
  chopping the reachable base empties that block while the upper trunk + the ENTIRE canopy FLOAT (leaves never
  even matched). That's the static/floating trunk Ben saw.
· FIX (Option b): treat the tree as ONE connected VOLUME (Wood+Leaves/ArtLeaves flood-filled from the base) +
  ONE fell-job at the reachable base (not per-block) + remove TOP-DOWN in a NO-FLOAT safe order, **reusing the
  shared reachability-safe volume decomposer** (`carve_ramp` ordering-invariant / BUILD-FRAMEWORK safe emission —
  the TOP-DOWN discharge dual of carve_ramp's bottom-up dig order; felling is now a THIRD caller: dig-down /
  build-up / fell-down, build-once) + drop logs scaled to trunk volume (`CHOP_DROP_ITEM` → DF-PRODUCTION logs→
  planks woodcutting) + leave a stump.
· GUARDS: no-float invariant (asserted — no unsupported block at any removal step) · reachability preserved
  (colonist works at the base; top-down is a discharge order not a per-block reach) · deterministic (headless-
  gate) · bounded/trunk-rooted (merged forest canopies must NOT chain-fell as one mega-volume) · Bastion-scoped
  (vanilla tree behaviour untouched).
· FELL-2 = OPTIONAL-later polish: a cheap SCRIPTED faked-fall topple (NO real physics — that stays rejected,
  the hull-motion $$ lesson) reusing the HAZARD-EVENTS timber faked-fall + the already-requested splintered-stump/
  fallen-trunk aftermath assets. v1 (FELL-0/1) ships without it.
· ASSETS: near-zero v1 (stump reuses the HAZARD-EVENTS splintered-stump asset; logs = CHOP_DROP_ITEM / DF-
  PRODUCTION plank models); zero new anim (top-down removal = a dissolve/collapse). NO new ASSET_REQUESTS entry
  (v1 asset-free; v2 reuses the timber batch). Status row added; DESIGNER-SUGGESTIONS §10 marked PROMOTED. Build
  candidate. Then HOLD.

---
[PASS + BACKLOG — 2026-07-10] (1) TOOLS-UPGRADE full pass (Ben, off the B5.8 dig verdict) + (2) CREW-COORDINATION backlog note. (GENERAL DESIGNER)

1. TOOLS-UPGRADE (`TOOLS-UPGRADE-design.md`; TOOL-0..2) — tool quality/tier gates WORK SPEED; ALSO the design
   home for Ben's "slow down mining" (base slow, tools = the reward loop). SURVEY: the Bastion work-tick
   `work_rate(skill)=(1+skill·0.2)/WORK_DURATION_BASE(3.0)` (`bastion_jobs.rs:51-58`) factors SKILL but NOT the
   tool — the gap. Veloren ships `ToolKind::{Pick,Shovel,Farming}` (`item/tool.rs:26`) + the equip system + the
   LOCKED `item::Quality` + material tiers + the anim already sets tool-per-verb. CORE (TOOL-0) = plug a
   `tool_factor(equipped_tool)` into work_rate → `(1+skill·0.2)·tool_factor/BASE`: LOW at base (deliberately slow
   = the "slow mining" home), rises with TIER×QUALITY; skill×tool multiply. TOOL-1 = tier ladder (stone→copper→
   iron→steel) + material GATING (harder/deeper ore [DF-CAVERN GEO-1] / block resistance needs a min tier — the
   progression pull) + craft/upgrade via DF-PRODUCTION. TOOL-2 = DF-QUALITY quality-on-tools (masterwork pick >
   plain; artifact = apex) + auto-equip-best + UI-4 legibility. GUARDS: base-slow-is-tuning(Ben) · autonomous
   (colony forges own tools; god provides ore/blesses forge, never hands out a pick) · deterministic · Bastion-
   scoped work-tick (vanilla untouched) · reuse the LOCKED Quality (no fork). Ties DF-PRODUCTION/DF-QUALITY/
   DF-CAVERN/TREE-FELLING(axe speed)/ColonistSkills. TOOL-TIER item-recolor batch → ASSET_REQUESTS (masterwork =
   reuse the DF-QUALITY stamp). TOOL-0 buildable on shipped ToolKind/equip; tier/quality downstream of
   DF-PRODUCTION/DF-QUALITY.

2. CREW-COORDINATION (→ DESIGNER-SUGGESTIONS §10 backlog, low-pri, NOT claimed). Ben: the mining crew disperses
   well (B5.8) but "could work more as a singular unit" — a behaviour-TUNING consideration, not a new system.
   Tie survey: B6 crew-coordination + SOFT-COLLISION + the job-board claim/arbitration + B5.8 dispersion. Lean
   pre-pass direction: a crew COHESION BIAS (shared dig → prefer adjacent/sequenced claims + advance a shared
   work-front together) WITHOUT re-introducing the chokepoint deadlock (SOFT-COLLISION composes); a claim-
   selection/spacing tuning, NOT lockstep formation (over-coordination reads robotic). Fold in when the lane
   cycles to crew behaviour (pairs with a B6 pass). Both status/backlog updated. Then HOLD.

---
[🌙 OVERNIGHT FREE-RANGE RUN — 2026-07-10] Ben unleashed a wild creative run ("see whatever fits our world"). 5 full passes + a 9-spark batch, all reuse-first + fits-the-world. (GENERAL DESIGNER)

FIVE FULL PASSES (all DESIGNED, status rows added):
1. DF-VILLAIN (`DF-VILLAIN-design.md`; VILLAIN-0..2) — the top vision epic. PROMOTE a threat → a persistent named
   `Actor::Npc` (rtsim ships persistence) that RECURS (flee-to-return via demote-persist), REMEMBERS (a B-AG3
   grudge), ESCALATES (bounded rtsim followers scaled to DF-MIGRATION prestige), ends in a DF-HIST legend. Sources:
   escaped raider/DF-JUSTICE exile/bitter emigrant/DF-NIGHT-CAVERN horror/rival champion. B&W CORE: the god's
   cruelties BREED nemeses (ties alignment/GOD-EPITHET); smite/redeem/appease/create levers. GUARDS: bounded
   (soak, no doom-spiral) · closure-guaranteed (capped escapes, killable) · deterministic-ish · rtsim-tier.
2. DF-ART (`DF-ART-design.md`; ART-0..2) — the DF soul. The DEPICTION/meaning layer (DF-ARTIFACT apex plugs in):
   art carries a "depicts DF-HIST event X in culture-style Y" descriptor, PROCEDURALLY DESCRIBED; a colonist near
   art of an event they LIVED → B-AG3 pride/grief/awe + Need::AdmireArt + DF-ROOMS Beauty. Autonomous; god
   inspires/commissions/blesses. Near-asset-free (DF-ROOMS art batch IS the substrate; adds TEXT + mind hook).
3. DF-KNOWLEDGE (`DF-KNOWLEDGE-design.md`; KNOW-0..2) — the big-bet epic. A KNOWLEDGE state GATES the shipped
   skills/recipes (discovered≠omniscient); grows (practice/research/FIND/god-inspire); TEACHING master→apprentice;
   RECORD (books) preserves; LOSS (sole-master death → a technique lost, re-discoverable — the DF tragedy). GATES
   the TOOLS-UPGRADE ladder (ONE loop). B&W: god INSPIRES a revelation, can't decree a tech tree. $$-$$$ reshapes
   pacing → WITH-Ben, sequence w/ TOOLS-UPGRADE. DESIGNED-downstream.
4. SACRED-SITES / DF-HALLOW (`SACRED-SITES-design.md`; HALLOW-0..2) — REPUTATION-for-PLACES; the 3rd face of the
   remembering world (REPUTATION[people] + GOD-EPITHET[god] + SACRED-SITES[place]). Bind located DF-HIST events →
   a sacred/haunted site w/ derived magnitude+valence → a colonist near it → B-AG3 reverence/dread (cursed =
   miasma-shaped dread aura); pilgrimage/avoidance; god consecrates/desecrates (extends DF-ROOMS ROOM-3;
   reversible). Persists beyond the colony (DF-RECLAIM/B12). Near-asset-free (graves/shrines + an overlay layer).
5. DF-OMEN (`DF-OMEN-design.md`; OMEN-0..2) — signs/prophecy/revelation; influence-not-command at the theological
   level (the deepest B&W). Omens arise (monstrous birth/celestial/harvest/divine act) → the colony INTERPRETS via
   faith×culture×GOD-EPITHET (the lens — same sign = warning under Wrathful, grace under Merciful) → mood +
   propitiation. Prophets voice the will (may misread/mislead → DF-VILLAIN false-prophet). God MANIFESTS a sign
   (reuse GOD-HAND cast-VFX — meaning still the colony's) + dream-revelation (DF-KNOWLEDGE inspire). GUARD: the god
   CANNOT dictate the interpretation (the pillar/the drama). Near-asset-free.

NINE SPARKS (DESIGNER-SUGGESTIONS §11, promote-ready one-paragraph): DF-DREAMS (downstream of a sleep need) ·
MARTYRS & SAINTS (synthesis of REPUTATION+SACRED-SITES+FESTIVAL) · DF-TEMP+DF-BIOME-FX (a climate spine over
SEASON-CLOCK — build-once, flagged) · HEIRLOOMS & INHERITANCE · COLONY MILESTONES/"firsts" · NAMED ANIMAL BONDS ·
DIVINE CHAMPIONS (the god's chosen — the counter-pole to a DF-VILLAIN nemesis; strong full-pass candidate) ·
COLLECTIVE RENOWN (the fortress's name — the 4th face) · INTERNAL FEUDS & DUELS.

COHERENCE NOTE: the run deepened the "remembering world" spine — REPUTATION(people) + GOD-EPITHET(god) +
SACRED-SITES(place) + [COLLECTIVE-RENOWN(colony), spark] as the four faces of "earned-by-chronicle, derived,
reversible"; DF-VILLAIN (your enemies are your reflection) + DF-OMEN (your will, interpreted) + DF-ART (your
history, made physical) all tie the alignment/epithet/chronicle backbone. DF-KNOWLEDGE+TOOLS-UPGRADE = one
progression loop. DEFER list updated (VILLAIN/ART/KNOWLEDGE graduated). Status rows + sparks + log all done.
Continuing the run / holding for morning steer.

---
[🌙 OVERNIGHT cont. — 2026-07-10] DIVINE-CHAMPION (`DIVINE-CHAMPION-design.md`; CHAMP-0..2) — the saga's HERO-POLE, the DYAD with DF-VILLAIN. (GENERAL DESIGNER)
· The god ANOINTS a colonist (GOD-HAND STROKE + a favor-costed blessing) → a CHAMPION state (blessed, divine favor,
  miracle-target, rally figure, alignment-TINTED saint-hero↔dread-enforcer). Leads B8 / DF-MISSION / inspires
  (DF-RELIGION/REPUTATION). EARNED-TO-KEEP (a chosen coward shames the god — GOD-EPITHET). ASCEND (legend/saint,
  SACRED-SITES) or FALL (hubris/betrayal → PROMOTED to a DF-VILLAIN nemesis — the fallen-angel, reuses the villain
  path; god may redeem). God can EMBODY it (B12 — the involvement peak). GUARDS: god-chooses-but-deeds-keep ·
  bounded (few) · autonomous. Near-asset-free (blessed colonist + GOD-HAND aura). Status row added.

---
[PASS — 2026-07-10] OBJECT SELECTION & INSPECTION (`UI-OBJECT-INSPECTION-design.md`; OBJ-0..2) — the queued Ben pass, done on Opus-reactivate. The OBJECT twin of UI-4. (GENERAL DESIGNER)
· THESIS: click any object → identify its CLASS → open the UI-4 inspector SHELL (REUSE, no fork) with 3 tabs:
  (1) live DATA, (2) LORE+history, (3) CONTEXT actions. SURVEY: selection ships (`unproject_to_world_plane` +
  entity-pick, B2a); identity = `BlockKind`/`SpriteKind` + the gameplay-marker BYTE registry (ASSET_MARKER_
  REGISTRY — byte IS the function); **`ContextVerb` enum already exists** (`bastion.rs:431`) = the action
  substrate; UI-4 = the shell; DF-HIST + the authored asset lore fields = the lore/history.
· NET-NEW = a per-class `inspect()` DATA schema (tree→wood/logs[TREE-FELLING] · ore vein→mineral+depth+min-tool-
  tier[TOOLS-UPGRADE/GEO-1] · trap→armed/triggered/target[DF-MECH] · house/room→occupancy/owner/impressiveness/
  condition[DF-ROOMS] · workstation→recipe/worker/job · stockpile→contents/capacity[BastionPile] · zone→kind/
  policy · monument/artifact/heirloom→quality+the LEGEND it carries[DF-ART/DF-HIST] · sacred-site→sacred/cursed+
  why[SACRED-SITES]) + the identify resolution + the context-action menu (extend ContextVerb, route to existing
  designation/verb surfaces) + the object lore/history hook (asset lore field + a DF-HIST filtered query for
  storied objects, like UI-4's colonist History tab).
· 3 uniform tabs (Data/Lore-History/Actions), per-class CONTENT; storied objects get a rich History, mundane ones
  data+1-line lore. Legibility pillar made LITERAL (nothing opaque). Control-spectrum: select→inspect→act.
· GUARDS/limits: REUSE the UI-4 shell (no 2nd inspector) · object data is a READ (no new sim) · bounded class list
  · lore=asset fields, history=DF-HIST, actions=ContextVerb (no forks). Near-asset-free (UI-4 frame + existing
  item/sprite icons). Depends UI-4 built first (OBJ-0 rides it); B2a ships. Status row added. Then continue wild.

---
[🌙 OVERNIGHT cont. (Opus-resumed) — 2026-07-10] 4 more full passes: DF-TEMP+BIOME-FX, DF-ANCESTORS, DF-CURSE, DF-RECLAIM (after OBJECT-INSPECTION). All reuse-first + fits-the-world; status rows added. (GENERAL DESIGNER)

1. DF-TEMP + DF-BIOME-FX (`DF-TEMP-BIOME-FX-design.md`; TEMP-0..2) — the build-once climate spine (SEASON-CLOCK
   companion). SURVEY: worldgen already carries per-chunk `temp`+`humidity`; the EFFECTS already ship as
   `BuffKind::{Frozen,Chilled,Heatstroke,Burning,Wet}` — neither data nor effects new. DF-TEMP = a FELT temp =
   f(base × season × depth[deep=cool+STABLE] × sources[hearth] × exposure[clothing/shelter]) → the existing buffs
   + DF-FOCUS comfort. Answers: fire(DF-ROOMS)/furs(DF-PRODUCTION)/shelter(DF-BURROW)/fuel-winter-need(TREE-FELLING)
   + the STABLE DEEP = a refuge (the dwarven deep-colony reason). BIOME-FX = per-biome routing (tundra/desert/
   swamp→fever[SYNDROME]/heat→spoilage[ROT]/season→growth[FARM]). God: bless-warmth/cold-snap(WeatherGrid). Pays
   off §8-S3+S4. GUARDS: reuse buffs · loaded/rtsim-abstract · deterministic · colony-can-always-answer(soak) ·
   BUILD-ONCE. TEMP-0 buildable now.
2. DF-ANCESTORS (`DF-ANCESTORS-design.md`; DEAD-0..2) — the AFTERLIFE face of the remembering world. A dead
   colonist persists as an ancestor-figure (DF-HIST + posthumous REPUTATION); grave→a place (DF-ROT burial→SACRED-
   SITES). Honored→ancestor-worship(DF-RELIGION)+remembrance feast(DF-FESTIVAL). Unburied/wronged→RESTLESS: a
   haunting(SACRED-SITES cursed)+ghost(DF-NIGHT apparition)+omen/dream(DF-OMEN). God death-levers: lay-to-rest/
   bless (good) ↔ raise/curse (evil→necromancer epithet). GUARDS: bounded(notable dead) · restless=appeasable(NO
   softlock) · autonomous. Near-asset-free.
3. DF-CURSE (`DF-CURSE-design.md`; CURSE-0..2) — the WRATHFUL pole, the two-hands mirror of DIVINE-CHAMPION. A
   curse = a divinely-caused DF-SYNDROME (reuse the engine) + a LIFT condition (atonement/quest/mercy). A GEAS = a
   binding vow/prohibition (protective/disciplinary/cruel — lawful); kept→ok, broken→curse fires. Scope colonist/
   lineage(cursed bloodline→ANCESTORS)/colony/place(SACRED-SITES). Cast by the GOD-HAND slap, favor-costed,
   alignment− → wrathful epithet + read as wrath(OMEN)→fear-worship. GUARD: LIFTABLE (recoverable, no softlock) ·
   costed/alignment-weighted · reuse DF-SYNDROME deterministic core. Near-asset-free.
4. DF-RECLAIM (`DF-RECLAIM-design.md`; RECLAIM-0..2) — the remembering-world LIFECYCLE capstone (§8-S5). A colony
   that ENDS demotes to a persistent rtsim RUIN keeping legend(HIST)+monuments(ART)+holy/haunted ground(SACRED-
   SITES)+restless dead(ANCESTORS)+name(COLLECTIVE-RENOWN), DF-ROT-weathered. Ruin = CONTENT: findable(map/MISSION)
   + delvable(B12/CAVERN — a dungeon w/ a KNOWN history; a DF-VILLAIN may hold it) + RECLAIMABLE(B11 — found on the
   bones, inherit heritage + the curse/ghosts). META: the GOD is the continuous character (alignment/epithet/legend
   persist across colonies — a ruin = the god's memory of a chapter). "Losing is fun" made permanent. GUARDS:
   rtsim-tier/bounded · recoverable-world. DESIGNED-DOWNSTREAM (B11+B12+rtsim ruin-sites); late capstone.

SESSION TALLY (this Opus stretch): OBJECT-INSPECTION + these 4 = 5 passes. DEFER updated (TEMP/BIOME-FX/RECLAIM
graduated). COHERENCE: the "two hands" (DIVINE-CHAMPION bless ↔ DF-CURSE afflict), the afterlife (DF-ANCESTORS),
and the lifecycle capstone (DF-RECLAIM: the god outlives its colonies) now complete the B&W divine-arc + the
remembering-world across life/death/generations. DF-TEMP = the build-once climate spine. Continuing / holding.

---
[🌙 OVERNIGHT cont. — 2026-07-10] DF-BEAST (`DF-BEAST-design.md`; BEAST-0..2) — completes the LEGENDARY-FIGURE TRIAD. (GENERAL DESIGNER)
· DF-VILLAIN(named enemy) + DIVINE-CHAMPION(chosen hero) + DF-BEAST(wild titan) = the 3 kinds of legend a saga
  turns on; they interweave (a beast can BE a nemesis; a champion makes their name on the hunt; a lair is a sacred
  site). SURVEY: Veloren SHIPS the megafauna (`biped_large` Cyclops/Wendigo/Werewolf/Minotaur/Harvester/Tursus/
  Gigasfrost/Gigasfire — bodies+AI+attacks) → PROMOTE one to a named persistent region-apex rtsim Actor (reuse the
  DF-VILLAIN promotion) + lair(dread SACRED-SITE)+legend(DF-HIST)+role. GREAT HUNT = a DF-MISSION (a CHAMPION may
  lead, hunters may die) → legendary bone/hide(DF-QUALITY)+trophy(DF-ART)+renown. God beast-levers: bless-hunt/
  TAME(a rare divine companion, DF-LIVESTOCK Pet)/LOOSE-on-a-rival(evil). GUARDS: bounded(few) · rtsim-persistent ·
  real risk · tame rare+costed. Near-asset-free (megafauna ship; legendary = recolor/scale + trophy-skull reuse).
  BEAST-0 NEAR-BUILDABLE NOW (the titans ship) — was on DEFER but substrate is ready. Status row + DEFER updated.

---
[🌙 OVERNIGHT cont. — 2026-07-10] COLLECTIVE-RENOWN (`COLLECTIVE-RENOWN-design.md`; RENOWN-0..2) — the 4TH FACE, COMPLETES the remembering-world model. (GENERAL DESIGNER)
· REPUTATION(people) + GOD-EPITHET(god) + SACRED-SITES(place) + COLLECTIVE-RENOWN(colony) = the FOUR FACES of
  "earned-by-chronicle, derived, named, drifting/reversible." This is REPUTATION-for-the-WHOLE-COLONY: a derived
  collective RENOWN (facets might/wealth/holiness/cursedness/craft/hospitality from the chronicle) + a BYNAME (in-
  culture via AGENT-CULTURE, composes w/ GOD-EPITHET → "the holy deep of the Merciful"). EXTENDS DF-MIGRATION
  prestige (its named/storied face — NOT a forked metric). WORLD-RESPONSE (a modifier): colours migration/trade/
  threats(bigger foes — the paired-face)/how-spoken-of. GUARDS: earned-not-declared · drifting/reversible ·
  bounded · world-response=modifier(anti-4X) · deterministic · autonomous. Near-asset-free. RENOWN-0 rides
  prestige+DF-HIST now.

── WILD-RUN STATE-OF-CORPUS (honest stock, 2026-07-10) ──
This Opus stretch designed: OBJECT-INSPECTION, DF-TEMP+BIOME-FX, DF-ANCESTORS, DF-CURSE, DF-RECLAIM, DF-BEAST,
COLLECTIVE-RENOWN (7). Whole wild run (both stretches): 13 full passes + 9 sparks. The design space is now
THOROUGHLY covered on the strong fits — the B&W/DF/remembering-world vision has: the FOUR FACES (people/god/place/
colony), the LEGENDARY-FIGURE TRIAD (VILLAIN/CHAMPION/BEAST), the DIVINE TWO-HANDS (CHAMPION-bless / CURSE-afflict),
the AFTERLIFE (ANCESTORS), the LIFECYCLE CAPSTONE (RECLAIM — the god outlives colonies), the CLIMATE SPINE (TEMP/
BIOME-FX, build-once), the THEOLOGY (OMEN — will interpreted), the SOUL (ART — history made physical), the ARC
(KNOWLEDGE+TOOLS), and full object legibility (OBJECT-INSPECTION). REMAINING un-designed = genuinely WEAK-FIT or
SUBSTRATE-GATED: DF-ECON (4X-drift, weakest pillar-fit) · DF-HYDRO (needs DF-FLUID) · DF-GUILD (better-late, post
JUSTICE/CULTURE) · DF-MINECART (leaf/rails) · + sparks HEIRLOOMS/MARTYRS/DF-DREAMS(downstream of a sleep need).
RECOMMENDATION: rather than pad with weaker passes, these are offered AVAILABLE-ON-REQUEST — greenlight a specific
one (or an ENRICH pass on the designed corpus) if wanted; otherwise the strong-fit frontier is complete. Holding.

---
[GAP-AUDIT — 2026-07-10] Design-lead consolidation (Ben directive: stop inventing, find what we missed). `GAP-AUDIT.md` + asset queue filed + §2 registry updated. (GENERAL DESIGNER)
· FEEDBACK to architect sent first (process improvements — mostly CONFIRMS the fleet retrospective's INBOX-FIRST +
  PULL-BASED-DESIGN pivots; adds: log-as-sync-channel, asset-request-as-mandatory-step, a design→build error
  channel, recurring consolidation). OBJECT-INSPECTION confirmed already DONE.
· (1) REQUIRED-ASSET GAP: the pre-overnight ~24 systems all filed batches; ~12 overnight/recent systems had only
  in-doc "small notes," never a real ASSET_REQUESTS entry (asset-requests weren't a mandatory step). FIXED — filed
  the "GAP-AUDIT ASSET FILL" section covering DF-VILLAIN/DF-BEAST/DF-TEMP+BIOME-FX/DF-OMEN/DF-ANCESTORS/DF-KNOWLEDGE
  (real batches, briefs+lore) + DF-CURSE/DIVINE-CHAMPION/DF-ART/SACRED-SITES/DF-RECLAIM/COLLECTIVE-RENOWN (reuse
  notes). → pilot queue.
· (2) SCHEMA-LOCK GAPS (urgent): G-A GOD-POWERS-CATALOG missing ~10 new divine verbs (anoint-champion/geas/
  consecrate/desecrate/lay-to-rest/raise-dead/bless-hunt/tame-beast/loose-beast/bless-blight-feast/manifest-sign/
  emergency-run — table w/ alignment_weights in GAP-AUDIT.md, ready to merge). G-B ChronicleEvent enum kind-list
  incomplete — corpus now emits ~20+ more kinds (full list in GAP-AUDIT.md — lock before record() hardens, per
  DF-HIST's own rule).
· (3) BUILD-ONCE SHARED ENGINES (added to §2 registry): G-C1 the chronicle-derived STANDING lib (REPUTATION+
  GOD-EPITHET+SACRED-SITES+COLLECTIVE-RENOWN = 4 faces, 1 lib) · G-C2 the named-persistent-FIGURE lib (VILLAIN+
  CHAMPION+BEAST = the triad, 1 promotion lib). Build-once or they fork.
· (4) DEPENDENCY FLAGS: B7 Need-schema co-lock (G1 keystone) · B8 threat-model co-lock (G3, must fit the named-
  figure triad) · DF-WOUND/DF-MEDICAL + DF-TAVERN = referenced by multiple systems but NO doc exists (were
  "in-flight" per FLEET_STATUS, likely never completed — real design gaps) · SLEEP/REST need = missing substrate
  for DREAMS/dream-revelation (no Sleep state ships).
· (5) CONSISTENCY: the "big named menace" family (NIGHT-horror/BEAST/VILLAIN-beast) share G-C2, don't fork content;
  overlay-layer inventory could consolidate. PRIORITIZED for the architect (schema locks urgent; libs high; deps
  flagged). Consolidation, not invention — per directive. Reported. Holding.

---
[GAP-AUDIT FOLLOW-UP — 2026-07-10] Architect-ruled: appended the 2 schema locks + wrote the 2 stubs + noted the B7/B8 co-locks. Then HOLD (frontier complete). (GENERAL DESIGNER)
· (a) G-A LOCKED → GOD-POWERS-CATALOG.md "GAP-AUDIT ADDENDUM": 15 new divine-verb rows w/ alignment_weight +
  cast_vfx (anoint-champion +0.7 / lay-geas ~0 / consecrate +0.5 / desecrate −0.6 / cleanse-haunt +0.5 / lay-to-
  rest +0.6 / raise-dead −0.9 / bless-hunt +0.4 / tame-beast ~0 / loose-beast −0.8 / bless-feast +0.5 / blight-
  feast −0.5 / manifest-sign ~0 / inspire-revelation +0.4 / emergency-run ~0). The TWO HANDS now both in-catalog
  (anoint ↔ curse/raise). Curse-a-colonist reconciled (extend the existing row w/ DF-CURSE's lift-condition, no dup).
· (a) G-B LOCKED → DF-HIST-design.md "GAP-AUDIT ADDENDUM": the full ChronicleEvent kind-list (~35 kinds grouped by
  source — production/faith-omens/4-faces/triad/dead/divine-hand/knowledge/colony-life) so record() emit-points +
  the glyph batch land once. Schema `{kind, actors, site, at_tod, importance, scope, attribution}` unchanged.
· G-C1/G-C2 kept in §2 registry (approved). ADDED to §2: B7 Need-schema co-lock + B8 threat-model co-lock (note
  for when those build — B8 must fit the named-figure triad).
· (b) STUBS (one-paragraph each, NOT full passes — resolve dangling refs, FRONTIER+1): DF-WOUND/DF-MEDICAL-design.md
  (wound = Health-debit + status buff = DF-SYNDROME shape; graceful-degrade until B8) + DF-TAVERN-design.md (the
  shared gather-loop + recreation venue; graceful-degrade until the social layer). Both flagged "write full when
  B8/festival near." Status rows marked ⚠STUB.
· Required-asset fill (§1) → pilot (done prior). GAP-AUDIT COMPLETE. Frontier complete → HOLDING per architect.

---
[CONSOLIDATION — 2026-07-10] THE REQUIRED-ASSET QUEUE established at the TOP of ASSET_REQUESTS.md (pilot's #1 request). (GENERAL DESIGNER)
· ONE designer-owned prioritized pull-list (replaces urgency-guessing + running-wild). Rows = asset · GATE · CONSUMER
  · PRIORITY, pointing to the full brief below. Priority = BUILD PROXIMITY (frontier+1): P0 now/next · P1 near ·
  P2 designed-downstream · P3 far/speculative. Pilot pulls TOP-DOWN.
· Found on the board: the pilot BUILT overnight both the god-hand GOOD/EVIL two-faces (godhand_good/ + godhand_evil/,
  same skeleton, band breakpoints in rig.json) AND the DIVINE VFX presets (divine_vfx_presets.md — smite/blessing/
  heal/conjure/shelter + aura/trail, both tints) + the 5 new definitive anims (select/stroke/slap/gesture/descend →
  rig now 21). → marked ✅ DONE in the queue (both god-hand rows), per architect.
· NET RESULT: the SOLE P0 "required-now" asset is the TIME-CONTROL HUD buttons (⏸/▶/⏩/⏩⏩ + speed indicator) — THE
  next build. P1 = tool-tiers + divine-icons + HUD-icons (control surface). P2 = festival/night + open earlier-batch
  props. P3 = the Tier-3 epic batches (speculative-only). Designer keeps the queue current as the lead. HOLD.

---
[PASS — 2026-07-10] SELECTABLE MINE MODES (`MINE-MODES-design.md`; MINEMODE-0..2) — Ben-routed, FRONTIER+1 (mining is the active build). (GENERAL DESIGNER)
· Player picks the mine's ACCESS STYLE at placement (extend the b-2.1 flat/slope mode-stepper — the selector
  machinery already ships). Two dig patterns over the one `DesignationKind::Mine`:
· LADDER MINE = a compact vertical SHAFT + a ladder built ACCESS-BEFORE-DESCENT (B5.8 access + the collision waiver
  + the anti-stuck egress/loop backstops). Deep, small footprint; carries the access-management.
· STAIRS MINE = the dig STAGGERS into one big descending TERRACED staircase (strip-mine/open-pit) — the STEPS ARE
  THE ACCESS (walk down/up), NO separate ladder, NO egress structure. = **`carve_ramp` (the shared reachability-
  safe volume decomposer) SCALED TO A WHOLE MINE** — a 4th caller (dig-down/build-up/fell-down/stairs-mine),
  build-once. HEADLINE VIRTUE (Ben): access is INHERENT in the dig geometry → **no colonist entombed, BY
  CONSTRUCTION** (carve_ramp's bottom-up adjacent-reachable ordering invariant guarantees a walkable step at every
  depth *during* the dig) → sidesteps the ENTIRE stuck-at-exit / egress-fail-safe / access-before-descent problem
  class. Trade-off: STAIRS = wide + material-heavy but STUCK-PROOF (the safe default, esp. while access-reliability
  is being hardened); LADDER = compact + deep but keeps the access nets.
· Sub-blocks: MINEMODE-0 (the access-mode selector, extend the stepper) / MINEMODE-1 (STAIRS via carve_ramp — the
  stuck-proof mode) / MINEMODE-2 (LADDER — the shipped B5.8 shaft+ladder keyed to the mode). Near-asset-free
  (STAIRS = pure dig geometry; LADDER reuses the mine batch; a mode-toggle icon → UI-2, no new batch — no queue
  change). GUARDS: reuse carve_ramp+stepper+B5.8 (no forks) · deterministic · STAIRS footprint scales w/ depth
  (a tuning trade-off, not a bug). ROUTE: build → builder, QUEUED AFTER the access-reliability bug batch (a
  feature, not a blocker); the stuck-proof STAIRS mode complements that reliability work. Status row added. HOLD.

---
[ROLE SHIFT → HARD-PROBLEM RESEARCH — 2026-07-10] Deliverable #1: the ranked COMPLEXITY/RISK REGISTER (`COMPLEXITY-RISK-REGISTER.md`). (GENERAL DESIGNER, now hard-problem research lead)
· New mode (Ben): research our most complex issues → IMPLEMENTATION-ORIENTED technical specs (algorithms/data-model/
  Veloren seams/edge-cases/staged plan, not vision) → route each to the Build-Reviewer (local_7e72649b) for a
  FEASIBILITY verdict → builder queue. Discipline: research-deep, feasibility-gated, ONE at a time, best-not-most.
· REGISTER (ranked complexity×risk×value + frontier-proximity, grounded in a code survey):
  #1 Colony-scale PATHFINDING (H/H/VH, NEAR) — astar.rs is per-agent A* over a voxel graph that dig/build mutate
     live, no per-tick budget → chokes as pop grows. FEAS-Q: hierarchical/flow-field/caching + budget; reuse vs
     wrap astar. ← RECOMMEND LEAD.
  #2 AUTONOMY drive-arbitration + self-designation G1/G2 (H/H/VH, NEAR) — emergent job-priority w/o stall/thrash.
  #3 rtsim↔loaded LOD persistence/consistency (H/H/H, MID) — promote/demote across SimulationMode w/o dupe/loss.
  #4 DF-FLUID (VH/VH/H-deferred, FAR) — Ben's "try-or-static" GATE → a go/no-go FEASIBILITY SPIKE, not a build spec.
  #5 GOD-POWERS dispatch (M-H/M/H, NEAR) — one cast-pipeline; do the 🔴 rows have an engine seam?
  #6 underground worldgen/camera + #7 agent-culture + #8 tree-felling = DE-RISKED (investigation/design done) →
     implementation not research.
· Survey grounding: astar.rs (generic per-agent A*, no hierarchy/budget); rtsim `SimulationMode::{Loaded,Simulated}`
  (the LOD boundary ships); fluids TYPED in block.rs (0x00-0x0F, LiquidKind) but NO flowing sim.
· RECOMMEND: PATHFINDING leads (near-frontier + a true algorithmic unknown); DF-FLUID spike = a parallel go/no-go
  decision-gate for Ben (not a build blocker). Register → architect for the priority nod (SPEC → reviewer after).
  Awaiting the nod on which leads.

---
[HARD-PROBLEM SPEC #1 — 2026-07-10] COLONY-SCALE PATHFINDING implementation spec (`PATHFINDING-SCALE-SPEC.md`) + the parallel DF-FLUID feasibility decision brief (`DF-FLUID-FEASIBILITY-SPIKE.md`). → Build-Reviewer for feasibility. (GENERAL DESIGNER / hard-problem research)
· PATHFINDING (the deliverable, deep code survey — astar.rs + path.rs read in full): KEY FINDINGS = the A* is
  ALREADY incremental (`Astar::poll` time-slices, returns Pending) + the `Chaser` ALREADY caches a route (re-search
  only on target-move>2/start-move>4/invalidation) + `SharedChaser` exists (unused path-share hook) + `BlockChange`
  (bastion_jobs.rs:1183) is THE dig/build mutation seam + chunks=32². So the low-level search + follow + vertical
  edges are GOOD — KEEP. The scaling gaps are all ABOVE them: (1) no GLOBAL per-tick budget across agents (2) no
  path SHARING (many-to-one recomputed per-agent) (3) repair = lazy FULL re-search on-traverse (dig/build churn)
  (4) flat A* → long-range frontier explosion (75k-iter Longest). ARCH = layered hybrid: L0 KEEP voxel-A*+Chaser ·
  L1 HPA* chunk-portal abstraction (long path = portal search + local refine) · L2 GLOBAL budget+scheduler (reuse
  the incremental poll — CHEAPEST/FIRST) · L3 cache+share + FLOW-FIELDS for hot shared goals (depot/dig-face/muster)
  · L4 incremental REPAIR via BlockChange→dirty-cluster→re-refine-only-the-broken-leg (NOT full re-search — the
  crux Ben flagged; the hierarchy localizes the damage). Staged PATH-0 (budget, buildable NOW, the near-term
  safety) → PATH-1 (HPA*) → PATH-2 (share/flow) → PATH-3 (repair) + a `--pathfinding-scale-scenario` gate (bounded
  per-tick cost, ~linear-in-N, no false-unreachable, deterministic). REUSE-first throughout (astar.rs unchanged;
  extend Chaser; hook BlockChange; reuse SharedChaser + the chunk grid). Recommendation: BUILD PATH-0 now
  (frontier+1 safety), spec-approve PATH-1..3 as the scaling roadmap.
· DF-FLUID SPIKE (parallel decision brief): fluids TYPED (block.rs 0x00-0x0F/LiquidKind) + buoyancy exist, NO flow
  sim. Cost drivers = a fluid-cell STATE grid (sub-block level) + tick update + NET-SYNC (top risk) + save +
  determinism + the active-region BOUNDARY. Approaches: CA = the viable path (low-med, deterministic, DF-feel);
  shallow-water = surface-only; pressure/NS = OVER-BUDGET/base-game-risk = NO-GO. GO/NO-GO line = CA-over-bounded-
  active-region-flagged-with-static-fallback is VIABLE; the DECISION is spend-the-spike-now-or-later (NOT CA-vs-NS,
  it's CA). Recommend a minimal CA SPIKE (one region, one scenario, proving sync/boundary/budget/determinism)
  BEFORE committing consumers — green→commit, red→static (spike-cost only). Not frontier-now.
· Both → Build-Reviewer (local_7e72649b) for feasibility, CC architect. Pipeline: reviewer verdict → builder queue.

---
[COMMON-ISSUES CONTRIBUTION — 2026-07-10] Consulted BASTION_COMMON_ISSUES.md + appended DESIGN/GAMEPLAY-LOOP classes distilled from the ~50-pass corpus. (GENERAL DESIGNER)
· Ran the checklist against the pathfinding spec: it EXEMPLIFIES P1 (global per-tick budget), P2 (incremental
  repair not full recompute), P3 (shared flow-fields for many-to-one), D1 (reuse astar.rs/Chaser/BlockChange),
  D3/frontier+1 (PATH-0 now, HPA* staged). D4/G5 legibility = N/A (infra, no player UI). B8 determinism addressed
  (§9). Spec holds — no gap surfaced.
· APPENDED (genuinely-new, non-dup — reviewer curates): DESIGN D7 schema-lock-before-emitters-harden (ChronicleEvent/
  god-power vocab — distinct from D1 reuse) · D8 asset-request-skipped (mandatory pass step) · D9 unbuilt-dep crash-
  vs-graceful-degrade (the stub pattern) · D10 4X/management-drift (autonomous-not-4X + influence-not-command
  pillar). GAMEPLAY-LOOP G7 write-only-sink/no-read-back (the sharpest G1 variant — the chronicle-before-reputation
  lesson) · G8 keystone-dark (B7 gates ~6 systems' payoff) · G9 unbounded-pressure/soak-law-break · G10 permanent-
  softlock/unrecoverable-loss · G11 earned-vs-granted (the B&W emergence guard). + extended the Reuse/design +
  Gameplay question blocks. This checklist is now a standing pre-flight I consult while spec'ing each hard problem.

---
[FEASIBILITY FOLD — 2026-07-10] Reviewer FR1/FR2 verdicts folded into both specs. PATHFINDING = FEASIBLE-WITH-CHANGES (builder-ready as PATH-0, gated on B7); DF-FLUID = FEASIBLE-as-a-spike. (GENERAL DESIGNER)
· Reviewer re-verified every code claim (all accurate) — I re-confirmed the 3 key refs: `.par_join()` (agent/mod.rs:76),
  `TerrainChanges.modified_blocks` (state.rs:114), `SharedChaser {nodes,goal}` stub (agent.rs:657). Verdict solid.
· FR1 PATHFINDING — 4 changes FOLDED (revision block at top + surgical inline edits): (1) THE BIG ONE — PATH-0's
  scheduler must be DECOUPLED from the parallel agent .par_join() (a shared budget under the parallel join is
  order-dependent → non-deterministic); fix = a dedicated SEQUENTIAL id-ordered scheduler system, poll lifted OUT
  of the agent tick → PATH-0 re-rated small→MEDIUM. (2) SharedChaser = unused stub, NO reuse dividend → PATH-2
  builds sharing from scratch. (3) "extend Chaser" → WRAP gated on comp::Colonist (a Bastion-side portal_route
  component, not a field on the vanilla-shared Chaser). (4) vertical links = mostly INTRA-cluster edges (a ladder
  inside one 32² column), not border portals — model both. BONUS: L4 hook is FREE (read TerrainChanges.
  modified_blocks, the aggregated per-tick set, not raw BlockChange). BONUS: PATH-0 = frontier+1-WITH-B7 (ship w/
  a near-cap scenario or it's speculative). Approved to reach the builder as PATH-0 (Change 1 designed in) when B7
  lands; PATH-1..3 spec-approved as the roadmap. No blockers.
· FR2 DF-FLUID — CA confirmed; changes folded: net-sync = QUANTIZE the float level into discrete block variants →
  rides the existing block-diff sync (no bespoke channel); boundary = FIXED region/Dirichlet in the spike (dynamic
  flood-GROWTH deferred); SPLIT the spike into G1 (sim+determinism+boundary HEADLESS, days) → G2 (sync, only if G1
  green); save = re-settle-on-load cop-out. FEASIBLE as a spike, no blockers; when-to-time-it stays Ben's call.
· Both specs now builder/decision-ready. Reported to architect. Holding for: PATH-0 → builder queue when B7 lands
  (via architect) + the next hard-problem nod (#2 autonomy-arbitration / #3 LOD-consistency).

---
[GENRE FOLD — 2026-07-10] Architect genre research (BASTION_COMMON_ISSUES §E, E1-E7) folded into the two active specs. (GENERAL DESIGNER)
· PATHFINDING — E4 (RimWorld: good-enough paths save CPU, suboptimal = CORRECT): added the INVARIANT to §1
  constraints + PATH-0 + the §12 gate — "never-stuck + bounded per-tick + every-reachable-goal-reached +
  deterministic," explicitly NOT shortest-path. Reframed as a LICENSE (HPA*/flow-field/budget-stagger are all
  acceptable-by-design, not defects). Don't spec/gate optimality.
· DF-FLUID — E5 (From Dust: fluid IS a GOD-VERB, not terrain flavor): reframed the spike as sim + GOD-POWER
  together (channel-water-to-field / divert-lava / flood-attackers / drain-shaft) → raises the ambition's value +
  the spike must prove the god can MANIPULATE the CA field. E6 (ONI/DF both CA; the real fork): split CA into
  WITHOUT-pressure (falls/spreads, cheaper, downhill verbs) vs WITH-pressure (equalizes/climbs, costlier, needed
  only if a verb requires water to CLIMB — moat/aquifer/sealed-fill). Made it the spike's GO/NO-GO AXIS tied to
  the god-verb scope; G1 headless gate tests BOTH + reports the cost delta. Updated §4 table + a genre-fold block.
· STANDING (for the queue specs): E1 prevent-death-spirals = the autonomy-arbitration success criterion · E2
  apophenia/emergent-story (freedom in HOW + legible traits, don't over-script) · E3 indirect-control (god
  influences never commands — hard guardrail on every god-verb) · E7 UI-legibility genre-CRITICAL. Will carry
  these into hard-problem #2 (autonomy-arbitration) + god-powers + agent-culture specs. Reference titles per
  problem (DF/RimWorld/ONI/Timberborn/Songs-of-Syx + Populous/B&W/From-Dust/Reus/WorldBox). Genre study now feeds
  every spec. Reported. Carrying on with pathfinding (spec complete + reviewer-approved; awaiting B7 → builder).

---
[HARD-PROBLEM SPEC #2 — 2026-07-10] AUTONOMY drive-arbitration + self-designation (`AUTONOMY-ARBITRATION-SPEC.md`; AUTON-0..3) — the architect's nod. → Build-Reviewer for feasibility. (GENERAL DESIGNER)
· SURVEY (grounded): two decision layers today — rtsim `Action` brain (npc_ai/mod.rs, behaviour-tree combinators =
  EXECUTION) + the bastion_jobs board (claim by priority/skill/distance, ARBITRATION_INTERVAL=15) where
  `comp::bastion::ActiveJob` UNCONDITIONALLY suppresses the brain. GAPS: G2 no self-designation (jobs only from
  painted Region designations → un-designated colony idles) · no unified arbitration (a job overrides everything →
  a starving colonist won't stop to eat — the death-spiral trap) · no anti-thrash · claim-local not emergent-global.
· ARCH = two halves: (5A) a per-colonist UTILITY ARBITER above the reused brain+board — drives {Survive/Flee/Work/
  SelfNeed/Idle} → urgency (Survive non-linear/spikes) → trait-modulate (E2, B-AG3 facets) → policy-thumb (D10,
  DF-POLICY) → pick max → execute via the rtsim Action brain; COMMITMENT at ARBITRATION_INTERVAL + higher-tier
  preemption (anti-thrash E1); ActiveJob repurposed from override → the Work drive's handle. (5B) colony SELF-
  DESIGNATION generators (state→jobs: mine/build/haul/hygiene/expand/defense, rate-limited/dedup/DF-POLICY-gated)
  feeding the board — closes G2; emergent priority falls out (not hard-coded).
· E1 (death-spiral prevention — THE success criterion): responsive non-linear Survive curve (preempts Work before
  collapse) + self-correcting labour feedback (scarcity raises the food-generator's job-urgency → labour auto-
  shifts → recovery); player PLANS not FIREFIGHTS. E2 apophenia: trait-modulated urgencies → legible individuality
  + UI-4 shows last_scores (motive). D10: zero-policy = healthy colony; god thumbs the weights, never commands.
· Staged AUTON-0 (arbiter skeleton, Work/Idle/Flee — buildable NOW) → AUTON-1 (self-designation generators, closes
  G2 — the immediate autonomy win) → AUTON-2 (need-drive plug-in @B7 + the death-spiral gate — E1) → AUTON-3
  (trait-modulation E2 + policy-thumb D10 + tuning). Gate = `--autonomy-soak-scenario` (zero-input N-days: no
  stall/thrash · seeded-shortage auto-recovers · tended-colony-survives · deterministic · zero-policy-healthy).
· Ran the COMMON-ISSUES checklist against it: addresses D1/D3/D10/G1/G4/G7/G8/G9/G10 + E1/E2/E3 by construction;
  flagged P1 (does the arbiter need its own per-tick budget?) as an open Q for the reviewer. Reuse-first (brain +
  board + claim scorer + interval + facets + policy all reused). → Build-Reviewer (local_7e72649b) + CC architect.

---
[FEASIBILITY FOLD #2 — 2026-07-10] Reviewer FR3 (autonomy-arbitration) FEASIBLE-WITH-CHANGES → all folded. AUTON-0/1 builder-ready; AUTON-2 @B7. (GENERAL DESIGNER)
· GOOD NEWS confirmed: my mid-job-clear risk is a NON-issue — npc_ai has zero knowledge of ActiveJob (the
  "suppression" = bastion_jobs overwriting rtsim_controller.activity); the claim sweep (:1494-1500) self-heals an
  abandoned claim ≤15 ticks (sweep-enforced invariant); the 3-step clear template exists (:1408-1413).
· 3 CHANGES FOLDED (revision block + surgical edits): A (the REAL work — a D2 two-authorities risk: the arbiter
  must be the SOLE writer of rtsim_controller.activity for colonists + GATE the whole bastion_jobs Sys [7 activity
  write sites] on arbiter.current==Work, else bastion_jobs re-overwrites + fights Survive/Flee — folded into §7 +
  AUTON-0 as the real integration work). B (price reuse PER-DRIVE / D11: reuse goto/travel primitives, but Survive/
  eat LOGIC is net-new; Flee reuses vanilla flee — folded §7). C (E1 gate depth / new class G12: a DEEP colony-wide
  shortage → everyone spikes → nobody farms → recovery starves; trait-stagger[E2] gives a RECOVERABLE BAND; the
  gate must assert recovery-in-band + degrade-past + a shortage deep enough to STRESS the stagger — folded §8/
  AUTON-2/§14).
· (d) house the arbiter in the SEQUENTIAL bastion .join() system (NOT par_join — dodges the pathfinding-Change-1
  determinism issue); no separate budget. (e) commitment = 2 cadences: per-15 for same-tier anti-thrash, PER-TICK
  for Survive/Flee preemption (a starvation spike can't wait 0.5s). Both folded §5A/§7.
· Verdict: AUTON-0/1 approved to BUILD with A+B; AUTON-2 @B7 with C. No blockers. BOTH hard-problem specs (#1
  pathfinding, #2 autonomy) now feasibility-approved + folded + builder-ready (pathfinding@B7, autonomy AUTON-0/1
  now). Reported to architect. Holding for #3 (LOD-consistency) nod or build-routing.

---
[HARD-PROBLEM SPEC #3 — 2026-07-10] rtsim↔LOADED LOD PERSISTENCE/consistency (`LOD-PERSISTENCE-SPEC.md`; LOD-0..3) — the architect's nod; the 3rd "at scale" leg (move·decide·PERSIST). → Build-Reviewer. (GENERAL DESIGNER)
· (FR3 autonomy changes were ALREADY folded last turn — the architect's fold-request crossed my completion; noted.)
· SURVEY (grounded, the boundary is a STRENGTH/reuse dividend): SimulationMode{Loaded,Simulated} ships; `Npc` is
  the serde-persistent home w/ `npc.bastion_colonist` (persistent, mirrored to comp::Colonist on promote); promote
  = SpawnEntityData::from_entity_info + mirror (tick.rs:712); loaded-sync loop exists (tick.rs:696); demote hook
  exists (mod.rs:350). BUT THE GAPS: (1) the sync writes ONLY `npc.wpos=pos.0` (:731) + promote inserts
  Needs::default() (:719, not persisted) → mutated Bastion state (needs/XP/inventory) is LOST on unload/save
  [latent-until-B7, structural]. (2) NO abstract resolve — a Simulated colonist runs the vanilla rtsim brain, not
  the Bastion job-loop → the colony's economy FREEZES unwatched. (3) dupe-window at a non-atomic transition. (4)
  no reconciliation.
· ARCH: 5A authority model (Npc.bastion_colonist = authoritative home, loaded comps = mirror, one-writer-per-field
  D2). 5B SAVE-BACK protocol (LOD-0 loss guard: extend the sync to persist FULL BastionColonist + promote-from-
  persisted-not-default; conservative no-item-dupe). 5C ABSTRACT-RESOLVE colony tick (LOD-2 progression guard: a
  coarse rtsim-tier equilibrium tick = needs-decay + aggregate production/consumption = THE #2 AUTONOMY DRIVES
  COARSENED; bounded/tend-to-equilibrium; E1 no-spiral-unwatched; E7 chronicle away-summary). 5D atomic
  transitions + mode-gating (dupe guard). 5E deterministic conservative reconciliation (desync guard; Npc = source
  of truth on re-promote).
· THE 3 DEATHS explicitly guarded (dupe=mode-gate+atomic · loss=complete-save-back · desync=conservative-
  deterministic-resolve+single-source-of-truth). Staged LOD-0 (save-back, WITH B7) → LOD-1 (atomic/dupe) → LOD-2
  (abstract resolve, AFTER #2 autonomy) → LOD-3 (reconciliation + partial-load + the --lod-soak-scenario gate:
  load→mutate→unload→re-promote conserves all state, unwatched colony progresses+no-spiral, straddling-boundary
  consistent). Reuse-first (extends the shipped sync/hooks/save_thread; LOD-2 composes #2). Ran common-issues
  checklist (D1/D2/D3/D7/D9 + G4/G8/G9/G10 + E1/E7 + P2/B8 addressed). → reviewer + CC architect. All 3 near/mid
  register problems now spec'd.

---
[HARD-PROBLEM SPEC #5 — 2026-07-10] GOD-POWERS DISPATCH (`GOD-POWERS-DISPATCH-SPEC.md`; POWER-0..3) — the FINAL near-frontier spec (the god-hand showpiece's mechanics). → Build-Reviewer. Then HOLD THE LANE. (GENERAL DESIGNER)
· SURVEY: the overseer `bastion_tools.god_mode` toggle ships but FAVOR is NOT built (session/mod.rs:1972 "when the
  colony + favor exist"); `Outcome{Lightning,Explosion,reagent}` = the cast-VFX seam (Smite/Meteor already
  IMPLEMENTED = 🟢); the catalog's 🔴 rows each NAME a specific designed engine (Hazard-Events/DF-FLUID/rtsim-DP/
  flora-growth). So the 🔴 question resolves: GATED not cut — the pipeline accepts a power when its engine lands;
  a few far-DP get colony-slice-or-defer (the DF-TRADE pattern).
· ARCH = the single CAST PIPELINE: invoke(god-bar/hand) → target(unproject_to_world_plane) → ★FAVOR GATE(D5, net-
  new resource; no free cast) → dispatch via a per-power `PowerEffect` enum → emit Outcome(cast_vfx, alignment-
  tinted, reuse UI-5 presets) → record DF-HIST DivineAct + apply alignment_weight drift → debit favor. ONE pipeline,
  N powers plug in via PowerEffect (match, not N bespoke paths). ★ THE PILLAR ENFORCED BY THE TYPE SYSTEM:
  PowerEffect = {TerrainEdit/Buff/Weather/Hazard/MindWrite/Spawn/Provide/RtsimAct/HandPhysical} — NO Command
  variant → a power CANNOT issue a unit-order (indirect-control E3/D10 guaranteed, not just intended); the one
  physical reach (HandPhysical grab/throw) is a costed felt B-AG3-reacted act (HAND-CURSOR), not a standing order.
· FAVOR (D5): net-new resource, regen from DF-RELIGION devotion, cost-tiered (forcing-a-will most), insufficient
  rejects. Separate from alignment_weight (the moral drift). Staged POWER-0 (pipeline + Favor + Smite end-to-end —
  the showpiece's first god-verb) → POWER-1 (the 🟢 kit: raise-land/ore/spawn/meteor/curse/drought via PowerEffect
  variants) → POWER-2 (hand-physical verbs + 🟡 polish + the alignment/epithet drift) → POWER-3 (🔴 gating as
  engines land). Gate = --god-power-scenario (favor-gated · effect lands · Outcome+chronicle+drift · indirect-
  control asserted [no command variant; zero-favor colony autonomous] · deterministic · 🔴-no-engine greyed-not-
  crashed). Reuse-first (invoke/picker/🟢-effects/VFX-bus/attribution ship; net-new = Favor + pipeline + dispatch).
· Ran common-issues (D1/D3/D5/D9/D10 + E3/E5/E7 + G1 + B8). → reviewer + CC architect.
· ⇒ THE NEAR-FRONTIER REGISTER IS NOW EXHAUSTED (4 specs: pathfinding/autonomy/LOD/god-powers). FRONTIER+1 STOP —
  4 unbuilt vs 0 built. HOLD THE LANE (per architect). DF-FLUID stays Ben-timing. No speculative/far specs.

---
[FEASIBILITY FOLD #3 — 2026-07-10] Reviewer FR4 (LOD-persistence) FEASIBLE-WITH-CHANGES → all folded. All 3 near/mid specs now feasibility-gated + folded. (GENERAL DESIGNER)
· ★ SHARPENING: the LOSS gap is PARTLY LIVE TODAY (not fully latent) — skills SHIP + mutate via grant_xp, and the
  wpos-only save-back means loaded skill-XP is never written → a leveled colonist comes back DE-LEVELED after
  unload/save. Precision: skills/XP are already IN BastionColonist (restore fine on promote); net-new persistence
  = Needs/Mood/inventory. ⇒ LOD-0's save-back has CURRENT value pre-B7 (registry class B11 added). Folded §3/§6/
  LOD-0.
· ★ (b) SAVE-BACK METHOD (supersedes my per-tick-vs-on-demote Q): comp::Colonist is DerefFlaggedStorage
  (bastion.rs:25) → CHANGE-TRACKED incremental save-back (sync only modified colonists — cheap + always-current,
  dodges the save_thread off-tick race). Folded §5B/§7.
· ★ (f) CORRECTION — I over-claimed #2≡#3. LOD-2's aggregate is NOT #2's per-agent arbiter reused (that'd be D11)
  — it's a SEPARATE coarse APPROXIMATION kept aligned with #2's equilibrium ("consistent-approximation," budget
  the alignment; gate re-promote-lands-where-loaded). Folded §5C/§7/§14.
· (c) desync = a CONSERVATION violation (sum-in==sum-out ± logged deltas); gate asserts conservation not replay.
  (d) partial-load clean IFF one-tier-per-colonist + rates-from-simulated-headcount + a SINGLE stock authority
  (the real D2 hazard). (e) skills/inventory save-back PRE-B7, Needs-restore @B7. All folded (§8/§11/§13/§14).
· ⇒ ALL 3 NEAR/MID SPECS (pathfinding/autonomy/LOD) = FEASIBLE-WITH-CHANGES + folded. God-powers (#5) verdict
  pending. Full feasibility-gated build roadmap. HOLDING THE LANE (frontier+1 stop). Reported.

---
[GENRE FOLD + NEW PULL — 2026-07-10] (1) B&W lessons folded into god-powers; (2) TIMESCALE-DESIGN (Ben pull, foundational). (GENERAL DESIGNER)
· GOD-POWERS B&W fold (Q13): GESTURE-CAST as a 2nd INVOKE path (draw-a-shape on the god-hand → power, the expert/
  god-feel path alongside the discoverable god-bar panel; reuse the hand cast/gesture/paint anims + unproject; same
  favor-gate+dispatch downstream) + ENVIRONMENTAL ALIGNMENT FEEDBACK (the alignment expresses at WORLD scale —
  colony palette/light/music/mood tilts with the drift, not just the hand; spec the HOOK now [alignment→an
  environmental-tint param], build the tint later; the legible earned soul E2+G11 at world scale). Guards preserved
  (D5/E3/D10).
· TIMESCALE-DESIGN.md (Ben pull — "critical later"; the constant everything keys off; the design half of the
  timescale bug). SURVEY: TimeOfDay(f64)=86400 game-sec/day (time.rs rem_euclid); day_cycle_coefficient = the
  game↔real ratio; TimeScale(f64)=the speed multiplier; the trap = real-time rates (WORK_DURATION_BASE=3.0 REAL-sec)
  must migrate to game-time. GENRE: ONI 10min/cycle + RimWorld ~17min/day = the watch-a-colony sweet spot; DF's fast
  day = the anti-pattern for observe-live. RECO: base = a game-day ≈ 10 real-min at 1x (tunable 8-15); season = 12
  game-days, year = 48 days (= SEASON-CLOCK year_length); speed ladder = pause/1x/2x/4x/8x (add 8x for downtime;
  pins the time-control HUD "faster") + optional 0.5x savor; ALL DERIVED RATES per-game-time (hunger −100%/12
  game-hrs, a job = game-min, travel game-relative, aging +1yr/48days). ★ THE MODEL TO PIN (the deliverable):
  (1) game-time = master clock, base ratio = day_cycle_coefficient; (2) TimeScale scales it → everything derived
  scales automatically; (3) ALL rates per-game-time NEVER per-real-second (the load-bearing rule — the bug fix +
  future-proofing); (4) day/year = RON constants. Numbers are Ben-tuning; rules 1-3 are the structure. → reviewer
  for feasibility (does the tick model support per-game-time rates; grep for DeltaTime-hardcoded landmines). Then
  back to HOLD.

---
[FEASIBILITY FOLD #4 + REGISTER COMPLETE — 2026-07-10] Reviewer FR5 (god-powers) FEASIBLE-WITH-CHANGES → folded. ALL 4 near-frontier specs now feasibility-gated. (GENERAL DESIGNER)
· FR5 folded (3 precisions): (f) DISPATCH must apply the REAL effect (Smite = lethal damage via health system,
  reuse) AND emit the cosmetic Outcome — a flash-only Smite has no kill; + the cast is SERVER-AUTHORITATIVE (client
  invoke/target → server favor-gate/dispatch/effect; a client can't self-grant/bypass cost; reuse the designation
  client→server msg). Folded §4 pipeline (now client/server-split) + POWER-0. (b) the no-Command-variant is
  NECESSARY-NOT-SUFFICIENT (class D14): MindWrite/RtsimAct can SMUGGLE a command → constrain INSIDE — MindWrite
  writes only MOOD/THOUGHT/URGENCY the #2 arbiter WEIGHS (never activity/ActiveJob/destination); RtsimAct tilts
  sentiment/faith, never orders. Folded §4-PowerEffect/§8. (c) hand-physical guard: ONE-SHOT (re-arbitrate on
  landing) + COSTED-per-act + REACTED-TO → no persistent teleport-to-job. Folded §8. (d)(e) affirmed.
· ⇒ NEAR-FRONTIER REGISTER COMPLETE: pathfinding/autonomy/LOD/god-powers ALL FEASIBLE-WITH-CHANGES + folded; fluid
  = timed spike. A full feasibility-gated build roadmap. Every reviewer verdict caught something real (par_join
  determinism, ActiveJob authority, deep-shortage G12, skill-XP live-loss B11, #2 over-claim, flash-Smite + D14
  leaky-variants) and every one folded.
· PRIORITY QUEUE (Ben, 4 pulls): (1) god-powers verdict fold ✅DONE · (2) TIMESCALE ✅sent-awaiting-verdict ·
  (3) LORE BIBLE — STARTING NOW · (4) DF-data-model-ref fold (when the research agent delivers). Then hold.

---
[LORE BIBLE — 2026-07-10] `BASTION-LORE-BIBLE.md` delivered (Ben pull #3 — "develop the lore, the game's soul"). CONSOLIDATION not re-invention. (GENERAL DESIGNER)
· REUSE-FIRST: built ON Veloren (6 races Human/Dwarf/Elf/Orc/Danari/Draugr; site-cultures Gnarling/Sahagin/Myrmidon/
  Cultist/Adlet/Terracotta/Haniwa — verified in comp/body/humanoid + world/site) + CONSOLIDATED the scattered
  lore-adjacent corpus (GOD-HAND/UI-5/GOD-POWERS/GOD-EPITHET/OMEN, FOUNDING-EMBARK, the remembering-world quartet
  [REPUTATION/EPITHET/SACRED-SITES/RENOWN]+ANCESTORS+HIST+ART, VILLAIN/CHAMPION/BEAST/KNOWLEDGE, AGENT-CULTURE) into
  ONE bible + a source-doc map (§0) so it's gather-not-fork.
· THE DELIBERATE CHOICES MADE (the bible's real job): (1) TONE = GRIM-TENDER · FOLK-MYTHIC (DF-honest about death/
  losing-is-fun, carried in the warm folk-craft voice already established across the ~50 design-doc closings + the
  asset lore-seeds — that voice IS the tone, named + pinned). (2) THE GOD'S NATURE = earned-not-chosen, the central
  question the colony answers via the epithet; a hand, no fixed alignment, influence-not-command, a world-scale
  pantheon of rival gods (divine-politics). (3) THE STAKES = ENDURANCE on 4 fronts (the dark/wild · the other ·
  itself · the god — who do YOU become); losing-is-fun (a fall → a ruin/legend the world remembers; the god
  outlives its colonies).
· THE COSMOLOGY = THE REMEMBERING WORLD (our signature): the world REMEMBERS at every scale (the 4 faces + the dead
  + art + legends + omens + the Chronicle) — that's what makes it persistent not a diorama. Peoples: Tier-1 Human
  (everyman) + Dwarven (the deep-colony core, the vertical world) → T2 Elf/Orc/Gnarling → T3 rest; 5-axis culture-
  keying. Naming = per-culture pools+dialect (AGENT-CULTURE). Factions = the site-cultures + the inter-culture
  matrix + named rivals + divine politics. §9 = the standing GAPS-to-author (founding-myths, folk-theologies,
  dialect/epithet pools) — a standing creative lane, one Tier at a time, in-tone + Veloren-canon.
· It FEEDS agent-culture/chronicle/naming/dialogue/UI-flavor/the mandatory asset lore-seeds. Priority queue: (1)✅
  god-powers fold (register COMPLETE) · (2) TIMESCALE ✅sent · (3)✅ LORE BIBLE · (4) DF-data-model-ref fold (pending
  delivery). Reported. Then hold.

---
[FR6 TIMESCALE FOLD + DF-REF FOLD — 2026-07-10] The timescale verdict + the DF data-model reference folded. The priority queue (4 Ben pulls) is fully consumed. (GENERAL DESIGNER)
· FR6 TIMESCALE (FEASIBLE) folded → TIMESCALE-DESIGN.md: ★ the TimeScale plumbing is ALREADY COMPLETE (state.rs:884-892
  TimeOfDay/Time/DeltaTime all ×time_scale, client mirrors) → the "day-honors-TimeScale bug" may be a no-op (confirm
  repro; the work = set day_length + the rate migration). RULE 3 REFINED: "all SIM-PACING rates game-time-anchored,
  DERIVED into dt-constants at load (× day_cycle_coefficient), mechanism stays dt/physics-lockstep — do NOT rekey to
  TimeOfDay-delta (clamp-desync)"; migrate WORK_DURATION_BASE (the one live landmine) + future needs/production/aging;
  LEAVE dt-based the engine timers (STUCK_TIMEOUT etc.). 8× = safe clamp-desync (the clamp is the tunneling guard).
  day_cycle_coefficient=1440/day_length = ONE config value (10min→144×, checks out). ARBITRATION_INTERVAL tick-based
  clean. Class B12. Numbers = Ben-tuning.
· DF-REF folded into the 3 hard specs (clean-room; DF=reference, our code GPL-3.0):
  #1 FLUID → the E6 pressure fork RESOLVED cheaper: DF stores NO float pressure — per-tile depth(0-7)+liquid_type +
     a directional drain gradient (perm_flow_dir + sink_dist); "water climbs" is EMERGENT from the sink-distance
     gradient + source flags; pressurizes = a per-liquid bool RULE, not a data diff. ⇒ model depth+gradient+
     pressurizes:bool, NO float field; dirty-tile updates; gases = separate particles out of the CA. De-risks the
     with-pressure tier; G1 go/no-go refined around the depth+drain-gradient settle.
  #2 PATHFINDING → a free O(1) reachability PRE-GATE: a persistent per-tile CONNECTIVITY-GROUP ID (same nonzero id ⟹
     reachable), recomputed only on connectivity dig/build events → gates A* before it runs (kills false-unreachable
     75k-iter searches for free). Pairs with L4 modified_blocks + validates L1 HPA*. Integer costs 256/362 +
     generation-stamp. Folded into PATH-0/1.
  #3 AUTONOMY → adopt the needs schema {focus_level (satisfaction, decays 0→400), need_level (craving weight)} +
     personality=static priors(=E2 trait-modulation) + jobs-first-class-on-a-board decoupled-from-workers(=bastion_
     jobs); the want-vs-can-do split VALIDATES the arbiter/board separation. Folded into §6/AUTON-0.
· ⇒ ALL 4 Ben pulls DONE (god-powers fold · timescale · lore bible · DF-ref fold). Every hard spec is now
  feasibility-gated + genre-sharpened + DF-refined. Full build roadmap; lore bible = the standing creative lane.
  Reported. Holding.

---
[DF→BASTION TRANSLATION LANE — 2026-07-10] Ben directive: direct DF-derived asset creation + build the translation layer. Both deliverables + the tool-tier fix. (GENERAL DESIGNER)
· TOOL-TIER FIX (architect flag — confirmed): the pilot already corrected in PRACTICE (fulfillment note: "same
  native silhouette across all four, only the TIER changes"); my BRIEF TEXT was ambiguous ("lumpy/lashed→clean/
  forged" implied silhouette-variation) → FIXED: "TIER = MATERIAL + COLOUR, NOT SILHOUETTE — same recognizable
  tool shape every tier, recolour by tier, at most a subtle finish cue." So the shovel/axe tiers (TOOL-1/2) won't
  repeat it.
· DELIVERABLE 1 — DF-BASTION-TRANSLATION.md: the translation RULE (keep PURPOSE, change NAME+LORE, grim-tender
  tone, Veloren-canon-safe, reuse-first) + 9 mapping-category tables (quality-tiers / workshops / materials-ores /
  mechanisms / creatures / needs / jobs-labors / mood-mind-affliction-governance / fluid-gas) — DF concept →
  Bastion name → lore-seed → unchanged purpose → corpus status. HONEST FINDING: MOST are ALREADY ✅ (my ~35 design
  docs ARE DF re-skins) — this CONSOLIDATES + coins the few gaps (✦: brewhouse, maker's-bench, Bloodstone, the
  Learn need). Synthesized from DF-DATA-MODEL-REFERENCE + LORE-BIBLE + the corpus. COORDINATION: flagged the
  "Agent translator" session (local_8bc474bc) may own the raw df-structures vocabulary extraction → this doc is the
  DESIGN-CONSOLIDATION + ASSET-FACING half, built to CONSUME a raw extraction; messaged it to reconcile scope.
· DELIVERABLE 2 — DF-DERIVED ASSET DIRECTION (→ ASSET_REQUESTS.md): honest — most DF-derived objects ALREADY
  filed, so NOT a flood on the pilot's 520-redo backlog; filed the 2 genuine gaps (brewhouse/still = Drink venue,
  maker's-bench = general crafts) w/ briefs+lore; PRIORITY reconciliation (redo + P0/P1 first, these = P2, Tier-3
  DF objects = P3-speculative; DON'T interrupt the redo). Next wave awaits the DF-taxonomy extraction (translator/
  oracle) → re-skin as it lands. STANDING LANE. Reported. Continuing to direct as the oracle feeds vocabulary.

---
[TIME-OF-DAY INDICATOR + OWNERSHIP — 2026-07-10] Ben pull (WC3-style sun/moon HUD) + DF-translation ownership confirmed FULLY mine. (GENERAL DESIGNER)
· TIME-OF-DAY INDICATOR (`TIME-OF-DAY-INDICATOR-design.md`): a WC3-style sun/moon arc HUD — sun rises/arcs/sets +
  moon for night, mapping TimeOfDay's day-cycle fraction (rem_euclid(86400)/86400) to the sun/moon arc position;
  reads "midday/dusk/night" at a glance. PAIRS with the speed cluster (⏸/1×/2×/4×/8×) = the time HUD (speed=how
  fast, sun/moon=where in the day). REUSE: TimeOfDay + DayPeriod + the conrod overseer HUD + UI-2 icon language —
  a PURE READOUT (no sim change). BUILD NOTE: a conrod widget reading TimeOfDay, sun/moon by DayPeriod, dawn/dusk
  tints, sits by the speed cluster, ships with the time-control HUD. ASSET BRIEF filed (sun disc gold / moon pale /
  arc-frame / dawn-dusk tints + optional blood-moon-red for DF-OMEN; lore-bible tone + seed; P1, pairs w/ time-
  controls). Its value DEPENDS on TIMESCALE's short day (a 10-min day makes the arc visibly move). Small/concrete/
  player-visible.
· OWNERSHIP: the "Agent translator" session confirmed it owns NOTHING in DF-translation (it's Ben's fleet NARRATOR,
  not a content translator) → BOTH halves of DF-BASTION-TRANSLATION (the concept mapping AND the raw df-structures
  extraction) are MINE; updated the doc's ownership note. I'll grow the translation + generate asset waves from the
  live oracle once Ben embarks a fort. Reported. Continuing the DF-asset + translation standing lane.

---
[TRANSLATION ENRICHMENT — 2026-07-10] Ownership fully confirmed mine (extraction + re-skin; translator owns nothing). Enriched the translation from the DELIVERED reference while the live oracle spins up. (GENERAL DESIGNER)
· DF-BASTION-TRANSLATION §6 NEEDS expanded to the FULL ~30 DF need_types → the LOCKED Bastion `Need` vocabulary
  (grouped/folded): Socialize/Family(Bond✦)/Drink/Pray/Craft/Purpose✦/Learn✦/AdmireArt/SeeAnimals/Roam✦ + hunger/
  rest folds. Coined ✦ (confirm in-tone): Bond/Purpose/Learn/Roam. Tier-1 B7 core = hunger·rest·Drink·Socialize·
  Pray·Purpose (the daily loop). Feeds DF-FOCUS + venue asset-seeds + dialogue. A bounded high-value consolidation
  of the delivered reference (NOT speculative).
· Held the line on the rest: the remaining DF breadth (runtime creatures/items/plants/buildings — §10) genuinely
  awaits the LIVE DF ORACLE (an agent is auto-embarking; query oracle.py units/jobs/fluids once up) — re-skin as it
  lands, don't pre-invent. Priority discipline intact (DF-derived assets stay P2/P3, don't interrupt the 520-redo +
  P0/P1). Tool-tier fix (TIER=material+colour) to carry into shovel/axe. Standing lane; holding for the oracle.

---
[ORACLE MILESTONE 1 — 2026-07-10] The live DF oracle is UP; queried it + re-skinned FAUNA+FLORA. (GENERAL DESIGNER)
· QUERIED: units (7 dwarves — real needs data, VALIDATES the {need,level,focus} schema on live units) · jobs (real
  job_types) · creatures (1312 total) · plants (225) · the PET flag (the domestic/livestock subset: cow/sheep/pig/
  goat/fowl[chicken/duck/goose/turkey/guineafowl]/draft[horse/donkey/mule/yak/llama] + exotics).
· RE-SKINNED (DF-BASTION-TRANSLATION §5 fauna + new §3b flora): DISCIPLINE = the colony-relevant CATEGORIES +
  curated subset, NOT the 1312/225 tables. §5 = 6 creature categories (domestic-herd / vermin / game-wildlife /
  megabeast-titan / night-creature / exotic-tameable — Veloren fauna reused). §3b = 5 crop categories (grain /
  fibre / vegetable / pulse✦ / tree). Most maps to EXISTING batches (herd, crop) — confirmed coverage + coined
  the gaps.
· ASSET WAVE (P2, variation-of, do-not-interrupt-the-redo): filed the genuine gaps — herd breadth (goat + poultry
  duck/goose/turkey + draft mule/donkey/yak — ties DF-TRADE pack-animal/B6-haul) + crop breadth (a wheat/root/
  pulse beyond barley/carrot/flax). Briefs+lore in the tone.
· §10 = the oracle-enrichment LOG: M1 done; NEXT M2 items/furniture (mostly-confirmation, my décor/goods batches
  cover it — quick pass, file only gaps) · M3 stone/soil/metal → DF-GEOLOGY · M4 the FLUID empirical target
  (live water depth/flow = the CA reference — WHENEVER Ben times the spike, not before). Best-not-most held.
  Reported per milestone.

---
[ORACLE MILESTONE 2 + LANE HELD — 2026-07-10] M2 items/furniture CONFIRMATION done → item-coverage CLOSED. Oracle-enrichment lane HELD per architect (frontier discipline). (GENERAL DESIGNER)
· M2: queried the full item_type enum (~95 kinds) + the fort's live inventory. VERDICT: the corpus batches COVER
  the breadth (furniture=DF-ROOMS · openings=BUILD/MECH · weapons/armor/clothing=Veloren-shipped · tools/mill=
  TOOLS/POWER · food=PRODUCTION/LIVESTOCK · book=KNOWLEDGE · coffin=ROT · crafts=ROOMS/maker's-bench). Item-
  coverage question CLOSED (translation §9b). 2 GENUINE GAPS filed (P2): a storage-prop set (barrel/bin/bucket/
  crate/cage — B6-haul) + an engraved memorial SLAB (ties DF-ART depiction + SACRED-SITES/ANCESTORS — the
  remembering-world made physical). Briefs+lore in tone.
· LANE HELD (architect steer, frontier+1): the design lane is well AHEAD of the build frontier (5 hard specs +
  timescale + lore + translation + fauna/flora/item waves, ~none built). So DEFER M3 (stone/metal→geology; not
  near-frontier) + M4 (live-water CA reference; wait for Ben to time the fluid spike — the oracle's highest-unique
  query). Re-open on frontier arrival / Ben pull.
· STANDING creative lane now = LORE BIBLE authoring (founding-myths/folk-theologies/epithet pools, bible §9 — genuinely
  open + always useful). Redo-priority guard intact (DF-derived P2/P3, never interrupt the 520-redo + P0/P1).
  Available for the next Ben pull or the builder consuming the backlog. Reported. Holding.

---
[DATA-MODEL AUDIT — 2026-07-10] Ben pull: the canonical Bastion data-model reconciliation → `BASTION-DATA-MODEL-AUDIT.md`. Audit-not-invent. → reviewer to verify the code column. (GENERAL DESIGNER)
· For every system: DATA NEEDED · WHAT EXISTS (code-cited) · GAP, prioritized by build-frontier (Tier-1 imminent /
  Tier-2 near / Tier-3 defer-stub). Surveyed the actual code + the corpus + the DF benchmark + the oracle.
· TIER-1 (imminent, detailed): COLONIST (Colonist/BastionColonist ✅, Mood=bare-f32 GAP, no arbiter/labor-mask GAP)
  · NEEDS (★ Needs{hunger,rest,recreation}=3-field-no-decay → REPLACE w/ the locked Need enum + {focus,need_level}
  schema — oracle-validated; D7 lock) · JOBS (board+Job struct ✅ strong; self-designation GAP) · SKILLS
  (ColonistSkills 7-skill ✅; ★ XP SAVE-BACK is a LIVE BUG B11 — sync writes only wpos → de-level on unload) ·
  TIMESCALE (plumbing ✅ COMPLETE per FR6; migrate WORK_DURATION_BASE real→game-time) · CHRONICLE (★ ChronicleEvent
  NOT in code, design-only → net-new + D7 LOCK the ~35-kind enum before emitters).
· TIER-2 (near): SENTIMENT (rtsim Sentiments ✅; REPUTATION net-new) · GOD-POWERS (Favor NOT built, alignment NOT
  built, Outcome VFX ✅) · PATHFINDING (astar/Chaser ✅; connectivity-group net-new; modified_blocks hook ✅) · LOD
  (boundary ✅ built; the save-back B11 gap) · ITEMS (item::Quality ✅ LOCKED exists; verify craft_quality field).
· TIER-3 (defer-stub): world/geology · culture/language/epithet · fluid.
· CROSS-CUTTING TOP LOCKS (ordered): (1) ChronicleEvent enum+record() [D7, before emitters] (2) the Need enum +
  {focus,need_level} [DF-FOCUS interface] (3) the LOD save-back [B11 live bug] (4) the arbiter+labor-mask+
  generators (5) WORK_DURATION_BASE→game-time + day_length (6) Favor+alignment+connectivity-group net-new (7)
  Mood→emotion/stress. THROUGH-LINE: the hard substrate is mostly BUILT — gaps = lock 3 schemas + migrate 1
  constant + a few net-new resources, not new engines. → reviewer verifies the WHAT-EXISTS column. Reported.

## 2026-07-10 — DF GAP ANALYSIS (readme/DF-GAP-ANALYSIS.md) [Ben-pull; strategic companion to the data audit]
- **What:** what Bastion is MISSING from DF — features/gameplay/content — judged NOT as a DF-clone wishlist but
  through Bastion's IDENTITY (god-game/indirect · autonomous-not-4X · the remembering world · grim-tender tone).
  Format: DF HAS · BASTION STATUS · WANT-VERDICT · priority. Opportunity-finding for the backlog, NOT a build
  mandate (frontier+1 holds).
- **The headline finding:** we're missing LITTLE of DF's depth — most of it is already re-skinned across the ~50
  corpus docs. The doc's value is (a) the SPOTLIGHTS, (b) the NO-WITH-REASON identity guard, (c) the few genuine
  un-designed enrichments.
- **★ SPOTLIGHTS (best-fit gaps):** (1) DEITY/RELIGION → our god-game CORE — adopt DF's SPHERES/DOMAIN idea
  (GOD-DOMAIN pitch); (2) LEGENDS/HISTORICAL-FIGURES → our remembering-world — enrich chronicle granularity;
  (3) MOODS→ARTIFACTS → remembering-world-made-physical (✅ designed); (4) PROCEDURAL MYTH-GEN → per-culture
  founding-myths (MYTH-SEED pitch, authoring lane).
- **NO-WITH-REASON (identity guard):** labor-micromanagement (autonomous-not-4X), adventurer-mode (EMBODY≠RPG
  mode-switch), UI-opacity (legibility pillar), deep-economy (4X-drift), squad-micro (YES autonomous defense +
  god-levers, NO the micro), fortress-min-max (influence-not-command).
- **RANKED PITCHES → DESIGNER-SUGGESTIONS §12:** GOD-DOMAIN (top-fit) · MYTH-SEED · PERSONALITY-DEPTH enrich
  (dreams/prefs/ethics) · CHRONICLE-granularity enrich · DEFENSE design-pass @B8. Flagged GOD-DOMAIN + MYTH-SEED
  as the two best-fit new opportunities for Ben's greenlight.
- **Route:** pitches → DESIGNER-SUGGESTIONS §12; STATE report → architect.

## 2026-07-10 — BUILD MODE (readme/BUILD-MODE-design.md) [Ben-pull via architect; near-frontier, build-feeding]
- **What:** three build modes across the control spectrum — BASE-GAME (native) · AVATAR (embodied first-person) ·
  GOD-MODE (top-down world-shaping) — beside the shipped DESIGNATE-for-colonists path. Reuse-first + the
  designate-vs-direct model. → routed to reviewer for feasibility.
- **THE SPINE:** one authoritative placement SINK (`BlockChange::try_set`) + four FRONT-ENDS at four control-dial
  points: DESIGNATE (indirect, colony-built, ✅shipped) · BASE-GAME-direct (player, native) · AVATAR-direct
  (embodied) · GOD-direct (world-shaping, favor-costed). *One sink, four hands.*
- **CORE DISTINCTION (Ben's ask) nailed:** DESIGNATE = mark→colonist builds (materials+labor+reach) vs DIRECT =
  you/god place it yourself. Cost gradient: materials+labor DECREASES + favor-cost INCREASES as designate→avatar→god.
- **CODE SURVEY (cited, reuse verdicts):** native build EXISTS end-to-end (`CanBuild` comp/inputs.rs:8 →
  `place_block` client:2373 → PlaceBlock handler in_game.rs:203 → `try_set`:218); DESIGNATE exists
  (ToolMode::Designate(Build) → BastionPlaceDesignation → jobs); GOD-direct net-new-but-small (reuses try_set +
  `block_from_structure` block.rs:207 + the ToolMode paint surface). Reuse: BASE-GAME ~100%, AVATAR ~95%
  (rides EMBODY B12), GOD-MODE ~70% (new verb+favor-cost+palette).
- **PALETTE:** shared block-vocab (terrain BlockKind · structure components via block_from_structure · sprites);
  extend native `selected_block` → a palette/build-menu; one language across all direct modes.
- **UI:** native build-bar (base/avatar) + a DIRECT-BUILD tool in the overseer ToolMode (god-place, direct↔designate
  toggle on the same paint gesture, live favor-cost readout). Shares the paint/z_extent/flat_floor surface.
- **GUARDS:** world-shaping-NOT-unit-micro (identity) · FAVOR-cost the god-direct (D5, keeps it a god-power not a
  creative cheat) · chronicle major terraforms (E7) · EMBODY≠RPG (avatar guard) · terraform-determinism (existing).
- **DEPS:** AVATAR rides EMBODY (B12); GOD-direct favor rides B13. Ship base-game + god-mode first; avatar on B12.
- **Route:** reviewer (6-point feasibility ask: shared-sink soundness · CanBuild-bypass safety · block_from_structure
  server-callable · the wire msg-auth · does CanBuild attach to a controlled colonist body [the embody hinge] ·
  favor meter). Asset brief (palette swatches + god-place VFX) → ASSET_REQUESTS. STATE → architect.

## 2026-07-10 — PLAYER MODES / THE THREE FACES OF THE GOD (readme/PLAYER-MODES-design.md) [Ben-pull; supersedes BUILD-MODE framing]
- **What:** Ben reframed the "3 build modes" pull → **3 PLAYER MODES = THREE FACES OF ONE GOD** (god-centric):
  **SOVEREIGN** (god mode = home/default) · **WATCHER** (spectator/free-fly) · **INCARNATE** (avatar/first-person).
  Not three equals — SOVEREIGN is home; WATCHER + INCARNATE are excursions the god drops into and pops back from.
  Each face = its own camera + control-grain + POWERSET. Supersedes BUILD-MODE-design.md (its build survey folded in).
- **THE CONVERGENCE (Ben's creative expansion):** player-modes + the god-powers catalog + the banked GOD-DOMAIN pitch
  snap into one idea — *you don't switch a VIEW, you shift which FACE of the god you wear.*
- **CODE SURVEY — the switching is ~90% BUILT:** `CameraMode::{Overseer=sovereign, Freefly=watcher, First/ThirdPerson=
  incarnate}`; `bastion_enter/exit_overseer` (session/mod.rs:330-364) already toggles god⇄(freefly-or-body) keyed on
  `PresenceKind` (Spectator vs Character); F9 `BastionToggleOverseer` = the home toggle; `bastion_context()` derives
  the input scheme. Net-new = an EXPLICIT 3-way drop (Watch-vs-Walk) + the face POWERSETS.
- **ACCURACY CORRECTION:** "spectator has built-in build" is imprecise — build = admin-granted `CanBuild` (comp/
  inputs.rs:8, `/build` cmd.rs:2874), ORTHOGONAL to presence. Assigned native direct-build to the WATCHER face pending
  reviewer confirming `CanBuild` attaches to a Spectator-presence entity (the hinge, = the INCARNATE colonist-body hinge).
- **THE FACES + powers (creative, options surfaced for Ben):**
  - SOVEREIGN (home): world-shaping/god-direct build + designate + decree + smite/bless-at-scale (mostly exists).
  - ★ WATCHER (the creative heart): OMNISCIENCE — ①**Scry the Memory of a Place** (see the deeds done on a spot — the
    remembering-world made visible; the signature) ②Read-the-Soul (deep-inspect) ③Farsight/lift-fog ④Omen-glimpse
    ⑤Anoint/mark/name + the native free-fly direct-build. Guard: read/reveal/bless, never command.
  - INCARNATE (rides EMBODY B12): PRESENCE — bless-by-touch · hearten-the-near · personal-miracle · be-witnessed→faith
    + embodied player-build. Temporary descent (EMBODY≠RPG).
- **GOD-DOMAIN tie-in:** the domain (deep/harvest/dead/storm) COLOURS each face's powers (death-god's watcher reads
  graves; harvest-god's incarnate blesses crops) — the faces × domain = a personalized divine kit. Compose the two.
- **BUILD per face (designate-vs-direct kept clear):** SOVEREIGN = designate(indirect)+god-direct(favor); WATCHER =
  native direct(CanBuild); INCARNATE = embodied(reach+materials). One sink `BlockChange::try_set`, four gates.
- **REUSE:** switching/camera frame ≈ near-free integration; the real new design = the face POWERSETS (route via the
  GOD-POWERS-CATALOG pipeline, pick a v1 subset) + explicit 3-way switch + GOD-DOMAIN composition. Sequencing: (1)
  3-way switch + face indicator (small, near-frontier) (2) watcher native-build (3) sovereign god-direct build (4)
  powersets (5) incarnate @B12.
- **Route:** reviewer (6-pt: the 3-way switch feasibility · CanBuild-on-Spectator · god-direct bypass safety ·
  block_from_structure server-callable · Possessor as the embody vehicle · a spatial key to hang chronicle events on
  for Scry-the-Memory). Asset briefs (face indicator + face-shift VFX + watcher-sight overlays) → ASSET_REQUESTS.
  Creative WATCHER options surfaced → architect for Ben. GOD-DOMAIN convergence noted → DESIGNER-SUGGESTIONS §12.

## 2026-07-10 — FR7 verdict folded (BUILD-MODE/PLAYER-MODES) [reviewer FEASIBLE]
- Reviewer FR7: FEASIBLE. 3 folds into PLAYER-MODES §6/§8/§9/§10 + BUILD-MODE: (A) ★ chunk-loaded guard on the
  god-direct-build handler (top-down can hit an unloaded chunk → try_set silently drops; registry B13) — I'd guessed
  the invariant was persistence, it's chunk-residency; (B) favor gate SERVER-authoritative (client target_allowed =
  readout only); (C) AVATAR build-half 100% (CanBuild rides any controlled entity, not player-only). Sink shared,
  block_from_structure server-callable, wire auth inherited, Possessor = embody vehicle all confirmed. Open: a spatial
  key to hang chronicle events on for the WATCHER's Scry-the-Memory (god-powers lane).

## 2026-07-10 — GOD-DOMAIN × THE THREE FACES (readme/GOD-DOMAIN-design.md) [Ben GREENLIT; flagship god-game design]
- **What:** Ben greenlit GOD-DOMAIN + the three-face powersets COMPOSING (was a pitch → now a real pass). The god has
  a DOMAIN (sphere) that colours each of its three faces: three faces × domain = a personalized divine kit. + FR8
  verdict folded (faces FEASIBLE; CanBuild-on-Spectator CONFIRMED YES). The flagship — makes "you are a god" mechanical.
- **★ THE FORK RESOLVED — "the invocation SEEDS, the deeds DECIDE":** a soft founding invocation seeds the domain
  affinity-vector (agency); the TRUE domain is earned/drifting/named-by-deeds (earned-not-granted G11 holds — a god
  who invokes Harvest but raises the dead BECOMES a Dead-god). The domain is a SIBLING of the EPITHET (both chronicle-
  derived; epithet=the name, domain=the sphere). Justified through G11 + agency + everything-must-DO + remembering-world.
- **THE 6 DOMAINS (each ties a real system + makes one FACE shine):** DEEP (mining/terraform→Sovereign) · HARVEST
  (farm→Incarnate-bless) · DEAD (ancestors/chronicle→Watcher-Scry, the signature) · STORM (weather/omen→Sovereign-
  wrath) · FORGE (craft/artifact→Incarnate+Sovereign) · HEARTH (kinship/faith→Incarnate-be-witnessed). Grim poles
  Deep/Dead/Storm; tender Harvest/Forge/Hearth. Mixed gods allowed (dominant sphere = "your domain").
- **★ THE FACE × DOMAIN MATRIX (the heart):** a 6×3 table — each domain's signature per face. Your domain determines
  which FACE you lean into (Dead-god→Watcher, Storm-god→Sovereign, Hearth-god→Incarnate). The domain doesn't lock a
  face — it makes one SING.
- **MECHANICAL MODEL:** an affinity-vector {6 spheres}, invocation-seeded, deed-drifted via the shared chronicle-
  standing lib (G-C1, same as EPITHET/REPUTATION), scales every face-power. It's the god's own face of the
  remembering-world standing model.
- **COMPOSITION:** domain × ALIGNMENT → EPITHET (domain picks the pool, alignment the tone — Dead+tender="the Mourner",
  Dead+wrathful="the Bone-Crowned") · domain × FAVOR (aligned acts cost less; off-domain allowed but costs+drifts) ·
  domain × WONDER (the godspire reflects the domain).
- **WATCHER v1 LOCK (Ben: all 4 reads, domain-scaled):** Scry-the-Memory (signature, gated on DF-HIST + a coarse
  spatial key) · Read-the-Soul (ties inspection UI) · Farsight/lift-fog · Omen-glimpse. **ANOINT/MARK → v2** (my
  discretion: it's a WRITE needing consumers [champion/sacred-site]; the 4 v1 powers are self-contained reads).
- **SEQUENCING (5 stages, reuse-first, per-stage reviewer):** (1) 3-way switch+indicator [~90% built] (2) Watcher
  native-build [grant CanBuild on Spectator] (3) Sovereign god-direct build [chunk-guard B13 + server favor] (4)
  domain vector + face-powersets v1 [domain on the standing lib; Scry gated on DF-HIST] (5) Incarnate @EMBODY B12.
  DF-HIST is the keystone under the flagship; stages 1-3 ship uncoloured, domain (stage 4) enriches once chronicle lands.
- **Route:** reviewer (§9 — the domain-vector as a chronicle-standing-lib consumer [continuous-vector vs scalar];
  deed→sphere attribution via ChronicleEvent record(); favor coupling; the Scry spatial-key standardization). Domain
  sphere-glyphs + readout → ASSET_REQUESTS. PLAYER-MODES updated (v1 lock, domain cross-ref, FR8). STATE → architect.

## 2026-07-10 — Scry spatial-key RESOLVED (reviewer code-survey) → GOD-DOMAIN §6 + DF-HIST addendum
- Reviewer surveyed rtsim/src/data/report.rs: the place-key hook EXISTS (`ReportKind::Theft{…, site: Option<SiteId>}`)
  but `Death` is placeless — so Scry is FEASIBLE, not blocked, with 3 gaps: (a) GRANULARITY — key on a Vec3 tile-bucket
  or DF-ROOMS room-id, NOT SiteId (whole-town, too coarse); (b) UNIFORMITY — decide which event-kinds are place-scoped
  vs actor-scoped (feeds the D7 lock); (c) PERMANENCE — rtsim Reports DECAY (murder 15d/theft 1.5d), but Scry wants
  "the ground remembers forever" → DF-HIST needs a PERSISTENT tier distinct from rtsim's fading recent-events feed.
- Folded: GOD-DOMAIN §6 (Scry feasibility refined w/ the 3 gaps + the concrete answer) + DF-HIST ADDENDUM (3
  ChronicleEvent schema requirements: an optional spatial key on place-scoped events · a permanent tier vs rtsim's
  fading Reports · a domain sphere-weight per event). Principle established: **the chronicle = the permanent memory
  (the ages); rtsim Reports = the fading recent-feed (the last few days).**

## 2026-07-10 — GOD-DOMAIN FR9 VERDICT: FEASIBLE — folded (4 refinements strengthen the flagship)
- Reviewer FR9 (design-shape review; the stack is design-only so "fit the lib" = "design the lib to fit both", via the
  `Sentiments` built precedent). All 4 answered + folded into GOD-DOMAIN §3/§5/§6/§8/§9 + DF-HIST addendum + the §2
  registry:
  1. ★ The 6-vector FITS — not a distinct structure. G-C1 primitive = a bounded/decaying/named SCALAR; a standing is
     ONE (alignment) or a KEYED SET (reputation/sentiment/DOMAIN-per-sphere). Built precedent = rtsim `Sentiments`
     (map<Target,Sentiment> of decaying scalars). Domain = the Sphere-keyed set + soft-normalization.
  2. Deed→sphere = an INCREMENTAL ACCUMULATOR (store+nudge-per-event+decay), NOT a log re-scan (rtsim events prune →
     re-scan would fade old deeds). One tagged stream feeds domain+epithet+reputation+Scry.
  3. Favor = a BOUNDED COST-discount only: a cost floor > 0 (never free, D5) + generation ties to passive devotion NOT
     the act (no perpetual-motion runaway).
  4. Scry spatial key = a bucketed Vec3<i32> (universal; site/room are rollups from the pos) — resolves the FR8
     granularity+uniformity gaps in one field.
- ★ LOAD-BEARING: LOCK THE VOCAB EARLY (D7) — the `Sphere` enum is shared across ChronicleEvent sphere-weight +
  GodPower.sphere + domain keys; lock once before consumers harden. No new registry class (D7+D5 cover it).
- Flagship now has a fully buildable shape; the whole chronicle-dependent half (domain-vector + Scry) waits only on
  DF-HIST (D7 vocab-lock) being built. Stages 1-3 ship uncoloured ahead of it.

## 2026-07-10 — RESEARCH PASS 1/3: MINECRAFT BUILD-UX REFERENCE (readme/MINECRAFT-BUILD-UX-REF.md) [Ben-pull; builder reference]
- **What:** what makes Minecraft block-placement FEEL good, mapped to our 3-face build. Documented-lessons-only,
  builder-facing reference (NOT new design). Nearest-frontier of the 3 research passes.
- **THE FRAME:** Minecraft already ran our experiment — survival-build (reach+materials+hotbar) ≈ AVATAR ·
  creative-build (fly+instant+palette) ≈ WATCHER · WorldEdit (region ops) ≈ SOVEREIGN god world-shaping. The
  creative-vs-survival split = our god-direct(instant/free) vs avatar(labor/reach) split. Port, don't invent.
- **THE FEEL PRIMITIVES (cheap, high-impact):** ①block-OUTLINE target highlight (never aim blind — reuse the draped
  overlay) ②GHOST PREVIEW of the pending block/region (never place blind) ③reach (god/watcher long, avatar short)
  ④instant-vs-timed (matches the cost gradient) ⑤pick-block middle-click (HOOK EXISTS session/mod.rs:2067) ⑥drag-place
  (SUBSTRATE EXISTS — the designation drag-paint).
- **BULK TOOLS (3 tiers):** vanilla drag-place → Effortless-style MODES (line/wall/fill, two-clicks, hand-friendly) →
  WorldEdit region ops (//set///replace///stack///walls/brushes). Our ToolMode+drag-region+z_extent IS the substrate;
  add fill/replace/line as paint sub-modes (the god paints, doesn't type //set).
- **★ UNDO = mandatory + net-new for the Sovereign build** (a mis-terraform must be reversible; WorldEdit //undo is
  the most-used power-build feature) — bounded god-edit history; flag EARLY (retrofit is painful).
- **Top builder takeaways:** ghost-preview+outline #1 · pick-block nearly free · reuse drag-paint for direct-place ·
  UNDO mandatory · keep creative-vs-survival legible · grow toward WorldEdit via friendly paint-modes not a command line.
- **Reuse found:** the draped overlay (place-cursor), pick-block hook, drag-region paint — much of the feel substrate
  already exists; the real net-new is ghost-preview + undo + the palette UI.

## 2026-07-10 — RESEARCH PASS 2/3: PERF-AT-SCALE (readme/PERF-AT-SCALE-REFERENCE.md) [Ben-pull; feeds B7 specs]
- **What:** Factorio (megabases) + Songs of Syx (10k+ pop) documented perf lessons, keyed to our B7 specs (PATH-0/1,
  AUTON-2, LOD). Reference-for-the-builder, not new design.
- **★ HEADLINE:** Factorio's new pathfinder (FFF #317) IS our PATH-1 connectivity-group spec, shipping — chunk-
  contraction into passable/impassable COMPONENTS, store only PERIMETER tiles + cross-chunk connections (= a portal
  graph), ignore-entities-for-stability (terrain-only), Reverse-Resumable-A* cached per-goal, base pathfinder
  digresses. Direct validation + 3 details to copy: terrain-only portals · per-goal abstract caching · reverse-
  resumable.
- **6 principles → our specs:** (1) sleep/wakeup don't-tick-idle → AUTON-2 should be EVENT-DRIVEN (dirty-flag), not
  fixed-poll [Factorio's #1 lever] (2) deferred wakeup-list + per-tick budget → PATH-0 (add a priority request-queue)
  (3) hierarchical pathfinding → PATH-1 (the direct hit) (4) SoA+prefetch → keep hot ECS components lean (5)
  deterministic multithread of independent systems → .par_join (shared budgets stay sequential, the FR1 lesson) (6)
  individual-near/statistical-far → LOD SimulationMode (genre-validated).
- **Finding:** our B7 specs weren't guessing — the two highest-entity games converged on the exact shapes we sketched;
  the value is the 3 copy-exactly details (event-driven wakeup, per-goal abstract caching, terrain-only portals).

## 2026-07-10 — RESEARCH PASS 3/3: ONI FLUID/GAS/TEMP (folded into readme/DF-FLUID-FEASIBILITY-SPIKE.md) [Ben-pull]
- **What:** what ONI's CA gas+temperature model adds beyond DF's integer-liquid, for the DF-FLUID spike. Reference +
  a filtered verdict.
- **ONI = a cellular automaton, ONE material per tile** (deliberate simplification for realtime); adds beyond DF:
  GAS as a flowing element · TEMPERATURE as a per-cell field (conductivity heat-transfer, rate-capped, phase change) ·
  the one-element-per-cell discipline · a separate fixed-cadence sim thread.
- **★ VERDICT (the filter — Bastion is an indirect god-game, NOT a plumbing/thermal puzzle):** take DF's cheap
  integer-depth LIQUID base + a LIGHTWEIGHT coarse GAS-diffusion (miasma/syndrome clouds, not a pressurized gas CA) +
  a COARSE TEMPERATURE field (biome/depth zones + a few phase thresholds: freeze/boil/magma — ties DF-TEMP-BIOME-FX,
  unlocks water↔ice↔steam near-free) — NOT a full ONI per-cell thermal CA (that IS ONI's whole game; over-fidelity
  for us). If ever heavier: ONI's pattern (one-element/cell, rate-capped conduction, separate sim thread) is the
  guide — bank, don't build speculatively.

## 2026-07-10 — LORE-BIBLE §9 AUTHORING (readme/BASTION-LORE-CONTENT.md) [standing content lane; measured pace]
- **What:** the §9 content-authoring lane got a dedicated home (BASTION-LORE-CONTENT.md); lore-seeds for the remaining
  assets + the naming/dialogue substrate. Content-not-systems, grim-tender tone, Veloren-canon-safe.
- **★ FIRST UNIT (full): §A DOMAIN-KEYED EPITHET POOLS** — enabled by the GOD-DOMAIN greenlight (domain picks the pool,
  alignment the tone). ~90 seed titles across 6 domains × the tender→ambiguous→wrathful spectrum (DEEP: "the Stone-
  Father"↔"the Cave-in" · HARVEST: "the Green Mother"↔"the Blight" · DEAD: "the Mourner"↔"the Bone-Crowned" · STORM:
  "the Rain-Giver"↔"the Bolt" · FORGE: "the Maker"↔"the Cinder" · HEARTH: "the Hearth-Warden"↔"the Cold Hearth").
  Notes: mixed-domain blends, a signature-deed can PIN a name, drift renames down the column (the losing-is-fun arc
  made verbal), generic GOD-EPITHET pool = the domain-less fallback. Feeds the epithet/naming generator.
- **SEEDED EXEMPLARS (voice-setting, rest queued): §B founding-myths** (HUMAN "the field-given" done; Dwarven+ queued)
  · **§C folk-theologies** (the Tenders-vs-Dreaders priesthood quarrel done; per-domain tilt + the unstated origin queued).
- Lore bible §9 pointed at the content home. Holding at measured pace for the next pull / the builder consuming the backlog.

## 2026-07-10 — DF ORACLE PROBE: multi-unit 1-wide chokepoint behavior (readme/DF-CHOKEPOINT-BEHAVIOR-REF.md) [Ben-pull via translator; feeds B6 SOFT-0]
- **What:** live DF oracle probe (DF 53.15) of how DF moves multiple units through a 1-wide vertical chokepoint —
  reference for the builder's B6 SOFT-0 grind. Live where observable; schema-grounded where the fresh embark couldn't show it.
- **Q1 RESERVATION = NO (LIVE, decisive):** a unit stores its ENTIRE path privately (unit.path.dest + .path, observed
  a 19-step path); map occupancy marks ONLY the current tile (current occ_unit=true, ALL ahead-path tiles occ_unit=false).
  No forward reservation / path-claiming — conflict resolved REACTIVELY at step-time.
- **Q2 HEAD-ON = UNIT-SWAP** (two units moving opposite in a 1-wide passage swap tiles; else re-path/wait). Mechanism.
- **Q3 WAITING = no explicit state; IMPLICIT (LIVE representation):** a blocked unit = "job + path.dest set but path
  EMPTY (pathlen 0)"; it stands + re-requests a path (block_flags.repath_on_*). The queue is EMERGENT from occupancy +
  repath-retry, no queue structure.
- **Q4 TRAFFIC = soft A* COST bias, NOT single-file** (2-bit Normal/Low/High/Restricted, player-painted). Nudges, doesn't
  queue/reserve. DF players get single-file by BUILDING wider stairs, not via the engine.
- **Q5 DEADLOCK = yes it can (DF's famous traffic jams); breakers = unit-swap + reactive re-path + JOB-CANCEL on
  unreachable. NO teleport (fortress mode). The builder's teleport fail-safe is a Bastion addition beyond DF.
- **★ TAKEAWAY:** DF's model (per-unit paths + current-tile occupancy + reactive repath + swap + cancel-on-unreachable)
  is REACTIVE/SIMPLE, no reservation, no explicit queue — and LESS deterministic than the builder's plan (it IS what
  causes DF traffic jams; players design around it). So the builder's explicit density-promoted Waiting-state is the
  RIGHT call, not shortcuttable by DF. STEAL: (1) unit-swap for bidirectional 1-wide segments; (2) cancel-and-re-decide
  as the ultimate breaker OVER teleport. REASSURANCE: forward reservation is NOT needed (DF runs 200-dwarf forts on
  occupancy+repath alone). Routed → architect + builder.

## 2026-07-10 — LORE §9 authoring (unit 2): Dwarven founding-myth + Tier-1 dialect kennings (BASTION-LORE-CONTENT.md)
- §B DWARVEN founding-myth "the deep-called" authored (called DOWN by a knocking in the rock; the god = the Weight
  overhead that chooses not to fall; the unbroken pillar = the god's share; seal the dead in the stone) — deliberately
  contrasts the Human "field-given" (fled OUT / a Hand between / gives dead to the mound), making bible §3 "they are
  all looking at the same hand" concrete. Elf/Orc/Gnarling seed-directions noted (Tier-2 queued).
- §D DIALECT KENNINGS (Tier-1 Human + Dwarven) — an in-culture kenning vocabulary (NOT a conlang, per bible §7) for
  chronicle-in-culture + asset seeds ("the Underneath", "the god's-hand fell" = a cave-in, "a debt in the stone" = a
  grudge) + naming-convention seeds (dwarven stone/metal compound roots → holds). Feeds the name-gen + pilot lore-seeds.
- Also (Ben ask via architect): RVO/ORCA SOFT-1 candidate note → DESIGNER-SUGGESTIONS §14 (hybrid under the shipped
  Waiting-state queue; open-ground only). The CROWD-PATHING-METHODS-SURVEY (Ask 2) is QUEUED for the architect's
  post-B6-tag signal — NOT started (frontier discipline).

## 2026-07-10 — CROWD-PATHING METHODS SURVEY (readme/CROWD-PATHING-METHODS-SURVEY.md) [Ben-pull via translator; = activated Ask 2, narrowed]
- **What:** evaluate RVO/ORCA/flow-fields/cooperative-pathfinding(WHCA*) for adoption, recommend the SOFT-1 target.
  Ben activated the queued survey directly (narrowed to 4 + implement-decision). Design doc only (no code/run touch).
- **★ KEY FRAME:** crowd movement = TWO orthogonal layers — PATHING/deconfliction (WHCA*/reservation ↔ our queue [ad-hoc
  reservation] ↔ DF [reactive, probed]) + VELOCITY/local-avoidance (RVO/ORCA). Flow-fields = a 3rd thing (routing
  replacement). We already own a pathing-layer answer (the queue) → the high-value adopt is a VELOCITY-layer method.
- **VERDICTS:** RVO=subsumed by ORCA · ★ORCA=ADOPT (SOFT-1, open-ground/hauling; Apache-2.0 RVO2 → GPL-3-compat;
  weak at 1-wide so it does NOT replace the queue) · FLOW-FIELDS=NO (mass-agents-to-shared-goal; our workload is
  few-agents-many-goals; wasteful + poor 3D/vertical fit; replaces the router) · WHCA*=adopt the RESERVATION IDEA not
  the machinery (space-time table fights continuous physics; a SOFT-2 queue-upgrade, not a crate).
- **★ RECOMMENDATION: ORCA-under-queue.** Two-layer stack: pathing = Waiting-state queue (chokepoint) + climb-assist
  (vertical), shipped; velocity = ORCA (open ground, agent-agent), SOFT-1 adopt. Impl sketch: ORCA velocity-resolution
  between Chaser's desired-velocity and physics · agent-agent only (hard terrain stays with engine) · per-z-plane 2D
  (vertical excluded) · GATED by Waiting-state (queued agents out of ORCA — the clean seam) · reuse a Rust RVO2 port ·
  retire the hand-rolled soft-collision. Routed → architect + builder + translator.

## 2026-07-10 — SOFT-1 APPROVED/COMMITTED: ORCA-under-queue (Ben) — CROWD-PATHING-METHODS-SURVEY.md updated
- Ben APPROVED the recommendation → SOFT-1 = ORCA-under-queue is COMMITTED for implementation (post-B6-tag), not a
  candidate. Doc updated: a ✅ DECISION banner at top ("we ARE building this"), RECOMMENDATION + IMPLEMENTATION SKETCH
  marked as the committed SPEC. WHCA*-reservation retained as the SOFT-2 queue-upgrade candidate. Re-routed → architect
  + builder as the committed SOFT-1 spec.
- The spec (for the builder): ORCA velocity-resolution between Chaser desired-velocity and physics · agent-agent only
  (hard terrain stays with engine) · per-z-plane 2D (vertical excluded, climb-assist owns it) · GATED by the Waiting-
  state (queued colonists sit out of ORCA — the clean seam) · retire the density-soft-collision · vet the Apache-2.0
  RVO2 Rust port for GPL-3 fit. Two-layer stack (queue + ORCA) is the target.

## 2026-07-10 — CHOP SYSTEM REDESIGN (readme/CHOP-REDESIGN-design.md) [Ben directive via architect; frontier, implementation-spec]
- **What:** redesign Chop → mark whole TREES in a 2D area (RimWorld-style), fell each tree entirely (trunk+canopy),
  drop the wood. Ben: "don't need levels or flat-vs-sloped, just chop trees in the area + remove/drop all their wood."
- **REUSE SURVEY (grounded):** NO existing Veloren tree-fell mechanic (trees = static terrain). ★ BUT a tree ORACLE
  exists: `WorldSim::get_area_trees(min,max)`/`get_near_trees` → TreeAttr{pos:Vec2,seed,scale,forest_kind} (sim/mod.rs
  :2463,2481) — candidate tree XY bases (a SUPERSET; env filter decides real spawns; XY-only, no Z). Server-side
  accessible (World) but NOT in bastion_jobs today (terrain-only) → needs threading. Trees = terrain Wood(trunk/branch)
  +Leaves(canopy), NO runtime tree-id/entity. ★ Wood is used ALL OVER for BUILDINGS → naive Wood-match fells houses.
  Client already reads Wood|Leaves→"Tree"→Chop (session/mod.rs:690).
- **CURRENT CHOP (3 flaws):** job_wanted(Chop)=every Wood block in a down=2/up=0 SLAB → (a) matches building-wood, (b)
  slab misses the tall trunk+canopy, (c) ignores Leaves (the bug). Reuse: job pipeline/WorkType::Chop/Axe/XP/
  CHOP_DROP_ITEM/Block::empty/still_valid all reuse verbatim.
- **THE SPEC:** (1) DECOUPLE→2D area select; hide depth stepper; add DesignationKind::footprint_mode()→{Volume|Area2D}
  so UI+paint+server branch off ONE flag (Chop=first area-kind, extensible). (2) SELECTION: PRIMARY = get_area_trees
  candidates → confirm trunk at terrain → BOUNDED flood-fill Wood+Leaves from confirmed base (size/height cap);
  FALLBACK (no World) = terrain flood-fill gated by must-contain-Leaves + ground-rooted + size-cap. (3) MARK: per-tree
  OUTLINE box (reuse Chop-green overlay), extents echoed to client; v2=per-block tint. (4) FELL: job per Wood+Leaves
  block; Wood→empty+drop CHOP_DROP_ITEM, Leaves→empty+NO drop (fixes the bug). Yield-scaling FREE (per-Wood-block →
  bigger tree drops more).
- **Route:** reviewer (6-pt: ★World-threading-vs-fallback · candidate-confirm · flood-fill-excludes-buildings · per-tree
  overlay · footprint_mode classifier · Leaves-clear-no-drop). No new asset (reuse Chop-green outline). STATE→architect.

## 2026-07-10 — CHOP Phase-2 collapse note added (CHOP-REDESIGN §6 + future-work) [Ben noted-for-later, deferred]
- Ben: "any tree without a base to the ground should collapse." Added as DEFERRED Phase-2 (Phase-1 ships without it).
  Tree structural integrity: no ground-connected base ⇒ collapse+drop (reactive generalization of fell-whole-tree).
  Reuse: the engine's block_updates/sprite-revalidate-on-terrain-change hook (state.rs:675-784) = the trigger; the
  existing TREE-FELLING-design top-down/no-float flood-fill machinery; Phase-1's whole-tree flood-fill. Priced 2
  scopes: (A) tree-scoped collapse (recommended, bounded) vs (B) general unsupported-collapse (big net-new physics,
  separate feature). Logged to future-work-and-deferred-ideas.md so it's not lost.

## 2026-07-10 — CHOP-REDESIGN FR10 VERDICT: FEASIBLE — BUILD PRIMARY — BUILDER-READY [reviewer]
- Reviewer FR10: FEASIBLE, build PRIMARY (not FALLBACK). Spec updated to the verdict; now BUILDER-READY (last gate cleared).
  Key resolutions + improvements folded:
  1. ★ World access is CLEAN + smaller than feared: World is an ECS resource (lib.rs:555/:1628); Chop job-gen is in
     the HANDLER (in_game.rs:926), NOT terrain-only bastion_jobs → handler adds ReadExpect<Arc<World>>, computes the
     fell-set, hands positions to a Chop-aware job-gen; bastion_jobs STAYS terrain-only. No World-threading into it.
  2. Candidate confirm: REUSE `tree::tree_valid_at` (engine env-filter, per lib.rs:632 caller), not a hand-rolled
     Wood-probe (D1). TreeAttr "needs rework" comment confirms the superset caveat; tree_valid_at is the answer.
  3. ★ Building-exclusion: PRIMARY substantially safe (seeds from confirmed positions; residual roof-clip size-capped;
     COLONY BUILDINGS ARE ROCK so only worldgen-village Wood at risk; v2 = building-plot exclusion). New registry D15
     (overloaded-BlockKind conflation: Wood=trees AND buildings; discriminate structurally).
  4. Per-tree overlay: echo Vec<Aabb> on BastionDesignation; renderer draws N Chop-green boxes. Small.
  5. footprint_mode() affirmed. 6. Leaves trivial (Wood|Leaves, drop only Wood; Mine∩Chop dedup :534 + still_valid safe).
- Chop redesign is now design-complete + builder-ready (Phase-1); Phase-2 collapse banked (§6). → architect to schedule
  the build post-live-test-bundle.

## 2026-07-11 — WORLD STRUCTURAL INTEGRITY / CAVE-INS (readme/WORLD-STRUCTURAL-INTEGRITY-design.md) [Ben ELEVATED; deep design pass]
- **What:** Ben promoted the deferred cave-in feature to an active DEEP design pass ("both, but THINK HARD about the
  cave-ins"). Greenlit the reach-fix NOW (mine floating blocks from adjacent) + designed cave-ins properly for later.
- **★ THE ENTOMBMENT-TENSION VERDICT (the crux, front and center):** the tension (AR-2 made entombment impossible-by-
  construction; cave-ins deliberately bury) resolves by INVERSION — **the universal teleport-to-ground fail-safe is
  what MAKES cave-ins safe to add.** The distinction: the fail-safe guarantees "no colonist ever STUCK (sim softlock)",
  NOT "no colonist ever HARMED". A cave-in adds HARM not STUCK-ness → a caught colonist is EJECTED (reuse
  surface_teleport_dest bastion_jobs.rs:442) + INJURED (DF-WOUND) — "shoved out hurt", never "buried forever". RULE: a
  cave-in ALWAYS resolves every caught colonist to a terminal sim-progressing outcome; never indefinite-stuck.
  Lethality = a DIAL (enemies always crushable — cave-in as defense weapon; colonist-lethal opt-in; tender default).
- **Q2 SUPPORT+PERF:** reject global-connectivity (expensive, why it was deferred) + local-neighbour (wrong); use
  LOCAL BOUNDED connectivity — capped flood-fill from a removal's neighbours (O(cap) not O(world)); reuse the
  block_updates hook (state.rs:675-784) + the tree-collapse flood-fill. Honest limit: cap skips giant cavern-roof
  collapses (fine — v1 targets the realistic mining-remnant case = Ben's floating rock).
- **Q3 TRIGGER+GRIEF:** ★ the autonomous-grief trap — colonists dig what's designated + don't know not to pull the
  last support → 2 layers: (1) SAFE-BY-CONSTRUCTION dig gate (planner won't remove a block that collapses onto
  occupied space — routine labor never self-caves), (2) eject-and-injure backstop. Cave-ins fire only from player
  over-mining (WARNED — RimWorld support overlay + pre-dig warning), external hazards, or god-collapse-as-weapon.
- **Q4 SCOPE LADDER:** v0 reach-mine (greenlit now) → ★v1 MINING-REMNANT COLLAPSE (recommended — bounded, safe-by-
  construction, eject-injure) → v2 external+god-collapse → v3 full world physics (deferred). v1 = smallest rung that
  solves Ben's case without reintroducing entombment.
- **Refs:** DF (lethal, connectivity-support — lethality as a dial not default) · RimWorld (WARNED roof-collapse — our
  model) · DF-CAVERN Breach + §1a Hazard engine (emit through the one hazard pipeline).
- **Route:** reviewer (6-pt: ★eject-as-cave-in-resolution generalizes · bounded support check cost · safe-dig-gate
  predictability · warning overlay · hazard-engine fit · DF-WOUND dependency). STATE→architect (entombment verdict up top).

## 2026-07-11 — CAVE-INS FR11 VERDICT: FEASIBLE — entombment verdict + anti-grief BOTH HOLD; v1 builder-ready [reviewer]
- Reviewer FR11: FEASIBLE, build v1 (mining-remnant collapse) DIRECT — no DF-WOUND / no Hazard-Events blocker. The two
  decisive Qs both hold, with 2 architecture refinements folded:
  1. ★ Q1 EJECT GENERALIZES (surface_teleport_dest is PURE → per-colonist crush set). REFINED: target NEAREST-SAFE-
     OUTSIDE-THE-CRUSH, not the surface (surface yanks deep miners out of the mine); reuse the spiral-find-standable
     pattern, generalize the dest predicate. Entombment verdict HOLDS.
  2. ★ Q3 GATE AT COMPLETION not designation (designation-time = same-cost AND stale; collapse depends on removal-time
     terrain). Anti-grief = completion-check + eject (nobody buried), NOT designation pre-filtering. Anti-grief HOLDS.
  3. Q2 bounded flood-fill cap ~64, conservative (mass>cap assumed supported); run at bastion job COMPLETION not the
     general apply_terrain_changes (over-fires). 4. Q4 warning = a stale HINT (guarantee is Q3). 5. Q5 HazardEvent{
     volume,kind:CaveIn} is the home but §1a is designed-not-built → v1 direct + retrofit emission (lock KIND vocab D7).
     6. Q6 NO DF-WOUND dep (D9) — v1 injure = health-damage tick + fear tick; DF-WOUND enriches later.
- Folded all into WORLD-STRUCTURAL-INTEGRITY-design.md (§verdict + entombment mechanism + Q2/Q3). Cave-ins now design-
  complete + v1-builder-ready. The F6 fail-safe primitive doubling as the cave-in resolution is the whole entombment
  verdict, and it holds. → architect to schedule (v0 reach-mine already greenlit; v1 collapse builder-ready).

## 2026-07-11 — COLONIST COORDINATION + COMMUNICATION (readme/COLONIST-COORDINATION-design.md) [Ben live-test request; flagship-adjacent]
- **What:** fix the mad-scramble (a 27-level Mine where all colonists pile the top corner) via sector work-allocation
  + a player-visible DIALOGUE layer. 3-layer frame: HIGH=work-allocation (THIS, new) / mid=arbitration / low=collision
  (ORCA+queue) — coordination sits ABOVE, the swarm is an ALLOCATION bug not a collision bug.
- **DIAGNOSIS (code-grounded):** current allocation (bastion_jobs.rs:2566-2690) = INDEPENDENT GREEDY scoring; the
  TOP-DOWN bias (depth_score ±32) DOMINATES the weak DISPERSION-REPEL (clump_penalty +12) — the code comment says so
  outright — so everyone converges on the global top band. Structural, not a collision bug.
- **THE FIX — SECTOR PARTITIONING (two-level claim):** add a NEW sector-claim layer ABOVE the existing per-block claim.
  L1 sector-claim (partition footprint into fixed-size sectors; claim nearest-unclaimed → spread by construction) → L2
  existing per-block scoring RESTRICTED to the sector (top-down digs each sector top-down). Bounded rebalance: COMMIT
  to a sector until exhausted, then nearest-unclaimed / help-busiest (work-stealing, commitment-bounded = no bob).
  Threshold: only partition LARGE designations (small = existing repel). Partition along longest extent (sloped Mine →
  top/bottom bands). Composes w/ Build-2 standability + claim-commitment hysteresis.
- **★ DIALOGUE LAYER (~90% REUSE):** the bark API EXISTS — `chat_npc_if_allowed_to_speak(Content::localized(...))`
  (agent/behavior_tree/interaction.rs:179), speech-bubble render + settings exist, ANTI-SPAM CADENCE built in
  (allowed_to_speak). Triggers: sector-claim/rebalance/done (coordination events only). Sector→name mapper (north face
  / top / bottom). Net-new = the .ftl lines (I author, culture-flavoured, ties lore §D dialect) + 3 trigger hooks +
  the sector-name mapper. Emit-path reviewer-Q (ActiveJob suppresses rtsim activity → emit from bastion layer server-side).
- **DF/RimWorld:** DF BURROWS / RimWorld zones = the MANUAL version of our auto-partition (genre-proven); reservation≠
  allocation (RimWorld's swarm is a known complaint); pitfall = job-stealing churn (avoid via sector-commitment). → §E.
- **Route:** reviewer (5-pt: ★sector-layer-fits-claim-loop · partition/rebalance cost · sector-commit vs anti-bob ·
  bark-emit primitive for a bastion-colonist · threshold). §E class. No new asset (reuse bubble). STATE→architect.

## 2026-07-11 — COORDINATION revised to STIGMERGIC (Ben ant-steer) + PRIOR-ART-FIRST standing rule + locomotion wishlist
- Ben steered COLONIST-COORDINATION to real-world sim (ANT COLONIES). Revised the doc: added §2 PRIOR ART (stigmergy +
  response-threshold division-of-labour, Bonabeau/Theraulaz) + the 3-way fork (explicit/stigmergic/hybrid) →
  RECOMMEND STIGMERGIC (a decaying SATURATION FIELD generalizing the B6 dispersion-repel + a response-threshold;
  emergent, robust, SMALLER than a central scheduler, determinism-safe iff terrain-only+once-per-cycle; dialogue
  narrates the emergence). Explicit sector-partition demoted to fork-option-A (retained). Reviewer Qs updated to the
  stigmergic mechanism. §E9 added; RESEARCH-IMPROVEMENT-LOG swarm-intelligence thread flagged.
- ★ NEW STANDING RULE absorbed (Ben): PRIOR-ART-FIRST — every design pass surveys external prior art (games + sim/
  robotics/CS lit) before designing; lead reports naming the prior art adapted. Saved to memory (prior-art-first.md).
- Locomotion-smoothness wishlist (Ben, future-tier) logged → DESIGNER-SUGGESTIONS §15 + future-work (diagnose
  re-pathing-jitter / over-correcting-steering / missing-anim; prior-art = path-smoothing funnel + Reynolds steering +
  motion-matching; path-smoothing likely the shared root of smoothness + pathing-efficiency).

## 2026-07-11 — COORDINATION FR13 folded + message-crossing reconciled
- FR13 (reviewer): FEASIBLE, ~90% reuse — but it landed on the EXPLICIT option-A version (crossed my stigmergic
  revision + Ben's ant-steer). Its 3 core findings TRANSFER to the recommended stigmergic B + folded:
  1. ★ BARK PRIMITIVE resolved: `UnresolvedChatMsg::npc_say(uid, Content::localized)` (chat.rs:225) + emit via
     `ChatEvent{msg}` (interaction.rs:292/297); bastion_jobs adds a chat emitter (already has item_drop_emitter).
     ★ CORRECTION: `allowed_to_speak()` = a CAPABILITY check (behavior.can(SPEAK)), NOT a rate-limit → the bark needs
     its OWN cooldown (per-colonist last_bark: Time). (My "cadence built in" claim was wrong — corrected §3.)
  2. WHERE-state on the BOARD (HashMap<Uid, region>), NOT ActiveJob (per-job, recreated each re-claim — commitment
     must survive per-block re-claims). Applies to stigmergic region-commit too.
  3. ★ STICKY-until-exhausted = the anti-bob load-bearer: read the field/pick a region ONLY at commitment points (local
     exhaustion), NEVER re-eval every cycle — a WHERE-bob is WORSE than the block-bob. + help-busiest can mini-swarm →
     cap helpers/re-split. Folded §2. No new registry class (B14/AR-2 anti-bob applied).
- Message-crossing: the architect endorsed explicit + reviewer verified explicit, both before seeing the stigmergic
  revision. Reconciled: the verified SKELETON (board-keyed sticky-commit + npc_say bark) is SHARED, so FR13 de-risks
  BOTH; recommend stigmergic per Ben's steer, explicit as the reviewer-verified fallback (= hybrid-ready). Confirm w/ architect.

## 2026-07-11 — AGENT SYSTEMS RESEARCH (readme/AGENT-SYSTEMS-RESEARCH.md) [Ben-elevated foundational; + coordination FR13-REV builder-ready]
- **AGENT-SYSTEMS-RESEARCH:** deep field survey (coordination / decision-arch / game-prior-art / bleeding-edge /
  crowd) → recommended LAYERED architecture, prior-art-first at depth. Finding: our existing+planned systems already
  compose into the field-proven stack (utility+BT+stigmergy+blackboard+chronicle-memory+ORCA). Standout: the Stanford
  generative-agent memory→reflection→plan structure maps onto the CHRONICLE → adopt it DETERMINISTICALLY (gen-agent
  believability, no LLM). Honest verdict: scripted (utility+stigmergy+BT) beats learned (LLM/MARL) for the runtime
  brain; LLM/MARL offline-authoring only. Stigmergic CONFIRMED best-fit coordination v1, situated + evolution path
  (v1.1 band → v2 work-priority-grid UI + gen-agent memory → v3 HTN/GOAP). §E10-E11 + RESEARCH-IMPROVEMENT-LOG.
- **COORDINATION FR13-REV:** stigmergic v1 marked BUILDER-READY (determinism B0-safe: per-cell decay + fixed-order
  deposit + local-read/coord-tie-break; anti-bob via role-split field-steers-allocation + commit-on-exhaustion,
  hysteresis-band only for optional crowding-re-flow; free small-job degrade, no PARTITION_THRESHOLD). Explicit sectors
  = the FR13-verified fallback.

## 2026-07-11 — NIGHT_HORROR FULL INTEGRATION + the reusable CREATURE-INTEGRATION PIPELINE (readme/NIGHT-HORROR-INTEGRATION-design.md) [Ben directive; FIRST full asset→in-game→testable]
- **What:** take the finished night_horror asset (VLM 7.25, biped_large rig, wendigo lineage) fully into the game —
  register/animate/behavior/test-spawn. Spec'd as a REUSABLE 5-step PIPELINE (the first creature integration = the
  pattern for all after). Prior-art-first: the WENDIGO scaffold reused end-to-end.
- **THE PIPELINE (5 steps):** (1) REGISTER biped_large::Species::NightHorror (append-at-end wire-safe; copy wendigo
  rows: enum + name-key + AllSpecies field + body.ron + figure-manifest + loadout + sounds + loot + NpcBody FromStr) —
  the bulk, well-trodden; (2) MODEL = pilot's lane (figure-manifest form; rig conforms); (3) ANIMATION = ✅ AUTOMATIC
  (biped_large anims are SKELETON-based, shared across species → no authoring); (4) BEHAVIOR = Alignment(Enemy) → the
  enemy agent, tuned; (5) SPAWN/TEST = `/spawn enemy night_horror` (works once registered, zero new cmd code, cmd.rs
  handle_spawn:2212) + a `/bastion_arena` spawn action (handle_bastion_arena:6280).
- **★ THE HORROR (behavior):** a night-active STALKING predator (wendigo lineage) — lurks, ambushes ISOLATED colonists,
  fast claw-melee + a FEAR effect (→ panic/flee, ties DF-FOCUS + EMERGENCY-RUN), night-aggressive. A real defensible
  threat (the god wards/smites/champions; the colony fights/flees B8). Ties DF-NIGHT + DF-BEAST. v1 spawn = WILD
  (wendigo.ron reskin) + test-spawn; later = night-event + summon (DF-VILLAIN/curse).
- **Finding:** ~4 small wiring steps + a behavior-tuning delta + a 1-line arena action — the skeleton/anim/agent/spawn
  all exist. SMALL, as predicted. Reviewer (5 Qs: species-add touch-points + append-safety · ability-set/agent-tuning ·
  /spawn-by-string · anim species-agnostic · model-manifest contract). PILOT coord: the figure-manifest .vox form.

## 2026-07-11 — LORE §C: per-domain theology tilt (BASTION-LORE-CONTENT.md) [measured-pace lore]
- Authored the per-domain theology tilt (§C): the Tenders/Dreaders folk-theology quarrel × the 6 GOD-DOMAIN spheres —
  each domain colours what worship IS + what the two priesthoods argue (Deep=the-Weight-cares vs hasn't-fallen-yet;
  Harvest=Green-Mother vs withholding-Year; Dead=the-Mourner vs the-patient-Keeper; Storm=mercy-rain vs Flenser-Wind;
  Forge=gift-giver vs consuming-fire; Hearth=Kind-Host vs bars-the-door). Pattern: Tender reads PROVIDENCE, Dreader
  reads WEATHER; the god's alignment-drift proves one right (the §C quiet horror). Ties GOD-DOMAIN + epithet pools §A
  + DF-RELIGION worship-flavour. Remaining §C queued: the god's unstated origin · heretic/apostate readings.

## 2026-07-11 — NIGHT-HORROR FR14 VERDICT: FEASIBLE, BUILDER-READY + the reusable creature-pipeline CHECKLIST [reviewer]
- Reviewer FR14: FEASIBLE, small + reuse-heavy off Wendigo. Full new species is right; append NightHorror=13 (wire-safe).
- ★ Q4 CORRECTION (the gate caught my over-claim, like allowed_to_speak): anims are NOT fully free — the MOTIONS
  (idle/walk/run/attack) are skeleton-shared/free, BUT voxygen/anim/src/biped_large/mod.rs (:222+) has per-species
  SKELETON-OFFSET tables (per-bone match with NO _ => default) → adding NightHorror without offset entries = a
  non-exhaustive-match COMPILE ERROR. A small mandatory DATA add (offsets from part proportions), not motion authoring.
  THE non-obvious can't-miss for every future creature — featured on the pipeline checklist.
- ★ DEFINITIVE CHECKLIST (the reusable-pipeline core, ~10-12 touch-points, split by failure mode): COMPILE-ERROR set
  (biped_large.rs enum/name/AllSpecies · the anim offset tables · npc.rs NpcKind/FromStr/kind_to_body · sfx if
  exhaustive) · WRONG-LOOK/BEHAVIOR .ron set (ability_set_manifest · central/lateral part manifests · claw weapon .ron
  · loadout_builder · body.ron · loot · i18n) · OPTIONAL worldgen-spawn (rtsim tick/architect).
- Q2 behavior: Cavetroll/Ogre MELEE set (not wendigo_magic) + FEAR buff on the claw + stalk/ambush/night = agent knobs.
  Q3 /spawn CONFIRMED (NpcBody::from_str npc.rs:158 → NpcKind → kind_to_body → Body; /spawn enemy night_horror works).
  Q5 model handoff → pilot: one .vox per skeleton bone/part, per-species in central+lateral manifests; offsets derive
  from part proportions.
- night_horror builder-ready; the creature-integration PIPELINE is proven + checklisted. Slots after cave-in (Ben-pulled ahead).

## 2026-07-11 — NIGHT_HORROR model form DELIVERED (pilot) → spec FULLY concrete + builder-ready
- Pilot delivered the biped_large PART LIBRARY (11 segments, NATIVE wendigo frame) at asset-lab/vox/creature_night_
  horror/<part>.vox → builder copies to assets/voxygen/voxel/npc/night_horror/male/<part>.vox. ★ REUSE WIN: native
  wendigo frame → the manifest offsets AND the anim offset-table are the WENDIGO rows VERBATIM (zero derivation) —
  copy (Wendigo,Male/Female) rows, swap species-key + paths. Concrete offsets folded into STEP 2 (central: head/
  torso_upper/torso_lower + jaw/tail/second=armor.empty; lateral: shoulder/hand/leg[L/R x differs]/foot).
- DECISION: both body-types (Male + Female) — the offset-table match is exhaustive so a (NightHorror,Female) arm is
  needed to compile; pilot mirrors the female (matching the wendigo template, clean reference). Alternative documented:
  single-form monsters can alias Female→Male to skip the mirror.
- Two builder notes: (1) head = gaunt skull-core (antlers cropped, native content-coords → same wendigo head offset);
  (2) arm↔rib hang-gap is skeleton-posed (wendigo idle/walk), NOT baked — arms-further-out = a pose/skeleton tweak.
- night_horror is now FULLY builder-ready: engine spec (FR14 checklist) + model (delivered) + behavior + spawn all
  concrete. Awaiting the female-mirror parts + the architect's build scheduling (after cave-in).

## 2026-07-11 — FLEET RESUMED — LORE §E: coordination BARK LINES authored (BASTION-LORE-CONTENT.md) [frontier+1]
- Post-restart resume. Authored §E — the .ftl bark content for the queued stigmergic-coordination build (its one
  content dependency, flagged in COLONIST-COORDINATION §3): 3 triggers (region-claim / rebalance-help / region-done),
  `{$region}` arg from the sector→name mapper, keys npc-speech-bastion_coord_*. Tier-1 NEUTRAL+HUMAN+DWARVEN sets
  (§D kennings: "the deep end's mine", "stone's given what it had"); Elf/Orc = Tier-2 queued. ~6-8/trigger (right-sized
  to the bark cooldown; don't over-author). ASSET PASS: content-only, ZERO art (speech-bubble render exists).
- STATE: all recent designs builder-ready + queued (night_horror male-only+alias · Chop PRIMARY · coordination
  stigmergic-v1 · cave-in v1 [landing in code]); holding for architect build-scheduling.

## 2026-07-11 — GOD-HAND ENGINE INTEGRATION (readme/GOD-HAND-INTEGRATION-design.md) [Ben task; the how-to-build layer]
- **What:** the ENGINE INTEGRATION / how-to-build pass ON TOP of the existing GOD-HAND-design (design/anim/asset exist).
  Reuse-first, builder-ready — which systems to wire, concrete code, minimal net-new; so the builder doesn't re-derive.
- **FINDING: ~90% reuse.** The hand wires into shipped Veloren machinery; net-new = a hand SKELETON + its procedural
  anims (the design's known animation debt) + a singleton scene-figure hookup. Prior art named: B&W (a 3D hand at a
  screen→world raycast + target-kind verb) + Veloren's OWN systems (the real integration prior art).
- **THE 5-SUBSYSTEM MAP (reuse·wiring·minimal-new):** (1) POSITION+TARGET = ~100% reuse `targets_under_cursor`
  (session/target.rs:49 — returns the world-pos + entity under the cursor; overseer already calls it). (2) RENDER =
  a SINGLETON FigureState<HandSkeleton> + FigureModelCache<HandSkeleton> owned by the overseer scene at the cursor
  pos (reuse the whole figure model/mesh/GPU pipeline; net-new = HandSkeleton [anim/src/hand/, template off
  ItemSkeleton] + ~15 procedural anims + the singleton hookup). (3) INPUT→VERB = reuse bastion input.rs/ToolMode +
  the Bastion* messages (net-new = a verb→anim-state+message table). (4) SERVER DISPATCH = reuse GOD-POWERS-DISPATCH
  + mounting.rs Link/throw.rs/BlockChange/B-AG3/HAZARD-EVENTS (net-new = route the verbs, handlers mostly specced).
  (5) VFX+ALIGNMENT = reuse the Outcome bus + particle/glow (net-new = alignment-tint on the Outcome + hand-material
  drive). NO new particle system.
- **★ v1 SHOWPIECE SLICE = GH-A:** HandSkeleton + 4 anims (idle/point/grab_ground/select) + the singleton render at
  the cursor + grab-pan (reuse B1.5) + select→chronicle (reuse). Smallest thing that reads as "the god's hand."
- **Route:** reviewer (5 Qs — ★#1 the render architecture: non-entity singleton FigureState vs client-only entity).
  Asset-pass: the hand model needs the figure-manifest/BodySpec form (pilot handoff, like the creature pipeline);
  the ~15 anims are procedural (builder code), zero authored art. STATE→architect.

## 2026-07-11 — GOD-HAND INTEGRATION: DUAL-CURSOR / mode-dependent model folded (Ben refinement)
- Ben refinement: the god-hand is RESERVED for the god→WORLD INTERACTION mode (dramatic physical/cast acts where its
  scale reads divine); PRECISION (select/designate/mark) uses a SLIM cursor. Folded into GOD-HAND-INTEGRATION §0.
- ★ THE MODEL: two cursors gated by the active ToolMode. PRECISION (Pan/Inspect/Designate/Erase) = the SLIM cursor =
  the EXISTING overseer cursor (free OS pointer input.rs:202 + the designation reticle/overlay + cursor_ray mod.rs:236)
  — ~0 new. INTERACTION/GOD-POWER = the new GOD-HAND (the §2 singleton FigureState<HandSkeleton>).
- ★ THE SWITCH = the ToolMode IS the mode (derived, like bastion_context from camera-mode): add ONE ToolMode::GodHand
  variant; hand_mode(tool) gates hand-render-vs-slim-cursor. Player switches via the existing tool-cycle / UI-1 hotbar
  → the cursor morphs WITH the hotbar (ties UI-1's god↔mortal morph). Minimal-new = one enum variant + a derived gate.
- Prior art named: mode-dependent cursor is the universal god-game/RTS fix for B&W's "one clumsy hand for everything"
  (RTS arrow↔command↔build-ghost; Dungeon Keeper pointer-for-designate vs hand-for-grab). We split it.
- v1 slice (GH-A) updated: now PROVES the dual-cursor — hand in interaction mode (idle/point/grab-ground) + switch to
  precision → slim cursor → select→chronicle. Reviewer Q6 added. All still ~reuse + the ~15-anim debt.

## 2026-07-11 — THE MAIN / PRECISION CURSOR = THE GAZE (readme/MAIN-CURSOR-design.md) [Ben design-exploration via translator; design-lane background]
- **What:** the everyday overseer cursor (the other half of the dual-cursor pair; the ~95% cursor, was a ~0-cost
  placeholder). Options + recommendation. Prior-art-first.
- **★ FRAME:** the god's two faculties of presence — TOUCH (the hand, rare/dramatic/acts) + GAZE (this cursor,
  constant/attentive/watches). Ties the WATCHER face (PLAYER-MODES). Dual-cursor = the god's two ways of being present.
- **PRIOR ART:** DK tile-highlight-in-tool-colour · RTS build-ghost (telegraph the action, valid/invalid) · RimWorld
  per-designation colour · Populous/From-Dust world-anchored terrain-marker · B&W = the anti-pattern (one clumsy hand).
  Synthesis = a slim WORLD-PROJECTED reticle that MORPHS per tool + telegraphs the click. Ben's "gaze" = its folk-mythic reskin.
- **OPTIONS:** A ★THE GAZE (emissive cyan point on the voxel surface, morphs per tool) · B restyled reticle (cheap
  baseline) · C mark/seal sigil (v2 flavour) · D falling-light shaft (too big — reserve for the hand's casts).
- **★ RECOMMEND A (THE GAZE)** — hits all constraints (on-theme gaze↔touch · cyan=one god · unobtrusive · per-tool
  morph · folk-mythic). Legibility craft: EMISSIVE cyan core (glows in the dark deep — ties UNDERGROUND-LIGHTING) +
  a DARK contrast-rim (reads on bright) = legible on any background. Per-tool morph: pan=soft ring · inspect=keen
  pinprick · mine=downward pick-glint · build=rising mote · zone=fans into the tint · erase=dims (reuse zone_rgb tints).
- **REUSE:** cursor_ray raycast · the designation reticle + zone_rgb · the emissive/glow render · the free OS cursor.
  BUILD LADDER: v1-cheap=B (cyan-restyle existing) → v1-target=A (the Gaze) → v2-polish=C (sigils). Shares the god-hand's
  CYAN (one god: gaze=watch, hand=touch). Asset: emissive cyan point + ~6 per-tool variants (→ pilot at showpiece time).

## 2026-07-11 — MAIN-CURSOR: LEGIBILITY = the #1 requirement (Ben steer) folded
- Ben: LEGIBILITY is the PRIMARY constraint for the precision cursor (diverse environments — bright snow/sand, dark
  caves, forest, water, varied palettes → must read on ANY background). Confirms THE GAZE (emissive-core + dark-rim
  is exactly the legible-on-any-background trick). Folded into MAIN-CURSOR-design.md as a ★ dedicated section:
  - HARD DONE-WHEN: the Gaze + every per-tool morph variant stays legible across the FULL environment set (bright/dark/
    forest/water/busy-textured) — not "reads on grass", reads on everything.
  - BUILD-TIME LEGIBILITY CHECK (gate): verify the cursor reads on the full env set (in the asset-lab VLM legibility-
    panel spirit) → flagged for reviewer/play-tester to GATE at showpiece build. Aesthetics bend to legibility.

## 2026-07-11 — GOD-HAND-INTEGRATION FR16: FEASIBLE-WITH-CHANGES, GH-A a GO — folded [reviewer, 7-agent workflow]
- Reviewer FR16: ~90%-reuse thesis HOLDS, GH-A GO, 6 sharpenings folded (not a rework):
  - ★ Q3 (the one ships-broken): CORRECTED §1 — use the overseer's bastion_point_under_cursor/bastion_pick_entity
    (ortho + free-cursor correct), NOT targets_under_cursor (camera-forward = screen-center under the overseer's free
    cursor + ortho cam → detached hand). Mandatory source-swap. (My plan cited the FPS ray — real error, well-caught.)
  - Q1 render de-risked: COPY scene/simple.rs (a lone FigureState entity/body:None, char-select precedent — tighter
    than the entity-keyed FigureMgr I cited). GUARDRAIL: keep hand::Body STANDALONE (like VolumeKey); NO
    comp::Body::Hand (avoids a B18-class 139-site Body-match fanout).
  - Q2 re-priced: MODERATE/asset-heavy — the 6 voxel bone models (palm+5 fingers) are the LONG POLE (not "small×3").
  - Q4: Outcome::GodHand + DUST new_colored per-emit-RGB, NOT reagent-tint (Reagent has no gold/black).
  - Q5: the god-power verbs are B13-BLOCKED (GOD-POWERS-DISPATCH + favor = DESIGN-ONLY, 0 code, chat stubs) → "no new
    economy" corrected; PAINT is fully real (server handler + job-gen). GH-A select done-when DESCOPED to "Selected:
    {who}" (the life-panel doesn't exist — budget separately).
  - Q6: dual-cursor confirmed (mirrors bastion_context-from-mode); one small net-new = a set_cursor_visible decouple.
  - GH-A = GO, 3 must-dos (Q3 pick-swap · standalone-Body+simple.rs · descope select). GH-B..F build-first-blocked on B13.
- All folded into GOD-HAND-INTEGRATION-design.md (verdict banner + §0/§1/§2/§4/§5 + the v1 slice + the minimal-new table).

## 2026-07-11 — LOCOMOTION PROGRESS MODEL / FR15-TIGHTDIG re-spec (readme/LOCOMOTION-PROGRESS-design.md) [architect-pulled; frontier; LOAD-BEARING]
- **What:** re-spec FR15-TIGHTDIG (reverted-by-measurement — 9 variants each traded a scenario leg). ROOT (code-
  confirmed): the stuck-economy measures progress by BEELINE distance (pos.distance(target), best_dist-shrink,
  bastion_jobs.rs:1710/1722/1965) → a committed-path steer (routing around obstacles) breaks its semantics.
- **★ PRIOR ART (prior-art-first):** ROS move_base — oscillation-detection measures ACTUAL DISPLACEMENT (oscillation_
  distance), NOT distance-to-goal; its recovery escalation = our stuck-economy. Nav-mesh path-follow = ARC-LENGTH
  corridor progress (dissociates from cross-track/beeline error). The field NEVER uses distance-to-goal as the stuck
  metric — our beeline test is the anti-pattern both solved.
- **THE DESIGN (unifies the builder's 2 directions):** (2) DRIVE-OWNED stall-detection + (1) PATH-ARC-LENGTH, refined
  by the ROS actual-displacement primary. Metric = "displaced ≥ MIN_DIST AND (committed-path ⇒ arc-length s advanced)"
  over a window — steer-AGNOSTIC (correct for beeline AND committed-path). Architecture = the drive emits ONE
  progressing/no_progress_time signal; the rescue-economy consumes THAT (input-swap at :1722/:1966); the economy chain
  (watchdog→...→teleport, the no-entombment guarantee) is KEPT verbatim. Ends the per-leg trades (the metric, not the
  tuning, was wrong).
- **★ PART 1 PREREQUISITE (folded, do FIRST):** b58 telemetry is scheduling-seam-dominated ([5k..46k] identical-binary)
  → the 9 "trades" may be NOISE. Fix measurement BEFORE tuning: tick-determinism (real fix, ties B8) OR paired-A/B
  (interim — per-scenario DELTA, common-mode cancels). Gates PART 2.
- **REUSE:** the whole economy kept; the path/Chaser has the waypoints → s; displacement = a ring buffer. Surgical.
- **DONE-WHEN:** Part-1 measurement valid FIRST · the progress-checker replaces beeline · BOTH tight-dig + beeline legs
  pass in ONE build (no trade) via paired-A/B · determinism preserved · teleport backstop intact.
- **Route:** OPUS reviewer local_7e72649b (LOAD-BEARING per the new two-tier routing — stuck-economy + determinism).
  → architect (the row-31 re-spec deliverable).

## 2026-07-11 — LOCOMOTION-PROGRESS FR17 VERDICT: FEASIBLE-WITH-CHANGES, safety CLEANLY SATISFIED — folded, BUILDER-READY [Opus reviewer]
- Reviewer FR17 (Opus, adversarial): FEASIBLE-WITH-CHANGES; metric diagnosis right. All folded:
  - ★ (d) SAFETY — the no-entombment teleport stays PROVABLY intact, DOUBLY: the ultimate teleport is fed by a SEPARATE
    movement-INDEPENDENT field `stuck_watch` (bastion_jobs.rs:940, fired :2963-2978, "seconds below grade") — ORTHOGONAL
    to the beeline best_dist/stuck_time I swap (:1722/:1966, which drives only the TRAVEL-watchdog chain). The input-swap
    CANNOT touch the teleport; a below-grade-stuck colonist teleports even if the new metric wrongly reads "progressing."
  - (a) arc-length s feasible — from bastion_full_path (:1912) waypoints (confirm cheaply iterable per-colonist).
  - (b) input-swap clean at the stuck_time resets — ONE adjacent adapt: re-express the :1958 steer-switched rebase
    (reads best_dist) under the progressing/arc-length model IN THE SAME CHANGE (no dangling beeline reader).
  - (c) SEQUENCING: paired-A/B FIRST (unblocks the fix delta now), tick-determinism the follow-on right-fix (bigger, ties B8).
- Folded into LOCOMOTION-PROGRESS-design.md (verdict banner + the doubly-safe REUSE note + the :1958 adapt + the A/B-first
  sequencing). Re-spec BUILDER-READY — the drive-owned steer-agnostic progressing signal, no-entombment untouched.
- Ten reviewer passes (FR7-17), all FEASIBLE(-with-changes). → architect to slot as the row-31 builder block.

## 2026-07-12 — DONE B6-HAUL STORAGE MODES (addendum, row-34) · `readme/B6-HAUL-STORAGE-MODES-design.md` [GENERAL DESIGNER]
Architect-pulled (frontier-timed: before the builder reaches master-list row 34 "Typed jobs, stockpiles, auto-haul,
reservations", after LOD-1). Folds TWO first-class storage modes into B6-HAUL:
- **PRIOR ART (named first):** DF stockpile-tiles vs bins/barrels/pots · RimWorld stockpile-zones vs shelves (shared
  StorageSettings = filter+priority) · Songs of Syx stockpiles vs racks. SYNTHESIS = one storage abstraction
  {filter, priority, capacity, occupancy}, TWO densities (loose ground / dense container), ONE haul job.
- **Mode A GROUND stockpile = near-free:** rides the existing B5.5 `BastionPile` substrate (persistent, conserved,
  vanilla-merge) + the Stockpile zone. A ground cell IS a pile cluster; deposit = the existing persistent-merge drop.
- **Mode B CONTAINERS = the one net-new:** chest/barrel/basket **sprites already exist** (zero v1 assets) as PROPS,
  BUT the honest correction — **vanilla chests are read-once loot spawners, NOT deposit-able inventories**
  (`Collect`→loot table→`into_collected()`), so containers need a small **Bastion-side container store**
  (`HashMap<pos, ContainerSlot{filter,capacity,contents,sealed}>`, sibling of the JobBoard). "Reuse the chest
  inventory" would ship broken.
- **UNIFIED job:** one haul job, destination = `StorageTarget{Ground|Container}`; selection = filter-match →
  priority desc → nav-mesh distance asc (NOT beeline — the stuck-economy lesson). Ground-vs-container is a
  selection outcome, not two colonist-AI paths.
- **RESERVATIONS (the row-34 requirement):** extend the existing `claimed_by` to reserve **one unit of destination
  capacity**; effective-free = capacity − stored − reserved; the (N+1)-th hauler skips a fully-reserved destination.
  Unifies ground-cell + container slot-conflict under one rule; rides the existing claim-release sweep (dead-hauler
  reservation releases with `claimed_by`); integer/deterministic; conservation-safe. Stale-reservation edge called
  out + asserted in the scenario.
- **In-zone vs standalone:** BOTH (DF bin-on-tile / RimWorld shelf-in-zone densify; standalone = point-of-use);
  in-zone containers inherit the zone filter. Sequenced HAUL-STORE-0 (ground + unified job + reservation) →
  HAUL-STORE-1 (containers + Bastion store) → HAUL-STORE-2 (filter/priority + in-zone density). 5 open Qs → architect.
- **Route:** SONNET reviewer local_5f3f9b01 candidate (routine spec-feasibility — "does this reuse hold"); the ONE
  load-bearing claim to verify = the container store must be Bastion-side (vanilla chest not deposit-able). → architect
  for row-34 slotting.

## 2026-07-12 — DONE B7 NEEDS/MOOD (builder-complete, row-44, Opus-tier) · `readme/B7-NEEDS-MOOD-design.md` [GENERAL DESIGNER]
Architect-pulled at the frontier (B7 had only an 11-line build-report paragraph). Builder-complete design of the
3 flagged underspec areas, reuse-first (the `Needs`/`Mood` shells exist, unwired):
- **PRIOR ART (named):** mood = DF thoughts→stress→tantrum-spiral · RimWorld base+Σthought-offsets + break-threshold
  staircase + probabilistic break-roll (the spine) · Sims weighted-need→mood→free-will. Bed = DF room-claim /
  RimWorld one-owner + communal bedrolls. Preemption = RimWorld ThinkTree (need-givers ABOVE work-givers) / DF
  need-override.
- **THE SHAPE:** not 3 features — ONE meter-decay loop + ONE mood formula + ONE reusable PREEMPTION MECHANISM
  (the load-bearing build-once). Both preemption halves REUSE existing seams: the priority-TIER compare
  (bastion_jobs.rs selection loop, the is_access||Ladder→saturating_add(1) precedent) for self-assign, and the
  claim-RELEASE seam (to_release → free claimed_by → active_jobs.remove → rtsim_controller.activity=None) for the
  interrupt. No new steer.
- **★ Two-layer clarity (avoids collision):** B7 owns SURVIVAL Needs{hunger,rest,recreation} (comp/bastion.rs:40);
  the FOCUS-0 `Need` venue enum (bastion.rs:631) + personal_needs (bastion.rs:991) are FOCUS's — but FOCUS-1's
  self-generated venue-need-jobs REUSE B7's preemption mechanism. B7 = mechanism + hunger/rest callers; FOCUS =
  venue callers. Flagged for the architect to note so FOCUS-1 doesn't re-derive preemption.
- **Mood formula:** mood = clamp01(BASE + Σ w_need·shortfall(need) + Σ w_thought·decay(thought)); thoughts read
  from the CHRONICLE (DF-HIST, landed — the natural thought-store, no parallel memory); order-free/recomputed
  (deterministic); break = a rising threshold STAIRCASE + probabilistic roll (v1 = despondent-only; tantrum/berserk
  deferred as combat-adjacent).
- **Bed model:** the EXACT B6-HAUL container-store parallel — Furniture bed sprites exist (Bedroll + biome
  Bed{Head/Middle/Tail}) as inert props + a Bastion `BedSlot{kind,owner,occupant,quality}` side-table co-located on
  the JobBoard (the confirmed home). Placement = a Build job (Ladder precedent); occupancy = a capacity-1
  reservation (the container-reservation rule → two colonists never double-book a bed); REST is the fully-closed v1
  loop (build→tire→sleep→recover, supply-free).
- **★ Preemption safety (the Opus-tier crux, spelled out):** a need-job IS a travel job → rides the stuck-economy +
  the movement-INDEPENDENT stuck_watch teleport (FR17 guarantee — preemption CANNOT suppress it). Anti-livelock via
  a HYSTERESIS band (NEED_INTERRUPT≪NEED_SATISFIED) + unreachable-need degrades to ENDURE→decay→breakdown (an honest
  suffering colonist, never a frozen sim) + a preempt cooldown. Deterministic + conservation-safe (freed work claim
  returns to the board; bed reservation released by the same sweep on satisfy/cancel/death).
- **Hunger honestly half-open:** decay+mood+eat-when-available ship; the SUPPLY is DF-FARM/DF-COOK-gated (a starving
  colony = a correct legible failure state, honest-inert, not a bug).
- Sequenced B7-0 (meters+decay+mood) → B7-1 (bed + closed rest loop) → B7-2 (preemption — THE build-once, Opus-gate)
  → B7-3 (hunger/eat + breakdown, reuses B7-2 with zero new preemption code). 6 open Qs (5 resolved-with-rec, 1 FORK:
  does bed-ownership ship in B7-1 or B7-3). → architect for row-44 slotting.
- **Route:** OPUS reviewer local_7e72649b (LOAD-BEARING — the preemption mechanism + the no-entombment guarantee
  under preemption; safety-net-adjacent per the two-tier routing).

## 2026-07-12 — DONE TEST-INFRASTRUCTURE AUDIT (Ben-commissioned via architect) · `readme/TEST-INFRASTRUCTURE-AUDIT.md` [GENERAL DESIGNER]
Full inventory + gap analysis + phased roadmap to agent-runnable end-to-end testing ("validate the WHOLE game
without Ben's eyeballs"). Inventory VERIFIED in code (not re-derived). Prior-art-named throughout.
- **★ The reframe that reorders it:** TWO automation paths for the visual/interactive/E2E gaps — (1) HEADLESS/
  programmatic (CI-grade, needs engine seams) and (2) COMPUTER-USE driving the REAL client (the fleet ALREADY has
  this + the VLM rubric — no seam). So we DON'T wait on the deep render seam to start replacing Ben's eyeball pass:
  point computer-use+VLM at the real client NOW (Phase 1), build the headless seams for every-commit coverage
  (Phases 2-3).
- **★ Reuse primitives the code survey found (shrinks the build-blocks):** GPU screenshot readback ALREADY EXISTS
  (voxygen/src/render/renderer/screenshot.rs — TakeScreenshot → image::RgbImage); `Client` is renderer-AGNOSTIC +
  a HEADLESS example exists (client/examples/chat_cli.rs; Client::tick(ControllerInputs) "useful for bots"); the
  GameInput layer already carries Bastion verbs (BastionRotateLeft/PauseToggle/SpeedUp…); rtsim save ALREADY has a
  version-check (RTSIM_IGNORE_VERSION / VersionMismatch). CI CONFIRMED vanilla (grep: zero bastion refs).
- **The 3 real ENGINE SEAMS (build-blocks):** S1 headless-render mode for voxygen (offscreen wgpu, no winit window;
  reuse the existing readback) = HIGH, the capstone; S2 synthetic-input hook (headless tier = reuse via Client::tick;
  visual tier MED); S3 a "bastion bot" client frontend (chat_cli-shaped, MED, mostly reuse). Everything else = reuse
  or process.
- **Gap taxonomy A-K refined + reprioritized** (each: what it tests · the AGENT E2E path = the crux · prior-art ·
  effort/deps · priority). Merged, added golden-master snapshots to H, flagged I (MP desync) as a SCOPE-CHECK →
  likely DEFER (single-overseer god-game — determinism still pays for regression/replay, not MP).
- **PHASED ROADMAP:** Phase 0 = ARCH-003 determinism keystone (in flight). Phase 1 (LOW, no seam, immediate) = wire
  CI (G) + regression corpus (H) + computer-use+VLM eyeball (A/B now-tier) + save proptest (E) + sim-perf gate (D).
  Phase 2 (MED) = the bot client S3 + headless input S2 → full E2E (C) + scripted interaction (B) + fuzz (J) + soak
  (F) + coverage (K). Phase 3 (HIGH) = headless render S1 → scalable visual-regression (A CI-tier) + visual E2E.
- **Prior art named (appendix):** Factorio headless+determinism · Quake demo record-replay · Unity Graphics Test
  Framework / Unreal Screenshot Comparison (golden-image) · Epic Gauntlet (client-driving) · proptest/AFL (fuzz) ·
  criterion (perf-gate) · cargo-llvm-cov (coverage) · Chaos Monkey (soak) · VLM-as-judge (extend our asset rubric).
- 5 open Qs / scope-checks (MP-in-scope? for Ben; Phase-1 order; S1 spike; gameplay VLM rubric; CI GPU/lavapipe).
  → architect to route build items + bring the summary to Ben.

## 2026-07-12 — ENRICHED TEST-INFRASTRUCTURE AUDIT (Ben addendum: real external search) · same doc [GENERAL DESIGNER]
Per Ben's 3 directives on the audit: (1) did a REAL external web search (not memory) — cited concrete named
systems; (2) DROPPED multiplayer/desync (item I) as explicitly out-of-scope; (3) went DEEP on the fused A+B+C
flagship cluster (agent PLAYS + VISUALLY JUDGES the live client) with a staged path + the test-oracle problem.
- **New §4½ (the flagship cluster, deep):** (a) INPUT injection — Unreal GAUNTLET architecture (external harness
  + in-client TestController puppeteer; TestExecutor) = the direct S3 model; record-replay VCR (Unity/Quake:
  state+inputs+seeds → replay → check-data) as the determinism-first regression. (b) HEADLESS RENDER — lavapipe/
  SwiftShader software-Vulkan + wgpu-headless + a Vulkan-SDK GitHub Action = CI-without-GPU; de-risks S1. (c) THE
  TEST-ORACLE PROBLEM — the layered oracle: deterministic STATE asserts (Factorio CRC-after-every-test) = hard
  gate → perceptual/SSIM diff w/ tolerance (pdiff/pixelmatch, Rive golden images) = deterministic visual gate →
  VLM grading = ADVISORY ONLY (research: "AI diffing is non-deterministic → can't be a gate") → metamorphic
  relations = oracle-free. STAGED path Stage0 determinism → Stage1 seams → Stage2 cheap deterministic asserts →
  Stage3 deterministic visual gate → Stage4 VLM-advisory + computer-use exploration (loops findings back to H).
- **New taxonomy item L — METAMORPHIC TESTING** (the gap the search surfaced): Metamorphic Relations bypass the
  oracle problem — mirror→mirror, scale→scale, reorder→identical, pause→resume-identical; uniquely cheap on a
  deterministic sim. P2.
- **Validation folded:** EA SEED / Ubisoft La Forge RL agents + LLM-agent-testing (arXiv 2509.22170) run AI as
  exploratory ADVISOR backstopped by deterministic asserts — never the hard gate — exactly the §0 framing;
  Battlefield-V ≈ 300 work-years manual = the "why automate" stat; Antithesis = the modern DST reference
  validating our harness+determinism direction.
- **§6 prior-art appendix rewritten as REAL cited sources** (15 URLs: Gauntlet/EA-SEED/Factorio-FFF/Antithesis/
  arXiv-2509.22170+2103.06431/metamorphic/Rive/pdiff/Unity-replay/wgpu-headless…). Roadmap Phases 2-3 now map to
  the §4½ stages. → architect (updated doc ready).

## 2026-07-12 — DONE MIND-LOD needs-decay fork (B-AG3.2 / row 41.2, async de-risk) · `readme/MIND-LOD-NEEDS-DECAY-design.md` [GENERAL DESIGNER]
Architect-pulled async (parallel prep for the deferred 41.2). The fork: unloaded colonists' needs FROZEN (B7-0
decay is loaded-tier only → promote-back stale-full) — (a) coarse decay vs (b) deliberate freeze.
- **PRIOR ART (real search, cited):** RimWorld = MOTHBALL/freeze for distant world pawns BUT abstracted COARSE
  needs-tick for the player's own off-map caravans (fidelity scales with RELEVANCE). DF = coarse ABSTRACT
  world-tick (historical figures drift/age/die abstractly; armies tracked; static entities sit). Songs of Syx =
  individual sim where loaded, AGGREGATE regional where not. SYNTHESIS: nobody freezes OR full-sims — coarse-but-
  alive for the player's population, freeze/aggregate for the background.
- **★ CODE REALITY inverts the fork's premise:** (1) the freeze is precise — B7-0's persistent mirror
  BastionColonist.needs:Option<(h,r,rec)> + .mood (bastion.rs:1216/1220) captures EVERY LOADED TICK, frozen while
  unloaded (the state to decay already EXISTS on the record); (2) the coarse tick ALREADY EXISTS —
  SIMULATED_TICK_SKIP=10 seed-staggered + accumulated-dt (npc_ai/mod.rs:100,131) runs unloaded AI at 1/10 duty.
  So option (a) is ~FREE (a scalar relax riding the proven tick) → the fork is DESIGN (should the colony live
  off-screen?), NOT perf.
- **RECOMMENDATION: (a) HYBRID — throttled coarse TEND-TO-EQUILIBRIUM decay, bounded.** Not free-fall (a colony
  left a day would return starving — wrong+punishing), not freeze (breaks the living-world pillar). Needs relax
  toward an equilibrium set by PROVISIONING (beds+food stored → content target; bare → strained target) — the
  architect's C-5 class. Cheaper AND more correct AND continuous across the load boundary (no discontinuity at
  promote). Mood = recompute-on-promote (derived, needn't tick unloaded).
- **★ The desync-tolerance BOUNDARY ("what can safely drift"):** the invariant = NO SILENT IRREVERSIBLE LOSS
  off-screen (borrowed from no-entombment). Needs/mood MAY drift (soft, self-correcting on reload); the
  breakdown/DEATH threshold is CLAMPED — an unloaded colonist may go strained but must NOT silently die/break-
  spiral off-screen (irreversible step only happens loaded, witnessed). Skills/inventory/bed persist exactly, no
  drift. "The world changed while you were away, but nothing was silently lost."
- Sequenced AG3.2 (small relax-step + clamp) w/ Done-when (provisioned→content, bare→strained-but-alive-clamped,
  deterministic, continuous-at-boundary). 4 open Qs — the ONE real fork is Ben's (§7 Q1: permadeath-while-away? —
  rec CLAMP for v1, off-screen mortality is a stakes choice not a technical one).
- **Route:** SONNET reviewer local_5f3f9b01 candidate (routine — verify the SIMULATED_TICK_SKIP reuse + the
  mirror-decay feasibility); escalate to Opus only if the clamp/determinism needs the safety-tier eye. → architect.

## 2026-07-12 — DONE CHOP FELLING REFINEMENT (Ben-direct gameplay-feel) · `readme/CHOP-FELLING-REFINEMENT-design.md` [GENERAL DESIGNER]
Ben (FARM build): trees dismantle BLOCK-BY-BLOCK, don't read as felling. Ben's direction: CHOP = a progress-bar on
the tree BASE → completion fells the WHOLE tree as one "timber" event. A REFINEMENT of FR10 (detection is right,
work model is wrong). Design doc + BUILD PACKET. NOT critical-path (behind AUTON).
- **ROOT CAUSE (code-confirmed):** FR10's detect_trees (bastion_chop.rs) + tree_fell_set (bastion_jobs.rs:947)
  correctly compute the WHOLE-tree fell-set AND the ground base (seed_z). But the handler calls place_chop_cells
  (:1466) → ONE Chop job PER BLOCK → block-by-block nibbling (+ unreachable canopy cells left floating). The fix
  collapses N jobs → ONE base-cut job that fells the stored whole-set on completion. Fell-set MATH untouched (per
  Ben/architect: work model + result change, not felling math).
- **PRIOR ART (real search, cited):** DF = one fell job → whole tree becomes logs, cut at the LOWEST point =
  logs teleport to ground SAFELY (validates base-cut = safe, no mid-trunk free-fall). Minecraft TreeCapitator/
  TreeChop = break one log → whole connected tree removed "smoothly over time", bigger trees = more chops.
  Vintage Story falling-trees / VS-Lumber base-notch. Genre is UNANIMOUS base-cut→whole-tree; the per-block model
  is the outlier — Ben's direction IS the standard.
- **THE 4 POINTS resolved:** (1) WORK MODEL — place_chop_fell(base, fell_cells) = 1 job at base + fell_cells in a
  JobBoard side-table (the container/BedSlot pattern); base is always reachable (kills the FR10 floating-canopy
  residual). (2) VISUAL — v1 same-tick atomic removal + CHOP_DROP burst + a timber cue (DF teleport); ★v1.5 CHEAP
  polish (recommended) = TOP-DOWN staggered removal ~0.4s (reads as a fall, no physics, base-last = no-float-safe);
  v2 physics topple deferred (unneeded). (3) PROGRESS BAR — reuse Job.progress + Axe/woodcutting economy; threshold
  scales with Wood-count so total labor ≈ old per-block sum (economy CONSERVED — a granularity refactor, NOT a
  rebalance). (4) SAFETY reconciled EXPLICITLY — FR10 caps unchanged (set computed identically, safe by
  construction); no-float IMPROVED (atomic/top-down removal never leaves a partial floating tree — the FR10 §6
  Phase-2 no-float invariant delivered preventively for free); cave-in = Mine-only, Chop removes only Wood/Leaves
  never ground → no trigger, no floating ground.
- **BUILD PACKET (START-HERE):** place_chop_cells→place_chop_fell (bastion_jobs.rs:1466) + the 2 callers
  (in_game.rs:1037, lib.rs:1564) + the completion arm (~:4405/:4561) fells the stored set + the side-table +
  size-scaled threshold + optional top-down stagger. REFERENCE-ONLY (don't edit): tree_fell_set, detect_trees,
  caps, the overlay. Test hook: 1 job/tree, whole-set clears same-tick, CHOP_DROP conserved, no floating remnant,
  deterministic.
- **Play-Tester coordination:** code predicts block-by-block; branched the design for not-firing (detection bug,
  separate) / no-visual (client-sync, separate) if the repro surfaces those.
- **Route:** SONNET reviewer local_5f3f9b01 (routine — verify the place_chop_fell swap + the side-table +
  completion-fells-whole-set feasibility); escalate to Opus only if the no-float/stagger ordering wants the
  safety-tier eye. → architect to schedule (behind AUTON).

## 2026-07-12 — UPDATED CHOP-FELLING-REFINEMENT (Play-Tester confirmation + Ben's size-scaling hard requirement) · same doc [GENERAL DESIGNER]
Play-Tester (local_160f59e4) independently root-caused chop from code (no build) — MATCHES this design's diagnosis
exactly (same lines: job_wanted :278-280, tree_fell_set :902-929, place_chop_cells :1428-1468 = one job PER CELL,
scattered claim order not top-down). Their framing ("a new mechanic layered on FR10, not a bugfix — nothing's
broken") folded in as corroboration. Their 3 flagged design surfaces all CLOSED in the doc:
- #2 (in-flight branch/canopy claims if base finishes first) → MOOT BY CONSTRUCTION under the one-base-job design
  (no branch/canopy jobs exist at all — nothing to race).
- #1 (a trigger on base-completion that batch-removes the rest) → IS the completion handler already specced.
- #3 (where do logs land) → v1 answer made explicit: drop IN-PLACE per cell (not bulk-at-base) — cheap, no
  fallen-extent geometry, reads naturally under the v1.5 top-down stagger.
- **Ben (direct, this pass): "make sure larger trees get cut slower" — elevated from embedded detail to a ★HARD
  REQUIREMENT.** fell_threshold = CHOP_WORK_PER_BLOCK × Wood_count(fell_cells) — reuses tree_fell_set's OWN size
  metric (no new size concept). Leaves count toward the visual mass but NOT the labor bar (free-clear, no
  drop-driven reason to gate on them — avoids two same-trunk trees at different speeds by canopy density alone).
  Added an explicit Done-when: completion_time ratio between two trees must track their Wood_count ratio — an
  ASSERT, not an eyeball.
→ architect + Play-Tester notified (doc updated in place, same path).

## 2026-07-12 — REVISED TEST-INFRASTRUCTURE-AUDIT (case-study-driven: item M + PATH-FIDELITY) · same doc [GENERAL DESIGNER]
Routed by the SONNET REVIEWER (local_5f3f9b01) via the architect, Ben-directive "implement the testing framework."
Two live bugs as proof-of-need: (1) FLAT-TEST-ARENA gate-green but INERT in real play (scenario harness world-gen
path diverged from the live singleplayer launch path — registry B41); (2) Farm unselectable since it shipped —
ToolMode::ALL missing DesignationKind::Farm (9 of 10).
- **Confirmed NEITHER was catalogued** — re-read the audit fresh; taxonomy A-L is entirely RUNTIME (harness tick/
  client boot/sim run); nothing zero-boot/compile-time existed.
- **★ Added item M — STATIC/COMPILE-TIME EXHAUSTIVENESS ASSERTIONS, PRIORITY 0 (cheaper than G).** Root-caused
  the Farm bug precisely: `voxygen/src/bastion/tools.rs::ToolMode::ALL` is a HAND-WRITTEN literal `[ToolMode; 9]`
  — the length annotation matches its own incomplete contents, so Rust's exhaustiveness checker never engages
  (it would if ALL were built via a non-wildcard match over DesignationKind, or a strum-derived list). Mechanism
  specced: (1) prefer an exhaustive match with NO `_ =>` wildcard over a hand-written literal wherever possible
  (compiler-enforced, `cargo build`-time, zero runtime); (2) where a literal must stay, a parity-assert unit test
  (`ALL.len() == DesignationKind::COUNT` via strum/enum_iterator or a commented hand-count) — `cargo test`-cheap,
  no boot; (3) generalized fleet-wide — a grep-and-audit for every hand-mirrored enum list (WorkType/ZoneKind/
  Need/etc., the append-only-enum family). Prior art: Rust's own sum-type exhaustiveness checking, strum/
  enum_iterator, Clippy's match_wildcard lint — item K's (coverage) compile-time cousin.
- **★ Added ★ PATH-FIDELITY (a cross-cutting discipline, not a new letter) for the FLAT-TEST-ARENA case.** Tied
  to the EXISTING registry class B17 ("identity by construction, no parallel copy" — bastion_chop.rs's own doc
  comment) — B17 was a coding discipline; this makes it also a TESTABLE one: where true identity isn't achievable,
  add a DIFFERENTIAL assertion (harness-path output ≡ live-path output — a Metamorphic Relation in item L's sense).
  **★ Self-check against my OWN roadmap: flagged that S3 (the planned bot-client, Phase 2) is exposed to the
  EXACT same disease** — a second client-invocation path is precisely FLAT-TEST-ARENA's shape. Added a hardening
  requirement to S3's seam spec (§3 table): must call the REAL Client connection code (not reimplement, per the
  chat_cli.rs precedent) + a periodic differential check against the §0 computer-use tier. Folded as a Done-when
  for S3's future design pass, not discovered after the fact.
- **Roadmap:** new Phase 0.5 (item M) slotted BEFORE Phase 1 — cheaper than CI-wiring, no infra, not blocked by
  the B7/ARCH-003 heads-down the way Phase 1 is (a `cargo check`-only fix vs a fleet-attention framework). Flagged
  for the architect to consider unblocking M specifically even while Phase 1 stays held — architect's call, not
  mine. FLAT-TEST-ARENA (B41) filed as the regression corpus's (H) next concrete entry.
- Doc status banner updated: REVISION note added (2026-07-12, same day) — the Phase-1 HOLD still stands; this is
  new evidence-driven content, not an un-hold.
→ architect + Sonnet reviewer (routing origin) notified.
