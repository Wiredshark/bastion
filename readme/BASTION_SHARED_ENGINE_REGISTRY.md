# Project Bastion — Shared-Engine Registry

> Ownership contract for the [Master Build List](BASTION_MASTER_BUILD_LIST.md) and
> [Implementation Playbook](BASTION_IMPLEMENTATION_PLAYBOOK.md). A consumer extends its owner; it never creates
> a competing engine.

## 1. Typed jobs and reservations

- **Owner block:** B6-HAUL + JOB-CORE (#34)
- **Defining modules:** `common/src/bastion.rs`, `server/src/bastion_jobs.rs`
- **Shape:** serialized `DesignationKind` remains player intent; server `JobKind` is executable work; one
  `ReservationTable` owns items, stations, and destinations.
- **Invariants:** one claimant/job; one reservation/resource; physical item conservation; transient claims rebuilt.
- **Consumers:** Gather, Farm, Produce, Cook, Eat/Sleep, Worship, Medical, Missions, autonomous generators.
- **Extension rule:** add a variant and executor with the owning feature block; reuse claim/travel/progress/release.
- **Forbidden fork:** feature-specific job boards, private reservations, or using fake designations for autonomous work.

## 2. Zones and spatial policy

- **Owner block:** ZONE-0 (#37)
- **Defining modules:** canonical Bastion common data plus server `ZoneRegistry`; overlay via existing Bastion HUD.
- **Shape:** `ZoneId`, locked `ZoneKind`, persisted Region/extent/enabled state; one soft-magnet score mechanism.
- **Invariants:** soft zones bias but do not command; hard Burrows remain a separate policy consumer; IDs stable.
- **Consumers:** Gather, Refuse, Farm, Pasture, Temple, Tavern, TradeDepot, Infirmary, Rally, shelter.
- **Extension rule:** add thin per-kind behavior in the owning system and render through existing overlay layers.
- **Forbidden fork:** per-feature area registries or movement written directly by a zone.

## 3. Need vocabulary

- **Owner block:** FOCUS-0 (#43), co-locked with B7 (#44)
- **Defining modules:** Bastion persisted colonist/needs data.
- **Shape:** one `NeedKind` vocabulary and serde-defaulted state collection.
- **Invariants:** old saves default safely; missing venue causes bounded distress; critical survival preempts work.
- **Consumers:** hunger/rest/recreation, Focus, Religion, Tavern, Festival, climate comfort, Emergency Run.
- **Extension rule:** new venue declares `satisfies(NeedKind)` and exposes ordinary jobs/activities.
- **Forbidden fork:** adding independent worship/drink/socialize meters outside the shared model.

## 4. Order evaluator and stock census

- **Owner block:** PROD-1 + POL-0 (#78)
- **Defining modules:** production/policy server resources over physical piles/inventories/reservations.
- **Shape:** `deficit = target - available - reserved`; dependency expansion is depth-capped and deduplicated.
- **Invariants:** zero-policy healthy; recipe cycles terminate; counts are derived; reservations counted once.
- **Consumers:** production chains, Cook permissions, tool quotas, trade surplus, auto-refuse, dashboards.
- **Extension rule:** rules return desired deficits or bounded score/generator parameters.
- **Forbidden fork:** a production-only census or second mutable inventory/economy count.

## 5. Reachability-safe volume DAG

- **Owner block:** DIG-0..4 (#74), shared with BUILD-0..6 (#75–76)
- **Defining modules:** server-side Bastion planning library; authoritative terrain events perform cells.
- **Shape:** immutable volume/template → ordered job DAG with standable stance and preserved return route.
- **Invariants:** access leads work; no emitted next job entombs a worker; deterministic ordering.
- **Consumers:** channel/ramp/stair/shaft, vertical building, site prep, composed templates, future volume work.
- **Extension rule:** new verb supplies geometry/template and material rule, not a new decomposer.
- **Forbidden fork:** deepest-first voxel loops, direct bulk terrain mutation, or independent build-up ordering.

## 6. Chronicle capture API

- **Owner block:** HIST-0 (#39)
- **Defining modules:** persisted Chronicle store plus one `record(ChronicleEvent)` API.
- **Shape:** versioned event ID/time/kind/actors/place/cause/importance; append-only; indexed read models later.
- **Invariants:** idempotent event identity; chronological persistence; browser/feed query same store.
- **Consumers:** work/production, hazard, death, faith, trade, quality/artifact, divine acts, missions, nature.
- **Extension rule:** register an event kind/emitter; never maintain another historical log.
- **Forbidden fork:** UI-only announcements as history, per-system permanent logs, mutable past events.

## 7. HazardEvent and attribution

- **Owner block:** HAZ-0..3 (#53)
- **Defining modules:** server façade over existing radius effects, terrain events, damage, Outcome, and mind events.
- **Shape:** `HazardEvent { pos, radius, kind, cause }` with deterministic bounded application.
- **Invariants:** physical effect and mind attribution agree; one event; bounded radius/work.
- **Consumers:** cave-in/breach, traps, fluid flood, fire, timber/rockfall, god wrath.
- **Extension rule:** add a Hazard kind/adapter over existing effect operation.
- **Forbidden fork:** private trap/flood damage engines or un-attributed mind writes.

## 8. Chronicle-derived standing

- **Owner block:** REP-0..2 (#113)
- **Defining modules:** one bounded/decaying standing library consuming Chronicle event IDs.
- **Shape:** keyed subject plus facets, decay, band/name derivation, and last-applied event identity.
- **Invariants:** each deed applies once; standing is reversible/outweighable; deterministic bounded cache.
- **Consumers:** Reputation (person), Epithet (god), Sacred Sites (place), Renown (colony).
- **Extension rule:** supply subject key, event→facet mapping, and presentation bands.
- **Forbidden fork:** four independent reputation accumulators or direct morality mutation.

## 9. Named persistent figures

- **Owner block:** VILLAIN-0..2 (#111)
- **Defining modules:** rtsim NPC promotion/identity plus Chronicle record and bounded arc state.
- **Shape:** promote existing actor → stable name/role/legend/source/rise-fall-resolution data.
- **Invariants:** bounded population and escalation; promote/demote preserves one actor; closure possible.
- **Consumers:** Villain, Divine Champion, Legendary Beast.
- **Extension rule:** add role-specific scoring/arc over the same actor identity.
- **Forbidden fork:** special boss entities outside rtsim identity or duplicate “named actor” stores.

## 10. Bounded population model

- **Owner block:** STOCK-0..3 (#93), generalized by B-AG6 (#97)
- **Defining modules:** rtsim aggregate population/capacity/pressure model.
- **Shape:** carrying capacity + bounded growth + mortality/emigration/slaughter pressure; individual detail only loaded/notable.
- **Invariants:** no extinction-to-zero without real pressure; no exponential runaway; LOD rates correspond.
- **Consumers:** livestock, wildlife, colony growth, migration inputs.
- **Extension rule:** provide capacity and pressure sources; reuse equilibrium tick.
- **Forbidden fork:** spawn timers as populations or loaded-only breeding for world-scale groups.

## 11. God-power cast pipeline

- **Owner block:** B13-W0 / POWER-0 (#56)
- **Defining modules:** common `PowerDef`, server `ApplyInfluence` dispatcher, Bastion HUD/hand targeting.
- **Shape:** validate target → validate/spend favor → authoritative operation → Outcome/Chronicle attribution.
- **Invariants:** failure has no spend/effect; one cast identity; zero favor leaves colony fully playable.
- **Consumers:** all power waves, hand physical verbs, Religion divine seam, Divine Politics rivals.
- **Extension rule:** register a data definition and adapter to an existing owning engine.
- **Forbidden fork:** chat-command invocation, direct job/controller writes, feature-private favor balances.

## 12. Loaded↔abstract transition protocol

- **Owner blocks:** LOD-0/1 (#32–33), completed by LOD-2/3 (#65–66)
- **Defining modules:** `rtsim/src/data/npc.rs`, `server/src/rtsim/tick.rs`, unload/demotion hook.
- **Shape:** rtsim NPC/Bastion record is durable truth; loaded ECS syncs changes; atomic mode transition; abstract aggregate tick.
- **Invariants:** exactly one simulation tier per actor/tick; complete save-back; single stock authority; conservative reconcile.
- **Consumers:** every persistent agent/system, missions, caravans/naval, growth, threats, ruins.
- **Extension rule:** declare durable fields, loaded behavior, abstract approximation, and transition handling in the owning block.
- **Forbidden fork:** parallel loaded/abstract copies, persisted transient claims, or running per-agent arbiter offscreen.

## 13. Pathfinding scheduler and hierarchy

- **Owner blocks:** PATH-0 (#45), PATH-1..3 (#98)
- **Defining modules:** sequential request scheduler over existing A*/chaser; cluster/portal/field caches later.
- **Invariants:** global budget; deterministic request order; every reachable goal remains reachable; block changes invalidate safely.
- **Consumers:** every movement job, hauling, shared depots, muster/shelter, caravans, build/dig churn.
- **Extension rule:** enqueue/consume through scheduler; shared-goal users request fields.
- **Forbidden fork:** polling shared budgets inside parallel agent joins or feature-specific long-range path engines.

## 14. Fluid solver

- **Owner block:** FLUID-0..3 (#105)
- **Defining modules:** flagged bounded sparse active-region solver over static water baseline.
- **Invariants:** conservation, fixed neighbor order, deterministic budget, safe static fallback, flag-off untouched.
- **Consumers:** pumps, magma, hydro/aquifer/flood, irrigation, floodgates, divine terraform.
- **Extension rule:** add material/source/sink/boundary adapters to the same solver.
- **Forbidden fork:** independent magma/flood/pump solvers or world-wide unbounded updates.

## 15. Document navigation and change discipline

- Current action: [Current Block](BASTION_CURRENT_BLOCK.md)
- Sequence: [Master Build List](BASTION_MASTER_BUILD_LIST.md)
- References: [Master Design Index](BASTION_MASTER_DESIGN_INDEX.md)
- Coding: [Implementation Playbook](BASTION_IMPLEMENTATION_PLAYBOOK.md)
- Any new shared engine must first name an owner block, consumers, invariants, extension rule, and forbidden fork here.
