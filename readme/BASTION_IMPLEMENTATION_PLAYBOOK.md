# Project Bastion — Approved Implementation Playbook

> Coding direction for every TODO row in the [Master Build List](BASTION_MASTER_BUILD_LIST.md). The active
> assignment is always [BASTION_CURRENT_BLOCK.md](BASTION_CURRENT_BLOCK.md). Designs resolve through the
> [Design Index](BASTION_MASTER_DESIGN_INDEX.md); ownership contracts live in the
> [Shared-Engine Registry](BASTION_SHARED_ENGINE_REGISTRY.md).

## ⛔ GLOBAL BUILDER RULES (apply to EVERY block, no exceptions)
- **NO SUB-AGENTS, EVER** — never spawn `Task` / `Agent` / `Workflow` or anything that starts a second agent context. They are massive token dumps and are **permanently BANNED**. Do 100% of the work in your own single context: read → grep only what you need → write → run the gate.
- **Only offload allowed:** a long review → the **Sonnet reviewer SESSION** via `send_message` (`local_5f3f9b01`); the safety gate → the **Opus reviewer session**. Never a spawn.
- **Never idle-wait** (pipeline other code while a build/test runs). **Low token:** terse bookkeeping, don't reread the whole roadmap.

## Universal implementation law

1. Extend authoritative Veloren paths; never create private terrain, inventory, combat, buff, or persistence paths.
2. Shared mutable decisions run deterministically and sequentially: observations → generators → arbiter → claim/execute → events → Chronicle/save-back.
3. Durable state is serde-ready with safe defaults. Claims, caches, path requests, and UI snapshots are transient.
4. Loaded ECS and abstract rtsim have one owner each and an atomic transition. Abstract simulation is a conservative approximation, not copied per-agent code.
5. Add the deterministic scenario with the block. Assert invariants, not brittle literal outcomes.
6. Animation and UI present server truth; they never own gameplay completion.

## JOB-CORE — folded into B6-HAUL

Current `Job` stores `DesignationKind` directly (`common/src/bastion.rs`), which cannot honestly represent
autonomous Haul/Gather/Farm/Produce/Eat/Worship work. Keep `DesignationKind` as serialized player intent and
introduce a server executable vocabulary:

```rust
enum JobKind {
    Designated(DesignationKind),
    Gather { resource: TerrainResource },
    Haul { item: Uid, destination: ZoneId },
    // Added only by later owning blocks: Farm, Produce, Eat, Worship, Medical…
}

struct Job {
    id: JobId,
    kind: JobKind,
    target: JobTarget,
    work: WorkType,
    reservation: Option<ReservationId>,
    claimed_by: Option<Uid>,
    progress: f32,
    unreachable: bool,
}
```

- One `ReservationTable` owns item, station, and destination reservations.
- Derive stock counts from physical items and `BastionPile`; no second mutable count.
- Append serialized enum variants; never reorder them.
- Rebuild claims/reservations after load unless a later approved design makes one durable.
- Add later variants only with their owning block; do not front-load an abstract mega-framework.

## GATHER — approved concrete solution

Reuse already present code: `NpcActivity::Gather`, `TerrainResource::{Fruit,Vegetable,Mushroom,Plant}`;
`Block::is_collectible()`/`into_collected()`; `ControlAction::Collect`; and the server interaction/inventory path.
The loaded Gather action is currently a placeholder and must call the shared collection helper.

1. Append `DesignationKind::Gather` last and give it `FootprintMode::Area2D`.
2. Scan only the painted footprint and filter by collectible metadata plus the approved resource allowlist.
3. Emit one deduplicated `JobKind::Gather` per target.
4. Approach through the shared standable-work-position solver.
5. Execute via `ControlAction::Collect`; never delete the sprite or mint replacement food in `bastion_jobs`.
6. Let authoritative interaction create the inventory item, then use ordinary Haul/deposit.
7. Full inventory, vanished target, cancellation, or inaccessible target releases/defer cleanly.
8. Gate: multiple resources; inside-area only; race for one target; full inventory; cancel; exact
   item→inventory→stockpile conservation; deterministic repeat; vanilla unaffected.

## Per-block coding guide

### Safety, logistics, and autonomy

| Block | Preferred implementation | Primary gate / do not |
|---|---|---|
| FR15-TIGHTDIG | Centralize stance choice in `find_standable_work_pos(target, verb)`; instrument failure/progress before changing movement. | Every reachable tight target is worked; do not add another velocity shove. |
| LOD-0 | Make rtsim `Npc.bastion_colonist` persistent truth; change-track ECS mutations and flush before demotion/save. | XP and exact inventory survive promote cycles with no dupe. |
| LOD-1 | One atomic mode-transition API releases claims then flips ownership; loaded systems gate on `SimulationMode::Loaded`. | No NPC is processed by both tiers in one tick. |
| B6-HAUL + JOB-CORE | Use typed jobs/reservations; generate from loose `PickupItem`; execute through existing pickup/inventory/drop events. | Two jobs cannot spend one item; never maintain parallel stock totals. |
| B-AG1 | On promotion, continue current rtsim intent through `RtSimController`/existing action nodes; missing targets replan. | Trader/guard/hunter continues rather than idling. |
| B-AG5-CORE | Small shared helpers—approach, collect, interact, work-at, deposit—called by jobs and rtsim. | Gather proves both caller paths share an action. |
| ZONE-0 | Persist `ZoneRegistry { ZoneId → Zone }`; soft magnet returns a score modifier, never direct movement. | Attraction affects choice statistically without overriding urgent work. |
| GATHER | Implement the approved Collect-path solution above. | Exact conservation; no direct terrain deletion. |
| HIST-0 | Append-only versioned `ChronicleEvent { id,time,kind,actors,place,cause,importance }`; one idempotent `record()`. | Save/reload preserves IDs/order; retry records once. |
| B-AG2 | RON scoring weights/allowed activities keyed by archetype; one shared brain. | Identical state yields data-driven differences without AI forks. |
| B-AG3 | Persist traits/values/memories; one mind-event API derives short-lived emotions/mood. | Trait-sensitive deterministic reaction; no per-system thought writers. |
| SEASON-0..2 | Pure derived `Season/year_phase` over game time; consumers own effects. | Pause/speed/save never drift the calendar. |
| FOCUS-0 | Lock `NeedKind`; use a serde-defaulted collection rather than endlessly adding fixed fields. | Old saves default safely; every venue uses the same enum. |
| B7 | Sequential game-time needs system; threshold crossings wake arbiter; eating consumes normal consumables. | Food stabilizes; no food declines predictably; no wall-time dependence. |
| PATH-0 | Entity-ID-ordered sequential path-request queue with global iteration budget. | Cap always holds and repeat runs match. |
| FARM / PROD-2 | Plot zone plus sprite `Growth` state/scheduled terrain changes; phase jobs Till/Sow/Tend/Harvest. | Seed/crop accounting conserved; renewable output. |
| RUN-0..2 | Urgency token raises `Goto` speed; drain existing Energy and apply bounded Winded state. | Low energy forces walk; normal work never sprints. |
| AUTON-0 | Transient loaded arbiter runs before jobs and is sole colonist activity writer. | Flee preempts Work and jobs cannot overwrite it that tick. |
| AUTON-1 | Independent bounded/dedup job generators for Mine/Gather/Farm/Haul/Build. | Empty board self-populates without unbounded growth. |
| AUTON-2 | Nonlinear need urgency changes job scoring/labor allocation, never direct role assignment. | Recoverable shortage recovers; terminal shortage degrades without deadlock. |
| AUTON-3 | Traits/policy are bounded score multipliers; cache latest explanation for inspector. | Policy tilts; it never commands a unit. |
| B8 | Promote rtsim bandits; reuse combat AI; one threat assessment emits Muster/Fight/Flee and rally policy. | Zero-input response returns to work after threat. |

### Hazards, hand, UI, and persistence

| Block | Preferred implementation | Primary gate / do not |
|---|---|---|
| HAZ-0..3 | `HazardEvent` façade over RadiusEffect/terrain/damage/Outcome plus `Cause` and mind adapter. | One event produces physical and correctly attributed mind effects. |
| HIST-1..2 | Subscribe existing Outcome/rtsim Report/server events; UI queries the same Chronicle store. | Feed and saved history share one event ID. |
| GH-A | Standalone hand body/figure anchored to `bastion_point_under_cursor`; v1 selection updates existing Selected line. | Render/pan/point/select without favor dependency. |
| B13-W0 / POWER-0 | Data `PowerDef` plus one dispatcher: validate target → spend favor → existing operation → Outcome/Chronicle. | Invalid/zero-favor cast has no partial effect/spend. |
| GH-B | Existing Link/mount ownership to hand anchor; existing throw/velocity; Hazard handles landing. | Exactly one owner/controller; no duplicate/stranded entity. |
| B13-W1 / POWER-1 | Adapters to Lightning, Buff, WeatherGrid, Spawn/MakeSprite, authoritative terrain events—not command strings. | Every power uses one validation/favor pipeline. |
| GH-C | Gesture presents validated cast state; server acknowledgement owns completion. | Cancelled animation cannot duplicate a cast. |
| GH-D | One bounded alignment scalar derived from attributed Chronicle deeds; appearance is pure interpolation. | Drift reverses predictably; no second morality state. |
| B9 / UI-PLATFORMS | Compact read-only server snapshots; typed UI requests; extend Bastion HUD/overlay toolkit. | UI cannot mutate local truth or become a second state owner. |
| B-AG4 / UI-4 | One tabbed inspector keyed by selected ID; event box/history query HIST. | Rapid selection never mixes identities. |
| OBJ-0..2 | Server query returns typed inspect result for entity/block/zone; context actions map to typed requests. | Stale target says gone; no reused-ID action. |
| B10 | Persist durable colony data; rebuild claims/caches/path/UI state. | Mid-job save resumes without stale claims or dupe. |
| LOD-2 | Separate coarse `AbstractColony` conservative debit/credit tick; never run loaded arbiter offscreen. | Unwatched progress is balanced and bounded. |
| LOD-3 | Single additive stock authority across partial load plus deterministic reconciliation. | Load/unload/partial-load no dupe/loss/desync. |
| WORLDGEN-TUNE | RON-configurable density first; deterministic starter veins; Bastion flag for near-site caves. | Same seed identical; previewed differences real. |
| B11 | Reuse map/site data; persisted `FoundingScenario`; existing chunk generation + colony spawn. | Preview equals spawned biome/resources/threats. |
| B12 | Adapt `PresenceKind::Possessor`; atomic release-job/disable-AI/controller handoff and reverse. | Exactly one controller, including death/switch. |
| PLAYER-MODES | One `PlayerFace` drives camera/input/HUD capability; never three player entities. | Atomic switch preserves selection/identity. |
| GOD-DOMAIN | Persist invoked domain separately from deed-derived face; domain filters catalog. | Domain choice cannot set alignment. |
| GH-E | Hand action delegates entirely to B12 transition. | No duplicate possession path. |
| B13-W2 / POWER-2 | Region-buff/panic/ward/farm adapters change conditions read by autonomy. | No power writes jobs/controllers directly. |

### Construction, production, and society

| Block | Preferred implementation | Primary gate / do not |
|---|---|---|
| DIG-0..4 | One reachability-safe volume decomposer emits ordered job DAG; each verb is a template. | Every next job has a stance and return path. |
| BUILD-0..3 | `BuildPlan = template + transform + bill + ordered cells`; reuse `StructuresGroup`; same DAG for prep/materialization. | Cannot seal workers or spend unreserved material. |
| BUILD-4..6 | Generate immutable deterministic plans before work; autonomy chooses validated plans. | Same inputs yield same plan/bill. |
| PROD-0 | Wrap authoritative recipe/crafting operation in Produce job; reserve inputs, consume once at completion. | Cancel releases; complete consumes once. |
| PROD-1 + POL-0 | One census/quota evaluator: target − available − reserved; depth-cap dependency propagation. | Cycles terminate; zero-policy healthy. |
| PROD-3 / COOK | Ordinary Produce plus crop permission; reserve/protect seed items explicitly. | Cannot cook final forbidden seeds. |
| QUAL-0..1 | Per-instance craft quality persisted and valued; stamp only at successful craft. | Stack merge cannot erase/upgrade quality. |
| TOOL-1..2 | One `tool_factor` from ToolKind/material/quality; data gates; work systems call it. | Best valid tool auto-equips; no verb-specific ladders. |
| PROD-4..5 | UI derives from census/order state; animation follows server work phase. | Display totals equal physical/reserved totals. |
| QUAL-2..3 | Strange mood is bounded state machine using normal claims/reservations/production. | Exactly one artifact or explicit tragedy exit. |
| ROT-0..3 | Freshness belongs to stack identity/buckets; transform food→spoiled→rotten→inert. | Quantity conserved; merging never refreshes food. |
| ZONE-1-REST | Refuse is a filter on normal Haul destination choice. | Refuse/stockpile cannot reserve same item. |
| POL-1..4 | Data `PolicyRule` evaluators return score/generator parameters. | Disabled/unknown rule harmless; no parallel policy engine. |
| BURROW-0..3 | Filter job eligibility/idle destinations; critical survival bypasses before escape selection. | Burrow alone cannot starve a colonist. |
| ROOM-0..3 | Event-driven dirty-region enclosure flood-fill and cached room IDs. | Opening wall invalidates only affected graph. |
| FOCUS-1..3 | Venue-seeking drives through `satisfies(NeedKind)`; Focus derived into work/quality. | Missing venue causes bounded distress, not spam. |
| REL-0..3 | Temple is Zone/Room; worship is Focus need; priests/congregations use jobs/roles; bounded devotion. | Empty/unavailable service degrades safely. |
| DF-TAVERN | SPEC gate; preferred v1 is generic Venue plus shared converge/use/disperse satisfying Drink/Socialize. | Religion/Festival reuse same gather loop. |
| FEST-0..2 | Scheduled event creates temporary venue demand/feast consumption; autonomous opt-in. | Event releases workers and ends cleanly. |
| STOCK-0..3 | Reuse tame/pet/stay; pasture capacity feeds bounded logistic herd; products are typed jobs/recipes. | Bounded population and conserved outputs. |
| TRADE-0..6 | Live colony economy compatible with SitePrices/Good concepts; do not mutate frozen worldgen sites; caravan LOD. | Loaded/abstract settlement balances match. |
| MIG-0..3 | Drive existing `wanted_population`; admission uses B3; emigration demotes, never deletes. | Growth and threat pressure both enabled/bounded. |
| JUST-0..3 | Data positions/mandates; existing ReportKind/Chronicle starts crime; responses are jobs/policies. | God influences but never runs court. |
| B-AG6 | Aggregate rtsim population offscreen; individual genealogy only for tracked/notable lines. | No extinction or exponential runaway. |
| PATH-1..3 | Per-chunk clusters/portals, shared goal fields, block-change dirty invalidation. | Near-linear scaling; vertical links preserved. |

### Deep and late systems

| Block | Preferred implementation | Primary gate / do not |
|---|---|---|
| CAVERN/GEOLOGY | Reuse shipped cave layers/resources; separate colony knowledge from world truth; breach emits Hazard. | Undiscovered ore hidden; breach bounded. |
| LIGHT-0..3 | Server `lit_at()` spatial index plus daylight/existing lanterns; not renderer GI. | Bounded query; server is gameplay truth. |
| UNDERGROUND-UX | Feed real selected/work targets to occlusion; hand light is ordinary source at cursor world point. | Camera never changes simulation visibility. |
| STRUCT-1 | Supports modify cave-in trigger heuristic; no structural-physics solver. | Supported span suppresses trigger deterministically. |
| MECH-0..3 | Persist nodes/abstract links; triggers enqueue effects; operables scheduled changes; traps HazardEvents. | Edge/cooldown prevents repeat fire; friend/foe honored. |
| POW-0..1 | Dirty connected-component supply/demand graph; wind reads WeatherGrid. | Disconnected demand receives no invented power. |
| FLUID-0..3 | Sparse bounded active cells, double buffer, fixed neighbor order/budget; flag-off static; over-budget safe fallback. | Conservation/determinism/fallback over visual fidelity. |
| TEMP-0..2 | Pure felt-temperature over world temp/season/depth/exposure/heat; apply existing buffs. | Every danger has a colony answer. |
| SYN-0..3 | RON syndromes compose buffs; budgeted incubation/transmission/recovery/immunity. | Bounded transmission; LOD rates align. |
| DF-WOUND / MEDICAL | SPEC gate; recommended v1 wound records + existing Bleeding/Crippled/Infection buffs, not organ physics. | Treatable/self-resolving bounded states. |
| DF-MILITARY / RANGED | SPEC gate; reuse combat AI/equipment/rally policy; squads are policy groups. | No per-unit attack-move. |
| NIGHT-0..2 | Bounded director reads DayPeriod/light/prestige/threat budget; existing creature AI. | Lit/sheltered tended colony survives untouched. |
| VILLAIN-0..2 | Shared named-figure promotion: rtsim NPC + Chronicle identity/grudge/bounded escalation. | Escapes/escalation capped; closure possible. |
| MISS-0..2 | Adapt rtsim Quests: create, demote participants, resolve, promote survivors/loot. | No actor/equipment duplication. |
| REP-0..2 | Shared Chronicle-standing library with facets/decay; dedup by event ID. | Deeds apply once and redemption can outweigh them. |
| AGENT-CULTURE | Culture-keyed data tables over shared mind/standing plus bounded relations matrix. | Missing data falls back safely. |
| HIST-3..6 | Index/query one Chronicle by actor/site/time/importance; compact world summaries. | Browser never mutates history. |
| ART-0..2 | Art item references Chronicle event + description seed; existing production/quality creates it. | Missing event has stable fallback description. |
| KNOW-0..2 | Persist knowledge graph; recipe eligibility checks it; discovery records events. | Loss recoverable; existing items remain valid. |
| HALLOW-0..2 | Place-keyed caller of shared standing library plus spatial aura. | Bounded/decaying/reversible significance. |
| EPITHET-0..2 | God-keyed standing caller mapped to culture names. | Alignment remains source of morality. |
| OMEN-0..2 | Store omen fact then derived interpretation from culture/faith/epithet. | Player cannot dictate interpretation. |
| CHAMP-0..2 | Named-figure promotion with Champion role and earned-retention evaluator. | Bounded anointments; fall path real. |
| DEAD-0..2 | Compact notable ancestor records; graves/sites reference them; restless uses existing body/VFX. | Dead actors never run living systems. |
| CURSE-0..2 | Syndrome with divine Cause and explicit lift predicate; geas is event/policy trigger. | Every curse liftable unless explicitly approved permanent. |
| BEAST-0..2 | Named-figure promotion over shipped megafauna plus lair/threat/mission reward. | Bounded regional apex count. |
| RENOWN-0..2 | Colony-keyed shared standing caller with derived cultural byname. | Modifier, never a forced response. |
| RECLAIM-0..2 | Atomic colony-end → compact ruin-site retaining Chronicle/site/object refs. | Original colony stops simulating after conversion. |
| GH-F | Thin adapters to object/Chronicle/build/mind systems already built. | No hand-only duplicate subsystem. |
| B13-W3 / POWER-3 / REL-4 | Register late adapters in existing cast pipeline. | Adapter unavailable until owning engine exists. |
| DP1..5 | rtsim faction interest/faith state; powers bias inputs only. | Stable zero-god equilibrium and bounded rivals. |
| NAVAL-0..3 | Adapt airship state-machine shape to water; reuse ship body/buoyancy; mirror abstract route actor. | Promote/demote preserves cargo/crew exactly. |
| DF-PUMP / MAGMA / HYDRO | SPEC gate; all are Fluid sources/sinks/material adapters, never separate solvers. | Same conservation/budget/fallback law. |
| DF-ECON / GUILD | SPEC gate; extend census/prices/roles, never parallel economy. | Zero-property/guild remains complete game. |
| DF-MINECART | SPEC gate; route is logistics graph, cart is vehicle/capacity multiplier. | No second inventory/path authority. |
| DF-PREF | SPEC gate; preference data feeds existing mind/quality/venue scoring. | No bespoke behavior trees. |
| DF-BOOKS / DF-NOTES | Notes are persisted annotations; accounting accuracy modifies census read-model only. | Neither becomes stock authority. |
| AUTON-FULL-SOAK | Compose fixtures into long deterministic matrix with event assertions/checksums. | Zero-input colony lives; no stall/dupe/loss; bounded ticks. |

## Builder packet rule

The architect copies only the current row’s relevant material into [BASTION_CURRENT_BLOCK.md](BASTION_CURRENT_BLOCK.md):
outcome, reason-next, design sections, current symbols, approved solution, reused systems, new code, shared owners,
do-not list, LOD/persistence, deterministic scenario, invariants, inherited gates, review tier, tag/rollback, and
next block for context only.
