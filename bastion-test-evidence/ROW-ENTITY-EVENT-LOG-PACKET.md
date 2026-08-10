# ROW: THE ENTITY EVENT LOG — PILOT BUILD PACKET

**Design:** `readme/ENGINE-INTERNALS/DESIGN-ENTITY-EVENT-LOG.md` (DECISIONS #99,
forks ruled #100). **Scope:** the ruled pilot — **items + colonists only.**

**This is a LARGER row than the recent ones** and it is staged accordingly. Stage 1
is the whole pilot's foundation and is independently gate-able; stages 2 and 3 do
not begin until stage 1's determinism floor holds.

---

## ★ READ FIRST — A COLLISION WITH THE ROW CURRENTLY IN FLIGHT

**`record_pickup_verdict` (`server/src/events/inventory_manip.rs:86`) is being
touched RIGHT NOW** by the item-6 witness row: refusal counters split by reason ×
picker class are being added beside it.

**This design wants that same function to become an EVENT PRODUCER.** The design's
own rule is explicit:

> *"NOTHING IS DUPLICATED. Every existing trace either becomes a producer or is
> deleted in favour of one. Two logs of the same fact is how the pickup trace and
> the delete trace disagreed for two days."*

> ## **THEREFORE: the witness counters and the event stream MUST BE RECONCILED, NEVER STACKED.**

**The reconciliation, decided here so it is not rediscovered at the boundary:**

- **The counters STAY.** They are cheap aggregates, they are what a corpus fan
  reads, and they are the item-6 acceptance instrument. **The event log does not
  replace them.**
- **The event stream is the DETAIL layer**, carrying `actor` and ordering — which
  a counter structurally cannot carry.
- **What must NOT happen: a second free-text record of the same refusal.** The
  existing flight-recorder `note` string (`format!("item={item}; verdict=...")`)
  is exactly the parallel log the design forbids. **When the event producer
  lands, that `note` write is DELETED in its favour**, not left beside it.
- **The counters and the stream must agree by construction** — incremented and
  emitted at the same site, from the same branch, so a divergence is impossible
  rather than merely unlikely. **A test asserts count == stream events of that
  kind.**

---

## STAGE 1 — THE STREAM, THE PER-ENTITY RING, THE PROMOTION FLAG

### What must be BUILT (none of this exists — verified at `48886adfa0`)

The chassis (`bastion_flight_recorder.rs`) was read and **is not what the first
draft assumed**: it is `cap-and-DROP-NEWEST`, JSONL to disk, filtered to **ONE**
uid (`uid_filter: Option<u64>`). So:

| needed | status |
|---|---|
| multi-entity indexing | **build** |
| per-entity ring, oldest-out | **build** |
| promotion mechanism | **build** |

> ★ **DROP-NEWEST IS THE WRONG POLICY HERE and this is load-bearing.** It preserves
> a run's *beginning* — right for a startup bug, wrong for *"what happened to this
> item."* **And under a GLOBAL cap, one flooding entity starves every other
> entity's history.** The ring is **PER-ENTITY**, oldest-out. Do not reuse the
> global cap.

### What IS reused (do not rebuild)

- **The truncation flag.** `truncated_events = true`, surfaced in the summary —
  the recorder knows when it dropped data and says so. **Carry it forward
  per-entity**: a ring that silently discarded history is indistinguishable from
  an entity that had none. *This is the self-accounting law already implemented.*
- **Disabled-unless-env-set**, documented as not initialising, not doing I/O, not
  mutating ECS, not altering scheduling when off.
- **Versioned schema strings** and the config-from-env pattern.

### The record

```
EntityEvent { tick, subject: Uid, kind: EventKind, actor: Option<Uid>, data }
```

**Event-driven ONLY — never a per-tick sweep.** `actor` is the second uid (the
picker, the claimant, the killer): it is what makes *"taken BY Y"* expressible and
what would have named the ambient NPC in run one.

**Closed vocabulary, typed enums, never free strings** — greppable, countable,
cannot drift into prose. Pilot sets per the design (ITEM: `Created`, `Dropped{by}`,
`PickedUp{by}`, `Reserved{by}`, `Released{reason}`, `Consumed{by}`,
`Despawned{cause}`, `Split`, `Merged` — COLONIST: `Claimed{job}`,
`Released{job,reason}`, `Preempted{need}`, `Teleported{cause}`,
`NeedCrossed{need,dir}`, `Ate{item}`, `Stuck{cause}`).

### ★ CAPTURE BEFORE MUTATION

**The emission must not read state the mutation has already changed.** Capture the
values the event describes **before** the mutation, not after. *Three separate
instruments got this wrong in one week* — it is the default failure mode, not an
edge case.

### STAGE 1 GATE — determinism floor, PAIRED

**A paired floor run; the fingerprint must hold, BEFORE any producer defaults on.**
Event-driven emission with no collection walks *should* be free — **"should be" is
precisely what the floor run exists to reject.** Two per-cell diag reads once broke
bit-reproducibility; the density budget is not negotiable.

**Stages 2 and 3 do not start until this gate is green.**

---

## STAGE 2 — PRODUCERS (items + colonists only)

Convert existing traces; **do not add parallel ones.** Every existing trace either
**becomes** a producer or is **deleted in favour of** one.

- `record_pickup_verdict` → event producer (**and its `note` write deleted**, per
  the reconciliation above).
- The chronicle becomes a **PROJECTION over the stream** — `OnDeath` / `OnTheft`
  already *are* entity events; make that literal rather than analogous.

**Per DECISIONS #100 fork 3: refused-thief events are recorded AT THE PILE, not at
the NPC.** The subject is the pile; the refused NPC is the `actor`.

---

## ★ VOCABULARY RULING — **"PROMOTED" IS TAKEN. THIS CONCEPT IS "RETAINED".**

**5b found the collision in read-only prep, before it reached a doc.**
`BastionColonist`'s own comment (`common/src/bastion.rs:1441`) already uses it:

> *"…mirrored into the ECS `comp::Colonist` when the NPC is **promoted** to a
> loaded entity."*

**That is rtsim's established meaning — simulated-but-unloaded → live ECS
entity.** The event log's concept is a *different axis entirely*: ring →
unbounded permanent store. **A stage-3 log line reading "colonist promoted" would
be read as the wrong one by anyone who knows rtsim.**

> ## **RULING: the event-log concept is RETAINED / RETENTION. "Promotion" stays
> rtsim's, unqualified.** *`retained: bool`, `retain(uid)`, the "retained set".*

★ **Chosen because it describes the MECHANISM (kept, versus ring-evicted) rather
than a status**, so it cannot drift back into meaning "became important" — which
is the sense that collides. **The design's prose about significance is unchanged;
only the term for the mechanism moves.**

★★ **Rename before stage 3 opens.** *A term is cheap to change while it lives in
one packet and expensive once it is in five docs, a field name, and a log line —
and this one would have been actively misleading, not merely ambiguous.*

## STAGE 3 — RETENTION + PERSISTENCE

**Promotion is one-way and cheap: a flag plus a move from ring to permanent.** An
entity promotes on acquiring significance — a colonist is named; an item is
crafted, gifted, or **player-touched**; anything a chronicle entry references;
anything involved in a death.

**Per DECISIONS #100 fork 2, player-touch means DIRECT GOD-HAND MANIPULATION** —
not incidental proximity.

**Per fork 1, persistence SPLITS ALONG THE PROMOTION BOUNDARY:** promoted entities
persist across save/load as a save component **with version migration**; unpromoted
ring contents do not.

---

## ACCEPTANCE FRAMEWORK

**Per the newly banked law — every measure below is written as an expression that
can be evaluated, and BOTH branches are stated. A measure whose refutation has no
expression is not ready.**

| # | measure | PASS expression | FAIL expression |
|---|---|---|---|
| **0** | ★ **THE MOVER PREDICTION — the pilot's FIRST REAL CUSTOMER** (see below) | orphaned-claim releases **cluster in tick near removal events** on mover seeds | they spread **uniformly** → the claim-overlap hypothesis dies clean |
| 1 | **The Voonoo query** — given a drop's uid, return every actor that touched it, in order | query returns ≥1 event with a non-`None` `actor`, ordered by tick, naming the ambient picker | returns empty, or returns events with `actor == None` where an actor existed |
| 2 | **Provenance render** — one item's history as a sentence | a projection renders end-to-end with no parser over free text | rendering requires parsing a string field |
| 3 | **Determinism floor** | paired run, fingerprint identical | any fingerprint divergence |
| 4 | **Volume** | bytes/sim-hour **reported** at the default ring size | unreported — *an unmeasured retention policy IS the 1 GB failure mode* |

★ **Measure 1 is the row's original reason for existing.** That question cost two
runs and an instrument build; it must become **one query over a run's stream**.

### ★★ MEASURE 0 — **PROMOTED: THE PILOT ARRIVES WITH A CUSTOMER ALREADY WAITING**

**`Released{job, reason, tick, actor}` is REQUIRED in the colonist vocabulary**
(architect-ruled), because a real open investigation is already blocked on it.

**The case.** `b5_release_removed_externally` is the only surviving mechanism
candidate for the wave32→wave33 movers — concentrated **5/6 on movers vs 10/42
elsewhere**, and it survived a permutation test (**2000 random 6-seed subsets;
1957 showed no such field; p = 0.021**), so the concentration is not a
multiple-comparisons artefact.

**But reading its producer (`bastion_jobs.rs:10887`) changed what it means:**

```rust
let Some(job) = board.jobs.get_mut(&active.job) else {
    // Cancelled out from under the colonist → re-idle.
    to_release.push((entity, ReleaseReason::RemovedExternally));
```

**It is a SITE SCAN over active claims — an observer-assigned ORPHAN DETECTOR, not
a removal-path witness.** None of the 20+ `remove_job` callers assigns this
reason. **So the counter measures a COMPOSITION: a removal AND a live claimant at
the same moment.** *That is why it concentrates on movers while raw access-plan
counts churn corpus-wide.*

> ## **THE PREDICTION, AND WHY THE PILOT IS THE ONLY THING THAT CAN DECIDE IT:**
> *if this is claim-OVERLAP (colonists who no longer sit hold claims over
> different intervals — same removals, different overlap), the orphanings should
> **cluster in tick near removal events**. If they spread uniformly, it dies.*
>
> ★★★ **`release_removed_externally` IS A RUN-TOTAL. There is no per-event
> timestamp anywhere in the corpus.** *The question is undecidable at any seed
> count, by any fan, with the instruments that exist.*

★ **This is the non-vacuity question asked AT REGISTRATION and answered NO** —
which is what converts *"run another fan"* into *"the instrument this programme
already designed is the only thing that can answer it."* **A pilot whose first
customer arrived before the pilot did.**

### Named failure modes, each with a planted test that must go red BY NAME

- **A flooding entity starves others' history.** *Plant:* one entity emitting far
  past the ring size; every OTHER entity's history must be intact. **This is the
  test that proves the ring is per-entity and not global** — the single most
  likely thing to be got wrong.
- **Silent truncation.** *Plant:* overflow one entity's ring; the truncation flag
  must be set and surfaced for THAT entity. A ring that dropped history must never
  be indistinguishable from an entity that had none.
- **Duplicate fact.** *Plant:* a refusal; assert exactly ONE record exists (the
  counter increments AND one event) — **not two logs of the same fact.**
- **Capture-after-mutation.** *Plant:* an event describing a value that the
  mutation changes; the recorded value must be the pre-mutation one.
- **Emission when disabled.** With the env unset: no I/O, no ECS mutation, no
  allocation on the hot path, fingerprint unchanged.

### Non-vacuity

The pilot scenario must **demonstrably** produce both an item with ≥2 distinct
actors and a colonist with ≥2 distinct job claims. **Zero multi-actor items = VOID,
not PASS** — measure 1 would be untested and a green result would mean nothing.

### What a PASS does NOT establish

Pilot scope is **items + colonists**. A pass says nothing about ambient NPCs,
sites, or combat, and nothing about persistence until stage 3 lands. **Say so in
the results**, so a green stage-1 gate is not over-read as the feature working.

---

## COST NOTE

**Event-driven, no sweeps, no collection walks, disabled by default until the
floor run says otherwise.** The retention policy is the design's centre of
gravity, not an afterthought: **both prior-art systems (RimWorld `TaleDef`, DF
historical figures) solve volume by SIGNIFICANCE, and so does this.** Ring size
and promotion rules are **TUNABLE and must be measured, not guessed** — the first
fan after this lands reports stream bytes per sim-hour.
