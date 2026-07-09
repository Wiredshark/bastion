# Project Bastion — DF-RELIGION Design v0.1 (temples · worship · prophets · the faith seam)

**The colony's religious life — and the one topic where the player *is* the subject.** Temples/shrines as
places colonists worship, worship as an autonomous need, prophets/priests who tend the flock, and the seam
where colony-tier faith hands off to the LATE Divine-Politics faith layer (DP2). Companion to the main build
report (§1a, B7 Needs, B13 divine layer), the **DF gap ledger** (§H "Temples/religion/worship/prophets/
monastic orders" = DF-RELIGION; §H "Festivals" = DF-FESTIVAL, deferred), the **God-Powers catalog** (the
divine verbs that act *on* faith — sanctify, answer-prayer, bless), the **Divine-Politics bible** (DP2 faith
system — the world-scale layer this feeds), and `BASTION-SYSTEM-FRAMEWORKS.md` (§1 control spectrum, §2 the
canonical `faith`-purpose zone).

**Which wall:** primarily **SIMULATION** (autonomous worship behavior) with a **DESIGN-FIT** hazard at the
front (religion in a god-game is a trap that pulls toward player-commanded ritual) and a **LEGIBILITY** tail
(how the god reads the colony's devotion). The **CONTENT wall is half-fallen** — Veloren ships a temple plot,
the entire congregation-gesture vocabulary (dance/cheer/sit), and the blessing substrate — but the colony
needs a *buildable* worship structure and the faith-asset batch (altar/idol/pews), so content is a real but
bounded slice.

**Fit-check verdict: PASS (strongest thematic fit in the ledger — with one hard guardrail).** DF-RELIGION is
the single item where the pillar *inverts in our favor*: in DF the player is an atheist overseer watching
dwarves worship abstract deities; **in Bastion the player literally IS the god being worshipped** (divine-
politics-bible §4 — the colony is "your home flock"). That is not a feature to bolt on; it is the game's
premise made concrete at colony scale. **The guardrail:** religion must stay *autonomous* like tavern
socializing — colonists worship because they *need* to, prophets *arise*, congregations *gather* on their own.
The player must NEVER get a "make colonist pray" command; the RTS temptation here is a scheduled-ritual micro
screen, and it is AVOID. The player's entire involvement is the **divine layer** (be worshipped → favor;
answer prayers / sanctify ground → *attributed* acts that deepen faith). Held to that, religion is the most
Bastion thing in the whole ledger.

**Ledger/corpus entries this consolidates:** `df-feature-gap-ledger.md` §H **DF-RELIGION** (temples/worship/
prophets/monastic orders). It **splits** that one `$$` line into a near-frontier colony tier (`$`, mostly
wire) and a LATE faith-politics tier (Divine-Politics-owned — see §9). It touches, and defers to, three
already-designed neighbours it must not fork: the **God-Powers catalog** (owns the divine verbs), the
**Divine-Politics bible DP2** (owns world-scale faith state), and ledger **DF-FOCUS** (owns the "pray"
personal-need — worship is its first instance; §8 seam). It appends to the corpus; it rewrites nothing.

---

## 0. The one thing to get right first — the god does not schedule the sermon

Every colony sim that touches religion makes worship a *thing the player arranges*: in DF you build a temple,
assign it to a deity, appoint a priest from a menu, and set festival dates. That is **religion as management
UI**. It is also, for us, a category error: we are not the overseer arranging worship of a distant god — **we
are the god**. The correct model is the one Veloren already ships for taverns: colonists have a **need**
(there, recreation; here, worship/faith), and they **autonomously travel to the relevant plot, perform social
gestures, satisfy the need, and leave** (`rtsim/src/rule/npc_ai/mod.rs` `go_to_tavern` / `socialize`). Worship
is that loop retargeted at a temple. The player's involvement is **influence at the divine tier**, never a
worship command:

- **Autonomous (default + soul):** colonists worship from need; a temple the colony built fills with a
  congregation; a devout colonist becomes a lay-prophet on their own. *This is a complete religious life with
  zero player input* (Tier-1b soak law) — you can watch a faith flourish and never touch it.
- **Manage (optional):** you *zone* a temple (a Build designation, like any structure) and MAY appoint/bless a
  prophet. Policy and provision, never a worship order.
- **Direct — deliberately EMPTY.** There is no "command colonist to pray." Its absence is a design feature: the
  moment worship is a click, it stops being faith and becomes a chore-assignment. Guard this hole.
- **God layer (the real player surface — B13 / God-Powers catalog):** you are **worshipped** (passive →
  favor), you **answer prayers** and **sanctify ground** (attributed acts → deepened faith). This is where a
  god's relationship to religion lives — indirect, ambient, and *earned by the colony's devotion*, exactly per
  the god-powers three laws (act on conditions, cost favor, ripple through the sim).

**North star: provide and be worshipped; never schedule the rite. Worship is a need the colony meets on its
own, and faith is a force you tend from above — never a task you assign from within.**

---

## 1. The reuse split — the de-risk table (SUBSTRATE vs BUILD, real symbols)

This is the heart of the pass. **The colony-tier religion mountain is ~70% already in the repo** — as the
tavern behaviour loop, the gesture vocabulary, the needs schema, the buff (blessing) system, and a temple
plot. What Veloren ships (verified against the tree, not guessed):

### SUBSTRATE — exists, needs wiring

| Piece | Real symbol / location | What it gives us |
|---|---|---|
| **The gather-at-a-plot behaviour** (THE template) | `rtsim/src/rule/npc_ai/mod.rs` — `go_to_tavern(site, plot)`, `socialize()`, the arena-seat block; `ctx.controller.do_dance/do_cheer/do_sit` | The complete "colonist travels to a social plot, picks a spot (chair/stage), performs gestures, stays a while, leaves" loop. **Worship is this, retargeted from a Tavern plot to a Temple plot and from `Detail::Bar/Stage` to `Detail::Altar`.** The single biggest de-risk. |
| **The gesture vocabulary** | `common/src/rtsim.rs` — `NpcActivity::{Dance, Cheer, Sit, Talk}`; `CharacterState::Dance`/`Sit`/`Talk`; `voxygen/anim/src/character/{dance,sit,talk}.rs` | Non-T-pose worship *today*: a congregation is colonists who `Sit`/`Cheer` facing an altar (literally the tavern arena-seat code). A dedicated pray/kneel is enrichment, not a blocker. |
| **The need clock** | `common/src/comp/bastion.rs` — `Needs { hunger, rest, recreation }`, `Mood(f32)`; "decay + satisfaction land in B7" | The demand signal. Worship rides this exact struct (a `worship`/`faith` field, or a typed slot of `recreation`). Decay+satisfy is **B7's job** — worship is a B7 need, not a new needs engine. |
| **Temple structure** | `world/src/site/plot/desert_city_temple.rs` (646 lines), `PlotKind::DesertCityTemple`; the file even has a "sun god" sculpture placeholder | Proof a temple *renders* and slots into the plot system. It is worldgen- and desert-specific, so the **colony-buildable** temple is new — but the plot-authoring pattern (rooms + details, à la `tavern.rs`) is the template. |
| **Blessing / curse substrate** | `common/src/comp/buff.rs` — `BuffKind::{Agility, EnergyRegen, ProtectingWard, Frenzied, Hastened, Fortitude, …}` | Every SUCCOR/blessing god-power already rides these (god-powers catalog §2.3 🟢). A "blessed congregant" or "sanctified ground" effect is a `Buff`, not a new system. |
| **Faith-toward-a-target substrate** | `rtsim/src/data/sentiment.rs` — `Sentiments{ map: HashMap<Target, Sentiment> }`, `Target::{Npc,Faction,Character}`, decaying with `POSITIVE/NEGATIVE` half-lives | Devotion is a decaying, targeted sentiment. The pattern (a bounded, decaying value *toward* a target) is exactly what colony devotion + (later, DP2) faction faith need. Gods aren't a `Target` yet — that's the DP2 extension. |
| **A religious-role seed** | `common/src/rtsim.rs` — `Profession::Cultist` (already exists), alongside `Chef/Merchant/Guard/Farmer/…` | The profession→behaviour mapping (a Chef seeks a tavern bar; a Cultist could seek a shrine). A `Priest`/`Prophet` profession is a new enum arm on a proven pattern, not a new role system. |
| **The canonical zone kind** | `BASTION-SYSTEM-FRAMEWORKS.md` §2 — the 8-kind enum: `religious→faith` | A temple is a **`faith`-purpose zone**. The taxonomy *already predicts this exact system* — the zone kind is reserved. No new zone taxonomy. |
| **The job/need-job board** | `server/src/bastion_jobs.rs` — claim→travel→arrive→work loop; `NpcActivity::Goto` | If worship-attendance is modelled as a low-priority need-job (DF-FOCUS style), the board already runs it. (Or it rides pure rtsim-AI like the tavern does — see S2 open question.) |
| **The divine verbs (already catalogued)** | `readme/GOD-POWERS-CATALOG.md` §2.1 "Sanctify ground / found shrine", §2.3 "Answer a prayer", "Bless a colonist"; §2.6 passive "Favor accrual" / "Dominion ambience" | The *god-side* of religion is already designed as god-powers. DF-RELIGION does not re-invent them; it builds the *colony-side substrate they act on* and wires the seam. |

### BUILD — genuinely net-new (colony tier)

| Piece | Why it's new | Folds into |
|---|---|---|
| **Colony-buildable temple/shrine** | The only temple is a worldgen desert plot. The colony needs a placeable worship structure (a Build designation → colonists construct it) with a reachable **worship point / altar**. | The **B4 Build** path + the **zone↔asset taxonomy** (`faith` zone) |
| **The worship-need field + decay** | `Needs` has no worship/faith slot; nothing decays it. | **B7 Needs** (hard dep — worship is a B7 need) |
| **The attend-worship behaviour** | The tavern loop targets `PlotKind::Tavern`; nothing targets a temple or performs a worship gesture. New: retarget the loop + a worship-gesture selection. | rtsim `npc_ai` (extend the tavern pattern) / optionally the need-job board |
| **The prophet/priest role** | `Profession` has no priest; no "tend shrine / lead congregation" behaviour. | `Profession` enum + rtsim role behaviour |
| **Colony devotion aggregate** | No faith/devotion value exists on a colony. A bounded, decaying "how well-worshipped are you here" scalar that feeds favor and (later) DP2. | A sentiment-shaped accumulator (the DP2 seam) |
| **Faith legibility** | No devotion readout, temple inspector, or Chronicle religious events. | B9 HUD + B-AG4 inspector + DF-LOG |
| **Worship/pray animation** | `NpcActivity` has no worship gesture (Sit/Cheer are stand-ins). | §3u animation debt (`anim::pray`) |
| **rtsim aggregate devotion** | Unloaded colonies need a devotion that drifts to equilibrium, never per-colonist worship sim. | The loaded↔simulated LOD law |

### DEFER — LATE, Divine-Politics-owned (design the seam only, do NOT build now)

| Piece | Owner | Why late |
|---|---|---|
| **Faction faith state** (which deity, devotion level) | **Divine-Politics DP2** | World-scale; needs mature factions. This design *feeds* it (colony devotion is DP2's first data source) but must not build it. |
| **Conversion / rival gods / holy war** | **Divine-Politics DP3–DP4** | The whole world-faith contest. Tier-3 epic; premature. |
| **Festivals / ceremonies / holy days** | ledger **DF-FESTIVAL** (deferred list) | Scheduled congregation events; nice, but Tier-3 and best built on the worship loop once it exists. Design the hook (§7 REL-2 note), not the system. |
| **Monastic orders / religious guilds** | DF-RELIGION-LATE (ties DF-GUILD) | A profession-guild with religious demands; sits on DF-GUILD (deferred). |

**The collapse:** the colony tier of DF-RELIGION is **mostly wiring** — retarget the tavern behaviour at a
temple, add a worship field to the B7 needs it already plans, add a `Priest` profession arm, and stamp a
devotion accumulator. The *content* is a bounded faith-asset batch (temple + altar/idol/pews). The genuinely
scary parts (conversion, rival gods, holy war, festivals) are **not DF-RELIGION** — they are the Divine-
Politics build, and this pass's job is to design the **clean seam** that hands colony devotion up to them, not
to build them. Same "reuse-first collapses a scary topic" result as B3/B6/DF-PRODUCTION.

---

## 2. How the tavern loop already IS the worship loop (the key insight)

DF-RELIGION's headline — "colonists worship at temples, congregations gather, prophets lead" — reads as a
system to build. Most of it is an **existing behaviour retargeted**. The tavern AI (`npc_ai/mod.rs`) already
does, for recreation, everything worship needs for faith:

- **Travel to the plot:** `go_to_tavern(site, tavern_plot)` walks the NPC to the plot's door, picks a spot
  inside. → `go_to_temple(site, temple_plot)`: identical, targeting `PlotKind::Temple`.
- **Pick a spot + perform gestures:** the tavern picks a chair/stage and loops `do_dance/do_cheer/do_sit`. →
  worship picks a pew/altar-facing spot and loops a worship gesture (v1: `do_sit`+`do_cheer` facing the
  altar — *literally the arena-seat code*, `npc_ai/mod.rs` ~L887 "Walk to an arena seat, cheer, sit and
  dance").
- **A congregation for free:** the arena block already makes *many* NPCs gather, seat, and emote toward a focal
  point. A congregation is that crowd, facing an altar instead of a stage.
- **Stay a while, then leave:** `stop_if(timeout(wait_time))` — same bounded visit.
- **Role-seeking:** a `Chef` NPC seeks a tavern bar (`npc_ai/mod.rs` ~L1018). A `Priest` NPC seeks a temple
  altar by the identical `profession → plot-detail` match.

**So we do not design "worship behaviour." We retarget an existing, shipping behaviour.** The one net-new
sim is the **worship need's decay** (a B7 clock) and the **devotion accumulator** (a bounded, decaying scalar
the congregation feeds). Everything else is wiring the proven tavern machinery to a temple. This is the same
"build once, many uses" discipline as the world-verb library: the gather-at-a-social-plot engine, built once
for taverns, now serves faith for free.

---

## 3. Systems needed (with deps + which build-once engine each folds into)

### S1 — Colony-buildable temple/shrine (rides B4 Build + the `faith` zone)
**What:** a **shrine** (small, one worship point — a single altar block) and a **temple** (a `faith`-purpose
zone grouping an altar + congregation space + optional priest station), placed via the **B4 Build
designation** (you designate "build shrine/temple here"; colonists construct it, exactly like any structure).
The structure exposes a reachable **worship point** (the altar-facing congregation spot) as the addressing
unit for attendance and the legibility unit for the inspector — mirroring how the tavern plot exposes
`Detail::Bar`/`Stage` spots.
**Where:** reuses B4 Build + B5.6b zone schema (`purpose = faith`, from the canonical §2 enum) + a temple plot
authored on the `tavern.rs` room/detail pattern (new `Detail::Altar`/`Detail::Pew`). New: the colony temple
prefab + the worship-point marker.
**Deps:** B4/B5 Build, B5.6b-2 zone `purpose` enum. **Folds into:** the **zone↔asset taxonomy** (build-once)
+ the Build path. Not a new placement system.
**Function gate (per the asset-lab marker discipline):** the temple must ship a colonist-reachable worship
point (clearance for a congregation, door ≥2.2) — the same function-harness gate DF-PRODUCTION workshop shells
carry.

### S2 — Worship as a need + the attend-worship behaviour (rides B7 + the tavern loop)
**What:** (a) a **worship/faith need** on `Needs` (a `worship: f32` field, or a typed `recreation` sub-slot —
open question §8), decaying over time like hunger/rest, **B7-owned**; (b) the **attend-worship behaviour** —
the `go_to_tavern`/`socialize` loop retargeted at the nearest `faith` zone, where the colonist performs a
worship gesture (S-anim), satisfying the need and nudging **mood** up; (c) **personality-scaling** — the
decay rate / need weight scales with a colonist's devoutness (a devout mind needs worship badly; a faithless
one barely — ties B-AG3 values), so faith is *character*, not a uniform chore.
**Where:** the need field in `comp/bastion.rs::Needs` (co-locked with B7); the behaviour in rtsim `npc_ai`
(a `go_to_temple` sibling of `go_to_tavern`); personality weight reads B-AG3.
**Deps:** **B7 Needs decay+satisfy (HARD)** — worship is a B7 need; without B7 there is no decay to drive
attendance. The tavern behaviour (SUBSTRATE) is the code template. **Folds into:** B7 Needs (build-once needs
engine) + the rtsim social-plot loop. **This is the colony-tier heart of DF-RELIGION and it is B7-gated.**
**DF-FOCUS seam:** the worship need is the **first concrete instance of DF-FOCUS's "pray" personal-need**
(ledger §D DF-FOCUS: "personal needs derived from facets/values — pray, family, romance…"). Co-design: build
worship as the pilot need that proves the DF-FOCUS pattern; DF-FOCUS generalises it. Flag §8.

### S3 — Prophet / priest role + the congregation multiplier (extends Profession)
**What:** (a) a **`Priest`/`Prophet`** profession (new `Profession` arm) whose behaviour is "tend the shrine /
occupy the altar / lead worship" (a `go_to_temple`-and-stay, like the Chef-at-bar); (b) a **congregation
effect** — worship performed while a priest attends is *more effective* (bigger need refill / mood bump /
devotion gain), turning a temple-with-priest into a real focal point; (c) **emergence** — a sufficiently
devout colonist can *become* a lay-prophet autonomously (DF-style role emergence), and the player MAY
appoint/bless one (Manage / a god-act).
**Where:** `Profession` enum (`common/src/rtsim.rs`) + rtsim role behaviour + a congregation buff (rides
`BuffKind` — e.g. a transient `Fortitude`/mood effect on congregants near a tending priest).
**Deps:** S1 (a temple to tend), S2 (worship to lead). **Folds into:** the profession→behaviour mapping
(build-once) + the buff system. **rtsim law:** a priest is one NPC with a role, not a per-tick sim.

### S4 — Colony devotion aggregate + legibility (the DP2 seam — the load-bearing schema)
**What:** a bounded, **decaying colony-scale `devotion` scalar** — "how well-worshipped you are here" — fed by
congregation activity (S2/S3), decaying without temples/priests, and **capped by temple worship-capacity**
(carrying capacity). It is the single **god-game read** of the colony's faith and the **hand-off value** to
Divine-Politics DP2 (which turns per-colony devotion into faction faith state). Feeds the B13 favor supply
(devotion → favor-regen, per god-powers §2.6 passive "Favor accrual"). This is **load-bearing schema** — it
hardens into the DP2 interface and should be locked once, not re-invented per consumer (same discipline as the
DF-PRODUCTION `Quality` enum).
**Where:** a devotion accumulator on the colony record (sentiment-shaped: bounded, decaying), read by the HUD,
the favor economy (B13), and DP2.
**Deps:** S2 (the thing that feeds it). **Co-lock with Divine-Politics DP2 + B13 favor.** **Folds into:** the
sentiment-style accumulator pattern; it is DP2's first data source. **Flag §8 as a shared seam.**

### S5 — The divine seam: sanctify-ground / answer-prayer / be-worshipped→favor (rides B13 + God-Powers)
**What:** wire the **already-catalogued** divine verbs to the colony-tier substrate above:
- **Be worshipped → favor** (passive): congregation activity drips favor to the god (god-powers §2.6). The
  supply side of B13.
- **Sanctify ground / found shrine** (② blessing, god-powers §2.1): a god-act that marks a site holy → a local
  **devotion + favor-regen boost** (a spatial buff on the `faith` zone). The dominion-ambience anchor.
- **Answer a prayer** (① miracle, god-powers §2.3): a petitioning colonist's need answered → a **faith deepen**
  (devotion gain, *attributed*). The prayer-feed is Divine-Politics-owned, but the colony-tier hook (a colonist
  in worship can *voice an unmet need* — future-work §dialogue "no time to pray, it weighs on me") is here.
**Where:** these are **god-powers** (B13) — this design does NOT build them; it provides the devotion/zone
substrate they act on and asserts the wiring. **Deps:** B13 favor economy + the god-powers catalog. **Folds
into:** the God-Powers catalog (owner) + S4 devotion. **Flag:** B13-gated; ships when the god layer does.

---

## 4. Assets needed (READY / NEEDS-tagged) — the faith-asset batch

The faith batch is the visible content DF-RELIGION unlocks. Demand-ordered by what a colony builds first, and
it **flips NEEDS→READY as each sub-block lands** (§3i delegation model). A worldgen temple already renders, so
the *pattern* is proven; the colony batch is new.

| Asset | Tag | Notes |
|---|---|---|
| **Shrine** (small, one altar — the starter worship structure) | **NEEDS:DF-RELIGION** → READY on **REL-0** | The minimum viable faith building; per-race where sensible. First real demand. |
| **Temple** (the `faith`-zone structure — altar + congregation hall) | **NEEDS:DF-RELIGION** → READY on **REL-0** | Zone-scale; barn/witch-hut prefab pattern. Must ship a reachable worship point (function gate). |
| **Altar / idol / effigy** prop | **NEEDS:DF-RELIGION** → READY on **REL-0** | The worship focal point (the tavern's `Bar`/`Stage` analog). The congregation faces it. |
| **Pew / prayer-mat / kneeler** prop | **NEEDS:DF-RELIGION** → READY on **REL-1** | The congregation-spot dressing (the tavern's chairs analog). Makes worship *read*. |
| **Offering bowl / brazier / incense** prop | **NEEDS:DF-RELIGION** → READY on **REL-1** | Ambience + a future offering hook. Low priority. |
| **Priest/prophet vestment** (figure dressing) | **NEEDS:DF-RELIGION** → READY on **REL-2** | Makes the priest legible in a congregation. Ties the per-race cultural-look system (future-work §cultural). |
| **Faith/devotion overlay + HUD icons** | **NEEDS:DF-RELIGION** → READY on **REL-3** | Shares the overlay-rendering layer (future-work §overlays — build with mood/needs overlays). |
| Sanctified-ground VFX (holy shimmer on a zone) | **NEEDS:B13** → READY on **REL-4/B13** | The visible mark of a god-act; god-powers-owned. |

Near-term (REL-0/REL-1 consume): shrine + temple structures, altar/idol, pews → written to
`readme/ASSET_REQUESTS.md`. The rest stays on `BASTION-CONTENT-WISHLIST.md` until its sub-block lands.
**Style note for the pilot:** temples carry *lore that can affect the game* (future-work §lore: "a temple with
lore of a war-god could bias the faith it generates") — author the batch with a lore field so a later system
(inspector/chronicle/DP2 faith) can surface it. And per the coherence rule, temple *look* should track the
race's cultural identity (a dwarven hall vs an elven grove-shrine) — a per-race set where the pilot can manage
it, not one generic temple.

---

## 5. Animations needed (the no-T-posing rule — future-work §3u)

Worship is a named §3u animation priority (ordered *after* craft/farm/build: "…build-hammering → **worship/
prayer** (the faith layer's visibility) → the rest"). Every worship verb carries its line-item — and crucially,
**v1 worship is NATIVE**, because the congregation-gesture vocabulary already exists:

| Verb | Tag | Plan |
|---|---|---|
| **Worship / attend** (congregant at a temple) | **v1: NATIVE / enrichment: NEEDS:animation-code** | v1 reuses `NpcActivity::Sit` + `Cheer` facing the altar — *literally the tavern arena-seat crowd* (`npc_ai/mod.rs` "cheer, sit and dance"). Not a T-pose; ships free. Enrichment = `anim::pray` (kneel + bow). Named debt: `anim::pray`. |
| **Congregation** (many worshippers gathered) | **NATIVE** | The arena crowd pattern (many NPCs `Sit`/`Cheer` toward a focal point) — reused wholesale. Free. |
| **Kneel / prostrate** (deeper devotion, priest-led) | **NEEDS:animation-code** | `anim::kneel` — a new `CharacterState` + Animation. Enrichment only; v1 uses Sit. |
| **Priest — lead / bless gesture** | **v1: NATIVE (Talk/Cheer) / enrichment: NEEDS:animation-code** | v1 reuses `Talk`/`Cheer` (the priest "addresses" the congregation). Enrichment = `anim::bless` (raised-arms). Named debt: `anim::bless`. |

**The rule honored:** no worship verb ships as a T-pose — v1 bends toward existing states (Sit/Cheer/Talk),
and because the *crowd* pattern already exists, worship is the **cheapest** custom-animation topic in the
ledger (unlike production, whose craft-at-station had no native pose). Enrichment (REL-5-anim, or fold into the
§3u batch) replaces the stand-ins with `anim::pray/kneel/bless`. Debt is visible, not hidden.

---

## 6. Legibility · Control-spectrum · LOD (the three pillars every system answers)

### Legibility — how the god SEES the colony's faith
- **The temple renders itself** — a built temple with a gathered congregation *is* the legibility: you see
  your people worship. Free (the crowd renders today).
- **Devotion readout (the one god-game glance):** a colony **devotion meter** — "how well-worshipped are you
  here" — with a trend arrow (rising with attendance, falling without a temple/priest). This is the single
  most important faith read and the visible face of S4. It ties directly to the B13 favor supply.
- **Temple inspector (B-AG4 tab):** this temple's congregation size, its tending priest (if any), its
  worship-capacity vs current attendance, its devotion contribution, and its **lore** (which shapes the faith).
- **Faith overlay (B9, shared overlay layer):** devotion by area — where the faithful gather, where faith is
  thin. Built with the mood/needs overlays (future-work §overlays — one overlay engine).
- **The Chronicle (DF-LOG):** notable faith events — *first shrine raised*, *a prophet arises*, *the temple
  stood empty a season* (faith declining), *a prayer answered* (attributed god-act). The world's memory of its
  faith; also where an *ambiguous* divine act gets **attributed** after the fact (god-powers §1.2 — the faith
  payoff of a quiet miracle arrives when the chronicle names it).

### Control-spectrum placement (§7 / frameworks §1 / god-powers §1)
- **Autonomous (default):** colonists worship from the worship-need; congregations gather; prophets arise.
  Complete with zero player input.
- **Manage:** zone/build a temple (a Build designation); optionally appoint/bless a prophet. Provision, not
  command.
- **Direct — deliberately empty** (§0). No worship order exists; its absence is the guardrail.
- **God layer (the real surface — B13 / God-Powers catalog):** the divine control-spectrum applies (god-powers
  §1): **① Miracle** — *Answer a prayer* (attributed → deep faith), *Smite a heretic* (dear near the faithless);
  **② Blessing** — *Sanctify ground / found shrine* (standing devotion + favor-regen anchor), *Bless a
  congregation* (transient buff on worshippers); **③ Passive** — *be worshipped* (devotion → favor drip),
  *dominion ambience* (blessings cheaper on sanctified ground), *devout focus tilt* (the faithful run at
  slightly higher mood — the invisible passive). Attribution is the lever: a loud omen is a faith transaction;
  quiet fortune converts slowly and deniably.

### LOD story (loaded↔simulated — the rtsim law)
- **Loaded:** per-colonist worship-need decay + attend behaviour + the congregation gesture at the temple;
  a real priest tending a real altar.
- **Unloaded (rtsim):** a colony is an **aggregate devotion scalar** drifting toward the equilibrium its
  temples+priests support — *never* per-colonist worship sim (gotcha #1). A temple is `{capacity, has_priest}`;
  devotion tends toward `f(capacity, priest)` and decays below it. This is the DP2 tier's native
  representation.
- **The two laws:** every **accumulation** (devotion) has a **decay** (faith fades without worship — a temple
  left empty loses devotion, per the Chronicle "stood empty a season" event); every **population** (devotion)
  has a **carrying capacity** (temple worship-capacity × priest — you cannot bank infinite faith from one
  shrine). No unbounded faith.
- **Reconciliation on reload:** aggregate devotion promotes/demotes without dupe/loss across the boundary
  (the B10 persistence gate); a reloaded colony's devotion is consistent with its recent worship history.

---

## 7. Sequenced sub-blocks, each with a concrete Done-when (the buildable output)

Dependency-ordered. Each ships value alone, has an independent + (where sim) harness-assertable Done-when, and
a working entry point. **v1 colony tier = REL-0..REL-3; divine seam = REL-4 (B13-gated); world faith = REL-5
(LATE, DP-gated — seam only).** All Done-whens are invariant-first (bounded, decaying, conserved, no-panic)
where sim, screenshot/eyeball where visual.

### REL-0 — Temple/shrine as a buildable `faith` zone · [DF-RELIGION content + zone]
**Depends:** B4/B5 Build, B5.6b-2 zone `purpose` enum. Builds S1.
**Scope:** a shrine (one altar) and a temple (`faith` zone) placeable via the Build designation; colonists
construct it; it registers as a `faith`-purpose zone exposing a reachable worship point.
**Done-when (`--religion-build-scenario`):** designate a temple; colonists build it to completion; it
registers as a `faith` zone; the worship point is **colonist-reachable** (the function-harness pathability gate
passes — a colonist can path to and stand at the altar-facing spot, clearance ≥3, door ≥2.2); the structure
conserves materials through construction (B5 build conservation). Eyeball: it reads as a temple (altar +
congregation space render). Zero-input soak stable (no leaked zone records, no panic).

### REL-1 — Worship as a need + attend-worship behaviour · [DF-RELIGION core] — **B7-gated**
**Depends:** REL-0, **B7 Needs (HARD)**, the tavern behaviour (SUBSTRATE template). Builds S2.
**Scope:** a `worship` need on `Needs` decaying over time (B7-owned), personality-scaled; the `go_to_temple`
attend behaviour (the retargeted tavern loop) where a colonist with low worship-need travels to the nearest
`faith` zone, performs a worship gesture (v1 NATIVE Sit/Cheer facing the altar), refills the need, and nudges
mood up.
**Done-when (`--worship-scenario`):** a colonist whose `worship` need has decayed below threshold
autonomously travels to a built temple, performs a non-T-pose worship gesture at the worship point, the
`worship` need refills toward 1.0, and `Mood` rises measurably; the colonist then leaves (bounded visit). The
`worship` need is **bounded [0,1] and monotonic within a visit** (refills, never overflows); a devout-
personality colonist attends more often than a faithless one (personality scaling observable). Zero-input soak
stable: needs decay+refill in a bounded cycle, no runaway attendance, no leaked activity, no panic. *(Cannot
build or harness-test before B7 — near-frontier, not at-frontier; see §9.)*

### REL-2 — Prophet/priest role + congregation multiplier · [DF-RELIGION role]
**Depends:** REL-0, REL-1. Builds S3.
**Scope:** a `Priest`/`Prophet` profession whose behaviour is tend-the-altar (Chef-at-bar pattern); worship
performed while a priest attends is more effective (larger need/mood/devotion gain); a sufficiently devout
colonist can become a lay-prophet autonomously, and the player MAY appoint/bless one.
**Done-when (`--priest-scenario`):** a temple with a tending priest produces a **measurably larger** worship
benefit (need-refill or mood gain per visit) than the same temple without one; a congregation renders (the
reused crowd pattern — multiple worshippers seated/cheering toward the altar with the priest at it); a devout
colonist emerges as a lay-prophet over a soak without player input. The congregation buff is transient and
bounded (no permanent stacking). No panic; role assignment is stable across save/load.
**Festival hook (design note, NOT built here):** REL-2 is where a later DF-FESTIVAL would attach — a scheduled
congregation event that gathers the *whole* colony (the arena-crowd at max). Flag the hook; do not build the
scheduler (Tier-3, deferred).

### REL-3 — Colony devotion aggregate + legibility + rtsim LOD · [DF-RELIGION read + DP2 seam]
**Depends:** REL-1, REL-2. Builds S4 + §6 legibility.
**Scope:** the bounded, decaying colony `devotion` scalar fed by congregation activity, decaying without
temples/priests, capped by worship-capacity; the devotion meter + temple inspector + faith overlay + Chronicle
entries; rtsim aggregate devotion for unloaded colonies + reload reconciliation.
**Done-when:** (sim, `--devotion-scenario`) devotion **rises** with sustained worship, **decays** when the
temple stands empty, and is **capped** by temple capacity (a single shrine cannot bank unbounded faith); an
unloaded colony's devotion **drifts toward its temple-supported equilibrium** over rtsim ticks and, on reload,
materializes with **no dupe/loss** across the promote boundary (B10 gate). (visual) the overseer shows a
devotion meter with a correct trend arrow under a running/declining faith; the temple inspector shows
congregation + priest + capacity; the Chronicle logs *first temple* and *a prophet arises*. Devotion is
**bounded and conserved** at the LOD boundary.

### REL-4 — The divine seam: be-worshipped→favor + sanctify-ground · [rides B13 / God-Powers] — **B13-gated**
**Depends:** REL-3, **B13 favor economy + God-Powers catalog (S5)**. Wires, does not build, the god-powers.
**Scope:** congregation activity drips **favor** to the god (the passive supply side of B13); the *Sanctify
ground / found shrine* god-power marks a `faith` zone → a standing **devotion + favor-regen boost**; the
*Answer a prayer* hook deepens devotion (attributed).
**Done-when (`--sanctify-scenario`):** with the god layer active, a worshipping colony's activity increases
the god's favor over time (devotion → favor drip, bounded by capacity); casting *Sanctify* on a temple zone
raises that zone's devotion/favor-regen (a measurable, decaying boost, per god-powers §2.1 ② blessing); the
effect routes through the same authoritative buff/zone path as B5/B6 (no dupe, no griefing — god-powers §4
guardrail). *(B13-gated; ships when the god layer does.)*

### REL-5 — World faith-politics hand-off · [LATE — Divine-Politics DP2, seam only]
**Depends:** Divine-Politics **DP1–DP2** (LATE). **Not buildable now — this sub-block is a SEAM SPEC, not a
build.**
**Scope (design only):** colony devotion (S4) becomes the **input to DP2 faction faith state** — the colony is
a faction whose devotion-to-you sets its DP2 faith level; from there conversion, rival gods, and holy war
(DP3–DP4) take over. Festivals (DF-FESTIVAL), monastic orders (DF-GUILD tie), and per-colonist belief (B-AG3
values) also attach here.
**Done-when (of the *seam*, provable now):** S4's devotion scalar is shaped so DP2 can consume it directly
(bounded, decaying, per-faction-mappable, deity-attributable) — i.e. the schema is DP2-ready. **The world-
faith systems themselves are DEFERRED** (divine-politics-bible §6, Tier-3, "built late, after a single colony
reliably survives"). Do NOT build ahead of the colony core + agency + Divine-Politics substrate. Flagged
premature; designed as a seam so it doesn't fork.

---

## 8. Dependencies · open questions · tuning-data · corpus contradictions

### Dependencies (build-order truth)
- **B7 (Needs decay + satisfy) — HARD for REL-1..REL-3.** Worship is a B7 need; the whole colony-tier faith
  loop is a demand-driven attendance behaviour with no decay to drive it before B7. **This cluster sits just
  past B7 on the frontier** (near-term real, not buildable before B7). REL-0 (the buildable temple) is the one
  piece that can precede B7 — it's pure Build+zone.
- **B4/B5 Build + B5.6b-2 zone `purpose` enum — for REL-0** (the temple is a `faith`-purpose built structure).
- **B-AG3 (minds/values) — for REL-1 personality-scaling + REL-5 per-colonist belief.** Worship-devoutness is
  a mind trait; the loop works without it (uniform need) but is *characterful* with it.
- **B13 favor economy + God-Powers catalog — for REL-4** (the divine seam). Ships when the god layer does.
- **Divine-Politics DP1–DP2 — for REL-5** (world faith). LATE; seam only.
- **DF-FOCUS — co-design seam (not a blocker).** Worship is DF-FOCUS's first "pray" personal-need; build it as
  the pilot, generalise in DF-FOCUS.

### Open questions (flagged for Ben — genuine design calls, not defaults)
1. **Worship need — new field or `recreation` sub-slot?** Add `worship: f32` to `Needs` (clean, distinct
   behaviour + feeds devotion) or model it as a typed slot of the existing `recreation`? *Recommendation:* a
   **distinct `worship` field**, personality-scaled in magnitude — it drives a distinct plot (temple, not
   tavern) and a distinct output (devotion, not just mood). But it is **B7-owned schema** — co-lock with the B7
   designer so `Needs` is defined once (same "lock the schema once" discipline as the DF-PRODUCTION `Quality`
   enum). *Sub-question:* is the worship need **universal** (everyone has it, devout more so) or **gated by
   personality** (only the faithful have it at all)? *Recommendation:* universal but personality-scaled — a
   faithless colonist's worship need decays so slowly it rarely drives attendance, which reads correctly
   without a separate gate.
2. **Whom do they worship, pre-DP2?** Before the rival-god/faith-state system (DP2) exists, is there any deity
   model, or do colonists simply worship **you** (the player-god) natively? *Recommendation:* **they worship
   you** — divine-politics-bible §4 names the colony "your home flock"; colony-tier religion is
   worship-of-you, devotion accrues to you, favor flows to you. No rival-deity or multi-god model is needed
   until DP2 — which keeps DF-RELIGION firmly colony-tier and defers all the theology-contest complexity to
   Divine-Politics, exactly as the tier split intends.
3. **Prophet — emergent, appointed, or both?** DF-style the prophet *arises* from a devout population;
   RimWorld-style the player *assigns* a role. *Recommendation:* **both** — a devout colonist emerges as a
   lay-prophet autonomously (the autonomous default), and the player MAY appoint/bless one as a **Manage**/
   god-act (optional depth). Never *mandatory* appointment (that would make faith a management chore).
4. **Devotion granularity — colony-aggregate or per-colonist belief?** Track faith as one colony scalar (S4)
   or per-colonist belief values (rides B-AG3)? *Recommendation:* **colony-aggregate for the near term**
   (cheap, legible, LOD-native, DP2-ready); per-colonist belief is a B-AG3/DP2 enrichment, deferred. Design S4
   as the aggregate; note the per-colonist refinement as a later layer.
5. **Attendance — pure rtsim-AI (like taverns) or a need-job on the board?** The tavern loop runs in rtsim
   `npc_ai` directly; DF-FOCUS frames personal-needs as low-priority *jobs*. *Recommendation:* **rtsim-AI like
   taverns** for v1 (it's the proven, cheaper path and matches the "casual/consider" idle-activity model), with
   the need-job framing reserved for when DF-FOCUS generalises personal-needs. Flag for the B7/DF-FOCUS seam.

### Tuning-data (RON/config, not code — per §7-point-12)
Worship-need decay rate (+ personality-devoutness multiplier curve); worship-visit duration + need-refill +
mood bump; priest congregation multiplier; lay-prophet emergence threshold (devoutness × colony devotion);
devotion decay rate + regen rate + temple worship-capacity (the carrying cap); favor-per-devotion drip rate
(B13-shared); sanctify-ground boost magnitude + upkeep (god-powers-shared). **Balance lives in RON; the systems
read it.**

### Corpus contradictions / refinements found (flagged, not silently fixed)
- **Ledger cost/scope refinement (flag for architect):** `df-feature-gap-ledger.md` §H tags DF-RELIGION a
  single "$$ SUBSTRATE" line. The survey shows it is **two tiers with very different costs**: the **colony
  tier** (temples, worship-need, prophets, devotion) is **mostly wire** — a retargeted tavern loop + a B7 need
  field + a `Priest` profession arm + a devotion accumulator — closer to **`$`**; the **world faith-politics
  tier** (conversion, rival gods, holy war, festivals) is **`$$$` and is NOT DF-RELIGION at all — it is the
  Divine-Politics build (DP2–DP4)**. Recommend the ledger **split DF-RELIGION** into `DF-RELIGION` (colony
  tier, `$`, near-frontier behind B7) and a cross-reference that the faith-politics belongs to Divine-Politics.
  Flagged; not edited here.
- **Consistency, not conflict (the corpus *predicts* this design):** the canonical zone enum reserves
  `religious→faith` (frameworks §2); §3u lists worship/prayer as an animation priority; DF-FOCUS names the
  "pray" personal-need; the God-Powers catalog already catalogues sanctify/answer-prayer/be-worshipped; the
  Divine-Politics bible §4 names the colony "your home flock" and DP2 the faith system this feeds; future-work
  §social-faith marks temple a POINT zone "near housing (feeds FOCUS/religion)". Every corner of the corpus
  already anticipated DF-RELIGION — nothing contradicts this design; it slots into reserved sockets.

---

## 9. Honest limits (grading my own design)
- **B7 is a real gate.** The colony-tier heart (REL-1..REL-3) is a *need-driven* behaviour and cannot be built
  or harness-tested end-to-end before B7 Needs exists (no decay → no attendance to observe). It is
  **near-frontier, not at-frontier** — correct to design now (the schema is load-bearing: the `worship` need
  field, the devotion accumulator, and the DP2 seam all harden into code that other blocks touch), correct
  **not** to build before B7. REL-0 (the buildable temple) is the one piece that jumps the queue.
- **The payoff is split across three unbuilt systems.** The full loop — worship (B7) → devotion (this) → favor
  (B13) → world faith (DP2) — only closes when B7, B13, and Divine-Politics all exist. This design ships the
  colony substrate and the *seams*; it must not oversell a closed god↔faith loop until B13/DP2 land. The
  colony tier *is* complete and playable on its own (people worship, faith rises and fades, prophets arise) —
  but the "your worship makes you stronger" and "faith bends the world" payoffs are B13/DP2-owned and offstage
  until then.
- **Devotion (S4) is designed as the DP2 interface, so it is a seam, not a finished politics system.** Its
  shape must be co-locked with the Divine-Politics designer or it will fork — the same risk as the DF-PRODUCTION
  `Quality` enum. I've specified its invariants (bounded, decaying, capacity-capped, per-faction-mappable,
  deity-attributable) so DP2 can consume it, but the world-faith mechanics themselves are correctly deferred.
- **Rival gods, conversion, holy war, festivals are explicitly OUT.** They are the ledger's / Divine-Politics'
  Tier-3 epic (divine-politics-bible §6, "built late"). Designing them here would be exactly the "stale design
  ahead of a moving repo" failure the workflow guards against. I've drawn the seam (REL-5) and stopped.
- **Content is a real slice, not zero.** Unlike DF-PRODUCTION (whose content wall had fully fallen), the
  colony needs a *new buildable* temple/shrine + the altar/pew/idol batch — a bounded faith-asset request, but
  a genuine one. Tagged and demand-ordered; the worldgen temple proves the pattern but doesn't supply the
  colony structure.

*End of DF-RELIGION design. The scary topic (theology, conversion, holy war) turned out to be two topics: a
near-frontier colony tier that is mostly the tavern loop retargeted at a temple plus a B7 worship-need — and a
LATE world-faith tier that belongs to Divine-Politics. This pass builds the first and hands a clean seam to
the second, honoring the one truth that makes religion the most Bastion system in the ledger: you are not the
overseer arranging worship — you are the god being worshipped.*
