# Project Bastion — DF-PRODUCTION Design v0.1 (the industry cluster)

**One interlocking design pass for DF-WORKSHOP + DF-CHAIN + DF-FARM + DF-COOK** — the production/industry
layer that gives colonists something to *do* after B6 (stockpiles/hauling), and unlocks the biggest asset
batch (workshops, crops, crafted goods). Companion to the main build report (Slice B / B6), the DF gap ledger
(§D), `BASTION-SYSTEM-FRAMEWORKS.md` (§1 control spectrum, §2 zone↔asset taxonomy, §6 mining framework — its
sibling), and the animation rule (future-work §3u).

**Which wall:** primarily **SIMULATION** (autonomous colonist production behavior) with a **LEGIBILITY** tail
(how the god *reads* a colony economy). The **CONTENT wall already fell** — Veloren ships the entire recipe +
station + crafted-good corpus (see the reuse split). Almost nothing here is "generate assets"; it is "wire the
colonist brain to machinery that already exists."

**Fit-check verdict: PASS (with one guardrail).** Production is the single most dangerous cluster for pillar
drift — DF/RimWorld make you *queue jobs at a workshop*, which is exactly the 4X/unit-micro the pillar forbids.
The reframe that keeps it Bastion: **you never issue "craft X now."** You **zone** (this is the workshop
district), **place** stations (a designation, like B4 Build), and set **standing policy** ("keep ≥20 meals",
"cook wheat, never seeds"); colonists autonomously decide what to make from colony need + those policies. The
Manage layer is optional depth; the Direct layer (hand-queue a job) is a discouraged convenience, never
required. Held to that, production is a *beautiful* fit — it is the visible life of the colony you influence.

**Ledger/corpus entries this consolidates:** `df-feature-gap-ledger.md` §D — **DF-WORKSHOP** (typed production
buildings), **DF-CHAIN** (multi-step production chains), **DF-FARM** (plots/seeds/seasons), **DF-COOK**
(cooking/brewing/quality) — plus §K "kitchen permissions" (folds into DF-COOK) and a lightweight slice of §J
**DF-ORDERS** (standing production targets, the Manage layer). It appends to the corpus; it rewrites nothing.
It **refines** the ledger's cost estimates (see §9 — the ledger under-counted the Veloren substrate; this
cluster is more *wire* than *build* than "$$" implied).

---

## 0. The one thing to get right first — the god does not run the workshop

Every colony sim we steal from puts the player's hand *on the machine*: in DF you place a workshop and queue
"make stone door ×10"; in RimWorld you set a bill "make 4 meals, repeat forever, at this workbench." That is
**management as the mandatory core**. Bastion inverts it (Pillar §1a): production is **autonomous by default**
— colonists work stations on their own, driven by their needs (hungry → someone cooks) and the colony's
standing character, exactly as they already mine and chop in B4/B5 without you selecting a block. The player's
involvement is **influence at three optional depths** (the control spectrum, §7):

- **Autonomous (default + soul):** colonists produce what the colony needs — food when hungry, tools when
  worn, planks when building. You watch a living economy. *This must be a complete game with zero player input*
  (Tier-1b soak law).
- **Manage (optional):** you set **standing production policy** — targets ("keep ≥20 bread"), permissions
  ("never cook seeds"), zone purpose ("this district is industry"). Policy, not commands.
- **Direct (optional, discouraged):** you may hand-queue one job at one station. A convenience for the fiddler,
  never the loop. If it becomes the loop, the design failed.

**North star for the whole cluster: zone and policy, never command; produce from need, never from a click.**

---

## 1. The reuse split — the de-risk table (SUBSTRATE vs BUILD, real symbols)

This is the heart of the pass. **The DF-industry mountain is ~80% already in the repo as data.** What Veloren
ships (verified against the tree, not guessed):

### SUBSTRATE — exists, needs wiring

| Piece | Real symbol / location | What it gives us |
|---|---|---|
| **Recipe engine** | `common/src/recipe.rs` — `Recipe { output: (Arc<ItemDef>, u32), inputs: Vec<(RecipeInput, u32, bool)>, craft_sprite: Option<SpriteKind> }`; `RecipeInput::{Item, Tag, TagSameItem, ListSameItem}` | Multi-input recipes with amounts + tag/list matching. **This IS DF's reaction system.** |
| **Recipe corpus** | `assets/common/recipe_book_manifest.ron` — **326 recipes** | The entire crafted-good catalog already authored. DF-CHAIN's chains **already exist as data** (see below). |
| **Workshop stations** | `common/src/terrain/sprite/mod.rs` — `SpriteKind::{CraftingBench, Forge, Cauldron, Anvil, CookingPot, SpinningWheel, TanningRack, Loom, DismantlingBench, RepairBench}` | The full DF workshop set as placeable sprites. Recipe usage: CraftingBench 132, Anvil 72, Loom 56, Cauldron 20, Forge 10, CookingPot 8, SpinningWheel 7, TanningRack 4. |
| **Craft execution** | `Recipe::craft_simple(inv, slots, ability_map, msm)` + server handler `server/src/events/inventory_manip.rs` `InventoryManip::CraftRecipe` / `CraftEvent::Simple` | Consumes inputs from an inventory, emits outputs — **conservation-correct already**. Gated by `within_pickup_range` of the required `craft_sprite`. |
| **Plant growth attribute** | `common/src/terrain/sprite/mod.rs` — `Growth(pub u8)` (0..16), `Plant` category `has_attr::<Growth>()`; `WheatGreen` etc. | Crops **already carry a growth stage and render it**. Farming's legibility is half-free. |
| **Food items** | `ItemKind::Consumable { kind: ConsumableKind::{Food, ComplexFood, Drink, …}, effects }`; existing cooked foods (`fish_cooked`, `bird_cooked`, cheese/dough) | The cooked-good outputs already exist with buff effects. DF-COOK's *outputs* are authored. |
| **Cooking recipes** | `recipe_book_manifest.ron` — 8 `CookingPot` + 20 `Cauldron` recipes | The kitchen/still content exists; only the *permission layer* + *colonist cook AI* are new. |
| **Job board loop** | `server/src/bastion_jobs.rs` — claim→travel(`NpcActivity::Goto`)→arrive→work-tick→complete; `WorkType`, `ActiveJob`, `JobBoard` | The whole B4/B5 designation→job→arbitration→work pipeline. A produce-job is a **new `WorkType` on this board**, not a new system. |
| **Zone taxonomy** | `BASTION-SYSTEM-FRAMEWORKS.md` §2 (hardening in B5.6b-2) — `industrial→production`, `agricultural→farming` | Workshops are `production` zones; farms are `farming` zones. Rides the zone system, not a parallel one. |
| **Needs / skills** | `common/src/comp/bastion.rs` — `Needs { hunger, rest, recreation }`, `Mood`; `ColonistSkills`/`WorkPriorities`, `ColonistSkills::grant_xp` | The demand signal (hunger) and the skill→speed/quality feedback already exist as fields. |
| **Item stockpiles / piles** | `comp::bastion::BastionPile` (B5.5) — count-based `PickupItem` piles | Input sourcing + output storage. **The B6 hauling loop is the logistics for production.** |

### BUILD — genuinely net-new

| Piece | Why it's new | Folds into |
|---|---|---|
| **The produce-job** (work-at-station) | The craft handler is *player-inventory-driven*; no colonist AI selects a recipe, sources inputs, stands at a station, and runs a craft-tick. | The B4/B5 **job board** (new `WorkType::Produce`) |
| **Input sourcing as haul** | A colonist must fetch recipe inputs from a stockpile to the station before crafting (DF's "haul reagents to workshop"). | The **B6 hauling** loop (hard dependency) |
| **Standing production orders** | The "keep ≥N" pull that turns isolated crafts into chains and keeps it *policy not command*. | Lightweight **DF-ORDERS** (Manage layer) |
| **Plant growth ticking** | `Growth` exists but **nothing advances it** (worldgen-set only; verified: no server/state system increments it). This is the one real sim to build. | New **farm-growth system** (`bastion_farm`) |
| **Farm verbs** (till / sow / harvest) | No plant-placement or harvest interaction exists. | The **job board** (new `WorkType`s) + the growth system |
| **Seed items** | No dedicated seed items exist (only foods). DF-style: plant a seed, harvest returns crop + seeds. | Content (RON item defs) — small |
| **Cook/brew permission policy** | "Which crops may be cooked/brewed" (don't cook your seeds) — the gameplay depth of §K, colony-wide standing data. | The Manage/policy layer |
| **Meal / good quality tier** | Quality from crafter skill → value + mood (DF's masterwork ladder). | **DF-QUALITY** (shared schema — coordinate) |
| **Production legibility** | Economy readout, workshop/farm inspector, Chronicle entries. | The B9 HUD + B-AG4 inspector + DF-LOG |
| **rtsim aggregate production** | Unloaded colonies produce/consume as rates (never per-station/per-plant sim in rtsim). | The loaded↔simulated LOD law |

**The collapse:** DF-WORKSHOP + DF-CHAIN are *almost entirely wiring* (recipes, stations, chains, execution
all exist — the new work is one `WorkType` + the order-pull). DF-COOK is *mostly wiring* + a permission policy.
**DF-FARM carries the only real net-new simulation** (growth over time) — and even that has its data
substrate (the `Growth` attribute + crop sprites) and its legibility (the sprite renders the stage) already in
place. This is the same "reuse-first collapses a scary topic" result as B3/B6/B8/B13.

---

## 2. How Veloren's recipes already ARE DF-CHAIN (the key insight)

DF-CHAIN's headline — "ore→bar→goods; plant→thread→cloth→dye" — reads as a system to build. It is not. It is
an **emergent property of recipes whose inputs are other recipes' outputs**, and Veloren's 326-recipe manifest
already contains those links:

- **Metal chain:** ore → (`Forge`, smelt) → bar → (`Anvil`, forge) → tool/weapon/armor. 10 Forge + 72 Anvil
  recipes.
- **Textile chain:** fibre → (`SpinningWheel`) → thread → (`Loom`) → cloth → (`CraftingBench`) → garment.
  7 SpinningWheel + 56 Loom recipes.
- **Food chain:** crop → (`Cauldron`/`CookingPot`) → meal/drink (feeds Needs).

**So we do not design chains. We design the *pull* that walks them.** When a "keep 20 iron tools" order finds
no bars in stock, and the bar recipe finds no ore, the order system emits the upstream produce-jobs
(smelt, then mine) automatically — the chain is discovered by following `RecipeInput` backward, not authored.
This is the same "build once, many uses" discipline as the world-verb library: **one demand-propagation
mechanism, every chain for free.** (v1 may cap propagation depth for legibility/safety — see PROD-1.)

---

## 3. Systems needed (with deps + which build-once engine each folds into)

### S1 — The produce-job executor (`bastion_jobs` extension)
**What:** a new `WorkType::Produce { recipe, station }` on the B4/B5 job board. Lifecycle: arbitration
assigns an idle colonist with the right labor/skill → colonist **sources inputs** (haul sub-jobs, B6) → travels
to a station of `craft_sprite` kind → runs a **craft work-tick** accumulating progress at `work_rate(skill)`
(reuse B5's rate) → on completion calls `Recipe::craft_simple` against the colonist's inventory → emits output
as a `BastionPile` drop (or hands to haul-to-storage) → grants XP.
**Where:** `server/src/bastion_jobs.rs` (+ a small `bastion_produce.rs` for recipe/station selection).
**Deps:** **B6 hauling (hard)** — input sourcing and output storage are haul jobs. Reuses S-craft execution
(`craft_simple`) verbatim, so conservation is inherited, not re-proven.
**Folds into:** the **job board** (build-once). A produce-job is a Mine/Chop/Build sibling; the work-tick's
*effect* is `craft_simple` instead of `BlockChange::set`.

### S2 — Station placement & the workshop zone (rides zones + designation)
**What:** a station is a placed `craft_sprite` block (placed via the B4 Build designation path — you designate
"build CraftingBench here", colonists construct it). A **production zone** (`industrial→production`, from the
canonical §2 taxonomy) groups stations + a linked input/output stockpile; it is the addressing unit for orders
and the legibility unit for the inspector.
**Where:** reuses B5.6b zone schema (`z_extent`, `purpose`) + B4 Build. New: station↔zone linkage data.
**Deps:** B5.6b-2 zone schema (purpose enum), B4/B5 build. **Folds into:** the **zone↔asset taxonomy**.

### S3 — Standing production orders (the Manage layer / DF-ORDERS-lite)
**What:** per-zone (or colony-wide) standing targets: `keep ≥ N of <item>`, `craft <recipe> ×K`, `repeat`.
An order with unmet target and available inputs generates produce-jobs (S1); unmet inputs propagate upstream
(§2). This is the entire "player influences production" surface — **policy, not commands**.
**Where:** new `bastion_orders.rs` (server) + a small HUD panel (B9). Order data is RON-tunable defaults +
per-save state.
**Deps:** S1. **Folds into:** the **control spectrum** (Manage tier); this is the v1 slice of ledger DF-ORDERS
(full conditional orders remain a later block — flag §10).

### S4 — Farm plots + the growth system (`bastion_farm`) — the one real sim
**What:** (a) a **farm zone** (`agricultural→farming`, thin `z_extent`) of tillable ground; (b) **till** and
**sow** verbs (job-board `WorkType`s) that place a `Growth(0)` crop sprite in a tilled cell; (c) a
**growth-ticking system** that advances `Growth` over game-time — season/temperature-scaled, LOD-aware,
**tendency-first** (a crop matures toward harvest; failure = withers, not a crash); (d) a **harvest** verb
(auto-generated when `Growth == max`) that yields crop items + seeds and returns the cell to tilled.
**Where:** new `server/src/bastion_farm.rs` (growth tick) + job-board verbs + a few RON crop defs + seed items.
**Deps:** the job board (verbs), zones (farm zone). **Independent of B6** for the core loop (till/sow/grow/
harvest can run on one plot without hauling), so **DF-FARM can partly precede B6** — a scheduling win.
**Folds into:** the job board + zones; the growth tick is genuinely new (the only net-new sim engine here).
**rtsim law:** never per-plant in rtsim — an unloaded farm is `{crop, plot_count, maturity_date}`; it yields a
lump at the season boundary. Every crop yield has a **carrying capacity** (plot count × fertility); the balancing
**drain** on the food it produces is colonist consumption (B7).

### S5 — Cooking, brewing & the permission layer (DF-COOK)
**What:** cooking/brewing are **produce-jobs (S1) at CookingPot/Cauldron using the existing recipes** — almost
free. Net-new: (a) the **cook/brew permission policy** — colony-wide standing data marking each cookable/
brewable item allowed/forbidden ("cook wheat→bread; never cook seeds; brew barley"), the gameplay depth of §K;
(b) **meal quality** (S6) from cook skill; (c) the **consumption→mood payoff** (a hungry colonist eats a meal;
meal quality shifts mood) — **flagged B7-gated** (the `Needs.hunger` decay + eat behavior is B7, unbuilt).
**Where:** permission data in RON + a HUD toggle list (the DF Kitchen subtab, reframed as policy); consumption
in B7's needs system.
**Deps:** S1 (production half ships now); **B7 (consumption/mood half)**. **Folds into:** the produce-job +
the Manage/policy layer.

### S6 — Quality tier (shared schema — coordinate with DF-QUALITY)
**What:** a `Quality` tier on produced goods (crude→…→masterwork→artifact), derived from crafter skill,
feeding item **value** and (on consumption/possession) **mood**. This is **load-bearing schema** — it hardens
into item data and should be locked once, not re-invented per producer.
**Where:** a `Quality` enum + an item field; producers stamp it at `craft_simple` time.
**Deps:** none technically, but **must be co-designed with ledger DF-QUALITY + DF-ARTIFACT** (the artifact is
the top of this ladder). **Flag §10.** **Folds into:** DF-QUALITY (this cluster is its first consumer).

---

## 4. Assets needed (READY / NEEDS-tagged)

The content wall already fell for the *core* — stations and crafted goods render today. Asset work here is
**breadth and polish**, demand-ordered by what colonists actually make, and it **flips from NEEDS to READY as
each sub-block lands** (§3i delegation model).

| Asset | Tag | Notes |
|---|---|---|
| Workshop station models (Anvil/Forge/Loom/CookingPot/…) | **READY** | Already sprites in the game; asset-lab may make richer/varied models, but the system consumes them today. |
| Workshop *buildings* (the smithy/kitchen/loom-house shells the stations sit in) | **NEEDS:DF-WORKSHOP** → READY on **PROD-0** | The zone-scale structures; barn/witch-hut prefab pattern (§3i). |
| Crop models + growth-stage sprites (new crops beyond wheat) | **NEEDS:DF-FARM** → READY on **PROD-2** | Wheat-style multi-stage sprites keyed to `Growth(0..max)`. Existing crops already staged. |
| Seed item icons | **NEEDS:DF-FARM** → READY on **PROD-2** | Small icon set; one per farmable crop. |
| Crafted-good models (furniture, tools, trade goods) | **READY** (recipes exist) | The huge batch; generate demand-ordered, not all 300 at once (§3i warning). |
| Prepared-meal / drink icons (beyond existing) | **READY** | Consumable items already render; new meals are new item defs + icons. |
| Farm-plot / tilled-soil ground texture + fence/trellis dressing | **NEEDS:DF-FARM** → READY on **PROD-2** | Makes a farm *read* as a farm. |

Near-term (PROD-0..PROD-2 will consume): workshop-building shells, tilled-soil + crop-stage sprites, seed
icons → written to `readme/ASSET_REQUESTS.md`. The rest stays on the wishlist until its sub-block lands.

---

## 5. Animations needed (the no-T-posing rule — future-work §3u)

Production is the **#1 and #2 custom-animation priority** in §3u (craft-at-station, then farm gestures).
Every new verb carries its line-item:

| Verb | Tag | Plan |
|---|---|---|
| **Craft at station** (hammer at anvil/forge; work at bench) | **v1: NATIVE stand-in / enrichment: NEEDS:animation-code** | v1 reuses the equipped-tool wield/swing (the B5 mining-swing wiring) as a generic "working" pose so it isn't a T-pose; enrichment = a proper `CharacterState` + Animation impl per station family (hammer, stir, weave). Named debt: `anim::craft_hammer`, `anim::craft_stir`, `anim::craft_weave`. |
| **Stir pot** (cook/brew) | folds into craft-at-station | CookingPot/Cauldron use `anim::craft_stir`. |
| **Farm — hoe / till** | **NEEDS:animation-code** | `anim::farm_hoe` (downward tool gesture). v1 stand-in: crouch/interact pose. |
| **Farm — sow** | **NEEDS:animation-code** | `anim::farm_sow` (scatter gesture). |
| **Farm — harvest** | **NEEDS:animation-code** | `anim::farm_harvest` (bend/gather; can reuse the gather/pickup state as v1 stand-in). |
| Haul inputs/outputs | **NATIVE** | Locomotion + B6 carry (individual-carry polish is a separate §3u item). |

**The rule honored:** no produce verb ships as a T-pose — v1 bends toward an existing state (mining-swing /
gather / crouch stand-ins), enrichment (PROD-5) replaces them with named custom impls. The debt is *visible*,
not hidden.

---

## 6. Legibility · Control-spectrum · LOD (the three pillars every system answers)

### Legibility — how the god SEES the economy
- **The farm renders itself** — the `Growth` attribute already drives the crop sprite through visible stages;
  a green field ripening to gold *is* the legibility, free. Tilled soil + fences read the zone.
- **Workshop/station inspector (B-AG4 tab):** what this station is making, its input backlog (what it's
  waiting on), the assigned colonist, output stock. A station idle-for-lack-of-inputs shows *why*.
- **Production zone panel (B9):** per-zone summary — active orders, throughput, bottleneck ("no ore").
- **Colony economy readout (the DF Stocks screen, reframed):** stocks of key goods (food / drink / materials /
  tools) with **trend arrows** (net production vs consumption) — the single most important "is my colony
  healthy?" glance. Surplus/deficit is the whole god-game read.
- **The Chronicle (DF-LOG):** notable production events — first harvest, a masterwork/artifact, a famine
  warning (food trend negative), a stalled workshop. The world's memory of its economy.

### Control-spectrum placement (§7 / frameworks §1)
- **Autonomous (default):** colonists produce from need — hunger drives cooking, wear drives smithing, builds
  pull materials. Complete with zero player input.
- **Manage:** standing orders (S3), cook/brew permissions (S5), zone purpose (S2). Policy dials.
- **Direct (discouraged):** hand-queue one job at one station. Present for the fiddler; never the loop.
- **God layer (B13 / God-Powers catalog):** production is a rich target for divine acts —
  **① Miracle:** *Blessed Harvest* (instant-ripen a farm), *Kindle the Forge* (a burst of production).
  **② Blessing:** *Abundance* (a farm/workshop's output tier +1 while the blessing holds — a standing
  enchantment, upkeep drip). **③ Passive:** worship/favor gives a background fertility/craft-luck drift.
  Attribution/legibility per the catalog (miracles loud, passives read as fortune). These are **causes the
  colony reacts to**, never unit orders — a blessed harvest ripples into the economy (surplus → trade, mood).

### LOD story (loaded↔simulated)
- **Loaded:** full per-station craft-ticks; per-plant `Growth` advance; real hauling.
- **Unloaded (rtsim):** a colony is **aggregate rates** — stockpile totals drift by (net production −
  consumption); a farm is `{crop, plots, maturity_date}` yielding a lump at the season boundary; workshops are
  a throughput number, not per-station sim. **Never push per-station/per-plant sim into rtsim** (gotcha #1).
- **Reconciliation on reload:** aggregate stocks materialize as piles; no dupe/loss across the promote/demote
  boundary (the B10 persistence + conservation invariants gate this).
- **The two laws:** every **accumulation** (a stockpile) has a **decay/drain** (consumption/spoilage — spoilage
  ties DF-ROT); every **population** (crop yield, workshop throughput) has a **carrying capacity** (plot count ×
  fertility; station count × labor). No unbounded growth.

---

## 7. Sequenced sub-blocks, each with a concrete Done-when (the buildable output)

Dependency-ordered. Each ships value alone, has an independent + harness-assertable Done-when, and a working
entry point. **v1 slice = PROD-0..PROD-3; enrichment = PROD-4..PROD-5.** All Done-whens are invariant-first
(conservation, bounded, no-panic) where sim, screenshot/eyeball where visual — a builder gates against them
without ambiguity.

### PROD-0 — The produce-job (work-at-station core) · [DF-WORKSHOP]
**Depends:** B6 hauling (input sourcing / output storage). Builds S1 + S2.
**Scope:** one `WorkType::Produce`; colonist claims a station, (hauls inputs), runs the craft-tick, emits
output via `craft_simple`, grants XP. Station placed via B4 Build.
**Done-when (`--prod-scenario`):** a colonist with the smith labor + a CraftingBench + the required inputs in
a linked stockpile completes a single recipe: colonist travels to the station, input items are consumed, the
output item appears as a pile, XP is granted to the correct skill, and **conservation holds** across the tick
(Σ inputs consumed == recipe inputs; Σ outputs == recipe output; no dupe, no loss) — asserted like the B5 drop
conservation. Zero-input soak from spawn is stable (no leaked claims, bounded tick time, no panic).

### PROD-1 — Standing production orders + emergent chains · [DF-CHAIN + DF-ORDERS-lite]
**Depends:** PROD-0. Builds S3 + the §2 demand-propagation.
**Scope:** per-zone `keep ≥ N` / `craft ×K` orders generate produce-jobs; unmet inputs propagate one or more
steps upstream (depth-capped for v1).
**Done-when (`--chain-scenario`):** (a) with a "keep ≥ N planks" order and logs in stock, colonists produce
planks until stock ≥ N then **stop**; raising N **resumes**; lowering N below stock produces nothing.
(b) A two-step chain — order "keep ≥ K furniture" with only logs in stock — completes: the system auto-emits
the upstream "make planks" jobs, then the furniture jobs, and K furniture appear with **exact conservation**
along the chain (no material dupe/loss at any step). Bounded, no runaway job creation (order queue length
capped and asserted).

### PROD-2 — Farm plots + growth sim + harvest · [DF-FARM]
**Depends:** job board + zones (can partly precede B6 — the single-plot loop needs no hauling). Builds S4.
**Scope:** farm zone; till + sow verbs place `Growth(0)` crop sprites; `bastion_farm` growth-tick advances
`Growth` over accelerated game-time; harvest at `Growth==max` yields crop + seeds and re-tills.
**Done-when (`--farm-scenario`):** designate a farm plot; a colonist tills and sows a seed (a `Growth(0)`
crop sprite appears and **renders through its stages** as growth ticks); over accelerated ticks `Growth`
reaches max; a harvest job auto-generates; the colonist harvests → crop items + seeds land in a pile and the
cell returns to tilled. **Seed conservation:** harvest returns ≥ seeds planted (net-positive or break-even per
the RON yield, never a net loss that would extinguish the crop). Growth is **bounded and monotonic toward
maturity** under normal conditions; the zero-input soak is stable (no runaway sprite edits, bounded tick).

### PROD-3 — Cooking, brewing & the permission layer · [DF-COOK]
**Depends:** PROD-0, PROD-2 (raw crops to cook). Builds S5 (production half) + the permission policy.
**Scope:** cook/brew produce-jobs at CookingPot/Cauldron using existing recipes; colony-wide cook/brew
permission data. (Consumption/mood explicitly **deferred to B7** — see §10.)
**Done-when (`--cook-scenario`):** with a permission set {cook wheat→bread: allow; brew barley: allow; cook
seeds: forbid} and raw wheat + barley + seeds in stock, colonists produce bread and drink but **never consume
the forbidden seed stock** (seed count invariant across the run); outputs are `Consumable` items with correct
kinds; conservation holds. Toggling a permission off mid-run stops that product's new jobs within one
arbitration cycle.

### PROD-4 — Production legibility + rtsim aggregate LOD · [enrichment]
**Depends:** PROD-0..PROD-3. Builds §6 legibility + the LOD summary.
**Scope:** economy stocks readout (trend arrows), workshop/farm inspector tabs, Chronicle entries; rtsim
aggregate production/consumption for unloaded colonies + reload reconciliation.
**Done-when:** (visual) the overseer shows a stocks panel with key goods + correct trend direction under a
running economy; a stalled workshop shows its bottleneck. (sim, `--prod-lod-scenario`) an unloaded colony's
stock totals advance by the aggregate net rate over rtsim ticks and, on reload, materialize with **no dupe/
loss** across the promote boundary (B10 conservation gate).

### PROD-5 — Craft & farm animations · [enrichment]
**Depends:** PROD-0..PROD-3. Pays the §5 animation debt.
**Scope:** replace the v1 stand-in poses with named custom impls (`anim::craft_hammer/stir/weave`,
`anim::farm_hoe/sow/harvest`) + their `CharacterState` triggers.
**Done-when:** (screenshot/eyeball) a crafting colonist plays a station-appropriate work animation (hammer at
anvil, stir at pot, weave at loom), and a farming colonist hoes/sows/harvests with the matching gesture — not
a T-pose and not the raw mining-swing stand-in. The job executor sets `CharacterState` + tool per verb.

---

## 8. Dependencies · open questions · tuning-data · corpus contradictions

### Dependencies (build-order truth)
- **B6 (stockpiles/hauling) — HARD for PROD-0/1/3.** Production logistics *is* hauling: fetch inputs to the
  station, carry outputs to storage. **This cluster sits just past B6 on the frontier** (near-term real, not
  buildable before B6). **Exception: PROD-2 (farm core loop) can partly precede B6** — a single self-contained
  plot needs no hauling — a useful parallel-scheduling option.
- **B7 (Needs decay + eat behavior) — for the DF-COOK consumption/mood payoff and the "why farm" pressure.**
  Production ships on B6; consumption rides B7. Don't block the cluster on B7 — ship production, wire
  consumption when B7 lands.
- **B5.6b-2 (zone `purpose` enum) — for S2/S4 zone typing.** The canonical 8-kind taxonomy (frameworks §2).
- **DF-QUALITY / DF-ARTIFACT — co-design S6.** The quality tier is shared schema; the artifact is its apex.

### Open questions (flagged for Ben — genuine design calls, not defaults)
1. **Standing-orders scope in this cluster.** Ship the minimal `keep ≥N` target pull inside PROD-1, or gate all
   ordering behind a full DF-ORDERS block? *Recommendation:* ship the minimal target-pull here (it's what makes
   chains emergent and keeps it policy-not-command); full conditional orders ("if winter and food<X…") stay a
   later DF-ORDERS block.
2. **Farm growth cadence.** Continuous per-tick `Growth` advance (loaded) scaled so a crop matures in ~one
   in-game season, with a discrete season-jump in rtsim? Or fully discrete season-boundary growth everywhere?
   *Recommendation:* continuous-when-loaded (it's watchable and pretty), discrete-when-unloaded (LOD).
3. **Consumption loop home.** Wait for B7 entirely, or ship a minimal "colonist eats from stockpile when hunger
   low" stub inside PROD-3 so the food economy closes the loop earlier? *Recommendation:* wait for B7 (avoid a
   throwaway stub); PROD-3's Done-when is production-only by design.
4. **Quality schema now or later.** Define the `Quality` enum in this cluster (S6) as the shared schema since it
   hardens into item data, or defer entirely to DF-QUALITY? *Recommendation:* co-design the enum *now* with the
   DF-QUALITY designer and lock it once (load-bearing schema); this cluster is its first consumer.

### Tuning-data (RON/config, not code — per §7-point-12)
Per-crop growth rate + season windows + fertilizer bonus; per-station work-rate; order target defaults +
propagation depth cap + order-queue length cap; meal-quality→value/mood curve; skill→quality thresholds;
spoilage/rot rate per food (ties DF-ROT); seed yield-per-harvest per crop. **Balance lives in RON; the systems
read it.**

### Corpus contradictions / refinements found (flagged, not silently fixed)
- **Ledger cost refinement (not a contradiction):** `df-feature-gap-ledger.md` §D tags DF-WORKSHOP "$$" and
  "SUBSTRATE-ish," DF-CHAIN "$$", DF-COOK "$". The verified survey shows the **content is fully present** (326
  recipes, all stations, craft execution, cooked-good items) — the net-new is a job-board `WorkType` + an
  order-pull + a permission list. **DF-WORKSHOP/CHAIN/COOK are closer to `$` (wire) than `$$` (build);
  DF-FARM is the only `$$` here** (the growth sim). Recommend the ledger note this leverage. Flagged for the
  architect; not edited here.
- **Consistency, not conflict:** the zone taxonomy (frameworks §2: `industrial→production`,
  `agricultural→farming`) and the animation rule (§3u priority order: craft → farm → build → worship) both
  *predict* this cluster exactly. Nothing in the corpus contradicts this design; it slots in.

---

## 9. Honest limits (grading my own design)
- **B6 is a real gate.** Most of this cluster cannot be built or even harness-tested end-to-end before B6
  hauling exists (inputs can't reach stations). It is **near-frontier, not at-frontier** — correct to design
  now (the schema/vocabulary is load-bearing and the recipes are stable), correct **not** to build before B6.
  PROD-2 (farm) is the one piece that can jump the queue.
- **The consumption loop is the payoff and it's B7-owned.** Until Needs decay + eat behavior exist (B7), the
  food economy is production without a demand sink — the "why does this matter" is real but *offstage*. This
  design deliberately ships the producing half and hands the consuming half to B7; don't oversell it as a
  closed loop until then.
- **Quality is designed as a stub pending DF-QUALITY.** S6 is a seam, not a finished system — it must be
  co-locked with the DF-QUALITY pass or it will fork.
- **rtsim aggregate production (PROD-4) is the least-specified.** The loaded loop is concrete; the unloaded
  aggregate is a principle (rates + carrying capacity) that needs its own numeric model when built — flagged,
  not hand-waved as done.

*End of DF-PRODUCTION design. The mountain (DF's economic depth) was mostly already in the repo as recipe
data; this pass finds the small, specific net-new work — a produce-job, an order-pull, and a growth sim — and
sequences it behind B6 with testable Done-whens.*
