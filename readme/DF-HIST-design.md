# Project Bastion — DF-HIST Design v0.1 (the Chronicle / Legends — the world's memory)

**One design pass for DF-HIST + DF-LOG** — the world's memory: a **live event feed** (the scrolling chronicle
of what happens now) building up to a **browsable Legends viewer** (the persistent history of figures, sites,
factions, and events). Companion to the main build report (Pillar §1a legibility; §3d control spectrum), the
DF gap ledger (§C DF-HIST, §J DF-LOG, §I "Legends mode"), `BASTION-SYSTEM-FRAMEWORKS.md` (§1 control spectrum),
the God-Powers catalog (§1.2 attribution axis — a hard cross-system seam), and `future-work-and-deferred-ideas.md`
§3t (ages / unreliable accounts — optional depth).

**Which wall:** primarily **LEGIBILITY** — and not one system's legibility but *the* legibility organ of the
whole game. **DF-HIST IS the answer to "how does the player read the living world."** Every other system in
the corpus ends its own design with "…surfaces in the chronicle" (DF-PRODUCTION §6, God-Powers §1.2,
future-work §3t, the nature/weather/dungeon-repopulation notes). This pass builds the thing all of them emit
into. It has a small **SIMULATION** tail (a persistent, importance-weighted history store, LOD-split like
everything) and a small **CONTENT** tail (UI/2D only — panel frames, event-type glyphs; **no 3D, no
animation**).

**Fit-check verdict: PASS — this is pillar infrastructure, not a feature.** Reinterpreted through §1a: the god
**influences, never commands** — which means the god's primary *verb is watching*. A god that cannot read its
world cannot influence it wisely. The chronicle is the god's-eye organ of perception: it is almost entirely a
**read surface**, sitting *outside* the control spectrum (you don't command the chronicle, you consult it).
There is exactly **one** thread back into the control spectrum — divine **attribution** (the god chooses how
loudly its acts are recorded; God-Powers §1.2). No drift risk: a history browser cannot become 4X/unit-micro
because it issues no orders. The one guardrail is the reverse — **it must DO something** (a decorative log is
cut): every chronicle entry is *clickable into the live world* (a figure → the unit inspector; a site → the
map), and the feed is the alert layer that tells the god *when to look*. Legibility that closes back into
attention and action, not a museum.

**Ledger/corpus entries this consolidates:** `df-feature-gap-ledger.md` §C **DF-HIST** ("rtsim already accrues
history; needs a Legends/Chronicle browser. $"), §J **DF-LOG** ("scrolling event log → the Chronicle. $"), and
§I "Legends mode / chronicle browser" (a pointer to DF-HIST). **DF-LOG and DF-HIST are one system** — the feed
is the *near-term slice* (live, glanceable), the browser is the *deep slice* (persistent, browsable); they read
the **same capture layer**. This pass unifies them (like DF-MECH unified trap/operable). It appends to the
corpus; it rewrites nothing. It **refines** the ledger cost (§8 — the *data* is substrate as the ledger says,
but the ledger under-counts the client/UI + capture-API half; see the split).

---

## 0. The one thing to get right first — the capture API is the product, not the browser

The temptation is to design "a Legends screen." That is the *last* deliverable, not the first, and it is the
cheap half (a view over data that already exists). The **load-bearing, build-once artifact of this whole pass
is a single seam**: a **Chronicle capture API** — one `record(event)` call that *every other system in the game
emits into*, exactly mirroring how rtsim's event bus already works (`OnDeath`, `OnTheft`, … → `Reports`). It is
the legibility counterpart to the world-verb action library and the trigger→link→effect engine: **one thin
API, every system's history for free.**

This matters for sequencing: the capture API and its `ChronicleEvent` schema are **load-bearing schema that
hardens into code many systems call** — the same class of artifact as the shared `Quality` enum (DF-PRODUCTION
S6 / DF-QUALITY). It must be **defined and locked early** (HIST-0), *before* the systems that emit into it
harden their event points, or every producer forks its own logging. The browser can come last; the API cannot.

**North star for the whole pass: build the sink first (the API), tap the sources second (the feed), open the
museum last (the browser). History is captured, not reconstructed.**

---

## 1. The reuse split — the de-risk table (SUBSTRATE vs BUILD, real symbols)

The headline reuse finding: **rtsim already ships both halves of a history system — the event medium AND the
world-memory data — but neither is player-facing, and the event medium is built to be *forgotten*, which is the
exact opposite of a chronicle.** So DF-HIST is not "build an event system"; it is "add a **persistent,
player-facing sink** alongside the existing ephemeral one, tap the existing bus into it, and build the two
viewers." Verified against the tree:

### SUBSTRATE — exists, needs wiring

| Piece | Real symbol / location | What it gives us |
|---|---|---|
| **The event medium (the pattern to mirror)** | `rtsim/src/data/report.rs` — `Report { kind: ReportKind, at_tod }`, `ReportKind::{Death{actor,killer}, Theft{thief,site,sprite}}`, `Reports { reports: DenseSlotMap<ReportId, Report> }` | rtsim **already has an event record with typed kinds, actors, a site, and a timestamp.** This is 80% of a `ChronicleEvent`'s shape — but it's **ephemeral by design** (see the gotcha below). The Chronicle is its persistent sibling. |
| **The event bus (the emit points)** | `rtsim/src/event.rs` — `OnDeath`, `OnTheft`, `OnHelped`, `OnHealthChange`, `OnMountVolume`, `OnTick`; `rtsim/src/rule/report.rs` `ReportEvents` binds `OnDeath`/`OnTheft` → `reports.create(...)` | The **capture wiring already exists** for deaths and thefts. HIST-1 taps this same bus into the Chronicle sink instead of (or alongside) `Reports`. New emitters bind the same way. |
| **The world-memory data (what the browser views)** | `rtsim/src/data/mod.rs` `Data { npcs, sites, factions, reports, architect, quests, … }`; `data/site.rs` `Site`, `data/faction.rs` `Faction`, `data/npc.rs` `Npc`, `data/architect.rs` `Architect { deaths: VecDeque<Death> }` | The **Legends browser is a *view* over data that already accrues.** Sites, factions, NPCs, and a running death log already persist in rtsim. The browser is mostly wiring a UI onto this. |
| **rtsim persistence** | `Data` `#[derive(Serialize, Deserialize)]` + `CURRENT_VERSION` (data/mod.rs); `from_reader`/`write_to` | A new `chronicle` field on `Data` **persists for free** across save/load and rides the existing version/purge machinery (B10). History as world-state. |
| **The client-facing feed substrate** | `common/src/comp/chat.rs` — `ChatType<G>`, `GenericChatMsg`/`ChatMsg`, `into_msg(Content)`; the existing HUD chat/notification panel (voxygen) | The **live feed's rendering + client-sync path already exists.** A chronicle stream can ride a `ChatType`-style channel to a chat-like HUD panel — no new networking primitive, reuse the message pump. |
| **Localized content / templating** | `chat.rs` `Content` (localization keys with args), used by `into_msg` | Event text ("{killer} slew {victim} at {site}") is authored as localization templates, not hardcoded strings — the DF "sentence generator" pattern, cheaply. |
| **Actor/Site addressing** | `common/src/rtsim.rs` — `Actor`, `SiteId`, `ReportId`, `NpcId`, `FactionId` | Chronicle entries reference figures/sites by the **same stable IDs** the inspector + map already resolve → an entry is *clickable into the live world* for free (the "must DO something" guardrail). |

### BUILD — genuinely net-new

| Piece | Why it's new | Folds into |
|---|---|---|
| **The `ChronicleEvent` schema + persistent store** | `Reports` is ephemeral (decays, capped, no player path). A chronicle needs a **persistent, importance-weighted, append log** with actor/site/time/**importance**/**scope**/**attribution**. | New `Chronicle` field on rtsim `Data` (rides B10 persistence) |
| **The `record()` capture API (the seam)** | Nothing today lets an arbitrary system say "a notable thing happened." This is the **one build-once artifact everything emits into.** | The **legibility engine (build-once)** — mirrors the rtsim event bus |
| **Event capture from non-rtsim systems** | The rtsim bus covers world-tier events; **bastion job/production/farm/god events live server-side** (`server/src/bastion_jobs.rs`) and have no path to rtsim's bus. The capture API must be callable from there too. | The capture API (a server-visible `record`) |
| **The live event feed (DF-LOG)** | Client sync of recent high-importance entries → a scrolling, filterable HUD panel. `Reports` never reach the client today (verified — no client path). | The B9 HUD + the chat message pump |
| **The Legends browser (the viewer)** | A full-screen, browsable UI over `Chronicle` + rtsim `Data` (figures / sites / factions / timeline), drill-in, cross-linked to inspector/map. | The B9 HUD / B-AG4 inspector |
| **Importance model + LOD retention** | Every accumulation needs a decay: low-importance entries prune; legendary entries persist forever; world-tier events record as coarse summaries. | The loaded↔simulated LOD law |
| **Attribution tagging** | God-acts recorded with an attribution level; the chronicle is where an *ambiguous* act later gets *attributed* (God-Powers §1.2). | God-Powers (B13) + the faith layer (DP2/DF-RELIGION) |

**The collapse:** the *data* half is substrate (rtsim accrues sites/factions/npcs/deaths and already has a
typed-event record and an event bus). The *net-new* is a **persistent player-facing sink + the one API that
feeds it + two viewers (feed, browser) + the LOD retention model.** No new simulation of any weight, no 3D, no
animation. This is a **legibility + light-persistence + UI** build — cheap per piece, but the *capture API is
load-bearing* and must be locked first.

---

## 2. The key insight — the Chronicle is `Reports`' persistent, player-facing twin

rtsim's `Report` doc-comment is explicit: reports are "the medium through which rtsim represents information
sharing **between NPCs**." They are **built to be forgotten** — `remember_for()` gives a murder 15 days, a
theft 1.5, then `cleanup()` deletes them, and a `TODO` even asks to *cap the global count*. That is correct for
**gossip** (an NPC's decaying knowledge) and exactly **wrong for a chronicle** (the world's permanent memory).

So the design is not "extend Reports." It is: **the same event, forked to two sinks with opposite retention
policies.**

- **`Reports` (exists):** ephemeral, per-NPC, decays fast → drives NPC *behavior* (sentiment, gossip, fear).
- **`Chronicle` (new):** persistent, colony/world-scoped, importance-weighted retention → drives *player
  legibility* (feed + browser).

One emit point (`OnDeath`), two records. This is the same "one event, many consumers" discipline as the rest of
the build-once corpus — and it means the Chronicle's **importance-retention model is the natural home for the
`Reports` `TODO` too** ("limit global number of reports" / "track reports by chunks"): both want the same
capped, importance-weighted store. Flag this convergence to whoever hardens rtsim reports — don't build two
capping schemes (§8).

---

## 3. Systems needed (with deps + which build-once engine each folds into)

### S1 — The Chronicle store + `record()` capture API (the load-bearing seam)
**What:** a `Chronicle` data structure — an append log of `ChronicleEvent { kind, actors, site, at_tod,
importance, scope, attribution }` — plus a single `record(event)` entry point every system calls. `kind` is a
canonical enum (`Death`, `Theft`, `Founding`, `WarDeclared`, `Harvest`, `Masterwork`, `Famine`, `DivineAct`,
…), extended as emitters land. Retention is **importance-banded** (S5).
**Where:** a new `chronicle` field on rtsim `Data` (`rtsim/src/data/chronicle.rs`), so it persists + versions
for free; `record` exposed to both the rtsim event rules and (server-side) `bastion_jobs`.
**Deps:** rtsim `Data` (exists); B10 persistence for the save/load guarantee. **Folds into:** this **IS** the
build-once **legibility engine** — the counterpart to the world-verb library. Lock the `ChronicleEvent` schema
here (load-bearing, like the `Quality` enum).

### S2 — Event capture wiring (tap the bus + the first emitters)
**What:** bind the Chronicle into the existing rtsim event bus (`OnDeath`, `OnTheft`, plus new `OnSiteFounded`/
`OnWarDeclared` as those land) — a sibling of the existing `ReportEvents` rule — and expose `record()` to
server-side bastion systems so production/farm/combat/god events (which don't touch rtsim's bus) can emit too.
**Where:** a new `ChronicleEvents` rule in `rtsim/src/rule/` (mirrors `rule/report.rs`); a `record` hook usable
from `server/src/bastion_jobs.rs` + future producers.
**Deps:** S1. **Folds into:** the rtsim event bus (extends it, doesn't replace it — same event, second sink).

### S3 — The live event feed (DF-LOG — the near-term slice)
**What:** client sync of recent, above-threshold `ChronicleEvent`s → a **scrolling, importance-filterable HUD
panel** (the DF announcements/combat-log surface). Rides the existing chat/notification message pump; entries
render from localization templates (`Content`) and are **click-through** (a figure → inspector, a site → map).
Critically-important events (famine, siege, a colonist death) also raise an **alert** (the "when to look"
signal).
**Where:** a `ChatType::Chronicle`-style channel (or a dedicated notification stream — open Q §8) + a voxygen
HUD panel (B9). Server pushes above-threshold entries to clients.
**Deps:** S1/S2; the B9 HUD; client sync. **Folds into:** the B9 HUD + the chat pump. **This is the v1 value
delivery** — a living feed of what the colony is doing, shippable long before the full browser.

### S4 — The Legends browser (the deep viewer)
**What:** a full-screen, browsable UI over `Chronicle` + rtsim `Data`: **Figures** (NPCs — birth, death, deeds,
kin via B-AG6), **Sites** (founding, population, events there), **Factions** (relations, wars), and a
**Timeline** of high-importance events. Every row cross-links (figure → inspector, site → map, event → the
figures/site it names). LOD: a summary row cheaply; full detail materializes on drill-in.
**Where:** a new voxygen full-screen UI (sibling to the map/inspector screens); reads the synced Chronicle +
an rtsim-data query surface.
**Deps:** S1/S2; the world-memory data (rtsim `Data`); **NPC name persistence** (npc.rs:447 `TODO` — a chronicle
of unnamed figures is illegible; §8). **Folds into:** the B9 HUD / B-AG4 inspector family.

### S5 — Importance model + LOD retention (the cheap-tier + the decay law)
**What:** each `ChronicleEvent` carries an **importance band** (`Routine` / `Notable` / `Legendary` — a small
canonical enum, purpose-enum discipline). Retention is per-band: `Routine` prunes fast (feed only, short
window), `Notable` persists a long window, `Legendary` persists forever. World-tier (unloaded/rtsim) events are
recorded **as coarse summaries** at `Notable`+; loaded colonies record fine `Routine` detail. This is the
loaded↔simulated law applied to *history*, and the mandatory **decay for the accumulation** (the log) + a **cap
per band** (the carrying capacity).
**Where:** the `importance` field (S1) + a prune pass on the Chronicle (a cheap periodic sweep, like
`Reports::cleanup`). **Deps:** S1. **Folds into:** the LOD law + the accumulation/decay discipline.

### S6 — Attribution of divine acts (the God-Powers §1.2 cross-system seam)
**What:** god-acts `record()` with an **attribution level** (`Attributed` / `Ambiguous` / `Hidden`, from
God-Powers §1.2). An `Attributed` act names the god in the chronicle immediately; an `Ambiguous` act is recorded
**as nature** ("a lucky vein", "a calm season") and the chronicle is **where it later gets attributed** — a
faith/omen event (DP2 / DF-RELIGION) can flip an ambiguous entry to reveal the divine hand (the rival-god
mechanic: "indistinguishable from nature until the chronicle attributes it"). This is the concrete data seam
God-Powers §1.2 describes.
**Where:** the `attribution` field on `ChronicleEvent` (S1) + a flip operation invoked by the faith layer.
**Deps:** S1; **God-Powers (B13)** as the emitter; **the faith layer (DP2 / DF-RELIGION)** for the flip payoff
(partly unbuilt — §8). **Folds into:** the God-Powers catalog + Divine Politics.

### S7 (optional depth) — Ages / epochs + unreliable accounts (future-work §3t)
**What:** (a) **named ages** derived from world state ("Age of the Wolf-God" — advances when a megabeast dies /
a faction falls; future-work §3t) as free mythic framing over the timeline; (b) **unreliable accounts** (Caves
of Qud §3t) — the chronicle need not be omniscient; different sites record the same war differently, lore items
carry *versions*. **Both explicitly deferred** — (b) depends on the belief/memory-drift layer (Agency Bible),
which is unbuilt. Named here so the debt is visible, not designed ahead of substrate (§8).
**Deps:** S1 + a mature world-memory + (for b) the belief layer. **Folds into:** the timeline (a) / the lore
layer §3e (b).

---

## 4. Assets needed (READY / NEEDS-tagged)

**This is a UI system — almost no content wall.** No 3D models, no sprites-in-world, no textures-on-terrain.
Asset needs are **2D UI**: panel frames, event-type glyphs, and figure/site/faction avatars-or-glyphs. Most are
"UI authored in code + the existing icon style," i.e. effectively READY or a small icon batch.

| Asset | Tag | Notes |
|---|---|---|
| Chronicle **feed panel** frame + scroll styling | **READY** | Rides the existing chat/HUD panel style; a reskin, not new art. |
| **Event-type glyphs** (death, theft, birth, founding, war, harvest, masterwork, famine, divine-act) | **NEEDS:DF-HIST-UI** → small icon batch | ~10–15 small monochrome glyphs keyed to `ChronicleEvent::kind`; the feed's legibility. One-per-kind, grows with the kind enum. |
| **Importance-band styling** (Routine / Notable / Legendary color/weight) | **READY** | Pure CSS/UI styling (color + weight), no art. |
| Legends browser **layout** (figures/sites/factions/timeline tabs) | **NEEDS:DF-HIST-UI** | UI composition; reuses inspector/map screen furniture. Art-light. |
| **Figure / site / faction glyphs** (browser + feed) | **NEEDS:DF-HIST-UI** → optional | Can ship v1 with generic role glyphs (reuse existing NPC/site icons); bespoke portraits are far-future polish. |
| **Attribution marker** (a "divine hand" glyph for attributed god-acts) | **NEEDS:DF-HIST-UI** → gated on S6 | One glyph; distinguishes attributed divine entries. Gated on God-Powers/faith. |

**Near-term (HIST-2 feed will consume): the event-type glyph batch** — written to `ASSET_REQUESTS.md` as a
small, real-demand icon set. Everything else is UI-in-code or far-future polish and stays on the wishlist.
**No T-posing risk — there are no verbs here** (§5).

---

## 5. Animations needed — **NONE** (explicitly)

**This system introduces no work verb, no creature, no body plan, and no character action.** It is a pure
legibility/UI + light-persistence system: capture (a function call), retention (a data sweep), feed + browser
(2D UI). There is **zero animation debt** — the no-T-posing rule is satisfied vacuously because nothing new is
performed in the world. (The *events* the chronicle records are produced by other systems that carry their own
animation line-items — e.g. the craft/farm animations in DF-PRODUCTION §5; DF-HIST only *reads* that they
happened.) Noted explicitly per the manual so the absence is a decision, not an oversight.

---

## 6. Legibility · Control-spectrum · LOD (the three pillars)

### Legibility — this system *is* the legibility answer, but it still answers for itself
DF-HIST is what every other design's "…surfaces in the chronicle" line resolves to. But a legibility system
must itself be legible, at two depths (DF's own split: the announcements bar vs Legends mode):
- **Glanceable (the feed, S3):** a scrolling HUD panel of recent notable events + alerts for the critical ones.
  This is the **"when to look" layer** — the god's peripheral vision. Importance-filterable so it never floods.
- **Deep (the browser, S4):** the browsable history — figures, sites, factions, timeline — the **"what
  happened / who is this" layer**. Reached on demand, not pushed.
- **The closing loop (the "must DO something" guardrail):** every entry is **click-through into the live world**
  (figure → B-AG4 inspector, site → map, event → its participants). The chronicle is not a museum; it is a
  **router for the god's attention** — read an alert, click the colonist, decide whether to intervene. That is
  the influence loop the pillar wants: perception → attention → optional action.

### Control-spectrum placement — deliberately *outside* the spectrum (with one thread in)
The chronicle issues no orders, so it sits **off** the Autonomous/Manage/Direct axis — it is the **perception
substrate that makes influence *informed*.** The one thread back in is the **god layer**: divine **attribution**
(S6 / God-Powers §1.2) — the god doesn't command the chronicle, but *chooses how loudly its own acts are
recorded* (a loud attributed miracle vs a deniable ambiguous blessing). A light optional **Manage-ish** touch:
the god may **pin / annotate** an entry (ties DF-NOTES) — bookmark a figure or a place to watch. Never required;
the feed + browser are complete read-only.

### LOD story (loaded↔simulated — the law applied to *history*)
This is the locked LOD law (**continuous-when-loaded / discrete-in-rtsim**) applied to the world's memory:
- **Loaded (a watched colony):** fine-grained events — *this* colonist crafted a masterwork, *this* plot was
  harvested, *this* raider fell. `Routine`+ importance, full detail.
- **Unloaded (rtsim world tier):** **coarse summaries only** — a war was declared, a site was founded or fell,
  a historical figure died. Recorded at `Notable`/`Legendary`, never per-NPC/per-tick. **Never push fine-grained
  per-entity history into rtsim** (gotcha #1) — the world tier remembers *epochs and figures*, not footsteps.
- **On inspect:** drilling into a summarized world event materializes what detail the persisted data supports
  (the figures/sites it names resolve via their IDs) — full-res only where the god looks.
- **The two laws:** the Chronicle is an **accumulation**, so it has a mandatory **decay** — `Routine` entries
  prune fast, `Notable` on a long window, `Legendary` forever (S5). And a **carrying capacity** — a **cap per
  importance band** so the log is bounded no matter how long the world runs (the same need the `Reports` `TODO`
  names). No unbounded history growth.

---

## 7. Sequenced sub-blocks, each with a concrete Done-when (the buildable output)

Dependency-ordered. **v1 slice = HIST-0..HIST-2 (the capture API + first emitters + the live feed) — this is
the DF-LOG near-term win and ships value alone.** HIST-3..HIST-4 build the browser + LOD. HIST-5 is the
cross-system attribution enrichment; HIST-6 optional depth. Each Done-when is invariant-first (persist/
conservation/bounded/no-panic) where sim, screenshot/eyeball where visual.

### HIST-0 — The Chronicle store + `record()` capture API + schema lock · [the seam]
**Depends:** rtsim `Data`; B10 persistence for the save/load half. Builds S1.
**Scope:** the `Chronicle` field on `Data`; the `ChronicleEvent` schema (kind/actors/site/at_tod/importance/
scope/attribution); the `record()` entry point; the importance enum. **Lock the schema** (load-bearing, like
the Quality enum) — coordinate the enum with any parallel producer designs.
**Done-when (`--chronicle-scenario`):** a test `record()` call appends a `ChronicleEvent` that (a) **survives
save/load byte-for-byte** across the rtsim `Data` round-trip (persistence conservation — no entry lost or
duplicated across the B10 boundary), and (b) the store is **bounded** — a soak that records N ≫ cap entries
holds total count ≤ the per-band caps (no unbounded growth), with `Legendary` entries never pruned. No panic,
bounded record-time.

### HIST-1 — Tap the event bus + the first emitters · [DF-HIST capture]
**Depends:** HIST-0. Builds S2.
**Scope:** a `ChronicleEvents` rule binding `OnDeath`/`OnTheft` (sibling to `ReportEvents`) → `record()`; the
first server-side emitter from `bastion_jobs` (e.g. a colonist death / a founding). Same event → both `Reports`
(ephemeral) and `Chronicle` (persistent).
**Done-when (`--chronicle-capture-scenario`):** a death and a theft in a harness scenario each produce
**exactly one** `Chronicle` entry with the correct actors, site, and timestamp (no dupes, no drops — one event,
one record), *and* the existing `Reports` behavior is unchanged (the ephemeral sink still fires — no regression).
Conservation: entry count == event count.

### HIST-2 — The live event feed (DF-LOG) · [v1 value — the near-term slice]
**Depends:** HIST-0/1; the B9 HUD; client sync. Builds S3.
**Scope:** client sync of above-threshold entries → a scrolling, importance-filterable HUD panel rendering from
localization templates; click-through to inspector/map; alerts for critical entries.
**Done-when:** (visual/eyeball) with a running scenario, colonist deaths / thefts / a founding appear in the
feed panel **in chronological order, correctly templated** ("{killer} slew {victim} at {site}"), within one
tick of the event; the importance filter hides `Routine` when set to `Notable`+; clicking a figure entry opens
its inspector. (sim, `--feed-scenario`) the feed's synced buffer is **bounded** (length-capped, oldest evicted)
and drops nothing above threshold.

### HIST-3 — The Legends browser (figures / sites / timeline) · [DF-HIST viewer]
**Depends:** HIST-0..2; rtsim `Data`; **NPC name persistence** (§8 dep). Builds S4.
**Scope:** a full-screen browser over `Chronicle` + `Data`: a Figures list (NPC → birth/death/deeds/kin), a
Sites list (site → founding/population/events), and a Timeline of `Notable`+ events; cross-linked to inspector/
map. (Factions tab folds in as faction data richens.)
**Done-when:** (visual/eyeball) the browser lists real historical figures and sites drawn from rtsim `Data`;
opening a figure shows its recorded life-events (at minimum birth + death + any recorded deeds) and links to its
inspector; opening a site shows its founding + events there; the timeline shows `Notable`+ entries in order.
Every row resolves its IDs (no dangling figure/site references — a named actor always links to a real record or
is gracefully marked "lost to history").

### HIST-4 — LOD retention + rtsim world-tier history · [LOD law]
**Depends:** HIST-0..3. Builds S5 + the world-tier emitters.
**Scope:** importance-banded retention/prune; world-tier (unloaded) events recorded as coarse summaries
(war declared, site founded/fell, figure died) at `Notable`/`Legendary`; loaded colonies keep `Routine` detail.
**Done-when (`--chronicle-lod-scenario`):** over a long rtsim soak, (a) unloaded-region events appear as coarse
summaries (no per-NPC/per-tick spam entering the store — assert entry rate stays bounded per sim-time), (b)
`Routine` entries are pruned on their window while `Legendary` entries survive the whole soak, and (c) total
store size stays ≤ the per-band caps across the soak (bounded history — the accumulation/decay law holds). No
panic, bounded prune-time.

### HIST-5 — Attribution of divine acts · [God-Powers §1.2 cross-system seam · enrichment]
**Depends:** HIST-0/1; **God-Powers (B13)** as emitter; **faith layer (DP2/DF-RELIGION)** for the flip payoff.
Builds S6.
**Scope:** the `attribution` field wired end-to-end; god-acts record with their level; an `Ambiguous` entry
renders as nature; a faith/omen event flips it to reveal the god.
**Done-when (`--attribution-scenario`):** an `Attributed` divine act names the god in the chronicle immediately;
an `Ambiguous` act records as a natural-phrasing entry (no divine attribution shown) and, after a faith/omen
flip operation, the **same entry** now reveals the divine hand (a flip in place, not a duplicate). Bounded, no
dupe.

### HIST-6 — Ages/epochs + unreliable accounts · [optional depth — future-work §3t]
**Depends:** HIST-0..4 + a mature world-memory; (unreliable accounts) the belief/memory-drift layer (unbuilt).
**Scope:** named ages over the timeline from world state; per-source account variance. **Deferred** — designed
as a named seam only; do not build ahead of the belief substrate (§8/§9).
**Done-when (deferred):** an age name derives from world state and labels the timeline; (later) two sites record
the same event with differing detail. *Not scheduled for v1.*

---

## 8. Dependencies · open questions · tuning-data · corpus contradictions

### Dependencies (build-order truth)
- **rtsim event bus + `Data` — SUBSTRATE, ready now.** The capture wiring and the world-memory data exist; this
  is why HIST-0/1 are buildable early (the seam is cheap and load-bearing — good to lock now).
- **B10 persistence — for HIST-0's save/load Done-when.** The Chronicle rides rtsim `Data` serialization; the
  persistence guarantee is B10's.
- **The B9 HUD + client sync — for HIST-2 (feed) and HIST-3 (browser).** The client half needs the HUD frame +
  a sync path (the chat pump substrate covers most of it).
- **NPC name persistence (`rtsim/src/data/npc.rs:447` `TODO: actually persist names`) — for HIST-3.** A
  legible Legends browser needs *stable named figures*; today names are regenerated from deterministic RNG
  (`get_name`), not persisted. **Flag to the architect:** the chronicle makes this `TODO` load-bearing (a
  history of figures whose names can drift is broken). Small but real dep for the browser (not the feed).
- **Emitters across the corpus — DF-HIST is only as rich as what records into it.** DF-PRODUCTION PROD-4
  already declares it emits economy events into the chronicle; God-Powers §1.2 declares divine acts; the
  nature/weather/dungeon-repopulation notes (future-work §3t) all declare chronicle entries. **The capture API
  (HIST-0) must land before those systems harden their event points** or each forks its own log — this is the
  sequencing argument for building the seam early even though the *browser* is later.
- **Cross-seam — the shared canonical-enum discipline:** the `ChronicleEvent::kind` and `importance` enums are
  load-bearing schema, same class as the `Quality` enum (DF-PRODUCTION S6). Lock them once, canonically.

### Open questions (flagged for Ben — genuine design calls)
1. **Where does the Chronicle store live** — a new field on rtsim `Data` (persists + versions for free, but
   couples the chronicle to the rtsim tick/save), or a parallel bastion-side store? *Recommendation:* **on rtsim
   `Data`** — history *is* world-state, and this inherits persistence, versioning, and the purge machinery for
   free. Server-side bastion events call `record()` into it.
2. **Importance model — discrete bands or a numeric score?** *Recommendation:* **discrete bands** (`Routine`/
   `Notable`/`Legendary`) — they map cleanly to retention windows + feed filters and follow the canonical-enum
   discipline (a numeric score invites per-system fudging and un-lockable schema).
3. **Feed transport — reuse a `ChatType::Chronicle` channel, or a dedicated Notification stream?**
   *Recommendation:* a **dedicated chronicle stream** (chat is player/NPC *speech*; mixing history into it
   muddies both filtering and localization) — but **rendered in a chat-like panel** to reuse the HUD furniture.
4. **Unreliable accounts (§3t) — in or out for v1?** *Recommendation:* **OUT** — the single-source omniscient
   chronicle ships first; unreliability needs the belief/memory-drift layer (Agency Bible), which is unbuilt.
   HIST-6, deferred.
5. **Legends browser scope at v1** — minimal (figures + sites + timeline over existing `Data`) or full DF-style
   multi-tab (regions / artifacts / detailed wars)? *Recommendation:* **minimal v1** — the browser's depth is
   gated on how much world-memory actually accrues; over-building tabs for data that doesn't exist yet is the
   "stale design ahead of substrate" trap. Expand as factions/artifacts/wars richen.

### Tuning-data (RON/config, not code)
Per-`kind`→`importance` mapping (which events are Routine/Notable/Legendary); retention window per band; feed
length cap + default importance filter; per-band store cap (carrying capacity); world-tier summary thresholds
(what rtsim events warrant a chronicle entry); age-naming templates (S7); event-text localization templates.
**Balance + taxonomy lives in RON; the systems read it.**

### Corpus contradictions / refinements found (flagged, not silently fixed)
- **Ledger cost refinement (not a contradiction):** `df-feature-gap-ledger.md` tags DF-HIST/DF-LOG "$" and
  SUBSTRATE. The **data** half is indeed substrate (verified — rtsim accrues sites/factions/npcs/deaths + a
  typed-event record + an event bus). But the ledger **under-counts the client/UI + capture-API half**: a
  persistent player-facing sink, the `record()` seam, client sync, the feed panel, and the Legends browser are
  real net-new UI/sync work. Refine: **DF-HIST is "$ data-wiring + $ UI build" — the memory is substrate, the
  *surfacing* is build.** Flagged for the architect; ledger not edited here.
- **DF-LOG folds into DF-HIST (consolidation, not conflict):** the ledger lists DF-LOG (§J) and DF-HIST (§C) as
  separate IDs. They are **one system, two slices** (feed = near, browser = deep, same capture layer). This
  pass designs them together; recommend the architect mark DF-LOG as the HIST-2 slice of DF-HIST in the queue.
- **`Reports` retention `TODO` convergence (flag, not fix):** `report.rs` carries `TODO: Limit global number of
  reports` and `report.rs` (rule) `TODO: ... a dedicated data structure that tracks reports by chunks`. The
  Chronicle's importance-banded capped store is the natural home for that capping discipline — **flag to whoever
  hardens rtsim reports** so two capping schemes aren't built. Not a contradiction; a convergence to coordinate.
- **NPC name persistence `TODO` (dep, flagged above):** `npc.rs:447` — the chronicle makes it load-bearing for
  HIST-3. Surfaced, not fixed.

---

## 9. Honest limits (grading my own design)
- **The chronicle is only as rich as its emitters — and at the current frontier (B5.6b→B6) almost nothing emits
  yet.** v1's chronicle is *sparse* (deaths, thefts, foundings). It becomes the legible organ the corpus
  promises only as production/religion/combat/nature land and emit into it. This is the **correct** thing to
  build now anyway — the capture API is the load-bearing seam that must exist *before* those systems harden, and
  the feed is cheap value — but do not oversell a rich Legends browser before the world produces history.
- **HIST-3 (browser) is gated on world-memory depth that is itself thin.** Reports decay; names aren't
  persisted; faction/war data is shallow. The browser is correct to design (the view is cheap) but its *depth*
  will lag the world sim — hence the "minimal v1" recommendation (open Q 5) and the name-persistence dep.
- **HIST-5 (attribution) sits partly on unbuilt faith.** The attribution *tag* is trivial and shippable with
  God-Powers; the *flip payoff* ("ambiguous → attributed via an omen") needs DP2/DF-RELIGION, which is
  partly unbuilt. The seam is designed and cheap; the payoff is gated.
- **HIST-6 (unreliable accounts) is deferred on principle** — it needs the belief/memory-drift layer. Named as a
  seam, not designed ahead of its substrate (the exact anti-pattern this workflow guards against).
- **The client-sync specifics are under-specified.** The transport *approach* is concrete (ride the chat pump);
  the exact sync cadence/filtering for the feed vs the browser query surface needs its own pass at build time —
  flagged, not hand-waved as done.

*End of DF-HIST design. rtsim already remembers the world — it accrues sites, factions, figures, and a typed
event record on a live event bus — but it remembers for the NPCs, ephemerally, and never tells the player. This
pass finds the small, specific net-new work: a persistent player-facing sink, the one `record()` API every
system emits into (the load-bearing seam, lock it first), and two viewers (the live feed = the near-term DF-LOG
win, the Legends browser = the deep slice) — sequenced HIST-0..HIST-6 with testable Done-whens, and the LOD law
applied to the world's memory.*

---

## GAP-AUDIT ADDENDUM — the full ChronicleEvent kind-list (2026-07-10, architect-approved lock)

*(HIST-0's rule: lock the `ChronicleEvent` schema BEFORE emitters harden. The original spec named ~10 kinds
(`Death, Theft, Founding, WarDeclared, Harvest, Masterwork, Famine, Siege, DivineAct, Birth`). The corpus now
emits ~20+ more — enumerated here so `record()` callers + the DF-HIST glyph batch don't churn. Add as the enum
grows; this is the intended coverage list, grouped by source.)*

- **Production / economy** (DF-PRODUCTION, DF-TRADE, BUILD-FRAMEWORK): `GreatWorkCompleted`, `CaravanArrived`,
  `CaravanLost`, `TradeDealStruck`.
- **Faith / omens** (DF-RELIGION, DF-OMEN, DF-FESTIVAL): `TempleBuilt`, `ProphetArose`, `PrayerAnswered`,
  `TempleStoodEmpty`, `FestivalHeld`, `OmenSeen`, `ProphecyFulfilled`, `ProphecyFalse`.
- **The remembering world — the 4 faces** (REPUTATION, GOD-EPITHET, SACRED-SITES, COLLECTIVE-RENOWN): `ReputationRose`,
  `ReputationFell`, `EpithetShifted`, `SacredSiteMade`, `SiteDesecrated`, `RenownEarned`, `ColonyBynamed`.
- **Legendary figures — the triad** (DF-VILLAIN, DIVINE-CHAMPION, DF-BEAST): `NemesisRose`, `NemesisFell`,
  `ChampionAnointed`, `ChampionFell`, `BeastSlain`, `BeastNamed`.
- **The dead** (DF-ANCESTORS): `HeroMartyred`, `AncestorVenerated`, `GhostRestless`, `SoulLaidToRest`.
- **The divine hand** (DF-CURSE, SACRED-SITES, GOD-POWERS): `CurseLaid`, `CurseLifted`, `GeasBound`, `GeasBroken`,
  `Consecration`, `Miracle`.
- **Knowledge** (DF-KNOWLEDGE): `TechDiscovered`, `KnowledgeLost`, `KnowledgeTaught`.
- **The colony's life** (DF-RECLAIM, milestones, hazards, DF-CAVERN): `ColonyFell`, `MilestoneFirst` (first-birth/
  death/winter-survived/masterwork), `CaveIn`, `Breach`, `Flood`, `Migration`.

**Schema stays `ChronicleEvent { kind, actors, site, at_tod, importance, scope, attribution }`** — the kind-list
above is the enum's intended coverage; the `importance`/`scope`/`attribution` fields (already specced) handle the
weighting/filtering. **Lock the enum with this list so the glyph batch + `record()` emit-points land once.** (The
glyph batch: the core 6 shipped; the rest attach to their kind as emitters land — do NOT fork per-system glyph sets.)

## ADDENDUM (2026-07-10) — ChronicleEvent schema requirements from GOD-DOMAIN/Scry + reviewer code-survey
The GOD-DOMAIN flagship (WATCHER's **Scry-the-Memory-of-a-Place**) + the domain-vector both DERIVE from this
chronicle, and the reviewer surveyed the live rtsim event model (`rtsim/src/data/report.rs`). Three requirements this
places on the ChronicleEvent schema (D7 vocabulary-lock — decide these when locking the enum):
1. **A SPATIAL KEY on events — RESOLVED (FR9): a bucketed `Vec3<i32>`.** Precedent exists: `ReportKind::Theft{…,
   site: Option<SiteId>, …}` place-keys an event, but `ReportKind::Death{actor, killer}` is placeless. **Standardize
   on a bucketed `Vec3<i32>`** (a tile-cell — a chunk-column or 4-block grid) as the **canonical spatial field**, NOT
   `SiteId` or room-id: **every deed has a `pos`** (universal — `SiteId` is null outside sites, which is the exact
   `Death` inconsistency; room-id needs DF-ROOMS), it's fine enough to scry-a-spot, and **site/room are ROLLUPS
   derived from the pos** (always coarsen pos→site, never refine SiteId→spot). This one field resolves the
   granularity+uniformity gaps: every scry-able event carries the bucketed pos. (Kinds may still be *actor-scoped* —
   a life-arc — but carry the pos where the deed happened when there is one.)
2. **A PERMANENT tier, distinct from rtsim's FADING Reports.** rtsim `Report`s DECAY (`Report::remember_for` — murder
   15d, theft 1.5d, then forgotten) — that's the *recent-events feed*. The **chronicle is the PERMANENT memory** ("the
   ground remembers forever"): a fallen colony's ruin, a legendary death, the god's domain-drift must persist for the
   remembering-world. So the ChronicleEvent sink is a **separate persistent store**, not an extension of the fading
   rtsim Reports. (rtsim Reports = the last few days; the chronicle = the ages.)
3. **A DOMAIN SPHERE-WEIGHT on each event** (for GOD-DOMAIN): each event tags which sphere(s) it feeds (raise-dead →
   +dead, forge-masterwork → +forge…) so the god's domain-vector derives from the same stream as the epithet. A small
   optional field on the schema.
Scale note: rtsim ships ~2 ReportKinds; the chronicle corpus is ~35 (the gap-audit) — the sink is a big net-new
enum regardless, so fold these three fields in at the D7 lock rather than retrofitting. (Source: reviewer §FR8-claim6.)
