# PRE-REGISTRATION — do not merge a reserved item

Written **before** the change. This is a modification to a **vanilla** system on
the strength of a mechanism found minutes ago, in a pipeline that has refuted
four confident stories tonight. It gets the full treatment.

## The mechanism

`server/src/sys/item.rs:224` — `delete_emitter.emit(DeleteEvent(source))`. The
merge system combines nearby stacks and **deletes the source entity**. Its filter
checks distance, persistence class and item compatibility. **It never asks
whether the item is reserved.**

So a colonist reserves a cooked dish, walks toward it, the dish merges into the
pile beside it, the uid stops resolving, and the eat job dies "sniped".

## Why the cooked dish specifically

Dishes drop with `should_merge: true` and accumulate **at the cook station** —
they are the food most likely to have a mergeable neighbour in range. Wheat is
scattered across fields. Measured: `def=apple_mushroom_curry` on **every** snipe.

## The change

Add a reservation test to the merge filter. `JobBoard` is an ECS resource, so the
item system can read `is_reserved` the same way HAUL-GEN already does.

## The prediction

**PASS requires:**

1. `food sniped — eat moot` with `cause="despawned"` falls toward **zero**.
2. `eat completions` rise (baseline 9 per full-day leg; 17 with provisions).
3. `fed` stops collapsing to 0–2 while `food_stock` is high.

**FAIL / VOID branches, named now:**

| Observation | Means |
|---|---|
| Snipes persist with `cause="despawned"` | Merging was **not** the deleter. Some other path deletes the entity — and `entity_still_exists=false` conflates a *fourth* thing. |
| Snipes fall but `fed` still collapses | Merge-theft was real and **not** the binding constraint on feeding. The remaining cause is downstream of reaching the food. |
| Food piles stop merging **at all** | The predicate is too broad — reserved should block merging *that item*, not freeze the whole ledger. Watch total loose-item count: if it climbs without bound, the filter is over-refusing. |
| Eat completions rise but hunger still wins every preempt | The arbitration livelock has a second driver beyond satisfiability. |

## What this run cannot test

- Whether the colony **survives a full day**. That needs a leg past tick 54,000
  unaccelerated, or ~18,000 at 3× decay, and this measures the eat pipeline only.
- Teleports and above-grade rescue — untouched by this.
- Anything visual.

## The risk I am taking

This changes an item system **all players use**, not just colonists. A reserved
item is by construction a colony concept, so vanilla play should see no
difference — but if `is_reserved` were ever true for a non-colony item, ordinary
loot would stop stacking. The gate must be reservation, **not** "is a colonist
nearby", and the over-refusal branch above is what would catch it.
