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

=== SESSION PAUSE (this designer, 2026-07-09) — clean stop, resume point set ===
This session DESIGNED (5): DF-PRODUCTION, DF-DIG-VERBS, DF-QUALITY(+DF-ARTIFACT), DF-ZONES, DF-BURROW. Parallel
session(s) DESIGNED: DF-HIST(+DF-LOG), DF-RELIGION. Two schema LOCKs landed (Quality → frameworks §2b; ZoneKind
→ §2 recommended). Stopping cleanly on context budget, NOT on blocker — ready backlog is NOT exhausted.
NEXT UNCLAIMED near-frontier Tier-1/2 (verify no live CLAIM first): DF-ORDERS (PROD-1 seeds it), DF-TAVERN
(unblocks DF-ZONES Meeting wire + DF-RELIGION gather loop), DF-WOUND, DF-MECH/TRAP/OPERABLE (trigger→link→
effect cluster), DF-TRADE, DF-CAVERN+DF-GEOLOGY. Architect: re-fire or run parallels from here.
