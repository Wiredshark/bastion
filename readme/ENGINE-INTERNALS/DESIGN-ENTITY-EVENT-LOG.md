# DESIGN â€” **THE ENTITY EVENT LOG** (DECISIONS #99)

**Ben:** *"we essentially need all items and NPCs etc to have a log."*
**Commissioned to the reviewer for design; 5b builds when the wave clears.**

> **NAMING (2026-08-10, renamed from "promotion" — Opus ruling, Fable doc sync):** the
> keep-vs-ring-evict concept is **RETAINED / RETENTION** everywhere. "Promoted" is
> ALREADY rtsim's word for simulated-unloaded → live ECS entity (`common/src/bastion.rs`
> ~1441, `BastionColonist`'s own doc) — a different axis entirely; an event line reading
> "colonist promoted" would be read as a LOAD EVENT by anyone who knows rtsim. RETAINED
> names the mechanism (kept vs evicted), so it cannot drift back toward the colliding
> "became important" sense. Code renamed at `b60fe1737a`; this doc synced same day.

## â˜…â˜…â˜…â˜…â˜…â˜…â˜… WHY â€” **TRACEABILITY AS A PROPERTY OF THE WORLD**

**Every investigation this week succeeded exactly when an entity's history was
traceable and stalled when it was not:**

| investigation | outcome | why |
|---|---|---|
| the food thief | **2 runs + an instrument build** | *every pickup trace was gated on `bastion_piles`; a `persistent:false` drop was structurally invisible* |
| the sit trap | â˜… **one read** | *the fail-safe warn happened to carry `character_state`* |
| the reservation famine | **one read, after the fact** | *`already_on_need_job` counts existed by luck of an unrelated diag* |

> ## â˜…â˜…â˜…â˜…â˜… **THE DIFFERENCE WAS NEVER THE DIFFICULTY OF THE QUESTION. IT WAS
> WHETHER SOMEONE HAD REMEMBERED TO ENABLE THE RIGHT INSTRUMENT BEFOREHAND.**

â˜…â˜…â˜… **This makes history a property of the WORLD rather than of whichever flag was
set.** **Dual payoff, and the second is why this is rare infrastructure:**

- â˜…â˜… **Any future Voonoo is ONE QUERY instead of a two-run instrument build.**
- â˜…â˜…â˜… **Item provenance and NPC biographies â€” *"forged by X, stolen by Y, carried at
  the siege of Z"* â€” are Dwarf-Fortress-grade CONTENT that falls out of the same
  stream.** *Infrastructure and game feature from one build.*

---

## â˜…â˜…â˜…â˜…â˜… PRIOR ART â€” **SURVEYED BEFORE DESIGNING** (standing rule)

### RimWorld â€” `TaleDef` / the Tale system

**Typed event definitions, each carrying its own narrative rule-pack.** *Tales
record bills completed, marriages, downings, raids; they are organised into
categories (Combat, General) and rendered through `ruleStrings` with parameter
substitution (`[pawn_nameShortDef]`, `[circumstance_group]`).* â˜…â˜…â˜…â˜… **The player
-facing artefact â€” a sculpture's description â€” is a PROJECTION over typed events,
not a separate record.**

â˜…â˜… **TAKE:** *typed events with rendering attached; the narrative view as a
projection.* â˜… **LEAVE:** *social tale-spreading between pawns â€” charming, out of
scope.*

### Dwarf Fortress â€” legends / historical figures / artifacts

**Artifact creation, ownership and transfer are recorded and browsable; artifacts
move between civilisations by trade, theft, or war.** *Legends mode is a VIEW with
export, over a history generated during worldgen.*

â˜…â˜…â˜…â˜…â˜… **AND THE NUMBER THAT SHAPES OUR DESIGN: a 1000-year world can export a
~1 GB history dump.** **DF can afford that because its history is generated
OFFLINE, once. Ours accrues LIVE, in a running sim, under a diag-density budget.**

> ## â˜…â˜…â˜…â˜… **SO WE TAKE DF's AMBITION AND REJECT ITS VOLUME PROFILE. Bounded
> retention is not a nice-to-have; it is the difference between this shipping and
> this being a memory leak with a story attached.**

---

## â˜…â˜…â˜…â˜…â˜…â˜…â˜… THE DESIGN

### 1 Â· ONE STREAM, UID-INDEXED, EVENT-DRIVEN

    EntityEvent { tick, subject: Uid, kind: EventKind, actor: Option<Uid>, data }

â˜…â˜…â˜…â˜… **Append-only, global, indexed by subject uid. EVENT-DRIVEN ONLY â€” never a
per-tick sweep.** *The density budget is non-negotiable
([[the-instrument-changes-what-it-sees]]: two per-cell diag reads once broke
bit-reproducibility).*

â˜…â˜… **`actor` is the second uid â€” the picker, the claimant, the killer.** *It is what
makes "stolen BY Y" expressible and what would have named Voonoo in run one.*

### 2 Â· VOCABULARY PER CLASS â€” **typed, closed sets**

    ITEM       Created Â· Dropped{by} Â· PickedUp{by} Â· Reserved{by} Â· Released{reason}
               Â· Consumed{by} Â· Despawned{cause} Â· Split Â· Merged
    COLONIST   Claimed{job} Â· Released{job,reason} Â· Preempted{need} Â· Teleported{cause}
               Â· NeedCrossed{need,dir} Â· Ate{item} Â· Stuck{cause}
    AMBIENT    SAMPLED â€” arrival/departure/interaction-with-colony only

â˜…â˜…â˜… **Closed sets, not free strings.** *A closed vocabulary is greppable, countable,
and cannot drift into prose â€” and it is what lets the projection render without a
parser.*

### 3 Â· RETENTION â€” **RETENTION, NOT UNIFORM SAMPLING**

> ## â˜…â˜…â˜…â˜…â˜… **THE VOLUME PROBLEM IS THE DESIGN. Both prior-art systems solve it by
> SIGNIFICANCE, and so should we.**

    UNRETAINED entities  ->  ring buffer, bounded, oldest-out
    RETAINED entities    ->  retained for the world's life

**An entity is RETAINED when it acquires significance:** *a colonist is named Â· an item
is crafted, gifted, or player-touched Â· anything a chronicle entry references Â·
anything involved in a death.* â˜…â˜…â˜… **Retention is one-way and cheap: a flag plus a
move from ring to permanent.**

â˜…â˜…â˜…â˜… **This is exactly how DF keeps historical figures forever without keeping every
peasant, and how RimWorld culls by interest.** **It also matches the payoff: an
artifact's provenance IS the retained set; a mushroom's is not.**

â˜…â˜… **Ring size and retention rules are TUNABLE and must be measured, not guessed â€”
the first fan after this lands reports stream bytes per sim-hour.**

### 4 Â· UNIFICATION â€” **EXISTING MACHINERY BECOMES PRODUCERS**

### â˜…â˜…â˜…â˜…â˜…â˜…â˜… CHASSIS â€” **VERIFIED, AND IT IS NOT WHAT WE ASSUMED**

**`bastion_flight_recorder.rs` read at `48886adfa0`. It was proposed as "the
uid-sampling ring-buffer pattern". It is not a ring, and it is not multi-entity.**

    fn record_event(&mut self, event) {
        if self.config.uid_filter.is_some_and(|uid| uid != event.uid) { return Ok(()); }  // ONE uid
        if self.event_count >= self.config.max_events {
            self.truncated_events = true;                                                  // FLAGS it
            return Ok(());                                                                 // DROPS NEWEST
        }
        serde_json::to_writer(&mut self.events, &event)?;                                  // JSONL to disk
    }

â˜…â˜…â˜…â˜… **THREE CORRECTIONS to this design's first draft:**

| assumed | actual |
|---|---|
| ring buffer, oldest-out | â˜…â˜…â˜… **cap-and-DROP-NEWEST, JSONL to disk** |
| uid-indexed (many) | â˜…â˜…â˜… **`uid_filter: Option<u64>` â€” ONE entity, focused** |
| in-memory sampling ring | **file writer with `BufWriter`** |

### â˜…â˜…â˜…â˜…â˜… WHAT IS GENUINELY REUSABLE

- â˜…â˜…â˜…â˜…â˜… **THE TRUNCATION FLAG.** *`truncated_events = true` is surfaced in the
  summary â€” **the recorder knows when it dropped data and says so***. **That is the
  self-accounting law already implemented; the event log MUST carry it forward.**
- â˜…â˜…â˜… **Disabled-unless-env-set, documented as not initialising, not doing I/O, not
  mutating ECS, and not altering scheduling when off.** *That is the determinism
  posture we need, already written and already reviewed.*
- â˜…â˜… **Versioned JSONL schema strings** (`bastion.flight-recorder.summary/v1`) and
  the config-from-env pattern.

### â˜…â˜…â˜…â˜… WHAT MUST BE BUILT, NOT REUSED

â˜…â˜…â˜… **Multi-entity indexing Â· per-entity ring (oldest-out) Â· the retention
mechanism.** *None exist.*

> â˜…â˜…â˜…â˜…â˜… **AND DROP-NEWEST IS THE WRONG POLICY HERE, WHICH IS THE USEFUL FINDING.**
> *It preserves a run's BEGINNING â€” right for a startup bug, wrong for "what
> happened to this item".* **Worse, one flooding entity would starve every other
> entity's history under a global cap.**

â˜…â˜…â˜…â˜… **So the ring must be PER-ENTITY, not global** â€” *each entity keeps its own
recent history and cannot be crowded out.* **That is a stronger design than the
first draft, and it came from reading the chassis instead of trusting its
description.**

### 4b Â· PRODUCERS

â˜…â˜…â˜… **`record_pickup_verdict` becomes an event producer**, not a parallel log.
â˜…â˜…â˜… **The chronicle becomes a PROJECTION over the stream** â€” *`OnDeath` / `OnTheft`
already ARE entity events; the design should make that literal rather than
analogous.* â˜…â˜… **The flight recorder's uid-sampling ring is the likely chassis** â€”
*reuse it; do not build a second ring.*

> â˜…â˜…â˜…â˜… **NOTHING IS DUPLICATED. Every existing trace either becomes a producer or is
> deleted in favour of one.** *Two logs of the same fact is how the pickup trace and
> the delete trace disagreed for two days.*

### 5 Â· DETERMINISM â€” **SCREENED BEFORE ANYTHING DEFAULTS ON**

â˜…â˜…â˜…â˜… **Per the instrument law: a paired floor run, fingerprint must hold, BEFORE any
producer is on by default.** â˜…â˜… **Event-driven emission with no collection walks
should be free â€” but "should be" is what the floor run exists to reject.**

â˜… **And the emission must not read state it changes:** *capture values BEFORE the
mutation the event describes* â€” **three separate instruments got that wrong this
week.**

---

## â˜…â˜…â˜… PILOT SCOPE â€” **ITEMS + COLONISTS**

**Not ambient NPCs, not sites, not combat.** *Items and colonists are where both
payoffs land first: the thief query and the provenance feature.*

**ACCEPTANCE:**
- â˜…â˜…â˜…â˜… **The Voonoo query:** *given a drop's uid, return every actor that touched it,
  in order.* **That question cost two runs and an instrument build; it must become
  one query over a run's stream.**
- â˜…â˜…â˜… **Provenance render:** *one item's history rendered as a sentence, proving the
  projection works end-to-end.*
- â˜…â˜… **Determinism floor: paired run, fingerprint holds.**
- â˜…â˜… **Volume: bytes per sim-hour reported, with the ring at its default.** *An
  unmeasured retention policy is the 1 GB failure mode.*

## â˜…â˜…â˜…â˜…â˜…â˜…â˜… FORKS RULED â€” **DECISIONS #100**

### 1 Â· PERSISTENCE **SPLITS ALONG the retention BOUNDARY**

    RETAINED    ->  persists across save/load, as a save component WITH version migration
    UNRETAINED  ->  session-scoped, dies with the process

> ## â˜…â˜…â˜…â˜…â˜… **THE SPLIT MAKES STORAGE CLEAN BY CONSTRUCTION: saves carry only what
> retention already bounds, and the ring never touches saves at all.**

â˜…â˜…â˜…â˜… **Rationale: provenance-as-content is the directive's better half â€” a colony's
story must survive a restart. The debugging payoff never needs yesterday's
mushroom.** â˜…â˜… **Rides item 18's save-integrity work naturally.** â˜… *The
bytes-per-sim-hour fan measurement gates retention tuning before anything defaults
on.*

### 2 Â· PLAYER-TOUCH = **DIRECT GOD-HAND MANIPULATION**

**Spawn, place, give.** â˜…â˜… *Not proximity, not selection, not looking.* **Cheap,
unambiguous, and true to the intent: player-touched things are story things.**

### 3 Â· REFUSED-THIEF EVENTS â€” **RECORDED AT THE PILE, NOT THE NPC**

â˜…â˜…â˜… **A refusal is written into the PILE's history with the would-be thief's uid as
`actor`.** *The pile is retained by nature â€” colony provisioning.* â˜…â˜…â˜…â˜… **The ambient
NPC is NOT retained on refusal: refusal is common, and retention must stay
meaningful or the permanent set becomes the ring with extra steps.**

> â˜…â˜…â˜… **The thief is retained on a SUCCESSFUL theft, the day the feature ships â€” which
> hands thievery its chronicle wiring on day one.**

â˜…â˜… *Refusals still enrich the lift-day delta; they just accrue to the victim's
ledger rather than the perpetrator's.*

**Sources:**
[RimWorld TaleDef templates](https://github.com/DrSakuu/RimWorld-Templates/blob/master/DefInjected/TaleDef/Tales_SinglePawn.xml) Â·
[PawnTaleLog](https://steamcommunity.com/workshop/filedetails/?id=3446240192) Â·
[DF Legendary artifact](https://dwarffortresswiki.org/index.php/DF2014:Legendary_artifact) Â·
[DF Legends](https://dwarffortresswiki.org/index.php/DF2014:Legends) Â·
[DF World generation](https://dwarffortresswiki.org/index.php/World_generation)
